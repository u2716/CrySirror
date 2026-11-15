// SPDX-License-Identifier: MPL-2.0
use crate::{files::ImportedFiles, storage::BrowserStorage};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    service::{ClientStorage, InputFrame},
    state::ClientId,
};
use crysirror_runtime::{
    frame::{PendingFrame, RenderSnapshot},
    AdapterRegistry, Project, Runtime,
};
use crysirror_wgpu::{Gpu, RenderTarget, Renderer};
use std::{collections::BTreeMap, sync::Arc};
use wasm_bindgen::prelude::*;
use web_sys::OffscreenCanvas;

fn js_error(error: impl std::fmt::Display) -> JsValue {
    js_sys::Error::new(&error.to_string()).into()
}
/// Dedicated-worker browser host. GPU objects and Runtime ownership never
/// leave this worker; messages contain only input, imported bytes and PCM.
/// The project's runtime, driven directly (data projects) or by the engine
/// App with its JS script host (scripted projects such as the library
/// launcher). Dereferences to the runtime either way.
enum Core {
    Plain(Runtime),
    Scripted(Box<crysirror_engine::App>),
}
impl Core {
    fn tick(&mut self, elapsed: u64, input: InputFrame) -> Result<()> {
        match self {
            Self::Plain(rt) => rt.tick_control(elapsed, input),
            Self::Scripted(app) => app.frame(elapsed, input),
        }
    }
    fn quit_requested(&self) -> bool {
        matches!(self, Self::Scripted(app) if app.quit_requested())
    }
}
impl std::ops::Deref for Core {
    type Target = Runtime;
    fn deref(&self) -> &Runtime {
        match self {
            Self::Plain(rt) => rt,
            // The App owns its runtime until into_runtime, never called here.
            Self::Scripted(app) => app.runtime().expect("engine runtime"),
        }
    }
}
impl std::ops::DerefMut for Core {
    fn deref_mut(&mut self) -> &mut Runtime {
        match self {
            Self::Plain(rt) => rt,
            Self::Scripted(app) => app.runtime_mut().expect("engine runtime"),
        }
    }
}

#[wasm_bindgen]
pub struct BrowserPlayer {
    runtime: Core,
    snapshot_storage: Arc<BrowserStorage>,
    scene_audio: crysirror_audio::scene::SceneAudio,
    mixer: crysirror_audio::mixer::Mixer,
    audio_frame: Option<(u64, crysirror_audio::render_job::MixFrame)>,
    audio_remainder: u64,
    pcm: Vec<[f32; 2]>,
    _instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    gpu: Arc<Gpu>,
    renderer: Renderer,
    config: wgpu::SurfaceConfiguration,
    pending: Option<PendingFrame>,
    snapshot: Option<RenderSnapshot>,
    elapsed: u64,
    pressed: u64,
    presented: u64,
}
#[wasm_bindgen]
impl BrowserPlayer {
    /// Files are imported completely before construction; no synchronous fetch
    /// or native filesystem emulation is used by engine storage calls.
    /// `scripts` names imported JS/TS files run by the engine App as
    /// `script_client` (empty for data projects); `font` is an optional
    /// owner font registered as "default".
    #[wasm_bindgen(js_name = create)]
    pub async fn create(
        canvas: OffscreenCanvas,
        project_json: &str,
        names: js_sys::Array,
        data: js_sys::Array,
        save_key: &str,
        scripts: js_sys::Array,
        script_client: &str,
        font: Option<Vec<u8>>,
    ) -> std::result::Result<BrowserPlayer, JsValue> {
        Self::create_inner(canvas, project_json, names, data, save_key, scripts, script_client, font)
            .await
            .map_err(js_error)
    }
    /// Whether a scripted project asked to stop (the launcher after a choice).
    #[wasm_bindgen(js_name = quitRequested)]
    pub fn quit_requested(&self) -> bool {
        self.runtime.quit_requested()
    }
    /// Read a UTF-8 save file of the first client, or null.
    #[wasm_bindgen(js_name = readSave)]
    pub fn read_save(&mut self, path: &str) -> Option<String> {
        use crysirror_core::{path::{Mount, VfsPath}, service::FileSystem};
        let path = VfsPath::new(Mount::Save, path).ok()?;
        String::from_utf8(self.runtime.host.fs.read(ClientId(1), &path).ok()?).ok()
    }
    /// One owner poll, not a join. Input edges remain queued until audio permits
    /// the next simulation tick; incomplete drawing retains the last snapshot.
    pub fn frame(
        &mut self,
        delta_ms: u32,
        down: u32,
        pressed: u32,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> std::result::Result<bool, JsValue> {
        self.frame_inner(delta_ms, down, pressed, x, y, width, height)
            .map_err(js_error)
    }
    /// Interleaved stereo PCM at the engine's 48 kHz output rate.
    #[wasm_bindgen(js_name = takeAudio)]
    pub fn take_audio(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.pcm)
            .into_iter()
            .flatten()
            .collect()
    }
    pub fn inspect(&self) -> String {
        serde_json::json!({"runtime": self.runtime.inspect(), "presented": self.presented, "execution": if crysirror_tasks::browser_workers() > 0 { "dedicated-worker-parallel" } else { "dedicated-worker-cooperative" }, "canvas": self.runtime.project.canvas}).to_string()
    }
    /// Persist the existing engine snapshot through the same transaction
    /// receipts exposed to VM storage clients. Completion means committed.
    #[wasm_bindgen(js_name = saveSnapshot)]
    pub async fn save_snapshot(&self) -> std::result::Result<(), JsValue> {
        use crysirror_core::{
            path::{Mount, VfsPath},
            service::StorageWriteState,
        };
        let bytes = self.runtime.snapshot().map_err(js_error)?;
        let path = VfsPath::new(Mount::Save, "runtime.snapshot").map_err(js_error)?;
        let mut write = self
            .snapshot_storage
            .begin_write_atomic(&path, bytes.into())
            .map_err(js_error)?;
        loop {
            match write.poll().map_err(js_error)? {
                StorageWriteState::Ready => return Ok(()),
                StorageWriteState::Failed(error) => return Err(js_error(error)),
                StorageWriteState::Pending => gloo_timers::future::TimeoutFuture::new(1).await,
            }
        }
    }
    #[wasm_bindgen(js_name = loadSnapshot)]
    pub async fn load_snapshot(&mut self) -> std::result::Result<(), JsValue> {
        use crysirror_core::path::{Mount, VfsPath};
        let storage = self.snapshot_storage.refreshed().await.map_err(js_error)?;
        let path = VfsPath::new(Mount::Save, "runtime.snapshot").map_err(js_error)?;
        let bytes = storage.read(&path).map_err(js_error)?;
        self.restore(&bytes)?;
        self.snapshot_storage = storage;
        Ok(())
    }
    pub fn snapshot(&self) -> std::result::Result<Vec<u8>, JsValue> {
        self.runtime.snapshot().map_err(js_error)
    }
    pub fn restore(&mut self, bytes: &[u8]) -> std::result::Result<(), JsValue> {
        let prepared = self
            .runtime
            .restore_plan()
            .decode(bytes)
            .map_err(js_error)?;
        self.cancel_audio();
        self.runtime.commit_restore(prepared).map_err(js_error)?;
        self.scene_audio.reset(&mut self.mixer);
        self.audio_remainder = 0;
        self.pcm.clear();
        self.pending = None;
        self.snapshot = None;
        self.pressed = 0;
        self.elapsed = 0;
        Ok(())
    }
}
impl BrowserPlayer {
    #[allow(clippy::too_many_arguments)]
    async fn create_inner(
        canvas: OffscreenCanvas,
        json: &str,
        names: js_sys::Array,
        data: js_sys::Array,
        save_key: &str,
        scripts: js_sys::Array,
        script_client: &str,
        font: Option<Vec<u8>>,
    ) -> Result<Self> {
        ensure!(
            json.len() <= crysirror_runtime::model::MAX_PROJECT_BYTES,
            "browser project exceeds 4 MiB"
        );
        let project: Project = serde_json::from_str(json).context("browser project JSON")?;
        project.validate()?;
        ensure!(
            !save_key.is_empty()
                && save_key.len() <= 128
                && save_key
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b)),
            "invalid browser save namespace"
        );
        ensure!(
            names.length() == data.length() && names.length() <= 65536,
            "browser file table mismatch"
        );
        let mut imported = ImportedFiles::default();
        for i in 0..names.length() {
            let name = names.get(i).as_string().context("browser filename")?;
            let bytes = data
                .get(i)
                .dyn_into::<js_sys::Uint8Array>()
                .map_err(|_| anyhow::anyhow!("browser asset must be Uint8Array"))?;
            ensure!(
                bytes.length() as usize <= crate::files::MAX_IMPORT_FILE_BYTES,
                "browser imported file exceeds 64 MiB"
            );
            imported.insert(&name, bytes.to_vec())?;
        }
        let imported = Arc::new(imported);
        // Project.assets has the same meaning as native project-file launch.
        let root = if project.assets == "." {
            ""
        } else {
            &project.assets
        };
        let project_storage = imported.scoped(root)?;
        let mut clients: BTreeMap<ClientId, Arc<dyn ClientStorage>> = BTreeMap::new();
        for (i, spec) in project.clients.iter().enumerate() {
            let prefix = [root, &spec.asset_prefix]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("/");
            let storage = BrowserStorage::open(
                imported.scoped(&prefix)?,
                format!("{save_key}/{}/{}/{}", spec.adapter, spec.game, spec.name),
            )
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
            clients.insert(ClientId((i + 1) as u32), storage);
        }
        let snapshot_storage =
            BrowserStorage::open(project_storage.clone(), format!("{save_key}/host"))
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        let mut runtime = Runtime::with_storage(
            project,
            project_storage,
            clients,
            AdapterRegistry::default(),
        )?;
        runtime.assets.set_nonblocking(true);
        runtime.set_audio_nonblocking(true);
        if let Some(font) = font {
            runtime.host.text.add_font("default", font.clone())?;
            runtime.assets.text.add_font("default", font)?;
        }
        let mut sources = Vec::new();
        for i in 0..scripts.length() {
            let name = scripts.get(i).as_string().context("browser script name")?;
            let bytes = imported.read_file(&name).with_context(|| format!("script {name}"))?;
            sources.push((name, String::from_utf8(bytes).context("script is not UTF-8")?));
        }
        let runtime = if sources.is_empty() {
            Core::Plain(runtime)
        } else {
            let mut app = crysirror_engine::App::new(runtime, script_client)?;
            use crysirror_engine::ScriptHost as _;
            let mut host = crysirror_script_js::JsHost::new("scripts");
            for (name, source) in &sources {
                host.add_source(name, source)?;
            }
            app.add_script_host(Box::new(host))?;
            app.init()?;
            app.start()?;
            Core::Scripted(Box::new(app))
        };
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = instance.create_surface(wgpu::SurfaceTarget::OffscreenCanvas(canvas))?;
        let gpu = Arc::new(
            Gpu::request(&instance, Some(&surface))
                .await
                .context("browser WebGPU unavailable")?,
        );
        let caps = surface.get_capabilities(&gpu.adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .or_else(|| caps.formats.first().copied())
            .context("browser surface has no texture format")?;
        let view_format = format.add_srgb_suffix();
        let config = wgpu::SurfaceConfiguration {
            color_space: wgpu::SurfaceColorSpace::Auto,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: runtime.project.canvas[0],
            height: runtime.project.canvas[1],
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: if format == view_format {
                vec![]
            } else {
                vec![view_format]
            },
        };
        surface.configure(&gpu.device, &config);
        let renderer = Renderer::new(gpu.clone(), view_format);
        Ok(Self {
            runtime,
            snapshot_storage,
            scene_audio: Default::default(),
            mixer: crysirror_audio::mixer::Mixer::new(crysirror_audio::mixer::MixerConfig {
                max_voices: crysirror_core::state::Limits::default().voice_count,
                ..Default::default()
            })?,
            audio_frame: None,
            audio_remainder: 0,
            pcm: Vec::new(),
            _instance: instance,
            surface,
            gpu,
            renderer,
            config,
            pending: None,
            snapshot: None,
            elapsed: 0,
            pressed: 0,
            presented: 0,
        })
    }
    fn begin_audio(&mut self, delta_ms: u64) -> Result<()> {
        ensure!(self.audio_frame.is_none(), "browser audio frame pending");
        let rt = &mut self.runtime;
        self.scene_audio.sync(
            &mut self.mixer,
            &rt.host.scene,
            rt.restored,
            &rt.assets.io_tasks(),
            true,
            |client| {
                rt.host
                    .fs
                    .storage(client)
                    .map_err(|e| anyhow::anyhow!("{e}"))
            },
        )?;
        let numerator = delta_ms
            .checked_mul(48_000)
            .and_then(|n| n.checked_add(self.audio_remainder))
            .context("browser audio clock overflow")?;
        let count = (numerator / 1000) as usize;
        self.audio_remainder = numerator % 1000;
        let native = rt.host.channel_audio.prepare_render(delta_ms)?;
        let extra = rt.host.take_pcm_output();
        self.audio_frame = Some((
            rt.restored,
            crysirror_audio::render_job::MixFrame::new(
                rt.assets.cpu_tasks(),
                vec![count, native],
                extra,
            )?,
        ));
        Ok(())
    }
    fn cancel_audio(&mut self) {
        if let Some((_, mut frame)) = self.audio_frame.take() {
            frame.cancel(&mut [&mut self.mixer, &mut self.runtime.host.channel_audio.mixer]);
        }
    }
    fn poll_audio(&mut self) -> Result<bool> {
        let Some((epoch, mut frame)) = self.audio_frame.take() else {
            return Ok(true);
        };
        if epoch != self.runtime.restored {
            frame.cancel(&mut [&mut self.mixer, &mut self.runtime.host.channel_audio.mixer]);
            self.scene_audio.reset(&mut self.mixer);
            return Ok(true);
        }
        match frame.poll(&mut [&mut self.mixer, &mut self.runtime.host.channel_audio.mixer]) {
            Ok(Some(pcm)) => {
                ensure!(
                    self.pcm.len() + pcm.len() <= 96_000,
                    "browser audio output queue full; consume takeAudio before advancing"
                );
                self.pcm.extend(pcm);
                Ok(true)
            }
            Ok(None) => {
                self.audio_frame = Some((epoch, frame));
                Ok(false)
            }
            Err(error) => {
                frame.cancel(&mut [&mut self.mixer, &mut self.runtime.host.channel_audio.mixer]);
                Err(error)
            }
        }
    }
    fn frame_inner(
        &mut self,
        delta: u32,
        down: u32,
        pressed: u32,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> Result<bool> {
        // Pause time accumulation for background tabs instead of replaying an
        // unbounded catch-up batch. CSS resizing never changes the game canvas.
        self.elapsed = self
            .elapsed
            .saturating_add(u64::from(delta.min(100)))
            .min(100);
        self.pressed |= u64::from(pressed);
        if self.poll_audio()? {
            let input = InputFrame {
                buttons_down: down.into(),
                buttons_pressed: self.pressed,
                pointer_x: x,
                pointer_y: y,
            };
            let elapsed = std::mem::take(&mut self.elapsed);
            self.runtime.tick(elapsed, input)?;
            self.begin_audio(elapsed)?;
            self.pressed = 0;
        }
        if let Some(pending) = &mut self.pending {
            if let Some(snapshot) = pending.poll(&self.runtime)? {
                self.snapshot = Some(snapshot);
                self.pending = None;
            }
        }
        if self.pending.is_none()
            && (self.snapshot.is_none() || !self.runtime.presentation_pending())
        {
            self.pending = Some(self.runtime.schedule_render_snapshot()?);
        }
        if width == 0 || height == 0 {
            return Ok(false);
        }
        ensure!(
            width <= self.gpu.device.limits().max_texture_dimension_2d
                && height <= self.gpu.device.limits().max_texture_dimension_2d,
            "browser canvas exceeds GPU limits"
        );
        if self.config.width != width || self.config.height != height {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.gpu.device, &self.config);
        }
        let Some(snapshot) = &self.snapshot else {
            return Ok(false);
        };
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.gpu.device, &self.config);
                return Ok(false);
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(false)
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                anyhow::bail!("browser surface validation failed")
            }
        };
        self.renderer
            .set_profiler(self.runtime.profiler(), snapshot.frame);
        self.renderer.set_scene3d(snapshot.simple.as_ref())?;
        self.renderer.set_pbr_scene(snapshot.pbr.clone())?;
        self.renderer.set_engine_frame(snapshot.layers.clone())?;
        self.renderer.set_transition(snapshot.transition.clone())?;
        self.renderer.set_presentation_scale(
            self.runtime
                .host
                .engine
                .ui
                .presentation_scale
                .map(|(_, mode)| mode)
                .unwrap_or_default(),
        );
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.config.format.add_srgb_suffix()),
            ..Default::default()
        });
        let (canvas, background) = (self.runtime.project.canvas, self.runtime.project.background);
        let rendered =
            self.runtime
                .assets
                .with_frame_surfaces(snapshot.surfaces.clone(), |assets| {
                    self.renderer.render(
                        &snapshot.scene,
                        assets,
                        RenderTarget {
                            view: &view,
                            size: [width, height],
                        },
                        canvas,
                        background,
                    )
                });
        match rendered {
            Err(error) if error.is::<crysirror_wgpu::PreparationPending>() => return Ok(false),
            other => other?,
        }
        self.renderer.gpu.queue.present(frame);
        self.presented += 1;
        Ok(true)
    }
}

impl Drop for BrowserPlayer {
    fn drop(&mut self) {
        self.cancel_audio();
    }
}

/// The shared game library launcher project (crysirror-library) for the
/// page's entries: `entries` is JSON `[{id, name, detail}]`, `covers` holds
/// an optional image file (PNG/JPEG bytes) per entry. Returns `[names,
/// files]` for `BrowserPlayer.create`, with `launcher.js` as the script.
#[wasm_bindgen(js_name = libraryProject)]
pub fn library_project(
    entries: &str,
    covers: js_sys::Array,
    width: u32,
    height: u32,
    font: Option<Vec<u8>>,
) -> std::result::Result<js_sys::Array, JsValue> {
    use crysirror_library::{cover, library};
    let list: Vec<serde_json::Value> = serde_json::from_str(entries).map_err(js_error)?;
    let mut items = Vec::new();
    for (i, entry) in list.iter().enumerate() {
        let text = |key: &str| entry[key].as_str().unwrap_or_default().to_owned();
        let image = covers
            .get(i as u32)
            .dyn_into::<js_sys::Uint8Array>()
            .ok()
            .and_then(|bytes| image::load_from_memory(&bytes.to_vec()).ok());
        items.push(library::Entry {
            id: text("id"),
            name: text("name"),
            detail: text("detail"),
            cover: image.map(|image| cover::Cover {
                image: image.to_rgba8(),
                kind: cover::CoverKind::Image,
                source: Default::default(),
            }),
        });
    }
    let style = library::Style {
        confirm: "Enter",
        cross: false,
        add_hint: "Open a project folder to add it here".into(),
    };
    let files = library::build(&items, (width, height), font.as_deref(), &style).map_err(js_error)?;
    let (names, data) = (js_sys::Array::new(), js_sys::Array::new());
    for (name, bytes) in files {
        names.push(&JsValue::from_str(&name));
        data.push(&js_sys::Uint8Array::from(bytes.as_slice()));
    }
    Ok(js_sys::Array::of2(&names, &data))
}

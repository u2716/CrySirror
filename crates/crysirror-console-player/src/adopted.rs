// SPDX-License-Identifier: MPL-2.0
//! Adopted-engine session: the shared CrySirror runtime and the unchanged
//! shared renderer (`crysirror-wgpu`) on the console's wgpu backend.
//!
//! The sequence per frame is the desktop's (`crysirror-launch` and the
//! headless launcher): publish the virtual window, input sources and pointer,
//! tick the core, materialize compacted canvases, then render the frame's
//! immutable snapshot into the display framebuffer and present it.
use anyhow::{Context, Result};
use crysirror_core::service::{InputFrame, InputSourcesFrame, PointerObservation, buttons};
use crysirror_launch::{core as player, detect, desktop_provider, integrated_roots, virtual_window::VirtualWindow};
use crysirror_runtime::{Project, Runtime};
use crysirror_wgpu::{RenderTarget, Renderer, geometry::Viewport};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct Adopted {
    core: player::Core,
    renderer: Renderer,
    gpu: Arc<crysirror_wgpu_deko3d::Gpu>,
    window: VirtualWindow,
    canvas: [u32; 2],
    held: u64,
    touch: Option<[f64; 2]>,
    touch_down: bool,
    pub engine: String,
    dump: Option<PathBuf>,
    /// Console render tier applied to every 3D frame (crysirror-render-tier).
    pub tier: crysirror_render_tier::Tier,
    pub tier_report: Vec<crysirror_render_tier::Decision>,
}

/// Shared console button index (libnx order) to engine button bits.
fn button_bits(button: u8) -> u64 {
    match button {
        0 => buttons::ENTER,
        1 => buttons::ESCAPE,
        2 => buttons::SPACE,
        3 => buttons::TAB,
        6 => buttons::SHIFT,
        7 | 8 => buttons::CTRL,
        9 => buttons::MOUSE_RIGHT,
        10 => buttons::ESCAPE,
        11 => buttons::TAB,
        12 | 16 | 20 => buttons::LEFT,
        13 | 17 | 21 => buttons::UP,
        14 | 18 | 22 => buttons::RIGHT,
        15 | 19 | 23 => buttons::DOWN,
        _ => 0,
    }
}

impl Adopted {
    /// Detect and launch `game_dir` with saves under `state_dir`. A folder
    /// holding `project.json` is an authored project: its `.ts`/`.js`
    /// scripts run in the engine's JS host, as `crysirror run --script`.
    pub fn open(game_dir: &Path, state_dir: &Path, font: Option<PathBuf>, gpu: crysirror_wgpu_deko3d::Gpu) -> Result<Self> {
        let dir = game_dir.canonicalize().context("game directory")?;
        if dir.join("project.json").is_file() {
            return Self::open_project(&dir, state_dir, font, gpu);
        }
        let plan = detect::detect(&dir)?;
        let mut project: Project = serde_json::from_value(plan.project.clone())?;
        std::fs::create_dir_all(state_dir)?;
        integrated_roots::bind(&mut project, &dir, state_dir, font.as_deref())?;
        let mut rt = Runtime::with_asset_root(project, &dir, state_dir, player::adapter_registry()?)?;
        desktop_provider::install(&mut rt, false)?;
        rt.assets.decoders.register("g00", Arc::new(crysirror_siglus::SiglusImageDecoder))?;
        let window = VirtualWindow::new(rt.project.canvas);
        window.publish(&mut rt);
        player::load_fonts(&mut rt, font.as_deref())?;
        if let Some(path) = font.as_deref() {
            let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            let covered = rt.host.text.measure_simple("default", 16.0, "\u{203b}").is_ok();
            crate::log(&format!("adopted: font {}: {size} bytes, U+203B {covered}", path.display()));
        }
        let canvas = rt.project.canvas;
        let _ = window;
        let core = player::bootstrap(rt, &plan)?;
        Self::finish(core, canvas, gpu, plan.engine.to_string())
    }

    fn open_project(dir: &Path, state_dir: &Path, font: Option<PathBuf>, gpu: crysirror_wgpu_deko3d::Gpu) -> Result<Self> {
        use crysirror_engine::ScriptHost as _;
        let manifest = dir.join("project.json");
        crate::log("adopted: project: load");
        let mut project = player::load_project(&manifest)?;
        let root = crysirror_runtime::runtime::asset_root(&manifest, &project)?;
        std::fs::create_dir_all(state_dir)?;
        integrated_roots::bind(&mut project, &root, state_dir, font.as_deref())?;
        crate::log("adopted: project: runtime");
        let mut rt = Runtime::with_asset_root(project, &root, state_dir, player::adapter_registry()?)?;
        desktop_provider::install(&mut rt, false)?;
        rt.assets.decoders.register("g00", Arc::new(crysirror_siglus::SiglusImageDecoder))?;
        let window = VirtualWindow::new(rt.project.canvas);
        window.publish(&mut rt);
        player::load_fonts(&mut rt, font.as_deref())?;
        // Without an owner font, authored projects (the launcher) draw with
        // the platform's system font (Switch shared font), never copied to
        // storage.
        #[cfg(any(target_os = "horizon", target_os = "vita"))]
        if font.is_none() {
            if let Some(bytes) = crate::session::system_font() {
                rt.host.text.add_font("default", bytes.to_vec())?;
                rt.assets.text.add_font("default", bytes.to_vec())?;
            }
        }
        let canvas = rt.project.canvas;
        let mut scripts: Vec<PathBuf> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "ts" || x == "js") && !p.to_string_lossy().ends_with(".d.ts"))
            .collect();
        scripts.sort();
        // `console-scripts` names the scripts explicitly (one per line);
        // otherwise `game.ts` alone, since sibling scripts are alternatives.
        if let Ok(list) = std::fs::read_to_string(dir.join("console-scripts")) {
            scripts = list.lines().map(str::trim).filter(|l| !l.is_empty()).map(|l| dir.join(l)).collect();
        } else if dir.join("game.ts").is_file() {
            scripts = vec![dir.join("game.ts")];
        }
        let core = if scripts.is_empty() {
            player::Core::Plain(rt)
        } else {
            let client = rt.project.clients.first().map(|c| c.name.clone()).unwrap_or_else(|| "game".into());
        if dir.parent().and_then(|g| g.parent()).is_some_and(|r| r.join("js-probe-after-runtime").exists()) {
            crate::log("adopted: probe after runtime");
            #[cfg(any(target_os = "horizon", target_os = "vita"))]
            crate::session::js_probe();
        }
        crate::log("adopted: project: engine app");
            let mut app = crysirror_engine::App::new(rt, &client)?;
            let mut host = crysirror_script_js::JsHost::new("scripts");
            for path in &scripts {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("script.js");
                host.add_source(name, &std::fs::read_to_string(path)?)?;
            }
            app.add_script_host(Box::new(host))?;
        crate::log("adopted: project: init");
            app.init()?;
        crate::log("adopted: project: start");
            app.start()?;
            player::Core::Scripted(Box::new(app))
        };
        Self::finish(core, canvas, gpu, "CrySirror project".into())
    }

    fn finish(mut core: player::Core, canvas: [u32; 2], gpu: crysirror_wgpu_deko3d::Gpu, engine: String) -> Result<Self> {
        let gpu = Arc::new(gpu);
        let shared = Arc::new(crysirror_wgpu::Gpu { adapter: gpu.adapter.clone(), device: gpu.device.clone(), queue: gpu.queue.clone() });
        crate::log("adopted: renderer: create");
        let renderer = Renderer::new(shared, wgpu::TextureFormat::Rgba8UnormSrgb);
        core.rt().host.mesh_frame_history = Some(renderer.mesh_frame_history_provider());
        core.rt().set_canvas_image_presenter(true);
        crate::log("adopted: settle");
        player::settle_vm_preparations(core.rt())?;
        crate::log("adopted: ready");
        let window = VirtualWindow::new(canvas);
        Ok(Self { core, renderer, gpu, window, canvas, held: 0, touch: None, touch_down: false, engine, dump: None, tier: if cfg!(target_os = "vita") { crysirror_render_tier::Tier::Vita } else { crysirror_render_tier::Tier::SwitchHandheld }, tier_report: Vec::new() })
    }

    pub fn button(&mut self, button: u8, down: bool) {
        let bit = button_bits(button);
        if down {
            self.held |= bit;
        } else {
            self.held &= !bit;
        }
    }

    /// Display-pixel touch, mapped through the renderer's letterbox.
    pub fn touch(&mut self, phase: i32, x: f64, y: f64) {
        let size = self.gpu.display().size();
        let vp = Viewport::with_mode(self.canvas, size, crysirror_core::service::PresentationScale::Fit);
        let cx = (x - f64::from(vp.x)) * f64::from(self.canvas[0]) / f64::from(vp.width.max(1.));
        let cy = (y - f64::from(vp.y)) * f64::from(self.canvas[1]) / f64::from(vp.height.max(1.));
        self.touch = Some([cx, cy]);
        self.touch_down = phase != 2;
    }

    fn input(&mut self, previous: u64) -> InputFrame {
        let mut down = self.held;
        if self.touch_down {
            down |= buttons::MOUSE_LEFT;
        }
        let [px, py] = self.touch.unwrap_or([-1., -1.]);
        InputFrame { buttons_down: down, buttons_pressed: down & !previous, pointer_x: px as i32, pointer_y: py as i32, ..Default::default() }
    }

    /// One frame; true when the game asked to quit.
    pub fn step(&mut self, dt_ms: u32, previous: &mut u64) -> Result<bool> {
        let input = self.input(*previous);
        *previous = input.buttons_down;
        let canvas = self.canvas;
        let rt = self.core.rt();
        self.window.begin_tick(rt);
        if rt.host.pointer_activity.is_none() || rt.host.pointer_observation.position != Some([input.pointer_x, input.pointer_y]) {
            rt.host.note_pointer_activity()?;
        }
        rt.host.pointer_observation = PointerObservation {
            position: Some([input.pointer_x, input.pointer_y]),
            inside: Some(input.pointer_x >= 0 && input.pointer_y >= 0 && input.pointer_x < canvas[0] as i32 && input.pointer_y < canvas[1] as i32),
        };
        rt.host.publish_input_sources(InputSourcesFrame { keyboard_pointer: input, controller_backend: false, controllers: Vec::new() })?;
        match &mut self.core {
            player::Core::Plain(rt) => rt.tick(u64::from(dt_ms), input)?,
            player::Core::Scripted(engine) => engine.frame(u64::from(dt_ms), input)?,
        }
        player::materialize_canvases(&mut self.renderer, self.core.rt())?;
        self.present()?;
        Ok(matches!(&self.core, player::Core::Scripted(e) if e.quit_requested()))
    }

    fn present(&mut self) -> Result<()> {
        let rt = self.core.rt();
        let snapshot = rt.schedule_render_snapshot()?.wait(rt)?;
        let display = self.gpu.display();
        let Some(frame) = display.acquire() else { return Ok(()) };
        let size = display.size();
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor { format: Some(wgpu::TextureFormat::Rgba8UnormSrgb), ..Default::default() });
        let r = &mut self.renderer;
        r.set_scene3d(snapshot.simple.as_ref())?;
        // Equivalent reduced 3D features for this console tier; VN layers
        // never pass through the tier policy.
        let pbr = snapshot.pbr.map(|scene| {
            let reduced = crysirror_render_tier::reduce(&scene, self.tier);
            if reduced.decisions != self.tier_report {
                self.tier_report = reduced.decisions.clone();
            }
            reduced.scene
        });
        r.set_pbr_scene(pbr)?;
        r.set_engine_frame(snapshot.layers)?;
        r.set_transition(snapshot.transition)?;
        let (canvas, background) = (rt.project.canvas, rt.project.background);
        rt.assets.with_frame_surfaces(snapshot.surfaces, |assets| {
            r.render(&snapshot.scene, assets, RenderTarget { view: &view, size }, canvas, background)
        })?;
        if let Some(path) = self.dump.take() {
            Self::dump_display(&self.gpu, &frame.texture, size, &path)?;
        }
        display.present(frame);
        Ok(())
    }

    /// Copy the presented framebuffer to a PNG (diagnostics: what the
    /// display shows, not an offscreen re-render).
    fn dump_display(gpu: &crysirror_wgpu_deko3d::Gpu, texture: &wgpu::Texture, size: [u32; 2], path: &Path) -> Result<()> {
        let bpr = (size[0] * 4).next_multiple_of(256);
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("display dump"),
            size: u64::from(bpr) * u64::from(size[1]),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo { buffer: &buffer, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(bpr), rows_per_image: Some(size[1]) } },
            wgpu::Extent3d { width: size[0], height: size[1], depth_or_array_layers: 1 },
        );
        gpu.queue.submit([encoder.finish()]);
        buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        gpu.device.poll(wgpu::PollType::wait_indefinitely())?;
        let data = buffer.slice(..).get_mapped_range().map_err(|e| anyhow::anyhow!("{e:?}"))?;
        let mut rgba = Vec::with_capacity((size[0] * size[1] * 4) as usize);
        for row in 0..size[1] as usize {
            rgba.extend_from_slice(&data[row * bpr as usize..row * bpr as usize + size[0] as usize * 4]);
        }
        image::RgbaImage::from_raw(size[0], size[1], rgba).context("display image")?.save(path)?;
        Ok(())
    }

    /// Save the next presented display frame to `path`.
    pub fn capture_png(&mut self, path: &Path) -> Result<()> {
        self.dump = Some(path.to_path_buf());
        Ok(())
    }
}

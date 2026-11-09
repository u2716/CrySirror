//! CrySirror owner/worker boundary for the original SoftPAL runtime.
//! The owner publishes immutable pictures and PCM; only the worker touches PAL.
use crate::native_vm::{
    scene::{rasterize_scene_rgba, FrameScene},
    system::PalWindowRequest,
    AudioConfig, Engine, EngineConfig, InputEvent, MouseButton, PalEvent, RuntimeStatus,
};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    service::{buttons, HostServices, InputFrame, PresentationScale, WindowGeometry},
    state::{ClientId, Color, Command, Rect, Sprite, Visual, VisualNode},
    vm::{VmAdapter, VmYield},
    Error,
};
use crysirror_runtime::{model::ClientSpec, runtime::VmFactory, AdapterRegistry};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{
        mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
        Arc,
    },
    thread,
    time::Duration,
};

const FRAME_ID: u32 = 0x5350_0000;
const MAX_EVENTS: usize = 4096;
const SAMPLE_RATE: u32 = 48_000;
const MAX_FRAME_PIXELS: u64 = 16 * 1024 * 1024;

/// Physical roots are injected by the desktop bootstrap, not authored by scripts.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub remote_root: Option<String>,
    pub resource_root: PathBuf,
    pub user_root: PathBuf,
    #[serde(default)]
    pub font_file: Option<PathBuf>,
    #[serde(default = "default_nls")]
    pub nls: String,
    #[serde(default = "default_lang")]
    pub lang: String,
    #[serde(default = "default_audio")]
    pub audio: bool,
    /// Portable host policies for PAL's four retained aspect-mode indices.
    /// The VM retains the raw mode (including mode 3's sprite positioning).
    /// Callers can choose a different output policy without changing the VM.
    #[serde(default = "default_aspect_modes")]
    pub aspect_modes: [PresentationScale; 4],
}
fn default_aspect_modes() -> [PresentationScale; 4] {
    [
        PresentationScale::Stretch,
        PresentationScale::Fit,
        PresentationScale::Center,
        PresentationScale::Fit,
    ]
}
fn default_nls() -> String {
    "sjis".into()
}
fn default_lang() -> String {
    "jp".into()
}
fn default_audio() -> bool {
    true
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.resource_root.is_absolute(),
            "SoftPAL resource_root must be absolute"
        );
        ensure!(
            self.user_root.is_absolute(),
            "SoftPAL user_root must be absolute"
        );
        if let Some(font) = &self.font_file {
            ensure!(font.is_absolute(), "SoftPAL font_file must be absolute");
        }
        self.nls
            .parse::<crate::native_asset::Nls>()
            .map_err(anyhow::Error::msg)?;
        self.lang
            .parse::<crate::native_vm::TextLang>()
            .map_err(anyhow::Error::msg)?;
        Ok(())
    }
}
struct Factory;
impl VmFactory for Factory {
    fn create(&self, client: ClientId, spec: &ClientSpec) -> Result<Box<dyn VmAdapter>> {
        ensure!(
            spec.program.is_empty(),
            "SoftPAL executes original Script.src"
        );
        Ok(Box::new(SoftpalVm::new(
            client,
            serde_json::from_value(spec.config.clone())?,
        )?))
    }
}
pub fn register(registry: &mut AdapterRegistry) -> Result<()> {
    registry.register("softpal", Arc::new(Factory))
}

struct Tick {
    delta_ms: u64,
    instruction_budget: u32,
    events: Vec<PalEvent>,
    geometry: Option<WindowGeometry>,
}
enum Request {
    Tick(Tick),
    Stop,
}
struct CompletedFrame {
    size: [u32; 2],
    rgba: Arc<[u8]>,
    pcm: Arc<[[f32; 2]]>,
    window: Vec<PalWindowRequest>,
    status: RuntimeStatus,
    index: u64,
}
enum Response {
    Ready([u32; 2]),
    Frame(CompletedFrame),
    Failed(String),
}

pub struct SoftpalVm {
    storage: Option<SyncSender<Arc<dyn crysirror_core::service::ClientStorage>>>,
    client: ClientId,
    aspect_modes: [PresentationScale; 4],
    requests: Option<SyncSender<Request>>,
    responses: Receiver<Response>,
    // The worker is detached on drop after disconnecting its bounded mailbox.
    // It can finish its current native slice without blocking the desktop owner.
    _worker: Option<thread::JoinHandle<()>>,
    preparing: bool,
    in_flight: bool,
    halted: bool,
    events: Vec<PalEvent>,
    previous_down: u64,
    previous_pointer: Option<[i32; 2]>,
    previous_focus: Option<bool>,
    last_dispatch: Option<u64>,
    last_input_sample: Option<(u64, InputFrame)>,
    size: [u32; 2],
    completed_frames: u64,
    status: String,
}
impl SoftpalVm {
    pub fn new(client: ClientId, config: Config) -> Result<Self> {
        config.validate()?;
        let aspect_modes = config.aspect_modes;
        let (storage,storage_rx)=mpsc::sync_channel(1);
        let (requests, incoming) = mpsc::sync_channel(1);
        let (outgoing, responses) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name(format!("softpal-{}", client.0))
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    worker(config, incoming, &outgoing, storage_rx)
                }));
                let failure = match result {
                    Ok(Ok(())) => None,
                    Ok(Err(error)) => Some(format!("{error:#}")),
                    Err(payload) => Some(
                        payload
                            .downcast_ref::<String>()
                            .cloned()
                            .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                            .unwrap_or_else(|| "native worker panic".to_owned()),
                    ),
                };
                if let Some(message) = failure {
                    let _ = outgoing.send(Response::Failed(message));
                }
            })
            .context("spawn SoftPAL VM owner")?;
        Ok(Self {
            client,
            aspect_modes,
            storage:Some(storage),
            requests: Some(requests),
            responses,
            _worker: Some(worker),
            preparing: true,
            in_flight: false,
            halted: false,
            events: Vec::new(),
            previous_down: 0,
            previous_pointer: None,
            previous_focus: None,
            last_dispatch: None,
            last_input_sample: None,
            size: [1280, 720],
            completed_frames: 0,
            status: "preparing".into(),
        })
    }
    fn event(&mut self, event: PalEvent) -> crysirror_core::Result<()> {
        // Pointer observations can replace only an adjacent pointer observation;
        // never overwrite an intervening press, release, wheel or focus edge.
        if matches!(event, PalEvent::Input(InputEvent::CursorMoved { .. }))
            && matches!(
                self.events.last(),
                Some(PalEvent::Input(InputEvent::CursorMoved { .. }))
            )
        {
            *self.events.last_mut().expect("last pointer event") = event;
            return Ok(());
        }
        if self.events.len() >= MAX_EVENTS {
            return Err(Error::Limit("SoftPAL pending input events"));
        }
        self.events.push(event);
        Ok(())
    }
    fn observe(
        &mut self,
        host: &dyn HostServices,
        input: InputFrame,
    ) -> crysirror_core::Result<()> {
        let sample = (host.now_ms(), input);
        if self.last_input_sample == Some(sample) {
            return Ok(());
        }
        self.last_input_sample = Some(sample);
        let pointer = [input.pointer_x, input.pointer_y];
        if self.previous_pointer != Some(pointer) {
            self.event(PalEvent::Input(InputEvent::CursorMoved {
                x: pointer[0] as f64,
                y: pointer[1] as f64,
            }))?;
            self.previous_pointer = Some(pointer);
        }
        let changed = self.previous_down ^ input.buttons_down;
        for &(name, bit) in buttons::NAMES {
            if bit & (buttons::WHEEL_UP | buttons::WHEEL_DOWN) != 0 {
                continue;
            }
            let down = input.buttons_down & bit != 0;
            if changed & bit != 0 {
                self.button(name, down)?;
            } else if input.buttons_pressed & bit != 0 {
                // Explicit pulses while released are delivered as a full pair.
                // An already-held pulse is a repeat press, never a release.
                self.button(name, true)?;
                if !down {
                    self.button(name, false)?;
                }
            }
        }
        self.previous_down = input.buttons_down;
        let wheel = i32::from(input.buttons_pressed & buttons::WHEEL_UP != 0)
            - i32::from(input.buttons_pressed & buttons::WHEEL_DOWN != 0);
        if wheel != 0 {
            self.event(PalEvent::Input(InputEvent::MouseWheel {
                delta_x: 0.,
                delta_y: wheel as f32,
            }))?;
        }
        if let Some(focused) = host.window_focused(self.client)? {
            if self.previous_focus != Some(focused) {
                self.event(PalEvent::Focused { focused })?;
                self.previous_focus = Some(focused);
            }
        }
        Ok(())
    }
    fn button(&mut self, name: &str, pressed: bool) -> crysirror_core::Result<()> {
        let mouse = match name {
            "mouse_left" => Some(MouseButton::Left),
            "mouse_right" => Some(MouseButton::Right),
            "mouse_middle" => Some(MouseButton::Middle),
            _ => None,
        };
        let input = if let Some(button) = mouse {
            InputEvent::MouseInput { button, pressed }
        } else {
            let key_name = match name {
                "enter" => "Enter".into(),
                "space" => "Space".into(),
                "left" => "ArrowLeft".into(),
                "right" => "ArrowRight".into(),
                "up" => "ArrowUp".into(),
                "down" => "ArrowDown".into(),
                "tab" => "Tab".into(),
                "shift" => "Shift".into(),
                "ctrl" => "Control".into(),
                "escape" => "Escape".into(),
                "backspace" => "Backspace".into(),
                "home" => "Home".into(),
                "end" => "End".into(),
                "page_up" => "PageUp".into(),
                "page_down" => "PageDown".into(),
                _ => name.to_ascii_uppercase(),
            };
            InputEvent::Keyboard { key_name, pressed }
        };
        self.event(PalEvent::Input(input))
    }
    fn publish(
        &mut self,
        host: &mut dyn HostServices,
        frame: CompletedFrame,
    ) -> crysirror_core::Result<()> {
        if self.size != frame.size {
            host.request_canvas_size(self.client, frame.size)?;
            self.size = frame.size;
        }
        let path = host.present_surface(
            self.client,
            FRAME_ID,
            frame.size[0],
            frame.size[1],
            &frame.rgba,
        )?;
        host.submit(
            self.client,
            &[Command::PutVisual {
                id: FRAME_ID,
                node: VisualNode {
                    z: 0,
                    visual: Visual::Sprite(Sprite {
                        asset: path.clone(),
                        bounds: Rect {
                            x: 0,
                            y: 0,
                            width: frame.size[0],
                            height: frame.size[1],
                        },
                        tint: Color([255; 4]),
                    }),
                },
            }],
        )?;
        if !frame.pcm.is_empty() {
            host.submit_pcm(self.client, SAMPLE_RATE, &frame.pcm)?;
        }
        for request in frame.window {
            let command = match request {
                PalWindowRequest::ChangeMode { mode } => {
                    serde_json::json!({"op":"set_fullscreen","enabled":mode!=0})
                }
                PalWindowRequest::ChangeSize { width, height } => {
                    if width <= 0 || height <= 0 {
                        return Err(Error::Invalid("SoftPAL requested window dimensions"));
                    }
                    serde_json::json!({"op":"set_window_size","size":[width,height]})
                }
                PalWindowRequest::SetPos { x, y } => {
                    serde_json::json!({"op":"set_window_position","position":[x,y]})
                }
                PalWindowRequest::ChangeAspect { mode } => {
                    let index = usize::try_from(mode)
                        .map_err(|_| Error::Invalid("SoftPAL aspect mode index"))?;
                    let scale = self
                        .aspect_modes
                        .get(index)
                        .ok_or(Error::Invalid("SoftPAL aspect mode index"))?;
                    serde_json::json!({"op":"set_presentation_scale","mode":scale})
                }
            };
            let bytes = serde_json::to_vec(&serde_json::json!({"ui":[command]}))
                .map_err(|e| Error::Io(e.to_string()))?;
            host.submit_engine(self.client, 1, &bytes)?;
        }
        self.completed_frames = frame.index;
        self.status = frame.status.to_string();
        self.halted = matches!(frame.status, RuntimeStatus::Halted { .. });
        Ok(())
    }
}
impl VmAdapter for SoftpalVm {
    fn preparation_pending(&self) -> bool {
        self.preparing
    }
    fn engine_id(&self) -> &'static str {
        "softpal"
    }
    fn snapshot_schema(&self) -> u32 {
        1
    }
    fn step(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        instruction_budget: u32,
    ) -> crysirror_core::Result<VmYield> {
        if let Some(sender)=self.storage.take(){sender.try_send(host.storage(self.client)?).map_err(|_|Error::Io("SoftPAL resource capability delivery failed".into()))?;}
        self.observe(host, input)?;
        match self.responses.try_recv() {
            Ok(Response::Ready(size)) => {
                self.preparing = false;
                self.size = size;
                host.request_canvas_size(self.client, size)?;
                self.status = "ready".into();
                // Preparation polling does not execute the first normal slice.
                return Ok(VmYield::BudgetExhausted);
            }
            Ok(Response::Frame(frame)) => {
                self.in_flight = false;
                self.publish(host, frame)?;
            }
            Ok(Response::Failed(message)) => return Err(Error::Io(format!("SoftPAL: {message}"))),
            Err(TryRecvError::Disconnected) if !self.halted => {
                return Err(Error::Io("SoftPAL worker disconnected".into()))
            }
            Err(_) => {}
        }
        if self.halted {
            return Ok(VmYield::Halted);
        }
        let now = host.now_ms();
        if !self.preparing && !self.in_flight && self.last_dispatch != Some(now) {
            let tick = Tick {
                delta_ms: self
                    .last_dispatch
                    .map_or(16, |previous| now.saturating_sub(previous))
                    .min(100),
                instruction_budget,
                events: std::mem::take(&mut self.events),
                geometry: host.window_geometry(self.client)?,
            };
            let request = Request::Tick(tick);
            match self
                .requests
                .as_ref()
                .ok_or_else(|| Error::Io("SoftPAL worker stopped".into()))?
                .try_send(request)
            {
                Ok(()) => {
                    self.in_flight = true;
                    self.last_dispatch = Some(now);
                }
                Err(TrySendError::Full(Request::Tick(tick))) => {
                    self.events = tick.events;
                }
                Err(_) => return Err(Error::Io("SoftPAL worker mailbox disconnected".into())),
            }
        }
        Ok(VmYield::WaitUntil(now.saturating_add(1)))
    }
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Err(Error::Unsupported(
            "SoftPAL framework snapshot; original game saves remain native",
        ))
    }
    fn restore(&mut self, _: &[u8]) -> crysirror_core::Result<()> {
        Err(Error::Unsupported("SoftPAL framework snapshot"))
    }
    fn diagnostics(&self) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({"engine":"softpal","preparing":self.preparing,"worker_pending":self.in_flight,"completed_frames":self.completed_frames,"size":self.size,"status":self.status})).unwrap_or_default()
    }
}
impl Drop for SoftpalVm {
    fn drop(&mut self) {
        if let Some(sender) = self.requests.take() {
            let _ = sender.try_send(Request::Stop);
        }
    }
}
fn dimensions(size: [u32; 2]) -> Result<()> {
    ensure!(
        size[0] > 0
            && size[1] > 0
            && size[0] <= 8192
            && size[1] <= 8192
            && u64::from(size[0]) * u64::from(size[1]) <= MAX_FRAME_PIXELS,
        "SoftPAL frame dimensions exceed host budget"
    );
    Ok(())
}
fn worker(
    config: Config,
    requests: Receiver<Request>,
    responses: &SyncSender<Response>,
    storage_rx:Receiver<Arc<dyn crysirror_core::service::ClientStorage>>,
) -> Result<()> {
    let storage=storage_rx.recv().context("resource capability disconnected")?;
    #[cfg(any(feature="nfs",feature="smb"))]
    let _mount=if let Some(prefix)=&config.remote_root {Some(game_network::fs::Mount::from_directory(game_network::Directory::from_client_storage(storage,prefix)?,&config.user_root)?)}else{None};
    #[cfg(not(any(feature="nfs",feature="smb")))]
    ensure!(config.remote_root.is_none(),"enable SoftPAL nfs or smb feature for remote resources");
    let game_root = config
        .resource_root
        .canonicalize()
        .context("SoftPAL resource_root")?;
    #[cfg(any(feature="nfs",feature="smb"))]
    let game_root=_mount.as_ref().map_or(game_root.clone(),|mount|mount.path().to_path_buf());
    ensure!(
        crate::native_asset::vfs::directory_exists(&game_root),
        "SoftPAL resource_root is not a directory"
    );
    std::fs::create_dir_all(&config.user_root).context("SoftPAL user_root")?;
    let save_root = config.user_root.canonicalize()?;
    ensure!(
        !save_root.starts_with(&game_root),
        "SoftPAL writable user_root must be outside resource_root"
    );
    crate::native_vm::host_paths::install(save_root);
    crate::native_vm::font::install_host_fallback(config.font_file.as_deref())?;
    let mut engine = Engine::new(EngineConfig {
        game_root: Some(game_root),
        nls: config.nls.parse().map_err(anyhow::Error::msg)?,
        lang: config.lang.parse().map_err(anyhow::Error::msg)?,
        font_file: config.font_file,
        audio: AudioConfig {
            enabled: config.audio,
        },
        ..EngineConfig::default()
    })?;
    ensure!(
        !config.audio || engine.audio().is_enabled(),
        "SoftPAL requested audio backend did not initialize"
    );
    // CPU composition and host PCM are explicit; no original GPU or device.
    engine.set_host_gpu_renderer(false);
    engine.set_host_touch_available(false);
    let (width, height) = engine.logical_size_from_config(
        FrameScene::PAL_DEFAULT_WIDTH,
        FrameScene::PAL_DEFAULT_HEIGHT,
    );
    dimensions([width, height])?;
    engine.handle_event(PalEvent::Resized { width, height });
    if responses.send(Response::Ready([width, height])).is_err() {
        return Ok(());
    }
    let sink = engine.audio().host_sink();
    let mut fullscreen = false;
    while let Ok(request) = requests.recv() {
        let Request::Tick(tick) = request else {
            break;
        };
        if let Some(geometry) = tick.geometry {
            let [x, y] = geometry.outer_position.unwrap_or([0, 0]);
            engine.observe_window_geometry(
                x,
                y,
                geometry.inner_size[0] as i32,
                geometry.inner_size[1] as i32,
                fullscreen,
            );
        }
        for event in tick.events {
            engine.handle_event(event);
        }
        engine.set_host_instruction_budget(tick.instruction_budget.max(1) as usize);
        let frame = engine.update_with_delta(Duration::from_millis(tick.delta_ms))?;
        match &frame.runtime_status {
            RuntimeStatus::UnsupportedCommand { .. }
            | RuntimeStatus::UnsupportedExtCall { .. }
            | RuntimeStatus::Faulted { .. } => {
                anyhow::bail!("{}", frame.runtime_status);
            }
            RuntimeStatus::NotBooted => anyhow::bail!("SoftPAL native script did not boot"),
            _ => {}
        }
        let size = [frame.scene.logical_width, frame.scene.logical_height];
        dimensions(size)?;
        let rgba: Arc<[u8]> = rasterize_scene_rgba(&frame.scene).into();
        let sample_frames = (frame.timing.delta.as_millis() as usize).min(100) * 48;
        let mut samples = vec![0i16; if config.audio { sample_frames * 2 } else { 0 }];
        sink.render_i16(&mut samples);
        let pcm: Arc<[[f32; 2]]> = samples
            .chunks_exact(2)
            .map(|s| [s[0] as f32 / 32768., s[1] as f32 / 32768.])
            .collect::<Vec<_>>()
            .into();
        let window = engine.take_window_requests();
        for request in &window {
            if let PalWindowRequest::ChangeMode { mode } = request {
                fullscreen = *mode != 0;
            }
        }
        let halted = matches!(frame.runtime_status, RuntimeStatus::Halted { .. });
        let complete = CompletedFrame {
            size,
            rgba,
            pcm,
            window,
            status: frame.runtime_status,
            index: frame.timing.frame_index,
        };
        engine.input_begin_frame();
        engine.handle_event(PalEvent::Resized {
            width: size[0],
            height: size[1],
        });
        if responses.send(Response::Frame(complete)).is_err() || halted {
            break;
        }
    }
    Ok(())
}

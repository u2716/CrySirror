//! Device-free owner bridge for the complete BGI session.
use crate::native_app::host_runtime::{HostConfig, HostEvent, HostFrame, HostRuntime};
use anyhow::{Context, Result, bail, ensure};
use crysirror_core::{
    Error,
    service::{HostServices, InputFrame, buttons},
    state::{ClientId, Color, Command, Rect, Sprite, Visual, VisualNode},
    vm::{VmAdapter, VmYield},
};
use crysirror_runtime::{AdapterRegistry, model::ClientSpec, runtime::VmFactory};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{Arc, mpsc},
};
#[path = "adapter/dialog.rs"]
mod dialog;
const FRAME: u32 = 0xb610_0000;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub root: String,
    pub script: String,
    #[serde(default)]
    pub font: Option<String>,
    /// Canonical physical capability roots injected by the owning desktop host.
    pub resource_root: PathBuf,
    pub user_root: PathBuf,
    #[serde(default)]
    pub font_file: Option<PathBuf>,
    #[serde(default)]
    pub font_face_index: u32,
    #[serde(default)]
    pub gbk: bool,
    #[serde(default)]
    pub trace: bool,
    #[serde(default)]
    pub silent: bool,
}
impl Config {
    fn validate(&self) -> Result<()> {
        ensure!(
            self.resource_root.is_absolute() && self.user_root.is_absolute(),
            "BGI requires host-owned absolute resource/user roots"
        );
        ensure!(
            !self.script.is_empty() && self.script.len() <= 4096 && !self.script.contains('\0'),
            "BGI native bootstrap name"
        );
        for name in self.script.split(':') {
            ensure!(
                crysirror_core::path::VfsPath::new(crysirror_core::path::Mount::Asset, name)?
                    .relative()
                    == name,
                "BGI canonical bootstrap path"
            );
        }
        if !self.root.is_empty() {
            ensure!(
                crysirror_core::path::VfsPath::new(crysirror_core::path::Mount::Asset, &self.root)?
                    .relative()
                    == self.root,
                "BGI canonical relative root"
            );
        }
        Ok(())
    }
}
struct Factory;
impl VmFactory for Factory {
    fn create(&self, client: ClientId, spec: &ClientSpec) -> Result<Box<dyn VmAdapter>> {
        ensure!(spec.program.is_empty(), "BGI executes native BP programs");
        let config: Config = serde_json::from_value(spec.config.clone())?;
        config.validate()?;
        Ok(Box::new(IntegratedVm::new(client, config)?))
    }
}
pub fn register(registry: &mut AdapterRegistry) -> Result<()> {
    registry.register("bgi", Arc::new(Factory))
}
enum WorkerCommand {
    Tick {
        now_ns: u64,
        inputs: Vec<InputFrame>,
        surface_size: Option<[u32; 2]>,
    },
    Request {
        id: u64,
        result: i32,
    },
    Dialog {
        id: u64,
        response: String,
    },
    Close,
    Stop,
}
struct Event {
    native: HostEvent,
    payload: Option<String>,
}
struct Completed {
    frame: Option<HostFrame>,
    pcm: Vec<[f32; 2]>,
    events: Vec<Event>,
    size: [u32; 2],
    closed: bool,
    diagnostics: String,
}
enum WorkerOutput {
    Ready,
    Frame(Completed),
    Failed(String),
}
struct Worker {
    commands: mpsc::SyncSender<WorkerCommand>,
    output: mpsc::Receiver<WorkerOutput>,
}
impl Worker {
    fn new(config: Config) -> Result<Self> {
        let (commands, incoming) = mpsc::sync_channel(4);
        let (outgoing, output) = mpsc::sync_channel(2);
        std::thread::Builder::new()
            .name("crysirror-bgi-session".into())
            .spawn(move || {
                // All original state, including non-Send VM/Flash state, is born,
                // driven and dropped on this worker. Only immutable products leave.
                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<()> {
                        let base = config.resource_root.canonicalize()?;
                        let resource_root = base.join(&config.root).canonicalize()?;
                        ensure!(
                            resource_root.starts_with(&base),
                            "BGI resource root escapes its host capability"
                        );
                        let mut session = HostRuntime::create_with_bootstrap(
                            HostConfig {
                                resource_root,
                                user_root: config.user_root,
                                font_path: config.font_file,
                                font_data: None,
                                font_face_index: config.font_face_index,
                                gbk: config.gbk,
                                backend: 0,
                                strict: true,
                                trace: config.trace,
                                silent: config.silent,
                            },
                            &config.script,
                        )?;
                        outgoing
                            .send(WorkerOutput::Ready)
                            .map_err(|_| anyhow::anyhow!("BGI owner disconnected"))?;
                        let mut previous = InputFrame::default();
                        let mut native_clock: Option<u64> = None;
                        while let Ok(command) = incoming.recv() {
                            match command {
                                WorkerCommand::Tick {
                                    now_ns,
                                    inputs,
                                    surface_size,
                                } => {
                                    if let Some([w, h]) = surface_size {
                                        session.resize_surface(w, h)?;
                                    }
                                    for input in inputs {
                                        feed_input(&mut session, &mut previous, input)?;
                                    }
                                    // The owner can advance while a native pass
                                    // is preparing. Catch up in bounded slices,
                                    // draining each PCM block before the next;
                                    // retain the time debt rather than dropping
                                    // samples or admitting an unbounded block.
                                    let step_time = native_clock.map_or(now_ns, |last| {
                                        now_ns.min(last.saturating_add(100_000_000))
                                    });
                                    session.step(step_time)?;
                                    native_clock = Some(step_time);
                                    let mut events = Vec::new();
                                    while let Some(native) = session.poll_event() {
                                        let payload = if native.kind == 5 {
                                            session.request_payload(native.request_id)
                                        } else {
                                            None
                                        };
                                        events.push(Event { native, payload });
                                        ensure!(events.len() <= 4096, "BGI host event budget");
                                    }
                                    let completed = Completed {
                                        frame: session.take_frame(),
                                        pcm: session.take_pcm(),
                                        events,
                                        size: session.logical_size(),
                                        closed: session.is_closed(),
                                        diagnostics: session.diagnostics(),
                                    };
                                    if outgoing.send(WorkerOutput::Frame(completed)).is_err() {
                                        break;
                                    }
                                }
                                WorkerCommand::Request { id, result } => {
                                    session.complete_request(id, result)?
                                }
                                WorkerCommand::Dialog { id, response } => {
                                    session.complete_dialog(id, &response)?
                                }
                                WorkerCommand::Close => session.request_close()?,
                                WorkerCommand::Stop => break,
                            }
                        }
                        Ok(())
                    }));
                let error = match result {
                    Ok(Ok(())) => None,
                    Ok(Err(error)) => Some(format!("{error:#}")),
                    Err(payload) => Some(format!(
                        "BGI session panic: {}",
                        payload
                            .downcast_ref::<String>()
                            .map(String::as_str)
                            .or_else(|| payload.downcast_ref::<&str>().copied())
                            .unwrap_or("non-string panic")
                    )),
                };
                if let Some(error) = error {
                    let _ = outgoing.send(WorkerOutput::Failed(error));
                }
            })
            .context("start BGI session worker")?;
        Ok(Self { commands, output })
    }
}
fn feed_input(
    runtime: &mut HostRuntime,
    previous: &mut InputFrame,
    input: InputFrame,
) -> Result<()> {
    let x = input.pointer_x as f32;
    let y = input.pointer_y as f32;
    if [input.pointer_x, input.pointer_y] != [previous.pointer_x, previous.pointer_y] {
        runtime.input(1, x, y, 0, 0.)?;
    }
    for code in 1..=0x7b {
        let Some(bit) = buttons::windows_virtual_key(code) else {
            continue;
        };
        let was = previous.buttons_down & bit != 0;
        let down = input.buttons_down & bit != 0;
        let pressed = input.buttons_pressed & bit != 0;
        let mouse = matches!(code, 1 | 2 | 4);
        let descriptor = if code == 4 { 3 } else { code as i32 };
        if (!was && down) || pressed {
            runtime.input(if mouse { 2 } else { 4 }, x, y, descriptor, 0.)?;
        }
        if (was && !down) || (pressed && !down) {
            runtime.input(if mouse { 3 } else { 5 }, x, y, descriptor, 0.)?;
        }
    }
    if input.buttons_pressed & buttons::WHEEL_UP != 0 {
        runtime.input(6, x, y, 0, 1.)?;
    }
    if input.buttons_pressed & buttons::WHEEL_DOWN != 0 {
        runtime.input(6, x, y, 0, -1.)?;
    }
    *previous = input;
    Ok(())
}
struct IntegratedVm {
    client: ClientId,
    config: Config,
    worker: Worker,
    ready: bool,
    in_flight: bool,
    halted: bool,
    close_requested: bool,
    inputs: Vec<InputFrame>,
    controls: VecDeque<WorkerCommand>,
    events: VecDeque<Event>,
    size: [u32; 2],
    canvas_initialized: bool,
    input: InputFrame,
    last_input_sample: Option<(u64, InputFrame)>,
    diagnostics: String,
    dialog: Option<dialog::Dialog>,
}
impl IntegratedVm {
    fn new(client: ClientId, config: Config) -> Result<Self> {
        Ok(Self {
            client,
            worker: Worker::new(config.clone())?,
            config,
            ready: false,
            in_flight: false,
            halted: false,
            close_requested: false,
            inputs: Vec::new(),
            controls: VecDeque::new(),
            events: VecDeque::new(),
            size: [1280, 720],
            canvas_initialized: false,
            input: InputFrame::default(),
            last_input_sample: None,
            diagnostics: String::new(),
            dialog: None,
        })
    }
    fn submit(&self, host: &mut dyn HostServices, value: Value) -> Result<()> {
        host.submit_engine(self.client, 1, &serde_json::to_vec(&value)?)?;
        Ok(())
    }
    fn poll(&mut self, host: &mut dyn HostServices) -> Result<()> {
        loop {
            match self.worker.output.try_recv() {
                Ok(WorkerOutput::Ready) => self.ready = true,
                Ok(WorkerOutput::Failed(error)) => bail!("complete BGI runtime: {error}"),
                Ok(WorkerOutput::Frame(completed)) => {
                    self.in_flight = false;
                    if !self.canvas_initialized || self.size != completed.size {
                        host.request_canvas_size(self.client, completed.size)?;
                        self.canvas_initialized = true;
                    }
                    self.size = completed.size;
                    self.halted = completed.closed;
                    self.diagnostics = completed.diagnostics;
                    if let Some(frame) = completed.frame {
                        let path = host.present_surface(
                            self.client,
                            FRAME,
                            frame.width,
                            frame.height,
                            &frame.rgba,
                        )?;
                        host.submit(
                            self.client,
                            &[Command::PutVisual {
                                id: FRAME,
                                node: VisualNode {
                                    z: 0,
                                    visual: Visual::Sprite(Sprite {
                                        asset: path,
                                        bounds: Rect {
                                            x: 0,
                                            y: 0,
                                            width: frame.width,
                                            height: frame.height,
                                        },
                                        tint: Color([255; 4]),
                                    }),
                                },
                            }],
                        )?;
                    }
                    if !completed.pcm.is_empty() {
                        host.submit_pcm(self.client, 48_000, &completed.pcm)?;
                    }
                    ensure!(
                        self.events.len().saturating_add(completed.events.len()) <= 4096,
                        "BGI pending event budget"
                    );
                    self.events.extend(completed.events);
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) if self.halted => break,
                Err(mpsc::TryRecvError::Disconnected) => bail!("BGI session worker disconnected"),
            }
        }
        Ok(())
    }
    fn host_events(&mut self, host: &mut dyn HostServices) -> Result<()> {
        if self.dialog.is_some() {
            self.poll_dialog(host)?;
        }
        while self.dialog.is_none() {
            let Some(event) = self.events.pop_front() else {
                break;
            };
            let native = event.native;
            match native.kind {
                1 => tracing::info!(code=native.code,message=%native.text,"BGI runtime"),
                2 | 5 => {
                    self.open_dialog(host, native, event.payload)?;
                }
                3 => self.halted = true,
                4 => self.submit(host, json!({"ui":[{"op":"set_title","text":native.text}]}))?,
                6 => self.submit(
                    host,
                    json!({"ui":[{"op":"set_cursor_visible","visible":native.code!=0}]}),
                )?,
                7 => {
                    let position: [f32; 2] = serde_json::from_str(&native.text)?;
                    self.submit(host,json!({"ui":[{"op":"warp_pointer","position":[position[0] as i32,position[1] as i32]}]}))?;
                }
                8 => self.submit(host, serde_json::from_str(&native.text)?)?,
                _ => bail!("BGI unknown host event {}", native.kind),
            }
        }
        Ok(())
    }
}
impl VmAdapter for IntegratedVm {
    fn engine_id(&self) -> &'static str {
        "bgi"
    }
    fn snapshot_schema(&self) -> u32 {
        1
    }
    fn preparation_pending(&self) -> bool {
        !self.ready
    }
    fn step(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        _instruction_budget: u32,
    ) -> crysirror_core::Result<VmYield> {
        let mut run = || -> Result<VmYield> {
            self.input = input;
            let was_ready = self.ready;
            self.poll(host)?;
            self.host_events(host)?;
            if self.halted {
                return Ok(VmYield::Halted);
            }
            if self.close_requested {
                self.controls.push_back(WorkerCommand::Close);
                self.close_requested = false;
            }
            let sample = (host.now_ms(), input);
            if self.last_input_sample != Some(sample) {
                self.last_input_sample = Some(sample);
                // Idle owner redraws are not native input events. Preserve all
                // edges and pulses, but do not fill the mailbox with repeated
                // held-state observations while a resource pass is pending.
                if input.buttons_pressed != 0
                    || self.inputs.last().is_none_or(|last| {
                        last.buttons_down != input.buttons_down
                            || last.pointer_x != input.pointer_x
                            || last.pointer_y != input.pointer_y
                    })
                {
                    ensure!(self.inputs.len() < 4096, "BGI pending input frame budget");
                    self.inputs.push(input);
                }
            }
            if self.ready && was_ready && !self.in_flight {
                while let Some(control) = self.controls.pop_front() {
                    self.worker
                        .commands
                        .try_send(control)
                        .map_err(|_| anyhow::anyhow!("BGI command worker disconnected"))?;
                }
                // Only one frame is outstanding. The owner never waits for a
                // VM pass, CPU compositor, decoder, filesystem or GPU readback.
                self.worker
                    .commands
                    .try_send(WorkerCommand::Tick {
                        now_ns: host
                            .now_ms()
                            .checked_mul(1_000_000)
                            .context("BGI host clock")?,
                        inputs: std::mem::take(&mut self.inputs),
                        surface_size: host
                            .window_geometry(self.client)?
                            .map(|geometry| geometry.inner_size),
                    })
                    .map_err(|error| anyhow::anyhow!("BGI worker admission: {error}"))?;
                self.in_flight = true;
            }
            Ok(VmYield::WaitUntil(host.now_ms().saturating_add(1)))
        };
        run().map_err(|error| Error::Io(format!("BGI: {error:#}")))
    }
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Err(Error::Unsupported(
            "complete BGI runtime snapshots; use native game saves",
        ))
    }
    fn restore(&mut self, _bytes: &[u8]) -> crysirror_core::Result<()> {
        Err(Error::Unsupported(
            "complete BGI runtime snapshots; use native game saves",
        ))
    }
    fn diagnostics(&self) -> Vec<u8> {
        serde_json::to_vec(&json!({"runtime":"bgi-integrated","ready":self.ready,"worker_pending":self.in_flight,"native":self.diagnostics})).unwrap_or_default()
    }
    fn request_close(&mut self) -> bool {
        self.close_requested = true;
        true
    }
}
impl Drop for IntegratedVm {
    fn drop(&mut self) {
        let _ = self.worker.commands.try_send(WorkerCommand::Stop);
    }
}

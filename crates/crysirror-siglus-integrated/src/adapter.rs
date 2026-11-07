// SPDX-License-Identifier: MPL-2.0
//! Full runtime session ownership and immutable host products.
use crate::native_vm::{
    host::{SiglusHost, SiglusHostConfig},
    render::Renderer,
    runtime::{
        input::VmMouseButton,
        native_ui::{NativeMessageBoxRequest, NativeUiBackend},
    },
};
use anyhow::{Context, Result, bail, ensure};
use crysirror_core::{
    Error,
    path::{Mount, VfsPath},
    service::{EngineQuery, HostServices, InputFrame, buttons},
    state::{ClientId, Color, Command, Rect, Sprite, Visual, VisualNode},
    vm::{VmAdapter, VmYield},
};
use crysirror_runtime::{AdapterRegistry, model::ClientSpec, runtime::VmFactory};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
};
const KEY_CODES: &[(u64, i32)] = &[
    (buttons::ENTER, 0x0d),
    (buttons::SPACE, 0x20),
    (buttons::LEFT, 0x25),
    (buttons::RIGHT, 0x27),
    (buttons::UP, 0x26),
    (buttons::DOWN, 0x28),
    (buttons::TAB, 0x09),
    (buttons::SHIFT, 0x10),
    (buttons::CTRL, 0x11),
    (buttons::ESCAPE, 0x1b),
    (buttons::BACKSPACE, 0x08),
    (buttons::HOME, 0x24),
    (buttons::END, 0x23),
    (buttons::PAGE_UP, 0x21),
    (buttons::PAGE_DOWN, 0x22),
    (buttons::INSERT, 0x2d),
    (buttons::KEY_W, 0x57),
    (buttons::KEY_A, 0x41),
    (buttons::KEY_S, 0x53),
    (buttons::KEY_D, 0x44),
    (buttons::KEY_E, 0x45),
    (buttons::KEY_Q, 0x51),
    (buttons::KEY_F, 0x46),
    (buttons::KEY_R, 0x52),
    (buttons::KEY_C, 0x43),
    (buttons::KEY_L, 0x4c),
    (buttons::KEY_Z, 0x5a),
    (buttons::KEY_0, 0x30),
    (buttons::KEY_1, 0x31),
    (buttons::KEY_2, 0x32),
    (buttons::KEY_3, 0x33),
    (buttons::KEY_4, 0x34),
    (buttons::KEY_5, 0x35),
    (buttons::KEY_6, 0x36),
    (buttons::KEY_7, 0x37),
    (buttons::KEY_8, 0x38),
    (buttons::KEY_9, 0x39),
    (buttons::F1, 0x70),
    (buttons::F2, 0x71),
    (buttons::F3, 0x72),
    (buttons::F4, 0x73),
    (buttons::F5, 0x74),
    (buttons::F6, 0x75),
    (buttons::F7, 0x76),
    (buttons::F8, 0x77),
    (buttons::F9, 0x78),
    (buttons::F10, 0x79),
    (buttons::F11, 0x7a),
    (buttons::F12, 0x7b),
];
const FRAME: u32 = 0x5100_0000;
const DIALOG: u32 = FRAME + 1;
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
/// Host-injected configuration for the integrated Siglus runtime.
pub struct Config {
    pub resource_root: PathBuf,
    pub user_root: PathBuf,
    #[serde(default = "scene_default")]
    pub scene: String,
    #[serde(default = "game_default")]
    pub gameexe: String,
    #[serde(default)]
    pub key_file: Option<String>,
    #[serde(default)]
    pub font_file: Option<PathBuf>,
    #[serde(default)]
    pub font_face_index: u32,
    #[serde(default)]
    pub font: Option<String>,
}
fn scene_default() -> String {
    "Scene.pck".into()
}
fn game_default() -> String {
    "Gameexe.dat".into()
}
struct Factory;
impl VmFactory for Factory {
    fn create(&self, client: ClientId, spec: &ClientSpec) -> Result<Box<dyn VmAdapter>> {
        ensure!(
            spec.program.is_empty(),
            "Siglus integrated runtime executes native scenes"
        );
        let config: Config = serde_json::from_value(spec.config.clone())?;
        ensure!(
            config.resource_root.is_absolute() && config.user_root.is_absolute(),
            "Siglus requires host-owned resource/save roots"
        );
        for name in [&config.scene, &config.gameexe] {
            VfsPath::new(Mount::Asset, name)?;
        }
        Ok(Box::new(IntegratedVm::new(client, config)?))
    }
}
pub fn register(registry: &mut AdapterRegistry) -> Result<()> {
    registry.register("siglus", Arc::new(Factory))
}
enum Control {
    Tick {
        now: u64,
        inputs: Vec<InputFrame>,
        focused: bool,
    },
    Dialog {
        request: u64,
        value: i64,
    },
    Stop,
}
struct Product {
    frame: Option<crate::native_vm::assets::RgbaImage>,
    pcm: Vec<[f32; 2]>,
    requests: Vec<NativeMessageBoxRequest>,
    halted: bool,
    diagnostic: String,
}
enum Output {
    Ready([u32; 2]),
    Frame(Product),
    Failed(String),
}
struct Ui(Arc<Mutex<VecDeque<NativeMessageBoxRequest>>>);
impl NativeUiBackend for Ui {
    fn show_system_messagebox(&self, request: NativeMessageBoxRequest) {
        let mut queue = self.0.lock().expect("Siglus host UI queue poisoned");
        assert!(queue.len() < 16, "Siglus host UI queue budget");
        queue.push_back(request);
    }
}
struct Worker {
    commands: mpsc::SyncSender<Control>,
    output: mpsc::Receiver<Output>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Worker {
    fn new(config: Config) -> Result<Self> {
        let (commands, incoming) = mpsc::sync_channel(2);
        let (outgoing, output) = mpsc::sync_channel(2);
        let thread = std::thread::Builder::new()
            .name("crysirror-siglus-session".into())
            .spawn(move || {
                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<()> {
                        let root = config.resource_root.canonicalize()?;
                        ensure!(root.is_dir(), "Siglus resource root");
                        let read = |name: &str, limit: u64| -> Result<Vec<u8>> {
                            let path = root.join(name).canonicalize()?;
                            ensure!(
                                path.starts_with(&root),
                                "Siglus bootstrap path escapes root"
                            );
                            ensure!(
                                path.metadata()?.len() <= limit,
                                "Siglus bootstrap size budget"
                            );
                            Ok(std::fs::read(path)?)
                        };
                        let material = if let Some(key) = &config.key_file {
                            crate::key_toml::parse_key_toml(std::str::from_utf8(&read(
                                key, 65536,
                            )?)?)?
                        } else {
                            Default::default()
                        };
                        let material = crate::decode_material::resolve(
                            &read(&config.gameexe, 16 * 1024 * 1024)?,
                            &read(&config.scene, 64 * 1024 * 1024)?,
                            material,
                        )?;
                        crate::host_scope::install(crate::host_scope::Scope {
                            resource_root: root.clone(),
                            user_root: config.user_root.clone(),
                            font: config.font_file.clone(),
                            font_face_index: config.font_face_index,
                            material,
                        });
                        let native = SiglusHostConfig::new(root);
                        let (w, h) = SiglusHost::resolve_initial_size(&native);
                        let renderer = pollster::block_on(Renderer::new_offscreen(w, h))?;
                        let mut session = SiglusHost::new_with_renderer_sync(native, renderer)?;
                        let ui = Arc::new(Mutex::new(VecDeque::new()));
                        session.install_host_ui(Arc::new(Ui(ui.clone())));
                        outgoing
                            .send(Output::Ready([w, h]))
                            .map_err(|_| anyhow::anyhow!("Siglus owner disconnected"))?;
                        let (mut previous, mut pointer, mut last_time) = (0u64, None, None);
                        let mut global = Some(session.persist_global_if_changed(None));
                        while let Ok(control) = incoming.recv() {
                            match control {
                                Control::Stop => break,
                                Control::Dialog { request, value } => {
                                    session.submit_native_messagebox_result(request, value)
                                }
                                Control::Tick {
                                    now,
                                    inputs,
                                    focused,
                                } => {
                                    session.host_focus(focused);
                                    for input in inputs {
                                        let down = input.buttons_down as u64;
                                        let pressed =
                                            input.buttons_pressed as u64 | (down & !previous);
                                        let released = previous & !down;
                                        for &(bit, vk) in KEY_CODES {
                                            if pressed & bit != 0 {
                                                session.key_down_code(vk);
                                            }
                                            if released & bit != 0 {
                                                session.key_up_code(vk);
                                            }
                                        }
                                        if pointer != Some([input.pointer_x, input.pointer_y]) {
                                            session.mouse_move(
                                                input.pointer_x as f64,
                                                input.pointer_y as f64,
                                            );
                                            pointer = Some([input.pointer_x, input.pointer_y]);
                                        }
                                        for (bit, button) in [
                                            (buttons::MOUSE_LEFT, VmMouseButton::Left),
                                            (buttons::MOUSE_RIGHT, VmMouseButton::Right),
                                            (buttons::MOUSE_MIDDLE, VmMouseButton::Middle),
                                        ] {
                                            if pressed & bit != 0 {
                                                session.mouse_down(button);
                                            }
                                            if released & bit != 0 {
                                                session.mouse_up(button);
                                            }
                                        }
                                        if pressed & buttons::WHEEL_UP != 0 {
                                            session.mouse_wheel(1);
                                        }
                                        if pressed & buttons::WHEEL_DOWN != 0 {
                                            session.mouse_wheel(-1);
                                        }
                                        // A press/release within one host sample still needs
                                        // a native release on the next sample.
                                        previous = down | pressed;
                                    }
                                    let dt = last_time
                                        .map(|last: u64| now.saturating_sub(last))
                                        .unwrap_or(0)
                                        .min(1000);
                                    last_time = Some(now);
                                    let halted = session.step(dt as u32)?;
                                    global = Some(session.persist_global_if_changed(global));
                                    let frame = session.renderer_mut().take_host_frame();
                                    let pcm = session.render_host_pcm(dt as usize * 48)?;
                                    let requests = ui
                                        .lock()
                                        .map_err(|_| anyhow::anyhow!("Siglus UI queue poisoned"))?
                                        .drain(..)
                                        .collect();
                                    outgoing
                                        .send(Output::Frame(Product {
                                            frame,
                                            pcm,
                                            requests,
                                            halted,
                                            diagnostic: session.host_diagnostics(),
                                        }))
                                        .map_err(|_| {
                                            anyhow::anyhow!("Siglus owner disconnected")
                                        })?;
                                }
                            }
                        }
                        Ok(())
                    }));
                let error = match result {
                    Ok(Ok(())) => None,
                    Ok(Err(e)) => Some(format!("{e:#}")),
                    Err(_) => Some("Siglus session panicked".into()),
                };
                if let Some(error) = error {
                    let _ = outgoing.send(Output::Failed(error));
                }
            })?;
        Ok(Self {
            commands,
            output,
            thread: Some(thread),
        })
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.commands.send(Control::Stop);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
struct IntegratedVm {
    client: ClientId,
    worker: Worker,
    ready: bool,
    pending: bool,
    halted: bool,
    size: [u32; 2],
    inputs: Vec<InputFrame>,
    last_sample: Option<(u64, InputFrame)>,
    diagnostic: String,
    dialog: Option<NativeMessageBoxRequest>,
    requests: VecDeque<NativeMessageBoxRequest>,
    font: String,
}
impl IntegratedVm {
    fn new(client: ClientId, config: Config) -> Result<Self> {
        let font = config.font.clone().unwrap_or_else(|| "default".into());
        Ok(Self {
            client,
            worker: Worker::new(config)?,
            ready: false,
            pending: false,
            halted: false,
            size: [1, 1],
            inputs: Vec::new(),
            last_sample: None,
            diagnostic: String::new(),
            dialog: None,
            requests: VecDeque::new(),
            font,
        })
    }
    fn submit(&self, host: &mut dyn HostServices, value: Value) -> Result<()> {
        host.submit_engine(self.client, 1, &serde_json::to_vec(&value)?)?;
        Ok(())
    }
    fn poll(&mut self, host: &mut dyn HostServices) -> Result<()> {
        loop {
            match self.worker.output.try_recv() {
                Ok(Output::Ready(size)) => {
                    self.ready = true;
                    self.size = size;
                }
                Ok(Output::Failed(error)) => bail!("{error}"),
                Ok(Output::Frame(product)) => {
                    self.pending = false;
                    self.halted = product.halted;
                    self.diagnostic = product.diagnostic;
                    self.requests.extend(product.requests);
                    if let Some(frame) = product.frame {
                        self.size = [frame.width, frame.height];
                        let asset = host.present_surface(
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
                                        asset,
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
                    if !product.pcm.is_empty() {
                        host.submit_pcm(self.client, 48000, &product.pcm)?;
                    }
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) if self.halted => break,
                Err(_) => bail!("Siglus session disconnected"),
            }
        }
        Ok(())
    }
    fn ui(&mut self, host: &mut dyn HostServices) -> Result<()> {
        if let Some(dialog) = self.dialog.as_ref() {
            for (i, button) in dialog.buttons.iter().enumerate() {
                let state: Value = serde_json::from_slice(&host.query_engine(
                    self.client,
                    EngineQuery::UiInteraction(DIALOG + 10 + i as u32),
                )?)?;
                if state["activated"].as_bool() == Some(true) {
                    self.worker
                        .commands
                        .try_send(Control::Dialog {
                            request: dialog.request_id,
                            value: button.value,
                        })
                        .context("Siglus dialog reply")?;
                    self.submit(host, json!({"ui":[{"op":"remove","id":DIALOG}]}))?;
                    self.dialog = None;
                    break;
                }
            }
        }
        if self.dialog.is_none() {
            if let Some(request) = self.requests.pop_front() {
                ensure!(request.buttons.len() <= 16, "Siglus dialog button budget");
                let w = self.size[0].saturating_sub(48).min(800).max(32);
                let x = self.size[0].saturating_sub(w) / 2;
                let mut ui = vec![
                    json!({"op":"put","widget":{"id":DIALOG,"rect":[0,0,self.size[0],self.size[1]],"modal":true,"color":[0,0,0,160],"z":2147479000,"keyboard_focusable":false}}),
                    json!({"op":"put","widget":{"id":DIALOG+1,"parent":DIALOG,"rect":[x,32,w,320],"color":[36,36,44,255],"keyboard_focusable":false}}),
                    json!({"op":"put","widget":{"id":DIALOG+2,"parent":DIALOG+1,"kind":"label","rect":[16,12,w.saturating_sub(32),48],"font":self.font,"text":request.title}}),
                    json!({"op":"put","widget":{"id":DIALOG+3,"parent":DIALOG+1,"kind":"label","rect":[16,64,w.saturating_sub(32),160],"font":self.font,"text":request.message}}),
                ];
                for (i, button) in request.buttons.iter().enumerate() {
                    ui.push(json!({"op":"put","widget":{"id":DIALOG+10+i as u32,"parent":DIALOG+1,"kind":"button","rect":[16+i as u32*120,260,112,40],"font":self.font,"text":button.label}}));
                }
                self.submit(host, json!({"ui":ui}))?;
                self.dialog = Some(request);
            }
        }
        Ok(())
    }
}
impl VmAdapter for IntegratedVm {
    fn engine_id(&self) -> &'static str {
        "siglus"
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
        _budget: u32,
    ) -> crysirror_core::Result<VmYield> {
        let mut run = || -> Result<VmYield> {
            let was_ready = self.ready;
            self.poll(host)?;
            self.ui(host)?;
            if self.halted {
                return Ok(VmYield::Halted);
            }
            let sample = (host.now_ms(), input);
            if self.last_sample != Some(sample) {
                self.last_sample = Some(sample);
                if input.buttons_pressed != 0 || self.inputs.last().is_none_or(|p| *p != input) {
                    ensure!(self.inputs.len() < 4096, "Siglus pending input budget");
                    self.inputs.push(input);
                }
            }
            if self.ready && was_ready && !self.pending {
                let inputs = if self.dialog.is_some() {
                    self.inputs.clear();
                    vec![InputFrame::default()]
                } else {
                    std::mem::take(&mut self.inputs)
                };
                self.worker
                    .commands
                    .try_send(Control::Tick {
                        now: host.now_ms(),
                        inputs,
                        focused: host.window_focused(self.client)?.unwrap_or(true),
                    })
                    .context("Siglus frame admission")?;
                self.pending = true;
            }
            Ok(VmYield::WaitUntil(host.now_ms().saturating_add(1)))
        };
        run().map_err(|e| Error::Io(format!("Siglus integrated: {e:#}")))
    }
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Err(Error::Unsupported(
            "Siglus integrated snapshots; use native saves",
        ))
    }
    fn restore(&mut self, _: &[u8]) -> crysirror_core::Result<()> {
        Err(Error::Unsupported(
            "Siglus integrated snapshots; use native saves",
        ))
    }
    fn diagnostics(&self) -> Vec<u8> {
        serde_json::to_vec(&json!({"runtime":"siglus-integrated","ready":self.ready,"worker_pending":self.pending,"native":self.diagnostic})).unwrap_or_default()
    }
}

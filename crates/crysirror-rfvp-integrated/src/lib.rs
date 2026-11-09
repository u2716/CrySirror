//! Integrated RFVP: complete native composition/mixing on one session owner.
#![forbid(unsafe_code)]
use anyhow::{Context, Result, bail, ensure};
use crysirror_core::{
    Error,
    path::{Mount, VfsPath},
    service::{
        ClientStorage, HostServices, InputFrame, MediaPhase, MediaRequest, MediaSourceOptions,
        buttons,
    },
    state::{ClientId, Color, Command, Rect, Sprite, Visual, VisualNode},
    vm::{VmAdapter, VmYield},
};
use crysirror_rfvp_native as rfvp;
use crysirror_runtime::{AdapterRegistry, model::ClientSpec, runtime::VmFactory};
use rfvp::host_api::*;
use rfvp::{RfvpBootConfig, RfvpCore, RfvpCoreConfig};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{Arc, mpsc},
};
mod audio;
use audio::Audio;
pub use crysirror_rfvp::Nls;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub root: String,
    #[serde(default)]
    pub nls: Nls,
    pub default_font: String,
    #[serde(default)]
    pub screen: Option<[u32; 2]>,
    /// Set only by a host that has mounted a remote asset capability.
    #[serde(default)]
    pub remote: bool,
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        crysirror_rfvp::Config {
            root: self.root.clone(),
            nls: self.nls.clone(),
            default_font: self.default_font.clone(),
            screen: self.screen,
        }
        .validate()?;
        if let Some([w, h]) = self.screen {
            ensure!(
                u64::from(w) * u64::from(h) <= 8_388_608,
                "RFVP native frame pixel budget"
            );
        }
        #[cfg(not(any(feature = "nfs", feature = "smb")))]
        ensure!(
            !self.remote,
            "enable RFVP nfs or smb feature for remote resources"
        );
        Ok(())
    }
}
struct Factory;
impl VmFactory for Factory {
    fn create(&self, id: ClientId, spec: &ClientSpec) -> Result<Box<dyn VmAdapter>> {
        ensure!(spec.program.is_empty(), "RFVP executes native HCB");
        Ok(Box::new(RfvpVm::new(
            id,
            serde_json::from_value(spec.config.clone())?,
        )?))
    }
}
pub fn register(registry: &mut AdapterRegistry) -> Result<()> {
    registry.register("rfvp", Arc::new(Factory))
}
const FRAME: u32 = 0x52000000;
struct NativeHost {
    root: String,
    default_font: String,
    storage: Arc<dyn ClientStorage>,
    now_us: u64,
    renderer: SoftRaster,
    audio: Audio,
}
impl NativeHost {
    fn asset(&self, path: &str) -> RfvpResult<VfsPath> {
        let path = path.replace('\\', "/");
        let path = path.strip_prefix("./").unwrap_or(&path);
        let rel = if path.eq_ignore_ascii_case("default.ttf") {
            self.default_font.clone()
        } else if self.root.is_empty() {
            path.to_owned()
        } else {
            format!("{}/{path}", self.root)
        };
        VfsPath::new(Mount::Asset, &rel).map_err(|_| RfvpError::InvalidArgument)
    }
    fn capability(&self) -> RfvpResult<&Arc<dyn ClientStorage>> {
        Ok(&self.storage)
    }
}
/// A file opened from the capability storage.
pub struct AssetFile(Arc<dyn crysirror_core::service::AssetReader>);
impl RfvpFile for AssetFile {
    fn len(&mut self) -> RfvpResult<u64> {
        self.0.len().map_err(|_| RfvpError::Io)
    }
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> RfvpResult<usize> {
        self.0.read_at(offset, buf).map_err(|_| RfvpError::Io)
    }
}

/// Saves under the save mount (`rfvp/<game path>`).
struct SaveStorage(Arc<dyn ClientStorage>);
impl SaveStorage {
    fn path(path: &str) -> RfvpResult<VfsPath> {
        let path = path.replace('\\', "/");
        let path = path.trim_start_matches("./").trim_start_matches('/');
        VfsPath::new(Mount::Save, &format!("rfvp/{path}")).map_err(|_| RfvpError::InvalidArgument)
    }
}
impl RfvpStorage for SaveStorage {
    fn read(&self, path: &str) -> RfvpResult<Vec<u8>> {
        self.0
            .read(&Self::path(path)?)
            .map_err(|_| RfvpError::NotFound)
    }
    fn write(&self, path: &str, bytes: &[u8]) -> RfvpResult<()> {
        self.0
            .write_atomic(&Self::path(path)?, bytes)
            .map_err(|_| RfvpError::Io)
    }
    fn exists(&self, path: &str) -> bool {
        Self::path(path).is_ok_and(|p| self.0.exists(&p))
    }
    fn remove(&self, path: &str) -> RfvpResult<()> {
        self.0.remove(&Self::path(path)?).map_err(|_| RfvpError::Io)
    }
    fn list(&self, dir: &str) -> RfvpResult<Vec<String>> {
        let dir = dir.replace('\\', "/");
        let dir = dir.trim_matches('/');
        let rel = if dir.is_empty() {
            "rfvp".to_owned()
        } else {
            format!("rfvp/{dir}")
        };
        self.0
            .list(Mount::Save, &rel)
            .map_err(|_| RfvpError::NotFound)
    }
}

impl RfvpFileSystem for NativeHost {
    type File = AssetFile;
    fn open(&mut self, path: &str) -> RfvpResult<AssetFile> {
        let p = self.asset(path)?;
        let reader = self
            .capability()?
            .open_asset(&p)
            .map_err(|_| RfvpError::NotFound)?;
        Ok(AssetFile(reader))
    }
    fn storage(&mut self) -> Option<Arc<dyn RfvpStorage>> {
        Some(Arc::new(SaveStorage(self.storage.clone())))
    }
    fn game_files(&mut self) -> Option<Arc<dyn RfvpStorage>> {
        Some(Arc::new(GameStorage {
            storage: self.storage.clone(),
            root: self.root.clone(),
        }))
    }
    fn metadata(&mut self, path: &str) -> RfvpResult<RfvpFileInfo> {
        let p = self.asset(path)?;
        let storage = self.capability()?;
        let meta = storage
            .metadata(Mount::Asset, p.relative())
            .map_err(|_| RfvpError::Io)?
            .ok_or(RfvpError::NotFound)?;
        Ok(if meta.is_directory {
            RfvpFileInfo {
                len: 0,
                kind: rfvp::host_api::RfvpFileKind::Directory,
            }
        } else {
            RfvpFileInfo::file(meta.size.unwrap_or(0))
        })
    }
    fn enumerate_by_extension(
        &mut self,
        root: &str,
        extension_without_dot: &str,
        visitor: &mut dyn FnMut(&str, RfvpFileInfo) -> RfvpResult<()>,
    ) -> RfvpResult<()> {
        let dir = self.asset(if root.is_empty() || root == "." {
            "."
        } else {
            root
        });
        let dir = match dir {
            Ok(d) => d.relative().to_owned(),
            Err(_) => self.root.clone(),
        };
        let dir = if root.is_empty() || root == "." {
            self.root.clone()
        } else {
            dir
        };
        let storage = self.capability()?.clone();
        let names = storage
            .list(Mount::Asset, &dir)
            .map_err(|_| RfvpError::NotFound)?;
        let ext = format!(".{}", extension_without_dot.to_ascii_lowercase());
        for name in names {
            if name.to_ascii_lowercase().ends_with(&ext) {
                let rel = if dir.is_empty() {
                    name.clone()
                } else {
                    format!("{dir}/{name}")
                };
                let len = storage
                    .metadata(Mount::Asset, &rel)
                    .ok()
                    .flatten()
                    .and_then(|m| m.size)
                    .unwrap_or(0);
                visitor(&name, RfvpFileInfo::file(len))?;
            }
        }
        Ok(())
    }
}

struct GameStorage {
    storage: Arc<dyn ClientStorage>,
    root: String,
}
impl GameStorage {
    fn path(&self, path: &str) -> RfvpResult<VfsPath> {
        let path = path.replace('\\', "/");
        let path = path.strip_prefix("./").unwrap_or(&path);
        VfsPath::new(
            Mount::Asset,
            &if self.root.is_empty() {
                path.into()
            } else {
                format!("{}/{path}", self.root)
            },
        )
        .map_err(|_| RfvpError::InvalidArgument)
    }
}
impl RfvpStorage for GameStorage {
    fn read(&self, p: &str) -> RfvpResult<Vec<u8>> {
        let mut file = self.open(p)?;
        file.read_to_vec(256 * 1024 * 1024)
    }
    fn read_prefix(&self, p: &str, n: usize) -> RfvpResult<Vec<u8>> {
        let mut file = self.open(p)?;
        let mut buf = vec![0; n.min(file.len()? as usize)];
        file.read_exact_at(0, &mut buf)?;
        Ok(buf)
    }
    fn open(&self, p: &str) -> RfvpResult<Box<dyn RfvpFile + Send>> {
        Ok(Box::new(AssetFile(
            self.storage
                .open_asset(&self.path(p)?)
                .map_err(|_| RfvpError::NotFound)?,
        )))
    }
    fn write(&self, _: &str, _: &[u8]) -> RfvpResult<()> {
        Err(RfvpError::Unsupported)
    }
    fn remove(&self, _: &str) -> RfvpResult<()> {
        Err(RfvpError::Unsupported)
    }
    fn exists(&self, p: &str) -> bool {
        self.path(p).is_ok_and(|p| self.storage.exists(&p))
    }
    fn list(&self, p: &str) -> RfvpResult<Vec<String>> {
        let rel = if p.is_empty() {
            self.root.clone()
        } else {
            self.path(p)?.relative().into()
        };
        self.storage
            .list(Mount::Asset, &rel)
            .map_err(|_| RfvpError::Io)
    }
}
impl RfvpClock for NativeHost {
    fn ticks_us(&mut self) -> u64 {
        self.now_us
    }
    fn wall_time(&mut self) -> Option<WallTime> {
        use chrono::{Datelike, Timelike};
        let t = chrono::Local::now();
        Some(WallTime {
            year: t.year() as u16,
            month: t.month() as u8,
            day: t.day() as u8,
            day_of_week: t.weekday().num_days_from_sunday() as u8,
            hour: t.hour() as u8,
            minute: t.minute() as u8,
        })
    }
}
impl RfvpHost for NativeHost {
    type FileSystem = Self;
    type Renderer = SoftRaster;
    type Audio = Audio;
    type Clock = Self;
    fn fs(&mut self) -> &mut Self {
        self
    }
    fn renderer(&mut self) -> &mut SoftRaster {
        &mut self.renderer
    }
    fn audio(&mut self) -> &mut Audio {
        &mut self.audio
    }
    fn clock(&mut self) -> &mut Self {
        self
    }
    fn log(&mut self, level: RfvpLogLevel, msg: &str) {
        if matches!(level, RfvpLogLevel::Error | RfvpLogLevel::Warn) {
            eprintln!("RFVP {level:?}: {msg}");
        }
    }
}
struct Session {
    core: RfvpCore,
    host: NativeHost,
    size: [u32; 2],
    last_down: u64,
    last_pointer: [i32; 2],
    cursors: BTreeMap<u32, String>,
    cursor: Option<u32>,
}
impl Drop for Session {
    fn drop(&mut self) {
        self.core.shutdown(&mut self.host);
    }
}
impl Session {
    fn push_input(&mut self, input: InputFrame) -> Result<()> {
        let pointer = [input.pointer_x, input.pointer_y];
        let in_screen = pointer[0] >= 0
            && pointer[1] >= 0
            && (pointer[0] as u32) < self.size[0]
            && (pointer[1] as u32) < self.size[1];
        let mut events = Vec::new();
        if pointer != self.last_pointer {
            events.push(RfvpEvent::PointerMove {
                x: pointer[0],
                y: pointer[1],
                in_screen,
            });
            self.last_pointer = pointer;
        }
        let down = input.buttons_down | input.buttons_pressed;
        let pressed = (down & !self.last_down) | input.buttons_pressed;
        let released = self.last_down & !input.buttons_down;
        let modifiers = if down & buttons::CTRL != 0 {
            InputModifiers::CONTROL
        } else {
            InputModifiers::empty()
        };
        for (bit, button) in [
            (buttons::MOUSE_LEFT, PointerButton::Left),
            (buttons::MOUSE_RIGHT, PointerButton::Right),
        ] {
            if pressed & bit != 0 {
                events.push(RfvpEvent::PointerDown {
                    button,
                    x: pointer[0],
                    y: pointer[1],
                });
            }
            if released & bit != 0 {
                events.push(RfvpEvent::PointerUp {
                    button,
                    x: pointer[0],
                    y: pointer[1],
                });
            }
        }
        for (bit, key) in [
            (buttons::ENTER, KeyCode::Return),
            (buttons::SPACE, KeyCode::Space),
            (buttons::ESCAPE, KeyCode::Escape),
            (buttons::LEFT, KeyCode::Left),
            (buttons::RIGHT, KeyCode::Right),
            (buttons::UP, KeyCode::Up),
            (buttons::DOWN, KeyCode::Down),
            (buttons::CTRL, KeyCode::Control),
            (buttons::SHIFT, KeyCode::Shift),
            (buttons::TAB, KeyCode::Tab),
            (buttons::BACKSPACE, KeyCode::Backspace),
            (buttons::F1, KeyCode::Function(1)),
            (buttons::F2, KeyCode::Function(2)),
            (buttons::F3, KeyCode::Function(3)),
            (buttons::F4, KeyCode::Function(4)),
            (buttons::F5, KeyCode::Function(5)),
            (buttons::F6, KeyCode::Function(6)),
            (buttons::F7, KeyCode::Function(7)),
            (buttons::F8, KeyCode::Function(8)),
            (buttons::F9, KeyCode::Function(9)),
            (buttons::F10, KeyCode::Function(10)),
            (buttons::F11, KeyCode::Function(11)),
            (buttons::F12, KeyCode::Function(12)),
        ] {
            if pressed & bit != 0 {
                events.push(RfvpEvent::KeyDown {
                    key,
                    repeat: false,
                    modifiers,
                });
            }
            if released & bit != 0 {
                events.push(RfvpEvent::KeyUp { key, modifiers });
            }
        }
        for (bit, delta_y) in [(buttons::WHEEL_UP, 1), (buttons::WHEEL_DOWN, -1)] {
            if input.buttons_pressed & bit != 0 {
                events.push(RfvpEvent::Wheel {
                    delta_x: 0,
                    delta_y,
                });
            }
        }
        self.last_down = down;
        for event in events {
            self.core
                .push_event(event)
                .map_err(|e| anyhow::anyhow!("RFVP input: {e:?}"))?;
        }
        Ok(())
    }

    fn cursors(&mut self) -> Result<Vec<Value>> {
        let mut ops = Vec::new();
        for name in self.host.storage.list(Mount::Asset, &self.host.root)? {
            let Some((stem, ext)) = name.rsplit_once('.') else {
                continue;
            };
            if !ext.eq_ignore_ascii_case("ani") {
                continue;
            }
            let prefix = stem.trim_end_matches(|c: char| c.is_ascii_digit());
            if prefix.is_empty()
                || prefix == stem
                || !prefix.bytes().all(|c| c.is_ascii_alphabetic() || c == b'_')
            {
                continue;
            }
            let Some(id) = stem[prefix.len()..]
                .parse::<u32>()
                .ok()
                .filter(|i| *i > 0 && *i < 256)
            else {
                continue;
            };
            ensure!(
                !self.cursors.contains_key(&id),
                "ambiguous RFVP cursor slot"
            );
            let mut file = self.host.open(&name).map_err(native_error)?;
            let bytes = file.read_to_vec(16 * 1024 * 1024).map_err(native_error)?;
            let cursor = crysirror_image::cursor::decode_cursor(&bytes)?;
            let frames:Vec<_>=cursor.frames.into_iter().map(|f|json!({"width":f.image.width,"height":f.image.height,"hotspot":f.hotspot,"rgba":f.image.rgba})).collect();
            let steps: Vec<_> = cursor
                .steps
                .into_iter()
                .map(|(i, ms)| [i as u32, ms])
                .collect();
            ops.push(
                json!({"op":"define_cursor","id":id,"cursor":{"frames":frames,"steps":steps}}),
            );
            self.cursors.insert(id, name);
        }
        self.core.initialize_cursor(self.cursors.contains_key(&1));
        Ok(ops)
    }
    fn cursor_ops(&mut self) -> Vec<Value> {
        let c = self.core.take_cursor_state();
        let mut ops = Vec::new();
        if self.cursor != Some(c.index) {
            let cursor = if self.cursors.contains_key(&c.index) {
                json!({"kind":"custom","id":c.index})
            } else {
                json!({"kind":"standard","shape":"default"})
            };
            ops.push(json!({"op":"set_cursor","cursor":cursor}));
            self.cursor = Some(c.index);
        }
        if let Some(v) = c.visible {
            ops.push(json!({"op":"set_cursor_visible","visible":v}));
        }
        if let Some(p) = c.position {
            ops.push(json!({"op":"warp_pointer","position":p}));
        }
        ops
    }
}
fn native_error(e: RfvpError) -> anyhow::Error {
    anyhow::anyhow!("RFVP native host: {e:?}")
}
struct Movie {
    epoch: u64,
    playing: bool,
    frame: Option<(u32, u32, Vec<u8>)>,
}
enum Control {
    Tick {
        now: u64,
        inputs: Vec<InputFrame>,
        focused: bool,
        movie: Option<Movie>,
    },
    Stop,
}
struct MovieOutput {
    command: rfvp::subsystem::resources::videoplayer::MovieCommand,
    reader: Option<Arc<dyn crysirror_core::service::AssetReader>>,
}
struct Product {
    rgba: Vec<u8>,
    pcm: Vec<[f32; 2]>,
    ui: Vec<Value>,
    movies: Vec<MovieOutput>,
    halted: bool,
    frames: u64,
}
enum Output {
    Ready(Vec<Value>),
    Frame(Product),
    Failed(String),
}
struct Worker {
    storage: Option<mpsc::SyncSender<Arc<dyn ClientStorage>>>,
    tx: mpsc::SyncSender<Control>,
    rx: mpsc::Receiver<Output>,
}
impl Worker {
    fn new(config: Config) -> Result<Self> {
        let (storage, srx) = mpsc::sync_channel::<Arc<dyn ClientStorage>>(1);
        let (tx, commands) = mpsc::sync_channel(1);
        let (products, rx) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("crysirror-rfvp-session".into())
            .spawn(move || {
                let run =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<()> {
                        let storage = srx
                            .recv()
                            .context("RFVP resource capability disconnected")?;
                        let size = config.screen.unwrap_or([1024, 640]);
                        let mut s = Session {
                            core: RfvpCore::new(RfvpCoreConfig {
                                virtual_width: size[0],
                                virtual_height: size[1],
                                max_pending_events: 4096,
                            }),
                            host: NativeHost {
                                root: config.root,
                                default_font: config.default_font,
                                storage,
                                now_us: 0,
                                renderer: SoftRaster::new(size[0], size[1]),
                                audio: Audio::new(),
                            },
                            size,
                            last_down: 0,
                            last_pointer: [-1, -1],
                            cursors: BTreeMap::new(),
                            cursor: None,
                        };
                        let nls = match config.nls {
                            Nls::Sjis => rfvp::script::parser::Nls::ShiftJIS,
                            Nls::Gbk => rfvp::script::parser::Nls::GBK,
                            Nls::Utf8 => rfvp::script::parser::Nls::UTF8,
                        };
                        s.core
                            .boot(
                                &mut s.host,
                                RfvpBootConfig {
                                    asset_root: ".",
                                    nls,
                                    ..Default::default()
                                },
                            )
                            .map_err(|e| {
                                anyhow::anyhow!(
                                    "RFVP boot: {e:?}: {}",
                                    s.core.last_error_detail().unwrap_or_default()
                                )
                            })?;
                        let ui = s.cursors()?;
                        products.send(Output::Ready(ui))?;
                        let mut last = None;
                        let mut last_focus = true;
                        while let Ok(command) = commands.recv() {
                            match command {
                                Control::Stop => {
                                    break;
                                }
                                Control::Tick {
                                    now,
                                    inputs,
                                    focused,
                                    movie,
                                } => {
                                    if focused != last_focus {
                                        s.core
                                            .push_event(if focused {
                                                RfvpEvent::FocusGained
                                            } else {
                                                RfvpEvent::FocusLost
                                            })
                                            .map_err(native_error)?;
                                        last_focus = focused;
                                    }
                                    let elapsed = last
                                        .replace(now)
                                        .map_or(0, |p: u64| now.saturating_sub(p).min(100));
                                    s.host.now_us = s.host.now_us.saturating_add(if focused {
                                        elapsed * 1000
                                    } else {
                                        0
                                    });
                                    if let Some(m) = movie {
                                        s.core
                                            .observe_movie(m.epoch, m.playing, m.frame)
                                            .map_err(native_error)?;
                                    }
                                    s.core.observe_audio_sources(&s.host.audio.observations());
                                    for input in inputs {
                                        s.push_input(if focused {
                                            input
                                        } else {
                                            InputFrame::default()
                                        })?;
                                    }
                                    if focused {
                                        s.core.tick(&mut s.host).map_err(|e| {
                                            anyhow::anyhow!(
                                                "RFVP tick: {e:?}: {}",
                                                s.core.last_error_detail().unwrap_or_default()
                                            )
                                        })?;
                                    }
                                    let pcm =
                                        s.host.audio.render(if focused { elapsed } else { 0 })?;
                                    let rgba = if focused {
                                        s.host
                                            .renderer
                                            .frame()
                                            .iter()
                                            .flat_map(|p| p.to_le_bytes())
                                            .collect()
                                    } else {
                                        Vec::new()
                                    };
                                    let mut movies=Vec::new();
                                    for command in s.core.drain_movie_commands(){
                                        let reader=match &command{rfvp::subsystem::resources::videoplayer::MovieCommand::Play{name,..}=>Some(audio::media_reader(s.core.open_game_stream(name).map_err(native_error)?).map_err(native_error)?),_=>None};
                                        movies.push(MovieOutput{command,reader});
                                    }
                                    let product = Product {
                                        rgba,
                                        pcm,
                                        ui: s.cursor_ops(),
                                        movies,
                                        halted: s.core.quit_requested(),
                                        frames: s.core.frame_index(),
                                    };
                                    products.send(Output::Frame(product))?;
                                }
                            }
                        }
                        Ok(())
                    }));
                let error = match run {
                    Ok(Ok(())) => None,
                    Ok(Err(e)) => Some(format!("{e:#}")),
                    Err(_) => Some("RFVP owner thread panicked".into()),
                };
                if let Some(error) = error {
                    let _ = products.send(Output::Failed(error));
                }
            })?;
        Ok(Self {
            storage: Some(storage),
            tx,
            rx,
        })
    }
}
pub struct RfvpVm {
    client: ClientId,
    config: Config,
    worker: Worker,
    ready: bool,
    pending: bool,
    halted: bool,
    inputs: Vec<InputFrame>,
    last: Option<(u64, InputFrame)>,
    last_dispatch: Option<u64>,
    movie: Option<(u64, u64)>,
    frames: u64,
}
impl RfvpVm {
    pub fn new(client: ClientId, config: Config) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            client,
            worker: Worker::new(config.clone())?,
            config,
            ready: false,
            pending: false,
            halted: false,
            inputs: Vec::new(),
            last: None,
            last_dispatch: None,
            movie: None,
            frames: 0,
        })
    }
    fn ui(&self, host: &mut dyn HostServices, ops: Vec<Value>) -> Result<()> {
        if !ops.is_empty() {
            host.submit_engine(self.client, 1, &serde_json::to_vec(&json!({"ui":ops}))?)?;
        }
        Ok(())
    }
    fn run(&mut self, host: &mut dyn HostServices, input: InputFrame) -> Result<VmYield> {
        if let Some(tx) = self.worker.storage.take() {
            tx.try_send(host.storage(self.client)?)
                .map_err(|_| anyhow::anyhow!("RFVP capability delivery"))?;
        }
        let was_ready = self.ready;
        loop {
            match self.worker.rx.try_recv() {
                Ok(Output::Ready(ui)) => {
                    self.ready = true;
                    self.ui(host, ui)?;
                }
                Ok(Output::Failed(e)) => bail!("{e}"),
                Ok(Output::Frame(p)) => {
                    self.pending = false;
                    self.halted = p.halted;
                    self.frames = p.frames;
                    self.ui(host, p.ui)?;
                    let [width, height] = self.config.screen.unwrap_or([1024, 640]);
                    if !p.rgba.is_empty() {
                        let asset =
                            host.present_surface(self.client, FRAME, width, height, &p.rgba)?;
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
                                            width,
                                            height,
                                        },
                                        tint: Color([255; 4]),
                                    }),
                                },
                            }],
                        )?;
                    }
                    if !p.pcm.is_empty() {
                        host.submit_pcm(self.client, 48000, &p.pcm)?;
                    }
                    for MovieOutput { command: m, reader } in p.movies {
                        use rfvp::subsystem::resources::videoplayer::MovieCommand;
                        match m {
                            MovieCommand::Play { epoch, name, audio } => {
                                let _ = name;
                                host.media_reader(
                                    self.client,
                                    0,
                                    reader.context("RFVP native Movie reader missing")?,
                                    MediaSourceOptions {
                                        looping: false,
                                        audio,
                                        gain: 1.,
                                    },
                                )?;
                                self.movie = Some((epoch, 0));
                            }
                            MovieCommand::Stop { epoch } => {
                                if self.movie.is_some_and(|(e, _)| e == epoch) {
                                    host.media_request(self.client, MediaRequest::Stop { id: 0 })?;
                                    self.movie = None;
                                }
                            }
                        }
                    }
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(_) if self.halted => break,
                Err(_) => bail!("RFVP owner disconnected"),
            }
        }
        if self.halted {
            return Ok(VmYield::Halted);
        }
        let now = host.now_ms();
        let sample = (now, input);
        if self.last != Some(sample) {
            self.last = Some(sample);
            if input.buttons_pressed != 0 || self.inputs.last().is_none_or(|p| *p != input) {
                ensure!(self.inputs.len() < 4096, "RFVP pending input budget");
                self.inputs.push(input);
            }
        }
        if self.ready && was_ready && !self.pending && self.last_dispatch != Some(now) {
            let movie = if let Some((epoch, revision)) = self.movie {
                let status = host.media_info(self.client, 0)?;
                ensure!(status.phase != MediaPhase::Failed, "RFVP Movie failed");
                let frame = if status.published_frames != revision {
                    let p = host
                        .media_frame(self.client, 0)?
                        .context("movie picture missing")?;
                    let [w, h] = p.size();
                    Some((w, h, p.rgba().to_vec()))
                } else {
                    None
                };
                let playing = matches!(status.phase, MediaPhase::Loading | MediaPhase::Playing);
                self.movie = playing.then_some((epoch, status.published_frames));
                Some(Movie {
                    epoch,
                    playing,
                    frame,
                })
            } else {
                None
            };
            self.worker.tx.try_send(Control::Tick {
                now,
                inputs: std::mem::take(&mut self.inputs),
                focused: host.window_focused(self.client)?.unwrap_or(true),
                movie,
            })?;
            self.pending = true;
            self.last_dispatch = Some(now);
        }
        Ok(VmYield::WaitUntil(now.saturating_add(1)))
    }
}
impl VmAdapter for RfvpVm {
    fn engine_id(&self) -> &'static str {
        "rfvp"
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
        budget: u32,
    ) -> crysirror_core::Result<VmYield> {
        if budget == 0 {
            return Ok(VmYield::BudgetExhausted);
        }
        self.run(host, input)
            .map_err(|e| Error::Io(format!("{e:#}")))
    }
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Err(Error::Unsupported(
            "RFVP framework snapshot; use original game saves",
        ))
    }
    fn restore(&mut self, _: &[u8]) -> crysirror_core::Result<()> {
        Err(Error::Unsupported("RFVP framework snapshot"))
    }
    fn diagnostics(&self) -> Vec<u8> {
        serde_json::to_vec(&json!({"engine":"rfvp","mode":"integrated","ready":self.ready,"worker_pending":self.pending,"frames":self.frames})).unwrap_or_default()
    }
}
impl Drop for RfvpVm {
    fn drop(&mut self) {
        let _ = self.worker.tx.try_send(Control::Stop);
    }
}

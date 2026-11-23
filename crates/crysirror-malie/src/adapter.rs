// SPDX-License-Identifier: MPL-2.0
//! Dormant Malie owner. Resource I/O/decoding is worker-owned; EXEC and UI VM
//! state stay on the session owner. Native frame IDs are logical handles.
//!
//! Script helpers address the MalieSystem frame (System_GetScreen(1000));
//! its callback 62CF80 is the message table implemented in [`MalieVm::system`].
use crate::{
    adv::{self, Adv, Attrs},
    message,
    program::Program,
    screen::{Action, Document, Screen},
    vm::{Call, Machine, Step},
};
use anyhow::{bail, ensure, Context, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{
        buttons, AudioAsset, MediaPhase, MediaRequest, AudioParameters, AudioPreparePhase, AudioRequest, ClientStorage,
        EngineQuery, FontHandle, FontJob, FontLibrary, FontLoadSource, FontLoadSpec,
        FontLoadState, HostServices, InputFrame,
    },
    state::ClientId,
    vm::{VmAdapter, VmYield},
    Error,
};
use crysirror_tasks::{Task, TaskPool};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::{BTreeMap, BTreeSet}, sync::Arc};
mod dialog;
mod natives;
mod saves;
mod tags;
use saves::{QUICK_SLOT, THUMB};
use crate::settings::{Settings, Volume};
use dialog::MessageBoxPurpose;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub executable: String,
    pub canvas: [u32; 2],
    pub font: String,
    pub title: Option<String>,
    #[serde(default)]
    pub font_source: Option<String>,
}
struct Boot {
    program: Arc<Program>,
    svg: String,
    source: String,
    ini: BTreeMap<String, BTreeMap<String, String>>,
    system_voice_csv: bool,
    nameconv: BTreeMap<String, String>,
    storage: Arc<dyn ClientStorage>,
    extra: ExtraData,
    system_voices: BTreeMap<String, String>,
}
fn read(storage: &dyn ClientStorage, path: &str, max: u64) -> Result<Vec<u8>> {
    let path = VfsPath::new(Mount::Asset, path)?;
    let reader = storage.open_read(&path)?;
    crysirror_resource::read_all(reader.as_ref(), max)
}
/// Game files for the script compiler (Asset mount, 64 MiB per file).
struct ScriptFiles<'a>(&'a dyn ClientStorage);
impl crate::compiler::Source for ScriptFiles<'_> {
    fn read(&mut self, path: &str) -> Result<Option<Vec<u8>>> {
        if VfsPath::new(Mount::Asset, path).is_err() {
            return Ok(None);
        }
        read_optional(self.0, path, 64 * 1024 * 1024)
    }
}
fn read_optional(storage: &dyn ClientStorage, path: &str, max: u64) -> Result<Option<Vec<u8>>> {
    let p = VfsPath::new(Mount::Asset, path)?;
    if storage.metadata(Mount::Asset, p.relative())?.is_none() {
        return Ok(None);
    }
    read(storage, path, max).map(Some)
}
/// Decode a shipped text file: UTF-16LE with BOM, UTF-8, else Shift-JIS.
fn decode_text(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xff, 0xfe]) {
        let units: Vec<u16> = bytes[2..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        return String::from_utf16_lossy(&units);
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => s.trim_start_matches('\u{feff}').to_owned(),
        Err(_) => encoding_rs::SHIFT_JIS.decode(bytes).0.into_owned(),
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Jump {
    Select,
    Chapter,
}
#[derive(Clone, Debug)]
enum Wait {
    None,
    Until(u64),
    /// `_ms_pause`/`_ms_page`: a click completes the reveal, the next resumes.
    Click,
    Layer(Attrs),
    Title,
    Select,
    /// `k`/`await` (6137D0) until a click or `event` resumes the VM.
    Suspended,
    /// 6137D0 without a click release (`screen` to an unregistered screen,
    /// ADV messages 1005 and 4/2001); only 6138F0 (ADV 4/2002, `screen
    /// src='adv'`, OpenDialog 441) resumes it.
    Halted,
    /// `voicewait`/`soundwait`: until the channel's source ends.
    Channel(u32),
    /// A native dialog/message box owns input.
    Dialog,
}
const BGM: u32 = 1;
const VOICE: u32 = 2;
const SYSTEM_VOICE: u32 = 3;
const SE_FIRST: u32 = 10;
const SE_COUNT: u32 = 16;
const ENV_FIRST: u32 = 40;
const SYSTEM_SE: u32 = 4;
pub struct MalieVm {
    client: ClientId,
    config: Config,
    pool: Option<TaskPool>,
    boot: Option<Task<Boot>>,
    machine: Option<Machine>,
    /// Title SVG and Squirrel source; each visit builds a fresh DOM/VM.
    title_data: Option<(String, String)>,
    screen: Option<Screen>,
    init: bool,
    executed: u64,
    wait: Wait,
    result: u32,
    error: Option<String>,
    audio: Vec<u64>,
    ini: BTreeMap<String, BTreeMap<String, String>>,
    fonts: Option<Arc<dyn FontLibrary>>,
    font_job: Option<FontJob>,
    font_handle: Option<FontHandle>,
    font_ready: bool,
    title_visits: u32,
    system_voice_csv: bool,
    nameconv: BTreeMap<String, String>,
    storage: Option<Arc<dyn ClientStorage>>,
    /// exec.dat, and the source module compiled by MalieSystem 120 (613800)
    /// when one replaced it.
    exec_program: Option<Arc<Program>>,
    module: Option<String>,
    adv: Option<Adv>,
    frame_doc: Option<(String, Document)>,
    tag_builder: Option<(String, Attrs)>,
    pending_voice: Option<String>,
    transition: Option<(Value, u64)>,
    imgblk: Option<(String, u64)>,
    se_next: u32,
    se_channels: BTreeMap<String, u32>,
    env_channels: BTreeMap<String, u32>,
    previous_down: u64,
    label: String,
    backlog: Vec<(String, String, Option<String>)>,
    bgm: Option<u32>,
    trace: bool,
    movie_ids: BTreeMap<String, u32>,
    tags: tags::TagState,
    after_wait: Option<String>,
    skip_active: bool,
    auto_active: bool,
    system_dirty: bool,
    music_heard: BTreeSet<u32>,
    delayed_se: Vec<(u64, u32, String, bool, f32)>,
    settings: Settings,
    system_voices: BTreeMap<String, String>,
    last_voice: Option<String>,
    autosave_requested: bool,
    dialog: Option<dialog::Dialog>,
    dialog_return: Wait,
    civil: Option<(i64, i32, u64)>,
    civil_job: Option<Box<dyn crysirror_core::service::CivilTimeQuery>>,
    system: saves::SystemData,
    seen_pictures: BTreeSet<String>,
    seen_movies: BTreeSet<String>,
    env_sources: BTreeMap<String, (String, f32)>,
    thumbnail: Option<Vec<u8>>,
    thumb_job: Option<crysirror_core::image_processing::ImageJob>,
    thumb_wanted: bool,
    pending_thumbnails: Vec<(usize, Vec<u8>)>,
    extra: ExtraData,
    cg_viewer: Option<(Vec<String>, usize)>,
    movie_viewer: Option<String>,
    movie_viewer_started: Option<String>,
    exit_requested: bool,
    replay: Option<String>,
    system_saved_at: u64,
    auto_since: Option<u64>,
    skip_mode: bool,
    rng: u32,
    csv: Vec<Option<Vec<Vec<String>>>>,
    unlinked: BTreeSet<String>,
    adv_frame_state: BTreeMap<u32, (u32, u32)>,
    /// Flag 886838 toggled by ADV message 1052.
    frame_flag_886838: bool,
    /// A click released by ADV messages 1001/1009 (6121A0, 612EF0).
    pending_click: bool,
    /// Active screen (4FA860): 300 ADV, 1000 MalieSystem, 12346 capture.
    active_screen: u32,
    /// Jump mode (883B18): skip with the ADV hidden until the next choice,
    /// or (883B20, `s_sceneskip`) the next `chapter` tag.
    jump: Option<Jump>,
    /// Choice snapshots (883B14 pending, 883B10 the last ten).
    choice_pending: Option<Box<saves::SaveRecord>>,
    choice_history: Vec<saves::SaveRecord>,
    /// Take the choice snapshot once the native call has returned.
    choice_snapshot_due: bool,
    string_vars: Vec<String>,
    current_cg: String,
    cg_params: (u32, u32),
    register_90d138: u32,
    mark_time: u64,
    bgm_loop_flag: bool,
    name_slots: Vec<String>,
    ini_section: String,
    font_position: u32,
    window_mode: u32,
    register_86c560: u32,
    int_vars: [u32; 32],
    se_loop_flag: bool,
    save_disable: u32,
    pending_save: Option<saves::PendingSave>,
    preview_surface: Option<String>,
}
/// data/extra/*.csv catalogues for the extra screens.
#[derive(Clone, Debug, Default)]
pub struct ExtraData {
    pub cg: Vec<Vec<String>>,
    pub scene: Vec<SceneEntry>,
    pub music: Vec<u32>,
    pub movie: Vec<String>,
}
#[derive(Clone, Debug)]
pub struct SceneEntry {
    pub label: String,
    pub caption: String,
    pub thumbnail: String,
}
impl ExtraData {
    fn parse(cg: &str, scene: &str, music: &str, movie: &str) -> Self {
        let rows = |t: &str| -> Vec<Vec<String>> {
            t.lines().map(|l| l.trim().split(',').map(|v| v.trim().to_owned()).collect::<Vec<_>>()).filter(|r| r.iter().any(|v| !v.is_empty())).collect()
        };
        Self {
            // category, then picture variants of one CG.
            cg: rows(cg).into_iter().map(|r| r.into_iter().skip(1).filter(|v| !v.is_empty()).map(|v| v.to_ascii_lowercase()).collect()).collect(),
            // category, label, id, thumbnail picture, caption.
            scene: rows(scene).into_iter().filter(|r| r.len() >= 5).map(|r| SceneEntry { label: r[1].clone(), thumbnail: r[3].to_ascii_lowercase(), caption: r[4].clone() }).collect(),
            music: rows(music).into_iter().filter_map(|r| r.first().and_then(|v| v.parse().ok())).collect(),
            movie: rows(movie).into_iter().filter_map(|r| r.first().map(|v| v.to_ascii_lowercase())).collect(),
        }
    }
}
impl MalieVm {
    pub fn new(client: ClientId, config: Config) -> Result<Self> {
        ensure!(
            config.canvas.iter().all(|v| (1..=8192).contains(v)),
            "Malie canvas"
        );
        VfsPath::new(Mount::Asset, &config.executable)?;
        Ok(Self {
            client,
            config,
            pool: None,
            boot: None,
            machine: None,
            title_data: None,
            screen: None,
            init: true,
            executed: 0,
            wait: Wait::None,
            result: 0,
            error: None,
            audio: Vec::new(),
            ini: BTreeMap::new(),
            fonts: None,
            font_job: None,
            font_handle: None,
            font_ready: false,
            title_visits: 0,
            system_voice_csv: false,
            nameconv: BTreeMap::new(),
            storage: None,
            exec_program: None,
            module: None,
            adv: None,
            frame_doc: None,
            tag_builder: None,
            pending_voice: None,
            transition: None,
            imgblk: None,
            se_next: 0,
            se_channels: BTreeMap::new(),
            env_channels: BTreeMap::new(),
            previous_down: 0,
            label: String::new(),
            backlog: Vec::new(),
            bgm: None,
            trace: std::env::var_os("MALIE_TRACE").is_some(),
            movie_ids: BTreeMap::new(),
            tags: tags::TagState::default(),
            after_wait: None,
            skip_active: false,
            auto_active: false,
            system_dirty: false,
            music_heard: BTreeSet::new(),
            delayed_se: Vec::new(),
            settings: Settings::default(),
            system_voices: BTreeMap::new(),
            last_voice: None,
            autosave_requested: false,
            dialog: None,
            dialog_return: Wait::None,
            civil: None,
            civil_job: None,
            system: saves::SystemData::default(),
            seen_pictures: BTreeSet::new(),
            seen_movies: BTreeSet::new(),
            env_sources: BTreeMap::new(),
            thumbnail: None,
            thumb_job: None,
            thumb_wanted: false,
            pending_thumbnails: Vec::new(),
            extra: ExtraData::default(),
            cg_viewer: None,
            movie_viewer: None,
            movie_viewer_started: None,
            exit_requested: false,
            replay: None,
            system_saved_at: 0,
            auto_since: None,
            skip_mode: false,
            rng: 1,
            csv: Vec::new(),
            unlinked: BTreeSet::new(),
            adv_frame_state: BTreeMap::new(),
            frame_flag_886838: false,
            pending_click: false,
            active_screen: 300,
            jump: None,
            choice_pending: None,
            choice_history: Vec::new(),
            choice_snapshot_due: false,
            string_vars: vec![String::new(); 10],
            current_cg: String::new(),
            cg_params: (0, 0),
            register_90d138: 0,
            mark_time: 0,
            bgm_loop_flag: true,
            name_slots: vec![String::new(); 64],
            ini_section: String::new(),
            font_position: 0,
            window_mode: 0,
            register_86c560: 0,
            int_vars: [0; 32],
            se_loop_flag: false,
            save_disable: 0,
            pending_save: None,
            preview_surface: None,
        })
    }
    pub(crate) fn volume(&self, v: Volume) -> f32 {
        self.settings.volume(v)
    }
    /// `[CONFIG] CHARAnn=<who>,<test voice>` maps a voice prefix to its slot.
    fn character_slot(&self, who: &str) -> Option<String> {
        let config = self.ini.get("config")?;
        for n in 1..=19 {
            let k = format!("chara{n:02}");
            if let Some(v) = config.get(&k) {
                if v.split(',').next().is_some_and(|w| w.trim().eq_ignore_ascii_case(who)) {
                    return Some(k);
                }
            }
        }
        // The last slot ("others") collects unlisted speakers.
        (1..=19).rev().map(|n| format!("chara{n:02}")).find(|k| config.get(k).is_some_and(|v| v.starts_with("others")))
    }
    fn voice_enabled(&self, who: &str) -> bool {
        self.character_slot(who).is_none_or(|k| *self.settings.checks.get(&k).unwrap_or(&true))
    }
    fn character_volume(&self, who: &str) -> f32 {
        self.character_slot(who).map_or(1., |k| *self.settings.values.get(&k).unwrap_or(&100) as f32 / 100.)
    }
    fn request_autosave(&mut self) {
        if self.settings.option("autosave") != "no" {
            self.autosave_requested = true;
        }
    }
    fn ini(&self, section: &str, key: &str) -> Option<&str> {
        self.ini.get(section)?.get(&key.to_ascii_lowercase()).map(String::as_str)
    }
    fn prepare(&mut self, h: &mut dyn HostServices) -> Result<bool> {
        if !self.font_ready {
            if let Some(path) = &self.config.font_source {
                if self.fonts.is_none() {
                    self.fonts = Some(h.font_library(self.client)?);
                }
                let library = self.fonts.as_ref().unwrap();
                if self.font_job.is_none() {
                    self.font_job = Some(library.begin(FontLoadSpec {
                        name: self.config.font.clone(),
                        source: FontLoadSource::Storage(VfsPath::new(Mount::Asset, path)?),
                        face_index: 0,
                    })?);
                }
                match library.status(self.font_job.unwrap())? {
                    FontLoadState::Pending => {}
                    FontLoadState::Failed(e) => bail!("Malie font preparation: {e}"),
                    FontLoadState::Ready(info) => {
                        library.release_job(self.font_job.take().unwrap())?;
                        self.config.font = info.alias;
                        self.font_handle = Some(info.handle);
                        self.font_ready = true;
                    }
                }
            } else {
                if self.fonts.is_none() {
                    self.fonts = Some(h.font_library(self.client)?);
                }
                self.font_ready = true;
            }
        }
        if self.machine.is_some() {
            return Ok(self.font_ready);
        }
        if self.boot.is_none() {
            let storage = h.storage(self.client)?;
            let images = h.image_metadata(self.client)?;
            let pool = TaskPool::new("malie-startup", 1, 2)?;
            self.boot = Some(pool.spawn(move |cancel| {
                cancel.check()?;
                let program = Arc::new(Program::parse(&read(
                    storage.as_ref(),
                    "data/system/exec.dat",
                    64 * 1024 * 1024,
                )?)?);
                let text = String::from_utf8(read(
                    storage.as_ref(),
                    "data/screen/title/title.svg",
                    1024 * 1024,
                )?)?;
                let document = Document::parse(&text, "data/screen/title")?;
                let svg = text;
                let script = VfsPath::new(
                    Mount::Asset,
                    &format!("data/screen/title/{}", document.script),
                )?;
                let source =
                    String::from_utf8(read(storage.as_ref(), script.relative(), 256 * 1024)?)?;
                let mut pixels = 0u64;
                for image in document.images()? {
                    cancel.check()?;
                    let size = images
                        .size(&image)
                        .with_context(|| format!("Malie title image {image:?}"))?;
                    pixels = pixels
                        .checked_add(u64::from(size[0]) * u64::from(size[1]))
                        .context("Malie title image budget")?;
                    ensure!(pixels <= 64 * 1024 * 1024, "Malie title image byte budget");
                }
                let bytes = read(storage.as_ref(), "malie.ini", 1024 * 1024)?;
                let text = encoding_rs::SHIFT_JIS.decode(&bytes).0.into_owned();
                let mut section = String::new();
                let mut ini: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
                for line in text.lines() {
                    let line = line.trim();
                    if line.starts_with(';') {
                        continue;
                    }
                    if line.starts_with('[') && line.ends_with(']') {
                        section = line[1..line.len() - 1].to_ascii_lowercase();
                    } else if let Some((k, v)) = line.split_once('=') {
                        ini.entry(section.clone())
                            .or_default()
                            .insert(k.trim().to_ascii_lowercase(), v.trim().into());
                    }
                }
                let system_voice_csv = storage
                    .metadata(Mount::Asset, "data/system/systemvoice.csv")?
                    .is_some();
                let text_of = |p: &str| -> Result<String> { Ok(read_optional(storage.as_ref(), p, 4 * 1024 * 1024)?.map(|b| decode_text(&b)).unwrap_or_default()) };
                let extra = ExtraData::parse(&text_of("data/extra/cg.csv")?, &text_of("data/extra/scene.csv")?, &text_of("data/extra/music.csv")?, &text_of("data/extra/movie.csv")?);
                // 627780: name,voice rows; absent file means no mapping.
                let mut system_voices = BTreeMap::new();
                for line in text_of("data/system/systemvoice.csv")?.lines() {
                    if let Some((k, v)) = line.trim().split_once(',') {
                        system_voices.insert(k.trim().to_ascii_lowercase(), v.trim().to_owned());
                    }
                }
                let mut nameconv = BTreeMap::new();
                if let Some(bytes) = read_optional(storage.as_ref(), "data/system/nameconv.csv", 1024 * 1024)? {
                    for line in decode_text(&bytes).lines() {
                        if let Some((from, to)) = line.trim().split_once(',') {
                            nameconv.insert(from.trim().to_owned(), to.trim().to_owned());
                        }
                    }
                }
                Ok(Boot {
                    program,
                    svg,
                    source,
                    ini,
                    system_voice_csv,
                    nameconv,
                    storage,
                    extra,
                    system_voices,
                })
            })?);
            self.pool = Some(pool);
        }
        if let Some(boot) = self.boot.as_mut().unwrap().try_take()? {
            if let Some(title) = &self.config.title {
                h.submit_engine(
                    self.client,
                    1,
                    &serde_json::to_vec(&json!({"ui":[{"op":"set_title","text":title}]}))?,
                )?;
            }
            // Native presentation clears to black every frame; a retained
            // canvas-sized base layer reproduces that under every root.
            let [w, h_] = self.config.canvas;
            h.submit_engine(
                self.client,
                1,
                &serde_json::to_vec(&json!({"layers":[{"op":"put","layer":{"id":99_999,"rect":[0,0,w,h_],"paint":{"kind":"solid","color":[0,0,0,255]},"order":[0,0]}}]}))?,
            )?;
            self.exec_program = Some(boot.program.clone());
            let mut machine = Machine::new(boot.program);
            machine.function("system_onInit")?;
            self.machine = Some(machine);
            self.apply_system_globals();
            self.title_data = Some((boot.svg, boot.source));
            self.ini = boot.ini;
            self.system_voice_csv = boot.system_voice_csv;
            self.nameconv = boot.nameconv;
            self.storage = Some(boot.storage);
            self.extra = boot.extra;
            self.system_voices = boot.system_voices;
            self.settings = Settings::defaults(&self.ini);
            self.load_system()?;
            self.apply_system_globals();
            self.civil_job = h.begin_civil_time(self.client).ok();
            let size = self.ini("system", "fontsize").and_then(|v| v.parse::<f32>().ok()).unwrap_or(20.);
            self.adv = Some(Adv::new(
                [self.config.canvas[0] as f32, self.config.canvas[1] as f32],
                self.config.font.clone(),
                size.clamp(8., 64.),
            ));
            self.boot = None;
        }
        Ok(self.machine.is_some() && self.font_ready)
    }
    fn submit(&mut self, h: &mut dyn HostServices, layers: Vec<Value>, ui: Vec<Value>) -> Result<()> {
        if layers.is_empty() && ui.is_empty() {
            return Ok(());
        }
        h.submit_engine(self.client, 1, &serde_json::to_vec(&json!({"layers":layers,"ui":ui}))?)?;
        Ok(())
    }
    fn publish_screen(&mut self, h: &mut dyn HostServices) -> Result<()> {
        if let Some(screen) = &mut self.screen {
            let mut layers = screen.document.borrow_mut().layers()?;
            let mut parents = BTreeMap::new();
            for op in &layers {
                let l = &op["layer"];
                parents.insert(l["id"].as_u64().unwrap_or(0) as u32, l["parent"].as_u64().map(|v| v as u32));
            }
            let gone: std::collections::BTreeSet<u32> = screen.parents.keys().filter(|id| !parents.contains_key(id)).copied().collect();
            layers.extend(crate::screen::removals(&gone, &screen.parents));
            screen.live_count = parents.len();
            screen.parents = parents;
            let mut ui = screen.document.borrow_mut().widgets()?;
            let controls = screen.document.borrow().controls()?;
            let ids = controls.iter().map(|(_, id)| *id).collect::<std::collections::BTreeSet<_>>();
            for id in screen.widgets.difference(&ids) {
                ui.push(json!({"op":"remove","id":id}));
            }
            screen.widgets = ids;
            self.submit(h, layers, ui)?;
        }
        Ok(())
    }
    /// The program for a module name (`None` = exec.dat); `Ok(None)` when
    /// the source file is absent.
    fn module_program(&self, module: Option<&str>) -> Result<Option<Arc<Program>>> {
        match module {
            None => Ok(self.exec_program.clone()),
            Some(name) => {
                let storage = self.storage.as_ref().context("Malie storage")?;
                Ok(crate::compiler::compile(&mut ScriptFiles(storage.as_ref()), name)?.map(Arc::new))
            }
        }
    }
    /// 613800: replace the EXEC module and run its `main`.
    pub(super) fn switch_module(&mut self, name: &str) -> Result<bool> {
        let Some(program) = self.module_program(Some(name)).with_context(|| format!("Malie script module {name:?}"))? else {
            return Ok(false);
        };
        let mut machine = Machine::new(program);
        machine.function("main")?;
        self.machine = Some(machine);
        self.module = Some(name.to_owned());
        self.apply_system_globals();
        self.wait = Wait::None;
        Ok(true)
    }
    fn hide_screen(&mut self, h: &mut dyn HostServices) -> Result<()> {
        if let Some(screen) = self.screen.take() {
            let gone = screen.parents.keys().copied().collect();
            let layers = crate::screen::removals(&gone, &screen.parents);
            let ui = screen.widgets.into_iter().map(|id| json!({"op":"remove","id":id})).collect();
            self.submit(h, layers, ui)?;
        }
        Ok(())
    }
    fn open_title(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let (svg, source) = self.title_data.clone().context("Malie title resources")?;
        let mut document = Document::parse(&svg, "data/screen/title")?;
        document.config.insert("continue".into(), self.system.last_slot.is_some());
        document.flags = self.global_flags();
        // The title is drawn without the ADV scene beneath it.
        if let Some(adv) = &mut self.adv {
            let ops = adv.forget();
            self.submit(h, ops, Vec::new())?;
        }
        self.screen = Some(Screen::new(
            document,
            &source,
            h.now_ms(),
            // 563FD0: logo on the first visit, `restoreScene` after a scene
            // replay, otherwise the title animation.
            if self.title_visits == 0 { "showLogo" } else if self.replay.take().is_some() { "restoreScene" } else { "showTitle" },
        )?);
        self.title_visits += 1;
        self.wait = Wait::Title;
        self.publish_screen(h)
    }
    fn audio_ticket(&mut self, h: &mut dyn HostServices, channel: u32, path: &str, parameters: AudioParameters) -> Result<()> {
        let storage = self.storage.as_ref().context("Malie storage")?;
        let p = VfsPath::new(Mount::Asset, path)?;
        if storage.metadata(Mount::Asset, p.relative())?.is_none() {
            bail!("Malie audio asset {path:?} is absent");
        }
        let ticket = h.begin_audio(
            self.client,
            AudioRequest::PlayAsset {
                channel,
                source: AudioAsset { path: p, frame_limit: None },
                intro: None,
                parameters,
            },
        )?;
        self.audio.push(ticket);
        Ok(())
    }
    fn voice_path(name: &str) -> Result<String> {
        let lower = name.to_ascii_lowercase();
        let stem = lower.strip_prefix("v_").unwrap_or(&lower);
        let dir: String = stem.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
        ensure!(!dir.is_empty() && lower.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'), "Malie voice name {name:?}");
        Ok(format!("data/voice/{dir}/{lower}.ogg"))
    }
    fn read_text(&self, path: &str) -> Result<Option<String>> {
        let storage = self.storage.as_ref().context("Malie storage")?;
        Ok(read_optional(storage.as_ref(), path, 1024 * 1024)?.map(|b| decode_text(&b)))
    }
    fn load_frame(&mut self, src: &str) -> Result<()> {
        if self.frame_doc.as_ref().is_some_and(|(s, _)| s == src) {
            return Ok(());
        }
        let path = format!("data/screen/messageframe/{src}.svg");
        let text = self.read_text(&path)?.with_context(|| format!("Malie message frame {src:?} is absent"))?;
        let mut doc = Document::parse(&text, "data/screen/messageframe")?;
        doc.layer_base = 600_000;
        doc.widget_base = 650_000;
        doc.order_major = 40;
        let adv = self.adv.as_mut().unwrap();
        if let Some(r) = doc.rect_of("message")? {
            adv.text_rect = r;
        }
        if let Some(r) = doc.rect_of("name")? {
            adv.name_rect = r;
        }
        if let Some(r) = doc.rect_of("face")? {
            adv.face_rect = r;
        }
        self.frame_doc = Some((src.to_owned(), doc));
        Ok(())
    }
    fn frame_layers(&mut self) -> Result<()> {
        let opacity = self.adv.as_ref().unwrap().state.frame.opacity;
        let src = self.adv.as_ref().unwrap().state.frame.src.clone();
        self.load_frame(&src)?;
        let (_, doc) = self.frame_doc.as_mut().unwrap();
        // No speaker: the name plate is hidden with the name.
        let unnamed = self.adv.as_ref().unwrap().state.name.is_empty();
        for id in ["name_base", "name"] {
            if unnamed {
                doc.hidden.insert(id.into());
            } else {
                doc.hidden.remove(id);
            }
        }
        if let Some(&base) = doc.ids.get("base") {
            doc.nodes[base].attrs.insert("opacity".into(), opacity.to_string());
        }
        let layers: Vec<Value> = doc
            .layers()?
            .into_iter()
            .map(|op| {
                let mut l = op["layer"].clone();
                if l["parent"].is_null() {
                    l["parent"] = json!(adv::ROOT);
                }
                l
            })
            .collect();
        self.adv.as_mut().unwrap().frame_layers = layers;
        Ok(())
    }
    // ---- tags -----------------------------------------------------------
    fn format(&self, call: &Call) -> Result<String> {
        let machine = self.machine.as_ref().unwrap();
        let format = machine.string(call.args[0])?;
        let mut out = String::new();
        let mut args = call.args[1..].iter();
        let mut chars = format.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '%' {
                out.push(c);
                continue;
            }
            let mut spec = String::new();
            while let Some(&n) = chars.peek() {
                spec.push(n);
                chars.next();
                if n.is_ascii_alphabetic() || n == '%' {
                    break;
                }
            }
            match spec.chars().last() {
                Some('%') => out.push('%'),
                Some('d') | Some('i') => out.push_str(&(*args.next().context("Malie tag format arity")? as i32).to_string()),
                Some('s') => out.push_str(&machine.string(*args.next().context("Malie tag format arity")?)?),
                _ => bail!("Malie tag format %{spec}"),
            }
        }
        ensure!(out.len() <= 64 * 1024, "Malie formatted tag budget");
        Ok(out)
    }
    fn tag(&mut self, h: &mut dyn HostServices, text: &str) -> Result<()> {
        if self.trace {
            eprintln!("MALIE_TRACE {} tag {text}", h.now_ms());
        }
        for (name, a) in adv::parse_tags(text)? {
            self.tag_one(h, &name, &a).with_context(|| format!("Malie tag <{name}>"))?;
        }
        Ok(())
    }
    fn advance_input(&mut self, input: &InputFrame) -> bool {
        let pressed = input.buttons_pressed | (input.buttons_down & !self.previous_down);
        self.previous_down = input.buttons_down;
        pressed & (buttons::MOUSE_LEFT | buttons::ENTER | buttons::SPACE) != 0
            || input.buttons_down & buttons::CTRL != 0
    }
    fn render_adv(&mut self, h: &mut dyn HostServices) -> Result<()> {
        if self.screen.is_some() {
            return Ok(());
        }
        let now = h.now_ms();
        if self.adv.as_ref().unwrap().state.frame.visible {
            self.frame_layers()?;
        }
        {
            let interval = self.tags.text_speed.unwrap_or_else(|| self.settings.text_interval_ms());
            let color = if self.tags.user_color { None } else { self.tags.text_color };
            let adv = self.adv.as_mut().unwrap();
            adv.text_interval_ms = if self.skip_active { 0 } else { interval };
            adv.text_color = color.map_or([255, 255, 255, 255], |c| [c[0], c[1], c[2], 255]);
            if let Some(size) = self.tags.font_size {
                adv.font_size = (size as f32).clamp(8., 64.);
            }
        }
        let fonts = self.fonts.clone().context("Malie font library")?;
        let font = self.config.font.clone();
        let advance = move |c: char, size: f32| -> f32 {
            fonts.metrics(&font, size, c).map(|m| m.advance).unwrap_or(size)
        };
        let mut ops = self.adv.as_mut().unwrap().render(now, &advance)?;
        self.sync_movies(h)?;
        if let Some((t, _)) = self.transition.take() {
            ops.insert(0, t);
        }
        self.submit(h, ops, Vec::new())
    }
    /// Start shared-media players for newly drawn movie elements and release
    /// those no longer drawn. Pictures are decoded on media workers.
    fn sync_movies(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let adv = self.adv.as_mut().unwrap();
        let wanted = adv.movies_wanted.clone();
        let stale: Vec<String> = adv
            .movie_surfaces
            .iter()
            .filter(|(k, (asset, _))| wanted.get(*k).is_none_or(|(a, _)| a != asset))
            .map(|(k, _)| k.clone())
            .collect();
        for key in stale {
            adv.movie_surfaces.remove(&key);
            if let Some(id) = self.movie_ids.remove(&key) {
                h.media_request(self.client, MediaRequest::Stop { id })?;
                h.media_request(self.client, MediaRequest::Remove { id })?;
            }
        }
        for (key, (asset, looping)) in wanted {
            if adv.movie_surfaces.contains_key(&key) {
                continue;
            }
            ensure!(self.movie_ids.len() < 16, "Malie concurrent movie budget");
            let id = (1..=64).find(|i| !self.movie_ids.values().any(|v| v == i)).context("Malie movie id")?;
            let surface = h.media_request(self.client, MediaRequest::Play { id, asset: asset.clone(), looping, audio: false, gain: 1. })?;
            self.movie_ids.insert(key.clone(), id);
            adv.movie_surfaces.insert(key, (asset, surface.relative().to_owned()));
        }
        Ok(())
    }
    /// `layerwait type='MOVIE'`: every selected stream has ended.
    fn movies_ended(&self, h: &mut dyn HostServices, a: &Attrs) -> Result<bool> {
        for key in self.adv.as_ref().unwrap().layer_movies(a)? {
            if let Some(id) = self.movie_ids.get(&key) {
                let info = h.media_info(self.client, *id)?;
                if self.trace {
                    eprintln!("MALIE_TRACE movie {key} {:?} pos={} frames={} dur={:?} live={:?}", info.phase, info.position_ms, info.published_frames, info.duration_ms, self.movie_ids);
                }
                // The presentation clock passing the declared duration ends the
                // wait even when decoding lags behind (e.g. accelerated hosts).
                let clock_done = info.duration_ms.is_some_and(|d| info.position_ms >= d);
                if !clock_done && !matches!(info.phase, MediaPhase::Ended | MediaPhase::Stopped | MediaPhase::Failed) {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
    fn step_inner(&mut self, h: &mut dyn HostServices, input: InputFrame, budget: u32) -> Result<VmYield> {
        if self.machine.is_none() || !self.font_ready {
            self.prepare(h)?;
            return Ok(VmYield::BudgetExhausted);
        }
        let now = h.now_ms();
        for ticket in std::mem::take(&mut self.audio) {
            let state = h.audio_preparation(self.client, ticket)?;
            match state.phase {
                AudioPreparePhase::Pending => self.audio.push(ticket),
                AudioPreparePhase::Ready => h.release_audio(self.client, ticket)?,
                AudioPreparePhase::Failed => {
                    h.release_audio(self.client, ticket)?;
                    let e = state.error.unwrap_or_default();
                    self.adv.as_mut().unwrap().gap(format!("audio: {e}"));
                }
            }
        }
        for (at, channel, path, looping, gain) in std::mem::take(&mut self.delayed_se) {
            if now >= at {
                self.audio_ticket(h, channel, &path, AudioParameters { looping, gain, ..Default::default() })?;
            } else {
                self.delayed_se.push((at, channel, path, looping, gain));
            }
        }
        if let Some(job) = self.civil_job.as_mut() {
            match job.poll()? {
                crysirror_core::service::CivilTimeState::Pending => {}
                crysirror_core::service::CivilTimeState::Ready(t) => {
                    self.civil = Some((t.unix_ms, t.utc_offset_seconds, now));
                    self.civil_job = None;
                }
                crysirror_core::service::CivilTimeState::Failed(_) => self.civil_job = None,
            }
        }
        self.poll_thumbnail(h)?;
        self.collect_seen();
        if now >= self.system_saved_at + 2000 {
            self.system_saved_at = now;
            self.save_system()?;
        }
        if self.exit_requested {
            self.save_system()?;
            return Ok(VmYield::Halted);
        }
        let click = self.advance_input(&input) | std::mem::take(&mut self.pending_click);
        if self.cg_viewer.is_some() || self.movie_viewer.is_some() {
            self.viewer(h, &input)?;
            return Ok(VmYield::WaitForInput);
        }
        if self.dialog.is_some() {
            self.dialog_input(h, &input)?;
            if let Some(screen) = &mut self.screen {
                screen.tick(now)?;
                self.publish_screen(h)?;
            } else {
                self.render_adv(h)?;
            }
            self.render_dialog(h)?;
            return Ok(VmYield::WaitForInput);
        }
        if let Some(screen) = &mut self.screen {
            screen.tick(now)?;
            let controls = screen.document.borrow().controls()?;
            let mut states = Vec::new();
            for (name, id) in controls {
                states.push((name, serde_json::from_slice(&h.query_engine(self.client, EngineQuery::UiInteraction(id))?)?));
            }
            screen.input(states)?;
            let actions: Vec<_> = screen.document.borrow_mut().actions.drain(..).collect();
            for action in actions {
                match action {
                    Action::Tag(text) => {
                        // Title-script waits pace the DOM, not the EXEC thread.
                        self.tag(h, &text)?;
                        self.wait = Wait::Title;
                    }
                    Action::Label(name) => {
                        self.hide_screen(h)?;
                        self.wait = Wait::None;
                        if name == "maliescenario" {
                            // main@title: GetResult()==0 proceeds to main@adv.
                            self.result = 0;
                        } else {
                            self.start_replay(h, &name)?;
                        }
                    }
                    Action::Goto(label) => {
                        self.hide_screen(h)?;
                        self.machine.as_mut().unwrap().label(&label)?;
                        self.init = false;
                        self.wait = Wait::None;
                    }
                    Action::EndModal(r) => {
                        self.hide_screen(h)?;
                        self.result = r;
                        self.wait = Wait::None;
                    }
                    Action::EndDialog => {
                        if self.dialog.is_some() {
                            self.close_dialog(h)?;
                        }
                    }
                    Action::SystemSe(name) => self.system_se(h, &name.to_ascii_lowercase())?,
                    Action::SetFlag(name, v) => {
                        let m = self.machine.as_mut().unwrap();
                        if let Some(at) = m.program.symbols.iter().find(|s| s.name == name).map(|s| s.offset) {
                            if let Some(slot) = m.globals.get_mut(at as usize..at as usize + 4) {
                                slot.copy_from_slice(&v.to_le_bytes());
                            }
                        }
                        self.system_dirty = true;
                    }
                    Action::SetConfig(k, v) => {
                        match v.parse::<i32>() {
                            Ok(n) => {
                                self.settings.values.insert(k, n);
                            }
                            Err(_) => {
                                self.settings.options.insert(k, v);
                            }
                        }
                        self.settings.sync_derived();
                    }
                    Action::Dialog(name) => {
                        self.open_dialog(h, &name)?;
                        if self.dialog.is_some() {
                            self.dialog.as_mut().unwrap().from_title = true;
                            self.dialog_return = Wait::Title;
                            self.wait = Wait::Dialog;
                        }
                    }
                }
            }
            if self.screen.is_some() {
                self.publish_screen(h)?;
                return Ok(VmYield::WaitForInput);
            }
        }
        if self.frame_buttons(h, &input)? {
            self.render_adv(h)?;
            if self.dialog.is_some() {
                self.render_dialog(h)?;
            }
            return Ok(VmYield::WaitForInput);
        }
        let ctrl = input.buttons_down & buttons::CTRL != 0;
        let skipping = ctrl || self.skip_allowed() || self.jump.is_some();
        self.skip_active = skipping;
        // Resolve the current wait.
        let resumed = match &self.wait {
            Wait::None => true,
            Wait::Title | Wait::Dialog => false,
            Wait::Until(t) => now >= *t || skipping,
            Wait::Click => {
                let revealing = self.adv.as_ref().unwrap().revealing(now);
                if skipping {
                    self.adv.as_mut().unwrap().complete_reveal();
                    true
                } else if click {
                    if self.auto_active {
                        self.auto_active = false;
                    }
                    if revealing {
                        self.adv.as_mut().unwrap().complete_reveal();
                        false
                    } else {
                        true
                    }
                } else if self.auto_active && !revealing {
                    let voice = h.audio_info(self.client, VOICE)?.is_some_and(|i| i.source_playing);
                    match (voice, self.auto_since) {
                        (true, _) => {
                            self.auto_since = None;
                            false
                        }
                        (false, None) => {
                            self.auto_since = Some(now);
                            false
                        }
                        (false, Some(t)) => now >= t + self.settings.auto_delay_ms(),
                    }
                } else {
                    false
                }
            }
            Wait::Layer(a) => {
                let a = a.clone();
                let movie = a.get("type").is_some_and(|t| t.eq_ignore_ascii_case("movie"));
                if movie {
                    let cancel = a.get("cancel").is_some_and(|c| c.eq_ignore_ascii_case("enable")) || a.get("skip").is_some_and(|c| c.eq_ignore_ascii_case("true"));
                    self.movies_ended(h, &a)? || skipping || (cancel && click)
                } else {
                    skipping || !self.adv.as_ref().unwrap().layer_busy(&a, now)?
                }
            }
            Wait::Suspended => click || skipping,
            Wait::Halted => false,
            Wait::Channel(c) => {
                let c = *c;
                skipping || click || !h.audio_info(self.client, c)?.is_some_and(|i| i.source_playing)
            }
            Wait::Select => {
                if std::mem::take(&mut self.choice_snapshot_due) {
                    self.choice_pending = Some(Box::new(self.record(now)?));
                }
                let adv = self.adv.as_mut().unwrap();
                let rects = adv.select_rects();
                let (x, y) = (input.pointer_x as f32, input.pointer_y as f32);
                let hit = rects.iter().position(|r| x >= r[0] && y >= r[1] && x < r[0] + r[2] && y < r[1] + r[3]);
                let sel = adv.state.select.as_mut().unwrap();
                if hit.is_some() {
                    sel.focus = hit;
                }
                let keys = input.buttons_pressed;
                let n = sel.items.len();
                if keys & buttons::DOWN != 0 {
                    sel.focus = Some(sel.focus.map_or(0, |f| (f + 1) % n));
                }
                if keys & buttons::UP != 0 {
                    sel.focus = Some(sel.focus.map_or(n - 1, |f| (f + n - 1) % n));
                }
                // Skip and auto stop at choices.
                self.auto_active = false;
                self.skip_mode = false;
                let chosen = if keys & buttons::MOUSE_LEFT != 0 { hit } else if keys & (buttons::ENTER | buttons::SPACE) != 0 { sel.focus } else { None };
                match chosen {
                    Some(i) => {
                        self.result = sel.items[i].0;
                        adv.state.select = None;
                        // 631710: keep the snapshot taken at this choice.
                        if let Some(r) = self.choice_pending.take() {
                            if self.choice_history.len() >= 10 {
                                self.choice_history.remove(0);
                            }
                            self.choice_history.push(*r);
                        }
                        true
                    }
                    None => false,
                }
            }
        };
        if resumed {
            self.wait = Wait::None;
            self.auto_since = None;
            if let Some(t) = self.after_wait.take() {
                self.tag(h, &t)?;
            }
            for _ in 0..budget {
                self.executed += 1;
                match self.machine.as_mut().unwrap().step()? {
                    Step::Continue => {}
                    Step::Stop => {
                        if self.init {
                            self.init = false;
                            self.machine.as_mut().unwrap().label("main")?;
                        } else {
                            return Ok(VmYield::Halted);
                        }
                    }
                    Step::Call(call) => {
                        let result = self.import(h, &call).with_context(|| format!("Malie native pc={:#x}", call.pc))?;
                        if self.machine.as_ref().unwrap().pending.is_some() {
                            self.machine.as_mut().unwrap().complete(result)?;
                        }
                        if !matches!(self.wait, Wait::None) || self.screen.is_some() || self.dialog.is_some() {
                            break;
                        }
                    }
                }
            }
            if matches!(self.wait, Wait::Click | Wait::Select) && std::mem::take(&mut self.autosave_requested) {
                // Snapshot first; the autosave is written when it completes.
                self.thumb_wanted = true;
                self.pending_save = Some(saves::PendingSave::Auto);
            }
        }
        self.render_adv(h)?;
        if self.dialog.is_some() {
            self.render_dialog(h)?;
        }
        Ok(match self.wait {
            Wait::None => VmYield::BudgetExhausted,
            _ => VmYield::WaitForInput,
        })
    }
    /// Read-only skip: the current label must have been read before.
    fn skip_allowed(&self) -> bool {
        if !self.skip_mode {
            return false;
        }
        match self.settings.option("skip") {
            "all" => true,
            _ => self.system.read_labels.contains(&self.label),
        }
    }
    fn collect_seen(&mut self) {
        let adv = self.adv.as_mut().unwrap();
        for p in std::mem::take(&mut adv.seen_pictures) {
            if self.seen_pictures.insert(p) {
                self.system_dirty = true;
            }
        }
        for m in std::mem::take(&mut adv.seen_movies) {
            if self.seen_movies.insert(m) {
                self.system_dirty = true;
            }
        }
        if !self.label.is_empty() && self.tags.read_labels.insert(self.label.clone()) {
            self.system_dirty = true;
        }
    }
    /// Message-frame buttons (normal/novel1 SVG): returns true when input
    /// was consumed by the window chrome.
    fn frame_buttons(&mut self, h: &mut dyn HostServices, input: &InputFrame) -> Result<bool> {
        let pressed = input.buttons_pressed;
        // 4FFE50: any click shows a user-hidden window and is consumed.
        if self.adv.as_ref().unwrap().window_hidden && pressed & (buttons::MOUSE_LEFT | buttons::MOUSE_RIGHT | buttons::ENTER | buttons::SPACE | buttons::ESCAPE) != 0 {
            self.adv.as_mut().unwrap().window_hidden = false;
            self.previous_down = input.buttons_down;
            return Ok(true);
        }
        if pressed & (buttons::MOUSE_RIGHT | buttons::ESCAPE) != 0 && matches!(self.wait, Wait::Click | Wait::Select) {
            // [SHORTCUT] MouseRButtonClick=GameConfig.
            let action = self.ini("shortcut", "mousebuttonclick").or(self.ini("shortcut", "mouserbuttonclick")).unwrap_or("GameConfig").to_ascii_lowercase();
            return self.shortcut(h, &action).map(|_| true);
        }
        let visible = self.adv.as_ref().unwrap().state.frame.visible;
        if !visible || !matches!(self.wait, Wait::Click | Wait::Suspended) {
            return Ok(false);
        }
        let Some((_, doc)) = self.frame_doc.as_mut() else { return Ok(false) };
        let events = doc.pointer(input.pointer_x as f32, input.pointer_y as f32, pressed & buttons::MOUSE_LEFT != 0, input.buttons_down & buttons::MOUSE_LEFT != 0);
        for e in events {
            if let crate::screen::UiEvent::Button(b) = e {
                let action = match b.as_str() {
                    "auto_chk" => "auto",
                    "skip_chk" => "skip",
                    "save_btn" => "save",
                    "load_btn" => "load",
                    "qsave_btn" => "qsave",
                    "qload_btn" => "qload",
                    "backlog_btn" => "backlog",
                    "voice_btn" => "voicerepeat",
                    "config_btn" => "gameconfig",
                    "exit_btn" => "title",
                    _ => continue,
                };
                self.system_se(h, "ok")?;
                self.shortcut(h, action)?;
                return Ok(true);
            }
        }
        Ok(false)
    }
    /// Native shortcut action names ([SHORTCUT] list in malie.ini).
    fn shortcut(&mut self, h: &mut dyn HostServices, action: &str) -> Result<()> {
        match action {
            "gameconfig" => self.open_dialog(h, "game"),
            "textconfig" => self.open_dialog(h, "text"),
            "soundconfig" => self.open_dialog(h, "sound"),
            "voiceconfig" => self.open_dialog(h, "voice"),
            "shortcut" => self.open_dialog(h, "shortcut"),
            "keyconfig" => self.open_dialog(h, "key"),
            "backlog" => self.open_dialog(h, "backlog"),
            "save" => self.open_dialog(h, "save"),
            "load" => self.open_dialog(h, "load"),
            "qsave" => self.quick_save(h),
            "qload" => self.open_dialog(h, "qload"),
            "voicerepeat" => match self.last_voice.clone() {
                Some(v) => self.play_voice(h, &v, 1.),
                None => Ok(()),
            },
            "auto" => {
                self.auto_active = !self.auto_active;
                self.skip_mode = false;
                self.sync_frame_checks();
                Ok(())
            }
            "skip" => {
                self.skip_mode = !self.skip_mode;
                self.auto_active = false;
                self.sync_frame_checks();
                Ok(())
            }
            "hide" => {
                let adv = self.adv.as_mut().unwrap();
                adv.window_hidden = !adv.window_hidden;
                Ok(())
            }
            "click" => {
                self.adv.as_mut().unwrap().complete_reveal();
                Ok(())
            }
            "title" => self.open_dialog(h, "title"),
            "end" => self.open_dialog(h, "end"),
            "disable" | "minimize" | "" => Ok(()),
            other => bail!("Malie shortcut {other:?}"),
        }
    }
    fn sync_frame_checks(&mut self) {
        if let Some((_, doc)) = self.frame_doc.as_mut() {
            doc.checked.insert("auto_chk".into(), self.auto_active);
            doc.checked.insert("skip_chk".into(), self.skip_mode);
        }
    }
    /// Extra-mode CG and movie viewers.
    fn viewer(&mut self, h: &mut dyn HostServices, input: &InputFrame) -> Result<()> {
        let pressed = input.buttons_pressed;
        let next = pressed & (buttons::MOUSE_LEFT | buttons::ENTER | buttons::SPACE) != 0;
        let cancel = pressed & (buttons::MOUSE_RIGHT | buttons::ESCAPE) != 0;
        let [w, hh] = self.config.canvas.map(|v| v as f32);
        let mut ops = Vec::new();
        if let Some((list, i)) = self.cg_viewer.clone() {
            let i = if next { i + 1 } else { i };
            if cancel || i >= list.len() {
                self.cg_viewer = None;
                ops.push(json!({"op":"remove","id":VIEWER}));
            } else {
                self.cg_viewer = Some((list.clone(), i));
                let storage = self.storage.clone().context("Malie storage")?;
                let mut read = move |path: &str| -> Result<Option<String>> { Ok(read_optional(storage.as_ref(), path, 1024 * 1024)?.map(|b| decode_text(&b))) };
                let pic = self.adv.as_mut().unwrap().picture(&list[i], &mut read)?;
                let k = (w / pic.size[0]).min(hh / pic.size[1]);
                ops.push(json!({"op":"put","layer":{"id":VIEWER,"rect":[0,0,w,hh],"paint":{"kind":"solid","color":[0,0,0,255]},"order":[80,0]}}));
                for (n, e) in pic.elements.iter().enumerate() {
                    if let crate::picture::Paint::Image(asset) = &e.paint {
                        ops.push(json!({"op":"put","layer":{"id":VIEWER+1+n as u32,"parent":VIEWER,"rect":[e.rect[0]*k+(w-pic.size[0]*k)/2.,e.rect[1]*k+(hh-pic.size[1]*k)/2.,e.rect[2]*k,e.rect[3]*k],"paint":{"kind":"image","asset":asset},"order":[80,1+n as i32]}}));
                    }
                }
            }
        } else if let Some(m) = self.movie_viewer.clone() {
            let id = 63;
            if self.movie_viewer_started.is_none() {
                let path = format!("data/picture/event/{m}.webm");
                let surface = h.media_request(self.client, MediaRequest::Play { id, asset: path, looping: false, audio: false, gain: 1. })?;
                self.movie_viewer_started = Some(surface.relative().to_owned());
            }
            let info = h.media_info(self.client, id)?;
            let ended = matches!(info.phase, MediaPhase::Ended | MediaPhase::Stopped | MediaPhase::Failed) || info.duration_ms.is_some_and(|d| info.position_ms >= d);
            if cancel || next || ended {
                h.media_request(self.client, MediaRequest::Stop { id })?;
                h.media_request(self.client, MediaRequest::Remove { id })?;
                self.movie_viewer = None;
                self.movie_viewer_started = None;
                ops.push(json!({"op":"remove","id":VIEWER}));
            } else {
                let surface = self.movie_viewer_started.clone().unwrap();
                ops.push(json!({"op":"put","layer":{"id":VIEWER,"rect":[0,0,w,hh],"paint":{"kind":"solid","color":[0,0,0,255]},"order":[80,0]}}));
                ops.push(json!({"op":"put","layer":{"id":VIEWER+1,"parent":VIEWER,"rect":[0.,12.,w,hh-24.],"paint":{"kind":"image","asset":surface},"order":[80,1]}}));
            }
        }
        self.submit(h, ops, Vec::new())
    }
    pub(super) fn refresh_dialog(&mut self) -> Result<()> {
        self.sync_page()
    }
}
const VIEWER: u32 = 990_000;
impl Drop for MalieVm {
    fn drop(&mut self) {
        if let Some(library) = &self.fonts {
            if let Some(job) = self.font_job {
                let _ = library.release_job(job);
            }
            if let Some(font) = self.font_handle {
                let _ = library.release_font(font);
            }
        }
    }
}
impl VmAdapter for MalieVm {
    fn engine_id(&self) -> &'static str {
        "malie"
    }
    fn snapshot_schema(&self) -> u32 {
        1
    }
    fn preparation_pending(&self) -> bool {
        self.machine.is_none() || !self.font_ready
    }
    fn instruction_budget(&self) -> u32 {
        16384
    }
    fn step(&mut self, h: &mut dyn HostServices, input: InputFrame, budget: u32) -> crysirror_core::Result<VmYield> {
        self.step_inner(h, input, budget).map_err(|e| {
            let message = format!("{e:#}");
            self.error = Some(message.clone());
            Error::Io(message)
        })
    }
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Err(Error::Unsupported("Malie framework snapshot"))
    }
    fn restore(&mut self, _: &[u8]) -> crysirror_core::Result<()> {
        Err(Error::Unsupported("Malie framework snapshot"))
    }
    fn diagnostics(&self) -> Vec<u8> {
        let adv = self.adv.as_ref();
        serde_json::to_vec(&json!({"adapter":"malie","profile":"ragnarok-1.0","mode":"adopt",
            "pc":self.machine.as_ref().map(|m|format!("{:#x}",m.pc)),"executed":self.executed,
            "title":self.screen.is_some(),"wait":format!("{:?}",self.wait),"label":self.label,
            "layers":adv.map(|a|a.state.layers.iter().map(|((g,i),v)|format!("{g:?}/{i}:{}:{:.2}",v.picture.as_ref().map_or("-",|p|p.name.as_str()),v.opacity.at(0)[0])).collect::<Vec<_>>()),"split":adv.and_then(|a|a.state.split.as_ref().map(|s|(s.visible,s.position,s.angle.at(0)[0]))),"chars":adv.map(|a|a.state.chars.values().filter(|c|c.shown).count()),
            "gaps":adv.map(|a|a.gaps.iter().take(64).cloned().collect::<Vec<_>>()),
            "backlog":self.backlog.len(),"error":self.error})).unwrap_or_default()
    }
}
#[cfg(feature = "runtime")]
pub fn register(registry: &mut crysirror_runtime::AdapterRegistry) -> Result<()> {
    registry.register("malie", Arc::new(Factory))
}
#[cfg(feature = "runtime")]
struct Factory;
#[cfg(feature = "runtime")]
impl crysirror_runtime::runtime::VmFactory for Factory {
    fn create(
        &self,
        client: ClientId,
        spec: &crysirror_runtime::model::ClientSpec,
    ) -> Result<Box<dyn VmAdapter>> {
        ensure!(spec.program.is_empty(), "Malie uses EXEC, not data actions");
        Ok(Box::new(MalieVm::new(
            client,
            serde_json::from_value(spec.config.clone())?,
        )?))
    }
}

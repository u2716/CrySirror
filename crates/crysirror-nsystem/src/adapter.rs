// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! N-System `CDIBSnrScreen` command execution over CrySirror services.
//! Native semantics are documented beside each handler with the HANANYU.EXE
//! address of the original method (names from the binary's own trace calls).
use crate::{
    message::{self, Area, Item, Token},
    opcodes::*,
    script::{Command, Operand, Script, TextPart},
};
use anyhow::{Context, Result, bail, ensure};
use crysirror_core::{
    Error,
    path::{Mount, VfsPath},
    service::{
        AudioAsset, AudioParameters, AudioPreparePhase, AudioRequest, ClientStorage, HostServices,
        InputFrame, MediaPhase, MediaRequest, UNWRAPPED_TEXT_LAYOUT, buttons,
    },
    state::ClientId,
    vm::{VmAdapter, VmYield},
};
use crysirror_tasks::{Task, TaskPool};
mod omake;
mod option;
mod panel;
mod screens;
mod slots;
mod title;
use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// First script (the native New Game target, `start.MSD`).
    pub entry: String,
    /// The build's window title (`lpWindowName`), the MSD key (UTF-8 here).
    pub title: String,
    pub canvas: [u32; 2],
    /// `bank/lower-case member` → mounted asset path.
    pub assets: BTreeMap<String, String>,
    pub font: String,
}

const VARS: usize = 8192;
const STRINGS: usize = 1024;
const LAYERS: usize = 16;
const BGM_CHANNEL: u32 = 1;
const VOICE_CHANNEL: u32 = 2;
const SE_CHANNEL: u32 = 16;
const LAYER_BASE: u32 = 100;
const WINDOW_ID: u32 = 400;
const TEXT_BASE: u32 = 1000;
/// `CDIBMesWnd::Initialize` 0x404CD0 reads the glyph-escape sheet by name.
const FONT_SHEET: &str = "FONT";

#[derive(Clone, Debug, PartialEq)]
struct Layer {
    image: Option<String>,
    pos: [i32; 3],
    blend: bool,
    anti: bool,
    action: i32,
    /// `SetColor` 0x43A460 vertex colour [r,g,b,a]; None is the default
    /// 0xFFFFFFFF plain blit (layer defaults at .data 0x5C88E8).
    color: Option<[u8; 4]>,
}
impl Default for Layer {
    /// `.data` 0x5C88E8: blend (+40) on, anti off, colour plain, no action.
    fn default() -> Self {
        Self { image: None, pos: [0; 3], blend: true, anti: false, action: 0, color: None }
    }
}
/// `CDIBSnrAction` (0x430360..): only the Scroll family (ACTMAKE type 1,
/// internal 2) is implemented; the game uses no other.
#[derive(Clone, Debug)]
struct Action {
    duration: u64,
    looping: bool,
    points: Vec<[i32; 3]>,
    elapsed: u64,
    playing: bool,
}
impl Action {
    /// `DrawScroll` 0x4310A0, linear branch: the point at arc length
    /// t·L along the polyline.
    fn offset(&self) -> [i32; 2] {
        let t = self.elapsed as f64 / self.duration.max(1) as f64;
        let seg = |i: usize| {
            let (a, b) = (self.points[i], self.points[i + 1]);
            ((b[0] - a[0]) as f64).hypot((b[1] - a[1]) as f64)
        };
        let total: f64 = (0..self.points.len().saturating_sub(1)).map(seg).sum();
        let mut want = total * t;
        for i in 0..self.points.len().saturating_sub(1) {
            let l = seg(i);
            if want < l {
                let (a, b) = (self.points[i], self.points[i + 1]);
                let f = want / l;
                return [a[0] + ((b[0] - a[0]) as f64 * f) as i32, a[1] + ((b[1] - a[1]) as f64 * f) as i32];
            }
            want -= l;
        }
        let last = self.points.last().copied().unwrap_or_default();
        [last[0], last[1]]
    }
}
#[derive(Default)]
struct Window {
    open: bool,
    /// MESTXDOWN: indent after an opening quote (saved; layout not mapped).
    down: bool,
    area: Option<Area>,
    origin: [i32; 2],
    /// Parts sheet (setup key 0): region 0 is the frame, other regions
    /// the button cells; name plate sheet (key 2).
    image: Option<String>,
    names: Option<String>,
    /// Button positions (keys 30..41) and normal-state cells (keys 50..61).
    panel: Option<panel::Panel>,
    /// `SetMesSkip` / `SetMesAuto` (window buttons 7 and 8).
    skip_mode: bool,
    auto_mode: bool,
    /// Window hidden by a right click (`CDIBMesWnd::Move`, +269).
    hidden: bool,
    name_pos: [i32; 2],
    /// Click-wait icon: parts cell and position (key 117).
    click_icon: Option<(usize, [i32; 2])>,
    /// `SetOutNameNo` 0x403570: NAME.MGD region, −1 none.
    name: i32,
    colors: [[u8; 4]; 2],
    shadow: bool,
    /// Placed characters of the current page and how many are shown.
    glyphs: Vec<message::Glyph>,
    tokens: Vec<Token>,
    next: usize,
    waiting_click: bool,
    busy: bool,
    /// MESRUN read id and whether it was read before this display.
    msg_id: i32,
    read: bool,
    /// Typewriter progress in characters (MovePutWord 0x405AD0).
    budget: f64,
    paragraph: usize,
    /// Auto-mode wait left at a click (CalcAutoTime 0x404000).
    auto_left: i64,
    /// Finished pages (`PushBLBuffer` 0x402040 / `AddBLBuffer` 0x4021D0):
    /// name plate and glyphs; `bl_view` is the page shown while browsing.
    backlog: std::collections::VecDeque<(i32, Vec<message::Glyph>)>,
    bl_view: Option<usize>,
}
enum Wait {
    None,
    Until(u64),
    Frame,
}
/// `MakeSelData` 0x401BC0: one choice button.
#[derive(Clone, Debug)]
struct Choice {
    enabled: bool,
    pos: [i32; 2],
    cells: [i32; 5],
}
/// `SelectStart` 0x401C40 state.
#[derive(Clone, Debug)]
struct Select {
    var: i32,
    count: usize,
    sheet: String,
    elapsed: u64,
    focus: usize,
}
/// `MoveSelect` 0x4057D0 slide/fade time (`dword_4A35B4`, .data = 500).
const SELECT_IN_MS: u64 = 500;
const CHOICE_ID: u32 = 450;
pub struct NSystemVm {
    client: ClientId,
    config: Config,
    title_cp932: Vec<u8>,
    pool: Option<TaskPool>,
    boot: Option<Task<Boot>>,
    /// MGD sprite region tables of the native window sheets.
    regions: BTreeMap<String, Arc<Vec<[u16; 4]>>>,
    scripts: BTreeMap<String, Arc<Script>>,
    script: Option<Arc<Script>>,
    ip: usize,
    /// Native per-command continuation counter (`this[3]`).
    phase: u32,
    vars: Vec<i32>,
    strings: Vec<Vec<u8>>,
    slots: BTreeMap<i32, String>,
    staged: Vec<Layer>,
    shown: Vec<Layer>,
    /// Scene layer ids currently put on the host (removal is strict).
    live: BTreeSet<u32>,
    se: BTreeMap<i32, String>,
    audio_pending: Vec<u64>,
    window: Window,
    /// ATIMES duration in ms (−1 forever) and elapsed time.
    timer: Option<(i64, u64)>,
    input_flags: [bool; 5],
    input: InputFrame,
    choices: BTreeMap<usize, Choice>,
    /// Scripts entered this session (diagnostics).
    visited: BTreeSet<String>,
    /// Current BGM (name, BGMPLAY flag) and looping SE slots, as slots store them.
    bgm: Option<(String, u8)>,
    se_loops: BTreeSet<i32>,
    /// The last LPT snapshot (what a save restores).
    snapshot_at: Option<crate::save::Screen>,
    extra: slots::NativeExtra,
    save_job: Option<slots::SaveJob>,
    /// Native screens above the title/script (save/load, confirmation).
    screens: Vec<screens::NativeScreen>,
    load_job: Option<Task<slots::SlotFile>>,
    quick_exists: bool,
    next_image: u32,
    timezone_query: Option<Box<dyn crysirror_core::service::CivilTimeZoneQuery>>,
    timezone: Option<crysirror_core::service::CivilTimeZoneProfile>,
    /// Novel.ini ConfigEx and the SETCHARID speaker.
    config_ex: crate::system::ConfigEx,
    current_char: i32,
    /// MESWND key 92: auto ms per character, min, max.
    auto_params: [i32; 3],
    /// Ctrl held this frame (key table 0x4A35EC: forced skip).
    force_skip: bool,
    frame_dt: u64,
    /// Last MESVPLAY voice (window Voice button replays it).
    last_voice: Option<String>,
    /// The native title screen while it owns the frame.
    title: Option<title::Title>,
    /// Action slots by native index (ACTMAKE slot + 1).
    actions: BTreeMap<i32, Action>,
    select: Option<Select>,
    clicked: bool,
    wait: Wait,
    last_ms: Option<u64>,
    /// CONFIG.SAV state (globals live in `vars[1000..2000]`).
    system: crate::system::System,
    lists: crate::system::Lists,
    system_write: Option<Task<()>>,
    ini_write: Option<Task<()>>,
    /// A CG/scene replay started from the omake screen.
    replay: Option<omake::ReplayReturn>,
    subtitle: Vec<u8>,
    executed: u64,
    halted: bool,
    error: Option<String>,
}

pub mod opcodes {
    //! `CDIBSnrScreen` dispatch (0x435B20 + vtable 0x47EA40).
    pub const LPT: u16 = 1;
    pub const LS: u16 = 2;
    pub const JP: u16 = 3;
    pub const ONJP: u16 = 4;
    pub const MOV: u16 = 5;
    pub const IF: u16 = 6;
    pub const ATIMES: u16 = 7;
    pub const AWAIT: u16 = 8;
    pub const RANDOM: u16 = 9;
    pub const CMPSTR: u16 = 10;
    pub const IMREAD: u16 = 100;
    pub const IMFREE: u16 = 101;
    pub const LYIMAGE: u16 = 102;
    pub const LYPOS: u16 = 103;
    pub const LYSIZE: u16 = 104;
    pub const LYCOLOR: u16 = 105;
    pub const LYBLEND: u16 = 106;
    pub const LYANTI: u16 = 107;
    pub const LYACTION: u16 = 108;
    pub const LYCAMSTATE: u16 = 109;
    pub const OVERLAP: u16 = 110;
    pub const CAMMOVE: u16 = 111;
    pub const CURSOR: u16 = 112;
    pub const MOVIE: u16 = 113;
    pub const BGMLIST: u16 = 200;
    pub const BGMPLAY: u16 = 201;
    pub const BGMSTOP: u16 = 202;
    pub const BGMWAIT: u16 = 203;
    pub const VOICELIST: u16 = 204;
    pub const VOICEPLAY: u16 = 205;
    pub const VOICESTOP: u16 = 206;
    pub const VOICEWAIT: u16 = 207;
    pub const SEPLAY: u16 = 208;
    pub const SESTOP: u16 = 209;
    pub const SEWAIT: u16 = 210;
    pub const SEREAD: u16 = 211;
    pub const SEFREE: u16 = 212;
    pub const MSG: u16 = 1000;
    pub const MWO: u16 = 1001;
    pub const MWC: u16 = 1002;
    pub const SELECT: u16 = 1003;
    pub const CHKSCEXEC: u16 = 1004;
    pub const SUBTITLE: u16 = 1005;
    pub const SETCGFLAG: u16 = 1006;
    pub const SETSCENEFLAG: u16 = 1007;
    pub const SETMESSKIP: u16 = 1008;
    pub const SELITEM: u16 = 1009;
    pub const MESFTSIZE: u16 = 2000;
    pub const MESFTCOLOR: u16 = 2001;
    pub const MESFTBLOD: u16 = 2002;
    pub const MESFTSHADOW: u16 = 2003;
    pub const MESTXAREA: u16 = 2004;
    pub const MESTXKINSOKU: u16 = 2005;
    pub const MESTXDOWN: u16 = 2006;
    pub const MESNAME: u16 = 2007;
    pub const MESVPLAY: u16 = 2008;
    pub const MESVSTOP: u16 = 2009;
    pub const MESRUN: u16 = 2010;
    pub const BLOGSAVEBEGIN: u16 = 2011;
    pub const BLOGSAVEEND: u16 = 2012;
    pub const SELMAKE: u16 = 2013;
    pub const SELRUN: u16 = 2014;
    pub const ACTMAKE: u16 = 2015;
    pub const ACTFREE: u16 = 2016;
    pub const ACTPLAY: u16 = 2017;
    pub const ACTSTOP: u16 = 2018;
    pub const SCREENACT: u16 = 2019;
    pub const SAVESAM: u16 = 2020;
    pub const FACE: u16 = 2021;
    pub const MOVIEPAUSE: u16 = 2040;
    pub const MOVIERATE: u16 = 2041;
    pub const MOVIELOOP: u16 = 2042;
    pub const SETCHARID: u16 = 7000;
    pub const END: u16 = 10000;
    pub fn name(id: u16) -> &'static str {
        match id {
            LPT => "LPT", LS => "LS", JP => "JP", ONJP => "ONJP", MOV => "MOV", IF => "IF",
            ATIMES => "ATIMES", AWAIT => "AWAIT", RANDOM => "RANDOM", CMPSTR => "CMPSTR",
            IMREAD => "IMREAD", IMFREE => "IMFREE", LYIMAGE => "LYIMAGE", LYPOS => "LYPOS",
            LYSIZE => "LYSIZE", LYCOLOR => "LYCOLOR", LYBLEND => "LYBLEND", LYANTI => "LYANTI",
            LYACTION => "LYACTION", LYCAMSTATE => "LYCAMSTATE", OVERLAP => "OVERLAP",
            CAMMOVE => "CAMMOVE", CURSOR => "CURSOR", MOVIE => "MOVIE", BGMLIST => "BGMLIST",
            BGMPLAY => "BGMPLAY", BGMSTOP => "BGMSTOP", BGMWAIT => "BGMWAIT", VOICELIST => "VOICELIST",
            VOICEPLAY => "VOICEPLAY", VOICESTOP => "VOICESTOP", VOICEWAIT => "VOICEWAIT",
            SEPLAY => "SEPLAY", SESTOP => "SESTOP", SEWAIT => "SEWAIT", SEREAD => "SEREAD",
            SEFREE => "SEFREE", MSG => "MSG", MWO => "MWO", MWC => "MWC", SELECT => "SELECT",
            CHKSCEXEC => "CHKSCEXEC", SUBTITLE => "SUBTITLE", SETCGFLAG => "SETCGFLAG",
            SETSCENEFLAG => "SETSCENEFLAG", SETMESSKIP => "SETMESSKIP", SELITEM => "SELITEM",
            MESFTSIZE => "MESFTSIZE", MESFTCOLOR => "MESFTCOLOR", MESFTBLOD => "MESFTBLOD",
            MESFTSHADOW => "MESFTSHADOW", MESTXAREA => "MESTXAREA", MESTXKINSOKU => "MESTXKINSOKU",
            MESTXDOWN => "MESTXDOWN", MESNAME => "MESNAME", MESVPLAY => "MESVPLAY",
            MESVSTOP => "MESVSTOP", MESRUN => "MESRUN", BLOGSAVEBEGIN => "BLOGSAVEBEGIN",
            BLOGSAVEEND => "BLOGSAVEEND", SELMAKE => "SELMAKE", SELRUN => "SELRUN",
            ACTMAKE => "ACTMAKE", ACTFREE => "ACTFREE", ACTPLAY => "ACTPLAY", ACTSTOP => "ACTSTOP",
            SCREENACT => "SCREENACT", SAVESAM => "SAVESAM", FACE => "FACE", MOVIEPAUSE => "MOVIEPAUSE",
            MOVIERATE => "MOVIERATE", MOVIELOOP => "MOVIELOOP", SETCHARID => "SETCHARID", END => "END",
            0 => "SETUP",
            _ => "unknown",
        }
    }
}

/// Outcome of one command: continue in this frame, or yield it.
enum Flow {
    Next,
    Stay,
    Jump(usize),
    Yield,
}

fn cp932(text: &str) -> Vec<u8> {
    encoding_rs::SHIFT_JIS.encode(text).0.into_owned()
}
fn utf8(bytes: &[u8]) -> String {
    encoding_rs::SHIFT_JIS.decode(bytes).0.into_owned()
}

impl NSystemVm {
    pub fn new(client: ClientId, mut config: Config) -> Result<Self> {
        ensure!(config.canvas == [800, 600], "N-System HANANYU profile is 800x600");
        ensure!(!config.title.is_empty() && !config.font.is_empty(), "N-System title and font required");
        ensure!(config.assets.len() <= 200_000, "N-System asset catalog budget");
        config.assets = config.assets.into_iter().map(|(k, v)| (k.to_ascii_lowercase(), v)).collect();
        let title_cp932 = cp932(&config.title);
        Ok(Self {
            client,
            config,
            title_cp932,
            pool: None,
            boot: None,
            regions: BTreeMap::new(),
            scripts: BTreeMap::new(),
            script: None,
            ip: 0,
            phase: 0,
            vars: vec![0; VARS],
            strings: vec![Vec::new(); STRINGS],
            slots: BTreeMap::new(),
            staged: vec![Layer::default(); LAYERS],
            shown: vec![Layer::default(); LAYERS],
            live: BTreeSet::new(),
            se: BTreeMap::new(),
            audio_pending: Vec::new(),
            window: Window { colors: [[255; 4], [0, 0, 0, 255]], name: -1, ..Default::default() },
            timer: None,
            input_flags: [false; 5],
            input: InputFrame::default(),
            choices: BTreeMap::new(),
            actions: BTreeMap::new(),
            title: None,
            bgm: None,
            se_loops: BTreeSet::new(),
            snapshot_at: None,
            extra: slots::NativeExtra::default(),
            save_job: None,
            screens: Vec::new(),
            load_job: None,
            quick_exists: false,
            next_image: 1,
            timezone_query: None,
            timezone: None,
            last_voice: None,
            config_ex: crate::system::ConfigEx::default(),
            current_char: -1,
            auto_params: [100, 2000, 8000],
            force_skip: false,
            frame_dt: 0,
            visited: BTreeSet::new(),
            select: None,
            clicked: false,
            wait: Wait::None,
            last_ms: None,
            system: crate::system::System::new(DEFAULT_SETTINGS),
            lists: crate::system::Lists::default(),
            system_write: None,
            ini_write: None,
            replay: None,
            subtitle: Vec::new(),
            executed: 0,
            halted: false,
            error: None,
        })
    }
    fn asset(&self, bank: &str, name: &str, ext: &str) -> Result<String> {
        let key = format!("{bank}/{}", name.to_ascii_lowercase());
        let with_ext = format!("{key}.{ext}");
        self.config
            .assets
            .get(&key)
            .or_else(|| self.config.assets.get(&with_ext))
            .cloned()
            .with_context(|| format!("N-System resource {bank}/{name} not found"))
    }
    /// Boot: read and decrypt every `msd/` member on a worker.
    fn begin_boot(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let storage = h.storage(self.client)?;
        self.timezone_query = h.begin_civil_timezone(self.client).ok();
        self.quick_exists = storage.metadata(Mount::Save, slots::slot_path(slots::QUICK_SLOT)?.relative()).ok().flatten().is_some();
        if self.pool.is_none() {
            self.pool = Some(TaskPool::new("nsystem-resources", 1, 4)?);
        }
        let entries: Vec<(String, String)> = self
            .config
            .assets
            .iter()
            .filter_map(|(k, v)| k.strip_prefix("msd/").map(|n| (n.trim_end_matches(".msd").to_owned(), v.clone())))
            .collect();
        let title = self.title_cp932.clone();
        let images: BTreeMap<String, String> =
            self.config.assets.iter().filter(|(k, _)| k.starts_with("mgd/")).map(|(k, v)| (k.clone(), v.clone())).collect();
        self.boot = Some(self.pool.as_ref().unwrap().spawn(move |cancel| {
            let scripts = load_scripts(storage.clone(), entries, &title, &cancel)?;
            let regions = load_window_regions(storage.clone(), &scripts, &images, &cancel)?;
            let system = load_system(&storage)?;
            let ini = {
                let path = VfsPath::new(Mount::Save, "Novel.ini")?;
                match storage.metadata(Mount::Save, path.relative())? {
                    Some(m) if !m.is_directory => {
                        let r = storage.open_read(&path)?;
                        Some(encoding_rs::SHIFT_JIS.decode(&crysirror_resource::read_all(r.as_ref(), 1 << 20)?).0.into_owned())
                    }
                    _ => None,
                }
            };
            Ok(Boot { scripts, regions, system, ini })
        })?);
        Ok(())
    }
    fn enter(&mut self, name: &str) -> Result<()> {
        let key = name.trim_end_matches(".MSD").trim_end_matches(".msd").to_ascii_lowercase();
        let script = self.scripts.get(&key).cloned().with_context(|| format!("N-System script {name} not found"))?;
        self.visited.insert(key);
        self.script = Some(script);
        self.ip = 0;
        self.phase = 0;
        Ok(())
    }
    // ---- operands (CSnrBuffer::GetValueParam / GetStringParam)
    fn var(&self, i: u32) -> Result<i32> {
        self.vars.get(i as usize).copied().with_context(|| format!("N-System variable {i} out of range"))
    }
    fn value(&self, o: &Operand) -> Result<i32> {
        Ok(match o {
            Operand::Int(v) => *v,
            Operand::Var(i) => self.var(*i)?,
            Operand::Expr(pairs) => {
                let mut stack: Vec<i32> = Vec::new();
                for (op, value) in pairs {
                    match op {
                        0 => stack.push(*value),
                        1 => stack.push(self.var(*value as u32)?),
                        2..=6 => {
                            let b = stack.pop().context("N-System expression underflow")?;
                            let a = stack.pop().context("N-System expression underflow")?;
                            stack.push(match op {
                                2 => a.wrapping_add(b),
                                3 => a.wrapping_sub(b),
                                4 => a.wrapping_mul(b),
                                5 => a.checked_div(b).context("N-System division by zero")?,
                                _ => a.checked_rem(b).context("N-System remainder by zero")?,
                            });
                        }
                        other => bail!("N-System expression operator {other}"),
                    }
                }
                stack.pop().unwrap_or(0)
            }
            Operand::Text(_) | Operand::StrVar(_) => bail!("N-System numeric operand expected"),
        })
    }
    fn string(&self, o: &Operand) -> Result<Vec<u8>> {
        Ok(match o {
            Operand::Text(parts) => {
                let mut out = Vec::new();
                for p in parts {
                    match p {
                        TextPart::Bytes(b) => out.extend(b),
                        TextPart::StrVar(i) => out.extend(self.strings.get(*i as usize).context("N-System string variable")?),
                    }
                }
                out
            }
            Operand::StrVar(i) => self.strings.get(*i as usize).cloned().context("N-System string variable")?,
            _ => bail!("N-System string operand expected"),
        })
    }
    fn set_var(&mut self, i: i32, v: i32) -> Result<()> {
        let slot = self.vars.get_mut(usize::try_from(i).ok().context("N-System negative variable")?).context("N-System variable range")?;
        *slot = v;
        Ok(())
    }
    fn arity(c: &Command, n: usize) -> Result<()> {
        ensure!(c.operands.len() == n, "N-System {} takes {n} operands, got {} (native ErrorNumParam)", name(c.id), c.operands.len());
        Ok(())
    }
    fn ints(&self, c: &Command, n: usize) -> Result<Vec<i32>> {
        Self::arity(c, n)?;
        c.operands.iter().map(|o| self.value(o)).collect()
    }

    fn submit(&mut self, h: &mut dyn HostServices, mut layers: Vec<Json>, ui: Vec<Json>) -> Result<()> {
        layers.retain(|op| match (op["op"].as_str(), op["id"].as_u64(), op["layer"]["id"].as_u64()) {
            (Some("remove"), Some(id), _) => self.live.remove(&(id as u32)),
            (Some("put"), _, Some(id)) => {
                self.live.insert(id as u32);
                true
            }
            _ => true,
        });
        let bytes = serde_json::to_vec(&json!({"layers":layers,"ui":ui}))?;
        h.submit_engine(self.client, 1, &bytes)?;
        Ok(())
    }
    fn layer_put(&self, h: &mut dyn HostServices, index: usize, layer: &Layer) -> Result<Json> {
        let id = LAYER_BASE + index as u32;
        // An attached action slot that ACTMAKE never filled draws nothing
        // (CDIBSnrAction::Draw 0x4307B0, type 0); so does an empty layer.
        let action = if layer.action == 0 { None } else { self.actions.get(&layer.action) };
        let Some(name) = layer.image.as_ref().filter(|_| layer.action == 0 || action.is_some()) else {
            return Ok(json!({"op":"remove","id":id}));
        };
        let off = action.map_or([0, 0], Action::offset);
        ensure!(layer.pos[2] == 0, "N-System LYPOS camera depth {} needs the camera transform", layer.pos[2]);
        // blend off (+40 = 0) is an opaque copy; the game only clears it on
        // full-screen opaque backgrounds, where alpha drawing is identical.
        let asset = self.asset("mgd", name, "mgd")?;
        let size = h.image_size(self.client, &asset)?;
        let tint = layer.color.unwrap_or([255; 4]);
        Ok(json!({"op":"put","layer":{"id":id,"order":[index as i32 * 10,0],
            "rect":[layer.pos[0] + off[0],layer.pos[1] + off[1],size[0],size[1]],"paint":{"kind":"image","asset":asset,"tint":tint}}}))
    }
    /// The staged page becomes the shown page (CDIBSnrVVram commit).
    fn commit_layers(&mut self, h: &mut dyn HostServices) -> Result<Vec<Json>> {
        let mut out = Vec::new();
        for i in 0..LAYERS {
            if self.staged[i] != self.shown[i] {
                out.push(self.layer_put(h, i, &self.staged[i].clone())?);
            }
        }
        // CDIBSnrVVram::Move 0x43B2A0: after the effect the pages flip and
        // the old front (now back) layers are reset (SetDefaultProp).
        self.shown = std::mem::replace(&mut self.staged, vec![Layer::default(); LAYERS]);
        Ok(out)
    }
    fn play(&mut self, h: &mut dyn HostServices, channel: u32, path: String, looping: bool) -> Result<()> {
        let ticket = h.begin_audio(
            self.client,
            AudioRequest::PlayAsset {
                channel,
                source: AudioAsset { path: VfsPath::new(Mount::Asset, &path)?, frame_limit: None },
                intro: None,
                parameters: AudioParameters {
                    looping,
                    // Option volumes act on these groups (CBGMBuffer/CSEBuffer/CVoiceBuffer).
                    mix_group: Some(match channel {
                        BGM_CHANNEL => "nsystem-bgm".into(),
                        VOICE_CHANNEL => "nsystem-voice".into(),
                        _ => "nsystem-se".into(),
                    }),
                    ..Default::default()
                },
            },
        )?;
        self.audio_pending.push(ticket);
        Ok(())
    }
    fn poll_audio(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let mut keep = Vec::new();
        for ticket in std::mem::take(&mut self.audio_pending) {
            let prepared = h.audio_preparation(self.client, ticket)?;
            match prepared.phase {
                AudioPreparePhase::Pending => keep.push(ticket),
                AudioPreparePhase::Ready => h.release_audio(self.client, ticket)?,
                AudioPreparePhase::Failed => {
                    h.release_audio(self.client, ticket)?;
                    bail!("N-System audio preparation: {}", prepared.error.unwrap_or_default());
                }
            }
        }
        self.audio_pending = keep;
        Ok(())
    }
    // ---- message window (CDIBMesWnd)
    fn window_layout(&mut self) -> Result<()> {
        if self.window.area.is_some() {
            return Ok(());
        }
        // MESWND.MSD screen-3 setup records read by CDIBMesWnd::SetupStatic
        // 0x4025B0: 0 window image, 110 text rect, 111/112 pitches, 113 font
        // size, 114 window position (key meanings inferred from values).
        let setup = self.scripts.get("meswnd").cloned().context("N-System MESWND.MSD")?;
        let get = |key: i32| -> Result<Vec<i32>> {
            setup
                .setup(3, key)
                .with_context(|| format!("MESWND setup {key}"))?
                .iter()
                .map(|o| self.value(o))
                .collect()
        };
        let rect = get(110)?;
        // Key 92 → dword_4A3548..50 (auto per-char ms, min, max).
        let auto = get(92)?;
        ensure!(auto.len() == 3, "MESWND auto timing");
        let auto_params = [auto[0], auto[1], auto[2]];
        let origin = get(114)?;
        ensure!(rect.len() == 4 && origin.len() == 2, "MESWND setup shape");
        let image = setup.setup(3, 0).and_then(|o| o.first()).map(|o| self.string(o)).transpose()?;
        let area = Area {
            left: rect[0],
            top: rect[1],
            right: rect[2],
            bottom: rect[3],
            size: get(113)?[0],
            pitch: get(112)?[0],
            line_pitch: get(111)?[0],
            kinsoku: false,
        };
        let names = setup.setup(3, 2).and_then(|o| o.first()).map(|o| self.string(o)).transpose()?;
        let names = names.map(|b| utf8(&b).trim_end_matches(".MGD").to_owned()).filter(|n| !n.is_empty());
        // CDIBMesWnd::Initialize 0x404CD0: twelve buttons; button i takes
        // its position from key 30+i and its state cells from key 50+i
        // (the first cell is the normal state; 7 and 8 are toggles).
        let sheet = image.as_ref().map(|b| utf8(b).trim_end_matches(".MGD").to_owned()).context("MESWND parts sheet")?;
        let buttons = panel::Panel::from_setup(self, &setup, 3, 12, 30, 50, &sheet, WINDOW_ID + 1, 5003)?;
        let name_pos = get(116)?;
        ensure!(name_pos.len() == 2, "MESWND name position");
        let name_pos = [name_pos[0], name_pos[1]];
        let icon = get(117)?;
        ensure!(icon.len() == 3 && icon[0] >= 0, "MESWND click icon");
        let click_icon = Some((icon[0] as usize, [icon[1], icon[2]]));
        self.window.image = image.map(|b| utf8(&b).trim_end_matches(".MGD").to_owned());
        self.window.names = names;
        self.window.panel = Some(buttons);
        self.window.name_pos = name_pos;
        self.window.click_icon = click_icon;
        self.window.origin = [origin[0], origin[1]];
        self.window.area = Some(area);
        self.auto_params = auto_params;
        Ok(())
    }
    fn window_commands(&self, h: &mut dyn HostServices) -> Result<Vec<Json>> {
        let mut out = vec![json!({"op":"remove","id":WINDOW_ID})];
        for i in 0..512u32 {
            out.push(json!({"op":"remove","id":TEXT_BASE + i}));
        }
        let mut chrome: Vec<Json> = (1..16).map(|i| json!({"op":"remove","id":WINDOW_ID + i})).collect();
        if !self.window.open {
            out.append(&mut chrome);
            return Ok(out);
        }
        let [ox, oy] = self.window.origin;
        if let Some(sheet) = &self.window.image {
            // UpdateWndImage 0x404180 / UpdateWndImageVB 0x404090: region 0
            // of the parts sheet at the window origin.
            out[0] = self.cell(h, WINDOW_ID, 5000, sheet, 0, [ox, oy])?;
            if let Some(panel) = &self.window.panel {
                for (i, put) in panel.layers(self, h)?.into_iter().enumerate() {
                    chrome[i] = put;
                }
            }
            if let (Some((cell, pos)), true) = (self.window.click_icon, self.window.waiting_click) {
                chrome[12] = self.cell(h, WINDOW_ID + 13, 5004, sheet, cell, pos)?;
            }
        }
        // UpdataNameVB 0x402380: the selected NAME.MGD region at key 116.
        // Browsing the backlog shows that page and its plate instead.
        let (name, glyphs) = match self.window.bl_view.and_then(|i| self.window.backlog.get(i)) {
            Some((n, g)) => (*n, g),
            None => (self.window.name, &self.window.glyphs),
        };
        if let (Some(sheet), Ok(n)) = (&self.window.names, usize::try_from(name)) {
            chrome[13] = self.cell(h, WINDOW_ID + 14, 5003, sheet, n, self.window.name_pos)?;
        }
        out.append(&mut chrome);
        let size = self.window.area.map_or(24, |a| a.size);
        // UpdateTextImage 0x4042A0: text with its shadow at (+2,+2); a
        // glyph escape (byte 1, n) is FONT.MGD region n−1 stretched to the
        // font size at (+1,+1), its shadow at (+3,+3), tinted.
        for (i, g) in glyphs.iter().enumerate().take(256) {
            let gaiji = g.text.strip_prefix('\u{1}').map(|n| n.parse::<usize>()).transpose()?;
            let ids = [TEXT_BASE + 2 * i as u32, TEXT_BASE + 2 * i as u32 + 1];
            let layers: [(usize, i32, [u8; 4], u32, i32); 2] =
                [(0, 2, self.window.colors[1], ids[0], 5001), (1, 0, self.window.colors[0], ids[1], 5002)];
            for (slot, d, color, id, order) in layers {
                if slot == 0 && !self.window.shadow {
                    continue;
                }
                out[1 + 2 * i + slot] = match gaiji {
                    Some(n) => {
                        let cell = n.checked_sub(1).context("N-System glyph escape 0")?;
                        let mut put = self.cell(h, id, order, FONT_SHEET, cell, [ox + g.x + 1 + d, oy + g.y + 1 + d])?;
                        put["layer"]["rect"][2] = json!(size);
                        put["layer"]["rect"][3] = json!(size);
                        put["layer"]["paint"]["tint"] = json!(color);
                        put
                    }
                    None => json!({"op":"put","layer":{"id":id,"order":[order,0],
                        "rect":[ox + g.x + d, oy + g.y + d, size * 2, size + 8],
                        "paint":{"kind":"text","text":g.text,"font":self.config.font,"size":size,"color":color,"layout":UNWRAPPED_TEXT_LAYOUT}}}),
                };
            }
        }
        Ok(out)
    }
    /// Advance playing actions (`MoveScroll` 0x431010) and redraw the shown
    /// layers attached to any action.
    fn tick_actions(&mut self, dt: u64) {
        for a in self.actions.values_mut().filter(|a| a.playing) {
            a.elapsed += dt;
            if a.elapsed >= a.duration {
                a.elapsed = if a.looping { a.elapsed % a.duration } else { a.duration };
            }
        }
    }
    fn refresh_actions(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let mut cmds = Vec::new();
        for i in 0..LAYERS {
            if self.shown[i].action != 0 && self.shown[i].image.is_some() {
                cmds.push(self.layer_put(h, i, &self.shown[i].clone())?);
            }
        }
        if !cmds.is_empty() {
            self.submit(h, cmds, vec![])?;
        }
        Ok(())
    }
    /// One frame of the choice buttons; returns (variable, index) once picked.
    fn select_step(&mut self, h: &mut dyn HostServices) -> Result<Option<(i32, usize)>> {
        let sel = self.select.clone().context("select state")?;
        let t = (sel.elapsed.min(SELECT_IN_MS) as f32) / SELECT_IN_MS as f32;
        let input = self.input;
        let mut focus = sel.focus;
        let mut picked = None;
        let origin = self.choices[&0].pos;
        let mut rects = Vec::new();
        for i in 0..sel.count {
            let ch = &self.choices[&i];
            let pos = if i == 0 {
                ch.pos
            } else {
                [origin[0] + ((ch.pos[0] - origin[0]) as f32 * t) as i32, origin[1] + ((ch.pos[1] - origin[1]) as f32 * t) as i32]
            };
            let key = format!("mgd/{}.mgd", sel.sheet.to_ascii_lowercase());
            let r = *self
                .regions
                .get(&key)
                .and_then(|r| r.get(usize::try_from(ch.cells[0]).unwrap_or(usize::MAX)))
                .with_context(|| format!("N-System {} has no region {}", sel.sheet, ch.cells[0]))?;
            rects.push((pos, [i32::from(r[2]), i32::from(r[3])]));
        }
        if t >= 1.0 {
            let [x, y] = [input.pointer_x, input.pointer_y];
            if let Some(i) = rects.iter().position(|(p, s)| x >= p[0] && y >= p[1] && x < p[0] + s[0] && y < p[1] + s[1]) {
                if self.choices[&i].enabled {
                    focus = i;
                    if input.buttons_pressed & buttons::MOUSE_LEFT != 0 {
                        picked = Some(i);
                    }
                }
            }
            let step = |from: usize, down: bool| -> usize {
                let mut i = from;
                for _ in 0..sel.count {
                    i = if down { (i + 1) % sel.count } else { (i + sel.count - 1) % sel.count };
                    if self.choices[&i].enabled {
                        return i;
                    }
                }
                from
            };
            if input.buttons_pressed & buttons::DOWN != 0 {
                focus = step(focus, true);
            }
            if input.buttons_pressed & buttons::UP != 0 {
                focus = step(focus, false);
            }
            if input.buttons_pressed & (buttons::ENTER | buttons::SPACE) != 0 && self.choices[&focus].enabled {
                picked = Some(focus);
            }
        } else if input.buttons_pressed & (buttons::MOUSE_LEFT | buttons::ENTER | buttons::SPACE) != 0 {
            // A click during the slide finishes it (MoveSelect 0x4057D0).
            self.select.as_mut().unwrap().elapsed = SELECT_IN_MS;
        }
        self.select.as_mut().unwrap().focus = focus;
        let mut cmds = Vec::new();
        for i in 0..10usize {
            let id = CHOICE_ID + i as u32;
            match (picked, rects.get(i)) {
                (None, Some((pos, _))) => {
                    let ch = &self.choices[&i];
                    // Cell 0 normal, 1 focused, 3 disabled (inferred from the sheet).
                    let cell = if !ch.enabled { ch.cells[3] } else if i == focus && t >= 1.0 { ch.cells[1] } else { ch.cells[0] };
                    let mut put = self.cell(h, id, 5010, &sel.sheet, usize::try_from(cell)?, *pos)?;
                    if i > 0 {
                        put["layer"]["opacity"] = json!((255.0 * t) as u8);
                    }
                    cmds.push(put);
                }
                _ => cmds.push(json!({"op":"remove","id":id})),
            }
        }
        self.submit(h, cmds, vec![])?;
        Ok(picked.map(|i| (sel.var, i)))
    }
    /// CONFIG.SAV write (skipped while the previous one is in flight; the
    /// next save point writes the newer state).
    fn write_system(&mut self, h: &mut dyn HostServices) -> Result<()> {
        if let Some(task) = &mut self.system_write {
            if task.try_take()?.is_none() {
                return Ok(());
            }
            self.system_write = None;
        }
        self.system.globals.copy_from_slice(&self.vars[1000..2000]);
        let bytes = self.system.encode()?;
        let storage = h.storage(self.client)?;
        let path = VfsPath::new(Mount::Save, "CONFIG.SAV")?;
        let pool = self.pool.as_ref().context("N-System task pool")?;
        self.system_write = Some(pool.spawn(move |cancel| {
            cancel.check()?;
            storage.write_atomic(&path, &bytes)?;
            Ok(())
        })?);
        Ok(())
    }
    fn read_lists(&self) -> Result<crate::system::Lists> {
        let mut lists = crate::system::Lists::default();
        let text = |o: &Operand| -> Result<String> { Ok(utf8(&self.string(o)?)) };
        if let Some(cg) = self.scripts.get("cglist") {
            let mut per_page = [0usize; 20];
            for c in cg.commands.iter().filter(|c| c.id == 0) {
                let o = &c.operands;
                if o.len() < 4 || o[0] != Operand::Int(100) || o[1] != Operand::Int(0) {
                    continue;
                }
                let count = usize::try_from(self.value(&o[2])?).unwrap_or(0).min(20);
                let page = usize::try_from(self.value(&o[3])?).ok().filter(|p| *p < 20).context("CGLIST page")?;
                ensure!(per_page[page] < 100 && o.len() >= 4 + count, "CGLIST page {page} shape");
                let names = o[4..4 + count].iter().map(text).collect::<Result<_>>()?;
                lists.cg.push((page, per_page[page], names));
                per_page[page] += 1;
            }
        }
        if let Some(sc) = self.scripts.get("scenelist") {
            let mut per = [0usize; 20];
            for c in sc.commands.iter().filter(|c| c.id == 0) {
                let o = &c.operands;
                if o.len() != 4 || o[0] != Operand::Int(101) || o[1] != Operand::Int(0) {
                    continue;
                }
                let list = usize::try_from(self.value(&o[2])?).ok().filter(|l| *l < 20).context("SCENELIST list")?;
                ensure!(per[list] < 20, "SCENELIST list {list} size");
                lists.scenes.push((list, per[list], text(&o[3])?));
                per[list] += 1;
            }
        }
        Ok(lists)
    }
    /// `CDIBMesWnd::Move` 0x4060F0: window buttons while it is shown; a
    /// right click hides/shows the window. True when the input was used.
    fn window_input(&mut self, h: &mut dyn HostServices, input: InputFrame) -> Result<bool> {
        if !self.window.open || self.script.is_none() {
            return Ok(false);
        }
        if input.buttons_pressed & buttons::MOUSE_RIGHT != 0 {
            self.window.hidden = !self.window.hidden;
            let cmds = self.window_commands(h)?;
            self.submit(h, cmds, vec![])?;
            return Ok(true);
        }
        if self.window.hidden {
            // Any click shows it again.
            if input.buttons_pressed & (buttons::MOUSE_LEFT | buttons::ENTER | buttons::SPACE) != 0 {
                self.window.hidden = false;
                let cmds = self.window_commands(h)?;
                self.submit(h, cmds, vec![])?;
                return Ok(true);
            }
            return Ok(false);
        }
        let Some(mut panel) = self.window.panel.take() else { return Ok(false) };
        // UpdateEnableBLUpDownBtn 0x402290: prev needs an older page, next a newer one.
        panel.controls[1].on = !self.window.backlog.is_empty() && self.window.bl_view != Some(0);
        panel.controls[2].on = self.window.bl_view.is_some();
        panel.controls[4].on = self.quick_exists;
        let before = panel.focus;
        // Only pointer clicks press window buttons; Enter advances text.
        let mouse = InputFrame { buttons_pressed: input.buttons_pressed & buttons::MOUSE_LEFT, ..input };
        let over = panel.focus;
        let picked = panel.step(self, mouse);
        let hovered = panel.focus != over || picked.is_some();
        let _ = hovered;
        let changed = panel.focus != before;
        self.window.panel = Some(panel);
        if let Some(i) = picked {
            self.window_button(h, i)?;
        } else if self.window.bl_view.is_some() && input.buttons_pressed & (buttons::MOUSE_LEFT | buttons::ENTER | buttons::SPACE) != 0 {
            // Move 0x4060F0: a click while browsing returns to the current page.
            self.window.bl_view = None;
            let cmds = self.window_commands(h)?;
            self.submit(h, cmds, vec![])?;
            return Ok(true);
        }
        if changed || picked.is_some() {
            let cmds = self.window_commands(h)?;
            self.submit(h, cmds, vec![])?;
        }
        Ok(picked.is_some())
    }
    fn window_button(&mut self, h: &mut dyn HostServices, i: usize) -> Result<()> {
        match i {
            // OnClickVOICEBTN 0x4036C0: replay the current voice.
            0 => {
                if let Some(name) = self.last_voice.clone() {
                    let path = self.asset("voice", &name, "ogg")?;
                    self.play(h, VOICE_CHANNEL, path, false)?;
                }
            }
            // OnClickBLPREVBTN 0x405350 / OnClickBLNEXTBTN 0x405490.
            1 => {
                let n = self.window.backlog.len();
                self.window.bl_view = match self.window.bl_view {
                    _ if n == 0 => None,
                    None => Some(n - 1),
                    Some(i) => Some(i.saturating_sub(1)),
                };
            }
            2 => {
                self.window.bl_view = self.window.bl_view.and_then(|i| (i + 1 < self.window.backlog.len()).then_some(i + 1));
            }
            // OnClickQSAVEBTN 0x405540: slot 999 without a dialog.
            3 => {
                self.begin_save(h, slots::QUICK_SLOT)?;
                self.quick_exists = true;
            }
            4 => self.begin_load(h, slots::QUICK_SLOT)?,
            5 => self.open_save_load(true)?,
            6 => self.open_save_load(false)?,
            7 => {
                self.window.skip_mode = !self.window.skip_mode;
                self.window.auto_mode = false;
            }
            8 => {
                self.window.auto_mode = !self.window.auto_mode;
                self.window.skip_mode = false;
            }
            // OnClickSYSTEMBTN 0x403B20.
            9 => self.open_option()?,
            10 => self.open_conf(1, screens::ConfFor::ToTitle)?,
            11 => self.open_conf(0, screens::ConfFor::Exit)?,
            _ => {}
        }
        if let Some(p) = &mut self.window.panel {
            p.controls[7].toggled = self.window.skip_mode;
            p.controls[8].toggled = self.window.auto_mode;
        }
        Ok(())
    }
    /// One sprite-sheet region as a UV-cropped image layer.
    fn cell(&self, h: &mut dyn HostServices, id: u32, order: i32, sheet: &str, cell: usize, at: [i32; 2]) -> Result<Json> {
        let key = format!("mgd/{}.mgd", sheet.to_ascii_lowercase());
        let r = *self
            .regions
            .get(&key)
            .and_then(|r| r.get(cell))
            .with_context(|| format!("N-System {sheet}.MGD has no region {cell}"))?;
        let asset = self.asset("mgd", sheet, "mgd")?;
        let size = h.image_size(self.client, &asset)?;
        ensure!(
            u32::from(r[0]) + u32::from(r[2]) <= size[0] && u32::from(r[1]) + u32::from(r[3]) <= size[1],
            "N-System {sheet}.MGD region {cell} is outside the image"
        );
        let (w, hh) = (size[0] as f32, size[1] as f32);
        let uv = [r[0] as f32 / w, r[1] as f32 / hh, (r[0] + r[2]) as f32 / w, (r[1] + r[3]) as f32 / hh];
        Ok(json!({"op":"put","layer":{"id":id,"order":[order,0],"rect":[at[0],at[1],r[2],r[3]],"uv":uv,
            "paint":{"kind":"image","asset":asset}}}))
    }
    /// Advance the page: show glyphs up to the next click/page token.
    /// Reveal up to `limit` glyphs (None = to the next stop); stops at a
    /// click or page token, or the end of the message. True if anything changed.
    fn reveal(&mut self, limit: Option<usize>) -> bool {
        let mut shown = 0;
        let mut changed = false;
        while let Some(t) = self.window.tokens.get(self.window.next).cloned() {
            match t {
                Token::Glyph(g) => {
                    if limit.is_some_and(|l| shown >= l) {
                        return changed;
                    }
                    self.window.glyphs.push(g);
                    self.window.paragraph += 1;
                    shown += 1;
                }
                Token::Clear => {
                    if !self.window.glyphs.is_empty() {
                        // Retained page count is not recovered; 256 kept here.
                        if self.window.backlog.len() >= 256 {
                            self.window.backlog.pop_front();
                        }
                        let page = std::mem::take(&mut self.window.glyphs);
                        self.window.backlog.push_back((self.window.name, page));
                    }
                }
                Token::Click => {
                    self.window.next += 1;
                    self.window.waiting_click = true;
                    self.start_auto();
                    return true;
                }
                Token::Page => {
                    // A full page waits for a click, then clears.
                    self.window.tokens[self.window.next] = Token::Clear;
                    self.window.waiting_click = true;
                    self.start_auto();
                    return true;
                }
                Token::Flag(_) => {}
            }
            self.window.next += 1;
            changed = true;
        }
        self.window.busy = false;
        true
    }
    /// CalcAutoTime 0x404000: characters of the paragraph just shown times
    /// MESWND key 92 [per char ms] × (255 − auto speed) / 255, clamped to
    /// [key 92 min, key 92 max] (.data defaults 100 / 2000 / 8000).
    fn start_auto(&mut self) {
        let k = self.auto_params;
        let chars = self.window.paragraph as i64;
        let speed = i64::from(u32::from_le_bytes(self.system.settings[12..16].try_into().unwrap()).min(255));
        let t = chars * i64::from(k[0]) * (255 - speed) / 255;
        self.window.auto_left = t.clamp(i64::from(k[1]), i64::from(k[2]).max(i64::from(k[1])));
        self.window.paragraph = 0;
    }

    fn execute(&mut self, h: &mut dyn HostServices, c: &Command, now: u64) -> Result<Flow> {
        let script = self.script.clone().unwrap();
        Ok(match c.id {
            // 0x411480: save point marker (native also refreshes the quick-save
            // availability); execution continues.
            LPT => {
                let v = self.ints(c, 1)?;
                self.snapshot_at = Some(self.snapshot(v[0], c.pc)?);
                // MoveCmdLPT 0x411480 → SaveConfig 0x4196B0.
                self.write_system(h)?;
                Flow::Next
            }
            // 0x4362F0: replace the running script and start at its first command.
            LS => {
                Self::arity(c, 1)?;
                let name = utf8(&self.string(&c.operands[0])?);
                self.enter(&name)?;
                Flow::Stay
            }
            // 0x4334D0
            JP => {
                let v = self.ints(c, 1)?;
                Flow::Jump(script.label(v[0])?)
            }
            // 0x433570: jump to the n-th label operand, else fall through.
            ONJP => {
                ensure!(c.operands.len() >= 2, "ONJP operands");
                let n = self.value(&c.operands[0])?;
                let targets: Vec<i32> = c.operands[1..].iter().map(|o| self.value(o)).collect::<Result<_>>()?;
                match usize::try_from(n).ok().and_then(|n| targets.get(n)) {
                    Some(t) => Flow::Jump(script.label(*t)?),
                    None => Flow::Next,
                }
            }
            // 0x436430: variable index, value.
            MOV => {
                let v = self.ints(c, 2)?;
                self.set_var(v[0], v[1])?;
                Flow::Next
            }
            // 0x4336A0: (a, op, b) groups, all true → jump to the final label.
            IF => {
                let n = c.operands.len();
                let groups = if n == 4 { 1 } else if n >= 4 && (n + 1) % 4 == 0 { (n + 1) / 4 } else { bail!("IF operand count {n}") };
                let mut pass = 0;
                for g in 0..groups {
                    let a = self.value(&c.operands[3 * g])?;
                    let op = match &c.operands[3 * g + 1] {
                        o if o.is_string() => {
                            let s = self.string(o)?;
                            match utf8(&s).as_str() {
                                "＝" | "=" | "==" => 0,
                                "≠" | "!=" | "<>" => 1,
                                "＜" | "<" => 2,
                                "＞" | ">" => 3,
                                "≦" | "<=" => 4,
                                "≧" | ">=" => 5,
                                other => bail!("IF operator {other}"),
                            }
                        }
                        o => self.value(o)?,
                    };
                    let b = self.value(&c.operands[3 * g + 2])?;
                    let ok = match op {
                        0 => a == b,
                        1 => a != b,
                        2 => a < b,
                        3 => a > b,
                        4 => a <= b,
                        5 => a >= b,
                        _ => false,
                    };
                    if !ok {
                        break;
                    }
                    pass += 1;
                }
                if pass == groups {
                    let label = self.value(c.operands.get(3 * groups).context("IF label")?)?;
                    Flow::Jump(script.label(label)?)
                } else {
                    Flow::Next
                }
            }
            // 0x433BB0: arm a timer (ms; negative = forever).
            ATIMES => {
                let v = self.ints(c, 1)?;
                self.timer = Some((if v[0] < 0 { -1 } else { i64::from(v[0]) }, 0));
                self.input_flags = [false; 5];
                Flow::Next
            }
            // 0x436550: (skippable, five flag variables): wait for the timer
            // (or any input when skippable), then store the input flags.
            AWAIT => {
                Self::arity(c, 6)?;
                let skippable = self.value(&c.operands[0])? != 0;
                let (total, elapsed) = self.timer.unwrap_or((0, 0));
                let done = (skippable && self.input_flags.iter().any(|f| *f)) || (total >= 0 && elapsed as i64 >= total);
                if !done {
                    return Ok(Flow::Yield);
                }
                for (k, o) in c.operands[1..].iter().enumerate() {
                    let index = self.value(o)?;
                    self.set_var(index, i32::from(self.input_flags[k]))?;
                }
                self.timer = None;
                Flow::Next
            }
            // 0x436880: image cache slot ← sprite name.
            IMREAD => {
                Self::arity(c, 2)?;
                let slot = self.value(&c.operands[0])?;
                let name = utf8(&self.string(&c.operands[1])?);
                self.asset("mgd", &name, "mgd")?;
                self.slots.insert(slot, name);
                Flow::Next
            }
            IMFREE => {
                let v = self.ints(c, 1)?;
                self.slots.remove(&v[0]);
                Flow::Next
            }
            // 0x436AB0: layer image from a name or an IMREAD slot (back page).
            LYIMAGE => {
                Self::arity(c, 2)?;
                let l = self.layer_index(self.value(&c.operands[0])?)?;
                let name = if c.operands[1].is_string() {
                    utf8(&self.string(&c.operands[1])?)
                } else {
                    let slot = self.value(&c.operands[1])?;
                    self.slots.get(&slot).cloned().with_context(|| format!("LYIMAGE empty image slot {slot}"))?
                };
                self.asset("mgd", &name, "mgd")?;
                self.staged[l].image = Some(name);
                Flow::Next
            }
            // 0x436CC0 / CDIBSnrLayer::SetPos 0x43A400: x, y, camera depth.
            LYPOS => {
                let v = self.ints(c, 4)?;
                let l = self.layer_index(v[0])?;
                self.staged[l].pos = [v[1], v[2], v[3]];
                Flow::Next
            }
            LYBLEND | LYANTI => {
                let v = self.ints(c, 2)?;
                let l = self.layer_index(v[0])?;
                if c.id == LYBLEND {
                    self.staged[l].blend = v[1] != 0;
                } else {
                    // Anti-aliased (filtered) stretch flag; unscaled layers are unaffected.
                    self.staged[l].anti = v[1] != 0;
                }
                Flow::Next
            }
            // 0x437440: attach action slot value+1 (SetAction 0x43A540).
            LYACTION => {
                let v = self.ints(c, 2)?;
                let l = self.layer_index(v[0])?;
                self.staged[l].action = v[1] + 1;
                Flow::Next
            }
            OVERLAP => return self.overlap(h, c, now),
            // 0x439A00: save-thumbnail sampling settings only.
            SAVESAM => Flow::Next,
            // 0x437C80: name, wait, flag, fade ms. Loops; fades are not mapped.
            BGMPLAY => {
                Self::arity(c, 4)?;
                let name = if c.operands[0].is_string() { utf8(&self.string(&c.operands[0])?) } else { bail!("BGMPLAY BGMLIST index is not mapped") };
                let path = self.asset("bgm", &name, "ogg")?;
                self.play(h, BGM_CHANNEL, path, true)?;
                let flag = self.value(&c.operands[2])?.clamp(0, 255) as u8;
                self.bgm = Some((name, flag));
                Flow::Next
            }
            // 0x437FC0: wait flag, fade ms.
            BGMSTOP => {
                let v = self.ints(c, 2)?;
                h.submit_audio(self.client, AudioRequest::Stop { channel: BGM_CHANNEL, fade_ms: v[1].max(0) as f32 })?;
                self.bgm = None;
                if v[0] != 0 && self.phase == 0 {
                    self.phase = 1;
                    self.wait = Wait::Until(now + v[1].max(0) as u64);
                    return Ok(Flow::Yield);
                }
                Flow::Next
            }
            // 0x4390F0: SE slot ← name; yields one frame.
            SEREAD => {
                Self::arity(c, 2)?;
                let slot = self.value(&c.operands[0])?;
                let name = utf8(&self.string(&c.operands[1])?);
                self.asset("se", &name, "ogg")?;
                self.se.insert(slot, name);
                if self.phase == 0 {
                    self.phase = 1;
                    return Ok(Flow::Yield);
                }
                Flow::Next
            }
            // 0x438890: slot, fade ms, wait, loop.
            SEPLAY => {
                let v = self.ints(c, 4)?;
                let channel = SE_CHANNEL + u32::try_from(v[0]).ok().filter(|s| *s < 64).context("SE slot")?;
                if self.phase == 0 {
                    let name = self.se.get(&v[0]).cloned().with_context(|| format!("SEPLAY unread slot {}", v[0]))?;
                    let path = self.asset("se", &name, "ogg")?;
                    self.play(h, channel, path, v[3] != 0)?;
                    if v[3] != 0 {
                        self.se_loops.insert(v[0]);
                    }
                    if v[2] != 0 {
                        self.phase = 1;
                        return Ok(Flow::Yield);
                    }
                    return Ok(Flow::Next);
                }
                if !self.audio_pending.is_empty() || h.audio_playing(self.client, channel)? {
                    return Ok(Flow::Yield);
                }
                Flow::Next
            }
            // 0x438BE0: slot, fade ms, wait.
            SESTOP => {
                let v = self.ints(c, 3)?;
                let channel = SE_CHANNEL + u32::try_from(v[0]).ok().filter(|s| *s < 64).context("SE slot")?;
                h.submit_audio(self.client, AudioRequest::Stop { channel, fade_ms: v[1].max(0) as f32 })?;
                self.se_loops.remove(&v[0]);
                Flow::Next
            }
            // 0x438DD0: mode 2 waits until the slot stops.
            SEWAIT => {
                let v = self.ints(c, 2)?;
                ensure!(v[1] == 2, "SEWAIT mode {} is not mapped", v[1]);
                let channel = SE_CHANNEL + u32::try_from(v[0]).ok().filter(|s| *s < 64).context("SE slot")?;
                if !self.audio_pending.is_empty() || h.audio_playing(self.client, channel)? {
                    return Ok(Flow::Yield);
                }
                Flow::Next
            }
            // 0x4101F0 / 0x4102A0: open / close the message window.
            MWO | MWC => {
                Self::arity(c, 0)?;
                self.window_layout()?;
                let open = c.id == MWO;
                if self.window.open != open {
                    self.window.open = open;
                    if !open {
                        self.window.glyphs.clear();
                    }
                    let cmds = self.window_commands(h)?;
                    self.submit(h, cmds, vec![])?;
                }
                Flow::Next
            }
            // 0x439330: variable ← scene-replay mode (never in normal play).
            CHKSCEXEC => {
                let v = self.ints(c, 1)?;
                // +219140: set while the omake screen replays a CG/scene.
                self.set_var(v[0], i32::from(self.replay.is_some()))?;
                Flow::Next
            }
            // 0x439420: chapter title kept for save slots.
            SUBTITLE => {
                Self::arity(c, 1)?;
                self.subtitle = self.string(&c.operands[0])?;
                Flow::Next
            }
            SETCGFLAG | SETSCENEFLAG => {
                Self::arity(c, 1)?;
                let name = utf8(&self.string(&c.operands[0])?);
                // Names missing from CGLIST/SCENELIST set nothing (native loop falls through).
                if c.id == SETCGFLAG {
                    if let Some(i) = self.lists.cg_index(&name) {
                        self.system.cg[i] = 1;
                    }
                } else if let Some(i) = self.lists.scene_index(&name) {
                    self.system.scenes[i] = 1;
                }
                Flow::Next
            }
            // 0x411B50: index 0 text colour, 1 shadow colour.
            MESFTCOLOR => {
                let v = self.ints(c, 4)?;
                let slot = usize::try_from(v[0]).ok().filter(|i| *i < 2).context("MESFTCOLOR index")?;
                self.window.colors[slot] = [v[1] as u8, v[2] as u8, v[3] as u8, 255];
                Flow::Next
            }
            MESFTSHADOW => {
                let v = self.ints(c, 1)?;
                self.window.shadow = v[0] != 0;
                Flow::Next
            }
            MESFTSIZE => {
                let v = self.ints(c, 1)?;
                self.window_layout()?;
                if let Some(a) = &mut self.window.area {
                    a.size = v[0];
                }
                Flow::Next
            }
            MESTXKINSOKU => {
                let v = self.ints(c, 1)?;
                self.window_layout()?;
                if let Some(a) = &mut self.window.area {
                    a.kinsoku = v[0] != 0;
                }
                Flow::Next
            }
            // 0x411E50: indentation after an opening quote; recorded only.
            MESTXDOWN => {
                let v = self.ints(c, 1)?;
                self.window.down = v[0] != 0;
                Flow::Next
            }
            // 0x410410: speaker name plate number.
            MESNAME => {
                let v = self.ints(c, 1)?;
                self.window_layout()?;
                let count = match &self.window.names {
                    Some(n) => self.regions.get(&format!("mgd/{}.mgd", n.to_ascii_lowercase())).map_or(0, |r| r.len()),
                    None => 0,
                };
                ensure!(v[0] < count as i32, "MESNAME {} is outside the name sheet ({count} plates)", v[0]);
                self.window.name = v[0].max(-1);
                Flow::Next
            }
            // 0x4122A0: speaker id selecting the text colour preset.
            // 0x4122A0: speaker id; ids < 256 take their Novel.ini colour.
            SETCHARID => {
                let v = self.ints(c, 1)?;
                self.current_char = v[0];
                if let Some((_, [a, r, g, b])) = usize::try_from(v[0]).ok().and_then(|i| self.config_ex.chars.get(i)) {
                    self.window.colors[0] = [*r, *g, *b, *a];
                }
                Flow::Next
            }
            // 0x411F00: read-flag id, voice name ('' = none).
            MESVPLAY => {
                Self::arity(c, 2)?;
                let id = self.value(&c.operands[0])?;
                let name = utf8(&self.string(&c.operands[1])?);
                // 0x411F00: no voice while skipping (read text only, under the
                // skip-read setting) or with Ctrl, or when the speaker's voice is off.
                let script_name = cp932(&self.native_script_name(&script.name));
                let read = u32::try_from(id).is_ok_and(|id| self.system.read(&script_name, id));
                let kidoku_only = u32::from_le_bytes(self.system.settings[4..8].try_into()?) == 1;
                let skipping = self.window.skip_mode && (!kidoku_only || read);
                if let Ok(id) = u32::try_from(id) {
                    self.system.history_mut(&script_name).set(id)?;
                }
                let speaker_on = usize::try_from(self.current_char).map_or(true, |i| self.config_ex.chars.get(i).is_none_or(|c| c.0));
                if !name.is_empty() && !skipping && !self.force_skip && speaker_on {
                    let path = self.asset("voice", &name, "ogg")?;
                    self.play(h, VOICE_CHANNEL, path, false)?;
                }
                self.last_voice = (!name.is_empty()).then_some(name);
                Flow::Next
            }
            // 0x412140: stop the voice when the message advances.
            MESVSTOP => {
                Self::arity(c, 0)?;
                h.submit_audio(self.client, AudioRequest::Stop { channel: VOICE_CHANNEL, fade_ms: 0. })?;
                Flow::Next
            }
            // 0x4104D0: read-flag id, then strings and function codes.
            // 0x4104D0 → CDIBMesWnd::OutStart 0x405260 / MoveText 0x405F80.
            MESRUN => {
                ensure!((1..=25).contains(&c.operands.len()), "MESRUN operand count");
                let name = cp932(&self.native_script_name(&script.name));
                if self.phase == 0 {
                    let id = self.value(&c.operands[0])?;
                    self.window_layout()?;
                    let mut items = Vec::new();
                    for o in &c.operands[1..] {
                        items.push(if o.is_string() { Item::Text(self.string(o)?) } else { Item::Code(self.value(o)?) });
                    }
                    let area = self.window.area.context("message area")?;
                    self.window.tokens = message::layout(&items, area)?;
                    self.window.next = 0;
                    self.window.busy = true;
                    self.window.waiting_click = false;
                    self.window.msg_id = id;
                    self.window.read = u32::try_from(id).is_ok_and(|id| self.system.read(&name, id));
                    self.window.budget = 0.0;
                    self.window.paragraph = 0;
                    self.clicked = false;
                    self.phase = 1;
                }
                let dt = self.frame_dt;
                let mut changed = self.phase == 1;
                self.phase = 2;
                // OnClickWait 0x4035B0: skip mode advances (read text only when
                // the skip-read setting is on); a held Ctrl forces it.
                let kidoku_only = u32::from_le_bytes(self.system.settings[4..8].try_into()?) == 1;
                let skipping = self.force_skip || (self.window.skip_mode && (!kidoku_only || self.window.read));
                if self.window.waiting_click {
                    let voice = h.audio_playing(self.client, VOICE_CHANNEL).unwrap_or(false);
                    if !voice {
                        self.window.auto_left -= dt as i64;
                    }
                    let auto = self.window.auto_mode && self.window.auto_left <= 0;
                    if skipping || self.clicked || auto {
                        self.clicked = false;
                        self.window.waiting_click = false;
                        self.window.budget = 0.0;
                        changed = true;
                    }
                }
                if !self.window.waiting_click && self.window.busy {
                    // MovePutWord: 100·speed/255 (≥10) per mille characters per ms;
                    // 100 (speed 255), skip or a click show the rest of the run.
                    let speed = u32::from_le_bytes(self.system.settings[8..12].try_into()?).min(255) as f64;
                    let rate = (100.0 * speed / 255.0).floor().max(10.0);
                    let instant = rate >= 100.0 || skipping || self.window.skip_mode || std::mem::take(&mut self.clicked);
                    if instant {
                        changed |= self.reveal(None);
                    } else {
                        self.window.budget += rate * dt as f64 / 1000.0;
                        let n = self.window.budget.floor() as usize;
                        self.window.budget -= n as f64;
                        if n > 0 {
                            changed |= self.reveal(Some(n));
                        }
                    }
                }
                if changed {
                    let cmds = self.window_commands(h)?;
                    self.submit(h, cmds, vec![])?;
                }
                if self.window.busy || self.window.waiting_click {
                    return Ok(Flow::Yield);
                }
                // MovePutNull: the finished message is read.
                if let Ok(id) = u32::try_from(self.window.msg_id) {
                    self.system.history_mut(&name).set(id)?;
                }
                Flow::Next
            }
            // 0x436FE0: (layer, a, r, g, b), each clamped to 0..=255.
            LYCOLOR => {
                let v = self.ints(c, 5)?;
                let l = self.layer_index(v[0])?;
                let [a, r, g, b] = [v[1], v[2], v[3], v[4]].map(|x| x.clamp(0, 255) as u8);
                self.staged[l].color = ([r, g, b, a] != [255; 4]).then_some([r, g, b, a]);
                Flow::Next
            }
            // 0x4349D0: (slot, type, ...). Type 1 is Scroll: (duration ms,
            // loop, spline, then x,y,z points); other types are not used here.
            ACTMAKE => {
                ensure!(c.operands.len() >= 2, "ACTMAKE operands");
                let slot = self.value(&c.operands[0])?;
                ensure!((0..255).contains(&slot), "ACTMAKE slot {slot}");
                let kind = self.value(&c.operands[1])?;
                ensure!(kind == 1, "ACTMAKE type {kind} (CDIBSnrAction family {}) is not mapped", kind + 1);
                let v = self.ints(c, c.operands.len())?;
                ensure!(v.len() >= 8 && (v.len() - 5) % 3 == 0 && v.len() - 5 <= 48, "ACTMAKE scroll takes 1..=16 points");
                ensure!(v[4] == 0, "ACTMAKE scroll spline paths (CSpline) are not mapped");
                let points: Vec<[i32; 3]> = v[5..].chunks(3).map(|p| [p[0], p[1], p[2]]).collect();
                ensure!(points.iter().all(|p| p[2] == 0), "ACTMAKE scroll camera depth needs the camera transform");
                self.actions.insert(slot + 1, Action { duration: v[2].max(1) as u64, looping: v[3] != 0, points, elapsed: 0, playing: false });
                Flow::Next
            }
            // 0x4353E0 / 0x4354F0: start or stop the listed slots.
            ACTPLAY | ACTSTOP => {
                let v = self.ints(c, c.operands.len())?;
                for slot in v {
                    if let Some(a) = self.actions.get_mut(&(slot + 1)) {
                        a.playing = c.id == ACTPLAY;
                    }
                }
                self.refresh_actions(h)?;
                Flow::Next
            }
            // 0x4352C0: free the slot and detach layers that used it.
            ACTFREE => {
                let v = self.ints(c, 1)?;
                if self.actions.remove(&(v[0] + 1)).is_some() {
                    for l in self.staged.iter_mut().chain(self.shown.iter_mut()) {
                        if l.action == v[0] + 1 {
                            l.action = 0;
                        }
                    }
                    self.refresh_actions(h)?;
                }
                Flow::Next
            }
            // 0x410650 → MakeSelData: (index, enabled, x, y, five state cells).
            SELMAKE => {
                let v = self.ints(c, 9)?;
                let index = usize::try_from(v[0]).ok().filter(|i| *i < 10).context("SELMAKE choice index 0..=9")?;
                self.choices.insert(index, Choice { enabled: v[1] != 0, pos: [v[2], v[3]], cells: [v[4], v[5], v[6], v[7], v[8]] });
                Flow::Next
            }
            // 0x410860 → SelectStart / MoveSelect: (result variable, count,
            // parts sheet, hit mask). Choices 1.. slide from choice 0's
            // position and fade in; the picked index goes to the variable.
            SELRUN => {
                Self::arity(c, 4)?;
                if self.phase == 0 {
                    let var = self.value(&c.operands[0])?;
                    let count = usize::try_from(self.value(&c.operands[1])?).ok().filter(|n| (1..=10).contains(n)).context("SELRUN count 1..=10")?;
                    let sheet = utf8(&self.string(&c.operands[2])?);
                    for i in 0..count {
                        ensure!(self.choices.contains_key(&i), "SELRUN choice {i} has no SELMAKE");
                    }
                    let focus = (0..count).find(|i| self.choices[i].enabled).unwrap_or(0);
                    self.select = Some(Select { var, count, sheet, elapsed: 0, focus });
                    self.phase = 1;
                }
                match self.select_step(h)? {
                    None => Flow::Yield,
                    Some((var, picked)) => {
                        self.set_var(var, picked as i32)?;
                        self.select = None;
                        self.choices.clear();
                        Flow::Next
                    }
                }
            }
            // 0x410350 → SetMesSkip.
            SETMESSKIP => {
                let v = self.ints(c, 1)?;
                self.window.skip_mode = v[0] != 0;
                if v[0] != 0 {
                    self.window.auto_mode = false;
                }
                if let Some(p) = &mut self.window.panel {
                    p.controls[7].toggled = self.window.skip_mode;
                    p.controls[8].toggled = self.window.auto_mode;
                }
                Flow::Next
            }
            // 0x411240: back to the native title screen (not adopted yet).
            // 0x435630 → CRootFlow: back to the title menu (Initialize a2 ≠ 0).
            END => {
                if self.end_replay(h)? {
                    return Ok(Flow::Yield);
                }
                h.submit_audio(self.client, AudioRequest::Stop { channel: BGM_CHANNEL, fade_ms: 0.0 })?;
                self.clear_scene(h)?;
                self.title = Some(title::Title::menu());
                return Ok(Flow::Yield);
            }
            other => bail!("N-System command {} ({other}) at {}:{:#x} is not implemented", name(other), script.name, c.pc),
        })
    }
    fn layer_index(&self, v: i32) -> Result<usize> {
        usize::try_from(v).ok().filter(|l| *l < LAYERS).with_context(|| format!("N-System layer {v} out of range"))
    }
    /// 0x437810: frame 0 yields, frame 1 commits the back page and starts the
    /// effect (ReadOVParam 0x433E80: script type + 1), later frames wait.
    fn overlap(&mut self, h: &mut dyn HostServices, c: &Command, now: u64) -> Result<Flow> {
        match self.phase {
            0 => {
                self.phase = 1;
                Ok(Flow::Yield)
            }
            1 => {
                ensure!(!c.operands.is_empty(), "OVERLAP operands");
                let kind = self.value(&c.operands[0])? + 1;
                let (effect, duration) = match kind {
                    // Blt: immediate.
                    1 => (None, 0),
                    // Fade: duration operand + 1 (ReadOVParam case 2).
                    2 => {
                        let v = self.ints(c, 2)?;
                        (Some(json!({"kind":"crossfade"})), (v[1] + 1).max(1) as u64)
                    }
                    // Alpha: (ms, mode, rule, vague). Mode 2 is the soft rule
                    // wipe of DrawAlpha02 0x42B6F0, mode 1 the hard edge.
                    3 => {
                        ensure!(matches!(c.operands.len(), 4 | 5), "OVERLAP alpha operands");
                        let ms = self.value(&c.operands[1])?;
                        let mode = self.value(&c.operands[2])?;
                        let rule = utf8(&self.string(&c.operands[3])?);
                        let vague = if c.operands.len() == 5 { self.value(&c.operands[4])? } else { 255 }.clamp(0, 255);
                        let asset = self.asset("mgd", &rule, "mgd")?;
                        let effect = match mode {
                            1 => json!({"kind":"mask","asset":asset,"fade":0}),
                            2 if vague > 0 => json!({"kind":"mask","asset":asset,"vague":vague}),
                            2 => json!({"kind":"mask","asset":asset,"fade":0}),
                            other => bail!("OVERLAP alpha mode {other} (DrawAlpha0{other}) is not mapped"),
                        };
                        (Some(effect), ms.max(1) as u64)
                    }
                    // Rotate (InitRotate 0x42A570): (ms, direction).
                    5 => {
                        let v = self.ints(c, 3)?;
                        ensure!(matches!(v[2], 0 | 1), "OVERLAP rotate direction {}", v[2]);
                        (Some(json!({"kind":"spin_zoom","reverse":v[2] == 1})), v[1].max(1) as u64)
                    }
                    // Bound (InitBound 0x42C9E0): (ms, mode, count, vx, vy).
                    8 => {
                        let v = self.ints(c, 6)?;
                        ensure!(matches!(v[2], 0 | 1 | 10 | 11), "OVERLAP bound mode {} (DrawBound draws nothing)", v[2]);
                        ensure!((0..=1024).contains(&v[3]), "OVERLAP bound count {}", v[3]);
                        let vector = [i16::try_from(v[4])?, i16::try_from(v[5])?];
                        (Some(json!({"kind":"bounce","count":v[3],"vector":vector,"damped":v[2] >= 10,"stretch":v[2] % 10 == 1})), v[1].max(1) as u64)
                    }
                    // RotDoor (InitRotDoor 0x42AB20): (ms, axis); the
                    // MoveRotDoor camera sits 2500 px from the page.
                    14 => {
                        let v = self.ints(c, 3)?;
                        ensure!((0..=2).contains(&v[2]), "OVERLAP rotdoor axis {}", v[2]);
                        (Some(json!({"kind":"door_flip","axis":v[2],"focal":2500})), v[1].max(1) as u64)
                    }
                    other => bail!("OVERLAP effect type {other} is not mapped"),
                };
                // CDIBOverlap::Move 0x42ED30: time × FrameSpeed (½ at 0);
                // ConfigEx NoneEffect draws no effect.
                let fs = self.config_ex.frame_speed;
                let duration = if fs <= 0 { duration * 2 } else { duration / fs as u64 };
                let effect = effect.filter(|_| !self.config_ex.none_effect);
                let duration = if effect.is_none() { 0 } else { duration.max(1) };
                let mut cmds = self.commit_layers(h)?;
                if let Some(effect) = effect {
                    cmds.insert(0, json!({"op":"transition","transition":{"effect":effect,"duration_ms":duration.min(600_000)}}));
                }
                self.submit(h, cmds, vec![])?;
                self.phase = 2;
                self.wait = Wait::Until(now + duration);
                Ok(Flow::Yield)
            }
            _ => Ok(Flow::Next),
        }
    }
    fn step_inner(&mut self, h: &mut dyn HostServices, input: InputFrame, budget: u32) -> Result<VmYield> {
        if self.halted {
            return Ok(VmYield::Halted);
        }
        if self.scripts.is_empty() {
            if self.boot.is_none() {
                self.begin_boot(h)?;
            }
            match self.boot.as_mut().unwrap().try_take()? {
                None => return Ok(VmYield::BudgetExhausted),
                Some(boot) => {
                    self.scripts = boot.scripts;
                    self.regions = boot.regions;
                    if let Some(system) = boot.system {
                        self.vars[1000..2000].copy_from_slice(&system.globals);
                        self.system = system;
                    }
                    self.lists = self.read_lists()?;
                    if let Some(ini) = &boot.ini {
                        self.config_ex = crate::system::ConfigEx::parse_ini(ini);
                    }
                    let settings = self.system.settings;
                    self.apply_volumes(h, &settings)?;
                    if settings[0] != 0 {
                        self.submit(h, vec![], vec![json!({"op":"set_fullscreen","enabled":true})])?;
                    }
                    self.boot = None;
                    // WinMain → CDIBTitleScreen (logos, movie, menu).
                    self.title = Some(title::Title::boot());
                }
            }
        }
        self.poll_audio(h)?;
        let now = h.now_ms();
        let dt = self.last_ms.map_or(0, |l| now.saturating_sub(l));
        self.last_ms = Some(now);
        if let Some((_, elapsed)) = &mut self.timer {
            *elapsed += dt;
        }
        self.input = input;
        self.frame_dt = dt;
        self.force_skip = input.buttons_down & buttons::CTRL != 0;
        if let Some(q) = &mut self.timezone_query {
            match q.poll() {
                Ok(crysirror_core::service::CivilTimeZoneState::Pending) => {}
                Ok(crysirror_core::service::CivilTimeZoneState::Ready(p)) => {
                    self.timezone = Some(p);
                    self.timezone_query = None;
                }
                _ => self.timezone_query = None,
            }
        }
        if !self.poll_save(h)? {
            return Ok(VmYield::WaitUntil(now + 16));
        }
        if self.poll_load(h)? {
            return Ok(VmYield::WaitUntil(now + 16));
        }
        if !self.screens.is_empty() {
            return self.screen_step(h, now, input);
        }
        let consumed = self.title.is_none() && self.select.is_none() && self.window_input(h, input)?;
        let input = if consumed { InputFrame { buttons_pressed: 0, ..input } } else { input };
        if !self.screens.is_empty() || self.load_job.is_some() {
            return Ok(VmYield::WaitUntil(now + 16));
        }
        if let Some(sel) = &mut self.select {
            sel.elapsed += dt;
        }
        if self.actions.values().any(|a| a.playing) {
            self.tick_actions(dt);
            self.refresh_actions(h)?;
        }
        let pressed = self.select.is_none()
            && input.buttons_pressed & (buttons::MOUSE_LEFT | buttons::ENTER | buttons::SPACE) != 0;
        if pressed {
            self.clicked = true;
            self.input_flags[0] = true;
        }
        if self.title.is_some() {
            return self.title_step(h, now, dt, input);
        }
        match self.wait {
            Wait::Until(t) if now < t => return Ok(VmYield::WaitUntil(t)),
            _ => self.wait = Wait::None,
        }
        let mut steps = 0;
        while steps < budget {
            let script = self.script.clone().unwrap();
            let Some(c) = script.commands.get(self.ip).cloned() else {
                bail!("N-System script {} ran past its end", script.name);
            };
            steps += 1;
            self.executed += 1;
            let flow = self
                .execute(h, &c, now)
                .with_context(|| format!("{} {}:{:#x}", name(c.id), script.name, c.pc))?;
            match flow {
                Flow::Next => {
                    self.ip += 1;
                    self.phase = 0;
                }
                Flow::Jump(i) => {
                    self.ip = i;
                    self.phase = 0;
                }
                Flow::Stay => {}
                Flow::Yield => {
                    self.clicked = false;
                    if self.halted {
                        return Ok(VmYield::Halted);
                    }
                    return Ok(match self.wait {
                        Wait::Until(t) => VmYield::WaitUntil(t),
                        _ => VmYield::WaitUntil(now + 16),
                    });
                }
            }
        }
        Ok(VmYield::BudgetExhausted)
    }
}

struct Boot {
    scripts: BTreeMap<String, Arc<Script>>,
    regions: BTreeMap<String, Arc<Vec<[u16; 4]>>>,
    system: Option<crate::system::System>,
    ini: Option<String>,
}
/// `CSaveData::Initialize` 0x419C00 → `LoadConfig`; absent = defaults.
fn load_system(storage: &Arc<dyn ClientStorage>) -> Result<Option<crate::system::System>> {
    let path = VfsPath::new(Mount::Save, "CONFIG.SAV")?;
    let Some(meta) = storage.metadata(Mount::Save, path.relative())? else { return Ok(None) };
    ensure!(!meta.is_directory, "N-System CONFIG.SAV is a directory");
    let reader = storage.open_read(&path)?;
    let bytes = crysirror_resource::read_all(reader.as_ref(), crate::system::LIMIT)?;
    Ok(Some(crate::system::System::decode(&bytes).context("N-System CONFIG.SAV")?))
}
/// `.data` 0x5C2F30: CONFIG.SAV settings before any save.
pub(crate) const DEFAULT_SETTINGS: [u8; 32] = [0, 0, 0, 0, 1, 0, 0, 0, 0x80, 0, 0, 0, 0x80, 0, 0, 0, 1, 1, 1, 0, 0x80, 0, 0, 0, 0x80, 0, 0, 0, 0x80, 0, 0, 0];
/// Region tables of the MESWND parts and name sheets (setup keys 0 and 2).
fn load_window_regions(
    storage: Arc<dyn ClientStorage>,
    scripts: &BTreeMap<String, Arc<Script>>,
    images: &BTreeMap<String, String>,
    cancel: &crysirror_tasks::Cancellation,
) -> Result<BTreeMap<String, Arc<Vec<[u16; 4]>>>> {
    let mut out = BTreeMap::new();
    let mut sheets: Vec<&Vec<crate::script::TextPart>> = Vec::new();
    if let Some([crate::script::Operand::Text(parts), ..]) = scripts.get("title").and_then(|t| t.setup(0, 1)) {
        sheets.push(parts);
    }
    if let Some(setup) = scripts.get("meswnd") {
        for key in [0, 2] {
            if let Some([crate::script::Operand::Text(parts), ..]) = setup.setup(3, key) {
                sheets.push(parts);
            }
        }
    }
    // Native screen parts sheets: SetupStatic records (script, screen, key)
    // and names the EXE holds itself (FONT.MGD, CONF_P).
    for (script, screen, key) in [("saveload", 4, 2), ("option", 2, 1), ("option", 2, 6), ("option", 2, 8)] {
        if let Some([crate::script::Operand::Text(parts), ..]) = scripts.get(script).and_then(|t| t.setup(screen, key)) {
            sheets.push(parts);
        }
    }
    // OMAKE.MSD names every sheet it uses (parts, MUSIC, per-character grids).
    if let Some(om) = scripts.get("omake") {
        for c in om.commands.iter().filter(|c| c.id == 0) {
            for o in &c.operands {
                if let crate::script::Operand::Text(parts) = o {
                    let bytes: Vec<u8> = parts.iter().filter_map(|p| if let crate::script::TextPart::Bytes(b) = p { Some(b.clone()) } else { None }).flatten().collect();
                    let n = utf8(&bytes).to_ascii_lowercase();
                    let n = if n.ends_with(".mgd") { n } else { format!("{n}.mgd") };
                    if images.contains_key(&format!("mgd/{n}")) {
                        sheets.push(parts);
                    }
                }
            }
        }
    }
    let fixed: Vec<Vec<crate::script::TextPart>> =
        [&b"FONT.MGD"[..], b"CONF_P.MGD"].iter().map(|n| vec![crate::script::TextPart::Bytes(n.to_vec())]).collect();
    sheets.extend(fixed.iter());
    // SELRUN parts sheets are inline strings in every build seen.
    for script in scripts.values() {
        for c in script.commands.iter().filter(|c| c.id == opcodes::SELRUN) {
            if let Some(crate::script::Operand::Text(parts)) = c.operands.get(2) {
                sheets.push(parts);
            }
        }
    }
    for parts in sheets {
        let name: Vec<u8> = parts
            .iter()
            .filter_map(|p| if let crate::script::TextPart::Bytes(b) = p { Some(b.clone()) } else { None })
            .flatten()
            .collect();
        let mut name = utf8(&name).to_ascii_lowercase();
        if name.is_empty() {
            continue;
        }
        if !name.ends_with(".mgd") {
            name.push_str(".mgd");
        }
        if out.contains_key(&format!("mgd/{name}")) {
            continue;
        }
        cancel.check()?;
        let logical = format!("mgd/{name}");
        let path = images.get(&logical).with_context(|| format!("N-System window sheet {name}"))?;
        let source = storage.open_asset(&VfsPath::new(Mount::Asset, path)?)?;
        let bytes = crysirror_resource::read_all(source.as_ref(), 64 * 1024 * 1024)?;
        out.insert(logical, Arc::new(crysirror_formats_arc::images_c::mgd::regions(&bytes)?));
    }
    Ok(out)
}
fn load_scripts(
    storage: Arc<dyn ClientStorage>,
    entries: Vec<(String, String)>,
    title: &[u8],
    cancel: &crysirror_tasks::Cancellation,
) -> Result<BTreeMap<String, Arc<Script>>> {
    ensure!(entries.len() <= 4096, "N-System script count");
    let mut out = BTreeMap::new();
    for (name, path) in entries {
        cancel.check()?;
        let source = storage.open_asset(&VfsPath::new(Mount::Asset, &path)?)?;
        let mut bytes = crysirror_resource::read_all(source.as_ref(), crate::script::MAX_SCRIPT_BYTES as u64)?;
        crate::script::decrypt(&mut bytes, title).with_context(|| format!("{name}.MSD"))?;
        out.insert(name.clone(), Script::parse(&name, &bytes)?);
    }
    Ok(out)
}

impl VmAdapter for NSystemVm {
    fn engine_id(&self) -> &'static str {
        "nsystem"
    }
    fn snapshot_schema(&self) -> u32 {
        1
    }
    fn preparation_pending(&self) -> bool {
        self.scripts.is_empty()
    }
    fn step(&mut self, h: &mut dyn HostServices, input: InputFrame, budget: u32) -> crysirror_core::Result<VmYield> {
        self.step_inner(h, input, budget).map_err(|e| {
            let message = format!("{e:#}");
            self.error = Some(message.clone());
            Error::Io(message)
        })
    }
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Err(Error::Unsupported("N-System framework snapshot"))
    }
    fn restore(&mut self, _: &[u8]) -> crysirror_core::Result<()> {
        Err(Error::Unsupported("N-System framework snapshot"))
    }
    fn diagnostics(&self) -> Vec<u8> {
        serde_json::to_vec(&json!({"adapter":"nsystem","profile":"hananyu-2015","mode":"adopt",
            "script":self.script.as_ref().map(|s|s.name.clone()),"ip":self.ip,
            "pc":self.script.as_ref().and_then(|s|s.commands.get(self.ip)).map(|c|format!("{:#x}",c.pc)),
            "executed":self.executed,"window_open":self.window.open,"glyphs":self.window.glyphs.len(),
            "text":self.window.glyphs.iter().map(|g|g.text.as_str()).collect::<String>(),
            "subtitle":utf8(&self.subtitle),"title":self.title.as_ref().map(title::Title::step),"media":self.title.as_ref().and_then(|t|t.media.clone()),"now":self.last_ms,"visited":self.visited,"screens":self.screens.len(),"saving":self.save_job.is_some(),"loading":self.load_job.is_some(),"error":self.error}))
        .unwrap_or_default()
    }
}

#[cfg(feature = "runtime")]
pub fn register(registry: &mut crysirror_runtime::AdapterRegistry) -> Result<()> {
    registry.register("nsystem", Arc::new(Factory))
}
#[cfg(feature = "runtime")]
struct Factory;
#[cfg(feature = "runtime")]
impl crysirror_runtime::runtime::VmFactory for Factory {
    fn create(&self, client: ClientId, spec: &crysirror_runtime::model::ClientSpec) -> Result<Box<dyn VmAdapter>> {
        ensure!(spec.program.is_empty(), "N-System executes native MSD scripts, not data actions");
        Ok(Box::new(NSystemVm::new(client, serde_json::from_value(spec.config.clone())?)?))
    }
}

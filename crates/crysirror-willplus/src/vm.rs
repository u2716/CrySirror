// Copyright (c) 2026 CrySirror contributors. MIT; see LICENSE.
//! AdvHD WS2 executor over shared Layer, transition, glyph and audio services.
//!
//! Semantics come from the operand evidence in the installed scripts (the
//! shipped AdvHD.exe is packed, so its handlers were not read); every opcode
//! below is one whose effect the scripts make observable:
//!
//! * control: 0x01 conditional branch (operator byte: bit 7 selects a
//!   variable, else a flag; 2 `==`, 3 `!=`; target 0 = next), 0x02/0x06 jump,
//!   0x04 call script, 0x05 return, 0x07 jump to script, 0xFF script end;
//! * values: 0x09 variable operator (0 `=`, 1 `-`, 2 `+`, 3 `*`; bit 7 =
//!   operand is a variable index), 0x0B flag set, 0x0D/0x0E variable/flag
//!   range fill (start, count, value);
//! * layers: 0x33 image, 0x34 PNA, 0x39 PNA part ids, 0x37 erase (`*` all),
//!   0x3F stacking order (LAYER_ORDER.ws2), 0x46 set property group (0
//!   position, 1 rotation, 2 scale, 6 colour; flag byte bits 7..4 mark
//!   variable operands), 0x45 read a group into variables, 0x47 queue a
//!   tween (group 0 colour, 1 position, 2 scale, 3 rotation; seconds),
//!   0x48 start a named tween queue, 0x57 wait for a layer's tweens;
//! * screen: 0x64 prepare transition, 0x66 transition rule, 0x65 run it
//!   (duration seconds; last byte 1 waits);
//! * text (`message`): 0x15 speaker, 0x14 message into the Lua window
//!   (`\n`, `%K` click, `%P` page), 0x16 close, 0x0F choices, 0x32 face;
//! * time/audio: 0x11/0x12 named timer start/wait (byte 1 = click skips),
//!   0x1E BGM, 0x1F BGM stop, 0x20/0x2A volume, 0x28 sound/voice, 0x29
//!   sound stop, 0x35/0x3A movies;
//! * system: 0x1A/0x1C Lua calls (menus hold the script while paused),
//!   0x1B title system screens, 0x40 screen-space layer masks, 0x56/0x5C
//!   streak weather (inferred from its two uses).
//!
//! 0x42 layer attributes and 0x46 groups 10/11 are recorded without a
//! visible meaning; all other unlisted opcodes fail visibly.
use crate::program::{Instruction, Operand, Profile, Program};
use anyhow::{bail, ensure, Context, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{buttons, AudioAsset, AudioParameters, AudioRequest, HostServices, InputFrame, PcmBus},
    state::ClientId,
    vm::VmYield,
};
use crysirror_image::pna::Pna;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as Json};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub const CANVAS: [f32; 2] = [1280., 720.];
/// Save thumbnail extent (the data page frame is sized by the scripts'
/// PNA; this is the composed scene scaled to it).
pub const THUMB_SIZE: [f64; 2] = [192., 108.];
const ROOT: u32 = 0x5700_0000;
const MOVIE_MEDIA: u32 = 1;
const SYSTEM_ROOT: u32 = 0x5a00_0000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Source {
    Image(String),
    Pna { file: String, parts: Vec<i32> },
    /// Shared media surface of a playing movie.
    Movie { surface: String },
    /// 0x56 falling-streak overlay (rain), drawn from crysirror-vfx streaks.
    Streaks { effect: String, seed: u32, count: u32, speed: f32, length: f32, start: u64 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Layer {
    pub id: u32,
    pub source: Source,
    pub position: [f32; 4],
    /// Tween-space position (tween group 1), added to `position`.
    pub offset: [f32; 4],
    /// 0x40 screen-space mask image (coverage around the layer).
    pub mask: Option<String>,
    /// 0x42 attribute ids (recorded; see module notes).
    pub attributes: BTreeSet<i16>,
    /// 0x46 groups without a connected meaning (10, 11).
    pub extra: BTreeMap<i16, [f32; 4]>,
    pub rotation: [f32; 4],
    pub scale: [f32; 4],
    pub color: [f32; 4],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub group: i16,
    /// 1 = target is an offset from the value at track start; otherwise an
    /// absolute target (0 linear, 2 decelerate, 3 accelerate).
    pub mode: u8,
    pub target: [f32; 4],
    pub seconds: f32,
    pub delay: f32,
}

///One started tween queue: tracks run in order; `loops` < 0 repeats forever.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tween {
    pub layer: String,
    pub queue: String,
    pub tracks: Vec<Track>,
    pub index: usize,
    pub from: Option<[f32; 4]>,
    pub start: u64,
    pub loops: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Quake {
    pub kind: u8,
    pub amplitude: [f32; 2],
    pub period: f32,
    pub start: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum Wait {
    #[default]
    None,
    Until { deadline: u64, click: bool },
    Click,
    Tweens(String),
    Movie { skippable: bool },
    /// A Lua menu paused the game (0x1A openTitle and friends).
    System,
    /// Message reveal / click wait (`message`).
    Text,
    /// Choice list (`message`).
    Select,
    /// Message window open/close effect.
    WindowEffect,
}

///Maps keyed by (String, String) as JSON entry lists (JSON object keys
///must be strings).
mod pair_map {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::BTreeMap;
    pub fn serialize<V: Serialize, S: Serializer>(map: &BTreeMap<(String, String), V>, s: S) -> Result<S::Ok, S::Error> {
        map.iter().collect::<Vec<_>>().serialize(s)
    }
    pub fn deserialize<'de, V: Deserialize<'de>, D: Deserializer<'de>>(d: D) -> Result<BTreeMap<(String, String), V>, D::Error> {
        Ok(Vec::<((String, String), V)>::deserialize(d)?.into_iter().collect())
    }
}

///Saves written by earlier builds omit newer fields; they take defaults.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    pub scene: String,
    pub pc: u32,
    pub stack: Vec<(String, u32)>,
    pub vars: BTreeMap<u16, f32>,
    pub flags: BTreeSet<u16>,
    pub layers: BTreeMap<String, Layer>,
    pub order: Vec<String>,
    pub next_id: u32,
    pub timers: BTreeMap<String, u64>,
    #[serde(with = "pair_map")]
    pub queued: BTreeMap<(String, String), Vec<Track>>,
    pub tweens: Vec<Tween>,
    pub rule: Option<String>,
    pub speaker: String,
    /// Speaker as written (alignment codes kept for the name layout).
    pub speaker_raw: String,
    /// A log layer scrolled: the host runs `onLogDraw` after the Lua call.
    #[serde(skip)]
    pub log_redraw: bool,
    /// Voice of the line on screen (voice repeat).
    pub line_voice: Option<(String, String)>,
    pub master_volume: Option<i64>,
    #[serde(skip)]
    pub volume_update: bool,
    pub master_mute: bool,
    /// Host local UTC offset (seconds), sampled at start.
    #[serde(skip)]
    pub utc_offset: i32,
    /// `lua_DataSort` deletion marks (normal slots).
    pub data_marks: BTreeSet<i64>,
    pub body: String,
    /// 0x32 face chip of the current line ("" = none).
    pub face: String,
    /// 0x6E layer keys (`@NAME` -> layer name).
    pub keys: BTreeMap<String, String>,
    pub quake: Option<Quake>,
    pub movie: Option<String>,
    pub window: bool,
    pub wait: Wait,
    pub halted: bool,
    /// Host channels a named sound slot has used (for `*` stops).
    pub channels: BTreeSet<u32>,
    /// Instructions executed (diagnostics).
    pub executed: u64,
    /// `LegacyGame__lua_{Get,Set}GameState` values (host-assigned ids).
    pub game_state: BTreeMap<i64, bool>,
    /// `LegacyGame__lua_{Get,Set}String` slots.
    pub strings: BTreeMap<i64, String>,
    /// `cclass.lua_Layers` sets (Lua system layer).
    pub ui: crate::layers::Layers,
    /// `NeedProcess(LUA_PROCESS_GAME_PAUSED)` until `..._UNPAUSED`.
    pub paused: bool,
    /// Pointer in canvas pixels.
    pub cursor: [f64; 2],
    /// `lua_Registry` DWORD values by key (settings).
    pub registry_dwords: BTreeMap<String, i64>,
    /// `lua_MeasureTime` instance start times.
    pub measures: BTreeMap<u32, u64>,
    /// Lua system boot (initLegacyGame/initSystemScreen) done.
    pub system_ready: bool,
    /// `CLegacyFontInfo` styles by (set, key): font, size, colour, edge.
    #[serde(with = "pair_map")]
    pub fonts: BTreeMap<(String, String), FontStyle>,
    /// `lua_Sounds` slots started and not stopped.
    pub lua_sounds: BTreeSet<String>,
    pub msg: crate::message::Message,
    pub select: Option<crate::message::Select>,
    /// Message ids (0x14 first operand) already shown.
    pub read: BTreeSet<i32>,
    /// Choice ids already picked.
    pub chosen: BTreeSet<i16>,
    /// Recent messages for the log.
    pub log: Vec<LogEntry>,
    /// `NeedProcess` requests to run after the current Lua call.
    pub processes: Vec<(i64, i64)>,
    /// `SetSaveComment` text for the next save.
    pub save_comment: String,
    /// Looping/BGM slots playing (slot -> file, looping, volume tenths),
    /// restarted on load.
    pub music: BTreeMap<String, (String, bool, i16)>,
    /// Last voice (slot, file) for voice repeat.
    pub voice: Option<(String, String)>,
    /// Audio preparation receipts awaiting Ready (released then).
    #[serde(skip)]
    pub audio_pending: Vec<u64>,
    /// Save files read so far (None = empty slot).
    #[serde(skip)]
    pub saves: BTreeMap<String, Option<Arc<crate::save::SaveFile>>>,
    pub last_frame: Option<u64>,
    pub mouse_held: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LogEntry {
    pub speaker: String,
    pub text: String,
    pub voice: Option<(String, String)>,
    /// State at the line's start (scene jump-back and preview).
    #[serde(skip)]
    pub checkpoint: Option<Arc<State>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FontStyle {
    pub font: String,
    pub size: f64,
    pub frame: f64,
    pub color: u32,
    pub edge: u32,
}

pub struct Machine {
    pub client: ClientId,
    pub state: State,
    pub system: crate::system::System,
    pub res: Resources,
    pub(crate) programs: BTreeMap<String, Arc<Program>>,
    uploads: BTreeMap<(String, i32), (String, [u32; 2])>,
    next_upload: u32,
    published: BTreeSet<u32>,
    /// Parent of each published layer (ids are reassigned between flushes).
    parents: BTreeMap<u32, u64>,
    pub(crate) dirty: bool,
    /// State at the start of the current line/choice (what a save stores).
    pub(crate) checkpoint: Option<Box<State>>,
    pub(crate) checkpoint_text: String,
    pub(crate) global_loaded: bool,
    pub(crate) global_written: u64,
    lua_depth: u32,
    log_colors: Option<([u8; 4], [u8; 4], [u8; 4], [u8; 4])>,
    /// Host local-clock sample in flight (save dates and os.date use the
    /// local UTC offset it reports).
    civil: Option<Box<dyn crysirror_core::service::CivilTimeQuery>>,
    civil_started: bool,
    pub last_error: Option<String>,
}

pub(crate) fn string(program: &Program, operand: &Operand) -> Result<String> {
    let Operand::String(range) = operand else { bail!("WS2 string operand expected") };
    let (text, _, errors) = encoding_rs::SHIFT_JIS.decode(&program.bytes()[range.clone()]);
    ensure!(!errors, "WS2 string is not CP932");
    Ok(text.into_owned())
}
fn float(operand: &Operand) -> Result<f32> {
    match operand {
        Operand::FloatBits(bits) => Ok(f32::from_bits(*bits)),
        _ => bail!("WS2 float operand expected"),
    }
}
pub(crate) fn short(operand: &Operand) -> Result<i16> {
    match operand {
        Operand::Short(v) => Ok(*v),
        _ => bail!("WS2 short operand expected"),
    }
}
pub(crate) fn byte(operand: &Operand) -> Result<u8> {
    match operand {
        Operand::Byte(v) => Ok(*v),
        _ => bail!("WS2 byte operand expected"),
    }
}
fn address(operand: &Operand) -> Result<u32> {
    match operand {
        Operand::Address(v) => Ok(*v),
        _ => bail!("WS2 address operand expected"),
    }
}
fn ease(mode: u8, t: f32) -> f32 {
    match mode {
        2 => 1. - (1. - t) * (1. - t),
        3 => t * t,
        _ => t,
    }
}

impl Machine {
    pub fn new(client: ClientId, archives: Vec<String>, scene: &str) -> Self {
        Self {
            client,
            state: State { scene: scene.to_ascii_uppercase(), next_id: 1, ..Default::default() },
            system: Default::default(),
            res: Resources { client, archives, index: BTreeMap::new(), pna: BTreeMap::new() },
            programs: BTreeMap::new(),
            uploads: BTreeMap::new(),
            next_upload: 0x5800_0000,
            published: BTreeSet::new(),
            parents: BTreeMap::new(),
            dirty: true,
            checkpoint: None,
            checkpoint_text: String::new(),
            global_loaded: false,
            global_written: 0,
            lua_depth: 0,
            log_colors: None,
            civil: None,
            civil_started: false,
            last_error: None,
        }
    }

    fn resolve(&mut self, host: &mut dyn HostServices, name: &str) -> Result<String> {
        self.res.resolve(host, name)
    }

    ///Script.arc's Lua chunks: the info chunks, LegacyGame.lua, then the
    ///modules LegacyGame.inc lists, in that order.
    fn boot_system(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let read = |this: &mut Self, host: &mut dyn HostServices, name: &str| -> Result<Vec<u8>> {
            let path = this.resolve(host, name)?;
            Ok(host.storage(this.client)?.read(&VfsPath::new(Mount::Asset, &path)?)?.to_vec())
        };
        let mut names: Vec<String> = ["VariableSize.lua", "VersionInfo.lua", "GameInfo.lua", "LegacyGame.lua"].map(String::from).to_vec();
        let include = read(self, host, "LegacyGame.inc")?;
        for line in String::from_utf8_lossy(&include).lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let name = line.strip_prefix("include").map(str::trim).and_then(|n| n.strip_prefix('"')?.strip_suffix('"')).with_context(|| format!("LegacyGame.inc line {line:?}"))?;
            names.push(format!("{name}.lua"));
        }
        let mut chunks = Vec::new();
        for name in names {
            let bytes = read(self, host, &name)?;
            chunks.push((name, bytes));
        }
        self.system.install()?;
        let now = host.now_ms();
        let Machine { system, state, client, res, .. } = self;
        let crate::system::System { lua, registry, loaded } = system;
        let mut bridge = Bridge { state, registry, services: host, client: *client, res, now };
        crate::system::run_chunks(lua, &chunks, &mut bridge)?;
        *loaded = true;
        Ok(())
    }

    ///Call a global Lua function by name (WS2 0x1A/0x1C).
    pub fn lua_call(&mut self, host: &mut dyn HostServices, name: &str, args: Vec<crysirror_lua::Value>) -> Result<Vec<crysirror_lua::Value>> {
        if !self.system.loaded {
            self.boot_system(host).context("AdvHD Lua system boot")?;
        }
        if !self.state.system_ready && !matches!(name, "initLegacyGame" | "initSystemScreen" | "checkInitVersion") {
            self.state.system_ready = true;
            //g_checkInit starts false; initLegacyGame returns at once (no
            //settings loaded) unless the host first calls checkInitVersion.
            self.lua_call(host, "checkInitVersion", vec![])?;
            self.lua_call(host, "initLegacyGame", vec![crysirror_lua::Value::str("YadorisiOtome")])?;
            self.lua_call(host, "initSystemScreen", vec![])?;
            if std::env::var_os("WILLPLUS_TEXT_TRACE").is_some() {
                eprintln!("WILLPLUS init g_msgspeed={:?} g_checkInit={:?} registry={:?}", self.system.lua.global("g_msgspeed"), self.system.lua.global("g_checkInit"), self.state.registry_dwords);
            }
        }
        let now = host.now_ms();
        let Machine { system, state, client, res, .. } = self;
        let crate::system::System { lua, registry, .. } = system;
        let function = lua.global(name);
        ensure!(matches!(function, crysirror_lua::Value::Function(_)), "AdvHD Lua function {name:?} is not defined");
        let mut bridge = Bridge { state, registry, services: host, client: *client, res, now };
        let result = lua.call(&function, args, &mut bridge).with_context(|| format!("AdvHD Lua {name}"));
        if lua.allocations() > 50_000 {
            lua.collect(&[]);
        }
        //Lua never re-enters the host's lua_call, so the outermost call
        //owns the queue; processes may call Lua again (and queue more).
        if self.lua_depth == 0 && result.is_ok() && std::mem::take(&mut self.state.log_redraw) {
            self.lua_depth += 1;
            let drawn = self.lua_call(host, "onLogDraw", vec![]);
            self.lua_depth -= 1;
            drawn?;
        }
        if self.lua_depth == 0 && result.is_ok() && std::mem::take(&mut self.state.volume_update) {
            self.lua_depth += 1;
            let applied = self.apply_volumes(host);
            self.lua_depth -= 1;
            applied?;
        }
        if self.lua_depth == 0 && result.is_ok() && !self.state.processes.is_empty() {
            self.lua_depth += 1;
            let ran = self.run_processes(host);
            self.lua_depth -= 1;
            ran?;
        }
        result
    }

    pub(crate) fn program(&mut self, host: &mut dyn HostServices, scene: &str) -> Result<Arc<Program>> {
        let key = scene.to_ascii_uppercase();
        if let Some(program) = self.programs.get(&key) {
            return Ok(program.clone());
        }
        let path = self.resolve(host, &format!("{scene}.ws2"))?;
        let raw = host.storage(self.client)?.read(&VfsPath::new(Mount::Asset, &path)?)?;
        ensure!(raw.len() <= crate::program::MAX_SCRIPT_BYTES, "WS2 byte budget");
        let bytes: Arc<[u8]> = raw.iter().map(|b| b.rotate_right(2)).collect::<Vec<_>>().into();
        let program = Arc::new(Program::parse(Profile::V3, bytes, || Ok(()))?);
        self.programs.insert(key, program.clone());
        Ok(program)
    }

    fn var(&self, index: i16) -> f32 {
        u16::try_from(index).ok().and_then(|i| self.state.vars.get(&i).copied()).unwrap_or(0.)
    }
    fn set_var(&mut self, index: i16, value: f32) -> Result<()> {
        let index = u16::try_from(index).context("WS2 variable index")?;
        self.state.vars.insert(index, value);
        Ok(())
    }
    ///Group operands whose flag bit (7 - n) is set are variable indices.
    fn operands4(&self, flags: u8, values: [f32; 4]) -> [f32; 4] {
        let mut out = values;
        for (n, value) in out.iter_mut().enumerate() {
            if flags & (0x80 >> n) != 0 {
                *value = self.var(*value as i16);
            }
        }
        out
    }
    ///`@KEY` names resolve through 0x6E keys.
    fn layer_name(&self, name: &str) -> Result<String> {
        if name.starts_with('@') {
            let target = self.state.keys.get(name).with_context(|| format!("AdvHD layer key {name:?} is not bound"))?;
            return Ok(target.clone());
        }
        Ok(name.to_owned())
    }
    fn layer_mut(&mut self, name: &str) -> Result<&mut Layer> {
        let name = self.layer_name(name)?;
        self.state.layers.get_mut(&name).with_context(|| format!("AdvHD layer {name:?} is not loaded"))
    }
    fn group(layer: &mut Layer, group: i16, tween: bool) -> Result<&mut [f32; 4]> {
        Ok(match (tween, group) {
            (false, 0) => &mut layer.position,
            (true, 1) | (true, 10) => &mut layer.offset,
            (false, 1) | (true, 3) => &mut layer.rotation,
            (false, 2) | (true, 2) => &mut layer.scale,
            (false, 6) | (true, 0) => &mut layer.color,
            _ => bail!("AdvHD layer property group {group} ({}) is not connected", if tween { "tween" } else { "set" }),
        })
    }

    fn put_layer(&mut self, name: &str, source: Source) {
        let id = match self.state.layers.get(name) {
            Some(layer) => layer.id,
            None => {
                let id = self.state.next_id;
                self.state.next_id += 1;
                id
            }
        };
        self.state.layers.insert(
            name.to_owned(),
            Layer { id, source, position: [0.; 4], offset: [0.; 4], mask: None, attributes: BTreeSet::new(), extra: BTreeMap::new(), rotation: [0.; 4], scale: [1., 1., 1., 0.], color: [1.; 4] },
        );
        self.dirty = true;
    }

    fn pna(&mut self, host: &mut dyn HostServices, file: &str) -> Result<Arc<(Pna, Vec<u8>)>> {
        self.res.pna(host, file)
    }

    fn part_image(&mut self, host: &mut dyn HostServices, file: &str, id: i32) -> Result<Option<(String, [u32; 2], [i32; 2])>> {
        let pna = self.pna(host, file)?;
        let Some(part) = pna.0.parts.iter().find(|part| part.id == id) else { return Ok(None) };
        let key = (file.to_owned(), id);
        if !self.uploads.contains_key(&key) {
            let upload = self.next_upload;
            self.next_upload += 1;
            let image = host.upload_encoded_image(self.client, upload, &format!("{file}#{id}.png"), &pna.1[part.png.clone()])?;
            self.uploads.insert(key.clone(), (image.path.relative().to_owned(), image.size));
        }
        let (path, size) = self.uploads[&key].clone();
        Ok(Some((path, size, part.offset)))
    }


    ///Publish the retained layer state.
    pub fn flush(&mut self, host: &mut dyn HostServices) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }
        let shake = self.shake(host.now_ms());
        let mut ops = vec![json!({"op":"put","layer":{"id":ROOT,"rect":[shake[0],shake[1],CANVAS[0],CANVAS[1]],"order":[0,0],"paint":{"kind":"solid","color":[0,0,0,255]}}})];
        let mut live = BTreeSet::from([ROOT]);
        let layers: Vec<(String, Layer)> = self.state.layers.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        let order = self.state.order.clone();
        for (name, layer) in layers {
            let root = 0x5710_0000 + layer.id * 64;
            self.scene_layer(host, &name, &layer, &order, root, ROOT, &mut ops, &mut live)?;
        }
        self.flush_system(host, &mut ops, &mut live)?;
        flatten_order(&mut ops);
        //A put does not move an existing layer to another parent, so an id
        //reused under a different parent is removed and created afresh.
        let mut parents = BTreeMap::new();
        for op in &ops {
            if let Some(id) = op["layer"]["id"].as_u64() {
                parents.insert(id as u32, op["layer"]["parent"].as_u64().unwrap_or(u64::MAX));
            }
        }
        let mut removes: Vec<u32> = self.published.difference(&live).copied().collect();
        removes.extend(parents.iter().filter(|(id, parent)| self.parents.get(id).is_some_and(|old| old != *parent)).map(|(id, _)| *id));
        removes.sort_unstable();
        removes.dedup();
        //Children first: removing a parent also drops its subtree.
        let depth = |mut id: u32| {
            let mut n = 0;
            while let Some(parent) = self.parents.get(&id) {
                n += 1;
                id = *parent as u32;
                if n > 64 {
                    break;
                }
            }
            n
        };
        removes.sort_by_key(|id| std::cmp::Reverse(depth(*id)));
        ops.splice(0..0, removes.into_iter().map(|id| json!({"op":"remove","id":id})));
        self.parents = parents;
        host.submit_engine(self.client, 1, &serde_json::to_vec(&json!({"layers":ops}))?)?;
        self.published = live;
        self.dirty = false;
        Ok(())
    }

    ///Ops for one WS2 layer under `parent` (ids root..root+63).
    #[allow(clippy::too_many_arguments)]
    fn scene_layer(&mut self, host: &mut dyn HostServices, name: &str, layer: &Layer, order: &[String], root: u32, parent: u32, ops: &mut Vec<Json>, live: &mut BTreeSet<u32>) -> Result<()> {
            let mut leaves: Vec<(String, [u32; 2], [i32; 2])> = Vec::new();
            match &layer.source {
                Source::Image(path) => {
                    let size = host.image_size(self.client, path)?;
                    leaves.push((path.clone(), size, [0, 0]));
                }
                Source::Movie { surface } => leaves.push((surface.clone(), [CANVAS[0] as u32, CANVAS[1] as u32], [0, 0])),
                Source::Streaks { .. } => {}
                Source::Pna { file, parts } => {
                    let pna = self.pna(host, file)?;
                    //Later records are lower in the stack (body last).
                    //0x39 lists PNA record indices (body, eyes, face), as the
                    //Lua layer's sublayer numbers do.
                    let order: Vec<usize> = pna.0.parts.iter().rev().map(|part| part.index).filter(|index| parts.contains(&(*index as i32))).collect();
                    for record in order {
                        if let Some(leaf) = self.record_image(host, file, record)? {
                            leaves.push(leaf);
                        }
                    }
                }
            }
            let extent = match &layer.source {
                Source::Image(_) | Source::Movie { .. } => leaves.first().map_or([1, 1], |leaf| leaf.1),
                Source::Streaks { .. } => [CANVAS[0] as u32, CANVAS[1] as u32],
                Source::Pna { file, .. } => self.pna(host, file)?.0.canvas,
            };
            let [r, g, b, a] = layer.color.map(|v| (v.clamp(0., 1.) * 255.).round() as u8);
            let rank = order.iter().position(|item| item.eq_ignore_ascii_case(name)).map_or(500, |n| n as i32 + 1);
            //0x40 mask: a screen-space image (white shows, black hides) in
            //the client coverage plane around this layer's draws only.
            if let Some(mask) = &layer.mask {
                ops.push(json!({"op":"put","layer":{"id":root + 62,"parent":parent,"rect":[0,0,CANVAS[0],CANVAS[1]],"order":[rank,0],
                    "paint":{"kind":"coverage_write","paint":{"kind":"image","asset":mask}}}}));
                ops.push(json!({"op":"put","layer":{"id":root + 63,"parent":parent,"rect":[0,0,CANVAS[0],CANVAS[1]],"order":[rank,2],
                    "paint":{"kind":"coverage_reset","value":255}}}));
                live.insert(root + 62);
                live.insert(root + 63);
            }
            ops.push(json!({"op":"put","layer":{
                "id":root,"parent":parent,"rect":[layer.position[0]+layer.offset[0],layer.position[1]+layer.offset[1],extent[0],extent[1]],
                "order":[rank,1],"opacity":a,"paint":{"kind":"solid","color":[0,0,0,0]},
                "transform":{"scale_x":layer.scale[0],"scale_y":layer.scale[1],"rotate":layer.rotation[2].to_radians(),
                    "pivot_x":extent[0] as f32/2.,"pivot_y":extent[1] as f32/2.}}}));
            live.insert(root);
            if let Source::Streaks { seed, count, speed, length, start, .. } = &layer.source {
                let field = crysirror_vfx::streaks::StreakField { seed: *seed, count: *count, area: CANVAS, speed: *speed, slant: 0.08, length: *length };
                let base = 0x5c00_0000 + (layer.id & 0xff) * 0x1000;
                for (n, streak) in field.at(host.now_ms().saturating_sub(*start)).into_iter().enumerate() {
                    let id = base + n as u32;
                    let alpha = (streak.opacity * 160.) as u8;
                    ops.push(json!({"op":"put","layer":{"id":id,"parent":root,"rect":[streak.from[0],streak.from[1],1.5,streak.to[1]-streak.from[1]],
                        "order":[n as i32,0],"transform":{"rotate":-(streak.to[0]-streak.from[0]).atan2(streak.to[1]-streak.from[1]),"pivot_x":0,"pivot_y":0},
                        "paint":{"kind":"solid","color":[r,g,b,alpha]}}}));
                    live.insert(id);
                }
            }
            for (n, (path, size, offset)) in leaves.into_iter().enumerate() {
                let id = root + 1 + n as u32;
                ops.push(json!({"op":"put","layer":{"id":id,"parent":root,"rect":[offset[0],offset[1],size[0],size[1]],
                    "order":[n as i32,0],"paint":{"kind":"image","asset":path,"tint":[r,g,b,255]}}}));
                live.insert(id);
            }
        Ok(())
    }

    ///Image of one PNA record (by record index).
    fn record_image(&mut self, host: &mut dyn HostServices, file: &str, record: usize) -> Result<Option<(String, [u32; 2], [i32; 2])>> {
        let pna = self.pna(host, file)?;
        let Some(id) = pna.0.parts.iter().find(|p| p.index == record).map(|p| p.id) else { return Ok(None) };
        self.part_image(host, file, id)
    }

    ///Compose `cclass.lua_Layers` sets above the scene (add order = stacking).
    fn flush_system(&mut self, host: &mut dyn HostServices, ops: &mut Vec<Json>, live: &mut BTreeSet<u32>) -> Result<()> {
        use crate::layers::Kind;
        let mut sets = self.state.ui.sets.clone();
        if let Some(overlay) = &self.state.ui.overlay {
            if let Some(at) = sets.iter().position(|s| &s.name == overlay) {
                let set = sets.remove(at);
                sets.push(set);
            }
        }
        let mut next = SYSTEM_ROOT + 1;
        let mut id = || {
            next += 1;
            next
        };
        if sets.is_empty() {
            return Ok(());
        }
        ops.push(json!({"op":"put","layer":{"id":SYSTEM_ROOT,"parent":ROOT,"rect":[0,0,CANVAS[0],CANVAS[1]],"order":[20000,0],"paint":{"kind":"solid","color":[0,0,0,0]}}}));
        live.insert(SYSTEM_ROOT);
        for (n, set) in sets.iter().enumerate() {
            let alpha = (set.opacity.clamp(0., 1.) * 255.).round() as u8;
            if alpha == 0 {
                continue;
            }
            let group = id();
            if std::env::var_os("WILLPLUS_SET_TRACE").is_some() {
                eprintln!("WILLPLUS set {n} {} group={group:#x} alpha={alpha} items={:?}", set.name, set.items.iter().filter(|i| i.visible).map(|i| &i.key).collect::<Vec<_>>());
            }
            ops.push(json!({"op":"put","layer":{"id":group,"parent":SYSTEM_ROOT,"rect":[0,0,CANVAS[0],CANVAS[1]],"order":[n as i32,0],"opacity":alpha,"paint":{"kind":"solid","color":[0,0,0,0]}}}));
            live.insert(group);
            for (m, item) in set.items.iter().enumerate() {
                if !item.visible {
                    continue;
                }
                let p = &item.params;
                let tint = |c: [f64; 4]| -> [u8; 4] { c.map(|v| (v.clamp(0., 1.) * 255.).round() as u8) };
                let item_id = id();
                let [ix, iy] = [p.pos[0], p.pos[1]];
                let base = json!({"id":item_id,"parent":group,"rect":[ix,iy,CANVAS[0],CANVAS[1]],"order":[m as i32,0],"opacity":tint(p.color)[3],
                    "transform":{"scale_x":p.scale[0],"scale_y":p.scale[1],"rotate":p.rotate[2].to_radians()},"paint":{"kind":"solid","color":[0,0,0,0]}});
                ops.push(json!({"op":"put","layer":base}));
                live.insert(item_id);
                let mut leaves: Vec<(String, [u32; 2], [f64; 2], [u8; 4])> = Vec::new();
                match &item.kind {
                    Kind::Std { path } => {
                        let resolved = self.resolve(host, path)?;
                        let size = host.image_size(self.client, &resolved)?;
                        leaves.push((resolved, size, [0., 0.], [255; 4]));
                    }
                    Kind::Pna { path } => {
                        //Higher record indices are lower in the stack.
                        for (record, sub) in item.subs.iter().rev() {
                            if !sub.visible || sub.params.color[3] <= 0. {
                                continue;
                            }
                            if let Some((asset, size, offset)) = self.record_image(host, path, *record)? {
                                let c = tint(sub.params.color);
                                leaves.push((asset, size, sub.origin(offset), c));
                            }
                        }
                    }
                    Kind::Log { .. } => {
                        let font = self.state.fonts.get(&(set.name.clone(), item.key.clone())).cloned();
                        let Some(g) = crate::log::geometry(item, font.as_ref().map(|f| f.size)) else { continue };
                        let lines = crate::log::lines(&self.state.log, &g)?;
                        let Kind::Log { top, .. } = &item.kind else { continue };
                        let top = if *top == usize::MAX { lines.len().saturating_sub(g.view()) } else { *top };
                        let colors = self.log_colors(host)?;
                        let (fill, edge) = match &font {
                            Some(f) => (rgba(f.color), (f.frame > 0.).then(|| rgba(f.edge))),
                            None => ([255; 4], None),
                        };
                        for (row, line) in lines.iter().skip(top).take(g.view()).enumerate() {
                            let entry = &self.state.log[line.entry()];
                            let (body, color, edge) = match line {
                                crate::log::Line::Name(_) => (entry.speaker.clone(), colors.0, Some(colors.1)),
                                crate::log::Line::Text(_, row) if entry.voice.is_some() => (row.clone(), colors.2, Some(colors.3)),
                                crate::log::Line::Text(_, row) => (row.clone(), fill, edge),
                                crate::log::Line::Blank(_) => continue,
                            };
                            let glyphs = layout_glyphs(&body, g.width(), g.font, 0., 0)?;
                            let leaf = id();
                            let mut paint = json!({"kind":"glyph_run","font":"default","size":g.font,"color":color,"glyphs":glyphs});
                            if let Some(edge) = edge {
                                paint["outline"] = json!({"radius":1,"color":edge,"kernel":"square"});
                            }
                            ops.push(json!({"op":"put","layer":{"id":leaf,"parent":item_id,"rect":[g.margins[0],g.margins[1] + row as f64 * g.pitch,g.width(),g.pitch],"order":[row as i32,0],"paint":paint}}));
                            live.insert(leaf);
                        }
                    }
                    Kind::Thumbnail { .. } | Kind::Preview { .. } | Kind::LogPreview { .. } => {
                        let (scene, size) = match &item.kind {
                            Kind::Thumbnail { path } => {
                                let save = path.strip_prefix(crate::save::THUMB_PREFIX).and_then(|rest| rest.split_once('/')).and_then(|(m, n)| Some((m.parse::<i64>().ok()?, n.parse::<i64>().ok()?)));
                                (save.and_then(|(m, n)| self.read_save(host, m, n)).map(|s| (s.state.layers.clone(), s.state.order.clone())), THUMB_SIZE)
                            }
                            Kind::Preview { size } => (Some((self.state.layers.clone(), self.state.order.clone())), *size),
                            Kind::LogPreview { entry, size } => (self.state.log.get(*entry).and_then(|e| e.checkpoint.clone()).map(|c| (c.layers.clone(), c.order.clone())), *size),
                            _ => unreachable!(),
                        };
                        if let Some((layers, order)) = scene {
                            let group = id();
                            ops.push(json!({"op":"put","layer":{"id":group,"parent":item_id,"rect":[0,0,CANVAS[0],CANVAS[1]],"order":[0,0],"viewport_clip":[ix,iy,size[0],size[1]],
                                "transform":{"scale_x":size[0] / f64::from(CANVAS[0]),"scale_y":size[1] / f64::from(CANVAS[1]),"pivot_x":0,"pivot_y":0},"paint":{"kind":"solid","color":[0,0,0,255]}}}));
                            live.insert(group);
                            let mut base = 0x5b00_0000 + (group & 0xff) * 0x10000;
                            for (name, layer) in &layers {
                                self.scene_layer(host, name, layer, &order, base, group, ops, live)?;
                                base += 64;
                            }
                        }
                    }
                    Kind::PnaRef { path, record } => {
                        if let Some((asset, size, _)) = self.record_image(host, path, *record)? {
                            leaves.push((asset, size, [0., 0.], [255; 4]));
                        }
                    }
                    Kind::Text { size, margins, line, text, font_size, color, edge, reveal } => {
                        let style = self.state.fonts.get(&(set.name.clone(), item.key.clone())).cloned();
                        let (body, visible) = if set.name == "system" && item.key == "Txt01" {
                            (self.state.msg.page.clone(), Some(self.message_visible(host.now_ms())))
                        } else if set.name == "system" && item.key == "Name01" {
                            (if self.state.speaker_raw.is_empty() { self.state.speaker.clone() } else { self.state.speaker_raw.clone() }, None)
                        } else {
                            let shown = reveal.map(|start| (host.now_ms().saturating_sub(start) / self.state.ui.text_interval.max(1)) as usize);
                            (text.clone(), shown)
                        };
                        let (align, body) = crate::message::alignment(&body);
                        //Choice items: the scripts size SelTxt to the button
                        //image and place it at the button origin; the text is
                        //centred inside it.
                        let centred = set.name == "system" && item.key.starts_with("SelTxt");
                        let align = if centred { 1 } else { align };
                        //Script text writes newlines as `\n` and ruby as {base:reading}.
                        let body = body.replace("\\n", "\n");
                        let body = if body.contains('%') { Self::message(&body).unwrap_or_else(|_| body.clone()) } else { body };
                        let (body, rubies) = ruby_spans(&body);
                        if !body.is_empty() {
                            let (px, fill, outline) = match &style {
                                Some(f) => (f.size, rgba(f.color), (f.frame > 0.).then(|| rgba(f.edge))),
                                None => (*font_size, *color, edge.map(rgba)),
                            };
                            let width = (size[0] - margins[0] - margins[2]).max(1.);
                            let mut glyphs = layout_glyphs(&body, width, px, *line, align)?;
                            if centred {
                                let height = (size[1] - margins[1] - margins[3]).max(1.);
                                let lines = glyphs.iter().filter_map(|g| g["baseline"][1].as_f64()).fold(0f64, f64::max) + px / 8.;
                                let shift = ((height - lines) / 2.).max(0.).round();
                                for g in &mut glyphs {
                                    let y = g["baseline"][1].as_f64().unwrap_or(0.);
                                    g["baseline"][1] = json!(y + shift);
                                }
                            }
                            if !rubies.is_empty() {
                                glyphs = with_ruby(glyphs, &rubies, px);
                            }
                            let shown = visible.unwrap_or(glyphs.len()).min(glyphs.len());
                            if visible.is_some_and(|v| v < glyphs.len()) {
                                self.dirty = true;
                            }
                            let leaf = id();
                            let mut paint = json!({"kind":"glyph_run","font":"default","size":px.clamp(4., 200.),"color":fill,"glyphs":&glyphs[..shown]});
                            if let Some(edge) = outline {
                                paint["outline"] = json!({"radius":1,"color":edge,"kernel":"square"});
                            }
                            let op = json!({"op":"put","layer":{"id":leaf,"parent":item_id,"rect":[margins[0],margins[1],width,(size[1]-margins[1]-margins[3]).max(1.)],"order":[0,0],"paint":paint}});
                            if std::env::var_os("WILLPLUS_TEXT_TRACE").is_some() {
                                eprintln!("WILLPLUS text {}/{} shown={} start={} interval={} now={} {}", set.name, item.key, self.state.msg.shown, self.state.msg.start, self.state.msg.interval, host.now_ms(), op);
                            }
                            ops.push(op);
                            live.insert(leaf);
                        }
                    }
                }
                for (k, (asset, size, offset, c)) in leaves.into_iter().enumerate() {
                    let leaf = id();
                    ops.push(json!({"op":"put","layer":{"id":leaf,"parent":item_id,"rect":[offset[0],offset[1],size[0],size[1]],"order":[k as i32,0],"opacity":c[3],
                        "paint":{"kind":"image","asset":asset,"tint":[c[0],c[1],c[2],255]}}}));
                    live.insert(leaf);
                }
            }
        }
        Ok(())
    }

    ///WS2 sound: volume operands are tenths, scaled by the scripts'
    ///per-slot setting (`getSoundVolume`, 0..1000).
    ///Log name / voice-line colours (`getMessageLogTextColor(0|1)`).
    fn log_colors(&mut self, host: &mut dyn HostServices) -> Result<([u8; 4], [u8; 4], [u8; 4], [u8; 4])> {
        if let Some(colors) = self.log_colors {
            return Ok(colors);
        }
        let mut out = [[255u8; 4]; 4];
        for kind in 0..2 {
            let values = self.lua_call(host, "getMessageLogTextColor", vec![crysirror_lua::Value::Int(kind)])?;
            for n in 0..2 {
                out[kind as usize * 2 + n] = rgba(values.get(n).and_then(crysirror_lua::Value::as_f64).unwrap_or(4294967295.) as u32);
            }
        }
        let colors = (out[0], out[1], out[2], out[3]);
        self.log_colors = Some(colors);
        Ok(colors)
    }

    ///Re-apply setting gains to the looping slots still playing.
    fn apply_volumes(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let master = if self.state.master_mute { 0. } else { self.state.master_volume.unwrap_or(1000) as f32 / 1000. };
        for (slot, (_, _, volume)) in self.state.music.clone() {
            let setting = self.lua_call(host, "getSoundVolume", vec![crysirror_lua::Value::str(&slot)])?.first().and_then(crysirror_lua::Value::as_f64).unwrap_or(1000.);
            let gain = f32::from(volume.clamp(0, 10)) / 10. * (setting / 1000.).clamp(0., 1.) as f32 * master;
            host.submit_audio(self.client, AudioRequest::SetGain { channel: sound_channel(&slot), gain, fade_ms: 0. })?;
        }
        Ok(())
    }

    pub(crate) fn play(&mut self, host: &mut dyn HostServices, channel: u32, slot: &str, file: &str, looping: bool, music: bool, volume: i16) -> Result<()> {
        let setting = if self.system.loaded { self.lua_call(host, "getSoundVolume", vec![crysirror_lua::Value::str(slot)])?.first().and_then(crysirror_lua::Value::as_f64).unwrap_or(1000.) } else { 1000. };
        let gain = f32::from(volume.clamp(0, 10)) / 10. * (setting / 1000.).clamp(0., 1.) as f32;
        let path = self.resolve(host, file)?;
        let bus = if music { PcmBus::Music } else if slot.starts_with("char") { PcmBus::Voice } else { PcmBus::Sfx };
        start_audio(host, self.client, &mut self.state, channel, &path, looping, gain, bus)?;
        if looping {
            self.state.music.insert(slot.to_owned(), (file.to_owned(), looping, volume));
        }
        Ok(())
    }
    fn channel(&self, name: &str) -> u32 {
        sound_channel(name)
    }

    fn tick_tweens(&mut self, now: u64) -> Result<()> {
        if self.state.tweens.is_empty() {
            return Ok(());
        }
        let tweens = std::mem::take(&mut self.state.tweens);
        let mut keep = Vec::new();
        for mut tween in tweens {
            let Some(mut layer) = self.state.layers.get(&tween.layer).cloned() else { continue };
            let mut alive = true;
            //Zero-length looping queues advance at most one cycle per frame.
            for _ in 0..=tween.tracks.len() {
                let track = tween.tracks[tween.index].clone();
                let begin = tween.start + (track.delay.max(0.) * 1000.) as u64;
                if now < begin {
                    break;
                }
                let slot = Self::group(&mut layer, track.group, true)?;
                let from = *tween.from.get_or_insert(*slot);
                let duration = (track.seconds.max(0.) * 1000.) as u64;
                let t = if duration == 0 { 1. } else { (now.saturating_sub(begin) as f32 / duration as f32).min(1.) };
                let k = ease(track.mode, t);
                for n in 0..4 {
                    let to = if track.mode == 1 { from[n] + track.target[n] } else { track.target[n] };
                    slot[n] = from[n] + (to - from[n]) * k;
                }
                self.dirty = true;
                if t < 1. {
                    break;
                }
                tween.start = begin + duration;
                tween.from = None;
                tween.index += 1;
                if tween.index == tween.tracks.len() {
                    if tween.loops == 0 {
                        alive = false;
                        break;
                    }
                    tween.loops -= i32::from(tween.loops > 0);
                    tween.index = 0;
                }
            }
            self.state.layers.insert(tween.layer.clone(), layer);
            if alive {
                keep.push(tween);
            }
        }
        self.state.tweens = keep;
        Ok(())
    }
    fn tweens_pending(&self, layer: &str) -> bool {
        self.state.tweens.iter().any(|tween| tween.layer == layer)
    }
    ///Screen quake offset (0x67): damped alternating displacement.
    fn shake(&mut self, now: u64) -> [f32; 2] {
        let Some(quake) = &self.state.quake else { return [0.; 2] };
        let elapsed = now.saturating_sub(quake.start) as f32 / 1000.;
        let period = quake.period.max(0.016);
        let phase = (elapsed / period * 4.).floor() as i64;
        let sign = if phase % 2 == 0 { 1. } else { -1. };
        let damp = (1. - (elapsed / period).fract() * 0.5).max(0.);
        self.dirty = true;
        [quake.amplitude[0] * sign * damp * if quake.kind == 4 { 0. } else { 1. }, quake.amplitude[1] * -sign * damp]
    }

    ///Text with the native inline codes stripped; returns (text, page).
    pub(crate) fn message(text: &str) -> Result<String> {
        let mut out = String::new();
        let mut rest = text;
        while let Some(at) = rest.find(['%', '\\']) {
            out.push_str(&rest[..at]);
            let tail = &rest[at..];
            if let Some(after) = tail.strip_prefix("\\n") {
                out.push('\n');
                rest = after;
            } else if let Some(after) = tail.strip_prefix("%K").or_else(|| tail.strip_prefix("%P")) {
                rest = after;
            } else if let Some(after) = tail.strip_prefix("%LC").or_else(|| tail.strip_prefix("%LR")).or_else(|| tail.strip_prefix("%LL")) {
                rest = after;
            } else if let Some(after) = tail.strip_prefix("%XS") {
                //%XS<n> ... %XE: styled span (size/effect parameter n); the
                //span text is kept, the style is not connected.
                rest = after.trim_start_matches(|c: char| c.is_ascii_digit());
            } else if let Some(after) = tail.strip_prefix("%XE") {
                rest = after;
            } else {
                bail!("AdvHD message code {:?} is not connected", &tail[..tail.len().min(4)]);
            }
        }
        out.push_str(rest);
        //CP932 user-defined F0xx gaiji used as a heart mark in these lines.
        Ok(out.replace('\u{e63e}', "\u{2665}"))
    }

    pub fn step(&mut self, host: &mut dyn HostServices, input: InputFrame, budget: u32) -> Result<VmYield> {
        if self.state.halted {
            return Ok(VmYield::Halted);
        }
        let now = host.now_ms();
        if !self.global_loaded {
            self.global_loaded = true;
            self.load_global(host)?;
            self.refresh_data_states(host);
        }
        poll_audio(host, self.client, &mut self.state)?;
        if !self.civil_started {
            self.civil_started = true;
            self.civil = host.begin_civil_time(self.client).ok();
        }
        if let Some(query) = &mut self.civil {
            match query.poll() {
                Ok(crysirror_core::service::CivilTimeState::Pending) => {}
                Ok(crysirror_core::service::CivilTimeState::Ready(time)) => {
                    self.state.utc_offset = time.utc_offset_seconds;
                    self.civil = None;
                }
                _ => self.civil = None,
            }
        }
        self.tick_tweens(now)?;
        let consumed = self.ui_frame(host, &input, now)?;
        if self.state.paused {
            self.flush(host)?;
            return Ok(VmYield::WaitUntil(now + 16));
        }
        let pressed = !consumed && input.buttons_pressed & (buttons::MOUSE_LEFT | buttons::ENTER | buttons::SPACE) != 0;
        let skip = self.system.loaded && self.skipping(&input);
        match self.state.wait.clone() {
            Wait::None => {}
            //Paused returned above, so the menu has closed.
            Wait::System => self.state.wait = Wait::None,
            Wait::Until { deadline, click } => {
                if now < deadline && !(click && (pressed || skip)) {
                    self.flush(host)?;
                    return Ok(VmYield::WaitUntil(if self.animating() { now + 16 } else { deadline.min(now + 16) }));
                }
                self.state.wait = Wait::None;
                self.state.game_state.insert(Self::state_id("LUA_STATE_TRANSITION"), false);
            }
            Wait::Click => {
                if !pressed && !skip {
                    self.flush(host)?;
                    return Ok(VmYield::WaitUntil(now + 16));
                }
                self.state.wait = Wait::None;
            }
            Wait::Text => {
                if !self.message_wait(host, &input, pressed, now)? {
                    self.flush(host)?;
                    return Ok(VmYield::WaitUntil(now + 16));
                }
                self.state.wait = Wait::None;
            }
            Wait::Select => {
                if !self.select_wait(host, now)? {
                    self.flush(host)?;
                    return Ok(VmYield::WaitUntil(now + 16));
                }
                if matches!(self.state.wait, Wait::Select) {
                    self.state.wait = Wait::None;
                }
            }
            Wait::WindowEffect => {
                if self.lua_call(host, "isMessageWindowEffect", vec![])?.first().is_some_and(|v| v.truthy()) {
                    self.flush(host)?;
                    return Ok(VmYield::WaitUntil(now + 16));
                }
                self.state.wait = Wait::None;
            }
            Wait::Movie { skippable } => {
                let info = host.media_info(self.client, MOVIE_MEDIA)?;
                let ended = matches!(info.phase, crysirror_core::service::MediaPhase::Ended | crysirror_core::service::MediaPhase::Stopped | crysirror_core::service::MediaPhase::Failed);
                if !ended && !(skippable && pressed) {
                    self.flush(host)?;
                    return Ok(VmYield::WaitUntil(now + 16));
                }
                self.stop_movie(host)?;
                self.state.wait = Wait::None;
            }
            Wait::Tweens(layer) => {
                if self.tweens_pending(&layer) && !skip {
                    self.flush(host)?;
                    return Ok(VmYield::WaitUntil(now + 16));
                }
                self.state.wait = Wait::None;
            }
        }
        if !matches!(self.state.wait, Wait::None) {
            self.flush(host)?;
            return Ok(VmYield::WaitUntil(now + 16));
        }
        for _ in 0..budget {
            let program = self.program(host, &self.state.scene.clone())?;
            let instruction = program.instruction_at(self.state.pc)?.clone();
            //Debug only (validation captures): WILLPLUS_HALT=SCENE:pc stops
            //before that instruction, keeping the composed screen.
            if let Ok(spec) = std::env::var("WILLPLUS_HALT") {
                if spec.split_once(':').is_some_and(|(scene, pc)| scene.eq_ignore_ascii_case(&self.state.scene) && u32::from_str_radix(pc.trim_start_matches("0x"), 16) == Ok(instruction.pc)) {
                    self.flush(host)?;
                    return Ok(VmYield::WaitUntil(now + 16));
                }
            }
            self.state.executed += 1;
            let result = self.execute(host, &program, &instruction, now)
                .with_context(|| format!("AdvHD {} pc={:#x} opcode={:#04x}", self.state.scene, instruction.pc, instruction.opcode))?;
            if let Some(yielded) = result {
                self.flush(host)?;
                return Ok(yielded);
            }
        }
        self.flush(host)?;
        Ok(VmYield::WaitUntil(now + 16))
    }

    pub(crate) fn stop_movie(&mut self, host: &mut dyn HostServices) -> Result<()> {
        if let Some(layer) = self.state.movie.take() {
            host.media_request(self.client, crysirror_core::service::MediaRequest::Stop { id: MOVIE_MEDIA })?;
            host.media_request(self.client, crysirror_core::service::MediaRequest::Remove { id: MOVIE_MEDIA })?;
            if matches!(self.state.layers.get(&layer).map(|l| &l.source), Some(Source::Movie { .. })) {
                self.state.layers.remove(&layer);
            }
            self.dirty = true;
        }
        Ok(())
    }
    fn animating(&self) -> bool {
        !self.state.tweens.is_empty() || self.state.quake.is_some() || self.state.layers.values().any(|l| matches!(l.source, Source::Streaks { .. }))
    }

    pub(crate) fn execute(&mut self, host: &mut dyn HostServices, program: &Program, i: &Instruction, now: u64) -> Result<Option<VmYield>> {
        let o = &i.operands;
        self.state.pc = i.next;
        match i.opcode {
            0x00 => {}
            0x01 => {
                let op = byte(&o[0])?;
                let index = short(&o[1])?;
                let value = float(&o[2])?;
                let current = if op & 0x80 != 0 {
                    self.var(index)
                } else {
                    f32::from(u8::from(u16::try_from(index).is_ok_and(|i| self.state.flags.contains(&i))))
                };
                let truth = match op & 0x7f {
                    0 => current >= value,
                    1 => current <= value,
                    2 => current == value,
                    3 => current != value,
                    other => bail!("AdvHD 0x01 comparison operator {other} is not connected"),
                };
                //True falls through; false goes to the else target, or the
                //end target when the block has no else (else target 0).
                if !truth {
                    let (otherwise, end) = (address(&o[3])?, address(&o[4])?);
                    self.state.pc = if otherwise != 0 { otherwise } else if end != 0 { end } else { i.next };
                }
            }
            0x02 | 0x06 => self.state.pc = address(&o[0])?,
            0x04 => {
                let scene = string(program, &o[0])?;
                self.state.stack.push((self.state.scene.clone(), i.next));
                ensure!(self.state.stack.len() <= 64, "AdvHD call depth");
                self.state.scene = scene.to_ascii_uppercase();
                self.state.pc = 0;
            }
            0x05 => {
                let (scene, pc) = self.state.stack.pop().context("AdvHD return without call")?;
                self.state.scene = scene;
                self.state.pc = pc;
            }
            0x07 => {
                self.state.scene = string(program, &o[0])?.to_ascii_uppercase();
                self.state.pc = 0;
            }
            0xff => {
                if let Some((scene, pc)) = self.state.stack.pop() {
                    self.state.scene = scene;
                    self.state.pc = pc;
                } else {
                    self.state.halted = true;
                    return Ok(Some(VmYield::Halted));
                }
            }
            0x09 => {
                let op = byte(&o[0])?;
                let index = short(&o[1])?;
                let raw = float(&o[2])?;
                let operand = if op & 0x80 != 0 { self.var(raw as i16) } else { raw };
                let current = self.var(index);
                let value = match op & 0x7f {
                    0 => operand,
                    1 => current - operand,
                    2 => current + operand,
                    3 => current * operand,
                    other => bail!("AdvHD 0x09 operator {other} is not connected"),
                };
                self.set_var(index, value)?;
            }
            0x0b => {
                let index = u16::try_from(short(&o[0])?).context("AdvHD flag index")?;
                if byte(&o[1])? != 0 {
                    self.state.flags.insert(index);
                } else {
                    self.state.flags.remove(&index);
                }
            }
            0x0d => {
                let (start, count, value) = (short(&o[0])?, short(&o[1])?, float(&o[2])?);
                for n in 0..count.max(0) {
                    self.set_var(start + n, value)?;
                }
            }
            0x0e => {
                let (start, count, value) = (short(&o[0])?, short(&o[1])?, byte(&o[2])?);
                for n in 0..count.max(0) {
                    let index = u16::try_from(start + n).context("AdvHD flag index")?;
                    if value != 0 {
                        self.state.flags.insert(index);
                    } else {
                        self.state.flags.remove(&index);
                    }
                }
            }
            0x11 => {
                let name = string(program, &o[0])?;
                let seconds = float(&o[2])?;
                self.state.timers.insert(name, now + (seconds.max(0.) * 1000.) as u64);
            }
            0x12 => {
                let name = string(program, &o[0])?;
                let click = byte(&o[1])? != 0;
                let deadline = *self.state.timers.get(&name).with_context(|| format!("AdvHD timer {name:?} not started"))?;
                if now < deadline {
                    self.state.wait = Wait::Until { deadline, click };
                    return Ok(Some(VmYield::WaitUntil(deadline)));
                }
            }
            0x14 => {
                let id = match o[0] {
                    Operand::Integer(v) => v,
                    _ => bail!("AdvHD 0x14 message id"),
                };
                let text = string(program, &o[2])?;
                return self.message_show(host, i.pc, id, &text, now);
            }
            0x15 => self.message_speaker(host, &string(program, &o[0])?)?,
            0x16 => return self.message_close(host, now),
            0x0f => {
                ensure!(!i.choices.is_empty(), "AdvHD 0x0F without choices");
                return self.select_start(host, program, i, now);
            }
            0x1e => {
                let id = string(program, &o[0])?;
                let file = string(program, &o[1])?;
                let channel = self.channel(&id);
                self.play(host, channel, &id, &file, short(&o[4])? == -1, true, short(&o[5])?)?;
            }
            0x1f | 0x29 => {
                let id = string(program, &o[0])?;
                let fade_ms = (float(&o[1])?.max(0.) * 1000.) as f32;
                let channels: Vec<u32> = if id == "*" { self.state.channels.iter().copied().collect() } else { vec![self.channel(&id)] };
                if id == "*" {
                    self.state.music.clear();
                } else {
                    self.state.music.remove(&id);
                }
                for channel in channels {
                    host.submit_audio(self.client, AudioRequest::Stop { channel, fade_ms })?;
                }
            }
            0x28 => {
                let id = string(program, &o[0])?;
                let file = string(program, &o[1])?;
                let channel = self.channel(&id);
                self.play(host, channel, &id, &file, false, false, short(&o[5])?)?;
                if id.starts_with("char") {
                    self.state.voice = Some((id.clone(), file.clone()));
                }
            }
            0x2e => {}
            //0x20 BGM / 0x2A sound volume change: (slot, seconds, tenths).
            0x20 | 0x2a => {
                let id = string(program, &o[0])?;
                let fade_ms = float(&o[1])?.max(0.) * 1000.;
                let gain = f32::from(short(&o[2])?.clamp(0, 10)) / 10.;
                let channel = self.channel(&id);
                if self.state.channels.contains(&channel) {
                    host.submit_audio(self.client, AudioRequest::SetGain { channel, gain, fade_ms })?;
                }
            }
            //Inferred markers without an observable effect in these scripts:
            //0xE6 (pair of increasing ids after a line), 0x1D h-1 (title
            //reset), 0x43 (layer named before a transition).
            0xe6 | 0x1d | 0x43 => {}
            //0xFB b1/b0 brackets the brand logos in start.ws2 and 0xFD
            //precedes new-game/replay fades; neither has a visible effect here.
            0xfb | 0xfd => {}
            //0xFC h: system voice by SYSVOICE_* id (h1 = SYSVOICE_STARTUP after
            //the brand logo), played by the Lua layer.
            0xfc => {
                let id = short(&o[0])?;
                self.lua_call(host, "playSystemVoice", vec![crysirror_lua::Value::Int(i64::from(id))])?;
            }
            //0x08: title EXIT.
            0x08 => {
                self.state.halted = true;
                self.flush(host)?;
                return Ok(Some(VmYield::Halted));
            }
            //0x17 / 0x71 close the title/menu reset sequences (after 0x68 b1):
            //clear running tweens and pending queues / screen effect state.
            0x17 => {
                self.state.tweens.clear();
                self.state.queued.clear();
                self.state.timers.clear();
            }
            0x71 => {
                self.state.quake = None;
                self.state.rule = None;
                self.dirty = true;
            }
            0x35 => {
                let layer = string(program, &o[0])?;
                let file = string(program, &o[1])?;
                self.stop_movie(host)?;
                //Movies sit beside AdvHD.exe (OP.DAT/ED.DAT), not in an ARC.
                let asset = match self.resolve(host, &file) {
                    Ok(path) => path,
                    Err(_) => VfsPath::new(Mount::Asset, &file)?.relative().to_owned(),
                };
                let surface = host.media_request(self.client, crysirror_core::service::MediaRequest::Play { id: MOVIE_MEDIA, asset, looping: false, audio: true, gain: 1. })?;
                self.put_layer(&layer, Source::Movie { surface: surface.relative().to_owned() });
                self.state.movie = Some(layer);
            }
            0x3a => {
                let mode = byte(&o[2])?;
                ensure!(matches!(mode, 1 | 2), "AdvHD 0x3A movie wait mode {mode}");
                if self.state.movie.is_some() {
                    //1: first viewing (flag 1016 unset) cannot be skipped.
                    self.state.wait = Wait::Movie { skippable: mode == 2 };
                    return Ok(Some(VmYield::WaitUntil(now + 16)));
                }
            }
            0x1a => {
                let name = string(program, &o[0])?;
                self.lua_call(host, &name, vec![])?;
                self.dirty = true;
                if self.state.paused {
                    self.state.wait = Wait::System;
                    return Ok(Some(VmYield::WaitUntil(now + 16)));
                }
            }
            //0x1B b: title-menu system screens (MAINMENU: b1 after CONFIG,
            //b5 after LOAD) -> openConfig page (0 system, 4 save, 5 load),
            //the pages the window buttons open.
            0x1b => {
                let page = match byte(&o[0])? {
                    1 => 0,
                    n @ (4 | 5) => i64::from(n),
                    other => bail!("AdvHD 0x1B system screen {other} is not connected"),
                };
                self.lua_call(host, "openConfig", vec![crysirror_lua::Value::Int(page), crysirror_lua::Value::Bool(false)])?;
                self.dirty = true;
                if self.state.paused {
                    self.state.wait = Wait::System;
                    return Ok(Some(VmYield::WaitUntil(now + 16)));
                }
            }
            0x1c => {
                let name = string(program, &o[0])?;
                let argument = string(program, &o[1])?;
                let slot = short(&o[2])?;
                self.lua_call(host, &name, vec![crysirror_lua::Value::str(&argument), crysirror_lua::Value::Int(i64::from(slot))])?;
                //A menu the call opened (openGalleryTitle, openGallery) holds
                //the script until it unpauses, as 0x1A does.
                self.dirty = true;
                if self.state.paused {
                    self.state.wait = Wait::System;
                    return Ok(Some(VmYield::WaitUntil(now + 16)));
                }
            }
            0x33 => {
                let layer = string(program, &o[0])?;
                let file = string(program, &o[1])?;
                let path = self.resolve(host, &file)?;
                self.put_layer(&layer, Source::Image(path));
            }
            0x34 => {
                let layer = string(program, &o[0])?;
                let file = string(program, &o[1])?;
                self.pna(host, &file)?;
                self.put_layer(&layer, Source::Pna { file, parts: Vec::new() });
            }
            0x37 => {
                let pattern = string(program, &o[0])?;
                if pattern == "*" {
                    self.state.layers.clear();
                    self.state.tweens.clear();
                } else {
                    self.state.layers.remove(&pattern);
                    self.state.tweens.retain(|tween| tween.layer != pattern);
                }
                self.dirty = true;
            }
            0x39 => {
                let name = string(program, &o[0])?;
                let ids = match o.get(3) {
                    Some(Operand::Array(items)) => items.iter().map(|item| short(item).map(i32::from)).collect::<Result<Vec<_>>>()?,
                    _ => bail!("AdvHD 0x39 part list"),
                };
                let layer = self.layer_mut(&name)?;
                let Source::Pna { parts, .. } = &mut layer.source else { bail!("AdvHD 0x39 on a non-PNA layer {name:?}") };
                *parts = ids;
                self.dirty = true;
            }
            0x3f => {
                let Some(Operand::Array(items)) = o.first() else { bail!("AdvHD 0x3F order list") };
                self.state.order = items.iter().map(|item| string(program, item)).collect::<Result<_>>()?;
                self.dirty = true;
            }
            0x45 => {
                let name = self.layer_name(&string(program, &o[0])?)?;
                let group = short(&o[1])?;
                //Reading an absent layer yields its defaults (scenes started
                //mid-route read layers an earlier scene created).
                let mut layer = self.state.layers.get(&name).cloned().unwrap_or(Layer { id: 0, source: Source::Image(String::new()), position: [0.; 4], offset: [0.; 4], mask: None, attributes: BTreeSet::new(), rotation: [0.; 4], scale: [1., 1., 1., 0.], color: [1.; 4], extra: BTreeMap::new() });
                //Reads use tween numbering for position (the offset) and set
                //numbering otherwise (h6 colour in the scripts).
                let values = if group == 0 { layer.offset } else { *Self::group(&mut layer, group, false)? };
                for n in 0..4 {
                    let index = float(&o[2 + n])? as i16;
                    self.set_var(index, values[n])?;
                }
            }
            0x46 => {
                let name = string(program, &o[0])?;
                let group = short(&o[1])?;
                let flags = byte(&o[2])?;
                let values = self.operands4(flags, [float(&o[3])?, float(&o[4])?, float(&o[5])?, float(&o[6])?]);
                let name = self.layer_name(&name)?;
                let Some(layer) = self.state.layers.get_mut(&name) else { return Ok(None) };
                if matches!(group, 10 | 11) {
                    //Groups 10/11 have no connected visual meaning; retained.
                    layer.extra.insert(group, values);
                } else {
                    *Self::group(layer, group, false)? = values;
                }
                self.dirty = true;
            }
            0x47 => {
                let name = self.layer_name(&string(program, &o[0])?)?;
                let queue = string(program, &o[1])?;
                let group = short(&o[2])?;
                let mode = byte(&o[3])?;
                ensure!(mode <= 3, "AdvHD 0x47 track mode {mode}");
                let flags = byte(&o[4])?;
                let target = self.operands4(flags, [float(&o[5])?, float(&o[6])?, float(&o[7])?, float(&o[8])?]);
                let seconds = if flags & 0x08 != 0 { self.var(float(&o[9])? as i16) } else { float(&o[9])? };
                let delay = if flags & 0x04 != 0 { self.var(float(&o[11])? as i16) } else { float(&o[11])? };
                self.state.queued.entry((name, queue)).or_default().push(Track { group, mode, target, seconds, delay });
            }
            0x48 => {
                let name = self.layer_name(&string(program, &o[0])?)?;
                let queue = string(program, &o[1])?;
                let loops = i32::from(short(&o[2])?);
                let tracks = self.state.queued.remove(&(name.clone(), queue.clone())).unwrap_or_default();
                if !self.state.layers.contains_key(&name) {
                    //A queue on an absent layer animates nothing.
                    return Ok(None);
                }
                self.state.tweens.retain(|tween| !(tween.layer == name && tween.queue == queue));
                if !tracks.is_empty() {
                    self.state.tweens.push(Tween { layer: name, queue, tracks, index: 0, from: None, start: now, loops: if loops < 0 { -1 } else { (loops - 1).max(0) } });
                    self.tick_tweens(now)?;
                }
            }
            0x57 => {
                let name = self.layer_name(&string(program, &o[0])?)?;
                //h0 follows nearly every 0x48 whose tween overlaps later text
                //(90-second pans), so only a non-zero mode blocks.
                if short(&o[1])? != 0 && self.tweens_pending(&name) {
                    self.state.wait = Wait::Tweens(name);
                    return Ok(Some(VmYield::WaitUntil(now + 16)));
                }
            }
            0x58 => {
                let name = self.layer_name(&string(program, &o[0])?)?;
                let queue = string(program, &o[1])?;
                self.state.tweens.retain(|tween| !(tween.layer == name && tween.queue == queue));
            }
            //0x56: named weather effect whose output is layer o[21] (the
            //scripts tween that layer afterwards); o[17] particle count,
            //o[15] speed and o[18] size. Drawn as falling streaks; 0x5C
            //stops it by effect name. Inferred from the two uses only.
            0x56 => {
                let effect = string(program, &o[0])?;
                let layer = string(program, &o[21])?;
                let count = float(&o[17])?.clamp(0., 4096.) as u32;
                let speed = float(&o[15])?.max(0.05) * 1200.;
                let length = float(&o[18])?.max(1.) * 8.;
                let seed = effect.bytes().fold(0x811c_9dc5u32, |h, b| (h ^ u32::from(b)).wrapping_mul(0x0100_0193));
                self.put_layer(&layer, Source::Streaks { effect, seed, count, speed, length, start: now });
            }
            0x5c => {
                let effect = string(program, &o[0])?;
                self.state.layers.retain(|_, l| !matches!(&l.source, Source::Streaks { effect: e, .. } if *e == effect));
                self.dirty = true;
            }
            0x6e => {
                let key = string(program, &o[0])?;
                let layer = string(program, &o[1])?;
                if layer.is_empty() {
                    self.state.keys.remove(&key);
                } else {
                    self.state.keys.insert(key, layer);
                }
            }
            0x6f => {
                self.state.keys.remove(&string(program, &o[0])?);
            }
            0x40 => {
                let name = string(program, &o[0])?;
                let mask = string(program, &o[1])?;
                let path = if mask.is_empty() { None } else { Some(self.resolve(host, &mask)?) };
                self.layer_mut(&name)?.mask = path;
                self.dirty = true;
            }
            0x42 => {
                let name = string(program, &o[0])?;
                let attribute = short(&o[1])?;
                self.layer_mut(&name)?.attributes.insert(attribute);
            }
            0x32 => {
                //Face chip of the next line: the scripts' faceRead loads
                //`<name>.png` into system/MsgFace (".png" alone removes it).
                let face = string(program, &o[0])?;
                self.lua_call(host, "faceRead", vec![crysirror_lua::Value::str(&format!("{face}.png"))])?;
                self.state.face = face;
                self.dirty = true;
            }
            0x67 => {
                let kind = byte(&o[0])?;
                ensure!(matches!(kind, 3 | 4), "AdvHD 0x67 quake kind {kind}");
                self.state.quake = Some(Quake { kind, amplitude: [float(&o[4])?, float(&o[5])?], period: float(&o[3])?, start: now });
            }
            0x68 => {
                self.state.quake = None;
                self.dirty = true;
            }
            0x64 => {
                //Prepare: the presented frame is the transition source.
                self.flush(host)?;
            }
            0x66 => self.state.rule = Some(string(program, &o[0])?),
            0x65 => {
                let seconds = float(&o[2])?;
                let wait = byte(&o[4])? == 1;
                let duration = (seconds.max(0.) * 1000.) as u64;
                self.dirty = true;
                self.flush(host)?;
                if duration > 0 {
                    let effect = match self.state.rule.take() {
                        Some(rule) => json!({"kind":"mask","asset":self.resolve(host, &rule)?,"fade":0}),
                        None => json!({"kind":"crossfade"}),
                    };
                    host.submit_engine(self.client, 1, &serde_json::to_vec(&json!({"layers":[{"op":"transition","transition":{"effect":effect,"duration_ms":duration}}]}))?)?;
                    if wait {
                        self.state.game_state.insert(Self::state_id("LUA_STATE_TRANSITION"), true);
                        self.state.wait = Wait::Until { deadline: now + duration, click: true };
                        return Ok(Some(VmYield::WaitUntil(now + duration)));
                    }
                }
            }
            other => bail!("AdvHD WS2 opcode {other:#04x} is not connected"),
        }
        Ok(None)
    }
}

///`crysirror_lua::Host` for the AdvHD bindings (`system` module names).
struct Bridge<'a> {
    state: &'a mut State,
    registry: &'a mut crate::system::Registry,
    services: &'a mut dyn HostServices,
    client: ClientId,
    res: &'a mut Resources,
    now: u64,
}

///PNA/image geometry for `layers` through the scoped resources.
pub(crate) struct GeometryOf<'a> {
    pub(crate) res: &'a mut Resources,
    pub(crate) services: &'a mut dyn HostServices,
}
impl crate::layers::Geometry for GeometryOf<'_> {
    fn records(&mut self, path: &str) -> Result<Vec<Option<([i32; 2], [u32; 2])>>> {
        let pna = self.res.pna(self.services, path)?;
        let count = pna.0.parts.iter().map(|p| p.index + 1).max().unwrap_or(0);
        let mut out = vec![None; count];
        for part in &pna.0.parts {
            out[part.index] = Some((part.offset, part.size));
        }
        Ok(out)
    }
    fn image_size(&mut self, path: &str) -> Result<[u32; 2]> {
        if path.to_ascii_lowercase().ends_with(".pna") {
            return Ok(self.res.pna(self.services, path)?.0.canvas);
        }
        let resolved = self.res.resolve(self.services, path)?;
        Ok(self.services.image_size(self.res.client, &resolved)?)
    }
}

impl Bridge<'_> {
    fn save(&mut self, mode: i64, slot: i64) -> Option<Arc<crate::save::SaveFile>> {
        crate::save::cached(&mut self.state.saves, self.services, self.client, mode, slot)
    }
    fn cfunc(&mut self, name: &str, args: &[crysirror_lua::Value]) -> Result<Vec<crysirror_lua::Value>> {
        use crate::system::{int, num, text};
        use crysirror_lua::Value as V;
        Ok(match name {
            "LegacyGame__lua_GetGameState" => vec![V::Bool(self.state.game_state.get(&int(args, 0)?).copied().unwrap_or(false))],
            //One argument toggles: hide/unhide, skip on/off and the
            //DISABLEDATA backup/restore pairs all call it that way.
            "LegacyGame__lua_SetGameState" => {
                let id = int(args, 0)?;
                let on = match args.get(1) {
                    Some(v) => v.truthy() && !matches!(v, V::Int(0)),
                    None => !self.state.game_state.get(&id).copied().unwrap_or(false),
                };
                self.state.game_state.insert(id, on);
                vec![]
            }
            "LegacyGame__lua_SelectItem" => {
                let item = usize::try_from(int(args, 0)?).context("AdvHD select item")?;
                let select = self.state.select.as_mut().context("AdvHD SelectItem without a choice list")?;
                anyhow::ensure!(item < select.ids.len(), "AdvHD SelectItem({item}) out of range");
                select.chosen = Some(item);
                vec![]
            }
            "LegacyGame__lua_GetFlag" => vec![V::Bool(u16::try_from(int(args, 0)?).is_ok_and(|i| self.state.flags.contains(&i)))],
            "LegacyGame__lua_SetFlag" => {
                let index = u16::try_from(int(args, 0)?).context("AdvHD flag index")?;
                let on = match args.get(1) {
                    Some(V::Bool(b)) => *b,
                    Some(v) => v.as_f64().is_some_and(|f| f != 0.),
                    None => false,
                };
                if on {
                    self.state.flags.insert(index);
                } else {
                    self.state.flags.remove(&index);
                }
                vec![]
            }
            "LegacyGame__lua_GetVariable" => {
                let index = u16::try_from(int(args, 0)?).context("AdvHD variable index")?;
                let value = self.state.vars.get(&index).copied().unwrap_or(0.);
                vec![if value.fract() == 0. { V::Int(value as i64) } else { V::Num(f64::from(value)) }]
            }
            "LegacyGame__lua_SetVariable" => {
                let index = u16::try_from(int(args, 0)?).context("AdvHD variable index")?;
                self.state.vars.insert(index, num(args, 1)? as f32);
                vec![]
            }
            "LegacyGame__lua_GetString" => vec![V::str(self.state.strings.get(&int(args, 0)?).map_or("", String::as_str))],
            "LegacyGame__lua_SetString" => {
                self.state.strings.insert(int(args, 0)?, text(args, 1)?);
                vec![]
            }
            "LegacyGame__lua_NeedProcess" => {
                let process = int(args, 0)?;
                if std::env::var_os("WILLPLUS_TEXT_TRACE").is_some() {
                    eprintln!("WILLPLUS process {:?} {:?}", crate::system::constants().into_iter().find(|(k, v)| *v == process && k.starts_with("LUA_PROCESS_")).map(|x| x.0), args.get(1));
                }
                let named = |n: &str| crate::system::constants().into_iter().find(|(k, _)| *k == n).map(|(_, v)| v);
                if Some(process) == named("LUA_PROCESS_GAME_PAUSED") {
                    self.state.paused = true;
                } else if Some(process) == named("LUA_PROCESS_GAME_UNPAUSED") {
                    self.state.paused = false;
                } else if Some(process) == named("LUA_PROCESS_RTSNAP") {
                    //Snapshot of the composed screen for menu backgrounds;
                    //menus compose over the live scene instead.
                } else {
                    //Saves, loads, title and the rest run after this Lua call.
                    self.state.processes.push((process, int(args, 1)?));
                }
                vec![]
            }
            //Re-deliver the pointer next frame (hover refresh after a menu).
            "D3DApp__lua_PostMouseMove" => {
                self.state.cursor = [f64::NAN; 2];
                vec![]
            }
            //Volume settings changed for a category: playing voices take
            //their new gains after this call.
            "LegacyGame__lua_NeedVolumeUpdate" => {
                self.state.volume_update = true;
                vec![]
            }
            //Load hook names (cast-off/tips features this title disables).
            "LegacyGame__lua_SetLoadProc" => vec![],
            //Config display-mode preview (window/fullscreen are host policy).
            "LegacyGame__lua_NeedPreview" => vec![],
            //Opening a web page (survey / site links) is not done from game
            //data; the request is reported.
            "LegacyGame__lua_NeedShell" => {
                eprintln!("CrySirror: AdvHD shell request not opened: {}", text(args, 0)?);
                vec![]
            }
            "LegacyGame__lua_SetSaveComment" => {
                self.state.save_comment = text(args, 0)?;
                vec![]
            }
            "CSaveDataManager__lua_IsResume" => vec![V::Bool(self.save(crate::save::MODE_RESUME, 0).is_some())],
            "CSaveDataManager__lua_IsData" | "CSaveDataManager__lua_GetEnabled" => vec![V::Bool(self.save(int(args, 0)?, int(args, 1)?).is_some())],
            "CSaveDataManager__lua_GetSaveDataDate" => vec![V::str(&self.save(int(args, 0)?, int(args, 1)?).map(|s| s.date.clone()).unwrap_or_default())],
            "CSaveDataManager__lua_GetSaveDataText" => vec![V::str(&self.save(int(args, 0)?, int(args, 1)?).map(|s| s.text.clone()).unwrap_or_default())],
            "CSaveDataManager__lua_GetThumbnailPath" => {
                let (mode, slot) = (int(args, 0)?, int(args, 1)?);
                vec![V::str(&if self.save(mode, slot).is_some() { format!("{}{mode}/{slot}", crate::save::THUMB_PREFIX) } else { String::new() })]
            }
            //(mode, ..) -> (mode of the newest data, its slot); slot 0 when empty.
            "CSaveDataManager__lua_FindNewData" => {
                let mode = int(args, 0)?;
                let top = if mode == crate::save::MODE_NORMAL { 100 } else { 5 };
                let newest = (1..=top).filter_map(|n| self.save(mode, n).map(|s| (s.time, n))).max_by_key(|(t, n)| (*t, -*n)).map_or(0, |(_, n)| n);
                vec![V::Int(if newest > 0 { mode } else { -1 }), V::Int(newest)]
            }
            "CSaveDataManager__lua_GetSavePath" => vec![V::str("save")],
            "MyConv__lua_utf8tosjis" => match args.first() {
                Some(V::Str(bytes)) => match std::str::from_utf8(bytes) {
                    Ok(t) => vec![V::bytes(&encoding_rs::SHIFT_JIS.encode(t).0)],
                    Err(_) => vec![V::Str(bytes.clone())],
                },
                other => vec![other.cloned().unwrap_or_default()],
            },
            "MyConv__lua_sjistoutf8" => match args.first() {
                Some(V::Str(bytes)) => vec![V::str(&encoding_rs::SHIFT_JIS.decode(bytes).0)],
                other => vec![other.cloned().unwrap_or_default()],
            },
            other => bail!("AdvHD cfunc.{other} is not connected"),
        })
    }

    fn method(&mut self, lua: &mut crysirror_lua::Lua, object: crate::system::Object, id: u32, name: &str, args: &[crysirror_lua::Value]) -> Result<Vec<crysirror_lua::Value>> {
        use crate::system::{int, text, Object};
        use crysirror_lua::Value as V;
        let _ = lua;
        let class = match &object {
            Object::Class(class) | Object::Instance { class, .. } => *class,
            Object::File { .. } => "file",
        };
        let this = match args.first() {
            Some(V::UserData(u)) => *u,
            _ => bail!("AdvHD {class}:{name} called without its object"),
        };
        let a = &args[1..];
        let _ = id;
        if let Object::File { .. } = object {
            return self.file_method(this, name, a);
        }
        Ok(match (class, name) {
            ("lua_Layers", _) => {
                let mut geometry = GeometryOf { res: &mut *self.res, services: &mut *self.services };
                match name {
                    //The manager is a singleton: `create` returns it.
                    "create" => vec![V::UserData(this)],
                    _ => match crate::log::call(&mut self.state.ui, &self.state.log, &self.state.fonts, name, args, &mut self.state.log_redraw)? {
                        Some(out) => out,
                        None => self.state.ui.call(name, args, self.now, self.state.cursor, &mut geometry)?,
                    },
                }
            }
            (_, "new") => {
                let instance = self.registry.add(Object::Instance { class, id: this });
                if class == "lua_MeasureTime" {
                    self.state.measures.insert(instance, self.now);
                }
                vec![V::UserData(instance)]
            }
            (_, "delete") => {
                if matches!(object, Object::Instance { .. }) {
                    self.registry.objects.remove(&this);
                    self.state.measures.remove(&this);
                }
                vec![]
            }
            ("CLegacyFontInfo", "create") => vec![V::UserData(this)],
            ("CLegacyFontInfo", "add") => {
                let style = FontStyle { font: text(a, 2)?, size: crate::system::num(a, 3)?, frame: crate::system::num(a, 4)?, color: int(a, 6)? as u32, edge: int(a, 7)? as u32 };
                self.state.fonts.insert((text(a, 0)?, text(a, 1)?), style);
                vec![V::Bool(true)]
            }
            ("CLegacyFontInfo", "remove") => {
                self.state.fonts.remove(&(text(a, 0)?, text(a, 1)?));
                vec![]
            }
            ("CLegacyFontInfo", "changeColor") => {
                if let Some(style) = self.state.fonts.get_mut(&(text(a, 0)?, text(a, 1)?)) {
                    style.color = int(a, 2)? as u32;
                    style.edge = int(a, 3)? as u32;
                }
                vec![]
            }
            ("lua_Sounds", "BgmPlay" | "SndPlay") => {
                //(slot, archive, file, volume 0..1000 from getSoundVolume, ...).
                let (slot, file) = (text(a, 0)?, text(a, 2)?);
                let gain = (crate::system::num(a, 3)? / 1000.).clamp(0., 1.) as f32;
                let music = name == "BgmPlay";
                let channel = sound_channel(&slot);
                let path = self.res.resolve(self.services, &file)?;
                start_audio(self.services, self.client, self.state, channel, &path, music, gain, if music { PcmBus::Music } else { PcmBus::Ui })?;
                self.state.lua_sounds.insert(slot);
                vec![]
            }
            ("lua_Sounds", "BgmStop" | "SndStop") => {
                let slot = text(a, 0)?;
                let fade_ms = crate::system::num(a, 1)? as f32;
                self.services.submit_audio(self.client, AudioRequest::Stop { channel: sound_channel(&slot), fade_ms })?;
                self.state.lua_sounds.remove(&slot);
                vec![]
            }
            ("lua_Sounds", "IsPlaying") => vec![V::Bool(self.state.lua_sounds.contains(&text(a, 0)?))],
            ("lua_AutoCursor", "SetCursor") => vec![],
            //Pointer warp toward a button (mouseAutoMove setting): the host
            //does not move the OS pointer, so no auto-move is ever running.
            ("lua_AutoCursor", "AutoCursor") => vec![],
            ("lua_AutoCursor", "IsMove") => vec![V::Bool(false)],
            ("lua_AutoCursor", "GetCursorPos") => vec![V::Num(self.state.cursor[0]), V::Num(self.state.cursor[1])],
            //Scale for effect delays measured since construction (1 = as authored).
            ("lua_MeasureTime", "diff") => vec![V::Int(1)],
            //Save-list edit object: Insert lists a normal slot, Toggle marks
            //it for deletion, DeleteData removes the marked slots.
            ("lua_DataSort", "ResetMark") => {
                self.state.data_marks.clear();
                vec![]
            }
            ("lua_DataSort", "Insert") => vec![],
            ("lua_DataSort", "Toggle") => {
                let slot = int(a, 0)?;
                if !self.state.data_marks.remove(&slot) {
                    self.state.data_marks.insert(slot);
                }
                vec![]
            }
            ("lua_DataSort", "GetProtect") => vec![V::Bool(false)],
            ("lua_DataSort", "size") => {
                let count = (1..=100).filter(|n| self.save(crate::save::MODE_NORMAL, *n).is_some()).count();
                vec![V::Int(count as i64)]
            }
            ("lua_DataSort", "DeleteData") => {
                for slot in std::mem::take(&mut self.state.data_marks) {
                    if let Some(name) = crate::save::file_name(crate::save::MODE_NORMAL, slot) {
                        let _ = self.services.storage(self.client)?.remove(&VfsPath::new(Mount::Save, &name)?);
                        self.state.saves.insert(name, None);
                    }
                }
                vec![]
            }
            //Master volume 0..1000 and mute (config SOUND page); applied to
            //every voice's gain at start.
            ("lua_Mixer", "getMasterVolume") => vec![V::Int(self.state.master_volume.unwrap_or(1000))],
            ("lua_Mixer", "setMasterVolume") => {
                self.state.master_volume = Some(int(a, 0)?.clamp(0, 1000));
                self.state.registry_dwords.insert("MasterVolume".into(), int(a, 0)?.clamp(0, 1000));
                self.state.volume_update = true;
                vec![]
            }
            ("lua_Mixer", "getMasterMute") => vec![V::Bool(self.state.master_mute)],
            ("lua_Mixer", "setMasterMute") => {
                self.state.master_mute = a.first().is_some_and(V::truthy);
                self.state.volume_update = true;
                vec![]
            }
            ("lua_Registry", "GetDWORD") => vec![V::Int(self.state.registry_dwords.get(&text(a, 0)?).copied().unwrap_or(0))],
            ("lua_Registry", "IsKeyDWORD") => vec![V::Bool(self.state.registry_dwords.contains_key(&text(a, 0)?))],
            ("lua_Registry", "SetDWORD") => {
                self.state.registry_dwords.insert(text(a, 0)?, int(a, 1)?);
                vec![]
            }
            (class, name) => bail!("AdvHD {class}:{name} is not connected"),
        })
    }
}

///`save\\name` paths address the client's Save mount; others read the
///game directory (Asset) and never write it.
fn lua_path(path: &str) -> Result<(bool, VfsPath)> {
    let path = path.replace('\\', "/");
    let (save, rest) = match path.strip_prefix("save/") {
        Some(rest) => (true, rest.to_owned()),
        None => (false, path.trim_start_matches("./").to_owned()),
    };
    Ok((save, VfsPath::new(if save { Mount::Save } else { Mount::Asset }, &rest)?))
}

///Civil date from Unix seconds (UTC).
pub(crate) fn civil(seconds: i64) -> [i64; 6] {
    let days = seconds.div_euclid(86_400);
    let rem = seconds.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    [year, month, day, rem / 3600, rem / 60 % 60, rem % 60]
}

impl Bridge<'_> {
    fn file_method(&mut self, this: u32, name: &str, a: &[crysirror_lua::Value]) -> Result<Vec<crysirror_lua::Value>> {
        use crate::system::{text, Object};
        use crysirror_lua::Value as V;
        let Some(Object::File { path, save, data, at, write }) = self.registry.objects.get_mut(&this) else { bail!("AdvHD file handle is closed") };
        Ok(match name {
            "read" => {
                let format = if a.is_empty() { "*l".to_owned() } else { text(a, 0)? };
                let rest = &data[(*at).min(data.len())..];
                match format.trim_start_matches('*').chars().next() {
                    Some('n') => {
                        let skip = rest.iter().take_while(|b| b.is_ascii_whitespace()).count();
                        let len = rest[skip..].iter().take_while(|b| b.is_ascii_digit() || matches!(b, b'-' | b'+' | b'.' | b'e' | b'E')).count();
                        *at += skip + len;
                        vec![std::str::from_utf8(&rest[skip..skip + len]).ok().and_then(crysirror_lua::value::parse_number).unwrap_or_default()]
                    }
                    Some('l') => {
                        if rest.is_empty() {
                            vec![V::Nil]
                        } else {
                            let len = rest.iter().position(|b| *b == b'\n').unwrap_or(rest.len());
                            let line = rest[..len].strip_suffix(b"\r").unwrap_or(&rest[..len]).to_vec();
                            *at += (len + 1).min(rest.len());
                            vec![V::bytes(&line)]
                        }
                    }
                    Some('a') => {
                        *at = data.len();
                        vec![V::bytes(rest)]
                    }
                    _ => bail!("AdvHD file:read({format:?}) is not connected"),
                }
            }
            "write" => {
                ensure!(*write, "AdvHD file opened for reading was written");
                for value in a {
                    data.extend(value.display());
                }
                vec![V::UserData(this)]
            }
            "close" => {
                if *write && *save {
                    let path = VfsPath::new(Mount::Save, path)?;
                    let bytes = data.clone();
                    self.services.write_atomic(self.client, &path, &bytes)?;
                }
                self.registry.objects.remove(&this);
                vec![V::Bool(true)]
            }
            other => bail!("AdvHD file:{other} is not connected"),
        })
    }

    fn library(&mut self, (lib, name): (&str, &str), args: &[crysirror_lua::Value]) -> Result<Vec<crysirror_lua::Value>> {
        use crate::system::{text, Object};
        use crysirror_lua::Value as V;
        let unix = || std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
        Ok(match (lib, name) {
            ("os", "time") => vec![V::Int(unix())],
            ("os", "clock") => vec![V::Num(self.now as f64 / 1000.)],
            ("os", "date") => {
                let format = if args.is_empty() { "%c".to_owned() } else { text(args, 0)? };
                let t = args.get(1).and_then(V::as_i64).unwrap_or_else(unix) + i64::from(self.state.utc_offset);
                let [y, mo, d, h, mi, se] = civil(t);
                let out = format.replace("%Y", &format!("{y:04}")).replace("%m", &format!("{mo:02}")).replace("%d", &format!("{d:02}"))
                    .replace("%H", &format!("{h:02}")).replace("%M", &format!("{mi:02}")).replace("%S", &format!("{se:02}"));
                ensure!(!out.contains('%'), "AdvHD os.date format {format:?} is not connected");
                vec![V::str(&out)]
            }
            ("os", "remove") => {
                let (save, path) = lua_path(&text(args, 0)?)?;
                ensure!(save, "AdvHD os.remove outside the save directory");
                match self.services.storage(self.client)?.remove(&path) {
                    Ok(()) => vec![V::Bool(true)],
                    Err(e) => vec![V::Nil, V::str(&e.to_string())],
                }
            }
            ("io", "open") => {
                let raw = text(args, 0)?;
                let mode = if args.len() > 1 { text(args, 1)? } else { "r".to_owned() };
                let (save, path) = lua_path(&raw)?;
                let write = mode.contains('w') || mode.contains('a');
                ensure!(save || !write, "AdvHD io.open({raw:?}, {mode:?}) would write the game directory");
                let data = if mode.contains('w') {
                    Vec::new()
                } else {
                    match self.services.storage(self.client)?.read(&path) {
                        Ok(bytes) => bytes.to_vec(),
                        Err(_) if write => Vec::new(),
                        Err(e) => return Ok(vec![V::Nil, V::str(&e.to_string())]),
                    }
                };
                let at = if mode.contains('a') { data.len() } else { 0 };
                let id = self.registry.add(Object::File { path: path.relative().to_owned(), save, data, at, write });
                vec![V::UserData(id)]
            }
            other => bail!("AdvHD {}.{} is not connected", other.0, other.1),
        })
    }
}

impl crysirror_lua::Host for Bridge<'_> {
    fn call(&mut self, lua: &mut crysirror_lua::Lua, id: u32, args: Vec<crysirror_lua::Value>) -> Result<Vec<crysirror_lua::Value>> {
        use crate::system::{CFUNCS, LIBS, LIB_BASE, METHOD_BASE};
        if (LIB_BASE..LIB_BASE + LIBS.len() as u32).contains(&id) {
            return self.library(LIBS[(id - LIB_BASE) as usize], &args);
        }
        if id < METHOD_BASE {
            let name = CFUNCS.get(id as usize).copied().context("AdvHD cfunc id")?;
            return self.cfunc(name, &args);
        }
        let method = self.registry.method_name(id).context("AdvHD method id")?;
        let object = match args.first() {
            Some(crysirror_lua::Value::UserData(object)) => self.registry.objects.get(object).cloned(),
            _ => None,
        }
        .with_context(|| format!("AdvHD method {method} called on a non-object"))?;
        self.method(lua, object, id, &method, &args)
    }
    fn index(&mut self, lua: &mut crysirror_lua::Lua, object: u32, key: &crysirror_lua::Value) -> Result<crysirror_lua::Value> {
        ensure!(self.registry.objects.contains_key(&object), "AdvHD object {object} is gone");
        let name = key.as_bytes().context("AdvHD object key is not a string")?;
        Ok(self.registry.method(lua, name))
    }
    fn print(&mut self, text: &[u8]) {
        eprintln!("AdvHD Lua: {}", encoding_rs::SHIFT_JIS.decode(text).0);
    }
}

///Mounted archive lookup and parsed PNA cache.
pub struct Resources {
    pub client: ClientId,
    pub archives: Vec<String>,
    index: BTreeMap<String, String>,
    pna: BTreeMap<String, Arc<(Pna, Vec<u8>)>>,
}
impl Resources {
    ///ArcFileName policy over the mounted archives: names are matched
    ///case-insensitively (scripts write `EF_WHITE.PNG` for `ef_white.png`);
    ///the first archive in mount order that holds the name wins.
    pub fn resolve(&mut self, host: &mut dyn HostServices, name: &str) -> Result<String> {
        if self.index.is_empty() {
            let storage = host.storage(self.client)?;
            for archive in &self.archives {
                for entry in storage.list(Mount::Asset, archive)? {
                    self.index.entry(entry.to_ascii_lowercase()).or_insert_with(|| format!("{archive}/{entry}"));
                }
            }
            ensure!(!self.index.is_empty(), "AdvHD mounted archives are empty");
        }
        self.index
            .get(&name.replace('\\', "/").to_ascii_lowercase())
            .cloned()
            .with_context(|| format!("AdvHD resource {name:?} is absent from the mounted archives"))
    }
    pub fn pna(&mut self, host: &mut dyn HostServices, file: &str) -> Result<Arc<(Pna, Vec<u8>)>> {
        if let Some(pna) = self.pna.get(file) {
            return Ok(pna.clone());
        }
        let path = self.resolve(host, file)?;
        let bytes = host.storage(self.client)?.read(&VfsPath::new(Mount::Asset, &path)?)?;
        let pna = Arc::new((Pna::parse(&bytes).with_context(|| format!("AdvHD PNA {file}"))?, bytes.to_vec()));
        self.pna.insert(file.to_owned(), pna.clone());
        Ok(pna)
    }
}

///`{base:reading}` ruby: plain text and (first, end, reading) spans over
///the plain text's non-newline characters.
pub(crate) fn ruby_spans(text: &str) -> (String, Vec<(usize, usize, String)>) {
    let mut plain = String::new();
    let mut spans = Vec::new();
    let mut count = 0;
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}').map(|c| open + c) else { break };
        let inner = &rest[open + 1..close];
        let Some((base, reading)) = inner.split_once(':') else { break };
        let before = &rest[..open];
        plain.push_str(before);
        count += before.chars().filter(|c| *c != '\n').count();
        let first = count;
        plain.push_str(base);
        count += base.chars().filter(|c| *c != '\n').count();
        spans.push((first, count, reading.trim().to_owned()));
        rest = &rest[close + 1..];
    }
    plain.push_str(rest);
    (plain, spans)
}

///Insert each span's reading (half size, centred over its base glyphs,
///above the line) after the span's last base glyph.
pub(crate) fn with_ruby(glyphs: Vec<Json>, spans: &[(usize, usize, String)], size: f64) -> Vec<Json> {
    let mut out = Vec::with_capacity(glyphs.len() + spans.len() * 4);
    let small = (size / 2.).round().max(4.);
    for (n, glyph) in glyphs.iter().enumerate() {
        out.push(glyph.clone());
        for (first, end, reading) in spans.iter().filter(|s| s.1 == n + 1 && s.0 < s.1) {
            let base = &glyphs[*first..(*end).min(glyphs.len())];
            let left = base.first().and_then(|g| g["baseline"][0].as_f64()).unwrap_or(0.);
            let right = base.last().and_then(|g| g["baseline"][0].as_f64()).unwrap_or(left) + size;
            let y = base.first().and_then(|g| g["baseline"][1].as_f64()).unwrap_or(0.) - size;
            let count = reading.chars().count() as f64;
            let pitch = ((right - left) / count.max(1.)).min(small * 1.5);
            let start = left + ((right - left) - pitch * count) / 2.;
            for (k, c) in reading.chars().enumerate() {
                out.push(json!({"character":c.to_string(),"baseline":[start + pitch * k as f64 + (pitch - small) / 2., y + small / 8.],"size":small}));
            }
        }
    }
    out
}

///Layer `order` is one flat [major, minor] space (parents are not sorting
///fences), so the tree's sibling orders become a depth-first rank.
fn flatten_order(ops: &mut [Json]) {
    let mut children: BTreeMap<u64, Vec<(i64, i64, u64, usize)>> = BTreeMap::new();
    let mut roots = Vec::new();
    for (at, op) in ops.iter().enumerate() {
        let layer = &op["layer"];
        let Some(id) = layer["id"].as_u64() else { continue };
        let key = (layer["order"][0].as_i64().unwrap_or(0), layer["order"][1].as_i64().unwrap_or(0), id, at);
        match layer["parent"].as_u64() {
            Some(parent) => children.entry(parent).or_default().push(key),
            None => roots.push(key),
        }
    }
    let mut rank = 0i32;
    let mut stack: Vec<(i64, i64, u64, usize)> = Vec::new();
    roots.sort();
    stack.extend(roots.into_iter().rev());
    while let Some((_, _, id, at)) = stack.pop() {
        ops[at]["layer"]["order"] = json!([0, rank]);
        rank += 1;
        if let Some(mut list) = children.remove(&id) {
            list.sort();
            stack.extend(list.into_iter().rev());
        }
    }
}

///AdvHD colour dwords are 0xRRGGBBAA (255 = opaque black, the edge of
///white message text 0x2A1953FF).
pub(crate) fn rgba(v: u32) -> [u8; 4] {
    [(v >> 24) as u8, (v >> 16) as u8, (v >> 8) as u8, v as u8]
}

///Cell layout of a text layer: full-width cells of `size`, half-width ASCII,
///line pitch `size * (1 + line_margin)`, kinsoku on closing punctuation;
///`align` 1 centres and 2 right-aligns each line within `width`.
pub(crate) fn layout_glyphs(text: &str, width: f64, size: f64, line_margin: f64, align: u8) -> Result<Vec<Json>> {
    use crysirror_text::grid::{layout_cells, CellCursor, CellLayoutOptions};
    let px = size.round().clamp(1., 512.) as i32;
    let options = CellLayoutOptions {
        bounds: [width.clamp(1., 32767.) as i32, 32767],
        size: px,
        spacing: [0, (size * line_margin).round() as i32],
        half_width: true,
        half_spacing: false,
        hanging: px,
        forbidden_line_start: "、。，．・：；？！ゝゞヽヾー…‥）」』】〉》〕］｝ぁぃぅぇぉっゃゅょァィゥェォッャュョ".into(),
        indent_pairs: vec![['「', '」'], ['『', '』'], ['（', '）']],
        cell_widths: Vec::new(),
    };
    let layout = layout_cells(text, &options, &mut CellCursor::at_start())?;
    let mut rows: BTreeMap<i32, i32> = BTreeMap::new();
    for glyph in &layout.glyphs {
        let half = glyph.character.is_ascii() || ('\u{ff61}'..='\u{ff9f}').contains(&glyph.character);
        let right = glyph.position[0] + if half { px / 2 } else { px };
        let end = rows.entry(glyph.position[1]).or_insert(0);
        *end = (*end).max(right);
    }
    Ok(layout
        .glyphs
        .iter()
        .map(|g| {
            let row = rows.get(&g.position[1]).copied().unwrap_or(0);
            let shift = match align {
                1 => (width as i32 - row) / 2,
                2 => width as i32 - row,
                _ => 0,
            };
            json!({"character":g.character.to_string(),"baseline":[g.position[0] + shift, g.position[1] + px * 7 / 8]})
        })
        .collect())
}

///Begin playback; the preparation receipt is kept until Ready (releasing
///an unpublished request cancels it).
#[allow(clippy::too_many_arguments)]
pub(crate) fn start_audio(host: &mut dyn HostServices, client: ClientId, state: &mut State, channel: u32, path: &str, looping: bool, gain: f32, bus: PcmBus) -> Result<()> {
    let master = if state.master_mute { 0. } else { state.master_volume.or_else(|| state.registry_dwords.get("MasterVolume").copied()).unwrap_or(1000) as f32 / 1000. };
    let gain = gain * master.clamp(0., 1.);
    let ticket = host.begin_audio(
        client,
        AudioRequest::PlayAsset {
            channel,
            source: AudioAsset { path: VfsPath::new(Mount::Asset, path)?, frame_limit: None },
            intro: None,
            parameters: AudioParameters { looping, gain, bus, ..Default::default() },
        },
    )?;
    state.audio_pending.push(ticket);
    state.channels.insert(channel);
    Ok(())
}

///Release receipts whose preparation finished (Ready keeps playing).
pub(crate) fn poll_audio(host: &mut dyn HostServices, client: ClientId, state: &mut State) -> Result<()> {
    use crysirror_core::service::AudioPreparePhase;
    let mut keep = Vec::new();
    for ticket in std::mem::take(&mut state.audio_pending) {
        let prepared = host.audio_preparation(client, ticket)?;
        match prepared.phase {
            AudioPreparePhase::Pending => keep.push(ticket),
            AudioPreparePhase::Ready => host.release_audio(client, ticket)?,
            AudioPreparePhase::Failed => {
                host.release_audio(client, ticket)?;
                bail!("AdvHD audio preparation: {}", prepared.error.unwrap_or_default());
            }
        }
    }
    state.audio_pending = keep;
    Ok(())
}

///Named sound slots (WS2 and Lua share them) map onto stable host channels.
pub(crate) fn sound_channel(name: &str) -> u32 {
    let mut hash = 0u32;
    for b in name.to_ascii_lowercase().bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(u32::from(b));
    }
    16 + hash % 4000
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ruby_spans_index_plain_characters() {
        let (plain, spans) = ruby_spans("{表示:ひょうじ}の\n{漢字:かんじ}");
        assert_eq!(plain, "表示の\n漢字");
        assert_eq!(spans, vec![(0, 2, "ひょうじ".to_owned()), (3, 5, "かんじ".to_owned())]);
    }
    #[test]
    fn flat_order_follows_the_tree_depth_first() {
        let mut ops = vec![
            json!({"op":"put","layer":{"id":1,"order":[0,0]}}),
            json!({"op":"put","layer":{"id":10,"parent":1,"order":[5,0]}}),
            json!({"op":"put","layer":{"id":11,"parent":10,"order":[9,0]}}),
            json!({"op":"put","layer":{"id":20,"parent":1,"order":[2,0]}}),
            json!({"op":"put","layer":{"id":21,"parent":20,"order":[99,0]}}),
        ];
        flatten_order(&mut ops);
        let rank = |id: u64| ops.iter().find(|o| o["layer"]["id"] == id).unwrap()["layer"]["order"][1].as_i64().unwrap();
        assert!(rank(1) < rank(20) && rank(20) < rank(21) && rank(21) < rank(10) && rank(10) < rank(11));
    }
    #[test]
    fn colours_are_rgba_dwords() {
        assert_eq!(rgba(0x2a19_53ff), [0x2a, 0x19, 0x53, 0xff]);
        assert_eq!(rgba(255), [0, 0, 0, 255]);
    }
    #[test]
    fn save_slots_follow_the_scripts_numbering() {
        use crate::save::{file_name, MODE_AUTO, MODE_NORMAL, MODE_QUICK};
        assert_eq!(file_name(MODE_NORMAL, 1).as_deref(), Some("willplus_001.sav"));
        assert_eq!(file_name(MODE_QUICK, 5).as_deref(), Some("willplus_q5.sav"));
        assert!(file_name(MODE_AUTO, 6).is_none() && file_name(MODE_NORMAL, 0).is_none());
    }
}

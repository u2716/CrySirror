// Copyright (c) 2026 CrySirror contributors. MIT; see LICENSE.
//! AdvHD Lua system layer: Script.arc's Lua 5.3 chunks run on the shared
//! `crysirror-lua` VM. This module is the AdvHD side of that boundary: the
//! `cfunc` function table, `cclass` objects and `LUA_*` constants the
//! scripts read but never define.
//!
//! AdvHD.exe is packed, so the native implementations were not read. Names
//! and call shapes come from the chunks; constant values are host-assigned
//! here (the scripts only pass them back to the host or compare them with
//! values the host produced). Every binding without an implementation fails
//! visibly with its name.
use anyhow::{bail, Context, Result};
use crysirror_lua::{Host, Lua, Value};
use std::collections::BTreeMap;

/// `cfunc.*` names in Script.arc.
pub const CFUNCS: &[&str] = &[
    "CSaveDataManager__lua_FindNewData",
    "CSaveDataManager__lua_GetEnabled",
    "CSaveDataManager__lua_GetSaveDataDate",
    "CSaveDataManager__lua_GetSaveDataText",
    "CSaveDataManager__lua_GetSavePath",
    "CSaveDataManager__lua_GetThumbnailPath",
    "CSaveDataManager__lua_IsData",
    "CSaveDataManager__lua_IsResume",
    "D3DApp__lua_PostMouseMove",
    "LegacyGame__lua_GetFlag",
    "LegacyGame__lua_GetGameState",
    "LegacyGame__lua_GetString",
    "LegacyGame__lua_GetVariable",
    "LegacyGame__lua_NeedInterrupt",
    "LegacyGame__lua_NeedPreview",
    "LegacyGame__lua_NeedProcess",
    "LegacyGame__lua_NeedShell",
    "LegacyGame__lua_NeedVolumeUpdate",
    "LegacyGame__lua_SelectItem",
    "LegacyGame__lua_SetFlag",
    "LegacyGame__lua_SetGameState",
    "LegacyGame__lua_SetLoadProc",
    "LegacyGame__lua_SetSaveComment",
    "LegacyGame__lua_SetString",
    "LegacyGame__lua_SetVariable",
    "MyConv__lua_sjistoutf8",
    "MyConv__lua_utf8tosjis",
];

/// `cclass.*` names in Script.arc.
pub const CLASSES: &[&str] = &[
    "CLegacyFontInfo",
    "lua_AutoCursor",
    "lua_DataSort",
    "lua_Layers",
    "lua_MeasureTime",
    "lua_Mixer",
    "lua_Registry",
    "lua_Sounds",
];

/// Host-assigned constant values. Key codes use the Windows virtual-key
/// numbering and charsets the Windows charset ids, which the host also
/// produces; the others are opaque enumerations.
pub fn constants() -> Vec<(&'static str, i64)> {
    let mut out = vec![
        ("LUA_KEYCODE_ENTER", 13),
        ("LUA_KEYCODE_SPACE", 32),
        ("LUA_KEYCODE_PAGEUP", 33),
        ("LUA_KEYCODE_PAGEDOWN", 34),
        ("LUA_KEYCODE_END", 35),
        ("LUA_KEYCODE_HOME", 36),
        ("LUA_KEYCODE_LEFT", 37),
        ("LUA_KEYCODE_UP", 38),
        ("LUA_KEYCODE_RIGHT", 39),
        ("LUA_KEYCODE_DOWN", 40),
        ("SHIFTJIS_CHARSET", 128),
        ("GB2312_CHARSET", 134),
        ("CHINESEBIG5_CHARSET", 136),
    ];
    let groups: &[&[&str]] = &[
        &[
            "LUA_STATE_DISABLEDATA", "LUA_STATE_MSGSKIP", "LUA_STATE_MSGHIDE", "LUA_STATE_TRANSITION", "LUA_STATE_ENABLELOAD",
            "LUA_STATE_MSGOPEN", "LUA_STATE_MSGAUTO", "LUA_STATE_MSGCTRLSKIP", "LUA_STATE_ENABLESKIP", "LUA_STATE_ENABLELOG",
            "LUA_STATE_ENABLEQLOAD", "LUA_STATE_ENABLEVOICEREPEAT", "LUA_STATE_ENABLESELECTLOAD",
        ],
        &[
            "LUA_PROCESS_GAME_UNPAUSED", "LUA_PROCESS_GAME_PAUSED", "LUA_PROCESS_UPDATEBGMCUT", "LUA_PROCESS_ASPECTMODE",
            "LUA_PROCESS_QUICKSAVEGAME", "LUA_PROCESS_SCREENMODE", "LUA_PROCESS_TITLE", "LUA_PROCESS_UPDATEBGV",
            "LUA_PROCESS_QUICKLOADGAME", "LUA_PROCESS_SAVEGAME", "LUA_PROCESS_UPDATEDATA", "LUA_PROCESS_UPDATEMSGSPEED",
            "LUA_PROCESS_CHANGELANGUAGE", "LUA_PROCESS_VOICEREPEAT", "LUA_PROCESS_SELECTSAVEGAME", "LUA_PROCESS_MINIMIZED",
            "LUA_PROCESS_MOUSEMOVE", "LUA_PROCESS_QUIT", "LUA_PROCESS_AUTOLOADGAME", "LUA_PROCESS_LOADGAME",
            "LUA_PROCESS_BACKGAME", "LUA_PROCESS_SELECTLOADGAME", "LUA_PROCESS_LOG_VOICEPLAY", "LUA_PROCESS_RESUMEGAME",
            "LUA_PROCESS_RTSNAP",
        ],
        &["LUA_PARAM_POS", "LUA_PARAM_COLOR", "LUA_PARAM_SIZE", "LUA_PARAM_SCALE", "LUA_PARAM_RECT", "LUA_PARAM_ROTATE", "LUA_PARAM_CENTERZ"],
        &["LUA_EFFECT_OPACITY", "LUA_EFFECT_POS", "LUA_EFFECT_SCALE", "LUA_EFFECT_ROTATE"],
        &["LUA_CONFIRM_TITLE", "LUA_CONFIRM_ALBUM", "LUA_CONFIRM_LOAD", "LUA_CONFIRM_OVERWRITE", "LUA_CONFIRM_QUIT", "LUA_CONFIRM_QUICKLOAD", "LUA_CONFIRM_SELECTLOAD", "LUA_CONFIRM_MEMLOAD", "LUA_CONFIRM_DEFAULT"],
        &["LUA_SKIPSPEED_NONE", "LUA_SKIPSPEED_OMIT"],
        &["LUA_TRANSITION_NORMAL", "LUA_TRANSITION_ERASE"],
        &["LUA_LAYER_STD", "LUA_LAYER_PNA"],
        &["DATA_SORT_UPDATEDATE"],
        &["DATA_ORDER_DESC"],
    ];
    //The scripts also pass these as literals (FindNewData(2, ..) for auto).
    out.extend([("DATA_MODE_NORMAL", 0), ("DATA_MODE_QUICK", 1), ("DATA_MODE_AUTO", 2)]);
    for group in groups {
        for (n, name) in group.iter().enumerate() {
            out.push((name, n as i64 + 1));
        }
    }
    out
}

/// Host function ids: cfunc entries, then class constructors/methods.
pub const CFUNC_BASE: u32 = 0;
pub const METHOD_BASE: u32 = 0x1000;

#[derive(Clone, Debug)]
pub enum Object {
    Class(&'static str),
    Instance { class: &'static str, id: u32 },
    /// `io.open` handle: contents, read cursor, and the save path to commit.
    File { path: String, save: bool, data: Vec<u8>, at: usize, write: bool },
}

/// `os`/`io` host functions (storage and clock are host policy).
pub const LIBS: &[(&str, &str)] = &[("os", "time"), ("os", "date"), ("os", "remove"), ("os", "clock"), ("io", "open")];
pub const LIB_BASE: u32 = 0x800;

/// AdvHD objects addressed from Lua through userdata handles, and the
/// shared method functions.
#[derive(Default)]
pub struct Registry {
    pub objects: BTreeMap<u32, Object>,
    pub next_object: u32,
    methods: BTreeMap<Vec<u8>, Value>,
    method_names: Vec<Vec<u8>>,
}
impl Registry {
    pub fn add(&mut self, object: Object) -> u32 {
        self.next_object = self.next_object.max(1);
        let id = self.next_object;
        self.next_object += 1;
        self.objects.insert(id, object);
        id
    }
    /// The bound-method function for `name` (one shared value per name).
    pub fn method(&mut self, lua: &mut Lua, name: &[u8]) -> Value {
        if let Some(f) = self.methods.get(name) {
            return f.clone();
        }
        let id = METHOD_BASE + self.method_names.len() as u32;
        self.method_names.push(name.to_vec());
        let f = lua.host_function(id);
        lua.registry.push(f.clone());
        self.methods.insert(name.to_vec(), f.clone());
        f
    }
    pub fn method_name(&self, id: u32) -> Option<String> {
        self.method_names.get(id.checked_sub(METHOD_BASE)? as usize).map(|n| String::from_utf8_lossy(n).into_owned())
    }
}

/// Lua VM plus AdvHD objects.
#[derive(Default)]
pub struct System {
    pub lua: Lua,
    pub registry: Registry,
    pub loaded: bool,
}

impl System {
    /// Install the AdvHD globals (`cfunc`, `cclass`, constants).
    pub fn install(&mut self) -> Result<()> {
        let lua = &mut self.lua;
        let cfunc = lua.new_table();
        for (n, name) in CFUNCS.iter().enumerate() {
            let f = lua.host_function(CFUNC_BASE + n as u32);
            lua.set_field(cfunc, name, f)?;
        }
        lua.set_global("cfunc", Value::Table(cfunc))?;
        let cclass = lua.new_table();
        for name in CLASSES {
            let id = self.registry.add(Object::Class(name));
            lua.set_field(cclass, name, Value::UserData(id))?;
        }
        lua.set_global("cclass", Value::Table(cclass))?;
        let mut tables = BTreeMap::new();
        for (n, (lib, name)) in LIBS.iter().enumerate() {
            let table = *tables.entry(*lib).or_insert_with(|| lua.new_table());
            let f = lua.host_function(LIB_BASE + n as u32);
            lua.set_field(table, name, f)?;
        }
        for (lib, table) in tables {
            lua.set_global(lib, Value::Table(table))?;
        }
        for (name, value) in constants() {
            lua.set_global(name, Value::Int(value))?;
        }
        Ok(())
    }
}

/// Run chunk main functions in order.
pub fn run_chunks(lua: &mut Lua, chunks: &[(String, Vec<u8>)], host: &mut dyn Host) -> Result<()> {
    for (name, bytes) in chunks {
        let proto = crysirror_lua::chunk::load(bytes).with_context(|| format!("AdvHD Lua chunk {name}"))?;
        let main = lua.load(proto)?;
        lua.call(&main, vec![], host).with_context(|| format!("AdvHD Lua chunk {name}"))?;
    }
    Ok(())
}

/// Lua value helpers for bindings.
pub fn int(args: &[Value], n: usize) -> Result<i64> {
    let v = args.get(n).cloned().unwrap_or_default();
    match v {
        Value::Bool(b) => Ok(i64::from(b)),
        Value::Nil => Ok(0),
        other => other.as_i64().or_else(|| other.as_f64().map(|f| f as i64)).with_context(|| format!("AdvHD binding argument {} is {}", n + 1, other.type_name())),
    }
}
pub fn num(args: &[Value], n: usize) -> Result<f64> {
    let v = args.get(n).cloned().unwrap_or_default();
    match v {
        Value::Bool(b) => Ok(f64::from(u8::from(b))),
        Value::Nil => Ok(0.),
        other => other.as_f64().with_context(|| format!("AdvHD binding argument {} is {}", n + 1, other.type_name())),
    }
}
pub fn text(args: &[Value], n: usize) -> Result<String> {
    match args.get(n) {
        Some(Value::Str(s)) => {
            //Script strings are CP932 except LegacyGame_utf8's UTF-8 text.
            match std::str::from_utf8(s) {
                Ok(t) => Ok(t.to_owned()),
                Err(_) => Ok(encoding_rs::SHIFT_JIS.decode(s).0.into_owned()),
            }
        }
        Some(v @ (Value::Int(_) | Value::Num(_))) => Ok(String::from_utf8_lossy(&v.display()).into_owned()),
        Some(Value::Nil) | None => Ok(String::new()),
        Some(other) => bail!("AdvHD binding argument {} is {}", n + 1, other.type_name()),
    }
}

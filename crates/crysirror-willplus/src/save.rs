// Copyright (c) 2026 CrySirror contributors. MIT; see LICENSE.
//! AdvHD save data behind the scripts' `CSaveDataManager__lua_*` queries
//! and `NeedProcess(LUA_PROCESS_*SAVE*/*LOAD*)` requests.
//!
//! Slot numbering follows the scripts: normal data are 1-based
//! (`(page-1)*10 + no + 1`), quick and auto data rotate through slots 1..5
//! (`FindNewData` returns the mode and newest slot; the scripts walk
//! backwards from it), and `DATA_MODE_NORMAL/QUICK/AUTO` are 0/1/2 (the
//! scripts also pass those as literals). A save holds the game state as it
//! was when the current line (0x14) or choice (0x0F) began, so loading
//! re-runs that instruction; the Lua system layer and settings stay live.
//!
//! Persistent data (read lines, picked choices, flags/variables >= 1000,
//! registry settings) live in `system.json`; flags/variables below 1000 are
//! per-playthrough. The original file layout is not reproduced: AdvHD.exe is
//! packed and its format is not observable from the scripts.
use crate::vm::{Machine, State, Wait};
use anyhow::{Context, Result};
use crysirror_core::path::{Mount, VfsPath};
use crysirror_core::service::{AudioRequest, HostServices};
use crysirror_lua::Value as V;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub const MODE_NORMAL: i64 = 0;
pub const MODE_QUICK: i64 = 1;
pub const MODE_AUTO: i64 = 2;
/// Host-internal modes (not visible to the scripts as data pages).
pub const MODE_SELECT: i64 = 10;
pub const MODE_RESUME: i64 = 11;
/// First flag/variable index kept across playthroughs.
pub const GLOBAL_INDEX: u16 = 1000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveFile {
    pub version: u32,
    /// Unix seconds.
    pub time: i64,
    pub date: String,
    pub text: String,
    pub state: State,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Global {
    pub read: BTreeSet<i32>,
    pub chosen: BTreeSet<i16>,
    pub flags: BTreeSet<u16>,
    pub vars: BTreeMap<u16, f32>,
    pub registry: BTreeMap<String, i64>,
    pub strings: BTreeMap<i64, String>,
}

pub fn file_name(mode: i64, slot: i64) -> Option<String> {
    Some(match mode {
        MODE_NORMAL if (1..=999).contains(&slot) => format!("willplus_{slot:03}.sav"),
        MODE_QUICK if (1..=5).contains(&slot) => format!("willplus_q{slot}.sav"),
        MODE_AUTO if (1..=5).contains(&slot) => format!("willplus_a{slot}.sav"),
        MODE_SELECT => "willplus_select.sav".into(),
        MODE_RESUME => "willplus_resume.sav".into(),
        _ => return None,
    })
}

/// `GetThumbnailPath` value: the save's own scene is composed as the
/// thumbnail (no screen readback).
pub const THUMB_PREFIX: &str = "willplus-thumb:";

pub fn cached(saves: &mut BTreeMap<String, Option<Arc<SaveFile>>>, host: &mut dyn HostServices, client: crysirror_core::state::ClientId, mode: i64, slot: i64) -> Option<Arc<SaveFile>> {
    let name = file_name(mode, slot)?;
    if let Some(found) = saves.get(&name) {
        return found.clone();
    }
    let loaded = VfsPath::new(Mount::Save, &name)
        .ok()
        .and_then(|path| host.storage(client).ok()?.read(&path).ok())
        .and_then(|bytes| match serde_json::from_slice::<SaveFile>(&bytes) {
            Ok(file) => Some(file),
            Err(error) => {
                //Unreadable data shows as an empty slot (as a foreign or
                //damaged file would); the cause is reported once.
                eprintln!("CrySirror: AdvHD save {name} is unreadable: {error}");
                None
            }
        })
        .map(Arc::new);
    saves.insert(name, loaded.clone());
    loaded
}

pub(crate) fn unix() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

/// Date text `YYYY/MM/DD HH:MM` of local seconds (Unix + UTC offset).
pub fn date_text(seconds: i64) -> String {
    let [y, mo, d, h, mi, _] = crate::vm::civil(seconds);
    format!("{y:04}/{mo:02}/{d:02} {h:02}:{mi:02}")
}

impl Machine {
    pub(crate) fn read_save(&mut self, host: &mut dyn HostServices, mode: i64, slot: i64) -> Option<Arc<SaveFile>> {
        cached(&mut self.state.saves, host, self.client, mode, slot)
    }

    fn write_save(&mut self, host: &mut dyn HostServices, mode: i64, slot: i64, file: SaveFile) -> Result<()> {
        let name = file_name(mode, slot).with_context(|| format!("AdvHD save slot {mode}/{slot}"))?;
        host.write_atomic(self.client, &VfsPath::new(Mount::Save, &name)?, &serde_json::to_vec(&file)?)?;
        self.state.saves.insert(name, Some(Arc::new(file)));
        self.save_global(host)?;
        self.refresh_data_states(host);
        Ok(())
    }

    /// Newest quick/auto slot (0 when none).
    pub(crate) fn newest(&mut self, host: &mut dyn HostServices, mode: i64) -> i64 {
        let mut best = (i64::MIN, 0);
        let top = if mode == MODE_NORMAL { 100 } else { 5 };
        for slot in 1..=top {
            if let Some(save) = self.read_save(host, mode, slot) {
                if save.time > best.0 || (save.time == best.0 && best.1 == 0) {
                    best = (save.time, slot);
                }
            }
        }
        best.1
    }

    /// State at the start of the current line/choice, for saving.
    pub(crate) fn capture(&self, pc: u32) -> State {
        let mut state = self.state.clone();
        state.pc = pc;
        state.wait = Wait::None;
        state.ui = Default::default();
        state.msg = Default::default();
        state.select = None;
        state.read.clear();
        state.chosen.clear();
        state.log.clear();
        state.processes.clear();
        state.saves.clear();
        state.audio_pending.clear();
        state.paused = false;
        state.flags.retain(|f| *f < GLOBAL_INDEX);
        state.vars.retain(|v, _| *v < GLOBAL_INDEX);
        state
    }

    pub(crate) fn save_to(&mut self, host: &mut dyn HostServices, mode: i64, slot: i64) -> Result<()> {
        let state = self.checkpoint.as_deref().cloned().unwrap_or_else(|| self.capture(self.state.pc));
        let time = unix();
        let text = if self.state.save_comment.is_empty() { self.checkpoint_text.clone() } else { std::mem::take(&mut self.state.save_comment) };
        let date = date_text(time + i64::from(self.state.utc_offset));
        self.write_save(host, mode, slot, SaveFile { version: 1, time, date, text, state })
    }

    /// Replace the game state with a save; the Lua layer, settings and
    /// persistent data stay.
    pub(crate) fn load_from(&mut self, host: &mut dyn HostServices, mode: i64, slot: i64) -> Result<()> {
        let save = self.read_save(host, mode, slot).with_context(|| format!("AdvHD save {mode}/{slot} is empty"))?;
        self.load_state(host, save.state.clone())
    }

    pub(crate) fn load_state(&mut self, host: &mut dyn HostServices, mut next: State) -> Result<()> {
        self.stop_all_audio(host)?;
        self.stop_movie(host)?;
        if self.state.msg.open {
            self.lua_call(host, "closeMessageWindow", vec![V::Bool(true)])?;
        }
        self.lua_call(host, "faceRead", vec![V::str(".png")])?;
        let current = &mut self.state;
        next.ui = std::mem::take(&mut current.ui);
        next.game_state = std::mem::take(&mut current.game_state);
        next.registry_dwords = std::mem::take(&mut current.registry_dwords);
        next.fonts = std::mem::take(&mut current.fonts);
        next.read = std::mem::take(&mut current.read);
        next.chosen = std::mem::take(&mut current.chosen);
        next.log = std::mem::take(&mut current.log);
        next.measures = std::mem::take(&mut current.measures);
        next.lua_sounds = std::mem::take(&mut current.lua_sounds);
        next.utc_offset = current.utc_offset;
        next.saves = std::mem::take(&mut current.saves);
        next.audio_pending = std::mem::take(&mut current.audio_pending);
        next.system_ready = current.system_ready;
        next.cursor = current.cursor;
        next.paused = current.paused;
        next.last_frame = current.last_frame;
        next.flags.extend(current.flags.iter().filter(|f| **f >= GLOBAL_INDEX));
        next.vars.extend(current.vars.iter().filter(|v| *v.0 >= GLOBAL_INDEX));
        next.wait = Wait::None;
        next.msg = Default::default();
        next.select = None;
        next.channels.clear();
        for key in ["LUA_STATE_MSGOPEN", "LUA_STATE_MSGSKIP", "LUA_STATE_MSGAUTO", "LUA_STATE_MSGHIDE"] {
            next.game_state.insert(Self::state_id(key), false);
        }
        let music = next.music.clone();
        self.state = next;
        for (slot, (file, looping, volume)) in music {
            let channel = crate::vm::sound_channel(&slot);
            self.play(host, channel, &slot, &file, looping, true, volume)?;
        }
        self.checkpoint = None;
        self.dirty = true;
        Ok(())
    }

    pub(crate) fn stop_all_audio(&mut self, host: &mut dyn HostServices) -> Result<()> {
        for channel in std::mem::take(&mut self.state.channels) {
            host.submit_audio(self.client, AudioRequest::Stop { channel, fade_ms: 0. })?;
        }
        self.state.music.clear();
        Ok(())
    }

    pub(crate) fn load_global(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let Ok(path) = VfsPath::new(Mount::Save, "willplus_system.json") else { return Ok(()) };
        let Ok(bytes) = host.storage(self.client)?.read(&path) else { return Ok(()) };
        let global: Global = serde_json::from_slice(&bytes).context("AdvHD system data")?;
        self.state.read.extend(global.read);
        self.state.chosen.extend(global.chosen);
        self.state.flags.extend(global.flags);
        self.state.vars.extend(global.vars);
        self.state.registry_dwords.extend(global.registry);
        for (k, v) in global.strings {
            self.state.strings.entry(k).or_insert(v);
        }
        Ok(())
    }

    pub(crate) fn save_global(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let global = Global {
            read: self.state.read.clone(),
            chosen: self.state.chosen.clone(),
            flags: self.state.flags.iter().copied().filter(|f| *f >= GLOBAL_INDEX).collect(),
            vars: self.state.vars.iter().filter(|v| *v.0 >= GLOBAL_INDEX).map(|(k, v)| (*k, *v)).collect(),
            registry: self.state.registry_dwords.clone(),
            strings: BTreeMap::new(),
        };
        host.write_atomic(self.client, &VfsPath::new(Mount::Save, "willplus_system.json")?, &serde_json::to_vec(&global)?)?;
        self.global_written = host.now_ms();
        Ok(())
    }

    /// `NeedProcess` requests queued during a Lua call, run after it.
    pub(crate) fn run_processes(&mut self, host: &mut dyn HostServices) -> Result<()> {
        while !self.state.processes.is_empty() {
            let (process, argument) = self.state.processes.remove(0);
            let name = crate::system::constants().into_iter().find(|(k, v)| *v == process && k.starts_with("LUA_PROCESS_")).map_or("?", |(k, _)| k);
            match name {
                "LUA_PROCESS_SAVEGAME" => self.save_to(host, MODE_NORMAL, argument)?,
                "LUA_PROCESS_QUICKSAVEGAME" => {
                    let next = self.newest(host, MODE_QUICK) % 5 + 1;
                    self.save_to(host, MODE_QUICK, next)?;
                }
                "LUA_PROCESS_SELECTSAVEGAME" => self.save_to(host, MODE_SELECT, 0)?,
                "LUA_PROCESS_LOADGAME" => self.load_from(host, MODE_NORMAL, argument)?,
                "LUA_PROCESS_QUICKLOADGAME" => {
                    let slot = if argument > 0 { argument } else { self.newest(host, MODE_QUICK) };
                    self.load_from(host, MODE_QUICK, slot)?;
                }
                "LUA_PROCESS_AUTOLOADGAME" => {
                    let slot = if argument > 0 { argument } else { self.newest(host, MODE_AUTO) };
                    self.load_from(host, MODE_AUTO, slot)?;
                }
                "LUA_PROCESS_SELECTLOADGAME" => self.load_from(host, MODE_SELECT, 0)?,
                "LUA_PROCESS_RESUMEGAME" => self.load_from(host, MODE_RESUME, 0)?,
                "LUA_PROCESS_TITLE" => self.to_title(host)?,
                //Scene jump-back from the log: that line's state, with the
                //log cut before it (the line is shown and logged again).
                "LUA_PROCESS_BACKGAME" => {
                    let entry = usize::try_from(argument).ok().filter(|e| *e < self.state.log.len()).context("AdvHD BACKGAME entry")?;
                    let state = self.state.log[entry].checkpoint.clone().context("AdvHD BACKGAME entry without a checkpoint")?;
                    self.state.log.truncate(entry);
                    self.load_state(host, (*state).clone())?;
                }
                "LUA_PROCESS_QUIT" => {
                    self.save_global(host)?;
                    self.state.halted = true;
                }
                "LUA_PROCESS_VOICEREPEAT" => {
                    if let Some((slot, file)) = self.state.line_voice.clone() {
                        let channel = crate::vm::sound_channel(&slot);
                        self.play(host, channel, &slot, &file, false, false, 10)?;
                    }
                }
                "LUA_PROCESS_LOG_VOICEPLAY" => {
                    let entry = usize::try_from(argument).ok().and_then(|n| self.state.log.get(n)).and_then(|e| e.voice.clone());
                    if let Some((slot, file)) = entry {
                        let channel = crate::vm::sound_channel(&slot);
                        self.play(host, channel, &slot, &file, false, false, 10)?;
                    }
                }
                //Settings the scripts already hold (re-read where used),
                //data list refreshes, and window-system requests a
                //host-presented canvas does not have.
                "LUA_PROCESS_UPDATEDATA" | "LUA_PROCESS_UPDATEMSGSPEED" | "LUA_PROCESS_UPDATEBGMCUT" | "LUA_PROCESS_UPDATEBGV" | "LUA_PROCESS_ASPECTMODE"
                | "LUA_PROCESS_SCREENMODE" | "LUA_PROCESS_MINIMIZED" | "LUA_PROCESS_MOUSEMOVE" | "LUA_PROCESS_CHANGELANGUAGE" | "LUA_PROCESS_RTSNAP" => {}
                other => anyhow::bail!("AdvHD NeedProcess({other}, {argument}) is not connected"),
            }
        }
        Ok(())
    }

    /// Back to the title: playthrough state is dropped and TITLE.ws2 runs.
    pub(crate) fn to_title(&mut self, host: &mut dyn HostServices) -> Result<()> {
        self.save_global(host)?;
        let mut fresh = State { scene: "TITLE".into(), next_id: 1, ..Default::default() };
        fresh.flags = self.state.flags.iter().copied().filter(|f| *f >= GLOBAL_INDEX).collect();
        fresh.vars = self.state.vars.iter().filter(|v| *v.0 >= GLOBAL_INDEX).map(|(k, v)| (*k, *v)).collect();
        self.load_state(host, fresh)
    }
}

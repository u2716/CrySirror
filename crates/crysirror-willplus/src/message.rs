// Copyright (c) 2026 CrySirror contributors. MIT; see LICENSE.
//! AdvHD message window, choices and in-game input over the Lua system layer.
//!
//! Host contract, from the LegacyGame.lua call shapes (AdvHD.exe is packed):
//! WS2 text is drawn by the host into the text layers the scripts name
//! (`getMessageWindowKey` -> `system/Txt01`, `getNameWindowKey` ->
//! `system/Name01`) with the `CLegacyFontInfo` style registered for them;
//! the window itself opens/closes through `openMessageWindow` /
//! `closeMessageWindow` and runs its own effects (`isMessageWindowEffect`).
//! Click waits run `startClickWaitAnimation`/`stopClickWaitAnimation`; the
//! character interval is `getMessageSpeed()` ms and the auto-mode interval
//! `getAutoWaitTime()` ms per character. Choices call `initSelect(n)`, write
//! each item into `getSelectTextKey(i)` and colour it with
//! `setSelectTextColor`/`disableSelectItem`; the scripts' buttons report the
//! pick through `LegacyGame__lua_SelectItem(i)`.
//!
//! Pointer events go to the topmost set: a paused game's menu, otherwise
//! `system` when a hit-test sublayer is under the pointer (message window
//! buttons, choices); other clicks advance text. Wheel-up opens the log
//! (the scripts' `onMouseWheel` ignores `system`), right click toggles the
//! window through the scripts' `RD` path.
use crate::program::{Instruction, Program};
use crate::vm::{Machine, Wait};
use anyhow::{Context, Result};
use crysirror_core::service::{buttons, HostServices, InputFrame};
use crysirror_core::vm::VmYield;
use crysirror_lua::Value as V;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Message {
    /// `openMessageWindow` ran and `closeMessageWindow` has not.
    pub open: bool,
    /// Text on the page (shown and revealing).
    pub page: String,
    /// Characters of `page` visible before the current segment.
    pub shown: usize,
    /// Segments after the current one; each follows a `%K` click wait.
    pub pending: Vec<String>,
    /// `%P`: the next message starts a new page.
    pub clear: bool,
    pub start: u64,
    /// Milliseconds per character (0 = instant).
    pub interval: u64,
    /// Click-wait animation is running.
    pub clicking: bool,
    pub clicked_at: u64,
    pub id: i32,
    /// The current message was read in an earlier pass.
    pub read: bool,
    /// Name window shown (`nameWindowVisible`).
    pub named: bool,
    /// The window's open effect was running last frame.
    pub effect: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Select {
    /// 0x0F instruction offset in the current scene.
    pub pc: u32,
    pub ids: Vec<i16>,
    pub chosen: Option<usize>,
}

/// Text alignment codes at the start of a WS2 string.
pub fn alignment(text: &str) -> (u8, &str) {
    for (code, align) in [("%LC", 1), ("%LR", 2), ("%LL", 0)] {
        if let Some(rest) = text.strip_prefix(code) {
            return (align, rest);
        }
    }
    (0, text)
}

impl Machine {
    /// Load/quick-load/select-load availability for the window buttons.
    pub(crate) fn refresh_data_states(&mut self, host: &mut dyn HostServices) {
        use crate::save::{MODE_NORMAL, MODE_QUICK, MODE_SELECT};
        let quick = self.newest(host, MODE_QUICK) > 0;
        let normal = self.newest(host, MODE_NORMAL) > 0 || quick || self.newest(host, crate::save::MODE_AUTO) > 0;
        let select = self.read_save(host, MODE_SELECT, 0).is_some();
        self.set_game_state("LUA_STATE_ENABLEQLOAD", quick);
        self.set_game_state("LUA_STATE_ENABLELOAD", normal);
        self.set_game_state("LUA_STATE_ENABLESELECTLOAD", select);
    }
    pub(crate) fn state_id(name: &str) -> i64 {
        crate::system::constants().into_iter().find(|(k, _)| *k == name).map_or(-1, |(_, v)| v)
    }
    pub(crate) fn game_state(&self, name: &str) -> bool {
        self.state.game_state.get(&Self::state_id(name)).copied().unwrap_or(false)
    }
    pub(crate) fn set_game_state(&mut self, name: &str, on: bool) {
        self.state.game_state.insert(Self::state_id(name), on);
    }
    fn lua_int(&mut self, host: &mut dyn HostServices, name: &str) -> Result<i64> {
        let out = self.lua_call(host, name, vec![])?;
        Ok(out.first().and_then(V::as_f64).map_or(0, |v| v as i64))
    }
    fn lua_bool(&mut self, host: &mut dyn HostServices, name: &str, args: Vec<V>) -> Result<bool> {
        Ok(self.lua_call(host, name, args)?.first().is_some_and(|v| v.truthy()))
    }
    pub(crate) fn skipping(&self, input: &InputFrame) -> bool {
        self.game_state("LUA_STATE_MSGSKIP") || input.buttons_down & buttons::CTRL != 0
    }

    /// WS2 0x14: show one message (segments split at `%K`).
    pub(crate) fn message_show(&mut self, host: &mut dyn HostServices, pc: u32, id: i32, raw: &str, now: u64) -> Result<Option<VmYield>> {
        self.checkpoint = Some(Box::new(self.capture(pc)));
        self.checkpoint_text = format!("{}{}", if self.state.speaker.is_empty() { String::new() } else { format!("{}：", self.state.speaker) }, Self::message(&raw.replace("%K", "").replace("%P", ""))?.replace('\n', ""));
        //Continue data (title CONTINUE) and persistent data, at most every 10s.
        if now.saturating_sub(self.global_written) >= 10_000 {
            self.save_to(host, crate::save::MODE_RESUME, 0)?;
        }
        let parts: Vec<&str> = raw.split("%K").collect();
        let (last, waits) = parts.split_last().context("AdvHD message")?;
        let page_end = last.contains("%P");
        let mut segments = waits.iter().map(|s| Self::message(s)).collect::<Result<Vec<_>>>()?;
        let tail = Self::message(&last.replace("%P", ""))?;
        if !tail.is_empty() {
            segments.push(tail);
        }
        if segments.is_empty() {
            segments.push(String::new());
        }
        if !self.state.msg.open {
            self.lua_call(host, "openMessageWindow", vec![V::Bool(false)])?;
            self.state.msg.open = true;
            self.set_game_state("LUA_STATE_MSGOPEN", true);
            //The host asks for the name layer once per window; the scripts
            //mark the name window in use (MenuInf.nameWindow) there.
            self.lua_call(host, "getMessageWindowKey", vec![V::Int(0)])?;
            self.lua_call(host, "getNameWindowKey", vec![V::Int(0)])?;
        }
        let named = !self.state.speaker.is_empty();
        if named != self.state.msg.named {
            self.state.msg.named = named;
            self.lua_call(host, "nameWindowVisible", vec![V::Bool(named)])?;
        }
        if self.state.msg.clear {
            self.state.msg.page.clear();
            self.state.msg.clear = false;
        }
        let read = self.state.read.contains(&id);
        self.state.read.insert(id);
        self.lua_call(host, "setMessageColor", vec![V::Int(i64::from(read))])?;
        let interval = self.lua_int(host, "getMessageSpeed")?.max(0) as u64;
        let voiced = self.state.voice.is_some();
        self.set_game_state("LUA_STATE_ENABLESKIP", read);
        self.set_game_state("LUA_STATE_ENABLELOG", true);
        self.set_game_state("LUA_STATE_ENABLEVOICEREPEAT", voiced);
        self.set_game_state("LUA_STATE_TRANSITION", false);
        self.refresh_data_states(host);
        self.lua_call(host, "setButtonState", vec![])?;
        let first = segments.remove(0);
        let msg = &mut self.state.msg;
        msg.id = id;
        msg.read = read;
        msg.shown = msg.page.chars().count();
        msg.page.push_str(&first);
        msg.pending = segments;
        msg.clear = page_end;
        msg.start = now;
        msg.interval = interval;
        msg.clicking = false;
        self.state.body = self.state.msg.page.clone();
        self.state.window = true;
        self.dirty = true;
        let voice = self.state.voice.take();
        self.state.line_voice = voice.clone();
        let checkpoint = self.checkpoint.as_deref().cloned().map(std::sync::Arc::new);
        self.state.log.push(crate::vm::LogEntry { speaker: self.state.speaker.clone(), text: raw.to_owned(), voice, checkpoint });
        if self.state.log.len() > 256 {
            self.state.log.remove(0);
        }
        self.state.wait = Wait::Text;
        Ok(Some(VmYield::WaitUntil(now + 16)))
    }

    pub(crate) fn message_visible(&self, now: u64) -> usize {
        let msg = &self.state.msg;
        let total = msg.page.chars().count();
        if msg.interval == 0 {
            return total;
        }
        (msg.shown + (now.saturating_sub(msg.start) / msg.interval) as usize).min(total)
    }

    /// Wait::Text: reveal, click wait, auto and skip. Returns true when the
    /// message is finished and the script continues.
    pub(crate) fn message_wait(&mut self, host: &mut dyn HostServices, input: &InputFrame, pressed: bool, now: u64) -> Result<bool> {
        if self.lua_bool(host, "isMessageWindowEffect", vec![])? {
            self.state.msg.start = now;
            self.state.msg.effect = true;
            return Ok(false);
        }
        if std::mem::take(&mut self.state.msg.effect) {
            //Button availability depends on the open effect having ended.
            self.lua_call(host, "setButtonState", vec![])?;
        }
        //Ctrl forces skipping; skip mode passes read text only unless the
        //all-skip setting is on.
        let skip = input.buttons_down & buttons::CTRL != 0
            || (self.game_state("LUA_STATE_MSGSKIP") && (self.state.msg.read || self.lua_bool(host, "getAllSkip", vec![])?));
        let total = self.state.msg.page.chars().count();
        if self.message_visible(now) < total {
            if pressed || skip {
                self.state.msg.shown = total;
                self.state.msg.interval = 0;
                self.dirty = true;
            } else {
                self.dirty = true;
                return Ok(false);
            }
            if !skip {
                return Ok(false);
            }
        }
        if !self.state.msg.clicking {
            self.state.msg.clicking = true;
            self.state.msg.clicked_at = now;
            self.lua_call(host, "startClickWaitAnimation", vec![])?;
        }
        let auto = self.game_state("LUA_STATE_MSGAUTO") && {
            let per = self.lua_int(host, "getAutoWaitTime")?.max(0) as u64;
            now >= self.state.msg.clicked_at + per * total.saturating_sub(self.state.msg.shown).max(1) as u64
        };
        if !(pressed || skip || auto) {
            return Ok(false);
        }
        self.lua_call(host, "stopClickWaitAnimation", vec![])?;
        self.state.msg.clicking = false;
        if self.state.msg.pending.is_empty() {
            return Ok(true);
        }
        let next = self.state.msg.pending.remove(0);
        let msg = &mut self.state.msg;
        msg.shown = msg.page.chars().count();
        msg.page.push_str(&next);
        msg.start = now;
        self.state.body = self.state.msg.page.clone();
        self.dirty = true;
        Ok(false)
    }

    /// WS2 0x15: speaker line for the next message.
    pub(crate) fn message_speaker(&mut self, host: &mut dyn HostServices, raw: &str) -> Result<()> {
        let (_, rest) = alignment(raw);
        let name = Self::message(rest)?;
        let _ = host;
        self.state.speaker = name;
        self.state.speaker_raw = raw.to_owned();
        self.dirty = true;
        Ok(())
    }

    /// WS2 0x16: close the window (scene changes).
    pub(crate) fn message_close(&mut self, host: &mut dyn HostServices, now: u64) -> Result<Option<VmYield>> {
        self.state.window = false;
        self.state.body.clear();
        self.state.speaker.clear();
        self.state.speaker_raw.clear();
        self.state.msg.page.clear();
        self.state.msg.pending.clear();
        self.state.msg.clear = false;
        self.dirty = true;
        if self.state.msg.open {
            self.state.msg.open = false;
            self.state.msg.named = false;
            self.set_game_state("LUA_STATE_MSGOPEN", false);
            self.lua_call(host, "closeMessageWindow", vec![V::Bool(false)])?;
            self.state.wait = Wait::WindowEffect;
            return Ok(Some(VmYield::WaitUntil(now + 16)));
        }
        Ok(None)
    }

    /// WS2 0x0F: choice list (id, text, condition kind, flag) per item.
    pub(crate) fn select_start(&mut self, host: &mut dyn HostServices, program: &Program, i: &Instruction, now: u64) -> Result<Option<VmYield>> {
        self.checkpoint = Some(Box::new(self.capture(i.pc)));
        let next = self.newest(host, crate::save::MODE_AUTO) % 5 + 1;
        self.save_to(host, crate::save::MODE_AUTO, next)?;
        let mut items = Vec::new();
        for choice in &i.choices {
            let id = crate::vm::short(&choice.operands[0])?;
            let text = Self::message(&crate::vm::string(program, &choice.operands[1])?)?;
            let kind = crate::vm::byte(&choice.operands[2])?;
            let flag = crate::vm::short(&choice.operands[3])?;
            anyhow::ensure!(kind == 0, "AdvHD choice condition kind {kind} is not connected");
            let enabled = u16::try_from(flag).is_ok_and(|f| self.state.flags.contains(&f));
            items.push((id, text, enabled));
        }
        self.lua_call(host, "initSelect", vec![V::Int(items.len() as i64)])?;
        for (n, (id, text, enabled)) in items.iter().enumerate() {
            let key = self.lua_call(host, "getSelectTextKey", vec![V::Int(n as i64)])?;
            let (set, key) = (key.first().and_then(|v| v.as_bytes()).map(|b| String::from_utf8_lossy(b).into_owned()), key.get(1).and_then(|v| v.as_bytes()).map(|b| String::from_utf8_lossy(b).into_owned()));
            let (Some(set), Some(key)) = (set, key) else { anyhow::bail!("AdvHD getSelectTextKey({n})") };
            let item = self.state.ui.item_mut(&set, &key)?;
            let crate::layers::Kind::Text { text: body, .. } = &mut item.kind else { anyhow::bail!("AdvHD select item {set}/{key} is not a text layer") };
            *body = text.clone();
            let seen = self.state.chosen.contains(id);
            self.lua_call(host, "setSelectTextColor", vec![V::Int(n as i64), V::Int(i64::from(seen))])?;
            if !enabled {
                self.lua_call(host, "disableSelectItem", vec![V::Int(n as i64)])?;
            }
        }
        self.state.select = Some(Select { pc: i.pc, ids: items.iter().map(|i| i.0).collect(), chosen: None });
        self.state.wait = Wait::Select;
        self.dirty = true;
        Ok(Some(VmYield::WaitUntil(now + 16)))
    }

    /// Wait::Select: once picked and the close effect ended, run the item's
    /// embedded continuation instruction.
    pub(crate) fn select_wait(&mut self, host: &mut dyn HostServices, now: u64) -> Result<bool> {
        let Some(select) = self.state.select.clone() else { return Ok(true) };
        //Polling isSelectEffect also ends the open effect (MenuInf.select_effect),
        //which gates the scripts' button input.
        let effect = self.lua_bool(host, "isSelectEffect", vec![])?;
        let Some(chosen) = select.chosen else { return Ok(false) };
        if effect {
            return Ok(false);
        }
        let program = self.program(host, &self.state.scene.clone())?;
        let instruction = program.instruction_at(select.pc)?.clone();
        let choice = instruction.choices.get(chosen).with_context(|| format!("AdvHD choice {chosen}"))?;
        self.state.chosen.insert(select.ids[chosen]);
        self.state.select = None;
        self.state.pc = instruction.next;
        let continuation = choice.continuation.clone();
        self.execute(host, &program, &continuation, now)?;
        Ok(true)
    }

    /// Pointer/keys to the Lua system layer; returns true when the event was
    /// consumed (a menu is open, or a system button was hit).
    pub(crate) fn ui_frame(&mut self, host: &mut dyn HostServices, input: &InputFrame, now: u64) -> Result<bool> {
        if !self.system.loaded || !self.state.system_ready {
            return Ok(false);
        }
        if self.state.ui.tick(now)? {
            self.dirty = true;
        }
        let delta = now.saturating_sub(self.state.last_frame.unwrap_or(now));
        self.state.last_frame = Some(now);
        let paused = self.state.paused;
        let top = self.state.ui.sets.iter().rev().find(|s| s.opacity > 0. && s.name != "system" && s.name != "overlay").map(|s| s.name.clone());
        let target = match (&top, paused) {
            (Some(set), true) => set.clone(),
            _ => "system".to_owned(),
        };
        if let Some(set) = &top {
            self.lua_call(host, "addFrame", vec![V::str(set), V::Int(delta as i64)])?;
        }
        let pointer = [f64::from(input.pointer_x), f64::from(input.pointer_y)];
        let moved = pointer != self.state.cursor;
        self.state.cursor = pointer;
        let hit = {
            let mut geometry = crate::vm::GeometryOf { res: &mut self.res, services: host };
            self.state.ui.hit(&target, pointer[0], pointer[1], &mut geometry)?
        };
        let (key, record) = hit.clone().map_or((String::new(), -1), |(k, r)| (k, r as i64));
        let args = |flag: bool| vec![V::str(&target), V::str(&key), V::Int(record), V::Num(pointer[0]), V::Num(pointer[1]), V::Bool(flag)];
        let mut consumed = paused;
        if moved {
            self.lua_call(host, "onMouseMove", args(false))?;
        }
        let left = input.buttons_pressed & buttons::MOUSE_LEFT != 0;
        if left && std::env::var_os("WILLPLUS_TEXT_TRACE").is_some() {
            let subs: Vec<String> = self.state.ui.sets.iter().filter(|s| s.name == target).flat_map(|s| &s.items).map(|i| format!("{}:{:?}", i.key, i.subs.iter().filter(|x| x.1.hit.is_some()).map(|x| *x.0).collect::<Vec<_>>())).collect();
            eprintln!("WILLPLUS click {target} {pointer:?} hit={hit:?} hittable={subs:?}");
            let items: Vec<(String, String, Vec<usize>)> = self.state.ui.sets.iter().filter(|s| s.name == target).flat_map(|s| &s.items).filter_map(|i| match &i.kind { crate::layers::Kind::Pna { path } => Some((i.key.clone(), path.clone(), i.subs.iter().filter(|x| x.1.visible).map(|x| *x.0).collect())), _ => None }).collect();
            for (key, path, visible) in items {
                let mut geometry = crate::vm::GeometryOf { res: &mut self.res, services: host };
                let records = crate::layers::Geometry::records(&mut geometry, &path)?;
                eprintln!("  {key} {path} visible={visible:?} rects={:?}", records.iter().enumerate().filter_map(|(n, r)| r.map(|r| (n, r))).collect::<Vec<_>>());
            }
        }
        let released = self.state.mouse_held && input.buttons_down & buttons::MOUSE_LEFT == 0;
        if left {
            if !paused && self.game_state("LUA_STATE_MSGHIDE") {
                self.lua_call(host, "hideMessageWindow", vec![V::Int(0)])?;
                self.set_game_state("LUA_STATE_MSGHIDE", false);
                consumed = true;
            } else if !paused && self.game_state("LUA_STATE_MSGSKIP") {
                self.set_game_state("LUA_STATE_MSGSKIP", false);
                consumed = true;
            } else if paused || hit.is_some() {
                self.lua_call(host, "onLButtonDown", args(false))?;
                if input.buttons_down & buttons::MOUSE_LEFT != 0 {
                    self.state.mouse_held = true;
                } else {
                    self.lua_call(host, "onLButtonUp", args(false))?;
                }
                consumed = true;
            }
        } else if released {
            self.state.mouse_held = false;
            self.lua_call(host, "onLButtonUp", args(false))?;
            consumed = true;
        }
        if input.buttons_pressed & buttons::MOUSE_RIGHT != 0 && (paused || self.state.msg.open) {
            self.lua_call(host, "onRButtonUp", args(false))?;
            consumed = true;
        }
        if input.buttons_pressed & buttons::WHEEL_UP != 0 {
            if paused {
                self.lua_call(host, "onMouseWheel", vec![V::str(&target), V::Int(1)])?;
            } else if self.state.msg.open {
                self.lua_call(host, "openMessageLog", vec![])?;
            }
            consumed = true;
        }
        if input.buttons_pressed & buttons::WHEEL_DOWN != 0 && paused {
            self.lua_call(host, "onMouseWheel", vec![V::str(&target), V::Int(-1)])?;
            consumed = true;
        }
        self.lua_call(host, "isSystemEffect", vec![V::Bool(false)])?;
        //effectText reveals: the interval follows the current setting; the
        //end is reported once per text through onMessageOutEnd(set).
        self.state.ui.text_interval = self.lua_int(host, "getMessageSpeed")?.max(0) as u64;
        let interval = self.state.ui.text_interval.max(1);
        let mut ended = Vec::new();
        for set in &mut self.state.ui.sets {
            for item in &mut set.items {
                if let crate::layers::Kind::Text { text, reveal, .. } = &mut item.kind {
                    if reveal.is_some_and(|start| now.saturating_sub(start) / interval >= text.chars().count() as u64) {
                        *reveal = None;
                        ended.push(set.name.clone());
                    }
                }
            }
        }
        for set in ended {
            self.lua_call(host, "onMessageOutEnd", vec![V::str(&set)])?;
        }
        //The message window's per-frame jobs (end of open/close effects,
        //hop-up menu, icons, tips) run inside its effect poll.
        if !paused && !matches!(self.state.wait, Wait::Text | Wait::WindowEffect) {
            self.lua_call(host, "isMessageWindowEffect", vec![])?;
        }
        self.dirty = true;
        Ok(consumed)
    }
}

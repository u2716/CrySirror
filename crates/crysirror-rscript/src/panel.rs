// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! The companion panel's commands, backlog browsing and button mode 1 of
//! the Shapeshifter build.
//!
//! Panel buttons (CTexBoxEx 0x427400, callbacks 0x42ADD0..0x429860) call
//! the game's callbacks at +228..+276: skip (+228, needs the tab switch),
//! next (+232, skip to the next choice), rev (+236, previous choice), hide
//! (+240), menu (+244, configuration), save (+248) and load (+252, both
//! need the panel save/load switch), voc (+256, replay of the page voice),
//! auto (+260), the window slider (+264), extd (+268, a plug-in DLL),
//! qsave (+272) and qload (+276). bak and fow page the backlog.
//!
//! Backlog browsing (sub_4294D0 back, sub_429590 forward, sub_428D10 show,
//! sub_429D80 close) shows a page start of the ring in box 0 with that
//! entry's box state, face and voice; moving forward onto the newest page
//! closes it and restores the live page.
use crate::memory::{message as M, scene as S};
use crate::scene::{Ctx, MessageWindow};
use crate::vm::Vm;
use anyhow::Result;
use crysirror_core::service::HostServices;

fn entry(i: usize) -> usize {
    S::MESSAGE + M::BACKLOG + i * M::BACKLOG_STRIDE
}

impl Vm {
    fn page_start(&self, i: usize) -> bool {
        self.mem.scene[entry(i) + M::ENTRY_PAGE_START] != 0
    }
    /// sub_4294D0 (bak, wheel up, cursor up). False when it does nothing.
    pub(crate) fn backlog_back(&mut self, h: &mut dyn HostServices) -> bool {
        if !self.message.input_active {
            return false;
        }
        let current = match self.backlog_view {
            Some(i) => i,
            None => {
                if MessageWindow::dword(&self.mem, M::BACKLOG_ENABLED) == 0 {
                    return false;
                }
                // sub_429F80: the newest page start.
                match (0..M::BACKLOG_COUNT).rev().find(|&i| self.page_start(i)) {
                    Some(i) => i,
                    None => return true,
                }
            }
        };
        // sub_42A010: the page start before it.
        let Some(prev) = (0..current).rev().find(|&i| self.page_start(i)) else { return true };
        if self.backlog_view.is_none() {
            self.message.set_waiting(&self.mem, false);
        }
        self.show_backlog(h, prev);
        true
    }
    /// sub_429590 (fow, wheel down, cursor down). False when not browsing.
    pub(crate) fn backlog_forward(&mut self, h: &mut dyn HostServices) -> bool {
        let Some(current) = self.backlog_view else { return false };
        match (current + 1..M::BACKLOG_COUNT).find(|&i| self.page_start(i)) {
            Some(next) if (next + 1..M::BACKLOG_COUNT).any(|i| self.page_start(i)) => self.show_backlog(h, next),
            _ => self.close_backlog(h),
        }
        true
    }
    /// sub_428D10.
    fn show_backlog(&mut self, h: &mut dyn HostServices, index: usize) {
        self.backlog_view = Some(index);
        let color = self.apini.backlog_color;
        let mem = self.mem.clone();
        let programs = self.programs.clone();
        let mut string = |script: u16, i: u32| -> Option<Vec<u8>> {
            let p = programs.get(&i32::from(script))?;
            let s = p.strings.get(i as usize)?;
            Some(crate::text::expand(
                s,
                |levels, mut v| {
                    for _ in 0..levels {
                        v = i32::from(mem.var(v));
                    }
                    v
                },
                |r| mem.str_at(S::STRING_REGISTERS + r * S::STRING_STRIDE, S::STRING_STRIDE).to_vec(),
            ))
        };
        let text = crate::scene::backlog_page(&self.mem, index, &mut string);
        let e = entry(index);
        let (script, name) = (self.mem.uw(e + M::ENTRY_SCRIPT), self.mem.d(e + M::ENTRY_NAME) as u32);
        let name = if name != 0 { string(script, name) } else { None };
        let mut cx = Ctx { h: &mut *h, client: self.client, res: &mut self.res, apini: &self.apini };
        let b = &mut self.message.boxes[0];
        b.record = e + M::ENTRY_BOX;
        b.clear_text();
        b.apply(&self.mem, &mut cx);
        if let Some(c) = color {
            b.style.color = c & 0xFFFFFF;
        }
        b.append_text(&text);
        b.finish();
        b.name.clear();
        b.name_visible = name.as_ref().is_some_and(|n| !n.is_empty());
        if let Some(n) = name.filter(|n| !n.is_empty()) {
            let style = b.name_style.clone();
            b.name.append(&n, &style);
        }
        // sub_429C10: the entry's face and voice (+112 page block).
        let page = e + M::ENTRY_PAGE;
        let voice = self.mem.d(page + (M::VOICE - M::PAGE));
        self.message.set_voice_available(voice != 0);
        self.dirty = true;
    }
    /// sub_429D80: the live page comes back.
    pub(crate) fn close_backlog(&mut self, h: &mut dyn HostServices) {
        if self.backlog_view.take().is_none() {
            return;
        }
        self.message.boxes[0].record = S::MESSAGE + M::BOXES;
        self.rebuild_message(h);
        self.message.set_waiting(&self.mem, self.flags.wait_click);
        self.dirty = true;
    }

    /// Companion panel commands (the CTexBoxEx callbacks).
    pub(crate) fn panel_command(&mut self, h: &mut dyn HostServices, command: &str) -> Result<()> {
        if !self.message.input_active {
            return Ok(());
        }
        let state = |vm: &Self, o: usize| MessageWindow::dword(&vm.mem, o) != 0;
        if self.backlog_view.is_some() && !matches!(command, "bak" | "fow" | "voc") {
            self.close_backlog(h);
        }
        match command {
            // sub_42EEA0.
            "skip" => {
                if state(self, M::TAB_ENABLED) && !self.flags.skip {
                    self.flags.skip_held = true;
                    self.click(h)?;
                }
            }
            // sub_42EF00: skips to the next choice unless one is open.
            "next" => {
                if state(self, M::TAB_ENABLED) && !self.flags.choice && self.flags.button_wait == 0 {
                    self.flags.fast_skip_request = true;
                    self.click(h)?;
                }
            }
            // sub_42EE50.
            "rev" => self.previous_choice(h)?,
            "bak" => {
                self.backlog_back(h);
            }
            "fow" => {
                if !self.backlog_forward(h) {
                    self.click(h)?;
                }
            }
            // sub_42ED30.
            "hide" => self.show_window(false),
            // sub_42EED0: the shown page's voice (browsing: the entry's).
            "voc" => {
                let block = match self.backlog_view {
                    Some(i) => entry(i) + M::ENTRY_PAGE,
                    None => S::MESSAGE + M::PAGE,
                };
                let voice = self.mem.d(block + (M::VOICE - M::PAGE)) as u32;
                let pan = i32::from(self.mem.w(block + (M::VOICE_PAN - M::PAGE)));
                if voice != 0 {
                    self.play_voice(h, voice, 0, false, pan)?;
                }
            }
            // sub_42EB90.
            "auto" => {
                if self.flags.choice || self.flags.button_wait != 0 {
                    self.message.auto_hidden = false;
                    self.message.update_panel(&self.mem);
                    return Ok(());
                }
                self.message.auto_hidden = true;
                self.message.update_panel(&self.mem);
                self.flags.auto = true;
                self.click(h)?;
            }
            // sub_4297D0 / sub_42ED20.
            "menu" => {
                if state(self, M::MENU_ENABLED) {
                    self.open_config(h, false)?;
                }
            }
            // sub_429F40 / sub_429450.
            "save" | "load" => {
                if state(self, M::PANEL_BUTTONS) {
                    self.open_save_screen(h, command == "save")?;
                }
            }
            // sub_42EC20: the APINI plug-in DLL is game code.
            // sub_42EC20: the APINI extension DLL (crate::plugins).
            "extd" => {
                let name = self.apini.extension_dll.clone();
                if !name.is_empty() {
                    if let crate::plugins::Resolved::Facade(f) = self.resolve_plugin(h, &name, "the extd button")? {
                        match f.extension {
                            Some(run) => run(self, h)?,
                            None => anyhow::bail!("RScript plug-in {name:?} has no extension interface"),
                        }
                    }
                }
            }
            // sub_42EE10.
            "qsave" => {
                self.system_sound(h, 1)?;
                self.ask(h, crate::save::Ask::QuickSave)?;
            }
            // sub_42EDB0.
            "qload" => self.quick_load(h)?,
            other => self.res.diag(format!("unknown panel command {other}")),
        }
        self.dirty = true;
        Ok(())
    }
    /// Dragging the panel slider.
    pub(crate) fn drag_panel_slider(&mut self, x: i32, y: i32) {
        let p = self.message.panel_pos;
        let Some(sl) = &mut self.message.slider else { return };
        let v = sl.value_at(x - p.x, y - p.y);
        if v != sl.value {
            sl.value = v;
            sl.place();
            self.set_window_alpha(v);
        }
    }
    /// The slider shows the configuration word 0x1A.
    pub(crate) fn sync_panel_slider(&mut self) {
        let v = i32::from(self.mem.cfg(crate::memory::config::WINDOW_ALPHA));
        if let Some(sl) = &mut self.message.slider {
            sl.value = v.clamp(0, sl.max);
            sl.place();
        }
    }
    /// sub_4289D0 + sub_42EBE0: the panel slider sets every box's back
    /// transparency and the configuration word 0x1A.
    pub(crate) fn set_window_alpha(&mut self, value: i32) {
        let v = value.clamp(0, 255);
        self.mem.set_cfg(crate::memory::config::WINDOW_ALPHA, v);
        for b in &mut self.message.boxes {
            b.back_alpha = v as u8;
            b.back.level = 255 - b.back_alpha;
        }
        self.dirty = true;
    }

    /// Button mode 1 (sub_431440): native screens and states by id.
    pub(crate) fn native_button(&mut self, h: &mut dyn HostServices, id: i32) -> Result<()> {
        if self.flags.button_wait == 0 {
            return Ok(());
        }
        self.close_backlog(h);
        match id {
            0 => {
                self.system_sound(h, 1)?;
                self.quit(h)?;
            }
            1 => {
                self.system_sound(h, 1)?;
                self.return_to_title(h)?;
            }
            2 => {
                self.system_sound(h, 1)?;
                self.enter_state(h, crate::extras::State::TitleLoad)?;
            }
            3 => {
                self.system_sound(h, 1)?;
                self.enter_state(h, crate::extras::State::TitleConfig)?;
            }
            4 => {
                self.system_sound(h, 1)?;
                self.open_config(h, false)?;
            }
            5 => {
                self.system_sound(h, 1)?;
                self.open_save_screen(h, false)?;
            }
            6 => {
                self.system_sound(h, 1)?;
                self.open_save_screen(h, true)?;
            }
            7 => {
                self.system_sound(h, 1)?;
                self.enter_state(h, crate::extras::State::MusicMenu)?;
            }
            8 => {
                self.system_sound(h, 1)?;
                self.enter_state(h, crate::extras::State::CgView)?;
            }
            9 => {
                self.system_sound(h, 1)?;
                self.enter_state(h, crate::extras::State::SceneView)?;
            }
            10 => {
                self.system_sound(h, 1)?;
                self.enter_state(h, crate::extras::State::ExtraMenu)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// sub_42E8D0: input ends the idle count and brings back what the
    /// auto-hide switch hid.
    pub(crate) fn track_idle(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let input = self.input;
        let active = input.buttons_pressed != 0 || [input.pointer_x, input.pointer_y] != self.pointer;
        if !active {
            return Ok(());
        }
        self.idle_ticks = 0;
        if self.idle_hidden {
            self.idle_hidden = false;
            h.submit_engine(self.client, 1, &serde_json::to_vec(&serde_json::json!({"ui":[{"op":"set_cursor_visible","visible":true}]}))?)?;
            if !self.flags.window_hidden {
                self.message.hidden = false;
            }
            self.dirty = true;
        }
        Ok(())
    }
    /// sub_4308C0 during button waits: after 100 idle ticks the cursor and
    /// the message window hide when the auto-hide switch is on.
    pub(crate) fn idle_tick(&mut self, h: &mut dyn HostServices) -> Result<()> {
        if self.mem.cfg(crate::memory::config::AUTO_HIDE) == 0 || self.idle_hidden || self.flags.button_wait == 0 {
            return Ok(());
        }
        self.idle_ticks += 1;
        if self.idle_ticks > 100 {
            self.idle_hidden = true;
            h.submit_engine(self.client, 1, &serde_json::to_vec(&serde_json::json!({"ui":[{"op":"set_cursor_visible","visible":false}]}))?)?;
            self.message.hidden = true;
            self.dirty = true;
        }
        Ok(())
    }
}

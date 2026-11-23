// SPDX-License-Identifier: MPL-2.0
//! `MessageWindowMgr` under-buttons (`Skip`, `Auto`, `Voice`, `BackLog`,
//! `Config`), the backlog viewer (`BacklogCtrl`) and the `ADVKagManager`
//! skip/auto advance.
use crate::{
    adapter::{Game, Opener},
    sound::{Kind, SoundOp},
};
use anyhow::{Context, Result};

const WINDOW: &str = "__GameMain__/SystemUI Root/MessageWindowPanel";
const BUTTONS: [&str; 5] = ["Skip", "Auto", "Voice", "BackLog", "Config"];
/// `MessageWindowMgr.kBackLogSaveUnit`.
pub const BACKLOG_UNITS: usize = 30;

/// Message window state of `MessageWindowMgr`/`ADVKagManager`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MessageWindow {
    /// `ADVKagManager.auto_mode`.
    pub auto_mode: bool,
    /// `MessageWindowCtrl.IsDisplayBackLog`.
    pub backlog_open: bool,
    /// `SystemButton.select` per under-button.
    selected: [bool; 5],
    /// Frame counter of the skip advance (every second update).
    update_count: u64,
}

fn button_node(game: &Game, name: &str) -> Result<usize> {
    game.main.scene.find(&format!("{WINDOW}/MessageViewer/Buttons/{name}")).with_context(|| format!("message window button {name}"))
}

impl Game {
    /// `SystemButton.enabled` setter.
    fn window_button_enabled(&mut self, i: usize, on: bool) -> Result<()> {
        let n = button_node(self, BUTTONS[i])?;
        self.main.states.entry(n).or_default().enabled = Some(on);
        if !on {
            self.window_button_select(i, false)?;
        }
        Ok(())
    }

    /// `SystemButton.select`: the selected button shows its default colour
    /// at full alpha.
    fn window_button_select(&mut self, i: usize, on: bool) -> Result<()> {
        if self.msgwnd.selected[i] == on {
            return Ok(());
        }
        let n = button_node(self, BUTTONS[i])?;
        let mut c = self.main.scene.script(n, "UIButton").and_then(|f| f.floats("mColor").or_else(|| f.floats("defaultColor")).map(|c| [c[0], c[1], c[2], c[3]])).unwrap_or([1., 1., 1., 0.67058825]);
        if let Some(f) = self.main.scene.script(n, "UISprite").and_then(|f| f.floats("mColor")) {
            c = [f[0], f[1], f[2], f[3]];
        }
        if on {
            self.main.states.entry(n).or_default().enabled = Some(true);
            c[3] = 1.;
        }
        self.main.states.entry(n).or_default().color = Some(c);
        self.msgwnd.selected[i] = on;
        Ok(())
    }

    fn current_voice(&self) -> Option<String> {
        self.backlog.last().map(|b| b.2.clone()).filter(|v| !v.is_empty())
    }

    /// `MessageWindowMgr.CallEvent`.
    pub fn window_call_event(&mut self, opener: &mut Opener, name: &str) -> Result<()> {
        if self.cm_system.open {
            return Ok(());
        }
        let voice = self.current_voice();
        match name {
            "Skip" => {
                let skip = self.skip_mode;
                self.window_button_select(0, !skip)?;
                for i in [1, 2, 3, 4] {
                    self.window_button_enabled(i, skip)?;
                }
                self.skip_mode = !skip;
                if skip {
                    self.window_button_enabled(2, voice.is_some())?;
                }
            }
            "Auto" => {
                let auto = self.msgwnd.auto_mode;
                self.window_button_select(1, !auto)?;
                for i in [0, 2, 3, 4] {
                    self.window_button_enabled(i, auto)?;
                }
                self.msgwnd.auto_mode = !auto;
                if auto {
                    self.window_button_enabled(2, voice.is_some())?;
                }
            }
            "Voice" => {
                if let Some(v) = voice {
                    self.sound_ops.push(SoundOp::Stop { kind: Kind::Voice, fade_s: 0. });
                    self.sound_ops.push(SoundOp::Play { kind: Kind::Voice, file: crate::sound::ogg(&v), fade_s: 0., looping: false });
                }
            }
            "BackLog" => self.open_backlog(opener)?,
            "Config" => self.open_config()?,
            other => self.log.push(format!("不適切なボタンがクリックされました。{other}")),
        }
        Ok(())
    }

    /// `MessageWindowMgr.CancelSkipAndAuto`.
    pub fn cancel_skip_and_auto(&mut self, opener: &mut Opener) -> Result<()> {
        if self.skip_mode {
            self.window_call_event(opener, "Skip")?;
        }
        if self.msgwnd.auto_mode {
            self.window_call_event(opener, "Auto")?;
        }
        Ok(())
    }

    /// `MessageWindowMgr.OpenBacklog` → `BacklogCtrl.CreateBacklog`.
    fn open_backlog(&mut self, opener: &mut Opener) -> Result<()> {
        self.msgwnd.backlog_open = true;
        self.set_main_active(&format!("{WINDOW}/BacklogViewer"), true)?;
        self.set_main_active(&format!("{WINDOW}/MessageViewer"), false)?;
        self.set_main_active(&format!("{WINDOW}/SelectorViewer"), false)?;
        let parent_path = format!("{WINDOW}/BacklogViewer/Contents/BacklogUnitParent");
        let parent = self.main.scene.find(&parent_path).context("BacklogUnitParent")?;
        for c in std::mem::take(&mut self.main.scene.nodes[parent].children) {
            self.main.states.remove(&c);
        }
        let go = self
            .unity
            .resource_of(opener, "SystemUI/MessageWindow/Backlog/Prefabs/BacklogUnit", crysirror_unity::class_id::GAME_OBJECT)?
            .context("ロードされるプレハブが見つかりませんでした。パス=SystemUI/MessageWindow/Backlog/Prefabs/BacklogUnit")?;
        let prefab = self.unity.prefab(opener, &go)?;
        let start = self.backlog.len().saturating_sub(BACKLOG_UNITS);
        for (speaker, msg, voice) in self.backlog[start..].to_vec() {
            let node = self.main.scene.instantiate(&prefab, prefab.roots[0], Some(parent));
            self.main.scene.nodes[node].position = [0.; 3];
            self.main.scene.nodes[node].scale = [1.; 3];
            let speaker = self.replace_chara_name(&speaker);
            let msg = self.replace_chara_name(&msg);
            if let Some(v) = self.main.scene.child(node, "Voice") {
                let st = self.main.states.entry(v).or_default();
                st.active = Some(!voice.is_empty());
                st.click = Some(format!("BacklogCtrl.ClickVoiceButton:{voice}"));
            }
            for (child, text) in [("SpeakerName", speaker), ("Message", msg)] {
                if let Some(l) = self.main.scene.child(node, child) {
                    self.main.states.entry(l).or_default().text = Some(text);
                }
            }
        }
        for (n, p) in crate::ngui::reposition_positions(&self.main.scene, &self.main.states, parent) {
            self.main.states.entry(n).or_default().position = Some(p);
        }
        // AdjustTargetPosInScrollView: ResetPosition.
        if let Some(sv) = self.main.scene.find(&format!("{WINDOW}/BacklogViewer/Contents")) {
            let st = self.main.states.entry(sv).or_default();
            st.scroll_reset = Some(st.scroll_reset.map_or(1, |v| v.wrapping_add(1)));
        }
        Ok(())
    }

    /// `MessageWindowMgr.CloseBacklog`.
    pub fn close_backlog(&mut self) -> Result<()> {
        self.msgwnd.backlog_open = false;
        self.set_main_active(&format!("{WINDOW}/BacklogViewer"), false)?;
        self.set_main_active(&format!("{WINDOW}/MessageViewer"), true)?;
        self.set_main_active(&format!("{WINDOW}/SelectorViewer"), true)?;
        Ok(())
    }

    /// The message window's NGUI events; the rest stay queued.
    pub fn window_events(&mut self, opener: &mut Opener) -> Result<()> {
        use crysirror_core::service::buttons as b;
        let right = self.input.buttons_pressed & b::MOUSE_RIGHT != 0;
        let mut rest = Vec::new();
        for e in std::mem::take(&mut self.events) {
            let (si, node, handler) = &e;
            let (method, arg) = handler.split_once(':').map_or((handler.as_str(), None), |(m, a)| (m, Some(a)));
            match method {
                "MessageWindowMgr.ClickMessageWindowUnderButton" if *si == 0 => {
                    let name = self.main.scene.nodes[*node].name.clone();
                    if !right {
                        self.window_call_event(opener, &name)?;
                    }
                }
                // Left clicks advance through the ADV click below.
                "MessageWindowMgr.ClickMessageBox" | "MessageWindowMgr.ClickBackLogClose" if *si == 0 => {
                    if self.msgwnd.backlog_open && right && !self.cm_system.open {
                        self.close_backlog()?;
                    }
                }
                "BacklogCtrl.ClickVoiceButton" => {
                    if let Some(v) = arg.filter(|v| !v.is_empty()) {
                        self.sound_ops.push(SoundOp::Stop { kind: Kind::Voice, fade_s: 0. });
                        self.sound_ops.push(SoundOp::Play { kind: Kind::Voice, file: crate::sound::ogg(v), fade_s: 0., looping: false });
                    }
                }
                m if m.starts_with("ConfigMgr.") => self.config_event(e.1, m)?,
                _ => rest.push(e),
            }
        }
        self.events = rest;
        if self.msgwnd.backlog_open && right && !self.cm_system.open && !self.click_on_ui {
            self.close_backlog()?;
        }
        Ok(())
    }

    /// `MessageWindowMgr.IsClickAction`.
    pub fn window_click_action(&self) -> bool {
        let open = self.main.scene.find(WINDOW).is_some_and(|n| self.main.states.get(&n).and_then(|s| s.active).unwrap_or(false));
        open && !self.msgwnd.backlog_open && !self.cm_system.open
    }

    /// The ADV click of this frame: a left click (or Enter) not taken by a
    /// widget other than the message box, and the skip/auto advance of
    /// `ADVKagManager.Update`.
    pub fn adv_click(&mut self, opener: &mut Opener) -> Result<bool> {
        use crysirror_core::service::buttons as b;
        let pressed = self.input.buttons_pressed;
        let user = pressed & (b::MOUSE_LEFT | b::ENTER) != 0 && !self.click_on_ui;
        let action = self.window_click_action();
        self.msgwnd.update_count += 1;
        if user && self.skip_mode && action {
            // OnClickEvent while skipping: Skip off.
            self.window_call_event(opener, "Skip")?;
            return Ok(false);
        }
        if self.skip_mode && action {
            return Ok(self.msgwnd.update_count % 2 == 0);
        }
        if self.msgwnd.auto_mode && action {
            // auto_wait_count = (100 − MsgAutoSpeed) × 30 ms after the text
            // is shown; the voice is stopped (MsgVoiceNoStop false).
            if let Some(since) = self.adv.hitret_since() {
                if self.now >= since + ((100 - self.cm_system.auto_speed.clamp(0, 100)) * 30) as u64 {
                    self.sound_ops.push(SoundOp::Stop { kind: Kind::Voice, fade_s: 0. });
                    return Ok(true);
                }
            }
        }
        // Clicks outside the open window (no message box) still advance
        // waits that allow it, as UICamera.isFallThrough does.
        Ok(user && (action || !self.msgwnd.backlog_open && !self.cm_system.open))
    }
}

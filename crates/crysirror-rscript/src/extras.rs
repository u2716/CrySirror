// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! The title and the states reached from it or from button mode 1.
//!
//! Title (sub_440A20): after movies 2 and 1 (sub_441170), sub_4409A0 starts
//! the APINI start script when the top menu is skipped; otherwise a black
//! 640x480 screen shows `.\grps\top_me01`..`top_me07` (CSelBtnEx type 3:
//! two frames stacked vertically, the second while focused) at
//! (54, 223 + 23·i): new game with script 0 and 1, load, configuration,
//! extra menu, new game with script 7000 (shown while variable 7000 is
//! set) and quit. The movies play only once per run (dword_4ED284).
//!
//! Load from the title (sub_432310): `.\grps\bg01` under the load screen;
//! a slot loads after the question (sub_432530), exit returns
//! (sub_432510). Configuration from the title (sub_41CFE0): `confback`
//! under the configuration screen in title mode; its title button writes
//! the system save and returns (sub_41CF70).
use crate::sprite::{Content, Sprite, pt};
use crate::vm::{Vm, Wait};
use anyhow::Result;
use crysirror_core::service::{HostServices, buttons};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    TopMenu,
    TitleLoad,
    TitleConfig,
    CgView,
    SceneView,
    MusicMenu,
    ExtraMenu,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TopAction {
    Start(u16),
    Load,
    Config,
    Extra,
    Quit,
}

pub struct TopMenu {
    pub buttons: Vec<(Sprite, TopAction)>,
    pub hovered: Option<usize>,
}

/// A state other than the game: what is shown and how it ends.
pub struct StateScreen {
    pub state: State,
    /// Black screen (title) or the state's background image.
    pub backdrop: Option<Sprite>,
    pub top: Option<TopMenu>,
    pub gallery: Option<crate::gallery::Gallery>,
}

impl Vm {
    /// sub_4409A0: start or show the top menu.
    pub(crate) fn show_title(&mut self, h: &mut dyn HostServices) -> Result<()> {
        if self.apini.skip_top_menu {
            self.state_screen = None;
            let script = self.apini.start_script;
            return self.new_game(h, script);
        }
        self.enter_state(h, State::TopMenu)
    }
    pub(crate) fn enter_state(&mut self, h: &mut dyn HostServices, state: State) -> Result<()> {
        if self.state_screen.is_none() {
            // Leaving a running game state fades it out (sub_44FCF0).
            let from_game = self.thread.program.is_some();
            self.leave_game(h)?;
            if from_game {
                self.begin_switch_fade(h)?;
            }
        }
        self.save_screen = None;
        self.config_screen = None;
        let sys = self.apini.dir(crate::apini::Dir::System).to_owned();
        let mut screen = StateScreen { state, backdrop: None, top: None, gallery: None };
        match state {
            State::TopMenu => {
                let actions = [
                    TopAction::Start(0),
                    TopAction::Start(1),
                    TopAction::Load,
                    TopAction::Config,
                    TopAction::Extra,
                    TopAction::Start(7000),
                    TopAction::Quit,
                ];
                let mut buttons = Vec::new();
                for (i, action) in actions.into_iter().enumerate() {
                    let Some(c) = self.res.image(h, self.client, &format!(".\\grps\\top_me{:02}", i + 1)) else { continue };
                    let mut s = Sprite { content: Some(c), pos: pt(54, 223 + 23 * i as i32), visible: true, interactive: true, ..Default::default() };
                    s.set_frame(0, 2);
                    if action == TopAction::Start(7000) && self.mem.vars[7000] == 0 {
                        s.visible = false;
                        s.interactive = false;
                    }
                    buttons.push((s, action));
                }
                screen.backdrop = Some(Sprite { content: Some(Content::Fill { color: 0, size: [640, 480] }), visible: true, ..Default::default() });
                screen.top = Some(TopMenu { buttons, hovered: None });
            }
            State::TitleLoad => {
                if let Some(c) = self.res.image(h, self.client, ".\\grps\\bg01") {
                    screen.backdrop = Some(Sprite { content: Some(c), visible: true, ..Default::default() });
                }
                self.state_screen = Some(screen);
                self.open_save_screen(h, false)?;
                self.wait = Some(Wait::State);
                self.dirty = true;
                return Ok(());
            }
            State::TitleConfig => {
                if let Some(c) = self.res.image(h, self.client, &crate::res::join(&sys, "confback")) {
                    screen.backdrop = Some(Sprite { content: Some(c), visible: true, ..Default::default() });
                }
                self.state_screen = Some(screen);
                self.open_config(h, true)?;
                self.wait = Some(Wait::State);
                self.dirty = true;
                return Ok(());
            }
            State::CgView | State::SceneView | State::MusicMenu | State::ExtraMenu => {
                // sub_43FD10 keeps the scene once until the view is left.
                if state == State::SceneView && self.scene_view_saved.is_none() {
                    self.scene_view_saved = Some(self.mem.scene.clone());
                }
                let mut cx = crate::scene::Ctx { h: &mut *h, client: self.client, res: &mut self.res, apini: &self.apini };
                match crate::gallery::Gallery::open(&mut cx, &self.mem, state) {
                    Some(g) => screen.gallery = Some(g),
                    None => anyhow::bail!("RScript {state:?}: its screen layout is missing from this game"),
                }
            }
        }
        self.state_screen = Some(screen);
        self.wait = Some(Wait::State);
        self.dirty = true;
        Ok(())
    }
    /// Message 0x411: back to the title.
    pub(crate) fn leave_state(&mut self, h: &mut dyn HostServices) -> Result<()> {
        self.save_screen = None;
        self.config_screen = None;
        self.state_screen = None;
        self.wait = None;
        self.show_title(h)
    }
    /// Input of a state screen; true when it consumed the frame.
    pub(crate) fn state_input(&mut self, h: &mut dyn HostServices) -> Result<bool> {
        let Some(screen) = self.state_screen.as_ref() else { return Ok(false) };
        match screen.state {
            State::TitleLoad => {
                if self.save_screen.is_none() {
                    self.system_sound(h, 1)?;
                    self.leave_state(h)?;
                }
                return Ok(false);
            }
            State::TitleConfig => {
                if self.config_screen.is_none() {
                    self.write_system(h)?;
                    self.leave_state(h)?;
                }
                return Ok(false);
            }
            State::TopMenu => {}
            _ => return self.gallery_input(h).map(|_| true),
        }
        let input = self.input;
        let pressed = |b: u64| input.buttons_pressed & b != 0;
        let (x, y) = (input.pointer_x, input.pointer_y);
        let Some(top) = self.state_screen.as_mut().and_then(|s| s.top.as_mut()) else { return Ok(true) };
        let hit = top.buttons.iter().position(|(s, _)| s.interactive && s.contains(x, y));
        if hit != top.hovered {
            if let Some(old) = top.hovered {
                top.buttons[old].0.set_frame(0, 2);
            }
            if let Some(new) = hit {
                top.buttons[new].0.set_frame(1, 2);
            }
            top.hovered = hit;
            self.dirty = true;
        }
        if !pressed(buttons::MOUSE_LEFT) {
            return Ok(true);
        }
        let Some(action) = hit.map(|i| top.buttons[i].1) else { return Ok(true) };
        match action {
            // sub_432BC0: the snapshots take the new scene.
            TopAction::Start(script) => {
                self.state_screen = None;
                self.top_menu_start = true;
                self.new_game(h, script)?;
            }
            TopAction::Load => {
                self.system_sound(h, 1)?;
                self.enter_state(h, State::TitleLoad)?;
            }
            TopAction::Config => {
                self.system_sound(h, 1)?;
                self.enter_state(h, State::TitleConfig)?;
            }
            TopAction::Extra => {
                self.system_sound(h, 1)?;
                self.enter_state(h, State::ExtraMenu)?;
            }
            TopAction::Quit => self.quit(h)?,
        }
        Ok(true)
    }
}

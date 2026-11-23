// SPDX-License-Identifier: MPL-2.0
//! `SystemShortcut` of `__GameMain__`: the gear toggles the button base;
//! Config, Exit, Info, screenshots and ToTitle with their system dialogs.
//! Its events are taken before the scene's own (the scene would otherwise
//! consume every click of the frame).
use crate::adapter::{Game, Opener};
use anyhow::Result;

const ROOT: &str = "__GameMain__/SystemUI Root/SystemShortcut";
const DLG: &str = "__GameMain__/SystemUI Root/SystemDialog";
/// `m_aryObjPath` with their handlers.
const BUTTONS: [(&str, &str); 6] = [
    ("Base/Config", "SystemShortcut.OnClick_Config"),
    ("Base/Exit", "SystemShortcut.OnClick_Exit"),
    ("Base/Info", "SystemShortcut.OnClick_Info"),
    ("Base/Ss", "SystemShortcut.OnClick_Ss"),
    ("Base/SsUi", "SystemShortcut.OnClick_SsUi"),
    ("Base/ToTitle", "SystemShortcut.OnClick_ToTitle"),
];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shortcut {
    /// `m_bEnableBase` and its 0.25 s colour tween start.
    base_open: bool,
    tween: Option<(u64, f32, f32)>,
    bound: bool,
}

fn set_color(game: &mut Game, path: &str, c: [f32; 4]) {
    if let Some(n) = game.main.scene.find(path) {
        game.main.states.entry(n).or_default().color = Some(c);
    }
}

fn click(game: &mut Game, path: &str, handler: &str) {
    if let Some(n) = game.main.scene.find(path) {
        game.main.states.entry(n).or_default().click = Some(handler.into());
    }
}

impl Game {
    /// `SystemShortcut.Awake`, once.
    fn bind_shortcut(&mut self) {
        if self.shortcut.bound {
            return;
        }
        self.shortcut.bound = true;
        click(self, &format!("{ROOT}/Gear"), "SystemShortcut.OnClick_Gear");
        for (path, h) in BUTTONS {
            click(self, &format!("{ROOT}/{path}"), h);
        }
        set_color(self, &format!("{ROOT}/Base"), [1., 1., 1., 0.]);
        set_color(self, &format!("{ROOT}/Gear"), [1., 1., 1., 0.3]);
    }

    fn base_toggle(&mut self, open: bool) {
        if self.shortcut.base_open == open {
            return;
        }
        self.shortcut.base_open = open;
        let from = if open { 0. } else { 1. };
        self.shortcut.tween = Some((self.now, from, 1. - from));
        set_color(self, &format!("{ROOT}/Gear"), [1., 1., 1., if open { 1. } else { 0.3 }]);
    }

    fn dialog(&mut self, text: &str, ok: &str, cancel: bool) -> Result<()> {
        self.show_system_dialog(text, ok)?;
        self.set_main_active(&format!("{DLG}/Base/Cancel"), cancel)?;
        click(self, &format!("{DLG}/Base/Cancel"), "SystemShortcut.OnCancel");
        Ok(())
    }

    /// Shortcut and its dialogs' events of this frame; returns whether the
    /// game quits (`ToApplicationQuit`).
    pub fn system_shortcut(&mut self, opener: &mut Opener) -> Result<bool> {
        let _ = opener;
        self.bind_shortcut();
        if let Some((t, from, to)) = self.shortcut.tween {
            let f = ((self.now - t) as f32 / 250.).min(1.);
            set_color(self, &format!("{ROOT}/Base"), [1., 1., 1., from + (to - from) * f]);
            if f >= 1. {
                self.shortcut.tween = None;
            }
        }
        let mut quit = false;
        let in_title = self.level.as_ref().is_some_and(|l| l.name == "SceneTitle");
        let mut rest = Vec::new();
        for e in std::mem::take(&mut self.events) {
            match e.2.as_str() {
                "SystemShortcut.OnClick_Gear" => {
                    let open = !self.shortcut.base_open;
                    self.base_toggle(open);
                }
                "SystemShortcut.OnClick_Exit" => {
                    self.dialog("終了しますか？", "SystemShortcut.OnToEnd", true)?;
                    self.base_toggle(false);
                }
                "SystemShortcut.OnClick_Info" => {
                    self.dialog("カスタムメイド3D 2\nVer. 1.00", "SystemShortcut.OnCancel", false)?;
                    self.base_toggle(false);
                }
                "SystemShortcut.OnClick_ToTitle" => {
                    if in_title {
                        self.dialog("現在タイトルに戻れません。", "SystemShortcut.OnCancel", false)?;
                    } else {
                        self.dialog("タイトルに戻りますか？", "SystemShortcut.OnToTitle", true)?;
                    }
                    self.base_toggle(false);
                }
                "SystemShortcut.OnClick_Config" => {
                    self.open_config()?;
                    self.base_toggle(false);
                }
                // MainCamera.ScreenShot: no capture target in this port.
                "SystemShortcut.OnClick_Ss" | "SystemShortcut.OnClick_SsUi" => self.log.push("SystemShortcut screenshot is not ported".into()),
                "SystemShortcut.OnCancel" => self.set_main_active(DLG, false)?,
                "SystemShortcut.OnToEnd" => {
                    self.set_main_active(DLG, false)?;
                    quit = true;
                }
                "SystemShortcut.OnToTitle" => {
                    self.set_main_active(DLG, false)?;
                    self.fade_to(1., 0.);
                    self.pending_scene = Some("SceneToTitle".into());
                }
                _ => rest.push(e),
            }
        }
        self.events = rest;
        Ok(quit)
    }
}

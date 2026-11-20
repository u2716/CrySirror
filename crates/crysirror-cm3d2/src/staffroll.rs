// SPDX-License-Identifier: MPL-2.0
//! `SceneStaffRoll`: `StaffRollMgr`/`StaffRollCtrl`/`StaffRollAPI` (the
//! credits scroll with fading pictures, then the logo until a click).
use crate::{
    adapter::{Game, Opener},
    kag::AdvUi,
    sound::{Kind, SoundOp},
    unity_scene::ObjectKey,
};
use anyhow::{Context, Result};

const PANEL: &str = "UI Root/StaffRollPanel";
const SCROLL: &str = "UI Root/StaffRollPanel/StaffRollViewer/StaffRollUnitParent";
const TEXTURE: &str = "UI Root/StaffRollPanel/StaffRollViewer/Texture";
/// `SetDurationToFadeIn(2)` / `SetDurationToFadeOut(2)`.
const PANEL_FADE_MS: u64 = 2000;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Viewer {
    StaffRoll,
    Logo,
}

#[derive(Clone, Debug, PartialEq)]
enum Step {
    /// `StaffRollAPI.CoCharaLoad`.
    Loading,
    PanelIn(u64),
    Open,
    PanelOut(u64),
    /// `EndNextScene`: `StopBGM(1)`, `FadeOut(1)`.
    Leaving,
}

/// `StaffRollCtrl.TxetureState` with the tween start time.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Pic {
    None,
    FadeIn(u64, f32),
    Display(u64),
    FadeOut(u64, f32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct StaffRoll {
    step: Step,
    viewer: Viewer,
    pic: Pic,
    textures: Vec<ObjectKey>,
    number: usize,
    /// `TweenPosition` of the scroll: (start, x, from y, to y).
    scroll: Option<(u64, f32, f32)>,
    duration_ms: u64,
    to_y: f32,
    fade_in_ms: u64,
    fade_out_ms: u64,
    display_ms: u64,
}

fn node(game: &Game, path: &str) -> Result<usize> {
    crate::wf::node(game, path)
}

/// `StaffRollMgr.Init` + `OpenStaffRollPanel` → `StaffRollAPI.SceneStart`.
pub fn start(game: &mut Game, opener: &mut Opener) -> Result<StaffRoll> {
    let panel = node(game, PANEL)?;
    let f = game.level.as_ref().and_then(|l| l.scene.script(panel, "StaffRollCtrl")).context("StaffRollCtrl")?.clone();
    let duration = f.int("durationToScroll").unwrap_or(150);
    let from_y = f.int("moveFromPosY").unwrap_or(0) as f32;
    let to_y = f.int("moveToPosY").unwrap_or(0) as f32;
    let fade_in = f.float("fadeInTimeTexure").unwrap_or(1.) as f32;
    let fade_out = f.float("fadeOutTimeTexure").unwrap_or(1.) as f32;
    let textures = game.unity.resources_under(opener, "SceneStaffRoll/Texture", crysirror_unity::class_id::TEXTURE_2D)?;
    anyhow::ensure!(textures.len() > 1, "SceneStaffRoll/Texture has {} textures", textures.len());
    // m_dispTextureTime: integer division of the scroll seconds.
    let display = (duration / textures.len() as i64) as f32 - (fade_in + fade_out);
    let scroll = node(game, SCROLL)?;
    let x = game.level.as_ref().context("staff roll")?.scene.nodes[scroll].position[0];
    crate::wf::state(game, scroll).position = Some([x, from_y, 0.]);
    crate::wf::set_active(game, "UI Root/StaffRollPanel/LogoViewer", false)?;
    crate::wf::set_active(game, PANEL, false)?;
    // StaffRollAPI.SceneStart.
    game.fade_to(1., 0.);
    game.adv_ui(AdvUi::Window(false), opener)?;
    game.adv_ui(AdvUi::LightMain { rotation: None, intensity: None }, opener)?;
    // SATD-cm3d2-rendering: MainCamera.Vignett (Assembly-UnityScript effect).
    game.camera = ([1.713189, 1.198801, -3.111253], 6.499998, [136.2881, 19.62733]);
    game.camera_offset = 0.;
    game.adv_ui(AdvUi::SetBg("Syosai".into()), opener)?;
    game.light.1 = 0.65;
    for c in &mut game.characters {
        c.visible = false;
    }
    Ok(StaffRoll {
        step: Step::Loading,
        viewer: Viewer::StaffRoll,
        pic: Pic::None,
        textures,
        number: 0,
        scroll: None,
        duration_ms: (duration.max(0) as u64) * 1000,
        to_y,
        fade_in_ms: (fade_in * 1000.) as u64,
        fade_out_ms: (fade_out * 1000.) as u64,
        display_ms: (display.max(0.) * 1000.) as u64,
    })
}

fn set_alpha(game: &mut Game, path: &str, a: f32) -> Result<()> {
    let n = node(game, path)?;
    crate::wf::state(game, n).alpha = Some(a);
    Ok(())
}

fn texture_alpha(game: &mut Game) -> Result<f32> {
    let n = node(game, TEXTURE)?;
    let level = game.level.as_ref().context("staff roll")?;
    Ok(level.states.get(&n).and_then(|s| s.alpha).unwrap_or(1.))
}

impl StaffRoll {
    /// `StaffRollMgr.OpenPanel`.
    fn open_panel(&mut self, game: &mut Game) -> Result<()> {
        match self.viewer {
            Viewer::StaffRoll => {
                // PlayAnimation: scroll, the second texture, FadeIn.
                let n = node(game, SCROLL)?;
                let p = game.level.as_ref().and_then(|l| l.states.get(&n)).and_then(|s| s.position).unwrap_or([0.; 3]);
                self.scroll = Some((game.now, p[0], p[1]));
                let t = node(game, TEXTURE)?;
                crate::wf::state(game, t).unity_texture = Some(self.textures[1].clone());
                let a = texture_alpha(game)?;
                self.pic = Pic::FadeIn(game.now, a);
            }
            Viewer::Logo => {
                self.pic = Pic::None;
                crate::wf::set_active(game, "UI Root/StaffRollPanel/StaffRollViewer", false)?;
                crate::wf::set_active(game, "UI Root/StaffRollPanel/LogoViewer", true)?;
            }
        }
        Ok(())
    }

    /// `StaffRollCtrl.Update` and the scroll tween.
    fn update_pictures(&mut self, game: &mut Game) -> Result<()> {
        let now = game.now;
        if let Some((t0, x, from)) = self.scroll {
            let f = if self.duration_ms == 0 { 1. } else { ((now - t0) as f32 / self.duration_ms as f32).min(1.) };
            let n = node(game, SCROLL)?;
            crate::wf::state(game, n).position = Some([x, from + (self.to_y - from) * f, 0.]);
        }
        match self.pic {
            Pic::None => {}
            Pic::FadeIn(t0, from) => {
                let f = if self.fade_in_ms == 0 { 1. } else { ((now - t0) as f32 / self.fade_in_ms as f32).min(1.) };
                set_alpha(game, TEXTURE, from + (1. - from) * f)?;
                if f >= 1. {
                    self.pic = Pic::Display(now + self.display_ms);
                }
            }
            Pic::Display(until) if until < now => self.pic = Pic::FadeOut(now, texture_alpha(game)?),
            Pic::Display(_) => {}
            Pic::FadeOut(t0, from) => {
                let f = if self.fade_out_ms == 0 { 1. } else { ((now - t0) as f32 / self.fade_out_ms as f32).min(1.) };
                set_alpha(game, TEXTURE, from * (1. - f))?;
                if f >= 1. {
                    // Next: GetNextTexture pre-increments from 0, so the
                    // second texture shows twice and the first never.
                    self.number += 1;
                    match self.textures.get(self.number).cloned() {
                        Some(k) => {
                            let t = node(game, TEXTURE)?;
                            crate::wf::state(game, t).unity_texture = Some(k);
                            self.pic = Pic::FadeIn(now, 0.);
                        }
                        None => self.end(),
                    }
                }
            }
        }
        Ok(())
    }

    /// `StaffRollCtrl.End`/`SetEnd` → `CloseViewerAndPanel`.
    fn end(&mut self) {
        self.pic = Pic::None;
        self.scroll = None;
        if self.step == Step::Open {
            self.step = Step::PanelOut(u64::MAX);
        }
    }
}

pub fn update(game: &mut Game, opener: &mut Opener, s: &mut StaffRoll) -> Result<()> {
    use crysirror_core::service::buttons as b;
    let now = game.now;
    let pressed = game.input.buttons_pressed;
    if s.viewer == Viewer::StaffRoll && matches!(s.step, Step::Open | Step::PanelIn(_)) {
        s.update_pictures(game)?;
    }
    match s.step {
        Step::Loading => {
            if game.characters.iter().all(|c| !c.busy()) {
                game.adv_ui(AdvUi::AllResetPos, opener)?;
                game.fade_to(0., 1.);
                game.sound_ops.push(SoundOp::Play { kind: Kind::Bgm, file: "DokiDokiFallinLove_Short.ogg".into(), fade_s: 0., looping: false });
                crate::wf::set_active(game, PANEL, true)?;
                set_alpha(game, PANEL, 0.)?;
                s.open_panel(game)?;
                s.step = Step::PanelIn(now);
            }
        }
        Step::PanelIn(t) => {
            let f = ((now - t) as f32 / PANEL_FADE_MS as f32).min(1.);
            set_alpha(game, PANEL, f)?;
            if f >= 1. {
                s.step = Step::Open;
            }
        }
        Step::Open => {
            // StaffRollMgr.Update: Return skips the roll, a left click
            // closes the logo.
            match s.viewer {
                Viewer::StaffRoll if pressed & b::ENTER != 0 => s.end(),
                Viewer::Logo if pressed & b::MOUSE_LEFT != 0 => s.step = Step::PanelOut(now),
                _ => {}
            }
        }
        Step::PanelOut(u64::MAX) => s.step = Step::PanelOut(now),
        Step::PanelOut(t) => {
            let f = ((now - t) as f32 / PANEL_FADE_MS as f32).min(1.);
            set_alpha(game, PANEL, 1. - f)?;
            if f >= 1. {
                // AfterClose.
                match s.viewer {
                    Viewer::StaffRoll => {
                        s.viewer = Viewer::Logo;
                        s.open_panel(game)?;
                        s.step = Step::PanelIn(now);
                    }
                    Viewer::Logo => {
                        crate::wf::set_active(game, PANEL, false)?;
                        game.sound_ops.push(SoundOp::Stop { kind: Kind::Bgm, fade_s: 1. });
                        game.fade_to(1., 1.);
                        s.step = Step::Leaving;
                    }
                }
            }
        }
        Step::Leaving => {
            if game.fade.done(now) {
                // OnFinishFadeOut: camera reset, SceneMgr.CloseScene.
                game.adv_ui(AdvUi::CameraReset, opener)?;
                crate::daily::close_scene(game, opener, "label")?;
            }
        }
    }
    Ok(())
}

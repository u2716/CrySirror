// SPDX-License-Identifier: MPL-2.0
//! `SceneTrophy`: `SceneTrophyManager` (`Main` and `Move` screens),
//! `ScreenTrophyMain` (trophy list, certification status, total maid
//! points) and `TrophyInfo` units.
use crate::{
    adapter::{Game, Opener},
    kag::AdvUi,
    wf::{self, Child, FadeStatus, Manager, Screens},
};
use anyhow::{Context, Result};

const ROOT: &str = "UI Root/Parent";
const GRID: &str = "UI Root/Parent/List/Content/Parent";
const DARK: [f32; 4] = [0.25, 0.25, 0.25, 1.];
const GREY: [f32; 4] = [0.5, 0.5, 0.5, 1.];

#[derive(Clone, Debug, Default, PartialEq)]
struct State {
    next_label: String,
    wait: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneTrophy {
    wf: Manager,
    s: State,
}

/// `Trophy.GetImageName`.
fn image_name(rarity: i32) -> &'static str {
    match rarity {
        2 => "cm3d2_trophy_trophymark_silver",
        3 => "cm3d2_trophy_trophymark_gold",
        _ => "cm3d2_trophy_trophymark_bronze",
    }
}

/// `SceneTrophyManager.Start` + `ScreenTrophyMain.Awake`.
pub fn start(game: &mut Game, opener: &mut Opener) -> Result<SceneTrophy> {
    let child = |name: &'static str, fade: f32| Child { name, root: ROOT.into(), fade_time: fade, status: FadeStatus::Null };
    let mut wf = Manager::new(game, vec![child("Main", 0.5), child("Move", 0.)]);
    let mut s = State::default();
    let tag = game.adv.tag_backup.clone();
    if tag.get("name").map(String::as_str) == Some("SceneTrophy") {
        s.next_label = tag.get("label").cloned().context("SceneCallにlabelの設定がされていませんでした")?;
    }
    wf::set_click(game, &format!("{ROOT}/OK"), "ScreenTrophyMain.OnClickOK")?;
    wf.call_screen(game, opener, &mut s, "Main")?;
    Ok(SceneTrophy { wf, s })
}

pub fn update(game: &mut Game, opener: &mut Opener, t: &mut SceneTrophy) -> Result<()> {
    t.wf.update(game, opener, &mut t.s)
}

impl Screens for State {
    /// `ScreenTrophyMain.OnCall`.
    fn on_call(&mut self, _wf: &mut Manager, game: &mut Game, opener: &mut Opener, name: &str) -> Result<()> {
        if name != "Main" {
            return Ok(());
        }
        self.wait = 0;
        game.adv_ui(AdvUi::LightMain { rotation: None, intensity: None }, opener)?;
        game.adv_ui(AdvUi::AllResetPos, opener)?;
        game.adv_ui(AdvUi::CameraReset, opener)?;
        let (list, have) = {
            let mut tjs = game.adv.tjs.lock().map_err(|_| anyhow::anyhow!("TJS lock"))?;
            let t = &mut tjs.runtime.host_mut().state;
            let d = t.data()?;
            let have: Vec<i32> = t.player.have_trophy.keys().copied().collect();
            (d.trophies.values().cloned().collect::<Vec<_>>(), have)
        };
        let grid = wf::node(game, GRID)?;
        wf::clear_children(game, grid);
        let (mut total, mut owned, mut points) = (0, 0, 0);
        for tr in &list {
            let unit = wf::create_prefab(game, opener, grid, "SceneTrophy/Prefab/TrophyInfo")?;
            // TrophyInfo.SetData / SetHaveVisible.
            let visible = have.contains(&tr.id);
            let bg = wf::child(game, unit, "BG")?;
            let image = wf::child(game, unit, "TrophyImage/Image")?;
            let title = wf::child(game, unit, "TitleLabel")?;
            let comment = wf::child(game, unit, "CommentParent/CommentLabel")?;
            let item = wf::child(game, unit, "CommentParent/GetItemLabel")?;
            let point = wf::child(game, unit, "MaidPoint/PointLabel")?;
            wf::state(game, point).text = Some(tr.maid_point.to_string());
            wf::state(game, comment).text = Some(tr.info_text.clone());
            wf::state(game, item).text = Some(tr.bonus_text.clone());
            wf::state(game, image).sprite = Some(image_name(tr.rarity).into());
            let (text, color, image_alpha, bg_alpha) = if visible {
                (tr.name.clone(), DARK, 1., 1.)
            } else {
                (if tr.title_mask { "????????".into() } else { tr.name.clone() }, GREY, 0.3, 0.8)
            };
            let st = wf::state(game, title);
            st.text = Some(text);
            st.color = Some(color);
            wf::state(game, point).color = Some(color);
            wf::state(game, image).alpha = Some(image_alpha);
            wf::state(game, bg).alpha = Some(bg_alpha);
            total += 1;
            if visible {
                owned += 1;
                points += tr.maid_point;
            }
        }
        wf::set_text(game, &format!("{ROOT}/TotalInfo/CertificationStatus"), format!("{owned}/{total}"))?;
        wf::set_text(game, &format!("{ROOT}/TotalInfo/TotalMaidPoint"), points.to_string())?;
        wf::reposition(game, grid);
        wf::reset_scroll(game, &format!("{ROOT}/List"))?;
        // TutorialPanel.SetImage(Trophy): the tutorial overlay is not ported
        // (SATD-cm3d2-game-logic).
        Ok(())
    }

    fn is_call_fade_in(&mut self, _wf: &mut Manager, _game: &mut Game, _opener: &mut Opener, name: &str) -> Result<bool> {
        if name != "Main" {
            return Ok(true);
        }
        self.wait += 1;
        Ok(2 < self.wait)
    }

    fn on_finish(&mut self, wf: &mut Manager, game: &mut Game, opener: &mut Opener, name: &str) -> Result<()> {
        match name {
            "Main" => wf.call_screen(game, opener, self, "Move")?,
            "Move" => {
                let label = std::mem::take(&mut self.next_label);
                if !label.is_empty() {
                    let storage = opener.storage.clone();
                    game.adv.jump_label(&game.files, storage.as_ref(), &label)?;
                    game.adv.exec();
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn update(&mut self, wf: &mut Manager, game: &mut Game, opener: &mut Opener, name: &str) -> Result<()> {
        let status = wf.status(name);
        if name == "Move" && status == FadeStatus::Wait {
            wf.finish(game, opener, self, "Move")?;
            return Ok(());
        }
        if name != "Main" || status != FadeStatus::Wait {
            return Ok(());
        }
        for (_, _, handler) in std::mem::take(&mut game.events) {
            match handler.as_str() {
                "ScreenTrophyMain.OnClickOK" => {
                    wf.finish(game, opener, self, "Main")?;
                }
                _ => game.log.push(format!("SceneTrophy event {handler} not ported")),
            }
        }
        Ok(())
    }
}

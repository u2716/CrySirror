// SPDX-License-Identifier: MPL-2.0
//! `SceneCharacterSelect` over the `WfScreenManager`/`WfScreenChildren`
//! flow: the `Main` screen (`CharacterSelectMain` with the
//! `CharacterSelectManager` maid plates) and the `Move` screen
//! (`WfScreenMoveChildren`: jump to the next label).
use crate::{
    adapter::{Game, Opener},
    kag::AdvUi,
    sound::{Kind, SoundOp},
};
use anyhow::{Context, Result};

const GRID: &str = "UI Root/Parent/CharacterSelectPanel/Contents/MaidSkillUnitParent";
const BUTTONS: &str = "UI Root/Parent/ButtonParent";
/// `WfScreenChildren.fade_time_` of `Main`.
const MAIN_FADE_S: f32 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SelectType {
    Communication,
    Yotogi,
    Vip,
    Dance,
}

/// `WfScreenChildren.FadeStatus` of the running screen.
#[derive(Clone, Debug, PartialEq)]
enum Screen {
    /// `Main` called; `IsCallFadeIn` counts frames (`2 < wait_count_`).
    MainFadeInWait(u32),
    MainFadeIn,
    MainWait,
    MainFadeOut,
    /// `FadeEnd`: `OnFadeEnd` waits for the loaded maids, then `Move`.
    MainFadeEnd,
    /// `Move` (fade time 0): `OnFinish` jumps to the label.
    Move,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CharaSelect {
    select_type: SelectType,
    next_label: String,
    cancel_label: String,
    /// Plates: (`MaidPlate` node, its `Button` node, character index).
    plates: Vec<(usize, usize, usize)>,
    selected: Option<usize>,
    loaded: Vec<usize>,
    screen: Screen,
    /// The Yotogi cancel confirmation is open.
    dialog: bool,
}

fn node(game: &Game, path: &str) -> Result<usize> {
    game.level.as_ref().and_then(|l| l.scene.find(path)).with_context(|| format!("character select node {path}"))
}

fn set_active(game: &mut Game, path: &str, on: bool) -> Result<()> {
    let n = node(game, path)?;
    game.level.as_mut().unwrap().states.entry(n).or_default().active = Some(on);
    Ok(())
}

/// `SceneCharacterSelect.Start` + `WfScreenManager.Start` + `CallScreen("Main")`.
pub fn start(game: &mut Game, opener: &mut Opener) -> Result<CharaSelect> {
    game.light = ([50., 0., 0.], 0.95);
    game.adv_ui(AdvUi::AllResetPos, opener)?;
    game.camera_offset = 0.;
    // SATD-cm3d2-game-logic: SoundMgr.VoiceStopAll (no voice channel yet).
    let tag = game.adv.tag_backup.clone();
    let select_type = match tag.get("type").map(String::as_str) {
        Some("Yotogi") => SelectType::Yotogi,
        Some("Vip") => SelectType::Vip,
        Some("Dance") => SelectType::Dance,
        _ => SelectType::Communication,
    };
    let next_label = tag.get("label").cloned().context("SceneCharacterSelectにlabelの設定がされていませんでした")?;
    let cancel_label = tag.get("cancel_label").cloned().unwrap_or_default();
    game.fade_to(1., 0.);
    let mut s = CharaSelect { select_type, next_label, cancel_label, plates: Vec::new(), selected: None, loaded: Vec::new(), screen: Screen::MainFadeInWait(0), dialog: false };
    // CharacterSelectMain.Awake: Cancel hidden until SetCancelLabel.
    set_active(game, &format!("{BUTTONS}/Cancel"), !s.cancel_label.is_empty())?;
    set_active(game, &format!("{BUTTONS}/Tower/ステータス"), false)?;
    for b in ["ステータス", "OK", "Cancel"] {
        let path = if b == "ステータス" { format!("{BUTTONS}/Tower/{b}") } else { format!("{BUTTONS}/{b}") };
        let n = node(game, &path)?;
        game.level.as_mut().unwrap().states.entry(n).or_default().click = Some(format!("CharacterSelectMain.OnClickButton:{b}"));
    }
    on_call(game, opener, &mut s)?;
    Ok(s)
}

/// `CharacterSelectMain.OnCall`.
fn on_call(game: &mut Game, opener: &mut Opener, s: &mut CharaSelect) -> Result<()> {
    for c in &mut game.characters {
        c.visible = false;
    }
    let (bgm, bg, camera): (&str, &str, ([f32; 3], f32, [f32; 2])) = match s.select_type {
        SelectType::Communication => ("BGM015.ogg", "Syosai", ([-2.526511, 1.715126, -2.023701], 7.999995, [-128.2036, -6.149067])),
        SelectType::Yotogi | SelectType::Vip => ("BGM014.ogg", "Salon", ([1.485176, 0.5354422, 1.606312], 13.60001, [-208.1436, -0.869565])),
        SelectType::Dance => ("BGM009.ogg", "Salon", ([0.2325, 0.4275452, 0.1891029], 5.899996, [180., 10.])),
    };
    game.sound_ops.push(SoundOp::Play { kind: Kind::Bgm, file: bgm.into(), fade_s: 0.5, looping: true });
    game.adv_ui(AdvUi::SetBg(bg.into()), opener)?;
    game.camera = camera;
    // The callback lists of OnCall.
    let list: Vec<usize> = {
        let tjs = game.adv.tjs.lock().map_err(|_| anyhow::anyhow!("TJS lock"))?;
        let state = &tjs.runtime.host().state;
        match s.select_type {
            SelectType::Communication => state.communication_maids(),
            SelectType::Yotogi | SelectType::Vip => state.chara_guid_list.iter().filter_map(|g| state.stock.iter().copied().find(|i| state.maids.get(i).is_some_and(|m| &m.guid_id == g))).collect(),
            // CharacterSelectManager.DefaultMaidList: every stock maid.
            SelectType::Dance => state.stock.clone(),
        }
    };
    if s.select_type == SelectType::Dance {
        game.log.push("SATD-cm3d2-game-logic: Dance multi-select (UIWFSwitchPanel) is not ported".into());
    }
    create_plates(game, opener, s, &list)?;
    for c in game.characters.iter_mut().filter(|c| !c.man && c.is_stock()) {
        let sets = c.blend_sets.clone();
        c.face.anime("通常", &sets);
        c.face.blend("無し");
    }
    Ok(())
}

/// `CharacterSelectManager.Create(Select)`: one `MaidPlate` per maid,
/// sorted by `SortMaidStandard`, the first one selected.
fn create_plates(game: &mut Game, opener: &mut Opener, s: &mut CharaSelect, list: &[usize]) -> Result<()> {
    let mut list = list.to_vec();
    let rows: Vec<(usize, crate::params::MaidStatus)> = {
        let tjs = game.adv.tjs.lock().map_err(|_| anyhow::anyhow!("TJS lock"))?;
        let state = &tjs.runtime.host().state;
        state.sort_maid_standard(&mut list);
        list.iter().map(|i| (*i, state.maids.get(i).cloned().unwrap_or_default())).collect()
    };
    let go = game
        .unity
        .resource_of(opener, "CharacterSelect/Prefab/MaidPlate", crysirror_unity::class_id::GAME_OBJECT)?
        .context("Resources CharacterSelect/Prefab/MaidPlate")?;
    let prefab = game.unity.prefab(opener, &go)?;
    let level = game.level.as_mut().context("character select")?;
    let grid = level.scene.find(GRID).context("MaidSkillUnitParent")?;
    for c in std::mem::take(&mut level.scene.nodes[grid].children) {
        level.states.remove(&c);
    }
    s.plates.clear();
    for (i, m) in rows {
        let plate = level.scene.instantiate(&prefab, prefab.roots[0], Some(grid));
        level.scene.nodes[plate].position = [0.; 3];
        level.scene.nodes[plate].scale = [1.; 3];
        fill_level_plate(level, plate, &m);
        // SATD-cm3d2-game-logic: Maid.GetThumIcon (ThumShot thumbnails).
        let button = level.scene.child(plate, "Button").context("MaidPlate/Button")?;
        level.states.entry(button).or_default().click = Some("CharacterSelectManager.OnSelect".into());
        s.plates.push((plate, button, i));
    }
    for (n, p) in crate::ngui::reposition_positions(&level.scene, &level.states, grid) {
        level.states.entry(n).or_default().position = Some(p);
    }
    s.selected = s.plates.first().map(|p| p.2);
    Ok(())
}

/// `MaidPlate.SetMaidData` on a plate of the current level.
pub fn fill_plate(game: &mut Game, plate: usize, m: &crate::params::MaidStatus) -> Result<()> {
    fill_level_plate(game.level.as_mut().context("level")?, plate, m);
    Ok(())
}

fn fill_level_plate(level: &mut crate::adapter::Level, plate: usize, m: &crate::params::MaidStatus) {
    let mut text = |path: &str, t: String| {
        if let Some(n) = level.scene.child(plate, path) {
            level.states.entry(n).or_default().text = Some(t);
        }
    };
    text("LeftPlate/FrameRankingData/RankLabel", if (1..=99).contains(&m.popular_rank) { m.popular_rank.to_string() } else { "-".into() });
    text("LeftPlate/FrameBaseData/TextLastName", m.last_name.clone());
    text("LeftPlate/FrameBaseData/TextFirstName", m.first_name.clone());
    text("LeftPlate/FrameBaseData/TextTypeName", crate::params::enum_text(crate::params::CONTRACT_TEXT, m.contract_type));
    text("RightPlate/RankingBGFrame/TextEvaluation", m.evaluation.to_string());
    text("RightPlate/RankingBGFrame/Frame0/TextTotalEvaluation", m.total_evaluation.to_string());
    text("RightPlate/RankingBGFrame/TextSales", crate::daily::grouped(m.sales));
    text("RightPlate/RankingBGFrame/Frame0/TextTotalSales", crate::daily::grouped(m.total_sales));
    for k in 1..=3 {
        if let Some(n) = level.scene.child(plate, &format!("LeftPlate/FrameRankingData/Rank{k}")) {
            level.states.entry(n).or_default().active = Some(m.popular_rank == k);
        }
    }
    if let Some(n) = level.scene.child(plate, "LeftPlate/FrameBaseData/LeaderPlate") {
        level.states.entry(n).or_default().active = Some(m.leader);
    }
    // The dance order marks show only in the Dance selection.
    for k in 1..=3 {
        if let Some(n) = level.scene.child(plate, &format!("LeftPlate/FrameThumbnail/Select{k}")) {
            level.states.entry(n).or_default().active = Some(false);
        }
    }
}

/// `CharacterSelectMain.OnFinish`; returns whether `Move` is called now.
fn on_finish(game: &mut Game, s: &mut CharaSelect) -> Result<bool> {
    set_active(game, &format!("{BUTTONS}/Cancel"), false)?;
    s.loaded.clear();
    {
        let mut tjs = game.adv.tjs.lock().map_err(|_| anyhow::anyhow!("TJS lock"))?;
        let state = &mut tjs.runtime.host_mut().state;
        match (s.select_type, s.selected) {
            (SelectType::Communication, Some(m)) => {
                s.loaded.push(m);
                let guid = state.maids.get(&m).map(|x| x.guid_id.clone()).unwrap_or_default();
                if let Some(slot) = state.player.schedule_slot.iter_mut().find(|x| x.maid_guid == guid) {
                    slot.communication = true;
                }
                if let Some(other) = state.training_partner(m) {
                    s.loaded.push(other);
                }
            }
            (SelectType::Yotogi | SelectType::Vip, Some(m)) => {
                s.loaded.push(m);
                let guid = state.maids.get(&m).map(|x| x.guid_id.clone()).unwrap_or_default();
                state.chara_guid_list.retain(|g| *g != guid);
            }
            (SelectType::Yotogi | SelectType::Vip, None) => state.chara_guid_list.clear(),
            _ => {}
        }
        state.tmp_flags.insert("ダンス設定人数".into(), 0);
    }
    for (k, m) in s.loaded.clone().into_iter().enumerate() {
        if let Some(slot) = game.active_maids.get_mut(k) {
            *slot = Some(m);
        }
        if let Some(c) = game.characters.get_mut(m) {
            c.visible = true;
        }
    }
    Ok(!s.cancel_label.is_empty() && s.loaded.is_empty())
}

/// `WfScreenManager.Update` + the children's `Update` of this scene.
pub fn update(game: &mut Game, opener: &mut Opener, s: &mut CharaSelect) -> Result<()> {
    let now = game.now;
    match s.screen.clone() {
        Screen::MainFadeInWait(n) => {
            if n + 1 > 2 {
                game.fade_to(0., MAIN_FADE_S);
                s.screen = Screen::MainFadeIn;
            } else {
                s.screen = Screen::MainFadeInWait(n + 1);
            }
        }
        Screen::MainFadeIn => {
            if game.fade.done(now) {
                s.screen = Screen::MainWait;
            }
        }
        Screen::MainWait => {
            for (_, node, handler) in std::mem::take(&mut game.events) {
                match handler.as_str() {
                    "CharacterSelectManager.OnSelect" => {
                        if let Some(p) = s.plates.iter().find(|p| p.1 == node) {
                            s.selected = Some(p.2);
                        }
                    }
                    "CharacterSelectMain.OnClickButton:OK" => {
                        game.fade_to(1., MAIN_FADE_S);
                        s.screen = Screen::MainFadeOut;
                    }
                    "CharacterSelectMain.OnClickButton:Cancel" if s.select_type == SelectType::Yotogi => {
                        game.show_system_dialog("まだ夜伽を実行していないメイドがいます。\n本日の夜伽を終了しますか？", "CharacterSelectMain.CancelOk")?;
                        game.set_main_active("__GameMain__/SystemUI Root/SystemDialog/Base/Cancel", true)?;
                        if let Some(n) = game.main.scene.find("__GameMain__/SystemUI Root/SystemDialog/Base/Cancel") {
                            game.main.states.entry(n).or_default().click = Some("CharacterSelectMain.CancelNo".into());
                        }
                        s.dialog = true;
                    }
                    "CharacterSelectMain.OnClickButton:Cancel" | "CharacterSelectMain.CancelOk" => {
                        if s.dialog {
                            game.set_main_active("__GameMain__/SystemUI Root/SystemDialog", false)?;
                            s.dialog = false;
                        }
                        s.selected = None;
                        s.next_label = s.cancel_label.clone();
                        game.fade_to(1., MAIN_FADE_S);
                        s.screen = Screen::MainFadeOut;
                    }
                    "CharacterSelectMain.CancelNo" => {
                        game.set_main_active("__GameMain__/SystemUI Root/SystemDialog", false)?;
                        s.dialog = false;
                    }
                    other => game.log.push(format!("SceneCharacterSelect event {other} not ported")),
                }
            }
        }
        Screen::MainFadeOut => {
            if game.fade.done(now) {
                let move_now = on_finish(game, s)?;
                s.screen = if move_now { Screen::Move } else { Screen::MainFadeEnd };
            }
        }
        Screen::MainFadeEnd => {
            // OnFadeEnd: once the loaded maids are not busy (loading is
            // synchronous here), CallScreen("Move").
            if !s.loaded.is_empty() {
                s.loaded.clear();
                s.screen = Screen::Move;
            }
        }
        Screen::Move => {
            // WfScreenMoveChildren: FadeIn(0) → Finish → FadeOut(0) → OnFinish.
            game.fade_to(0., 0.);
            if !s.next_label.is_empty() {
                let label = std::mem::take(&mut s.next_label);
                let storage = opener.storage.clone();
                game.adv.jump_label(&game.files, storage.as_ref(), &label)?;
                game.adv.exec();
                game.active = crate::scenes::Active::Adv;
            }
        }
    }
    Ok(())
}

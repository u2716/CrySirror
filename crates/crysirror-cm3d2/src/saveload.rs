// SPDX-License-Identifier: MPL-2.0
//! `SaveAndLoadMgr` / `SaveAndLoadCtrl`: ten pages of ten slots, a
//! `DataUnit` (or `EmptyUnit`) per slot with date, lapsed days, manager,
//! employees and comment; OK/Cancel dialogs, save, load and cancel back to
//! the panel that opened it.
use crate::{
    adapter::{Game, Opener},
    wf,
};
use anyhow::{Context, Result};

const PANEL: &str = "UI Root/SaveAndLoadPanel";
const UNIT_PARENT: &str = "UI Root/SaveAndLoadPanel/DataViewer/DataUnitParent";
/// `BaseCreatePanel` `TweenAlpha` duration.
const FADE_MS: u64 = 500;
const DLG: &str = "__GameMain__/SystemUI Root/SystemDialog";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Save,
    Load,
}

#[derive(Clone, Debug, PartialEq)]
enum Step {
    FadeIn(u64),
    Open,
    /// Closing (`BaseClosePanel`), then reopening the caller's panel.
    FadeOut(u64),
    /// `ClosePanelAfterAction(Deserialize)`.
    Loading(u64, i32),
}

/// What the caller does after this frame.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Running,
    /// Cancelled: the caller reopens its panel.
    Closed,
    /// A save was loaded; the ADV KAG runs from it.
    Loaded,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SaveLoad {
    view: View,
    page: i32,
    step: Step,
    /// Slot unit nodes: (node, serial, empty).
    units: Vec<(usize, i32, bool)>,
    /// `m_saveDataNo` / `m_loadDataNo` waiting for the dialog's OK.
    pending: Option<i32>,
    active: Option<i32>,
    title: bool,
}

/// `GameMain.UpdateSaveDataDay`: the slot with the latest save time.
fn newest(game: &Game, opener: &Opener) -> Option<i32> {
    let mut best: Option<(i64, i32)> = None;
    for slot in 0..100 {
        if let Some(h) = header(game, opener, slot) {
            if let Ok(t) = h.save_time.parse::<i64>() {
                if best.is_none_or(|b| b.0 < t) {
                    best = Some((t, slot));
                }
            }
        }
    }
    let _ = game;
    best.map(|b| b.1)
}

fn header(_game: &Game, opener: &Opener, slot: i32) -> Option<crate::save::Header> {
    let bytes = opener.storage.read(&crate::save::save_path(slot).ok()?).ok()?;
    crate::save::header(&bytes).ok()
}

/// `yyyyMMddHHmmss` → `yyyy.MM.dd HH:mm`.
pub(crate) fn date_text(s: &str) -> String {
    if s.len() == 14 && s.bytes().all(|b| b.is_ascii_digit()) {
        format!("{}.{}.{} {}:{}", &s[0..4], &s[4..6], &s[6..8], &s[8..10], &s[10..12])
    } else {
        s.to_owned()
    }
}

/// `SaveAndLoadMgr.OpenSavePanel` / `OpenLoadPanel` (`title`: the title
/// scene's copy, without the Save/Load switch buttons).
pub fn open(game: &mut Game, opener: &mut Opener, view: View, title: bool) -> Result<SaveLoad> {
    // GetPageNoLatestData: NewSaveDataNo / 10 + 1 (−1 → page 1).
    let page = newest(game, opener).map_or(1, |n| n / 10 + 1).clamp(1, 10);
    let mut s = SaveLoad { view, page, step: Step::FadeIn(game.now), units: Vec::new(), pending: None, active: None, title };
    wf::set_active(game, PANEL, true)?;
    let n = wf::node(game, PANEL)?;
    wf::state(game, n).alpha = Some(0.);
    for p in 1..=10 {
        wf::set_click(game, &format!("{PANEL}/PageGroup/PageButtonParent/Page_{p}"), format!("SaveAndLoadMgr.ClickPageNo:{p}"))?;
    }
    wf::set_click(game, &format!("{PANEL}/Cancel"), "SaveAndLoadMgr.CloseSaveAndLoadPanel")?;
    if title {
        wf::set_active(game, &format!("{PANEL}/DisplayButtonGroup/DisplaySave"), false)?;
        wf::set_active(game, &format!("{PANEL}/DisplayButtonGroup/DisplayLoad"), false)?;
    } else {
        wf::set_click(game, &format!("{PANEL}/DisplayButtonGroup/DisplaySave"), "SaveAndLoadMgr.OpenSavePanel")?;
        wf::set_click(game, &format!("{PANEL}/DisplayButtonGroup/DisplayLoad"), "SaveAndLoadMgr.OpenLoadPanel")?;
    }
    create_viewer(game, opener, &mut s)?;
    Ok(s)
}

/// `SaveAndLoadCtrl.CreateSaveAndLoadDataViewer`.
fn create_viewer(game: &mut Game, opener: &mut Opener, s: &mut SaveLoad) -> Result<()> {
    let parent = wf::node(game, UNIT_PARENT)?;
    wf::clear_children(game, parent);
    s.units.clear();
    let latest = newest(game, opener);
    for serial in (s.page - 1) * 10..s.page * 10 {
        let h = header(game, opener, serial);
        let prefab = if h.is_some() { "SceneDaily/SaveAndLoad/Prefab/DataUnit" } else { "SceneDaily/SaveAndLoad/Prefab/EmptyUnit" };
        let unit = wf::create_prefab(game, opener, parent, prefab)?;
        game.level.as_mut().context("save panel level")?.scene.nodes[unit].name = format!("serialNo_{serial}");
        if let Ok(c) = wf::child(game, unit, "Content/SelectCursor") {
            wf::state(game, c).active = Some(false);
        }
        let st = wf::state(game, unit);
        st.click = Some("SaveAndLoadMgr.ClickSaveOrLoadData".into());
        match &h {
            None => {
                if s.view == View::Load {
                    st.enabled = Some(false);
                }
            }
            Some(h) => {
                let set = |game: &mut Game, path: &str, t: String| -> Result<()> {
                    let n = wf::child(game, unit, path)?;
                    wf::state(game, n).text = Some(t);
                    Ok(())
                };
                if let Ok(n) = wf::child(game, unit, "NewLabel") {
                    wf::state(game, n).active = Some(latest == Some(serial));
                }
                set(game, "Content/DataInfo/Date", date_text(&h.save_time))?;
                set(game, "Content/DataInfo/LapsedDays/Number", h.game_day.to_string())?;
                set(game, "Content/DataInfo/Manager/Name", h.player_name.clone())?;
                set(game, "Content/DataInfo/NumberOfEmployees/Number", h.maid_num.to_string())?;
                set(game, "Content/Comment/InputField", h.comment.clone())?;
            }
        }
        s.units.push((unit, serial, h.is_none()));
    }
    wf::reposition(game, parent);
    // SetSaveUI.
    let save = s.view == View::Save;
    wf::set_active(game, &format!("{PANEL}/TitleGroup/TitleSave"), save)?;
    wf::set_active(game, &format!("{PANEL}/TitleGroup/TitleLoad"), !save)?;
    if !s.title {
        wf::set_active(game, &format!("{PANEL}/DisplayButtonGroup/DisplaySave"), !save)?;
        wf::set_active(game, &format!("{PANEL}/DisplayButtonGroup/DisplayLoad"), save)?;
    }
    // SetPageButtonActive.
    for p in 1..=10 {
        wf::set_active(game, &format!("{PANEL}/PageGroup/PageButtonParent/Page_{p}/SelectCursor"), p == s.page)?;
    }
    Ok(())
}

fn dialog(game: &mut Game, text: &str, ok: &str, cancel: bool) -> Result<()> {
    game.show_system_dialog(text, ok)?;
    game.set_main_active(&format!("{DLG}/Base/Cancel"), cancel)?;
    if cancel {
        if let Some(n) = game.main.scene.find(&format!("{DLG}/Base/Cancel")) {
            game.main.states.entry(n).or_default().click = Some("SaveAndLoadCtrl.CloseDialog".into());
        }
    }
    Ok(())
}

/// Per-frame update; events are taken from `game.events` while open.
pub fn update(game: &mut Game, opener: &mut Opener, s: &mut SaveLoad) -> Result<Outcome> {
    let now = game.now;
    let panel = wf::node(game, PANEL)?;
    match s.step.clone() {
        Step::FadeIn(t) => {
            let f = ((now - t) as f32 / FADE_MS as f32).min(1.);
            wf::state(game, panel).alpha = Some(f);
            if f >= 1. {
                s.step = Step::Open;
            }
        }
        Step::FadeOut(t) | Step::Loading(t, _) => {
            let f = ((now - t) as f32 / FADE_MS as f32).min(1.);
            wf::state(game, panel).alpha = Some(1. - f);
            if f >= 1. {
                wf::set_active(game, PANEL, false)?;
                if let Step::Loading(_, slot) = s.step {
                    game.load_slot(opener, slot)?;
                    return Ok(Outcome::Loaded);
                }
                return Ok(Outcome::Closed);
            }
        }
        Step::Open => {
            for (_, node, handler) in std::mem::take(&mut game.events) {
                let (method, arg) = handler.split_once(':').map_or((handler.as_str(), None), |(m, a)| (m, Some(a)));
                match method {
                    "SaveAndLoadMgr.ClickPageNo" => {
                        let p: i32 = arg.and_then(|a| a.parse().ok()).context("page")?;
                        if p != s.page {
                            s.page = p;
                            create_viewer(game, opener, s)?;
                        }
                    }
                    "SaveAndLoadMgr.OpenSavePanel" if s.view != View::Save => {
                        s.view = View::Save;
                        create_viewer(game, opener, s)?;
                    }
                    "SaveAndLoadMgr.OpenLoadPanel" if s.view != View::Load => {
                        s.view = View::Load;
                        create_viewer(game, opener, s)?;
                    }
                    "SaveAndLoadMgr.CloseSaveAndLoadPanel" => s.step = Step::FadeOut(now),
                    // ClickSaveOrLoadData → SetSaveOrLoadDataActive + SaveAndLoad.
                    "SaveAndLoadMgr.ClickSaveOrLoadData" => {
                        let Some(&(_, serial, empty)) = s.units.iter().find(|u| u.0 == node) else { continue };
                        if s.active == Some(serial) {
                            continue;
                        }
                        for &(n, k, _) in &s.units.clone() {
                            if let Ok(c) = wf::child(game, n, "Content/SelectCursor") {
                                wf::state(game, c).active = Some(k == serial);
                            }
                        }
                        s.active = Some(serial);
                        match s.view {
                            View::Save => {
                                s.pending = Some(serial);
                                dialog(game, if empty { "データを保存しますか？" } else { "上書きして保存しますが宜しいですか？" }, "SaveAndLoadCtrl.SaveData", true)?;
                            }
                            View::Load if !empty => {
                                s.pending = Some(serial);
                                dialog(game, "データをロードしますか？", "SaveAndLoadCtrl.LoadData", true)?;
                            }
                            View::Load => {}
                        }
                    }
                    "SaveAndLoadCtrl.SaveData" => {
                        let serial = s.pending.take().context("save slot")?;
                        // The overwritten slot keeps its comment.
                        let comment = header(game, opener, serial).map(|h| h.comment).unwrap_or_default();
                        game.save_slot(opener, serial, &comment)?;
                        create_viewer(game, opener, s)?;
                        s.active = None;
                        dialog(game, "データが保存されました。", "SaveAndLoadCtrl.CloseDialog", false)?;
                    }
                    "SaveAndLoadCtrl.LoadData" => {
                        game.set_main_active(DLG, false)?;
                        let serial = s.pending.take().context("load slot")?;
                        s.step = Step::Loading(now, serial);
                    }
                    "SaveAndLoadCtrl.CloseDialog" => {
                        game.set_main_active(DLG, false)?;
                        s.pending = None;
                        s.active = None;
                        for &(n, _, _) in &s.units.clone() {
                            if let Ok(c) = wf::child(game, n, "Content/SelectCursor") {
                                wf::state(game, c).active = Some(false);
                            }
                        }
                    }
                    other => game.log.push(format!("SaveAndLoad event {other} not ported")),
                }
            }
        }
    }
    Ok(Outcome::Running)
}

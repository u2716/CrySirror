// SPDX-License-Identifier: MPL-2.0
//! The daily menu's schedule panel: `ScheduleMgr`, `ScheduleCtrl` (six
//! `MaidStatusUnit` rows), `MaidStatusListCtrl` (the `CharacterSelectPanel`
//! of stock maids outside the slots), `DaytimeTaskCtrl` (noon works with
//! level stars and simulated parameter changes) and `NightTaskCtrl` (night
//! works with description and conditions), over `ScheduleScene`.
use crate::{
    adapter::{Game, Opener},
    params::SCHEDULE_SLOTS,
    wf,
};
use anyhow::{Context, Result};

const PANEL: &str = "UI Root/SchedulePanel";
const ROWS: &str = "UI Root/SchedulePanel/SelectedMaidViewer/SelectedMaidUnitParent";
const LIST: &str = "UI Root/SchedulePanel/CharacterSelectPanel";
const DAY: &str = "UI Root/SchedulePanel/DaytimeTaskViewer";
const NIGHT: &str = "UI Root/SchedulePanel/NightTaskViewer";
const DLG: &str = "__GameMain__/SystemUI Root/SystemDialog";
const FADE_MS: u64 = 500;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Column {
    MaidStatus,
    Daytime,
    Night,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Viewer {
    None,
    MaidList,
    Daytime,
    Night,
}

#[derive(Clone, Debug, PartialEq)]
enum Step {
    FadeIn(u64),
    Open,
    FadeOut(u64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScheduleUi {
    step: Step,
    /// `CurrentActiveButton`.
    active: Option<(usize, Column)>,
    viewer: Viewer,
    /// Row nodes: (maid status, daytime, night).
    rows: Vec<(usize, usize, usize)>,
    /// List plates: (button node, character).
    plates: Vec<(usize, usize)>,
    /// Task buttons of the open viewer: (node, work id, enabled).
    tasks: Vec<(usize, i32, bool)>,
    /// Slot waiting for the delete dialog's OK.
    delete: Option<usize>,
}

fn lock<R>(game: &Game, f: impl FnOnce(&mut crate::tjs::TjsState) -> Result<R>) -> Result<R> {
    let mut tjs = game.adv.tjs.lock().map_err(|_| anyhow::anyhow!("TJS lock"))?;
    f(&mut tjs.runtime.host_mut().state)
}

fn text(game: &mut Game, n: usize, path: &str, t: impl Into<String>) -> Result<()> {
    let c = wf::child(game, n, path)?;
    wf::state(game, c).text = Some(t.into());
    Ok(())
}

fn set_active(game: &mut Game, n: usize, path: &str, on: bool) -> Result<()> {
    let c = if path.is_empty() { n } else { wf::child(game, n, path)? };
    wf::state(game, c).active = Some(on);
    Ok(())
}

/// A `TextureBank` icon (`<name>.tex`) on a `UITexture`; none clears it.
fn icon(game: &mut Game, n: usize, name: Option<&str>) {
    match name {
        Some(f) if game.files.exists(&format!("{f}.tex")) => game.pending_textures.push((1, n, format!("{f}.tex"))),
        _ => {
            let st = wf::state(game, n);
            st.texture = None;
            st.alpha = Some(0.);
        }
    }
}

fn sign(v: i64, grouped: bool) -> String {
    let s = if grouped { crate::daily::grouped(v.abs()) } else { v.abs().to_string() };
    match v {
        v if v > 0 => format!("+{s}"),
        v if v < 0 => format!("-{s}"),
        _ => "0".into(),
    }
}

/// `ScheduleMgr.OpenPanel` (after `BaseOpenPanel` shows the panel).
pub fn open(game: &mut Game, opener: &mut Opener) -> Result<ScheduleUi> {
    let mut s = ScheduleUi { step: Step::FadeIn(game.now), active: None, viewer: Viewer::None, rows: Vec::new(), plates: Vec::new(), tasks: Vec::new(), delete: None };
    wf::set_active(game, PANEL, true)?;
    let n = wf::node(game, PANEL)?;
    wf::state(game, n).alpha = Some(0.);
    wf::set_click(game, &format!("{PANEL}/Ok"), "ScheduleMgr.CheckSelectedDataBeforeClosePanel")?;
    create_rows(game, opener, &mut s)?;
    set_viewer(game, &mut s, Viewer::None)?;
    Ok(s)
}

/// `ScheduleCtrl.SetViewerActive`.
fn set_viewer(game: &mut Game, s: &mut ScheduleUi, v: Viewer) -> Result<()> {
    wf::set_active(game, LIST, v == Viewer::MaidList)?;
    wf::set_active(game, DAY, v == Viewer::Daytime)?;
    wf::set_active(game, NIGHT, v == Viewer::Night)?;
    s.viewer = v;
    Ok(())
}

/// `ScheduleCtrl.CreateSelectedMaidViewer` → `SetDataForViewer`.
fn create_rows(game: &mut Game, opener: &mut Opener, s: &mut ScheduleUi) -> Result<()> {
    let parent = wf::node(game, ROWS)?;
    wf::clear_children(game, parent);
    s.rows.clear();
    let d = lock(game, |t| t.data())?;
    let slots: Vec<Option<(usize, crate::params::MaidStatus)>> = lock(game, |t| Ok((0..SCHEDULE_SLOTS).map(|i| t.schedule_slot(i).and_then(|m| t.maids.get(&m).cloned().map(|st| (m, st)))).collect()))?;
    for (i, slot) in slots.iter().enumerate() {
        let unit = wf::create_prefab(game, opener, parent, "SceneDaily/Schedule/Prefab/MaidStatusUnit")?;
        game.level.as_mut().context("schedule level")?.scene.nodes[unit].name = format!("slot_{i}");
        let status = wf::child(game, unit, "MaidStatus")?;
        let day = wf::child(game, unit, "DaytimeTask")?;
        let night = wf::child(game, unit, "NightTask")?;
        match slot {
            None => {
                // SetEmptyMaidStatus.
                for p in ["Ranking", "Figure", "Status"] {
                    set_active(game, status, p, false)?;
                }
                set_active(game, day, "", false)?;
                set_active(game, night, "", false)?;
            }
            Some((_, st)) => {
                let rank = if st.popular_rank == 0 { "-".into() } else { st.popular_rank.to_string() };
                text(game, status, "Ranking/Value", rank)?;
                let crown = match st.popular_rank {
                    1 => Some("cm3d2_management_crown_gold"),
                    2 => Some("cm3d2_management_crown_silver"),
                    3 => Some("cm3d2_management_crown_bronze"),
                    _ => None,
                };
                let ci = wf::child(game, status, "Ranking/Icon")?;
                icon(game, ci, crown);
                text(game, status, "Status/Name/FirstName", st.first_name.clone())?;
                text(game, status, "Status/Name/LastName", st.last_name.clone())?;
                text(game, status, "Status/ContractType/Value", crate::params::enum_text(crate::params::CONTRACT_TEXT, st.contract_type))?;
                let reader = wf::child(game, status, "Status/Reader")?;
                icon(game, reader, st.leader.then_some("cm3d2_management_maidchou"));
                // SATD-cm3d2-game-logic: Figure/Icon is the maid thumbnail (ThumShot).
                let fig = wf::child(game, status, "Figure/Icon")?;
                icon(game, fig, None);
                let noon = d.schedule.noon.get(&st.noon_work_id).map(|w| w.icon.clone());
                icon(game, day, noon.as_deref());
                let nightw = d.schedule.night.get(&st.night_work_id).map(|w| w.icon.clone());
                icon(game, night, nightw.as_deref());
            }
        }
        for (n, col) in [(status, "MaidStatus"), (day, "DaytimeTask"), (night, "NightTask")] {
            set_active(game, n, "SelectCursor", false)?;
            wf::state(game, n).click = Some(format!("ScheduleMgr.Click{col}:{i}"));
        }
        s.rows.push((status, day, night));
    }
    wf::reposition(game, parent);
    if let Some((slot, col)) = s.active {
        cursor(game, s, slot, col, true)?;
    }
    Ok(())
}

fn cursor(game: &mut Game, s: &ScheduleUi, slot: usize, col: Column, on: bool) -> Result<()> {
    let Some(&(a, b, c)) = s.rows.get(slot) else { return Ok(()) };
    let n = match col {
        Column::MaidStatus => a,
        Column::Daytime => b,
        Column::Night => c,
    };
    set_active(game, n, "SelectCursor", on)
}

/// `ScheduleCtrl.SetSelectedRowActive` + `CurrentActiveButton`.
fn select(game: &mut Game, s: &mut ScheduleUi, slot: usize, col: Column) -> Result<()> {
    if let Some((os, oc)) = s.active {
        cursor(game, s, os, oc, false)?;
    }
    cursor(game, s, slot, col, true)?;
    s.active = Some((slot, col));
    Ok(())
}

/// `MaidStatusListCtrl.CreateTaskViewer`: `CharacterSelectManager.Create
/// (Click)` of the stock maids, without those already in a slot.
fn create_list(game: &mut Game, opener: &mut Opener, s: &mut ScheduleUi) -> Result<()> {
    set_viewer(game, s, Viewer::MaidList)?;
    let grid = wf::node(game, &format!("{LIST}/Contents/MaidSkillUnitParent"))?;
    wf::clear_children(game, grid);
    s.plates.clear();
    let rows = lock(game, |t| {
        let mut list = t.stock.clone();
        t.sort_maid_standard(&mut list);
        let in_slot: Vec<usize> = (0..SCHEDULE_SLOTS).filter_map(|i| t.schedule_slot(i)).collect();
        Ok(list.into_iter().filter(|m| !in_slot.contains(m)).map(|m| (m, t.maids.get(&m).cloned().unwrap_or_default())).collect::<Vec<_>>())
    })?;
    for (m, st) in rows {
        let plate = wf::create_prefab(game, opener, grid, "CharacterSelect/Prefab/MaidPlate")?;
        crate::charselect::fill_plate(game, plate, &st)?;
        let button = wf::child(game, plate, "Button")?;
        wf::state(game, button).click = Some("CharacterSelectManager.OnSelect".into());
        s.plates.push((button, m));
    }
    wf::reposition(game, grid);
    wf::reset_scroll(game, &format!("{LIST}/Contents"))?;
    Ok(())
}

/// `DaytimeTaskCtrl.CreateTaskViewer` / `NightTaskCtrl.CreateTaskViewer`.
fn create_tasks(game: &mut Game, opener: &mut Opener, s: &mut ScheduleUi, slot: usize, noon: bool) -> Result<()> {
    set_viewer(game, s, if noon { Viewer::Daytime } else { Viewer::Night })?;
    let root = if noon { DAY } else { NIGHT };
    let parent = wf::node(game, &format!("{root}/TaskViewer/Contents/TaskUnitParent"))?;
    wf::clear_children(game, parent);
    s.tasks.clear();
    let d = lock(game, |t| t.data())?;
    let (maid, current, works) = lock(game, |t| {
        let m = t.schedule_slot(slot).context("schedule slot maid")?;
        let st = t.maids.get(&m).cloned().unwrap_or_default();
        let cur = if noon { st.noon_work_id } else { st.night_work_id };
        let mut works = Vec::new();
        if noon {
            for (id, w) in &d.schedule.noon {
                // NoonWork: Trainer works are hidden; visible &= enabled.
                let enabled = t.enable_noon_work(*id, Some(m));
                if enabled && w.work_type != crate::schedule::NOON_TRAINER_TYPE {
                    works.push((*id, w.name.clone(), w.icon.clone(), enabled, t.noon_work_lv(m, *id)));
                }
            }
        } else {
            for (id, w) in &d.schedule.night {
                if t.visible_night_work(*id) {
                    works.push((*id, w.name.clone(), w.icon.clone(), t.enable_night_work(*id, Some(m), false), 0));
                }
            }
        }
        Ok((m, cur, works))
    })?;
    let prefab = if noon { "SceneDaily/Schedule/Prefab/DaytimeTaskUnit" } else { "SceneDaily/Schedule/Prefab/NightTaskUnit" };
    for (id, name, ic, enabled, lv) in works {
        let unit = wf::create_prefab(game, opener, parent, prefab)?;
        game.level.as_mut().context("schedule level")?.scene.nodes[unit].name = id.to_string();
        text(game, unit, "Title", name)?;
        let main = wf::child(game, unit, "Main")?;
        icon(game, main, Some(&ic));
        if noon {
            for k in 1..=3 {
                set_active(game, unit, &format!("Stars/Icon/Star_{k}"), lv >= k)?;
            }
        }
        set_active(game, main, "SelectCursor", id == current)?;
        if id == current && !enabled {
            wf::state(game, main).color = Some([0.7, 0.7, 0.7, 1.]);
        }
        wf::state(game, main).click = Some(format!("ScheduleMgr.Click{}TaskInTaskViewer", if noon { "Daytime" } else { "Night" }));
        s.tasks.push((main, id, enabled));
    }
    wf::reposition(game, parent);
    wf::reset_scroll(game, &format!("{root}/TaskViewer/Contents"))?;
    if noon {
        noon_parameters(game, maid, current)?;
    } else {
        night_description(game, opener, current)?;
    }
    Ok(())
}

/// `DaytimeTaskCtrl.LoadDaytimeTaskData` + `ExamineDisplayItem`.
fn noon_parameters(game: &mut Game, maid: usize, work: i32) -> Result<()> {
    let (st, r) = lock(game, |t| {
        let mut r = t.noon_result(maid, work, crate::schedule::SUCCESS, false);
        let st = t.maids.get(&maid).cloned().unwrap_or_default();
        r.study_rate /= 10;
        r.teach_rate = (r.teach_rate + st.teach_rate / 1000) / 10;
        Ok((st, r))
    })?;
    let class_exp = st.maid_class.get(st.current_maid_class as usize).map_or(0, |c| c.exp.current);
    let maid_rows: [(&str, i64, i32); 8] = [
        ("StudyRate", r.study_rate.into(), st.a_study_rate() / 10),
        ("Reception", r.reception.into(), st.a_reception()),
        ("Care", r.care.into(), st.a_care()),
        ("Lovely", r.lovely.into(), st.a_lovely()),
        ("Elegance", r.elegance.into(), st.a_elegance()),
        ("Charm", r.charm.into(), st.a_charm()),
        ("TeachRate", r.teach_rate.into(), st.a_teach_rate() / 10),
        ("MaidClassExp", r.exp.into(), class_exp),
    ];
    let base = wf::node(game, &format!("{DAY}/ParameterViewer"))?;
    let mut maid_count = 0;
    for (name, v, cur) in maid_rows {
        let row = wf::child(game, base, &format!("MaidParameter/ParameterParent/{name}"))?;
        text(game, row, "Value", sign(v, false))?;
        text(game, row, "CurrentValue", cur.to_string())?;
        // SetActiveExceptForNothing: rows reading "0" hide.
        set_active(game, row, "", v != 0)?;
        maid_count += usize::from(v != 0);
    }
    let salon_rows: [(&str, i64, bool); 3] = [("Cleanliness", r.cleanliness.into(), false), ("Brilliant", r.gorgeous.into(), false), ("Income", r.income.into(), true)];
    let mut salon_count = 0;
    for (name, v, grouped) in salon_rows {
        let row = wf::child(game, base, &format!("SalonParameter/ParameterParent/{name}"))?;
        text(game, row, "Value", sign(v, grouped))?;
        set_active(game, row, "", v != 0)?;
        salon_count += usize::from(v != 0);
    }
    set_active(game, base, "MaidParameter/Title", maid_count > 0)?;
    set_active(game, base, "SalonParameter/Title", salon_count > 0)?;
    set_active(game, base, "", maid_count + salon_count > 0)?;
    for p in ["MaidParameter/ParameterParent", "SalonParameter/ParameterParent"] {
        let n = wf::child(game, base, p)?;
        wf::reposition(game, n);
    }
    Ok(())
}

/// `NightTaskCtrl.LoadNightTaskData`.
fn night_description(game: &mut Game, opener: &mut Opener, work: i32) -> Result<()> {
    let (info, conds) = lock(game, |t| {
        let d = t.data()?;
        let w = d.schedule.night.get(&work).cloned().unwrap_or_default();
        Ok((w.information.clone(), w.cond_info.iter().map(|c| t.info_replace(work, c)).collect::<Vec<_>>()))
    })?;
    wf::set_text(game, &format!("{NIGHT}/DescriptionViewer/Description/Message"), info)?;
    let parent = wf::node(game, &format!("{NIGHT}/DescriptionViewer/Condition/ConditionParent"))?;
    wf::clear_children(game, parent);
    let mut shown = 0;
    for (k, c) in conds.iter().enumerate() {
        let unit = wf::create_prefab(game, opener, parent, "SceneDaily/Schedule/Prefab/ConditionUnit")?;
        text(game, unit, "No", (k + 1).to_string())?;
        text(game, unit, "Message", c.clone())?;
        set_active(game, unit, "", !c.is_empty())?;
        shown += usize::from(!c.is_empty());
    }
    wf::set_active(game, &format!("{NIGHT}/DescriptionViewer/Condition/Title"), shown > 0)?;
    wf::reposition(game, parent);
    Ok(())
}

/// What the daily scene does after this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Running,
    Closed,
}

pub fn update(game: &mut Game, opener: &mut Opener, s: &mut ScheduleUi) -> Result<Outcome> {
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
        Step::FadeOut(t) => {
            let f = ((now - t) as f32 / FADE_MS as f32).min(1.);
            wf::state(game, panel).alpha = Some(1. - f);
            if f >= 1. {
                wf::set_active(game, PANEL, false)?;
                return Ok(Outcome::Closed);
            }
        }
        Step::Open => {
            let right = game.input.buttons_pressed & crysirror_core::service::buttons::MOUSE_RIGHT != 0;
            for (_, node, handler) in std::mem::take(&mut game.events) {
                let (method, arg) = handler.split_once(':').map_or((handler.as_str(), None), |(m, a)| (m, Some(a)));
                let slot: Option<usize> = arg.and_then(|a| a.parse().ok());
                match method {
                    "ScheduleMgr.ClickMaidStatus" => {
                        let slot = slot.context("slot")?;
                        if right {
                            // CanDeleteData: a filled slot, not the leader when she is the only one.
                            let (filled, leader, count) = lock(game, |t| {
                                let m = t.schedule_slot(slot);
                                Ok((m.is_some(), m.and_then(|m| t.maids.get(&m)).is_some_and(|x| x.leader), (0..SCHEDULE_SLOTS).filter(|i| t.schedule_slot(*i).is_some()).count()))
                            })?;
                            if filled && !(leader && count == 1) {
                                let name = lock(game, |t| Ok(t.schedule_slot(slot).and_then(|m| t.maids.get(&m)).map(|x| format!("{} {}", x.last_name, x.first_name)).unwrap_or_default()))?;
                                game.show_system_dialog(&format!("名前：{name}\nスケジュールから削除して\n宜しいですか。"), "ScheduleCtrl.DeleteMaidAndReDraw")?;
                                game.set_main_active(&format!("{DLG}/Base/Cancel"), true)?;
                                if let Some(n) = game.main.scene.find(&format!("{DLG}/Base/Cancel")) {
                                    game.main.states.entry(n).or_default().click = Some("ScheduleCtrl.CloseDialog".into());
                                }
                                s.delete = Some(slot);
                            }
                        } else if s.active != Some((slot, Column::MaidStatus)) {
                            select(game, s, slot, Column::MaidStatus)?;
                            create_list(game, opener, s)?;
                        }
                    }
                    "ScheduleMgr.ClickDaytimeTask" | "ScheduleMgr.ClickNightTask" => {
                        let slot = slot.context("slot")?;
                        let col = if method.ends_with("DaytimeTask") { Column::Daytime } else { Column::Night };
                        if s.active != Some((slot, col)) {
                            select(game, s, slot, col)?;
                            create_tasks(game, opener, s, slot, col == Column::Daytime)?;
                        }
                    }
                    "CharacterSelectManager.OnSelect" => {
                        // MaidStatusListCtrl.MoveMaidToSlot.
                        let Some(&(_, m)) = s.plates.iter().find(|p| p.0 == node) else { continue };
                        let Some((slot, _)) = s.active else { continue };
                        lock(game, |t| {
                            t.set_slot_safe(slot, Some(m));
                            Ok(())
                        })?;
                        create_rows(game, opener, s)?;
                        create_list(game, opener, s)?;
                    }
                    "ScheduleMgr.ClickDaytimeTaskInTaskViewer" | "ScheduleMgr.ClickNightTaskInTaskViewer" => {
                        let noon = method.contains("Daytime");
                        let Some(&(_, id, enabled)) = s.tasks.iter().find(|t| t.0 == node) else { continue };
                        let Some((slot, _)) = s.active else { continue };
                        if noon {
                            lock(game, |t| {
                                t.set_noon_work_slot_safe(slot, id);
                                Ok(())
                            })?;
                        } else if enabled {
                            lock(game, |t| {
                                t.set_night_work_slot_safe(slot, id);
                                Ok(())
                            })?;
                        } else {
                            // UpdateTaskViewer: show the work without setting it.
                            for &(n, k, en) in &s.tasks.clone() {
                                set_active(game, n, "SelectCursor", k == id)?;
                                wf::state(game, n).color = (k == id && !en).then_some([0.7, 0.7, 0.7, 1.]);
                            }
                            night_description(game, opener, id)?;
                            continue;
                        }
                        create_rows(game, opener, s)?;
                        create_tasks(game, opener, s, slot, noon)?;
                    }
                    "ScheduleCtrl.DeleteMaidAndReDraw" => {
                        game.set_main_active(DLG, false)?;
                        if let Some(slot) = s.delete.take() {
                            lock(game, |t| {
                                t.set_slot_safe(slot, None);
                                Ok(())
                            })?;
                            if s.active.is_some_and(|a| a.0 == slot) {
                                s.active = None;
                                set_viewer(game, s, Viewer::None)?;
                            }
                            create_rows(game, opener, s)?;
                            if s.viewer == Viewer::MaidList {
                                create_list(game, opener, s)?;
                            }
                        }
                    }
                    "ScheduleCtrl.CloseDialog" => {
                        game.set_main_active(DLG, false)?;
                        s.delete = None;
                    }
                    "ScheduleMgr.CheckSelectedDataBeforeClosePanel" => {
                        let any = lock(game, |t| Ok((0..SCHEDULE_SLOTS).any(|i| t.schedule_slot(i).is_some())))?;
                        if any {
                            s.step = Step::FadeOut(now);
                        } else {
                            game.show_system_dialog("スケジュールにキャラクターが\n設定されておりません。", "ScheduleCtrl.CloseDialog")?;
                        }
                    }
                    other => game.log.push(format!("Schedule event {other} not ported")),
                }
            }
        }
    }
    Ok(Outcome::Running)
}

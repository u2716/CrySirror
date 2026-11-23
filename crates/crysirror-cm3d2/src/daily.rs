// SPDX-License-Identifier: MPL-2.0
//! Scenes of `com.workman.cm3d2.scene.dailyEtc.SceneMgr` beyond the title
//! and user edit: `SceneStartDaily` (`StartDailyMgr`/`StartDailyCtrl`/
//! `StartDailyAPI`).
use crate::{
    adapter::{Game, Opener},
    kag::AdvUi,
    sound::{Kind, SoundOp},
};
use anyhow::{Context, Result};

const PANEL: &str = "UI Root/StartDailyPanel";
/// `BasePanelMgr.DURATION_TO_FADE`.
const PANEL_FADE_MS: u64 = 500;

#[derive(Clone, Debug, PartialEq)]
pub enum StartStep {
    /// `StartDailyAPI.CoCharaLoad` → `FadeIn(1)`, then `BaseOpenPanel`.
    Loading,
    /// Panel `TweenAlpha` 0 → 1 since the time given.
    PanelIn(u64),
    /// `m_enableClick`.
    ClickWait,
    /// Panel `TweenAlpha` 1 → 0 since the time given.
    PanelOut(u64),
    /// `EndNextScene`: `StopBGM(1)`, `FadeOut(1)`.
    Leaving,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StartDaily {
    step: StartStep,
}

/// `StartDailyAPI.SceneStart` camera/background pairs.
const START_VIEWS: [([f32; 3], f32, [f32; 2], &str); 7] = [
    ([1.713189, 1.198801, -3.111253], 6.499998, [136.2881, 19.62733], "Syosai"),
    ([-0.6832541, 1.506257, 0.3760189], 3., [146.6462, 19.31676], "Kitchen"),
    ([0.4380804, 0.6998997, -0.3468595], 3.499998, [309.2213, 8.757775], "DressRoom_NoMirror"),
    ([-1.20052, 1.817684, 1.051682], 2., [152.6346, 16.52173], "MaidRoom"),
    ([-2.834785, 2.3657, 0.3462016], 2., [114.3114, 16.21119], "MyBedRoom"),
    ([-6.389582, 0.7459781, 12.50107], 2., [149.0419, 0.06210642], "Salon"),
    ([-0.9598175, 1.245041, -1.976358], 2., [397.8448, 4.099369], "Shitsumu"),
];

fn set_alpha(game: &mut Game, path: &str, a: f32) -> Result<()> {
    let level = game.level.as_mut().context("daily scene")?;
    let n = level.scene.find(path).with_context(|| format!("node {path}"))?;
    level.states.entry(n).or_default().alpha = Some(a);
    Ok(())
}

fn set_active(game: &mut Game, path: &str, on: bool) -> Result<()> {
    let level = game.level.as_mut().context("daily scene")?;
    let n = level.scene.find(path).with_context(|| format!("node {path}"))?;
    level.states.entry(n).or_default().active = Some(on);
    Ok(())
}

/// `StartDailyMgr.Init` + `OpenStartDailyPanel` → `StartDailyAPI.SceneStart`.
pub fn start_daily(game: &mut Game, opener: &mut Opener) -> Result<StartDaily> {
    set_active(game, PANEL, false)?;
    // ExecuteRelatedAPI: ScheduleAPI.DayStartManage, CheckTrophyFromMorning.
    let pick = {
        let mut tjs = game.adv.tjs.lock().map_err(|_| anyhow::anyhow!("TJS lock"))?;
        let state = &mut tjs.runtime.host_mut().state;
        for id in state.day_start_manage()? {
            state.requests.push(crate::tjs::Request::TrophyEffect(id));
        }
        state.random(START_VIEWS.len())
    };
    game.fade_to(1., 0.);
    game.adv_ui(AdvUi::Window(false), opener)?;
    game.light = ([50., 0., 0.], 0.95);
    // SATD-cm3d2-rendering: MainCamera.Vignett (Assembly-UnityScript effect).
    game.camera_offset = 0.;
    let (target, distance, angle, bg) = START_VIEWS[pick];
    game.camera = (target, distance, angle);
    game.adv_ui(AdvUi::SetBg(bg.into()), opener)?;
    for c in &mut game.characters {
        c.visible = false;
    }
    Ok(StartDaily { step: StartStep::Loading })
}

/// `StartDailyMgr.Update` and the panel fades.
pub fn update_start_daily(game: &mut Game, opener: &mut Opener, s: &mut StartDaily) -> Result<()> {
    let now = game.now;
    match s.step.clone() {
        StartStep::Loading => {
            // CoCharaLoad: ResetCharaPosAll, FadeIn(1), BaseOpenPanel.
            game.adv_ui(AdvUi::AllResetPos, opener)?;
            game.fade_to(0., 1.);
            let days = game.adv.tjs.lock().map_err(|_| anyhow::anyhow!("TJS lock"))?.runtime.host().state.player.days;
            create_days_viewer(game, opener, days)?;
            set_active(game, PANEL, true)?;
            set_alpha(game, PANEL, 0.)?;
            s.step = StartStep::PanelIn(now);
        }
        StartStep::PanelIn(t) => {
            let f = ((now - t) as f32 / PANEL_FADE_MS as f32).min(1.);
            set_alpha(game, PANEL, f)?;
            if f >= 1. {
                s.step = StartStep::ClickWait;
            }
        }
        StartStep::ClickWait => {
            // Any key or a left click not over the UI closes the panel.
            if game.any_click() {
                s.step = StartStep::PanelOut(now);
            }
        }
        StartStep::PanelOut(t) => {
            let f = ((now - t) as f32 / PANEL_FADE_MS as f32).min(1.);
            set_alpha(game, PANEL, 1. - f)?;
            if f >= 1. {
                set_active(game, PANEL, false)?;
                game.sound_ops.push(SoundOp::Stop { kind: Kind::Bgm, fade_s: 1. });
                game.fade_to(1., 1.);
                s.step = StartStep::Leaving;
            }
        }
        StartStep::Leaving => {
            if game.fade.done(now) {
                // OnFinishFadeOut: camera reset, SceneMgr.CloseScene.
                game.camera_offset = 0.;
                close_scene(game, opener, "label")?;
            }
        }
    }
    Ok(())
}

/// `StartDailyCtrl.CreateViewer`: one `NumberUnit` per digit.
fn create_days_viewer(game: &mut Game, opener: &mut Opener, days: i32) -> Result<()> {
    let atlas_go = game
        .unity
        .resource_of(opener, "SceneStartDaily/Atlas/AtlasSceneStartDaily", crysirror_unity::class_id::GAME_OBJECT)?
        .context("Resources SceneStartDaily/Atlas/AtlasSceneStartDaily")?;
    let atlas_scene = game.unity.prefab(opener, &atlas_go)?;
    let atlas = atlas_scene.nodes[atlas_scene.roots[0]]
        .components
        .iter()
        .find_map(|(k, c)| matches!(c, crate::unity_scene::Component::Script { class, .. } if class == "UIAtlas").then(|| k.clone()))
        .context("AtlasSceneStartDaily has no UIAtlas")?;
    let unit_go = game
        .unity
        .resource_of(opener, "SceneStartDaily/Prefab/NumberUnit", crysirror_unity::class_id::GAME_OBJECT)?
        .context("Resources SceneStartDaily/Prefab/NumberUnit")?;
    let unit = game.unity.prefab(opener, &unit_go)?;
    let level = game.level.as_mut().context("daily scene")?;
    let parent = level.scene.find(&format!("{PANEL}/DaysUnitParent")).context("DaysUnitParent")?;
    for c in std::mem::take(&mut level.scene.nodes[parent].children) {
        level.states.remove(&c);
    }
    for c in days.to_string().chars() {
        let node = level.scene.instantiate(&unit, unit.roots[0], Some(parent));
        level.scene.nodes[node].name = format!("Num_{c}");
        level.scene.nodes[node].position = [0.; 3];
        level.scene.nodes[node].scale = [1.; 3];
        let st = level.states.entry(node).or_default();
        st.atlas = Some(atlas.clone());
        st.sprite = Some(format!("cmd2_eyecatch_number{c}"));
    }
    for (n, p) in crate::ngui::grid_positions(&level.scene, &level.states, parent) {
        level.states.entry(n).or_default().position = Some(p);
    }
    Ok(())
}

/// `StatusViewer.Start` → `UpdateInfoData` of every `StatusViewer` in the
/// level: salon figures, stock maid count, funds, days and grade stars.
pub fn update_status_viewers(game: &mut Game) -> Result<()> {
    let (clean, beautiful, evaluation, money, days, grade, stock) = {
        let tjs = game.adv.tjs.lock().map_err(|_| anyhow::anyhow!("TJS lock"))?;
        let s = &tjs.runtime.host().state;
        let p = &s.player;
        let stock = game.characters.iter().filter(|c| !c.man && c.is_stock()).count();
        (p.salon_clean, p.salon_beautiful, p.salon_evaluation, p.money, p.days, p.current_salon_grade, stock)
    };
    let Some(level) = game.level.as_mut() else { return Ok(()) };
    let viewers: Vec<usize> = (0..level.scene.nodes.len()).filter(|i| level.scene.script(*i, "StatusViewer").is_some()).collect();
    for v in viewers {
        for (path, text) in [
            ("SalonStatus/Cleanliness/Value", clean.to_string()),
            ("SalonStatus/Splendor/Value", beautiful.to_string()),
            ("SalonStatus/Valuation/Value", evaluation.to_string()),
            ("SalonStatus/NumberOfEmployees/Value", stock.to_string()),
            ("WorkingFunds/Value", grouped(money)),
            ("LapsedDays/Value", days.to_string()),
        ] {
            if let Some(n) = level.scene.child(v, path) {
                level.states.entry(n).or_default().text = Some(text);
            }
        }
        if let Some(group) = level.scene.child(v, "SalonStatus/Rank/StarGroup") {
            for (i, star) in level.scene.nodes[group].children.clone().into_iter().enumerate() {
                level.states.entry(star).or_default().active = Some((i as i32) < grade);
            }
        }
    }
    Ok(())
}

/// `SceneMgr.GoToLabel(labelName)`: jump to the `SceneCall` attribute.
pub fn close_scene(game: &mut Game, opener: &mut Opener, label_name: &str) -> Result<()> {
    let label = game.adv.tag_backup.get(label_name).cloned().with_context(|| format!("SceneCall has no {label_name}"))?;
    let storage = opener.storage.clone();
    game.adv.jump_label(&game.files, storage.as_ref(), &label)?;
    game.adv.exec();
    game.active = crate::scenes::Active::Adv;
    Ok(())
}

/// The `BasePanelMgr` life cycle shared by the `SceneDaily` panels.
#[derive(Clone, Debug, PartialEq)]
pub enum PanelStep {
    /// `*API.SceneStart` done; `CoCharaLoad` next.
    Loading,
    PanelIn(u64),
    Open,
    PanelOut(u64),
    /// `EndNextScene`: waiting for `FadeOut(1)`, then the label.
    Leaving,
    /// `BaseClosePanel(OpenSavePanel/OpenLoadPanel)`: the panel fades out
    /// since the time given, then the save/load panel opens.
    ToSaveLoad(u64, crate::saveload::View),
    /// `SaveAndLoadMgr` is open over the scene.
    SaveLoad(Box<crate::saveload::SaveLoad>),
    /// `BaseClosePanel(ScheduleMgr.BaseOpenPanel)`: fading out since the time.
    ToSchedule(u64),
    Schedule(Box<crate::schedule_ui::ScheduleUi>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum DailyKind {
    /// `DailyMgr` (Daytime when true).
    Menu(bool),
    /// `ResultWorkMgr` (Daytime when true).
    ResultWork(bool),
    ResultIncome,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Daily {
    kind: DailyKind,
    step: PanelStep,
    /// `SceneMgr.GoToLabel` target after the fade-out.
    label: String,
    /// `DailyAPI.EndNextScene(Trophy)` keeps the BGM.
    keep_bgm: bool,
}

/// `DailyMgr.Button` names (their `UIButton.onClick` → `DailyMgr.OnClick`).
const DAILY_BUTTONS: [&str; 7] = ["Save", "Load", "Schedule", "Dance", "MaidManagement", "Shop", "CompetitiveShow"];

impl Daily {
    fn panel(&self) -> &'static str {
        match self.kind {
            DailyKind::Menu(_) => "UI Root/DailyPanel",
            DailyKind::ResultWork(_) => "UI Root/ResultWorkPanel",
            DailyKind::ResultIncome => "UI Root/ResultIncomePanel",
        }
    }
}

/// `"#,0"`.
pub fn grouped(v: i64) -> String {
    let s = v.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if v < 0 { format!("-{out}") } else { out }
}

fn set_text(game: &mut Game, path: &str, text: &str) -> Result<()> {
    let level = game.level.as_mut().context("daily scene")?;
    let n = level.scene.find(path).with_context(|| format!("node {path}"))?;
    level.states.entry(n).or_default().text = Some(text.to_owned());
    Ok(())
}

fn set_click(game: &mut Game, path: &str, handler: &str) -> Result<()> {
    let level = game.level.as_mut().context("daily scene")?;
    let n = level.scene.find(path).with_context(|| format!("node {path}"))?;
    level.states.entry(n).or_default().click = Some(handler.to_owned());
    Ok(())
}

/// `SceneMgr.Start` for `SceneDaily`: `DispatcherSceneDaily(type)`.
pub fn start_daily_scene(game: &mut Game, opener: &mut Opener) -> Result<Daily> {
    let kind = match game.adv.tag_backup.get("type").map(String::as_str) {
        Some("Daytime") => DailyKind::Menu(true),
        Some("Night") => DailyKind::Menu(false),
        Some("ResultWorkDaytime") => DailyKind::ResultWork(true),
        Some("ResultWorkNight") => DailyKind::ResultWork(false),
        Some("ResultIncome") => DailyKind::ResultIncome,
        other => anyhow::bail!("SceneDaily type {other:?} is not ported"),
    };
    let d = Daily { kind: kind.clone(), step: PanelStep::Loading, label: "label".into(), keep_bgm: false };
    for p in ["UI Root/DailyPanel", "UI Root/ResultWorkPanel", "UI Root/ResultIncomePanel"] {
        let _ = set_active(game, p, false);
    }
    game.fade_to(1., 0.);
    game.adv_ui(AdvUi::Window(false), opener)?;
    game.light = ([50., 0., 0.], 0.95);
    game.camera_offset = 0.;
    match kind {
        DailyKind::Menu(day) => {
            // DailyAPI.SceneStart.
            game.camera = ([0.5609447, 1.380762, -1.382336], 1.6, [245.5691, 6.273283]);
            game.sound_ops.push(SoundOp::Play { kind: Kind::Bgm, file: "BGM015.ogg".into(), fade_s: 5., looping: true });
            game.adv_ui(AdvUi::SetBg(if day { "Shitsumu_ChairRot" } else { "Shitsumu_ChairRot_Night" }.into()), opener)?;
            for c in game.characters.iter_mut().filter(|c| !c.man && c.is_stock()) {
                let sets = c.blend_sets.clone();
                c.face.anime("通常", &sets);
                c.face.blend("無し");
            }
        }
        DailyKind::ResultWork(day) => {
            // ResultWorkAPI.SceneStart (SATD-cm3d2-rendering: MainCamera.Blur).
            game.camera = ([0.08306749, 1.299064, 0.0249091], 2.2, [177.9044, 2.546583]);
            game.sound_ops.push(SoundOp::Play { kind: Kind::Bgm, file: "BGM015.ogg".into(), fade_s: 1., looping: true });
            game.adv_ui(AdvUi::SetBg(if day { "Shitsumu" } else { "Shitsumu_Night" }.into()), opener)?;
        }
        DailyKind::ResultIncome => {
            // ResultIncomeAPI.SceneStart.
            game.camera = ([0.08306749, 1.299064, 0.0249091], 2.2, [177.9044, 2.546583]);
            game.sound_ops.push(SoundOp::Play { kind: Kind::Bgm, file: "BGM015.ogg".into(), fade_s: 1., looping: true });
            game.adv_ui(AdvUi::SetBg("Shitsumu_Night".into()), opener)?;
        }
    }
    for c in &mut game.characters {
        c.visible = false;
    }
    Ok(d)
}

/// `OpenPanel` of the active manager.
fn open_panel(game: &mut Game, d: &mut Daily) -> Result<()> {
    match d.kind {
        DailyKind::Menu(day) => {
            // DailyCtrl.DisplayViewer.
            let (shop, competitive) = {
                let tjs = game.adv.tjs.lock().map_err(|_| anyhow::anyhow!("TJS lock"))?;
                let p = &tjs.runtime.host().state.player;
                (2 <= p.scenario_phase, 2 <= p.scenario_phase || p.flag("品評会フラグ") == 1)
            };
            let root = d.panel();
            for b in DAILY_BUTTONS {
                set_click(game, &format!("{root}/DailyButtonGroup/{b}"), &format!("DailyMgr.OnClick:{b}"))?;
            }
            set_click(game, &format!("{root}/Trophy"), "DailyMgr.ClickTrophy")?;
            set_click(game, &format!("{root}/Next"), "DailyMgr.ClickNext")?;
            set_active(game, &format!("{root}/DailyButtonGroup/Schedule"), day)?;
            set_active(game, &format!("{root}/DailyButtonGroup/Dance"), !day)?;
            set_active(game, &format!("{root}/DailyButtonGroup/CompetitiveShow"), day && competitive)?;
            set_active(game, &format!("{root}/DailyButtonGroup/Shop"), shop)?;
            // Dance: IsAvailableDance is always true at night.
            set_active(game, &format!("{root}/DailyButtonGroup/Dance"), !day)?;
            let level = game.level.as_mut().context("daily scene")?;
            let parent = level.scene.find(&format!("{root}/DailyButtonGroup")).context("DailyButtonGroup")?;
            for (n, p) in crate::ngui::reposition_positions(&level.scene, &level.states, parent) {
                level.states.entry(n).or_default().position = Some(p);
            }
        }
        DailyKind::ResultWork(day) => {
            // ResultWorkMgr.DisplayResultDaytime/Night: the WorkResultScene
            // simulation, then the trophies of the result.
            let mut tjs = game.adv.tjs.lock().map_err(|_| anyhow::anyhow!("TJS lock"))?;
            let state = &mut tjs.runtime.host_mut().state;
            if day {
                state.work_result_noon()?;
            } else {
                state.work_result_night()?;
            }
            for id in state.check_trophy_from_work_result(day)? {
                state.requests.push(crate::tjs::Request::TrophyEffect(id));
            }
            drop(tjs);
            let root = d.panel();
            set_click(game, &format!("{root}/Ok"), "ResultWorkMgr.CloseResultWorkPanel")?;
            let _ = set_active(game, &format!("{root}/ResultDaytimeViewer"), day);
            let _ = set_active(game, &format!("{root}/ResultNightViewer"), !day);
            // SATD-cm3d2-game-logic: the per-maid result rows of
            // ResultDaytimeCtrl/ResultNightCtrl.
        }
        DailyKind::ResultIncome => {
            let root = d.panel();
            let mut tjs = game.adv.tjs.lock().map_err(|_| anyhow::anyhow!("TJS lock"))?;
            let state = &mut tjs.runtime.host_mut().state;
            let data = state.data()?;
            // ScheduleAPI.OneDayParamDiff(): slot 0 (the original's search
            // never leaves it), MIDNIGHT − MORNING.
            let diff = state.schedule_slot(0).map(|_| state.backup_params(0, crate::schedule::MIDNIGHT).sub(&state.backup_params(0, crate::schedule::MORNING)));
            let mut texts: Vec<(String, String)> = Vec::new();
            let mut actives: Vec<(String, bool)> = Vec::new();
            let mut income = 0i64;
            if let Some(diff) = diff {
                let mut p = crate::maid_param::PlayerParam { st: &mut state.player, data: &data };
                income = diff.money();
                let ev = diff.get("salon_evaluation");
                texts.push(("SalonEvaluation/ClientEvaluation/Value".into(), (p.st.salon_evaluation - ev).to_string()));
                texts.push(("SalonEvaluation/AcquisitionEvaluation/Value".into(), if ev >= 0 { format!("+{ev}") } else { format!("-{ev}") }));
                texts.push(("SalonEvaluation/Result/Value".into(), p.st.salon_evaluation.to_string()));
                if p.st.salon_loan < 0 {
                    let balance = p.st.salon_loan.abs();
                    let pay = balance.min(income);
                    p.add_salon_loan(pay);
                    p.add_money(-pay);
                    actives.push(("Loan".into(), true));
                    actives.push(("Income".into(), false));
                    texts.push(("Loan/Balance/Value".into(), format!("-{}", grouped(balance))));
                    texts.push(("Loan/ClubIncome/Value".into(), format!("+{}", grouped(pay))));
                    let until = balance - pay;
                    texts.push(("Loan/UntilFullPayment/Value".into(), if until == 0 { "+0".into() } else { format!("-{}", grouped(until)) }));
                } else {
                    actives.push(("Loan".into(), false));
                    actives.push(("Income".into(), true));
                    texts.push(("Income/WorkingFunds/Value".into(), grouped(p.st.money - income)));
                    texts.push(("Income/ClubIncome/Value".into(), format!("+{}", grouped(income))));
                    texts.push(("Income/TotalAmount/Value".into(), grouped(p.st.money)));
                }
                texts.push(("ClubCondition/Cleanliness/BeforeValue".into(), (p.st.salon_clean - diff.get("salon_cleanliness")).to_string()));
                texts.push(("ClubCondition/Cleanliness/AfterValue".into(), p.st.salon_clean.to_string()));
                texts.push(("ClubCondition/Brilliant/BeforeValue".into(), (p.st.salon_beautiful - diff.get("salon_gorgeous")).to_string()));
                texts.push(("ClubCondition/Brilliant/AfterValue".into(), p.st.salon_beautiful.to_string()));
            }
            // CheckTrophyFromDayResult.
            let mut p = crate::maid_param::PlayerParam { st: &mut state.player, data: &data };
            let mut got = Vec::new();
            if ![100, 400].iter().all(|i| p.st.have_trophy.contains(i)) {
                if 5 <= p.st.current_salon_grade && p.add_have_trophy(100) {
                    got.push(100);
                }
                if 1_000_000 <= income as i32 as i64 && p.add_have_trophy(400) {
                    got.push(400);
                }
            }
            for id in got {
                state.requests.push(crate::tjs::Request::TrophyEffect(id));
            }
            drop(tjs);
            for (path, on) in actives {
                set_active(game, &format!("{root}/{path}"), on)?;
            }
            for (path, t) in texts {
                set_text(game, &format!("{root}/{path}"), &t)?;
            }
            set_click(game, &format!("{root}/Ok"), "ResultIncomeMgr.CheckUpGradeSalon")?;
        }
    }
    Ok(())
}

/// Per-frame update of `SceneDaily`.
pub fn update_daily_scene(game: &mut Game, opener: &mut Opener, d: &mut Daily) -> Result<()> {
    let now = game.now;
    let panel = d.panel();
    match d.step.clone() {
        PanelStep::Loading => {
            game.adv_ui(AdvUi::AllResetPos, opener)?;
            game.fade_to(0., 1.);
            open_panel(game, d)?;
            set_active(game, panel, true)?;
            set_alpha(game, panel, 0.)?;
            d.step = PanelStep::PanelIn(now);
        }
        PanelStep::PanelIn(t) => {
            let f = ((now - t) as f32 / PANEL_FADE_MS as f32).min(1.);
            set_alpha(game, panel, f)?;
            if f >= 1. {
                d.step = PanelStep::Open;
            }
        }
        PanelStep::Open => {
            for (si, node, handler) in std::mem::take(&mut game.events) {
                let _ = (si, node);
                let close = |d: &mut Daily, label: &str| {
                    d.label = label.to_owned();
                    d.step = PanelStep::PanelOut(now);
                };
                match handler.as_str() {
                    "DailyMgr.ClickNext" => close(d, "label"),
                    "DailyMgr.ClickTrophy" => {
                        // LoadTrophy is called without closing the panel.
                        d.label = "label_trophy".into();
                        d.keep_bgm = true;
                        game.fade_to(1., 1.);
                        d.step = PanelStep::Leaving;
                    }
                    "DailyMgr.OnClick:Schedule" => d.step = PanelStep::ToSchedule(now),
                    "DailyMgr.OnClick:Save" => d.step = PanelStep::ToSaveLoad(now, crate::saveload::View::Save),
                    "DailyMgr.OnClick:Load" => d.step = PanelStep::ToSaveLoad(now, crate::saveload::View::Load),
                    "DailyMgr.OnClick:MaidManagement" => close(d, "label_maidmanagement"),
                    "DailyMgr.OnClick:Shop" => close(d, "label_shop"),
                    "DailyMgr.OnClick:CompetitiveShow" => close(d, "label_competitiveshow"),
                    "DailyMgr.OnClick:Dance" => close(d, "label_dance"),
                    "ResultWorkMgr.CloseResultWorkPanel" => close(d, "label"),
                    "ResultIncomeMgr.CheckUpGradeSalon" => {
                        let up = {
                            let mut tjs = game.adv.tjs.lock().map_err(|_| anyhow::anyhow!("TJS lock"))?;
                            let state = &mut tjs.runtime.host_mut().state;
                            let data = state.data()?;
                            let mut p = crate::maid_param::PlayerParam { st: &mut state.player, data: &data };
                            p.check_salon_grade_level_up().then(|| (p.st.current_salon_grade, p.is_available_shop()))
                        };
                        match up {
                            // ResultIncomeCtrl.DispLevelUp.
                            Some((grade, shop)) => {
                                let mut text = format!("クラブのグレードが{grade}にあがりました\n新しい夜伽ステージが追加されました");
                                if 3 <= grade {
                                    text += "\n新しい依頼が追加されました";
                                }
                                if shop {
                                    text += "\nショップに新しい商品が追加されました";
                                }
                                game.sound_ops.push(SoundOp::Play { kind: Kind::Se, file: "SE008.ogg".into(), fade_s: 0., looping: false });
                                game.show_system_dialog(&text, "ResultIncomeMgr.CloseResultIncome")?;
                            }
                            None => close(d, "label"),
                        }
                    }
                    "ResultIncomeMgr.CloseResultIncome" => {
                        game.set_main_active("__GameMain__/SystemUI Root/SystemDialog", false)?;
                        close(d, "label");
                    }
                    other => game.log.push(format!("SceneDaily event {other} not ported (SATD-cm3d2-game-logic)")),
                }
            }
        }
        PanelStep::PanelOut(t) => {
            let f = ((now - t) as f32 / PANEL_FADE_MS as f32).min(1.);
            set_alpha(game, panel, 1. - f)?;
            if f >= 1. {
                set_active(game, panel, false)?;
                if !d.keep_bgm {
                    game.sound_ops.push(SoundOp::Stop { kind: Kind::Bgm, fade_s: 1. });
                }
                game.fade_to(1., 1.);
                d.step = PanelStep::Leaving;
            }
        }
        PanelStep::ToSaveLoad(t, view) => {
            let f = ((now - t) as f32 / PANEL_FADE_MS as f32).min(1.);
            set_alpha(game, panel, 1. - f)?;
            if f >= 1. {
                set_active(game, panel, false)?;
                d.step = PanelStep::SaveLoad(Box::new(crate::saveload::open(game, opener, view, false)?));
            }
        }
        PanelStep::ToSchedule(t) => {
            let f = ((now - t) as f32 / PANEL_FADE_MS as f32).min(1.);
            set_alpha(game, panel, 1. - f)?;
            if f >= 1. {
                set_active(game, panel, false)?;
                d.step = PanelStep::Schedule(Box::new(crate::schedule_ui::open(game, opener)?));
            }
        }
        // ScheduleMgr.AfterFadeOut: DailyMgr.BaseOpenPanel(current).
        PanelStep::Schedule(mut s) => match crate::schedule_ui::update(game, opener, &mut s)? {
            crate::schedule_ui::Outcome::Running => d.step = PanelStep::Schedule(s),
            crate::schedule_ui::Outcome::Closed => {
                open_panel(game, d)?;
                set_active(game, panel, true)?;
                set_alpha(game, panel, 0.)?;
                d.step = PanelStep::PanelIn(now);
            }
        },
        PanelStep::SaveLoad(mut s) => match crate::saveload::update(game, opener, &mut s)? {
            crate::saveload::Outcome::Running => d.step = PanelStep::SaveLoad(s),
            // SaveAndLoadMgr.AfterFadeOut: DailyMgr.BaseOpenPanel(current).
            crate::saveload::Outcome::Closed => {
                open_panel(game, d)?;
                set_active(game, panel, true)?;
                set_alpha(game, panel, 0.)?;
                d.step = PanelStep::PanelIn(now);
            }
            // The loaded script continues; its SceneCall replaces this scene.
            crate::saveload::Outcome::Loaded => d.step = PanelStep::Open,
        },
        PanelStep::Leaving => {
            if game.fade.done(now) {
                game.camera_offset = 0.;
                for c in &mut game.characters {
                    c.visible = false;
                }
                let label = d.label.clone();
                close_scene(game, opener, &label)?;
            }
        }
    }
    Ok(())
}

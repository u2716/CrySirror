// SPDX-License-Identifier: MPL-2.0
//! `SceneMaidManagement`: `MaidManagement` (`Main` and `Move` screens),
//! `MaidManagementMain` (maid plates, employment, dismissal, leader, edit,
//! ranking), `ClassChangePanel`/`ClassUnit` and the `StatusCtrl` panel.
use crate::{
    adapter::{Game, Opener},
    kag::AdvUi,
    params::*,
    sound::{Kind, SoundOp},
    wf::{self, Child, FadeStatus, Manager, Screens},
};
use anyhow::{Context, Result};

const ROOT: &str = "UI Root/Parent";
const GRID: &str = "UI Root/Parent/ButtonParent/Tower/Grid";
const PLATES: &str = "UI Root/Parent/CharacterSelectPanel/Contents/MaidSkillUnitParent";
const CLASS: &str = "UI Root/Parent/ClassChangePanel";
const STATUS: &str = "UI Root/Parent/StatusPanel";
const DLG: &str = "__GameMain__/SystemUI Root/SystemDialog";
/// `MaidManagementMain.GetEmploymentPrice`.
const EMPLOYMENT_PRICE: i64 = 3_000_000;
/// `StatusAccess.banishment_price`.
const BANISHMENT_PRICE: i64 = 10_000;
const BUTTONS: [&str; 6] = ["雇用", "解雇", "メイド長", "エディット", "ランキング", "クラス変更"];

#[derive(Clone, Debug, Default, PartialEq)]
struct State {
    edit_label: String,
    next_label: String,
    class_only: bool,
    night: bool,
    ranking: bool,
    selected: Option<usize>,
    plates: Vec<(usize, usize)>,
    /// ClassUnit buttons: (node, yotogi list, class).
    units: Vec<(usize, bool, i32)>,
    class_open: bool,
    status_open: bool,
    wait: u32,
    dialog: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaidManagement {
    wf: Manager,
    s: State,
}

fn lock<R>(game: &Game, f: impl FnOnce(&mut crate::tjs::TjsState) -> Result<R>) -> Result<R> {
    let mut tjs = game.adv.tjs.lock().map_err(|_| anyhow::anyhow!("TJS lock"))?;
    f(&mut tjs.runtime.host_mut().state)
}

fn button(game: &mut Game, name: &str) -> Result<usize> {
    if name == "ステータス" {
        return wf::node(game, &format!("{ROOT}/ButtonParent/Tower/ステータス"));
    }
    wf::node(game, &format!("{GRID}/{name}"))
}

/// `MaidManagement.Start` + `MaidManagementMain.Awake`.
pub fn start(game: &mut Game, opener: &mut Opener) -> Result<MaidManagement> {
    let child = |name: &'static str, fade: f32| Child { name, root: ROOT.into(), fade_time: fade, status: FadeStatus::Null };
    let mut wf = Manager::new(game, vec![child("Main", 0.5), child("Move", 0.)]);
    let tag = game.adv.tag_backup.clone();
    let mut s = State::default();
    if tag.get("name").map(String::as_str) == Some("SceneMaidManagement") {
        s.next_label = tag.get("label").cloned().context("SceneCallにlabelの設定がされていませんでした")?;
        if tag.get("type").map(String::as_str) == Some("class_change") {
            s.class_only = true;
        } else {
            s.edit_label = tag.get("edit_label").cloned().context("SceneCallにedit_labelの設定がされていませんでした")?;
        }
        if !tag.contains_key("loop") {
            game.maid_management_backup = None;
        }
    }
    for b in BUTTONS.iter().chain(["ステータス"].iter()) {
        let n = button(game, b)?;
        wf::state(game, n).click = Some(format!("MaidManagementMain.OnClickButton:{b}"));
    }
    wf::set_click(game, &format!("{ROOT}/ButtonParent/Cancel"), "MaidManagementMain.OnClickButton:Cancel")?;
    wf::set_click(game, &format!("{STATUS}/Ok"), "MaidManagementMain.OnClickButton:ステータス")?;
    if s.class_only {
        for b in ["雇用", "解雇", "メイド長", "エディット", "ランキング"] {
            let n = button(game, b)?;
            wf::state(game, n).active = Some(false);
        }
    }
    class_awake(game, opener, &mut s)?;
    wf.call_screen(game, opener, &mut s, "Main")?;
    Ok(MaidManagement { wf, s })
}

pub fn update(game: &mut Game, opener: &mut Opener, m: &mut MaidManagement) -> Result<()> {
    m.wf.update(game, opener, &mut m.s)
}

impl Screens for State {
    fn on_call(&mut self, _wf: &mut Manager, game: &mut Game, opener: &mut Opener, name: &str) -> Result<()> {
        if name != "Main" {
            return Ok(());
        }
        self.wait = 0;
        let (night, employ, class_change, ranking, stock) = lock(game, |t| {
            let d = t.data()?;
            let p = crate::maid_param::PlayerParam { st: &mut t.player, data: &d };
            Ok((p.st.flag("時間帯") == 3, p.is_available_employment(), p.is_available_class_change(), p.is_available_ranking(), t.stock.len()))
        })?;
        self.night = night;
        game.adv_ui(AdvUi::LightMain { rotation: None, intensity: None }, opener)?;
        game.adv_ui(AdvUi::AllResetPos, opener)?;
        game.adv_ui(AdvUi::CameraReset, opener)?;
        game.sound_ops.push(SoundOp::Stop { kind: Kind::Voice, fade_s: 0. });
        game.sound_ops.push(SoundOp::Play { kind: Kind::Bgm, file: "BGM017.ogg".into(), fade_s: 0.5, looping: true });
        if self.class_only {
            game.adv_ui(AdvUi::SetBg("Salon".into()), opener)?;
            self.set_class_panel(game, true)?;
        } else {
            game.adv_ui(AdvUi::SetBg(if night { "Shitsumu_Night" } else { "Shitsumu" }.into()), opener)?;
            for (b, on) in [("雇用", employ), ("解雇", employ), ("クラス変更", class_change), ("ランキング", ranking)] {
                let n = button(game, b)?;
                wf::state(game, n).active = Some(on);
            }
            self.set_class_panel(game, false)?;
        }
        self.selected = None;
        create_plates(game, opener, self)?;
        let backup = game.maid_management_backup.take();
        let pick = lock(game, |t| Ok(backup.as_ref().and_then(|g| t.stock.iter().copied().find(|i| t.maids.get(i).is_some_and(|m| &m.guid_id == g)))))?;
        if let Some(m) = pick.or_else(|| self.plates.first().map(|p| p.1)) {
            self.on_select(game, m)?;
        }
        if self.class_only {
            let n = button(game, "クラス変更")?;
            wf::state(game, n).enabled = Some(false);
        }
        let n = button(game, "雇用")?;
        wf::state(game, n).enabled = Some(stock < 99);
        if night {
            for b in ["雇用", "解雇", "クラス変更", "メイド長"] {
                let n = button(game, b)?;
                wf::state(game, n).active = Some(false);
            }
        }
        wf::reposition(game, wf::node(game, GRID)?);
        for c in &mut game.characters {
            c.visible = false;
        }
        wf::set_active(game, STATUS, false)?;
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
            "Main" => {
                if self.next_label == self.edit_label {
                    if let Some(c) = game.active_maids[0].and_then(|i| game.characters.get_mut(i)) {
                        c.visible = true;
                    }
                }
                if !self.next_label.is_empty() {
                    wf.call_screen(game, opener, self, "Move")?;
                }
            }
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
        for (_, node, handler) in std::mem::take(&mut game.events) {
            let (method, arg) = handler.split_once(':').map_or((handler.as_str(), None), |(m, a)| (m, Some(a)));
            match (method, arg) {
                ("CharacterSelectManager.OnSelect", _) => {
                    if let Some(&(_, m)) = self.plates.iter().find(|p| p.0 == node) {
                        self.on_select(game, m)?;
                    }
                }
                ("MaidManagementMain.OnClickButton", Some(b)) => self.on_click(wf, game, opener, b)?,
                ("ClassUnit.OnSelect", _) => {
                    let Some(&(_, yotogi, class)) = self.units.iter().find(|u| u.0 == node) else { continue };
                    let Some(m) = self.selected else { continue };
                    lock(game, |t| {
                        if let Some(st) = t.maids.get_mut(&m) {
                            if yotogi { st.current_yotogi_class = class } else { st.current_maid_class = class }
                        }
                        Ok(())
                    })?;
                    update_class_panel(game, self, m, Some((yotogi, class)))?;
                }
                ("ClassChangePanel.OnSelectType", Some(t)) => {
                    let yotogi = t == "yotogi";
                    wf::set_active(game, &format!("{CLASS}/ClassViewer/Common/ClassListMaid"), !yotogi)?;
                    wf::set_active(game, &format!("{CLASS}/ClassViewer/Common/ClassListYotogi"), yotogi)?;
                    if let Some(m) = self.selected {
                        let class = lock(game, |t| Ok(t.maids.get(&m).map_or(0, |s| if yotogi { s.current_yotogi_class } else { s.current_maid_class })))?;
                        update_class_panel(game, self, m, Some((yotogi, class)))?;
                    }
                }
                ("MaidManagementMain.Employment", _) => {
                    game.set_main_active(DLG, false)?;
                    self.employment(wf, game, opener)?;
                }
                ("MaidManagementMain.BanishmentMaid", _) => {
                    game.set_main_active(DLG, false)?;
                    self.banishment(game, opener)?;
                }
                ("MaidManagementMain.CloseDialog", _) => game.set_main_active(DLG, false)?,
                _ => game.log.push(format!("SceneMaidManagement event {handler} not ported")),
            }
        }
        Ok(())
    }
}

impl State {
    /// `MaidManagementMain.OnSelectChara`.
    fn on_select(&mut self, game: &mut Game, m: usize) -> Result<()> {
        self.selected = Some(m);
        let st = lock(game, |t| Ok(t.maids.get(&m).cloned().unwrap_or_default()))?;
        let leader_btn = button(game, "メイド長")?;
        wf::state(game, leader_btn).enabled = Some(!st.leader && st.contract_type != 0);
        let fire = button(game, "解雇")?;
        wf::state(game, fire).enabled = Some(!st.leader);
        for &(p, k) in &self.plates.clone() {
            // UIWFTabButton selection frame of the plate (`Button` is a child).
            let plate = game.level.as_ref().unwrap().scene.nodes[p].parent.unwrap_or(p);
            if let Ok(f) = wf::child(game, plate, "LeftPlate/SelectFrame") {
                wf::state(game, f).alpha = Some(if k == m { 1. } else { 0. });
            }
        }
        update_class_panel(game, self, m, None)?;
        if self.status_open {
            fill_status(game, m)?;
        }
        Ok(())
    }

    fn dialog(&mut self, game: &mut Game, text: &str, ok: &str, cancel: bool) -> Result<()> {
        game.show_system_dialog(text, ok)?;
        game.set_main_active(&format!("{DLG}/Base/Cancel"), cancel)?;
        if let Some(n) = game.main.scene.find(&format!("{DLG}/Base/Cancel")) {
            game.main.states.entry(n).or_default().click = Some("MaidManagementMain.CloseDialog".into());
        }
        Ok(())
    }

    /// `MaidManagementMain.OnClickButton`.
    fn on_click(&mut self, wf: &mut Manager, game: &mut Game, opener: &mut Opener, b: &str) -> Result<()> {
        let (money, stock) = lock(game, |t| Ok((t.player.money, t.stock.len())))?;
        match b {
            "雇用" if stock < 99 => {
                if !self.edit_label.is_empty() {
                    let price = crate::daily::grouped(EMPLOYMENT_PRICE);
                    if money - EMPLOYMENT_PRICE < 0 {
                        self.dialog(game, &format!("新しいメイドを雇用するには\n{price}CRが必要です。"), "MaidManagementMain.CloseDialog", false)?;
                    } else {
                        self.dialog(game, &format!("新しいメイドを雇用しますか?\n資金 -{price}CR"), "MaidManagementMain.Employment", true)?;
                    }
                }
            }
            "解雇" => {
                let Some(m) = self.selected else { return Ok(()) };
                let st = lock(game, |t| Ok(t.maids.get(&m).cloned().unwrap_or_default()))?;
                if 2 <= stock && !st.leader {
                    let text = format!("{} {}\nを解雇しますか?\n資金 +{}CR", st.last_name, st.first_name, crate::daily::grouped(BANISHMENT_PRICE));
                    self.dialog(game, &text, "MaidManagementMain.BanishmentMaid", true)?;
                }
            }
            "メイド長" => {
                let Some(m) = self.selected else { return Ok(()) };
                let changed = lock(game, |t| {
                    if t.maids.get(&m).is_some_and(|s| s.leader) {
                        return Ok(false);
                    }
                    for i in t.stock.clone() {
                        if let Some(s) = t.maids.get_mut(&i) {
                            s.leader = i == m;
                        }
                    }
                    t.schedule_error_all_check();
                    Ok(true)
                })?;
                if changed {
                    create_plates(game, opener, self)?;
                    self.on_select(game, m)?;
                }
            }
            "ランキング" => self.change_ranking(game, opener)?,
            "クラス変更" => {
                let open = !self.class_open;
                self.set_class_panel(game, open)?;
                if self.class_only {
                    let c = button(game, "クラス変更")?;
                    wf::state(game, c).enabled = Some(false);
                    let s = button(game, "ステータス")?;
                    wf::state(game, s).enabled = Some(true);
                }
            }
            "エディット" => {
                let Some(m) = self.selected else { return Ok(()) };
                if !self.edit_label.is_empty() {
                    game.active_maids[0] = Some(m);
                    game.maid_management_backup = lock(game, |t| Ok(t.maids.get(&m).map(|s| s.guid_id.clone())))?;
                    self.next_label = self.edit_label.clone();
                    wf.finish(game, opener, self, "Main")?;
                }
            }
            "ステータス" => {
                let Some(m) = self.selected else { return Ok(()) };
                fill_status(game, m)?;
                let open = !self.status_open;
                self.set_status_panel(game, open)?;
                if self.class_only {
                    let c = button(game, "クラス変更")?;
                    wf::state(game, c).enabled = Some(true);
                    let s = button(game, "ステータス")?;
                    wf::state(game, s).enabled = Some(false);
                }
            }
            "Cancel" => {
                wf.finish(game, opener, self, "Main")?;
            }
            _ => {}
        }
        Ok(())
    }

    /// `MaidManagementMain.Employment`: `AddStockMaid` then the edit label.
    fn employment(&mut self, wf: &mut Manager, game: &mut Game, opener: &mut Opener) -> Result<()> {
        let index = game.characters.len();
        let guid = lock(game, |t| {
            let d = t.data()?;
            crate::maid_param::PlayerParam { st: &mut t.player, data: &d }.add_money(-EMPLOYMENT_PRICE);
            t.init_maid(index, None, crate::adapter::create_time())?;
            t.requests.push(crate::tjs::Request::CreateMaid(index));
            Ok(t.maids.get(&index).map(|s| s.guid_id.clone()))
        })?;
        game.run_tjs_requests(opener)?;
        game.maid_management_backup = guid;
        game.active_maids[0] = Some(index);
        self.next_label = self.edit_label.clone();
        wf.finish(game, opener, self, "Main")?;
        Ok(())
    }

    /// `MaidManagementMain.BanishmentMaid` (`CharacterMgr.BanishmentMaid`).
    fn banishment(&mut self, game: &mut Game, opener: &mut Opener) -> Result<()> {
        let Some(m) = self.selected.take() else { return Ok(()) };
        let st = lock(game, |t| Ok(t.maids.get(&m).cloned().unwrap_or_default()))?;
        if let Some(c) = game.characters.get_mut(m) {
            c.banished = true;
            c.visible = false;
        }
        for s in game.active_maids.iter_mut() {
            if *s == Some(m) {
                *s = None;
            }
        }
        game.mirror_tjs()?;
        lock(game, |t| {
            let d = t.data()?;
            crate::maid_param::PlayerParam { st: &mut t.player, data: &d }.add_money(BANISHMENT_PRICE);
            t.schedule_error_all_check();
            Ok(())
        })?;
        create_plates(game, opener, self)?;
        let stock = lock(game, |t| Ok(t.stock.len()))?;
        let n = button(game, "雇用")?;
        wf::state(game, n).enabled = Some(stock < 99);
        if let Some(first) = self.plates.first().map(|p| p.1) {
            self.on_select(game, first)?;
        }
        self.dialog(game, &format!("{} {}\nを解雇しました", st.last_name, st.first_name), "MaidManagementMain.CloseDialog", false)
    }

    /// `ChangeRankingMode`: ranking order and the plates' right panel.
    fn change_ranking(&mut self, game: &mut Game, opener: &mut Opener) -> Result<()> {
        if !self.ranking {
            if self.class_open {
                self.set_class_panel(game, false)?;
            }
            self.set_status_panel(game, false)?;
        }
        self.ranking = !self.ranking;
        let tower = wf::node(game, &format!("{ROOT}/ButtonParent/Tower"))?;
        wf::state(game, tower).position = Some(if self.ranking { [265., 0., 0.] } else { [0.; 3] });
        create_plates(game, opener, self)
    }

    fn set_class_panel(&mut self, game: &mut Game, open: bool) -> Result<()> {
        if self.ranking && open {
            self.ranking = false;
        }
        if open {
            self.set_status_panel(game, false)?;
        }
        self.class_open = open;
        wf::set_active(game, CLASS, open)?;
        // BigThumbnail (maid thumbnail) shows when no panel covers it.
        wf::set_active(game, &format!("{ROOT}/CharacterSelectPanel/BigThumbnail"), !open && !self.class_only)?;
        Ok(())
    }

    fn set_status_panel(&mut self, game: &mut Game, open: bool) -> Result<()> {
        if open {
            if self.ranking {
                self.ranking = false;
            }
            if self.class_open {
                self.set_class_panel(game, false)?;
            }
        }
        self.status_open = open;
        wf::set_active(game, STATUS, open)?;
        wf::set_active(game, &format!("{ROOT}/CharacterSelectPanel/BigThumbnail"), !open && !self.class_only && !self.class_open)?;
        Ok(())
    }
}

/// `CharacterSelectManager.Create(Select)` with the normal or ranking sort.
fn create_plates(game: &mut Game, opener: &mut Opener, s: &mut State) -> Result<()> {
    let grid = wf::node(game, PLATES)?;
    wf::clear_children(game, grid);
    s.plates.clear();
    let ranking = s.ranking;
    let rows = lock(game, |t| {
        let mut list = t.stock.clone();
        if ranking {
            // SortType.Ranking: popular rank, unranked last.
            list.sort_by_key(|i| t.maids.get(i).map_or(i32::MAX, |m| if m.popular_rank == 0 { i32::MAX } else { m.popular_rank }));
        } else {
            t.sort_maid_standard(&mut list);
        }
        Ok(list.into_iter().map(|m| (m, t.maids.get(&m).cloned().unwrap_or_default())).collect::<Vec<_>>())
    })?;
    for (m, st) in rows {
        let plate = wf::create_prefab(game, opener, grid, "CharacterSelect/Prefab/MaidPlate")?;
        crate::charselect::fill_plate(game, plate, &st)?;
        if let Ok(r) = wf::child(game, plate, "RightPlate") {
            wf::state(game, r).active = Some(ranking);
        }
        let button = wf::child(game, plate, "Button")?;
        wf::state(game, button).click = Some("CharacterSelectManager.OnSelect".into());
        s.plates.push((button, m));
    }
    wf::reposition(game, grid);
    wf::reset_scroll(game, &format!("{ROOT}/CharacterSelectPanel/Contents"))?;
    Ok(())
}

/// `ClassChangePanel.Awake`: seven `ClassUnit`s per list.
fn class_awake(game: &mut Game, opener: &mut Opener, s: &mut State) -> Result<()> {
    for (list, yotogi) in [("ClassListMaid", false), ("ClassListYotogi", true)] {
        let parent = wf::node(game, &format!("{CLASS}/ClassViewer/Common/{list}/Contents/UnitParent"))?;
        wf::clear_children(game, parent);
        for k in 0..7 {
            let unit = wf::create_prefab(game, opener, parent, "SceneCompetitiveShow/Prefab/ClassUnit")?;
            let name = enum_text(if yotogi { YOTOGI_CLASS_TEXT } else { MAID_CLASS_TEXT }, k);
            let n = wf::child(game, unit, "ClassName")?;
            wf::state(game, n).text = Some(name);
            let l = wf::child(game, unit, "Level/Value")?;
            wf::state(game, l).text = Some("0".into());
            wf::state(game, unit).click = Some("ClassUnit.OnSelect".into());
            s.units.push((unit, yotogi, k));
        }
        wf::reposition(game, parent);
    }
    wf::set_click(game, &format!("{CLASS}/ClassViewer/TabButton/MaidClass"), "ClassChangePanel.OnSelectType:maid")?;
    wf::set_click(game, &format!("{CLASS}/ClassViewer/TabButton/YotogiClass"), "ClassChangePanel.OnSelectType:yotogi")?;
    wf::set_active(game, &format!("{CLASS}/ClassViewer/Common/ClassListMaid"), true)?;
    wf::set_active(game, &format!("{CLASS}/ClassViewer/Common/ClassListYotogi"), false)?;
    Ok(())
}

/// `ClassChangePanel.SetTargetMaid` / `ClassUnit.UpdateMaidData` and
/// `SetInfoData` of `info` (list, class).
fn update_class_panel(game: &mut Game, s: &State, m: usize, info: Option<(bool, i32)>) -> Result<()> {
    let st = lock(game, |t| Ok(t.maids.get(&m).cloned().unwrap_or_default()))?;
    for &(unit, yotogi, k) in &s.units {
        let c = if yotogi { st.yotogi_class.get(k as usize) } else { st.maid_class.get(k as usize) };
        let (have, level) = c.map_or((false, 0), |c| (c.is_have, c.level()));
        let l = wf::child(game, unit, "Level/Value")?;
        wf::state(game, l).text = Some(level.to_string());
        let current = if yotogi { st.current_yotogi_class } else { st.current_maid_class };
        let us = wf::state(game, unit);
        us.enabled = Some(have);
        us.color = (k == current).then_some([1., 1., 1., 1.]);
    }
    let yotogi_list = game.level.as_ref().and_then(|l| l.scene.find(&format!("{CLASS}/ClassViewer/Common/ClassListYotogi"))).and_then(|n| game.level.as_ref().unwrap().states.get(&n).and_then(|x| x.active)).unwrap_or(false);
    let (yotogi, class) = info.unwrap_or((yotogi_list, if yotogi_list { st.current_yotogi_class } else { st.current_maid_class }));
    // SetInfoData: the level and exp are the current class's (as the original).
    let cur = if yotogi { st.yotogi_class.get(st.current_yotogi_class as usize) } else { st.maid_class.get(st.current_maid_class as usize) }.cloned().unwrap_or_default();
    let base = format!("{CLASS}/ClassViewer/Common/ClassDescription");
    wf::set_text(game, &format!("{base}/ClassName"), enum_text(if yotogi { YOTOGI_CLASS_TEXT } else { MAID_CLASS_TEXT }, class))?;
    wf::set_text(game, &format!("{base}/LevelGroup/Level/Value"), cur.level().to_string())?;
    wf::set_text(game, &format!("{base}/LevelGroup/Exp/Value"), cur.exp.current.to_string())?;
    wf::set_text(game, &format!("{base}/LevelGroup/RequiredExp/Value"), cur.exp.next_level_exp(cur.level()).to_string())?;
    let d = lock(game, |t| t.data())?;
    let names = if yotogi { YOTOGI_CLASS } else { MAID_CLASS };
    let info_table = d.table(if yotogi { "yotogi_class_infotext.nei" } else { "maid_class_infotext.nei" })?;
    let text = (1..info_table.rows).find(|y| info_table.exists(0, *y) && info_table.string(0, *y) == names[class as usize]).map(|y| info_table.string(1, y)).unwrap_or_default();
    wf::set_text(game, &format!("{base}/Description"), text)?;
    // Special skills: skills whose acquisition requires this class, by level.
    let mut by_level: std::collections::BTreeMap<i32, Vec<String>> = Default::default();
    for cat in &d.skills {
        for sk in cat.values() {
            let g = &sk.get;
            if yotogi && 1 <= g.yotogi_class_level && g.yotogi_class == class {
                by_level.entry(g.yotogi_class_level).or_default().push(sk.name.clone());
            } else if !yotogi && 1 <= g.maid_class_level && g.maid_class == class {
                by_level.entry(g.maid_class_level).or_default().push(sk.name.clone());
            }
        }
    }
    let parent = wf::node(game, &format!("{CLASS}/ClassViewer/Yotogi/SkillList/Contents/UnitParent"))?;
    let labels = game.level.as_ref().unwrap().scene.nodes[parent].children.clone();
    let mut list: Vec<String> = by_level.into_values().flatten().take(labels.len()).collect();
    if list.len() == labels.len() && !labels.is_empty() {
        *list.last_mut().unwrap() = "etc...".into();
    }
    for (i, l) in labels.into_iter().enumerate() {
        let st = wf::state(game, l);
        match list.get(i) {
            Some(t) => {
                st.active = Some(true);
                st.text = Some(t.clone());
            }
            None => st.active = Some(false),
        }
    }
    Ok(())
}

/// `StatusMgr.LoadData` → `StatusCtrl.SetData`.
fn fill_status(game: &mut Game, m: usize) -> Result<()> {
    let (st, days) = lock(game, |t| Ok((t.maids.get(&m).cloned().unwrap_or_default(), t.player.days)))?;
    let mc = st.maid_class.get(st.current_maid_class as usize).cloned().unwrap_or_default();
    let yc = st.yotogi_class.get(st.current_yotogi_class as usize).cloned().unwrap_or_default();
    let personal = enum_text(PERSONAL_TEXT, st.personal);
    let personal = if personal == "クールで寡黙" { "クールで大人びている".into() } else { personal };
    let rank = if st.popular_rank == 0 { "-".into() } else { st.popular_rank.to_string() };
    let rows: Vec<(&str, String)> = vec![
        ("Name/FirstName", st.first_name.clone()),
        ("Name/LastName", st.last_name.clone()),
        ("ContractType/Value", enum_text(CONTRACT_TEXT, st.contract_type)),
        ("Character/Value", personal),
        ("SexExperience/Value", enum_text(SEIKEIKEN_TEXT, st.seikeiken)),
        ("MaidClass/Value", enum_text(MAID_CLASS_TEXT, st.current_maid_class)),
        ("MaidClass/Level/Value", mc.level().to_string()),
        ("MaidClass/Exp/CurrentExp/Value", mc.exp.current.to_string()),
        ("MaidClass/Exp/RequiredExp/Value", mc.exp.next_level_exp(mc.level()).to_string()),
        ("YotogiClass/Value", enum_text(YOTOGI_CLASS_TEXT, st.current_yotogi_class)),
        ("YotogiClass/Level/Value", yc.level().to_string()),
        ("YotogiClass/Exp/CurrentExp/Value", yc.exp.current.to_string()),
        ("YotogiClass/Exp/RequiredExp/Value", yc.exp.next_level_exp(yc.level()).to_string()),
        ("BodySize/Height/Value", st.body.height.to_string()),
        ("BodySize/Weight/Value", st.body.weight.to_string()),
        ("BodySize/Bust/Value", st.body.bust.to_string()),
        ("BodySize/Bust/Cup/Value", st.body.cup.clone()),
        ("BodySize/Waist/Value", st.body.waist.to_string()),
        ("BodySize/Hip/Value", st.body.hip.to_string()),
        ("Parameter/Parent/Hp/Value", st.a_hp().to_string()),
        ("Parameter/Parent/Reason/Value", st.a_reason().to_string()),
        ("Parameter/Parent/Mind/Value", st.a_mind().to_string()),
        ("Parameter/Parent/Likability/Value", st.likability.to_string()),
        ("Parameter/Parent/Care/Value", st.a_care().to_string()),
        ("Parameter/Parent/Reception/Value", st.a_reception().to_string()),
        ("Parameter/Parent/StudyRate/Value", st.a_study_rate().to_string()),
        ("Parameter/Parent/TeachRate/Value", st.a_teach_rate().to_string()),
        ("Parameter/Parent/Lovely/Value", st.a_lovely().to_string()),
        ("Parameter/Parent/Elegance/Value", st.a_elegance().to_string()),
        ("Parameter/Parent/Charm/Value", st.a_charm().to_string()),
        ("Parameter/Parent/Inran/Value", st.a_inyoku().to_string()),
        ("Parameter/Parent/MValue/Value", st.a_m_value().to_string()),
        ("Parameter/Parent/Hentai/Value", st.a_hentai().to_string()),
        ("Parameter/Parent/Housi/Value", st.a_housi().to_string()),
        ("Parameter/Condition/Value", enum_text(CONDITION_TEXT, st.condition)),
        ("Parameter/YotogiPlayCount/Value", st.yotogi_play_count.to_string()),
        ("Parameter/OthersPlayCount/Value", st.others_play_count.to_string()),
        ("ErogenousZone/Mouth/Value", st.sexual.mouth.to_string()),
        ("ErogenousZone/Throat/Value", st.sexual.throat.to_string()),
        ("ErogenousZone/Nipple/Value", st.sexual.nipple.to_string()),
        ("ErogenousZone/Clitoris/Value", st.sexual.curi.to_string()),
        ("Ranking/Value", rank),
        ("Evaluation/AcquisitionOfClientEvaluation/Value", st.total_evaluation.to_string()),
        ("Evaluation/AcquisitionOfWorkingFunds/Value", crate::daily::grouped(st.total_sales)),
        ("DaysOfEmployment/Value", (days - st.employment_day).to_string()),
    ];
    for (path, t) in rows {
        // Some layouts lack a row; the original's GetChildObject would assert.
        let full = format!("{STATUS}/{path}");
        wf::set_text(game, &full, t).with_context(|| format!("StatusCtrl {path}"))?;
    }
    for (k, crown) in [(1, "Gold"), (2, "Silver"), (3, "Bronze")] {
        wf::set_active(game, &format!("{STATUS}/Ranking/CrownIcon/{crown}"), st.popular_rank == k)?;
    }
    Ok(())
}

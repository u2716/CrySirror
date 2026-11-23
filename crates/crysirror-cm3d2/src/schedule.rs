// SPDX-License-Identifier: MPL-2.0
//! `Schedule.ScheduleAPI` over the TJS-owned parameters: schedule slots,
//! work availability and error checks, `DayStartManage`, random work
//! assignment, success levels and the noon/night work results, with
//! `param.Correction` for the feature/propensity bonuses.
//!
//! Maids are character indices; a schedule slot names its maid by GUID
//! (`GetScheduleSlot`). `Dictionary` iteration over work tables follows
//! table id order (the `.nei` rows are in id order).
use crate::{
    maid_param::{MaidParam, PlayerParam},
    params::{parse_enum, FEATURE, SCHEDULE_SLOTS},
    schedule_data::*,
    tjs::TjsState,
};
use anyhow::Result;

const NOON_BASIC: i32 = 0;
const NOON_DISPATCH: i32 = 1;
const NOON_TRAINER: i32 = 2;
/// `NoonWorkType.Trainer` (hidden in the schedule's noon work list).
pub const NOON_TRAINER_TYPE: i32 = NOON_TRAINER;
const NOON_TRAINEE: i32 = 3;
const NIGHT_TRAINER: i32 = 4;
const NIGHT_TRAINEE: i32 = 5;
const CONTRACT_NURTURE: i32 = 0;
/// `WorkSuccessLv`.
pub const MISS: i32 = 0;
pub const SUCCESS: i32 = 1;
pub const PERFECT: i32 = 2;

/// `param.Correction.Data`: excite, reason, mind, reception, care, lovely,
/// elegance, charm, inyoku, hentai, housi, m_value, teach_rate,
/// cleanliness, gorgeous.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Correction(pub [i32; 15]);

impl Correction {
    pub fn reception(&self) -> i32 {
        self.0[3]
    }
    pub fn care(&self) -> i32 {
        self.0[4]
    }
    pub fn lovely(&self) -> i32 {
        self.0[5]
    }
    pub fn elegance(&self) -> i32 {
        self.0[6]
    }
    pub fn charm(&self) -> i32 {
        self.0[7]
    }
    pub fn teach_rate(&self) -> i32 {
        self.0[12]
    }
    pub fn cleanliness(&self) -> i32 {
        self.0[13]
    }
    pub fn gorgeous(&self) -> i32 {
        self.0[14]
    }
}

/// `ScheduleAPI.NoonWorkResultSimulateParam`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NoonResult {
    pub study_rate: i32,
    pub reception: i32,
    pub care: i32,
    pub lovely: i32,
    pub elegance: i32,
    pub charm: i32,
    pub teach_rate: i32,
    pub exp: i32,
    pub cleanliness: i32,
    pub gorgeous: i32,
    pub inyoku: i32,
    pub m_value: i32,
    pub hentai: i32,
    pub housi: i32,
    pub income: i32,
    pub command_exp: i32,
}

/// `ScheduleAPI.NightWorkResultSimulateParam`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NightResult {
    pub income: i32,
    pub evaluation: i32,
    pub cleanliness: i32,
    pub gorgeous: i32,
}

fn basic(work: i32, maid: i32, lv: i32, success: f64) -> i32 {
    (work as f64 * (100 + maid + lv) as f64 / 100. * success).ceil() as i32
}
fn trainee(trainer: i32, lv: i32, teach: i32) -> i32 {
    (trainer as f64 / 25. * (100 + lv + teach) as f64 / 100.).ceil() as i32
}

impl TjsState {
    /// `PlayerParam.status.GetScheduleSlot(no)`.
    pub fn schedule_slot(&self, no: usize) -> Option<usize> {
        let guid = &self.player.schedule_slot.get(no)?.maid_guid;
        if guid.is_empty() {
            return None;
        }
        self.stock.iter().copied().find(|i| self.maids.get(i).is_some_and(|m| &m.guid_id == guid))
    }
    fn slot_maids(&self) -> Vec<usize> {
        (0..SCHEDULE_SLOTS).filter_map(|i| self.schedule_slot(i)).collect()
    }
    fn stock_by_guid(&self, guid: &str) -> Option<usize> {
        if guid.is_empty() {
            return None;
        }
        self.stock.iter().copied().find(|i| self.maids.get(i).is_some_and(|m| m.guid_id == guid))
    }
    fn noon_id(&self, m: usize) -> i32 {
        self.maids.get(&m).map_or(0, |s| s.noon_work_id)
    }
    fn night_id(&self, m: usize) -> i32 {
        self.maids.get(&m).map_or(0, |s| s.night_work_id)
    }
    fn set_noon(&mut self, m: usize, id: i32) {
        if let Some(s) = self.maids.get_mut(&m) {
            s.noon_work_id = id;
        }
    }
    fn set_night(&mut self, m: usize, id: i32) {
        if let Some(s) = self.maids.get_mut(&m) {
            s.night_work_id = id;
        }
    }
    fn work_id_of(&self, noon: bool, t: i32) -> i32 {
        let Some(d) = &self.data else { return 0 };
        if noon {
            d.schedule.noon.values().find(|w| w.work_type == t).map_or(0, |w| w.id)
        } else {
            d.schedule.night.values().find(|w| w.work_type == t).map_or(0, |w| w.id)
        }
    }
    /// `GetNoon/NightTrainer/TraineeMaid`.
    fn slot_with(&self, noon: bool, work_type: i32) -> Option<usize> {
        let id = self.work_id_of(noon, work_type);
        self.slot_maids().into_iter().find(|m| if noon { self.noon_id(*m) == id } else { self.night_id(*m) == id })
    }
    /// `ScheduleScene.SetSlot_Safe`: put `maid` into a slot; if she sits in
    /// another slot the two swap.
    pub fn set_slot_safe(&mut self, slot: usize, maid: Option<usize>) {
        if slot >= SCHEDULE_SLOTS {
            return;
        }
        let guid = |s: &Self, m: Option<usize>| m.and_then(|m| s.maids.get(&m)).map(|x| x.guid_id.clone()).unwrap_or_default();
        if let Some(m) = maid {
            if self.schedule_slot(slot) == Some(m) {
                return;
            }
            if let Some(other) = (0..SCHEDULE_SLOTS).find(|i| *i != slot && self.schedule_slot(*i) == Some(m)) {
                let g = guid(self, self.schedule_slot(slot));
                self.player.schedule_slot[other].maid_guid = g;
            }
        }
        let g = guid(self, maid);
        self.player.schedule_slot[slot].maid_guid = g;
        self.maid_work_id_error_check(None, true);
    }

    /// `ScheduleScene.SetNoonWorkSlot_Safe`.
    pub fn set_noon_work_slot_safe(&mut self, slot: usize, work: i32) {
        let Some(m) = self.schedule_slot(slot) else { return };
        let Some(d) = self.data.clone() else { return };
        if work == 0 {
            // The original draws and discards a random work here.
            let personal = self.maids.get(&m).map_or(0, |s| s.personal).clamp(0, 2) as usize;
            self.work_random(&d.schedule.noon_init[personal]);
        }
        if d.schedule.noon_type(work) == Some(NOON_TRAINEE) {
            self.set_noon(m, work);
            self.repeated_work_del(m, true);
            let trainee_night = self.work_id_of(false, NIGHT_TRAINEE);
            self.set_night(m, trainee_night);
            self.repeated_work_del(m, false);
            if let Some(l) = self.slot_maids().into_iter().find(|x| self.maids.get(x).is_some_and(|s| s.leader)) {
                let (noon, night) = (self.work_id_of(true, NOON_TRAINER), self.work_id_of(false, NIGHT_TRAINER));
                self.set_noon(l, noon);
                self.repeated_work_del(l, true);
                self.set_night(l, night);
                self.repeated_work_del(l, false);
            }
        } else {
            self.set_noon(m, work);
        }
        self.maid_work_id_error_check(None, true);
    }

    /// `ScheduleScene.SetNightWorkSlot_Safe`.
    pub fn set_night_work_slot_safe(&mut self, slot: usize, work: i32) {
        let Some(m) = self.schedule_slot(slot) else { return };
        let Some(d) = self.data.clone() else { return };
        if work == 0 {
            let personal = self.maids.get(&m).map_or(0, |s| s.personal).clamp(0, 2) as usize;
            self.work_random(&d.schedule.night_init[personal]);
        }
        self.set_night(m, work);
        if matches!(d.schedule.night_type(work), Some(NIGHT_VIP) | Some(NIGHT_VIP_CALL)) {
            self.repeated_work_del(m, false);
        }
        self.maid_work_id_error_check(None, true);
    }

    /// `RepeatedNoonWorkDel` / `RepeatedNightWorkDel`.
    fn repeated_work_del(&mut self, base: usize, noon: bool) {
        let id = if noon { self.noon_id(base) } else { self.night_id(base) };
        for m in self.stock.clone() {
            if m != base && (if noon { self.noon_id(m) } else { self.night_id(m) }) == id {
                if noon { self.set_noon(m, 0) } else { self.set_night(m, 0) }
            }
        }
    }

    /// `NoonWork.lv` (`NoonWorkPlayToLv(NoonWorkPlayCount)`).
    pub fn noon_work_lv(&self, m: usize, work: i32) -> i32 {
        self.noon_lv_revision(m, work) / 10 + 1
    }

    /// `ScheduleAPI.InfoReplace`: `[HF]` → the called maid's first name.
    pub fn info_replace(&self, work: i32, info: &str) -> String {
        let called = self.player.night_works_state.get(&work).and_then(|s| self.stock_by_guid(&s.called_maid_guid));
        match called.and_then(|m| self.maids.get(&m)) {
            Some(st) => info.replace("[HF]", &st.first_name),
            None => info.to_owned(),
        }
    }

    /// `GetStockMaidLeader`.
    pub fn leader(&self) -> Option<usize> {
        self.stock.iter().copied().find(|i| self.maids.get(i).is_some_and(|m| m.leader))
    }

    /// `ScheduleAPI.VipCalledMaid`.
    fn vip_called_maid(&self, work: i32, m: usize) -> bool {
        let Some(st) = self.player.night_works_state.get(&work) else { return false };
        if st.called_maid_guid.is_empty() {
            return false;
        }
        self.maids.get(&m).is_none_or(|s| s.guid_id != st.called_maid_guid)
    }

    /// `ScheduleAPI.EnableNoonWork`.
    pub fn enable_noon_work(&self, work: i32, maid: Option<usize>) -> bool {
        let Some(d) = self.data.clone() else { return false };
        let Some(w) = d.schedule.noon.get(&work) else { return false };
        let Some(m) = maid else { return true };
        let s = self.maids.get(&m).cloned().unwrap_or_default();
        if w.work_type == NOON_TRAINER && (Some(m) != self.leader() || s.contract_type == CONTRACT_NURTURE) {
            return false;
        }
        if w.work_type == NOON_TRAINEE {
            if Some(m) == self.leader() || s.a_study_rate() < -w.study_rate {
                return false;
            }
            // The original checks the trainer with the night trainer work id.
            let trainer_night = self.work_id_of(false, NIGHT_TRAINER);
            if !self.slot_maids().into_iter().any(|o| self.enable_noon_work(trainer_night, Some(o))) {
                return false;
            }
        }
        if !w.cond_contract.is_empty() && !w.cond_contract.contains(&s.contract_type) {
            return false;
        }
        for c in &w.cond_maid_class {
            if !s.maid_class.get(*c as usize).is_some_and(|x| x.is_have) {
                return false;
            }
        }
        true
    }

    /// `ScheduleAPI.VisibleNightWork`.
    pub fn visible_night_work(&self, work: i32) -> bool {
        let Some(d) = &self.data else { return false };
        let Some(w) = d.schedule.night.get(&work) else { return false };
        match w.work_type {
            NIGHT_TRAINER | NIGHT_TRAINEE => false,
            NIGHT_VIP | NIGHT_VIP_CALL => match self.player.night_works_state.get(&work) {
                None => false,
                Some(st) => !(st.finish && (self.player.best_salon_grade < 5 || self.player.days < d.schedule.def("vipFullOpenDay"))),
            },
            _ => true,
        }
    }

    /// `ScheduleAPI.EnableNightWork`.
    pub fn enable_night_work(&self, work: i32, maid: Option<usize>, called_check: bool) -> bool {
        let Some(d) = self.data.clone() else { return false };
        let Some(w) = d.schedule.night.get(&work) else { return false };
        if w.cond_salon_grade > self.player.best_salon_grade {
            return false;
        }
        let Some(m) = maid else { return true };
        let s = self.maids.get(&m).cloned().unwrap_or_default();
        if w.work_type == NIGHT_VIP_CALL && called_check && !self.vip_called_maid(work, m) {
            return false;
        }
        let noon_type = d.schedule.noon_type(s.noon_work_id);
        if w.work_type == NIGHT_TRAINER && noon_type != Some(NOON_TRAINER) {
            return false;
        }
        if w.work_type == NIGHT_TRAINEE && noon_type != Some(NOON_TRAINEE) {
            return false;
        }
        for (id, lv) in &w.cond_skill {
            match s.skill_data.get(id) {
                Some(k) if *lv <= k.exp.level => {}
                _ => return false,
            }
        }
        let any = |list: &[i32], v: i32| list.is_empty() || list.contains(&v);
        if !any(&w.cond_contract, s.contract_type) || !any(&w.cond_maid_class, s.current_maid_class) || !any(&w.cond_yotogi_class, s.current_yotogi_class) || !any(&w.cond_seikeiken, s.seikeiken) {
            return false;
        }
        if !w.cond_propensity.is_empty() && !w.cond_propensity.iter().any(|p| s.is_propensity(*p)) {
            return false;
        }
        if w.cond_flag1.iter().any(|f| s.flag(f) < 1) || w.cond_flag0.iter().any(|f| s.flag(f) > 0) {
            return false;
        }
        true
    }

    /// `ScheduleAPI.NightWorksStateErrorCheck`.
    pub fn night_works_state_error_check(&mut self, care: bool) {
        let Some(d) = self.data.clone() else { return };
        for (id, w) in &d.schedule.night {
            let Some(st) = self.player.night_works_state.get(id).cloned() else { continue };
            if st.finish || w.work_type != NIGHT_VIP_CALL {
                continue;
            }
            let ok = self.stock_by_guid(&st.called_maid_guid).is_some_and(|m| self.enable_night_work(*id, Some(m), false));
            if !ok {
                self.player.night_works_state.remove(id);
            }
        }
        if care {
            self.maid_work_id_error_check(None, care);
        }
    }

    /// `ScheduleAPI.SlotEmptyCheck`: the leader fills slot 0 of an empty
    /// schedule.
    pub fn slot_empty_check(&mut self) {
        if self.slot_maids().is_empty() {
            if let Some(l) = self.leader() {
                let guid = self.maids[&l].guid_id.clone();
                self.player.schedule_slot[0].maid_guid = guid;
            }
        }
    }

    /// `ScheduleAPI.MaidWorkIdErrorCheck`.
    pub fn maid_work_id_error_check(&mut self, maid: Option<usize>, care: bool) {
        self.slot_empty_check();
        let Some(d) = self.data.clone() else { return };
        let Some(m) = maid else {
            let (nt, gt) = (self.slot_with(true, NOON_TRAINER), self.slot_with(false, NIGHT_TRAINER));
            let (ne, ge) = (self.slot_with(true, NOON_TRAINEE), self.slot_with(false, NIGHT_TRAINEE));
            if (nt != gt || ne != ge || nt.is_none() || gt.is_none()) && (nt.is_some() || gt.is_some() || ne.is_some() || ge.is_some()) {
                if let Some(x) = nt {
                    self.set_noon(x, 0);
                }
                if let Some(x) = gt {
                    self.set_night(x, 0);
                }
                if let Some(x) = ne {
                    self.set_noon(x, 0);
                }
                if let Some(x) = ge {
                    self.set_night(x, 0);
                }
            }
            for i in 0..SCHEDULE_SLOTS {
                if let Some(x) = self.schedule_slot(i) {
                    self.maid_work_id_error_check(Some(x), care);
                }
            }
            return;
        };
        let noon = self.noon_id(m);
        match d.schedule.noon.get(&noon) {
            Some(w) if w.work_type == NOON_TRAINER => {
                let bad = !self.enable_noon_work(noon, Some(m))
                    || d.schedule.night_type(self.night_id(m)) != Some(NIGHT_TRAINER)
                    || !self.slot_maids().into_iter().any(|o| d.schedule.noon_type(self.noon_id(o)) == Some(NOON_TRAINEE));
                if bad {
                    self.set_noon(m, 0);
                    if care {
                        self.empty_to_random_work(m);
                    }
                    if let Some(o) = self.slot_maids().into_iter().find(|o| d.schedule.noon_type(self.noon_id(*o)) == Some(NOON_TRAINEE)) {
                        self.set_noon(o, 0);
                        if care {
                            self.empty_to_random_work(o);
                        }
                    }
                }
            }
            Some(w) if w.work_type == NOON_TRAINEE => {
                let bad = !self.enable_noon_work(noon, Some(m)) || d.schedule.night_type(self.night_id(m)) != Some(NIGHT_TRAINEE);
                if bad {
                    self.set_noon(m, 0);
                    if let Some(o) = self.slot_maids().into_iter().find(|o| d.schedule.noon_type(self.noon_id(*o)) == Some(NOON_TRAINER)) {
                        self.set_noon(o, 0);
                        if care {
                            self.empty_to_random_work(o);
                        }
                    }
                }
            }
            Some(_) => {}
            None => self.set_noon(m, 0),
        }
        let night = self.night_id(m);
        match d.schedule.night.get(&night) {
            Some(w) => {
                let vip = matches!(w.work_type, NIGHT_VIP | NIGHT_VIP_CALL);
                let mut bad = vip && !self.visible_night_work(night);
                if !bad && vip {
                    bad = self.slot_maids().into_iter().any(|o| o != m && self.night_id(o) == night);
                }
                if !bad && (vip || matches!(w.work_type, NIGHT_ENTERTAIN | NIGHT_TRAINER | NIGHT_TRAINEE)) && !self.enable_night_work(night, Some(m), true) {
                    bad = true;
                }
                if bad {
                    self.set_night(m, 0);
                }
            }
            None => self.set_night(m, 0),
        }
        if care {
            self.empty_to_random_work(m);
        }
    }

    /// `ScheduleAPI.ScheduleErrorAllCheck`.
    pub fn schedule_error_all_check(&mut self) {
        self.slot_empty_check();
        self.night_works_state_error_check(false);
        self.maid_work_id_error_check(None, false);
        for m in self.slot_maids() {
            self.empty_to_random_work(m);
        }
    }

    /// `ScheduleAPI.EmptyToRandomWork`.
    pub fn empty_to_random_work(&mut self, m: usize) -> bool {
        let Some(d) = self.data.clone() else { return false };
        let personal = self.maids.get(&m).map_or(0, |s| s.personal).clamp(0, 2) as usize;
        let mut changed = false;
        if self.noon_id(m) == 0 {
            let id = self.work_random(&d.schedule.noon_init[personal]);
            self.set_noon(m, id);
            changed = true;
        }
        if self.night_id(m) == 0 {
            let id = self.work_random(&d.schedule.night_init[personal]);
            self.set_night(m, id);
            changed = true;
        }
        changed
    }

    fn work_random(&mut self, db: &std::collections::BTreeMap<i32, i32>) -> i32 {
        let total: i32 = db.values().sum();
        if total <= 0 {
            return 0;
        }
        let mut r = self.random(total as usize) as i32;
        for (id, w) in db {
            r -= w;
            if r < 0 {
                return *id;
            }
        }
        0
    }

    /// `ScheduleAPI.OccurNightWork`.
    fn occur_night_work(&mut self) {
        let Some(d) = self.data.clone() else { return };
        for (id, w) in &d.schedule.night {
            match w.work_type {
                NIGHT_VIP if !self.player.night_works_state.contains(id) && self.enable_night_work(*id, None, true) => {
                    self.player.night_works_state.insert(*id, crate::params::NightWorkState { work_id: *id, called_maid_guid: String::new(), finish: false });
                }
                NIGHT_VIP_CALL if !self.player.night_works_state.contains(id) && self.enable_night_work(*id, None, true) => {
                    let meet: Vec<usize> = self.stock.clone().into_iter().filter(|m| self.enable_night_work(*id, Some(*m), false)).collect();
                    if !meet.is_empty() {
                        // Random.Range(0, Count − 1): the last maid is never drawn
                        // unless she is the only one.
                        let k = if meet.len() > 1 { self.random(meet.len() - 1) } else { 0 };
                        let guid = self.maids[&meet[k]].guid_id.clone();
                        self.player.night_works_state.insert(*id, crate::params::NightWorkState { work_id: *id, called_maid_guid: guid, finish: false });
                    }
                }
                _ => {}
            }
        }
    }

    /// `ScheduleAPI.DayStartManage` (+ `PlayerParam.CheckTrophyFromMorning`
    /// of `StartDailyMgr`); returns newly owned trophies.
    pub fn day_start_manage(&mut self) -> Result<Vec<i32>> {
        let data = self.data()?;
        for i in 0..SCHEDULE_SLOTS {
            if self.schedule_slot(i).is_some() {
                self.player.schedule_slot[i].communication = false;
            }
        }
        for m in self.stock.clone() {
            let night = self.night_id(m);
            match data.schedule.night_type(night) {
                Some(NIGHT_HAVE_SEX | NIGHT_VIP | NIGHT_VIP_CALL) | None => self.set_night(m, 0),
                Some(_) => {}
            }
        }
        self.slot_empty_check();
        self.night_works_state_error_check(false);
        self.occur_night_work();
        self.maid_work_id_error_check(None, false);
        for m in self.stock.clone() {
            self.empty_to_random_work(m);
        }
        let mut p = PlayerParam { st: &mut self.player, data: &data };
        p.set_flag("_Schedule_Noon_Resulted", 0);
        p.set_flag("_Schedule_Night_Resulted", 0);
        let mut got = Vec::new();
        for (days, id) in [(50, 280), (100, 290), (500, 300)] {
            if days <= p.st.days && p.add_have_trophy(id) {
                got.push(id);
            }
        }
        Ok(got)
    }

    /// `ScheduleAPI.DecideSuccess`.
    pub fn decide_success(&mut self) {
        let Some(d) = self.data.clone() else { return };
        let fatigue = parse_enum(FEATURE, "疲労").unwrap_or(19);
        for i in 0..SCHEDULE_SLOTS {
            let Some(m) = self.schedule_slot(i) else { continue };
            let level = if self.player.schedule_slot[i].communication {
                SUCCESS
            } else if self.maids[&m].is_feature(fatigue) {
                MISS
            } else {
                let r = 1 + self.random(99) as i32;
                if r <= d.schedule.def("workSuccessLvMissIncidence") {
                    MISS
                } else if r > 100 - d.schedule.def("workSuccessLvPerfectIncidence") {
                    PERFECT
                } else {
                    SUCCESS
                }
            };
            self.player.schedule_slot[i].noon_success_level = level;
        }
    }

    /// `param.Correction.GetData`.
    pub fn correction(&self, m: usize) -> Correction {
        let mut c = Correction::default();
        let (Some(d), Some(s)) = (&self.data, self.maids.get(&m)) else { return c };
        for (f, v) in &d.feature_correction {
            if s.is_feature(*f) {
                for k in 0..15 {
                    c.0[k] += v.0[k];
                }
            }
        }
        for (p, v) in &d.propensity_correction {
            if s.is_propensity(*p) {
                for k in 0..15 {
                    c.0[k] += v.0[k];
                }
            }
        }
        c
    }

    /// `NoonWorkPlayCount` → `NoonWorkPlayToLv` → `NoonWorkLvToRevision`.
    fn noon_lv_revision(&self, m: usize, work: i32) -> i32 {
        let Some(d) = &self.data else { return 0 };
        let count = self.maids.get(&m).and_then(|s| s.work_data.get(&work)).map_or(0, |w| w.play_count as i32);
        let lv = if count < d.schedule.def("noonWorkLv2Exp") {
            1
        } else if count < d.schedule.def("noonWorkLv3Exp") {
            2
        } else {
            3
        };
        (lv - 1) * 10
    }

    /// `ScheduleAPI.NoonWorkResultSimulate`.
    pub fn noon_result(&self, m: usize, work: i32, success: i32, communication: bool) -> NoonResult {
        let mut r = NoonResult::default();
        let Some(d) = &self.data else { return r };
        let Some(w) = d.schedule.noon.get(&work) else { return r };
        match w.work_type {
            NOON_BASIC | NOON_DISPATCH => {
                let c = self.correction(m);
                let lv = self.noon_lv_revision(m, work);
                let s = if communication {
                    0.3
                } else {
                    match success {
                        MISS => 0.5,
                        PERFECT => 1.5,
                        _ => 1.0,
                    }
                };
                r.study_rate = w.study_rate;
                r.reception = basic(w.reception, c.reception(), lv, s);
                r.care = basic(w.care, c.care(), lv, s);
                r.lovely = basic(w.lovely, c.lovely(), lv, s);
                r.elegance = basic(w.elegance, c.elegance(), lv, s);
                r.charm = basic(w.charm, c.charm(), lv, s);
                r.teach_rate = basic(w.teach_rate, c.teach_rate(), lv, s);
                r.income = basic(w.income, 0, lv, s);
                let care = self.maids[&m].a_care() as f64;
                r.income = (r.income as f64 * ((care / d.schedule.def("dispatchIncomCoefficient") as f64).ceil() / 100. + 1.)) as i32;
                r.exp = basic(w.exp, 0, lv, s);
                r.cleanliness = basic(w.cleanliness, c.cleanliness(), lv, s);
                r.gorgeous = basic(w.gorgeous, c.gorgeous(), lv, s);
                if communication {
                    r.study_rate = 0;
                }
            }
            NOON_TRAINEE => {
                if let Some(l) = self.leader() {
                    let lv = self.noon_lv_revision(l, self.work_id_of(true, NOON_TRAINER));
                    let s = &self.maids[&l];
                    let teach = s.a_teach_rate() / 50;
                    r.study_rate = w.study_rate;
                    r.reception = trainee(s.a_reception(), lv, teach);
                    r.care = trainee(s.a_care(), lv, teach);
                    r.lovely = trainee(s.a_lovely(), lv, teach);
                    r.elegance = trainee(s.a_elegance(), lv, teach);
                    r.charm = trainee(s.a_charm(), lv, teach);
                    r.inyoku = trainee(s.a_inyoku(), lv, teach);
                    r.m_value = trainee(s.a_m_value(), lv, teach);
                    r.hentai = trainee(s.a_hentai(), lv, teach);
                    r.housi = trainee(s.a_housi(), lv, teach);
                }
            }
            _ => {}
        }
        r
    }

    /// `ScheduleAPI.AddNoonWorkResultParam`.
    pub fn add_noon_result(&mut self, m: usize, r: &NoonResult) -> Result<()> {
        let data = self.data()?;
        let ranking = self.player.flag("__ランキング解放") == 1;
        let s = self.maids.get_mut(&m).ok_or_else(|| anyhow::anyhow!("no maid {m}"))?;
        let mut p = MaidParam { st: s, data: &data };
        for (k, v) in [("習得率", r.study_rate), ("接待", r.reception), ("お世話", r.care), ("可憐", r.lovely), ("気品", r.elegance), ("魅惑", r.charm), ("指導", r.teach_rate)] {
            p.add_status(k, v);
        }
        p.add_sales(r.income.into(), ranking);
        let c = p.st.current_maid_class as usize;
        p.st.maid_class[c].exp.add(r.exp);
        for (k, v) in [("淫欲", r.inyoku), ("M性", r.m_value), ("変態", r.hentai), ("奉仕", r.housi)] {
            p.add_status(k, v);
        }
        let mut pp = PlayerParam { st: &mut self.player, data: &data };
        pp.add_money(r.income.into());
        pp.add_salon_clean(r.cleanliness);
        pp.add_salon_beautiful(r.gorgeous);
        Ok(())
    }

    /// `ScheduleAPI.NightWorkResultSimulate` (adds play numbers as the
    /// original does while simulating).
    pub fn night_result(&mut self, m: usize, work: i32) -> NightResult {
        let mut r = NightResult::default();
        let Some(d) = self.data.clone() else { return r };
        let Some(w) = d.schedule.night.get(&work) else { return r };
        match w.work_type {
            NIGHT_ENTERTAIN => {
                let grade = self.player.current_salon_grade.max(0) as usize;
                let n = d.schedule.entertain_number.get(grade).copied().unwrap_or(0);
                let guests = d.schedule.entertain_guest.get(grade).cloned().unwrap_or_default();
                let (mut income, mut evaluation) = (0, 0);
                let threshold = d.schedule.def("entertainParameterRevisionThreshold");
                for _ in 0..n {
                    if guests.is_empty() {
                        break;
                    }
                    let g = guests[self.random(guests.len())].clone();
                    let s = self.maids[&m].clone();
                    let up = |v: i32| (v as f64 / 500.).ceil();
                    let mut a = if !g.sexual {
                        up(s.a_lovely()) + up(s.a_elegance()) + up(s.a_charm()) + up(s.a_reception())
                    } else {
                        up(s.a_inyoku()) + up(s.a_m_value()) + up(s.a_hentai()) + up(s.a_housi())
                    };
                    a /= 2.;
                    a = a / 100. + 1.;
                    let mut b = self.range(-1, 2) as f64 * 10.;
                    for _ in 0..s.propensity.len() {
                        b += self.range(-1, 2) as f64 * 10.;
                    }
                    for v in [s.a_inyoku(), s.a_m_value(), s.a_hentai(), s.a_housi(), s.a_lovely(), s.a_elegance(), s.a_charm(), s.a_reception()] {
                        if v >= threshold {
                            b += self.range(-1, 2) as f64 * 10.;
                        }
                    }
                    b = b / 100. + 1.;
                    // Integer division of the original (min/max per cent / 100).
                    let (lo, hi) = ((d.schedule.def("entertainPropensityRevisionMin") / 100) as f64, (d.schedule.def("entertainPropensityRevisionMax") / 100) as f64);
                    if b < lo {
                        b = lo;
                    } else if b > hi {
                        b = hi;
                    }
                    if g.sexual {
                        match s.contract_type {
                            1 => {
                                let mut p = MaidParam { st: self.maids.get_mut(&m).unwrap(), data: &d };
                                p.add_status("経験人数", 1);
                            }
                            2 => b /= 2.,
                            _ => {}
                        }
                    }
                    let inc = (g.base_income as f64 * a * b) as i32;
                    let ev = (g.base_evaluation as f64 * a * b) as i32;
                    income += ((inc as f64 / 1000.).ceil() as i32) * 1000;
                    evaluation += ev;
                }
                r.income = income;
                r.evaluation = evaluation;
                r.cleanliness = n * d.schedule.def("entertainAddSalonClean");
                r.gorgeous = n * d.schedule.def("entertainAddSalonBeautiful");
            }
            NIGHT_VIP | NIGHT_VIP_CALL => {
                let id = self.night_id(m);
                if self.player.night_works_state.get(&id).is_some_and(|s| !s.finish) {
                    r.income = w.income;
                    r.evaluation = w.evaluation;
                }
                let mut p = MaidParam { st: self.maids.get_mut(&m).unwrap(), data: &d };
                p.add_status("経験人数", 1);
            }
            NIGHT_HAVE_SEX => {
                if self.maids[&m].flag("_YotogiPlayed") > 0 {
                    r.cleanliness = d.schedule.def("haveSexAddSalonClean");
                    r.gorgeous = d.schedule.def("haveSexAddSalonBeautiful");
                }
            }
            _ => {}
        }
        r
    }

    /// `ScheduleAPI.AddNightWorkResultParam`.
    pub fn add_night_result(&mut self, m: usize, r: &NightResult) -> Result<()> {
        let data = self.data()?;
        let ranking = self.player.flag("__ランキング解放") == 1;
        let s = self.maids.get_mut(&m).ok_or_else(|| anyhow::anyhow!("no maid {m}"))?;
        let mut p = MaidParam { st: s, data: &data };
        p.add_evaluation(r.evaluation.into(), ranking);
        p.add_sales(r.income.into(), ranking);
        let mut pp = PlayerParam { st: &mut self.player, data: &data };
        pp.add_salon_evaluation(r.evaluation);
        pp.add_money(r.income.into());
        pp.add_salon_clean(r.cleanliness);
        pp.add_salon_beautiful(r.gorgeous);
        Ok(())
    }

    /// `UnityEngine.Random.Range(min, max)` for integers (max exclusive).
    pub fn range(&mut self, min: i32, max: i32) -> i32 {
        if max <= min {
            return min;
        }
        min + self.random((max - min) as usize) as i32
    }
}

/// `BackupParamAccessor.SCENE_ID`.
pub const MORNING: &str = "MORNING";
pub const EVENING_WITHOUT_COMMUNICATION_BONUS: &str = "EVENING_WITHOUT_COMMUNICATION_BONUS";
pub const EVENING_WITH_COMMUNICATION_BONUS: &str = "EVENING_WITH_COMMUNICATION_BONUS";
pub const MIDNIGHT: &str = "MIDNIGHT";

/// `BackupParamAccessor.Params` field names in `BackupParam` order.
pub const BACKUP_FIELDS: [&str; 26] = [
    "salon_cleanliness", "salon_gorgeous", "salon_evaluation", "money_top0", "money_top10", "evaluation", "sales_top0", "sales_top10", "study_rate",
    "reception", "care", "lovely", "elegance", "charm", "inyoku", "m_value", "hentai", "housi", "teach_rate", "maidClassLv", "maidClassExp",
    "yotogiClassLv", "yotogiClassExp", "yotogi_play_count", "others_play_count", "likability",
];

/// `BackupParamAccessor.Params` (by field name) with its `-` operator.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Params {
    pub v: std::collections::BTreeMap<&'static str, i32>,
    /// `cleanliness`/`gorgeous` filled from the "apped" values.
    pub cleanliness: i32,
    pub gorgeous: i32,
}

impl Params {
    pub fn get(&self, k: &str) -> i32 {
        self.v.get(k).copied().unwrap_or(0)
    }
    pub fn sub(&self, b: &Params) -> Params {
        Params { v: BACKUP_FIELDS.iter().map(|k| (*k, self.get(k) - b.get(k))).collect(), cleanliness: 0, gorgeous: 0 }
    }
    pub fn money(&self) -> i64 {
        self.get("money_top10") as i64 * 1_000_000_000 + self.get("money_top0") as i64
    }
    pub fn sales(&self) -> i64 {
        self.get("sales_top10") as i64 * 1_000_000_000 + self.get("sales_top0") as i64
    }
}

impl TjsState {
    /// `BackupParamAccessor.BackupParam(slotId, sceneId)`; `None` backs up
    /// every slot (the original then repeats slot 0, with the same values).
    pub fn backup_param(&mut self, slot: Option<usize>, scene: &str) {
        let slots: Vec<usize> = match slot {
            Some(s) => vec![s],
            None => (0..SCHEDULE_SLOTS).collect(),
        };
        for no in slots {
            let mut v: std::collections::BTreeMap<&'static str, i32> = BACKUP_FIELDS.iter().map(|k| (*k, 0)).collect();
            if let Some(m) = self.schedule_slot(no) {
                let p = &self.player;
                let s = &self.maids[&m];
                let money = p.money;
                let sales = s.total_sales;
                let mc = &s.maid_class[s.current_maid_class.clamp(0, 15) as usize];
                let yc = &s.yotogi_class[s.current_yotogi_class.clamp(0, 15) as usize];
                for (k, x) in [
                    ("salon_cleanliness", p.salon_clean),
                    ("salon_gorgeous", p.salon_beautiful),
                    ("salon_evaluation", p.salon_evaluation),
                    ("money_top0", (money % 1_000_000_000) as i32),
                    ("money_top10", (money / 1_000_000_000) as i32),
                    ("evaluation", s.total_evaluation as i32),
                    ("sales_top0", (sales % 1_000_000_000) as i32),
                    ("sales_top10", (sales / 1_000_000_000) as i32),
                    ("study_rate", s.a_study_rate()),
                    ("reception", s.a_reception()),
                    ("care", s.a_care()),
                    ("lovely", s.a_lovely()),
                    ("elegance", s.a_elegance()),
                    ("charm", s.a_charm()),
                    ("inyoku", s.a_inyoku()),
                    ("m_value", s.a_m_value()),
                    ("hentai", s.a_hentai()),
                    ("housi", s.a_housi()),
                    ("teach_rate", s.a_teach_rate()),
                    ("maidClassLv", mc.level()),
                    ("maidClassExp", mc.exp.total),
                    ("yotogiClassLv", yc.level()),
                    ("yotogiClassExp", yc.exp.total),
                    ("yotogi_play_count", s.yotogi_play_count),
                    ("others_play_count", s.others_play_count),
                    ("likability", s.likability),
                ] {
                    v.insert(k, x);
                }
            }
            let dic = &mut self.player.schedule_slot[no].backup_status;
            for k in BACKUP_FIELDS {
                dic.insert(format!("{scene}_{k}"), v[k]);
            }
        }
    }

    /// `BackupParamAccessor.GetBackupParams`.
    pub fn backup_params(&self, no: usize, scene: &str) -> Params {
        let dic = &self.player.schedule_slot[no].backup_status;
        Params { v: BACKUP_FIELDS.iter().map(|k| (*k, dic.get(format!("{scene}_{k}").as_str()).copied().unwrap_or(0))).collect(), cleanliness: 0, gorgeous: 0 }
    }

    pub fn set_backup_apped(&mut self, no: usize, scene: &str, cleanliness: i32, gorgeous: i32) {
        if self.schedule_slot(no).is_some() {
            let dic = &mut self.player.schedule_slot[no].backup_status;
            dic.insert(format!("{scene}_apped_cleanliness"), cleanliness);
            dic.insert(format!("{scene}_apped_gorgeous"), gorgeous);
        }
    }

    pub fn backup_apped(&self, no: usize, scene: &str) -> (i32, i32) {
        if self.schedule_slot(no).is_none() {
            return (0, 0);
        }
        let dic = &self.player.schedule_slot[no].backup_status;
        let get = |k: &str| dic.get(format!("{scene}_{k}").as_str()).copied().unwrap_or(0);
        (get("apped_cleanliness"), get("apped_gorgeous"))
    }

    /// `BackupParamAccessor.YotogiPlayed`.
    pub fn yotogi_played(&self, no: usize) -> bool {
        self.backup_params(no, MIDNIGHT).sub(&self.backup_params(no, MORNING)).get("yotogi_play_count") > 0
    }

    fn set_maid_flag(&mut self, m: usize, k: &str, v: i32) {
        if let Some(s) = self.maids.get_mut(&m) {
            s.generic_flag.insert(k.to_owned(), v);
        }
    }

    /// `new WorkResultScene(Noon)`: success levels, noon results with and
    /// without the communication bonus, work levels and the backups.
    pub fn work_result_noon(&mut self) -> Result<()> {
        if self.player.flag("_Schedule_Noon_Resulted") > 0 {
            return Ok(());
        }
        let data = self.data()?;
        self.player.generic_flag.insert("_Schedule_Noon_Resulted".into(), 1);
        for no in 0..SCHEDULE_SLOTS {
            if let Some(m) = self.schedule_slot(no) {
                let c = i32::from(self.player.schedule_slot[no].communication);
                self.set_maid_flag(m, "_communication", c);
            }
        }
        for no in 0..SCHEDULE_SLOTS {
            if let Some(m) = self.schedule_slot(no) {
                if matches!(data.schedule.noon_type(self.noon_id(m)), Some(NOON_TRAINER | NOON_TRAINEE)) {
                    self.player.schedule_slot[no].communication = false;
                }
            }
        }
        self.decide_success();
        self.backup_param(None, MORNING);
        for no in 0..SCHEDULE_SLOTS {
            let Some(m) = self.schedule_slot(no) else { continue };
            let work = self.noon_id(m);
            let level = self.player.schedule_slot[no].noon_success_level;
            let r = self.noon_result(m, work, level, false);
            self.add_noon_result(m, &r)?;
            self.set_backup_apped(no, EVENING_WITHOUT_COMMUNICATION_BONUS, r.cleanliness, r.gorgeous);
            let s = self.maids.get_mut(&m).unwrap();
            let mut p = MaidParam { st: s, data: &data };
            p.set_new_get_work(work);
            if let Some(w) = p.st.work_data.get_mut(&work) {
                w.play_count += 1;
            }
            let lv = self.noon_lv_revision(m, work) / 10 + 1;
            if let Some(w) = self.maids.get_mut(&m).and_then(|s| s.work_data.get_mut(&work)) {
                w.level = lv;
            }
        }
        self.backup_param(None, EVENING_WITHOUT_COMMUNICATION_BONUS);
        for no in 0..SCHEDULE_SLOTS {
            let Some(m) = self.schedule_slot(no) else { continue };
            let (mut clean, mut gorgeous) = self.backup_apped(no, EVENING_WITHOUT_COMMUNICATION_BONUS);
            if self.player.schedule_slot[no].communication {
                let work = self.noon_id(m);
                let level = self.player.schedule_slot[no].noon_success_level;
                let r = self.noon_result(m, work, level, true);
                self.add_noon_result(m, &r)?;
                clean += r.cleanliness;
                gorgeous += r.gorgeous;
                self.set_backup_apped(no, EVENING_WITHOUT_COMMUNICATION_BONUS, clean, gorgeous);
            }
            self.set_backup_apped(no, EVENING_WITH_COMMUNICATION_BONUS, clean, gorgeous);
        }
        self.backup_param(None, EVENING_WITH_COMMUNICATION_BONUS);
        Ok(())
    }

    /// `new WorkResultScene(Night)`: yotogi flags, the day's salon gain,
    /// night results, played-work flags and finished VIP works.
    pub fn work_result_night(&mut self) -> Result<()> {
        if self.player.flag("_Schedule_Night_Resulted") > 0 {
            return Ok(());
        }
        let data = self.data()?;
        self.player.generic_flag.insert("_Schedule_Night_Resulted".into(), 1);
        for no in 0..SCHEDULE_SLOTS {
            if let Some(m) = self.schedule_slot(no) {
                let played = i32::from(self.yotogi_played(no));
                self.set_maid_flag(m, "_YotogiPlayed", played);
            }
        }
        {
            let mut p = PlayerParam { st: &mut self.player, data: &data };
            p.add_salon_clean(data.schedule.def("dayAddSalonClean"));
            p.add_salon_beautiful(data.schedule.def("dayAddSalonBeautiful"));
        }
        for no in 0..SCHEDULE_SLOTS {
            let Some(m) = self.schedule_slot(no) else { continue };
            let work = self.night_id(m);
            let r = self.night_result(m, work);
            self.add_night_result(m, &r)?;
            self.set_backup_apped(no, MIDNIGHT, r.cleanliness, r.gorgeous);
        }
        self.backup_param(None, MIDNIGHT);
        for no in 0..SCHEDULE_SLOTS {
            let Some(m) = self.schedule_slot(no) else { continue };
            let work = self.night_id(m);
            self.set_maid_flag(m, &format!("_PlayedNightWorkId{work}"), 1);
            if matches!(data.schedule.night_type(work), Some(NIGHT_VIP | NIGHT_VIP_CALL)) {
                self.set_maid_flag(m, "_PlayedNightWorkVip", 1);
            }
        }
        for no in 0..SCHEDULE_SLOTS {
            let Some(m) = self.schedule_slot(no) else { continue };
            let work = self.night_id(m);
            if matches!(data.schedule.night_type(work), Some(NIGHT_VIP | NIGHT_VIP_CALL)) {
                if let Some(st) = self.player.night_works_state.get_mut(&work) {
                    st.finish = true;
                }
            }
        }
        Ok(())
    }
}

impl TjsState {
    /// `PlayerParam.CheckTrophyFromNoonResult` (`day`) or
    /// `CheckTrophyFromNightResult`; returns newly owned trophies.
    pub fn check_trophy_from_work_result(&mut self, day: bool) -> Result<Vec<i32>> {
        let data = self.data()?;
        let stock: Vec<crate::params::MaidStatus> = self.stock.iter().filter_map(|i| self.maids.get(i).cloned()).collect();
        let mut p = PlayerParam { st: &mut self.player, data: &data };
        let mut got = Vec::new();
        let mut add = |p: &mut PlayerParam, id: i32| {
            if p.add_have_trophy(id) {
                got.push(id);
            }
        };
        if 0 <= p.st.salon_loan {
            if 1_000_000 <= p.st.money {
                add(&mut p, 70);
            }
            if 10_000_000 <= p.st.money {
                add(&mut p, 80);
            }
        }
        if day {
            if ![110, 120, 130, 140, 150, 160].iter().all(|i| p.st.have_trophy.contains(i)) {
                // Novice, Lovely, Elegance, Sexy, Innocent, Ready.
                for m in &stock {
                    for (c, id) in [(0usize, 110), (1, 120), (2, 130), (3, 140), (4, 150), (6, 160)] {
                        if m.maid_class[c].is_have && 5 <= m.maid_class[c].level() {
                            add(&mut p, id);
                        }
                    }
                }
            }
        } else if ![170, 180, 190, 200, 210, 220, 230, 260, 270].iter().all(|i| p.st.have_trophy.contains(i)) {
            // Debut, Escort, Rudeness, Soap, Abnormal, Slave, Service.
            for m in &stock {
                for (c, id) in [(0usize, 170), (5, 180), (1, 190), (6, 200), (3, 210), (2, 220), (4, 230)] {
                    if m.yotogi_class[c].is_have && 5 <= m.yotogi_class[c].level() {
                        add(&mut p, id);
                    }
                }
                if 50 <= m.others_play_count {
                    add(&mut p, 260);
                }
                if 100 <= m.others_play_count {
                    add(&mut p, 270);
                }
            }
        }
        Ok(got)
    }
}

impl TjsState {
    /// `ScheduleAPI.CanCommunicationMaids`.
    pub fn communication_maids(&self) -> Vec<usize> {
        self.schedule_maids(true, &[NOON_BASIC, NOON_TRAINER, NOON_TRAINEE])
    }
    /// The trainee of a noon trainer or the trainer of a noon trainee
    /// (`CharacterSelectMain.OnFinish` for communication).
    pub fn training_partner(&self, m: usize) -> Option<usize> {
        let noon = self.noon_id(m);
        if noon == self.work_id_of(true, NOON_TRAINER) {
            self.slot_with(true, NOON_TRAINEE)
        } else if noon == self.work_id_of(true, NOON_TRAINEE) {
            self.slot_with(true, NOON_TRAINER)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_formulas_round_up() {
        assert_eq!(basic(10, 10, 0, 1.5), 17);
        assert_eq!(trainee(100, 10, 2), 5);
    }
}

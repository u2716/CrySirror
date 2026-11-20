// SPDX-License-Identifier: MPL-2.0
//! `Schedule.ScheduleCSVData`: noon/night work definitions, initial work
//! levels per personality, entertainment guests and `schedule_define.nei`.
use crate::{
    nei::Table,
    params::{MAID_CLASS, PROPENSITY, SALON_GRADE_MAX, SEIKEIKEN, YOTOGI_CLASS},
};
use anyhow::Result;
use std::collections::BTreeMap;

pub const NOON_TYPES: &[&str] = &["Basic", "Dispatch", "Trainer", "Trainee"];
pub const NIGHT_TYPES: &[&str] = &["Basic", "Rest", "Entertain", "HaveSex", "Trainer", "Trainee", "Vip", "VipCall"];
pub const NIGHT_ENTERTAIN: i32 = 2;
pub const NIGHT_HAVE_SEX: i32 = 3;
pub const NIGHT_VIP: i32 = 6;
pub const NIGHT_VIP_CALL: i32 = 7;
const CONTRACT: &[&str] = &["Nurture", "Free", "Exclusive"];

#[derive(Clone, Debug, Default)]
pub struct NoonWork {
    pub id: i32,
    pub name: String,
    pub icon: String,
    pub work_type: i32,
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
    pub income: i32,
    pub cmd_exp: i32,
    pub cond_contract: Vec<i32>,
    pub cond_maid_class: Vec<i32>,
}

#[derive(Clone, Debug, Default)]
pub struct NightWork {
    pub id: i32,
    pub name: String,
    pub icon: String,
    pub work_type: i32,
    pub information: String,
    pub income: i32,
    pub evaluation: i32,
    pub cond_info: Vec<String>,
    pub cond_salon_grade: i32,
    pub cond_skill: BTreeMap<i32, i32>,
    pub cond_contract: Vec<i32>,
    pub cond_maid_class: Vec<i32>,
    pub cond_yotogi_class: Vec<i32>,
    pub cond_propensity: Vec<i32>,
    pub cond_seikeiken: Vec<i32>,
    pub cond_flag1: Vec<String>,
    pub cond_flag0: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct EntertainGuest {
    pub cond_salon_grade: i32,
    pub base_income: i32,
    pub base_evaluation: i32,
    pub sexual: bool,
}

#[derive(Clone, Debug)]
pub struct ScheduleData {
    pub noon: BTreeMap<i32, NoonWork>,
    pub night: BTreeMap<i32, NightWork>,
    /// Per personality: work id → initial level.
    pub noon_init: [BTreeMap<i32, i32>; 3],
    pub night_init: [BTreeMap<i32, i32>; 3],
    pub entertain_number: Vec<i32>,
    pub entertain_guest: Vec<Vec<EntertainGuest>>,
    pub define: BTreeMap<&'static str, i32>,
}

/// `Enum.IsDefined` + `Parse` of a `|`/`&` separated cell (names only).
fn enums(cell: &str, sep: char, names: &[&str]) -> Vec<i32> {
    cell.split(sep).filter(|s| !s.is_empty()).filter_map(|s| names.iter().position(|n| *n == s).map(|i| i as i32)).collect()
}

const DEFINES: [(&str, i32); 15] = [
    ("dayAddSalonClean", 0), ("dayAddSalonBeautiful", 0), ("haveSexAddSalonClean", 0), ("haveSexAddSalonBeautiful", 0), ("entertainAddSalonClean", 0),
    ("entertainAddSalonBeautiful", 0), ("entertainParameterRevisionThreshold", 0), ("entertainPropensityRevisionMin", 0), ("entertainPropensityRevisionMax", 0),
    ("workSuccessLvPerfectIncidence", 10), ("workSuccessLvMissIncidence", 10), ("dispatchIncomCoefficient", 500), ("vipFullOpenDay", 100),
    ("noonWorkLv2Exp", 10), ("noonWorkLv3Exp", 30),
];

impl ScheduleData {
    pub fn empty() -> Self {
        Self {
            noon: BTreeMap::new(),
            night: BTreeMap::new(),
            noon_init: Default::default(),
            night_init: Default::default(),
            entertain_number: vec![0; SALON_GRADE_MAX as usize + 1],
            entertain_guest: vec![Vec::new(); SALON_GRADE_MAX as usize + 1],
            define: DEFINES.into_iter().collect(),
        }
    }

    pub fn load<'t>(table: impl Fn(&str) -> Result<&'t Table>) -> Result<Self> {
        let mut d = Self::empty();
        let t = table("schedule_work_noon.nei")?;
        for y in 1..t.rows {
            let mut n = 0;
            let mut c = || {
                n += 1;
                n - 1
            };
            let mut w = NoonWork { id: t.int(c(), y), name: t.string(c(), y), icon: t.string(c(), y), ..Default::default() };
            w.work_type = parse_name(NOON_TYPES, &t.string(c(), y));
            for f in [&mut w.study_rate, &mut w.reception, &mut w.care, &mut w.lovely, &mut w.elegance, &mut w.charm, &mut w.teach_rate, &mut w.exp, &mut w.cleanliness, &mut w.gorgeous, &mut w.income, &mut w.cmd_exp] {
                *f = t.int(c(), y);
            }
            w.cond_contract = enums(&t.string(c(), y), '|', CONTRACT);
            w.cond_maid_class = enums(&t.string(c(), y), '&', MAID_CLASS);
            if w.id > 0 {
                d.noon.insert(w.id, w);
            }
        }
        let t = table("schedule_work_night.nei")?;
        for y in 1..t.rows {
            let mut n = 0;
            let mut c = || {
                n += 1;
                n - 1
            };
            let mut w = NightWork { id: t.int(c(), y), name: t.string(c(), y), icon: t.string(c(), y), ..Default::default() };
            w.work_type = parse_name(NIGHT_TYPES, &t.string(c(), y));
            w.information = t.string(c(), y);
            w.income = t.int(c(), y);
            w.evaluation = t.int(c(), y);
            for _ in 0..9 {
                let s = t.string(c(), y);
                if !s.is_empty() {
                    w.cond_info.push(s);
                }
            }
            w.cond_salon_grade = t.int(c(), y);
            for p in t.string(c(), y).split('&').filter(|s| s.contains(',')) {
                let (a, b) = p.split_once(',').unwrap_or_default();
                w.cond_skill.insert(a.trim().parse()?, b.trim().parse()?);
            }
            w.cond_contract = enums(&t.string(c(), y), '|', CONTRACT);
            w.cond_maid_class = enums(&t.string(c(), y), '|', MAID_CLASS);
            w.cond_yotogi_class = enums(&t.string(c(), y), '|', YOTOGI_CLASS);
            w.cond_propensity = enums(&t.string(c(), y), '|', PROPENSITY);
            w.cond_seikeiken = enums(&t.string(c(), y), '|', SEIKEIKEN);
            w.cond_flag1 = t.string(c(), y).split('&').filter(|s| !s.is_empty()).map(str::to_owned).collect();
            w.cond_flag0 = t.string(c(), y).split('&').filter(|s| !s.is_empty()).map(str::to_owned).collect();
            if w.id > 0 {
                d.night.insert(w.id, w);
            }
        }
        for (file, init) in [("schedule_init_noon.nei", &mut d.noon_init), ("schedule_init_night.nei", &mut d.night_init)] {
            let t = table(file)?;
            for y in 1..t.rows {
                let id = t.int(0, y);
                for (p, map) in init.iter_mut().enumerate() {
                    let v = t.int(2 + p, y);
                    if v > 0 {
                        map.insert(id, v);
                    }
                }
            }
        }
        let t = table("schedule_entertain_number.nei")?;
        for y in 1..t.rows {
            if let Some(slot) = d.entertain_number.get_mut(t.int(0, y) as usize) {
                *slot = t.int(1, y);
            }
        }
        let t = table("schedule_entertain_guest.nei")?;
        for y in 1..t.rows {
            let g = EntertainGuest { cond_salon_grade: t.int(0, y), base_income: t.int(1, y), base_evaluation: t.int(2, y), sexual: t.string(3, y) == "○" };
            if let Some(list) = d.entertain_guest.get_mut(g.cond_salon_grade as usize) {
                list.push(g);
            }
        }
        let t = table("schedule_define.nei")?;
        for y in 0..t.rows {
            let k = t.string(0, y);
            if let Some((name, _)) = DEFINES.iter().find(|(n, _)| *n == k) {
                d.define.insert(name, t.int(1, y));
            }
        }
        Ok(d)
    }

    pub fn def(&self, k: &str) -> i32 {
        self.define.get(k).copied().unwrap_or(0)
    }
    pub fn noon_type(&self, id: i32) -> Option<i32> {
        self.noon.get(&id).map(|w| w.work_type)
    }
    pub fn night_type(&self, id: i32) -> Option<i32> {
        self.night.get(&id).map(|w| w.work_type)
    }
}

/// `Enum.IsDefined ? Parse : default(0)`.
fn parse_name(names: &[&str], s: &str) -> i32 {
    if s.is_empty() { 0 } else { names.iter().position(|n| *n == s).map_or(0, |i| i as i32) }
}


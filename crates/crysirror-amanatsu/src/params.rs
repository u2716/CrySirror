// SPDX-License-Identifier: MPL-2.0
//! `AL.User.GameParameter` of a heroine: favorability, inclusiveness,
//! proactivity and curiosity (`ParameterInfo`: Point, LV, IsMaxLv), the
//! favorability overflow `LatePoint`, and the command points of
//! `PropertyData._param` (`CommandPoint._ptns/_selects` ranges scaled by
//! `IndividualityRate[characteristic]`) as `EventManager.Talk` and
//! `CommandUI.AddPoint/AddPointItem` apply them.
use crysirror_unity::managed::{Fields, Value};

/// `PropertyData.PointParameter`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointParameter {
    pub max_lv: i32,
    pub lv_point: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ParameterInfo {
    pub point: i32,
    pub lv: i32,
    pub is_max_lv: bool,
}

impl ParameterInfo {
    /// `AddPointItem`: one level at most per add; the overflow is dropped
    /// and the gauge restarts at 0 (full at the last level). Returns
    /// whether the parameter levelled up.
    pub fn add(&mut self, add: i32, p: PointParameter) -> bool {
        if self.is_max_lv || add == 0 {
            return false;
        }
        let end = self.point + add;
        if end < p.lv_point {
            self.point = end.max(0);
            return false;
        }
        self.lv = (self.lv + 1).min(p.max_lv);
        self.is_max_lv = self.lv == p.max_lv;
        self.point = if self.is_max_lv { p.lv_point } else { 0 };
        true
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GameParameter {
    pub favorability: ParameterInfo,
    pub inclusiveness: ParameterInfo,
    pub proactivity: ParameterInfo,
    pub curiosity: ParameterInfo,
    pub late_point: i32,
    pub cost: i32,
}

/// `NPCData.Count` progress used by the favorability gates.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Count {
    pub massage: i32,
    pub h: i32,
    pub night_events: i32,
}

/// (inclusiveness, proactivity, curiosity) ranges of one pattern.
pub type Ranges = [(i32, i32); 3];

/// `PropertyData._param`.
#[derive(Clone, Debug, Default)]
pub struct ParamTable {
    /// Favorability, inclusiveness, proactivity, curiosity.
    pub points: Vec<PointParameter>,
    /// `CommandPoint` by kind (`Talk`, `Listen`, `HTopic`, …): (`_ptns`, `_selects`).
    pub commands: Vec<(String, Vec<Ranges>, Vec<Ranges>)>,
    /// `IndividualityRate` by card characteristic.
    pub rates: Vec<[f32; 3]>,
}

fn ranges(v: &Value) -> Option<Ranges> {
    let Value::Struct(f) = v else { return None };
    let r = |k: &str| f.fields(k).map_or((0, 0), |m| (m.int("Min").unwrap_or(0) as i32, m.int("Max").unwrap_or(0) as i32));
    Some([r("_inclusiveness"), r("_proactivity"), r("_curiosity")])
}

impl ParamTable {
    pub fn from_property_data(param: &Fields) -> Self {
        let points = param
            .array("Point")
            .unwrap_or(&[])
            .iter()
            .filter_map(|v| if let Value::Struct(f) = v { Some(PointParameter { max_lv: f.int("MaxLV").unwrap_or(4) as i32, lv_point: f.int("LVPoint").unwrap_or(100).max(1) as i32 }) } else { None })
            .collect();
        let mut commands = Vec::new();
        for (name, v) in &param.0 {
            let Value::Struct(f) = v else { continue };
            if f.get("_ptns").is_none() && f.get("_selects").is_none() {
                continue;
            }
            let list = |k: &str| f.array(k).unwrap_or(&[]).iter().filter_map(ranges).collect::<Vec<_>>();
            commands.push((name.clone(), list("_ptns"), list("_selects")));
        }
        let rates = param
            .array("IndividualityRate")
            .unwrap_or(&[])
            .iter()
            .filter_map(|v| if let Value::Struct(f) = v { Some(["_inclusiveness", "_proactivity", "_curiosity"].map(|k| f.float(k).unwrap_or(1.) as f32)) } else { None })
            .collect();
        Self { points, commands, rates }
    }

    fn point(&self, i: usize) -> PointParameter {
        self.points.get(i).copied().unwrap_or(PointParameter { max_lv: if i == 0 { 6 } else { 4 }, lv_point: 100 })
    }

    /// `EventManager.Talk`: the points of a `kind` command — the choice's
    /// `_selects` row when one was chosen (`GetSelect`), else a random
    /// `_ptns` pattern (`GetPtn`), each value uniform in [Min, Max] and
    /// scaled by the characteristic's rate (truncated).
    pub fn command_points(&self, kind: &str, select: Option<usize>, characteristic: usize, random: &mut dyn FnMut(usize) -> usize) -> [i32; 3] {
        let Some((_, ptns, selects)) = self.commands.iter().find(|c| c.0 == kind) else { return [0; 3] };
        let row = match select {
            Some(i) if i < selects.len() => Some(selects[i]),
            _ if !ptns.is_empty() => Some(ptns[random(ptns.len())]),
            _ => None,
        };
        let Some(row) = row else { return [0; 3] };
        let rate = self.rates.get(characteristic).copied().unwrap_or([0.; 3]);
        [0, 1, 2].map(|i| {
            let (lo, hi) = row[i];
            let v = if hi > lo { lo + random((hi - lo + 1) as usize) as i32 } else { lo };
            (v as f32 * rate[i]) as i32
        })
    }

    /// `CommandUI.AddPoint`: the three parameters take their points; each
    /// level-up adds half a level of favorability, and a full gauge keeps
    /// the excess as `LatePoint`. Returns the level-ups.
    pub fn add_points(&self, gp: &mut GameParameter, add: [i32; 3]) -> usize {
        let ups = [gp.inclusiveness.add(add[0], self.point(1)), gp.proactivity.add(add[1], self.point(2)), gp.curiosity.add(add[2], self.point(3))].iter().filter(|u| **u).count();
        if ups > 0 {
            let p = self.point(0);
            let end = gp.favorability.point + ups as i32 * (p.lv_point as f32 * 0.5) as i32;
            if end >= p.lv_point {
                gp.favorability.point = p.lv_point;
                gp.late_point = end - p.lv_point;
            } else {
                gp.favorability.point = end;
                gp.late_point = 0;
            }
        }
        ups
    }
}

/// `CommandUI.IsNextShortMovie`: whether a full favorability gauge may
/// level up (the `with` short movie) given the massage/H progress.
pub fn favorability_gate(f: &ParameterInfo, count: &Count) -> bool {
    match f.lv {
        0 => count.massage >= 1,
        1 | 2 => count.h >= 1,
        3 | 4 => count.massage >= 1 && count.h >= 1,
        5 => count.h >= 2,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_level_up_and_fill_favorability() {
        let p = PointParameter { max_lv: 4, lv_point: 100 };
        let mut i = ParameterInfo::default();
        assert!(!i.add(60, p));
        assert_eq!(i.point, 60);
        assert!(i.add(60, p));
        assert_eq!((i.lv, i.point), (1, 0));
        assert!(!i.add(-10, p));
        assert_eq!(i.point, 0);
        let table = ParamTable { points: vec![PointParameter { max_lv: 6, lv_point: 100 }, p, p, p], ..Default::default() };
        let mut gp = GameParameter::default();
        gp.inclusiveness.point = 90;
        gp.proactivity.point = 90;
        gp.curiosity.point = 90;
        assert_eq!(table.add_points(&mut gp, [20, 20, 20]), 3);
        assert_eq!((gp.favorability.point, gp.late_point), (100, 50));
        assert!(favorability_gate(&ParameterInfo { lv: 1, ..Default::default() }, &Count { h: 1, ..Default::default() }));
        assert!(!favorability_gate(&ParameterInfo::default(), &Count::default()));
    }

    #[test]
    fn command_points_use_the_choice_row_and_rate() {
        let table = ParamTable { commands: vec![("Listen".into(), vec![], vec![[(15, 15), (0, 0), (0, 0)], [(0, 0), (20, 20), (0, 0)]])], rates: vec![[1., 1., 1.], [1.2, 0.85, 1.]], ..Default::default() };
        let mut r = |_: usize| 0;
        assert_eq!(table.command_points("Listen", Some(1), 1, &mut r), [0, 17, 0]);
        assert_eq!(table.command_points("Listen", Some(0), 1, &mut r), [18, 0, 0]);
        assert_eq!(table.command_points("Talk", None, 0, &mut r), [0, 0, 0]);
    }
}

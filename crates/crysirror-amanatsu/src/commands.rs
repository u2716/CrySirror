// SPDX-License-Identifier: MPL-2.0
//! `EventManager` scenario choice for the command menu (`Get*Asset` over
//! `PropertyData._adv`): each command gathers candidate assets by
//! favourability, the map (`_mapID`) and the time zone, then plays a random
//! asset of the lowest `Priority`; and the heroine's command cost
//! (`ExploreProperties.Cost`: `GetMax`, `ConsumptionCost`, `RecoveryCost`).
use crysirror_unity::managed::{Fields, Value};
use std::collections::HashMap;

/// `PropertyData_ADV.AssetProperty`: (asset, priority).
pub type Asset = (String, i64);

/// `PropertyData._adv` groups (`_talk`, `_listen`, `_hTopic`, `_agree`,
/// `_end`, `_preludes`, …): key → asset.
#[derive(Clone, Debug, Default)]
pub struct AdvAssets {
    groups: HashMap<String, HashMap<String, Asset>>,
}

/// `PropertyData._mapID` keys of the map-specific assets.
const MAP_KEYS: [(&str, &str); 6] = [("Pool", "Pool"), ("Beach", "Beach"), ("Cottage", "Cottage"), ("Shopping", "Shopping"), ("Waterfall", "Waterfall"), ("FountainPlaza", "Fountain")];
const ZONE_KEYS: [&str; 4] = ["Morning", "Afternoon", "Evening", "Night"];

impl AdvAssets {
    pub fn from_property_data(adv: &Fields, map_ids: Option<&Fields>) -> (Self, HashMap<i32, &'static str>) {
        let mut groups = HashMap::new();
        for (name, v) in &adv.0 {
            // Top-level asset lists (`_shortMoiveAfterAssets`): `#index`.
            if let Value::Array(list) = v {
                let g: HashMap<String, Asset> = list
                    .iter()
                    .enumerate()
                    .filter_map(|(i, a)| if let Value::Struct(a) = a { Some((format!("#{i}"), (a.string("Asset")?.to_owned(), a.int("Priority").unwrap_or(0)))) } else { None })
                    .collect();
                groups.insert(name.clone(), g);
                continue;
            }
            let Value::Struct(f) = v else { continue };
            let mut g = HashMap::new();
            for (k, a) in &f.0 {
                match a {
                    Value::Struct(a) => {
                        if let Some(asset) = a.string("Asset") {
                            g.insert(k.clone(), (asset.to_owned(), a.int("Priority").unwrap_or(0)));
                        }
                    }
                    // Asset lists (`Evaluation`): `name#index`.
                    Value::Array(list) => {
                        for (i, a) in list.iter().enumerate() {
                            if let Value::Struct(a) = a {
                                if let Some(asset) = a.string("Asset") {
                                    g.insert(format!("{k}#{i}"), (asset.to_owned(), a.int("Priority").unwrap_or(0)));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            groups.insert(name.clone(), g);
        }
        let maps = MAP_KEYS.iter().filter_map(|(id_key, asset_key)| Some((map_ids?.int(id_key)? as i32, *asset_key))).collect();
        (Self { groups }, maps)
    }

    fn get(&self, group: &str, key: &str) -> Option<Asset> {
        self.groups.get(group)?.get(key).filter(|a| !a.0.is_empty()).cloned()
    }
}

/// What the choice depends on.
pub struct Context {
    pub favorability_lv: i32,
    /// `PropertyData._mapID` asset key of the current map.
    pub map_key: Option<&'static str>,
    pub zone: i32,
}

/// The lowest-priority candidates, one at random (`Min(Priority)` then a
/// random element of the matching assets).
pub fn choose(candidates: &[Asset], random: &mut dyn FnMut(usize) -> usize) -> Option<String> {
    let min = candidates.iter().map(|c| c.1).min()?;
    let best: Vec<&Asset> = candidates.iter().filter(|c| c.1 == min).collect();
    Some(best[random(best.len())].0.clone())
}

impl AdvAssets {
    /// `GetTalkAsset` / `GetListenAsset`: the general assets, the
    /// favourability asset from LV 4, the map's and the time zone's assets
    /// (talk adds the food/tourism/leisure topics). SATD-amanatsu-port: the
    /// heroine-motion assets (`Sleeping`, `Reading`, …) and `WaterGunAfter`
    /// need action motions and the water gun.
    pub fn talk(&self, group: &str, ctx: &Context) -> Vec<Asset> {
        let mut c = Vec::new();
        let mut add = |k: &str| c.extend(self.get(group, k));
        add("General_1");
        add("General_2");
        if ctx.favorability_lv >= 4 {
            add(if group == "_listen" { "Romance" } else { "Favorability" });
        }
        if let Some(m) = ctx.map_key {
            add(m);
        }
        add(ZONE_KEYS[ctx.zone.clamp(0, 3) as usize]);
        if group == "_talk" {
            for k in ["FoodAndBeverage", "Tourism", "Leisure"] {
                add(k);
            }
        }
        c
    }

    /// `GetHTopicQuestionAsset` / `GetHTopicAsset` (a 50% choice between
    /// them): the three questions/topics (and the favourability/curiosity
    /// ones from LV 4). SATD-amanatsu-port: the `*_Fin` assets after H acts.
    pub fn h_topic(&self, question: bool, favorability_lv: i32, curiosity_lv: i32) -> Vec<Asset> {
        let mut c = Vec::new();
        let prefix = if question { "Question" } else { "Topic" };
        for i in 1..=3 {
            c.extend(self.get("_hTopic", &format!("{prefix}_{i}")));
        }
        if question && favorability_lv >= 4 {
            c.extend(self.get("_hTopic", "Question_Favorability"));
        }
        if !question && curiosity_lv >= 4 {
            c.extend(self.get("_hTopic", "Topic_Curiosity"));
        }
        c
    }

    /// `GetGeneralCommandAgreeAsset`: above favourability LV 3 the
    /// command's own pair (`Takl_*`, `Listen_*`, `HTopic_*`), else the
    /// general pair; `Exhaustion` without cost.
    pub fn agree(&self, kind: &str, favorability_lv: i32, exhausted: bool) -> Vec<Asset> {
        if exhausted {
            return self.get("_agree", "Exhaustion").into_iter().collect();
        }
        if kind == "H" {
            let level = if favorability_lv > 3 { "H_High" } else { "H_General" };
            return (1..=2).filter_map(|i| self.get("_agree", &format!("{level}_{i}"))).collect();
        }
        if kind == "Massage" {
            let level = if favorability_lv > 3 { "High" } else { "Normal" };
            return (1..=2).filter_map(|i| self.get("_agree", &format!("Massage_{level}_{i}"))).collect();
        }
        let prefix = match kind {
            "Talk" if favorability_lv > 3 => "Takl",
            "Listen" if favorability_lv > 3 => "Listen",
            "HTopic" if favorability_lv > 3 => "HTopic",
            "With" if favorability_lv > 3 => "With",
            _ => "General",
        };
        (1..=2).filter_map(|i| self.get("_agree", &format!("{prefix}_{i}"))).collect()
    }

    /// `GetMassageStartAsset`: `Before_Normal_*` / `Before_High_*`.
    pub fn massage_before(&self, favorability_lv: i32) -> Vec<Asset> {
        let level = if favorability_lv > 3 { "High" } else { "Normal" };
        (1..=2).filter_map(|i| self.get("_massage", &format!("Before_{level}_{i}"))).collect()
    }

    /// `_shortMoiveAfterAssets[lv]`: the scenario after the favourability
    /// short movie of level `lv`.
    pub fn short_movie_after(&self, lv: i32) -> Vec<Asset> {
        self.get("_shortMoiveAfterAssets", &format!("#{lv}")).into_iter().collect()
    }

    /// `GetHStartAsset`: the general H openers, or `Map_No_Here` when the
    /// map has no H place (the heroine proposes moving).
    pub fn h_start(&self, here: bool) -> Vec<Asset> {
        if !here {
            return self.get("_h", "Map_No_Here").into_iter().collect();
        }
        (1..=4).filter_map(|i| self.get("_h", &format!("General_{i}"))).collect()
    }

    /// `_event` assets of `NightEvent` (`Before_Night_Cottage`, `Night_H_After`, …).
    pub fn event(&self, key: &str) -> Option<String> {
        self.get("_event", key).map(|a| a.0)
    }

    /// `_h.Event_After` (after a morning H).
    pub fn h_event_after(&self) -> Option<String> {
        self.get("_h", "Event_After").map(|a| a.0)
    }

    /// The H result's `Evaluation[index]` asset.
    pub fn h_evaluation(&self, index: usize) -> Vec<Asset> {
        self.get("_h", &format!("Evaluation#{index}")).into_iter().collect()
    }

    /// The massage result's `Evaluation[stage]` asset.
    pub fn massage_evaluation(&self, stage: usize) -> Vec<Asset> {
        self.get("_massage", &format!("Evaluation#{stage}")).into_iter().collect()
    }

    /// The `_end` assets: `DoNothing` (priority 0) when no command was
    /// played, `Exhaustion` without cost, else the general ends by
    /// favourability. SATD-amanatsu-port: the H-after, water-gun-only and
    /// move-map ends.
    pub fn end(&self, played: bool, exhausted: bool, favorability_lv: i32) -> Vec<Asset> {
        if !played {
            return self.get("_end", "DoNothing").into_iter().collect();
        }
        if exhausted {
            return self.get("_end", "Exhaustion").into_iter().collect();
        }
        let level = if favorability_lv > 3 { "High" } else { "Normal" };
        (1..=2).filter_map(|i| self.get("_end", &format!("General_{level}_{i}"))).collect()
    }
}

/// `ExploreProperties.Cost`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CostProperty {
    pub default_max: i32,
    pub max: i32,
}

impl CostProperty {
    pub fn from_property_data(explore: Option<&Fields>) -> (Self, Vec<i32>) {
        let c = explore.and_then(|e| e.fields("Cost"));
        let ups = c.and_then(|c| c.array("_costUpLVs")).unwrap_or(&[]).iter().filter_map(|v| if let Value::Int(i) = v { Some(*i as i32) } else { None }).collect();
        (Self { default_max: c.and_then(|c| c.int("_defaultMaxCost")).unwrap_or(5) as i32, max: c.and_then(|c| c.int("_maxCost")).unwrap_or(7) as i32 }, ups)
    }

    /// `GetMax(lv)`: the default plus one per `_costUpLVs` level passed,
    /// at most `_maxCost`.
    pub fn get_max(&self, up_lvs: &[i32], lv: i32) -> i32 {
        let mut m = self.default_max;
        for (i, up) in up_lvs.iter().enumerate() {
            if lv > *up {
                m = self.max.min(i as i32 + self.default_max + 1);
            }
        }
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowest_priority_wins_and_cost_grows_with_favourability() {
        let mut r = |n: usize| n - 1;
        assert_eq!(choose(&[("a".into(), 10), ("b".into(), 1), ("c".into(), 1)], &mut r).as_deref(), Some("c"));
        assert_eq!(choose(&[], &mut r), None);
        let c = CostProperty { default_max: 5, max: 7 };
        assert_eq!([0, 2, 3, 4, 5, 6].map(|lv| c.get_max(&[2, 4], lv)), [5, 5, 6, 6, 7, 7]);
    }
}

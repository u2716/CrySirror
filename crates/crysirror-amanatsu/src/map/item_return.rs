// SPDX-License-Identifier: MPL-2.0
use super::{ClientId, Game, HostServices, game_random};
use crate::{map::wandering, tables::Table, unity::Unity};
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};

/// EventManager.GetHeroine (BF3330) shuffles the eligible list with no
/// conversation in this time zone, falling back to all eligible heroines.
pub(super) fn choose_heroine<T>(
    roster: Vec<(T, i32, i32)>,
    mut random: impl FnMut(usize) -> usize,
) -> Option<T> {
    let mut eligible: Vec<_> = roster
        .into_iter()
        .filter(|(_, level, _)| *level >= 2)
        .collect();
    if eligible.iter().any(|(_, _, mask)| mask & 7 == 0) {
        eligible.retain(|(_, _, mask)| mask & 7 == 0);
    }
    for length in (2..=eligible.len()).rev() {
        eligible.swap(length - 1, random(length));
    }
    eligible.into_iter().next().map(|(actor, _, _)| actor)
}

/// PropertyData.ExploreProperties.MotionProperty.GetActionAnimationID
/// (C865B0) and ALCommandPoseActionContent (CF43D0).
pub(super) struct Motions {
    idle: [i32; 6],
    elbow: [i32; 2],
    elbow_voice: [i32; 4],
    standby: HashMap<i32, i32>,
}

impl Motions {
    fn load(unity: &mut Unity) -> Result<Self> {
        let pd = unity
            .find_script_object("StreamingAssets/aa", "AL.PropertyData")?
            .context("AL.PropertyData")?;
        let motion = pd
            .fields("_explore")
            .and_then(|f| f.fields("_motion"))
            .context("explore._motion")?;
        let id = |key| {
            motion
                .int(key)
                .map(|v| v as i32)
                .with_context(|| format!("missing motion {key}"))
        };
        let mut standby = HashMap::new();
        for key in unity
            .bundle_files()
            .into_iter()
            .filter(|k| k.starts_with("bundle:lib/am/lis/") && !k.ends_with(".resS"))
        {
            let assets: Vec<_> = unity
                .file(&key)?
                .objects
                .iter()
                .filter(|o| o.class_id == 49)
                .map(|o| o.path_id)
                .collect();
            for path_id in assets {
                if !unity
                    .decode(&key, path_id)?
                    .string("m_Name")
                    .is_some_and(|n| n.starts_with("al_command_action["))
                {
                    continue;
                }
                let t = Table::parse(&unity.text_asset(&key, path_id)?)?;
                let key_col = t
                    .keys
                    .iter()
                    .position(|&k| k == 5)
                    .context("al_command_action ID")?;
                let value_col = t
                    .keys
                    .iter()
                    .position(|&k| k == 71)
                    .context("al_command_action AnimID")?;
                for row in t.rows {
                    let key = row
                        .get(key_col)
                        .context("action ID missing")?
                        .trim()
                        .parse()?;
                    if let Some(value) = row
                        .get(value_col)
                        .and_then(|s| s.trim().parse::<i32>().ok())
                    {
                        standby.insert(key, value);
                    }
                }
            }
        }
        Ok(Self {
            idle: [
                id("Default_IN")?,
                id("Default")?,
                id("PoolIdle_SM")?,
                id("PoolIdle_ML_IN")?,
                id("PoolIdle_ML")?,
                id("SeaIdle")?,
            ],
            elbow: [id("PoolSide_Elbow_SM")?, id("PoolSide_Elbow_ML")?],
            elbow_voice: [
                id("PoolSide_Elbow_SM_Voice_Loop")?,
                id("PoolSide_Elbow_SM_Voice_Action")?,
                id("PoolSide_Elbow_ML_Voice_Loop")?,
                id("PoolSide_Elbow_ML_Voice_Action")?,
            ],
            standby,
        })
    }

    fn resolve(&self, id: i32, area: i32, height: f32) -> i32 {
        let tall = usize::from(height >= 0.5);
        if self.elbow.contains(&id) {
            return self.elbow[tall];
        }
        if self.elbow_voice.contains(&id) {
            return self.elbow_voice[tall * 2];
        }
        let id = if id == -1 || self.idle.contains(&id) {
            id
        } else {
            self.standby.get(&id).copied().unwrap_or(id)
        };
        if id != -1 && !self.idle.contains(&id) {
            return id;
        }
        match area {
            1 => self.idle[2 + tall],
            2 => self.idle[5],
            _ => self.idle[1],
        }
    }
}

impl Game {
    /// ItemEventEnd(C02A60) calls SetHeroineWanderingPoint(F0A7B0)
    /// and ChangeState(ThinkingWait) for the event heroine on this map.
    pub(super) fn resume_item_heroine(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        card: &str,
    ) -> Result<()> {
        let Some(pop) = &mut self.walks else {
            return Ok(());
        };
        if !pop.actors.contains_key(card) {
            return Ok(());
        }
        let occupied: HashSet<i32> = pop
            .actors
            .values()
            .map(|actor| pop.points[actor.point].id)
            .collect();
        let index = pop
            .order
            .next(&pop.points, &occupied)
            .context("item return has no wandering point")?;
        let point = pop.points[index].clone();
        let height = self
            .standing
            .get(card)
            .context("item return actor transform")?
            .2;
        let personality = self
            .spots
            .iter()
            .find(|s| s.0 == card)
            .context("item return actor")?
            .2;
        if self.item_return_motions.is_none() {
            self.item_return_motions = Some(Motions::load(&mut self.unity)?);
        }
        let picked = self.point_motion_id(&point.motions, -1).unwrap_or(-1);
        let id = self
            .item_return_motions
            .as_ref()
            .context("item return motions")?
            .resolve(picked, point.area, height);
        let state =
            crate::tables::motion_state(&mut self.unity, &format!("anim_f_{personality:03}"), id)?
                .with_context(|| format!("item return motion {id} missing for {card}"))?;
        self.map_actor_transform(card, point.position, point.rotation);
        self.set_cloth_underwater(card, point.area != 0);
        self.map_change_motion(host, client, card, &state)?;
        self.extra(card).action_motion_id = id;
        let wait = self
            .walk_rules
            .wait(true, |n| game_random(&mut self.rng, n));
        let actor = self
            .walks
            .as_mut()
            .and_then(|p| p.actors.get_mut(card))
            .context("item return actor removed")?;
        actor.point = index;
        actor.phase = wandering::Phase::Thinking(wait);
        self.log.push(format!(
            "item return point {} {:?}: motion {id} {state}, ThinkingWait {wait}s",
            point.id, point.position
        ));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn item_heroine_prefers_unvisited_and_shuffles_only_that_list() {
        let mut calls = Vec::new();
        let selected = choose_heroine(
            vec![
                ("low", 1, 0),
                ("talked", 4, 1),
                ("other_activity", 2, 8),
                ("fresh", 3, 0),
            ],
            |n| {
                calls.push(n);
                0
            },
        );
        assert_eq!(selected, Some("fresh"));
        assert_eq!(calls, [2]);
        calls.clear();
        let selected = choose_heroine(vec![("a", 2, 2), ("b", 3, 4), ("c", 4, 5)], |n| {
            calls.push(n);
            0
        });
        assert_eq!(selected, Some("b"));
        assert_eq!(calls, [3, 2]);
    }

    #[test]
    fn no_choice_does_not_consume_the_event_roll() {
        for mask in [0, 1, 2, 4] {
            assert_eq!(
                choose_heroine(vec![("eligible", 2, mask), ("low", 1, 0)], |_| panic!(
                    "unexpected random draw"
                )),
                Some("eligible")
            );
        }
        assert_eq!(
            choose_heroine(vec![("low", 1, 0)], |_| panic!("unexpected random draw")),
            None
        );
    }

    #[test]
    fn return_pose_selects_authored_standby_and_area_or_height_variant() {
        let m = Motions {
            idle: [10, 11, 12, 13, 14, 15],
            elbow: [20, 21],
            elbow_voice: [30, 31, 32, 33],
            standby: [(50, 11), (51, 55), (20, 99)].into_iter().collect(),
        };
        assert_eq!(m.resolve(-1, 0, 0.1), 11);
        assert_eq!(m.resolve(50, 1, 0.499), 12);
        assert_eq!(m.resolve(50, 1, 0.5), 13);
        assert_eq!(m.resolve(14, 2, 0.1), 15);
        assert_eq!(m.resolve(20, 1, 0.6), 21);
        assert_eq!(m.resolve(33, 1, 0.4), 30);
        assert_eq!(m.resolve(51, 2, 0.6), 55);
        assert_eq!(m.resolve(99, 2, 0.6), 99);
    }
}

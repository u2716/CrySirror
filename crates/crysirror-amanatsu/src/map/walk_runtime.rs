// SPDX-License-Identifier: MPL-2.0
use super::{ClientId, Controller, Game, HostServices, actionmap, game_random, unity_transform};
use crate::map::wandering::{Actor, Phase, Population};
use anyhow::{Context, Result, ensure};
use glam::Quat;
use serde_json::json;

impl Game {
    pub(super) fn update_map_walks(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        dt: f32,
    ) {
        if !matches!(&self.controller,Controller::ActionMap(m) if matches!(m.phase,actionmap::Phase::Map(_)))
            || self.overlays.iter().any(|(l, _)| l.name != "ActionMapUI")
            || self.pending.is_some()
            || self.pending_map.is_some()
            || self.fade.fading(self.now)
            || self.talk.is_some()
            || self.command.is_some()
            || self.item.is_some()
            || self.movie.is_some()
            || self.h.is_some()
            || self.night.is_some()
            || self.massage.is_some()
        {
            return;
        }
        let Some(mut pop) = self.walks.take() else {
            return;
        };
        let cards: Vec<_> = pop.actors.keys().cloned().collect();
        for card in cards {
            if !self.spots.iter().any(|s| s.0 == card) {
                continue;
            }
            let Some(mut actor) = pop.actors.remove(&card) else {
                continue;
            };
            if let Err(e) = self.map_actor_step(host, client, &mut pop, &card, &mut actor, dt) {
                self.log.push(format!("map wandering {card}: {e:#}"));
                actor.phase = Phase::Unsupported;
            }
            pop.actors.insert(card, actor);
        }
        self.walks = Some(pop);
    }

    fn map_actor_step(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        pop: &mut Population,
        card: &str,
        actor: &mut Actor,
        dt: f32,
    ) -> Result<()> {
        let Some((_, _, personal, _)) = self.spots.iter().find(|s| s.0 == card) else {
            return Ok(());
        };
        let personal = *personal;
        let point = &pop.points[actor.point];
        match &mut actor.phase {
            Phase::Action(left) | Phase::Thinking(left) if *left > dt => {
                *left -= dt;
            }
            Phase::Action(_) => {
                actor.phase = Phase::Thinking(
                    self.walk_rules
                        .wait(true, |n| game_random(&mut self.rng, n)),
                );
            }
            Phase::Thinking(_) => {
                let moving = 1 + self.random(100) as i32 <= self.walk_rules.move_ratio;
                let target = if moving {
                    let mut occupied: std::collections::HashSet<_> = pop
                        .actors
                        .values()
                        .map(|a| pop.points[a.point].id)
                        .collect();
                    occupied.insert(point.id);
                    pop.order
                        .next(&pop.points, &occupied)
                        .context("no wandering destination")?
                } else {
                    actor.point
                };
                let dest = &pop.points[target];
                let previous = self.extra(card).next_motion_id;
                let (next, state) = self
                    .point_activity(personal, &dest.motions, previous)
                    .context("unresolved next ActionPoint motion")?;
                self.extra(card).next_motion_id = next;
                if target == actor.point {
                    self.map_change_motion(host, client, card, &state)?;
                    self.extra(card).action_motion_id = next;
                    self.log
                        .push(format!("map follow-up at {}: {next} {state}", dest.id));
                    actor.phase = Phase::Action(
                        self.walk_rules
                            .wait(false, |n| game_random(&mut self.rng, n)),
                    );
                } else {
                    ensure!(
                        !dest.spline && point.area == 0 && dest.area == 0,
                        "transition {} → {} requires a spline or water-area link",
                        point.id,
                        dest.id
                    );
                    let (_, _, height) = self
                        .standing
                        .get(card)
                        .copied()
                        .context("missing map actor transform")?;
                    let mut path = pop
                        .nav
                        .route(point.destination, dest.destination, 3.)
                        .with_context(|| format!("no connected navigation corridor {} {:?} → {} {:?}",point.id,point.destination,dest.id,dest.destination))?;
                    let start = path[0];
                    path.remove(0);
                    let (motion, speed) = self.walk_rules.locomotion(point.area, height, personal);
                    let walk = crate::tables::motion_state(
                        &mut self.unity,
                        &format!("anim_f_{personal:03}"),
                        motion,
                    )?
                    .context("missing locomotion state")?;
                    self.map_change_motion(host, client, card, &walk)?;
                    self.log.push(format!(
                        "map walk {} → {}: {} corridor points at {speed:.2} units/s, {walk}",
                        point.id,
                        dest.id,
                        path.len()
                    ));
                    actor.point = target;
                    actor.phase = Phase::Moving {
                        path: path.into(),
                        position: start,
                        speed,
                    };
                }
            }
            Phase::Moving {
                path,
                position,
                speed,
            } => {
                let mut budget = dt * *speed;
                let start = *position;
                while let Some(&target) = path.front() {
                    let diff = target - *position;
                    let length = diff.length();
                    if length > budget {
                        *position += diff * (budget / length);
                        break;
                    }
                    *position = target;
                    budget -= length;
                    path.pop_front();
                }
                if path.is_empty() {
                    self.map_actor_transform(card, point.position, point.rotation);
                    self.set_cloth_underwater(card, point.area != 0);
                    let next = self.extra(card).next_motion_id;
                    let state = crate::tables::motion_state(
                        &mut self.unity,
                        &format!("anim_f_{personal:03}"),
                        next,
                    )?
                    .context("missing arrival state")?;
                    self.map_change_motion(host, client, card, &state)?;
                    self.extra(card).action_motion_id = next;
                    self.log.push(format!(
                        "map arrived {} {:?}: {next} {state}",
                        point.id, point.position
                    ));
                    actor.phase = Phase::Action(
                        self.walk_rules
                            .wait(false, |n| game_random(&mut self.rng, n)),
                    );
                } else if *position != start {
                    let dir = *position - start;
                    let rot = Quat::from_rotation_y(dir.x.atan2(dir.z));
                    self.map_actor_transform(card, position.to_array(), rot.to_array());
                }
            }
            Phase::Unsupported => {}
        }
        Ok(())
    }

    pub(super) fn map_actor_transform(&mut self, card: &str, pos: [f32; 3], rot: [f32; 4]) {
        if let Some(s) = self.standing.get_mut(card) {
            (s.0, s.1) = (pos, rot);
        }
        self.chara_roots.insert(card.to_owned(), (pos, rot));
        if let Some(s) = self.spots.iter_mut().find(|s| s.0 == card) {
            s.1 = crysirror_unity::gltf::mirror_point(pos);
        }
        let transform = unity_transform(pos, rot);
        for id in self.character_entities(card) {
            self.world
                .push(json!({"op":"set_transform","id":id,"transform":transform}));
        }
    }

    pub(super) fn map_change_motion(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        card: &str,
        state: &str,
    ) -> Result<()> {
        let (pos, rot, _) = self
            .standing
            .get(card)
            .copied()
            .context("map actor transform")?;
        let (bundle, controller) = crate::chara::motion_controller(true);
        self.despawn_character(host, client, card)?;
        self.spawn_character_motion(
            host,
            client,
            card,
            Some(unity_transform(pos, rot)),
            self.map_sun,
            Some((&bundle, controller, state)),
        )?;
        Ok(())
    }
}

// SPDX-License-Identifier: MPL-2.0
//! Type 0x55 controls three atlases through the native projected-sprite camera.

use super::{hidden_layer, hundredths, sine, SceneSound};
use crate::adapter::visual::projected_atlas_cell;
use anyhow::Result;
use crysirror_core::service::UploadedImage;
use glam::Vec3;
use serde_json::Value;

pub(in crate::adapter) const TEXTURES: [&str; 3] = ["ci001a.pb3", "ci001b.pb3", "ci001c.pb3"];
pub(in crate::adapter) const SAMPLES: &[(usize, &str)] = &[(16, "SE035a.ogg"), (17, "SE037b.ogg")];

#[derive(Clone, Copy)]
struct Turn {
    ms: u32,
    end: i32,
    flags: u16,
    next: Option<usize>,
}

const fn turn(ms: u32, degrees: i32, flags: u16, next: usize) -> Turn {
    Turn {
        ms,
        end: 100 * degrees,
        flags,
        next: Some(next),
    }
}

const SPIN: &[Turn] = &[Turn {
    ms: 1000,
    end: 0,
    flags: 0,
    next: None,
}];
const ROCK: &[Turn] = &[
    turn(500, 5, 1, 1),
    turn(1000, -5, 0x8001, 2),
    turn(1000, 5, 1, 1),
];
const JUMP: &[Turn] = &[
    turn(100, 5, 0, 1),
    turn(100, 0, 0x8000, 2),
    turn(100, 5, 0, 3),
    turn(100, 0, 0x8000, 4),
    turn(300, 0, 255, 0),
];
const LAND: &[Turn] = &[
    turn(400, 0, 0, 1),
    turn(400, 0, 0, 2),
    turn(500, 5, 1, 3),
    turn(1000, -5, 0x8001, 4),
    turn(1000, 5, 1, 3),
];
const LAND_REVERSE: &[Turn] = &[
    turn(400, 0, 0x8000, 1),
    turn(400, 0, 0x8000, 2),
    turn(500, 5, 1, 3),
    turn(1000, -5, 0x8001, 4),
    turn(1000, 5, 1, 3),
];

struct Rotation {
    turns: &'static [Turn],
    step: usize,
    elapsed: u32,
    origin: i32,
}

impl Rotation {
    fn advance(&mut self, delta: u32, angle: &mut i32) -> bool {
        self.elapsed += delta;
        loop {
            let turn = self.turns[self.step];
            let mode = turn.flags as u8;
            if self.elapsed < turn.ms {
                if mode <= 1 {
                    let reverse = turn.flags & 0x8000 != 0;
                    let diff = if reverse {
                        self.origin - turn.end
                    } else {
                        turn.end - self.origin
                    };
                    let distance = if diff <= 0 { diff + 36000 } else { diff };
                    let step = if mode == 0 {
                        self.elapsed as i32 * distance / turn.ms as i32
                    } else {
                        let radians = f64::from(9000 * self.elapsed / turn.ms) / 18_000.
                            * std::f64::consts::PI;
                        (radians.sin() * f64::from(distance)) as i32
                    };
                    *angle = (self.origin + if reverse { -step } else { step }).rem_euclid(36000);
                }
                return true;
            }
            if mode < 128 {
                self.origin = turn.end;
                *angle = self.origin;
            }
            let Some(next) = turn.next else { return false };
            self.elapsed -= turn.ms;
            self.step = next;
            if self.elapsed < self.turns[next].ms {
                return true;
            }
        }
    }
}

struct Movement {
    from: Vec3,
    to: Vec3,
    ms: u32,
    elapsed: u32,
    ease: bool,
}

#[derive(Default)]
struct Actor {
    active: bool,
    cell: i16,
    cell_ms: u32,
    angle: i32,
    position: Vec3,
    target: Vec3,
    motion: Option<Movement>,
    rotation: Option<Rotation>,
}

impl Actor {
    fn initialize(&mut self, position: Vec3) {
        *self = Self {
            active: true,
            position,
            target: position,
            ..Self::default()
        };
    }

    fn select_cell(&mut self, cell: i16) {
        self.cell = cell;
        self.cell_ms = 0;
    }

    fn set_position(&mut self, position: Vec3) {
        self.position = position;
        self.target = position;
    }

    fn move_to(&mut self, to: [f32; 3], ms: u32, ease: bool) {
        let to = Vec3::from_array(to);
        self.motion = Some(Movement {
            from: self.position,
            to,
            ms,
            elapsed: 0,
            ease,
        });
        self.target = to;
    }

    fn turn(&mut self, turns: &'static [Turn]) {
        self.rotation = Some(Rotation {
            turns,
            step: 0,
            elapsed: 0,
            origin: 0,
        });
    }

    fn clear_rotation(&mut self) {
        self.rotation = None;
        self.angle = 0;
    }

    fn advance(&mut self, delta: u32) {
        if !self.active {
            return;
        }
        let delta = delta.min(200);
        self.cell_ms += delta;
        while self.cell_ms >= 100 {
            self.cell_ms -= 100;
            self.cell = (self.cell / 4) * 4 + (self.cell + 1) % 4;
        }
        if let Some(motion) = &mut self.motion {
            motion.elapsed += delta;
            if motion.elapsed >= motion.ms {
                self.position = motion.to;
                self.motion = None;
            } else {
                let t = motion.elapsed as f32 / motion.ms as f32;
                let offset = motion.to - motion.from;
                let quantize = |v: Vec3| Vec3::from_array(v.to_array().map(hundredths));
                self.position = if motion.ease {
                    motion.to - quantize(offset * (1. - t) * (1. - t))
                } else {
                    motion.from + quantize(offset * t)
                };
            }
        }
        if let Some(rotation) = &mut self.rotation {
            if !rotation.advance(delta, &mut self.angle) {
                self.rotation = None;
            }
        }
    }
}

pub(super) struct Actors {
    actors: [Actor; 3],
    phase: u32,
    queued: Option<u32>,
    action: u32,
    path_ms: u32,
    sector: u32,
    chase_ms: u32,
    cue_phase: u8,
    cue_pending: bool,
    tracked_sector: Option<u32>,
    finish_ms: u32,
    finish_cue: bool,
    departure: Vec3,
    follower_started: bool,
}

impl Default for Actors {
    fn default() -> Self {
        Self {
            actors: Default::default(),
            phase: 0,
            queued: Some(0),
            action: 0,
            path_ms: 0,
            sector: 0,
            chase_ms: 0,
            cue_phase: 0,
            cue_pending: false,
            tracked_sector: None,
            finish_ms: 0,
            finish_cue: false,
            departure: Vec3::ZERO,
            follower_started: false,
        }
    }
}

impl Actors {
    pub fn trigger(&mut self, key: u32) {
        match key {
            5 => self.actors[1].select_cell(4),
            15 => self.actors[1].select_cell(8),
            16 => self.actors[0].select_cell(20),
            1..=4 | 6..=8 | 10..=14 | 17 => self.queued = Some(key),
            _ => {}
        }
    }

    pub fn status(&self) -> u32 {
        if self.queued.is_some() {
            u32::MAX
        } else {
            self.phase
        }
    }

    pub fn layers(
        &mut self,
        ids: [u32; 3],
        scene: usize,
        textures: &[Option<UploadedImage>],
        camera: &[u32; 40],
        size: [u32; 2],
        delta: u32,
    ) -> Result<Vec<Value>> {
        for actor in &mut self.actors {
            actor.advance(delta);
        }
        let mut order = [0, 1, 2];
        order.sort_by(|&a, &b| {
            self.actors[b]
                .position
                .z
                .total_cmp(&self.actors[a].position.z)
        });
        order
            .into_iter()
            .enumerate()
            .map(|(rank, i)| {
                let id = ids[i];
                let actor = &self.actors[i];
                let mut command = match textures[i].as_ref().filter(|_| actor.active) {
                    Some(image) => projected_atlas_cell(
                        id,
                        scene,
                        image,
                        actor.cell,
                        actor.position.to_array(),
                        actor.angle,
                        camera,
                        size,
                    )?
                    .unwrap_or_else(|| hidden_layer(id, scene)),
                    None => hidden_layer(id, scene),
                };
                command["layer"]["tie_order"] = Value::from(u64::from(ids[0]) + rank as u64);
                Ok(command)
            })
            .collect()
    }

    pub fn advance(&mut self, delta: u32) -> SceneSound {
        if self.phase == 0 && self.queued.is_some_and(|key| key != 0) {
            self.phase = self.queued.take().expect("queued actor cue");
        }
        match self.phase {
            1..=3 => self.enter(self.phase as usize - 1),
            4 => {
                self.phase = 90;
                self.actors[0].move_to([1., 2.8, -1.], 500, false);
                self.actors[2].move_to([-2.5, 3., -2.], 500, false);
                self.actors[1].move_to([-1., 2.7, 5.], 5000, false);
            }
            6 => {
                self.phase = 95;
                self.actors[0].select_cell(8);
                self.actors[0].set_position(Vec3::new(6., 4., -1.));
                self.actors[0].move_to([-6., 4.7, 11.], 1000, false);
                self.actors[0].turn(SPIN);
                return SceneSound::Duplicate(17, None);
            }
            7 | 12 | 13 => return self.cross_over(),
            8 => {
                self.action = 0;
                self.path_ms = 0;
                self.follower_started = false;
                self.chase_ms = 0;
                self.phase = 9;
                return self.jump_left(delta);
            }
            9 => return self.jump_left(delta),
            10 => return self.jump_around(),
            14 => {
                self.phase = 20;
                self.action = 0;
                self.path_ms = 0;
                self.sector = 0;
                self.finish_cue = false;
                self.follower_started = false;
                self.chase_ms = 0;
                self.cue_pending = false;
                self.cue_phase = 0;
                self.tracked_sector = None;
                return self.loop_and_depart(delta);
            }
            17 | 20 => return self.loop_and_depart(delta),
            90..=92 => {
                if self.actors[(self.phase - 90) as usize].motion.is_none() {
                    self.phase = 0;
                }
            }
            95 => {
                if self.actors.iter().all(|actor| actor.motion.is_none()) {
                    self.phase = 0;
                }
            }
            _ => {}
        }
        SceneSound::None
    }

    fn enter(&mut self, index: usize) {
        self.phase = 90 + index as u32;
        let starts = [[-4., 1.5, 7.], [3.3, 4., 5.], [3., 3.5, 3.]];
        let ends = [[-0.7, 3.2, 5.], [1., 4., 5.], [-1.6, 2.5, 5.]];
        let actor = &mut self.actors[index];
        actor.initialize(Vec3::from_array(starts[index]));
        if index == 1 {
            actor.turn(ROCK)
        }
        actor.move_to(ends[index], [500, 3000, 1000][index], index != 1);
    }

    fn cross_over(&mut self) -> SceneSound {
        let phase = self.phase;
        self.phase = 95;
        if phase == 7 {
            self.actors[0].select_cell(4);
            self.actors[0].set_position(Vec3::new(-6., 4., 11.));
            self.actors[0].move_to([1., 2.5, -1.], 300, false);
            self.actors[1].turn(LAND);
            self.actors[1].move_to([-0.5, 3.2, 5.], 1000, true);
        } else {
            self.actors[0].set_position(Vec3::new(6., 4.5, 11.));
            self.actors[0].move_to([if phase == 12 { -1. } else { -6. }, 2.5, -1.], 300, false);
            self.actors[1].turn(LAND_REVERSE);
            let target = if phase == 12 {
                [-1., 3.2, 5.]
            } else {
                [-1.5, 3.8, 5.]
            };
            self.actors[1].move_to(target, 1000, true);
        }
        if phase == 12 {
            SceneSound::Duplicate(16, Some(17))
        } else {
            SceneSound::Duplicate(17, None)
        }
    }

    fn jump_left(&mut self, delta: u32) -> SceneSound {
        let mut cue = SceneSound::None;
        if self.action == 0 {
            self.actors[0].set_position(Vec3::new(6., 3.3, 5.));
            self.actors[0].move_to([0.5, 3.3, 5.], 800, false);
            self.action = 1;
            self.cue_phase = 0;
        }
        if self.action == 1 && self.actors[0].motion.is_none() {
            self.action = 2;
            self.actors[0].turn(JUMP);
            self.departure = self.actors[0].target;
        }
        if self.action == 2 {
            self.path_ms = self.path_ms.wrapping_add(delta);
            let t = self.path_ms % 700;
            if (t < 100 && self.cue_phase == 0) || ((200..300).contains(&t) && self.cue_phase == 1)
            {
                self.cue_phase ^= 1;
                cue = SceneSound::Duplicate(16, None);
            }
            if self.actors[0].target.x > -1. {
                let first = t.min(100) as f32 * 0.1 / 100.;
                let second = if (200..300).contains(&t) {
                    (t - 200) as f32 * 0.1 / 100.
                } else {
                    0.1
                };
                let distance = self.path_ms / 700;
                self.actors[0].set_position(
                    self.departure - Vec3::X * (distance as f32 * 0.2 + first + second),
                );
            }
        }
        if self.queued == Some(10) && self.action == 2 {
            self.phase = 10;
            self.queued = None;
            self.actors[0].clear_rotation();
            let end = self.actors[0].target + Vec3::new(-3., 3., 0.);
            self.actors[0].move_to(end.to_array(), 300, false);
            self.actors[0].select_cell(16);
            self.action = 0;
            self.path_ms = 0;
            self.cue_pending = false;
        } else if !self.follower_started {
            let p = self.actors[0].target;
            self.actors[1].set_position(Vec3::new(p.x - 1., p.y - 0.1, 5.));
        }
        cue
    }

    fn jump_around(&mut self) -> SceneSound {
        let target = self.actors[1].target;
        let left = target + Vec3::new(-4., 5., 0.);
        let right = target + Vec3::new(4., 5., 0.);
        if self.actors[0].motion.is_none() {
            match self.action {
                0 | 2 if self.queued == Some(11) => {
                    self.queued = None;
                    self.actors[0].set_position(Vec3::new(6., 0.5, -1.));
                    self.actors[0].move_to([-6., 3.5, 11.], 300, false);
                    self.actors[1].turn(LAND);
                    self.actors[1].move_to([-0.5, 2.8, 5.], 1000, true);
                    self.phase = 95;
                    return SceneSound::Duplicate(17, None);
                }
                0 | 2 => {
                    self.actors[0].set_position(if self.action == 0 { left } else { right });
                    self.actors[0].move_to((target + Vec3::Y * 0.8).to_array(), 300, false);
                    self.action += 1;
                }
                1 | 3 => {
                    self.actors[0].move_to(
                        if self.action == 1 { right } else { left }.to_array(),
                        300,
                        false,
                    );
                    self.action = (self.action + 1) % 4;
                    self.cue_pending = true;
                }
                _ => {}
            }
        }
        if !self.follower_started && std::mem::take(&mut self.cue_pending) {
            if target.y > 2. {
                self.actors[1].set_position(target - Vec3::Y * 0.1);
            }
            SceneSound::Duplicate(16, None)
        } else {
            SceneSound::None
        }
    }

    fn loop_and_depart(&mut self, delta: u32) -> SceneSound {
        let mut first = None;
        let mut second = None;
        self.path_ms = self.path_ms.wrapping_add(delta);
        self.chase_ms = self.chase_ms.wrapping_add(delta);
        if self.phase == 17 {
            if !self.finish_cue {
                self.finish_cue = true;
                first = Some(17);
            }
            self.cue_pending = true;
            self.finish_ms = self.finish_ms.wrapping_add(delta);
            let z = 11. - self.finish_ms.min(300) as f32 * 10.5 / 300.;
            self.actors[0].set_position(Vec3::new(
                0.,
                if self.finish_ms >= 300 { 3.1 } else { 3.05 },
                z,
            ));
            if self.finish_ms < 500 && self.cue_phase < 2 {
                self.cue_phase = 3;
                second = Some(16);
            }
            if self.finish_ms < 500 {
                self.actors[1].angle = (100 * (720 * self.finish_ms / 500 % 360)) as i32;
            }
            let t = self.finish_ms.min(500) as f32 / 500.;
            self.actors[1].set_position(self.departure + Vec3::new(4., 1., -6.) * t);
        } else {
            let cycle = self.path_ms % 2000;
            let sector = cycle / 500;
            let t = (cycle % 500) as f32 / 500.;
            if sector != self.sector {
                first = Some(17)
            }
            self.sector = sector;
            let even = sector % 2 == 0;
            let x = 9. * t - if even { 6. } else { 3. };
            let y = if sector < 2 {
                6.5 - 6. * t
            } else {
                0.5 + 6. * t
            };
            let z = if even { 11. - 12. * t } else { 12. * t - 1. };
            self.actors[0].set_position(Vec3::new(x, y, z));
            if !self.follower_started && self.chase_ms >= 300 {
                self.chase_ms -= 300;
                self.actors[1].clear_rotation();
                self.follower_started = true;
            }
            if self.follower_started {
                let t = self.chase_ms % 1000;
                let swing = (t % 500) as f32 / 500.;
                let phase = u8::from(t >= 500);
                if self.cue_phase == phase {
                    self.cue_phase ^= 1;
                    second = Some(16);
                }
                let x = if phase == 0 {
                    3. * swing - 1.5
                } else {
                    1.5 - 3. * swing
                };
                let angle = 720 * t / 500 % 360;
                self.actors[1].angle = (100
                    * if phase == 0 {
                        (360 - angle) % 360
                    } else {
                        angle
                    }) as i32;
                self.actors[1].set_position(Vec3::new(
                    x,
                    3.8 + hundredths(sine(swing * 180.) * 0.8),
                    5.,
                ));
            }
        }
        if self.queued == Some(17) {
            let tracked = self.tracked_sector.get_or_insert(self.sector);
            if *tracked != self.sector {
                *tracked = self.sector;
                self.phase = 17;
                self.finish_ms = self.path_ms % 500;
            }
        }
        match (first, second) {
            (Some(first), second) => SceneSound::Duplicate(first, second),
            (None, Some(second)) => SceneSound::Duplicate(second, None),
            (None, None) => SceneSound::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_frames_and_motion_finish_before_the_next_queued_action() {
        let mut scene = Actors::default();
        assert_eq!(scene.status(), u32::MAX);
        scene.trigger(1);
        scene.advance(0);
        assert_eq!(scene.status(), 90);
        scene.actors[0].advance(500);
        assert_eq!(scene.actors[0].cell, 2);
        assert!(scene.actors[0].motion.is_some());
        scene.trigger(2);
        for _ in 0..2 {
            scene.actors[0].advance(200);
        }
        scene.advance(200);
        assert_eq!(scene.actors[0].position, Vec3::new(-0.7, 3.2, 5.));
        assert_eq!(scene.status(), u32::MAX);
        scene.advance(0);
        assert_eq!(scene.status(), 91);
        scene.trigger(15);
        assert_eq!(scene.actors[1].cell, 8);
        scene.actors[1].advance(100);
        assert_eq!(scene.actors[1].cell, 9);
    }

    #[test]
    fn departure_waits_for_a_new_sector_and_admits_both_resident_cues() {
        let mut scene = Actors::default();
        scene.trigger(14);
        assert!(matches!(scene.advance(0), SceneSound::None));
        assert_eq!(scene.status(), 20);
        scene.trigger(17);
        assert!(matches!(
            scene.advance(490),
            SceneSound::Duplicate(16, None)
        ));
        assert_eq!(scene.phase, 20);
        assert!(matches!(scene.advance(20), SceneSound::Duplicate(17, None)));
        assert_eq!(scene.phase, 17);
        assert_eq!(scene.status(), u32::MAX);
        assert!(matches!(
            scene.advance(10),
            SceneSound::Duplicate(17, Some(16))
        ));
        scene.advance(500);
        assert_eq!(scene.actors[0].position, Vec3::new(0., 3.1, 0.5));
    }
}

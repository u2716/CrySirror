// SPDX-License-Identifier: MPL-2.0
//! Type 0x54's five rising meshes and 512 drifting, blinking billboards.

use super::{append_transparent, camera_view, plane, quad, Rand, SceneSound};
use anyhow::Result;
use crysirror_core::service::UploadedImage;
use crysirror_scene::{layer::Blend, projected_mesh::Part};
use glam::{Mat4, Vec3};

pub(in crate::adapter) const TEXTURES: [&str; 6] = [
    "try008a.pb3",
    "try008b.pb3",
    "try008c.pb3",
    "try008d.pb3",
    "try008e.pb3",
    "try008z.pb3",
];
pub(in crate::adapter) const SAMPLE: &[(usize, &str)] = &[(16, "st03_magic1b.wav")];
pub(super) const RELOAD: &[&str] = &["try008z2.pb3"];

struct Billboard {
    position: Vec3,
    travel_ms: f32,
    delay: u32,
    blink_ms: u32,
    opacity: u8,
}

impl Billboard {
    fn new(rand: &mut Rand) -> Self {
        let unit = |rand: &mut Rand| rand.next() as f32 / 32767.;
        let x = unit(rand) * 4. - 2.;
        let z = unit(rand) * 2. + 1.;
        let y = unit(rand) * 10. - 5.;
        let travel_ms = (unit(rand) * 2000. - 1000.) + 8000.;
        let delay = (rand.next() % 1000) as u32;
        Self {
            position: Vec3::new(x, y, z),
            travel_ms,
            delay,
            blink_ms: 0,
            opacity: 0,
        }
    }

    fn advance(&mut self, delta: u32, fade_ms: u32, phase: &mut u32) {
        self.position.y += delta as f32 * 10. / self.travel_ms;
        if self.position.y >= 5. {
            self.position.y -= 10.;
        }
        if self.delay != 0 {
            self.delay = self.delay.saturating_sub(delta);
        } else {
            self.blink_ms = self.blink_ms.wrapping_add(delta);
        }
        if self.blink_ms == 0 {
            return;
        }
        let mut alpha = if self.blink_ms < 500 {
            self.blink_ms * 128 / 500
        } else {
            let cycle = (self.blink_ms - 500) % 1000;
            if cycle < 500 {
                128 + cycle * 128 / 500
            } else {
                256 - (cycle * 128 - 64_000) / 500
            }
        };
        match *phase {
            20 | 22 => {
                if fade_ms >= 1000 {
                    *phase += 1;
                    alpha = 0;
                } else {
                    alpha -= alpha * fade_ms / 1000;
                }
            }
            21 | 23 => alpha = 0,
            _ => {}
        }
        self.opacity = alpha.min(255) as u8;
    }
}

pub(super) struct Particles {
    billboards: Vec<Billboard>,
    queued: Option<u32>,
    phase: u32,
    rotation_ms: u32,
    growth_ms: u32,
    fade_ms: u32,
    cue_started: bool,
    scale: [f32; 5],
    opacity: [u8; 5],
}

impl Particles {
    pub fn new(rand: &mut Rand) -> Self {
        Self {
            billboards: (0..512).map(|_| Billboard::new(rand)).collect(),
            queued: None,
            phase: 0,
            rotation_ms: 0,
            growth_ms: 0,
            fade_ms: 0,
            cue_started: false,
            scale: [1.; 5],
            opacity: [0; 5],
        }
    }

    pub fn status(&self) -> u32 {
        if self.queued.is_some() {
            u32::MAX
        } else {
            self.phase
        }
    }

    pub fn trigger(&mut self, key: u32) {
        if matches!(key, 10 | 11 | 20 | 22) {
            self.queued = Some(key);
        }
    }

    pub fn advance(&mut self, delta: u32) -> SceneSound {
        let delta = delta.min(200);
        self.rotation_ms = self.rotation_ms.wrapping_add(delta);
        if let Some(phase) = self.queued.filter(|&phase| phase != self.phase) {
            self.phase = phase;
            self.queued = None;
        }
        let mut cue = SceneSound::None;
        if self.phase < 10 || self.phase == 11 {
            self.growth_ms = self.growth_ms.wrapping_add(delta);
            let start = if self.phase == 11 { 0 } else { 1500 };
            let mut complete = 0;
            for i in 0..5 {
                let delay = start + 800 - 200 * i as u32;
                let duration = 500 + 50 * i as u32;
                if self.growth_ms > delay {
                    if !self.cue_started {
                        self.cue_started = true;
                        cue = SceneSound::Play {
                            slot: 16,
                            stop_stream: false,
                        };
                    }
                    let age = (self.growth_ms - delay).min(duration);
                    self.scale[i] = age as f32 / duration as f32;
                    self.opacity[i] = (256 * age / duration).min(255) as u8;
                    complete += usize::from(age == duration);
                }
            }
            if self.phase == 11 && complete == 5 {
                self.phase = 12;
            }
        }
        if matches!(self.phase, 20 | 22) {
            self.fade_ms = self.fade_ms.wrapping_add(delta);
        }
        if self.phase == 20 {
            let alpha = 256 - 256 * self.fade_ms.min(1000) / 1000;
            self.opacity.fill(alpha.min(255) as u8);
            if self.fade_ms >= 1000 {
                self.phase = 21;
            }
        } else if self.phase == 21 {
            self.opacity.fill(0);
        }
        for particle in &mut self.billboards {
            particle.advance(delta, self.fade_ms, &mut self.phase);
        }
        cue
    }

    pub fn geometry(
        &self,
        parts: &mut Vec<Part>,
        textures: &[Option<UploadedImage>],
        camera: [f32; 7],
    ) -> Result<()> {
        let mut quads = Vec::with_capacity(517);
        let yaw = (self.rotation_ms.wrapping_mul(3600) / 6000 % 3600) as f32 / 10.;
        let position = Vec3::new(-0.16, 0.97, 2.);
        for (i, (image, y)) in textures[..5]
            .iter()
            .zip([0., 0.1, 0.3, 0.4, 0.5])
            .enumerate()
        {
            let Some(image) = image.as_ref().filter(|_| self.opacity[i] != 0) else {
                continue;
            };
            let yaw = if matches!(i, 0 | 2) { yaw } else { 360. - yaw };
            let world = Mat4::from_translation(position)
                * Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2)
                * Mat4::from_rotation_y(yaw.to_radians())
                * Mat4::from_scale(Vec3::splat(self.scale[i]))
                * Mat4::from_translation(Vec3::Y * y);
            quads.push((
                position,
                plane(
                    image,
                    [-1., 1.],
                    [-1., 1.],
                    world,
                    self.opacity[i],
                    Blend::Normal,
                ),
            ));
        }
        if let Some(image) = &textures[5] {
            let orientation = camera_view(camera)?.inverse();
            let corners = [
                Vec3::new(-0.05, 0.05, 0.),
                Vec3::new(0.05, 0.05, 0.),
                Vec3::new(-0.05, -0.05, 0.),
                Vec3::new(0.05, -0.05, 0.),
            ];
            for billboard in self.billboards.iter().filter(|sprite| sprite.opacity != 0) {
                let world = Mat4 {
                    w_axis: billboard.position.extend(1.),
                    ..orientation
                };
                quads.push((
                    billboard.position,
                    quad(image, corners, world, billboard.opacity, Blend::Add),
                ));
            }
        }
        append_transparent(parts, quads, camera);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particle_phases_reuse_native_clock_and_finish_at_frame_thresholds() {
        let mut rand = Rand::seeded(1);
        let mut scene = Particles::new(&mut rand);
        assert_eq!(scene.billboards.len(), 512);
        assert_eq!(scene.status(), 0);
        scene.trigger(10);
        assert!(matches!(scene.advance(300), SceneSound::None));
        assert_eq!(scene.rotation_ms, 200);
        assert_eq!(scene.growth_ms, 0);
        assert_eq!(scene.status(), 10);

        scene.trigger(11);
        assert!(matches!(
            scene.advance(200),
            SceneSound::Play { slot: 16, .. }
        ));
        assert_eq!(scene.status(), 11);
        assert_eq!(scene.opacity, [0, 0, 0, 0, 73]);
        for _ in 0..6 {
            assert!(matches!(scene.advance(200), SceneSound::None));
        }
        assert_eq!(scene.status(), 12);
        assert_eq!(scene.opacity, [255; 5]);

        scene.trigger(22);
        for _ in 0..4 {
            scene.advance(200);
        }
        assert_eq!(scene.status(), 22);
        scene.advance(200);
        assert_eq!(scene.status(), 23);
        assert_eq!(scene.opacity, [255; 5]);
        assert!(scene
            .billboards
            .iter()
            .all(|particle| particle.opacity == 0));
        scene.trigger(20);
        scene.advance(0);
        assert_eq!(scene.status(), 21);
        assert_eq!(scene.opacity, [0; 5]);
    }
}

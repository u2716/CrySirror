// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! TAIYO51's shared light bank, native setters and per-Print timers/RNG.
//! Reusable circular geometry and saturated drawing live in CircularOffset.
use anyhow::{Result, ensure};
use crysirror_core::image_processing::ImageInput;
use crysirror_image::{
    circular_offset::CircularOffset, effects::ImageEffect, temporal_pattern::PatternRng,
};
#[derive(Clone, Copy, Debug, Default)]
struct Light {
    distance: i32,
    radius: i32,
    hole: i32,
    color: [u8; 3],
    timer: i32,
    torus: bool,
}
#[derive(Clone, Debug)]
pub(crate) struct TaiyoClass {
    lights: [Light; 20],
}
impl Default for TaiyoClass {
    fn default() -> Self {
        // Uninitialized constructor bank is defined black/zero. SetParam changes
        // only its requested prefix; old slots can still address the retained tail.
        Self {
            lights: [Light::default(); 20],
        }
    }
}
pub(crate) fn parameters(values: &[i32], canvas: [u32; 2]) -> [i32; 11] {
    let defaults = [
        20,
        10,
        120,
        canvas[1] as i32 - 40,
        -100,
        -100,
        canvas[0] as i32 - 200,
        canvas[1] as i32 * 3 / 4,
        canvas[0] as i32 / 2,
        canvas[1] as i32 / 2,
        2,
    ];
    let mut p = std::array::from_fn(|i| {
        if i < 10 {
            values.get(i).copied().unwrap_or(defaults[i])
        } else {
            if values.len() < 11 { 2 } else { 0 }
        }
    });
    // Source copies at most TEN args but conditionally initializes word10
    // only when fewer than11 supplied. Define that uninitialized word as0.
    p[1] = p[1].clamp(1, 20);
    p
}
pub(crate) fn validate(p: &[i32; 11]) -> Result<()> {
    ensure!((1..=20).contains(&p[1]), "NNN TAIYO light count");
    Ok(())
}
impl TaiyoClass {
    pub(crate) fn set(&mut self, p: &[i32; 11], rng: &mut PatternRng) -> Result<()> {
        validate(p)?;
        let count = p[1];
        let mut torus_count = 0;
        for i in 0..count {
            let distance = (i + 1) * 150 / count + (rng.next_draw()? % (150 / count) as u32) as i32;
            let mut radius = 16 + (rng.next_draw()? % 64) as i32;
            // The last element still draws the discarded ordinary radius.
            if i == count - 1 {
                radius = 12 + (rng.next_draw()? % 12) as i32;
            }
            let mut color = [
                24 + (rng.next_draw()? % 64) as u8,
                24 + (rng.next_draw()? % 64) as u8,
                24 + (rng.next_draw()? % 64) as u8,
            ];
            let mut hole = 0;
            let mut torus = false;
            if torus_count < p[10] && ((rng.next_draw()? % 100) < 18 || i == count - 3) {
                torus = true;
                radius = 72 + (rng.next_draw()? % 64) as i32;
                hole = (radius - 20 - (rng.next_draw()? % 16) as i32).max(3);
                color = [
                    10 + (rng.next_draw()? % 24) as u8,
                    10 + (rng.next_draw()? % 24) as u8,
                    10 + (rng.next_draw()? % 24) as u8,
                ];
                torus_count += 1;
            }
            let timer = 20 + (rng.next_draw()? % 30) as i32;
            self.lights[i as usize] = Light {
                distance,
                radius,
                hole,
                color,
                timer,
                torus,
            };
        }
        Ok(())
    }
    pub(crate) fn image_input(
        &mut self,
        p: &[i32; 11],
        mut count: i32,
        mut frames: i32,
        base: ImageInput,
        rng: &mut PatternRng,
    ) -> Result<ImageInput> {
        validate(p)?;
        if frames <= 0 {
            frames = 1;
            count = 1;
        }
        let lerp = |a: i32, b: i32| a.wrapping_add(b.wrapping_sub(a).wrapping_mul(count) / frames);
        let point = [lerp(p[2], p[4]), lerp(p[3], p[5])];
        let center = [lerp(p[6], p[8]), lerp(p[7], p[9])];
        let mut effects = Vec::new();
        for light in self.lights.iter_mut().take(p[1] as usize) {
            let x = point[0].wrapping_add(
                center[0]
                    .wrapping_sub(point[0])
                    .wrapping_mul(light.distance)
                    / 100,
            );
            let y = point[1].wrapping_add(
                center[1]
                    .wrapping_sub(point[1])
                    .wrapping_mul(light.distance)
                    / 100,
            );
            light.timer = light.timer.wrapping_sub(1);
            if light.timer > 0 {
                effects.push(ImageEffect::CircularOffset {
                    spec: CircularOffset {
                        center: [x, y],
                        radius: light.radius.max(0) as u32,
                        cutout_radius: light.torus.then_some(light.hole.max(0) as u32),
                        cutout_excludes_outer_right: true,
                        add: light.color,
                        subtract: [0; 3],
                    },
                });
            } else {
                light.timer = 30 + (rng.next_draw()? % 60) as i32;
            }
        }
        Ok(ImageInput::Process {
            input: Box::new(base),
            effects: serde_json::to_vec(&effects)?.into(),
        })
    }
}
// InitParameter reruns SetParam with32 arguments: its broken copy bound above
// consequently defines word10=0 on restore. This class bank is NOT raw EFFECT
// storage. Calcu only clamps count to max; BeginPrint/Init/End are empty,
// CountIsMax TRUE and inherited ordinary picture pass precedes custom drawing.

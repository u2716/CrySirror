// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Two-source radial displacement using a worker-prepared packed vector field.
//! Stored signed-word multiply-high offsets and low-word summed colour weights
//! are explicit; source alpha and the previous destination are not inputs.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS, rgba_len};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadialWavePair {
    pub canvas: [u32; 2],
    pub count: i32,
    pub wave_length: i32,
    pub speed: i32,
    pub angular_speed: i32,
    pub hold: i32,
    pub power: f64,
    pub source_origins: [[i32; 2]; 2],
    pub weights: [u16; 2],
}
impl RadialWavePair {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.canvas.iter().all(|n| (1..=16384).contains(n))
                && u64::from(self.canvas[0]) * u64::from(self.canvas[1]) <= MAX_EFFECT_PIXELS,
            "radial wave output budget"
        );
        ensure!(
            self.wave_length > 0
                && self.power.is_finite()
                && (0.0..=1_000_000.0).contains(&self.power),
            "radial wave divisor/power domain"
        );
        ensure!(
            [
                self.count,
                self.wave_length,
                self.speed,
                self.angular_speed,
                self.hold
            ]
            .iter()
            .chain(self.source_origins.iter().flatten())
            .all(|n| n.unsigned_abs() <= 1_000_000),
            "radial wave scalar bounds"
        );
        Ok(())
    }
    pub fn radius_count(&self) -> Result<usize> {
        self.validate()?;
        let square = u64::from(self.canvas[0]) * u64::from(self.canvas[0])
            + u64::from(self.canvas[1]) * u64::from(self.canvas[1]);
        Ok(((square as f64).sqrt() + 1.0) as usize)
    }
    fn displacements(&self, check: &dyn Fn() -> Result<()>) -> Result<Vec<i16>> {
        let length = self.radius_count()?;
        let end = 10i32.wrapping_add(self.hold);
        let mut values = vec![0; length];
        for (i, value) in values.iter_mut().enumerate() {
            if i % 4096 == 0 {
                check()?;
            }
            let k = self.count.wrapping_mul(self.speed).wrapping_sub(i as i32);
            if k < 0 || k >= end {
                continue;
            }
            let amplitude = if k < 10 {
                self.power * f64::from(k) / 10.0
            } else {
                self.power * f64::from(end.wrapping_sub(k)) / f64::from(end.wrapping_sub(10))
            };
            let phase = self
                .count
                .wrapping_mul(self.angular_speed.wrapping_neg())
                .wrapping_add(i as i32)
                % self.wave_length;
            let mut angle = f64::from(phase);
            angle *= 3.14159 * 2.0;
            angle /= f64::from(self.wave_length);
            let d = angle.cos() * amplitude + 0.5;
            // Invalid float-to-int conversion was undefined in native code;
            // bound that case visibly rather than accepting arbitrary memory.
            ensure!(
                d.is_finite() && d >= f64::from(i32::MIN) && d <= f64::from(i32::MAX),
                "radial wave displacement integer domain"
            );
            *value = (d as i32).wrapping_mul(256) as i16;
        }
        Ok(values)
    }
    pub fn apply(
        &self,
        field: &ImageSurface,
        first: &ImageSurface,
        second: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        for image in [field, first, second] {
            image.validate()?;
            ensure!(
                u64::from(image.width) * u64::from(image.height) <= MAX_EFFECT_PIXELS,
                "radial wave input budget"
            );
        }
        let aligned = (self.canvas[0] + 3) & !3;
        ensure!(
            field.width == aligned * 3 / 2 && field.height == self.canvas[1],
            "radial wave field storage layout"
        );
        check()?;
        let values = self.displacements(check)?;
        let mut output = ImageSurface {
            width: self.canvas[0],
            height: self.canvas[1],
            rgba: vec![0; rgba_len(self.canvas[0], self.canvas[1])?],
        };
        let sample =
            |image: &ImageSurface, origin: [i32; 2], position: [i32; 2], delta: [i32; 2]| {
                let x = position[0].wrapping_add(delta[0]).wrapping_add(origin[0]);
                let y = position[1].wrapping_add(delta[1]).wrapping_add(origin[1]);
                if x < 0 || y < 0 || x >= image.width as i32 || y >= image.height as i32 {
                    [0; 3]
                } else {
                    let at = (y as usize * image.width as usize + x as usize) * 4;
                    [image.rgba[at], image.rgba[at + 1], image.rgba[at + 2]]
                }
            };
        for y in 0..self.canvas[1] {
            check()?;
            for x in 0..self.canvas[0] {
                if x % 4096 == 0 {
                    check()?;
                }
                let at = y as usize * field.width as usize * 4 + (x / 4) as usize * 24;
                let lane = (x % 4) as usize;
                let word = |offset| {
                    u16::from_le_bytes(field.rgba[at + offset..at + offset + 2].try_into().unwrap())
                };
                let radius = word(16 + lane * 2);
                ensure!(radius % 4 == 0, "radial wave field radius alignment");
                let displacement = *values
                    .get(radius as usize / 4)
                    .ok_or_else(|| anyhow::anyhow!("radial wave field radius span"))?;
                let delta = std::array::from_fn::<_, 2, _>(|axis| {
                    ((i32::from(word(lane * 4 + axis * 2) as i16) * i32::from(displacement)) >> 16)
                });
                let a = sample(first, self.source_origins[0], [x as i32, y as i32], delta);
                let b = sample(second, self.source_origins[1], [x as i32, y as i32], delta);
                let out = (y as usize * self.canvas[0] as usize + x as usize) * 4;
                for c in 0..3 {
                    output.rgba[out + c] = (u16::from(a[c])
                        .wrapping_mul(self.weights[0])
                        .wrapping_add(u16::from(b[c]).wrapping_mul(self.weights[1]))
                        >> 8) as u8;
                }
                output.rgba[out + 3] = 255;
            }
        }
        // Native odd widths wrote one pixel past a row with a two-pixel MMX
        // loop. Explicit canvas bounds define only that invalid extra write away.
        check()?;
        Ok(output)
    }
}

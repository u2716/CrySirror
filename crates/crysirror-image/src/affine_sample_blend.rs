// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Explicit double affine sampling with integer texel centres and optional
//! scale-distance neighbours. Stored RGB/coverage interpolation and composition
//! are independent of engine adapters, entities and graphics devices.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AffineSampleFilter {
    Nearest,
    ScaleNeighbor,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AffineSampleBlend {
    Normal,
    Add,
    Subtract,
    Copy,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AffineSampleBlendByte {
    pub rectangle: [i32; 4],
    pub destination_origin: [i32; 2],
    /// Matrix products/sum precede post_scale, translation, then sample_bias;
    /// these stages intentionally are not flattened into one coefficient.
    pub linear: [[f64; 2]; 2],
    pub post_scale: [f64; 2],
    pub translation: [i32; 2],
    pub sample_bias: [f64; 2],
    pub filter: AffineSampleFilter,
    /// Distances to the alternative horizontal/vertical texel for ScaleNeighbor.
    pub neighbor_step: [f64; 2],
    pub color_offset: [i32; 3],
    pub percent: i32,
    pub blend: AffineSampleBlend,
}
impl AffineSampleBlendByte {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.rectangle
                .iter()
                .chain(&self.destination_origin)
                .chain(&self.translation)
                .all(|n| n.unsigned_abs() <= 1_000_000)
                && self.rectangle[2] >= 0
                && self.rectangle[3] >= 0,
            "affine sample geometry bounds"
        );
        ensure!(
            self.linear
                .iter()
                .flatten()
                .chain(&self.post_scale)
                .chain(&self.sample_bias)
                .chain(&self.neighbor_step)
                .all(|n| n.is_finite() && n.abs() <= 1_000_000.0),
            "affine sample coefficient bounds"
        );
        ensure!(
            self.color_offset
                .iter()
                .all(|n| n.unsigned_abs() <= 1_000_000)
                && self.percent.unsigned_abs() <= 1_000_000,
            "affine sample blend bounds"
        );
        ensure!(
            self.rectangle[2] as u64 * self.rectangle[3] as u64 <= MAX_EFFECT_PIXELS * 4,
            "affine sample work budget"
        );
        Ok(())
    }
    pub fn apply(
        &self,
        destination: &ImageSurface,
        source: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        for image in [destination, source] {
            image.validate()?;
            ensure!(
                u64::from(image.width) * u64::from(image.height) <= MAX_EFFECT_PIXELS,
                "affine sample image budget"
            );
        }
        check()?;
        let mut output = destination.clone();
        let [x, y, width, height] = self.rectangle.map(i64::from);
        for dy in y.max(0)..(y + height).min(i64::from(destination.height)) {
            check()?;
            for dx in x.max(0)..(x + width).min(i64::from(destination.width)) {
                let delta = [
                    dx - i64::from(self.destination_origin[0]),
                    dy - i64::from(self.destination_origin[1]),
                ];
                let sample = std::array::from_fn::<_, 2, _>(|axis| {
                    let mapped = self.linear[axis][0] * delta[0] as f64
                        + self.linear[axis][1] * delta[1] as f64;
                    (f64::from(self.translation[axis]) + mapped * self.post_scale[axis])
                        + self.sample_bias[axis]
                });
                if !sample.iter().all(|v| v.is_finite() && *v >= 0.0) {
                    continue;
                }
                let point = sample.map(|v| v as i64);
                if point[0] >= i64::from(source.width) || point[1] >= i64::from(source.height) {
                    continue;
                }
                let load = |at: [i64; 2]| -> [i32; 4] {
                    let index = (at[1] as usize * source.width as usize + at[0] as usize) * 4;
                    std::array::from_fn(|c| i32::from(source.rgba[index + c]))
                };
                let mut pixel = load(point);
                if matches!(self.filter, AffineSampleFilter::ScaleNeighbor) {
                    let mut alpha = [0.0; 2];
                    let mut peek = point;
                    for axis in 0..2 {
                        let remainder = sample[axis] - point[axis] as f64;
                        let direction = if remainder >= 0.5 {
                            alpha[axis] = 1.5 - remainder;
                            1.0
                        } else {
                            alpha[axis] = 0.5 + remainder;
                            -1.0
                        };
                        let candidate = sample[axis] + direction * self.neighbor_step[axis];
                        let integer = candidate as i64;
                        let extent = if axis == 0 {
                            source.width
                        } else {
                            source.height
                        };
                        if candidate >= 0.0 && integer >= 0 && integer < i64::from(extent) {
                            peek[axis] = integer;
                        }
                    }
                    let weights = [
                        (alpha[0] * alpha[1] * 256.0) as i32,
                        ((1.0 - alpha[0]) * alpha[1] * 256.0) as i32,
                        (alpha[0] * (1.0 - alpha[1]) * 256.0) as i32,
                        ((1.0 - alpha[0]) * (1.0 - alpha[1]) * 256.0) as i32,
                    ];
                    let pixels = [
                        pixel,
                        load([peek[0], point[1]]),
                        load([point[0], peek[1]]),
                        load(peek),
                    ];
                    pixel = std::array::from_fn(|channel| {
                        pixels
                            .iter()
                            .zip(weights)
                            .map(|(p, w)| p[channel] * w)
                            .sum::<i32>()
                            >> 8
                    });
                }
                if pixel[3] <= 0 {
                    continue;
                }
                let rgb = std::array::from_fn::<_, 3, _>(|c| {
                    (pixel[c] + self.color_offset[c]).clamp(0, 255)
                });
                let at = (dy as usize * destination.width as usize + dx as usize) * 4;
                // Percent and packed-word RGB arithmetic are explicitly32-bit;
                // unusual weights retain their byte carries rather than being
                // silently clamped to a modern alpha0..1 approximation.
                let weight = 256i32.wrapping_mul(pixel[3].wrapping_mul(self.percent)) / 25500;
                let result = std::array::from_fn::<_, 3, _>(|c| {
                    let old = i32::from(output.rgba[at + c]);
                    match self.blend {
                        AffineSampleBlend::Copy => rgb[c],
                        AffineSampleBlend::Normal if pixel[3] == 255 && self.percent == 100 => {
                            rgb[c]
                        }
                        AffineSampleBlend::Normal => {
                            rgb[c]
                                .wrapping_mul(weight)
                                .wrapping_add(old.wrapping_mul(256i32.wrapping_sub(weight)))
                                >> 8
                        }
                        AffineSampleBlend::Add => {
                            old.wrapping_add(rgb[c].wrapping_mul(weight) >> 8).min(255)
                        }
                        AffineSampleBlend::Subtract => {
                            old.wrapping_sub(rgb[c].wrapping_mul(weight) >> 8).max(0)
                        }
                    }
                });
                let packed = result[0].wrapping_shl(16) | result[1].wrapping_shl(8) | result[2];
                output.rgba[at] = (packed >> 16) as u8;
                output.rgba[at + 1] = (packed >> 8) as u8;
                output.rgba[at + 2] = packed as u8;
            }
        }
        check()?;
        Ok(output)
    }
}

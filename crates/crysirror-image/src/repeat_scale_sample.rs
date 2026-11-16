// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Repeated fixed-point scaled samples in O(destination pixels), independent
//! of the number/size of virtual tiles. Clipping quantizes the first visible
//! tile's starting phase independently of its16.16 per-pixel step.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS, fixed_scanlines::ScanlineMath};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepeatScaleSample {
    pub period: [u32; 2],
    pub origin: [i32; 2],
    pub source: [u32; 4],
    pub math: ScanlineMath,
    pub corner_rgb_key: bool,
}
fn fixed_positive(value: f64) -> i64 {
    let whole = value as i64;
    (whole << 16) | (((value - whole as f64) * 65536.) as i64 & 65535)
}
impl RepeatScaleSample {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.period.iter().all(|n| (1..=16_777_216).contains(n)),
            "repeated scaled period"
        );
        ensure!(
            self.origin.iter().all(|n| n.unsigned_abs() <= 16_777_216),
            "repeated scaled origin"
        );
        ensure!(
            self.source[2..].iter().all(|n| (1..=16384).contains(n)),
            "repeated scaled source size"
        );
        // FixedScanlines owns the same byte profile and its validation.
        crate::fixed_scanlines::FixedScanlines {
            spans: vec![crate::fixed_scanlines::FixedSampleSpan {
                destination: [0, 0, 0],
                source_q16: [0, 0],
                source_step_q16: [0, 0],
                math: self.math,
            }],
            corner_rgb_key: false,
            source_color: None,
        }
        .validate()
    }
    pub fn apply(
        &self,
        destination: &ImageSurface,
        source: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        destination.validate()?;
        source.validate()?;
        ensure!(
            [source, destination]
                .iter()
                .all(|i| u64::from(i.width) * u64::from(i.height) <= MAX_EFFECT_PIXELS),
            "repeated scaled image budget"
        );
        ensure!(
            u64::from(self.source[0]) + u64::from(self.source[2]) <= u64::from(source.width)
                && u64::from(self.source[1]) + u64::from(self.source[3])
                    <= u64::from(source.height),
            "repeated scaled source bounds"
        );
        check()?;
        let mapping = |axis: usize, length: u32| {
            let period = i64::from(self.period[axis]);
            let origin = i64::from(self.origin[axis]);
            let step = f64::from(self.source[axis + 2]) / self.period[axis] as f64;
            let delta = fixed_positive(step);
            (0..length)
                .map(|at| {
                    let at = i64::from(at);
                    let tile = origin + (at - origin).div_euclid(period) * period;
                    let skipped = (-tile).max(0);
                    let first = tile.max(0);
                    i64::from(self.source[axis])
                        + (fixed_positive(step * skipped as f64) + (at - first) * delta >> 16)
                })
                .collect::<Vec<_>>()
        };
        let xs = mapping(0, destination.width);
        let ys = mapping(1, destination.height);
        let mut out = destination.clone();
        let key = &source.rgba[..3];
        for (y, sy) in ys.into_iter().enumerate() {
            check()?;
            if sy < 0 || sy >= i64::from(source.height) {
                continue;
            }
            for (x, sx) in xs.iter().copied().enumerate() {
                if sx < 0 || sx >= i64::from(source.width) {
                    continue;
                }
                let si = (sy as usize * source.width as usize + sx as usize) * 4;
                if self.corner_rgb_key && &source.rgba[si..si + 3] == key {
                    continue;
                }
                let di = (y * destination.width as usize + x) * 4;
                self.math
                    .pixel(&mut out.rgba[di..di + 4], &source.rgba[si..si + 4]);
            }
        }
        check()?;
        Ok(out)
    }
}

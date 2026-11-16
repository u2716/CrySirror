// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Saturated channel offsets over squared-distance circular spans. This
//! explicit calculation retains integer-square / sqrt / +0.5 quantization.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CircularOffset {
    pub center: [i32; 2],
    pub radius: u32,
    pub cutout_radius: Option<u32>,
    /// When a hole intersects a row, the rightmost outer boundary is excluded.
    pub cutout_excludes_outer_right: bool,
    pub add: [u8; 3],
    pub subtract: [u8; 3],
}
impl CircularOffset {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.center.iter().all(|n| n.unsigned_abs() <= 1_000_000)
                && self.radius <= 46340
                && self.cutout_radius.is_none_or(|r| r <= self.radius),
            "circular offset geometry bounds"
        );
        Ok(())
    }
    pub fn apply(
        &self,
        input: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        input.validate()?;
        ensure!(
            u64::from(input.width) * u64::from(input.height) <= MAX_EFFECT_PIXELS,
            "circular offset pixel budget"
        );
        check()?;
        let mut output = input.clone();
        let [cx, cy] = self.center.map(i64::from);
        let r = i64::from(self.radius);
        let width = i64::from(input.width);
        for y in (cy - r).max(0)..=(cy + r).min(i64::from(input.height) - 1) {
            check()?;
            let delta = y - cy;
            let rr = r * r - delta * delta;
            if rr < 0 {
                continue;
            }
            let half = ((rr as f64).sqrt() + 0.5) as i64;
            let left = (cx - half).max(0);
            let right = (cx + half).min(width - 1);
            let spans = if let Some(hole) = self
                .cutout_radius
                .filter(|h| i64::from(*h) * i64::from(*h) >= delta * delta)
            {
                let inside = i64::from(hole) * i64::from(hole) - delta * delta;
                let half = ((inside as f64).sqrt() + 0.5) as i64;
                let cut_left = (cx - half).min(width - 1);
                let cut_right = (cx + half).max(0);
                [
                    [left, cut_left - 1],
                    [
                        cut_right,
                        right - i64::from(self.cutout_excludes_outer_right),
                    ],
                ]
            } else {
                [[left, right], [1, 0]]
            };
            for [left, right] in spans {
                for x in left.max(0)..=right.min(width - 1) {
                    let at = (y as usize * input.width as usize + x as usize) * 4;
                    for c in 0..3 {
                        output.rgba[at + c] = output.rgba[at + c]
                            .saturating_add(self.add[c])
                            .saturating_sub(self.subtract[c]);
                    }
                }
            }
        }
        check()?;
        Ok(output)
    }
}

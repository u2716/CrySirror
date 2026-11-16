// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Ordered add/subtract coverage brushes with exact rational nearest sampling.
//! A logical destination pitch can address retained linear storage across a
//! surface-size change; RGB is never modified and one batch clones only once.
use crate::ImageSurface;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum AlphaBrushMath {
    SaturatingAdd,
    SaturatingSubtract,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlphaBrush {
    pub destination: [i32; 4],
    pub source_origin: [i32; 2],
    /// Added before rational scaling, so a clipped scaled brush preserves its
    /// original sample phase: sourceOrigin+floor((phase+local)*numerator/divisor).
    pub source_phase: [i32; 2],
    pub numerator: [u32; 2],
    pub divisor: [u32; 2],
    pub math: AlphaBrushMath,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlphaBrushes {
    pub destination_pitch: u32,
    pub brushes: Vec<AlphaBrush>,
}
impl AlphaBrushes {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (1..=16384).contains(&self.destination_pitch) && self.brushes.len() <= 4096,
            "alpha brushes pitch/count"
        );
        let mut work = 0u64;
        for brush in &self.brushes {
            ensure!(
                brush
                    .destination
                    .iter()
                    .chain(&brush.source_origin)
                    .chain(&brush.source_phase)
                    .all(|n| n.unsigned_abs() <= 1_000_000)
                    && brush.destination[2] >= 0
                    && brush.destination[3] >= 0
                    && brush.numerator.iter().all(|n| *n <= 1_000_000)
                    && brush.divisor.iter().all(|n| (1..=1_000_000).contains(n)),
                "alpha brushes coordinate/scale domain"
            );
            work += brush.destination[2] as u64 * brush.destination[3] as u64;
        }
        ensure!(
            work <= crate::effects::MAX_EFFECT_PIXELS,
            "alpha brushes work budget"
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
        destination.validate()?;
        source.validate()?;
        let capacity = i64::from(destination.width) * i64::from(destination.height);
        check()?;
        let mut out = destination.clone();
        for brush in &self.brushes {
            let [x, y, w, h] = brush.destination;
            for row in 0..h {
                check()?;
                let sy = i64::from(brush.source_origin[1])
                    + (i64::from(brush.source_phase[1]) + i64::from(row))
                        * i64::from(brush.numerator[1])
                        / i64::from(brush.divisor[1]);
                if sy < 0 || sy >= i64::from(source.height) {
                    continue;
                }
                let start = (i64::from(y) + i64::from(row)) * i64::from(self.destination_pitch)
                    + i64::from(x);
                for col in 0..w {
                    if col % 4096 == 0 {
                        check()?;
                    }
                    let to = start + i64::from(col);
                    let sx = i64::from(brush.source_origin[0])
                        + (i64::from(brush.source_phase[0]) + i64::from(col))
                            * i64::from(brush.numerator[0])
                            / i64::from(brush.divisor[0]);
                    // The logical byte stream is bounded by its real retained
                    // allocation, not by a fictitious re-shaped row rectangle.
                    // Former accesses beyond that allocation are explicit no-write.
                    if to < 0 || to >= capacity || sx < 0 || sx >= i64::from(source.width) {
                        continue;
                    }
                    let from = (sy as usize * source.width as usize + sx as usize) * 4 + 3;
                    let to = to as usize * 4 + 3;
                    let a = source.rgba[from];
                    out.rgba[to] = match brush.math {
                        AlphaBrushMath::SaturatingAdd => out.rgba[to].saturating_add(a),
                        AlphaBrushMath::SaturatingSubtract => out.rgba[to].saturating_sub(a),
                    };
                }
            }
        }
        check()?;
        Ok(out)
    }
}

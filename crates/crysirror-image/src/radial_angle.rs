// Copyright (c)2026 CrySirror contributors. MPL-2.0.
//! Radius-indexed angular nearest sampling with explicit coordinate origins.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

/// Immutable radius-to-angle mapping. Radius is integer floor distance from
/// `field_origin`; entries beyond the table repeat its final angle. Trig values
/// are degree-indexed binary32, samples round to nearest ties-even and clamp.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadialAngleLookup {
    pub field_origin: [i32; 2],
    pub vector_origin: [i32; 2],
    pub source_center: [i32; 2],
    /// True negates both the destination vector y and sampled source y.
    pub y_up: bool,
    /// Each radius selects an explicit integer angle in0..359 degrees.
    pub angles: Vec<u16>,
}
impl RadialAngleLookup {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.field_origin
                .iter()
                .chain(&self.vector_origin)
                .chain(&self.source_center)
                .all(|n| n.unsigned_abs() <= 1_000_000),
            "radial angle coordinate origins"
        );
        ensure!(
            !self.angles.is_empty()
                && self.angles.len() <= 65536
                && self.angles.iter().all(|a| *a < 360),
            "radial angle table"
        );
        Ok(())
    }
    pub fn auxiliary_bytes(&self) -> usize {
        self.angles.len() * std::mem::size_of::<u16>() + 360 * 2 * std::mem::size_of::<f32>()
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
            "radial angle pixel budget"
        );
        check()?;
        let mut trig = [[0f32; 2]; 360];
        for (degree, pair) in trig.iter_mut().enumerate() {
            if degree % 16 == 0 {
                check()?;
            }
            let angle = degree as f64 * 0.0174532925199433;
            *pair = [angle.cos() as f32, angle.sin() as f32];
        }
        let mut output = input.clone();
        for y in 0..input.height {
            check()?;
            for x in 0..input.width {
                let dx = i64::from(x) - i64::from(self.field_origin[0]);
                let dy = i64::from(y) - i64::from(self.field_origin[1]);
                let radius = (dx * dx + dy * dy) as u64;
                let radius = (radius.isqrt() as usize).min(self.angles.len() - 1);
                let [c, s] = trig[usize::from(self.angles[radius])].map(f64::from);
                let vx = f64::from(x) - f64::from(self.vector_origin[0]);
                let vy = if self.y_up {
                    f64::from(self.vector_origin[1]) - f64::from(y)
                } else {
                    f64::from(y) - f64::from(self.vector_origin[1])
                };
                let rx = (vx * c - vy * s).round_ties_even() as i64;
                let ry = (vx * s + vy * c).round_ties_even() as i64;
                let sx =
                    (i64::from(self.source_center[0]) + rx).clamp(0, i64::from(input.width) - 1);
                let sy = (i64::from(self.source_center[1]) + if self.y_up { -ry } else { ry })
                    .clamp(0, i64::from(input.height) - 1);
                let from = ((sy * i64::from(input.width) + sx) * 4) as usize;
                let to = ((u64::from(y) * u64::from(input.width) + u64::from(x)) * 4) as usize;
                output.rgba[to..to + 3].copy_from_slice(&input.rgba[from..from + 3]);
            }
        }
        check()?;
        Ok(output)
    }
}

// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Radius-scaled angular sampling with independently quantized integer radius
//! and tenths-radius twist, followed by stored coverage-percent composition.
use crate::{ImageSurface, byte_raster::ByteRasterMath, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadialTwistSample {
    pub field_origin: [i32; 2],
    pub source_center: [i32; 2],
    pub radius_scale: f64,
    pub twist_degrees: i32,
    pub reference_radius: u32,
    pub angle_pi: f64,
    pub coverage_percent: u8,
}
impl RadialTwistSample {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.field_origin
                .iter()
                .chain(&self.source_center)
                .all(|n| n.unsigned_abs() <= 1_000_000)
                && self.radius_scale.is_finite()
                && self.radius_scale > 0.0
                && self.radius_scale <= 1000.0
                && self.reference_radius > 0
                && self.reference_radius <= 16384
                && self.angle_pi.is_finite()
                && (3.0..=3.2).contains(&self.angle_pi)
                && self.coverage_percent <= 100,
            "radial twist sampling bounds"
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
                "radial twist image budget"
            );
        }
        check()?;
        let mut output = destination.clone();
        for y in 0..destination.height {
            check()?;
            for x in 0..destination.width {
                let dx = i64::from(x) - i64::from(self.field_origin[0]);
                let dy = i64::from(y) - i64::from(self.field_origin[1]);
                // Wide checked-domain squared distance replaces native signed
                // integer overflow for extreme offscreen centres.
                let distance = ((dx * dx + dy * dy) as f64).sqrt();
                let mut radius = (distance * self.radius_scale) as i32;
                let radius10 = (distance * self.radius_scale * 10.0) as i32;
                if radius < 1 {
                    radius = 1;
                }
                let twist =
                    self.twist_degrees.wrapping_mul(radius10) / (self.reference_radius as i32 * 10);
                let twist = twist % 360;
                let mut angle = f64::from(twist) / 360.0;
                angle *= self.angle_pi * 2.0;
                angle += (dy as f64).atan2(dx as f64);
                let offset = [
                    (angle.cos() * f64::from(radius) + 0.5) as i32,
                    (angle.sin() * f64::from(radius) + 0.5) as i32,
                ];
                let sx = self.source_center[0].wrapping_add(offset[0]);
                let sy = self.source_center[1].wrapping_add(offset[1]);
                if sx < 0 || sy < 0 || sx as u32 >= source.width || sy as u32 >= source.height {
                    continue;
                }
                let from = (sy as usize * source.width as usize + sx as usize) * 4;
                let to = (y as usize * destination.width as usize + x as usize) * 4;
                ByteRasterMath::AlphaPercentRgb256 {
                    percent: self.coverage_percent,
                }
                .pixel(&mut output.rgba[to..to + 4], &source.rgba[from..from + 4]);
            }
        }
        check()?;
        Ok(output)
    }
}

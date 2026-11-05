// SPDX-License-Identifier: MPL-2.0
//! Immutable alpha-plane modulation against a retained baseline and mask.
//! RGB and pixels outside the mask overlap retain their current stored bytes.
use crate::ImageSurface;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlphaMaskPlane {
    pub channel: u8,
    pub offset: f32,
    /// Restore the complete baseline coverage after a completed reveal.
    #[serde(default)]
    pub restore: bool,
}
impl AlphaMaskPlane {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.channel < 4 && self.offset.is_finite(),
            "alpha mask plane parameters"
        );
        Ok(())
    }
    pub fn apply(
        &self,
        destination: &ImageSurface,
        baseline: &ImageSurface,
        mask: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        destination.validate()?;
        baseline.validate()?;
        mask.validate()?;
        ensure!(
            (destination.width, destination.height) == (baseline.width, baseline.height),
            "alpha mask baseline extent"
        );
        ensure!(
            u64::from(destination.width) * u64::from(destination.height)
                <= crate::effects::MAX_EFFECT_PIXELS,
            "alpha mask pixel budget"
        );
        let mut out = destination.clone();
        let width = if self.restore {
            out.width
        } else {
            out.width.min(mask.width)
        };
        let height = if self.restore {
            out.height
        } else {
            out.height.min(mask.height)
        };
        for y in 0..height {
            check()?;
            for x in 0..width {
                let at = ((y * out.width + x) * 4) as usize;
                let alpha = baseline.rgba[at + 3];
                if self.restore {
                    out.rgba[at + 3] = alpha;
                    continue;
                }
                if alpha == 0 {
                    continue;
                }
                let m = mask.rgba[((y * mask.width + x) * 4) as usize + usize::from(self.channel)];
                let shifted = (f64::from(m) + f64::from(self.offset)) as i32;
                let shifted = shifted.clamp(0, 255);
                let result = if alpha == 255 || shifted == 0 {
                    shifted.min(i32::from(alpha)) as u8
                } else {
                    let product = (f64::from(alpha) / 255. * f64::from(shifted)) as f32;
                    product.clamp(0., f32::from(alpha)) as u8
                };
                out.rgba[at + 3] = result;
            }
        }
        Ok(out)
    }
}

// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Independent two-source packed blends. Masked composition has a signed
//!14-bit destination residual; the unmasked profile omits destination RGB.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackedPairBlend {
    pub sources: [[i32; 4]; 2],
    pub destination: [i32; 2],
    /// Each source is independently0..256. Their sum may exceed256.
    pub weights256: [u16; 2],
    /// With masks, alpha0 skips,255 uses the full14-bit weight and partial
    /// alpha uses floor(mask*weight/4). Without masks, ignore both alpha planes.
    pub use_masks: bool,
}
impl PackedPairBlend {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.sources.iter().all(|r| r[2] >= 0 && r[3] >= 0)
                && self.sources[0][2..] == self.sources[1][2..],
            "packed pair matching rectangles"
        );
        ensure!(
            self.sources
                .iter()
                .flatten()
                .chain(&self.destination)
                .all(|n| n.unsigned_abs() <= 1_000_000)
                && self.weights256.iter().all(|n| *n <= 256),
            "packed pair coordinates/weights"
        );
        ensure!(
            self.sources[0][2] as u64 * self.sources[0][3] as u64 <= MAX_EFFECT_PIXELS,
            "packed pair work budget"
        );
        Ok(())
    }
    pub fn apply(
        &self,
        destination: &ImageSurface,
        first: &ImageSurface,
        second: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        for image in [destination, first, second] {
            image.validate()?;
            ensure!(
                u64::from(image.width) * u64::from(image.height) <= MAX_EFFECT_PIXELS,
                "packed pair image budget"
            );
        }
        check()?;
        let mut out = destination.clone();
        self.apply_to(&mut out, first, second, check)?;
        Ok(out)
    }
    /// Shared executor for a validated ordered batch, retaining the same
    /// source leases while prior writes contribute destination residuals.
    pub(crate) fn apply_to(
        &self,
        out: &mut ImageSurface,
        first: &ImageSurface,
        second: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<()> {
        let [ax, ay, w, h] = self.sources[0].map(i64::from);
        let [bx, by, _, _] = self.sources[1].map(i64::from);
        let [dx, dy] = self.destination.map(i64::from);
        let left = 0.max(-ax).max(-bx).max(-dx);
        let top = 0.max(-ay).max(-by).max(-dy);
        let right = w
            .min(i64::from(first.width) - ax)
            .min(i64::from(second.width) - bx)
            .min(i64::from(out.width) - dx);
        let bottom = h
            .min(i64::from(first.height) - ay)
            .min(i64::from(second.height) - by)
            .min(i64::from(out.height) - dy);
        for y in top..bottom {
            check()?;
            for x in left..right {
                if x % 4096 == 0 {
                    check()?;
                }
                let ai = (((ay + y) * i64::from(first.width) + ax + x) * 4) as usize;
                let bi = (((by + y) * i64::from(second.width) + bx + x) * 4) as usize;
                let di = (((dy + y) * i64::from(out.width) + dx + x) * 4) as usize;
                let (a, b) = if self.use_masks {
                    let mask_a = first.rgba[ai + 3];
                    let mask_b = second.rgba[bi + 3];
                    if mask_a == 0 && mask_b == 0 {
                        continue;
                    }
                    let coverage = |mask: u8, weight: u16| -> i64 {
                        if mask == 255 {
                            i64::from(weight) << 6
                        } else {
                            i64::from(mask) * i64::from(weight) >> 2
                        }
                    };
                    (
                        coverage(mask_a, self.weights256[0]),
                        coverage(mask_b, self.weights256[1]),
                    )
                } else {
                    (
                        i64::from(self.weights256[0]) << 6,
                        i64::from(self.weights256[1]) << 6,
                    )
                };
                for c in 0..3 {
                    let mut value = (i64::from(first.rgba[ai + c]) * a >> 14)
                        + (i64::from(second.rgba[bi + c]) * b >> 14);
                    if self.use_masks {
                        // Signed pmulhw floors negative residual products;
                        // division toward zero would be observably different.
                        value += i64::from(out.rgba[di + c]) * (16384 - a - b) >> 14;
                    }
                    out.rgba[di + c] = value.clamp(0, 255) as u8;
                }
            }
        }
        check()?;
        Ok(())
    }
}

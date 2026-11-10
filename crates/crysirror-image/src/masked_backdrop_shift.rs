// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Thresholded background displacement using an independent mask image.
//! All reads use the admission-time destination; alpha is retained.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaskedBackdropShift {
    pub mask: [i32; 4],
    pub destination: [i32; 2],
    pub offset: [i32; 2],
    pub channel: u8,
    pub threshold: u8,
}
impl MaskedBackdropShift {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.mask[2] >= 0 && self.mask[3] >= 0 && self.channel < 4,
            "masked backdrop extent/channel"
        );
        ensure!(
            self.mask
                .iter()
                .chain(&self.destination)
                .chain(&self.offset)
                .all(|n| n.unsigned_abs() <= 1_000_000),
            "masked backdrop coordinates"
        );
        ensure!(
            self.mask[2] as u64 * self.mask[3] as u64 <= MAX_EFFECT_PIXELS,
            "masked backdrop work budget"
        );
        Ok(())
    }
    pub fn apply(
        &self,
        destination: &ImageSurface,
        mask: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        destination.validate()?;
        mask.validate()?;
        ensure!(
            [destination, mask]
                .iter()
                .all(|i| u64::from(i.width) * u64::from(i.height) <= MAX_EFFECT_PIXELS),
            "masked backdrop image budget"
        );
        check()?;
        let mut out = destination.clone();
        let [mx, my, w, h] = self.mask.map(i64::from);
        let [dx, dy] = self.destination.map(i64::from);
        let [ox, oy] = self.offset.map(i64::from);
        let left = 0.max(-mx).max(-dx).max(-dx - ox);
        let top = 0.max(-my).max(-dy).max(-dy - oy);
        let right = w
            .min(i64::from(mask.width) - mx)
            .min(i64::from(out.width) - dx)
            .min(i64::from(out.width) - dx - ox);
        let bottom = h
            .min(i64::from(mask.height) - my)
            .min(i64::from(out.height) - dy)
            .min(i64::from(out.height) - dy - oy);
        for y in top..bottom {
            check()?;
            for x in left..right {
                if x % 4096 == 0 {
                    check()?;
                }
                let mi = (((my + y) * i64::from(mask.width) + mx + x) * 4) as usize
                    + usize::from(self.channel);
                if mask.rgba[mi] < self.threshold {
                    continue;
                }
                let from = (((dy + y + oy) * i64::from(out.width) + dx + x + ox) * 4) as usize;
                let to = (((dy + y) * i64::from(out.width) + dx + x) * 4) as usize;
                out.rgba[to..to + 3].copy_from_slice(&destination.rgba[from..from + 3]);
            }
        }
        check()?;
        Ok(out)
    }
}

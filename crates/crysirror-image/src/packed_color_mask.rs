// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Stored-channel coverage transforms over three immutable engine images.
//! Word multiplication/addition and unsigned saturation are explicit; clipping
//! never changes source/mask alignment. Destination alpha remains untouched.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackedColorMask {
    pub source: [i32; 4],
    pub destination: [i32; 2],
    /// Mask texel aligned with the upper-left source rectangle before clipping.
    pub mask: [i32; 2],
    pub reverse: bool,
    pub multiplier: u16,
    pub offset: u16,
    /// Wrapping addition followed by unsigned saturated subtraction emulates a
    /// signed lower bound while retaining the full packed-word input domain.
    pub nonnegative_bias: u16,
    /// Upper coverage limit after the lower-bound operation;0..=256.
    pub limit: u16,
}
impl PackedColorMask {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.source
                .iter()
                .chain(&self.destination)
                .chain(&self.mask)
                .all(|v| v.unsigned_abs() <= 1_000_000),
            "packed colour mask coordinates"
        );
        ensure!(
            self.source[2] >= 0 && self.source[3] >= 0 && self.limit <= 256,
            "packed colour mask extent/coverage"
        );
        ensure!(
            self.source[2] as u64 * self.source[3] as u64 <= MAX_EFFECT_PIXELS * 4,
            "packed colour mask work budget"
        );
        Ok(())
    }
    pub fn apply(
        &self,
        destination: &ImageSurface,
        source: &ImageSurface,
        mask: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        for image in [destination, source, mask] {
            image.validate()?;
            ensure!(
                u64::from(image.width) * u64::from(image.height) <= MAX_EFFECT_PIXELS,
                "packed colour mask image budget"
            );
        }
        check()?;
        let mut output = destination.clone();
        let [sx, sy, w, h] = self.source.map(i64::from);
        let [dx, dy] = self.destination.map(i64::from);
        let [mx, my] = self.mask.map(i64::from);
        let left = 0.max(-sx).max(-dx).max(-mx);
        let top = 0.max(-sy).max(-dy).max(-my);
        let right = w
            .min(i64::from(source.width) - sx)
            .min(i64::from(destination.width) - dx)
            .min(i64::from(mask.width) - mx);
        let bottom = h
            .min(i64::from(source.height) - sy)
            .min(i64::from(destination.height) - dy)
            .min(i64::from(mask.height) - my);
        for y in top..bottom {
            check()?;
            for x in left..right {
                let si = (((sy + y) * i64::from(source.width) + sx + x) * 4) as usize;
                let mi = (((my + y) * i64::from(mask.width) + mx + x) * 4) as usize;
                let di = (((dy + y) * i64::from(destination.width) + dx + x) * 4) as usize;
                let alpha = u16::from(source.rgba[si + 3]);
                if alpha == 0 {
                    continue;
                }
                for c in 0..3 {
                    let m = u16::from(if self.reverse {
                        255 - mask.rgba[mi + c]
                    } else {
                        mask.rgba[mi + c]
                    });
                    let raw = m
                        .wrapping_mul(self.multiplier)
                        .wrapping_add(self.offset)
                        .wrapping_add(self.nonnegative_bias)
                        .saturating_sub(self.nonnegative_bias);
                    let ceiling = u16::MAX - self.limit;
                    let coverage = raw.saturating_add(ceiling).saturating_sub(ceiling);
                    let a = coverage.wrapping_mul(alpha) >> 8;
                    let b = 256u16.wrapping_sub(a);
                    output.rgba[di + c] = (u16::from(source.rgba[si + c])
                        .wrapping_mul(a)
                        .wrapping_add(u16::from(output.rgba[di + c]).wrapping_mul(b))
                        >> 8) as u8;
                }
            }
        }
        check()?;
        Ok(output)
    }
}

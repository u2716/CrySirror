// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Two coverage planes and optional stored-RGB transformation, with explicit
//! low-word multiplication and addition. Three immutable image inputs share one
//! bounded clipping/cancellation domain; callers supply geometry and time.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackedDualMask {
    pub source: [i32; 4],
    pub destination: [i32; 2],
    pub mask: [i32; 2],
    /// Independent low-word scale after alpha1*(alpha2+1)/256.
    pub weight: u16,
    /// Optional source RGB multiply/256 followed by packed-word addition.
    /// RGB lanes remain words until their final coverage product.
    pub source_color: Option<([u8; 3], [u16; 3])>,
}
impl PackedDualMask {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.source
                .iter()
                .chain(&self.destination)
                .chain(&self.mask)
                .all(|v| v.unsigned_abs() <= 1_000_000),
            "packed dual mask coordinates"
        );
        ensure!(
            self.source[2] >= 0 && self.source[3] >= 0,
            "packed dual mask extent"
        );
        ensure!(
            self.source[2] as u64 * self.source[3] as u64 <= MAX_EFFECT_PIXELS * 4,
            "packed dual mask work budget"
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
                "packed dual mask image budget"
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
                let first = u16::from(source.rgba[si + 3]);
                let second = u16::from(mask.rgba[mi + 3]);
                if first == 0 || second == 0 {
                    continue;
                }
                let a = (first * (second + 1) >> 8).wrapping_mul(self.weight) >> 8;
                let b = 256u16.wrapping_sub(a);
                for c in 0..3 {
                    let mut value = u16::from(source.rgba[si + c]);
                    if let Some((mul, add)) = &self.source_color {
                        value = (value * u16::from(mul[c]) >> 8).wrapping_add(add[c]);
                    }
                    output.rgba[di + c] = (value
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

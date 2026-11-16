// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Scaled saturated addition/subtraction followed by separately truncated
//! coverage products. Both images remain immutable; alpha is retained.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
pub(crate) fn pixel(destination: &mut [u8], source: &[u8], scale: u16, subtract: bool) {
    let alpha = u32::from(source[3]);
    if alpha == 0 {
        return;
    }
    for c in 0..3 {
        // Signed packed-word multiplication, reconstructed after >>8, then
        // signed-word to unsigned-byte saturation. Accepted legacy positive
        // scales retain the original result; negative word profiles are also
        // available to the generic row-batch math without scalar substitution.
        let value = ((i32::from(source[c]) * i32::from(scale as i16)) >> 8).clamp(0, 255) as u8;
        let old = destination[c];
        let mixed = if subtract {
            old.saturating_sub(value)
        } else {
            old.saturating_add(value)
        };
        destination[c] =
            ((u32::from(mixed) * alpha >> 8) + (u32::from(old) * (256 - alpha) >> 8)) as u8;
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaskedAddSubtract256 {
    pub source: [i32; 4],
    pub destination: [i32; 2],
    pub scale: u16,
    pub subtract: bool,
}
impl MaskedAddSubtract256 {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.source
                .iter()
                .chain(&self.destination)
                .all(|v| v.unsigned_abs() <= 1_000_000)
                && self.source[2] >= 0
                && self.source[3] >= 0,
            "masked add rectangles"
        );
        ensure!(self.scale <= 32767, "masked add signed Q8 scale");
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
        ensure!(
            [destination, source]
                .iter()
                .all(|i| u64::from(i.width) * u64::from(i.height) <= MAX_EFFECT_PIXELS),
            "masked add pixel budget"
        );
        let [sx, sy, w, h] = self.source.map(i64::from);
        let [dx, dy] = self.destination.map(i64::from);
        let x0 = 0.max(-sx).max(-dx);
        let y0 = 0.max(-sy).max(-dy);
        let x1 = w
            .min(i64::from(source.width) - sx)
            .min(i64::from(destination.width) - dx);
        let y1 = h
            .min(i64::from(source.height) - sy)
            .min(i64::from(destination.height) - dy);
        check()?;
        let mut output = destination.clone();
        for y in y0..y1 {
            check()?;
            for x in x0..x1 {
                let si = (((sy + y) * i64::from(source.width) + sx + x) * 4) as usize;
                let di = (((dy + y) * i64::from(output.width) + dx + x) * 4) as usize;
                pixel(
                    &mut output.rgba[di..di + 4],
                    &source.rgba[si..si + 4],
                    self.scale,
                    self.subtract,
                );
            }
        }
        check()?;
        Ok(output)
    }
}

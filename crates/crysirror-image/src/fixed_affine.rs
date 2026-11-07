// SPDX-License-Identifier: MPL-2.0
//! Explicit fixed-point inverse affine sampling into an immutable destination.
//! A destination rectangle and integer coefficients define coverage; there is
//! no transformed-quad expansion, trigonometry or hidden sampling offset.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS, rgb_lut::ConstantByteBlend};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixedAffineMath {
    Copy,
    CopyRgb,
    MaskHoldAlphaScalar256,
}
impl FixedAffineMath {
    pub(crate) fn apply_pixel(self, destination: &mut [u8], source: &[u8]) {
        match self {
            Self::Copy => destination.copy_from_slice(source),
            Self::CopyRgb => destination[..3].copy_from_slice(&source[..3]),
            Self::MaskHoldAlphaScalar256 => {
                for lane in 0..3 {
                    destination[lane] = ConstantByteBlend::Scalar256.channel(
                        destination[lane],
                        source[lane],
                        source[3],
                    );
                }
            }
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixedAffineNearest {
    /// Destination XYWH; clipping keeps coordinates relative to `origin`.
    pub rectangle: [i32; 4],
    pub origin: [i32; 2],
    /// Rows of a fixed-point inverse matrix. Source coordinate i is
    /// (linear[i][0]*(x-origin.x)+linear[i][1]*(y-origin.y)+translation[i])
    /// arithmetic-shifted by fractional_bits. Negative samples are skipped.
    pub linear: [[i32; 2]; 2],
    pub translation: [i64; 2],
    pub fractional_bits: u8,
    pub math: FixedAffineMath,
}
impl FixedAffineNearest {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.fractional_bits <= 24, "fixed affine fraction bits");
        ensure!(
            self.rectangle
                .iter()
                .chain(&self.origin)
                .all(|value| value.unsigned_abs() <= 1_000_000)
                && self.rectangle[2..].iter().all(|value| *value >= 0),
            "fixed affine rectangle/origin"
        );
        ensure!(
            self.linear
                .iter()
                .flatten()
                .all(|value| value.unsigned_abs() <= 0x4000_0000)
                && self
                    .translation
                    .iter()
                    .all(|value| value.unsigned_abs() <= (1u64 << 48)),
            "fixed affine coefficient range"
        );
        ensure!(
            u64::try_from(self.rectangle[2])? * u64::try_from(self.rectangle[3])?
                <= MAX_EFFECT_PIXELS * 4,
            "fixed affine work budget"
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
        ensure!(
            u64::from(destination.width) * u64::from(destination.height) <= MAX_EFFECT_PIXELS
                && u64::from(source.width) * u64::from(source.height) <= MAX_EFFECT_PIXELS,
            "fixed affine pixel budget"
        );
        check()?;
        let mut output = destination.clone();
        let [x, y, width, height] = self.rectangle.map(i64::from);
        let end_x = (x + width).min(i64::from(output.width));
        let end_y = (y + height).min(i64::from(output.height));
        for dy in y.max(0)..end_y {
            check()?;
            for dx in x.max(0)..end_x {
                let local = [
                    dx - i64::from(self.origin[0]),
                    dy - i64::from(self.origin[1]),
                ];
                //Validation bounds each product and sum within i64; signed
                //right shift specifies floor rounding for negative values.
                let point = std::array::from_fn::<_, 2, _>(|axis| {
                    (i64::from(self.linear[axis][0]) * local[0]
                        + i64::from(self.linear[axis][1]) * local[1]
                        + self.translation[axis])
                        >> self.fractional_bits
                });
                if point[0] < 0
                    || point[1] < 0
                    || point[0] >= i64::from(source.width)
                    || point[1] >= i64::from(source.height)
                {
                    continue;
                }
                let from = ((point[1] * i64::from(source.width) + point[0]) * 4) as usize;
                let to = ((dy * i64::from(output.width) + dx) * 4) as usize;
                let src = &source.rgba[from..from + 4];
                let dst = &mut output.rgba[to..to + 4];
                self.math.apply_pixel(dst, src);
            }
        }
        check()?;
        Ok(output)
    }
}

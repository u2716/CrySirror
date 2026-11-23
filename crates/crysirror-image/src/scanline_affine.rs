// Copyright (c)2026 CrySirror contributors. MPL-2.0.
//! Bounded nearest sampling with an independent fixed origin for each row.
//! Unlike a global affine matrix, caller-truncated row origins are preserved.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS, fixed_affine::FixedAffineMath};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

/// Fixed-coordinate arithmetic before the signed fractional shift.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanlineCoordinateWrap {
    /// Wide checked coordinates; out-of-image texels leave destination intact.
    #[default]
    None,
    /// Keep the low signed32 bits before shifting. This only wraps numerical
    /// sample coordinates; the resulting texel still undergoes bounds checks.
    I32,
}
/// An immutable source sampled over an existing immutable destination.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScanlineAffineNearest {
    /// Destination XYWH, clipped without changing the declared row/column origin.
    pub rectangle: [i32; 4],
    /// One source fixed-point XY at rectangle.x for every declared row.
    pub row_origins: Vec<[i64; 2]>,
    /// Source fixed-point increment for one destination column.
    pub column_step: [i32; 2],
    pub fractional_bits: u8,
    #[serde(default)]
    pub coordinate_wrap: ScanlineCoordinateWrap,
    pub math: FixedAffineMath,
}
impl ScanlineAffineNearest {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.fractional_bits <= 24, "scanline affine fraction bits");
        ensure!(
            self.rectangle[..2]
                .iter()
                .all(|v| v.unsigned_abs() <= 1_000_000)
                && self.rectangle[2..].iter().all(|v| (0..=8192).contains(v)),
            "scanline affine rectangle"
        );
        ensure!(
            self.row_origins.len() == self.rectangle[3] as usize,
            "scanline affine row count"
        );
        ensure!(
            self.row_origins
                .iter()
                .flatten()
                .all(|v| v.unsigned_abs() <= (1u64 << 48)),
            "scanline affine origin range"
        );
        ensure!(
            u64::try_from(self.rectangle[2])? * u64::try_from(self.rectangle[3])?
                <= MAX_EFFECT_PIXELS,
            "scanline affine work budget"
        );
        //Each origin plus at most8191 signed32 increments fits comfortably in
        //i64, including clipped rows/columns. No unchecked pointer arithmetic.
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
            "scanline affine image budget"
        );
        check()?;
        let mut output = destination.clone();
        let [x, y, w, h] = self.rectangle.map(i64::from);
        let end_x = (x + w).min(i64::from(output.width));
        let end_y = (y + h).min(i64::from(output.height));
        for dy in y.max(0)..end_y {
            check()?;
            let origin = self.row_origins[(dy - y) as usize];
            for dx in x.max(0)..end_x {
                let point = std::array::from_fn::<_, 2, _>(|axis| {
                    let fixed = origin[axis] + (dx - x) * i64::from(self.column_step[axis]);
                    let fixed = match self.coordinate_wrap {
                        ScanlineCoordinateWrap::None => fixed,
                        ScanlineCoordinateWrap::I32 => i64::from(fixed as i32),
                    };
                    fixed >> self.fractional_bits
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

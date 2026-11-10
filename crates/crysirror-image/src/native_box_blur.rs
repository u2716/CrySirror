// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Clipped box sums with explicit packed-integer normalisation profiles.
//! Source samples are immutable; only the specified destination rectangle is
//! affected. Empty neighbourhoods preserve the destination, including alpha.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeBoxBlurMath {
    /// RGB sums use one sample per covered pixel; floor(sum/2) is multiplied
    /// by min(32768/count,32767) and shifted by14.
    CoveredRgb,
    /// RGB is first multiplied by source alpha. Normalisation reproduces
    /// separate packed-word products, then composites with the retained RGB.
    PremultipliedMask256,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeBoxBlur {
    /// Source position corresponding to the top-left destination pixel.
    pub source: [i32; 2],
    pub destination: [i32; 4],
    /// Half-open source coverage bounds [left,top,right,bottom].
    pub source_bounds: [u32; 4],
    /// Window offsets are [-floor(size/2),size-floor(size/2)) on each axis.
    pub window: [u16; 2],
    pub math: NativeBoxBlurMath,
    /// Initial sums truncate at bottom. Subsequent sliding updates beyond
    /// bottom read the last row instead; top and horizontal edges truncate.
    pub repeat_bottom_updates: bool,
}

impl NativeBoxBlur {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.window.iter().all(|n| (1..=256).contains(n))
                && u32::from(self.window[0]) * u32::from(self.window[1]) <= 256,
            "native box blur window"
        );
        ensure!(
            self.source
                .iter()
                .chain(&self.destination)
                .all(|n| n.unsigned_abs() <= 1_000_000),
            "native box blur coordinates"
        );
        ensure!(
            self.destination[2] >= 0
                && self.destination[3] >= 0
                && u64::from(self.destination[2] as u32) * u64::from(self.destination[3] as u32)
                    <= MAX_EFFECT_PIXELS,
            "native box blur rectangle"
        );
        ensure!(
            self.source_bounds[0] <= self.source_bounds[2]
                && self.source_bounds[1] <= self.source_bounds[3],
            "native box blur coverage bounds"
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
        source.validate()?;
        destination.validate()?;
        ensure!(
            [source, destination]
                .iter()
                .all(|image| u64::from(image.width) * u64::from(image.height) <= MAX_EFFECT_PIXELS),
            "native box blur image budget"
        );
        ensure!(
            self.source_bounds[2] <= source.width && self.source_bounds[3] <= source.height,
            "native box blur coverage outside source"
        );
        check()?;
        let mut output = destination.clone();
        let [left, top, right, bottom] = self.source_bounds.map(i64::from);
        if left == right || top == bottom || self.destination[2] == 0 || self.destination[3] == 0 {
            return Ok(output);
        }
        let [dx, dy, width, height] = self.destination.map(i64::from);
        // Clipping must precede vertical initialisation. Skipping hidden rows
        // after initialising would change repeat_bottom_updates' first sum.
        let x0 = 0.max(-dx);
        let x1 = width.min(i64::from(destination.width) - dx);
        let y0 = 0.max(-dy);
        let y1 = height.min(i64::from(destination.height) - dy);
        if x0 >= x1 || y0 >= y1 {
            return Ok(output);
        }
        let sx = i64::from(self.source[0]) + x0;
        let sy = i64::from(self.source[1]) + y0;
        let width = x1 - x0;
        let height = y1 - y0;
        let [wx, wy] = self.window.map(i64::from);
        let (before_x, before_y) = (wx / 2, wy / 2);
        let (after_x, after_y) = (wx - before_x, wy - before_y);
        let start_x = (sx - before_x).max(left);
        let end_x = (sx + width - 1 + after_x).min(right);
        if start_x >= end_x {
            return Ok(output);
        }
        let mut columns = vec![[0u32; 4]; (end_x - start_x) as usize];
        let sample = |x: i64, y: i64| -> [u32; 4] {
            let at = ((y as usize) * source.width as usize + x as usize) * 4;
            let alpha = match self.math {
                NativeBoxBlurMath::CoveredRgb => 1,
                NativeBoxBlurMath::PremultipliedMask256 => u32::from(source.rgba[at + 3]),
            };
            [
                u32::from(source.rgba[at]) * alpha,
                u32::from(source.rgba[at + 1]) * alpha,
                u32::from(source.rgba[at + 2]) * alpha,
                alpha,
            ]
        };
        for y in (sy - before_y).max(top)..(sy + after_y).min(bottom) {
            check()?;
            for (column, x) in columns.iter_mut().zip(start_x..end_x) {
                let value = sample(x, y);
                for c in 0..4 {
                    column[c] = column[c].wrapping_add(value[c]);
                }
            }
        }
        let area = u32::from(self.window[0]) * u32::from(self.window[1]);
        let factor = (65536 / area).min(32767);
        let read_column = |columns: &[[u32; 4]], x: i64| -> [u32; 4] {
            if (start_x..end_x).contains(&x) {
                columns[(x - start_x) as usize]
            } else {
                [0; 4]
            }
        };
        for row in 0..height {
            check()?;
            let mut sums = [0u32; 4];
            for x in sx - before_x..sx + after_x {
                let value = read_column(&columns, x);
                for c in 0..4 {
                    sums[c] = sums[c].wrapping_add(value[c]);
                }
            }
            for x in 0..width {
                if x % 4096 == 0 {
                    check()?;
                }
                let at = (((dy + y0 + row) as usize) * destination.width as usize
                    + (dx + x0 + x) as usize)
                    * 4;
                if sums[3] != 0 {
                    match self.math {
                        NativeBoxBlurMath::CoveredRgb => {
                            // Window area<=256 bounds packed16 sums/count.
                            ensure!(sums[3] <= 256, "native box blur sample count");
                            let reciprocal = (32768 / sums[3]).min(32767);
                            for c in 0..3 {
                                output.rgba[at + c] =
                                    (((sums[c] >> 1) * reciprocal) >> 14).min(255) as u8;
                            }
                        }
                        NativeBoxBlurMath::PremultipliedMask256 => {
                            let average_even = (((sums[3] >> 1) * factor) >> 16) * 2;
                            let complement = 254u32.wrapping_sub(average_even) as u16;
                            for c in 0..3 {
                                let high = ((sums[c] >> 16) * factor) as u16;
                                let low = ((((sums[c] & 65535) >> 1) * factor) >> 16) as u16;
                                let normal = high.wrapping_add(low.wrapping_mul(2));
                                let retained =
                                    u16::from(destination.rgba[at + c]).wrapping_mul(complement);
                                output.rgba[at + c] = (normal.wrapping_add(retained) >> 8) as u8;
                            }
                        }
                    }
                }
                let remove = read_column(&columns, sx + x - before_x);
                let add = read_column(&columns, sx + x + after_x);
                for c in 0..4 {
                    sums[c] = sums[c].wrapping_sub(remove[c]).wrapping_add(add[c]);
                }
            }
            if row + 1 == height {
                break;
            }
            let effective_row = |y: i64| -> Option<i64> {
                if y < top {
                    None
                } else if y < bottom {
                    Some(y)
                } else if self.repeat_bottom_updates && bottom - 1 != 0 {
                    Some(bottom - 1)
                } else {
                    None
                }
            };
            let remove_y = effective_row(sy + row - before_y);
            let add_y = effective_row(sy + row + after_y);
            for (column, x) in columns.iter_mut().zip(start_x..end_x) {
                if let Some(y) = remove_y {
                    let value = sample(x, y);
                    for c in 0..4 {
                        column[c] = column[c].wrapping_sub(value[c]);
                    }
                }
                if let Some(y) = add_y {
                    let value = sample(x, y);
                    for c in 0..4 {
                        column[c] = column[c].wrapping_add(value[c]);
                    }
                }
            }
        }
        check()?;
        Ok(output)
    }
}

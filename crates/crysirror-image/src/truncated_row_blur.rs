// SPDX-License-Identifier: MPL-2.0
//! Horizontal box mean over [x-radius, x+radius] limited to the row. With
//! `row_overrun` the right limit is one sample past the row: that sample is
//! the first pixel of the next row (zero after the last row), and it counts
//! in the divisor. This reproduces legacy blitters that bound the window by
//! `x <= width` instead of `x < width`. RGB means truncate; alpha becomes 255.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TruncatedRowBlur {
    pub radius: u32,
    #[serde(default)]
    pub row_overrun: bool,
}
impl TruncatedRowBlur {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.radius <= 16384, "truncated row blur radius");
        Ok(())
    }
    pub fn apply(&self, input: &ImageSurface, check: &dyn Fn() -> Result<()>) -> Result<ImageSurface> {
        self.validate()?;
        input.validate()?;
        ensure!(
            u64::from(input.width) * u64::from(input.height) <= MAX_EFFECT_PIXELS,
            "truncated row blur pixel budget"
        );
        let (w, h, r) = (input.width as i64, input.height as usize, i64::from(self.radius));
        let mut out = input.clone();
        let extended = w + i64::from(self.row_overrun);
        let mut prefix = vec![[0u64; 3]; extended as usize + 1];
        for y in 0..h {
            if y % 64 == 0 {
                check()?;
            }
            for i in 0..extended as usize {
                let pixel = if (i as i64) < w {
                    let at = (y * w as usize + i) * 4;
                    [input.rgba[at], input.rgba[at + 1], input.rgba[at + 2]]
                } else if y + 1 < h {
                    let at = (y + 1) * w as usize * 4;
                    [input.rgba[at], input.rgba[at + 1], input.rgba[at + 2]]
                } else {
                    [0; 3]
                };
                for c in 0..3 {
                    prefix[i + 1][c] = prefix[i][c] + u64::from(pixel[c]);
                }
            }
            for x in 0..w {
                let lo = (x - r).max(0);
                let hi = (x + r).min(extended - 1);
                let count = (hi - lo + 1) as u64;
                let at = (y * w as usize + x as usize) * 4;
                for c in 0..3 {
                    let sum = prefix[hi as usize + 1][c] - prefix[lo as usize][c];
                    out.rgba[at + c] = (sum / count) as u8;
                }
                out.rgba[at + 3] = 255;
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn edges_truncate_and_overrun_reads_next_row() {
        let mut rgba = vec![0u8; 4 * 2 * 4];
        for (i, p) in rgba.chunks_exact_mut(4).enumerate() {
            p[0] = (i * 10) as u8;
        }
        let input = ImageSurface { width: 4, height: 2, rgba };
        let out = TruncatedRowBlur { radius: 1, row_overrun: true }.apply(&input, &|| Ok(())).unwrap();
        // x=0: (0+10)/2; x=3: (20+30+40)/3 with row1 pixel0 = 40.
        assert_eq!(out.rgba[0], 5);
        assert_eq!(out.rgba[12], 30);
        // last row x=3: (60+70+0)/3.
        assert_eq!(out.rgba[28], 43);
    }
}

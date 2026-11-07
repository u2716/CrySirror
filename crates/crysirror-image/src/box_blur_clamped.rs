//! Repeated two-dimensional box averages of stored RGBA. Edge texels repeat;
//! one division follows the complete2D sum, with no intermediate row rounding.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoxBlurClamped {
    pub radii: [u16; 2],
    pub passes: u8,
}
impl BoxBlurClamped {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.radii.iter().all(|v| *v <= 8192),
            "clamped blur radius limit"
        );
        ensure!((1..=8).contains(&self.passes), "clamped blur pass limit");
        Ok(())
    }
    pub fn apply(
        &self,
        input: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        input.validate()?;
        ensure!(
            u64::from(input.width) * u64::from(input.height) <= MAX_EFFECT_PIXELS,
            "clamped blur pixel budget"
        );
        check()?;
        let [rx, ry] = self.radii.map(i64::from);
        let (w, h) = (i64::from(input.width), i64::from(input.height));
        let divisor = ((2 * rx + 1) * (2 * ry + 1)) as u64;
        let mut current = input.clone();
        for _ in 0..self.passes {
            check()?;
            let mut columns = vec![[0u64; 4]; w as usize];
            let row = |y: i64| &current.rgba[(y * w * 4) as usize..((y + 1) * w * 4) as usize];
            // Account for repeated endpoints by multiplication, so a tiny
            //image with a large radius does not traverse offscreen windows.
            for y in 0..=ry.min(h - 1) {
                check()?;
                for (sum, pixel) in columns.iter_mut().zip(row(y).chunks_exact(4)) {
                    for c in 0..4 {
                        sum[c] += u64::from(pixel[c]);
                    }
                }
            }
            for (x, sum) in columns.iter_mut().enumerate() {
                for c in 0..4 {
                    sum[c] += u64::from(current.rgba[x * 4 + c]) * ry as u64;
                    sum[c] += u64::from(current.rgba[((h - 1) * w * 4) as usize + x * 4 + c])
                        * (ry - (h - 1)).max(0) as u64;
                }
            }
            let mut output = ImageSurface {
                width: input.width,
                height: input.height,
                rgba: vec![0; current.rgba.len()],
            };
            for y in 0..h {
                check()?;
                let mut sums = std::array::from_fn::<_, 4, _>(|c| {
                    columns[0][c] * rx as u64
                        + columns[(w - 1) as usize][c] * (rx - (w - 1)).max(0) as u64
                });
                for col in &columns[..=rx.min(w - 1) as usize] {
                    for c in 0..4 {
                        sums[c] += col[c];
                    }
                }
                for x in 0..w {
                    let at = ((y * w + x) * 4) as usize;
                    for c in 0..4 {
                        output.rgba[at + c] = (sums[c] / divisor) as u8;
                    }
                    let old = columns[(x - rx).clamp(0, w - 1) as usize];
                    let new = columns[(x + rx + 1).clamp(0, w - 1) as usize];
                    for c in 0..4 {
                        sums[c] = sums[c] - old[c] + new[c];
                    }
                }
                if y + 1 < h {
                    let old = row((y - ry).clamp(0, h - 1));
                    let new = row((y + ry + 1).clamp(0, h - 1));
                    for (x, sum) in columns.iter_mut().enumerate() {
                        for c in 0..4 {
                            sum[c] = sum[c] - u64::from(old[x * 4 + c]) + u64::from(new[x * 4 + c]);
                        }
                    }
                }
            }
            current = output;
        }
        check()?;
        Ok(current)
    }
}

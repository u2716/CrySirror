// SPDX-License-Identifier: MPL-2.0
//! Random horizontal runs: each row is cut into runs of `rand() % span + 1`
//! pixels from an MSVC-style linear congruential stream, and every run is
//! filled with one colour taken from the source run (its middle pixel or its
//! truncated channel mean). The last run of a row is clipped to the width.
//! The stream state after the whole image is available without pixels, so a
//! caller can keep a process-wide generator in step.
use crate::temporal_pattern::PatternRng;
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunColor {
    /// The pixel at run start + length / 2.
    Middle,
    /// Truncated per-channel mean of the run.
    Mean,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RandomRuns {
    /// Run lengths are rand() % span + 1. A span of 1 copies the source.
    pub span: u32,
    pub color: RunColor,
    /// Generator state before the first draw.
    pub state: u32,
}
impl RandomRuns {
    pub fn validate(&self) -> Result<()> {
        ensure!((1..=65536).contains(&self.span), "random run span");
        Ok(())
    }
    /// Visits (y, x, length) in draw order and returns the final state.
    fn runs(&self, width: u32, height: u32, mut visit: impl FnMut(u32, u32, u32)) -> Result<u32> {
        let mut rng = PatternRng::msvc(self.state);
        if self.span == 1 {
            return Ok(rng.state);
        }
        let (w, n) = (width as i64, i64::from(self.span));
        for y in 0..height {
            let mut x = 0i64;
            if w - n > 0 {
                loop {
                    let len = i64::from(rng.next_draw()?) % n + 1;
                    visit(y, x as u32, len as u32);
                    x += len;
                    if x >= w - n {
                        break;
                    }
                }
            }
            if x < w {
                loop {
                    let step = i64::from(rng.next_draw()?) % n + 1;
                    let end = x + step;
                    let len = if end <= w { step } else { w - x };
                    visit(y, x as u32, len as u32);
                    x = end;
                    if end >= w {
                        break;
                    }
                }
            }
        }
        Ok(rng.state)
    }
    /// Generator state after processing a `width` x `height` image.
    pub fn state_after(&self, width: u32, height: u32) -> Result<u32> {
        self.validate()?;
        self.runs(width, height, |_, _, _| {})
    }
    pub fn apply(&self, input: &ImageSurface, check: &dyn Fn() -> Result<()>) -> Result<ImageSurface> {
        self.validate()?;
        input.validate()?;
        ensure!(
            u64::from(input.width) * u64::from(input.height) <= MAX_EFFECT_PIXELS,
            "random run pixel budget"
        );
        check()?;
        if self.span == 1 {
            return Ok(input.clone());
        }
        let mut out = input.clone();
        let width = input.width as usize;
        self.runs(input.width, input.height, |y, x, len| {
            let row = y as usize * width;
            let (x, len) = (x as usize, len as usize);
            let color = match self.color {
                RunColor::Middle => {
                    let at = (row + x + len / 2) * 4;
                    [input.rgba[at], input.rgba[at + 1], input.rgba[at + 2]]
                }
                RunColor::Mean => {
                    let mut sums = [0usize; 3];
                    for i in 0..len {
                        let at = (row + x + i) * 4;
                        for c in 0..3 {
                            sums[c] += usize::from(input.rgba[at + c]);
                        }
                    }
                    sums.map(|s| (s / len) as u8)
                }
            };
            for i in 0..len {
                let at = (row + x + i) * 4;
                out.rgba[at..at + 3].copy_from_slice(&color);
                out.rgba[at + 3] = 255;
            }
        })?;
        check()?;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runs_cover_rows_and_report_state() {
        let input = ImageSurface { width: 64, height: 2, rgba: (0..64 * 2 * 4).map(|v| v as u8).collect() };
        let spec = RandomRuns { span: 16, color: RunColor::Mean, state: 7 };
        let out = spec.apply(&input, &|| Ok(())).unwrap();
        assert_eq!(out.rgba.len(), input.rgba.len());
        let mut covered = 0;
        let end = spec.runs(64, 2, |_, _, len| covered += len).unwrap();
        assert_eq!(covered, 128);
        assert_eq!(end, spec.state_after(64, 2).unwrap());
        assert_ne!(end, 7);
    }
}

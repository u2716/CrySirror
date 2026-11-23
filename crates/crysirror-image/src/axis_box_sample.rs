// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! One-axis sliding box windows with quantized, output-delayed phase changes.
//! RGB sums retain packed-word truncation; source and destination are immutable
//! admission-time resources. The alpha plane belongs to the destination.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AxisBoxSample {
    pub source: [i32; 2],
    pub vertical: bool,
    pub window: u16,
    pub phase: [i32; 2],
    /// Vertical packed-pair loops leave an odd final destination column intact.
    pub paired_columns: bool,
}
impl AxisBoxSample {
    pub fn validate(&self) -> Result<()> {
        ensure!((2..=255).contains(&self.window), "axis-box window");
        ensure!(
            self.source
                .iter()
                .chain(&self.phase)
                .all(|v| v.unsigned_abs() <= 1_000_000),
            "axis-box phase budget"
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
        for image in [destination, source] {
            ensure!(
                u64::from(image.width) * u64::from(image.height) <= MAX_EFFECT_PIXELS,
                "axis-box image budget"
            );
        }
        check()?;
        let mut out = destination.clone();
        let [major, mut cross] = if self.vertical {
            [destination.height, destination.width]
        } else {
            [destination.width, destination.height]
        };
        if self.vertical && self.paired_columns {
            cross &= !1;
        }
        let read = |along: i64, across: u32, c: usize| {
            let [x, y] = if self.vertical {
                [
                    i64::from(self.source[0]) + i64::from(across),
                    i64::from(self.source[1]) + along,
                ]
            } else {
                [
                    i64::from(self.source[0]) + along,
                    i64::from(self.source[1]) + i64::from(across),
                ]
            };
            // Tiny native canvases could put the requested window beyond the
            // allocation. Define those formerly invalid reads as zero samples.
            if x < 0 || y < 0 || x >= i64::from(source.width) || y >= i64::from(source.height) {
                0
            } else {
                u16::from(source.rgba[((y as usize * source.width as usize + x as usize) * 4) + c])
            }
        };
        let mut sums = vec![[0u16; 3]; cross as usize];
        for across in 0..cross {
            check()?;
            for offset in 0..i64::from(self.window) {
                for c in 0..3 {
                    sums[across as usize][c] =
                        sums[across as usize][c].wrapping_add(read(offset, across, c));
                }
            }
        }
        let reciprocal = (65536 / u32::from(self.window)).min(32767);
        let mut previous = i64::from(self.phase[0]);
        let mut offset = 0i64;
        for along in 0..major {
            check()?;
            let phase = i64::from(self.phase[0])
                + (i64::from(self.phase[1]) - i64::from(self.phase[0])) * i64::from(along)
                    / i64::from(major);
            for across in 0..cross {
                let [x, y] = if self.vertical {
                    [across, along]
                } else {
                    [along, across]
                };
                let at = (y as usize * destination.width as usize + x as usize) * 4;
                for c in 0..3 {
                    out.rgba[at + c] = (2
                        * ((u32::from(sums[across as usize][c] >> 1) * reciprocal) >> 16))
                        .min(255) as u8;
                }
            }
            // Advance AFTER writing the current line/column, and at most ONE
            // source sample even when the target phase changes by more than1.
            if phase != previous && along + 1 < major {
                for across in 0..cross {
                    for c in 0..3 {
                        sums[across as usize][c] = sums[across as usize][c]
                            .wrapping_sub(read(offset, across, c))
                            .wrapping_add(read(offset + i64::from(self.window), across, c));
                    }
                }
                offset += 1;
                previous = phase;
            }
        }
        check()?;
        Ok(out)
    }
}

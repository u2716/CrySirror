// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Separable integer triangular convolution with explicit transparent padding.
use crate::{effects::MAX_EFFECT_PIXELS, ImageSurface};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriangularBlurMath {
    AlphaAwareFixed24,
    OpaqueFixed16,
}
impl Default for TriangularBlurMath {
    fn default() -> Self {
        Self::AlphaAwareFixed24
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TriangularBlur {
    pub radius: [u8; 2],
    /// Left, top, right, bottom; resource placement remains the caller's policy.
    pub padding: [u32; 4],
    #[serde(default)]
    pub math: TriangularBlurMath,
}
impl TriangularBlur {
    pub fn apply(
        &self,
        input: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        input.validate()?;
        ensure!(
            self.padding.iter().all(|v| *v <= 255),
            "triangular blur padding"
        );
        let width = input
            .width
            .checked_add(self.padding[0])
            .and_then(|v| v.checked_add(self.padding[2]))
            .ok_or_else(|| anyhow::anyhow!("triangular blur width"))?;
        let height = input
            .height
            .checked_add(self.padding[1])
            .and_then(|v| v.checked_add(self.padding[3]))
            .ok_or_else(|| anyhow::anyhow!("triangular blur height"))?;
        let pixels = u64::from(width) * u64::from(height);
        crate::rgba_len(width,height)?;
        ensure!(pixels <= MAX_EFFECT_PIXELS, "triangular blur pixel budget");
        ensure!(
            pixels
                * self
                    .radius
                    .iter()
                    .filter(|r| **r != 0)
                    .map(|r| u64::from(*r) * 2 + 1)
                    .sum::<u64>()
                <= 268_435_456,
            "triangular blur work budget"
        );
        check()?;
        let mut source = ImageSurface {
            width,
            height,
            rgba: vec![0; (pixels * 4) as usize],
        };
        let opaque = matches!(self.math, TriangularBlurMath::OpaqueFixed16);
        for y in 0..input.height as usize {
            check()?;
            let to =
                ((y + self.padding[1] as usize) * width as usize + self.padding[0] as usize) * 4;
            let from = y * input.width as usize * 4;
            source.rgba[to..to + input.width as usize * 4]
                .copy_from_slice(&input.rgba[from..from + input.width as usize * 4]);
        }
        if opaque {
            for pixel in source.rgba.chunks_exact_mut(4) {
                pixel[3] = 255;
            }
        }
        for (axis, radius) in self.radius.iter().copied().enumerate() {
            if radius == 0 {
                continue;
            }
            let r = i32::from(radius);
            let divisor = u32::from(radius) + 1;
            let weights: Vec<u32> = (-r..=r)
                .map(|offset| {
                    (u32::from(radius) / 2 + (divisor - offset.unsigned_abs()) * 256) / divisor
                })
                .collect();
            let scale = if opaque {
                256 / divisor
            } else {
                65536 / divisor
            };
            let mut output = ImageSurface {
                width,
                height,
                rgba: vec![0; source.rgba.len()],
            };
            for y in 0..height as i32 {
                check()?;
                for x in 0..width as i32 {
                    if x & 255 == 0 {
                        check()?;
                    }
                    let at = |offset: i32| {
                        let sx = if axis == 0 {
                            (x + offset).clamp(0, width as i32 - 1)
                        } else {
                            x
                        };
                        let sy = if axis == 1 {
                            (y + offset).clamp(0, height as i32 - 1)
                        } else {
                            y
                        };
                        (sy as usize * width as usize + sx as usize) * 4
                    };
                    if opaque {
                        // Deliberate repair of CatSystem's 24-bit horizontal
                        // scratch bug: calculate and write every RGB channel.
                        // This initialized output is also the next axis's input;
                        // alpha stays opaque, including the zero-filled padding.
                        let mut sums = [0u32; 3];
                        for (offset, weight) in (-r..=r).zip(&weights) {
                            let p = &source.rgba[at(offset)..][..4];
                            for c in 0..3 {
                                sums[c] += weight * u32::from(p[c]);
                            }
                        }
                        let destination = (y as usize * width as usize + x as usize) * 4;
                        for c in 0..3 {
                            output.rgba[destination + c] =
                                (sums[c].wrapping_mul(scale) >> 16) as u8;
                        }
                        output.rgba[destination + 3] = 255;
                        continue;
                    }
                    let mut mean = [0u32; 3];
                    let mut count = 0;
                    for offset in -r..=r {
                        let p = &source.rgba[at(offset)..][..4];
                        if p[3] != 0 {
                            for c in 0..3 {
                                mean[c] += u32::from(p[c]);
                            }
                            count += 1;
                        }
                    }
                    if count == 0 {
                        continue;
                    }
                    for c in &mut mean {
                        *c /= count;
                    }
                    let mut sums = [0u32; 4];
                    for (offset, weight) in (-r..=r).zip(&weights) {
                        let p = &source.rgba[at(offset)..][..4];
                        let alpha = u32::from(p[3]);
                        for c in 0..3 {
                            let value = match alpha {
                                0 => mean[c],
                                255 => u32::from(p[c]),
                                _ => (mean[c] * (255 - alpha) + alpha * u32::from(p[c])) >> 8,
                            };
                            sums[c] += weight * value;
                        }
                        sums[3] += weight * alpha;
                    }
                    let destination = (y as usize * width as usize + x as usize) * 4;
                    for c in 0..4 {
                        // Native fixed24 rounding deliberately uses a wrapping DWORD product.
                        let value = sums[c].wrapping_mul(scale);
                        output.rgba[destination + c] =
                            ((value >> 24) + u32::from(value & 0x800000 != 0)) as u8;
                    }
                }
            }
            source = output;
        }
        check()?;
        Ok(source)
    }
}

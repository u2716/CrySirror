// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Immutable RGB-mask pair composition and signed packed-word displacement.
//! Input coverage is intentionally independent of RGB math; output retains the
//! first image's alpha. All three inputs are caller-aligned equal-sized planes.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MaskFieldPair {
    /// Per-channel weights: low-word multiply/add, biased unsigned lower clamp,
    /// upper clamp255, summed low-word RGB products, one final /256.
    ColorCoverage {
        multiplier: u16,
        offset: u16,
        nonnegative_bias: u16,
    },
    /// Blue controls horizontal displacement; green controls vertical. Signed
    /// low-word products shift arithmetically before bounded coordinate adds.
    /// RGB contributions are independently floored at /128, then saturated.
    Displacement {
        amplitude: u16,
        second_weight128: u16,
    },
}
impl MaskFieldPair {
    pub fn validate(&self) -> Result<()> {
        if let Self::Displacement {
            amplitude,
            second_weight128,
        } = self
        {
            ensure!(
                *amplitude <= 256 && *second_weight128 <= 128,
                "mask-field displacement weights"
            );
        }
        Ok(())
    }
    pub fn apply(
        &self,
        first: &ImageSurface,
        second: &ImageSurface,
        mask: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        for image in [first, second, mask] {
            image.validate()?;
            ensure!(
                image.width == first.width && image.height == first.height,
                "mask-field plane sizes differ"
            );
        }
        ensure!(
            u64::from(first.width) * u64::from(first.height) <= MAX_EFFECT_PIXELS,
            "mask-field pixel budget"
        );
        check()?;
        let mut out = first.clone();
        for y in 0..first.height {
            check()?;
            for x in 0..first.width {
                let i = ((u64::from(y) * u64::from(first.width) + u64::from(x)) * 4) as usize;
                match *self {
                    Self::ColorCoverage {
                        multiplier,
                        offset,
                        nonnegative_bias,
                    } => {
                        for c in 0..3 {
                            let raw = u16::from(mask.rgba[i + c])
                                .wrapping_mul(multiplier)
                                .wrapping_add(offset)
                                .wrapping_add(nonnegative_bias)
                                .saturating_sub(nonnegative_bias);
                            // ff00ff00 is replicated as ff00 WORDS: despite the
                            // old comment saying256, the active MMX ceiling is255.
                            let a = raw.saturating_add(65280).saturating_sub(65280);
                            out.rgba[i + c] = (u16::from(first.rgba[i + c])
                                .wrapping_mul(256u16.wrapping_sub(a))
                                .wrapping_add(u16::from(second.rgba[i + c]).wrapping_mul(a))
                                >> 8) as u8;
                        }
                    }
                    Self::Displacement {
                        amplitude,
                        second_weight128,
                    } => {
                        let displaced = |position: u32, channel: u8, extent: u32| {
                            let signed =
                                u16::from(channel).wrapping_sub(128).wrapping_mul(amplitude) as i16;
                            let raw = ((signed >> 8) as u16).wrapping_add(position as u16);
                            let lower = raw.wrapping_add(256).saturating_sub(256);
                            let ceiling = u16::MAX - (extent as u16 - 1);
                            u32::from(lower.saturating_add(ceiling).saturating_sub(ceiling))
                        };
                        let sx = displaced(x, mask.rgba[i + 2], first.width);
                        let sy = displaced(y, mask.rgba[i + 1], first.height);
                        let si =
                            ((u64::from(sy) * u64::from(first.width) + u64::from(sx)) * 4) as usize;
                        for c in 0..3 {
                            let a = u16::from(first.rgba[si + c]) * (128 - second_weight128) >> 7;
                            let b = u16::from(second.rgba[si + c]) * second_weight128 >> 7;
                            out.rgba[i + c] = (a + b).min(255) as u8;
                        }
                    }
                }
            }
        }
        check()?;
        Ok(out)
    }
}

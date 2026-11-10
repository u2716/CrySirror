// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Stored-byte tonal balancing. A256-entry table is indexed by original RGB
//! luminance; additive and subtractive bytes saturate separately. Independent
//! of native pictures, destination canvases and mask composition.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TonalBand {
    All,
    Shadows,
    Midtones,
    Highlights,
    ShadowsNarrow,
    MidtonesNarrow,
    HighlightsNarrow,
    ShadowsWide,
    MidtonesWide,
    HighlightsWide,
}
impl TonalBand {
    fn weight(self, value: i32) -> i32 {
        match self {
            Self::All => 255,
            Self::Shadows => 255 - value,
            // The stored-byte band has a discontinuity at128. Retain the
            // declared table rather than substituting a symmetric triangle.
            Self::Midtones => {
                if value < 128 {
                    128 + value
                } else {
                    value
                }
            }
            Self::Highlights => value,
            Self::ShadowsNarrow => {
                if value < 128 {
                    (127 - value) * 2
                } else {
                    0
                }
            }
            Self::MidtonesNarrow => value.min(255 - value) * 2,
            Self::HighlightsNarrow => {
                if value >= 128 {
                    (value - 128) * 2
                } else {
                    0
                }
            }
            Self::ShadowsWide => {
                if value < 85 {
                    255
                } else if value < 255 {
                    (254 - value) * 3 / 2
                } else {
                    0
                }
            }
            Self::MidtonesWide => {
                if (85..=170).contains(&value) {
                    255
                } else {
                    value.min(255 - value) * 3 / 2 + 127
                }
            }
            Self::HighlightsWide => {
                if value < 170 {
                    value * 3 / 2
                } else {
                    255
                }
            }
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TonalBalance {
    pub offsets: [i32; 3],
    pub bands: [TonalBand; 3],
    pub preserve_luminance: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LumaOffsetLookup {
    pub add: Vec<[u8; 3]>,
    pub subtract: Vec<[u8; 3]>,
    pub coefficients: [u16; 3],
    pub fractional_bits: u8,
}
impl TonalBalance {
    /// Q8 band weights, truncation toward zero and optional original f32
    /// luminance compensation. Packed byte spills are defined, including
    /// oversized/negative caller offsets; no uninitialised table entries exist.
    pub fn byte_lookup(&self) -> LumaOffsetLookup {
        let pack = |v: [i32; 3]| {
            let word = v[0].wrapping_shl(16) | v[1].wrapping_shl(8) | v[2];
            [(word >> 16) as u8, (word >> 8) as u8, word as u8]
        };
        let mut add = Vec::with_capacity(256);
        let mut subtract = Vec::with_capacity(256);
        for value in 0..256 {
            let mut offsets: [i32; 3] = std::array::from_fn(|c| {
                self.offsets[c].wrapping_mul(self.bands[c].weight(value)) / 256
            });
            if self.preserve_luminance {
                let values = offsets.map(|v| v as f32);
                let luma = values[0] * 0.299f32 + values[1] * 0.587f32 + values[2] * 0.114f32;
                let factors = [0.299f32, 0.587f32, 0.114f32];
                offsets = std::array::from_fn(|c| {
                    ((values[c] - luma / factors[c] * 0.33333f32) as i32).clamp(-255, 255)
                });
            }
            add.push(pack(offsets.map(|v| v.max(0))));
            subtract.push(pack(offsets.map(|v| v.wrapping_neg().max(0))));
        }
        LumaOffsetLookup {
            add,
            subtract,
            coefficients: [77, 150, 29],
            fractional_bits: 8,
        }
    }
}
impl LumaOffsetLookup {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.add.len() == 256 && self.subtract.len() == 256,
            "luminance offset table length"
        );
        ensure!(
            self.fractional_bits <= 16
                && self.coefficients.iter().map(|v| u32::from(*v)).sum::<u32>()
                    <= 1u32 << self.fractional_bits,
            "luminance offset coefficients"
        );
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
            "luminance offset pixel budget"
        );
        check()?;
        let mut output = input.clone();
        for row in output.rgba.chunks_exact_mut(input.width as usize * 4) {
            check()?;
            for pixel in row.chunks_exact_mut(4) {
                let index = (pixel[..3]
                    .iter()
                    .zip(self.coefficients)
                    .map(|(v, c)| u32::from(*v) * u32::from(c))
                    .sum::<u32>()
                    >> self.fractional_bits) as usize;
                for c in 0..3 {
                    pixel[c] = pixel[c]
                        .saturating_add(self.add[index][c])
                        .saturating_sub(self.subtract[index][c]);
                }
            }
        }
        check()?;
        Ok(output)
    }
}

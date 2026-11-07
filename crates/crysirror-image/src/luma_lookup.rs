//! Immutable256-entry colour maps indexed by stored alpha or integer luminance.
use crate::{effects::MAX_EFFECT_PIXELS, ImageSurface};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LookupIndex {
    Alpha,
    WeightedRgb {
        coefficients: [u16; 3],
        fractional_bits: u8,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LumaLookup {
    pub table: Vec<[u8; 4]>,
    pub index: LookupIndex,
    pub preserve_alpha: bool,
}
impl LumaLookup {
    /// Stable stop ordering, inclusive segments and10-bit truncated phase.
    /// Duplicate stops are overwritten by the later segment's end colour.
    pub fn gradient_byte1024(
        stops: &[(u8, [u8; 4])],
        index: LookupIndex,
        preserve_alpha: bool,
    ) -> Result<Self> {
        ensure!(
            !stops.is_empty() && stops.len() <= 256,
            "colour-map stop count"
        );
        let mut stops = stops.to_vec();
        stops.sort_by_key(|(at, _)| *at);
        let mut table = vec![stops[0].1; 256];
        for pair in stops.windows(2) {
            let ((start, a), (end, b)) = (pair[0], pair[1]);
            let span = i32::from(end) - i32::from(start);
            if span == 0 {
                table[usize::from(start)] = b;
                continue;
            }
            for at in start..=end {
                let phase = (i32::from(at - start) << 10) / span;
                table[usize::from(at)] = std::array::from_fn(|c| {
                    (i32::from(a[c]) + phase * (i32::from(b[c]) - i32::from(a[c])) / 1024) as u8
                });
            }
        }
        let (last, color) = *stops.last().unwrap();
        table[usize::from(last)..].fill(color);
        let value = Self {
            table,
            index,
            preserve_alpha,
        };
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.table.len() == 256,
            "colour-map table requires256 entries"
        );
        if let LookupIndex::WeightedRgb {
            coefficients,
            fractional_bits,
        } = self.index
        {
            ensure!(fractional_bits <= 16, "colour-map luminance shift");
            ensure!(
                coefficients.iter().map(|n| u32::from(*n)).sum::<u32>() <= 1u32 << fractional_bits,
                "colour-map luminance coefficient sum"
            );
        }
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
            "colour-map pixel budget"
        );
        check()?;
        let mut output = input.clone();
        for row in output.rgba.chunks_exact_mut(input.width as usize * 4) {
            check()?;
            for pixel in row.chunks_exact_mut(4) {
                let alpha = pixel[3];
                let at = match self.index {
                    LookupIndex::Alpha => usize::from(alpha),
                    LookupIndex::WeightedRgb {
                        coefficients,
                        fractional_bits,
                    } => {
                        (pixel[..3]
                            .iter()
                            .zip(coefficients)
                            .map(|(n, c)| u32::from(*n) * u32::from(c))
                            .sum::<u32>()
                            >> fractional_bits) as usize
                    }
                };
                pixel.copy_from_slice(&self.table[at]);
                if self.preserve_alpha {
                    pixel[3] = alpha;
                }
            }
        }
        check()?;
        Ok(output)
    }
}

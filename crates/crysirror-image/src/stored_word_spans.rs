// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Ordered stored-byte RGB spans with explicit packed-word arithmetic.
//! One immutable source clone serves every span; all alpha bytes are retained.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum StoredWordMath {
    /// Low-word products/addition wrap before the logical right shift.
    WrappingAffine {
        multiplier: [u16; 3],
        addend: [u16; 3],
        shift: u8,
    },
    /// Arithmetic-shift an ordered signed ramp; skip negative samples, retain
    /// its low word, add with unsigned-word saturation and pack to bytes.
    SignedRampAdd { start: i32, step: i32, shift: u8 },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredWordSpan {
    pub destination: [i32; 3],
    pub math: StoredWordMath,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredWordSpans {
    pub spans: Vec<StoredWordSpan>,
}
impl StoredWordSpans {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.spans.len() <= 131072, "stored word span count");
        let mut samples = 0u64;
        for span in &self.spans {
            ensure!(
                span.destination
                    .iter()
                    .all(|n| n.unsigned_abs() <= 1_000_000)
                    && span.destination[2] >= 0,
                "stored word span coordinates"
            );
            let shift = match &span.math {
                StoredWordMath::WrappingAffine { shift, .. }
                | StoredWordMath::SignedRampAdd { shift, .. } => *shift,
            };
            ensure!(shift <= 15, "stored word shift domain");
            samples += span.destination[2] as u64;
            ensure!(
                samples <= MAX_EFFECT_PIXELS * 4,
                "stored word span sample budget"
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
            "stored word image budget"
        );
        check()?;
        let mut output = input.clone();
        for span in &self.spans {
            check()?;
            let [x, y, length] = span.destination.map(i64::from);
            if y < 0 || y >= i64::from(input.height) {
                continue;
            }
            let first = x.max(0);
            let end = (x + length).min(i64::from(input.width));
            for column in first..end {
                if (column - first) % 4096 == 0 {
                    check()?;
                }
                let at = ((y as usize) * input.width as usize + column as usize) * 4;
                match span.math {
                    StoredWordMath::WrappingAffine {
                        multiplier,
                        addend,
                        shift,
                    } => {
                        for c in 0..3 {
                            output.rgba[at + c] = ((u16::from(output.rgba[at + c])
                                .wrapping_mul(multiplier[c])
                                .wrapping_add(addend[c]))
                                >> shift)
                                .min(255) as u8;
                        }
                    }
                    StoredWordMath::SignedRampAdd { start, step, shift } => {
                        // Clip does not restart the ramp. Per-sample wrapping
                        // is equivalent to the original repeated packed adds.
                        let level = start.wrapping_add(step.wrapping_mul((column - x) as i32));
                        if level < 0 {
                            continue;
                        }
                        let add = (level >> shift) as u16;
                        for c in 0..3 {
                            output.rgba[at + c] =
                                u16::from(output.rgba[at + c]).saturating_add(add).min(255) as u8;
                        }
                    }
                }
            }
        }
        check()?;
        Ok(output)
    }
}

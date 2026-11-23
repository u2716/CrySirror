// Copyright (C)2026 CrySirror contributors. MPL-2.0.
//! Stored-byte axis averages with an explicit asymmetric half-open window.
//! Even windows keep their sample phase instead of becoming odd-radius blurs.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
/// Repeat-edge arithmetic mean of offsets[-before,after), truncated once.
/// Source and output are independent immutable images; every RGBA channel is
/// averaged alike. RGB-only coverage policy belongs to later composition.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoxBlurSpan {
    pub vertical: bool,
    pub before: u16,
    pub after: u16,
}
impl BoxBlurSpan {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.before <= 16384
                && self.after <= 16384
                && u32::from(self.before) + u32::from(self.after) > 0,
            "box span offsets must form a nonempty bounded window"
        );
        Ok(())
    }
    /// Sliding sums perform O(pixels+axis) work, including repeated endpoints
    /// whose multiplicity is counted without traversing off-image coordinates.
    pub fn apply(
        &self,
        input: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        input.validate()?;
        ensure!(
            u64::from(input.width) * u64::from(input.height) <= MAX_EFFECT_PIXELS,
            "box span pixel budget"
        );
        check()?;
        let length = if self.vertical {
            input.height
        } else {
            input.width
        } as usize;
        let lines = if self.vertical {
            input.width
        } else {
            input.height
        } as usize;
        let stride = if self.vertical {
            input.width as usize * 4
        } else {
            4
        };
        let before = usize::from(self.before);
        let after = usize::from(self.after);
        let divisor = (before + after) as u64;
        let mut output = input.clone();
        for line in 0..lines {
            check()?;
            let start = if self.vertical {
                line * 4
            } else {
                line * input.width as usize * 4
            };
            let first = &input.rgba[start..start + 4];
            let last_at = start + (length - 1) * stride;
            let last = &input.rgba[last_at..last_at + 4];
            let mut sums = std::array::from_fn::<_, 4, _>(|c| {
                u64::from(first[c]) * before as u64
                    + u64::from(last[c]) * after.saturating_sub(length) as u64
            });
            for sample in 0..after.min(length) {
                if sample % 4096 == 0 {
                    check()?;
                }
                let at = start + sample * stride;
                for c in 0..4 {
                    sums[c] += u64::from(input.rgba[at + c]);
                }
            }
            for position in 0..length {
                if position % 4096 == 0 {
                    check()?;
                }
                let at = start + position * stride;
                for c in 0..4 {
                    output.rgba[at + c] = (sums[c] / divisor) as u8;
                }
                if position + 1 == length {
                    break;
                }
                let remove = position.saturating_sub(before);
                let add = (position + after).min(length - 1);
                let old = start + remove * stride;
                let new = start + add * stride;
                for c in 0..4 {
                    sums[c] =
                        sums[c] - u64::from(input.rgba[old + c]) + u64::from(input.rgba[new + c]);
                }
            }
        }
        check()?;
        Ok(output)
    }
}

// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Checked immutable scanline translations with explicit cleared pixels.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShiftRows {
    pub shifts: Vec<i32>,
    pub clear: [u8; 4],
    /// Positive shifts first erase the source prefix before the overlapping
    /// copy. This retains defined results of destructive legacy row copying.
    pub erase_positive_source: bool,
}
impl ShiftRows {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.shifts.len() <= 16384 && self.shifts.iter().all(|v| v.unsigned_abs() <= 1_000_000),
            "scanline shift bounds"
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
            self.shifts.len() == input.height as usize
                && u64::from(input.width) * u64::from(input.height) <= MAX_EFFECT_PIXELS,
            "scanline shift image extent"
        );
        check()?;
        let mut out = input.clone();
        let width = i64::from(input.width);
        for (y, shift) in self.shifts.iter().enumerate() {
            check()?;
            let shift = i64::from(*shift);
            if shift == 0 {
                continue;
            }
            let row = y * input.width as usize * 4;
            for pixel in out.rgba[row..row + input.width as usize * 4].chunks_exact_mut(4) {
                pixel.copy_from_slice(&self.clear);
            }
            let sx = if shift > 0 && self.erase_positive_source {
                shift
            } else {
                (-shift).max(0)
            };
            let dx = sx + shift;
            let count = (width - sx).min(width - dx);
            if count <= 0 {
                continue;
            }
            let from = row + sx as usize * 4;
            let to = row + dx as usize * 4;
            let bytes = count as usize * 4;
            out.rgba[to..to + bytes].copy_from_slice(&input.rgba[from..from + bytes]);
        }
        check()?;
        Ok(out)
    }
}

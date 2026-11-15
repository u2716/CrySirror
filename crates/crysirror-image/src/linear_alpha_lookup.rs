// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Alpha from a byte table indexed along a linear image coordinate:
//! `alpha = table[offset + x*step[0] + y*step[1]]`, or its complement.
//! Indices outside the table select `outside`. RGB bytes are unchanged; the
//! caller composes the result (e.g. a captured screen over a live one).
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinearAlphaLookup {
    pub table: Vec<u8>,
    pub offset: i64,
    pub step: [i32; 2],
    #[serde(default)]
    pub outside: u8,
    /// Write `255 - value` instead of `value`.
    #[serde(default)]
    pub complement: bool,
}
impl LinearAlphaLookup {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.table.is_empty() && self.table.len() <= 1 << 20,
            "linear alpha lookup table length"
        );
        ensure!(
            self.offset.unsigned_abs() <= 1 << 24 && self.step.iter().all(|s| s.unsigned_abs() <= 4096),
            "linear alpha lookup coordinates"
        );
        Ok(())
    }
    pub fn apply(&self, input: &ImageSurface, check: &dyn Fn() -> Result<()>) -> Result<ImageSurface> {
        self.validate()?;
        input.validate()?;
        ensure!(
            u64::from(input.width) * u64::from(input.height) <= MAX_EFFECT_PIXELS,
            "linear alpha lookup pixel budget"
        );
        let mut output = input.clone();
        let width = input.width as usize;
        for (y, row) in output.rgba.chunks_exact_mut(width * 4).enumerate() {
            if y % 64 == 0 {
                check()?;
            }
            let base = self.offset + y as i64 * i64::from(self.step[1]);
            for (x, pixel) in row.chunks_exact_mut(4).enumerate() {
                let index = base + x as i64 * i64::from(self.step[0]);
                let value = usize::try_from(index)
                    .ok()
                    .and_then(|i| self.table.get(i).copied())
                    .unwrap_or(self.outside);
                pixel[3] = if self.complement { 255 - value } else { value };
            }
        }
        Ok(output)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagonal_index_and_complement() {
        let input = ImageSurface { width: 3, height: 2, rgba: vec![9; 24] };
        let lookup = LinearAlphaLookup { table: vec![10, 20, 30], offset: 0, step: [1, 1], outside: 0, complement: true };
        let out = lookup.apply(&input, &|| Ok(())).unwrap();
        let alpha: Vec<u8> = out.rgba.chunks(4).map(|p| p[3]).collect();
        assert_eq!(alpha, [245, 235, 225, 235, 225, 255]);
        assert!(out.rgba.chunks(4).all(|p| p[..3] == [9, 9, 9]));
    }
}

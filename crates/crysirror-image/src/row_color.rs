// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Per-row packed-word channel offsets over immutable stored RGB.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS, rgb_lut::RgbLookup};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RowWordAdd {
    pub first_row: u32,
    pub columns: u32,
    pub offsets: Vec<[i32; 3]>,
}
impl RowWordAdd {
    /// Interpolate signed offsets with division toward zero and an exclusive
    /// lower endpoint: row n uses start+(end-start)*n/rows.
    pub fn gradient(
        first_row: u32,
        columns: u32,
        rows: u32,
        start: [i32; 3],
        end: [i32; 3],
    ) -> Result<Self> {
        ensure!(rows <= 16384, "row gradient extent");
        let offsets = (0..rows)
            .map(|i| {
                std::array::from_fn(|c| {
                    start[c].wrapping_add(
                        end[c].wrapping_sub(start[c]).wrapping_mul(i as i32) / (rows as i32),
                    )
                })
            })
            .collect();
        let spec = Self {
            first_row,
            columns,
            offsets,
        };
        spec.validate()?;
        Ok(spec)
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.first_row <= 16384 && self.offsets.len() <= 16384 && self.columns <= 16384,
            "row colour bounds"
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
            u64::from(input.width) * u64::from(input.height) <= MAX_EFFECT_PIXELS
                && self.first_row as u64 + self.offsets.len() as u64 <= u64::from(input.height)
                && self.columns <= input.width,
            "row colour image extent"
        );
        check()?;
        let mut out = input.clone();
        for (row, color) in self.offsets.iter().enumerate() {
            check()?;
            let lookup = RgbLookup::packed_word_add(*color);
            let at = (self.first_row as usize + row) * input.width as usize * 4;
            for pixel in out.rgba[at..at + self.columns as usize * 4].chunks_exact_mut(4) {
                pixel[0] = lookup.red[pixel[0] as usize];
                pixel[1] = lookup.green[pixel[1] as usize];
                pixel[2] = lookup.blue[pixel[2] as usize];
            }
        }
        check()?;
        Ok(out)
    }
}

// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Packed Q16 row repetition with signed-high-word wrapping and explicit
//! stored-byte coverage rules. Native row/colour tables remain caller data.
use crate::ImageSurface;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackedRepeatRow {
    pub source_y: i32,
    pub source_x_q16: u32,
    pub step_q16: u32,
    pub add_rgb: [u8; 3],
    pub subtract_rgb: [u8; 3],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackedRepeatRows {
    pub rows: Vec<PackedRepeatRow>,
    pub masked: bool,
    pub offset_colors: bool,
    /// A stored-byte implementation can process unmasked offsets in pairs,
    /// retaining the destination's odd last pixel rather than inventing a tail.
    pub unmasked_offset_pairs: bool,
    /// Some packed implementations reuse the prior partial blend's bytes as
    /// the subtraction operand for a later opaque sample in the SAME row.
    pub opaque_subtract_prior_blend: bool,
}
impl PackedRepeatRows {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.rows.len() <= 16384, "packed repeat row count");
        ensure!(
            self.rows
                .iter()
                .all(|r| r.source_y.unsigned_abs() <= 1_000_000),
            "packed repeat row source Y"
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
        ensure!(
            u64::from(destination.width) * u64::from(destination.height)
                <= crate::effects::MAX_EFFECT_PIXELS,
            "packed repeat output budget"
        );
        check()?;
        let mut out = destination.clone();
        for (y, row) in self.rows.iter().take(out.height as usize).enumerate() {
            check()?;
            let mut xq = row.source_x_q16;
            let mut prior = row.subtract_rgb;
            let width = if !self.masked && self.offset_colors && self.unmasked_offset_pairs {
                out.width & !1
            } else {
                out.width
            };
            // A zero-pair MMX count used to underflow and loop forever. Its
            // bounded replacement is no writes, including a one-pixel row.
            for x in 0..width {
                if x % 4096 == 0 {
                    check()?;
                }
                let sx = xq >> 16;
                xq = xq.wrapping_add(row.step_q16);
                let next = (xq >> 16) as u16;
                if (next as i16) > ((source.width - 1) as i16) {
                    xq = (xq & 0xffff) | u32::from(next.wrapping_sub(source.width as u16)) << 16;
                }
                // A signed-high-word overflow can leave x beyond the nominal
                // width. The original reads the LINEAR allocation, not modulo
                // x a second time. Former reads outside it are bounded no-write.
                let from = i64::from(row.source_y) * i64::from(source.width) + i64::from(sx);
                if from < 0 || from >= i64::from(source.width) * i64::from(source.height) {
                    continue;
                }
                let from = from as usize * 4;
                let to = (y * out.width as usize + x as usize) * 4;
                let a = if self.masked {
                    source.rgba[from + 3]
                } else {
                    255
                };
                if a == 0 {
                    continue;
                }
                let subtract = if a == 255 && self.opaque_subtract_prior_blend {
                    prior
                } else {
                    row.subtract_rgb
                };
                let mut rgb = [0; 3];
                for c in 0..3 {
                    let s = if self.offset_colors {
                        source.rgba[from + c]
                            .saturating_add(row.add_rgb[c])
                            .saturating_sub(subtract[c])
                    } else {
                        source.rgba[from + c]
                    };
                    rgb[c] = if a == 255 {
                        s
                    } else {
                        // Both packed products are added before the shift.
                        ((u32::from(s) * u32::from(a)
                            + u32::from(out.rgba[to + c]) * (256 - u32::from(a)))
                            >> 8) as u8
                    };
                    out.rgba[to + c] = rgb[c];
                }
                if a != 255 && self.masked && self.offset_colors && self.opaque_subtract_prior_blend
                {
                    prior = rgb;
                }
                // BGRX's high byte was unused; keep engine coverage unchanged.
            }
        }
        check()?;
        Ok(out)
    }
}

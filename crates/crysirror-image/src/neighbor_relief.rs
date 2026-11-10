// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Packed-byte neighbour relief with independently truncated blend products.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NeighborRelief {
    /// Relief contribution in 1/256 units; the other contribution is 256-weight.
    pub weight: u16,
    /// Transform this many pixels of the linear image prefix. Each transformed
    /// pixel reads its original immediate successor, including across rows.
    pub pixels: u32,
    /// After filtering, copy each row's penultimate RGB into its last RGB.
    /// A one-pixel-wide row is left unchanged by this repair.
    pub copy_row_last: bool,
}

impl NeighborRelief {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.weight <= 256, "neighbour relief weight");
        ensure!(u64::from(self.pixels) <= MAX_EFFECT_PIXELS, "neighbour relief prefix budget");
        Ok(())
    }

    /// RGB arithmetic is performed on stored bytes, without colour-space
    /// conversion. Alpha is coverage and remains unchanged. Native BGRX's
    /// unused high byte must not become the shared image's coverage channel.
    pub fn apply(
        &self,
        input: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        input.validate()?;
        let length = u64::from(input.width) * u64::from(input.height);
        ensure!(length <= MAX_EFFECT_PIXELS, "neighbour relief image budget");
        ensure!(u64::from(self.pixels) < length, "neighbour relief successor outside image");
        check()?;
        let mut output = input.clone();
        let weight = i32::from(self.weight);
        for position in 0..self.pixels as usize {
            if position % 4096 == 0 { check()?; }
            let at = position * 4;
            for channel in 0..3 {
                let value = i32::from(input.rgba[at + channel]);
                let next = i32::from(input.rgba[at + 4 + channel]);
                // Signed arithmetic shift is required for negative odd
                // differences. The two products are floored independently.
                let relief = ((value - next) >> 1) + 128;
                output.rgba[at + channel] =
                    ((relief * weight >> 8) + (value * (256 - weight) >> 8)) as u8;
            }
        }
        if self.copy_row_last && input.width >= 2 {
            let pitch = input.width as usize * 4;
            for row in 0..input.height as usize {
                if row % 256 == 0 { check()?; }
                let last = row * pitch + pitch - 4;
                for channel in 0..3 {
                    output.rgba[last + channel] = output.rgba[last - 4 + channel];
                }
            }
        }
        check()?;
        Ok(output)
    }
}

// Copyright (C)2026 CrySirror contributors. MPL-2.0.
//! Piecewise-constant grid sampling of stored RGBA. Each output cell selects
//! its upper-left source sample; no averaging or alpha conversion occurs.
use crate::{effects::MAX_EFFECT_PIXELS, ImageSurface};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NearestCellSample {
    pub cell: [u16; 2],
    /// Grid boundaries are origin + k*cell for every signed integer k.
    pub origin: [i32; 2],
}
impl NearestCellSample {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.cell.iter().all(|v| (1..=16384).contains(v)),
            "nearest-cell size budget"
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
            "nearest-cell pixel budget"
        );
        check()?;
        let anchor = |position: u32, axis: usize, length: u32| -> usize {
            let origin = i64::from(self.origin[axis]);
            let cell = i64::from(self.cell[axis]);
            (origin + (i64::from(position) - origin).div_euclid(cell) * cell)
                .clamp(0, i64::from(length) - 1) as usize
        };
        let mut columns = Vec::with_capacity(input.width as usize);
        for x in 0..input.width {
            if x % 4096 == 0 {
                check()?;
            }
            columns.push(anchor(x, 0, input.width) * 4);
        }
        let mut output = input.clone();
        let pitch = input.width as usize * 4;
        for y in 0..input.height {
            check()?;
            let source = anchor(y, 1, input.height) * pitch;
            let destination = y as usize * pitch;
            for (x, column) in columns.iter().enumerate() {
                let at = destination + x * 4;
                output.rgba[at..at + 4]
                    .copy_from_slice(&input.rgba[source + column..source + column + 4]);
            }
        }
        check()?;
        Ok(output)
    }
}

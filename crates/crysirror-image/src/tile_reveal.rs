// Copyright (c)2026 CrySirror contributors. MPL-2.0.
//! Explicit per-cell byte thresholds over two immutable images.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

/// Select source RGB where a cell's stored byte is less than `value`.
/// Cell thresholds are caller-owned data, not an implicit RNG or time source.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TileReveal {
    pub cell: [u16; 2],
    pub grid: [u16; 2],
    /// Row-major cells, exactly grid.x*grid.y entries, at most65536bytes.
    pub thresholds: Vec<u8>,
    ///0 retains destination;256 selects source in every admitted cell.
    pub value: u16,
}
impl TileReveal {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.cell.iter().all(|v| (1..=8192).contains(v))
                && self.grid.iter().all(|v| (1..=8192).contains(v)),
            "tile reveal dimensions"
        );
        let n = usize::from(self.grid[0]) * usize::from(self.grid[1]);
        ensure!(
            n <= 65536 && self.thresholds.len() == n && self.value <= 256,
            "tile reveal thresholds/value"
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
            [destination.width, destination.height] == [source.width, source.height]
                && u64::from(source.width) * u64::from(source.height) <= MAX_EFFECT_PIXELS,
            "tile reveal image extent/budget"
        );
        ensure!(
            u32::from(self.grid[0]) * u32::from(self.cell[0]) >= source.width
                && u32::from(self.grid[1]) * u32::from(self.cell[1]) >= source.height,
            "tile reveal grid coverage"
        );
        check()?;
        let mut output = destination.clone();
        for y in 0..source.height {
            check()?;
            let row = (y / u32::from(self.cell[1])) as usize * usize::from(self.grid[0]);
            for x in 0..source.width {
                let cell = row + (x / u32::from(self.cell[0])) as usize;
                if u16::from(self.thresholds[cell]) < self.value {
                    let at = ((u64::from(y) * u64::from(source.width) + u64::from(x)) * 4) as usize;
                    output.rgba[at..at + 3].copy_from_slice(&source.rgba[at..at + 3]);
                }
            }
        }
        check()?;
        Ok(output)
    }
}

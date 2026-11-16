// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Device-free integer sprite sampling for UI/layers. The caller owns time;
//! no floating clock, renderer, resource cache or native script is retained.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpriteSamples {
    pub cells: u32,
    pub ticks_per_cell: u32,
    pub ping_pong: bool,
    pub hold: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpriteSample {
    pub current: u32,
    pub next: u32,
    /// Exact interpolation numerator; clients choose their own byte/float
    /// quantisation. Both endpoints can coincide without losing this fraction.
    pub fraction_ticks: u32,
    pub ticks_per_cell: u32,
}
impl SpriteSamples {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (1..=1_000_000).contains(&self.cells) && (1..=864000).contains(&self.ticks_per_cell),
            "integer sprite sample bounds"
        );
        Ok(())
    }
    pub fn cycle_ticks(&self) -> Result<u64> {
        self.validate()?;
        let cells = u64::from(self.cells);
        let intervals = if self.ping_pong {
            (2 * (cells - 1)).max(1)
        } else {
            cells
        };
        Ok(intervals * u64::from(self.ticks_per_cell))
    }
    pub fn sample(&self, ticks: u64) -> Result<SpriteSample> {
        let cycle = self.cycle_ticks()?;
        let ticks = if self.hold {
            ticks.min(cycle)
        } else {
            ticks % cycle
        };
        let period = u64::from(self.ticks_per_cell);
        let cursor = ticks / period;
        let cells = u64::from(self.cells);
        let map = |index: u64| -> u32 {
            if self.ping_pong {
                if cells == 1 {
                    return 0;
                }
                let length = 2 * (cells - 1);
                let phase = index % length;
                if phase < cells {
                    phase as u32
                } else {
                    (length - phase) as u32
                }
            } else if self.hold {
                index.min(cells - 1) as u32
            } else {
                (index % cells) as u32
            }
        };
        let next = if self.hold && ticks == cycle {
            cursor
        } else {
            cursor + 1
        };
        Ok(SpriteSample {
            current: map(cursor),
            next: map(next),
            fraction_ticks: (ticks % period) as u32,
            ticks_per_cell: self.ticks_per_cell,
        })
    }
}

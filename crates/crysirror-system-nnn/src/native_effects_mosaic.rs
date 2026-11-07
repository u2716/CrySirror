// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Legacy MOZAIC43: Set4AC250, Print4AC1C0, raster policy4ABF20.
//! Partial cells are not painted. Every nonempty Print consumes exactly101
//! MSVC rand draws; shared CyclicCellSample owns all pixels and cancellation.
use anyhow::{ensure, Result};
use crysirror_image::{
    cyclic_cell_sample::CyclicCellSample, effects::ImageEffect, temporal_pattern::PatternRng,
};
#[derive(Clone, Debug)]
pub struct MosaicState {
    pub frames: i32,
    pub count: i32,
    /// Original para0 is independent of the authoritative saved countMax.
    pub parameter_frames: i32,
    pub cell: [i32; 2],
    pub region: [i32; 4],
}
impl MosaicState {
    pub fn parse(values: &[i32], canvas: [u32; 2]) -> Result<Self> {
        let get = |i: usize, default| values.get(i).copied().unwrap_or(default);
        let state = Self {
            frames: get(0, 10000),
            count: 0,
            parameter_frames: get(0, 10000),
            cell: [get(1, 16), get(2, 16)],
            // Native discards ALL four partial rectangle arguments unless
            // at least seven parameters were supplied.
            region: if values.len() >= 7 {
                values[3..7].try_into().unwrap()
            } else {
                [0, 0, canvas[0] as i32, canvas[1] as i32]
            },
        };
        ensure!(
            (-864000..=1000000).contains(&state.frames),
            "NNN MOZAIC frame budget"
        );
        Ok(state)
    }
    pub fn parameters(&self) -> [i32; 7] {
        [
            self.parameter_frames,
            self.cell[0],
            self.cell[1],
            self.region[0],
            self.region[1],
            self.region[2],
            self.region[3],
        ]
    }
    pub fn count_up(&mut self, steps: u32) {
        // CountIsMax returns TRUE only to invalidate damage, not to retire43.
        if self.count < self.frames {
            self.count = self.count.wrapping_add(steps as i32).min(self.frames);
        }
    }
    pub fn effect(&self, canvas: [u32; 2], rng: &mut PatternRng) -> Result<Option<ImageEffect>> {
        let cell = self.cell.map(|n| n.max(1));
        if cell == [1, 1] {
            return Ok(None);
        }
        let start = [
            self.region[0].clamp(0, canvas[0] as i32),
            self.region[1].clamp(0, canvas[1] as i32),
        ];
        let end = [
            self.region[0]
                .wrapping_add(self.region[2])
                .clamp(0, canvas[0] as i32),
            self.region[1]
                .wrapping_add(self.region[3])
                .clamp(0, canvas[1] as i32),
        ];
        if end[0] <= start[0]
            || end[1] <= start[1]
            || (end[0] - start[0]) / cell[0] == 0
            || (end[1] - start[1]) / cell[1] == 0
        {
            return Ok(None);
        }
        // A nonempty cell fits the bounded canvas, so these conversions do
        // not narrow an arbitrary giant native cell into a different effect.
        let mut offsets = Vec::with_capacity(101);
        for _ in 0..101 {
            let random = rng.next_draw()? % 16384;
            offsets.push([
                ((random & 255) % cell[0] as u32) as u16,
                ((random >> 8) % cell[1] as u32) as u16,
            ]);
        }
        Ok(Some(ImageEffect::CyclicCellSample {
            spec: CyclicCellSample {
                region: [
                    start[0] as u32,
                    start[1] as u32,
                    (end[0] - start[0]) as u32,
                    (end[1] - start[1]) as u32,
                ],
                cell: [cell[0].try_into()?, cell[1].try_into()?],
                offsets,
            },
        }))
    }
}

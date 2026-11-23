// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Immutable full-cell sampling with a bounded cyclic sample-offset sequence.
//! Partial cells and pixels outside the region preserve the original bytes.
use crate::{effects::MAX_EFFECT_PIXELS, ImageSurface};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CyclicCellSample {
    pub region: [u32; 4],
    pub cell: [u16; 2],
    /// Row-major full cells cycle through these offsets, relative to each cell.
    pub offsets: Vec<[u16; 2]>,
}
impl CyclicCellSample {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.cell.iter().all(|n| (1..=16384).contains(n)),
            "cyclic-cell extents"
        );
        ensure!(
            (1..=4096).contains(&self.offsets.len()),
            "cyclic-cell offset budget"
        );
        ensure!(
            self.region.iter().all(|n| *n <= 16384),
            "cyclic-cell region budget"
        );
        ensure!(
            self.offsets
                .iter()
                .all(|p| p[0] < self.cell[0] && p[1] < self.cell[1]),
            "cyclic-cell sample outside cell"
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
            "cyclic-cell pixel budget"
        );
        let [x, y, w, h] = self.region;
        ensure!(
            u64::from(x) + u64::from(w) <= u64::from(input.width)
                && u64::from(y) + u64::from(h) <= u64::from(input.height),
            "cyclic-cell region outside source"
        );
        check()?;
        let [cw, ch] = self.cell.map(u32::from);
        let mut out = input.clone();
        let mut phase = 0;
        for row in 0..h / ch {
            check()?;
            for column in 0..w / cw {
                let [ox, oy] = self.offsets[phase];
                phase = (phase + 1) % self.offsets.len();
                let left = x + column * cw;
                let top = y + row * ch;
                let sample = ((top + u32::from(oy)) as usize * input.width as usize
                    + (left + u32::from(ox)) as usize)
                    * 4;
                let pixel = &input.rgba[sample..sample + 4];
                for dy in 0..ch {
                    check()?;
                    let start = ((top + dy) as usize * input.width as usize + left as usize) * 4;
                    for p in out.rgba[start..start + cw as usize * 4].chunks_exact_mut(4) {
                        p.copy_from_slice(pixel);
                    }
                }
            }
        }
        Ok(out)
    }
}

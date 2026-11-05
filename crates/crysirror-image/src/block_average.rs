//! Straight-channel rectangular block averaging, including partial edge cells.
//! Alpha is averaged independently; hidden RGB remains part of the average.
use crate::ImageSurface;
use anyhow::{ensure, Result};
impl ImageSurface {
    pub fn block_average(&self, cell: [u16; 2], check: &dyn Fn() -> Result<()>) -> Result<Self> {
        ensure!(
            cell.iter().all(|v| (1..=1024).contains(v)),
            "block average cell must be1..1024"
        );
        ensure!(
            u64::from(self.width) * u64::from(self.height) <= crate::effects::MAX_EFFECT_PIXELS,
            "block average pixel budget"
        );
        ensure!(
            self.rgba.len() == crate::rgba_len(self.width, self.height)?,
            "block average image layout"
        );
        check()?;
        let mut out = self.clone();
        for top in (0..self.height).step_by(usize::from(cell[1])) {
            let bottom = top.saturating_add(u32::from(cell[1])).min(self.height);
            for left in (0..self.width).step_by(usize::from(cell[0])) {
                let right = left.saturating_add(u32::from(cell[0])).min(self.width);
                let mut sums = [0u64; 4];
                for y in top..bottom {
                    check()?;
                    for x in left..right {
                        let at = ((y * self.width + x) * 4) as usize;
                        for c in 0..4 {
                            sums[c] += u64::from(self.rgba[at + c]);
                        }
                    }
                }
                let count = u64::from(right - left) * u64::from(bottom - top);
                let mean = sums.map(|sum| (sum / count) as u8);
                for y in top..bottom {
                    check()?;
                    for x in left..right {
                        let at = ((y * self.width + x) * 4) as usize;
                        out.rgba[at..at + 4].copy_from_slice(&mean);
                    }
                }
            }
        }
        Ok(out)
    }
}

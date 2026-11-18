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
    /// Complete `cell`x`cell` blocks only; partial right/bottom strips are kept.
    pub fn full_block_average(&self, cell: u16, check: &dyn Fn() -> Result<()>) -> Result<Self> {
        ensure!((1..=4096).contains(&cell), "full block average cell must be 1..4096");
        ensure!(
            u64::from(self.width) * u64::from(self.height) <= crate::effects::MAX_EFFECT_PIXELS
                && self.rgba.len() == crate::rgba_len(self.width, self.height)?,
            "full block average image layout/budget"
        );
        let cell = u32::from(cell);
        let mut out = self.clone();
        if cell >= self.width || cell >= self.height {
            return Ok(out);
        }
        let area = u64::from(cell) * u64::from(cell);
        for by in 0..self.height / cell {
            check()?;
            for bx in 0..self.width / cell {
                let (left, top) = (bx * cell, by * cell);
                let mut sums = [0u64; 4];
                for y in top..top + cell {
                    for x in left..left + cell {
                        let at = ((y * self.width + x) * 4) as usize;
                        for c in 0..4 {
                            sums[c] += u64::from(self.rgba[at + c]);
                        }
                    }
                }
                let mean = sums.map(|sum| (sum / area) as u8);
                for y in top..top + cell {
                    for x in left..left + cell {
                        let at = ((y * self.width + x) * 4) as usize;
                        out.rgba[at..at + 4].copy_from_slice(&mean);
                    }
                }
            }
        }
        check()?;
        Ok(out)
    }
}
#[cfg(test)]
mod full_cell_tests {
    use crate::ImageSurface;
    #[test]
    fn partial_strips_keep_source_pixels() {
        let mut rgba = vec![0u8; 5 * 5 * 4];
        for (i, p) in rgba.chunks_exact_mut(4).enumerate() {
            p.copy_from_slice(&[i as u8, 0, 0, 255]);
        }
        let image = ImageSurface { width: 5, height: 5, rgba };
        let out = image.full_block_average(2, &|| Ok(())).unwrap();
        // First block {0,1,5,6} averages to 3; column/row 4 stay untouched.
        assert_eq!(out.rgba[0], 3);
        assert_eq!(out.rgba[4 * 4], 4);
        assert_eq!(out.rgba[(4 * 5) * 4], 20);
        // A cell as large as the image is a no-op, as in the original.
        assert_eq!(image.full_block_average(5, &|| Ok(())).unwrap().rgba, image.rgba);
    }
}

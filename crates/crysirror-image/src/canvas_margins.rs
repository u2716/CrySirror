//! Signed canvas margins without resampling or alpha blending.
use crate::ImageSurface;
use anyhow::{ensure, Result};
pub fn output_size(size: [u32; 2], margins: [i32; 4]) -> Result<[u32; 2]> {
    ensure!(
        margins.iter().all(|v| v.unsigned_abs() <= 8192),
        "canvas margin bounds"
    );
    let width = i64::from(size[0]) + i64::from(margins[0]) + i64::from(margins[2]);
    let height = i64::from(size[1]) + i64::from(margins[1]) + i64::from(margins[3]);
    ensure!(
        (1..=8192).contains(&width) && (1..=8192).contains(&height),
        "canvas margin output extent"
    );
    ensure!(
        width * height <= crate::effects::MAX_EFFECT_PIXELS as i64,
        "canvas margin pixel budget"
    );
    Ok([width as u32, height as u32])
}
impl ImageSurface {
    pub fn canvas_margins(
        &self,
        margins: [i32; 4],
        fill: [u8; 4],
        check: &dyn Fn() -> Result<()>,
    ) -> Result<Self> {
        ensure!(
            self.rgba.len() == crate::rgba_len(self.width, self.height)?,
            "canvas margin source layout"
        );
        let [width, height] = output_size([self.width, self.height], margins)?;
        check()?;
        let mut rgba = vec![0; crate::rgba_len(width, height)?];
        for row in rgba.chunks_exact_mut(width as usize * 4) {
            check()?;
            for pixel in row.chunks_exact_mut(4) {
                pixel.copy_from_slice(&fill);
            }
        }
        let sx = (-i64::from(margins[0])).max(0) as u32;
        let sy = (-i64::from(margins[1])).max(0) as u32;
        let dx = margins[0].max(0) as u32;
        let dy = margins[1].max(0) as u32;
        let copy_width = self.width.saturating_sub(sx).min(width.saturating_sub(dx));
        let copy_height = self
            .height
            .saturating_sub(sy)
            .min(height.saturating_sub(dy));
        if copy_width > 0 {
            for row in 0..copy_height {
                check()?;
                let source = (((row + sy) * self.width + sx) * 4) as usize;
                let target = (((row + dy) * width + dx) * 4) as usize;
                let count = copy_width as usize * 4;
                rgba[target..target + count].copy_from_slice(&self.rgba[source..source + count]);
            }
        }
        Ok(Self {
            width,
            height,
            rgba,
        })
    }
}

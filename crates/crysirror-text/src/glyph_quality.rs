//! Bounded retained glyph coverage profiles, independent of a VM or device.
use crate::{
    TextSystem,
    glyph::{GlyphCell, GlyphRaster},
};
use anyhow::{Result, ensure};
use crysirror_core::image_processing::GlyphCoverageQuality;

impl TextSystem {
    /// Supersampling preserves absolute baseline placement and cell origin;
    /// edge samples outside the high-resolution outline contribute zero.
    pub fn rasterize_glyph_cell_quality(
        &self,
        font: &str,
        request: GlyphRaster,
        horizontal_scale: f32,
        quality: GlyphCoverageQuality,
    ) -> Result<GlyphCell> {
        quality.validate()?;
        self.glyph_metrics(font, request.glyph_id, request.size)?;
        let GlyphCoverageQuality::Supersample { factor } = quality else {
            let mut cell = self.rasterize_glyph_cell_scaled(font, request, horizontal_scale)?;
            if let GlyphCoverageQuality::Binary { threshold } = quality {
                for sample in &mut cell.coverage {
                    *sample = if *sample * 255. >= f32::from(threshold) {
                        1.
                    } else {
                        0.
                    };
                }
            }
            return Ok(cell);
        };
        ensure!(
            self.precomputed_face(font)?.is_none(),
            "bitmap font cannot be supersampled as an outline"
        );
        ensure!(
            request
                .baseline
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 1_000_000.),
            "invalid glyph baseline"
        );
        let translation = request.baseline.map(|v| v.floor() as i32);
        let high = self.rasterize_glyph_cell_scaled_limit(
            font,
            GlyphRaster {
                glyph_id: request.glyph_id,
                size: request.size * f32::from(factor),
                baseline: std::array::from_fn(|i| {
                    (request.baseline[i] - translation[i] as f32) * f32::from(factor)
                }),
            },
            horizontal_scale,
            8192.,
        )?;
        if high.width == 0 || high.height == 0 {
            return Ok(high);
        }
        let f = i32::from(factor);
        let origin = high.origin.map(|v| v.div_euclid(f));
        let end = [
            high.origin[0] + high.width as i32,
            high.origin[1] + high.height as i32,
        ]
        .map(|v| -(-v).div_euclid(f));
        let width = (end[0] - origin[0]) as u32;
        let height = (end[1] - origin[1]) as u32;
        let mut coverage = vec![0.; width as usize * height as usize];
        let weight = 1. / f32::from(factor).powi(2);
        for y in 0..high.height {
            let dy = (high.origin[1] + y as i32).div_euclid(f) - origin[1];
            for x in 0..high.width {
                let dx = (high.origin[0] + x as i32).div_euclid(f) - origin[0];
                coverage[dy as usize * width as usize + dx as usize] +=
                    high.coverage[(y * high.width + x) as usize] * weight;
            }
        }
        Ok(GlyphCell {
            origin: std::array::from_fn(|i| origin[i] + translation[i]),
            width,
            height,
            coverage,
        })
    }
}

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
        let (factor, alpha) = match quality {
            GlyphCoverageQuality::Grayscale => {
                return self.rasterize_glyph_cell_scaled(font, request, horizontal_scale);
            }
            GlyphCoverageQuality::Binary { threshold } => {
                let mut cell = self.rasterize_glyph_cell_scaled(font, request, horizontal_scale)?;
                for sample in &mut cell.coverage {
                    *sample = if *sample * 255. >= f32::from(threshold) {
                        1.
                    } else {
                        0.
                    };
                }
                return Ok(cell);
            }
            GlyphCoverageQuality::Supersample { factor } => (factor, None),
            GlyphCoverageQuality::SupersampleAlpha4 { table } => (4, Some(table)),
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
        if let Some(table) = alpha {
            for sample in &mut coverage {
                let bin = ((*sample * 16.).round() as usize).min(16);
                *sample = f32::from(table[bin]) / 255.;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires CRYSIRROR_TEST_FONT pointing to a local outline font"]
    fn alpha_table_remaps_coverage_without_changing_cell_geometry() -> Result<()> {
        let path = std::env::var_os("CRYSIRROR_TEST_FONT").expect("local outline font");
        let mut fonts = TextSystem::default();
        fonts.add_font("test", std::fs::read(path)?)?;
        let glyph = GlyphRaster {
            glyph_id: fonts.glyph_id("test", 'A')?,
            size: 23.,
            baseline: [-2.75, 31.5],
        };
        let raw = fonts.rasterize_glyph_cell_quality(
            "test",
            glyph,
            1.,
            GlyphCoverageQuality::Supersample { factor: 4 },
        )?;
        let table = std::array::from_fn(|bin| (bin * 15) as u8);
        let mapped = fonts.rasterize_glyph_cell_quality(
            "test",
            glyph,
            1.,
            GlyphCoverageQuality::SupersampleAlpha4 { table },
        )?;
        assert_eq!(
            (mapped.origin, mapped.width, mapped.height),
            (raw.origin, raw.width, raw.height)
        );
        assert!(raw.coverage.iter().any(|alpha| 0. < *alpha && *alpha < 1.));
        for (before, after) in raw.coverage.iter().zip(&mapped.coverage) {
            assert_eq!(
                *after,
                f32::from(table[(*before * 16.).round() as usize]) / 255.
            );
        }
        Ok(())
    }
}

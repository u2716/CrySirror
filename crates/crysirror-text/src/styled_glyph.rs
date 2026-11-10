// Copyright (c)2026 CrySirror contributors. MPL-2.0.
//! Explicit glyph coverage processing, on the same outline/bitmap/rich kernels.
use crate::{
    raster::{dilate_cancellable, GlyphMask},
    TextSystem,
};
use ab_glyph::{point, Font, Glyph, GlyphId, PxScale, ScaleFont};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    image_processing::{GlyphImageOrigin, GlyphRasterOptions, GlyphSizeBasis},
    service::Glyph as Positioned,
};
impl TextSystem {
    pub fn rasterize_glyphs_styled_cancellable(
        &self,
        font: &str,
        size: u16,
        canvas: [u32; 2],
        glyphs: &[Positioned],
        options: &GlyphRasterOptions,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<Vec<u8>> {
        options
            .validate()
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        check()?;
        Self::validate_surface(size, canvas)?;
        if options.supersample == 1
            && options.binary_cutoff.is_none()
            && options.embolden_radius == 0
            && options.coverage_offsets.is_none()
            && options.coverage_lut.is_none()
            && options.size_basis == GlyphSizeBasis::LineHeight
            && options.origin == GlyphImageOrigin::Baseline
            && !options.underline
            && !options.strikeout
        {
            return self.rasterize_glyphs_cancellable(font, size, canvas, glyphs, check);
        }
        ensure!(glyphs.len() <= 65536, "styled glyph count");
        let factor = u32::from(options.supersample);
        let working = [
            canvas[0]
                .checked_mul(factor)
                .context("styled width overflow")?,
            canvas[1]
                .checked_mul(factor)
                .context("styled height overflow")?,
        ];
        let size = size
            .checked_mul(u16::from(options.supersample))
            .context("styled size overflow")?;
        Self::validate_surface(size, working)?;
        let radius = u64::from(options.embolden_radius);
        let expanded = (u64::from(working[0]) + 2 * radius) * (u64::from(working[1]) + 2 * radius);
        ensure!(
            expanded * (radius * 2 + 3) <= 64 * 1024 * 1024,
            "styled coverage work"
        );
        let scaled = glyphs
            .iter()
            .map(|g| {
                Ok(Positioned {
                    glyph_id: g.glyph_id,
                    x_26_6: g
                        .x_26_6
                        .checked_mul(factor as i32)
                        .context("styled glyph x")?,
                    y_26_6: g
                        .y_26_6
                        .checked_mul(factor as i32)
                        .context("styled glyph y")?,
                    advance_26_6: g
                        .advance_26_6
                        .checked_mul(factor as i32)
                        .context("styled advance")?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let mut rgba;
        if let Some(face) = self.precomputed_face(font)? {
            ensure!(
                options.size_basis == GlyphSizeBasis::LineHeight
                    && !options.underline
                    && !options.strikeout,
                "styled bitmap font has no outline design/decorations"
            );
            face.check_size(f32::from(size))?;
            let mut positioned = scaled;
            if options.origin == GlyphImageOrigin::Top {
                for g in &mut positioned {
                    g.y_26_6 = g
                        .y_26_6
                        .checked_add((face.ascent() * 64.).round() as i32)
                        .context("styled bitmap baseline")?;
                }
            }
            rgba = face.paint_cancellable(working, &positioned, check)?;
        } else {
            let face = self.font(font)?;
            let scale = if options.size_basis == GlyphSizeBasis::Em {
                f32::from(size) * face.height_unscaled()
                    / face.units_per_em().context("font units per em")?
            } else {
                f32::from(size)
            };
            ensure!(
                scale.is_finite() && scale > 0. && scale <= 2048.,
                "styled scale"
            );
            let ascent = face.as_scaled(PxScale::from(scale)).ascent();
            let mut placed = Vec::with_capacity(scaled.len());
            for (index, g) in scaled.iter().enumerate() {
                if index % 256 == 0 {
                    check()?;
                }
                ensure!(
                    (g.glyph_id as usize) < face.glyph_count(),
                    "styled glyph id"
                );
                let id = u16::try_from(g.glyph_id).context("styled glyph id representation")?;
                placed.push(Glyph {
                    id: GlyphId(id),
                    scale: PxScale::from(scale),
                    position: point(
                        g.x_26_6 as f32 / 64.,
                        g.y_26_6 as f32 / 64.
                            + if options.origin == GlyphImageOrigin::Top {
                                ascent
                            } else {
                                0.
                            },
                    ),
                });
            }
            rgba = Self::paint_cancellable(&face, working, &placed, check)?;
            if options.underline || options.strikeout {
                let (bytes, index) = self.font_source(font)?;
                let design =
                    rustybuzz::Face::from_slice(&bytes, index).context("styled decoration face")?;
                let units = scale / face.height_unscaled();
                let mut lines = Vec::new();
                if options.underline {
                    lines.push(
                        design
                            .underline_metrics()
                            .context("font has no underline metrics")?,
                    );
                }
                if options.strikeout {
                    lines.push(
                        design
                            .strikeout_metrics()
                            .context("font has no strikeout metrics")?,
                    );
                }
                let mut work = 0u64;
                for (g, placed) in scaled.iter().zip(&placed) {
                    for line in &lines {
                        let first = placed.position.x;
                        let last = first + g.advance_26_6 as f32 / 64.;
                        let y = placed.position.y - f32::from(line.position) * units;
                        let thickness = (f32::from(line.thickness).abs() * units).ceil().max(1.);
                        let x0 = first.min(last).floor().max(0.) as u32;
                        let x1 = first.max(last).ceil().max(0.).min(working[0] as f32) as u32;
                        let y0 = (y - thickness * 0.5).floor().max(0.) as u32;
                        let y1 = (y + thickness * 0.5).ceil().max(0.).min(working[1] as f32) as u32;
                        work = work
                            .checked_add(
                                u64::from(x1.saturating_sub(x0)) * u64::from(y1.saturating_sub(y0)),
                            )
                            .context("glyph decoration overflow")?;
                        ensure!(work <= 64 * 1024 * 1024, "glyph decoration work");
                        for y in y0..y1 {
                            check()?;
                            for x in x0..x1 {
                                rgba[((y * working[0] + x) * 4 + 3) as usize] = 255;
                            }
                        }
                    }
                }
            }
        }
        if let Some(cutoff) = options.binary_cutoff {
            for row in rgba.chunks_exact_mut(working[0] as usize * 4) {
                check()?;
                for p in row.chunks_exact_mut(4) {
                    p[3] = if p[3] >= cutoff { 255 } else { 0 };
                }
            }
        }
        let radius = usize::from(options.embolden_radius);
        let mut mask = GlyphMask {
            x: 0,
            y: 0,
            width: working[0] as usize,
            height: working[1] as usize,
            alpha: rgba.chunks_exact(4).map(|p| p[3]).collect(),
        };
        drop(rgba);
        if radius != 0 {
            mask = dilate_cancellable(&mask, radius, check)?;
        }
        if let Some(offsets) = &options.coverage_offsets {
            mask = crate::glyph_coverage_offsets::stamp(&mask, offsets, check)?;
        }
        let mut output = vec![255u8; canvas[0] as usize * canvas[1] as usize * 4];
        let divisor = factor * factor;
        for y in 0..canvas[1] {
            check()?;
            for x in 0..canvas[0] {
                let mut sum = 0u32;
                for sy in 0..factor {
                    for sx in 0..factor {
                        let mx = (x * factor + sx) as i64 - i64::from(mask.x);
                        let my = (y * factor + sy) as i64 - i64::from(mask.y);
                        if mx >= 0 && my >= 0 && mx < mask.width as i64 && my < mask.height as i64 {
                            sum += u32::from(mask.alpha[my as usize * mask.width + mx as usize]);
                        }
                    }
                }
                let value = if let Some(table) = &options.coverage_lut {
                    table[(sum / 255) as usize]
                } else {
                    ((sum
                        + if options.downsample_nearest {
                            divisor / 2
                        } else {
                            0
                        })
                        / divisor) as u8
                };
                output[((y * canvas[0] + x) * 4 + 3) as usize] = value;
            }
        }
        check()?;
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamic::PreparedFontFace;
    use crysirror_core::state::ClientId;
    use std::sync::Arc;
    pub(crate) fn bitmap() -> Arc<[u8]> {
        let count = 10u32;
        let index = 36u32;
        let records = index + count * 2;
        let pixels = records + count * 20;
        let mut b = b"TVP pre-rendered font\x1a".to_vec();
        b.extend([1, 2]);
        b.extend(count.to_le_bytes());
        b.extend(index.to_le_bytes());
        b.extend(records.to_le_bytes());
        for i in 0..count {
            b.extend((b'A' as u16 + i as u16).to_le_bytes());
        }
        for i in 0..count {
            b.extend((pixels + i * 9).to_le_bytes());
            for v in [3i16, 3, 0, 3, 3, 0, 3, 0] {
                b.extend(v.to_le_bytes());
            }
        }
        let order = [4, 0, 2, 6, 8, 1, 3, 5, 7];
        for n in 0..10 {
            let mut pixels = [0u8; 9];
            for p in &order[..n] {
                pixels[*p] = 64;
            }
            b.extend(pixels);
        }
        b.into()
    }
    fn text() -> TextSystem {
        let text = TextSystem::default();
        let font =
            Arc::new(PreparedFontFace::prepare_precomputed(bitmap(), 48, &|| Ok(())).unwrap());
        text.publish_font(ClientId(1), "__font/1/style", font)
            .unwrap();
        text.for_client(ClientId(1))
    }
    fn glyph(id: u32) -> Positioned {
        Positioned {
            glyph_id: id,
            x_26_6: 0,
            y_26_6: 64,
            advance_26_6: 64,
        }
    }
    #[test]
    fn supersampled_binary_coverage_uses_exact_declared_lookup_and_distinct_box_rounding() {
        let text = text();
        let curve: Arc<[u8]> = Arc::from([0, 28, 56, 113, 141, 169, 198, 226, 254, 255]);
        let options = GlyphRasterOptions {
            supersample: 3,
            binary_cutoff: Some(128),
            coverage_lut: Some(curve.clone()),
            ..Default::default()
        };
        for n in 0..10 {
            let pixels = text
                .rasterize_glyphs_styled_cancellable(
                    "__font/1/style",
                    16,
                    [1, 1],
                    &[glyph(n + 1)],
                    &options,
                    &|| Ok(()),
                )
                .unwrap();
            assert_eq!(pixels, [255, 255, 255, curve[n as usize]]);
        }
        let mut options = options;
        options.coverage_lut = None;
        options.downsample_nearest = false;
        assert_eq!(
            text.rasterize_glyphs_styled_cancellable(
                "__font/1/style",
                16,
                [1, 1],
                &[glyph(4)],
                &options,
                &|| Ok(())
            )
            .unwrap(),
            [255, 255, 255, 85]
        );
        options.downsample_nearest = true;
        assert_eq!(
            text.rasterize_glyphs_styled_cancellable(
                "__font/1/style",
                16,
                [1, 1],
                &[glyph(3)],
                &options,
                &|| Ok(())
            )
            .unwrap(),
            [255, 255, 255, 57]
        );
    }
    #[test]
    fn working_disk_embolden_shares_the_rich_kernel_and_clips_back_to_the_cell() {
        let text = text();
        let options = GlyphRasterOptions {
            supersample: 3,
            binary_cutoff: Some(128),
            embolden_radius: 1,
            coverage_lut: Some(Arc::from([0, 28, 56, 113, 141, 169, 198, 226, 254, 255])),
            ..Default::default()
        };
        assert_eq!(
            text.rasterize_glyphs_styled_cancellable(
                "__font/1/style",
                16,
                [1, 1],
                &[glyph(2)],
                &options,
                &|| Ok(())
            )
            .unwrap(),
            [255, 255, 255, 169],
            "center point must become five working pixels, not a square-nine"
        );
        assert!(text
            .rasterize_glyphs_styled_cancellable(
                "__font/1/style",
                16,
                [1, 1],
                &[glyph(2)],
                &options,
                &|| anyhow::bail!("cancelled")
            )
            .is_err());
    }
    #[test]
    fn bitmap_design_profiles_and_invalid_style_never_silently_fall_back() {
        let text = text();
        for options in [
            GlyphRasterOptions {
                supersample: 5,
                ..Default::default()
            },
            GlyphRasterOptions {
                supersample: 3,
                coverage_lut: Some(Arc::from([0; 10])),
                ..Default::default()
            },
            GlyphRasterOptions {
                embolden_radius: 17,
                ..Default::default()
            },
            GlyphRasterOptions {
                supersample: 3,
                size_basis: GlyphSizeBasis::Em,
                ..Default::default()
            },
            GlyphRasterOptions {
                supersample: 3,
                underline: true,
                ..Default::default()
            },
            GlyphRasterOptions {
                supersample: 3,
                strikeout: true,
                ..Default::default()
            },
        ] {
            assert!(text
                .rasterize_glyphs_styled_cancellable(
                    "__font/1/style",
                    16,
                    [1, 1],
                    &[glyph(2)],
                    &options,
                    &|| Ok(())
                )
                .is_err());
        }
    }
    #[test]
    #[ignore = "reads caller-selected system outline font; no font bytes committed"]
    fn outline_em_top_decorations_use_real_face_metrics_and_default_style_is_unchanged() {
        let bytes =
            std::fs::read(std::env::var("CRYSIRROR_TEST_FONT").expect("system font path")).unwrap();
        let mut text = TextSystem::default();
        text.add_font("system", bytes).unwrap();
        let id = text.glyph_id("system", 'A').unwrap();
        let g = Positioned {
            glyph_id: id,
            x_26_6: 4 * 64,
            y_26_6: 2 * 64,
            advance_26_6: 32 * 64,
        };
        let plain = GlyphRasterOptions {
            size_basis: GlyphSizeBasis::Em,
            origin: GlyphImageOrigin::Top,
            supersample: 3,
            ..Default::default()
        };
        let baseline = text
            .rasterize_glyphs_styled_cancellable("system", 24, [64, 48], &[g], &plain, &|| Ok(()))
            .unwrap();
        let decorated = GlyphRasterOptions {
            underline: true,
            strikeout: true,
            ..plain
        };
        let decorated = text
            .rasterize_glyphs_styled_cancellable("system", 24, [64, 48], &[g], &decorated, &|| {
                Ok(())
            })
            .unwrap();
        let before = baseline
            .chunks_exact(4)
            .map(|p| u64::from(p[3]))
            .sum::<u64>();
        let after = decorated
            .chunks_exact(4)
            .map(|p| u64::from(p[3]))
            .sum::<u64>();
        assert!(before > 0 && after > before);
        assert!(baseline
            .chunks_exact(4)
            .zip(decorated.chunks_exact(4))
            .all(|(a, b)| b[3] >= a[3]));
        let old = text
            .rasterize_glyphs_cancellable("system", 24, [64, 48], &[g], &|| Ok(()))
            .unwrap();
        let default = text
            .rasterize_glyphs_styled_cancellable(
                "system",
                24,
                [64, 48],
                &[g],
                &Default::default(),
                &|| Ok(()),
            )
            .unwrap();
        assert_eq!(old, default);
    }
}

//! Engine-owned integer glyph-record rasterization with explicit font leases.
use ab_glyph::{point, Font, FontArc, PxScale, ScaleFont};
use anyhow::{ensure, Result};
use crysirror_core::image_processing::BitmapTextSpec;
use crysirror_image::ImageSurface as DecodedImage;
fn valid_position(x: f32, y: f32) -> bool {
    x.is_finite() && y.is_finite() && x.abs() <= 1_000_000. && y.abs() <= 1_000_000.
}
fn valid_glyph_bounds(b: ab_glyph::Rect) -> bool {
    valid_position(b.min.x, b.min.y)
        && valid_position(b.max.x, b.max.y)
        && (0.0..=4096.0).contains(&b.width())
        && (0.0..=4096.0).contains(&b.height())
        && b.width() * b.height() <= 4_194_304.
}
fn new_raster(width: u32, height: u32) -> Result<DecodedImage> {
    let len = crysirror_image::rgba_len(width, height)?;
    ensure!(len <= 64 * 1024 * 1024, "glyph record byte budget");
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(len)?;
    rgba.resize(len, 0);
    Ok(DecodedImage {
        width,
        height,
        rgba,
    })
}
fn glyph_cell(
    font: &FontArc,
    ch: char,
    x: f32,
    y: f32,
    size: f32,
    rgb: [u8; 3],
) -> Result<Option<(DecodedImage, i32, i32)>> {
    if !valid_position(x, y) {
        anyhow::bail!("bitmap glyph position");
    }
    let scale = PxScale::from(size);
    let baseline = y + font.as_scaled(scale).ascent();
    let Some(outlined) = font.outline_glyph(
        font.glyph_id(ch)
            .with_scale_and_position(scale, point(x, baseline)),
    ) else {
        return Ok(None);
    };
    let bounds = outlined.px_bounds();
    ensure!(valid_glyph_bounds(bounds), "bitmap glyph outline budget");
    let (left, top) = (bounds.min.x as i32, bounds.min.y as i32);
    let width = (bounds.max.x as i32 - left).max(1) as u32;
    let height = (bounds.max.y as i32 - top).max(1) as u32;
    let mut cell = new_raster(width, height)?;
    outlined.draw(|gx, gy, coverage| {
        if gx < width && gy < height {
            let at = ((gy * width + gx) * 4) as usize;
            let alpha = (coverage.clamp(0.0, 1.0) * 255.0).round() as u8;
            if alpha > cell.rgba[at + 3] {
                cell.rgba[at..at + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], alpha]);
            }
        }
    });
    Ok(Some((cell, left, top)))
}

impl crate::TextSystem {
    pub fn rasterize_bitmap_records(
        &self,
        dst: &mut DecodedImage,
        spec: &BitmapTextSpec,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<()> {
        validate(spec)?;
        ensure!(
            crysirror_image::rgba_len(dst.width, dst.height)? == dst.rgba.len(),
            "bitmap text destination length"
        );
        let font = self.font(&spec.font)?;
        for glyph in &spec.glyphs {
            ensure!(
                glyph.character.is_whitespace() || font.glyph_id(glyph.character).0 != 0,
                "font {} lacks glyph {}",
                spec.font,
                glyph.character
            );
        }
        let size = f32::from(spec.size);
        let size = size.max(1.0);
        let shadow = spec.shadow;
        let mut work = 0u64;
        for glyph in &spec.glyphs {
            check()?;
            let rgb = glyph.color;
            let Some((cell, left, top)) = glyph_cell(
                &font,
                glyph.character,
                glyph.position[0],
                glyph.position[1],
                size,
                rgb,
            )?
            else {
                continue;
            };
            work = work.saturating_add(u64::from(cell.width) * u64::from(cell.height));
            ensure!(work <= 100_000_000, "bitmap text total work budget");
            let Some(shadow) = shadow else {
                crysirror_image::integer_raster::bitmap_blend::blit_mode(
                    dst,
                    i32::from(spec.destination_format),
                    &cell,
                    2,
                    left,
                    top,
                    0,
                    0,
                );
                continue;
            };
            if i32::from(shadow.offset[0]) > 1024 || i32::from(shadow.offset[1]) > 1024 {
                anyhow::bail!("text shadow raster budget exceeded");
            }
            let (dx, dy) = (
                i32::from(shadow.offset[0]).max(0),
                i32::from(shadow.offset[1]).max(0),
            );
            let shadow_rgb = shadow.color;
            let parameter = 256i32.saturating_sub(i32::from(shadow.concentration));
            let (spread_x, spread_y) = if shadow.mode == 1 {
                (dx, dy)
            } else {
                (2 * dx, 2 * dy)
            };
            let width = cell.width + spread_x as u32;
            let height = cell.height + spread_y as u32;
            work = work.saturating_add(
                u64::from(width)
                    * u64::from(height)
                    * if shadow.mode == 2 {
                        (2 * dx + 1) as u64 * (2 * dy + 1) as u64
                    } else {
                        2
                    },
            );
            ensure!(work <= 100_000_000, "bitmap text total shadow work budget");
            if shadow.mode == 2
                && u64::from(width) * u64::from(height) * (2 * dx + 1) as u64 * (2 * dy + 1) as u64
                    > 50_000_000
            {
                anyhow::bail!("text glow work budget exceeded");
            }
            let mut record = new_raster(width, height)?;
            if shadow.mode == 1 {
                let mut tinted = cell.clone();
                for pixel in tinted.rgba.as_chunks_mut::<4>().0 {
                    pixel[..3].copy_from_slice(&shadow_rgb);
                }
                crysirror_image::integer_raster::bitmap_blend::blit_mode(
                    &mut record,
                    2,
                    &tinted,
                    2,
                    dx,
                    dy,
                    1,
                    parameter,
                );
                crysirror_image::integer_raster::bitmap_blend::blit_mode(
                    &mut record,
                    2,
                    &cell,
                    2,
                    0,
                    0,
                    0,
                    0,
                );
            } else {
                let mut glow = record.clone();
                for y in 0..height as i32 {
                    check()?;
                    for x in 0..width as i32 {
                        let mut sum = 0u32;
                        for sy in (y - 2 * dy)..=y {
                            for sx in (x - 2 * dx)..=x {
                                if (0..cell.width as i32).contains(&sx)
                                    && (0..cell.height as i32).contains(&sy)
                                {
                                    sum += u32::from(
                                        cell.rgba[((sy as u32 * cell.width + sx as u32) * 4 + 3)
                                            as usize],
                                    );
                                }
                            }
                        }
                        let at = ((y as u32 * width + x as u32) * 4) as usize;
                        glow.rgba[at..at + 4].copy_from_slice(&[
                            shadow_rgb[0],
                            shadow_rgb[1],
                            shadow_rgb[2],
                            sum.min(255) as u8,
                        ]);
                    }
                }
                crysirror_image::integer_raster::bitmap_blend::blit_mode(
                    &mut record,
                    2,
                    &glow,
                    2,
                    0,
                    0,
                    1,
                    parameter,
                );
                crysirror_image::integer_raster::bitmap_blend::blit_mode(
                    &mut record,
                    2,
                    &cell,
                    2,
                    dx,
                    dy,
                    0,
                    0,
                );
            }
            crysirror_image::integer_raster::bitmap_blend::blit_mode(
                dst,
                i32::from(spec.destination_format),
                &record,
                2,
                left,
                top,
                0,
                0,
            );
        }
        Ok(())
    }
}
pub fn validate(spec: &BitmapTextSpec) -> Result<()> {
    ensure!(
        !spec.font.is_empty() && spec.font.len() <= 256,
        "bitmap text font alias"
    );
    ensure!(
        (1..=512).contains(&spec.size) && matches!(spec.destination_format, 1 | 2),
        "bitmap text size/format"
    );
    ensure!(spec.glyphs.len() <= 65536, "bitmap text glyph count");
    ensure!(
        spec.glyphs
            .iter()
            .all(|g| valid_position(g.position[0], g.position[1])),
        "bitmap text coordinates"
    );
    if let Some(shadow) = spec.shadow {
        ensure!(
            matches!(shadow.mode, 1 | 2)
                && shadow.offset.iter().all(|&n| n <= 1024)
                && shadow.concentration <= 256,
            "bitmap text shadow"
        );
    }
    Ok(())
}

// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Bounded glyph-mask effects, independent of layout, renderer and native VM.
use anyhow::{ensure, Result};

pub struct Coverage {
    pub pixels: Vec<u8>,
    pub size: [usize; 2],
    pub origin: [i32; 2],
}
fn validate(src: &[u8], width: usize, height: usize, radius: u8) -> Result<()> {
    ensure!(radius <= 16, "glyph coverage radius exceeds 16");
    ensure!(
        width <= 8192
            && height <= 8192
            && width.checked_mul(height) == Some(src.len())
            && src.len() <= 4 * 1024 * 1024,
        "glyph coverage dimensions/budget"
    );
    ensure!(
        (width + 2 * usize::from(radius))
            .checked_mul(height + 2 * usize::from(radius))
            .is_some_and(|n| n <= 8 * 1024 * 1024),
        "glyph effect output budget"
    );
    Ok(())
}
/// Circular maximum coverage, sharing the rich-text outline raster kernel.
pub fn disk_outline(src: &[u8], width: usize, height: usize, radius: u8) -> Result<Coverage> {
    validate(src, width, height, radius)?;
    if width == 0 || height == 0 {
        return Ok(Coverage {
            pixels: vec![],
            size: [0, 0],
            origin: [0, 0],
        });
    }
    let result = crate::raster::dilate(
        &crate::raster::GlyphMask {
            x: 0,
            y: 0,
            width,
            height,
            alpha: src.to_vec(),
        },
        usize::from(radius),
    );
    Ok(Coverage {
        pixels: result.alpha,
        size: [result.width, result.height],
        origin: [result.x, result.y],
    })
}
/// Union repeated source-over copies at every (x,y) in 0..=extent. The
/// truncating union a+b-floor(a*b/255) preserves native coverage arithmetic.
/// Policy reference: siglus_rs 5f358ef text_render.rs draw_glyph_bitmap_face;
/// its window/font defaults and pass placement remain in the native adapter.
pub fn square_face(src: &[u8], width: usize, height: usize, extent: u8) -> Result<Coverage> {
    validate(src, width, height, extent)?;
    if width == 0 || height == 0 {
        return Ok(Coverage {
            pixels: vec![],
            size: [0, 0],
            origin: [0, 0],
        });
    }
    let n = usize::from(extent) + 1;
    ensure!(
        src.len()
            .checked_mul(n * n)
            .is_some_and(|work| work <= 64 * 1024 * 1024),
        "glyph face work budget"
    );
    let (w, h) = (width + usize::from(extent), height + usize::from(extent));
    let mut pixels = vec![0u8; w * h];
    for oy in 0..n {
        for ox in 0..n {
            for y in 0..height {
                for x in 0..width {
                    let source = u32::from(src[y * width + x]);
                    let dest = &mut pixels[(y + oy) * w + x + ox];
                    *dest = (source + u32::from(*dest) - source * u32::from(*dest) / 255) as u8;
                }
            }
        }
    }
    Ok(Coverage {
        pixels,
        size: [w, h],
        origin: [0, 0],
    })
}
/// Separable unnormalised box sum: each row, then each column, becomes the
/// sum of its (2*radius+1) neighbours clamped to 255 (zero outside the mask),
/// growing the mask by `radius` on every side. CatSystem2's text-window edge
/// (6A3EF0, radius 1) paints this coverage in the edge colour under the face.
pub fn box_sum(src: &[u8], width: usize, height: usize, radius: u8) -> Result<Coverage> {
    validate(src, width, height, radius)?;
    if width == 0 || height == 0 {
        return Ok(Coverage {
            pixels: vec![],
            size: [0, 0],
            origin: [0, 0],
        });
    }
    let r = usize::from(radius);
    let (w, h) = (width + 2 * r, height + 2 * r);
    let mut padded = vec![0u8; w * h];
    for y in 0..height {
        padded[(y + r) * w + r..(y + r) * w + r + width]
            .copy_from_slice(&src[y * width..(y + 1) * width]);
    }
    let sum = |values: &[u8], at: usize, len: usize| -> u8 {
        let lo = at.saturating_sub(r);
        let hi = (at + r).min(len - 1);
        values[lo..=hi]
            .iter()
            .map(|v| u32::from(*v))
            .sum::<u32>()
            .min(255) as u8
    };
    let mut rows = vec![0u8; w * h];
    for y in 0..h {
        let line = &padded[y * w..(y + 1) * w];
        for x in 0..w {
            rows[y * w + x] = sum(line, x, w);
        }
    }
    let mut pixels = vec![0u8; w * h];
    let mut column = vec![0u8; h];
    for x in 0..w {
        for y in 0..h {
            column[y] = rows[y * w + x];
        }
        for y in 0..h {
            pixels[y * w + x] = sum(&column, y, h);
        }
    }
    Ok(Coverage {
        pixels,
        size: [w, h],
        origin: [-(r as i32); 2],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn outline_and_face_have_distinct_shape_origin_and_fractional_alpha_contracts() {
        let outline = disk_outline(&[128], 1, 1, 1).unwrap();
        assert_eq!(outline.origin, [-1, -1]);
        assert_eq!(outline.pixels, [0, 128, 0, 128, 128, 128, 0, 128, 0]);
        let face = square_face(&[128, 128], 2, 1, 1).unwrap();
        assert_eq!(face.origin, [0, 0]);
        assert_eq!(face.size, [3, 2]);
        assert_eq!(face.pixels, [128, 192, 128, 128, 192, 128]);
        assert!(square_face(&[0; 4], 1, 1, 1).is_err());
        assert!(disk_outline(&[1], 1, 1, 17).is_err());
        assert!(square_face(&[1], 1, 1, 17).is_err());
        assert!(square_face(&[], 0, 0, 2).unwrap().pixels.is_empty());
    }
}

/// Rectangular zero-padded sum, clamped once in the 8-bit coverage domain.
/// Sliding windows preserve the exact sum without work proportional to the
/// rectangle area. Positive saturation between the separable passes is exact.
pub fn box_sum_rect(
    src: &[u8],
    width: usize,
    height: usize,
    radius_x: u8,
    radius_y: u8,
) -> Result<Coverage> {
    ensure!(
        width <= 8192
            && height <= 8192
            && width.checked_mul(height) == Some(src.len())
            && src.len() <= 4 * 1024 * 1024,
        "glyph rectangular coverage dimensions"
    );
    if width == 0 || height == 0 {
        return Ok(Coverage {
            pixels: Vec::new(),
            size: [0; 2],
            origin: [0; 2],
        });
    }
    let (rx, ry) = (usize::from(radius_x), usize::from(radius_y));
    let (w, h) = (width + 2 * rx, height + 2 * ry);
    ensure!(
        w.checked_mul(h).is_some_and(|n| n <= 8 * 1024 * 1024),
        "glyph rectangular coverage output budget"
    );
    let mut rows = vec![0u8; w * h];
    for y in 0..height {
        let row = &src[y * width..(y + 1) * width];
        let mut sum = 0u32;
        for x in 0..w {
            if x < width {
                sum += u32::from(row[x]);
            }
            if let Some(old) = x.checked_sub(2 * rx + 1).filter(|&i| i < width) {
                sum -= u32::from(row[old]);
            }
            rows[(y + ry) * w + x] = sum.min(255) as u8;
        }
    }
    let mut pixels = vec![0u8; w * h];
    for x in 0..w {
        let mut sum = 0u32;
        for y in 0..h {
            let add = y + ry;
            if add < h {
                sum += u32::from(rows[add * w + x]);
            }
            // At y=0 include all available samples in the symmetric window.
            if y == 0 {
                for initial in 0..ry.min(h) {
                    sum += u32::from(rows[initial * w + x]);
                }
            }
            if let Some(old) = y.checked_sub(ry + 1) {
                sum -= u32::from(rows[old * w + x]);
            }
            pixels[y * w + x] = sum.min(255) as u8;
        }
    }
    Ok(Coverage {
        pixels,
        size: [w, h],
        origin: [-(rx as i32), -(ry as i32)],
    })
}

/// Shear the immutable coverage cell first, then apply rectangular coverage.
/// Each scanline shifts by round((height-y)*numerator/denominator), retaining
/// the original baseline/origin; this is independent of Layer transforms.
pub fn oblique_box_sum(
    src: &[u8],
    width: usize,
    height: usize,
    numerator: u8,
    denominator: u8,
    radius_x: u8,
    radius_y: u8,
) -> Result<Coverage> {
    ensure!(
        numerator <= 16 && (1..=16).contains(&denominator),
        "glyph oblique ratio"
    );
    ensure!(
        width <= 8192
            && height <= 8192
            && width.checked_mul(height) == Some(src.len())
            && src.len() <= 4 * 1024 * 1024,
        "glyph oblique dimensions"
    );
    if width == 0 || height == 0 {
        return Ok(Coverage {
            pixels: Vec::new(),
            size: [0; 2],
            origin: [0; 2],
        });
    }
    let shift = |row: usize| {
        ((height - row) * usize::from(numerator) + usize::from(denominator) / 2)
            / usize::from(denominator)
    };
    let w = width
        .checked_add(shift(0))
        .ok_or_else(|| anyhow::anyhow!("glyph oblique extent overflow"))?;
    ensure!(
        w <= 8192 && w.checked_mul(height).is_some_and(|n| n <= 4 * 1024 * 1024),
        "glyph oblique output budget"
    );
    let mut pixels = vec![0; w * height];
    for y in 0..height {
        let at = y * w + shift(y);
        pixels[at..at + width].copy_from_slice(&src[y * width..(y + 1) * width]);
    }
    box_sum_rect(&pixels, w, height, radius_x, radius_y)
}

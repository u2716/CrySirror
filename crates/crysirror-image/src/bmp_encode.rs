// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Uncompressed Windows BMP encoding of straight RGBA surfaces, including the
//! ordered-dither 252-colour (6x7x6) palette form. The byte layout, header
//! fields and dither tables follow krkrz `TVPSaveAsBMP`
//! (`visual/GraphicsLoaderIntf.cpp`) and `TVPInitDitherTable` /
//! `TVPDither32BitTo8Bit_c` (`visual/tvpgl.c`) at commit
//! fd5c4baa6a2ef5978db1bd043634351f48667daf (krkrz, modified BSD licence),
//! reimplemented here; no source is copied.
use anyhow::{ensure, Result};

/// Pixel storage of an encoded BMP.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BmpDepth {
    /// 32bpp BGRA rows, alpha byte kept.
    Bgra32,
    /// 24bpp BGR rows.
    Bgr24,
    /// 8bpp rows indexing the fixed 6x7x6 palette through a 4x4 ordered dither.
    Dither252,
}

const BMP_HEADER_BYTES: u32 = 54;

fn header(size: [u32; 2], depth: u16, offset: u32, length: u32, image_bytes: u32) -> [u8; 54] {
    let [width, height] = size;
    let mut header = [0; 54];
    header[..2].copy_from_slice(b"BM");
    header[2..6].copy_from_slice(&length.to_le_bytes());
    header[10..14].copy_from_slice(&offset.to_le_bytes());
    header[14..18].copy_from_slice(&40u32.to_le_bytes());
    header[18..22].copy_from_slice(&width.to_le_bytes());
    header[22..26].copy_from_slice(&height.to_le_bytes());
    header[26..28].copy_from_slice(&1u16.to_le_bytes());
    header[28..30].copy_from_slice(&depth.to_le_bytes());
    header[34..38].copy_from_slice(&image_bytes.to_le_bytes());
    header
}

const DITHER_4X4: [[u8; 4]; 4] = [[0, 12, 2, 14], [8, 4, 10, 6], [3, 15, 1, 13], [11, 7, 9, 5]];

/// `TVPDitherTable_676[channel][i][j][n]`: per-channel palette-index terms.
fn dither_676() -> &'static [[[[u8; 256]; 4]; 4]; 3] {
    static TABLE: std::sync::OnceLock<Box<[[[[u8; 256]; 4]; 4]; 3]>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let mut t = Box::new([[[[0u8; 256]; 4]; 4]; 3]);
        for j in 0..4 {
            for i in 0..4 {
                let v1 = f64::from(DITHER_4X4[j][i]) / 16.0;
                let v2 = f64::from(DITHER_4X4[(j + 1) % 2][(i + 1) % 2]) / 16.0;
                let v3 = f64::from(DITHER_4X4[j][(i + 1) % 2]) / 16.0;
                for n in 0..256usize {
                    let nt = n as f64 / 255.0;
                    let level = |scale: f64, threshold: f64| {
                        let main = (nt * scale) as i32;
                        let frac = nt * scale - f64::from((nt * scale) as i32);
                        main + i32::from(threshold < frac)
                    };
                    t[2][i][j][n] = (level(5.0, v1) * 42) as u8;
                    t[0][i][j][n] = level(5.0, v2) as u8;
                    t[1][i][j][n] = (level(6.0, v3) * 6) as u8;
                }
            }
        }
        t
    })
}

/// The 256-entry BMP palette (BGRx byte order as stored): entry `c` of the
/// 252 used colours is `(c / 42, c / 6 % 7, c % 6)` scaled to 0..255.
pub fn dither252_palette() -> [[u8; 4]; 256] {
    let mut palette = [[0u8; 4]; 256];
    let mut c = 0;
    for r in 0..6u32 {
        for g in 0..7u32 {
            for b in 0..6u32 {
                palette[c] = [
                    (r * 255 / 5) as u8,
                    (g * 255 / 6) as u8,
                    (b * 255 / 5) as u8,
                    0,
                ];
                c += 1;
            }
        }
    }
    palette
}

/// Ordered-dither one RGBA pixel row into palette indices; `y` selects the
/// dither row and the column phase starts at zero.
pub fn dither252_row(rgba: &[u8], y: u32, out: &mut [u8]) -> Result<()> {
    ensure!(rgba.len() == out.len() * 4, "dither row length");
    let t = dither_676();
    let row = (y & 3) as usize;
    for (x, (pixel, index)) in rgba.chunks_exact(4).zip(out.iter_mut()).enumerate() {
        let col = x & 3;
        // tvpgl.c reads ARGB words: line[x + R] + line[16*256*2 + x + B] +
        // line[16*256 + x + G], i.e. tables 0, 2 and 1 respectively.
        *index = t[0][row][col][usize::from(pixel[0])]
            .wrapping_add(t[2][row][col][usize::from(pixel[2])])
            .wrapping_add(t[1][row][col][usize::from(pixel[1])]);
    }
    Ok(())
}

/// Encode `rgba` (straight, row-major, top-down) as a bottom-up BI_RGB BMP.
pub fn encode_bmp(width: u32, height: u32, rgba: &[u8], depth: BmpDepth) -> Result<Vec<u8>> {
    ensure!(width > 0 && height > 0, "BMP dimensions must be nonzero");
    ensure!(
        u64::from(width) * u64::from(height) <= 64 * 1024 * 1024,
        "BMP encode pixel budget"
    );
    ensure!(
        rgba.len() as u64 == u64::from(width) * u64::from(height) * 4,
        "BMP source layout"
    );
    let pixel_bytes: u32 = match depth {
        BmpDepth::Bgra32 => 4,
        BmpDepth::Bgr24 => 3,
        BmpDepth::Dither252 => 1,
    };
    let pitch = ((width * pixel_bytes - 1) / 4 + 1) * 4;
    let palette_bytes = if depth == BmpDepth::Dither252 {
        1024
    } else {
        0
    };
    let offset = BMP_HEADER_BYTES + palette_bytes;
    let size = offset + pitch * height;
    let mut out = Vec::with_capacity(size as usize);
    out.extend_from_slice(&header(
        [width, height],
        (pixel_bytes * 8) as u16,
        offset,
        size,
        0,
    ));
    if depth == BmpDepth::Dither252 {
        for entry in dither252_palette() {
            out.extend_from_slice(&entry);
        }
    }
    let stride = width as usize * 4;
    let mut line = vec![0u8; pitch as usize];
    for y in (0..height).rev() {
        let src = &rgba[y as usize * stride..(y as usize + 1) * stride];
        match depth {
            BmpDepth::Bgra32 => {
                for (d, s) in line.chunks_exact_mut(4).zip(src.chunks_exact(4)) {
                    d.copy_from_slice(&[s[2], s[1], s[0], s[3]]);
                }
            }
            BmpDepth::Bgr24 => {
                for (d, s) in line.chunks_exact_mut(3).zip(src.chunks_exact(4)) {
                    d.copy_from_slice(&[s[2], s[1], s[0]]);
                }
            }
            BmpDepth::Dither252 => dither252_row(src, y, &mut line[..width as usize])?,
        }
        out.extend_from_slice(&line);
    }
    Ok(out)
}

/// Exact output admission for a BGR24 box-averaged BMP.
pub fn box_average_bmp_len(size: [u32; 2]) -> Result<usize> {
    let [width, height] = size;
    crate::rgba_len(width, height)?;
    let pitch = (3 * width).next_multiple_of(4);
    Ok((BMP_HEADER_BYTES + pitch * height) as usize)
}

/// Encode a bottom-up BGR24 thumbnail directly from a straight RGBA frame.
/// Origins use a truncated 10-bit fixed-point step. The integer part of that
/// step sets the box extent. Average only in-bounds RGB samples, or write
/// black if either extent is zero, and write the BMP image byte length.
pub fn encode_box_average_bmp(
    source: [u32; 2],
    rgba: &[u8],
    size: [u32; 2],
    check: &dyn Fn() -> Result<()>,
) -> Result<Vec<u8>> {
    encode_box_average_bmp_with_rows(source, rgba, size, BoxSampleRows::VerticalRatio, check)
}

/// Fixed-point interval used to advance from one output row to the next.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BoxSampleRows {
    VerticalRatio,
    /// Use the horizontal interval on both axes. The sample box itself
    /// still has the height derived from the vertical ratio.
    HorizontalRatio,
}

pub fn encode_box_average_bmp_with_rows(
    source: [u32; 2],
    rgba: &[u8],
    size: [u32; 2],
    rows: BoxSampleRows,
    check: &dyn Fn() -> Result<()>,
) -> Result<Vec<u8>> {
    check()?;
    ensure!(
        crate::rgba_len(source[0], source[1])? == rgba.len(),
        "thumbnail source layout"
    );
    let length = box_average_bmp_len(size)?;
    let [width, height] = size;
    let pitch = (3 * width).next_multiple_of(4) as usize;
    let mut out = Vec::new();
    out.try_reserve_exact(length)?;
    out.extend_from_slice(&header(
        size,
        24,
        BMP_HEADER_BYTES,
        length as u32,
        (length - BMP_HEADER_BYTES as usize) as u32,
    ));
    out.resize(length, 0);

    let step = std::array::from_fn::<_, 2, _>(|axis| {
        (u64::from(source[axis]) << 10) / u64::from(size[axis])
    });
    let extent = step.map(|value| (value >> 10) as usize);
    let row_step = match rows {
        BoxSampleRows::VerticalRatio => step[1],
        BoxSampleRows::HorizontalRatio => {
            ensure!(!extent.contains(&0), "empty thumbnail averaging box");
            let last = (u64::from(height - 1) * step[0]) >> 10;
            ensure!(last + extent[1] as u64 <= u64::from(source[1]), "thumbnail sampling exceeds frame height");
            step[0]
        }
    };
    if extent.contains(&0) {
        check()?;
        return Ok(out);
    }
    for y in 0..height as usize {
        check()?;
        let top = ((y as u64 * row_step) >> 10) as usize;
        let bottom = (top + extent[1]).min(source[1] as usize);
        let row = BMP_HEADER_BYTES as usize + (height as usize - 1 - y) * pitch;
        for (x, pixel) in out[row..row + width as usize * 3]
            .chunks_exact_mut(3)
            .enumerate()
        {
            let left = ((x as u64 * step[0]) >> 10) as usize;
            let right = (left + extent[0]).min(source[0] as usize);
            let count = right.saturating_sub(left) * bottom.saturating_sub(top);
            if count == 0 {
                continue;
            }
            let mut rgb = [0u64; 3];
            for y in top..bottom {
                check()?;
                let start = 4 * (y * source[0] as usize + left);
                for input in rgba[start..start + (right - left) * 4].chunks_exact(4) {
                    for channel in 0..3 {
                        rgb[channel] += u64::from(input[channel]);
                    }
                }
            }
            if rows == BoxSampleRows::HorizontalRatio {
                rgb = rgb.map(|value| value & u64::from(u32::MAX));
            }
            let [r, g, b] = rgb.map(|value| (value / count as u64) as u8);
            pixel.copy_from_slice(&[b, g, r]);
        }
    }
    check()?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnail_averages_truncated_integer_boxes_and_encodes_native_rows() {
        let mut pixels = Vec::new();
        for y in 0..5 {
            for x in 0..8 {
                pixels.extend_from_slice(&[10 * x + y, 2 * x + 10 * y, x + 3 * y, 15]);
            }
        }
        let bmp = encode_box_average_bmp([8, 5], &pixels, [3, 2], &|| Ok(())).unwrap();
        assert_eq!(bmp.len(), box_average_bmp_len([3, 2]).unwrap());
        assert_eq!(&bmp[10..14], &54u32.to_le_bytes());
        assert_eq!(&bmp[28..30], &24u16.to_le_bytes());
        assert_eq!(&bmp[34..38], &24u32.to_le_bytes());
        assert_eq!(
            &bmp[54..],
            [8, 26, 7, 10, 30, 27, 13, 36, 57, 0, 0, 0, 2, 6, 5, 4, 10, 25, 7, 16, 55, 0, 0, 0]
        );

        let black = encode_box_average_bmp([8, 5], &pixels, [3, 6], &|| Ok(())).unwrap();
        assert!(black[54..].iter().all(|&byte| byte == 0));
        assert!(box_average_bmp_len([0, 2]).is_err());
        assert!(encode_box_average_bmp([8, 5], &pixels[..12], [3, 2], &|| Ok(())).is_err());
        let checks = std::cell::Cell::new(0);
        assert!(encode_box_average_bmp([8, 5], &pixels, [3, 2], &|| {
            checks.set(checks.get() + 1);
            ensure!(checks.get() < 3, "cancelled");
            Ok(())
        })
        .is_err());
    }

    #[test]
    fn horizontal_row_interval_keeps_the_vertical_box_height() {
        let mut pixels = Vec::new();
        for y in 0..8 {
            for _ in 0..4 {
                pixels.extend_from_slice(&[y, 0, 0, 255]);
            }
        }
        let bmp = encode_box_average_bmp_with_rows(
            [4, 8], &pixels, [2, 2], BoxSampleRows::HorizontalRatio, &|| Ok(()),
        ).unwrap();
        // Rows average source y = 2..6 and 0..4 in bottom-up BMP order.
        assert_eq!([bmp[56], bmp[64]], [3, 1]);
        assert!(encode_box_average_bmp_with_rows(
            [8, 4], &pixels, [2, 2], BoxSampleRows::HorizontalRatio, &|| Ok(()),
        ).is_err());
    }

    #[test]
    fn header_rows_and_padding_match_the_native_writer() {
        let rgba = [10, 20, 30, 40, 50, 60, 70, 80];
        let bmp = encode_bmp(1, 2, &rgba, BmpDepth::Bgr24).unwrap();
        assert_eq!(&bmp[0..2], b"BM");
        assert_eq!(u32::from_le_bytes(bmp[2..6].try_into().unwrap()), 54 + 8);
        assert_eq!(u32::from_le_bytes(bmp[10..14].try_into().unwrap()), 54);
        assert_eq!(u16::from_le_bytes(bmp[28..30].try_into().unwrap()), 24);
        // Bottom-up rows, BGR, padded to four bytes.
        assert_eq!(&bmp[54..62], &[70, 60, 50, 0, 30, 20, 10, 0]);
        let bmp = encode_bmp(1, 1, &rgba[..4], BmpDepth::Bgra32).unwrap();
        assert_eq!(&bmp[54..58], &[30, 20, 10, 40]);
        // The decoder reads the same picture back.
        let decoded = image::load_from_memory(&encode_bmp(1, 2, &rgba, BmpDepth::Bgr24).unwrap())
            .unwrap()
            .to_rgb8();
        assert_eq!(decoded.as_raw(), &vec![10, 20, 30, 50, 60, 70]);
    }

    #[test]
    fn dither_palette_maps_primaries_and_extremes_exactly() {
        let palette = dither252_palette();
        assert_eq!(palette[0], [0, 0, 0, 0]);
        assert_eq!(palette[251], [255, 255, 255, 0]);
        assert_eq!(palette[252], [0, 0, 0, 0]);
        let mut index = [0u8; 4];
        dither252_row(
            &[
                255, 255, 255, 255, 0, 0, 0, 255, 255, 0, 0, 255, 0, 0, 255, 255,
            ],
            0,
            &mut index,
        )
        .unwrap();
        // White 5 + 6*6 + 5*42 = 251; black 0; pure R/B land on the major
        // (blue, palette byte 0) and minor (red, byte 2) ramps.
        assert_eq!(index, [251, 0, 5, 210]);
        assert_eq!(palette[usize::from(index[2])][2], 255);
        assert_eq!(palette[usize::from(index[3])][0], 255);
        let bmp = encode_bmp(2, 1, &[255; 8], BmpDepth::Dither252).unwrap();
        assert_eq!(bmp.len(), 14 + 40 + 1024 + 4);
        assert_eq!(&bmp[1078..1082], &[251, 251, 0, 0]);
    }
}

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
use anyhow::{Result, ensure};

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
                palette[c] = [(r * 255 / 5) as u8, (g * 255 / 6) as u8, (b * 255 / 5) as u8, 0];
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
    let palette_bytes = if depth == BmpDepth::Dither252 { 1024 } else { 0 };
    let offset = 14 + 40 + palette_bytes;
    let size = offset + pitch * height;
    let mut out = Vec::with_capacity(size as usize);
    out.extend_from_slice(&0x4d42u16.to_le_bytes());
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&offset.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&height.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&((pixel_bytes * 8) as u16).to_le_bytes());
    out.extend_from_slice(&[0; 24]); // BI_RGB, sizes, resolution, colour counts
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

#[cfg(test)]
mod tests {
    use super::*;

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
        dither252_row(&[255, 255, 255, 255, 0, 0, 0, 255, 255, 0, 0, 255, 0, 0, 255, 255], 0, &mut index).unwrap();
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

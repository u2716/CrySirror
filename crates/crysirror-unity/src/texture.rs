// SPDX-License-Identifier: MPL-2.0
//! Texture2D level-0 decoding to top-down RGBA8. Unity stores rows bottom-up.
use crate::classes::Texture2D;
use anyhow::{bail, ensure, Context, Result};
use crysirror_image::ImageSurface;

pub const ALPHA8: i32 = 1;
pub const ARGB4444: i32 = 2;
pub const RGB24: i32 = 3;
pub const RGBA32: i32 = 4;
pub const ARGB32: i32 = 5;
pub const DXT1: i32 = 10;
pub const DXT5: i32 = 12;

fn dds(width: u32, height: u32, fourcc: &[u8; 4], blocks: &[u8]) -> Vec<u8> {
    let mut h = vec![0u8; 128];
    let mut put = |at: usize, v: u32| h[at..at + 4].copy_from_slice(&v.to_le_bytes());
    put(4, 124);
    put(8, 0x1 | 0x2 | 0x4 | 0x1000 | 0x80000);
    put(12, height);
    put(16, width);
    put(20, blocks.len() as u32);
    put(28, 1);
    put(76, 32);
    put(80, 0x4);
    put(108, 0x1000);
    h[..4].copy_from_slice(b"DDS ");
    h[84..88].copy_from_slice(fourcc);
    h.extend_from_slice(blocks);
    h
}

/// Level 0 as top-down RGBA8.
pub fn decode(t: &Texture2D) -> Result<ImageSurface> {
    ensure!(t.width > 0 && t.height > 0, "empty Unity texture {}", t.name);
    let (w, h) = (t.width as usize, t.height as usize);
    let pixels = w.checked_mul(h).context("Unity texture size")?;
    ensure!(pixels * 4 <= crysirror_image::MAX_DECODED_BYTES, "Unity texture exceeds the decode limit");
    let take = |n: usize| t.data.get(..n).context("truncated Unity texture data");
    let mut rgba = Vec::with_capacity(pixels * 4);
    match t.format {
        ALPHA8 => take(pixels)?.iter().for_each(|a| rgba.extend([255, 255, 255, *a])),
        ARGB4444 => {
            for p in take(pixels * 2)?.chunks_exact(2) {
                let v = u16::from_le_bytes([p[0], p[1]]);
                let n = |s: u16| (((v >> s) & 0xf) as u8) * 17;
                rgba.extend([n(8), n(4), n(0), n(12)]);
            }
        }
        RGB24 => take(pixels * 3)?.chunks_exact(3).for_each(|p| rgba.extend([p[0], p[1], p[2], 255])),
        RGBA32 => rgba.extend_from_slice(take(pixels * 4)?),
        ARGB32 => take(pixels * 4)?.chunks_exact(4).for_each(|p| rgba.extend([p[1], p[2], p[3], p[0]])),
        DXT1 | DXT5 => {
            let block = if t.format == DXT1 { 8 } else { 16 };
            let size = w.div_ceil(4).max(1) * h.div_ceil(4).max(1) * block;
            let fourcc = if t.format == DXT1 { b"DXT1" } else { b"DXT5" };
            let surface = crysirror_image::decode_standard(&dds(t.width, t.height, fourcc, take(size)?))?;
            ensure!(surface.width == t.width && surface.height == t.height, "DXT decode size");
            rgba = surface.rgba;
        }
        other => bail!("unsupported Unity texture format {other} ({})", t.name),
    }
    let row = w * 4;
    let mut flipped = Vec::with_capacity(rgba.len());
    for y in (0..h).rev() {
        flipped.extend_from_slice(&rgba[y * row..(y + 1) * row]);
    }
    Ok(ImageSurface { width: t.width, height: t.height, rgba: flipped })
}

/// Unity 5.6 standalone Texture2D, including its optional sibling .resS stream.
pub fn decode_56(bytes: &[u8], open: impl FnOnce(&str) -> Result<std::sync::Arc<dyn crysirror_core::service::AssetReader>>) -> Result<ImageSurface> {
    use crate::reader::Reader;
    let mut r = Reader::new(bytes);
    let name = r.string()?;
    let (width, height) = (r.u32()?, r.u32()?);
    ensure!(width > 0 && height > 0 && u64::from(width)*u64::from(height) <= 64 << 20, "Unity texture dimensions");
    let _size = r.u32()?;
    let format = r.i32()?;
    let mipmap = r.i32()? > 1;
    r.take(2)?;
    r.align()?;
    let image_count = r.i32()?;
    ensure!(image_count == 1 && r.i32()? == 2, "expected a single 2D image");
    let filter_mode = r.i32()?;
    r.take(8)?;
    let wrap_mode = r.i32()?;
    r.take(8)?;
    let embedded = r.bytes()?;
    let data = if !embedded.is_empty() {
        embedded.to_vec()
    } else {
        let offset = u64::from(r.u32()?);
        let size = r.u32()? as usize;
        let path = r.string()?;
        ensure!(size <= 256 << 20, "Unity texture stream size");
        let reader = open(&path)?;
        ensure!(offset + size as u64 <= reader.len()?, "Unity texture stream outside source");
        let mut out = vec![0; size];
        super::read_exact(reader.as_ref(), offset, &mut out)?;
        out
    };
    decode(&Texture2D { name, width, height, format, mipmap, image_count, filter_mode, wrap_mode, data })
}

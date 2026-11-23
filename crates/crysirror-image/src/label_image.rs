// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Integer labels from source indices, never palette-expanded display colours.
//! PNG grayscale samples expand to8-bit; indexed PNG/BMP preserve each index.
use anyhow::{Context, Result, ensure};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct LabelImage {
    pub size: [u32; 2],
    pub labels: Arc<[u8]>,
}
impl LabelImage {
    pub fn decode(bytes: &[u8], check: &dyn Fn() -> Result<()>) -> Result<Self> {
        check()?;
        ensure!(
            bytes.len() <= crate::MAX_DECODED_BYTES,
            "label image encoded budget"
        );
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            let rows = crate::png_rows::PngRows::decode(bytes, check)?;
            ensure!(
                matches!(rows.color_type, 0 | 3) && rows.bit_depth <= 8,
                "label PNG requires grayscale or indexed samples of at most8 bits"
            );
            let count = extent(rows.size)?;
            let mut labels = vec![0; count];
            let depth = usize::from(rows.bit_depth);
            let max = (1u16 << depth) - 1;
            for y in 0..rows.size[1] as usize {
                check()?;
                for x in 0..rows.size[0] as usize {
                    let bit = x * depth;
                    let value = (u16::from(rows.bytes[y * rows.stride + bit / 8])
                        >> (8 - depth - bit % 8))
                        & max;
                    labels[y * rows.size[0] as usize + x] = if rows.color_type == 0 {
                        (value * 255 / max) as u8
                    } else {
                        value as u8
                    };
                }
            }
            return Ok(Self {
                size: rows.size,
                labels: labels.into(),
            });
        }
        ensure!(
            bytes.starts_with(b"BM"),
            "unsupported integer-label image codec"
        );
        bmp(bytes, check)
    }
}
fn extent(size: [u32; 2]) -> Result<usize> {
    ensure!(
        size.iter().all(|v| (1..=8192).contains(v)),
        "label image dimensions"
    );
    let count = u64::from(size[0]) * u64::from(size[1]);
    ensure!(count <= 16 * 1024 * 1024, "label image pixel budget");
    Ok(count as usize)
}
fn bmp(bytes: &[u8], check: &dyn Fn() -> Result<()>) -> Result<LabelImage> {
    let u16_at = |at: usize| -> Result<u16> {
        Ok(u16::from_le_bytes(
            bytes
                .get(at..at + 2)
                .context("truncated label BMP")?
                .try_into()?,
        ))
    };
    let u32_at = |at: usize| -> Result<u32> {
        Ok(u32::from_le_bytes(
            bytes
                .get(at..at + 4)
                .context("truncated label BMP")?
                .try_into()?,
        ))
    };
    let offset = usize::try_from(u32_at(10)?)?;
    let dib = usize::try_from(u32_at(14)?)?;
    let (width, height, depth, planes, top_down, palette_count, palette_stride) = if dib == 12 {
        (
            u32::from(u16_at(18)?),
            u32::from(u16_at(20)?),
            u16_at(24)?,
            u16_at(22)?,
            false,
            None,
            3usize,
        )
    } else {
        ensure!(dib >= 40 && dib <= 124, "label BMP header profile");
        let w = u32_at(18)? as i32;
        let h = u32_at(22)? as i32;
        ensure!(w > 0 && h != 0, "label BMP signed dimensions");
        ensure!(
            u32_at(30)? == 0,
            "compressed/bitfield label BMP unsupported"
        );
        let count = u32_at(46)?;
        (
            w as u32,
            h.unsigned_abs(),
            u16_at(28)?,
            u16_at(26)?,
            h < 0,
            Some(count),
            4usize,
        )
    };
    ensure!(
        planes == 1 && matches!(depth, 1 | 4 | 8),
        "label BMP requires1/4/8-bit indices"
    );
    let count = extent([width, height])?;
    let palette_count = palette_count.filter(|v| *v != 0).unwrap_or(1u32 << depth) as usize;
    ensure!(palette_count <= 1usize << depth, "label BMP palette count");
    let min_offset = 14usize
        .checked_add(dib)
        .and_then(|n| n.checked_add(palette_count * palette_stride))
        .context("label BMP offset overflow")?;
    ensure!(
        offset >= min_offset && offset <= bytes.len(),
        "label BMP pixel offset"
    );
    let stride = (u64::from(width) * u64::from(depth)).div_ceil(32) * 4;
    let length = stride
        .checked_mul(u64::from(height))
        .context("label BMP pixel extent overflow")?;
    ensure!(
        length <= bytes.len().saturating_sub(offset) as u64,
        "truncated label BMP rows"
    );
    let mut labels = vec![0; count];
    let depth = usize::from(depth);
    for y in 0..height as usize {
        check()?;
        let sy = if top_down { y } else { height as usize - 1 - y };
        for x in 0..width as usize {
            let bit = x * depth;
            labels[y * width as usize + x] = (bytes[offset + sy * stride as usize + bit / 8]
                >> (8 - depth - bit % 8))
                & ((1u16 << depth) - 1) as u8;
        }
    }
    Ok(LabelImage {
        size: [width, height],
        labels: labels.into(),
    })
}

impl LabelImage {
    /// Expand stored indices through a caller-owned RGBA palette.
    pub fn expand_rgba(&self, palette: &[[u8; 4]; 256]) -> Result<crate::ImageSurface> {
        let [width, height] = self.size;
        let len = crate::rgba_len(width, height)?;
        ensure!(self.labels.len() == len / 4, "indexed image size mismatch");
        let mut rgba = Vec::new();
        rgba.try_reserve_exact(len)?;
        for &index in self.labels.iter() {
            rgba.extend_from_slice(&palette[usize::from(index)]);
        }
        Ok(crate::ImageSurface { width, height, rgba })
    }
}

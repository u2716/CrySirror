// Copyright (C)2020 kichikuou; Rust (C)2026 CrySirror contributors.
// GPL-3.0-or-later. QNT writer reference: complete libsys4
// b14839f23af0963abbdb17f7cac57a36fd8e9bd2 src/qnt.c encode/write section,
// GPL-2.0-or-later, used under GPLv3+. Original qnt.c additionally credits
// Masaki Chikama and Nunuhara Cabbage. No VM, filesystem or renderer is hosted.
//! Bounded QNT1 encoding of independent stored RGBA bytes.
use anyhow::{Context, Result, ensure};
use crysirror_resource::DecodeContext;
use flate2::{Compression, write::ZlibEncoder};
use std::io::Write;
const BYTES: usize = 64 * 1024 * 1024;
/// Explicit encoded plane presence and native header profile. Missing planes
/// are not inferred from normalized RGBA pixels or opacity.
#[derive(Clone, Copy, Debug)]
pub struct QntEncodeOptions {
    /// Version1 uses a52-byte header; version2 uses the native68-byte header.
    pub version: u8,
    /// Signed original resource origin stored in the header.
    pub origin: [i32; 2],
    /// Emit the independently filtered BGR colour plane.
    pub color_plane: bool,
    /// Emit the independently filtered coverage plane.
    pub alpha_plane: bool,
}
impl Default for QntEncodeOptions {
    fn default() -> Self {
        Self {
            version: 1,
            origin: [0, 0],
            color_plane: true,
            alpha_plane: true,
        }
    }
}
/// Encode QNT version1/header52 with zero origin, explicit24-bit colour and
/// an independent alpha plane. Both planes preserve every stored byte,
/// including RGB under zero alpha. Compression bytes are not a native-zlib
/// identity claim. Cancellation/failure returns no partially published file.
pub fn encode_rgba(size: [u32; 2], rgba: &[u8], cx: &DecodeContext) -> Result<Vec<u8>> {
    encode_rgba_with(size, rgba, QntEncodeOptions::default(), cx)
}
/// The same bounded encoder with explicit QNT1/QNT2 header and plane metadata.
/// QNT2 was additionally checked against the complete original EncodeCG QNT
/// interface61D3B10B-659C-4025-A696-8E9AEBEADE3A (read as data, never executed).
pub fn encode_rgba_with(
    size: [u32; 2],
    rgba: &[u8],
    options: QntEncodeOptions,
    cx: &DecodeContext,
) -> Result<Vec<u8>> {
    cx.check_cancelled()?;
    ensure!(
        matches!(options.version, 1 | 2),
        "QNT encode version profile"
    );
    let header = if options.version == 1 {
        52usize
    } else {
        68usize
    };
    let input = crysirror_image::rgba_len(size[0], size[1])?;
    ensure!(rgba.len() == input, "QNT encode RGBA layout");
    let [width, height] = size.map(|v| v as usize);
    let ew = (width + 1) & !1;
    let eh = (height + 1) & !1;
    let padded = ew
        .checked_mul(eh)
        .and_then(|v| v.checked_mul(4))
        .context("QNT encode padded extent")?;
    ensure!(padded <= BYTES, "QNT encode padded byte budget");
    let limit = usize::try_from(cx.limits.max_entry_size.min(BYTES as u64))?;
    let bound = padded
        .checked_add(padded / 1000)
        .and_then(|n| n.checked_add(4096))
        .context("QNT encode compression bound")?;
    let compressed_bound = limit.min(bound);
    let compression_peak = input
        .checked_add(padded)
        .and_then(|n| n.checked_add(compressed_bound))
        .and_then(|n| n.checked_add(128 * 1024))
        .context("QNT encode working overflow")?;
    let publication_peak = compressed_bound
        .checked_mul(2)
        .and_then(|n| n.checked_add(input))
        .context("QNT encode output working overflow")?;
    let peak = compression_peak.max(publication_peak);
    ensure!(
        peak as u64 <= cx.limits.max_resident_bytes,
        "QNT encode configured working budget"
    );
    let mut filtered = vec![0u8; padded];
    for y in 0..height {
        cx.check_cancelled()?;
        filtered[y * ew * 4..y * ew * 4 + width * 4]
            .copy_from_slice(&rgba[y * width * 4..(y + 1) * width * 4]);
    }
    // Reverse traversal keeps upper/left predictors in their original form.
    // Padding is initialized zero and is not included in prediction filtering.
    for y in (1..height).rev() {
        cx.check_cancelled()?;
        for x in (1..width).rev() {
            for channel in 0..4 {
                let at = (y * ew + x) * 4 + channel;
                let predicted =
                    (u16::from(filtered[at - ew * 4]) + u16::from(filtered[at - 4])) / 2;
                filtered[at] = (predicted as u8).wrapping_sub(filtered[at]);
            }
        }
        for channel in 0..4 {
            let at = y * ew * 4 + channel;
            filtered[at] = filtered[at - ew * 4].wrapping_sub(filtered[at]);
        }
    }
    for x in (1..width).rev() {
        for channel in 0..4 {
            let at = x * 4 + channel;
            filtered[at] = filtered[at - 4].wrapping_sub(filtered[at]);
        }
    }
    let mut color = PlaneWriter::new(limit, cx);
    if options.color_plane {
        for channel in [2, 1, 0] {
            for y in (0..eh).step_by(2) {
                cx.check_cancelled()?;
                for x in (0..ew).step_by(2) {
                    for (x, y) in [(x, y), (x, y + 1), (x + 1, y), (x + 1, y + 1)] {
                        color.push(filtered[(y * ew + x) * 4 + channel])?;
                    }
                }
            }
        }
    }
    let color = if options.color_plane {
        color.finish()?
    } else {
        vec![]
    };
    let alpha_limit = limit
        .checked_sub(header + color.len())
        .context("QNT encode total byte budget")?;
    let mut alpha = PlaneWriter::new(alpha_limit, cx);
    if options.alpha_plane {
        for y in 0..eh {
            cx.check_cancelled()?;
            for x in 0..ew {
                alpha.push(filtered[(y * ew + x) * 4 + 3])?;
            }
        }
    }
    let alpha = if options.alpha_plane {
        alpha.finish()?
    } else {
        vec![]
    };
    // Plane prediction storage is no longer needed. Retire it explicitly
    // before allocating the final contiguous output alongside both compressed
    // planes; NLL alone does not specify Vec destruction at its last use.
    drop(filtered);
    cx.check_cancelled()?;
    let count = header
        .checked_add(color.len())
        .and_then(|v| v.checked_add(alpha.len()))
        .context("QNT encode output overflow")?;
    ensure!(count <= limit, "QNT encode output byte budget");
    let mut output = Vec::with_capacity(count);
    output.extend_from_slice(b"QNT\0");
    for value in [
        u32::from(options.version),
        header as u32,
        options.origin[0] as u32,
        options.origin[1] as u32,
        size[0],
        size[1],
        24,
        1,
        u32::try_from(color.len())?,
        u32::try_from(alpha.len())?,
    ] {
        output.extend(value.to_le_bytes());
    }
    output.resize(header, 0);
    output.extend(color);
    output.extend(alpha);
    cx.check_cancelled()?;
    Ok(output)
}
struct PlaneWriter<'a> {
    encoder: ZlibEncoder<Vec<u8>>,
    chunk: Vec<u8>,
    limit: usize,
    cx: &'a DecodeContext,
}
impl<'a> PlaneWriter<'a> {
    fn new(limit: usize, cx: &'a DecodeContext) -> Self {
        Self {
            encoder: ZlibEncoder::new(vec![], Compression::best()),
            chunk: Vec::with_capacity(65536),
            limit,
            cx,
        }
    }
    fn push(&mut self, value: u8) -> Result<()> {
        self.chunk.push(value);
        if self.chunk.len() == 65536 {
            self.flush()?;
        }
        Ok(())
    }
    fn flush(&mut self) -> Result<()> {
        self.cx.check_cancelled()?;
        self.encoder.write_all(&self.chunk)?;
        self.chunk.clear();
        ensure!(
            self.encoder.get_ref().len() <= self.limit,
            "QNT compressed plane byte budget"
        );
        self.cx.check_cancelled()?;
        Ok(())
    }
    fn finish(mut self) -> Result<Vec<u8>> {
        self.flush()?;
        let result = self.encoder.finish()?;
        ensure!(
            result.len() <= self.limit,
            "QNT compressed plane byte budget"
        );
        self.cx.check_cancelled()?;
        Ok(result)
    }
}

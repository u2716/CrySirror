// SPDX-License-Identifier: MPL-2.0
//! Original PNG/JPEG header properties. Alpha-plane presence is independent
//! of decoded opacity; metadata inspection never scans or decodes pixels.
use anyhow::{Result, bail, ensure};
use crysirror_core::image_encoding::ImageEncodingInfo;
fn info(format: &str, size: [u32; 2], bits: Option<u16>, alpha: bool) -> Result<ImageEncodingInfo> {
    crate::rgba_len(size[0], size[1])?;
    Ok(ImageEncodingInfo {
        format: format.into(),
        size,
        source_origin: None,
        color_bits: bits,
        has_color_plane: Some(true),
        has_alpha_plane: Some(alpha),
        color_row_bytes: None,
        alpha_sample_bytes: None,
    })
}
pub(super) fn inspect(path: &str, bytes: &[u8]) -> Result<Option<ImageEncodingInfo>> {
    let extension = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    if extension == "png" && bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        ensure!(
            bytes.len() >= 33 && &bytes[12..16] == b"IHDR" && bytes[8..12] == 13u32.to_be_bytes(),
            "PNG metadata IHDR"
        );
        let width = u32::from_be_bytes(bytes[16..20].try_into()?);
        let height = u32::from_be_bytes(bytes[20..24].try_into()?);
        let depth = bytes[24];
        let color = bytes[25];
        ensure!(
            match color {
                0 => matches!(depth, 1 | 2 | 4 | 8 | 16),
                2 | 4 | 6 => matches!(depth, 8 | 16),
                3 => matches!(depth, 1 | 2 | 4 | 8),
                _ => false,
            },
            "PNG metadata color depth"
        );
        let mut alpha = matches!(color, 4 | 6);
        let mut at = 33usize;
        while at < bytes.len() {
            ensure!(bytes.len() - at >= 12, "PNG metadata chunk header");
            let len = u32::from_be_bytes(bytes[at..at + 4].try_into()?) as usize;
            let end = at
                .checked_add(len)
                .and_then(|n| n.checked_add(12))
                .filter(|n| *n <= bytes.len())
                .ok_or_else(|| anyhow::anyhow!("PNG metadata chunk range"))?;
            let kind = &bytes[at + 4..at + 8];
            if kind == b"tRNS" {
                ensure!(
                    len > 0 && !matches!(color, 4 | 6),
                    "PNG metadata transparency chunk"
                );
                alpha = true;
            }
            if kind == b"IDAT" || kind == b"IEND" {
                break;
            }
            at = end;
        }
        let channels = match color {
            0 | 4 => Some(1),
            2 | 6 => Some(3),
            _ => None,
        };
        return Ok(Some(info(
            "standard/png",
            [width, height],
            channels.map(|n| n * u16::from(depth)),
            alpha,
        )?));
    }
    if matches!(extension.as_str(), "jpg" | "jpeg") && bytes.starts_with(&[0xff, 0xd8]) {
        let mut at = 2usize;
        while at < bytes.len() {
            ensure!(bytes[at] == 0xff, "JPEG metadata marker");
            while at < bytes.len() && bytes[at] == 0xff {
                at += 1;
            }
            ensure!(at < bytes.len(), "JPEG metadata marker range");
            let marker = bytes[at];
            at += 1;
            if matches!(marker, 0xd8 | 0x01 | 0xd0..=0xd7) {
                continue;
            }
            if matches!(marker, 0xda | 0xd9) {
                bail!("JPEG metadata missing frame before scan/end");
            }
            ensure!(bytes.len() - at >= 2, "JPEG metadata segment header");
            let len = u16::from_be_bytes(bytes[at..at + 2].try_into()?) as usize;
            ensure!(
                len >= 2 && len <= bytes.len() - at,
                "JPEG metadata segment range"
            );
            if matches!(marker,0xc0..=0xc3|0xc5..=0xc7|0xc9..=0xcb|0xcd..=0xcf) {
                ensure!(len >= 8, "JPEG metadata frame");
                let depth = bytes[at + 2];
                let height = u16::from_be_bytes(bytes[at + 3..at + 5].try_into()?);
                let width = u16::from_be_bytes(bytes[at + 5..at + 7].try_into()?);
                let channels = bytes[at + 7];
                ensure!(
                    channels > 0 && len == 8 + usize::from(channels) * 3,
                    "JPEG metadata frame components"
                );
                return Ok(Some(info(
                    "standard/jpeg",
                    [u32::from(width), u32::from(height)],
                    Some(u16::from(depth) * u16::from(channels)),
                    false,
                )?));
            }
            at += len;
        }
        bail!("JPEG metadata missing frame");
    }
    Ok(None)
}

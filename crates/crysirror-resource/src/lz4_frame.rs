// SPDX-License-Identifier: MPL-2.0
// Written from the LZ4 Frame Format Description (lz4 `doc/lz4_Frame_format.md`).
//! Whole-file LZ4 frame passthrough (KiriKiri `lzfs` media, others).
//!
//! A payload is decoded only when it starts with one *complete* modern LZ4
//! frame (magic `0x184D2204`, valid descriptor, end mark present) whose
//! checksums and declared size verify; anything else is "not LZ4" and the
//! caller serves the bytes raw. A truncated frame is therefore raw rather
//! than half-decoded.
use std::io::Read;

const LZ4_FRAME_MAGIC: u32 = 0x184D_2204;

/// Decoded bytes of the leading complete LZ4 frame, or `None` (serve raw).
/// `limit` bounds the decoded size; exceeding it is also `None`.
pub fn decode_complete_frame(bytes: &[u8], limit: usize) -> Option<Vec<u8>> {
    let extent = frame_extent(bytes)?;
    let mut decoder = lz4_flex::frame::FrameDecoder::new(&bytes[..extent]);
    let mut out = Vec::new();
    (&mut decoder).take(limit as u64 + 1).read_to_end(&mut out).ok()?;
    (out.len() <= limit).then_some(out)
}

/// Whether `bytes` begins with the LZ4 frame magic.
pub fn has_frame_magic(bytes: &[u8]) -> bool {
    bytes.get(..4).is_some_and(|m| u32::from_le_bytes([m[0], m[1], m[2], m[3]]) == LZ4_FRAME_MAGIC)
}

/// Offset just past the first frame, or `None` without a complete frame.
fn frame_extent(bytes: &[u8]) -> Option<usize> {
    if !has_frame_magic(bytes) {
        return None;
    }
    let (flg, bd) = (*bytes.get(4)?, *bytes.get(5)?);
    // Version 01, no reserved FLG bit, only BD's block-size field.
    if flg >> 6 != 0b01 || flg & 0b0000_0010 != 0 || bd & 0b1000_1111 != 0 {
        return None;
    }
    let block_max = match (bd >> 4) & 0b111 {
        4 => 64 * 1024,
        5 => 256 * 1024,
        6 => 1024 * 1024,
        7 => 4 * 1024 * 1024,
        _ => return None,
    };
    let block_checksums = flg & 0b0001_0000 != 0;
    let content_checksum = flg & 0b0000_0100 != 0;
    let extras = usize::from(flg & 0b0000_1000 != 0) * 8 + usize::from(flg & 0b0000_0001 != 0) * 4;
    let mut at = 6 + extras + 1;
    loop {
        let header = u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?);
        at += 4;
        if header == 0 {
            if content_checksum {
                at += 4;
            }
            return (at <= bytes.len()).then_some(at);
        }
        let size = (header & 0x7FFF_FFFF) as usize;
        if size > block_max {
            return None;
        }
        at = at.checked_add(size + if block_checksums { 4 } else { 0 })?;
        if at > bytes.len() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn frame(data: &[u8]) -> Vec<u8> {
        let mut encoder = lz4_flex::frame::FrameEncoder::new(Vec::new());
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn complete_frames_decode_and_everything_else_is_raw() {
        let data: Vec<u8> = (0..100_000u32).map(|i| (i % 251) as u8).collect();
        let encoded = frame(&data);
        assert_eq!(decode_complete_frame(&encoded, 1 << 20).unwrap(), data);
        assert!(decode_complete_frame(&encoded[..encoded.len() - 2], 1 << 20).is_none());
        assert!(decode_complete_frame(&encoded, 10).is_none());
        assert!(decode_complete_frame(b"plain text", 1 << 20).is_none());
        assert!(decode_complete_frame(&[], 1 << 20).is_none());
    }
}

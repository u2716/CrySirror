// SPDX-License-Identifier: MPL-2.0
//! GPU block-compressed textures (BC1–BC7, including BC6H) decoded to
//! top-down RGBA8 through `texture2ddecoder` (MIT/Apache-2.0). Rows follow
//! the stored order; consumers flip bottom-up sources themselves.
use crate::{ImageSurface, MAX_DECODED_BYTES};
use anyhow::{ensure, Context, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Block {
    Bc1,
    Bc3,
    Bc4,
    Bc5,
    /// Unsigned BC6H, tone-clamped to 8 bits.
    Bc6hUnsigned,
    Bc7,
}

impl Block {
    /// Bytes per 4×4 block.
    pub fn block_bytes(self) -> usize {
        match self {
            Block::Bc1 | Block::Bc4 => 8,
            _ => 16,
        }
    }

    /// Bytes of one `width`×`height` level.
    pub fn level_bytes(self, width: u32, height: u32) -> usize {
        (width as usize).div_ceil(4).max(1) * (height as usize).div_ceil(4).max(1) * self.block_bytes()
    }
}

/// Decode one level.
pub fn decode(kind: Block, width: u32, height: u32, data: &[u8]) -> Result<ImageSurface> {
    ensure!(width > 0 && height > 0, "empty block-compressed texture");
    let pixels = (width as usize).checked_mul(height as usize).context("texture size")?;
    ensure!(pixels * 4 <= MAX_DECODED_BYTES, "block-compressed texture exceeds the decode limit");
    let need = kind.level_bytes(width, height);
    ensure!(data.len() >= need, "truncated {kind:?} data: {} of {need} bytes", data.len());
    let mut out = vec![0u32; pixels];
    let (w, h) = (width as usize, height as usize);
    let data = &data[..need];
    let r = match kind {
        Block::Bc1 => texture2ddecoder::decode_bc1(data, w, h, &mut out),
        Block::Bc3 => texture2ddecoder::decode_bc3(data, w, h, &mut out),
        Block::Bc4 => texture2ddecoder::decode_bc4(data, w, h, &mut out),
        Block::Bc5 => texture2ddecoder::decode_bc5(data, w, h, &mut out),
        Block::Bc6hUnsigned => texture2ddecoder::decode_bc6_unsigned(data, w, h, &mut out),
        Block::Bc7 => texture2ddecoder::decode_bc7(data, w, h, &mut out),
    };
    r.map_err(|e| anyhow::anyhow!("{kind:?} decode: {e}"))?;
    // texture2ddecoder writes BGRA packed in u32 (B in the low byte).
    let mut rgba = Vec::with_capacity(pixels * 4);
    for p in out {
        let [b, g, r, a] = p.to_le_bytes();
        rgba.extend_from_slice(&[r, g, b, a]);
    }
    Ok(ImageSurface { width, height, rgba })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bc1_solid_block_decodes() {
        // color0 = color1 = pure red (RGB565 0xF800), all indices 0.
        let block = [0x00, 0xF8, 0x00, 0xF8, 0, 0, 0, 0];
        let s = decode(Block::Bc1, 4, 4, &block).unwrap();
        assert_eq!(&s.rgba[..4], &[255, 0, 0, 255]);
    }

    #[test]
    fn truncated_data_fails() {
        assert!(decode(Block::Bc7, 8, 8, &[0; 16]).is_err());
    }
}

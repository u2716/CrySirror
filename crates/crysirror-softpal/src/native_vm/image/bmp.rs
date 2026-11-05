//! Windows bitmap input used by PAL's Mask/MaskNN.bmp transitions.

use super::{read_u16, read_u32, DecodedImage};

pub(super) fn decode(bytes: &[u8]) -> anyhow::Result<DecodedImage> {
    let pixel_offset = read_u32(bytes, 10)? as usize;
    let dib_size = read_u32(bytes, 14)? as usize;
    anyhow::ensure!(dib_size >= 40, "unsupported BMP DIB header size {dib_size}");
    let header_end = 14usize
        .checked_add(dib_size)
        .ok_or_else(|| anyhow::anyhow!("BMP header size overflow"))?;
    anyhow::ensure!(
        header_end <= bytes.len() && header_end <= pixel_offset,
        "truncated BMP header"
    );
    let width = read_u32(bytes, 18)? as i32;
    let signed_height = read_u32(bytes, 22)? as i32;
    anyhow::ensure!(
        width > 0 && signed_height != 0 && signed_height != i32::MIN,
        "invalid BMP dimensions"
    );
    anyhow::ensure!(read_u16(bytes, 26)? == 1, "invalid BMP plane count");
    let bpp = read_u16(bytes, 28)? as usize;
    let compression = read_u32(bytes, 30)?;
    anyhow::ensure!(
        compression == 0 && matches!(bpp, 1 | 4 | 8 | 24 | 32),
        "unsupported BMP encoding: {bpp} bpp, compression {compression}"
    );
    let width = width as usize;
    let height = signed_height.unsigned_abs() as usize;
    let stride = width
        .checked_mul(bpp)
        .and_then(|n| n.checked_add(31))
        .map(|n| (n / 32) * 4)
        .ok_or_else(|| anyhow::anyhow!("BMP stride overflow"))?;
    let end = height
        .checked_mul(stride)
        .and_then(|n| pixel_offset.checked_add(n))
        .ok_or_else(|| anyhow::anyhow!("BMP image size overflow"))?;
    anyhow::ensure!(end <= bytes.len(), "truncated BMP pixel array");
    let palette_entries = if bpp <= 8 {
        let used = read_u32(bytes, 46)? as usize;
        if used == 0 {
            1 << bpp
        } else {
            used
        }
    } else {
        0
    };
    anyhow::ensure!(
        palette_entries <= 256 && header_end + palette_entries * 4 <= pixel_offset,
        "truncated BMP palette"
    );
    let size = width
        .checked_mul(height)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| anyhow::anyhow!("BMP output size overflow"))?;
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(size)?;
    rgba.resize(size, 0);
    for y in 0..height {
        let row_y = if signed_height < 0 { y } else { height - 1 - y };
        let row = &bytes[pixel_offset + row_y * stride..][..stride];
        for x in 0..width {
            let pixel = if bpp <= 8 {
                let bit = x * bpp;
                let index = ((row[bit / 8] >> (8 - bpp - bit % 8)) as usize) & ((1 << bpp) - 1);
                anyhow::ensure!(index < palette_entries, "BMP palette index out of range");
                &bytes[header_end + index * 4..][..4]
            } else {
                &row[x * (bpp / 8)..][..bpp / 8]
            };
            // BI_RGB's fourth byte is reserved, including in palette entries.
            rgba[(y * width + x) * 4..][..4].copy_from_slice(&[pixel[2], pixel[1], pixel[0], 255]);
        }
    }
    Ok(DecodedImage {
        width: width as u32,
        height: height as u32,
        cell_width: width as u32,
        cell_height: height as u32,
        offset_x: 0,
        offset_y: 0,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bitmap(height: i32) -> Vec<u8> {
        let mut bytes = vec![0u8; 70];
        bytes[..2].copy_from_slice(b"BM");
        for (offset, value) in [
            (2, 70),
            (10, 62),
            (14, 40),
            (18, 3),
            (22, height as u32),
            (46, 2),
        ] {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes[26] = 1;
        bytes[28] = 8;
        bytes[54..62].copy_from_slice(&[10, 20, 30, 0, 40, 50, 60, 0]);
        bytes[62..70].copy_from_slice(&[0, 1, 0, 99, 1, 0, 1, 99]);
        bytes
    }

    #[test]
    fn palette_padding_orientation_and_reserved_alpha() {
        let bottom_up = decode(&bitmap(2)).unwrap();
        assert_eq!(
            bottom_up.rgba,
            [
                60, 50, 40, 255, 30, 20, 10, 255, 60, 50, 40, 255, 30, 20, 10, 255, 60, 50, 40,
                255, 30, 20, 10, 255
            ]
        );
        let top_down = decode(&bitmap(-2)).unwrap();
        assert_eq!(&top_down.rgba[..12], &bottom_up.rgba[12..]);
        assert_eq!(&top_down.rgba[12..], &bottom_up.rgba[..12]);
    }

    #[test]
    fn rejects_truncated_pixels_and_invalid_palette_index() {
        assert!(decode(&bitmap(2)[..69]).is_err());
        let mut bytes = bitmap(2);
        bytes[62] = 2;
        assert!(decode(&bytes).is_err());
    }
}

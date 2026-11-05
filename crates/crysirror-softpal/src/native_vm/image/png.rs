//! Static PNG resources for the newer PAL image suffix table.
//!
//! Preserve sample values and straight alpha; no host color management, gamma
//! correction or animation playback is introduced. Sixteen-bit channels use
//! their high byte. Native decoder rounding/profile parity remains unverified.

use super::{bounded_rgba_len, DecodedImage, MAX_RASTER_BYTES};
use ::png::{BitDepth, ColorType, Decoder, Limits, Transformations};

pub(super) fn decode(bytes: &[u8]) -> anyhow::Result<DecodedImage> {
    let mut decoder = Decoder::new_with_limits(
        std::io::Cursor::new(bytes),
        Limits {
            bytes: MAX_RASTER_BYTES,
        },
    );
    decoder.set_transformations(Transformations::EXPAND | Transformations::STRIP_16);
    decoder.set_ignore_text_chunk(true);
    decoder.set_ignore_iccp_chunk(true);
    let header = decoder.read_header_info()?;
    let (width, height) = (header.width, header.height);
    let rgba_len = bounded_rgba_len(width, height)?;
    let mut reader = decoder.read_info()?;
    let output_len = reader.output_buffer_size();
    anyhow::ensure!(
        output_len <= rgba_len,
        "PNG transformed output exceeds image size"
    );
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(output_len)?;
    pixels.resize(output_len, 0);
    let frame = reader.next_frame(&mut pixels)?;
    anyhow::ensure!(
        frame.width == width && frame.height == height,
        "PNG default frame does not cover its canvas"
    );
    anyhow::ensure!(
        frame.bit_depth == BitDepth::Eight,
        "PNG output is not 8-bit"
    );
    // next_frame alone can succeed without seeing IEND. Require complete chunk
    // framing/CRCs, including the tail after the image's compressed data.
    reader.finish()?;
    pixels.truncate(frame.buffer_size());
    let samples = match frame.color_type {
        ColorType::Grayscale => 1,
        ColorType::GrayscaleAlpha => 2,
        ColorType::Rgb => 3,
        ColorType::Rgba => 4,
        ColorType::Indexed => anyhow::bail!("PNG palette was not expanded"),
    };
    anyhow::ensure!(
        pixels.len() == rgba_len / 4 * samples,
        "PNG pixel count mismatch"
    );
    let rgba = if frame.color_type == ColorType::Rgba {
        pixels
    } else {
        let mut rgba = Vec::new();
        rgba.try_reserve_exact(rgba_len)?;
        for pixel in pixels.chunks_exact(samples) {
            match frame.color_type {
                ColorType::Grayscale => {
                    rgba.extend_from_slice(&[pixel[0], pixel[0], pixel[0], 255])
                }
                ColorType::GrayscaleAlpha => {
                    rgba.extend_from_slice(&[pixel[0], pixel[0], pixel[0], pixel[1]])
                }
                ColorType::Rgb => rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]),
                _ => unreachable!(),
            }
        }
        rgba
    };
    Ok(DecodedImage {
        width,
        height,
        cell_width: width,
        cell_height: height,
        offset_x: 0,
        offset_y: 0,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode(
        width: u32,
        height: u32,
        color: ColorType,
        depth: BitDepth,
        palette: Option<&[u8]>,
        transparency: Option<&[u8]>,
        pixels: &[u8],
    ) -> Vec<u8> {
        let mut encoded = Vec::new();
        {
            let mut encoder = ::png::Encoder::new(&mut encoded, width, height);
            encoder.set_color(color);
            encoder.set_depth(depth);
            if let Some(palette) = palette {
                encoder.set_palette(palette);
            }
            if let Some(transparency) = transparency {
                encoder.set_trns(transparency);
            }
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(pixels).unwrap();
            writer.finish().unwrap();
        }
        encoded
    }

    #[test]
    fn rgba_preserves_transparent_rgb_and_partial_alpha_in_row_order() {
        let pixels = [
            12, 34, 56, 0, 21, 43, 65, 127, 98, 76, 54, 255, 87, 65, 43, 1,
        ];
        let bytes = encode(2, 2, ColorType::Rgba, BitDepth::Eight, None, None, &pixels);
        let image = super::super::decode_image(&bytes).unwrap();
        assert_eq!(
            (
                image.width,
                image.height,
                image.cell_width,
                image.cell_height
            ),
            (2, 2, 2, 2)
        );
        assert_eq!((image.offset_x, image.offset_y), (0, 0));
        assert_eq!(image.rgba, pixels);
    }

    #[test]
    fn packed_palette_transparency_expands_and_missing_alpha_is_opaque() {
        let bytes = encode(
            3,
            1,
            ColorType::Indexed,
            BitDepth::Two,
            Some(&[255, 0, 0, 0, 255, 0, 0, 0, 255]),
            Some(&[0, 127]),
            &[0b00011000],
        );
        assert_eq!(
            decode(&bytes).unwrap().rgba,
            [255, 0, 0, 0, 0, 255, 0, 127, 0, 0, 255, 255]
        );
    }

    #[test]
    fn low_bit_grayscale_and_transparent_rgb_are_expanded() {
        let gray = encode(
            4,
            1,
            ColorType::Grayscale,
            BitDepth::Two,
            None,
            None,
            &[0b00011011],
        );
        assert_eq!(
            decode(&gray).unwrap().rgba,
            [0, 0, 0, 255, 85, 85, 85, 255, 170, 170, 170, 255, 255, 255, 255, 255]
        );
        let rgb = encode(
            2,
            1,
            ColorType::Rgb,
            BitDepth::Eight,
            None,
            Some(&[0, 10, 0, 20, 0, 30]),
            &[10, 20, 30, 10, 20, 31],
        );
        assert_eq!(decode(&rgb).unwrap().rgba, [10, 20, 30, 0, 10, 20, 31, 255]);
    }

    #[test]
    fn grayscale_alpha_and_sixteen_bit_high_bytes_are_preserved() {
        let gray = encode(
            2,
            1,
            ColorType::GrayscaleAlpha,
            BitDepth::Eight,
            None,
            None,
            &[12, 34, 56, 78],
        );
        assert_eq!(
            decode(&gray).unwrap().rgba,
            [12, 12, 12, 34, 56, 56, 56, 78]
        );
        let rgba = encode(
            1,
            1,
            ColorType::Rgba,
            BitDepth::Sixteen,
            None,
            None,
            &[0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0],
        );
        assert_eq!(decode(&rgba).unwrap().rgba, [0x12, 0x56, 0x9a, 0xde]);
    }

    #[test]
    fn truncated_or_corrupted_png_fails_including_missing_iend() {
        let bytes = encode(
            1,
            1,
            ColorType::Rgba,
            BitDepth::Eight,
            None,
            None,
            &[1, 2, 3, 4],
        );
        for length in [0, 8, 20, bytes.len() / 2, bytes.len() - 1, bytes.len() - 12] {
            assert!(
                decode(&bytes[..length]).is_err(),
                "accepted truncated PNG of {length} bytes"
            );
        }
        let mut corrupt = bytes.clone();
        corrupt[29] ^= 1;
        assert!(decode(&corrupt).is_err());
    }

    #[test]
    fn oversized_header_is_rejected_before_image_buffer_allocation() {
        let mut bytes = Vec::new();
        let encoder = ::png::Encoder::new(&mut bytes, 100_000, 100_000);
        drop(encoder.write_header().unwrap());
        let error = decode(&bytes).unwrap_err().to_string();
        assert!(error.contains("decoded size limit"), "{error}");
        assert!(bounded_rgba_len(u32::MAX, u32::MAX).is_err());
        assert!(bounded_rgba_len(0, 1).is_err());
    }
}

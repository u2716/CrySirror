//! JPEG backgrounds used by the newer PAL resource loader.
//! Samples are converted to opaque RGB without ICC or EXIF transformations.
//! Native CMYK/profile/IDCT rounding parity has not been established.
use super::{bounded_rgba_len, DecodedImage, MAX_RASTER_BYTES};
use jpeg_decoder::{Decoder, PixelFormat};

pub(super) fn decode(bytes: &[u8]) -> anyhow::Result<DecodedImage> {
    let mut decoder = Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_max_decoding_buffer_size(MAX_RASTER_BYTES);
    decoder.read_info()?;
    let info = decoder
        .info()
        .ok_or_else(|| anyhow::anyhow!("JPEG metadata missing"))?;
    let rgba_len = bounded_rgba_len(u32::from(info.width), u32::from(info.height))?;
    anyhow::ensure!(
        info.pixel_format != PixelFormat::L16,
        "16-bit lossless JPEG is not a verified PAL image format"
    );
    let count = rgba_len / 4;
    let pixels = decoder.decode()?;
    anyhow::ensure!(
        pixels.len() == count * info.pixel_format.pixel_bytes(),
        "invalid JPEG pixel count"
    );
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(rgba_len)?;
    match info.pixel_format {
        PixelFormat::L8 => {
            for value in pixels {
                rgba.extend_from_slice(&[value, value, value, 255]);
            }
        }
        PixelFormat::RGB24 => {
            for rgb in pixels.chunks_exact(3) {
                rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
            }
        }
        PixelFormat::CMYK32 => {
            for cmyk in pixels.chunks_exact(4) {
                let black = 255 - u16::from(cmyk[3]);
                let rgb = |component: u8| ((255 - u16::from(component)) * black / 255) as u8;
                rgba.extend_from_slice(&[rgb(cmyk[0]), rgb(cmyk[1]), rgb(cmyk[2]), 255]);
            }
        }
        PixelFormat::L16 => {
            anyhow::bail!("16-bit lossless JPEG is not a verified PAL image format")
        }
    }
    Ok(DecodedImage {
        width: u32::from(info.width),
        height: u32::from(info.height),
        cell_width: u32::from(info.width),
        cell_height: u32::from(info.height),
        offset_x: 0,
        offset_y: 0,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn jpeg_signature_decodes_to_opaque_rgb_in_display_order() {
        let image =
            super::super::decode_image(include_bytes!("../../tests/fixtures/color_blocks.jpg"))
                .unwrap();
        assert_eq!((image.width, image.height), (16, 8));
        assert_eq!((image.cell_width, image.cell_height), (16, 8));
        assert!(image.rgba.chunks_exact(4).all(|p| p[3] == 255));
        let left = &image.rgba[(4 * 16 + 2) * 4..][..4];
        let right = &image.rgba[(4 * 16 + 12) * 4..][..4];
        assert!(left[0] > 240 && left[1] < 10 && left[2] < 10);
        assert!(right[0] < 10 && right[1] > 240 && right[2] < 10);
        assert!(super::decode(&[0xff, 0xd8, 0xff]).is_err());
    }

    #[test]
    fn grayscale_and_cmyk_jpeg_convert_to_opaque_rgba() {
        let gray = super::decode(include_bytes!("../../tests/fixtures/gray_73.jpg")).unwrap();
        assert_eq!((gray.width, gray.height), (8, 8));
        assert!(gray
            .rgba
            .chunks_exact(4)
            .all(|pixel| pixel == [73, 73, 73, 255]));
        let cmyk = super::decode(include_bytes!("../../tests/fixtures/cmyk_blocks.jpg")).unwrap();
        let left = &cmyk.rgba[(4 * 16 + 2) * 4..][..4];
        let right = &cmyk.rgba[(4 * 16 + 12) * 4..][..4];
        assert_eq!(left, [255, 0, 0, 255]);
        assert_eq!(right, [0, 255, 0, 255]);
    }

    #[test]
    fn jpeg_truncation_and_oversized_dimensions_fail_before_pixel_decode() {
        let bytes = include_bytes!("../../tests/fixtures/color_blocks.jpg");
        for length in [0, 3, 20, bytes.len() / 2, bytes.len() - 2] {
            assert!(
                super::decode(&bytes[..length]).is_err(),
                "accepted truncated JPEG of {length} bytes"
            );
        }
        let mut oversized = bytes.to_vec();
        let sof = oversized
            .windows(2)
            .position(|marker| marker == [0xff, 0xc0])
            .unwrap();
        oversized[sof + 5..sof + 9].fill(0xff);
        let error = super::decode(&oversized).unwrap_err().to_string();
        assert!(error.contains("decoded size limit"), "{error}");
    }
}

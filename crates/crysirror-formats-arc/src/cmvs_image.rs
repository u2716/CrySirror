// SPDX-License-Identifier: GPL-3.0-or-later
//! CMVS unnamed memory images at `0x140036700`. File profiles keep their
//! normal decoders. The memory entry point has its own row and alpha rules.

use crate::images_a::{common::C, pb2, pb3};
use anyhow::{bail, ensure, Context, Result};
use crysirror_image::{png_rows::PngRows, DecoderRegistry, ImageSource, ImageSurface};
use crysirror_resource::DecodeContext;

pub(crate) const PROFILE: &str = "purple-software/cmvs-memory-image";

/// Decode a scene-uploadable image using the CMVS unnamed memory-image profile.
/// The returned flag is the image's native alpha declaration.
pub fn decode(
    bytes: &[u8],
    source: &ImageSource<'_>,
    context: &DecodeContext,
) -> Result<(ImageSurface, bool)> {
    context.check_cancelled()?;
    if !bytes.starts_with(b"PB3B") {
        return decode_raster(bytes, &|| Ok(context.check_cancelled()?));
    }
    let mut context = context.clone();
    context.parameters.insert(PROFILE.into(), true.into());
    let mut registry = DecoderRegistry::default();
    crate::register_images(&mut registry)?;
    let image = registry.decode_with(
        "preview.pb3",
        bytes,
        Some(source),
        Some(&context),
        Some("purple-software/pb3"),
    )?;
    context.check_cancelled()?;
    Ok((image, pb3::bit_depth(bytes)? == 32))
}

pub(crate) fn decode_raster(
    bytes: &[u8],
    check: &dyn Fn() -> Result<()>,
) -> Result<(ImageSurface, bool)> {
    ensure!(
        bytes.len() <= crysirror_image::MAX_DECODED_BYTES,
        "CMVS image byte limit"
    );
    let decoded = if bytes.starts_with(b"BM") {
        let mut dimensions = C::at(bytes, 18)?;
        let size = [dimensions.u32()?, dimensions.u32()?];
        dimensions.skip(2)?;
        let pixel_stride = usize::from(dimensions.u16()? >> 3);
        crysirror_image::rgba_len(size[0], size[1])?;
        let stride = (size[0] as usize * pixel_stride).next_multiple_of(4);
        let length = stride
            .checked_mul(size[1] as usize)
            .context("CMVS BMP rows overflow")?;
        rows(
            C::at(bytes, 54)?.take(length)?,
            size,
            stride,
            pixel_stride,
            true,
            check,
        )
    } else if bytes.starts_with(b"\x89PNG") {
        let source = PngRows::decode_with_palette_expansion(bytes, check)?;
        rows(
            &source.bytes,
            source.size,
            source.stride,
            source.stride / source.size[0] as usize,
            false,
            check,
        )
    } else if bytes.starts_with(b"PB2A") {
        pb2::decode_with_alpha(bytes)
    } else if bytes.starts_with(b"MSK0") {
        // The standalone decoder returns an 8-bit plane. The original scene
        // entry discards its depth and would read four bytes per pixel.
        bail!("CMVS MSK0 source cannot satisfy the scene upload stride")
    } else {
        bail!("unrecognized CMVS memory image")
    }?;
    check()?;
    Ok(decoded)
}

fn rows(
    bytes: &[u8],
    [width, height]: [u32; 2],
    stride: usize,
    pixel_stride: usize,
    bottom_up: bool,
    check: &dyn Fn() -> Result<()>,
) -> Result<(ImageSurface, bool)> {
    ensure!(
        pixel_stride >= 3,
        "CMVS source pixels are shorter than the upload stride"
    );
    let mut image = ImageSurface::filled(width, height, [0; 4])?;
    let width = width as usize;
    let height = height as usize;
    let alpha = pixel_stride == 4;
    for (line, output) in image.rgba.chunks_exact_mut(width * 4).enumerate() {
        check()?;
        let line = if bottom_up { height - 1 - line } else { line };
        let start = line
            .checked_mul(stride)
            .context("CMVS row offset overflow")?;
        let row = C::at(bytes, start)?.take(width * pixel_stride)?;
        for (pixel, input) in output
            .chunks_exact_mut(4)
            .zip(row.chunks_exact(pixel_stride))
        {
            pixel.copy_from_slice(&[
                input[2],
                input[1],
                input[0],
                if alpha { input[3] } else { 255 },
            ]);
        }
    }
    Ok((image, alpha))
}

// SPDX-License-Identifier: MPL-2.0
//! A one-channel matte stretched over a target size with clamped bilinear
//! filtering, for masks a renderer samples in screen space (normalized device
//! position to UV) rather than through the sprite's own coordinates.
use crate::ImageSurface;
use anyhow::{ensure, Result};

/// Returns an RGBA surface of `size` whose four channels all hold the sampled
/// `channel` (0 red .. 3 alpha) of `source`. Pixel centres map to source UV
/// `(x + 0.5) / width`, as a full-target quad does.
pub fn stretched_channel(source: &ImageSurface, channel: usize, size: [u32; 2]) -> Result<ImageSurface> {
    ensure!(channel < 4, "matte channel");
    ensure!(source.width > 0 && source.height > 0, "empty matte source");
    ensure!(size.iter().all(|axis| (1..=16384).contains(axis)), "matte size");
    let [width, height] = size;
    let (sw, sh) = (source.width as usize, source.height as usize);
    let texel = |x: usize, y: usize| f32::from(source.rgba[(y * sw + x) * 4 + channel]);
    let mut rgba = vec![0; crate::rgba_len(width, height)?];
    for y in 0..height as usize {
        let v = ((y as f32 + 0.5) / height as f32 * sh as f32 - 0.5).clamp(0., (sh - 1) as f32);
        let (y0, fy) = (v.floor() as usize, v.fract());
        let y1 = (y0 + 1).min(sh - 1);
        for x in 0..width as usize {
            let u = ((x as f32 + 0.5) / width as f32 * sw as f32 - 0.5).clamp(0., (sw - 1) as f32);
            let (x0, fx) = (u.floor() as usize, u.fract());
            let x1 = (x0 + 1).min(sw - 1);
            let top = texel(x0, y0) * (1. - fx) + texel(x1, y0) * fx;
            let bottom = texel(x0, y1) * (1. - fx) + texel(x1, y1) * fx;
            let value = (top * (1. - fy) + bottom * fy).round().clamp(0., 255.) as u8;
            rgba[(y * width as usize + x) * 4..][..4].fill(value);
        }
    }
    Ok(ImageSurface { width, height, rgba })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stretches_the_selected_channel() {
        let source = ImageSurface { width: 2, height: 1, rgba: vec![9, 0, 9, 9, 9, 255, 9, 9] };
        let matte = stretched_channel(&source, 1, [4, 2]).unwrap();
        let row: Vec<u8> = matte.rgba.chunks(4).take(4).map(|pixel| pixel[0]).collect();
        assert_eq!(row, [0, 64, 191, 255]);
        assert!(matte.rgba.chunks(4).all(|pixel| pixel.iter().all(|&v| v == pixel[0])));
    }
}

//! Small save previews sampled directly from the opaque display buffer.
//! Bilinear sampling uses four source pixels per output pixel, no full-frame copy.
use alloc::vec::Vec;

fn axis(source: usize, destination: usize, index: usize) -> (usize, usize, u32) {
    // Pixel-center mapping, with 8 fractional bits; division is outside the pixel loop.
    let position = (((2 * index as u64 + 1) * source as u64 * 128) / destination as u64)
        .saturating_sub(128).min((source as u64 - 1) * 256);
    let low = (position >> 8) as usize;
    (low, (low + 1).min(source - 1), (position & 255) as u32)
}

/// Crop coordinates are (left, top, right, bottom) in a packed RGBA8 framebuffer.
/// Intended for opaque display pixels, not for transparent sprite resampling.
pub fn capture_rgba8(
    frame: &[u8], frame_width: usize, frame_height: usize,
    crop: (usize, usize, usize, usize), width: u32, height: u32,
) -> Option<Vec<u8>> {
    let frame_bytes = frame_width.checked_mul(frame_height)?.checked_mul(4)?;
    let (left, top, right, bottom) = crop;
    if width == 0 || height == 0 || frame.len() < frame_bytes
        || left >= right || top >= bottom || right > frame_width || bottom > frame_height {
        return None;
    }
    let (width, height) = (width as usize, height as usize);
    let output_bytes = width.checked_mul(height)?.checked_mul(4)?;
    let columns: Vec<_> = (0..width).map(|x| axis(right - left, width, x)).collect();
    let mut out = Vec::with_capacity(output_bytes);
    for y in 0..height {
        let (y0, y1, wy) = axis(bottom - top, height, y);
        let row0 = (top + y0) * frame_width;
        let row1 = (top + y1) * frame_width;
        for &(x0, x1, wx) in &columns {
            let p00 = (row0 + left + x0) * 4;
            let p01 = (row0 + left + x1) * 4;
            let p10 = (row1 + left + x0) * 4;
            let p11 = (row1 + left + x1) * 4;
            for channel in 0..4 {
                let upper = frame[p00 + channel] as u32 * (256 - wx) + frame[p01 + channel] as u32 * wx;
                let lower = frame[p10 + channel] as u32 * (256 - wx) + frame[p11 + channel] as u32 * wx;
                out.push(((upper * (256 - wy) + lower * wy + 32768) >> 16) as u8);
            }
        }
    }
    Some(out)
}


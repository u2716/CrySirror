//! Straight RGBA bilinear sampling with independently truncated7-bit passes.
use crate::ImageSurface;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ByteSampleEdge {
    Zero,
    Clamp,
    /// Clamp RGB to the nearest edge but clear coverage outside the image.
    ClampTransparent,
    /// Historical signed-coordinate profile: clear bit31 before modulo.
    SignMaskedWrap,
    /// Reflect the absolute coordinate, repeating each endpoint once.
    AbsoluteMirror,
}
pub fn bilinear128(image: &ImageSurface, point: [f32; 2]) -> Result<[u8; 4]> {
    image.validate()?;
    ensure!(
        point.iter().all(|v| v.is_finite() && v.abs() <= 1_000_000.),
        "byte sampler coordinate bounds"
    );
    Ok(bilinear128_validated(image, point))
}
pub(crate) fn bilinear128_validated(image: &ImageSurface, [x, y]: [f32; 2]) -> [u8; 4] {
    bilinear128_edge_validated(image, [x, y], ByteSampleEdge::Zero)
}
pub(crate) fn bilinear128_edge_validated(
    image: &ImageSurface,
    [x, y]: [f32; 2],
    edge: ByteSampleEdge,
) -> [u8; 4] {
    let floor = |v: f32| {
        if v < 0. {
            v.trunc() as i32 - 1
        } else {
            v.trunc() as i32
        }
    };
    let (ix, iy) = (floor(x), floor(y));
    let wx = ((f64::from(x) - f64::from(ix)) * 128. + 0.5).trunc() as u32;
    let wy = ((f64::from(y) - f64::from(iy)) * 128. + 0.5).trunc() as u32;
    let tap = |x: i32, y: i32| -> [u8; 4] {
        let outside = x < 0 || y < 0 || x >= image.width as i32 || y >= image.height as i32;
        if outside && matches!(edge, ByteSampleEdge::Zero) {
            return [0; 4];
        }
        let address = |v: i32, size: u32| match edge {
            ByteSampleEdge::SignMaskedWrap => (v as u32 & 0x7fffffff) % size,
            ByteSampleEdge::AbsoluteMirror => {
                let p = v.unsigned_abs() % (2 * size);
                if p >= size { 2 * size - 1 - p } else { p }
            }
            _ => v.clamp(0, size as i32 - 1) as u32,
        };
        let at = ((address(y, image.height) * image.width + address(x, image.width)) * 4) as usize;
        let mut pixel: [u8; 4] = image.rgba[at..at + 4].try_into().unwrap();
        if outside && matches!(edge, ByteSampleEdge::ClampTransparent) {
            pixel[3] = 0;
        }
        pixel
    };
    let mix = |a: [u8; 4], b: [u8; 4], w: u32| {
        std::array::from_fn(|c| ((u32::from(a[c]) * (128 - w) + u32::from(b[c]) * w) >> 7) as u8)
    };
    mix(
        mix(tap(ix, iy), tap(ix + 1, iy), wx),
        mix(tap(ix, iy + 1), tap(ix + 1, iy + 1), wx),
        wy,
    )
}

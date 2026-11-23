//! Tiled affine redraw and channel range stretch.
//!
//! `TiledTransform` draws the source stretched to the output size as a
//! `(2r+1)^2` grid of tiles centred on the output, then scales each corner
//! about the output centre, rotates it (degrees, `x' = x cos - y sin`,
//! `y' = x sin + y cos`) and offsets it. Pixels outside every tile keep
//! `background`. Sampling is bilinear (or nearest) inside each tile with its
//! texture coordinates clamped to that tile, so tiles do not blend across
//! their seams. QLIE 53C288 prepares transition rule images this way.
//!
//! `ChannelRangeStretch` maps one channel's observed `[min, max]` onto
//! `0..=255` with `((v - min) * (0xFF00 / (max - min))) >> 8` (signed, rounded
//! toward zero), leaving the image unchanged when `max - min <= 1` or the
//! range is already `0..=255` (QLIE 53C108/53C190).
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TiledTransform {
    pub size: [u32; 2],
    /// Tile radius around the centre tile (0..=8).
    pub radius: u8,
    /// Added after rotation, in output pixels.
    pub offset: [f32; 2],
    pub scale: [f32; 2],
    pub rotation_degrees: f32,
    pub smooth: bool,
    pub background: [u8; 4],
}
impl TiledTransform {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.size.iter().all(|n| (1..=16384).contains(n))
                && u64::from(self.size[0]) * u64::from(self.size[1]) <= MAX_EFFECT_PIXELS,
            "tiled transform extent"
        );
        ensure!(self.radius <= 8, "tiled transform radius");
        ensure!(
            self.offset.iter().all(|v| v.is_finite() && v.abs() <= 1e6)
                && self.scale.iter().all(|v| v.is_finite() && v.abs() >= 1e-4 && v.abs() <= 1e4)
                && self.rotation_degrees.is_finite()
                && self.rotation_degrees.abs() <= 1e6,
            "tiled transform geometry"
        );
        Ok(())
    }
    pub fn apply(&self, src: &ImageSurface, check: &dyn Fn() -> Result<()>) -> Result<ImageSurface> {
        self.validate()?;
        src.validate()?;
        let [w, h] = self.size;
        let (wf, hf) = (f64::from(w), f64::from(h));
        let (cx, cy) = (wf * 0.5, hf * 0.5);
        let angle = std::f64::consts::PI * f64::from(self.rotation_degrees) / 180.;
        let (sin, cos) = if angle == 0. { (0., 1.) } else { angle.sin_cos() };
        let [sx, sy] = self.scale.map(f64::from);
        let [ox, oy] = self.offset.map(f64::from);
        let r = f64::from(self.radius);
        let (sw, sh) = (src.width as usize, src.height as usize);
        let fetch = |x: usize, y: usize| -> [f64; 4] {
            let at = (y * sw + x) * 4;
            std::array::from_fn(|c| f64::from(src.rgba[at + c]))
        };
        let mut rgba = Vec::with_capacity(w as usize * h as usize * 4);
        for y in 0..h {
            check()?;
            for x in 0..w {
                // Invert: subtract offset/centre, unrotate, unscale.
                let px = f64::from(x) + 0.5 - cx - ox;
                let py = f64::from(y) + 0.5 - cy - oy;
                let (ux, uy) = (px * cos + py * sin, -px * sin + py * cos);
                let (qx, qy) = (ux / sx + cx, uy / sy + cy);
                let (tx, ty) = ((qx / wf).floor(), (qy / hf).floor());
                if tx < -r || tx > r || ty < -r || ty > r {
                    rgba.extend_from_slice(&self.background);
                    continue;
                }
                // Source coordinates of this tile, in texels.
                let u = (qx / wf - tx) * sw as f64;
                let v = (qy / hf - ty) * sh as f64;
                let pixel = if self.smooth {
                    let fx = (u - 0.5).clamp(0., (sw - 1) as f64);
                    let fy = (v - 0.5).clamp(0., (sh - 1) as f64);
                    let (x0, y0) = (fx as usize, fy as usize);
                    let (x1, y1) = ((x0 + 1).min(sw - 1), (y0 + 1).min(sh - 1));
                    let (ax, ay) = (fx - x0 as f64, fy - y0 as f64);
                    let (a, b, c, d) = (fetch(x0, y0), fetch(x1, y0), fetch(x0, y1), fetch(x1, y1));
                    std::array::from_fn::<u8, 4, _>(|i| {
                        let top = a[i] + (b[i] - a[i]) * ax;
                        let bottom = c[i] + (d[i] - c[i]) * ax;
                        (top + (bottom - top) * ay).round().clamp(0., 255.) as u8
                    })
                } else {
                    let fetched = fetch(
                        (u as usize).min(sw - 1),
                        (v as usize).min(sh - 1),
                    );
                    fetched.map(|v| v as u8)
                };
                rgba.extend_from_slice(&pixel);
            }
        }
        check()?;
        Ok(ImageSurface { width: w, height: h, rgba })
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelRangeStretch {
    /// 0 red, 1 green, 2 blue, 3 alpha.
    pub channel: u8,
}
impl ChannelRangeStretch {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.channel < 4, "range stretch channel");
        Ok(())
    }
    pub fn apply(&self, src: &ImageSurface, check: &dyn Fn() -> Result<()>) -> Result<ImageSurface> {
        self.validate()?;
        let c = self.channel as usize;
        let (mut min, mut max) = (255i32, 0i32);
        for p in src.rgba.chunks_exact(4) {
            let v = i32::from(p[c]);
            min = min.min(v);
            max = max.max(v);
        }
        let mut out = src.clone();
        if max - min <= 1 || (min == 0 && max == 255) {
            return Ok(out);
        }
        let factor = 0xFF00 / (max - min);
        for row in out.rgba.chunks_mut(src.width as usize * 4) {
            check()?;
            for p in row.chunks_exact_mut(4) {
                p[c] = (((i32::from(p[c]) - min) * factor) / 256) as u8;
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn surface(w: u32, h: u32, f: impl Fn(u32, u32) -> u8) -> ImageSurface {
        let mut rgba = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let v = f(x, y);
                rgba.extend_from_slice(&[v, v, v, 255]);
            }
        }
        ImageSurface { width: w, height: h, rgba }
    }
    #[test]
    fn identity_stretches_source_to_size() {
        let src = surface(2, 1, |x, _| if x == 0 { 10 } else { 200 });
        let t = TiledTransform {
            size: [4, 2],
            radius: 2,
            offset: [0.; 2],
            scale: [1.; 2],
            rotation_degrees: 0.,
            smooth: false,
            background: [0; 4],
        };
        let out = t.apply(&src, &|| Ok(())).unwrap();
        let reds: Vec<u8> = out.rgba.chunks(4).map(|p| p[0]).collect();
        assert_eq!(reds, [10, 10, 200, 200, 10, 10, 200, 200]);
    }
    #[test]
    fn half_scale_tiles_and_offset_shift() {
        let src = surface(2, 1, |x, _| if x == 0 { 10 } else { 200 });
        let mut t = TiledTransform {
            size: [4, 1],
            radius: 2,
            offset: [0.; 2],
            scale: [0.5, 1.],
            rotation_degrees: 0.,
            smooth: false,
            background: [0; 4],
        };
        let reds = |t: &TiledTransform| -> Vec<u8> {
            t.apply(&src, &|| Ok(())).unwrap().rgba.chunks(4).map(|p| p[0]).collect()
        };
        // The centre copy spans x 1..3; its neighbours fill both edges.
        assert_eq!(reds(&t), [200, 10, 200, 10]);
        t.offset = [1., 0.];
        assert_eq!(reds(&t), [10, 200, 10, 200]);
    }
    #[test]
    fn stretch_maps_range() {
        let src = surface(3, 1, |x, _| [64u8, 128, 192][x as usize]);
        let out = ChannelRangeStretch { channel: 0 }.apply(&src, &|| Ok(())).unwrap();
        // factor = 0xFF00 / 128 = 510.
        assert_eq!(out.rgba[0], 0);
        assert_eq!(out.rgba[4], (64 * 510 / 256) as u8);
        assert_eq!(out.rgba[8], (128 * 510 / 256) as u8);
        assert_eq!(out.rgba[1], 64);
    }
}

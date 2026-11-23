// SPDX-License-Identifier: MPL-2.0
//! Scaled/rotated sprite composition onto a transparent canvas. Each draw
//! copies a source rectangle, scales it about its own centre (negative scale
//! mirrors), optionally rotates it, samples nearest and blends straight-alpha over the
//! canvas. Used by N-System Split/TurnOver/Drop/FloatBubble, whose original
//! draws are per-tile/per-particle surface transforms.
use crate::ImageSurface;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TileDraw {
    /// Source rectangle x, y, width, height (clipped to the input).
    pub src: [i32; 4],
    /// Canvas position of the unscaled rectangle's top-left corner.
    pub dst: [i32; 2],
    /// Horizontal/vertical scale in 1/256 units; the sign mirrors the axis.
    pub scale: [i32; 2],
    pub opacity: u8,
    /// Rotation around the scaled tile centre, clockwise in canvas pixels.
    /// Zero keeps the existing axis-scaled sampling path exactly unchanged.
    #[serde(default)]
    pub rotation_degrees: i32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TileCompose {
    pub canvas: [u32; 2],
    pub tiles: Vec<TileDraw>,
}
const MAX_TILES: usize = 65_536;
const MAX_DRAW_PIXELS: u64 = 64 * 1024 * 1024;
impl TileCompose {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.canvas.iter().all(|n| (1..=16_384).contains(n))
                && u64::from(self.canvas[0]) * u64::from(self.canvas[1])
                    <= crate::effects::MAX_EFFECT_PIXELS
                && self.tiles.len() <= MAX_TILES
                && self.tiles.iter().all(|t| {
                    t.rotation_degrees.unsigned_abs() <= 1_000_000
                        && t.scale.iter().all(|s| s.unsigned_abs() <= 256 * 64)
                        && t.src.iter().all(|v| v.unsigned_abs() <= 1 << 20)
                        && t.dst.iter().all(|v| v.unsigned_abs() <= 1 << 20)
                }),
            "tile compose bounds"
        );
        Ok(())
    }
    pub fn apply(
        &self,
        input: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        input.validate()?;
        let [cw, ch] = self.canvas;
        let mut out = ImageSurface {
            width: cw,
            height: ch,
            rgba: vec![0; crate::rgba_len(cw, ch)?],
        };
        let (iw, ih) = (i64::from(input.width), i64::from(input.height));
        let mut budget = 0u64;
        for (n, tile) in self.tiles.iter().enumerate() {
            if n % 256 == 0 {
                check()?;
            }
            let [sx, sy, sw, sh] = tile.src.map(i64::from);
            let (x0, y0) = (sx.max(0), sy.max(0));
            let (x1, y1) = ((sx + sw).min(iw), (sy + sh).min(ih));
            if x1 <= x0 || y1 <= y0 || tile.opacity == 0 {
                continue;
            }
            let (w, h) = (x1 - x0, y1 - y0);
            let (kx, ky) = (i64::from(tile.scale[0]), i64::from(tile.scale[1]));
            let (dw, dh) = (w * kx.abs() / 256, h * ky.abs() / 256);
            if dw == 0 || dh == 0 {
                continue;
            }
            if tile.rotation_degrees.rem_euclid(360) != 0 {
                let angle = f64::from(tile.rotation_degrees).to_radians();
                let (sin, cos) = angle.sin_cos();
                let center = [
                    f64::from(tile.dst[0]) + w as f64 * 0.5,
                    f64::from(tile.dst[1]) + h as f64 * 0.5,
                ];
                let half = [
                    (dw as f64 * cos.abs() + dh as f64 * sin.abs()) * 0.5,
                    (dw as f64 * sin.abs() + dh as f64 * cos.abs()) * 0.5,
                ];
                let left = (center[0] - half[0]).floor() as i64;
                let top = (center[1] - half[1]).floor() as i64;
                let right = (center[0] + half[0]).ceil() as i64;
                let bottom = (center[1] + half[1]).ceil() as i64;
                budget += ((right - left) * (bottom - top)) as u64;
                ensure!(
                    budget <= MAX_DRAW_PIXELS,
                    "rotated tile compose draw budget"
                );
                for y in top.max(0)..bottom.min(i64::from(ch)) {
                    check()?;
                    for x in left.max(0)..right.min(i64::from(cw)) {
                        let dx = x as f64 + 0.5 - center[0];
                        let dy = y as f64 + 0.5 - center[1];
                        let u = ((cos * dx + sin * dy) * 256. / kx as f64 + w as f64 * 0.5).floor()
                            as i64;
                        let v = ((-sin * dx + cos * dy) * 256. / ky as f64 + h as f64 * 0.5).floor()
                            as i64;
                        if u < 0 || v < 0 || u >= w || v >= h {
                            continue;
                        }
                        let source = (((y0 + v) * iw + x0 + u) * 4) as usize;
                        let dest = ((y * i64::from(cw) + x) * 4) as usize;
                        over(
                            &mut out.rgba[dest..dest + 4],
                            &input.rgba[source..source + 4],
                            tile.opacity,
                        );
                    }
                }
                continue;
            }
            budget += (dw * dh) as u64;
            ensure!(budget <= MAX_DRAW_PIXELS, "tile compose draw budget");
            // Scale about the centre of the unscaled destination rectangle.
            let left = (i64::from(tile.dst[0]) * 2 + w - dw) / 2;
            let top = (i64::from(tile.dst[1]) * 2 + h - dh) / 2;
            for dy in 0..dh {
                let y = top + dy;
                if y < 0 || y >= i64::from(ch) {
                    continue;
                }
                let mut row = dy * h / dh;
                if ky < 0 {
                    row = h - 1 - row;
                }
                for dx in 0..dw {
                    let x = left + dx;
                    if x < 0 || x >= i64::from(cw) {
                        continue;
                    }
                    let mut col = dx * w / dw;
                    if kx < 0 {
                        col = w - 1 - col;
                    }
                    let s = (((y0 + row) * iw + x0 + col) * 4) as usize;
                    let d = ((i64::from(y) * i64::from(cw) + x) * 4) as usize;
                    over(&mut out.rgba[d..d + 4], &input.rgba[s..s + 4], tile.opacity);
                }
            }
        }
        check()?;
        Ok(out)
    }
}
fn over(dest: &mut [u8], source: &[u8], opacity: u8) {
    let alpha = u32::from(source[3]) * u32::from(opacity) / 255;
    if alpha == 0 {
        return;
    }
    let back = u32::from(dest[3]);
    if back == 0 {
        dest[..3].copy_from_slice(&source[..3]);
        dest[3] = alpha as u8;
        return;
    }
    let kept = back * (255 - alpha) / 255;
    let total = alpha + kept;
    for c in 0..3 {
        dest[c] = ((u32::from(source[c]) * alpha + u32::from(dest[c]) * kept) / total) as u8;
    }
    dest[3] = total.min(255) as u8;
}
#[cfg(test)]
mod tests {
    use super::*;
    fn solid(w: u32, h: u32, p: [u8; 4]) -> ImageSurface {
        ImageSurface {
            width: w,
            height: h,
            rgba: p.repeat((w * h) as usize),
        }
    }
    #[test]
    fn tile_moves_scales_and_mirrors() {
        let mut src = solid(2, 1, [10, 0, 0, 255]);
        src.rgba[4..8].copy_from_slice(&[200, 0, 0, 255]);
        let spec = TileCompose {
            canvas: [6, 3],
            tiles: vec![TileDraw {
                src: [0, 0, 2, 1],
                dst: [1, 1],
                scale: [-256, 256],
                opacity: 255,
                rotation_degrees: 0,
            }],
        };
        let out = spec.apply(&src, &|| Ok(())).unwrap();
        let at = |x: usize, y: usize| out.rgba[(y * 6 + x) * 4];
        assert_eq!((at(1, 1), at(2, 1)), (200, 10));
        assert_eq!(out.rgba[3], 0);
    }
}

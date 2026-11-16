// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Byte-exact layer utilities from wamsoft layerExBTOA (a57e7a25) and
//! shrinkCopy (007fbba3), both MIT-style plug-in sources: channel moves,
//! alpha fill, threshold overwrite and area-average / integer shrinking.
//! Images are straight RGBA; the natives' BGRA order only permutes channels.
use crate::ImageSurface;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

/// `copyRightBlueToLeftAlpha` (`vertical: false`) or `copyBottomBlueToTopAlpha`:
/// the second half's blue becomes the first half's alpha. Whole image.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlueToAlpha {
    pub vertical: bool,
}
impl BlueToAlpha {
    pub fn apply(&self, image: &ImageSurface) -> Result<ImageSurface> {
        image.validate()?;
        let (w, h) = (image.width as usize, image.height as usize);
        let mut out = image.clone();
        let (cw, ch) = if self.vertical { (w, h / 2) } else { (w / 2, h) };
        for y in 0..ch {
            for x in 0..cw {
                let (sx, sy) = if self.vertical { (x, y + ch) } else { (x + cw, y) };
                out.rgba[(y * w + x) * 4 + 3] = image.rgba[(sy * w + sx) * 4 + 2];
            }
        }
        Ok(out)
    }
}

/// `fillAlpha`: alpha 255 inside the clip rectangle.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FillAlpha {
    pub rect: [i32; 4],
}
impl FillAlpha {
    pub fn apply(&self, image: &ImageSurface) -> Result<ImageSurface> {
        image.validate()?;
        let mut out = image.clone();
        let w = image.width as i32;
        let x0 = self.rect[0].clamp(0, w);
        let y0 = self.rect[1].clamp(0, image.height as i32);
        let x1 = (self.rect[0] + self.rect[2]).clamp(x0, w);
        let y1 = (self.rect[1] + self.rect[3]).clamp(y0, image.height as i32);
        for y in y0..y1 {
            for x in x0..x1 {
                out.rgba[((y * w + x) * 4 + 3) as usize] = 255;
            }
        }
        Ok(out)
    }
}

/// layerExBTOA `CalcClipArea`: true when nothing remains.
fn clip_area(d: &mut [i32; 2], dsize: [i32; 2], s: &mut [i32; 2], ssize: [i32; 2], wh: &mut [i32; 2]) -> bool {
    for i in 0..2 {
        if s[i] + wh[i] <= 0 || s[i] >= ssize[i] {
            return true;
        }
    }
    for i in 0..2 {
        if s[i] < 0 {
            wh[i] += s[i];
            d[i] -= s[i];
            s[i] = 0;
        }
        let cut = s[i] + wh[i] - ssize[i];
        if cut > 0 {
            wh[i] -= cut;
        }
    }
    for i in 0..2 {
        if d[i] + wh[i] <= 0 || d[i] >= dsize[i] {
            return true;
        }
    }
    for i in 0..2 {
        if d[i] < 0 {
            wh[i] += d[i];
            s[i] -= d[i];
            d[i] = 0;
        }
        let cut = d[i] + wh[i] - dsize[i];
        if cut > 0 {
            wh[i] -= cut;
        }
    }
    wh[0] <= 0 || wh[1] <= 0
}

/// `overwrapRect`: copy source pixels whose alpha is at least `threshold`
/// into the destination clip (coordinates are layer image coordinates).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverwrapRect {
    pub destination: [i32; 2],
    pub source: [i32; 4],
    pub clip: [i32; 4],
    pub threshold: u8,
}
impl OverwrapRect {
    pub fn apply(&self, destination: &ImageSurface, source: &ImageSurface) -> Result<ImageSurface> {
        destination.validate()?;
        source.validate()?;
        ensure!(self.source[2] > 0 && self.source[3] > 0, "overwrapRect extent");
        let mut out = destination.clone();
        let [cl, ct, cw, ch] = self.clip;
        let mut d = [self.destination[0] - cl, self.destination[1] - ct];
        let mut s = [self.source[0], self.source[1]];
        let mut wh = [self.source[2], self.source[3]];
        if clip_area(&mut d, [cw, ch], &mut s, [source.width as i32, source.height as i32], &mut wh) {
            return Ok(out);
        }
        let (dw, sw) = (destination.width as usize, source.width as usize);
        for y in 0..wh[1] as usize {
            for x in 0..wh[0] as usize {
                let si = ((s[1] as usize + y) * sw + s[0] as usize + x) * 4;
                if source.rgba[si + 3] >= self.threshold {
                    let di = ((ct + d[1]) as usize + y) * dw + (cl + d[0]) as usize + x;
                    out.rgba[di * 4..di * 4 + 4].copy_from_slice(&source.rgba[si..si + 4]);
                }
            }
        }
        Ok(out)
    }
}

/// `shrinkCopyFast(src, sx, sy)`: integer box shrink ignoring alpha; the
/// result is `ceil(w/sx) × ceil(h/sy)` and opaque.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShrinkFast {
    pub step: [u32; 2],
}
impl ShrinkFast {
    pub fn apply(&self, source: &ImageSurface) -> Result<ImageSurface> {
        source.validate()?;
        let [sx, sy] = self.step;
        ensure!(sx > 0 && sy > 0, "shrinkCopyFast factor");
        let (w, h) = (source.width as usize, source.height as usize);
        let (sx, sy) = (sx as usize, sy as usize);
        let dw = w.div_ceil(sx);
        let dh = h.div_ceil(sy);
        let px = |x: usize, y: usize, c: usize| source.rgba[(y * w + x) * 4 + c] as u32;
        // Horizontal pass per source row, then vertical over row groups.
        let xdiv = w / sx;
        let xrem = w - xdiv * sx;
        let row = |y: usize| -> Vec<[u32; 3]> {
            let mut out = Vec::with_capacity(dw);
            for i in 0..xdiv {
                let mut s = [0u32; 3];
                for k in 0..sx {
                    for (c, v) in s.iter_mut().enumerate() {
                        *v += px(i * sx + k, y, c);
                    }
                }
                out.push(if sx == 1 { s } else { s.map(|v| v / sx as u32) });
            }
            if xrem > 0 {
                let mut s = [0u32; 3];
                for k in 0..xrem {
                    for (c, v) in s.iter_mut().enumerate() {
                        *v += px(xdiv * sx + k, y, c);
                    }
                }
                out.push(if xrem == 1 { s } else { s.map(|v| v / xrem as u32) });
            }
            out
        };
        let mut rgba = Vec::with_capacity(dw * dh * 4);
        if sy <= 1 {
            for y in 0..h {
                for p in row(y) {
                    rgba.extend([p[0] as u8, p[1] as u8, p[2] as u8, 255]);
                }
            }
        } else {
            let mut y = 0;
            while y < h {
                let n = sy.min(h - y);
                let rows: Vec<_> = (y..y + n).map(row).collect();
                for x in 0..dw {
                    let mut s = [0u32; 3];
                    for r in &rows {
                        for c in 0..3 {
                            s[c] += r[x][c] & 0xff;
                        }
                    }
                    let s = if n == 1 { s } else { s.map(|v| v / n as u32) };
                    rgba.extend([s[0] as u8, s[1] as u8, s[2] as u8, 255]);
                }
                y += n;
            }
        }
        Ok(ImageSurface { width: dw as u32, height: dh as u32, rgba })
    }
}

/// `shrinkCopy`: area-average reduction of a source rectangle onto a
/// fractional destination rectangle (enlargement is invalid).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShrinkCopy {
    pub destination: [f64; 4],
    pub source: [i32; 4],
}
#[derive(Clone, Copy, Default)]
struct Avg {
    offset: i64,
    step: i64,
    ta: u64,
    tc: u64,
    ba: u64,
    bc: u64,
    total: u64,
}
fn rtol(r: f64) -> i64 {
    if r < 0. { -((-r) as i64) } else { r as i64 }
}
impl ShrinkCopy {
    pub fn validate(&self) -> Result<()> {
        let [_, _, dw, dh] = self.destination;
        let [_, _, sw, sh] = self.source;
        ensure!(
            sw > 0 && sh > 0 && dw > 0. && dh > 0. && sw as i64 >= dw as i64 && sh as i64 >= dh as i64,
            "shrinkCopy rectangle (enlargement or empty)"
        );
        Ok(())
    }
    pub fn apply(&self, destination: &ImageSurface, source: &ImageSurface) -> Result<ImageSurface> {
        self.validate()?;
        destination.validate()?;
        source.validate()?;
        let mut out = destination.clone();
        let [mut dx, mut dy, mut dw, mut dh] = self.destination;
        let [mut sx, mut sy, mut sw, mut sh] = self.source.map(i64::from);
        let (siw, sih) = (source.width as i64, source.height as i64);
        let (diw, dih) = (destination.width as i64, destination.height as i64);
        let zx = dw / sw as f64;
        let zy = dh / sh as f64;
        if sx + sw <= 0 || sy + sh <= 0 || sx >= siw || sy >= sih {
            return Ok(out);
        }
        if sx < 0 {
            sw += sx;
            let cut = zx * (-sx) as f64;
            dw -= cut;
            sx = 0;
            dx += cut;
        }
        if sy < 0 {
            sh += sy;
            let cut = zy * (-sy) as f64;
            dh -= cut;
            sy = 0;
            dy += cut;
        }
        let cut = sx + sw - siw;
        if cut > 0 {
            sw -= cut;
            dw -= zx * cut as f64;
        }
        let cut = sy + sh - sih;
        if cut > 0 {
            sh -= cut;
            dh -= zy * cut as f64;
        }
        let dtx = rtol(dx);
        let dty = rtol(dy);
        let mut dtw = rtol(dx + dw) - dtx;
        let mut dth = rtol(dy + dh) - dty;
        if dx + dw > (dtx + dtw) as f64 {
            dtw += 1;
        }
        if dy + dh > (dty + dth) as f64 {
            dth += 1;
        }
        if dtx + dtw <= 0 || dty + dth <= 0 || dtx >= diw || dty >= dih {
            return Ok(out);
        }
        let dsx = if dtx < 0 { -dtx } else { 0 };
        let dsy = if dty < 0 { -dty } else { 0 };
        let dex = if dtx + dtw > diw { diw - dtx } else { dtw };
        let dey = if dty + dth > dih { dih - dty } else { dth };
        // Offsets index pixels (x) or rows (y) rather than bytes.
        let table = |pos: i64, end: i64, max: f64, ratio: f64, diff: f64, stop: i64| -> (Vec<Avg>, u64) {
            let mut unit = 256u64;
            if ratio <= 1. / 16. {
                unit /= ((2. / 16.) / ratio) as u64;
                if unit == 0 {
                    unit = 1;
                }
            }
            let info = |p: i64| {
                let r1 = (p as f64 - diff) * ratio;
                let r2 = r1 + ratio;
                let (t1, t2) = (rtol(r1), rtol(r2));
                let ta = unit - rtol((r1 - t1 as f64) * unit as f64) as u64;
                let ba = rtol((r2 - t2 as f64) * unit as f64) as u64;
                Avg { offset: stop + t1 + 1, step: t2 - t1 - 1, ta, tc: ta, ba, bc: ba, total: ta + ba + ((t2 - t1 - 1) as u64).wrapping_mul(unit) }
            };
            let edge = |p: i64| {
                let r1 = (p as f64 - diff) * ratio;
                let r2 = r1 + ratio;
                let f1 = r1.max(0.);
                let f2 = r2.min(max);
                let (t1, t2) = (rtol(r1), rtol(r2));
                let (u1, u2) = (rtol(f1), rtol(f2));
                let tc = unit - rtol((r1 - t1 as f64) * unit as f64) as u64;
                let bc = rtol((r2 - t2 as f64) * unit as f64) as u64;
                Avg {
                    offset: stop + u1 + 1,
                    total: tc + bc + ((t2 - t1 - 1) as u64).wrapping_mul(unit),
                    tc,
                    bc,
                    ta: unit - rtol((f1 - u1 as f64) * unit as f64) as u64,
                    ba: rtol((f2 - u2 as f64) * unit as f64) as u64,
                    step: u2 - u1 - 1,
                }
            };
            let mut v = vec![];
            if pos == end {
                v.push(edge(pos));
            } else {
                v.push(edge(pos));
                for p in pos + 1..end {
                    v.push(info(p));
                }
                v.push(edge(end));
            }
            (v, unit)
        };
        let (horz, hu) = table(dsx, dex - 1, sw as f64, sw as f64 / dw, dx - dtx as f64, sx);
        let (vert, vu) = table(dsy, dey - 1, sh as f64, sh as f64 / dh, dy - dty as f64, sy);
        let unit = hu * vu;
        let sw_px = source.width as i64;
        let pixel = |x: i64, y: i64| -> [u64; 4] {
            // The native reads neighbours outside the rectangle only with a
            // zero weight; clamp so such reads stay in bounds.
            let x = x.clamp(0, siw - 1);
            let y = y.clamp(0, sih - 1);
            let i = ((y * sw_px + x) * 4) as usize;
            [0, 1, 2, 3].map(|c| source.rgba[i + c] as u64)
        };
        let add = |sum: &mut [u64; 4], p: [u64; 4], mul_a: u64, mul_c: u64| {
            if mul_a != 0 {
                for c in 0..3 {
                    sum[c] += p[c] * mul_c;
                }
                sum[3] += p[3] * mul_a;
            }
        };
        let dwidth = destination.width as i64;
        for (yi, vi) in vert.iter().enumerate() {
            let y = dsy + yi as i64;
            for (xi, hi) in horz.iter().enumerate() {
                let x = dsx + xi as i64;
                let mut sum = [0u64; 4];
                let (rx, ry) = (hi.offset, vi.offset);
                let (w, h) = (hi.step, vi.step);
                add(&mut sum, pixel(rx - 1, ry - 1), hi.ta * vi.ta, hi.tc * vi.tc);
                for k in 0..w.max(0) {
                    add(&mut sum, pixel(rx + k, ry - 1), hu * vi.ta, hu * vi.tc);
                }
                add(&mut sum, pixel(rx + w, ry - 1), hi.ba * vi.ta, hi.bc * vi.tc);
                for k in 0..h.max(0) {
                    add(&mut sum, pixel(rx - 1, ry + k), hi.ta * vu, hi.tc * vu);
                }
                if unit != 0 {
                    for j in 0..h.max(0) {
                        for k in 0..w.max(0) {
                            add(&mut sum, pixel(rx + k, ry + j), unit, unit);
                        }
                    }
                }
                for k in 0..h.max(0) {
                    add(&mut sum, pixel(rx + w, ry + k), hi.ba * vu, hi.bc * vu);
                }
                add(&mut sum, pixel(rx - 1, ry + h), hi.ta * vi.ba, hi.tc * vi.bc);
                for k in 0..w.max(0) {
                    add(&mut sum, pixel(rx + k, ry + h), hu * vi.ba, hu * vi.bc);
                }
                add(&mut sum, pixel(rx + w, ry + h), hi.ba * vi.ba, hi.bc * vi.bc);
                let div = (hi.total * vi.total).max(1);
                let o = (((dty + y) * dwidth + dtx + x) * 4) as usize;
                for c in 0..4 {
                    out.rgba[o + c] = (sum[c] / div) as u8;
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn img(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> ImageSurface {
        let mut rgba = vec![];
        for y in 0..h {
            for x in 0..w {
                rgba.extend(f(x, y));
            }
        }
        ImageSurface { width: w, height: h, rgba }
    }
    #[test]
    fn glitch_transition_endpoints_tint_and_copy_equivalence() {
        let old = img(37, 9, |x, y| [x as u8 * 5, y as u8 * 20, 100, 200]);
        let new = img(37, 9, |x, y| [200, x as u8 * 3, y as u8 * 7, 255]);
        let params = GlitchParams { noise: 6., shift: [20., 9., 8.], probability: [0.7, 0.4, 0.5], reset: 0.02, block: 8 };
        let exchange = GlitchTransition::Exchange { gamma_in: 1., gamma_out: 2. };
        // t = 0: every row is old at zero intensity; t = 1: every row new.
        assert_eq!(exchange.frame(&params, [37, 9], 0., 5).render(&old, &new).unwrap(), old);
        assert_eq!(exchange.frame(&params, [37, 9], 1., 5).render(&old, &new).unwrap(), new);
        let mid = exchange.frame(&params, [37, 9], 0.5, 5);
        assert!(mid.rows.iter().any(|r| r.0) && mid.rows.iter().any(|r| !r.0));
        assert_ne!(mid.render(&old, &new).unwrap(), old);
        // Fully faded to black: channels drop by 256, alpha is kept.
        let fade = GlitchTransition::Fade { fade_in: false, gamma: 1., color: Some([0, 0, 0]) };
        let out = fade.frame(&params, [37, 9], 1., 5).render(&old, &new).unwrap();
        assert!(out.rgba.chunks(4).all(|p| p[..3] == [0, 0, 0] && p[3] == 200));
        assert_eq!(glitch_tint([255, 0, 64], 0.25), [159, 96, 112]);
        assert_eq!(apply_glitch_tint([10, 250, 128], [100, 200, 128]), [0, 255, 128]);
        // glitchCopy over a whole image equals a single-source frame at coef 1.
        let copy = GlitchCopy {
            destination: [0, 0, 37, 9],
            source: [0, 0],
            noise: params.noise,
            shift: params.shift,
            probability: params.probability,
            reset: params.reset,
            block: params.block,
            seed: 11,
            coef: 1.,
        };
        let fade = GlitchTransition::Fade { fade_in: false, gamma: 1., color: None };
        assert_eq!(copy.apply(&new, &old).unwrap(), fade.frame(&params, [37, 9], 1., 11).render(&old, &new).unwrap());
    }
    #[test]
    fn blue_to_alpha_and_fill() {
        let i = img(4, 2, |x, _| [0, 0, x as u8 * 10, 7]);
        let o = BlueToAlpha { vertical: false }.apply(&i).unwrap();
        assert_eq!([o.rgba[3], o.rgba[7], o.rgba[11]], [20, 30, 7]);
        let o = FillAlpha { rect: [1, 0, 2, 1] }.apply(&i).unwrap();
        assert_eq!([o.rgba[3], o.rgba[7], o.rgba[11], o.rgba[15], o.rgba[19]], [7, 255, 255, 7, 7]);
    }
    #[test]
    fn shrinks_average_exact_halves() {
        let src = img(4, 4, |x, y| [(x * 40) as u8, (y * 40) as u8, 100, 200]);
        let dst = img(2, 2, |_, _| [0; 4]);
        let o = ShrinkCopy { destination: [0., 0., 2., 2.], source: [0, 0, 4, 4] }.apply(&dst, &src).unwrap();
        // Pixel (0,0) averages x∈{0,1}: (0+40)/2 = 20; y likewise.
        assert_eq!(&o.rgba[0..4], &[20, 20, 100, 200]);
        assert_eq!(&o.rgba[4..8], &[100, 20, 100, 200]);
        let f = ShrinkFast { step: [2, 2] }.apply(&src).unwrap();
        assert_eq!((f.width, f.height), (2, 2));
        assert_eq!(&f.rgba[0..4], &[20, 20, 100, 255]);
        assert!(ShrinkCopy { destination: [0., 0., 5., 2.], source: [0, 0, 4, 4] }.validate().is_err());
    }
    #[test]
    fn overwrap_respects_threshold_and_clip() {
        let src = img(2, 1, |x, _| [9, 9, 9, if x == 0 { 0 } else { 128 }]);
        let dst = img(3, 1, |_, _| [1, 1, 1, 255]);
        let o = OverwrapRect { destination: [0, 0], source: [0, 0, 2, 1], clip: [0, 0, 3, 1], threshold: 1 }.apply(&dst, &src).unwrap();
        assert_eq!(&o.rgba[0..8], &[1, 1, 1, 255, 9, 9, 9, 128]);
    }
}

/// layerStwCopy `stitchWrappedCopy` (game-kr `plugin/layerStwCopy.dll`,
/// IDA: argument split 0x10002E50, rectangles 0x100028C0, rows 0x10002CB0,
/// wrapped row move 0x10002C50). `destination`/`source` are clipped
/// `[left, top, right, bottom]`; `offset` is `[requested dx - left,
/// requested dy - top]`. Destination row `i` reads source row
/// `(i + offset.y) mod height` from column `(columns[(i + offset.y) mod n] +
/// offset.x) mod width`, wrapping horizontally; all four channels move.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StitchWrappedCopy {
    pub destination: [i32; 4],
    pub source: [i32; 4],
    pub offset: [i32; 2],
    #[serde(default)]
    pub columns: Vec<i32>,
}
impl StitchWrappedCopy {
    pub fn apply(&self, destination: &ImageSurface, source: &ImageSurface) -> Result<ImageSurface> {
        destination.validate()?;
        source.validate()?;
        let [dl, dt, dr, db] = self.destination;
        let [sl, st, sr, sb] = self.source;
        ensure!(
            dl >= 0 && dt >= 0 && dr <= destination.width as i32 && db <= destination.height as i32 && dr > dl && db > dt,
            "stitchWrappedCopy destination"
        );
        ensure!(
            sl >= 0 && st >= 0 && sr <= source.width as i32 && sb <= source.height as i32 && sr > sl && sb > st,
            "stitchWrappedCopy source"
        );
        let mut out = destination.clone();
        let (dw, sw, sh) = (dr - dl, sr - sl, sb - st);
        let n = self.columns.len() as i32;
        for i in 0..db - dt {
            let r = i + self.offset[1];
            let col = if n > 0 { self.columns[r.rem_euclid(n) as usize] } else { 0 };
            let mut x = (col + self.offset[0]).rem_euclid(sw);
            let row = r.rem_euclid(sh) + st;
            let mut left = dw;
            let mut d = (((dt + i) as usize) * destination.width as usize + dl as usize) * 4;
            while left > 0 {
                let run = (sw - x).min(left);
                let s = ((row as usize) * source.width as usize + (sl + x) as usize) * 4;
                out.rgba[d..d + run as usize * 4].copy_from_slice(&source.rgba[s..s + run as usize * 4]);
                d += run as usize * 4;
                left -= run;
                x += run;
                if x >= sw {
                    x = 0;
                }
            }
        }
        Ok(out)
    }
}

/// GlitchEffect `glitchCopy`/`doGlitch` (game-kr `plugin/GlitchEffect.dll`,
/// IDA: parameters 0x10001AB0, state 0x10002420, line noise 0x10005FC0,
/// block walks 0x10005DC0/0x10002250, rows 0x100048F0, channel-split copy
/// 0x10004F40, xoroshiro128** 0x10005BA0 seeded by two splitmix64 steps
/// 0x10005C50, polar normal 0x10002980).
///
/// Each row gets a normal horizontal jitter (`noise`). Blocks of `block`
/// pixels carry three random walks (x shift, y shift, channel split): each
/// step moves with probability `per_*` by a normal of deviation `sft_*` and
/// resets to zero with probability `per_reset`. A block copies from the
/// source at the shifted position; a non-zero split `c` takes the outer
/// channels from `x ∓ 4c` in an order chosen by `c & 3`. All offsets are
/// scaled by `coef` and truncated; reads clamp to the source rectangle's
/// memory span.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlitchCopy {
    /// Clipped destination `[left, top, right, bottom]`.
    pub destination: [i32; 4],
    /// Source pixel matching the destination's top-left.
    pub source: [i32; 2],
    pub noise: f64,
    pub shift: [f64; 3],
    pub probability: [f64; 3],
    pub reset: f64,
    pub block: u32,
    pub seed: u64,
    pub coef: f64,
}
struct Xoroshiro([u64; 2]);
impl Xoroshiro {
    fn seeded(seed: u64) -> Self {
        let mut state = seed;
        let mut next = || {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };
        let a = next();
        Self([a, next()])
    }
    fn next(&mut self) -> u64 {
        let [s0, mut s1] = self.0;
        let result = s0.wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        s1 ^= s0;
        self.0 = [s0.rotate_left(24) ^ s1 ^ (s1 << 16), s1.rotate_left(37)];
        result
    }
    fn unit(&mut self) -> f64 {
        f64::from_bits((self.next() >> 12) | 0x3FF0_0000_0000_0000) - 1.
    }
}
/// Marsaglia polar normal with the spare value kept, as the native's.
#[derive(Default)]
struct Normal(Option<f64>);
impl Normal {
    fn sample(&mut self, rng: &mut Xoroshiro) -> f64 {
        if let Some(v) = self.0.take() {
            return v;
        }
        loop {
            let a = rng.unit() * 2. - 1.;
            let b = rng.unit() * 2. - 1.;
            let s = b * b + a * a;
            if s >= 1. || s == 0. {
                continue;
            }
            let f = (-2. * s.ln() / s).sqrt();
            self.0 = Some(f * a);
            return f * b;
        }
    }
}
/// The eight GlitchEffect displacement options (IDA 0x10001AB0/0x10001BC0)
/// plus the block size, shared by `glitchCopy` and the transitions.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlitchParams {
    pub noise: f64,
    /// Walk deviations: x shift, y shift, channel split.
    pub shift: [f64; 3],
    /// Walk step probabilities, in the same order.
    pub probability: [f64; 3],
    pub reset: f64,
    pub block: u32,
}
impl GlitchParams {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            [self.noise, self.reset].iter().chain(&self.shift).chain(&self.probability).all(|v| v.is_finite()),
            "glitch parameters must be finite"
        );
        ensure!((1..=4096).contains(&self.block), "glitch block size");
        Ok(())
    }
}
/// One frame's random displacement field over a `width`×`height` image:
/// per-row jitter and per-block walks, generated in the native order
/// (rows, then x, y and split walks over row-major blocks).
pub struct GlitchField {
    width: usize,
    block: usize,
    cols: usize,
    lines: Vec<f64>,
    walks: [Vec<f64>; 3],
    rng: Xoroshiro,
}
impl GlitchField {
    pub fn new(params: &GlitchParams, width: u32, height: u32, seed: u64) -> Self {
        let (w, h) = (width as usize, height as usize);
        let block = params.block.max(1) as usize;
        let (cols, rows) = (w.div_ceil(block), h.div_ceil(block));
        let mut rng = Xoroshiro::seeded(seed);
        let mut normal = Normal::default();
        let lines: Vec<f64> = (0..h).map(|_| normal.sample(&mut rng) * params.noise).collect();
        let mut walk = |p: f64, sigma: f64| -> Vec<f64> {
            let mut normal = Normal::default();
            let mut at = 0f64;
            (0..cols * rows)
                .map(|_| {
                    if p > rng.unit() {
                        at += normal.sample(&mut rng) * sigma;
                    }
                    if params.reset > rng.unit() {
                        at *= 0.;
                    }
                    at
                })
                .collect()
        };
        let walks = [
            walk(params.probability[0], params.shift[0]),
            walk(params.probability[1], params.shift[1]),
            walk(params.probability[2], params.shift[2]),
        ];
        Self { width: w, block, cols, lines, walks, rng }
    }
    pub fn block(&self) -> usize {
        self.block
    }
    pub fn columns(&self) -> usize {
        self.cols
    }
    /// TransGlitch row selection (IDA 0x10005EF0), drawn after the walks:
    /// rows in 64-row words, one draw per bit; a row switches to the new
    /// image when `t` exceeds its draw.
    pub fn row_mask(&mut self, t: f64) -> Vec<bool> {
        let rows = self.lines.len();
        let mut mask = Vec::with_capacity(rows.div_ceil(64) * 64);
        for _ in 0..rows.div_ceil(64) * 64 {
            mask.push(t > self.rng.unit());
        }
        mask.truncate(rows);
        mask
    }
    /// Truncated (x, y, split) offsets of row `v`, block column `bx`, at
    /// intensity `coef` (0x100048F0/0x10005160).
    pub fn offsets(&self, v: usize, bx: usize, coef: f64) -> [i64; 3] {
        let i = bx + self.cols * (v / self.block);
        [
            ((self.lines[v] + self.walks[0][i]) * coef) as i64,
            (self.walks[1][i] * coef) as i64,
            (self.walks[2][i] * coef) as i64,
        ]
    }
    /// Copy row `v` of a `width`-pixel block row from `source` (whose
    /// rectangle starts at `origin` and spans `rows` rows) into `out` at
    /// byte offset `dest`, with the channel-split rule of 0x10004F40.
    #[allow(clippy::too_many_arguments)]
    fn copy_row(&self, out: &mut [u8], dest: usize, source: &ImageSurface, origin: [i64; 2], rows: i64, v: usize, coef: f64) {
        let sw = source.width as i64;
        let [sx, sy] = origin;
        let w = self.width;
        // Memory span of the source rectangle, in pixels.
        let lo = sy * sw + sx;
        let hi = (sy + rows - 1) * sw + sx + w as i64;
        let mut u = 0;
        while u < w {
            let len = self.block.min(w - u);
            let [ox, oy, c] = self.offsets(v, u / self.block, coef);
            let at = |dx: i64| -> usize {
                let p = (sy + v as i64 + oy) * sw + sx + u as i64 + ox + dx;
                p.clamp(lo, (hi - len as i64).max(lo)) as usize
            };
            let (p, pm, pp) = (at(0), at(-4 * c), at(4 * c));
            let pick = glitch_pick(c);
            for k in 0..len {
                let d = dest + (u + k) * 4;
                for (n, (from, ch)) in pick.iter().enumerate() {
                    let base = [pm, p, pp][*from] + k;
                    out[d + CH[n]] = source.rgba[base * 4 + CH[*ch]];
                }
                out[d + 3] = source.rgba[(p + k) * 4 + 3];
            }
            u += len;
        }
    }
}
// Native byte order is BGRA; map its channel index onto RGBA.
const CH: [usize; 4] = [2, 1, 0, 3];
/// (minus, centre, plus) native channel picked for B, G, R by split `c`.
pub fn glitch_pick(c: i64) -> [(usize, usize); 3] {
    match (c != 0, c & 3) {
        (false, _) => [(1, 0), (1, 1), (1, 2)],
        (_, 0) => [(0, 0), (1, 1), (2, 2)],
        (_, 1) => [(0, 2), (1, 1), (2, 0)],
        (_, 2) => [(0, 1), (1, 0), (2, 2)],
        _ => [(0, 0), (1, 2), (2, 1)],
    }
}
/// FadeGlitch colour (0x10003710 then 0x100033F0): the tint channel is
/// `trunc(color·w + 128·(1-w))` for weight `w`; each pixel channel then
/// moves by twice the tint's distance from 128, saturated; alpha is kept.
pub fn glitch_tint(color: [u8; 3], weight: f64) -> [u8; 3] {
    color.map(|c| ((f64::from(c) + 0.) * weight + (1. - weight) * 128.) as i64 as u8)
}
pub fn apply_glitch_tint(pixel: [u8; 3], tint: [u8; 3]) -> [u8; 3] {
    std::array::from_fn(|i| (i32::from(pixel[i]) + 2 * (i32::from(tint[i]) - 128)).clamp(0, 255) as u8)
}
/// One frame of a GlitchEffect transition over equally sized images.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GlitchTransition {
    /// `glitch` (TransGlitch, exchange): rows switch to the new image with
    /// probability `t`; old rows glitch at `t^gamma_in`, new rows at
    /// `(1-t)^gamma_out` (0x10004DA0).
    Exchange { gamma_in: f64, gamma_out: f64 },
    /// `fadeglitch` (FadeGlitch, simple): one image glitches at
    /// `v^gamma` with `v = t` (or `1-t` when `fade_in`, which reads the
    /// transition source), tinted toward `color` by `v²` unless `nofade`.
    Fade { fade_in: bool, gamma: f64, color: Option<[u8; 3]> },
}
/// Per-row source choice and intensity, and the frame tint, of a transition.
pub struct GlitchFrame {
    pub field: GlitchField,
    /// (uses the new image, intensity) for every row.
    pub rows: Vec<(bool, f64)>,
    pub tint: Option<[u8; 3]>,
}
impl GlitchTransition {
    pub fn validate(&self) -> Result<()> {
        let gammas = match self {
            Self::Exchange { gamma_in, gamma_out } => [*gamma_in, *gamma_out],
            Self::Fade { gamma, .. } => [*gamma, 1.],
        };
        ensure!(gammas.iter().all(|g| g.is_finite()), "glitch gamma must be finite");
        Ok(())
    }
    /// The frame at phase `t` (0..1) seeded by the frame tick (0x10005730,
    /// 0x10005540: two splitmix64 steps of the tick seed the generator).
    pub fn frame(&self, params: &GlitchParams, size: [u32; 2], t: f64, seed: u64) -> GlitchFrame {
        let mut field = GlitchField::new(params, size[0], size[1], seed);
        let h = size[1] as usize;
        let (rows, tint) = match self {
            Self::Exchange { gamma_in, gamma_out } => {
                let mask = field.row_mask(t);
                let (old, new) = (t.powf(*gamma_in), (1. - t).powf(*gamma_out));
                (mask.into_iter().map(|m| (m, if m { new } else { old })).collect(), None)
            }
            Self::Fade { fade_in, gamma, color } => {
                let v = if *fade_in { 1. - t } else { t };
                let coef = v.powf(*gamma);
                (vec![(*fade_in, coef); h], color.map(|c| glitch_tint(c, v * v)))
            }
        };
        GlitchFrame { field, rows, tint }
    }
}
impl GlitchFrame {
    /// CPU reference of the frame: `old`/`new` are the equally sized inputs.
    pub fn render(&self, old: &ImageSurface, new: &ImageSurface) -> Result<ImageSurface> {
        old.validate()?;
        new.validate()?;
        ensure!(old.width == new.width && old.height == new.height, "glitch inputs differ in size");
        ensure!(self.field.width == old.width as usize && self.rows.len() == old.height as usize, "glitch field size");
        let mut out = old.clone();
        let row_bytes = old.width as usize * 4;
        for (v, (use_new, coef)) in self.rows.iter().enumerate() {
            let source = if *use_new { new } else { old };
            self.field.copy_row(&mut out.rgba, v * row_bytes, source, [0, 0], old.height as i64, v, *coef);
            if let Some(tint) = self.tint {
                for px in out.rgba[v * row_bytes..(v + 1) * row_bytes].chunks_exact_mut(4) {
                    let [b, g, r] = apply_glitch_tint([px[2], px[1], px[0]], [tint[2], tint[1], tint[0]]);
                    px[0] = r;
                    px[1] = g;
                    px[2] = b;
                }
            }
        }
        Ok(out)
    }
}
impl GlitchCopy {
    pub fn apply(&self, destination: &ImageSurface, source: &ImageSurface) -> Result<ImageSurface> {
        destination.validate()?;
        source.validate()?;
        let [l, t, r, b] = self.destination;
        ensure!(
            l >= 0 && t >= 0 && r <= destination.width as i32 && b <= destination.height as i32 && r > l && b > t,
            "glitchCopy destination"
        );
        let (w, h) = ((r - l) as usize, (b - t) as usize);
        let [sx, sy] = self.source;
        ensure!(
            sx >= 0 && sy >= 0 && sx as usize + w <= source.width as usize && sy as usize + h <= source.height as usize,
            "glitchCopy source"
        );
        let params = GlitchParams {
            noise: self.noise,
            shift: self.shift,
            probability: self.probability,
            reset: self.reset,
            block: self.block.max(1),
        };
        let field = GlitchField::new(&params, w as u32, h as u32, self.seed);
        let mut out = destination.clone();
        for v in 0..h {
            let d = ((t as usize + v) * destination.width as usize + l as usize) * 4;
            field.copy_row(&mut out.rgba, d, source, [sx as i64, sy as i64], h as i64, v, self.coef);
        }
        Ok(out)
    }
}

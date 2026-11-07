// SPDX-License-Identifier: MPL-2.0
//! Packed 8-bit-index / 24-bit-BGR planes with an 8-bit *transparency*
//! convention (0 = opaque, 255 = invisible), as used by DIB-section software
//! engines. Kernels keep the native integer rounding of each operation:
//!
//! * `blend`: `d = ((t+1)*d + (256-t)*s) >> 8` per byte;
//! * `masked_blend`: per-pixel mask `m` (transparency) and global `t`:
//!   `v = m+1` (t == 0) or `m+t+1`; `v >= 255` skips, `v <= 2` copies,
//!   otherwise `d = (v*d + (257-v)*s) >> 8`; a destination mask becomes
//!   `dm = dm*(m+1) >> 8`;
//! * `additive`: `d = min(255, d + ((255-t)*s >> 8))`;
//! * `fill_blend`: `d = (c*(257-w) + w*d) >> 8` with weight `w` in 1..=256.
//!
//! Rectangles are `[x, y, w, h]`. Callers clip with [`clip`]/[`clip_pair`],
//! which reproduce the native joint source/destination clipping.
use anyhow::{Result, ensure};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaneFormat {
    /// One palette index per pixel.
    Index8,
    /// Three bytes per pixel in B, G, R order.
    Bgr24,
}
#[derive(Clone, Debug)]
pub struct Plane {
    pub width: u32,
    pub height: u32,
    pub format: PlaneFormat,
    pub pixels: Vec<u8>,
    /// 256 B,G,R entries for `Index8`; empty for `Bgr24`.
    pub palette: Vec<[u8; 3]>,
}
pub type Rect = [i32; 4];
pub const MAX_PLANE_PIXELS: u64 = 64 * 1024 * 1024;

/// Identity grey palette (index i -> B=G=R=i), the native default for new
/// indexed pages.
pub fn grey_palette() -> Vec<[u8; 3]> {
    (0..=255u8).map(|i| [i, i, i]).collect()
}

impl Plane {
    pub fn new(width: u32, height: u32, format: PlaneFormat) -> Result<Self> {
        ensure!(width > 0 && height > 0, "plane dimensions");
        ensure!(
            u64::from(width) * u64::from(height) <= MAX_PLANE_PIXELS,
            "plane pixel budget"
        );
        let bpp = match format {
            PlaneFormat::Index8 => 1,
            PlaneFormat::Bgr24 => 3,
        };
        Ok(Self {
            width,
            height,
            format,
            pixels: vec![0; width as usize * height as usize * bpp],
            palette: if format == PlaneFormat::Index8 {
                grey_palette()
            } else {
                vec![]
            },
        })
    }
    pub fn from_pixels(
        width: u32,
        height: u32,
        format: PlaneFormat,
        pixels: Vec<u8>,
        palette: Vec<[u8; 3]>,
    ) -> Result<Self> {
        let mut plane = Self::new(width, height, format)?;
        ensure!(pixels.len() == plane.pixels.len(), "plane pixel extent");
        plane.pixels = pixels;
        if format == PlaneFormat::Index8 {
            ensure!(palette.len() == 256, "indexed plane palette extent");
            plane.palette = palette;
        }
        Ok(plane)
    }
    pub fn bpp(&self) -> usize {
        match self.format {
            PlaneFormat::Index8 => 1,
            PlaneFormat::Bgr24 => 3,
        }
    }
    pub fn stride(&self) -> usize {
        self.width as usize * self.bpp()
    }
    fn offset(&self, x: i32, y: i32) -> usize {
        y as usize * self.stride() + x as usize * self.bpp()
    }
    /// Native single-rectangle clip; None when empty.
    pub fn clip(&self, rect: Rect) -> Option<Rect> {
        clip(self.width, self.height, rect)
    }
    /// Pixel as B,G,R (palette-expanded for indexed planes).
    pub fn bgr_at(&self, x: i32, y: i32) -> Option<[u8; 3]> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        let at = self.offset(x, y);
        Some(match self.format {
            PlaneFormat::Bgr24 => [self.pixels[at], self.pixels[at + 1], self.pixels[at + 2]],
            PlaneFormat::Index8 => self.palette[self.pixels[at] as usize],
        })
    }
    pub fn index_at(&self, x: i32, y: i32) -> Option<u8> {
        if self.format != PlaneFormat::Index8
            || x < 0
            || y < 0
            || x >= self.width as i32
            || y >= self.height as i32
        {
            return None;
        }
        Some(self.pixels[self.offset(x, y)])
    }
    /// Fill with an index (indexed) or COLORREF 0x00BBGGRR (true colour).
    pub fn fill(&mut self, rect: Rect, color: u32) {
        let Some([x, y, w, h]) = self.clip(rect) else {
            return;
        };
        let bytes = match self.format {
            PlaneFormat::Index8 => vec![color as u8],
            PlaneFormat::Bgr24 => vec![(color >> 16) as u8, (color >> 8) as u8, color as u8],
        };
        let n = bytes.len();
        for row in y..y + h {
            let start = self.offset(x, row);
            for chunk in self.pixels[start..start + w as usize * n].chunks_exact_mut(n) {
                chunk.copy_from_slice(&bytes);
            }
        }
    }
    /// Weighted fill: weight 1 writes the colour, 256 keeps the pixel.
    pub fn fill_blend(&mut self, rect: Rect, color: u32, weight: u16) {
        let Some([x, y, w, h]) = self.clip(rect) else {
            return;
        };
        let weight = u32::from(weight.clamp(1, 256));
        let keep = 257 - weight;
        let bytes: Vec<u32> = match self.format {
            PlaneFormat::Index8 => vec![color & 0xff],
            PlaneFormat::Bgr24 => vec![(color >> 16) & 0xff, (color >> 8) & 0xff, color & 0xff],
        };
        let n = bytes.len();
        for row in y..y + h {
            let start = self.offset(x, row);
            for chunk in self.pixels[start..start + w as usize * n].chunks_exact_mut(n) {
                for (d, c) in chunk.iter_mut().zip(&bytes) {
                    *d = (((c * keep + weight * u32::from(*d)) & 0xffff) >> 8) as u8;
                }
            }
        }
    }
    /// Expand to straight RGBA. `mask` is an optional transparency plane of
    /// the same size (coverage = 255 - mask).
    pub fn to_rgba(&self, mask: Option<&Plane>) -> Vec<u8> {
        let n = self.width as usize * self.height as usize;
        let mut out = vec![255u8; n * 4];
        for i in 0..n {
            let [b, g, r] = match self.format {
                PlaneFormat::Bgr24 => [self.pixels[i * 3], self.pixels[i * 3 + 1], self.pixels[i * 3 + 2]],
                PlaneFormat::Index8 => self.palette[self.pixels[i] as usize],
            };
            out[i * 4] = r;
            out[i * 4 + 1] = g;
            out[i * 4 + 2] = b;
        }
        if let Some(mask) = mask.filter(|m| m.width == self.width && m.height == self.height) {
            for i in 0..n {
                out[i * 4 + 3] = 255 - mask.pixels[i * mask.bpp()];
            }
        }
        out
    }
}

/// Native single-rectangle clip against a plane extent.
pub fn clip(width: u32, height: u32, [mut x, mut y, mut w, mut h]: Rect) -> Option<Rect> {
    if x < 0 {
        w = w.saturating_add(x);
        x = 0;
    }
    if y < 0 {
        h = h.saturating_add(y);
        y = 0;
    }
    if x.saturating_add(w) > width as i32 {
        w = width as i32 - x;
    }
    if y.saturating_add(h) > height as i32 {
        h = height as i32 - y;
    }
    (w > 0 && h > 0).then_some([x, y, w, h])
}
/// Joint source/destination clip preserving pixel correspondence. Returns
/// the clipped source rectangle and destination origin.
pub fn clip_pair(
    source: [u32; 2],
    [mut sx, mut sy, mut w, mut h]: Rect,
    destination: [u32; 2],
    [mut dx, mut dy]: [i32; 2],
) -> Option<(Rect, [i32; 2])> {
    if sx < 0 {
        dx = dx.saturating_sub(sx);
        w = w.saturating_add(sx);
        sx = 0;
    }
    if sy < 0 {
        dy = dy.saturating_sub(sy);
        h = h.saturating_add(sy);
        sy = 0;
    }
    w = w.min(source[0] as i32 - sx);
    h = h.min(source[1] as i32 - sy);
    if dx < 0 {
        sx = sx.saturating_sub(dx);
        w = w.saturating_add(dx);
        dx = 0;
    }
    if dy < 0 {
        sy = sy.saturating_sub(dy);
        h = h.saturating_add(dy);
        dy = 0;
    }
    w = w.min(destination[0] as i32 - dx);
    h = h.min(destination[1] as i32 - dy);
    (w > 0 && h > 0).then_some(([sx, sy, w, h], [dx, dy]))
}
fn rows<'a>(
    src: &'a Plane,
    [sx, sy, w, h]: Rect,
    dst: &'a mut Plane,
    [dx, dy]: [i32; 2],
) -> impl Iterator<Item = (&'a [u8], &'a mut [u8])> {
    let (ss, sb) = (src.stride(), src.bpp());
    let (ds, db) = (dst.stride(), dst.bpp());
    let src_rows = src.pixels.chunks_exact(ss).skip(sy as usize).take(h as usize);
    let dst_rows = dst.pixels.chunks_exact_mut(ds).skip(dy as usize).take(h as usize);
    src_rows.zip(dst_rows).map(move |(s, d)| {
        (
            &s[sx as usize * sb..(sx + w) as usize * sb],
            &mut d[dx as usize * db..(dx + w) as usize * db],
        )
    })
}
/// Straight copy. Index->BGR expands through the source palette; BGR->index
/// has no native conversion and writes nothing. Clipping must be done.
pub fn copy(src: &Plane, rect: Rect, dst: &mut Plane, at: [i32; 2]) {
    let palette = src.palette.clone();
    match (src.format, dst.format) {
        (PlaneFormat::Index8, PlaneFormat::Bgr24) => {
            for (s, d) in rows(src, rect, dst, at) {
                for (i, px) in d.chunks_exact_mut(3).enumerate() {
                    px.copy_from_slice(&palette[s[i] as usize]);
                }
            }
        }
        (a, b) if a == b => {
            for (s, d) in rows(src, rect, dst, at) {
                d.copy_from_slice(s);
            }
        }
        _ => {}
    }
}
/// Global-transparency blend onto a BGR destination.
pub fn blend(src: &Plane, rect: Rect, dst: &mut Plane, at: [i32; 2], transparency: u8) {
    if dst.format != PlaneFormat::Bgr24 {
        return;
    }
    let keep = u32::from(transparency) + 1;
    let take = 256 - u32::from(transparency);
    let palette = src.palette.clone();
    let indexed = src.format == PlaneFormat::Index8;
    for (s, d) in rows(src, rect, dst, at) {
        for (i, px) in d.chunks_exact_mut(3).enumerate() {
            let sp = if indexed {
                palette[s[i] as usize]
            } else {
                [s[i * 3], s[i * 3 + 1], s[i * 3 + 2]]
            };
            for c in 0..3 {
                px[c] = ((keep * u32::from(px[c]) + take * u32::from(sp[c])) >> 8) as u8;
            }
        }
    }
}
/// Per-pixel transparency mask blend. `mask` has the source's dimensions;
/// `dst_mask` (destination's dimensions) is multiplied by the source mask.
pub fn masked_blend(
    src: &Plane,
    mask: &Plane,
    rect: Rect,
    dst: &mut Plane,
    dst_mask: Option<&mut Plane>,
    at: [i32; 2],
    transparency: u8,
) {
    let [sx, sy, w, h] = rect;
    if mask.width != src.width || mask.height != src.height || mask.format != PlaneFormat::Index8 {
        return;
    }
    if transparency < 255 && dst.format == PlaneFormat::Bgr24 {
        let palette = src.palette.clone();
        let indexed = src.format == PlaneFormat::Index8;
        let extra = if transparency == 0 { 1 } else { u32::from(transparency) + 1 };
        for row in 0..h {
            let mrow = &mask.pixels[(sy + row) as usize * mask.stride()..];
            let srow = &src.pixels[(sy + row) as usize * src.stride()..];
            let dstride = dst.stride();
            let dstart = (at[1] + row) as usize * dstride;
            let drow = &mut dst.pixels[dstart..dstart + dstride];
            for col in 0..w {
                let v = u32::from(mrow[(sx + col) as usize]) + extra;
                if v >= 255 {
                    continue;
                }
                let x = (sx + col) as usize;
                let sp = if indexed {
                    palette[srow[x] as usize]
                } else {
                    [srow[x * 3], srow[x * 3 + 1], srow[x * 3 + 2]]
                };
                let d = &mut drow[(at[0] + col) as usize * 3..(at[0] + col) as usize * 3 + 3];
                if v <= 2 {
                    d.copy_from_slice(&sp);
                } else {
                    for c in 0..3 {
                        d[c] = (((v * u32::from(d[c]) + (257 - v) * u32::from(sp[c])) & 0xffff) >> 8) as u8;
                    }
                }
            }
        }
    }
    if let Some(dm) = dst_mask {
        if dm.format != PlaneFormat::Index8 || dm.width != dst.width || dm.height != dst.height {
            return;
        }
        for row in 0..h {
            let mrow = &mask.pixels[(sy + row) as usize * mask.stride()..];
            let dstart = (at[1] + row) as usize * dm.stride();
            for col in 0..w {
                let d = &mut dm.pixels[dstart + (at[0] + col) as usize];
                *d = ((u32::from(*d) * (u32::from(mrow[(sx + col) as usize]) + 1)) >> 8) as u8;
            }
        }
    }
}
/// Saturating additive draw of BGR pixels.
pub fn additive(src: &Plane, rect: Rect, dst: &mut Plane, at: [i32; 2], transparency: u8) {
    if src.format != PlaneFormat::Bgr24 || dst.format != PlaneFormat::Bgr24 {
        return;
    }
    let weight = 255 - u32::from(transparency);
    for (s, d) in rows(src, rect, dst, at) {
        for (d, s) in d.iter_mut().zip(s) {
            *d = (u32::from(*d) + ((weight * u32::from(*s)) >> 8)).min(255) as u8;
        }
    }
}
/// Exchange two equally formatted regions (rectangles already clipped).
pub fn swap(a: &mut Plane, rect: Rect, b: &mut Plane, at: [i32; 2]) {
    if a.format != b.format {
        return;
    }
    let [sx, sy, w, h] = rect;
    let n = a.bpp();
    for row in 0..h {
        let ar = (sy + row) as usize * a.stride() + sx as usize * n;
        let br = (at[1] + row) as usize * b.stride() + at[0] as usize * n;
        a.pixels[ar..ar + w as usize * n].swap_with_slice(&mut b.pixels[br..br + w as usize * n]);
    }
}
/// Nearest-neighbour stretch with native-style integer stepping. Index->index
/// copies indices; index->BGR expands; BGR->index writes nothing.
pub fn stretch(src: &Plane, src_rect: Rect, dst: &mut Plane, dst_rect: Rect) {
    let [sx, sy, sw, sh] = src_rect;
    let [dx, dy, dw, dh] = dst_rect;
    if sw <= 0 || sh <= 0 || dw <= 0 || dh <= 0 {
        return;
    }
    if src.format == PlaneFormat::Bgr24 && dst.format == PlaneFormat::Index8 {
        return;
    }
    let Some([cx, cy, cw, ch]) = dst.clip(dst_rect) else {
        return;
    };
    let palette = src.palette.clone();
    for y in cy..cy + ch {
        let py = sy as i64 + (i64::from(y - dy) * i64::from(sh)) / i64::from(dh);
        if py < 0 || py >= i64::from(src.height) {
            continue;
        }
        for x in cx..cx + cw {
            let px = sx as i64 + (i64::from(x - dx) * i64::from(sw)) / i64::from(dw);
            if px < 0 || px >= i64::from(src.width) {
                continue;
            }
            let s = src.offset(px as i32, py as i32);
            let d = dst.offset(x, y);
            match (src.format, dst.format) {
                (PlaneFormat::Index8, PlaneFormat::Index8) => dst.pixels[d] = src.pixels[s],
                (PlaneFormat::Index8, PlaneFormat::Bgr24) => {
                    dst.pixels[d..d + 3].copy_from_slice(&palette[src.pixels[s] as usize])
                }
                _ => {
                    let p = [src.pixels[s], src.pixels[s + 1], src.pixels[s + 2]];
                    dst.pixels[d..d + 3].copy_from_slice(&p);
                }
            }
        }
    }
}
/// Two-colour luminance ramp (sepia): `l = B + 2*(R + 2*G) + 1`, each
/// channel `(dark*(2049-l) + light*l) / 2048` with COLORREF endpoints, then an
/// optional retain weight `w` in 1..=256: `(w*old + (257-w)*new) / 256`.
/// Indexed planes recolour their palette instead of pixels.
pub fn duotone(plane: &mut Plane, rect: Rect, dark: u32, light: u32, weight: Option<u16>) {
    let ramp = |px: &mut [u8]| {
        let l = u32::from(px[0]) + 2 * (u32::from(px[2]) + 2 * u32::from(px[1])) + 1;
        let channel = |shift: u32| -> u32 {
            (((dark >> shift) & 255) * (2049 - l) + ((light >> shift) & 255) * l) / 2048
        };
        // B,G,R bytes take COLORREF bytes 2,1,0.
        let new = [channel(16), channel(8), channel(0)];
        for c in 0..3 {
            px[c] = match weight {
                Some(w) => {
                    let w = u32::from(w.clamp(1, 256));
                    ((w * u32::from(px[c]) + (257 - w) * new[c]) / 256) as u8
                }
                None => new[c] as u8,
            };
        }
    };
    match plane.format {
        PlaneFormat::Index8 => {
            for entry in &mut plane.palette {
                ramp(entry);
            }
        }
        PlaneFormat::Bgr24 => {
            let Some([x, y, w, h]) = plane.clip(rect) else {
                return;
            };
            for row in y..y + h {
                let start = plane.offset(x, row);
                for px in plane.pixels[start..start + w as usize * 3].chunks_exact_mut(3) {
                    ramp(px);
                }
            }
        }
    }
}
/// Square mosaic: each `size` block takes its top-left pixel.
pub fn mosaic(plane: &mut Plane, rect: Rect, size: i32) {
    if size <= 1 {
        return;
    }
    let Some([x, y, w, h]) = plane.clip(rect) else {
        return;
    };
    let n = plane.bpp();
    let mut by = y;
    while by < y + h {
        let mut bx = x;
        while bx < x + w {
            let s = plane.offset(bx, by);
            let sample = plane.pixels[s..s + n].to_vec();
            for yy in by..(by + size).min(y + h) {
                for xx in bx..(bx + size).min(x + w) {
                    let d = plane.offset(xx, yy);
                    plane.pixels[d..d + n].copy_from_slice(&sample);
                }
            }
            bx += size;
        }
        by += size;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blend_and_mask_rounding() {
        let mut dst = Plane::new(2, 1, PlaneFormat::Bgr24).unwrap();
        dst.pixels = vec![100, 100, 100, 0, 0, 0];
        let mut src = Plane::new(2, 1, PlaneFormat::Bgr24).unwrap();
        src.pixels = vec![200; 6];
        blend(&src, [0, 0, 2, 1], &mut dst, [0, 0], 0);
        assert_eq!(dst.pixels[0], ((100 + 256 * 200) >> 8) as u8);
        let mut mask = Plane::new(2, 1, PlaneFormat::Index8).unwrap();
        mask.pixels = vec![0, 255];
        let mut out = Plane::new(2, 1, PlaneFormat::Bgr24).unwrap();
        masked_blend(&src, &mask, [0, 0, 2, 1], &mut out, None, [0, 0], 0);
        assert_eq!(out.pixels, vec![200, 200, 200, 0, 0, 0]);
    }
    #[test]
    fn joint_clip_keeps_correspondence() {
        let (r, at) = clip_pair([10, 10], [-2, 0, 5, 5], [4, 4], [0, 1]).unwrap();
        assert_eq!((r, at), ([0, 0, 2, 3], [2, 1]));
    }
}

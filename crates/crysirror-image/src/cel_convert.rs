// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Load-time picture conversions of codeX RScript image layers, recovered
//! from the Shapeshifter build (CMainCelLim loader 0x415810).
//!
//! In loader order: an optional mirror (bit 0 horizontal, bit 1 vertical,
//! sub_462710), an optional rotation in degrees with a zoom percentage
//! (sub_4622B0: point sampling in 16.16 steps through the 256-entry sine
//! and cosine tables scaled by 65535, CMath::Init; without rotation the
//! percentage sampler of sub_462870), then one tone conversion:
//! 1 RGB set to black (sub_4458A0(0)), 2 RGB inverted (sub_445670),
//! 3 the integer average of the three channels (sub_445550), 4..6 a tint
//! of the average through the emboss table (sub_445380 with 0x8B4513,
//! 0x483D8B, 0xFF8000), 7 and 11 the night conversion (CGrapConvNight
//! 0x461C90 with 0x181020, 0x904030), 8..10, 12, 13 the per-channel
//! emboss tint (0x461A70 with 0x282040, 0x383080, 0x802D50, 0x805030,
//! 0xA06040) and 15 the hue lift around blue (CGrapConvPower 0x461E50).
//!
//! The emboss table of level `l` (CMath::GetEmbosTable) is, for a channel
//! value `c`, `c·(2l+2)/256` below 128 and `c + (256-c)(2l-255)/256` from
//! 128 (the darkening and lightening tables of CMath::Init).
use crate::ImageSurface;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CelConvert {
    /// Bit 0 mirrors horizontally, bit 1 vertically.
    #[serde(default)]
    pub mirror: u8,
    /// Rotation in degrees (0 none).
    #[serde(default)]
    pub degrees: u16,
    /// Zoom percentage (0 or 100 none).
    #[serde(default)]
    pub percent: u16,
    /// Tone conversion number (0 none).
    #[serde(default)]
    pub tone: u16,
}

/// CMath darkening table: `c · (256 - a) / 256`.
fn darken(a: u32, c: u32) -> u8 {
    ((256 - a) as f64 * c as f64 * 0.00390625) as u8
}
/// CMath lightening table: `(256 - c) · a / 256 + c`.
fn lighten(a: u32, c: u32) -> u8 {
    ((256 - c) as f64 * a as f64 * 0.00390625 + c as f64) as u8
}
/// CMath::GetEmbosTable(level)[c].
pub fn emboss(level: u8, c: u8) -> u8 {
    let l = u32::from(level);
    if l < 128 { darken(254 - 2 * l, u32::from(c)) } else { lighten(2 * l - 255, u32::from(c)) }
}

fn tables() -> ([i32; 256], [i32; 256]) {
    let mut sin = [0; 256];
    let mut cos = [0; 256];
    for i in 0..256 {
        let a = i as f64 * 0.00390625 * 6.283185308;
        sin[i] = (a.sin() * 65535.0) as i32;
        cos[i] = (a.cos() * 65535.0) as i32;
    }
    (sin, cos)
}

fn mirror(src: &ImageSurface, bits: u8) -> ImageSurface {
    let (w, h) = (src.width as usize, src.height as usize);
    let mut out = vec![0; src.rgba.len()];
    for y in 0..h {
        for x in 0..w {
            let sx = if bits & 1 != 0 { w - 1 - x } else { x };
            let sy = if bits & 2 != 0 { h - 1 - y } else { y };
            out[(y * w + x) * 4..][..4].copy_from_slice(&src.rgba[(sy * w + sx) * 4..][..4]);
        }
    }
    ImageSurface { width: src.width, height: src.height, rgba: out }
}

/// sub_4622B0 with an angle: the bounding box of the turned picture, each
/// pixel sampled backwards; outside the source it is transparent.
fn rotate(src: &ImageSurface, degrees: u16, percent: u16) -> ImageSurface {
    let z = if percent == 0 { 100 } else { i64::from(percent) };
    let (w, h) = (i64::from(src.width), i64::from(src.height));
    let index = ((u32::from(degrees) % 360) << 8) / 360;
    let (sin, cos) = tables();
    let (s, c) = (i64::from(sin[index as usize]), i64::from(cos[index as usize]));
    let out_w = (z * (((w * c).abs() + (h * s).abs()) / 0xFFFF) / 100).max(0);
    let out_h = (z * (((w * s).abs() + (h * c).abs()) / 0xFFFF) / 100).max(0);
    let (sz, cz) = (s * z / 100, c * z / 100);
    let (ds, dc) = (100 * s / z, 100 * c / z);
    let div = |v: i64| v / 0xFFFF;
    let (ox, oy) = match index >> 6 {
        0 => (div(w * sz), 0),
        1 => (div(w * sz - h * cz), -div(w * cz)),
        2 => (-div(h * cz), -div(w * cz + h * sz)),
        _ => (0, -div(h * sz)),
    };
    let mut out = Vec::with_capacity((out_w * out_h * 4) as usize);
    let (mut row_x, mut row_y) = (-(dc * ox), -(ds * ox));
    for _ in 0..out_h {
        let mut v = row_x - ds * oy;
        let mut u = -(row_y + dc * oy);
        for _ in 0..out_w {
            let (sx, sy) = (u >> 16, v >> 16);
            if sx < 0 || sy < 0 || sx >= w || sy >= h {
                out.extend([0, 0, 0, 0]);
            } else {
                out.extend_from_slice(&src.rgba[((sy * w + sx) * 4) as usize..][..4]);
            }
            u += dc;
            v += ds;
        }
        row_y += ds;
        row_x += dc;
    }
    ImageSurface { width: out_w as u32, height: out_h as u32, rgba: out }
}

fn hue_channel(h: f32, p: f32, q: f32) -> f32 {
    let h = if h > 360.0 { h - 360.0 } else if h < 0.0 { h + 360.0 } else { h };
    if h < 60.0 {
        p + (q - p) * h / 60.0
    } else if h < 180.0 {
        q
    } else if h < 240.0 {
        p + (q - p) * (240.0 - h) / 60.0
    } else {
        p
    }
}

/// CGrapConvPower: HSL with hue in bytes, hues within 41 of 168 lifted.
fn power(p: &mut [u8]) {
    let (b, g, r) = (p[0], p[1], p[2]);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let sum = u32::from(max) + u32::from(min);
    let light = (f64::from(255 * sum) / 510.0 + 0.5) as i32 as u8;
    let (mut hue, sat) = if max == min {
        (0u8, 0u8)
    } else {
        let d = f32::from(max - min);
        let s = if sum > 255 { 510 - sum } else { sum } as f32;
        let sat = ((d / s) * 255.0 + 0.5) as i32 as u8;
        let h = if r == max {
            (f32::from(g) - f32::from(b)) / d
        } else if g == max {
            (f32::from(b) - f32::from(r)) / d + 2.0
        } else {
            (f32::from(r) - f32::from(g)) / d + 4.0
        };
        ((h * 42.0 + 0.5) as i32 as u8, sat)
    };
    let d = i32::from(hue) - 168;
    if (d + 41) as u32 <= 82 {
        hue = (120 * (1764 - d * d) / 1764 + i32::from(hue)) as u8;
    }
    let (s, l) = (f32::from(sat) / 255.0, f32::from(light) / 255.0);
    if s == 0.0 {
        let v = (l * 255.0 + 0.5) as i32 as u8;
        p[..3].fill(v);
        return;
    }
    let q = if light >= 128 { l + s - l * s } else { l * s + l };
    let pp = 2.0 * l - q;
    let h = f32::from(hue) * 360.0 / 255.0;
    p[2] = (hue_channel(h + 120.0, pp, q) * 255.0 + 0.5) as i32 as u8;
    p[1] = (hue_channel(h, pp, q) * 255.0 + 0.5) as i32 as u8;
    p[0] = (hue_channel(h - 120.0, pp, q) * 255.0 + 0.5) as i32 as u8;
}

fn tone(rgba: &mut [u8], tone: u16) {
    let tint = |color: u32| [(color >> 16) as u8, (color >> 8) as u8, color as u8];
    for px in rgba.chunks_exact_mut(4) {
        let [r, g, b] = [px[0], px[1], px[2]];
        let avg = ((u32::from(r) + u32::from(g) + u32::from(b)) / 3) as u8;
        match tone {
            1 => px[..3].fill(0),
            2 => {
                for c in &mut px[..3] {
                    *c = !*c;
                }
            }
            3 => px[..3].fill(avg),
            4..=6 => {
                let [tr, tg, tb] = tint([0x8B_4513, 0x48_3D8B, 0xFF_8000][usize::from(tone - 4)]);
                px[0] = emboss(avg, tr);
                px[1] = emboss(avg, tg);
                px[2] = emboss(avg, tb);
            }
            7 | 11 => {
                let [lr, lg, lb] = tint(if tone == 7 { 0x18_1020 } else { 0x90_4030 });
                let m = u32::from(r.min(g).min(b));
                for (c, level) in [(0, lr), (1, lg), (2, lb)] {
                    let tinted = u32::from(emboss(level, px[c]));
                    px[c] = darken(m, u32::from(px[c])).wrapping_add(darken(255 - m, tinted));
                }
            }
            8..=10 | 12 | 13 => {
                let color = match tone {
                    8 => 0x28_2040,
                    9 => 0x38_3080,
                    10 => 0x80_2D50,
                    12 => 0x80_5030,
                    _ => 0xA0_6040,
                };
                let [lr, lg, lb] = tint(color);
                px[0] = emboss(lr, r);
                px[1] = emboss(lg, g);
                px[2] = emboss(lb, b);
            }
            15 => {
                let mut bgr = [b, g, r];
                power(&mut bgr);
                px[0] = bgr[2];
                px[1] = bgr[1];
                px[2] = bgr[0];
            }
            _ => {}
        }
    }
}

impl CelConvert {
    /// The picture size after the conversion (the rotation's bounding box
    /// or the zoom percentage).
    pub fn output_size(&self, [w, h]: [u32; 2]) -> [u32; 2] {
        if self.degrees != 0 {
            let z = if self.percent == 0 { 100 } else { i64::from(self.percent) };
            let index = ((u32::from(self.degrees) % 360) << 8) / 360;
            let (sin, cos) = tables();
            let (s, c) = (i64::from(sin[index as usize]), i64::from(cos[index as usize]));
            let (w, h) = (i64::from(w), i64::from(h));
            let ow = (z * (((w * c).abs() + (h * s).abs()) / 0xFFFF) / 100).max(0);
            let oh = (z * (((w * s).abs() + (h * c).abs()) / 0xFFFF) / 100).max(0);
            [ow as u32, oh as u32]
        } else if self.percent != 0 && self.percent != 100 {
            let p = u32::from(self.percent);
            [p * w / 100, p * h / 100]
        } else {
            [w, h]
        }
    }
    pub fn apply(&self, input: &ImageSurface, check: &dyn Fn() -> Result<()>) -> Result<ImageSurface> {
        input.validate()?;
        ensure!(self.tone <= 15, "unknown RScript tone conversion {}", self.tone);
        let mut img = if self.mirror & 3 != 0 { mirror(input, self.mirror) } else { input.clone() };
        check()?;
        if self.degrees != 0 {
            img = rotate(&img, self.degrees, self.percent);
        } else if self.percent != 0 && self.percent != 100 {
            let (w, h, rgba) = crate::wcg_encode::percent_shrink(img.width, img.height, &img.rgba, self.percent, self.percent)?;
            img = ImageSurface { width: w, height: h, rgba };
        }
        check()?;
        tone(&mut img.rgba, self.tone);
        Ok(img)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn one(rgba: [u8; 4], tone_id: u16) -> [u8; 4] {
        let img = ImageSurface { width: 1, height: 1, rgba: rgba.to_vec() };
        let out = CelConvert { mirror: 0, degrees: 0, percent: 0, tone: tone_id }.apply(&img, &|| Ok(())).unwrap();
        out.rgba.try_into().unwrap()
    }
    #[test]
    fn tones_follow_the_native_tables() {
        assert_eq!(one([10, 20, 30, 200], 1), [0, 0, 0, 200]);
        assert_eq!(one([10, 20, 30, 200], 2), [245, 235, 225, 200]);
        assert_eq!(one([10, 20, 31, 255], 3), [20, 20, 20, 255]);
        // Level 0 darkens to 2/256; level 255 lifts black to 255.
        assert_eq!(emboss(0, 128), 1);
        assert_eq!(emboss(127, 200), 200);
        assert_eq!(emboss(255, 0), 255);
        // A grey picture keeps its hue-less value through the lift.
        assert_eq!(one([90, 90, 90, 255], 15), [90, 90, 90, 255]);
    }
    #[test]
    fn mirror_and_quarter_turn() {
        let img = ImageSurface { width: 2, height: 1, rgba: vec![1, 0, 0, 255, 2, 0, 0, 255] };
        let m = CelConvert { mirror: 1, degrees: 0, percent: 0, tone: 0 }.apply(&img, &|| Ok(())).unwrap();
        assert_eq!((m.rgba[0], m.rgba[4]), (2, 1));
        let spec = CelConvert { mirror: 0, degrees: 90, percent: 100, tone: 0 };
        let r = spec.apply(&img, &|| Ok(())).unwrap();
        assert_eq!((r.width, r.height), (1, 2));
        assert_eq!(spec.output_size([2, 1]), [1, 2]);
    }
}

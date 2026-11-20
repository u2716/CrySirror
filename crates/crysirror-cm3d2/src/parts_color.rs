// SPDX-License-Identifier: MPL-2.0
//! `MaidParts` parts colours (無限色) and the `_MultiColTex` colour table.
//!
//! `UTY.UpdateColorTableTexture` clamps the colour into `ColorEdit.Status`
//! (hue/chroma 0..255, brightness 0..510, contrast 0..200, shadow rate
//! 0..255), `ColorEdit.DllHslSet` recentres brightness/contrast, and the
//! native `DLL_Color_WriteColorTableToTexture`/`…ShadowTableToTexture`
//! (`cm3d2_x64.dll` 0x18000CEC0/0x18000D240) fill 256 RGBA texels: a
//! lightness ramp `(i − 128)·contrast + brightness`, converted from HSL; with
//! a shadow rate, lightness and colour run from the shadow colour to the
//! main colour over the first `rate` entries. Alpha is not written by the
//! native code; it is 255 here (the shaders read RGB only).
use anyhow::{ensure, Result};
use crysirror_asset::interchange::cm3d2::Reader;

/// `MaidParts.PARTS_COLOR` names, in enum order.
pub const IDS: [&str; 7] = ["EYE_L", "EYE_R", "HAIR", "EYE_BROW", "UNDER_HAIR", "SKIN", "NIPPLE"];

pub fn id(name: &str) -> Option<usize> {
    IDS.iter().position(|n| n.eq_ignore_ascii_case(name))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PartsColor {
    pub used: bool,
    pub main_hue: i32,
    pub main_chroma: i32,
    pub main_brightness: i32,
    pub main_contrast: i32,
    pub shadow_rate: i32,
    pub shadow_hue: i32,
    pub shadow_chroma: i32,
    pub shadow_brightness: i32,
    pub shadow_contrast: i32,
}

/// `MaidParts.m_aryPartsColor` initial values.
pub fn defaults() -> [PartsColor; 7] {
    let main = |h, c, b, k| PartsColor { main_hue: h, main_chroma: c, main_brightness: b, main_contrast: k, ..Default::default() };
    [main(6, 117, 179, 94), main(6, 117, 179, 94), main(69, 186, 270, 94), main(0, 0, 183, 0), PartsColor::default(), main(18, 149, 247, 100), main(18, 149, 247, 100)]
}

/// `MaidParts.DeserializePre` (`CM3D2_MULTI_COL`).
pub fn deserialize(r: &mut Reader) -> Result<[PartsColor; 7]> {
    let _version = r.header("CM3D2_MULTI_COL")?;
    let n = r.i32()?;
    ensure!(n == 7, "parts colour count {n}");
    let mut out = [PartsColor::default(); 7];
    for c in &mut out {
        c.used = r.bool()?;
        c.main_hue = r.i32()?;
        c.main_chroma = r.i32()?;
        c.main_brightness = r.i32()?;
        c.main_contrast = r.i32()?;
        c.shadow_rate = r.i32()?;
        c.shadow_hue = r.i32()?;
        c.shadow_chroma = r.i32()?;
        c.shadow_brightness = r.i32()?;
        c.shadow_contrast = r.i32()?;
    }
    Ok(out)
}

/// `DLLColorPalette.DllHslParam` after `DllHslSet`.
#[derive(Clone, Copy)]
struct Hsl {
    brightness: i32,
    contrast: i32,
    hue: i32,
    sat: i32,
}

impl Hsl {
    fn new(hue: i32, sat: i32, brightness: i32, contrast: i32) -> Self {
        Self { brightness: brightness.clamp(0, 510) - 255, contrast: contrast.clamp(0, 200) - 100, hue: hue.clamp(0, 255), sat: sat.clamp(0, 255) }
    }
    /// Clamped lightness byte of entry `i`.
    fn lightness(self, i: i32) -> i32 {
        let v = ((i - 128) as f32 * ((self.contrast + 100) as f32 / 100.) + (self.brightness + 128) as f32) as i32;
        if v >= 255 {
            255
        } else {
            v.max(0)
        }
    }
    fn rgb(self, l: f32) -> [u8; 3] {
        let s = self.sat as f32 / 255.;
        if s == 0. {
            let v = (l * 255.) as i32 as u8;
            return [v; 3];
        }
        let h = self.hue as f32 * 360. / 255.;
        let q = if l > 0.5 { l + s - l * s } else { (s + 1.) * l };
        let p = l * 2. - q;
        let channel = |t: f32| {
            let t = if t > 360. { t - 360. } else if t < 0. { t + 360. } else { t };
            let v = if t < 60. {
                (q - p) * t / 60. + p
            } else if t < 180. {
                q
            } else if t < 240. {
                (q - p) * (240. - t) / 60. + p
            } else {
                p
            };
            (v * 255.) as i32 as u8
        };
        [channel(h + 120.), channel(h), channel(h - 120.)]
    }
}

/// The `_MultiColTex` texels of `c` (`UTY.UpdateColorTableTexture`).
pub fn table(c: &PartsColor) -> [[u8; 4]; 256] {
    let main = Hsl::new(c.main_hue, c.main_chroma, c.main_brightness, c.main_contrast);
    let shadow = Hsl::new(c.shadow_hue, c.shadow_chroma, c.shadow_brightness, c.shadow_contrast);
    let rate = c.shadow_rate.clamp(0, 255);
    let mut out = [[0, 0, 0, 255]; 256];
    for (i, texel) in out.iter_mut().enumerate() {
        let i = i as i32;
        if rate <= 0 {
            let l = main.lightness(i) as u8;
            let [r, g, b] = main.rgb(l as f32 / 255.);
            *texel = [r, g, b, 255];
            continue;
        }
        let t = i.min(rate) as f32 / rate as f32;
        let (ln, ls) = (main.lightness(i), shadow.lightness(i));
        let l = (ls + ((ln - ls) as f32 * t) as i32) as u8;
        let (n, s) = (main.rgb(l as f32 / 255.), shadow.rgb(l as f32 / 255.));
        let mix = |k: usize| (s[k] as i32 + ((n[k] as i32 - s[k] as i32) as f32 * t) as i32) as u8;
        *texel = [mix(0), mix(1), mix(2), 255];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grey_ramp_without_chroma() {
        // Brightness 255 / contrast 100 → identity lightness, sat 0 → grey.
        let c = PartsColor { main_brightness: 255, main_contrast: 100, ..Default::default() };
        let t = table(&c);
        assert_eq!(t[0], [0, 0, 0, 255]);
        assert_eq!(t[128], [128, 128, 128, 255]);
        assert_eq!(t[255], [255, 255, 255, 255]);
    }

    #[test]
    fn hue_zero_is_red_at_mid_lightness() {
        let c = PartsColor { main_brightness: 255, main_contrast: 100, main_chroma: 255, ..Default::default() };
        let t = table(&c);
        // l = 128/255, s = 1 → q ≈ 1, p ≈ 0: R full, G/B empty.
        assert!(t[128][0] >= 254 && t[128][1] <= 1 && t[128][2] <= 1, "{:?}", t[128]);
    }

    #[test]
    fn shadow_rate_blends_from_shadow_colour() {
        let c = PartsColor { main_brightness: 255, main_contrast: 100, shadow_rate: 100, shadow_brightness: 0, shadow_contrast: 100, ..Default::default() };
        let t = table(&c);
        assert_eq!(t[0], [0, 0, 0, 255]);
        assert_eq!(t[200], [200, 200, 200, 255]);
    }
}

// SPDX-License-Identifier: MPL-2.0
//! Engine UI chrome themes compiled from style/ui.css (see build.rs) and the
//! rounded-panel helper they are drawn with. The host reports the system
//! appearance; nothing here is retained per client or serialized.
use crate::layer::{Paint, Primitive, rectangle};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub backdrop: [u8; 4],
    pub shadow: [u8; 4],
    pub panel: [u8; 4],
    pub panel_border: [u8; 4],
    pub title: [u8; 4],
    pub text: [u8; 4],
    pub disabled: [u8; 4],
    pub control: [u8; 4],
    pub control_text: [u8; 4],
    pub accent: [u8; 4],
    pub accent_text: [u8; 4],
    pub menu: [u8; 4],
    pub separator: [u8; 4],
    pub radius: f32,
    pub control_radius: f32,
    pub menu_radius: f32,
}

include!(concat!(env!("OUT_DIR"), "/ui_theme.rs"));

static DARK_MODE: AtomicBool = AtomicBool::new(false);

/// Host-reported system appearance (desktop: winit `Theme`).
pub fn set_dark(dark: bool) {
    DARK_MODE.store(dark, Ordering::Relaxed);
}

pub fn current() -> &'static Theme {
    if DARK_MODE.load(Ordering::Relaxed) { &DARK } else { &LIGHT }
}

/// Horizontal bands per corner. Each band is inset to the circle at its
/// middle row, so the outline steps no more than about r/BANDS pixels.
const BANDS: usize = 4;
/// Primitive `sub` slots one [`rounded`] call uses.
pub const ROUNDED_SUBS: u8 = (2 * BANDS + 1) as u8;

/// A filled rounded rectangle as `ROUNDED_SUBS` solid bands, using
/// `sub..sub + ROUNDED_SUBS`.
#[allow(clippy::too_many_arguments)]
pub fn rounded(
    owner: u32,
    id: u32,
    sub: u8,
    rect: [f32; 4],
    radius: f32,
    color: [u8; 4],
    clip: Option<[f32; 4]>,
    order: [i32; 2],
) -> Vec<Primitive> {
    let [x, y, w, h] = rect;
    let r = radius.min(w / 2.).min(h / 2.).max(0.);
    let band = r / BANDS as f32;
    let paint = || Paint::Solid { color };
    let mut out = Vec::with_capacity(ROUNDED_SUBS as usize);
    out.push(rectangle(owner, id, sub, [x, y + r, w, (h - 2. * r).max(1.)], paint(), clip, order));
    for i in 0..BANDS {
        // Distance from the circle centre to the middle of this band.
        let dy = r - (i as f32 + 0.5) * band;
        let inset = r - (r * r - dy * dy).max(0.).sqrt();
        let bw = (w - 2. * inset).max(1.);
        let top = y + i as f32 * band;
        let bottom = y + h - (i as f32 + 1.) * band;
        let s = sub + 1 + 2 * i as u8;
        out.push(rectangle(owner, id, s, [x + inset, top, bw, band.max(1.)], paint(), clip, order));
        out.push(rectangle(owner, id, s + 1, [x + inset, bottom, bw, band.max(1.)], paint(), clip, order));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stylesheet_compiles_to_distinct_themes() {
        assert_ne!(LIGHT.panel, DARK.panel);
        assert_eq!(LIGHT.accent_text, [255, 255, 255, 255]);
        assert_eq!(LIGHT.radius, 12.);
    }
}

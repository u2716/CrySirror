// SPDX-License-Identifier: MPL-2.0
//! The game library launcher shared by every host: desktop, web (wasm),
//! PS Vita and Switch.
//!
//! The launcher is an authored CrySirror project (`library/launcher.js`) run
//! by the shared runtime and renderer, so each platform draws it through its
//! own backend (wgpu, WebGPU, GXM, deko3d) and nothing is CPU-composed into
//! a screen image. [`build`] produces the whole project as files (manifest,
//! script, game list and the small decorative images: backdrop, rounded
//! cards with cover art, focus glow, button glyphs); hosts write them to disk
//! or hand them to their in-memory file system, run the project, and read
//! the chosen entry with [`choice_from`] after the script quits.
use crate::cover::{Cover, CoverKind};
use anyhow::Result;
use image::{Rgba, RgbaImage};
use serde_json::json;
use std::path::{Path, PathBuf};

/// The launcher script.
pub const SCRIPT: &str = include_str!("../library/launcher.js");
/// Save file the script writes: `{"game": "<entry id>"}`.
pub const CHOICE_FILE: &str = "launch.json";
/// Project client and game names (the save directory follows them).
pub const CLIENT: &str = "launcher";

/// One library entry.
#[derive(Clone, Debug)]
pub struct Entry {
    /// Stable id returned by [`choice_from`].
    pub id: String,
    /// Display name.
    pub name: String,
    /// Secondary line (folder name, kind).
    pub detail: String,
    pub cover: Option<Cover>,
}

impl Entry {
    /// An installed game folder: id and detail are the folder name, the
    /// name is the folder name with underscores as spaces, and the cover is
    /// found by [`crate::cover::resolve`] (cached under `cache` when given,
    /// since that can mean reading the game's executable).
    pub fn folder(dir: &Path, cache: Option<&Path>) -> Self {
        let id = dir.file_name().map_or_else(|| dir.display().to_string(), |n| n.to_string_lossy().into_owned());
        let cover = match cache {
            Some(cache) => cached_cover(dir, &id, cache),
            None => crate::cover::resolve(dir),
        };
        Self { name: id.replace('_', " "), detail: id.clone(), id, cover }
    }
}

/// Platform presentation details.
pub struct Style {
    /// Label drawn on the confirm button glyph ("A", "Enter"); empty when
    /// the glyph image carries the symbol (the PS Vita cross).
    pub confirm: &'static str,
    /// Draw a cross into the confirm glyph (PS Vita).
    pub cross: bool,
    /// Shown in the empty state: where games are added.
    pub add_hint: String,
}

/// The launcher project for `entries` at `display` size, as (path, bytes)
/// pairs relative to the project root. `font` is the face the launcher
/// draws with; text it cannot render is reduced to the characters it
/// covers, since a missing glyph is an error in the engine.
pub fn build(entries: &[Entry], display: (u32, u32), font: Option<&[u8]>, style: &Style) -> Result<Vec<(String, Vec<u8>)>> {
    let (w, h) = display;
    let scale = w as f32 / 960.;
    let card = ((w as f32 * 0.17).round() as u32, (w as f32 * 0.17 * 1.3).round() as u32);
    let text = font.map(coverage).transpose()?;
    let covered = |s: &str| -> String {
        match &text {
            Some(t) => s.chars().filter(|c| t.measure_simple("default", 16., &c.to_string()).is_ok()).collect(),
            None => s.to_owned(),
        }
    };
    let mut files = Vec::new();
    let mut list = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        let mut name = covered(&entry.name);
        if name.trim().is_empty() {
            name = format!("Game {}", i + 1);
        }
        let initials: String = match entry.cover {
            Some(_) => String::new(),
            None => name
                .split(|c: char| c.is_whitespace() || c == '-')
                .filter_map(|word| word.chars().next())
                .take(2)
                .flat_map(char::to_uppercase)
                .collect(),
        };
        list.push(json!({"id": entry.id, "name": name, "folder": covered(&entry.detail), "initials": initials}));
        let image = match &entry.cover {
            Some(cover) => cover_card(card, cover),
            None => card_image(card, hue(&entry.id)),
        };
        files.push((format!("assets/card-{i}.png"), png(&image)?));
    }
    files.push((
        "assets/games.json".into(),
        serde_json::to_vec(&json!({
            "canvas": [w, h], "card": [card.0, card.1], "scale": scale,
            "games": list, "games_dir": covered(&style.add_hint), "confirm": style.confirm,
        }))?,
    ));
    let g = (18. * scale).round() as u32;
    files.push(("assets/backdrop.png".into(), png(&backdrop(w / 2, h / 2))?));
    files.push(("assets/glow.png".into(), png(&glow(((card.0 as f32 * 1.1) as u32 + 2 * g, (card.1 as f32 * 1.1) as u32 + 2 * g), g))?));
    files.push(("assets/pill.png".into(), png(&pill(((290. * scale) as u32, (38. * scale) as u32)))?));
    files.push(("assets/button.png".into(), png(&button((24. * scale) as u32, style.cross))?));
    files.push(("assets/dpad.png".into(), png(&dpad((24. * scale) as u32))?));
    files.push((
        "project.json".into(),
        serde_json::to_vec_pretty(&json!({
            "schema": 2, "name": "CrySirror", "content_revision": "launcher-1",
            "canvas": [w, h], "background": [10, 12, 28, 255], "assets": "assets",
            "clients": [{"name": CLIENT, "adapter": "crysirror-script-v1", "game": CLIENT}]
        }))?,
    ));
    files.push(("launcher.js".into(), SCRIPT.as_bytes().to_vec()));
    Ok(files)
}

/// Write [`build`] output to `dir` (replacing an earlier launcher project).
pub fn write(dir: &Path, files: &[(String, Vec<u8>)]) -> Result<()> {
    let _ = std::fs::remove_dir_all(dir);
    for (path, bytes) in files {
        let path = dir.join(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, bytes)?;
    }
    Ok(())
}

/// The entry id in the script's `launch.json` save text.
pub fn choice_from(save: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(save).ok()?;
    value["game"].as_str().map(str::to_owned)
}

/// Find and consume `launch.json` below a host save directory `state`, so
/// the next start shows the launcher again.
pub fn take_choice(state: &Path) -> Option<String> {
    let path = find(state, CHOICE_FILE)?;
    let text = std::fs::read_to_string(&path).ok();
    let _ = std::fs::remove_file(&path);
    choice_from(&text?)
}

fn find(dir: &Path, name: &str) -> Option<PathBuf> {
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find(&path, name) {
                return Some(found);
            }
        } else if entry.file_name() == name {
            return Some(path);
        }
    }
    None
}

/// Cover lookup cached as `<cache>/<hash>-{image,icon,none}.png` by folder
/// name. Delete the cache to pick up a newly added cover file.
fn cached_cover(dir: &Path, id: &str, cache: &Path) -> Option<Cover> {
    let key = format!("{:08x}", fnv(id));
    for (suffix, kind) in [("image", CoverKind::Image), ("icon", CoverKind::Icon)] {
        let path = cache.join(format!("{key}-{suffix}.png"));
        if let Ok(image) = image::open(&path) {
            return Some(Cover { image: image.to_rgba8(), kind, source: path });
        }
    }
    if cache.join(format!("{key}-none")).exists() {
        return None;
    }
    let found = crate::cover::resolve(dir);
    let _ = std::fs::create_dir_all(cache);
    match &found {
        Some(cover) => {
            let suffix = if cover.kind == CoverKind::Image { "image" } else { "icon" };
            let _ = cover.image.save(cache.join(format!("{key}-{suffix}.png")));
        }
        None => {
            let _ = std::fs::write(cache.join(format!("{key}-none")), b"");
        }
    }
    found
}

fn png(image: &RgbaImage) -> Result<Vec<u8>> {
    let mut out = std::io::Cursor::new(Vec::new());
    image.write_to(&mut out, image::ImageFormat::Png)?;
    Ok(out.into_inner())
}

fn coverage(font: &[u8]) -> Result<crysirror_text::TextSystem> {
    let mut text = crysirror_text::TextSystem::default();
    text.add_font("default", font.to_vec())?;
    Ok(text)
}

fn fnv(id: &str) -> u32 {
    id.bytes().fold(0x811c_9dc5u32, |h, b| (h ^ u32::from(b)).wrapping_mul(0x0100_0193))
}

/// Stable per-entry hue in degrees.
fn hue(id: &str) -> f32 {
    (fnv(id) % 360) as f32
}

fn hsv(h: f32, s: f32, v: f32) -> [f32; 3] {
    let c = v * s;
    let x = c * (1. - ((h / 60.) % 2. - 1.).abs());
    let (r, g, b) = match (h / 60.) as u32 % 6 {
        0 => (c, x, 0.),
        1 => (x, c, 0.),
        2 => (0., c, x),
        3 => (0., x, c),
        4 => (x, 0., c),
        _ => (c, 0., x),
    };
    let m = v - c;
    [r + m, g + m, b + m]
}

/// Signed distance from (x, y) to a rounded rectangle centred in w x h.
fn rounded(x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
    let qx = (x - w / 2.).abs() - (w / 2. - r);
    let qy = (y - h / 2.).abs() - (h / 2. - r);
    let outside = (qx.max(0.).powi(2) + qy.max(0.).powi(2)).sqrt();
    outside + qx.max(qy).min(0.) - r
}

fn pixel(rgb: [f32; 3], a: f32) -> Rgba<u8> {
    let c = |v: f32| (v.clamp(0., 1.) * 255.).round() as u8;
    Rgba([c(rgb[0]), c(rgb[1]), c(rgb[2]), c(a)])
}

fn round_corners(image: &mut RgbaImage) {
    let (w, h) = (image.width() as f32, image.height() as f32);
    let r = w * 0.075;
    for (x, y, p) in image.enumerate_pixels_mut() {
        let a = (0.5 - rounded(x as f32 + 0.5, y as f32 + 0.5, w, h, r)).clamp(0., 1.);
        p[3] = (f32::from(p[3]) * a) as u8;
    }
}

/// A card without cover art: a diagonal two-hue gradient, a soft top sheen
/// and a darker lower band behind the initials.
fn card_image((w, h): (u32, u32), hue: f32) -> RgbaImage {
    let mut image = RgbaImage::from_fn(w, h, |x, y| {
        let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
        let t = (fx / w as f32 + fy / h as f32) / 2.;
        let top = hsv(hue, 0.62, 0.86);
        let bottom = hsv((hue + 38.) % 360., 0.70, 0.42);
        let mut c = [0.; 3];
        for i in 0..3 {
            c[i] = top[i] * (1. - t) + bottom[i] * t;
        }
        let sheen = (1. - fy / (h as f32 * 0.45)).max(0.).powi(2) * 0.16;
        let shade = ((fy / h as f32 - 0.55) / 0.45).clamp(0., 1.) * 0.28;
        for v in &mut c {
            *v = *v + sheen - shade * *v;
        }
        pixel(c, 1.)
    });
    round_corners(&mut image);
    image
}

/// Cover art on a card: pictures fill it (centre crop), icons sit
/// integer-scaled (crisp) on a gradient from their own average colour.
fn cover_card((w, h): (u32, u32), cover: &Cover) -> RgbaImage {
    let src = &cover.image;
    let mut out = match cover.kind {
        CoverKind::Image => {
            let scale = (w as f32 / src.width() as f32).max(h as f32 / src.height() as f32);
            let (sw, sh) = (((w as f32 / scale) as u32).clamp(1, src.width()), ((h as f32 / scale) as u32).clamp(1, src.height()));
            let crop = image::imageops::crop_imm(src, (src.width() - sw) / 2, (src.height() - sh) / 2, sw, sh).to_image();
            image::imageops::resize(&crop, w, h, image::imageops::FilterType::Triangle)
        }
        CoverKind::Icon => {
            let (mut sum, mut n) = ([0f32; 3], 0f32);
            for p in src.pixels().filter(|p| p[3] > 128) {
                for c in 0..3 {
                    sum[c] += f32::from(p[c]) / 255.;
                }
                n += 1.;
            }
            let avg = if n > 0. { sum.map(|v| v / n) } else { [0.4, 0.4, 0.6] };
            let mut base = RgbaImage::from_fn(w, h, |_, y| {
                let t = y as f32 / h as f32;
                pixel(avg.map(|v| (v * (0.75 - 0.45 * t)).clamp(0., 1.)), 1.)
            });
            let factor = ((w as f32 * 0.62) / src.width() as f32).floor().max(1.) as u32;
            let icon = image::imageops::resize(src, src.width() * factor, src.height() * factor, image::imageops::FilterType::Nearest);
            let x = w.saturating_sub(icon.width()) / 2;
            let y = h.saturating_sub(icon.height()) * 2 / 5;
            image::imageops::overlay(&mut base, &icon, i64::from(x), i64::from(y));
            base
        }
    };
    round_corners(&mut out);
    out
}

/// Night-sky backdrop: deep indigo gradient with two soft colour glows.
fn backdrop(w: u32, h: u32) -> RgbaImage {
    RgbaImage::from_fn(w, h, |x, y| {
        let (u, v) = (x as f32 / w as f32, y as f32 / h as f32);
        let base = [0.045 + 0.04 * v, 0.05 + 0.02 * v, 0.12 + 0.10 * v];
        let glow = |cx: f32, cy: f32, radius: f32| {
            let d = ((u - cx).powi(2) * (w as f32 / h as f32).powi(2) + (v - cy).powi(2)).sqrt();
            (-(d / radius).powi(2)).exp()
        };
        let violet = glow(0.85, 0.05, 0.55) * 0.40;
        let teal = glow(0.05, 1.0, 0.50) * 0.22;
        pixel(
            [
                base[0] + violet * 0.42 + teal * 0.05,
                base[1] + violet * 0.30 + teal * 0.42,
                base[2] + violet * 0.85 + teal * 0.55,
            ],
            1.,
        )
    })
}

/// Focus ring with an outer bloom, `pad` pixels of bloom on each side.
fn glow((w, h): (u32, u32), pad: u32) -> RgbaImage {
    let inner = (w - 2 * pad, h - 2 * pad);
    let r = inner.0 as f32 * 0.075;
    let accent = [0.55, 0.62, 1.0];
    RgbaImage::from_fn(w, h, |x, y| {
        let d = rounded(x as f32 + 0.5 - pad as f32, y as f32 + 0.5 - pad as f32, inner.0 as f32, inner.1 as f32, r);
        let ring = (1.5 - (d - 1.5).abs()).clamp(0., 1.);
        let bloom = if d > 0. { (-(d / (pad as f32 * 0.45)).powi(2)).exp() * 0.55 } else { 0. };
        pixel([accent[0] + (1. - accent[0]) * ring, accent[1] + (1. - accent[1]) * ring, accent[2]], ring.max(bloom))
    })
}

fn pill((w, h): (u32, u32)) -> RgbaImage {
    RgbaImage::from_fn(w, h, |x, y| {
        let a = (0.5 - rounded(x as f32 + 0.5, y as f32 + 0.5, w as f32, h as f32, h as f32 / 2.)).clamp(0., 1.);
        pixel([1., 1., 1.], a * 0.10)
    })
}

/// Confirm button glyph: a white disc, with the PS Vita cross drawn in.
fn button(size: u32, cross: bool) -> RgbaImage {
    let s = size as f32;
    RgbaImage::from_fn(size, size, |x, y| {
        let (fx, fy) = (x as f32 + 0.5 - s / 2., y as f32 + 0.5 - s / 2.);
        let disc = (s / 2. - 0.5 - (fx * fx + fy * fy).sqrt()).clamp(0., 1.);
        let mut c = [0.94, 0.95, 1.0];
        if cross {
            let arm = s * 0.27;
            let line = |a: f32, b: f32| (s * 0.07 - (a - b).abs() / std::f32::consts::SQRT_2).clamp(0., 1.);
            let k = if fx.abs() < arm && fy.abs() < arm { line(fx, fy).max(line(fx, -fy)) } else { 0. };
            for (v, t) in c.iter_mut().zip([0.30, 0.47, 0.92]) {
                *v = *v * (1. - k) + t * k;
            }
        }
        pixel(c, disc)
    })
}

fn dpad(size: u32) -> RgbaImage {
    let s = size as f32;
    RgbaImage::from_fn(size, size, |x, y| {
        let (fx, fy) = ((x as f32 + 0.5 - s / 2.).abs(), (y as f32 + 0.5 - s / 2.).abs());
        let bar = |a: f32, b: f32| ((s * 0.17 - a).min(s * 0.46 - b)).clamp(0., 1.);
        pixel([1., 1., 1.], bar(fx, fy).max(bar(fy, fx)) * 0.86)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_project_and_reads_choice() {
        let root = std::env::temp_dir().join(format!("crysirror-library-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("games/alpha_one")).unwrap();
        let entries = vec![
            Entry::folder(&root.join("games/alpha_one"), Some(&root.join("covers"))),
            Entry { id: "demo".into(), name: "Beta".into(), detail: "Demo".into(), cover: None },
        ];
        let style = Style { confirm: "A", cross: false, add_hint: "games/".into() };
        let files = build(&entries, (1280, 720), None, &style).unwrap();
        write(&root.join("launcher"), &files).unwrap();
        let list: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("launcher/assets/games.json")).unwrap()).unwrap();
        assert_eq!(list["games"][0]["name"], "alpha one");
        assert_eq!(list["games"][0]["initials"], "AO");
        assert!(root.join("launcher/assets/card-1.png").is_file() && root.join("launcher/launcher.js").is_file());
        assert!(root.join("covers").read_dir().unwrap().count() == 1);
        let save = root.join("state/crysirror-script-v1/launcher/save");
        std::fs::create_dir_all(&save).unwrap();
        std::fs::write(save.join(CHOICE_FILE), r#"{"game":"demo"}"#).unwrap();
        assert_eq!(take_choice(&root.join("state")).as_deref(), Some("demo"));
        assert!(take_choice(&root.join("state")).is_none());
        std::fs::remove_dir_all(&root).unwrap();
    }
}

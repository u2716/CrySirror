// SPDX-License-Identifier: MPL-2.0
//! Presentation of uGUI layouts ([`crysirror_unity::ugui`]) as engine layer
//! operations: sprite crops uploaded once, nine-slice quads, and
//! `TextMeshProUGUI` runs drawn with the TrueType data of the font asset's
//! dynamic fallback (`m_SourceFontFile` → `Font.m_FontData`; the static SDF
//! atlases are generated from the same faces).
use crate::unity::Unity;
use anyhow::Result;
use crysirror_core::service::{FontLibrary, FontJob, FontLoadSource, FontLoadSpec, FontLoadState, TextMeasurement};
use crysirror_unity::{managed::Value, ugui, PPtr};
use serde_json::{json, Value as Json};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// Uploads and font registration through the host.
pub trait Host {
    fn upload(&mut self, id: u32, width: u32, height: u32, rgba: &[u8]) -> Result<String>;
    fn fonts(&mut self) -> Result<Arc<dyn FontLibrary>>;
    fn measurement(&mut self) -> Result<Arc<dyn TextMeasurement>>;
    /// The session's TMP font registrations, shared by every hierarchy.
    fn font_cache(&mut self) -> &mut FontCache;
}

/// TMP font asset (file, path id) → its registration. A font is
/// registered once per session; its load job is released when it resolves
/// (the font library bounds outstanding jobs per client).
pub type FontCache = HashMap<(String, i64), FontSlot>;

#[derive(Clone, Debug)]
pub enum FontSlot {
    Pending(FontJob),
    Ready(String),
    Missing,
}

/// A sprite crop uploaded as its own image: asset, size in pixels.
#[derive(Clone, Debug)]
struct Uploaded {
    asset: String,
    width: f32,
    height: f32,
}

/// The uGUI presentation of one hierarchy (build scene or prefab
/// instance). Each `slot` owns its own layer and upload id range.
pub struct Ui {
    sprites: HashMap<(String, i64), Option<Uploaded>>,
    next_upload: u32,
    submitted: BTreeMap<u32, String>,
    pub hits: Vec<(usize, ugui::Rect)>,
    layer_base: u32,
    upload_base: u32,
}

impl Default for Ui {
    fn default() -> Self {
        Self::new(0)
    }
}

fn byte(v: f32) -> u8 {
    (v.clamp(0., 1.) * 255. + 0.5) as u8
}

impl Ui {
    /// Slot 0 is the build scene; prefab overlays use 1.. (at most 15).
    pub fn new(slot: u32) -> Self {
        Self {
            sprites: HashMap::new(),
            next_upload: 0,
            submitted: BTreeMap::new(),
            hits: Vec::new(),
            layer_base: 0x1000 + slot * 0x10_0000,
            upload_base: 0x4000_0000 + slot * 0x0100_0000,
        }
    }

    /// Remove every submitted layer (the hierarchy is gone).
    pub fn clear(&mut self) -> Vec<Json> {
        self.hits.clear();
        std::mem::take(&mut self.submitted).into_keys().map(|id| json!({"op": "remove", "id": id})).collect()
    }

    /// The uploaded crop of a sprite of `file`.
    fn sprite(&mut self, unity: &mut Unity, host: &mut dyn Host, file: &str, p: PPtr) -> Result<Option<Uploaded>> {
        if p.is_null() {
            return Ok(None);
        }
        let (fname, path_id) = unity.target(file, p)?;
        if let Some(s) = self.sprites.get(&(fname.clone(), path_id)) {
            return Ok(s.clone());
        }
        let up = match unity.sprite_image(&fname, path_id)? {
            Some(img) => {
                self.next_upload += 1;
                let asset = host.upload(self.upload_base + self.next_upload, img.width, img.height, &img.rgba)?;
                Some(Uploaded { asset, width: img.width as f32, height: img.height as f32 })
            }
            None => None,
        };
        self.sprites.insert((fname, path_id), up.clone());
        Ok(up)
    }

    /// The registered alias of a TMP font asset's TrueType fallback.
    fn font(&mut self, unity: &mut Unity, host: &mut dyn Host, file: &str, p: PPtr) -> Result<Option<String>> {
        if p.is_null() {
            return Ok(None);
        }
        let key = unity.target(file, p)?;
        let slot = match host.font_cache().get(&key) {
            Some(s) => s.clone(),
            None => {
                let slot = match unity.tmp_font_data(&key.0, key.1)? {
                    Some((name, data)) => FontSlot::Pending(host.fonts()?.begin(FontLoadSpec {
                        name: format!("amanatsu-{}", name.to_ascii_lowercase().replace(|c: char| !c.is_ascii_alphanumeric(), "-")),
                        source: FontLoadSource::Bytes(data.into()),
                        face_index: 0,
                    })?),
                    None => FontSlot::Missing,
                };
                host.font_cache().insert(key.clone(), slot.clone());
                slot
            }
        };
        Ok(match slot {
            FontSlot::Ready(a) => Some(a),
            FontSlot::Missing => None,
            FontSlot::Pending(job) => match host.fonts()?.status(job)? {
                FontLoadState::Pending => None,
                FontLoadState::Ready(info) => {
                    host.fonts()?.release_job(job)?;
                    host.font_cache().insert(key, FontSlot::Ready(info.alias.clone()));
                    Some(info.alias)
                }
                FontLoadState::Failed(e) => {
                    host.fonts()?.release_job(job)?;
                    host.font_cache().insert(key, FontSlot::Missing);
                    anyhow::bail!("TMP font registration: {e}")
                }
            },
        })
    }

    /// Text width of an unwrapped run in pixels (for layout preferred sizes).
    pub fn measure(&mut self, unity: &mut Unity, host: &mut dyn Host, file: &str, text: &str, font: PPtr, size: f32) -> [f32; 2] {
        let fallback = [text.chars().map(|c| if c.is_ascii() { size * 0.55 } else { size }).sum::<f32>(), size * 1.25];
        let Ok(Some(alias)) = self.font(unity, host, file, font) else { return fallback };
        let Ok(m) = host.measurement() else { return fallback };
        m.measure_simple(&alias, size.clamp(1., 512.), text).map(|e| [e.advance, size * 1.25]).unwrap_or(fallback)
    }

    /// Layer operations for this frame's layout of a scene of `file`.
    pub fn compose(&mut self, unity: &mut Unity, host: &mut dyn Host, file: &str, layout: &ugui::Layout, base_order: i32) -> Result<Vec<Json>> {
        let mut layers: BTreeMap<u32, Json> = BTreeMap::new();
        for (i, item) in layout.items.iter().enumerate() {
            let id = self.layer_base + (i as u32) * 16;
            let r = item.rect;
            if r.width() <= 0. || r.height() <= 0. {
                continue;
            }
            let order = [base_order + item.order.0 as i32, item.order.1 as i32];
            match &item.paint {
                ugui::Paint::Image { sprite, color, sliced, image, .. } => {
                    let tint = [byte(color[0]), byte(color[1]), byte(color[2]), byte(color[3])];
                    if tint[3] == 0 {
                        continue;
                    }
                    if let Some(asset) = image {
                        layers.insert(id, json!({"id": id, "rect": [r.x0, r.y0, r.width(), r.height()], "paint": {"kind": "image", "asset": asset, "tint": tint}, "uv": [0., 0., 1., 1.], "order": order}));
                        continue;
                    }
                    let Some(up) = self.sprite(unity, host, file, *sprite)? else {
                        if sprite.is_null() {
                            layers.insert(id, json!({"id": id, "rect": [r.x0, r.y0, r.width(), r.height()], "paint": {"kind": "solid", "color": tint}, "order": order}));
                        }
                        continue;
                    };
                    match sliced {
                        Some((b, tb)) => {
                            // Borders (left, bottom, right, top): canvas units → screen px.
                            let s = item.scale;
                            let (bl, bb, br, bt) = (b[0] * s, b[1] * s, b[2] * s, b[3] * s);
                            let xs = [r.x0, r.x0 + bl, r.x1 - br, r.x1];
                            let ys = [r.y0, r.y0 + bt, r.y1 - bb, r.y1];
                            let us = [0., tb[0] / up.width, 1. - tb[2] / up.width, 1.];
                            let vs = [0., tb[3] / up.height, 1. - tb[1] / up.height, 1.];
                            for row in 0..3 {
                                for col in 0..3 {
                                    let (x0, x1, y0, y1) = (xs[col], xs[col + 1], ys[row], ys[row + 1]);
                                    if x1 <= x0 || y1 <= y0 {
                                        continue;
                                    }
                                    let sid = id + 1 + (row * 3 + col) as u32;
                                    layers.insert(sid, json!({"id": sid, "rect": [x0, y0, x1 - x0, y1 - y0], "paint": {"kind": "image", "asset": up.asset, "tint": tint}, "uv": [us[col], vs[row], us[col + 1], vs[row + 1]], "order": order}));
                                }
                            }
                        }
                        None => {
                            layers.insert(id, json!({"id": id, "rect": [r.x0, r.y0, r.width(), r.height()], "paint": {"kind": "image", "asset": up.asset, "tint": tint}, "uv": [0., 0., 1., 1.], "order": order}));
                        }
                    }
                }
                ugui::Paint::Text { text, font, size, color, alignment, margin, .. } => {
                    let text = strip_rich_text(text);
                    if text.is_empty() || color[3] <= 0. {
                        continue;
                    }
                    let Some(alias) = self.font(unity, host, file, *font)? else { continue };
                    let px = (size * item.scale).clamp(1., 512.);
                    let m = host.measurement()?;
                    let lines: Vec<&str> = text.split('\n').collect();
                    let line_h = px * 1.25;
                    let inner = ugui::Rect { x0: r.x0 + margin[0] * item.scale, y0: r.y0 + margin[1] * item.scale, x1: r.x1 - margin[2] * item.scale, y1: r.y1 - margin[3] * item.scale };
                    // TextAlignmentOptions: horizontal bits 1/2/4, vertical 256/512/1024.
                    let ax = if alignment & 2 != 0 { 0.5 } else if alignment & 4 != 0 { 1. } else { 0. };
                    let ay = if alignment & 512 != 0 || alignment & 4096 != 0 { 0.5 } else if alignment & 1024 != 0 { 1. } else { 0. };
                    let block = lines.len() as f32 * line_h;
                    let top = inner.y0 + (inner.height() - block) * ay;
                    for (li, line) in lines.iter().enumerate().take(15) {
                        if line.is_empty() {
                            continue;
                        }
                        let w = m.measure_simple(&alias, px, line).map(|e| e.advance).unwrap_or(px * line.chars().count() as f32);
                        let x = inner.x0 + (inner.width() - w) * ax;
                        let lid = id + 1 + li as u32;
                        layers.insert(lid, json!({
                            "id": lid,
                            "rect": [x, top + li as f32 * line_h, w + px, line_h],
                            "paint": {"kind": "text", "text": line, "font": alias, "size": px as u16, "color": [byte(color[0]), byte(color[1]), byte(color[2]), byte(color[3])], "layout": crysirror_core::service::SIMPLE_TEXT_LAYOUT},
                            "order": order,
                        }));
                    }
                }
            }
        }
        self.hits = layout.hits.clone();
        let mut ops = Vec::new();
        let mut next = BTreeMap::new();
        for (id, spec) in layers {
            let text = spec.to_string();
            if self.submitted.get(&id) != Some(&text) {
                ops.push(json!({"op": "put", "layer": spec}));
            }
            next.insert(id, text);
        }
        for id in self.submitted.keys() {
            if !next.contains_key(id) {
                ops.push(json!({"op": "remove", "id": id}));
            }
        }
        self.submitted = next;
        Ok(ops)
    }
}

/// TMP rich text tags removed (`<color=…>`, `<size=…>`, `<br>` → newline).
pub fn strip_rich_text(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('<') {
        out.push_str(&rest[..i]);
        match rest[i..].find('>') {
            Some(j) => {
                let tag = &rest[i + 1..i + j];
                if tag.eq_ignore_ascii_case("br") {
                    out.push('\n');
                }
                rest = &rest[i + j + 1..];
            }
            None => {
                out.push_str(&rest[i..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

#[allow(dead_code)]
fn _value_used(_: &Value) {}

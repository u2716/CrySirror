// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Presentation: the native priority tree (containers sorted stably by
//! ascending priority) flattened into shared retained layers.
//! Native blend modes map onto shared blends and filters; the composition
//! itself is the shared renderer's.
use crate::memory::scene as S;
use crate::sprite::{Content, Point, Sprite};
use crate::text::{Block, Reveal, Style};
use crate::vm::Vm;
use anyhow::Result;
use crysirror_core::service::HostServices;
use serde_json::{Value as Json, json};
use std::collections::BTreeMap;

/// Line ascent of the fixed-pitch message face as a fraction of the cell:
/// GDI places each glyph below its tmAscent inside the cell.
const ASCENT: f32 = 0.86;

#[derive(Default)]
pub struct Presented {
    layers: BTreeMap<u32, Json>,
    /// Draw order of the zoom transition's first layer.
    zoom_order: i32,
}

struct Out {
    draws: Vec<(u32, Json)>,
    shake: [i32; 2],
    font: String,
}

fn rgb(c: u32) -> [u8; 3] {
    [(c >> 16) as u8, (c >> 8) as u8, c as u8]
}

impl Out {
    fn push(&mut self, id: u32, mut layer: Json) {
        let order = self.draws.len() as i32;
        layer["id"] = json!(id);
        layer["order"] = json!([order, 0]);
        if let Some(r) = layer["rect"].as_array_mut() {
            r[0] = json!(r[0].as_f64().unwrap_or(0.) + f64::from(self.shake[0]));
            r[1] = json!(r[1].as_f64().unwrap_or(0.) + f64::from(self.shake[1]));
        }
        self.draws.push((id, layer));
    }
    /// A sprite at `origin + pos`, with its native blend mode (dispatcher
    /// 0x4464E0 in the Shapeshifter build). Modes without a native case draw
    /// nothing, like the dispatcher's default.
    fn sprite(&mut self, id: u32, s: &Sprite, origin: Point) {
        if !s.visible {
            return;
        }
        let Some(content) = &s.content else { return };
        let [w, h] = content.size();
        let [cx, cy] = s.crop.map_or([0, 0], |c| [c[0], c[1]]);
        let rect = json!([origin.x + s.pos.x - cx, origin.y + s.pos.y - cy, w, h]);
        let mut paint = match content {
            Content::Image { asset, .. } => json!({"kind":"image","asset":asset,"tint":[255,255,255,255]}),
            Content::Fill { color, .. } => {
                let [r, g, b] = rgb(*color);
                json!({"kind":"solid","color":[r,g,b,255]})
            }
            Content::Text { text, cell, shadow, .. } => {
                let style = Style { size: *cell, width: i32::MAX / 4, height: *cell, color: 0xFFFFFF, shadow: *shadow, ..Default::default() };
                let mut block = Block::new(256);
                block.append(text, &style);
                let at = Point { x: origin.x + s.pos.x, y: origin.y + s.pos.y };
                let level = if s.mode == 2 { 1.0 - f32::from(s.level) / 255.0 } else { 1.0 };
                self.glyphs(id, &block, None, &Style { width: w as i32, ..style }, self.font.clone().as_str(), at, level);
                return;
            }
        };
        let level = i32::from(s.level);
        let mut layer = json!({"rect":rect});
        if let Some(c) = s.crop {
            layer["source_crop"] = json!(c);
        }
        match s.mode {
            0 => {}
            // Level fades: the level hides the sprite (0 opaque, 255 gone).
            // Dissolve (0x12) and blinds (0x14/0x15) patterns are drawn as
            // the same level fade.
            2 | 0x12 | 0x14 | 0x15 => layer["opacity"] = json!(255 - level),
            // CMath add-table darkening / del-table brightening by level.
            3 => {
                let k = ((256 - level) * 255 / 256) as u8;
                paint["tint"] = json!([k, k, k, 255]);
                if let Content::Fill { color, .. } = content {
                    let [r, g, b] = rgb(*color).map(|c| (u32::from(c) * k as u32 / 255) as u8);
                    paint["color"] = json!([r, g, b, 255]);
                }
            }
            4 => layer["filter"] = json!({"bright": level as f32 / 256.0}),
            8 => {
                if level >= 5 {
                    layer["opacity"] = json!(128);
                }
            }
            0x0D => layer["filter"] = json!({"invert": 1.0}),
            0x0E => {
                layer["blend"] = json!("add");
                layer["opacity"] = json!(255 - level);
            }
            0x67 => layer["blend"] = json!("inverse_destination"),
            // Emboss rows: values above 128 brighten, below darken (hard light).
            0x68 => layer["blend"] = json!({"stored_byte":{"mode":"hard_light"}}),
            0x6B => {
                let [r, g, b] = rgb(s.color);
                paint["tint"] = json!([r, g, b, 255]);
                layer["blend"] = json!("add");
                layer["opacity"] = json!(level);
            }
            // Darken / brighten the destination by the level.
            0x6C | 0x6D => {
                let c = if s.mode == 0x6C { 0 } else { 255 };
                paint = json!({"kind":"solid","color":[c,c,c,255]});
                layer["opacity"] = json!(level);
            }
            _ => return,
        }
        layer["paint"] = paint;
        self.push(id, layer);
    }
    fn glyphs(&mut self, id: u32, block: &Block, reveal: Option<&Reveal>, style: &Style, font: &str, at: Point, dim: f32) {
        let mut glyphs = Vec::new();
        for (i, g) in block.glyphs.iter().enumerate() {
            let alpha = match reveal {
                Some(r) if i >= r.revealed => continue,
                Some(r) => 255 - i32::from(r.levels.get(i).copied().unwrap_or(0)),
                None => 255,
            };
            let alpha = (alpha as f32 * dim) as u8;
            if alpha == 0 {
                continue;
            }
            let [r, gg, b] = rgb(g.color);
            let baseline_y = if style.baseline_at_bottom { g.y + g.height } else { g.y + (g.height as f32 * ASCENT) as i32 };
            let mut glyph = json!({"character":g.text.chars().next().unwrap_or(' '),"baseline":[g.x,baseline_y],
                "size":g.size,"color":[r,gg,b,alpha],"cell_width":g.advance});
            if g.bold {
                glyph["bold"] = json!(true);
            }
            if style.shadow {
                glyph["style"] = json!({"shadow":{"offset":[g.advance / 12, g.height / 12],"color":[0,0,0,alpha]}});
            }
            glyphs.push(glyph);
            // A half-width pair holds its second character in the same cell.
            if let Some(second) = g.text.chars().nth(1) {
                let mut extra = glyphs.last().unwrap().clone();
                extra["character"] = json!(second);
                extra["baseline"][0] = json!(g.x + g.advance / 2);
                extra["cell_width"] = json!(g.advance / 2);
                glyphs.push(extra);
            }
            for rb in &g.ruby {
                glyphs.push(json!({"character":rb.text.chars().next().unwrap_or(' '),
                    "baseline":[rb.x, rb.y + (rb.size as f32 * ASCENT) as i32],"size":rb.size,"color":[r,gg,b,alpha],"cell_width":rb.width}));
            }
        }
        if glyphs.is_empty() {
            return;
        }
        let size = block.glyphs.first().map_or(style.size, |g| g.size);
        self.push(id, json!({"rect":[at.x, at.y, style.width.max(1), style.height.max(1)],
            "paint":{"kind":"glyph_run","font":font,"size":size,"glyphs":glyphs}}));
    }
}

impl Vm {
    /// Builds the frame's layers and submits the difference.
    pub fn present(&mut self, h: &mut dyn HostServices, font: &str) -> Result<()> {
        // The old frame is captured from what was presented last.
        if self.zoom.as_ref().is_some_and(|z| z.capturing_old()) {
            return Ok(());
        }
        if !std::mem::take(&mut self.dirty) && self.transition.is_none() {
            return Ok(());
        }
        let font = self.text_font.clone().unwrap_or_else(|| font.to_owned());
        let font = font.as_str();
        let mut out = Out { draws: Vec::new(), shake: self.shake, font: font.to_owned() };
        let zero = Point::default();
        // The save screen draws once its thumbnail frame is captured.
        let save_shown = self.save_screen.as_ref().is_some_and(|s| s.capture.is_none());
        let game = self.state_screen.is_none();
        // Root children: (priority, insertion index, node).
        enum Node {
            Layer(usize),
            Overlay,
            Effect,
            Message,
            Choice,
            Map,
            Gauges,
        }
        let mut root: Vec<(i16, usize, Node)> = (0..S::LAYER_COUNT).map(|i| (self.mem.w(S::LAYER_ORDERS + 2 * i), i, Node::Layer(i))).collect();
        root.push((self.mem.w(S::OVERLAY_ORDER), 100, Node::Overlay));
        root.push((self.mem.w(S::EFFECT_ORDER), 101, Node::Effect));
        root.push((50, 102, Node::Message));
        root.push((100, 103, Node::Choice));
        root.push((self.map.priority, 104, Node::Map));
        root.push((50, 105, Node::Gauges));
        root.sort_by_key(|(p, i, _)| (*p, *i));
        if !game {
            root.clear();
        }
        for (_, _, node) in &root {
            match node {
                Node::Layer(i) => {
                    let l = &self.layers[*i];
                    if !l.visible {
                        continue;
                    }
                    let base = 10_000 + *i as u32 * 4;
                    out.sprite(base, &l.under[0], zero);
                    out.sprite(base + 1, &l.under[1], zero);
                    out.sprite(base + 2, &l.special, zero);
                    if l.convert.is_none() {
                        out.sprite(base + 3, &l.main, l.frame_offset());
                        if l.overlay.visible {
                            let o = Sprite { pos: l.main.pos, mode: l.main.mode, level: l.main.level, visible: l.main.visible, ..l.overlay.clone() };
                            out.sprite(70_000 + *i as u32, &o, zero);
                        }
                    }
                }
                Node::Overlay => out.sprite(9_001, &self.overlay, zero),
                Node::Effect => out.sprite(9_000, &self.effect, zero),
                Node::Message => {
                    // sub_430FB0 / sub_430EA0 hide the window under the screens.
                    if self.message.hidden || save_shown || self.config_screen.is_some() {
                        continue;
                    }
                    // The face sits behind the boxes at order 0, above them at 10.
                    let face_at = Point {
                        x: self.mem.d(S::MESSAGE + crate::memory::message::FACE_X),
                        y: self.mem.d(S::MESSAGE + crate::memory::message::FACE_Y),
                    };
                    let face_front = self.mem.w(S::MESSAGE + crate::memory::message::FACE_ORDER) != 0;
                    if !face_front {
                        out.sprite(11_050, &self.message.face_old, face_at);
                        out.sprite(11_051, &self.message.face, face_at);
                    }
                    for (b, bx) in self.message.boxes.iter().enumerate() {
                        if !bx.visible {
                            continue;
                        }
                        let base = 11_000 + b as u32 * 16;
                        let o = bx.pos;
                        out.sprite(base, &bx.back, o);
                        out.sprite(base + 1, &bx.front, o);
                        let at = Point { x: o.x + bx.text_pos.x, y: o.y + bx.text_pos.y };
                        out.glyphs(base + 2, &bx.text, Some(&bx.reveal), &bx.style, font, at, 1.0);
                        if bx.waiting {
                            let w = Point { x: o.x + bx.wait_pos.x, y: o.y + bx.wait_pos.y };
                            out.sprite(base + 3, &bx.wait_body, w);
                            out.sprite(base + 4, &bx.wait_glow, w);
                        }
                        if bx.name_visible {
                            out.sprite(base + 5, &bx.plate, o);
                            let at = Point { x: o.x + bx.name_pos.x, y: o.y + bx.name_pos.y };
                            out.glyphs(base + 6, &bx.name, None, &bx.name_style, font, at, 1.0);
                        }
                    }
                    if face_front {
                        out.sprite(11_050, &self.message.face_old, face_at);
                        out.sprite(11_051, &self.message.face, face_at);
                    }
                    if self.message.panel_visible {
                        let p = self.message.panel_pos;
                        out.sprite(11_100, &self.message.panel_bg, p);
                        for (i, b) in self.message.panel.iter().enumerate() {
                            let mut face = b.face.clone();
                            if face.hovered {
                                if let Some(f) = &b.focus {
                                    face.content = Some(f.clone());
                                }
                            }
                            out.sprite(11_101 + i as u32, &face, p);
                        }
                        if let Some(sl) = &self.message.slider {
                            out.sprite(11_130, &sl.knob.face, p);
                        }
                    }
                }
                Node::Gauges => {
                    for (g, parts) in self.gauges.iter().enumerate() {
                        let r = S::GAUGES + S::GAUGE_STRIDE * g;
                        if self.mem.w(r) == 0 {
                            continue;
                        }
                        let o = Point { x: self.mem.d(r + 4), y: self.mem.d(r + 8) };
                        let base = 11_600 + g as u32 * 16;
                        out.sprite(base, &parts.fix, o);
                        if let Some(bar) = &parts.bar {
                            let (max, min) = (self.mem.d(r + 36), self.mem.d(r + 40));
                            let span = (max - min).max(1);
                            let fill = (parts.shown - min).clamp(0, span);
                            let [w, hh] = bar.size();
                            let shown = (w as i64 * i64::from(fill) / i64::from(span)) as u32;
                            if let (crate::sprite::Content::Image { asset, .. }, true) = (bar, shown > 0) {
                                let at = Point { x: o.x + self.mem.d(r + 20), y: o.y + self.mem.d(r + 24) };
                                out.push(base + 1, json!({"rect":[at.x, at.y, shown, hh],"paint":{"kind":"image","asset":asset},
                                    "uv":[0.0, 0.0, f64::from(shown) / f64::from(w.max(1)), 1.0]}));
                            }
                        }
                        if let Some(crate::sprite::Content::Image { asset, size }) = &parts.digits {
                            let cell = (size[0] / 10).max(1);
                            let at = Point { x: o.x + self.mem.d(r + 12), y: o.y + self.mem.d(r + 16) };
                            let text = parts.shown.unsigned_abs().to_string();
                            for (k, d) in text.bytes().enumerate() {
                                let digit = u32::from(d - b'0');
                                let x = at.x + (k as i32 - text.len() as i32 + 8) * cell as i32;
                                out.push(base + 2 + k as u32, json!({"rect":[x, at.y, cell, size[1]],"paint":{"kind":"image","asset":asset},
                                    "uv":[f64::from(digit * cell) / f64::from(size[0]), 0.0, f64::from((digit + 1) * cell) / f64::from(size[0]), 1.0]}));
                            }
                        }
                    }
                }
                Node::Map => {
                    if self.map.visible() {
                        for (i, b) in self.map.buttons.iter().enumerate() {
                            out.sprite(11_400 + i as u32, &b.face, zero);
                        }
                    }
                }
                Node::Choice => {
                    let Some(c) = &self.choice else { continue };
                    if !c.visible {
                        continue;
                    }
                    let mut items: Vec<(i32, usize, &crate::scene::ChoiceItem)> = Vec::new();
                    if let Some(q) = &c.question {
                        items.push((0, 0, q));
                    }
                    for (i, a) in c.answers.iter().enumerate() {
                        items.push((3 - i as i32, i + 1, a));
                    }
                    items.sort_by_key(|(p, i, _)| (*p, *i));
                    for (_, i, item) in items {
                        let base = 11_200 + i as u32 * 4;
                        out.sprite(base, &item.sprite, item.plate_at);
                        if item.sprite.visible {
                            let at = Point { x: item.sprite.pos.x + item.text_at.x, y: item.sprite.pos.y + item.text_at.y };
                            out.glyphs(base + 1, &item.text, None, &item.style, font, at, 1.0);
                        }
                    }
                }
            }
        }
        let [w, hgt] = [self.apini.width, self.apini.height];
        if let Some(d) = &self.edit {
            // The scene still behind the dialog, darkened by 80% unless the
            // APINI keeps its brightness (sub_445A50(80)).
            if !self.apini.keep_backdrop {
                out.push(12_100, json!({"rect":[0,0,w,hgt],"paint":{"kind":"solid","color":[0,0,0,204]}}));
            }
            for (i, s) in d.layers.iter().enumerate() {
                out.sprite(12_110 + i as u32, s, d.origin);
            }
            for (i, f) in d.fields.iter().enumerate() {
                let [fx, fy, fw, fh] = f.rect;
                let at = Point { x: d.origin.x + fx, y: d.origin.y + fy };
                out.push(12_200 + i as u32 * 2, json!({"rect":[at.x, at.y, fw.max(1), fh.max(1)],"paint":{"kind":"solid","color":[255,255,255,255]}}));
                let mut text = f.text.clone();
                if i == d.focus {
                    text.extend_from_slice(&encoding_rs::SHIFT_JIS.encode(&d.preedit).0);
                }
                let size = (fh - 2).max(1);
                let style = Style { size, width: fw.max(1), height: fh.max(1), color: 0, ..Default::default() };
                let mut block = Block::new(201);
                block.append(&text, &style);
                out.glyphs(12_201 + i as u32 * 2, &block, None, &style, font, at, 1.0);
            }
            for (k, b) in [&d.ok, &d.cancel].into_iter().enumerate() {
                if let Some(b) = b {
                    out.sprite(12_300 + k as u32, &b.face, d.origin);
                }
            }
        }
        if let Some(st) = &self.state_screen {
            if let Some(b) = &st.backdrop {
                out.sprite(12_390, b, zero);
            }
            if let Some(top) = &st.top {
                for (k, (sprite, _)) in top.buttons.iter().enumerate() {
                    out.sprite(12_391 + k as u32, sprite, zero);
                }
            }
            if let Some(g) = &st.gallery {
                self.draw_gallery(&mut out, g, font);
            }
        }
        if let (Some(c), false) = (&self.config_screen, save_shown) {
            self.draw_config(&mut out, c, font, w, hgt);
        }
        if let Some(sv) = self.save_screen.as_ref().filter(|_| save_shown) {
            if !self.apini.keep_backdrop && game {
                out.push(12_400, json!({"rect":[0,0,w,hgt],"paint":{"kind":"solid","color":[0,0,0,204]}}));
            }
            out.sprite(12_401, &sv.bg, zero);
            // Slots by priority 10 - index; parts by their priorities
            // (icons 0/1, date/text/hit area 1, thumbnail 2, marker 3).
            let last = self.mem.cfg(crate::memory::config::SAVE_SLOT) as u16;
            let page = self.mem.cfg(crate::memory::config::SAVE_PAGE) as u16;
            for i in (0..sv.slots.len()).rev() {
                let slot = &sv.slots[i];
                let o = slot.at;
                let base = 12_410 + i as u32 * 10;
                let image = |c: &Option<Content>, at: Point| Sprite { content: c.clone(), pos: at, visible: c.is_some(), ..Default::default() };
                if let Some(hd) = &slot.header {
                    out.sprite(base, &image(&slot.icon1, zero), o);
                    out.sprite(base + 1, &image(&slot.icon2, sv.layout.icon_at), o);
                    let color = self.apini.palette.get(usize::from(hd.vars[2])).copied().unwrap_or(0xFFFFFF);
                    let size = i32::from(self.apini.date_size).max(1);
                    let style = Style { size, width: 300, height: 30, color, ..Default::default() };
                    let mut block = Block::new(32);
                    block.append(&crate::save::date_text(hd.date), &style);
                    let at = Point { x: o.x + sv.layout.date_at.x, y: o.y + sv.layout.date_at.y };
                    out.glyphs(base + 2, &block, None, &style, font, at, 1.0);
                    if let Some((tp, [tw, th])) = sv.layout.text {
                        let size = i32::from(self.apini.save_text_size).max(1);
                        let style = Style { size, width: tw as i32, height: th as i32, color: self.apini.save_text_color, ..Default::default() };
                        let mut block = Block::new(81);
                        block.append(&hd.text, &style);
                        out.glyphs(base + 3, &block, None, &style, font, Point { x: o.x + tp.x, y: o.y + tp.y }, 1.0);
                    }
                }
                if slot.hovered {
                    let [sx, sy, sw, sh] = sv.slot_rect(i);
                    out.push(base + 4, json!({"rect":[sx, sy, sw.max(1), sh.max(1)],"paint":{"kind":"solid","color":[255,255,255,255]},"blend":"inverse_destination"}));
                }
                if slot.header.is_some() {
                    if let Some(at) = sv.layout.thumb_at {
                        out.sprite(base + 5, &image(&slot.thumb, at), o);
                    }
                    if let Some((m, at)) = &sv.layout.new_marker {
                        if last == page * 10 + i as u16 + 1 {
                            out.sprite(base + 6, &image(&Some(m.clone()), *at), o);
                        }
                    }
                }
            }
            for (k, b) in [&sv.exit, &sv.next, &sv.prev].into_iter().enumerate() {
                if let Some(b) = b {
                    out.sprite(12_520 + k as u32, &b.face, zero);
                }
            }
            if let (Some(at), Some(Some((c, d)))) = (sv.number_at, sv.digits.get(usize::from(page))) {
                let s = Sprite { content: Some(c.clone()), pos: *d, visible: true, ..Default::default() };
                out.sprite(12_523, &s, at);
            }
            for (k, b) in sv.tabs.iter().enumerate() {
                out.sprite(12_530 + k as u32, &b.face, zero);
            }
        }
        if let Some(path) = &self.movie {
            out.push(12_000, json!({"rect":[0,0,w,hgt],"paint":{"kind":"image","asset":path}}));
        }
        if let Some(c) = self.dip.or(self.flash) {
            let [r, g, b] = rgb(c);
            out.push(12_001, json!({"rect":[0,0,w,hgt],"paint":{"kind":"solid","color":[r,g,b,255]}}));
        }
        if let Some(z) = &self.zoom {
            self.presented.zoom_order = out.draws.len() as i32;
            let image = |asset: &str, r: [i32; 4]| json!({"rect":r,"paint":{"kind":"image","asset":asset,"tint":[255,255,255,255]}});
            match z.kind {
                // sub_453A60: each step at half level over the result so far.
                crate::zoom::Kind::Zoom => {
                    for (k, (asset, r)) in z.trail.iter().enumerate() {
                        let mut l = image(asset, *r);
                        if k > 0 {
                            l["opacity"] = json!(128);
                        }
                        out.push(crate::zoom::LAYER_BASE + k as u32, l);
                    }
                }
                _ => {
                    out.push(crate::zoom::LAYER_BASE, json!({"rect":[0,0,w,hgt],"paint":{"kind":"solid","color":[0,0,0,255]}}));
                    if let (Some((r, _)), Some((asset, _))) = (z.rect([w as i32, hgt as i32]), &z.image) {
                        out.push(crate::zoom::LAYER_BASE + 1, image(asset, r));
                    }
                    // sub_44FCF0: the frame darkens by the step's level.
                    if let Some(d) = z.darkness() {
                        out.push(crate::zoom::LAYER_BASE + 2, json!({"rect":[0,0,w,hgt],"paint":{"kind":"solid","color":[0,0,0,255]},"opacity":d}));
                    }
                }
            }
        }
        let mut ops = Vec::new();
        if let Some(t) = self.transition.take() {
            ops.push(json!({"op":"transition","transition":t}));
        }
        let mut next = BTreeMap::new();
        for (id, layer) in out.draws {
            if self.presented.layers.get(&id) != Some(&layer) {
                ops.push(json!({"op":"put","layer":layer.clone()}));
            }
            next.insert(id, layer);
        }
        for id in self.presented.layers.keys() {
            if !next.contains_key(id) {
                ops.push(json!({"op":"remove","id":id}));
            }
        }
        self.presented.layers = next;
        if !ops.is_empty() {
            let bytes = serde_json::to_vec(&json!({"layers":ops}))?;
            h.submit_engine(self.client, 1, &bytes)?;
        }
        Ok(())
    }
}

impl Vm {
    /// Draw order of a reserved layer in the last presentation.
    pub(crate) fn presented_order(&self, _id: u32) -> i32 {
        self.presented.zoom_order
    }
    /// CConfig over the scene (sub_41B170): the scene darkens unless the
    /// APINI keeps it, the main page or a tab page, the font window on top.
    fn draw_config(&self, out: &mut Out, c: &crate::config::ConfigScreen, font: &str, w: u32, hgt: u32) {
        let zero = Point::default();
        if !self.apini.keep_backdrop && self.state_screen.is_none() {
            out.push(13_000, json!({"rect":[0,0,w,hgt],"paint":{"kind":"solid","color":[0,0,0,204]}}));
        }
        out.sprite(13_001, &c.bg, zero);
        let mut id = 13_010;
        let mut put = |out: &mut Out, s: &Sprite, origin: Point| {
            out.sprite(id, s, origin);
            id += 1;
        };
        for b in [&c.save, &c.load, &c.exit, &c.title, &c.close].into_iter().flatten() {
            put(out, &b.face, zero);
        }
        for b in c.tabs.iter().flatten() {
            put(out, &b.face, zero);
        }
        if c.main_page() {
            for r in &c.radios {
                for b in &r.buttons {
                    put(out, &b.face, zero);
                }
            }
            for s in &c.sliders {
                put(out, &s.knob.face, zero);
            }
            for g in &c.banks {
                for b in g {
                    put(out, &b.face, zero);
                }
            }
            for b in c.bank_tests.iter().flatten().chain([&c.test_music, &c.test_sound, &c.test_voice, &c.font].into_iter().flatten()) {
                put(out, &b.face, zero);
            }
            if let Some((at, [tw, th])) = c.font_text_at {
                let name = encoding_rs::SHIFT_JIS.encode(self.text_font.as_deref().unwrap_or(font)).0.into_owned();
                let style = Style { size: th as i32, width: tw as i32, height: th as i32, color: 0xFFFFFF, ..Default::default() };
                let mut block = Block::new(64);
                block.append(&name, &style);
                out.glyphs(13_400, &block, None, &style, font, at, 1.0);
            }
        } else {
            let p = &c.pages;
            put(out, if c.tab == 1 { &p.status_bg } else { &p.desig_bg }, zero);
            for (k, col) in p.columns.iter().enumerate() {
                put(out, &col.image, zero);
                let style = Style { size: 16, width: 200, height: 100, color: 0xFFFFFF, palette: p.text.as_ref().map_or([0; 10], |t| t.palette), ..Default::default() };
                let mut block = Block::new(256);
                block.append(&col.upper, &style);
                out.glyphs(13_410 + 2 * k as u32, &block, None, &style, font, col.upper_at, 1.0);
                let style = Style { width: 250, ..style };
                let mut block = Block::new(256);
                block.append(&col.lower, &style);
                out.glyphs(13_411 + 2 * k as u32, &block, None, &style, font, col.lower_at, 1.0);
            }
            for (a, b) in &p.marks {
                put(out, a, zero);
                put(out, b, zero);
            }
        }
        if let Some(fw) = c.font_window.as_ref().filter(|f| f.visible) {
            let o = fw.origin;
            out.sprite(13_500, &fw.bg, o);
            for (k, b) in fw.rows.iter().chain(fw.caps.iter()).enumerate() {
                out.sprite(13_501 + k as u32, &b.face, o);
            }
            for (k, b) in [&fw.prev, &fw.next, &fw.exit].into_iter().flatten().enumerate() {
                out.sprite(13_530 + k as u32, &b.face, o);
            }
            // Each row: the face name, then a sample drawn in that face.
            let size = (fw.row_size[1] as i32 - 4).max(8);
            for row in 0..10 {
                let Some(alias) = c.fonts.get(fw.page * 10 + row) else { break };
                let name = encoding_rs::SHIFT_JIS.encode(alias).0.into_owned();
                let style = Style { size, width: fw.row_size[0] as i32, height: size + 4, color: 0xFFFFFF, ..Default::default() };
                let dy = 22 * row as i32;
                let mut block = Block::new(64);
                block.append(&name, &style);
                out.glyphs(13_540 + 2 * row as u32, &block, None, &style, font, Point { x: o.x + fw.name_at.x, y: o.y + fw.name_at.y + dy }, 1.0);
                let mut block = Block::new(64);
                block.append(&name, &style);
                out.glyphs(13_541 + 2 * row as u32, &block, None, &style, alias, Point { x: o.x + fw.sample_at.x, y: o.y + fw.sample_at.y + dy }, 1.0);
            }
        }
    }
    fn draw_gallery(&self, out: &mut Out, g: &crate::gallery::Gallery, font: &str) {
        for (k, d) in g.draws().into_iter().enumerate() {
            match d {
                crate::gallery::Draw::Sprite(s, origin) => out.sprite(14_000 + k as u32, &s, origin),
                crate::gallery::Draw::Text(text, style, at) => {
                    let mut block = Block::new(256);
                    block.append(&text, &style);
                    out.glyphs(14_000 + k as u32, &block, None, &style, font, at, 1.0);
                }
            }
        }
    }
}

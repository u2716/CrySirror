// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Visible scene objects rebuilt from the native records: script layers,
//! text boxes, the companion panel and the choice window. Records in `Memory`
//! stay authoritative; these objects carry only presentation and animation.
use crate::apini::Apini;
use crate::memory::{Memory, layer as L, message as M, scene as S, text_box as T};
use crate::res::{Resources, join};
use crate::sprite::{Content, Point, Sprite, pt};
use crate::text::{Block, Reveal, Style};
use crysirror_core::{service::HostServices, state::ClientId};

pub struct Ctx<'a> {
    pub h: &'a mut dyn HostServices,
    pub client: ClientId,
    pub res: &'a mut Resources,
    pub apini: &'a Apini,
}
impl Ctx<'_> {
    pub fn image(&mut self, native: &str) -> Option<Content> {
        self.res.image(self.h, self.client, native)
    }
    pub fn lwg_layer(&mut self, native: &str, name: &str) -> Option<(Content, Point)> {
        self.res.lwg_layer(self.h, self.client, native, name)
    }
    pub fn size(&self) -> [i32; 2] {
        [self.apini.width as i32, self.apini.height as i32]
    }
}

// ---------------------------------------------------------------- layers

#[derive(Default)]
pub struct Layer {
    pub index: usize,
    pub visible: bool,
    pub main: Sprite,
    /// Crossfade source / hover images (button slots 0 and 1).
    pub under: [Sprite; 2],
    /// Hover image slot 2.
    pub special: Sprite,
    /// Kind 2: the LWG frames and their `.fsc` flow (CMainCelAnim).
    pub anim: Option<crate::anim::Anim>,
    /// Kind 0 load conversion in flight (CMainCelLim sub_415810).
    pub convert: Option<crysirror_core::image_processing::ImageJob>,
    /// Kind 0 picture drawn over the image (record +32, sub_416460).
    pub overlay: Sprite,
}

fn rec(i: usize) -> usize {
    S::LAYERS + i * S::LAYER_STRIDE
}

impl Layer {
    pub fn new(index: usize) -> Self {
        Self { index, visible: true, ..Default::default() }
    }
    fn r(&self) -> usize {
        rec(self.index)
    }
    pub fn directory(&self, mem: &Memory) -> String {
        encoding_rs::SHIFT_JIS.decode(mem.str_at(self.r() + L::DIRECTORY, 23)).0.into_owned()
    }
    fn image_path(&self, mem: &Memory, image: u16) -> String {
        join(&self.directory(mem), &format!("{image:04}"))
    }
    /// Placement: anchors and edge clamping for a sprite size.
    fn place(&self, mem: &Memory, x: i32, y: i32, s: &Sprite, screen: [i32; 2]) -> Point {
        let (mut x, mut y) = (x, y);
        if mem.d(self.r() + L::CENTER_X) != 0 {
            x -= ((s.width() as u32) >> 1) as i32;
        }
        if mem.d(self.r() + L::CENTER_Y) != 0 {
            y -= ((s.height() as u32) >> 1) as i32;
        }
        if mem.d(self.r() + L::KEEP_ON_SCREEN) != 0 {
            x = x.max(0);
            y = y.max(0);
            if x + s.width() > screen[0] {
                x = screen[0] - s.width();
            }
            if y + s.height() > screen[1] {
                y = screen[1] - s.height();
            }
        }
        pt(x, y)
    }
    /// The record's current source as a sprite content.
    fn load_source(&mut self, mem: &Memory, cx: &mut Ctx) -> bool {
        let r = self.r();
        let source = mem.uw(r + L::SOURCE);
        let content = match source {
            3 => Some(Content::Fill { color: mem.d(r + L::COLOR) as u32 & 0xFFFFFF, size: [cx.apini.width, cx.apini.height] }),
            2 => {
                let text = mem.str_at(r + L::NAME, 81).to_vec();
                let cell = match mem.uw(r + L::PRIORITY) {
                    0 => 30,
                    s => i32::from(s),
                };
                Some(text_content(text, cell, false))
            }
            _ => {
                let image = mem.uw(r + L::IMAGE);
                if image == 0 {
                    None
                } else {
                    let path = self.image_path(mem, image);
                    if mem.uw(r + L::KIND) == 2 {
                        // sub_402280: every LWG layer is a frame.
                        let frames: Vec<(Content, Point)> = match cx.res.lwg_of(&path).cloned() {
                            Some(lwg) => lwg.layers.iter().filter_map(|l| cx.lwg_layer(&path, &l.name)).collect(),
                            None => Vec::new(),
                        };
                        let flow = cx.res.file(&format!("{path}.fsc")).cloned().and_then(|asset| {
                            let p = crysirror_core::path::VfsPath::new(crysirror_core::path::Mount::Asset, &asset).ok()?;
                            cx.h.storage(cx.client).ok()?.read(&p).ok()
                        });
                        let anim = crate::anim::Anim::new(frames, flow.map(|b| crate::anim::parse(&b)));
                        let first = anim.current().map(|f| f.0.clone());
                        if first.is_none() {
                            cx.res.diag(format!("missing animated layer {path}.lwg"));
                        }
                        self.anim = Some(anim);
                        first
                    } else {
                        self.anim = None;
                        let c = cx.image(&path);
                        self.convert = None;
                        self.overlay = Sprite::default();
                        if mem.uw(r + L::KIND) == 0 {
                            self.start_convert(mem, cx, c.as_ref());
                            let extra = mem.uw(r + L::OVERLAY_IMAGE);
                            if extra != 0 {
                                let content = cx.image(&join(&self.directory(mem), &format!("{extra:04}")));
                                self.overlay = Sprite { visible: content.is_some(), content, ..Default::default() };
                            }
                        }
                        c
                    }
                }
            }
        };
        let ok = content.is_some();
        self.main.stop_animation();
        self.main.set_content(content);
        self.apply_blend(mem);
        ok
    }
    /// Numbered image with entrance effect.
    pub fn load(&mut self, mem: &mut Memory, cx: &mut Ctx, image: u16, effect: i32, x: i32, y: i32, flags: i32, skipping: bool) {
        self.stop_button(mem, true, skipping);
        let r = self.r();
        mem.set_w(r + L::IMAGE, i32::from(image));
        mem.set_w(r + L::SOURCE, 0);
        mem.set_d(r + L::X, x);
        mem.set_d(r + L::Y, y);
        mem.set_w(r + L::LOAD_FLAGS, flags);
        mem.set_w(r + L::PATTERN, 0);
        if skipping {
            return;
        }
        let previous = (self.main.visible, self.main.content.clone(), self.main.pos);
        if !self.load_source(mem, cx) {
            return;
        }
        if previous.0 && previous.1.is_some() {
            self.under[0].content = previous.1;
            self.under[0].pos = previous.2;
        }
        self.under[0].visible = false;
        self.appear(mem, cx.size(), effect, x, y, previous.0);
    }
    /// A text layer from a script string.
    #[allow(clippy::too_many_arguments)]
    pub fn load_text(&mut self, mem: &mut Memory, cx: &mut Ctx, text: &[u8], effect: i32, x: i32, y: i32, shadow: bool, skipping: bool) {
        self.stop_button(mem, true, skipping);
        let r = self.r();
        mem.set_w(r + L::IMAGE, 0);
        mem.set_w(r + L::SOURCE, 2);
        mem.set_d(r + L::X, x);
        mem.set_d(r + L::Y, y);
        mem.set_str(r + L::NAME, 81, text);
        mem.set_w(r + L::PATTERN, 0);
        if skipping {
            return;
        }
        let previous = self.main.visible;
        self.load_source(mem, cx);
        if let Some(Content::Text { shadow: s, .. }) = &mut self.main.content {
            *s = shadow;
        }
        self.appear(mem, cx.size(), effect, x, y, previous);
    }
    /// A numbered image placed at the current origin.
    pub fn load_at_origin(&mut self, mem: &mut Memory, cx: &mut Ctx, image: u16, effect: i32, flags: i32, skipping: bool) {
        self.stop_button(mem, true, skipping);
        let r = self.r();
        mem.set_w(r + L::IMAGE, i32::from(image));
        mem.set_w(r + L::SOURCE, 0);
        mem.set_w(r + L::LOAD_FLAGS, flags);
        mem.set_w(r + L::PATTERN, 0);
        if skipping {
            return;
        }
        let previous = self.main.visible;
        if !self.load_source(mem, cx) {
            return;
        }
        let mut at = self.main.pos;
        if mem.d(r + L::CENTER_X) != 0 {
            at.x += ((self.main.width() as u32) >> 1) as i32;
        }
        if mem.d(r + L::CENTER_Y) != 0 {
            at.y += ((self.main.height() as u32) >> 1) as i32;
        }
        self.appear(mem, cx.size(), effect, at.x, at.y, previous);
    }
    /// The animation pattern of a layer holding an image or source.
    pub fn set_pattern(&mut self, mem: &mut Memory, _res: &mut crate::res::Resources, pattern: i32, _skipping: bool) {
        let r = self.r();
        if mem.uw(r + L::IMAGE) == 0 && mem.uw(r + L::SOURCE) == 0 {
            return;
        }
        mem.set_w(r + L::PATTERN, pattern);
    }
    /// sub_402650 each tick: the next frame of an animated layer.
    pub fn tick_anim(&mut self, var: &dyn Fn(usize) -> i16, rand: &mut dyn FnMut() -> i32) -> bool {
        let Some(a) = self.anim.as_mut() else { return false };
        if !self.visible || !a.tick(var, rand) {
            return false;
        }
        let content = a.current().map(|f| f.0.clone());
        if content != self.main.content {
            self.main.content = content;
            return true;
        }
        false
    }
    /// sub_415810: mirror (+26), zoom (+28), rotation (+30) and the load
    /// flags' tone conversion run on the shared image worker.
    fn start_convert(&mut self, mem: &Memory, cx: &mut Ctx, content: Option<&Content>) {
        let r = self.r();
        let spec = crysirror_image::cel_convert::CelConvert {
            mirror: mem.uw(r + L::MIRROR) as u8,
            percent: mem.uw(r + L::ZOOM),
            degrees: mem.uw(r + L::ROTATION),
            tone: mem.uw(r + L::LOAD_FLAGS),
        };
        let identity = spec.mirror & 3 == 0 && spec.degrees == 0 && matches!(spec.percent, 0 | 100) && spec.tone == 0;
        let Some(Content::Image { asset, size }) = content.filter(|_| !identity) else { return };
        let effects = match serde_json::to_vec(&[crysirror_image::effects::ImageEffect::CelConvert { spec }]) {
            Ok(e) => e,
            Err(e) => return cx.res.diag(format!("layer conversion: {e}")),
        };
        let input = crysirror_core::image_processing::ImageInput::Asset { path: asset.clone(), size: *size, fingerprint: None };
        match cx.h.image_metadata(cx.client).and_then(|m| m.begin_process(input, effects.into())) {
            Ok(job) => self.convert = Some(job),
            Err(e) => cx.res.diag(format!("layer {} conversion: {e}", self.index)),
        }
    }
    /// The converted picture replaces the loaded one and is placed again.
    pub fn finish_convert(&mut self, mem: &Memory, content: Content, screen: [i32; 2]) {
        self.convert = None;
        self.main.content = Some(content);
        let r = self.r();
        let main = self.main.clone();
        self.main.pos = self.place(mem, mem.d(r + L::X), mem.d(r + L::Y), &main, screen);
    }
    /// Offset of the shown frame inside the LWG canvas.
    pub fn frame_offset(&self) -> Point {
        self.anim.as_ref().and_then(|a| a.current()).map_or_else(Point::default, |f| f.1)
    }
    /// Full-screen colour.
    pub fn fill(&mut self, mem: &mut Memory, cx: &mut Ctx, color: u32, skipping: bool) {
        self.stop_button(mem, true, skipping);
        let r = self.r();
        mem.set_w(r + L::IMAGE, 0);
        mem.set_w(r + L::SOURCE, 3);
        mem.set_d(r + L::COLOR, color as i32);
        mem.set_w(r + L::PATTERN, 0);
        if skipping {
            return;
        }
        if self.load_source(mem, cx) {
            self.appear(mem, cx.size(), 0, 0, 0, false);
        }
    }
    /// Shows the main sprite with one of the entrance effects.
    fn appear(&mut self, mem: &Memory, screen: [i32; 2], effect: i32, x: i32, y: i32, had_image: bool) {
        let target = self.place(mem, x, y, &self.main, screen);
        let m = &mut self.main;
        m.stop_animation();
        let (w, h) = (m.width(), m.height());
        let [sw, sh] = screen;
        let slide = |m: &mut Sprite, from: Point, distance: i32| {
            m.pos = from;
            m.curved(from, target, (distance & 0xFFFF) / 15, true);
        };
        let fade = |m: &mut Sprite, mode: u8, step: i32| {
            m.pos = target;
            m.start_fade(mode, 0, step);
        };
        let hyp = |a: i32, b: i32| f64::from(a).hypot(f64::from(b)) as i32;
        match effect {
            2 => {
                if had_image {
                    self.under[0].visible = true;
                    self.under[0].start_fade(2, 1, 32);
                    self.under[0].animating = true;
                }
                fade(m, 2, 32);
            }
            3 => fade(m, 2, 32),
            4 => fade(m, 0x12, 32),
            5 => fade(m, 4, 32),
            6 => fade(m, 3, 32),
            7 => slide(m, pt(-w, target.y), target.x + w),
            8 => slide(m, pt(sw, target.y), sw - target.x),
            9 => slide(m, pt(target.x, -h), target.y + h),
            10 => slide(m, pt(target.x, sh), sh - target.y),
            11 => slide(m, pt(-w, -h), hyp(target.x + w, target.y + h)),
            12 => slide(m, pt(sw, -h), hyp(sw - target.x, target.y + h)),
            13 => slide(m, pt(-w, sh), hyp(target.x + w, sh - target.y)),
            14 => slide(m, pt(sw, sh), hyp(sw - target.x, sh - target.y)),
            15 => fade(m, 4, 16),
            16 => fade(m, 2, 24),
            17 => fade(m, 0x14, 16),
            18 => fade(m, 0x15, 16),
            19 => fade(m, 2, 8),
            20 => {
                m.pos = pt(target.x, target.y - 150);
                m.start_fade(2, 0, 8);
                m.linear(m.pos, target, 5);
            }
            21..=23 => {
                let from = match effect {
                    21 => pt(target.x, target.y + 150),
                    22 => pt(target.x - 150, target.y),
                    _ => pt(target.x + 150, target.y),
                };
                m.pos = from;
                m.start_fade(2, 0, 16);
                m.linear(from, target, 10);
            }
            _ => m.pos = target,
        }
        m.show(true);
        m.animating = effect != 0;
    }
    /// Clears the image with an exit effect.
    pub fn hide(&mut self, mem: &mut Memory, screen: [i32; 2], effect: i32, skipping: bool) {
        let r = self.r();
        if mem.uw(r + L::IMAGE) == 0 && mem.uw(r + L::SOURCE) == 0 {
            return;
        }
        mem.set_w(r + L::IMAGE, 0);
        mem.set_w(r + L::SOURCE, 0);
        mem.set_w(r + L::PATTERN, 0);
        self.stop_button(mem, true, skipping);
        if skipping || !self.main.visible {
            return;
        }
        let m = &mut self.main;
        m.stop_animation();
        let (w, h, p) = (m.width(), m.height(), m.pos);
        let [sw, sh] = screen;
        let hyp = |a: i32, b: i32| f64::from(a).hypot(f64::from(b)) as i32;
        let slide = |m: &mut Sprite, to: Point, distance: i32| {
            m.curved(m.pos, to, (distance & 0xFFFF) / 15, true);
            m.animating = true;
        };
        let out = |m: &mut Sprite, mode: u8, step: i32| {
            m.start_fade(mode, 1, step);
            m.animating = true;
        };
        match effect {
            3 => out(m, 2, 32),
            4 => out(m, 0x12, 32),
            5 => out(m, 4, 32),
            6 => out(m, 3, 32),
            7 => slide(m, pt(-w, p.y), p.x + w),
            8 => slide(m, pt(sw, p.y), sw - p.x),
            9 => slide(m, pt(p.x, -h), p.y + h),
            10 => slide(m, pt(p.x, sh), sh - p.y),
            11 => slide(m, pt(-w, -h), hyp(p.x + w, p.y + h)),
            12 => slide(m, pt(sw, -h), hyp(sw - p.x, p.y + h)),
            13 => slide(m, pt(-w, sh), hyp(p.x + w, sh - p.y)),
            14 => slide(m, pt(sw, sh), hyp(sw - p.x, sh - p.y)),
            15 => out(m, 4, 16),
            16 => out(m, 2, 24),
            17 => out(m, 0x14, 16),
            18 => out(m, 0x15, 16),
            19 => out(m, 2, 8),
            20..=23 => {
                let to = match effect {
                    20 => pt(p.x, p.y - 150),
                    21 => pt(p.x, p.y + 150),
                    22 => pt(p.x - 150, p.y),
                    _ => pt(p.x + 150, p.y),
                };
                m.start_fade(2, 1, 16);
                m.linear(p, to, 10);
                m.animating = true;
            }
            _ => m.visible = false,
        }
    }
    /// Moves the main sprite.
    pub fn move_to(&mut self, mem: &mut Memory, screen: [i32; 2], effect: i32, x: i32, y: i32, speed: i32, skipping: bool) {
        let kind = effect % 100;
        let r = self.r();
        if (mem.uw(r + L::IMAGE) != 0 || mem.uw(r + L::SOURCE) != 0) && kind < 7 {
            mem.set_d(r + L::X, x);
            mem.set_d(r + L::Y, y);
        }
        if skipping {
            return;
        }
        let target = self.place(mem, x, y, &self.main, screen);
        let m = &mut self.main;
        let distance = f64::from(m.pos.x - target.x).hypot(f64::from(m.pos.y - target.y)) as i32;
        let pace = (distance / if speed == 0 { 15 } else { speed }).max(1);
        m.motion_finish();
        let from = m.pos;
        match kind {
            0 => {
                m.pos = target;
                return;
            }
            1 => m.linear(from, target, pace),
            2 => m.curved(from, target, pace, false),
            3 => m.curved(from, target, pace, true),
            4 => m.arc(from, target, pace, 2, 3),
            5 | 6 => m.swing(from, target, pace, kind == 5),
            7 | 9 => m.bounce(from, 5, 30, 25, if kind == 9 { 1 } else { 0 }),
            8 | 10 => m.bounce(from, 5, 5, 4, if kind == 10 { 1 } else { 0 }),
            11 => m.hop(from, 40, 10),
            12 => m.hop(from, 100, 10),
            13 => m.hop(from, 300, 20),
            _ => return,
        }
        m.animating = true;
    }
    pub fn set_visible(&mut self, mem: &mut Memory, visible: i32, skipping: bool) {
        mem.set_d(self.r() + L::VISIBLE, visible & 0xFFFF);
        if !skipping {
            self.visible = visible != 0;
        }
    }
    /// Blend presets with a percentage level.
    pub fn set_blend(&mut self, mem: &mut Memory, mode: i32, percent: i32, skipping: bool) {
        mem.set_w(self.r() + L::BLEND_MODE, mode);
        mem.set_w(self.r() + L::BLEND_VALUE, percent);
        if !skipping {
            self.apply_blend(mem);
        }
    }
    pub fn apply_blend(&mut self, mem: &Memory) {
        if self.main.fade.active {
            return;
        }
        self.main.mode = match mem.w(self.r() + L::BLEND_MODE) {
            1 => 2,
            2 => 14,
            3 => 104,
            4 => 3,
            5 => 4,
            _ => 0,
        };
        self.main.level = (255 * i32::from(mem.uw(self.r() + L::BLEND_VALUE)) / 100) as u8;
    }
    /// The next load uses the new kind.
    pub fn set_kind(&mut self, mem: &mut Memory, screen: [i32; 2], kind: i32, skipping: bool) {
        self.hide(mem, screen, 0, skipping);
        mem.set_w(self.r() + L::KIND, kind);
    }
    pub fn set_button(&mut self, mem: &mut Memory, mode: i32, hover: i32, id: i32, keep: i32, skipping: bool) {
        let r = self.r();
        mem.set_d(r + L::BUTTON, 1);
        mem.set_w(r + L::HOVER_STYLE, hover);
        mem.set_w(r + L::BUTTON_ID, id);
        mem.set_w(r + L::BUTTON_MODE, mode);
        mem.set_w(r + L::KEEP_HOVER, keep);
        if !skipping {
            self.main.interactive = true;
        }
    }
    pub fn is_button(&self, mem: &Memory) -> bool {
        mem.d(self.r() + L::BUTTON) != 0
    }
    pub fn button_mode(&self, mem: &Memory) -> u16 {
        mem.uw(self.r() + L::BUTTON_MODE)
    }
    pub fn button_id(&self, mem: &Memory) -> i16 {
        mem.w(self.r() + L::BUTTON_ID)
    }
    /// Disables the button (mode-2 buttons only when `all`).
    pub fn stop_button(&mut self, mem: &mut Memory, all: bool, skipping: bool) {
        let r = self.r();
        if mem.d(r + L::BUTTON) == 0 || (!all && mem.uw(r + L::BUTTON_MODE) == 2) {
            return;
        }
        mem.set_d(r + L::BUTTON, 0);
        for slot in 0..3 {
            mem.set_w(r + L::OVERLAYS + slot * 2, 0);
        }
        if skipping {
            return;
        }
        self.main.interactive = false;
        self.hover(mem, false);
        for s in self.under.iter_mut().chain([&mut self.special]) {
            s.visible = false;
        }
    }
    pub fn set_button_input(&mut self, mem: &Memory, enabled: bool, only_interrupts: bool) {
        if !self.is_button(mem) || (only_interrupts && self.button_mode(mem) != 2) {
            return;
        }
        self.main.interactive = enabled;
        if !enabled && self.main.hovered {
            self.hover(mem, false);
        }
    }
    pub fn set_overlay(&mut self, mem: &mut Memory, cx: &mut Ctx, slot: usize, image: u16, x: i32, y: i32, skipping: bool) {
        if slot > 2 {
            return;
        }
        let r = self.r();
        mem.set_w(r + L::OVERLAYS + slot * 2, i32::from(image));
        mem.set_d(r + L::OVERLAY_POSITIONS + slot * 8, x);
        mem.set_d(r + L::OVERLAY_POSITIONS + slot * 8 + 4, y);
        if skipping {
            return;
        }
        let content = if image != 0 { cx.image(&self.image_path(mem, image)) } else { None };
        if slot == 2 && content.is_some() {
            cx.res.diag("button hover animation slot 2 shows its first frame only");
        }
        let s = if slot == 2 { &mut self.special } else { &mut self.under[slot] };
        s.set_content(content);
        s.pos = pt(x, y);
        s.visible = false;
    }
    /// Hover in/out of a button (style 1 inverts, 2 adds at 64, 3 selects
    /// blend 0x17, which the Shapeshifter build's dispatcher 0x4464E0 does not
    /// draw; overlays 0..2 show while hovered).
    pub fn hover(&mut self, mem: &Memory, inside: bool) {
        let r = self.r();
        self.main.hovered = inside;
        if inside {
            match mem.uw(r + L::HOVER_STYLE) {
                1 => self.main.mode = 13,
                2 => {
                    self.main.mode = 4;
                    self.main.level = 64;
                }
                3 => self.main.mode = 23,
                _ => {}
            }
        } else {
            self.apply_blend(mem);
        }
        for slot in 0..2 {
            if mem.uw(r + L::OVERLAYS + slot * 2) != 0 {
                self.under[slot].visible = inside && self.under[slot].content.is_some();
            }
        }
        if mem.uw(r + L::OVERLAYS + 4) != 0 {
            self.special.visible = inside && self.special.content.is_some();
        }
    }
    /// Press: restores the face and drops overlay 1 unless kept.
    pub fn press(&mut self, mem: &Memory) {
        self.apply_blend(mem);
        if mem.uw(self.r() + L::KEEP_HOVER) == 0 {
            self.under[1].visible = false;
        }
    }
    /// Rebuild from the record after a load, skip or nested call.
    pub fn restore(&mut self, mem: &Memory, cx: &mut Ctx) {
        let r = self.r();
        self.main.stop_animation();
        for s in self.under.iter_mut().chain([&mut self.special]) {
            s.stop_animation();
            s.visible = false;
        }
        self.visible = mem.d(r + L::VISIBLE) != 0;
        let source = mem.uw(r + L::SOURCE);
        if source == 0 && mem.uw(r + L::IMAGE) == 0 {
            self.main.set_content(None);
        } else if self.load_source(mem, cx) {
            let fill = source == 3;
            let target = self.place(mem, if fill { 0 } else { mem.d(r + L::X) }, if fill { 0 } else { mem.d(r + L::Y) }, &self.main, cx.size());
            self.main.pos = target;
            self.main.show(true);
        }
        self.main.interactive = self.is_button(mem);
        for slot in 0..3 {
            let image = mem.uw(r + L::OVERLAYS + slot * 2);
            if image == 0 {
                continue;
            }
            let content = cx.image(&self.image_path(mem, image));
            let s = if slot == 2 { &mut self.special } else { &mut self.under[slot] };
            s.set_content(content);
            s.pos = pt(mem.d(r + L::OVERLAY_POSITIONS + slot * 8), mem.d(r + L::OVERLAY_POSITIONS + slot * 8 + 4));
            s.visible = false;
        }
    }
    pub fn tick(&mut self) -> bool {
        let mut running = false;
        if self.visible {
            let [u0, u1] = &mut self.under;
            for s in [&mut self.main, u0, u1, &mut self.special] {
                running |= s.tick();
            }
        }
        running
    }
}

/// A text layer's content: one line of fixed cells (`cell` px high).
pub fn text_content(text: Vec<u8>, cell: i32, shadow: bool) -> Content {
    let style = Style { size: cell.max(1), width: i32::MAX / 4, height: cell.max(1), color: 0xFFFFFF, ..Default::default() };
    let mut b = Block::new(256);
    b.append(&text, &style);
    let w = b.glyphs.iter().map(|g| g.x + g.advance).max().unwrap_or(0).max(1);
    Content::Text { text, cell: cell.max(1), shadow, size: [w as u32, cell.max(1) as u32] }
}

// ------------------------------------------------------------- text boxes

#[derive(Default)]
pub struct TextBox {
    /// Scene offset of the record in use.
    pub record: usize,
    pub visible: bool,
    pub pos: Point,
    pub back: Sprite,
    pub front: Sprite,
    pub plate: Sprite,
    pub wait_body: Sprite,
    pub wait_glow: Sprite,
    pub wait_pos: Point,
    pub waiting: bool,
    pub text: Block,
    pub text_pos: Point,
    pub style: Style,
    pub reveal: Reveal,
    pub name: Block,
    pub name_pos: Point,
    pub name_style: Style,
    pub name_visible: bool,
    frame_loaded: Option<u16>,
    wait_loaded: Option<u16>,
    pub back_alpha: u8,
}

impl TextBox {
    fn style(&self, mem: &Memory, apini: &Apini, name: bool) -> Style {
        let r = self.record;
        Style {
            face: mem.uw(r + T::FACE),
            size: i32::from(mem.uw(r + T::SIZE)).max(1),
            color: mem.d(r + T::COLOR) as u32 & 0xFFFFFF,
            palette: apini.palette,
            shadow: apini.text_shadow,
            speed: i32::from(mem.uw(r + T::SPEED)),
            line_spacing: i32::from(mem.uw(r + T::LINE_SPACING)),
            char_spacing: i32::from(mem.uw(r + T::CHAR_SPACING)),
            indent: if name { 0 } else { i32::from(mem.uw(r + T::INDENT)) },
            first_indent: if name { 0 } else { i32::from(mem.w(r + T::FIRST_INDENT)) },
            align: i32::from(mem.uw(r + T::ALIGN)),
            width: mem.d(r + if name { T::NAME_WIDTH } else { T::TEXT_WIDTH }),
            height: mem.d(r + if name { T::NAME_HEIGHT } else { T::TEXT_HEIGHT }),
            ruby_face: mem.uw(r + T::RUBY_FACE),
            ruby_size: match mem.uw(r + T::RUBY_SIZE) {
                0 => 12,
                s => i32::from(s),
            },
            ruby_raise: i32::from(mem.uw(r + T::RUBY_RAISE)),
            baseline_at_bottom: false,
        }
    }
    fn load_layer(cx: &mut Ctx, s: &mut Sprite, lwg: &str, name: &str) -> bool {
        match cx.lwg_layer(lwg, name) {
            Some((content, at)) => {
                s.set_content(Some(content));
                s.pos = at;
                s.visible = true;
                true
            }
            None => {
                s.set_content(None);
                false
            }
        }
    }
    /// Applies the whole record to the visible objects.
    pub fn apply(&mut self, mem: &Memory, cx: &mut Ctx) {
        let r = self.record;
        let sys = cx.apini.dir(crate::apini::Dir::System).to_owned();
        let frame = mem.uw(r + T::FRAME);
        if self.frame_loaded != Some(frame) {
            self.frame_loaded = Some(frame);
            let lwg = join(&sys, &format!("tbox{frame:02}"));
            Self::load_layer(cx, &mut self.front, &lwg, "front");
            Self::load_layer(cx, &mut self.back, &lwg, "back");
            Self::load_layer(cx, &mut self.plate, &lwg, "plate");
            self.back.mode = 2;
        }
        let icon = mem.uw(r + T::WAIT_ICON);
        if self.wait_loaded != Some(icon) {
            self.wait_loaded = Some(icon);
            let lwg = join(&sys, &format!("wait{icon:02}"));
            Self::load_layer(cx, &mut self.wait_body, &lwg, "body");
            if Self::load_layer(cx, &mut self.wait_glow, &lwg, "grow") {
                self.wait_glow.start_fade(0x6B, 0, 8);
                self.wait_glow.fade.looping = true;
            }
        }
        self.back.level = 255 - self.back_alpha;
        self.pos = pt(mem.d(r + T::X), mem.d(r + T::Y));
        self.text_pos = pt(mem.d(r + T::TEXT_X), mem.d(r + T::TEXT_Y));
        self.name_pos = pt(mem.d(r + T::NAME_X), mem.d(r + T::NAME_Y));
        self.wait_pos = pt(mem.d(r + T::WAIT_X), mem.d(r + T::WAIT_Y));
        self.wait_glow.color = mem.d(r + T::GLOW) as u32 & 0xFFFFFF;
        self.style = self.style(mem, cx.apini, false);
        self.name_style = self.style(mem, cx.apini, true);
        self.name_visible = mem.d(r + T::NAME_VISIBLE) != 0;
        self.visible = mem.d(r + T::VISIBLE) != 0;
    }
    pub fn clear_text(&mut self) {
        self.text.clear();
        self.reveal.reset();
    }
    pub fn append_text(&mut self, text: &[u8]) {
        let style = self.style.clone();
        if !self.text.append(text, &style) {
            log::debug!("RScript message overflows its box");
        }
    }
    pub fn set_name(&mut self, mem: &mut Memory, name: Option<&[u8]>) {
        self.name.clear();
        let visible = name.is_some_and(|n| !n.is_empty());
        if let Some(n) = name.filter(|n| !n.is_empty()) {
            let style = self.name_style.clone();
            self.name.append(n, &style);
        }
        mem.set_d(self.record + T::NAME_VISIBLE, i32::from(visible));
        self.name_visible = visible;
    }
    pub fn finish(&mut self) {
        self.reveal.finish(&self.text.glyphs);
    }
}

pub struct MessageWindow {
    pub boxes: [TextBox; 4],
    /// Companion panel buttons from compane.lwg: (command, face, hover).
    pub panel: Vec<PanelButton>,
    pub panel_bg: Sprite,
    pub panel_pos: Point,
    pub panel_visible: bool,
    /// The panel's window slider (`slide_lev` track, `slide` knob).
    pub slider: Option<crate::config::Slider>,
    pub input_active: bool,
    pub auto_hidden: bool,
    pub hidden: bool,
    pub page_text: Vec<u8>,
    /// The face window (message state +468): current and outgoing images.
    pub face: Sprite,
    pub face_old: Sprite,
}

pub struct PanelButton {
    pub command: &'static str,
    pub face: Sprite,
    pub focus: Option<Content>,
}

/// Panel commands in `compane.lwg` construction order.
pub const PANEL_COMMANDS: [&str; 14] =
    ["skip", "next", "rev", "fow", "bak", "hide", "voc", "auto", "menu", "save", "load", "extd", "qsave", "qload"];

impl MessageWindow {
    pub fn new() -> Self {
        Self {
            boxes: std::array::from_fn(|i| TextBox { record: S::MESSAGE + M::BOXES + i * M::BOX_STRIDE, back_alpha: 128, ..Default::default() }),
            panel: Vec::new(),
            panel_bg: Sprite::default(),
            panel_pos: Point::default(),
            panel_visible: false,
            slider: None,
            input_active: false,
            auto_hidden: false,
            hidden: false,
            page_text: Vec::new(),
            face: Sprite::default(),
            face_old: Sprite::default(),
        }
    }
    pub fn load_panel(&mut self, cx: &mut Ctx) {
        let lwg = join(cx.apini.dir(crate::apini::Dir::System), "compane");
        if let Some((c, at)) = cx.lwg_layer(&lwg, "bg") {
            self.panel_bg.set_content(Some(c));
            self.panel_bg.pos = at;
            self.panel_bg.visible = true;
        }
        if let Some(track) = cx.res.lwg_of(&lwg).and_then(|l| l.layers.iter().find(|x| x.name == "slide_lev").cloned()) {
            let size = track.asset.as_ref().and_then(|a| cx.res.size(cx.h, cx.client, a)).unwrap_or([1, 1]);
            if let Some(knob) = crate::screens::LwgButton::load(cx, &lwg, "slide", 0) {
                self.slider = Some(crate::config::Slider::new(crate::config::Volume::Window, [track.x, track.y, size[0] as i32, size[1] as i32], knob, 255));
            }
        }
        for command in PANEL_COMMANDS {
            let Some((c, at)) = cx.lwg_layer(&lwg, command) else { continue };
            let focus = cx.lwg_layer(&lwg, &format!("{command}_f")).map(|f| f.0);
            let mut face = Sprite { content: Some(c), pos: at, visible: command != "voc", ..Default::default() };
            face.interactive = false;
            self.panel.push(PanelButton { command, face, focus });
        }
    }
    /// sub_40B500 (show) / sub_40B2F0 (hide): the face image from the face
    /// directory with effect 0 (instant), 1/3 (crossfade at 32/16 levels
    /// per tick), 2 (grow, drawn as a fade at 10), 4..10 (slides by the
    /// image width or height over distance/15 ticks).
    pub fn set_face(&mut self, cx: &mut Ctx, image: u16, effect: i32) {
        let at = Point::default();
        if image == 0 {
            let f = &mut self.face;
            f.stop_animation();
            match effect {
                0 => f.visible = false,
                1 => f.start_fade(2, 1, 32),
                2 => f.start_fade(2, 1, 10),
                3 => f.start_fade(2, 1, 16),
                4 | 6 | 10 | 5 | 7 | 9 | 8 => {
                    let (w, h) = (f.width(), f.height());
                    let d = match effect {
                        5 => pt(w, 0),
                        7 | 9 => pt(0, h),
                        8 => pt(0, -h),
                        _ => pt(-w, 0),
                    };
                    let to = pt(f.pos.x + d.x, f.pos.y + d.y);
                    let speed = (if d.x != 0 { w } else { h } / 15).max(1);
                    f.linear(f.pos, to, speed);
                }
                _ => f.visible = false,
            }
            f.animating = effect != 0;
            return;
        }
        let path = join(cx.apini.dir(crate::apini::Dir::Faces), &format!("{image:04}"));
        let Some(content) = cx.image(&path) else { return };
        let had = self.face.visible;
        self.face_old = self.face.clone();
        self.face_old.visible = false;
        let f = &mut self.face;
        f.stop_animation();
        f.set_content(Some(content));
        f.pos = at;
        f.visible = true;
        match effect {
            1 | 3 => {
                let step = if effect == 1 { 32 } else { 16 };
                if had {
                    self.face_old.visible = true;
                    self.face_old.start_fade(2, 1, step);
                    self.face_old.animating = true;
                }
                f.start_fade(2, 0, step);
            }
            2 => f.start_fade(2, 0, 10),
            4..=10 => {
                let (w, h) = (f.width(), f.height());
                let from = match effect {
                    5 => pt(w, 0),
                    7 | 9 => pt(0, h),
                    8 => pt(0, -h),
                    _ => pt(-w, 0),
                };
                f.pos = from;
                let speed = (if from.x != 0 { w } else { h } / 15).max(1);
                f.linear(from, at, speed);
            }
            _ => {}
        }
        f.animating = effect != 0;
    }
    pub fn dword(mem: &Memory, o: usize) -> i32 {
        mem.d(S::MESSAGE + o)
    }
    /// The panel shows with box 0 while enabled.
    pub fn update_panel(&mut self, mem: &Memory) {
        self.panel_pos = pt(Self::dword(mem, M::PANEL_X), Self::dword(mem, M::PANEL_Y));
        self.panel_visible = Self::dword(mem, M::PANEL_ENABLED) != 0 && self.boxes[0].visible && !self.auto_hidden;
    }
    pub fn set_input(&mut self, active: bool) {
        self.input_active = active;
        for b in &mut self.panel {
            b.face.interactive = active;
            if !active {
                b.face.hovered = false;
            }
        }
    }
    pub fn set_voice_available(&mut self, available: bool) {
        for b in &mut self.panel {
            if b.command == "voc" {
                b.face.visible = available;
            }
        }
    }
    /// (script, text, name) of a box; the record stores script, name, text
    /// (sub_42A4D0 writes the name at +4, the page renderers read +8).
    fn source(mem: &Memory, b: usize) -> (u16, u32, u32) {
        let base = S::MESSAGE + M::SOURCES + b * M::SOURCE_STRIDE;
        (mem.uw(base), mem.d(base + 8) as u32, mem.d(base + 4) as u32)
    }
    pub fn set_source(mem: &mut Memory, b: usize, (script, text, name): (u16, u32, u32)) {
        let base = S::MESSAGE + M::SOURCES + b * M::SOURCE_STRIDE;
        mem.set_w(base, i32::from(script));
        mem.set_d(base + 4, name as i32);
        mem.set_d(base + 8, text as i32);
    }
    /// Appends box 0's page state to the backlog ring.
    pub fn push_backlog(mem: &mut Memory, page_start: bool) {
        let base = S::MESSAGE + M::BACKLOG;
        let stride = M::BACKLOG_STRIDE;
        let last = base + stride * (M::BACKLOG_COUNT - 1);
        if mem.uw(last + M::ENTRY_SCRIPT) != 0 || mem.d(last + M::ENTRY_TEXT) != 0 {
            mem.scene.copy_within(base + stride..base + stride * M::BACKLOG_COUNT, base);
            mem.scene[last + M::ENTRY_PAGE_START] = u8::from(page_start);
        } else {
            mem.scene[last + M::ENTRY_PAGE_START] = 1;
        }
        let page = S::MESSAGE + M::PAGE;
        mem.scene.copy_within(page..page + 32, last + M::ENTRY_PAGE);
        let src = S::MESSAGE + M::SOURCES;
        mem.scene.copy_within(src..src + 12, last + M::ENTRY_SCRIPT);
        let b0 = S::MESSAGE + M::BOXES;
        mem.scene.copy_within(b0..b0 + 96, last + M::ENTRY_BOX);
    }
    pub fn clear_backlog(mem: &mut Memory) {
        let s = S::MESSAGE + M::BACKLOG;
        mem.scene[s..s + M::BACKLOG_STRIDE * M::BACKLOG_COUNT].fill(0);
    }
    pub fn backlog_range() -> std::ops::Range<usize> {
        let s = S::MESSAGE + M::BACKLOG;
        s..s + M::BACKLOG_STRIDE * M::BACKLOG_COUNT
    }
    /// Shows or hides a box; hiding also clears it.
    pub fn show_box(&mut self, mem: &mut Memory, cx: &mut Ctx, b: usize, visible: bool, skipping: bool) {
        mem.set_d(self.boxes[b].record + T::VISIBLE, i32::from(visible));
        if !skipping {
            self.boxes[b].visible = visible;
        }
        if !visible {
            self.clear(mem, b, skipping);
        }
        if !skipping && visible {
            self.boxes[b].apply(mem, cx);
        }
        if !skipping && b == 0 {
            self.update_panel(mem);
        }
    }
    /// Clears a box's text.
    pub fn clear(&mut self, mem: &mut Memory, b: usize, skipping: bool) {
        Self::set_source(mem, b, (0, 0, 0));
        if b == 0 {
            Self::push_backlog(mem, true);
        }
        if skipping {
            return;
        }
        self.page_text.clear();
        self.boxes[b].clear_text();
        self.boxes[b].set_name(mem, None);
    }
    pub fn set_waiting(&mut self, mem: &Memory, waiting: bool) {
        let current = (mem.uw(S::MESSAGE + M::CURRENT_BOX) & 3) as usize;
        for (i, b) in self.boxes.iter_mut().enumerate() {
            b.waiting = waiting && i == current && mem.d(b.record + T::WAIT_ENABLED) != 0;
        }
    }
    pub fn finish(&mut self) {
        for b in &mut self.boxes {
            b.finish();
        }
    }
    pub fn reveal(&mut self) {
        for b in &mut self.boxes {
            if b.visible {
                b.reveal.start();
            }
        }
    }
    /// Rebuild boxes and their current page from the state.
    pub fn restore(&mut self, mem: &mut Memory, cx: &mut Ctx, string: &mut dyn FnMut(u16, u32) -> Option<Vec<u8>>) {
        self.set_waiting(mem, false);
        for i in 0..4 {
            let b = &mut self.boxes[i];
            b.clear_text();
            b.frame_loaded = None;
            b.wait_loaded = None;
            b.apply(mem, cx);
            let (script, text, name) = Self::source(mem, i);
            if script == 0 {
                b.set_name(mem, None);
                continue;
            }
            let page = if i == 0 { backlog_page(mem, last_page(mem), string) } else { string(script, text).unwrap_or_default() };
            if i == 0 {
                self.page_text = page.clone();
            }
            let b = &mut self.boxes[i];
            b.append_text(&page);
            b.finish();
            let n = if name != 0 { string(script, name) } else { None };
            b.set_name(mem, n.as_deref());
        }
        self.update_panel(mem);
        let voice = Self::dword(mem, M::VOICE);
        self.set_voice_available(voice != 0);
    }
}

fn entry(i: usize) -> usize {
    S::MESSAGE + M::BACKLOG + i * M::BACKLOG_STRIDE
}
/// The last page start.
pub fn last_page(mem: &Memory) -> usize {
    (0..M::BACKLOG_COUNT).rev().find(|&i| mem.scene[entry(i) + M::ENTRY_PAGE_START] != 0).unwrap_or(0)
}
/// Concatenated text of the page starting at `index`.
pub fn backlog_page(mem: &Memory, index: usize, string: &mut dyn FnMut(u16, u32) -> Option<Vec<u8>>) -> Vec<u8> {
    let mut out = Vec::new();
    for i in index..M::BACKLOG_COUNT {
        if i > index && mem.scene[entry(i) + M::ENTRY_PAGE_START] != 0 {
            break;
        }
        let (script, text) = (mem.uw(entry(i) + M::ENTRY_SCRIPT), mem.d(entry(i) + M::ENTRY_TEXT) as u32);
        if script == 0 && text == 0 {
            continue;
        }
        out.extend(string(script, text).unwrap_or_default());
    }
    out
}

// ----------------------------------------------------------------- choices

pub struct ChoiceItem {
    pub sprite: Sprite,
    /// Normal, focused and pressed plates.
    pub states: Vec<Content>,
    pub text: Block,
    pub text_at: Point,
    /// Body layer placement inside the plate canvas.
    pub plate_at: Point,
    pub canvas: [i32; 2],
    pub style: Style,
    pub final_pos: Point,
    pub start: Point,
    pub end: Point,
}

#[derive(Default)]
pub struct Choice {
    pub question: Option<ChoiceItem>,
    pub answers: Vec<ChoiceItem>,
    pub selected: usize,
    pub visible: bool,
}

/// A leading `<N>` selects plate set N.
fn image_set(text: &[u8]) -> (u32, &[u8]) {
    if text.first() != Some(&b'<') {
        return (0, text);
    }
    let Some(close) = text.iter().position(|&b| b == b'>') else { return (0, text) };
    let n = text[1..close].iter().take_while(|b| b.is_ascii_digit()).fold(0u32, |n, b| n * 10 + u32::from(b - b'0'));
    (n, &text[close + 1..])
}

impl Choice {
    fn build(cx: &mut Ctx, question: bool, source: &[u8]) -> Option<ChoiceItem> {
        let (set, text) = image_set(source);
        let kind = if question { 'q' } else { 'a' };
        let lwg = join(cx.apini.dir(crate::apini::Dir::System), &format!("sel_{kind}{set:02}"));
        let canvas = cx.res.lwg_of(&lwg).map(|l| l.canvas);
        let mut plate_at = pt(0, 0);
        let (states, rect) = if let Some([w, h]) = canvas {
            let rect = match cx.lwg_layer(&lwg, "text") {
                Some((c, at)) => (at, c.size()),
                None => (pt(((w - w * 80 / 100) / 2) as i32, ((h - h * 90 / 100) / 2) as i32), [w * 80 / 100, h * 90 / 100]),
            };
            let names: &[&str] = if question { &["body"] } else { &["body", "body_f", "body_c"] };
            let mut states = Vec::new();
            for n in names {
                if let Some((c, at)) = cx.lwg_layer(&lwg, n) {
                    if states.is_empty() {
                        plate_at = at;
                    }
                    states.push(c);
                }
            }
            (states, rect)
        } else {
            let plain = cx.image(&join(cx.apini.dir(crate::apini::Dir::System), &format!("sel_{kind}")));
            let rect = if question { (pt(20, 14), [510, 29]) } else { (pt(20, 11), [510, 27]) };
            (plain.into_iter().collect(), rect)
        };
        if states.is_empty() {
            return None;
        }
        let style = Style {
            size: i32::from(if question { cx.apini.question_size } else { cx.apini.answer_size }).max(1),
            color: if question { cx.apini.question_color } else { cx.apini.answer_color } & 0xFFFFFF,
            palette: cx.apini.palette,
            width: rect.1[0] as i32,
            height: rect.1[1] as i32,
            ruby_size: 12,
            baseline_at_bottom: true,
            ..Default::default()
        };
        let mut block = Block::new(50);
        block.append(text, &style);
        let size = canvas.map_or(states[0].size(), |c| c);
        let sprite = Sprite { content: Some(states[0].clone()), ..Default::default() };
        Some(ChoiceItem { sprite, states, text: block, text_at: rect.0, plate_at, canvas: [size[0] as i32, size[1] as i32], style, final_pos: pt(0, 0), start: pt(0, 0), end: pt(0, 0) })
    }
    /// Plates for the question and answers, laid out.
    pub fn open(cx: &mut Ctx, question: &[u8], answers: &[Vec<u8>], arrangement: i32) -> Self {
        let mut c = Choice { question: if question.is_empty() { None } else { Self::build(cx, true, question) }, ..Default::default() };
        for a in answers {
            if let Some(item) = Self::build(cx, false, a) {
                c.answers.push(item);
            }
        }
        let [w, h] = cx.size();
        let qh = c.question.as_ref().map_or(0, |q| q.canvas[1]);
        let total = qh + c.answers.iter().map(|a| a.canvas[1] + 5).sum::<i32>();
        let qx = ((w - c.question.as_ref().map_or(0, |q| q.canvas[0])) >> 1) - 30;
        let qy = (80 * h / 100 - total).max(0) >> 1;
        if let Some(q) = &mut c.question {
            q.final_pos = pt(qx, qy);
            q.start = q.final_pos;
            q.end = q.final_pos;
        }
        let mut y = qh + qy + 5;
        for a in &mut c.answers {
            let x = (w - a.canvas[0]) >> 1;
            a.final_pos = pt(x, y);
            a.start = pt(x, qy);
            a.end = pt(x, h);
            y += a.canvas[1] + 5;
        }
        for item in c.question.iter_mut().chain(c.answers.iter_mut()) {
            let ([iw, ih], f) = (item.canvas, item.final_pos);
            match arrangement {
                2 => (item.start, item.end) = (pt(f.x, -ih), pt(f.x, h)),
                3 => (item.start, item.end) = (pt(f.x, h), pt(f.x, -ih)),
                4 => (item.start, item.end) = (pt(-iw, f.y), pt(w, f.y)),
                _ => {}
            }
        }
        c
    }
    fn slide(item: &mut ChoiceItem, from: Point, to: Point, divisor: i32, accelerate: bool) {
        let s = &mut item.sprite;
        s.stop_animation();
        s.pos = from;
        let distance = f64::from(to.x - from.x).hypot(f64::from(to.y - from.y)) as i32;
        s.curved(from, to, distance / divisor, accelerate);
        s.animating = s.motion.active;
        if !s.motion.active {
            s.pos = to;
        }
    }
    /// Shows the window, sliding plates in unless `instant`.
    pub fn appear(&mut self, instant: bool) {
        self.visible = true;
        for item in self.question.iter_mut().chain(self.answers.iter_mut()) {
            item.sprite.visible = true;
            if instant {
                item.sprite.pos = item.final_pos;
            } else {
                let (from, to) = (item.start, item.final_pos);
                Self::slide(item, from, to, 10, true);
            }
        }
    }
    pub fn set_input(&mut self, enabled: bool) {
        for a in &mut self.answers {
            a.sprite.interactive = enabled;
            if !enabled {
                Self::hover(a, false);
            }
        }
    }
    pub fn hover(item: &mut ChoiceItem, inside: bool) {
        item.sprite.hovered = inside;
        if item.states.len() == 1 {
            item.sprite.mode = if inside { 13 } else { 0 };
        } else {
            item.sprite.content = Some(item.states[usize::from(inside)].clone());
        }
    }
    pub fn press(&mut self, index: usize) -> bool {
        let Some(item) = self.answers.get_mut(index) else { return false };
        if !item.sprite.interactive {
            return false;
        }
        item.sprite.content = Some(item.states.get(2).or(item.states.get(1)).unwrap_or(&item.states[0]).clone());
        self.selected = index + 1;
        true
    }
    /// Slides away everything but the chosen answer.
    pub fn disappear(&mut self, instant: bool) {
        let selected = self.selected;
        let items = self.question.iter_mut().chain(self.answers.iter_mut().enumerate().filter(|(i, _)| *i + 1 != selected).map(|(_, a)| a));
        for item in items {
            if instant {
                item.sprite.visible = false;
            } else {
                let (from, to) = (item.sprite.pos, item.end);
                Self::slide(item, from, to, 5, false);
            }
        }
    }
    pub fn tick(&mut self) -> bool {
        let mut running = false;
        for item in self.question.iter_mut().chain(self.answers.iter_mut()) {
            running |= item.sprite.tick();
        }
        running
    }
}

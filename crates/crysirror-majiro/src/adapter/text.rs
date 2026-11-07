// Copyright (c) 2026 CrySirror contributors. MIT.
//! Native text drawing (413AC0 fontout, 417070 PROC) over the engine glyph
//! service. Glyph coverage comes from shared worker raster jobs and is cached
//! by face/size/character; composition uses the native colour/shadow modes
//! into VM-visible pages. Line layout, hooks and PC rewind follow 417070.
use super::*;
use crysirror_core::image_processing::{GlyphImageSpec, ImageJob, ImageProcessState};
use crysirror_image::transparency_planes::PlaneFormat;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct TextFont {
    pub alias: String,
    pub height: i32,
    pub half: i32,
    pub pitch: i32,
}
/// 108-byte native fontout state (unk_480EF8 defaults).
#[derive(Clone)]
pub(super) struct Fontout {
    pub font: Option<TextFont>,
    pub face: Vec<u8>,
    pub color: u32,
    pub shadow: i32,
    pub page: Option<u32>,
    pub x: i32,
    pub y: i32,
}
pub(super) struct Glyph {
    width: u32,
    height: u32,
    origin: [i32; 2],
    coverage: Vec<u8>,
}
type GlyphKey = (String, u16, u32);
pub(super) enum ProcStep {
    Empty,
    Wait,
    Hooked,
    Line,
}
#[derive(Default)]
pub(super) struct GlyphCache {
    ready: BTreeMap<GlyphKey, Option<Arc<Glyph>>>,
    jobs: Vec<(GlyphKey, ImageJob)>,
}

fn decode_char(bytes: &[u8]) -> Option<char> {
    encoding_rs::SHIFT_JIS
        .decode_without_bom_handling_and_without_replacement(bytes)
        .and_then(|s| s.chars().next())
}
pub(super) fn chars(bytes: &[u8]) -> Vec<&[u8]> {
    let mut out = vec![];
    let mut i = 0;
    while i < bytes.len() {
        let lead = bytes[i];
        let n = if (0x81..=0x9f).contains(&lead) || (0xe0..=0xfc).contains(&lead) { 2 } else { 1 };
        let end = (i + n).min(bytes.len());
        out.push(&bytes[i..end]);
        i = end;
    }
    out
}
fn advance(font: &TextFont, ch: &[u8]) -> i32 {
    // Monospace GDI faces: double-byte characters take the full cell.
    let half_kana = ch.len() == 1 && (0xa1..=0xdf).contains(&ch[0]);
    if ch.len() == 2 { font.half * 2 } else if half_kana { font.half } else { font.half }
}
pub(super) fn measure(font: &TextFont, bytes: &[u8]) -> i32 {
    chars(bytes).iter().map(|c| advance(font, c)).sum()
}
/// Line-start prohibited characters (gyoutou kinsoku defaults).
const NO_LINE_START: &[&str] = &["、", "。", "，", "．", "」", "』", "）", "】", "！", "？", "ー", "…", "っ", "ゃ", "ゅ", "ょ", "ッ", "ャ", "ュ", "ョ", "ぁ", "ぃ", "ぅ", "ぇ", "ぉ", "々", "〜", "・", "）", "〕", "”"];

impl MajiroVm {
    /// Ensure coverage exists for every character; false while workers run.
    pub(super) fn ensure_glyphs(&mut self, host: &mut dyn HostServices, font: &TextFont, text: &[u8]) -> Result<bool> {
        let size = font.height.clamp(1, 512) as u16;
        let mut wanted = vec![];
        for ch in chars(text) {
            let Some(c) = decode_char(ch) else { continue };
            if c.is_whitespace() || c == '\u{3000}' {
                continue;
            }
            let key = (font.alias.clone(), size, c as u32);
            if !self.glyphs.ready.contains_key(&key) && !self.glyphs.jobs.iter().any(|(k, _)| *k == key) && !wanted.contains(&key) {
                wanted.push(key);
            }
        }
        if !wanted.is_empty() {
            let fonts = host.font_library(self.client).map_err(core_error)?;
            let images = host.image_metadata(self.client).map_err(core_error)?;
            for key in wanted {
                let c = char::from_u32(key.2).unwrap_or('?');
                let s = i32::from(key.1);
                let job = fonts.glyph_id(&key.0, c).and_then(|glyph_id| {
                    images.begin_glyph_image(GlyphImageSpec {
                        font: key.0.clone(),
                        size: key.1,
                        canvas: [(s * 2) as u32, (s * 2) as u32],
                        glyphs: vec![crysirror_core::service::Glyph {
                            glyph_id,
                            x_26_6: (s / 2) * 64,
                            y_26_6: (s * 3 / 2) * 64,
                            advance_26_6: s * 64,
                        }],
                    })
                });
                match job {
                    Ok(job) => self.glyphs.jobs.push((key, job)),
                    Err(_) => {
                        self.glyphs.ready.insert(key, None);
                    }
                }
            }
        }
        let mut still = vec![];
        for (key, mut job) in std::mem::take(&mut self.glyphs.jobs) {
            match job.poll().map_err(core_error)? {
                ImageProcessState::Pending => still.push((key, job)),
                ImageProcessState::Failed(_) => {
                    self.glyphs.ready.insert(key, None);
                }
                ImageProcessState::Ready(image) => {
                    let [w, h] = image.size();
                    let coverage = image.alpha_region([0, 0, w, h]).map_err(core_error)?.to_vec();
                    let s = i32::from(key.1);
                    self.glyphs.ready.insert(
                        key,
                        Some(Arc::new(Glyph { width: w, height: h, origin: [s / 2, s * 3 / 2], coverage })),
                    );
                }
            }
        }
        self.glyphs.jobs = still;
        Ok(self.glyphs.jobs.is_empty())
    }
    /// Composite one coverage pass with a COLORREF into a page (415640).
    fn stamp(&mut self, page: u32, glyph: &Glyph, at: [i32; 2], color: u32) {
        let alpha = self.gfx.pages.get(&page).and_then(|p| p.alpha);
        let Some(p) = self.gfx.pages.get_mut(&page) else { return };
        let (pw, ph) = (p.plane.width as i32, p.plane.height as i32);
        let bgr = [(color >> 16) as u8, (color >> 8) as u8, color as u8];
        let mut mask_writes = vec![];
        for gy in 0..glyph.height as i32 {
            let y = at[1] + gy;
            if y < 0 || y >= ph {
                continue;
            }
            for gx in 0..glyph.width as i32 {
                let x = at[0] + gx;
                if x < 0 || x >= pw {
                    continue;
                }
                let c = u32::from(glyph.coverage[(gy as u32 * glyph.width + gx as u32) as usize]);
                if c == 0 {
                    continue;
                }
                match p.plane.format {
                    PlaneFormat::Bgr24 => {
                        let o = (y as usize * pw as usize + x as usize) * 3;
                        for k in 0..3 {
                            let d = u32::from(p.plane.pixels[o + k]);
                            p.plane.pixels[o + k] = ((d * (255 - c) + u32::from(bgr[k]) * c + 127) / 255) as u8;
                        }
                    }
                    PlaneFormat::Index8 => {
                        let o = y as usize * pw as usize + x as usize;
                        if c >= 128 {
                            p.plane.pixels[o] = color as u8;
                        }
                    }
                }
                mask_writes.push((x, y, c));
            }
        }
        if let Some(alpha) = alpha.and_then(|a| self.gfx.pages.get_mut(&a)) {
            let w = alpha.plane.width as usize;
            for (x, y, c) in mask_writes {
                let o = y as usize * w + x as usize;
                if o < alpha.plane.pixels.len() {
                    let t = 255 - c;
                    alpha.plane.pixels[o] = alpha.plane.pixels[o].min(t as u8);
                }
            }
        }
    }
    /// Draw a CP932 string with native shadow modes; returns the advance.
    pub(super) fn draw_text(&mut self, page: u32, x: i32, y: i32, font: &TextFont, text: &[u8], color: u32, shadow: i32) -> i32 {
        let size = font.height.clamp(1, 512) as u16;
        let ascent = (font.height * 86 + 50) / 100;
        let s = (font.height / 18).max(1);
        let offsets: Vec<[i32; 2]> = if shadow == -1 {
            vec![]
        } else if (shadow as u32 & 0x7f00_0000) == 0 {
            vec![[s, s]]
        } else {
            vec![[-s, -s], [0, -s], [s, -s], [-s, 0], [s, 0], [-s, s], [0, s], [s, s]]
        };
        let shadow_color = (shadow as u32) & 0x00ff_ffff;
        let mut pen = x;
        for ch in chars(text) {
            let adv = advance(font, ch);
            if let Some(c) = decode_char(ch) {
                if let Some(Some(glyph)) = self.glyphs.ready.get(&(font.alias.clone(), size, c as u32)).cloned() {
                    let at = [pen - glyph.origin[0], y + ascent - glyph.origin[1]];
                    for o in &offsets {
                        self.stamp(page, &glyph, [at[0] + o[0], at[1] + o[1]], shadow_color);
                    }
                    self.stamp(page, &glyph, at, color);
                }
            }
            pen += adv;
        }
        let width = pen - x;
        let extra = if offsets.is_empty() { 0 } else { s + 1 };
        self.gfx.invalidate(page, [x - extra, y - extra, width + 2 * extra, font.height + 2 * extra]);
        width
    }
    fn default_fontout(&mut self, host: &mut dyn HostServices) -> Fontout {
        let face = "ＭＳ ゴシック";
        let alias = self.resolve_font_alias(face, host).ok();
        Fontout {
            font: alias.map(|alias| TextFont { alias, height: 20, half: 10, pitch: 25 }),
            face: encoding_rs::SHIFT_JIS.encode(face).0.into_owned(),
            color: 0xffffff,
            shadow: -1,
            page: self.gfx.primary,
            x: 0,
            y: 0,
        }
    }
    /// fontout family (413360/4132C0/4131E0/413560).
    pub(super) fn try_fontout(
        &mut self,
        m: &mut Machine,
        hash: u32,
        a: &[Value],
        host: &mut dyn HostServices,
    ) -> Result<Option<Option<i32>>> {
        if !matches!(hash, 0xe186d19a | 0x90816c6e | 0x57f6310d | 0x35a89417) {
            return Ok(None);
        }
        let thread = m.current_thread();
        if !self.fontout.contains_key(&thread) {
            let state = self.default_fontout(host);
            self.fontout.insert(thread, state);
        }
        let int = |i: usize| a.get(i).and_then(|v| v.integer().ok());
        match hash {
            0xe186d19a => {
                let face = match a.first() {
                    Some(Value::String(b)) => b[..b.iter().position(|x| *x == 0).unwrap_or(b.len())].to_vec(),
                    _ => vec![],
                };
                let height = int(1).unwrap_or(20);
                let half = int(2).unwrap_or(height / 2);
                let pitch = int(3).unwrap_or(5 * height / 4);
                let name = encoding_rs::SHIFT_JIS.decode(&face).0.into_owned();
                let alias = self.resolve_font_alias(&name, host).ok();
                let state = self.fontout.get_mut(&thread).unwrap();
                state.face = face;
                state.font = alias.map(|alias| TextFont { alias, height: height.clamp(1, 512), half: half.max(1), pitch });
            }
            0x90816c6e => {
                let state = self.fontout.get_mut(&thread).unwrap();
                state.color = int(0).unwrap_or(0xffffff) as u32;
                state.shadow = int(1).unwrap_or(-1);
            }
            0x57f6310d => {
                let page = int(0).unwrap_or(0) as u32;
                let page = if self.gfx.pages.contains_key(&page) { Some(page) } else { self.gfx.primary };
                let state = self.fontout.get_mut(&thread).unwrap();
                state.page = page;
                state.x = int(1).unwrap_or(0);
                state.y = int(2).unwrap_or(0);
            }
            _ => {
                let text = match a.first() {
                    Some(Value::String(b)) => b[..b.iter().position(|x| *x == 0).unwrap_or(b.len())].to_vec(),
                    _ => vec![],
                };
                let state = self.fontout[&thread].clone();
                let (Some(font), Some(page)) = (state.font.clone(), state.page) else {
                    self.warn("fontout without an available font/page".into());
                    return Ok(Some(Some(0)));
                };
                if !self.ensure_glyphs(host, &font, &text)? {
                    return Ok(Some(None));
                }
                let width = measure(&font, &text);
                let (mut x, mut y) = (state.x, state.y);
                if a.len() >= 3 {
                    let (bw, bh) = (int(1).unwrap_or(0), int(2).unwrap_or(0));
                    match int(3).unwrap_or(0) {
                        -2 => x += bw - width,
                        -1 => {}
                        _ => x += (bw - width) / 2,
                    }
                    y += (bh - font.height) / 2;
                }
                let drawn = self.draw_text(page, x, y, &font, &text, state.color, state.shadow);
                let state = self.fontout.get_mut(&thread).unwrap();
                state.x = x + drawn;
            }
        }
        Ok(Some(Some(0)))
    }

    /// 417070 PROC. Returns false only while glyph coverage is prepared.
    pub(super) fn process_text(&mut self, m: &mut Machine, host: &mut dyn HostServices) -> Result<bool> {
        loop {
            match self.proc_line(m, host, false)? {
                ProcStep::Empty => {
                    self.history_pc = None;
                    m.acknowledge(vec![])?;
                    return Ok(true);
                }
                ProcStep::Wait => return Ok(false),
                ProcStep::Hooked => return Ok(true),
                ProcStep::Line => continue,
            }
        }
    }
    /// One native PROC execution: a hook, one drawn line, or empty buffer.
    pub(super) fn proc_line(&mut self, m: &mut Machine, host: &mut dyn HostServices, replay: bool) -> Result<ProcStep> {
        if self.text_buffer.is_empty() {
            return Ok(ProcStep::Empty);
        }
        // 417070 records a message once per PROC instruction (4F1200).
        if !replay && self.history_pc != Some(m.pc) {
            self.history_pc = Some(m.pc);
            let text = self.text_buffer.clone();
            self.console.record_text(&text)?;
        }
        if !replay && !self.console.visible {
            self.console.visible = true;
            if m.interrupt_with_event(0xf7a4c8d8, vec![Value::Integer(0), Value::Integer(0)])? {
                return Ok(ProcStep::Hooked);
            }
        }
        for b in self.text_buffer.iter_mut() {
            if *b == b'|' {
                *b = b' ';
            }
        }
        // NAME_DISP: text before the opening bracket is a speaker name.
        let open = [0x81u8, 0x75];
        if let Some(p) = self.text_buffer.windows(2).position(|w| w == open) {
            if p > 0 && chars(&self.text_buffer[..p]).concat().len() == p && m.hook_exists(0x44a4ff72) {
                let name = self.text_buffer[..p].to_vec();
                self.text_buffer.drain(..p);
                m.interrupt_with_event(0x44a4ff72, vec![Value::native_string(&name)?])?;
                return Ok(ProcStep::Hooked);
            }
        }
        let Some(page) = self.console.page else {
            self.warn("PROC without console_set page; text dropped".into());
            self.text_buffer.clear();
            return Ok(ProcStep::Empty);
        };
        let Some(font) = self.console_text_font(host) else {
            self.warn("PROC without a console font; text dropped".into());
            self.text_buffer.clear();
            return Ok(ProcStep::Empty);
        };
        let [ox, oy] = self.console.origin;
        let [ew, eh] = self.console.extent;
        if self.console.cursor[1] > oy + eh - font.pitch.max(font.height) {
            self.console.cursor = self.console.origin;
            if m.interrupt_with_event(0x93b38a0b, vec![])? {
                return Ok(ProcStep::Hooked);
            }
        }
        if !self.ensure_glyphs(host, &font, &self.text_buffer.clone())? {
            return Ok(ProcStep::Wait);
        }
        let available = ox + ew - self.console.cursor[0];
        let all = chars(&self.text_buffer).into_iter().map(|c| c.to_vec()).collect::<Vec<_>>();
        let mut width = 0;
        let mut count = 0usize;
        for c in &all {
            let w = advance(&font, c);
            if width + w > available && count > 0 {
                break;
            }
            width += w;
            count += 1;
        }
        // Gyoutou kinsoku: never start the next line with closing punctuation.
        if count < all.len() && count > 1 {
            let next = encoding_rs::SHIFT_JIS.decode(&all[count]).0.into_owned();
            if NO_LINE_START.contains(&next.as_str()) {
                count += 1;
            }
        }
        let line: Vec<u8> = all[..count].concat();
        let rest: Vec<u8> = all[count..].concat();
        let [x, y] = self.console.cursor;
        let (color, shadow) = (self.console.colors[0] as u32, self.console.colors[1]);
        let (color, shadow) = if color == 0 && shadow == 0 { (0xffffff, -1) } else { (color, shadow) };
        let drawn = self.draw_text(page, x, y, &font, &line, color, shadow);
        // 417070 accumulates the page's text (byte_49B28C) until a wrap.
        if !replay && self.page_text_open && self.page_text.len() + line.len() < 1024 {
            self.page_text.extend_from_slice(&line);
        }
        if !rest.is_empty() {
            self.page_text_open = false;
        }
        self.text_buffer = rest;
        if self.text_buffer.is_empty() {
            self.console.cursor[0] = x + drawn;
        } else {
            self.console.cursor = [ox, y + font.pitch];
        }
        let args = vec![
            Value::Integer(page as i32),
            Value::Integer(x),
            Value::Integer(y),
            Value::Integer(drawn),
            Value::Integer(font.height),
            Value::Integer(i32::from(replay)),
        ];
        if m.interrupt_with_event(0x47bb540c, args)? {
            return Ok(ProcStep::Hooked);
        }
        Ok(ProcStep::Line)
    }
    fn console_text_font(&mut self, host: &mut dyn HostServices) -> Option<TextFont> {
        if let Some(f) = &self.console.font {
            return Some(TextFont { alias: f.alias.clone(), height: f.height, half: f.half_width, pitch: f.line_pitch });
        }
        let alias = self.resolve_font_alias("ＭＳ ゴシック", host).ok()?;
        Some(TextFont { alias, height: 20, half: 10, pitch: 25 })
    }
}

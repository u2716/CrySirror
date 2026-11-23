// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Script text: `@`/`$` expansion, message markup and the native
//! fixed-cell layout. Cells are
//! `size` square for double-byte characters and `size/2` wide otherwise, so
//! layout needs no font metrics; the shared text renderer draws each cell.

pub fn is_lead(b: u8) -> bool {
    matches!(b, 0x81..=0x9F | 0xE0..=0xFC)
}

/// `@@-12`: '@' count is the indirection depth of a signed decimal.
fn variable_ref(bytes: &[u8]) -> (u32, i32) {
    let mut i = 0;
    let mut levels = 0;
    while bytes.get(i) == Some(&b'@') {
        levels += 1;
        i += 1;
    }
    let neg = bytes.get(i) == Some(&b'-');
    if neg {
        i += 1;
    }
    let mut v: i32 = 0;
    while let Some(d) = bytes.get(i).filter(|d| d.is_ascii_digit()) {
        v = v.wrapping_mul(10).wrapping_add(i32::from(d - b'0'));
        i += 1;
    }
    (levels, i32::from((if neg { -v } else { v }) as i16))
}

/// Expansion: `@` references become ASCII decimal variable values, then
/// `$0`..`$9` insert string registers. Both passes skip trail bytes.
pub fn expand(src: &[u8], var: impl Fn(u32, i32) -> i32, reg: impl Fn(usize) -> Vec<u8>) -> Vec<u8> {
    let mut first = Vec::with_capacity(src.len());
    let mut i = 0;
    while i < src.len() && src[i] != 0 {
        let b = src[i];
        if is_lead(b) && i + 1 < src.len() {
            first.extend_from_slice(&src[i..i + 2]);
            i += 2;
        } else if b == b'@' {
            let start = i;
            while i < src.len() && (src[i] == b'@' || src[i] == b'-' || src[i].is_ascii_digit()) {
                i += 1;
            }
            let (levels, value) = variable_ref(&src[start..i]);
            first.extend_from_slice(var(levels, value).to_string().as_bytes());
        } else {
            first.push(b);
            i += 1;
        }
    }
    let mut out = Vec::with_capacity(first.len());
    let mut i = 0;
    while i < first.len() {
        let b = first[i];
        if is_lead(b) && i + 1 < first.len() {
            out.extend_from_slice(&first[i..i + 2]);
            i += 2;
        } else if b == b'$' && first.get(i + 1).is_some_and(u8::is_ascii_digit) {
            out.extend(reg(usize::from(first[i + 1] - b'0')));
            i += 2;
        } else {
            out.push(b);
            i += 1;
        }
    }
    out
}

/// Colours selected by `^C` letters, as 0xRRGGBB.
fn named_color(letter: u8) -> Option<u32> {
    Some(match letter.to_ascii_uppercase() {
        b'B' => 0x2020FF,
        b'G' => 0x7FDFA5,
        b'K' => 0x000000,
        b'O' => 0xFAA25A,
        b'P' => 0xF8B2EF,
        b'R' => 0xB73233,
        b'S' => 0x79F1F2,
        b'V' => 0xC187F6,
        b'W' => 0xFFFFFF,
        b'Y' => 0xFFDD00,
        _ => return None,
    })
}

/// Half-width pairs drawn in one full-width cell ("?!", "!?", "!!").
const PAIRS: [u16; 3] = [0x213F, 0x3F21, 0x2121];
/// Characters that may hang past the line end (class 1).
const HANGING: &[u16] = &[
    0x20, 0x27, 0x22, 0x2C, 0x2E, 0x3F, 0x21, 0x29, 0x7D, 0x5D, 0x8140, 0x8141, 0x8142, 0x8143, 0x8144, 0x8148, 0x8149,
    0x815B, 0x8160, 0x8163, 0x816A, 0x816C, 0x816E, 0x8170, 0x8172, 0x8174, 0x8176, 0x8178, 0x817A, 0x8166, 0x8168,
    0x82C1, 0x829F, 0x82A1, 0x82A3, 0x82A5, 0x82A7, 0x82E1, 0x82E3, 0x82E5, 0x8340, 0x8342, 0x8344, 0x8346, 0x8348,
    0x8383, 0x8385, 0x8387, 0x213F, 0x3F21, 0x2121,
];
/// Opening brackets that move to the next line (class 2).
const OPENING: &[u16] = &[0x8165, 0x8167, 0x8169, 0x816B, 0x816D, 0x816F, 0x8171, 0x8173, 0x8175, 0x8177, 0x8179, 0x814A];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Style {
    pub face: u16,
    pub size: i32,
    /// 0xRRGGBB.
    pub color: u32,
    pub palette: [u32; 10],
    pub shadow: bool,
    /// Ticks between revealed glyphs; 0 shows everything at once.
    pub speed: i32,
    pub line_spacing: i32,
    pub char_spacing: i32,
    pub indent: i32,
    pub first_indent: i32,
    /// 0 left, 1 centre, 2 right.
    pub align: i32,
    pub width: i32,
    pub height: i32,
    pub ruby_face: u16,
    pub ruby_size: i32,
    pub ruby_raise: i32,
    /// Plain text objects (choices) sit glyph baselines on the line bottom.
    pub baseline_at_bottom: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Glyph {
    /// Shift-JIS code (lead byte high) or single byte.
    pub code: u16,
    pub text: String,
    pub size: i32,
    pub advance: i32,
    pub height: i32,
    pub newline: bool,
    kinsoku: u8,
    /// Ticks before this glyph appears after the previous one.
    pub delay: i32,
    pub color: u32,
    pub bold: bool,
    pub italic: bool,
    pub x: i32,
    pub y: i32,
    pub ruby: Vec<RubyGlyph>,
    ruby_text: Vec<u8>,
    ruby_end: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RubyGlyph {
    pub text: String,
    pub size: i32,
    pub color: u32,
    pub x: i32,
    pub y: i32,
    pub width: i32,
}

fn decode(code: u16) -> String {
    let bytes = if code > 0xFF { vec![(code >> 8) as u8, code as u8] } else { vec![code as u8] };
    if PAIRS.contains(&code) {
        return String::from_utf8_lossy(&bytes).into_owned();
    }
    encoding_rs::SHIFT_JIS.decode(&bytes).0.into_owned()
}

/// One message text object: glyphs, layout and reveal progress.
#[derive(Clone, Debug, Default)]
pub struct Block {
    pub glyphs: Vec<Glyph>,
    pub capacity: usize,
    ruby_start: Option<usize>,
    newline_next: bool,
    pub fits: bool,
}

impl Block {
    pub fn new(capacity: usize) -> Self {
        Self { capacity, fits: true, ..Default::default() }
    }
    pub fn clear(&mut self) {
        self.glyphs.clear();
        self.ruby_start = None;
        self.newline_next = false;
    }
    fn cell(code: u16, size: i32) -> (i32, i32) {
        if code >= 0x100 || PAIRS.contains(&code) { (size, size) } else { (size >> 1, size) }
    }
    fn add(&mut self, code: u16, size: i32, color: u32, bold: bool, italic: bool, delay: i32) {
        if self.glyphs.len() >= self.capacity {
            return;
        }
        let (advance, height) = Self::cell(code, size);
        self.glyphs.push(Glyph {
            code,
            text: decode(code),
            size,
            advance,
            height,
            newline: std::mem::take(&mut self.newline_next),
            kinsoku: if HANGING.contains(&code) {
                1
            } else if OPENING.contains(&code) {
                2
            } else {
                0
            },
            delay,
            color,
            bold,
            italic,
            x: 0,
            y: 0,
            ruby: Vec::new(),
            ruby_text: Vec::new(),
            ruby_end: 0,
        });
    }
    /// Ruby over the glyphs from the last `|` to the last glyph.
    fn add_ruby(&mut self, text: &[u8], color: u32, style: &Style) {
        let Some(last) = self.glyphs.len().checked_sub(1) else { return };
        let first = self.ruby_start.take().unwrap_or(last).min(last);
        let mut ruby = Vec::new();
        let mut i = 0;
        while i < text.len() {
            let double = is_lead(text[i]) && i + 1 < text.len();
            let code = if double { u16::from_be_bytes([text[i], text[i + 1]]) } else { u16::from(text[i]) };
            i += if double { 2 } else { 1 };
            let (w, _) = Self::cell(code, style.ruby_size);
            ruby.push(RubyGlyph { text: decode(code), size: style.ruby_size, color, x: 0, y: 0, width: w });
        }
        let base = &mut self.glyphs[first];
        base.ruby = ruby;
        base.ruby_text = text.to_vec();
        base.ruby_end = last;
    }

    /// Appends marked-up script text and lays everything out again; returns
    /// whether it fits the area.
    pub fn append(&mut self, text: &[u8], style: &Style) -> bool {
        let mut color = style.color;
        let (mut bold, mut italic, mut scale, mut delay) = (false, false, 1i32, style.speed);
        let size = |scale: i32| match scale {
            0 => style.size >> 1,
            1 => style.size,
            2 => 3 * style.size / 2,
            3 => 2 * style.size,
            s => (s - 1) * style.size,
        };
        let mut i = 0;
        while i < text.len() && text[i] != 0 {
            let b = text[i];
            let next = text.get(i + 1).copied().unwrap_or(0);
            if is_lead(b) && next != 0 {
                self.add(u16::from_be_bytes([b, next]), size(scale), color, bold, italic, delay);
                i += 2;
            } else if PAIRS.contains(&(u16::from(b) | u16::from(next) << 8)) {
                self.add(u16::from_be_bytes([b, next]), size(scale), color, bold, italic, delay);
                i += 2;
            } else if b == b'|' {
                self.ruby_start = Some(self.glyphs.len());
                i += 1;
            } else if b == b'[' {
                let mut end = i + 1;
                while end < text.len() && text[end] != 0 && text[end] != b']' {
                    end += if is_lead(text[end]) { 2 } else { 1 };
                }
                let end = end.min(text.len());
                self.add_ruby(&text[i + 1..end], color, style);
                i = if text.get(end) == Some(&b']') { end + 1 } else { end };
            } else if b == b'^' && next != 0 {
                let arg = text.get(i + 2).copied().unwrap_or(0);
                i += 2;
                match next.to_ascii_uppercase() {
                    b'B' => bold = !bold,
                    b'I' => italic = !italic,
                    b'C' => {
                        if let Some(c) = named_color(arg) {
                            color = c;
                        } else if arg.is_ascii_digit() {
                            color = style.palette[usize::from(arg - b'0')];
                        }
                        if arg != 0 {
                            i += 1;
                        }
                    }
                    b'D' => {
                        if style.speed != 0 {
                            delay = if arg == b'0' { 0 } else { style.speed + 2 * (i32::from(arg) - 0x30) - 2 };
                        }
                        if arg != 0 {
                            i += 1;
                        }
                    }
                    b'F' => {
                        if arg != 0 {
                            i += 1;
                        }
                    }
                    b'S' => {
                        scale = i32::from(arg) - 0x30;
                        if arg != 0 {
                            i += 1;
                        }
                    }
                    b'L' => {
                        color = style.color;
                        bold = false;
                        italic = false;
                        scale = 1;
                        delay = style.speed;
                    }
                    b'N' => self.newline_next = true,
                    b'V' => {
                        while let Some(&c) = text.get(i).filter(|c| **c == b'-' || c.is_ascii_digit()) {
                            let code = if c == b'-' { 0x817C } else { u16::from(c) + 0x821F };
                            self.add(code, size(scale), color, bold, italic, delay);
                            i += 1;
                        }
                    }
                    // Inline pictures: their number is skipped (unused here).
                    b'A' | b'G' => {
                        while text.get(i).is_some_and(|c| *c == b'-' || c.is_ascii_digit()) {
                            i += 1;
                        }
                    }
                    _ => {}
                }
            } else {
                self.add(u16::from(b), size(scale), color, bold, italic, delay);
                i += 1;
            }
        }
        self.fits = self.layout(style);
        self.fits
    }

    /// Horizontal layout; false when a line passes the area.
    fn layout(&mut self, s: &Style) -> bool {
        let first_line = if s.line_spacing == 0 { 1 } else { 0 };
        let mut index = 0;
        let mut y = s.line_spacing;
        let mut fits = true;
        let n = self.glyphs.len();
        while index < n {
            let start = index;
            let left = s.indent + if y == first_line { s.first_indent } else { 0 };
            let mut x = left;
            let mut hung = false;
            while index < n {
                let g = &self.glyphs[index];
                if x + g.advance > s.width {
                    if x == left && index == start {
                        return false;
                    }
                    if g.kinsoku == 1 && !hung {
                        hung = true;
                    } else {
                        if index > start + 1 && self.glyphs[index - 1].kinsoku == 2 {
                            index -= 1;
                        }
                        break;
                    }
                }
                x += g.advance + s.char_spacing;
                index += 1;
                if self.glyphs.get(index).is_some_and(|g| g.newline) {
                    break;
                }
            }
            let line_width: i32 = self.glyphs[start..index].iter().map(|g| g.advance).sum();
            let line_height = self.glyphs[start..index].iter().map(|g| g.height).max().unwrap_or(0);
            let mut cursor = left
                + match s.align {
                    1 => (s.width - line_width) >> 1,
                    2 => s.width - line_width,
                    _ => 0,
                };
            for g in &mut self.glyphs[start..index] {
                g.x = cursor;
                g.y = y + line_height - g.height;
                cursor += g.advance + s.char_spacing;
            }
            if index >= n {
                break;
            }
            y += s.line_spacing + line_height;
            if y + s.size > s.height {
                fits = false;
            }
        }
        for i in 0..n {
            self.place_ruby(i, s);
        }
        fits
    }

    /// Ruby spread over its base span, or centred when wider.
    fn place_ruby(&mut self, index: usize, s: &Style) {
        if self.glyphs[index].ruby.is_empty() {
            return;
        }
        let (fx, fy) = (self.glyphs[index].x, self.glyphs[index].y);
        let last = &self.glyphs[self.glyphs[index].ruby_end];
        let span = last.x + last.advance - fx;
        let base = &mut self.glyphs[index];
        let total: i32 = base.ruby.iter().map(|r| r.width).sum();
        let y = fy - s.ruby_raise;
        if total <= span {
            let step = span / base.ruby.len() as i32;
            let mut x = fx + (step >> 1);
            for r in &mut base.ruby {
                r.x = x - (r.width >> 1);
                r.y = y;
                x += step;
            }
        } else {
            let mut x = fx - (total - span) / 2;
            for r in &mut base.ruby {
                r.x = x;
                r.y = y;
                x += r.width;
            }
        }
    }

    /// Decoded text of the block (diagnostics and the save-page text).
    pub fn plain(&self) -> String {
        self.glyphs.iter().map(|g| g.text.as_str()).collect()
    }
}

/// Reveal progress of a text object: glyphs appear after their
/// delay and fade in with level steps of 64 per tick (blend mode 2).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reveal {
    pub revealed: usize,
    /// Fade level (255 hidden .. 0 opaque) of each shown glyph.
    pub levels: Vec<u8>,
    wait: i32,
    pub running: bool,
}

impl Reveal {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    pub fn start(&mut self) {
        self.wait = 0;
        self.running = true;
    }
    /// Shows every glyph immediately.
    pub fn finish(&mut self, glyphs: &[Glyph]) {
        self.revealed = glyphs.len();
        self.levels = vec![0; glyphs.len()];
        self.running = false;
    }
    pub fn complete(&self, glyphs: &[Glyph]) -> bool {
        self.revealed >= glyphs.len() && self.levels.iter().all(|&l| l == 0)
    }
    /// One scene tick; returns whether anything still animates.
    pub fn tick(&mut self, glyphs: &[Glyph]) -> bool {
        let mut fading = false;
        for l in &mut self.levels {
            if *l > 0 {
                *l = l.saturating_sub(64);
                fading |= *l > 0;
            }
        }
        if !self.running {
            return fading;
        }
        if self.wait > 0 {
            self.wait -= 1;
            return true;
        }
        while self.revealed < glyphs.len() {
            let g = &glyphs[self.revealed];
            self.levels.push(if g.delay != 0 { 255 } else { 0 });
            self.revealed += 1;
            if let Some(next) = glyphs.get(self.revealed).filter(|n| n.delay != 0) {
                self.wait = next.delay - 1;
                return true;
            }
        }
        self.running = false;
        fading || self.levels.iter().any(|&l| l > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn style() -> Style {
        Style { size: 24, width: 120, height: 200, line_spacing: 5, ruby_size: 12, ..Default::default() }
    }
    #[test]
    fn expansion_reads_variables_then_registers() {
        let out = expand(b"a@12b$1c", |levels, v| if levels == 1 && v == 12 { -7 } else { 0 }, |r| if r == 1 { b"XY".to_vec() } else { vec![] });
        assert_eq!(out, b"a-7bXYc");
        // A trail byte equal to '@' is not a reference.
        assert_eq!(expand(&[0x81, 0x40, b'x'], |_, _| 9, |_| vec![]), vec![0x81, 0x40, b'x']);
    }
    #[test]
    fn layout_wraps_full_and_half_width_cells_with_hanging_punctuation() {
        let mut b = Block::new(200);
        // Five full-width cells fill 120px; a hanging ideographic full stop
        // may pass the end, the next glyph wraps.
        let text = encoding_rs::SHIFT_JIS.encode("あいうえお。か").0;
        assert!(b.append(&text, &style()));
        let pos: Vec<_> = b.glyphs.iter().map(|g| (g.x, g.y)).collect();
        assert_eq!(pos[4], (96, 5));
        assert_eq!(pos[5], (120, 5));
        assert_eq!(pos[6], (0, 34));
        let mut b = Block::new(200);
        b.append(b"ab^Ncd", &style());
        assert_eq!((b.glyphs[2].x, b.glyphs[2].y, b.glyphs[0].advance), (0, 34, 12));
    }
    #[test]
    fn reveal_follows_delays_and_fades() {
        let mut b = Block::new(10);
        let s = Style { speed: 2, ..style() };
        b.append(b"ab", &s);
        let mut r = Reveal::default();
        r.start();
        assert!(r.tick(&b.glyphs));
        assert_eq!((r.revealed, r.levels.clone()), (1, vec![255]));
        assert!(r.tick(&b.glyphs));
        assert_eq!(r.levels, vec![191]);
        r.tick(&b.glyphs);
        assert_eq!(r.revealed, 2);
        r.finish(&b.glyphs);
        assert!(r.complete(&b.glyphs));
    }
}

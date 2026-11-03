// SPDX-License-Identifier: MPL-2.0
//! KCS text window: task class 8 (constructor 5E38C0, handler 5E4540, a Plane
//! subclass whose other messages fall to 5E01F0). The original lays glyph
//! records out itself (5E5AE0 parser, 5E4F60 placement, 5E3B30 vertical
//! alignment) and rasterises them into a bitmap through a GDI glyph cache.
//! Here the adapter keeps that original record layout and publishes the drawn
//! records as an engine GlyphRun; shaping, fonts and coverage stay in CrySirror.
use super::Ctx;
use anyhow::{bail, ensure, Context, Result};
use crysirror_core::{service::HostServices, state::ClientId};
use serde_json::json;
use std::collections::BTreeMap;

/// Natives implemented by this family.
pub(crate) const IDS: &[u32] = &[123, 124, 125, 418, 557];

/// Engine layer ids of a window's edge and face glyph runs (under the Plane payload).
fn glyph_layers(id: u32) -> Result<[u32; 2]> {
    let base = 0x1400_0000u32.checked_add(id.checked_mul(2).context("CatSystem2 text window layer range")?).filter(|v| *v < 0x1600_0000).context("CatSystem2 text window layer range")?;
    Ok([base, base + 1])
}

/// One 40-byte record at this+0x2AC.
#[derive(Clone, Copy, Debug, Default)]
struct Glyph {
    /// +0: SJIS code as a big-endian WORD (lead byte high); 0 is a line break.
    code: u16,
    /// +4: 0xAARRGGBB text colour (this+0xB8 when placed).
    colour: u32,
    /// +8/+0xC: top-left in the window bitmap.
    x: i32,
    y: i32,
    /// +0x10 advance, +0x14 line height (the font size when placed).
    w: i32,
    h: i32,
    /// +0x24: 5E3F90 drew this record (at the position it then had).
    drawn: Option<[i32; 2]>,
}

/// Message 101's 1128-byte block at this+708 (first 96 bytes from the VM).
#[derive(Clone, Debug)]
struct Config {
    words: [i32; 24],
    /// this+0x324/0x328 and this+0x528/0x52C: kinsoku character sets.
    starts: Vec<u16>,
    ends: Vec<u16>,
}
impl Config {
    fn x(&self) -> i32 { self.words[0] }
    fn y(&self) -> i32 { self.words[1] }
    fn width(&self) -> i32 { self.words[2] }
    fn height(&self) -> i32 { self.words[3] }
    /// +0x2D8: 5E4220/5E3F00 copy the bitmap to the Plane only when set.
    fn draws(&self) -> bool { self.words[5] != 0 }
    /// +0x2DC: current font size; '\f' escapes rewrite it in place.
    fn size(&self) -> i32 { self.words[6] }
    /// +0x2E0: ruby font size and mode-1 first line top.
    fn top(&self) -> i32 { self.words[7] }
    /// +0x2E4 letter spacing, +0x2E8 line spacing.
    fn letter(&self) -> i32 { self.words[8] }
    fn line(&self) -> i32 { self.words[9] }
    /// +0x2F0/+0x2F4: cursor reset by message 104; +0x2F4 is mode-2's first line top.
    fn start(&self) -> [i32; 2] { [self.words[11], self.words[12]] }
    /// +0x300 glyph record capacity.
    fn capacity(&self) -> usize { self.words[15].max(0) as usize }
    /// +0x304 colour reset value of this+0xB8.
    fn colour(&self) -> u32 { self.words[16] as u32 }
    /// +0x308: 1 reveals glyphs on the timer instead of at once.
    fn mode(&self) -> i32 { self.words[17] }
    /// +0x310 vertical alignment (5E3B30), +0x314 tag parsing, +0x318 type,
    /// +0x31C kinsoku, +0x320 kinsoku look-ahead.
    fn align(&self) -> i32 { self.words[19] }
    fn tags(&self) -> bool { self.words[20] != 0 }
    fn kind(&self) -> i32 { self.words[21] }
    fn kinsoku(&self) -> bool { self.words[22] != 0 }
}

#[derive(Clone, Debug)]
struct Window {
    config: Option<Config>,
    glyphs: Vec<Glyph>,
    /// this+0x72C/0x730.
    cursor: [i32; 2],
    /// this+0xB8.
    colour: u32,
    /// this+0x750: a leading 「 indents the following lines; +0x754 enables it.
    hanging: bool,
    hanging_enabled: bool,
    /// this+0x758 (message 121): edge colour 0xAARRGGBB.
    edge: u32,
    /// this+0x75C..0x76C: '\fss', '\fs', '\fn', '\fl', '\fll' sizes.
    sizes: [i32; 5],
    published: bool,
}
impl Default for Window {
    /// 5E38C0: everything zero except the edge colour and the five sizes.
    fn default() -> Self {
        Self { config: None, glyphs: Vec::new(), cursor: [0; 2], colour: 0, hanging: false, hanging_enabled: false, edge: 0xff00_0000, sizes: [20; 5], published: false }
    }
}

#[derive(Default)]
pub(crate) struct State {
    windows: BTreeMap<u32, Window>,
}

/// Native 66 built a class 8 task (a Plane with this window state).
pub(crate) fn created(state: &mut State, id: u32) {
    state.windows.insert(id, Window::default());
}

/// 6A49D0: optional sign, then "0x"/"0X" hex or decimal digits; a '.' after
/// the digits fails. Answers (value, consumed bytes).
pub(super) fn number(bytes: &[u8]) -> Option<(i32, usize)> {
    let mut at = usize::from(matches!(bytes.first(), Some(b'-' | b'+')));
    let negative = bytes.first() == Some(&b'-');
    if !bytes.get(at).is_some_and(u8::is_ascii_digit) {
        return None;
    }
    let mut value: i32 = 0;
    if bytes[at] == b'0' && matches!(bytes.get(at + 1), Some(b'x' | b'X')) {
        at += 2;
        while let Some(digit) = bytes.get(at).and_then(|b| (*b as char).to_digit(16).filter(|_| !b.is_ascii_uppercase() || (b'A'..=b'F').contains(b))) {
            value = value.wrapping_shl(4).wrapping_add(digit as i32);
            at += 1;
        }
    } else {
        while let Some(b) = bytes.get(at).filter(|b| b.is_ascii_digit()) {
            value = value.wrapping_mul(10).wrapping_add(i32::from(*b - b'0'));
            at += 1;
        }
        if bytes.get(at) == Some(&b'.') {
            return None;
        }
    }
    Some((if negative { value.wrapping_neg() } else { value }, at))
}

fn is_lead(byte: u8) -> bool {
    matches!(byte, 0x81..=0x9f | 0xe0..=0xfc)
}

impl Window {
    /// 5E4F60's line break: a zero record, the next line, the indent.
    fn break_line(&mut self, config: &Config) -> bool {
        self.glyphs.push(Glyph::default());
        if self.glyphs.len() + 1 >= config.capacity() {
            return false;
        }
        self.cursor[1] += config.line() + config.size();
        self.cursor[0] = if self.hanging { config.letter() + config.size() } else { 0 };
        true
    }
    /// 5E4F60: place one SJIS code (0 = line break). False when the record
    /// table or the window height is full.
    fn place(&mut self, config: &Config, code: u16) -> Result<bool> {
        if self.glyphs.len() + 1 >= config.capacity() {
            return Ok(false);
        }
        if self.hanging_enabled && self.glyphs.is_empty() && code == 0x8175 {
            self.hanging = true;
        }
        if code == 0 {
            return Ok(self.break_line(config));
        }
        let size = config.size();
        let advance = if code & 0xff00 != 0 { size } else { (size - 3) / 2 + 4 };
        ensure!(!config.kinsoku(), "CatSystem2 text window kinsoku (+0x31C) is not implemented");
        if self.cursor[0] + advance > config.width() && !self.break_line(config) {
            return Ok(false);
        }
        if config.line() + self.cursor[1] + config.size() > config.height() {
            return Ok(false);
        }
        self.glyphs.push(Glyph { code, colour: self.colour, x: self.cursor[0], y: self.cursor[1], w: advance, h: config.size(), drawn: None });
        self.cursor[0] += config.letter() + advance;
        Ok(true)
    }
    /// 5E3B30: mode 1 tops every line at +0x2E0, mode 2 bottoms each glyph on
    /// its line (first line top +0x2F4); line height is the tallest glyph.
    fn align(&mut self, config: &Config) {
        let mut top = match config.align() {
            1 => config.top(),
            2 => config.start()[1],
            _ => return,
        };
        let mut start = 0;
        let count = self.glyphs.len();
        for end in 0..=count {
            if end < count && self.glyphs[end].code != 0 {
                continue;
            }
            let height = self.glyphs[start..end].iter().map(|g| g.h).max().unwrap_or(0).max(0);
            for glyph in &mut self.glyphs[start..end] {
                glyph.y = if config.align() == 1 { top } else { top + height - glyph.h };
            }
            top += config.line() + height;
            start = end + 1;
        }
    }
    /// 5E5AE0: parse one formatted string. Answers whether a glyph was placed.
    fn append(&mut self, text: &[u8]) -> Result<bool> {
        let Some(mut config) = self.config.clone() else { return Ok(false) };
        let mut added = false;
        let mut at = 0;
        let mut ruby_base = false;
        while at < text.len() && text[at] != 0 {
            let byte = text[at];
            if is_lead(byte) && at + 1 < text.len() {
                ensure!(!ruby_base, "CatSystem2 text window ruby ('[base/ruby]', 5E5660) is not implemented");
                let code = u16::from_be_bytes([byte, text[at + 1]]);
                if !self.place(&config, code)? {
                    break;
                }
                added = true;
                at += 2;
                continue;
            }
            if !config.tags() {
                if !self.place(&config, u16::from(byte))? {
                    break;
                }
                added = true;
                at += 1;
                continue;
            }
            match byte {
                b'[' => {
                    ruby_base = true;
                    at += 1;
                }
                b'/' if ruby_base => bail!("CatSystem2 text window ruby ('[base/ruby]', 5E5660) is not implemented"),
                b'\\' => {
                    at += 1;
                    let letter = text.get(at).copied().unwrap_or(0);
                    at += 1;
                    match letter {
                        b'c' | b'C' => {
                            if let Some((value, used)) = number(&text[at..]) {
                                self.colour = value as u32;
                                at += used;
                                if text.get(at) == Some(&b';') {
                                    at += 1;
                                }
                            }
                        }
                        b'f' | b'F' => {
                            let next = text.get(at).copied().unwrap_or(0);
                            let size = match next {
                                b'S' | b's' => {
                                    at += 1;
                                    if matches!(text.get(at), Some(b's' | b'S')) { at += 1; self.sizes[0] } else { self.sizes[1] }
                                }
                                b'L' | b'l' => {
                                    at += 1;
                                    if matches!(text.get(at), Some(b'l' | b'L')) { at += 1; self.sizes[4] } else { self.sizes[3] }
                                }
                                b'N' | b'n' => {
                                    at += 1;
                                    self.sizes[2]
                                }
                                _ => match number(&text[at..]) {
                                    Some((value, used)) => {
                                        at += used;
                                        if matches!(next, b'+' | b'-') { config.words[6].wrapping_add(value) } else { value }
                                    }
                                    None => {
                                        at += 1;
                                        self.sizes[2]
                                    }
                                },
                            };
                            config.words[6] = size;
                            if text.get(at) == Some(&b';') {
                                at += 1;
                            }
                        }
                        b'n' | b'N' => {
                            self.place(&config, 0)?;
                        }
                        b'\\' => {
                            if self.place(&config, u16::from(b'\\'))? {
                                added = true;
                            }
                        }
                        // 5E5BCD: any other escape letter is skipped.
                        _ => {}
                    }
                }
                _ => {
                    // A single-byte code that does not fit is skipped, not fatal (5E5BEF).
                    if self.place(&config, u16::from(byte))? {
                        added = true;
                    }
                    at += 1;
                }
            }
        }
        self.config = Some(config.clone());
        self.align(&config);
        Ok(added)
    }
}

fn decode(code: u16) -> Option<char> {
    let bytes = code.to_be_bytes();
    let source: &[u8] = if code & 0xff00 != 0 { &bytes } else { &bytes[1..] };
    let (text, _, bad) = encoding_rs::SHIFT_JIS.decode(source);
    if bad {
        return None;
    }
    text.chars().next()
}

fn rgba(argb: u32) -> [u8; 4] {
    let [b, g, r, _] = argb.to_le_bytes();
    [r, g, b, 255]
}

/// 5E3F90 + 5E4220: draw every undrawn record at its current place, then copy
/// the bitmap to the Plane when the draw flag is set.
fn present(host: &mut dyn HostServices, client: ClientId, font: &str, tasks: &crate::task_policy::NativeTaskPolicy, id: u32, window: &mut Window) -> Result<()> {
    let Some(config) = window.config.clone() else { return Ok(()) };
    for glyph in &mut window.glyphs {
        if glyph.drawn.is_none() && glyph.code != 0 && glyph.code != 0xffff {
            glyph.drawn = Some([glyph.x, glyph.y]);
        }
    }
    if !config.draws() {
        return Ok(());
    }
    let plane = tasks.plane(id)?;
    let payload = plane.payload.context("CatSystem2 text window Plane has no target")?;
    let mut runs = Vec::new();
    for glyph in &window.glyphs {
        let Some([x, y]) = glyph.drawn else { continue };
        let Some(character) = decode(glyph.code) else { continue };
        // 5E3F90 asks the cache for size h-2; GGO bitmaps sit on the cell's ascent.
        let size = (glyph.h - 2).max(1) as f32;
        let ascent = host.font_metrics(font, size, 'あ').context("CatSystem2 text window font metrics")?.ascent;
        runs.push(crysirror_scene::layer::PositionedGlyph { crop: None, character, baseline: [x as f32, y as f32 + ascent], size: Some(size), color: Some(rgba(glyph.colour)), bold: None });
    }
    let [edge_layer, face_layer] = glyph_layers(id)?;
    let rect = [0., 0., config.width().max(1) as f32, config.height().max(1) as f32];
    let mut commands = Vec::new();
    // Type 2/3 set this+0x74C: 6A3EF0 turns the glyph into a 3x3 box sum in the edge colour.
    let edge = match config.kind() {
        2 | 3 => true,
        1 => bail!("CatSystem2 text window shadow type 1 (6A40D0) is not implemented"),
        _ => false,
    };
    if window.published {
        commands.push(json!({"op":"remove","id":edge_layer}));
        commands.push(json!({"op":"remove","id":face_layer}));
    }
    let run = |id: u32, order: i32, colour: Option<[u8; 4]>, blur: Option<crysirror_scene::layer::GlyphBlur>| -> Result<serde_json::Value> {
        let glyphs: Vec<_> = runs.iter().cloned().map(|mut g| { if colour.is_some() { g.color = colour; } g }).collect();
        let paint = crysirror_scene::layer::Paint::GlyphRun { font: font.to_owned(), size: config.size().max(1) as f32, color: [255; 4], bold: false, glyphs, reveal: None, blur };
        Ok(json!({"op":"put","layer":{"id":id,"parent":payload,"rect":rect,"order":[0,order],"paint":serde_json::to_value(paint)?}}))
    };
    if edge {
        let kernel = crysirror_scene::layer::GlyphBlur { radius: 1, level: 256, kernel: crysirror_scene::layer::GlyphCoverageKernel::BoxSum };
        commands.push(run(edge_layer, 0, Some(rgba(window.edge)), Some(kernel))?);
    }
    commands.push(run(face_layer, 1, None, None)?);
    host.submit_engine(client, 1, &serde_json::to_vec(&json!({ "layers": commands }))?)?;
    window.published = true;
    Ok(())
}

pub(crate) fn handle(cx: &mut Ctx) -> Result<bool> {
    let a = cx.call.arguments.clone();
    ensure!(!a.is_empty(), "CatSystem2 text window native {} arguments", cx.call.id);
    let id = a[0];
    // Only 5E4540 has cases 101/103/104/121; any other receiver ignores them.
    let Some(mut window) = cx.state.text_window.windows.remove(&id) else {
        cx.machine.preflight_native(false, &[])?;
        cx.machine.acknowledge(None)?;
        return Ok(true);
    };
    let result = message(cx, id, &a, &mut window);
    cx.state.text_window.windows.insert(id, window);
    result?;
    Ok(true)
}

fn message(cx: &mut Ctx, id: u32, a: &[u32], window: &mut Window) -> Result<()> {
    match cx.call.id {
        // 652150 (418) / 6522E0 (123) → 101: 96 config bytes, then one (123) or
        // two (418) kinsoku sets converted to big-endian SJIS WORDs.
        123 | 418 => {
            ensure!(a.len() == if cx.call.id == 418 { 4 } else { 3 }, "CatSystem2 text window config arguments");
            cx.machine.preflight_native(false, &[])?;
            let block = cx.machine.read_bytes(a[1], 96)?;
            let mut words = [0i32; 24];
            for (i, word) in words.iter_mut().enumerate() {
                *word = i32::from_le_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
            }
            let set = |cx: &mut Ctx, address: u32| -> Result<Vec<u16>> {
                let bytes = cx.machine.read_cstring(address, 512)?;
                let mut codes = Vec::new();
                let mut i = 0;
                while i < bytes.len() {
                    if is_lead(bytes[i]) && i + 1 < bytes.len() {
                        codes.push(u16::from_be_bytes([bytes[i], bytes[i + 1]]));
                        i += 2;
                    } else {
                        codes.push(u16::from(bytes[i]));
                        i += 1;
                    }
                }
                ensure!(codes.len() < 256, "CatSystem2 text window kinsoku set extent");
                Ok(codes)
            };
            let starts = set(cx, a[2])?;
            let ends = if cx.call.id == 418 { set(cx, a[3])? } else { Vec::new() };
            let config = Config { words, starts, ends };
            // 5E4540 case 101 returns at once when the 1128-byte block's word 15 is zero.
            if config.words[15] == 0 {
                cx.machine.acknowledge(None)?;
                return Ok(());
            }
            ensure!(config.width() > 0 && config.height() > 0 && config.width() <= 8192 && config.height() <= 8192, "CatSystem2 text window extent");
            ensure!(config.capacity() <= 65536, "CatSystem2 text window glyph capacity");
            window.colour = config.colour();
            window.sizes = [config.size(); 5];
            window.glyphs.clear();
            if window.published {
                let [edge, face] = glyph_layers(id)?;
                cx.host.submit_engine(cx.client, 1, &serde_json::to_vec(&json!({"layers":[{"op":"remove","id":edge},{"op":"remove","id":face}]}))?)?;
                window.published = false;
            }
            // 5E01F0 message 41 (target w x h) and 51 (base x, y, 0).
            let plane = cx.tasks.plane_mut(id).context("CatSystem2 text window is not a Plane")?;
            crate::native_plane::remove_payload(cx.host, cx.client, plane)?;
            crate::native_plane::create_target(cx.host, cx.client, plane, config.width() as f32, config.height() as f32)?;
            plane.pose.base = [config.x() as f32, config.y() as f32, 0.];
            crate::native_plane::publish(cx.host, cx.client, plane)?;
            window.config = Some(config);
            present(cx.host, cx.client, cx.font, cx.tasks, id, window)?;
            cx.machine.acknowledge(None)?;
        }
        // 652680 → 103: append formatted text, then draw it unless the window
        // reveals on its timer (+0x308 == 1).
        124 => {
            ensure!(a.len() == 2, "CatSystem2 text window append arguments");
            cx.machine.preflight_native(false, &[])?;
            let text = crate::native_values::format_bytes(cx.machine, a[1])?;
            let added = window.append(&text)?;
            if added && window.config.as_ref().is_some_and(|c| c.mode() != 1) {
                present(cx.host, cx.client, cx.font, cx.tasks, id, window)?;
            } else if added {
                bail!("CatSystem2 text window timed reveal (+0x308 == 1) is not implemented");
            }
            cx.machine.acknowledge(None)?;
        }
        // 652080 → 104: forget every record, cursor to +0x2F0/+0x2F4, colour reset;
        // 5E3F00 clears and recopies the bitmap.
        125 => {
            ensure!(a.len() == 1, "CatSystem2 text window clear arguments");
            cx.machine.preflight_native(false, &[])?;
            if let Some(config) = window.config.clone() {
                window.glyphs.clear();
                window.cursor = config.start();
                window.colour = config.colour();
                window.hanging = false;
                present(cx.host, cx.client, cx.font, cx.tasks, id, window)?;
            }
            cx.machine.acknowledge(None)?;
        }
        // 652650 → 121: this+0x758 edge colour.
        557 => {
            ensure!(a.len() == 2, "CatSystem2 text window edge colour arguments");
            cx.machine.preflight_native(false, &[])?;
            window.edge = a[1];
            cx.machine.acknowledge(None)?;
        }
        other => bail!("CatSystem2 native {other} has no handler in the text window family"),
    }
    Ok(())
}

/// Native 67 destroyed task `id`.
pub(crate) fn retire(host: &mut dyn HostServices, client: ClientId, state: &mut State, id: u32) -> Result<()> {
    if let Some(window) = state.windows.remove(&id) {
        if window.published {
            let [edge, face] = glyph_layers(id)?;
            host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers":[{"op":"remove","id":edge},{"op":"remove","id":face}]}))?)?;
        }
    }
    Ok(())
}

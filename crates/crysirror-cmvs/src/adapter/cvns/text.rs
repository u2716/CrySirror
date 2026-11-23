// SPDX-License-Identifier: MPL-2.0
//! CVNS message windows (`0x4217f0`, 0xb8 bytes) and their glyph records.
//!
//! `0x4220f0` places text at once and `0x422340` places it with per-glyph
//! reveal timing. Each glyph is rasterised by `0x407580` into the window
//! buffer and drawn as a type-10 record at the window position. CrySirror
//! draws the same glyphs as one positioned glyph run per window.

use anyhow::{bail, Result};
use serde_json::json;

/// Reveal durations selected by `EFFECT_TIMER` in `0x422340`.
const REVEAL_TIMES: [u32; 5] = [50, 200, 350, 500, 650];

/// Text style `0x4206f0` copies from the settings block for ADV output.
mod checkpoint;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Style {
    /// `TEXT_WAIT` 0..4 scales the window delay by 0, ½, 1, 1½ or 2.
    pub speed: u32,
    /// `EFFECT_TYPE`: 1 fades and 2 zooms each glyph in.
    pub effect: u32,
    /// `EFFECT_TIMER` index into the reveal durations.
    pub timer: u32,
    /// `TEXT_SADOW`.
    pub shadow: bool,
    /// The skip state; places every glyph without delay.
    pub instant: bool,
    /// `TEXT_WAIT_EX_1..4` in 200 ms units after 、 。 ！␣ and ？␣.
    pub pauses: [u32; 4],
}

#[derive(Clone, Debug)]
struct Reveal {
    /// +132: 1 fades, 2 zooms, others end at once.
    mode: u32,
    /// +136.
    delay: u32,
    /// +140.
    duration: u32,
    /// +144.
    elapsed: u32,
}

#[derive(Clone, Debug)]
struct Glyph {
    shadow_color: u32,
    character: char,
    code: u16,
    generation: usize,
    font: std::sync::Arc<str>,
    /// Cell origin inside the window.
    cell: [i32; 2],
    size: u32,
    full_width: bool,
    colors: [u32; 2],
    gradient: u32,
    shadow: Option<u32>,
    /// +32, 0..256.
    alpha: u32,
    /// +40/+42 in thousandths.
    scale: u32,
    reveal: Option<Reveal>,
}

#[derive(Clone, Debug)]
pub(super) struct Window {
    pub native_face: Vec<u8>,
    font_height:u16,
    font_weight:u16,
    /// +20 face name of the GDI font.
    pub font: std::sync::Arc<str>,
    pub default_face: bool,
    /// [2],[3]: buffer size.
    pub buffer: [i32; 2],
    /// [4],[5].
    pub position: [i32; 2],
    /// [6]..[9]: text area.
    pub area: [i32; 4],
    /// [10],[11],[12].
    pub size: i32,
    pub spacing: i32,
    pub line_spacing: i32,
    /// [29]: 、 。 」 』 do not start a new line.
    pub kinsoku: bool,
    /// [30],[31].
    pub colors: [u32; 2],
    /// [32] default shadow flag and [33] shadow colour.
    pub shadow: (bool, u32),
    /// [34] gradient: 1 vertical, 2 horizontal, 3 diagonal.
    pub gradient: u32,
    /// [35],[36] reveal mode and duration without a style.
    pub reveal: [u32; 2],
    /// [37] milliseconds between glyphs.
    pub delay: u32,
    /// [38].
    pub shown: bool,
    /// [39] bucket.
    pub layer: u32,
    /// [40],[41].
    pub cursor: [i32; 2],
    glyphs: Vec<Glyph>,
}

impl Window {
    /// `0x4217f0`.
    pub fn new(font: std::sync::Arc<str>) -> Self {
        Self {
            font,
            default_face: true,
            native_face: Vec::new(),
            font_height:24,
            font_weight:2,
            buffer: [0; 2],
            position: [0; 2],
            area: [0; 4],
            size: 0,
            spacing: 0,
            line_spacing: 0,
            kinsoku: true,
            colors: [0xff_ffff; 2],
            shadow: (true, 0),
            gradient: 0,
            reveal: [0, 0],
            delay: 30,
            shown: true,
            layer: 1,
            cursor: [0; 2],
            glyphs: Vec::new(),
        }
    }

    /// `0x421c00` also returns the cursor to the area origin.
    pub fn set_area(&mut self, area: [i32; 4]) {
        self.area = area;
        self.cursor = [area[0], area[1]];
    }

    /// `0x421dd0`.
    pub fn clear(&mut self) {
        self.glyphs.clear();
        self.cursor = [self.area[0], self.area[1]];
    }

    fn newline(&mut self) {
        self.cursor = [self.area[0], self.cursor[1] + self.size + self.line_spacing];
    }

    /// The `0x4220f0`/`0x422340` wrap test, in the original unsigned compare.
    fn wraps(&self) -> bool {
        let limit = self.area[0] + self.area[2] - self.spacing - self.size;
        self.cursor[0] as u32 >= limit as u32
    }

    /// Places `text`. `reveal` selects `0x422340`; otherwise `0x4220f0`.
    pub fn print(&mut self, text: &[u8], style: Option<Style>, reveal: bool, generation: usize) -> Result<()> {
        let shadow = style.map_or(self.shadow.0, |style| style.shadow);
        let step = match style {
            Some(style) if style.instant => 0,
            Some(style) => match style.speed {
                0 => 0,
                1 => self.delay >> 1,
                2 => self.delay,
                3 => (3 * self.delay) >> 1,
                4 => self.delay * 2,
                speed => bail!("CVNS TEXT_WAIT {speed}"),
            },
            None => self.delay,
        };
        let pauses = match style {
            Some(style) if !style.instant => style.pauses.map(|pause| 200 * pause),
            _ => [0; 4],
        };
        let (mode, duration) = match style {
            Some(style) => {
                let duration = *REVEAL_TIMES
                    .get(style.timer as usize)
                    .ok_or_else(|| anyhow::anyhow!("CVNS EFFECT_TIMER {}", style.timer))?;
                (style.effect, duration)
            }
            None => (self.reveal[0], self.reveal[1]),
        };
        let mut at = 0;
        let mut waiting_click = false;
        let mut elapsed = 0;
        while at < text.len() {
            let byte = text[at];
            if byte == b'\\' {
                match text.get(at + 1) {
                    Some(b'c') => {
                        if reveal {
                            self.clear();
                        }
                        at += 2;
                    }
                    Some(b'n') => {
                        self.newline();
                        at += 2;
                    }
                    Some(b'w') => {
                        if reveal {
                            match text.get(at + 2) {
                                Some(b'c') => {
                                    waiting_click = true;
                                    elapsed = 0;
                                }
                                Some(b's') => waiting_click = false,
                                _ => {}
                            }
                        }
                        at += 3;
                    }
                    // The original loops forever on any other escape.
                    other => bail!("CVNS text escape \\{:?} never ends natively", other.map(|b| *b as char)),
                }
                continue;
            }
            let full_width = (0x81..=0x9f).contains(&byte) || (0xe0..=0xfc).contains(&byte);
            let code = if full_width {
                let low = *text.get(at + 1).ok_or_else(|| anyhow::anyhow!("CVNS text ends inside a double-byte character"))?;
                at += 2;
                u16::from_be_bytes([byte, low])
            } else {
                at += 1;
                u16::from(byte)
            };
            let no_break = full_width && self.kinsoku && matches!(code, 0x8141 | 0x8142 | 0x8176 | 0x8178);
            if self.wraps() && !no_break {
                self.newline();
            }
            // 0x422340 adds a pause after 、 。 and after ！ or ？ followed by a full-width space.
            let next_space = text.get(at..at + 2) == Some(&[0x81, 0x40]);
            let pause = match code {
                0x8141 => 1,
                0x8142 => 2,
                0x8149 if next_space => 3,
                0x8148 if next_space => 4,
                _ => 0,
            };
            let pause = if at >= text.len() { 0 } else { pause };
            let bytes = if full_width { code.to_be_bytes().to_vec() } else { vec![code as u8] };
            let (decoded, _, invalid) = encoding_rs::SHIFT_JIS.decode(&bytes);
            let character = if invalid { '\u{fffd}' } else { decoded.chars().next().unwrap_or(' ') };
            self.font_height=self.size as u16;
            let mut glyph = Glyph {
                shadow_color:self.shadow.1,
                character,
                code,
                generation,
                font: self.font.clone(),
                cell: self.cursor,
                size: self.size.max(0) as u32,
                full_width,
                colors: self.colors,
                gradient: self.gradient & 3,
                shadow: shadow.then_some(self.shadow.1),
                alpha: 256,
                scale: 1000,
                reveal: None,
            };
            if reveal && !waiting_click {
                if step != 0 {
                    match mode {
                        1 => glyph.alpha = 0,
                        2 => glyph.scale = 0,
                        _ => {}
                    }
                    glyph.reveal = Some(Reveal { mode, delay: elapsed, duration, elapsed: 0 });
                }
                elapsed += step;
                if pause > 0 {
                    elapsed += pauses[pause - 1];
                }
            }
            self.glyphs.push(glyph);
            self.cursor[0] += if full_width { self.size + self.spacing } else { self.spacing + (self.size >> 1) };
        }
        Ok(())
    }

    /// `0x421ed0`: glyphs still revealing.
    pub fn revealing(&self) -> usize {
        self.glyphs.iter().filter(|glyph| glyph.reveal.is_some()).count()
    }

    /// `0x421f10`.
    pub fn glyph_count(&self) -> usize {
        self.glyphs.len()
    }

    /// `0x421d80` completes every reveal.
    pub fn complete(&mut self) {
        for glyph in &mut self.glyphs {
            if glyph.reveal.take().is_some() {
                glyph.alpha = 256;
                glyph.scale = 1000;
            }
        }
    }

    /// `0x421f30` per frame.
    pub fn advance(&mut self, delta: u32) {
        for glyph in &mut self.glyphs {
            let Some(reveal) = &mut glyph.reveal else {
                continue;
            };
            let mut step = delta;
            if reveal.delay > 0 {
                if reveal.delay > step {
                    reveal.delay -= step;
                    continue;
                }
                step -= reveal.delay;
                reveal.delay = 0;
                // A delay ending exactly on the frame leaves a revealing glyph unchanged.
                if step == 0 && reveal.mode != 0 {
                    continue;
                }
            }
            reveal.elapsed += step;
            let done = match reveal.mode {
                1 if reveal.elapsed < reveal.duration => {
                    glyph.alpha = (reveal.elapsed << 8) / reveal.duration;
                    false
                }
                1 => {
                    glyph.alpha = 256;
                    true
                }
                2 if reveal.elapsed < reveal.duration => {
                    glyph.scale = 1000 * reveal.elapsed / reveal.duration;
                    false
                }
                2 => {
                    glyph.scale = 1000;
                    true
                }
                _ => true,
            };
            if done {
                glyph.reveal = None;
            }
        }
    }

    /// The glyph run `0x410700` would draw for this window's records.
    pub fn layer(&self, id: u32, order: [i32; 2], offset: [i32; 2],
        mut mask: impl FnMut(super::mask_fonts::Ink, usize) -> Result<Option<String>>,
    ) -> Result<Option<serde_json::Value>> {
        if !self.shown {
            return Ok(None);
        }
        let glyphs: Vec<_> = self
            .glyphs
            .iter()
            // 0x421e70 leaves out glyphs that are still waiting for their delay.
            .filter(|glyph| glyph.reveal.as_ref().is_none_or(|reveal| reveal.delay == 0))
            .filter(|glyph| glyph.alpha > 0 && glyph.scale > 0)
            .map(|glyph| {
                let opacity = (glyph.alpha * 255 / 256) as u8;
                let gradient = match glyph.gradient {
                    1 => Some("vertical"),
                    2 => Some("horizontal"),
                    3 => Some("diagonal"),
                    _ => None,
                }.map(|axis| json!({"axis":axis, "colors":glyph.colors.map(super::rgb), "cell_size":glyph.size}));
                let [r, g, b] = if gradient.is_some() { [255; 3] } else { super::rgb(glyph.colors[0]) };
                let size = glyph.size as f32 * glyph.scale as f32 / 1000.0;
                // 0x421a00 scales around the cell centre.
                let pivot = [
                    if glyph.full_width { (glyph.size >> 1) + 1 } else { (glyph.size >> 2) + 1 } as f32,
                    ((glyph.size >> 1) + 1) as f32,
                ];
                let ratio = glyph.scale as f32 / 1000.0;
                let x = glyph.cell[0] as f32 + pivot[0] * (1.0 - ratio);
                let y = glyph.cell[1] as f32 + pivot[1] * (1.0 - ratio);
                let ink = super::mask_fonts::Ink { code: glyph.code, size: glyph.size,
                    colors: glyph.colors, gradient: glyph.gradient, shadow: glyph.shadow };
                if let Some(asset) = mask(ink, glyph.generation)? {
                    let width = if glyph.full_width { glyph.size } else { glyph.size / 2 } + 1;
                    let height = glyph.size + 1;
                    return Ok(json!({"character":glyph.character, "baseline":[x,y + size], "size":size,
                        "image":{"asset":asset, "rect":[x,y,width as f32 * ratio,height as f32 * ratio],
                        "uv":[0,0,width as f32 / height as f32,1],"tint":[255,255,255,opacity]}}));
                }
                let shadow = glyph.shadow.map(|color| {
                    let [r, g, b] = super::rgb(color);
                    json!({"offset":[1.0, 1.0],"color":[r, g, b, opacity]})
                });
                Ok(json!({"character":glyph.character,"font":glyph.font.as_ref(),"baseline":[x, y + size],"size":size,
                    "color":[r, g, b, opacity],"style":{"shadow":shadow, "gradient":gradient}}))
            })
            .collect::<Result<_>>()?;
        if glyphs.is_empty() {
            return Ok(None);
        }
        let [w, h] = self.buffer.map(|axis| axis.max(1) as f32);
        Ok(Some(json!({"op":"put","layer":{"id":id,"rect":[self.position[0] as f32 + offset[0] as f32, self.position[1] as f32 + offset[1] as f32, w, h],
            "order":order,"paint":{"kind":"glyph_run","font":"default","size":self.size.max(1),
            "color":[255, 255, 255, 255],"glyphs":glyphs}}})))
    }
}

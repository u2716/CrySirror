//! D.C.II P.C. message presentation (41C530 cases 75/76/7, process76
//! 421F90 and process7 422E60). The VM keeps the original name/message
//! buffers, native cell layout (421620/421160), ruby spreading (420E80) and
//! reveal/wait policy. Window frames are composed by the shared bitmap stage
//! and glyphs are a shared retained glyph run; no raster loop lives here.
use super::*;
use crate::dc2_bitmap::Draw;
use crate::machine::{
    gesture_ops::{GestureReply, GestureRequest},
    system_ops::{SystemReply, SystemRequest},
};
use crysirror_core::service::buttons;

/// One native 421160 cell. `color`/`shadow` are resolved palette RGB.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Glyph {
    pub position: [i32; 2],
    pub size: u8,
    pub text: String,
    pub color: [u8; 3],
    pub shadow: Option<[u8; 3]>,
}
/// Window template parts (420090/420520 records, drawn after weather) and
/// the glyphs currently drawn into the native message window.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextFrame {
    pub window: Vec<Draw>,
    pub glyphs: Vec<Glyph>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Message {
    pc: u32,
    /// Name cells (421ED0) followed by message cells (421620), in reveal order.
    name: Vec<Glyph>,
    body: Vec<Glyph>,
    started: Option<u64>,
    shown: usize,
    published: Option<usize>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct TextState {
    /// AFA050: 0x4b name, cleared by a 0x4c not directly preceded by 0x4b.
    pub name: String,
    /// CEBA44: the window template is part of the presented screen.
    pub window_shown: bool,
    /// Window parts presented while CEBA44 is set.
    #[serde(default)]
    pub window: Vec<Draw>,
    pub message: Option<Message>,
    /// Cells drawn on screen after the last completed process76.
    pub displayed: Vec<Glyph>,
    /// Process7 owner: (pc, input observation already armed).
    pub key_wait: Option<u32>,
}
/// 421F90 timer while revealing. The native SetTimer(5) interval is raised
/// by Windows to USER_TIMER_MINIMUM (10 ms); text modes 1/3 have none.
const REVEAL_MS: u64 = 10;
const CONFIRM: u64 = buttons::MOUSE_LEFT | buttons::ENTER;

pub(super) fn wide(c: char) -> bool {
    !(c.is_ascii() || ('\u{ff61}'..='\u{ff9f}').contains(&c))
}
impl Machine {
    fn rgb(&self, index: i32) -> Result<[u8; 3]> {
        let raw = self
            .state
            .palette
            .get(&((index & 63) as u8))
            .copied()
            .context("DC2 text palette")?;
        Ok([raw[1], raw[0], raw[2]])
    }
    ///421160/421290: colour index is style&0x3f; bit 0x80 adds a (+1,+1)
    ///shadow in index 0, or index 9 when the text colour itself is 0.
    pub(super) fn cell(&self, x: i32, y: i32, size: u8, text: String, style: i32) -> Result<Glyph> {
        ensure!(style & 0x40 == 0, "DC2 boxed message cells (style 0x40) are not connected");
        Ok(Glyph {
            position: [x, y],
            size,
            text,
            color: self.rgb(style & 0x3f)?,
            shadow: if style & 0x80 != 0 {
                Some(self.rgb(if style & 0x3f != 0 { 0 } else { 9 })?)
            } else {
                None
            },
        })
    }
    fn template_geometry(&mut self, selector: u8, host: &mut dyn NativeHost) -> Result<Option<Vec<draw::Record>>> {
        let records = self.state.draw.snapshots.get(&selector).cloned().unwrap_or_default();
        let mut visible = Vec::with_capacity(records.len());
        for record in records {
            let Some(shown) = self.display_binding(record.check, host)? else {
                return Ok(None);
            };
            if shown {
                visible.push(record);
            }
        }
        Ok(Some(visible))
    }
    ///420090/420520 geometry fields from the visible type-20 template records.
    fn layout(&mut self, text: &str, host: &mut dyn NativeHost) -> Result<Option<(Vec<Glyph>, Vec<Glyph>)>> {
        let Some(main) = self.template_geometry(1, host)? else { return Ok(None) };
        let Some(plate) = self.template_geometry(2, host)? else { return Ok(None) };
        let font_style = self.state.settings.prefix(18)?.unwrap_or(0) != 0;
        let body = main
            .iter()
            .find(|r| r.kind == 20 && r.size[0] != 0)
            .context("DC2 message window template has no native text origin")?;
        let (origin, line, mut style) = (body.position, body.size[0], body.size[1]);
        if !font_style {
            style &= 0xffbf;
        }
        let mut name = Vec::new();
        let text_name = self.state.text.name.clone();
        if !text_name.is_empty() {
            let field = plate
                .iter()
                .find(|r| r.kind == 20 && r.size[0] != 0)
                .context("DC2 name template has no native text origin")?;
            let width = plate
                .iter()
                .find(|r| r.kind == 20 && r.size[0] == 0)
                .map_or(0, |r| r.position[1]);
            //421ED0 centres strlen() CP932 bytes in 24-px half cells.
            let bytes: i32 = text_name.chars().map(|c| if wide(c) { 2 } else { 1 }).sum();
            let mut x = field.position[0] + (24 * width - 24 * bytes / 2) / 2;
            for c in text_name.chars().filter(|c| *c != '\n') {
                name.push(self.cell(x, field.position[1], 24, c.into(), field.size[1])?);
                x += 24;
            }
        }
        let mut cells = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        let [mut x, mut y] = origin;
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if c == '\n' {
                x = origin[0];
                y += line;
                i += 1;
                continue;
            }
            if c == '@' {
                bail!("DC2 inline @ size/continuation codes are not connected");
            }
            if c == '｛' {
                //420E80: `｛ruby／base｝`; base cells use the ordinary pitch and
                //the 12-px ruby is spread over them at y-11.
                let slash = chars[i + 1..].iter().position(|c| *c == '／').map(|p| p + i + 1);
                let close = chars[i + 1..].iter().position(|c| *c == '｝').map(|p| p + i + 1);
                let (Some(slash), Some(close)) = (slash, close) else {
                    bail!("DC2 unterminated ruby group")
                };
                ensure!(slash < close, "DC2 ruby separator order");
                let ruby = &chars[i + 1..slash];
                let base = &chars[slash + 1..close];
                ensure!(ruby.iter().chain(base).all(|c| wide(*c)), "DC2 half-width ruby group");
                let (rc, bc) = (ruby.len() as i32, base.len() as i32);
                let mut start = x;
                let mut spread = 28 * bc - 12 * rc;
                if spread < 0 {
                    start = x - (12 * rc - 28 * bc) / 2;
                    spread = 0;
                }
                for c in base {
                    cells.push(self.cell(x, y, 28, c.to_string(), style)?);
                    x += 28;
                }
                for (k, c) in ruby.iter().enumerate() {
                    let v14 = 2 * k as i32;
                    let offset = if spread > 0 { spread * (v14 + 1) / rc / 2 } else { 0 };
                    cells.push(self.cell(start + offset + 6 * v14, y - 11, 12, c.to_string(), style & !0x40)?);
                }
                i = close + 1;
                continue;
            }
            //421620: an ideographic space at the line start that ends the line
            //only advances.
            let last = chars.get(i + 1).is_none_or(|c| *c == '\n');
            if !(c == '\u{3000}' && x == origin[0] && last) {
                cells.push(self.cell(x, y, 28, c.to_string(), style)?);
            }
            x += 28;
            i += 1;
        }
        Ok(Some((name, cells)))
    }
    fn window_draws(&mut self, host: &mut dyn NativeHost) -> Result<Option<Vec<Draw>>> {
        let mut draws = Vec::new();
        for selector in [1u8, 2] {
            let Some(records) = self.template_geometry(selector, host)? else { return Ok(None) };
            for record in records {
                if record.kind == 20 {
                    continue;
                }
                let Some(draw) = self.record_draw(&record)? else { continue };
                draws.push(draw);
            }
        }
        Ok(Some(draws))
    }
    pub(super) fn clear_text(&mut self, host: &mut dyn NativeHost) -> Result<bool> {
        if !self.state.text.window_shown && self.state.text.displayed.is_empty() {
            return Ok(true);
        }
        if !host.dc2_text(None)? {
            return Ok(false);
        }
        self.state.text.window_shown = false;
        self.state.text.window.clear();
        self.state.text.displayed.clear();
        Ok(true)
    }
    fn input_released(&mut self, host: &mut dyn NativeHost) -> Result<Option<u64>> {
        let Some(reply) = host.system(SystemRequest::CallbackInput)? else { return Ok(None) };
        let SystemReply::CallbackInput { pressed, .. } = reply else {
            bail!("DC2 message input observation receipt")
        };
        Ok(Some(pressed))
    }
    fn consume(&mut self, mask: u64, host: &mut dyn NativeHost) -> Result<bool> {
        Ok(match host.system(SystemRequest::Gesture(GestureRequest::ConsumeInput { mask }))? {
            None => false,
            Some(SystemReply::Gesture(GestureReply::Done)) => true,
            Some(_) => bail!("DC2 message input-consumption receipt"),
        })
    }
    pub(super) fn execute_text(
        &mut self,
        instruction: &Instruction,
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Option<Outcome>> {
        let skipping = self.state.runtime.get(&25).copied().unwrap_or(0) == 0
            && (control || self.state.runtime.get(&242).copied().unwrap_or(0) != 0);
        match instruction.opcode {
            0x4b => {
                ensure!(!text.contains('$'), "DC2 41B710 variable expansion in names is not connected");
                self.state.text.name = text.to_owned();
                self.advance(instruction.next, instruction.opcode).map(Some)
            }
            0x4c => self.execute_message(instruction, text, host, now, skipping).map(Some),
            0x07 => {
                if self.state.last_opcode != Some(0x4c) {
                    return self.advance(instruction.next, instruction.opcode).map(Some);
                }
                let mode = self.state.settings.prefix(13)?.unwrap_or(0);
                ensure!(matches!(mode, 0 | 1), "DC2 automatic text mode {mode} (422E60 process7) is not connected");
                if skipping {
                    self.state.text.key_wait = None;
                    return self.advance(instruction.next, instruction.opcode).map(Some);
                }
                if self.state.text.key_wait != Some(instruction.offset) {
                    self.state.text.key_wait = Some(instruction.offset);
                    //Publish the completed message before an infinite wait.
                    return Ok(Some(Outcome::WaitUntil(now)));
                }
                let Some(released) = self.input_released(host)? else { return Ok(Some(Outcome::Pending)) };
                if released & (CONFIRM | buttons::PAGE_DOWN) == 0 {
                    return Ok(Some(Outcome::WaitForInput));
                }
                if !self.consume(released & (CONFIRM | buttons::PAGE_DOWN), host)? {
                    return Ok(Some(Outcome::Pending));
                }
                self.state.text.key_wait = None;
                self.advance(instruction.next, instruction.opcode).map(Some)
            }
            _ => Ok(None),
        }
    }
    fn execute_message(
        &mut self,
        instruction: &Instruction,
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
        skipping: bool,
    ) -> Result<Outcome> {
        //B0CE14 (0x16 selector 11, full-screen 4224F0 layout) is not admitted
        //by the selector dispatch, so only the ordinary window path runs here.
        if self.state.text.message.as_ref().is_none_or(|m| m.pc != instruction.offset) {
            ensure!(!text.contains('$'), "DC2 41B710 variable expansion in messages is not connected");
            if self.state.last_opcode != Some(0x4b) {
                self.state.text.name.clear();
            }
            let Some((name, body)) = self.layout(text, host)? else { return Ok(Outcome::Pending) };
            self.state.text.message = Some(Message {
                pc: instruction.offset,
                name,
                body,
                started: None,
                shown: 0,
                published: None,
            });
        }
        let mut message = self.state.text.message.clone().unwrap();
        if message.started.is_none() {
            //421F90 B0CE2C==0: 420E50(1) redraws native0 with the window
            //template, erasing the previous message; 421ED0 draws the name.
            if !self.state.text.window_shown {
                let Some(draw) = self.window_draws(host)? else { return Ok(Outcome::Pending) };
                self.state.text.window = draw;
                self.state.text.window_shown = true;
            }
            message.started = Some(now);
            message.shown = message.name.len();
        }
        let total = message.name.len() + message.body.len();
        let mode = self.state.settings.prefix(13)?.unwrap_or(0);
        let instant = matches!(mode, 1 | 3)
            || self.state.settings.bytes()[..4] != [0; 4]
            || skipping;
        if message.shown < total {
            if instant {
                message.shown = total;
            } else {
                let Some(released) = self.input_released(host)? else { return Ok(Outcome::Pending) };
                if released & CONFIRM != 0 {
                    if !self.consume(released & CONFIRM, host)? {
                        return Ok(Outcome::Pending);
                    }
                    message.shown = total;
                } else {
                    let elapsed = now.saturating_sub(message.started.unwrap_or(now));
                    let count = usize::try_from(elapsed / REVEAL_MS + 1).unwrap_or(usize::MAX);
                    message.shown = (message.name.len() + count).min(total);
                }
            }
        }
        if message.published != Some(message.shown) {
            let glyphs: Vec<Glyph> = message.name.iter().chain(&message.body).take(message.shown).cloned().collect();
            let frame = TextFrame { window: self.state.text.window.clone(), glyphs: glyphs.clone() };
            if !host.dc2_text(Some(&frame))? {
                self.state.text.message = Some(message);
                return Ok(Outcome::Pending);
            }
            self.state.text.displayed = glyphs;
            message.published = Some(message.shown);
        }
        if message.shown < total {
            let started = message.started.unwrap_or(now);
            let due = started + REVEAL_MS * (message.shown - message.name.len()) as u64;
            self.state.text.message = Some(message);
            return Ok(Outcome::WaitUntil(due.max(now + 1)));
        }
        self.state.text.message = None;
        self.advance(instruction.next, instruction.opcode)
    }
}

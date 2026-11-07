// SPDX-License-Identifier: MPL-2.0
//!44C880/44BBB0 text compilation and439F00's native execution backing.
//!Display strings never serve as lpString/6004/62CC save state.
use crate::machine::Machine;
use anyhow::{Context, Result, bail, ensure};
pub(crate) mod cells;
mod reload;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stop {
    Line,
    Page,
    Inline,
    Prompt,
    Cue,
}
#[derive(Clone, Debug)]
pub(crate) struct Prepared {
    pub body: String,
    pub caption: Option<String>,
    pub voice: Option<String>,
    pub native: String,
    pub stop: Stop,
    pub units: usize,
    end: usize,
    next: std::collections::VecDeque<Segment>,
}
#[derive(Clone, Debug)]
struct Segment {
    body: String,
    caption: Option<String>,
    voice: Option<String>,
    end: usize,
    stop: Stop,
}
#[derive(Clone, Debug, Default)]
pub(crate) struct Execution {
    ///Substituted CP932 source in the native419C work buffer, represented as
    ///Unicode only between checked encode/decode boundaries.
    pub buffer: String,
    //4561A0 may place lpString beyond its first newly inserted NUL. Retain
    //the initialized64KiB work image independently of its first C string.
    work_image: Option<Vec<u8>>,
    ///Native62CC accumulated redraw source, including literal escape bytes.
    pub committed: String,
    ///Byte indices relative to buffer, corresponding to6004 and lpString.
    pub resume_start: usize,
    pub cursor: usize,
    ///One-based source node number6000, not physical CRLF or UTF-8 offset.
    pub line: usize,
    pub waiting: i32,
    pub inline_clear: i32,
    pub units: usize,
    stop: Option<Stop>,
    complete: bool,
    projection_valid: bool,
    current_end: usize,
    current_start: usize,
    next: std::collections::VecDeque<Segment>,
}
impl Execution {
    fn work_bytes(&self) -> Result<std::borrow::Cow<'_, [u8]>> {
        if let Some(image) = &self.work_image {
            return Ok(std::borrow::Cow::Borrowed(image));
        }
        let (bytes, _, invalid) = encoding_rs::SHIFT_JIS.encode(&self.buffer);
        ensure!(!invalid, "SugarPot native work CP932");
        Ok(bytes)
    }
    pub(crate) fn blank(&mut self, line: usize, novel: bool) -> Result<bool> {
        let advance = novel && !self.committed.is_empty() && self.inline_clear == 0;
        self.begin_command(line);
        self.buffer.clear();
        if let Some(image) = &mut self.work_image {
            image[0] = 0;
        }
        self.resume_start = 0;
        if advance {
            self.committed.push_str("\\n");
            ensure!(
                encoding_rs::SHIFT_JIS.encode(&self.committed).0.len() < 0x10000,
                "SugarPot native novel blank buffer budget"
            );
        }
        Ok(advance)
    }
    pub(crate) fn layout_segment(&self) -> Result<(String, usize)> {
        let bytes = self.work_bytes()?;
        ensure!(
            self.current_start <= self.current_end && self.current_end <= bytes.len(),
            "SugarPot layout segment boundary"
        );
        let tail = &bytes[self.current_start..];
        let end = tail
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(tail.len());
        ensure!(
            self.current_end - self.current_start <= end,
            "SugarPot layout crosses native NUL"
        );
        Ok((decode(&tail[..end])?, self.current_end - self.current_start))
    }
    pub(crate) fn advances_line_on_completion(&self) -> bool {
        !self.complete && self.stop == Some(Stop::Line)
    }
    ///44C880 increments6000 and resets lpString to419C before dispatching
    ///every node. Command branches retain the old work bytes;6004 is reset
    ///only when62CC is empty. This differs from beginning another text line.
    pub fn begin_command(&mut self, line: usize) {
        self.line = line;
        self.cursor = 0;
        if self.committed.is_empty() {
            self.resume_start = 0;
        }
        if self.buffer.is_empty() && self.committed.is_empty() {
            self.projection_valid = true;
        }
        self.waiting = 0;
        self.stop = None;
        self.current_end = 0;
        self.current_start = 0;
        self.next.clear();
        self.complete = true;
    }
    pub fn begin(&mut self, prepared: &Prepared, line: usize) -> Result<()> {
        let (bytes, _, invalid) = encoding_rs::SHIFT_JIS.encode(&prepared.native);
        ensure!(
            !invalid && bytes.len() < 0x10000,
            "SugarPot native text buffer CP932/budget"
        );
        self.buffer = prepared.native.clone();
        self.work_image = None;
        self.units = prepared.units;
        self.line = line;
        self.resume_start = 0;
        self.cursor = 0;
        self.waiting = 128;
        self.inline_clear = 0;
        self.stop = Some(prepared.stop);
        self.current_end = prepared.end;
        self.current_start = 0;
        self.next = prepared.next.clone();
        self.complete = false;
        self.projection_valid = true;
        Ok(())
    }
    ///A completed reveal has consumed the work buffer.439F00 appends the raw
    ///6004..lpString slice; its NUL branch additionally appends literal\\n.
    pub fn complete(&mut self) -> Result<()> {
        if self.complete {
            return Ok(());
        }
        let bytes = self.work_bytes()?.into_owned();
        ensure!(
            self.current_end <= bytes.len() && self.resume_start <= self.current_end,
            "SugarPot native segment cursor range"
        );
        self.cursor = self.current_end;
        let stop = self.stop.context("SugarPot text execution stop")?;
        if stop == Stop::Cue {
            //A voice opcode is not a redraw-source commit.439F00 leaves6004
            //unchanged while consuming its cue and resuming character output.
            self.waiting = 128;
            self.complete = true;
            return Ok(());
        }
        let committed = &bytes[self.resume_start..self.cursor];
        let end = committed
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(committed.len());
        let slice = encoding_rs::SHIFT_JIS
            .decode_without_bom_handling_and_without_replacement(&committed[..end])
            .context("SugarPot native segment CP932 boundary")?;
        self.committed.push_str(&slice);
        match stop {
            Stop::Line => {
                self.committed.push_str("\\n");
                self.resume_start = 0;
                self.waiting = 8;
            }
            Stop::Page => {
                self.waiting = 16;
                self.resume_start = 0;
            }
            Stop::Inline => {
                self.resume_start = 0;
                self.waiting = 0;
                self.inline_clear = 1;
            }
            Stop::Prompt => {
                self.waiting = 8;
                self.resume_start = self.cursor;
            }
            Stop::Cue => unreachable!(),
        }
        let (bytes, _, invalid) = encoding_rs::SHIFT_JIS.encode(&self.committed);
        ensure!(
            !invalid && bytes.len() < 0x10000,
            "SugarPot native redraw text budget"
        );
        self.complete = true;
        Ok(())
    }
    pub fn has_automatic_boundary(&self) -> bool {
        self.complete && matches!(self.stop, Some(Stop::Cue | Stop::Inline))
    }
    pub fn continuation(&mut self) -> Option<Prepared> {
        if !self.complete || !matches!(self.stop, Some(Stop::Prompt | Stop::Cue)) {
            return None;
        }
        let segment = self.next.pop_front()?;
        self.current_start = self.current_end;
        self.current_end = segment.end;
        self.stop = Some(segment.stop);
        self.waiting = 128;
        self.complete = false;
        Some(Prepared {
            body: segment.body,
            caption: segment.caption,
            voice: segment.voice,
            native: self.buffer.clone(),
            stop: segment.stop,
            units: self.units,
            end: segment.end,
            next: self.next.clone(),
        })
    }
    pub fn confirm(&mut self) {
        if self.stop == Some(Stop::Page) {
            self.committed.clear();
            self.cursor = 0;
            self.resume_start = 0;
        }
        self.waiting = 0;
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn invalidate_continuation(&mut self) {
        self.projection_valid = false;
    }
    pub fn slot(&self) -> Result<(String, String, i32, i32)> {
        ensure!(
            self.projection_valid,
            "SugarPot native source cursor for a display page not mapped"
        );
        ensure!(self.complete, "SugarPot native slot during glyph reveal");
        let bytes = self.work_bytes()?;
        ensure!(
            self.cursor <= bytes.len() && self.resume_start <= bytes.len(),
            "SugarPot native text cursor range"
        );
        let tail = &bytes[self.resume_start..];
        let end = tail
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(tail.len());
        let resume = encoding_rs::SHIFT_JIS
            .decode_without_bom_handling_and_without_replacement(&tail[..end])
            .context("SugarPot native saved resume byte boundary")?
            .into_owned();
        Ok((
            self.committed.clone(),
            resume,
            i32::try_from(self.cursor)?,
            self.waiting,
        ))
    }
}
fn name(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut key = String::new();
    if chars.peek().is_some_and(|c| matches!(c, '$' | '%' | '!')) {
        chars.next();
    }
    while chars
        .peek()
        .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_')
    {
        key.push(chars.next().unwrap());
    }
    key
}
pub(crate) fn prepare(source: &str, machine: &Machine, append_page: bool) -> Result<Prepared> {
    prepare_inner(source, machine, append_page, true)
}
pub(crate) fn prepare_display(source: &str, machine: &Machine) -> Result<Prepared> {
    plan_native(source.to_owned(), machine, 0, false)
}
fn substitution<'a>(machine: &'a Machine, key: &str, allow_defs: bool) -> Result<&'a str> {
    //Live44C880 falls back from VAR to DEF; detached44BBB0 checks VAR only.
    //Both known missing-name branches append no bytes, rather than allocating
    //a dummy variable or invoking an unresolved native operation.
    let value = machine.state.variables.get(key).or_else(|| {
        if allow_defs {
            machine.state.definitions.get(key)
        } else {
            None
        }
    });
    value.map_or(Ok(""), |value| value.string())
}
fn prepare_inner(
    source: &str,
    machine: &Machine,
    append_page: bool,
    emit_voice: bool,
) -> Result<Prepared> {
    let (native, units) = compile_native(source, machine, append_page, true)?;
    plan_native(native, machine, units, emit_voice)
}
fn compile_native(
    source: &str,
    machine: &Machine,
    append_page: bool,
    allow_defs: bool,
) -> Result<(String, usize)> {
    let mut native = String::new();
    let mut units = 0usize;
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        ensure!(
            native.len() <= 0x30000,
            "SugarPot native text compiler budget"
        );
        if c == '[' {
            native.push(c);
            loop {
                ensure!(
                    native.len() <= 0x30000,
                    "SugarPot native speaker compiler budget"
                );
                match chars.next().context("SugarPot unterminated speaker")? {
                    ']' => {
                        native.push(']');
                        break;
                    }
                    '\\' => {
                        //44C880 advances over the marker/code without testing
                        //the code in a speaker header, then performs lookup.
                        chars.next().context("SugarPot speaker substitution code")?;
                        let key = name(&mut chars);
                        native.push_str(substitution(machine, &key, allow_defs)?);
                    }
                    c => native.push(c),
                }
            }
            continue;
        }
        if c == '\n' {
            native.push_str("\\n");
            continue;
        }
        if !matches!(c, '\\' | '@') {
            if !c.is_ascii() {
                let mut utf8 = [0; 4];
                let (bytes, _, invalid) = encoding_rs::SHIFT_JIS.encode(c.encode_utf8(&mut utf8));
                ensure!(!invalid, "SugarPot text CP932 character");
                if bytes.len() == 2 {
                    units += 1;
                }
            }
            native.push(c);
            continue;
        }
        let code = chars.next().context("SugarPot incomplete text escape")?;
        match code.to_ascii_lowercase() {
            's' => {
                let key = name(&mut chars);
                ensure!(!key.is_empty(), "SugarPot text substitution name");
                native.push_str(substitution(machine, &key, allow_defs)?);
            }
            'f' | 'i' => {
                native.push(c);
                native.push(code);
                break;
            }
            'c' => {
                native.push(c);
                native.push(code);
                //The compiler copies the complete ASCII suffix.439F00 and
                //44BEA0 each impose their own16-byte execution/history limit.
                while chars
                    .peek()
                    .is_some_and(|c| c.is_ascii() && !matches!(c, '\\' | '@' | '[' | '\r' | '\n'))
                {
                    native.push(chars.next().unwrap());
                }
            }
            '0'..='9' => {
                units += 1;
                native.push(c);
                native.push(code);
            }
            'n' | 'p' => {
                native.push(c);
                native.push(code);
            }
            _ => {
                //44C880's reviewed default finalizes the compiled prefix and
                //discards the remaining source. It is not an unimplemented
                //operation acknowledged as successful.
                break;
            }
        }
        ensure!(
            native.len() <= 0x30000,
            "SugarPot native text compiler budget"
        );
    }
    //44C880 adds this AFTER compilation, even after an explicit f/i has
    //already truncated the input. The planner records only consumed bytes.
    if append_page {
        native.push_str("\\f");
    }
    Ok((native, units))
}
fn dbcs_lead(byte: u8) -> bool {
    (0x81..=0x9f).contains(&byte) || (0xe0..=0xef).contains(&byte)
}
fn decode(bytes: &[u8]) -> Result<String> {
    Ok(encoding_rs::SHIFT_JIS
        .decode_without_bom_handling_and_without_replacement(bytes)
        .context("SugarPot native CP932 text boundary")?
        .into_owned())
}
///439F00 interprets the substituted419C buffer, not source tokens. This also
///interprets escapes introduced by s substitutions and preserves native
///two-byte ASCII consumption (which may swallow an adjacent escape marker).
fn plan_native(
    native: String,
    machine: &Machine,
    units: usize,
    emit_voice: bool,
) -> Result<Prepared> {
    let (encoded, _, invalid) = encoding_rs::SHIFT_JIS.encode(&native);
    ensure!(
        !invalid && encoded.len() < 0x10000 && !encoded.contains(&0),
        "SugarPot native text buffer CP932/budget"
    );
    let bytes = encoded.into_owned();
    let mut at = 0;
    let mut body = String::new();
    let mut caption = None;
    let mut voice = None;
    let mut segments = Vec::new();
    let mut stop = Stop::Line;
    while at < bytes.len() {
        match bytes[at] {
            b'[' if emit_voice || at == 0 => {
                let start = at + 1;
                let mut end = start;
                while end < bytes.len() && bytes[end] != b']' {
                    end += if dbcs_lead(bytes[end]) { 2 } else { 1 };
                }
                ensure!(end < bytes.len(), "SugarPot native speaker terminator");
                if emit_voice && !body.is_empty() {
                    segments.push(Segment {
                        body: std::mem::take(&mut body),
                        caption: caption.take(),
                        voice: voice.take(),
                        end: at,
                        stop: Stop::Cue,
                    });
                }
                let speaker = decode(&bytes[start..end])?;
                caption = Some(speaker.clone());
                body.push('[');
                body.push_str(&speaker);
                body.push(']');
                at = end + 1;
            }
            b'\\' | b'@' => {
                let marker = at;
                let code = *bytes
                    .get(at + 1)
                    .context("SugarPot native work buffer incomplete escape")?;
                match code {
                    b'n' | b'N' => {
                        body.push('\n');
                        at += 2;
                    }
                    b'p' | b'P' => {
                        at += 2;
                        if emit_voice {
                            segments.push(Segment {
                                body: std::mem::take(&mut body),
                                caption: caption.take(),
                                voice: voice.take(),
                                end: at,
                                stop: Stop::Prompt,
                            });
                        }
                    }
                    b'f' | b'F' => {
                        at += 2;
                        if emit_voice {
                            stop = Stop::Page;
                            break;
                        }
                        //43B3F0 extracts only an initial name, then43C010
                        //skips F/I/P throughout the retained redraw source.
                    }
                    b'i' if emit_voice => {
                        at += 2;
                        stop = Stop::Inline;
                        break;
                    }
                    b'i' | b'I' if !emit_voice => {
                        at += 2;
                    } //44BEA0 history/redraw skips I/P.
                    b'c' | b'C' => {
                        if emit_voice && (!body.is_empty() || voice.is_some()) {
                            segments.push(Segment {
                                body: std::mem::take(&mut body),
                                caption: caption.take(),
                                voice: voice.take(),
                                end: marker,
                                stop: Stop::Cue,
                            });
                        }
                        at += 2;
                        let start = at;
                        while at < bytes.len()
                            && at - start < 16
                            && !dbcs_lead(bytes[at])
                            && !matches!(bytes[at], b'\\' | b'@')
                            && (emit_voice || bytes[at] != b'[')
                        {
                            at += 1;
                        }
                        if emit_voice {
                            let prefix = machine
                                .state
                                .variables
                                .get("cv")
                                .context("SugarPot native CV prefix variable")?
                                .string()?;
                            voice = Some(format!("{prefix}{}", decode(&bytes[start..at])?));
                        }
                    }
                    b'0'..=b'9' => {
                        body.push_str(&decode(&bytes[at..at + 2])?);
                        at += 2;
                    }
                    _ => {
                        //439F00's known default consumes the marker only;
                        //the code byte is the next ordinary two-byte unit.
                        at += 1;
                    }
                }
            }
            byte => {
                let width = if dbcs_lead(byte) {
                    2
                } else if at + 1 < bytes.len() && !dbcs_lead(bytes[at + 1]) {
                    2
                } else {
                    1
                };
                ensure!(
                    at + width <= bytes.len(),
                    "SugarPot truncated native DBCS glyph"
                );
                //Portable repair: do not split a valid DBCS character after
                //an odd single-byte prefix or read beyond the terminal NUL.
                //Determined two-single-byte units retain their native pairing.
                body.push_str(&decode(&bytes[at..at + width])?);
                at += width;
            }
        }
        ensure!(segments.len() < 4096, "SugarPot native text segment budget");
    }
    segments.push(Segment {
        body,
        caption,
        voice,
        end: at,
        stop,
    });
    if !emit_voice {
        let mut body = String::new();
        let mut caption = None;
        for segment in &segments {
            body.push_str(&segment.body);
            if segment.caption.is_some() {
                caption = segment.caption.clone();
            }
        }
        return Ok(Prepared {
            body,
            caption,
            voice: None,
            native: decode(&bytes)?,
            stop,
            units,
            end: at,
            next: Default::default(),
        });
    }
    let mut next = std::collections::VecDeque::from(segments);
    let first = next.pop_front().unwrap();
    Ok(Prepared {
        body: first.body,
        caption: first.caption,
        voice: first.voice,
        native: decode(&bytes)?,
        stop: first.stop,
        units,
        end: first.end,
        next,
    })
}

///44BEA0 captures the final c suffix into the history record's+20 field.
///This is independent of the speaker header and does not play any audio.
pub(crate) fn voice_suffix_override(native: &str) -> Result<Option<String>> {
    let (encoded, _, invalid) = encoding_rs::SHIFT_JIS.encode(native);
    ensure!(
        !invalid && encoded.len() < 0x10000,
        "SugarPot history source CP932/budget"
    );
    let bytes = encoded.as_ref();
    let mut at = 0;
    let mut last = None;
    while at < bytes.len() {
        match bytes[at] {
            b'[' => {
                at += 1;
                while at < bytes.len() && bytes[at] != b']' {
                    at += if dbcs_lead(bytes[at]) { 2 } else { 1 };
                }
                ensure!(at < bytes.len(), "SugarPot history speaker terminator");
                at += 1;
            }
            b'\\' | b'@' => {
                let code = *bytes
                    .get(at + 1)
                    .context("SugarPot history incomplete escape")?;
                match code {
                    b'f' | b'F' => break,
                    b'c' | b'C' => {
                        at += 2;
                        let start = at;
                        //44BEA0 additionally stops before '[';439F00's live
                        //voice helper does not. Keep their distinct semantics.
                        while at < bytes.len()
                            && at - start < 16
                            && !dbcs_lead(bytes[at])
                            && !matches!(bytes[at], b'\\' | b'@' | b'[')
                        {
                            at += 1;
                        }
                        last = Some(decode(&bytes[start..at])?);
                    }
                    b'n' | b'N' | b'i' | b'I' | b'p' | b'P' | b'0'..=b'9' => at += 2,
                    _ => at += 1,
                }
            }
            byte => {
                let width = if dbcs_lead(byte) {
                    2
                } else if at + 1 < bytes.len() && !dbcs_lead(bytes[at + 1]) {
                    2
                } else {
                    1
                };
                ensure!(at + width <= bytes.len(), "SugarPot history truncated DBCS");
                at += width;
            }
        }
    }
    Ok(last)
}

impl Execution {
    ///4561A0/44BBB0 reconstruction at a saved complete page or command safe
    ///point. The native reader rewrites the compiled work buffer from62CC
    ///when the saved redraw source differs, before applying its byte offset.
    pub(crate) fn from_original_slot(
        slot: &crate::native_storage::slot::Slot,
        machine: &Machine,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<(Self, Option<String>)> {
        let pc = usize::try_from(slot.line)?.checked_sub(1);
        let source = pc
            .and_then(|pc| {
                machine.programs[&machine.state.frame.script]
                    .instructions
                    .get(pc)
            })
            .map_or("", |i| i.source.as_str());
        let reload::Work { memory, caption } = reload::rebuild(slot, source, machine, check)?;
        let cursor = usize::try_from(slot.text_offset)?;
        ensure!(
            cursor < memory.len(),
            "SugarPot saved native text cursor exceeds rebuilt work buffer"
        );
        let first = memory.iter().position(|byte| *byte == 0).unwrap();
        let buffer = decode(&memory[..first])?;
        let mut next = std::collections::VecDeque::new();
        let stop = match slot.execution[3] {
            16 => Some(Stop::Page),
            8 if cursor != 0 || memory[0] != 0 => {
                //After a restored8 wait,439F00 resumes at the actual saved
                //pointer. It can be the inserted NUL, or initialized bytes
                //past it. Do not resume from display text or concatenate the
                //saved6004 string:4561A0 deliberately resets6004 to419C.
                let tail = &memory[cursor..];
                let end = tail
                    .iter()
                    .position(|byte| *byte == 0)
                    .context("SugarPot restored work terminator")?;
                let planned = plan_native(decode(&tail[..end])?, machine, 0, true)?;
                check()?;
                next.push_back(Segment {
                    body: planned.body,
                    caption: planned.caption,
                    voice: planned.voice,
                    end: cursor + planned.end,
                    stop: planned.stop,
                });
                for mut segment in planned.next {
                    segment.end += cursor;
                    next.push_back(segment);
                }
                Some(Stop::Prompt)
            }
            8 => Some(Stop::Line),
            0 | 4 | 32 | 64 => None,
            _ => bail!(
                "SugarPot original text/effect continuation {} not fully mapped",
                slot.execution[3]
            ),
        };
        Ok((
            Self {
                buffer,
                work_image: Some(memory),
                committed: slot.current_text.clone(),
                resume_start: 0,
                cursor,
                line: slot.line as usize,
                waiting: slot.execution[3],
                inline_clear: slot.execution[5],
                //4561A0 deliberately uses strlen(62CC)>>1 after reconstruction,
                //where44C880 counted DBCS body cells and digit escapes only.
                units: encoding_rs::SHIFT_JIS.encode(&slot.current_text).0.len() / 2,
                stop,
                complete: true,
                projection_valid: true,
                current_end: cursor,
                current_start: cursor,
                next,
            },
            caption,
        ))
    }
}

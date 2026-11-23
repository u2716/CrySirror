// SPDX-License-Identifier: MPL-2.0

//! Bounded byte-oriented counterpart of 0x14005ca80 (see vm-semantics.md).
use super::Text;

/// 0x14005b050: 1 for a character of the hanging set (+336), 2 for the inseparable set
/// (+400), else 0. Shared by measurement and output.
pub(super) fn line_break_kind(text: &Text, code: u16) -> Result<u8, &'static str> {
    for (kind, bytes) in [
        (1, &text.break_characters[0]),
        (2, &text.break_characters[1]),
    ] {
        // 0x14005b050 walks two bytes at a time; an odd final byte has the NUL
        // terminator as its second byte. Its next iteration would read beyond it.
        if bytes
            .as_chunks::<2>()
            .0
            .iter()
            .any(|b| u16::from_be_bytes([b[0], b[1]]) == code)
        {
            return Ok(kind);
        }
        if bytes.len() % 2 != 0 {
            return Err("odd-length line-break set has an unbounded original scan");
        }
    }
    Ok(0)
}

pub(super) struct Layout<'a> {
    pub(super) text: &'a Text,
    pub(super) x: u32,
    pub(super) y: u32,
    pub(super) line: u32,
    pub(super) count: u32,
    pub(super) max: u32,
    pub(super) overflow: u32,
    pub(super) previous: u16,
}

/// 0x140008080: Shift-JIS lead bytes, without checking the trail byte.
pub(super) fn lead(byte: u8) -> bool {
    (0x81..=0x9f).contains(&byte) || (0xe0..=0xfc).contains(&byte)
}

impl Layout<'_> {
    pub(super) fn limit(&self) -> u32 {
        (self.text.region[0] as u32)
            .wrapping_add(self.text.region[2] as u32)
            .wrapping_sub(self.text.spacing[0])
            .wrapping_sub(self.text.size)
    }

    pub(super) fn newline(&mut self, reset: bool, ruby: bool) -> Result<(), &'static str> {
        self.line += 1;
        // The original indexes the 32-word ruby-line array even when every-line is set.
        if ruby {
            let marked = self
                .text
                .ruby_lines
                .get(self.line as usize)
                .ok_or("text measurement exceeds the 32 ruby-line flags")?;
            if *marked != 0 || self.text.reserve_ruby {
                self.y = self.y.wrapping_add(self.text.ruby_size);
            }
        }
        self.x = self.text.region[0] as u32;
        self.y = self
            .y
            .wrapping_add(self.text.size)
            .wrapping_add(self.text.spacing[1]);
        if reset {
            self.count = 0;
            self.overflow = 0;
            self.previous = 0;
        }
        Ok(())
    }

    fn classify(&self, code: u16) -> Result<u8, &'static str> {
        line_break_kind(self.text, code)
    }

    pub(super) fn wide_wrap(&mut self, code: u16) -> Result<(), &'static str> {
        if self.text.break_configured {
            if self.x < self.limit() {
                self.previous = code;
            } else {
                let kind = self.classify(code)?;
                let allowed = match kind {
                    1 => true,
                    2 => self.previous == code,
                    _ => false,
                };
                if allowed {
                    self.overflow = self.overflow.wrapping_add(1);
                    if self.text.overflow >= self.overflow {
                        return Ok(());
                    }
                }
                self.newline(true, true)?;
            }
        } else if self.x >= self.limit() && !super::output::hangs(self.text, code) {
            // This branch increments the line number without resetting the byte count.
            self.newline(false, true)?;
        }
        Ok(())
    }

    fn add(&mut self, wide: bool) {
        self.count = self.count.wrapping_add(if wide { 2 } else { 1 });
        let size = self.text.size;
        self.x = self
            .x
            .wrapping_add(if wide { size } else { size >> 1 })
            .wrapping_add(self.text.spacing[0]);
        self.max = self.max.max(self.count);
    }

    pub(super) fn ruby_wrap(&mut self) -> Result<(), &'static str> {
        if self.x < self.limit() {
            if self.text.break_configured {
                self.previous = 0;
            }
        } else if self.text.break_configured {
            let kind = self.classify(0)?;
            if kind == 1 || (kind == 2 && self.previous == 0) {
                self.overflow = self.overflow.wrapping_add(1);
                if self.text.overflow >= self.overflow {
                    return Ok(());
                }
            }
            self.newline(true, true)?;
        } else {
            self.newline(false, true)?;
        }
        Ok(())
    }
}

pub(super) fn measure(text: &Text, bytes: &[u8]) -> Result<([u32; 3], [u32; 2]), &'static str> {
    let bytes = bytes
        .split(|byte| *byte == 0)
        .next()
        .expect("split has first item");
    let mut state = Layout {
        text,
        x: text.pen[0] as u32,
        y: text.pen[1] as u32,
        line: 0,
        count: 0,
        max: 0,
        overflow: 0,
        previous: 0,
    };
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                let control = *bytes.get(i + 1).ok_or("truncated text escape")?;
                match control {
                    b'n' => state.newline(true, false)?,
                    b'c' | b't' => {}
                    b'w' => {
                        bytes.get(i + 2).ok_or("truncated text wait escape")?;
                    }
                    b'"' | b'%' | b'@' | b'{' | b'}' => {
                        if state.x >= state.limit() {
                            state.newline(text.break_configured, true)?;
                        }
                        state.add(false);
                    }
                    _ => return Err("unknown text escape would leave the original scan stuck"),
                }
                i += if control == b'w' { 3 } else { 2 };
            }
            b'{' => {
                state.ruby_wrap()?;
                i += 1;
                // Only double-byte characters in the base advance the original counter.
                loop {
                    let byte = *bytes.get(i).ok_or("ruby base is missing its slash")?;
                    i += 1;
                    if lead(byte) {
                        bytes.get(i).ok_or("truncated double-byte ruby character")?;
                        i += 1;
                        state.add(true);
                    } else if byte == b'/' {
                        break;
                    }
                }
                loop {
                    let byte = *bytes
                        .get(i)
                        .ok_or("ruby annotation is missing its closing brace")?;
                    i += 1;
                    if lead(byte) {
                        bytes
                            .get(i)
                            .ok_or("truncated double-byte ruby annotation")?;
                        i += 1;
                    } else if byte == b'}' {
                        break;
                    }
                }
            }
            byte if lead(byte) => {
                let second = *bytes
                    .get(i + 1)
                    .ok_or("truncated double-byte text character")?;
                state.wide_wrap(u16::from_be_bytes([byte, second]))?;
                state.add(true);
                i += 2;
            }
            byte => {
                if state.x >= state.limit() {
                    state.newline(text.break_configured, true)?;
                } else if text.break_configured {
                    // The original sign-extends single-byte characters to a 16-bit word.
                    state.previous = byte as i8 as i16 as u16;
                }
                state.add(false);
                i += 1;
            }
        }
    }
    Ok((
        [state.max, state.line + 1, state.count],
        [state.overflow, u32::from(state.previous)],
    ))
}

/// 0x14006e4e0 (`0x211d`): pixel extents from the font's real cell widths. Returns the widest
/// line, the line count and the final line width. The pen is left unchanged.
pub(super) fn measure_pixels(
    text: &Text,
    bytes: &[u8],
    metrics: &dyn crysirror_core::service::TextMeasurement,
) -> Result<[u32; 3], &'static str> {
    use super::output::{cell_width, escape_w, parse_group, wraps};
    let mut scratch = text.clone();
    scratch.layout_scratch = [0; 2];
    let limit = super::output::line_end(text);
    let spacing = text.spacing[0];
    let (mut x, mut line, mut width, mut widest) = (text.pen[0] as u32, 0u32, 0u32, 0u32);
    let mut i = 0;
    while i < bytes.len() && bytes[i] != 0 {
        let byte = bytes[i];
        let code = if byte == b'\\' {
            match bytes.get(i + 1) {
                Some(b'n') => {
                    (x, width) = (text.region[0] as u32, 0);
                    line += 1;
                    scratch.layout_scratch = [0; 2];
                    i += 2;
                    continue;
                }
                Some(b'c' | b't') => {
                    i += 2;
                    continue;
                }
                Some(b'w') => {
                    i += escape_w(bytes, i)?.0;
                    continue;
                }
                Some(&literal @ (b'"' | b'%' | b'@' | b'{' | b'}')) => {
                    i += 2;
                    if x >= limit {
                        (x, width) = (text.region[0] as u32, 0);
                        line += 1;
                    }
                    u16::from(if literal == b'@' { b'\\' } else { literal })
                }
                _ => return Err("unknown text escape would leave the original scan stuck"),
            }
        } else if byte == b'{' {
            let group = parse_group(bytes, i + 1)?;
            i = group.end;
            if wraps(&mut scratch, 0, x, limit)? {
                (x, width) = (text.region[0] as u32, 0);
                line += 1;
                scratch.layout_scratch = [0; 2];
            }
            for code in group.base.iter().flatten() {
                let advance = cell_width(text, metrics, *code, text.size)?.wrapping_add(spacing);
                x = x.wrapping_add(advance);
                width = width.wrapping_add(advance);
                widest = widest.max(width);
            }
            continue;
        } else if lead(byte) {
            let second = *bytes
                .get(i + 1)
                .filter(|&&b| b != 0)
                .ok_or("truncated double-byte text character")?;
            i += 2;
            let code = u16::from_be_bytes([byte, second]);
            if wraps(&mut scratch, code, x, limit)? {
                (x, width) = (text.region[0] as u32, 0);
                line += 1;
                scratch.layout_scratch = [0; 2];
            }
            code
        } else {
            i += 1;
            if x >= limit {
                (x, width) = (text.region[0] as u32, 0);
                line += 1;
            }
            byte as i8 as i16 as u16
        };
        let advance = cell_width(text, metrics, code, text.size)?.wrapping_add(spacing);
        x = x.wrapping_add(advance);
        width = width.wrapping_add(advance);
        widest = widest.max(width);
    }
    Ok([widest, line + 1, width])
}

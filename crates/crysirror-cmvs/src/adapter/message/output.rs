// SPDX-License-Identifier: MPL-2.0

//! Native CP932 cell layout and glyph descriptors. CrySirror renders the resulting glyph run.

use super::{
    measure::{lead, line_break_kind},
    Arc, Text,
};
use crysirror_core::service::TextMeasurement;

#[derive(Clone, Copy)]
pub(super) struct OutputSettings {
    pub speed: u32,
    pub outline: u32,
    pub reveal_mode: u32,
    pub reveal_duration: u32,
    /// Text objects store the reveal time in milliseconds; windows store a table index.
    pub direct_duration: bool,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct Glyph {
    pub font: Arc<str>,
    pub bold: bool,
    pub character: char,
    pub position: [i32; 2],
    pub size: u32,
    pub color: u32,
    pub outline: Option<u32>,
    delay_ms: u32,
    elapsed_ms: u32,
    duration_ms: u32,
    pub active: bool,
    pub shown: bool,
    pub opacity: u32,
    pub reveal_mode: u32,
    pub reveal_width: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alpha_revision: Option<bool>,
    /// Published image font cell drawn instead of the outline (`0x23c0`).
    #[serde(skip)]
    pub image: Option<Arc<str>>,
}

impl Glyph {
    pub(super) fn advance(&mut self, delta: u32) {
        if !self.active {
            return;
        }
        let available = delta.saturating_sub(self.delay_ms);
        self.delay_ms = self.delay_ms.saturating_sub(delta);
        if available == 0 && (self.delay_ms != 0 || self.reveal_mode != 0) {
            return;
        }
        self.shown = true;
        self.elapsed_ms = self.elapsed_ms.wrapping_add(available);
        if self.reveal_mode == 0 || self.elapsed_ms >= self.duration_ms {
            self.finish();
        } else {
            match self.reveal_mode {
                1 => self.opacity = 255u32.wrapping_mul(self.elapsed_ms) / self.duration_ms,
                2 => {
                    self.reveal_width =
                        self.elapsed_ms.wrapping_mul(self.size + 4) / self.duration_ms
                }
                _ => self.finish(),
            }
        }
    }
    pub(super) fn finish(&mut self) {
        self.active = false;
        self.delay_ms = 0;
        self.shown = true;
        self.opacity = 255;
        self.reveal_width = self.size + 4;
    }
}

/// Line feed of 0x14005bc90 and 0x14005b120: the new line reserves the ruby height when it is
/// marked or every line reserves it, then the pen returns to the area's left edge. The
/// original reads past the 32 ruby-line flags from line 32 on.
pub(super) fn new_line(
    text: &Text,
    line: &mut u32,
    x: &mut u32,
    y: &mut u32,
) -> Result<(), &'static str> {
    *line += 1;
    let marked = *text
        .ruby_lines
        .get(*line as usize)
        .ok_or("message output exceeds the 32 ruby-line flags")?;
    if marked != 0 || text.reserve_ruby {
        *y = y.wrapping_add(text.ruby_size as u32);
    }
    *x = text.region[0] as u32;
    *y = y
        .wrapping_add(text.size as u32)
        .wrapping_add(text.spacing[1] as u32);
    Ok(())
}

const ESCAPES: &str = "text escape that does not advance in the original";

/// `\w` and its selector skip three bytes in 0x14005b360/0x14005bc90; a missing selector
/// would step past the terminator. Returns the length and Some(instant) for `\wc` (true)
/// and `\ws` (false); other selectors change nothing.
pub(super) fn escape_w(bytes: &[u8], i: usize) -> Result<(usize, Option<bool>), &'static str> {
    match bytes.get(i + 2) {
        Some(b'c') => Ok((3, Some(true))),
        Some(b's') => Ok((3, Some(false))),
        Some(&b) if b != 0 => Ok((3, None)),
        _ => Err("text escape \\w without a selector runs past the terminator"),
    }
}

/// Line end shared by the layout functions: area x + width − spacing x − glyph size.
pub(super) fn line_end(text: &Text) -> u32 {
    (text.region[0] as u32)
        .wrapping_add(text.region[2] as u32)
        .wrapping_sub(text.spacing[0] as u32)
        .wrapping_sub(text.size as u32)
}

/// One `{base/ruby}` group as 0x14005bc90 walks it: each loop iteration is a double-byte
/// character (Some) or a single byte (None), the closing `/` or `}` included.
pub(super) struct Group {
    pub(super) base: Vec<Option<u16>>,
    pub(super) ruby: Vec<Option<u16>>,
    /// Index just after the closing brace.
    pub(super) end: usize,
}

/// Parses the group whose `{` precedes `start`. The original reads past a terminator
/// instead of stopping, so a missing `/` or `}` is an error.
pub(super) fn parse_group(bytes: &[u8], start: usize) -> Result<Group, &'static str> {
    fn part(
        bytes: &[u8],
        i: &mut usize,
        close: u8,
        missing: &'static str,
    ) -> Result<Vec<Option<u16>>, &'static str> {
        let mut items = Vec::new();
        loop {
            let byte = *bytes.get(*i).filter(|&&b| b != 0).ok_or(missing)?;
            if lead(byte) {
                let second = *bytes
                    .get(*i + 1)
                    .filter(|&&b| b != 0)
                    .ok_or("truncated double-byte message character")?;
                *i += 2;
                items.push(Some(u16::from_be_bytes([byte, second])));
            } else {
                *i += 1;
                items.push(None);
                if byte == close {
                    return Ok(items);
                }
            }
        }
    }
    let mut i = start;
    let base = part(bytes, &mut i, b'/', "ruby base is missing its slash")?;
    let ruby = part(
        bytes,
        &mut i,
        b'}',
        "ruby annotation is missing its closing brace",
    )?;
    Ok(Group { base, ruby, end: i })
}

/// 0x2119 counts encoded bytes, skipping escapes and ruby annotations. Within a
/// ruby base the original counts double-byte pairs only, even when ASCII is present.
pub(super) fn encoded_length(bytes: &[u8]) -> Result<u32, &'static str> {
    let (mut i, mut count) = (0, 0u32);
    while i < bytes.len() && bytes[i] != 0 {
        match bytes[i] {
            b'\\' => {
                i += match bytes.get(i + 1) {
                    Some(b'c' | b'n' | b't') => 2,
                    Some(b'w') => escape_w(bytes, i)?.0,
                    _ => return Err(ESCAPES),
                }
            }
            b'{' => {
                let group = parse_group(bytes, i + 1)?;
                count = count
                    .wrapping_add(2 * group.base.iter().filter(|v| v.is_some()).count() as u32);
                i = group.end;
            }
            byte if lead(byte) => {
                bytes
                    .get(i + 1)
                    .filter(|&&b| b != 0)
                    .ok_or("truncated double-byte message character")?;
                count = count.wrapping_add(2);
                i += 2;
            }
            _ => {
                count = count.wrapping_add(1);
                i += 1;
            }
        }
    }
    Ok(count)
}

/// 0x14005b360 / CHRONOCLOCK 0x140058840: from the pen, marks ruby group lines
/// and returns [line count, maximum encoded bytes on a line, final line bytes].
/// Its own wrapping ignores 0x211e rules but keeps the +112 punctuation exception, so the
/// marks can differ from the actual layout, as in the original.
pub(super) fn ruby_layout(
    text: &Text,
    bytes: &[u8],
) -> Result<([u32; 32], [u32; 3]), &'static str> {
    let limit = line_end(text);
    let size = text.size as u32;
    let spacing = text.spacing[0] as u32;
    let mut lines = [0; 32];
    let (mut x, mut line) = (text.pen[0] as u32, 0usize);
    let (mut count, mut max) = (0u32, 0u32);
    let mut i = 0;
    while i < bytes.len() && bytes[i] != 0 {
        let byte = bytes[i];
        if byte == b'\\' {
            match bytes.get(i + 1) {
                // 0x14005b547..0x14005b552: explicit newline, without a glyph advance.
                Some(b'n') => {
                    x = text.region[0] as u32;
                    line += 1;
                    count = 0;
                    i += 2;
                }
                // `\c` and `\t` are skipped here without moving the pen.
                Some(b'c' | b't') => i += 2,
                Some(b'w') => i += escape_w(bytes, i)?.0,
                Some(b'"' | b'%' | b'@' | b'{' | b'}') => {
                    if x >= limit {
                        (x, line) = (text.region[0] as u32, line + 1);
                        count = 0;
                    }
                    x = x.wrapping_add(size >> 1).wrapping_add(spacing);
                    count = count.wrapping_add(1);
                    max = max.max(count);
                    i += 2;
                }
                _ => return Err(ESCAPES),
            }
            continue;
        }
        let (advance, wrap) = if byte == b'{' {
            if x >= limit {
                (x, line) = (text.region[0] as u32, line + 1);
                count = 0;
            }
            // The original writes past the 32 flags from line 32 on.
            *lines
                .get_mut(line)
                .ok_or("ruby group beyond the 32 ruby-line flags")? = 1;
            let group = parse_group(bytes, i + 1)?;
            for _ in group.base.iter().flatten() {
                x = x.wrapping_add(size).wrapping_add(spacing);
                count = count.wrapping_add(2);
                max = max.max(count);
            }
            i = group.end;
            continue;
        } else if lead(byte) {
            let second = *bytes
                .get(i + 1)
                .filter(|&&b| b != 0)
                .ok_or("truncated double-byte message character")?;
            i += 2;
            let code = u16::from_be_bytes([byte, second]);
            (size, !hangs(text, code))
        } else {
            i += 1;
            (size >> 1, true)
        };
        if wrap && x >= limit {
            (x, line) = (text.region[0] as u32, line + 1);
            count = 0;
        }
        x = x.wrapping_add(advance).wrapping_add(spacing);
        count = count.wrapping_add(if lead(byte) { 2 } else { 1 });
        max = max.max(count);
    }
    Ok((lines, [line as u32 + 1, max, count]))
}

/// The `+112` exception: these double-byte marks may stay past the line end.
pub(super) fn hangs(text: &Text, code: u16) -> bool {
    text.hang_punctuation && matches!(code, 0x8141 | 0x8142 | 0x8176 | 0x8178)
}

/// CP932 character of a native glyph code.
pub(super) fn decode(code: u16) -> Result<char, &'static str> {
    let data = code.to_be_bytes();
    let encoded = if code < 0x100 || data[0] == 0xff {
        &data[1..]
    } else {
        &data[..]
    };
    let (decoded, bad) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(encoded);
    decoded
        .chars()
        .next()
        .filter(|_| !bad)
        .ok_or("malformed CP932 glyph")
}

/// Cell width of the selected font, the value GDI reports as `gmCellIncX`.
pub(super) fn cell_width(
    text: &Text,
    metrics: &dyn TextMeasurement,
    code: u16,
    size: u32,
) -> Result<u32, &'static str> {
    let character = decode(code)?.to_string();
    metrics
        .measure_simple(&text.font, size as f32, &character)
        .map(|extent| extent.advance.round() as u32)
        .map_err(|_| "font cell measurement failed")
}

/// Wrap decision for a double-byte character, or for a ruby group as code 0: inline in
/// 0x14005bc90 without rules (the +112 punctuation exception), 0x14005b120 (a2 = 0) with
/// them. Past the line end, +528 counts hanging characters and repeats of an inseparable
/// one; past the unsigned limit they wrap as well.
pub(super) fn wraps(text: &mut Text, code: u16, x: u32, limit: u32) -> Result<bool, &'static str> {
    Ok(match text.break_configured {
        false => x >= limit && !hangs(text, code),
        true if x < limit => {
            text.layout_scratch[1] = u32::from(code);
            false
        }
        true => {
            let overflow = match line_break_kind(text, code)? {
                1 => true,
                2 => text.layout_scratch[1] == u32::from(code),
                _ => false,
            };
            if overflow {
                text.layout_scratch[0] = text.layout_scratch[0].wrapping_add(1);
            }
            !overflow || text.overflow < text.layout_scratch[0]
        }
    })
}

/// 0x14005a830: a glyph shown at once; 0x14005b5a0 may then delay its reveal. Window
/// output passes settings +1528 (1) as the outline mode, direct output text +124.
pub(super) fn push(
    text: &mut Text,
    code: u16,
    [x, y]: [u32; 2],
    size: u32,
    outline_mode: u32,
) -> Result<usize, &'static str> {
    if text.glyphs.len() >= 4096 {
        return Err("recorded message glyph limit exceeded");
    }
    let character = decode(code)?;
    text.glyphs.push(Glyph {
        font: text.font.clone(),
        bold: text.bold,
        character,
        position: [x as i32, y as i32],
        size,
        color: text.colors[0],
        outline: (outline_mode != 0).then_some(text.glyph_parameter & 0xffffff),
        delay_ms: 0,
        elapsed_ms: 0,
        duration_ms: 350,
        active: false,
        shown: true,
        opacity: 255,
        reveal_mode: 0,
        reveal_width: size + 4,
        alpha_revision: None,
        image: None,
    });
    Ok(text.glyphs.len() - 1)
}

/// 0x14005b5a0 using the current output settings. After `\wc` (a5 != 0)
/// it returns 0 at once, leaving the glyph shown and restarting the delay. Otherwise a
/// nonzero interval hides the glyph until `delay` has passed and returns delay + interval.
pub(super) fn retime(
    text: &mut Text,
    glyph: Option<usize>,
    delay: u32,
    interval: u32,
    instant: bool,
    settings: OutputSettings,
) -> Result<u32, &'static str> {
    if instant {
        return Ok(0);
    }
    if interval == 0 {
        return Ok(delay);
    }
    let glyph = glyph.and_then(|index| text.glyphs.get_mut(index)).ok_or(
        "ruby markup before any glyph re-times the original's uninitialized glyph pointer",
    )?;
    glyph.delay_ms = delay;
    glyph.elapsed_ms = 0;
    glyph.active = true;
    glyph.shown = false;
    glyph.reveal_mode = settings.reveal_mode;
    glyph.duration_ms = if settings.direct_duration {
        settings.reveal_duration
    } else {
        *[50, 200, 350, 500, 650]
            .get(settings.reveal_duration as usize)
            .ok_or("unknown configured text reveal duration")?
    };
    glyph.opacity = if settings.reveal_mode == 1 { 0 } else { 255 };
    glyph.reveal_width = if settings.reveal_mode == 2 {
        0
    } else {
        glyph.size + 4
    };
    Ok(delay.wrapping_add(interval))
}

/// Uses current settings +1544 (speed), +1536/+1540 (effect) and +1528 (outline).
/// Their constructor defaults are 4/1/2/1; startup INI and VM setters can change them.
/// Wrapping follows 0x14005bc90 with +112 = 1 (constructor and 0x14005d3b0; only the
/// unimplemented 0x2108 clears it). Without line-break rules a double-byte 、。」』 never
/// starts a line; with rules, double-byte characters go through 0x14005b120.
///
/// A `{base/ruby}` group wraps as a whole (as code 0). Its double-byte base characters
/// advance the pen; with ruby enabled, its double-byte annotations use the ruby size on the
/// pen line minus the ruby size, spread in unsigned 22.10 fixed point: with the annotation
/// no wider than the base, from base start + gap/(2n) by ruby size + 2·gap/(2n) + ruby
/// spacing (added unscaled), otherwise centred at the ruby size. Single bytes draw nothing.
/// Every group loop iteration re-times the last glyph through 0x14005b5a0, so `/`, `}` and
/// single bytes delay the previous glyph again and add an interval; annotations start at
/// the group's delay and leave the following text's delay unchanged.
pub(super) fn append(
    text: &mut Text,
    bytes: &[u8],
    skip: bool,
    settings: OutputSettings,
    metrics: Option<&dyn TextMeasurement>,
) -> Result<(), &'static str> {
    let mut x = text.pen[0] as u32;
    let mut y = text.pen[1] as u32;
    let mut line = 0u32;
    let size = text.size as u32;
    let limit = line_end(text);
    text.ruby_lines = if text.ruby {
        ruby_layout(text, bytes)?.0
    } else {
        [0; 32]
    };
    if text.ruby && (text.ruby_lines[0] != 0 || text.reserve_ruby) {
        y = y.wrapping_add(text.ruby_size as u32);
    }
    text.layout_scratch = [0; 2];
    let interval = if skip {
        0
    } else {
        settings.speed.wrapping_mul(text.delay as u32) / 10
    };
    let mut delay = 0u32;
    // `\wc` / `\ws` (v83): reveal glyphs at once until switched back; starts off per call.
    let mut instant = false;
    // 0x14005bc90's last-glyph pointer is uninitialized at the start of every call.
    let mut last = None;
    let mut i = 0;
    while i < bytes.len() && bytes[i] != 0 {
        let byte = bytes[i];
        if byte == b'\\' {
            match bytes.get(i + 1) {
                // 0x14005c46a..0x14005c4b8: no glyph or reveal interval for the newline.
                Some(b'n') => {
                    new_line(text, &mut line, &mut x, &mut y)?;
                    text.layout_scratch = [0; 2];
                    i += 2;
                }
                // `\c` (as 0x14005ad50): drop the glyphs and return the pen to the area
                // origin; the line count and ruby marks stay. The original's last-glyph
                // pointer then refers to a freed glyph, so it is forgotten here.
                Some(b'c') => {
                    text.glyphs.clear();
                    last = None;
                    x = text.region[0] as u32;
                    y = text.region[1] as u32;
                    i += 2;
                }
                // `\t` restarts the reveal delay (v104).
                Some(b't') => {
                    delay = 0;
                    i += 2;
                }
                Some(b'w') => {
                    let (length, selector) = escape_w(bytes, i)?;
                    if let Some(value) = selector {
                        instant = value;
                        if value {
                            delay = 0;
                        }
                    }
                    i += length;
                }
                // Escaped literals draw as single-byte characters; `\@` is a backslash.
                Some(&literal @ (b'"' | b'%' | b'@' | b'{' | b'}')) => {
                    i += 2;
                    let code = u16::from(if literal == b'@' { b'\\' } else { literal });
                    if x >= limit {
                        new_line(text, &mut line, &mut x, &mut y)?;
                        if text.break_configured {
                            text.layout_scratch = [0; 2];
                        }
                    }
                    last = Some(push(text, code, [x, y], size, settings.outline)?);
                    delay = retime(text, last, delay, interval, instant, settings)?;
                    let advance = match metrics {
                        Some(metrics) => cell_width(text, metrics, code, size)?,
                        None => size >> 1,
                    };
                    x = x.wrapping_add(advance).wrapping_add(text.spacing[0] as u32);
                }
                _ => return Err(ESCAPES),
            }
            continue;
        }
        if byte == b'{' {
            let group = parse_group(bytes, i + 1)?;
            i = group.end;
            if wraps(text, 0, x, limit)? {
                new_line(text, &mut line, &mut x, &mut y)?;
                if text.break_configured {
                    text.layout_scratch = [0; 2];
                }
            }
            let (start, ruby_y, group_delay) = (x, y.wrapping_sub(text.ruby_size as u32), delay);
            let mut cells = Vec::new();
            for item in &group.base {
                if let Some(code) = *item {
                    last = Some(push(text, code, [x, y], size, settings.outline)?);
                    let cell = match metrics {
                        Some(metrics) => cell_width(text, metrics, code, size)?,
                        None => size,
                    };
                    cells.push([x, cell]);
                    x = x.wrapping_add(cell).wrapping_add(text.spacing[0] as u32);
                }
                delay = retime(text, last, delay, interval, instant, settings)?;
            }
            if !text.ruby {
                continue;
            }
            let ruby_size = text.ruby_size as u32;
            if let Some(metrics) = metrics {
                // Ruby of the same count made only of U+30FB marks sits over each base glyph.
                let codes: Vec<u16> = group.ruby.iter().flatten().copied().collect();
                let widths = codes
                    .iter()
                    .map(|&code| cell_width(text, metrics, code, ruby_size))
                    .collect::<Result<Vec<_>, _>>()?;
                let total = widths.iter().fold(0u32, |sum, width| sum.wrapping_add(*width));
                let paired = codes.len() == cells.len() && codes.iter().all(|&code| code == 0x8145);
                let width = x.wrapping_sub(start) << 10;
                let mut pen = (start << 10)
                    .wrapping_add(width >> 1)
                    .wrapping_sub(total.wrapping_shl(10) >> 1);
                let mut ruby_delay = group_delay;
                let mut index = 0;
                for item in &group.ruby {
                    if let Some(code) = *item {
                        let rx = if paired {
                            let [base, cell] = cells[index];
                            base.wrapping_add(cell >> 1)
                                .wrapping_sub(widths[widths.len() - 1] >> 1)
                        } else {
                            pen >> 10
                        };
                        last = Some(push(text, code, [rx, ruby_y], ruby_size, settings.outline)?);
                        pen = pen.wrapping_add(widths[index] << 10);
                        index += 1;
                    }
                    ruby_delay = retime(text, last, ruby_delay, interval, instant, settings)?;
                }
                continue;
            }
            let count = group.ruby.iter().flatten().count() as u32;
            let width = x.wrapping_sub(start) << 10;
            let total = count.wrapping_mul(ruby_size) << 10;
            let (mut ruby_x, step) = if total <= width {
                let gap = (width - total).checked_div(count.wrapping_mul(2)).ok_or(
                    "ruby annotation without double-byte characters divides by zero in the original",
                )?;
                let step = (ruby_size << 9).wrapping_add(gap).wrapping_mul(2);
                (
                    (start << 10).wrapping_add(gap),
                    step.wrapping_add(text.ruby_spacing as u32),
                )
            } else {
                (
                    (start << 10).wrapping_sub((total - width) >> 1),
                    ruby_size << 10,
                )
            };
            let mut ruby_delay = group_delay;
            for item in &group.ruby {
                if let Some(code) = *item {
                    last = Some(push(
                        text,
                        code,
                        [ruby_x >> 10, ruby_y],
                        ruby_size,
                        settings.outline,
                    )?);
                    ruby_x = ruby_x.wrapping_add(step);
                }
                ruby_delay = retime(text, last, ruby_delay, interval, instant, settings)?;
            }
            continue;
        }
        let wide = lead(byte);
        let (code, wrap) = if wide {
            let second = *bytes
                .get(i + 1)
                .filter(|&&b| b != 0)
                .ok_or("truncated double-byte message character")?;
            i += 2;
            let code = u16::from_be_bytes([byte, second]);
            (code, wraps(text, code, x, limit)?)
        } else {
            i += 1;
            // Sign-extended to the 16-bit character word.
            let code = byte as i8 as i16 as u16;
            if text.break_configured && x < limit {
                text.layout_scratch[1] = u32::from(code);
            }
            (code, x >= limit)
        };
        if wrap {
            new_line(text, &mut line, &mut x, &mut y)?;
            if text.break_configured {
                text.layout_scratch = [0; 2];
            }
        }
        let gx = if code == 0x814a && metrics.is_none() {
            x.wrapping_sub(size / 5)
        } else {
            x
        };
        last = Some(push(text, code, [gx, y], size, settings.outline)?);
        delay = retime(text, last, delay, interval, instant, settings)?;
        let advance = if let Some(metrics) = metrics {
            cell_width(text, metrics, code, size)?
        } else if code == 0x814a {
            (size >> 1).wrapping_sub(size / 5)
        } else if wide {
            size
        } else {
            size >> 1
        };
        x = x.wrapping_add(advance).wrapping_add(text.spacing[0] as u32);
    }
    text.pen = [x as i32, y as i32];
    Ok(())
}

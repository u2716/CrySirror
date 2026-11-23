// SPDX-License-Identifier: MPL-2.0
//! Text field state beyond the shared text object: scrolling, the movement clamp, the visible
//! frame and native measurement (0x14003f9b0, 0x140041750, 0x14003ffc0).

use super::measure::lead;
use super::output::cell_width;
use super::Text;
use anyhow::Result;
use crysirror_core::service::TextMeasurement;

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub(super) struct Field {
    /// Minimum x, maximum x, minimum y and maximum y (`0x239a`).
    pub clamp: [i32; 4],
    pub motion: Option<Motion>,
    pub attached: Option<u32>,
    /// `0x23b8` overflow rule: prefix, suffix and mode.
    pub rule: Option<(Vec<u8>, Vec<u8>, u32)>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct Motion {
    from: [i32; 2],
    to: [i32; 2],
    duration: u32,
    elapsed: u32,
    mode: u32,
}

impl Field {
    /// 0x140041dd0 and 0x140041e80: refuses a move already resting on its clamp.
    pub fn start(&mut self, position: [i32; 2], delta: [i32; 2], duration: u32, mode: u32) -> bool {
        for axis in 0..2 {
            let [low, high] = [self.clamp[axis * 2], self.clamp[axis * 2 + 1]];
            if (delta[axis] > 0 && position[axis] >= high)
                || (delta[axis] < 0 && position[axis] <= low)
            {
                return false;
            }
        }
        self.motion = Some(Motion {
            from: position,
            to: [position[0] + delta[0], position[1] + delta[1]],
            duration,
            elapsed: 0,
            mode,
        });
        true
    }

    /// 0x14003f9b0: linear, or a quarter sine for mode 1. Reaching the clamp ends the move.
    pub fn advance(&mut self, delta: u32) -> Option<[i32; 2]> {
        let motion = self.motion.as_mut()?;
        motion.elapsed = motion.elapsed.saturating_add(delta);
        let mut done = motion.elapsed >= motion.duration || motion.mode > 1;
        let mut point = if done {
            motion.to
        } else {
            let (elapsed, duration) = (motion.elapsed as i32, motion.duration as i32);
            std::array::from_fn(|axis| {
                let span = motion.to[axis] - motion.from[axis];
                motion.from[axis]
                    + if motion.mode == 1 {
                        let angle = (elapsed as f32 / duration as f32).to_radians() * 90.;
                        (span as f32 * angle.sin()) as i32
                    } else {
                        elapsed.wrapping_mul(span) / duration
                    }
            })
        };
        for axis in 0..2 {
            let [low, high] = [self.clamp[axis * 2], self.clamp[axis * 2 + 1]];
            if point[axis] < low || point[axis] > high {
                done = true;
            }
            point[axis] = point[axis].max(low).min(high);
        }
        if done {
            self.motion = None;
        }
        Some(point)
    }
}

/// 0x140041750 (`0x2390`): line count, widest line and final line in half-width columns.
pub(super) fn count_columns(text: &Text, bytes: &[u8]) -> [u32; 3] {
    let left = text.region[0] as f32;
    let size = text.size as f32;
    let spacing = text.spacing[0] as f32;
    let limit = left + text.region[2] as f32 - (size + spacing);
    let mut x = text.pen[0] as f32;
    let (mut lines, mut column, mut widest) = (0, 0, 0);
    let mut i = 0;
    let byte = |at: usize| bytes.get(at).copied().unwrap_or(0);
    while byte(i) != 0 {
        let wrap = |x: &mut f32, lines: &mut u32, column: &mut u32| {
            if *x >= limit {
                *lines += 1;
                *x = left;
                *column = 0;
            }
        };
        match byte(i) {
            b'\\' => match byte(i + 1) {
                b'"' | b'%' | b'@' | b'{' | b'}' => {
                    i += 2;
                    wrap(&mut x, &mut lines, &mut column);
                    column += 1;
                    widest = widest.max(column);
                    x += size * 0.5 + spacing;
                }
                b'c' | b't' => i += 2,
                b'n' => {
                    x = left;
                    i += 2;
                    lines += 1;
                    column = 0;
                }
                b'w' => i += 3,
                _ => i += 1,
            },
            b'{' => {
                wrap(&mut x, &mut lines, &mut column);
                i += 1;
                while byte(i) != 0 && byte(i) != b'/' {
                    if lead(byte(i)) {
                        column += 2;
                        widest = widest.max(column);
                        x += size + spacing;
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                while byte(i) != 0 && byte(i) != b'}' {
                    i += if lead(byte(i)) { 2 } else { 1 };
                }
                i += usize::from(byte(i) != 0);
            }
            value if lead(value) => {
                let code = u16::from(value) << 8 | u16::from(byte(i + 1));
                i += 2;
                let hangs = matches!(code, 0x8141 | 0x8142 | 0x8176 | 0x8178);
                if x >= limit && !(text.hang_punctuation && hangs) {
                    lines += 1;
                    x = left;
                    column = 0;
                }
                column += 2;
                x += size + spacing;
                widest = widest.max(column);
            }
            _ => {
                i += 1;
                wrap(&mut x, &mut lines, &mut column);
                column += 1;
                x += size * 0.5 + spacing;
                widest = widest.max(column);
            }
        }
    }
    [lines + 1, widest, column]
}

struct Word {
    bytes: Vec<u8>,
    width: f32,
    line: u32,
}

fn alphanumeric(word: &Word) -> bool {
    word.bytes.first().is_some_and(u8::is_ascii_alphanumeric)
}

/// 0x14003ffc0 (`0x23ba`) and 0x14006d800 (`0x20e4`): breaks text at spaces. Returns the wrapped
/// text with the widest line, the line count and the final line width.
pub(super) fn wrap_words(
    text: &Text,
    source: &[u8],
    metrics: &dyn TextMeasurement,
) -> Result<(Vec<u8>, [u32; 3]), &'static str> {
    let spacing = text.spacing[0] as f32;
    let advance = |code| -> Result<f32, &'static str> {
        Ok(cell_width(text, metrics, code, text.size)? as f32 + spacing)
    };
    let mut words = Vec::new();
    let (mut bytes, mut width) = (Vec::new(), 0f32);
    let mut i = 0;
    while let Some(&value) = source.get(i).filter(|&&value| value != 0) {
        match value {
            b' ' => {
                i += 1;
                if !bytes.is_empty() {
                    let bytes = std::mem::take(&mut bytes);
                    words.push(Word { bytes, width, line: 0 });
                }
                width = 0.;
            }
            b'%' | b'{' | b'}' => i += 1,
            b'\\' => match source.get(i + 1).copied().unwrap_or(0) {
                escaped @ (b'"' | b'%' | b'@' | b'{' | b'}') => {
                    bytes.extend_from_slice(&[value, escaped]);
                    width += advance(u16::from(escaped))?;
                    i += 2;
                }
                b'c' | b't' => i += 2,
                b'n' => {
                    i += 2;
                    let bytes = std::mem::take(&mut bytes);
                    words.push(Word { bytes, width, line: 0 });
                    width = 0.;
                }
                b'w' => i += 3,
                _ => i += 1,
            },
            value if lead(value) => {
                let trail = source.get(i + 1).copied().unwrap_or(0);
                bytes.extend_from_slice(&[value, trail]);
                width += advance(u16::from(value) << 8 | u16::from(trail))?;
                i += 2;
            }
            value => {
                bytes.push(value);
                width += advance(u16::from(value))?;
                i += 1;
            }
        }
    }
    if !bytes.is_empty() {
        words.push(Word { bytes, width, line: 0 });
    }
    let space = advance(32)?;
    let left = text.region[0] as f32;
    let limit = left + text.region[2] as f32 - (text.size as f32 + spacing);
    let (mut x, mut line, mut anchor, mut last_word) = (left, 0, 0, None);
    let mut at = 0;
    while at < words.len() {
        words[at].line = line;
        let end = x + words[at].width;
        if end < limit {
            x = end + space;
        } else {
            line += 1;
            if let Some(word) = last_word.filter(|&word| !alphanumeric(&words[at]) && word != anchor)
            {
                at = word;
            }
            words[at].line = line;
            anchor = at;
            x = words[at].width + left;
        }
        if alphanumeric(&words[at]) {
            last_word = Some(at);
        }
        at += 1;
    }
    let mut output = Vec::new();
    let (mut current, mut widest) = (0f32, 0i32);
    for (index, word) in words.iter().enumerate() {
        output.extend_from_slice(&word.bytes);
        current += word.width;
        if let Some(next) = words.get(index + 1) {
            if next.line == word.line {
                output.push(b' ');
                current += space;
            } else {
                output.extend_from_slice(b"\\n");
                widest = widest.max(current as i32);
                current = 0.;
            }
        }
    }
    let lines = words.last().map_or(0, |word| word.line + 1);
    Ok((output, [widest.max(current as i32) as u32, lines, current as u32]))
}

//! Shared page wrapping and exact span/timeline remapping for message streams.
use super::{
    ParsedMessageMarkup, RuntimeLinkSpan, RuntimeRubySpan, RuntimeTextStyleSpan,
    RuntimeTimelineMark,
};
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug)]
pub struct MessageLayoutState {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub font_size: f32,
    pub line_height: f32,
}
pub fn wrap_parsed_message(
    parsed: ParsedMessageMarkup,
    state: &MessageLayoutState,
    advance: &dyn Fn(char) -> f32,
) -> (ParsedMessageMarkup, Vec<usize>) {
    let (wrapped, map) =
        wrap_text_to_state_with_map(&parsed.text, &parsed.ruby_spans, state, advance);
    let ruby_spans = parsed
        .ruby_spans
        .into_iter()
        .filter_map(|span| {
            Some(RuntimeRubySpan {
                start_char: *map.get(span.start_char)?,
                end_char: *map.get(span.end_char)?,
                reading: span.reading,
            })
        })
        .collect();
    let style_spans = parsed
        .style_spans
        .into_iter()
        .filter_map(|span| {
            let start_char = *map.get(span.start_char)?;
            let end_char = *map.get(span.end_char)?;
            (start_char < end_char).then_some(RuntimeTextStyleSpan {
                start_char,
                end_char,
                style: span.style,
            })
        })
        .collect();
    let link_spans = parsed
        .link_spans
        .into_iter()
        .filter_map(|span| {
            Some(RuntimeLinkSpan {
                start_char: *map.get(span.start_char)?,
                end_char: *map.get(span.end_char)?,
                text: span.text,
            })
        })
        .collect();
    let remap = |at_char: usize| map.get(at_char).copied().unwrap_or(wrapped.chars().count());
    let timeline = parsed
        .timeline
        .into_iter()
        .map(|mark| match mark {
            RuntimeTimelineMark::SetTime { at_char, units } => RuntimeTimelineMark::SetTime {
                at_char: remap(at_char),
                units,
            },
            RuntimeTimelineMark::Event {
                at_char,
                ordinal,
                id,
            } => RuntimeTimelineMark::Event {
                at_char: remap(at_char),
                ordinal,
                id,
            },
        })
        .collect();
    let wrapped = ParsedMessageMarkup {
        text: wrapped,
        ruby_spans,
        style_spans,
        link_spans,
        timeline,
    };
    (wrapped, map)
}

/// `next` drawn after `page`: its spans move past the page's text. Only
/// `next`'s timeline records are kept; the page's have already run.
pub fn append_message_markup(
    page: &ParsedMessageMarkup,
    next: ParsedMessageMarkup,
) -> ParsedMessageMarkup {
    let offset = page.text.chars().count();
    let mut out = ParsedMessageMarkup {
        timeline: Vec::new(),
        ..page.clone()
    };
    out.text.push_str(&next.text);
    out.ruby_spans
        .extend(next.ruby_spans.into_iter().map(|span| RuntimeRubySpan {
            start_char: span.start_char + offset,
            end_char: span.end_char + offset,
            reading: span.reading,
        }));
    out.style_spans.extend(
        next.style_spans
            .into_iter()
            .map(|span| RuntimeTextStyleSpan {
                start_char: span.start_char + offset,
                end_char: span.end_char + offset,
                style: span.style,
            }),
    );
    out.link_spans
        .extend(next.link_spans.into_iter().map(|span| RuntimeLinkSpan {
            start_char: span.start_char + offset,
            end_char: span.end_char + offset,
            text: span.text,
        }));
    out.timeline = next
        .timeline
        .into_iter()
        .map(|mark| match mark {
            RuntimeTimelineMark::SetTime { at_char, units } => RuntimeTimelineMark::SetTime {
                at_char: at_char + offset,
                units,
            },
            RuntimeTimelineMark::Event {
                at_char,
                ordinal,
                id,
            } => RuntimeTimelineMark::Event {
                at_char: at_char + offset,
                ordinal,
                id,
            },
        })
        .collect();
    out
}

fn wrap_text_to_state_with_map(
    text: &str,
    protected_spans: &[RuntimeRubySpan],
    state: &MessageLayoutState,
    advance: &dyn Fn(char) -> f32,
) -> (String, Vec<usize>) {
    let size = state.font_size.max(1.0);
    let bound = state.width.max(120.0);
    let max_lines = ((state.height.max(state.line_height) / state.line_height.max(1.0)).floor()
        as usize)
        .max(1);
    let chars = text.chars().collect::<Vec<_>>();
    let mut protected_ends = BTreeMap::new();
    for span in protected_spans {
        if span.start_char < span.end_char && span.end_char <= chars.len() {
            protected_ends.insert(span.start_char, span.end_char);
        }
    }
    let mut out = String::new();
    let mut map = vec![0usize; chars.len() + 1];
    let mut output_chars = 0usize;
    let mut line_x = 0.0f32;
    let mut lines = 1usize;
    let mut index = 0usize;
    while index < chars.len() {
        map[index] = output_chars;
        if chars[index] == '\n' {
            if lines >= max_lines {
                break;
            }
            out.push('\n');
            output_chars += 1;
            line_x = 0.0;
            lines += 1;
            index += 1;
            continue;
        }

        let chunk_end = protected_ends.get(&index).copied().unwrap_or(index + 1);
        let chunk_width = chars[index..chunk_end]
            .iter()
            .map(|ch| advance(*ch))
            .sum::<f32>();
        let single = chunk_end == index + 1;
        let word_start =
            single && is_word_char(chars[index]) && (index == 0 || !is_word_char(chars[index - 1]));
        let needed = if single && chars[index] == ' ' {
            0.0
        } else if word_start {
            let length = chars[index..]
                .iter()
                .take_while(|ch| is_word_char(**ch))
                .count();
            let word = chars[index..index + length]
                .iter()
                .map(|ch| advance(*ch))
                .sum::<f32>();
            // sub_435290's rule for bitmap text: keep one font height free
            // at the right edge unless the word ends in hanging punctuation.
            let reserve = if crate::bitmap_layout::HANGING.contains(chars[index + length - 1]) {
                0.0
            } else {
                size
            };
            if word + reserve > bound {
                chunk_width
            } else {
                word + reserve
            }
        } else if single && is_kinsoku_line_end(chars[index]) {
            chunk_width
                + chars
                    .get(chunk_end)
                    .filter(|ch| **ch != '\n')
                    .map(|ch| advance(*ch))
                    .unwrap_or_default()
        } else {
            chunk_width
                + chars[chunk_end..]
                    .iter()
                    .take_while(|ch| **ch != '\n' && is_kinsoku_line_start(**ch))
                    .map(|ch| advance(*ch))
                    .sum::<f32>()
        };
        if line_x > 0.0 && line_x + needed > bound {
            if lines >= max_lines {
                break;
            }
            out.push('\n');
            output_chars += 1;
            line_x = 0.0;
            lines += 1;
        }
        for ch in &chars[index..chunk_end] {
            map[index] = output_chars;
            out.push(*ch);
            output_chars += 1;
            line_x += advance(*ch);
            index += 1;
        }
    }
    map[index..].fill(output_chars);
    (out, map)
}

/// Glyphs a line keeps together as a word: all but whitespace and the wide
/// CJK glyphs (CJK symbols, kana, ideographs, Hangul, full- and half-width
/// forms).
fn is_word_char(ch: char) -> bool {
    !ch.is_whitespace()
        && !matches!(
            ch as u32,
            0x2E80..=0x9FFF | 0xAC00..=0xD7AF | 0xF900..=0xFAFF | 0xFE30..=0xFE4F | 0xFF00..=0xFFEF
        )
}

fn is_kinsoku_line_start(ch: char) -> bool {
    "\"':;?!ﾞﾟ･，．、。：；？！”゛゜‐]})）〕］｝〉≫》」』】ヽヾゝゞ々ー～っゃゅょッャュョ"
        .contains(ch)
}

fn is_kinsoku_line_end(ch: char) -> bool {
    "[{(（〔［｛〈≪《「『【“".contains(ch)
}

/// Explicit message glyph origins. Vertical flow keeps glyphs upright and
/// advances top-to-bottom, then moves the next column toward the left.
#[derive(Clone, Copy, Debug)]
pub struct MessageGlyphPlacement {
    pub character: char,
    pub index: usize,
    pub position: [f32; 2],
}
pub fn place_message_glyphs(
    text: &str,
    origin: [f32; 2],
    line_height: f32,
    vertical: bool,
    advance: &dyn Fn(char) -> f32,
) -> (Vec<MessageGlyphPlacement>, [f32; 2]) {
    let mut point = origin;
    let mut glyphs = Vec::with_capacity(text.chars().count());
    for (index, ch) in text.chars().enumerate() {
        if ch == '\n' {
            if vertical {
                point[0] -= line_height;
                point[1] = origin[1];
            } else {
                point[0] = origin[0];
                point[1] += line_height;
            }
            continue;
        }
        if (ch as u32) < 0x20 {
            continue;
        }
        glyphs.push(MessageGlyphPlacement {
            character: ch,
            index,
            position: point,
        });
        if vertical {
            point[1] += advance(ch);
        } else {
            point[0] += advance(ch);
        }
    }
    (glyphs, point)
}

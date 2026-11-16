// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! N-System message pages: `MESRUN` items to placed characters
//! (`CMesTextBuffer::ChangePage` 0x4402B0, `Convert` 0x43FEC0) and the
//! reveal/click steps of `CDIBMesWnd::MovePutParag` 0x405F80.
//!
//! Items are CP932 strings or function codes: 0 clears the page (token
//! PutClear), 1 breaks the line (a full area turns the page), 2 turns the
//! page, 3 waits for a click (token PutClick); codes ≥ 100 are glyph
//! escapes (byte 1, code − 99) concatenated to following text; other codes
//! below 100 pass through as raw tokens (the game brackets every message with
//! 10/11, which toggle a window flag at +19044 not read by the reveal code).
//! A double-byte character is `size` wide, a single-byte one `size/2`, plus
//! the character pitch; a line that would pass the right edge wraps, moving
//! back over line-start-forbidden characters (`IsKinsoku` 0x43FD90, table
//! 0x5CD138, enabled by `MESTXKINSOKU`).
use anyhow::{Result, ensure};

/// Line-start-forbidden characters (HANANYU.EXE 0x5CD138, 36 entries).
pub const KINSOKU: &str = ",.)]、。）」』！？ー―…゛・．ぁぃぅぇぉっゃゅょァィゥェォッャュョ";

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Text(Vec<u8>),
    Code(i32),
}
#[derive(Clone, Debug, PartialEq)]
pub struct Glyph {
    pub text: String,
    pub x: i32,
    pub y: i32,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Glyph(Glyph),
    Clear,
    Click,
    Page,
    Flag(i32),
}
#[derive(Clone, Copy, Debug)]
pub struct Area {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub size: i32,
    pub pitch: i32,
    pub line_pitch: i32,
    pub kinsoku: bool,
}
fn chars(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        // `Convert`: lead bytes < 0xA0 (signed < -96), >= 0xE0, or 1 (glyph escape) take two bytes.
        let two = (b >= 0x80 && b < 0xA0) || b >= 0xE0 || b == 1;
        let n = if two { 2 } else { 1 }.min(bytes.len() - i);
        out.push(bytes[i..i + n].to_vec());
        i += n;
    }
    out
}
fn decode(c: &[u8]) -> String {
    if c.first() == Some(&1) {
        return format!("\u{1}{}", c.get(1).copied().unwrap_or(0));
    }
    encoding_rs::SHIFT_JIS.decode(c).0.into_owned()
}
pub fn layout(items: &[Item], area: Area) -> Result<Vec<Token>> {
    ensure!(area.size > 0 && area.right > area.left && area.bottom > area.top, "N-System message area");
    let mut out = Vec::new();
    let (mut x, mut y) = (area.left, area.top);
    let line = area.size + area.line_pitch;
    let mut pending: Vec<u8> = Vec::new();
    let flush = |pending: &mut Vec<u8>, out: &mut Vec<Token>, x: &mut i32, y: &mut i32| {
        if pending.is_empty() {
            return;
        }
        let cs = chars(pending);
        pending.clear();
        let width = |c: &Vec<u8>| area.pitch + area.size * c.len() as i32 / 2;
        let mut i = 0;
        while i < cs.len() {
            // Fit as many characters as the line allows, then back over
            // kinsoku characters so none starts the next line.
            let mut end = i;
            let mut cx = *x;
            while end < cs.len() && cx + width(&cs[end]) <= area.right {
                cx += width(&cs[end]);
                end += 1;
            }
            if end < cs.len() && area.kinsoku {
                while end > i && KINSOKU.contains(&decode(&cs[end])) {
                    end -= 1;
                }
            }
            if end == i && *x == area.left {
                end = i + 1;
            }
            for c in &cs[i..end] {
                out.push(Token::Glyph(Glyph { text: decode(c), x: *x, y: *y }));
                *x += width(c);
            }
            i = end;
            if i < cs.len() {
                *x = area.left;
                *y += line;
                if *y + area.size > area.bottom {
                    out.push(Token::Page);
                    *y = area.top;
                }
            }
        }
    };
    for item in items {
        match item {
            Item::Text(t) => pending.extend(t),
            Item::Code(c) if *c >= 100 => {
                ensure!(*c - 99 <= 255, "N-System glyph escape code");
                pending.extend([1, (*c - 99) as u8]);
            }
            Item::Code(c) => {
                flush(&mut pending, &mut out, &mut x, &mut y);
                match c {
                    0 => {
                        x = area.left;
                        y = area.top;
                        out.push(Token::Clear);
                    }
                    1 => {
                        x = area.left;
                        y += line;
                        if y + area.size > area.bottom {
                            out.push(Token::Page);
                            y = area.top;
                        }
                    }
                    2 => {
                        x = area.left;
                        y = area.top;
                        out.push(Token::Page);
                    }
                    3 => out.push(Token::Click),
                    other => out.push(Token::Flag(*other)),
                }
            }
        }
    }
    flush(&mut pending, &mut out, &mut x, &mut y);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wraps_with_kinsoku_and_codes() {
        let area = Area { left: 0, top: 0, right: 100, bottom: 100, size: 20, pitch: 0, line_pitch: 4, kinsoku: true };
        let text = encoding_rs::SHIFT_JIS.encode("あいうえお、").0.into_owned();
        let tokens = layout(&[Item::Code(10), Item::Code(0), Item::Text(text), Item::Code(3), Item::Code(11)], area).unwrap();
        assert_eq!(tokens[0], Token::Flag(10));
        assert_eq!(tokens[1], Token::Clear);
        // Five fit (100 px), but "、" may not start the next line: "お" wraps with it.
        let glyphs: Vec<_> = tokens.iter().filter_map(|t| if let Token::Glyph(g) = t { Some(g.clone()) } else { None }).collect();
        assert_eq!(glyphs[3], Glyph { text: "え".into(), x: 60, y: 0 });
        assert_eq!(glyphs[4], Glyph { text: "お".into(), x: 0, y: 24 });
        assert_eq!(glyphs[5], Glyph { text: "、".into(), x: 20, y: 24 });
        assert!(matches!(tokens[tokens.len() - 2], Token::Click));
    }
}

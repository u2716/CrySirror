// SPDX-License-Identifier: MPL-2.0
//! Malie message records. Control codes follow the native tokenizer 6631D0
//! used by layout 662530: `7,1 base \n ruby \0` ruby, `7,4` and LF line
//! breaks, `7,6` the `_ms_out` pause boundary, `7,8 name \0` / `7,9` voice
//! begin/end, `7,2 w`/`7,3` and `7,15`/`7,16` render-state toggles. Codes
//! 1..4 and 6 outside an escape carry colour/size operands.
use anyhow::{bail, ensure, Context, Result};

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Token {
    Char(char),
    Newline,
    Ruby { base: String, reading: String },
    Voice(String),
    VoiceEnd,
    Color(Option<[u8; 3]>),
    Size(Option<u16>),
}
/// A message split at its `7,6` pause boundaries; each `_ms_out` emits one.
pub fn segments(text: &str) -> Result<Vec<Vec<Token>>> {
    let units: Vec<char> = text.chars().collect();
    let mut out = vec![Vec::new()];
    let mut i = 0;
    let take_until = |i: &mut usize, end: char| -> Result<String> {
        let mut s = String::new();
        loop {
            let c = *units.get(*i).context("Malie message escape is unterminated")?;
            *i += 1;
            if c == end {
                return Ok(s);
            }
            ensure!(s.len() < 1024, "Malie message escape budget");
            s.push(c);
        }
    };
    while i < units.len() {
        let c = units[i];
        i += 1;
        let current = out.last_mut().unwrap();
        match c {
            '\u{7}' => {
                let code = *units.get(i).context("Malie message escape code")? as u32;
                i += 1;
                match code {
                    1 => {
                        let base = take_until(&mut i, '\n')?;
                        let reading = take_until(&mut i, '\0')?;
                        current.push(Token::Ruby { base, reading });
                    }
                    2 => i += 1,
                    3 | 15 | 16 => {}
                    4 => current.push(Token::Newline),
                    6 => out.push(Vec::new()),
                    8 => {
                        let name = take_until(&mut i, '\0')?;
                        current.push(Token::Voice(name));
                    }
                    9 => current.push(Token::VoiceEnd),
                    other => bail!("Malie message escape 7,{other}"),
                }
            }
            '\n' => current.push(Token::Newline),
            '\u{1}' => {
                let w: Vec<u32> = units.get(i..i + 3).context("Malie colour operand")?.iter().map(|c| *c as u32).collect();
                i += 3;
                current.push(Token::Color(Some([w[0] as u8, (w[0] >> 8) as u8, w[1] as u8])));
            }
            '\u{2}' => current.push(Token::Color(None)),
            '\u{3}' => {
                let size = *units.get(i).context("Malie size operand")? as u32;
                i += 1;
                current.push(Token::Size(Some(size.min(256) as u16)));
            }
            '\u{4}' => current.push(Token::Size(None)),
            '\u{6}' => i += 1,
            '\0' => {}
            c if (c as u32) < 0x20 => bail!("Malie message control {:#x}", c as u32),
            c => current.push(Token::Char(c)),
        }
    }
    // The record ends with its final pause; drop the trailing empty segment.
    if out.len() > 1 && out.last().is_some_and(Vec::is_empty) {
        out.pop();
    }
    Ok(out)
}
/// Plain text of tokens for backlog/history.
pub fn plain(tokens: &[Token]) -> String {
    let mut s = String::new();
    for t in tokens {
        match t {
            Token::Char(c) => s.push(*c),
            Token::Newline => s.push('\n'),
            Token::Ruby { base, .. } => s.push_str(base),
            _ => {}
        }
    }
    s
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn splits_pauses_and_ruby() {
        let t = "\u{7}\u{8}v_a\0\u{3000}輝く\u{7}\u{1}希望\nヒカリ\0を\u{7}\u{9}\u{7}\u{6}\n次\u{7}\u{6}";
        let s = segments(t).unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!(s[0][0], Token::Voice("v_a".into()));
        assert!(s[0].contains(&Token::Ruby { base: "希望".into(), reading: "ヒカリ".into() }));
        assert_eq!(plain(&s[1]), "\n次");
    }
}

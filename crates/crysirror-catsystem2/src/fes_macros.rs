// SPDX-License-Identifier: MPL-2.0
//! Original 766510/766720 one-pass, case-sensitive identifier substitution.
//! This is VM lexical policy, not a general text formatter or preprocessor.
use anyhow::{ensure, Result};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub(crate) struct Macros(BTreeMap<Vec<u8>, Vec<u8>>);
impl Macros {
    pub fn insert(&mut self, name: Vec<u8>, value: Vec<u8>) -> Result<()> {
        ensure!(
            !name.is_empty() && name.len() < 512 && value.len() < 512,
            "CatSystem2 FES macro record extent"
        );
        ensure!(
            !name.contains(&0) && !value.contains(&0),
            "CatSystem2 FES macro NUL"
        );
        ensure!(
            self.0.contains_key(&name) || self.0.len() < 4096,
            "CatSystem2 FES macro allocation budget"
        );
        //766510 replaces an existing exact key, rather than keeping the first.
        self.0.insert(name, value);
        Ok(())
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn expand(&self, source: &[u8], check: &dyn Fn() -> Result<()>) -> Result<Vec<u8>> {
        ensure!(
            source.len() < 4096 && !source.contains(&0),
            "CatSystem2 FES macro input extent/NUL"
        );
        //The original bypasses the lexer altogether when its map is empty.
        if self.0.is_empty() {
            check()?;
            return Ok(source.to_vec());
        }
        let mut output = Vec::new();
        let mut at = 0;
        while at < source.len() {
            check()?;
            let tail = &source[at..];
            if tail[0] == 26 {
                break;
            }
            //878760 consumes comments/line continuation before producing a
            //token. They are absent from the raw token copied by766720.
            if tail.starts_with(b"\\\r\n") {
                at += 3;
                continue;
            }
            if tail.starts_with(b"//") {
                if let Some(end) = tail.windows(2).position(|w| w == b"\r\n") {
                    at += end;
                    continue;
                }
                break;
            }
            if tail.starts_with(b"/*") {
                let end = tail.windows(2).position(|w| w == b"*/");
                let Some(end) = end else {
                    break;
                };
                at += end + 2;
                continue;
            }
            let start = at;
            let mut identifier = None;
            if matches!(tail[0], b'\'' | b'"') {
                let quote = tail[0];
                at += 1;
                let mut closed = false;
                while at < source.len() {
                    let byte = source[at];
                    if quote == b'"' && sjis_lead(byte) {
                        // A UTF-8 script's last byte can look like a lead byte: the row just ends.
                        at = (at + 2).min(source.len());
                    } else if byte == quote {
                        at += 1;
                        closed = true;
                        break;
                    } else if byte == b'\\'
                        && source.get(at + 1).is_some_and(|next| {
                            *next == quote || (quote == b'\'' && *next == b'\\')
                        })
                    {
                        at += 2;
                    } else {
                        at += 1;
                    }
                }
                let _ = closed; // An apostrophe in free text has no partner: the rest of the row is its literal.
            } else if tail[0].is_ascii_digit() {
                if tail.starts_with(b"0x") || tail.starts_with(b"0X") {
                    at += 2;
                    while source.get(at).is_some_and(u8::is_ascii_hexdigit) {
                        at += 1;
                    }
                } else {
                    while source.get(at).is_some_and(u8::is_ascii_digit) {
                        at += 1;
                    }
                    //877940 recognizes an integer before877780 tries float.
                    //Consequently1F is integer1 followed by identifierF.
                    if source.get(at) == Some(&b'.') {
                        at += 1;
                        while source.get(at).is_some_and(u8::is_ascii_digit) {
                            at += 1;
                        }
                        if source.get(at).is_some_and(|b| matches!(b, b'f' | b'F')) {
                            at += 1;
                        }
                    }
                }
            } else if tail[0].is_ascii_alphabetic() || tail[0] == b'_' || escaped_quote(tail) {
                let mut name = Vec::new();
                while at < source.len() {
                    if source[at].is_ascii_alphanumeric() || source[at] == b'_' {
                        name.push(source[at]);
                        at += 1;
                    } else if escaped_quote(&source[at..]) {
                        name.push(source[at + 1]);
                        at += 2;
                    } else {
                        break;
                    }
                }
                identifier = Some(name);
            } else if sjis_lead(tail[0]) {
                at = (at + 2).min(source.len());
            } else {
                at += 1;
            }
            let raw = &source[start..at];
            let value = identifier.as_ref().and_then(|name| self.0.get(name));
            output.extend_from_slice(value.map(Vec::as_slice).unwrap_or(raw));
            ensure!(
                output.len() < 4096,
                "CatSystem2 FES expanded native line buffer extent"
            );
        }
        check()?;
        Ok(output)
    }
}
fn sjis_lead(byte: u8) -> bool {
    matches!(byte, 0x81..=0x9f | 0xe0..=0xfc)
}
fn escaped_quote(bytes: &[u8]) -> bool {
    bytes.first() == Some(&b'\\') && bytes.get(1).is_some_and(|b| matches!(b, b'\'' | b'"'))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn substitutions_are_exact_one_pass_with_native_numeric_and_quoted_tokens() {
        let mut macros = Macros::default();
        for (name, value) in [
            (b"A".as_slice(), b"B".as_slice()),
            (b"B", b"7"),
            (b"F", b"9"),
        ] {
            macros.insert(name.to_vec(), value.to_vec()).unwrap();
        }
        assert_eq!(
            macros
                .expand(b"A B a AA 1F 1.0F 0xAB \"A\" 'A'", &|| Ok(()))
                .unwrap(),
            b"B 7 a AA 19 1.0F 0xAB \"A\" 'A'"
        );
        macros.insert(b"A".to_vec(), b"42".to_vec()).unwrap();
        assert_eq!(macros.expand(b"A", &|| Ok(())).unwrap(), b"42");
    }
    #[test]
    fn original_raw_tokens_preserve_trivia_but_skip_comments_and_continuations() {
        let mut macros = Macros::default();
        macros.insert(b"A".to_vec(), b"8".to_vec()).unwrap();
        assert_eq!(
            macros
                .expand(b" A\t/*A*/ + \\\r\nA// A\r\nA", &|| Ok(()))
                .unwrap(),
            b" 8\t + 8\r\n8"
        );
        assert_eq!(
            macros
                .expand(b"\x81\x41 A \"\x81\x5c\"", &|| Ok(()))
                .unwrap(),
            b"\x81\x41 8 \"\x81\x5c\""
        );
        assert!(macros.expand(b"A", &|| anyhow::bail!("cancelled")).is_err());
        macros.insert(b"A".to_vec(), vec![b'x'; 511]).unwrap();
        assert!(macros.expand(b"A A A A A A A A A", &|| Ok(())).is_err());
    }
}

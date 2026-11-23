// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! KCS byte-string natives over VM memory (dispatch table A2DD38). Strings are the
//! original's NUL-terminated Shift-JIS bytes; nothing here decodes them.
use crate::kcs::Machine;
use anyhow::Result;

type Outcome = (Option<u32>, Vec<(u32, Vec<u8>)>);
const LIMIT: usize = 65534;
fn terminated(mut bytes: Vec<u8>) -> Vec<u8> {
    bytes.push(0);
    bytes
}
/// 6A7590: a Shift-JIS lead byte followed by a valid trail byte.
fn double_byte(bytes: &[u8]) -> bool {
    match bytes {
        [lead, trail, ..] if matches!(*lead, 0x81..=0x9f | 0xe0..=0xef) => matches!(*trail, 0x40..=0x7e) || *trail as i8 <= -4,
        _ => false,
    }
}
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return None;
    }
    haystack.windows(needle.len()).position(|window| window == needle)
}
/// 6A54F0: replace every occurrence.
pub(crate) fn replace(source: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(source.len());
    let mut rest = source;
    while let Some(at) = find(rest, from) {
        out.extend_from_slice(&rest[..at]);
        out.extend_from_slice(to);
        rest = &rest[at + from.len()..];
    }
    out.extend_from_slice(rest);
    out
}
/// 6A57D0: split on delimiter bytes into a NUL list. A parenthesis group is one token
/// copied whole, `(:...)` without its markers, and an alphabetic token before it ends.
pub(crate) fn tokenize(source: &[u8], delimiters: &[u8]) -> Vec<Vec<u8>> {
    let mut tokens = Vec::new();
    let mut current = Vec::new();
    let mut at = 0;
    while at < source.len() {
        let byte = source[at];
        if byte == b'(' {
            if current.last().is_some_and(u8::is_ascii_alphabetic) {
                tokens.push(core::mem::take(&mut current));
            }
            let mut depth = 0;
            let mut end = at;
            while end < source.len() {
                match source[end] {
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    _ => {}
                }
                end += 1;
                if depth == 0 {
                    break;
                }
            }
            let group = &source[at..end];
            if group.starts_with(b"(:") {
                current.extend_from_slice(group[2..].strip_suffix(b")").unwrap_or(&group[2..]));
            } else {
                current.extend_from_slice(group);
            }
            at = end;
            continue;
        }
        if delimiters.contains(&byte) {
            if !current.is_empty() {
                tokens.push(core::mem::take(&mut current));
            }
        } else {
            current.push(byte);
        }
        at += 1;
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}
pub(crate) fn apply(id: u32, a: &[u32], machine: &Machine) -> Result<Option<Outcome>> {
    let text = |index: usize| machine.read_cstring(a[index], LIMIT);
    Ok(Some(match id {
        25 => {
            let source = crate::native_values::format_bytes(machine, a[0])?;
            let value = crate::native_values::integer_prefix(&source).map_or(0, |(_, value)| value);
            (Some(value), Vec::new())
        }
        38 => {
            let source = text(1)?;
            let raw = text(2)?;
            let delimiters = replace(&raw, b"\\t", b"\t");
            let limit = (a[3] as i32 - 2).max(0) as usize;
            let length = source.iter().take(limit).position(|b| delimiters.contains(b)).unwrap_or(source.len().min(limit));
            let mut out = source[..length].to_vec();
            out.extend([0, 0]);
            (Some(length as u32), vec![(a[0], out)])
        }
        43 => {
            let mut out = Vec::new();
            for token in tokenize(&text(1)?, &text(2)?) {
                out.extend(token);
                out.push(0);
            }
            out.extend([0, 0]);
            (None, vec![(a[0], out)])
        }
        44 => {
            // 655230: join NUL-list items first..=last with spaces; -1 means through the end.
            let (first, last) = (a[2] as i32, if a[3] == u32::MAX { i32::MAX } else { a[3] as i32 });
            let mut at = a[1];
            let mut index = 0;
            let mut out = Vec::new();
            loop {
                let item = machine.read_cstring(at, LIMIT)?;
                if item.is_empty() || index > last {
                    break;
                }
                if index >= first {
                    out.extend_from_slice(&item);
                    out.push(b' ');
                }
                at += item.len() as u32 + 1;
                index += 1;
            }
            (None, vec![(a[0], terminated(out))])
        }
        45 => (None, vec![(a[0], terminated(replace(&text(1)?, &text(2)?, &text(3)?)))]),
        46 => {
            // 655070: "N%" of a whole becomes its value, written back in place.
            let source = text(0)?;
            let Some(number) = source.strip_suffix(b"%").and_then(crate::native_values::integer_prefix) else { return Ok(Some((None, Vec::new()))) };
            let value = (number.1 as i32 as f32 * a[1] as f32 / 100.) as i32;
            (None, vec![(a[0], terminated(value.to_string().into_bytes()))])
        }
        47 => {
            let source = text(0)?;
            let (mut at, mut count) = (0, 0u32);
            while at < source.len() {
                at += if double_byte(&source[at..]) { 2 } else { 1 };
                count += 1;
            }
            (Some(count), Vec::new())
        }
        49 => {
            let (source, needle) = (text(0)?, text(1)?);
            let mut at = 0;
            let mut answer = source.len();
            while at < source.len() {
                if source[at..].starts_with(&needle) {
                    answer = at;
                    break;
                }
                at += if double_byte(&source[at..]) { 2 } else { 1 };
            }
            (Some(answer as u32), Vec::new())
        }
        519 => {
            let tokens = tokenize(&text(1)?, b" \t");
            match tokens.get(a[2] as usize) {
                Some(token) => {
                    let mut out = token.clone();
                    out.extend([0, 0]);
                    (Some(1), vec![(a[0], out)])
                }
                None => (Some(0), vec![(a[0], vec![0])]),
            }
        }
        540 => {
            // 659A30: byte offset of the Nth space-separated word; -1 past the end.
            let source = text(0)?;
            let mut at = 0;
            let mut word = 0;
            let answer = loop {
                if word >= a[1] {
                    break at as u32;
                }
                while at < source.len() && source[at] != b' ' {
                    at += 1;
                }
                if at >= source.len() {
                    break u32::MAX;
                }
                while at < source.len() && source[at] == b' ' {
                    at += 1;
                }
                word += 1;
            };
            (Some(answer), Vec::new())
        }
        _ => return Ok(None),
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tokens_split_on_delimiters_and_keep_groups() {
        assert_eq!(tokenize(b"bg  a(b c) d", b" "), [b"bg".to_vec(), b"a".to_vec(), b"(b c)".to_vec(), b"d".to_vec()]);
        assert_eq!(tokenize(b"(:x y) z", b" "), [b"x y".to_vec(), b"z".to_vec()]);
    }
    #[test]
    fn replace_handles_every_occurrence() {
        assert_eq!(replace(b"a0xb0x", b"0x", b"-"), b"a-b-");
        assert_eq!(replace(b"abc", b"", b"-"), b"abc");
    }
    #[test]
    fn shift_jis_pairs_are_recognised() {
        assert!(double_byte(&[0x82, 0xa0]));
        assert!(!double_byte(b"ab"));
    }
}

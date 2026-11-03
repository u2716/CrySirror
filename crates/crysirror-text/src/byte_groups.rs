// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Raw-byte parenthesis/quote token buffers. Delimiters are a byte set; this
//! profile does not decode Unicode/SJIS or process backslash escapes. Parenthesis
//! groups nest; quotes inside them have no special meaning. Normal quote groups
//! remove their quotes but retain the source routine's two NUL bytes and consumed
//! cursor width. Thus a quoted group can end visible token enumeration early.
//! `(:...)` and `":..."` retain the exact historical copy/unwrap byte behavior.
//! This is intentionally distinct from byte_split's delimiter-only contract.
use anyhow::{ensure, Context, Result};
use std::ops::Range;

pub const MAX_BYTE_GROUP_INPUT: usize = 1024 * 1024;
pub const MAX_BYTE_GROUP_OUTPUT: usize = 2 * MAX_BYTE_GROUP_INPUT + 8;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ByteGroupSplitLimits {
    pub max_input_bytes: usize,
    pub max_output_bytes: usize,
    pub max_tokens: usize,
    pub max_depth: usize,
}
impl Default for ByteGroupSplitLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: MAX_BYTE_GROUP_INPUT,
            max_output_bytes: MAX_BYTE_GROUP_OUTPUT,
            max_tokens: 65536,
            max_depth: 64,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ByteGroupSplitOutput {
    /// Every byte actually written, including helper-copy tails beyond the last
    /// logical NUL pair. Native numeric-buffer adapters must retain those writes.
    pub written_bytes: Vec<u8>,
    /// Cursor plus its final two NUL bytes; may be shorter than written_bytes.
    pub logical_len: usize,
    /// Visible nonempty token ranges in written_bytes, not in the source input.
    /// Enumeration stops at the first empty C-string, as a NUL-list reader does.
    pub spans: Vec<Range<usize>>,
}
struct Writer<'a> {
    bytes: Option<&'a mut [u8]>,
    extent: usize,
    limit: usize,
}
impl Writer<'_> {
    fn put(&mut self, at: usize, value: u8) -> Result<()> {
        let end = at.checked_add(1).context("byte group output overflow")?;
        ensure!(end <= self.limit, "byte group output capacity");
        self.extent = self.extent.max(end);
        if let Some(bytes) = &mut self.bytes {
            bytes[at] = value;
        }
        Ok(())
    }
    fn zero_pair(&mut self, at: usize) -> Result<()> {
        self.put(at, 0)?;
        self.put(at + 1, 0)
    }
    fn copy(&mut self, at: usize, source: &[u8]) -> Result<()> {
        let end = at
            .checked_add(source.len())
            .context("byte group output overflow")?;
        ensure!(end <= self.limit, "byte group output capacity");
        self.extent = self.extent.max(end);
        if let Some(bytes) = &mut self.bytes {
            bytes[at..end].copy_from_slice(source);
        }
        Ok(())
    }
    fn unwrap(&mut self, at: usize, len: usize) -> Result<()> {
        let end = at.checked_add(len).context("byte group output overflow")?;
        ensure!(end <= self.limit, "byte group output capacity");
        self.extent = self.extent.max(end);
        if let Some(bytes) = &mut self.bytes {
            bytes.copy_within(at + 2..at + 2 + len, at);
        }
        Ok(())
    }
}
fn run(
    source: &[u8],
    delimiters: &[u8],
    max_depth: usize,
    writer: &mut Writer<'_>,
    check: &dyn Fn() -> Result<()>,
) -> Result<usize> {
    let mut set = [false; 256];
    for b in delimiters {
        set[usize::from(*b)] = true;
    }
    let (mut input, mut output, mut previous, mut at_delimiter) = (0usize, 0usize, 0u8, true);
    while input < source.len() {
        check()?;
        let b = source[input];
        if b == b'(' || b == b'"' {
            if previous.is_ascii_alphabetic() && !at_delimiter {
                writer.put(output, 0)?;
                output += 1;
            }
            let mut end = input + 1;
            if b == b'(' {
                let mut depth = 1usize;
                ensure!(depth <= max_depth, "byte group nesting budget");
                while end < source.len() {
                    check()?;
                    match source[end] {
                        b'(' => {
                            depth += 1;
                            ensure!(depth <= max_depth, "byte group nesting budget");
                        }
                        b')' => {
                            depth -= 1;
                            if depth == 0 {
                                end += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    end += 1;
                }
                writer.copy(output, &source[input..end])?;
                writer.zero_pair(output + end - input)?;
            } else {
                while end < source.len() && source[end] != b'"' {
                    check()?;
                    end += 1;
                }
                let content_end = end;
                writer.copy(output, &source[input + 1..content_end])?;
                writer.zero_pair(output + content_end - input - 1)?;
                if end < source.len() {
                    end += 1;
                }
            }
            let consumed = end - input;
            if source.get(input + 1) == Some(&b':') {
                ensure!(
                    consumed >= 3,
                    "byte group unwrap would underflow native extent"
                );
                writer.unwrap(output, consumed - 3)?;
                writer.put(output + consumed - 3, 0)?;
                output += consumed - 3;
            } else {
                output += consumed;
            }
            input = end;
            at_delimiter = false;
            //The original routine deliberately does not update previous here.
        } else {
            if set[usize::from(b)] {
                if !at_delimiter {
                    writer.put(output, 0)?;
                    output += 1;
                    previous = 0;
                    at_delimiter = true;
                }
            } else {
                writer.put(output, b)?;
                output += 1;
                previous = b;
                at_delimiter = false;
            }
            input += 1;
        }
    }
    writer.zero_pair(output)?;
    check()?;
    Ok(output + 2)
}
/// Capacity/overflow/depth are checked by a complete first pass before output
/// allocation. Cancellation on either pass publishes no partial token buffer.
pub fn split_grouped_bytes(
    source: &[u8],
    delimiters: &[u8],
    limits: ByteGroupSplitLimits,
    check: &dyn Fn() -> Result<()>,
) -> Result<ByteGroupSplitOutput> {
    check()?;
    ensure!(
        limits.max_input_bytes <= MAX_BYTE_GROUP_INPUT
            && limits.max_output_bytes <= MAX_BYTE_GROUP_OUTPUT
            && limits.max_tokens <= 65536
            && limits.max_depth <= 256,
        "byte group configured budget"
    );
    ensure!(
        source.len() <= limits.max_input_bytes && !source.contains(&0),
        "byte group input extent/NUL"
    );
    ensure!(delimiters.len() <= 256, "byte group delimiter budget");
    let mut preflight = Writer {
        bytes: None,
        extent: 0,
        limit: limits.max_output_bytes,
    };
    let logical_len = run(source, delimiters, limits.max_depth, &mut preflight, check)?;
    let mut written_bytes = vec![0; preflight.extent];
    let mut writer = Writer {
        bytes: Some(&mut written_bytes),
        extent: 0,
        limit: limits.max_output_bytes,
    };
    ensure!(
        run(source, delimiters, limits.max_depth, &mut writer, check)? == logical_len,
        "byte group pass mismatch"
    );
    let mut spans = Vec::new();
    let mut at = 0;
    while at < logical_len && written_bytes[at] != 0 {
        check()?;
        ensure!(spans.len() < limits.max_tokens, "byte group token budget");
        let end = at
            + written_bytes[at..logical_len]
                .iter()
                .position(|b| *b == 0)
                .context("byte group missing terminator")?;
        spans.push(at..end);
        at = end + 1;
    }
    check()?;
    Ok(ByteGroupSplitOutput {
        written_bytes,
        logical_len,
        spans,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn split(b: &[u8]) -> ByteGroupSplitOutput {
        split_grouped_bytes(b, b" ", Default::default(), &|| Ok(())).unwrap()
    }
    fn tokens(b: &[u8]) -> Vec<Vec<u8>> {
        let o = split(b);
        o.spans
            .iter()
            .map(|s| o.written_bytes[s.clone()].to_vec())
            .collect()
    }
    #[test]
    fn raw_groups_nested_parentheses_and_saved_ascii_prefix_are_preserved() {
        assert_eq!(
            tokens(b"  a(b)(c)z  d"),
            [
                b"a".to_vec(),
                b"(b)".to_vec(),
                b"(c)z".to_vec(),
                b"d".to_vec()
            ]
        );
        assert_eq!(
            tokens(b"if ($1 == (2 + 3))"),
            [b"if".to_vec(), b"($1 == (2 + 3))".to_vec()]
        );
        assert_eq!(
            tokens(b"x (a\"(b)\"c)"),
            [b"x".to_vec(), b"(a\"(b)\"c)".to_vec()]
        );
        assert_eq!(tokens(b"(unclosed"), [b"(unclosed".to_vec()]);
    }
    #[test]
    fn quote_consumed_width_and_colon_unwrap_keep_actual_nul_and_tail_bytes() {
        let o = split(b"a \"b c\" z");
        assert_eq!(
            o.spans
                .iter()
                .map(|s| &o.written_bytes[s.clone()])
                .collect::<Vec<_>>(),
            [b"a".as_slice(), b"b c"]
        );
        assert!(o.written_bytes.windows(2).any(|b| b == b"z\0"));
        let o = split(b"(:abc)");
        assert_eq!(o.logical_len, 5);
        assert_eq!(o.written_bytes, b"abc\0\0)\0\0");
        assert_eq!(tokens(b"\":abc\""), [b"bc".to_vec()]);
        assert_eq!(tokens(b"(:abc"), [b"ab".to_vec()]);
        assert!(split_grouped_bytes(b"(:", b" ", Default::default(), &|| Ok(())).is_err());
        assert_eq!(split(b"").written_bytes, b"\0\0");
    }
    #[test]
    fn byte_delimiters_do_not_guess_sjis_and_limits_and_cancel_are_atomic() {
        let o = split_grouped_bytes(&[0x81, 0x5c, b'a'], b"\\", Default::default(), &|| Ok(()))
            .unwrap();
        assert_eq!(
            o.spans
                .iter()
                .map(|s| o.written_bytes[s.clone()].to_vec())
                .collect::<Vec<_>>(),
            [vec![0x81], vec![b'a']]
        );
        let mut limit = ByteGroupSplitLimits::default();
        limit.max_output_bytes = 5;
        assert!(split_grouped_bytes(b"(:abc)", b" ", limit, &|| Ok(()))
            .unwrap_err()
            .to_string()
            .contains("capacity"));
        let mut limit = ByteGroupSplitLimits::default();
        limit.max_depth = 1;
        assert!(split_grouped_bytes(b"((x))", b" ", limit, &|| Ok(())).is_err());
        let mut limit = ByteGroupSplitLimits::default();
        limit.max_tokens = 1;
        assert!(split_grouped_bytes(b"a b", b" ", limit, &|| Ok(())).is_err());
        let calls = std::cell::Cell::new(0);
        assert!(
            split_grouped_bytes(b"abc (def)", b" ", Default::default(), &|| {
                calls.set(calls.get() + 1);
                anyhow::ensure!(calls.get() < 5, "cancelled");
                Ok(())
            })
            .is_err()
        );
        let calls = std::cell::Cell::new(0);
        assert!(split_grouped_bytes(b"abc", b" ", Default::default(), &|| {
            calls.set(calls.get() + 1);
            // Five checks complete preflight; this cancels during output fill.
            anyhow::ensure!(calls.get() < 7, "second pass cancelled");
            Ok(())
        })
        .is_err());
    }
}

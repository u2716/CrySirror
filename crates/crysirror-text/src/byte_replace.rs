// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Bounded, encoding-independent replacement of already identified byte spans.
use anyhow::{Context, Result, ensure};
use std::ops::Range;

/// Apply ordered, disjoint source ranges once. Replacement bytes are never
/// scanned again. Empty ranges permit insertion, including repeated insertions
/// at the same offset. All bounds and the final size are checked before output
/// allocation; `check` lets task owners cancel admission and copying.
pub fn replace_spans_bytes(
    source: &[u8],
    replacements: &[(Range<usize>, &[u8])],
    max_bytes: usize,
    check: &dyn Fn() -> Result<()>,
) -> Result<Vec<u8>> {
    check()?;
    ensure!(
        source.len() <= 16 * 1024 * 1024 && replacements.len() <= 1024 * 1024,
        "byte replacement input extent"
    );
    ensure!(
        max_bytes <= 16 * 1024 * 1024,
        "byte replacement output limit"
    );
    let (mut end, mut size) = (0, source.len());
    for (range, bytes) in replacements {
        check()?;
        ensure!(
            range.start >= end && range.start <= range.end && range.end <= source.len(),
            "byte replacement ranges must be ordered, disjoint and within the source"
        );
        size = size
            .checked_sub(range.len())
            .and_then(|n| n.checked_add(bytes.len()))
            .context("byte replacement size overflow")?;
        end = range.end;
    }
    ensure!(size <= max_bytes, "byte replacement output extent");
    let mut out = Vec::with_capacity(size);
    end = 0;
    for (range, bytes) in replacements {
        check()?;
        out.extend_from_slice(&source[end..range.start]);
        out.extend_from_slice(bytes);
        end = range.end;
    }
    check()?;
    out.extend_from_slice(&source[end..]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replacement_is_binary_single_pass_and_can_shrink_before_limit() {
        assert_eq!(
            replace_spans_bytes(
                b"a\x81bTOKENc",
                &[(1..2, b"\0"), (3..8, b"TOKEN")],
                9,
                &|| Ok(())
            )
            .unwrap(),
            b"a\0bTOKENc"
        );
        assert_eq!(
            replace_spans_bytes(b"long", &[(0..4, b"x")], 1, &|| Ok(())).unwrap(),
            b"x"
        );
        assert_eq!(
            replace_spans_bytes(b"ab", &[(1..1, b"X"), (1..1, b"Y")], 4, &|| Ok(())).unwrap(),
            b"aXYb"
        );
    }
    #[test]
    fn malformed_ranges_budget_and_cancellation_fail() {
        for ranges in [
            vec![(2..1, b"x".as_slice())],
            vec![(0..4, b"x")],
            vec![(1..2, b"x"), (0..1, b"x")],
            vec![(0..2, b"x"), (1..3, b"x")],
        ] {
            assert!(replace_spans_bytes(b"abc", &ranges, 10, &|| Ok(())).is_err());
        }
        assert!(replace_spans_bytes(b"abc", &[], 2, &|| Ok(())).is_err());
        let count = std::cell::Cell::new(0);
        assert!(
            replace_spans_bytes(b"a", &[(0..1, b"b")], 1, &|| {
                count.set(count.get() + 1);
                ensure!(count.get() < 3, "cancelled");
                Ok(())
            })
            .is_err()
        );
    }
}

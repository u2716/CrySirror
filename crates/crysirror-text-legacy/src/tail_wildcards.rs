// Copyright (C) 2026 CrySirror contributors. GPL-2.0-or-later.
//! Bytewise, symmetric wildcards with tail alignment rather than backtracking.
use super::*;
impl ByteStringContext<'_> {
    /// Match NUL prefixes bytewise. `?` on either side accepts one byte; `*`
    /// aligns the opposite cursor so both remaining tails have equal lengths,
    /// then consumes one byte on both sides. This is deliberately distinct from
    /// a glob: a star cannot consume an empty opposite tail. Alignment before
    /// either allocation (undefined in native pointer code) is an explicit
    /// error. Cancellation and total work are bounded by this context.
    pub fn matches_tail_wildcards(&self, left: &[u8], right: &[u8]) -> Result<bool> {
        let mut work = self.work()?;
        work.input(left)?;
        work.input(right)?;
        work.visit(left.len())?;
        work.visit(right.len())?;
        let left = &left[..left.iter().position(|b| *b == 0).unwrap_or(left.len())];
        let right = &right[..right.iter().position(|b| *b == 0).unwrap_or(right.len())];
        let (mut a, mut b) = (0usize, 0usize);
        while a < left.len() && b < right.len() {
            work.visit(1)?;
            if left[a] == b'*' {
                b = right
                    .len()
                    .checked_sub(left.len() - a)
                    .context("tail wildcard alignment precedes input")?;
            } else if right[b] == b'*' {
                a = left
                    .len()
                    .checked_sub(right.len() - b)
                    .context("tail wildcard alignment precedes input")?;
            } else if left[a] != b'?' && right[b] != b'?' && left[a] != right[b] {
                return Ok(false);
            }
            a += 1;
            b += 1;
        }
        Ok(a == left.len() && b == right.len())
    }
}

// Copyright (c) 2026 CrySirror contributors. GPL-2.0-or-later.
//! Fixed Windows CP932/LCID0411 CRT comparison. No host locale or Unicode
//! version is consulted. The source CRT uses lowercase for SBCS and uppercase
//! for DBCS, then compares unsigned encoded character values. Crucially it
//! stops at the first difference, without decoding an unrelated string tail.
use super::windows932_crt_tables::{CANONICAL_ALIASES, VALID_DBCS};
use super::*;
use std::cmp::Ordering;

impl ByteStringContext<'_> {
    /// Reviewed Windows CP932 multibyte CRT comparison, ending at NUL or the
    /// first difference. Fixed Japanese NLS tables canonicalize DBCS aliases;
    /// ASCII folds to lowercase, fullwidth Latin/Greek/Cyrillic/Roman to upper.
    /// A final lead byte is treated as NUL as in this CRT. Unknown DBCS units
    /// fail only if reached. Other byte/locale profiles are not substituted.
    pub fn compare_windows932_crt_case(&self, left: &[u8], right: &[u8]) -> Result<Ordering> {
        ensure!(
            self.profile == ByteStringProfile::SHIFT_JIS_CRT,
            "Windows932 CRT comparison requires the fixed CP932 profile"
        );
        let mut work = self.work()?;
        work.input(left)?;
        work.input(right)?;
        let mut a = 0;
        let mut b = 0;
        loop {
            let x = next_unit(left, &mut a, &mut work)?;
            let y = next_unit(right, &mut b, &mut work)?;
            if x != y {
                return Ok(x.cmp(&y));
            }
            if x == 0 {
                return Ok(Ordering::Equal);
            }
        }
    }
}
fn next_unit(bytes: &[u8], at: &mut usize, work: &mut Work<'_, '_>) -> Result<u16> {
    work.visit(1)?;
    let first = bytes.get(*at).copied().unwrap_or(0);
    if first == 0 {
        return Ok(0);
    }
    *at += 1;
    if !ByteStringProfile::SHIFT_JIS_CRT.lead.is_lead(first) {
        return Ok(u16::from(first.to_ascii_lowercase()));
    }
    let second = bytes.get(*at).copied().unwrap_or(0);
    if second == 0 {
        return Ok(0);
    }
    *at += 1;
    work.visit(1)?;
    let unit = u16::from_be_bytes([first, second]);
    let index = VALID_DBCS.partition_point(|&(_, end)| end < unit);
    ensure!(
        VALID_DBCS
            .get(index)
            .is_some_and(|&(start, end)| start <= unit && unit <= end),
        "Windows932 CRT unknown NLS character {unit:#06x}"
    );
    let canonical = CANONICAL_ALIASES
        .binary_search_by_key(&unit, |&(original, _)| original)
        .map(|i| CANONICAL_ALIASES[i].1)
        .unwrap_or(unit);
    // CP932's complete cased DBCS families. The mappings are fixed encoded
    // ranges, not Rust Unicode casefolding. Roman aliases normalize above.
    Ok(match canonical {
        0x8281..=0x829a => canonical - 0x21,
        0x83bf..=0x83d6 => canonical - 0x20,
        0x8470..=0x847e => canonical - 0x30,
        0x8480..=0x8491 => canonical - 0x31,
        0xfa40..=0xfa49 => 0x8754 + canonical - 0xfa40,
        _ => canonical,
    })
}

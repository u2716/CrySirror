// Copyright (c) 2026 CrySirror contributors. GPL-2.0-or-later.
//! ASCII casing at encoded character boundaries; DBCS trails are never folded.
use super::*;
impl ByteStringContext<'_> {
    pub fn map_ascii_case(&self, bytes: &[u8], upper: bool) -> Result<Vec<u8>> {
        let mut work = self.work()?;
        work.input(bytes)?;
        let bytes = &bytes[..bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len())];
        work.output(bytes.len())?;
        let mut output = bytes.to_vec();
        let mut at = 0;
        while at < output.len() {
            work.visit(1)?;
            if self.profile.lead.is_lead(output[at]) {
                ensure!(at + 1 < output.len(), "truncated byte-case character");
                at += 2;
            } else {
                output[at] = if upper {
                    output[at].to_ascii_uppercase()
                } else {
                    output[at].to_ascii_lowercase()
                };
                at += 1;
            }
        }
        Ok(output)
    }
    /// Compare unsigned encoded character values, with ASCII case folded.
    /// A double-byte unit sorts above any single-byte unit, independently of
    /// lexicographic byte order or the decoded Unicode scalar's code point.
    pub fn compare_ascii_case(&self, left: &[u8], right: &[u8]) -> Result<std::cmp::Ordering> {
        let left = self.map_ascii_case(left, true)?;
        let right = self.map_ascii_case(right, true)?;
        let mut a = 0;
        let mut b = 0;
        let unit = |bytes: &[u8], at: &mut usize| -> u16 {
            let Some(first) = bytes.get(*at).copied() else {
                return 0;
            };
            *at += 1;
            if self.profile.lead.is_lead(first) {
                let second = bytes[*at];
                *at += 1;
                u16::from_be_bytes([first, second])
            } else {
                u16::from(first)
            }
        };
        loop {
            (self.check)()?;
            let x = unit(&left, &mut a);
            let y = unit(&right, &mut b);
            if x != y {
                return Ok(x.cmp(&y));
            }
            if x == 0 {
                return Ok(std::cmp::Ordering::Equal);
            }
        }
    }
}

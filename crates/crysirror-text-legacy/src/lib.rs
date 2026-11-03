// Copyright (C) 2019 Nunuhara Cabbage <nunuhara@haniwa.technology>
// Copyright (C) 1997 Yutaka OIWA <oiwa@is.s.u-tokyo.ac.jp>
// Copyright (C) 2026 CrySirror contributors.
// GPL-2.0-or-later. See SOURCE.json and LICENSE.
//! Shared bounded byte-string operations for legacy character profiles.
//!
//! Stored bytes and the prefix ending at the first NUL are deliberately
//! distinct. These operations do not decode Unicode or validate a code page.
//! VM heap ownership, native stack values and game-specific exceptions belong
//! to their adapters. No file, device, font or renderer is used here.
#![forbid(unsafe_code)]

use anyhow::{ensure, Context, Result};

pub mod numeric;
pub use numeric::{ByteNumberSymbols, NumericCompaction};

pub const MAX_BYTE_STRING: usize = 4_000_000;
pub const MAX_BYTE_WORK: usize = 64_000_000;

/// Test only the first byte. A native lead-byte mask can intentionally admit
/// bytes that a standard code-page decoder would reject.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ByteLeadProfile {
    SingleByte,
    MaskedBands { mask: u8, first: u8, second: u8 },
    InclusiveRange { first: u8, last: u8 },
}
impl ByteLeadProfile {
    pub fn is_lead(self, byte: u8) -> bool {
        match self {
            Self::SingleByte => false,
            Self::MaskedBands {
                mask,
                first,
                second,
            } => {
                let band = byte & mask;
                band == first || band == second
            }
            Self::InclusiveRange { first, last } => (first..=last).contains(&byte),
        }
    }
    fn validate(self) -> Result<()> {
        match self {
            Self::MaskedBands {
                mask,
                first,
                second,
            } => ensure!(
                mask != 0 && first & !mask == 0 && second & !mask == 0,
                "invalid native byte lead mask"
            ),
            Self::InclusiveRange { first, last } => {
                ensure!(
                    first != 0 && first <= last,
                    "invalid native byte lead range"
                )
            }
            Self::SingleByte => {}
        }
        ensure!(!self.is_lead(0), "native NUL cannot be a lead byte");
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ByteStringProfile {
    pub lead: ByteLeadProfile,
    pub signed_single_byte: bool,
}
impl ByteStringProfile {
    /// Historical bit-mask profile: 80..9f and e0..ff are lead bytes. This is
    /// intentionally broader than validated Shift JIS/Windows-31J.
    pub const SHIFT_JIS_MASK: Self = Self {
        lead: ByteLeadProfile::MaskedBands {
            mask: 0xe0,
            first: 0x80,
            second: 0xe0,
        },
        signed_single_byte: true,
    };
    pub const GBK_RANGE: Self = Self {
        lead: ByteLeadProfile::InclusiveRange {
            first: 0x81,
            last: 0xfe,
        },
        signed_single_byte: true,
    };
    pub const UNSIGNED_BYTES: Self = Self {
        lead: ByteLeadProfile::SingleByte,
        signed_single_byte: false,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ByteStringLimits {
    pub input_bytes: usize,
    pub output_bytes: usize,
    /// Bounds byte comparisons as well as scans, including quadratic searches.
    pub work_bytes: usize,
}
impl Default for ByteStringLimits {
    fn default() -> Self {
        Self {
            input_bytes: MAX_BYTE_STRING,
            output_bytes: MAX_BYTE_STRING,
            work_bytes: MAX_BYTE_WORK,
        }
    }
}

/// Immutable worker-safe operation context. Cancellation and all configured
/// limits are observed before publishing a new string. Each operation has an
/// independent work counter; no hidden RNG, clock or process state is retained.
pub struct ByteStringContext<'a> {
    pub profile: ByteStringProfile,
    limits: ByteStringLimits,
    check: &'a (dyn Fn() -> Result<()> + Sync),
}
struct Work<'a, 'b> {
    context: &'a ByteStringContext<'b>,
    remaining: usize,
}
impl Work<'_, '_> {
    fn visit(&mut self, amount: usize) -> Result<()> {
        self.remaining = self
            .remaining
            .checked_sub(amount)
            .context("native byte-string work budget")?;
        (self.context.check)()
    }
    fn input(&mut self, bytes: &[u8]) -> Result<()> {
        ensure!(
            bytes.len() <= self.context.limits.input_bytes,
            "native byte-string input budget"
        );
        self.visit(0)
    }
    fn output(&mut self, count: usize) -> Result<()> {
        ensure!(
            count <= self.context.limits.output_bytes,
            "native byte-string output budget"
        );
        self.visit(count)
    }
    /// Original `sjis_index`: index zero can point at a final lead byte; a
    /// missing/zero second byte while *traversing* a character means no index.
    fn index(&mut self, bytes: &[u8], index: usize) -> Result<Option<usize>> {
        let mut at = 0;
        for _ in 0..index {
            self.visit(1)?;
            let Some(&byte) = bytes.get(at).filter(|&&byte| byte != 0) else {
                return Ok(None);
            };
            if self.context.profile.lead.is_lead(byte) {
                at += 1;
                if bytes.get(at).copied().unwrap_or(0) == 0 {
                    return Ok(None);
                }
            }
            at += 1;
        }
        Ok(bytes.get(at).filter(|&&byte| byte != 0).map(|_| at))
    }
    /// Bounded C strncmp over byte strings with an implicit final NUL.
    fn equal_prefix(&mut self, left: &[u8], right: &[u8], count: usize) -> Result<bool> {
        for at in 0..count {
            self.visit(1)?;
            let a = left.get(at).copied().unwrap_or(0);
            let b = right.get(at).copied().unwrap_or(0);
            if a != b {
                return Ok(false);
            }
            if a == 0 {
                return Ok(true);
            }
        }
        Ok(true)
    }
}
impl<'a> ByteStringContext<'a> {
    pub fn new(
        profile: ByteStringProfile,
        limits: ByteStringLimits,
        check: &'a (dyn Fn() -> Result<()> + Sync),
    ) -> Result<Self> {
        profile.lead.validate()?;
        ensure!(
            limits.input_bytes <= MAX_BYTE_STRING
                && limits.output_bytes <= MAX_BYTE_STRING
                && limits.work_bytes <= MAX_BYTE_WORK,
            "native byte-string configured budget"
        );
        check()?;
        Ok(Self {
            profile,
            limits,
            check,
        })
    }
    fn work(&self) -> Result<Work<'_, 'a>> {
        (self.check)()?;
        Ok(Work {
            context: self,
            remaining: self.limits.work_bytes,
        })
    }
    /// Character index in the NUL prefix, expressed as a stored-byte offset.
    pub fn byte_index(&self, bytes: &[u8], index: usize) -> Result<Option<usize>> {
        let mut work = self.work()?;
        work.input(bytes)?;
        work.index(bytes, index)
    }
    pub fn character_count(&self, bytes: &[u8]) -> Result<usize> {
        let mut work = self.work()?;
        work.input(bytes)?;
        let (mut at, mut count) = (0, 0);
        while let Some(&byte) = bytes.get(at).filter(|&&byte| byte != 0) {
            work.visit(1)?;
            if self.profile.lead.is_lead(byte) {
                // The unbounded C routine would continue beyond its terminator
                // on this malformed input. Never reproduce that memory read.
                ensure!(
                    bytes.get(at + 1).is_some_and(|&byte| byte != 0),
                    "truncated native byte character count"
                );
                at += 1;
            }
            at += 1;
            count += 1;
        }
        Ok(count)
    }
    /// Packed little-endian two-byte character or native signed/unsigned char.
    /// The index bound intentionally uses the stored BYTE count, not characters.
    pub fn get_character(&self, bytes: &[u8], index: i32) -> Result<i32> {
        let mut work = self.work()?;
        work.input(bytes)?;
        ensure!(
            index >= 0 && index as usize <= bytes.len(),
            "native byte character index out of bounds"
        );
        let Some(at) = work.index(bytes, index as usize)? else {
            return Ok(0);
        };
        let byte = bytes[at];
        Ok(if self.profile.lead.is_lead(byte) {
            i32::from(byte) | (i32::from(bytes.get(at + 1).copied().unwrap_or(0)) << 8)
        } else if self.profile.signed_single_byte {
            i32::from(byte as i8)
        } else {
            i32::from(byte)
        })
    }
    pub fn set_character(&self, bytes: &[u8], index: i32, character: u32) -> Result<Vec<u8>> {
        let mut work = self.work()?;
        work.input(bytes)?;
        ensure!(
            index >= 0 && (index as usize) < bytes.len(),
            "native byte character index out of bounds"
        );
        let Some(at) = work.index(bytes, index as usize)? else {
            work.output(bytes.len())?;
            return Ok(bytes.to_vec());
        };
        if character == 0 {
            work.output(at)?;
            return Ok(bytes[..at].to_vec());
        }
        let old_len = if self.profile.lead.is_lead(bytes[at]) {
            2
        } else {
            1
        };
        ensure!(
            at + old_len <= bytes.len(),
            "truncated native byte character replacement"
        );
        let new_len = if self.profile.lead.is_lead(character as u8) {
            2
        } else {
            1
        };
        let output_len = bytes.len() - old_len + new_len;
        work.output(output_len)?;
        let mut output = Vec::with_capacity(output_len);
        output.extend_from_slice(&bytes[..at]);
        output.push(character as u8);
        if new_len == 2 {
            output.push((character >> 8) as u8)
        }
        output.extend_from_slice(&bytes[at + old_len..]);
        (self.check)()?;
        Ok(output)
    }
    /// Clamp negative indices to zero. When requested end is past the NUL
    /// prefix, copy through the original STORED extent, including NUL tails.
    pub fn substring(&self, bytes: &[u8], index: i32, count: i32) -> Result<Vec<u8>> {
        let mut work = self.work()?;
        work.input(bytes)?;
        if count <= 0 {
            return Ok(Vec::new());
        }
        let Some(start) = work.index(bytes, index.max(0) as usize)? else {
            return Ok(Vec::new());
        };
        let length = work
            .index(&bytes[start..], count as usize)?
            .unwrap_or(bytes.len() - start);
        work.output(length)?;
        Ok(bytes[start..start + length].to_vec())
    }
    /// Search at native character boundaries across the stored extent. An
    /// empty needle returns zero only when the haystack has stored bytes.
    pub fn find(&self, haystack: &[u8], needle: &[u8]) -> Result<i32> {
        let mut work = self.work()?;
        work.input(haystack)?;
        work.input(needle)?;
        let (mut at, mut character) = (0, 0i32);
        while at < haystack.len() {
            work.visit(1)?;
            if work.equal_prefix(&haystack[at..], needle, needle.len())? {
                return Ok(character);
            }
            at += if self.profile.lead.is_lead(haystack[at]) {
                2
            } else {
                1
            };
            character += 1;
        }
        Ok(-1)
    }
    pub fn push_character(&self, bytes: &[u8], character: u32) -> Result<Vec<u8>> {
        let mut work = self.work()?;
        work.input(bytes)?;
        let count = if self.profile.lead.is_lead(character as u8) {
            2
        } else {
            1
        };
        let length = bytes
            .len()
            .checked_add(count)
            .context("native byte-string length overflow")?;
        work.output(length)?;
        let mut output = Vec::with_capacity(length);
        output.extend_from_slice(bytes);
        output.push(character as u8);
        if count == 2 {
            output.push((character >> 8) as u8)
        }
        Ok(output)
    }
    /// Last character is found over the stored extent, including NUL tails.
    pub fn pop_character(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        let mut work = self.work()?;
        work.input(bytes)?;
        let (mut at, mut last) = (0, 0);
        while at < bytes.len() {
            work.visit(1)?;
            last = at;
            at += if self.profile.lead.is_lead(bytes[at]) {
                2
            } else {
                1
            };
        }
        work.output(last)?;
        Ok(bytes[..last].to_vec())
    }
    pub fn erase_character(&self, bytes: &[u8], index: i32) -> Result<Vec<u8>> {
        let mut work = self.work()?;
        work.input(bytes)?;
        let index = index.max(0) as usize;
        let at = if index < bytes.len() {
            work.index(bytes, index)?
        } else {
            None
        };
        let Some(at) = at else {
            work.output(bytes.len())?;
            return Ok(bytes.to_vec());
        };
        let count = if self.profile.lead.is_lead(bytes[at]) {
            2
        } else {
            1
        };
        ensure!(
            at + count <= bytes.len(),
            "truncated native byte character erase"
        );
        work.output(bytes.len() - count)?;
        let mut output = Vec::with_capacity(bytes.len() - count);
        output.extend_from_slice(&bytes[..at]);
        output.extend_from_slice(&bytes[at + count..]);
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn context() -> ByteStringContext<'static> {
        ByteStringContext::new(
            ByteStringProfile::SHIFT_JIS_MASK,
            ByteStringLimits::default(),
            &|| Ok(()),
        )
        .unwrap()
    }
    #[test]
    fn original_mask_signed_character_and_nul_indices_are_explicit() {
        let c = context();
        assert!(c.profile.lead.is_lead(0x80));
        assert!(c.profile.lead.is_lead(0xff));
        assert!(!c.profile.lead.is_lead(0xa5));
        let bytes = [0x80, b'A', 0xa5, b'x', 0, b'y'];
        assert_eq!(c.character_count(&bytes).unwrap(), 3);
        assert_eq!(c.get_character(&bytes, 0).unwrap(), 0x4180);
        assert_eq!(c.get_character(&bytes, 1).unwrap(), -91);
        assert_eq!(c.get_character(&bytes, 3).unwrap(), 0);
        assert_eq!(c.byte_index(&bytes, 2).unwrap(), Some(3));
        assert_eq!(c.byte_index(&bytes, 3).unwrap(), None);
        assert!(c.get_character(&bytes, 7).is_err());
        assert_eq!(c.get_character(&[0x81], 0).unwrap(), 0x81);
        assert_eq!(c.byte_index(&[0x81], 1).unwrap(), None);
        assert!(c.character_count(&[0x81]).is_err());
    }
    #[test]
    fn edits_use_stored_extent_and_character_boundaries() {
        let c = context();
        assert_eq!(c.substring(b"a\0tail", 0, 7).unwrap(), b"a\0tail");
        assert_eq!(c.substring(b"a\0tail", 0, 1).unwrap(), b"a\0tail");
        assert!(c.substring(b"abc", 1, 0).unwrap().is_empty());
        assert_eq!(c.substring(b"abc", -9, 1).unwrap(), b"a");
        assert_eq!(c.pop_character(b"a\0z").unwrap(), b"a\0");
        assert_eq!(c.pop_character(&[b'a', 0x81]).unwrap(), b"a");
        assert_eq!(c.push_character(b"a", 0x4281).unwrap(), [b'a', 0x81, b'B']);
        assert_eq!(
            c.set_character(b"abc", 1, 0x4281).unwrap(),
            [b'a', 0x81, b'B', b'c']
        );
        assert_eq!(
            c.set_character(&[b'a', 0x81, b'B', b'c'], 1, b'x' as u32)
                .unwrap(),
            b"axc"
        );
        assert_eq!(c.set_character(b"abc", 1, 0).unwrap(), b"a");
        assert_eq!(
            c.erase_character(&[b'a', 0x81, b'B', b'c'], 1).unwrap(),
            b"ac"
        );
        assert_eq!(c.erase_character(b"a\0tail", 2).unwrap(), b"a\0tail");
        assert!(c.erase_character(&[0x81], 0).is_err());
    }
    #[test]
    fn find_has_c_comparison_and_stored_extent_semantics() {
        let c = context();
        assert_eq!(c.find(b"", b"").unwrap(), -1);
        assert_eq!(c.find(b"abc", b"").unwrap(), 0);
        assert_eq!(c.find(b"a\0z", b"z").unwrap(), 2);
        assert_eq!(c.find(&[0x81, b'x', b'x'], b"x").unwrap(), 1);
        assert_eq!(c.find(b"a\0left", b"a\0right").unwrap(), 0);
    }
    #[test]
    fn different_profiles_limits_and_cancel_are_independent_of_vm() {
        let raw = ByteStringContext::new(
            ByteStringProfile::UNSIGNED_BYTES,
            ByteStringLimits::default(),
            &|| Ok(()),
        )
        .unwrap();
        assert_eq!(raw.get_character(&[0xa5], 0).unwrap(), 165);
        assert_eq!(raw.character_count(&[0x81]).unwrap(), 1);
        let budget = ByteStringContext::new(
            ByteStringProfile::UNSIGNED_BYTES,
            ByteStringLimits {
                input_bytes: 8,
                output_bytes: 2,
                work_bytes: 4,
            },
            &|| Ok(()),
        )
        .unwrap();
        assert!(budget.push_character(b"ab", 42).is_err());
        assert!(budget.find(b"aaaaa", b"aaaz").is_err());
        assert!(ByteStringContext::new(
            ByteStringProfile::UNSIGNED_BYTES,
            ByteStringLimits::default(),
            &|| anyhow::bail!("cancelled")
        )
        .is_err());
    }
}

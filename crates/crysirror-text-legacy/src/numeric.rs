// Copyright (C) 2019 Nunuhara Cabbage <nunuhara@haniwa.technology>
// Copyright (C) 2026 CrySirror contributors. GPL-2.0-or-later.
//! Numeric byte-string profiles, kept out of Unicode/font layout and VM state.
use super::{ByteStringContext, Result};
use anyhow::{ensure, Context};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ByteNumberSymbols {
    Ascii,
    ShiftJis,
    Gbk,
}
impl ByteNumberSymbols {
    fn digit(self) -> Option<(u8, u8)> {
        match self {
            Self::Ascii => None,
            Self::ShiftJis => Some((0x82, 0x4f)),
            Self::Gbk => Some((0xa3, 0xb0)),
        }
    }
    fn minus(self) -> Option<[u8; 2]> {
        match self {
            Self::Ascii => None,
            Self::ShiftJis => Some([0x81, 0x7c]),
            Self::Gbk => Some([0xa3, 0xad]),
        }
    }
    fn point(self) -> Option<[u8; 2]> {
        match self {
            Self::Ascii => None,
            Self::ShiftJis => Some([0x81, 0x44]),
            Self::Gbk => Some([0xa3, 0xae]),
        }
    }
    fn space(self) -> Option<[u8; 2]> {
        match self {
            Self::Ascii => None,
            Self::ShiftJis => Some([0x81, 0x40]),
            Self::Gbk => Some([0xa1, 0xa1]),
        }
    }
}

/// Whether the historical in-place narrowing writes a new NUL at the compacted
/// end. Retaining the original buffer tail is an observable source behavior,
/// e.g. fullwidth ONE followed by ASCII TWO becomes ASCII "122" in that profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericCompaction {
    Terminate,
    PreserveStoredTail,
}

impl ByteStringContext<'_> {
    pub fn decimal_integer(
        &self,
        bytes: &[u8],
        symbols: ByteNumberSymbols,
        compaction: NumericCompaction,
    ) -> Result<i32> {
        let mut work = self.work()?;
        work.input(bytes)?;
        // strdup copies the NUL prefix, not a string object's later stored tail.
        work.visit(bytes.len())?;
        let length = bytes
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(bytes.len());
        work.output(length)?;
        let mut buffer = Vec::with_capacity(length + 1);
        buffer.extend_from_slice(&bytes[..length]);
        buffer.push(0);
        let (mut source, mut destination) = (0usize, 0usize);
        while source < length {
            work.visit(1)?;
            let first = buffer[source];
            if symbols != ByteNumberSymbols::Ascii && self.profile.lead.is_lead(first) {
                ensure!(
                    source + 1 < length,
                    "truncated numeric native byte character"
                );
                let second = buffer[source + 1];
                let pair = [first, second];
                if let Some((lead, start)) = symbols
                    .digit()
                    .filter(|&(lead, start)| first == lead && (start..start + 10).contains(&second))
                {
                    let _ = lead;
                    buffer[destination] = b'0' + second - start;
                    destination += 1;
                } else if symbols.minus() == Some(pair) {
                    buffer[destination] = b'-';
                    destination += 1;
                } else if symbols.point() == Some(pair) {
                    buffer[destination] = b'.';
                    destination += 1;
                } else if symbols.space() == Some(pair) {
                    buffer[destination] = b' ';
                    destination += 1;
                } else {
                    buffer[destination] = first;
                    buffer[destination + 1] = second;
                    destination += 2;
                }
                source += 2;
            } else {
                buffer[destination] = first;
                destination += 1;
                source += 1;
            }
        }
        if compaction == NumericCompaction::Terminate {
            buffer[destination] = 0
        }
        let mut at = 0usize;
        while matches!(
            buffer.get(at),
            Some(b' ' | b'\t' | b'\r' | b'\n' | 0x0b | 0x0c)
        ) {
            work.visit(1)?;
            at += 1;
        }
        let negative = buffer.get(at) == Some(&b'-');
        if matches!(buffer.get(at), Some(b'-' | b'+')) {
            at += 1
        }
        let mut magnitude = 0i64;
        while let Some(&byte @ b'0'..=b'9') = buffer.get(at) {
            work.visit(1)?;
            magnitude = magnitude
                .checked_mul(10)
                .and_then(|v| v.checked_add(i64::from(byte - b'0')))
                .context("native decimal integer overflow")?;
            ensure!(
                magnitude <= i64::from(i32::MAX) + i64::from(negative),
                "native decimal integer outside defined i32 range"
            );
            at += 1;
        }
        (self.check)()?;
        Ok(if negative { -magnitude } else { magnitude } as i32)
    }

    /// Native `%.*f` over a promoted f32. Negative precision means six; undefined
    /// C fixed-buffer overflow is rejected before publication. Non-finite values
    /// retain C's lowercase spellings and their sign bit.
    pub fn fixed_float(&self, value: f32, precision: i32) -> Result<Vec<u8>> {
        let mut work = self.work()?;
        let precision = if precision < 0 { 6 } else { precision as usize };
        let output = if value.is_nan() {
            if value.is_sign_negative() {
                "-nan".to_owned()
            } else {
                "nan".to_owned()
            }
        } else if value.is_infinite() {
            if value.is_sign_negative() {
                "-inf".to_owned()
            } else {
                "inf".to_owned()
            }
        } else {
            ensure!(
                precision <= 510,
                "native fixed float precision exceeds original buffer"
            );
            format!("{:.*}", precision, f64::from(value))
        };
        ensure!(
            output.len() < 512,
            "native fixed float exceeds original buffer"
        );
        work.output(output.len())?;
        Ok(output.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ByteStringLimits, ByteStringProfile};
    fn context() -> ByteStringContext<'static> {
        ByteStringContext::new(
            ByteStringProfile::SHIFT_JIS_MASK,
            ByteStringLimits::default(),
            &|| Ok(()),
        )
        .unwrap()
    }
    #[test]
    fn compaction_tail_and_c_integer_prefix_are_explicit_profiles() {
        let c = context();
        let one_two = [0x82, 0x50, b'2'];
        assert_eq!(
            c.decimal_integer(
                &one_two,
                ByteNumberSymbols::ShiftJis,
                NumericCompaction::PreserveStoredTail
            )
            .unwrap(),
            122
        );
        assert_eq!(
            c.decimal_integer(
                &one_two,
                ByteNumberSymbols::ShiftJis,
                NumericCompaction::Terminate
            )
            .unwrap(),
            12
        );
        assert_eq!(
            c.decimal_integer(
                b" \t-2147483648rest",
                ByteNumberSymbols::Ascii,
                NumericCompaction::Terminate
            )
            .unwrap(),
            i32::MIN
        );
        assert_eq!(
            c.decimal_integer(
                b"+27.9",
                ByteNumberSymbols::Ascii,
                NumericCompaction::Terminate
            )
            .unwrap(),
            27
        );
        assert_eq!(
            c.decimal_integer(
                b"no number",
                ByteNumberSymbols::Ascii,
                NumericCompaction::Terminate
            )
            .unwrap(),
            0
        );
        assert_eq!(
            c.decimal_integer(
                b"3\0ignored9",
                ByteNumberSymbols::Ascii,
                NumericCompaction::Terminate
            )
            .unwrap(),
            3
        );
        assert!(c
            .decimal_integer(
                b"2147483648",
                ByteNumberSymbols::Ascii,
                NumericCompaction::Terminate
            )
            .is_err());
        assert!(c
            .decimal_integer(
                &[0x81],
                ByteNumberSymbols::ShiftJis,
                NumericCompaction::PreserveStoredTail
            )
            .is_err());
    }
    #[test]
    fn fixed_float_preserves_promoted_f32_and_defined_buffer_boundary() {
        let c = context();
        assert_eq!(c.fixed_float(1.25, -1).unwrap(), b"1.250000");
        assert_eq!(c.fixed_float(1.25, 1).unwrap(), b"1.2");
        assert_eq!(c.fixed_float(-0.0, 3).unwrap(), b"-0.000");
        assert_eq!(c.fixed_float(f32::INFINITY, 6).unwrap(), b"inf");
        assert_eq!(c.fixed_float(f32::NEG_INFINITY, 6).unwrap(), b"-inf");
        assert_eq!(c.fixed_float(f32::NAN, 6).unwrap(), b"nan");
        assert!(c.fixed_float(1.25, 511).is_err());
    }
}

// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Safe byte-oriented C32 integer formatting. No host pointer dereferencing,
//! floating varargs, `%n`, locale dependency, or native code is involved.
use anyhow::{Context, Result, bail, ensure};

/// Values are explicit: a `%s` requires borrowed bytes, never an integer cast
/// into a pointer. Width, precision, `%c` and integer conversions require words.
#[derive(Clone, Copy, Debug)]
pub enum Printf32Argument<'a> {
    Word(u32),
    Bytes(&'a [u8]),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Printf32Output {
    pub bytes: Vec<u8>,
    pub arguments_used: usize,
}
#[derive(Default)]
struct Flags {
    left: bool,
    plus: bool,
    space: bool,
    alternate: bool,
    zero: bool,
}
fn word(args: &[Printf32Argument<'_>], index: &mut usize) -> Result<u32> {
    let arg = args.get(*index).context("printf32 missing word argument")?;
    let Printf32Argument::Word(value) = arg else {
        bail!("printf32 expected word argument")
    };
    *index += 1;
    Ok(*value)
}
fn number(source: &[u8], at: &mut usize, limit: usize) -> Result<usize> {
    let mut n = 0usize;
    while let Some(b @ b'0'..=b'9') = source.get(*at) {
        n = n
            .checked_mul(10)
            .and_then(|n| n.checked_add((b - b'0') as usize))
            .context("printf32 field overflow")?;
        ensure!(n <= limit, "printf32 field exceeds output budget");
        *at += 1;
    }
    Ok(n)
}
fn append(out: &mut Vec<u8>, bytes: &[u8], limit: usize) -> Result<()> {
    ensure!(
        out.len()
            .checked_add(bytes.len())
            .is_some_and(|n| n <= limit),
        "printf32 output extent"
    );
    out.extend_from_slice(bytes);
    Ok(())
}
fn padding(out: &mut Vec<u8>, byte: u8, n: usize, limit: usize) -> Result<()> {
    ensure!(
        out.len().checked_add(n).is_some_and(|n| n <= limit),
        "printf32 output extent"
    );
    out.resize(out.len() + n, byte);
    Ok(())
}
/// Format NUL-terminated byte strings using the C32 word model. Supported
/// conversions are `d i u o x X c s %%`, optional `hh h l I32`, flags, widths,
/// precision and `*`. Plain template/input bytes retain their encoding. Unused
/// arguments are ignored. Used strings and templates are bounded to16MiB.
/// Undefined C profiles fail instead of reading memory.
pub fn format_printf32_bytes(
    template: &[u8],
    args: &[Printf32Argument<'_>],
    max_bytes: usize,
) -> Result<Printf32Output> {
    ensure!(
        max_bytes <= 16 * 1024 * 1024 && template.len() <= 16 * 1024 * 1024 && args.len() <= 65536,
        "printf32 input/output extent"
    );
    let end = template
        .iter()
        .position(|b| *b == 0)
        .unwrap_or(template.len());
    let source = &template[..end];
    let (mut at, mut used, mut out) = (0, 0, Vec::new());
    while at < source.len() {
        if source[at] != b'%' {
            append(&mut out, &source[at..at + 1], max_bytes)?;
            at += 1;
            continue;
        }
        at += 1;
        if source.get(at) == Some(&b'%') {
            append(&mut out, b"%", max_bytes)?;
            at += 1;
            continue;
        }
        let mut flags = Flags::default();
        loop {
            match source.get(at) {
                Some(b'-') => flags.left = true,
                Some(b'+') => flags.plus = true,
                Some(b' ') => flags.space = true,
                Some(b'#') => flags.alternate = true,
                Some(b'0') => flags.zero = true,
                _ => break,
            };
            at += 1;
        }
        let width = if source.get(at) == Some(&b'*') {
            at += 1;
            let value = word(args, &mut used)? as i32;
            if value < 0 {
                flags.left = true;
            }
            let n = value.unsigned_abs() as usize;
            ensure!(n <= max_bytes, "printf32 field exceeds output budget");
            n
        } else {
            number(source, &mut at, max_bytes)?
        };
        let precision = if source.get(at) == Some(&b'.') {
            at += 1;
            if source.get(at) == Some(&b'*') {
                at += 1;
                let value = word(args, &mut used)? as i32;
                if value < 0 {
                    None
                } else {
                    let n = value as usize;
                    ensure!(n <= max_bytes, "printf32 precision exceeds output budget");
                    Some(n)
                }
            } else {
                Some(number(source, &mut at, max_bytes)?)
            }
        } else {
            None
        };
        let length_start = at;
        let bits = if source.get(at..at + 2) == Some(b"hh") {
            at += 2;
            8
        } else if source.get(at) == Some(&b'h') {
            at += 1;
            16
        } else if source.get(at..at + 3) == Some(b"I32") {
            at += 3;
            32
        } else if source.get(at) == Some(&b'l') && source.get(at + 1) != Some(&b'l') {
            at += 1;
            32
        } else {
            32
        };
        let conversion = *source.get(at).context("printf32 incomplete conversion")?;
        at += 1;
        let (prefix, digits, zeroes) = match conversion {
            b'd' | b'i' | b'u' | b'o' | b'x' | b'X' => {
                let raw = word(args, &mut used)?;
                let raw = match bits {
                    8 => raw as u8 as u32,
                    16 => raw as u16 as u32,
                    _ => raw,
                };
                let signed = matches!(conversion, b'd' | b'i');
                let value = match bits {
                    8 => raw as i8 as i32,
                    16 => raw as i16 as i32,
                    _ => raw as i32,
                };
                let magnitude = if signed { value.unsigned_abs() } else { raw };
                let mut digits = match conversion {
                    b'o' => format!("{magnitude:o}"),
                    b'x' => format!("{magnitude:x}"),
                    b'X' => format!("{magnitude:X}"),
                    _ => magnitude.to_string(),
                }
                .into_bytes();
                if precision == Some(0) && magnitude == 0 {
                    digits.clear();
                }
                let mut prefix = if signed && value < 0 {
                    b"-".to_vec()
                } else if signed && flags.plus {
                    b"+".to_vec()
                } else if signed && flags.space {
                    b" ".to_vec()
                } else {
                    Vec::new()
                };
                if flags.alternate && matches!(conversion, b'x' | b'X') && magnitude != 0 {
                    prefix.extend_from_slice(if conversion == b'X' { b"0X" } else { b"0x" });
                }
                let mut zeroes = precision.unwrap_or(0).saturating_sub(digits.len());
                if flags.alternate
                    && conversion == b'o'
                    && zeroes == 0
                    && digits.first() != Some(&b'0')
                {
                    zeroes = 1;
                }
                (prefix, digits, zeroes)
            }
            b'c' => {
                ensure!(
                    at - 1 == length_start && precision.is_none(),
                    "printf32 unsupported character length/precision"
                );
                (Vec::new(), vec![word(args, &mut used)? as u8], 0)
            }
            b's' => {
                ensure!(at - 1 == length_start, "printf32 unsupported wide string");
                let arg = args
                    .get(used)
                    .context("printf32 missing byte-string argument")?;
                let Printf32Argument::Bytes(bytes) = arg else {
                    bail!("printf32 string argument must be explicit bytes")
                };
                used += 1;
                ensure!(
                    bytes.len() <= 16 * 1024 * 1024,
                    "printf32 byte-string input extent"
                );
                let len = bytes
                    .iter()
                    .position(|b| *b == 0)
                    .unwrap_or(bytes.len())
                    .min(precision.unwrap_or(usize::MAX));
                ensure!(len <= max_bytes, "printf32 string exceeds output budget");
                (Vec::new(), bytes[..len].to_vec(), 0)
            }
            _ => bail!("printf32 unsupported conversion or length 0x{conversion:02x}"),
        };
        let body = prefix
            .len()
            .checked_add(digits.len())
            .and_then(|n| n.checked_add(zeroes))
            .context("printf32 size overflow")?;
        let pad = width.saturating_sub(body);
        let integer = matches!(conversion, b'd' | b'i' | b'u' | b'o' | b'x' | b'X');
        let zero_pad = integer && flags.zero && !flags.left && precision.is_none();
        if !flags.left && !zero_pad {
            padding(&mut out, b' ', pad, max_bytes)?;
        }
        append(&mut out, &prefix, max_bytes)?;
        padding(
            &mut out,
            b'0',
            zeroes + if zero_pad { pad } else { 0 },
            max_bytes,
        )?;
        append(&mut out, &digits, max_bytes)?;
        if flags.left {
            padding(&mut out, b' ', pad, max_bytes)?;
        }
    }
    Ok(Printf32Output {
        bytes: out,
        arguments_used: used,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fmt(template: &[u8], words: &[u32]) -> Vec<u8> {
        format_printf32_bytes(
            template,
            &words
                .iter()
                .map(|v| Printf32Argument::Word(*v))
                .collect::<Vec<_>>(),
            256,
        )
        .unwrap()
        .bytes
    }
    #[test]
    fn c32_sign_width_precision_and_radix() {
        assert_eq!(
            fmt(
                b"%+08d|%#08x|%#.0o|%.0u|%hhd|%hu|%I32d|%ld",
                &[
                    (-12i32) as u32,
                    0x2a,
                    0,
                    0,
                    255,
                    65537,
                    0x80000000,
                    0xffffffff
                ]
            ),
            b"-0000012|0x00002a|0||-1|1|-2147483648|-1"
        );
        assert_eq!(
            fmt(
                b"[%*.*d][% .*i][%#o][%%]",
                &[(-7i32) as u32, 4, 12, (-1i32) as u32, 3, 8]
            ),
            b"[0012   ][ 3][010][%]"
        );
        assert_eq!(fmt(b"%#.0x/%#.3o/%-05d", &[0, 1, 4]), b"/001/4    ");
    }
    #[test]
    fn explicit_bytes_nul_and_argument_receipt() {
        let out = format_printf32_bytes(
            b"\x82\xa0[%5.2s]%c\0ignored",
            &[
                Printf32Argument::Bytes(b"\x82\xa2x\0tail"),
                Printf32Argument::Word(0),
                Printf32Argument::Word(7),
            ],
            32,
        )
        .unwrap();
        assert_eq!(out.bytes, b"\x82\xa0[   \x82\xa2]\0");
        assert_eq!(out.arguments_used, 2);
        assert_eq!(fmt(b"plain%%", &[]), b"plain%");
    }
    #[test]
    fn unsafe_unknown_or_unbounded_profiles_fail() {
        for template in [
            b"%s".as_slice(),
            b"%n",
            b"%f",
            b"%lld",
            b"%I64d",
            b"%p",
            b"%ls",
            b"%",
            b"%99999999999d",
            b"%1000d",
        ] {
            assert!(
                format_printf32_bytes(template, &[Printf32Argument::Word(1)], 32).is_err(),
                "{template:?}"
            );
        }
        assert!(format_printf32_bytes(b"%d", &[], 32).is_err());
        assert!(format_printf32_bytes(b"%d", &[Printf32Argument::Bytes(b"a")], 32).is_err());
        assert!(
            format_printf32_bytes(
                b"%*d",
                &[
                    Printf32Argument::Word(i32::MIN as u32),
                    Printf32Argument::Word(0)
                ],
                32
            )
            .is_err()
        );
        assert!(format_printf32_bytes(b"plain", &[], 4).is_err());
    }
}

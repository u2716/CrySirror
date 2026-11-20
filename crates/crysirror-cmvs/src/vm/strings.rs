// SPDX-License-Identifier: MPL-2.0

//! Bounded native CP932 byte-string syscalls.

use super::{abi::StringRef, evaluate::error, machine::Machine, DecodeError};

pub(super) fn command(vm: &mut Machine, opcode: u16) -> Result<u16, DecodeError> {
    let pc = vm.pc();
    let fail = |why| error(pc, opcode, why);
    let arg = |index| vm.stack().peek_u32(index).map_err(|e| fail(e.0));
    let string = |index| vm.resolve_string(arg(index)?).map_err(|e| fail(e.reason));
    let cvns = vm.profile() == super::VmProfile::Cvns20040515X86;
    let operation = if cvns {
        match opcode {
            0x20f0 => 0x20f1,
            0x20f1 => 0x20f0,
            0x20f8..=0x20fc => opcode + 1,
            0x20fd => 0x20f8,
            _ => opcode,
        }
    } else {
        opcode
    };
    let cells = vm.state().string_cells();
    let overlaps = |a, sa, la, b, sb, lb| overlaps(cells, a, sa, la, b, sb, lb);
    let (cleanup, result) = match operation {
        0x20f0 | 0x20f2 => {
            let target = arg(4)?;
            let source_ref = arg(8)?;
            let source = string(8)?;
            let output = if operation == 0x20f2 {
                let mut buffer = string(4)?.to_vec();
                buffer.extend_from_slice(source);
                buffer
            } else {
                source.to_vec()
            };
            if overlaps(target, 0, output.len() + 1, source_ref, 0, source.len() + 1) {
                return Err(fail("overlapping CMVS copy or append operands"));
            }
            vm.write_string_reference(opcode, target, &output)?;
            return Ok(8);
        }
        0x20f1 => {
            let equal = compare_folded(string(4)?, string(8)?, true).map_err(fail)?;
            (8, u32::from(if cvns { !equal } else { equal }))
        }
        0x20f3 => {
            let target = arg(4)?;
            let start = arg(12)? as i32;
            let end = start.wrapping_add(arg(16)? as i32);
            let bytes = string(8)?;
            let mut i = start;
            let mut count = 0;
            while if cvns { (i as u32) < end as u32 } else { i < end } {
                let offset =
                    usize::try_from(i).map_err(|_| fail("negative CMVS substring offset"))?;
                if offset > bytes.len() {
                    return Err(fail("CMVS substring offset beyond its source"));
                }
                let Some(&byte) = bytes.get(offset) else {
                    break;
                };
                let width = if lead(byte) { 2 } else { 1 };
                i = i.wrapping_add(width);
                if i as usize > bytes.len() {
                    return Err(fail("truncated CP932 pair in CMVS substring"));
                }
                count += 1;
            }
            if count != 0 {
                let output = bytes[start as usize..i as usize].to_vec();
                if overlaps(
                    target,
                    0,
                    output.len() + 1,
                    arg(8)?,
                    start as usize,
                    output.len() + 1,
                ) {
                    return Err(fail("overlapping CMVS substring source and destination"));
                }
                vm.write_string_reference(opcode, target, &output)?;
            }
            (16, count)
        }
        0x20f4 => {
            let target = arg(4)?;
            let (delimiter, source) = if cvns {
                (string(12)?, string(8)?)
            } else {
                (string(8)?, string(12)?)
            };
            let width = 1 + usize::from(delimiter.first().is_some_and(|&b| lead(b)));
            let offset = delimiter
                .get(..width)
                .and_then(|needle| source.windows(width).position(|window| window == needle));
            let output = source[..offset.unwrap_or(source.len())].to_vec();
            vm.write_string_reference(opcode, target, &output)?;
            (12, offset.unwrap_or(0) as u32)
        }
        0x20f5 => {
            let starts_with = compare_folded(string(8)?, string(4)?, false).map_err(fail)?;
            (8, u32::from(starts_with))
        }
        0x20f6 => (4, native_integer(string(4)?, !cvns) as u32),
        0x20f7 | 0x20f9 => {
            let source = string(4)?;
            let needle = byte_at(string(8)?, 0);
            let matches = |&byte: &u8| byte == needle;
            let offset = if operation == 0x20f7 {
                source.iter().position(matches)
            } else {
                source.iter().rposition(matches)
            };
            (8, offset.map_or(u32::MAX, |i| i as u32))
        }
        0x20f8 => (4, string(4)?.len() as u32),
        0x20fa | 0x20fc | 0x20fd => {
            let target = arg(4)?;
            let value = match operation {
                0x20fa => FormatArg::Integer(arg(8)?),
                0x20fc => FormatArg::Integer(arg(12)?),
                _ => FormatArg::String(string(12)?),
            };
            let pattern = if operation == 0x20fa { b"%d-" } else { string(8)? };
            let output = sprintf(pattern, value, cvns, |offset| {
                vm.string_storage_byte(opcode, target, offset)
                    .map_err(|e| e.reason)
            })
            .map_err(fail)?;
            if (operation != 0x20fa
                && overlaps(target, 0, output.len() + 1, arg(8)?, 0, pattern.len() + 1))
                || matches!(value, FormatArg::String(source)
                    if overlaps(target, 0, output.len() + 1, arg(12)?, 0, source.len() + 1))
            {
                return Err(fail("overlapping CMVS format operands"));
            }
            vm.write_string_reference(opcode, target, &output)?;
            return Ok(if operation == 0x20fa { 8 } else { 12 });
        }
        0x20fb => {
            let source = arg(4)?;
            let mut sum = 0i32;
            for (index, power) in [
                (0, 10_000_000i32),
                (1, 1_000_000),
                (2, 100_000),
                (3, 10_000),
                (5, 1_000),
                (6, 100),
                (8, 10),
                (9, 1),
            ] {
                let byte = vm.string_storage_byte(opcode, source, index)?;
                sum += i32::from(byte as i8) * power;
            }
            (4, sum as u32)
        }
        0x20ff => {
            let target = arg(4)?;
            let source = string(4)?;
            let extension = string(8)?;
            let mut output = source[..last_single_byte_dot(source)
                .map_err(fail)?
                .unwrap_or(source.len())]
                .to_vec();
            output.push(b'.');
            output.extend_from_slice(extension);
            if overlaps(target, 0, output.len() + 1, arg(8)?, 0, extension.len() + 1) {
                return Err(fail("overlapping CMVS extension operands"));
            }
            vm.write_string_reference(opcode, target, &output)?;
            return Ok(8);
        }
        _ => unreachable!(),
    };
    vm.state_mut().set_system_integer(0, result).map_err(fail)?;
    Ok(cleanup)
}

fn lead(byte: u8) -> bool {
    (0x81..=0x9f).contains(&byte) || (0xe0..=0xfc).contains(&byte)
}

fn byte_at(bytes: &[u8], index: usize) -> u8 {
    bytes.get(index).copied().unwrap_or(0)
}

fn overlaps(cells: u32, a: u32, start_a: usize, len_a: usize, b: u32, start_b: usize, len_b: usize) -> bool {
    let (base_a, base_b) = match (StringRef::decode(a), StringRef::decode(b)) {
        (StringRef::StringBank(a), StringRef::StringBank(b)) => {
            let slot = |index| if index < cells { index } else { 0 };
            return slot(a) == slot(b) && start_a < start_b + len_b && start_b < start_a + len_a;
        }
        (StringRef::ScriptData(a), StringRef::ScriptData(b))
        | (StringRef::Text(a), StringRef::Text(b))
        | (StringRef::Frame(a), StringRef::Frame(b)) => (a as usize, b as usize),
        _ => return false,
    };
    let a = base_a + start_a;
    let b = base_b + start_b;
    a < b + len_b && b < a + len_a
}

/// 0x140008220 accepts lowercase 0x, treats nondigits as decimal zeroes,
/// and negates the result if a minus occurs anywhere in a decimal string.
fn native_integer(bytes: &[u8], signed_decimal: bool) -> i32 {
    if let Some(hex) = bytes.strip_prefix(b"0x") {
        return hex.iter().fold(0i32, |value, &byte| {
            let digit = if byte.is_ascii_digit() {
                i32::from(byte - b'0')
            } else {
                (i32::from(byte as i8) | 0x20) - 0x57
            };
            value.wrapping_mul(16).wrapping_add(digit)
        });
    }
    let value = bytes.iter().fold(0i32, |value, &byte| {
        let digit = if byte.is_ascii_digit() {
            byte - b'0'
        } else {
            0
        };
        value.wrapping_mul(10).wrapping_add(i32::from(digit))
    });
    if signed_decimal && bytes.contains(&b'-') {
        value.wrapping_neg()
    } else {
        value
    }
}

fn last_single_byte_dot(bytes: &[u8]) -> Result<Option<usize>, &'static str> {
    let mut last_dot = None;
    let mut i = 0;
    while let Some(&byte) = bytes.get(i) {
        if byte == b'.' {
            last_dot = Some(i);
        }
        i += 1 + usize::from(lead(byte));
    }
    if i > bytes.len() {
        return Err("CMVS extension scan steps past a trailing lead byte");
    }
    Ok(last_dot)
}

#[derive(Clone, Copy)]
enum FormatArg<'a> {
    Integer(u32),
    String(&'a [u8]),
}

/// Reviewed paths in 0x140006ed0. These callers supply a single vararg.
fn sprintf(
    pattern: &[u8],
    argument: FormatArg<'_>,
    single_x86_word: bool,
    read_destination: impl Fn(usize) -> Result<u8, &'static str>,
) -> Result<Vec<u8>, &'static str> {
    let mut output = Vec::new();
    let mut arg = Some(argument);
    let mut i = 0;
    while i < pattern.len() {
        let byte = pattern[i];
        i += 1;
        if byte != b'%' {
            output.push(byte);
            continue;
        }
        let mut conversion = *pattern
            .get(i)
            .ok_or("format ends in %; the original reads past the terminator")?;
        i += 1;
        match conversion {
            b'%' => {
                output.push(b'%');
                continue;
            }
            b'r' => {
                while let byte @ 1..=255 = read_destination(output.len())? {
                    output.push(byte);
                }
                continue;
            }
            b'l' => {
                conversion = *pattern.get(i).ok_or("format ends in %l")?;
                i += 1;
                if !matches!(conversion.to_ascii_lowercase(), b'd' | b'u' | b'x' | b'o') {
                    continue;
                }
            }
            _ => {}
        }
        let kind = conversion.to_ascii_lowercase();
        if conversion != b'c'
            && !matches!(kind, b'd' | b'u' | b'z' | b'x' | b'o' | b's' | b'f' | b't')
        {
            continue;
        }
        if single_x86_word && matches!(kind, b'f' | b't') {
            return Err("64-bit conversion reads past the single CVNS vararg");
        }
        let wide = conversion.is_ascii_uppercase();
        let value = arg
            .take()
            .ok_or("the original call supplies one format argument")?;
        if let (b'c', FormatArg::Integer(value)) = (kind, value) {
            output.push(value as u8);
            continue;
        }
        let sign = if !matches!(kind, b's' | b'c') && matches!(pattern.get(i), Some(b'+' | b'-')) {
            let sign = pattern[i];
            i += 1;
            Some(sign)
        } else {
            None
        };
        let mut width = format_width(pattern, &mut i);
        let precision = if kind == b'f' && pattern.get(i) == Some(&b'.') {
            i += 1;
            format_width(pattern, &mut i)
        } else {
            0
        };
        match (kind, value) {
            (b's', FormatArg::String(bytes)) => {
                let wide_bytes;
                let bytes = if wide {
                    wide_bytes = format_full_width(bytes)?;
                    width *= 2;
                    wide_bytes.as_slice()
                } else {
                    bytes
                };
                let count = if width == 0 {
                    bytes.len()
                } else {
                    width.min(bytes.len())
                };
                output.extend_from_slice(&bytes[..count]);
                output.resize(output.len() + width.saturating_sub(count), b' ');
            }
            (b'd' | b'u' | b'z' | b'x' | b'o' | b'f' | b't', FormatArg::Integer(value)) => {
                let digits = match kind {
                    b'x' => fixed_radix(value, 4, width)?,
                    b'o' => fixed_radix(value, 3, width)?,
                    b't' => decimal_i64_word(value, width)?,
                    b'f' => decimal_subnormal(width, precision, sign)?,
                    _ => decimal(kind, value, sign, width)?,
                };
                if wide {
                    output.extend(format_full_width(&digits)?);
                } else {
                    output.extend(digits);
                }
            }
            _ => return Err("format conversion does not match the native argument type"),
        }
    }
    Ok(output)
}

fn format_width(pattern: &[u8], index: &mut usize) -> usize {
    let mut width = 0;
    for _ in 0..2 {
        let Some(digit @ b'0'..=b'9') = pattern.get(*index) else {
            break;
        };
        width = width * 10 + usize::from(digit - b'0');
        *index += 1;
    }
    width
}

fn decimal_i64_word(value: u32, width: usize) -> Result<Vec<u8>, &'static str> {
    let value = i64::from(value);
    let mut digits = Vec::new();
    if width == 0 {
        let mut remainder = value;
        let mut divisor = 10_i64.wrapping_pow(19);
        while divisor != 0 {
            let digit = remainder / divisor;
            if !(0..=9).contains(&digit) {
                return Err("signed 64-bit format indexes outside its digit table");
            }
            digits.push(b'0' + digit as u8);
            remainder %= divisor;
            divisor /= 10;
        }
        return Ok(digits);
    }

    digits.resize(width, b' ');
    let mut power = 1i64;
    for (position, digit) in digits.iter_mut().rev().enumerate() {
        if position == 0 || value >= power {
            let value = value
                .checked_div(power)
                .ok_or("signed 64-bit format has a zero divisor")?
                % 10;
            let value = u8::try_from(value)
                .map_err(|_| "signed 64-bit format indexes outside its digit table")?;
            *digit = b'0' + value;
        }
        power = power.wrapping_mul(10);
    }
    Ok(digits)
}

/// The integer syscall passes r8d. All its binary64 bit patterns truncate to
/// zero with the native wrapped 32-bit fraction scale, whatever the script word.
fn decimal_subnormal(
    width: usize,
    precision: usize,
    sign: Option<u8>,
) -> Result<Vec<u8>, &'static str> {
    if width == 0 {
        return Err("zero-width float format writes before its native scratch cell");
    }
    if 10u32.wrapping_pow(precision as u32) == 0 {
        return Err("float format has a zero divisor");
    }
    let mut digits = vec![b' '; width - 1];
    digits.extend_from_slice(b"0.");
    if let Some(sign) = sign {
        if width == 1 {
            return Err("float format width leaves no room for the sign");
        }
        digits[width - 2] = if sign == b'+' { b'+' } else { b' ' };
    }
    digits.resize(digits.len() + precision, b'0');
    Ok(digits)
}

fn fixed_radix(value: u32, bits: usize, width: usize) -> Result<Vec<u8>, &'static str> {
    if width > 32usize.div_ceil(bits) {
        return Err("format width reaches a zero divisor in the original");
    }
    let mask = (1u32 << bits) - 1;
    Ok((0..width)
        .rev()
        .map(|digit| b"0123456789ABCDEF"[((value >> (digit * bits)) & mask) as usize])
        .collect())
}

/// CP932 mapping in formatter helper 0x140006b80. Existing pairs pass through.
fn format_full_width(mut bytes: &[u8]) -> Result<Vec<u8>, &'static str> {
    let mut output = Vec::new();
    while let Some((&byte, rest)) = bytes.split_first() {
        bytes = rest;
        let word = match byte {
            b' '..=b'/' => [
                0x8140u16, 0x8149, 0x8168, 0x8194, 0x8190, 0x8193, 0x8195, 0x8166, 0x8169, 0x816a,
                0x8196, 0x817b, 0x8143, 0x8151, 0x8144, 0x815e,
            ][(byte - b' ') as usize],
            b':'..=b'@' => {
                [0x8146, 0x8147, 0x8183, 0x8181, 0x8184, 0x8148, 0x8197][(byte - b':') as usize]
            }
            b'['..=b'`' => [0x816d, 0x818f, 0x816e, 0x814f, 0x8151, 0x814d][(byte - b'[') as usize],
            b'{'..=b'~' => [0x816f, 0x8162, 0x8170, 0x8150][(byte - b'{') as usize],
            b'0'..=b'9' => 0x824f + u16::from(byte - b'0'),
            b'A'..=b'Z' => 0x8260 + u16::from(byte - b'A'),
            b'a'..=b'z' => 0x8281 + u16::from(byte - b'a'),
            byte if lead(byte) => {
                let (&trail, remaining) = bytes
                    .split_first()
                    .ok_or("truncated CP932 pair in format argument")?;
                bytes = remaining;
                u16::from_be_bytes([byte, trail])
            }
            byte => u16::from(byte),
        };
        let [high, low] = word.to_be_bytes();
        if high != 0 {
            output.push(high);
        }
        output.push(low);
        if output.len() >= 256 {
            return Err("full-width conversion exceeds the native format scratch cell");
        }
    }
    Ok(output)
}

/// Decimal paths use wrapped u32 powers of ten and drop digits outside the requested width.
fn decimal(kind: u8, value: u32, sign: Option<u8>, width: usize) -> Result<Vec<u8>, &'static str> {
    let signed = value as i32;
    let magnitude = if kind == b'd' {
        signed.unsigned_abs()
    } else {
        value
    };
    let prefix = if kind == b'd' && sign.is_some() && signed < 0 {
        Some(b'-')
    } else if kind == b'd' && sign == Some(b'+') {
        Some(b'+')
    } else {
        None
    };
    if width == 0 {
        if kind == b'z' {
            return if magnitude == 0 {
                Err("zero with a zero %z width writes before the original buffer")
            } else {
                Ok(Vec::new())
            };
        }
        let mut bytes = Vec::new();
        bytes.extend(prefix);
        bytes.extend(magnitude.to_string().bytes());
        return Ok(bytes);
    }
    let mut digits = vec![if kind == b'z' { b'0' } else { b' ' }; width];
    let mut power = 1u32;
    for (position, digit) in digits.iter_mut().rev().enumerate() {
        if position == 0 || magnitude >= power {
            let quotient = magnitude
                .checked_div(power)
                .ok_or("format width reaches a zero divisor in the original")?;
            *digit = b'0' + (quotient % 10) as u8;
        }
        power = power.wrapping_mul(10);
    }
    if kind == b'd' && sign.is_some() {
        let first = digits.iter().position(|&b| b != b' ').expect("one digit");
        if first == 0 {
            return Err("signed format width leaves no room for the sign");
        }
        digits[first - 1] = prefix.unwrap_or(b' ');
    }
    Ok(digits)
}

/// 0x140008140 walks the first string and checks the terminator of the second.
/// 0x1400081b0 walks only the pattern, which is the second native operand.
fn compare_folded(first: &[u8], second: &[u8], whole: bool) -> Result<bool, &'static str> {
    let mut i = 0;
    while let Some(&b) = first.get(i) {
        if lead(b) {
            if b != byte_at(second, i) || byte_at(first, i + 1) != byte_at(second, i + 1) {
                return Ok(false);
            }
            if i + 1 == first.len() {
                return Err("string comparison steps past a trailing lead byte");
            }
            i += 2;
        } else {
            if !b.eq_ignore_ascii_case(&byte_at(second, i)) {
                return Ok(false);
            }
            i += 1;
        }
    }
    Ok(!whole || i == second.len())
}

// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Bounded Unicode printf over explicit numeric/string values, without C
//! varargs or host pointers. Conversion policy follows NitroSharp's MIT
//! PrintfFormatter.cs (77a93db4); integer words use its signed 64-bit profile.
use anyhow::{Result, ensure};

#[derive(Clone, Copy, Debug)]
pub enum Value<'a> {
    Number(f64),
    Text(&'a str),
}
#[derive(Clone, Debug)]
pub struct Output {
    pub text: String,
    pub invalid: bool,
    pub arguments_used: usize,
}

fn special(number: f64) -> Option<&'static str> {
    if number.is_nan() {
        Some("NaN")
    } else if number == f64::INFINITY {
        Some("Infinity")
    } else if number == f64::NEG_INFINITY {
        Some("-Infinity")
    } else {
        None
    }
}
fn exponent_notation(text: String) -> String {
    let Some((mantissa, exponent)) = text.split_once('e') else {
        return text;
    };
    let exponent: i32 = exponent.parse().expect("Rust numeric exponent");
    format!(
        "{mantissa}E{}{exponent:02}",
        if exponent < 0 { '-' } else { '+' },
        exponent = exponent.unsigned_abs()
    )
}
fn general(number: f64, precision: Option<usize>) -> String {
    if let Some(text) = special(number) {
        return text.into();
    }
    if number == 0. {
        return number.to_string();
    }
    let exponent = number.abs().log10().floor() as i32;
    let Some(precision) = precision.filter(|p| *p != 0) else {
        return if exponent < -4 || exponent >= 17 {
            exponent_notation(format!("{number:e}"))
        } else {
            number.to_string()
        };
    };
    let mut result = if exponent < -4 || exponent >= precision as i32 {
        format!("{:.*e}", precision.saturating_sub(1), number)
    } else {
        format!(
            "{:.*}",
            (precision as i32 - exponent - 1).max(0) as usize,
            number
        )
    };
    let split = result.find('e').unwrap_or(result.len());
    if result[..split].contains('.') {
        let end = result[..split]
            .trim_end_matches('0')
            .trim_end_matches('.')
            .len();
        result.replace_range(end..split, "");
    }
    exponent_notation(result)
}

/// Supports d/i/u/x/X/s/f/F/g/G, %% and the terminal numeric-width shorthand
/// `%02`. Invalid conversions retain their exact template and set `invalid`.
/// String width/precision count UTF-16 units, independently of UTF-8 encoding.
pub fn format(template: &str, values: &[Value<'_>], limit: usize) -> Result<Output> {
    ensure!(
        limit <= 16 * 1024 * 1024 && template.len() <= limit,
        "printf value byte budget"
    );
    let mut output = Output {
        text: String::new(),
        invalid: false,
        arguments_used: 0,
    };
    let bytes = template.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        let begin = at;
        if bytes[at] != b'%' {
            at += template[at..].chars().next().unwrap().len_utf8();
            ensure!(
                output.text.len() + at - begin <= limit,
                "printf output byte budget"
            );
            output.text.push_str(&template[begin..at]);
            continue;
        }
        at += 1;
        if bytes.get(at) == Some(&b'%') {
            ensure!(output.text.len() < limit, "printf output byte budget");
            output.text.push('%');
            at += 1;
            continue;
        }
        let left = bytes.get(at) == Some(&b'-');
        if left {
            at += 1;
        }
        let zero = bytes.get(at) == Some(&b'0');
        let width_start = at;
        let mut width = 0usize;
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            width = (width * 10 + usize::from(bytes[at] - b'0')).min(10000);
            at += 1;
        }
        let width_only = at > width_start && !left;
        let precision = if bytes.get(at) == Some(&b'.') {
            at += 1;
            let mut p = 0;
            while bytes.get(at).is_some_and(u8::is_ascii_digit) {
                p = (p * 10 + usize::from(bytes[at] - b'0')).min(99);
                at += 1;
            }
            Some(p)
        } else {
            None
        };
        let value = values.get(output.arguments_used).copied();
        let shorthand = at == bytes.len()
            && width_only
            && precision.is_none()
            && matches!(value, Some(Value::Number(_)));
        let code = if shorthand {
            b'd'
        } else {
            bytes.get(at).copied().unwrap_or(0)
        };
        if !shorthand && at < bytes.len() {
            at += template[at..].chars().next().unwrap().len_utf8();
        }
        let mut text = None;
        if let Some(value) = value.filter(|_| b"disuxXfFgG".contains(&code)) {
            output.arguments_used += 1;
            if code == b's' {
                let s = match value {
                    Value::Number(n) => general(n, None),
                    Value::Text(s) => s.into(),
                };
                text = Some(if let Some(p) = precision {
                    String::from_utf16_lossy(&s.encode_utf16().take(p).collect::<Vec<_>>())
                } else {
                    s
                });
            } else {
                let number = match value {
                    Value::Number(n) => Some(n),
                    Value::Text(s) => s.parse::<f64>().ok(),
                };
                if let Some(number) = number {
                    text = match code {
                        b'd' | b'i' | b'u' | b'x' | b'X'
                            if number.is_finite()
                                && number >= i64::MIN as f64
                                && number < 9223372036854775808. =>
                        {
                            let n = number.trunc() as i64;
                            Some(match code {
                                b'u' => (n as u32).to_string(),
                                b'x' => format!("{n:x}"),
                                b'X' => format!("{n:X}"),
                                _ => n.to_string(),
                            })
                        }
                        b'f' | b'F' => {
                            Some(special(number).map(str::to_owned).unwrap_or_else(|| {
                                format!("{:.*}", precision.unwrap_or(6), number)
                            }))
                        }
                        b'g' | b'G' => Some(general(number, precision)),
                        _ => None,
                    };
                }
            }
        }
        if let Some(text) = text {
            let padding = width.saturating_sub(text.encode_utf16().count());
            ensure!(
                output.text.len() + padding + text.len() <= limit,
                "printf output byte budget"
            );
            if left {
                output.text.push_str(&text);
                output.text.extend(std::iter::repeat_n(' ', padding));
            } else if zero && text.starts_with('-') {
                output.text.push('-');
                output.text.extend(std::iter::repeat_n('0', padding));
                output.text.push_str(&text[1..]);
            } else {
                output
                    .text
                    .extend(std::iter::repeat_n(if zero { '0' } else { ' ' }, padding));
                output.text.push_str(&text);
            }
        } else {
            output.invalid = true;
            ensure!(
                output.text.len() + at - begin <= limit,
                "printf output byte budget"
            );
            output.text.push_str(&template[begin..at]);
        }
    }
    Ok(output)
}

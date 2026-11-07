// SPDX-License-Identifier: MPL-2.0
//! Native integer/value grammar: 44F800 groups +,-,&,|,^ at one precedence;
//! 44F880 groups *,/,% and exponentiation. Unknown names fail at the caller PC.
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Integer(i32),
    Float(f32),
    String(String),
    List(Vec<Value>),
}
impl Value {
    pub fn integer(&self) -> Result<i32> {
        match self {
            Self::Integer(n) => Ok(*n),
            // Native __ftol2_sse returns integer-indefinite on invalid/range.
            Self::Float(n) => Ok(
                if n.is_finite() && *n >= -2147483648.0 && *n < 2147483648.0 {
                    n.trunc() as i32
                } else {
                    i32::MIN
                },
            ),
            _ => bail!("SugarPot integer required"),
        }
    }
    pub fn string(&self) -> Result<&str> {
        match self {
            Self::String(s) => Ok(s),
            _ => bail!("SugarPot string required"),
        }
    }
}
pub fn value(source: &str, vars: &BTreeMap<String, Value>) -> Result<Value> {
    value_banks(source, vars, vars)
}
///451920 selects VAR only for explicit $/%/! references and DEF for bare
///names. Distinct banks may legally contain the same spelling.
pub(crate) fn value_banks(
    source: &str,
    vars: &BTreeMap<String, Value>,
    defs: &BTreeMap<String, Value>,
) -> Result<Value> {
    let source = source.trim();
    if source.starts_with(['$', '"', '\'']) {
        let mut quote = None;
        let mut split = vec![];
        let mut start = 0;
        for (at, c) in source.char_indices() {
            if let Some(q) = quote {
                if c == q {
                    quote = None;
                }
            } else if matches!(c, '"' | '\'') {
                quote = Some(c);
            } else if c == '+' {
                split.push(&source[start..at]);
                start = at + 1;
            }
        }
        if !split.is_empty() {
            ensure!(quote.is_none(), "SugarPot concatenation literal");
            split.push(&source[start..]);
            let mut joined = String::new();
            for part in split {
                joined.push_str(value_banks(part, vars, defs)?.string()?);
                ensure!(joined.len() <= 65535, "SugarPot string budget");
            }
            return Ok(Value::String(joined));
        }
    }

    if source.starts_with(['\"', '\'']) {
        let q = source.chars().next().unwrap();
        ensure!(
            source.len() >= 2 && source.ends_with(q),
            "SugarPot string literal"
        );
        return Ok(Value::String(source[1..source.len() - 1].into()));
    }
    let identifier = source.strip_prefix(['%', '$', '!']).unwrap_or(source);
    if !identifier.is_empty()
        && identifier
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_')
    {
        let bank = if source.starts_with(['%', '$', '!']) {
            vars
        } else {
            defs
        };
        if let Some(value) = bank.get(identifier) {
            return Ok(value.clone());
        }
    }
    if source.starts_with('$') {
        bail!("unknown SugarPot string variable {source}");
    }
    if let Ok(n) = source.parse::<f32>() {
        if source.contains('.') {
            ensure!(n.is_finite(), "SugarPot nonfinite float");
            return Ok(Value::Float(n));
        }
    }
    Ok(Value::Integer(integer_banks(source, vars, defs)?))
}
pub fn integer(source: &str, vars: &BTreeMap<String, Value>) -> Result<i32> {
    integer_banks(source, vars, vars)
}
pub(crate) fn integer_banks(
    source: &str,
    vars: &BTreeMap<String, Value>,
    defs: &BTreeMap<String, Value>,
) -> Result<i32> {
    let mut p = Parser {
        s: source.as_bytes(),
        at: 0,
        vars,
        defs,
        depth: 0,
    };
    let n = p.expression()?;
    p.space();
    ensure!(
        p.at == p.s.len(),
        "unsupported SugarPot integer expression {source:?}"
    );
    Ok(n)
}
pub fn condition(source: &str, vars: &BTreeMap<String, Value>) -> Result<bool> {
    condition_banks(source, vars, vars)
}
pub(crate) fn condition_banks(
    source: &str,
    vars: &BTreeMap<String, Value>,
    defs: &BTreeMap<String, Value>,
) -> Result<bool> {
    let source = source.trim();
    let mut quote = false;
    let mut depth = 0;
    for (i, c) in source.char_indices() {
        match c {
            '"' => quote = !quote,
            '(' if !quote => depth += 1,
            ')' if !quote => depth -= 1,
            _ => {}
        }
        if quote || depth != 0 {
            continue;
        }
        for op in ["==", "!=", ">=", "<=", ">", "<"] {
            if source[i..].starts_with(op) {
                let a = value_banks(&source[..i], vars, defs)?;
                let b = value_banks(&source[i + op.len()..], vars, defs)?;
                let order =
                    match (&a, &b) {
                        (Value::Integer(a), Value::Integer(b)) => a.cmp(b),
                        (Value::Float(a), Value::Float(b)) => (*a as f64)
                            .partial_cmp(&(*b as f64))
                            .context("SugarPot nonfinite comparison")?,
                        (Value::Integer(a), Value::Float(b)) => (*a as f64)
                            .partial_cmp(&(*b as f64))
                            .context("SugarPot nonfinite comparison")?,
                        (Value::Float(a), Value::Integer(b)) => (*a as f64)
                            .partial_cmp(&(*b as f64))
                            .context("SugarPot nonfinite comparison")?,
                        (Value::String(a), Value::String(b)) => {
                            ensure!(
                                !matches!(op, ">=" | "<="),
                                "SugarPot native string comparison operator"
                            );
                            let (a, _, ae) = encoding_rs::SHIFT_JIS.encode(a);
                            let (b, _, be) = encoding_rs::SHIFT_JIS.encode(b);
                            ensure!(!ae && !be, "SugarPot comparison CP932");
                            a.cmp(&b)
                        }
                        _ => bail!("SugarPot condition type mismatch"),
                    };
                return Ok(match op {
                    "==" => order.is_eq(),
                    "!=" => !order.is_eq(),
                    ">=" => !order.is_lt(),
                    "<=" => !order.is_gt(),
                    ">" => order.is_gt(),
                    "<" => order.is_lt(),
                    _ => unreachable!(),
                });
            }
        }
    }
    bail!("SugarPot comparison operator required")
}
struct Parser<'a> {
    s: &'a [u8],
    at: usize,
    vars: &'a BTreeMap<String, Value>,
    defs: &'a BTreeMap<String, Value>,
    depth: usize,
}
impl Parser<'_> {
    fn space(&mut self) {
        while self.s.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }
    fn take(&mut self, chars: &[u8]) -> Option<u8> {
        self.space();
        let c = *self.s.get(self.at)?;
        if chars.contains(&c) {
            self.at += 1;
            Some(c)
        } else {
            None
        }
    }
    fn expression(&mut self) -> Result<i32> {
        let mut n = self.product()?;
        while let Some(op) = self.take(b"+-&|^") {
            let b = self.product()?;
            n = match op {
                b'+' => n.wrapping_add(b),
                b'-' => n.wrapping_sub(b),
                b'&' => n & b,
                b'|' => n | b,
                b'^' => n ^ b,
                _ => unreachable!(),
            };
        }
        Ok(n)
    }
    fn product(&mut self) -> Result<i32> {
        let mut n = self.atom()?;
        while let Some(op) = self.take(b"*/%`") {
            let b = self.atom()?;
            n = match op {
                b'*' => n.wrapping_mul(b),
                b'`' => {
                    let value = (n as f64).powi(b);
                    if value.is_finite() && value >= i32::MIN as f64 && value < i32::MAX as f64 + 1.
                    {
                        value as i32
                    } else {
                        i32::MIN
                    }
                }
                b'/' => {
                    ensure!(b != 0, "SugarPot division by zero");
                    n.wrapping_div(b)
                }
                b'%' => {
                    ensure!(b != 0, "SugarPot remainder by zero");
                    n.wrapping_rem(b)
                }
                _ => unreachable!(),
            };
        }
        Ok(n)
    }
    fn atom(&mut self) -> Result<i32> {
        self.space();
        self.depth += 1;
        ensure!(self.depth <= 64, "SugarPot expression depth");
        let result = if let Some(sign) = self.take(b"+-~") {
            let n = self.atom()?;
            match sign {
                b'-' => n.wrapping_neg(),
                b'~' => !n,
                _ => n,
            }
        } else if self.take(b"(").is_some() {
            let n = self.expression()?;
            ensure!(self.take(b")").is_some(), "SugarPot expression parenthesis");
            n
        } else {
            let variable = self.take(b"%!").is_some();
            self.space();
            let start = self.at;
            while self
                .s
                .get(self.at)
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
            {
                self.at += 1;
            }
            ensure!(self.at > start, "SugarPot integer token missing");
            let name = std::str::from_utf8(&self.s[start..self.at])?;
            if variable {
                self.vars
                    .get(name)
                    .with_context(|| format!("unknown SugarPot variable %{name}"))?
                    .integer()?
            } else if let Some(hex) = name.strip_prefix("0x").or_else(|| name.strip_prefix("0X")) {
                crt_positive(hex, 16)
            } else if name.bytes().all(|c| c.is_ascii_digit()) {
                crt_positive(name, 10)
            } else {
                self.defs
                    .get(name)
                    .with_context(|| format!("unknown SugarPot constant {name}"))?
                    .integer()?
            }
        };
        self.depth -= 1;
        Ok(result)
    }
}

/// The native 32-bit CRT strtol clips positive magnitudes before its caller
/// applies a separately parsed minus/complement prefix (45BD80/44F950).
pub(crate) fn crt_positive(digits: &str, radix: u32) -> i32 {
    digits
        .chars()
        .filter_map(|c| c.to_digit(radix))
        .fold(0_i32, |n, d| {
            n.saturating_mul(radix as i32).saturating_add(d as i32)
        })
}

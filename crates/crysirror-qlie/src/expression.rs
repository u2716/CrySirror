//! Bounded native integer/string expressions. Arithmetic wraps at 32 bits.
use anyhow::{bail, ensure, Context, Result};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Int(i32),
    Text(String),
}
impl Value {
    pub fn integer(&self) -> Result<i32> {
        match self {
            Self::Int(n) => Ok(*n),
            Self::Text(s) if s.is_empty() => Ok(0),
            Self::Text(s) => parse_integer(s).context("QLIE integer conversion"),
        }
    }
    pub fn text(&self) -> String {
        match self {
            Self::Int(n) => n.to_string(),
            Self::Text(s) => s.clone(),
        }
    }
    pub fn truth(&self) -> bool {
        match self {
            Self::Int(n) => *n != 0,
            Self::Text(s) => !s.is_empty(),
        }
    }
}
pub fn parse_integer(s: &str) -> Option<i32> {
    let s = s.trim();
    if let Some(s) = s.strip_prefix('$') {
        u32::from_str_radix(s, 16).ok().map(|n| n as i32)
    } else {
        s.parse().ok()
    }
}
#[derive(Clone, Debug)]
enum Token {
    Number(i32),
    String(String),
    Name(String),
    Op(String),
    End,
}
fn tokens(s: &str) -> Result<Vec<Token>> {
    ensure!(s.len() <= 65536, "QLIE expression length limit");
    let chars = s.chars().collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        if c.is_whitespace() {
            at += 1;
            continue;
        }
        if c == '"' {
            let start = at + 1;
            at += 1;
            while at < chars.len() && chars[at] != '"' {
                at += 1
            }
            ensure!(at < chars.len(), "QLIE string delimiter");
            out.push(Token::String(chars[start..at].iter().collect()));
            at += 1;
            continue;
        }
        if c.is_ascii_digit() || c == '$' {
            let start = at;
            at += 1;
            while at < chars.len() && chars[at].is_ascii_hexdigit() {
                at += 1
            }
            let s = chars[start..at].iter().collect::<String>();
            out.push(Token::Number(
                parse_integer(&s).with_context(|| format!("QLIE integer literal {s}"))?,
            ));
            continue;
        }
        if c.is_alphanumeric() || matches!(c, '_' | '#' | '@') {
            let start = at;
            at += 1;
            while at < chars.len()
                && (chars[at].is_alphanumeric()
                    || matches!(chars[at], '_' | '#' | '@' | '.')
                    || (chars[start] == '@' && chars[at] == '!'))
            {
                at += 1
            }
            out.push(Token::Name(chars[start..at].iter().collect()));
            continue;
        }
        let mut op = c.to_string();
        if let Some(next) = chars.get(at + 1) {
            let pair = format!("{c}{next}");
            if matches!(
                pair.as_str(),
                "==" | "!=" | "<=" | ">=" | "&&" | "||" | "<<" | ">>"
            ) {
                op = pair;
                at += 1;
            }
        }
        ensure!(
            matches!(
                op.as_str(),
                "+" | "-"
                    | "*"
                    | "/"
                    | "%"
                    | "&"
                    | "|"
                    | "^"
                    | "!"
                    | "~"
                    | "("
                    | ")"
                    | "["
                    | "]"
                    | ","
                    | "=="
                    | "!="
                    | "<="
                    | ">="
                    | "<"
                    | ">"
                    | "&&"
                    | "||"
                    | "<<"
                    | ">>"
            ),
            "unsupported QLIE expression token {op}"
        );
        out.push(Token::Op(op));
        at += 1;
    }
    ensure!(out.len() <= 4096, "QLIE expression token limit");
    out.push(Token::End);
    Ok(out)
}
struct Parser<'a, F> {
    tokens: Vec<Token>,
    at: usize,
    lookup: &'a mut F,
    depth: usize,
}
impl<F: FnMut(&str) -> Result<Value>> Parser<'_, F> {
    fn expression(&mut self, min: u8, evaluate: bool) -> Result<Value> {
        self.depth += 1;
        ensure!(self.depth < 128, "QLIE expression nesting limit");
        let token = self.tokens[self.at].clone();
        self.at += 1;
        let mut left = match token {
            Token::Number(n) => Value::Int(n),
            Token::String(s) => Value::Text(s),
            Token::Name(mut name) => {
                if self.op("[") {
                    self.at += 1;
                    let mut indices = Vec::new();
                    loop {
                        ensure!(indices.len() < 4, "QLIE array dimension limit");
                        let value = if self.op(",") || self.op("]") {
                            Value::Int(0)
                        } else {
                            self.expression(0, evaluate)?
                        };
                        indices.push(if evaluate { value.integer()? } else { 0 }.to_string());
                        if !self.op(",") {
                            break;
                        }
                        self.at += 1;
                    }
                    ensure!(self.op("]"), "QLIE array index delimiter");
                    self.at += 1;
                    name = format!("{name}[{}]", indices.join(","));
                }
                if !evaluate {
                    Value::Int(0)
                } else if name.starts_with("@@") {
                    Value::Text(name)
                } else {
                    (self.lookup)(&name)?
                }
            }
            Token::Op(op) if op == "(" => {
                let value = self.expression(0, evaluate)?;
                ensure!(self.op(")"), "QLIE expression closing parenthesis");
                self.at += 1;
                value
            }
            Token::Op(op) if matches!(op.as_str(), "+" | "-" | "!" | "~") => {
                let value = self.expression(11, evaluate)?;
                if !evaluate {
                    Value::Int(0)
                } else {
                    match op.as_str() {
                        "!" => Value::Int(i32::from(!value.truth())),
                        "~" => Value::Int(!value.integer()?),
                        "-" => Value::Int(value.integer()?.wrapping_neg()),
                        _ => Value::Int(value.integer()?),
                    }
                }
            }
            _ => bail!("QLIE expected expression value"),
        };
        loop {
            let Token::Op(op) = &self.tokens[self.at] else {
                break;
            };
            let op = op.clone();
            let precedence = match op.as_str() {
                "||" => 1,
                "&&" => 2,
                // The native calculator's bit masks bind before comparisons:
                // system scripts use `flags & mask != 0` without extra brackets.
                "==" | "!=" => 3,
                "<" | ">" | "<=" | ">=" => 4,
                "|" => 5,
                "^" => 6,
                "&" => 7,
                "<<" | ">>" => 8,
                "+" | "-" => 9,
                "*" | "/" | "%" => 10,
                _ => break,
            };
            if precedence < min {
                break;
            }
            self.at += 1;
            let run = evaluate && !(op == "&&" && !left.truth() || op == "||" && left.truth());
            let right = self.expression(precedence + 1, run)?;
            if !evaluate {
                left = Value::Int(0);
                continue;
            }
            left = match op.as_str() {
                "&&" => Value::Int(i32::from(left.truth() && right.truth())),
                "||" => Value::Int(i32::from(left.truth() || right.truth())),
                "+" if matches!(left, Value::Text(_)) || matches!(right, Value::Text(_)) => {
                    Value::Text(left.text() + &right.text())
                }
                "==" => Value::Int(i32::from(left == right)),
                "!=" => Value::Int(i32::from(left != right)),
                _ => {
                    let a = left.integer()?;
                    let b = right.integer()?;
                    Value::Int(match op.as_str() {
                        "+" => a.wrapping_add(b),
                        "-" => a.wrapping_sub(b),
                        "*" => a.wrapping_mul(b),
                        "/" => {
                            ensure!(b != 0, "QLIE division by zero");
                            a.wrapping_div(b)
                        }
                        "%" => {
                            ensure!(b != 0, "QLIE remainder by zero");
                            a.wrapping_rem(b)
                        }
                        "&" => a & b,
                        "|" => a | b,
                        "^" => a ^ b,
                        "<<" => a.wrapping_shl(b as u32),
                        ">>" => a.wrapping_shr(b as u32),
                        "<" => i32::from(a < b),
                        ">" => i32::from(a > b),
                        "<=" => i32::from(a <= b),
                        ">=" => i32::from(a >= b),
                        _ => unreachable!(),
                    })
                }
            };
        }
        self.depth -= 1;
        Ok(left)
    }
    fn op(&self, op: &str) -> bool {
        matches!(&self.tokens[self.at],Token::Op(s) if s==op)
    }
}
pub fn evaluate(s: &str, mut lookup: impl FnMut(&str) -> Result<Value>) -> Result<Value> {
    if s.trim().is_empty() {
        return Ok(Value::Int(0));
    }
    let mut p = Parser {
        tokens: tokens(s)?,
        at: 0,
        lookup: &mut lookup,
        depth: 0,
    };
    let value = p.expression(0, true)?;
    ensure!(
        matches!(p.tokens[p.at], Token::End),
        "QLIE extra expression tokens: {s}"
    );
    Ok(value)
}

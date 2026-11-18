// SPDX-License-Identifier: MPL-2.0
// NsScript value behavior adapted from NitroSharp ConstantValue.cs at
// e3869842bb5bdd5ed95344027297620cd54454a6 (MIT, Committee of Zero).
use crate::catalog::CONSTANTS;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Value {
    #[default]
    Null,
    Number(f32),
    Delta(f32),
    Boolean(bool),
    String(String),
    Constant(u8),
    Array(u32, Box<Value>),
    Curve(Vec<[(Box<Value>, Box<Value>); 4]>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Binary {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}

impl Value {
    pub fn scalar(&self) -> &Self {
        match self {
            Self::Array(_, value) => value.scalar(),
            _ => self,
        }
    }

    pub fn number(&self) -> Result<f32> {
        match self.scalar() {
            Self::Number(n) => Ok(*n),
            Self::Boolean(b) => Ok(u8::from(*b) as f32),
            Self::Constant(n) => Ok(f32::from(*n)),
            Self::String(s) if s.is_empty() => Ok(0.),
            _ => bail!("N2System numeric operand required, got {self:?}"),
        }
    }
    pub fn truth(&self) -> Result<bool> {
        match self.scalar() {
            Self::Boolean(b) => Ok(*b),
            Self::Number(n) => Ok(*n > 0.),
            Self::String(s) => Ok(!s.is_empty()),
            _ => bail!("N2System boolean operand required, got {self:?}"),
        }
    }
    pub fn text(&self) -> Result<&str> {
        match self.scalar() {
            Self::String(s) => Ok(s),
            _ => bail!("N2System string operand required, got {self:?}"),
        }
    }
    pub fn string(&self) -> Result<String> {
        Ok(match self.scalar() {
            Self::Null => "null".into(),
            Self::Number(n) => n.to_string(),
            Self::Delta(n) => format!("@{n}"),
            Self::Boolean(b) => b.to_string(),
            Self::String(s) => s.clone(),
            Self::Constant(n) => CONSTANTS
                .get(usize::from(*n))
                .ok_or_else(|| anyhow::anyhow!("N2System constant index {n}"))?
                .to_string(),
            Self::Curve(_) => bail!("N2System cannot stringify a Bezier curve"),
            Self::Array(..) => unreachable!("scalar strips array identity"),
        })
    }
    pub fn constant(name: &str) -> Option<Self> {
        let name = match name {
            "Clickable" => "Enter",
            "UnClickable" => "Erase",
            "UnLock" => "Unlock",
            "OverlayRender" => "AddRender",
            other => other,
        };
        CONSTANTS
            .iter()
            .position(|s| s.to_lowercase() == name.to_lowercase())
            .and_then(|n| u8::try_from(n).ok())
            .map(Self::Constant)
    }
    pub fn is_constant(&self, name: &str) -> bool {
        matches!(self, Self::Constant(n) if CONSTANTS.get(usize::from(*n)).is_some_and(|s| s.eq_ignore_ascii_case(name)))
    }
    pub fn equivalent(&self, other: &Self) -> bool {
        if let (Ok(a), Ok(b)) = (self.number(), other.number()) {
            return a == b;
        }
        matches!((self, other), (Self::Null, Self::Null))
            || matches!((self,other), (Self::String(a),Self::String(b)) if a==b)
    }
    pub fn binary(&self, op: Binary, other: &Self) -> Result<Self> {
        if matches!(self, Self::Array(..)) || matches!(other, Self::Array(..)) {
            return self.scalar().binary(op, other.scalar());
        }
        use Binary::*;
        if op == Equal {
            return Ok(Self::Boolean(self.equivalent(other)));
        }
        if op == NotEqual {
            return Ok(Self::Boolean(!self.equivalent(other)));
        }
        if matches!(op, And | Or) {
            let (a, b) = (self.truth()?, other.truth()?);
            return Ok(Self::Boolean(if op == And { a && b } else { a || b }));
        }
        if op == Add {
            if let (Self::Number(a), Self::Number(b)) = (self, other) {
                return Ok(Self::Number(a + b));
            }
            if let (Self::String(a), Self::Number(b)) = (self, other) {
                if a == "@" {
                    return Ok(Self::Delta(*b));
                }
            }
            return Ok(Self::String(self.string()? + &other.string()?));
        }
        if op == Divide {
            // Unquoted `Parent/Child` in script (e.g. `case MSGWND/MSG_no`) is
            // an entity path; two non-numeric words join with a separator.
            if let (Self::String(a), Self::String(b)) = (self, other) {
                if !a.is_empty() && !b.is_empty() && a.parse::<f32>().is_err() && b.parse::<f32>().is_err() {
                    return Ok(Self::String(format!("{a}/{b}")));
                }
            }
        }
        let (Self::Number(a), Self::Number(b)) = (self, other) else {
            bail!("N2System invalid binary operands for {op:?}: {self:?}, {other:?}");
        };
        Ok(match op {
            Subtract => Self::Number(a - b),
            Multiply => Self::Number(a * b),
            Divide => Self::Number(a / b),
            Remainder => Self::Number(a % b),
            Less => Self::Boolean(a < b),
            LessEqual => Self::Boolean(a <= b),
            Greater => Self::Boolean(a > b),
            GreaterEqual => Self::Boolean(a >= b),
            _ => unreachable!(),
        })
    }
}

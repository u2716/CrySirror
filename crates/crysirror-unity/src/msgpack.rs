// SPDX-License-Identifier: MPL-2.0
//! MessagePack values (the wire format of MessagePack-CSharp, used by Unity
//! games for save and character data): a bounded decoder into a value tree.
use anyhow::{bail, ensure, Context, Result};

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Nil,
    Bool(bool),
    Int(i64),
    /// Unsigned values above `i64::MAX`.
    UInt(u64),
    Float(f64),
    Str(String),
    Bin(Vec<u8>),
    Array(Vec<Value>),
    Map(Vec<(Value, Value)>),
    /// Extension type and payload (MessagePack-CSharp uses −1 timestamps
    /// and 99/100 for LZ4 blocks).
    Ext(i8, Vec<u8>),
}

/// Nesting limit (ILLGAMES data nests a few levels).
const MAX_DEPTH: usize = 64;

pub struct Reader<'a> {
    b: &'a [u8],
    pub at: usize,
}

impl<'a> Reader<'a> {
    pub fn new(b: &'a [u8]) -> Self {
        Self { b, at: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let s = self.b.get(self.at..self.at.checked_add(n).context("msgpack length")?).context("msgpack truncated")?;
        self.at += n;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn be<const N: usize>(&mut self) -> Result<[u8; N]> {
        Ok(self.take(N)?.try_into()?)
    }
    fn len(&mut self, n: usize) -> Result<usize> {
        Ok(match n {
            1 => usize::from(self.u8()?),
            2 => usize::from(u16::from_be_bytes(self.be()?)),
            _ => u32::from_be_bytes(self.be()?) as usize,
        })
    }

    pub fn value(&mut self) -> Result<Value> {
        self.value_at(0)
    }

    fn value_at(&mut self, depth: usize) -> Result<Value> {
        ensure!(depth < MAX_DEPTH, "msgpack nesting");
        let t = self.u8()?;
        Ok(match t {
            0x00..=0x7f => Value::Int(i64::from(t)),
            0x80..=0x8f => self.map(usize::from(t & 0x0f), depth)?,
            0x90..=0x9f => self.array(usize::from(t & 0x0f), depth)?,
            0xa0..=0xbf => self.str(usize::from(t & 0x1f))?,
            0xc0 => Value::Nil,
            0xc2 => Value::Bool(false),
            0xc3 => Value::Bool(true),
            0xc4..=0xc6 => {
                let n = self.len(1 << (t - 0xc4))?;
                Value::Bin(self.take(n)?.to_vec())
            }
            0xc7..=0xc9 => {
                let n = self.len(1 << (t - 0xc7))?;
                let ty = self.u8()? as i8;
                Value::Ext(ty, self.take(n)?.to_vec())
            }
            0xca => Value::Float(f64::from(f32::from_be_bytes(self.be()?))),
            0xcb => Value::Float(f64::from_be_bytes(self.be()?)),
            0xcc => Value::Int(i64::from(self.u8()?)),
            0xcd => Value::Int(i64::from(u16::from_be_bytes(self.be()?))),
            0xce => Value::Int(i64::from(u32::from_be_bytes(self.be()?))),
            0xcf => {
                let v = u64::from_be_bytes(self.be()?);
                i64::try_from(v).map_or(Value::UInt(v), Value::Int)
            }
            0xd0 => Value::Int(i64::from(self.u8()? as i8)),
            0xd1 => Value::Int(i64::from(i16::from_be_bytes(self.be()?))),
            0xd2 => Value::Int(i64::from(i32::from_be_bytes(self.be()?))),
            0xd3 => Value::Int(i64::from_be_bytes(self.be()?)),
            0xd4..=0xd8 => {
                let n = 1usize << (t - 0xd4);
                let ty = self.u8()? as i8;
                Value::Ext(ty, self.take(n)?.to_vec())
            }
            0xd9..=0xdb => {
                let n = self.len(1 << (t - 0xd9))?;
                self.str(n)?
            }
            0xdc | 0xdd => {
                let n = self.len(if t == 0xdc { 2 } else { 4 })?;
                self.array(n, depth)?
            }
            0xde | 0xdf => {
                let n = self.len(if t == 0xde { 2 } else { 4 })?;
                self.map(n, depth)?
            }
            0xe0..=0xff => Value::Int(i64::from(t as i8)),
            0xc1 => bail!("msgpack reserved byte at {}", self.at - 1),
        })
    }

    fn str(&mut self, n: usize) -> Result<Value> {
        Ok(Value::Str(String::from_utf8_lossy(self.take(n)?).into_owned()))
    }
    fn array(&mut self, n: usize, depth: usize) -> Result<Value> {
        ensure!(n <= self.b.len() - self.at, "msgpack array length {n}");
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(self.value_at(depth + 1)?);
        }
        Ok(Value::Array(v))
    }
    fn map(&mut self, n: usize, depth: usize) -> Result<Value> {
        ensure!(n <= (self.b.len() - self.at) / 2 + 1, "msgpack map length {n}");
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            let k = self.value_at(depth + 1)?;
            v.push((k, self.value_at(depth + 1)?));
        }
        Ok(Value::Map(v))
    }
}

/// Decode one value from the start of `b`.
pub fn decode(b: &[u8]) -> Result<Value> {
    Reader::new(b).value()
}

impl Value {
    /// A map entry by string key.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Map(m) => m.iter().find(|(k, _)| matches!(k, Value::Str(s) if s == key)).map(|(_, v)| v),
            _ => None,
        }
    }
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::UInt(u) => i64::try_from(*u).ok(),
            Value::Float(f) => Some(*f as i64),
            _ => None,
        }
    }
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Value::Float(f) => Some(*f as f32),
            Value::Int(i) => Some(*i as f32),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Bin(b) => Some(b),
            _ => None,
        }
    }
    pub fn as_array(&self) -> &[Value] {
        match self {
            Value::Array(a) => a,
            _ => &[],
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            Value::Int(i) => Some(*i != 0),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_common_types() {
        // {"a": [1, -1, 300, 1.5f32, "x", true, nil], "b": bin8 [1,2]}
        let b = [0x82, 0xa1, b'a', 0x97, 0x01, 0xff, 0xcd, 0x01, 0x2c, 0xca, 0x3f, 0xc0, 0x00, 0x00, 0xa1, b'x', 0xc3, 0xc0, 0xa1, b'b', 0xc4, 0x02, 1, 2];
        let v = decode(&b).unwrap();
        let a = v.get("a").unwrap().as_array();
        assert_eq!((a[0].as_i64(), a[1].as_i64(), a[2].as_i64(), a[3].as_f32(), a[4].as_str(), a[5].as_bool()), (Some(1), Some(-1), Some(300), Some(1.5), Some("x"), Some(true)));
        assert_eq!(a[6], Value::Nil);
        assert_eq!(v.get("b").unwrap().as_bytes(), Some(&[1u8, 2][..]));
        assert!(decode(&[0x92, 0x01]).is_err());
    }
}

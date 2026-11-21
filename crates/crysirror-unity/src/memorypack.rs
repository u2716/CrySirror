// SPDX-License-Identifier: MPL-2.0
//! Cysharp MemoryPack wire format (Unity save data): little-endian
//! unmanaged values, variable-length integers, object headers (member
//! count, 255 null), version-tolerant objects (member count, a varint byte
//! length per member, then the members), UTF-8/UTF-16 strings and
//! length-prefixed arrays.
use anyhow::{bail, ensure, Context, Result};

/// `MemoryPackCode.NullObject`.
pub const NULL_OBJECT: u8 = 255;
/// `MemoryPackCode.NullCollection` (also a null string length).
pub const NULL_COLLECTION: i32 = -1;

// `VarIntCodes`: one signed byte holds -120..=127; these mark wider values.
const VAR_BYTE: i8 = -121;
const VAR_SBYTE: i8 = -122;
const VAR_U16: i8 = -123;
const VAR_I16: i8 = -124;
const VAR_U32: i8 = -125;
const VAR_I32: i8 = -126;
const VAR_U64: i8 = -127;
const VAR_I64: i8 = -128;

pub struct Reader<'a> {
    b: &'a [u8],
    pub at: usize,
}

impl<'a> Reader<'a> {
    pub fn new(b: &'a [u8]) -> Self {
        Self { b, at: 0 }
    }
    pub fn remaining(&self) -> usize {
        self.b.len() - self.at
    }
    pub fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let s = self.b.get(self.at..self.at.checked_add(n).context("memorypack length")?).context("memorypack truncated")?;
        self.at += n;
        Ok(s)
    }
    fn le<const N: usize>(&mut self) -> Result<[u8; N]> {
        Ok(self.take(N)?.try_into()?)
    }
    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    pub fn bool(&mut self) -> Result<bool> {
        Ok(self.u8()? != 0)
    }
    pub fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.le()?))
    }
    pub fn i64(&mut self) -> Result<i64> {
        Ok(i64::from_le_bytes(self.le()?))
    }
    pub fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(self.le()?))
    }
    pub fn var_int(&mut self) -> Result<i64> {
        let t = self.u8()? as i8;
        Ok(match t {
            VAR_BYTE => i64::from(self.u8()?),
            VAR_SBYTE => i64::from(self.u8()? as i8),
            VAR_U16 => i64::from(u16::from_le_bytes(self.le()?)),
            VAR_I16 => i64::from(i16::from_le_bytes(self.le()?)),
            VAR_U32 => i64::from(u32::from_le_bytes(self.le()?)),
            VAR_I32 => i64::from(self.i32()?),
            VAR_U64 => i64::try_from(u64::from_le_bytes(self.le()?)).context("memorypack varint")?,
            VAR_I64 => self.i64()?,
            v => i64::from(v),
        })
    }
    /// An object header: the member count, `None` for null.
    pub fn object_header(&mut self) -> Result<Option<u8>> {
        let n = self.u8()?;
        Ok((n != NULL_OBJECT).then_some(n))
    }
    /// A version-tolerant object: each member's bytes (`None`: null).
    pub fn tolerant(&mut self) -> Result<Option<Vec<&'a [u8]>>> {
        let Some(n) = self.object_header()? else { return Ok(None) };
        let mut lengths = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let l = self.var_int()?;
            ensure!(l >= 0, "memorypack member length");
            lengths.push(l as usize);
        }
        lengths.into_iter().map(|l| self.take(l)).collect::<Result<Vec<_>>>().map(Some)
    }
    /// A string (UTF-16 by char count, or UTF-8 as `~byteCount`, UTF-16
    /// length, bytes); `None` for null.
    pub fn string(&mut self) -> Result<Option<String>> {
        let h = self.i32()?;
        if h == NULL_COLLECTION {
            return Ok(None);
        }
        if h == 0 {
            return Ok(Some(String::new()));
        }
        if h < 0 {
            let bytes = !h as usize;
            let _utf16 = self.i32()?;
            return Ok(Some(String::from_utf8(self.take(bytes)?.to_vec()).context("memorypack UTF-8 string")?));
        }
        let units: Vec<u16> = self.take(h as usize * 2)?.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        Ok(Some(String::from_utf16(&units).context("memorypack UTF-16 string")?))
    }
    /// An unmanaged array of `size`-byte elements; `None` for null.
    pub fn array(&mut self, size: usize) -> Result<Option<&'a [u8]>> {
        let n = self.i32()?;
        if n == NULL_COLLECTION {
            return Ok(None);
        }
        ensure!(n >= 0, "memorypack array length {n}");
        Ok(Some(self.take((n as usize).checked_mul(size).context("memorypack array size")?)?))
    }
}

#[derive(Default)]
pub struct Writer {
    pub out: Vec<u8>,
}

impl Writer {
    pub fn u8(&mut self, v: u8) {
        self.out.push(v);
    }
    pub fn bool(&mut self, v: bool) {
        self.out.push(u8::from(v));
    }
    pub fn i32(&mut self, v: i32) {
        self.out.extend_from_slice(&v.to_le_bytes());
    }
    pub fn i64(&mut self, v: i64) {
        self.out.extend_from_slice(&v.to_le_bytes());
    }
    pub fn f32(&mut self, v: f32) {
        self.out.extend_from_slice(&v.to_le_bytes());
    }
    pub fn var_int(&mut self, v: i64) {
        if (-120..=127).contains(&v) {
            self.out.push(v as i8 as u8);
        } else if (0..=255).contains(&v) {
            self.out.extend_from_slice(&[VAR_BYTE as u8, v as u8]);
        } else if (0..=0xFFFF).contains(&v) {
            self.out.push(VAR_U16 as u8);
            self.out.extend_from_slice(&(v as u16).to_le_bytes());
        } else if i32::try_from(v).is_ok() {
            self.out.push(VAR_I32 as u8);
            self.i32(v as i32);
        } else {
            self.out.push(VAR_I64 as u8);
            self.i64(v);
        }
    }
    pub fn null_object(&mut self) {
        self.out.push(NULL_OBJECT);
    }
    /// A version-tolerant object from its members' bytes.
    pub fn tolerant(&mut self, members: &[Vec<u8>]) -> Result<()> {
        if members.len() >= NULL_OBJECT as usize {
            bail!("memorypack object with {} members", members.len());
        }
        self.u8(members.len() as u8);
        for m in members {
            self.var_int(m.len() as i64);
        }
        for m in members {
            self.out.extend_from_slice(m);
        }
        Ok(())
    }
    /// A UTF-8 string (`MemoryPackSerializerOptions.Utf8`).
    pub fn string(&mut self, v: Option<&str>) {
        match v {
            None => self.i32(NULL_COLLECTION),
            Some("") => self.i32(0),
            Some(s) => {
                self.i32(!(s.len() as i32));
                self.i32(s.encode_utf16().count() as i32);
                self.out.extend_from_slice(s.as_bytes());
            }
        }
    }
    pub fn array(&mut self, v: Option<&[u8]>, size: usize) {
        match v {
            None => self.i32(NULL_COLLECTION),
            Some(b) => {
                self.i32((b.len() / size.max(1)) as i32);
                self.out.extend_from_slice(b);
            }
        }
    }
}

/// Build one member's bytes with a writer closure.
pub fn member(f: impl FnOnce(&mut Writer)) -> Vec<u8> {
    let mut w = Writer::default();
    f(&mut w);
    w.out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_round_trip() {
        let mut w = Writer::default();
        for v in [0i64, -120, 127, 200, -5000, 70_000, 1 << 40] {
            w.var_int(v);
        }
        w.string(Some("恵莉佳"));
        w.string(None);
        w.string(Some(""));
        w.tolerant(&[member(|w| w.i32(7)), Vec::new(), member(|w| w.bool(true))]).unwrap();
        w.array(Some(&[1, 2, 3, 4]), 4);
        let mut r = Reader::new(&w.out);
        for v in [0i64, -120, 127, 200, -5000, 70_000, 1 << 40] {
            assert_eq!(r.var_int().unwrap(), v);
        }
        assert_eq!(r.string().unwrap().as_deref(), Some("恵莉佳"));
        assert_eq!(r.string().unwrap(), None);
        assert_eq!(r.string().unwrap().as_deref(), Some(""));
        let m = r.tolerant().unwrap().unwrap();
        assert_eq!((m[0], m[1].len(), m[2]), (&7i32.to_le_bytes()[..], 0, &[1u8][..]));
        assert_eq!(r.array(4).unwrap(), Some(&[1u8, 2, 3, 4][..]));
        assert_eq!(r.remaining(), 0);
        // UTF-16 strings (the default encoding of older writers).
        let mut b = 2i32.to_le_bytes().to_vec();
        b.extend_from_slice(&[0x42, 0x30, 0x44, 0x30]);
        assert_eq!(Reader::new(&b).string().unwrap().as_deref(), Some("あい"));
    }
}

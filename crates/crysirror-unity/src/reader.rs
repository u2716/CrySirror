// SPDX-License-Identifier: MPL-2.0
//! Little-endian object reader with Unity's 4-byte alignment rules.
use crate::PPtr;
use anyhow::{bail, ensure, Context, Result};

pub struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }
    pub fn position(&self) -> usize {
        self.at
    }
    pub fn remaining(&self) -> usize {
        self.bytes.len() - self.at
    }
    pub fn seek(&mut self, at: usize) -> Result<()> {
        ensure!(at <= self.bytes.len(), "Unity reader seek outside the object");
        self.at = at;
        Ok(())
    }
    pub fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        let end = self
            .at
            .checked_add(count)
            .filter(|end| *end <= self.bytes.len())
            .context("truncated Unity object")?;
        let slice = &self.bytes[self.at..end];
        self.at = end;
        Ok(slice)
    }
    pub fn align(&mut self) -> Result<()> {
        let pad = (4 - self.at % 4) % 4;
        self.take(pad.min(self.remaining()))?;
        Ok(())
    }
    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    /// A bool; Unity aligns after a run of bools, so callers align explicitly.
    pub fn bool(&mut self) -> Result<bool> {
        Ok(self.u8()? != 0)
    }
    pub fn i16(&mut self) -> Result<i16> {
        Ok(i16::from_le_bytes(self.take(2)?.try_into()?))
    }
    pub fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into()?))
    }
    pub fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into()?))
    }
    pub fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into()?))
    }
    pub fn i64(&mut self) -> Result<i64> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into()?))
    }
    pub fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into()?))
    }
    pub fn f32s<const N: usize>(&mut self) -> Result<[f32; N]> {
        let mut out = [0.; N];
        for v in &mut out {
            *v = self.f32()?;
        }
        Ok(out)
    }
    /// A non-negative element count whose elements of at least `unit` bytes fit.
    pub fn count(&mut self, unit: usize) -> Result<usize> {
        let n = self.i32()?;
        ensure!(n >= 0, "negative Unity element count");
        let n = n as usize;
        ensure!(n.saturating_mul(unit.max(1)) <= self.remaining(), "Unity element count exceeds the object");
        Ok(n)
    }
    /// `std::string`: i32 length, UTF-8 bytes, aligned.
    pub fn string(&mut self) -> Result<String> {
        let n = self.count(1)?;
        let raw = self.take(n)?;
        self.align()?;
        Ok(String::from_utf8_lossy(raw).into_owned())
    }
    /// A byte array (`vector<UInt8>` / `TypelessData`), aligned.
    pub fn bytes(&mut self) -> Result<&'a [u8]> {
        let n = self.count(1)?;
        let raw = self.take(n)?;
        self.align()?;
        Ok(raw)
    }
    pub fn cstring(&mut self) -> Result<String> {
        let rest = &self.bytes[self.at..];
        let Some(end) = rest.iter().position(|b| *b == 0) else {
            bail!("unterminated Unity string");
        };
        let s = String::from_utf8_lossy(&rest[..end]).into_owned();
        self.at += end + 1;
        Ok(s)
    }
    /// `PPtr<T>` of format 9: i32 file id, i32 path id.
    pub fn pptr(&mut self) -> Result<PPtr> {
        Ok(PPtr { file_id: self.i32()?, path_id: i64::from(self.i32()?) })
    }
    pub fn pptrs(&mut self) -> Result<Vec<PPtr>> {
        let n = self.count(8)?;
        (0..n).map(|_| self.pptr()).collect()
    }
    pub fn strings(&mut self) -> Result<Vec<String>> {
        let n = self.count(4)?;
        (0..n).map(|_| self.string()).collect()
    }
}

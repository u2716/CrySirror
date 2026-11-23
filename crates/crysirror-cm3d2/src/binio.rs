// SPDX-License-Identifier: MPL-2.0
//! .NET `BinaryWriter`/`BinaryReader` encoding of the original save data:
//! little-endian scalars, one-byte booleans, strings as a 7-bit encoded
//! byte length followed by UTF-8.
use anyhow::{bail, ensure, Context, Result};

#[derive(Default)]
pub struct Writer {
    pub bytes: Vec<u8>,
}

impl Writer {
    pub fn i32(&mut self, v: i32) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }
    pub fn u32(&mut self, v: u32) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }
    pub fn i64(&mut self, v: i64) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }
    pub fn u64(&mut self, v: u64) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }
    pub fn f32(&mut self, v: f32) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }
    pub fn bool(&mut self, v: bool) {
        self.bytes.push(u8::from(v));
    }
    pub fn str(&mut self, v: &str) {
        let mut n = v.len() as u32;
        while n >= 0x80 {
            self.bytes.push((n as u8) | 0x80);
            n >>= 7;
        }
        self.bytes.push(n as u8);
        self.bytes.extend_from_slice(v.as_bytes());
    }
}

pub struct Reader<'a> {
    bytes: &'a [u8],
    pub at: usize,
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }
    pub fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.at.checked_add(n).filter(|e| *e <= self.bytes.len()).with_context(|| format!("save data ends at {} (needs {n} more)", self.at))?;
        let s = &self.bytes[self.at..end];
        self.at = end;
        Ok(s)
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
    pub fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into()?))
    }
    pub fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into()?))
    }
    pub fn bool(&mut self) -> Result<bool> {
        Ok(self.take(1)?[0] != 0)
    }
    /// A collection count; negative counts are corrupt data.
    pub fn count(&mut self) -> Result<usize> {
        let n = self.i32()?;
        ensure!(n >= 0 && (n as usize) <= self.bytes.len() - self.at, "save data count {n} at {} is invalid", self.at - 4);
        Ok(n as usize)
    }
    pub fn str(&mut self) -> Result<String> {
        let mut n = 0u32;
        for shift in (0..35).step_by(7) {
            let b = self.take(1)?[0];
            n |= u32::from(b & 0x7f) << shift;
            if b & 0x80 == 0 {
                let s = self.take(n as usize)?;
                return Ok(String::from_utf8(s.to_vec()).context("save data string is not UTF-8")?);
            }
        }
        bail!("save data string length at {} is malformed", self.at)
    }
    pub fn expect_str(&mut self, tag: &str) -> Result<()> {
        let s = self.str()?;
        ensure!(s == tag, "expected save section {tag}, found {s:?}");
        Ok(())
    }
}

/// `Dictionary<K, V>`/`HashSet<T>` in the enumeration order of a .NET
/// dictionary that only grows (insertion order), which is what the original
/// serializers write. A removal followed by an insertion would reuse the freed
/// .NET slot instead; that order difference does not change the read data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderedMap<K, V> {
    entries: Vec<(K, V)>,
}

impl<K, V> Default for OrderedMap<K, V> {
    fn default() -> Self {
        Self { entries: Vec::new() }
    }
}

impl<K: PartialEq, V> OrderedMap<K, V> {
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: std::borrow::Borrow<Q>,
        Q: PartialEq,
    {
        self.entries.iter().find(|(x, _)| x.borrow() == k).map(|(_, v)| v)
    }
    pub fn get_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: std::borrow::Borrow<Q>,
        Q: PartialEq,
    {
        self.entries.iter_mut().find(|(x, _)| x.borrow() == k).map(|(_, v)| v)
    }
    pub fn contains<Q: ?Sized>(&self, k: &Q) -> bool
    where
        K: std::borrow::Borrow<Q>,
        Q: PartialEq,
    {
        self.get(k).is_some()
    }
    /// `dic[k] = v`: replaces in place or appends.
    pub fn insert(&mut self, k: K, v: V) {
        match self.entries.iter_mut().find(|(x, _)| *x == k) {
            Some(e) => e.1 = v,
            None => self.entries.push((k, v)),
        }
    }
    pub fn remove<Q: ?Sized>(&mut self, k: &Q) -> Option<V>
    where
        K: std::borrow::Borrow<Q>,
        Q: PartialEq,
    {
        let i = self.entries.iter().position(|(x, _)| x.borrow() == k)?;
        Some(self.entries.remove(i).1)
    }
    pub fn clear(&mut self) {
        self.entries.clear();
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.entries.iter_mut().map(|(_, v)| v)
    }
}

/// `HashSet<T>` with insertion order.
pub type OrderedSet<T> = OrderedMap<T, ()>;

impl<T: PartialEq> OrderedMap<T, ()> {
    /// `HashSet.Add`: true when newly added.
    pub fn add(&mut self, v: T) -> bool {
        if self.contains(&v) {
            return false;
        }
        self.entries.push((v, ()));
        true
    }
    pub fn keys(&self) -> impl Iterator<Item = &T> {
        self.entries.iter().map(|(k, _)| k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_use_seven_bit_lengths() {
        let mut w = Writer::default();
        let long = "あ".repeat(50);
        w.str("CM3D2_PPARAM");
        w.str(&long);
        w.i32(-5);
        assert_eq!(w.bytes[0], 12);
        assert_eq!(&w.bytes[13..15], &[150, 1]);
        let mut r = Reader::new(&w.bytes);
        r.expect_str("CM3D2_PPARAM").unwrap();
        assert_eq!(r.str().unwrap(), long);
        assert_eq!(r.i32().unwrap(), -5);
        assert!(r.i32().is_err());
    }

    #[test]
    fn ordered_map_keeps_insertion_order() {
        let mut m = OrderedMap::default();
        m.insert("b", 1);
        m.insert("a", 2);
        m.insert("b", 3);
        assert_eq!(m.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>(), vec![("b", 3), ("a", 2)]);
        let mut s = OrderedSet::default();
        assert!(s.add(4));
        assert!(!s.add(4));
    }
}

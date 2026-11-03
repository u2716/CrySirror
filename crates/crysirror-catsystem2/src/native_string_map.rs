// SPDX-License-Identifier: MPL-2.0
//! The script-owned string-to-integer maps behind KCS natives 515-518, 885-902.
//! Handles are opaque VM dwords; a std::map's byte order is kept for iteration.
use anyhow::{ensure, Context, Result};
use std::collections::BTreeMap;

const FIRST_HANDLE: u32 = 0x4000_0001;
const MAX_MAPS: usize = 4096;

#[derive(Clone, Debug, Default)]
pub(crate) struct StringMaps {
    next: u32,
    maps: BTreeMap<u32, BTreeMap<Vec<u8>, u32>>,
}
fn padded(key: &[u8]) -> usize {
    (key.len() + 1 + 3) & !3
}
impl StringMaps {
    pub fn create(&mut self) -> Result<u32> {
        ensure!(self.maps.len() < MAX_MAPS, "CatSystem2 string map budget");
        let handle = self.next.max(FIRST_HANDLE);
        self.next = handle.checked_add(1).context("CatSystem2 string map handles exhausted")?;
        self.maps.insert(handle, BTreeMap::new());
        Ok(handle)
    }
    /// Every key and value of a map, in key order.
    pub fn entries(&self, handle: u32) -> Vec<(Vec<u8>, u32)> {
        self.maps.get(&handle).map_or(Vec::new(), |map| map.iter().map(|(key, value)| (key.clone(), *value)).collect())
    }
    pub fn destroy(&mut self, handle: u32) {
        self.maps.remove(&handle);
    }
    /// 659AA0 inserts without replacing an existing key.
    pub fn insert_new(&mut self, handle: u32, key: Vec<u8>, value: u32) {
        if let Some(map) = self.maps.get_mut(&handle) {
            map.entry(key).or_insert(value);
        }
    }
    /// 648790 replaces the value of an existing key.
    pub fn set(&mut self, handle: u32, key: Vec<u8>, value: u32) {
        if let Some(map) = self.maps.get_mut(&handle) {
            map.insert(key, value);
        }
    }
    /// 659C80 answers -1 for a missing key or map.
    pub fn get(&self, handle: u32, key: &[u8]) -> u32 {
        self.maps.get(&handle).and_then(|map| map.get(key)).copied().unwrap_or(u32::MAX)
    }
    pub fn clear(&mut self, handle: u32) {
        if let Some(map) = self.maps.get_mut(&handle) {
            map.clear();
        }
    }
    /// 648170: the key at an iteration index.
    pub fn key_at(&self, handle: u32, index: u32) -> Option<&[u8]> {
        self.maps.get(&handle)?.keys().nth(index as usize).map(Vec::as_slice)
    }
    /// 6480C0: the byte size `serialize` writes; 0 without a map.
    pub fn serialized_len(&self, handle: u32) -> u32 {
        self.maps.get(&handle).map_or(0, |map| {
            4 + map.keys().map(|key| padded(key) as u32 + 4).sum::<u32>()
        })
    }
    /// 647EF0: count, then each NUL-terminated key padded to 4 bytes and its value.
    pub fn serialize(&self, handle: u32) -> Option<Vec<u8>> {
        let map = self.maps.get(&handle)?;
        let mut out = (map.len() as u32).to_le_bytes().to_vec();
        for (key, value) in map {
            let at = out.len();
            out.extend_from_slice(key);
            out.resize(at + padded(key), 0);
            out.extend_from_slice(&value.to_le_bytes());
        }
        Some(out)
    }
    /// 648210: as 648620, keeping keys the map already has.
    pub fn deserialize_new(&mut self, handle: u32, bytes: &[u8]) -> Result<()> {
        let mut staged = StringMaps::default();
        let temporary = staged.create()?;
        staged.deserialize(temporary, bytes)?;
        for (key, value) in staged.entries(temporary) {
            self.insert_new(handle, key, value);
        }
        Ok(())
    }
    /// 648620: the inverse, adding each entry to the map.
    pub fn deserialize(&mut self, handle: u32, bytes: &[u8]) -> Result<()> {
        let count = u32::from_le_bytes(bytes.get(..4).context("CatSystem2 string map header")?.try_into()?);
        let mut at = 4;
        for _ in 0..count {
            let rest = bytes.get(at..).context("CatSystem2 string map entry")?;
            let end = rest.iter().position(|b| *b == 0).context("CatSystem2 string map key")?;
            let key = rest[..end].to_vec();
            at += padded(&key);
            let value = bytes.get(at..at + 4).context("CatSystem2 string map value")?;
            self.set(handle, key, u32::from_le_bytes(value.try_into()?));
            at += 4;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::StringMaps;
    #[test]
    fn maps_round_trip_in_key_order_and_report_the_native_miss() -> anyhow::Result<()> {
        let mut maps = StringMaps::default();
        let a = maps.create()?;
        maps.insert_new(a, b"zeta".to_vec(), 1);
        maps.insert_new(a, b"abc".to_vec(), 2);
        maps.insert_new(a, b"abc".to_vec(), 3);
        assert_eq!(maps.get(a, b"abc"), 2);
        maps.set(a, b"abc".to_vec(), 4);
        assert_eq!(maps.get(a, b"abc"), 4);
        assert_eq!(maps.get(a, b"none"), u32::MAX);
        assert_eq!(maps.key_at(a, 1), Some(&b"zeta"[..]));
        let bytes = maps.serialize(a).unwrap();
        assert_eq!(bytes.len() as u32, maps.serialized_len(a));
        assert_eq!(bytes[4..8], *b"abc\0");
        let b = maps.create()?;
        maps.deserialize(b, &bytes)?;
        assert_eq!(maps.serialize(b).unwrap(), bytes);
        maps.clear(a);
        assert_eq!(maps.serialized_len(a), 4);
        maps.destroy(a);
        assert_eq!(maps.serialized_len(a), 0);
        Ok(())
    }
}

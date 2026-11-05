// SPDX-License-Identifier: MPL-2.0
//! The gallery register, natives 419..425, 458, 459 (78FFB0 family): which image
//! variations were seen, as `name,variant,flag,flag…` keys, plus the group list file.
use super::Ctx;
use anyhow::{ensure, Context, Result};
use std::collections::{BTreeMap, BTreeSet};

/// Natives implemented by this family.
pub(crate) const IDS: &[u32] = &[419, 420, 421, 422, 423, 424, 425, 458, 459];

/// Saved as this adapter's own record (only this adapter reads it back).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct Register {
    /// 791EE0's map of nested flag trees, kept as its complete paths.
    seen: BTreeMap<Vec<u8>, BTreeSet<Vec<i32>>>,
    #[serde(skip)]
    list: Vec<u8>,
    /// 7902B0: string width, 64, or 256 with the "CGL" header.
    #[serde(skip)]
    stride: usize,
}

/// A group named by its ordinal or by list number and name.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Group {
    Number(u32),
    Name(u32, Vec<u8>),
}

#[derive(Default)]
pub(crate) struct State {
    registers: BTreeMap<u32, Register>,
    load: Option<(u32, crysirror_tasks::Task<Option<Vec<u8>>>)>,
}

/// 790760: `name,variant,flag…` → key `name_variant` and each flag's first byte. Only a
/// present first part counts.
fn parse_key(text: &[u8]) -> Option<(Vec<u8>, Vec<i32>)> {
    let mut parts = text.split(|b| *b == b',').filter(|part| !part.is_empty());
    let mut key = parts.next()?.to_vec();
    let mut flags = Vec::new();
    if let Some(variant) = parts.next() {
        key.push(b'_');
        key.extend_from_slice(variant);
        flags = parts.map(|flag| i32::from(flag[0] as i8)).collect();
    }
    Some((key, flags))
}

impl Register {
    /// 77AB10: the record saved under a key, then the list file.
    pub(crate) fn from_parts(record: Option<&[u8]>, list: Option<&[u8]>) -> Self {
        let mut register: Register = record.and_then(|bytes| serde_json::from_slice(bytes).ok()).unwrap_or_default();
        register.stride = 256;
        if let Some(list) = list {
            register.set_list(list);
        }
        register
    }
    /// Every record in file order: {list, name, strings}. 791BE0 numbers them across lists.
    fn records(&self) -> Vec<(u32, Vec<u8>, Vec<Vec<u8>>)> {
        let word = |at: usize| self.list.get(at..at + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
        let terminated = |slice: &[u8]| slice.iter().position(|b| *b == 0).map_or(slice.to_vec(), |end| slice[..end].to_vec());
        let mut out = Vec::new();
        let mut base = 0;
        while let (Some(number), Some(count)) = (word(base + 4), word(base + 8)) {
            let mut at = base + 12;
            for _ in 0..count {
                let (Some(size), Some(name), Some(strings)) = (word(at), self.list.get(at + 4..at + 68), word(at + 68)) else { return out };
                let items = (0..strings as usize).filter_map(|index| self.list.get(at + 72 + index * self.stride..at + 72 + (index + 1) * self.stride).map(terminated)).collect();
                out.push((number, terminated(name), items));
                if size == 0 {
                    break;
                }
                at += size as usize;
            }
            match word(base) {
                Some(next) if next != 0 => base += next as usize,
                _ => break,
            }
        }
        out
    }
    fn is_seen(&self, text: &[u8]) -> bool {
        parse_key(text).is_some_and(|(key, flags)| self.has(&key, &flags))
    }
    fn strings_of(&self, group: &Group) -> Vec<Vec<u8>> {
        let records = self.records();
        let found = match group {
            Group::Number(index) => records.into_iter().nth(*index as usize),
            Group::Name(list, name) => records.into_iter().find(|(number, record, _)| number == list && record.eq_ignore_ascii_case(name)),
        };
        found.map_or(Vec::new(), |(_, _, strings)| strings)
    }
    /// 77ADF0 → 790A70/790980: any string of the group seen.
    pub(crate) fn any_seen(&self, group: &Group) -> bool {
        self.strings_of(group).iter().any(|text| self.is_seen(text))
    }
    /// 77AF20 → 790FC0/790ED0: how many were seen.
    pub(crate) fn seen_count(&self, group: &Group) -> u32 {
        self.strings_of(group).iter().filter(|text| self.is_seen(text)).count() as u32
    }
    /// 77B050 → 790EB0: how many there are.
    pub(crate) fn total(&self, group: &Group) -> u32 {
        self.strings_of(group).len() as u32
    }
    /// 790D50/790C00: the n-th seen string.
    pub(crate) fn nth_seen(&self, group: &Group, n: u32) -> Option<Vec<u8>> {
        self.strings_of(group).into_iter().filter(|text| self.is_seen(text)).nth(n as usize)
    }
    /// 77B180 → 7910B0: a name's ordinal among the records, -1 when absent.
    pub(crate) fn ordinal(&self, list: u32, name: &[u8]) -> i32 {
        let _ = list;
        self.records().iter().position(|(_, record, _)| record.eq_ignore_ascii_case(name)).map_or(-1, |at| at as i32)
    }
    /// 77AD00: one `name,variant,flags` query.
    pub(crate) fn query(&self, text: &[u8]) -> bool {
        self.is_seen(text)
    }
    /// 7900B0.
    fn add(&mut self, key: Vec<u8>, flags: Vec<i32>) {
        self.seen.entry(key).or_default().insert(flags);
    }
    /// 790220 → 7915C0: the exact flag path under the key.
    fn has(&self, key: &[u8], flags: &[i32]) -> bool {
        self.seen.get(key).is_some_and(|paths| paths.contains(flags))
    }
    /// 7902B0: a list file replaces the previous one.
    fn set_list(&mut self, bytes: &[u8]) {
        if bytes.len() > 4 && bytes[..4] == [0x43, 0x47, 0x4c, 0] {
            self.list = bytes[4..].to_vec();
            self.stride = 256;
        } else {
            self.list = bytes.to_vec();
            self.stride = 64;
        }
    }
    /// 791B70: chained lists { next, list number, record count, records }; a record is
    /// { size, name[64], string count, strings }. Names compare without case.
    fn group(&self, list: u32, name: &[u8]) -> Option<Vec<Vec<u8>>> {
        let word = |at: usize| self.list.get(at..at + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
        let terminated = |slice: &[u8]| slice.iter().position(|b| *b == 0).map_or(slice.to_vec(), |end| slice[..end].to_vec());
        let mut base = 0;
        loop {
            if word(base + 4)? == list {
                let mut at = base + 12;
                for _ in 0..word(base + 8)? {
                    let size = word(at)? as usize;
                    if terminated(self.list.get(at + 4..at + 68)?).eq_ignore_ascii_case(name) {
                        let mut strings = Vec::new();
                        for index in 0..word(at + 68)? as usize {
                            let start = at + 72 + index * self.stride;
                            strings.push(terminated(self.list.get(start..start + self.stride)?));
                        }
                        return Some(strings);
                    }
                    if size == 0 {
                        break;
                    }
                    at += size;
                }
            }
            let next = word(base)? as usize;
            if next == 0 {
                return None;
            }
            base += next;
        }
    }
}

/// 652880: the key, and when it holds `_l` or `_L` after its first byte, the same without it.
fn without_large(key: &[u8]) -> Option<Vec<u8>> {
    let at = (1..key.len().saturating_sub(1)).find(|at| key[*at] == b'_' && key[*at + 1].eq_ignore_ascii_case(&b'l'))?;
    let mut out = key[..at].to_vec();
    out.extend_from_slice(&key[at + 2..]);
    Some(out)
}

pub(crate) fn handle(cx: &mut Ctx) -> Result<bool> {
    let a = cx.call.arguments.clone();
    match cx.call.id {
        // 652A20.
        419 => {
            cx.machine.preflight_native(true, &[])?;
            let id = cx.handles.allocate()?;
            cx.state.cg_register.registers.insert(id, Register { stride: 256, ..Default::default() });
            cx.machine.acknowledge(Some(id))?;
        }
        420 => {
            cx.machine.preflight_native(false, &[])?;
            cx.state.cg_register.registers.remove(&a[0]);
            cx.machine.acknowledge(None)?;
        }
        // 652A00 → 790210.
        421 => {
            cx.machine.preflight_native(false, &[])?;
            if let Some(register) = cx.state.cg_register.registers.get_mut(&a[0]) {
                register.seen.clear();
            }
            cx.machine.acknowledge(None)?;
        }
        422 => {
            ensure!(a.len() == 2, "CatSystem2 gallery add arguments");
            cx.machine.preflight_native(false, &[])?;
            let text = crate::native_values::format_bytes(cx.machine, a[1])?;
            if let (Some(register), Some((key, flags))) = (cx.state.cg_register.registers.get_mut(&a[0]), parse_key(&text)) {
                if let Some(small) = without_large(&key) {
                    register.add(small, flags.clone());
                }
                register.add(key, flags);
            }
            cx.machine.acknowledge(None)?;
        }
        423 => {
            ensure!(a.len() == 2, "CatSystem2 gallery query arguments");
            cx.machine.preflight_native(true, &[])?;
            let text = crate::native_values::format_bytes(cx.machine, a[1])?;
            let seen = match (cx.state.cg_register.registers.get(&a[0]), parse_key(&text)) {
                (Some(register), Some((key, flags))) => register.has(&key, &flags),
                _ => false,
            };
            cx.machine.acknowledge(Some(u32::from(seen)))?;
        }
        // 649B00 → 78B750 under key a1.
        424 => {
            ensure!(a.len() == 3, "CatSystem2 gallery save arguments");
            cx.machine.preflight_native(true, &[])?;
            let saved = match (a[0], cx.state.cg_register.registers.get(&a[2])) {
                (0, _) | (_, None) => 0,
                (container, Some(register)) => {
                    let bytes = serde_json::to_vec(register)?;
                    cx.saves.get_mut(&container).context("CatSystem2 gallery save container")?.append(a[1], &bytes)?;
                    1
                }
            };
            cx.machine.acknowledge(Some(saved))?;
        }
        // 648F50: a record replaces what the register held.
        425 => {
            ensure!(a.len() == 3, "CatSystem2 gallery load arguments");
            cx.machine.preflight_native(true, &[])?;
            let record = (a[0] != 0).then(|| cx.saves.get(&a[0]).and_then(|save| save.first(a[1])).map(<[u8]>::to_vec)).flatten();
            let loaded = match (record, cx.state.cg_register.registers.get_mut(&a[2])) {
                (Some(bytes), Some(register)) => {
                    let saved: Register = serde_json::from_slice(&bytes).context("CatSystem2 gallery record")?;
                    register.seen = saved.seen;
                    1
                }
                _ => 0,
            };
            cx.machine.acknowledge(Some(loaded))?;
        }
        // 652BE0: the list file; a missing one leaves the register's list as it was.
        458 => {
            ensure!(a.len() == 2, "CatSystem2 gallery list arguments");
            cx.machine.preflight_native(false, &[])?;
            if a[0] == 0 {
                cx.machine.acknowledge(None)?;
                return Ok(true);
            }
            if cx.state.cg_register.load.as_ref().is_none_or(|(pc, _)| *pc != cx.call.pc) {
                let name = String::from_utf8(crate::native_values::format_bytes(cx.machine, a[1])?).context("CatSystem2 gallery list name")?;
                let storage = cx.host.storage(cx.client)?;
                let permit = match cx.pool.context("CatSystem2 worker pool missing")?.reserve() {
                    Ok(permit) => permit,
                    Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => return Ok(false),
                    Err(e) => return Err(e),
                };
                let task = permit.spawn(move |_| {
                    let startup = crate::native_settings::optional_asset(storage.as_ref(), "config/startup.xml")?.context("CatSystem2 startup missing for gallery list")?;
                    let startup = crysirror_resource::xml::XmlDocument::parse_with_fallback(&startup, encoding_rs::SHIFT_JIS)?;
                    crate::native_settings::optional_native_asset(storage.as_ref(), &startup, &name)
                })?;
                cx.state.cg_register.load = Some((cx.call.pc, task));
                return Ok(false);
            }
            let Some(bytes) = cx.state.cg_register.load.as_mut().unwrap().1.try_take()? else { return Ok(false) };
            cx.state.cg_register.load = None;
            if let (Some(bytes), Some(register)) = (bytes, cx.state.cg_register.registers.get_mut(&a[0])) {
                register.set_list(&bytes);
            }
            cx.machine.acknowledge(None)?;
        }
        // 652B70 → 7906A0: every string of a named group becomes seen.
        459 => {
            ensure!(a.len() == 3, "CatSystem2 gallery group arguments");
            cx.machine.preflight_native(false, &[])?;
            let name = crate::native_values::format_bytes(cx.machine, a[2])?;
            if let Some(register) = cx.state.cg_register.registers.get_mut(&a[0]) {
                for text in register.group(a[1], &name).unwrap_or_default() {
                    if let Some((key, flags)) = parse_key(&text) {
                        register.add(key, flags);
                    }
                }
            }
            cx.machine.acknowledge(None)?;
        }
        id => anyhow::bail!("CatSystem2 gallery native {id} is not implemented"),
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_paths_and_groups() {
        assert_eq!(parse_key(b"CG3,01,a,bc"), Some((b"CG3_01".to_vec(), vec![97, 98])));
        assert_eq!(parse_key(b"CG3_01a"), Some((b"CG3_01a".to_vec(), vec![])));
        assert_eq!(without_large(b"ev_l01"), Some(b"ev01".to_vec()));
        assert_eq!(without_large(b"ev01"), None);
        let mut register = Register::default();
        register.add(b"k".to_vec(), vec![1, 2]);
        assert!(register.has(b"k", &[1, 2]));
        assert!(!register.has(b"k", &[1]));
        let mut list = vec![0u8; 12];
        list[8] = 1;
        let mut record = vec![0u8; 72 + 64];
        record[..4].copy_from_slice(&(136u32).to_le_bytes());
        record[4..6].copy_from_slice(b"G1");
        record[68] = 1;
        record[72..77].copy_from_slice(b"CG_01");
        list.extend(record);
        register.set_list(&list);
        assert_eq!(register.group(0, b"g1"), Some(vec![b"CG_01".to_vec()]));
        assert_eq!(register.group(0, b"none"), None);
    }
}

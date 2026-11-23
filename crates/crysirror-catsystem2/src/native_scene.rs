// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! KCS scene natives 323..338: the CatScene reader object (72AEA0) and its name table
//! (72CCB0). Handles are logical integers, never pointers; CatScene decoding is
//! `crate::scene`, and the script itself interprets every line.
use crate::scene::Scene;
use anyhow::{ensure, Result};
use std::{collections::BTreeMap, sync::Arc};

/// One 72CCB0 list node: name[32] +8, value +0x28, 7349F0 list +0x2C, data +0x30/+0x34.
#[derive(Clone, Debug, Default)]
pub(crate) struct SceneEntry {
    pub name: Vec<u8>,
    pub value: u32,
    /// (key, number) → text of at most 62 bytes.
    pub attributes: Vec<(u32, u32, Vec<u8>)>,
    pub blob: Option<Vec<u8>>,
}
#[derive(Clone, Debug, Default)]
pub(crate) struct NativeScene {
    pub scene: Option<Arc<Scene>>,
    pub name: Vec<u8>,
    /// 72CCB0 first list: unique names keeping the largest value (72CCF0).
    pub names: Vec<SceneEntry>,
    /// 72CCB0 second list: the visit history (72CDB0, 72D080, 72CF40).
    pub history: Vec<SceneEntry>,
}
impl NativeScene {
    /// 72BC20: a negative input answers the input count, a negative line that input's
    /// line count, otherwise the line's kind. No loaded scene answers -1.
    pub fn query(&self, input: i32, line: i32) -> i32 {
        let Some(scene) = &self.scene else { return -1 };
        if input < 0 {
            return scene.input_ranges.len() as i32;
        }
        let Some(&(length, index)) = scene.input_ranges.get(input as usize) else { return -1 };
        if line < 0 {
            return length as i32;
        }
        if line as u32 >= length {
            return -1;
        }
        scene.lines.get((index + line as u32) as usize).map_or(-1, |l| i32::from(l.kind))
    }
    /// 72B660: the line's bytes without its two-byte prefix.
    pub fn text(&self, input: i32, line: i32) -> Option<Vec<u8>> {
        let scene = self.scene.as_ref()?;
        let &(length, index) = scene.input_ranges.get(usize::try_from(input).ok()?)?;
        let line = u32::try_from(line).ok().filter(|l| *l < length)?;
        scene.lines.get((index + line) as usize).map(|l| l.bytes.clone())
    }
    /// 72BD20: lines of kind 0x20 over every input.
    pub fn count_kind_20(&self) -> u32 {
        self.scene.as_ref().map_or(0, |scene| scene.lines.iter().filter(|line| line.kind == 0x20).count() as u32)
    }
    pub fn insert(&mut self, name: &[u8], value: u32) {
        if let Some(entry) = self.names.iter_mut().find(|e| e.name.eq_ignore_ascii_case(name)) {
            entry.value = entry.value.max(value);
        } else {
            self.names.push(SceneEntry { name: name.to_vec(), value, ..Default::default() });
        }
    }
    pub fn value(&self, name: &[u8]) -> u32 {
        self.names.iter().find(|e| e.name.eq_ignore_ascii_case(name)).map_or(0, |e| e.value)
    }
    /// 72CDB0: the same name as the last entry keeps the larger value, any other
    /// is appended. Answers the last index.
    pub fn visit(&mut self, name: &[u8], value: u32) -> u32 {
        match self.history.last_mut().filter(|e| e.name.eq_ignore_ascii_case(name)) {
            Some(entry) => entry.value = entry.value.max(value),
            None => self.history.push(SceneEntry { name: name.to_vec(), value, ..Default::default() }),
        }
        self.history.len() as u32 - 1
    }
    /// 72D7C0 → 734AD0: attach `text` (cut to 62 bytes) under (key, number) to
    /// the history entry at `index`, clamped to the last; an empty list does nothing.
    pub fn set_attribute(&mut self, index: u32, key: u32, number: u32, text: &[u8]) {
        if self.history.is_empty() {
            return;
        }
        let index = (index as usize).min(self.history.len() - 1);
        let text = text[..text.len().min(62)].to_vec();
        let list = &mut self.history[index].attributes;
        match list.iter_mut().find(|(k, n, _)| *k == key && *n == number) {
            Some(entry) => entry.2 = text,
            None => list.push((key, number, text)),
        }
    }
    /// 72D8F0: copy an existing name's value to an absent alias.
    pub fn alias(&mut self, existing: &[u8], alias: &[u8]) {
        let source = self.names.iter().find(|e| e.name.eq_ignore_ascii_case(existing)).map(|e| e.value);
        if let (Some(value), false) = (source, self.names.iter().any(|e| e.name.eq_ignore_ascii_case(alias))) {
            self.names.push(SceneEntry { name: alias.to_vec(), value, ..Default::default() });
        }
    }
}
/// 72D120: word 0, then per entry name[32], value, attribute count, data size,
/// data offset (from the entry, 0 when none), 72-byte attributes and the data.
pub(crate) fn serialize_entries(list: &[SceneEntry], with_blob: bool) -> Vec<u8> {
    let mut out = 0u32.to_le_bytes().to_vec();
    for entry in list {
        let mut name = [0u8; 32];
        let used = entry.name.len().min(31);
        name[..used].copy_from_slice(&entry.name[..used]);
        out.extend(name);
        let blob = entry.blob.as_deref().filter(|_| with_blob).unwrap_or(&[]);
        let offset = if blob.is_empty() { 0 } else { 0x30 + 0x48 * entry.attributes.len() as u32 };
        for word in [entry.value, entry.attributes.len() as u32, blob.len() as u32, offset] {
            out.extend(word.to_le_bytes());
        }
        for (key, number, text) in &entry.attributes {
            out.extend(key.to_le_bytes());
            out.extend(number.to_le_bytes());
            let mut field = [0u8; 64];
            field[..text.len().min(62)].copy_from_slice(&text[..text.len().min(62)]);
            out.extend(field);
        }
        out.extend(blob);
    }
    out
}
/// 72D320: a first word of 0 is the form above; anything else is the older
/// form without that word, with 0x28-byte headers and no data.
pub(crate) fn parse_entries(bytes: &[u8]) -> Result<Vec<SceneEntry>> {
    use anyhow::Context;
    let word = |at: usize| -> Result<u32> { Ok(u32::from_le_bytes(bytes.get(at..at + 4).context("CatSystem2 scene list record")?.try_into().unwrap())) };
    let current = word(0)? == 0;
    let (mut at, header) = if current { (4, 0x30) } else { (0, 0x28) };
    let mut list = Vec::new();
    while at < bytes.len() {
        let raw = bytes.get(at..at + 32).context("CatSystem2 scene list name")?;
        let name = raw[..raw.iter().position(|b| *b == 0).unwrap_or(32)].to_vec();
        let (value, count) = (word(at + 0x20)?, word(at + 0x24)? as usize);
        let (size, offset) = if current { (word(at + 0x28)? as usize, word(at + 0x2C)? as usize) } else { (0, 0) };
        ensure!(count <= 65536, "CatSystem2 scene list attribute count");
        let mut attributes = Vec::with_capacity(count);
        for i in 0..count {
            let base = at + header + i * 0x48;
            let raw = bytes.get(base + 8..base + 0x48).context("CatSystem2 scene list attribute")?;
            attributes.push((word(base)?, word(base + 4)?, raw[..raw.iter().position(|b| *b == 0).unwrap_or(64)].to_vec()));
        }
        let blob = (size > 0).then(|| bytes.get(at + offset..at + offset + size).map(<[u8]>::to_vec).context("CatSystem2 scene list data")).transpose()?;
        list.push(SceneEntry { name, value, attributes, blob });
        at += header + count * 0x48 + size;
    }
    Ok(list)
}
/// 863: fixed-element stack (element size, growth), pushed by 865 and popped by 866.
#[derive(Clone, Debug, Default)]
pub(crate) struct NativeStack {
    pub element: usize,
    pub bytes: Vec<u8>,
}
impl NativeStack {
    pub fn count(&self) -> usize {
        if self.element == 0 { 0 } else { self.bytes.len() / self.element }
    }
}
#[derive(Clone, Debug, Default)]
pub(crate) struct Handles {
    next: u32,
    pub scenes: BTreeMap<u32, NativeScene>,
    pub stacks: BTreeMap<u32, NativeStack>,
    /// 294: KCS-owned motion sets, the same machinery FES objects use.
    pub motions: BTreeMap<u32, crate::motion_set::MotionSet>,
    pub motion_programs: BTreeMap<Vec<u8>, Arc<crate::kcs::Program>>,
    /// 710: images decoded for KCS planes.
    #[cfg(feature = "runtime")]
    pub images: BTreeMap<u32, crate::native_image_resource::Prepared>,
    #[cfg(feature = "runtime")]
    pub uploads: BTreeMap<u32, crysirror_core::service::UploadedImage>,
}
impl Handles {
    pub fn allocate(&mut self) -> Result<u32> {
        self.next = self.next.max(0x4100_0000).checked_add(1).unwrap_or(0);
        ensure!(self.next != 0 && self.scenes.len() + self.stacks.len() + self.motions.len() < 65536, "CatSystem2 native handle budget");
        Ok(self.next)
    }
}
/// `name` without its extension, as 6A6A10(name, 0) trims it.
pub(crate) fn stem(name: &[u8]) -> &[u8] {
    match name.iter().rposition(|b| *b == b'.') {
        Some(at) => &name[..at],
        None => name,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_keep_the_largest_value_and_aliases_do_not_replace() {
        let mut s = NativeScene::default();
        s.insert(b"A", 2);
        s.insert(b"a", 1);
        assert_eq!(s.value(b"A"), 2);
        s.alias(b"A", b"B");
        s.insert(b"C", 9);
        s.alias(b"C", b"B");
        assert_eq!(s.value(b"B"), 2);
        assert_eq!(stem(b"x.cst"), b"x");
    }
    #[test]
    fn an_empty_reader_answers_minus_one() {
        assert_eq!(NativeScene::default().query(0, 0), -1);
    }
}

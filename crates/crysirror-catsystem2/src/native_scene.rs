// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! KCS scene natives 323..338: the CatScene reader object (72AEA0) and its name table
//! (72CCB0). Handles are logical integers, never pointers; CatScene decoding is
//! `crate::scene`, and the script itself interprets every line.
use crate::scene::Scene;
use anyhow::{ensure, Result};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Debug, Default)]
pub(crate) struct NativeScene {
    pub scene: Option<Arc<Scene>>,
    pub name: Vec<u8>,
    /// 72CCF0 keeps insertion order and the largest value seen for each name.
    pub names: Vec<(Vec<u8>, u32)>,
    /// Entry +0x2C (7349F0 list): (key, number) → text of at most 62 bytes.
    pub attributes: BTreeMap<usize, Vec<(u32, u32, Vec<u8>)>>,
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
    pub fn insert(&mut self, name: &[u8], value: u32) {
        if let Some(entry) = self.names.iter_mut().find(|(n, _)| n.eq_ignore_ascii_case(name)) {
            entry.1 = entry.1.max(value);
        } else {
            self.names.push((name.to_vec(), value));
        }
    }
    pub fn value(&self, name: &[u8]) -> u32 {
        self.names.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map_or(0, |(_, v)| *v)
    }
    /// 72D7C0 → 734AD0: attach `text` (cut to 62 bytes) under (key, number) to
    /// the entry at `index`, clamped to the last entry; an empty table does nothing.
    pub fn set_attribute(&mut self, index: u32, key: u32, number: u32, text: &[u8]) {
        if self.names.is_empty() {
            return;
        }
        let index = (index as usize).min(self.names.len() - 1);
        let text = text[..text.len().min(62)].to_vec();
        let list = self.attributes.entry(index).or_default();
        match list.iter_mut().find(|(k, n, _)| *k == key && *n == number) {
            Some(entry) => entry.2 = text,
            None => list.push((key, number, text)),
        }
    }
    /// 72D8F0: copy an existing name's value to an absent alias.
    pub fn alias(&mut self, existing: &[u8], alias: &[u8]) {
        let source = self.names.iter().find(|(n, _)| n.eq_ignore_ascii_case(existing)).map(|(_, v)| *v);
        if let (Some(value), false) = (source, self.names.iter().any(|(n, _)| n.eq_ignore_ascii_case(alias))) {
            self.names.push((alias.to_vec(), value));
        }
    }
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

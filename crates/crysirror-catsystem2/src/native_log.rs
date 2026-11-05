// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! The script's message history (72DF30 family, natives 696..700, 785, 850, 851, 876, 977):
//! rows the backlog screen (LOGSTRING) reads. Saved as the adapter's own record, not the
//! original's layout, because only this adapter reads it back.
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// Native 698 has seven arguments; 977 adds an explicit eighth history index.
// The adapter supplies -1 for 698 and expands bounded KCS strings before this
// module filters controls and inserts a row. CP932 trail bytes are not escapes.
// The original preserve-layout global is not currently bound; the retained
// optional mode must not be presented as an inferred native configuration.

/// 72FC20/72FD60: prepare message-history text without interpreting glyph layout.
/// CP932 trail bytes are never mistaken for ASCII control markers. The optional
/// layout-preserving mode drops only page/read markers, retaining layout commands.
pub(crate) fn message_bytes(source: &[u8], preserve_layout: bool, limit: usize) -> Result<Vec<u8>> {
    ensure!(source.len() <= limit && !source.contains(&0), "CatSystem2 message log byte budget/NUL");
    let mut spans = Vec::new();
    let mut at = 0;
    while at < source.len() {
        if matches!(source[at], 0x81..=0x9f | 0xe0..=0xfc) {
            ensure!(at + 1 < source.len(), "CatSystem2 message log truncated CP932 character");
            at += 2;
            continue;
        }
        if source[at] != b'\\' { at += 1; continue; }
        let tail = &source[at..];
        let command = tail.get(1).copied().unwrap_or(0);
        let count = if preserve_layout {
            match command {
                b'@' => 2,
                b'R' | b'r' => if tail.get(2) == Some(&b'\\') && matches!(tail.get(3), Some(b'n' | b'N')) { 4 } else { 2 },
                _ => 0,
            }
        } else {
            let number = |start: usize| {
                let n = tail.get(start..).and_then(crate::native_values::integer_prefix).map_or(0, |(n, _)| n);
                n + usize::from(n != 0 && tail.get(start + n) == Some(&b';'))
            };
            match command {
                b'@' | b'n' | b'N' => 2,
                b'c' | b'C' => {
                    let start = if matches!(tail.get(2), Some(b'R' | b'S' | b'r' | b's')) { 3 } else { 2 };
                    start + number(start)
                }
                b'f' | b'F' => match tail.get(2) {
                    Some(b'+' | b'-' | b'R' | b'r') => 3 + number(3),
                    Some(b'L' | b'l' | b'S' | b's') => {
                        let start = if tail.get(3).is_some_and(|value| value.eq_ignore_ascii_case(&tail[2])) { 4 } else { 3 };
                        start + usize::from(tail.get(start) == Some(&b';'))
                    }
                    Some(b'N' | b'n') => if tail.get(4) == Some(&b';') { 4 } else { 3 },
                    _ => 2 + number(2),
                },
                b'p' | b'P' => if matches!(tail.get(2), Some(b'C' | b'L' | b'R' | b'c' | b'l' | b'r')) { 3 } else { 2 },
                b'w' | b'W' | b'x' | b'X' | b'y' | b'Y' => 2 + number(2),
                b'[' | b'\\' => { at += tail.len().min(2); continue; }
                _ => 1,
            }
        }.min(tail.len());
        if count == 0 { at += 1; }
        else { spans.push((at..at + count, b"".as_slice())); at += count; }
    }
    crysirror_text::byte_replace::replace_spans_bytes(source, &spans, limit, &|| Ok(()))
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct Row {
    /// Scene and line that produced it (72E8E0's first words).
    pub scene: String,
    pub line: u32,
    /// Script text: raw script bytes, whichever encoding the game writes.
    pub name: Vec<u8>,
    pub text: Vec<u8>,
    pub voice: String,
    /// Native row bit 4 records that this message owns a voice entry.
    #[serde(default)]
    pub flags: u32,
    /// A novel page delimiter precedes this row (731160's -1 string entry).
    #[serde(default = "starts_page")]
    pub page_start: bool,
    /// Native visit-history index, or -1 for a live message.
    #[serde(default = "live_history_index")]
    pub history_index: i32,
}
fn live_history_index() -> i32 { -1 }
fn starts_page() -> bool { true }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Log {
    pub rows: Vec<Row>,
    pub limit: usize,
    #[serde(default)]
    pub preserve_layout: bool,
}
#[derive(Clone, Debug, Default)]
pub(crate) struct Logs {
    next: u32,
    logs: BTreeMap<u32, Log>,
    message_calls: u64,
    message_inserts: u64,
}
const FIRST: u32 = 0x4c00_0001;
impl Logs {
    pub fn create(&mut self) -> u32 {
        let id = self.next.max(FIRST);
        self.next = id + 1;
        self.logs.insert(id, Log { rows: Vec::new(), limit: 100, preserve_layout: false });
        id
    }
    pub fn contains(&self, id: u32) -> bool { self.logs.contains_key(&id) }
    /// Counts only; no dialogue, speaker, scene name or resource path is exposed.
    pub fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({
            "handles": self.logs.len(),
            "rows": self.logs.values().map(|log| log.rows.len()).sum::<usize>(),
            "message_calls": self.message_calls,
            "message_inserts": self.message_inserts,
        })
    }
    /// Native 698/977 share the same message-history insertion path.
    pub fn add_message(&mut self, id: u32, mut row: Row) -> Result<()> {
        let preserve_layout = self.logs.get(&id).context("CatSystem2 message log missing")?.preserve_layout;
        row.name = message_bytes(&row.name, preserve_layout, 255)?;
        row.text = message_bytes(&row.text, preserve_layout, 4095)?;
        row.flags = u32::from(!row.text.is_empty()) | u32::from(!row.name.is_empty()) * 2 | (row.flags & 4);
        let duplicate = self.logs.get(&id).and_then(|log| log.rows.last()).is_some_and(|last| last.scene == row.scene && last.line == row.line);
        self.add(id, row);
        self.message_calls = self.message_calls.saturating_add(1);
        if !duplicate { self.message_inserts = self.message_inserts.saturating_add(1); }
        Ok(())
    }
    pub fn destroy(&mut self, id: u32) {
        self.logs.remove(&id);
    }
    pub fn clear(&mut self, id: u32) {
        if let Some(log) = self.logs.get_mut(&id) {
            log.rows.clear();
        }
    }
    pub fn set_limit(&mut self, id: u32, limit: u32) {
        if let Some(log) = self.logs.get_mut(&id) {
            log.limit = limit.max(1) as usize;
            let excess = log.rows.len().saturating_sub(log.limit);
            log.rows.drain(..excess);
        }
    }
    /// 72E8E0: a row; the same scene line again (a reread) replaces nothing.
    pub fn add(&mut self, id: u32, row: Row) {
        let Some(log) = self.logs.get_mut(&id) else { return };
        if log.rows.last().is_some_and(|last| last.scene == row.scene && last.line == row.line) {
            return;
        }
        log.rows.push(row);
        let excess = log.rows.len().saturating_sub(log.limit);
        log.rows.drain(..excess);
    }
    /// 72EFB0: this[33], the row limit; absent logs answer none.
    pub fn limit(&self, id: u32) -> Option<usize> {
        self.logs.get(&id).map(|log| log.limit)
    }
    pub fn rows(&self, id: u32) -> &[Row] {
        self.logs.get(&id).map_or(&[], |log| log.rows.as_slice())
    }
    pub fn serialize(&self, id: u32) -> Result<Vec<u8>> {
        serde_json::to_vec(self.logs.get(&id).context("CatSystem2 message log missing")?).map_err(Into::into)
    }
    pub fn restore(&mut self, id: u32, bytes: &[u8]) -> Result<()> {
        if self.logs.contains_key(&id) {
            self.logs.insert(id, serde_json::from_slice(bytes)?);
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rows_keep_their_limit_and_survive_a_save() -> Result<()> {
        let mut logs = Logs::default();
        let id = logs.create();
        logs.set_limit(id, 2);
        for line in 0..3 {
            logs.add(id, Row { scene: "a".into(), line, text: line.to_string().into_bytes(), ..Default::default() });
        }
        logs.add(id, Row { scene: "a".into(), line: 2, text: b"2".to_vec(), ..Default::default() });
        assert_eq!(logs.rows(id).iter().map(|r| r.line).collect::<Vec<_>>(), [1, 2]);
        let saved = logs.serialize(id)?;
        logs.clear(id);
        logs.restore(id, &saved)?;
        assert_eq!(logs.rows(id)[1].text, b"2");
        Ok(())
    }
}

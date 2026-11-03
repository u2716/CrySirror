// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! The script's message history (72DF30 family, natives 696..700, 785, 850, 851, 876, 977):
//! rows the backlog screen (LOGSTRING) reads. Saved as the adapter's own record, not the
//! original's layout, because only this adapter reads it back.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct Row {
    /// Scene and line that produced it (72E8E0's first words).
    pub scene: String,
    pub line: u32,
    /// Script text: raw script bytes, whichever encoding the game writes.
    pub name: Vec<u8>,
    pub text: Vec<u8>,
    pub voice: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Log {
    pub rows: Vec<Row>,
    pub limit: usize,
}
#[derive(Clone, Debug, Default)]
pub(crate) struct Logs {
    next: u32,
    logs: BTreeMap<u32, Log>,
}
const FIRST: u32 = 0x4c00_0001;
impl Logs {
    pub fn create(&mut self) -> u32 {
        let id = self.next.max(FIRST);
        self.next = id + 1;
        self.logs.insert(id, Log { rows: Vec::new(), limit: 100 });
        id
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
        if log.rows.last().is_some_and(|last| last.scene == row.scene && last.line == row.line && last.text == row.text) {
            return;
        }
        log.rows.push(row);
        let excess = log.rows.len().saturating_sub(log.limit);
        log.rows.drain(..excess);
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

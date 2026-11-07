// Copyright (c)2026 CrySirror contributors. MPL-2.0.
//! Bounded, client-owned byte records for histories, journals and event logs.
//! Native tag meanings, line boundaries and file layouts belong to consumers.
use anyhow::{ensure, Result};
use crysirror_core::state::ClientId;
use std::{collections::VecDeque, sync::Arc};
/// An immutable payload with a consumer-defined portable tag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ByteRecord {
    pub tag: i32,
    pub bytes: Arc<[u8]>,
}
/// Per-sequence retained record and payload limits, before any mutation.
#[derive(Clone, Copy, Debug)]
pub struct RecordSequenceLimits {
    pub records: usize,
    pub bytes: usize,
}
impl Default for RecordSequenceLimits {
    fn default() -> Self {
        Self {
            records: 1_000_000,
            bytes: 64 * 1024 * 1024,
        }
    }
}
/// Clone shares immutable payloads; callers schedule large metadata snapshots
/// and bulk replacement on workers. Push/pop themselves never scan history.
#[derive(Clone, Debug)]
pub struct RecordSequence {
    owner: ClientId,
    limits: RecordSequenceLimits,
    records: VecDeque<ByteRecord>,
    bytes: usize,
}
impl RecordSequence {
    pub fn new(owner: ClientId, limits: RecordSequenceLimits) -> Result<Self> {
        ensure!(
            limits.records <= 1_000_000 && limits.bytes <= 64 * 1024 * 1024,
            "record sequence maximum limits"
        );
        Ok(Self {
            owner,
            limits,
            records: VecDeque::new(),
            bytes: 0,
        })
    }
    pub fn client(&self) -> ClientId {
        self.owner
    }
    fn require(&self, owner: ClientId) -> Result<()> {
        ensure!(owner == self.owner, "foreign record sequence");
        Ok(())
    }
    pub fn len(&self, owner: ClientId) -> Result<usize> {
        self.require(owner)?;
        Ok(self.records.len())
    }
    pub fn byte_len(&self, owner: ClientId) -> Result<usize> {
        self.require(owner)?;
        Ok(self.bytes)
    }
    pub fn get(&self, owner: ClientId, index: usize) -> Result<Option<&ByteRecord>> {
        self.require(owner)?;
        Ok(self.records.get(index))
    }
    pub fn records(&self, owner: ClientId) -> Result<&VecDeque<ByteRecord>> {
        self.require(owner)?;
        Ok(&self.records)
    }
    pub fn push(&mut self, owner: ClientId, record: ByteRecord) -> Result<()> {
        self.require(owner)?;
        ensure!(
            self.records.len() < self.limits.records,
            "record sequence count budget"
        );
        let bytes = self
            .bytes
            .checked_add(record.bytes.len())
            .filter(|n| *n <= self.limits.bytes)
            .ok_or_else(|| anyhow::anyhow!("record sequence payload budget"))?;
        self.records.push_back(record);
        self.bytes = bytes;
        Ok(())
    }
    pub fn pop_front(&mut self, owner: ClientId) -> Result<Option<ByteRecord>> {
        self.require(owner)?;
        let record = self.records.pop_front();
        if let Some(record) = &record {
            self.bytes -= record.bytes.len();
        }
        Ok(record)
    }
    pub fn clear(&mut self, owner: ClientId) -> Result<()> {
        self.require(owner)?;
        self.records.clear();
        self.bytes = 0;
        Ok(())
    }
    /// Validate every record before replacing the old sequence. Failure retains
    /// the exact previous records and ownership; no partial import is visible.
    pub fn replace(
        &mut self,
        owner: ClientId,
        records: VecDeque<ByteRecord>,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<()> {
        self.require(owner)?;
        ensure!(
            records.len() <= self.limits.records,
            "record sequence import count budget"
        );
        let mut bytes = 0usize;
        for record in &records {
            check()?;
            bytes = bytes
                .checked_add(record.bytes.len())
                .filter(|n| *n <= self.limits.bytes)
                .ok_or_else(|| anyhow::anyhow!("record sequence import payload budget"))?;
        }
        check()?;
        self.records = records;
        self.bytes = bytes;
        Ok(())
    }
}

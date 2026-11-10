// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CCommonChartMenu's automatic EXT actor. The original layout is selection
//! words followed by byte-valued outgoing-road indices; it is not an empty EXT.
//! Legacy constructor 0x522f40 confirms blockSize=1000, aligned road bytes at
//! this+295 words and aligned selection-word count at this+296 words. The
//! 0x51ca90/0x51ca00 callbacks copy selections first, then roads; they do not
//! serialize the newer source's UI/editor fields.
use crate::config::Names;
use anyhow::{Context, Result, ensure};
mod model;
mod checkpoint;
pub(crate) use checkpoint::ChartCheckpoint;
pub(crate) use model::{ChartFunctionEffect, ChartRuntime, ChartWrite, ChartSnapshotRequest};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ChartLayout {
    pub slots: usize,
    pub selection_words: usize,
    pub road_bytes: usize,
}
impl ChartLayout {
    pub(crate) fn read(table: &Names) -> Result<Self> {
        let charts = usize::try_from(table.integer(b"chartListNumber")?.unwrap_or(1))?;
        let block = usize::try_from(table.integer(b"chartBlockSize")?.unwrap_or(1000))?;
        let slots = charts
            .checked_mul(block)
            .context("NNN ChartMenu slot extent overflow")?;
        ensure!(
            charts > 0 && charts <= 256 && block > 0 && slots <= 1_048_576,
            "NNN ChartMenu slot budget"
        );
        Ok(Self {
            slots,
            selection_words: (slots.div_ceil(32) + 3) & !3,
            road_bytes: (slots + 15) & !15,
        })
    }
    pub(crate) fn bytes(self) -> usize {
        self.selection_words * 4 + self.road_bytes
    }
}
#[derive(Clone, Debug)]
pub(crate) struct ChartState {
    pub layout: ChartLayout,
    selection: Vec<u32>,
    roads: Vec<u8>,
}
impl ChartState {
    pub fn fresh(layout: ChartLayout) -> Self {
        Self {
            layout,
            selection: vec![u32::MAX; layout.selection_words],
            roads: vec![0; layout.road_bytes],
        }
    }
    pub fn load(&mut self, bytes: &[u8]) -> Result<()> {
        ensure!(
            bytes.len() == self.layout.bytes(),
            "NNN ChartMenu automatic EXT size mismatch"
        );
        let (selection, roads) = bytes.split_at(self.layout.selection_words * 4);
        self.selection = selection
            .chunks_exact(4)
            .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
            .collect();
        self.roads.copy_from_slice(roads);
        Ok(())
    }
    pub fn store(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.layout.bytes());
        for word in &self.selection {
            bytes.extend(word.to_le_bytes());
        }
        bytes.extend(&self.roads);
        bytes
    }
    pub fn reset_roads(&mut self) {
        self.roads.fill(0);
    }
    pub fn set_selection(&mut self, slot: i32, enabled: bool) -> Result<()> {
        let slot = usize::try_from(slot)?;
        ensure!(
            slot < self.layout.slots,
            "NNN ChartMenu selection slot bounds"
        );
        let mask = 1u32 << (slot & 31);
        self.selection[slot / 32] &= !mask;
        if enabled {
            self.selection[slot / 32] |= mask;
        }
        Ok(())
    }
    pub fn selection(&self, slot: usize) -> Result<bool> {
        ensure!(
            slot < self.layout.slots,
            "NNN ChartMenu selection slot bounds"
        );
        Ok(self.selection[slot / 32] & (1 << (slot & 31)) != 0)
    }
    pub fn set_road(&mut self, slot: usize, outgoing: i32) -> Result<()> {
        ensure!(slot < self.layout.slots, "NNN ChartMenu road slot bounds");
        self.roads[slot] = outgoing.wrapping_add(1) as u8;
        Ok(())
    }
    pub fn road(&self, slot: usize) -> Result<u8> {
        self.roads.get(slot).copied().filter(|_|slot<self.layout.slots).context("NNN ChartMenu road slot bounds")
    }
}

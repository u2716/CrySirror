// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Retained source-seek completion, independent of a voice's later EOF/removal.
//! Mutable DSP stays with the mixer owner; this mailbox contains only bounded
//! completion data. Consumers observe it through the mixer's publication fence.
use anyhow::{Result, ensure};
use std::sync::{Arc, Mutex};
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceSeekState {
    Pending,
    Ready { frame: u64 },
    Failed(String),
    Cancelled,
}
#[derive(Clone, Debug)]
pub struct SourceSeekReceipt {
    pub(crate) mixer: u64,
    pub(crate) owner: u32,
    pub(crate) sequence: u64,
    signal: Arc<Mutex<SourceSeekState>>,
}
pub(crate) struct SeekSender {
    signal: Arc<Mutex<SourceSeekState>>,
}
impl SourceSeekReceipt {
    pub(crate) fn pair(mixer: u64, owner: u32, sequence: u64) -> (Self, SeekSender) {
        let signal = Arc::new(Mutex::new(SourceSeekState::Pending));
        (
            Self {
                mixer,
                owner,
                sequence,
                signal: signal.clone(),
            },
            SeekSender { signal },
        )
    }
    // Only the owner may reject a result whose worker frame was never published.
    // A late worker cannot overwrite this terminal rejection.
    pub(crate) fn reject_unpublished(&self) {
        if let Ok(mut state) = self.signal.lock() {
            *state = SourceSeekState::Cancelled;
        }
    }
    pub(crate) fn read(&self, mixer: u64, owner: u32) -> Result<SourceSeekState> {
        ensure!(
            self.mixer == mixer && self.owner == owner,
            "foreign source-seek receipt"
        );
        self.signal
            .lock()
            .map(|s| s.clone())
            .map_err(|_| anyhow::anyhow!("source-seek receipt poisoned"))
    }
}
impl SeekSender {
    pub(crate) fn complete(&self, state: SourceSeekState) {
        if let Ok(mut current) = self.signal.lock() {
            if matches!(*current, SourceSeekState::Pending) {
                *current = state;
            }
        }
    }
}
impl Drop for SeekSender {
    fn drop(&mut self) {
        self.complete(SourceSeekState::Cancelled);
    }
}

// SPDX-License-Identifier: MPL-2.0
//! Device-free playlist clock. Frame resources and presentation remain with
//! the caller; pause/stop, first-tick admission and terminal policy are explicit.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrameSequenceClock {
    pub frames: u32,
    pub interval_ms: u64,
    pub looping: bool,
    /// Finish when entering the last frame, or after displaying its interval.
    pub finish_on_last_entry: bool,
    pub index: u32,
    pub complete: bool,
    remainder_ms: u64,
    initialized: bool,
    stopped: bool,
}
impl FrameSequenceClock {
    pub fn new(
        frames: u32,
        interval_ms: u64,
        looping: bool,
        finish_on_last_entry: bool,
    ) -> Result<Self> {
        ensure!(
            (1..=4096).contains(&frames) && (1..=86_400_000).contains(&interval_ms),
            "frame sequence bounds"
        );
        Ok(Self {
            frames,
            interval_ms,
            looping,
            finish_on_last_entry,
            index: 0,
            complete: false,
            remainder_ms: 0,
            initialized: false,
            stopped: false,
        })
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (1..=4096).contains(&self.frames)
                && (1..=86_400_000).contains(&self.interval_ms)
                && self.index < self.frames
                && self.remainder_ms < self.interval_ms,
            "frame sequence state bounds"
        );
        Ok(())
    }
    pub fn advance(&mut self, delta_ms: u64) -> Result<()> {
        self.validate()?;
        if self.complete {
            return Ok(());
        }
        if !self.initialized {
            self.initialized = true;
            return Ok(());
        }
        let elapsed = u128::from(self.remainder_ms) + u128::from(delta_ms);
        let steps = elapsed / u128::from(self.interval_ms);
        self.remainder_ms = (elapsed % u128::from(self.interval_ms)) as u64;
        if self.looping {
            self.index = ((u128::from(self.index) + steps) % u128::from(self.frames)) as u32;
        } else {
            let remaining = if self.finish_on_last_entry {
                (self.frames - 1 - self.index).max(1)
            } else {
                self.frames - self.index
            };
            if steps >= u128::from(remaining) {
                self.index = self.frames - 1;
                self.complete = true;
                self.remainder_ms = 0;
            } else {
                self.index += steps as u32;
            }
        }
        Ok(())
    }
    pub fn stop(&mut self) {
        if !self.complete {
            self.complete = true;
            self.stopped = true;
            self.remainder_ms = 0;
        }
    }
    pub fn resume(&mut self) {
        if self.stopped {
            self.complete = false;
            self.stopped = false;
            self.initialized = false;
        }
    }
}

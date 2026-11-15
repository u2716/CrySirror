// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original CGameCallBack voiceSpeakFlag/count: measured duration followed by
//! FrameControl's millisecond countdown, independent of mixer playing state.
use anyhow::{Context, Result};
#[derive(Clone, Debug, Default)]
pub(crate) struct VoiceSpeakClock {
    flag: bool,
    remaining_ms: u64,
    last_control_ms: Option<u64>,
}
impl VoiceSpeakClock {
    pub(crate) fn started(&mut self, length_ms: u32) {
        self.flag = true;
        self.remaining_ms = u64::from(length_ms);
    }
    pub(crate) fn observe_control(&mut self, now_ms: u64) -> Result<()> {
        let elapsed = match self.last_control_ms {
            Some(previous) => now_ms
                .checked_sub(previous)
                .context("NNN callback speaking clock moved backwards")?,
            None => 0,
        };
        self.last_control_ms = Some(now_ms);
        if self.flag {
            self.remaining_ms = self.remaining_ms.saturating_sub(elapsed);
            if self.remaining_ms == 0 {
                self.flag = false;
            }
        }
        Ok(())
    }
    pub(crate) fn speaking(&self) -> bool {
        self.flag
    }
}
// Starting a voice does not restart FrameControl's global elapsed counter.
// Muted successful measurements retain their native receipt policy in Audio;
// stop/skip without a new voice receipt does not arbitrarily clear this flag.

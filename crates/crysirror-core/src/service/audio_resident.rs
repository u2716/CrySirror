// SPDX-License-Identifier: MPL-2.0

use super::PcmAudio;
use crate::{state::ClientId, Error, Result};
use alloc::boxed::Box;

#[derive(Clone, Debug)]
pub enum ResidentAudioState {
    Pending,
    Ready(PcmAudio),
    Failed(Error),
}

/// Client-scoped completion for PCM preparation. The returned PCM and all its
/// playback clones retain the underlying memory reservation. Drop cancels an
/// unfinished job.
pub trait ResidentAudioProcess: Send {
    fn client(&self) -> ClientId;
    fn poll(&mut self) -> Result<ResidentAudioState>;
}

pub type ResidentAudioJob = Box<dyn ResidentAudioProcess>;

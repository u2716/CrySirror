// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Detached audio replacement preparation. Preparing never stops live channels;
//! only a host-owned original-restore transaction may install the ready value.
use crate::{state::ClientId, Error, Result};
use alloc::{boxed::Box, sync::Arc};
use core::any::Any;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioReplacementInfo {
    pub client: ClientId,
    pub channels: usize,
}
/// Opaque one-shot prepared sources. No decoder/DSP pointer or serialized
/// project data is exposed. Implementations must check actual host provenance.
pub trait AudioReplacementResource: Any + Send + Sync + core::fmt::Debug {
    fn client(&self) -> ClientId;
    fn info(&self) -> AudioReplacementInfo;
    fn as_any(&self) -> &dyn Any;
}
#[derive(Clone, Debug)]
pub enum AudioReplacementState {
    Pending,
    Ready(Arc<dyn AudioReplacementResource>),
    Failed(Error),
}
/// Completion is retained. Drop cancels unfinished preparation and releases its
/// resource budget; it never cancels the client's existing playback/seeks.
pub trait AudioReplacementProcess: Send {
    fn client(&self) -> ClientId;
    fn poll(&mut self) -> Result<AudioReplacementState>;
}
pub type AudioReplacementJob = Box<dyn AudioReplacementProcess>;

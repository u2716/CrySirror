// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original-save preparation is distinct from framework snapshots. Jobs own
//! detached native state; only the runtime commits a fully validated plan.
use crate::{
    service::{AudioReplacementResource, AudioRequest, HostServices},
    state::{ClientId, Command},
    vm::VmAdapter,
    Error, Result,
};
use alloc::{boxed::Box, sync::Arc, vec::Vec};

/// Runtime-instance, client and epoch scoped handle. A released/stale handle
/// never becomes authority for a later preparation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OriginalRestoreTicket {
    pub owner: u64,
    pub client: ClientId,
    pub epoch: u64,
    pub serial: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OriginalRestoreStatus {
    Preparing,
    Ready,
    Failed(Error),
    Committed,
    Cancelled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalRestoreProgress {
    Pending,
    Ready,
}
/// Versioned shared scene batch, scoped by the ticket rather than embedded
/// native client IDs. It is validated against a detached engine state.
#[derive(Clone, Debug)]
pub struct OriginalRestoreBatch {
    pub schema: u32,
    pub bytes: Arc<[u8]>,
}
/// Detached replacement plus declarative engine effects. No live VM is moved
/// into a preparation. Sequential audio commands stay unsupported; ready audio
/// uses one engine-owned replacement rather than partially executing requests.
pub struct OriginalRestoreCommit {
    pub vm: Box<dyn VmAdapter>,
    pub commands: Vec<Command>,
    pub engine_batches: Vec<OriginalRestoreBatch>,
    pub audio: Vec<AudioRequest>,
    /// One prepared engine-owned replacement, including an empty set which
    /// explicitly clears this client's voices. It cannot be installed by a VM.
    pub prepared_audio: Option<Arc<dyn AudioReplacementResource>>,
    /// End old caller leases atomically; new scene references retain them.
    pub release_images: Vec<alloc::string::String>,
}
/// poll receives a restricted resource host: scoped read-only storage, shared
/// image preparation and reserved immutable publications. Playback, storage
/// writes, input/scene mutation and clock advancement are forbidden. finish
/// must only assemble already-ready state, without I/O or another VM step.
/// Dropping a job must release/cancel its admitted worker dependencies.
pub trait OriginalRestorePreparation {
    fn client(&self) -> ClientId;
    fn poll(&mut self, resources: &mut dyn HostServices) -> Result<OriginalRestoreProgress>;
    fn finish(self: Box<Self>) -> Result<OriginalRestoreCommit>;
}

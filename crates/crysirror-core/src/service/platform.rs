// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Optional distribution/platform integrations, separate from the operating
//! system, entitlement and gameplay data. No backend means unavailable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformIntegration {
    Steam,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlatformIntegrationInfo {
    pub client: crate::state::ClientId,
    pub integration: PlatformIntegration,
    pub available: bool,
    /// None when no integration backend is installed in this host. This does
    /// not describe a failed SDK call, user entitlement or installation state.
    pub backend: Option<&'static str>,
}
pub trait PlatformIntegrationProvider: Send + Sync {
    /// Read a previously prepared host capability. This must not initialize an
    /// SDK, perform I/O or wait on a worker. SDK ownership/preparation stays in
    /// the provider; available means its integration is ready for use.
    fn info(&self, client: crate::state::ClientId, integration: PlatformIntegration)
        -> crate::Result<PlatformIntegrationInfo>;
}

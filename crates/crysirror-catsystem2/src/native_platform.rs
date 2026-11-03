// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original B9BD9C creation policy. Host SDK ownership is a shared capability;
//! this cache preserves5AF7E0's initialize-once behavior and785920's null branch.
use anyhow::{Result, ensure};
use crysirror_core::{
    service::{HostServices, PlatformIntegration},
    state::ClientId,
};
#[derive(Clone, Debug, Default)]
pub(crate) struct SteamManager {
    available: Option<bool>,
}
impl SteamManager {
    pub fn initialize(&mut self, host: &dyn HostServices, client: ClientId) -> Result<()> {
        if self.available.is_some() {
            return Ok(());
        }
        let info = host.platform_integration(client, PlatformIntegration::Steam)?;
        ensure!(
            info.client == client
                && info.integration == PlatformIntegration::Steam
                && (!info.available || info.backend.is_some()),
            "CatSystem2 invalid platform integration receipt"
        );
        self.available = Some(info.available);
        Ok(())
    }
    pub fn enabled(&self) -> bool {
        self.available.unwrap_or(false)
    }
    pub fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"created":self.available.is_some(),"available":self.available})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_core::{
        path::VfsPath,
        service::{FileSystem, PlatformIntegrationInfo},
        state::Command,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Host {
        calls: AtomicUsize,
        ready: bool,
        foreign: bool,
    }
    impl FileSystem for Host {
        fn read(&mut self, _: ClientId, _: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
            Err(crysirror_core::Error::NotFound)
        }
        fn write_atomic(
            &mut self,
            _: ClientId,
            _: &VfsPath,
            _: &[u8],
        ) -> crysirror_core::Result<()> {
            Err(crysirror_core::Error::ReadOnly)
        }
    }
    impl HostServices for Host {
        fn now_ms(&self) -> u64 {
            0
        }
        fn submit(&mut self, _: ClientId, _: &[Command]) -> crysirror_core::Result<()> {
            Err(crysirror_core::Error::Unsupported(
                "authored legacy commands",
            ))
        }
        fn platform_integration(
            &self,
            client: ClientId,
            integration: PlatformIntegration,
        ) -> crysirror_core::Result<PlatformIntegrationInfo> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(PlatformIntegrationInfo {
                client: if self.foreign { ClientId(999) } else { client },
                integration,
                available: self.ready,
                backend: self.ready.then_some("authored-ready-provider"),
            })
        }
    }
    #[test]
    fn original_null_query_does_not_initialize_and_initialization_latches_real_host_state() {
        let mut manager = SteamManager::default();
        let mut host = Host {
            calls: AtomicUsize::new(0),
            ready: true,
            foreign: false,
        };
        assert!(!manager.enabled());
        assert_eq!(host.calls.load(Ordering::SeqCst), 0);
        manager.initialize(&host, ClientId(1)).unwrap();
        assert!(manager.enabled());
        host.ready = false;
        manager.initialize(&host, ClientId(1)).unwrap();
        assert!(manager.enabled());
        assert_eq!(host.calls.load(Ordering::SeqCst), 1);
        let mut manager = SteamManager::default();
        host.foreign = true;
        assert!(manager.initialize(&host, ClientId(1)).is_err());
        assert_eq!(manager.diagnostics()["created"], false);
        host.foreign = false;
        manager.initialize(&host, ClientId(1)).unwrap();
        assert_eq!(
            manager.diagnostics(),
            serde_json::json!({"created":true,"available":false})
        );
    }
}

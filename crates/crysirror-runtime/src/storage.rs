// SPDX-License-Identifier: MPL-2.0
//! Host-supplied capability roots. Native paths never enter portable hosts.
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{ClientStorage, FileSystem},
    state::ClientId,
    vfs::VfsProvider,
    Error, Result,
};
use std::{collections::BTreeMap, sync::Arc};

/// Runtime filesystem router. Each injected handle already has one client's
/// authority; the Runtime never derives a root from an ambient browser path.
pub struct RuntimeStorage {
    #[cfg(not(target_arch = "wasm32"))]
    disk: Option<crate::vfs::DiskVfs>,
    clients: BTreeMap<ClientId, Arc<dyn ClientStorage>>,
    redirected: std::sync::RwLock<BTreeMap<ClientId, Arc<crysirror_resource::redirect::RedirectStorage>>>,
}
impl RuntimeStorage {
    pub fn from_capabilities(clients: BTreeMap<ClientId, Arc<dyn ClientStorage>>) -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            disk: None,
            clients,
            redirected: Default::default(),
        }
    }
    pub fn storage(&self, client: ClientId) -> Result<Arc<dyn ClientStorage>> {
        if let Some(storage)=self.redirected.read().map_err(|_|Error::Io("client storage lock".into()))?.get(&client){return Ok(storage.clone());}
        let source=self.base_storage(client)?;
        let mut storage=self.redirected.write().map_err(|_|Error::Io("client storage lock".into()))?;
        Ok(storage.entry(client).or_insert_with(||Arc::new(crysirror_resource::redirect::RedirectStorage::new(source))).clone())
    }
    fn base_storage(&self, client: ClientId) -> Result<Arc<dyn ClientStorage>> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(disk) = &self.disk {
            return disk.storage(client);
        }
        self.clients.get(&client).cloned().ok_or(Error::NotFound)
    }
    pub fn mount_provider(
        &mut self,
        client: ClientId,
        point: &str,
        provider: Arc<dyn VfsProvider>,
    ) -> Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(disk) = &mut self.disk {
            return disk.mount_provider(client, point, provider);
        }
        // Provider-aware injected roots own their container routing. Reject
        // before mutation rather than acknowledging an uninstalled archive.
        let _ = (self.storage(client)?, point, provider);
        Err(Error::Unsupported("mounting providers on injected storage"))
    }
    pub fn mount_fallback_provider(
        &mut self,
        client: ClientId,
        point: &str,
        provider: Arc<dyn VfsProvider>,
    ) -> Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(disk) = &mut self.disk {
            return disk.mount_fallback_provider(client, point, provider);
        }
        let _ = (self.storage(client)?, point, provider);
        Err(Error::Unsupported(
            "mounting fallback providers on injected storage",
        ))
    }
    pub fn provider_mounts(&self, client: ClientId) -> Vec<(String, &'static str)> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(disk) = &self.disk {
            return disk.provider_mounts(client);
        }
        Vec::new()
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn desktop_roots(&self, client: ClientId) -> Result<crate::vfs::DesktopStorageRoots> {
        self.disk
            .as_ref()
            .ok_or(Error::Unsupported("desktop paths for injected storage"))?
            .desktop_roots(client)
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open_asset_file(
        &self,
        client: ClientId,
        path: &VfsPath,
    ) -> Result<Option<std::fs::File>> {
        let _ = self.storage(client)?;
        let path = self.redirected.read().map_err(|_|Error::Io("client storage lock".into()))?
            .get(&client).ok_or(Error::NotFound)?.resolve(path)?;
        match &self.disk {
            Some(disk) => disk.open_asset_file(client, &path),
            None => Ok(None),
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl From<crate::vfs::DiskVfs> for RuntimeStorage {
    fn from(disk: crate::vfs::DiskVfs) -> Self {
        Self {
            disk: Some(disk),
            clients: BTreeMap::new(),
            redirected: Default::default(),
        }
    }
}
impl FileSystem for RuntimeStorage {
    fn read(&mut self, client: ClientId, path: &VfsPath) -> Result<Vec<u8>> {
        self.storage(client)?.read(path)
    }
    fn write_atomic(&mut self, client: ClientId, path: &VfsPath, data: &[u8]) -> Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(disk) = &mut self.disk {
            return disk.write_atomic(client, path, data);
        }
        self.storage(client)?.write_atomic(path, data)
    }
    fn remove(&mut self, client: ClientId, path: &VfsPath) -> Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(disk) = &mut self.disk {
            return disk.remove(client, path);
        }
        self.storage(client)?.remove(path)
    }
}

/// Read-only asset view under one capability root. Logical prefixes are
/// validated with the same core path rules used by native storage.
pub(crate) struct AssetPrefix {
    pub source: Arc<dyn ClientStorage>,
    pub prefix: String,
}
impl AssetPrefix {
    fn path(&self, path: &VfsPath) -> Result<VfsPath> {
        if path.mount() != Mount::Asset {
            return Err(Error::ReadOnly);
        }
        VfsPath::new(
            Mount::Asset,
            &if self.prefix.is_empty() {
                path.relative().into()
            } else {
                format!("{}/{}", self.prefix, path.relative())
            },
        )
    }
}
impl ClientStorage for AssetPrefix {
    fn read(&self, path: &VfsPath) -> Result<Vec<u8>> {
        self.source.read(&self.path(path)?)
    }
    fn open_asset(&self, path: &VfsPath) -> Result<Arc<dyn crysirror_core::service::AssetReader>> {
        self.source.open_asset(&self.path(path)?)
    }
    fn open_read(&self, path: &VfsPath) -> Result<Arc<dyn crysirror_core::service::AssetReader>> {
        self.open_asset(path)
    }
    fn write_atomic(&self, _: &VfsPath, _: &[u8]) -> Result<()> {
        Err(Error::ReadOnly)
    }
    fn remove(&self, _: &VfsPath) -> Result<()> {
        Err(Error::ReadOnly)
    }
    fn list(&self, mount: Mount, dir: &str) -> Result<Vec<String>> {
        if mount != Mount::Asset {
            return Err(Error::ReadOnly);
        }
        let path = if dir.is_empty() {
            self.prefix.clone()
        } else {
            self.path(&VfsPath::new(mount, dir)?)?.relative().into()
        };
        self.source.list(mount, &path)
    }
}

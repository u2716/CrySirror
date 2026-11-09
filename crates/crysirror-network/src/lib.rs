//! Capability-scoped network services. SMB asset reads and explicitly granted
//! HTTP operations run on bounded workers, independently of native VMs.
#![forbid(unsafe_code)]
#[cfg(feature = "smb")]
mod smb;
#[cfg(feature = "smb")]
pub use smb::SmbRoot;
#[cfg(feature = "nfs")]
mod nfs;
#[cfg(feature = "nfs")]
pub use nfs::NfsRoot;
#[cfg(feature = "http")]
pub mod http;

use crysirror_core::{service::AssetReader, vfs::VfsProvider, Result};
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub struct Metadata {
    pub directory: bool,
    pub len: u64,
}

/// Metadata is needed by engine detection in addition to ordinary asset reads.
/// All names are canonical, slash-separated, relative to the selected share.
pub trait RemoteStorage: VfsProvider {
    fn metadata(&self, relative: &str) -> Result<Metadata>;
    /// Credential-free origin; suitable for diagnostics and local save identity.
    fn origin(&self) -> &str;
}

/// Restrict a provider to a selected game/client directory within its share.
pub struct Subdirectory {
    source: Arc<dyn RemoteStorage>,
    prefix: String,
}
impl Subdirectory {
    pub fn new(source: Arc<dyn RemoteStorage>, prefix: &str) -> Result<Self> {
        if !prefix.is_empty() {
            crysirror_core::path::VfsPath::new(crysirror_core::path::Mount::Asset, prefix)?;
        }
        Ok(Self {
            source,
            prefix: prefix.into(),
        })
    }
    fn path(&self, relative: &str) -> Result<String> {
        if !relative.is_empty() {
            crysirror_core::path::VfsPath::new(crysirror_core::path::Mount::Asset, relative)?;
        }
        Ok(match (self.prefix.is_empty(), relative.is_empty()) {
            (true, _) => relative.into(),
            (_, true) => self.prefix.clone(),
            _ => format!("{}/{relative}", self.prefix),
        })
    }
}
impl VfsProvider for Subdirectory {
    fn kind(&self) -> &'static str {
        "network-directory"
    }
    fn read(&self, relative: &str) -> Result<Vec<u8>> {
        self.source.read(&self.path(relative)?)
    }
    fn open(&self, relative: &str) -> Result<Arc<dyn AssetReader>> {
        self.source.open(&self.path(relative)?)
    }
    fn exists(&self, relative: &str) -> bool {
        self.path(relative).is_ok_and(|p| self.source.exists(&p))
    }
    fn list_dir(&self, relative: &str) -> Result<Vec<String>> {
        self.source.list_dir(&self.path(relative)?)
    }
}

/// Read-only project asset view. Save/cache authority stays with the host.
impl crysirror_core::service::ClientStorage for Subdirectory {
    fn read(&self, path: &crysirror_core::path::VfsPath) -> Result<Vec<u8>> {
        if path.mount() != crysirror_core::path::Mount::Asset {
            return Err(crysirror_core::Error::ReadOnly);
        }
        VfsProvider::read(self, path.relative())
    }
    fn open_asset(&self, path: &crysirror_core::path::VfsPath) -> Result<Arc<dyn AssetReader>> {
        if path.mount() != crysirror_core::path::Mount::Asset {
            return Err(crysirror_core::Error::ReadOnly);
        }
        VfsProvider::open(self, path.relative())
    }
    fn open_read(&self, path: &crysirror_core::path::VfsPath) -> Result<Arc<dyn AssetReader>> {
        crysirror_core::service::ClientStorage::open_asset(self, path)
    }
    fn list(&self, mount: crysirror_core::path::Mount, dir: &str) -> Result<Vec<String>> {
        if mount != crysirror_core::path::Mount::Asset {
            return Err(crysirror_core::Error::ReadOnly);
        }
        VfsProvider::list_dir(self, dir)
    }
    fn exists(&self, path: &crysirror_core::path::VfsPath) -> bool {
        path.mount() == crysirror_core::path::Mount::Asset
            && VfsProvider::exists(self, path.relative())
    }
    fn write_atomic(&self, _: &crysirror_core::path::VfsPath, _: &[u8]) -> Result<()> {
        Err(crysirror_core::Error::ReadOnly)
    }
    fn remove(&self, _: &crysirror_core::path::VfsPath) -> Result<()> {
        Err(crysirror_core::Error::ReadOnly)
    }
}

mod native_io;
pub use native_io::{Directory, File};
#[path="native_fs.rs"]
pub mod fs;

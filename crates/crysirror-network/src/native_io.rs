//! Scoped native-runtime file handles over the existing network service. The
//! compatibility paths are identities only; no remote game files are staged.
use crate::{Metadata, RemoteStorage};
use crysirror_core::service::AssetReader;
use std::{
    fmt,
    io::{self, Read, Seek, SeekFrom},
    sync::Arc,
};
fn io_error(error: crysirror_core::Error) -> io::Error {
    match error {
        crysirror_core::Error::NotFound => io::Error::from(io::ErrorKind::NotFound),
        e => io::Error::other(e.to_string()),
    }
}
fn validate(path: &str) -> io::Result<()> {
    if !path.is_empty()
        && (path.contains(['\\', '\0', ':'])
            || path.split('/').any(|p| matches!(p, "" | "." | "..")))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "network path must have canonical relative components",
        ));
    }
    Ok(())
}
#[derive(Clone)]
pub struct Directory {
    source: Arc<dyn RemoteStorage>,
    prefix: String,
}
impl fmt::Debug for Directory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeResourceDirectory")
            .field("origin", &self.source.origin())
            .field("prefix", &self.prefix)
            .finish()
    }
}
impl Directory {
    pub fn from_provider(source: Arc<dyn RemoteStorage>) -> anyhow::Result<Self> {
        if !source.metadata("")?.directory {
            anyhow::bail!("resource root is not a directory");
        }
        Ok(Self {
            source,
            prefix: String::new(),
        })
    }
    pub fn connect(url: &str) -> anyhow::Result<Self> {
        let (source, prefix): (Arc<dyn RemoteStorage>, String) =
            match url.split_once("://").map(|v| v.0) {
                #[cfg(feature = "nfs")]
                Some("nfs") => {
                    let (s, p) = crate::NfsRoot::connect(url)?;
                    (Arc::new(s), p)
                }
                #[cfg(feature = "smb")]
                Some("smb") => {
                    let (s, p) = crate::SmbRoot::connect(url)?;
                    (Arc::new(s), p)
                }
                _ => anyhow::bail!("enable the requested nfs or smb feature"),
            };
        validate(&prefix)?;
        if !source.metadata(&prefix)?.directory {
            anyhow::bail!("network root is not a directory");
        }
        Ok(Self { source, prefix })
    }
    fn path(&self, p: &str) -> io::Result<String> {
        validate(p)?;
        Ok(match (self.prefix.is_empty(), p.is_empty()) {
            (true, _) => p.into(),
            (_, true) => self.prefix.clone(),
            _ => format!("{}/{p}", self.prefix),
        })
    }
    pub fn metadata(&self, p: &str) -> io::Result<Metadata> {
        self.source.metadata(&self.path(p)?).map_err(io_error)
    }
    pub fn list(&self, p: &str) -> io::Result<Vec<String>> {
        self.source.list_dir(&self.path(p)?).map_err(io_error)
    }
    pub fn read(&self, p: &str) -> io::Result<Vec<u8>> {
        self.source.read(&self.path(p)?).map_err(io_error)
    }
    pub fn open(&self, p: &str) -> io::Result<File> {
        let source = self.source.open(&self.path(p)?).map_err(io_error)?;
        let len = source.len().map_err(io_error)?;
        Ok(File {
            source,
            position: 0,
            len,
        })
    }
}
#[derive(Clone)]
pub struct File {
    source: Arc<dyn AssetReader>,
    position: u64,
    len: u64,
}
impl fmt::Debug for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeResourceFile")
            .field("position", &self.position)
            .field("len", &self.len)
            .finish()
    }
}
impl File {
    pub fn len(&self) -> u64 {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn read_at(&self, offset: u64, b: &mut [u8]) -> io::Result<usize> {
        self.source.read_at(offset, b).map_err(io_error)
    }
}
impl Read for File {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        let n = self.read_at(self.position, b)?;
        self.position = self
            .position
            .checked_add(n as u64)
            .ok_or_else(|| io::Error::other("native network cursor overflow"))?;
        Ok(n)
    }
}
impl Seek for File {
    fn seek(&mut self, s: SeekFrom) -> io::Result<u64> {
        let n = match s {
            SeekFrom::Start(n) => i128::from(n),
            SeekFrom::End(n) => i128::from(self.len) + i128::from(n),
            SeekFrom::Current(n) => i128::from(self.position) + i128::from(n),
        };
        self.position = u64::try_from(n).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "native network cursor out of range",
            )
        })?;
        Ok(self.position)
    }
}

/// Client-scoped engine storage is already bound to its asset capability. This
/// adapter retains that authority and the existing network connection/cache.
struct ClientDirectory {
    storage: Arc<dyn crysirror_core::service::ClientStorage>,
}
impl crate::RemoteStorage for ClientDirectory {
    fn origin(&self) -> &str {
        "client-asset://"
    }
    fn metadata(&self, p: &str) -> crysirror_core::Result<Metadata> {
        use crysirror_core::{
            Error,
            path::{Mount, VfsPath},
        };
        if p.is_empty() {
            self.storage.list(Mount::Asset, "")?;
            return Ok(Metadata {
                directory: true,
                len: 0,
            });
        }
        VfsPath::new(Mount::Asset, p)?;
        match self.storage.metadata(Mount::Asset, p) {
            Ok(Some(m)) => {
                return Ok(Metadata {
                    directory: m.is_directory,
                    len: m.size.unwrap_or(0),
                });
            }
            Ok(None) | Err(Error::Unsupported(_)) => {}
            Err(e) => return Err(e),
        }
        match self.storage.open_asset(&VfsPath::new(Mount::Asset, p)?) {
            Ok(file) => Ok(Metadata {
                directory: false,
                len: file.len()?,
            }),
            Err(Error::NotFound) | Err(Error::Unsupported(_)) => {
                self.storage.list(Mount::Asset, p)?;
                Ok(Metadata {
                    directory: true,
                    len: 0,
                })
            }
            Err(e) => Err(e),
        }
    }
}
impl crysirror_core::vfs::VfsProvider for ClientDirectory {
    fn kind(&self) -> &'static str {
        "client-assets"
    }
    fn read(&self, p: &str) -> crysirror_core::Result<Vec<u8>> {
        self.storage.read(&crysirror_core::path::VfsPath::new(
            crysirror_core::path::Mount::Asset,
            p,
        )?)
    }
    fn open(&self, p: &str) -> crysirror_core::Result<Arc<dyn AssetReader>> {
        self.storage.open_asset(&crysirror_core::path::VfsPath::new(
            crysirror_core::path::Mount::Asset,
            p,
        )?)
    }
    fn exists(&self, p: &str) -> bool {
        crate::RemoteStorage::metadata(self, p).is_ok()
    }
    fn list_dir(&self, p: &str) -> crysirror_core::Result<Vec<String>> {
        self.storage.list(crysirror_core::path::Mount::Asset, p)
    }
}
impl Directory {
    pub fn from_client_storage(
        storage: Arc<dyn crysirror_core::service::ClientStorage>,
        prefix: &str,
    ) -> anyhow::Result<Self> {
        validate(prefix)?;
        let source: Arc<dyn RemoteStorage> = Arc::new(ClientDirectory { storage });
        if !source.metadata(prefix)?.directory {
            anyhow::bail!("native resource capability is not a directory");
        }
        Ok(Self {
            source,
            prefix: prefix.into(),
        })
    }
}

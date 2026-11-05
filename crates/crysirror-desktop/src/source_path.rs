//! Host launch/detection paths over a local directory or network capability.
//! This facade is not visible to VMs; runtime reads use the shared VFS directly.
use anyhow::{ensure, Context, Result};
use crysirror_core::{service::AssetReader, vfs::VfsProvider};
use crysirror_network::{RemoteStorage, SmbRoot, NfsRoot, Subdirectory};
use std::{
    borrow::Cow,
    fmt,
    io::{self, Read, Seek, SeekFrom},
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub(crate) struct SourcePath {
    path: PathBuf,
    remote: Option<Arc<dyn RemoteStorage>>,
}
impl AsRef<SourcePath> for SourcePath {
    fn as_ref(&self) -> &Self {
        self
    }
}
impl fmt::Display for SourcePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(remote) = &self.remote {
            write!(
                f,
                "{}{}",
                remote.origin(),
                self.path.to_string_lossy().replace('\\', "/")
            )
        } else {
            self.path.display().fmt(f)
        }
    }
}
impl fmt::Debug for SourcePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl SourcePath {
    pub fn local(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().into(),
            remote: None,
        }
    }
    pub fn from_input(path: &Path) -> Result<Self> {
        let text = path.to_string_lossy();
        if text.starts_with("smb://") {
            let (root, relative) = SmbRoot::connect(&text)?;
            return Ok(Self {
                path: relative.into(),
                remote: Some(Arc::new(root)),
            });
        }
        if text.starts_with("nfs://") {
            let (root,relative)=NfsRoot::connect(&text)?;
            return Ok(Self {path:relative.into(),remote:Some(Arc::new(root))});
        }
        ensure!(
            !text.contains("://"),
            "unsupported game source protocol; supported: local path, smb://, nfs://"
        );
        Ok(Self::local(path))
    }
    pub fn local_path(&self) -> Option<&Path> {
        self.remote.is_none().then_some(self.path.as_path())
    }
    pub fn is_remote(&self) -> bool {
        self.remote.is_some()
    }
    pub fn file_name(&self) -> Option<&std::ffi::OsStr> {
        self.path.file_name()
    }
    pub fn join(&self, path: impl AsRef<Path>) -> Self {
        Self {
            path: self.path.join(path),
            remote: self.remote.clone(),
        }
    }
    pub fn to_path_buf(&self) -> Self {
        self.clone()
    }
    pub fn parent(&self) -> Option<Self> {
        Some(Self {
            path: self.path.parent()?.into(),
            remote: self.remote.clone(),
        })
    }
    pub fn display(&self) -> &Self {
        self
    }
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        Cow::Owned(self.to_string())
    }
    pub fn relative(&self) -> Result<String> {
        let mut parts = Vec::new();
        for component in self.path.components() {
            match component {
                Component::CurDir => {}
                Component::Normal(part) => {
                    parts.push(part.to_str().context("network source name must be UTF-8")?)
                }
                _ => anyhow::bail!("network source escapes share capability"),
            }
        }
        let path = parts.join("/").replace('\\', "/");
        if !path.is_empty() {
            crysirror_core::path::VfsPath::new(crysirror_core::path::Mount::Asset, &path)?;
        }
        Ok(path)
    }
    pub fn canonicalize(&self) -> Result<Self> {
        if self.remote.is_none() {
            return Ok(Self::local(self.path.canonicalize()?));
        }
        let mut canonical = self.clone();
        canonical.path = self.relative()?.into();
        canonical.metadata()?;
        Ok(canonical)
    }
    pub fn starts_with(&self, other: impl AsRef<SourcePath>) -> bool {
        let other = other.as_ref();
        let same = match (&self.remote, &other.remote) {
            (None, None) => true,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            _ => false,
        };
        same && self.path.starts_with(&other.path)
    }
    pub fn strip_prefix(&self, other: impl AsRef<SourcePath>) -> Result<&Path> {
        ensure!(
            self.starts_with(other.as_ref()),
            "source path is outside selected directory"
        );
        Ok(self.path.strip_prefix(&other.as_ref().path)?)
    }
    pub fn metadata(&self) -> io::Result<Metadata> {
        if let Some(remote) = &self.remote {
            let path = self.relative().map_err(io::Error::other)?;
            let m = remote.metadata(&path).map_err(io_error)?;
            Ok(Metadata {
                file: !m.directory,
                directory: m.directory,
                len: m.len,
            })
        } else {
            let m = std::fs::metadata(&self.path)?;
            Ok(Metadata {
                file: m.is_file(),
                directory: m.is_dir(),
                len: m.len(),
            })
        }
    }
    pub fn is_file(&self) -> bool {
        self.metadata().is_ok_and(|m| m.is_file())
    }
    pub fn is_dir(&self) -> bool {
        self.metadata().is_ok_and(|m| m.is_dir())
    }
    pub fn exists(&self) -> bool {
        self.metadata().is_ok()
    }
    pub fn read(&self) -> io::Result<Vec<u8>> {
        let file = SourceFile::open(self)?;
        let limit = 64 * 1024 * 1024;
        if file.metadata()?.len() > limit {
            return Err(io::Error::other("detection whole-file budget"));
        }
        let mut bytes = Vec::new();
        file.take(limit + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > limit {
            return Err(io::Error::other("detection whole-file budget"));
        }
        Ok(bytes)
    }
    pub fn read_dir(&self) -> io::Result<std::vec::IntoIter<io::Result<SourceEntry>>> {
        let names = if let Some(remote) = &self.remote {
            remote
                .list_dir(&self.relative().map_err(io::Error::other)?)
                .map_err(io_error)?
                .into_iter()
                .map(std::ffi::OsString::from)
                .collect::<Vec<_>>()
        } else {
            std::fs::read_dir(&self.path)?
                .map(|entry| entry.map(|entry| entry.file_name()))
                .collect::<io::Result<Vec<_>>>()?
        };
        Ok(names
            .into_iter()
            .map(|name| {
                Ok(SourceEntry {
                    path: self.join(&name),
                    name,
                })
            })
            .collect::<Vec<_>>()
            .into_iter())
    }
    pub fn provider(&self) -> Result<Option<Arc<dyn VfsProvider>>> {
        self.remote
            .as_ref()
            .map(|remote| {
                Ok(
                    Arc::new(Subdirectory::new(remote.clone(), &self.relative()?)?)
                        as Arc<dyn VfsProvider>,
                )
            })
            .transpose()
    }
    pub fn storage(&self) -> Result<Arc<dyn crysirror_core::service::ClientStorage>> {
        let remote = self.remote.as_ref().context("network source missing")?;
        Ok(Arc::new(Subdirectory::new(
            remote.clone(),
            &self.relative()?,
        )?))
    }
}
fn io_error(error: crysirror_core::Error) -> io::Error {
    match error {
        crysirror_core::Error::NotFound => io::Error::from(io::ErrorKind::NotFound),
        other => io::Error::other(other.to_string()),
    }
}
#[derive(Clone, Copy)]
pub(crate) struct Metadata {
    file: bool,
    directory: bool,
    len: u64,
}
impl Metadata {
    pub fn is_file(&self) -> bool {
        self.file
    }
    pub fn is_dir(&self) -> bool {
        self.directory
    }
    pub fn len(&self) -> u64 {
        self.len
    }
}
pub(crate) struct SourceEntry {
    path: SourcePath,
    name: std::ffi::OsString,
}
impl SourceEntry {
    pub fn path(&self) -> SourcePath {
        self.path.clone()
    }
    pub fn file_name(&self) -> std::ffi::OsString {
        self.name.clone()
    }
    pub fn metadata(&self) -> io::Result<Metadata> {
        self.path.metadata()
    }
    pub fn file_type(&self) -> io::Result<Metadata> {
        self.metadata()
    }
}
pub(crate) struct SourceFile {
    source: Arc<dyn AssetReader>,
    position: u64,
}
impl SourceFile {
    pub fn open(path: impl AsRef<SourcePath>) -> io::Result<Self> {
        let path = path.as_ref();
        let source: Arc<dyn AssetReader> = if let Some(remote) = &path.remote {
            remote
                .open(&path.relative().map_err(io::Error::other)?)
                .map_err(io_error)?
        } else {
            Arc::new(LocalFile(Mutex::new(std::fs::File::open(&path.path)?)))
        };
        Ok(Self {
            source,
            position: 0,
        })
    }
    pub fn metadata(&self) -> io::Result<Metadata> {
        Ok(Metadata {
            file: true,
            directory: false,
            len: self.source.len().map_err(io_error)?,
        })
    }
}
impl Read for SourceFile {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let count = self.source.read_at(self.position, out).map_err(io_error)?;
        self.position += count as u64;
        Ok(count)
    }
}
impl Seek for SourceFile {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let at = match from {
            SeekFrom::Start(n) => i128::from(n),
            SeekFrom::Current(n) => i128::from(self.position) + i128::from(n),
            SeekFrom::End(n) => i128::from(self.source.len().map_err(io_error)?) + i128::from(n),
        };
        self.position =
            u64::try_from(at).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
        Ok(self.position)
    }
}
struct LocalFile(Mutex<std::fs::File>);
impl AssetReader for LocalFile {
    fn len(&self) -> crysirror_core::Result<u64> {
        self.0
            .lock()
            .map_err(|_| crysirror_core::Error::Invalid("detection file lock"))?
            .metadata()
            .map(|m| m.len())
            .map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
    fn read_at(&self, offset: u64, out: &mut [u8]) -> crysirror_core::Result<usize> {
        let mut file = self
            .0
            .lock()
            .map_err(|_| crysirror_core::Error::Invalid("detection file lock"))?;
        file.seek(SeekFrom::Start(offset))
            .and_then(|_| file.read(out))
            .map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
}

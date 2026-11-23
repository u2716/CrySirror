// SPDX-License-Identifier: MPL-2.0
//! The console backend (also compiled for host tests).

/// Marker matching `cap_std::AmbientAuthority`: only hosts mint roots.
#[derive(Clone, Copy, Debug)]
pub struct AmbientAuthority(());
pub fn ambient_authority() -> AmbientAuthority {
    AmbientAuthority(())
}

pub mod time {
    /// `cap_std::time::SystemTime` equivalent.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub struct SystemTime(std::time::SystemTime);
    impl SystemTime {
        pub fn from_std(t: std::time::SystemTime) -> Self {
            Self(t)
        }
        pub fn into_std(self) -> std::time::SystemTime {
            self.0
        }
    }
}

pub mod fs {
    use super::{AmbientAuthority, time::SystemTime};
    use std::{
        ffi::OsString,
        io::{self, Read, Seek, SeekFrom, Write},
        path::{Component, Path, PathBuf},
    };
    pub use std::fs::{FileType, Permissions};

    fn denied() -> io::Error {
        io::Error::new(io::ErrorKind::PermissionDenied, "path escapes capability root")
    }

    #[derive(Debug)]
    pub struct Dir {
        root: PathBuf,
    }
    impl Dir {
        pub fn open_ambient_dir(path: impl AsRef<Path>, _: AmbientAuthority) -> io::Result<Self> {
            let root = path.as_ref().to_path_buf();
            if !std::fs::metadata(&root)?.is_dir() {
                return Err(io::Error::new(io::ErrorKind::NotADirectory, "capability root is not a directory"));
            }
            Ok(Self { root })
        }
        /// Resolve a root-relative path; never leaves the root. A component
        /// longer than 255 bytes cannot exist on any supported filesystem
        /// (exFAT, FAT32); it is reported absent here because some hosts fail
        /// on the lookup itself (Vita3K raises an uncaught exception).
        fn resolve(&self, path: &Path) -> io::Result<PathBuf> {
            let mut out = self.root.clone();
            for component in path.components() {
                match component {
                    Component::Normal(name) if name.len() > 255 => {
                        return Err(io::Error::new(io::ErrorKind::NotFound, "path component longer than 255 bytes"));
                    }
                    Component::Normal(name) => out.push(name),
                    Component::CurDir => {}
                    Component::ParentDir | Component::RootDir | Component::Prefix(_) => return Err(denied()),
                }
            }
            Ok(out)
        }
        pub fn try_clone(&self) -> io::Result<Self> {
            Ok(Self { root: self.root.clone() })
        }
        pub fn open(&self, path: impl AsRef<Path>) -> io::Result<File> {
            File::open_read(&self.resolve(path.as_ref())?)
        }
        pub fn open_with(&self, path: impl AsRef<Path>, options: &OpenOptions) -> io::Result<File> {
            let path = self.resolve(path.as_ref())?;
            Ok(File::from_std(options.0.open(path)?))
        }
        pub fn open_dir(&self, path: impl AsRef<Path>) -> io::Result<Self> {
            let root = self.resolve(path.as_ref())?;
            if !std::fs::metadata(&root)?.is_dir() {
                return Err(io::Error::new(io::ErrorKind::NotADirectory, "not a directory"));
            }
            Ok(Self { root })
        }
        pub fn create_dir(&self, path: impl AsRef<Path>) -> io::Result<()> {
            std::fs::create_dir(self.resolve(path.as_ref())?)
        }
        pub fn create_dir_all(&self, path: impl AsRef<Path>) -> io::Result<()> {
            std::fs::create_dir_all(self.resolve(path.as_ref())?)
        }
        pub fn remove_file(&self, path: impl AsRef<Path>) -> io::Result<()> {
            std::fs::remove_file(self.resolve(path.as_ref())?)
        }
        pub fn remove_dir(&self, path: impl AsRef<Path>) -> io::Result<()> {
            std::fs::remove_dir(self.resolve(path.as_ref())?)
        }
        pub fn remove_dir_all(&self, path: impl AsRef<Path>) -> io::Result<()> {
            std::fs::remove_dir_all(self.resolve(path.as_ref())?)
        }
        pub fn rename(&self, from: impl AsRef<Path>, target: &Dir, to: impl AsRef<Path>) -> io::Result<()> {
            std::fs::rename(self.resolve(from.as_ref())?, target.resolve(to.as_ref())?)
        }
        pub fn copy(&self, from: impl AsRef<Path>, target: &Dir, to: impl AsRef<Path>) -> io::Result<u64> {
            std::fs::copy(self.resolve(from.as_ref())?, target.resolve(to.as_ref())?)
        }
        pub fn metadata(&self, path: impl AsRef<Path>) -> io::Result<Metadata> {
            let path = self.resolve(path.as_ref())?;
            Ok(Metadata::of(std::fs::metadata(&path)?, &path))
        }
        pub fn symlink_metadata(&self, path: impl AsRef<Path>) -> io::Result<Metadata> {
            let path = self.resolve(path.as_ref())?;
            Ok(Metadata::of(std::fs::symlink_metadata(&path)?, &path))
        }
        pub fn dir_metadata(&self) -> io::Result<Metadata> {
            Ok(Metadata(std::fs::metadata(&self.root)?, None))
        }
        pub fn exists(&self, path: impl AsRef<Path>) -> bool {
            self.resolve(path.as_ref()).is_ok_and(|p| p.exists())
        }
        pub fn is_dir(&self, path: impl AsRef<Path>) -> bool {
            self.resolve(path.as_ref()).is_ok_and(|p| p.is_dir())
        }
        pub fn is_file(&self, path: impl AsRef<Path>) -> bool {
            self.resolve(path.as_ref()).is_ok_and(|p| p.is_file())
        }
        pub fn set_permissions(&self, path: impl AsRef<Path>, permissions: Permissions) -> io::Result<()> {
            std::fs::set_permissions(self.resolve(path.as_ref())?, permissions)
        }
        pub fn read(&self, path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
            std::fs::read(self.resolve(path.as_ref())?)
        }
        pub fn write(&self, path: impl AsRef<Path>, bytes: impl AsRef<[u8]>) -> io::Result<()> {
            std::fs::write(self.resolve(path.as_ref())?, bytes)
        }
        pub fn create(&self, path: impl AsRef<Path>) -> io::Result<File> {
            Ok(File::from_std(std::fs::File::create(self.resolve(path.as_ref())?)?))
        }
        pub fn entries(&self) -> io::Result<ReadDir> {
            Ok(ReadDir { inner: std::fs::read_dir(&self.root)?, parent: self.root.clone() })
        }
        pub fn read_dir(&self, path: impl AsRef<Path>) -> io::Result<ReadDir> {
            let parent = self.resolve(path.as_ref())?;
            Ok(ReadDir { inner: std::fs::read_dir(&parent)?, parent })
        }
    }

    pub struct ReadDir {
        inner: std::fs::ReadDir,
        parent: PathBuf,
    }
    impl Iterator for ReadDir {
        type Item = io::Result<DirEntry>;
        fn next(&mut self) -> Option<Self::Item> {
            self.inner.next().map(|r| {
                r.map(|entry| DirEntry { name: entry.file_name(), parent: self.parent.clone(), entry })
            })
        }
    }
    pub struct DirEntry {
        entry: std::fs::DirEntry,
        parent: PathBuf,
        name: OsString,
    }
    impl DirEntry {
        pub fn file_name(&self) -> OsString {
            self.name.clone()
        }
        pub fn file_type(&self) -> io::Result<FileType> {
            self.entry.file_type()
        }
        pub fn metadata(&self) -> io::Result<Metadata> {
            Ok(Metadata::of(self.entry.metadata()?, &self.parent.join(&self.name)))
        }
        pub fn open(&self) -> io::Result<File> {
            File::open_read(&self.parent.join(&self.name))
        }
        pub fn open_dir(&self) -> io::Result<Dir> {
            let root = self.parent.join(&self.name);
            if !std::fs::metadata(&root)?.is_dir() {
                return Err(io::Error::new(io::ErrorKind::NotADirectory, "not a directory"));
            }
            Ok(Dir { root })
        }
    }

    /// `len` overrides std's size: on PS Vita std reports sizes through a
    /// 32-bit `off_t` (see `crysirror-vita-io`).
    #[derive(Clone, Debug)]
    pub struct Metadata(std::fs::Metadata, Option<u64>);
    impl Metadata {
        fn of(metadata: std::fs::Metadata, _path: &Path) -> Self {
            #[cfg(target_os = "vita")]
            if metadata.is_file() {
                let len = crysirror_vita_io::stat(_path).ok().map(|(_, len)| len);
                return Self(metadata, len);
            }
            Self(metadata, None)
        }
        pub fn is_file(&self) -> bool {
            self.0.is_file()
        }
        pub fn is_dir(&self) -> bool {
            self.0.is_dir()
        }
        pub fn is_symlink(&self) -> bool {
            self.0.is_symlink()
        }
        pub fn len(&self) -> u64 {
            self.1.unwrap_or_else(|| self.0.len())
        }
        pub fn file_type(&self) -> FileType {
            self.0.file_type()
        }
        pub fn permissions(&self) -> Permissions {
            self.0.permissions()
        }
        pub fn modified(&self) -> io::Result<SystemTime> {
            self.0.modified().map(SystemTime::from_std)
        }
        pub fn created(&self) -> io::Result<SystemTime> {
            self.0.created().map(SystemTime::from_std)
        }
        pub fn accessed(&self) -> io::Result<SystemTime> {
            self.0.accessed().map(SystemTime::from_std)
        }
    }

    #[derive(Clone, Debug)]
    pub struct OpenOptions(std::fs::OpenOptions);
    impl OpenOptions {
        #[allow(clippy::new_without_default)]
        pub fn new() -> Self {
            Self(std::fs::OpenOptions::new())
        }
        pub fn read(&mut self, v: bool) -> &mut Self {
            self.0.read(v);
            self
        }
        pub fn write(&mut self, v: bool) -> &mut Self {
            self.0.write(v);
            self
        }
        pub fn append(&mut self, v: bool) -> &mut Self {
            self.0.append(v);
            self
        }
        pub fn truncate(&mut self, v: bool) -> &mut Self {
            self.0.truncate(v);
            self
        }
        pub fn create(&mut self, v: bool) -> &mut Self {
            self.0.create(v);
            self
        }
        pub fn create_new(&mut self, v: bool) -> &mut Self {
            self.0.create_new(v);
            self
        }
    }

    /// A file opened through a capability root. On PS Vita, read-only opens
    /// use 64-bit SceIo reads (`crysirror-vita-io`): std there cannot seek or
    /// size past 2 GiB, and game archives are often larger.
    #[derive(Debug)]
    pub struct File(Inner);
    #[derive(Debug)]
    enum Inner {
        Std(std::fs::File),
        #[cfg(target_os = "vita")]
        Large { file: crysirror_vita_io::File, position: std::sync::atomic::AtomicU64, path: PathBuf },
    }
    impl File {
        pub fn from_std(file: std::fs::File) -> Self {
            Self(Inner::Std(file))
        }
        /// Open read-only, with 64-bit offsets where std lacks them.
        fn open_read(path: &Path) -> io::Result<Self> {
            #[cfg(target_os = "vita")]
            {
                let file = crysirror_vita_io::File::open(path)?;
                Ok(Self(Inner::Large { file, position: std::sync::atomic::AtomicU64::new(0), path: path.to_path_buf() }))
            }
            #[cfg(not(target_os = "vita"))]
            {
                Ok(Self(Inner::Std(std::fs::File::open(path)?)))
            }
        }
        fn std(&self) -> io::Result<&std::fs::File> {
            match &self.0 {
                Inner::Std(f) => Ok(f),
                #[cfg(target_os = "vita")]
                Inner::Large { .. } => Err(io::Error::new(io::ErrorKind::Unsupported, "read-only large file")),
            }
        }
        /// The std file. A Vita read-only handle reopens its path through std
        /// (offsets past 2 GiB are then unavailable to the caller).
        pub fn into_std(self) -> std::fs::File {
            match self.0 {
                Inner::Std(f) => f,
                #[cfg(target_os = "vita")]
                Inner::Large { path, .. } => std::fs::File::open(&path).expect("reopen a read-only capability file through std"),
            }
        }
        pub fn metadata(&self) -> io::Result<Metadata> {
            match &self.0 {
                Inner::Std(f) => Ok(Metadata(f.metadata()?, None)),
                #[cfg(target_os = "vita")]
                Inner::Large { file, path, .. } => Ok(Metadata(std::fs::metadata(path)?, Some(file.len()))),
            }
        }
        pub fn sync_all(&self) -> io::Result<()> {
            match &self.0 {
                Inner::Std(f) => f.sync_all(),
                #[cfg(target_os = "vita")]
                Inner::Large { .. } => Ok(()),
            }
        }
        pub fn sync_data(&self) -> io::Result<()> {
            match &self.0 {
                Inner::Std(f) => f.sync_data(),
                #[cfg(target_os = "vita")]
                Inner::Large { .. } => Ok(()),
            }
        }
        pub fn set_len(&self, len: u64) -> io::Result<()> {
            self.std()?.set_len(len)
        }
        pub fn try_clone(&self) -> io::Result<Self> {
            match &self.0 {
                Inner::Std(f) => Ok(Self(Inner::Std(f.try_clone()?))),
                #[cfg(target_os = "vita")]
                Inner::Large { path, .. } => Self::open_read(path),
            }
        }
    }
    #[cfg(target_os = "vita")]
    fn large_read(file: &crysirror_vita_io::File, position: &std::sync::atomic::AtomicU64, buf: &mut [u8]) -> io::Result<usize> {
        use std::sync::atomic::Ordering;
        let at = position.load(Ordering::Acquire);
        let n = file.read_at(at, buf)?;
        position.store(at + n as u64, Ordering::Release);
        Ok(n)
    }
    #[cfg(target_os = "vita")]
    fn large_seek(file: &crysirror_vita_io::File, position: &std::sync::atomic::AtomicU64, pos: SeekFrom) -> io::Result<u64> {
        use std::sync::atomic::Ordering;
        let at = match pos {
            SeekFrom::Start(n) => i128::from(n),
            SeekFrom::Current(n) => i128::from(position.load(Ordering::Acquire)) + i128::from(n),
            SeekFrom::End(n) => i128::from(file.len()) + i128::from(n),
        };
        let at = u64::try_from(at).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
        position.store(at, Ordering::Release);
        Ok(at)
    }
    impl Read for File {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            (&*self).read(buf)
        }
    }
    impl Write for File {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let mut f = self.std()?;
            f.write(buf)
        }
        fn flush(&mut self) -> io::Result<()> {
            let mut f = self.std()?;
            f.flush()
        }
    }
    impl Seek for File {
        fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
            (&*self).seek(pos)
        }
    }
    impl Read for &File {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            match &self.0 {
                Inner::Std(f) => { let mut f = f; f.read(buf) }
                #[cfg(target_os = "vita")]
                Inner::Large { file, position, .. } => large_read(file, position, buf),
            }
        }
    }
    impl Seek for &File {
        fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
            match &self.0 {
                Inner::Std(f) => { let mut f = f; f.seek(pos) }
                #[cfg(target_os = "vita")]
                Inner::Large { file, position, .. } => large_seek(file, position, pos),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn root_cannot_be_escaped() {
            let tmp = tempfile::tempdir().unwrap();
            std::fs::create_dir(tmp.path().join("root")).unwrap();
            std::fs::write(tmp.path().join("secret"), b"x").unwrap();
            let dir = Dir::open_ambient_dir(tmp.path().join("root"), crate::console::ambient_authority()).unwrap();
            for bad in ["../secret", "/etc/hosts", "a/../../secret"] {
                assert_eq!(dir.open(bad).unwrap_err().kind(), io::ErrorKind::PermissionDenied, "{bad}");
            }
            dir.create_dir_all("a/b").unwrap();
            dir.write("a/b/c.txt", b"hi").unwrap();
            assert_eq!(dir.open_dir("a").unwrap().read("b/c.txt").unwrap(), b"hi");
            let names: Vec<_> = dir.read_dir("a").unwrap().map(|e| e.unwrap().file_name()).collect();
            assert_eq!(names, vec![OsString::from("b")]);
            let sub = dir.open_dir("a").unwrap();
            dir.rename("a/b/c.txt", &sub, "d.txt").unwrap();
            assert!(dir.is_file("a/d.txt"));
            assert!(dir.open_dir("./a").unwrap().metadata("d.txt").unwrap().modified().is_ok());
        }
    }
}

//! Handle-relative desktop storage. cap-std keeps traversal inside each root,
//! including when a directory is renamed while an operation is in flight.
use super::{PolicyMetadata, relative, user_data_alias};
use cap_std::fs::{Dir, OpenOptions};
use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
const USER_LIMIT: u64 = 64 * 1024 * 1024;
const RESOURCE_LIMIT: u64 = 256 * 1024 * 1024;
#[derive(Debug)]
pub(super) struct Roots {
    resource: Dir,
    user: Dir,
    resource_path: PathBuf,
}
fn denied() -> io::Error {
    io::ErrorKind::PermissionDenied.into()
}
fn checked(name: &str) -> io::Result<PathBuf> {
    relative(name).ok_or_else(denied)
}
fn bounded(file: File, limit: u64) -> io::Result<Vec<u8>> {
    let meta = file.metadata()?;
    if !meta.is_file() || meta.len() > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "native file byte budget",
        ));
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "native file grew beyond budget",
        ));
    }
    Ok(bytes)
}
fn walk(
    dir: &Dir,
    prefix: &Path,
    recurse: bool,
    depth: usize,
    count: &mut usize,
    visit: &mut impl FnMut(&Path, &cap_std::fs::DirEntry) -> io::Result<()>,
) -> io::Result<()> {
    if depth > 64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "directory depth budget",
        ));
    }
    let mut entries = dir.entries()?.collect::<io::Result<Vec<_>>>()?;
    entries.sort_by_key(|e| e.file_name().to_string_lossy().to_ascii_lowercase());
    for entry in entries {
        *count += 1;
        if *count > 100_000 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "directory entry budget",
            ));
        }
        if entry
            .file_name()
            .to_string_lossy()
            .to_ascii_lowercase()
            .starts_with(".ethornell-")
        {
            continue;
        }
        let kind = entry.file_type()?;
        if !kind.is_file() && !kind.is_dir() {
            return Err(denied());
        }
        let path = prefix.join(entry.file_name());
        visit(&path, &entry)?;
        if recurse && kind.is_dir() {
            walk(&entry.open_dir()?, &path, true, depth + 1, count, visit)?;
        }
    }
    Ok(())
}
impl Roots {
    pub fn new(resource: &Path, user: &Path) -> io::Result<Self> {
        let resource_path = resource.canonicalize()?;
        let user_path = user.canonicalize()?;
        if resource_path.starts_with(&user_path) || user_path.starts_with(&resource_path) {
            return Err(denied());
        }
        Ok(Self {
            resource: Dir::open_ambient_dir(&resource_path, cap_std::ambient_authority())?,
            user: Dir::open_ambient_dir(&user_path, cap_std::ambient_authority())?,
            resource_path,
        })
    }
    pub fn lock_writer(&self) -> io::Result<File> {
        let file = self
            .user
            .open_with(
                ".ethornell-session.lock",
                OpenOptions::new().read(true).write(true).create(true),
            )?
            .into_std();
        file.try_lock().map_err(io::Error::other)?;
        Ok(file)
    }
    pub fn open_resource(&self, name: &str) -> io::Result<File> {
        let name = name.replace('\\', "/");
        if name.starts_with("bgi-user/") {
            return Err(denied());
        }
        let path = checked(name.strip_prefix("bgi-resource/").unwrap_or(&name))?;
        match self.user.open(&path) {
            Ok(file) => Ok(file.into_std()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                Ok(self.resource.open(path)?.into_std())
            }
            Err(error) => Err(error),
        }
    }
    pub fn read_resource(&self, name: &str, limit: u64) -> io::Result<Vec<u8>> {
        bounded(self.open_resource(name)?, limit)
    }
    pub fn validate_resource_tree(&self) -> io::Result<()> {
        walk(
            &self.resource,
            Path::new(""),
            true,
            0,
            &mut 0,
            &mut |_, _| Ok(()),
        )
    }
    pub fn open_manager(
        &self,
    ) -> crate::native_core::Result<crate::native_archive::ResourceManager> {
        use crate::native_archive::{
            ArchiveFormat, ResourceBudget, ResourceManager, detect_archive_format_from_bytes,
        };
        let mut files = Vec::new();
        walk(
            &self.resource,
            Path::new(""),
            true,
            0,
            &mut 0,
            &mut |path, entry| {
                if entry.file_type()?.is_file() {
                    let mut file = entry.open()?.into_std();
                    let mut header = [0; 64];
                    let n = file.read(&mut header)?;
                    if detect_archive_format_from_bytes(&header[..n]) != ArchiveFormat::Unknown {
                        if files.len() >= 512 {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "archive descriptor budget",
                            ));
                        }
                        file.seek(SeekFrom::Start(0))?;
                        files.push((self.resource_path.join(path), file));
                    }
                }
                Ok(())
            },
        )?;
        files.sort_by(|a, b| a.0.cmp(&b.0));
        ResourceManager::from_authorized_files_with_limits(
            self.resource_path.clone(),
            files,
            ResourceBudget::android(),
        )
    }
    fn root<'a>(&'a self, name: &'a str) -> (&'a Dir, &'a str, bool) {
        if let Some(name) = name.strip_prefix("bgi-resource/") {
            (&self.resource, name, true)
        } else if let Some(name) = name.strip_prefix("bgi-user/") {
            (&self.user, name, true)
        } else {
            (&self.user, name, false)
        }
    }
    pub fn metadata(&self, name: &str) -> Option<PolicyMetadata> {
        let name = name.replace('\\', "/");
        if let Some(relative) = name.strip_prefix("bgi-resource/") {
            if let Some(metadata) = self.metadata(&format!("bgi-user/{relative}")) {
                return Some(metadata);
            }
        }
        let (root, name, explicit) = self.root(&name);
        let path = checked(name).ok()?;
        let path = if path.as_os_str().is_empty() {
            Path::new(".")
        } else {
            &path
        };
        let m = match root.metadata(path) {
            Ok(m) => m,
            Err(e) if !explicit && e.kind() == io::ErrorKind::NotFound => {
                self.resource.metadata(path).ok()?
            }
            Err(_) => return None,
        };
        if !m.is_dir() && !m.is_file() {
            return None;
        }
        Some(PolicyMetadata {
            is_dir: m.is_dir(),
            len: m.len(),
            readonly: m.permissions().readonly(),
        })
    }
    pub fn read_loose(&self, name: &str) -> io::Result<Vec<u8>> {
        let name = name.replace('\\', "/");
        if name.starts_with("bgi-resource/") {
            return self.read_resource(&name, RESOURCE_LIMIT);
        }
        let (root, name, explicit) = self.root(&name);
        let path = checked(name)?;
        match root.open(&path) {
            Ok(f) => bounded(
                f.into_std(),
                if std::ptr::eq(root, &self.user) {
                    USER_LIMIT
                } else {
                    RESOURCE_LIMIT
                },
            ),
            Err(e) if !explicit && e.kind() == io::ErrorKind::NotFound => {
                self.read_resource(name, RESOURCE_LIMIT)
            }
            Err(e) => Err(e),
        }
    }
    pub fn enumerate(
        &self,
        pattern: &str,
        recursive: bool,
        max: usize,
        dirs: bool,
    ) -> io::Result<Vec<String>> {
        let name = pattern.replace('\\', "/");
        let (root, name, _) = self.root(&name);
        let path = checked(name)?;
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let wildcard = path.file_name().and_then(|v| v.to_str()).unwrap_or("*");
        let mut out = Vec::new();
        let limit = if max == 0 { 10_000 } else { max.min(10_000) };
        walk(
            &root.open_dir(parent)?,
            Path::new(""),
            recursive,
            0,
            &mut 0,
            &mut |name, entry| {
                if out.len() < limit
                    && entry.file_type()?.is_dir() == dirs
                    && super::super::windows_wildcard_matches(
                        wildcard,
                        &entry.file_name().to_string_lossy(),
                    )
                {
                    out.push(name.to_string_lossy().replace('/', "\\"));
                }
                Ok(())
            },
        )?;
        Ok(out)
    }
    fn writable(&self, name: &str, parents: bool) -> io::Result<PathBuf> {
        let name = user_data_alias(name);
        if name.starts_with("bgi-resource/") {
            return Err(denied());
        }
        let path = checked(name.strip_prefix("bgi-user/").unwrap_or(&name))?;
        if path.file_name().is_none() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        if parents {
            if let Some(p) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                self.user.create_dir_all(p)?;
            }
        }
        Ok(path)
    }
    pub fn create_directory(&self, name: &str) -> io::Result<()> {
        self.user.create_dir(self.writable(name, false)?)
    }
    pub fn probe_writable(&self, name: &str) -> io::Result<()> {
        let aliased = user_data_alias(name);
        let path = checked(aliased.strip_prefix("bgi-user/").unwrap_or(&aliased))?;
        if aliased.starts_with("bgi-resource/") {
            return Err(denied());
        }
        if name.replace('\\', "/").starts_with("bgi-resource/") {
            self.user.create_dir_all(&path)?;
        }
        let dir = self.user.open_dir(if path.as_os_str().is_empty() {
            Path::new(".")
        } else {
            &path
        })?;
        let temp = format!(".ethornell-probe-{}", std::process::id());
        drop(dir.open_with(&temp, OpenOptions::new().write(true).create_new(true))?);
        dir.remove_file(temp)
    }
    pub fn rename(&self, from: &str, to: &str) -> io::Result<()> {
        self.user.rename(
            self.writable(from, false)?,
            &self.user,
            self.writable(to, true)?,
        )
    }
    pub fn delete(&self, name: &str) -> io::Result<()> {
        self.user.remove_file(self.writable(name, false)?)
    }
    pub fn read_user(&self, name: &str) -> io::Result<Vec<u8>> {
        bounded(
            self.user.open(self.writable(name, false)?)?.into_std(),
            USER_LIMIT,
        )
    }
    pub fn atomic_write(&self, name: &str, bytes: &[u8]) -> io::Result<()> {
        if bytes.len() as u64 > USER_LIMIT {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "save too large"));
        }
        let dest = self.writable(name, true)?;
        let parent = dest
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let dir = self.user.open_dir(parent)?;
        static SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let temp = format!(
            ".ethornell-save-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        let result = (|| {
            let mut f = dir
                .open_with(&temp, OpenOptions::new().write(true).create_new(true))?
                .into_std();
            f.write_all(bytes)?;
            f.sync_all()?;
            drop(f);
            dir.rename(&temp, &dir, dest.file_name().unwrap())
        })();
        if result.is_err() {
            let _ = dir.remove_file(temp);
        }
        result
    }
}

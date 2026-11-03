// SPDX-License-Identifier: MPL-2.0
//! In-memory synchronous view of the client's capability filesystem. Missing
//! reads are retried by the adapter before an instruction can commit.
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    io::{self, Cursor, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct State {
    #[serde(skip)]
    pub(crate) pending_texts: BTreeMap<u64, crate::native_string::Request>,
    #[serde(skip)]
    pub(crate) text_sizes: BTreeMap<u64, [u32; 2]>,
    #[serde(skip)]
    pub files: BTreeMap<String, std::sync::Arc<Vec<u8>>>,
    pub absent: BTreeSet<String>,
    #[serde(skip)]
    pub missing: BTreeSet<String>,
    #[serde(skip)]
    pub writes: BTreeMap<String, Vec<u8>>,
    /// Client save-mount deletions committed with `writes`. A later write to
    /// the same key cancels its deletion and vice versa.
    #[serde(skip)]
    pub removes: BTreeSet<String>,
    /// Deferred canonical engine logs; discarded with an uncommitted native
    /// instruction, then appended through HostServices at its owner boundary.
    #[serde(skip)]
    pub logs: Vec<String>,
    /// Per-pump observations supplied by the shared capability metadata API.
    /// They are not serialized native or framework save state.
    #[serde(skip)]
    pub probes: BTreeMap<String, bool>,
    #[serde(skip)]
    pub pending_probes: BTreeSet<String>,
    /// Engine canvas capture of the current composed frame, supplied by the
    /// adapter for the instruction that requested it.
    #[serde(skip)]
    pub frame_capture: Option<std::sync::Arc<crate::assets::RgbaImage>>,
    #[serde(skip)]
    pub frame_capture_requested: bool,
}
thread_local! {static STATE:RefCell<State>=RefCell::new(State::default());}
pub(crate) fn text_size(request: crate::native_string::Request) -> io::Result<[u32; 2]> {
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        let generation = request.definition.generation;
        if let Some(size) = state.text_sizes.get(&generation) {
            return Ok(*size);
        }
        if state.pending_texts.len() >= 256 {
            return Err(io::Error::other("native STRING preparation budget"));
        }
        state.pending_texts.insert(generation, request);
        Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "native STRING engine preparation pending",
        ))
    })
}
pub fn frame_capture() -> io::Result<crate::assets::RgbaImage> {
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        if let Some(image) = &state.frame_capture {
            return Ok(image.as_ref().clone());
        }
        state.frame_capture_requested = true;
        Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "native frame capture pending",
        ))
    })
}
pub fn append_engine_log(message: String) -> io::Result<()> {
    if message.len() > crysirror_core::service::MAX_LOG_LINE_BYTES {
        return Err(io::Error::other("native engine log line limit"));
    }
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if state.logs.len() >= 128 {
            return Err(io::Error::other("native engine log batch limit"));
        }
        state.logs.push(message);
        Ok(())
    })
}
pub fn scoped<T>(state: &mut State, f: impl FnOnce() -> T) -> T {
    struct Guard<'a>(&'a mut State);
    impl Drop for Guard<'_> {
        fn drop(&mut self) {
            STATE.with(|s| std::mem::swap(self.0, &mut s.borrow_mut()));
        }
    }
    STATE.with(|s| std::mem::swap(state, &mut s.borrow_mut()));
    let guard = Guard(state);
    let result = f();
    drop(guard);
    result
}
pub fn canonical(p: &Path) -> io::Result<String> {
    let text = p
        .to_str()
        .ok_or_else(|| io::Error::other("non UTF-8 native path"))?;
    let text = text.replace('\\', "/");
    let mut parts = text.split('/');
    let mount = parts.next().unwrap_or("");
    if !matches!(mount, "asset" | "save") {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "native path outside client mounts",
        ));
    }
    let rest = parts.collect::<Vec<_>>().join("/");
    if rest.is_empty() {
        return Ok(mount.into());
    }
    crysirror_core::path::VfsPath::new(
        if mount == "asset" {
            crysirror_core::path::Mount::Asset
        } else {
            crysirror_core::path::Mount::Save
        },
        &rest,
    )
    .map(|v| format!("{mount}/{}", v.relative()))
    .map_err(|e| io::Error::new(io::ErrorKind::PermissionDenied, e.to_string()))
}
pub fn read(p: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    let key = canonical(p.as_ref())?;
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if let Some(b) = s.files.get(&key) {
            return Ok(b.as_ref().clone());
        }
        if !s.absent.contains(&key) {
            s.missing.insert(key);
        }
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "native capability read pending/missing",
        ))
    })
}
pub fn read_to_string(p: impl AsRef<Path>) -> io::Result<String> {
    String::from_utf8(read(p)?).map_err(io::Error::other)
}
/// Native file-existence policy, with no ambient filesystem or whole-file read.
/// An unresolved observation causes the adapter to retry the instruction after
/// the owner supplies metadata; false is never committed for a pending query.
pub fn probe_font(name: &str) -> io::Result<bool> {
    if name.is_empty() {
        return Ok(false);
    }
    if name.len() > 256 || name.chars().any(char::is_control) {
        return Err(io::Error::other("native font query name budget"));
    }
    let key = format!("font:{name}");
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        if let Some(value) = state.probes.get(&key) {
            return Ok(*value);
        }
        if state.probes.len() >= 4096 || state.pending_probes.len() >= 32 {
            return Err(io::Error::other("native font query budget"));
        }
        state.pending_probes.insert(key);
        Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "native font capability query pending",
        ))
    })
}
pub fn probe_file(p: impl AsRef<Path>) -> io::Result<bool> {
    let key = canonical(p.as_ref())?;
    let key = if key == "asset" || key == "save" {
        format!("{key}/")
    } else {
        key
    };
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if let Some(present) = state.probes.get(&key) {
            return Ok(*present);
        }
        if state.probes.len() >= 4096 || state.pending_probes.len() >= 32 {
            return Err(io::Error::other("native file metadata query limit"));
        }
        state.pending_probes.insert(key);
        Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "native file metadata pending",
        ))
    })
}
pub fn write(p: impl AsRef<Path>, data: impl AsRef<[u8]>) -> io::Result<()> {
    let key = canonical(p.as_ref())?;
    if !key.starts_with("save/") {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "native asset mount is read-only",
        ));
    }
    if data.as_ref().len() > 64 * 1024 * 1024 {
        return Err(io::Error::other("native file limit"));
    }
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.files
            .insert(key.clone(), std::sync::Arc::new(data.as_ref().to_vec()));
        s.probes.insert(key.clone(), true);
        s.removes.remove(&key);
        s.writes.insert(key, data.as_ref().to_vec());
    });
    Ok(())
}
pub fn create_dir_all(p: impl AsRef<Path>) -> io::Result<()> {
    let key = canonical(p.as_ref())?;
    if key == "save" || key.starts_with("save/") {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "cannot create asset directories",
        ))
    }
}
fn save_key(p: &Path) -> io::Result<String> {
    let key = canonical(p)?;
    if key.starts_with("save/") {
        Ok(key)
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "native asset mount is read-only",
        ))
    }
}
pub fn copy(a: impl AsRef<Path>, b: impl AsRef<Path>) -> io::Result<u64> {
    let bytes = read(a)?;
    write(b, &bytes)?;
    Ok(bytes.len() as u64)
}
pub fn rename(a: impl AsRef<Path>, b: impl AsRef<Path>) -> io::Result<()> {
    save_key(a.as_ref())?;
    copy(a.as_ref(), b)?;
    remove_file(a)
}
pub fn remove_file(p: impl AsRef<Path>) -> io::Result<()> {
    let key = save_key(p.as_ref())?;
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.files.remove(&key);
        s.writes.remove(&key);
        s.probes.insert(key.clone(), false);
        s.absent.insert(key.clone());
        s.removes.insert(key);
    });
    Ok(())
}
/// Removes the files this view knows under a save directory. Host directory
/// listing is not consulted; unknown files are left in place.
pub fn remove_dir_all(p: impl AsRef<Path>) -> io::Result<()> {
    let prefix = save_key(p.as_ref())? + "/";
    let keys: Vec<String> = STATE.with(|s| {
        let s = s.borrow();
        s.files
            .keys()
            .chain(s.writes.keys())
            .filter(|k| k.starts_with(&prefix))
            .cloned()
            .collect()
    });
    for key in keys {
        remove_file(Path::new(&key))?;
    }
    Ok(())
}
pub struct Permissions {
    read_only: bool,
}
impl Permissions {
    pub fn readonly(&self) -> bool {
        self.read_only
    }
}
pub struct Metadata {
    len: u64,
    read_only: bool,
}
impl Metadata {
    pub fn len(&self) -> u64 {
        self.len
    }
    pub fn is_file(&self) -> bool {
        true
    }
    pub fn is_dir(&self) -> bool {
        false
    }
    pub fn permissions(&self) -> Permissions {
        Permissions {
            read_only: self.read_only,
        }
    }
}
pub fn metadata(p: impl AsRef<Path>) -> io::Result<Metadata> {
    Ok(Metadata {
        len: read(&p)?.len() as u64,
        read_only: canonical(p.as_ref())?.starts_with("asset/"),
    })
}
pub struct File {
    path: PathBuf,
    buf: Cursor<Vec<u8>>,
    writable: bool,
}
impl File {
    pub fn open(p: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self {
            path: p.as_ref().into(),
            buf: Cursor::new(read(p)?),
            writable: false,
        })
    }
    pub fn metadata(&self) -> io::Result<Metadata> {
        Ok(Metadata {
            len: self.buf.get_ref().len() as u64,
            read_only: !self.writable,
        })
    }
    pub fn set_permissions(&self, p: Permissions) -> io::Result<()> {
        if p.readonly() {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "native permission changes unsupported",
            ))
        } else {
            Ok(())
        }
    }
    pub fn sync_all(&self) -> io::Result<()> {
        Ok(())
    }
}
impl Read for File {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        self.buf.read(b)
    }
}
impl Seek for File {
    fn seek(&mut self, p: SeekFrom) -> io::Result<u64> {
        self.buf.seek(p)
    }
}
impl Write for File {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        if !self.writable {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "read-only native file",
            ));
        }
        let n = self.buf.write(b)?;
        write(&self.path, self.buf.get_ref())?;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[derive(Default)]
pub struct OpenOptions {
    writable: bool,
    create_new: bool,
}
impl OpenOptions {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn write(&mut self, b: bool) -> &mut Self {
        self.writable = b;
        self
    }
    pub fn create_new(&mut self, b: bool) -> &mut Self {
        self.create_new = b;
        self
    }
    pub fn open(&self, p: impl AsRef<Path>) -> io::Result<File> {
        let buf = if self.create_new {
            if read(&p).is_ok() {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "native file exists",
                ));
            }
            Vec::new()
        } else {
            read(&p)?
        };
        Ok(File {
            path: p.as_ref().into(),
            buf: Cursor::new(buf),
            writable: self.writable,
        })
    }
}
pub struct Entry(PathBuf);
impl Entry {
    pub fn file_name(&self) -> std::ffi::OsString {
        self.0.file_name().unwrap_or_default().into()
    }
    pub fn path(&self) -> PathBuf {
        self.0.clone()
    }
    pub fn file_type(&self) -> io::Result<FileType> {
        Ok(FileType)
    }
}
pub struct FileType;
impl FileType {
    pub fn is_dir(&self) -> bool {
        false
    }
    pub fn is_file(&self) -> bool {
        true
    }
}
pub fn read_dir(p: impl AsRef<Path>) -> io::Result<std::vec::IntoIter<io::Result<Entry>>> {
    let prefix = canonical(p.as_ref())? + "/";
    Ok(STATE
        .with(|s| {
            s.borrow()
                .files
                .keys()
                .filter(|k| k.starts_with(&prefix) && !k[prefix.len()..].contains('/'))
                .map(|k| Ok(Entry(k.into())))
                .collect::<Vec<_>>()
        })
        .into_iter())
}

#[cfg(test)]
mod save_mutation_tests {
    use super::*;

    #[test]
    fn remove_copy_and_rename_stay_inside_the_save_mount_and_commit_together() {
        let mut state = State::default();
        scoped(&mut state, || {
            write("save/a.sav", b"one").unwrap();
            assert_eq!(copy("save/a.sav", "save/b.sav").unwrap(), 3);
            rename("save/b.sav", "save/c.sav").unwrap();
            assert_eq!(read("save/c.sav").unwrap(), b"one");
            assert!(read("save/b.sav").is_err());
            remove_file("save/a.sav").unwrap();
            assert_eq!(probe_file("save/a.sav").unwrap(), false);
            assert!(remove_file("asset/Scene.pck").is_err());
            assert!(copy("save/c.sav", "asset/x").is_err());
        });
        assert_eq!(
            state.writes.keys().collect::<Vec<_>>(),
            ["save/c.sav"],
            "a removed or renamed file is not also written"
        );
        assert_eq!(
            state.removes.iter().collect::<Vec<_>>(),
            ["save/a.sav", "save/b.sav"]
        );
        let mut state2 = state.clone();
        scoped(&mut state2, || write("save/a.sav", b"two").unwrap());
        assert!(!state2.removes.contains("save/a.sav"), "a later write cancels deletion");
    }
}

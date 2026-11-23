// SPDX-License-Identifier: MPL-2.0
//! 64-bit read-only file access on PS Vita.
//!
//! On `armv7-sony-vita-newlibeabihf`, Rust std's file backend uses newlib's
//! `off_t`, which the libc crate defines as a 32-bit `c_long` for this
//! target. `File::seek` and `Metadata::len` then fail or wrap above 2 GiB,
//! while game archives are often larger (SugarPot `grp.aos`: 2.8 GB).
//! SceIo itself is 64-bit (`SceOff`). This crate reads through it:
//! `sceIoOpen`, `sceIoPread` at an explicit 64-bit offset, and
//! `sceIoGetstat`/`sceIoGetstatByFd` for sizes. On other targets the same API
//! is backed by std, so callers need no target switches.
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

/// A read-only file with 64-bit length, offsets and positioned reads.
pub struct File {
    #[cfg(target_os = "vita")]
    fd: vitasdk_sys::SceUID,
    #[cfg(not(target_os = "vita"))]
    file: std::sync::Mutex<std::fs::File>,
    len: u64,
    position: u64,
}

#[cfg(target_os = "vita")]
fn sce_error(code: i32) -> io::Error {
    // SCE_ERROR_ERRNO_ENOENT / ENOTDIR are reported as absent files.
    match code as u32 {
        0x8001_0002 | 0x8001_0014 => io::Error::new(io::ErrorKind::NotFound, format!("SceIo {code:#x}")),
        0x8001_000d => io::Error::new(io::ErrorKind::PermissionDenied, format!("SceIo {code:#x}")),
        _ => io::Error::other(format!("SceIo {code:#x}")),
    }
}

#[cfg(target_os = "vita")]
fn c_path(path: &Path) -> io::Result<std::ffi::CString> {
    let text = path.to_str().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path is not UTF-8"))?;
    std::ffi::CString::new(text).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains NUL"))
}

/// Size and kind of `path` with a 64-bit size: `(is_directory, len)`.
pub fn stat(path: &Path) -> io::Result<(bool, u64)> {
    #[cfg(target_os = "vita")]
    {
        let c = c_path(path)?;
        let mut st: vitasdk_sys::SceIoStat = unsafe { std::mem::zeroed() };
        let rc = unsafe { vitasdk_sys::sceIoGetstat(c.as_ptr(), &mut st) };
        if rc < 0 {
            return Err(sce_error(rc));
        }
        let dir = st.st_mode & vitasdk_sys::SCE_S_IFDIR as i32 != 0;
        Ok((dir, u64::try_from(st.st_size).unwrap_or(0)))
    }
    #[cfg(not(target_os = "vita"))]
    {
        let m = std::fs::metadata(path)?;
        Ok((m.is_dir(), m.len()))
    }
}

impl File {
    pub fn open(path: &Path) -> io::Result<Self> {
        #[cfg(target_os = "vita")]
        {
            let c = c_path(path)?;
            let fd = unsafe { vitasdk_sys::sceIoOpen(c.as_ptr(), vitasdk_sys::SCE_O_RDONLY as i32, 0) };
            if fd < 0 {
                return Err(sce_error(fd));
            }
            let mut st: vitasdk_sys::SceIoStat = unsafe { std::mem::zeroed() };
            let rc = unsafe { vitasdk_sys::sceIoGetstatByFd(fd, &mut st) };
            if rc < 0 {
                unsafe { vitasdk_sys::sceIoClose(fd) };
                return Err(sce_error(rc));
            }
            if st.st_mode & vitasdk_sys::SCE_S_IFDIR as i32 != 0 {
                unsafe { vitasdk_sys::sceIoClose(fd) };
                return Err(io::Error::new(io::ErrorKind::IsADirectory, "is a directory"));
            }
            Ok(Self { fd, len: u64::try_from(st.st_size).unwrap_or(0), position: 0 })
        }
        #[cfg(not(target_os = "vita"))]
        {
            let file = std::fs::File::open(path)?;
            let metadata = file.metadata()?;
            if metadata.is_dir() {
                return Err(io::Error::new(io::ErrorKind::IsADirectory, "is a directory"));
            }
            let len = metadata.len();
            Ok(Self { file: std::sync::Mutex::new(file), len, position: 0 })
        }
    }
    pub fn len(&self) -> u64 {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    /// Read at an absolute offset without moving the cursor.
    pub fn read_at(&self, offset: u64, out: &mut [u8]) -> io::Result<usize> {
        if offset >= self.len || out.is_empty() {
            return Ok(0);
        }
        let count = out.len().min((self.len - offset).min(1 << 30) as usize);
        #[cfg(target_os = "vita")]
        {
            let at = i64::try_from(offset).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
            let n = unsafe { vitasdk_sys::sceIoPread(self.fd, out.as_mut_ptr().cast(), count as u32, at) };
            if n < 0 {
                return Err(sce_error(n));
            }
            Ok(n as usize)
        }
        #[cfg(not(target_os = "vita"))]
        {
            let mut file = self.file.lock().map_err(|_| io::Error::other("file lock poisoned"))?;
            file.seek(SeekFrom::Start(offset))?;
            file.read(&mut out[..count])
        }
    }
}

impl Read for File {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let n = self.read_at(self.position, out)?;
        self.position += n as u64;
        Ok(n)
    }
}

impl Seek for File {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let at = match from {
            SeekFrom::Start(n) => i128::from(n),
            SeekFrom::Current(n) => i128::from(self.position) + i128::from(n),
            SeekFrom::End(n) => i128::from(self.len) + i128::from(n),
        };
        self.position = u64::try_from(at).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
        Ok(self.position)
    }
}

#[cfg(target_os = "vita")]
impl Drop for File {
    fn drop(&mut self) {
        unsafe { vitasdk_sys::sceIoClose(self.fd) };
    }
}

impl std::fmt::Debug for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("File").field("len", &self.len).field("position", &self.position).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_seeks_and_sizes() {
        let dir = std::env::temp_dir().join(format!("crysirror-vita-io-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("f.bin");
        std::fs::write(&path, b"0123456789").unwrap();
        let mut f = File::open(&path).unwrap();
        assert_eq!(f.len(), 10);
        assert_eq!(stat(&path).unwrap(), (false, 10));
        f.seek(SeekFrom::End(-3)).unwrap();
        let mut b = [0; 8];
        assert_eq!(f.read(&mut b).unwrap(), 3);
        assert_eq!(&b[..3], b"789");
        assert_eq!(f.read_at(2, &mut b[..2]).unwrap(), 2);
        assert_eq!(&b[..2], b"23");
        assert!(File::open(&dir).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}

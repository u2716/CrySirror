// Copyright (C) 2026 CrySirror contributors. GPL-2.0-or-later.
//! Confirm2 (Rance7 DLL SHA256 111b56f4bd5a3d4d5f5b56b91da78cef5a7e5af2
//! 32e84a87c192a272359798ed). Original semantics, read from the DLL:
//! - Init 0x100019c0 caches two IMainSystem interfaces only.
//! - ExistKeyFile 0x10001080 / CreateKeyFile 0x100014a0: a 16-byte key file
//!   of LE dwords magic 0x11223344 (.data 0x1000cc18), 0, volume serial and
//!   nKeyNum^0x52. The serial helper 0x10001610 only queries a root whose
//!   second byte is ';' (0x1000169f), so ordinary `X:\` roots record 0.
//! - CheckProtectFile 0x10001220: open the named file on a CD-ROM root, pick
//!   offset = gen(time) % 0x59682F00 and compare its byte with
//!   (gen((offset*nKeyNum)|1) >> 8) & 0xFF. `gen` is 0x10001780: a four-word
//!   twister (N4, M3) seeded by `s0=seed|1, s[i+1]=s[i]*69069`, MT tempering.
//! The protect file is read only through this client's asset capability; no
//! drive scan or key search exists here. Owner policy (2026-11-09): when the
//! protect file is not reachable from that capability the check passes, so
//! play is not blocked; a reachable file still gets the original comparison.
use crate::{
    adapter::Dispatch,
    native_files::NativeFiles,
    vm::{ExternalCall, ExternalTarget, Machine, external::{Argument, HllReturn}},
};
use anyhow::{Result, bail, ensure};
use crysirror_core::{
    Error,
    path::{Mount, VfsPath},
    service::{ClientStorage, HostServices},
    state::ClientId,
};
use crysirror_tasks::{Task, TaskPool};

const MAGIC: u32 = 0x1122_3344;
const PROTECT_SPAN: u32 = 0x5968_2F00;

/// Confirm2 0x10001780 first output for a freshly seeded generator.
pub(crate) fn generator(seed: u32) -> u32 {
    let mut s = [seed | 1, 0, 0, 0];
    for i in 1..4 {
        s[i] = s[i - 1].wrapping_mul(69069);
    }
    let twist = |a: u32, b: u32, prior: u32| {
        let y = (((b ^ a) & 0x7fff_fffe) ^ a) >> 1;
        y ^ if b & 1 != 0 { 0x9908_b0df } else { 0 } ^ prior
    };
    let mut y = twist(s[0], s[1], s[3]);
    y ^= y >> 11;
    y ^= (y & 0xff3a_58ad) << 7;
    y ^= (y & 0xffff_df8c) << 15;
    y ^ (y >> 18)
}
pub(crate) fn protect_offset(clock: u32) -> u32 {
    generator(clock) % PROTECT_SPAN
}
pub(crate) fn protect_byte(offset: u32, key: i32) -> u8 {
    (generator(offset.wrapping_mul(key as u32) | 1) >> 8) as u8
}
pub(crate) fn key_file(key: i32) -> [u8; 16] {
    let mut out = [0; 16];
    out[..4].copy_from_slice(&MAGIC.to_le_bytes());
    out[12..].copy_from_slice(&((key ^ 0x52) as u32).to_le_bytes());
    out
}
/// 0x10001080 reads sequentially and requires each dword inside the file.
pub(crate) fn key_file_matches(bytes: &[u8], key: i32) -> bool {
    let word = |i: usize| bytes.get(i * 4..i * 4 + 4).map(|w| u32::from_le_bytes(w.try_into().unwrap()));
    word(0) == Some(MAGIC) && word(1) == Some(0) && word(2) == Some(0) && word(3) == Some((key ^ 0x52) as u32)
}

enum Work {
    Exist { save: VfsPath, original: VfsPath, key: i32 },
    Create { save: VfsPath, key: i32 },
    Protect { path: VfsPath, offset: u32, key: i32 },
}
fn run(storage: &dyn ClientStorage, work: Work) -> Result<i32> {
    let read_small = |path: &VfsPath, mount_read: bool| -> Result<Option<Vec<u8>>> {
        let opened = if mount_read { storage.open_read(path) } else { storage.open_asset(path) };
        match opened {
            Ok(reader) => Ok(Some(crysirror_resource::read_all(reader.as_ref(), 4096)?)),
            Err(Error::NotFound) => Ok(None),
            Err(error) => Err(error.into()),
        }
    };
    let resolve = |path: &VfsPath| -> Result<VfsPath> {
        Ok(crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(storage, path)?
            .unwrap_or_else(|| path.clone()))
    };
    Ok(match work {
        Work::Exist { save, original, key } => {
            let bytes = match read_small(&resolve(&save)?, true)? {
                Some(bytes) => Some(bytes),
                None => read_small(&resolve(&original)?, false)?,
            };
            i32::from(bytes.is_some_and(|bytes| key_file_matches(&bytes, key)))
        }
        Work::Create { save, key } => match storage.write_atomic(&resolve(&save)?, &key_file(key)) {
            Ok(()) => 1,
            Err(Error::Io(_) | Error::ReadOnly | Error::NotFound) => 0,
            Err(error) => return Err(error.into()),
        },
        Work::Protect { path, offset, key } => {
            let path = resolve(&path)?;
            let reader = match storage.open_asset(&path) {
                Ok(reader) => reader,
                // Owner decision 2026-11-09: while no media-root capability
                // exists, an unreachable protect file does not block play.
                // A reachable file still gets the original byte check.
                Err(Error::NotFound) => return Ok(1),
                Err(error) => return Err(error.into()),
            };
            if u64::from(offset) + 1 > reader.len()? {
                return Ok(0);
            }
            let mut byte = [0u8];
            if reader.read_at(u64::from(offset), &mut byte)? != 1 {
                return Ok(0);
            }
            i32::from(byte[0] == protect_byte(offset, key))
        }
    })
}

pub(crate) struct NativeConfirm {
    owner: ClientId,
    pool: TaskPool,
    pending: Option<(ExternalCall, Task<i32>)>,
}
impl NativeConfirm {
    pub(crate) fn new(owner: ClientId, pool: TaskPool) -> Self {
        Self { owner, pool, pending: None }
    }
    pub(crate) fn has_state(&self) -> bool {
        self.pending.is_some()
    }
    pub(crate) fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        files: &NativeFiles,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        let ExternalTarget::Hll { library, function } = call.target else {
            ensure!(self.pending.is_none(), "Confirm2 changed pending target");
            return Ok(None);
        };
        let program = machine.program.clone();
        let library = &program.libraries[library];
        if library.name != b"Confirm2" {
            ensure!(self.pending.is_none(), "Confirm2 changed pending library");
            return Ok(None);
        }
        ensure!(owner == self.owner, "Confirm2 foreign owner");
        let function = &library.functions[function];
        let args = machine.hll_arguments(call)?;
        if let Some((pending, task)) = &mut self.pending {
            ensure!(*pending == *call, "Confirm2 stale pending call");
            let Some(result) = task.try_take()? else {
                return Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))));
            };
            self.pending = None;
            machine.complete_hll(call, HllReturn::Scalar(result))?;
            return Ok(Some(Dispatch::Completed));
        }
        let work = match (function.name.as_slice(), function.return_kind, args.as_slice()) {
            (b"Init", 0, [Argument::Scalar { kind: 26, .. }]) => {
                machine.complete_hll(call, HllReturn::Void)?;
                return Ok(Some(Dispatch::Completed));
            }
            (name, 10, [Argument::String { bytes, .. }, Argument::Scalar { kind: 10, bits: key }]) => match name {
                b"ExistKeyFile" => {
                    let (save, original) = files.save_path(bytes)?;
                    Work::Exist { save, original, key: *key }
                }
                b"CreateKeyFile" => Work::Create { save: files.save_path(bytes)?.0, key: *key },
                b"CheckProtectFile" => {
                    let (mount, relative) = files.hll_path(bytes, Mount::Asset)?;
                    ensure!(mount == Mount::Asset, "Confirm2 protect file mount");
                    // 0x10001340: a non-null cached interface supplies the
                    // seed through vtable+4; the engine clock is that clock.
                    Work::Protect {
                        path: VfsPath::new(Mount::Asset, &relative)?,
                        offset: protect_offset(host.now_ms() as u32),
                        key: *key,
                    }
                }
                _ => bail!("unsupported Confirm2 export {}", String::from_utf8_lossy(name)),
            },
            _ => bail!("unsupported Confirm2 function/ABI {}", String::from_utf8_lossy(&function.name)),
        };
        let storage = host.storage(owner)?;
        let task = self.pool.reserve()?.spawn(move |_cancel| run(storage.as_ref(), work))?;
        self.pending = Some((call.clone(), task));
        Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn key_file_round_trips_original_layout() {
        let bytes = key_file(1215);
        assert_eq!(bytes, [0x44, 0x33, 0x22, 0x11, 0, 0, 0, 0, 0, 0, 0, 0, 0xed, 0x04, 0, 0]);
        assert!(key_file_matches(&bytes, 1215));
        assert!(!key_file_matches(&bytes, 1216));
        assert!(!key_file_matches(&bytes[..12], 1215));
    }
    #[test]
    fn protect_generator_reproduces_owner_disc_bytes() {
        // Bytes sampled from the owner's Rance7 protect file (key1215); the
        // file itself is never part of the repository.
        let Some(path) = std::env::var_os("CRYSIRROR_RANCE7_PROTECT_FILE") else { return };
        use std::io::{Read, Seek, SeekFrom};
        let mut file = std::fs::File::open(path).unwrap();
        for offset in [0u32, 1, 2, 3, 1000, 123_456_789, 1_499_999_999] {
            let mut byte = [0];
            file.seek(SeekFrom::Start(u64::from(offset))).unwrap();
            file.read_exact(&mut byte).unwrap();
            assert_eq!(byte[0], protect_byte(offset, 1215), "offset {offset}");
        }
    }
}

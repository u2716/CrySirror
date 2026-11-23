// GPL-2.0-or-later. Native v5 syscall policy from xsystem4 vm.c a85467d.
// (C) 2019 Nunuhara Cabbage; Rust (C) 2026 CrySirror contributors.
// Storage/capability IO and metadata are shared services, never ambient paths.
use crate::{
    adapter::Dispatch,
    bootstrap::Bootstrap,
    vm::{ExternalCall, ExternalTarget, Machine},
};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{ClientStorage, HostServices},
    state::ClientId,
    Error,
};
use crysirror_tasks::{Task, TaskPool};
struct Pending {
    owner: ClientId,
    call: ExternalCall,
    task: Task<bool>,
    owned_strings: usize,
}
pub(crate) struct NativeFiles {
    owner: ClientId,
    folder: String,
    folder_bytes: Vec<u8>,
    game_name: Option<Vec<u8>>,
    pool: TaskPool,
    pending: Option<Pending>,
}
impl NativeFiles {
    pub(crate) fn cancel_pending_for_shutdown(&mut self) {
        self.pending = None;
    }
    pub(crate) fn io_pool(&self) -> TaskPool {
        self.pool.clone()
    }
    pub(crate) fn hll_path(&self, raw: &[u8], default: Mount) -> Result<(Mount, String)> {
        let raw = raw.split(|byte| *byte == 0).next().unwrap();
        ensure!(!raw.is_empty() && raw.len() < 260, "System4 File native filename extent");
        let name = crysirror_resource::text_stream::decode_shift_jis(raw, 4096)?;
        resolve(&self.folder, &name, default)
    }
    pub(crate) fn save_path(&self, bytes: &[u8]) -> Result<(VfsPath, VfsPath)> {
        let bytes = bytes.split(|b| *b == 0).next().unwrap();
        ensure!(
            !bytes.is_empty() && bytes.len() < 260,
            "System4 MsgSkip native filename profile"
        );
        let name = crysirror_resource::text_stream::decode_shift_jis(bytes, 4096)?;
        let (mount, relative) = resolve(&self.folder, &name, Mount::Save)?;
        ensure!(
            mount == Mount::Save,
            "System4 native message history save mount"
        );
        let path = VfsPath::new(Mount::Save, &relative)?;
        let original = VfsPath::new(
            Mount::Asset,
            &format!("{}/{}", self.folder, path.relative()),
        )?;
        Ok((path, original))
    }
    pub(crate) fn new(owner: ClientId) -> Result<Self> {
        Ok(Self {
            owner,
            folder: "SaveData".into(),
            folder_bytes: b"SaveData".to_vec(),
            game_name: None,
            pool: TaskPool::new("system4-files", 1, 2)?,
            pending: None,
        })
    }
    pub(crate) fn configure(&mut self, bootstrap: &Bootstrap) -> Result<()> {
        ensure!(
            VfsPath::new(Mount::Save, &bootstrap.save_folder)?.relative() == bootstrap.save_folder,
            "System4 native save-root alias must be canonical"
        );
        ensure!(
            self.pending.is_none(),
            "System4 changed native root during file dependency"
        );
        let folder = cp932(&bootstrap.save_folder)?;
        let game = cp932(&bootstrap.game_name)?;
        self.folder = bootstrap.save_folder.clone();
        self.folder_bytes = folder;
        self.game_name = Some(game);
        Ok(())
    }
    pub(crate) fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        let ExternalTarget::System(code) = call.target else {
            ensure!(
                self.pending.is_none(),
                "System4 changed pending file syscall target"
            );
            return Ok(None);
        };
        if !matches!(code, 10 | 12 | 14 | 16 | 26 | 28) {
            ensure!(
                self.pending.is_none(),
                "System4 changed pending file syscall"
            );
            return Ok(None);
        }
        ensure!(owner == self.owner, "System4 foreign file service owner");
        machine.validate_system(call)?;
        if code == 12 {
            // Native SaveFolder is a logical alias for this client's engine
            // Save capability, not the ambient OS directory of another client.
            // The same alias is consumed by all native save-path operations.
            let _scope = host.storage(owner)?;
            machine.complete_system_bytes(call, self.folder_bytes.clone())?;
            return Ok(Some(Dispatch::Completed));
        }
        if code == 14 {
            machine.complete_system_bytes(
                call,
                self.game_name
                    .as_ref()
                    .context("System4 original GameName metadata unavailable")?
                    .clone(),
            )?;
            return Ok(Some(Dispatch::Completed));
        }
        if matches!(code, 26 | 28) && self.pending.is_none() {
            machine.validate_system_owned_strings(call, if code == 28 { 2 } else { 1 })?;
            let destination =
                machine
                    .heap
                    .string(machine.stack_value(if code == 28 { 1 } else { 0 })?)?;
            let (destination, _) = self.save_path(destination.as_slice())?;
            let source = if code == 28 {
                let source = machine.heap.string(machine.stack_value(0)?)?;
                Some(self.save_path(source.as_slice())?)
            } else {
                None
            };
            let storage = host.storage(owner)?;
            let task = self.pool.spawn(move |cancel| {
                cancel.check()?;
                let resolve = |path: &VfsPath| {
                    crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(
                        storage.as_ref(),
                        path,
                    )
                };
                let existing = resolve(&destination)?;
                if let Some((source, original)) = source {
                    let source = if let Some(path) = resolve(&source)? {
                        path
                    } else if let Some(path) = resolve(&original)? {
                        path
                    } else {
                        return Ok(false);
                    };
                    let reader = storage.open_read(&source)?;
                    let length = reader.len()?;
                    ensure!(length <= 64 * 1024 * 1024, "System4 save-copy byte budget");
                    let bytes = crysirror_resource::read_all(reader.as_ref(), length)?;
                    cancel.check()?;
                    // The source is fully read before publication, including a
                    // copy onto itself. Atomic writes belong to ClientStorage.
                    native_io_bool(
                        storage.write_atomic(existing.as_ref().unwrap_or(&destination), &bytes),
                    )
                } else {
                    let Some(path) = existing else {
                        return Ok(false);
                    };
                    cancel.check()?;
                    native_io_bool(storage.remove(&path))
                }
            })?;
            self.pending = Some(Pending {
                owner,
                call: call.clone(),
                task,
                owned_strings: if code == 28 { 2 } else { 1 },
            });
        }
        if self.pending.is_none() {
            let raw = machine.heap.string(machine.stack_value(0)?)?;
            let bytes = raw.split(|b| *b == 0).next().unwrap();
            if code == 10 && (bytes.is_empty() || bytes.ends_with(b"/") || bytes.ends_with(b"\\")) {
                machine.complete_system_owned_string(call, Some(0))?;
                return Ok(Some(Dispatch::Completed));
            }
            let path = crysirror_resource::text_stream::decode_shift_jis(bytes, 4096)?;
            let location = resolve(
                &self.folder,
                &path,
                if code == 16 {
                    Mount::Save
                } else {
                    Mount::Asset
                },
            )?;
            let storage = host.storage(owner)?;
            let folder = self.folder.clone();
            let task = self.pool.spawn(move |cancel| {
                cancel.check()?;
                let found = metadata_exists(storage.as_ref(), location.0, &location.1)?;
                let result = if !found && location.0 == Mount::Save && !location.1.is_empty() {
                    // Existing owner installs remain a read-only fallback;
                    // newly written client-scoped saves take precedence.
                    let original = VfsPath::new(Mount::Asset, &format!("{folder}/{}", location.1))?;
                    metadata_exists(storage.as_ref(), Mount::Asset, original.relative())?
                } else {
                    found
                };
                cancel.check()?;
                Ok(result)
            })?;
            self.pending = Some(Pending {
                owner,
                call: call.clone(),
                task,
                owned_strings: 1,
            });
        }
        let pending = self.pending.as_mut().unwrap();
        ensure!(
            pending.owner == owner && pending.call == *call,
            "System4 stale file metadata completion"
        );
        let Some(found) = pending.task.try_take()? else {
            return Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))));
        };
        machine.complete_system_owned_strings(
            call,
            pending.owned_strings,
            Some(i32::from(found)),
        )?;
        self.pending = None;
        Ok(Some(Dispatch::Completed))
    }
}
// Original file_copy/delete return false for ordinary filesystem failure;
// a missing host capability or violated validation budget remains a visible error.
fn native_io_bool(result: crysirror_core::Result<()>) -> Result<bool> {
    match result {
        Ok(()) => Ok(true),
        Err(Error::NotFound | Error::ReadOnly | Error::Io(_)) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn cp932(text: &str) -> Result<Vec<u8>> {
    let (bytes, _, errors) = encoding_rs::SHIFT_JIS.encode(text);
    ensure!(
        !errors,
        "System4 native metadata cannot be represented as CP932"
    );
    Ok(bytes.into_owned())
}
fn resolve(folder: &str, name: &str, default: Mount) -> Result<(Mount, String)> {
    let name = name.replace('\\', "/");
    let name = if name.is_empty() {
        name
    } else {
        VfsPath::new(default, &name)?.relative().to_owned()
    };
    if name.eq_ignore_ascii_case(folder) {
        return Ok((Mount::Save, String::new()));
    }
    if name
        .get(..folder.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(folder))
        && name.as_bytes().get(folder.len()) == Some(&b'/')
    {
        let relative = &name[folder.len() + 1..];
        return Ok((
            Mount::Save,
            VfsPath::new(Mount::Save, relative)?.relative().to_owned(),
        ));
    }
    Ok((default, name))
}
fn metadata_exists(storage: &dyn ClientStorage, mount: Mount, name: &str) -> Result<bool> {
    let resolved = if name.is_empty() {
        None
    } else {
        let requested = VfsPath::new(mount, name)?;
        let Some(path) = crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(
            storage, &requested,
        )?
        else {
            return Ok(false);
        };
        Some(path)
    };
    Ok(
        match storage.metadata(mount, resolved.as_ref().map_or("", |p| p.relative())) {
            Ok(value) => value.is_some(),
            Err(Error::NotFound) => false,
            Err(error) => return Err(error.into()),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ain::{Function, Program},
        vm::{heap::Object, Step},
    };
    use crysirror_core::{
        service::{FileSystem, StorageMetadata},
        state::Command,
    };
    use std::{
        collections::{BTreeMap, BTreeSet},
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::{Duration, Instant},
    };
    struct Storage {
        owner: std::thread::ThreadId,
        calls: AtomicUsize,
        entries: BTreeMap<(Mount, String), bool>,
    }
    impl Storage {
        fn offthread(&self) {
            assert_ne!(std::thread::current().id(), self.owner);
            self.calls.fetch_add(1, Ordering::Relaxed);
        }
    }
    impl ClientStorage for Storage {
        fn metadata(
            &self,
            mount: Mount,
            name: &str,
        ) -> crysirror_core::Result<Option<StorageMetadata>> {
            self.offthread();
            Ok(self
                .entries
                .get(&(mount, name.into()))
                .map(|directory| StorageMetadata {
                    is_directory: *directory,
                    size: if *directory { None } else { Some(3) },
                    created_ms: None,
                    modified_ms: None,
                    accessed_ms: None,
                }))
        }
        fn list(&self, mount: Mount, name: &str) -> crysirror_core::Result<Vec<String>> {
            self.offthread();
            let prefix = if name.is_empty() {
                String::new()
            } else {
                format!("{name}/")
            };
            Ok(self
                .entries
                .keys()
                .filter(|(m, _)| *m == mount)
                .filter_map(|(_, n)| n.strip_prefix(&prefix))
                .filter(|n| !n.is_empty())
                .map(|n| n.split('/').next().unwrap().to_string())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect())
        }
        fn read(&self, _: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
            panic!("presence query must not read a payload")
        }
        fn write_atomic(&self, _: &VfsPath, _: &[u8]) -> crysirror_core::Result<()> {
            Err(Error::ReadOnly)
        }
        fn remove(&self, _: &VfsPath) -> crysirror_core::Result<()> {
            Err(Error::ReadOnly)
        }
    }
    struct Host {
        storage: Arc<Storage>,
    }
    impl FileSystem for Host {
        fn read(&mut self, _: ClientId, _: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
            panic!("owner IO")
        }
        fn write_atomic(
            &mut self,
            _: ClientId,
            _: &VfsPath,
            _: &[u8],
        ) -> crysirror_core::Result<()> {
            panic!("owner IO")
        }
    }
    impl HostServices for Host {
        fn submit(&mut self, _: ClientId, _: &[Command]) -> crysirror_core::Result<()> {
            Err(Error::Unsupported("test scene"))
        }
        fn now_ms(&self) -> u64 {
            500
        }
        fn storage(&mut self, owner: ClientId) -> crysirror_core::Result<Arc<dyn ClientStorage>> {
            if owner != ClientId(1) {
                return Err(Error::NotFound);
            }
            Ok(self.storage.clone())
        }
    }
    fn host() -> Host {
        Host {
            storage: Arc::new(Storage {
                owner: std::thread::current().id(),
                calls: AtomicUsize::new(0),
                entries: BTreeMap::from([
                    ((Mount::Asset, "".into()), true),
                    ((Mount::Save, "".into()), true),
                    ((Mount::Asset, "Folder".into()), true),
                    ((Mount::Asset, "Folder/Music.DAT".into()), false),
                    ((Mount::Save, "NEW.sav".into()), false),
                    ((Mount::Asset, "SaveData".into()), true),
                    ((Mount::Asset, "SaveData/old.sav".into()), false),
                ]),
            }),
        }
    }
    fn pending(code: i32, name: Option<&[u8]>) -> (Machine, ExternalCall) {
        let mut bytes = 0x63u16.to_le_bytes().to_vec();
        bytes.extend(code.to_le_bytes());
        bytes.extend(0x2fu16.to_le_bytes());
        let mut m = Machine::new(Arc::new(Program {
            version: 5,
            code: bytes,
            functions: vec![Function {
                address: 0,
                name: b"main".to_vec(),
                is_label: false,
                return_kind: 0,
                return_structure: -1,
                arguments: 0,
                variables: vec![],
                crc: 0,
            }],
            ..Default::default()
        }))
        .unwrap();
        if let Some(name) = name {
            let slot = m
                .heap
                .alloc(Object::String(Arc::new(name.to_vec())))
                .unwrap();
            m.stack.push(slot);
        }
        for _ in 0..100 {
            if let Step::External(call) = m.step().unwrap() {
                return (m, call);
            }
        }
        panic!("missing native syscall")
    }
    fn finish(
        files: &mut NativeFiles,
        m: &mut Machine,
        call: &ExternalCall,
        host: &mut Host,
    ) -> Result<()> {
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            match files.dispatch(ClientId(1), m, call, host)? {
                Some(Dispatch::Completed) => return Ok(()),
                Some(Dispatch::WaitUntil(_)) => {
                    ensure!(
                        m.pc == call.pc,
                        "file syscall advanced before metadata receipt"
                    );
                    ensure!(Instant::now() < until, "metadata timeout");
                    std::thread::yield_now();
                }
                _ => anyhow::bail!("native file operation unhandled"),
            }
        }
    }
    #[test]
    fn configured_native_save_alias_and_game_name_are_owned_strings_not_host_paths() {
        let mut files = NativeFiles::new(ClientId(1)).unwrap();
        files
            .configure(&Bootstrap {
                program: "native.ain".into(),
                canvas: [800, 600],
                game_name: "戦国ランス".into(),
                save_folder: "SaveData".into(),
                native_vm: Some("Sys42VM".into()),
                joypad: false,
                mixers: vec![],
            })
            .unwrap();
        let mut host = host();
        for (code, expected) in [
            (12, b"SaveData".to_vec()),
            (14, cp932("戦国ランス").unwrap()),
        ] {
            let (mut m, call) = pending(code, None);
            m.stack.push(77);
            finish(&mut files, &mut m, &call, &mut host).unwrap();
            assert_eq!(m.stack[0], 77);
            assert_eq!(&**m.heap.string(m.stack[1]).unwrap(), &expected);
        }
        assert_eq!(host.storage.calls.load(Ordering::Relaxed), 0);
    }
    #[test]
    fn native_file_queries_use_shared_case_lookup_on_worker_and_retire_owned_name() {
        let mut files = NativeFiles::new(ClientId(1)).unwrap();
        let mut host = host();
        for (code, name, expected) in [
            (10, b"folder\\music.dat".as_slice(), 1),
            (10, b"Folder/", 0),
            (10, b"Folder", 1),
            (16, b"savedata\\new.SAV", 1),
            (16, b"old.sav", 1),
            (16, b"absent", 0),
            (16, b"", 1),
        ] {
            let (mut m, call) = pending(code, Some(name));
            let slot = m.stack[0];
            finish(&mut files, &mut m, &call, &mut host).unwrap();
            assert_eq!(m.stack, [expected]);
            assert!(m.heap.slot(slot).is_err());
        }
        assert!(host.storage.calls.load(Ordering::Relaxed) > 0);
    }
    #[test]
    fn native_paths_cannot_escape_mount_or_complete_for_foreign_owner() {
        let mut files = NativeFiles::new(ClientId(1)).unwrap();
        let mut host = host();
        for name in [
            b"SaveData\\..\\other".as_slice(),
            b"C:\\elsewhere",
            b"/outside",
        ] {
            let (mut m, call) = pending(16, Some(name));
            let stack = m.stack.clone();
            assert!(files
                .dispatch(ClientId(1), &mut m, &call, &mut host)
                .is_err());
            assert_eq!(m.pc, call.pc);
            assert_eq!(m.stack, stack);
            assert!(files.pending.is_none());
        }
        let (mut m, call) = pending(12, None);
        assert!(files
            .dispatch(ClientId(2), &mut m, &call, &mut host)
            .is_err());
        assert_eq!(m.pc, call.pc);
    }
}

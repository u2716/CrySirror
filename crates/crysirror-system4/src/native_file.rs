// Copyright (C)2019 Nunuhara Cabbage; Rust (C)2026 CrySirror contributors.
// GPL-2.0-or-later. Complete xsystem4 File.c a85467d and original Rance7 File.dll
// exports/helpers10001019..10002AFc statically inspected, never executed.
//! One native File object, with unique VM/state ownership on a worker.
//! Native type/cursor/bitflag policy lives here; tree operations, DOS matching,
//! filesystem authority and calendar conversion remain shared capabilities.
mod codec;
use crate::{
    adapter::Dispatch,
    native_files::NativeFiles,
    vm::{
        ExternalCall, ExternalTarget, Machine,
        external::{Argument, HllReturn},
    },
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    Error,
    path::{Mount, VfsPath},
    service::{CivilTime, CivilTimeQuery, CivilTimeState, ClientStorage, HostServices},
    state::ClientId,
};
use crysirror_tasks::{Task, TaskPool};
use std::{
    io::{Read, Write},
    sync::Arc,
};
const LIMIT: usize = 64 * 1024 * 1024;
#[derive(Clone)]
struct Stream {
    path: VfsPath,
    flags: i32,
    bytes: Arc<Vec<u8>>,
    cursor: usize,
    dirty: bool,
}
#[derive(Clone, Default)]
struct State {
    stream: Option<Stream>,
    find: Option<Find>,
}
#[derive(Clone)]
struct Find {
    mount: Mount,
    directory: String,
    original_directory: String,
    pattern: String,
    entries: Option<Arc<Vec<crysirror_resource::capability_tree::TreeEntry>>>,
    cursor: usize,
}
#[derive(Clone)]
struct Location {
    writable: VfsPath,
    original: VfsPath,
}
enum Request {
    False,
    Open(Option<Location>, i32),
    Close,
    Read(i32),
    Write(i32),
    Copy(Location, Location),
    Delete(Location),
    GetTime(Location, i32, i32),
    SetFind(Mount, String, String, String),
    GetFind(i32),
    MakeDirectory(Mount, String),
}
struct Completed {
    machine: Machine,
    state: State,
    error: Option<String>,
}
struct Clock {
    call: ExternalCall,
    query: Box<dyn CivilTimeQuery>,
}
struct Flushed {
    state: State,
    error: Option<String>,
}
pub(crate) struct NativeFile {
    owner: ClientId,
    pool: TaskPool,
    state: Option<State>,
    job: Option<Task<Completed>>,
    clock: Option<Clock>,
    flush: Option<Task<Flushed>>,
}
impl NativeFile {
    pub(crate) fn new(owner: ClientId, pool: TaskPool) -> Self {
        Self {
            owner,
            pool,
            state: Some(State::default()),
            job: None,
            clock: None,
            flush: None,
        }
    }
    pub(crate) fn handles(machine: &Machine, call: &ExternalCall) -> bool {
        match call.target {
            ExternalTarget::Hll { library, .. } => {
                machine.program.libraries[library].name == b"File"
            }
            _ => false,
        }
    }
    pub(crate) fn has_worker(&self) -> bool {
        self.job.is_some()
    }
    pub(crate) fn has_pending(&self) -> bool {
        self.job.is_some() || self.clock.is_some() || self.flush.is_some()
    }
    pub(crate) fn needs_close(&self) -> bool {
        self.has_pending()
            || self
                .state
                .as_ref()
                .is_some_and(|state| state.stream.is_some())
    }
    pub(crate) fn cancel(&mut self) {
        self.job = None;
        self.clock = None;
        self.flush = None;
        self.state = Some(State::default());
    }
    pub(crate) fn poll(&mut self, machine: &mut Option<Machine>) -> Result<bool> {
        let Some(task) = self.job.as_mut() else {
            return Ok(true);
        };
        let Some(done) = task.try_take()? else {
            return Ok(false);
        };
        ensure!(machine.is_none(), "System4 File worker VM ownership");
        self.job = None;
        self.state = Some(done.state);
        *machine = Some(done.machine);
        if let Some(error) = done.error {
            anyhow::bail!("System4 File: {error}");
        }
        Ok(true)
    }
    pub(crate) fn close(&mut self, host: &mut dyn HostServices) -> Result<Dispatch> {
        self.clock = None;
        if self.has_worker() {
            return Ok(Dispatch::WaitUntil(host.now_ms().saturating_add(1)));
        }
        if let Some(task) = self.flush.as_mut() {
            let Some(done) = task.try_take()? else {
                return Ok(Dispatch::WaitUntil(host.now_ms().saturating_add(1)));
            };
            self.flush = None;
            self.state = Some(done.state);
            if let Some(error) = done.error {
                anyhow::bail!("System4 File shutdown: {error}");
            }
            return Ok(Dispatch::Completed);
        }
        if self
            .state
            .as_ref()
            .is_none_or(|state| state.stream.is_none())
        {
            return Ok(Dispatch::Completed);
        }
        let storage = host.storage(self.owner)?;
        let permit = self.pool.reserve()?;
        let mut state = self.state.take().unwrap();
        self.flush = Some(permit.spawn(move |cancel| {
            let result = close(&mut state, storage.as_ref(), &|| cancel.check());
            let error = match result {
                Ok(1) => None,
                Ok(_) => Some("native file close write failed".into()),
                Err(error) => Some(format!("{error:#}")),
            };
            Ok(Flushed { state, error })
        })?);
        Ok(Dispatch::WaitUntil(host.now_ms().saturating_add(1)))
    }
    pub(crate) fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Option<Machine>,
        call: &ExternalCall,
        files: &NativeFiles,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        let m = machine.as_ref().context("System4 File missing VM")?;
        if !Self::handles(m, call) {
            ensure!(!self.has_pending(), "System4 changed pending File target");
            return Ok(None);
        }
        ensure!(
            owner == self.owner && !self.has_worker() && self.flush.is_none(),
            "System4 File owner/worker boundary"
        );
        let ExternalTarget::Hll { library, function } = call.target else {
            unreachable!()
        };
        let function = &m.program.libraries[library].functions[function];
        ensure!(function.return_kind == 10, "System4 File return ABI");
        let name = function.name.as_slice();
        let args = m.hll_arguments(call)?;
        let abi = match name {
            b"Open" => matches!(
                args.as_slice(),
                [Argument::String { .. }, Argument::Scalar { kind: 10, .. }]
            ),
            b"Close" => args.is_empty(),
            b"Read" => matches!(args.as_slice(), [Argument::HeapReference { kind: 21, .. }]),
            b"Write" => matches!(args.as_slice(), [Argument::Page { kind: 13, .. }]),
            b"Copy" => matches!(
                args.as_slice(),
                [Argument::String { .. }, Argument::String { .. }]
            ),
            b"Delete" | b"SetFind" | b"MakeDirectory" => {
                matches!(args.as_slice(), [Argument::String { .. }])
            }
            b"GetTime" => matches!(
                args.as_slice(),
                [
                    Argument::String { .. },
                    Argument::HeapReference { kind: 21, .. }
                ]
            ),
            b"GetFind" => matches!(args.as_slice(), [Argument::HeapReference { kind: 20, .. }]),
            _ => false,
        };
        ensure!(
            abi,
            "System4 unsupported File export/ABI: {}",
            String::from_utf8_lossy(name)
        );
        ensure!(
            !args.is_empty() || m.stack.len() < 65536,
            "System4 File result stack budget"
        );
        let mut offset = 0;
        if name == b"GetTime" && !args.iter().any(|arg|matches!(arg,Argument::String{bytes,..} if bytes.first().is_none_or(|b|*b==0))) {
            ensure!(matches!(args.as_slice(),[Argument::String{..},Argument::HeapReference{kind:21,..}]),"System4 File.GetTime argument ABI");
            if self.clock.is_none() {
                let query = host.begin_civil_time(owner)?;
                ensure!(query.client() == owner, "System4 File foreign civil query");
                self.clock = Some(Clock {
                    call: call.clone(),
                    query,
                });
            }
            let pending = self.clock.as_mut().unwrap();
            ensure!(pending.call == *call, "System4 File stale civil receipt");
            match pending.query.poll()? {
                CivilTimeState::Pending => {
                    return Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))));
                }
                CivilTimeState::Failed(error) => anyhow::bail!("System4 File local clock: {error}"),
                CivilTimeState::Ready(sample) => {
                    ensure!(sample.client == owner, "System4 File foreign civil result");
                    offset = sample.utc_offset_seconds;
                }
            }
        } else {
            ensure!(self.clock.is_none(), "System4 changed File civil request");
        }
        let location = |bytes: &[u8]| -> Result<Location> {
            let (mount, name) = files.hll_path(bytes, Mount::Save)?;
            ensure!(
                mount == Mount::Save && !name.is_empty(),
                "System4 File writable capability root"
            );
            let raw = bytes.split(|b| *b == 0).next().unwrap();
            let raw =
                crysirror_resource::text_stream::decode_shift_jis(raw, 4096)?.replace('\\', "/");
            Ok(Location {
                writable: VfsPath::new(Mount::Save, &name)?,
                original: VfsPath::new(Mount::Asset, &raw)?,
            })
        };
        let empty_string = args.iter().any(
            |arg| matches!(arg,Argument::String{bytes,..} if bytes.first().is_none_or(|b|*b==0)),
        );
        let request = if empty_string && name != b"Open" {
            Request::False
        } else {
            match (name, args.as_slice()) {
                (
                    b"Open",
                    [
                        Argument::String { bytes, .. },
                        Argument::Scalar { kind: 10, bits },
                    ],
                ) => Request::Open(
                    if bytes.first().is_none_or(|b| *b == 0) {
                        None
                    } else {
                        Some(location(bytes)?)
                    },
                    *bits,
                ),
                (b"Close", []) => Request::Close,
                (b"Read", [Argument::HeapReference { kind: 21, slot }]) => Request::Read(*slot),
                (b"Write", [Argument::Page { kind: 13, slot }]) => Request::Write(*slot),
                (
                    b"Copy",
                    [
                        Argument::String { bytes: a, .. },
                        Argument::String { bytes: b, .. },
                    ],
                ) => Request::Copy(location(a)?, location(b)?),
                (b"Delete", [Argument::String { bytes, .. }]) => Request::Delete(location(bytes)?),
                (
                    b"GetTime",
                    [
                        Argument::String { bytes, .. },
                        Argument::HeapReference { kind: 21, slot },
                    ],
                ) => Request::GetTime(location(bytes)?, *slot, offset),
                (b"SetFind", [Argument::String { bytes, .. }]) => {
                    let raw = bytes.split(|b| *b == 0).next().unwrap();
                    ensure!(
                        !raw.is_empty() && raw.len() < 260,
                        "System4 File find name extent"
                    );
                    let name = crysirror_resource::text_stream::decode_shift_jis(raw, 4096)?
                        .replace('\\', "/");
                    let (directory, pattern) = name.rsplit_once('/').unwrap_or(("", name.as_str()));
                    let original_directory = directory.to_owned();
                    let (mount, directory) = if directory.is_empty() {
                        (Mount::Save, String::new())
                    } else {
                        let (bytes, _, errors) = encoding_rs::SHIFT_JIS.encode(directory);
                        ensure!(!errors, "System4 File directory encoding");
                        files.hll_path(&bytes, Mount::Save)?
                    };
                    Request::SetFind(mount, directory, original_directory, pattern.to_owned())
                }
                (b"GetFind", [Argument::HeapReference { kind: 20, slot }]) => {
                    Request::GetFind(*slot)
                }
                (b"MakeDirectory", [Argument::String { bytes, .. }]) => {
                    let (mount, name) = files.hll_path(bytes, Mount::Save)?;
                    Request::MakeDirectory(mount, name)
                }
                _ => anyhow::bail!(
                    "System4 unsupported File export/ABI: {}",
                    String::from_utf8_lossy(name)
                ),
            }
        };
        let storage = host.storage(owner)?;
        let permit = self.pool.reserve()?;
        let original = machine.take().unwrap();
        let state = self.state.take().context("System4 File state ownership")?;
        let call = call.clone();
        self.job = Some(permit.spawn(move |cancel| {
            let mut candidate = original.clone();
            let mut staged = state.clone();
            let result = (|| -> Result<()> {
                cancel.check()?;
                let value = execute(
                    &mut candidate,
                    &mut staged,
                    request,
                    storage.as_ref(),
                    owner,
                    &|| cancel.check(),
                )?;
                candidate.complete_hll(&call, HllReturn::Scalar(value))?;
                cancel.check()?;
                Ok(())
            })();
            Ok(match result {
                Ok(()) => Completed {
                    machine: candidate,
                    state: staged,
                    error: None,
                },
                Err(error) => Completed {
                    machine: original,
                    state,
                    error: Some(format!("{error:#}")),
                },
            })
        })?);
        self.clock = None;
        Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))))
    }
}
fn resolve(storage: &dyn ClientStorage, path: &VfsPath) -> Result<Option<VfsPath>> {
    crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(storage, path)
}
fn read_path(storage: &dyn ClientStorage, location: &Location) -> Result<Option<VfsPath>> {
    if let Some(path) = resolve(storage, &location.writable)? {
        return Ok(Some(path));
    }
    resolve(storage, &location.original)
}
fn ordinary<T>(result: crysirror_core::Result<T>) -> Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(Error::NotFound | Error::ReadOnly | Error::Io(_)) => Ok(None),
        Err(error) => Err(error.into()),
    }
}
fn bool_io(result: Result<()>) -> Result<i32> {
    match result {
        Ok(()) => Ok(1),
        Err(error)
            if error
                .downcast_ref::<Error>()
                .is_some_and(|e| matches!(e, Error::NotFound | Error::ReadOnly | Error::Io(_))) =>
        {
            Ok(0)
        }
        Err(error) => Err(error),
    }
}
fn tree_io(result: Result<crysirror_resource::capability_tree::TreeWorkReport>) -> Result<i32> {
    match result {
        Ok(report) => Ok(i32::from(report.complete)),
        Err(error) => bool_io(Err(error)),
    }
}
fn close(
    state: &mut State,
    storage: &dyn ClientStorage,
    check: &dyn Fn() -> Result<()>,
) -> Result<i32> {
    let Some(stream) = state.stream.take() else {
        return Ok(0);
    };
    if stream.flags & 2 != 0 && stream.dirty {
        check()?;
        let mut zlib = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        zlib.write_all(&stream.bytes)?;
        let encoded = zlib.finish()?;
        let mut bytes = Vec::with_capacity(4 + encoded.len());
        bytes.extend_from_slice(&(stream.bytes.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&encoded);
        let target = resolve(storage, &stream.path)?.unwrap_or(stream.path);
        check()?;
        return bool_io(
            storage
                .write_atomic_existing_parent(&target, &bytes)
                .map_err(Into::into),
        );
    }
    Ok(1)
}
fn execute(
    m: &mut Machine,
    state: &mut State,
    request: Request,
    storage: &dyn ClientStorage,
    owner: ClientId,
    check: &dyn Fn() -> Result<()>,
) -> Result<i32> {
    use crysirror_resource::capability_tree as tree;
    check()?;
    Ok(match request {
        Request::False => 0,
        Request::Open(location, flags) => {
            close(state, storage, check)?;
            let Some(location) = location else {
                return Ok(0);
            };
            let bytes = if flags & 1 != 0 {
                let Some(path) = read_path(storage, &location)? else {
                    return Ok(0);
                };
                let Some(reader) = ordinary(storage.open_read(&path))? else {
                    return Ok(0);
                };
                ensure!(
                    reader.len()? <= LIMIT as u64,
                    "System4 File input byte budget"
                );
                let data = crysirror_resource::read_all(reader.as_ref(), LIMIT as u64)?;
                check()?;
                if data.len() < 4 {
                    return Ok(0);
                };
                let size = u32::from_le_bytes(data[..4].try_into().unwrap()) as usize;
                ensure!(size <= LIMIT, "System4 File inflated byte budget");
                let mut output = Vec::new();
                let mut decoder = flate2::read::ZlibDecoder::new(&data[4..]).take(size as u64 + 1);
                if decoder.read_to_end(&mut output).is_err() || output.len() != size {
                    return Ok(0);
                };
                output
            } else {
                // Original allocates one uninitialized byte, even if Write is
                // given an empty structure. Initialize it so no host memory can
                // enter a native file; all subsequent bytes preserve its ABI.
                vec![0]
            };
            state.stream = Some(Stream {
                path: location.writable,
                flags,
                bytes: Arc::new(bytes),
                cursor: 0,
                dirty: false,
            });
            1
        }
        Request::Close => close(state, storage, check)?,
        Request::Read(slot) => {
            let Some(stream) = state.stream.as_mut() else {
                return Ok(0);
            };
            if slot == -1 {
                return Ok(0);
            };
            i32::from(codec::transfer(m, slot, stream, true, check)?)
        }
        Request::Write(slot) => {
            let Some(stream) = state.stream.as_mut() else {
                return Ok(0);
            };
            stream.dirty = true;
            if slot == -1 {
                return Ok(0);
            };
            i32::from(codec::transfer(m, slot, stream, false, check)?)
        }
        Request::Copy(from, to) => {
            if from
                .writable
                .relative()
                .eq_ignore_ascii_case(to.writable.relative())
            {
                return Ok(0);
            }
            let Some(source) = read_path(storage, &from)? else {
                return Ok(0);
            };
            tree_io(tree::copy_with_policy(
                storage,
                &source,
                &to.writable,
                tree::TreeLimits::default(),
                tree::TreeCopyPolicy {
                    preserve_metadata: true,
                    replace_read_only: false,
                    merge_directories: false,
                    continue_on_io_error: true,
                },
                check,
            ))?
        }
        Request::Delete(location) => {
            // Destructive operations never fall through to the original Asset
            // installation when the scoped Save overlay has no such file.
            let Some(path) = resolve(storage, &location.writable)? else {
                return Ok(0);
            };
            tree_io(tree::remove_with_policy(
                storage,
                &path,
                tree::TreeLimits::default(),
                true,
                true,
                check,
            ))?
        }
        Request::GetTime(location, slot, offset) => {
            let Some(path) = read_path(storage, &location)? else {
                return Ok(0);
            };
            let Some(meta) = storage.metadata(path.mount(), path.relative())? else {
                return Ok(0);
            };
            if meta.is_directory {
                return Ok(0);
            };
            let Some(time) = meta.modified_ms else {
                return Ok(0);
            };
            // FileTimeToLocalFileTime uses the current host offset, not a new
            // historical CRT rule inferred from the timestamp's calendar year.
            let time =
                CivilTime::from_unix_ms(owner, time, offset).context("System4 File time extent")?;
            let fields = [
                time.year,
                i32::from(time.month),
                i32::from(time.day),
                i32::from(time.hour),
                i32::from(time.minute),
                i32::from(time.second),
                i32::from(time.weekday().context("System4 File weekday")?),
            ];
            let page = m.heap.page(slot)?;
            let crate::vm::heap::PageKind::Structure(index) = page.kind else {
                anyhow::bail!("System4 File.GetTime structure ABI")
            };
            let kinds = m.program.structures[index]
                .members
                .iter()
                .take(page.values.len().min(7))
                .map(|v| v.kind)
                .collect::<Vec<_>>();
            let mut next = 0;
            for (at, kind) in kinds.into_iter().enumerate() {
                if kind == 10 {
                    m.heap.set(slot, at as i32, fields[next])?;
                    next += 1;
                }
            }
            1
        }
        Request::SetFind(mount, directory, original_directory, pattern) => {
            state.find = Some(Find {
                mount,
                directory,
                original_directory,
                pattern,
                entries: None,
                cursor: 0,
            });
            1
        }
        Request::GetFind(slot) => {
            let Some(find) = state.find.as_mut() else {
                return Ok(0);
            };
            if find.entries.is_none() {
                let mut entries = Vec::new();
                let mut seen = std::collections::BTreeSet::new();
                for (mount, directory) in [
                    (find.mount, find.directory.as_str()),
                    (Mount::Asset, find.original_directory.as_str()),
                ] {
                    let directory = if directory.is_empty() {
                        String::new()
                    } else {
                        let Some(path) = resolve(storage, &VfsPath::new(mount, directory)?)? else {
                            continue;
                        };
                        path.relative().to_owned()
                    };
                    let mut within = std::collections::BTreeSet::new();
                    for entry in tree::find(
                        storage,
                        mount,
                        &directory,
                        &find.pattern,
                        tree::TreeLimits::default(),
                        check,
                    )? {
                        let key = entry
                            .path
                            .relative()
                            .rsplit('/')
                            .next()
                            .unwrap()
                            .to_ascii_lowercase();
                        ensure!(
                            within.insert(key.clone()),
                            "System4 File ambiguous case-insensitive find name"
                        );
                        if seen.insert(key) {
                            entries.push(entry);
                        }
                        ensure!(entries.len() <= 65536, "System4 File find entry budget");
                    }
                }
                find.entries = Some(Arc::new(entries));
            }
            let Some(entry) = find.entries.as_ref().unwrap().get(find.cursor) else {
                return Ok(0);
            };
            let name = entry.path.relative().rsplit('/').next().unwrap();
            let (bytes, _, errors) = encoding_rs::SHIFT_JIS.encode(name);
            ensure!(!errors, "System4 File find name not representable in CP932");
            m.heap.set_string(slot, Arc::new(bytes.into_owned()))?;
            find.cursor += 1;
            if entry.metadata.is_directory { 2 } else { 1 }
        }
        Request::MakeDirectory(mount, name) => {
            ensure!(
                mount == Mount::Save,
                "System4 File directory write authority"
            );
            if name.is_empty() {
                return Ok(i32::from(
                    storage
                        .metadata(mount, "")?
                        .is_some_and(|info| info.is_directory),
                ));
            }
            let path = VfsPath::new(mount, &name)?;
            if let Some(existing) = resolve(storage, &path)? {
                return Ok(i32::from(
                    storage
                        .metadata(mount, existing.relative())?
                        .is_some_and(|info| info.is_directory),
                ));
            }
            bool_io(storage.create_directory(&path).map_err(Into::into))?
        }
    })
}

// Copyright (C)2019 Nunuhara Cabbage; Rust (C)2026 CrySirror contributors.
// GPL-2.0-or-later. Full pinned MsgLogManager.c/MsgLogViewer.c plus authorized
// original DLL static class methods (SOURCE.json). Public warning stubs and
// simplified viewer voice/separator defaults are not used as implementations.
//! Native message tags and VM fields over shared bounded record sequences.
use crate::{
    adapter::Dispatch,
    native_files::NativeFiles,
    vm::{
        external::{Argument, HllReturn},
        heap::PageKind,
        ExternalCall, ExternalTarget, Machine,
    },
};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{ClientStorage, HostServices},
    state::ClientId,
    Error,
};
use crysirror_resource::{
    record_sequence::{ByteRecord, RecordSequence, RecordSequenceLimits},
    DecodeContext,
};
use crysirror_tasks::{Cancellation, Task, TaskPool};
use std::sync::{
    atomic::{AtomicI32, Ordering},
    Arc,
};
static NEXT_INTERFACE: AtomicI32 = AtomicI32::new(1);
#[derive(Clone)]
struct State {
    owner: ClientId,
    interface: i32,
    manager: RecordSequence,
    viewer: RecordSequence,
    line_max: i32,
    line_count: i32,
}
impl State {
    fn new(owner: ClientId) -> Result<Self> {
        let interface = NEXT_INTERFACE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
                n.checked_add(1).filter(|n| *n > 0)
            })
            .map_err(|_| anyhow::anyhow!("System4 message interface identity budget"))?;
        Ok(Self {
            owner,
            interface,
            manager: RecordSequence::new(owner, RecordSequenceLimits::default())?,
            viewer: RecordSequence::new(owner, RecordSequenceLimits::default())?,
            line_max: 1000,
            line_count: 0,
        })
    }
    fn add(&mut self, record: ByteRecord, cancel: &Cancellation) -> Result<()> {
        let boundary = line_boundary(record.tag);
        self.manager.push(self.owner, record)?;
        if boundary {
            self.line_count = self.line_count.wrapping_add(1);
        }
        while self.manager.len(self.owner)? > 0 && self.line_count > self.line_max {
            cancel.check()?;
            if let Some(record) = self.manager.pop_front(self.owner)? {
                if line_boundary(record.tag) {
                    self.line_count = self.line_count.wrapping_sub(1);
                }
            }
        }
        Ok(())
    }
}
pub(crate) struct Completed {
    pub machine: Machine,
    pub error: Option<String>,
    state: State,
}
pub(crate) struct NativeLogs {
    owner: ClientId,
    pool: TaskPool,
    state: Option<State>,
    task: Option<Task<Completed>>,
}
impl NativeLogs {
    pub(crate) fn new(owner: ClientId, pool: TaskPool) -> Result<Self> {
        Ok(Self {
            owner,
            pool,
            state: Some(State::new(owner)?),
            task: None,
        })
    }
    pub(crate) fn handles(machine: &Machine, call: &ExternalCall) -> bool {
        let ExternalTarget::Hll { library, .. } = call.target else {
            return false;
        };
        matches!(
            machine.program.libraries[library].name.as_slice(),
            b"MsgLogManager" | b"MsgLogViewer"
        )
    }
    pub(crate) fn has_pending(&self) -> bool {
        self.task.is_some()
    }
    pub(crate) fn cancel(&mut self) {
        self.task = None;
    }
    pub(crate) fn poll(&mut self, machine: &mut Option<Machine>) -> Result<bool> {
        let Some(task) = &mut self.task else {
            return Ok(false);
        };
        let Some(done) = task.try_take()? else {
            return Ok(false);
        };
        ensure!(machine.is_none(), "System4 duplicate message worker VM");
        self.task = None;
        self.state = Some(done.state);
        *machine = Some(done.machine);
        if let Some(error) = done.error {
            anyhow::bail!("{error}");
        }
        Ok(true)
    }
    pub(crate) fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Option<Machine>,
        call: &ExternalCall,
        files: &NativeFiles,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        let m = machine
            .as_ref()
            .context("System4 message worker VM absent")?;
        if !Self::handles(m, call) {
            return Ok(None);
        }
        ensure!(
            owner == self.owner && self.task.is_none() && self.state.is_some(),
            "System4 message worker ownership"
        );
        let (library, name, args) = signature(m, call)?;
        let paths = if library == b"MsgLogManager" && matches!(name.as_slice(), b"Save" | b"Load") {
            let Argument::String { bytes, .. } = &args[0] else {
                unreachable!()
            };
            let (mount, relative) = files.hll_path(bytes, Mount::Save)?;
            ensure!(
                mount == Mount::Save && !relative.is_empty(),
                "System4 message writable path"
            );
            let destination = VfsPath::new(Mount::Save, &relative)?;
            let (mount, relative) = files.hll_path(bytes, Mount::Asset)?;
            let original = if mount == Mount::Save {
                files.save_path(bytes)?.1
            } else {
                VfsPath::new(mount, &relative)?
            };
            Some((destination, original, host.storage(owner)?))
        } else {
            None
        };
        let permit = self.pool.reserve()?;
        // All owner preflight/reservation precedes moving either unique state.
        let m = machine.take().unwrap();
        let state = self.state.take().unwrap();
        let call = call.clone();
        self.task = Some(
            permit.spawn(move |cancel| Ok(run(m, state, call, library, name, paths, cancel)))?,
        );
        Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))))
    }
}
fn signature(m: &Machine, call: &ExternalCall) -> Result<(Vec<u8>, Vec<u8>, Vec<Argument>)> {
    let ExternalTarget::Hll { library, function } = call.target else {
        anyhow::bail!("System4 log HLL expected")
    };
    let lib = &m.program.libraries[library];
    let f = &lib.functions[function];
    let args = m.hll_arguments(call)?;
    let scalar = match (lib.name.as_slice(), f.name.as_slice(), args.as_slice()) {
        (b"MsgLogManager", b"Numof" | b"GetInterface", []) => true,
        (b"MsgLogManager", b"GetType" | b"GetInt", [Argument::Scalar { kind: 10, .. }]) => true,
        (b"MsgLogManager", b"Save" | b"Load", [Argument::String { .. }]) => true,
        (
            b"MsgLogManager",
            b"AddInt",
            [Argument::Scalar { kind: 10, .. }, Argument::Scalar { kind: 10, .. }],
        ) => false,
        (
            b"MsgLogManager",
            b"AddString",
            [Argument::Scalar { kind: 10, .. }, Argument::String { .. }],
        ) => false,
        (
            b"MsgLogManager",
            b"GetString",
            [Argument::Scalar { kind: 10, .. }, Argument::HeapReference { kind: 20, slot }],
        ) => {
            m.heap.string(*slot)?;
            false
        }
        (b"MsgLogManager", b"SetLineMax", [Argument::Scalar { kind: 10, .. }]) => false,
        (b"MsgLogViewer", b"Numof", []) => true,
        (b"MsgLogViewer", b"Clear", []) => false,
        (b"MsgLogViewer", b"Add", [Argument::Page { kind: 13, .. }]) => false,
        (
            b"MsgLogViewer",
            b"Get",
            [Argument::Scalar { kind: 10, .. }, Argument::HeapReference { kind: 21, .. }],
        ) => false,
        (b"MsgLogViewer", b"ConvertFromMsgLogManager", [Argument::Scalar { kind: 10, .. }]) => {
            false
        }
        _ => anyhow::bail!(
            "unsupported original message HLL {}/{} ABI",
            String::from_utf8_lossy(&lib.name),
            String::from_utf8_lossy(&f.name)
        ),
    };
    ensure!(
        f.return_kind == if scalar { 10 } else { 0 },
        "System4 message result ABI"
    );
    ensure!(
        !scalar || !args.is_empty() || m.stack.len() < 65536,
        "System4 message return stack budget"
    );
    Ok((lib.name.clone(), f.name.clone(), args))
}
type Paths = (VfsPath, VfsPath, Arc<dyn ClientStorage>);
fn run(
    mut m: Machine,
    mut state: State,
    call: ExternalCall,
    library: Vec<u8>,
    name: Vec<u8>,
    paths: Option<Paths>,
    cancel: Cancellation,
) -> Completed {
    let result = (|| -> Result<()> {
        cancel.check()?;
        let (_, _, args) = signature(&m, &call)?;
        let mut candidate = if matches!(
            name.as_slice(),
            b"AddInt"
                | b"AddString"
                | b"SetLineMax"
                | b"Clear"
                | b"Add"
                | b"ConvertFromMsgLogManager"
                | b"Load"
        ) {
            Some(state.clone())
        } else {
            None
        };
        let s = candidate.as_mut().unwrap_or(&mut state);
        let owner = s.owner;
        let value = match (library.as_slice(), name.as_slice(), args.as_slice()) {
            (b"MsgLogManager", b"Numof", []) => Some(i32::try_from(s.manager.len(owner)?)?),
            (b"MsgLogManager", b"GetInterface", []) => Some(s.interface),
            (
                b"MsgLogManager",
                b"AddInt",
                [Argument::Scalar { bits: tag, .. }, Argument::Scalar { bits: value, .. }],
            ) => {
                s.add(
                    ByteRecord {
                        tag: *tag,
                        bytes: Arc::from(value.to_le_bytes()),
                    },
                    &cancel,
                )?;
                None
            }
            (
                b"MsgLogManager",
                b"AddString",
                [Argument::Scalar { bits: tag, .. }, Argument::String { bytes, .. }],
            ) => {
                s.add(
                    ByteRecord {
                        tag: *tag,
                        bytes: Arc::from(cstr(bytes)),
                    },
                    &cancel,
                )?;
                None
            }
            (b"MsgLogManager", b"SetLineMax", [Argument::Scalar { bits, .. }]) => {
                s.line_max = *bits;
                None
            }
            (b"MsgLogManager", b"GetType", [Argument::Scalar { bits, .. }]) => {
                Some(record(&s.manager, owner, *bits)?.map_or(0, |r| r.tag))
            }
            (b"MsgLogManager", b"GetInt", [Argument::Scalar { bits, .. }]) => Some(
                record(&s.manager, owner, *bits)?
                    .map(raw_int)
                    .transpose()?
                    .unwrap_or(0),
            ),
            (
                b"MsgLogManager",
                b"GetString",
                [Argument::Scalar { bits, .. }, Argument::HeapReference { slot, .. }],
            ) => {
                let text = record(&s.manager, owner, *bits)?
                    .map(|r| cstr(&r.bytes))
                    .unwrap_or_default();
                m.heap.set_string(*slot, Arc::new(text.to_vec()))?;
                None
            }
            (b"MsgLogManager", b"Save", _) => {
                let (destination, _, storage) =
                    paths.as_ref().context("System4 log save authority")?;
                let mut cx = DecodeContext::new(owner).with_cancellation(cancel.clone());
                cx.limits.max_entries = 1_000_000;
                let bytes = crysirror_formats_arc::alice_message_log::encode(
                    s.manager.records(owner)?,
                    &cx,
                )?;
                cancel.check()?;
                let destination =
                    resolve(storage.as_ref(), destination)?.unwrap_or_else(|| destination.clone());
                let ok = match storage.write_atomic(&destination, &bytes) {
                    Ok(()) => true,
                    Err(Error::Io(_) | Error::ReadOnly | Error::NotFound) => false,
                    Err(e) => return Err(e.into()),
                };
                Some(i32::from(ok))
            }
            (b"MsgLogManager", b"Load", _) => {
                let (destination, original, storage) =
                    paths.as_ref().context("System4 log load authority")?;
                // Native clears records BEFORE attempting to open, but retains
                // its independent line counter/max. Load does not re-count.
                s.manager.clear(owner)?;
                let data = read(storage.as_ref(), destination, original, &cancel)?;
                let ok = if let Some(data) = data {
                    let mut cx = DecodeContext::new(owner).with_cancellation(cancel.clone());
                    cx.limits.max_entries = 1_000_000;
                    match crysirror_formats_arc::alice_message_log::decode(&data, &cx) {
                        Ok(records) => {
                            s.manager.replace(owner, records, &|| cancel.check())?;
                            true
                        }
                        Err(_) => {
                            cancel.check()?;
                            false
                        }
                    }
                } else {
                    false
                };
                Some(i32::from(ok))
            }
            (b"MsgLogViewer", b"Numof", []) => Some(i32::try_from(s.viewer.len(owner)?)?),
            (b"MsgLogViewer", b"Clear", []) => {
                s.viewer.clear(owner)?;
                None
            }
            (b"MsgLogViewer", b"Add", [Argument::Page { slot, .. }]) => {
                let line = read_line(&m, *slot)?;
                push_line(&mut s.viewer, owner, &line.text, line.voice, line.separator)?;
                None
            }
            (
                b"MsgLogViewer",
                b"Get",
                [Argument::Scalar { bits, .. }, Argument::HeapReference { slot, .. }],
            ) => {
                let line = if let Some(record) = record(&s.viewer, owner, *bits)? {
                    line_record(record)?
                } else {
                    Line {
                        text: Vec::new(),
                        voice: 0,
                        separator: 0,
                    }
                };
                write_line(&mut m, *slot, &line)?;
                None
            }
            (
                b"MsgLogViewer",
                b"ConvertFromMsgLogManager",
                [Argument::Scalar {
                    bits: interface, ..
                }],
            ) => {
                ensure!(
                    *interface == s.interface,
                    "System4 foreign/stale message interface"
                );
                convert(s, &cancel)?;
                None
            }
            _ => anyhow::bail!("System4 message dispatch changed after admission"),
        };
        cancel.check()?;
        m.complete_hll(
            &call,
            if let Some(value) = value {
                HllReturn::Scalar(value)
            } else {
                HllReturn::Void
            },
        )?;
        if let Some(candidate) = candidate {
            state = candidate;
        }
        Ok(())
    })();
    Completed {
        machine: m,
        state,
        error: result.err().map(|e| format!("{e:#}")),
    }
}
fn line_boundary(tag: i32) -> bool {
    matches!(tag, 1101 | 1102 | 1105)
}
fn cstr(bytes: &[u8]) -> &[u8] {
    bytes.split(|b| *b == 0).next().unwrap()
}
fn record(sequence: &RecordSequence, owner: ClientId, index: i32) -> Result<Option<&ByteRecord>> {
    if let Ok(index) = usize::try_from(index) {
        sequence.get(owner, index)
    } else {
        Ok(None)
    }
}
fn raw_int(record: &ByteRecord) -> Result<i32> {
    ensure!(
        record.bytes.len() <= 4,
        "System4 message integer payload would overflow native4-byte output"
    );
    let mut out = [0; 4];
    out[..record.bytes.len()].copy_from_slice(&record.bytes);
    Ok(i32::from_le_bytes(out))
}
struct Line {
    text: Vec<u8>,
    voice: i32,
    separator: i32,
}
fn push_line(
    sequence: &mut RecordSequence,
    owner: ClientId,
    text: &[u8],
    voice: i32,
    separator: i32,
) -> Result<()> {
    let mut bytes = Vec::with_capacity(text.len() + 4);
    bytes.extend(voice.to_le_bytes());
    bytes.extend(text);
    sequence.push(
        owner,
        ByteRecord {
            tag: separator,
            bytes: bytes.into(),
        },
    )
}
fn line_record(record: &ByteRecord) -> Result<Line> {
    ensure!(record.bytes.len() >= 4, "System4 viewer record extent");
    Ok(Line {
        text: record.bytes[4..].to_vec(),
        voice: i32::from_le_bytes(record.bytes[..4].try_into()?),
        separator: record.tag,
    })
}
fn member(m: &Machine, slot: i32, name: &[u8], kinds: &[i32]) -> Result<usize> {
    let page = m.heap.page(slot)?;
    let PageKind::Structure(kind) = page.kind else {
        anyhow::bail!("System4 viewer expects native structure")
    };
    let definition = m
        .program
        .structures
        .get(kind)
        .context("System4 viewer class identity")?;
    let index = definition
        .members
        .iter()
        .position(|v| v.name == name)
        .context("System4 viewer native named field")?;
    ensure!(
        kinds.contains(&definition.members[index].kind) && index < page.values.len(),
        "System4 viewer field type/extent"
    );
    Ok(index)
}
fn read_line(m: &Machine, slot: i32) -> Result<Line> {
    let text = member(m, slot, b"m_sText", &[12])?;
    let voice = member(m, slot, b"m_nVoice", &[10])?;
    let separator = member(m, slot, b"m_bSeparator", &[47, 10])?;
    Ok(Line {
        text: cstr(&m.heap.string(m.heap.get(slot, text as i32)?)?).to_vec(),
        voice: m.heap.get(slot, voice as i32)?,
        separator: m.heap.get(slot, separator as i32)?,
    })
}
fn write_line(m: &mut Machine, slot: i32, line: &Line) -> Result<()> {
    let text = member(m, slot, b"m_sText", &[12])?;
    let voice = member(m, slot, b"m_nVoice", &[10])?;
    let separator = member(m, slot, b"m_bSeparator", &[47, 10])?;
    let text = m.heap.get(slot, text as i32)?;
    // The only fallible mutator runs first and is atomic. Remaining integer
    // writes target already-validated existing cells; aliases keep native order.
    m.heap.set_string(text, Arc::new(line.text.clone()))?;
    m.heap.set(slot, voice as i32, line.voice)?;
    m.heap.set(slot, separator as i32, line.separator)?;
    Ok(())
}
fn convert(s: &mut State, cancel: &Cancellation) -> Result<()> {
    let owner = s.owner;
    let mut text = Vec::new();
    let mut voice = 0;
    let mut present = false;
    // Actual native conversion APPENDS to the viewer. Unlike the public
    // simplified version, it also preserves voice and separator records.
    for record in s.manager.records(owner)? {
        cancel.check()?;
        match record.tag {
            1001 => {
                let bytes = cstr(&record.bytes);
                ensure!(
                    text.len()
                        .checked_add(bytes.len())
                        .is_some_and(|n| n <= 64 * 1024 * 1024),
                    "System4 viewer concatenated text budget"
                );
                text.extend(bytes);
                present = true;
            }
            1211 => voice = raw_int(record)?,
            1101 => {
                push_line(&mut s.viewer, owner, &text, voice, 0)?;
                text.clear();
            }
            1102 | 1105 => {
                if present {
                    push_line(&mut s.viewer, owner, &text, voice, 0)?;
                    push_line(&mut s.viewer, owner, &[], 0, 1)?;
                    present = false;
                }
                voice = 0;
                text.clear();
            }
            _ => {}
        }
    }
    Ok(())
}
fn resolve(storage: &dyn ClientStorage, path: &VfsPath) -> Result<Option<VfsPath>> {
    crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(storage, path)
}
fn read(
    storage: &dyn ClientStorage,
    destination: &VfsPath,
    original: &VfsPath,
    cancel: &Cancellation,
) -> Result<Option<Vec<u8>>> {
    let path = if let Some(path) = resolve(storage, destination)? {
        path
    } else if let Some(path) = resolve(storage, original)? {
        path
    } else {
        return Ok(None);
    };
    let reader = match storage.open_read(&path) {
        Ok(r) => r,
        Err(Error::Io(_) | Error::NotFound) => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let len = reader.len()?;
    ensure!(len <= 64 * 1024 * 1024, "System4 message file byte budget");
    let mut bytes = vec![0; len as usize];
    for (i, block) in bytes.chunks_mut(65536).enumerate() {
        cancel.check()?;
        crysirror_resource::read_exact_at(reader.as_ref(), i as u64 * 65536, block)?;
    }
    ensure!(
        reader.len()? == len,
        "System4 message file changed during read"
    );
    Ok(Some(bytes))
}

// Copyright (C)2026 CrySirror contributors. GPL-2.0-or-later.
//! Original SACT2 CSV10 and complete native table/file/scanner helpers
//! (100017E0..10002020,10011800/11830), read as static data. The pinned public
//! SACT2.c has only TODO declarations, which are not success implementations.
//! Shared ByteTable owns parsing/table mutation; only original syscall policy
//! and scoped storage/VM acknowledgement remain here.
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
    service::{ClientStorage, HostServices},
    state::ClientId,
};
use crysirror_resource::byte_table::{
    ByteCsvReadOptions, ByteCsvWriteOptions, ByteTable, ByteTableLimits, LongLinePolicy,
};
use crysirror_tasks::{Cancellation, Task, TaskPool};
use std::sync::Arc;
#[derive(Clone)]
struct State {
    table: ByteTable,
    filename: Option<Arc<Vec<u8>>>,
    stopped_at_long_line: bool,
}
struct Paths {
    save: VfsPath,
    original: VfsPath,
    storage: Arc<dyn ClientStorage>,
}
struct Completed {
    machine: Machine,
    state: State,
    error: Option<String>,
}
pub(crate) struct NativeCsv {
    owner: ClientId,
    pool: TaskPool,
    state: Option<State>,
    task: Option<Task<Completed>>,
}
impl NativeCsv {
    pub(crate) fn new(owner: ClientId, pool: TaskPool) -> Result<Self> {
        Ok(Self {
            owner,
            pool,
            state: Some(State {
                table: ByteTable::new(ByteTableLimits::default())?,
                filename: None,
                stopped_at_long_line: false,
            }),
            task: None,
        })
    }
    pub(crate) fn handles(m: &Machine, call: &ExternalCall) -> bool {
        let ExternalTarget::Hll { library, function } = call.target else {
            return false;
        };
        let library = &m.program.libraries[library];
        library.name == b"SACT2" && library.functions[function].name.starts_with(b"CSV_")
    }
    pub(crate) fn has_pending(&self) -> bool {
        self.task.is_some()
    }
    pub(crate) fn cancel(&mut self) {
        self.task = None;
    }
    pub(crate) fn poll(&mut self, machine: &mut Option<Machine>) -> Result<bool> {
        let Some(task) = self.task.as_mut() else {
            return Ok(false);
        };
        let Some(done) = task.try_take()? else {
            return Ok(false);
        };
        ensure!(machine.is_none(), "native CSV duplicate VM owner");
        self.task = None;
        self.state = Some(done.state);
        *machine = Some(done.machine);
        if let Some(error) = done.error {
            anyhow::bail!("{error}")
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
        let m = machine.as_ref().context("native CSV VM absent")?;
        if !Self::handles(m, call) {
            return Ok(None);
        }
        ensure!(
            owner == self.owner && self.task.is_none(),
            "native CSV worker ownership"
        );
        let (name, args) = signature(m, call)?;
        let filename = if matches!(name.as_slice(), b"CSV_Load" | b"CSV_SaveAs") {
            let Argument::String { bytes, .. } = &args[0] else {
                unreachable!()
            };
            Some(bytes.clone())
        } else if name == b"CSV_Save" {
            self.state
                .as_ref()
                .context("native CSV state absent")?
                .filename
                .clone()
        } else {
            None
        };
        let paths =
            if let Some(filename) = filename.as_ref().filter(|bytes| !cstr(bytes).is_empty()) {
                let (mount, relative) = files.hll_path(filename, Mount::Save)?;
                ensure!(mount == Mount::Save, "native CSV writable capability");
                let save = VfsPath::new(Mount::Save, &relative)?;
                let (mount, relative) = files.hll_path(filename, Mount::Asset)?;
                let original = if mount == Mount::Save {
                    files.save_path(filename)?.1
                } else {
                    VfsPath::new(mount, &relative)?
                };
                Some(Paths {
                    save,
                    original,
                    storage: host.storage(owner)?,
                })
            } else {
                None
            };
        let permit = self.pool.reserve()?;
        let m = machine.take().unwrap();
        let state = self.state.take().context("native CSV state absent")?;
        let call = call.clone();
        self.task =
            Some(permit.spawn(move |cancel| {
                Ok(run(m, state, call, name, args, paths, filename, cancel))
            })?);
        Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))))
    }
}
fn signature(m: &Machine, call: &ExternalCall) -> Result<(Vec<u8>, Vec<Argument>)> {
    let ExternalTarget::Hll { library, function } = call.target else {
        anyhow::bail!("native CSV target")
    };
    let f = &m.program.libraries[library].functions[function];
    let args = m.hll_arguments(call)?;
    let scalar = match (f.name.as_slice(), args.as_slice()) {
        (b"CSV_Load" | b"CSV_SaveAs", [Argument::String { .. }]) => true,
        (b"CSV_Save" | b"CSV_CountLines" | b"CSV_CountColumns", []) => true,
        (
            b"CSV_Get",
            [
                Argument::HeapReference { kind: 20, slot },
                Argument::Scalar { kind: 10, .. },
                Argument::Scalar { kind: 10, .. },
            ],
        ) => {
            m.heap.string(*slot)?;
            false
        }
        (
            b"CSV_Set",
            [
                Argument::Scalar { kind: 10, .. },
                Argument::Scalar { kind: 10, .. },
                Argument::String { .. },
            ],
        ) => true,
        (
            b"CSV_GetInt",
            [
                Argument::Scalar { kind: 10, .. },
                Argument::Scalar { kind: 10, .. },
            ],
        ) => true,
        (
            b"CSV_SetInt",
            [
                Argument::Scalar { kind: 10, .. },
                Argument::Scalar { kind: 10, .. },
                Argument::Scalar { kind: 10, .. },
            ],
        ) => false,
        (
            b"CSV_Realloc",
            [
                Argument::Scalar { kind: 10, .. },
                Argument::Scalar { kind: 10, .. },
            ],
        ) => false,
        _ => anyhow::bail!(
            "unsupported native CSV export/ABI {}",
            String::from_utf8_lossy(&f.name)
        ),
    };
    ensure!(
        f.return_kind == if scalar { 10 } else { 0 },
        "native CSV return ABI"
    );
    Ok((f.name.clone(), args))
}
fn run(
    original: Machine,
    state: State,
    call: ExternalCall,
    name: Vec<u8>,
    args: Vec<Argument>,
    paths: Option<Paths>,
    filename: Option<Arc<Vec<u8>>>,
    cancel: Cancellation,
) -> Completed {
    let result = (|| -> Result<(Machine, State)> {
        cancel.check()?;
        // These bounded clones are worker-only. A late decode/heap/output error
        // returns the untouched original Machine/PC/refcounts and CSV state.
        let mut machine = original.clone();
        let mut next = state.clone();
        let scalar = match (name.as_slice(), args.as_slice()) {
            (b"CSV_CountLines", []) => Some(i32::try_from(state.table.row_count())?),
            (b"CSV_CountColumns", []) => Some(i32::try_from(state.table.column_count())?),
            (
                b"CSV_Get",
                [
                    Argument::HeapReference { slot, .. },
                    Argument::Scalar { bits: row, .. },
                    Argument::Scalar { bits: column, .. },
                ],
            ) => {
                let bytes = cell(&state.table, *row, *column).unwrap_or_default();
                machine.heap.set_string(*slot, Arc::new(bytes.to_vec()))?;
                None
            }
            (
                b"CSV_GetInt",
                [
                    Argument::Scalar { bits: row, .. },
                    Argument::Scalar { bits: column, .. },
                ],
            ) => {
                let bytes = cell(&state.table, *row, *column).unwrap_or_default();
                Some(parse_integer(bytes, &cancel)?)
            }
            (
                b"CSV_Set",
                [
                    Argument::Scalar { bits: row, .. },
                    Argument::Scalar { bits: column, .. },
                    Argument::String { bytes, .. },
                ],
            ) => Some(i32::from(set(
                &mut next.table,
                *row,
                *column,
                cstr(bytes).into(),
            )?)),
            (
                b"CSV_SetInt",
                [
                    Argument::Scalar { bits: row, .. },
                    Argument::Scalar { bits: column, .. },
                    Argument::Scalar { bits: value, .. },
                ],
            ) => {
                set(
                    &mut next.table,
                    *row,
                    *column,
                    value.to_string().into_bytes().into(),
                )?;
                None
            }
            (
                b"CSV_Realloc",
                [
                    Argument::Scalar { bits: rows, .. },
                    Argument::Scalar { bits: columns, .. },
                ],
            ) => {
                next.table
                    .resize(usize::try_from(*rows)?, usize::try_from(*columns)?, &|| {
                        cancel.check()
                    })?;
                None
            }
            (b"CSV_Load", _) => {
                next.filename = filename;
                next.table = ByteTable::new(ByteTableLimits::default())?;
                next.stopped_at_long_line = false;
                let source = if let Some(paths) = paths.as_ref() {
                    match resolve(paths.storage.as_ref(), &paths.save)? {
                        Some(path) => Some(path),
                        None => resolve(paths.storage.as_ref(), &paths.original)?,
                    }
                } else {
                    None
                };
                if let Some(path) = source {
                    let paths = paths.as_ref().unwrap();
                    let source = paths.storage.open_asset(&path)?;
                    let bytes = crysirror_resource::read_all(source.as_ref(), 64 * 1024 * 1024)?;
                    let read = ByteTable::decode_lines(
                        &bytes,
                        ByteCsvReadOptions {
                            separator: b',',
                            quote: b'"',
                            line_limit: 4095,
                            windows_text: true,
                            c_prefix: true,
                            long_line: LongLinePolicy::Stop,
                        },
                        ByteTableLimits::default(),
                        &|| cancel.check(),
                    )?;
                    next.table = read.table;
                    next.stopped_at_long_line = read.stopped_at_long_line;
                    Some(1)
                } else {
                    Some(0)
                }
            }
            (b"CSV_Save" | b"CSV_SaveAs", _) => {
                if let Some(paths) = paths.as_ref() {
                    let bytes = state.table.encode_lines(
                        ByteCsvWriteOptions {
                            separator: b',',
                            quote_some: None,
                            windows_text: true,
                            c_prefix: true,
                        },
                        &|| cancel.check(),
                    )?;
                    let target = resolve(paths.storage.as_ref(), &paths.save)?
                        .unwrap_or_else(|| paths.save.clone());
                    cancel.check()?;
                    Some(
                        match paths.storage.write_atomic_existing_parent(&target, &bytes) {
                            Ok(()) => 1,
                            Err(Error::Io(_) | Error::NotFound | Error::ReadOnly) => 0,
                            Err(error) => return Err(error.into()),
                        },
                    )
                } else {
                    Some(0)
                }
            }
            _ => anyhow::bail!("native CSV worker ABI changed"),
        };
        cancel.check()?;
        machine.complete_hll(
            &call,
            scalar.map(HllReturn::Scalar).unwrap_or(HllReturn::Void),
        )?;
        Ok((machine, next))
    })();
    match result {
        Ok((machine, state)) => Completed {
            machine,
            state,
            error: None,
        },
        Err(error) => Completed {
            machine: original,
            state,
            error: Some(format!("{error:#}")),
        },
    }
}
fn resolve(storage: &dyn ClientStorage, path: &VfsPath) -> Result<Option<VfsPath>> {
    match crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(storage, path) {
        Ok(path) => Ok(path),
        Err(error)
            if error
                .downcast_ref::<Error>()
                .is_some_and(|e| matches!(e, Error::NotFound | Error::Io(_))) =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}
fn cstr(bytes: &[u8]) -> &[u8] {
    bytes.split(|b| *b == 0).next().unwrap()
}
fn cell(table: &ByteTable, row: i32, column: i32) -> Option<&[u8]> {
    Some(
        table
            .get(usize::try_from(row).ok()?, usize::try_from(column).ok()?)?
            .as_ref(),
    )
}
fn set(table: &mut ByteTable, row: i32, column: i32, bytes: Arc<[u8]>) -> Result<bool> {
    let (Ok(row), Ok(column)) = (usize::try_from(row), usize::try_from(column)) else {
        return Ok(false);
    };
    table.set(row, column, bytes)
}
#[cfg(feature = "native-text")]
fn parse_integer(bytes: &[u8], cancel: &Cancellation) -> Result<i32> {
    use crysirror_text_legacy::{ByteStringContext, ByteStringLimits, ByteStringProfile};
    let check = || cancel.check();
    let context = ByteStringContext::new(
        ByteStringProfile::UNSIGNED_BYTES,
        ByteStringLimits::default(),
        &check,
    )?;
    Ok(context.unsigned_base0(bytes)?.value as i32)
}
#[cfg(not(feature = "native-text"))]
fn parse_integer(_bytes: &[u8], _cancel: &Cancellation) -> Result<i32> {
    anyhow::bail!("native CSV unsigned32 parsing requires the shared native-text feature")
}

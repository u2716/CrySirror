// Copyright (C) 2019 Nunuhara Cabbage, 2023 kichikuou;
// Rust (C) 2026 CrySirror contributors. GPL-2.0-or-later.
//! Global/Group save policy; a single worker owns the complete native Machine.
//! No owner-thread heap clone, live Scene mutation or hosted engine is involved.
pub(crate) mod codec;
mod legacy_json;
pub(crate) mod resume;
use crate::vm::{global_save::SaveArguments, ExternalCall, Machine};
use anyhow::{ensure, Result};
use crysirror_core::{path::VfsPath, service::ClientStorage, Error};
use crysirror_tasks::Cancellation;
use std::sync::Arc;
pub(crate) struct Completed {
    pub machine: Machine,
    pub error: Option<String>,
}
pub(crate) fn run(
    mut original: Machine,
    call: ExternalCall,
    args: SaveArguments,
    paths: (VfsPath, VfsPath),
    storage: Arc<dyn ClientStorage>,
    cancel: Cancellation,
) -> Completed {
    let result = (|| -> Result<Option<Machine>> {
        cancel.check()?;
        if args.loading {
            let Some(bytes) = read(storage.as_ref(), &paths, &cancel)? else {
                original.finish_global_save(&call, &args, false, None)?;
                return Ok(None);
            };
            let decoded = if has_envelope(&bytes) {
                codec::Save::decode(&bytes, &|| cancel.check())
            } else {
                match legacy_json::parse(&bytes, &|| cancel.check()) {
                    Ok(json) => {
                        // A wrong key is a native VM error, not ordinary failed I/O.
                        legacy_json::check_key(&json, cstr(&args.key))?;
                        legacy_json::globals(
                            &original.program,
                            &json,
                            cstr(&args.key),
                            args.group.as_ref().map(|g| cstr(g)).unwrap_or_default(),
                            &|| cancel.check(),
                        )
                    }
                    Err(error) => Err(error),
                }
            };
            let save = match decoded {
                Ok(save) => save,
                Err(_) => {
                    cancel.check()?;
                    original.finish_global_save(&call, &args, false, None)?;
                    return Ok(None);
                }
            };
            // Staging is on the worker and uses the existing bounded native heap. Every
            // late parse/type/slot/refcount error drops staging, returning original.
            let mut staged = original.clone();
            staged.apply_globals(&args, &save, &|| cancel.check())?;
            staged.finish_global_save(&call, &args, true, None)?;
            cancel.check()?;
            Ok(Some(staged))
        } else {
            let save = original.capture_globals(&args, &|| cancel.check())?;
            let count = save.globals.len();
            let bytes = save.encode(&|| cancel.check())?;
            // Preflight the complete native return before publishing filesystem bytes.
            // This clone also stays on the worker; failed writes keep original state.
            let mut finished = original.clone();
            finished.finish_global_save(&call, &args, true, Some(count))?;
            cancel.check()?;
            let target = crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(
                storage.as_ref(),
                &paths.0,
            )?
            .unwrap_or(paths.0);
            let written = match storage.write_atomic(&target, &bytes) {
                Ok(()) => true,
                Err(Error::Io(_) | Error::ReadOnly | Error::NotFound) => false,
                Err(error) => return Err(error.into()),
            };
            if written {
                Ok(Some(finished))
            } else {
                original.finish_global_save(&call, &args, false, None)?;
                Ok(None)
            }
        }
    })();
    match result {
        Ok(Some(machine)) => Completed {
            machine,
            error: None,
        },
        Ok(None) => Completed {
            machine: original,
            error: None,
        },
        Err(error) => Completed {
            machine: original,
            error: Some(format!("{error:#}")),
        },
    }
}
fn read(
    storage: &dyn ClientStorage,
    paths: &(VfsPath, VfsPath),
    cancel: &Cancellation,
) -> Result<Option<Vec<u8>>> {
    let resolve =
        |p| crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(storage, p);
    let path = if let Some(p) = resolve(&paths.0)? {
        p
    } else if let Some(p) = resolve(&paths.1)? {
        p
    } else {
        return Ok(None);
    };
    let reader = match storage.open_read(&path) {
        Ok(r) => r,
        Err(Error::NotFound | Error::Io(_)) => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let length = reader.len()?;
    ensure!(
        length <= codec::LIMIT as u64,
        "System4 native save input byte budget"
    );
    let mut bytes = vec![0; length as usize];
    for (index, block) in bytes.chunks_mut(65536).enumerate() {
        cancel.check()?;
        crysirror_resource::read_exact_at(reader.as_ref(), index as u64 * 65536, block)?;
    }
    ensure!(
        reader.len()? == length,
        "System4 native save changed while reading"
    );
    Ok(Some(bytes))
}

pub(crate) fn run_resume(
    mut original: Machine,
    call: ExternalCall,
    args: crate::vm::resume::ResumeArguments,
    paths: (VfsPath, VfsPath),
    storage: Arc<dyn ClientStorage>,
    cancel: Cancellation,
) -> Completed {
    let result = (|| -> Result<Machine> {
        cancel.check()?;
        if args.code == 9 {
            let bytes = read(storage.as_ref(), &paths, &cancel)?
                .ok_or_else(|| anyhow::anyhow!("System4 original resume file not found"))?;
            let save = if has_envelope(&bytes) {
                let raw = codec::open_envelope(&bytes, &|| cancel.check())?;
                resume::Resume::decode_payload(&raw, &|| cancel.check())?
            } else {
                let json = legacy_json::parse(&bytes, &|| cancel.check())?;
                legacy_json::resume(
                    &original.program,
                    &json,
                    cstr(&args.key),
                    original.heap.original_slots().len(),
                    &|| cancel.check(),
                )?
            };
            let mut staged = original.clone();
            staged.apply_resume(&args, &save, &|| cancel.check())?;
            cancel.check()?;
            Ok(staged)
        } else if args.code == 23 {
            let loaded = read(storage.as_ref(), &paths, &cancel)?;
            let comments = match loaded.as_ref() {
                Some(bytes) if !has_envelope(bytes) => {
                    let json = legacy_json::parse(bytes, &|| cancel.check())?;
                    Some(legacy_json::comments(&json, cstr(&args.key))?)
                }
                Some(bytes) => {
                    let save = codec::open_envelope(bytes, &|| cancel.check())
                        .ok()
                        .and_then(|raw| {
                            resume::Resume::decode_payload(&raw, &|| cancel.check()).ok()
                        });
                    cancel.check()?;
                    if let Some(save) = save {
                        ensure!(
                            save.key == cstr(&args.key),
                            "System4 resume comment key mismatch"
                        );
                        Some(save.comments)
                    } else {
                        None
                    }
                }
                None => None,
            };
            let mut staged = original.clone();
            staged.apply_resume_comments(
                &call,
                &args,
                comments.as_deref().unwrap_or_default(),
                comments.is_some(),
                &|| cancel.check(),
            )?;
            Ok(staged)
        } else if args.code == 22 {
            let comments = original.capture_resume_comments(&args)?;
            let loaded = read(storage.as_ref(), &paths, &cancel)?;
            let old = if let Some(bytes) = loaded {
                let raw = match codec::open_envelope(&bytes, &|| cancel.check()) {
                    Ok(raw) => raw,
                    Err(_) => {
                        cancel.check()?;
                        original.finish_resume_save(&call, &args, false)?;
                        return Ok(original.clone());
                    }
                };
                match resume::Resume::decode_payload(&raw, &|| cancel.check()) {
                    Ok(save) => Some(save),
                    Err(_) => {
                        cancel.check()?;
                        original.finish_resume_save(&call, &args, false)?;
                        return Ok(original.clone());
                    }
                }
            } else {
                None
            };
            let save = original.prepare_comment_resume(&args, old, comments)?;
            let bytes =
                codec::seal_envelope(&save.encode_payload(&|| cancel.check())?, 1, &|| {
                    cancel.check()
                })?;
            let mut finished = original.clone();
            finished.finish_resume_save(&call, &args, true)?;
            cancel.check()?;
            let target = crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(
                storage.as_ref(),
                &paths.0,
            )?
            .unwrap_or(paths.0);
            match storage.write_atomic(&target, &bytes) {
                Ok(()) => Ok(finished),
                Err(Error::Io(_) | Error::ReadOnly | Error::NotFound) => {
                    original.finish_resume_save(&call, &args, false)?;
                    Ok(original.clone())
                }
                Err(error) => Err(error.into()),
            }
        } else {
            ensure!(args.code == 8, "System4 unknown native resume operation");
            let save = original.capture_resume(&call, &args, &|| cancel.check())?;
            let bytes =
                codec::seal_envelope(&save.encode_payload(&|| cancel.check())?, 1, &|| {
                    cancel.check()
                })?;
            let mut finished = original.clone();
            finished.finish_resume_save(&call, &args, true)?;
            cancel.check()?;
            let target = crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(
                storage.as_ref(),
                &paths.0,
            )?
            .unwrap_or(paths.0);
            match storage.write_atomic(&target, &bytes) {
                Ok(()) => Ok(finished),
                Err(Error::Io(_) | Error::ReadOnly | Error::NotFound) => {
                    original.finish_resume_save(&call, &args, false)?;
                    Ok(original.clone())
                }
                Err(error) => Err(error.into()),
            }
        }
    })();
    match result {
        Ok(machine) => Completed {
            machine,
            error: None,
        },
        Err(error) => Completed {
            machine: original,
            error: Some(format!("{error:#}")),
        },
    }
}

// savefile_read selects JSON only for an invalid signature, not corrupt zlib
// bearing a valid GD11 header. Short headers also have invalid signature.
fn has_envelope(bytes: &[u8]) -> bool {
    bytes.len() >= 8 && bytes.starts_with(b"GD\x01\x01")
}
fn cstr(bytes: &[u8]) -> &[u8] {
    bytes.split(|b| *b == 0).next().unwrap()
}

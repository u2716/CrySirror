// SPDX-License-Identifier: MPL-2.0
//! Scoped persistence for this adapter. The explicit CrySirror envelope is
//! distinct from full Windows N2 slots. Native common/val.npf variables are
//! decoded separately; script/frame/backlog restoration is not inferred.
//! Slot zero retains global (#) values and read marks, not local ($) values.
use super::*;

pub(super) const COMMON: &str = "n2system-v1/common.json";
const NATIVE_COMMON: &str = "common/val.npf";
const FILE_LIMIT: u64 = 32 * 1024 * 1024;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct Common {
    pub schema: u32,
    pub flags: BTreeMap<String, Value>,
    pub arrays: crate::array::Arrays,
    pub read: BTreeMap<String, BTreeMap<String, bool>>,
    #[serde(default)]
    pub native_variables: Option<crate::native_save::Variables>,
}
impl Common {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema == 1 && self.flags.len() <= 32766 && self.read.len() <= 4096,
            "N2System common-data schema/count"
        );
        self.arrays.validate()?;
        if let Some(native) = &self.native_variables {
            ensure!(native.global, "N2System common variable scope");
            native.encode()?;
        }
        for (name, value) in &self.flags {
            ensure!(name.len() <= 4096, "N2System common flag name");
            if let Value::Array(id, _) = value {
                ensure!(
                    self.arrays.nodes.contains_key(id),
                    "N2System common indexed root"
                );
            }
        }
        for (module, marks) in &self.read {
            vm::module_name(module)?;
            ensure!(
                marks.len() <= 32766 && marks.keys().all(|s| s.len() <= 4096),
                "N2System common read marks"
            );
        }
        Ok(())
    }
    pub fn load(
        storage: &dyn ClientStorage,
        material: Option<&crate::archive::ArchiveMaterial>,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<Option<Self>> {
        let mut common = if storage.metadata(Mount::Save, COMMON)?.is_some() {
            let value: Self =
                serde_json::from_slice(&read(storage, &VfsPath::new(Mount::Save, COMMON)?)?)
                    .context("N2System CrySirror common data")?;
            value.validate()?;
            Some(value)
        } else {
            None
        };
        if storage.metadata(Mount::Save, NATIVE_COMMON)?.is_some() {
            let bytes = native_file::decode(
                storage,
                material,
                NATIVE_COMMON,
                read(storage, &VfsPath::new(Mount::Save, NATIVE_COMMON)?)?,
                FILE_LIMIT as usize,
                check,
            )?;
            let native = crate::native_save::Variables::decode(&bytes)?;
            ensure!(native.global, "N2System common NPF scope");
            let (flags, arrays) = native.values()?;
            // Original variables remain authoritative, including edits made
            // by the original runtime after a CrySirror session. The JSON
            // sidecar supplies portable read-history data independently.
            let mut value = common.take().unwrap_or_else(|| Self {
                schema: 1,
                ..Default::default()
            });
            value.flags = flags;
            value.arrays = arrays;
            value.native_variables = Some(native);
            value.validate()?;
            return Ok(Some(value));
        }
        Ok(common)
    }
}
pub(super) fn read(storage: &dyn ClientStorage, path: &VfsPath) -> Result<Vec<u8>> {
    let reader = storage.open_read(path)?;
    ensure!(
        reader.len()? <= FILE_LIMIT,
        "N2System scoped file byte budget"
    );
    crysirror_resource::read_all(reader.as_ref(), FILE_LIMIT)
}
pub(super) enum FileResult {
    Slot(Box<restore::Restore>),
    Value(Value),
    Common(Common),
}
fn native_path(path: &str) -> Result<VfsPath> {
    let normalized = path.replace('\\', "/");
    let normalized = normalized.trim_start_matches("./");
    VfsPath::new(Mount::Save, normalized).map_err(Into::into)
}

impl N2SystemVm {
    pub(super) fn native_storage(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        state: &mut Execution,
        args: &Arguments<'_>,
        call: &Call,
    ) -> Result<bool> {
        if let Some(task) = &mut state.file {
            let Some(result) = task.try_take()? else {
                return Ok(false);
            };
            state.file = None;
            let value = match result {
                FileResult::Slot(restore) => {
                    // Applied at the owner boundary; this thread is replaced by
                    // the checkpoint's threads, so the call never completes.
                    self.pending_restore = Some(restore);
                    state.wait = Wait::Frame(self.frame);
                    return Ok(false);
                }
                FileResult::Value(value) => value,
                FileResult::Common(common) => {
                    // Original469890 replaces persistent flags; legacy JSON
                    // retains its prior merge policy. Neither mode invalidates
                    // arena IDs retained by the live local threads.
                    let mut remapped = common;
                    self.machine.merge_common(
                        &mut remapped.flags,
                        &remapped.arrays,
                        remapped.native_variables.is_some(),
                    )?;
                    self.read_marks = remapped.read;
                    Value::Null
                }
            };
            self.machine.complete(&mut state.thread, value)?;
            return Ok(true);
        }
        let storage = self.storage.as_ref().unwrap().clone();
        let material = self.config.archive_material.clone();
        let permit = match self.pool.as_ref().unwrap().reserve() {
            Ok(p) => p,
            Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => return Ok(false),
            Err(e) => return Err(e),
        };
        state.file = Some(match call.function {
            Builtin::Save if args.integer(0)? != 0 => {
                let slot = args.integer(0)?;
                ensure!((1..=9999).contains(&slot), "N2System save slot range");
                let checkpoint = self.capture_checkpoint(host, current, state, slot as u32)?;
                permit.spawn(move |cancel| {
                    cancel.check()?;
                    checkpoint.validate()?;
                    let bytes = serde_json::to_vec(&checkpoint)?;
                    ensure!(
                        bytes.len() as u64 <= FILE_LIMIT,
                        "N2System slot checkpoint byte budget"
                    );
                    cancel.check()?;
                    storage.write_atomic(
                        &VfsPath::new(Mount::Save, &format!("n2system-v1/slots/{slot:04}.json"))?,
                        &bytes,
                    )?;
                    Ok(FileResult::Value(Value::Null))
                })?
            }
            Builtin::Save => {
                let common = Common {
                    schema: 1,
                    flags: self.machine.flags.clone(),
                    arrays: self.machine.arrays.reachable(self.machine.flags.values())?,
                    read: self.read_marks.clone(),
                    native_variables: None,
                };
                permit.spawn(move |cancel| {
                    cancel.check()?;
                    common.validate()?;
                    // Read the previous native table without mutating the
                    // live VM. Retain its inactive scalar fields/labels, then
                    // validate every current value before admitting any write.
                    let previous = if storage.metadata(Mount::Save, NATIVE_COMMON)?.is_some() {
                        let bytes = native_file::decode(
                            storage.as_ref(),
                            material.as_ref(),
                            NATIVE_COMMON,
                            read(storage.as_ref(), &VfsPath::new(Mount::Save, NATIVE_COMMON)?)?,
                            FILE_LIMIT as usize,
                            &|| cancel.check(),
                        )?;
                        Some(crate::native_save::Variables::decode(&bytes)?)
                    } else {
                        None
                    };
                    ensure!(
                        previous.as_ref().is_none_or(|table| table.global),
                        "N2System common NPF scope"
                    );
                    let native = crate::native_save::Variables::from_values(
                        true,
                        &common.flags,
                        &common.arrays,
                        previous.as_ref(),
                    )?;
                    let native_bytes = native_file::encode(
                        storage.as_ref(),
                        material.as_ref(),
                        NATIVE_COMMON,
                        &native.encode()?,
                        FILE_LIMIT as usize - 16,
                        &|| cancel.check(),
                    )?;
                    let bytes = serde_json::to_vec(&common)?;
                    ensure!(
                        bytes.len() as u64 <= FILE_LIMIT,
                        "N2System common-data byte budget"
                    );
                    cancel.check()?;
                    storage.write_atomic(&VfsPath::new(Mount::Save, COMMON)?, &bytes)?;
                    cancel.check()?;
                    storage
                        .write_atomic(&VfsPath::new(Mount::Save, NATIVE_COMMON)?, &native_bytes)?;
                    Ok(FileResult::Value(Value::Null))
                })?
            }
            Builtin::Load if args.integer(0)? != 0 => {
                let slot = args.integer(0)?;
                ensure!((1..=9999).contains(&slot), "N2System load slot range");
                host.append_log(self.client, &format!("N2System: Load({slot}) requested"))?;
                let catalog = self.catalog.as_ref().unwrap().clone();
                let config = self.config.clone();
                let existing: BTreeSet<String> = self.machine.modules.keys().cloned().collect();
                permit.spawn(move |cancel| {
                    restore::load_slot(storage, catalog, config, existing, slot as u32, &cancel)
                        .map(|r| FileResult::Slot(Box::new(r)))
                })?
            }
            Builtin::Load => {
                permit.spawn(move |cancel| {
                    cancel.check()?;
                    Ok(FileResult::Common(
                        Common::load(storage.as_ref(), material.as_ref(), &|| cancel.check())?
                            .context("N2System common data missing")?,
                    ))
                })?
            }
            Builtin::ExistSave | Builtin::MountSavedata | Builtin::XBOX360_ExistContent => {
                let slot = if call.function == Builtin::XBOX360_ExistContent {
                    0
                } else {
                    args.integer(0)?
                };
                ensure!((0..=9999).contains(&slot), "N2System save slot range");
                let path = if slot == 0 {
                    COMMON.into()
                } else {
                    format!("n2system-v1/slots/{slot:04}.json")
                };
                permit.spawn(move |cancel| {
                    cancel.check()?;
                    let exists = storage.metadata(Mount::Save, &path)?.is_some()
                        || (slot == 0 && storage.metadata(Mount::Save, NATIVE_COMMON)?.is_some());
                    Ok(FileResult::Value(Value::Boolean(exists)))
                })?
            }
            Builtin::DeleteSaveFile => {
                let slot = args.integer(0)?;
                ensure!((0..=9999).contains(&slot), "N2System delete slot range");
                let path = if slot == 0 {
                    COMMON.into()
                } else {
                    format!("n2system-v1/slots/{slot:04}.json")
                };
                permit.spawn(move |cancel| {
                    cancel.check()?;
                    match storage.remove(&VfsPath::new(Mount::Save, &path)?) {
                        Ok(()) | Err(crysirror_core::Error::NotFound) => {}
                        Err(error) => return Err(error.into()),
                    }
                    if slot == 0 {
                        cancel.check()?;
                        match storage.remove(&VfsPath::new(Mount::Save, NATIVE_COMMON)?) {
                            Ok(()) | Err(Error::NotFound) => {}
                            Err(error) => return Err(error.into()),
                        }
                    }
                    Ok(FileResult::Value(Value::Null))
                })?
            }
            Builtin::ReadFile => {
                let path = native_path(&args.string(0)?)?;
                let encoding = self.config.encoding.clone();
                permit.spawn(move |cancel| {
                    cancel.check()?;
                    let value = if storage.metadata(Mount::Save, path.relative())?.is_some() {
                        crate::resource::decode(&read(storage.as_ref(), &path)?, &encoding)?
                    } else {
                        String::new()
                    };
                    Ok(FileResult::Value(Value::String(value)))
                })?
            }
            Builtin::WriteFile => {
                let path = native_path(&args.string(0)?)?;
                let text = args.string(1)?;
                let append = args.truth(2)?;
                let encoding = self.config.encoding.clone();
                permit.spawn(move |cancel| {
                    cancel.check()?;
                    let encoded = if encoding == "utf-8" {
                        text.into_bytes()
                    } else {
                        let (encoded, _, errors) = encoding_rs::SHIFT_JIS.encode(&text);
                        ensure!(!errors, "N2System file text is not representable in CP932");
                        encoded.into_owned()
                    };
                    let mut bytes =
                        if append && storage.metadata(Mount::Save, path.relative())?.is_some() {
                            read(storage.as_ref(), &path)?
                        } else {
                            Vec::new()
                        };
                    ensure!(
                        bytes.len().saturating_add(encoded.len()) as u64 <= FILE_LIMIT,
                        "N2System file write byte budget"
                    );
                    bytes.extend_from_slice(&encoded);
                    cancel.check()?;
                    storage.write_atomic(&path, &bytes)?;
                    Ok(FileResult::Value(Value::Null))
                })?
            }
            _ => bail!("N2System non-storage call"),
        });
        Ok(false)
    }
}

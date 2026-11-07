// Copyright (c) 2026 CrySirror contributors. MIT.
//! Save slots (41F7A0 save_point, 41FCB0 save, 420150 do_load, 420360/4205A0
//! slot queries) as portable CrySirror records in the client Save mount.
//! The slot naming follows the original (majiro_%04d / quick%d / last), but
//! the bytes are not interchangeable with original MajiroSavV1.103 files.
use super::*;
use crate::machine::save::SaveState;
use crysirror_core::service::StorageWrite;

const FORMAT: &str = "crysirror-majiro-save-v1";
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct SaveRecord {
    format: String,
    label: Vec<u8>,
    time: String,
    music: Option<Vec<u8>>,
    state: SaveState,
}
#[derive(Default)]
pub(super) struct Saves {
    /// 51A188 (save points armed) and 51A184 (saving allowed).
    pub points_enabled: bool,
    pub enabled: bool,
    pub label: Vec<u8>,
    pub point: Option<SaveRecord>,
    pub just_loaded: bool,
    writes: Vec<Box<dyn StorageWrite>>,
}
fn stamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    // Civil-from-days (proleptic Gregorian), UTC.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{:02}/{:02}/{:02} {:02}:{:02}:{:02}", y % 100, m, d, rem / 3600, rem % 3600 / 60, rem % 60)
}
#[derive(Serialize, Deserialize)]
struct SystemRecord {
    format: String,
    persistent: Vec<(u32, crate::machine::save::SavedValue)>,
    flags: Vec<u32>,
}
const SYSTEM_FILE: &str = "majiro_system.json";
impl MajiroVm {
    /// Startup: restore persistent globals and the flag table.
    pub(super) fn load_system(&mut self, m: &mut Machine, host: &mut dyn HostServices) {
        let Ok(storage) = host.storage(self.client) else { return };
        let Ok(path) = VfsPath::new(Mount::Save, SYSTEM_FILE) else { return };
        let Ok(bytes) = storage.read(&path) else { return };
        match serde_json::from_slice::<SystemRecord>(&bytes) {
            Ok(record) if record.format == "crysirror-majiro-system-v1" => {
                if let Err(e) = m.import_persistent(&record.persistent) {
                    self.warn(format!("Majiro system data: {e:#}"));
                }
                self.flags.extend(record.flags);
            }
            _ => self.warn("Majiro system data is not a CrySirror record; ignored".into()),
        }
    }
    pub(super) fn save_system(&mut self, m: &Machine, host: &mut dyn HostServices) -> Result<()> {
        let record = SystemRecord {
            format: "crysirror-majiro-system-v1".into(),
            persistent: m.export_persistent(),
            flags: self.flags.iter().copied().collect(),
        };
        let storage = host.storage(self.client).map_err(core_error)?;
        let write = storage
            .begin_write_atomic(&VfsPath::new(Mount::Save, SYSTEM_FILE)?, Arc::from(serde_json::to_vec(&record)?))
            .map_err(core_error)?;
        self.saves.writes.push(write);
        Ok(())
    }
    fn slot_path(&self, slot: i32) -> Result<VfsPath> {
        let name = if slot < 0 || slot == 10001 {
            "majiro_last.json".to_owned()
        } else if slot >= 10000 {
            let quick = self.settings.as_ref().and_then(|s| s.registry_integer("QuickNum", 0).ok()).unwrap_or(0);
            format!("majiro_quick{}.json", (10011 - slot + quick).rem_euclid(8))
        } else {
            format!("majiro_{slot:04}.json")
        };
        Ok(VfsPath::new(Mount::Save, &name)?)
    }
    fn read_slot(&self, host: &mut dyn HostServices, slot: i32) -> Option<SaveRecord> {
        let storage = host.storage(self.client).ok()?;
        let path = self.slot_path(slot).ok()?;
        let bytes = storage.read(&path).ok()?;
        let record: SaveRecord = serde_json::from_slice(&bytes).ok()?;
        (record.format == FORMAT).then_some(record)
    }
    pub(super) fn poll_save_writes(&mut self) {
        let mut failed = vec![];
        self.saves.writes.retain_mut(|write| match write.poll() {
            Ok(StorageWriteState::Pending) => true,
            Ok(StorageWriteState::Ready) => false,
            Ok(StorageWriteState::Failed(e)) => {
                failed.push(e);
                false
            }
            Err(e) => {
                failed.push(e.to_string());
                false
            }
        });
        for e in failed {
            self.warn(format!("Majiro save write failed: {e}"));
        }
    }
    /// Save-related syscalls; None when not handled here.
    pub(super) fn try_saves(
        &mut self,
        m: &mut Machine,
        hash: u32,
        a: &[Value],
        returns: bool,
        host: &mut dyn HostServices,
    ) -> Result<Option<syscalls::Out>> {
        use syscalls::Out;
        let int = |i: usize| a.get(i).and_then(|v| v.integer().ok()).unwrap_or(0);
        let output = |n: i32| if returns { vec![Value::Integer(n)] } else { vec![] };
        let out = match hash {
            0x1bb47604 => {
                self.saves.points_enabled = true;
                Out::Value(Value::Integer(0))
            }
            0x7c3b36c5 => {
                self.saves.points_enabled = false;
                Out::Value(Value::Integer(0))
            }
            0x4d849aa6 => {
                self.saves.enabled = true;
                Out::Value(Value::Integer(0))
            }
            0x44b5c4ed => {
                self.saves.enabled = false;
                Out::Value(Value::Integer(0))
            }
            0x4ffab7c4 => Out::Value(Value::Integer(i32::from(!self.saves.enabled))),
            0xf8004993 => {
                if let Some(label) = a.first().and_then(|v| v.bytes().ok()) {
                    self.saves.label = label[..label.len().min(127)].to_vec();
                }
                m.acknowledge(output(0))?;
                if self.saves.points_enabled && self.saves.enabled {
                    match m.capture_save() {
                        Ok(state) => {
                            self.saves.point = Some(SaveRecord {
                                format: FORMAT.into(),
                                label: self.saves.label.clone(),
                                time: String::new(),
                                music: self.audio_names.get(&syscalls::CH_MUSIC).cloned(),
                                state,
                            });
                        }
                        Err(e) => self.warn(format!("Majiro save point: {e:#}")),
                    }
                }
                Out::Done
            }
            0x89018dd1 => {
                let slot = int(0);
                let last = slot < 0 || slot == 10001;
                if self.saves.enabled && !(last && self.saves.just_loaded) {
                    if let Some(point) = &self.saves.point {
                        let mut record = point.clone();
                        record.time = stamp();
                        let bytes = serde_json::to_vec(&record)?;
                        let path = self.slot_path(slot)?;
                        let storage = host.storage(self.client).map_err(core_error)?;
                        let write = storage.begin_write_atomic(&path, Arc::from(bytes)).map_err(core_error)?;
                        self.saves.writes.push(write);
                    }
                }
                self.save_system(m, host)?;
                self.saves.just_loaded = false;
                Out::Value(Value::Integer(0))
            }
            0x65075141 => {
                let label = self.read_slot(host, int(0)).map(|r| r.label).unwrap_or_default();
                Out::Value(Value::native_string(&label)?)
            }
            0xa549e852 => {
                let time = self.read_slot(host, int(0)).map(|r| r.time.into_bytes()).unwrap_or_default();
                Out::Value(Value::native_string(&time)?)
            }
            0x4c99b0ea => Out::Value(Value::native_string(&self.saves.label)?),
            0x3a3ca0f1 => {
                let Some(record) = self.read_slot(host, int(0)) else {
                    return Ok(Some(Out::Value(Value::Integer(-1))));
                };
                self.load_record(m, record, host)?;
                Out::Done
            }
            _ => return Ok(None),
        };
        Ok(Some(out))
    }
    /// 424CB0 load path: world reset, restore, then HOT_RESET(1, "", 0).
    fn load_record(&mut self, m: &mut Machine, record: SaveRecord, host: &mut dyn HostServices) -> Result<()> {
        let mut programs = BTreeMap::new();
        for name in record.state.modules() {
            if m.linked_module(&name).is_none() {
                let storage = host.storage(self.client).map_err(core_error)?;
                let reader = storage.open_asset(&VfsPath::new(Mount::Asset, &name)?)?;
                let bytes = crysirror_resource::read_all(reader.as_ref(), MAX_MODULE_BYTES as u64)?;
                programs.insert(name, Arc::new(Program::parse(&bytes)?));
            }
        }
        // 41E580: audio, sprites, per-thread services and message state.
        for channel in 0..32 {
            let _ = host.submit_audio(self.client, crysirror_core::service::AudioRequest::Stop { channel, fade_ms: 0.0 });
        }
        self.audio_names.clear();
        self.gfx.set_primary(self.base_primary);
        self.gfx.reset();
        self.regions.clear();
        self.parked_regions = None;
        self.fontout.clear();
        self.text_buffer.clear();
        self.console.visible = false;
        self.round_yields.clear();
        self.timed_waits.clear();
        self.wait = None;
        self.suspended_services.clear();
        m.restore_save(&record.state, &programs)?;
        self.saves.label = record.label.clone();
        self.saves.point = Some(record.clone());
        self.saves.just_loaded = true;
        self.saves.enabled = true;
        self.saves.points_enabled = true;
        if let Some(music) = record.music.filter(|n| !n.is_empty()) {
            let gain = self.music_gain();
            self.play_audio(host, syscalls::CH_MUSIC, &music, None, true, crysirror_core::service::PcmBus::Music, gain, 0.0)?;
            self.audio_names.insert(syscalls::CH_MUSIC, music);
        }
        m.dispatch_event_now(
            crate::program::name_hash(b"HOT_RESET"),
            vec![Value::Integer(1), Value::native_string(b"")?, Value::Integer(0)],
        )?;
        Ok(())
    }
}

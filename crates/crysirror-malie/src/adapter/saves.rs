// SPDX-License-Identifier: MPL-2.0
//! Save slots and persistent system data. Slot layout follows malie.ini
//! (`SaveFileMax=240`, `SaveFileParPage=20`, `QuickSave=200`, `AutoSave=220`).
//! A record holds the interpreter state (PC, stack, globals), the ADV
//! presentation state, native tag state, BGM/environment audio, backlog and
//! a thumbnail. Records are CrySirror's own versioned format: the original
//! `savedata` layout has not been recovered, so original files are neither
//! read nor written. System data (seen pictures/movies/music, scenes, read
//! labels) and settings are separate files.
use super::*;
use crate::vm::MachineState;

pub const QUICK_SLOT: u32 = 200;
pub const AUTO_SLOT: u32 = 220;
pub const THUMB: [u32; 2] = [122, 69];
const VERSION: u32 = 1;
const SYSTEM_FILE: &str = "malie/system.json";

#[derive(Clone, Serialize, Deserialize)]
pub struct SaveRecord {
    pub version: u32,
    pub date: String,
    pub chapter: String,
    pub comment: String,
    pub machine: MachineState,
    pub adv: adv::State,
    pub tags: tags::TagState,
    pub backlog: Vec<(String, String, Option<String>)>,
    pub label: String,
    pub result: u32,
    pub wait: SavedWait,
    pub env: BTreeMap<String, (String, f32)>,
    pub frame_name: String,
    pub thumbnail: Vec<u8>,
    /// Source module compiled by MalieSystem 120; absent for exec.dat.
    #[serde(default)]
    pub module: Option<String>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum SavedWait {
    None,
    Click,
    Select,
    Suspended,
    Halted,
}
/// A save waiting for its thumbnail capture.
#[derive(Clone, Copy, Debug)]
pub enum PendingSave {
    Quick,
    Auto,
}
pub struct SlotInfo {
    pub date: String,
    pub thumbnail: Option<Vec<u8>>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SystemData {
    pub seen_pictures: BTreeSet<String>,
    pub seen_movies: BTreeSet<String>,
    pub music: BTreeSet<u32>,
    pub scenes: Vec<String>,
    pub read_labels: BTreeSet<String>,
    pub flags: BTreeMap<String, u32>,
    pub last_slot: Option<u32>,
}
fn slot_path(slot: u32) -> String {
    format!("malie/save{slot:03}.json")
}
/// Gregorian date of a Unix day number (Howard Hinnant's civil_from_days).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}
impl MalieVm {
    fn storage_ref(&self) -> Result<Arc<dyn ClientStorage>> {
        self.storage.clone().context("Malie storage")
    }
    /// Local wall time from the sampled civil clock, advanced by engine time.
    pub(super) fn local_time(&self, now: u64) -> Option<(i64, u32, u32, u32, u32, u32)> {
        let (unix_ms, offset, sampled) = self.civil?;
        let ms = unix_ms + i64::from(offset) * 1000 + (now.saturating_sub(sampled)) as i64;
        let secs = ms.div_euclid(1000);
        let (y, mo, d) = civil(secs.div_euclid(86_400));
        let s = secs.rem_euclid(86_400);
        Some((y, mo, d, (s / 3600) as u32, (s / 60 % 60) as u32, (s % 60) as u32))
    }
    pub(super) fn slot_exists(&self, slot: u32) -> Result<bool> {
        Ok(self.storage_ref()?.metadata(Mount::Save, &slot_path(slot))?.is_some())
    }
    fn read_record(&self, slot: u32) -> Result<Option<SaveRecord>> {
        let storage = self.storage_ref()?;
        if storage.metadata(Mount::Save, &slot_path(slot))?.is_none() {
            return Ok(None);
        }
        let bytes = storage.read(&VfsPath::new(Mount::Save, &slot_path(slot))?)?;
        ensure!(bytes.len() <= 64 * 1024 * 1024, "Malie save record size");
        let record: SaveRecord = serde_json::from_slice(&bytes).with_context(|| format!("Malie save slot {slot}"))?;
        ensure!(record.version == VERSION, "Malie save record version {}", record.version);
        Ok(Some(record))
    }
    pub(super) fn slot_info(&self, slot: u32) -> Result<Option<SlotInfo>> {
        Ok(self.read_record(slot)?.map(|r| SlotInfo {
            date: r.date,
            thumbnail: (r.thumbnail.len() == (THUMB[0] * THUMB[1] * 4) as usize).then_some(r.thumbnail),
        }))
    }
    /// Capture the record at the current safe point (a click/select wait).
    pub(super) fn record(&self, now: u64) -> Result<SaveRecord> {
        let machine = self.machine.as_ref().context("Malie VM")?.state()?;
        let adv = self.adv.as_ref().unwrap();
        let wait = match self.dialog_return_or_wait() {
            Wait::Click => SavedWait::Click,
            Wait::Select => SavedWait::Select,
            Wait::Suspended => SavedWait::Suspended,
            Wait::Halted => SavedWait::Halted,
            Wait::None => SavedWait::None,
            other => bail!("Malie cannot save while waiting on {other:?}"),
        };
        let date = self
            .local_time(now)
            .map(|(y, mo, d, hh, mm, _)| format!("{:02}/{mo:02}/{d:02} {hh:02}:{mm:02}", y % 100))
            .unwrap_or_default();
        let comment = crate::message::plain(&adv.state.page.tokens).replace('\n', "").chars().take(40).collect();
        Ok(SaveRecord {
            version: VERSION,
            date,
            chapter: self.tags.chapter.clone(),
            comment,
            machine,
            adv: adv.state.clone(),
            tags: self.tags.clone(),
            backlog: self.backlog.iter().rev().take(256).rev().cloned().collect(),
            label: self.label.clone(),
            result: self.result,
            wait,
            env: self.env_sources.clone(),
            frame_name: adv.state.frame.src.clone(),
            thumbnail: self.thumbnail.clone().unwrap_or_default(),
            module: self.module.clone(),
        })
    }
    fn dialog_return_or_wait(&self) -> Wait {
        if matches!(self.wait, Wait::Dialog) { self.dialog_return.clone() } else { self.wait.clone() }
    }
    pub(super) fn save_slot(&mut self, h: &mut dyn HostServices, slot: u32) -> Result<()> {
        let record = self.record(h.now_ms())?;
        let bytes = serde_json::to_vec(&record)?;
        self.storage_ref()?.write_atomic(&VfsPath::new(Mount::Save, &slot_path(slot))?, &bytes)?;
        self.system.last_slot = Some(slot);
        self.system_dirty = true;
        self.system_se(h, "ok")?;
        if self.dialog.is_some() {
            self.sync_page_public()?;
        }
        Ok(())
    }
    pub(super) fn quick_save(&mut self, h: &mut dyn HostServices) -> Result<()> {
        if !self.tags.save_enabled || self.screen.is_some() {
            return self.system_se(h, "error");
        }
        self.thumb_wanted = true;
        self.pending_save = Some(PendingSave::Quick);
        Ok(())
    }
    pub(super) fn autosave(&mut self, h: &mut dyn HostServices) -> Result<()> {
        if self.tags.save_enabled && self.screen.is_none() && self.machine.as_ref().is_some_and(|m| m.pending.is_none()) {
            // Auto slots rotate through the 20 slots of the auto page.
            let n = (0..20).max_by_key(|i| self.read_record(AUTO_SLOT + i).ok().flatten().map(|r| r.date)).unwrap_or(0);
            let next = if self.slot_exists(AUTO_SLOT + n)? { (n + 1) % 20 } else { n };
            let record = self.record(h.now_ms())?;
            self.storage_ref()?.write_atomic(&VfsPath::new(Mount::Save, &slot_path(AUTO_SLOT + next))?, &serde_json::to_vec(&record)?)?;
        }
        Ok(())
    }
    pub(super) fn load_continue(&mut self, h: &mut dyn HostServices) -> Result<()> {
        match self.system.last_slot {
            Some(s) if self.slot_exists(s)? => self.load_slot(h, s),
            _ => self.open_message_box("mb_yes", "セーブデータが存在しません。", MessageBoxPurpose::Info),
        }
    }
    /// Restore a record and resume at its wait; presentation is rebuilt.
    pub(super) fn load_slot(&mut self, h: &mut dyn HostServices, slot: u32) -> Result<()> {
        let Some(r) = self.read_record(slot)? else {
            return self.open_message_box("mb_yes", "ロードに失敗しました。\nセーブデータが存在しません。", MessageBoxPurpose::Info);
        };
        self.apply_record(h, r)?;
        self.system.last_slot = Some(slot);
        self.system_dirty = true;
        Ok(())
    }
    /// Replace the live game with a record (slot load, 62CD40 choice return).
    pub(super) fn apply_record(&mut self, h: &mut dyn HostServices, r: SaveRecord) -> Result<()> {
        self.stop_jump();
        // Validate before replacing any live state.
        self.machine.as_ref().context("Malie VM")?.program.functions.len();
        if self.dialog.is_some() {
            self.close_dialog(h)?;
        }
        self.hide_screen(h)?;
        for c in [BGM, VOICE, SYSTEM_VOICE].into_iter().chain(SE_FIRST..SE_FIRST + SE_COUNT).chain(self.env_channels.values().copied().collect::<Vec<_>>()) {
            h.submit_audio(self.client, AudioRequest::Stop { channel: c, fade_ms: 0. })?;
        }
        self.env_channels.clear();
        if r.module != self.module {
            let program = self.module_program(r.module.as_deref())?.with_context(|| format!("Malie save module {:?} is absent", r.module))?;
            self.machine = Some(Machine::new(program));
            self.module = r.module.clone();
        }
        self.machine.as_mut().unwrap().restore(&r.machine)?;
        self.apply_system_globals();
        self.init = false;
        let adv = self.adv.as_mut().unwrap();
        let ops = adv.forget();
        adv.state = r.adv;
        // Restart transitions, tweens and movie clocks from the load time.
        adv.rebase(h.now_ms());
        self.submit(h, ops, Vec::new())?;
        self.tags = r.tags;
        self.backlog = r.backlog;
        self.label = r.label;
        self.result = r.result;
        self.wait = match r.wait {
            SavedWait::None => Wait::None,
            SavedWait::Click => Wait::Click,
            SavedWait::Select => Wait::Select,
            SavedWait::Suspended => Wait::Suspended,
            SavedWait::Halted => Wait::Halted,
        };
        self.dialog_return = Wait::None;
        self.bgm = None;
        if let Some(name) = self.tags.bgm.clone() {
            if let Ok(n) = name.parse::<u32>() {
                let lp = self.tags.bgm_loop;
                self.start_bgm(h, n, true, 0, 1000, lp)?;
            }
        }
        for (id, (src, gain)) in r.env {
            let mut a = Attrs::new();
            a.insert("id".into(), id);
            a.insert("volume".into(), gain.to_string());
            self.tag_one(h, "layer", &{
                a.insert("env-src".into(), src);
                a
            })?;
        }
        Ok(())
    }
    pub(super) fn return_to_title(&mut self, h: &mut dyn HostServices) -> Result<()> {
        for c in [BGM, VOICE, SYSTEM_VOICE].into_iter().chain(SE_FIRST..SE_FIRST + SE_COUNT) {
            h.submit_audio(self.client, AudioRequest::Stop { channel: c, fade_ms: 500. })?;
        }
        for c in std::mem::take(&mut self.env_channels).into_values() {
            h.submit_audio(self.client, AudioRequest::Stop { channel: c, fade_ms: 500. })?;
        }
        self.bgm = None;
        self.replay = None;
        // 563FD0 ends jumps (6317C0/6317A0); the choice history is per game.
        self.stop_jump();
        self.choice_pending = None;
        self.choice_history.clear();
        let adv = self.adv.as_mut().unwrap();
        adv.state.layers.clear();
        adv.state.chars.clear();
        adv.clear();
        adv.state.frame.visible = false;
        adv.state.select = None;
        self.tags = tags::TagState { read_labels: std::mem::take(&mut self.tags.read_labels), scenes: std::mem::take(&mut self.tags.scenes), ..Default::default() };
        self.machine.as_mut().unwrap().label("main@title")?;
        self.init = false;
        self.wait = Wait::None;
        Ok(())
    }
    /// Scene replay (extra/scene): run the scenario label, ending at
    /// MALIE_SCENE_END (global 192 marks replay mode for the script).
    pub(super) fn start_replay(&mut self, h: &mut dyn HostServices, label: &str) -> Result<()> {
        self.hide_screen(h)?;
        let m = self.machine.as_mut().unwrap();
        m.label(label)?;
        // MALIE_SCENE_END tests global 192 before calling MALIE_END.
        m.globals[192..196].copy_from_slice(&1u32.to_le_bytes());
        self.replay = Some(label.to_owned());
        self.init = false;
        self.wait = Wait::None;
        Ok(())
    }
    pub(super) fn load_system(&mut self) -> Result<()> {
        let storage = self.storage_ref()?;
        if storage.metadata(Mount::Save, SYSTEM_FILE)?.is_some() {
            let bytes = storage.read(&VfsPath::new(Mount::Save, SYSTEM_FILE)?)?;
            self.system = serde_json::from_slice(&bytes).unwrap_or_default();
        }
        if storage.metadata(Mount::Save, crate::settings::FILE)?.is_some() {
            let bytes = storage.read(&VfsPath::new(Mount::Save, crate::settings::FILE)?)?;
            if let Ok(s) = serde_json::from_slice::<Settings>(&bytes) {
                for (k, v) in s.options {
                    self.settings.options.insert(k, v);
                }
                for (k, v) in s.checks {
                    self.settings.checks.insert(k, v);
                }
                for (k, v) in s.values {
                    self.settings.values.insert(k, v);
                }
                self.settings.sync_derived();
            }
        }
        self.seen_pictures = self.system.seen_pictures.clone();
        self.seen_movies = self.system.seen_movies.clone();
        self.music_heard = self.system.music.clone();
        self.tags.scenes = self.system.scenes.clone();
        self.tags.read_labels = self.system.read_labels.clone();
        Ok(())
    }
    /// EXEC `g_*` symbols are system-wide (persist across loads/sessions).
    pub(super) fn global_flags(&self) -> BTreeMap<String, u32> {
        let Some(m) = self.machine.as_ref() else { return BTreeMap::new() };
        m.program
            .symbols
            .iter()
            .filter(|s| s.name.starts_with("g_") || s.name.starts_with("s_"))
            .filter_map(|s| {
                let at = s.offset as usize;
                m.globals.get(at..at + 4).map(|b| (s.name.clone(), u32::from_le_bytes(b.try_into().unwrap())))
            })
            .collect()
    }
    pub(super) fn apply_system_globals(&mut self) {
        let flags = self.system.flags.clone();
        let Some(m) = self.machine.as_mut() else { return };
        let symbols: Vec<(String, u32)> = m.program.symbols.iter().filter(|s| s.name.starts_with("g_")).map(|s| (s.name.clone(), s.offset)).collect();
        for (name, at) in symbols {
            if let (Some(v), Some(slot)) = (flags.get(&name), m.globals.get_mut(at as usize..at as usize + 4)) {
                slot.copy_from_slice(&v.to_le_bytes());
            }
        }
    }
    pub(super) fn save_system(&mut self) -> Result<()> {
        let flags: BTreeMap<String, u32> = self.global_flags().into_iter().filter(|(k, _)| k.starts_with("g_")).collect();
        if flags != self.system.flags {
            self.system.flags = flags;
            self.system_dirty = true;
        }
        if !self.system_dirty {
            return Ok(());
        }
        self.system_dirty = false;
        self.system.seen_pictures = self.seen_pictures.clone();
        self.system.seen_movies = self.seen_movies.clone();
        self.system.music = self.music_heard.clone();
        self.system.scenes = self.tags.scenes.clone();
        self.system.read_labels.extend(self.tags.read_labels.iter().cloned());
        let bytes = serde_json::to_vec(&self.system)?;
        self.storage_ref()?.write_atomic(&VfsPath::new(Mount::Save, SYSTEM_FILE)?, &bytes)?;
        Ok(())
    }
    pub(super) fn save_settings(&mut self, _h: &mut dyn HostServices) -> Result<()> {
        let bytes = serde_json::to_vec(&self.settings)?;
        self.storage_ref()?.write_atomic(&VfsPath::new(Mount::Save, crate::settings::FILE)?, &bytes)?;
        Ok(())
    }
    pub(super) fn sync_page_public(&mut self) -> Result<()> {
        self.refresh_dialog()
    }
    /// Start a frame capture for the next save thumbnail (taken when a page
    /// waits for input, as the native save snapshot is).
    pub(super) fn poll_thumbnail(&mut self, h: &mut dyn HostServices) -> Result<()> {
        if let Some(job) = self.thumb_job.as_mut() {
            match job.poll()? {
                crysirror_core::image_processing::ImageProcessState::Pending => {}
                crysirror_core::image_processing::ImageProcessState::Failed(e) => {
                    self.thumb_job = None;
                    self.adv.as_mut().unwrap().gap(format!("save thumbnail capture: {e}"));
                    self.thumbnail = None;
                    match self.pending_save.take() {
                        Some(PendingSave::Quick) => self.save_slot(h, QUICK_SLOT)?,
                        Some(PendingSave::Auto) => self.autosave(h)?,
                        None => {}
                    }
                }
                crysirror_core::image_processing::ImageProcessState::Ready(img) => {
                    self.thumb_job = None;
                    let [w, hh] = img.size();
                    let src = crysirror_image::ImageSurface { width: w, height: hh, rgba: img.rgba().to_vec() };
                    let mut dst = crysirror_image::ImageSurface { width: THUMB[0], height: THUMB[1], rgba: vec![0; (THUMB[0] * THUMB[1] * 4) as usize] };
                    dst.stretch_blit(&src, crysirror_image::bitmap::StretchOptions {
                        source_rect: crysirror_image::bitmap::BitmapRect::new(0, 0, w, hh),
                        destination: crysirror_image::bitmap::BitmapRect::new(0, 0, THUMB[0], THUMB[1]),
                        opacity: 255,
                        mode: crysirror_image::bitmap::PixelMath::Copy,
                        sampling: Default::default(),
                        color_key: None,
                    })?;
                    // The presented canvas carries composition alpha; a
                    // thumbnail is an opaque picture.
                    let mut rgba = dst.rgba;
                    for px in rgba.chunks_exact_mut(4) {
                        px[3] = 255;
                    }
                    self.thumbnail = Some(rgba);
                    match self.pending_save.take() {
                        Some(PendingSave::Quick) => {
                            self.save_slot(h, QUICK_SLOT)?;
                            self.system_voice(h, "qsave")?;
                        }
                        Some(PendingSave::Auto) => self.autosave(h)?,
                        None => {}
                    }
                }
            }
        } else if self.thumb_wanted {
            self.thumb_wanted = false;
            match h.begin_frame_capture(self.client) {
                Ok(job) => self.thumb_job = Some(job),
                Err(e) => self.adv.as_mut().unwrap().gap(format!("save thumbnail capture: {e}")),
            }
        }
        Ok(())
    }
}

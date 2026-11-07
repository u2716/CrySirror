//! SystemData.dat (6D5904 load at script construction, 6D5ABC write): the
//! svar calculator bank plus the flog/llog read logs and read-line marks.
//!
//! Native writes it before each dat save/load and on shutdown (6E1304).
//! This host has no synchronous shutdown hook for a VM, so a changed state
//! is also written at most every two seconds. The file holds a tagged
//! CrySirror body; an original-engine IMOSYSDATA12 file is reported and left
//! untouched rather than reinterpreted or overwritten.
use super::*;
use crysirror_core::service::ClientStorage;

const FILE: &str = "SystemData.dat";
const TAG: &[u8; 16] = b"CrySirrorQLIEsd\0";
const VERSION: u32 = 1;
const LIMIT: u64 = 32 * 1024 * 1024;
const PERIOD_MS: u64 = 2000;

/// Native 508FAC/50786C writes GameData\System\config.txt at shutdown; the
/// user copy lives in the save capability, the installation stays read-only.
const CONFIG: &str = "GameData/System/Config.txt";
/// (cfg property, config key) pairs read by the cfg loader.
pub(super) const CONFIG_KEYS: &[(&str, &str)] = &[
    ("fs", "FullScreen"),
    ("il", "Interlace"),
    ("fps", "FPSLimit"),
    ("fdm", "FullScreenDispMode"),
    ("sb", "StreamLength"),
    ("cs", "CacheSize"),
    ("blt", "BitBltMode"),
    ("gim", "GraphicIntegerMode"),
    ("sfs", "ScriptFrameSkip"),
    ("ms", "MessageSpeed"),
    ("mtw", "AutoPlayWaitTime"),
    ("mcw", "AutoPlayClickTime"),
];
#[derive(Default)]
pub(super) struct State {
    pub config: Option<String>,
    config_boot: Option<Task<Option<Vec<u8>>>>,
    config_written: Option<u64>,
    config_write: Option<Task<()>>,
    boot: Option<Task<Option<Vec<u8>>>>,
    pub loaded: bool,
    foreign: bool,
    write: Option<Task<()>>,
    written: Option<u64>,
    checked_at: Option<u64>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Log {
    locked: bool,
    lines: Vec<String>,
    table: BTreeMap<String, String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Body {
    version: u32,
    shared: serde_json::Value,
    flog: Option<Log>,
    llog: Option<Log>,
    read_lines: Vec<(String, usize)>,
}
fn read(storage: Arc<dyn ClientStorage>, cancel: &crysirror_tasks::Cancellation) -> Result<Option<Vec<u8>>> {
    read_file(storage, FILE, cancel)
}
fn read_file(
    storage: Arc<dyn ClientStorage>,
    file: &str,
    cancel: &crysirror_tasks::Cancellation,
) -> Result<Option<Vec<u8>>> {
    cancel.check()?;
    let Some(info) = storage.metadata(Mount::Save, file)? else {
        return Ok(None);
    };
    ensure!(
        !info.is_directory && info.size.is_none_or(|n| n <= LIMIT),
        "QLIE user data byte budget/type"
    );
    let reader = storage.open_read(&VfsPath::new(Mount::Save, file)?)?;
    Ok(Some(crysirror_resource::read_all(reader.as_ref(), LIMIT)?))
}
fn digest(bytes: &[u8]) -> u64 {
    // FNV-1a; only detects whether a rewrite is needed.
    bytes.iter().fold(0xcbf29ce484222325, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x100000001b3))
}
impl QlieVm {
    /// Returns true once SystemData has been applied (or is absent).
    pub(super) fn poll_system_data_boot(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
    ) -> Result<bool> {
        if self.system_data.loaded {
            return Ok(true);
        }
        if self.system_data.boot.is_none() {
            let storage = h.storage(self.client)?;
            let config = storage.clone();
            self.system_data.boot = Some(self.pool.spawn(move |cancel| read(storage, &cancel))?);
            self.system_data.config_boot =
                Some(self.pool.spawn(move |cancel| read_file(config, CONFIG, &cancel))?);
        }
        if let Some(task) = self.system_data.config_boot.as_mut() {
            let Some(bytes) = task.try_take()? else {
                return Ok(false);
            };
            self.system_data.config_boot = None;
            if let Some(bytes) = bytes {
                // TStrings.SaveToFile writes UTF-16LE with a byte-order mark.
                let units = bytes
                    .strip_prefix(&[0xff, 0xfe])
                    .context("QLIE user config encoding")?
                    .chunks_exact(2)
                    .map(|p| u16::from_le_bytes([p[0], p[1]]))
                    .collect::<Vec<_>>();
                self.system_data.config_written = Some(digest(&bytes));
                self.system_data.config = Some(String::from_utf16(&units)?);
            }
        }
        let Some(bytes) = self.system_data.boot.as_mut().unwrap().try_take()? else {
            return Ok(false);
        };
        self.system_data.boot = None;
        self.system_data.loaded = true;
        let Some(bytes) = bytes else {
            return Ok(true);
        };
        self.apply_system_data(h, m, &bytes)?;
        Ok(true)
    }
    /// 6D5904 body: shared calculator bank, flog/llog and read lines.
    /// Returns false (nothing applied) for an original-engine file.
    pub(super) fn apply_system_data(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        bytes: &[u8],
    ) -> Result<bool> {
        let Some(json) = bytes.strip_prefix(TAG.as_slice()) else {
            self.system_data.foreign = true;
            let message = "QLIE SystemData.dat is not a CrySirror reader file; original-engine IMOSYSDATA12 import is not implemented and the file is left unchanged";
            h.append_log(self.client, message)?;
            eprintln!("CrySirror: {message}");
            return Ok(false);
        };
        let body: Body = serde_json::from_slice(json).context("QLIE SystemData body")?;
        ensure!(body.version == VERSION, "unsupported QLIE SystemData version");
        ensure!(body.read_lines.len() <= 1_000_000, "QLIE SystemData read-line budget");
        m.restore_shared_bank(body.shared)?;
        for (name, log) in [("flog", body.flog), ("llog", body.llog)] {
            let Some(log) = log else { continue };
            ensure!(log.lines.len() <= 65536 && log.table.len() <= 1_000_000, "QLIE SystemData log budget");
            let layer = self.allocate_layer()?;
            let o = self.objects.entry(name.into()).or_insert_with(|| Object {
                kind: name.into(),
                layer,
                gain: 1.,
                ..Default::default()
            });
            o.locked = log.locked;
            o.lines = log.lines;
            o.table = log.table;
        }
        self.read_lines = body.read_lines.into_iter().collect();
        self.system_data.written = Some(digest(bytes));
        Ok(true)
    }
    /// 50786C Name=Value text: the base file with the cfg-owned keys replaced.
    fn config_bytes(&self) -> Option<Vec<u8>> {
        let cfg = self.objects.get("cfg").filter(|o| o.properties.contains_key("native/config_loaded"))?;
        let mut values = BTreeMap::new();
        for (property, key) in CONFIG_KEYS {
            if let Some(v) = cfg.properties.get(*property).and_then(|v| v.first()) {
                values.insert(key.to_ascii_lowercase(), (key.to_string(), v.text()));
            }
        }
        for (property, key, _) in super::cfg_settings::INT_SETTINGS {
            if let Some(v) = cfg.properties.get(*property).and_then(|v| v.first()) {
                values.insert(key.to_ascii_lowercase(), (key.to_string(), v.text()));
            }
        }
        if let Some(v) = cfg.properties.get("bgi").and_then(|v| v.first()) {
            values.insert("bgimagefilename".into(), ("BGImageFilename".into(), v.text()));
        }
        // 50786C writes Font[i].IL and clears the legacy Font[i].ILRate.
        let mut dropped = Vec::new();
        for i in 0..16 {
            if let Some(v) = cfg.properties.get(&format!("fil/{i}")).and_then(|v| v.first()) {
                let key = format!("Font[{i}].IL");
                values.insert(key.to_ascii_lowercase(), (key, v.text()));
                dropped.push(format!("font[{i}].ilrate"));
            }
        }
        for i in 0..4 {
            let key = if i == 0 { "MessageAlpha".to_owned() } else { format!("MessageAlpha{i}") };
            if let Some(v) = cfg.properties.get(&format!("ma/{i}")).and_then(|v| v.first()) {
                values.insert(key.to_ascii_lowercase(), (key, v.text()));
            }
        }
        for i in 0..16 {
            for (suffix, property) in [("Style", "fst2"), ("StyleColor", "fsc2"), ("AA", "faa2"), ("Name", "fn2")] {
                let key = format!("Font[{i}].{suffix}");
                if let Some(v) = cfg.properties.get(&format!("{property}/{i}")).and_then(|v| v.first()) {
                    values.insert(key.to_ascii_lowercase(), (key, v.text()));
                }
            }
        }
        let mut lines = Vec::new();
        for line in self.system_data.config.as_deref().unwrap_or("").lines() {
            if let Some((key, _)) = line.split_once('=') {
                if dropped.contains(&key.trim().to_ascii_lowercase()) {
                    continue;
                }
                if let Some((name, value)) = values.remove(&key.trim().to_ascii_lowercase()) {
                    lines.push(format!("{name}={value}"));
                    continue;
                }
            }
            lines.push(line.to_owned());
        }
        lines.extend(values.into_values().map(|(name, value)| format!("{name}={value}")));
        let mut out = vec![0xff, 0xfe];
        for unit in lines.join("\r\n").encode_utf16() {
            out.extend_from_slice(&unit.to_le_bytes());
        }
        Some(out)
    }
    pub(super) fn system_data_bytes(&self, m: &Machine) -> Result<Vec<u8>> {
        let log = |name: &str| {
            self.objects.get(name).map(|o| Log {
                locked: o.locked,
                lines: o.lines.clone(),
                table: o.table.clone(),
            })
        };
        let body = Body {
            version: VERSION,
            shared: m.shared_bank_state()?,
            flog: log("flog"),
            llog: log("llog"),
            read_lines: self.read_lines.iter().cloned().collect(),
        };
        let mut out = TAG.to_vec();
        out.extend_from_slice(&serde_json::to_vec(&body)?);
        ensure!(out.len() as u64 <= LIMIT, "QLIE SystemData byte budget");
        Ok(out)
    }
    /// Native save/load points (6D5ABC) force a write; otherwise a changed
    /// state is written periodically. Writes are ordered on one worker slot.
    pub(super) fn persist_system_data(
        &mut self,
        h: &mut dyn HostServices,
        m: &Machine,
        force: bool,
    ) -> Result<()> {
        if !self.system_data.loaded || self.system_data.foreign {
            return Ok(());
        }
        if let Some(task) = self.system_data.write.as_mut() {
            if task.try_take()?.is_none() {
                return Ok(());
            }
            self.system_data.write = None;
        }
        let now = h.now_ms();
        if !force
            && self
                .system_data
                .checked_at
                .is_some_and(|t| now.saturating_sub(t) < PERIOD_MS)
        {
            return Ok(());
        }
        self.system_data.checked_at = Some(now);
        if let Some(config) = self.config_bytes() {
            let hash = digest(&config);
            if self.system_data.config_written != Some(hash) {
                self.system_data.config_written = Some(hash);
                let storage = h.storage(self.client)?;
                // A newer write supersedes (and cancels) an unfinished older one.
                self.system_data.config_write = Some(self.pool.spawn(move |cancel| {
                    cancel.check()?;
                    storage.write_atomic(&VfsPath::new(Mount::Save, CONFIG)?, &config)?;
                    Ok(())
                })?);
            }
        }
        let bytes = self.system_data_bytes(m)?;
        let hash = digest(&bytes);
        if self.system_data.written == Some(hash) {
            return Ok(());
        }
        self.system_data.written = Some(hash);
        let storage = h.storage(self.client)?;
        self.system_data.write = Some(self.pool.spawn(move |cancel| {
            cancel.check()?;
            storage.write_atomic(&VfsPath::new(Mount::Save, FILE)?, &bytes)?;
            Ok(())
        })?);
        Ok(())
    }
}

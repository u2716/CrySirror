// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Reviewed Mugen2 legacy SystemFile data and save policy. Original record
//! layouts remain native; bit operations and asynchronous atomic writes use
//! CrySirror Core. No original host, renderer or filesystem is embedded.
use crate::expression::{VariableProfile, Variables};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    integer_fields,
    path::{Mount, VfsPath},
    service::{CivilTime, ClientStorage, StorageWrite, StorageWriteState},
    state::ClientId,
};
use std::sync::Arc;

const RECORDS: [(u32, usize, &[u8]); 10] = [
    (0, 64, b"FILE INFO      "),
    (1, 2048, b"SYSTEM DATA    "),
    (5, 32800, b"MES FLAG       "),
    (6, 544, b"SYSTEMVAR      "),
    (7, 1312, b"CGFLAG         "),
    (8, 2080, b"SCENEFLAG      "),
    (9, 16416, b"FILMFLAG       "),
    (10, 1056, b"VOICEFLAG      "),
    (11, 1056, b"MOVIEFLAG      "),
    (12, 1056, b"MUSICFLAG      "),
];
const FILE_BYTES: usize = 58432;

/// Exact native SYSTEMTIME ordering; weekday is Sunday0. The host supplies
/// civil time, never the simulation clock or an invented Unix-zero date.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SystemStamp(pub [i32; 8]);
impl SystemStamp {
    pub fn from_civil(time: CivilTime, client: ClientId) -> Result<Self> {
        ensure!(time.client == client, "NNN SystemFile civil time scope");
        let stamp = Self([
            time.year,
            i32::from(time.month),
            i32::from(time.day),
            i32::from(
                time.weekday()
                    .context("NNN SystemFile invalid civil date")?,
            ),
            i32::from(time.hour),
            i32::from(time.minute),
            i32::from(time.second),
            i32::from(time.millisecond),
        ]);
        stamp.validate()?;
        Ok(stamp)
    }
    fn validate(self) -> Result<()> {
        let [year, month, day, weekday, hour, minute, second, milli] = self.0;
        ensure!(
            (0..=65535).contains(&year)
                && (1..=12).contains(&month)
                && (1..=31).contains(&day)
                && (0..=6).contains(&weekday)
                && (0..=23).contains(&hour)
                && (0..=59).contains(&minute)
                && (0..=59).contains(&second)
                && (0..=999).contains(&milli),
            "NNN native SystemFile civil timestamp"
        );
        Ok(())
    }
}
/// Caller-prepared original default strings. This does not load or distribute
/// a font; fixed native fields retain their original encoded bytes.
#[derive(Clone, Debug)]
pub(crate) struct SystemNames {
    pub font: Vec<u8>,
    pub surname: Vec<u8>,
    pub given_name: Vec<u8>,
}
impl SystemNames {
    /// Inspect the authenticated file-backed font-name literal as data. The
    /// surname/given defaults are loader-zeroed native BSS, with no setter in
    /// this reviewed profile; no font file or executable mapping is created.
    pub fn from_module(module: &[u8]) -> Result<Self> {
        let pe = crysirror_resource::pe::PeData::parse(module)?;
        let digest: String = pe
            .sha256()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        ensure!(
            digest == crate::native_bootstrap::MUGEN2_MODULE_SHA256,
            "NNN unreviewed SystemFile constructor module"
        );
        let literal = pe.va_slice(0x553064, 32)?;
        let end = literal
            .iter()
            .position(|byte| *byte == 0)
            .context("NNN unterminated original font-name literal")?;
        ensure!(end > 0 && end < 1024, "NNN original font-name extent");
        for address in [0x586cd8u64, 0x586ce8] {
            let rva = address
                .checked_sub(pe.image_base())
                .context("NNN SystemFile BSS RVA")?;
            let matching: Vec<_> = pe
                .sections()
                .iter()
                .filter(|section| {
                    let start = u64::from(section.rva);
                    rva >= start + u64::from(section.file_size)
                        && rva + 16 <= start + u64::from(section.virtual_size)
                })
                .collect();
            ensure!(
                matching.len() == 1,
                "NNN original name defaults must be zero-fill data"
            );
        }
        Ok(Self {
            font: literal[..end].to_vec(),
            surname: vec![],
            given_name: vec![],
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SystemLimits {
    pub scripts: usize,
    pub film_stride: usize,
    pub message_stride: usize,
    pub cg_characters: usize,
    pub cg_stride: usize,
    pub movie_characters: usize,
    pub movie_stride: usize,
}
impl SystemLimits {
    /// Original SetMax* clamps, followed by CalcuInitParameter. MainControl
    /// handles its separate zero-count defaults before calling these setters.
    pub fn new(scripts: i32, cg_characters: i32, movie_characters: i32) -> Self {
        let scripts = scripts.clamp(2, 256) as usize;
        let grouped = scripts.next_power_of_two();
        let cg_characters = cg_characters.clamp(1, 256) as usize;
        let cg_group = match cg_characters {
            1 => 1,
            2 => 2,
            3..=5 => 5,
            6..=10 => 10,
            11..=20 => 20,
            21..=40 => 40,
            41..=80 => 80,
            81..=160 => 160,
            _ => 320,
        };
        let cg_stride = match cg_group {
            1 => 8192,
            2 => 4096,
            _ => 10240 / cg_group,
        };
        let movie_characters = movie_characters.clamp(1, 256) as usize;
        Self {
            scripts,
            film_stride: 131072 / grouped,
            message_stride: 262144 / grouped,
            cg_characters,
            cg_stride,
            movie_characters,
            movie_stride: (256 / movie_characters) * 32,
        }
    }
    pub fn prepare(
        configuration: &crate::native_bootstrap::NativeConfiguration,
        storage: &dyn ClientStorage,
        cancel: &crysirror_tasks::Cancellation,
    ) -> Result<Self> {
        let mut count = 0;
        for number in 1..=256 {
            cancel.check()?;
            let name = if number < 100 {
                format!("spt/cglist{number:02}.xtx")
            } else {
                format!(
                    "spt/cglist{}{}.xtx",
                    char::from(b'A' + ((number - 100) / 10) as u8),
                    number % 10
                )
            };
            let path = VfsPath::new(Mount::Asset, &configuration.resources.path(&name)?)?;
            match crate::native_resources::open_installed_asset(storage, &path) {
                Ok(_) => count += 1,
                Err(crysirror_core::Error::NotFound) => break,
                Err(error) => return Err(error.into()),
            }
        }
        Self::from_bootstrap(configuration.project.tokens.len(), count)
    }
    pub fn from_bootstrap(script_count: usize, cg_count: usize) -> Result<Self> {
        let scripts = i32::try_from(if script_count == 0 { 64 } else { script_count })?;
        let characters = i32::try_from(if cg_count == 0 { 20 } else { cg_count })?;
        Ok(Self::new(scripts, characters, characters))
    }
}
#[derive(Clone, Debug)]
struct Record {
    words: Vec<i64>,
}
impl Record {
    fn word(&self, offset: usize) -> Result<i32> {
        ensure!(offset % 4 == 0, "NNN SystemFile aligned word");
        Ok(
            integer_fields::get(&self.words, 32, i64::try_from(offset / 4)?)
                .context("NNN SystemFile field bounds")? as i32,
        )
    }
    fn set_word(&mut self, offset: usize, value: i32) -> Result<bool> {
        let previous = self.word(offset)?;
        ensure!(
            integer_fields::set(
                &mut self.words,
                32,
                i64::try_from(offset / 4)?,
                i64::from(value)
            ),
            "NNN SystemFile field bounds"
        );
        Ok(previous != value)
    }
    fn string(&self, offset: usize, capacity: usize) -> Result<Vec<u8>> {
        let mut bytes = Vec::with_capacity(capacity);
        for index in 0..capacity {
            let value = integer_fields::get(&self.words, 8, i64::try_from(offset + index)?)
                .context("NNN SystemFile string field bounds")? as u8;
            if value == 0 {
                return Ok(bytes);
            }
            bytes.push(value);
        }
        anyhow::bail!("NNN unterminated original SystemFile string")
    }
    /// Native strcpy writes through its terminator, preserving later padding.
    fn copy_string(&mut self, offset: usize, capacity: usize, bytes: &[u8]) -> Result<()> {
        ensure!(
            bytes.len() < capacity && !bytes.contains(&0),
            "NNN native fixed string extent"
        );
        for (index, byte) in bytes.iter().copied().chain(std::iter::once(0)).enumerate() {
            ensure!(
                integer_fields::set(
                    &mut self.words,
                    8,
                    i64::try_from(offset + index)?,
                    i64::from(byte)
                ),
                "NNN SystemFile string field bounds"
            );
        }
        Ok(())
    }
    fn set_string(&mut self, offset: usize, capacity: usize, bytes: &[u8]) -> Result<()> {
        ensure!(
            bytes.len() < capacity
                && !bytes.contains(&0)
                && offset
                    .checked_add(capacity)
                    .is_some_and(|end| end <= self.words.len() * 4),
            "NNN SystemFile default string bounds"
        );
        for index in 0..capacity {
            ensure!(
                integer_fields::set(
                    &mut self.words,
                    8,
                    (offset + index) as i64,
                    i64::from(bytes.get(index).copied().unwrap_or(0))
                ),
                "NNN SystemFile byte field bounds"
            );
        }
        Ok(())
    }
}
/// Validate the complete present prefix before a caller overlays it onto the
/// proven constructor state. Native labels/padding are preserved, not invented.
fn decode_prefix(bytes: &[u8]) -> Result<Vec<Record>> {
    ensure!(
        (64..=FILE_BYTES).contains(&bytes.len()),
        "NNN legacy SystemFile byte budget"
    );
    let word = |at: usize| -> Result<u32> {
        Ok(u32::from_le_bytes(
            bytes
                .get(at..at + 4)
                .context("NNN SystemFile truncated word")?
                .try_into()?,
        ))
    };
    ensure!(
        word(0)? == 64 && word(4)? == 0,
        "NNN legacy SystemFile FILE INFO"
    );
    let count = usize::try_from(word(32)?)?;
    ensure!(
        (1..=RECORDS.len()).contains(&count),
        "NNN legacy SystemFile record count"
    );
    let mut at = 0usize;
    let mut records = Vec::with_capacity(count);
    for (code, size, _) in &RECORDS[..count] {
        ensure!(
            word(at)? as usize == *size && word(at + 4)? == *code,
            "NNN legacy SystemFile record layout/code at {at}"
        );
        let end = at
            .checked_add(*size)
            .context("NNN SystemFile record overflow")?;
        let raw = bytes
            .get(at..end)
            .context("NNN truncated legacy SystemFile record")?;
        records.push(Record {
            words: raw
                .chunks_exact(4)
                .map(|word| i64::from(i32::from_le_bytes(word.try_into().unwrap())))
                .collect(),
        });
        at = end;
    }
    ensure!(at == bytes.len(), "NNN legacy SystemFile trailing records");
    Ok(records)
}
#[derive(Clone, Debug)]
pub(crate) struct NativeSystemFile {
    records: Vec<Record>,
    pub limits: SystemLimits,
    revision: u64,
    saved_revision: u64,
}
impl NativeSystemFile {
    pub fn defaults(limits: SystemLimits, stamp: SystemStamp, names: &SystemNames) -> Result<Self> {
        stamp.validate()?;
        let mut records = Vec::with_capacity(RECORDS.len());
        for (code, size, label) in RECORDS {
            let mut record = Record {
                words: vec![0; size / 4],
            };
            record.set_word(0, size as i32)?;
            record.set_word(4, code as i32)?;
            record.set_string(16, 16, label)?;
            records.push(record);
        }
        records[0].set_word(32, 10)?;
        let mut out = Self {
            records,
            limits,
            revision: 0,
            saved_revision: 0,
        };
        out.set_stamp_raw(stamp)?;
        // Complete reviewed CreateInitData0x4590d0, not later public defaults.
        for (offset, value) in [
            (72, 800),
            (76, 600),
            (80, 16),
            (96, 30),
            (100, 70),
            (104, 60),
            (108, 80),
            (112, 60),
            (116, 80),
            (120, 70),
            (124, 80),
            (128, 1),
            (132, 1),
            (136, 2),
            (176, 90),
            (180, 90),
            (184, 90),
            (188, 90),
            (192, 1),
            (196, 1),
            (200, 1),
            (204, 1),
            (212, 90),
            (216, 1),
            (1936, 2),
            (1940, 0),
            (1944, 2),
        ] {
            out.records[1].set_word(offset, value)?;
        }
        out.records[1].set_string(224, 16, &names.surname)?;
        out.records[1].set_string(240, 16, &names.given_name)?;
        out.records[1].set_string(256, 1024, &names.font)?;
        Ok(out)
    }
    pub fn load(bytes: &[u8], defaults: &Self) -> Result<Self> {
        let present = decode_prefix(bytes)?;
        let mut candidate = defaults.clone();
        for (index, record) in present.into_iter().enumerate() {
            candidate.records[index] = record;
        }
        candidate.revision = 0;
        candidate.saved_revision = 0;
        Ok(candidate)
    }
    /// Reviewed CGame initialization0x542180. Existing nonempty native names
    /// win; optional clear flags apply only when an empty field takes defaults.
    /// Prepare/validate both fields before changing either original record.
    pub fn initialize_player_names(&mut self, setup: &crate::config::Names) -> Result<()> {
        let mut candidate = self.records[1].clone();
        for (offset, value_key, clear_key) in [
            (224, b"seiInitData".as_slice(), b"seiClearFlag".as_slice()),
            (240, b"meiInitData".as_slice(), b"meiClearFlag".as_slice()),
        ] {
            if candidate.string(offset, 16)?.is_empty() {
                if let Some(value) = setup.parameter(value_key) {
                    candidate.copy_string(offset, 16, value)?;
                    if setup.integer(clear_key)?.unwrap_or(0) != 0 {
                        candidate.copy_string(offset, 16, b"")?;
                    }
                }
            }
        }
        if candidate.words != self.records[1].words {
            self.changed()?;
            self.records[1] = candidate;
        }
        Ok(())
    }
    pub(crate) fn validate_player_names(&self, names: [&[u8]; 2]) -> Result<()> {
        let mut record = self.records[1].clone();
        for (offset, name) in [224, 240].into_iter().zip(names) {
            record.copy_string(offset, 16, name)?;
        }
        if record.words != self.records[1].words {
            self.revision.checked_add(1).context("NNN SystemFile revision exhausted")?;
        }
        Ok(())
    }
    pub(crate) fn restore_player_names(&mut self, names: [&[u8]; 2]) -> Result<()> {
        let mut record = self.records[1].clone();
        for (offset, name) in [224, 240].into_iter().zip(names) {
            record.copy_string(offset, 16, name)?;
        }
        if record.words != self.records[1].words {
            self.changed()?;
            self.records[1] = record;
        }
        Ok(())
    }
    pub fn player_names(&self) -> Result<[Vec<u8>; 2]> {
        Ok([
            self.records[1].string(224, 16)?,
            self.records[1].string(240, 16)?,
        ])
    }
    pub fn dirty(&self) -> bool {
        self.revision != self.saved_revision
    }
    pub fn system_word(&self, offset: usize) -> Result<i32> {
        self.records[1].word(offset)
    }
    pub(crate) fn set_setting_word(&mut self,offset:usize,value:i32)->Result<()> {
        if self.records[1].word(offset)?!=value {
            self.changed()?;
            self.records[1].set_word(offset,value)?;
        }
        Ok(())
    }
    pub(crate) fn set_auto_mode(&mut self, enabled: bool) -> Result<()> {
        self.set_auto_mode_raw(i32::from(enabled))
    }
    /// Legacy SetSystemParam(6,value) retains the signed native mode word.
    pub(crate) fn set_auto_mode_raw(&mut self, value: i32) -> Result<()> {
        if self.records[1].word(164)? != value {
            self.changed()?;
            self.records[1].set_word(164, value)?;
        }
        Ok(())
    }
    /// Legacy LaunchCountUp's per-process guard lives in the game callback;
    /// this is the one admitted RAM increment, saturated at0x7ffffffe.
    pub(crate) fn record_launch(&mut self) -> Result<u32> {
        let count = self.system_word(208)?;
        ensure!(
            (0..=0x7ffffffe).contains(&count),
            "NNN original launch count profile"
        );
        let next = count.saturating_add(1).min(0x7ffffffe);
        if count != next {
            self.changed()?;
            self.records[1].set_word(208, next)?;
        }
        Ok(next as u32)
    }
    fn changed(&mut self) -> Result<()> {
        self.revision = self
            .revision
            .checked_add(1)
            .context("NNN SystemFile revision exhausted")?;
        Ok(())
    }
    fn set_stamp_raw(&mut self, stamp: SystemStamp) -> Result<()> {
        stamp.validate()?;
        for (index, value) in stamp.0.into_iter().enumerate() {
            self.records[1].set_word(32 + index * 4, value)?;
        }
        Ok(())
    }
    fn flag(&self, record: usize, index: Option<usize>) -> Result<bool> {
        let Some(index) = index else { return Ok(false) };
        Ok(
            integer_fields::get(&self.records[record].words[8..], 1, i64::try_from(index)?)
                .context("NNN native flag profile exceeds record")?
                != 0,
        )
    }
    fn mark(&mut self, record: usize, index: Option<usize>) -> Result<bool> {
        let Some(index) = index else { return Ok(false) };
        if self.flag(record, Some(index))? {
            return Ok(false);
        }
        let next = self
            .revision
            .checked_add(1)
            .context("NNN SystemFile revision exhausted")?;
        ensure!(
            integer_fields::set(
                &mut self.records[record].words[8..],
                1,
                i64::try_from(index)?,
                1
            ),
            "NNN native flag profile exceeds record"
        );
        self.revision = next;
        Ok(true)
    }
    fn pair(first: i32, second: i32, limit: usize, stride: usize) -> Option<usize> {
        let first = usize::try_from(first).ok()?;
        let second = usize::try_from(second).ok()?;
        (first < limit && second < stride).then(|| first * stride + second)
    }
    pub fn film(&self, script: i32, film: i32) -> Result<bool> {
        self.flag(
            6,
            Self::pair(script, film, self.limits.scripts, self.limits.film_stride),
        )
    }
    pub fn mark_film(&mut self, script: i32, film: i32) -> Result<bool> {
        self.mark(
            6,
            Self::pair(script, film, self.limits.scripts, self.limits.film_stride),
        )
    }
    pub fn message(&self, script: i32, message: i32) -> Result<bool> {
        self.flag(
            2,
            Self::pair(
                script,
                message,
                self.limits.scripts,
                self.limits.message_stride,
            ),
        )
    }
    pub fn mark_message(&mut self, script: i32, message: i32) -> Result<bool> {
        self.mark(
            2,
            Self::pair(
                script,
                message,
                self.limits.scripts,
                self.limits.message_stride,
            ),
        )
    }
    pub fn cg(&self, character: i32, number: i32) -> Result<bool> {
        self.flag(
            4,
            Self::pair(
                character,
                number,
                self.limits.cg_characters,
                self.limits.cg_stride,
            ),
        )
    }
    pub fn mark_cg(&mut self, character: i32, number: i32) -> Result<bool> {
        self.mark(
            4,
            Self::pair(
                character,
                number,
                self.limits.cg_characters,
                self.limits.cg_stride,
            ),
        )
    }
    pub fn movie(&self, character: i32, number: i32) -> Result<bool> {
        self.flag(
            8,
            Self::pair(
                character,
                number,
                self.limits.movie_characters,
                self.limits.movie_stride,
            ),
        )
    }
    pub fn mark_movie(&mut self, character: i32, number: i32) -> Result<bool> {
        self.mark(
            8,
            Self::pair(
                character,
                number,
                self.limits.movie_characters,
                self.limits.movie_stride,
            ),
        )
    }
    pub fn voice(&self, character: i32) -> Result<bool> {
        Ok(if (0..256).contains(&character) {
            self.records[7].word(32 + character as usize * 4)? != 0
        } else {
            false
        })
    }
    pub fn mark_voice(&mut self, character: i32) -> Result<bool> {
        self.mark(
            7,
            (0..256)
                .contains(&character)
                .then(|| character as usize * 32),
        )
    }
    pub fn music(&self, number: i32) -> Result<bool> {
        Ok(if (0..256).contains(&number) {
            self.records[9].word(32 + number as usize * 4)? > 0
        } else {
            false
        })
    }
    pub fn mark_music(&mut self, number: i32) -> Result<bool> {
        if !(0..256).contains(&number) {
            return Ok(false);
        }
        let offset = 32 + number as usize * 4;
        let value = self.records[9].word(offset)?;
        if value >= 9999 {
            return Ok(false);
        }
        self.changed()?;
        self.records[9].set_word(offset, value + 1)?;
        Ok(true)
    }
    pub fn restore_variables(
        &self,
        variables: &mut Variables,
        profile: VariableProfile,
    ) -> Result<()> {
        ensure!(
            profile == VariableProfile::Legacy1000,
            "NNN legacy SystemFile variable profile"
        );
        for index in 0..100 {
            variables.write_stored(200 + index as u16, self.records[3].word(32 + index * 4)?);
        }
        Ok(())
    }
    pub fn update_variables(
        &mut self,
        variables: &Variables,
        profile: VariableProfile,
    ) -> Result<()> {
        ensure!(
            profile == VariableProfile::Legacy1000,
            "NNN legacy SystemFile variable profile"
        );
        for index in 0..100 {
            if self.records[3]
                .set_word(32 + index * 4, variables.read_stored(200 + index as u16))?
            {
                self.changed()?;
            }
        }
        Ok(())
    }
    /// The original Save always writes the complete ten constructor records,
    /// even if Load overlaid an older partial prefix. Opaque words are retained.
    pub fn encode(&self) -> Result<Arc<[u8]>> {
        let mut bytes = Vec::with_capacity(FILE_BYTES);
        for (index, record) in self.records.iter().enumerate() {
            for (word, value) in record.words.iter().enumerate() {
                let value = if index == 0 && word == 8 {
                    10
                } else {
                    *value as i32
                };
                bytes.extend(value.to_le_bytes());
            }
        }
        ensure!(bytes.len() == FILE_BYTES, "NNN SystemFile encoded size");
        Ok(bytes.into())
    }
    /// Native Save updates its local timestamp/counter before attempting file
    /// opening. Keep that native in-memory effect on write failure too.
    pub fn begin_save(
        &mut self,
        storage: &dyn ClientStorage,
        variables: &Variables,
        profile: VariableProfile,
        stamp: SystemStamp,
        trial: bool,
    ) -> Result<SystemWrite> {
        stamp.validate()?;
        let mut candidate = self.clone();
        candidate.update_variables(variables, profile)?;
        candidate.set_stamp_raw(stamp)?;
        candidate.records[0].set_word(32, 10)?;
        let count = candidate.records[1].word(92)?.wrapping_add(1);
        candidate.records[1].set_word(92, count)?;
        candidate.changed()?;
        let bytes = candidate.encode()?;
        *self = candidate;
        let path = VfsPath::new(
            Mount::Save,
            if trial { "tsysfile.sav" } else { "sysfile.sav" },
        )?;
        let ticket = storage.begin_write_atomic(&path, bytes)?;
        Ok(SystemWrite {
            ticket,
            revision: self.revision,
            done: false,
        })
    }
}
pub(crate) struct SystemWrite {
    ticket: Box<dyn StorageWrite>,
    revision: u64,
    done: bool,
}
impl SystemWrite {
    pub fn poll(&mut self, file: &mut NativeSystemFile) -> Result<bool> {
        if self.done {
            return Ok(true);
        }
        match self.ticket.poll()? {
            StorageWriteState::Pending => Ok(false),
            StorageWriteState::Failed(error) => {
                anyhow::bail!("NNN native SystemFile atomic write: {error}")
            }
            StorageWriteState::Ready => {
                file.saved_revision = self.revision;
                self.done = true;
                Ok(true)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn stamp() -> SystemStamp {
        SystemStamp([2026, 11, 1, 0, 10, 20, 30, 456])
    }
    fn file() -> NativeSystemFile {
        NativeSystemFile::defaults(
            SystemLimits::new(64, 20, 20),
            stamp(),
            &SystemNames {
                font: b"fixture-font-name".to_vec(),
                surname: vec![],
                given_name: vec![],
            },
        )
        .unwrap()
    }
    #[test]
    fn original_launch_counter_changes_only_its_ram_word_and_saturates_before_overflow() {
        let mut file = file();
        let old = file.encode().unwrap();
        assert_eq!(file.record_launch().unwrap(), 1);
        let mut expected = old.to_vec();
        expected[64 + 208..64 + 212].copy_from_slice(&1i32.to_le_bytes());
        assert_eq!(file.encode().unwrap().as_ref(), expected.as_slice());
        file.records[1].set_word(208, 0x7ffffffd).unwrap();
        assert_eq!(file.record_launch().unwrap(), 0x7ffffffe);
        assert_eq!(file.record_launch().unwrap(), 0x7ffffffe);
        file.records[1].set_word(208, -1).unwrap();
        let before = file.encode().unwrap();
        assert!(file.record_launch().is_err());
        assert_eq!(file.encode().unwrap(), before);
    }
    #[test]
    fn game_name_defaults_preserve_existing_names_padding_and_atomic_validation() {
        let mut file = file();
        let names = crate::config::Names::parse(
            b"seiInitData family meiInitData given meiClearFlag 1",
            false,
        )
        .unwrap();
        file.initialize_player_names(&names).unwrap();
        assert_eq!(
            file.player_names().unwrap(),
            [b"family".to_vec(), Vec::new()]
        );
        let raw = file.encode().unwrap();
        assert_eq!(&raw[64 + 240..64 + 246], b"\0iven\0");
        assert!(file.dirty());
        file.initialize_player_names(
            &crate::config::Names::parse(b"seiInitData other seiClearFlag 1", false).unwrap(),
        )
        .unwrap();
        assert_eq!(file.player_names().unwrap()[0], b"family");
        let before = file.encode().unwrap();
        let invalid = crate::config::Names::parse(b"meiInitData 0123456789abcdef", false).unwrap();
        assert!(file.initialize_player_names(&invalid).is_err());
        assert_eq!(file.encode().unwrap().as_ref(), before.as_ref());
    }
    #[test]
    fn sampled_timestamp_uses_scoped_local_gregorian_fields() {
        let mut sample = CivilTime {
            client: ClientId(9),
            unix_ms: 0,
            utc_offset_seconds: 0,
            year: 2026,
            month: 11,
            day: 1,
            hour: 12,
            minute: 34,
            second: 56,
            millisecond: 789,
        };
        assert_eq!(
            SystemStamp::from_civil(sample, ClientId(9)).unwrap().0,
            [2026, 11, 1, 0, 12, 34, 56, 789]
        );
        assert!(SystemStamp::from_civil(sample, ClientId(8)).is_err());
        sample.day = 31;
        assert!(SystemStamp::from_civil(sample, ClientId(9)).is_err());
        assert!(SystemNames::from_module(b"not a PE").is_err());
    }
    #[test]
    fn all_legacy_records_roundtrip_and_constructor_differs_from_later_source() {
        let file = file();
        let encoded = file.encode().unwrap();
        assert_eq!(encoded.len(), 58432);
        let decoded = decode_prefix(&encoded).unwrap();
        assert_eq!(decoded.len(), 10);
        for (record, (code, size, _)) in decoded.iter().zip(RECORDS) {
            assert_eq!(record.word(0).unwrap(), size as i32);
            assert_eq!(record.word(4).unwrap(), code as i32);
        }
        assert_eq!(file.system_word(72).unwrap(), 800);
        assert_eq!(file.system_word(76).unwrap(), 600);
        assert_eq!(file.system_word(176).unwrap(), 90);
        assert_eq!(file.system_word(216).unwrap(), 1);
        assert_eq!(
            file.system_word(220).unwrap(),
            0,
            "later public autoSpeed50 must not replace the reviewed constructor"
        );
        let loaded = NativeSystemFile::load(&encoded, &file).unwrap();
        assert_eq!(loaded.encode().unwrap().as_ref(), encoded.as_ref());
        assert!(!loaded.dirty());
    }
    #[test]
    fn partial_prefix_overlays_defaults_opaque_words_survive_and_bad_layout_does_not_mutate() {
        let defaults = file();
        let mut raw = defaults.encode().unwrap().to_vec();
        raw[64 + 1500..64 + 1504].copy_from_slice(&0x92345678u32.to_le_bytes());
        let variable_record = RECORDS[..3].iter().map(|r| r.1).sum::<usize>();
        raw[variable_record + 532..variable_record + 536]
            .copy_from_slice(&0x87654321u32.to_le_bytes());
        let mut loaded = NativeSystemFile::load(&raw, &defaults).unwrap();
        let mut vars = Variables::fresh(VariableProfile::Legacy1000);
        vars.write_stored(200, 27);
        loaded
            .update_variables(&vars, VariableProfile::Legacy1000)
            .unwrap();
        let encoded = loaded.encode().unwrap();
        assert_eq!(&encoded[64 + 1500..64 + 1504], &0x92345678u32.to_le_bytes());
        assert_eq!(
            &encoded[variable_record + 532..variable_record + 536],
            &0x87654321u32.to_le_bytes()
        );
        let mut partial = raw[..64 + 2048].to_vec();
        partial[32..36].copy_from_slice(&2u32.to_le_bytes());
        let partial = NativeSystemFile::load(&partial, &defaults).unwrap();
        assert!(!partial.film(0, 0).unwrap());
        assert_eq!(partial.encode().unwrap().len(), FILE_BYTES);
        let baseline = defaults.encode().unwrap();
        for (offset, value) in [(0, 63), (4, 99), (32, 0), (32, 11), (64, 2044), (68, 5)] {
            let mut bad = raw.clone();
            bad[offset..offset + 4].copy_from_slice(&u32::to_le_bytes(value));
            assert!(NativeSystemFile::load(&bad, &defaults).is_err());
            assert_eq!(defaults.encode().unwrap().as_ref(), baseline.as_ref());
        }
        assert!(NativeSystemFile::load(&raw[..raw.len() - 1], &defaults).is_err());
    }
    #[test]
    fn every_progress_family_uses_shared_bit_views_with_original_bounds_and_music_counter() {
        let mut file = file();
        assert!(!file.mark_film(-1, 0).unwrap());
        assert!(!file.mark_film(64, 0).unwrap());
        assert!(!file.mark_message(0, 4096).unwrap());
        assert!(!file.dirty());
        assert!(file.mark_film(63, 2047).unwrap());
        assert!(file.film(63, 2047).unwrap());
        assert!(!file.mark_film(63, 2047).unwrap());
        assert!(!file.film(63, 2046).unwrap());
        assert!(file.mark_message(63, 4095).unwrap());
        assert!(file.message(63, 4095).unwrap());
        assert!(file.mark_cg(19, 511).unwrap());
        assert!(file.cg(19, 511).unwrap());
        assert!(!file.mark_cg(20, 0).unwrap());
        assert!(file.mark_movie(19, 383).unwrap());
        assert!(file.movie(19, 383).unwrap());
        assert!(!file.mark_movie(19, 384).unwrap());
        assert!(file.mark_voice(255).unwrap());
        assert!(file.voice(255).unwrap());
        assert!(!file.mark_voice(-1).unwrap());
        assert!(!file.mark_voice(256).unwrap());
        file.records[9].set_word(32 + 7 * 4, -1).unwrap();
        assert!(file.mark_music(7).unwrap());
        assert!(!file.music(7).unwrap());
        assert!(file.mark_music(7).unwrap());
        assert!(file.music(7).unwrap());
        file.records[9].set_word(32 + 7 * 4, 9999).unwrap();
        assert!(!file.mark_music(7).unwrap());
        let restored = NativeSystemFile::load(&file.encode().unwrap(), &file).unwrap();
        assert!(restored.film(63, 2047).unwrap());
        assert!(restored.message(63, 4095).unwrap());
        assert!(restored.cg(19, 511).unwrap());
        assert!(restored.movie(19, 383).unwrap());
        assert!(restored.voice(255).unwrap());
    }
    #[test]
    fn persistent_variable_range_and_grouped_limit_boundaries_are_native() {
        for count in 2..=256 {
            let limits = SystemLimits::new(count, count, count);
            assert!(limits.scripts * limits.film_stride <= 131072);
            assert!(limits.scripts * limits.message_stride <= 262144);
            assert!(limits.cg_characters * limits.cg_stride <= 10240);
            assert!(limits.movie_characters * limits.movie_stride <= 8192);
        }
        let limits = SystemLimits::from_bootstrap(0, 0).unwrap();
        assert_eq!(limits.scripts, 64);
        assert_eq!(limits.cg_characters, 20);
        assert_eq!(SystemLimits::new(3, 3, 3).film_stride, 32768);
        let mut file = file();
        let mut source = Variables::fresh(VariableProfile::Legacy1000);
        source.write_stored(199, 99);
        source.write_stored(200, -1);
        source.write_stored(299, 123);
        source.write_stored(300, 88);
        file.update_variables(&source, VariableProfile::Legacy1000)
            .unwrap();
        let mut restored = Variables::fresh(VariableProfile::Legacy1000);
        restored.write_stored(199, 44);
        restored.write_stored(300, 55);
        file.restore_variables(&mut restored, VariableProfile::Legacy1000)
            .unwrap();
        assert_eq!(
            [
                restored.read_stored(199),
                restored.read_stored(200),
                restored.read_stored(299),
                restored.read_stored(300)
            ],
            [44, -1, 123, 55]
        );
        assert!(
            file.restore_variables(&mut restored, VariableProfile::Expanded2200)
                .is_err()
        );
    }
    #[test]
    fn shared_atomic_write_is_retained_until_ready_and_does_not_advance_clock() {
        use crysirror_core::{service::HostServices, state::ClientId};
        use std::time::{Duration, Instant};
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("assets")).unwrap();
        let project:crysirror_runtime::Project=serde_json::from_value(serde_json::json!({"schema":2,"name":"native SystemFile","content_revision":"1","assets":"assets","canvas":[16,16],"clients":[{"name":"native","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let mut runtime = crysirror_runtime::Runtime::with_asset_root(
            project,
            dir.path(),
            &dir.path().join("state"),
            Default::default(),
        )
        .unwrap();
        runtime.tick(0, Default::default()).unwrap();
        let storage = runtime.host.storage(ClientId(1)).unwrap();
        let mut file = file();
        file.mark_film(1, 7).unwrap();
        let mut variables = Variables::fresh(VariableProfile::Legacy1000);
        variables.write_stored(201, 77);
        let mut write = file
            .begin_save(
                storage.as_ref(),
                &variables,
                VariableProfile::Legacy1000,
                stamp(),
                false,
            )
            .unwrap();
        assert_eq!(file.system_word(92).unwrap(), 1);
        assert!(file.dirty());
        let deadline = Instant::now() + Duration::from_secs(5);
        while !write.poll(&mut file).unwrap() {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(!file.dirty());
        assert!(write.poll(&mut file).unwrap());
        let data = storage
            .read(&VfsPath::new(Mount::Save, "sysfile.sav").unwrap())
            .unwrap();
        let mut loaded = NativeSystemFile::load(&data, &file).unwrap();
        assert!(loaded.film(1, 7).unwrap());
        let mut restored = Variables::fresh(VariableProfile::Legacy1000);
        loaded
            .restore_variables(&mut restored, VariableProfile::Legacy1000)
            .unwrap();
        assert_eq!(restored.read_stored(201), 77);
        assert_eq!(runtime.host.now_ms(), 0);
        struct ReadOnly;
        impl ClientStorage for ReadOnly {
            fn read(&self, _: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
                Err(crysirror_core::Error::NotFound)
            }
            fn write_atomic(&self, _: &VfsPath, _: &[u8]) -> crysirror_core::Result<()> {
                Err(crysirror_core::Error::ReadOnly)
            }
            fn remove(&self, _: &VfsPath) -> crysirror_core::Result<()> {
                Err(crysirror_core::Error::ReadOnly)
            }
            fn list(&self, _: Mount, _: &str) -> crysirror_core::Result<Vec<String>> {
                Ok(vec![])
            }
        }
        assert!(
            loaded
                .begin_save(
                    &ReadOnly,
                    &variables,
                    VariableProfile::Legacy1000,
                    stamp(),
                    true
                )
                .is_err()
        );
        assert_eq!(
            loaded.system_word(92).unwrap(),
            2,
            "native Save increments RAM metadata before opening even when the write fails"
        );
        assert!(loaded.dirty());
    }
}

#[cfg(test)]
mod original_constructor_tests {
    use super::*;
    #[test]
    #[ignore = "inspects owner-supplied authenticated module as immutable data; no execution or font file"]
    fn owner_original_constructor_font_name_and_zero_fill_defaults() {
        let module =
            std::fs::read(std::env::var("CRYSIRROR_NNN_TEST_MODULE").expect("owner module path"))
                .unwrap();
        let names = SystemNames::from_module(&module).unwrap();
        let (name, _, malformed) = encoding_rs::SHIFT_JIS.decode(&names.font);
        assert!(!malformed);
        assert_eq!(name, "ＭＳ ゴシック");
        assert!(names.surname.is_empty() && names.given_name.is_empty());
    }
}

/// One original savesystemfile function dependency. Native candidates remain
/// detached until the real scoped atomic write completes; no VM ACK is forged.
pub(crate) struct NativeSystemSave {
    client: ClientId,
    call: crate::machine::NativeCall,
    variables: Variables,
    civil: Option<Box<dyn crysirror_core::service::CivilTimeQuery>>,
    write: Option<SystemWrite>,
    pub api_calls: u64,
}
impl NativeSystemSave {
    pub fn begin(
        client: ClientId,
        call: &crate::machine::NativeCall,
        variables: Variables,
        host: &mut dyn crysirror_core::service::HostServices,
    ) -> Result<Self> {
        ensure!(
            client.0 != 0 && variables.stored_word_count() == 1000,
            "NNN system-save native scope/bank ABI"
        );
        ensure!(
            variables.native_writes().is_empty(),
            "NNN savesystemfile arguments with resource-write bindings need an atomic native owner plan"
        );
        let civil = host.begin_civil_time(client)?;
        ensure!(
            civil.client() == client,
            "NNN system-save civil capability scope"
        );
        Ok(Self {
            client,
            call: call.clone(),
            variables,
            civil: Some(civil),
            write: None,
            api_calls: 1,
        })
    }
    pub fn matches(&self, call: &crate::machine::NativeCall) -> bool {
        self.call == *call
    }
    pub fn poll(
        &mut self,
        file: &mut NativeSystemFile,
        machine: &mut crate::machine::Machine,
        call: &crate::machine::NativeCall,
        host: &mut dyn crysirror_core::service::HostServices,
    ) -> Result<bool> {
        ensure!(self.matches(call), "NNN system-save receipt changed");
        machine.validate_receipt(call)?;
        if !self.poll_storage(file, call, host)? {
            return Ok(false);
        }
        self.acknowledge(machine, call)?;
        Ok(true)
    }
    /// ResultData follows SystemFile in the native callback. Keep its one VM
    /// acknowledgement separate from the first storage completion.
    pub(crate) fn acknowledge(
        &self,
        machine: &mut crate::machine::Machine,
        call: &crate::machine::NativeCall,
    ) -> Result<()> {
        ensure!(self.matches(call), "NNN system-save receipt changed");
        machine.acknowledge_with_variables(call, self.variables.clone())
    }
    pub(crate) fn poll_storage(
        &mut self,
        file: &mut NativeSystemFile,
        call: &crate::machine::NativeCall,
        host: &mut dyn crysirror_core::service::HostServices,
    ) -> Result<bool> {
        ensure!(
            self.matches(call),
            "NNN system-save storage receipt changed"
        );
        if let Some(write) = self.write.as_mut() {
            if !write.poll(file)? {
                return Ok(false);
            }
            return Ok(true);
        }
        let sample = match self
            .civil
            .as_mut()
            .context("NNN system-save clock dependency absent")?
            .poll()?
        {
            crysirror_core::service::CivilTimeState::Pending => return Ok(false),
            crysirror_core::service::CivilTimeState::Failed(reason) => {
                anyhow::bail!("NNN system-save civil time: {reason}")
            }
            crysirror_core::service::CivilTimeState::Ready(time) => time,
        };
        let stamp = SystemStamp::from_civil(sample, self.client)?;
        self.write = Some(file.begin_save(
            host.storage(self.client)?.as_ref(),
            &self.variables,
            VariableProfile::Legacy1000,
            stamp,
            false,
        )?);
        self.api_calls += 1;
        self.civil = None;
        Ok(false)
    }
}

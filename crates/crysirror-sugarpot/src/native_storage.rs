// SPDX-License-Identifier: MPL-2.0
//! Tsukumo-2012 native storage, not a framework-snapshot envelope.
//! 453F00 reads and 453C90 writes the fixed 0x50C-byte system.dat body.
//! Native file operations run on an engine worker with client-scoped authority.
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::ClientStorage,
};
use crysirror_tasks::Cancellation;
use encoding_rs::SHIFT_JIS;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub(crate) mod slot;

pub(crate) const SYSTEM_SIZE: usize = 0x50c;
const SAVE_LIMIT: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SystemSettings {
    /// Native globals, in file order: 4208,420C,4210,4214,4218,4224,
    /// 4228,422C,4634,421C,4220 (all addresses relative to 0x4B0000).
    pub header: [i32; 11],
    pub font: String,
    /// 41F8,59FC,56B4,34C4,4664,463C,4640,4644,4648,4650,
    /// 4654,4658,465C. Keep raw values; adapters map reviewed meanings.
    pub values: [i32; 13],
    /// Fixed 128-word native banks at 4B4230 and 4B4430.
    pub character_flags: Vec<i32>,
    pub character_enabled: Vec<i32>,
}
impl Default for SystemSettings {
    fn default() -> Self {
        // 41F0E0 initializes the preference defaults; 453F00 installs them
        // only on a genuinely absent file. A malformed file is not absence.
        Self {
            header: [0, 1, 1, 1, 0, 70, 0, 45, 3, 1, 1],
            font: "ＭＳ ゴシック".into(),
            values: [20, 1, 1, 0, 3, 0, 50, 1, 0, 0, 70, 0, 1],
            character_flags: vec![0; 128],
            character_enabled: vec![1; 128],
        }
    }
}
impl SystemSettings {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        ensure!(
            bytes.len() == SYSTEM_SIZE,
            "SugarPot system.dat must be 0x50C bytes"
        );
        let mut cursor = Cursor::new(bytes);
        let mut header = [0; 11];
        for value in &mut header {
            *value = cursor.word()?;
        }
        let font = cursor.string(32)?;
        let mut values = [0; 13];
        for value in &mut values {
            *value = cursor.word()?;
        }
        let mut bank = || (0..128).map(|_| cursor.word()).collect::<Result<Vec<_>>>();
        Ok(Self {
            header,
            font,
            values,
            character_flags: bank()?,
            character_enabled: bank()?,
        })
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        ensure!(
            self.character_flags.len() == 128 && self.character_enabled.len() == 128,
            "SugarPot system character-bank length"
        );
        let mut out = Vec::with_capacity(SYSTEM_SIZE);
        for value in self.header {
            put_word(&mut out, value);
        }
        put_string(&mut out, &self.font, 32)?;
        for value in self.values {
            put_word(&mut out, value);
        }
        for value in self.character_flags.iter().chain(&self.character_enabled) {
            put_word(&mut out, *value);
        }
        ensure!(
            out.len() <= SYSTEM_SIZE,
            "SugarPot system.dat field overflow"
        );
        // Native writer allocates a zeroed 0x50C-byte buffer before packing.
        out.resize(SYSTEM_SIZE, 0);
        Ok(out)
    }
}

/// Native packed strings consume (strlen + 4) >> 2 DWORDs, including NUL.
struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}
impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }
    fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        let end = self
            .position
            .checked_add(count)
            .context("SugarPot save offset overflow")?;
        let out = self
            .bytes
            .get(self.position..end)
            .context("truncated SugarPot save field")?;
        self.position = end;
        Ok(out)
    }
    fn word(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into()?))
    }
    fn string(&mut self, capacity: usize) -> Result<String> {
        let tail = self
            .bytes
            .get(self.position..)
            .context("SugarPot save string offset")?;
        let length = tail
            .iter()
            .take(capacity)
            .position(|b| *b == 0)
            .context("unterminated SugarPot save string")?;
        let packed = self.take((length + 4) & !3)?;
        SHIFT_JIS
            .decode_without_bom_handling_and_without_replacement(&packed[..length])
            .map(|s| s.into_owned())
            .context("invalid SugarPot save CP932 string")
    }
}
fn put_word(out: &mut Vec<u8>, value: i32) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn put_string(out: &mut Vec<u8>, value: &str, capacity: usize) -> Result<()> {
    ensure!(!value.contains('\0'), "SugarPot save string contains NUL");
    let (bytes, _, invalid) = SHIFT_JIS.encode(value);
    ensure!(
        !invalid && bytes.len() < capacity,
        "SugarPot save string CP932/capacity"
    );
    let count = (bytes.len() + 4) & !3;
    ensure!(
        out.len()
            .checked_add(count)
            .is_some_and(|size| size <= 0x10000),
        "SugarPot packed save string budget"
    );
    out.extend_from_slice(&bytes);
    out.resize(out.len() + count - bytes.len(), 0);
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) enum NativeValue {
    /// Preserve all IEEE-754 bits, including signed zero and NaN payloads.
    FloatBits(u32),
    Integer(i32),
    Text(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Binding {
    pub name: String,
    pub value: NativeValue,
}

/// 454480/4546C0: name bytes are shifted by -97, with 0x9F terminator.
/// Name storage uses (strlen+5)>>2 DWORDs, unlike ordinary packed strings.
pub(crate) fn decode_globals(bytes: &[u8]) -> Result<Vec<Binding>> {
    ensure!(
        bytes.len() <= 0x10000 && bytes.len() % 4 == 0,
        "SugarPot global.dat size"
    );
    let mut cursor = Cursor::new(bytes);
    let mut bindings = Vec::new();
    loop {
        if cursor.bytes.get(cursor.position..cursor.position + 4) == Some(b"RF\0\0") {
            cursor.take(4)?;
            ensure!(
                cursor.position == bytes.len(),
                "SugarPot global.dat trailing data"
            );
            return Ok(bindings);
        }
        ensure!(bindings.len() < 8192, "SugarPot global binding limit");
        let tail = &cursor.bytes[cursor.position..];
        let length = tail
            .iter()
            .take(243)
            .position(|b| *b == 0x9f)
            .context("SugarPot global name terminator")?;
        let packed = cursor.take((length + 5) & !3)?;
        let raw = packed[..length]
            .iter()
            .map(|b| b.wrapping_add(97))
            .collect::<Vec<_>>();
        let name = SHIFT_JIS
            .decode_without_bom_handling_and_without_replacement(&raw)
            .context("SugarPot global name CP932")?
            .into_owned();
        let kind = cursor.word()?;
        let value = read_value(&mut cursor, kind)?;
        bindings.push(Binding { name, value });
    }
}
pub(crate) fn encode_globals(bindings: &[Binding]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    for binding in bindings {
        let (name, _, invalid) = SHIFT_JIS.encode(&binding.name);
        ensure!(
            !invalid && name.len() < 243 && !name.contains(&0),
            "SugarPot global name CP932/capacity"
        );
        let size = (name.len() + 5) & !3;
        let start = out.len();
        out.extend(name.iter().map(|b| b.wrapping_sub(97)));
        out.push(0x9f);
        out.resize(start + size, 0);
        write_value(&mut out, &binding.value)?;
        ensure!(out.len() <= 0x10000 - 4, "SugarPot global.dat budget");
    }
    out.extend_from_slice(b"RF\0\0");
    Ok(out)
}
fn read_value(cursor: &mut Cursor<'_>, kind: i32) -> Result<NativeValue> {
    Ok(match kind {
        33 => NativeValue::FloatBits(cursor.word()? as u32),
        37 => NativeValue::Integer(cursor.word()?),
        36 => NativeValue::Text(cursor.string(0x10000)?),
        _ => anyhow::bail!("unsupported SugarPot native stored value type {kind}"),
    })
}
fn write_value(out: &mut Vec<u8>, value: &NativeValue) -> Result<()> {
    match value {
        NativeValue::FloatBits(bits) => {
            put_word(out, 33);
            put_word(out, *bits as i32);
        }
        NativeValue::Integer(value) => {
            put_word(out, 37);
            put_word(out, *value);
        }
        NativeValue::Text(value) => {
            put_word(out, 36);
            put_string(out, value, 0x10000)?;
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ReadFlag {
    pub name: String,
    pub kind: i32,
    pub bits: u32,
}
/// 454B70 stores name/type/raw DWORD entries, followed by one zero DWORD.
pub(crate) fn decode_read_flags(bytes: &[u8]) -> Result<Vec<ReadFlag>> {
    ensure!(
        bytes.len() <= 0x40000 && bytes.len() % 4 == 0,
        "SugarPot readal.dat size"
    );
    let mut cursor = Cursor::new(bytes);
    let mut out = Vec::new();
    loop {
        if cursor.bytes.get(cursor.position..cursor.position + 4) == Some(&[0; 4]) {
            cursor.take(4)?;
            ensure!(
                cursor.position == bytes.len(),
                "SugarPot readal.dat trailing data"
            );
            return Ok(out);
        }
        let name = cursor.string(243)?;
        let kind = cursor.word()?;
        let bits = cursor.word()? as u32;
        ensure!(out.len() < 21845, "SugarPot read flag limit");
        out.push(ReadFlag { name, kind, bits });
    }
}
pub(crate) fn encode_read_flags(flags: &[ReadFlag]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    for flag in flags {
        ensure!(!flag.name.is_empty(), "SugarPot read flag name is empty");
        put_string(&mut out, &flag.name, 243)?;
        put_word(&mut out, flag.kind);
        put_word(&mut out, flag.bits as i32);
        ensure!(out.len() <= 0x40000 - 4, "SugarPot readal.dat budget");
    }
    put_word(&mut out, 0);
    Ok(out)
}

/// 4561A0: native slot 99 is the quick slot file00, other slots are +1.
pub(crate) fn slot_path(slot: i32) -> Result<VfsPath> {
    ensure!((0..=100).contains(&slot), "SugarPot save slot out of range");
    let number = if slot == 99 { 0 } else { slot + 1 };
    Ok(VfsPath::new(Mount::Save, &format!("file{number:02}.dat"))?)
}
/// Call only from an engine-owned task. Missing is distinct from corruption,
/// permission denial and unsupported storage providers.
pub(crate) fn read_file(
    storage: Arc<dyn ClientStorage>,
    path: &VfsPath,
    cancel: &Cancellation,
) -> Result<Option<Vec<u8>>> {
    ensure!(
        path.mount() == Mount::Save,
        "SugarPot read requires save capability"
    );
    cancel.check()?;
    let Some(meta) = storage.metadata(Mount::Save, path.relative())? else {
        return Ok(None);
    };
    ensure!(
        !meta.is_directory && meta.size.is_none_or(|size| size <= SAVE_LIMIT),
        "SugarPot native save size/type"
    );
    let reader = storage.open_read(path)?;
    let bytes = crysirror_resource::read_all(reader.as_ref(), SAVE_LIMIT)?;
    cancel.check()?;
    Ok(Some(bytes))
}
pub(crate) fn write_file(
    storage: Arc<dyn ClientStorage>,
    path: &VfsPath,
    bytes: &[u8],
    cancel: &Cancellation,
) -> Result<()> {
    ensure!(
        path.mount() == Mount::Save && bytes.len() as u64 <= SAVE_LIMIT,
        "SugarPot write requires bounded save capability"
    );
    cancel.check()?;
    storage.write_atomic(path, bytes)?;
    Ok(())
}

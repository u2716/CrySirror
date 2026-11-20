// SPDX-License-Identifier: MPL-2.0
//! Native CSV gameplay-slot header scan through the separate Save capability.

use anyhow::{Context, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::ClientStorage,
    Error,
};
use crysirror_resource::read_exact_at;

const MAX_HEADER: usize = 512;

fn packed_timestamp(timestamp: &[u8]) -> u32 {
    let digit = |at: usize| i32::from(timestamp[at] as i8) - i32::from(b'0');
    let pair = |at: usize| (10 * digit(at) + digit(at + 1)) as u32;
    (pair(2) << 25)
        | ((pair(5) & 15) << 21)
        | ((pair(8) & 31) << 16)
        | ((pair(11) & 31) << 11)
        | ((pair(14) & 63) << 5)
        | ((pair(17) >> 1) & 31)
}

fn csv_version(bytes: &[u8]) -> Option<u8> {
    let signature = bytes.get(..4)?;
    (&signature[..3] == b"CSV" && (b'2'..=b'9').contains(&signature[3]))
        .then(|| signature[3] - b'0')
}

#[cfg(feature = "runtime")]
fn cvns_signature(header: &[u8]) -> bool {
    header.starts_with(b"CSP") && header.get(3).is_some_and(|v| (b'1'..=b'9').contains(v))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveHeader {
    pub version: u8,
    pub label: Vec<u8>,
    ordering_key: u32,
}

impl SaveHeader {
    fn parse(bytes: &[u8]) -> Option<Self> {
        let version = csv_version(bytes)?;
        let tail = bytes.get(16..)?;
        let label = &tail[..tail.iter().position(|&byte| byte == 0)?];
        let timestamp = label.get(..19)?;
        for (index, &byte) in timestamp.iter().enumerate() {
            let valid = match index {
                4 | 7 => byte == b'-',
                10 => byte == b' ',
                13 | 16 => byte == b':',
                _ => byte.is_ascii_digit(),
            };
            if !valid {
                return None;
            }
        }
        Some(Self {
            version,
            label: label.to_vec(),
            ordering_key: packed_timestamp(timestamp),
        })
    }
}

#[cfg(feature = "runtime")]
const SLOT_HEADER_BYTES: usize = 64;

#[cfg(feature = "runtime")]
pub(crate) type SlotHeaderScratch = [u8; SLOT_HEADER_BYTES];

#[cfg(feature = "runtime")]
const SLOT_NAME_CYCLE: u32 = 1000;
#[cfg(feature = "runtime")]
const SLOT_PREFIX_BYTES: usize = 600;
#[cfg(feature = "runtime")]
const SLOT_COMMENT_OFFSET: usize = 272;

#[cfg(feature = "runtime")]
pub(crate) fn latest_slot_range(first: u32, count: u32) -> std::ops::Range<u32> {
    let end = first.wrapping_add(count);
    if (first as i32) < 0 || (end as i32) > 999 || first >= end {
        return 0..0;
    }
    first..end
}

#[cfg(feature = "runtime")]
pub(crate) fn scan_latest_slot(
    storage: &dyn ClientStorage,
    base: Option<(&VfsPath, bool)>,
    range: std::ops::Range<u32>,
    scratch: SlotHeaderScratch,
    check_cancel: impl FnMut() -> Result<()>,
) -> Result<(u32, SlotHeaderScratch)> {
    scan_slot_range(storage, base, range.start, range.end.saturating_sub(range.start),
        scratch, |h| csv_version(h).is_some(), check_cancel)
}

#[cfg(feature = "runtime")]
pub(crate) fn latest_cvns_slot(
    storage: &dyn ClientStorage, base: Option<(&VfsPath, bool)>, first: u32, count: u32,
    scratch: SlotHeaderScratch, check: impl FnMut() -> Result<()>,
) -> Result<(u32, SlotHeaderScratch)> {
    // 0x42fda0 checks twice the starting index, and 0x4362e0 opens first + 1.
    if first as i32 <= 0 || first.wrapping_mul(2) as i32 > 257 || first >= first.wrapping_add(count) {
        return Ok((u32::MAX, scratch));
    }
    scan_slot_range(storage, base, first + 1, count, scratch, cvns_signature, check)
}

#[cfg(feature = "runtime")]
fn scan_slot_range(
    storage: &dyn ClientStorage, base: Option<(&VfsPath, bool)>, first: u32, count: u32,
    mut scratch: SlotHeaderScratch, valid: impl Fn(&[u8]) -> bool, check: impl FnMut() -> Result<()>,
) -> Result<(u32, SlotHeaderScratch)> {
    if count == 0 { return Ok((u32::MAX, scratch)); }
    let suffix = first % SLOT_NAME_CYCLE;
    let headers = read_slot_headers(storage, base, suffix..suffix + count.min(SLOT_NAME_CYCLE), check)?;

    // The original %z3 repeats names every 1,000 slots. A full pass
    // overwrites the same scratch prefix on each iteration. A second pass
    // scores every file after that prefix is initialized. Later full passes
    // produce the same scores, and strict comparison keeps earlier winners.
    let prefix_end = count.min(2 * SLOT_NAME_CYCLE);
    let tail_start = prefix_end.max(count - count % SLOT_NAME_CYCLE);
    let mut newest = 0;
    let mut slot = u32::MAX;
    for offset in (0..prefix_end).chain(tail_start..count) {
        let candidate = first.wrapping_add(offset);
        let Some((bytes, len)) = headers.get(&(candidate % SLOT_NAME_CYCLE)) else {
            continue;
        };
        scratch[..*len].copy_from_slice(&bytes[..*len]);
        if valid(&scratch) {
            let key = packed_timestamp(&scratch[16..]);
            if key > newest {
                newest = key;
                slot = candidate;
            }
        }
    }
    Ok((slot, scratch))
}

#[cfg(feature = "runtime")]
pub(crate) fn probe_save_slot(
    storage: &dyn ClientStorage,
    base: Option<(&VfsPath, bool)>,
    slot: u32,
    scratch: SlotHeaderScratch,
    check_cancel: impl FnMut() -> Result<()>,
) -> Result<(u32, SlotHeaderScratch)> {
    if slot >= SLOT_NAME_CYCLE {
        return Ok((0, scratch));
    }
    probe_header(storage, base, slot, scratch, |h| csv_version(h).is_some(), check_cancel)
}

#[cfg(feature = "runtime")]
pub(crate) fn probe_cvns_slot(storage: &dyn ClientStorage, base: Option<(&VfsPath, bool)>,
    slot: u32, scratch: SlotHeaderScratch, check: impl FnMut() -> Result<()>,
) -> Result<(u32, SlotHeaderScratch)> {
    if slot == 0 { return Ok((0, scratch)); }
    probe_header(storage, base, slot % SLOT_NAME_CYCLE, scratch, cvns_signature, check)
}

#[cfg(feature = "runtime")]
fn probe_header(storage: &dyn ClientStorage, base: Option<(&VfsPath, bool)>,
    slot: u32, mut scratch: SlotHeaderScratch, valid: impl Fn(&[u8]) -> bool, check: impl FnMut() -> Result<()>,
) -> Result<(u32, SlotHeaderScratch)> {
    let headers = read_slot_headers(storage, base, slot..slot + 1, check)?;
    let Some((bytes, len)) = headers.get(&slot) else {
        return Ok((0, scratch));
    };
    scratch[..*len].copy_from_slice(&bytes[..*len]);
    Ok((u32::from(valid(&scratch)), scratch))
}

#[cfg(feature = "runtime")]
fn read_slot_headers(
    storage: &dyn ClientStorage,
    base: Option<(&VfsPath, bool)>,
    range: std::ops::Range<u32>,
    mut check_cancel: impl FnMut() -> Result<()>,
) -> Result<std::collections::BTreeMap<u32, (SlotHeaderScratch, usize)>> {
    let mut headers = std::collections::BTreeMap::new();
    for (suffix, path) in slot_paths(storage, base, range)? {
        check_cancel()?;
        let reader = match storage.open_read(&path) {
            Ok(reader) => reader,
            Err(Error::NotFound) => continue,
            Err(Error::Invalid("read stream must be a file")) => {
                headers.insert(suffix, ([0; SLOT_HEADER_BYTES], 0));
                continue;
            }
            Err(error) => {
                return Err(error).with_context(|| format!("CMVS read save slot {suffix}"))
            }
        };
        let mut bytes = [0; SLOT_HEADER_BYTES];
        let len = reader.len()?.min(bytes.len() as u64) as usize;
        read_exact_at(reader.as_ref(), 0, &mut bytes[..len])?;
        headers.insert(suffix, (bytes, len));
    }
    Ok(headers)
}

#[cfg(feature = "runtime")]
fn slot_paths(
    storage: &dyn ClientStorage,
    base: Option<(&VfsPath, bool)>,
    range: std::ops::Range<u32>,
) -> Result<std::collections::BTreeMap<u32, VfsPath>> {
    use std::collections::BTreeMap;

    let (directory, prefix) = match base {
        Some((path, true)) => (path.relative(), ""),
        Some((path, false)) => path
            .relative()
            .rsplit_once('/')
            .unwrap_or(("", path.relative())),
        None => ("", ""),
    };
    let resolved = if directory.is_empty() {
        None
    } else {
        let directory = VfsPath::new(Mount::Save, directory)?;
        match crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(
            storage, &directory,
        ) {
            Ok(Some(resolved)) => Some(resolved),
            Ok(None) => return Ok(BTreeMap::new()),
            Err(error) if error.downcast_ref::<Error>() == Some(&Error::NotFound) => {
                return Ok(BTreeMap::new());
            }
            Err(error) => return Err(error),
        }
    };
    let directory = resolved.as_ref().map_or("", VfsPath::relative);
    let names = match storage.list(Mount::Save, directory) {
        Ok(names) => names,
        Err(Error::NotFound) => return Ok(BTreeMap::new()),
        Err(error) => return Err(error.into()),
    };
    let mut paths = BTreeMap::new();
    for name in names {
        let Some((head, file)) = name.split_at_checked(prefix.len()) else {
            continue;
        };
        let bytes = file.as_bytes();
        if !head.eq_ignore_ascii_case(prefix)
            || bytes.len() != 11
            || !bytes[..4].eq_ignore_ascii_case(b"save")
            || !bytes[7..].eq_ignore_ascii_case(b".dat")
            || !bytes[4..7].iter().all(u8::is_ascii_digit)
        {
            continue;
        }
        let suffix = bytes[4..7]
            .iter()
            .fold(0, |value, &byte| value * 10 + u32::from(byte - b'0'));
        let first_suffix = range.start % SLOT_NAME_CYCLE;
        let offset = (suffix + SLOT_NAME_CYCLE - first_suffix) % SLOT_NAME_CYCLE;
        if offset >= range.end - range.start {
            continue;
        }
        let relative = if directory.is_empty() {
            name
        } else {
            format!("{directory}/{name}")
        };
        let path = VfsPath::new(Mount::Save, &relative)?;
        anyhow::ensure!(
            paths.insert(suffix, path).is_none(),
            "ambiguous CMVS save slot {suffix}"
        );
    }

    Ok(paths)
}

#[cfg(feature = "runtime")]
pub(crate) enum SlotHeaderEdit {
    Comment(Vec<u8>),
    Values([u32; 2]),
}

#[cfg(all(test, feature = "runtime"))]
fn edit_save_slot(
    storage: &dyn ClientStorage,
    base: Option<(&VfsPath, bool)>,
    slot: u32,
    edit: SlotHeaderEdit,
    check_cancel: impl Fn() -> Result<()>,
) -> Result<bool> {
    edit_slot_header(storage, base, slot, edit, false, check_cancel)
}

#[cfg(feature = "runtime")]
pub(crate) fn edit_slot_header(
    storage: &dyn ClientStorage, base: Option<(&VfsPath, bool)>, slot: u32, edit: SlotHeaderEdit,
    cvns: bool, check_cancel: impl Fn() -> Result<()>,
) -> Result<bool> {
    const VALUE_OFFSET: usize = 532;

    if (cvns && slot as i32 <= 0) || (!cvns && slot >= SLOT_NAME_CYCLE) {
        return Ok(false);
    }
    let Some((path, reader)) = open_save_slot(storage, base, slot)? else {
        return Ok(false);
    };
    if reader.len()? < SLOT_PREFIX_BYTES as u64 {
        return Ok(false);
    }
    let mut signature = [0; 4];
    read_exact_at(reader.as_ref(), 0, &mut signature)?;
    let valid = |header: &[u8]| if cvns { cvns_signature(header) } else { csv_version(header).is_some() };
    if !valid(&signature) {
        return Ok(false);
    }
    let mut bytes = crysirror_resource::io::read_all_checked(
        reader.as_ref(),
        crysirror_runtime::vfs::MAX_FILE,
        &check_cancel,
    )?;
    let Some(header) = bytes.get_mut(..SLOT_PREFIX_BYTES) else {
        return Ok(false);
    };
    if !valid(header) {
        return Ok(false);
    }
    match edit {
        SlotHeaderEdit::Comment(comment) => {
            // The original writes the 600-byte prefix after the comment copy.
            let field = &mut header[SLOT_COMMENT_OFFSET..];
            let len = comment.len().min(field.len());
            field[..len].copy_from_slice(&comment[..len]);
            if let Some(terminator) = field.get_mut(len) {
                *terminator = 0;
            }
        }
        SlotHeaderEdit::Values([first, second]) => {
            header[VALUE_OFFSET..VALUE_OFFSET + 4].copy_from_slice(&first.to_le_bytes());
            header[VALUE_OFFSET + 4..VALUE_OFFSET + 8].copy_from_slice(&second.to_le_bytes());
        }
    }
    check_cancel()?;
    storage
        .write_atomic_existing_parent(&path, &bytes)
        .with_context(|| format!("CMVS update save slot {slot}"))?;
    Ok(true)
}

#[cfg(feature = "runtime")]
pub(crate) fn open_save_slot(
    storage: &dyn ClientStorage,
    base: Option<(&VfsPath, bool)>,
    slot: u32,
) -> Result<
    Option<(
        VfsPath,
        std::sync::Arc<dyn crysirror_core::service::AssetReader>,
    )>,
> {
    let suffix = slot % SLOT_NAME_CYCLE;
    let Some(path) = slot_paths(storage, base, suffix..suffix + 1)?.remove(&suffix) else {
        return Ok(None);
    };
    match storage.open_read(&path) {
        Ok(reader) => Ok(Some((path, reader))),
        Err(Error::NotFound | Error::Invalid("read stream must be a file")) => Ok(None),
        Err(error) => Err(error).with_context(|| format!("CMVS read save slot {slot}")),
    }
}

#[cfg(feature = "runtime")]
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SlotMetadata {
    pub timestamp: Vec<u8>,
    pub comment: Vec<u8>,
    pub date_result: u32,
    pub values: [u32; 3],
    pub preview: Option<SlotPreview>,
}

#[cfg(feature = "runtime")]
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SlotPreview {
    Black,
    Encoded(Vec<u8>),
}

#[cfg(all(test, feature = "runtime"))]
fn read_slot_metadata(
    storage: &dyn ClientStorage,
    base: Option<(&VfsPath, bool)>,
    slot: i32,
    wants_image: bool,
    check_cancel: impl Fn() -> Result<()>,
) -> Result<Option<SlotMetadata>> {
    check_cancel()?;
    let Ok(slot) = u32::try_from(slot) else {
        return Ok(None);
    };
    read_gameplay_metadata(storage, base, slot, wants_image, false, check_cancel)
}

#[cfg(feature = "runtime")]
pub(crate) fn read_gameplay_metadata(
    storage: &dyn ClientStorage, base: Option<(&VfsPath, bool)>, slot: u32,
    wants_image: bool, cvns: bool, check_cancel: impl Fn() -> Result<()>,
) -> Result<Option<SlotMetadata>> {
    check_cancel()?;
    let Some((_, reader)) = open_save_slot(storage, base, slot)? else {
        return Ok(None);
    };
    if reader.len()? < SLOT_PREFIX_BYTES as u64 {
        return Ok(None);
    }
    let mut header = [0; SLOT_PREFIX_BYTES];
    read_exact_at(reader.as_ref(), 0, &mut header)?;
    if !(if cvns { cvns_signature(&header) } else { csv_version(&header).is_some() }) {
        return Ok(None);
    }
    let string = |offset: usize| -> Result<Vec<u8>> {
        let field: &[u8] = &header[offset..];
        let len = field
            .iter()
            .position(|&byte| byte == 0)
            .context("unterminated CMVS save header string")?;
        Ok(field[..len].to_vec())
    };
    let word = |offset: usize| u32::from_le_bytes(header[offset..offset + 4].try_into().unwrap());
    let timestamp = string(16)?;
    let comment = string(SLOT_COMMENT_OFFSET)?;
    let flag = u16::from_le_bytes(header[548..550].try_into().unwrap());
    let preview = if !wants_image || flag == 0 {
        None
    } else if word(564) == 0 {
        Some(SlotPreview::Black)
    } else {
        let size = word(584) as usize;
        let limit = crysirror_image::MAX_DECODED_BYTES;
        anyhow::ensure!((1..=limit).contains(&size), "CMVS preview decoded size");
        let payload = crysirror_resource::subrange(
            reader,
            SLOT_PREFIX_BYTES as u64 + u64::from(word(560)),
            u64::from(word(564)),
        )?;
        let packed = crysirror_resource::io::read_all_checked(
            payload.as_ref(),
            limit as u64 + limit.div_ceil(8) as u64,
            &check_cancel,
        )?;
        let bytes = crysirror_resource::lzss_decompress(&packed, size, &crate::script::PS2A_LZSS)
            .context("expand CMVS save preview")?;
        Some(SlotPreview::Encoded(bytes))
    };
    check_cancel()?;
    Ok(Some(SlotMetadata {
        timestamp,
        comment,
        date_result: packed_timestamp(&header[16..]),
        values: [word(528), word(532), word(536)],
        preview,
    }))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveSlot {
    Missing,
    Invalid,
    Recognized(SaveHeader),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveMenu {
    pub slots: Vec<SaveSlot>,
    pub latest: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveMenuRequest {
    pub(super) session: u64,
    pub(super) generation: u64,
    pub(super) pc: usize,
    pub(super) root: VfsPath,
    pub(super) count: u16,
}

pub struct PreparedSaveMenu {
    pub(super) request: SaveMenuRequest,
    pub(super) menu: SaveMenu,
}

impl PreparedSaveMenu {
    pub fn menu(&self) -> &SaveMenu {
        &self.menu
    }
}

impl SaveMenuRequest {
    /// Run on a storage worker. Only slot headers in the writable capability
    /// are read. Transport errors abort the candidate before publication.
    pub fn prepare(self, storage: &dyn ClientStorage) -> Result<PreparedSaveMenu> {
        let mut slots = Vec::with_capacity(usize::from(self.count));
        let mut latest = None;
        let mut newest = 0;
        for slot in 0..self.count {
            let path = VfsPath::new(
                Mount::Save,
                &format!("{}/save{slot:03}.dat", self.root.relative()),
            )?;
            let status = match storage.open_read(&path) {
                Err(Error::NotFound) => SaveSlot::Missing,
                result => {
                    let input = result.with_context(|| format!("CMVS read save slot {slot}"))?;
                    let mut header = [0; MAX_HEADER];
                    let size = input.len()?.min(MAX_HEADER as u64) as usize;
                    read_exact_at(input.as_ref(), 0, &mut header[..size])?;
                    match SaveHeader::parse(&header[..size]) {
                        Some(header) => {
                            if header.ordering_key > newest {
                                newest = header.ordering_key;
                                latest = Some(slot);
                            }
                            SaveSlot::Recognized(header)
                        }
                        None => SaveSlot::Invalid,
                    }
                }
            };
            slots.push(status);
        }
        Ok(PreparedSaveMenu {
            request: self,
            menu: SaveMenu { slots, latest },
        })
    }
}

/// 0x436040 scans CSP1..CSP9, starting at file 001. The checked index
/// is zero-based. Labels and ordering both come from the header at +16.
#[cfg(feature = "runtime")]
pub(crate) fn scan_cvns_menu(
    storage: &dyn ClientStorage,
    base: Option<(&VfsPath, bool)>,
    count: u32,
    check: impl Fn() -> Result<()>,
) -> Result<SaveMenu> {
    let count = count.min(256);
    let mut result = SaveMenu { slots: Vec::with_capacity(count as usize), latest: None };
    if count == 0 {
        return Ok(result);
    }
    let paths = slot_paths(storage, base, 1..count + 1)?;
    let mut newest = 0;
    for number in 1..=count {
        check()?;
        let Some(path) = paths.get(&number) else {
            result.slots.push(SaveSlot::Missing);
            continue;
        };
        let input = match storage.open_read(path) {
            Err(Error::NotFound) => {
                result.slots.push(SaveSlot::Missing);
                continue;
            }
            other => other?,
        };
        let mut prefix = [0; 600];
        let size = input.len()?.min(prefix.len() as u64) as usize;
        read_exact_at(input.as_ref(), 0, &mut prefix[..size])?;
        let header = &prefix[..size];
        let status = if header.len() >= 35 && cvns_signature(header) {
            let end = header[16..].iter().position(|&b| b == 0).context("unterminated CVNS slot label")?;
            let key = packed_timestamp(&header[16..35]);
            if key > newest {
                newest = key;
                result.latest = Some((number - 1) as u16);
            }
            SaveSlot::Recognized(SaveHeader { version: header[3] - b'0', label: header[16..16 + end].to_vec(), ordering_key: key })
        } else {
            SaveSlot::Invalid
        };
        result.slots.push(status);
    }
    Ok(result)
}

#[cfg(all(test, feature = "runtime"))]
mod latest_tests {
    use super::*;
    use crysirror_core::service::{AssetReader, MemoryAssetReader};
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    struct Storage {
        files: Mutex<BTreeMap<&'static str, Vec<u8>>>,
        writable: bool,
    }

    impl Storage {
        fn read_only(files: BTreeMap<&'static str, Vec<u8>>) -> Self {
            Self {
                files: Mutex::new(files),
                writable: false,
            }
        }
    }

    impl ClientStorage for Storage {
        fn read(&self, path: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
            assert_eq!(path.mount(), Mount::Save);
            self.files
                .lock()
                .unwrap()
                .get(path.relative())
                .cloned()
                .ok_or(Error::NotFound)
        }

        fn open_read(&self, path: &VfsPath) -> crysirror_core::Result<Arc<dyn AssetReader>> {
            Ok(Arc::new(MemoryAssetReader(self.read(path)?.into())))
        }

        fn write_atomic(&self, _: &VfsPath, _: &[u8]) -> crysirror_core::Result<()> {
            panic!("slot metadata must use the existing parent")
        }

        fn write_atomic_existing_parent(
            &self,
            path: &VfsPath,
            bytes: &[u8],
        ) -> crysirror_core::Result<()> {
            assert_eq!(path.mount(), Mount::Save);
            if !self.writable {
                return Err(Error::ReadOnly);
            }
            let mut files = self.files.lock().unwrap();
            let existing = files.get_mut(path.relative()).ok_or(Error::NotFound)?;
            *existing = bytes.to_vec();
            Ok(())
        }

        fn remove(&self, _: &VfsPath) -> crysirror_core::Result<()> {
            panic!("latest slot scan must be read-only")
        }

        fn list(&self, mount: Mount, dir: &str) -> crysirror_core::Result<Vec<String>> {
            assert_eq!(mount, Mount::Save);
            match dir {
                "" => Ok(vec!["Save".into()]),
                "Save" => Ok(self
                    .files
                    .lock()
                    .unwrap()
                    .keys()
                    .filter_map(|path| path.strip_prefix("Save/"))
                    .map(str::to_owned)
                    .collect()),
                _ => Err(Error::NotFound),
            }
        }
    }

    fn header(signature: &[u8; 4], timestamp: &[u8; 19]) -> Vec<u8> {
        let mut header = vec![0; SLOT_HEADER_BYTES];
        header[..4].copy_from_slice(signature);
        header[16..35].copy_from_slice(timestamp);
        header
    }

    #[test]
    fn cvns_probes_and_scans_csp_headers_using_one_based_range() -> Result<()> {
        let storage = Storage::read_only(BTreeMap::from([
            ("Save/SAVE001.DAT", header(b"CSP1", b"2026-11-19 08:00:00")),
            ("Save/save002.dat", header(b"CSV9", b"2099-11-19 08:00:00")),
            ("Save/save003.dat", header(b"CSP9", b"2026-11-19 08:00:00")),
        ]));
        let root = VfsPath::new(Mount::Save, "save")?;
        let base = Some((&root, true));
        let scratch = [0; 64];
        assert_eq!(latest_cvns_slot(&storage, base, 1, 2, scratch, || Ok(()))?.0, 3);
        assert_eq!(latest_cvns_slot(&storage, base, 129, 10, scratch, || Ok(()))?.0, u32::MAX);
        assert_eq!(probe_cvns_slot(&storage, base, 1001, scratch, || Ok(()))?.0, 1);
        assert_eq!(probe_cvns_slot(&storage, base, 2, scratch, || Ok(()))?.0, 0);
        assert_eq!(probe_cvns_slot(&storage, base, 0, scratch, || Ok(()))?.0, 0);
        Ok(())
    }

    #[test]
    fn latest_slot_reads_scoped_headers_and_keeps_first_packed_timestamp_tie() {
        let storage = Storage::read_only(BTreeMap::from([
            ("Save/save004.dat", header(b"CSV9", b"2099-12-31 23:59:59")),
            ("Save/SAVE005.DAT", header(b"CSV2", b"2026-11-19 08:00:01")),
            ("Save/save006.dat", header(b"CSV9", b"2026/11/19 08:00:00")),
            ("Save/save007.dat", header(b"CSV1", b"2099-12-31 23:59:59")),
            (
                "Save/Prefix_save008.dat",
                header(b"CSV9", b"2026-11-19 08:20:00"),
            ),
        ]));
        let root = VfsPath::new(Mount::Save, "save").unwrap();
        let (slot, scratch) = scan_latest_slot(
            &storage,
            Some((&root, true)),
            latest_slot_range(5, 4),
            [0; SLOT_HEADER_BYTES],
            || Ok(()),
        )
        .unwrap();
        assert_eq!(slot, 5);
        assert_eq!(&scratch[..4], b"CSV1");
        let prefix = VfsPath::new(Mount::Save, "save/prefix_").unwrap();
        let (slot, _) =
            scan_latest_slot(&storage, Some((&prefix, false)), 5..9, scratch, || Ok(())).unwrap();
        assert_eq!(slot, 8);

        let missing = VfsPath::new(Mount::Save, "empty").unwrap();
        assert_eq!(
            scan_latest_slot(&storage, Some((&missing, true)), 0..8, scratch, || Ok(())).unwrap(),
            (u32::MAX, scratch),
        );
    }

    #[test]
    fn latest_slot_retains_native_wrapping_range_and_short_header_reads() {
        assert_eq!(latest_slot_range(998, 1), 998..999);
        assert!(latest_slot_range(998, 2).is_empty());
        assert!(latest_slot_range(u32::MAX, 3).is_empty());
        assert!(latest_slot_range(10, u32::MAX).is_empty());
        let range = latest_slot_range(998, 0x8000_0008);
        assert_eq!(range, 998..0x8000_03ee);

        let storage = Storage::read_only(BTreeMap::from([
            ("Save/save005.dat", header(b"CSV9", b"2026-11-19 08:00:00")),
            ("Save/save006.dat", b"CSV2".to_vec()),
        ]));
        let root = VfsPath::new(Mount::Save, "Save").unwrap();
        let (slot, scratch) = scan_latest_slot(
            &storage,
            Some((&root, true)),
            range,
            [0; SLOT_HEADER_BYTES],
            || Ok(()),
        )
        .unwrap();
        assert_eq!(slot, 1005);
        assert_eq!(&scratch[..4], b"CSV2");
        assert_eq!(&scratch[16..35], b"2026-11-19 08:00:00");

        let (slot, _) =
            scan_latest_slot(&storage, Some((&root, true)), 6..7, scratch, || Ok(())).unwrap();
        assert_eq!(slot, 6);
    }

    #[test]
    fn single_slot_probe_keeps_its_signature_scratch_and_checks_unsigned_bounds() {
        let storage = Storage::read_only(BTreeMap::from([
            ("Save/save001.dat", vec![b'C']),
            ("Save/save002.dat", b"CSV1".to_vec()),
            ("Save/save003.dat", b"XYZ9".to_vec()),
            ("Save/SAVE999.DAT", b"CSV9".to_vec()),
        ]));
        let root = VfsPath::new(Mount::Save, "save").unwrap();
        let probe = |slot, scratch| {
            probe_save_slot(&storage, Some((&root, true)), slot, scratch, || Ok(())).unwrap()
        };

        assert_eq!(probe(1, [0; SLOT_HEADER_BYTES]).0, 0);
        let (found, scratch) = probe(999, [0; SLOT_HEADER_BYTES]);
        assert_eq!(found, 1);
        assert_eq!(probe(1, scratch), (1, scratch));
        for slot in [0, 1000, u32::MAX] {
            assert_eq!(probe(slot, scratch), (0, scratch));
        }
        for slot in [2, 3] {
            let (found, updated) = probe(slot, scratch);
            assert_eq!(found, 0);
            assert_ne!(updated, scratch);
        }
    }

    #[test]
    fn slot_edits_preserve_every_other_byte_and_refuse_missing_or_truncated_files() {
        let mut original = vec![0x6a; 921];
        original[..4].copy_from_slice(b"CSV8");
        let mut storage = Storage::read_only(BTreeMap::from([
            ("Save/SAVE009.DAT", original.clone()),
            ("Save/save010.dat", b"CSV9".to_vec()),
            ("Save/save011.dat", vec![0; 921]),
        ]));
        let root = VfsPath::new(Mount::Save, "save").unwrap();
        let base = Some((&root, true));
        let slot = VfsPath::new(Mount::Save, "Save/SAVE009.DAT").unwrap();
        let comment = || SlotHeaderEdit::Comment(vec![0x82, 0xa0]);
        assert!(edit_save_slot(&storage, base, 9, comment(), || Ok(())).is_err());
        assert_eq!(storage.read(&slot).unwrap(), original);
        storage.writable = true;
        for missing in [8, 10, 11, u32::MAX] {
            assert!(!edit_save_slot(&storage, base, missing, comment(), || Ok(())).unwrap());
        }
        assert!(
            edit_save_slot(&storage, base, 9, comment(), || anyhow::bail!("cancelled")).is_err()
        );
        assert_eq!(storage.read(&slot).unwrap(), original);

        assert!(edit_save_slot(&storage, base, 9, comment(), || Ok(())).unwrap());
        original[272..275].copy_from_slice(&[0x82, 0xa0, 0]);
        assert_eq!(storage.read(&slot).unwrap(), original);
        assert!(edit_save_slot(
            &storage,
            base,
            9,
            SlotHeaderEdit::Values([0x1234_5678, u32::MAX]),
            || Ok(()),
        )
        .unwrap());
        original[532..540].copy_from_slice(&[0x78, 0x56, 0x34, 0x12, 0xff, 0xff, 0xff, 0xff]);
        assert_eq!(storage.read(&slot).unwrap(), original);
    }

    #[test]
    fn slot_metadata_recognizes_versions_and_uses_native_filename_wrapping() {
        let timestamp = b"2026-11-19 08:00:01";
        let mut metadata = header(b"CSV2", timestamp);
        metadata.resize(SLOT_PREFIX_BYTES, 0);
        metadata[272..274].copy_from_slice(&[0x82, 0xa0]);
        for (index, word) in [8u32, 99, u32::MAX].into_iter().enumerate() {
            metadata[528 + index * 4..532 + index * 4].copy_from_slice(&word.to_le_bytes());
        }
        let mut black = metadata.clone();
        black[3] = b'9';
        black[549] = 1;
        let mut bad_signature = metadata.clone();
        bad_signature[3] = b'1';
        let storage = Storage::read_only(BTreeMap::from([
            ("Save/SAVE004.DAT", metadata),
            ("Save/save005.dat", black),
            ("Save/save006.dat", bad_signature),
            ("Save/save007.dat", header(b"CSV9", timestamp)),
        ]));
        let root = VfsPath::new(Mount::Save, "save").unwrap();
        let read = |slot, wants_image| {
            read_slot_metadata(&storage, Some((&root, true)), slot, wants_image, || Ok(())).unwrap()
        };
        let result = read(1004, true).unwrap();
        assert_eq!(result.timestamp, timestamp);
        assert_eq!(result.comment, [0x82, 0xa0]);
        assert_eq!(
            result.date_result,
            (26 << 25) | (11 << 21) | (19 << 16) | (8 << 11)
        );
        assert_eq!(result.values, [8, 99, u32::MAX]);
        assert_eq!(result.preview, None);
        assert_eq!(read(1005, true).unwrap().preview, Some(SlotPreview::Black));
        assert_eq!(read(1005, false).unwrap().preview, None);
        for slot in [-1, 6, 7, 8] {
            assert!(read(slot, true).is_none());
        }
    }

    #[test]
    fn slot_preview_reads_only_its_bounded_lzss_range_and_honors_cancellation() {
        let packed = [0b0000_0111, b'x', b'y', b'z', 0xdf, 0xe1];
        let mut valid = header(b"CSV9", b"2026-11-19 08:00:01");
        valid.resize(SLOT_PREFIX_BYTES, 0);
        valid[548] = 1;
        valid[560..564].copy_from_slice(&3u32.to_le_bytes());
        valid[564..568].copy_from_slice(&(packed.len() as u32).to_le_bytes());
        valid[584..588].copy_from_slice(&6u32.to_le_bytes());
        valid.extend_from_slice(&[0xcc; 3]);
        valid.extend_from_slice(&packed);
        let mut truncated = valid.clone();
        truncated.pop();
        valid.extend_from_slice(&[0xcc; 12]);
        let storage = Storage::read_only(BTreeMap::from([
            ("Save/Prefix_save004.dat", valid),
            ("Save/Prefix_save005.dat", truncated),
        ]));
        let prefix = VfsPath::new(Mount::Save, "save/prefix_").unwrap();
        let base = Some((&prefix, false));
        let read =
            |slot, wants_image| read_slot_metadata(&storage, base, slot, wants_image, || Ok(()));
        assert_eq!(
            read(4, true).unwrap().unwrap().preview,
            Some(SlotPreview::Encoded(b"xyzxyz".to_vec()))
        );
        assert!(read(5, true).is_err());
        assert_eq!(read(5, false).unwrap().unwrap().preview, None);
        let checks = std::cell::Cell::new(0);
        assert!(read_slot_metadata(&storage, base, 4, true, || {
            checks.set(checks.get() + 1);
            anyhow::ensure!(checks.get() < 3, "preview read cancelled");
            Ok(())
        })
        .is_err());
        assert_eq!(checks.get(), 3);
    }
}

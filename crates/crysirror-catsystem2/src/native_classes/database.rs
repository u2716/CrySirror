// SPDX-License-Identifier: MPL-2.0
//! Natives 869..873 and 878: the read-marks database (82A390) over a KCDB table.
use super::Ctx;
use anyhow::{ensure, Context, Result};
use std::collections::BTreeMap;
use std::io::Read;

/// Natives implemented by this family.
pub(crate) const IDS: &[u32] = &[869, 870, 871, 872, 873, 878];

/// 82AFB0: bytes of one exported entry; the name, then the three flag words.
const ENTRY: usize = 268;
const NAME: usize = 256;

/// B307A8: kana folded before a name is compared.
const FOLDED: [([u8; 2], [u8; 2]); 34] = [
    ([0x82, 0xaa], [0x82, 0xa9]), ([0x82, 0xac], [0x82, 0xab]), ([0x82, 0xae], [0x82, 0xad]),
    ([0x82, 0xb0], [0x82, 0xaf]), ([0x82, 0xb2], [0x82, 0xb1]), ([0x82, 0xb4], [0x82, 0xb3]),
    ([0x82, 0xb6], [0x82, 0xb5]), ([0x82, 0xb8], [0x82, 0xb7]), ([0x82, 0xba], [0x82, 0xb9]),
    ([0x82, 0xbc], [0x82, 0xbb]), ([0x82, 0xbe], [0x82, 0xbd]), ([0x82, 0xc0], [0x82, 0xbf]),
    ([0x82, 0xc3], [0x82, 0xc2]), ([0x82, 0xc5], [0x82, 0xc4]), ([0x82, 0xc7], [0x82, 0xc6]),
    ([0x82, 0xce], [0x82, 0xcd]), ([0x82, 0xd1], [0x82, 0xd0]), ([0x82, 0xd4], [0x82, 0xd3]),
    ([0x82, 0xd7], [0x82, 0xd6]), ([0x82, 0xda], [0x82, 0xd9]), ([0x82, 0xcf], [0x82, 0xcd]),
    ([0x82, 0xd2], [0x82, 0xd0]), ([0x82, 0xd5], [0x82, 0xd3]), ([0x82, 0xd8], [0x82, 0xd6]),
    ([0x82, 0xdb], [0x82, 0xd9]), ([0x82, 0x9f], [0x82, 0xa0]), ([0x82, 0xa1], [0x82, 0xa2]),
    ([0x82, 0xa3], [0x82, 0xa4]), ([0x82, 0xa5], [0x82, 0xa6]), ([0x82, 0xa7], [0x82, 0xa8]),
    ([0x82, 0xe1], [0x82, 0xe2]), ([0x82, 0xe3], [0x82, 0xe4]), ([0x82, 0xe5], [0x82, 0xe6]),
    ([0x82, 0xc1], [0x82, 0xc2]),
];

fn lead_byte(byte: u8) -> bool {
    matches!(byte, 0x81..=0x9f | 0xe0..=0xfc)
}

/// 82AA90. Its second pass, for the long-vowel mark, compares a sign-extended byte with
/// 0x81 and so never changes anything.
fn fold(name: &mut [u8]) {
    let mut at = 0;
    while at < name.len() {
        if lead_byte(name[at]) && at + 1 < name.len() {
            if let Some((_, to)) = FOLDED.iter().find(|(from, _)| name[at..at + 2] == *from) {
                name[at..at + 2].copy_from_slice(to);
            }
            at += 2;
        } else {
            at += 1;
        }
    }
}

fn until_nul(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len())]
}

/// One row of the table, as 82ABD0 keeps it.
#[derive(Clone, Debug, Default)]
pub(crate) struct Row {
    index: Vec<u8>,
    title_short: Vec<u8>,
    title: Vec<u8>,
    /// IND1..IND24, empty where the cell is absent or blank.
    inds: Vec<Vec<u8>>,
}

/// 79C4E0: `KCDB`, packed and plain sizes, then zlib over the column list and fixed rows.
/// An absent ID column means no rows, as 82ABD0 stops at its first failed ID read.
fn parse_table(file: &[u8]) -> Result<Vec<Row>> {
    ensure!(file.len() >= 12 && file[..4] == *b"KCDB", "CatSystem2 database is not a KCDB table");
    let word = |bytes: &[u8], at: usize| -> Result<u32> {
        Ok(u32::from_le_bytes(bytes.get(at..at + 4).context("CatSystem2 database table is truncated")?.try_into()?))
    };
    let packed = word(file, 4)? as usize;
    let plain_size = word(file, 8)? as usize;
    ensure!(plain_size <= 1 << 28, "CatSystem2 database table size");
    let packed = file.get(12..12 + packed).context("CatSystem2 database data is truncated")?;
    let mut plain = Vec::with_capacity(plain_size);
    flate2::read::ZlibDecoder::new(packed).take(plain_size as u64 + 1).read_to_end(&mut plain)?;
    ensure!(plain.len() == plain_size, "CatSystem2 database unpacked to {} of {plain_size} bytes", plain.len());
    let columns = word(&plain, 0)? as usize;
    ensure!(columns <= plain.len() / 24, "CatSystem2 database column count");
    let mut layout: BTreeMap<Vec<u8>, (usize, usize)> = BTreeMap::new();
    let mut offset = 0usize;
    for column in 0..columns {
        let at = 4 + column * 24;
        let size = word(&plain, at + 20)? as usize;
        layout.insert(until_nul(&plain[at..at + 16]).to_vec(), (offset, size));
        offset = offset.checked_add(size).context("CatSystem2 database row layout")?;
    }
    let rows_at = 4 + columns * 24;
    let rows = word(&plain, rows_at)? as usize;
    let row_size = word(&plain, rows_at + 4)? as usize;
    if !layout.contains_key(&b"ID"[..]) {
        return Ok(Vec::new());
    }
    ensure!(layout.contains_key(&b"INDEX"[..]), "CatSystem2 database has no INDEX column");
    let mut out = Vec::new();
    for row in 0..rows {
        let start = rows_at + 8 + row * row_size;
        let cell = |name: &[u8]| -> Result<Vec<u8>> {
            let Some(&(at, size)) = layout.get(name) else { return Ok(Vec::new()) };
            Ok(until_nul(plain.get(start + at..start + at + size).context("CatSystem2 database rows are truncated")?).to_vec())
        };
        let mut inds = Vec::with_capacity(24);
        for ind in 1..=24 {
            inds.push(cell(format!("IND{ind}").as_bytes())?);
        }
        out.push(Row { index: cell(b"INDEX")?, title_short: cell(b"TITLE_S")?, title: cell(b"TITLE")?, inds });
    }
    Ok(out)
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Database {
    /// By folded name, in the map order the export uses: seen bits, ever-set bits, and
    /// whether any bit has been set.
    entries: BTreeMap<Vec<u8>, [u32; 3]>,
    rows: BTreeMap<Vec<u8>, Row>,
    /// 82C180: text before and after each index term.
    affix: (Vec<u8>, Vec<u8>),
}

impl Database {
    fn load(file: &[u8]) -> Result<Self> {
        let mut database = Self::default();
        for row in parse_table(file)? {
            let mut name = row.index.clone();
            fold(&mut name);
            database.entries.entry(name.clone()).or_default();
            database.rows.entry(name).or_insert(row);
        }
        Ok(database)
    }

    fn key(name: &[u8]) -> Vec<u8> {
        let mut key = until_nul(name).to_vec();
        fold(&mut key);
        key
    }
    /// 82B0B0: the nth name whose first character lies in the range.
    pub(crate) fn find(&self, from: &[u8], to: &[u8], nth: u32) -> Option<&[u8]> {
        let (from, to) = (first_character(from), first_character(to));
        self.entries.keys().filter(|key| (from..=to).contains(&first_character(key))).nth(nth as usize).map(Vec::as_slice)
    }
    /// 82B220: the seen bits whose index term exists.
    pub(crate) fn appear(&self, name: &[u8]) -> u32 {
        let key = Self::key(name);
        let seen = self.entries.get(&key).map_or(0, |flags| flags[0]);
        seen & self.max_appear(name)
    }
    /// 82B8E0: every bit with an index term.
    pub(crate) fn max_appear(&self, name: &[u8]) -> u32 {
        let Some(row) = self.rows.get(&Self::key(name)) else { return 0 };
        row.inds.iter().enumerate().filter(|(_, ind)| !ind.is_empty()).fold(0, |mask, (bit, _)| mask | 1 << bit)
    }
    /// 82B420: the name's 1-based place in map order, 0 when absent.
    pub(crate) fn id(&self, name: &[u8]) -> u32 {
        let key = Self::key(name);
        self.entries.keys().position(|other| *other == key).map_or(0, |at| at as u32 + 1)
    }
    /// 82B630: the seen terms, each between the affixes.
    pub(crate) fn ind(&self, name: &[u8]) -> Option<Vec<u8>> {
        let key = Self::key(name);
        let row = self.rows.get(&key)?;
        let seen = self.entries.get(&key)?[0];
        let mut out = Vec::new();
        for (bit, ind) in row.inds.iter().enumerate() {
            if seen & (1 << bit) != 0 && !ind.is_empty() {
                out.extend_from_slice(&self.affix.0);
                out.extend_from_slice(ind);
                out.extend_from_slice(&self.affix.1);
            }
        }
        Some(out)
    }
    /// 82BA90 and 82BC00.
    pub(crate) fn titles(&self, name: &[u8]) -> Option<(&[u8], &[u8])> {
        self.rows.get(&Self::key(name)).map(|row| (row.title.as_slice(), row.title_short.as_slice()))
    }
    /// 82BD70.
    pub(crate) fn update_flags(&self, name: &[u8]) -> (u32, u32) {
        self.entries.get(&Self::key(name)).map_or((0, 0), |flags| (flags[2], flags[1]))
    }
    /// 82A810 for one name, 82A970 for all.
    pub(crate) fn clear_updates(&mut self, name: Option<&[u8]>) {
        match name {
            Some(name) => {
                if let Some(flags) = self.entries.get_mut(&Self::key(name)) {
                    flags[1] = 0;
                    flags[2] = 0;
                }
            }
            None => {
                for flags in self.entries.values_mut() {
                    flags[1] = 0;
                    flags[2] = 0;
                }
            }
        }
    }

    /// 82A640 with its value at 1, the only one 646E60 passes.
    fn mark(&mut self, name: &[u8], bit: u32) {
        let mut name = until_nul(name).to_vec();
        fold(&mut name);
        if let Some(flags) = self.entries.get_mut(&name) {
            if flags[0] == 0 {
                flags[2] = 1;
            }
            let mask = 1u32 << (bit & 31);
            if flags[0] & mask == 0 {
                flags[0] |= mask;
                flags[1] |= mask;
            }
        }
    }

    /// 82AFB0.
    fn export(&self) -> Vec<u8> {
        let mut out = (self.entries.len() as u32).to_le_bytes().to_vec();
        for (name, flags) in &self.entries {
            let mut entry = [0u8; ENTRY];
            // The original's strcpy has no bound; names longer than the field cannot round-trip.
            let length = name.len().min(NAME - 1);
            entry[..length].copy_from_slice(&name[..length]);
            for (slot, flag) in flags.iter().enumerate() {
                entry[NAME + slot * 4..NAME + slot * 4 + 4].copy_from_slice(&flag.to_le_bytes());
            }
            out.extend_from_slice(&entry);
        }
        out
    }

    fn size(&self) -> u32 {
        (self.entries.len() * ENTRY + 4) as u32
    }

    /// 82BF20: a saved entry's flags come back only under a name still present.
    fn import(&mut self, saved: &[u8]) -> Result<()> {
        ensure!(saved.len() >= 4, "CatSystem2 database saved header");
        let count = u32::from_le_bytes(saved[..4].try_into()?) as usize;
        for at in 0..count {
            let entry = saved.get(4 + at * ENTRY..4 + (at + 1) * ENTRY).context("CatSystem2 database saved entries are truncated")?;
            let mut name = until_nul(&entry[..NAME]).to_vec();
            fold(&mut name);
            if let Some(flags) = self.entries.get_mut(&name) {
                for (slot, flag) in flags.iter_mut().enumerate() {
                    *flag = u32::from_le_bytes(entry[NAME + slot * 4..NAME + slot * 4 + 4].try_into()?);
                }
            }
        }
        Ok(())
    }
}

/// 6A6FF0: a name's first character, double-byte where it leads with one.
fn first_character(name: &[u8]) -> u16 {
    match name {
        [lead, second, ..] if lead_byte(*lead) => u16::from(*lead) << 8 | u16::from(*second),
        [only, ..] => u16::from(*only),
        [] => 0,
    }
}

pub(crate) fn handle(cx: &mut Ctx) -> Result<bool> {
    let a = cx.call.arguments.clone();
    match cx.call.id {
        // 646F80: the handle exists whether or not the file does; 82C0C0 only fills it.
        869 => {
            ensure!(a.len() == 4, "CatSystem2 database arguments");
            cx.machine.preflight_native(true, &[])?;
            let name = String::from_utf8(crate::native_values::format_bytes(cx.machine, a[0])?).context("CatSystem2 database name")?;
            let storage = cx.host.storage(cx.client)?;
            let catalog = crate::native_resource_paths::Catalog::load(storage.as_ref())?;
            let mut database = Database::default();
            for path in catalog.candidates(storage.as_ref(), "database", &name, &|| Ok(()))? {
                if let Some(path) = crate::native_resource_paths::resolve(storage.as_ref(), &path)? {
                    database = Database::load(&storage.read(&path)?)?;
                    break;
                }
            }
            let affix = |at: u32, cx: &Ctx| crate::native_values::format_bytes(cx.machine, at);
            database.affix = (affix(a[1], cx)?, affix(a[3], cx)?);
            let id = cx.handles.allocate()?;
            cx.settings.as_mut().context("CatSystem2 database settings")?.databases.insert(id, database);
            cx.machine.acknowledge(Some(id))?;
        }
        870 => {
            cx.machine.preflight_native(false, &[])?;
            cx.settings.as_mut().context("CatSystem2 database settings")?.databases.remove(&a[0]);
            cx.machine.acknowledge(None)?;
        }
        // 646E60: a[2] is a format string naming the entry, a[1] the bit.
        871 => {
            ensure!(a.len() == 3, "CatSystem2 database mark arguments");
            cx.machine.preflight_native(false, &[])?;
            let name = crate::native_values::format_bytes(cx.machine, a[2])?;
            if let Some(database) = cx.settings.as_mut().context("CatSystem2 database settings")?.databases.get_mut(&a[0]) {
                database.mark(&name, a[1]);
            }
            cx.machine.acknowledge(None)?;
        }
        // 646EF0: answers the byte size; with a buffer, also writes the entries.
        872 => {
            ensure!(a.len() == 2, "CatSystem2 database store arguments");
            let database = cx.settings.as_ref().context("CatSystem2 database settings")?.databases.get(&a[0]).cloned();
            let database = database.as_ref();
            let size = database.map_or(0, Database::size);
            match database {
                Some(database) if a[1] != 0 => cx.machine.commit_native(Some(size), &[(a[1], database.export())])?,
                _ => {
                    cx.machine.preflight_native(true, &[])?;
                    cx.machine.acknowledge(Some(size))?;
                }
            }
        }
        // 646F20 → 82BF20.
        873 => {
            ensure!(a.len() == 2, "CatSystem2 database load arguments");
            cx.machine.preflight_native(true, &[])?;
            let mut answer = 0;
            if a[1] != 0 && cx.settings.as_mut().context("CatSystem2 database settings")?.databases.contains_key(&a[0]) {
                let count = u32::from_le_bytes(cx.machine.read_bytes(a[1], 4)?.as_slice().try_into()?) as usize;
                ensure!(count <= 1 << 20, "CatSystem2 database saved count");
                let saved = cx.machine.read_bytes(a[1], 4 + count * ENTRY)?;
                cx.settings.as_mut().context("CatSystem2 database settings")?.databases.get_mut(&a[0]).context("CatSystem2 database handle")?.import(&saved)?;
                answer = 1;
            }
            cx.machine.acknowledge(Some(answer))?;
        }
        // 82A800 clears a dirty flag no native reads.
        878 => {
            cx.machine.preflight_native(false, &[])?;
            cx.machine.acknowledge(None)?;
        }
        id => anyhow::bail!("CatSystem2 native {id} has no handler in this family"),
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn table(names: &[&[u8]]) -> Vec<u8> {
        let mut plain = 2u32.to_le_bytes().to_vec();
        for (name, size) in [(&b"ID"[..], 4u32), (&b"INDEX"[..], 16)] {
            let mut column = [0u8; 24];
            column[..name.len()].copy_from_slice(name);
            column[20..].copy_from_slice(&size.to_le_bytes());
            plain.extend_from_slice(&column);
        }
        plain.extend_from_slice(&(names.len() as u32).to_le_bytes());
        plain.extend_from_slice(&20u32.to_le_bytes());
        for (row, name) in names.iter().enumerate() {
            plain.extend_from_slice(&(row as u32).to_le_bytes());
            let mut cell = [0u8; 16];
            cell[..name.len()].copy_from_slice(name);
            plain.extend_from_slice(&cell);
        }
        let mut zlib = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        zlib.write_all(&plain).unwrap();
        let packed = zlib.finish().unwrap();
        let mut file = b"KCDB".to_vec();
        file.extend_from_slice(&(packed.len() as u32).to_le_bytes());
        file.extend_from_slice(&(plain.len() as u32).to_le_bytes());
        file.extend_from_slice(&packed);
        file
    }

    #[test]
    fn marks_survive_export_and_fold_voiced_kana() {
        let mut database = Database::load(&table(&[b"b", &[0x82, 0xaa]])).unwrap();
        database.mark(&[0x82, 0xa9], 3);
        database.mark(b"missing", 1);
        let saved = database.export();
        assert_eq!(saved.len() as u32, database.size());
        let mut restored = Database::load(&table(&[b"b", &[0x82, 0xa9]])).unwrap();
        restored.import(&saved).unwrap();
        assert_eq!(restored.entries[&[0x82, 0xa9][..]], [8, 8, 1]);
        assert_eq!(restored.entries[&b"b"[..]], [0, 0, 0]);
    }

    #[test]
    fn a_file_without_the_magic_fails() {
        assert!(Database::load(b"not a table at all").is_err());
    }
}

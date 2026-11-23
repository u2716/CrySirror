// Copyright (C) 2023 kichikuou; Rust (C) 2026 CrySirror contributors.
// GPL-2.0-or-later. libsys4 b14839f23af0963abbdb17f7cac57a36fd8e9bd2,
// src/savefile.c and include/system4/savefile.h. No original code is executed.
//! Native GD11 global/group records; encoded strings retain original bytes.
use anyhow::{ensure, Context, Result};
use std::io::{Read, Write};
pub(super) const LIMIT: usize = 64 * 1024 * 1024;
const ITEMS: usize = 1_000_000;
#[derive(Clone)]
pub(crate) struct Value {
    pub kind: i32,
    pub bits: i32,
}
pub(crate) struct Named {
    pub name: Vec<u8>,
    pub value: Value,
    pub unknown: i32,
}
pub(crate) struct Record {
    pub kind: i32,
    pub name: Vec<u8>,
    pub indices: Vec<i32>,
}
pub(crate) struct Array {
    pub dimensions: Option<Vec<usize>>,
    pub flat: Vec<Vec<Value>>,
}
pub(crate) struct Save {
    pub key: Vec<u8>,
    pub group: Vec<u8>,
    pub version: i32,
    pub globals_total: i32,
    pub records: Vec<Record>,
    pub globals: Vec<Named>,
    // JSON names globals by original numeric index; binary GD11 uses names.
    pub original_global_indices: Option<Vec<usize>>,
    pub strings: Vec<Vec<u8>>,
    pub arrays: Vec<Array>,
    pub fields: Vec<Named>,
}
impl Save {
    pub fn new(key: Vec<u8>, group: Vec<u8>, globals_total: i32) -> Self {
        Self {
            key,
            group,
            version: 5,
            globals_total,
            records: Vec::new(),
            globals: Vec::new(),
            original_global_indices: None,
            strings: Vec::new(),
            arrays: Vec::new(),
            fields: Vec::new(),
        }
    }
    pub fn validate_value(&self, v: &Value) -> Result<()> {
        let valid = match v.kind {
            0 | 10 | 11 | 27 | 47 | 55 | 63 => true,
            12 => v.bits >= 0 && (v.bits as usize) < self.strings.len(),
            13 => v.bits >= 0 && (v.bits as usize) < self.records.len(),
            k if crate::vm::heap::is_array(k) => {
                v.bits >= 0 && (v.bits as usize) < self.arrays.len()
            }
            k if crate::vm::heap::is_reference(k) => true,
            _ => false,
        };
        ensure!(valid, "System4 global-save typed value/index");
        Ok(())
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            matches!(self.version, 4 | 5) && self.globals_total >= 0,
            "System4 global-save version/count"
        );
        let counts = [
            self.records.len(),
            self.globals.len(),
            self.strings.len(),
            self.arrays.len(),
            self.fields.len(),
        ];
        ensure!(
            counts.iter().sum::<usize>() <= ITEMS,
            "System4 global-save record budget"
        );
        for rec in &self.records {
            let upper = match rec.kind {
                1000 => self.globals.len(),
                13 => self.fields.len(),
                _ => anyhow::bail!("System4 global-save record kind"),
            };
            ensure!(
                rec.indices.iter().all(|i| *i >= 0 && (*i as usize) < upper),
                "System4 global-save record index"
            );
        }
        for n in self.globals.iter().chain(&self.fields) {
            self.validate_value(&n.value)?;
        }
        let mut values = 0usize;
        for a in &self.arrays {
            if let Some(d) = &a.dimensions {
                ensure!(
                    !d.is_empty() && d.len() <= 32,
                    "System4 global-save array rank"
                );
                let expected = d
                    .iter()
                    .skip(1)
                    .try_fold(1usize, |a, b| a.checked_mul(*b).filter(|n| *n <= ITEMS))
                    .context("System4 global-save array extent")?;
                ensure!(
                    a.flat.len() == expected,
                    "System4 global-save flat-array count"
                );
                for row in &a.flat {
                    ensure!(
                        row.len() == d[0],
                        "System4 global-save flat-array dimension"
                    );
                    values = values
                        .checked_add(row.len())
                        .filter(|n| *n <= ITEMS)
                        .context("System4 global-save total values")?;
                    for v in row {
                        self.validate_value(v)?;
                    }
                }
            } else {
                ensure!(a.flat.is_empty(), "System4 unallocated global-save array");
            }
        }
        Ok(())
    }
    pub fn decode(bytes: &[u8], check: &dyn Fn() -> Result<()>) -> Result<Self> {
        let raw = open_envelope(bytes, check)?;
        let mut r = Reader {
            bytes: &raw,
            at: 0,
            items: 0,
        };
        let key = r.string()?;
        let _uk1 = r.int()?;
        let version = r.int()?;
        ensure!(
            matches!(version, 4 | 5),
            "System4 global-save supports native v4/v5"
        );
        let _uk2 = r.int()?;
        let globals_total = r.int()?;
        let mut sections = [(0usize, 0usize); 5];
        for section in &mut sections {
            section.0 = r.count()?;
            section.1 = r.count()?;
        }
        let total = sections
            .iter()
            .try_fold(0usize, |a, s| a.checked_add(s.1).filter(|n| *n <= ITEMS))
            .context("System4 global-save section count budget")?;
        r.claim(total)?;
        let group = if version >= 5 {
            r.string()?
        } else {
            Vec::new()
        };
        let mut out = Self::new(key, group, globals_total);
        out.version = version;
        r.offset(sections[0].0)?;
        for _ in 0..sections[0].1 {
            check()?;
            let kind = r.int()?;
            let name = r.string()?;
            let count = r.count()?;
            r.claim(count)?;
            let mut indices = Vec::with_capacity(count);
            for _ in 0..count {
                indices.push(r.int()?);
            }
            out.records.push(Record {
                kind,
                name,
                indices,
            });
        }
        r.offset(sections[1].0)?;
        for _ in 0..sections[1].1 {
            check()?;
            let kind = r.int()?;
            let bits = r.int()?;
            let name = r.string()?;
            let unknown = r.int()?;
            out.globals.push(Named {
                name,
                value: Value { kind, bits },
                unknown,
            });
        }
        r.offset(sections[2].0)?;
        for _ in 0..sections[2].1 {
            check()?;
            out.strings.push(r.string()?);
        }
        r.offset(sections[3].0)?;
        for _ in 0..sections[3].1 {
            check()?;
            let rank = r.int()?;
            ensure!(
                rank == -1 || (1..=32).contains(&rank),
                "System4 global-save array rank"
            );
            let dimensions = if rank == -1 {
                None
            } else {
                let mut d = Vec::with_capacity(rank as usize);
                for _ in 0..rank {
                    d.push(r.count()?);
                }
                Some(d)
            };
            let count = r.count()?;
            r.claim(count)?;
            let mut flat = Vec::with_capacity(count);
            for _ in 0..count {
                let count = r.count()?;
                r.claim(count)?;
                let mut row = Vec::with_capacity(count);
                for _ in 0..count {
                    row.push(Value {
                        bits: r.int()?,
                        kind: r.int()?,
                    });
                }
                flat.push(row);
            }
            out.arrays.push(Array { dimensions, flat });
        }
        r.offset(sections[4].0)?;
        for _ in 0..sections[4].1 {
            check()?;
            let kind = r.int()?;
            let bits = r.int()?;
            let name = r.string()?;
            out.fields.push(Named {
                name,
                value: Value { kind, bits },
                unknown: 1,
            });
        }
        ensure!(r.at == raw.len(), "System4 global-save trailing bytes");
        out.validate()?;
        Ok(out)
    }
    pub fn encode(&self, check: &dyn Fn() -> Result<()>) -> Result<Vec<u8>> {
        self.validate()?;
        let mut w = Writer(Vec::new());
        w.string(&self.key)?;
        for n in [1000, self.version, 56, self.globals_total] {
            w.int(n)?;
        }
        let mut patches = Vec::new();
        for count in [
            self.records.len(),
            self.globals.len(),
            self.strings.len(),
            self.arrays.len(),
            self.fields.len(),
        ] {
            patches.push(w.0.len());
            w.int(0)?;
            w.int(i32::try_from(count)?)?;
        }
        if self.version >= 5 {
            w.string(&self.group)?;
        }
        w.patch(patches[0])?;
        for rec in &self.records {
            check()?;
            w.int(rec.kind)?;
            w.string(&rec.name)?;
            w.int(i32::try_from(rec.indices.len())?)?;
            for i in &rec.indices {
                w.int(*i)?;
            }
        }
        w.patch(patches[1])?;
        for n in &self.globals {
            check()?;
            w.int(n.value.kind)?;
            w.int(n.value.bits)?;
            w.string(&n.name)?;
            w.int(n.unknown)?;
        }
        w.patch(patches[2])?;
        for s in &self.strings {
            check()?;
            w.string(s)?;
        }
        w.patch(patches[3])?;
        for a in &self.arrays {
            check()?;
            w.int(a.dimensions.as_ref().map_or(-1, |d| d.len() as i32))?;
            if let Some(d) = &a.dimensions {
                for n in d {
                    w.int(i32::try_from(*n)?)?;
                }
            }
            w.int(i32::try_from(a.flat.len())?)?;
            for row in &a.flat {
                w.int(i32::try_from(row.len())?)?;
                for v in row {
                    w.int(v.bits)?;
                    w.int(v.kind)?;
                }
            }
        }
        w.patch(patches[4])?;
        for n in &self.fields {
            check()?;
            w.int(n.value.kind)?;
            w.int(n.value.bits)?;
            w.string(&n.name)?;
        }
        seal_envelope(&w.0, 9, check)
    }
}
pub(super) fn open_envelope(bytes: &[u8], check: &dyn Fn() -> Result<()>) -> Result<Vec<u8>> {
    ensure!(
        bytes.len() >= 10 && bytes.len() <= LIMIT,
        "System4 global-save envelope size"
    );
    ensure!(
        &bytes[..4] == b"GD\x01\x01",
        "System4 native global-save signature"
    );
    let size = u32::from_le_bytes(bytes[4..8].try_into()?) as usize;
    ensure!(size <= LIMIT, "System4 global-save raw budget");
    check()?;
    let mut compressed = bytes[8..].to_vec();
    if compressed[0] == 0x1a {
        let mut mt = crysirror_resource::legacy_stream::Mt19937_1998::new(0x12320f);
        for block in compressed.chunks_mut(65536) {
            check()?;
            mt.xor_low_bytes(block);
        }
    }
    let mut decoder = flate2::read::ZlibDecoder::new(compressed.as_slice());
    let mut raw = Vec::with_capacity(size.min(65536));
    let mut block = [0u8; 65536];
    loop {
        check()?;
        let n = decoder.read(&mut block)?;
        if n == 0 {
            break;
        }
        ensure!(
            raw.len() + n <= size,
            "System4 global-save decompression size"
        );
        raw.extend_from_slice(&block[..n]);
    }
    ensure!(
        raw.len() == size && decoder.total_in() == compressed.len() as u64,
        "System4 global-save exact compressed stream"
    );
    Ok(raw)
}
pub(super) fn seal_envelope(
    raw: &[u8],
    level: u32,
    check: &dyn Fn() -> Result<()>,
) -> Result<Vec<u8>> {
    ensure!(raw.len() <= LIMIT, "System4 save envelope raw budget");
    let raw_len = u32::try_from(raw.len())?;
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(level));
    for block in raw.chunks(65536) {
        check()?;
        encoder.write_all(block)?;
    }
    let mut compressed = encoder.finish()?;
    ensure!(
        compressed.len() + 8 <= LIMIT,
        "System4 global-save encoded budget"
    );
    let mut mt = crysirror_resource::legacy_stream::Mt19937_1998::new(0x12320f);
    for block in compressed.chunks_mut(65536) {
        check()?;
        mt.xor_low_bytes(block);
    }
    let mut out = b"GD\x01\x01".to_vec();
    out.extend(raw_len.to_le_bytes());
    out.extend(compressed);
    Ok(out)
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
    items: usize,
}
impl Reader<'_> {
    fn int(&mut self) -> Result<i32> {
        let end = self
            .at
            .checked_add(4)
            .context("System4 save offset overflow")?;
        let v = i32::from_le_bytes(
            self.bytes
                .get(self.at..end)
                .context("System4 truncated global-save word")?
                .try_into()?,
        );
        self.at = end;
        Ok(v)
    }
    fn count(&mut self) -> Result<usize> {
        let n = usize::try_from(self.int()?)?;
        ensure!(n <= LIMIT, "System4 global-save count bound");
        Ok(n)
    }
    fn claim(&mut self, n: usize) -> Result<()> {
        self.items = self
            .items
            .checked_add(n)
            .filter(|n| *n <= ITEMS)
            .context("System4 global-save item budget")?;
        Ok(())
    }
    fn offset(&self, n: usize) -> Result<()> {
        ensure!(self.at == n, "System4 global-save section offset");
        Ok(())
    }
    fn string(&mut self) -> Result<Vec<u8>> {
        let rest = self
            .bytes
            .get(self.at..)
            .context("System4 global-save string offset")?;
        let n = rest
            .iter()
            .take(1024 * 1024 + 1)
            .position(|b| *b == 0)
            .context("System4 global-save string terminator/budget")?;
        let out = rest[..n].to_vec();
        self.at += n + 1;
        Ok(out)
    }
}
struct Writer(Vec<u8>);
impl Writer {
    fn int(&mut self, n: i32) -> Result<()> {
        ensure!(
            self.0.len() + 4 <= LIMIT,
            "System4 global-save writer budget"
        );
        self.0.extend(n.to_le_bytes());
        Ok(())
    }
    fn string(&mut self, b: &[u8]) -> Result<()> {
        let b = b.split(|b| *b == 0).next().unwrap();
        ensure!(
            b.len() <= 1024 * 1024 && self.0.len() + b.len() + 1 <= LIMIT,
            "System4 global-save string budget"
        );
        self.0.extend_from_slice(b);
        self.0.push(0);
        Ok(())
    }
    fn patch(&mut self, at: usize) -> Result<()> {
        let value = i32::try_from(self.0.len())?;
        self.0
            .get_mut(at..at + 4)
            .context("System4 global-save patch offset")?
            .copy_from_slice(&value.to_le_bytes());
        Ok(())
    }
}

// Copyright (C)2023 kichikuou; Rust (C)2026 CrySirror contributors.
// GPL-2.0-or-later. libsys4 b14839f savefile.c/savefile.h RSM records.
//! Original resume payload, not a framework snapshot or serialized host object.
use anyhow::{ensure, Context, Result};
const BYTES: usize = 64 * 1024 * 1024;
#[derive(Clone)]
pub(crate) enum Symbol {
    Id(i32),
    Name(Vec<u8>),
}
#[derive(Clone)]
pub(crate) struct ReturnRecord {
    pub address: i32,
    pub function: Option<Vec<u8>>,
    pub local: i32,
    pub crc: u32,
}
#[derive(Clone)]
pub(crate) struct CallFrame {
    pub kind: i32,
    pub local: i32,
    pub structure: i32,
}
#[derive(Clone)]
pub(crate) enum HeapValue {
    Null,
    Frame {
        references: u32,
        sequence: u32,
        global: bool,
        function: Symbol,
        types: Vec<i32>,
        structure: i32,
        values: Vec<i32>,
    },
    String {
        references: u32,
        sequence: u32,
        unknown: i32,
        bytes: Vec<u8>,
    },
    Array {
        references: u32,
        sequence: u32,
        rank_minus_one: i32,
        kind: i32,
        structure: Symbol,
        root_rank: i32,
        allocated: bool,
        values: Vec<i32>,
    },
    Structure {
        references: u32,
        sequence: u32,
        constructor: Symbol,
        destructor: Symbol,
        structure: Symbol,
        types: Vec<i32>,
        values: Vec<i32>,
    },
    Delegate {
        references: u32,
        sequence: u32,
        values: Vec<i32>,
    },
}
#[derive(Clone)]
pub(crate) struct Body {
    pub ip: ReturnRecord,
    pub stack: Vec<i32>,
    pub frames: Vec<CallFrame>,
    pub returns: Vec<ReturnRecord>,
    pub next_sequence: u32,
    pub heap: Vec<HeapValue>,
    pub functions: Vec<Vec<u8>>,
}
#[derive(Clone)]
pub(crate) struct Resume {
    pub version: i32,
    pub key: Vec<u8>,
    pub comments: Vec<Vec<u8>>,
    pub body: Option<Body>,
}
impl Resume {
    pub(super) fn decode_payload(raw: &[u8], check: &dyn Fn() -> Result<()>) -> Result<Self> {
        ensure!(raw.len() <= BYTES, "System4 resume payload byte limit");
        let mut r = Read {
            raw,
            at: 0,
            values: 0,
        };
        ensure!(r.bytes(4)? == b"RSM\0", "System4 resume signature");
        let version = r.int()?;
        ensure!(matches!(version, 4 | 6 | 7 | 9), "System4 resume version");
        let key = r.string()?;
        let comments = if version >= 7 {
            r.strings()?
        } else {
            Vec::new()
        };
        let mut out = Self {
            version,
            key,
            comments,
            body: None,
        };
        if version >= 7 && r.at == raw.len() {
            return Ok(out);
        }
        let ip = r.return_record()?;
        ensure!(r.int()? == 0, "System4 resume header reserved word");
        let stack = r.ints(65536)?;
        let locals = r.ints(4097)?;
        let kinds = r.ints(4097)?;
        let structures = r.ints(4097)?;
        ensure!(
            locals.len() == kinds.len(),
            "System4 resume parallel frame count"
        );
        let mut next_structure = 0;
        let mut frames = Vec::with_capacity(locals.len());
        for (kind, local) in kinds.into_iter().zip(locals) {
            check()?;
            ensure!(matches!(kind, 0 | 1 | 2 | 4), "System4 resume frame kind");
            let structure = if kind == 2 {
                let slot = *structures
                    .get(next_structure)
                    .context("System4 resume method frame structure")?;
                next_structure += 1;
                slot
            } else {
                -1
            };
            frames.push(CallFrame {
                kind,
                local,
                structure,
            });
        }
        ensure!(
            next_structure == structures.len(),
            "System4 resume excess method frames"
        );
        let count = r.count(4097)?;
        let mut returns = Vec::with_capacity(count);
        for _ in 0..count {
            check()?;
            returns.push(r.return_record()?);
        }
        for _ in 0..3 {
            ensure!(r.int()? == 0, "System4 resume reserved word");
        }
        let next_sequence = if version >= 9 { r.int()? as u32 } else { 0 };
        let count = r.count(1_000_000)?;
        let mut heap = Vec::with_capacity(count);
        for _ in 0..count {
            check()?;
            let tag = r.int()?;
            if tag == -1 {
                heap.push(HeapValue::Null);
                continue;
            }
            let references = u32::try_from(r.int()?)?;
            ensure!(references > 0, "System4 resume live heap refcount");
            let sequence = if version >= 9 { r.int()? as u32 } else { 0 };
            let value = match tag {
                0 | 1 => {
                    let function = if version == 4 || tag == 0 {
                        let id = r.int()?;
                        if tag == 0 && version != 4 {
                            ensure!(id == -1, "System4 resume global function marker");
                        }
                        Symbol::Id(id)
                    } else {
                        Symbol::Name(r.string()?)
                    };
                    let types = r.ints(8_000_000)?;
                    let structure = if tag == 1 && version >= 9 {
                        r.int()?
                    } else {
                        -1
                    };
                    let values = r.slot_bytes()?;
                    ensure!(
                        types.len() == values.len(),
                        "System4 resume typed frame extent"
                    );
                    HeapValue::Frame {
                        references,
                        sequence,
                        global: tag == 0,
                        function,
                        types,
                        structure,
                        values,
                    }
                }
                2 => {
                    let unknown = r.int()?;
                    ensure!(matches!(unknown, 0 | 1), "System4 resume string flags");
                    let n = r.count(1024 * 1024)?;
                    ensure!(n > 0, "System4 resume empty string storage");
                    let bytes = r.bytes(n)?.to_vec();
                    ensure!(bytes.last() == Some(&0), "System4 resume string terminator");
                    HeapValue::String {
                        references,
                        sequence,
                        unknown,
                        bytes,
                    }
                }
                3 => {
                    let rank_minus_one = r.int()?;
                    ensure!(
                        (-1..32).contains(&rank_minus_one),
                        "System4 resume array rank"
                    );
                    let kind = r.int()?;
                    let structure = r.symbol(version)?;
                    let root_rank = r.int()?;
                    let flag = r.int()?;
                    ensure!(
                        matches!(flag, 0 | 1),
                        "System4 resume array allocation flag"
                    );
                    let values = r.slot_bytes()?;
                    if rank_minus_one < 0 {
                        ensure!(values.is_empty(), "System4 resume null array values");
                    }
                    HeapValue::Array {
                        references,
                        sequence,
                        rank_minus_one,
                        kind,
                        structure,
                        root_rank,
                        allocated: flag == 1,
                        values,
                    }
                }
                4 => {
                    let constructor = r.symbol(version)?;
                    let destructor = r.symbol(version)?;
                    ensure!(r.int()? == 0, "System4 resume structure reserved word");
                    let structure = r.symbol(version)?;
                    let types = r.ints(8_000_000)?;
                    let values = r.slot_bytes()?;
                    ensure!(
                        types.len() == values.len(),
                        "System4 resume structure typed extent"
                    );
                    HeapValue::Structure {
                        references,
                        sequence,
                        constructor,
                        destructor,
                        structure,
                        types,
                        values,
                    }
                }
                5 => {
                    ensure!(version >= 9, "System4 resume delegate version");
                    let values = r.slot_bytes()?;
                    ensure!(
                        values.len() % 3 == 0,
                        "System4 resume delegate triple extent"
                    );
                    HeapValue::Delegate {
                        references,
                        sequence,
                        values,
                    }
                }
                _ => anyhow::bail!("unknown System4 native resume heap tag {tag}"),
            };
            heap.push(value);
        }
        let functions = if version >= 6 {
            r.strings()?
        } else {
            Vec::new()
        };
        ensure!(r.at == raw.len(), "System4 resume trailing bytes");
        out.body = Some(Body {
            ip,
            stack,
            frames,
            returns,
            next_sequence,
            heap,
            functions,
        });
        Ok(out)
    }
    pub(super) fn encode_payload(&self, check: &dyn Fn() -> Result<()>) -> Result<Vec<u8>> {
        ensure!(
            matches!(self.version, 4 | 6 | 7 | 9),
            "System4 resume version"
        );
        let mut w = Write(Vec::new());
        w.bytes(b"RSM\0")?;
        w.int(self.version)?;
        w.string(&self.key)?;
        if self.version >= 7 {
            w.strings(&self.comments)?;
        } else {
            ensure!(self.comments.is_empty(), "System4 resume comments need v7+");
        }
        if let Some(body) = &self.body {
            ensure!(
                body.stack.len() <= 65536
                    && body.frames.len() <= 4097
                    && body.returns.len() <= 4097
                    && body.heap.len() <= 1_000_000,
                "System4 resume state count limit"
            );
            w.return_record(&body.ip)?;
            w.int(0)?;
            w.ints(&body.stack)?;
            w.ints(&body.frames.iter().map(|f| f.local).collect::<Vec<_>>())?;
            w.ints(&body.frames.iter().map(|f| f.kind).collect::<Vec<_>>())?;
            w.ints(
                &body
                    .frames
                    .iter()
                    .filter(|f| f.kind == 2)
                    .map(|f| f.structure)
                    .collect::<Vec<_>>(),
            )?;
            w.int(i32::try_from(body.returns.len())?)?;
            for rr in &body.returns {
                check()?;
                w.return_record(rr)?;
            }
            for _ in 0..3 {
                w.int(0)?;
            }
            if self.version >= 9 {
                w.int(body.next_sequence as i32)?;
            }
            w.int(i32::try_from(body.heap.len())?)?;
            for value in &body.heap {
                check()?;
                let (tag, refs, seq) = match value {
                    HeapValue::Null => {
                        w.int(-1)?;
                        continue;
                    }
                    HeapValue::Frame {
                        references,
                        sequence,
                        global,
                        ..
                    } => (if *global { 0 } else { 1 }, *references, *sequence),
                    HeapValue::String {
                        references,
                        sequence,
                        ..
                    } => (2, *references, *sequence),
                    HeapValue::Array {
                        references,
                        sequence,
                        ..
                    } => (3, *references, *sequence),
                    HeapValue::Structure {
                        references,
                        sequence,
                        ..
                    } => (4, *references, *sequence),
                    HeapValue::Delegate {
                        references,
                        sequence,
                        ..
                    } => (5, *references, *sequence),
                };
                ensure!(
                    refs > 0 && refs <= i32::MAX as u32,
                    "System4 resume heap refcount limit"
                );
                w.int(tag)?;
                w.int(refs as i32)?;
                if self.version >= 9 {
                    w.int(seq as i32)?;
                }
                match value {
                    HeapValue::Frame {
                        function,
                        types,
                        structure,
                        values,
                        global,
                        ..
                    } => {
                        if *global && self.version != 4 {
                            w.int(-1)?;
                        } else {
                            w.symbol(function, self.version)?;
                        }
                        ensure!(types.len() == values.len(), "System4 resume frame types");
                        w.ints(types)?;
                        if !*global && self.version >= 9 {
                            w.int(*structure)?;
                        }
                        w.slot_bytes(values)?;
                    }
                    HeapValue::String { unknown, bytes, .. } => {
                        ensure!(
                            bytes.last() == Some(&0) && bytes.len() <= 1024 * 1024,
                            "System4 resume string storage"
                        );
                        w.int(*unknown)?;
                        w.int(i32::try_from(bytes.len())?)?;
                        w.bytes(bytes)?;
                    }
                    HeapValue::Array {
                        rank_minus_one,
                        kind,
                        structure,
                        root_rank,
                        allocated,
                        values,
                        ..
                    } => {
                        w.int(*rank_minus_one)?;
                        w.int(*kind)?;
                        w.symbol(structure, self.version)?;
                        w.int(*root_rank)?;
                        w.int(i32::from(*allocated))?;
                        w.slot_bytes(values)?;
                    }
                    HeapValue::Structure {
                        constructor,
                        destructor,
                        structure,
                        types,
                        values,
                        ..
                    } => {
                        w.symbol(constructor, self.version)?;
                        w.symbol(destructor, self.version)?;
                        w.int(0)?;
                        w.symbol(structure, self.version)?;
                        ensure!(
                            types.len() == values.len(),
                            "System4 resume structure types"
                        );
                        w.ints(types)?;
                        w.slot_bytes(values)?;
                    }
                    HeapValue::Delegate { values, .. } => {
                        ensure!(
                            self.version >= 9 && values.len() % 3 == 0,
                            "System4 resume delegate profile"
                        );
                        w.slot_bytes(values)?;
                    }
                    HeapValue::Null => unreachable!(),
                }
            }
            if self.version >= 6 {
                w.strings(&body.functions)?;
            }
        } else {
            ensure!(self.version >= 7, "System4 comments-only resume needs v7+");
        }
        Ok(w.0)
    }
}
struct Read<'a> {
    raw: &'a [u8],
    at: usize,
    values: usize,
}
impl<'a> Read<'a> {
    fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .at
            .checked_add(n)
            .context("System4 resume offset overflow")?;
        let b = self
            .raw
            .get(self.at..end)
            .context("System4 truncated resume bytes")?;
        self.at = end;
        Ok(b)
    }
    fn int(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.bytes(4)?.try_into()?))
    }
    fn count(&mut self, max: usize) -> Result<usize> {
        let n = usize::try_from(self.int()?)?;
        ensure!(n <= max, "System4 resume count limit");
        Ok(n)
    }
    fn string(&mut self) -> Result<Vec<u8>> {
        let n = self
            .raw
            .get(self.at..)
            .context("System4 resume string offset")?
            .iter()
            .take(1024 * 1024 + 1)
            .position(|b| *b == 0)
            .context("System4 resume string terminator/limit")?;
        let bytes = self.bytes(n + 1)?;
        Ok(bytes[..n].to_vec())
    }
    fn ints(&mut self, max: usize) -> Result<Vec<i32>> {
        let n = self.count(max)?;
        self.values = self
            .values
            .checked_add(n)
            .filter(|n| *n <= 8_000_000)
            .context("System4 resume total value limit")?;
        ensure!(
            n <= self.raw.len().saturating_sub(self.at) / 4,
            "System4 truncated resume vector"
        );
        (0..n).map(|_| self.int()).collect()
    }
    fn strings(&mut self) -> Result<Vec<Vec<u8>>> {
        let n = self.count(1_000_000)?;
        ensure!(
            n <= self.raw.len().saturating_sub(self.at),
            "System4 truncated resume string vector"
        );
        (0..n).map(|_| self.string()).collect()
    }
    fn slot_bytes(&mut self) -> Result<Vec<i32>> {
        let bytes = self.count(32_000_000)?;
        ensure!(bytes % 4 == 0, "System4 resume slot byte alignment");
        let n = bytes / 4;
        self.values = self
            .values
            .checked_add(n)
            .filter(|n| *n <= 8_000_000)
            .context("System4 resume total slot limit")?;
        ensure!(
            bytes <= self.raw.len().saturating_sub(self.at),
            "System4 truncated resume slots"
        );
        (0..n).map(|_| self.int()).collect()
    }
    fn symbol(&mut self, version: i32) -> Result<Symbol> {
        Ok(if version == 4 {
            Symbol::Id(self.int()?)
        } else {
            Symbol::Name(self.string()?)
        })
    }
    fn return_record(&mut self) -> Result<ReturnRecord> {
        let address = self.int()?;
        Ok(if address == -1 {
            ReturnRecord {
                address,
                function: None,
                local: 0,
                crc: 0,
            }
        } else {
            ReturnRecord {
                address,
                function: Some(self.string()?),
                local: self.int()?,
                crc: self.int()? as u32,
            }
        })
    }
}
struct Write(Vec<u8>);
impl Write {
    fn bytes(&mut self, b: &[u8]) -> Result<()> {
        ensure!(
            self.0
                .len()
                .checked_add(b.len())
                .is_some_and(|n| n <= BYTES),
            "System4 resume output byte limit"
        );
        self.0.extend_from_slice(b);
        Ok(())
    }
    fn int(&mut self, n: i32) -> Result<()> {
        self.bytes(&n.to_le_bytes())
    }
    fn string(&mut self, b: &[u8]) -> Result<()> {
        ensure!(
            b.len() <= 1024 * 1024 && !b.contains(&0),
            "System4 resume C string profile"
        );
        self.bytes(b)?;
        self.bytes(&[0])
    }
    fn ints(&mut self, v: &[i32]) -> Result<()> {
        self.int(i32::try_from(v.len())?)?;
        for n in v {
            self.int(*n)?;
        }
        Ok(())
    }
    fn strings(&mut self, v: &[Vec<u8>]) -> Result<()> {
        ensure!(v.len() <= 1_000_000, "System4 resume string vector count");
        self.int(i32::try_from(v.len())?)?;
        for b in v {
            self.string(b)?;
        }
        Ok(())
    }
    fn slot_bytes(&mut self, v: &[i32]) -> Result<()> {
        self.int(i32::try_from(
            v.len()
                .checked_mul(4)
                .context("System4 resume slot byte overflow")?,
        )?)?;
        for n in v {
            self.int(*n)?;
        }
        Ok(())
    }
    fn symbol(&mut self, s: &Symbol, version: i32) -> Result<()> {
        match (version, s) {
            (4, Symbol::Id(i)) => self.int(*i),
            (6 | 7 | 9, Symbol::Name(n)) => self.string(n),
            _ => anyhow::bail!("System4 resume symbol/version mismatch"),
        }
    }
    fn return_record(&mut self, r: &ReturnRecord) -> Result<()> {
        self.int(r.address)?;
        if r.address != -1 {
            self.string(
                r.function
                    .as_ref()
                    .context("System4 resume return function")?,
            )?;
            self.int(r.local)?;
            self.int(r.crc as i32)?;
        }
        Ok(())
    }
}

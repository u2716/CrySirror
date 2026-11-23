// SPDX-License-Identifier: MPL-2.0
//! Bounded reader for ECMA-335 (CLI) assemblies installed with a game:
//! PE/CLI headers, the metadata streams and tables (ECMA-335 II.22–24), and
//! method bodies. [`eval`] evaluates constant static initializers so data
//! tables compiled into managed code can be read from the user's own
//! installation instead of being copied into CrySirror.
//!
//! Only reading and constant evaluation are offered; no managed code runs.
use anyhow::{bail, ensure, Context, Result};

pub mod eval;

/// Upper bound on assembly size.
pub const MAX_ASSEMBLY_BYTES: usize = 64 * 1024 * 1024;

/// Metadata table numbers (ECMA-335 II.22).
pub mod table {
    pub const MODULE: usize = 0x00;
    pub const TYPE_REF: usize = 0x01;
    pub const TYPE_DEF: usize = 0x02;
    pub const FIELD: usize = 0x04;
    pub const METHOD_DEF: usize = 0x06;
    pub const PARAM: usize = 0x08;
    pub const MEMBER_REF: usize = 0x0A;
    pub const CONSTANT: usize = 0x0B;
    pub const FIELD_RVA: usize = 0x1D;
    pub const TYPE_SPEC: usize = 0x1B;
    pub const NESTED_CLASS: usize = 0x29;
    pub const METHOD_SPEC: usize = 0x2B;
}

/// Column kinds of a metadata table row.
#[derive(Clone, Copy)]
enum Col {
    U8,
    U16,
    U32,
    Str,
    Guid,
    Blob,
    /// Simple index into a table.
    Idx(usize),
    /// Coded index over these tables (`usize::MAX` = unused tag).
    Coded(&'static [usize]),
}

const NONE: usize = usize::MAX;
const TYPE_DEF_OR_REF: &[usize] = &[0x02, 0x01, 0x1B];
const HAS_CONSTANT: &[usize] = &[0x04, 0x08, 0x17];
const HAS_CUSTOM_ATTRIBUTE: &[usize] =
    &[0x06, 0x04, 0x01, 0x02, 0x08, 0x09, 0x0A, 0x00, 0x0E, 0x17, 0x14, 0x11, 0x1A, 0x1B, 0x20, 0x23, 0x26, 0x27, 0x28, 0x2A, 0x2C, 0x2B];
const HAS_FIELD_MARSHAL: &[usize] = &[0x04, 0x08];
const HAS_DECL_SECURITY: &[usize] = &[0x02, 0x06, 0x20];
const MEMBER_REF_PARENT: &[usize] = &[0x02, 0x01, 0x1A, 0x06, 0x1B];
const HAS_SEMANTICS: &[usize] = &[0x14, 0x17];
const METHOD_DEF_OR_REF: &[usize] = &[0x06, 0x0A];
const MEMBER_FORWARDED: &[usize] = &[0x04, 0x06];
const IMPLEMENTATION: &[usize] = &[0x26, 0x23, 0x27];
const CUSTOM_ATTRIBUTE_TYPE: &[usize] = &[NONE, NONE, 0x06, 0x0A, NONE];
const RESOLUTION_SCOPE: &[usize] = &[0x00, 0x1A, 0x23, 0x01];
const TYPE_OR_METHOD_DEF: &[usize] = &[0x02, 0x06];

/// Row layouts of tables 0x00..=0x2C.
fn schema(t: usize) -> &'static [Col] {
    use Col::*;
    match t {
        0x00 => &[U16, Str, Guid, Guid, Guid],
        0x01 => &[Coded(RESOLUTION_SCOPE), Str, Str],
        0x02 => &[U32, Str, Str, Coded(TYPE_DEF_OR_REF), Idx(0x04), Idx(0x06)],
        0x03 => &[Idx(0x04)],
        0x04 => &[U16, Str, Blob],
        0x05 => &[Idx(0x06)],
        0x06 => &[U32, U16, U16, Str, Blob, Idx(0x08)],
        0x07 => &[Idx(0x08)],
        0x08 => &[U16, U16, Str],
        0x09 => &[Idx(0x02), Coded(TYPE_DEF_OR_REF)],
        0x0A => &[Coded(MEMBER_REF_PARENT), Str, Blob],
        0x0B => &[U8, U8, Coded(HAS_CONSTANT), Blob],
        0x0C => &[Coded(HAS_CUSTOM_ATTRIBUTE), Coded(CUSTOM_ATTRIBUTE_TYPE), Blob],
        0x0D => &[Coded(HAS_FIELD_MARSHAL), Blob],
        0x0E => &[U16, Coded(HAS_DECL_SECURITY), Blob],
        0x0F => &[U16, U32, Idx(0x02)],
        0x10 => &[U32, Idx(0x04)],
        0x11 => &[Blob],
        0x12 => &[Idx(0x02), Idx(0x14)],
        0x13 => &[Idx(0x14)],
        0x14 => &[U16, Str, Coded(TYPE_DEF_OR_REF)],
        0x15 => &[Idx(0x02), Idx(0x17)],
        0x16 => &[Idx(0x17)],
        0x17 => &[U16, Str, Blob],
        0x18 => &[U16, Idx(0x06), Coded(HAS_SEMANTICS)],
        0x19 => &[Idx(0x02), Coded(METHOD_DEF_OR_REF), Coded(METHOD_DEF_OR_REF)],
        0x1A => &[Str],
        0x1B => &[Blob],
        0x1C => &[U16, Coded(MEMBER_FORWARDED), Str, Idx(0x1A)],
        0x1D => &[U32, Idx(0x04)],
        0x1E => &[U32, U32],
        0x1F => &[U32],
        0x20 => &[U32, U16, U16, U16, U16, U32, Blob, Str, Str],
        0x21 => &[U32],
        0x22 => &[U32, U32, U32],
        0x23 => &[U16, U16, U16, U16, U32, Blob, Str, Str, Blob],
        0x24 => &[U32, Idx(0x23)],
        0x25 => &[U32, U32, U32, Idx(0x23)],
        0x26 => &[U32, Str, Blob],
        0x27 => &[U32, U32, Str, Str, Coded(IMPLEMENTATION)],
        0x28 => &[U32, U32, Str, Coded(IMPLEMENTATION)],
        0x29 => &[Idx(0x02), Idx(0x02)],
        0x2A => &[U16, U16, Coded(TYPE_OR_METHOD_DEF), Str],
        0x2B => &[Coded(METHOD_DEF_OR_REF), Blob],
        0x2C => &[Idx(0x2A), Coded(TYPE_DEF_OR_REF)],
        _ => &[],
    }
}

/// A decoded coded index: target table and 1-based row (0 = null).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Token {
    pub table: usize,
    pub row: u32,
}

impl Token {
    /// From a metadata token (`table << 24 | row`).
    pub fn from_u32(t: u32) -> Self {
        Self { table: (t >> 24) as usize, row: t & 0x00ff_ffff }
    }
}

#[derive(Clone, Copy, Debug)]
struct Section {
    va: u32,
    size: u32,
    raw: u32,
}

/// One loaded assembly.
pub struct Assembly {
    bytes: Vec<u8>,
    sections: Vec<Section>,
    strings: (usize, usize),
    user_strings: (usize, usize),
    blobs: (usize, usize),
    rows: [u32; 64],
    /// Byte offset of each table and its row size.
    tables: [(usize, usize); 64],
    wide_str: bool,
    wide_guid: bool,
    wide_blob: bool,
}

fn rd16(b: &[u8], o: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(b.get(o..o + 2).context("truncated assembly")?.try_into()?))
}
fn rd32(b: &[u8], o: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(b.get(o..o + 4).context("truncated assembly")?.try_into()?))
}

impl Assembly {
    pub fn parse(bytes: Vec<u8>) -> Result<Self> {
        ensure!(bytes.len() <= MAX_ASSEMBLY_BYTES, "assembly exceeds 64 MiB");
        ensure!(bytes.get(..2) == Some(b"MZ"), "not a PE image");
        let pe = rd32(&bytes, 0x3c)? as usize;
        ensure!(bytes.get(pe..pe + 4) == Some(b"PE\0\0"), "PE signature missing");
        let coff = pe + 4;
        let nsec = rd16(&bytes, coff + 2)? as usize;
        let opt_size = rd16(&bytes, coff + 16)? as usize;
        let opt = coff + 20;
        let magic = rd16(&bytes, opt)?;
        let dirs = match magic {
            0x10b => opt + 96,
            0x20b => opt + 112,
            m => bail!("unknown PE optional header {m:#x}"),
        };
        let mut sections = Vec::with_capacity(nsec);
        let sec = opt + opt_size;
        for i in 0..nsec {
            let s = sec + i * 40;
            sections.push(Section { size: rd32(&bytes, s + 8)?.max(rd32(&bytes, s + 16)?), va: rd32(&bytes, s + 12)?, raw: rd32(&bytes, s + 20)? });
        }
        let mut a = Assembly {
            bytes,
            sections,
            strings: (0, 0),
            user_strings: (0, 0),
            blobs: (0, 0),
            rows: [0; 64],
            tables: [(0, 0); 64],
            wide_str: false,
            wide_guid: false,
            wide_blob: false,
        };
        let cli_rva = rd32(&a.bytes, dirs + 14 * 8)?;
        ensure!(cli_rva != 0, "no CLI header");
        let cli = a.offset(cli_rva)?;
        let md = a.offset(rd32(&a.bytes, cli + 8)?)?;
        ensure!(rd32(&a.bytes, md)? == 0x424a_5342, "metadata signature missing");
        let vlen = rd32(&a.bytes, md + 12)? as usize;
        let mut o = md + 16 + vlen + 2;
        let nstreams = rd16(&a.bytes, o)? as usize;
        o += 2;
        let mut tables_at = None;
        for _ in 0..nstreams {
            let off = md + rd32(&a.bytes, o)? as usize;
            let size = rd32(&a.bytes, o + 4)? as usize;
            o += 8;
            let end = a.bytes[o..].iter().position(|b| *b == 0).context("stream name")?;
            let name = std::str::from_utf8(&a.bytes[o..o + end])?.to_owned();
            o += (end + 4) & !3;
            ensure!(off + size <= a.bytes.len(), "stream {name} outside the image");
            match name.as_str() {
                "#~" | "#-" => tables_at = Some(off),
                "#Strings" => a.strings = (off, size),
                "#US" => a.user_strings = (off, size),
                "#Blob" => a.blobs = (off, size),
                _ => {}
            }
        }
        let t = tables_at.context("no #~ stream")?;
        let heaps = a.bytes[t + 6];
        a.wide_str = heaps & 1 != 0;
        a.wide_guid = heaps & 2 != 0;
        a.wide_blob = heaps & 4 != 0;
        let valid = u64::from_le_bytes(a.bytes[t + 8..t + 16].try_into()?);
        let mut o = t + 24;
        for i in 0..64 {
            if valid >> i & 1 != 0 {
                a.rows[i] = rd32(&a.bytes, o)?;
                o += 4;
            }
        }
        for i in 0..64 {
            if a.rows[i] == 0 {
                continue;
            }
            ensure!(i <= 0x2C, "unknown metadata table {i:#x}");
            let size: usize = schema(i).iter().map(|c| a.col_size(*c)).sum();
            a.tables[i] = (o, size);
            o += size * a.rows[i] as usize;
            ensure!(o <= a.bytes.len(), "metadata tables outside the image");
        }
        Ok(a)
    }

    fn col_size(&self, c: Col) -> usize {
        match c {
            Col::U8 => 1,
            Col::U16 => 2,
            Col::U32 => 4,
            Col::Str => if self.wide_str { 4 } else { 2 },
            Col::Guid => if self.wide_guid { 4 } else { 2 },
            Col::Blob => if self.wide_blob { 4 } else { 2 },
            Col::Idx(t) => if self.rows[t] > 0xffff { 4 } else { 2 },
            Col::Coded(ts) => {
                let bits = (usize::BITS - (ts.len() - 1).leading_zeros()) as u32;
                let max = ts.iter().filter(|t| **t != NONE).map(|t| self.rows[*t]).max().unwrap_or(0);
                if max < (1 << (16 - bits)) { 2 } else { 4 }
            }
        }
    }

    /// File offset of an RVA.
    pub fn offset(&self, rva: u32) -> Result<usize> {
        let s = self.sections.iter().find(|s| rva >= s.va && rva < s.va + s.size).with_context(|| format!("RVA {rva:#x} outside the sections"))?;
        Ok((rva - s.va + s.raw) as usize)
    }

    pub fn rows(&self, table: usize) -> u32 {
        self.rows[table]
    }

    /// Column `col` of 1-based `row`: plain values, heap offsets, table rows
    /// and coded indices (as `table << 24 | row`).
    pub fn cell(&self, table: usize, row: u32, col: usize) -> Result<u32> {
        ensure!(row >= 1 && row <= self.rows[table], "row {row} outside table {table:#x}");
        let (base, size) = self.tables[table];
        let cols = schema(table);
        let mut o = base + size * (row as usize - 1);
        for c in &cols[..col] {
            o += self.col_size(*c);
        }
        let c = cols[col];
        let raw = match self.col_size(c) {
            1 => self.bytes[o] as u32,
            2 => rd16(&self.bytes, o)? as u32,
            _ => rd32(&self.bytes, o)?,
        };
        Ok(match c {
            Col::Coded(ts) => {
                let bits = (usize::BITS - (ts.len() - 1).leading_zeros()) as u32;
                let tag = (raw & ((1 << bits) - 1)) as usize;
                let t = *ts.get(tag).context("coded index tag")?;
                ((t as u32) << 24) | (raw >> bits)
            }
            _ => raw,
        })
    }

    pub fn string(&self, off: u32) -> Result<&str> {
        let (base, size) = self.strings;
        let o = off as usize;
        ensure!(o < size.max(1), "#Strings offset {o}");
        let b = &self.bytes[base + o..base + size];
        let end = b.iter().position(|x| *x == 0).unwrap_or(b.len());
        Ok(std::str::from_utf8(&b[..end])?)
    }

    /// `#Blob` entry.
    pub fn blob(&self, off: u32) -> Result<&[u8]> {
        let (base, size) = self.blobs;
        let mut o = base + off as usize;
        ensure!((off as usize) < size, "#Blob offset {off}");
        let len = compressed(&self.bytes, &mut o)? as usize;
        self.bytes.get(o..o + len).context("blob outside the heap")
    }

    /// `#US` string (UTF-16).
    pub fn user_string(&self, off: u32) -> Result<String> {
        let (base, size) = self.user_strings;
        ensure!((off as usize) < size, "#US offset {off}");
        let mut o = base + off as usize;
        let len = compressed(&self.bytes, &mut o)? as usize;
        let b = self.bytes.get(o..o + len.saturating_sub(1)).context("#US outside the heap")?;
        let units: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        Ok(String::from_utf16_lossy(&units))
    }

    /// Bytes at an RVA.
    pub fn at_rva(&self, rva: u32, len: usize) -> Result<&[u8]> {
        let o = self.offset(rva)?;
        self.bytes.get(o..o + len).context("RVA data outside the image")
    }

    /// `namespace.name` of a TypeDef or TypeRef row, nested types as
    /// `Outer/Inner`.
    pub fn type_name(&self, t: Token) -> Result<String> {
        match t.table {
            table::TYPE_DEF => {
                let name = self.string(self.cell(table::TYPE_DEF, t.row, 1)?)?;
                let ns = self.string(self.cell(table::TYPE_DEF, t.row, 2)?)?;
                if let Some(outer) = self.enclosing(t.row)? {
                    return Ok(format!("{}/{name}", self.type_name(Token { table: table::TYPE_DEF, row: outer })?));
                }
                Ok(if ns.is_empty() { name.to_owned() } else { format!("{ns}.{name}") })
            }
            table::TYPE_REF => {
                let scope = Token::from_u32(self.cell(table::TYPE_REF, t.row, 0)?);
                let name = self.string(self.cell(table::TYPE_REF, t.row, 1)?)?;
                let ns = self.string(self.cell(table::TYPE_REF, t.row, 2)?)?;
                if scope.table == table::TYPE_REF && scope.row != 0 {
                    return Ok(format!("{}/{name}", self.type_name(scope)?));
                }
                Ok(if ns.is_empty() { name.to_owned() } else { format!("{ns}.{name}") })
            }
            table::TYPE_SPEC => Ok(format!("<typespec {}>", t.row)),
            other => bail!("token table {other:#x} is not a type"),
        }
    }

    /// The enclosing TypeDef of a nested TypeDef row.
    pub fn enclosing(&self, row: u32) -> Result<Option<u32>> {
        for r in 1..=self.rows[table::NESTED_CLASS] {
            if self.cell(table::NESTED_CLASS, r, 0)? == row {
                return Ok(Some(self.cell(table::NESTED_CLASS, r, 1)?));
            }
        }
        Ok(None)
    }

    /// TypeDef row by full name (`Ns.Name` or `Outer/Inner`).
    pub fn find_type(&self, name: &str) -> Result<Option<u32>> {
        for r in 1..=self.rows[table::TYPE_DEF] {
            if self.type_name(Token { table: table::TYPE_DEF, row: r })? == name {
                return Ok(Some(r));
            }
        }
        Ok(None)
    }

    /// Field and method row ranges of a TypeDef.
    pub fn members(&self, row: u32) -> Result<(std::ops::Range<u32>, std::ops::Range<u32>)> {
        let next = |col: usize, table: usize| -> Result<u32> {
            Ok(if row < self.rows[table::TYPE_DEF] { self.cell(table::TYPE_DEF, row + 1, col)? } else { self.rows[table] + 1 })
        };
        let f0 = self.cell(table::TYPE_DEF, row, 4)?;
        let m0 = self.cell(table::TYPE_DEF, row, 5)?;
        Ok((f0..next(4, table::FIELD)?, m0..next(5, table::METHOD_DEF)?))
    }

    /// The TypeDef owning a Field row.
    pub fn field_owner(&self, field: u32) -> Result<u32> {
        let n = self.rows[table::TYPE_DEF];
        for r in 1..=n {
            if self.members(r)?.0.contains(&field) {
                return Ok(r);
            }
        }
        bail!("field {field} has no owner")
    }

    /// The TypeDef owning a MethodDef row.
    pub fn method_owner(&self, method: u32) -> Result<u32> {
        let n = self.rows[table::TYPE_DEF];
        for r in 1..=n {
            if self.members(r)?.1.contains(&method) {
                return Ok(r);
            }
        }
        bail!("method {method} has no owner")
    }

    /// `Constant` value blob of a Field row: element type and bytes.
    pub fn field_constant(&self, field: u32) -> Result<Option<(u8, &[u8])>> {
        let want = ((table::FIELD as u32) << 24) | field;
        for r in 1..=self.rows[table::CONSTANT] {
            if self.cell(table::CONSTANT, r, 2)? == want {
                return Ok(Some((self.cell(table::CONSTANT, r, 0)? as u8, self.blob(self.cell(table::CONSTANT, r, 3)?)?)));
            }
        }
        Ok(None)
    }

    /// `FieldRVA` data of a Field row (array initializers).
    pub fn field_rva(&self, field: u32) -> Result<Option<u32>> {
        for r in 1..=self.rows[table::FIELD_RVA] {
            if self.cell(table::FIELD_RVA, r, 1)? == field {
                return Ok(Some(self.cell(table::FIELD_RVA, r, 0)?));
            }
        }
        Ok(None)
    }

    /// IL bytes of a MethodDef (tiny or fat header).
    pub fn method_body(&self, method: u32) -> Result<&[u8]> {
        let rva = self.cell(table::METHOD_DEF, method, 0)?;
        ensure!(rva != 0, "method {method} has no body");
        let o = self.offset(rva)?;
        let h = *self.bytes.get(o).context("method header")?;
        let (start, len) = match h & 3 {
            2 => (o + 1, (h >> 2) as usize),
            3 => {
                let size = (rd16(&self.bytes, o)? >> 12) as usize * 4;
                (o + size, rd32(&self.bytes, o + 4)? as usize)
            }
            _ => bail!("unknown method header {h:#x}"),
        };
        self.bytes.get(start..start + len).context("method body outside the image")
    }
}

/// ECMA-335 II.23.2 compressed unsigned integer.
pub fn compressed(b: &[u8], o: &mut usize) -> Result<u32> {
    let x = *b.get(*o).context("compressed integer")? as u32;
    let (v, n) = if x & 0x80 == 0 {
        (x, 1)
    } else if x & 0xc0 == 0x80 {
        (((x & 0x3f) << 8) | *b.get(*o + 1).context("compressed integer")? as u32, 2)
    } else {
        let s = b.get(*o..*o + 4).context("compressed integer")?;
        (((x & 0x1f) << 24) | (s[1] as u32) << 16 | (s[2] as u32) << 8 | s[3] as u32, 4)
    };
    *o += n;
    Ok(v)
}

#[cfg(test)]
mod tests {
    #[test]
    fn compressed_integers() {
        let mut o = 0;
        assert_eq!(super::compressed(&[0x03], &mut o).unwrap(), 3);
        o = 0;
        assert_eq!(super::compressed(&[0x80, 0x80], &mut o).unwrap(), 0x80);
        assert_eq!(o, 2);
        o = 0;
        assert_eq!(super::compressed(&[0xC0, 0x00, 0x40, 0x00], &mut o).unwrap(), 0x4000);
    }

    #[test]
    fn rejects_non_pe() {
        assert!(super::Assembly::parse(b"not a pe".to_vec()).is_err());
    }
}

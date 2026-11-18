// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Compiled codeX RScript programs (`.gsc`).
//!
//! A nine-dword header gives the section sizes; sections follow in header
//! order. String, array and label indexes are 32-bit entries into their data
//! sections. Label index 0 is a placeholder.
use anyhow::{Context, Result, bail, ensure};

/// One decoded program. Strings stay Shift-JIS bytes.
#[derive(Debug)]
pub struct Program {
    pub code: Vec<u8>,
    pub strings: Vec<Vec<u8>>,
    /// Word arrays copied into script variables by opcode 0x12;
    /// index 0 is unused.
    pub arrays: Vec<Vec<i16>>,
    pub labels: Vec<Label>,
}

#[derive(Debug, Clone)]
pub struct Label {
    pub name: Vec<u8>,
    pub offset: u32,
}

const HEADER: usize = 36;
/// Bounds every section so a corrupt header cannot request huge allocations.
const MAX_FILE: usize = 64 * 1024 * 1024;

fn u32_at(b: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(b.get(at..at + 4).context("GSC read past end")?.try_into()?))
}
fn i16_at(b: &[u8], at: usize) -> Result<i16> {
    Ok(i16::from_le_bytes(b.get(at..at + 2).context("GSC read past end")?.try_into()?))
}
fn c_string(data: &[u8], at: usize, what: &str) -> Result<Vec<u8>> {
    let tail = data.get(at..).with_context(|| format!("GSC {what} offset out of range"))?;
    let end = tail.iter().position(|&b| b == 0).with_context(|| format!("unterminated GSC {what}"))?;
    Ok(tail[..end].to_vec())
}

impl Program {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() >= HEADER && bytes.len() <= MAX_FILE, "GSC program size");
        let h: Vec<usize> = (0..9).map(|i| u32_at(bytes, i * 4).map(|v| v as usize)).collect::<Result<_>>()?;
        let [file_size, header_size, code_size, string_index, string_size, array_index, array_words, label_index, label_size] =
            h[..]
        else {
            unreachable!()
        };
        ensure!(header_size == HEADER && file_size == bytes.len(), "invalid GSC header");
        ensure!(string_index % 4 == 0 && array_index % 4 == 0 && label_index % 4 == 0, "misaligned GSC index");
        let mut at = HEADER;
        let mut take = |size: usize| -> Result<&[u8]> {
            let end = at.checked_add(size).filter(|&e| e <= bytes.len()).context("GSC section past end")?;
            let s = &bytes[at..end];
            at = end;
            Ok(s)
        };
        let code = take(code_size)?.to_vec();
        let string_ix = take(string_index)?;
        let string_data = take(string_size)?;
        let array_ix = take(array_index)?;
        let array_data = take(array_words.checked_mul(2).context("GSC array size")?)?;
        let label_names = take(label_index)?;
        let label_offsets = take(label_index)?;
        let label_data = take(label_size)?;
        ensure!(at == bytes.len(), "GSC section sizes do not match the file");
        let strings = (0..string_index / 4)
            .map(|i| c_string(string_data, u32_at(string_ix, i * 4)? as usize, "string"))
            .collect::<Result<_>>()?;
        let arrays = (0..array_index / 4)
            .map(|i| {
                let start = u32_at(array_ix, i * 4)? as usize;
                if start >= array_words {
                    return Ok(Vec::new());
                }
                let len = i16_at(array_data, start * 2)?;
                ensure!(len >= 0 && start + 1 + len as usize <= array_words, "invalid GSC array");
                (0..len as usize).map(|j| i16_at(array_data, (start + 1 + j) * 2)).collect()
            })
            .collect::<Result<_>>()?;
        let labels = (0..label_index / 4)
            .map(|i| {
                let offset = u32_at(label_offsets, i * 4)?;
                let name = if i == 0 { Vec::new() } else { c_string(label_data, u32_at(label_names, i * 4)? as usize, "label")? };
                Ok(Label { name, offset })
            })
            .collect::<Result<_>>()?;
        Ok(Self { code, strings, arrays, labels })
    }

    /// Exact match over labels 1..n; an unknown name selects offset 0.
    pub fn label_offset(&self, name: Option<&[u8]>) -> u32 {
        let Some(name) = name.filter(|n| !n.is_empty()) else { return 0 };
        self.labels.iter().skip(1).find(|l| l.name == name).map_or(0, |l| l.offset)
    }
}

/// Operand encodings of the per-revision dispatcher tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    U16,
    I16,
    U32,
    /// 32-bit value: low signed word, high word counts indirections through
    /// variables.
    Value,
    /// Index into the string table.
    Str,
    /// String-table index naming a label; 0 selects the program start.
    Label,
    /// Absolute code offset.
    Target,
    /// Index into the word-array table.
    Array,
}
impl Kind {
    pub const fn size(self) -> usize {
        match self {
            Kind::U16 | Kind::I16 => 2,
            _ => 4,
        }
    }
}

/// True for the expression family whose operand modes live in the opcode.
pub const fn is_expression(op: u16) -> bool {
    op & 0xF000 != 0
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub offset: u32,
    pub opcode: u16,
    /// Raw operand values in encoding order (values keep their 32-bit form;
    /// `I16` is sign-extended).
    pub operands: Vec<i32>,
    pub next: u32,
}

/// Decodes one instruction. Expression opcodes carry a destination register
/// and one (0xF000 unary load) or two signed 16-bit operands.
pub fn decode(code: &[u8], offset: u32, layout: impl Fn(u16) -> Option<&'static [Kind]>) -> Result<Instruction> {
    let at = offset as usize;
    let op = u16::from_le_bytes(code.get(at..at + 2).context("GSC opcode past end")?.try_into()?);
    let mut cursor = at + 2;
    let mut operands = Vec::new();
    let mut take = |kind: Kind| -> Result<()> {
        let b = code.get(cursor..cursor + kind.size()).context("GSC operand past end")?;
        operands.push(match kind {
            Kind::U16 => i32::from(u16::from_le_bytes([b[0], b[1]])),
            Kind::I16 => i32::from(i16::from_le_bytes([b[0], b[1]])),
            _ => i32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        });
        cursor += kind.size();
        Ok(())
    };
    if is_expression(op) {
        take(Kind::U16)?;
        take(Kind::I16)?;
        if op & 0xF000 != 0xF000 {
            take(Kind::I16)?;
        }
    } else {
        let Some(kinds) = layout(op) else { bail!("unknown GSC opcode {op:#x} at {offset:#x}") };
        for &k in kinds {
            take(k)?;
        }
    }
    Ok(Instruction { offset, opcode: op, operands, next: cursor as u32 })
}

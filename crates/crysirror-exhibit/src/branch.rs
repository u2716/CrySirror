// SPDX-License-Identifier: MPL-2.0
//! v1 RetouchSystem::liteCheckCondition and blockId2Index.
use super::{cstring, word};
use crate::registers::{Registers, Value};
use anyhow::{bail, ensure, Context, Result};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug)]
pub(super) struct Position {
    pub pc: usize,
    pub remaining: u32,
}

pub(super) fn labels(bytes: &[u8], mut pos: Position, abi: u32) -> Result<BTreeMap<i32, Position>> {
    let mut labels = BTreeMap::new();
    let start = pos;
    let mut block = 0i32;
    while pos.remaining > 0 {
        let header = word(bytes, pos.pc)?;
        let n = (header >> 16) as u8;
        let flags = (header >> 24) as u8;
        let operands = pos.pc + 4;
        let mut at = operands + usize::from(n) * 4;
        ensure!(at <= bytes.len(), "truncated RLD record at {:#x}", pos.pc);
        let mut first = None;
        for _ in 0..(flags & 15) {
            let (field, end) = cstring(bytes, at)?;
            first.get_or_insert(field);
            at = end;
        }
        match header as u16 {
            5 => {
                let id = if flags & (if abi == 3 { 0xf0 } else { 0x70 }) != 0 {
                    let id = first
                        .context("missing block definition")?
                        .split(|b| *b == b',')
                        .next()
                        .unwrap_or_default();
                    number(id)?
                } else {
                    ensure!(n > 0, "missing numeric block ID");
                    word(bytes, operands)? as i32
                };
                block = if id == -1 { block.wrapping_add(1) } else { id };
                labels.entry(block).or_insert(pos);
            }
            47 if n > 0 => {
                labels.entry(word(bytes, operands)? as i32).or_insert(pos);
            }
            _ => {}
        }
        pos = Position {
            pc: at,
            remaining: pos.remaining - 1,
        };
    }
    ensure!(pos.pc == bytes.len(), "trailing bytes after RLD records");
    labels.entry(0).or_insert(start);
    Ok(labels)
}

pub(super) fn number(raw: &[u8]) -> Result<i32> {
    Ok(std::str::from_utf8(raw.trim_ascii())?.parse()?)
}

#[derive(Debug, PartialEq)]
enum Comparison {
    Constant(bool),
    Numeric(Value, u8, Value),
    Text(i32, bool, Vec<u8>),
}

#[derive(Debug, PartialEq)]
pub(super) struct Condition(Vec<Vec<Comparison>>);

impl Condition {
    pub fn text(raw: &[u8]) -> Result<Self> {
        Self::for_abi(raw, 1)
    }
    pub fn for_abi(raw: &[u8], abi: u32) -> Result<Self> {
        if raw == b"*" || (abi == 3 && raw.is_empty()) {
            return Ok(Self(vec![vec![]]));
        }
        let mut groups = Vec::new();
        for group in raw
            .split(|b| *b == b';')
            .take(if abi == 3 { 24 } else { 16 })
        {
            let mut terms = Vec::new();
            for term in group.split(|b| *b == b'&') {
                let term = term.trim_ascii();
                if abi == 3 && matches!(term, b"true" | b"false") {
                    terms.push(Comparison::Constant(term == b"true"));
                    continue;
                }
                let at = term
                    .windows(2)
                    .position(|p| p[0] == 0x81 && (0x81..=0x86).contains(&p[1]))
                    .context("unsupported ExHibit text condition")?;
                // CP932 full-width =, !=, <, >, <=, >= mapped to the
                // comparison numbers used by the binary condition records.
                let op = [0, 1, 5, 2, 4, 3][usize::from(term[at + 1] - 0x81)];
                terms.push(Comparison::Numeric(
                    Value::expression(&term[..at])?,
                    op,
                    Value::expression(&term[at + 2..])?,
                ));
            }
            groups.push(terms);
        }
        Ok(Self(groups))
    }

    pub fn binary(values: &[i32], strings: &[&[u8]]) -> Result<Self> {
        ensure!(
            values.len() == 16 && strings.len() == 5,
            "invalid RLD binary condition layout"
        );
        let mut terms = Vec::new();
        for (v, s) in values[1..].chunks_exact(3).zip(strings) {
            let (kind, index, op) = (v[0], v[1], v[2]);
            if kind < 0 {
                continue;
            }
            match kind {
                0 | 1 if (0..=5).contains(&op) => {
                    let left = if kind == 0 {
                        Value::Local(index)
                    } else {
                        Value::RelativeSystem(index)
                    };
                    terms.push(Comparison::Numeric(left, op as u8, Value::expression(s)?));
                }
                2 if matches!(op, 0 | 1) => {
                    terms.push(Comparison::Text(index, op == 0, s.to_vec()))
                }
                _ => bail!("unsupported binary condition {kind}/{op}"),
            }
        }
        Ok(Self(vec![terms]))
    }
    pub fn matches(&self, regs: &Registers) -> Result<bool> {
        for group in &self.0 {
            let mut pass = true;
            for term in group {
                pass = match term {
                    Comparison::Constant(value) => *value,
                    Comparison::Numeric(left, op, right) => {
                        let (a, b) = (regs.read(*left)?, regs.read(*right)?);
                        match op {
                            0 => a == b,
                            1 => a != b,
                            2 => a > b,
                            3 => a >= b,
                            4 => a <= b,
                            5 => a < b,
                            _ => unreachable!(),
                        }
                    }
                    Comparison::Text(index, expected, bytes) => {
                        (regs.text(*index)? == bytes.as_slice()) == *expected
                    }
                };
                if !pass {
                    break;
                }
            }
            if pass {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[derive(Debug, PartialEq)]
pub(super) struct Block {
    pub id: i32,
    pub marker: bool,
    pub title: Vec<u8>,
    pub false_target: i32,
    pub condition: Condition,
    pub checkpoint_flag: Option<bool>,
}
impl Block {
    pub fn binary(id: i32, marker: i32, title: &[u8]) -> Result<Self> {
        Ok(Self {
            id,
            title: title.to_vec(),
            marker: marker != 0,
            false_target: -1,
            condition: Condition::text(b"*")?,
            checkpoint_flag: None,
        })
    }
    pub fn csv(raw: &[u8], abi: u32, extended: bool) -> Result<Self> {
        let fields: Vec<_> = raw
            .splitn(if extended { 7 } else { 5 }, |b| *b == b',')
            .collect();
        ensure!(
            fields.len() == if extended { 7 } else { 5 },
            "invalid ExHibit block layout"
        );
        let flags = number(fields[1])?;
        ensure!(
            abi != 3 || flags & 0x30 == 0,
            "unsupported ExHibit block message/control entry flags {flags:#x}"
        );
        Ok(Self {
            id: number(fields[0])?,
            title: fields[fields.len() - 1].to_vec(),
            marker: flags & 1 != 0,
            false_target: number(fields[2])?,
            condition: Condition::for_abi(fields[3], abi)?,
            checkpoint_flag: (abi == 3 && flags & 1 == 0).then_some(flags & 0x40 != 0),
        })
    }
}

// Copyright (c) 2026 CrySirror contributors. MIT; see LICENSE.
//! Bounded, zero-copy WS2 decoding. Profiles specify operand layouts only;
//! decoding a command does not execute it or acknowledge a native operation.
use anyhow::{bail, ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{ops::Range, sync::Arc};

pub const MAX_SCRIPT_BYTES: usize = 64 << 20;
const MAX_INSTRUCTIONS: usize = 1_000_000;
const MAX_OPERANDS: usize = 4_000_000;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    V1,
    V2,
    V3,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operand {
    Byte(u8),
    Short(i16),
    Integer(i32),
    Address(u32),
    FloatBits(u32),
    /// Raw CP932 bytes excluding NUL; no host-locale or lossy conversion.
    String(Range<usize>),
    Array(Vec<Operand>),
}
#[derive(Clone, Debug)]
pub struct Choice {
    pub operands: Vec<Operand>,
    /// Retained opcode and operands; the reference does not assume opcode02.
    pub continuation: Instruction,
}
#[derive(Clone, Debug)]
pub struct Instruction {
    pub pc: u32,
    pub next: u32,
    pub opcode: u8,
    pub operands: Vec<Operand>,
    pub choices: Vec<Choice>,
}
#[derive(Clone, Debug)]
pub struct Program {
    pub profile: Profile,
    bytes: Arc<[u8]>,
    instructions: Arc<[Instruction]>,
    pub trailer: Range<usize>,
}
struct Decoder<'a, F> {
    bytes: &'a [u8],
    profile: Profile,
    at: usize,
    operands: usize,
    check: F,
}
impl<F: Fn() -> Result<()>> Decoder<'_, F> {
    fn take(&mut self, n: usize) -> Result<&[u8]> {
        let end = self
            .at
            .checked_add(n)
            .context("WS2 operand extent overflow")?;
        let bytes = self
            .bytes
            .get(self.at..end)
            .context("truncated WS2 operand")?;
        self.at = end;
        Ok(bytes)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn operand(&mut self, kind: u8) -> Result<Operand> {
        (self.check)()?;
        self.operands = self
            .operands
            .checked_add(1)
            .context("WS2 operand count overflow")?;
        ensure!(self.operands <= MAX_OPERANDS, "WS2 operand budget");
        Ok(match kind {
            b'b' => Operand::Byte(self.byte()?),
            b'h' => Operand::Short(i16::from_le_bytes(self.take(2)?.try_into()?)),
            b'i' => Operand::Integer(i32::from_le_bytes(self.take(4)?.try_into()?)),
            b'f' => Operand::FloatBits(u32::from_le_bytes(self.take(4)?.try_into()?)),
            b'a' => {
                let value = i32::from_le_bytes(self.take(4)?.try_into()?);
                ensure!(
                    value >= 0 && (value as usize) < self.bytes.len(),
                    "WS2 address outside source"
                );
                Operand::Address(value as u32)
            }
            b's' => {
                let start = self.at;
                let end = self.bytes[start..]
                    .iter()
                    .take((1 << 20) + 1)
                    .position(|b| *b == 0)
                    .context("unterminated or oversized WS2 string")?;
                self.at = start + end + 1;
                Operand::String(start..start + end)
            }
            _ => bail!("unknown WS2 operand layout byte {kind:#x}"),
        })
    }
    fn operands(&mut self, template: &str) -> Result<Vec<Operand>> {
        let mut out = Vec::new();
        let mut tags = template.bytes().peekable();
        while let Some(kind) = tags.next() {
            if tags.peek() == Some(&b'*') {
                tags.next();
                let count = self.byte()?;
                let mut values = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    values.push(self.operand(kind)?);
                }
                out.push(Operand::Array(values));
            } else {
                out.push(self.operand(kind)?);
            }
        }
        Ok(out)
    }
    fn instruction(&mut self, choice_body: bool) -> Result<Instruction> {
        (self.check)()?;
        let pc = u32::try_from(self.at)?;
        let opcode = self.byte()?;
        let template = crate::tables::operands(self.profile, opcode).with_context(|| {
            format!(
                "unknown WS2 {:?} opcode {opcode:#04x} at pc={pc:#x}",
                self.profile
            )
        })?;
        let operands = self.operands(template)?;
        let mut choices = Vec::new();
        if opcode == 0x0f && choice_body {
            let Some(Operand::Byte(count)) = operands.first() else {
                bail!("WS2 choice count layout");
            };
            for _ in 0..*count {
                let operands = self.operands("hsbh")?;
                // ReadInstruction in the reference does not recursively invoke
                // its text/choice handlers on an embedded continuation.
                let continuation = self.instruction(false)?;
                choices.push(Choice {
                    operands,
                    continuation,
                });
            }
        }
        Ok(Instruction {
            pc,
            next: u32::try_from(self.at)?,
            opcode,
            operands,
            choices,
        })
    }
}
impl Program {
    pub fn parse(
        profile: Profile,
        bytes: Arc<[u8]>,
        check: impl Fn() -> Result<()>,
    ) -> Result<Self> {
        ensure!(
            (9..=MAX_SCRIPT_BYTES).contains(&bytes.len()),
            "WS2 source byte budget/empty program"
        );
        let mut decoder = Decoder {
            bytes: &bytes,
            profile,
            at: 0,
            operands: 0,
            check,
        };
        let mut instructions = Vec::new();
        // The reference's boundary leaves the final eight bytes uninterpreted.
        // Retain the actual unconsumed tail; do not label its unknown fields.
        while decoder.at < bytes.len() - 8 {
            ensure!(
                instructions.len() < MAX_INSTRUCTIONS,
                "WS2 instruction budget"
            );
            instructions.push(decoder.instruction(true)?);
        }
        (decoder.check)()?;
        let trailer = decoder.at..bytes.len();
        Ok(Self {
            profile,
            bytes,
            instructions: instructions.into(),
            trailer,
        })
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }
    pub fn instruction_at(&self, pc: u32) -> Result<&Instruction> {
        self.instructions
            .binary_search_by_key(&pc, |i| i.pc)
            .ok()
            .map(|i| &self.instructions[i])
            .with_context(|| {
                format!("WS2 pc={pc:#x} is not a top-level decoded instruction boundary")
            })
    }
}

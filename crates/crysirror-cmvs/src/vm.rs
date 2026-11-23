// SPDX-License-Identifier: MPL-2.0

//! Structural decoder for the CMVS 20160201 x64 profile recorded in `NATIVE_ABI.md`.
use std::fmt;

mod abi;
mod control;
mod cvns;
mod cvns_scratch;
mod evaluate;
mod gesture;
mod history;
mod input;
mod machine;
mod native_state;
mod noop;
mod resources;
mod save_menu;
#[cfg(any(test, feature = "runtime"))]
pub(crate) mod save_slot;
#[cfg(feature = "runtime")]
mod snapshot;
mod screen;
mod sequence;
mod startup;
mod strings;
mod system_save;
#[cfg(feature = "runtime")]
pub(crate) mod css0;
#[cfg(any(test, feature = "runtime"))]
mod csp_banks;
#[cfg(feature = "runtime")]
pub(crate) use resources::cpz6_from_pe;
#[cfg(feature = "runtime")]
pub use snapshot::SavedScripts;
#[cfg(feature="runtime")]
pub(crate) use startup::csp::RestoredVm;
pub(crate) use cvns::selector as cvns_selector;
#[cfg(feature = "runtime")]
pub(crate) use sequence::Kind as SequenceKind;
pub use resources::{Cpz6Resources, ResourceBindings, ResourceSource, ScriptSources};
#[cfg(feature = "runtime")]
pub(crate) use save_menu::{
    latest_slot_range, open_save_slot, probe_save_slot, scan_latest_slot, scan_cvns_menu, latest_cvns_slot, probe_cvns_slot, edit_slot_header, read_gameplay_metadata,
    SlotHeaderEdit, SlotHeaderScratch, SlotMetadata, SlotPreview,
};
pub use save_menu::{PreparedSaveMenu, SaveHeader, SaveMenu, SaveMenuRequest, SaveSlot};
pub use startup::{CmvsVm, PreparedTransfer, ScriptRequest, StartupSettings, VmYield};
#[cfg(feature = "runtime")]
pub(crate) use startup::{DrawProperty, EffectSound, FadeCommand, MainVmCheckpoint, SceneObject};
pub use system_save::{PreparedSystemSave, SystemSaveRequest};

const MAX_EXPRESSION_TERMS: usize = 4096;

/// Explicit executable ABI selection. Game identity remains script data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmProfile {
    Cvns20040515X86,
    // SATD-cmvs-20070119-profile covers behavior beyond the verified startup path.
    Pc20070119X86,
    Pc20160201X64,
    Pc20211007X64,
}

impl VmProfile {
    fn decode_at(self, code: &[u8], pc: usize) -> Result<Instruction, DecodeError> {
        match self {
            // The CVNS compiler leaves a 0xfffc word after some 0x0413 returns. The
            // 0x424850 loop has no case for it and would not advance, so the word
            // occupies a boundary that fails visibly if it is ever executed.
            Self::Cvns20040515X86 if code.get(pc..pc + 2) == Some(&[0xfc, 0xff]) => {
                Ok(Instruction {
                    pc,
                    end: pc + 2,
                    opcode: 0xfffc,
                    kind: InstructionKind::Simple,
                })
            }
            Self::Cvns20040515X86 | Self::Pc20070119X86 | Self::Pc20160201X64 | Self::Pc20211007X64 => decode_at(code, pc),
        }
    }

    fn script_format(self) -> crate::script::ScriptFormat {
        match self {
            Self::Cvns20040515X86 => crate::script::ScriptFormat::Pss0,
            Self::Pc20070119X86 | Self::Pc20160201X64 | Self::Pc20211007X64 => crate::script::ScriptFormat::Ps2a,
        }
    }

    fn archive_table_rva(self) -> u32 {
        match self {
            Self::Cvns20040515X86 => 0x0005_dd80,
            Self::Pc20070119X86 => 0x0009_0760,
            Self::Pc20160201X64 => 0x000d_9c40,
            Self::Pc20211007X64 => 0x000e_1be0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError {
    pub pc: usize,
    pub opcode: Option<u16>,
    pub reason: &'static str,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "code+0x{:x}", self.pc)?;
        if let Some(op) = self.opcode {
            write!(f, " opcode=0x{op:04x}")?;
        }
        write!(f, ": {}", self.reason)
    }
}
impl std::error::Error for DecodeError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    Scalar {
        pc: usize,
        tag: u16,
        raw: u32,
    },
    Nested {
        pc: usize,
        tag: u16,
        expression: Box<Expression>,
    },
    Indexed {
        pc: usize,
        tag: u16,
        base: u32,
        stride: u16,
        indices: Vec<Expression>,
    },
    Operator {
        pc: usize,
        opcode: u16,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expression {
    pub pc: usize,
    /// 0x0200 integer, 0x0201 string reference, 0x0202 float.
    pub kind: u16,
    pub end: usize,
    pub terms: Vec<Term>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    pub pc: usize,
    pub end: usize,
    pub opcode: u16,
    pub kind: InstructionKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstructionKind {
    Expression(Expression),
    Jump { target: u32 },
    ConditionalJump { target: u32 },
    CompareJump { value: u32, target: u32 },
    LongJump { reserved: u32, target: u32 },
    Label { index: u16 },
    StackAdjust { bytes: u16 },
    StackImmediate { bytes: u16, raw: u32 },
    ScriptReturn { reserved: u16 },
    Simple,
    System { command: u16 },
}

struct Reader<'a> {
    code: &'a [u8],
    pos: usize,
    remaining_terms: usize,
}
impl Reader<'_> {
    fn error(&self, pc: usize, opcode: Option<u16>, reason: &'static str) -> DecodeError {
        DecodeError { pc, opcode, reason }
    }
    fn word(&mut self) -> Result<u16, DecodeError> {
        let bytes = self
            .code
            .get(self.pos..self.pos + 2)
            .ok_or_else(|| self.error(self.pos, None, "truncated 16-bit operand"))?;
        self.pos += 2;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }
    fn dword(&mut self) -> Result<u32, DecodeError> {
        let bytes = self
            .code
            .get(self.pos..self.pos + 4)
            .ok_or_else(|| self.error(self.pos, None, "truncated 32-bit operand"))?;
        self.pos += 4;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
    fn expression(&mut self, depth: usize) -> Result<Expression, DecodeError> {
        let pc = self.pos;
        if depth >= 64 {
            return Err(self.error(pc, None, "expression nesting limit"));
        }
        let kind = self.word()?;
        if !(0x200..=0x202).contains(&kind) {
            return Err(self.error(pc, Some(kind), "expected expression prefix"));
        }
        let mut terms = Vec::new();
        loop {
            let at = self.pos;
            let tag = self.word()?;
            if tag == 0x20f {
                if terms.is_empty() {
                    return Err(self.error(at, Some(tag), "empty expression"));
                }
                return Ok(Expression {
                    pc,
                    kind,
                    end: self.pos,
                    terms,
                });
            }
            if terms.len() >= 128 {
                return Err(self.error(at, Some(tag), "expression term limit"));
            }
            if self.remaining_terms == 0 {
                return Err(self.error(at, Some(tag), "instruction expression term budget"));
            }
            self.remaining_terms -= 1;
            let term = match tag {
                0x200 if kind != 0x201 => {
                    self.pos = at;
                    Term::Nested {
                        pc: at,
                        tag,
                        expression: Box::new(self.expression(depth + 1)?),
                    }
                }
                0x101 | 0x102 | 0x12b if kind != 0x201 => {
                    let expression = self.expression(depth + 1)?;
                    if expression.kind != 0x200 {
                        return Err(self.error(at, Some(tag), "index must be integer expression"));
                    }
                    Term::Nested {
                        pc: at,
                        tag,
                        expression: Box::new(expression),
                    }
                }
                0x121 if kind == 0x201 => {
                    let expression = self.expression(depth + 1)?;
                    if expression.kind != 0x200 {
                        return Err(self.error(at, Some(tag), "string index must be integer"));
                    }
                    Term::Nested {
                        pc: at,
                        tag,
                        expression: Box::new(expression),
                    }
                }
                0x105 | 0x109 | 0x110 | 0x112 | 0x12d | 0x12f | 0x134 | 0x136
                    if kind != 0x201 && (tag < 0x130 || kind == 0x202) =>
                {
                    let base = self.dword()?;
                    let expression = self.expression(depth + 1)?;
                    if expression.kind != 0x200 {
                        return Err(self.error(at, Some(tag), "index must be integer expression"));
                    }
                    Term::Indexed {
                        pc: at,
                        tag,
                        base,
                        stride: 4,
                        indices: vec![expression],
                    }
                }
                0x107 | 0x10b | 0x111 | 0x113 | 0x131 | 0x133 | 0x135 | 0x137
                    if kind != 0x201 && (tag < 0x130 || kind == 0x202) =>
                {
                    let base = self.dword()?;
                    let stride = self.word()?;
                    let mut indices = vec![self.expression(depth + 1)?];
                    if matches!(tag, 0x111 | 0x113 | 0x135 | 0x137) {
                        indices.push(self.expression(depth + 1)?);
                    }
                    if indices.iter().any(|e| e.kind != 0x200) {
                        return Err(self.error(at, Some(tag), "index must be integer expression"));
                    }
                    Term::Indexed {
                        pc: at,
                        tag,
                        base,
                        stride,
                        indices,
                    }
                }
                0x160..=0x17e if kind != 0x201 || tag <= 0x172 => Term::Operator {
                    pc: at,
                    opcode: tag,
                },
                0x120 | 0x122 | 0x125 | 0x127 if kind == 0x201 => Term::Scalar {
                    pc: at,
                    tag,
                    raw: self.dword()?,
                },
                0x100 | 0x103 | 0x104 | 0x106 | 0x108 | 0x10a | 0x10e | 0x10f | 0x129 | 0x12a
                | 0x12c | 0x12e | 0x130 | 0x132
                    if kind != 0x201 && (tag < 0x130 || kind == 0x202) =>
                {
                    Term::Scalar {
                        pc: at,
                        tag,
                        raw: self.dword()?,
                    }
                }
                _ => return Err(self.error(at, Some(tag), "unsupported expression token")),
            };
            terms.push(term);
        }
    }
}

/// Decode a single instruction at a code-relative byte offset.
pub fn decode_at(code: &[u8], pc: usize) -> Result<Instruction, DecodeError> {
    // Bound the public offset before cursor arithmetic, including usize::MAX input.
    if pc >= code.len() {
        return Err(DecodeError {
            pc,
            opcode: None,
            reason: "PC outside code",
        });
    }
    let mut reader = Reader {
        code,
        pos: pc,
        remaining_terms: MAX_EXPRESSION_TERMS,
    };
    let opcode = reader.word()?;
    let kind = match opcode {
        0x200..=0x202 => {
            reader.pos = pc;
            InstructionKind::Expression(reader.expression(0)?)
        }
        0x400 => InstructionKind::Jump {
            target: reader.dword()?,
        },
        0x401 | 0x402 => InstructionKind::ConditionalJump {
            target: reader.dword()?,
        },
        0x403 => InstructionKind::CompareJump {
            value: reader.dword()?,
            target: reader.dword()?,
        },
        0x407 => InstructionKind::LongJump {
            reserved: reader.dword()?,
            target: reader.dword()?,
        },
        0x405 | 0x410 => InstructionKind::Label {
            index: reader.word()?,
        },
        0x411 | 0x412 => InstructionKind::StackAdjust {
            bytes: reader.word()?,
        },
        0x414 => InstructionKind::ScriptReturn {
            reserved: reader.word()?,
        },
        0x413 | 0x416 | 0x430 => InstructionKind::Simple,
        0x440 | 0x442 => InstructionKind::StackImmediate {
            bytes: reader.word()?,
            raw: reader.dword()?,
        },
        0x2000..=0x27ff => InstructionKind::System {
            command: opcode & 0x7ff,
        },
        _ => return Err(reader.error(pc, Some(opcode), "unsupported instruction")),
    };
    Ok(Instruction {
        pc,
        end: reader.pos,
        opcode,
        kind,
    })
}

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperandKind {
    Immediate,
    UserMemoryViaVar,
    SystemMemoryViaVar,
    StackSlot,
    VariableSlot,
    TempMemoryViaVar,
    MemDatDirect,
    MemDatIndirect,
    ArgumentStack,
    ArgumentBase,
    LiteralSlot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Operand {
    pub raw: u32,
    pub kind: OperandKind,
    pub lo: u16,
    pub bank: u16,
}

impl Operand {
    pub fn decode(raw: u32) -> Self {
        let tag = raw & 0xF000_0000;
        let lo = (raw & 0xFFFF) as u16;
        let bank = ((raw >> 16) & 0x0FFF) as u16;
        let kind = match tag {
            0x0000_0000 => OperandKind::Immediate,
            0x1000_0000 => OperandKind::UserMemoryViaVar,
            0x2000_0000 => OperandKind::SystemMemoryViaVar,
            0x3000_0000 => OperandKind::StackSlot,
            0x4000_0000 => OperandKind::VariableSlot,
            0x5000_0000 => OperandKind::TempMemoryViaVar,
            0x6000_0000 => OperandKind::MemDatDirect,
            0x7000_0000 => OperandKind::MemDatIndirect,
            0x8000_0000 => OperandKind::ArgumentStack,
            0x9000_0000 => OperandKind::ArgumentBase,
            _ => OperandKind::LiteralSlot,
        };
        Self {
            raw,
            kind,
            lo,
            bank,
        }
    }

    pub fn signed_immediate(&self) -> i32 {
        self.raw as i32
    }
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            OperandKind::Immediate => write!(f, "imm({})", self.signed_immediate()),
            OperandKind::UserMemoryViaVar => {
                write!(f, "mem_user[var[{}]] raw=0x{:08X}", self.lo, self.raw)
            }
            OperandKind::SystemMemoryViaVar => {
                write!(f, "mem_system[var[{}]] raw=0x{:08X}", self.lo, self.raw)
            }
            OperandKind::StackSlot => write!(f, "stack[{}]", self.lo),
            OperandKind::VariableSlot => write!(f, "var[{}]", self.lo),
            OperandKind::TempMemoryViaVar => write!(
                f,
                "mem_temp[var[{}]+base+{}] raw=0x{:08X}",
                self.lo, self.bank, self.raw
            ),
            OperandKind::MemDatDirect => write!(
                f,
                "memdat[var[{}]+{}] raw=0x{:08X}",
                self.lo, self.bank, self.raw
            ),
            OperandKind::MemDatIndirect => {
                let ptr = (self.raw >> 14) & 0x3FFC;
                write!(
                    f,
                    "memdat_indirect[var[{}]+ptr@0x{:X}] raw=0x{:08X}",
                    self.lo, ptr, self.raw
                )
            }
            OperandKind::ArgumentStack => {
                write!(f, "arg_stack[-{}] raw=0x{:08X}", self.lo, self.raw)
            }
            OperandKind::ArgumentBase => write!(f, "arg_base raw=0x{:08X}", self.raw),
            OperandKind::LiteralSlot => write!(f, "literal_slot(0x{:08X})", self.raw),
        }
    }
}

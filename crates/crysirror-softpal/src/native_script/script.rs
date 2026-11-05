use crate::native_script::error::{PalError, Result};
use crate::native_script::util::read_u32_le;

pub const SCRIPT_MAGIC: &[u8; 4] = b"Sv20";
pub const SCRIPT_CODE_BASE: u32 = 12;

#[derive(Debug, Clone, Copy)]
pub struct ScriptImage<'a> {
    bytes: &'a [u8],
    check_value: u32,
    entry_pc: u32,
}

impl<'a> ScriptImage<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        if bytes.len() < SCRIPT_CODE_BASE as usize {
            return Err(PalError::ScriptTooSmall { len: bytes.len() });
        }

        let got = [bytes[0], bytes[1], bytes[2], bytes[3]];
        if &got != SCRIPT_MAGIC {
            return Err(PalError::BadMagic { got });
        }

        let check_value = read_u32_le(bytes, 4)?;
        let entry_pc = read_u32_le(bytes, 8)?;

        Ok(Self {
            bytes,
            check_value,
            entry_pc,
        })
    }

    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn check_value(&self) -> u32 {
        self.check_value
    }

    pub fn entry_pc(&self) -> u32 {
        self.entry_pc
    }

    pub fn code_base(&self) -> u32 {
        SCRIPT_CODE_BASE
    }
}

pub fn format_script_header(script: &ScriptImage<'_>) -> String {
    format!(
        "# magic=Sv20 check=0x{:08X} code_base=0x{:08X} entry=0x{:08X} size=0x{:08X}",
        script.check_value(),
        script.code_base(),
        script.entry_pc(),
        script.len()
    )
}

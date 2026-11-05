use crate::native_script::error::{PalError, Result};
use crate::native_script::script::SCRIPT_CODE_BASE;
use crate::native_script::util::read_u32_le;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PointTable {
    offsets: Vec<u32>,
}

impl PointTable {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() % 4 != 0 {
            return Err(PalError::PointTableMisaligned { len: bytes.len() });
        }
        let mut offsets = Vec::with_capacity(bytes.len() / 4);
        let mut offset = 0usize;
        while offset < bytes.len() {
            offsets.push(read_u32_le(bytes, offset)?);
            offset += 4;
        }
        Ok(Self { offsets })
    }

    pub fn len(&self) -> usize {
        self.offsets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.offsets.is_empty()
    }

    pub fn raw_offsets(&self) -> &[u32] {
        &self.offsets
    }

    pub fn resolve_target_pc(&self, point_id: u32) -> Result<Option<u32>> {
        if point_id == 0 {
            return Ok(None);
        }
        let entries = self.offsets.len();
        let id = point_id as usize;
        if id > entries {
            return Err(PalError::PointIdOutOfRange { point_id, entries });
        }
        let reverse_index = entries - id;
        let relative = self.offsets[reverse_index];
        let target = SCRIPT_CODE_BASE
            .checked_add(relative)
            .ok_or(PalError::ArithmeticOverflow)?;
        Ok(Some(target))
    }
}

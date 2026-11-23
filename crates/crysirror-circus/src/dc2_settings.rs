//! D.C.II P.C.404C90/404BE0 original3312-byte system.dat. This is not the
//! DC3WY19744-byte orDC4PH19328-byte preference block.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
pub const SYSTEM_BYTES: usize = 0xcf0;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(try_from = "Vec<u8>", into = "Vec<u8>")]
pub struct SystemData(Vec<u8>);
impl TryFrom<Vec<u8>> for SystemData {
    type Error = anyhow::Error;
    fn try_from(bytes: Vec<u8>) -> Result<Self> {
        ensure!(
            bytes.len() == SYSTEM_BYTES,
            "DC2 original system.dat requires exactly{SYSTEM_BYTES} bytes"
        );
        Ok(Self(bytes))
    }
}
impl From<SystemData> for Vec<u8> {
    fn from(data: SystemData) -> Self {
        data.0
    }
}
impl Default for SystemData {
    fn default() -> Self {
        //437088: first18 native dwords, then two20-byte CP932 names.
        //The remaining3200 bytes initialize to zero in the original table.
        const WORDS: [u32; 28] = [
            0, 0, 8, 8, 1, 1, 1, 1, 0, 1, 0, 25, 1, 25, 1, 29, 1, 1, 3767793549, 0, 0, 0, 0,
            1452564619, 0, 0, 0, 0,
        ];
        let mut data = Self(vec![0; SYSTEM_BYTES]);
        for (index, word) in WORDS.into_iter().enumerate() {
            data.0[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        data
    }
}
impl SystemData {
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }
    pub fn byte(&self, bank: i16, index: i16) -> Result<Option<u8>> {
        let Some((base, length)) = byte_bank(bank) else {
            return Ok(None);
        };
        ensure!(
            index >= 0 && (index as usize) < length,
            "DC2 native preference byte index"
        );
        Ok(Some(
            *self
                .0
                .get(base + index as usize)
                .context("DC2 preference extent")?,
        ))
    }
    pub fn set_byte(&mut self, bank: i16, index: i16, value: i32) -> Result<bool> {
        let Some((base, length)) = byte_bank(bank) else {
            return Ok(false);
        };
        ensure!(
            index >= 0 && (index as usize) < length,
            "DC2 native preference byte index"
        );
        *self
            .0
            .get_mut(base + index as usize)
            .context("DC2 preference extent")? = value as u8;
        Ok(true)
    }
    pub fn prefix(&self, index: i16) -> Result<Option<i32>> {
        let Some(offset) = prefix(index) else {
            return Ok(None);
        };
        Ok(Some(i32::from_le_bytes(
            self.0[offset..offset + 4].try_into()?,
        )))
    }
    ///Only native persisted storage is changed here; setter callers own
    ///41A5C0's music/window side effects and100's mandatory save receipt.
    pub fn set_prefix(&mut self, index: i16, value: i32) -> Result<bool> {
        let Some(offset) = prefix(index) else {
            return Ok(false);
        };
        self.0[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        Ok(true)
    }
    pub fn names(&self) -> Result<[String; 2]> {
        let decode = |at| -> Result<String> {
            let bytes = &self.0[at..at + 20];
            let end = bytes
                .iter()
                .position(|byte| *byte == 0)
                .context("DC2 preference name terminator")?;
            let (text, error) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(&bytes[..end]);
            ensure!(!error, "DC2 preference name CP932");
            Ok(text.into_owned())
        };
        Ok([decode(72)?, decode(92)?])
    }
}
fn byte_bank(bank: i16) -> Option<(usize, usize)> {
    Some(match bank {
        100 => (112, 100),
        97 => (212, 1500),
        98 => (1712, 1500),
        95 => (3212, 100),
        _ => return None,
    })
}
fn prefix(index: i16) -> Option<usize> {
    Some(match index {
        5 => 64,
        6 => 68,
        7 => 44,
        8 => 52,
        9 => 60,
        10 => 16,
        11 => 8,
        12 => 24,
        13 => 4,
        14 => 36,
        15 => 48,
        16 => 12,
        17 => 32,
        18 => 20,
        20 => 28,
        _ => return None,
    })
}

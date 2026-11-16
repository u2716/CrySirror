//! DC3WY system.dat ABI, recovered from40B500/40D320/40D1B0.
//! Two discontiguous native buffers are serialized consecutively: 17704
//! preference bytes and2040 character-source bytes fromnative344988.
//! DC4PH's19328-byte preference block and356324-byte slots are unrelated.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

pub const PREFERENCES_BYTES: usize = 0x4528;
pub const SOURCE_BYTES: usize = 0x7f8;
pub const SYSTEM_BYTES: usize = PREFERENCES_BYTES + SOURCE_BYTES;
pub const SLOT_BYTES: usize = 0x12c40; //40B6A0/40B960, not a JSON snapshot
const DIRECTORY: usize = 2612;
const POOL: usize = 15616;
const SCENES: usize = 650;
const RANGES: usize = 500;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(try_from = "Vec<u8>", into = "Vec<u8>")]
pub struct SystemData(Vec<u8>);
impl TryFrom<Vec<u8>> for SystemData {
    type Error = anyhow::Error;
    fn try_from(bytes: Vec<u8>) -> Result<Self> {
        ensure!(
            bytes.len() == SYSTEM_BYTES,
            "DC3WY system.dat requires exactly {SYSTEM_BYTES} bytes"
        );
        Ok(Self(bytes))
    }
}
impl From<SystemData> for Vec<u8> {
    fn from(value: SystemData) -> Self {
        value.0
    }
}
impl Default for SystemData {
    fn default() -> Self {
        // Numeric defaults at499098. The remainder of its17704-byte table
        // is zero apart from the two CP932 name fields at160 and176.
        const DEFAULTS: [i32; 40] = [
            0, 3, 3, 3, 0, 1, 50, 7, 1, 1, 1, 6, 0, 1, 0, 65, 1, 80, 1, 100, 1, 0, 80, 0, 1, 1, 0,
            0, 0, 85, 0, 1, 100, 1, 1, 0, 1, 1, 5, 0,
        ];
        let mut data = Self(vec![0; SYSTEM_BYTES]);
        for (index, value) in DEFAULTS.into_iter().enumerate() {
            data.0[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        for (offset, bytes) in [
            (160, [0x92, 0xa9, 0x91, 0x71]),
            (176, [0x90, 0xb4, 0x97, 0xb2]),
        ] {
            data.0[offset..offset + 4].copy_from_slice(&bytes);
        }
        for (index, value) in [(86, 0i32), (87, 5), (88, 2), (89, 4)] {
            data.0[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        data
    }
}
impl SystemData {
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }
    pub fn prefix_value(&self, selector: i16) -> Result<Option<i32>> {
        if selector == 217 {
            return Ok(Some(self.checkbox(self.dword(4425)? as i16)?));
        }
        if selector == 11 {
            return Ok(Some(99i32.wrapping_sub(self.dword(6)?)));
        }
        Ok(prefix_index(selector)
            .map(|index| self.dword(index))
            .transpose()?)
    }
    pub fn set_prefix_value(&mut self, selector: i16, value: i32) -> Result<bool> {
        if selector == 217 {
            self.set_checkbox(self.dword(4425)? as i16, value)?;
            return Ok(true);
        }
        if selector == 11 {
            self.put_dword(6, 99i32.wrapping_sub(value))?;
            return Ok(true);
        }
        let Some(index) = prefix_index(selector) else {
            return Ok(false);
        };
        self.put_dword(index, value)?;
        Ok(true)
    }
    fn dword(&self, index: usize) -> Result<i32> {
        let at = index
            .checked_mul(4)
            .context("DC3WY preference index overflow")?;
        Ok(i32::from_le_bytes(
            self.0
                .get(at..at + 4)
                .context("DC3WY preference index")?
                .try_into()?,
        ))
    }
    pub fn checkbox(&self, index: i16) -> Result<i32> {
        ensure!((0..20).contains(&index), "DC3WY checkbox index");
        self.dword(4404 + index as usize)
    }
    pub fn set_checkbox(&mut self, index: i16, value: i32) -> Result<()> {
        ensure!((0..20).contains(&index), "DC3WY checkbox index");
        self.put_dword(4404 + index as usize, value)
    }
    fn put_dword(&mut self, index: usize, value: i32) -> Result<()> {
        let at = index
            .checked_mul(4)
            .context("DC3WY preference index overflow")?;
        self.0
            .get_mut(at..at + 4)
            .context("DC3WY preference index")?
            .copy_from_slice(&value.to_le_bytes());
        Ok(())
    }
    ///40D160 clears the original51 source records and initializes its
    ///fallback record50. Registration count is live VM state, not file data.
    pub fn reset_sources(&mut self) {
        self.0[PREFERENCES_BYTES..].fill(0);
        self.0[PREFERENCES_BYTES + 2000..PREFERENCES_BYTES + 2004]
            .copy_from_slice(&1i32.to_le_bytes());
        self.0[PREFERENCES_BYTES + 2004..PREFERENCES_BYTES + 2008]
            .copy_from_slice(&100i32.to_le_bytes());
    }
    pub fn register_source(&mut self, name: &[u8], count: &mut u8) -> Result<()> {
        ensure!(
            name.len() < 32 && !name.contains(&0),
            "DC3WY voice-source prefix extent"
        );
        ensure!(*count <= 50, "DC3WY voice-source count limit");
        for index in 0..usize::from(*count) {
            let at = PREFERENCES_BYTES + index * 40 + 8;
            if self.0[at..at + name.len()].eq_ignore_ascii_case(name) {
                return Ok(());
            }
        }
        ensure!(*count < 50, "DC3WY voice-source table exhausted");
        let at = PREFERENCES_BYTES + usize::from(*count) * 40;
        self.0[at..at + 40].fill(0);
        self.0[at..at + 4].copy_from_slice(&1i32.to_le_bytes());
        self.0[at + 4..at + 8].copy_from_slice(&100i32.to_le_bytes());
        self.0[at + 8..at + 8 + name.len()].copy_from_slice(name);
        *count += 1;
        Ok(())
    }
    pub fn source_value(&self, bank: i16, index: i16, count: u8) -> Result<i32> {
        ensure!(matches!(bank, 94 | 95), "DC3WY voice-source bank");
        if index != 50 && index >= i16::from(count) {
            return Ok(-1);
        }
        ensure!((0..=50).contains(&index), "DC3WY voice-source index");
        let at = PREFERENCES_BYTES + index as usize * 40 + if bank == 94 { 4 } else { 0 };
        Ok(i32::from_le_bytes(self.0[at..at + 4].try_into()?))
    }
    ///40CD20/40CDF0 select the CP932 suffix after the last single-byte
    ///underscore, then compare registered prefixes without ASCII case.
    pub fn voice_policy(&self, name: &str, count: u8) -> Result<(bool, i32)> {
        ensure!(count <= 50, "DC3WY voice-source count");
        let (encoded, _, errors) = encoding_rs::SHIFT_JIS.encode(name);
        ensure!(!errors, "DC3WY voice resource encoding");
        let mut cursor = 0;
        let mut suffix = 0;
        while cursor < encoded.len() {
            let value = encoded[cursor];
            if (0x81..=0x9f).contains(&value) || (0xe0..=0xfc).contains(&value) {
                ensure!(
                    cursor + 1 < encoded.len(),
                    "DC3WY truncated voice CP932 pair"
                );
                cursor += 2;
            } else {
                if value == b'_' {
                    suffix = cursor + 1
                }
                cursor += 1;
            }
        }
        let tail = &encoded[suffix..];
        ensure!(tail.len() < 32, "DC3WY voice-source suffix extent");
        let mut slot = 50;
        for index in 0..count {
            let at = PREFERENCES_BYTES + usize::from(index) * 40 + 8;
            let raw = &self.0[at..at + 32];
            let end = raw
                .iter()
                .position(|value| *value == 0)
                .context("DC3WY unterminated voice-source prefix")?;
            if tail
                .get(..end)
                .is_some_and(|candidate| candidate.eq_ignore_ascii_case(&raw[..end]))
            {
                slot = i16::from(index);
                break;
            }
        }
        let enabled = self.source_value(95, slot, count)? != 0;
        let volume = self.source_value(94, slot, count)?;
        let percent = self
            .prefix_value(9)?
            .context("DC3WY voice gain preference")?
            .wrapping_mul(volume)
            / 100;
        Ok((enabled, percent))
    }
    pub fn set_source_value(&mut self, bank: i16, index: i16, value: i32, count: u8) -> Result<()> {
        ensure!(matches!(bank, 94 | 95), "DC3WY voice-source bank");
        if index != 50 && index >= i16::from(count) {
            return Ok(());
        }
        ensure!((0..=50).contains(&index), "DC3WY voice-source index");
        let at = PREFERENCES_BYTES + index as usize * 40 + if bank == 94 { 4 } else { 0 };
        let value = if bank == 94 { value.min(100) } else { value };
        self.0[at..at + 4].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }
    pub fn reset_preferences(&mut self) {
        self.0[..PREFERENCES_BYTES].copy_from_slice(&Self::default().0[..PREFERENCES_BYTES]);
    }
    pub fn names(&self) -> Result<[String; 2]> {
        [160, 176]
            .map(|offset| {
                let raw = &self.0[offset..offset + 16];
                let end = raw
                    .iter()
                    .position(|value| *value == 0)
                    .context("unterminated DC3WY protagonist name")?;
                let (name, errors) =
                    encoding_rs::SHIFT_JIS.decode_without_bom_handling(&raw[..end]);
                ensure!(!errors, "invalid DC3WY protagonist name encoding");
                Ok(name.into_owned())
            })
            .into_iter()
            .collect::<Result<Vec<_>>>()?
            .try_into()
            .map_err(|_| anyhow::anyhow!("DC3WY protagonist name count"))
    }
    pub fn set_names(&mut self, names: &[String; 2]) -> Result<()> {
        let mut encoded = Vec::new();
        for name in names {
            ensure!(!name.contains('\0'), "DC3WY protagonist name contains NUL");
            let (bytes, _, errors) = encoding_rs::SHIFT_JIS.encode(name);
            ensure!(
                !errors && bytes.len() < 16,
                "DC3WY protagonist name CP932 extent"
            );
            encoded.push(bytes.into_owned());
        }
        for (offset, bytes) in [160, 176].into_iter().zip(encoded) {
            self.0[offset..offset + 16].fill(0);
            self.0[offset..offset + bytes.len()].copy_from_slice(&bytes);
        }
        Ok(())
    }
    ///40BF90/40C4B0's resource-byte banks. Bounds follow the native arrays,
    ///not the installed game's shorter NAMES count. Other native banks need
    ///their live/save owners and are deliberately not folded into this file.
    pub fn byte(&self, bank: i16, index: i16) -> Result<Option<u8>> {
        Ok(byte_offset(bank, index)?.map(|offset| self.0[offset]))
    }
    pub fn set_byte(&mut self, bank: i16, index: i16, value: i32) -> Result<bool> {
        let Some(offset) = byte_offset(bank, index)? else {
            return Ok(false);
        };
        self.0[offset] = value as u8;
        Ok(true)
    }
    pub fn label_is_read(&self, scene: usize, label: u16) -> Result<bool> {
        Ok(self
            .ranges(scene)?
            .iter()
            .any(|(first, last)| *first <= label && label <= *last))
    }
    fn ranges(&self, scene: usize) -> Result<Vec<(u16, u16)>> {
        ensure!(scene < SCENES, "DC3WY read-label scene index");
        let record = DIRECTORY + scene * 20;
        let pointer = u32::from_le_bytes(self.0[record..record + 4].try_into()?) as usize;
        ensure!(pointer < RANGES, "DC3WY read-label pool pointer");
        let (at, count) = if pointer == 0 {
            (record + 4, 4)
        } else {
            (POOL + pointer * 4, RANGES - pointer)
        };
        let mut result = Vec::new();
        for index in 0..count {
            let first = u16::from_le_bytes(self.0[at + index * 4..at + index * 4 + 2].try_into()?);
            if first == 0 {
                break;
            }
            let last =
                u16::from_le_bytes(self.0[at + index * 4 + 2..at + index * 4 + 4].try_into()?);
            ensure!(last >= first, "DC3WY inverted read-label interval");
            result.push((first, last));
        }
        Ok(result)
    }
    ///402870/42F7E0 use four inline intervals or a shared500-entry pool.
    ///Preserve unrecognized preference/source bytes; only repack that native
    ///directory and its allocator word. Validate capacity before committing.
    pub fn mark_read(&mut self, scene: usize, first: u16, last: u16) -> Result<()> {
        ensure!(
            scene < SCENES && first > 0 && first <= last,
            "DC3WY read-label interval"
        );
        let mut records = (0..SCENES)
            .map(|scene| self.ranges(scene))
            .collect::<Result<Vec<_>>>()?;
        records[scene].push((first, last));
        records[scene].sort_unstable();
        let mut merged: Vec<(u16, u16)> = Vec::new();
        for (first, last) in records[scene].drain(..) {
            if let Some(previous) = merged
                .last_mut()
                .filter(|previous| u32::from(first) <= u32::from(previous.1) + 1)
            {
                previous.1 = previous.1.max(last)
            } else {
                merged.push((first, last))
            }
        }
        records[scene] = merged;
        let needed: usize = records
            .iter()
            .filter(|ranges| ranges.len() > 4)
            .map(|ranges| ranges.len() + 1)
            .sum();
        ensure!(needed < RANGES, "DC3WY read-label pool exhausted");
        let mut bytes = self.0.clone();
        bytes[DIRECTORY..POOL + RANGES * 4].fill(0);
        let mut next = 1;
        for (scene, ranges) in records.into_iter().enumerate() {
            let record = DIRECTORY + scene * 20;
            let at = if ranges.len() <= 4 {
                record + 4
            } else {
                bytes[record..record + 4].copy_from_slice(&(next as u32).to_le_bytes());
                let at = POOL + next * 4;
                next += ranges.len() + 1;
                at
            };
            for (index, (first, last)) in ranges.into_iter().enumerate() {
                bytes[at + index * 4..at + index * 4 + 2].copy_from_slice(&first.to_le_bytes());
                bytes[at + index * 4 + 2..at + index * 4 + 4].copy_from_slice(&last.to_le_bytes());
            }
        }
        bytes[POOL - 4..POOL]
            .copy_from_slice(&(if needed == 0 { 0u32 } else { next as u32 }).to_le_bytes());
        self.0 = bytes;
        Ok(())
    }
}
fn byte_offset(bank: i16, index: i16) -> Result<Option<usize>> {
    let (base, count) = match bank {
        100 => (192, 150),
        96 => (360, 400),
        97 => (760, 1200),
        98 => (1960, 650),
        _ => return Ok(None),
    };
    ensure!(
        (0..count).contains(&index),
        "DC3WY byte bank {bank} index {index}"
    );
    Ok(Some(base + index as usize))
}

fn prefix_index(selector: i16) -> Option<usize> {
    Some(match selector {
        5 => 18,
        7 => 15,
        8 => 17,
        9 => 19,
        10 => 9,
        12 => 11,
        13 => 4,
        14 => 13,
        15 => 16,
        16 => 7,
        17 => 12,
        18 => 10,
        26 => 1,
        27 => 2,
        28 => 5,
        29 => 3,
        30 => 8,
        190 => 30,
        191 => 39,
        194 => 38,
        196 => 89,
        197 => 88,
        198 => 87,
        199 => 86,
        200 => 37,
        201 => 36,
        202 => 35,
        203 => 31,
        205 => 33,
        206 => 29,
        207 => 22,
        208 => 23,
        209 => 28,
        210 => 27,
        211 => 26,
        212 => 25,
        213 => 24,
        214 => 21,
        218 => 4425,
        221 => 4424,
        225 => 32,
        227 => 34,
        237 => 20,
        _ => return None,
    })
}

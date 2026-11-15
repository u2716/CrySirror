// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! commonSystemFile.h/.cpp source ABI extensions. Source's GetVarType and
//! SYSTEMNNN_VER2 are independent choices; neither is inferred from a save.
//! The installed Mugen2 bootstrap still selects its reviewed ten-record ABI.
use super::*;
pub(super) const RECORDS: [(u32, usize, &[u8]); 6] = [
    (13, 6432, b"OKIKAE         "),
    (14, 4128, b"ACHIEVEMENT    "),
    (15, 4128, b"TERM           "),
    (16, 65568, b"CHARAVOICEFLAG "),
    (17, 1056, b"SYSTEMFLAG2    "),
    (18, 4128, b"SELECTMESSAGE  "),
];
impl NativeSystemFile {
    /// Constructor data for the published source profile, independently of
    /// executable identification. This does not authorize a new native ABI.
    pub(crate) fn defaults_source(
        limits: SystemLimits,
        stamp: SystemStamp,
        names: &SystemNames,
        variables: VariableProfile,
        version2: bool,
    ) -> Result<Self> {
        let mut out = Self::defaults(limits, stamp, names)?;
        out.layout = SystemFileLayout::Source {
            variables,
            version2,
        };
        let specs = out.layout.records();
        if variables == VariableProfile::Expanded2200 {
            out.records[3].words.resize(4128 / 4, 0);
            out.records[3].set_word(0, 4128)?;
        }
        for (code, size, label) in &specs[10..] {
            let mut record = Record {
                words: vec![0; size / 4],
            };
            record.set_word(0, *size as i32)?;
            record.set_word(4, *code as i32)?;
            record.set_string(16, 16, label)?;
            out.records.push(record);
        }
        // CreateInitData sets15 even though Save subsequently writes14 or16.
        out.records[0].set_word(32, 15)?;
        out.records[1].set_word(220, 50)?; // autoSpeedSlider
        out.records[1].set_word(1968, 90)?; // totalVolume
        out.records[1].set_word(1972, 1)?; // totalVolumeSwitch
        Ok(out)
    }
    fn require_source(&self) -> Result<()> {
        ensure!(
            matches!(self.layout, SystemFileLayout::Source { .. }),
            "NNN SystemFile source extension absent in this native ABI"
        );
        Ok(())
    }
    pub(crate) fn source_version2(&self) -> bool {
        matches!(self.layout, SystemFileLayout::Source { version2: true, .. })
    }
    /// SYSTEMFLAG2 is record17, not an offset in SYSTEM DATA. VERSION1 keeps
    /// the zero-initialized RAM block but intentionally does not serialize it.
    pub(crate) fn flag2_word(&self, index: usize) -> Result<i32> {
        self.require_source()?;
        ensure!(index < 256, "NNN SYSTEMFLAG2 word bounds");
        self.records[14].word(32 + index * 4)
    }
    pub(crate) fn set_flag2_word(&mut self, index: usize, value: i32) -> Result<bool> {
        self.require_source()?;
        ensure!(index < 256, "NNN SYSTEMFLAG2 word bounds");
        self.set_source_word(14, 32 + index * 4, value)
    }
    pub(crate) fn gore_flag(&self) -> Result<i32> {
        self.flag2_word(9)
    }
    pub(crate) fn set_gore_flag(&mut self, value: i32) -> Result<bool> {
        self.set_flag2_word(9, value)
    }
    fn set_source_word(&mut self, record: usize, offset: usize, value: i32) -> Result<bool> {
        if self.records[record].word(offset)? == value {
            return Ok(false);
        }
        self.changed()?;
        self.records[record].set_word(offset, value)?;
        Ok(true)
    }
    pub(crate) fn okikae(&self, n: i32) -> Result<Option<Vec<u8>>> {
        self.require_source()?;
        if !(0..100).contains(&n) {
            return Ok(None);
        }
        Ok(Some(self.records[10].string(32 + n as usize * 64, 64)?))
    }
    pub(crate) fn set_okikae(&mut self, n: i32, text: Option<&[u8]>) -> Result<bool> {
        self.require_source()?;
        let Some(text) = text else {
            return Ok(false);
        };
        if !(0..100).contains(&n) {
            return Ok(false);
        }
        let text = &text[..text
            .iter()
            .position(|b| *b == 0)
            .unwrap_or(text.len())
            .min(32)];
        let mut record = self.records[10].clone();
        // Original writes two trailing NUL bytes, retaining the other bytes
        // of this64-byte slot. A shorter replacement is not a slot clear.
        let offset = 32 + n as usize * 64;
        record.copy_string(offset, 64, text)?;
        ensure!(
            integer_fields::set(&mut record.words, 8, (offset + text.len() + 1) as i64, 0),
            "NNN OKIKAE second terminator bounds"
        );
        if record.words == self.records[10].words {
            return Ok(false);
        }
        self.changed()?;
        self.records[10] = record;
        Ok(true)
    }
    pub(crate) fn achievement(&self, n: i32) -> Result<i32> {
        self.require_source()?;
        if !(0..1024).contains(&n) {
            return Ok(0);
        }
        self.records[11].word(32 + n as usize * 4)
    }
    pub(crate) fn set_achievement(&mut self, n: i32, value: i32) -> Result<bool> {
        self.require_source()?;
        if !(0..1024).contains(&n) {
            return Ok(false);
        }
        self.set_source_word(11, 32 + n as usize * 4, value)
    }
    pub(crate) fn term_flag(&self, n: i32, look: bool) -> Result<bool> {
        self.require_source()?;
        if !(0..1024).contains(&n) {
            return Ok(false);
        }
        Ok(self.records[12].word(32 + n as usize * 4)? & (if look { 2 } else { 1 }) != 0)
    }
    pub(crate) fn set_term_flag(&mut self, n: i32, look: bool, enabled: bool) -> Result<bool> {
        self.require_source()?;
        if !(0..1024).contains(&n) {
            return Ok(false);
        }
        let offset = 32 + n as usize * 4;
        let bit = if look { 2 } else { 1 };
        let old = self.records[12].word(offset)?;
        self.set_source_word(12, offset, if enabled { old | bit } else { old & !bit })
    }
    pub(crate) fn character_voice_flag(&self, flag: i32) -> Result<bool> {
        self.require_source()?;
        self.flag(13, (0..65536 * 8).contains(&flag).then_some(flag as usize))
    }
    pub(crate) fn set_character_voice_flag(&mut self, flag: i32, enabled: bool) -> Result<bool> {
        self.require_source()?;
        if !(0..65536 * 8).contains(&flag) {
            return Ok(false);
        }
        let offset = 32 + (flag as usize / 32) * 4;
        let bit = 1i32.wrapping_shl((flag & 31) as u32);
        let old = self.records[13].word(offset)?;
        self.set_source_word(13, offset, if enabled { old | bit } else { old & !bit })
    }
    pub(crate) fn selection_flag(&self, serial: i32, choice: i32) -> Result<bool> {
        self.require_source()?;
        if !(0..1024).contains(&serial) {
            return Ok(false);
        }
        // Original masks choice, including negative values, to an x86 bit.
        Ok(self.records[15].word(32 + serial as usize * 4)?
            & 1i32.wrapping_shl((choice & 31) as u32)
            != 0)
    }
    pub(crate) fn set_selection_flag(
        &mut self,
        serial: i32,
        choice: i32,
        enabled: bool,
    ) -> Result<bool> {
        self.require_source()?;
        if !(0..1024).contains(&serial) {
            return Ok(false);
        }
        let offset = 32 + serial as usize * 4;
        let bit = 1i32.wrapping_shl((choice & 31) as u32);
        let old = self.records[15].word(offset)?;
        self.set_source_word(15, offset, if enabled { old | bit } else { old & !bit })
    }
    /// Original clear helpers retain record headers. Flag2's helper clears
    /// only lastSaveFileNumber and pad[], leaving its other fields untouched.
    pub(crate) fn clear_source_record(&mut self, code: u32) -> Result<bool> {
        self.require_source()?;
        let record = match code {
            5 => 2,
            6 => 3,
            7 => 4,
            8 => 5,
            9 => 6,
            10 => 7,
            11 => 8,
            12 => 9,
            13 => 10,
            14 => 11,
            15 => 12,
            16 => 13,
            17 => 14,
            18 => 15,
            _ => anyhow::bail!("NNN unsupported SystemFile clear record {code}"),
        };
        let mut candidate = self.records[record].clone();
        if code == 17 {
            candidate.words[8] = 0;
            candidate.words[20..].fill(0);
        } else {
            candidate.words[8..].fill(0);
        }
        if candidate.words == self.records[record].words {
            return Ok(false);
        }
        self.changed()?;
        self.records[record] = candidate;
        Ok(true)
    }
}

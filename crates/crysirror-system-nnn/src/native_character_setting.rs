// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Private modal-control identities, distinct from native SetSystemParam IDs.
//! Packing the finite source range keeps UI value conversion stable over retries.
use anyhow::{Result, ensure};
const VOLUME: i32 = 0x40000000;
const ENABLED: i32 = 0x50000000;
#[derive(Clone, Copy, Debug)]
pub(crate) struct Setting {
    pub slot: usize,
    pub minimum: i32,
    pub maximum: i32,
    pub toggle: bool,
}
impl Setting {
    pub(crate) fn id(self) -> Result<i32> {
        ensure!(
            self.slot < 256
                && (-15..=15).contains(&self.minimum)
                && self.maximum >= self.minimum
                && self.maximum <= 15,
            "NNN character setting domain"
        );
        Ok((if self.toggle { ENABLED } else { VOLUME })
            | ((self.slot as i32) << 10)
            | ((self.minimum + 15) << 5)
            | (self.maximum + 15))
    }
    pub(crate) fn decode(id: i32) -> Option<Self> {
        if id < 0 {
            return None;
        }
        let tag = id & 0x7ffc0000;
        if !matches!(tag, VOLUME | ENABLED) {
            return None;
        }
        let minimum = ((id >> 5) & 31) - 15;
        let maximum = (id & 31) - 15;
        if minimum > maximum || maximum > 15 || minimum > 15 {
            return None;
        }
        Some(Self {
            slot: ((id >> 10) & 255) as usize,
            minimum,
            maximum,
            toggle: tag == ENABLED,
        })
    }
    pub(crate) fn steps(self) -> i32 {
        if self.toggle {
            1
        } else {
            (self.maximum - self.minimum).max(1)
        }
    }
    pub(crate) fn label(self) -> String {
        format!(
            "Character {} voice{}",
            self.slot + 1,
            if self.toggle { " enabled" } else { " volume" }
        )
    }
}
pub(crate) fn voice_enabled(
    file: &crate::native_system_file::NativeSystemFile,
    slot: usize,
) -> Result<bool> {
    ensure!(slot < 256, "NNN character setting slot");
    Ok(file.system_word(1536 + (slot & 31) * 4)? & (1i32.wrapping_shl((slot / 32) as u32)) == 0)
}
pub(crate) fn set_voice_enabled(
    file: &mut crate::native_system_file::NativeSystemFile,
    slot: usize,
    enabled: bool,
) -> Result<()> {
    ensure!(slot < 256, "NNN character setting slot");
    let at = 1536 + (slot & 31) * 4;
    let mask = 1i32.wrapping_shl((slot / 32) as u32);
    let value = file.system_word(at)?;
    file.set_setting_word(at, if enabled { value & !mask } else { value | mask })
}

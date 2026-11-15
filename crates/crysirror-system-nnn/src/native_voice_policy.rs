// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CGameCallBack::VoiceToCharaNumber/CheckVoiceOffByName and CommonConfig's
//! optional character-to-button mapping. Original flags remain in SystemFile;
//! common integer-field queries read them without maintaining another bitmap.
use crate::{config::Names, native_system_file::NativeSystemFile};
use anyhow::{Result, ensure};
#[derive(Clone, Debug)]
pub(crate) struct VoicePolicy {
    prefixes: Vec<[u8; 2]>,
    button_groups: Vec<Vec<i32>>,
    total_switch: bool,
}
impl VoicePolicy {
    pub(crate) fn button_voice_used(&self,file:&NativeSystemFile,index:usize)->Result<bool>{
        if let Some(group)=self.button_groups.get(index){for character in group{if file.voice(*character)?{return Ok(true);}}Ok(false)}else{file.voice(index as i32+1)}
    }

    pub(crate) fn volume_delta(
        &self,
        name: &[u8],
        file: &NativeSystemFile,
        scale: i32,
        config_present: bool,
    ) -> Result<i32> {
        let prefix = [
            name.first().copied().unwrap_or(0) | 0x20,
            name.get(1).copied().unwrap_or(0) | 0x20,
        ];
        let Some(character) = self
            .prefixes
            .iter()
            .enumerate()
            .skip(1)
            .find_map(|(index, value)| (*value == prefix).then_some(index as i32))
        else {
            return Ok(0);
        };
        let button = if config_present {
            self.button_groups
                .iter()
                .enumerate()
                .find_map(|(index, group)| group.contains(&character).then_some(index as i32 + 1))
                .unwrap_or(character)
                .wrapping_sub(1)
        } else {
            character
        };
        Ok(character_slider(file, button)?.wrapping_mul(scale))
    }
    pub(crate) fn read(character_names: &Names, config: &Names, game: &Names) -> Result<Self> {
        ensure!(
            character_names.tokens.len() <= 8192 && character_names.tokens.len() % 2 == 0,
            "NNN voice character name-table bounds"
        );
        let prefixes = character_names
            .tokens
            .iter()
            .step_by(2)
            .map(|name| {
                // The native loader expects at least two bytes. A shorter logical
                // name formerly read through its terminator; use a zero-padded
                // prefix rather than reading unrelated memory.
                [
                    name.first().copied().unwrap_or(0) | 0x20,
                    name.get(1).copied().unwrap_or(0) | 0x20,
                ]
            })
            .collect();
        let mut button_groups = Vec::new();
        let enabled = config.integer(b"voiceCutButtonFlag")?.unwrap_or(1) != 0;
        let count = if enabled {
            config.integer(b"voiceCutButtonNumber")?.unwrap_or(0)
        } else {
            0
        };
        ensure!((0..=256).contains(&count), "NNN voice-cut button count");
        if count > 0 && config.integer(b"voiceCutButtonCustomFlag")?.unwrap_or(0) != 0 {
            let mut character = 1i32;
            let mut total = 0usize;
            for index in 0..count {
                let number = config
                    .integer(format!("voiceOffNumberByButton{}", index + 1).as_bytes())?
                    .unwrap_or(1);
                ensure!((0..=4096).contains(&number), "NNN voice-cut group size");
                total = total
                    .checked_add(number as usize)
                    .ok_or_else(|| anyhow::anyhow!("NNN voice-cut extent overflow"))?;
                ensure!(total <= 16384, "NNN voice-cut mapping budget");
                let mut group = Vec::with_capacity(number as usize);
                for member in 0..number {
                    character = config
                        .integer(
                            format!("voiceOffButton{}chara{}", index + 1, member + 1).as_bytes(),
                        )?
                        .unwrap_or(character);
                    group.push(character);
                    character = character.wrapping_add(1);
                }
                button_groups.push(group);
            }
        }
        Ok(Self {
            prefixes,
            button_groups,
            total_switch: game.integer(b"totalVolumeUseFlag")?.unwrap_or(0) != 0,
        })
    }
    pub(crate) fn total_off(&self, file: &NativeSystemFile) -> Result<bool> {
        Ok(self.total_switch && file.system_word(1972)? == 0)
    }
    pub(crate) fn allows(&self, name: &[u8], file: &NativeSystemFile) -> Result<bool> {
        self.allows_with_config(name, file, true)
    }
    pub(crate) fn allows_with_config(
        &self,
        name: &[u8],
        file: &NativeSystemFile,
        config_present: bool,
    ) -> Result<bool> {
        let prefix = [
            name.first().copied().unwrap_or(0) | 0x20,
            name.get(1).copied().unwrap_or(0) | 0x20,
        ];
        // VoiceToCharaNumber deliberately skips pair0 and returns the first
        // two-byte match, using bitwise OR rather than Unicode case folding.
        let Some(character) = self
            .prefixes
            .iter()
            .enumerate()
            .skip(1)
            .find_map(|(index, value)| (*value == prefix).then_some(index as i32))
        else {
            return Ok(true);
        };
        let button = if config_present {
            self.button_groups
                .iter()
                .enumerate()
                .find_map(|(index, group)| group.contains(&character).then_some(index as i32 + 1))
                .unwrap_or(character)
                .wrapping_sub(1)
        } else {
            character
        };
        let row = (button & 31) as usize;
        let shift = if button >= 32 {
            (button / 32) as u32 & 31
        } else {
            0
        };
        // Native CheckVoiceFlag uses transposed storage: word(n&31), bit(n/32).
        // The x86 shift masks its count. Keep that explicit, bounded word
        // selection even for malformed custom mappings or button-1.
        let words = [i64::from(file.system_word(1536 + row * 4)?)];
        let off = crysirror_core::integer_fields::get(&words, 1, i64::from(shift));
        Ok(off == Some(0))
    }
}

/// SYSTEM DATA1424..1471 stores twelve native words, each holding SIX signed
/// five-bit sliders and two unused high bits (not a continuous5-bit bitstream).
/// This is original save packing policy, independent of decoded/mixed audio.
pub(crate) fn character_slider(file: &NativeSystemFile, character: i32) -> Result<i32> {
    if !(0..72).contains(&character) {
        return Ok(0);
    }
    let word = file.system_word(1424 + (character as usize / 6) * 4)?;
    let value = (word >> ((character % 6) * 5)) & 31;
    Ok(if value & 16 != 0 { value | !31 } else { value })
}
pub(crate) fn set_character_slider(
    file: &mut NativeSystemFile,
    character: i32,
    value: i32,
) -> Result<()> {
    if !(0..72).contains(&character) {
        return Ok(());
    }
    let at = 1424 + (character as usize / 6) * 4;
    let shift = (character % 6) * 5;
    let old = file.system_word(at)?;
    let next = (old & !(31 << shift)) | ((value.wrapping_shl(shift as u32)) & (31 << shift));
    file.set_setting_word(at, next)
}

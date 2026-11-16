// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CommonConfig character-button/slider geometry, filename and sound variants.
//! Init's used-character gate is supplied by the existing VoicePolicy; filename
//! lookups preserve the same client roots, banks and original atlas states.
use super::{check, n, variable};
use crate::{
    config::Names, native_bootstrap::NativeConfiguration, native_radio_group::RadioButtonSpec,
    native_resources::ImageLookup,
};
use anyhow::{Context, Result, ensure};
#[derive(Clone, Debug)]
pub(crate) struct Slider {
    pub file: Option<Vec<u8>>,
    pub origin: [i32; 2],
    pub size: [i32; 2],
    pub minimum: i32,
    pub maximum: i32,
    pub preview_sound: i32,
}
#[derive(Clone, Debug)]
pub(crate) struct Character {
    pub page: i32,
    pub buttons: [RadioButtonSpec; 2],
    pub only_used: bool,
    pub filename_variable: Option<u16>,
    pub sound_variable: Option<u16>,
    sound_override: [[i32; 4]; 2],
    pub slider: Option<Slider>,
}
impl Character {
    pub(crate) fn sounds(&self, state: usize, alternate: bool) -> Result<[i32; 4]> {
        ensure!(state < 2, "NNN Config character state");
        let base = self.buttons[state].raw_sounds;
        Ok(if alternate {
            std::array::from_fn(|i| {
                if self.sound_override[state][i] > 0 {
                    self.sound_override[state][i]
                } else {
                    base[i]
                }
            })
        } else {
            base
        })
    }
    pub(crate) fn native_name(&self,state:usize,variant:i32)->Result<Option<Vec<u8>>>{
        ensure!(state<2,"NNN Config character state");
        let Some(base)=&self.buttons[state].button.filename else{return Ok(None)};
        let mut name=[b"sys\\".as_slice(),base].concat();
        if variant>0{name.extend(format!("_{variant}").as_bytes());}
        ensure!(name.len()<=254,"NNN Config character native filename bound");
        Ok(Some(name))
    }
    pub(crate) fn image(&self, state: usize, variant: i32) -> Result<Option<ImageLookup>> {
        ensure!(state < 2, "NNN Config character state");
        let Some(mut lookup) = self.buttons[state].button.source.clone() else {
            return Ok(None);
        };
        if variant <= 0 {
            return Ok(Some(lookup));
        }
        let suffix = format!("_{variant}.dwq");
        let append = |name: &str| -> Result<String> {
            Ok(format!(
                "{}{}",
                name.strip_suffix(".dwq")
                    .context("NNN Config generated picture extension absent")?,
                suffix
            ))
        };
        lookup.loose = append(&lookup.loose)?;
        for entry in &mut lookup.packed {
            entry.member = append(&entry.member)?;
        }
        Ok(Some(lookup))
    }
    pub(crate) fn preview(&self, state: usize, alternate: bool) -> Result<Option<(i32, i32)>> {
        ensure!(state < 2, "NNN Config character state");
        let Some(slider) = &self.slider else {
            return Ok(None);
        };
        // Slider release plays the OPPOSITE state's ClickSound, but the
        // CURRENT state's volume selector. A nonzero configured override
        // replaces the sound, including negative sentinels that then no-play.
        let sound = if slider.preview_sound != 0 {
            slider.preview_sound
        } else {
            self.sounds(1 - state, alternate)?[0]
        };
        Ok((sound > 0).then_some((sound, self.buttons[state].volume_type)))
    }
}
pub(crate) fn read(table: &Names, config: &NativeConfiguration) -> Result<Vec<Character>> {
    if n(table, "voiceCutButtonFlag", 1)? == 0 {
        return Ok(Vec::new());
    }
    let count = n(table, "voiceCutButtonNumber", 0)?;
    ensure!(
        (0..=256).contains(&count),
        "NNN Config character control budget"
    );
    let page = n(table, "voicePrintPage", 1)?;
    let has_volume = n(table, "charaVoiceVolumeFlag", 0)? != 0;
    let minimum = n(table, "charaVoiceVolumeMin", -15)?.clamp(-15, 15);
    let maximum = n(table, "charaVoiceVolumeMax", 15)?
        .clamp(-15, 15)
        .max(minimum);
    let volume_file = table
        .parameter(b"filenameCharaVoiceVolume")
        .map(|v| v.to_vec());
    let volume_delta = [
        n(table, "charaVolumePrintX", 0)?,
        n(table, "charaVolumePrintY", 30)?,
    ];
    let volume_size = [
        n(table, "charaVolumeSizeX", 200)?,
        n(table, "charaVolumeSizeY", 20)?,
    ];
    ensure!(
        !has_volume || volume_size.iter().all(|v| (1..=16384).contains(v)),
        "NNN Config character slider geometry"
    );
    let name_change = n(table, "voiceNameChangeFlag", 0)? != 0;
    let sound_change = n(table, "voiceVoiceChangeFlag", 0)? != 0;
    let auto = n(table, "voiceAutoLineSetFlag", 0)? != 0;
    let origin = [
        n(table, "voiceAutoLinePrintX", 0)?,
        n(table, "voiceAutoLinePrintY", 0)?,
    ];
    let step = [
        n(table, "voiceAutoLineNextX", 100)?,
        n(table, "voiceAutoLineNextY", 30)?,
    ];
    let number_x = n(table, "voiceAutoLineNumberX", 5)?;
    let number_y = n(table, "voiceAutoLineNumberY", 0)?;
    let mut out: Vec<Character> = Vec::new();
    for index in 0..count {
        let tag = format!("voice{}Button", index + 1);
        let inherited = out.last().map(|character| &character.buttons[1]);
        let mut buttons = check(table, &tag, config, inherited)?;
        if auto && (number_y != 0 || number_x != 0) {
            let cell = if number_y != 0 {
                [index / number_y, index % number_y]
            } else {
                [index % number_x, index / number_x]
            };
            let auto_point = std::array::from_fn::<_, 2, _>(|axis| {
                origin[axis].wrapping_add(cell[axis].wrapping_mul(step[axis]))
            });
            for axis in 0..2 {
                if table
                    .parameter(format!("{tag}1Print{}", ["X", "Y"][axis]).as_bytes())
                    .is_none()
                {
                    buttons[0].button.origin[axis] = auto_point[axis];
                }
                if table
                    .parameter(format!("{tag}2Print{}", ["X", "Y"][axis]).as_bytes())
                    .is_none()
                {
                    buttons[1].button.origin[axis] = buttons[0].button.origin[axis];
                }
            }
        }
        let filename_variable = if name_change {
            variable(table, config, &format!("voiceNameChangeVar{}", index + 1))
        } else {
            None
        };
        let mut sound_override = [[0; 4]; 2];
        if sound_change {
            for (state, name) in ["Off", "On"].iter().enumerate() {
                for (kind, event) in ["Click", "Cannot", "Enter", "Exit"].iter().enumerate() {
                    sound_override[state][kind] = n(
                        table,
                        &format!("voiceVoiceChangeVoice{name}{event}{}", index + 1),
                        0,
                    )?;
                }
            }
        }
        let sound_variable = if sound_override.iter().flatten().any(|value| *value > 0) {
            // Original dereferenced the unallocated name-change array if that
            // feature was off. Define only that missing fallback as None;
            // an explicit voiceVoiceChangeVar still overrides normally.
            variable(table, config, &format!("voiceVoiceChangeVar{}", index + 1))
                .or(filename_variable)
        } else {
            None
        };
        let slider = if has_volume {
            Some(Slider {
                file: volume_file.clone(),
                origin: std::array::from_fn(|axis| {
                    buttons[0].button.origin[axis].wrapping_add(volume_delta[axis])
                }),
                size: volume_size,
                minimum,
                maximum,
                preview_sound: n(table, &format!("charaVoiceVolumeSE{}", index + 1), 0)?,
            })
        } else {
            None
        };
        out.push(Character {
            page,
            buttons,
            only_used: n(table, "voiceCutButtonCheckAppearFlag", 0)? != 0,
            filename_variable,
            sound_variable,
            sound_override,
            slider,
        });
    }
    Ok(out)
}

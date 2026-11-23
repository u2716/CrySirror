// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CommonConfig character-volume data/transition policy. Generic pointer
//! capture, slider hit testing and rendering belong to the shared UI owner.
//! Source: commonConfig.cpp constructor510..754, Init1745..1806,
//! Calcu2331..2373 and InitConfig4789..4838; commonSystemFile slider helpers.
use crate::{config::Names, native_system_file::NativeSystemFile};
use anyhow::{Result, ensure};
#[derive(Clone, Debug)]
pub(crate) struct Profile {
    pub check_used: bool,
    pub buttons: Vec<[crate::native_radio_group::RadioButtonSpec; 2]>,
    pub minimum: i32,
    pub maximum: i32,
    pub override_sounds: Vec<i32>,
    pub reset_gaze: i32,
    pub reset_voice: i32,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct SliderChange {
    /// Actual integer step produced by the shared slider, -1 for no change.
    pub gaze: i32,
    pub captured: bool,
    /// Native CCommonCheckButton state:0 off,1 on. This is not voiceOff's bit.
    pub button_state: i32,
    pub all_exist: bool,
    pub appears: bool,
    pub active_mode: i32,
    pub click_sounds: [i32; 2],
    pub volume_types: [i32; 2],
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Sound {
    /// The callback consumes zero-based indices; keep source's snd-1 here.
    pub zero_based: i32,
    pub volume_type: i32,
}
#[derive(Clone, Debug)]
pub(crate) struct Reset {
    /// Init retains this raw gaze even outside0..steps. Do not replace its
    /// later drag-clamp behavior with eager clamping during initialization.
    pub gazes: Vec<(usize, i32)>,
    pub voice_states: Vec<(usize, i32)>,
}
impl Profile {
    pub(crate) fn read(table: &Names) -> Result<Option<Self>> {
        let count = if table.integer(b"voiceCutButtonFlag")?.unwrap_or(1) != 0 {
            table.integer(b"voiceCutButtonNumber")?.unwrap_or(0)
        } else {
            0
        };
        ensure!(
            (0..=256).contains(&count),
            "NNN character-volume button count"
        );
        if count == 0 || table.integer(b"charaVoiceVolumeFlag")?.unwrap_or(0) == 0 {
            return Ok(None);
        }
        let minimum = table
            .integer(b"charaVoiceVolumeMin")?
            .unwrap_or(-15)
            .clamp(-15, 15);
        let maximum = table
            .integer(b"charaVoiceVolumeMax")?
            .unwrap_or(15)
            .clamp(-15, 15)
            .max(minimum);
        let override_sounds = (1..=count)
            .map(|index| {
                table
                    .integer(format!("charaVoiceVolumeSE{index}").as_bytes())
                    .map(|sound| sound.unwrap_or(0))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Some(Self {
            check_used: table
                .integer(b"voiceCutButtonCheckAppearFlag")?
                .unwrap_or(0)
                != 0,
            buttons: Vec::new(),
            minimum,
            maximum,
            override_sounds,
            reset_gaze: table.integer(b"initCharaVoiceVolume")?.unwrap_or(-1),
            reset_voice: table.integer(b"initCharaVoiceOff")?.unwrap_or(-1),
        }))
    }
    pub(crate) fn steps(&self) -> i32 {
        // CMySlider independently raises devide0 to1, even when both native
        // config endpoints are equal. Its last step then writes minimum+1.
        (self.maximum - self.minimum).max(1)
    }
    pub(crate) fn initial_gaze(&self, file: &NativeSystemFile, index: usize) -> Result<i32> {
        ensure!(
            index < self.override_sounds.len(),
            "NNN character-volume slot"
        );
        Ok(
            crate::native_voice_policy::character_slider(file, index as i32)?
                .wrapping_sub(self.minimum),
        )
    }
    pub(crate) fn changed(
        &self,
        file: &mut NativeSystemFile,
        index: usize,
        event: SliderChange,
    ) -> Result<Option<Sound>> {
        ensure!(
            index < self.override_sounds.len(),
            "NNN character-volume slot"
        );
        ensure!(
            matches!(event.button_state, 0 | 1),
            "NNN character-volume button state"
        );
        if !event.all_exist
            || !event.appears
            || event.button_state == 0
            || event.active_mode != -1
            || event.gaze == -1
        {
            return Ok(None);
        }
        ensure!(
            (0..=self.steps()).contains(&event.gaze),
            "NNN shared slider returned an invalid native step"
        );
        crate::native_voice_policy::set_character_slider(
            file,
            index as i32,
            event.gaze.wrapping_add(self.minimum),
        )?;
        if event.captured {
            return Ok(None);
        }
        let state = event.button_state as usize;
        // Source deliberately samples the OPPOSITE state's click sound but
        // the CURRENT state's volume type. A nonzero negative override mutes.
        let sound = if self.override_sounds[index] != 0 {
            self.override_sounds[index]
        } else {
            event.click_sounds[1 - state]
        };
        Ok((sound > 0).then_some(Sound {
            zero_based: sound.wrapping_sub(1),
            volume_type: event.volume_types[state],
        }))
    }
    pub(crate) fn reset(&self, file: &mut NativeSystemFile) -> Result<Reset> {
        let mut out = Reset {
            gazes: Vec::new(),
            voice_states: Vec::new(),
        };
        for index in 0..self.override_sounds.len() {
            if self.reset_gaze >= 0 {
                crate::native_voice_policy::set_character_slider(
                    file,
                    index as i32,
                    self.reset_gaze.wrapping_add(self.minimum),
                )?;
                out.gazes.push((index, self.reset_gaze));
            }
            if self.reset_voice >= 0 {
                // InitConfig's comments say 'off' for0 but its real call is
                // SetVoiceFlag(TRUE), i.e. a CLEARED voiceOff bit and state1.
                let enabled = self.reset_voice == 0;
                let at = 1536 + (index & 31) * 4;
                let mask = 1i32.wrapping_shl((index / 32) as u32);
                let value = file.system_word(at)?;
                file.set_setting_word(at, if enabled { value & !mask } else { value | mask })?;
                out.voice_states.push((index, i32::from(enabled)));
            }
        }
        Ok(out)
    }
}

// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native button request order and scoped preference policy. The Audio owner
//! retains independent system/button channels and asynchronous decoder work.
use super::*;
use crate::config::Names;
#[derive(Clone, Copy, Debug)]
pub(crate) enum NumberedVoiceProfile {
    /// PlayButtonVoice's entire body is excluded without SYSTEMNNN_VER2.
    LegacyNoNumberedVoice,
    /// Catalogue selection belongs to source SYSTEMFLAG2, never SYSTEMDATA.
    Source32 { catalogue: i32 },
}
#[derive(Clone, Debug)]
pub(crate) struct ButtonAudioPolicy {
    pub numbered: NumberedVoiceProfile,
    pub scale: i32,
    pub config_present: bool,
    pub enable_special_volume: bool,
    pub skip_switch: bool,
    pub skip_character: bool,
}
#[derive(Clone, Debug)]
pub(crate) struct ButtonAudioEvent {
    pub sound: i32,
    pub voice: i32,
    pub volume_type: i32,
    sound_done: bool,
}
impl ButtonAudioEvent {
    pub(crate) fn new(sound: i32, voice: i32, volume_type: i32) -> Self {
        Self {
            sound,
            voice,
            volume_type,
            sound_done: false,
        }
    }
}
impl Bridge {
    pub(crate) fn configure_button_audio(
        &mut self,
        catalogues: Vec<Arc<Names>>,
        number: i32,
        base: u32,
        resources: crate::resource_bridge::NativeResources,
        names: crate::native_resources::ResourceNames,
        scale: i32,
        enable_special_volume: bool,
        config_present: bool,
    ) -> Result<()> {
        ensure!(
            self.button_audio.is_none(),
            "NNN native button audio configured twice"
        );
        self.audio
            .as_mut()
            .context("NNN button voice audio capability absent")?
            .configure_button_voices(catalogues, number, base, resources, names)?;
        let file = self
            .system_file
            .as_ref()
            .context("NNN button voice SystemFile absent")?;
        let numbered = if file.source_version2() {
            let catalogue = file.flag2_word(6)?;
            ensure!(catalogue >= 0, "NNN negative source32 button catalogue");
            self.audio
                .as_ref()
                .unwrap()
                .button_voice_filename(catalogue, 1)?;
            NumberedVoiceProfile::Source32 { catalogue }
        } else {
            NumberedVoiceProfile::LegacyNoNumberedVoice
        };
        self.button_audio = Some(ButtonAudioPolicy {
            numbered,
            scale,
            config_present,
            enable_special_volume,
            skip_switch: false,
            // CGameCallBack's original constructor omitted skipVoiceOffChara.
            // Define the initial gate as enabled, matching later reset calls.
            skip_character: false,
        });
        Ok(())
    }
    pub(crate) fn set_numbered_button_voice_profile(
        &mut self,
        profile: NumberedVoiceProfile,
    ) -> Result<()> {
        if let NumberedVoiceProfile::Source32 { catalogue } = profile {
            ensure!(catalogue >= 0, "NNN negative source32 button catalogue");
            self.audio
                .as_ref()
                .context("NNN button audio absent")?
                .button_voice_filename(catalogue, 1)?;
            let file = self
                .system_file
                .as_mut()
                .context("NNN button voice SystemFile absent")?;
            ensure!(
                file.source_version2(),
                "NNN numbered button voices require source VERSION2 ABI"
            );
            file.set_flag2_word(6, catalogue)?;
        } else {
            ensure!(
                !self
                    .system_file
                    .as_ref()
                    .context("NNN button voice SystemFile absent")?
                    .source_version2(),
                "NNN cannot disable VERSION2 native button-voice policy"
            );
        }
        self.button_audio
            .as_mut()
            .context("NNN button policy absent")?
            .numbered = profile;
        Ok(())
    }
    pub(super) fn poll_button_sound_typed(
        &mut self,
        host: &mut dyn HostServices,
        number: i32,
        volume_type: i32,
    ) -> Result<bool> {
        let names = self
            .voice_policy
            .as_ref()
            .context("NNN system sound character policy absent")?;
        let file = self
            .system_file
            .as_ref()
            .context("NNN system sound preference owner absent")?;
        let policy = self
            .button_audio
            .as_ref()
            .context("NNN system sound button policy absent")?;
        let total_off = names.total_off(file)?;
        let voice_enabled = file.system_word(196)? != 0;
        let sound_enabled = file.system_word(200)? != 0;
        let allowed = !total_off
            && if volume_type == 0 {
                sound_enabled
            } else {
                voice_enabled
            };
        let delta = if allowed && volume_type != 0 && policy.enable_special_volume {
            // Source deliberately names the character delta from systemSeList,
            // even when later profiles select another playback catalogue.
            if let Some(filename) = self
                .audio
                .as_ref()
                .context("NNN system sound owner absent")?
                .system_sound_filename(number)?
            {
                names.volume_delta(&filename, file, policy.scale, policy.config_present)?
            } else {
                0
            }
        } else {
            0
        };
        let inputs = crate::audio_bridge::system_audio::SoundInputs {
            total_off,
            voice_enabled,
            sound_enabled,
            voice_volume: file.system_word(180)?,
            sound_volume: file.system_word(184)?,
            volume_type,
            name_delta: delta,
        };
        let audio = self
            .audio
            .as_mut()
            .context("NNN system sound owner absent")?;
        let before = audio.api_calls;
        let result = audio.poll_system_sound_typed(host, number, inputs);
        self.api_calls += audio.api_calls - before;
        result
    }
    pub(crate) fn poll_button_event(
        &mut self,
        host: &mut dyn HostServices,
        event: &mut ButtonAudioEvent,
    ) -> Result<bool> {
        if !event.sound_done {
            if event.sound > 0 {
                if !self.poll_button_sound_typed(host, event.sound, event.volume_type)? {
                    return Ok(false);
                }
            }
            event.sound_done = true;
        }
        if event.voice <= 0 {
            return Ok(true);
        }
        let use_system_voice = match self
            .button_audio
            .as_ref()
            .context("NNN button audio policy absent")?
            .numbered
        {
            NumberedVoiceProfile::LegacyNoNumberedVoice => return Ok(true),
            NumberedVoiceProfile::Source32 { .. } => self
                .system_file
                .as_ref()
                .context("NNN button voice SystemFile absent")?
                .flag2_word(6)?,
        };
        let filename = self
            .audio
            .as_ref()
            .context("NNN button voice audio capability absent")?
            .button_voice_filename(use_system_voice, event.voice)?;
        let Some(filename) = filename else {
            return Ok(true);
        };
        let policy = self
            .button_audio
            .as_ref()
            .context("NNN button voice preference policy absent")?;
        let file = self
            .system_file
            .as_ref()
            .context("NNN button voice system file absent")?;
        let names = self
            .voice_policy
            .as_ref()
            .context("NNN button voice character policy absent")?;
        let total_off = names.total_off(file)?;
        let voice_enabled = file.system_word(196)? != 0;
        let permitted = !total_off && (policy.skip_switch || voice_enabled);
        let character_allowed = if permitted && !policy.skip_character {
            names.allows_with_config(&filename, file, policy.config_present)?
        } else {
            true
        };
        let inputs = crate::native_button_voice::Inputs {
            total_off,
            voice_enabled,
            character_allowed,
            skip_switch_check: policy.skip_switch,
            skip_character_check: policy.skip_character,
            voice_volume: file.system_word(180)?,
            name_delta: if permitted && character_allowed {
                names.volume_delta(&filename, file, policy.scale, policy.config_present)?
            } else {
                0
            },
        };
        let audio = self.audio.as_mut().unwrap();
        let before = audio.api_calls;
        let result = audio.poll_button_voice(host, &filename, inputs);
        self.api_calls += audio.api_calls - before;
        result
    }
}

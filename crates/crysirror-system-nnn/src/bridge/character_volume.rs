// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Config changes write native packed words; released sliders retain their
//! real typed system-sound preparation instead of restarting unrelated music.
use super::*;
use crate::native_character_setting::Setting;
impl Bridge {
    /// Explicit published-source Config controls. Legacy Config51b3e0 has
    /// voice-cut buttons but no character-volume slider; unused setup keys
    /// cannot silently select this newer class behavior in an old executable.
    pub fn configure_source32_character_volume_controls(
        &mut self,
        setup: &crate::config::Names,
        configuration: &crate::native_bootstrap::NativeConfiguration,
    ) -> Result<()> {
        ensure!(
            !self.dependency_pending(),
            "NNN source Config admission during native dependency"
        );
        let mut profile = crate::native_character_volume::Profile::read(setup)?;
        if let Some(profile) = profile.as_mut() {
            profile.buttons = crate::native_radio_group::RadioSpec::read(
                setup,
                "voice",
                profile.override_sounds.len(),
                configuration,
            )?
            .buttons;
        }
        self.configure_character_volume(profile)
    }

    pub(crate) fn configure_character_volume(
        &mut self,
        profile: Option<crate::native_character_volume::Profile>,
    ) -> Result<()> {
        ensure!(
            self.character_volume.is_none(),
            "NNN character-volume profile already set"
        );
        if let Some(profile) = profile {
            ensure!(
                profile.buttons.len() == profile.override_sounds.len(),
                "NNN character-volume native button metadata absent"
            );
            self.character_volume = Some(Arc::new(profile));
        }
        Ok(())
    }
    pub(super) fn append_character_settings(
        &self,
        file: &crate::native_system_file::NativeSystemFile,
        out: &mut Vec<(i32, i32)>,
    ) -> Result<()> {
        let Some(profile) = self.character_volume.as_ref() else {
            return Ok(());
        };
        for slot in 0..profile.override_sounds.len() {
            if profile.check_used
                && !self
                    .voice_policy
                    .as_ref()
                    .context("NNN character policy absent")?
                    .button_voice_used(file, slot)?
            {
                continue;
            }
            let setting = Setting {
                slot,
                minimum: profile.minimum,
                maximum: profile.maximum,
                toggle: false,
            };
            out.push((setting.id()?, profile.initial_gaze(file, slot)?));
            out.push((
                Setting {
                    toggle: true,
                    ..setting
                }
                .id()?,
                i32::from(crate::native_character_setting::voice_enabled(file, slot)?),
            ));
        }
        Ok(())
    }
    pub(crate) fn apply_character_setting(
        &mut self,
        id: i32,
        value: i32,
        captured: bool,
    ) -> Result<Option<button_audio::ButtonAudioEvent>> {
        ensure!(
            !self.dependency_pending(),
            "NNN character setting during native dependency"
        );
        let setting = Setting::decode(id).context("NNN character setting identity")?;
        let profile = self
            .character_volume
            .as_ref()
            .context("NNN character-volume owner absent")?;
        ensure!(
            setting.minimum == profile.minimum
                && setting.maximum == profile.maximum
                && setting.slot < profile.buttons.len(),
            "NNN character setting profile changed"
        );
        let file = self
            .system_file
            .as_mut()
            .context("NNN character-volume SystemFile absent")?;
        let state = usize::from(crate::native_character_setting::voice_enabled(
            file,
            setting.slot,
        )?);
        let buttons = &profile.buttons[setting.slot];
        if setting.toggle {
            ensure!(matches!(value, 0 | 1), "NNN character voice switch value");
            crate::native_character_setting::set_voice_enabled(file, setting.slot, value != 0)?;
            let button = &buttons[state];
            return Ok((button.raw_sounds[0] > 0 || button.voices[0] > 0).then(|| {
                button_audio::ButtonAudioEvent::new(
                    button.raw_sounds[0],
                    button.voices[0],
                    button.volume_type,
                )
            }));
        }
        let sound = profile.changed(
            file,
            setting.slot,
            crate::native_character_volume::SliderChange {
                gaze: value,
                captured,
                button_state: state as i32,
                all_exist: true,
                appears: true,
                active_mode: -1,
                click_sounds: buttons.each_ref().map(|button| button.raw_sounds[0]),
                volume_types: buttons.each_ref().map(|button| button.volume_type),
            },
        )?;
        Ok(sound.map(|sound| {
            button_audio::ButtonAudioEvent::new(sound.zero_based + 1, 0, sound.volume_type)
        }))
    }
}

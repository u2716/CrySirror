// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original configuration writes and side effects. Data stays in SystemFile;
//! glyph reveal, window composition, fullscreen and audio remain host services.
use super::*;
impl Bridge {
    pub(crate) fn take_result_notices(&mut self) -> std::collections::VecDeque<i32> {
        self.result_data.as_mut().map(|data|data.take_notices()).unwrap_or_default()
    }
    pub(crate) fn gallery_progress(&self) -> Result<crate::native_system_file::NativeSystemFile> {
        self.system_file.clone().context("NNN gallery original SystemFile absent")
    }
    pub(crate) fn apply_system_message_action(&mut self,
        action:crate::native_message_control::SystemMessageAction,host:&mut dyn HostServices)->Result<()> {
        use crate::native_message_control::SystemMessageAction as A;
        ensure!(!self.dependency_pending()&&self.actor_frame.is_none(),
            "NNN SystemMenu message action requires restored, drained foreground state");
        match action {
            A::HideWindow=>{
                // CGameCallBack::WindowOn always targets PRINTMESSAGE, even
                // when SystemMenu returns to SELECTMESSAGE. Its separate
                // choice-window flag is intentionally not changed here.
                let text=self.text.as_mut().context("NNN SystemMenu PrintMessage object absent")?;
                text.set_window_on(false,host)?;
                self.actor_refresh_required|=text.take_window_redraw();
            }
            A::Skip=>self.message_skip=true,
            A::Automatic=>{
                let file=self.system_file.as_mut().context("NNN SystemMenu SystemFile absent")?;
                file.set_auto_mode(true)?;
                // SetSystemParam(6) does not change the separate DEMO flag.
                // Ordinary and restored PRINT waits consume this preference
                // on every admitted update, with their original timing bank.
                if let Some(text)=self.text.as_mut(){text.system_auto(true);}
            }
        }
        Ok(())
    }
    pub(crate) fn resume_after_menu(&mut self, host: &mut dyn HostServices) -> Result<()> {
        self.hide_film_button(host)?;
        self.actor_clock_at = None;
        if let Some(text) = self.text.as_mut() {
            text.resume_after_menu(host.now_ms());
        }
        if let Some(selection) = self.selection.as_mut() {
            selection.resume_after_menu(host.now_ms());
        }
        host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"layers":[{"op":"set_clock_paused","id":self.bindings.root,"paused":false}]}))?)?;
        Ok(())
    }
    pub(crate) fn menu_settings(&self) -> Result<Vec<(i32, i32)>> {
        let file = self
            .system_file
            .as_ref()
            .context("NNN settings SystemFile absent")?;
        crate::native_settings::SETTINGS
            .iter()
            .map(|s| Ok((s.id, file.system_word(s.offset)?)))
            .collect()
    }
    pub(crate) fn apply_menu_setting(
        &mut self,
        id: i32,
        value: i32,
        host: &mut dyn HostServices,
    ) -> Result<Option<[i32; 4]>> {
        ensure!(
            !self.dependency_pending(),
            "NNN settings change requires a drained actor"
        );
        let mut file = self
            .system_file
            .as_ref()
            .context("NNN settings SystemFile absent")?
            .clone();
        file.set_menu_parameter(id, value)?;
        let word = |offset| file.system_word(offset);
        let audio = crate::native_audio::AudioSettings {
            music_volume: word(176)?,
            voice_volume: word(180)?,
            sound_volume: word(184)?,
            sound_voice_volume: word(212)?,
            music_enabled: word(192)? != 0,
            voice_enabled: word(196)? != 0,
            sound_enabled: word(200)? != 0,
            sound_voice_enabled: word(216)? != 0,
        };
        let restart = self
            .audio
            .as_mut()
            .context("NNN settings audio capability absent")?
            .apply_settings(audio, host)?;
        match id {
            4 => host.submit_engine(
                self.client,
                1,
                &serde_json::to_vec(&json!({"ui":[{"op":"set_fullscreen","enabled":value!=0}]}))?,
            )?,
            6 => {
                self.legacy_demo_flag = value;
                if let Some(text) = self.text.as_mut() {
                    text.system_auto(value != 0);
                }
            }
            105 => {
                if let Some(text) = self.text.as_mut() {
                    text.change_speed(value as usize, host)?;
                }
            }
            106 => {
                if let Some(text) = self.text.as_mut() {
                    text.change_window_percent(100 - value)?;
                }
                if let Some(binding) = self.window_binding.as_mut() {
                    binding.change_transparency(value)?;
                }
                self.actor_refresh_required = true;
            }
            _ => (),
        }
        self.system_file = Some(file);
        Ok(restart)
    }
}

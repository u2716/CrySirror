//! Scene replay mode policy, over the same shared resources and audio owner.
use super::*;
impl Bridge {
    pub(crate) fn begin_scene_replay(&mut self, host: &mut dyn HostServices) -> Result<()> {
        if self.audio.is_some() {
            self.stop_for_outer_movie(host)?;
        }
        self.hide_film_button(host)?;
        self.enter_foreground_context();
        self.scene_replay = true;
        self.skip.stop_f5();
        self.skip.film_end = false;
        self.skip_film_enabled = false;
        self.auxiliary.cutin_mode = 0;
        if let Some(text) = self.text.as_mut() {
            text.profile.cutin_mode = 0;
        }
        self.native_layer_off = 0;
        self.actor_clock_at = None;
        Ok(())
    }
    pub(crate) fn hide_scene_skip(&mut self, host: &mut dyn HostServices) -> Result<()> {
        self.hide_film_button(host)
    }
    pub(crate) fn finish_scene_replay(&mut self, allow: bool) -> Result<()> {
        self.scene_replay = false;
        self.enter_foreground_context();
        self.configure_background_read_marks(allow)?;
        self.actor_clock_at = None;
        Ok(())
    }
}

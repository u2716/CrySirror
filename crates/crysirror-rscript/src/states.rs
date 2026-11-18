// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Native state switches of the Shapeshifter build. WinMain registers the
//! states with sub_44F000 in this order: title (sub_440A20), game
//! (sub_42F0C0), load from the title (sub_432310), configuration from the
//! title (sub_41CFE0), CG view (sub_4319B0), scene view (sub_43FD10), music
//! menu (sub_434730), extra menu (sub_42E2B0) and the plug-in title
//! (sub_42D680). Message 0x410 switches to a state; a switch stops sound
//! and voice, fades the music out over 10 steps (sub_4527E0) and fades the
//! screen out in 16 steps of 20 ms (sub_44FCF0). The game state's start
//! (message 0x40E) reloads the script on top of the scene's script stack,
//! resumes at its return address and rebuilds every object (sub_43CF20).
use crate::memory::scene as S;
use crate::scene::MessageWindow;
use crate::vm::{Flags, Thread, Vm, MUSIC_CHANNEL, SOUND_CHANNEL, VOICE_CHANNEL};
use anyhow::Result;
use crysirror_core::service::{AudioRequest, HostServices, MediaRequest};

/// sub_4527E0: music fade-out steps.
const MUSIC_FADE_STEPS: i32 = 10;

impl Vm {
    /// Ends the game state (sub_4306B0): the system save is written and
    /// the scene's objects, waits and audio end.
    pub(crate) fn leave_game(&mut self, h: &mut dyn HostServices) -> Result<()> {
        self.write_system(h)?;
        for channel in [SOUND_CHANNEL, SOUND_CHANNEL + 1, SOUND_CHANNEL + 2, VOICE_CHANNEL] {
            h.submit_audio(self.client, AudioRequest::Stop { channel, fade_ms: 0.0 })?;
        }
        if self.music.take().is_some() {
            let fade_ms = MUSIC_FADE_STEPS as f32 * crate::vm::FADE_STEP_MS;
            h.submit_audio(self.client, AudioRequest::Stop { channel: MUSIC_CHANNEL + self.music_stream, fade_ms })?;
            self.music_stream ^= 1;
        }
        self.loaded_sound = [None; 3];
        if self.movie.take().is_some() {
            h.media_request(self.client, MediaRequest::Remove { id: crate::vm::MOVIE_ID })?;
        }
        self.nest.clear();
        self.cont.clear();
        self.wait = None;
        self.flags = Flags::default();
        self.choice = None;
        self.dyn_select = None;
        self.edit = None;
        self.save_screen = None;
        self.config_screen = None;
        self.state_screen = None;
        self.backlog_view = None;
        self.hovered = None;
        self.movie_queue.clear();
        self.shake = [0, 0];
        self.flash = None;
        self.dip = None;
        self.dirty = true;
        Ok(())
    }
    /// Fresh objects of the game state (sub_42F0C0).
    fn fresh_objects(&mut self, h: &mut dyn HostServices) {
        self.layers = (0..S::LAYER_COUNT).map(crate::scene::Layer::new).collect();
        self.gauges = (0..S::GAUGE_COUNT).map(|_| Default::default()).collect();
        self.map = Default::default();
        self.effect = Default::default();
        self.overlay = Default::default();
        self.message = MessageWindow::new();
        let mut cx = crate::scene::Ctx { h: &mut *h, client: self.client, res: &mut self.res, apini: &self.apini };
        self.message.load_panel(&mut cx);
        let alpha = self.mem.cfg(crate::memory::config::WINDOW_ALPHA) as u8;
        for b in &mut self.message.boxes {
            b.back_alpha = alpha;
        }
        self.sync_panel_slider();
    }
    /// Leaves and enters the game state on the scene now in memory (a slot
    /// load, the previous-choice restore).
    pub(crate) fn reenter_game(&mut self, h: &mut dyn HostServices) -> Result<()> {
        self.leave_game(h)?;
        self.fresh_objects(h);
        self.rebuild(h);
        let script = self.current_script();
        self.thread = Thread { program: None, script: -1, pc: 0 };
        self.load(script)?;
        let pc = self.return_at(self.depth());
        self.jump(pc);
        self.apply_message_speed(h);
        self.begin_switch_fade(h)?;
        Ok(())
    }
    /// The rev button (sub_42EE50): the previous-choice snapshot replaces
    /// the live scene and variables (sub_4330C0) and the game state starts
    /// again.
    pub(crate) fn previous_choice(&mut self, h: &mut dyn HostServices) -> Result<()> {
        self.mem.restore_previous_snapshot();
        self.reenter_game(h)
    }
    /// Back to the title state (sub_4315B0 after its question; button
    /// mode 1, id 1 through sub_451CB0).
    pub(crate) fn return_to_title(&mut self, h: &mut dyn HostServices) -> Result<()> {
        self.top_menu_start = false;
        self.leave_game(h)?;
        self.fresh_objects(h);
        self.thread = Thread { program: None, script: -1, pc: 0 };
        self.begin_switch_fade(h)?;
        self.show_title(h)
    }
    /// WM_SYSCOMMAND SC_CLOSE (sub_42EFE0, button mode 1 id 0, the top
    /// menu's last item): the state exit writes the system save, then the
    /// program ends.
    pub(crate) fn quit(&mut self, h: &mut dyn HostServices) -> Result<()> {
        self.leave_game(h)?;
        self.halted = true;
        Ok(())
    }
}

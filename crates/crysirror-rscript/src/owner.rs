// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! The native scene owner: the fixed-period tick (period from APINI), window input and the resumption of
//! waiting handlers.
use crate::memory::{config as C, scene as S};
use crate::scene::Choice;
use crate::vm::{Cont, MUSIC_CHANNEL, SOUND_CHANNEL, VOICE_CHANNEL, Vm, Wait};
use anyhow::{Result, bail};
use crysirror_core::{
    image_processing::ImageProcessState,
    service::{AudioPreparePhase, AudioRequest, HostServices, InputFrame, MediaPhase, MediaRequest, buttons},
};

const MOVIE_ID: u32 = 1;
/// Ticks replayed after a long host stall before time is dropped.
const MAX_CATCH_UP: u32 = 8;

/// What the pointer is over.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hit {
    Answer(usize),
    Map(usize),
    Layer(usize),
    Panel(usize),
    PanelSlider,
}

impl Vm {
    /// One owner step: input, native ticks, waits, then script execution.
    pub fn update(&mut self, h: &mut dyn HostServices, input: InputFrame, now: u64, budget: u32) -> Result<()> {
        if self.next_tick == 0 {
            self.next_tick = now;
        }
        self.now = now;
        self.input = input;
        self.poll_audio(h)?;
        self.handle_input(h)?;
        let period = u64::from(self.apini.tick_ms);
        let mut ticks = 0;
        while now >= self.next_tick {
            if ticks == MAX_CATCH_UP {
                self.next_tick = now + period;
                break;
            }
            self.tick(h)?;
            self.next_tick += period;
            ticks += 1;
        }
        if matches!(self.wait, Some(Wait::Frame)) {
            self.wait = None;
        }
        self.poll_conversions(h)?;
        self.poll_zoom(h)?;
        self.resolve(h)?;
        let mut left = budget.max(1);
        // A layer conversion holds the thread until its picture is ready.
        while self.wait.is_none() && !self.halted && left > 0 && !self.converting() && self.state_screen.is_none() && self.zoom.is_none() {
            let chunk = left.min(64);
            self.run(h, chunk)?;
            left -= chunk;
            self.drain(h)?;
            self.resolve(h)?;
        }
        Ok(())
    }

    fn converting(&self) -> bool {
        self.layers.iter().any(|l| l.convert.is_some())
    }
    fn poll_conversions(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let screen = self.screen();
        for i in 0..self.layers.len() {
            let Some(job) = self.layers[i].convert.as_mut() else { continue };
            match job.poll()? {
                ImageProcessState::Pending => {}
                ImageProcessState::Failed(e) => {
                    self.layers[i].convert = None;
                    self.res.diag(format!("layer {i} conversion: {e}"));
                    self.dirty = true;
                }
                ImageProcessState::Ready(image) => {
                    let size = image.size();
                    self.next_image += 1;
                    let published = h.publish_image_resource(self.client, self.next_image, image)?;
                    let content = crate::sprite::Content::Image { asset: published.path.relative().to_owned(), size };
                    self.layers[i].finish_convert(&self.mem, content, screen);
                    self.dirty = true;
                }
            }
        }
        Ok(())
    }
    fn poll_audio(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let mut keep = Vec::new();
        for ticket in std::mem::take(&mut self.audio_pending) {
            let p = h.audio_preparation(self.client, ticket)?;
            match p.phase {
                AudioPreparePhase::Pending => keep.push(ticket),
                AudioPreparePhase::Ready => h.release_audio(self.client, ticket)?,
                AudioPreparePhase::Failed => {
                    h.release_audio(self.client, ticket)?;
                    self.res.diag(format!("audio preparation: {}", p.error.unwrap_or_default()));
                }
            }
        }
        self.audio_pending = keep;
        Ok(())
    }

    /// Runs continuations while nothing waits.
    pub(crate) fn drain(&mut self, h: &mut dyn HostServices) -> Result<()> {
        while self.wait.is_none() {
            let Some(c) = self.cont.pop() else { break };
            self.resume(h, c)?;
        }
        Ok(())
    }

    /// Resolves the current wait when its condition holds.
    fn resolve(&mut self, h: &mut dyn HostServices) -> Result<()> {
        loop {
            let done = match self.wait.clone() {
                None => {
                    self.drain(h)?;
                    if self.wait.is_none() {
                        return Ok(());
                    }
                    continue;
                }
                Some(Wait::Frame) => false,
                Some(Wait::Refresh) => !self.flags.wait_animation,
                Some(Wait::Click) => !self.flags.wait_click,
                Some(Wait::Auto { until }) => {
                    !self.flags.auto || (self.now >= until && !h.audio_playing(self.client, VOICE_CHANNEL).unwrap_or(false))
                }
                Some(Wait::Sleep { until }) => self.now >= until || !self.flags.sleeping,
                Some(Wait::Audio { channel }) => {
                    let waiting = if channel == VOICE_CHANNEL { self.flags.wait_voice } else { self.flags.wait_sound };
                    !waiting || (self.audio_pending.is_empty() && !h.audio_playing(self.client, channel).unwrap_or(false))
                }
                Some(Wait::Choice) => !self.flags.choice,
                Some(Wait::Buttons) => self.flags.button_wait == 0,
                Some(Wait::Edit) => self.edit.is_none(),
                Some(Wait::Screen) => self.save_screen.is_none(),
                Some(Wait::State) => self.state_screen.is_none(),
                Some(Wait::Zoom) => self.zoom.is_none(),
                Some(Wait::Until { until }) => self.now >= until,
                Some(Wait::Steps { mut steps, at }) => {
                    let mut at = at;
                    while let Some(&(ms, _, _)) = steps.first() {
                        if self.now < at + ms {
                            break;
                        }
                        at += ms;
                        steps.remove(0);
                    }
                    if let Some(&(_, offset, fill)) = steps.first() {
                        if (self.shake, self.flash) != (offset, fill) {
                            self.shake = offset;
                            self.flash = fill;
                            self.dirty = true;
                        }
                        self.wait = Some(Wait::Steps { steps, at });
                        false
                    } else {
                        self.shake = [0, 0];
                        self.flash = None;
                        self.dirty = true;
                        true
                    }
                }
                Some(Wait::Movie) => {
                    let info = h.media_info(self.client, MOVIE_ID)?;
                    matches!(info.phase, MediaPhase::Ended | MediaPhase::Failed | MediaPhase::Stopped)
                }
                Some(Wait::Mask { name, duration }) => {
                    let Some((_, job)) = self.mask_job.as_mut() else { bail!("RScript mask preparation lost") };
                    match job.poll()? {
                        ImageProcessState::Pending => false,
                        ImageProcessState::Failed(e) => bail!("RScript transition mask {name}: {e}"),
                        ImageProcessState::Ready(image) => {
                            self.mask_job = None;
                            self.next_image += 1;
                            let published = h.publish_image_resource(self.client, self.next_image, image)?;
                            let path = published.path.relative().to_owned();
                            self.masks.insert(name, path.clone());
                            self.wait = None;
                            self.begin_transition(serde_json::json!({"kind":"mask","asset":path,"vague":31}), duration);
                            continue;
                        }
                    }
                }
            };
            if !done {
                return Ok(());
            }
            if matches!(self.wait, Some(Wait::Movie)) {
                h.media_request(self.client, MediaRequest::Remove { id: MOVIE_ID })?;
                self.movie = None;
                self.dirty = true;
            }
            self.wait = None;
        }
    }

    fn resume(&mut self, h: &mut dyn HostServices, c: Cont) -> Result<()> {
        match c {
            Cont::ClickDone => {
                self.set_button_input(false, true);
                self.message.set_input(false);
                self.message.set_waiting(&self.mem, false);
                self.flags.wait_input = false;
                self.dirty = true;
                if self.flags.interrupt {
                    self.flags.interrupt = false;
                    self.cont.push(Cont::RetryClick);
                    let script = i32::from(self.mem.var(0));
                    self.system_call_with(h, script, None)?;
                }
            }
            Cont::RetryClick => self.wait_click(),
            Cont::ClearBox(b) => {
                let sk = self.flags.fast_skip;
                self.message.clear(&mut self.mem, b, sk);
                self.dirty = true;
            }
            Cont::MessageRevealed { b, wait, mode, new_page, voice, repeat } => self.after_reveal(b, wait, mode, new_page, voice, repeat),
            Cont::MessageClicked { b, mode, new_page, voice, repeat } => {
                self.after_click(b, mode, new_page, voice, repeat, true);
                if std::mem::take(&mut self.voice_stop) {
                    h.submit_audio(self.client, AudioRequest::Stop { channel: VOICE_CHANNEL, fade_ms: 0.0 })?;
                }
            }
            Cont::ChoiceShown { targets, effect_mode } => {
                if let Some(c) = &mut self.choice {
                    c.set_input(true);
                }
                self.flags.choice = true;
                self.flags.wait_input = true;
                self.message.set_input(true);
                self.cont.push(Cont::ChoiceAnswered { targets, effect_mode });
                self.wait = Some(Wait::Choice);
            }
            Cont::DynShown { effect_mode } => {
                if let Some(c) = &mut self.choice {
                    c.set_input(true);
                }
                self.flags.choice = true;
                self.flags.wait_input = true;
                self.message.set_input(true);
                self.cont.push(Cont::DynAnswered { effect_mode });
                self.wait = Some(Wait::Choice);
            }
            Cont::DynAnswered { effect_mode } => {
                self.message.set_input(false);
                self.flags.wait_input = false;
                let instant = self.mem.cfg(C::EFFECTS_ENABLED) == 0 || self.flags.skip || effect_mode != 0;
                if let Some(c) = &mut self.choice {
                    c.set_input(false);
                    c.disappear(instant);
                }
                self.cont.push(Cont::DynClosed);
                self.present_choice(h, effect_mode)?;
            }
            Cont::DynClosed => {
                let selected = self.choice.take().map_or(0, |c| c.selected);
                let d = self.dyn_select.take();
                let value = d.and_then(|d| selected.checked_sub(1).and_then(|i| d.page.get(i).copied().flatten()))
                    .map_or(0, |i| crate::screens::entry(&self.mem, i).2);
                self.mem.set_var(0, value);
                self.dirty = true;
            }
            Cont::MapDone { timer_var, hide_timer, cancel } => {
                self.map.set_input(false);
                self.flags.wait_input = false;
                self.message.set_input(false);
                self.flags.button_timer = false;
                if timer_var != 0 {
                    let elapsed = (self.now.saturating_sub(self.button_timer.1) / 10) as i32;
                    self.mem.set_var(timer_var, (self.button_timer.0 - elapsed).max(0) as i16);
                }
                self.dirty = true;
                if self.flags.interrupt {
                    self.flags.interrupt = false;
                    self.cont.push(Cont::RetryMap { timer_var, hide_timer, cancel });
                    let script = i32::from(self.mem.var(0));
                    self.system_call_with(h, script, None)?;
                }
            }
            Cont::RetryMap { timer_var, hide_timer, cancel } => self.map_wait(timer_var, hide_timer, cancel),
            Cont::ChoiceAnswered { targets, effect_mode } => {
                self.message.set_input(false);
                self.flags.wait_input = false;
                if self.mem.d(S::AUTO_REBUILD) != 0 && self.nest.is_empty() {
                    self.mem.promote_message_snapshot();
                }
                let instant = !(self.mem.cfg(C::EFFECTS_ENABLED) != 0) || self.flags.skip || effect_mode != 0;
                if let Some(c) = &mut self.choice {
                    c.set_input(false);
                    c.disappear(instant);
                }
                self.cont.push(Cont::ChoiceClosed { targets });
                self.present_choice(h, effect_mode)?;
            }
            Cont::ChoiceClosed { targets } => {
                let selected = self.choice.take().map_or(0, |c| c.selected);
                self.dirty = true;
                if let Some(&t) = selected.checked_sub(1).and_then(|i| targets.get(i)) {
                    self.jump(t);
                }
            }
            Cont::ButtonsDone { timer_var, hide_timer, cancel } => {
                self.flags.wait_input = false;
                self.message.set_input(false);
                self.flags.button_timer = false;
                if timer_var != 0 {
                    let elapsed = (self.now.saturating_sub(self.button_timer.1) / 10) as i32;
                    let remaining = (self.button_timer.0 - elapsed).max(0);
                    self.mem.set_var(timer_var, remaining as i16);
                }
                self.set_button_input(false, false);
                if self.flags.interrupt {
                    self.flags.interrupt = false;
                    self.cont.push(Cont::RetryButtons { timer_var, hide_timer, cancel });
                    let script = i32::from(self.mem.var(0));
                    self.system_call_with(h, script, None)?;
                    return Ok(());
                }
                if self.mem.d(S::LAYER_ANIMATION_RESET) != 0 {
                    for l in self.layers.iter_mut().skip(1) {
                        l.stop_button(&mut self.mem, false, false);
                    }
                }
                if self.mem.d(S::AUTO_REBUILD) != 0 && self.nest.is_empty() {
                    self.mem.promote_message_snapshot();
                }
                self.dirty = true;
            }
            Cont::RetryButtons { timer_var, hide_timer, cancel } => self.button_wait(timer_var, hide_timer, cancel),
            Cont::PointerDone { x_var, y_var, timer_var, hide_timer, cancel } => {
                self.flags.wait_input = false;
                if x_var != 0 {
                    self.mem.set_var(x_var, self.pointer[0] as i16);
                }
                if y_var != 0 {
                    self.mem.set_var(y_var, self.pointer[1] as i16);
                }
                self.message.set_input(false);
                self.flags.button_timer = false;
                if timer_var != 0 {
                    let elapsed = (self.now.saturating_sub(self.button_timer.1) / 10) as i32;
                    self.mem.set_var(timer_var, (self.button_timer.0 - elapsed).max(0) as i16);
                }
                if self.flags.interrupt {
                    self.flags.interrupt = false;
                    self.cont.push(Cont::RetryPointer { x_var, y_var, timer_var, hide_timer, cancel });
                    let script = i32::from(self.mem.var(0));
                    self.system_call_with(h, script, None)?;
                    return Ok(());
                }
                if self.mem.d(S::AUTO_REBUILD) != 0 && self.nest.is_empty() {
                    self.mem.promote_message_snapshot();
                }
            }
            Cont::RetryPointer { x_var, y_var, timer_var, hide_timer, cancel } => self.pointer_wait(x_var, y_var, timer_var, hide_timer, cancel),
            Cont::DipBack { duration } => {
                self.dip = None;
                self.begin_transition(serde_json::json!({"kind":"crossfade"}), duration);
            }
            Cont::WaitInputEnd => {
                self.flags.wait_input = false;
                self.flags.sleeping = false;
            }
            Cont::WaitSoundEnd => self.flags.wait_sound = false,
            Cont::WaitVoiceEnd => self.flags.wait_voice = false,
            Cont::Opening => self.next_movie(h)?,
        }
        Ok(())
    }

    // ------------------------------------------------------------------ tick

    fn tick(&mut self, h: &mut dyn HostServices) -> Result<()> {
        if self.save_screen.is_none() && self.config_screen.is_none() {
            self.idle_tick(h)?;
        }
        // Animated layers step every tick, waiting or not (sub_411320).
        let vars = self.mem.vars.clone();
        let rng = &mut self.rng;
        let mut rand = || rng.next() as i32;
        for l in &mut self.layers {
            if l.tick_anim(&|v| vars.get(v).copied().unwrap_or(0), &mut rand) {
                self.dirty = true;
            }
        }
        if self.flags.button_wait != 0 && self.flags.button_timer {
            let elapsed = (self.now.saturating_sub(self.button_timer.1) / 10) as i32;
            // sub_4308C0: the countdown runs on gauge 0 every 10 ms.
            self.update_timer_gauge((self.button_timer.0 - elapsed).max(0));
            if elapsed > self.button_timer.0 {
                self.flags.button_timer = false;
                self.button_pressed(0, 0);
            }
            return Ok(());
        }
        if !self.flags.wait_input {
            if !self.flags.wait_animation {
                return Ok(());
            }
            if self.flags.complete_animations {
                self.message.finish();
                self.flags.complete_animations = false;
            }
            let running = self.animate();
            self.dirty = true;
            if !running {
                self.flags.wait_animation = false;
            }
            return Ok(());
        }
        // Waiting for input: only idle objects tick (the click-wait icon).
        for b in &mut self.message.boxes {
            if b.waiting && b.visible {
                b.wait_glow.animating = true;
                b.wait_glow.tick();
                self.dirty = true;
            }
        }
        Ok(())
    }
    /// Root animate: every object from the top priority down.
    fn animate(&mut self) -> bool {
        let mut running = false;
        if let Some(c) = &mut self.choice {
            running |= c.tick();
        }
        for b in &mut self.message.boxes {
            if b.visible {
                running |= b.reveal.tick(&b.text.glyphs);
            }
        }
        running |= self.effect.tick() | self.overlay.tick();
        running |= self.message.face.tick() | self.message.face_old.tick();
        for g in &mut self.gauges {
            running |= g.tick();
        }
        for l in &mut self.layers {
            running |= l.tick();
        }
        running
    }

    // ----------------------------------------------------------------- input

    fn hit(&self, x: i32, y: i32) -> Option<Hit> {
        if self.map.input {
            for (i, b) in self.map.buttons.iter().enumerate().rev() {
                if b.face.interactive && b.face.contains(x, y) {
                    return Some(Hit::Map(i));
                }
            }
        }
        if let Some(c) = &self.choice {
            for (i, a) in c.answers.iter().enumerate().rev() {
                if a.sprite.interactive && a.sprite.contains(x, y) {
                    return Some(Hit::Answer(i));
                }
            }
        }
        if self.message.panel_visible && !self.message.hidden {
            let p = self.message.panel_pos;
            for (i, b) in self.message.panel.iter().enumerate() {
                if b.face.interactive && b.face.contains(x - p.x, y - p.y) {
                    return Some(Hit::Panel(i));
                }
            }
            if self.message.input_active && self.message.slider.as_ref().is_some_and(|s| s.hit(x - p.x, y - p.y)) {
                return Some(Hit::PanelSlider);
            }
        }
        let mut order: Vec<usize> = (0..self.layers.len()).collect();
        order.sort_by_key(|&i| self.mem.w(S::LAYER_ORDERS + 2 * i));
        for &i in order.iter().rev() {
            let l = &self.layers[i];
            if l.visible && l.main.interactive && l.main.contains(x, y) {
                return Some(Hit::Layer(i));
            }
        }
        None
    }

    /// Input while the 0x9C dialog is open: field focus, typing, OK/cancel.
    fn edit_input(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let input = self.input;
        let pressed = |b: u64| input.buttons_pressed & b != 0;
        let (x, y) = (input.pointer_x, input.pointer_y);
        let Some(d) = self.edit.as_mut() else { return Ok(()) };
        let o = d.origin;
        if let Some(handle) = d.session {
            let committed = h.take_text_input_committed(self.client, handle)?;
            if !committed.is_empty() {
                d.type_text(&committed);
                self.dirty = true;
            }
            let info = h.text_input_session_info(self.client, handle)?;
            if *info.preedit != d.preedit {
                d.preedit = info.preedit.to_string();
                self.dirty = true;
            }
        }
        if pressed(buttons::BACKSPACE) {
            d.backspace();
            self.dirty = true;
        }
        let inside = |b: &Option<crate::screens::LwgButton>| b.as_ref().is_some_and(|b| b.face.contains(x - o.x, y - o.y));
        let (on_ok, on_cancel) = (inside(&d.ok), inside(&d.cancel));
        if let Some(b) = &mut d.ok {
            if b.face.hovered != on_ok {
                b.hover(on_ok);
                self.dirty = true;
            }
        }
        if let Some(b) = &mut d.cancel {
            if b.face.hovered != on_cancel {
                b.hover(on_cancel);
                self.dirty = true;
            }
        }
        let mut close = false;
        if pressed(buttons::MOUSE_LEFT) {
            if on_ok {
                close = d.commit(&mut self.mem);
            } else if on_cancel {
                close = true;
            } else if let Some(i) = d.fields.iter().position(|f| {
                let [fx, fy, fw, fh] = f.rect;
                x >= o.x + fx && y >= o.y + fy && x < o.x + fx + fw && y < o.y + fy + fh
            }) {
                d.focus = i;
                self.dirty = true;
            }
        } else if pressed(buttons::ENTER) {
            close = d.commit(&mut self.mem);
        } else if pressed(buttons::TAB) && !d.fields.is_empty() {
            d.focus = (d.focus + 1) % d.fields.len();
            self.dirty = true;
        }
        if close {
            if let Some(handle) = d.session.take() {
                h.release_text_input_session(self.client, handle)?;
            }
            self.edit = None;
            self.system_sound(h, 1)?;
            self.dirty = true;
        }
        Ok(())
    }

    fn handle_input(&mut self, h: &mut dyn HostServices) -> Result<()> {
        self.apply_focus_audio(h)?;
        if self.poll_pending(h)? {
            return Ok(());
        }
        if self.edit.is_some() {
            self.pointer = [self.input.pointer_x, self.input.pointer_y];
            return self.edit_input(h);
        }
        if self.save_screen.is_some() {
            self.pointer = [self.input.pointer_x, self.input.pointer_y];
            self.save_screen_input(h)?;
            return self.state_input(h).map(|_| ());
        }
        if self.config_screen.is_some() {
            self.pointer = [self.input.pointer_x, self.input.pointer_y];
            self.config_input(h)?;
            return self.state_input(h).map(|_| ());
        }
        if self.state_screen.is_some() {
            self.pointer = [self.input.pointer_x, self.input.pointer_y];
            return self.state_input(h).map(|_| ());
        }
        self.track_idle(h)?;
        if self.panel_drag {
            if self.input.buttons_down & buttons::MOUSE_LEFT == 0 {
                self.panel_drag = false;
            } else {
                self.drag_panel_slider(self.input.pointer_x, self.input.pointer_y);
                self.pointer = [self.input.pointer_x, self.input.pointer_y];
                return Ok(());
            }
        }
        let input = self.input;
        let (x, y) = (input.pointer_x, input.pointer_y);
        let moved = [x, y] != self.pointer;
        self.pointer = [x, y];
        let pressed = |b: u64| input.buttons_pressed & b != 0;
        let hit = self.hit(x, y);
        if moved || pressed(buttons::MOUSE_LEFT) {
            self.update_hover(hit);
        }
        // Control held: skip (485360); released, skipping stops.
        let ctrl = input.buttons_down & buttons::CTRL != 0;
        if ctrl && !self.flags.skip_held {
            self.flags.skip_held = true;
            self.click(h)?;
        } else if !ctrl && self.ctrl_skip {
            self.flags.skip_held = false;
        }
        self.ctrl_skip = ctrl;
        if matches!(self.wait, Some(Wait::Movie)) && (pressed(buttons::MOUSE_LEFT) || pressed(buttons::ENTER) || pressed(buttons::ESCAPE)) {
            h.media_request(self.client, MediaRequest::Stop { id: MOVIE_ID })?;
            return Ok(());
        }
        if pressed(buttons::MOUSE_LEFT) && self.flags.button_wait == 3 {
            self.button_pressed(0, 1);
        } else if pressed(buttons::MOUSE_LEFT) {
            match hit {
                Some(Hit::Answer(i)) => {
                    let next_page = self.dyn_select.as_ref().is_some_and(|d| d.page.get(i) == Some(&None));
                    if self.flags.choice && next_page {
                        // The "next page" button pages without ending the wait.
                        self.system_sound(h, 1)?;
                        if let Some(d) = &mut self.dyn_select {
                            d.next_page();
                        }
                        self.hovered = None;
                        self.dyn_build(h, true)?;
                        if let Some(c) = &mut self.choice {
                            c.set_input(true);
                        }
                    } else if self.flags.choice && self.choice.as_mut().is_some_and(|c| c.press(i)) {
                        self.system_sound(h, 1)?;
                        self.flags.choice = false;
                        self.dirty = true;
                    }
                }
                Some(Hit::Map(i)) => {
                    // sub_417630: a map button ends the wait with its id.
                    let id = self.map.buttons[i].id;
                    self.map.buttons[i].press();
                    self.system_sound(h, 1)?;
                    self.button_pressed(0, i32::from(id));
                }
                Some(Hit::Layer(i)) => {
                    let (mode, id) = (self.layers[i].button_mode(&self.mem), self.layers[i].button_id(&self.mem));
                    self.layers[i].press(&self.mem);
                    self.dirty = true;
                    if mode == 1 {
                        self.native_button(h, i32::from(id))?;
                    } else {
                        self.button_pressed(mode, i32::from(id));
                    }
                }
                Some(Hit::Panel(i)) => {
                    let command = self.message.panel[i].command;
                    self.panel_command(h, command)?;
                }
                Some(Hit::PanelSlider) => {
                    self.panel_drag = true;
                    self.drag_panel_slider(x, y);
                }
                None => {
                    if self.flags.auto {
                        self.stop_auto();
                    } else if self.flags.skip {
                        self.flags.skip_held = false;
                    } else {
                        self.click(h)?;
                    }
                }
            }
        }
        if pressed(buttons::ENTER) || pressed(buttons::SPACE) {
            if self.flags.auto {
                self.stop_auto();
            } else if self.flags.skip {
                self.flags.skip_held = false;
            } else {
                self.click(h)?;
            }
        }
        if pressed(buttons::MOUSE_RIGHT) || pressed(buttons::ESCAPE) {
            if self.flags.window_hidden {
                self.show_window(true);
            } else if self.flags.button_wait != 0 && self.flags.button_cancel {
                self.button_pressed(0, 0);
            } else {
                self.panel_command(h, "menu")?;
            }
        }
        if pressed(buttons::TAB) {
            self.panel_command(h, "skip")?;
        }
        // VK_SHIFT: a hidden window comes back, otherwise it hides.
        if pressed(buttons::SHIFT) {
            if self.flags.window_hidden {
                self.show_window(true);
            } else {
                self.panel_command(h, "hide")?;
            }
        }
        // Wheel and cursor keys page the backlog (sub_4294D0, sub_429590).
        if pressed(buttons::WHEEL_UP) || pressed(buttons::UP) {
            self.backlog_back(h);
        } else if (pressed(buttons::WHEEL_DOWN) || pressed(buttons::DOWN)) && !self.backlog_forward(h) && pressed(buttons::WHEEL_DOWN) {
            self.click(h)?;
        }
        Ok(())
    }

    fn update_hover(&mut self, hit: Option<Hit>) {
        if hit == self.hovered {
            return;
        }
        match self.hovered.take() {
            Some(Hit::Answer(i)) => {
                if let Some(a) = self.choice.as_mut().and_then(|c| c.answers.get_mut(i)) {
                    Choice::hover(a, false);
                }
            }
            Some(Hit::Layer(i)) => {
                // A button disabled by its press keeps its hover images.
                if self.layers[i].main.interactive {
                    self.layers[i].hover(&self.mem, false);
                }
            }
            Some(Hit::Panel(i)) => {
                if let Some(b) = self.message.panel.get_mut(i) {
                    b.face.hovered = false;
                }
            }
            Some(Hit::PanelSlider) => {
                if let Some(sl) = &mut self.message.slider {
                    sl.knob.hover(false);
                }
            }
            Some(Hit::Map(i)) => {
                if let Some(b) = self.map.buttons.get_mut(i) {
                    b.hover(false);
                }
            }
            None => {}
        }
        match hit {
            Some(Hit::Answer(i)) => {
                if let Some(a) = self.choice.as_mut().and_then(|c| c.answers.get_mut(i)) {
                    Choice::hover(a, true);
                }
            }
            Some(Hit::Layer(i)) => self.layers[i].hover(&self.mem, true),
            Some(Hit::Panel(i)) => self.message.panel[i].face.hovered = true,
            Some(Hit::PanelSlider) => {
                if let Some(sl) = &mut self.message.slider {
                    sl.knob.hover(true);
                }
            }
            Some(Hit::Map(i)) => self.map.buttons[i].hover(true),
            None => {}
        }
        self.hovered = hit;
        self.dirty = true;
    }

    /// A click stops waits, shows a hidden window or resumes.
    pub(crate) fn click(&mut self, h: &mut dyn HostServices) -> Result<()> {
        // sub_430C50: a click while browsing only ends the backlog.
        if self.backlog_view.is_some() {
            self.close_backlog(h);
            return Ok(());
        }
        if self.flags.sleeping {
            self.flags.sleeping = false;
        } else if self.flags.window_hidden {
            self.show_window(true);
        } else if self.flags.wait_sound {
            h.submit_audio(self.client, AudioRequest::Stop { channel: SOUND_CHANNEL, fade_ms: 0.0 })?;
            self.flags.wait_sound = false;
        } else if self.flags.wait_voice {
            h.submit_audio(self.client, AudioRequest::Stop { channel: VOICE_CHANNEL, fade_ms: 0.0 })?;
            self.flags.wait_voice = false;
        } else if self.flags.wait_click {
            self.flags.wait_click = false;
        } else if self.flags.wait_animation {
            self.flags.complete_animations = true;
        }
        Ok(())
    }
    pub(crate) fn show_window(&mut self, visible: bool) {
        self.message.hidden = !visible;
        self.flags.window_hidden = !visible;
        self.dirty = true;
    }

    /// Button callbacks by mode 0 (result), 1 (native screen), 2 (interrupt).
    pub(crate) fn button_pressed(&mut self, mode: u16, id: i32) {
        match mode {
            0 => {
                if self.flags.button_wait == 0 {
                    return;
                }
                self.flags.button_wait = 0;
                if self.flags.window_hidden {
                    self.show_window(true);
                }
                self.mem.set_var(0, id as i16);
            }
            1 => {}
            _ => {
                if !self.flags.button_timer {
                    if self.flags.wait_click {
                        self.flags.wait_click = false;
                    }
                    self.flags.button_wait = 0;
                    self.flags.interrupt = true;
                    self.mem.set_var(0, id as i16);
                }
            }
        }
    }

    /// Stops every audio channel.
    pub fn stop_all_audio(&mut self, h: &mut dyn HostServices) -> Result<()> {
        for channel in [MUSIC_CHANNEL, MUSIC_CHANNEL + 1, SOUND_CHANNEL, SOUND_CHANNEL + 1, SOUND_CHANNEL + 2, VOICE_CHANNEL] {
            h.submit_audio(self.client, AudioRequest::Stop { channel, fade_ms: 0.0 })?;
        }
        Ok(())
    }
}

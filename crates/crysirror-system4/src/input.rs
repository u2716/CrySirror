// Copyright (C) 2019 Nunuhara Cabbage; Rust (C) 2026 CrySirror contributors.
// GPL-2.0-or-later. xsystem4 a85467d src/input.c key-state policy.
//! Native clearable key flags over CrySirror's client-owned input observation.
//! Unrepresented keys/focus/pointer remain explicit unavailable capabilities.
use anyhow::{ensure, Context, Result};
use crysirror_core::service::{buttons, InputObservation};
#[derive(Clone, Debug, Default)]
pub struct Input {
    revision: Option<u64>,
    physical: u64,
    flags: u64,
    wheel: i32,
    journal_revision: Option<(u64, u64)>,
    modal_held: u64,
}
impl Input {
    pub fn observe(&mut self, observation: InputObservation) {
        if self.revision == Some(observation.revision) {
            return;
        }
        self.apply(observation.buttons_down, observation.buttons_pressed);
        self.revision = Some(observation.revision);
    }
    pub(crate) fn observe_sample(
        &mut self,
        epoch: u64,
        sample: crysirror_core::input_buffer::InputSample,
    ) {
        if self.journal_revision == Some((epoch, sample.sequence)) {
            return;
        }
        self.apply(sample.frame.buttons_down, sample.frame.buttons_pressed);
        self.journal_revision = Some((epoch, sample.sequence));
    }
    pub(crate) fn suppress_modal_buttons(&mut self, down: u64) {
        self.modal_held |= down;
        self.flags = 0;
        self.wheel = 0;
        self.physical = 0;
        self.revision = None;
        self.journal_revision = None;
    }
    fn apply(&mut self, down: u64, pressed: u64) {
        self.modal_held &= down;
        let down = down & !self.modal_held;
        let pressed = pressed & !self.modal_held;
        self.flags = down & (self.flags | pressed | !self.physical);
        self.physical = down;
        if pressed & buttons::WHEEL_UP != 0 {
            self.wheel = 1;
        }
        if pressed & buttons::WHEEL_DOWN != 0 {
            self.wheel = -1;
        }
    }
    pub fn key(&self, code: i32) -> Result<i32> {
        if !(0..256).contains(&code) {
            return Ok(0);
        }
        let mask = buttons::windows_virtual_key(code as u32)
            .context("System4 native key is not represented by shared input profile")?;
        Ok(i32::from(self.flags & mask != 0))
    }
    pub fn clear(&mut self, no_ctrl: bool) {
        self.flags &= if no_ctrl { buttons::CTRL } else { 0 };
    }
    pub fn clear_wheel(&mut self) {
        self.wheel = 0;
    }
    pub fn wheel(&self) -> [i32; 2] {
        [i32::from(self.wheel > 0), i32::from(self.wheel < 0)]
    }
    pub fn pointer(observation: InputObservation) -> Result<([i32; 2], i32)> {
        let position = observation
            .pointer
            .context("System4 pointer position unavailable")?;
        let inside = observation
            .pointer_inside
            .context("System4 pointer focus unavailable")?;
        let focused = observation
            .focused
            .context("System4 keyboard focus unavailable")?;
        Ok((position, i32::from(inside && focused)))
    }
    pub fn validate_scope(
        observation: InputObservation,
        client: crysirror_core::state::ClientId,
    ) -> Result<()> {
        ensure!(
            observation.client == client,
            "foreign System4 input observation"
        );
        Ok(())
    }
}

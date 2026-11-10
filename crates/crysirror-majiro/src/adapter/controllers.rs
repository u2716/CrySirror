// Copyright (c) 2026 CrySirror contributors. MIT.
//! YUKIIRO4263A0/458220/457B10 joystick policy over source-aware host samples.
//! The host owns devices. We synthesize only client-local native key records;
//! no keybd_event or physical keyboard state is changed.
use super::*;
use crysirror_core::service::{
    ControllerAxis as A, ControllerButton as B, ControllerInputSample, InputSourcesFrame,
};
pub(super) struct Joystick {
    pub mode: i32,
    held: [bool; 148],
    physical_held: [bool; 148],
    physical_pressed: [bool; 148],
    polled_at: Option<u64>,
}
impl Default for Joystick {
    fn default() -> Self {
        Self {
            mode: 0,
            held: [false; 148],
            physical_held: [false; 148],
            physical_pressed: [false; 148],
            polled_at: None,
        }
    }
}
fn state(pad: &ControllerInputSample) -> u8 {
    let button = |b: B| pad.buttons[b as usize] >= 0.5;
    // Portable face order South/East/West/North supplies native buttons1..4.
    // Native aliases1|4 to cancel and2|8 to confirm are retained, independent
    // of the host's generic south/confirm navigation binding.
    let mut state = 0;
    if button(B::South) || button(B::West) {
        state |= 1;
    }
    if button(B::East) || button(B::North) {
        state |= 2;
    }
    // WinMM thresholds2AAA/D554 over0..65535 correspond to +/-2/3. Hats
    // are digital inputs and retain the same logical directions.
    let x = pad.axes[A::LeftX as usize];
    let y = pad.axes[A::LeftY as usize];
    if x < -2.0 / 3.0 || button(B::DpadLeft) || pad.axes[A::DpadX as usize] < -0.5 {
        state |= 4;
    }
    if x > 2.0 / 3.0 || button(B::DpadRight) || pad.axes[A::DpadX as usize] > 0.5 {
        state |= 8;
    }
    if y > 2.0 / 3.0 || button(B::DpadUp) || pad.axes[A::DpadY as usize] > 0.5 {
        state |= 16;
    }
    if y < -2.0 / 3.0 || button(B::DpadDown) || pad.axes[A::DpadY as usize] < -0.5 {
        state |= 32;
    }
    state
}
fn aliases(key: usize) -> &'static [usize] {
    match key {
        27 => &[27, 131],
        13 => &[13, 130],
        38 => &[38, 132],
        40 => &[40, 133],
        37 => &[37, 135],
        39 => &[39, 134],
        _ => &[],
    }
}
fn map(keys: &mut [bool; 148], bits: u8, order: [usize; 6]) {
    for (bit, key) in [1, 2, 16, 32, 4, 8].into_iter().zip(order) {
        if bits & bit != 0 {
            keys[key] = true;
            for &alias in aliases(key) {
                keys[alias] = true;
            }
        }
    }
}
impl syscalls::Input {
    pub(super) fn update_sources(&mut self, sources: &InputSourcesFrame, now: u64) {
        self.update(&sources.keyboard_pointer);
        self.joystick.physical_held = self.held;
        self.joystick.physical_pressed = self.pressed;
        let previous = self.joystick.held;
        if self
            .joystick
            .polled_at
            .is_none_or(|at| now.saturating_sub(at) >= 10)
        {
            self.joystick.polled_at = Some(now);
            let first = sources.controllers.first().map(state).unwrap_or(0);
            let second = sources.controllers.get(1).map(state).unwrap_or(0);
            let mut next = [false; 148];
            match self.joystick.mode {
                1 => map(&mut next, first | second, [27, 13, 38, 40, 37, 39]),
                2 => {
                    map(&mut next, first, [90, 88, 73, 77, 74, 76]);
                    map(&mut next, second, [39, 37, 104, 98, 100, 102]);
                }
                _ => {}
            }
            self.joystick.held = next;
        }
        for key in 0..148 {
            let held = self.joystick.held[key];
            self.held[key] |= held;
            if held && !previous[key] {
                self.pressed[key] = true;
                if key < 128 {
                    self.last_key = key as i32;
                    self.mouse_mode = false;
                }
            }
        }
    }
    pub(super) fn set_joystick_mode(&mut self, mode: i32) {
        // Native458220 releases all synthesized keys on every call, even if
        // mode is unchanged. Physical keyboard holds/consumed edges survive.
        for key in 0..148 {
            if self.joystick.held[key] {
                self.held[key] = self.joystick.physical_held[key];
                self.pressed[key] &= self.joystick.physical_pressed[key];
            }
        }
        self.joystick.held.fill(false);
        self.joystick.polled_at = None;
        self.joystick.mode = mode;
    }
}

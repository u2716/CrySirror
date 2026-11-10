// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Read-only owner observations. Unavailable pointer/focus metadata stays
//! unknown; a zero coordinate is a position only when the backend supplied it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PointerObservation {
    pub position: Option<[i32; 2]>,
    pub inside: Option<bool>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputObservation {
    pub client: crate::state::ClientId,
    /// Live host generation. Never restored from a game/framework snapshot.
    pub revision: u64,
    pub time_ms: u64,
    pub buttons_down: u64,
    pub buttons_pressed: u64,
    pub pointer: Option<[i32; 2]>,
    pub pointer_inside: Option<bool>,
    pub focused: Option<bool>,
}

/// Stable normalized button order for source-aware compatibility/input APIs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ControllerButton {
    South,
    East,
    West,
    North,
    LeftShoulder,
    RightShoulder,
    LeftTrigger,
    RightTrigger,
    Select,
    Start,
    Guide,
    LeftStick,
    RightStick,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
}
/// Stick/hat Y uses positive-up, X uses positive-right; axes retain -1..1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ControllerAxis {
    LeftX,
    LeftY,
    RightX,
    RightY,
    LeftTrigger,
    RightTrigger,
    DpadX,
    DpadY,
}
/// One connected logical device; IDs and samples are host facts, never saves.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ControllerInputSample {
    pub id: u32,
    /// Pressures in ControllerButton order, bounded0..1.
    pub buttons: [f32; 17],
    /// Positions in ControllerAxis order, bounded-1..1.
    pub axes: [f32; 8],
}
/// Owner-published physical sources before generic UI/controller navigation is
/// merged. Up to16 controllers; no OS handles or device pointers are exposed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InputSourcesFrame {
    pub keyboard_pointer: super::InputFrame,
    pub controller_backend: bool,
    pub controllers: alloc::vec::Vec<ControllerInputSample>,
}
impl InputSourcesFrame {
    pub fn validate(&self) -> crate::Result<()> {
        if self.controllers.len() > 16 || (!self.controller_backend && !self.controllers.is_empty())
        {
            return Err(crate::Error::Invalid(
                "input source controller availability/budget",
            ));
        }
        for (i, pad) in self.controllers.iter().enumerate() {
            if self.controllers[..i].iter().any(|other| other.id == pad.id)
                || pad
                    .buttons
                    .iter()
                    .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
                || pad
                    .axes
                    .iter()
                    .any(|v| !v.is_finite() || !(-1.0..=1.0).contains(v))
            {
                return Err(crate::Error::Invalid("input source controller sample"));
            }
        }
        Ok(())
    }
}
/// Read-only sources for the last admitted owner tick, scoped like pointer
/// observations. The query does not poll hardware, consume edges or advance time.
#[derive(Clone, Debug, PartialEq)]
pub struct InputSourcesObservation {
    pub client: crate::state::ClientId,
    pub revision: u64,
    pub time_ms: u64,
    pub sources: InputSourcesFrame,
}

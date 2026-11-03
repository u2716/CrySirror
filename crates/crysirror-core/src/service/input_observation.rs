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

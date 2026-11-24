// SPDX-License-Identifier: MPL-2.0
//! The player's animation state machine, `dc1.psarc/anim-player.bin`.
//!
//! The module's `anim-actor` entry `anim-player` holds the state count at
//! +0x30, an inline state array at +0x34 and the initial state's
//! `StringId` at +0x38 (`idle`). A state is 0x48 bytes: name (string,
//! `StringId`) at +0, its blend tree at +0x10, the logical animation name
//! at +0x14 (`move` for `run`, `idle^move-b` for `idle-run-f`,
//! `move^idle2` for `run-idle`; each is also a clip of the player rig) and
//! its `anim-transition-group` at +0x30: `{count, array}` of 0x24-byte
//! transitions `{request, flags, blend record, target state, 0, 0, fade
//! seconds, -1, 0}`. The request `auto` fires when the state's animation
//! ends (`idle-run-f` → `run` in 0.2 s, `run-idle` → `idle` in 2 s) and a
//! state whose `auto` targets itself loops (`run`).
//!
//! The blend trees at +0x10 are evaluated by [`crate::anim_tree`]; the
//! player's controller is [`crate::player`].
use crate::dc::{Dc, sid};
use anyhow::{Context, Result, ensure};

#[derive(Clone, Debug, PartialEq)]
pub struct Transition {
    pub request: u32,
    pub target: u32,
    pub fade: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub name: String,
    pub id: u32,
    /// Logical animation `StringId`.
    pub animation: u32,
    /// Blend-tree node address in the module (see [`crate::anim_tree`]).
    pub tree: u32,
    pub transitions: Vec<Transition>,
}

impl State {
    pub fn transition(&self, request: u32) -> Option<&Transition> {
        self.transitions.iter().find(|t| t.request == request)
    }
    /// The automatic transition taken when the animation ends.
    pub fn auto(&self) -> Option<&Transition> {
        self.transition(sid("auto"))
    }
}

#[derive(Clone, Debug, Default)]
pub struct StateTable {
    pub states: Vec<State>,
    pub initial: u32,
}

impl StateTable {
    pub fn get(&self, id: u32) -> Option<&State> {
        self.states.iter().find(|s| s.id == id)
    }
}

/// Parse `anim-player.bin`.
pub fn parse(bytes: &[u8]) -> Result<StateTable> {
    let mut dc = Dc::default();
    dc.load(sid("anim-player"), bytes)?;
    parse_loaded(&dc)
}

/// The states of an `anim-player` module already loaded into `dc`.
pub fn parse_loaded(dc: &Dc) -> Result<StateTable> {
    let module = dc.module(sid("anim-player")).context("anim-player module not loaded")?;
    let m = &dc.mem;
    let actor = module.entries.iter().map(|(_, a)| *a).find(|a| m.type_of(*a).ok() == Some(sid("anim-actor"))).context("anim-player.bin without anim-actor")?;
    let count = m.u32(actor + 0x30)?;
    ensure!(count <= 4096, "ND anim state count");
    let array = m.u32(actor + 0x34)?;
    let initial = m.u32(actor + 0x38)?;
    let mut states = Vec::with_capacity(count as usize);
    for i in 0..count {
        let s = array + 0x48 * i;
        let name = m.u32(s).ok().filter(|p| *p != 0).and_then(|p| m.cstr(p).ok()).unwrap_or_default();
        let mut transitions = Vec::new();
        let group = m.u32(s + 0x30)?;
        if group != 0 {
            let n = m.u32(group)?;
            ensure!(n <= 256, "ND anim transition count");
            let at = m.u32(group + 4)?;
            for k in 0..n {
                let e = at + 0x24 * k;
                transitions.push(Transition { request: m.u32(e)?, target: m.u32(e + 0xc)?, fade: m.f32(e + 0x18)? });
            }
        }
        states.push(State { name, id: m.u32(s + 4)?, animation: m.u32(s + 0x14)?, tree: m.u32(s + 0x10)?, transitions });
    }
    Ok(StateTable { states, initial })
}

/// A running instance: the current state, the time in it and the state
/// being faded out.
#[derive(Clone, Debug)]
pub struct Machine {
    pub state: u32,
    pub time: f32,
    pub previous: Option<(u32, f32)>,
    /// Fade duration of the last change, seconds.
    pub fade: f32,
}

impl Machine {
    pub fn new(table: &StateTable) -> Self {
        Self { state: table.initial, time: 0., previous: None, fade: 0. }
    }

    /// Enter `target` directly (no transition of the current state names
    /// it), fading over `fade` seconds.
    pub fn force(&mut self, target: u32, fade: f32) {
        self.enter(target, fade);
    }

    fn enter(&mut self, target: u32, fade: f32) {
        self.previous = Some((self.state, self.time));
        self.state = target;
        self.time = 0.;
        self.fade = fade.max(0.);
    }

    /// Request a transition by name; true when the current state has it.
    pub fn request(&mut self, table: &StateTable, request: u32) -> bool {
        let Some(t) = table.get(self.state).and_then(|s| s.transition(request)).cloned() else { return false };
        if table.get(t.target).is_none() {
            return false;
        }
        self.enter(t.target, t.fade);
        true
    }

    /// Advance; `duration` is the current animation's length. At the end
    /// the `auto` transition is taken (a self-loop restarts the clip).
    /// Returns true when the state changed or restarted.
    pub fn advance(&mut self, table: &StateTable, seconds: f32, duration: f32) -> bool {
        self.time += seconds;
        if duration <= 0. || self.time < duration {
            return false;
        }
        match table.get(self.state).and_then(State::auto).cloned() {
            Some(t) if t.target == self.state => {
                self.time %= duration.max(1e-3);
                false
            }
            Some(t) if table.get(t.target).is_some() => {
                self.enter(t.target, t.fade);
                true
            }
            _ => {
                self.time = duration;
                false
            }
        }
    }
}

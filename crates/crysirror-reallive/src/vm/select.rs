// SPDX-License-Identifier: MPL-2.0
// Adopted from siglus_rs 256c398 crates/reallive/src/select.rs (MPL-2.0).
//! Button-object selections (`select_objbtn`, `select_btnobj*`). The
//! window/button selections of `select`/`select_s` are the adapter's shared
//! UI widgets; these work on the VM's own button objects, whose layers the
//! adapter projects.

use anyhow::Result;

use crate::vm::machine::{LongOp, Machine};
use crate::vm::rect::Rect;
use crate::vm::sys_state::{Button, InputEvent, Key, Sys};

/// `select_objbtn(group)`: waits for a click on a button object.
#[derive(Debug)]
pub struct ObjectButtonSelect {
    group: i32,
    cancelable: bool,
    hovered: Option<(usize, i32)>,
    pressed: bool,
}

impl ObjectButtonSelect {
    pub fn new(group: i32, cancelable: bool) -> Self {
        Self {
            group,
            cancelable,
            hovered: None,
            pressed: false,
        }
    }

    fn buttons(&self, sys: &mut Sys) -> Vec<(usize, i32, Rect)> {
        group_buttons(sys, self.group)
    }

    fn set_state(sys: &mut Sys, slot: usize, state: i32) {
        set_button_state(sys, slot, state);
    }
}

/// Button objects of a group, topmost first: (object slot, button number,
/// bounds).
fn group_buttons(sys: &mut Sys, group: i32) -> Vec<(usize, i32, Rect)> {
    let now = sys.now();
    let gfx = &sys.gfx;
    let text_size = gfx.text_size();
    let mut out = Vec::new();
    for (slot, object) in gfx.fg.iter().enumerate() {
        let Some(object) = object else { continue };
        let b = &object.params.button;
        if b.is_button == 0 || b.group != group || !object.params.visible {
            continue;
        }
        let bounds = object.bounds(now, &text_size, (0, 0));
        out.push((slot, b.number, bounds));
    }
    out.sort_by_key(|&(slot, ..)| {
        let p = &gfx.fg[slot].as_ref().expect("present").params;
        std::cmp::Reverse((p.z_order, p.z_layer, p.z_depth, slot))
    });
    out
}

fn set_button_state(sys: &mut Sys, slot: usize, state: i32) {
    if let Some(object) = sys.gfx.fg.get_mut(slot).and_then(Option::as_mut) {
        if object.params.button.state != state {
            object.params.button.state = state;
            sys.gfx.dirty = true;
        }
    }
}

/// `select_btnobjstart`..`select_btnobjend`: a button-object selection
/// the script polls (`select_btnobjnow_hit`, `select_btnobjnow_decide`)
/// instead of waiting for it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PolledButtons {
    pub group: i32,
    hovered: Option<(usize, i32)>,
    /// The button held down, and whether with the right button.
    pushed: Option<(i32, bool)>,
    /// A click on a button not yet collected by `now_decide`, and whether
    /// it was a right click.
    decided: Option<(i32, bool)>,
    /// `select_btnobjstart_with_right_click`: right clicks work buttons too.
    pub right_click: bool,
}

impl PolledButtons {
    pub fn new(group: i32) -> Self {
        Self {
            group,
            ..Self::default()
        }
    }

    /// Per frame: hover highlighting and clicks.
    pub fn update(&mut self, sys: &mut Sys) {
        let buttons = group_buttons(sys, self.group);
        let (mx, my) = sys.input.mouse;
        let hit = buttons
            .iter()
            .find(|(_, _, r)| mx >= r.x && my >= r.y && mx < r.right() && my < r.bottom())
            .map(|&(slot, number, _)| (slot, number));
        if hit != self.hovered {
            if let Some((slot, _)) = self.hovered {
                set_button_state(sys, slot, 0);
            }
            if let Some((slot, _)) = hit {
                sys.play_se(0);
                set_button_state(sys, slot, 1);
            }
            self.hovered = hit;
        }
        let Some((slot, number)) = hit else {
            self.pushed = None;
            return;
        };
        if self.pushed.is_some_and(|(n, _)| n != number) {
            self.pushed = None;
        }
        let mut rest = Vec::new();
        for event in std::mem::take(&mut sys.input.events) {
            let right = match event {
                InputEvent::Press(Button::Left) | InputEvent::Release(Button::Left) => false,
                InputEvent::Press(Button::Right) | InputEvent::Release(Button::Right)
                    if self.right_click =>
                {
                    true
                }
                other => {
                    rest.push(other);
                    continue;
                }
            };
            if matches!(event, InputEvent::Press(_)) {
                set_button_state(sys, slot, 2);
                self.pushed = Some((number, right));
            } else if self.decided.is_none() {
                set_button_state(sys, slot, 1);
                sys.play_se(1);
                self.pushed = None;
                self.decided = Some((number, right));
            }
        }
        sys.input.events = rest;
    }

    pub fn hovered(&self) -> i32 {
        self.hovered.map_or(-1, |(_, number)| number)
    }

    /// The button held down (`None`: either mouse button, else only the
    /// left or right one), or -1.
    pub fn pushed(&self, right: Option<bool>) -> i32 {
        match self.pushed {
            Some((number, r)) if right.is_none_or(|want| want == r) => number,
            _ => -1,
        }
    }

    /// Collects a click (`None`: with either button), or -1.
    pub fn take_decided(&mut self, right: Option<bool>) -> i32 {
        match self.decided {
            Some((number, r)) if right.is_none_or(|want| want == r) => {
                self.decided = None;
                number
            }
            _ => -1,
        }
    }

    /// Returns the highlighted button to normal.
    pub fn finish(&mut self, sys: &mut Sys) {
        if let Some((slot, _)) = self.hovered.take() {
            set_button_state(sys, slot, 0);
        }
    }
}

impl LongOp for ObjectButtonSelect {
    fn step(&mut self, machine: &mut Machine) -> Result<bool> {
        let sys = &mut machine.sys;
        let buttons = self.buttons(sys);
        let (mx, my) = sys.input.mouse;
        let hit = buttons
            .iter()
            .find(|(_, _, r)| mx >= r.x && my >= r.y && mx < r.right() && my < r.bottom())
            .map(|&(slot, number, _)| (slot, number));
        if hit != self.hovered {
            if let Some((slot, _)) = self.hovered {
                Self::set_state(sys, slot, 0);
            }
            if let Some((slot, _)) = hit {
                sys.play_se(0);
                Self::set_state(sys, slot, 1);
            }
            self.hovered = hit;
        }
        let events = std::mem::take(&mut sys.input.events);
        let mut rest = Vec::new();
        let mut result = None;
        for event in events {
            match event {
                InputEvent::Press(Button::Left) => {
                    self.pressed = true;
                    if let Some((slot, _)) = hit {
                        Self::set_state(sys, slot, 2);
                    }
                }
                InputEvent::Release(Button::Left) => {
                    self.pressed = false;
                    if let Some((slot, number)) = hit {
                        Self::set_state(sys, slot, 0);
                        result = Some(number);
                    }
                }
                InputEvent::Press(Button::Right) | InputEvent::KeyDown(Key::Escape)
                    if self.cancelable =>
                {
                    result = Some(-1);
                }
                other => rest.push(other),
            }
            if result.is_some() {
                break;
            }
        }
        sys.input.events = rest;
        match result {
            Some(number) => {
                if number >= 0 {
                    sys.play_se(1);
                }
                if let Some((slot, _)) = self.hovered {
                    Self::set_state(sys, slot, 0);
                }
                machine.store = number;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    fn name(&self) -> &'static str {
        "select_objbtn"
    }
}

//! Native key-code modes (5C8074) used by skey bindings, inp\k2/kl2 and the
//! click checks.
//!
//! A code is `mode | modifiers | key`: key = low 12 bits (Windows virtual
//! key, 0xF0/0xF1 wheel, 0x100.. gesture keys), modifiers 0x1000 Ctrl,
//! 0x2000 Alt, 0x4000 Shift, and mode = bits 16..19:
//!   0/8 pressed (5C7720), 1/9 click release (5C797C), 2/A pressed or held
//!   for KeyCallbackTime (5C7CBC), 3/B held for KeyCallbackTime (5C7AE4),
//!   4 down (5C7824), 5 released (5C775C), C down without drag (5C7854);
//!   other modes are never active.
//! Every primitive first requires the exact modifier set (5C7798) unless the
//! code names a modifier key itself without modifier bits.
//! Gesture keys (5C6AFC): while a gesture-enabled button drags, 0x100+dir
//! (left), 0x108+dir (right) and 0x110+dir (middle) are down for the frame
//! with the eight-way direction from the press point (5C7B10); 0x120 is the
//! gesture flag and 0x121 the swipe latch.
use super::*;
use crysirror_core::{
    key_hold::KeyHoldClock,
    pointer_swipe::direction,
    service::buttons as B,
};

#[derive(Default)]
pub(super) struct NativeKeys {
    pub buttons: KeyHoldClock,
    gestures: KeyHoldClock,
    /// 70A3CC device mask: cfg MouseGestureEnable & the thread's inp\mge.
    gesture_mask: u8,
    /// 8A8: a gesture-enabled button is dragging.
    gesture_active: bool,
    hold_ms: u64,
}
const GESTURE_FLAG: u64 = 1 << 24;
const SWIPE_LATCH: u64 = 1 << 25;
fn gesture_bit(key: i32) -> Option<u64> {
    match key {
        0x100..=0x117 => Some(1 << (key - 0x100)),
        0x120 => Some(GESTURE_FLAG),
        0x121 => Some(SWIPE_LATCH),
        _ => None,
    }
}
#[derive(Clone, Copy)]
enum Edge {
    Pressed,
    Released,
    Down,
    Hold,
}
impl QlieVm {
    /// Per-frame device step after the swipe tracker (5C6AFC, 70A6E0).
    pub(super) fn advance_keys(&mut self, now: u64, gesture_mask: u8) {
        let wheel = self.input.buttons_pressed & (B::WHEEL_UP | B::WHEEL_DOWN);
        self.keys
            .buttons
            .step(self.input.buttons_down | wheel, self.input.buttons_pressed, now);
        self.keys.gesture_mask = gesture_mask & 7;
        self.keys.hold_ms = self.engine_setting("kct").max(50) as u64;
        let held = self.swipe.held() & self.keys.gesture_mask;
        let mut down = 0u64;
        if self.keys.gesture_mask == 0 || held == 0 {
            self.keys.gesture_active = false;
        } else if self.swipe.dragging() {
            self.keys.gesture_active = true;
            let dir = direction(self.swipe.press(), self.swipe.pointer(), true) as u32;
            for (bit, base) in [(1u8, 0u32), (2, 8), (4, 16)] {
                if held & bit != 0 {
                    down |= 1 << (base + dir);
                }
            }
        }
        if self.keys.gesture_active {
            down |= GESTURE_FLAG;
        }
        if self.swipe.latched() {
            down |= SWIPE_LATCH;
        }
        self.keys.gestures.step(down, 0, now);
    }
    /// Button (0 left, 1 right, 2 middle) of an active gesture drag.
    pub(super) fn gesture_button(&self) -> Option<usize> {
        if !self.keys.gesture_active {
            return None;
        }
        let held = self.swipe.held() & self.keys.gesture_mask;
        [1u8, 2, 4].iter().position(|b| held & b != 0)
    }
    pub(super) fn consume_key_edges(&mut self) {
        self.keys.buttons.clear_edges();
        self.keys.gestures.clear_edges();
    }
    /// 5C7798: the held Ctrl/Alt/Shift set must equal the code's bits.
    fn modifiers_match(&self, code: i32) -> bool {
        let down = self.keys.buttons.down();
        let mut held = 0;
        if down & B::CTRL != 0 {
            held |= 0x1000;
        }
        if down & B::ALT != 0 {
            held |= 0x2000;
        }
        if down & B::SHIFT != 0 {
            held |= 0x4000;
        }
        let wanted = code & 0xF000;
        held == wanted
            || (wanted == 0 && matches!(code & 0xFFF, 0x10..=0x12 | 0xA0..=0xA5))
    }
    fn key_edge(&self, code: i32, edge: Edge) -> Result<bool> {
        if !self.modifiers_match(code) {
            return Ok(false);
        }
        let key = code & 0xFFF;
        let (clock, mask) = match gesture_bit(key) {
            Some(bit) => (&self.keys.gestures, bit),
            None => (&self.keys.buttons, objects::native_key(key)?),
        };
        Ok(match edge {
            Edge::Pressed => clock.pressed() & mask != 0,
            Edge::Released => clock.released() & mask != 0,
            Edge::Down => clock.down() & mask != 0,
            Edge::Hold => clock.held_for(mask, self.keys.hold_ms) != 0,
        })
    }
    /// 5C797C/5C7854: gesture keys also need their button; plain mouse
    /// buttons are suppressed after a drag, a gesture or a swipe.
    fn key_without_drag(&self, code: i32, edge: Edge) -> Result<bool> {
        let key = code & 0xFFF;
        for (bit, range) in [(1u8, 0x100..=0x107), (2, 0x108..=0x10F), (4, 0x110..=0x117)] {
            if self.keys.gesture_mask & bit != 0 && range.contains(&key) {
                return Ok(self.key_edge(code, edge)? && self.key_edge(i32::from(bit), edge)?);
            }
        }
        if matches!(key, 1 | 2 | 4) {
            return Ok(self.key_edge(code, edge)?
                && !self.swipe.dragging()
                && !self.keys.gesture_active
                && !self.swipe.latched());
        }
        self.key_edge(code, edge)
    }
    /// 5C8074 for one native key code at the current frame.
    pub(super) fn native_key_active(&self, code: i32) -> Result<bool> {
        match code & 0xF0000 {
            0x00000 | 0x80000 => self.key_edge(code, Edge::Pressed),
            0x10000 | 0x90000 => self.key_without_drag(code, Edge::Released),
            0x20000 | 0xA0000 => {
                Ok(self.key_edge(code, Edge::Pressed)? || self.key_edge(code, Edge::Hold)?)
            }
            0x30000 | 0xB0000 => self.key_edge(code, Edge::Hold),
            0x40000 => self.key_edge(code, Edge::Down),
            0x50000 => self.key_edge(code, Edge::Released),
            0xC0000 => self.key_without_drag(code, Edge::Down),
            _ => Ok(false),
        }
    }
}

// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Per-frame edge and hold timing over 64-bit button masks.
//!
//! A frame is the current down mask plus press pulses that came and went
//! inside the frame. Each bit remembers the time its current hold began; the
//! time follows the frame clock until the bit is down on two consecutive
//! frames (QLIE 5C6AFC/5C7AA4 key repeat timing), so a hold of `ms` is
//! reported from the first frame at or after `start + ms`.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyHoldClock {
    previous: u64,
    down: u64,
    pressed: u64,
    released: u64,
    since: [u64; 64],
    now: u64,
}
impl Default for KeyHoldClock {
    fn default() -> Self {
        Self {
            previous: 0,
            down: 0,
            pressed: 0,
            released: 0,
            since: [0; 64],
            now: 0,
        }
    }
}
impl KeyHoldClock {
    /// Advance one frame. `pulses` are presses observed during the frame,
    /// including taps already released at its end.
    pub fn step(&mut self, down: u64, pulses: u64, now_ms: u64) {
        self.previous = self.down;
        self.pressed = (down & !self.previous) | pulses;
        self.released = (self.previous & !down) | (pulses & !down);
        let kept = self.previous & down;
        for bit in 0..64 {
            if kept & (1 << bit) == 0 {
                self.since[bit] = now_ms;
            }
        }
        self.down = down;
        self.now = now_ms;
    }
    pub fn down(&self) -> u64 {
        self.down
    }
    pub fn pressed(&self) -> u64 {
        self.pressed
    }
    pub fn released(&self) -> u64 {
        self.released
    }
    /// Forget this frame's edges once a consumer has handled them.
    pub fn clear_edges(&mut self) {
        self.pressed = 0;
        self.released = 0;
    }
    /// Bits of `mask` down for at least `ms` at the current frame.
    pub fn held_for(&self, mask: u64, ms: u64) -> u64 {
        let mut out = 0;
        for bit in 0..64 {
            let b = 1u64 << bit;
            if mask & self.down & b != 0 && self.now >= self.since[bit].saturating_add(ms) {
                out |= b;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn edges_and_hold_time() {
        let mut k = KeyHoldClock::default();
        k.step(1, 0, 100);
        assert_eq!((k.pressed(), k.released(), k.held_for(1, 500)), (1, 0, 0));
        k.step(1, 0, 400);
        assert_eq!((k.pressed(), k.held_for(1, 500)), (0, 0));
        k.step(1, 0, 600);
        assert_eq!(k.held_for(1, 500), 1);
        k.step(0, 0, 616);
        assert_eq!((k.released(), k.held_for(1, 500)), (1, 0));
        // A tap inside one frame is both pressed and released.
        k.step(0, 2, 632);
        assert_eq!((k.pressed(), k.released(), k.down()), (2, 2, 0));
    }
}

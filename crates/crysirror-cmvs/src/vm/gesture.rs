// SPDX-License-Identifier: MPL-2.0

//! Engine +4664 pointer drag/flick recognizer (constructor 0x1400533d0). Scripts configure
//! it (0x2350..0x2364, plus six region records through 0x2366) and query the per-frame
//! hold result (0x2362). The frame update
//! 0x140054a60 runs button gestures (0x140053930), region drag (0x1400541a0),
//! or unrestricted pointer drag (0x140054570). Fields stay named by offset.
use super::{evaluate::error, input::Actions, machine::Machine, DecodeError};
use std::collections::BTreeMap;
mod drag;

/// Tracking fields reset by every setter that shares the tail of 0x1400548e0.
const RESET_MINUS_ONE: [u16; 3] = [44, 88, 172];
const RESET_ZERO: [u16; 11] = [48, 52, 72, 76, 80, 84, 92, 108, 120, 180, 184];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PointerGesture {
    fields: BTreeMap<u16, u32>,
    now: u32,
}

#[cfg(feature = "runtime")]
impl PointerGesture {
    pub(super) fn snapshot(&self) -> serde_json::Value {
        serde_json::json!({"fields": self.fields.iter().collect::<Vec<_>>(), "now": self.now})
    }

    pub(super) fn from_snapshot(value: &serde_json::Value) -> Option<Self> {
        let mut fields = BTreeMap::new();
        for pair in value["fields"].as_array()? {
            fields.insert(pair[0].as_u64()? as u16, pair[1].as_u64()? as u32);
        }
        Some(Self {
            fields,
            now: super::snapshot::word(value, "now")?,
        })
    }
}

impl Default for PointerGesture {
    /// Constructor values for every field the modeled commands and mode 0 touch: +32 = 64
    /// (distance threshold), +36 = 90, +96 = 700 (hold time), +112 = 1500 (rest time);
    /// the memset leaves the anchors +56..+68 and +100/+104 zero.
    fn default() -> Self {
        let mut fields = BTreeMap::from([
            (32, 64),
            (36, 90),
            (40, 4),
            (96, 700),
            (112, 1500),
            (116, 1),
        ]);
        for offset in [0, 4, 8, 12, 16, 20, 24, 28, 56, 60, 64, 68, 100, 104]
            .into_iter()
            .chain([176])
            .chain((124..=168).step_by(4))
            .chain(RESET_ZERO)
            .chain((188..=304).step_by(4))
        {
            fields.insert(offset, 0);
        }
        for offset in RESET_MINUS_ONE {
            fields.insert(offset, u32::MAX);
        }
        Self { fields, now: 0 }
    }
}

/// Mouse flag indices of `Actions::mouse`.
const LEFT: usize = 0;
const RIGHT: usize = 1;
const WHEEL_UP: usize = 2;
const WHEEL_DOWN: usize = 3;
/// `[released, held, pressed]` positions.
const RELEASED: usize = 0;
const HELD: usize = 1;
const PRESSED: usize = 2;

fn measure(from: [i32; 2], to: [i32; 2]) -> (i32, i32) {
    let (angle, distance) = measure_scaled(from, to, 100);
    ((angle + 135) % 360, distance / 100)
}

/// Native 0x140008ab0 normalizes the truncated angle and returns distance in
/// the caller's scaled units. On x64, cvttsd2si yields i32::MIN on overflow.
fn measure_scaled(from: [i32; 2], to: [i32; 2], scale: i32) -> (i32, i32) {
    let scaled = |v: i32| v.wrapping_mul(scale);
    let dx = scaled(to[0]).wrapping_sub(scaled(from[0]));
    let dy = scaled(to[1]).wrapping_sub(scaled(from[1]));
    let theta = f64::from(dy).atan2(f64::from(dx));
    let mut angle = (theta / std::f64::consts::PI * -180.0) as i32;
    if angle < 0 {
        angle += 360;
    }
    angle = 360 - angle;
    if angle == 360 {
        angle = 0;
    }
    let distance = if dx != 0 {
        let distance = (f64::from(dx) / theta.cos()).trunc();
        if (f64::from(i32::MIN)..=f64::from(i32::MAX)).contains(&distance) {
            distance as i32
        } else {
            i32::MIN
        }
    } else {
        dy.wrapping_abs()
    };
    (angle, distance)
}

impl PointerGesture {
    #[cfg(feature = "runtime")]
    pub(super) fn holding(&self) -> bool {
        self.get(88) >= 0
    }
    fn get(&self, offset: u16) -> i32 {
        self.fields.get(&offset).copied().unwrap_or(0) as i32
    }
    fn set(&mut self, offset: u16, value: i32) {
        self.fields.insert(offset, value as u32);
    }
    fn reset_tracking(&mut self) {
        for offset in RESET_MINUS_ONE {
            self.fields.insert(offset, u32::MAX);
        }
        for offset in RESET_ZERO {
            self.fields.insert(offset, 0);
        }
    }
    /// The inline reset after 0x140059560 in 0x140053930: the same fields as the setters'
    /// reset, with the qword at +80 also clearing +84.
    fn reset_after_consume(&mut self, input: &mut Actions) {
        input.clear_all_edges();
        self.reset_tracking();
    }
    /// The disabled branch: +44/+88 = -1, +48/+52/+72/+76/+84/+92/+120 = 0.
    fn idle(&mut self) {
        self.set(44, -1);
        for offset in [48, 52, 72, 76, 84, 92, 120] {
            self.set(offset, 0);
        }
        self.set(88, -1);
    }
    /// One side of 0x140053930: the left button with +24/+48/+72/+56 and directions 0..3,
    /// or the right button with +28/+52/+76/+64 and directions 4..7.
    fn track(&mut self, input: &mut Actions, side: usize, wheel: i32, previous: i32, now: u32) {
        let (active, wheel_hold, anchor, base) = if side == LEFT {
            (48, 72, 56, 0)
        } else {
            (52, 76, 64, 4)
        };
        let flags = input.mouse(side);
        let pointer = input.pointer();
        if self.get(active) == 0 {
            if flags[PRESSED] {
                self.set(anchor, pointer[0]);
                self.set(anchor + 4, pointer[1]);
                self.set(44, -1);
                self.set(active, 1);
                self.set(92, now as i32);
                self.set(100, pointer[0]);
                self.set(104, pointer[1]);
                self.set(108, now as i32);
            }
            return;
        }
        if self.get(wheel_hold) == 0 && wheel != 0 {
            self.set(wheel_hold, 1);
        }
        if self.get(wheel_hold) != 0 {
            if flags[RELEASED] {
                input.clear_all_edges();
                self.set(44, -1);
                self.set(active, 0);
                self.set(wheel_hold, 0);
            } else if flags[HELD] {
                self.set(80, wheel);
                input.clear_edges();
            } else {
                self.set(44, -1);
                self.set(active, 0);
                self.set(wheel_hold, 0);
                self.set(80, 0);
            }
        } else if flags[RELEASED] {
            let (angle, distance) = measure([self.get(anchor), self.get(anchor + 4)], pointer);
            if distance < self.get(32) {
                if previous >= 0 {
                    input.clear_all_edges();
                }
            } else {
                self.set(44, (angle / 90).clamp(0, 3) + base);
                input.clear_all_edges();
            }
            self.set(active, 0);
        } else if !flags[HELD] {
            self.set(44, -1);
            self.set(active, 0);
        }
    }
    /// Frame update 0x140054a60 after the input refresh, preceded by 0x140053510 (+44 = -1).
    /// Mode +4 = 0 runs 0x140053930: +88 (the 0x2362 result) is -1 unless a press tracked by
    /// +48/+52 has lasted +96 ms without the two-button chord (+84). It then holds 98/99, or
    /// 20/21 after a wheel step during the hold, or a direction 0..3/4..7 once the pointer is
    /// at least +32 from the anchor. A release ends tracking; a far release sets the flick
    /// direction +44. Flicks, chord releases and wheel holds clear input edges
    /// (0x140059560 / 0x140054bf0). Pointer rest (+112 ms within 3 pixels) moves the anchor;
    /// as in the original, +100 and +104 then both receive the pointer y. `now` is the
    /// native VM frame clock. Modes 1 and 2 run the region and unrestricted
    /// drag recognizers. Other mode values only clear the per-frame flick.
    pub(super) fn frame(&mut self, input: &mut Actions, delta: u32) {
        self.now = self.now.wrapping_add(delta);
        let now = self.now;
        self.set(44, -1);
        match self.get(4) {
            0 => {}
            1 => return self.frame_drag(input, now),
            2 => return self.frame_unrestricted_drag(input, now),
            _ => return,
        }
        let previous = self.get(88);
        self.set(80, 0);
        self.set(88, -1);
        if self.get(0) == 0 {
            self.idle();
            return;
        }
        let mut wheel = i32::from(input.mouse(WHEEL_UP)[RELEASED]);
        if input.mouse(WHEEL_DOWN)[RELEASED] {
            wheel = -1;
        }
        if self.get(84) == 0
            && input.mouse(LEFT)[HELD] as u32 + input.mouse(RIGHT)[HELD] as u32 >= 2
        {
            self.set(84, 1);
        }
        if self.get(84) != 0 {
            let mut held = u32::from(input.mouse(LEFT)[HELD]);
            if input.mouse(LEFT)[RELEASED] {
                self.reset_after_consume(input);
            }
            held += u32::from(input.mouse(RIGHT)[HELD]);
            if input.mouse(RIGHT)[RELEASED] {
                self.reset_after_consume(input);
            }
            if wheel != 0 {
                self.reset_after_consume(input);
            }
            if held == 0 {
                self.set(84, 0);
            }
        }
        if [8, 12, 16, 20].iter().any(|&offset| self.get(offset) == 0) {
            self.idle();
            return;
        }
        if self.get(24) != 0 {
            self.track(input, LEFT, wheel, previous, now);
        }
        if self.get(28) != 0 {
            self.track(input, RIGHT, wheel, previous, now);
        }
        let tracking = self.get(48) != 0 || self.get(52) != 0;
        if tracking {
            let [x, y] = input.pointer();
            if x.wrapping_sub(self.get(100)).wrapping_abs() >= 3
                || y.wrapping_sub(self.get(104)).wrapping_abs() >= 3
            {
                self.set(100, x);
                self.set(104, y);
                self.set(108, now as i32);
            }
            if now.wrapping_sub(self.get(108) as u32) >= self.get(112) as u32 {
                for (active, anchor) in [(48, 56), (52, 64)] {
                    if self.get(active) != 0 {
                        self.set(anchor, x);
                        self.set(anchor + 4, y);
                        self.set(44, -1);
                    }
                }
                self.set(100, y);
                self.set(104, y);
                self.set(108, now as i32);
            }
        }
        if self.get(84) == 0
            && tracking
            && now.wrapping_sub(self.get(92) as u32) >= self.get(96) as u32
        {
            let pointer = input.pointer();
            for (active, wheel_hold, anchor, plain, wheeled, base) in
                [(48, 72, 56, 98, 20, 0), (52, 76, 64, 99, 21, 4)]
            {
                if self.get(active) == 0 {
                    continue;
                }
                self.set(88, plain);
                if self.get(wheel_hold) != 0 {
                    self.set(88, wheeled);
                } else {
                    let (angle, distance) =
                        measure([self.get(anchor), self.get(anchor + 4)], pointer);
                    if distance >= self.get(32) {
                        self.set(88, (angle / 90 + base).clamp(base, base + 3));
                    }
                }
            }
        } else {
            self.set(88, -1);
        }
    }

    pub(super) fn command(
        &mut self,
        machine: &mut Machine,
        opcode: u16,
        input: &Actions,
    ) -> Result<Option<u16>, DecodeError> {
        let pc = machine.pc();
        let arg = |i: usize| {
            machine
                .stack()
                .peek_u32(4 * i)
                .map_err(|e| error(pc, opcode, e.0))
        };
        let mut results = Vec::new();
        let words = match opcode {
            0x2350 | 0x2352 | 0x2356..=0x2358 | 0x2364 => {
                let offset = match opcode {
                    0x2350 => 0,
                    0x2352 => 8,
                    0x2364 => 4,
                    _ => 12 + (opcode - 0x2356) * 4,
                };
                let value = arg(1)?;
                self.set(
                    offset,
                    if offset == 4 {
                        value as i32
                    } else {
                        i32::from(value != 0)
                    },
                );
                self.reset_tracking();
                1
            }
            0x2351 => {
                let [left, right] = [arg(1)?, arg(2)?];
                self.set(24, i32::from(left != 0));
                self.set(28, i32::from(right != 0));
                self.reset_tracking();
                2
            }
            // 0x140065820 clears the tracking fields only.
            0x2354 => {
                self.reset_tracking();
                0
            }
            // 0x140066c40: -1 reads the 0x2350 enable, 0..=5 the per-gesture switches.
            0x2355 => {
                let index = arg(1)? as i32;
                let value = match index {
                    ..0 => self.get(0),
                    0..=5 => self.get(8 + 4 * index as u16),
                    _ => 0,
                };
                results.push((0, i32::from(value != 0)));
                1
            }
            0x2353 | 0x2360 => {
                results.push((0, self.get(if opcode == 0x2353 { 44 } else { 88 })));
                0
            }
            0x2359 => {
                let [left, right] = [self.get(72) != 0, self.get(76) != 0];
                if left == right {
                    self.set(80, 0);
                }
                results.extend([(0, self.get(80)), (4, i32::from(right))]);
                0
            }
            0x235a | 0x235b => {
                let value = arg(1)?;
                if opcode == 0x235a && (1..=90).contains(&value) {
                    self.set(36, value as i32);
                } else if opcode == 0x235b && value as i32 >= 0 {
                    self.set(32, value as i32);
                }
                1
            }
            0x2362 => {
                results.push((0, i32::from(arg(1)? == 1 && self.get(88) >= 0)));
                1
            }
            0x2365 => {
                let active = self.get(120) != 0;
                results.extend([(0, i32::from(active)), (6, self.get(176))]);
                if active {
                    results.extend(
                        [(1, 124), (2, 128), (4, 100), (5, 104)]
                            .map(|(slot, offset)| (slot, self.get(offset))),
                    );
                }
                0
            }
            0x2366 => {
                let [slot, enable, x, y, w, h] =
                    [arg(1)?, arg(2)?, arg(3)?, arg(4)?, arg(5)?, arg(6)?];
                if slot < 6 {
                    let base = 188 + slot as u16 * 20;
                    for (i, value) in [u32::from(enable != 0), x, y, w, h].into_iter().enumerate() {
                        self.set(base + i as u16 * 4, value as i32);
                    }
                }
                6
            }
            0x2367 => {
                self.set(172, -1);
                0
            }
            0x2368 => {
                results.extend(
                    [(0, 172), (4, 168), (5, 164), (6, 176)]
                        .map(|(slot, offset)| (slot, self.get(offset))),
                );
                0
            }
            0x2369 => {
                let force = arg(1)? != 0;
                results.push((0, i32::from(self.drag_press(input, force, self.now))));
                1
            }
            0x236a => {
                let elapsed = self.get(164) as u32;
                let speed = (self.get(184) as u32)
                    .wrapping_mul(16)
                    .checked_div(elapsed)
                    .unwrap_or(0);
                results.extend([(0, self.get(172)), (4, self.get(180)), (5, speed as i32)]);
                0
            }
            0x236b => {
                let threshold = arg(1)?;
                if (1..=128).contains(&threshold) {
                    self.set(40, threshold as i32);
                }
                1
            }
            _ => return Ok(None),
        };
        for (slot, value) in results {
            machine
                .state_mut()
                .set_system_integer(slot, value as u32)
                .map_err(|why| error(pc, opcode, why))?;
        }
        Ok(Some(words * 4))
    }
}

// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Pointer drag and fling ("swipe") tracking over per-frame pointer samples.
//!
//! Pure frame-stepped arithmetic, independent of any window system: a press
//! records a start point; holding a button past a distance threshold begins a
//! drag; while dragging the per-frame motion (clamped to +-50 per axis, with a
//! dead zone below a squared speed of 10) is scaled by `16 / frame_ms` (clamped
//! to 0.5..2) and kept in a 32-entry history. On release the strongest recent
//! sample within `permit_frames * 16` ms becomes the fling velocity, which is
//! integrated every 12 ms of accumulated time (the accumulator then drops by
//! 20 ms) and damped by `0.96 * (1 - (0.01 * (100 - intensity))^2)`.
//! These constants follow the QLIE input device (5C6AFC/5C845C) so that any
//! VM can request the same gesture semantics.

/// Button bits: 1 primary, 2 secondary, 4 middle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SwipeButtons(pub u8);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwipeConfig {
    /// Buttons that may drag (bit mask of [`SwipeButtons`]); 0 disables.
    pub buttons: u8,
    /// Keep a fling velocity after release.
    pub inertia: bool,
    /// Distance in pixels before a held button becomes a drag.
    pub threshold: i32,
    /// Recent frames (x16 ms, at most 30) considered for the fling velocity.
    pub permit_frames: i32,
    /// Damping intensity, 0..=100 (higher keeps velocity longer).
    pub intensity: i32,
}
impl Default for SwipeConfig {
    fn default() -> Self {
        Self {
            buttons: 0,
            inertia: false,
            threshold: 30,
            permit_frames: 6,
            intensity: 80,
        }
    }
}

/// One query result (5C845C layout).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SwipeReport {
    /// Drag start, or -65535 when no drag has started.
    pub start: [i32; 2],
    pub pointer: [i32; 2],
    /// Held drag buttons (bits), -1 while a fling is moving, else 0.
    pub state: i32,
    /// Dominant direction from the press point: 0 up, 1 down, 2 left, 3 right;
    /// -1 when there is no drag/fling.
    pub direction: i32,
    /// This frame's pointer delta during a drag, or the fling displacement.
    pub delta: [i32; 2],
}

#[derive(Clone, Debug)]
pub struct SwipeTracker {
    pub config: SwipeConfig,
    pointer: [i32; 2],
    previous: [i32; 2],
    press: [i32; 2],
    start: [i32; 2],
    dragging: bool,
    latched: bool,
    velocity: [f32; 2],
    offset: [f32; 2],
    previous_offset: [f32; 2],
    clock: i32,
    history: [([f32; 2], i32); 32],
    cursor: usize,
    held: u8,
}
const NONE: i32 = -65535;
impl Default for SwipeTracker {
    fn default() -> Self {
        Self {
            config: SwipeConfig::default(),
            pointer: [0; 2],
            previous: [0; 2],
            press: [0; 2],
            start: [NONE; 2],
            dragging: false,
            latched: false,
            velocity: [0.; 2],
            offset: [0.; 2],
            previous_offset: [0.; 2],
            clock: 0,
            history: [([0.; 2], 0); 32],
            cursor: 0,
            held: 0,
        }
    }
}
/// Rounded Euclidean distance: trunc(sqrt(dx^2 + dy^2) + 0.5) (5C7C68).
/// For integer offsets this is the integer square root rounded half up.
fn distance(a: [i32; 2], b: [i32; 2]) -> i32 {
    let dx = i64::from(b[0]) - i64::from(a[0]);
    let dy = i64::from(b[1]) - i64::from(a[1]);
    let squared = (dx * dx + dy * dy) as u64;
    let mut root = isqrt(squared);
    // sqrt(n) + 0.5 >= root + 1  <=>  n >= root^2 + root + 1/4.
    if squared > root * root + root {
        root += 1;
    }
    root.min(i32::MAX as u64) as i32
}
fn isqrt(n: u64) -> u64 {
    if n < 2 {
        return n;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}
/// atan2 in degrees, (-180, 180], from argument-reduced series arithmetic
/// (no platform maths library is required).
fn atan2_degrees(y: f64, x: f64) -> f64 {
    const PI: f64 = core::f64::consts::PI;
    if x == 0. && y == 0. {
        return 0.;
    }
    // atan(t) for |t| <= 1: halve the argument twice, then a Taylor series.
    fn atan_unit(t: f64) -> f64 {
        let mut t = t;
        let mut scale = 1.;
        for _ in 0..3 {
            // atan(t) = 2 atan(t / (1 + sqrt(1 + t^2)))
            t /= 1. + sqrt(1. + t * t);
            scale *= 2.;
        }
        let t2 = t * t;
        let (mut term, mut sum) = (t, 0.);
        for k in 0..20 {
            sum += term / f64::from(2 * k + 1);
            term *= -t2;
        }
        sum * scale
    }
    fn sqrt(v: f64) -> f64 {
        let mut r = if v > 1. { v } else { 1. };
        for _ in 0..64 {
            let next = 0.5 * (r + v / r);
            if next == r {
                break;
            }
            r = next;
        }
        r
    }
    let radians = if x.abs() >= y.abs() {
        let base = atan_unit(y / x);
        if x > 0. {
            base
        } else if y >= 0. {
            base + PI
        } else {
            base - PI
        }
    } else if y > 0. {
        PI / 2. - atan_unit(x / y)
    } else {
        -PI / 2. - atan_unit(x / y)
    };
    radians * 180. / PI
}
/// Screen-space direction from `from` to `to` (5C7B10). The angle is
/// trunc(atan2(-dy, dx) in degrees + 0.5) mod 360. Eight-way returns
/// 0 up, 1 down, 2 left, 3 right, 4 up-left, 5 up-right, 6 down-left,
/// 7 down-right; four-way returns 0..=3 only.
pub fn direction(from: [i32; 2], to: [i32; 2], eight_way: bool) -> i32 {
    let dx = f64::from((to[0] - from[0]) as f32);
    let dy = f64::from((to[1] - from[1]) as f32);
    let degrees = (atan2_degrees(-dy, dx) + 0.5) as i32;
    let angle = (degrees + 3600).rem_euclid(360);
    if eight_way {
        [3, 5, 0, 4, 2, 6, 1, 7][((angle + 22) % 360 / 45) as usize]
    } else {
        [3, 0, 2, 1][((angle + 45) % 360 / 90) as usize]
    }
}

/// Mouse-gesture query (5C7F50).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GestureReport {
    pub press: [i32; 2],
    pub pointer: [i32; 2],
    /// Held buttons while a drag is active, else 0.
    pub buttons: i32,
    /// [`direction`] from press to pointer, or -1 within the drag threshold.
    pub direction: i32,
}
fn speed2(v: [f32; 2]) -> f32 {
    v[0] * v[0] + v[1] * v[1]
}
impl SwipeTracker {
    /// Where the current (or last) single-button press began.
    pub fn press(&self) -> [i32; 2] {
        self.press
    }
    pub fn pointer(&self) -> [i32; 2] {
        self.pointer
    }
    /// Held buttons (bit mask of [`SwipeButtons`]) at the last step.
    pub fn held(&self) -> u8 {
        self.held
    }
    /// The press moved past the threshold; stays set on the release frame.
    pub fn dragging(&self) -> bool {
        self.dragging
    }
    /// The press became a swipe (or caught a fling) under the drag mask.
    pub fn latched(&self) -> bool {
        self.latched
    }
    /// Clear the latched drag start and velocity (5C7DA0).
    pub fn reset_latch(&mut self) {
        self.velocity = [0.; 2];
    }
    fn push(&mut self, sample: [f32; 2], ms: i32) {
        self.history[self.cursor] = (sample, ms);
        self.cursor = (self.cursor + 1) % 32;
    }
    fn pop(&mut self) -> ([f32; 2], i32) {
        self.cursor = (self.cursor + 31) % 32;
        self.history[self.cursor]
    }
    /// Advance one frame. `held` holds the currently pressed buttons, `frame_ms`
    /// the elapsed time since the previous frame.
    pub fn step(&mut self, pointer: [i32; 2], held: SwipeButtons, frame_ms: u32) {
        let ms = if frame_ms > 0xFFFFFF { 1 } else { frame_ms as i32 };
        self.previous = self.pointer;
        self.pointer = pointer;
        let held_now = held.0 & 7;
        let pressed = held_now & !self.held;
        let released = self.held & !held_now;
        self.held = held_now;
        let mask = self.config.buttons & 7;
        if pressed != 0 && held_now.count_ones() < 2 {
            self.press = pointer;
            self.latched = false;
            self.start = [NONE; 2];
            if held_now & mask != 0 && speed2(self.velocity) > 2.5 {
                // Catching a moving fling turns it into a new drag.
                self.latched = true;
                self.dragging = true;
                self.start = pointer;
                self.previous = pointer;
            }
            self.velocity = [0.; 2];
            self.history = [([0.; 2], 0); 32];
            self.cursor = 0;
        }
        if held_now != 0 {
            if distance(self.press, pointer) > self.config.threshold {
                self.dragging = true;
            }
        } else if released == 0 {
            self.dragging = false;
        }
        if mask == 0 {
            self.latched = false;
            self.start = [NONE; 2];
            self.velocity = [0.; 2];
            return;
        }
        if held_now & mask != 0 {
            if self.dragging {
                self.latched = true;
            }
            if self.latched && self.start[0] == NONE {
                self.start = pointer;
            }
            if self.dragging {
                if self.config.inertia {
                    let clamp = |v: i32| (v as f32).clamp(-50., 50.);
                    let mut v = [clamp(pointer[0] - self.previous[0]), clamp(pointer[1] - self.previous[1])];
                    if speed2(v) < 10. {
                        v = [0.; 2];
                    }
                    self.velocity = v;
                } else {
                    self.velocity = [0.; 2];
                }
                self.offset = [0.; 2];
                self.previous_offset = [0.; 2];
                let scale = if ms > 0 { 16. / ms as f32 } else { 1. }.clamp(0.5, 2.);
                self.velocity = self.velocity.map(|v| v * scale);
                self.push(self.velocity, ms);
            }
        } else if self.config.inertia && released & mask != 0 {
            // 5C69A4: strongest recent sample within the permitted window.
            let mut budget = 16 * self.config.permit_frames.clamp(0, 30);
            let mut best = speed2(self.velocity);
            for _ in 0..256 {
                if budget < 1 {
                    break;
                }
                let (sample, sample_ms) = self.pop();
                if sample_ms == 0 {
                    break;
                }
                budget -= sample_ms;
                let s = speed2(sample);
                if s > 10. && s > best {
                    best = s;
                    self.velocity = sample;
                }
            }
        }
        self.previous_offset = self.offset;
        self.clock += ms;
        if self.clock > 1000 {
            self.clock = 100;
        }
        let damping = {
            let d = (100 - self.config.intensity) as f32 * 0.01;
            (0.96 * (1. - d * d)).clamp(0., 0.96)
        };
        while self.clock >= 12 {
            self.offset[0] += self.velocity[0];
            self.offset[1] += self.velocity[1];
            self.velocity = self.velocity.map(|v| v * damping);
            if speed2(self.velocity) < 1. {
                self.velocity = [0.; 2];
            }
            self.clock = (self.clock - 20).max(0);
        }
    }
    fn direction(&self) -> i32 {
        let dx = self.start[0] - self.press[0];
        let dy = self.start[1] - self.press[1];
        if dx.abs() <= dy.abs() {
            i32::from(dy >= 0)
        } else if dx >= 0 {
            3
        } else {
            2
        }
    }
    /// 5C7F50: press/pointer, the held buttons of an active drag (restricted
    /// to the drag mask when `masked`), and the gesture direction.
    pub fn gesture(&self, masked: bool, eight_way: bool) -> GestureReport {
        let mut direction = direction(self.press, self.pointer, eight_way);
        if distance(self.press, self.pointer) <= self.config.threshold {
            direction = -1;
        }
        let buttons = if self.dragging {
            let bits = if masked { self.held & self.config.buttons } else { self.held };
            i32::from(bits & 7)
        } else {
            0
        };
        GestureReport { press: self.press, pointer: self.pointer, buttons, direction }
    }
    /// 5C845C: `masked` restricts the reported buttons to the drag mask.
    pub fn report(&mut self, masked: bool) -> SwipeReport {
        let mut out = SwipeReport {
            start: [0; 2],
            pointer: self.pointer,
            state: 0,
            direction: -1,
            delta: [0; 2],
        };
        let s = speed2(self.velocity);
        if self.start[0] != NONE || s > 0.5 {
            out.start = self.start;
            out.delta = [self.pointer[0] - self.previous[0], self.pointer[1] - self.previous[1]];
            out.direction = self.direction();
        }
        if self.dragging {
            let bits = if masked { self.held & self.config.buttons } else { self.held };
            out.state = i32::from(bits & 7);
        }
        if out.state == 0 && s >= 0.5 {
            out.state = -1;
            out.delta = [
                self.offset[0] as i32 - self.previous_offset[0] as i32,
                self.offset[1] as i32 - self.previous_offset[1] as i32,
            ];
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drag_then_fling_reports_motion_until_damped() {
        let mut t = SwipeTracker::default();
        t.config.buttons = 1;
        t.config.inertia = true;
        t.step([100, 100], SwipeButtons(0), 16);
        t.step([100, 100], SwipeButtons(1), 16);
        for y in 1..=5 {
            t.step([100, 100 + y * 20], SwipeButtons(1), 16);
        }
        let r = t.report(true);
        assert_eq!(r.state, 1);
        assert_eq!(r.delta, [0, 20]);
        assert_eq!(r.direction, 1);
        t.step([100, 200], SwipeButtons(0), 16);
        t.step([100, 200], SwipeButtons(0), 16);
        let r = t.report(true);
        assert_eq!(r.state, -1);
        assert!(r.delta[1] > 0);
        for _ in 0..400 {
            t.step([100, 200], SwipeButtons(0), 16);
        }
        assert_eq!(t.report(true).state, 0);
    }
    #[test]
    fn gesture_directions_and_threshold() {
        assert_eq!(direction([0, 0], [10, 0], true), 3);
        assert_eq!(direction([0, 0], [0, -10], true), 0);
        assert_eq!(direction([0, 0], [-10, 0], true), 2);
        assert_eq!(direction([0, 0], [0, 10], true), 1);
        assert_eq!(direction([0, 0], [-10, -10], true), 4);
        assert_eq!(direction([0, 0], [10, -10], true), 5);
        assert_eq!(direction([0, 0], [-10, 10], true), 6);
        assert_eq!(direction([0, 0], [10, 10], true), 7);
        assert_eq!(direction([0, 0], [10, -9], false), 3);
        // Boundaries: +22.5 degrees rounds into the next sector, while the
        // truncation keeps -23 degrees in sector 0 and moves -23.5+ out.
        for (deg, want) in [(10., 3), (22.4, 3), (22.6, 5), (-23.4, 3), (-23.6, 7), (179., 2), (-179., 2)] {
            let r: f64 = deg * core::f64::consts::PI / 180.;
            let (s, c) = (r.sin(), r.cos());
            let to = [(c * 100000.) as i32, (-s * 100000.) as i32];
            assert_eq!(direction([0, 0], to, true), want, "{deg}");
        }
        for (y, x) in [(1., 1.), (-1., 1.), (1., -1.), (-1., -1.), (0., -1.), (3., 0.5), (-0.2, -7.)] {
            let want: f64 = (y as f64).atan2(x) * 180. / core::f64::consts::PI;
            assert!((atan2_degrees(y, x) - want).abs() < 1e-9, "{y} {x}");
        }
        assert_eq!(distance([0, 0], [3, 4]), 5);
        assert_eq!(distance([0, 0], [1, 1]), 1);
        assert_eq!(distance([0, 0], [2, 2]), 3);
        let mut t = SwipeTracker::default();
        t.step([0, 0], SwipeButtons(2), 16);
        t.step([20, 0], SwipeButtons(2), 16);
        assert_eq!(t.gesture(false, true).direction, -1);
        t.step([40, 0], SwipeButtons(2), 16);
        let g = t.gesture(false, true);
        assert_eq!((g.press, g.pointer, g.buttons, g.direction), ([0, 0], [40, 0], 2, 3));
        assert_eq!(t.gesture(true, true).buttons, 0);
    }
    #[test]
    fn disabled_mask_never_drags() {
        let mut t = SwipeTracker::default();
        t.step([0, 0], SwipeButtons(1), 16);
        t.step([0, 200], SwipeButtons(1), 16);
        let r = t.report(true);
        assert_eq!((r.state, r.direction), (0, -1));
    }
}

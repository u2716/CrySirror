// SPDX-License-Identifier: MPL-2.0

//! Native region and unrestricted left-button drags and release measurements.
use super::Actions;
use super::{PointerGesture, HELD, LEFT, PRESSED, RELEASED};

impl PointerGesture {
    /// Mode 2 / 0x140054570. The press can begin anywhere, and movement
    /// across either axis activates it at the configurable threshold.
    pub(super) fn frame_unrestricted_drag(&mut self, input: &mut Actions, now: u32) {
        self.set(80, 0);
        self.set(88, -1);
        self.set(172, -1);
        if [0, 8, 12, 16, 20].into_iter().any(|at| self.get(at) == 0) {
            self.idle();
            self.set(180, 0);
            self.set(184, 0);
            return;
        }
        let [x, y] = input.pointer();
        let [released, held, pressed] = input.mouse(LEFT);
        if self.get(48) == 0 {
            if pressed {
                for (at, value) in [
                    (56, x),
                    (60, y),
                    (100, x),
                    (104, y),
                    (48, 1),
                    (176, 0),
                    (180, 0),
                    (184, 0),
                    (156, now as i32),
                ] {
                    self.set(at, value);
                }
            }
            return;
        }

        let anchor = [self.get(100), self.get(104)];
        let dx = x.wrapping_sub(anchor[0]);
        let dy = y.wrapping_sub(anchor[1]);
        if self.get(120) != 0 {
            self.set(124, dx);
            self.set(128, dy);
        } else {
            let threshold = self.get(40);
            if dx.wrapping_abs() >= threshold || dy.wrapping_abs() >= threshold {
                for (at, value) in [
                    (132, x),
                    (136, y),
                    (140, x),
                    (144, y),
                    (148, x),
                    (152, y),
                    (156, now as i32),
                    (160, now as i32),
                    (120, 1),
                ] {
                    self.set(at, value);
                }
            }
            self.set(124, 0);
            self.set(128, 0);
        }

        let active = self.get(120) != 0;
        if active {
            let elapsed = now.wrapping_sub(self.get(156) as u32);
            if elapsed == 0 || dx == 0 || dy == 0 {
                self.set(164, 0);
                self.set(180, 0);
                self.set(184, 0);
            } else {
                let (angle, distance) = super::measure_scaled(anchor, [x, y], 10);
                self.set(164, elapsed as i32);
                self.set(180, (angle + 90) % 360);
                self.set(184, distance);
            }
        }
        if released || !held {
            if active {
                self.set(172, 1);
                input.clear_all_edges();
            }
            self.set(48, 0);
            self.set(120, 0);
        }
    }

    /// 0x2369 / 0x1400537a0: a new left press in a configured region can
    /// immediately establish a drag while interrupting auxiliary inertia.
    pub(super) fn drag_press(&mut self, input: &Actions, force: bool, now: u32) -> bool {
        let [x, y] = input.pointer();
        let hit = self.get(4) == 1
            && input.mouse(LEFT)[PRESSED]
            && (0..6).any(|slot| {
                let base = 188 + slot * 20;
                let [left, top, w, h] = [4, 8, 12, 16].map(|d| self.get(base + d));
                self.get(base) != 0
                    && x >= left
                    && y >= top
                    && x < left.wrapping_add(w)
                    && y < top.wrapping_add(h)
            });
        if hit
            && force
            && self.get(120) == 0
            && [0, 8, 12, 16, 20].into_iter().all(|at| self.get(at) != 0)
        {
            for (at, value) in [
                (56, x),
                (60, y),
                (100, x),
                (104, y),
                (132, x),
                (136, y),
                (140, x),
                (144, y),
                (148, x),
                (152, y),
                (156, now as i32),
                (160, now as i32),
                (44, -1),
                (172, -1),
                (124, 0),
                (128, 0),
                (48, 1),
                (120, 1),
            ] {
                self.set(at, value);
            }
        }
        hit
    }

    pub(super) fn frame_drag(&mut self, input: &mut Actions, now: u32) {
        self.set(80, 0);
        self.set(88, -1);
        if [0, 8, 12, 16, 20].into_iter().any(|at| self.get(at) == 0) {
            self.idle();
            self.set(172, -1);
            return;
        }
        let [x, y] = input.pointer();
        if self.get(48) == 0 {
            if !input.mouse(LEFT)[PRESSED] {
                return;
            }
            for (at, value) in [(56, x), (60, y), (100, x), (104, y)] {
                self.set(at, value);
            }
            let mut hit = None;
            for slot in 0..6 {
                let base = 188 + slot * 20;
                let [left, top, w, h] = [4, 8, 12, 16].map(|d| self.get(base + d));
                if self.get(base) != 0
                    && x >= left
                    && y >= top
                    && x < left.wrapping_add(w)
                    && y < top.wrapping_add(h)
                {
                    hit = Some(slot);
                }
            }
            if let Some(slot) = hit {
                self.set(48, 1);
                self.set(44, -1);
                self.set(172, -1);
                self.set(176, slot as i32);
            }
            return;
        }
        if self.get(120) != 0 {
            self.set(124, x.wrapping_sub(self.get(100)));
            self.set(128, y.wrapping_sub(self.get(104)));
        } else {
            let difference = if self.get(116) != 0 {
                y.wrapping_sub(self.get(104))
            } else {
                x.wrapping_sub(self.get(100))
            };
            if difference.wrapping_abs() >= 4 {
                for (at, value) in [
                    (132, x),
                    (136, y),
                    (140, x),
                    (144, y),
                    (148, x),
                    (152, y),
                    (156, now as i32),
                    (160, now as i32),
                    (100, x),
                    (104, y),
                ] {
                    self.set(at, value);
                }
                self.set(172, -1);
                self.set(120, 1);
            }
            self.set(124, 0);
            self.set(128, 0);
        }
        if self.get(120) != 0 {
            let near = x.wrapping_sub(self.get(140)).wrapping_abs() < 4
                && y.wrapping_sub(self.get(144)).wrapping_abs() < 4;
            if !near || now.wrapping_sub(self.get(160) as u32) >= 100 {
                if near {
                    self.set(132, x);
                    self.set(136, y);
                    self.set(156, now as i32);
                }
                self.set(140, x);
                self.set(144, y);
                self.set(160, now as i32);
            }
        }
        let flags = input.mouse(LEFT);
        if !flags[RELEASED] && flags[HELD] {
            return;
        }
        if self.get(120) != 0 {
            let dx = x.wrapping_sub(self.get(132));
            let dy = y.wrapping_sub(self.get(136));
            let (distance, axis) = if dx.wrapping_abs() < dy.wrapping_abs() {
                (dy, 1)
            } else {
                (dx, 0)
            };
            self.set(164, now.wrapping_sub(self.get(156) as u32) as i32);
            self.set(168, distance);
            self.set(
                172,
                if distance.wrapping_abs() < 4 {
                    -1
                } else {
                    axis
                },
            );
        }
        if !flags[RELEASED] {
            self.set(44, -1);
        }
        self.set(48, 0);
        if self.get(120) != 0 {
            input.clear_all_edges();
        }
        self.set(120, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_core::service::{buttons, InputFrame};

    fn unrestricted() -> PointerGesture {
        let mut gesture = PointerGesture::default();
        for at in [0, 8, 12, 16, 20] {
            gesture.set(at, 1);
        }
        gesture.set(4, 2);
        gesture
    }

    fn frame(gesture: &mut PointerGesture, input: &mut Actions, point: [i32; 2], pressed: bool) {
        input.admit(InputFrame {
            buttons_down: if pressed { buttons::MOUSE_LEFT } else { 0 },
            pointer_x: point[0],
            pointer_y: point[1],
            ..Default::default()
        });
        input.begin_frame();
        gesture.frame(input, 10);
    }

    #[test]
    fn unrestricted_drag_uses_either_axis_threshold_and_reports_one_release_frame() {
        let mut gesture = unrestricted();
        let mut input = Actions::default();
        frame(&mut gesture, &mut input, [10, 20], true);
        frame(&mut gesture, &mut input, [13, 23], true);
        assert_eq!(gesture.get(120), 0);
        frame(&mut gesture, &mut input, [14, 24], true);
        assert_eq!(gesture.get(120), 1);
        assert_eq!(gesture.get(164), 0);

        frame(&mut gesture, &mut input, [74, 84], true);
        assert_eq!([gesture.get(124), gesture.get(128)], [64, 64]);
        assert_eq!(
            [gesture.get(164), gesture.get(180), gesture.get(184)],
            [10, 135, 905]
        );
        frame(&mut gesture, &mut input, [74, 84], false);
        assert_eq!(gesture.get(172), 1);
        assert_eq!(gesture.get(164), 20);
        assert!(!input.mouse(LEFT)[RELEASED]);
        frame(&mut gesture, &mut input, [74, 84], false);
        assert_eq!(gesture.get(172), -1);
    }

    #[test]
    fn unrestricted_drag_preserves_clicks_and_reports_zero_speed_on_an_axis() {
        let mut gesture = unrestricted();
        let mut input = Actions::default();
        frame(&mut gesture, &mut input, [10, 20], true);
        frame(&mut gesture, &mut input, [13, 20], false);
        assert_eq!(gesture.get(172), -1);
        assert!(input.mouse(LEFT)[RELEASED]);

        frame(&mut gesture, &mut input, [10, 20], true);
        frame(&mut gesture, &mut input, [14, 20], true);
        frame(&mut gesture, &mut input, [54, 20], false);
        assert_eq!(gesture.get(172), 1);
        assert_eq!(
            [gesture.get(164), gesture.get(180), gesture.get(184)],
            [0; 3]
        );
        assert!(!input.mouse(LEFT)[RELEASED]);
    }
}

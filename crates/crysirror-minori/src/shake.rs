// SPDX-License-Identifier: MPL-2.0
use crate::vm::ShakeRequest;
use anyhow::{Result, ensure};
use crysirror_image::temporal_pattern::PatternRng;
use serde_json::{Value, json};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ScreenAbi {
    #[default]
    Early,
    SignedDuration,
    PersistentZoom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Motion {
    Zoom,
    Vertical,
    Horizontal,
    Random,
    Roll,
    Tilt,
    Background,
}

pub(crate) struct Shake {
    mode: Motion,
    start: u64,
    duration: u32,
    amplitude: i32,
    abi: ScreenAbi,
    last_tick: Option<u64>,
    phase: u32,
}

impl Shake {
    pub fn new(request: ShakeRequest, abi: ScreenAbi, now: u64) -> Self {
        let mode = match request.mode.as_bytes().first().map(u8::to_ascii_uppercase) {
            Some(b'Z') => Motion::Zoom,
            Some(b'V') => Motion::Vertical,
            Some(b'H') => Motion::Horizontal,
            Some(b'O') => Motion::Roll,
            Some(b'T') => Motion::Tilt,
            _ if abi != ScreenAbi::Early && request.duration <= 0 => Motion::Background,
            _ => Motion::Random,
        };
        Self {
            mode,
            start: now,
            duration: if abi == ScreenAbi::Early {
                request.duration as u32
            } else {
                request.duration.unsigned_abs()
            },
            amplitude: request.amplitude,
            abi,
            last_tick: None,
            phase: 0,
        }
    }

    pub fn background(&self) -> bool {
        self.mode == Motion::Background
    }

    pub fn finished(&self, now: u64, skip: bool, advance: bool) -> bool {
        (now.saturating_sub(self.start) / 10) >= u64::from(self.duration)
            || skip
            || (advance && !matches!(self.mode, Motion::Zoom | Motion::Background))
    }

    pub fn sample(
        &mut self,
        now: u64,
        canvas: [i32; 2],
        rng: &mut PatternRng,
    ) -> Result<Option<Value>> {
        let tick = now.saturating_sub(self.start) / 10;
        let interval = match (self.mode, self.abi) {
            (_, ScreenAbi::Early) | (Motion::Tilt, _) => 2,
            _ => 3,
        };
        if self.last_tick.is_some_and(|t| tick - t < interval) {
            return Ok(None);
        }
        self.last_tick = Some(tick);
        let even = self.phase % 2 == 0;
        self.phase = self.phase.wrapping_add(1);
        let a = self.amplitude as f64;
        let [w, h] = canvas.map(f64::from);
        let shift = |x: f64, y: f64| json!({"pose_offset":{"translation":[x,y,0.0]}});
        let rotate = |degrees: f64, x: f64, y: f64| json!({"rotate":(degrees % 180.0).to_radians(),"pivot_x":x,"pivot_y":y});
        Ok(Some(match self.mode {
            Motion::Vertical => shift(0.0, if even { -a } else { a }),
            Motion::Horizontal => shift(if even { -a } else { a }, 0.0),
            Motion::Random => {
                // Every axis helper copies from the *same* saved screen and
                // clears the uncovered strip. The final call owns all pixels.
                match rng.next_draw()? % 8 {
                    0 | 1 => shift(-a, 0.0),
                    2 | 3 => shift(a, 0.0),
                    4 | 5 => shift(0.0, a),
                    _ => shift(0.0, -a),
                }
            }
            Motion::Background => {
                // The two ten-element native arrays are indexed by rand()%9.
                let offsets = [
                    (0, 0),
                    (-1, -1),
                    (0, -1),
                    (1, -1),
                    (-1, 0),
                    (0, 0),
                    (1, 0),
                    (-1, 1),
                    (0, 1),
                ];
                let (x, y) = offsets[(rng.next_draw()? % 9) as usize];
                shift(f64::from(x) * a, f64::from(y) * a)
            }
            Motion::Zoom if even || self.abi == ScreenAbi::PersistentZoom => {
                ensure!(
                    w > 2.0 * a && h > 2.0 * a,
                    "minori shakeScreen empty zoom source"
                );
                json!({"scale_x":w/(w-2.0*a),"scale_y":h/(h-2.0*a),"pivot_x":w/2.0,"pivot_y":h/2.0})
            }
            Motion::Zoom => json!({}),
            Motion::Roll => {
                ensure!(
                    self.amplitude != 0,
                    "minori shakeScreen zero rotation divisor"
                );
                let angle = i64::from(rng.next_draw()?) % (2 * i64::from(self.amplitude))
                    - i64::from(self.amplitude);
                rotate(angle as f64, w / 2.0, h / 2.0)
            }
            Motion::Tilt => rotate((tick as f64 * 1.25).min(a), 799.0, 0.0),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(mode: &str, duration: i32) -> ShakeRequest {
        ShakeRequest {
            mode: mode.into(),
            amplitude: 20,
            duration,
        }
    }
    #[test]
    fn signed_duration_changes_background_dispatch_only_for_later_registries() {
        let early = Shake::new(request("?", -2), ScreenAbi::Early, 10);
        assert!(!early.background());
        assert!(!early.finished(30, false, false));
        let later = Shake::new(request("?", -2), ScreenAbi::SignedDuration, 10);
        assert!(later.background());
        assert!(!later.finished(29, false, true));
        assert!(later.finished(30, false, false));
    }
    #[test]
    fn redraw_interval_and_zoom_alternation_follow_registry() {
        let mut rng = PatternRng::msvc(1);
        for abi in [ScreenAbi::SignedDuration, ScreenAbi::PersistentZoom] {
            let mut s = Shake::new(request("z", 10), abi, 50);
            let first = s.sample(50, [800, 600], &mut rng).unwrap().unwrap();
            assert!(first["scale_x"].as_f64().unwrap() > 1.0);
            assert!(s.sample(79, [800, 600], &mut rng).unwrap().is_none());
            let second = s.sample(80, [800, 600], &mut rng).unwrap().unwrap();
            assert_eq!(second == first, abi == ScreenAbi::PersistentZoom);
            assert!(!s.finished(149, false, true));
            assert!(s.finished(150, false, false));
        }
    }
}

// SPDX-License-Identifier: MPL-2.0
/// Scenario-space camera positions. The shared scene owns clipping and drawing.
pub(super) struct Scroll {
    pub start: [i32; 2],
    pub target: [i32; 2],
    pub clock: u64,
    pub motion: Motion,
}
pub(super) enum Motion {
    Axis(i32),
    Diagonal(i32),
    Timed(u32),
    Foreground(i32),
    Accelerated {
        ticks: i32,
        interruptible: bool,
    },
    Wave {
        period: i32,
        speed: i32,
        horizontal_period: u32,
        amplitude: i32,
    },
}
impl Scroll {
    pub fn finished(&self, now: u64, position: [i32; 2]) -> bool {
        let elapsed_ticks = (now.saturating_sub(self.clock) / 10) as u32;
        match self.motion {
            Motion::Wave { .. } => false,
            Motion::Accelerated { ticks, .. } => elapsed_ticks >= ticks.unsigned_abs(),
            Motion::Foreground(speed) => {
                let distance = self.start[0]
                    .abs_diff(self.target[0])
                    .max(self.start[1].abs_diff(self.target[1]));
                elapsed_ticks.wrapping_mul(speed as u32) / 10 >= distance
            }
            _ => position == self.target,
        }
    }

    pub fn position(&self, now: u64) -> [i32; 2] {
        let elapsed = now.saturating_sub(self.clock);
        let ticks = (elapsed / 10) as u32;
        if let Motion::Wave {
            period,
            speed,
            horizontal_period,
            amplitude,
        } = self.motion
        {
            let x = ((ticks % horizontal_period) as i32).wrapping_mul(speed) / 10;
            let phase = f64::from((ticks % period as u32) as i32) / f64::from(period);
            let y = f64::from(amplitude) * (1. + (phase * std::f64::consts::TAU).sin());
            return [self.start[0].wrapping_add(x), y as i32];
        }
        let offset = self.target.map(i128::from);
        let delta = [
            offset[0] - i128::from(self.start[0]),
            offset[1] - i128::from(self.start[1]),
        ];
        let position = |axis: usize| {
            let start = i128::from(self.start[axis]);
            let end = offset[axis];
            let value = match self.motion {
                Motion::Wave { .. } => unreachable!(),
                Motion::Accelerated {
                    ticks: duration, ..
                } => {
                    if ticks >= duration.unsigned_abs() {
                        return self.target[axis];
                    }
                    let t = f64::from(ticks) / f64::from(duration.unsigned_abs());
                    let weight = if duration < 0 {
                        1. - (1. - t).powi(2)
                    } else {
                        t * t
                    };
                    start + (delta[axis] as f64 * weight) as i128
                }
                Motion::Foreground(speed) => {
                    let distance = delta[0].abs().max(delta[1].abs());
                    if distance == 0 {
                        return self.target[axis];
                    }
                    let travelled = u32::wrapping_mul(ticks, speed as u32) / 10;
                    let t = f64::from(travelled) / distance as f64;
                    if t >= 1. {
                        return self.target[axis];
                    }
                    return (self.start[axis] as f64 + delta[axis] as f64 * t) as i32;
                }
                Motion::Timed(0) => end,
                Motion::Timed(duration) => {
                    let fraction = (elapsed as f64 / f64::from(duration)).min(1.0);
                    return (self.start[axis] as f64 + delta[axis] as f64 * fraction).round()
                        as i32;
                }
                Motion::Axis(speed) => {
                    if speed == 0 || start == end {
                        return self.start[axis];
                    }
                    let p = start + i128::from(speed) * i128::from(elapsed / 10) / 10;
                    if speed > 0 { p.min(end) } else { p.max(end) }
                }
                Motion::Diagonal(speed) => {
                    let distance = delta[0].abs().max(delta[1].abs());
                    let travelled = i128::from(speed) * i128::from(elapsed / 10) / 10;
                    if distance == 0 || travelled >= distance {
                        end
                    } else {
                        start + delta[axis] * travelled / distance
                    }
                }
            };
            value.clamp(i128::from(i32::MIN), i128::from(i32::MAX)) as i32
        };
        [position(0), position(1)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accelerated_scroll_uses_signed_duration_and_its_own_deadline() {
        for (ticks, midpoint) in [(10, [5, -5]), (-10, [15, -15])] {
            let scroll = Scroll {
                start: [0, 0],
                target: [20, -20],
                clock: 30,
                motion: Motion::Accelerated {
                    ticks,
                    interruptible: false,
                },
            };
            assert_eq!(scroll.position(80), midpoint);
            assert_eq!(scroll.position(130), scroll.target);
            let still = Scroll {
                target: [0, 0],
                ..scroll
            };
            assert!(!still.finished(129, still.position(129)));
            assert!(still.finished(130, still.position(130)));
        }
    }

    #[test]
    fn foreground_scroll_finishes_on_the_longer_axis() {
        let scroll = Scroll {
            start: [7, 2],
            target: [-3, 7],
            clock: 0,
            motion: Motion::Foreground(10),
        };
        assert_eq!(scroll.position(50), [2, 4]);
        assert!(!scroll.finished(90, scroll.position(90)));
        assert!(scroll.finished(100, scroll.position(100)));
        assert_eq!(scroll.position(100), [-3, 7]);
    }

    #[test]
    fn wave_repeats_without_completing_at_the_seam() {
        let scroll = Scroll {
            start: [3, 19],
            target: [3, 19],
            clock: 0,
            motion: Motion::Wave {
                period: 4,
                speed: 10,
                horizontal_period: 8,
                amplitude: 8,
            },
        };
        assert_eq!(scroll.position(0), [3, 8]);
        assert_eq!(scroll.position(10), [4, 16]);
        assert_eq!(scroll.position(30), [6, 0]);
        assert_eq!(scroll.position(80), scroll.position(0));
        assert!(!scroll.finished(80, scroll.position(80)));
    }
}

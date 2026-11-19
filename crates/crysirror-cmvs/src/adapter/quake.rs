// SPDX-License-Identifier: MPL-2.0

//! Native quake phase and return policy. Shared Layers apply the sampled offset.

use crate::vm::CmvsVm;
use anyhow::{ensure, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Quake {
    /// +0, SP-4. Low nibble: 0 shakes every queue item, 1 only slot objects, 2 only the
    /// message windows and text objects. Bits 4..7: direction (0 horizontal, 0x10
    /// vertical, 0x30 rotating corners). Bits 8..11: waveform (0 steps, 0x200 damped cosine).
    pub mode: u32,
    /// +4, SP-8.
    pub amplitude: i32,
    /// +8, SP-12: step period, or the whole duration of the damped cosine.
    pub period: u32,
    /// +12, SP-16: steps or cosine cycles; u32::MAX never ends a step quake.
    pub count: u32,
    /// +20, 0x2227.
    pub paused: bool,
    /// +24: unclamped accumulated milliseconds (steps subtract whole periods).
    pub elapsed: u32,
    /// +28: completed steps or cycles.
    pub steps: u32,
    /// +32/+36.
    pub offset: [i32; 2],
}

impl Quake {
    /// 0x1400526b0: the offset for queue items enqueued as message windows/text (`ui`) or
    /// as slot objects; zero while paused or for the layer the low nibble excludes.
    pub fn offset(&self, ui: bool) -> [i32; 2] {
        let excluded = if ui { 1 } else { 2 };
        if self.paused || self.mode & 0xf == excluded {
            [0, 0]
        } else {
            self.offset
        }
    }

    /// 0x140052380: true once the quake ends. Paused quakes keep their time.
    pub fn advance(&mut self, delta: u32) -> bool {
        if self.paused {
            return false;
        }
        self.elapsed = self.elapsed.wrapping_add(delta);
        if self.mode & 0xf00 == 0x200 {
            return self.damped();
        }
        let mut done = false;
        while self.elapsed >= self.period {
            self.elapsed -= self.period;
            let a = self.amplitude;
            let [x, y] = &mut self.offset;
            match self.mode & 0xf0 {
                0x00 => *x = if *x == 0 { a } else { x.wrapping_neg() },
                0x10 => *y = if *y == 0 { a } else { y.wrapping_neg() },
                0x30 => {
                    let neg = a.wrapping_neg();
                    [*x, *y] = match self.steps & 3 {
                        0 => [neg, neg],
                        1 => [a, a],
                        2 => [a, neg],
                        _ => [neg, a],
                    };
                }
                // 0x20 is rejected at creation; others leave the offset.
                _ => {}
            }
            // Compared before counting this step, so count + 2 steps run.
            if self.count != u32::MAX && self.count < self.steps {
                done = true;
            }
            self.steps = self.steps.wrapping_add(1);
        }
        done
    }

    /// The 0x200 waveform: the phase angle is (count × elapsed × 360) / duration in u32,
    /// the cosine of (angle + 90) mod 360 degrees scaled by the amplitude is truncated
    /// (0x1400089b0, f64), then damped by the remaining time with signed division.
    fn damped(&mut self) -> bool {
        let (value, done) = if self.period <= self.elapsed {
            self.steps = self.count;
            (0, true)
        } else {
            let angle = self.count.wrapping_mul(self.elapsed).wrapping_mul(360) / self.period;
            self.steps = angle / 360;
            let degrees = angle.wrapping_add(90) % 360;
            let radians = if degrees == 0 {
                0.0
            } else {
                f64::from(degrees) / 180.0 * std::f64::consts::PI
            };
            let cosine = (radians.cos() * f64::from(self.amplitude)) as i32;
            let remaining = self.period.wrapping_sub(self.elapsed) as i32;
            (
                remaining
                    .wrapping_mul(cosine)
                    .wrapping_div(self.period as i32),
                false,
            )
        };
        match self.mode & 0xf0 {
            0x00 => self.offset = [value, 0],
            0x10 => self.offset = [0, value],
            _ => {}
        }
        done
    }
}

pub(super) fn command(state: &mut Option<Quake>, vm: &mut CmvsVm, opcode: u16) -> Result<bool> {
    let (words, result, frame) = match opcode {
        0x220c => (
            0,
            Some(u32::from(state.as_ref().is_some_and(|q| q.paused))),
            false,
        ),
        0x2225 => {
            let [mode, amplitude, period, count] = vm.owner_arguments(opcode)?;
            ensure!(
                mode & 0xf00 == 0x200 || (mode & 0xf00 == 0 && mode & 0xf0 != 0x20),
                "CMVS unsupported quake waveform or random direction"
            );
            ensure!(period > 0, "CMVS quake period is zero");
            *state = Some(Quake {
                mode,
                amplitude: amplitude as i32,
                period,
                count,
                paused: false,
                elapsed: 0,
                steps: 0,
                offset: [0; 2],
            });
            (4, None, false)
        }
        0x2226 => {
            let left = state
                .as_ref()
                .filter(|q| q.count != u32::MAX)
                .map_or(u32::MAX, |q| q.count.wrapping_sub(q.steps));
            (0, Some(left), false)
        }
        0x2227 => {
            let [pause] = vm.owner_arguments(opcode)?;
            if let Some(quake) = state {
                quake.paused = pause != 0;
            }
            (1, None, false)
        }
        0x2228 => {
            let [wait] = vm.owner_arguments(opcode)?;
            if state.is_some() && wait != 0 {
                vm.wait_owner_call(opcode, Some(0))?;
                return Ok(true);
            }
            let was_active = state.take().is_some();
            (1, Some(1), was_active)
        }
        0x2229 => {
            *state = None;
            (0, None, false)
        }
        _ => unreachable!(),
    };
    vm.finish_owner_call(words, result, frame)?;
    Ok(frame)
}

pub(super) fn offsets(state: &Option<Quake>) -> [[i32; 2]; 2] {
    state.as_ref().map_or([[0; 2]; 2], |quake| {
        [quake.offset(false), quake.offset(true)]
    })
}

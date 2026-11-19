// SPDX-License-Identifier: MPL-2.0

//! CMVS frame clock, callbacks, and cross-script control state.

use super::{
    evaluate::{error, truncate_double},
    machine::Machine,
    DecodeError,
};

// The x64 rotation helpers use the single-precision π constant widened to double.
const NATIVE_PI: f64 = std::f32::consts::PI as f64;
const HALF_CIRCLE: f64 = 18_000.0;

#[derive(Clone, Copy)]
struct Callback {
    script: u8,
    target: u32,
    _parameters: [u32; 3],
    _registered_at: u32,
}

#[derive(Clone)]
pub(super) struct Control {
    callbacks: [Option<Callback>; 64],
    automatic: bool,
    inhibited: bool,
    version: u32,
    clock: u32,
    delta: u32,
    timers: [u32; 10],
    marked_at: u32,
    random: u32,
}

#[cfg(feature = "runtime")]
impl Control {
    pub(super) fn snapshot(&self) -> serde_json::Value {
        let callbacks: Vec<_> = self
            .callbacks
            .iter()
            .enumerate()
            .filter_map(|(index, callback)| {
                callback.map(|entry| {
                    serde_json::json!([index, entry.script, entry.target, entry._parameters, entry._registered_at])
                })
            })
            .collect();
        serde_json::json!({
            "callbacks": callbacks,
            "automatic": self.automatic,
            "inhibited": self.inhibited,
            "version": self.version,
            "clock": self.clock,
            "delta": self.delta,
            "timers": self.timers,
            "marked_at": self.marked_at,
            "random": self.random,
        })
    }

    pub(super) fn from_snapshot(value: &serde_json::Value) -> Option<Self> {
        use super::snapshot::{word, words};
        let mut control = Self::default();
        for entry in value["callbacks"].as_array()? {
            let parameters = entry[3].as_array()?;
            *control.callbacks.get_mut(entry[0].as_u64()? as usize)? = Some(Callback {
                script: entry[1].as_u64()? as u8,
                target: entry[2].as_u64()? as u32,
                _parameters: [
                    parameters[0].as_u64()? as u32,
                    parameters[1].as_u64()? as u32,
                    parameters[2].as_u64()? as u32,
                ],
                _registered_at: entry[4].as_u64()? as u32,
            });
        }
        control.automatic = value["automatic"].as_bool()?;
        control.inhibited = value["inhibited"].as_bool()?;
        control.version = word(value, "version")?;
        control.clock = word(value, "clock")?;
        control.delta = word(value, "delta")?;
        control.timers = words(value, "timers")?.try_into().ok()?;
        control.marked_at = word(value, "marked_at")?;
        control.random = word(value, "random")?;
        Some(control)
    }
}

impl Default for Control {
    fn default() -> Self {
        Self {
            callbacks: [None; 64],
            automatic: false,
            inhibited: false,
            version: 0,
            clock: 0,
            delta: 0,
            timers: [0; 10],
            marked_at: 0,
            random: 0,
        }
    }
}

impl Control {
    #[cfg(feature = "runtime")]
    pub(super) fn reset_frame_delta(&mut self) {
        self.delta = 0;
    }
    pub(super) fn replace_main(&mut self) {
        for entry in &mut self.callbacks {
            if entry.is_some_and(|callback| callback.script == 0) {
                *entry = None;
            }
        }
    }

    pub(super) fn begin_frame(
        &mut self,
        machine: &mut Machine,
        delta: u32,
        visible_messages: [bool; 8],
    ) -> Result<(), DecodeError> {
        self.clock = self.clock.wrapping_add(delta);
        self.delta = delta;
        for timer in &mut self.timers {
            *timer = timer.wrapping_add(delta);
        }
        self.advance_random();
        if self.inhibited {
            return Ok(());
        }
        let due: Vec<_> = self
            .callbacks
            .iter()
            .enumerate()
            .filter_map(|(index, callback)| {
                let callback = callback.as_ref()?;
                let due = index == 28
                    || self.automatic
                        && match index {
                            3..=6 => true,
                            10..=17 => visible_messages[index - 10],
                            _ => false,
                        };
                due.then_some((callback.script, callback.target as usize))
            })
            .collect();
        machine.dispatch_callbacks(&due)
    }

    pub(super) fn invoke(&self, machine: &mut Machine) -> Result<(), DecodeError> {
        let argument = Self::argument(machine, 0x208a, 4)?;
        let callback = self.callbacks[(argument & 63) as usize]
            .ok_or_else(|| error(machine.pc(), 0x208a, "CMVS callback is absent or disabled"))?;
        machine.call_callback(callback.script, callback.target as usize)
    }

    pub(super) fn command(
        &mut self,
        machine: &mut Machine,
        opcode: u16,
    ) -> Result<Option<u16>, DecodeError> {
        let pc = machine.pc();
        let cleanup = match opcode {
            0x201b | 0x20e9 | 0x23b5 => {
                machine
                    .state_mut()
                    .set_system_integer(0, 0)
                    .map_err(|why| error(pc, opcode, why))?;
                0
            }
            0x2008 | 0x2009 | 0x200f => {
                let value = match opcode {
                    0x2008 => self.clock,
                    0x2009 => self.delta,
                    _ => self.clock.wrapping_sub(self.marked_at),
                };
                machine
                    .state_mut()
                    .set_system_integer(0, value)
                    .map_err(|why| error(pc, opcode, why))?;
                0
            }
            0x200b | 0x200c => {
                let index = Self::argument(machine, opcode, 4)? as usize;
                let timer = self
                    .timers
                    .get_mut(index)
                    .ok_or_else(|| error(pc, opcode, "CMVS timer slot outside 0..9"))?;
                if opcode == 0x200b {
                    *timer = 0;
                } else {
                    machine
                        .state_mut()
                        .set_system_integer(0, *timer)
                        .map_err(|why| error(pc, opcode, why))?;
                }
                4
            }
            0x200e => {
                self.marked_at = self.clock;
                0
            }
            0x20d4 | 0x20d5 => {
                let first = Self::argument(machine, opcode, 4)? as i32;
                let (offset, divisor, cleanup) = if opcode == 0x20d4 {
                    (0, first, 4)
                } else {
                    let end = Self::argument(machine, opcode, 8)? as i32;
                    (first, end.wrapping_sub(first), 8)
                };
                if divisor == 0 {
                    return Err(error(pc, opcode, "CMVS random divisor is zero"));
                }
                let value = (self.advance_random() as i32 % divisor).wrapping_add(offset);
                machine
                    .state_mut()
                    .set_system_integer(0, value as u32)
                    .map_err(|why| error(pc, opcode, why))?;
                cleanup
            }
            0x20d0..=0x20d3 => {
                let count = match opcode {
                    0x20d0 => 1,
                    0x20d1 => 3,
                    _ => 2,
                };
                let mut operands = [0; 3];
                for (index, value) in operands[..count].iter_mut().enumerate() {
                    *value = Self::argument(machine, opcode, 4 * (index + 1))? as i32;
                }
                let [value, bound, upper] = operands;
                let result = match opcode {
                    0x20d0 => value.wrapping_abs(),
                    0x20d1 => value.max(bound).min(upper),
                    0x20d2 => value.max(bound),
                    _ => value.min(bound),
                };
                machine
                    .state_mut()
                    .set_system_integer(0, result as u32)
                    .map_err(|why| error(pc, opcode, why))?;
                (count * 4) as u16
            }
            0x20d6 => {
                let base = Self::argument(machine, opcode, 4)?;
                let exponent = Self::argument(machine, opcode, 8)? as i32;
                let result = base.wrapping_pow(exponent.max(1) as u32);
                machine
                    .state_mut()
                    .set_system_integer(0, result)
                    .map_err(|why| error(pc, opcode, why))?;
                8
            }
            0x20d7 => {
                let value = Self::argument(machine, opcode, 4)? as i32;
                let scale = Self::argument(machine, opcode, 8)? as i32;
                let mode = Self::argument(machine, opcode, 12)?;
                let result = match mode {
                    0 => value.wrapping_mul(scale) / 1000,
                    1 | 2 => {
                        let scaled = f64::from(value).sqrt() * f64::from(scale) / 1000.0;
                        let squared = truncate_double(scaled * scaled);
                        if mode == 1 {
                            squared
                        } else {
                            value.wrapping_sub(squared)
                        }
                    }
                    _ => return Ok(Some(12)),
                };
                machine
                    .state_mut()
                    .set_system_integer(0, result as u32)
                    .map_err(|why| error(pc, opcode, why))?;
                12
            }
            0x20d8 | 0x20d9 | 0x20dc => {
                let angle = Self::argument(machine, opcode, 4)? as i32;
                let (scale, cleanup) = if opcode == 0x20dc {
                    (4096, 4)
                } else {
                    (Self::argument(machine, opcode, 8)? as i32, 8)
                };
                let angle = if opcode == 0x20d8 {
                    angle.wrapping_mul(100)
                } else {
                    angle
                };
                let radians = f64::from(angle) / HALF_CIRCLE * NATIVE_PI;
                let scale = f64::from(scale);
                for (index, component) in [(1, radians.cos()), (2, radians.sin())] {
                    let value = truncate_double(component * scale);
                    machine
                        .state_mut()
                        .set_system_integer(index, value as u32)
                        .map_err(|why| error(pc, opcode, why))?;
                }
                cleanup
            }
            0x20da | 0x20db => {
                let x1 = Self::argument(machine, opcode, 4)? as i32;
                let y1 = Self::argument(machine, opcode, 8)? as i32;
                let x2 = Self::argument(machine, opcode, 12)? as i32;
                let y2 = Self::argument(machine, opcode, 16)? as i32;
                let x = x2.wrapping_sub(x1);
                let y = y2.wrapping_sub(y1);
                let radians = f64::from(y).atan2(f64::from(x));
                let clockwise = truncate_double(radians / -NATIVE_PI * HALF_CIRCLE);
                let angle = if clockwise < 0 {
                    -clockwise
                } else if clockwise == 0 {
                    0
                } else {
                    36_000 - clockwise
                };
                let distance = if x == 0 {
                    y.wrapping_abs()
                } else {
                    truncate_double(f64::from(x) / radians.cos())
                };
                let angle = if opcode == 0x20da { angle / 100 } else { angle };
                for (index, value) in [(4, angle), (5, distance)] {
                    machine
                        .state_mut()
                        .set_system_integer(index, value as u32)
                        .map_err(|why| error(pc, opcode, why))?;
                }
                16
            }
            0x20e0 => {
                let arg = |offset| Self::argument(machine, opcode, offset).map(f32::from_bits);
                let (x, y, z) = (f64::from(arg(4)?), f64::from(arg(8)?), f64::from(arg(12)?));
                let radians = |degrees: f32| f64::from(degrees / 180.0 * std::f32::consts::PI);
                let (pitch, yaw, roll) = (radians(arg(16)?), radians(arg(20)?), radians(arg(24)?));

                let (s, c) = (yaw.sin(), yaw.cos());
                let (x, z) = (z * s + x * c, z * c - x * s);
                let (s, c) = (pitch.sin(), pitch.cos());
                let (y, z) = (y * c - z * s, y * s + z * c);
                let (s, c) = (roll.sin(), roll.cos());
                let (x, y) = (x * c - y * s, x * s + y * c);

                for (index, value) in [(8, x), (9, y), (10, z)] {
                    let rounded = truncate_double(value * 100.0) as f32 / 100.0;
                    machine
                        .state_mut()
                        .set_system_float_bits(index, rounded.to_bits())
                        .map_err(|why| error(pc, opcode, why))?;
                }
                24
            }
            0x2083 => {
                let high = Self::argument(machine, opcode, 4)?;
                let low = Self::argument(machine, opcode, 8)?;
                self.version = high.wrapping_shl(16) | low;
                8
            }
            0x2088 | 0x208b => {
                let index = Self::argument(machine, opcode, 4)? as usize;
                let entry = self
                    .callbacks
                    .get_mut(index)
                    .ok_or_else(|| error(pc, opcode, "CMVS callback slot outside 0..63"))?;
                if opcode == 0x208b {
                    *entry = None;
                    return Ok(Some(4));
                }
                let target = Self::argument(machine, opcode, 8)?;
                let parameters = [
                    Self::argument(machine, opcode, 20)?,
                    Self::argument(machine, opcode, 16)?,
                    Self::argument(machine, opcode, 12)?,
                ];
                let script = machine.current_slot();
                if target != u32::MAX && !machine.is_boundary(script, target as usize) {
                    return Err(error(
                        pc,
                        opcode,
                        "CMVS callback target is not an instruction boundary",
                    ));
                }
                *entry = (target != u32::MAX).then_some(Callback {
                    script,
                    target,
                    _parameters: parameters,
                    _registered_at: self.clock,
                });
                20
            }
            0x2089 | 0x208c => {
                let enabled = Self::argument(machine, opcode, 4)? != 0;
                if opcode == 0x2089 {
                    self.automatic = enabled;
                } else {
                    self.inhibited = enabled;
                }
                4
            }
            _ => return Ok(None),
        };
        Ok(Some(cleanup))
    }

    fn argument(machine: &Machine, opcode: u16, distance: usize) -> Result<u32, DecodeError> {
        machine
            .stack()
            .peek_u32(distance)
            .map_err(|error_kind| error(machine.pc(), opcode, error_kind.0))
    }

    fn advance_random(&mut self) -> u32 {
        self.random = self.random.wrapping_mul(214013).wrapping_add(2531011);
        (self.random >> 16) & 0x7fff
    }
}

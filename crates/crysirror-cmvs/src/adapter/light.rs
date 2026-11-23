// SPDX-License-Identifier: MPL-2.0
//! The `0x2260..=0x2275` world object (renderer +32, update 0x140062e50 in the 2021 build): a
//! world-camera position and Euler angles, with native tween slots.

use crate::vm::CmvsVm;
use anyhow::Result;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct Light {
    channels: [f32; 7],
    tween: Option<Tween>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Tween {
    mode: u32,
    start: [f32; 5],
    position: Option<[f32; 3]>,
    values: Option<[f32; 2]>,
    duration: u32,
    elapsed: u32,
}

impl Default for Light {
    fn default() -> Self {
        Self { channels: [0., 1., 0., 0., 0., 0., 45.], tween: None }
    }
}

impl Light {
    pub fn view(&self) -> [f32; 7] {
        self.channels
    }

    pub fn set_view(&mut self, position: [f32; 3], pitch: Option<f32>, yaw: Option<f32>) {
        self.channels[..3].copy_from_slice(&position);
        if let Some(value) = pitch {
            self.channels[3] = value;
        }
        if let Some(value) = yaw {
            self.channels[4] = value;
        }
    }

    pub fn active(&self) -> bool {
        self.tween.is_some()
    }

    /// 0x1400633f0: stops the tween, optionally landing on its targets.
    pub fn finish(&mut self, land: bool) {
        let Some(tween) = self.tween.take().filter(|_| land) else {
            return;
        };
        if let Some(position) = tween.position {
            self.channels[..3].copy_from_slice(&position);
        }
        if let Some(values) = tween.values {
            self.channels[3..5].copy_from_slice(&values);
        }
    }

    pub fn advance(&mut self, delta: u32) {
        let Some(tween) = self.tween.as_mut() else {
            return;
        };
        tween.elapsed = tween.elapsed.saturating_add(delta);
        if tween.elapsed >= tween.duration {
            self.finish(true);
            return;
        }
        let progress = tween.elapsed as f32 / tween.duration as f32;
        let quarter = progress * std::f32::consts::FRAC_PI_2;
        let eased = match tween.mode {
            1 => quarter.sin(),
            2 => 1. - quarter.cos(),
            _ => progress,
        };
        let blend = |from: f32, to: f32| from + (to - from) * eased;
        if let Some(position) = tween.position {
            for axis in 0..3 {
                self.channels[axis] = blend(tween.start[axis], position[axis]);
            }
        }
        if let Some(values) = tween.values {
            for axis in 0..2 {
                self.channels[3 + axis] = blend(tween.start[3 + axis], values[axis]);
            }
        }
    }

    fn start(&mut self, mode: u32, position: Option<[f32; 3]>, values: Option<[f32; 2]>, duration: u32) {
        let start = std::array::from_fn(|channel| self.channels[channel]);
        self.tween = Some(Tween { mode, start, position, values, duration, elapsed: 0 });
    }

    /// Commands that complete immediately. `0x2271` waits in the adapter.
    pub fn command(&mut self, opcode: u16, vm: &mut CmvsVm) -> Result<()> {
        let float = f32::from_bits;
        match opcode {
            0x2260 => {
                let [x, y, z] = vm.owner_arguments(opcode)?;
                self.channels[..3].copy_from_slice(&[float(x), float(y), float(z)]);
                vm.complete_owner_call(3, None)?;
            }
            0x2261..=0x2264 => {
                let [value] = vm.owner_arguments(opcode)?;
                self.channels[3 + usize::from(opcode - 0x2261)] = float(value);
                vm.complete_owner_call(1, None)?;
            }
            0x2270 => {
                let [land] = vm.owner_arguments(opcode)?;
                self.finish(land != 0);
                vm.complete_owner_call(1, None)?;
            }
            0x2272 => vm.complete_owner_call(0, Some(u32::from(self.active())))?,
            0x2273 => {
                let [mode, x, y, z, duration] = vm.owner_arguments(opcode)?;
                self.start(mode, Some([float(x), float(y), float(z)]), None, duration);
                vm.complete_owner_call(5, None)?;
            }
            0x2274 => {
                let [mode, first, second, duration] = vm.owner_arguments(opcode)?;
                self.start(mode, None, Some([float(first), float(second)]), duration);
                vm.complete_owner_call(4, None)?;
            }
            0x2275 => {
                let [mode, x, y, z, first, second, duration] = vm.owner_arguments(opcode)?;
                let position = Some([float(x), float(y), float(z)]);
                self.start(mode, position, Some([float(first), float(second)]), duration);
                vm.complete_owner_call(7, None)?;
            }
            _ => anyhow::bail!("unhandled CMVS light command 0x{opcode:04x}"),
        }
        Ok(())
    }
}

// SPDX-License-Identifier: MPL-2.0
//! PC x64 device and camera VM state used by screen-reset commands.

use super::{
    evaluate::{error, truncate},
    machine::Machine,
    DecodeError,
};

/// Timed camera move started by `0x206b..=0x206f` or `0x207d` (0x140061c20).
#[derive(Clone)]
struct Tween {
    mode: u32,
    start: [f32; 3],
    target: [f32; 3],
    roll: Option<[f32; 2]>,
    duration: u32,
    elapsed: u32,
}

#[derive(Clone)]
pub(super) struct Screen {
    size: [u32; 2],
    cameras: [[u32; 40]; 2],
    depth_clear: bool,
    tween: Option<Tween>,
}

fn centi(value: f32) -> f32 {
    (value * 100.) as i32 as f32 / 100.
}

impl Default for Screen {
    fn default() -> Self {
        let mut words = [0; 40];
        words[0] = 1;
        for (index, value) in [
            (2, 3.0f32),
            (6, 10.),
            (7, 10.),
            (8, 6.),
            (9, 1024.),
            (10, 612.),
            (11, 24.),
            (15, 512.),
            (16, 306.),
            (17, 1.),
            (18, 1.),
        ] {
            words[index] = value.to_bits();
        }
        Self {
            size: [1024, 612],
            cameras: [words; 2],
            depth_clear: false,
            tween: None,
        }
    }
}

impl Screen {
    #[cfg(feature = "runtime")]
    pub(super) fn reset(&mut self) {
        *self = Self {
            size: self.size,
            ..Self::default()
        };
    }
    #[cfg(feature = "runtime")]
    pub fn set_size(&mut self, size: [u32; 2]) {
        self.size = size;
    }

    #[cfg(feature = "runtime")]
    pub fn cameras(&self) -> [[u32; 40]; 2] {
        self.cameras
    }

    #[cfg(feature = "runtime")]
    pub(super) fn snapshot(&self) -> serde_json::Value {
        let cameras: Vec<Vec<u32>> = self.cameras.iter().map(|camera| camera.to_vec()).collect();
        serde_json::json!({"size": self.size, "cameras": cameras, "depth_clear": self.depth_clear})
    }

    #[cfg(feature = "runtime")]
    pub(super) fn from_snapshot(value: &serde_json::Value) -> Option<Self> {
        let mut screen = Self::default();
        screen.size = super::snapshot::words(value, "size")?.try_into().ok()?;
        for (camera, saved) in screen.cameras.iter_mut().zip(value["cameras"].as_array()?) {
            let words: Option<Vec<u32>> = saved
                .as_array()?
                .iter()
                .map(|word| word.as_u64().map(|word| word as u32))
                .collect();
            *camera = words?.try_into().ok()?;
        }
        screen.depth_clear = value["depth_clear"].as_bool()?;
        Some(screen)
    }

    #[cfg(feature = "runtime")]
    pub fn set_camera_position(&mut self, position: [f32; 3]) {
        for (axis, value) in position.into_iter().enumerate() {
            self.cameras[0][1 + axis] = centi(value).to_bits();
        }
    }

    #[cfg(feature = "runtime")]
    pub fn tween_active(&self) -> bool {
        self.tween.is_some()
    }

    #[cfg(feature = "runtime")]
    pub fn finish_tween(&mut self) {
        if let Some(tween) = self.tween.take() {
            for axis in 0..3 {
                self.cameras[0][1 + axis] = tween.target[axis].to_bits();
            }
            if let Some([_, roll]) = tween.roll {
                self.cameras[0][14] = roll.to_bits();
            }
        }
    }

    #[cfg(feature = "runtime")]
    pub fn advance_tween(&mut self, delta: u32) {
        let Some(tween) = self.tween.as_mut() else {
            return;
        };
        tween.elapsed = tween.elapsed.saturating_add(delta.min(200));
        if tween.elapsed >= tween.duration {
            self.finish_tween();
            return;
        }
        let progress = tween.elapsed as f32 / tween.duration as f32;
        let quarter = progress * std::f32::consts::FRAC_PI_2;
        let eased = match tween.mode {
            1 => quarter.sin(),
            2 => 1. - quarter.cos(),
            _ => progress,
        };
        let blend = |from: f32, to: f32| centi(from + (to - from) * eased);
        for axis in 0..3 {
            self.cameras[0][1 + axis] = blend(tween.start[axis], tween.target[axis]).to_bits();
        }
        if let Some([from, to]) = tween.roll {
            self.cameras[0][14] = blend(from, to).to_bits();
        }
    }

    /// 0x1400621d0 without the roll channel: target words are rounded to hundredths.
    #[cfg(feature = "runtime")]
    pub fn tween_camera(&mut self, mode: u32, target: [f32; 3], duration: u32) {
        let start: [f32; 3] = std::array::from_fn(|axis| f32::from_bits(self.cameras[0][1 + axis]));
        self.tween = Some(Tween { mode, start, target: target.map(centi), roll: None, duration, elapsed: 0 });
    }

    fn start_tween(
        &mut self,
        machine: &Machine,
        opcode: u16,
    ) -> Result<u16, DecodeError> {
        let pc = machine.pc();
        let word = |index: usize| {
            machine
                .stack()
                .peek_u32(index * 4)
                .map_err(|e| error(pc, opcode, e.0))
        };
        let float = |index: usize| word(index).map(|raw| centi(f32::from_bits(raw)));
        let start: [f32; 3] = std::array::from_fn(|axis| f32::from_bits(self.cameras[0][1 + axis]));
        let mut target = start;
        let mut roll = None;
        let (words, duration) = match opcode {
            0x206b => {
                target[0] = float(2)?;
                target[2] = float(3)?;
                (4, word(4)?)
            }
            0x206c => {
                target = [float(2)?, float(3)?, float(4)?];
                (5, word(5)?)
            }
            0x206d => {
                target[1] = float(2)?;
                (3, word(3)?)
            }
            0x206e => {
                target[0] = float(2)?;
                (3, word(3)?)
            }
            0x206f => {
                target[2] = float(2)?;
                (3, word(3)?)
            }
            _ => {
                target = [float(3)?, float(4)?, float(5)?];
                roll = Some([f32::from_bits(self.cameras[0][14]), float(6)?]);
                (7, word(7)?)
            }
        };
        self.tween = Some(Tween {
            mode: word(1)?,
            start,
            target,
            roll,
            duration,
            elapsed: 0,
        });
        Ok(words * 4)
    }

    pub fn command(
        &mut self,
        machine: &mut Machine,
        opcode: u16,
    ) -> Result<Option<u16>, DecodeError> {
        let pc = machine.pc();
        match opcode {
            0x206b..=0x206f | 0x207d => return self.start_tween(machine, opcode).map(Some),
            0x206a => {
                machine
                    .state_mut()
                    .set_system_integer(0, u32::from(self.tween.is_some()))
                    .map_err(|why| error(pc, opcode, why))?;
                return Ok(Some(0));
            }
            0x2126 => {
                for (index, value) in (1..=2).zip(self.size) {
                    machine
                        .state_mut()
                        .set_system_integer(index, value)
                        .map_err(|why| error(pc, opcode, why))?;
                }
                return Ok(Some(0));
            }
            0x2250 => {
                self.depth_clear = machine
                    .stack()
                    .peek_u32(4)
                    .map_err(|e| error(pc, opcode, e.0))?
                    != 0;
                return Ok(Some(4));
            }
            0x2058..=0x2068 | 0x209c => {}
            _ => return Ok(None),
        }
        let indexed = !matches!(opcode, 0x2060..=0x2065 | 0x2068);
        let values = match opcode {
            0x2058 | 0x205a | 0x2060 | 0x2062 => 3,
            0x2059 | 0x205d | 0x2061 | 0x2065 | 0x2067 => 2,
            _ => 1,
        };
        let mut args = [0; 4];
        let count = values + usize::from(indexed);
        for (i, arg) in args[..count].iter_mut().enumerate() {
            *arg = machine
                .stack()
                .peek_u32((i + 1) * 4)
                .map_err(|e| error(pc, opcode, e.0))?;
        }
        let index = if indexed { args[0] as usize } else { 0 };
        let camera = self
            .cameras
            .get_mut(index)
            .ok_or_else(|| error(pc, opcode, "CMVS camera index outside 0..1"))?;
        let args = &args[usize::from(indexed)..];
        match opcode {
            0x2058 | 0x2060 => camera[6..9].copy_from_slice(&[args[2], args[0], args[1]]),
            0x2059 | 0x2061 => {
                camera[9..11].copy_from_slice(&args[..2]);
                for (i, raw) in args[..2].iter().enumerate() {
                    camera[15 + i] = (f32::from_bits(*raw) * 0.5).to_bits();
                }
            }
            0x205a | 0x2062 => {
                for (i, raw) in args[..3].iter().enumerate() {
                    camera[1 + i] = (truncate(f32::from_bits(*raw) * 100.) as f32 / 100.).to_bits();
                }
            }
            0x205b | 0x2063 => {
                let raw = truncate(f32::from_bits(args[0]) * 100.).wrapping_abs();
                camera[14] = ((raw % 36000) as f32 / 100.).to_bits();
            }
            0x205c | 0x2064 => camera[11] = args[0],
            0x205d | 0x2065 => camera[15..17].copy_from_slice(&args[..2]),
            0x205e => camera[37] = args[0],
            0x205f => camera[0] = args[0],
            0x2066 => camera[39] = args[0],
            0x2067 => camera[17..19].copy_from_slice(&args[..2]),
            0x2068 => {
                if camera[21] != 0 && args[0] != 0 {
                    camera.copy_within(23..26, 1);
                    for (bit, word) in [(0x80000000, 12), (0x40000000, 13), (0x20000000, 14)] {
                        if camera[22] & bit != 0 {
                            camera[word] = camera[word + 14];
                        }
                    }
                }
                camera[21] = 0;
            }
            0x209c => camera[38] = args[0],
            _ => unreachable!("recognized camera operation"),
        }
        Ok(Some((count * 4) as u16))
    }
}

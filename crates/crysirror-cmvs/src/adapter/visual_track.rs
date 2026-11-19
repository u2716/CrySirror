// SPDX-License-Identifier: MPL-2.0
//! CMVS draw record keyframe channels `0x21e0..=0x21fd`, apart from opacity.
//! The native classes are at 0x140037fe0..0x140039fc0 of the 2021-10-07 x64 build.

use anyhow::{ensure, Result};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Channel {
    Scale,
    Rotation,
    Frame,
    Position,
    World,
}

impl Channel {
    pub fn of(opcode: u16) -> Option<(Self, u16)> {
        let (channel, base) = match opcode {
            0x21e0..=0x21e4 => (Self::Scale, 0x21e0),
            0x21e5..=0x21e9 => (Self::Rotation, 0x21e5),
            0x21ef..=0x21f3 => (Self::Frame, 0x21ef),
            0x21f4..=0x21f8 => (Self::Position, 0x21f4),
            0x21f9..=0x21fd => (Self::World, 0x21f9),
            _ => return None,
        };
        Some((channel, opcode - base))
    }

    fn keys(self) -> usize {
        match self {
            Self::Frame => 64,
            Self::Position => 16,
            _ => 8,
        }
    }

    /// Stack words reclaimed by the create, key, start, finish and query commands.
    pub fn words(self, command: u16) -> u16 {
        match (self, command) {
            (Self::Scale, 0) => 4,
            (Self::Rotation, 0) => 3,
            (_, 0) => 2,
            (Self::Scale, 1) => 8,
            (Self::Rotation, 1) => 7,
            (Self::Frame, 1) => 11,
            (Self::Position, 1) => 8,
            (Self::World, 1) => 9,
            (_, 2) => 3,
            _ => 2,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct Key {
    next: i32,
    mode: u32,
    duration: u32,
    values: [f32; 6],
}

#[derive(Clone)]
pub(super) struct Track {
    channel: Channel,
    base: [f32; 6],
    playing: Option<(usize, u32)>,
    keys: Vec<Key>,
}

fn degrees(raw: i32) -> f32 {
    (raw % 360 * 100) as f32
}

impl Track {
    pub fn channel(&self) -> Channel {
        self.channel
    }

    pub fn playing(&self) -> bool {
        self.playing.is_some()
    }

    pub fn new(channel: Channel, base: [f32; 6]) -> Self {
        Self {
            channel,
            base,
            playing: None,
            keys: vec![Key::default(); channel.keys()],
        }
    }

    /// Stores a key from its native stack words, which begin with the key index.
    pub fn set_key(&mut self, words: &[u32]) -> Result<()> {
        let index = words[0] as usize;
        ensure!(index < self.keys.len(), "CMVS draw track key outside its channel");
        let float = |word: u32| f32::from_bits(word);
        let mut key = Key {
            next: words[1] as i32,
            ..Key::default()
        };
        match self.channel {
            Channel::Frame => {
                key.duration = words[2];
                for (value, word) in key.values.iter_mut().zip(&words[3..9]) {
                    *value = *word as i16 as f32;
                }
            }
            _ => {
                key.mode = words[2];
                key.duration = words[3];
                match self.channel {
                    Channel::Scale => key.values[..2].copy_from_slice(&[float(words[4]), float(words[5])]),
                    Channel::Rotation => key.values[0] = degrees(words[4] as i32),
                    Channel::Position => {
                        key.values[..2].copy_from_slice(&[words[4] as i16 as f32, words[5] as i16 as f32]);
                    }
                    _ => key.values[..3].copy_from_slice(&[float(words[4]), float(words[5]), float(words[6])]),
                }
            }
        }
        self.keys[index] = key;
        Ok(())
    }

    /// Starts at `index`. Frame keys take effect at once.
    pub fn start(&mut self, index: usize, base: [f32; 6]) -> Result<Option<[f32; 6]>> {
        ensure!(index < self.keys.len(), "CMVS draw track key outside its channel");
        self.base = base;
        self.playing = Some((index, 0));
        Ok((self.channel == Channel::Frame).then_some(self.keys[index].values))
    }

    /// Final values of the active key, applied when a script finishes the track.
    pub fn finish(&self) -> Option<[f32; 6]> {
        self.playing.map(|(index, _)| self.keys[index].values)
    }

    /// `-1` without a track is reported by the caller; `-2` means created but idle.
    pub fn state(&self) -> u32 {
        self.playing.map_or(u32::MAX - 1, |(index, _)| index as u32)
    }

    fn interpolate(&self, key: &Key, elapsed: u32) -> [f32; 6] {
        let progress = elapsed as f32 / key.duration as f32;
        let relative = key.mode & 0x8000 != 0;
        let mut out = self.base;
        for axis in 0..6 {
            let delta = if relative {
                key.values[axis]
            } else {
                key.values[axis] - self.base[axis]
            };
            let quarter = progress * std::f32::consts::FRAC_PI_2;
            let eased = match key.mode & 0xff {
                1 => quarter.sin(),
                2 => 1.0 - quarter.cos(),
                _ => progress,
            };
            out[axis] = self.base[axis] + delta * eased;
        }
        out
    }

    /// Advances by `delta` milliseconds and returns new values with any terminal next index.
    pub fn advance(&mut self, delta: u32) -> (Option<[f32; 6]>, Option<i32>) {
        let Some((mut index, elapsed)) = self.playing else {
            return (None, None);
        };
        let mut elapsed = elapsed.wrapping_add(delta.min(200));
        let key = self.keys[index];
        if self.channel == Channel::Rotation && key.mode & 0xff == 0x80 {
            let period = key.duration.max(1);
            elapsed %= period;
            let turn = (36000 * elapsed / period) as f32;
            self.playing = Some((index, elapsed));
            let mut out = self.base;
            out[0] = (self.base[0] + turn * (key.values[0] / 36000.0)).rem_euclid(36000.0);
            return (Some(out), None);
        }
        if key.duration > elapsed {
            self.playing = Some((index, elapsed));
            return if self.channel == Channel::Frame {
                (None, None)
            } else {
                (Some(self.interpolate(&key, elapsed)), None)
            };
        }
        let mut values = None;
        for _ in 0..=self.keys.len() {
            let key = self.keys[index];
            let target = if key.mode & 0x8000 != 0 {
                std::array::from_fn(|axis| self.base[axis] + key.values[axis])
            } else {
                key.values
            };
            self.base = target;
            values = Some(target);
            if key.next < 0 {
                self.playing = None;
                return (values, Some(key.next));
            }
            elapsed -= key.duration;
            index = key.next as usize;
            if index >= self.keys.len() {
                self.playing = None;
                return (values, Some(-1));
            }
            if self.channel == Channel::Frame {
                values = Some(self.keys[index].values);
            }
            if self.keys[index].duration > elapsed {
                break;
            }
        }
        self.playing = Some((index, elapsed));
        (values, None)
    }
}

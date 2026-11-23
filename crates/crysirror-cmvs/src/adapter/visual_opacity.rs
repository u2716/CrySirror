// SPDX-License-Identifier: MPL-2.0
//! CMVS draw record's eight-key opacity channel.

use crate::vm::CmvsVm;
use anyhow::{ensure, Result};

#[derive(Clone, Copy, Default)]
struct Key {
    next: i32,
    target: i32,
    hold: bool,
    duration: u32,
}

#[derive(Clone)]
pub(super) struct Opacity {
    base: i32,
    playing: Option<(usize, u32)>,
    keys: [Key; 8],
}

impl Opacity {
    pub fn playing(&self) -> bool {
        self.playing.is_some()
    }

    pub fn command(
        opcode: u16,
        vm: &CmvsVm,
        track: &mut Option<Self>,
        opacity: &mut i32,
    ) -> Result<(u16, Option<u32>)> {
        let magnitude = |raw: u32| (raw as i32).wrapping_abs() % 256;
        let (words, result) = match opcode {
            0x21ea => {
                let [_, _, base] = vm.owner_arguments(opcode)?;
                *track = Some(Self {
                    base: magnitude(base),
                    playing: None,
                    keys: [Key::default(); 8],
                });
                (3, None)
            }
            0x21eb => {
                let [_, _, index, next, hold, duration, target] = vm.owner_arguments(opcode)?;
                ensure!(
                    index < 8 && (next as i32) < 8,
                    "CMVS opacity key outside 0..7"
                );
                if let Some(track) = track {
                    track.keys[index as usize] = Key {
                        next: next as i32,
                        target: magnitude(target),
                        hold: hold & 255 != 0,
                        duration,
                    };
                }
                (7, None)
            }
            0x21ec => {
                let [_, _, index] = vm.owner_arguments(opcode)?;
                ensure!(index < 8, "CMVS opacity key outside 0..7");
                if let Some(track) = track {
                    track.playing = Some((index as usize, 0));
                }
                (3, None)
            }
            0x21ed => {
                if let Some(track) = track.take() {
                    if let Some((index, _)) = track.playing {
                        *opacity = track.keys[index].target;
                    }
                }
                (2, None)
            }
            _ => (
                2,
                Some(track.as_ref().map_or(u32::MAX, |track| {
                    track
                        .playing
                        .map_or(u32::MAX - 1, |(index, _)| index as u32)
                })),
            ),
        };
        Ok((words, result))
    }

    pub fn advance(&mut self, delta: u32, opacity: &mut i32) -> Option<i32> {
        let (mut index, elapsed) = self.playing?;
        let mut elapsed = elapsed.wrapping_add(delta.min(200));
        let key = self.keys[index];
        if key.duration > elapsed {
            if !key.hold {
                let span = (key.target.wrapping_sub(self.base) as u32).wrapping_mul(elapsed) as i32;
                *opacity = self
                    .base
                    .wrapping_add(span.wrapping_div(key.duration as i32));
            }
        } else {
            for _ in 0..64 {
                let key = self.keys[index];
                self.base = key.target;
                *opacity = key.target;
                if key.next < 0 {
                    return Some(key.next);
                }
                elapsed -= key.duration;
                index = key.next as usize;
                if self.keys[index].duration > elapsed {
                    break;
                }
            }
        }
        self.playing = Some((index, elapsed));
        None
    }
}

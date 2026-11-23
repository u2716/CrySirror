// SPDX-License-Identifier: MPL-2.0
//! Native four-lane ambient definitions and alternating v1 playback banks.
use crate::{
    branch::number,
    registers::{Registers, Value},
    sound::stereo,
};
use anyhow::{bail, ensure, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{
        AudioAsset, AudioParameters, AudioPcm16Gain, AudioPreparePhase, AudioRequest, HostServices,
        PcmBus,
    },
    state::ClientId,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Lane {
    effect: i32,
    mode: i32,
    delay: i32,
    levels: [i32; 2],
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Definition {
    id: i32,
    lanes: [Lane; 4],
}
impl Definition {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let numbers: Vec<i32> = bytes
            .split(|&b| b == b',')
            .take(22)
            .map(number)
            .collect::<Result<_>>()?;
        ensure!(
            numbers.len() == 21,
            "invalid ExHibit environment definition"
        );
        Ok(Self {
            id: numbers[0],
            lanes: std::array::from_fn(|i| {
                let at = 1 + 5 * i;
                Lane {
                    effect: numbers[at],
                    mode: numbers[at + 1],
                    delay: numbers[at + 2],
                    levels: [numbers[at + 3], numbers[at + 4]],
                }
            }),
        })
    }
    fn builtin(id: i32) -> Option<Self> {
        let (effect, level) = match id {
            1000 => (76, 256),
            1001 => (76, 96),
            1002 => (72, 128),
            1003 => (72, 256),
            1004 | 9999 => (71, 256),
            1005 => (71, 144),
            1006 => (70, 144),
            1007 => (70, 96),
            _ => return None,
        };
        let mut lanes = [Lane {
            effect: -1,
            mode: -1,
            delay: -1,
            levels: [-1; 2],
        }; 4];
        lanes[0] = Lane {
            effect,
            mode: 2,
            delay: 0,
            levels: [level; 2],
        };
        Some(Self { id, lanes })
    }
    fn sources(
        &self,
        r: &Registers,
        base: u32,
        fade: i32,
        overrides: [i32; 2],
    ) -> Result<Vec<(u32, AudioRequest)>> {
        let mut out = Vec::new();
        for (i, lane) in self.lanes.iter().enumerate().filter(|(_, l)| l.effect >= 0) {
            ensure!(
                matches!(lane.mode, 0 | 2),
                "unsupported ExHibit environment mode {} delay {}",
                lane.mode,
                lane.delay
            );
            let levels = std::array::from_fn(|side| {
                if i == 0 && overrides[side] >= 0 {
                    overrides[side]
                } else {
                    lane.levels[side]
                }
            });
            let path = VfsPath::new(Mount::Asset, &format!("res/s/e/{:03}.wav", lane.effect))?;
            let channel = base + i as u32;
            out.push((
                channel,
                AudioRequest::PlayAsset {
                    channel,
                    source: AudioAsset {
                        path,
                        frame_limit: None,
                    },
                    intro: None,
                    parameters: AudioParameters {
                        bus: PcmBus::Ambient,
                        looping: lane.mode == 2,
                        fade_ms: if i == 0 { fade.max(0) as f32 } else { 0. },
                        pcm16_gain: Some(AudioPcm16Gain {
                            stereo: stereo(r, 35, levels)?,
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                },
            ));
        }
        Ok(out)
    }
}
#[derive(Default)]
pub(super) struct Environment {
    definitions: BTreeMap<i32, Definition>,
    current: Option<i32>,
    bank: Option<u32>,
    tickets: Vec<u64>,
    retiring: Option<u32>,
}
impl Environment {
    pub fn define(&mut self, def: Definition) -> Result<()> {
        ensure!(
            self.definitions.contains_key(&def.id) || self.definitions.len() < 65536,
            "ExHibit environment definition limit"
        );
        self.definitions.insert(def.id, def);
        Ok(())
    }
    pub fn preparing(&self) -> bool {
        !self.tickets.is_empty()
    }
    fn stop_bank(h: &mut dyn HostServices, c: ClientId, base: u32) -> Result<()> {
        for channel in base..base + 4 {
            h.submit_audio(
                c,
                AudioRequest::Stop {
                    channel,
                    fade_ms: 0.,
                },
            )?;
        }
        Ok(())
    }
    pub fn play(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        r: &Registers,
        id: i32,
        fade: i32,
        levels: [i32; 2],
        scene: &mut [i32; 12],
    ) -> Result<()> {
        if id < 0 || self.current == Some(id) {
            return Ok(());
        }
        if r.read(Value::System(34))? != 0 {
            scene[7..10].copy_from_slice(&[id, levels[0], levels[1]]);
            return Ok(());
        }
        let def = self
            .definitions
            .get(&id)
            .copied()
            .or_else(|| Definition::builtin(id))
            .ok_or_else(|| anyhow::anyhow!("ExHibit environment {id} has no loaded definition"))?;
        let base = if self.bank == Some(4) { 8 } else { 4 };
        let sources = def.sources(r, base, fade, levels)?;
        for (_, request) in sources {
            match h.begin_audio(c, request) {
                Ok(ticket) => self.tickets.push(ticket),
                Err(e) => {
                    for ticket in self.tickets.drain(..) {
                        let _ = h.release_audio(c, ticket);
                    }
                    Self::stop_bank(h, c, base)?;
                    return Err(e.into());
                }
            }
        }
        if let Some(old) = self.bank.filter(|_| self.current.is_some()) {
            if fade > 0 && h.audio_playing(c, old)? {
                h.submit_audio(
                    c,
                    AudioRequest::Stop {
                        channel: old,
                        fade_ms: fade as f32,
                    },
                )?;
                self.retiring = Some(old);
            } else {
                Self::stop_bank(h, c, old)?;
            }
        }
        self.current = Some(id);
        self.bank = Some(base);
        scene[7..10].copy_from_slice(&[id, levels[0], levels[1]]);
        Ok(())
    }
    pub fn stop(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        fade: i32,
        scene: &mut [i32; 12],
    ) -> Result<Option<u32>> {
        let mut wait = None;
        if let Some(base) = self.bank.filter(|_| self.current.is_some()) {
            if fade <= 0 {
                Self::stop_bank(h, c, base)?;
            } else {
                h.submit_audio(
                    c,
                    AudioRequest::Stop {
                        channel: base,
                        fade_ms: fade as f32,
                    },
                )?;
                wait = Some(base);
            }
        }
        self.current = None;
        scene[7..10].fill(-1);
        Ok(wait)
    }
    pub fn poll(&mut self, h: &mut dyn HostServices, c: ClientId) -> Result<bool> {
        while let Some(&ticket) = self.tickets.last() {
            let state = h.audio_preparation(c, ticket)?;
            if state.phase == AudioPreparePhase::Pending {
                return Ok(false);
            }
            h.release_audio(c, ticket)?;
            self.tickets.pop();
            if state.phase == AudioPreparePhase::Failed {
                for ticket in self.tickets.drain(..) {
                    let _ = h.release_audio(c, ticket);
                }
                if let Some(base) = self.bank {
                    Self::stop_bank(h, c, base)?;
                }
                if let Some(base) = self.retiring.take() {
                    Self::stop_bank(h, c, base)?;
                }
                bail!("ExHibit environment preparation failed: {:?}", state.error)
            }
        }
        if let Some(old) = self.retiring {
            if h.audio_info(c, old)?
                .is_some_and(|a| a.stopping && a.fade_out_frames > 0)
            {
                return Ok(false);
            }
            Self::stop_bank(h, c, old)?;
            self.retiring = None;
        }
        Ok(true)
    }
}

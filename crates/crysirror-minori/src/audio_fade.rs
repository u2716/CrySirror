// SPDX-License-Identifier: MPL-2.0
//! Native sound clocks and logical-to-physical channel ownership.
use super::{
    AudioParameters, AudioRequest, ClientId, HostServices, PcmBus, Presentation, Result, ScreenAbi,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
enum Clock {
    PercentStep,
    Duration,
}

struct Fade {
    clock: Clock,
    start: u64,
    period: u32,
    from: i32,
    to: i32,
    stop: bool,
}

impl Fade {
    fn level(&mut self, tick: u64) -> i32 {
        let elapsed = tick.saturating_sub(self.start);
        let difference = self.to - self.from;
        if self.period == 0 {
            return self.to;
        }
        let delta = match self.clock {
            Clock::PercentStep => {
                if elapsed >= u64::from(self.period) {
                    self.from += difference.signum();
                    self.start = tick;
                }
                return self.from;
            }
            Clock::Duration => {
                (i64::from(difference) * elapsed.min(u64::from(self.period)) as i64
                    / i64::from(self.period)) as i32
            }
        };
        self.from + delta
    }
}

struct Playing {
    name: String,
    pan: i32,
    level: i32,
    target: i32,
    looping: bool,
    fade: Option<Fade>,
}

#[derive(Default)]
pub(super) struct SoundSlots {
    slots: BTreeMap<u32, u32>,
    voices: BTreeMap<u32, Playing>,
}
pub(super) fn amplitude(percent: i32) -> f32 {
    if percent <= 0 {
        return 0.;
    }
    let db100 = (2000. * (f64::from(percent.min(100)) / 100.).log10()).trunc();
    10.0_f64.powf(db100 / 2000.) as f32
}
pub(super) fn balance(pan: i32) -> (f32, f32) {
    let left = amplitude(100 - pan.max(0));
    let right = amplitude(100 + pan.min(0));
    let power = left * left + right * right;
    ((power / 2.).sqrt(), right * right / power)
}

impl Presentation {
    pub fn sound_fading(&self) -> bool {
        self.sounds.voices.values().any(|v| v.fade.is_some())
    }
    pub fn poll_fades(&mut self, h: &mut dyn HostServices, c: ClientId) -> Result<()> {
        let now = h.now_ms() / 10;
        let mut remove = Vec::new();
        for (&channel, voice) in &mut self.sounds.voices {
            let Some(fade) = &mut voice.fade else {
                continue;
            };
            let level = fade.level(now);
            if level != voice.level {
                h.submit_audio(
                    c,
                    AudioRequest::SetGain {
                        channel,
                        gain: amplitude(level) * balance(voice.pan).0,
                        fade_ms: 0.,
                    },
                )?;
                voice.level = level;
            }
            let complete = match fade.clock {
                Clock::Duration => now.saturating_sub(fade.start) >= u64::from(fade.period),
                Clock::PercentStep => level == fade.to,
            };
            if complete {
                if fade.stop {
                    h.submit_audio(
                        c,
                        AudioRequest::Stop {
                            channel,
                            fade_ms: 0.,
                        },
                    )?;
                    remove.push(channel);
                }
                voice.fade = None;
            }
        }
        for channel in remove {
            self.sounds.voices.remove(&channel);
            self.audio_channels.remove(&channel);
        }
        Ok(())
    }

    pub(super) fn play_sound_slot(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        slot: u32,
        name: String,
        looping: bool,
        level: i32,
        pan: i32,
        fade_in: i32,
        fade_out: i32,
    ) -> Result<()> {
        self.poll_fades(h, c)?;
        let clock = if self.screen_abi == ScreenAbi::PersistentZoom {
            Clock::Duration
        } else {
            Clock::PercentStep
        };
        let now = h.now_ms() / 10;
        let fade = |from, to, operand: i32, stop| Fade {
            clock,
            start: now,
            from,
            to,
            period: operand.max(0) as u32,
            stop,
        };
        let music = slot == 0 || slot == super::SECOND_MUSIC_CHANNEL;
        let key = if music { "volumeBGM" } else { "volumeSE" };
        let level = level.clamp(0, 100) * self.sound_settings.get(key, 100).clamp(0, 100) / 100;
        let muted = self
            .sound_settings
            .get(if music { "muteBGM" } else { "muteSE" }, 0)
            != 0;
        if let Some(&channel) = self.sounds.slots.get(&slot) {
            let old = self
                .sounds
                .voices
                .get_mut(&channel)
                .expect("live minori sound slot");
            if !muted
                && old.looping
                && looping
                && old.name == name
                && h.audio_playing(c, channel)?
            {
                if old.pan != pan {
                    old.pan = pan;
                    h.submit_audio(
                        c,
                        AudioRequest::SetPan {
                            channel,
                            pan: balance(pan).1,
                            fade_ms: 0.,
                        },
                    )?;
                    h.submit_audio(
                        c,
                        AudioRequest::SetGain {
                            channel,
                            gain: amplitude(old.level) * balance(pan).0,
                            fade_ms: 0.,
                        },
                    )?;
                }
                if old.target != level {
                    old.fade = Some(fade(old.level, level, if music {fade_in} else {0}, false));
                    old.target = level;
                }
                return self.poll_fades(h, c);
            }
            self.sounds.slots.remove(&slot);
            old.fade = Some(fade(old.level, 0, fade_out, true));
            self.poll_fades(h, c)?;
        }
        if muted || name.is_empty() || name == "*" {
            return Ok(());
        }
        let channel = if !self.sounds.voices.contains_key(&slot) {
            slot
        } else {
            (32..)
                .find(|n| !self.sounds.voices.contains_key(n))
                .expect("available minori mixer channel")
        };
        let initial = if fade_in > 0 { 0 } else { level };
        let (scale, pan_host) = balance(pan);
        self.sound(
            h,
            c,
            channel,
            if music { "bgm" } else { "se" },
            &name,
            AudioParameters {
                bus: if music { PcmBus::Music } else { PcmBus::Sfx },
                looping,
                gain: amplitude(initial) * scale,
                pan: pan_host,
                ..Default::default()
            },
        )?;
        if self.audio_ticket.is_some() {
            self.sounds.voices.insert(
                channel,
                Playing {
                    name,
                    pan,
                    level: initial,
                    target: level,
                    looping,
                    fade: (initial != level).then(|| fade(initial, level, fade_in, false)),
                },
            );
            self.sounds.slots.insert(slot, channel);
        }
        Ok(())
    }
}

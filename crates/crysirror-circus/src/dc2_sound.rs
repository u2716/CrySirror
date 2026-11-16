//! DC2 native0x5b sound effects (41C530 case91 -> 427070/427240/4272B0,
//! 427360 on timer2). Shared Audio owns decoding and mixing; the VM keeps
//! the two-channel selection, directory attenuation and fade policy.
use super::*;
use crate::dc2_audio::{MusicLookup, Request, SoundFade};
use crate::machine::Event;
///Native SE channels1/2 publish on host channels2/3.
const fn host_channel(channel: u8) -> u32 {
    channel as u32 + 1
}
impl Machine {
    fn settings_dword(&self, at: usize) -> Result<i32> {
        Ok(i32::from_le_bytes(
            self.state
                .settings
                .bytes()
                .get(at..at + 4)
                .context("DC2 native SE preference field")?
                .try_into()?,
        ))
    }
    ///426BE0(channel,0): CEBB58 attenuation plus the SE volume (52B3E4).
    fn sound_level(&self, channel: u8) -> Result<i32> {
        let volume = self.settings_dword(52)?.wrapping_mul(2);
        if volume <= 0 {
            return Ok(-10000);
        }
        Ok(self.state.audio.sound_attenuation_db100[usize::from(channel - 1)]
            .wrapping_add(20i32.wrapping_mul(5i32.wrapping_mul(volume).wrapping_sub(300)) / 2)
            .clamp(-3000, 0))
    }
    pub(super) fn execute_sound(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>> {
        if instruction.opcode != 0x5b {
            return Ok(None);
        }
        let (id, mode) = (a[0], i32::from(a[1]));
        if id == -1 {
            if mode == 1 {
                //4272B0 + timer2: fade every loaded channel at150/10ms.
                let mut fade = self.state.audio.sound_fade.clone().unwrap_or(SoundFade {
                    mask: 0,
                    level_db100: [0; 2],
                    tick_ms: now,
                });
                for channel in [1u8, 2] {
                    let bit = 1 << (channel - 1);
                    if fade.mask & bit == 0 && host.audio_playing(host_channel(channel))? {
                        fade.mask |= bit;
                        fade.level_db100[usize::from(channel - 1)] = self.sound_level(channel)?;
                        if channel == 1 {
                            self.state.audio.sound_held = 0;
                        }
                    }
                }
                self.state.audio.sound_fade = (fade.mask != 0).then_some(fade);
            } else {
                //427240 stops both channels at once and cancels the fade.
                for channel in [1u8, 2] {
                    if !host.emit(&Event::StopAudio { channel: host_channel(channel) })? {
                        return Ok(Some(Outcome::Pending));
                    }
                }
                self.state.audio.sound_fade = None;
                self.state.audio.sound_held = 0;
            }
            self.state.audio.sound_record = 0;
            return self.advance(instruction.next, instruction.opcode).map(Some);
        }
        //427070 returns for an out-of-directory ID or with SE disabled (52B3E0).
        let enabled = self.settings_dword(48)? != 0;
        if id >= 0 && enabled {
            let Some(row) = host.dc2_sound(id as u16)? else {
                return Ok(Some(Outcome::Pending));
            };
            if let MusicLookup::Found { name, attenuation_db100 } = row {
                let audio = &self.state.audio;
                let channel = if let Some((_, channel)) = audio.sound_pending.filter(|(pc, _)| *pc == instruction.offset) {
                    channel
                } else if mode != 0 {
                    1
                } else if audio.sound_held != 0 {
                    2
                } else if !host.audio_playing(host_channel(1))? {
                    1
                } else if host.audio_playing(host_channel(2))? && audio.sound_last_channel != 1 {
                    1
                } else {
                    2
                };
                self.state.audio.sound_pending = Some((instruction.offset, channel));
                //426760 resets CEBB58 at load, then427070 applies the row.
                self.state.audio.sound_attenuation_db100[usize::from(channel - 1)] = attenuation_db100;
                let gain_db100 = self.sound_level(channel)?;
                if !host.direct_audio(&Request {
                    name,
                    channel: host_channel(channel),
                    voice: false,
                    gain_db100,
                    balance_db100: 0,
                    looping: mode == 1,
                    streamed: false,
                })? {
                    return Ok(Some(Outcome::Pending));
                }
                let audio = &mut self.state.audio;
                audio.sound_pending = None;
                if mode != 0 {
                    audio.sound_held = mode;
                } else if channel == 1 {
                    audio.sound_held = 0;
                }
                audio.sound_last_channel = channel;
                if let Some(fade) = &mut audio.sound_fade {
                    fade.mask &= !(1 << (channel - 1));
                    if fade.mask == 0 {
                        audio.sound_fade = None;
                    }
                }
            }
        }
        self.state.audio.sound_record = i32::from(id as u16) | (mode << 16);
        self.advance(instruction.next, instruction.opcode).map(Some)
    }
    ///427360 on the10ms timer.
    pub(super) fn tick_sound_fade(&mut self, host: &mut dyn NativeHost, now: u64) -> Result<bool> {
        let Some(mut fade) = self.state.audio.sound_fade.clone() else { return Ok(true) };
        let ticks = now.saturating_sub(fade.tick_ms) / 10;
        if ticks == 0 {
            return Ok(true);
        }
        fade.tick_ms += ticks * 10;
        for channel in [1u8, 2] {
            let bit = 1 << (channel - 1);
            if fade.mask & bit == 0 {
                continue;
            }
            let level = &mut fade.level_db100[usize::from(channel - 1)];
            for _ in 0..ticks.min(1000) {
                if *level <= -5000 {
                    break;
                }
                *level -= 150;
            }
            if *level > -5000 && host.audio_playing(host_channel(channel))? {
                host.direct_audio_gain(host_channel(channel), (*level).max(-10000))?;
            } else {
                if !host.emit(&Event::StopAudio { channel: host_channel(channel) })? {
                    return Ok(false);
                }
                fade.mask &= !bit;
                if channel == 1 {
                    self.state.audio.sound_held = 0;
                }
            }
        }
        self.state.audio.sound_fade = (fade.mask != 0).then_some(fade);
        Ok(true)
    }
}

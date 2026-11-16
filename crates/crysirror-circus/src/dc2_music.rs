//! DC2 native05/06 music policy (426D40/426BE0/4277E0). Native music
//! sources use engine streaming; this module retains IDs, enable/seen fields,
//! source attenuation and repeat policy rather than a second mixer.
use super::*;
use crate::dc2_audio::{MusicLookup, Request};
use crate::machine::Event;
impl Machine {
    pub(super) fn execute_music(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>> {
        if !matches!(instruction.opcode, 5 | 6) {
            return Ok(None);
        }
        if instruction.opcode == 6 {
            ensure!(
                !host.audio_playing(0)?,
                "DC2 native active-music attenuation requires its shared gain update receipt"
            );
            self.state.audio.music_parameter = a[0].into();
            return self.advance(instruction.next, instruction.opcode).map(Some);
        }
        let mut next = instruction.next;
        let mut parameter = 0;
        if let Ok(extension) = program.instruction(next) {
            if extension.opcode == 6 {
                let Arguments::Bytes(bytes) = extension.arguments else {
                    bail!("DC2 music attenuation operand layout")
                };
                parameter = i32::from(bytes[0]);
                next = extension.next;
            }
        }
        if a[0] == 255 {
            //426FF0 stops at once and cancels a running timer fade.
            if !host.emit(&Event::StopAudio { channel: 0 })? {
                return Ok(Some(Outcome::Pending));
            }
            self.state.audio.music_fade = None;
            self.state.runtime.insert(244, -1);
            return self.advance(next, instruction.opcode).map(Some);
        }
        let id = if a[0] == 254 {
            self.state.runtime.get(&254).copied().unwrap_or(0)
        } else {
            i32::from(a[0])
        };
        if !(0..=2).contains(&a[1]) || id < 0 {
            self.state.runtime.insert(244, id);
            return self.advance(next, instruction.opcode).map(Some);
        }
        if a[1] == 1 && self.state.runtime.get(&244).copied().unwrap_or(-1) == id {
            ensure!(
                self.state.audio.music_parameter == parameter,
                "DC2 unchanged-music native attenuation update requires its shared gain receipt"
            );
            return self.advance(next, instruction.opcode).map(Some);
        }
        let enabled = i32::from_le_bytes(self.state.settings.bytes()[36..40].try_into()?) != 0;
        if enabled {
            let Some(row) = host.dc2_music(u16::try_from(id)?)? else {
                return Ok(Some(Outcome::Pending));
            };
            if let MusicLookup::Found {
                name,
                attenuation_db100,
            } = row
            {
                //426D40 marks the accepted original directory ID before buffer
                //publication. Commit both only after asynchronous readiness here.
                if let Some(pc) = self.state.audio.music_preparing_pc {
                    ensure!(
                        pc == instruction.offset,
                        "DC2 music preparation changed native PC"
                    );
                } else {
                    //426D40 with music playing: fade the old one at800/tick
                    //(CFAC48) and start the queued ID when it reaches -5000.
                    //The queued start is held at this PC (at most ~70ms).
                    if host.audio_playing(0)? {
                        if let Some(fade) = &mut self.state.audio.music_fade {
                            fade.step_db100 = 800;
                        } else {
                            self.begin_music_fade(host, now, 800)?;
                        }
                        return Ok(Some(Outcome::WaitUntil(now + 10)));
                    }
                    self.state.audio.music_preparing_pc = Some(instruction.offset);
                }
                let offset = if id >= 80 { 60 } else { 44 };
                let volume =
                    i32::from_le_bytes(self.state.settings.bytes()[offset..offset + 4].try_into()?);
                let level = volume.wrapping_mul(2).wrapping_sub(parameter);
                let gain_db100 = if level <= 0 {
                    -10000
                } else {
                    attenuation_db100
                        .wrapping_add(
                            20i32.wrapping_mul(5i32.wrapping_mul(level).wrapping_sub(300)) / 2,
                        )
                        .clamp(-3000, 0)
                };
                self.state.audio.music_gain_db100 = gain_db100;
                if !host.direct_audio(&Request {
                    name,
                    channel: 0,
                    voice: false,
                    gain_db100,
                    balance_db100: 0,
                    looping: a[1] != 2,
                    streamed: true,
                })? {
                    return Ok(Some(Outcome::Pending));
                }
                self.state.audio.music_preparing_pc = None;
                if id < 100 {
                    self.state.settings.set_byte(95, id as i16, 1)?;
                }
            }
        }
        self.state.audio.music_parameter = parameter;
        self.state.runtime.insert(244, id);
        self.advance(next, instruction.opcode).map(Some)
    }
}

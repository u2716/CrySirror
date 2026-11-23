//! DC2 native0x38 (41C530 case56 -> 422E60 case56). The VM keeps the
//! NoMovie gate, the 426FF0 music stop and the 427820/427810 audio device
//! release around the blocking 4153E0 call; shared Media owns decoding,
//! presentation and the right-button/Escape skip.
use super::*;
use crate::machine::Event;
use crate::machine::movie_ops::MovieRequest;
use crate::machine::text_ops::{TextReply, TextRequest};
impl Machine {
    pub(super) fn execute_movie(
        &mut self,
        instruction: &Instruction,
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>> {
        if instruction.opcode != 0x38 {
            return Ok(None);
        }
        //A240E8 is the HKCU "NoMovie" value; an absent value reads0. The
        //portable host has no registry, so the native default applies.
        if self.state.movie_pc != Some(instruction.offset) {
            //426FF0(0): stop music and its timer fade (B0AB1C = -1).
            for channel in [0, 2, 3, 8] {
                if !host.emit(&Event::StopAudio { channel })? {
                    return Ok(Some(Outcome::Pending));
                }
            }
            self.state.runtime.insert(244, -1);
            self.state.audio.music_fade = None;
            //427820 releases every DirectSound buffer and timer2.
            self.state.audio.sound_fade = None;
            if host
                .text(&TextRequest::Movie(MovieRequest::Start { name: text.into(), now, gain: 1. }))?
                .is_none()
            {
                return Ok(Some(Outcome::Pending));
            }
            self.state.movie_pc = Some(instruction.offset);
        }
        match host.text(&TextRequest::Movie(MovieRequest::Poll { now }))? {
            Some(TextReply::Input { advance: true }) | Some(TextReply::Done) => {
                self.state.movie_pc = None;
                self.advance(instruction.next, instruction.opcode).map(Some)
            }
            Some(TextReply::Input { advance: false }) | None => {
                Ok(Some(Outcome::WaitUntil(now + 10)))
            }
            _ => bail!("DC2 movie playback receipt"),
        }
    }
}

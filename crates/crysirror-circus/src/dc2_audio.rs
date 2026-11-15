//! DC2 voice36/427540 policy. Shared Audio owns PCM decoding, transport,
//! gain and stereo dB balance; the VM retains only source-enabled branches.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub name: String,
    pub channel: u32,
    pub voice: bool,
    pub gain_db100: i32,
    pub balance_db100: i32,
    pub looping: bool,
    ///Long native music is decoded by the engine's seekable asset stream.
    pub streamed: bool,
}
#[derive(Clone, Debug)]
pub enum MusicLookup {
    Missing,
    Found {
        name: String,
        attenuation_db100: i32,
    },
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AudioState {
    pub last_voice: String,
    ///0x36 whose voice load is in flight: native 427540 is synchronous, so
    ///the skip decision made when it started holds until it completes.
    #[serde(default)]
    pub voice_pc: Option<u32>,
    pub music_parameter: i32,
    ///The prepared paused source is already observable as source_playing.
    ///Only this native instruction may resume its own preparation receipt.
    pub music_preparing_pc: Option<u32>,
    ///426BE0 level applied when the current music started (CFA218 seed).
    #[serde(default)]
    pub music_gain_db100: i32,
    ///426F30 timer-2 fade: level, last10ms tick, step (150, or800 when a
    ///replacement is queued by426D40).
    #[serde(default)]
    pub music_fade: Option<MusicFade>,
    ///Native SE channels1/2 (host2/3): CFACB4 last channel, CFAC40 held
    ///non-zero mode of channel1, B0AB24 last SE record (id | mode<<16).
    #[serde(default)]
    pub sound_last_channel: u8,
    #[serde(default)]
    pub sound_held: i32,
    #[serde(default)]
    pub sound_record: i32,
    ///CEBB58[1..2]: per-channel directory attenuation set at load.
    #[serde(default)]
    pub sound_attenuation_db100: [i32; 2],
    ///4272B0/427360 timer-2 SE fade: CFAC3C mask, CEBB5C/CEBB60 levels.
    #[serde(default)]
    pub sound_fade: Option<SoundFade>,
    ///Channel chosen by 427070 for the SE instruction at this PC while its
    ///shared preparation is pending.
    #[serde(default)]
    pub sound_pending: Option<(u32, u8)>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoundFade {
    pub mask: u8,
    pub level_db100: [i32; 2],
    pub tick_ms: u64,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MusicFade {
    pub level_db100: i32,
    pub tick_ms: u64,
    pub step_db100: i32,
}
impl Machine {
    ///426F30 on the10ms timer: lower the playing music by the step until
    ///-5000, then stop it (426C60/426B60). Script execution is not blocked.
    pub(super) fn tick_music_fade(&mut self, host: &mut dyn NativeHost, now: u64) -> Result<bool> {
        let Some(mut fade) = self.state.audio.music_fade.clone() else { return Ok(true) };
        if !host.audio_playing(0)? {
            self.state.audio.music_fade = None;
            return Ok(true);
        }
        let ticks = now.saturating_sub(fade.tick_ms) / 10;
        if ticks == 0 { return Ok(true); }
        fade.tick_ms += ticks * 10;
        let mut level = fade.level_db100;
        for _ in 0..ticks.min(1000) {
            if level <= -5000 { break; }
            level -= fade.step_db100;
        }
        if level > -5000 {
            host.direct_audio_gain(0, level.max(-10000))?;
            fade.level_db100 = level;
            self.state.audio.music_fade = Some(fade);
            return Ok(true);
        }
        if !host.emit(&crate::machine::Event::StopAudio { channel: 0 })? {
            return Ok(false);
        }
        self.state.audio.music_fade = None;
        Ok(true)
    }
    ///426EE0: start the music fade-out unless one is already running.
    pub(super) fn begin_music_fade(&mut self, host: &mut dyn NativeHost, now: u64, step_db100: i32) -> Result<()> {
        if self.state.audio.music_fade.is_none() && host.audio_playing(0)? {
            self.state.audio.music_fade = Some(MusicFade {
                level_db100: self.state.audio.music_gain_db100,
                tick_ms: now,
                step_db100,
            });
        }
        Ok(())
    }
}
impl Machine {
    pub(super) fn execute_audio(
        &mut self,
        instruction: &Instruction,
        text: &str,
        host: &mut dyn NativeHost,
        control: bool,
    ) -> Result<Option<Outcome>> {
        if instruction.opcode != 0x36 {
            return Ok(None);
        }
        ensure!(
            !matches!(text, "-" | "--"),
            "DC2 voice replay/stop form requires its original resource state"
        );
        let skipping = self.state.runtime.get(&25).copied().unwrap_or(0) == 0
            && (control || self.state.runtime.get(&242).copied().unwrap_or(0) != 0);
        let started = self.state.audio.voice_pc == Some(instruction.offset);
        if !started && skipping && self.state.script_bytes[825] != 1 {
            return self.advance(instruction.next, instruction.opcode).map(Some);
        }
        if text.is_empty() { return self.advance(instruction.next, instruction.opcode).map(Some); }
        let (name, balance_db100) = match text.as_bytes()[0] {
            b'<' => (&text[1..], -1000),
            b'>' => (&text[1..], 1000),
            b'=' => (&text[1..], 0),
            _ => (text, 0),
        };
        let Some(request) = voice_request(&self.state.settings, name, balance_db100)? else {
            return self.advance(instruction.next, instruction.opcode).map(Some);
        };
        if !host.direct_audio(&request)? {
            self.state.audio.voice_pc = Some(instruction.offset);
            return Ok(Some(Outcome::Pending));
        }
        self.state.audio.voice_pc = None;
        self.state.audio.last_voice = name.into();
        self.advance(instruction.next, instruction.opcode).map(Some)
    }
}

///427540's native enable/mute/gain policy, shared by script and UI voice callers.
pub(crate) fn voice_request(settings: &crate::dc2_settings::SystemData, name: &str, balance_db100: i32) -> Result<Option<Request>> {
        let dword = |at: usize| -> Result<i32> {
            Ok(i32::from_le_bytes(
                settings
                    .bytes()
                    .get(at..at + 4)
                    .context("DC2 native audio preference field")?
                    .try_into()?,
            ))
        };
        //Source56 is voice-enable;64 independently gates buffer publication.
        if dword(56)? == 0 || dword(64)? == 0 || name.is_empty() {
            return Ok(None);
        }
        ensure!(!name.is_empty(), "DC2 empty native voice name after prefix");
        let system = name.split('\\').any(|component| {
            component
                .get(..3)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("sys"))
        });
        if !system {
            //427460 resolves the last underscore's3-byte character tag through its
            //native metadata table. Until that table is loaded, only the provably
            //equivalent all-enabled profile is admitted; do not guess a speaker ID.
            ensure!(
                (60..100).all(|index| settings
                    .byte(100, index)
                    .is_ok_and(|value| value == Some(0))),
                "DC2 per-character voice mute metadata is not connected"
            );
        }
        let volume = dword(60)?.wrapping_mul(2);
        let gain_db100 = if volume <= 0 {
            -10000
        } else {
            (20i32.wrapping_mul(5i32.wrapping_mul(volume).wrapping_sub(300)) / 2).clamp(-3000, 0)
        };
        Ok(Some(Request {
            name: name.into(),
            channel: 8,
            voice: true,
            gain_db100,
            balance_db100,
            looping: false,
            streamed: false,
        }))

}

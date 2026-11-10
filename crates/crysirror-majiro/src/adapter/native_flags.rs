// Copyright (c) 2026 CrySirror contributors. MIT.
//! Native input/console one-shot policy (YUKIIRO4022E0/426070/4203F0).
//! Input suppression is a press-edge filter shared by real mouse buttons and
//! keyboard aliases; it must not change physical held state or inject a click.
use super::*;

#[derive(Default)]
pub(super) struct NativeFlags {
    click_mask: i32,
    click_remaining: i32,
    last_click_ms: u32,
}
impl NativeFlags {
    fn suppress_click(&mut self, bit: i32, now: u32) -> bool {
        let elapsed = now.wrapping_sub(self.last_click_ms) as i32;
        let suppress = self.click_mask & bit != 0 && elapsed.wrapping_abs() < 350;
        self.last_click_ms = now;
        if suppress {
            self.click_remaining -= 1;
            if self.click_remaining <= 0 {
                self.click_mask = 0;
            }
        } else {
            self.click_mask = 0;
        }
        suppress
    }
}
impl MajiroVm {
    ///426110/41D250 records a synthetic middle event but clears its published
    ///press bit before returning; it is not an input event for script handlers.
    pub(super) fn synthesize_native_middle(&mut self, now: u32) {
        self.native_flags.suppress_click(4, now);
        self.input.pressed[syscalls::KEY_MIDDLE] = false;
    }
    pub(super) fn native_flags_native(
        &mut self,
        hash: u32,
        arguments: &[Value],
        returns: bool,
    ) -> Result<Option<Vec<Value>>> {
        match hash {
            //4022E0's absent argument is1.426070 deliberately preserves the
            //last input timestamp, so the350ms interval is not rearmed here.
            0xb7e3e81c => {
                ensure!(arguments.len() <= 1, "Majiro input suppression ABI");
                let mask = arguments
                    .first()
                    .map(Value::integer)
                    .transpose()?
                    .unwrap_or(1);
                self.native_flags.click_mask = mask;
                self.native_flags.click_remaining = 3;
            }
            0xdf9f7749 => {
                ensure!(arguments.is_empty(), "Majiro next-console-voice gender ABI");
                self.console.voice_male = true;
            }
            0xaccf4004 => {
                ensure!(arguments.is_empty(), "Majiro skip-next-console-voice ABI");
                self.console.skip_voice = Some(1);
            }
            0x0542e1b7 => {
                ensure!(
                    arguments.is_empty(),
                    "Majiro suppress-next-console-newline ABI"
                );
                self.console.suppress_next_newline = true;
            }
            _ => return Ok(None),
        }
        Ok(Some(if returns {
            vec![Value::Integer(0)]
        } else {
            vec![]
        }))
    }

    ///425D00/425D70: process each native left/right/middle edge once, after
    ///ordinary fast-mode cancellation and before delivering frame targets.
    /// Native input has one last-event clock across all three buttons.
    pub(super) fn filter_native_clicks(&mut self, now: u32) {
        for (bit, key) in [
            (1, syscalls::KEY_LEFT_CLICK),
            (2, syscalls::KEY_RIGHT_CLICK),
            (4, syscalls::KEY_MIDDLE),
        ] {
            if !self.input.pressed[key] {
                continue;
            }
            if self.native_flags.suppress_click(bit, now) {
                self.input.pressed[key] = false;
            }
        }
    }
}

impl MajiroVm {
    ///447B80's direct and console entry points share transport, one-shot gain,
    ///and balance. The console form alone applies VOICESKIPDEF and gender.
    pub(super) fn play_native_voice(
        &mut self,
        name: &[u8],
        repeat: i32,
        console: Option<(i32, bool)>,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        use crysirror_core::service::{AudioParameters, AudioRequest, EncodedAudio, PcmBus};
        let name = &name[..name
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(name.len())];
        if name.eq_ignore_ascii_case(b"NW") {
            return Ok(());
        }
        //4607E0 (PCM) and463750 (Ogg) decrement ((word & ~bit30)-1)
        //after each pass, then set bit30 again until the result is0. Bit30
        //is a flag, not a count bit: collapse that hole when deriving total
        //passes. Finite0 wraps the original u32 counter and takes2^31 passes;
        //it is neither zero playback nor a silently clamped single pass.
        let word = repeat as u32;
        let loop_passes = if word & 0x4000_0000 != 0 {
            let count = (word & 0x3fff_ffff) + ((word >> 31) << 30);
            Some(if count == 0 { 0x8000_0000 } else { count })
        } else {
            None
        };
        let (volume, talk_male, talk_female) = self
            .settings
            .as_ref()
            .map(|settings| {
                (
                    settings.voice_volume,
                    settings.talk_male,
                    settings.talk_female,
                )
            })
            .unwrap_or((1000, 1, 1));
        let permitted = match console {
            None => true,
            Some((skip, male)) => {
                skip == 0 && volume != 0 && (if male { talk_male } else { talk_female }) != 0
            }
        };
        let instance_gain = self.audio.instance_gain[native_audio::VOICE as usize];
        let gain = volume.wrapping_mul(instance_gain) / 1000;
        ensure!(
            (0..=4000).contains(&gain),
            "Majiro voice gain outside shared transport range"
        );
        host.submit_audio(
            self.client,
            AudioRequest::Stop {
                channel: native_audio::VOICE,
                fade_ms: 0.0,
            },
        )
        .map_err(core_error)?;
        self.api_calls += 1;
        self.audio_names.remove(&native_audio::VOICE);
        if self.read_audio_exists(name) {
            if permitted && (repeat != 0 || !self.fast_skip_active(host)?) {
                let bytes = self
                    .read_audio(host, name)?
                    .context("Majiro indexed voice resource disappeared")?;
                host.submit_audio(
                    self.client,
                    AudioRequest::Play {
                        channel: native_audio::VOICE,
                        source: EncodedAudio {
                            bytes,
                            frame_limit: None,
                        },
                        intro: None,
                        parameters: AudioParameters {
                            gain: gain as f32 / 1000.0,
                            looping: repeat != 0,
                            loop_passes,
                            bus: PcmBus::Voice,
                            restart: true,
                            ..Default::default()
                        },
                    },
                )
                .map_err(core_error)?;
                self.api_calls += 1;
                self.audio_names.insert(native_audio::VOICE, name.to_vec());
            }
        } else if !name.is_empty() {
            self.warn(format!(
                "Majiro voice {:?} is absent",
                String::from_utf8_lossy(name)
            ));
        }
        //The original clears one-shot fields after suppressed/missing names
        //too. consume_voice_pan only submits balance for an actual live voice.
        self.consume_voice_pan(host)?;
        self.audio.instance_gain[native_audio::VOICE as usize] = 1000;
        Ok(())
    }
    pub(super) fn play_console_voice(
        &mut self,
        name: &[u8],
        repeat: i32,
        skip: i32,
        male: bool,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        self.play_native_voice(name, repeat, Some((skip, male)), host)
    }
}

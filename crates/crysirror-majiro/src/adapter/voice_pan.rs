// Copyright (c) 2026 CrySirror contributors. MIT.
//! YUKIIRO447F40 / classic4386B0 raw DirectSound voice balance semantics.
//! 461720 (classic44D750) queues fields418/419 on the retained voice instance;
//! 45FF40 consumes them through45EA40, IDirectSoundBuffer::SetPan (+64).
//! This is hundredths of dB, not the engine's independent0..1 equal-power pan.
use super::*;
use crysirror_core::service::{AudioBalanceCommand, AudioRequest};

impl MajiroVm {
    pub(super) fn native_voice_pan(
        &mut self,
        arguments: &[Value],
        returns: bool,
        host: &mut dyn HostServices,
    ) -> Result<Option<Vec<Value>>> {
        ensure!(
            arguments.len() == 1,
            "Majiro voice pan requires one integer dB100 argument"
        );
        let value = arguments[0].integer()?;
        if self.audio_names.contains_key(&native_audio::VOICE) {
            // The original global can retain an ID after its task naturally
            // ended.461720 then finds no live instance and does not turn that
            // control into a setting for the following voice.
            if host
                .audio_info(self.client, native_audio::VOICE)
                .map_err(core_error)?
                .is_some()
            {
                self.submit_voice_pan(host, value)?;
            }
        } else {
            self.audio.pending_voice_pan = value;
        }
        Ok(Some(if returns {
            vec![Value::Integer(0)]
        } else {
            vec![]
        }))
    }

    /// Call after an ordinary447B80 playback attempt, including empty/missing
    /// names. The special case-insensitive "NW" control name bypasses447B80's
    /// entire body and must bypass this method too. Bank voices/BGM do not use
    /// this one-shot native global.
    pub(super) fn consume_voice_pan(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let value = self.audio.pending_voice_pan;
        if value != 0
            && self.audio_names.contains_key(&native_audio::VOICE)
            && host
                .audio_info(self.client, native_audio::VOICE)
                .map_err(core_error)?
                .is_some()
        {
            self.submit_voice_pan(host, value)?;
        }
        //447B80 clears59EA68 after successful, suppressed and missing-file
        // attempts alike. Zero leaves the newly created buffer's centre pan.
        // Shared-host failures above propagate before altering the receipt.
        self.audio.pending_voice_pan = 0;
        Ok(())
    }

    fn submit_voice_pan(&mut self, host: &mut dyn HostServices, value: i32) -> Result<()> {
        // SetPan rejects parameters outsideDSBPAN_LEFT..DSBPAN_RIGHT. The
        // native void45EA40 ignores that HRESULT; retain its unchanged balance
        // and report the rejected value rather than clamping it into a new pan.
        // Microsoft SetPan contract: learn.microsoft.com/en-us/previous-versions/
        // windows/desktop/mt708938(v=vs.85)
        if !(-10000..=10000).contains(&value) {
            self.warn(format!(
                "Majiro DirectSound voice SetPan rejected out-of-range dB100 {value}"
            ));
            return Ok(());
        }
        host.submit_audio(
            self.client,
            AudioRequest::Balance {
                channel: native_audio::VOICE,
                command: AudioBalanceCommand::Set {
                    db100: value,
                    cancel_track: true,
                },
            },
        )
        .map_err(core_error)?;
        self.api_calls += 1;
        Ok(())
    }
}

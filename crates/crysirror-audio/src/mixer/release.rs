//! Owner-only accessors for immutable-cursor voice retirement.
use super::*;
impl Mixer {
    pub(crate) fn release_start(
        &self,
        client: u32,
        h: VoiceHandle,
        freeze: bool,
    ) -> Result<(i32, u64)> {
        self.idle()?;
        ensure!(client == h.client, "foreign release voice");
        let v = self
            .voices
            .get(h.slot as usize)
            .and_then(Option::as_ref)
            .filter(|v| v.handle == h)
            .context("stale release voice")?;
        let group = if freeze {
            self.groups.current_gain(v.mix_group)
        } else {
            1.
        };
        let gain = (v.gain.value * v.envelope.value * group).clamp(0., 1.);
        let db = crate::levels::legacy_db100_from_gain(gain)?;
        Ok((db, v.position.floor() as u64))
    }

    pub(crate) fn capture_release(
        &mut self,
        client: u32,
        h: VoiceHandle,
        spec: &crysirror_core::service::AudioReleaseSpec,
    ) -> Result<(i32, u64)> {
        self.idle()?;
        let group = self
            .voices
            .get(h.slot as usize)
            .and_then(Option::as_ref)
            .context("release voice missing")?
            .mix_group;
        let group_gain = if spec.freeze_group_gain {
            self.groups.current_gain(group)
        } else {
            1.
        };
        let voice = self.voice(client, h)?;
        let gain = (voice.gain.value * voice.envelope.value * group_gain).clamp(0., 1.);
        let db100 = crate::levels::legacy_db100_from_gain(gain)?;
        if spec.freeze_group_gain {
            voice.spec.mix_group = None;
            voice.mix_group = None;
        }
        voice.spec.gain = gain;
        voice.gain = Ramp::new(gain);
        voice.envelope = Ramp::new(1.);
        if spec.source_channel!=spec.tail_channel {
            voice.spec.looping = false;
            voice.spec.paused = false;
            voice.spec.transport_active = true;
        }
        // Release handoff is explicit retirement, not a retained instance.
        voice.spec.retain_on_finish = false;
        if let Some(rate) = spec.playback_rate {
            voice.spec.rate = rate;
        }
        if let Some(pan) = spec.pan {
            voice.pan = Ramp::new((pan + 1.) * 0.5);
        }
        voice.resume_after = None;
        voice.stop_after_fade = false;
        voice.stop_remaining = 0;
        voice.pause_after_fade = false;
        Ok((db100, voice.position.floor() as u64))
    }
    pub(crate) fn release_db100(&mut self, client: u32, h: VoiceHandle, db100: i32) -> Result<()> {
        ensure!((-10000..=0).contains(&db100), "release gain bounds");
        let gain = crate::levels::gain_from_legacy_db100(db100)?;
        self.set_gain(client, h, gain)
    }
}

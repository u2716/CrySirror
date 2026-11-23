//! Owner controls and worker-published observations for output-clock gain.
use super::*;
use crysirror_core::service::{AudioGainEnvelopeInfo as Info, AudioGainEnvelopeSpec as Spec};
impl Mixer {
    pub fn set_gain_envelope(
        &mut self,
        client: u32,
        handle: VoiceHandle,
        spec: Option<Spec>,
    ) -> Result<()> {
        if let Some(spec) = spec {
            spec.validate().map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        let rate = self.config.sample_rate;
        let voice = self.voice(client, handle)?;
        let current_gain = voice.gain.value;
        let current_db100 = crate::levels::db100_from_gain(current_gain)?
            .unwrap_or(-10000)
            .max(-10000);
        voice.stepped_gain = spec.map(|spec| {
            crate::gain_envelope::Envelope::new(spec, rate, current_db100, current_gain)
        });
        voice.spec.gain_envelope = spec;
        if let Some(envelope) = voice.stepped_gain {
            voice.spec.gain = envelope.gain;
            voice.gain = Ramp::new(envelope.gain);
            voice.gain_stop_after_fade = false;
            if envelope.stop() {
                self.stop(client, handle, 0.)?;
            }
        }
        Ok(())
    }
    pub fn gain_envelope_info(&self, client: u32, handle: VoiceHandle) -> Result<Option<Info>> {
        ensure!(handle.client == client, "foreign stepped gain handle");
        if let Some(published) = &self.published {
            return published
                .voices
                .get(&handle)
                .map(|voice| voice.stepped_gain)
                .context("stepped gain voice ended or stale");
        }
        self.spec(client, handle)?;
        Ok(self.voices[handle.slot as usize]
            .as_ref()
            .unwrap()
            .stepped_gain
            .map(|envelope| envelope.info()))
    }
    pub fn has_gain_envelopes(&self) -> bool {
        if let Some(published) = &self.published {
            return published
                .voices
                .values()
                .any(|voice| voice.stepped_gain.is_some_and(|envelope| envelope.active));
        }
        self.voices.iter().flatten().any(|voice| {
            voice
                .stepped_gain
                .is_some_and(|envelope| envelope.info().active)
        })
    }
}
pub(super) fn cancel(voice: &mut Voice) {
    voice.stepped_gain = None;
    voice.spec.gain_envelope = None;
}
pub(super) fn finished(voice: &mut Voice) -> bool {
    voice.stepped_gain.is_some_and(|envelope| envelope.stop()) && !retained::gain_finished(voice)
}
impl crate::channels::ChannelAudio {
    pub fn set_gain_envelope(
        &mut self,
        client: u32,
        channel: u32,
        spec: Option<Spec>,
    ) -> Result<()> {
        if let Some(spec) = spec {
            spec.validate().map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        self.mixer.idle()?;
        if let Some(handle) = self
            .voices
            .get(&(client, channel))
            .copied()
            .filter(|handle| self.mixer.is_playing(client, *handle))
        {
            self.mixer.set_gain_envelope(client, handle, spec)?;
            self.cancel_release_channel(client, channel);
        }
        Ok(())
    }
    pub fn gain_envelope_info(&self, client: u32, channel: u32) -> Result<Option<Info>> {
        let Some(handle) = self
            .voices
            .get(&(client, channel))
            .copied()
            .filter(|handle| self.mixer.is_playing(client, *handle))
        else {
            return Ok(None);
        };
        self.mixer.gain_envelope_info(client, handle)
    }
}

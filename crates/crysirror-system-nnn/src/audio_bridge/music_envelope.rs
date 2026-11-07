//! Original46B160/46BD20 two-buffer music policy. Generic PCM clocks,
//! quantized logarithmic gain and source/channel transfer belong to Audio.
use super::*;
use anyhow::Context;
use crysirror_core::service::{AudioGainEnvelopeInfo, AudioGainEnvelopeSpec, AudioRequest};

#[derive(Clone)]
pub(super) struct MusicRuntime {
    current: usize,
    // The actual WaveMusic constructor initializes both retained fade-out
    // divisors to30. A new Play does not reset them (46BD20 case3).
    out_steps: [u32; 2],
}
impl Default for MusicRuntime {
    fn default() -> Self {
        Self {
            current: 0,
            out_steps: [30; 2],
        }
    }
}
impl MusicRuntime {
    pub(super) fn start_buffer(&mut self) {
        //446F40 selects the other WaveMusic object for every admitted play,
        //including53B630 original-slot replay. Its retained out divisor is
        //not reconstructed from the source's new fade-in duration.
        self.current ^= 1;
    }
}
fn phase(
    host: &dyn HostServices,
    client: ClientId,
    channel: u32,
    envelope: Option<AudioGainEnvelopeInfo>,
) -> Result<u32> {
    if let Some(envelope) = envelope {
        return Ok(envelope.phase_ms);
    }
    let Some(info) = host.audio_info(client, channel)? else {
        return Ok(0);
    };
    ensure!(info.output_rate > 0, "NNN music output rate");
    Ok(((u128::from(info.age_frames) * 1000 / u128::from(info.output_rate)) % 100) as u32)
}
impl AudioBridge {
    pub(super) fn tail_channel(&self) -> Result<u32> {
        let b = self.policy.bindings;
        (0..=65535u32)
            .rev()
            .find(|channel| {
                *channel != b.music
                    && !(*channel >= b.sound_base && *channel < b.sound_base + 8)
                    && !(*channel >= b.voice_base && *channel < b.voice_base + 8)
                    && !self.system_sounds.as_ref().is_some_and(|policy| {
                        (*channel >= policy.base) && (*channel <= policy.base + 1)
                    })
            })
            .context("NNN native music tail channel unavailable")
    }
    fn stop_envelope(
        &self,
        host: &dyn HostServices,
        channel: u32,
        fade: crate::native_audio::MusicFade,
    ) -> Result<AudioGainEnvelopeSpec> {
        let current = host.audio_gain_envelope_info(self.client, channel)?;
        let initial = if let Some(current) = current.filter(|current| current.active) {
            if current.spec.stop_at_end {
                current.elapsed_steps
            } else {
                //46BD20 case12 uses the retained old fade-out divisor before
                //assigning the newly requested duration. Keep that ordering.
                let remaining = current.spec.steps.saturating_sub(current.elapsed_steps);
                let product = u64::from(remaining)
                    * u64::from(self.music_runtime.out_steps[self.music_runtime.current]);
                ensure!(
                    product <= i32::MAX as u64,
                    "NNN music fade counter multiplication limit"
                );
                (product / u64::from(current.spec.steps.max(1))) as u32
            }
        } else {
            0
        };
        let spec = AudioGainEnvelopeSpec {
            interval_ms: 100,
            steps: fade.ticks_100ms,
            start_db100: fade.target_percent * 100 - 10000,
            end_db100: -10000,
            elapsed_steps: initial.min(fade.ticks_100ms),
            phase_ms: phase(host, self.client, channel, current)?,
            apply_initial: false,
            stop_at_end: true,
        };
        spec.validate()?;
        Ok(spec)
    }
    pub(super) fn control_plan(
        &mut self,
        host: &mut dyn HostServices,
        plan: &NativeAudioPlan,
    ) -> Result<()> {
        match plan.operation {
            AudioOperation::Stop {
                channel,
                fade: Some(fade),
            } => {
                ensure!(
                    channel == self.policy.bindings.music,
                    "NNN fade-stop music channel"
                );
                let spec = self.stop_envelope(host, channel, fade)?;
                host.set_audio_gain_envelope(self.client, channel, Some(spec))?;
                self.music_runtime.out_steps[self.music_runtime.current] = fade.ticks_100ms;
                self.api_calls += 1;
            }
            AudioOperation::Gain { channel, percent } if channel == self.policy.bindings.music => {
                if let Some(info) = host
                    .audio_gain_envelope_info(self.client, channel)?
                    .filter(|info| info.active)
                {
                    let mut spec = info.spec;
                    spec.elapsed_steps = info.elapsed_steps;
                    spec.phase_ms = info.phase_ms;
                    spec.apply_initial = true;
                    if spec.stop_at_end {
                        spec.start_db100 = percent * 100 - 10000;
                    } else {
                        spec.end_db100 = percent * 100 - 10000;
                    }
                    host.set_audio_gain_envelope(self.client, channel, Some(spec))?;
                    self.api_calls += 1;
                } else if let Some(request) = plan.control_request()? {
                    host.submit_audio(self.client, request)?;
                    self.api_calls += 1;
                }
            }
            _ => {
                if let Some(request) = plan.control_request()? {
                    host.submit_audio(self.client, request)?;
                    self.api_calls += 1;
                }
            }
        }
        Ok(())
    }
    pub(super) fn source_request(
        &mut self,
        host: &mut dyn HostServices,
        plan: &NativeAudioPlan,
        source: crate::resource_bridge::ResolvedAudio,
    ) -> Result<AudioRequest> {
        let mut immediate = plan.clone();
        immediate.prepare_music_start(self.policy.bindings.music)?;
        if let AudioOperation::Play {
            channel, fade_out, ..
        } = &plan.operation
        {
            if *channel == self.policy.bindings.music {
                let tail = self.tail_channel()?;
                let old = host.audio_playing(self.client, *channel)?;
                let old_fade = if old {
                    (*fade_out)
                        .map(|fade| self.stop_envelope(host, *channel, fade))
                        .transpose()?
                } else {
                    None
                };
                host.submit_audio(
                    self.client,
                    AudioRequest::Stop {
                        channel: tail,
                        fade_ms: 0.,
                    },
                )?;
                self.api_calls += 1;
                if old {
                    host.submit_audio(
                        self.client,
                        AudioRequest::ExchangeChannels {
                            left: *channel,
                            right: tail,
                        },
                    )?;
                    self.api_calls += 1;
                    if let Some(spec) = old_fade {
                        host.set_audio_gain_envelope(self.client, tail, Some(spec))?;
                        self.music_runtime.out_steps[self.music_runtime.current] = spec.steps;
                    } else {
                        host.submit_audio(
                            self.client,
                            AudioRequest::Stop {
                                channel: tail,
                                fade_ms: 0.,
                            },
                        )?;
                    }
                    self.api_calls += 1;
                }
                self.music_runtime.start_buffer();
            }
        }
        match source {
            crate::resource_bridge::ResolvedAudio::Asset(path) => immediate.asset_request(path),
            crate::resource_bridge::ResolvedAudio::Encoded(source) => {
                immediate.encoded_request(source)
            }
        }
    }
}

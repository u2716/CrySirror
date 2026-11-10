// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native two-channel system SE/voice ownership. Resource lookup and audio
//! preparation are cancellable services; each request retains its exact
//! identity until decoder completion and never changes the script voice bank.
use super::*;
use crysirror_core::service::{AudioParameters, AudioRequest, PcmBus};
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Request {
    Sound(i32),
    Voice(Vec<u8>),
}
pub(super) enum SystemPending {
    Resolve {
        request: Request,
        channel: u32,
        gain: f32,
        bus: PcmBus,
        task: Task<crate::resource_bridge::ResolvedAudio>,
    },
    Prepare {
        request: Request,
        ticket: u64,
    },
}
impl AudioBridge {
    pub(crate) fn poll_system_voice(
        &mut self,
        host: &mut dyn HostServices,
        name: &[u8],
    ) -> Result<bool> {
        self.poll_system_audio(host, Request::Voice(name.to_vec()))
    }
    pub(super) fn poll_system_audio(
        &mut self,
        host: &mut dyn HostServices,
        request: Request,
    ) -> Result<bool> {
        if self.pending.is_some() || self.outer_music.is_some() {
            return Ok(false);
        }
        if self.system_pending.is_none() {
            let policy = self
                .system_sounds
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("NNN system audio channels not configured"))?;
            let (enabled, multi_stop, gain, bus) = match request {
                Request::Sound(_) => (
                    self.policy.settings.sound_enabled,
                    !policy.multi,
                    crate::native_audio::legacy_gain(self.policy.settings.sound_volume)?,
                    PcmBus::Sfx,
                ),
                // PlaySystemVoiceByFileName does not consult systemSoundMulti
                // or stop the previously selected channel; only the next one.
                // Its firstOffFlag parameter is unused in the complete body.
                Request::Voice(_) => (
                    self.policy.settings.voice_enabled,
                    false,
                    crate::native_audio::legacy_gain(self.policy.settings.voice_volume)?,
                    PcmBus::Voice,
                ),
            };
            if !enabled {
                return Ok(true);
            }
            if multi_stop {
                host.submit_audio(
                    self.client,
                    AudioRequest::Stop {
                        channel: policy.base + policy.current,
                        fade_ms: 0.,
                    },
                )?;
                self.api_calls += 1;
            }
            policy.current = (policy.current + 1) % 2;
            let channel = policy.base + policy.current;
            host.submit_audio(
                self.client,
                AudioRequest::Stop {
                    channel,
                    fade_ms: 0.,
                },
            )?;
            self.api_calls += 1;
            let name = match &request {
                Request::Sound(number) => {
                    // Original tests n<=-1 only after rotating/stopping its
                    // two channels. The public UI request is one-based.
                    if *number <= 0 {
                        return Ok(true);
                    }
                    // noSystemSeFlag omits the catalogue, rather than
                    // bypassing PlaySystemSound's channel-stop prefix.
                    if policy.disabled {
                        return Ok(true);
                    }
                    let Some(names) = policy.names.as_ref() else {
                        return Ok(true);
                    };
                    let index = usize::try_from(number - 1)?
                        .checked_mul(2)
                        .ok_or_else(|| anyhow::anyhow!("NNN system sound table index overflow"))?;
                    names
                        .tokens
                        .get(index)
                        .ok_or_else(|| {
                            anyhow::anyhow!("NNN original system sound number {number} absent")
                        })?
                        .clone()
                }
                Request::Voice(name) => name.clone(),
            };
            let source = NativeSource {
                kind: crate::native_audio::SourceKind::SystemSound,
                name,
                candidates: Vec::new(),
            };
            let resolver = self.resolver.clone();
            let task = self.pool.spawn(move |cancel| resolver(&source, &cancel))?;
            self.system_pending = Some(SystemPending::Resolve {
                request,
                channel,
                gain,
                bus,
                task,
            });
            return Ok(false);
        }
        match self.system_pending.take().unwrap() {
            SystemPending::Resolve {
                request: expected,
                channel,
                gain,
                bus,
                mut task,
            } => {
                ensure!(
                    expected == request,
                    "NNN pending system audio request changed"
                );
                let Some(source) = task.try_take()? else {
                    self.system_pending = Some(SystemPending::Resolve {
                        request,
                        channel,
                        gain,
                        bus,
                        task,
                    });
                    return Ok(false);
                };
                let parameters = AudioParameters {
                    gain,
                    bus,
                    ..Default::default()
                };
                let play = match source {
                    crate::resource_bridge::ResolvedAudio::Playlist(_) => {
                        bail!("NNN system audio cannot resolve a music playlist")
                    }
                    crate::resource_bridge::ResolvedAudio::Asset(path) => AudioRequest::PlayAsset {
                        channel,
                        source: crysirror_core::service::AudioAsset {
                            path,
                            frame_limit: None,
                        },
                        intro: None,
                        parameters,
                    },
                    crate::resource_bridge::ResolvedAudio::Encoded(source) => AudioRequest::Play {
                        channel,
                        source,
                        intro: None,
                        parameters,
                    },
                };
                let ticket = host.begin_audio(self.client, play)?;
                self.api_calls += 1;
                self.system_pending = Some(SystemPending::Prepare { request, ticket });
                Ok(false)
            }
            SystemPending::Prepare {
                request: expected,
                ticket,
            } => {
                ensure!(
                    expected == request,
                    "NNN preparing system audio request changed"
                );
                let state = host.audio_preparation(self.client, ticket)?;
                match state.phase {
                    AudioPreparePhase::Pending => {
                        self.system_pending = Some(SystemPending::Prepare { request, ticket });
                        Ok(false)
                    }
                    AudioPreparePhase::Failed => {
                        host.release_audio(self.client, ticket)?;
                        bail!(
                            "NNN shared system audio preparation failed: {}",
                            state.error.unwrap_or_else(|| "no diagnostic".into())
                        )
                    }
                    AudioPreparePhase::Ready => {
                        host.release_audio(self.client, ticket)?;
                        Ok(true)
                    }
                }
            }
        }
    }
}

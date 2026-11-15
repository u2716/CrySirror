// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native two-channel system SE/voice ownership. Resource lookup and audio
//! preparation are cancellable services; each request retains its exact
//! identity until decoder completion and never changes the script voice bank.
use super::*;
use crysirror_core::service::{AudioParameters, AudioRequest, PcmBus};
/// CGameCallBack's constructor creates and loads this array before General
/// startup. Run on its resource worker; no output device/channel is allocated.
pub(crate) fn prepare_preloaded_waves(
    resources: &crate::resource_bridge::NativeResources,
    names: &crate::native_resources::ResourceNames,
    table: Option<&Names>,
    number: i32,
    cancel: &Cancellation,
) -> Result<Vec<Option<crate::resource_bridge::ResolvedAudio>>> {
    let Some(table) = table else {
        return Ok(Vec::new());
    };
    ensure!(number <= 256, "NNN preloaded system wave allocation budget");
    let mut waves = Vec::new();
    let mut bytes = 0usize;
    for i in 0..number.max(0) as usize {
        cancel.check()?;
        let name = table.tokens.get(i * 2).ok_or_else(|| {
            anyhow::anyhow!("NNN preloaded system wave {} has no catalogue entry", i + 1)
        })?;
        let wave = resources.optional_system_wave(names, name, cancel)?;
        // Native fresh CWaveData leaves format fields uninitialized until a
        // successful load. Define a missing initial wave as no-play instead of
        // submitting undefined channel/rate pointers to the shared Audio API.
        if let Some(crate::resource_bridge::ResolvedAudio::Encoded(audio)) = &wave {
            bytes = bytes
                .checked_add(audio.bytes.len())
                .ok_or_else(|| anyhow::anyhow!("NNN preloaded system wave extent overflow"))?;
        }
        ensure!(bytes <= 64 << 20, "NNN preloaded system wave byte budget");
        waves.push(wave);
    }
    cancel.check()?;
    Ok(waves)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SoundInputs {
    pub total_off: bool,
    pub voice_enabled: bool,
    pub sound_enabled: bool,
    pub voice_volume: i32,
    pub sound_volume: i32,
    pub volume_type: i32,
    pub name_delta: i32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Request {
    Sound(i32, SoundInputs),
    FilenameSound(Vec<u8>, i32),
    Voice(Vec<u8>),
}
pub(super) enum SystemPending {
    Resolve {
        request: Request,
        channel: u32,
        gain: f32,
        bus: PcmBus,
        task: Task<Option<crate::resource_bridge::ResolvedAudio>>,
    },
    Prepare {
        request: Request,
        ticket: u64,
    },
}
impl AudioBridge {
    pub(crate) fn configure_preloaded_system_waves(
        &mut self,
        waves: Vec<Option<crate::resource_bridge::ResolvedAudio>>,
    ) -> Result<()> {
        ensure!(
            !self.pending(),
            "NNN system preload configuration during an audio operation"
        );
        let policy = self
            .system_sounds
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("NNN system preload channels absent"))?;
        ensure!(
            policy.preloaded.is_empty() && waves.len() <= 256,
            "NNN system preload configuration bounds"
        );
        policy.preloaded = waves;
        Ok(())
    }
    fn prepare_system_play(
        &mut self,
        host: &mut dyn HostServices,
        request: Request,
        channel: u32,
        gain: f32,
        bus: PcmBus,
        source: crate::resource_bridge::ResolvedAudio,
    ) -> Result<bool> {
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
    /// The optional character-volume adjustment deliberately uses systemse's
    /// name, even if a newer native profile selects a systemvoice catalogue.
    pub(crate) fn system_sound_filename(&self, number: i32) -> Result<Option<Vec<u8>>> {
        if number <= 0 {
            return Ok(None);
        }
        let policy = self
            .system_sounds
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("NNN system sound channels absent"))?;
        let Some(names) = &policy.names else {
            return Ok(None);
        };
        let at = usize::try_from(number - 1)?
            .checked_mul(2)
            .ok_or_else(|| anyhow::anyhow!("NNN system sound index overflow"))?;
        Ok(names.tokens.get(at).cloned())
    }
    pub(crate) fn poll_system_sound_typed(
        &mut self,
        host: &mut dyn HostServices,
        number: i32,
        inputs: SoundInputs,
    ) -> Result<bool> {
        self.poll_system_audio(host, Request::Sound(number, inputs))
    }
    pub(crate) fn poll_system_voice(
        &mut self,
        host: &mut dyn HostServices,
        name: &[u8],
    ) -> Result<bool> {
        self.poll_system_audio(host, Request::Voice(name.to_vec()))
    }
    pub(crate) fn poll_system_wave_filename(
        &mut self,
        host: &mut dyn HostServices,
        name: &[u8],
        delta: i32,
    ) -> Result<bool> {
        self.poll_system_audio(host, Request::FilenameSound(name.to_vec(), delta))
    }
    pub(super) fn poll_system_audio(
        &mut self,
        host: &mut dyn HostServices,
        request: Request,
    ) -> Result<bool> {
        if self.pending.is_some() || self.outer_music.is_some() || self.button_voice_pending() {
            return Ok(false);
        }
        if self.system_pending.is_none() {
            let policy = self
                .system_sounds
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("NNN system audio channels not configured"))?;
            let (enabled, multi_stop, gain, bus) = match request {
                Request::Sound(_, inputs) => (
                    !inputs.total_off
                        && if inputs.volume_type == 0 {
                            inputs.sound_enabled
                        } else {
                            inputs.voice_enabled
                        },
                    !policy.multi,
                    crate::native_audio::legacy_gain(if inputs.volume_type == 0 {
                        inputs.sound_volume.clamp(0, 100)
                    } else {
                        inputs
                            .voice_volume
                            .wrapping_add(inputs.name_delta)
                            .clamp(0, 100)
                    })?,
                    if inputs.volume_type == 0 {
                        PcmBus::Sfx
                    } else {
                        PcmBus::Voice
                    },
                ),
                Request::FilenameSound(_, delta) => (
                    self.policy.settings.sound_enabled,
                    false,
                    crate::native_audio::legacy_gain(
                        self.policy
                            .settings
                            .sound_volume
                            .wrapping_add(delta)
                            .clamp(0, 100),
                    )?,
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
                Request::Sound(number, _) => {
                    // Original tests n<=-1 only after rotating/stopping its
                    // two channels. The public UI request is one-based.
                    if *number <= 0 {
                        return Ok(true);
                    }
                    if let Some(wave) = policy.preloaded.get((*number - 1) as usize) {
                        let wave = wave.clone();
                        return match wave {
                            Some(source) => {
                                self.prepare_system_play(host, request, channel, gain, bus, source)
                            }
                            None => Ok(true),
                        };
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
                Request::Voice(name) | Request::FilenameSound(name, _) => name.clone(),
            };
            let source = NativeSource {
                kind: crate::native_audio::SourceKind::SystemSound,
                name,
                candidates: Vec::new(),
            };
            let resolver = self.resolver.clone();
            let optional = self.system_wave_resolver.clone();
            let task = self.pool.spawn(move |cancel| {
                cancel.check()?;
                // All installed sys waves use CMyFile's optional read: a
                // missing wave returns FALSE after the stop/rotation prefix.
                let value = if let Some(optional) = optional {
                    optional(&source.name, &cancel)?
                } else {
                    Some(resolver(&source, &cancel)?)
                };
                cancel.check()?;
                Ok(value)
            })?;
            self.system_pending = Some(SystemPending::Resolve {
                request,
                channel,
                gain,
                bus,
                task,
            });
            return Ok(false);
        }
        let expected = match self.system_pending.as_ref().unwrap() {
            SystemPending::Resolve { request, .. } | SystemPending::Prepare { request, .. } => {
                request
            }
        };
        ensure!(
            *expected == request,
            "NNN pending system audio request changed"
        );
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
                let Some(source) = source else {
                    return Ok(true);
                };
                self.prepare_system_play(host, request, channel, gain, bus, source)
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

// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CGameCallBack button voices: a distinct two-channel owner, not system SE or
//! script voice channels. Resource resolution/decoding use retained workers.
use crate::{config::Names, native_audio::legacy_gain, resource_bridge::ResolvedAudio};
use anyhow::{Result, ensure};
use crysirror_core::{
    service::{AudioParameters, AudioPreparePhase, AudioRequest, HostServices, PcmBus},
    state::ClientId,
};
use crysirror_tasks::{Cancellation, Task, TaskPool};
use std::sync::Arc;

pub(crate) type Resolver =
    Arc<dyn Fn(&[u8], &Cancellation) -> Result<Option<ResolvedAudio>> + Send + Sync>;
#[derive(Clone, Debug)]
pub(crate) struct Catalogues {
    /// Includes table0 even when ordinary system SE is disabled. Native startup
    /// loads nya/buttonVoice0.xtx..buttonVoice{useSystemVoice}.xtx independently.
    pub names: Vec<Arc<Names>>,
}
impl Catalogues {
    pub(crate) fn filename(&self, use_system_voice: i32, number: i32) -> Result<Option<Vec<u8>>> {
        if number <= 0 {
            return Ok(None);
        }
        let index = usize::try_from(use_system_voice)
            .map_err(|_| anyhow::anyhow!("negative button voice catalogue"))?;
        let table = self
            .names
            .get(index)
            .ok_or_else(|| anyhow::anyhow!("button voice catalogue {index} absent"))?;
        let at = usize::try_from(number - 1)?
            .checked_mul(2)
            .ok_or_else(|| anyhow::anyhow!("button voice table index overflow"))?;
        // Original PlayButtonVoice checks this bound before calling GetName.
        // No description token is read, so an odd final table item is valid.
        Ok(table.tokens.get(at).cloned())
    }
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Inputs {
    pub total_off: bool,
    pub voice_enabled: bool,
    pub character_allowed: bool,
    pub skip_switch_check: bool,
    pub skip_character_check: bool,
    pub voice_volume: i32,
    pub name_delta: i32,
}
impl Inputs {
    fn enabled(self) -> bool {
        !self.total_off
            && (self.skip_switch_check || self.voice_enabled)
            && (self.skip_character_check || self.character_allowed)
    }
    fn gain(self) -> Result<f32> {
        legacy_gain(
            self.voice_volume
                .wrapping_add(self.name_delta)
                .clamp(0, 100),
        )
    }
}
enum Pending {
    Resolve {
        filename: Vec<u8>,
        channel: u32,
        gain: f32,
        task: Task<Option<ResolvedAudio>>,
    },
    Prepare {
        filename: Vec<u8>,
        ticket: u64,
    },
}
pub(crate) struct ButtonVoiceOwner {
    client: ClientId,
    pub base: u32,
    /// Source rotates only for exactly2. Other configured values reuse the
    /// current channel, then apply modulo2; they are not a concurrency count.
    number: i32,
    current: u32,
    filenames: [Vec<u8>; 2],
    pool: TaskPool,
    resolver: Resolver,
    pending: Option<Pending>,
    failed: Option<String>,
    pub api_calls: u64,
}
/// Callback-object metadata survives an original slot load. Native save bytes
/// never contain these device objects or their pending decode requests.
#[derive(Clone)]
pub(crate) struct RestoreSeed {
    client: ClientId,
    base: u32,
    number: i32,
    current: u32,
    filenames: [Vec<u8>; 2],
    pool: TaskPool,
    resolver: Resolver,
}
impl RestoreSeed {
    pub(crate) fn finish(self) -> ButtonVoiceOwner {
        ButtonVoiceOwner {
            client: self.client,
            base: self.base,
            number: self.number,
            current: self.current,
            filenames: self.filenames,
            pool: self.pool,
            resolver: self.resolver,
            pending: None,
            failed: None,
            api_calls: 0,
        }
    }
}
impl ButtonVoiceOwner {
    pub(crate) fn restore_seed(&self) -> Result<RestoreSeed> {
        ensure!(
            self.idle() && self.failed.is_none(),
            "NNN button voice restore requires drained dependencies"
        );
        Ok(RestoreSeed {
            client: self.client,
            base: self.base,
            number: self.number,
            current: self.current,
            filenames: self.filenames.clone(),
            pool: self.pool.clone(),
            resolver: self.resolver.clone(),
        })
    }
    pub(crate) fn new(
        client: ClientId,
        base: u32,
        number: i32,
        pool: TaskPool,
        resolver: Resolver,
    ) -> Result<Self> {
        ensure!(
            client.0 != 0 && base > 0 && base.checked_add(1).is_some(),
            "NNN button voice owner/channels"
        );
        Ok(Self {
            client,
            base,
            number,
            current: 0,
            filenames: [Vec::new(), Vec::new()],
            pool,
            resolver,
            pending: None,
            failed: None,
            api_calls: 0,
        })
    }
    pub(crate) fn idle(&self) -> bool {
        self.pending.is_none()
    }
    pub(crate) fn poll(
        &mut self,
        host: &mut dyn HostServices,
        filename: &[u8],
        inputs: Inputs,
    ) -> Result<bool> {
        if let Some(error) = &self.failed {
            anyhow::bail!("NNN button voice: {error}");
        }
        let result = self.poll_inner(host, filename, inputs);
        if let Err(error) = &result {
            self.failed = Some(format!("{error:#}"));
        }
        result
    }
    fn poll_inner(
        &mut self,
        host: &mut dyn HostServices,
        filename: &[u8],
        inputs: Inputs,
    ) -> Result<bool> {
        if self.pending.is_none() {
            // Gating precedes duplicate detection, rotation, stop and file I/O.
            if !inputs.enabled() {
                return Ok(true);
            }
            ensure!(
                !filename.is_empty() && filename.len() <= 243 && !filename.contains(&0),
                "NNN button voice name exceeds original vaw/sys filename buffer"
            );
            for i in 0..2 {
                if self.filenames[i] == filename
                    && host.audio_playing(self.client, self.base + i as u32)?
                {
                    return Ok(true);
                }
            }
            let gain = inputs.gain()?;
            if self.number == 2 {
                self.current = self.current.wrapping_add(1);
            }
            self.current %= 2;
            self.filenames[self.current as usize] = filename.to_vec();
            let channel = self.base + self.current;
            host.submit_audio(
                self.client,
                AudioRequest::Stop {
                    channel,
                    fade_ms: 0.,
                },
            )?;
            self.api_calls += 1;
            let name = filename.to_vec();
            let resolver = self.resolver.clone();
            let work_name = name.clone();
            let task = self.pool.spawn(move |cancel| {
                cancel.check()?;
                let value = resolver(&work_name, &cancel)?;
                cancel.check()?;
                Ok(value)
            })?;
            self.pending = Some(Pending::Resolve {
                filename: name,
                channel,
                gain,
                task,
            });
            return Ok(false);
        }
        let expected = match self.pending.as_ref().unwrap() {
            Pending::Resolve { filename, .. } | Pending::Prepare { filename, .. } => filename,
        };
        ensure!(expected == filename, "NNN pending button voice changed");
        match self.pending.take().unwrap() {
            Pending::Resolve {
                filename: expected,
                channel,
                gain,
                mut task,
            } => {
                ensure!(expected == filename, "NNN pending button voice changed");
                let Some(source) = task.try_take()? else {
                    self.pending = Some(Pending::Resolve {
                        filename: expected,
                        channel,
                        gain,
                        task,
                    });
                    return Ok(false);
                };
                // Missing original optional system wave is a completed no-play
                // result AFTER the selected channel's filename/Stop prefix.
                let Some(source) = source else {
                    return Ok(true);
                };
                let parameters = AudioParameters {
                    gain,
                    bus: PcmBus::Voice,
                    ..Default::default()
                };
                let request = match source {
                    ResolvedAudio::Encoded(source) => AudioRequest::Play {
                        channel,
                        source,
                        intro: None,
                        parameters,
                    },
                    ResolvedAudio::Asset(path) => AudioRequest::PlayAsset {
                        channel,
                        source: crysirror_core::service::AudioAsset {
                            path,
                            frame_limit: None,
                        },
                        intro: None,
                        parameters,
                    },
                    ResolvedAudio::Playlist(_) => {
                        anyhow::bail!("NNN button voice resolved a music playlist")
                    }
                };
                let ticket = host.begin_audio(self.client, request)?;
                self.api_calls += 1;
                self.pending = Some(Pending::Prepare {
                    filename: expected,
                    ticket,
                });
                Ok(false)
            }
            Pending::Prepare {
                filename: expected,
                ticket,
            } => {
                ensure!(expected == filename, "NNN preparing button voice changed");
                let state = host.audio_preparation(self.client, ticket)?;
                match state.phase {
                    AudioPreparePhase::Pending => {
                        self.pending = Some(Pending::Prepare {
                            filename: expected,
                            ticket,
                        });
                        Ok(false)
                    }
                    AudioPreparePhase::Ready => {
                        host.release_audio(self.client, ticket)?;
                        Ok(true)
                    }
                    AudioPreparePhase::Failed => {
                        host.release_audio(self.client, ticket)?;
                        anyhow::bail!(
                            "NNN shared button voice preparation failed: {}",
                            state.error.unwrap_or_else(|| "no diagnostic".into())
                        )
                    }
                }
            }
        }
    }
    pub(crate) fn shutdown(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let mut failure = None;
        if let Some(Pending::Prepare { ticket, .. }) = self.pending.take() {
            if let Err(error) = host.release_audio(self.client, ticket) {
                failure = Some(anyhow::Error::from(error));
            }
        }
        // Dropping a Resolve task cancels the resource worker. Channels remain
        // independently owned and never stop script/system voices.
        for channel in self.base..=self.base + 1 {
            if let Err(error) = host.submit_audio(
                self.client,
                AudioRequest::Stop {
                    channel,
                    fade_ms: 0.,
                },
            ) {
                failure.get_or_insert_with(|| anyhow::Error::from(error));
            }
            self.api_calls += 1;
        }
        failure.map_or(Ok(()), Err)
    }
}

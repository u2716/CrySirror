// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Scoped native audio dependency routing. Native lookup policy runs on an
//! admitted worker; engine workers own decoding/mixing and the owner publishes.
use crate::{
    config::Names,
    machine::{Machine, NativeCall},
    native_audio::{AudioOperation, NativeAudioPlan, NativeAudioPolicy, NativeSource},
};
use anyhow::{Result, bail, ensure};
use crysirror_core::{
    service::{AudioPreparePhase, HostServices},
    state::ClientId,
};
use crysirror_tasks::{Cancellation, Task, TaskPool};
use std::sync::Arc;
mod music_envelope;
pub(crate) mod system_audio;
use system_audio::SystemPending as SystemSoundPending;

/// Resolves reviewed native precedence using scoped engine archive/VFS APIs.
/// Never return a playlist/container as if it were decoded audio. Missing and
/// corrupt sources remain distinct; cancellation belongs to the current task.
pub type AudioResolver = Arc<
    dyn Fn(&NativeSource, &Cancellation) -> Result<crate::resource_bridge::ResolvedAudio>
        + Send
        + Sync,
>;
enum Pending {
    Resolve {
        call: NativeCall,
        plan: NativeAudioPlan,
        task: Task<crate::resource_bridge::ResolvedAudio>,
    },
    Prepare {
        call: NativeCall,
        plan: NativeAudioPlan,
        ticket: u64,
    },
    VoiceResource {
        call: NativeCall,
        plan: NativeAudioPlan,
        ticket: u64,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum OuterMusicRequest {
    Script { words: [i32; 4], restart: bool },
    Listen(Option<crate::native_audio::ListenMusic>),
    ReplayVoice { name: Vec<u8>, percent: i32 },
}
enum OuterMusicPending {
    Resolve {
        request: OuterMusicRequest,
        plan: NativeAudioPlan,
        task: Task<Option<crate::resource_bridge::ResolvedAudio>>,
    },
    Prepare {
        request: OuterMusicRequest,
        plan: NativeAudioPlan,
        ticket: u64,
    },
}
#[derive(Clone)]
struct SystemSoundPolicy {
    names: Option<Arc<Names>>,
    disabled: bool,
    multi: bool,
    base: u32,
    current: u32,
    /// Original CWaveData array, loaded at callback construction rather than
    /// re-reading the first systemSoundLoadNumber files on every button click.
    preloaded: Vec<Option<crate::resource_bridge::ResolvedAudio>>,
}
pub struct AudioBridge {
    pub policy: NativeAudioPolicy,
    pub api_calls: u64,
    client: ClientId,
    music: Arc<Names>,
    sound: Arc<Names>,
    resolver: AudioResolver,
    pool: TaskPool,
    pending: Option<Pending>,
    outer_music: Option<OuterMusicPending>,
    system_sounds: Option<SystemSoundPolicy>,
    system_pending: Option<SystemSoundPending>,
    system_wave_resolver: Option<crate::native_button_voice::Resolver>,
    replay_voice_resolver: Option<crate::native_button_voice::Resolver>,
    button_catalogues: Option<crate::native_button_voice::Catalogues>,
    button_voices: Option<crate::native_button_voice::ButtonVoiceOwner>,
    restored_work: Option<crate::native_save_audio::SavedAudioWork>,
    voice_lengths_ms: [Option<u32>; 8],
    music_runtime: music_envelope::MusicRuntime,
    pending_music_read: Option<i32>,
    music_read_mark: Option<i32>,
    message_voice_start: Option<(usize, u32)>,
}
/// Immutable native metadata/capability seed. Large name tables and workers
/// are Arc/shared task capabilities rather than owner-loop deep copies.
pub(crate) struct AudioRestoreSeed {
    music_runtime: music_envelope::MusicRuntime,
    client: ClientId,
    policy: NativeAudioPolicy,
    music: Arc<Names>,
    sound: Arc<Names>,
    resolver: AudioResolver,
    pool: TaskPool,
    system_sounds: Option<SystemSoundPolicy>,
    system_wave_resolver: Option<crate::native_button_voice::Resolver>,
    replay_voice_resolver: Option<crate::native_button_voice::Resolver>,
    button_catalogues: Option<crate::native_button_voice::Catalogues>,
    button_voices: Option<crate::native_button_voice::RestoreSeed>,
}
impl AudioRestoreSeed {
    pub(crate) fn prepare(
        &self,
        file: &crate::native_save::NativeSaveFile,
        profile: &crate::native_save::LegacySaveProfile,
    ) -> Result<crate::native_save_audio::SavedAudioPreparation> {
        crate::native_save_audio::SavedAudioPreparation::begin(
            self.client,
            file,
            profile,
            &self.policy,
            &self.music,
            &self.sound,
            self.resolver.clone(),
            &self.pool,
        )
    }
    /// Constructs only a detached native adapter component. The opaque audio
    /// resource must still go into OriginalRestoreCommit.prepared_audio; no
    /// live channel operations or original VM acknowledgements occur here.
    pub(crate) fn finish(
        self,
        prepared: crate::native_save_audio::PreparedSavedAudio,
    ) -> Result<(
        AudioBridge,
        Arc<dyn crysirror_core::service::AudioReplacementResource>,
    )> {
        ensure!(
            prepared.replacement.client() == self.client
                && prepared.replacement.info().client == self.client,
            "NNN saved audio component owner"
        );
        let a = self.policy.bindings;
        let b = prepared.policy.bindings;
        ensure!(
            a.music == b.music && a.sound_base == b.sound_base && a.voice_base == b.voice_base,
            "NNN saved audio component binding profile"
        );
        let replacement = prepared.replacement;
        let mut music_runtime = self.music_runtime;
        if prepared.original.music_number != -1 && prepared.policy.settings.music_enabled {
            music_runtime.start_buffer();
        }
        Ok((
            AudioBridge {
                policy: prepared.policy,
                api_calls: 0,
                client: self.client,
                music: self.music,
                sound: self.sound,
                resolver: self.resolver,
                pool: self.pool,
                pending: None,
                outer_music: None,
                system_sounds: self.system_sounds,
                system_pending: None,
                system_wave_resolver: self.system_wave_resolver,
                replay_voice_resolver: self.replay_voice_resolver,
                button_catalogues: self.button_catalogues,
                button_voices: self.button_voices.map(|seed| seed.finish()),
                restored_work: Some(prepared.original),
                voice_lengths_ms: [None; 8],
                music_runtime,
                pending_music_read: None,
                music_read_mark: None,
                message_voice_start: None,
            },
            replacement,
        ))
    }
}
impl AudioBridge {
    pub(crate) fn apply_settings(
        &mut self,
        settings: crate::native_audio::AudioSettings,
        host: &mut dyn HostServices,
    ) -> Result<Option<[i32; 4]>> {
        use crysirror_core::service::AudioRequest;
        settings.validate()?;
        ensure!(
            !self.pending(),
            "NNN audio settings during an admitted audio operation"
        );
        let old = self.policy.settings;
        let mut next = self.policy.clone();
        next.settings = settings;
        if old.music_enabled && !settings.music_enabled {
            host.submit_audio(
                self.client,
                AudioRequest::Stop {
                    channel: next.bindings.music,
                    fade_ms: 0.,
                },
            )?;
            self.api_calls += 1;
            next.music_number = None;
        }
        if old.music_volume != settings.music_volume {
            next.music_percent = settings.music_volume;
            if host.audio_playing(self.client, next.bindings.music)? {
                self.control_plan(
                    host,
                    &NativeAudioPlan {
                        operation: AudioOperation::Gain {
                            channel: next.bindings.music,
                            percent: settings.music_volume,
                        },
                        source: None,
                        next: next.clone(),
                    },
                )?;
            }
        }
        let restart = if !old.music_enabled && settings.music_enabled {
            next.last_music_number
                .map(|n| [n + 1, next.music_repeat_count, -9999, -1])
        } else {
            None
        };
        self.policy = next;
        Ok(restart)
    }
    pub(crate) fn restore_seed(&self) -> Result<AudioRestoreSeed> {
        ensure!(
            !self.pending(),
            "NNN audio restore seed requires drained native dependencies"
        );
        Ok(AudioRestoreSeed {
            music_runtime: self.music_runtime.clone(),
            client: self.client,
            policy: self.policy.clone(),
            music: self.music.clone(),
            sound: self.sound.clone(),
            resolver: self.resolver.clone(),
            pool: self.pool.clone(),
            system_sounds: self.system_sounds.clone(),
            system_wave_resolver: self.system_wave_resolver.clone(),
            replay_voice_resolver: self.replay_voice_resolver.clone(),
            button_catalogues: self.button_catalogues.clone(),
            button_voices: self
                .button_voices
                .as_ref()
                .map(|owner| owner.restore_seed())
                .transpose()?,
        })
    }
    /// Chart SetGameStatusByLoad restores native loop-work records without
    /// replaying each voice/SFX channel. Keep all raw filename tail bytes for
    /// the next native save, alongside the interpreted current-volume policy.
    pub(crate) fn restore_native_work_records(
        &mut self,
        work: crate::native_save_audio::SavedAudioWork,
    ) -> Result<()> {
        let policy = work.prepare_policy(&self.policy)?;
        self.policy = policy;
        self.restored_work = Some(work);
        Ok(())
    }
    /// Original export merges current native fields into the retained record,
    /// preserving raw name tails and unknown work words from a previous load.
    pub(crate) fn original_audio_work(&self) -> Result<crate::native_save_audio::SavedAudioWork> {
        ensure!(
            !self.pending(),
            "NNN audio export requires drained native dependencies"
        );
        let mut saved =
            self.restored_work
                .clone()
                .unwrap_or(crate::native_save_audio::SavedAudioWork {
                    music_number: -1,
                    music_repeat_count: 0,
                    sound: [[0; 16]; 8],
                    voice: [[0; 16]; 8],
                    voice_names: [[0; 64]; 8],
                });
        saved.music_number = self.policy.last_music_number.unwrap_or(-1);
        saved.music_repeat_count = self.policy.music_repeat_count;
        saved.sound = self.policy.sound_work;
        saved.voice = self.policy.voice_work;
        for (slot, name) in self.policy.voice_names.iter().enumerate() {
            let record = &mut saved.voice_names[slot];
            let end = record.iter().position(|byte| *byte == 0).unwrap_or(64);
            if &record[..end] != name.as_slice() {
                ensure!(
                    name.len() <= 62 && !name.contains(&0),
                    "NNN newly received voice name record"
                );
                record[..name.len()].copy_from_slice(name);
                record[name.len()] = 0;
                record[name.len() + 1] = 0;
            }
        }
        Ok(saved)
    }
    pub fn client(&self) -> ClientId {
        self.client
    }
    pub fn new(
        client: ClientId,
        policy: NativeAudioPolicy,
        music: Names,
        sound: Names,
        resolver: AudioResolver,
    ) -> Result<Self> {
        ensure!(client.0 != 0, "NNN audio client identity");
        Ok(Self {
            policy,
            client,
            music: Arc::new(music),
            sound: Arc::new(sound),
            resolver,
            pool: TaskPool::new("nnn-native-audio-lookup", 1, 1)?,
            pending: None,
            outer_music: None,
            system_sounds: None,
            system_pending: None,
            system_wave_resolver: None,
            replay_voice_resolver: None,
            button_catalogues: None,
            button_voices: None,
            restored_work: None,
            voice_lengths_ms: [None; 8],
            music_runtime: Default::default(),
            pending_music_read: None,
            music_read_mark: None,
            message_voice_start: None,
            api_calls: 0,
        })
    }
    pub fn pending(&self) -> bool {
        self.pending.is_some()
            || self.outer_music.is_some()
            || self.system_pending.is_some()
            || self.button_voice_pending()
    }
    pub(crate) fn callback_pending(&self) -> bool {
        self.pending.is_some()
    }
    pub(crate) fn outer_music_pending(&self) -> bool {
        self.outer_music.is_some()
    }
    pub(crate) fn system_sound_pending(&self) -> bool {
        self.system_pending.is_some()
    }
    pub(crate) fn button_voice_busy(&self) -> bool {
        self.button_voice_pending()
    }
    fn button_voice_pending(&self) -> bool {
        self.button_voices
            .as_ref()
            .is_some_and(|owner| !owner.idle())
    }
    pub(crate) fn configure_button_voices(
        &mut self,
        catalogues: Vec<Arc<Names>>,
        number: i32,
        base: u32,
        resources: crate::resource_bridge::NativeResources,
        names: crate::native_resources::ResourceNames,
    ) -> Result<()> {
        ensure!(
            self.button_voices.is_none() && resources.client() == self.client,
            "NNN button voice configuration owner"
        );
        ensure!(
            base > 0 && base.checked_add(1).is_some(),
            "NNN button voice channel range"
        );
        let b = self.policy.bindings;
        ensure!(
            !(base..=base + 1).contains(&b.music)
                && (base + 1 < b.sound_base || base >= b.sound_base + 8)
                && (base + 1 < b.voice_base || base >= b.voice_base + 8)
                && !self
                    .system_sounds
                    .as_ref()
                    .is_some_and(|s| base <= s.base + 1 && s.base <= base + 1),
            "NNN button voice channel collision"
        );
        let replay_resources = resources.clone();
        let replay_names = names.clone();
        self.replay_voice_resolver = Some(Arc::new(move |name, cancel| {
            replay_resources.optional_replay_voice(&replay_names, name, cancel)
        }));
        let resolver: crate::native_button_voice::Resolver =
            Arc::new(move |name, cancel| resources.optional_system_wave(&names, name, cancel));
        self.button_voices = Some(crate::native_button_voice::ButtonVoiceOwner::new(
            self.client,
            base,
            number,
            self.pool.clone(),
            resolver.clone(),
        )?);
        self.button_catalogues = Some(crate::native_button_voice::Catalogues { names: catalogues });
        self.system_wave_resolver = Some(resolver);
        Ok(())
    }
    pub(crate) fn button_voice_filename(
        &self,
        use_system_voice: i32,
        number: i32,
    ) -> Result<Option<Vec<u8>>> {
        self.button_catalogues
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("NNN button voice catalogues absent"))?
            .filename(use_system_voice, number)
    }
    pub(crate) fn poll_button_voice(
        &mut self,
        host: &mut dyn HostServices,
        name: &[u8],
        inputs: crate::native_button_voice::Inputs,
    ) -> Result<bool> {
        if self.pending.is_some() || self.outer_music.is_some() || self.system_pending.is_some() {
            return Ok(false);
        }
        let owner = self
            .button_voices
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("NNN button voice channels absent"))?;
        let before = owner.api_calls;
        let result = owner.poll(host, name, inputs);
        self.api_calls += owner.api_calls - before;
        result
    }
    /// Last fully decoded native voice duration, even when playback is muted.
    /// It is a measured source fact, independent of current channel playback.
    pub(crate) fn voice_length_ms(&self, channel: usize) -> Option<u32> {
        self.voice_lengths_ms.get(channel).copied().flatten()
    }
    /// Completion observation for the most recently admitted callback, not a
    /// test of stale decoder metadata or the current channel playing state.
    pub fn take_message_voice_start(&mut self) -> Option<(usize, u32)> {
        self.message_voice_start.take()
    }
    pub fn take_music_read_mark(&mut self) -> Option<i32> {
        self.music_read_mark.take()
    }
    fn complete_callback(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        plan: NativeAudioPlan,
    ) -> Result<bool> {
        let voice = if plan
            .source
            .as_ref()
            .is_some_and(|s| s.kind == crate::native_audio::SourceKind::Voice)
        {
            let channel = match plan.operation {
                AudioOperation::Play { channel, .. } | AudioOperation::MeasureVoice { channel } => {
                    channel
                }
                _ => bail!("NNN successful voice without voice operation"),
            };
            let index = usize::try_from(
                channel
                    .checked_sub(self.policy.bindings.voice_base)
                    .ok_or_else(|| anyhow::anyhow!("NNN message voice base"))?,
            )?;
            Some((
                index,
                self.voice_length_ms(index)
                    .ok_or_else(|| anyhow::anyhow!("NNN successful voice lacks measured length"))?,
            ))
        } else {
            None
        };
        machine.acknowledge(call)?;
        self.policy = plan.next;
        self.message_voice_start = voice;
        self.music_read_mark = self.pending_music_read.take();
        Ok(true)
    }
    pub(crate) fn configure_system_sounds(
        &mut self,
        names: Option<Names>,
        disabled: bool,
        multi: bool,
        base: u32,
    ) -> Result<()> {
        ensure!(
            self.system_sounds.is_none() && base > 0 && base.checked_add(1).is_some(),
            "NNN system sound policy identity"
        );
        let bindings = self.policy.bindings;
        ensure!(
            !(base..=base + 1).contains(&bindings.music)
                && (base + 1 < bindings.sound_base || base >= bindings.sound_base + 8)
                && (base + 1 < bindings.voice_base || base >= bindings.voice_base + 8),
            "NNN system sound channel collision"
        );
        ensure!(
            !self
                .button_voices
                .as_ref()
                .is_some_and(|v| base <= v.base + 1 && v.base <= base + 1),
            "NNN system/button voice channel collision"
        );
        self.system_sounds = Some(SystemSoundPolicy {
            names: names.map(Arc::new),
            disabled,
            multi,
            base,
            current: 0,
            preloaded: Vec::new(),
        });
        Ok(())
    }
    /// Original one-based UI sound numbers select sys waves and two alternating
    /// channels; this never reuses the script sound bank or its playback cursor.
    pub(crate) fn poll_system_sound(
        &mut self,
        host: &mut dyn HostServices,
        number: i32,
    ) -> Result<bool> {
        let s = self.policy.settings;
        self.poll_system_sound_typed(
            host,
            number,
            system_audio::SoundInputs {
                total_off: false,
                voice_enabled: s.voice_enabled,
                sound_enabled: s.sound_enabled,
                voice_volume: s.voice_volume,
                sound_volume: s.sound_volume,
                volume_type: 0,
                name_delta: 0,
            },
        )
    }
    /// Original GeneralInit PlayCommonBGM. One native audio policy is shared
    /// with background SPT; resource lookup and decoder receipts remain async.
    pub(crate) fn poll_outer_music(
        &mut self,
        host: &mut dyn HostServices,
        words: [i32; 4],
        restart: bool,
    ) -> Result<bool> {
        self.poll_outer_request(host, OuterMusicRequest::Script { words, restart })
    }
    pub(crate) fn poll_listen_music(
        &mut self,
        host: &mut dyn HostServices,
        music: Option<crate::native_audio::ListenMusic>,
    ) -> Result<bool> {
        self.poll_outer_request(host, OuterMusicRequest::Listen(music))
    }
    pub(crate) fn poll_replay_voice(
        &mut self,
        host: &mut dyn HostServices,
        name: Vec<u8>,
    ) -> Result<bool> {
        self.poll_replay_voice_with_percent(host, name, self.policy.settings.voice_volume)
    }
    pub(crate) fn poll_replay_voice_with_percent(
        &mut self,
        host: &mut dyn HostServices,
        name: Vec<u8>,
        percent: i32,
    ) -> Result<bool> {
        if self.pending.is_some() || self.system_pending.is_some() || self.button_voice_pending() {
            return Ok(false);
        }
        if self.outer_music.is_none() && self.policy.settings.voice_enabled {
            host.submit_audio(
                self.client,
                crysirror_core::service::AudioRequest::Stop {
                    channel: self.policy.bindings.voice_base,
                    fade_ms: 0.0,
                },
            )?;
            self.api_calls += 1;
        }
        self.poll_outer_request(host, OuterMusicRequest::ReplayVoice { name, percent })
    }
    pub(crate) fn stop_gallery_script_slots(
        &mut self,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        if self.pending() {
            return Ok(false);
        }
        self.stop_script_slots(host)?;
        Ok(true)
    }
    fn poll_outer_request(
        &mut self,
        host: &mut dyn HostServices,
        request: OuterMusicRequest,
    ) -> Result<bool> {
        if self.pending.is_some() || self.system_pending.is_some() || self.button_voice_pending() {
            return Ok(false);
        }
        if self.outer_music.is_none() {
            let playing = host.audio_playing(self.client, self.policy.bindings.music)?;
            let plan = match &request {
                OuterMusicRequest::Script { words, restart } => {
                    let number = words[0];
                    if number <= 0 {
                        if *restart {
                            let plan = self.policy.music(&[-1], &self.music, playing)?;
                            self.control_plan(host, &plan)?;
                            self.policy = plan.next;
                        }
                        return Ok(true);
                    }
                    if !restart && self.policy.music_number == Some(number - 1) {
                        return Ok(true);
                    }
                    self.policy.music(words, &self.music, playing)?
                }
                OuterMusicRequest::Listen(music) => {
                    self.policy.listen_music(music.as_ref(), playing)?
                }
                OuterMusicRequest::ReplayVoice { name, percent } => {
                    let mut plan = self.policy.replay_voice(name)?;
                    if let AudioOperation::Play { parameters, .. } = &mut plan.operation {
                        parameters.gain =
                            crate::native_audio::legacy_gain((*percent).clamp(0, 100))?;
                    }
                    plan
                }
            };
            let Some(source) = plan.source.clone() else {
                self.control_plan(host, &plan)?;
                self.policy = plan.next;
                return Ok(true);
            };
            let resolver = self.resolver.clone();
            let replay = matches!(&request, OuterMusicRequest::ReplayVoice { .. })
                .then(|| self.replay_voice_resolver.clone())
                .flatten();
            let task = self.pool.spawn(move |cancel| {
                cancel.check()?;
                let value = if let Some(replay) = replay {
                    replay(&source.name, &cancel)?
                } else {
                    Some(resolver(&source, &cancel)?)
                };
                cancel.check()?;
                Ok(value)
            })?;
            self.outer_music = Some(OuterMusicPending::Resolve {
                request,
                plan,
                task,
            });
            return Ok(false);
        }
        let expected = match self.outer_music.as_ref().unwrap() {
            OuterMusicPending::Resolve { request, .. }
            | OuterMusicPending::Prepare { request, .. } => request,
        };
        ensure!(
            request == *expected,
            "NNN outer music request changed while pending"
        );
        let pending = self.outer_music.take().unwrap();
        match pending {
            OuterMusicPending::Resolve {
                request: expected,
                plan,
                mut task,
            } => {
                ensure!(
                    request == expected,
                    "NNN outer music request changed while pending"
                );
                let Some(source) = task.try_take()? else {
                    self.outer_music = Some(OuterMusicPending::Resolve {
                        request,
                        plan,
                        task,
                    });
                    return Ok(false);
                };
                let Some(source) = source else {
                    return Ok(true);
                };
                let audio_request = self.source_request(host, &plan, source)?;
                let ticket = host.begin_audio(self.client, audio_request)?;
                self.api_calls += 1;
                self.outer_music = Some(OuterMusicPending::Prepare {
                    request,
                    plan,
                    ticket,
                });
                Ok(false)
            }
            OuterMusicPending::Prepare {
                request: expected,
                plan,
                ticket,
            } => {
                ensure!(
                    request == expected,
                    "NNN outer music request changed while preparing"
                );
                let state = host.audio_preparation(self.client, ticket)?;
                match state.phase {
                    AudioPreparePhase::Pending => {
                        self.outer_music = Some(OuterMusicPending::Prepare {
                            request,
                            plan,
                            ticket,
                        });
                        Ok(false)
                    }
                    AudioPreparePhase::Failed => {
                        host.release_audio(self.client, ticket)?;
                        bail!(
                            "NNN shared outer music preparation failed: {}",
                            state.error.unwrap_or_else(|| "no diagnostic".into())
                        )
                    }
                    AudioPreparePhase::Ready => {
                        host.release_audio(self.client, ticket)?;
                        self.policy = plan.next;
                        Ok(true)
                    }
                }
            }
        }
    }
    /// Original outer Movie Init uses StopMusic's configured default fade,
    /// then immediately stops the eight script-sound and eight voice slots.
    /// This is not a fabricated SPT receipt or an alternate mixer.
    /// Final native host shutdown: cancel owned lookup/prepare dependencies,
    /// then stop only this adapter's engine-owned channels. No VM receipt is
    /// acknowledged and another client's mixer/decoder state is untouched.
    pub(crate) fn shutdown(&mut self, host: &mut dyn HostServices) -> Result<()> {
        // A ready voice preparation may already own a published resource even
        // when the VM has not observed it. Retire both owners on cancellation.
        // Keep cleaning the other channels if an individual cleanup fails.
        let mut cleanup_error = None;
        if let Some(owner) = &mut self.button_voices {
            let before = owner.api_calls;
            if let Err(error) = owner.shutdown(host) {
                cleanup_error = Some(error);
            }
            self.api_calls += owner.api_calls - before;
        }
        let mut record = |result: crysirror_core::Result<()>| {
            if let Err(error) = result {
                cleanup_error.get_or_insert_with(|| anyhow::Error::from(error));
            }
        };
        match self.pending.take() {
            Some(Pending::Prepare { ticket, .. }) => {
                record(host.release_audio(self.client, ticket));
            }
            Some(Pending::VoiceResource { ticket, .. }) => {
                let resource = match host.audio_resource_preparation(self.client, ticket) {
                    Ok(state) => state.resource,
                    Err(error) => {
                        record(Err(error));
                        None
                    }
                };
                record(host.release_audio_resource_preparation(self.client, ticket));
                if let Some(resource) = resource {
                    record(host.release_audio_resource(self.client, resource));
                }
            }
            // Dropping the scoped task cancels an outstanding name lookup.
            Some(Pending::Resolve { .. }) | None => {}
        }
        if let Some(OuterMusicPending::Prepare { ticket, .. }) = self.outer_music.take() {
            record(host.release_audio(self.client, ticket));
        }
        if let Some(SystemSoundPending::Prepare { ticket, .. }) = self.system_pending.take() {
            record(host.release_audio(self.client, ticket));
        }
        let mut channels = vec![self.policy.bindings.music, self.tail_channel()?];
        for base in [
            self.policy.bindings.sound_base,
            self.policy.bindings.voice_base,
        ] {
            channels.extend(base..base + 8);
        }
        if let Some(system) = &self.system_sounds {
            channels.extend(system.base..system.base + 2);
        }
        channels.sort_unstable();
        channels.dedup();
        for channel in channels {
            record(host.submit_audio(
                self.client,
                crysirror_core::service::AudioRequest::Stop {
                    channel,
                    fade_ms: 0.,
                },
            ));
            self.api_calls += 1;
        }
        self.policy.music_number = None;
        self.policy.last_music_number = None;
        self.pending_music_read = None;
        self.music_read_mark = None;
        self.message_voice_start = None;
        cleanup_error.map_or(Ok(()), Err)
    }
    pub(crate) fn stop_for_outer_movie(&mut self, host: &mut dyn HostServices) -> Result<()> {
        ensure!(
            self.pending.is_none() && self.outer_music.is_none(),
            "NNN movie transition with pending native audio"
        );
        let playing = host.audio_playing(self.client, self.policy.bindings.music)?;
        let plan = self.policy.music(&[-1], &self.music, playing)?;
        self.control_plan(host, &plan)?;
        self.policy = plan.next;
        self.stop_script_slots(host)
    }
    /// Reviewed legacy52f4c0 stops8sound/4voice channels. ClearWork53fa30
    /// separately clears only each of8 stored work-row enable words.
    pub(crate) fn initialize_outer_title(&mut self, host: &mut dyn HostServices) -> Result<()> {
        ensure!(
            self.pending.is_none() && self.outer_music.is_none() && self.system_pending.is_none(),
            "NNN Title Init requires completed native audio dependencies"
        );
        self.stop_script_slots(host)?;
        for row in &mut self.policy.sound_work {
            row[0] = 0;
        }
        for row in &mut self.policy.voice_work {
            row[0] = 0;
        }
        Ok(())
    }
    fn stop_script_slots(&mut self, host: &mut dyn HostServices) -> Result<()> {
        for (base, count) in [
            (self.policy.bindings.sound_base, 8),
            (self.policy.bindings.voice_base, 4),
        ] {
            for index in 0..count {
                host.submit_audio(
                    self.client,
                    crysirror_core::service::AudioRequest::Stop {
                        channel: base + index,
                        fade_ms: 0.0,
                    },
                )?;
                self.api_calls += 1;
            }
        }
        Ok(())
    }
    pub fn try_commit(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        self.try_commit_with_skip(
            machine,
            call,
            host,
            &crate::native_skip::NativeSkipPolicy::default(),
            crate::native_audio::AudioCallbackContext::default(),
        )
    }
    pub fn try_commit_with_skip(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
        skip: &crate::native_skip::NativeSkipPolicy,
        context: crate::native_audio::AudioCallbackContext,
    ) -> Result<bool> {
        machine.validate_receipt(call)?;
        if self.outer_music.is_some()
            || self.system_pending.is_some()
            || self.button_voice_pending()
        {
            return Ok(false);
        }
        if self.pending.is_none() {
            let playing = host.audio_playing(self.client, self.policy.bindings.music)?;
            let admission = self.policy.admit_with_skip(
                call,
                &self.music,
                &self.sound,
                playing,
                skip,
                context,
            )?;
            let plan = admission.plan;
            self.pending_music_read = admission.music_mark;
            self.music_read_mark = None;
            self.message_voice_start = None;
            if let Some(channel) = admission.stop_before {
                host.submit_audio(
                    self.client,
                    crysirror_core::service::AudioRequest::Stop {
                        channel,
                        fade_ms: 0.,
                    },
                )?;
                self.api_calls += 1;
            }
            if let Some(source) = plan.source.clone() {
                let resolver = self.resolver.clone();
                let task = self.pool.spawn(move |cancel| {
                    cancel.check()?;
                    let path = resolver(&source, &cancel)?;
                    cancel.check()?;
                    Ok(path)
                })?;
                self.pending = Some(Pending::Resolve {
                    call: call.clone(),
                    plan,
                    task,
                });
            } else {
                self.control_plan(host, &plan)?;
                return self.complete_callback(machine, call, plan);
            }
        }
        let pending = self.pending.take().unwrap();
        match pending {
            Pending::Resolve {
                call: receipt,
                plan,
                mut task,
            } => {
                ensure!(receipt == *call, "NNN audio lookup receipt changed");
                if let Some(path) = task.try_take()? {
                    if plan
                        .source
                        .as_ref()
                        .is_some_and(|source| source.kind == crate::native_audio::SourceKind::Voice)
                    {
                        let ticket = match path {
                            crate::resource_bridge::ResolvedAudio::Playlist(_) => {
                                bail!("NNN voice cannot resolve a music playlist")
                            }
                            crate::resource_bridge::ResolvedAudio::Asset(path) => host
                                .begin_audio_resource(
                                    self.client,
                                    crysirror_core::service::AudioAsset {
                                        path,
                                        frame_limit: None,
                                    },
                                    true,
                                )?,
                            crate::resource_bridge::ResolvedAudio::Encoded(source) => host
                                .begin_encoded_audio_resource(
                                    self.client,
                                    "NNN native voice",
                                    source,
                                    true,
                                )?,
                        };
                        self.api_calls += 1;
                        self.pending = Some(Pending::VoiceResource {
                            call: receipt,
                            plan,
                            ticket,
                        });
                        return Ok(false);
                    }
                    let request = self.source_request(host, &plan, path)?;
                    let ticket = host.begin_audio(self.client, request)?;
                    self.api_calls += 1;
                    self.pending = Some(Pending::Prepare {
                        call: receipt,
                        plan,
                        ticket,
                    });
                } else {
                    self.pending = Some(Pending::Resolve {
                        call: receipt,
                        plan,
                        task,
                    });
                }
                Ok(false)
            }
            Pending::VoiceResource {
                call: receipt,
                plan,
                ticket,
            } => {
                ensure!(receipt == *call, "NNN voice resource receipt changed");
                let state = host.audio_resource_preparation(self.client, ticket)?;
                match state.phase {
                    AudioPreparePhase::Pending => {
                        self.pending = Some(Pending::VoiceResource {
                            call: receipt,
                            plan,
                            ticket,
                        });
                        Ok(false)
                    }
                    AudioPreparePhase::Failed => {
                        host.release_audio_resource_preparation(self.client, ticket)?;
                        bail!(
                            "NNN voice resource preparation failed: {}",
                            state.error.as_deref().unwrap_or("no diagnostic")
                        )
                    }
                    AudioPreparePhase::Ready => {
                        // Release the receipt even if metadata validation or
                        // playback admission fails. Published source ownership
                        // is independent and is released after the attempt.
                        let prepared = (|| -> Result<_> {
                            host.release_audio_resource_preparation(self.client, ticket)?;
                            let resource = state.resource.ok_or_else(|| {
                                anyhow::anyhow!("NNN ready voice has no resource")
                            })?;
                            let info = state.info.ok_or_else(|| {
                                anyhow::anyhow!("NNN ready voice has no metadata")
                            })?;
                            ensure!(
                                resource.client == self.client.0 && info.source_rate != 0,
                                "NNN voice resource owner/rate"
                            );
                            let frames = info.source_frames.ok_or_else(|| {
                                anyhow::anyhow!("NNN voice has no measured length")
                            })?;
                            let duration = u32::try_from(
                                frames.checked_mul(1000).ok_or_else(|| {
                                    anyhow::anyhow!("NNN voice duration overflow")
                                })? / u64::from(info.source_rate),
                            )?;
                            let channel = match &plan.operation {
                                AudioOperation::Play { channel, .. }
                                | AudioOperation::MeasureVoice { channel } => *channel,
                                _ => bail!("NNN voice preparation without voice operation"),
                            };
                            let native_channel = usize::try_from(
                                channel
                                    .checked_sub(self.policy.bindings.voice_base)
                                    .ok_or_else(|| anyhow::anyhow!("NNN voice channel base"))?,
                            )?;
                            ensure!(native_channel < 8, "NNN voice channel bounds");
                            let preparation = match &plan.operation {
                                AudioOperation::Play { .. } => host
                                    .begin_audio(self.client, plan.resource_request(resource)?)
                                    .map(Some),
                                AudioOperation::MeasureVoice { .. } => Ok(None),
                                _ => unreachable!(),
                            }?;
                            Ok((native_channel, duration, preparation))
                        })();
                        // Admitted playback owns its immutable source lease;
                        // disabled playback never allocates a mixer voice.
                        let released = state
                            .resource
                            .map(|resource| host.release_audio_resource(self.client, resource))
                            .transpose();
                        let (native_channel, duration, preparation) = prepared?;
                        self.voice_lengths_ms[native_channel] = Some(duration);
                        if let Some(ticket) = preparation {
                            self.api_calls += 1;
                            self.pending = Some(Pending::Prepare {
                                call: receipt,
                                plan,
                                ticket,
                            });
                            released?;
                            Ok(false)
                        } else {
                            released?;
                            self.complete_callback(machine, call, plan)
                        }
                    }
                }
            }
            Pending::Prepare {
                call: receipt,
                plan,
                ticket,
            } => {
                ensure!(receipt == *call, "NNN audio preparation receipt changed");
                let state = host.audio_preparation(self.client, ticket)?;
                let AudioOperation::Play { channel, .. } = plan.operation else {
                    bail!("NNN preparation without playback")
                };
                ensure!(
                    state.channel == channel,
                    "NNN shared audio channel receipt mismatch"
                );
                match state.phase {
                    AudioPreparePhase::Pending => {
                        self.pending = Some(Pending::Prepare {
                            call: receipt,
                            plan,
                            ticket,
                        });
                        Ok(false)
                    }
                    AudioPreparePhase::Failed => {
                        host.release_audio(self.client, ticket)?;
                        bail!(
                            "NNN shared audio preparation failed: {}",
                            state.error.as_deref().unwrap_or("no diagnostic")
                        )
                    }
                    AudioPreparePhase::Ready => {
                        host.release_audio(self.client, ticket)?;
                        self.complete_callback(machine, call, plan)
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod outer_system_sound_tests {
    use super::*;
    use crate::native_audio::{AudioBindings, NativeAudioLookup, SourceKind};
    use crate::native_resources::ResourceNames;
    use std::time::{Duration, Instant};
    fn wave() -> Arc<[u8]> {
        let mut bytes = Vec::new();
        bytes.extend(b"RIFF");
        bytes.extend(44u32.to_le_bytes());
        bytes.extend(b"WAVEfmt ");
        bytes.extend(16u32.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(48000u32.to_le_bytes());
        bytes.extend(96000u32.to_le_bytes());
        bytes.extend(2u16.to_le_bytes());
        bytes.extend(16u16.to_le_bytes());
        bytes.extend(b"data");
        bytes.extend(8u32.to_le_bytes());
        for value in [1000i16, -1000, 1000, -1000] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.into()
    }
    #[test]
    fn native_outer_catalogue_audio_preserves_loop_policy_and_original_saved_voice_work() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("assets")).unwrap();
        let project = serde_json::from_value(serde_json::json!({
            "schema": 2, "name": "listen music", "content_revision": "1", "assets": "assets",
            "canvas": [4,4], "clients": [{"name":"native","adapter":"crysirror-script-v1","game":"authored"}]
        })).unwrap();
        let mut runtime = crysirror_runtime::Runtime::with_asset_root(
            project,
            root.path(),
            &root.path().join("state"),
            Default::default(),
        )
        .unwrap();
        runtime.tick(0, Default::default()).unwrap();
        let mut settings =
            crate::native_services_bootstrap::NativePreferences::original_defaults().audio;
        settings.music_enabled = false;
        let mut policy = NativeAudioPolicy::new(
            settings,
            AudioBindings {
                music: 0,
                sound_base: 16,
                voice_base: 32,
            },
        )
        .unwrap();
        policy.music_number = Some(17);
        policy.last_music_number = Some(17);
        let bytes = wave();
        let resolver: AudioResolver = Arc::new(move |source, cancel| {
            cancel.check()?;
            match source.kind {
                SourceKind::Music => assert_eq!(source.name, b"catalogue_only"),
                SourceKind::Voice => assert_eq!(source.name, b"ga001"),
                _ => panic!("unexpected catalogue audio source"),
            }
            Ok(crate::resource_bridge::ResolvedAudio::Encoded(
                crysirror_core::service::EncodedAudio {
                    bytes: bytes.clone(),
                    frame_limit: None,
                },
            ))
        });
        // The table is deliberately empty: ListenBGM never requires a
        // synthetic song ID or mutates the native original-save music fields.
        let mut bridge = AudioBridge::new(
            ClientId(1),
            policy,
            Names { tokens: vec![] },
            Names { tokens: vec![] },
            resolver,
        )
        .unwrap();
        for looping in [false, true] {
            let music = crate::native_audio::ListenMusic {
                name: b"catalogue_only".to_vec(),
                looping,
                volume: 90,
                fade_in: 0,
                fade_out: 0,
            };
            let end = Instant::now() + Duration::from_secs(5);
            loop {
                if bridge
                    .poll_listen_music(&mut runtime.host, Some(music.clone()))
                    .unwrap()
                {
                    break;
                }
                runtime.tick(0, Default::default()).unwrap();
                assert!(Instant::now() < end);
                std::thread::yield_now();
            }
            let info = runtime.host.audio_info(ClientId(1), 0).unwrap().unwrap();
            assert!(info.source_playing);
            assert_eq!(info.looping, looping);
            assert_eq!(bridge.policy.music_number, None);
            assert_eq!(bridge.policy.last_music_number, None);
        }
        assert!(bridge.poll_listen_music(&mut runtime.host, None).unwrap());
        assert!(
            runtime
                .host
                .audio_gain_envelope_info(ClientId(1), 0)
                .unwrap()
                .unwrap()
                .spec
                .stop_at_end
        );
        bridge.policy.voice_work[0][0] = 1;
        bridge.policy.voice_work[0][1] = 77;
        bridge.policy.voice_names[0] = b"original_script".to_vec();
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            if bridge
                .poll_replay_voice(&mut runtime.host, b"ga001".to_vec())
                .unwrap()
            {
                break;
            }
            runtime.tick(0, Default::default()).unwrap();
            assert!(Instant::now() < end);
            std::thread::yield_now();
        }
        assert!(runtime.host.audio_playing(ClientId(1), 32).unwrap());
        assert_eq!(bridge.policy.voice_names[0], b"original_script");
        assert_eq!(bridge.policy.voice_work[0][..2], [1, 77]);
        bridge.policy.settings.voice_enabled = false;
        assert!(
            bridge
                .poll_replay_voice(&mut runtime.host, b"missing_when_disabled".to_vec())
                .unwrap()
        );
        assert!(runtime.host.audio_playing(ClientId(1), 32).unwrap());
        bridge.shutdown(&mut runtime.host).unwrap();
        assert!(!bridge.pending());
        assert!(!runtime.host.audio_playing(ClientId(1), 0).unwrap());
        assert_eq!(runtime.host.now_ms(), 0);
    }
    #[test]
    fn system_ui_uses_sys_lookup_two_channels_and_engine_decoder_without_touching_music_policy() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("assets")).unwrap();
        let project:crysirror_runtime::Project=serde_json::from_value(serde_json::json!({"schema":2,"name":"system sound","content_revision":"1","assets":"assets","canvas":[4,4],"clients":[{"name":"native","adapter":"crysirror-script-v1","game":"authored"},{"name":"other","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let mut runtime = crysirror_runtime::Runtime::with_asset_root(
            project,
            root.path(),
            &root.path().join("state"),
            Default::default(),
        )
        .unwrap();
        runtime.tick(0, Default::default()).unwrap();
        let mut policy = NativeAudioPolicy::new(
            crate::native_services_bootstrap::NativePreferences::original_defaults().audio,
            AudioBindings {
                music: 0,
                sound_base: 16,
                voice_base: 32,
            },
        )
        .unwrap();
        policy.music_number = Some(17);
        let bytes = wave();
        let resolver: AudioResolver = Arc::new(move |source, cancel| {
            cancel.check()?;
            assert_eq!(source.kind, SourceKind::SystemSound);
            let NativeAudioLookup::Sound { loose } = source.lookup(&ResourceNames::new("")?)?
            else {
                panic!()
            };
            assert_eq!(loose, "vaw/sys/click.vaw");
            Ok(crate::resource_bridge::ResolvedAudio::Encoded(
                crysirror_core::service::EncodedAudio {
                    bytes: bytes.clone(),
                    frame_limit: None,
                },
            ))
        });
        let mut bridge = AudioBridge::new(
            ClientId(1),
            policy,
            Names { tokens: vec![] },
            Names { tokens: vec![] },
            resolver,
        )
        .unwrap();
        bridge
            .configure_system_sounds(
                Some(Names {
                    tokens: vec![b"click".to_vec(), b"100".to_vec()],
                }),
                false,
                false,
                64,
            )
            .unwrap();
        for expected in [65, 64] {
            let end = Instant::now() + Duration::from_secs(5);
            loop {
                if bridge.poll_system_sound(&mut runtime.host, 1).unwrap() {
                    break;
                }
                runtime.tick(0, Default::default()).unwrap();
                assert!(Instant::now() < end);
                std::thread::yield_now();
            }
            assert!(runtime.host.audio_playing(ClientId(1), expected).unwrap());
            assert!(
                !runtime
                    .host
                    .audio_playing(ClientId(1), if expected == 65 { 64 } else { 65 })
                    .unwrap()
            );
            assert_eq!(bridge.policy.music_number, Some(17));
        }
        let ticket = runtime
            .host
            .begin_audio(
                ClientId(2),
                crysirror_core::service::AudioRequest::Play {
                    channel: 64,
                    source: crysirror_core::service::EncodedAudio {
                        bytes: wave(),
                        frame_limit: None,
                    },
                    intro: None,
                    parameters: crysirror_core::service::AudioParameters {
                        looping: true,
                        ..Default::default()
                    },
                },
            )
            .unwrap();
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            let prepared = runtime.host.audio_preparation(ClientId(2), ticket).unwrap();
            match prepared.phase {
                AudioPreparePhase::Pending => (),
                AudioPreparePhase::Ready => break,
                AudioPreparePhase::Failed => panic!("other client fixture: {:?}", prepared.error),
            }
            runtime.tick(0, Default::default()).unwrap();
            assert!(Instant::now() < end);
            std::thread::yield_now();
        }
        runtime.host.release_audio(ClientId(2), ticket).unwrap();
        bridge.shutdown(&mut runtime.host).unwrap();
        assert!(!bridge.pending());
        assert_eq!(bridge.policy.music_number, None);
        assert!(!runtime.host.audio_playing(ClientId(1), 64).unwrap());
        assert!(!runtime.host.audio_playing(ClientId(1), 65).unwrap());
        assert!(
            runtime.host.audio_playing(ClientId(2), 64).unwrap(),
            "native shutdown cannot stop another client at the same channel"
        );
        assert_eq!(
            runtime.host.now_ms(),
            0,
            "resource polls and shutdown never advance the native clock"
        );
    }
    #[test]
    fn original_title_init_stops_only_legacy_slots_and_clears_enable_words_without_music_or_peer_loss()
     {
        use crysirror_core::service::{AudioParameters, AudioRequest, PcmAudio};
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("assets")).unwrap();
        let project:crysirror_runtime::Project=serde_json::from_value(serde_json::json!({"schema":2,"name":"title audio init","content_revision":"1","assets":"assets","canvas":[4,4],"clients":[{"name":"one","adapter":"crysirror-script-v1","game":"authored"},{"name":"two","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let mut runtime = crysirror_runtime::Runtime::with_asset_root(
            project,
            dir.path(),
            &dir.path().join("state"),
            Default::default(),
        )
        .unwrap();
        for (client, channel) in [(1, 0), (1, 16), (1, 23), (1, 32), (1, 35), (1, 36), (2, 32)] {
            runtime
                .host
                .submit_audio(
                    ClientId(client),
                    AudioRequest::PlayPcm {
                        channel,
                        source: PcmAudio::new(48000, vec![[0.2, 0.4]; 100]).unwrap(),
                        parameters: AudioParameters::default(),
                    },
                )
                .unwrap();
        }
        let mut policy = crate::native_audio::NativeAudioPolicy::new(
            crate::native_services_bootstrap::NativePreferences::original_defaults().audio,
            crate::native_audio::AudioBindings {
                music: 0,
                sound_base: 16,
                voice_base: 32,
            },
        )
        .unwrap();
        policy.music_number = Some(17);
        for row in &mut policy.sound_work {
            row[0] = 1;
            row[1] = 37;
        }
        for row in &mut policy.voice_work {
            row[0] = 1;
            row[1] = 51;
        }
        let mut bridge = AudioBridge::new(
            ClientId(1),
            policy,
            Names { tokens: vec![] },
            Names { tokens: vec![] },
            Arc::new(|_, _| anyhow::bail!("no decode in Title Init")),
        )
        .unwrap();
        bridge.initialize_outer_title(&mut runtime.host).unwrap();
        assert_eq!(bridge.api_calls, 12);
        for channel in [16, 23, 32, 35] {
            assert!(!runtime.host.audio_playing(ClientId(1), channel).unwrap());
        }
        for (client, channel) in [(1, 0), (1, 36), (2, 32)] {
            assert!(
                runtime
                    .host
                    .audio_playing(ClientId(client), channel)
                    .unwrap()
            );
        }
        assert_eq!(bridge.policy.music_number, Some(17));
        assert!(
            bridge
                .policy
                .sound_work
                .iter()
                .all(|row| row[0] == 0 && row[1] == 37)
        );
        assert!(
            bridge
                .policy
                .voice_work
                .iter()
                .all(|row| row[0] == 0 && row[1] == 51)
        );
        assert_eq!(runtime.host.now_ms(), 0);
    }
}

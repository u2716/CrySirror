// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original53ad20/53b630 fixed audio records. These are native data, not
//! callback parameters, live voice handles or framework mixer snapshots.
use crate::{native_audio::NativeAudioPolicy, native_save::NativeSaveFile};
use anyhow::{Context, Result, ensure};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SavedAudioWork {
    pub music_number: i32,
    pub music_repeat_count: i32,
    pub sound: [[i32; 16]; 8],
    pub voice: [[i32; 16]; 8],
    /// Preserve all64 bytes, including the native tail beyond a terminating
    /// NUL. Only a replay consumer interprets an active filename as a C string.
    pub voice_names: [[u8; 64]; 8],
}
impl SavedAudioWork {
    /// Detached native policy only. Audio decoding/playback still requires an
    /// engine-owned replacement receipt and final original-save owner commit.
    /// SetGameStatusByLoad uses current volume preferences, not later source's
    /// bgmVolume/adjustLoadBGM extensions.
    pub(crate) fn prepare_policy(&self, current: &NativeAudioPolicy) -> Result<NativeAudioPolicy> {
        ensure!(self.music_number >= -1, "NNN saved music number");
        let mut names = std::array::from_fn(|_| Vec::new());
        for (index, raw) in self.voice_names.iter().enumerate() {
            let end = raw.iter().position(|byte| *byte == 0);
            if index < 4 && self.voice[index][0] != 0 {
                ensure!(self.voice[index][1] >= 0, "NNN saved active voice number");
                ensure!(
                    end.is_some_and(|end| end > 0),
                    "NNN saved active voice filename is unterminated or empty"
                );
            }
            names[index] = raw[..end.unwrap_or(raw.len())].to_vec();
        }
        for row in &self.sound {
            ensure!(row[0] == 0 || row[1] >= 0, "NNN saved active sound number");
        }
        let mut next = current.clone();
        next.sound_work = self.sound;
        next.voice_work = self.voice;
        next.voice_names = names;
        next.music_number = (self.music_number != -1).then_some(self.music_number);
        next.last_music_number = next.music_number;
        next.music_repeat_count = self.music_repeat_count;
        next.music_percent = next.settings.music_volume;
        Ok(next)
    }
}
impl NativeSaveFile {
    pub(crate) fn audio_work(&self) -> Result<SavedAudioWork> {
        Self::status_audio_work(self.blocks.get(2).context("NNN saved audio status absent")?)
    }
    pub(crate) fn status_audio_work(status:&crate::native_save::SaveBlock)->Result<SavedAudioWork> {
        ensure!(
            status.code == 2 && status.payload.len() == 7056,
            "NNN saved audio original status ABI"
        );
        let word = |native: usize| -> i32 {
            let at = (native - 8) * 4;
            i32::from_le_bytes(status.payload[at..at + 4].try_into().unwrap())
        };
        let sound =
            std::array::from_fn(|slot| std::array::from_fn(|index| word(52 + slot * 16 + index)));
        let voice =
            std::array::from_fn(|slot| std::array::from_fn(|index| word(184 + slot * 16 + index)));
        let voice_names = std::array::from_fn(|slot| {
            let start = (312 - 8) * 4 + slot * 64;
            status.payload[start..start + 64].try_into().unwrap()
        });
        Ok(SavedAudioWork {
            music_number: word(36),
            music_repeat_count: word(37),
            sound,
            voice,
            voice_names,
        })
    }
    /// Retain all non-audio status bytes and all16 words/64 name bytes. The
    /// input is an already-reviewed native data record, never an AudioRequest.
    pub(crate) fn replace_audio_work(&mut self, audio: &SavedAudioWork) -> Result<()> {
        let status = self
            .blocks
            .get_mut(2)
            .context("NNN saved audio status absent")?;
        ensure!(
            status.code == 2 && status.payload.len() == 7056,
            "NNN saved audio original status ABI"
        );
        let word = |payload: &mut [u8], native: usize, value: i32| {
            let at = (native - 8) * 4;
            payload[at..at + 4].copy_from_slice(&value.to_le_bytes());
        };
        word(&mut status.payload, 36, audio.music_number);
        word(&mut status.payload, 37, audio.music_repeat_count);
        for slot in 0..8 {
            for index in 0..16 {
                word(
                    &mut status.payload,
                    52 + slot * 16 + index,
                    audio.sound[slot][index],
                );
                word(
                    &mut status.payload,
                    184 + slot * 16 + index,
                    audio.voice[slot][index],
                );
            }
            let start = (312 - 8) * 4 + slot * 64;
            status.payload[start..start + 64].copy_from_slice(&audio.voice_names[slot]);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Names,
        machine::{NativeCall, Payload},
        native_audio::AudioBindings,
        native_save::SaveBlock,
    };
    fn file() -> NativeSaveFile {
        NativeSaveFile {
            blocks: vec![
                SaveBlock::new(0, b"authored", vec![]).unwrap(),
                SaveBlock::new(1, b"INFO", vec![]).unwrap(),
                SaveBlock::new(2, b"GAMESTATUS", vec![0xa5; 7056]).unwrap(),
            ],
        }
    }
    fn policy() -> NativeAudioPolicy {
        NativeAudioPolicy::new(
            crate::native_services_bootstrap::NativePreferences::original_defaults().audio,
            AudioBindings {
                music: 0,
                sound_base: 16,
                voice_base: 32,
            },
        )
        .unwrap()
    }
    #[test]
    fn complete_native_records_roundtrip_opaque_words_names_and_unrelated_status() {
        let mut file = file();
        let original = file.blocks[2].payload.clone();
        let mut audio = file.audio_work().unwrap();
        audio.music_number = 0;
        audio.music_repeat_count = 7;
        for slot in 0..8 {
            audio.sound[slot] = std::array::from_fn(|index| (slot * 100 + index) as i32);
            audio.voice[slot] = std::array::from_fn(|index| -(slot as i32 * 100 + index as i32));
            audio.voice_names[slot] = std::array::from_fn(|index| (slot * 13 + index) as u8);
        }
        file.replace_audio_work(&audio).unwrap();
        assert_eq!(file.audio_work().unwrap(), audio);
        for native in [8, 22, 38, 39, 44, 48, 180, 183, 440, 472, 486, 488, 1771] {
            let at = (native - 8) * 4;
            assert_eq!(
                file.blocks[2].payload[at..at + 4],
                original[at..at + 4],
                "unrelated native word {native}"
            );
        }
        let before = file.blocks[2].payload.clone();
        file.blocks[2].code = 9;
        assert!(file.replace_audio_work(&audio).is_err());
        assert_eq!(file.blocks[2].payload, before);
    }
    #[test]
    fn callback_parameters_are_reordered_and_stop_only_clears_original_loop_word() {
        let mut current = policy();
        current.sound_work[2] = std::array::from_fn(|index| 900 + index as i32);
        let call = |words: Vec<i32>| NativeCall {
            identify: 0x66660007,
            code: 0x33330009,
            payload: Payload::Words {
                count: words.len() as u32,
                words,
            },
            serial: 0,
            pc: 0,
            next: 0,
        };
        let names = Names::parse(b"a 0 b 0 c 0 d 0", false).unwrap();
        let plan = current
            .plan(
                &call(vec![3, 2, -17, 1, 255, 128, 127, 37, 254, 129, 0, 5]),
                &names,
                &names,
                false,
            )
            .unwrap();
        assert_eq!(
            plan.next.sound_work[2],
            [
                1, 3, -17, -1, -128, 127, 37, -2, -127, 0, 5, 911, 912, 913, 914, 915
            ]
        );
        assert_eq!(current.sound_work[2][0], 900, "plan cannot mutate source");
        let stopped = plan
            .next
            .plan(&call(vec![-1, 2]), &names, &names, false)
            .unwrap();
        assert_eq!(stopped.next.sound_work[2][0], 0);
        assert_eq!(
            stopped.next.sound_work[2][1..],
            plan.next.sound_work[2][1..]
        );
        let silent_profile = current
            .plan(&call(vec![0, 2, -101, 1]), &names, &names, false)
            .unwrap();
        let crate::native_audio::AudioOperation::Play { parameters, .. } = silent_profile.operation
        else {
            panic!("ordinary native playback")
        };
        assert_eq!(
            parameters.gain,
            crate::native_audio::legacy_gain(0).unwrap(),
            "actual531dc0 has no later-source -101 override"
        );
    }
    #[test]
    fn original_x86_volume_addition_wraps_before_native_clamp() {
        let mut current = policy();
        current.settings.music_volume = 100;
        current.settings.sound_volume = 100;
        let names = Names::parse(b"track 0", false).unwrap();
        for delta in [i32::MIN, i32::MAX] {
            let call = NativeCall {
                serial: 0,
                pc: 0,
                next: 0,
                identify: 0x66660007,
                code: 0x33330009,
                payload: Payload::Words {
                    count: 4,
                    words: vec![0, 0, delta, 1],
                },
            };
            let plan = current.plan(&call, &names, &names, false).unwrap();
            let crate::native_audio::AudioOperation::Play { parameters, .. } = plan.operation
            else {
                panic!("native sound")
            };
            assert_eq!(
                parameters.gain,
                crate::native_audio::legacy_gain(0).unwrap()
            );
            let plan = current.music(&[1, 1, delta, 0, 0], &names, false).unwrap();
            let crate::native_audio::AudioOperation::Play { parameters, .. } = plan.operation
            else {
                panic!("native music")
            };
            assert_eq!(
                parameters.gain,
                crate::native_audio::legacy_gain(1).unwrap()
            );
            let plan = current.music(&[-2, delta], &names, false).unwrap();
            assert!(matches!(
                plan.operation,
                crate::native_audio::AudioOperation::Gain { percent: 1, .. }
            ));
        }
    }
    #[test]
    fn restored_voice_reads_full_saved_name_without_reentering_callback_truncation() {
        let mut file = file();
        file.blocks[2].payload.fill(0);
        let mut audio = file.audio_work().unwrap();
        audio.music_number = -1;
        audio.voice[0][0] = 1;
        for japanese_end in [false, true] {
            audio.voice_names[0][..63].fill(b'v');
            audio.voice_names[0][..2].copy_from_slice(b"AB");
            if japanese_end {
                audio.voice_names[0][61..63].copy_from_slice(&[0x82, 0xa0]);
            }
            let candidate = audio.prepare_policy(&policy()).unwrap();
            let names = Names::parse(b"track 0", false).unwrap();
            let plan = candidate.saved_plain_loop(0, true, &names).unwrap();
            assert_eq!(
                plan.source.as_ref().unwrap().name,
                audio.voice_names[0][..63]
            );
            assert_eq!(plan.next.voice_names[0], audio.voice_names[0][..63]);
            assert_eq!(candidate.voice_work[0], audio.voice[0]);
        }
    }
    #[test]
    fn saved_policy_is_detached_keeps_current_preferences_and_rejects_bad_active_names() {
        let mut file = file();
        file.blocks[2].payload.fill(0);
        let mut audio = file.audio_work().unwrap();
        audio.music_number = 0;
        audio.music_repeat_count = 3;
        audio.sound[7] = std::array::from_fn(|index| index as i32 + 60);
        audio.voice[0][0] = 1;
        audio.voice_names[0][..8].copy_from_slice(b"AB_voice");
        audio.voice_names[0][63] = 203;
        let mut current = policy();
        current.music_percent = 11;
        let next = audio.prepare_policy(&current).unwrap();
        assert_eq!(next.music_number, Some(0));
        assert_eq!(next.music_repeat_count, 3);
        assert_eq!(next.music_percent, current.settings.music_volume);
        assert_eq!(next.sound_work[7], audio.sound[7]);
        assert_eq!(next.voice_names[0], b"AB_voice");
        assert_eq!(current.music_percent, 11);
        audio.voice_names[0].fill(65);
        assert!(audio.prepare_policy(&current).is_err());
        assert_eq!(current.voice_work[0], [0; 16]);
        assert_eq!(
            audio.voice_names[0], [65; 64],
            "invalid saved bytes are not rewritten"
        );
    }
}

#[cfg(test)]
mod music_repetition_tests {
    use crate::{
        config::Names,
        native_audio::{AudioBindings, NativeAudioPolicy},
    };
    use crysirror_core::{path::VfsPath, service::AudioRequest};
    #[test]
    fn original_music_zero_once_and_finite_passes_use_engine_loop_schedules() {
        let policy = NativeAudioPolicy::new(
            crate::native_services_bootstrap::NativePreferences::original_defaults().audio,
            AudioBindings {
                music: 0,
                sound_base: 16,
                voice_base: 32,
            },
        )
        .unwrap();
        let names = Names::parse(b"track 0", false).unwrap();
        for (count, looping, passes) in [
            (0, true, None),
            (1, false, None),
            (-1, false, None),
            (2, true, Some(2)),
            (i32::MAX, true, Some(i32::MAX as u32)),
        ] {
            let plan = policy
                .music(&[1, count, -9999, 0, 0], &names, false)
                .unwrap();
            let AudioRequest::PlayAsset { parameters, .. } = plan
                .asset_request(
                    VfsPath::new(crysirror_core::path::Mount::Asset, "track.wav").unwrap(),
                )
                .unwrap()
            else {
                panic!("native music must prepare shared playback")
            };
            assert_eq!(
                (parameters.looping, parameters.loop_passes),
                (looping, passes)
            );
        }
    }
}

#[cfg(test)]
mod music_pcm_tests {
    use crate::{
        config::Names,
        native_audio::{AudioBindings, NativeAudioPolicy},
    };
    use crysirror_core::{
        path::{Mount, VfsPath},
        service::{AudioRequest, HostServices, PcmAudio},
        state::ClientId,
    };
    #[test]
    fn finite_native_music_counts_complete_real_pcm_and_unbounded_music_remains_active() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("assets")).unwrap();
        let project:crysirror_runtime::Project=serde_json::from_value(serde_json::json!({"schema":2,"name":"native repeat PCM","content_revision":"1","assets":"assets","canvas":[4,4],"clients":[{"name":"one","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let policy = NativeAudioPolicy::new(
            crate::native_services_bootstrap::NativePreferences::original_defaults().audio,
            AudioBindings {
                music: 0,
                sound_base: 16,
                voice_base: 32,
            },
        )
        .unwrap();
        let names = Names::parse(b"track 0", false).unwrap();
        for count in [0, 1, 2, 3] {
            let mut runtime = crysirror_runtime::Runtime::with_asset_root(
                project.clone(),
                dir.path(),
                &dir.path().join("state"),
                Default::default(),
            )
            .unwrap();
            let plan = policy
                .music(&[1, count, -9999, 0, 0], &names, false)
                .unwrap();
            let AudioRequest::PlayAsset { parameters, .. } = plan
                .asset_request(VfsPath::new(Mount::Asset, "track.wav").unwrap())
                .unwrap()
            else {
                unreachable!()
            };
            runtime
                .host
                .submit_audio(
                    ClientId(1),
                    AudioRequest::PlayPcm {
                        channel: 0,
                        source: PcmAudio::new(48000, vec![[0.25, 0.25]; 4]).unwrap(),
                        parameters,
                    },
                )
                .unwrap();
            let mut pcm = vec![[0.; 2]; 20];
            runtime.host.channel_audio.mixer.render(&mut pcm).unwrap();
            let active = if count == 0 { 20 } else { count as usize * 4 };
            assert!(
                pcm[..active]
                    .iter()
                    .all(|frame| frame[0] > 0. && frame[1] > 0.),
                "real positive PCM count={count}: {pcm:?}"
            );
            assert!(
                pcm[active..].iter().all(|frame| *frame == [0.; 2]),
                "must stop after original pass count {count}: {pcm:?}"
            );
            assert_eq!(
                runtime.host.audio_playing(ClientId(1), 0).unwrap(),
                count == 0
            );
            assert_eq!(runtime.host.now_ms(), 0);
        }
    }
}

/// Native resource/policy staging only; it can be consumed by the generic
/// original-restore transaction after every other native family is ready.
/// Keeping this opaque engine receipt cannot alter live playback itself.
pub(crate) struct PreparedSavedAudio {
    pub replacement: std::sync::Arc<dyn crysirror_core::service::AudioReplacementResource>,
    pub policy: NativeAudioPolicy,
    pub original: SavedAudioWork,
}
pub(crate) struct SavedAudioPreparation {
    client: crysirror_core::state::ClientId,
    original: SavedAudioWork,
    policy: Option<NativeAudioPolicy>,
    task: Option<crysirror_tasks::Task<Vec<crysirror_core::service::AudioRequest>>>,
    replacement: Option<crysirror_core::service::AudioReplacementJob>,
    result: Option<PreparedSavedAudio>,
    failed: Option<String>,
}
impl SavedAudioPreparation {
    pub(crate) fn begin(
        client: crysirror_core::state::ClientId,
        file: &NativeSaveFile,
        profile: &crate::native_save::LegacySaveProfile,
        current: &NativeAudioPolicy,
        music: &crate::config::Names,
        sound: &crate::config::Names,
        resolver: crate::audio_bridge::AudioResolver,
        pool: &crysirror_tasks::TaskPool,
    ) -> Result<Self> {
        ensure!(client.0 != 0, "NNN saved audio client");
        file.validate(profile)?;
        let original = file.audio_work()?;
        let mut policy = original.prepare_policy(current)?;
        let mut plans = Vec::new();
        // Actual53b630 uses direct zero-based PlayMusic with sentinel delta
        // -9999/default fade times. The callback helper is one-based, so only
        // this semantic index translation is shared; no NativeCall is forged.
        if original.music_number != -1 {
            let number = original
                .music_number
                .checked_add(1)
                .context("NNN saved music index overflow")?;
            let mut plan = policy.music(
                &[number, original.music_repeat_count, -9999, -1, -1],
                music,
                false,
            )?;
            plan.prepare_music_start(policy.bindings.music)?;
            policy = plan.next.clone();
            if let Some(source) = plan.source.clone() {
                plans.push((plan, source));
            }
        }
        // Complete RestartLoopSoundAndVoice iterates8 sound and4 voice rows.
        // Other saved voice rows are retained policy, not played accidentally.
        for (voice, count) in [(false, 8), (true, 4)] {
            for channel in 0..count {
                let enabled = if voice {
                    policy.voice_work[channel][0]
                } else {
                    policy.sound_work[channel][0]
                };
                if enabled == 0 {
                    continue;
                }
                let plan = policy.saved_plain_loop(channel, voice, sound)?;
                policy = plan.next.clone();
                if let Some(source) = plan.source.clone() {
                    plans.push((plan, source));
                }
            }
        }
        // All fixed record/name/policy validation precedes source admission.
        let task = pool.spawn(move |cancel| {
            let mut requests = Vec::with_capacity(plans.len());
            for (plan, source) in plans {
                cancel.check()?;
                let source = resolver(&source, &cancel)?;
                cancel.check()?;
                let request = match source {
                    crate::resource_bridge::ResolvedAudio::Playlist(source) => {
                        plan.playlist_request(source)?
                    }
                    crate::resource_bridge::ResolvedAudio::Asset(path) => {
                        plan.asset_request(path)?
                    }
                    crate::resource_bridge::ResolvedAudio::Encoded(source) => {
                        plan.encoded_request(source)?
                    }
                };
                requests.push(request);
            }
            cancel.check()?;
            Ok(requests)
        })?;
        Ok(Self {
            client,
            original,
            policy: Some(policy),
            task: Some(task),
            replacement: None,
            result: None,
            failed: None,
        })
    }
    pub(crate) fn poll(
        &mut self,
        host: &mut dyn crysirror_core::service::HostServices,
    ) -> Result<bool> {
        if let Some(error) = &self.failed {
            anyhow::bail!("NNN saved audio preparation: {error}");
        }
        if self.result.is_some() {
            return Ok(true);
        }
        let result = self.advance(host);
        if let Err(error) = &result {
            self.failed = Some(format!("{error:#}"));
        }
        result
    }
    fn advance(&mut self, host: &mut dyn crysirror_core::service::HostServices) -> Result<bool> {
        if let Some(task) = self.task.as_mut() {
            let Some(requests) = task.try_take()? else {
                return Ok(false);
            };
            self.task = None;
            self.replacement = Some(host.begin_audio_replacement(self.client, requests)?);
        }
        match self
            .replacement
            .as_mut()
            .context("NNN saved audio preparation phase")?
            .poll()?
        {
            crysirror_core::service::AudioReplacementState::Pending => Ok(false),
            crysirror_core::service::AudioReplacementState::Failed(error) => Err(error.into()),
            crysirror_core::service::AudioReplacementState::Ready(replacement) => {
                ensure!(
                    replacement.client() == self.client && replacement.info().client == self.client,
                    "NNN foreign saved audio replacement"
                );
                self.result = Some(PreparedSavedAudio {
                    replacement,
                    policy: self
                        .policy
                        .take()
                        .context("NNN saved audio policy consumed")?,
                    original: self.original.clone(),
                });
                self.replacement = None;
                Ok(true)
            }
        }
    }
    pub(crate) fn finish(mut self) -> Result<PreparedSavedAudio> {
        ensure!(self.failed.is_none(), "NNN failed saved audio preparation");
        self.result.take().context("NNN saved audio is not ready")
    }
}

#[cfg(test)]
mod preparation_tests {
    use super::*;
    use crate::{
        config::Names,
        native_audio::{AudioBindings, SourceKind},
        native_save::SaveBlock,
        resource_bridge::ResolvedAudio,
    };
    use crysirror_core::{
        service::{AudioParameters, AudioRequest, EncodedAudio, HostServices, PcmAudio},
        state::ClientId,
    };
    use std::{
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
        time::{Duration, Instant},
    };
    fn profile() -> crate::native_save::LegacySaveProfile {
        crate::native_save::LegacySaveProfile {
            header_name: b"authored".to_vec(),
            version: 1,
            extension_sizes: vec![],
            omake: false,
            cutin: false,
        }
    }
    fn file() -> NativeSaveFile {
        let mut info = vec![0; 32];
        info[..4].copy_from_slice(&8u32.to_le_bytes());
        info[4..8].copy_from_slice(&1u32.to_le_bytes());
        let mut mini = vec![0; 20];
        mini[..4].copy_from_slice(&1u32.to_le_bytes());
        mini[4..8].copy_from_slice(&1u32.to_le_bytes());
        let mut file = NativeSaveFile {
            blocks: vec![
                SaveBlock::new(0, b"authored", vec![]).unwrap(),
                SaveBlock::new(1, b"INFO", info).unwrap(),
                SaveBlock::new(2, b"GAMESTATUS", vec![0; 7056]).unwrap(),
                SaveBlock::new(3, b"MINICG", mini).unwrap(),
                SaveBlock::new(4, b"GAMEVAR", vec![0; 4096]).unwrap(),
                SaveBlock::new(5, b"EFFECT", vec![0; 16384]).unwrap(),
                SaveBlock::new(6, b"EFFECTFILENAME", vec![0; 4096]).unwrap(),
                SaveBlock::new(7, b"MESSAGE", vec![0; 16384]).unwrap(),
            ],
        };
        let mut audio = file.audio_work().unwrap();
        audio.music_number = 0;
        audio.music_repeat_count = 2;
        audio.sound[7][0] = 1;
        audio.sound[7][11..].copy_from_slice(&[71, 72, 73, 74, 75]);
        audio.voice[0][0] = 1;
        audio.voice_names[0][..8].copy_from_slice(b"AB_voice");
        audio.voice_names[0][63] = 207;
        // Original restart loops only voice0..3. The other record remains
        // opaque retained policy and must not trigger a read of this name.
        audio.voice[4][0] = 1;
        audio.voice_names[4].fill(255);
        file.replace_audio_work(&audio).unwrap();
        file
    }
    fn policy() -> NativeAudioPolicy {
        NativeAudioPolicy::new(
            crate::native_services_bootstrap::NativePreferences::original_defaults().audio,
            AudioBindings {
                music: 0,
                sound_base: 16,
                voice_base: 32,
            },
        )
        .unwrap()
    }
    fn runtime(root: &std::path::Path) -> crysirror_runtime::Runtime {
        std::fs::create_dir_all(root.join("assets")).unwrap();
        let project:crysirror_runtime::Project=serde_json::from_value(serde_json::json!({"schema":2,"name":"saved audio preparation","content_revision":"1","assets":"assets","canvas":[4,4],"clients":[{"name":"one","adapter":"crysirror-script-v1","game":"authored"},{"name":"two","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let mut runtime = crysirror_runtime::Runtime::with_asset_root(
            project,
            root,
            &root.join("state"),
            Default::default(),
        )
        .unwrap();
        for (client, channel) in [(ClientId(1), 99), (ClientId(2), 16)] {
            runtime
                .host
                .submit_audio(
                    client,
                    AudioRequest::PlayPcm {
                        channel,
                        source: PcmAudio::new(48000, vec![[0.1, 0.2]; 100]).unwrap(),
                        parameters: AudioParameters {
                            looping: true,
                            ..Default::default()
                        },
                    },
                )
                .unwrap();
        }
        runtime
    }
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
    fn actual_engine_prepares_saved_music_sound_and_voice_without_mutating_live_playback() {
        let root = tempfile::tempdir().unwrap();
        let mut runtime = runtime(root.path());
        let owner = std::thread::current().id();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let worker_seen = seen.clone();
        let resolver = Arc::new(
            move |source: &crate::native_audio::NativeSource,
                  cancel: &crysirror_tasks::Cancellation| {
                cancel.check()?;
                assert_ne!(std::thread::current().id(), owner);
                worker_seen
                    .lock()
                    .unwrap()
                    .push((source.kind, source.name.clone()));
                Ok(ResolvedAudio::Encoded(EncodedAudio {
                    bytes: wave(),
                    frame_limit: None,
                }))
            },
        );
        let file = file();
        let current = policy();
        let names = Names::parse(b"track 0", false).unwrap();
        let original_bridge = crate::audio_bridge::AudioBridge::new(
            ClientId(1),
            current.clone(),
            names.clone(),
            names.clone(),
            resolver,
        )
        .unwrap();
        let seed = original_bridge.restore_seed().unwrap();
        let mut pending = seed.prepare(&file, &profile()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !pending.poll(&mut runtime.host).unwrap() {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(
            pending.poll(&mut runtime.host).unwrap(),
            "ready is retained"
        );
        let ready = pending.finish().unwrap();
        assert_eq!(ready.replacement.info().channels, 3);
        assert_eq!(ready.original, file.audio_work().unwrap());
        assert_eq!(ready.policy.sound_work[7][11..], [71, 72, 73, 74, 75]);
        assert_eq!(ready.policy.voice_names[0], b"AB_voice");
        assert_eq!(ready.policy.voice_names[4], [255; 64]);
        assert_eq!(
            *seen.lock().unwrap(),
            vec![
                (SourceKind::Music, b"track".to_vec()),
                (SourceKind::Sound, b"track".to_vec()),
                (SourceKind::Voice, b"AB_voice".to_vec())
            ]
        );
        assert_eq!(current.music_number, None);
        assert_eq!(runtime.host.now_ms(), 0);
        assert!(runtime.host.audio_playing(ClientId(1), 99).unwrap());
        assert!(runtime.host.audio_playing(ClientId(2), 16).unwrap());
        for channel in [0, 23, 32] {
            assert!(!runtime.host.audio_playing(ClientId(1), channel).unwrap());
        }
        let (detached, replacement) = seed.finish(ready).unwrap();
        assert_eq!(
            detached.original_audio_work().unwrap(),
            file.audio_work().unwrap(),
            "native export retains work and complete raw name tails after import"
        );
        assert_eq!(
            original_bridge.policy.music_number, None,
            "finishing detached component cannot mutate original bridge"
        );
        assert_eq!(detached.api_calls, 0);
        drop(detached);
        drop(replacement);
        assert!(
            runtime.host.audio_playing(ClientId(1), 99).unwrap(),
            "dropping prepared replacement never clears old channels"
        );
    }
    #[test]
    fn late_source_failure_is_retained_and_invalid_native_records_do_not_admit_io() {
        let root = tempfile::tempdir().unwrap();
        let mut runtime = runtime(root.path());
        let calls = Arc::new(AtomicUsize::new(0));
        let reads = calls.clone();
        let resolver = Arc::new(
            move |_: &crate::native_audio::NativeSource, _: &crysirror_tasks::Cancellation| {
                if reads.fetch_add(1, Ordering::SeqCst) == 1 {
                    anyhow::bail!("authored later source corruption");
                }
                Ok(ResolvedAudio::Encoded(EncodedAudio {
                    bytes: wave(),
                    frame_limit: None,
                }))
            },
        );
        let file = file();
        let current = policy();
        let names = Names::parse(b"track 0", false).unwrap();
        let pool = crysirror_tasks::TaskPool::new("saved-audio-test-fail", 1, 1).unwrap();
        let mut bad = file.clone();
        let mut audio = bad.audio_work().unwrap();
        audio.voice_names[0].fill(65);
        bad.replace_audio_work(&audio).unwrap();
        assert!(
            SavedAudioPreparation::begin(
                ClientId(1),
                &bad,
                &profile(),
                &current,
                &names,
                &names,
                resolver.clone(),
                &pool
            )
            .is_err()
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        let mut pending = SavedAudioPreparation::begin(
            ClientId(1),
            &file,
            &profile(),
            &current,
            &names,
            &names,
            resolver,
            &pool,
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match pending.poll(&mut runtime.host) {
                Ok(false) => {}
                Err(_) => break,
                other => panic!("must retain original failure: {other:?}"),
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(pending.poll(&mut runtime.host).is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert!(runtime.host.audio_playing(ClientId(1), 99).unwrap());
        assert!(runtime.host.audio_playing(ClientId(2), 16).unwrap());
        assert!(pending.finish().is_err());
        assert_eq!(current.music_number, None);
        assert_eq!(runtime.host.now_ms(), 0);
    }
}

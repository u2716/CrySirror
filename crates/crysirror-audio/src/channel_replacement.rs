// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Detached channel preparation and a single owner commit for native restoration.
//! A commit token exclusively borrows the destination until install or rollback.
use crate::{
    buffered::{BufferedSource, StreamBufferSpec},
    channels::{ChannelAudio, PreparedChannel, asset, prepare, source},
    decoder::open_pcm,
    mixer::ClientVoiceReplacement,
};
use anyhow::{Context, Result, ensure};
use crysirror_core::service::{AudioParameters, AudioRequest, ClientStorage};
use crysirror_tasks::{Task, TaskPool};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
struct Charge {
    budget: Arc<AtomicUsize>,
    bytes: usize,
}
impl Drop for Charge {
    fn drop(&mut self) {
        self.budget.fetch_sub(self.bytes, Ordering::AcqRel);
    }
}
struct Input {
    channel: u32,
    request: AudioRequest,
    resource: Option<Arc<crate::resources::PreparedSound>>,
}
/// Owns only admitted work; dropping it never changes existing channels.
pub struct ChannelReplacementJob {
    mixer: u64,
    epoch: u64,
    owner: u32,
    revision: u64,
    task: Option<Task<Vec<(u32, PreparedChannel)>>>,
}
/// Ready sources/decoders are private engine-owned values, never live handles.
pub struct PreparedChannelReplacement {
    mixer: u64,
    epoch: u64,
    owner: u32,
    revision: u64,
    channels: Vec<(u32, PreparedChannel)>,
}
impl ChannelReplacementJob {
    pub fn poll(&mut self) -> Result<Option<PreparedChannelReplacement>> {
        let Some(channels) = self
            .task
            .as_mut()
            .context("audio replacement job consumed")?
            .try_take()?
        else {
            return Ok(None);
        };
        self.task = None;
        Ok(Some(PreparedChannelReplacement {
            mixer: self.mixer,
            epoch: self.epoch,
            owner: self.owner,
            revision: self.revision,
            channels,
        }))
    }
}
/// All fallible checks have completed. Exclusive borrowing prevents a new frame
/// or control operation between validation and the infallible install.
pub struct ChannelReplacementCommit<'a> {
    audio: &'a mut ChannelAudio,
    owner: u32,
    revision: u64,
    voices: ClientVoiceReplacement,
    channels: BTreeMap<(u32, u32), crate::mixer::VoiceHandle>,
}
impl ChannelReplacementCommit<'_> {
    pub fn install(self) {
        let Self {
            audio,
            owner,
            revision,
            voices,
            channels,
        } = self;
        audio.jobs.clear_owner(owner);
        audio.seeks.clear_owner(owner);
        audio.releases.clear_owner(owner);
        audio.sequences.clear_client(owner);
        voices.install(&mut audio.mixer);
        audio.voices.retain(|(client, _), _| *client != owner);
        audio.voices.extend(channels);
        audio.clients.insert(owner);
        audio.replacement_versions.insert(owner, revision);
        // Existing AudioResource catalog and group/bus/listener configuration
        // remain valid. Only this client's voices and play/control intents swap.
    }
}
fn recipe(request: &AudioRequest) -> Result<(u32, &AudioParameters)> {
    match request {
        AudioRequest::Play {
            channel,
            parameters,
            ..
        }
        | AudioRequest::PlayResident {channel,parameters,..}
        | AudioRequest::PlayAsset {
            channel,
            parameters,
            ..
        }
        | AudioRequest::PlayResource {
            channel,
            parameters,
            ..
        }
        | AudioRequest::PlayPcm {
            channel,
            parameters,
            ..
        } => Ok((*channel, parameters)),
        _ => anyhow::bail!("audio replacement accepts only pure play recipes"),
    }
}
impl ChannelAudio {
    /// Capture resources on the owner, then open/decode/seek/prewarm exclusively
    /// on the worker. This does not stop playback or register any live handle.
    pub fn prepare_client_replacement(
        &self,
        owner: u32,
        requests: Vec<AudioRequest>,
        storage: Arc<dyn ClientStorage>,
        pool: TaskPool,
    ) -> Result<ChannelReplacementJob> {
        ensure!(
            owner > 0 && requests.len() <= 64,
            "audio replacement owner/count limit"
        );
        ensure!(
            self.replacement_epoch < u64::MAX,
            "audio replacement epoch exhausted"
        );
        let revision = self.replacement_versions.get(&owner).copied().unwrap_or(0);
        ensure!(
            revision < u64::MAX,
            "audio replacement client revision exhausted"
        );
        let mut channels = BTreeSet::new();
        let mut bytes = 0usize;
        let mut inputs = Vec::new();
        for request in requests {
            ensure!(!matches!(&request,AudioRequest::PlayResident{..}),"resident client replacement requires a prepared PCM recipe");
            let (channel, p) = recipe(&request)?;
            ensure!(channels.insert(channel), "duplicate replacement channel");
            ensure!(
                p.restart && p.replace_fade_ms == 0.,
                "audio replacement cannot depend on old playback"
            );
            let input_bytes = match &request {
                AudioRequest::Play { source, intro, .. } | AudioRequest::PlayResident {source,intro,..} => {
                    ensure!(
                        source.bytes.len() <= 64 * 1024 * 1024
                            && intro
                                .as_ref()
                                .is_none_or(|s| s.bytes.len() <= 64 * 1024 * 1024),
                        "replacement encoded-input limit"
                    );
                    source.bytes.len() + intro.as_ref().map_or(0, |s| s.bytes.len())
                }
                AudioRequest::PlayPcm { source, .. } => {
                    let n = source
                        .frame_count()
                        .checked_mul(8)
                        .context("replacement PCM byte overflow")?;
                    ensure!(n <= 64 * 1024 * 1024, "replacement PCM input limit");
                    n
                }
                _ => 0,
            };
            bytes = bytes
                .checked_add(input_bytes + 4 * 1024 * 1024)
                .context("replacement reservation overflow")?;
            let resource = if let AudioRequest::PlayResource { resource, .. } = &request {
                Some(self.resources.capture(owner, *resource)?)
            } else {
                None
            };
            inputs.push(Input {
                channel,
                request,
                resource,
            });
        }
        let budget = self.jobs.budget.clone();
        ensure!(
            budget
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| n
                    .checked_add(bytes)
                    .filter(|n| *n <= 512 * 1024 * 1024))
                .is_ok(),
            "audio replacement shared memory reservation exceeded"
        );
        let charge = Arc::new(Charge { budget, bytes });
        let realtime = self.jobs.nonblocking;
        let workers = pool.clone();
        let task = pool.spawn(move |cancel| {
            let mut out = Vec::with_capacity(inputs.len());
            for input in inputs {
                cancel.check()?;
                let (tail, intro, p) = match input.request {
                    AudioRequest::Play {
                        source: encoded,
                        intro,
                        parameters: p,
                        ..
                    } => {
                        let gain = p.decode_gain;
                        (
                            source(encoded, gain)?,
                            intro.map(|s| source(s, gain)).transpose()?,
                            p,
                        )
                    }
                    AudioRequest::PlayAsset {
                        source: encoded,
                        intro,
                        parameters: p,
                        ..
                    } => {
                        let gain = p.decode_gain;
                        (
                            asset(encoded, gain, storage.as_ref())?,
                            intro
                                .map(|s| asset(s, gain, storage.as_ref()))
                                .transpose()?,
                            p,
                        )
                    }
                    AudioRequest::PlayResource { parameters: p, .. } => {
                        let opened = input
                            .resource
                            .context("replacement resource lease missing")?
                            .open()?;
                        (
                            crate::channels::wrap_source(opened, None, p.decode_gain)?,
                            None,
                            p,
                        )
                    }
                    AudioRequest::PlayPcm {
                        source: pcm,
                        parameters: p,
                        ..
                    } => (
                        crate::channels::wrap_source(open_pcm(pcm), None, p.decode_gain)?,
                        None,
                        p,
                    ),
                    _ => unreachable!(),
                };
                let mut prepared = prepare(tail, intro, p)?;
                if realtime {
                    let mut source = BufferedSource::prepare_at_cancellable(
                        prepared.source,
                        workers.clone(),
                        StreamBufferSpec::default(),
                        prepared.spec.loop_points.map(|p| p.start),
                        prepared.start,
                        &cancel,
                    )?;
                    source.retain_storage(charge.clone())?;
                    prepared.source = Box::new(source);
                } else {
                    prepared.source = Box::new(ChargedSource {
                        inner: prepared.source,
                        _charge: charge.clone(),
                    });
                }
                out.push((input.channel, prepared));
            }
            cancel.check()?;
            Ok(out)
        })?;
        Ok(ChannelReplacementJob {
            mixer: self.mixer.identity(),
            epoch: self.replacement_epoch,
            owner,
            revision,
            task: Some(task),
        })
    }
    pub fn stage_client_replacement(
        &mut self,
        owner: u32,
        prepared: PreparedChannelReplacement,
    ) -> Result<ChannelReplacementCommit<'_>> {
        ensure!(
            prepared.owner == owner
                && prepared.mixer == self.mixer.identity()
                && prepared.epoch == self.replacement_epoch
                && prepared.revision == self.replacement_versions.get(&owner).copied().unwrap_or(0),
            "foreign or obsolete prepared audio replacement"
        );
        let revision = prepared
            .revision
            .checked_add(1)
            .context("audio replacement client revision exhausted")?;
        let (ids, channels): (Vec<_>, Vec<_>) = prepared.channels.into_iter().unzip();
        let voices = self.mixer.prepare_client_replacement(owner, channels)?;
        let channels = ids
            .into_iter()
            .zip(voices.handles())
            .map(|(id, h)| ((owner, id), h))
            .collect();
        Ok(ChannelReplacementCommit {
            audio: self,
            owner,
            revision,
            voices,
            channels,
        })
    }
}
// Retain accounting until the final source lease drops, including offline
// playback; this forwards the source contract without introducing a decoder.
struct ChargedSource {
    inner: Box<dyn crate::decoder::AudioSource>,
    _charge: Arc<Charge>,
}
impl crate::decoder::AudioSource for ChargedSource {
    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }
    fn frame_count(&self) -> Option<u64> {
        self.inner.frame_count()
    }
    fn read(&mut self, out: &mut [crate::decoder::StereoFrame]) -> Result<usize> {
        self.inner.read(out)
    }
    fn seek(&mut self, frame: u64) -> Result<()> {
        self.inner.seek(frame)
    }
    fn poll_read(
        &mut self,
        out: &mut [crate::decoder::StereoFrame],
    ) -> Result<crate::decoder::SourceRead> {
        self.inner.poll_read(out)
    }
    fn poll_seek(&mut self, frame: u64) -> Result<bool> {
        self.inner.poll_seek(frame)
    }
    fn service(&mut self) -> Result<()> {
        self.inner.service()
    }
    fn read_position(&self) -> Option<u64> {
        self.inner.read_position()
    }
    fn is_nonblocking(&self) -> bool {
        self.inner.is_nonblocking()
    }
    fn has_replay_gain(&self) -> bool {
        self.inner.has_replay_gain()
    }
    fn channel_count(&self) -> Option<u16> {
        self.inner.channel_count()
    }
    fn buffer_status(&self) -> Option<crate::decoder::SourceBufferStatus> {
        self.inner.buffer_status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mixer::{Mixer, MixerConfig};
    use crysirror_core::{
        path::{Mount, VfsPath},
        service::PcmAudio,
    };
    use std::{
        thread,
        time::{Duration, Instant},
    };
    struct Storage;
    impl ClientStorage for Storage {
        fn read(&self, _: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
            Err(crysirror_core::Error::NotFound)
        }
        fn write_atomic(&self, _: &VfsPath, _: &[u8]) -> crysirror_core::Result<()> {
            Err(crysirror_core::Error::ReadOnly)
        }
        fn remove(&self, _: &VfsPath) -> crysirror_core::Result<()> {
            Err(crysirror_core::Error::ReadOnly)
        }
        fn list(&self, _: Mount, _: &str) -> crysirror_core::Result<Vec<String>> {
            Ok(vec![])
        }
    }
    fn play(channel: u32, gain: f32) -> AudioRequest {
        AudioRequest::PlayPcm {
            channel,
            source: PcmAudio::new(48000, vec![[gain; 2]; 4096]).unwrap(),
            parameters: Default::default(),
        }
    }
    fn prepare(
        a: &ChannelAudio,
        owner: u32,
        requests: Vec<AudioRequest>,
    ) -> PreparedChannelReplacement {
        let p = TaskPool::new("audio-restore", 1, 4).unwrap();
        let mut job = a
            .prepare_client_replacement(owner, requests, Arc::new(Storage), p)
            .unwrap();
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(v) = job.poll().unwrap() {
                return v;
            }
            assert!(Instant::now() < end);
            thread::yield_now();
        }
    }
    fn setup() -> ChannelAudio {
        let mut a = ChannelAudio::default();
        a.submit(1, play(7, 0.125)).unwrap();
        a.submit(2, play(7, 0.25)).unwrap();
        a
    }
    #[test]
    fn detached_prepare_and_dropped_preflight_preserve_live_state_then_one_install_replaces_only_owner()
     {
        let mut a = setup();
        let own = a.voices[&(1, 7)];
        let peer = a.voices[&(2, 7)];
        let ready = prepare(&a, 1, vec![play(10, 0.5), play(11, 0.125)]);
        assert!(a.mixer.is_playing(1, own));
        assert!(!a.is_playing(1, 10));
        let before = a.mixer.playback_info(2, peer).unwrap();
        drop(a.stage_client_replacement(1, ready).unwrap());
        assert!(a.mixer.is_playing(1, own));
        assert!(!a.is_playing(1, 10));
        let ready = prepare(&a, 1, vec![play(10, 0.5), play(11, 0.125)]);
        a.stage_client_replacement(1, ready).unwrap().install();
        assert!(!a.mixer.is_playing(1, own));
        assert!(!a.is_playing(1, 7));
        assert!(a.is_playing(1, 10) && a.is_playing(1, 11));
        assert_eq!(a.voices[&(2, 7)], peer);
        assert_eq!(
            a.mixer.playback_info(2, peer).unwrap().position_frames,
            before.position_frames
        );
        a.render(1).unwrap();
        assert!(a.take_output().iter().all(|f| *f == [0.875; 2]));
        let empty = prepare(&a, 1, vec![]);
        a.stage_client_replacement(1, empty).unwrap().install();
        a.render(1).unwrap();
        assert!(a.take_output().iter().all(|f| *f == [0.25; 2]));
    }
    #[test]
    fn later_invalid_group_or_peer_capacity_cannot_partially_publish_or_steal() {
        let mut a = setup();
        let old = a.voices.clone();
        let mut bad = play(11, 0.5);
        if let AudioRequest::PlayPcm { parameters, .. } = &mut bad {
            parameters.mix_group = Some("missing".into());
        }
        let ready = prepare(&a, 1, vec![play(10, 0.5), bad]);
        assert!(a.stage_client_replacement(1, ready).is_err());
        assert_eq!(a.voices, old);
        a.mixer = Mixer::new(MixerConfig {
            max_voices: 2,
            ..Default::default()
        })
        .unwrap();
        a.voices.clear();
        a.submit(1, play(7, 0.125)).unwrap();
        a.submit(2, play(7, 0.25)).unwrap();
        let old = a.voices.clone();
        let ready = prepare(&a, 1, vec![play(10, 0.5), play(11, 0.125)]);
        assert!(a.stage_client_replacement(1, ready).is_err());
        assert_eq!(a.voices, old);
        a.render(1).unwrap();
        assert!(a.take_output().iter().all(|f| *f == [0.375; 2]));
    }
    #[test]
    fn wrong_owner_mixer_and_reset_epoch_reject_opaque_ready_values_without_live_changes() {
        let mut a = setup();
        let b = setup();
        let own = a.voices[&(1, 7)];
        let foreign = prepare(&b, 1, vec![play(10, 0.5)]);
        assert!(a.stage_client_replacement(1, foreign).is_err());
        let wrongowner = prepare(&a, 1, vec![play(10, 0.5)]);
        assert!(a.stage_client_replacement(2, wrongowner).is_err());
        assert!(a.mixer.is_playing(1, own));
        let obsolete = prepare(&a, 1, vec![play(10, 0.5)]);
        a.reset();
        a.submit(1, play(7, 0.125)).unwrap();
        let current = a.voices[&(1, 7)];
        assert!(a.stage_client_replacement(1, obsolete).is_err());
        assert!(a.mixer.is_playing(1, current));
    }
    #[test]
    fn failed_resource_worker_and_invalid_recipe_do_not_stop_live_audio() {
        let mut a = setup();
        let old = a.voices.clone();
        let p = TaskPool::new("failed-restore", 1, 4).unwrap();
        let missing = AudioRequest::PlayAsset {
            channel: 11,
            source: crysirror_core::service::AudioAsset {
                path: VfsPath::new(Mount::Asset, "missing.wav").unwrap(),
                frame_limit: None,
            },
            intro: None,
            parameters: Default::default(),
        };
        let mut job = a
            .prepare_client_replacement(
                1,
                vec![play(10, 0.5), missing],
                Arc::new(Storage),
                p.clone(),
            )
            .unwrap();
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            match job.poll() {
                Err(_) => break,
                Ok(None) => {
                    assert!(Instant::now() < end);
                    thread::yield_now()
                }
                Ok(Some(_)) => panic!("missing source cannot be ready"),
            }
        }
        drop(job);
        assert_eq!(a.voices, old);
        assert!(
            a.prepare_client_replacement(
                1,
                vec![AudioRequest::Stop {
                    channel: 7,
                    fade_ms: 0.
                }],
                Arc::new(Storage),
                p.clone()
            )
            .is_err()
        );
        assert!(
            a.prepare_client_replacement(
                1,
                vec![play(10, 0.5), play(10, 0.25)],
                Arc::new(Storage),
                p
            )
            .is_err()
        );
        a.render(1).unwrap();
        assert!(a.take_output().iter().all(|f| *f == [0.375; 2]));
    }
    #[test]
    fn replacement_preserves_peer_seek_receipts_and_reverb_dsp_while_retiring_owner_intents() {
        use crate::reverb::{PreparedReverb, ReverbSettings};
        let mut a = setup();
        a.mixer
            .set_reverb(
                2,
                Some(PreparedReverb::prepare(48000, ReverbSettings::default()).unwrap()),
            )
            .unwrap();
        let peer = a.begin_channel_seek(2, 7, 20).unwrap();
        let own = a.begin_channel_seek(1, 7, 30).unwrap();
        let ready = prepare(&a, 1, vec![]);
        a.stage_client_replacement(1, ready).unwrap().install();
        assert!(a.channel_seek_preparation(1, own).is_err());
        assert_eq!(
            a.channel_seek_preparation(2, peer).unwrap().phase,
            crysirror_core::service::AudioPreparePhase::Ready
        );
        assert!(a.mixer.reverb(2).is_some());
        a.mixer
            .set_reverb(
                1,
                Some(PreparedReverb::prepare(48000, ReverbSettings::default()).unwrap()),
            )
            .unwrap();
        let ready = prepare(&a, 1, vec![play(10, 0.5)]);
        assert!(a.stage_client_replacement(1, ready).is_err());
        assert!(a.mixer.reverb(1).is_some());
    }
    #[test]
    fn gain_loop_pause_pan_and_source_offset_publish_together_from_prepared_recipe() {
        let mut a = setup();
        let mut recipe = play(10, 0.5);
        if let AudioRequest::PlayPcm { parameters, .. } = &mut recipe {
            *parameters = AudioParameters {
                gain: 0.5,
                start_frame: Some(20),
                paused: true,
                looping: true,
                loop_passes: Some(3),
                swap_channels: true,
                pan: 0.,
                ..Default::default()
            };
        }
        let ready = prepare(&a, 1, vec![recipe]);
        a.stage_client_replacement(1, ready).unwrap().install();
        let info = a.playback_info(1, 10).unwrap().unwrap();
        assert_eq!(info.position_frames, 20);
        assert!(info.paused && info.swap_channels);
        assert_eq!(info.loop_passes, Some(3));
        assert_eq!(info.gain, 0.5);
        a.render(1).unwrap();
        assert!(a.take_output().iter().all(|f| *f == [0.25; 2]));
        a.submit(
            1,
            AudioRequest::Pause {
                channel: 10,
                paused: false,
                fade_ms: 0.,
            },
        )
        .unwrap();
        a.render(1).unwrap();
        let out = a.take_output();
        assert!((out[0][0] - (0.25 + 0.25 * std::f32::consts::SQRT_2)).abs() < 1e-6);
        assert_eq!(out[0][1], 0.25);
    }
    #[test]
    fn peer_reverb_tail_remains_bit_exact_across_another_clients_install() {
        use crate::reverb::{PreparedReverb, ReverbSettings};
        fn room() -> ChannelAudio {
            let mut a = ChannelAudio::default();
            a.mixer
                .set_reverb(
                    2,
                    Some(PreparedReverb::prepare(48000, ReverbSettings::default()).unwrap()),
                )
                .unwrap();
            a.submit(1, play(7, 0.)).unwrap();
            a.submit(
                2,
                AudioRequest::PlayPcm {
                    channel: 7,
                    source: PcmAudio::new(48000, vec![[0.5; 2]]).unwrap(),
                    parameters: Default::default(),
                },
            )
            .unwrap();
            a
        }
        let mut a = room();
        let mut reference = room();
        a.render(10).unwrap();
        reference.render(10).unwrap();
        assert_eq!(a.take_output(), reference.take_output());
        let ready = prepare(&a, 1, vec![]);
        a.stage_client_replacement(1, ready).unwrap().install();
        reference
            .submit(
                1,
                AudioRequest::Stop {
                    channel: 7,
                    fade_ms: 0.,
                },
            )
            .unwrap();
        a.render(100).unwrap();
        reference.render(100).unwrap();
        let actual = a.take_output();
        assert_eq!(actual, reference.take_output());
        assert!(actual.iter().flatten().any(|v| v.abs() > 1e-8));
    }
    #[test]
    fn same_client_commit_obsoletes_older_preparations_but_peer_commit_does_not() {
        let mut a = setup();
        let first = prepare(&a, 1, vec![play(10, 0.5)]);
        let stale = prepare(&a, 1, vec![play(11, 0.125)]);
        let peer = prepare(&a, 2, vec![play(20, 0.25)]);
        a.stage_client_replacement(2, peer).unwrap().install();
        a.stage_client_replacement(1, first).unwrap().install();
        assert!(a.stage_client_replacement(1, stale).is_err());
        assert!(a.is_playing(1, 10));
        assert!(!a.is_playing(1, 11));
        assert!(a.is_playing(2, 20));
    }
    #[test]
    fn actual_asset_open_decode_seek_and_prewarm_run_on_worker_and_budget_follows_source_lease() {
        use std::sync::Mutex;
        struct Recorded(Arc<Mutex<Option<thread::ThreadId>>>);
        impl ClientStorage for Recorded {
            fn read(&self, _: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
                *self.0.lock().unwrap() = Some(thread::current().id());
                let mut wav = b"RIFF".to_vec();
                wav.extend(1060u32.to_le_bytes());
                wav.extend(b"WAVEfmt ");
                wav.extend(16u32.to_le_bytes());
                wav.extend(1u16.to_le_bytes());
                wav.extend(1u16.to_le_bytes());
                wav.extend(48000u32.to_le_bytes());
                wav.extend(96000u32.to_le_bytes());
                wav.extend(2u16.to_le_bytes());
                wav.extend(16u16.to_le_bytes());
                wav.extend(b"data");
                wav.extend(1024u32.to_le_bytes());
                for _ in 0..512 {
                    wav.extend(8192i16.to_le_bytes());
                }
                Ok(wav)
            }
            fn write_atomic(&self, _: &VfsPath, _: &[u8]) -> crysirror_core::Result<()> {
                Err(crysirror_core::Error::ReadOnly)
            }
            fn remove(&self, _: &VfsPath) -> crysirror_core::Result<()> {
                Err(crysirror_core::Error::ReadOnly)
            }
            fn list(&self, _: Mount, _: &str) -> crysirror_core::Result<Vec<String>> {
                Ok(vec![])
            }
        }
        let pool = TaskPool::new("replacement-codec", 1, 8).unwrap();
        let mut a = setup();
        a.configure_preparation(pool.clone(), true);
        let observed = Arc::new(Mutex::new(None));
        let budget = a.jobs.budget.clone();
        let request = AudioRequest::PlayAsset {
            channel: 10,
            source: crysirror_core::service::AudioAsset {
                path: VfsPath::new(Mount::Asset, "tone.wav").unwrap(),
                frame_limit: None,
            },
            intro: None,
            parameters: AudioParameters {
                start_frame: Some(100),
                ..Default::default()
            },
        };
        let mut job = a
            .prepare_client_replacement(
                1,
                vec![request],
                Arc::new(Recorded(observed.clone())),
                pool.clone(),
            )
            .unwrap();
        assert!(budget.load(Ordering::Acquire) > 0);
        let end = Instant::now() + Duration::from_secs(5);
        let ready = loop {
            if let Some(ready) = job.poll().unwrap() {
                break ready;
            }
            assert!(Instant::now() < end);
            thread::yield_now();
        };
        assert_ne!(observed.lock().unwrap().unwrap(), thread::current().id());
        assert!(a.is_playing(1, 7));
        a.stage_client_replacement(1, ready).unwrap().install();
        assert_eq!(
            a.playback_info(1, 10).unwrap().unwrap().position_frames,
            100
        );
        a.render(1).unwrap();
        assert!(a.take_output().iter().all(|f| *f == [0.5; 2]));
        let empty = prepare(&a, 1, vec![]);
        a.stage_client_replacement(1, empty).unwrap().install();
        let end = Instant::now() + Duration::from_secs(5);
        while budget.load(Ordering::Acquire) > 0 {
            assert!(Instant::now() < end);
            thread::yield_now();
        }
    }
    #[test]
    fn restore_clears_retained_eof_and_parked_owner_sources_and_seek_receipts_but_preserves_peers() {
        let mut a=ChannelAudio::default();let mut reference=ChannelAudio::default();
        let mut retained=PcmAudio::new(48000,vec![[0.125;2];48]).unwrap();
        let lease=Arc::new(());let weak=Arc::downgrade(&lease);retained.retain_storage(lease.clone());drop(lease);
        a.submit(1,AudioRequest::PlayPcm{channel:1,source:retained,parameters:AudioParameters{retain_on_finish:true,..Default::default()}}).unwrap();
        a.submit(1,AudioRequest::PlayPcm{channel:2,source:PcmAudio::new(48000,vec![[0.5;2];1024]).unwrap(),parameters:AudioParameters{retain_on_finish:true,transport_active:false,..Default::default()}}).unwrap();
        for output in [&mut a,&mut reference] {
            output.submit(2,play(7,0.25)).unwrap();
            output.submit(2,AudioRequest::PlayPcm{channel:8,source:PcmAudio::new(48000,vec![[0.0625;2];48]).unwrap(),parameters:AudioParameters{retain_on_finish:true,..Default::default()}}).unwrap();
            output.render(2).unwrap();output.take_output();
        }
        let eof=a.playback_info(1,1).unwrap().unwrap();assert!(eof.retain_on_finish&&!eof.transport_active&&!eof.source_playing);assert!(weak.upgrade().is_some());
        let own_seek=a.begin_channel_seek(1,1,0).unwrap();let peer_seek=a.begin_channel_seek(2,8,0).unwrap();
        let peer=a.voices[&(2,7)];let held_peer=a.voices[&(2,8)];let held_before=a.mixer.playback_info(2,held_peer).unwrap();
        let ready=prepare(&a,1,vec![]);a.stage_client_replacement(1,ready).unwrap().install();
        assert!(a.playback_info(1,1).unwrap().is_none()&&a.playback_info(1,2).unwrap().is_none());assert!(a.channel_seek_preparation(1,own_seek).is_err());assert!(a.channel_seek_preparation(2,peer_seek).is_ok());assert_eq!(a.voices[&(2,7)],peer);assert_eq!(a.voices[&(2,8)],held_peer);assert_eq!(a.mixer.playback_info(2,held_peer).unwrap(),held_before);assert!(weak.upgrade().is_none());
        a.render(2).unwrap();reference.render(2).unwrap();assert_eq!(a.take_output(),reference.take_output());
    }

}

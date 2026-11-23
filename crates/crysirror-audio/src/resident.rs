//! Complete immutable PCM preparation on shared workers, without an SFX-only
//! clip-size limit or decoder work inside the mixer callback.
use crate::channel_async::Charge;
use crate::channels::{wrap_source, ChannelAudio};
use crate::decoder::{self, AudioSource, SourceRead, StereoFrame};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    service::{
        AssetReader, PcmAudio, ResidentAudioJob, ResidentAudioProcess, ResidentAudioState,
        MAX_PCM_FRAMES,
    },
    state::ClientId,
    Error,
};
use crysirror_tasks::{Cancellation, Task, TaskPool};
use std::sync::Arc;
pub(crate) fn decode(
    input: Box<dyn AudioSource>,
    cancel: &Cancellation,
    storage: Option<Arc<dyn Send + Sync>>,
) -> Result<Box<dyn AudioSource>> {
    let replay_gain = input.has_replay_gain();
    let channels = input.channel_count();
    let mut pcm = decode_pcm(input, cancel, None)?;
    if let Some(storage) = storage {
        pcm.retain_storage(storage);
    }
    Ok(Box::new(Resident {
        inner: decoder::open_pcm(pcm),
        replay_gain,
        channels,
    }))
}

fn decode_pcm(
    mut input: Box<dyn AudioSource>,
    cancel: &Cancellation,
    capacity: Option<usize>,
) -> Result<PcmAudio> {
    let rate = input.sample_rate();
    if let Some(frames) = input.frame_count() {
        ensure!(frames <= MAX_PCM_FRAMES as u64, "resident PCM frame budget");
    }
    let mut samples = Vec::new();
    if let Some(frames) = capacity {
        samples.try_reserve_exact(frames)?;
    }
    let mut block = [[0.; 2]; 4096];
    loop {
        cancel.check()?;
        let count = input.read(&mut block)?;
        ensure!(count <= block.len(), "resident decoder read extent");
        if count == 0 {
            break;
        }
        ensure!(
            samples
                .len()
                .checked_add(count)
                .is_some_and(|count| count <= MAX_PCM_FRAMES),
            "resident PCM frame budget"
        );
        samples.try_reserve(count)?;
        samples.extend_from_slice(&block[..count]);
    }
    cancel.check()?;
    Ok(PcmAudio::new(rate, samples)?)
}

struct Preparation {
    client: ClientId,
    task: Option<Task<PcmAudio>>,
    state: ResidentAudioState,
}

impl ResidentAudioProcess for Preparation {
    fn client(&self) -> ClientId {
        self.client
    }

    fn poll(&mut self) -> crysirror_core::Result<ResidentAudioState> {
        if let Some(task) = self.task.as_mut() {
            self.state = match task.poll_owner() {
                Ok(None) => return Ok(ResidentAudioState::Pending),
                Ok(Some(pcm)) => ResidentAudioState::Ready(pcm),
                Err(error) => ResidentAudioState::Failed(Error::Io(format!("{error:#}"))),
            };
            self.task = None;
        }
        Ok(self.state.clone())
    }
}

impl ChannelAudio {
    /// Open/decode on the worker and share the resulting allocation with
    /// every PCM voice that plays it.
    pub fn prepare_resident_audio(
        &self,
        client: ClientId,
        reader: Arc<dyn AssetReader>,
        extension: Option<String>,
        unknown_pcm16_bytes: Option<u64>,
        pool: &TaskPool,
    ) -> Result<ResidentAudioJob> {
        let budget = self.jobs.budget.clone();
        let task = pool.spawn(move |cancel| {
            cancel.check()?;
            ensure!(
                (1..=64 * 1024 * 1024).contains(&reader.len()?),
                "resident encoded audio size"
            );
            let mut source = decoder::open_asset(reader, extension.as_deref())?;
            if let (None, Some(bytes)) = (source.frame_count(), unknown_pcm16_bytes) {
                let channels = source
                    .channel_count()
                    .context("unknown PCM16 channel count")?;
                ensure!(
                    (1..=2).contains(&channels),
                    "PCM16 layout must be mono or stereo"
                );
                source = wrap_source(source, Some(bytes / (u64::from(channels) * 2)), 1.)?;
            }
            let frames = source.frame_count().unwrap_or(MAX_PCM_FRAMES as u64);
            ensure!(
                frames > 0 && frames <= MAX_PCM_FRAMES as u64,
                "resident PCM frame budget"
            );
            // The frame vector and final shared allocation can coexist.
            let reserved = frames as usize * 16 + 4 * 1024 * 1024;
            let charge = Arc::new(Charge::reserve(budget, reserved)?);
            let mut pcm = decode_pcm(
                wrap_source(source, Some(frames), 1.)?,
                &cancel,
                Some(frames as usize),
            )?;
            charge.shrink_to(pcm.byte_len())?;
            pcm.retain_storage(charge);
            Ok(pcm)
        })?;
        Ok(Box::new(Preparation {
            client,
            task: Some(task),
            state: ResidentAudioState::Pending,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_core::service::{AudioParameters, AudioRequest, MemoryAssetReader};
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};

    #[test]
    fn resident_audio_preparation_shares_pcm_and_retains_its_charge_during_playback() {
        let mut audio = ChannelAudio::default();
        let wave = crysirror_resource::WaveSpec {
            sample_rate: 48_000,
            channels: 1,
            ..Default::default()
        }
        .encode(&[0, 32].repeat(128))
        .unwrap();
        let reader = Arc::new(MemoryAssetReader(wave.into()));
        let pool = TaskPool::new("resident-test", 1, 4).unwrap();
        let mut job = audio
            .prepare_resident_audio(ClientId(7), reader, Some("wav".into()), None, &pool)
            .unwrap();
        assert_eq!(job.client(), ClientId(7));
        let deadline = Instant::now() + Duration::from_secs(5);
        let pcm = loop {
            match job.poll().unwrap() {
                ResidentAudioState::Ready(pcm) => break pcm,
                ResidentAudioState::Failed(error) => panic!("{error}"),
                ResidentAudioState::Pending => {
                    assert!(Instant::now() < deadline);
                    std::thread::yield_now();
                }
            }
        };
        assert_eq!(pcm.frames()[0], [0.25; 2]);
        for channel in [0, 1] {
            audio
                .submit(
                    7,
                    AudioRequest::PlayPcm {
                        channel,
                        source: pcm.clone(),
                        parameters: AudioParameters::default(),
                    },
                )
                .unwrap();
        }
        let bytes = pcm.byte_len();
        drop(pcm);
        drop(job);
        assert_eq!(audio.jobs.budget.load(Ordering::Acquire), bytes);
        audio.render(1).unwrap();
        assert!(audio.take_output().iter().all(|frame| *frame == [0.5; 2]));
        for channel in [0, 1] {
            audio
                .submit(
                    7,
                    AudioRequest::Stop {
                        channel,
                        fade_ms: 0.,
                    },
                )
                .unwrap();
        }
        assert_eq!(audio.jobs.budget.load(Ordering::Acquire), 0);
    }
}
struct Resident {
    inner: Box<dyn AudioSource>,
    replay_gain: bool,
    channels: Option<u16>,
}
impl AudioSource for Resident {
    fn has_replay_gain(&self) -> bool {
        self.replay_gain
    }
    fn channel_count(&self) -> Option<u16> {
        self.channels
    }
    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }
    fn frame_count(&self) -> Option<u64> {
        self.inner.frame_count()
    }
    fn read_position(&self) -> Option<u64> {
        self.inner.read_position()
    }
    fn is_nonblocking(&self) -> bool {
        true
    }
    fn read(&mut self, out: &mut [StereoFrame]) -> Result<usize> {
        self.inner.read(out)
    }
    fn poll_read(&mut self, out: &mut [StereoFrame]) -> Result<SourceRead> {
        self.inner.poll_read(out)
    }
    fn seek(&mut self, frame: u64) -> Result<()> {
        self.inner.seek(frame)
    }
}

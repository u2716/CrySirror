// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Scoped retained native/media channel seeks over the shared mixer cursor.
use crate::{
    channels::ChannelAudio,
    mixer::VoiceHandle,
    seek_receipt::{SourceSeekReceipt, SourceSeekState},
};
use anyhow::{Context, Result, ensure};
use crysirror_core::service::{AudioPreparation, AudioPreparePhase};
use std::collections::BTreeMap;
struct Seek {
    owner: u32,
    channel: u32,
    voice: VoiceHandle,
    sequence: u64,
    receipt: Option<SourceSeekReceipt>,
    result: Option<Result<(), String>>,
}
#[derive(Default)]
pub(super) struct ChannelSeeks {
    entries: BTreeMap<u64, Seek>,
}
impl ChannelSeeks {
    pub(super) fn exchange_reserved(&self,client:u32,left:u32,right:u32)->bool{
        self.entries.values().any(|seek|seek.owner==client&&(seek.channel==left||seek.channel==right)&&seek.result.is_none())
    }
    pub(super) fn clear_owner(&mut self, owner: u32) {
        self.entries.retain(|_, entry| entry.owner != owner);
    }
    pub(super) fn clear(&mut self) {
        self.entries.clear();
    }
}
impl ChannelAudio {
    pub(super) fn admit_channel_operation(&self, owner: u32) -> Result<()> {
        self.jobs.jobs.admit(owner)?;
        let tickets = self.jobs.jobs.tickets();
        ensure!(
            tickets.len() + self.seeks.entries.len() < 64
                && tickets.iter().filter(|(o, _, _)| *o == owner).count()
                    + self
                        .seeks
                        .entries
                        .values()
                        .filter(|s| s.owner == owner)
                        .count()
                    < 16,
            "channel operation capacity exceeded; release retained results"
        );
        Ok(())
    }
    /// Reserve a scoped result before changing the real source cursor. Ready
    /// remains true after later EOF/replacement; worker frames fence publication.
    /// Offline hosts may explicitly use synchronous sources; realtime requires
    /// a nonblocking resident or buffered source.
    pub fn begin_channel_seek(&mut self, owner: u32, channel: u32, frame: u64) -> Result<u64> {
        self.mixer.idle()?;
        self.admit_channel_operation(owner)?;
        self.poll_preparations()?;
        let voice = *self
            .voices
            .get(&(owner, channel))
            .context("audio channel is not allocated")?;
        let info = self.mixer.playback_info(owner, voice)?;
        ensure!(
            info.source_frames.is_none_or(|n| frame <= n),
            "channel seek outside source"
        );
        let ticket = crysirror_tasks::table::next_operation_id()?;
        let (sequence, receipt, result) = if self.mixer.source_seek_is_nonblocking(owner, voice)? {
            let sequence = self.mixer.begin_seek(owner, voice, frame)?;
            let receipt = self.mixer.source_seek_receipt(owner, voice, sequence)?;
            let result = match self.mixer.source_seek_state(owner, &receipt)? {
                SourceSeekState::Ready { .. } => Some(Ok(())),
                SourceSeekState::Failed(e) => Some(Err(e)),
                SourceSeekState::Cancelled => Some(Err("source seek cancelled".into())),
                SourceSeekState::Pending => None,
            };
            (sequence, Some(receipt), result)
        } else {
            ensure!(
                !self.jobs.nonblocking,
                "realtime channel seek needs a buffered source"
            );
            self.mixer.seek(owner, voice, frame)?;
            (0, None, Some(Ok(())))
        };
        self.seeks.entries.insert(
            ticket,
            Seek {
                owner,
                channel,
                voice,
                sequence,
                receipt,
                result,
            },
        );
        Ok(ticket)
    }
    pub fn channel_seek_preparation(
        &mut self,
        owner: u32,
        ticket: u64,
    ) -> Result<AudioPreparation> {
        let record = self
            .seeks
            .entries
            .get(&ticket)
            .context("unknown channel seek ticket")?;
        ensure!(record.owner == owner, "foreign channel seek ticket");
        if record.result.is_none() && !self.mixer.is_rendering() {
            let receipt = record
                .receipt
                .clone()
                .context("channel seek lost completion receipt")?;
            let service_error = self.mixer.service_sources().err();
            let state = self.mixer.source_seek_state(owner, &receipt)?;
            let result = match state {
                SourceSeekState::Ready { .. } => Some(Ok(())),
                SourceSeekState::Failed(e) => Some(Err(e)),
                SourceSeekState::Cancelled => {
                    Some(Err("source seek cancelled or superseded".into()))
                }
                SourceSeekState::Pending => {
                    if let Some(error) = service_error {
                        return Err(error.context("channel seek service"));
                    }
                    None
                }
            };
            self.seeks.entries.get_mut(&ticket).unwrap().result = result;
        }
        let record = &self.seeks.entries[&ticket];
        Ok(AudioPreparation {
            channel: record.channel,
            phase: match record.result {
                None => AudioPreparePhase::Pending,
                Some(Ok(())) => AudioPreparePhase::Ready,
                Some(Err(_)) => AudioPreparePhase::Failed,
            },
            error: record
                .result
                .as_ref()
                .and_then(|r| r.as_ref().err().cloned()),
        })
    }
    /// Pending cancellation retains the old logical source position while the
    /// buffered decoder realigns. A newer seek cannot be cancelled by this ID.
    pub fn release_channel_seek(&mut self, owner: u32, ticket: u64) -> Result<()> {
        let record = self
            .seeks
            .entries
            .get(&ticket)
            .context("unknown channel seek ticket")?;
        ensure!(record.owner == owner, "foreign channel seek ticket");
        if record.result.is_none() {
            self.mixer.idle()?;
            if self.mixer.is_playing(owner, record.voice) {
                self.mixer
                    .cancel_seek(owner, record.voice, record.sequence)?;
            }
        }
        self.seeks.entries.remove(&ticket);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        decoder::{AudioSource, StereoFrame},
        mixer::VoiceSpec,
        render_job::MixJob,
    };
    use crysirror_tasks::TaskPool;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::{Duration, Instant},
    };
    struct GateSource {
        ready: Arc<AtomicBool>,
        fail_read: bool,
        at: u64,
    }
    impl AudioSource for GateSource {
        fn sample_rate(&self) -> u32 {
            48000
        }
        fn frame_count(&self) -> Option<u64> {
            Some(1000)
        }
        fn read_position(&self) -> Option<u64> {
            Some(self.at)
        }
        fn is_nonblocking(&self) -> bool {
            true
        }
        fn seek(&mut self, at: u64) -> Result<()> {
            self.at = at;
            Ok(())
        }
        fn poll_seek(&mut self, at: u64) -> Result<bool> {
            if !self.ready.load(Ordering::SeqCst) {
                return Ok(false);
            }
            self.seek(at)?;
            Ok(true)
        }
        fn read(&mut self, out: &mut [StereoFrame]) -> Result<usize> {
            ensure!(!self.fail_read, "authored read failure after actual seek");
            let count = out.len().min(1000usize.saturating_sub(self.at as usize));
            out[..count].fill([0.25; 2]);
            self.at += count as u64;
            Ok(count)
        }
    }
    fn setup(fail_read: bool) -> (ChannelAudio, Arc<AtomicBool>, VoiceHandle) {
        let mut audio = ChannelAudio::default();
        let ready = Arc::new(AtomicBool::new(false));
        let h = audio
            .mixer
            .play(
                1,
                Box::new(GateSource {
                    ready: ready.clone(),
                    fail_read,
                    at: 0,
                }),
                VoiceSpec::default(),
            )
            .unwrap();
        audio.voices.insert((1, 7), h);
        (audio, ready, h)
    }
    fn collect(job: &mut MixJob, audio: &mut ChannelAudio) -> Result<Vec<StereoFrame>> {
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(pcm) = job.poll(&mut [&mut audio.mixer])? {
                return Ok(pcm);
            }
            assert!(Instant::now() < end);
            std::thread::yield_now();
        }
    }
    #[test]
    fn pending_seek_keeps_logical_cursor_and_cancel_restores_prior_source_position() {
        let (mut a, ready, h) = setup(false);
        a.mixer.render(&mut [[0.; 2]; 5]).unwrap();
        a.mixer.set_gain(1, h, 0.5).unwrap();
        a.mixer.pause(1, h, true).unwrap();
        let t = a.begin_channel_seek(1, 7, 500).unwrap();
        assert_eq!(
            a.channel_seek_preparation(1, t).unwrap().phase,
            AudioPreparePhase::Pending
        );
        assert_eq!(a.playback_info(1, 7).unwrap().unwrap().position_frames, 5);
        assert!(a.channel_seek_preparation(2, t).is_err());
        assert!(a.release_channel_seek(2, t).is_err());
        a.release_channel_seek(1, t).unwrap();
        ready.store(true, Ordering::SeqCst);
        a.mixer.service_sources().unwrap();
        let info = a.playback_info(1, 7).unwrap().unwrap();
        assert_eq!(info.position_frames, 5);
        assert!(info.paused);
        assert_eq!(info.gain, 0.5);
        a.mixer.pause(1, h, false).unwrap();
        let mut pcm = [[0.; 2]; 2];
        a.mixer.render(&mut pcm).unwrap();
        assert_eq!(pcm, [[0.125; 2]; 2]);
        assert_eq!(a.playback_info(1, 7).unwrap().unwrap().position_frames, 7);
        assert!(a.channel_seek_preparation(1, t).is_err());
    }
    #[test]
    fn superseded_receipt_cannot_cancel_new_seek_and_cross_mixer_capability_is_rejected() {
        let (mut a, ready, h) = setup(false);
        let old = a.begin_channel_seek(1, 7, 300).unwrap();
        let seq = a.mixer.seek_progress(1, h, 1).unwrap().sequence;
        let receipt = a.mixer.source_seek_receipt(1, h, seq).unwrap();
        let b = crate::mixer::Mixer::default();
        assert!(b.source_seek_state(1, &receipt).is_err());
        assert!(a.mixer.source_seek_state(2, &receipt).is_err());
        let new = a.begin_channel_seek(1, 7, 700).unwrap();
        assert_eq!(
            a.channel_seek_preparation(1, old).unwrap().phase,
            AudioPreparePhase::Failed
        );
        a.release_channel_seek(1, old).unwrap();
        ready.store(true, Ordering::SeqCst);
        assert_eq!(
            a.channel_seek_preparation(1, new).unwrap().phase,
            AudioPreparePhase::Ready
        );
        assert_eq!(a.playback_info(1, 7).unwrap().unwrap().position_frames, 700);
        a.mixer.stop(1, h, 0.).unwrap();
        assert_eq!(
            a.channel_seek_preparation(1, new).unwrap().phase,
            AudioPreparePhase::Ready
        );
    }
    #[test]
    fn worker_seek_then_same_frame_eof_retains_ready_only_after_owner_publication() {
        let (mut a, ready, _) = setup(false);
        let t = a.begin_channel_seek(1, 7, 999).unwrap();
        ready.store(true, Ordering::SeqCst);
        let p = TaskPool::new("seek-eof", 1, 2).unwrap();
        let mut job = MixJob::begin(&p, &mut [&mut a.mixer], 8, vec![]).unwrap();
        assert_eq!(
            a.channel_seek_preparation(1, t).unwrap().phase,
            AudioPreparePhase::Pending
        );
        assert!(a.release_channel_seek(1, t).is_err());
        assert_eq!(
            collect(&mut job, &mut a).unwrap(),
            vec![
                [0.25; 2], [0.; 2], [0.; 2], [0.; 2], [0.; 2], [0.; 2], [0.; 2], [0.; 2]
            ]
        );
        assert!(!a.is_playing(1, 7));
        assert_eq!(
            a.channel_seek_preparation(1, t).unwrap().phase,
            AudioPreparePhase::Ready
        );
        a.release_channel_seek(1, t).unwrap();
    }
    #[test]
    fn failed_or_cancelled_worker_cannot_publish_an_uncommitted_seek_success() {
        for cancel in [false, true] {
            let (mut a, ready, _) = setup(!cancel);
            let t = a.begin_channel_seek(1, 7, 900).unwrap();
            ready.store(true, Ordering::SeqCst);
            let p = TaskPool::new("seek-failure", 1, 2).unwrap();
            let mut job = MixJob::begin(&p, &mut [&mut a.mixer], 8, vec![]).unwrap();
            // Wait for the worker to really finish, but deliberately do not
            // publish it before cancellation. This exercises late Ready data.
            let end = Instant::now() + Duration::from_secs(5);
            while p.stats().outstanding > 0 {
                assert!(Instant::now() < end);
                std::thread::yield_now();
            }
            assert_eq!(
                a.channel_seek_preparation(1, t).unwrap().phase,
                AudioPreparePhase::Pending
            );
            if cancel {
                job.cancel(&mut [&mut a.mixer]);
            } else {
                assert!(collect(&mut job, &mut a).is_err());
            }
            assert_eq!(
                a.channel_seek_preparation(1, t).unwrap().phase,
                AudioPreparePhase::Failed
            );
            assert!(!a.is_playing(1, 7));
        }
    }
    #[test]
    fn retained_results_are_bounded_before_source_mutation_and_reset_invalidates_ids() {
        let (mut a, ready, _) = setup(false);
        ready.store(true, Ordering::SeqCst);
        let tickets = (0..16)
            .map(|i| a.begin_channel_seek(1, 7, i).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(a.playback_info(1, 7).unwrap().unwrap().position_frames, 15);
        assert!(a.begin_channel_seek(1, 7, 900).is_err());
        assert_eq!(a.playback_info(1, 7).unwrap().unwrap().position_frames, 15);
        a.release_channel_seek(1, tickets[0]).unwrap();
        let new = a.begin_channel_seek(1, 7, 800).unwrap();
        assert_eq!(
            a.channel_seek_preparation(1, new).unwrap().phase,
            AudioPreparePhase::Ready
        );
        a.reset();
        assert!(a.channel_seek_preparation(1, new).is_err());
    }
}

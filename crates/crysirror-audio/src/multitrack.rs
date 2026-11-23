// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Multi-track sources: equally clocked stereo stems mixed with independent,
//! live gains. The mixer reads the stems in lockstep ([`TrackSet::take_tracks`])
//! and applies each stem's [`GainLine`] by source frame, so worker-buffered
//! stems still change gain at the exact frame requested. Read directly, a
//! track set sums its stems at their initial gains.
use crate::decoder::{AudioSource, SourceRead, StereoFrame};
use anyhow::{Result, ensure};

/// Most stems one voice mixes.
pub const MAX_TRACKS: usize = 64;

pub struct TrackSet {
    tracks: Vec<(Box<dyn AudioSource>, f32)>,
    rate: u32,
    frames: Option<u64>,
    scratch: Vec<StereoFrame>,
}
impl TrackSet {
    /// Stems must share a rate. Stems of known length are padded with silence
    /// to the longest one, so every stem ends at the same frame.
    pub fn new(tracks: Vec<(Box<dyn AudioSource>, f32)>) -> Result<Self> {
        ensure!(
            (1..=MAX_TRACKS).contains(&tracks.len()),
            "multi-track source needs 1..={MAX_TRACKS} tracks"
        );
        let rate = tracks[0].0.sample_rate();
        ensure!(
            tracks.iter().all(|(t, g)| t.sample_rate() == rate && g.is_finite() && (0.0..=16.).contains(g)),
            "multi-track stems differ in rate or have an invalid gain"
        );
        let lengths: Option<Vec<u64>> = tracks.iter().map(|(t, _)| t.frame_count()).collect();
        let frames = lengths.map(|l| l.into_iter().max().unwrap_or(0));
        let tracks = match frames {
            Some(total) => tracks
                .into_iter()
                .map(|(t, g)| (Box::new(Padded { inner: t, total, at: 0, ended: false }) as Box<dyn AudioSource>, g))
                .collect(),
            None => tracks,
        };
        Ok(Self { tracks, rate, frames, scratch: Vec::new() })
    }
}
impl AudioSource for TrackSet {
    fn channel_count(&self) -> Option<u16> {
        Some(2)
    }
    fn sample_rate(&self) -> u32 {
        self.rate
    }
    fn frame_count(&self) -> Option<u64> {
        self.frames
    }
    fn read(&mut self, out: &mut [StereoFrame]) -> Result<usize> {
        out.fill([0.; 2]);
        self.scratch.resize(out.len(), [0.; 2]);
        let mut longest = 0;
        for (track, gain) in &mut self.tracks {
            let mut done = 0;
            while done < out.len() {
                let n = track.read(&mut self.scratch[done..])?;
                if n == 0 {
                    break;
                }
                done += n;
            }
            for (o, s) in out.iter_mut().zip(&self.scratch[..done]) {
                o[0] += s[0] * *gain;
                o[1] += s[1] * *gain;
            }
            longest = longest.max(done);
        }
        Ok(longest)
    }
    fn seek(&mut self, frame: u64) -> Result<()> {
        self.tracks.iter_mut().try_for_each(|(t, _)| t.seek(frame))
    }
    fn track_count(&self) -> usize {
        self.tracks.len()
    }
    fn take_tracks(&mut self) -> Option<Vec<(Box<dyn AudioSource>, f32)>> {
        (!self.tracks.is_empty()).then(|| std::mem::take(&mut self.tracks))
    }
}
/// Give every stem of a multi-track source its own worker buffer (the first
/// retains `storage`) and reassemble the set for the mixer.
pub(crate) fn buffer_tracks(
    source: &mut Box<dyn AudioSource>,
    pool: crysirror_tasks::TaskPool,
    loop_start: Option<u64>,
    start: u64,
    cancel: &crysirror_tasks::Cancellation,
    storage: std::sync::Arc<dyn Send + Sync>,
) -> Result<()> {
    use anyhow::Context;
    let tracks = source.take_tracks().context("multi-track source lost its tracks")?;
    let mut buffered: Vec<(Box<dyn AudioSource>, f32)> = Vec::with_capacity(tracks.len());
    for (i, (track, gain)) in tracks.into_iter().enumerate() {
        let mut stem = crate::buffered::BufferedSource::prepare_at_cancellable(
            track,
            pool.clone(),
            crate::buffered::StreamBufferSpec::default(),
            loop_start,
            start,
            cancel,
        )?;
        if i == 0 {
            stem.retain_storage(storage.clone())?;
        }
        buffered.push((Box::new(stem), gain));
    }
    *source = Box::new(TrackSet::new(buffered)?);
    Ok(())
}
/// A stem that yields silence from its own end up to the set's length.
struct Padded {
    inner: Box<dyn AudioSource>,
    total: u64,
    at: u64,
    ended: bool,
}
impl Padded {
    fn pad(&mut self, out: &mut [StereoFrame]) -> usize {
        let n = (self.total.saturating_sub(self.at)).min(out.len() as u64) as usize;
        out[..n].fill([0.; 2]);
        self.at += n as u64;
        n
    }
}
impl AudioSource for Padded {
    fn has_replay_gain(&self) -> bool {
        self.inner.has_replay_gain()
    }
    fn channel_count(&self) -> Option<u16> {
        self.inner.channel_count()
    }
    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }
    fn frame_count(&self) -> Option<u64> {
        Some(self.total)
    }
    fn read(&mut self, out: &mut [StereoFrame]) -> Result<usize> {
        if self.ended {
            return Ok(self.pad(out));
        }
        let n = self.inner.read(out)?;
        if n == 0 {
            self.ended = true;
            return Ok(self.pad(out));
        }
        self.at += n as u64;
        Ok(n)
    }
    fn poll_read(&mut self, out: &mut [StereoFrame]) -> Result<SourceRead> {
        if self.ended {
            return Ok(match self.pad(out) {
                0 => SourceRead::End,
                n => SourceRead::Frames(n),
            });
        }
        match self.inner.poll_read(out)? {
            SourceRead::End => {
                self.ended = true;
                Ok(match self.pad(out) {
                    0 => SourceRead::End,
                    n => SourceRead::Frames(n),
                })
            }
            SourceRead::Frames(n) => {
                self.at += n as u64;
                Ok(SourceRead::Frames(n))
            }
            pending => Ok(pending),
        }
    }
    fn seek(&mut self, frame: u64) -> Result<()> {
        ensure!(frame <= self.total, "multi-track seek outside source");
        self.ended = false;
        self.at = frame;
        let own = self.inner.frame_count().unwrap_or(self.total);
        if frame >= own {
            self.ended = true;
            return Ok(());
        }
        self.inner.seek(frame)
    }
    fn poll_seek(&mut self, frame: u64) -> Result<bool> {
        let own = self.inner.frame_count().unwrap_or(self.total);
        if frame >= own {
            ensure!(frame <= self.total, "multi-track seek outside source");
            self.ended = true;
            self.at = frame;
            return Ok(true);
        }
        let ready = self.inner.poll_seek(frame)?;
        if ready {
            self.ended = false;
            self.at = frame;
        }
        Ok(ready)
    }
    fn service(&mut self) -> Result<()> {
        self.inner.service()
    }
    fn read_position(&self) -> Option<u64> {
        if self.ended { Some(self.at) } else { self.inner.read_position() }
    }
    fn is_nonblocking(&self) -> bool {
        self.inner.is_nonblocking()
    }
    fn buffer_status(&self) -> Option<crate::decoder::SourceBufferStatus> {
        self.inner.buffer_status()
    }
}

/// A linear gain ramp keyed by source frame: `from` before `start`, `to`
/// from `start + len`, linear between (KiriKiri wumultitrack 0x1000E380:
/// a zero time or an unchanged target switches at once).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GainLine {
    pub from: f32,
    pub to: f32,
    pub start: u64,
    pub len: u64,
}
impl GainLine {
    pub fn constant(gain: f32) -> Self {
        Self { from: gain, to: gain, start: 0, len: 0 }
    }
    pub fn at(&self, frame: u64) -> f32 {
        if frame < self.start {
            self.from
        } else if frame >= self.start + self.len {
            self.to
        } else {
            let t = (frame - self.start) as f32 / self.len as f32;
            self.from + (self.to - self.from) * t
        }
    }
    /// Retarget from the value at `frame`.
    pub fn retarget(&mut self, frame: u64, to: f32, len: u64) {
        let from = self.at(frame);
        let len = if from == to { 0 } else { len };
        *self = Self { from, to, start: frame, len };
    }
}
/// A gain change for some tracks (`None`: all) at a fade length in frames.
#[derive(Clone, Debug, PartialEq)]
pub struct TrackGainChange {
    pub tracks: Option<Vec<u32>>,
    pub gain: f32,
    pub fade_frames: u64,
}
/// Per-track gains of a voice plus changes waiting for a source frame.
#[derive(Clone, Debug, Default)]
pub struct TrackGains {
    pub lines: Vec<GainLine>,
    /// Sorted source frames; the first one reached applies `deferred`.
    pub positions: Vec<u64>,
    pub deferred: Vec<TrackGainChange>,
}
impl TrackGains {
    pub fn new(initial: &[f32]) -> Self {
        Self { lines: initial.iter().map(|g| GainLine::constant(*g)).collect(), ..Default::default() }
    }
    pub fn validate(&self, change: &TrackGainChange) -> Result<()> {
        ensure!(change.gain.is_finite() && (0.0..=16.).contains(&change.gain), "track gain out of range");
        if let Some(tracks) = &change.tracks {
            ensure!(tracks.iter().all(|t| (*t as usize) < self.lines.len()), "track index out of range");
        }
        Ok(())
    }
    pub fn apply(&mut self, frame: u64, change: &TrackGainChange) {
        let all: Vec<u32> = (0..self.lines.len() as u32).collect();
        for t in change.tracks.as_ref().unwrap_or(&all) {
            self.lines[*t as usize].retarget(frame, change.gain, change.fade_frames);
        }
    }
    /// Apply deferred changes once `frame` reaches the first pending position.
    pub fn advance(&mut self, frame: u64) {
        let Some(&at) = self.positions.first() else { return };
        if frame < at {
            return;
        }
        for change in std::mem::take(&mut self.deferred) {
            self.apply(at, &change);
        }
        self.positions.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::Clip;
    #[test]
    fn padded_sum_ramps_and_deferred_positions() {
        let a = Clip::new(8000, vec![[1., 0.5]; 4]).unwrap().source();
        let b = Clip::new(8000, vec![[0.25, 0.25]; 2]).unwrap().source();
        let mut set = TrackSet::new(vec![(a, 1.), (b, 2.)]).unwrap();
        assert_eq!(set.frame_count(), Some(4));
        let mut out = [[0.; 2]; 8];
        assert_eq!(set.read(&mut out).unwrap(), 4);
        assert_eq!(out[0], [1.5, 1.]);
        assert_eq!(out[3], [1., 0.5]);
        set.seek(1).unwrap();
        let tracks = set.take_tracks().unwrap();
        assert_eq!(tracks.len(), 2);
        let mut line = GainLine::constant(1.);
        line.retarget(10, 0., 10);
        assert_eq!([line.at(5), line.at(15), line.at(25)], [1., 0.5, 0.]);
        line.retarget(15, 0.5, 0);
        assert_eq!(line.at(16), 0.5);
        let mut gains = TrackGains::new(&[1., 1.]);
        gains.positions = vec![100];
        gains.deferred = vec![TrackGainChange { tracks: Some(vec![1]), gain: 0., fade_frames: 0 }];
        gains.advance(99);
        assert_eq!(gains.lines[1].at(99), 1.);
        gains.advance(120);
        assert_eq!([gains.lines[0].at(120), gains.lines[1].at(100), gains.lines[1].at(99)], [1., 0., 1.]);
        assert!(gains.deferred.is_empty() && gains.positions.is_empty());
    }
    #[test]
    fn mixer_applies_live_and_deferred_track_gains() {
        use crate::mixer::{Mixer, MixerConfig, VoiceSpec};
        use crysirror_core::service::AudioTrackGain;
        let a = Clip::new(8000, vec![[0.5, 0.5]; 8000]).unwrap().source();
        let b = Clip::new(8000, vec![[0.25, 0.25]; 8000]).unwrap().source();
        let set = TrackSet::new(vec![(a, 1.), (b, 1.)]).unwrap();
        let mut mixer = Mixer::new(MixerConfig { sample_rate: 8000, ..Default::default() }).unwrap();
        let h = mixer.play(1, Box::new(set), VoiceSpec::default()).unwrap();
        assert_eq!(mixer.track_count(1, h).unwrap(), 2);
        let mut out = vec![[0.; 2]; 800];
        let level = |m: &mut Mixer, out: &mut Vec<StereoFrame>| {
            m.render(out).unwrap();
            out[700][0].abs()
        };
        let both = level(&mut mixer, &mut out);
        assert!(both > 0.);
        let mute = |tracks: Vec<u32>| vec![AudioTrackGain { tracks: Some(tracks), gain: 0., fade_ms: 0 }];
        mixer.set_track_gains(1, h, None, &mute(vec![0])).unwrap();
        let only_b = level(&mut mixer, &mut out);
        assert!((only_b / both - 1. / 3.).abs() < 0.02, "{only_b} / {both}");
        // Deferred to source frame 2000 (the voice is at 1600 now).
        mixer.set_track_gains(1, h, Some(vec![2000]), &mute(vec![1])).unwrap();
        mixer.render(&mut out).unwrap();
        // Frames 1600..2400: gain holds until 2000, then the track is muted.
        assert!((out[300][0].abs() - only_b).abs() < 1e-4, "{:?}", out[300]);
        assert!(out[500][0].abs() < 1e-4, "{:?}", out[500]);
        assert!(mixer.set_track_gains(1, h, None, &mute(vec![2])).is_err());
    }
}

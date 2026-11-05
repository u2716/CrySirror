// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Device-independent additive parameter modulation. Time is measured in animation frames;
//! callers own the clock, random stream and publication of the resulting parameter values.
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct Oscillator {
    pub waveform: u32,
    pub negative: f32,
    pub positive: f32,
    pub phase: f32,
    pub duty: f32,
    pub period: u32,
}
impl Oscillator {
    pub fn sample(&self, frame: f64) -> f32 {
        if self.period == 0 && self.waveform != 0 { return 0.; }
        let period = f64::from(self.period);
        match self.waveform {
            0 => {
                let split = f64::from(self.duty) * period;
                let cycle = if frame < split {
                    if split > 0. { frame / split * 0.5 } else { 0. }
                } else {
                    let rest = period - split;
                    0.5 + if rest > 0. { (frame - split) / rest * 0.5 } else { 0. }
                };
                let wave = (f64::from(self.phase) + cycle * f64::from(std::f32::consts::TAU)).sin();
                (wave * f64::from(if wave >= 0. { self.positive } else { self.negative })) as f32
            }
            1 => ((1. - (frame / period * f64::from(std::f32::consts::TAU)).cos()) * 0.5 * f64::from(self.positive + self.negative) - f64::from(self.negative)) as f32,
            2 => (f64::from(self.positive) + ((frame / period * f64::from(std::f32::consts::TAU)).cos() - 1.) * 0.5 * f64::from(self.positive + self.negative)) as f32,
            _ => 0.,
        }
    }
}
#[derive(Clone, Debug)]
struct Track { oscillator: Oscillator, frame: f64, previous: f32, blend: f64, value: f32 }
/// A named modulation bank. Replacement preserves the current output and blends it
/// to the next oscillator over the requested number of frames; removed tracks fade to zero.
#[derive(Clone, Debug, Default)]
pub struct ModulationBank { tracks: BTreeMap<String, Track>, transition: f64 }
impl ModulationBank {
    pub fn select(&mut self, oscillators: BTreeMap<String, Oscillator>, transition_frames: f64) {
        self.transition = transition_frames.max(0.);
        for (name, track) in &mut self.tracks {
            if !oscillators.contains_key(name) {
                track.oscillator.negative = 0.; track.oscillator.positive = 0.;
                track.previous = track.value; track.blend = 0.; track.frame = 0.;
            }
        }
        for (name, oscillator) in oscillators {
            let old = self.tracks.get(&name);
            let previous = old.map_or(0., |track| track.value);
            let blend = if old.is_some() { 0. } else { self.transition };
            self.tracks.insert(name, Track { oscillator, frame: 0., previous, blend, value: previous });
        }
    }
    pub fn advance(&mut self, frames: f64) -> BTreeMap<String, f32> {
        self.advance_with(frames, |_| None)
    }
    /// Resample once after a period boundary without restarting the transition
    /// clock; late updates skip elapsed cycles without replaying random draws.
    pub fn advance_with(&mut self, frames: f64, mut resample: impl FnMut(&str) -> Option<Oscillator>) -> BTreeMap<String, f32> {
        let frames = if frames.is_finite() { frames.max(0.) } else { 0. };
        self.tracks.iter_mut().map(|(name, track)| {
            track.frame += frames;
            let transitioning = track.blend < self.transition;
            track.blend += frames;
            if !transitioning && track.oscillator.period != 0 && track.frame >= f64::from(track.oscillator.period) {
                // A late update skips whole old periods but admits one resampling
                // operation. Replaying missed cycles would consume extra random draws.
                track.frame %= f64::from(track.oscillator.period);
                if let Some(next) = resample(name) { track.oscillator = next; }
            }
            let frame = track.frame;
            let weight = if self.transition > 0. { (track.blend / self.transition).min(1.) as f32 } else { 1. };
            track.value = track.previous * (1. - weight) + track.oscillator.sample(frame) * weight;
            (name.clone(), track.value)
        }).collect()
    }
}

/// Reusable scalar interpolation, sampled on the caller's animation clock.
#[derive(Clone, Copy, Debug)]
pub enum Curve { Linear, SineOut, SineIn, SineInOut, Hold }
#[derive(Clone, Debug)]
pub struct ScalarTransition { from: f32, to: f32, duration: f32, elapsed: f32, curve: Curve }
impl ScalarTransition {
    pub fn new(from: f32, to: f32, frames: f32, curve: Curve) -> Self {
        Self { from, to, duration: frames.max(0.), elapsed: 0., curve }
    }
    pub fn advance(&mut self, frames: f32) -> f32 {
        self.elapsed = (self.elapsed + frames.max(0.)).min(self.duration);
        let t = if self.duration > 0. { self.elapsed / self.duration } else { 1. };
        let weight = match self.curve {
            Curve::Linear => t,
            Curve::SineOut => (t * std::f32::consts::FRAC_PI_2).sin(),
            Curve::SineIn => 1. - (t * std::f32::consts::FRAC_PI_2).cos(),
            Curve::SineInOut => (1. - (t * std::f32::consts::PI).cos()) * 0.5,
            Curve::Hold => if t < 1. { 0. } else { 1. },
        };
        self.from + (self.to - self.from) * weight
    }
}

#[derive(Clone, Debug)]
struct RangeGroup { users: usize, used: usize, value: f32 }
/// Equal range lists in a channel share a sampled value for one complete round of
/// consumers. This retains correlation even when consumers have different clocks.
#[derive(Clone, Copy, Debug)]
pub enum RandomDraw { Index, Unit }
#[derive(Clone, Debug, Default)]
pub struct CorrelatedRanges { groups: BTreeMap<(u32, Vec<[u32; 2]>), RangeGroup> }
impl CorrelatedRanges {
    pub fn new(entries: impl IntoIterator<Item = (u32, Vec<[f32; 2]>)>, mut random: impl FnMut(RandomDraw) -> anyhow::Result<f64>) -> anyhow::Result<Self> {
        let mut this = Self::default();
        for (channel, ranges) in entries {
            let key = (channel, ranges.iter().map(|range| range.map(f32::to_bits)).collect());
            if let Some(group) = this.groups.get_mut(&key) { group.users += 1; }
            else { this.groups.insert(key, RangeGroup { users: 1, used: 0, value: draw_ranges(&ranges, &mut random)? }); }
        }
        Ok(this)
    }
    pub fn reset_channel(&mut self, channel: u32) {
        for ((kind, _), group) in &mut self.groups { if *kind == channel { group.used = 0; } }
    }
    pub fn sample(&mut self, channel: u32, ranges: &[[f32; 2]], mut random: impl FnMut(RandomDraw) -> anyhow::Result<f64>) -> anyhow::Result<f32> {
        let key = (channel, ranges.iter().map(|range| range.map(f32::to_bits)).collect());
        let Some(group) = self.groups.get_mut(&key) else { return draw_ranges(ranges, &mut random) };
        let value = group.value;
        group.used += 1;
        if group.used >= group.users { group.used = 0; group.value = draw_ranges(ranges, &mut random)?; }
        Ok(value)
    }
}
fn draw_ranges(ranges: &[[f32; 2]], random: &mut impl FnMut(RandomDraw) -> anyhow::Result<f64>) -> anyhow::Result<f32> {
    if ranges.is_empty() { return Ok(0.); }
    let index=random(RandomDraw::Index)?;
    anyhow::ensure!(index.is_finite() && (0. ..=u32::MAX as f64).contains(&index) && index.fract()==0., "correlated range random index");
    let range = ranges[index as u32 as usize % ranges.len()];
    let unit=random(RandomDraw::Unit)?;
    anyhow::ensure!(unit.is_finite() && (0. ..=1.).contains(&unit) && range.iter().all(|v|v.is_finite()), "correlated range random fraction or bounds");
    Ok((f64::from(range[0]) + f64::from(range[1] - range[0]) * unit) as f32)
}

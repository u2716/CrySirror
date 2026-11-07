// SPDX-License-Identifier: MPL-2.0
//! Owner-scoped externally clocked incremental channel queues. Unlike a
//! normalized tween these retain velocity, activation, bank and loop state.
use super::motion::MotionProperty;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);
const MAX_CHANNELS: usize = 128;
const MAX_LANES: usize = 32;
const MAX_RECORDS: usize = 4096;
#[path = "queued_ieee.rs"]
mod ieee;
#[path = "queued_map_keys.rs"]
mod map_keys;
fn yes() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SequenceSpec {
    pub values: Vec<f32>,
    #[serde(default)]
    pub retained: Option<Vec<f32>>,
    #[serde(default)]
    pub rate_channel: Option<u8>,
    pub bindings: Vec<SequenceBinding>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SequenceBinding {
    pub layer: u32,
    pub channel: u8,
    pub target: BindingTarget,
    #[serde(default)]
    pub map: ValueMap,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BindingTarget {
    /// A derived projection depends on this channel without copying it into
    ///a finite layer property. Undefined IEEE input suppresses the subtree.
    FiniteGate,
    Motion {
        property: MotionProperty,
    },
    NineTapOffset {
        axis: u8,
    },
    /// Drive a retained image's atlas rectangle from an integer frame cursor.
    /// A terminal frame retains the atlas producer's final crop on completion.
    TexelImageUv {
        allocation:[u32;2],
        reference:[f32;2],
        source:[f32;4],
        step:[f32;2],
        #[serde(default)]
        terminal_frame:Option<u32>,
        #[serde(default)]
        active_lane:Option<u8>,
    },
    ImageUv {
        origin: [f32; 4],
        step: [f32; 2],
        #[serde(default)]
        terminal_frame: Option<u32>,
        #[serde(default)]
        active_lane: Option<u8>,
    },
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ValueMap {
    #[default]
    Identity,
    Affine {
        scale: f32,
        offset: f32,
    },
    /// One threshold applied to all listed channels before choosing a divisor.
    /// Useful for encoded percentages that share a domain switch across axes.
    Ratio {
        test_channels: Vec<u8>,
        threshold: f32,
        below: f32,
        above: f32,
    },
    /// Keep a scaled extent centered in a reference extent while the input
    ///position and scale channels remain independent CPU values.
    CenteredScale {
        source_extent:f32,
        reference_extent:f32,
        scale_channel:u8,
        test_channels:Vec<u8>,
        threshold:f32,
        below:f32,
        above:f32,
    },
    ZeroFallback {
        channel: u8,
        scale: f32,
    },
    Clamp {
        low: f32,
        high: f32,
        scale: f32,
    },
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeltaSource {
    Current,
    Retained,
    CurrentAtZeroBank,
    ExplicitStart,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SequenceTrack {
    Frames {
        channel: u8,
        frames: u32,
        hold_ticks: i32,
        #[serde(default)]
        looping: bool,
        #[serde(default = "yes")]
        capture: bool,
    },
    Linear {
        channels: Vec<u8>,
        #[serde(with = "ieee")]
        target: Vec<f32>,
        ticks: u32,
        /// Optional signed velocity denominator in ticks. Default uses `ticks`.
        /// Integration terminates at the target/forced predicate, never after
        /// an elapsed tick count; a negative divisor can move away indefinitely.
        /// Zero explicitly opts into IEEE division: nonzero deltas become
        /// infinity and snap on the advancing tick, while 0/0 retains a NaN
        /// until forced completion. Status transports nonfinite bits explicitly.
        #[serde(default)]
        velocity_divisor: Option<f32>,
        /// Explicitly retain IEEE inputs/outputs, including nonfinite target
        /// values. Only this track's owned channels gain that permission;
        /// finite GPU/layer properties still withhold nonfinite publication.
        #[serde(default)]
        ieee_inputs: bool,
        /// Velocity reference and loop origin can intentionally share another
        /// channel (uniform scale with initially different axes, for example).
        references: Vec<u8>,
        loop_origins: Vec<u8>,
        delta: DeltaSource,
        #[serde(default)]
        looping: bool,
        #[serde(default, with = "ieee::optional")]
        start: Option<Vec<f32>>,
        /// Scalar lanes may complete from their first component; vector moves
        /// generally wait for every component, or a zero velocity vector.
        #[serde(default)]
        complete_first: bool,
        #[serde(default)]
        complete_zero: bool,
        #[serde(default)]
        fast: bool,
        ///Restrict fast-mode multiplication to these output channel IDs.
        ///None preserves the default of accelerating every component.
        #[serde(default)]
        fast_channels: Option<Vec<u8>>,
        #[serde(default = "yes")]
        skip_zero: bool,
        #[serde(default = "yes")]
        capture_zero: bool,
    },
    Oscillation {
        channel: u8,
        amplitude: f32,
        start_degrees: f32,
        end_degrees: f32,
        ticks: u32,
        #[serde(default)]
        cosine: bool,
        #[serde(default)]
        looping: bool,
        radians_per_degree: f64,
        #[serde(default)]
        fast: bool,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SequenceOperation {
    /// Restore execution channels/activation without replacing queued banks or
    /// capture templates. Selected active lanes seed a neutral bank-zero entry
    /// only if it is absent; existing future records retain their metadata.
    RestoreControl {
        state: SequenceStatus,
        #[serde(default)]
        seed_missing: Vec<u8>,
    },
    /// Replace current and retained channel values and clear activation state,
    /// while retaining queued records and capture/replay ownership.
    Reset {
        values: Vec<f32>,
        retained: Vec<f32>,
    },
    Enqueue {
        lane: u8,
        #[serde(default)]
        bank: Option<u8>,
        track: SequenceTrack,
    },
    Set {
        channels: Vec<u8>,
        values: Vec<f32>,
        #[serde(default)]
        retained: bool,
    },
    /// Change only an existing binding's projection map. Retain its actor,
    /// channels, active execution fields and captured/queued records.
    SetBindingMap {layer:u32,channel:u8,map:ValueMap},
    /// Update a sampled image's retained source edges without replacing its
    /// actor, current channels, queued motion or frame cursor.
    SetTexelSource {
        layer:u32,
        source:[f32;4],
    },
    Capture {
        enabled: bool,
        repeats: u16,
        #[serde(default)]
        forever: bool,
    },
    Replay,
    Advance {
        #[serde(with = "crate::input_buffer::exact_u64")]
        generation: u64,
        #[serde(with = "crate::input_buffer::exact_u64")]
        revision: u64,
        rate: f32,
        #[serde(default)]
        fast: bool,
    },
    Finish {
        #[serde(default)]
        cancel: bool,
        #[serde(default)]
        lanes: Option<Vec<u8>>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SequenceStatus {
    #[serde(with = "crate::input_buffer::exact_u64")]
    pub generation: u64,
    #[serde(with = "crate::input_buffer::exact_u64")]
    pub revision: u64,
    /// Nonfinite values use {"ieee754":u32} only on zero-divisor-owned channels.
    #[serde(with = "ieee")]
    pub values: Vec<f32>,
    #[serde(with = "ieee")]
    pub retained: Vec<f32>,
    #[serde(deserialize_with = "map_keys::deserialize")]
    pub lanes: BTreeMap<u8, Lane>,
    pub capture: bool,
    #[serde(deserialize_with = "map_keys::nested")]
    pub captured: BTreeMap<u8, BTreeMap<u8, SequenceTrack>>,
    pub repeats: u16,
    pub forever: bool,
    pub complete: bool,
}
impl SequenceStatus {
    /// Whether this channel is owned by a retained zero-divisor or explicitly
    /// IEEE-enabled linear record.
    /// Nonfinite transport/restore is allowed only with this explicit provenance;
    /// it never grants permission to publish a nonfinite GPU/layer property.
    pub fn allows_nonfinite_channel(&self, channel: u8) -> bool {
        ieee_channels(self).contains(&channel)
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Lane {
    pub cursor: u16,
    #[serde(deserialize_with = "map_keys::deserialize")]
    pub records: BTreeMap<u8, Record>,
    pub active: Option<Active>,
    #[serde(default)]
    pub control: Option<Record>,
    pub last_record: Option<Record>,
    pub last_active: Option<Active>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
    #[serde(default)]
    pub progress: Option<Active>,
    pub track: SequenceTrack,
    #[serde(with = "ieee")]
    pub velocity: Vec<f32>,
    #[serde(with = "ieee")]
    pub origin: Vec<f32>,
}
impl Record {
    fn neutral(&self) -> Result<Self> {
        let mut record = self.clone();
        record.progress = None;
        record.velocity.fill(0.);
        record.origin.fill(0.);
        match &mut record.track {
            SequenceTrack::Linear {
                target,
                ticks,
                velocity_divisor,
                start,
                delta,
                looping,
                ..
            } => {
                target.fill(0.);
                *ticks = 1;
                *velocity_divisor = None;
                *start = None;
                *delta = DeltaSource::Current;
                *looping = false;
            }
            SequenceTrack::Oscillation {
                amplitude,
                start_degrees,
                end_degrees,
                ticks,
                looping,
                ..
            } => {
                *amplitude = 0.;
                *start_degrees = 0.;
                *end_degrees = 0.;
                *ticks = 1;
                *looping = false;
            }
            SequenceTrack::Frames {
                frames,
                hold_ticks,
                looping,
                ..
            } => {
                if *frames != 0 {
                    *frames = 1;
                }
                *hold_ticks = -1;
                *looping = false;
            }
        }
        Ok(record)
    }
    fn validate(&self, count: usize) -> Result<()> {
        self.track.validate(count)?;
        if let Some(progress) = &self.progress {
            ensure!(
                matches!((&self.track,progress),(SequenceTrack::Frames{frames,..},Active::Frames{frame,counter,..}) if *frame<=*frames && *counter<=i32::MAX as u32+1),
                "sequence record-owned frame progress"
            );
        }
        let arity = match &self.track {
            SequenceTrack::Frames { .. } => 0,
            SequenceTrack::Linear { channels, .. } => channels.len(),
            SequenceTrack::Oscillation { .. } => 1,
        };
        ensure!(
            self.velocity.len() == arity
                && (finite(&self.velocity) && finite(&self.origin) || ieee_track(&self.track)),
            "sequence record velocity/origin malformed"
        );
        ensure!(
            match self.track {
                SequenceTrack::Linear { .. } => self.origin.len() == arity,
                SequenceTrack::Oscillation { .. } | SequenceTrack::Frames { .. } =>
                    self.origin.is_empty(),
            },
            "sequence record origin arity"
        );
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Active {
    Frames {
        frame: u32,
        counter: u32,
        attached: bool,
    },
    Linear {
        #[serde(with = "ieee")]
        velocity: Vec<f32>,
        multiplier: f32,
    },
    Oscillation {
        phase: f32,
        velocity: f32,
        baseline: f32,
        multiplier: f32,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Actor {
    spec: SequenceSpec,
    status: SequenceStatus,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct Sequences {
    clients: BTreeMap<u32, BTreeMap<u32, Actor>>,
}
fn finite(values: &[f32]) -> bool {
    values.iter().all(|v| v.is_finite())
}
fn ieee_track(track: &SequenceTrack) -> bool {
    matches!(track,SequenceTrack::Linear { velocity_divisor:Some(value),.. } if *value==0.)
        || matches!(
            track,
            SequenceTrack::Linear {
                ieee_inputs: true,
                ..
            }
        )
}
fn ieee_channels(status: &SequenceStatus) -> BTreeSet<u8> {
    status
        .lanes
        .values()
        .flat_map(|lane| {
            lane.records
                .values()
                .chain(lane.control.iter())
                .chain(lane.last_record.iter())
        })
        .filter_map(|record| match &record.track {
            SequenceTrack::Linear { channels, .. } if ieee_track(&record.track) => {
                Some(channels.iter().copied())
            }
            _ => None,
        })
        .flatten()
        .collect()
}
fn finite_or_owned(values: &[f32], owned: &BTreeSet<u8>) -> bool {
    values
        .iter()
        .enumerate()
        .all(|(channel, value)| value.is_finite() || owned.contains(&(channel as u8)))
}
//447312/44734A and447FF0 use x87 FDIV, storing binary32. Masked invalid
//0/0 produces the x87 negative quiet indefinite, not a host-selected NaN.
//Import/export never performs arithmetic on bits. A single imported NaN's
//payload is preserved (and quieted) when it participates in an actual update.
fn quiet_nan(value: f32) -> f32 {
    f32::from_bits(value.to_bits() | 0x0040_0000)
}
fn ieee_divide(delta: f32, divisor: f32) -> f32 {
    if delta.is_nan() {
        return quiet_nan(delta);
    }
    if divisor == 0. {
        if delta == 0. {
            return f32::from_bits(0xffc0_0000);
        }
        return f32::from_bits(0x7f80_0000 | ((delta.to_bits() ^ divisor.to_bits()) & 0x8000_0000));
    }
    (f64::from(delta) / f64::from(divisor)) as f32
}
fn ieee_multiply(left: f32, right: f32) -> f32 {
    if left.is_nan() {
        return quiet_nan(left);
    }
    if right.is_nan() {
        return quiet_nan(right);
    }
    if left.is_infinite() && right == 0. || right.is_infinite() && left == 0. {
        return f32::from_bits(0xffc0_0000);
    }
    (f64::from(left) * f64::from(right)) as f32
}
fn ieee_add(left: f32, right: f32) -> f32 {
    if left.is_nan() {
        return quiet_nan(left);
    }
    if right.is_nan() {
        return quiet_nan(right);
    }
    if left.is_infinite()
        && right.is_infinite()
        && left.is_sign_negative() != right.is_sign_negative()
    {
        return f32::from_bits(0xffc0_0000);
    }
    (f64::from(left) + f64::from(right)) as f32
}
impl SequenceSpec {
    fn validate(&self) -> Result<()> {
        ensure!(
            !self.values.is_empty() && self.values.len() <= MAX_CHANNELS && finite(&self.values),
            "sequence channel budget/scalars"
        );
        if let Some(retained) = &self.retained {
            ensure!(
                retained.len() == self.values.len() && finite(retained),
                "sequence initial retained channels"
            );
        }
        ensure!(
            self.rate_channel
                .is_none_or(|n| usize::from(n) < self.values.len()),
            "sequence rate channel"
        );
        ensure!(self.bindings.len() <= 128, "sequence binding budget");
        for binding in &self.bindings {
            ensure!(
                usize::from(binding.channel) < self.values.len(),
                "sequence binding channel"
            );
            match &binding.map {
                ValueMap::Identity => {}
                ValueMap::Affine { scale, offset } => {
                    ensure!(finite(&[*scale, *offset]), "sequence affine map")
                }
                ValueMap::Ratio {
                    test_channels,
                    threshold,
                    below,
                    above,
                } => ensure!(
                    !test_channels.is_empty()
                        && test_channels
                            .iter()
                            .all(|n| usize::from(*n) < self.values.len())
                        && finite(&[*threshold, *below, *above])
                        && *below != 0.
                        && *above != 0.,
                    "sequence ratio map"
                ),
                ValueMap::CenteredScale{source_extent,reference_extent,scale_channel,test_channels,threshold,below,above}=>ensure!(
                    finite(&[*source_extent,*reference_extent,*threshold,*below,*above])&&*below!=0.&&*above!=0.&&usize::from(*scale_channel)<self.values.len()&&!test_channels.is_empty()&&test_channels.iter().all(|channel|usize::from(*channel)<self.values.len()),"sequence centered scale map"),
                ValueMap::ZeroFallback { channel, scale } => ensure!(
                    usize::from(*channel) < self.values.len() && scale.is_finite(),
                    "sequence zero fallback"
                ),
                ValueMap::Clamp { low, high, scale } => ensure!(
                    finite(&[*low, *high, *scale]) && low <= high,
                    "sequence clamp map"
                ),
            }
            if let BindingTarget::ImageUv {
                origin,
                step,
                active_lane,
                ..
            } = &binding.target
            {
                ensure!(
                    active_lane.is_none_or(|lane| usize::from(lane) < MAX_LANES),
                    "sequence image UV lane"
                );
                ensure!(
                    origin.iter().chain(step).all(|v| v.is_finite())
                        && origin[2] >= origin[0]
                        && origin[3] >= origin[1],
                    "sequence image UV binding"
                );
            }
            if let BindingTarget::TexelImageUv{allocation,reference,source,step,active_lane,..}=&binding.target {
                ensure!(allocation.iter().all(|size|*size>0)&&reference.iter().chain(source).chain(step).all(|value|value.is_finite())&&active_lane.is_none_or(|lane|usize::from(lane)<MAX_LANES),"sequence texel image binding");
            }
            if let BindingTarget::NineTapOffset { axis } = binding.target {
                ensure!(axis < 2, "sequence filter offset axis");
            }
        }
        Ok(())
    }
}
impl SequenceTrack {
    fn validate(&self, count: usize) -> Result<()> {
        match self {
            Self::Frames {
                channel, frames, ..
            } => ensure!(
                usize::from(*channel) < count,
                "sequence frame extent/channel"
            ),
            Self::Linear {
                channels,
                target,
                ticks,
                references,
                loop_origins,
                start,
                delta,
                fast_channels,
                velocity_divisor,
                ..
            } => {
                ensure!(
                    velocity_divisor.is_none_or(|v| v.is_finite()),
                    "sequence signed velocity divisor"
                );
                if let Some(selected) = fast_channels {
                    ensure!(
                        selected.iter().all(|channel| channels.contains(channel))
                            && selected.iter().collect::<BTreeSet<_>>().len() == selected.len(),
                        "sequence fast channel selection"
                    );
                }
                ensure!(
                    !channels.is_empty()
                        && channels.len() <= MAX_CHANNELS
                        && (*ticks > 0 || ieee_track(self))
                        && *ticks <= 1_000_000,
                    "sequence vector/duration"
                );
                ensure!(
                    target.len() == channels.len()
                        && references.len() == channels.len()
                        && loop_origins.len() == channels.len()
                        && (finite(target) || ieee_track(self)),
                    "sequence vector arity"
                );
                ensure!(
                    channels
                        .iter()
                        .chain(references)
                        .chain(loop_origins)
                        .all(|n| usize::from(*n) < count),
                    "sequence vector channel"
                );
                ensure!(
                    channels.iter().collect::<BTreeSet<_>>().len() == channels.len(),
                    "sequence duplicate output channel"
                );
                if let Some(start) = start {
                    ensure!(
                        start.len() == channels.len() && (finite(start) || ieee_track(self)),
                        "sequence explicit start"
                    );
                }
                ensure!(
                    !matches!(delta, DeltaSource::ExplicitStart) || start.is_some(),
                    "sequence explicit delta start missing"
                );
            }
            Self::Oscillation {
                channel,
                amplitude,
                start_degrees,
                end_degrees,
                ticks,
                radians_per_degree,
                ..
            } => ensure!(
                usize::from(*channel) < count
                    && finite(&[*amplitude, *start_degrees, *end_degrees])
                    && radians_per_degree.is_finite()
                    && *ticks > 0
                    && *ticks <= 1_000_000,
                "sequence oscillation scalars/duration"
            ),
        }
        Ok(())
    }
}
impl Actor {
    fn count(&self) -> usize {
        self.status
            .lanes
            .values()
            .map(|l| l.records.len())
            .sum::<usize>()
            + self
                .status
                .captured
                .values()
                .map(BTreeMap::len)
                .sum::<usize>()
    }
    fn enqueue(&mut self, lane: u8, bank: Option<u8>, track: SequenceTrack) -> Result<()> {
        ensure!(usize::from(lane) < MAX_LANES, "sequence lane budget");
        track.validate(self.status.values.len())?;
        ensure!(self.count() < MAX_RECORDS, "sequence record budget");
        if matches!(&track,SequenceTrack::Oscillation{start_degrees,end_degrees,..} if start_degrees==end_degrees)
        {
            return Ok(());
        }
        if self.status.capture && !matches!(&track, SequenceTrack::Frames { capture: false, .. }) {
            if matches!(&track,SequenceTrack::Linear{capture_zero:false,start:Some(start),target,..} if start==target)
            {
                return Ok(());
            }
            let map = self.status.captured.entry(lane).or_default();
            let bank = bank.map_or(map.len(), usize::from);
            ensure!(bank <= 100, "sequence captured bank budget");
            map.entry(bank as u8).or_insert(track);
            return Ok(());
        }
        let map = self.status.lanes.entry(lane).or_default();
        let bank = bank.map_or(map.records.len(), usize::from);
        ensure!(bank <= 100, "sequence bank budget");
        let values = &self.status.values;
        let retained = &mut self.status.retained;
        let (velocity, origin) = match &track {
            SequenceTrack::Frames { .. } => (vec![], vec![]),
            SequenceTrack::Linear {
                channels,
                target,
                ticks,
                velocity_divisor,
                references,
                loop_origins,
                delta,
                start,
                ..
            } => {
                let velocity = references
                    .iter()
                    .zip(target)
                    .enumerate()
                    .map(|(n, (reference, to))| {
                        let source = match delta {
                            DeltaSource::Current => values[usize::from(*reference)],
                            DeltaSource::Retained => retained[usize::from(*reference)],
                            DeltaSource::CurrentAtZeroBank => {
                                if bank == 0 {
                                    values[usize::from(*reference)]
                                } else {
                                    retained[usize::from(*reference)]
                                }
                            }
                            DeltaSource::ExplicitStart => {
                                start.as_ref().expect("validated explicit delta start")[n]
                            }
                        };
                        //Source evidence:4472A0/4476C0/447FF0 compute signed
                        //float division;448CE0's corresponding lanes finish
                        //from target±abs(step)/forced predicates, not duration.
                        //The signed divisor is only declaration arithmetic.
                        //tick() retains the same target±abs(step) completion,
                        //with no hidden absolute-duration countdown.
                        ieee_divide(
                            (*to as f64 - source as f64) as f32,
                            velocity_divisor.unwrap_or(*ticks as f32),
                        )
                    })
                    .collect::<Vec<_>>();
                let origin = start.clone().unwrap_or_else(|| {
                    loop_origins
                        .iter()
                        .map(|n| values[usize::from(*n)])
                        .collect()
                });
                for (channel, target) in channels.iter().zip(target) {
                    retained[usize::from(*channel)] = *target;
                }
                (velocity, origin)
            }
            SequenceTrack::Oscillation {
                start_degrees,
                end_degrees,
                ticks,
                ..
            } => (
                vec![(f64::from(*end_degrees) - f64::from(*start_degrees)) / f64::from(*ticks)]
                    .into_iter()
                    .map(|v| v as f32)
                    .collect(),
                vec![],
            ),
        };
        if matches!(
            &track,
            SequenceTrack::Linear {
                skip_zero: true,
                ..
            }
        ) && velocity.iter().all(|v| *v == 0.)
        {
            return Ok(());
        }
        ensure!(
            finite(&velocity) || ieee_track(&track),
            "sequence nonfinite velocity without zero-divisor ownership"
        );
        // Equal-key insertion keeps the existing record, matching a unique-key
        // bank. Retained targets above are still acknowledged declaration state.
        map.records.entry(bank as u8).or_insert(Record {
            progress: None,
            track,
            velocity,
            origin,
        });
        Ok(())
    }
    fn replay(&mut self) -> Result<()> {
        if self.status.capture || self.status.repeats == 0 {
            return Ok(());
        }
        let captured = self.status.captured.clone();
        self.status.capture = false;
        let repeats = self.status.repeats;
        for _ in 0..repeats {
            for (lane, records) in &captured {
                for record in records.values() {
                    self.enqueue(*lane, None, record.clone())?;
                }
            }
        }
        self.status.repeats = 0;
        Ok(())
    }
    fn tick(&mut self, rate: f32, fast: bool, finish: bool) -> Result<()> {
        ensure!(rate.is_finite(), "sequence rate nonfinite");
        for lane in self.status.lanes.values_mut() {
            if lane.records.is_empty() {
                lane.active = None;
                continue;
            }
            if lane.active.is_none() {
                lane.cursor = 0;
                while lane.cursor < 100 && !lane.records.contains_key(&(lane.cursor as u8)) {
                    lane.cursor += 1;
                }
                if lane.cursor == 100 {
                    continue;
                }
                let record = &lane.records[&(lane.cursor as u8)];
                lane.active = Some(match &record.track {
                    SequenceTrack::Frames { .. } => {
                        record.progress.clone().unwrap_or(Active::Frames {
                            frame: 0,
                            counter: 0,
                            attached: true,
                        })
                    }
                    SequenceTrack::Linear {
                        channels, start, ..
                    } => {
                        if let Some(start) = start {
                            for (channel, value) in channels.iter().zip(start) {
                                self.status.values[usize::from(*channel)] = *value;
                            }
                        }
                        Active::Linear {
                            velocity: record.velocity.clone(),
                            multiplier: rate,
                        }
                    }
                    SequenceTrack::Oscillation {
                        channel,
                        start_degrees,
                        ..
                    } => Active::Oscillation {
                        phase: *start_degrees,
                        velocity: record.velocity[0],
                        baseline: self.status.values[usize::from(*channel)],
                        multiplier: rate,
                    },
                });
                lane.control = Some(record.clone());
                lane.last_record = Some(record.clone());
                lane.last_active = lane.active.clone();
                // Activation is one real external tick; it does not move yet.
                continue;
            }
            let Some(map_record) = lane.records.get(&(lane.cursor as u8)) else {
                lane.active = None;
                lane.control = None;
                continue;
            };
            let record = lane.control.clone().unwrap_or_else(|| map_record.clone());
            let map_looping = match &map_record.track {
                SequenceTrack::Linear { looping, .. }
                | SequenceTrack::Oscillation { looping, .. }
                | SequenceTrack::Frames { looping, .. } => *looping,
            };
            let done = match (&record.track, lane.active.as_mut().unwrap()) {
                (
                    SequenceTrack::Frames {
                        channel,
                        frames,
                        hold_ticks,
                        looping: _,
                        ..
                    },
                    Active::Frames {
                        frame,
                        counter,
                        attached,
                    },
                ) => {
                    if *frames == 0 {
                        ensure!(!finish, "dormant frame record cannot be force-completed");
                        false
                    } else {
                        if finish || i64::from(*counter) > i64::from(*hold_ticks) {
                            *frame = frame.saturating_add(1);
                            *counter = 0;
                            if finish || *frame >= *frames {
                                if map_looping && !finish {
                                    *frame = 0;
                                } else {
                                    *attached = false;
                                }
                            }
                            self.status.values[usize::from(*channel)] =
                                if *attached { *frame as f32 } else { -1. };
                        }
                        *counter = counter
                            .checked_add(1)
                            .context("sequence frame counter overflow")?;
                        !*attached
                    }
                }
                (
                    SequenceTrack::Linear {
                        channels,
                        target,
                        looping: _,
                        complete_first,
                        complete_zero,
                        fast: allow_fast,
                        fast_channels,
                        ..
                    },
                    Active::Linear {
                        velocity,
                        multiplier,
                    },
                ) => {
                    let mut reached = Vec::new();
                    for ((channel, target), step) in
                        channels.iter().zip(target).zip(velocity.iter_mut())
                    {
                        *step = ieee_multiply(*step, *multiplier);
                        let delta = if fast
                            && *allow_fast
                            && fast_channels
                                .as_ref()
                                .is_none_or(|selected| selected.contains(channel))
                        {
                            ieee_multiply(*step, 4.)
                        } else {
                            *step
                        };
                        let current = &mut self.status.values[usize::from(*channel)];
                        *current = ieee_add(*current, delta);
                        if *current >= *target - delta.abs() && *current <= *target + delta.abs() {
                            *current = *target;
                        }
                        reached.push(*current == *target);
                    }
                    let done = finish
                        || if *complete_first {
                            reached[0]
                        } else {
                            reached.iter().all(|v| *v)
                        }
                        || (*complete_zero && velocity.iter().all(|v| *v == 0.));
                    if done {
                        let values = if map_looping && !finish {
                            &map_record.origin
                        } else {
                            target
                        };
                        for (channel, value) in channels.iter().zip(values) {
                            self.status.values[usize::from(*channel)] = *value;
                        }
                    }
                    done && (!map_looping || finish)
                }
                (
                    SequenceTrack::Oscillation {
                        channel,
                        amplitude,
                        start_degrees,
                        end_degrees,
                        cosine,
                        looping: _,
                        radians_per_degree,
                        fast: allow_fast,
                        ..
                    },
                    Active::Oscillation {
                        phase,
                        velocity,
                        baseline,
                        multiplier,
                    },
                ) => {
                    *velocity = (*velocity as f64 * f64::from(*multiplier)) as f32;
                    let step = if fast && *allow_fast {
                        (*velocity as f64 * 4.) as f32
                    } else {
                        *velocity
                    };
                    *phase = (*phase as f64 + f64::from(step)) as f32;
                    if *phase >= *end_degrees - step.abs() && *phase <= *end_degrees + step.abs() {
                        *phase = *end_degrees;
                    }
                    let done = finish || *phase == *end_degrees;
                    let evaluate = |angle: f32| {
                        let radians = (f64::from(angle) * radians_per_degree) as f32;
                        let wave = if *cosine {
                            f64::from(radians).cos()
                        } else {
                            f64::from(radians).sin()
                        } as f32;
                        (f64::from(wave) * f64::from(*amplitude) + f64::from(*baseline)) as f32
                    };
                    self.status.values[usize::from(*channel)] = if done {
                        if map_looping && !finish {
                            let (start, end) = match &map_record.track {
                                SequenceTrack::Oscillation {
                                    start_degrees,
                                    end_degrees,
                                    ..
                                } => (*start_degrees, *end_degrees),
                                _ => anyhow::bail!("sequence restored oscillator map kind"),
                            };
                            *phase = start;
                            if let Some(Record {
                                track:
                                    SequenceTrack::Oscillation {
                                        start_degrees,
                                        end_degrees,
                                        ..
                                    },
                                ..
                            }) = &mut lane.control
                            {
                                *start_degrees = start;
                                *end_degrees = end;
                            }
                            evaluate(*phase)
                        } else {
                            *baseline
                        }
                    } else {
                        evaluate(*phase)
                    };
                    done && (!map_looping || finish)
                }
                _ => anyhow::bail!("sequence active lane kind mismatch"),
            };
            ensure!(
                finite(&self.status.values),
                "sequence generated nonfinite channel"
            );
            if matches!(lane.active, Some(Active::Frames { .. })) {
                if let Some(current) = lane.records.get_mut(&(lane.cursor as u8)) {
                    current.progress = lane.active.clone();
                }
                if let Some(control) = &mut lane.control {
                    control.progress = lane.active.clone();
                }
            }
            lane.last_record = lane.control.clone();
            lane.last_active = lane.active.clone();
            if done {
                lane.records.remove(&(lane.cursor as u8));
                lane.active = None;
            }
        }
        if self.status.forever
            && self.status.lanes.values().all(|l| l.records.is_empty())
            && !finish
        {
            self.status.repeats = 1;
            self.replay()?;
        }
        self.status.complete = self.status.lanes.values().all(|l| l.records.is_empty());
        Ok(())
    }
    fn samples(&self) -> Result<Vec<(u32, BindingTarget, f32)>> {
        self.spec
            .bindings
            .iter()
            //Keep undefined source geometry in CPU state, never in finite
            //layer/GPU properties. Forced finish publishes its finite target.
            .filter(|binding| {
                let value=self.status.values[usize::from(binding.channel)];
                value.is_finite() && match &binding.map {
                    ValueMap::ZeroFallback {channel,..} if value==0. =>self.status.values[usize::from(*channel)].is_finite(),
                    ValueMap::CenteredScale{scale_channel,..}=>self.status.values[usize::from(*scale_channel)].is_finite(),
                    _=>true,
                }
            })
            .filter(|binding|match &binding.target {
                BindingTarget::ImageUv{active_lane:Some(lane),..}|BindingTarget::TexelImageUv{active_lane:Some(lane),..}=>self.status.lanes.get(lane).and_then(|lane|lane.control.as_ref().or(lane.last_record.as_ref())).is_some_and(|record|matches!(record.track,SequenceTrack::Frames{channel,..} if channel==binding.channel)),
                _=>true,
            })
            .map(|binding| {
                let value = self.status.values[usize::from(binding.channel)];
                let value = match &binding.map {
                    ValueMap::Identity => value,
                    ValueMap::Affine { scale, offset } => {
                        (f64::from(value) * f64::from(*scale) + f64::from(*offset)) as f32
                    }
                    ValueMap::Ratio {
                        test_channels,
                        threshold,
                        below,
                        above,
                    } => {
                        value
                            / if test_channels
                                .iter()
                                .all(|n| self.status.values[usize::from(*n)] >= *threshold)
                            {
                                *above
                            } else {
                                *below
                            }
                    }
                    ValueMap::CenteredScale{source_extent,reference_extent,scale_channel,test_channels,threshold,below,above}=>{
                        let divisor=if test_channels.iter().all(|channel|self.status.values[usize::from(*channel)]>=*threshold){*above}else{*below};
                        (f64::from(value)-(f64::from(*reference_extent)-f64::from(*source_extent)*f64::from(self.status.values[usize::from(*scale_channel)])/f64::from(divisor))*0.5) as f32
                    },
                    ValueMap::ZeroFallback { channel, scale } => {
                        if value == 0. {
                            self.status.values[usize::from(*channel)] * *scale
                        } else {
                            value
                        }
                    }
                    ValueMap::Clamp { low, high, scale } => value.clamp(*low, *high) * *scale,
                };
                ensure!(value.is_finite(), "sequence nonfinite binding");
                Ok((binding.layer, binding.target.clone(), value))
            })
            .collect()
    }
}
impl Sequences {
    pub(super) fn begin(
        &mut self,
        client: u32,
        id: u32,
        spec: SequenceSpec,
    ) -> Result<Vec<(u32, BindingTarget, f32)>> {
        spec.validate()?;
        ensure!(
            self.clients.get(&client).map_or(0, BTreeMap::len) < 2048,
            "sequence actor budget"
        );
        let generation = NEXT_GENERATION
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| anyhow::anyhow!("sequence generation exhausted"))?;
        let values = spec.values.clone();
        let retained = spec.retained.clone().unwrap_or_else(|| values.clone());
        let actor = Actor {
            spec,
            status: SequenceStatus {
                generation,
                revision: 0,
                values: values.clone(),
                retained,
                lanes: BTreeMap::new(),
                capture: false,
                captured: BTreeMap::new(),
                repeats: 0,
                forever: false,
                complete: true,
            },
        };
        let samples = actor.samples()?;
        self.clients.entry(client).or_default().insert(id, actor);
        Ok(samples)
    }
    pub(super) fn operate(
        &mut self,
        client: u32,
        id: u32,
        operation: SequenceOperation,
    ) -> Result<Vec<(u32, BindingTarget, f32)>> {
        let actor = self
            .clients
            .get_mut(&client)
            .and_then(|c| c.get_mut(&id))
            .context("sequence actor missing")?;
        match operation {
            SequenceOperation::RestoreControl {
                state,
                seed_missing,
            } => {
                let mut owned = ieee_channels(&actor.status);
                owned.extend(ieee_channels(&state));
                ensure!(
                    state.values.len() == actor.status.values.len()
                        && state.retained.len() == state.values.len()
                        && finite_or_owned(&state.values, &owned)
                        && finite_or_owned(&state.retained, &owned),
                    "sequence restored control channels"
                );
                ensure!(
                    state.lanes.len() <= MAX_LANES
                        && state
                            .lanes
                            .keys()
                            .all(|lane| usize::from(*lane) < MAX_LANES)
                        && seed_missing.len() <= MAX_LANES
                        && seed_missing
                            .iter()
                            .all(|lane| usize::from(*lane) < MAX_LANES),
                    "sequence restored control lanes"
                );
                actor.status.values = state.values;
                actor.status.retained = state.retained;
                let mut restored = state.lanes;
                for id in restored.keys().copied().collect::<Vec<_>>() {
                    actor.status.lanes.entry(id).or_default();
                }
                for (id, lane) in &mut actor.status.lanes {
                    let Some(snapshot) = restored.remove(id) else {
                        lane.cursor = 0;
                        lane.active = None;
                        lane.control = None;
                        lane.last_record = None;
                        lane.last_active = None;
                        continue;
                    };
                    ensure!(snapshot.cursor <= 100, "sequence restored cursor range");
                    let control = snapshot
                        .control
                        .or_else(|| snapshot.last_record.clone())
                        .or_else(|| snapshot.records.get(&(snapshot.cursor as u8)).cloned());
                    if let Some(control) = &control {
                        control.validate(actor.status.values.len())?;
                    }
                    if let Some(active) = &snapshot.active {
                        let control = control
                            .as_ref()
                            .context("sequence restored active control missing")?;
                        match (active, &control.track) {
                            (
                                Active::Linear {
                                    velocity,
                                    multiplier,
                                },
                                SequenceTrack::Linear { channels, .. },
                            ) => ensure!(
                                velocity.len() == channels.len()
                                    && (finite(velocity) || ieee_track(&control.track))
                                    && multiplier.is_finite(),
                                "sequence restored active velocity"
                            ),
                            (
                                Active::Oscillation {
                                    phase,
                                    velocity,
                                    baseline,
                                    multiplier,
                                },
                                SequenceTrack::Oscillation { .. },
                            ) => ensure!(
                                finite(&[*phase, *velocity, *baseline, *multiplier]),
                                "sequence restored active oscillator"
                            ),
                            (
                                Active::Frames { frame, counter, .. },
                                SequenceTrack::Frames { frames, .. },
                            ) => ensure!(
                                *frame <= *frames && *counter <= i32::MAX as u32 + 1,
                                "sequence restored frame state"
                            ),
                            _ => anyhow::bail!("sequence restored active/control kind mismatch"),
                        }
                    }
                    if snapshot.active.is_some() && seed_missing.contains(id) {
                        let dummy = control
                            .as_ref()
                            .context("sequence neutral record role missing")?
                            .neutral()?;
                        lane.records.entry(0).or_insert(dummy);
                    }
                    if snapshot.active.is_some() {
                        if let (Some(control), Some(current)) =
                            (&control, lane.records.get(&(snapshot.cursor as u8)))
                        {
                            let same_role = match (&control.track, &current.track) {
                                (
                                    SequenceTrack::Linear { channels: a, .. },
                                    SequenceTrack::Linear { channels: b, .. },
                                ) => a == b,
                                (
                                    SequenceTrack::Oscillation { channel: a, .. },
                                    SequenceTrack::Oscillation { channel: b, .. },
                                )
                                | (
                                    SequenceTrack::Frames { channel: a, .. },
                                    SequenceTrack::Frames { channel: b, .. },
                                ) => a == b,
                                _ => false,
                            };
                            ensure!(
                                same_role,
                                "sequence restored control/map channel role mismatch"
                            );
                        }
                    }
                    lane.cursor = snapshot.cursor;
                    lane.active = snapshot.active;
                    lane.control = control;
                    lane.last_record = snapshot.last_record;
                    lane.last_active = snapshot.last_active;
                }
                actor.status.complete = actor
                    .status
                    .lanes
                    .values()
                    .all(|lane| lane.records.is_empty() && lane.active.is_none());
            }
            SequenceOperation::Reset { values, retained } => {
                ensure!(
                    values.len() == actor.status.values.len()
                        && retained.len() == values.len()
                        && finite(&values)
                        && finite(&retained),
                    "sequence reset channel schema"
                );
                actor.status.values = values;
                actor.status.retained = retained;
                for lane in actor.status.lanes.values_mut() {
                    lane.cursor = 0;
                    lane.active = None;
                    lane.control = None;
                    lane.last_record = None;
                    lane.last_active = None;
                }
                actor.status.complete = actor
                    .status
                    .lanes
                    .values()
                    .all(|lane| lane.records.is_empty());
            }
            SequenceOperation::Enqueue { lane, bank, track } => {
                actor.enqueue(lane, bank, track)?;
                actor.status.complete = actor.status.lanes.values().all(|l| l.records.is_empty());
            }
            SequenceOperation::Set {
                channels,
                values,
                retained,
            } => {
                ensure!(
                    channels.len() == values.len()
                        && finite(&values)
                        && channels
                            .iter()
                            .all(|n| usize::from(*n) < actor.status.values.len()),
                    "sequence set channel/scalars"
                );
                for (channel, value) in channels.into_iter().zip(values) {
                    actor.status.values[usize::from(channel)] = value;
                    if retained {
                        actor.status.retained[usize::from(channel)] = value;
                    }
                }
            }
            SequenceOperation::SetBindingMap{layer,channel,map}=>{
                let mut spec=actor.spec.clone();let mut found=false;
                for binding in &mut spec.bindings {if binding.layer==layer&&binding.channel==channel{binding.map=map.clone();found=true;}}
                ensure!(found,"sequence projection binding missing");spec.validate()?;actor.spec=spec;
            },
            SequenceOperation::SetTexelSource {layer,source}=>{
                ensure!(source.iter().all(|value|value.is_finite()),"sequence texel source scalars");
                let mut found=false;
                for binding in &mut actor.spec.bindings {
                    if binding.layer==layer {
                        if let BindingTarget::TexelImageUv{source:current,..}=&mut binding.target {*current=source;found=true;}
                    }
                }
                ensure!(found,"sequence sampled image binding missing");
            },
            SequenceOperation::Capture {
                enabled,
                repeats,
                forever,
            } => {
                ensure!(repeats <= 1000, "sequence replay budget");
                actor.status.capture = enabled;
                actor.status.repeats = repeats;
                actor.status.forever = forever;
                if enabled {
                    actor.status.captured.clear();
                }
            }
            SequenceOperation::Replay => actor.replay()?,
            SequenceOperation::Advance {
                generation,
                revision,
                rate,
                fast,
            } => {
                ensure!(
                    generation == actor.status.generation
                        && revision
                            == actor
                                .status
                                .revision
                                .checked_add(1)
                                .context("sequence revision exhausted")?,
                    "sequence stale/foreign revision"
                );
                actor.tick(rate, fast, false)?;
                actor.status.revision = revision;
            }
            SequenceOperation::Finish { cancel, lanes } => {
                let mut held = BTreeMap::new();
                if let Some(selected) = &lanes {
                    ensure!(selected.len() <= MAX_LANES, "sequence finish lane budget");
                    for id in actor
                        .status
                        .lanes
                        .keys()
                        .copied()
                        .filter(|id| !selected.contains(id))
                        .collect::<Vec<_>>()
                    {
                        held.insert(id, actor.status.lanes.remove(&id).unwrap());
                    }
                }
                if cancel {
                    actor.status.lanes.clear();
                } else {
                    for _ in 0..MAX_RECORDS * 2 + 2 {
                        if actor.status.lanes.values().all(|l| l.records.is_empty()) {
                            break;
                        }
                        let rate = actor
                            .spec
                            .rate_channel
                            .map_or(1., |n| actor.status.values[usize::from(n)]);
                        actor.tick(rate, false, true)?;
                    }
                    ensure!(
                        actor.status.lanes.values().all(|l| l.records.is_empty()),
                        "sequence finish did not drain bounded banks"
                    );
                }
                actor.status.lanes.extend(held);
                if lanes.is_none() {
                    actor.status.captured.clear();
                    actor.status.capture = false;
                    actor.status.repeats = 0;
                    actor.status.forever = false;
                }
                actor.status.complete = actor.status.lanes.values().all(|l| l.records.is_empty());
            }
        }
        actor.samples()
    }
    pub(super) fn preview(
        &self,
        client: u32,
        id: u32,
        rate: f32,
        fast: bool,
    ) -> Result<Option<SequenceStatus>> {
        let Some(mut actor) = self
            .clients
            .get(&client)
            .and_then(|actors| actors.get(&id))
            .cloned()
        else {
            return Ok(None);
        };
        actor.tick(rate, fast, false)?;
        actor.status.revision = actor
            .status
            .revision
            .checked_add(1)
            .context("sequence revision exhausted")?;
        Ok(Some(actor.status))
    }
    /// Undefined IEEE inputs remain in their explicit CPU track profile. A
    /// bound subtree must not reuse an earlier finite projection while those
    /// inputs are undefined; finite completion automatically admits it again.
    pub(super) fn suppressed_layers(&self, client:u32)->BTreeSet<u32> {
        self.clients.get(&client).into_iter().flat_map(|actors|actors.values()).flat_map(|actor| {
            actor.spec.bindings.iter().filter_map(move |binding| {
                let value=actor.status.values[usize::from(binding.channel)];
                let undefined=!value.is_finite()&&actor.status.allows_nonfinite_channel(binding.channel)
                    ||matches!(&binding.map,ValueMap::ZeroFallback{channel,..} if value==0.&&!actor.status.values[usize::from(*channel)].is_finite()&&actor.status.allows_nonfinite_channel(*channel));
                let undefined=undefined||matches!(&binding.map,ValueMap::CenteredScale{scale_channel,..} if !actor.status.values[usize::from(*scale_channel)].is_finite()&&actor.status.allows_nonfinite_channel(*scale_channel));
                let empty=if let BindingTarget::TexelImageUv{allocation,reference,source,step,terminal_frame,active_lane}=&binding.target {
                    let admitted=active_lane.is_none_or(|lane|actor.status.lanes.get(&lane).and_then(|lane|lane.control.as_ref().or(lane.last_record.as_ref())).is_some_and(|record|matches!(record.track,SequenceTrack::Frames{channel,..} if channel==binding.channel)));
                    if admitted&&value.is_finite()&&(value>=0.||terminal_frame.is_some()) {
                        let frame=if value<0.{terminal_frame.unwrap() as f32}else{value.trunc()};
                        let shifted=[source[0]+frame*step[0],source[1]+frame*step[1],source[2]+frame*step[0],source[3]+frame*step[1]];
                        super::crop::texel_center_quad(*allocation,*reference,shifted,[0.,0.]).map_or(true,|quad|quad.is_none())
                    }else{false}
                }else{false};
                (undefined||empty).then_some(binding.layer)
            })
        }).collect()
    }
    pub(super) fn status(&self, client: u32, id: u32) -> Option<SequenceStatus> {
        Some(self.clients.get(&client)?.get(&id)?.status.clone())
    }
    pub(super) fn cancel_owner(&mut self, client: u32, id: u32) {
        if let Some(actors) = self.clients.get_mut(&client) {
            actors.remove(&id);
        }
    }
    pub(super) fn cancel(&mut self, client: u32, removed: &BTreeSet<u32>) {
        if let Some(actors) = self.clients.get_mut(&client) {
            actors.retain(|id, actor| {
                !removed.contains(id)
                    && actor
                        .spec
                        .bindings
                        .iter()
                        .all(|b| !removed.contains(&b.layer))
            });
        }
    }
    pub(super) fn validate(&self, exists: impl Fn(u32, u32) -> bool) -> Result<()> {
        for (client, actors) in &self.clients {
            ensure!(actors.len() <= 2048, "sequence actor budget");
            for (id, actor) in actors {
                ensure!(exists(*client, *id), "sequence owner layer missing");
                actor.spec.validate()?;
                ensure!(
                    actor.spec.bindings.iter().all(|b| exists(*client, b.layer)),
                    "sequence bound layer missing"
                );
                ensure!(
                    actor.status.values.len() == actor.spec.values.len()
                        && actor.status.retained.len() == actor.spec.values.len()
                        && finite_or_owned(&actor.status.values, &ieee_channels(&actor.status))
                        && finite_or_owned(&actor.status.retained, &ieee_channels(&actor.status))
                        && actor.count() <= MAX_RECORDS,
                    "sequence state malformed"
                );
                ensure!(
                    actor.status.generation != 0
                        && actor.status.lanes.len() <= MAX_LANES
                        && actor.status.captured.len() <= MAX_LANES,
                    "sequence identity/lane budget"
                );
                for (lane_id, lane) in &actor.status.lanes {
                    ensure!(
                        usize::from(*lane_id) < MAX_LANES && lane.cursor <= 100,
                        "sequence lane cursor malformed"
                    );
                    for (bank, record) in &lane.records {
                        ensure!(*bank <= 100, "sequence bank budget");
                        record.validate(actor.status.values.len())?;
                    }
                    if let Some(record) = &lane.control {
                        record.validate(actor.status.values.len())?;
                    }
                    if let Some(record) = &lane.last_record {
                        record.validate(actor.status.values.len())?;
                    }
                    for active in lane.active.iter().chain(lane.last_active.iter()) {
                        match active {
                            Active::Frames { frame, counter, .. } => ensure!(
                                *frame <= 1_000_000 && *counter <= i32::MAX as u32 + 1,
                                "sequence frame active bounds"
                            ),
                            Active::Linear {
                                velocity,
                                multiplier,
                            } => ensure!(
                                !velocity.is_empty()
                                    && (finite(velocity)
                                        || lane
                                            .control
                                            .as_ref()
                                            .or(lane.last_record.as_ref())
                                            .is_some_and(|record| ieee_track(&record.track)))
                                    && multiplier.is_finite(),
                                "sequence active velocity malformed"
                            ),
                            Active::Oscillation {
                                phase,
                                velocity,
                                baseline,
                                multiplier,
                            } => ensure!(
                                finite(&[*phase, *velocity, *baseline, *multiplier]),
                                "sequence active phase malformed"
                            ),
                        }
                    }
                    if lane.active.is_some() {
                        ensure!(
                            lane.records.contains_key(&(lane.cursor as u8)),
                            "sequence active record missing"
                        );
                    }
                }
                for (lane, records) in &actor.status.captured {
                    ensure!(
                        usize::from(*lane) < MAX_LANES,
                        "sequence captured lane budget"
                    );
                    for (bank, track) in records {
                        ensure!(*bank <= 100, "sequence captured bank budget");
                        track.validate(actor.status.values.len())?;
                    }
                }
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::{LayerCommand, LayerSpec, LayerSystem};
    use crysirror_core::state::ClientId;
    fn linear(to: f32, looping: bool) -> SequenceTrack {
        serde_json::from_value(serde_json::json!({"kind":"linear","channels":[0],"target":[to],"ticks":4,"references":[0],"loop_origins":[0],"delta":"retained","looping":looping,"complete_first":true,"complete_zero":true,"fast":true})).unwrap()
    }
    fn setup() -> Sequences {
        let mut sequences = Sequences::default();
        sequences
            .begin(
                7,
                1,
                SequenceSpec {
                    values: vec![0.],
                    retained: None,
                    rate_channel: None,
                    bindings: vec![],
                },
            )
            .unwrap();
        sequences
    }
    fn tick(sequences: &mut Sequences) {
        let status = sequences.status(7, 1).unwrap();
        sequences
            .operate(
                7,
                1,
                SequenceOperation::Advance {
                    generation: status.generation,
                    revision: status.revision + 1,
                    rate: 1.,
                    fast: false,
                },
            )
            .unwrap();
    }
    #[test]
    fn dependency_preview_is_scoped_and_never_commits_the_owner_tick() {
        let mut sequences = setup();
        sequences
            .operate(
                7,
                1,
                SequenceOperation::Enqueue {
                    lane: 0,
                    bank: None,
                    track: linear(8., false),
                },
            )
            .unwrap();
        for _ in 0..5 {
            let before = serde_json::to_value(sequences.status(7, 1).unwrap()).unwrap();
            let preview = sequences.preview(7, 1, 1., false).unwrap().unwrap();
            assert_eq!(
                serde_json::to_value(sequences.status(7, 1).unwrap()).unwrap(),
                before
            );
            assert!(sequences.preview(8, 1, 1., false).unwrap().is_none());
            tick(&mut sequences);
            assert_eq!(
                serde_json::to_value(sequences.status(7, 1).unwrap()).unwrap(),
                serde_json::to_value(preview).unwrap()
            );
        }
    }
    #[test]
    fn restored_control_keeps_current_bank_loop_metadata_and_future_records() {
        let mut source = setup();
        source
            .operate(
                7,
                1,
                SequenceOperation::Enqueue {
                    lane: 0,
                    bank: None,
                    track: linear(8., false),
                },
            )
            .unwrap();
        tick(&mut source);
        tick(&mut source);
        let snapshot = source.status(7, 1).unwrap();
        let mut current = setup();
        current
            .operate(
                7,
                1,
                SequenceOperation::Set {
                    channels: vec![0],
                    values: vec![2.],
                    retained: true,
                },
            )
            .unwrap();
        current
            .operate(
                7,
                1,
                SequenceOperation::Enqueue {
                    lane: 0,
                    bank: None,
                    track: linear(32., true),
                },
            )
            .unwrap();
        current
            .operate(
                7,
                1,
                SequenceOperation::Enqueue {
                    lane: 0,
                    bank: None,
                    track: linear(64., false),
                },
            )
            .unwrap();
        current
            .operate(
                7,
                1,
                SequenceOperation::RestoreControl {
                    state: snapshot.clone(),
                    seed_missing: vec![0],
                },
            )
            .unwrap();
        let status = current.status(7, 1).unwrap();
        assert_eq!(status.lanes[&0].records.len(), 2);
        assert!(
            matches!(&status.lanes[&0].records[&0].track,SequenceTrack::Linear{target,looping:true,..} if target==&[32.])
        );
        tick(&mut current);
        assert_eq!(current.status(7, 1).unwrap().values, [4.]);
        tick(&mut current);
        // Saved target8 completes, but the current map's loop=true/origin2
        // controls completion; neither its32 target nor future64 is replaced.
        let status = current.status(7, 1).unwrap();
        assert_eq!(status.values, [2.]);
        assert_eq!(status.lanes[&0].records.len(), 2);
        let mut empty = setup();
        empty
            .operate(
                7,
                1,
                SequenceOperation::RestoreControl {
                    state: snapshot,
                    seed_missing: vec![0],
                },
            )
            .unwrap();
        assert!(
            matches!(&empty.status(7,1).unwrap().lanes[&0].records[&0].track,SequenceTrack::Linear{target,looping:false,..} if target==&[0.])
        );
        tick(&mut empty);
        tick(&mut empty);
        assert_eq!(empty.status(7, 1).unwrap().values, [8.]);
        assert!(empty.status(7, 1).unwrap().complete);
    }
    #[test]
    fn value_reset_restarts_existing_record_without_discarding_its_delta() {
        let mut s = setup();
        s.operate(
            7,
            1,
            SequenceOperation::Enqueue {
                lane: 0,
                bank: None,
                track: linear(8., false),
            },
        )
        .unwrap();
        tick(&mut s);
        tick(&mut s);
        assert_eq!(s.status(7, 1).unwrap().values, [2.]);
        s.operate(
            7,
            1,
            SequenceOperation::Reset {
                values: vec![0.],
                retained: vec![0.],
            },
        )
        .unwrap();
        let status = s.status(7, 1).unwrap();
        assert_eq!(status.lanes[&0].records.len(), 1);
        assert!(status.lanes[&0].active.is_none());
        assert!(status.lanes[&0].last_record.is_none());
        tick(&mut s);
        assert_eq!(s.status(7, 1).unwrap().values, [0.]);
        tick(&mut s);
        assert_eq!(s.status(7, 1).unwrap().values, [2.]);
    }
    #[test]
    fn frame_clock_uses_strict_hold_and_ignores_scalar_rate() {
        let mut s = setup();
        s.operate(
            7,
            1,
            SequenceOperation::Enqueue {
                lane: 9,
                bank: None,
                track: SequenceTrack::Frames {
                    channel: 0,
                    frames: 2,
                    hold_ticks: 1,
                    looping: false,
                    capture: false,
                },
            },
        )
        .unwrap();
        let mut actual = Vec::new();
        for _ in 0..7 {
            let status = s.status(7, 1).unwrap();
            s.operate(
                7,
                1,
                SequenceOperation::Advance {
                    generation: status.generation,
                    revision: status.revision + 1,
                    rate: 100.,
                    fast: true,
                },
            )
            .unwrap();
            actual.push(s.status(7, 1).unwrap().values[0]);
        }
        assert_eq!(actual, [0., 0., 0., 1., 1., -1., -1.]);
        assert!(s.status(7, 1).unwrap().complete);
        assert!(matches!(
            s.status(7, 1).unwrap().lanes[&9].last_active,
            Some(Active::Frames {
                attached: false,
                ..
            })
        ));
    }
    #[test]
    fn external_binding_survives_target_payload_replace_but_not_ownership_loss() {
        let client = ClientId(7);
        let mut layers = LayerSystem::default();
        let layer = |id| {
            serde_json::from_value::<LayerSpec>(serde_json::json!({"id":id,"rect":[0,0,8,8],"paint":{"kind":"solid","color":[0,0,0,0]}})).unwrap()
        };
        layers
            .apply_batch(
                client,
                &[
                    LayerCommand::Put { layer: layer(1) },
                    LayerCommand::Put { layer: layer(2) },
                    LayerCommand::BeginSequence {
                        id: 1,
                        spec: SequenceSpec {
                            values: vec![0.],
                            retained: None,
                            rate_channel: None,
                            bindings: vec![SequenceBinding {
                                layer: 2,
                                channel: 0,
                                target: BindingTarget::Motion {
                                    property: MotionProperty::X,
                                },
                                map: ValueMap::Identity,
                            }],
                        },
                    },
                    LayerCommand::Sequence {
                        id: 1,
                        operation: SequenceOperation::Enqueue {
                            lane: 0,
                            bank: None,
                            track: linear(8., false),
                        },
                    },
                ],
            )
            .unwrap();
        let advance = |layers: &mut LayerSystem| {
            let status = layers.sequence_status(client, 1).unwrap().unwrap();
            layers
                .apply_batch(
                    client,
                    &[LayerCommand::Sequence {
                        id: 1,
                        operation: SequenceOperation::Advance {
                            generation: status.generation,
                            revision: status.revision + 1,
                            rate: 1.,
                            fast: false,
                        },
                    }],
                )
                .unwrap();
        };
        advance(&mut layers); // activation
        layers
            .apply_batch(client, &[LayerCommand::Put { layer: layer(2) }])
            .unwrap();
        assert!(layers.sequence_status(client, 1).unwrap().is_some());
        advance(&mut layers);
        assert_eq!(layers.get(client, 2).unwrap().rect[0], 2.);
        layers
            .apply_batch(client, &[LayerCommand::Put { layer: layer(1) }])
            .unwrap();
        assert!(layers.sequence_status(client, 1).unwrap().is_none());
        layers
            .apply_batch(
                client,
                &[
                    LayerCommand::BeginSequence {
                        id: 1,
                        spec: SequenceSpec {
                            values: vec![0.],
                            retained: None,
                            rate_channel: None,
                            bindings: vec![SequenceBinding {
                                layer: 2,
                                channel: 0,
                                target: BindingTarget::Motion {
                                    property: MotionProperty::X,
                                },
                                map: ValueMap::Identity,
                            }],
                        },
                    },
                    LayerCommand::Remove { id: 2 },
                ],
            )
            .unwrap();
        assert!(layers.sequence_status(client, 1).unwrap().is_none());
    }
    #[test]
    fn activation_and_native_incremental_snap_have_independent_tick_oracle() {
        let mut s = setup();
        s.operate(
            7,
            1,
            SequenceOperation::Enqueue {
                lane: 0,
                bank: None,
                track: linear(8., false),
            },
        )
        .unwrap();
        let mut values = Vec::new();
        for _ in 0..4 {
            tick(&mut s);
            values.push(s.status(7, 1).unwrap().values[0]);
        }
        assert_eq!(values, [0., 2., 4., 8.]);
        let status = s.status(7, 1).unwrap();
        assert!(status.complete);
        assert!(status.lanes[&0].last_record.is_some());
        assert!(status.lanes[&0].last_active.is_some());
    }
    #[test]
    fn loop_resets_origin_and_finish_drains_it_instead_of_hanging() {
        let mut s = setup();
        s.operate(
            7,
            1,
            SequenceOperation::Enqueue {
                lane: 0,
                bank: None,
                track: linear(8., true),
            },
        )
        .unwrap();
        for _ in 0..4 {
            tick(&mut s);
        }
        assert_eq!(s.status(7, 1).unwrap().values, [0.]);
        assert!(!s.status(7, 1).unwrap().complete);
        s.operate(
            7,
            1,
            SequenceOperation::Finish {
                cancel: false,
                lanes: None,
            },
        )
        .unwrap();
        assert_eq!(s.status(7, 1).unwrap().values, [8.]);
        assert!(s.status(7, 1).unwrap().complete);
    }
    #[test]
    fn captured_templates_recompute_velocity_and_stale_revision_cannot_advance() {
        let mut s = setup();
        s.operate(
            7,
            1,
            SequenceOperation::Capture {
                enabled: true,
                repeats: 0,
                forever: false,
            },
        )
        .unwrap();
        for to in [8., 16.] {
            s.operate(
                7,
                1,
                SequenceOperation::Enqueue {
                    lane: 0,
                    bank: None,
                    track: linear(to, false),
                },
            )
            .unwrap();
        }
        assert_eq!(s.status(7, 1).unwrap().retained, [0.]);
        s.operate(
            7,
            1,
            SequenceOperation::Capture {
                enabled: false,
                repeats: 1,
                forever: false,
            },
        )
        .unwrap();
        s.operate(7, 1, SequenceOperation::Replay).unwrap();
        assert_eq!(s.status(7, 1).unwrap().lanes[&0].records[&1].velocity, [2.]);
        for _ in 0..8 {
            tick(&mut s);
        }
        assert_eq!(s.status(7, 1).unwrap().values, [16.]);
        let status = s.status(7, 1).unwrap();
        assert!(
            s.operate(
                7,
                1,
                SequenceOperation::Advance {
                    generation: status.generation,
                    revision: status.revision,
                    rate: 1.,
                    fast: false
                }
            )
            .is_err()
        );
        assert_eq!(s.status(7, 1).unwrap().values, [16.]);
    }
    #[test]
    fn layer_owned_queue_snapshot_and_failed_batch_preserve_actual_progress() {
        let client = ClientId(7);
        let mut layers = LayerSystem::default();
        let spec:LayerSpec=serde_json::from_value(serde_json::json!({"id":1,"rect":[0,0,8,8],"paint":{"kind":"solid","color":[0,0,0,255]}})).unwrap();
        layers
            .apply_batch(
                client,
                &[
                    LayerCommand::Put { layer: spec },
                    LayerCommand::BeginSequence {
                        id: 1,
                        spec: SequenceSpec {
                            values: vec![0.],
                            retained: None,
                            rate_channel: None,
                            bindings: vec![SequenceBinding {
                                layer: 1,
                                channel: 0,
                                target: BindingTarget::Motion {
                                    property: MotionProperty::X,
                                },
                                map: ValueMap::Identity,
                            }],
                        },
                    },
                ],
            )
            .unwrap();
        layers
            .apply_batch(
                client,
                &[LayerCommand::Sequence {
                    id: 1,
                    operation: SequenceOperation::Enqueue {
                        lane: 0,
                        bank: None,
                        track: linear(8., false),
                    },
                }],
            )
            .unwrap();
        let status = layers.sequence_status(client, 1).unwrap().unwrap();
        let advance = LayerCommand::Sequence {
            id: 1,
            operation: SequenceOperation::Advance {
                generation: status.generation,
                revision: 1,
                rate: 1.,
                fast: false,
            },
        };
        assert!(
            layers
                .apply_batch(client, &[advance.clone(), LayerCommand::Remove { id: 99 }])
                .is_err()
        );
        assert_eq!(
            layers.sequence_status(client, 1).unwrap().unwrap().revision,
            0
        );
        layers.apply_batch(client, &[advance]).unwrap();
        let restored: LayerSystem =
            serde_json::from_slice(&serde_json::to_vec(&layers).unwrap()).unwrap();
        restored.validate().unwrap();
        assert_eq!(
            restored
                .sequence_status(client, 1)
                .unwrap()
                .unwrap()
                .revision,
            1
        );
        assert!(restored.sequence_status(ClientId(8), 1).is_err());
    }
}

impl Sequences {
    pub(super) fn retain_checkpoint_scope(&mut self, client:u32, ids:&BTreeSet<u32>) {
        self.clients.retain(|owner,layers| {if *owner!=client{return false} layers.retain(|id,_|ids.contains(id));!layers.is_empty()});
    }
    pub(super) fn restore_checkpoint_scope(&mut self, client:u32, removed:&BTreeSet<u32>, saved:&Self) {
        if let Some(owner)=self.clients.get_mut(&client){owner.retain(|id,_|!removed.contains(id));}
        if let Some(owner)=saved.clients.get(&client){self.clients.entry(client).or_default().extend(owner.clone());}
    }
}

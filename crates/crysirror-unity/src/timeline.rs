// SPDX-License-Identifier: MPL-2.0
//! Unity Timeline assets (`UnityEngine.Timeline.TimelineAsset`): the track
//! tree (`m_Tracks`, each `TrackAsset` with `m_Children`) and each track's
//! `TimelineClip`s (`m_Start`, `m_Duration`, `m_ClipIn`, `m_TimeScale`,
//! `m_Asset`). Track and clip asset semantics belong to their consumers;
//! this module only reads the structure and timing.
use crate::{managed::Fields, managed::Value, PPtr};
use anyhow::{Context, Result};

/// A `TimelineClip`.
#[derive(Clone, Debug, PartialEq)]
pub struct TimelineClip {
    pub start: f64,
    pub duration: f64,
    pub clip_in: f64,
    pub time_scale: f64,
    /// The clip's `PlayableAsset`.
    pub asset: PPtr,
    pub display_name: String,
}

impl TimelineClip {
    pub fn end(&self) -> f64 {
        self.start + self.duration
    }

    /// Whether timeline time `t` is inside the clip.
    pub fn contains(&self, t: f64) -> bool {
        t >= self.start && t < self.end()
    }

    /// The clip's local time at timeline time `t`.
    pub fn local(&self, t: f64) -> f64 {
        self.clip_in + (t - self.start) * self.time_scale
    }
}

/// A `TrackAsset`.
#[derive(Clone, Debug, PartialEq)]
pub struct TimelineTrack {
    pub name: String,
    /// The track's script class (`UnityEngine.Timeline.ActivationTrack`, …).
    pub script: String,
    pub muted: bool,
    pub clips: Vec<TimelineClip>,
    pub children: Vec<TimelineTrack>,
}

/// A `TimelineAsset`.
#[derive(Clone, Debug, PartialEq)]
pub struct Timeline {
    pub name: String,
    pub tracks: Vec<TimelineTrack>,
    /// `m_FixedDuration` when `m_DurationMode` is fixed (1).
    pub fixed_duration: Option<f64>,
}

fn float(f: &Fields, k: &str) -> f64 {
    match f.get(k) {
        Some(Value::Float(x)) => *x,
        Some(Value::Int(i)) => *i as f64,
        _ => 0.,
    }
}

fn pptrs(f: &Fields, k: &str) -> Vec<PPtr> {
    f.array(k).unwrap_or(&[]).iter().filter_map(|v| if let Value::PPtr(p) = v { Some(*p) } else { None }).collect()
}

/// Reads a track; `object` resolves a pointer to (script class, fields).
fn track(fields: &Fields, script: String, object: &mut dyn FnMut(PPtr) -> Option<(String, Fields)>, depth: usize) -> TimelineTrack {
    let clips = fields
        .array("m_Clips")
        .unwrap_or(&[])
        .iter()
        .filter_map(|v| {
            let Value::Struct(c) = v else { return None };
            Some(TimelineClip {
                start: float(c, "m_Start"),
                duration: float(c, "m_Duration"),
                clip_in: float(c, "m_ClipIn"),
                time_scale: match float(c, "m_TimeScale") {
                    s if s > 0. => s,
                    _ => 1.,
                },
                asset: c.pptr("m_Asset").unwrap_or_default(),
                display_name: c.string("m_DisplayName").unwrap_or("").to_owned(),
            })
        })
        .collect();
    let children = if depth < 16 {
        pptrs(fields, "m_Children").into_iter().filter_map(|p| object(p).map(|(s, f)| track(&f, s, object, depth + 1))).collect()
    } else {
        Vec::new()
    };
    TimelineTrack { name: fields.string("m_Name").unwrap_or("").to_owned(), script, muted: fields.int("m_Muted").unwrap_or(0) != 0, clips, children }
}

impl Timeline {
    /// Read a `TimelineAsset`; `object` resolves a track pointer to its
    /// script class and fields.
    pub fn decode(asset: &Fields, object: &mut dyn FnMut(PPtr) -> Option<(String, Fields)>) -> Result<Self> {
        let tracks = asset.array("m_Tracks").context("m_Tracks")?.iter().filter_map(|v| if let Value::PPtr(p) = v { Some(*p) } else { None }).filter_map(|p| object(p).map(|(s, f)| track(&f, s, object, 0))).collect();
        let fixed = (asset.int("m_DurationMode").unwrap_or(0) == 1).then(|| float(asset, "m_FixedDuration"));
        Ok(Self { name: asset.string("m_Name").unwrap_or("").to_owned(), tracks, fixed_duration: fixed })
    }

    /// Every track, depth first.
    pub fn all_tracks(&self) -> Vec<&TimelineTrack> {
        fn walk<'a>(t: &'a TimelineTrack, out: &mut Vec<&'a TimelineTrack>) {
            out.push(t);
            for c in &t.children {
                walk(c, out);
            }
        }
        let mut out = Vec::new();
        for t in &self.tracks {
            walk(t, &mut out);
        }
        out
    }

    /// `TimelineAsset.duration`: the fixed duration, else the last clip end
    /// of the unmuted tracks.
    pub fn duration(&self) -> f64 {
        self.fixed_duration.unwrap_or_else(|| self.all_tracks().into_iter().filter(|t| !t.muted).flat_map(|t| t.clips.iter().map(TimelineClip::end)).fold(0., f64::max))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(start: f64, duration: f64) -> Value {
        Value::Struct(Fields(vec![("m_Start".into(), Value::Float(start)), ("m_Duration".into(), Value::Float(duration)), ("m_TimeScale".into(), Value::Float(1.)), ("m_Asset".into(), Value::PPtr(PPtr { file_id: 0, path_id: 9 }))]))
    }

    #[test]
    fn tracks_and_duration() {
        let asset = Fields(vec![("m_Name".into(), Value::String("stm".into())), ("m_Tracks".into(), Value::Array(vec![Value::PPtr(PPtr { file_id: 0, path_id: 1 }), Value::PPtr(PPtr { file_id: 0, path_id: 2 })]))]);
        let mut object = |p: PPtr| -> Option<(String, Fields)> {
            match p.path_id {
                1 => Some(("A".into(), Fields(vec![("m_Name".into(), Value::String("Motion".into())), ("m_Clips".into(), Value::Array(vec![clip(1., 6.)])), ("m_Children".into(), Value::Array(vec![Value::PPtr(PPtr { file_id: 0, path_id: 3 })]))]))),
                2 => Some(("B".into(), Fields(vec![("m_Name".into(), Value::String("Muted".into())), ("m_Muted".into(), Value::Int(1)), ("m_Clips".into(), Value::Array(vec![clip(0., 20.)]))]))),
                3 => Some(("C".into(), Fields(vec![("m_Name".into(), Value::String("Child".into())), ("m_Clips".into(), Value::Array(vec![clip(2., 7.)]))]))),
                _ => None,
            }
        };
        let t = Timeline::decode(&asset, &mut object).unwrap();
        assert_eq!(t.all_tracks().iter().map(|t| t.name.as_str()).collect::<Vec<_>>(), ["Motion", "Child", "Muted"]);
        assert_eq!(t.duration(), 9.);
        let c = &t.tracks[0].clips[0];
        assert!(c.contains(1.) && !c.contains(7.));
        assert_eq!(c.local(3.), 2.);
    }
}

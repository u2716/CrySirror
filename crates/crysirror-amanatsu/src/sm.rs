// SPDX-License-Identifier: MPL-2.0
//! `AL.ShortMovie`: the short movies (`sm/t` `stm_NN` Unity Timelines over
//! the `sm/a` animators `stm_f_NN` (heroine), `stm_cam_NN` (camera) and
//! `stm_i_NN` (items)), played at the map's short movie point
//! (`MovePointInfo.GetShortMoviePoint`). The `AL.TimeLine` tracks used here:
//! `AnimatorTrack` `Motion` (heroine states by `_animationStateName`),
//! `AnimatorTrack` `Camera` (the camera's root position, rotation and
//! near/far/FOV curves, relative to the point) and `SceneFadeTrack`
//! (`SceneFadeInfo`: In covers, Out reveals, InOut both).
//!
//! SATD-amanatsu-port: Face, SE, ClothState, BasePos, FluidFlow, Wet,
//! TimeScale, Shake, Activation (lights, props), Control (clouds,
//! particles), Alembic and item animator tracks are not played; the skip
//! button is not ported.
use crate::unity::Unity;
use anyhow::{Context, Result};
use crysirror_unity::{
    animator::{Clip, Controller},
    timeline::Timeline,
};
use std::collections::HashMap;

pub const TIMELINES: &str = "lib/sm/t/000_00.unity3d";
pub const ANIMATORS: &str = "lib/sm/a/000_00.unity3d";

/// A state change of an animator track.
#[derive(Clone, Debug, PartialEq)]
pub struct StateClip {
    pub start: f64,
    pub end: f64,
    pub clip_in: f64,
    pub time_scale: f64,
    pub state: String,
    pub looping: bool,
}

/// `SceneFadeClip`.
#[derive(Clone, Debug, PartialEq)]
pub struct FadeClip {
    pub start: f64,
    pub duration: f64,
    /// `FadeCanvas.Fade`: 1 In (cover), 2 Out (reveal), 3 InOut.
    pub kind: i32,
    pub color: [f32; 3],
}

#[derive(Clone, Debug, Default)]
pub struct Movie {
    pub id: i32,
    pub heroine: Vec<StateClip>,
    pub camera: Vec<StateClip>,
    pub fades: Vec<FadeClip>,
    pub duration: f64,
}

impl Movie {
    /// `ShortMovie.Load`: the timeline `stm_NN`.
    pub fn load(unity: &mut Unity, id: i32) -> Result<Self> {
        let (key, oid) = unity.bundle_asset(TIMELINES, &format!("stm_{id:02}"), 114)?.with_context(|| format!("timeline stm_{id:02}"))?;
        let asset = unity.decode(&key, oid)?;
        let mut object = |p: crysirror_unity::PPtr| -> Option<(String, crysirror_unity::managed::Fields)> {
            let (f, o) = unity.target(&key, p).ok()?;
            let fields = unity.decode(&f, o).ok()?;
            let script = fields.pptr("m_Script").map(|s| unity.script_name(&f, s)).unwrap_or_default();
            Some((script, fields))
        };
        let timeline = Timeline::decode(&asset, &mut object)?;
        let mut movie = Movie { id, duration: timeline.duration(), ..Default::default() };
        for track in timeline.all_tracks().into_iter().filter(|t| !t.muted) {
            for c in &track.clips {
                let Some((_, fields)) = object(c.asset) else { continue };
                match track.script.as_str() {
                    "AL.TimeLine.AnimatorTrack" if track.name == "Motion" || track.name == "Camera" => {
                        let clip = StateClip { start: c.start, end: c.end(), clip_in: c.clip_in, time_scale: c.time_scale, state: fields.string("_animationStateName").unwrap_or("").to_owned(), looping: fields.int("_isLoop").unwrap_or(0) != 0 };
                        if track.name == "Motion" {
                            movie.heroine.push(clip);
                        } else {
                            movie.camera.push(clip);
                        }
                    }
                    "AL.TimeLine.SceneFadeTrack" => {
                        let info = fields.fields("_info");
                        let color = info.and_then(|i| i.fields("Color")).map_or([0.; 3], |c| ["r", "g", "b"].map(|k| c.float(k).unwrap_or(0.) as f32));
                        movie.fades.push(FadeClip { start: c.start, duration: info.and_then(|i| i.float("Duration")).unwrap_or(c.duration), kind: info.and_then(|i| i.int("FadeInOut")).unwrap_or(0) as i32, color });
                    }
                    _ => {}
                }
            }
        }
        for v in [&mut movie.heroine, &mut movie.camera] {
            v.sort_by(|a, b| a.start.total_cmp(&b.start));
        }
        movie.fades.sort_by(|a, b| a.start.total_cmp(&b.start));
        Ok(movie)
    }

    /// The heroine's distinct states.
    pub fn heroine_states(&self) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        for c in &self.heroine {
            if !v.contains(&c.state) {
                v.push(c.state.clone());
            }
        }
        v
    }

    /// The camera clips by state, with the heroine's height pick (the
    /// state's `_S`, `_M`, `_L` motions).
    pub fn camera_clips(&self, unity: &mut Unity, pick: usize) -> Result<HashMap<String, Clip>> {
        let (k, id) = unity.bundle_asset(ANIMATORS, &format!("stm_cam_{:02}", self.id), 91)?.context("camera animator")?;
        let ctl = Controller::decode(&unity.decode(&k, id)?)?;
        let mut out = HashMap::new();
        for c in &self.camera {
            if out.contains_key(&c.state) {
                continue;
            }
            let Some(si) = ctl.find_state(0, &c.state) else { continue };
            let Some(first) = ctl.layers[0].states[si].clip else { continue };
            let Some(&p) = ctl.clips.get(first + pick).or(ctl.clips.get(first)) else { continue };
            let (f, cid) = unity.target(&k, p)?;
            out.insert(c.state.clone(), Clip::decode(&unity.decode(&f, cid)?)?);
        }
        Ok(out)
    }
}

/// The camera of a camera clip at local time `t`: Unity-space position,
/// rotation and vertical FOV (the root's position and rotation; its scale
/// curves carry near, far and FOV).
pub fn camera_pose(clip: &Clip, t: f32) -> Option<([f32; 3], [f32; 4], Option<f32>)> {
    let values = clip.sample(t.clamp(clip.start, clip.stop));
    let (mut pos, mut rot, mut fov) = (None, None, None);
    let mut at = 0;
    for b in &clip.bindings {
        let v = values.get(at..at + b.curves)?;
        if b.path == 0 && b.class_id == 4 {
            match (b.attribute, b.curves) {
                (1, 3) => pos = Some([v[0], v[1], v[2]]),
                (2, 4) => rot = Some([v[0], v[1], v[2], v[3]]),
                (3, 3) => fov = Some(v[2]),
                _ => {}
            }
        }
        at += b.curves;
    }
    Some((pos?, rot?, fov))
}

/// `al_sm_personal_NN`: favourability LV → short movie id of a
/// personality (`tableSMPeronals`).
pub fn personal_movie(unity: &mut Unity, personality: i64, lv: i32) -> Result<Option<i32>> {
    let Some((key, id)) = unity.bundle_asset("lib/am/lis/000_00.unity3d", &format!("al_sm_personal_{personality:02}[000_00]"), 49)? else { return Ok(None) };
    let t = crate::tables::Table::parse(&unity.text_asset(&key, id)?)?;
    let col = |k: i32| t.keys.iter().position(|&c| c == k);
    let (Some(i), Some(s)) = (col(5), col(104)) else { return Ok(None) };
    Ok(t.rows.iter().find(|r| r.get(i).and_then(|v| v.parse::<i32>().ok()) == Some(lv)).and_then(|r| r.get(s)?.parse().ok()))
}

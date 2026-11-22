// SPDX-License-Identifier: MPL-2.0
//! H breath voices (`h/list/sound/voice/breath` `breath[personality]`:
//! MemoryPack `AL.H.List.Voice.BreathData` — `Datas` of `Data` objects with
//! ID, `StageEnum` (Normal, Lewdness, Weakness), `GroupEnum`, the voice
//! bundle and asset, `Override` and `FaceParams`).
//!
//! SATD-amanatsu-port: `HVoiceCtrl` chooses groups through per-state voice
//! updaters that are not reproduced; the H scene uses LoopWait for Idle,
//! `Loop{gauge third}` with `_S` (weak loop), `_F` (strong loop) or `_C`
//! (a finish available), and the climax groups for the finishes. Short
//! breaths, speech voices (`voice[NN]`), face parameters, lip sync and SE
//! lists are not played.
use crate::unity::Unity;
use anyhow::{ensure, Context, Result};
use crysirror_unity::memorypack::Reader;

pub const BREATH: &str = "lib/h/list/sound/voice/breath/000_01.unity3d";

/// `BreathData.Data.GroupEnum`.
pub mod group {
    pub const LOOP_WAIT: i32 = 0;
    /// `Loop{n}_S` = 1 + 3n, `_F` = 2 + 3n, `_C` = 3 + 3n.
    pub fn looped(level: i32, speed: usize) -> i32 {
        1 + 3 * level.clamp(0, 3) + speed.min(2) as i32
    }
    pub const CLIMAX_F: i32 = 13;
    pub const CLIMAX_IN: i32 = 15;
    pub const CLIMAX_OUT: i32 = 17;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Breath {
    pub stage: i32,
    pub group: i32,
    pub bundle: String,
    pub asset: String,
}

/// `FaceParam`: 3 floats, 4 ints, a float, 2 bools, 4 ints and three
/// `Range`s (two `Vector3` each), unmanaged after its member count.
const FACE_PARAM_BYTES: usize = 12 + 16 + 4 + 2 + 16 + 3 * 24;

pub fn parse(b: &[u8]) -> Result<Vec<Breath>> {
    let mut r = Reader::new(b);
    r.object_header()?.context("null BreathData")?;
    let n = r.i32()?;
    let mut out = Vec::new();
    for _ in 0..n.max(0) {
        let members = r.object_header()?.context("null breath")?;
        ensure!(members == 7, "breath data members {members}");
        let _id = r.i32()?;
        let stage = r.i32()?;
        let bundle = r.string()?.unwrap_or_default();
        let group = r.i32()?;
        let asset = r.string()?.unwrap_or_default();
        r.take(1)?;
        let faces = r.i32()?;
        for _ in 0..faces.max(0) {
            ensure!(r.object_header()? == Some(17), "face param members");
            r.take(FACE_PARAM_BYTES)?;
        }
        out.push(Breath { stage, group, bundle, asset });
    }
    Ok(out)
}

/// `breath[personality]`.
pub fn load(unity: &mut Unity, personality: i64) -> Result<Vec<Breath>> {
    let (key, id) = unity.bundle_asset(BREATH, &format!("breath[{personality:03}]"), 49)?.context("breath list")?;
    parse(&unity.text_asset(&key, id)?)
}

/// A random breath of a group (the Normal stage first).
pub fn pick<'a>(list: &'a [Breath], group: i32, stage: i32, random: &mut dyn FnMut(usize) -> usize) -> Option<&'a Breath> {
    let mut c: Vec<&Breath> = list.iter().filter(|b| b.group == group && b.stage == stage).collect();
    if c.is_empty() {
        c = list.iter().filter(|b| b.group == group).collect();
    }
    (!c.is_empty()).then(|| c[random(c.len())])
}

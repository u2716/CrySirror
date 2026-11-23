// SPDX-License-Identifier: MPL-2.0
//! The face (`ILLGAMES.Unity.FaceBlendShape` on the head part): the
//! eyebrow, eyes and mouth controllers (`FBSBase`), each with target
//! meshes and a pattern table of (close, open) blend shape indices. A
//! pattern at an openness weights its close shape by `1 − open` and its
//! open shape by `open` (`FBSBase.ChangePtn` / `SetOpenMax`).
//!
//! Blinking (`BlinkCtrl`) is timed by the session.
//!
//! Eye look (`EyeLookCalc` → `CmpEyeLookMaterial`) moves the iris
//! texture of `cf_O_hitomi_L`/`_R` toward a target.
//!
//! SATD-amanatsu-port: highlights, lip sync and pattern blend transitions
//! are not applied; the eyes/mouth openness is the `OpenMax` value.
use crate::unity::Unity;
use anyhow::Result;
use crysirror_unity::{managed::Value, scene6::Scene6};
use std::collections::BTreeMap;

/// One `FBSTarget`: the shape names of its mesh and the patterns.
#[derive(Clone, Debug, Default)]
struct Target {
    shapes: Vec<String>,
    patterns: Vec<(usize, usize)>,
}

#[derive(Clone, Debug, Default)]
pub struct Face {
    /// Eyebrow, eyes, mouth.
    ctrls: [Vec<Target>; 3],
}

/// `Manager.Game.Expression` (what the face shows).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Expression {
    pub eyebrow: usize,
    pub eyes: usize,
    pub mouth: usize,
    pub eyebrow_open: f32,
    pub eyes_open: f32,
    pub mouth_open: f32,
    /// `HumanFace.ChangeHohoAkaRate`: the flush (`_HohoakaIntensity`).
    pub hohoaka: f32,
    /// `Status.tearsLv` (0 none, 1–3 `TEARS` small to large).
    pub tears: u8,
}

/// `HumanFace.UpdateVisible`: the tear meshes, one shown per level 1–3.
pub const TEARS: [&str; 3] = ["cf_O_namida_S", "cf_O_namida_M", "cf_O_namida_L"];

/// The tear meshes hidden at a level.
pub fn hidden_tears(level: u8) -> impl Iterator<Item = &'static str> {
    TEARS.iter().enumerate().filter(move |(i, _)| *i + 1 != level as usize).map(|(_, n)| *n)
}

impl Default for Expression {
    fn default() -> Self {
        Self { eyebrow: 0, eyes: 0, mouth: 0, eyebrow_open: 1., eyes_open: 1., mouth_open: 0., hohoaka: 0., tears: 0 }
    }
}

impl Face {
    /// The `FaceBlendShape` of an assembled character (`file`: the scene's
    /// primary file).
    pub fn from_scene(unity: &mut Unity, scene: &Scene6, file: &str) -> Result<Option<Self>> {
        let Some((node, comp)) = scene.nodes.iter().enumerate().find_map(|(i, n)| n.components.iter().find(|c| c.script == "ILLGAMES.Unity.FaceBlendShape").map(|c| (i, c.clone()))) else { return Ok(None) };
        let origin = scene.nodes[node].origin;
        let nf = scene.file_of(node, file).to_owned();
        let mut face = Face::default();
        for (slot, key) in ["EyebrowCtrl", "EyesCtrl", "MouthCtrl"].iter().enumerate() {
            let Some(ctrl) = comp.fields.fields(key) else { continue };
            for t in ctrl.array("FBSTarget").unwrap_or(&[]) {
                let Value::Struct(t) = t else { continue };
                let patterns = t.array("PtnSet").unwrap_or(&[]).iter().filter_map(|p| if let Value::Struct(p) = p { Some((p.int("Close")?.max(0) as usize, p.int("Open")?.max(0) as usize)) } else { None }).collect();
                // The target renderer (same file as the component) → its mesh.
                let Some(target) = t.pptr("ObjTarget") else { continue };
                let tnode = scene.nodes.iter().position(|n| n.origin == origin && (n.game_object == target.path_id || n.components.iter().any(|c| c.path_id == target.path_id)));
                // `SkinnedMeshRenderer` (class 137).
                let renderer = tnode.and_then(|i| scene.nodes[i].components.iter().find(|c| c.class_id == 137)).cloned();
                let Some(mesh_ptr) = renderer.as_ref().and_then(|r| r.fields.pptr("m_Mesh")) else { continue };
                let Ok((mf, mid)) = unity.target(&nf, mesh_ptr) else { continue };
                let Ok(mesh) = unity.mesh(&mf, mid) else { continue };
                face.ctrls[slot].push(Target { shapes: mesh.blend_shapes.iter().map(|b| b.name.clone()).collect(), patterns });
            }
        }
        Ok(Some(face))
    }

    /// Morph weights (by shape name, 0..1) showing `e`; every shape of the
    /// controlled meshes is named, so a change clears the previous pattern.
    pub fn weights(&self, e: &Expression) -> BTreeMap<String, f32> {
        let mut out = BTreeMap::new();
        for (slot, targets) in self.ctrls.iter().enumerate() {
            let (ptn, open) = match slot {
                0 => (e.eyebrow, e.eyebrow_open),
                1 => (e.eyes, e.eyes_open),
                _ => (e.mouth, e.mouth_open),
            };
            for t in targets {
                for name in &t.shapes {
                    out.entry(name.clone()).or_insert(0.);
                }
                let Some(&(close, opened)) = t.patterns.get(ptn).or(t.patterns.first()) else { continue };
                let open = open.clamp(0., 1.);
                if let Some(n) = t.shapes.get(close) {
                    *out.entry(n.clone()).or_insert(0.) += 1. - open;
                }
                if let Some(n) = t.shapes.get(opened) {
                    *out.entry(n.clone()).or_insert(0.) += open;
                }
            }
        }
        for v in out.values_mut() {
            *v = v.clamp(0., 1.);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_weights() {
        let face = Face { ctrls: [Vec::new(), vec![Target { shapes: vec!["cl".into(), "op".into(), "smile".into()], patterns: vec![(0, 1), (0, 2)] }], Vec::new()] };
        let w = face.weights(&Expression { eyes: 1, eyes_open: 0.75, ..Default::default() });
        assert_eq!(w["cl"], 0.25);
        assert_eq!(w["op"], 0.);
        assert_eq!(w["smile"], 0.75);
    }
}

/// `ActionMapCameraManager.SetCommnadCamera` over the command prefab's
/// `AL.CinemachineCameraAngleData`: camera `id`'s `_s`/`_m`/`_l`
/// transforms (relative to the angle data object) interpolated by the
/// heroine's height (`CalcLower` S→M below 0.5, `CalcUpper` M→L above),
/// as (position, rotation) in the angle data's space.
/// SATD-amanatsu-port: `isFront`/`isReturn`, the reset angles, FOV limits
/// and the camera's user control are not applied.
pub fn command_camera(scene: &Scene6, id: i64, height: f32) -> Option<(glam::Vec3, glam::Quat)> {
    use crysirror_unity::world6::world_matrix;
    let node_of = |p: i64| scene.by_game_object.get(&p).copied().or_else(|| scene.nodes.iter().position(|n| n.transform == p || n.components.iter().any(|c| c.path_id == p)));
    let (root, data) = scene.nodes.iter().enumerate().find_map(|(i, n)| n.components.iter().find(|c| c.script == "AL.CinemachineCameraAngleData").map(|c| (i, c)))?;
    let table = data.fields.fields("_table")?;
    let keys = table.array("_keyData")?;
    let values = table.array("_valueData")?;
    let at = keys.iter().position(|k| matches!(k, Value::Int(v) if *v == id))?;
    let Value::PPtr(entry) = values.get(at)? else { return None };
    let entry = scene.nodes.iter().flat_map(|n| &n.components).find(|c| c.path_id == entry.path_id)?;
    let inv = world_matrix(scene, root).inverse();
    let local = |k: &str| -> Option<(glam::Vec3, glam::Quat)> {
        let n = node_of(entry.fields.pptr(k)?.path_id)?;
        let (_, r, t) = (inv * world_matrix(scene, n)).to_scale_rotation_translation();
        Some((t, r))
    };
    let (a, b, t) = if height < 0.5 { (local("_s")?, local("_m")?, height * 2.) } else { (local("_m")?, local("_l")?, (height - 0.5) * 2.) };
    let t = t.clamp(0., 1.);
    Some((a.0.lerp(b.0, t), a.1.slerp(b.1, t)))
}

/// `Manager.Game.LoadExpressionData` (`com/lis/exp` `cNN` `ExcelData`):
/// preset id → eyebrow, eyes, mouth, eyebrow open, eyes open, mouth open
/// min (the rows' columns 2–6 and 8).
pub fn expression_list(unity: &mut Unity, personality: i64) -> Result<std::collections::HashMap<i64, [Option<f32>; 8]>> {
    let mut out = std::collections::HashMap::new();
    let Some((key, id)) = unity.bundle_asset("lib/com/lis/exp/000_00.unity3d", &format!("c{personality:02}"), 114)? else { return Ok(out) };
    let f = unity.decode(&key, id)?;
    for row in f.array("list").unwrap_or(&[]) {
        let Value::Struct(r) = row else { continue };
        let cells: Vec<String> = r.array("List").unwrap_or(&[]).iter().map(|v| if let Value::String(s) = v { s.clone() } else { String::new() }).collect();
        let Some(pid) = cells.first().and_then(|c| c.trim().parse::<i64>().ok()) else { continue };
        let v = |i: usize| cells.get(i).and_then(|c| c.trim().parse::<f32>().ok());
        // Columns from 2: eyebrow, eyes, mouth, eyebrow open, eyes open,
        // mouth open max/min, eye look, flush rate (`HohoAkaRate`),
        // highlight, tears level, blink.
        out.insert(pid, [v(2), v(3), v(4), v(5), v(6), v(8), v(10), v(12)]);
    }
    Ok(out)
}

/// `EyeLookCalc.EyeTypeState`: the bending limits of one look pattern.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EyeTypeState {
    pub up: f32,
    pub down: f32,
    pub min: f32,
    pub max: f32,
    pub h_limit: f32,
    pub v_limit: f32,
}

/// `EyeLookCalc.EyeTypeStates` of the head bone prefab
/// (`chara/head/bone/000` `p_cf_al_head_bone_00`).
pub fn eye_type_states(unity: &mut Unity) -> Result<Vec<EyeTypeState>> {
    use anyhow::Context;
    let (key, go) = unity.bundle_asset("lib/chara/head/bone/000.unity3d", "p_cf_al_head_bone_00", crysirror_unity::class_id::GAME_OBJECT)?.context("p_cf_al_head_bone_00")?;
    let all = unity.scene(&key)?;
    let root = *all.by_game_object.get(&go).context("head bone root")?;
    let scene = all.subtree(root);
    let states = scene.nodes.iter().flat_map(|n| &n.components).find_map(|c| c.fields.array("EyeTypeStates")).context("EyeLookCalc")?;
    Ok(states
        .iter()
        .filter_map(|v| match v {
            Value::Struct(f) => {
                let g = |k: &str| f.float(k).unwrap_or(0.) as f32;
                Some(EyeTypeState { up: g("UpBendingAngle"), down: g("DownBendingAngle"), min: g("MinBendingAngle"), max: g("MaxBendingAngle"), h_limit: g("HAngleLimit"), v_limit: g("VAngleLimit") })
            }
            _ => None,
        })
        .collect())
}

/// The rest eye positions and forwards (`EyeTargetL`/`EyeTargetR`, the
/// `EyeLookCalc.EyeObjs`) in the character's space.
pub fn eye_rest(scene: &Scene6) -> Option<[([f32; 3], [f32; 3]); 2]> {
    let eye = |name: &str| {
        let n = scene.nodes.iter().position(|n| n.name == name)?;
        let m = crysirror_unity::world6::world_matrix(scene, n);
        Some((m.w_axis.truncate().to_array(), m.transform_vector3(glam::Vec3::Z).normalize_or_zero().to_array()))
    };
    Some([eye("EyeTargetL")?, eye("EyeTargetR")?])
}

/// `CmpEyeLookMaterial._eyeLookRange` of `p_cf_al_head_00` (x for the
/// left/right eye, y shared).
const LOOK_RANGE_X: [[f32; 2]; 2] = [[-0.2, 0.11], [-0.11, 0.2]];
const LOOK_RANGE_Y: [f32; 2] = [-0.07, 0.15];

/// `EyeLookCalc.AngleHRateCalc`/`AngleVRateCalc` of an eye at `eye`
/// looking along `forward` (Unity space, head up +Y) toward `target`, then
/// `CmpEyeLookMaterial.Update`'s position relative to the straight look,
/// as the iris offset in Unity UV (v up).
pub fn eye_offset(state: &EyeTypeState, eye_index: usize, eye: glam::Vec3, forward: glam::Vec3, target: glam::Vec3) -> [f32; 2] {
    let f = forward.normalize_or_zero();
    let r = glam::Vec3::Y.cross(f).normalize_or_zero();
    let u = f.cross(r);
    let d = target - eye;
    let (x, y, z) = (d.dot(r), d.dot(u), d.dot(f));
    let h = x.atan2(z).to_degrees();
    let v = -y.atan2((x * x + z * z).sqrt()).to_degrees();
    if f == glam::Vec3::ZERO || h.abs() > state.h_limit || v.abs() > state.v_limit {
        return [0.; 2];
    }
    let inv = |limit: f32, a: f32| if limit == 0. { 0. } else { (a / limit).clamp(0., 1.) };
    let hr = if h < 0. { -inv(state.min, h) } else { inv(state.max, h) };
    let vr = if v < 0. { -inv(state.up, v) } else { inv(state.down, v) };
    let lerp = |r: [f32; 2], t: f32| r[0] + (r[1] - r[0]) * t;
    let rx = LOOK_RANGE_X[eye_index.min(1)];
    [lerp(rx, (hr + 1.) * 0.5) - lerp(rx, 0.5), lerp(LOOK_RANGE_Y, (-vr + 1.) * 0.5) - lerp(LOOK_RANGE_Y, 0.5)]
}

/// The iris offsets as engine UV0 offsets of the eye meshes. The lookup
/// moves against the content in Unity UV; the exported UVs run top-down
/// (v) and the X-mirrored export turns the eye texture's u (both checked
/// against targets beside and above the head).
pub fn eye_uv_offsets(offsets: &[[f32; 2]; 2]) -> BTreeMap<String, [f32; 2]> {
    ["cf_O_hitomi_L", "cf_O_hitomi_R"].iter().zip(offsets).map(|(n, o)| ((*n).to_owned(), [o[0], o[1]])).collect()
}

#[cfg(test)]
mod eye_tests {
    use super::*;

    const STATE: EyeTypeState = EyeTypeState { up: -66., down: 25., min: -21.5, max: 25.5, h_limit: 110., v_limit: 80. };

    #[test]
    fn straight_look_is_neutral_and_sides_saturate() {
        let o = eye_offset(&STATE, 0, glam::Vec3::ZERO, glam::Vec3::Z, glam::Vec3::new(0., 0., 10.));
        assert!(o[0].abs() < 1e-6 && o[1].abs() < 1e-6);
        // 45° to the right: past `MaxBendingAngle`, the full range.
        let o = eye_offset(&STATE, 0, glam::Vec3::ZERO, glam::Vec3::Z, glam::Vec3::new(10., 0., 10.));
        assert!((o[0] - 0.155).abs() < 1e-4, "{o:?}");
        // Behind the head (> `HAngleLimit`): straight again.
        assert_eq!(eye_offset(&STATE, 0, glam::Vec3::ZERO, glam::Vec3::Z, glam::Vec3::new(0., 0., -10.)), [0.; 2]);
        // Looking up moves the iris up.
        assert!(eye_offset(&STATE, 1, glam::Vec3::ZERO, glam::Vec3::Z, glam::Vec3::new(0., 5., 10.))[1] > 0.);
    }
}

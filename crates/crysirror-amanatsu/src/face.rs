// SPDX-License-Identifier: MPL-2.0
//! The face (`ILLGAMES.Unity.FaceBlendShape` on the head part): the
//! eyebrow, eyes and mouth controllers (`FBSBase`), each with target
//! meshes and a pattern table of (close, open) blend shape indices. A
//! pattern at an openness weights its close shape by `1 − open` and its
//! open shape by `open` (`FBSBase.ChangePtn` / `SetOpenMax`).
//!
//! SATD-amanatsu-port: blinking (`BlinkCtrl`), eye look, cheeks
//! (`HohoAka`), tears, highlights, lip sync and pattern blend transitions
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
}

impl Default for Expression {
    fn default() -> Self {
        Self { eyebrow: 0, eyes: 0, mouth: 0, eyebrow_open: 1., eyes_open: 1., mouth_open: 0. }
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
pub fn expression_list(unity: &mut Unity, personality: i64) -> Result<std::collections::HashMap<i64, [Option<f32>; 6]>> {
    let mut out = std::collections::HashMap::new();
    let Some((key, id)) = unity.bundle_asset("lib/com/lis/exp/000_00.unity3d", &format!("c{personality:02}"), 114)? else { return Ok(out) };
    let f = unity.decode(&key, id)?;
    for row in f.array("list").unwrap_or(&[]) {
        let Value::Struct(r) = row else { continue };
        let cells: Vec<String> = r.array("List").unwrap_or(&[]).iter().map(|v| if let Value::String(s) = v { s.clone() } else { String::new() }).collect();
        let Some(pid) = cells.first().and_then(|c| c.trim().parse::<i64>().ok()) else { continue };
        let v = |i: usize| cells.get(i).and_then(|c| c.trim().parse::<f32>().ok());
        out.insert(pid, [v(2), v(3), v(4), v(5), v(6), v(8)]);
    }
    Ok(out)
}

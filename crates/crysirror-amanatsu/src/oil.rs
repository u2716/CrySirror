// SPDX-License-Identifier: MPL-2.0
//! `AL.Oil` massage: the `oil/list` tables (MemoryPack), the body hit
//! objects the touch raycast tests (`p_cf_al_body_00_hit` `o_hit_<tag>`
//! skinned colliders) and `MouseTouch`'s satisfaction gauge.
//!
//! The satisfaction rule (`UpdateSatisfactionAndExcitemen` →
//! `UpdateSatisfaction`): a touched part (`SatisfactionData` row by tag)
//! gains `MaxValue / (move ≤ Threshold ? Slow : Fast) × move`, clamped to
//! `MaxValue`; rows of type 0 belong to the front group, 1 to the back
//! group and 2 to both (each group keeps its own value), a touch updates
//! the group of the pose's side, and the gauge is the front plus the back
//! sum (`SatisfactionGauge.Value`); the evaluation stage is 0 below 25, 2/3/4 from 25/50/75
//! and 5 at 100 (`Evaluation` scenarios by stage).
//!
//! SATD-amanatsu-port: hit objects are approximated by a box per hit mesh
//! in its dominant bone's bind space; excitement (three body gauges,
//! the HScene flag), the oil gauge, pose reactions/transitions, IK, Obi effects,
//! clothes/light/colour UI and the camera's orbit limits are not ported.
use crate::{level::Level, unity::Unity};
use anyhow::{Context, Result};
use crysirror_unity::memorypack::Reader;
use glam::{Mat4, Vec3};

pub const LIST_SATISFACTION: &str = "lib/oil/list/gauge/satisfaction/000_01.unity3d";
pub const LIST_POSE: &str = "lib/oil/list/animationinfo/info/000_01.unity3d";
pub const SCENE_BUNDLE: &str = "lib/oil/scene/oilscene.unity3d";
pub const SCENE_PREFAB: &str = "OilMassageScene";
const HIT_BUNDLE: &str = "lib/chara/body/hit/000.unity3d";
const HIT_PREFAB: &str = "p_cf_al_body_00_hit";

/// `SatisfactionData.Data`.
#[derive(Clone, Debug, PartialEq)]
pub struct Satisfaction {
    pub ty: i32,
    pub tag: String,
    pub max: f32,
    pub threshold: f32,
    pub slow: f32,
    pub fast: f32,
}

/// `AnimationInfo.Data` (oil poses).
#[derive(Clone, Debug, PartialEq)]
pub struct Pose {
    pub name: String,
    pub front: bool,
    /// `Anime` (animator bundle, controller).
    pub anime: (String, String),
    pub camera: String,
}

/// A hit object as a box in its dominant bone's space (Unity axes).
#[derive(Clone, Debug, PartialEq)]
pub struct HitVolume {
    pub tag: String,
    pub bone: String,
    pub min: Vec3,
    pub max: Vec3,
}

fn file_info(r: &mut Reader) -> Result<Option<(String, String)>> {
    let Some(_) = r.object_header()? else { return Ok(None) };
    Ok(Some((r.string()?.unwrap_or_default(), r.string()?.unwrap_or_default())))
}

fn limit(r: &mut Reader) -> Result<()> {
    if r.object_header()?.is_some() {
        r.f32()?;
        r.f32()?;
    }
    Ok(())
}

fn table_bytes(unity: &mut Unity, bundle: &str) -> Result<Vec<u8>> {
    let key = unity.bundle_files().into_iter().find(|k| k.starts_with(&format!("bundle:{bundle}#"))).with_context(|| format!("bundle {bundle}"))?;
    let id = unity.file(&key)?.objects.iter().find(|o| o.class_id == 49).map(|o| o.path_id).context("oil list TextAsset")?;
    unity.text_asset(&key, id)
}

/// `SatisfactionData` (an `IntKeyDictionary` of rows).
pub fn parse_satisfaction(b: &[u8]) -> Result<Vec<Satisfaction>> {
    let mut r = Reader::new(b);
    r.object_header()?.context("null SatisfactionData")?;
    let n = r.i32()?;
    let mut out = Vec::new();
    for _ in 0..n.max(0) {
        let _key = r.i32()?;
        r.object_header()?.context("null satisfaction row")?;
        let ty = r.i32()?;
        let tag = r.string()?.unwrap_or_default();
        r.take(8)?; // `Overwrite` (Nullable<int>)
        out.push(Satisfaction { ty, tag, max: r.f32()?, threshold: r.f32()?, slow: r.f32()?, fast: r.f32()? });
    }
    Ok(out)
}

/// `AnimationInfo` (an `IntKeyDictionary` of poses).
pub fn parse_poses(b: &[u8]) -> Result<Vec<Pose>> {
    let mut r = Reader::new(b);
    r.object_header()?.context("null AnimationInfo")?;
    let n = r.i32()?;
    let mut out = Vec::new();
    for _ in 0..n.max(0) {
        let _key = r.i32()?;
        r.object_header()?.context("null pose")?;
        let name = r.string()?.unwrap_or_default();
        let front = r.bool()?;
        r.take(8)?; // `Conditional`
        let anime = file_info(&mut r)?.unwrap_or_default();
        let _ik = file_info(&mut r)?;
        let _state = r.string()?;
        let _collision = r.string()?;
        let camera = r.string()?.unwrap_or_default();
        for _ in 0..3 {
            limit(&mut r)?;
        }
        out.push(Pose { name, front, anime, camera });
    }
    Ok(out)
}

/// `oil/camera` data (`cam_b_00`, …): the orbit target, rotation (Unity
/// euler degrees), the eye offset from the target and the field of view,
/// relative to the character root.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraData {
    pub target: Vec3,
    pub rotation: Vec3,
    pub offset: Vec3,
    pub fov: f32,
}

impl CameraData {
    pub fn parse(b: &[u8]) -> Result<Self> {
        let mut r = Reader::new(b);
        let mut v = || -> Result<Vec3> { Ok(Vec3::new(r.f32()?, r.f32()?, r.f32()?)) };
        let (target, rotation, offset) = (v()?, v()?, v()?);
        Ok(Self { target, rotation, offset, fov: r.f32()? })
    }

    pub fn load(unity: &mut Unity, name: &str) -> Result<Self> {
        Self::load_from(unity, "lib/oil/camera/000_01.unity3d", name)
    }

    /// From a camera list bundle (`lib/h/list/camera/000_01.unity3d`, …).
    pub fn load_from(unity: &mut Unity, bundle: &str, name: &str) -> Result<Self> {
        let (k, id) = unity.bundle_asset(bundle, name, 49)?.with_context(|| format!("camera {name}"))?;
        Self::parse(&unity.text_asset(&k, id)?)
    }

    /// (eye, target, up) in Unity space under `root` (the character root).
    pub fn eye(&self, root: Mat4) -> (Vec3, Vec3, Vec3) {
        let r = self.rotation * std::f32::consts::PI / 180.;
        // `Quaternion.Euler`: Z, then X, then Y.
        let q = glam::Quat::from_euler(glam::EulerRot::YXZ, r.y, r.x, r.z);
        let target = root.transform_point3(self.target);
        let eye = root.transform_point3(self.target + q * self.offset);
        let up = root.transform_vector3(q * Vec3::Y).normalize_or_zero();
        (eye, target, up)
    }
}

/// The tables and hit objects of the massage.
pub struct OilData {
    pub satisfaction: Vec<Satisfaction>,
    pub poses: Vec<Pose>,
    pub volumes: Vec<HitVolume>,
}

impl OilData {
    pub fn load(unity: &mut Unity) -> Result<Self> {
        let satisfaction = parse_satisfaction(&table_bytes(unity, LIST_SATISFACTION)?)?;
        let poses = parse_poses(&table_bytes(unity, LIST_POSE)?)?;
        let volumes = hit_volumes(unity)?;
        Ok(Self { satisfaction, poses, volumes })
    }
}

/// The `o_hit_<tag>` skinned meshes of the female hit object as boxes in
/// their most-weighted bone's bind space.
pub fn hit_volumes(unity: &mut Unity) -> Result<Vec<HitVolume>> {
    let level = Level::prefab(unity, HIT_BUNDLE, HIT_PREFAB)?;
    let mut out = Vec::new();
    for n in &level.scene.nodes {
        let Some(tag) = n.name.strip_prefix("o_hit_") else { continue };
        let Some(smr) = n.components.iter().find(|c| c.class_id == 137) else { continue };
        let Some(mp) = smr.fields.pptr("m_Mesh").filter(|p| !p.is_null()) else { continue };
        let bones: Vec<String> = smr
            .fields
            .array("m_Bones")
            .unwrap_or(&[])
            .iter()
            .map(|v| match v {
                crysirror_unity::managed::Value::PPtr(p) => level.scene.nodes.iter().find(|b| b.transform == p.path_id).map(|b| b.name.clone()).unwrap_or_default(),
                _ => String::new(),
            })
            .collect();
        let (file, id) = match unity.target(&level.key, mp) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let mesh = crysirror_unity::world6::SceneAssets::mesh(unity, &file, id)?;
        // The bone holding the most vertex weight.
        let mut weight = vec![0f32; mesh.bind_poses.len()];
        for (j, w) in mesh.joints.iter().zip(&mesh.weights) {
            for k in 0..4 {
                if let Some(s) = weight.get_mut(j[k] as usize) {
                    *s += w[k];
                }
            }
        }
        let Some((bi, _)) = weight.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)) else { continue };
        let Some(bone) = bones.get(bi).filter(|b| !b.is_empty()) else { continue };
        let bind = Mat4::from_cols_array(&mesh.bind_poses[bi]);
        let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for p in &mesh.positions {
            let q = bind.transform_point3(Vec3::from_array(*p));
            min = min.min(q);
            max = max.max(q);
        }
        if min.x <= max.x {
            out.push(HitVolume { tag: tag.to_owned(), bone: bone.clone(), min, max });
        }
    }
    Ok(out)
}

/// Ray (engine space) against the volumes posed by `bone_world` (engine
/// world matrix of a bone by name): the nearest hit's tag.
pub fn raycast<'a>(volumes: &'a [HitVolume], origin: Vec3, dir: Vec3, bone_world: &dyn Fn(&str) -> Option<Mat4>) -> Option<&'a HitVolume> {
    // Unity bone space → engine bone space mirrors X.
    let mirror = Mat4::from_scale(Vec3::new(-1., 1., 1.));
    let mut best: Option<(f32, &HitVolume)> = None;
    for v in volumes {
        let Some(m) = bone_world(&v.bone) else { continue };
        let inv = (m * mirror).inverse();
        let o = inv.transform_point3(origin);
        let d = inv.transform_vector3(dir);
        let (mut t0, mut t1) = (0f32, f32::MAX);
        let mut hit = true;
        for a in 0..3 {
            if d[a].abs() < 1e-9 {
                if o[a] < v.min[a] || o[a] > v.max[a] {
                    hit = false;
                    break;
                }
                continue;
            }
            let (mut ta, mut tb) = ((v.min[a] - o[a]) / d[a], (v.max[a] - o[a]) / d[a]);
            if ta > tb {
                std::mem::swap(&mut ta, &mut tb);
            }
            t0 = t0.max(ta);
            t1 = t1.min(tb);
            if t0 > t1 {
                hit = false;
                break;
            }
        }
        if hit && best.is_none_or(|b| t0 < b.0) {
            best = Some((t0, v));
        }
    }
    best.map(|b| b.1)
}

/// `MouseTouch` satisfaction state of one massage.
#[derive(Clone, Debug, Default)]
pub struct Massage {
    /// Front and back group values per `SatisfactionData` row.
    pub front: Vec<f32>,
    pub back: Vec<f32>,
    pub pose: usize,
}

fn in_group(ty: i32, front: bool) -> bool {
    match ty {
        0 => front,
        1 => !front,
        _ => true,
    }
}

impl Massage {
    pub fn new(data: &OilData) -> Self {
        Self { front: vec![0.; data.satisfaction.len()], back: vec![0.; data.satisfaction.len()], pose: 0 }
    }

    /// Touching the part `tag` with a mouse movement of `moved`.
    pub fn touch(&mut self, data: &OilData, tag: &str, moved: f32) -> bool {
        let front = data.poses.get(self.pose).is_none_or(|p| p.front);
        let Some(i) = data.satisfaction.iter().position(|s| s.tag == tag && in_group(s.ty, front)) else { return false };
        let s = &data.satisfaction[i];
        let values = if front { &mut self.front } else { &mut self.back };
        if values.len() != data.satisfaction.len() {
            values.resize(data.satisfaction.len(), 0.);
        }
        if values[i] >= s.max {
            return false;
        }
        let time = if moved <= s.threshold { s.slow } else { s.fast };
        let rate = if time > 0. { s.max / time } else { 0. };
        values[i] = (values[i] + rate * moved).clamp(0., s.max);
        true
    }

    /// The satisfaction gauge: the front plus the back group sum.
    pub fn satisfaction(&self, data: &OilData) -> f32 {
        let sum = |values: &[f32], front: bool| data.satisfaction.iter().zip(values).filter(|(s, _)| in_group(s.ty, front)).map(|(_, v)| *v).sum::<f32>();
        sum(&self.front, true) + sum(&self.back, false)
    }

    /// The evaluation stage of the gauge.
    pub fn stage(&self, data: &OilData) -> usize {
        stage(self.satisfaction(data) as i32)
    }
}

pub fn stage(v: i32) -> usize {
    match v {
        i32::MIN..=24 => 0,
        25..=49 => 2,
        50..=74 => 3,
        75..=99 => 4,
        _ => 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn satisfaction_rows_and_gauge() {
        let mut b = vec![1u8];
        b.extend(1i32.to_le_bytes());
        b.extend(0i32.to_le_bytes());
        b.push(7);
        b.extend(0i32.to_le_bytes());
        b.extend((!4i32).to_le_bytes());
        b.extend(4i32.to_le_bytes());
        b.extend(b"head");
        b.extend([0u8; 8]);
        for f in [15f32, 5., 5., 1.5] {
            b.extend(f.to_le_bytes());
        }
        let rows = parse_satisfaction(&b).unwrap();
        assert_eq!(rows, vec![Satisfaction { ty: 0, tag: "head".into(), max: 15., threshold: 5., slow: 5., fast: 1.5 }]);
        let data = OilData { satisfaction: rows, poses: Vec::new(), volumes: Vec::new() };
        let mut m = Massage::new(&data);
        assert!(m.touch(&data, "head", 2.));
        assert!((m.front[0] - 6.).abs() < 1e-4);
        m.touch(&data, "head", 10.);
        assert_eq!(m.front[0], 15.);
        assert_eq!(m.satisfaction(&data), 15.);
        assert!(!m.touch(&data, "head", 1.));
        assert_eq!([stage(10), stage(30), stage(60), stage(80), stage(100)], [0, 2, 3, 4, 5]);
    }

    #[test]
    fn ray_hits_a_posed_box() {
        let v = HitVolume { tag: "head".into(), bone: "b".into(), min: Vec3::splat(-1.), max: Vec3::splat(1.) };
        let vols = vec![v];
        let at = Mat4::from_translation(Vec3::new(0., 0., 10.));
        let hit = raycast(&vols, Vec3::ZERO, Vec3::Z, &|_| Some(at));
        assert_eq!(hit.map(|h| h.tag.as_str()), Some("head"));
        assert!(raycast(&vols, Vec3::ZERO, Vec3::X, &|_| Some(at)).is_none());
    }
}

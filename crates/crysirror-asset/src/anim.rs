// SPDX-License-Identifier: MPL-2.0
//! Skeletal animation: keyframe sampling, multi-clip blending, hierarchy
//! evaluation and joint palettes (glTF 2.0 semantics). Device-free.
use crate::pbr::{ChannelPath, Clip, Interpolation, PbrModel, Trs};
use std::sync::Arc;
use glam::{Mat4, Quat, Vec3, Vec4};

pub const MAX_JOINTS: usize = 256;

fn sample(times: &[f32], t: f32) -> (usize, usize, f32) {
    if t <= times[0] {
        return (0, 0, 0.);
    }
    let last = times.len() - 1;
    if t >= times[last] {
        return (last, last, 0.);
    }
    let i = times.partition_point(|x| *x <= t) - 1;
    let span = (times[i + 1] - times[i]).max(1e-9);
    (i, i + 1, (t - times[i]) / span)
}
fn cubic(v0: Vec4, b0: Vec4, a1: Vec4, v1: Vec4, s: f32, dt: f32) -> Vec4 {
    let (s2, s3) = (s * s, s * s * s);
    v0 * (2. * s3 - 3. * s2 + 1.)
        + b0 * dt * (s3 - 2. * s2 + s)
        + v1 * (-2. * s3 + 3. * s2)
        + a1 * dt * (s3 - s2)
}

/// Local node transforms for one clip at time `t` (seconds, already wrapped).
pub fn sample_clip(model: &PbrModel, clip: &Clip, t: f32, out: &mut [Trs]) {
    for c in &clip.channels {
        let Some(node) = out.get_mut(c.node) else {
            continue;
        };
        let (i, j, f) = sample(&c.times, t);
        let v = match c.interpolation {
            Interpolation::Step => Vec4::from_array(c.values[i]),
            Interpolation::Linear => {
                let (a, b) = (Vec4::from_array(c.values[i]), Vec4::from_array(c.values[j]));
                if c.path == ChannelPath::Rotation {
                    let q = Quat::from_vec4(a).slerp(Quat::from_vec4(b), f);
                    Vec4::from(q)
                } else {
                    a.lerp(b, f)
                }
            }
            Interpolation::CubicSpline => {
                let k = |n: usize, m: usize| Vec4::from_array(c.values[n * 3 + m]);
                let dt = c.times[j] - c.times[i];
                let r = cubic(k(i, 1), k(i, 2), k(j, 0), k(j, 1), f, dt);
                if c.path == ChannelPath::Rotation {
                    r.normalize()
                } else {
                    r
                }
            }
        };
        match c.path {
            ChannelPath::Translation => node.translation = v.truncate(),
            ChannelPath::Scale => node.scale = v.truncate(),
            ChannelPath::Rotation => node.rotation = Quat::from_vec4(v).normalize(),
        }
    }
    let _ = model;
}

/// One weighted clip contribution.
#[derive(Clone, Copy, Debug)]
pub struct Layer {
    pub clip: usize,
    pub time: f32,
    pub weight: f32,
}

/// Blend clips into local transforms (weights normalized; rest pose when
/// no layer is given) and evaluate global node matrices.
pub fn pose(model: &PbrModel, layers: &[Layer]) -> Vec<Mat4> {
    let bound: Vec<(&Clip, f32, f32)> = layers
        .iter()
        .filter_map(|l| model.animations.get(l.clip).map(|c| (c, l.time, l.weight)))
        .collect();
    global_pose(model, &local_pose(model, &bound))
}

/// One clip contribution to a local pose.
#[derive(Clone, Debug)]
pub struct PoseLayer<C> {
    pub clip: C,
    /// Seconds; wrapped when `looping`, else clamped to the clip.
    pub time: f32,
    pub weight: f32,
    pub looping: bool,
    /// Priority layer (Unity legacy `AnimationState.layer`): per node, higher
    /// layers take their (normalized, at most 1) weight first and lower
    /// layers share the remainder.
    pub layer: u32,
}

/// Blend looping layer-0 clips (see [`local_pose_layered`]).
pub fn local_pose(model: &PbrModel, layers: &[(&Clip, f32, f32)]) -> Vec<Trs> {
    let layers: Vec<PoseLayer<&Clip>> =
        layers.iter().map(|&(clip, time, weight)| PoseLayer { clip, time, weight, looping: true, layer: 0 }).collect();
    local_pose_layered(model, &layers)
}

/// Blend clips whose channels already address `model`'s nodes (its own, or
/// [`retarget`]ed) into local transforms. A clip contributes only to the
/// nodes it has channels for; per node, layers are taken from the highest
/// priority down, each normalized when its weights exceed 1, until the
/// weight is used up; the result is normalized. Nodes no clip animates keep
/// their rest transform.
pub fn local_pose_layered(model: &PbrModel, layers: &[PoseLayer<&Clip>]) -> Vec<Trs> {
    let rest: Vec<Trs> = model.nodes.iter().map(|n| n.rest).collect();
    if layers.iter().all(|l| l.weight <= 1e-6) {
        return rest;
    }
    let n = rest.len();
    let mut samples = Vec::with_capacity(layers.len());
    for l in layers {
        let mut p = rest.clone();
        let mut mask = vec![false; n];
        let d = l.clip.duration;
        let t = if d > 0. {
            if l.looping { l.time.rem_euclid(d) } else { l.time.clamp(0., d) }
        } else {
            0.
        };
        sample_clip(model, l.clip, t, &mut p);
        for c in &l.clip.channels {
            if let Some(m) = mask.get_mut(c.node) {
                *m = true;
            }
        }
        samples.push((p, mask));
    }
    let mut order: Vec<usize> = (0..layers.len()).collect();
    order.sort_by_key(|i| std::cmp::Reverse(layers[*i].layer));
    let mut weights = vec![0f32; layers.len()];
    (0..n)
        .map(|node| {
            weights.iter_mut().for_each(|w| *w = 0.);
            let mut remaining = 1f32;
            let mut k = 0;
            while k < order.len() && remaining > 1e-6 {
                let layer = layers[order[k]].layer;
                let group: Vec<usize> = order[k..].iter().copied().take_while(|i| layers[*i].layer == layer).collect();
                k += group.len();
                let sum: f32 = group.iter().filter(|i| samples[**i].1[node]).map(|i| layers[*i].weight.max(0.)).sum();
                if sum <= 1e-6 {
                    continue;
                }
                let scale = if sum > 1. { 1. / sum } else { 1. };
                for i in group.iter().filter(|i| samples[**i].1[node]) {
                    weights[*i] = layers[*i].weight.max(0.) * scale * remaining;
                }
                remaining -= sum.min(1.) * remaining;
            }
            let total: f32 = weights.iter().sum();
            if total <= 1e-6 {
                return rest[node];
            }
            let (mut t, mut sc, mut r) = (Vec3::ZERO, Vec3::ZERO, Vec4::ZERO);
            let mut first: Option<Quat> = None;
            for (i, w) in weights.iter().enumerate().filter(|(_, w)| **w > 0.) {
                let x = samples[i].0[node];
                let w = w / total;
                t += x.translation * w;
                sc += x.scale * w;
                // Keep quaternions in one hemisphere before averaging.
                let q = match first {
                    Some(f) if f.dot(x.rotation) < 0. => -x.rotation,
                    Some(_) => x.rotation,
                    None => {
                        first = Some(x.rotation);
                        x.rotation
                    }
                };
                r += Vec4::from(q) * w;
            }
            Trs { translation: t, rotation: Quat::from_vec4(r).normalize(), scale: sc }
        })
        .collect()
}

/// Global node matrices of the default scene for the given local transforms.
pub fn global_pose(model: &PbrModel, local: &[Trs]) -> Vec<Mat4> {
    let mut global = vec![Mat4::IDENTITY; local.len()];
    let mut stack: Vec<(usize, Mat4)> = model.roots.iter().map(|r| (*r, Mat4::IDENTITY)).collect();
    let mut guard = 0;
    while let Some((n, parent)) = stack.pop() {
        guard += 1;
        if guard > 100_000 || n >= local.len() {
            break;
        }
        let g = parent * local[n].matrix();
        global[n] = g;
        for c in &model.nodes[n].children {
            stack.push((*c, g));
        }
    }
    global
}

/// `clip` of `source` with its channels bound to the same-named nodes of
/// `target`. Channels of nodes `target` lacks are omitted.
pub fn retarget(source: &PbrModel, clip: &Clip, target: &PbrModel) -> Clip {
    let names: std::collections::HashMap<&str, usize> = target
        .nodes
        .iter()
        .enumerate()
        .rev()
        .map(|(i, n)| (n.name.as_str(), i))
        .collect();
    Clip {
        name: clip.name.clone(),
        duration: clip.duration,
        channels: clip
            .channels
            .iter()
            .filter_map(|c| {
                let name = source.nodes.get(c.node)?.name.as_str();
                let node = *names.get(name)?;
                Some(crate::pbr::Channel { node, ..c.clone() })
            })
            .collect(),
    }
}

/// Ride `model`'s root nodes on `leader`'s `node` (a part parented to a bone
/// at load time in the rest pose): roots are premultiplied by the node's
/// current global transform times the inverse of its rest global transform.
/// An unknown node leaves the pose unchanged.
pub fn attach(model: &PbrModel, local: &mut [Trs], leader: &PbrModel, leader_local: &[Trs], node: &str) {
    let Some(n) = leader.nodes.iter().position(|x| x.name == node) else { return };
    let rest: Vec<Trs> = leader.nodes.iter().map(|x| x.rest).collect();
    let (now, then) = (global_pose(leader, leader_local), global_pose(leader, &rest));
    let (Some(now), Some(then)) = (now.get(n), then.get(n)) else { return };
    let m = *now * then.inverse();
    for r in &model.roots {
        if let Some(t) = local.get_mut(*r) {
            let (scale, rotation, translation) = (m * t.matrix()).to_scale_rotation_translation();
            *t = Trs { translation, rotation: rotation.normalize(), scale };
        }
    }
}

/// Copy translation and rotation of `leader`'s same-named nodes onto
/// `local` (a character part bound to a shared skeleton). Scale is kept.
pub fn follow(model: &PbrModel, local: &mut [Trs], leader: &PbrModel, leader_local: &[Trs]) {
    let names: std::collections::HashMap<&str, usize> = leader
        .nodes
        .iter()
        .enumerate()
        .rev()
        .map(|(i, n)| (n.name.as_str(), i))
        .collect();
    for (i, node) in model.nodes.iter().enumerate() {
        if let (Some(&j), Some(dst)) = (names.get(node.name.as_str()), local.get_mut(i)) {
            if let Some(src) = leader_local.get(j) {
                dst.translation = src.translation;
                dst.rotation = src.rotation;
            }
        }
    }
}

/// Replace the named nodes' local components (procedural bones).
pub fn override_nodes<'a>(
    model: &PbrModel,
    local: &mut [Trs],
    poses: impl IntoIterator<Item = (&'a str, &'a crysirror_scene::components::NodePose)>,
) {
    let names: std::collections::HashMap<&str, usize> = model
        .nodes
        .iter()
        .enumerate()
        .rev()
        .map(|(i, n)| (n.name.as_str(), i))
        .collect();
    for (name, pose) in poses {
        let Some(t) = names.get(name).and_then(|i| local.get_mut(*i)) else {
            continue;
        };
        if let Some(v) = pose.translation {
            t.translation = Vec3::from_array(v);
        }
        if let Some(q) = pose.rotation {
            t.rotation = Quat::from_array(q).normalize();
        }
        if let Some(v) = pose.scale {
            t.scale = Vec3::from_array(v);
        }
    }
}

/// Joint matrices for `skin`, in the space of the skinned mesh node.
pub fn palette(model: &PbrModel, skin: usize, mesh_node: usize, global: &[Mat4]) -> Vec<Mat4> {
    let s = &model.skins[skin];
    let inv_mesh = global[mesh_node].inverse();
    s.joints
        .iter()
        .zip(&s.inverse_bind)
        .map(|(j, ibm)| inv_mesh * global[*j] * *ibm)
        .collect()
}

pub fn clip_index(model: &PbrModel, name: &str) -> Option<usize> {
    model.animations.iter().position(|c| c.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pbr::{Channel, SceneNode, Skin};
    fn two_bone() -> PbrModel {
        let rest = |y: f32| Trs {
            translation: Vec3::new(0., y, 0.),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };
        PbrModel {
            nodes: vec![
                SceneNode {
                    name: "root".into(),
                    children: vec![1],
                    mesh: None,
                    skin: None,
                    rest: rest(0.),
                },
                SceneNode {
                    name: "tip".into(),
                    children: vec![],
                    mesh: None,
                    skin: None,
                    rest: rest(1.),
                },
            ],
            roots: vec![0],
            skins: vec![Skin {
                joints: vec![0, 1],
                inverse_bind: vec![
                    Mat4::IDENTITY,
                    Mat4::from_translation(Vec3::new(0., -1., 0.)),
                ],
            }],
            animations: vec![Clip {
                name: "bend".into(),
                duration: 1.,
                channels: vec![Channel {
                    node: 0,
                    path: ChannelPath::Rotation,
                    interpolation: Interpolation::Linear,
                    times: vec![0., 1.],
                    values: vec![
                        Quat::IDENTITY.to_array(),
                        Quat::from_rotation_z(std::f32::consts::FRAC_PI_2).to_array(),
                    ],
                }],
            }],
            ..Default::default()
        }
    }
    #[test]
    fn hierarchy_sampling_blending_and_palette() {
        let m = two_bone();
        // Rest pose: tip at y=1, palette identity.
        let g = pose(&m, &[]);
        assert!((g[1].w_axis.truncate() - Vec3::Y).length() < 1e-5);
        let p = palette(&m, 0, 0, &g);
        assert!(p.iter().all(|x| x.abs_diff_eq(Mat4::IDENTITY, 1e-5)));
        // Half-way: 45 degrees about Z moves the tip to (-sin45, cos45).
        let g = pose(
            &m,
            &[Layer {
                clip: 0,
                time: 0.5,
                weight: 1.,
            }],
        );
        let tip = g[1].w_axis.truncate();
        assert!(
            (tip - Vec3::new(-0.70710677, 0.70710677, 0.)).length() < 1e-4,
            "{tip}"
        );
        // 50/50 blend of t=0 and t=1 equals the half-way pose.
        let b = pose(
            &m,
            &[
                Layer {
                    clip: 0,
                    time: 0.,
                    weight: 1.,
                },
                Layer {
                    clip: 0,
                    time: 0.999_999,
                    weight: 1.,
                },
            ],
        );
        assert!((b[1].w_axis.truncate() - tip).length() < 1e-3);
        assert_eq!(clip_index(&m, "bend"), Some(0));
    }

    #[test]
    fn retargeted_clips_follow_and_overrides_bind_by_name() {
        let library = two_bone();
        // A part: different node order, an extra node, the same bone names.
        let mut part = two_bone();
        part.animations.clear();
        part.nodes.swap(0, 1);
        part.nodes[1].children = vec![0];
        part.nodes.push(SceneNode {
            name: "ribbon".into(),
            children: vec![],
            mesh: None,
            skin: None,
            rest: Trs { translation: Vec3::X, rotation: Quat::IDENTITY, scale: Vec3::ONE },
        });
        part.roots = vec![1, 2];
        let clip = retarget(&library, &library.animations[0], &part);
        assert_eq!(clip.channels[0].node, 1);
        let mut local = local_pose(&part, &[(&clip, 0.5, 1.)]);
        let expected = local_pose(&library, &[(&library.animations[0], 0.5, 1.)]);
        assert!(local[1].rotation.abs_diff_eq(expected[0].rotation, 1e-6));
        // Following copies rotation/translation of same-named nodes.
        let mut follower = local_pose(&part, &[]);
        follow(&part, &mut follower, &library, &expected);
        assert!(follower[1].rotation.abs_diff_eq(expected[0].rotation, 1e-6));
        assert_eq!(follower[2].translation, Vec3::X);
        // Procedural poses replace only the given components.
        let pose = crysirror_scene::components::NodePose {
            scale: Some([2., 2., 2.]),
            ..Default::default()
        };
        override_nodes(&part, &mut local, [("tip", &pose)]);
        assert_eq!(local[0].scale, Vec3::splat(2.));
        assert_eq!(local[0].translation, Vec3::Y);
        let global = global_pose(&part, &local);
        assert!(global[0].abs_diff_eq(global[1] * local[0].matrix(), 1e-5));
    }
}

/// Evaluate independent character poses on engine CPU workers. Results preserve
/// request order; hierarchy/blend evaluation within a character is unchanged.
pub fn poses_parallel(
    pool: &crysirror_tasks::TaskPool,
    requests: &[(std::sync::Arc<PbrModel>, Vec<Layer>)],
) -> anyhow::Result<Vec<Vec<Mat4>>> {
    let mut out = Vec::with_capacity(requests.len());
    let batch = pool.stats().workers.max(1) * 2;
    for chunk in requests.chunks(batch) {
        let jobs = chunk
            .iter()
            .map(|(model, layers)| {
                let model = model.clone();
                let layers = layers.clone();
                pool.spawn(move |cancel| {
                    cancel.check()?;
                    Ok(pose(&model, &layers))
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        for job in jobs {
            out.push(job.wait()?);
        }
    }
    Ok(out)
}

/// One character's local-pose request: model and its bound clip layers.
pub type LocalPoseRequest = (Arc<PbrModel>, Vec<PoseLayer<Arc<Clip>>>);

/// [`local_pose`] for independent characters on engine CPU workers; results
/// preserve request order.
pub fn local_poses_parallel(
    pool: &crysirror_tasks::TaskPool,
    requests: &[LocalPoseRequest],
) -> anyhow::Result<Vec<Vec<Trs>>> {
    let mut out = Vec::with_capacity(requests.len());
    let batch = pool.stats().workers.max(1) * 2;
    for chunk in requests.chunks(batch) {
        let jobs = chunk
            .iter()
            .map(|(model, layers)| {
                let model = model.clone();
                let layers = layers.clone();
                pool.spawn(move |cancel| {
                    cancel.check()?;
                    let bound: Vec<PoseLayer<&Clip>> = layers
                        .iter()
                        .map(|l| PoseLayer { clip: l.clip.as_ref(), time: l.time, weight: l.weight, looping: l.looping, layer: l.layer })
                        .collect();
                    Ok(local_pose_layered(&model, &bound))
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        for job in jobs {
            out.push(job.wait()?);
        }
    }
    Ok(out)
}

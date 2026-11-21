// SPDX-License-Identifier: MPL-2.0
//! Spring bones: secondary motion of joint chains (hair, skirts, ribbons)
//! on top of the animated pose. Each chain's root follows the animation;
//! every descendant is a Verlet particle pulled by gravity, damped, drawn
//! back toward its animated position (`stiffness`), kept at its bone length,
//! limited in angle and distance and pushed out of sphere/capsule colliders.
//! Parents are rotated so their child points at the particle (VRM spring
//! bone / MagicaCloth BoneCloth style). Simulation runs in world space with per-instance state.
use crate::pbr::{PbrModel, Trs};
use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A sphere, or with `tail` a capsule from `offset` to `tail`, on a node
/// (points and radius in the node's space: they scale with the node).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpringCollider {
    pub node: usize,
    #[serde(default)]
    pub offset: [f32; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tail: Option<[f32; 3]>,
    pub radius: f32,
}

/// One simulated chain rooted at `root` (all of its descendants move).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpringChain {
    pub root: usize,
    /// Acceleration in model units per second² (world direction).
    #[serde(default = "gravity")]
    pub gravity: [f32; 3],
    /// Velocity kept per 1/60 s is `1 − damping`.
    #[serde(default = "half")]
    pub damping: f32,
    /// Fraction of the way back to the animated position per 1/60 s.
    #[serde(default = "tenth")]
    pub stiffness: f32,
    /// Maximum angle from the animated direction in degrees (0: none).
    #[serde(default)]
    pub angle_limit: f32,
    /// Particle radius for collisions.
    #[serde(default)]
    pub radius: f32,
    /// Maximum distance from the animated position (0: none).
    #[serde(default)]
    pub max_distance: f32,
    #[serde(default)]
    pub colliders: Vec<SpringCollider>,
}

fn gravity() -> [f32; 3] {
    [0., -9.8, 0.]
}
fn half() -> f32 {
    0.5
}
fn tenth() -> f32 {
    0.1
}

impl SpringChain {
    pub fn validate(&self, nodes: usize) -> anyhow::Result<()> {
        anyhow::ensure!(self.root < nodes && self.colliders.iter().all(|c| c.node < nodes), "spring chain node index");
        let r = |x: f32, a: f32, b: f32| x.is_finite() && (a..=b).contains(&x);
        anyhow::ensure!(
            r(self.damping, 0., 1.) && r(self.stiffness, 0., 1.) && r(self.angle_limit, 0., 180.) && r(self.radius, 0., 1e4) && r(self.max_distance, 0., 1e4) && self.gravity.iter().all(|g| r(*g, -1e4, 1e4)) && self.colliders.iter().all(|c| r(c.radius, 0., 1e4)),
            "spring chain parameter range"
        );
        Ok(())
    }
}

/// Particles of one model instance: node → (current, previous) world
/// position, and the time of the last step.
#[derive(Clone, Debug, Default)]
pub struct SpringState {
    particles: HashMap<usize, (Vec3, Vec3)>,
    time: Option<f32>,
}

fn parents(model: &PbrModel) -> Vec<Option<usize>> {
    let mut p = vec![None; model.nodes.len()];
    for (i, n) in model.nodes.iter().enumerate() {
        for &c in &n.children {
            if c < p.len() {
                p[c] = Some(i);
            }
        }
    }
    p
}

fn trs_matrix(t: &Trs) -> Mat4 {
    Mat4::from_scale_rotation_translation(t.scale, t.rotation, t.translation)
}

/// Step every chain of `model` at scene `time` (seconds) for an instance at
/// `world`, rewriting the local rotations of the moved joints' parents.
pub fn simulate(model: &PbrModel, chains: &[SpringChain], local: &mut [Trs], world: Mat4, state: &mut SpringState, time: f32) {
    if chains.is_empty() || local.len() != model.nodes.len() {
        return;
    }
    let dt = match state.time {
        Some(t) if time > t => (time - t).min(0.1),
        Some(_) => 0.,
        None => 0.,
    };
    state.time = Some(time);
    let parent = parents(model);
    // World matrices from the (partially simulated) locals.
    let mut global = vec![Mat4::IDENTITY; local.len()];
    let mut order = Vec::with_capacity(local.len());
    let mut stack: Vec<usize> = model.roots.iter().rev().copied().collect();
    while let Some(n) = stack.pop() {
        order.push(n);
        stack.extend(model.nodes[n].children.iter().rev());
    }
    for &n in &order {
        let m = trs_matrix(&local[n]);
        global[n] = match parent[n] {
            Some(p) => global[p] * m,
            None => world * m,
        };
    }
    let steps = dt * 60.;
    for chain in chains {
        if chain.root >= local.len() {
            continue;
        }
        // Breadth-first joints under the root.
        let mut joints = Vec::new();
        let mut queue = std::collections::VecDeque::from(model.nodes[chain.root].children.clone());
        while let Some(j) = queue.pop_front() {
            joints.push(j);
            queue.extend(model.nodes[j].children.iter().copied());
        }
        // Colliders as world segments (a sphere is a zero-length segment).
        let colliders: Vec<(Vec3, Vec3, f32)> = chain
            .colliders
            .iter()
            .map(|c| {
                let m = global[c.node];
                let a = m.transform_point3(Vec3::from_array(c.offset));
                let b = c.tail.map_or(a, |t| m.transform_point3(Vec3::from_array(t)));
                let scale = m.x_axis.truncate().length().max(m.y_axis.truncate().length()).max(m.z_axis.truncate().length());
                (a, b, c.radius * scale + chain.radius)
            })
            .collect();
        let keep = (1. - chain.damping).clamp(0., 1.).powf(steps.max(0.));
        let pull = 1. - (1. - chain.stiffness).clamp(0., 1.).powf(steps.max(0.));
        let g = Vec3::from_array(chain.gravity);
        let cos_limit = (chain.angle_limit.to_radians()).cos();
        for &j in &joints {
            let Some(p) = parent[j] else { continue };
            // The animated position of `j` under its (already moved) parent.
            let animated = global[p] * trs_matrix(&local[j]);
            let anim = animated.w_axis.truncate();
            let origin = global[p].w_axis.truncate();
            let length = (anim - origin).length();
            let (cur, prev) = state.particles.get(&j).copied().unwrap_or((anim, anim));
            let mut next = if dt > 0. { cur + (cur - prev) * keep + g * dt * dt } else { cur };
            next += (anim - next) * pull;
            let anim_dir = (anim - origin).normalize_or_zero();
            let mut dir = (next - origin).normalize_or_zero();
            if dir == Vec3::ZERO {
                dir = anim_dir;
            }
            if chain.angle_limit > 0. && anim_dir != Vec3::ZERO && dir.dot(anim_dir) < cos_limit {
                let axis = anim_dir.cross(dir).normalize_or_zero();
                if axis != Vec3::ZERO {
                    dir = Quat::from_axis_angle(axis, chain.angle_limit.to_radians()) * anim_dir;
                }
            }
            next = origin + dir * length;
            if chain.max_distance > 0. && (next - anim).length() > chain.max_distance {
                next = anim + (next - anim).normalize_or_zero() * chain.max_distance;
                next = origin + (next - origin).normalize_or_zero() * length;
            }
            for (a, b, r) in &colliders {
                let ab = *b - *a;
                let t = if ab.length_squared() > 0. { ((next - *a).dot(ab) / ab.length_squared()).clamp(0., 1.) } else { 0. };
                let c = *a + ab * t;
                let d = next - c;
                if d.length() < *r && *r > 0. {
                    next = c + d.normalize_or_zero() * *r;
                    next = origin + (next - origin).normalize_or_zero() * length;
                }
            }
            state.particles.insert(j, (next, if dt > 0. { cur } else { prev }));
            // Rotate the parent so this child points at the particle (the
            // first simulated child of a branching parent decides).
            if model.nodes[p].children.first() == Some(&j) && anim_dir != Vec3::ZERO {
                let new_dir = (next - origin).normalize_or_zero();
                if new_dir != Vec3::ZERO {
                    let delta = Quat::from_rotation_arc(anim_dir, new_dir);
                    let (ps, prot, ppos) = global[p].to_scale_rotation_translation();
                    let rot = (delta * prot).normalize();
                    let parent_rot = parent[p].map_or_else(|| world.to_scale_rotation_translation().1, |pp| global[pp].to_scale_rotation_translation().1);
                    local[p].rotation = (parent_rot.inverse() * rot).normalize();
                    global[p] = Mat4::from_scale_rotation_translation(ps, rot, ppos);
                }
            }
            global[j] = global[p] * trs_matrix(&local[j]);
        }
        // Descendants outside the chain follow their moved ancestors.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pbr::SceneNode;
    use glam::Vec2;

    fn chain_model() -> PbrModel {
        let node = |name: &str, children: Vec<usize>, y: f32| SceneNode { name: name.into(), children, mesh: None, skin: None, rest: Trs { translation: Vec3::new(0., y, 0.), rotation: Quat::IDENTITY, scale: Vec3::ONE } };
        PbrModel { nodes: vec![node("root", vec![1], 0.), node("a", vec![2], -1.), node("b", vec![], -1.)], roots: vec![0], ..Default::default() }
    }

    #[test]
    fn hanging_chain_swings_back_and_keeps_length() {
        let model = chain_model();
        let chain = SpringChain { root: 0, gravity: [0., -9.8, 0.], damping: 0.2, stiffness: 0.05, angle_limit: 0., radius: 0., max_distance: 0., colliders: vec![] };
        let mut state = SpringState::default();
        let rest: Vec<Trs> = model.nodes.iter().map(|n| n.rest).collect();
        let mut local = rest.clone();
        simulate(&model, &[chain.clone()], &mut local, Mat4::IDENTITY, &mut state, 0.);
        // The root moves sideways: the chain lags behind, then settles.
        let mut lagged = false;
        for i in 1..240 {
            let mut l = rest.clone();
            let world = Mat4::from_translation(Vec3::new(if i < 60 { i as f32 * 0.05 } else { 3. }, 0., 0.));
            simulate(&model, &[chain.clone()], &mut l, world, &mut state, i as f32 / 60.);
            let b = state.particles[&2].0;
            let a = state.particles[&1].0;
            assert!(((a - b).length() - 1.).abs() < 1e-3);
            if i == 30 && b.x < 1.5 - 0.05 {
                lagged = true;
            }
            local = l;
        }
        assert!(lagged);
        let a = state.particles[&1].0;
        assert!((a - Vec3::new(3., -1., 0.)).length() < 0.05, "settled at {a}");
        assert!(local[0].rotation.is_normalized());
    }

    #[test]
    fn capsule_collider_pushes_out() {
        let model = chain_model();
        // Gravity along +x swings the chain toward a capsule (along z through
        // (1, -1.8)) that crosses the tip's arc; the tip must stay outside.
        let chain = SpringChain { root: 0, gravity: [9.8, 0., 0.], damping: 0.1, stiffness: 0., angle_limit: 0., radius: 0., max_distance: 0., colliders: vec![SpringCollider { node: 0, offset: [1., -1.8, -1.], tail: Some([1., -1.8, 1.]), radius: 0.5 }] };
        let mut state = SpringState::default();
        let mut moved = false;
        for i in 0..120 {
            let mut l: Vec<Trs> = model.nodes.iter().map(|n| n.rest).collect();
            simulate(&model, &[chain.clone()], &mut l, Mat4::IDENTITY, &mut state, i as f32 / 60.);
            let b = state.particles[&2].0;
            moved |= b.x > 0.3;
            let d = Vec2::new(b.x - 1., b.y + 1.8).length();
            assert!(d >= 0.5 - 0.02, "tip inside the capsule at {b}");
        }
        assert!(moved);
    }
}

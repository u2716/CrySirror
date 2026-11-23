// SPDX-License-Identifier: MPL-2.0
//! Authored wandering points and ActionPoint.GetAnimationID.
use anyhow::{Context, Result, ensure};
use crysirror_unity::{
    managed::{Fields, Value},
    scene6::Scene6,
    world6,
};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct Point {
    pub id: i32,
    pub name: String,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub motions: Vec<i32>,
    pub destination: glam::Vec3,
    pub area: i32,
    pub spline: bool,
}

/// Read MovePointInfo._wandering.Points in serialized order. The PPtrs
/// reference ActionPoint components, not GameObjects or Transforms.
pub fn points(scene: &Scene6) -> Result<Vec<Point>> {
    let Some(info) = scene
        .nodes
        .iter()
        .flat_map(|n| &n.components)
        .find(|c| c.script == "AL.MovePointInfo")
    else {
        return Ok(Vec::new());
    };
    let refs = info
        .fields
        .fields("_wandering")
        .and_then(|w| w.array("Points"))
        .context("MovePointInfo._wandering.Points")?;
    let components: HashMap<_, _> = scene
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(i, n)| {
            n.components
                .iter()
                .filter(|c| c.script == "AL.ActionPoint")
                .map(move |c| (c.path_id, (i, &c.fields)))
        })
        .collect();
    refs.iter()
        .map(|r| {
            let Value::PPtr(p) = r else {
                anyhow::bail!("invalid wandering point reference")
            };
            ensure!(p.file_id == 0, "external wandering point {}", p.file_id);
            let &(i, f) = components
                .get(&p.path_id)
                .context("missing wandering ActionPoint")?;
            let (_, rot, pos) = world6::world_matrix(scene, i).to_scale_rotation_translation();
            let motions = f
                .array("_animIDList")
                .context("ActionPoint._animIDList")?
                .iter()
                .map(|v| match v {
                    Value::Int(i) => Ok(i32::try_from(*i)?),
                    _ => anyhow::bail!("invalid ActionPoint animation ID"),
                })
                .collect::<Result<_>>()?;
            let mut children = vec![i];
            let mut destination = pos;
            while let Some(child) = children.pop() {
                if scene.nodes[child].name == "n_dest" {
                    destination = world6::world_matrix(scene, child).w_axis.truncate();
                    break;
                }
                children.extend(scene.nodes[child].children.iter().rev());
            }
            Ok(Point {
                id: f.int("_uniqueID").context("ActionPoint._uniqueID")? as i32,
                name: scene.nodes[i].name.clone(),
                position: pos.to_array(),
                rotation: rot.to_array(),
                motions,
                destination,
                area: f.int("_area").context("ActionPoint._area")? as i32,
                spline: f.pptr("_splineContainer").is_some_and(|p| p.path_id != 0),
            })
        })
        .collect()
}

/// PointClass.Initialize shuffles a copy, then chooses a cursor. Selection
/// advances before testing occupancy. A full cycle returns its starting point.
pub struct PointOrder {
    indices: Vec<usize>,
    cursor: usize,
}

impl PointOrder {
    pub fn new(len: usize, mut random: impl FnMut(usize) -> usize) -> Self {
        let mut indices: Vec<_> = (0..len).collect();
        for i in (1..len).rev() {
            indices.swap(i, random(i + 1));
        }
        Self {
            indices,
            cursor: if len > 0 { random(len) } else { 0 },
        }
    }

    pub fn next(&mut self, points: &[Point], occupied: &HashSet<i32>) -> Option<usize> {
        for _ in 0..self.indices.len() {
            self.cursor = (self.cursor + 1) % self.indices.len();
            let index = self.indices[self.cursor];
            if !occupied.contains(&points[index].id) {
                return Some(index);
            }
        }
        self.indices.get(self.cursor).copied()
    }
}

pub fn zone_motions(pd: &Fields) -> Result<Vec<[i32; 4]>> {
    let rows = pd
        .fields("_explore")
        .and_then(|e| e.fields("_motion"))
        .and_then(|m| m.array("TimeZone"))
        .context("PropertyData._explore._motion.TimeZone")?;
    rows.iter()
        .map(|v| {
            let Value::Struct(row) = v else {
                anyhow::bail!("invalid time zone motion row")
            };
            let mut ids = [-1; 4];
            for (i, key) in ["Morning", "Afternoon", "Evening", "Night"]
                .into_iter()
                .enumerate()
            {
                ids[i] = row
                    .int(key)
                    .with_context(|| format!("time zone motion {key}"))?
                    as i32;
            }
            Ok(ids)
        })
        .collect()
}

pub fn activity(
    motions: &[i32],
    row: &[(i32, f32)],
    zones: &[[i32; 4]],
    zone: i32,
    mut random: impl FnMut(usize) -> usize,
) -> Option<i32> {
    if motions.is_empty() {
        return None;
    }
    // The native dictionary is populated in _animIDList order. Duplicate
    // IDs overwrite the same entry and must not gain an extra weight.
    let mut seen = HashSet::new();
    let weighted: Vec<_> = motions
        .iter()
        .filter(|id| seen.insert(**id))
        .filter_map(|id| row.iter().find(|(next, w)| next == id && *w > 0.))
        .collect();
    if weighted.is_empty() {
        return Some(motions[random(motions.len())]);
    }
    let total: f32 = weighted.iter().map(|(_, w)| *w).sum();
    let mut ticket = random(1 << 24) as f32 * (total / (1 << 24) as f32);
    let &(mut id, _) = *weighted.last()?;
    for &&(candidate, weight) in &weighted {
        ticket -= weight;
        if ticket < 0. {
            id = candidate;
            break;
        }
    }
    let variant = zones
        .iter()
        .find(|row| row.contains(&id))
        .and_then(|r| r.get(zone as usize));
    Some(variant.copied().filter(|i| *i != -1).unwrap_or(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_activity_and_native_fallbacks() {
        let zones = [[208, 208, 209, 210], [30, 31, -1, 33]];
        assert_eq!(
            activity(&[30, 208, 208], &[(208, 1.), (30, 1.)], &zones, 3, |n| n
                / 2),
            Some(210)
        );
        assert_eq!(activity(&[30], &[(30, 2.)], &zones, 2, |_| 0), Some(30));
        assert_eq!(
            activity(&[4, 208], &[(8, 2.)], &zones, 3, |n| n - 1),
            Some(208)
        );
        assert_eq!(activity(&[], &[], &zones, 0, |_| panic!()), None);
    }
}

/// NavMeshSurface.AddData registers its asset at the component's world
/// position and rotation. Baked vertices are already in surface units.
#[derive(Debug)]
pub struct Navigation(Vec<(glam::Mat4, crysirror_unity::navmesh::Surface)>);

impl Navigation {
    pub fn load(unity: &mut crate::unity::Unity, info: &super::MapInfo) -> Result<Self> {
        let prefab = crate::level::Level::prefab(
            unity,
            &format!("lib/{}", info.navigation.0),
            &info.navigation.1,
        )?;
        let mut surfaces = Vec::new();
        for (i, node) in prefab.scene.nodes.iter().enumerate() {
            let Some(c) = node.components.iter().find(|c| {
                c.script == "Unity.AI.Navigation.NavMeshSurface"
                    && c.fields.int("m_Enabled") != Some(0)
            }) else {
                continue;
            };
            if std::iter::successors(Some(i), |n| prefab.scene.nodes[*n].parent)
                .any(|n| !prefab.scene.nodes[n].active)
            {
                continue;
            }
            let Some(asset) = c.fields.pptr("m_NavMeshData").filter(|p| p.path_id != 0) else {
                continue;
            };
            let (file, id) = unity.target(&prefab.key, asset)?;
            let (scale, rot, pos) =
                world6::world_matrix(&prefab.scene, i).to_scale_rotation_translation();
            ensure!(scale.is_finite(), "invalid NavMeshSurface placement");
            surfaces.push((
                glam::Mat4::from_rotation_translation(rot, pos),
                crysirror_unity::navmesh::Surface::decode(&unity.decode(&file, id)?)?,
            ));
        }
        Ok(Self(surfaces))
    }

    pub fn polygons(&self) -> usize {
        self.0.iter().map(|(_, s)| s.polygon_count()).sum()
    }

    pub fn route(&self, from: glam::Vec3, to: glam::Vec3, snap: f32) -> Option<Vec<glam::Vec3>> {
        self.0
            .iter()
            .filter_map(|(m, s)| {
                let inv = m.inverse();
                let path = s.path(inv.transform_point3(from), inv.transform_point3(to), snap)?;
                let points: Vec<_> = path.into_iter().map(|p| m.transform_point3(p)).collect();
                let error =
                    from.distance_squared(*points.first()?) + to.distance_squared(*points.last()?);
                Some((error, points))
            })
            .min_by(|(d, _), (e, _)| d.total_cmp(e))
            .map(|(_, p)| p)
    }
}

pub struct Rules {
    action: [i32; 2],
    thinking: [i32; 2],
    pub move_ratio: i32,
    walk: i32,
    pool: [i32; 2],
    sea: i32,
    personal: Vec<f32>,
    height: [f32; 3],
    swim: [f32; 3],
}

impl Rules {
    pub fn read(pd: &Fields) -> Result<Self> {
        let explore = pd.fields("_explore").context("PropertyData._explore")?;
        let npc = explore.fields("_npc").context("explore._npc")?;
        let motion = explore.fields("_motion").context("explore._motion")?;
        let int = |f: &Fields, k: &str| {
            f.int(k)
                .map(|i| i as i32)
                .with_context(|| format!("missing {k}"))
        };
        let speeds = |key: &str| -> Result<[f32; 3]> {
            motion
                .floats(key)
                .and_then(|s| s.try_into().ok())
                .with_context(|| format!("missing {key}"))
        };
        Ok(Self {
            action: [int(npc, "_actionTimeMin")?, int(npc, "_actionTimeMax")?],
            thinking: [int(npc, "_thinkingTimeMin")?, int(npc, "_thinkingTimeMax")?],
            move_ratio: int(npc, "_moveRatio")?,
            walk: int(motion, "Walk")?,
            pool: [int(motion, "PoolSwim_SM")?, int(motion, "PoolSwim_ML")?],
            sea: int(motion, "SeaSwim")?,
            personal: motion
                .floats("WalkPersonalVelocity")
                .context("WalkPersonalVelocity")?
                .to_vec(),
            height: speeds("WalkHeightVelocity")?,
            swim: speeds("SwimHeightVelocity")?,
        })
    }

    pub fn wait(&self, thinking: bool, mut random: impl FnMut(usize) -> usize) -> f32 {
        let [lo, hi] = if thinking { self.thinking } else { self.action };
        (lo + if hi > lo {
            random((hi - lo) as usize) as i32
        } else {
            0
        }) as f32
    }

    pub fn locomotion(&self, area: i32, height: f32, personality: i64) -> (i32, f32) {
        let lerp = |v: [f32; 3]| {
            let h = height.clamp(0., 1.) * 2.;
            let i = usize::from(h >= 1.);
            v[i] + (v[i + 1] - v[i]) * (h - i as f32)
        };
        match area {
            1 => (self.pool[usize::from(height >= 0.5)], lerp(self.swim)),
            2 => (self.sea, lerp(self.swim)),
            _ => (
                self.walk,
                self.personal
                    .get(personality as usize)
                    .map_or(1., |s| s * lerp(self.height)),
            ),
        }
    }
}

pub struct Actor {
    pub point: usize,
    pub phase: Phase,
}

pub enum Phase {
    Action(f32),
    Thinking(f32),
    Moving {
        path: std::collections::VecDeque<glam::Vec3>,
        position: glam::Vec3,
        speed: f32,
    },
    Unsupported,
}

pub struct Population {
    pub points: Vec<Point>,
    pub order: PointOrder,
    pub nav: Navigation,
    pub actors: std::collections::BTreeMap<String, Actor>,
}

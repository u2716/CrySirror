// SPDX-License-Identifier: MPL-2.0
//! Polygons, detail triangles and portals in Unity 6 baked DNAV v16 tiles.
use crate::managed::{Fields, Value};
use anyhow::{Context, Result, ensure};
use glam::Vec3;
use std::collections::{HashMap, VecDeque};

#[derive(Debug)]
struct Polygon {
    detail: Vec<[Vec3; 3]>,
    /// Neighbor polygon and its traversable edge segment.
    exits: Vec<(usize, Vec3, Vec3)>,
}

#[derive(Debug, Default)]
pub struct Surface {
    polygons: Vec<Polygon>,
}

struct Border {
    poly: usize,
    side: u16,
    a: Vec3,
    b: Vec3,
}

fn u16_at(data: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([data[i], data[i + 1]])
}

fn u32_at(data: &[u8], i: usize) -> u32 {
    u32::from_le_bytes(data[i..i + 4].try_into().unwrap())
}

fn vertex(data: &[u8], i: usize) -> Result<Vec3> {
    let v = Vec3::from_array([0, 4, 8].map(|n| f32::from_bits(u32_at(data, i + n))));
    ensure!(v.is_finite(), "non-finite DNAV vertex");
    Ok(v)
}

impl Surface {
    pub fn decode(fields: &Fields) -> Result<Self> {
        let mut surface = Self::default();
        let mut borders: HashMap<(i32, i32), Vec<Border>> = HashMap::new();
        for tile in fields.array("m_NavMeshTiles").context("m_NavMeshTiles")? {
            let Value::Struct(fields) = tile else {
                anyhow::bail!("invalid NavMesh tile")
            };
            let Some(Value::Bytes(b)) = fields.get("m_MeshData") else {
                anyhow::bail!("missing m_MeshData")
            };
            ensure!(
                b.len() >= 72 && &b[..4] == b"VAND" && u32_at(b, 4) == 16,
                "unsupported DNAV tile"
            );
            let coord = (u32_at(b, 8) as i32, u32_at(b, 12) as i32);
            let [pc, vc, mc, dv, dt, bv] = [20, 24, 28, 32, 36, 40].map(|i| u32_at(b, i) as u64);
            ensure!(
                mc == pc
                    && 72 + vc * 12 + pc * 32 + mc * 12 + dv * 12 + dt * 8 + bv * 16
                        == b.len() as u64,
                "invalid DNAV table sizes"
            );
            let (pc, vc, dv, dt) = (pc as usize, vc as usize, dv as usize, dt as usize);
            let polys = 72 + vc * 12;
            let meshes = polys + pc * 32;
            let extras = meshes + pc * 12;
            let tris = extras + dv * 12;
            let base = surface.polygons.len();
            for j in 0..pc {
                let o = polys + j * 32;
                let count = usize::from(b[o + 28]);
                ensure!(
                    (3..=6).contains(&count),
                    "invalid DNAV polygon vertex count"
                );
                let mut vertices = Vec::with_capacity(count);
                for k in 0..count {
                    let i = usize::from(u16_at(b, o + 2 * k));
                    ensure!(i < vc, "invalid DNAV vertex index");
                    vertices.push(vertex(b, 72 + i * 12)?);
                }
                let m = meshes + j * 12;
                let (first_v, first_t) = (u32_at(b, m) as usize, u32_at(b, m + 4) as usize);
                let (num_v, num_t) = (
                    usize::from(u16_at(b, m + 8)),
                    usize::from(u16_at(b, m + 10)),
                );
                ensure!(
                    first_v <= dv
                        && num_v <= dv - first_v
                        && first_t <= dt
                        && num_t <= dt - first_t,
                    "invalid DNAV detail ranges"
                );
                let mut detail = Vec::with_capacity(num_t);
                for t in first_t..first_t + num_t {
                    let mut triangle = [Vec3::ZERO; 3];
                    for (k, v) in triangle.iter_mut().enumerate() {
                        let i = usize::from(u16_at(b, tris + t * 8 + k * 2));
                        ensure!(i < count + num_v, "invalid DNAV detail vertex");
                        *v = if i < count {
                            vertices[i]
                        } else {
                            vertex(b, extras + (first_v + i - count) * 12)?
                        };
                    }
                    detail.push(triangle);
                }
                let mut exits = Vec::new();
                for k in 0..count {
                    let link = u16_at(b, o + 12 + 2 * k);
                    let (a, b) = (vertices[k], vertices[(k + 1) % count]);
                    if link & 0x8000 != 0 {
                        let side = link & 0x7fff;
                        ensure!(
                            matches!(side, 0 | 2 | 4 | 6),
                            "unsupported DNAV portal side {side}"
                        );
                        borders.entry(coord).or_default().push(Border {
                            poly: base + j,
                            side,
                            a,
                            b,
                        });
                    } else if link > 0 {
                        ensure!(usize::from(link) <= pc, "invalid DNAV neighbor");
                        exits.push((base + usize::from(link) - 1, a, b));
                    }
                }
                surface.polygons.push(Polygon { detail, exits });
            }
        }
        let climb = fields
            .fields("m_NavMeshBuildSettings")
            .and_then(|s| s.float("agentClimb"))
            .context("NavMesh agentClimb")? as f32;
        for (&(x, z), edges) in &borders {
            for e in edges {
                // Link each tile pair once. Horizontal/vertical boundaries may
                // be split into different numbers of edges on opposite tiles.
                let (axis, neighbor) = match e.side {
                    0 => (2, (x + 1, z)),
                    2 => (0, (x, z + 1)),
                    _ => continue,
                };
                for f in borders
                    .get(&neighbor)
                    .into_iter()
                    .flatten()
                    .filter(|f| f.side == e.side + 4)
                {
                    let l = e.a[axis].min(e.b[axis]).max(f.a[axis].min(f.b[axis]));
                    let r = e.a[axis].max(e.b[axis]).min(f.a[axis].max(f.b[axis]));
                    if r - l < 0.001 {
                        continue;
                    }
                    let interp = |v: &Border, s: f32| {
                        v.a.lerp(v.b, (s - v.a[axis]) / (v.b[axis] - v.a[axis]))
                    };
                    let (a, b, c, d) = (interp(e, l), interp(e, r), interp(f, l), interp(f, r));
                    let dy0 = a.y - c.y;
                    let dy1 = b.y - d.y;
                    if dy0 * dy1 > 0. && dy0.abs().min(dy1.abs()) > 2. * climb {
                        continue;
                    }
                    surface.polygons[e.poly].exits.push((f.poly, a, b));
                    surface.polygons[f.poly].exits.push((e.poly, c, d));
                }
            }
        }
        Ok(surface)
    }

    pub fn polygon_count(&self) -> usize {
        self.polygons.len()
    }

    fn nearest(&self, at: Vec3, radius: f32) -> Option<(usize, Vec3)> {
        let mut best = None;
        let mut distance = radius * radius;
        for (i, p) in self.polygons.iter().enumerate() {
            for &[a, b, c] in &p.detail {
                let cross = |u: Vec3, v: Vec3| u.x * v.z - u.z * v.x;
                let denom = cross(b - a, c - a);
                if denom.abs() < 0.000001 {
                    continue;
                }
                let u = cross(at - a, c - a) / denom;
                let v = cross(b - a, at - a) / denom;
                let point = if u >= 0. && v >= 0. && u + v <= 1. {
                    a + u * (b - a) + v * (c - a)
                } else {
                    let edge = |a: Vec3, b: Vec3| {
                        let delta = b - a;
                        a + delta
                            * ((at - a).dot(delta) / delta.length_squared().max(f32::EPSILON))
                                .clamp(0., 1.)
                    };
                    [edge(a, b), edge(b, c), edge(c, a)]
                        .into_iter()
                        .min_by(|u, v| u.distance_squared(at).total_cmp(&v.distance_squared(at)))
                        .unwrap()
                };
                let d = point.distance_squared(at);
                if d <= distance {
                    distance = d;
                    best = Some((i, point));
                }
            }
        }
        best
    }

    /// A connected polygon corridor. Crossing points lie on the shared
    /// traversable portals. Positions are in the NavMeshSurface's local space.
    pub fn path(&self, from: Vec3, to: Vec3, snap: f32) -> Option<Vec<Vec3>> {
        let (start, a) = self.nearest(from, snap)?;
        let (end, b) = self.nearest(to, snap)?;
        let mut parent = vec![None; self.polygons.len()];
        let mut todo = VecDeque::from([start]);
        parent[start] = Some((start, a));
        while let Some(i) = todo.pop_front() {
            if i == end {
                break;
            }
            for &(next, l, r) in &self.polygons[i].exits {
                if parent[next].is_none() {
                    parent[next] = Some((i, (l + r) * 0.5));
                    todo.push_back(next);
                }
            }
        }
        let mut out = vec![b];
        let mut i = end;
        while i != start {
            let (prev, portal) = parent[i]?;
            out.push(portal);
            i = prev;
        }
        out.push(a);
        out.reverse();
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rectangle(x: i32, side: u16, edge: usize) -> Value {
        let mut data = vec![0u8; 72 + 4 * 12 + 32 + 12 + 2 * 8];
        data[..4].copy_from_slice(b"VAND");
        for (off, n) in [(4, 16), (8, x), (20, 1), (24, 4), (28, 1), (36, 2)] {
            data[off..off + 4].copy_from_slice(&n.to_le_bytes());
        }
        let left = x as f32;
        for (i, v) in [
            [left, 0., 0.],
            [left + 1., 0., 0.],
            [left + 1., 0., 1.],
            [left, 0., 1.],
        ]
        .into_iter()
        .enumerate()
        {
            for (j, c) in v.into_iter().enumerate() {
                let off = 72 + i * 12 + j * 4;
                data[off..off + 4].copy_from_slice(&c.to_le_bytes());
            }
        }
        let p = 120;
        for i in 0..4u16 {
            let off = p + usize::from(i) * 2;
            data[off..off + 2].copy_from_slice(&i.to_le_bytes());
        }
        let e = p + 12 + 2 * edge;
        data[e..e + 2].copy_from_slice(&side.to_le_bytes());
        data[p + 28] = 4;
        data[162..164].copy_from_slice(&2u16.to_le_bytes());
        for (i, v) in [0u16, 1, 2, 0, 0, 2, 3, 0].iter().enumerate() {
            data[164 + 2 * i..166 + 2 * i].copy_from_slice(&v.to_le_bytes());
        }
        Value::Struct(Fields(vec![("m_MeshData".into(), Value::Bytes(data))]))
    }

    #[test]
    fn dnav_connects_only_reciprocal_tile_portals() {
        let a = Vec3::new(0.25, 0., 0.5);
        let b = Vec3::new(1.75, 0., 0.5);
        let decode = |other| {
            Surface::decode(&Fields(vec![
                (
                    "m_NavMeshTiles".into(),
                    Value::Array(vec![rectangle(0, 0x8000, 1), rectangle(1, other, 3)]),
                ),
                (
                    "m_NavMeshBuildSettings".into(),
                    Value::Struct(Fields(vec![("agentClimb".into(), Value::Float(0.3))])),
                ),
            ]))
            .unwrap()
        };
        assert_eq!(
            decode(0x8004).path(a, b, 0.1).unwrap(),
            vec![a, Vec3::new(1., 0., 0.5), b]
        );
        assert!(decode(0).path(a, b, 0.1).is_none());
    }
}

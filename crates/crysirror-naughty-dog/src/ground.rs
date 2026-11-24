// SPDX-License-Identifier: MPL-2.0
//! Walkable surfaces of the placed level meshes for keeping the player on
//! the ground: upward-facing triangles bucketed in a horizontal grid.
//! Native collision (Havok data in the level packages) is not used; this
//! is the viewer's ground model (SATD-nd-retail-runtime).
use crate::{mesh::Mesh, scene};
use std::collections::HashMap;

const CELL: f32 = 4.;
/// Triangles steeper than this (normal y below) are not walkable.
const MIN_UP: f32 = 0.5;
const MAX_TRIANGLES: usize = 6 << 20;

#[derive(Default)]
pub struct Ground {
    tris: Vec<[[f32; 3]; 3]>,
    cells: HashMap<(i32, i32), Vec<u32>>,
}

fn cell(v: f32) -> i32 {
    (v / CELL).floor() as i32
}

impl Ground {
    pub fn len(&self) -> usize {
        self.tris.len()
    }
    pub fn is_empty(&self) -> bool {
        self.tris.is_empty()
    }

    /// Add the walkable triangles of placed meshes (metres).
    pub fn add(&mut self, meshes: &[Mesh]) {
        self.add_placed(meshes, None);
    }

    /// Add placed meshes moved by a column-major world matrix (an actor's
    /// spawner transform).
    pub fn add_placed(&mut self, meshes: &[Mesh], world: Option<[f32; 16]>) {
        for m in meshes {
            for i in 0..m.instances.len() {
                let local = scene::placement(m, i);
                let t = match world {
                    None => local,
                    Some(w) => std::array::from_fn(|k| {
                        let (c, r) = (k / 4, k % 4);
                        (0..4).map(|j| w[j * 4 + r] * local[c * 4 + j]).sum()
                    }),
                };
                for f in &m.fragments {
                    for tri in &f.triangles {
                        if self.tris.len() >= MAX_TRIANGLES {
                            return;
                        }
                        let p = tri.map(|v| {
                            let p = f.positions[v as usize].map(|c| c * scene::METRES_PER_UNIT);
                            std::array::from_fn(|c| p[0] * t[c] + p[1] * t[4 + c] + p[2] * t[8 + c] + t[12 + c])
                        });
                        let (u, w) = (sub(p[1], p[0]), sub(p[2], p[0]));
                        let n = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
                        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                        if !(len > 1e-8) || (n[1] / len).abs() < MIN_UP {
                            continue;
                        }
                        let id = self.tris.len() as u32;
                        self.tris.push(p);
                        let (x0, x1) = (cell(p.iter().map(|v| v[0]).fold(f32::MAX, f32::min)), cell(p.iter().map(|v| v[0]).fold(f32::MIN, f32::max)));
                        let (z0, z1) = (cell(p.iter().map(|v| v[2]).fold(f32::MAX, f32::min)), cell(p.iter().map(|v| v[2]).fold(f32::MIN, f32::max)));
                        if (x1 - x0) > 64 || (z1 - z0) > 64 {
                            continue;
                        }
                        for x in x0..=x1 {
                            for z in z0..=z1 {
                                self.cells.entry((x, z)).or_default().push(id);
                            }
                        }
                    }
                }
            }
        }
    }

    /// The highest walkable surface at `(x, z)` not above `top`.
    pub fn height(&self, x: f32, z: f32, top: f32) -> Option<f32> {
        let mut best: Option<f32> = None;
        for id in self.cells.get(&(cell(x), cell(z))).into_iter().flatten() {
            let [a, b, c] = self.tris[*id as usize];
            let d = (b[2] - c[2]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[2] - c[2]);
            if d.abs() < 1e-9 {
                continue;
            }
            let l1 = ((b[2] - c[2]) * (x - c[0]) + (c[0] - b[0]) * (z - c[2])) / d;
            let l2 = ((c[2] - a[2]) * (x - c[0]) + (a[0] - c[0]) * (z - c[2])) / d;
            let l3 = 1. - l1 - l2;
            if l1 < -1e-4 || l2 < -1e-4 || l3 < -1e-4 {
                continue;
            }
            let y = l1 * a[1] + l2 * b[1] + l3 * c[1];
            if y <= top && best.is_none_or(|h| y > h) {
                best = Some(y);
            }
        }
        best
    }
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

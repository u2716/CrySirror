// SPDX-License-Identifier: MPL-2.0
//! Unit check against Havok background collision (metres): every vertex
//! array of a package's `COLLISION_DATA_HAVOK_BG` shapes (`{…, count at
//! +0x10, pointer at +0x14, stride at +0x18}` records referenced from the
//! item) is gathered, and placed render vertices are matched to their
//! nearest collision vertex for candidate units. The right unit gives the
//! smallest median distance. `collision_fit PAK`
use anyhow::Result;
use crysirror_core::service::MemoryAssetReader;
use crysirror_naughty_dog::{mesh::Loaded, package::Directory, scene};
use std::collections::HashMap;

fn main() -> Result<()> {
    let path = std::env::args().nth(1).expect("PAK");
    let source = MemoryAssetReader(std::fs::read(&path)?.into());
    let directory = Directory::read(&source)?;
    let p = Loaded::read(&source, 1 << 30)?;
    // Collision vertices: scan pointer targets reachable from the item for
    // vertex-array records (count, pointer, stride 12/16).
    let mut verts: Vec<[f32; 3]> = Vec::new();
    for item in directory.items.iter().filter(|i| i.kind == "COLLISION_DATA_HAVOK_BG") {
        let at = item.payload_offset as usize;
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![at];
        while let Some(a) = stack.pop() {
            if !seen.insert(a) || seen.len() > 4096 {
                continue;
            }
            for k in 0..16 {
                if let Ok(t) = p.ptr(a + 4 * k) {
                    stack.push(t);
                }
            }
            let (Ok(n), Ok(arr), Ok(stride)) = (p.u32(a + 0x10), p.ptr(a + 0x14), p.u32(a + 0x18)) else { continue };
            if (stride == 12 || stride == 16) && (3..200_000).contains(&n) {
                for i in 0..n as usize {
                    let v = [p.f32(arr + stride as usize * i)?, p.f32(arr + stride as usize * i + 4)?, p.f32(arr + stride as usize * i + 8)?];
                    if v.iter().all(|x| x.is_finite() && x.abs() < 1e5) && v != [0.; 3] {
                        verts.push(v);
                    }
                }
            }
        }
    }
    println!("{} collision vertices", verts.len());
    let cell = 2f32;
    let mut grid: HashMap<(i32, i32, i32), Vec<[f32; 3]>> = HashMap::new();
    let key = |v: [f32; 3]| ((v[0] / cell).floor() as i32, (v[1] / cell).floor() as i32, (v[2] / cell).floor() as i32);
    for v in &verts {
        grid.entry(key(*v)).or_default().push(*v);
    }
    let nearest = |q: [f32; 3]| {
        let c = key(q);
        let mut best = f32::MAX;
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    for v in grid.get(&(c.0 + dx, c.1 + dy, c.2 + dz)).into_iter().flatten() {
                        best = best.min((0..3).map(|k| (v[k] - q[k]).powi(2)).sum::<f32>().sqrt());
                    }
                }
            }
        }
        best
    };
    let meshes = scene::package_meshes(&source, &directory, 1 << 30)?;
    for unit in [0.01f32, 0.05, 0.06, 0.0625, 0.064, 0.07] {
        for centre in [true, false] {
            let mut d = Vec::new();
            for m in &meshes {
                for i in 0..m.instances.len() {
                    let mut t = m.instances[i];
                    if centre && t[12] == 0. && t[13] == 0. && t[14] == 0. {
                        t[12..15].copy_from_slice(&m.spheres[i][..3]);
                    }
                    for f in &m.fragments {
                        for p in f.positions.iter().step_by(7) {
                            let p = p.map(|c| c * unit);
                            let w: [f32; 3] = std::array::from_fn(|c| p[0] * t[c] + p[1] * t[4 + c] + p[2] * t[8 + c] + t[12 + c]);
                            d.push(nearest(w));
                        }
                    }
                }
            }
            let hit: Vec<f32> = d.iter().copied().filter(|x| *x < f32::MAX).collect();
            let mut s = hit.clone();
            s.sort_by(f32::total_cmp);
            println!("unit {unit:.4} sphere-centre {centre}: {} of {} within 3 m of collision, median {:.2}", hit.len(), d.len(), s.get(s.len() / 2).copied().unwrap_or(f32::NAN));
        }
    }
    Ok(())
}

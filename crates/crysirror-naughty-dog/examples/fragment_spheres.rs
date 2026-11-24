// SPDX-License-Identifier: MPL-2.0
//! Instance +0x14 holds one world-space bounding sphere per fragment.
//! For each mesh's first instance: the translation implied by every
//! fragment (sphere centre minus the fragment's decoded bounds centre
//! through the instance rotation) and the radius against the fragment's
//! decoded half-diagonal. `fragment_spheres PAK…`
use anyhow::Result;
use crysirror_core::service::MemoryAssetReader;
use crysirror_naughty_dog::{mesh::Loaded, package::Directory, scene};

fn main() -> Result<()> {
    for path in std::env::args().skip(1) {
        let source = MemoryAssetReader(std::fs::read(&path)?.into());
        let directory = Directory::read(&source)?;
        let p = Loaded::read(&source, 1 << 30)?;
        let meshes = scene::package_meshes(&source, &directory, 1 << 30)?;
        let mut records = Vec::new();
        for item in directory.items.iter().filter(|i| i.kind == "PM_COLLECTION_2") {
            let payload = item.payload_offset as usize;
            let n = p.u32(payload)? as usize;
            if n == 0x1001 {
                continue;
            }
            records.extend((0..n).map(|e| p.ptr(payload + 4 + 4 * e)));
        }
        let k: f32 = std::env::var("SCALE").ok().and_then(|v| v.parse().ok()).unwrap_or(1.);
        let mut meshes = meshes;
        for m in &mut meshes {
            m.unit *= k;
        }
        for (mi, (m, rec)) in meshes.iter().zip(records).enumerate() {
            let Ok(rec) = rec else { continue };
            if m.instances.is_empty() {
                continue;
            }
            let inst = p.ptr(p.ptr(rec + 4)?)?;
            let Ok(spheres) = p.ptr(inst + 0x14) else { continue };
            let t = m.instances[0];
            let mut rows = Vec::new();
            for (fi, f) in m.fragments.iter().enumerate() {
                if f.positions.is_empty() {
                    continue;
                }
                let s: [f32; 4] = std::array::from_fn(|k| p.f32(spheres + 16 * fi + 4 * k).unwrap_or(f32::NAN));
                let lo: [f32; 3] = std::array::from_fn(|k| f.positions.iter().map(|v| v[k]).fold(f32::MAX, f32::min) * m.unit);
                let hi: [f32; 3] = std::array::from_fn(|k| f.positions.iter().map(|v| v[k]).fold(f32::MIN, f32::max) * m.unit);
                let c: [f32; 3] = std::array::from_fn(|k| (lo[k] + hi[k]) * 0.5);
                let half = (0..3).map(|k| ((hi[k] - lo[k]) * 0.5).powi(2)).sum::<f32>().sqrt();
                let rc: [f32; 3] = std::array::from_fn(|j| c[0] * t[j] + c[1] * t[4 + j] + c[2] * t[8 + j]);
                rows.push(([s[0] - rc[0], s[1] - rc[1], s[2] - rc[2]], s[3] / half.max(1e-6)));
            }
            if rows.is_empty() {
                continue;
            }
            // Refine: a translation correction d with every vertex inside its
            // fragment sphere (alternating projections onto the balls).
            let mut d = [0f32; 3];
            {
                let mut pts = Vec::new();
                for (fi, f) in m.fragments.iter().enumerate() {
                    let s: [f32; 4] = std::array::from_fn(|k| p.f32(spheres + 16 * fi + 4 * k).unwrap_or(f32::NAN));
                    for v in f.positions.iter().step_by(3) {
                        let q = v.map(|c| c * m.unit);
                        let w: [f32; 3] = std::array::from_fn(|j| q[0] * t[j] + q[1] * t[4 + j] + q[2] * t[8 + j] + t[12 + j]);
                        pts.push(([s[0] - w[0], s[1] - w[1], s[2] - w[2]], s[3]));
                    }
                }
                let n = rows.len() as f32;
                let m0: [f32; 3] = std::array::from_fn(|k| rows.iter().map(|r| r.0[k]).sum::<f32>() / n);
                d = [m0[0] - t[12], m0[1] - t[13], m0[2] - t[14]];
                for _ in 0..200 {
                    let mut moved = false;
                    for (c, r) in &pts {
                        let e = [d[0] - c[0], d[1] - c[1], d[2] - c[2]];
                        let len = (e[0] * e[0] + e[1] * e[1] + e[2] * e[2]).sqrt();
                        if len > *r {
                            let k = (*r * 0.999) / len;
                            d = [c[0] + e[0] * k, c[1] + e[1] * k, c[2] + e[2] * k];
                            moved = true;
                        }
                    }
                    if !moved {
                        break;
                    }
                }
                let worst = pts.iter().map(|(c, r)| (((d[0] - c[0]).powi(2) + (d[1] - c[1]).powi(2) + (d[2] - c[2]).powi(2)).sqrt() - r)).fold(f32::MIN, f32::max);
                println!("    refined correction {:?} worst violation {:.3} m", d.map(|v| (v * 100.).round() / 100.), worst);
            }
            let spread = |k: usize| rows.iter().map(|r| r.0[k]).fold(f32::MIN, f32::max) - rows.iter().map(|r| r.0[k]).fold(f32::MAX, f32::min);
            let mean: Vec<f32> = (0..3).map(|k| rows.iter().map(|r| r.0[k]).sum::<f32>() / rows.len() as f32).collect();
            let mut ratio: Vec<f32> = rows.iter().map(|r| r.1).collect();
            ratio.sort_by(f32::total_cmp);
            println!(
                "{mi:3} frags {:3} matrix t {:?} sphere-centre {:?} implied t {:?} spread {:?} radius/half-diag median {:.2}",
                rows.len(),
                [t[12], t[13], t[14]].map(|v| v.round()),
                m.spheres.first().map(|s| [s[0], s[1], s[2]].map(|v| v.round())),
                mean.iter().map(|v| v.round()).collect::<Vec<_>>(),
                [spread(0), spread(1), spread(2)].map(|v| (v * 10.).round() / 10.),
                ratio[ratio.len() / 2]
            );
        }
    }
    Ok(())
}

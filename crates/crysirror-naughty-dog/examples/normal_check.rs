// SPDX-License-Identifier: MPL-2.0
//! Compare decoded vertex normals with the triangles' geometric normals:
//! the mean cosine per package, also with the components permuted or
//! negated, to validate the normal decode.
use anyhow::Result;
use crysirror_core::service::MemoryAssetReader;
use crysirror_naughty_dog::{package::Directory, scene};
use glam::Vec3;

fn main() -> Result<()> {
    for path in std::env::args().skip(1) {
        let source = MemoryAssetReader(std::fs::read(&path)?.into());
        let directory = Directory::read(&source)?;
        let variants: [(&str, fn(Vec3) -> Vec3); 4] = [("as decoded", |n| n), ("negated", |n| -n), ("y negated", |n| Vec3::new(n.x, -n.y, n.z)), ("x,z negated", |n| Vec3::new(-n.x, n.y, -n.z))];
        let mut sums = [0f64; 4];
        let mut count = 0usize;
        let mut up = 0usize;
        for m in scene::package_meshes(&source, &directory, 256 << 20)? {
            for f in &m.fragments {
                if f.normals.len() != f.positions.len() {
                    continue;
                }
                for t in &f.triangles {
                    let [a, b, c] = t.map(|i| Vec3::from_array(f.positions[usize::from(i)]));
                    let g = (b - a).cross(c - a);
                    if g.length() < 1e-8 {
                        continue;
                    }
                    let g = g.normalize();
                    for i in t {
                        let n = Vec3::from_array(f.normals[usize::from(*i)]).normalize_or_zero();
                        for (k, (_, v)) in variants.iter().enumerate() {
                            sums[k] += f64::from(v(n).dot(g));
                        }
                        up += usize::from(n.y > 0.7);
                        count += 1;
                    }
                }
            }
        }
        let n = count.max(1) as f64;
        println!("{path}: {count} corners, n.y > 0.7 for {:.1}%", 100. * up as f64 / n);
        for (k, (name, _)) in variants.iter().enumerate() {
            println!("  {name:12} mean cos {:.3}", sums[k] / n);
        }
    }
    Ok(())
}

// SPDX-License-Identifier: MPL-2.0
//! Large flat placed meshes (column-major placements) (horizontal extent over 40 m, height under
//! 4 m) of the given packages with their materials: water, cloud and
//! fog planes.
use anyhow::Result;
use crysirror_core::service::MemoryAssetReader;
use crysirror_naughty_dog::{package::Directory, scene};

fn main() -> Result<()> {
    for path in std::env::args().skip(1) {
        let source = MemoryAssetReader(std::fs::read(&path)?.into());
        let directory = Directory::read(&source)?;
        for (index, m) in scene::package_meshes(&source, &directory, 256 << 20)?.iter().enumerate() {
            for i in 0..m.instances.len() {
                let t = scene::placement(m, i);
                let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
                for p in m.fragments.iter().flat_map(|f| &f.positions) {
                    for c in 0..3 {
                        let v = p[0] * t[c] + p[1] * t[4 + c] + p[2] * t[8 + c] + t[12 + c];
                        lo[c] = lo[c].min(v);
                        hi[c] = hi[c].max(v);
                    }
                }
                let (w, h, d) = (hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]);
                if w.max(d) > 40. && h < 4. {
                    let names: Vec<&str> = m.groups.iter().map(|g| g.material.name.as_str()).collect();
                    println!("{path}: mesh {index}/{i} {w:.0}×{h:.1}×{d:.0} m at y {:.1}: {names:?}", lo[1]);
                }
            }
        }
    }
    Ok(())
}

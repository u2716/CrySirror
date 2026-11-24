// SPDX-License-Identifier: MPL-2.0
//! Compare each art group's `vis_sphere` tag (centre, radius in metres)
//! with the native bounds of its decoded meshes.
use anyhow::Result;
use crysirror_core::service::MemoryAssetReader;
use crysirror_naughty_dog::{mesh::Loaded, package::Directory, scene};

fn main() -> Result<()> {
    for path in std::env::args().skip(1) {
        let source = MemoryAssetReader(std::fs::read(&path)?.into());
        let directory = Directory::read(&source)?;
        let loaded = Loaded::read(&source, 256 << 20)?;
        let Some(item) = directory.items.iter().find(|i| i.kind == "TAG_VEC4" && i.name == "vis_sphere") else { continue };
        let at = item.payload_offset as usize;
        let v: Vec<f32> = (0..4).map(|k| loaded.f32(at + 4 * k).unwrap_or(f32::NAN)).collect();
        let meshes = scene::meshes_of(&loaded, &directory)?;
        let Some((lo, hi)) = scene::bounds(&meshes) else { continue };
        let c: Vec<f32> = (0..3).map(|k| (lo[k] + hi[k]) / 2.).collect();
        let r = (0..3).map(|k| ((hi[k] - lo[k]) / 2.).powi(2)).sum::<f32>().sqrt();
        let name = path.rsplit('/').next().unwrap_or(&path);
        let n = meshes.iter().map(|m| m.grid_bits).max().unwrap_or(0);
        // Hypothesis: millimetres × 2^(N − 12); decoded positions are native
        // units, bounds above are in those units.
        let predicted = r * 2f32.powi(n as i32 - 12) * 0.001;
        print!("N={n:2} predicted r={predicted:.2} ratio {:.2}  ", v[3] / predicted);
        println!("{name:40} sphere c=({:.2},{:.2},{:.2}) r={:.2}  native c=({:.1},{:.1},{:.1}) r={:.1}  r/r_native={:.5}", v[0], v[1], v[2], v[3], c[0], c[1], c[2], r, v[3] / r);
    }
    Ok(())
}

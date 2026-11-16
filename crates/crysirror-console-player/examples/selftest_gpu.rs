// SPDX-License-Identifier: MPL-2.0
//! The console renderer self-test on this machine's GPU (desktop reference
//! for the PS Vita `selftest-gxm` run):
//!   cargo run --release -p crysirror-console-player --example selftest_gpu -- OUT_DIR
use crysirror_console_player::selftest;
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    let out = std::path::PathBuf::from(std::env::args().nth(1).expect("OUT_DIR"));
    std::fs::create_dir_all(&out)?;
    let gpu = Arc::new(pollster::block_on(crysirror_wgpu::Gpu::request(&wgpu::Instance::default(), None))?);
    println!("gpu: {:?}", gpu.adapter.get_info().name);
    let (mean, edge) = selftest::box_filter(gpu.clone(), &out.join("box"))?;
    println!("box filter: (1,1)={mean:?} (0,1)={edge:?}");
    for (lite, name) in [(false, "compute"), (true, "lite")] {
        let image = selftest::rays(gpu.clone(), &out.join(format!("rays-{name}")), lite)?;
        image.save(out.join(format!("rays-{name}.png")))?;
        println!("rays {name} tiles {:?}", selftest::tile_means(&image));
    }
    // CPU reference of the lite ray data textures (crysirror-raytrace).
    let vertices = crysirror_wgpu::geometry::triangles(&selftest::ray_scene()?)?;
    let triangles: Vec<crysirror_raytrace::Triangle> = vertices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|v| crysirror_raytrace::Triangle { vertices: [v[0].position, v[1].position, v[2].position], color: [v[0].color[0], v[0].color[1], v[0].color[2]] })
        .collect();
    let packed = crysirror_raytrace::build(&triangles, 256).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let dump = serde_json::json!({
        "nodes": packed.nodes().texels()[..packed.node_count() as usize * 4],
        "triangles": packed.triangles().texels()[..packed.triangle_count() as usize * 6],
    });
    std::fs::write(out.join("ray-data.json"), serde_json::to_vec(&dump)?)?;
    println!("ray data: {} nodes, {} triangles", packed.node_count(), packed.triangle_count());
    Ok(())
}

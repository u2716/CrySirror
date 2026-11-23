// SPDX-License-Identifier: MPL-2.0
//! Renderer self-test for console drivers, without game data: the shared
//! renderer's blurred subtree transition (crysirror-wgpu's
//! `subtree_effects` GPU test scene) through the platform's wgpu backend.
//! The PS Vita player runs it when `<root>/selftest-gxm` exists; on GXM it
//! exercises the box filter's fragment lowering and float render targets.
use anyhow::{Context, Result, ensure};
use crysirror_core::{service::InputFrame, state::ClientId};
use crysirror_runtime::Runtime;
use crysirror_wgpu::Renderer;
use serde_json::json;
use std::path::Path;
use std::sync::Arc;

fn node(id: u32, parent: Option<u32>, order: i32, rect: [u32; 4], color: [u8; 4], visible: bool) -> serde_json::Value {
    json!({"op":"put","layer":{"id":id,"parent":parent,"order":[0,order],"rect":rect,"paint":{"kind":"solid","color":color},"visible":visible}})
}

/// Box-blurred transition of a black | grey | white strip. Returns the
/// filtered pixels at (1,1) and (0,1); the shared renderer's contract (and
/// its desktop GPU test) is byte mean 127 and replicated edge 42.
pub fn box_filter(gpu: Arc<crysirror_wgpu::Gpu>, scratch: &Path) -> Result<([u8; 4], [u8; 4])> {
    let _ = std::fs::remove_dir_all(scratch);
    std::fs::create_dir_all(scratch.join("assets"))?;
    std::fs::write(
        scratch.join("project.json"),
        serde_json::to_vec(&json!({"schema":2,"name":"renderer self-test","content_revision":"1","assets":"assets","canvas":[8,4],"clients":[{"name":"app","adapter":"crysirror-script-v1","game":"fixture"}]}))?,
    )?;
    let mut rt = Runtime::load(&scratch.join("project.json"), &scratch.join("state")).context("self-test runtime")?;
    let mut renderer = Renderer::new(gpu, wgpu::TextureFormat::Rgba8UnormSrgb);
    let submit = |rt: &mut Runtime, layers: serde_json::Value| -> Result<()> {
        let batch: crysirror_scene::ServiceBatch = serde_json::from_value(json!({ "layers": layers }))?;
        rt.host.engine.submit(ClientId(1), &batch)?;
        Ok(())
    };
    submit(
        &mut rt,
        json!([
            node(1, None, 0, [0, 0, 8, 4], [255; 4], true),
            node(2, Some(1), 1, [0, 0, 1, 4], [0, 0, 0, 255], true),
            node(3, Some(1), 2, [1, 0, 1, 4], [128, 128, 128, 255], true),
            node(10, None, 3, [0, 0, 8, 4], [255; 4], false),
            node(11, Some(10), 4, [0, 0, 1, 4], [0, 0, 0, 255], true),
            node(12, Some(10), 5, [1, 0, 1, 4], [128, 128, 128, 255], true)
        ]),
    )?;
    renderer.set_engine_frame(rt.host.engine.frame([8, 4])?)?;
    let plain = renderer.capture(&rt.host.scene, &mut rt.assets, [8, 4], [0; 4])?;
    crate::log(&format!("selftest plain capture: (0,1)={:?} (1,1)={:?} (4,1)={:?}", plain.get_pixel(0, 1).0, plain.get_pixel(1, 1).0, plain.get_pixel(4, 1).0));
    submit(
        &mut rt,
        json!([{"op":"begin_crossfade_effect","id":1,"source":10,"duration_ms":1000,"effect":{"kind":"blur","before":[2,0],"after":[2,0],"dynamic":true}}]),
    )?;
    rt.tick(500, InputFrame::default())?;
    renderer.set_engine_frame(rt.host.engine.frame([8, 4])?)?;
    let image = renderer.capture(&rt.host.scene, &mut rt.assets, [8, 4], [0; 4])?;
    ensure!(image.width() == 8 && image.height() == 4, "self-test capture size");
    for y in 0..4 {
        let row: Vec<u8> = (0..8).map(|x| image.get_pixel(x, y).0[0]).collect();
        let alpha: Vec<u8> = (0..8).map(|x| image.get_pixel(x, y).0[3]).collect();
        crate::log(&format!("selftest blurred row {y}: red {row:?} alpha {alpha:?}"));
    }
    Ok((image.get_pixel(1, 1).0, image.get_pixel(0, 1).0))
}

/// A lit 3D scene through `RenderingPath::ComputeRaytrace` (the lite fragment
/// profile on GPUs without compute storage): a grey floor, a red cube casting
/// a shadow and a blue cube, 64x36. Returns RGBA rows.
pub fn rays(gpu: Arc<crysirror_wgpu::Gpu>, scratch: &Path, lite: bool) -> Result<image::RgbaImage> {
    let _ = std::fs::remove_dir_all(scratch);
    std::fs::create_dir_all(scratch.join("assets"))?;
    std::fs::write(
        scratch.join("project.json"),
        serde_json::to_vec(&json!({"schema":2,"name":"ray self-test","content_revision":"1","assets":"assets","canvas":[64,36],"clients":[{"name":"app","adapter":"crysirror-script-v1","game":"fixture"}]}))?,
    )?;
    let mut rt = Runtime::load(&scratch.join("project.json"), &scratch.join("state")).context("self-test runtime")?;
    let mut renderer = Renderer::new(gpu, wgpu::TextureFormat::Rgba8UnormSrgb);
    if lite {
        renderer.use_lite_rays()?;
    }
    renderer.set_rendering_path(crysirror_core::graphics::RenderingPath::ComputeRaytrace)?;
    renderer.set_scene3d(Some(&ray_scene()?))?;
    renderer.set_engine_frame(rt.host.engine.frame([64, 36])?)?;
    Ok(renderer.capture(&rt.host.scene, &mut rt.assets, [64, 36], [40, 60, 90, 255])?)
}

/// The ray self-test scene.
pub fn ray_scene() -> Result<crysirror_scene::model::Scene3d> {
    Ok(serde_json::from_value(json!({
        "camera": {"eye": [3.0, 2.5, 4.0], "target": [0.0, 0.3, 0.0], "up": [0.0, 1.0, 0.0], "fov_y_degrees": 50.0, "near": 0.1, "far": 50.0},
        "meshes": [
            {"name": "floor", "geometry": {"kind": "triangles", "vertices": [[-3.0, 0.0, -3.0], [3.0, 0.0, -3.0], [3.0, 0.0, 3.0], [-3.0, 0.0, 3.0]], "indices": [0, 2, 1, 0, 3, 2]}, "color": [180, 180, 180, 255]},
            {"name": "red", "geometry": {"kind": "cube"}, "translation": [0.0, 0.5, 0.0], "color": [220, 40, 40, 255]},
            {"name": "blue", "geometry": {"kind": "cube"}, "translation": [1.3, 0.35, 1.0], "rotation_degrees": [0.0, 30.0, 0.0], "scale": [0.7, 0.7, 0.7], "color": [40, 80, 220, 255]}
        ]
    }))?)
}

/// Mean RGB of each 16x9 tile, a compact ray self-test signature.
pub fn tile_means(image: &image::RgbaImage) -> Vec<[u8; 3]> {
    let mut out = Vec::new();
    for ty in 0..image.height() / 9 {
        for tx in 0..image.width() / 16 {
            let mut sum = [0u32; 3];
            for y in ty * 9..ty * 9 + 9 {
                for x in tx * 16..tx * 16 + 16 {
                    let p = image.get_pixel(x, y).0;
                    for c in 0..3 {
                        sum[c] += u32::from(p[c]);
                    }
                }
            }
            out.push(sum.map(|v| (v / 144) as u8));
        }
    }
    out
}

/// Run the self-test on the GXM driver and describe the result.
#[cfg(target_os = "vita")]
pub fn gxm(root: &Path) -> String {
    crysirror_wgpu_gxm::gxm::set_log(crate::log);
    crysirror_wgpu_gxm::gxm::trace(true);
    let result = crysirror_wgpu_gxm::create().map_err(|e| anyhow::anyhow!("GXM renderer: {e}")).and_then(|gpu| {
        let shared = Arc::new(crysirror_wgpu::Gpu { adapter: gpu.adapter.clone(), device: gpu.device.clone(), queue: gpu.queue.clone() });
        let rays = rays(shared.clone(), &root.join("selftest-rays"), false);
        match &rays {
            Ok(image) => {
                crate::log(&format!("selftest rays tiles {:?}", tile_means(image)));
                let _ = image.save(root.join("selftest-rays.png"));
            }
            Err(e) => crate::log(&format!("selftest rays: error {e:#}")),
        }
        box_filter(shared, &root.join("selftest"))
    });
    match result {
        Ok((mean, edge)) => format!(
            "selftest box filter: (1,1)={mean:?} (0,1)={edge:?} expected rgb 127 / 42: {}",
            if mean[..3] == [127; 3] && edge[0] == 42 { "pass" } else { "FAIL" }
        ),
        Err(e) => format!("selftest box filter: error {e:#}"),
    }
}

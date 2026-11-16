// SPDX-License-Identifier: MPL-2.0
//! Run the shared renderer through the deko3d backend with the null driver
//! and record every shader module/entry it creates:
//!   cargo run --release -p crysirror-wgpu-deko3d --example capture -- OUT PROJECT.json [FRAMES]...
use crysirror_core::service::InputFrame;
use crysirror_wgpu_deko3d::{driver::Null, library::Capture, Gpu};
use std::sync::Arc;

fn main() {
    let mut args = std::env::args().skip(1);
    let out = args.next().expect("usage: capture OUT PROJECT.json [FRAMES]");
    let projects: Vec<String> = args.collect();
    let gpu = Gpu::new(Arc::new(Null::default()), Arc::new(Capture::new(&out)));
    let gpu = Arc::new(crysirror_wgpu::Gpu { adapter: gpu.adapter.clone(), device: gpu.device.clone(), queue: gpu.queue.clone() });
    let mut renderer = crysirror_wgpu::Renderer::new(gpu.clone(), wgpu::TextureFormat::Rgba8UnormSrgb);
    println!("renderer created");
    for project in projects {
        let state = std::env::temp_dir().join(format!("crysirror-capture-{}", std::process::id()));
        let mut rt = crysirror_runtime::Runtime::load(std::path::Path::new(&project), &state).expect("load project");
        for frame in 0..3 {
            rt.tick(if frame == 0 { 0 } else { 16 }, InputFrame::default()).expect("tick");
        }
        renderer.set_scene3d(rt.project.scene3d.as_ref()).unwrap();
        renderer.set_pbr_scene(rt.render_pbr().unwrap()).unwrap();
        renderer.set_engine_frame(rt.host.engine.frame(rt.project.canvas).unwrap()).unwrap();
        let pixels = renderer.capture(&rt.host.scene, &mut rt.assets, rt.project.canvas, rt.project.background).expect("capture");
        println!("{project}: captured {}x{}", pixels.width(), pixels.height());
    }
}

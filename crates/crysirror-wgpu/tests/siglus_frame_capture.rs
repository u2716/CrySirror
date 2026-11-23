// SPDX-License-Identifier: MPL-2.0
//! The original SYSCOM capture path through a real presenter: the native save
//! code receives the composed frame from the presenter capture service.
use crysirror_core::service::InputFrame;
use crysirror_runtime::Runtime;
use crysirror_siglus_vm::{
    fixture::{self, SceneBuilder},
    runtime::{constants::fm, forms::codes as c},
};
use crysirror_wgpu::{Gpu, Renderer};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

fn find(root: &Path, name: &str) -> Option<PathBuf> {
    for entry in std::fs::read_dir(root).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find(&path, name) {
                return Some(found);
            }
        } else if path.file_name().is_some_and(|n| n == name) {
            return Some(path);
        }
    }
    None
}

#[test]
#[ignore = "requires a real GPU"]
fn capture_buffer_receives_the_composed_client_frame() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("assets")).unwrap();
    let mut scene = SceneBuilder::default();
    let object = [c::ELM_GLOBAL_FRONT, c::ELM_STAGE_OBJECT, c::ELM_ARRAY, 0, c::ELM_OBJECT_CREATE_RECT];
    scene.command(&object, 2, &[0, 0, 4, 3, 255, 0, 0, 255, 1, 10, 20], fm::VOID);
    scene.command(&[c::ELM_GLOBAL_TIMEWAIT], 0, &[1], fm::VOID);
    scene.command_string(&[c::ELM_GLOBAL_SYSCOM, c::ELM_SYSCOM_SAVE_CAPTURE_BUFFER_TO_FILE], 0, "capture.png", &[], fm::INT);
    scene.command(&[c::ELM_GLOBAL_TIMEWAIT], 0, &[100000], fm::VOID);
    std::fs::write(root.path().join("assets/Scene.pck"), fixture::pack_scene(&scene.finish())).unwrap();
    let project = serde_json::json!({"schema":2,"name":"capture","content_revision":"1","assets":"assets","canvas":[1280,720],
        "clients":[{"name":"native","adapter":"siglus","game":"authored","config":{"scene":"Scene.pck","string_codec":"plain"}}]});
    let path = root.path().join("project.json");
    std::fs::write(&path, serde_json::to_vec(&project).unwrap()).unwrap();
    let mut rt = Runtime::load(&path, &root.path().join("state")).unwrap();
    rt.set_canvas_image_presenter(true);
    let gpu = Arc::new(pollster::block_on(Gpu::request(&wgpu::Instance::default(), None)).unwrap());
    let mut renderer = Renderer::new(gpu, wgpu::TextureFormat::Rgba8UnormSrgb);
    let until = Instant::now() + Duration::from_secs(30);
    let file = loop {
        rt.tick(16, InputFrame::default()).unwrap();
        rt.prepare_frame_captures(|rt| {
            let canvas = rt.project.canvas;
            renderer.set_engine_frame(rt.host.engine.frame(canvas)?)?;
            Ok(renderer.capture(&rt.host.scene, &mut rt.assets, canvas, [0, 0, 0, 255])?.into_raw())
        })
        .unwrap();
        if let Some(file) = find(root.path(), "capture.png") {
            break file;
        }
        assert!(Instant::now() < until, "capture never completed: {}", rt.inspect()["sessions"][0]["native"]);
        std::thread::sleep(Duration::from_millis(1));
    };
    let image = image::open(file).unwrap().to_rgba8();
    assert_eq!(image.dimensions(), (1280, 720));
    assert_eq!(image.get_pixel(11, 21).0, [255, 0, 0, 255], "object pixels reached the capture");
    assert_eq!(image.get_pixel(0, 0).0, [0, 0, 0, 255], "uncovered pixels show the background");
}

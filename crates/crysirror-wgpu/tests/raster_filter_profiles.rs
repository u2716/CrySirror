//! Offscreen proof of reusable clamped kernels, encoded sampling and UV phases.
use crysirror_core::service::InputFrame;
use crysirror_runtime::Runtime;
use crysirror_scene::layer::{LayerSpec,LayerSystem};
use crysirror_wgpu::{Gpu,Renderer};
use std::sync::Arc;
#[test]
#[ignore="requires an actual wgpu adapter; execute explicitly"]
fn clamped_nine_tap_and_encoded_quantization_reach_actual_pixels(){
    let tmp=tempfile::tempdir().unwrap();let dir=tmp.path();std::fs::create_dir(dir.join("assets")).unwrap();
    image::RgbaImage::from_fn(3,3,|x,y|image::Rgba([if x==0&&y==0{255}else{0},0,0,255])).save(dir.join("assets/corner.png")).unwrap();
    image::RgbaImage::from_fn(2,1,|x,_|image::Rgba(if x==0{[255,0,0,255]}else{[0,0,255,255]})).save(dir.join("assets/split.png")).unwrap();
    std::fs::write(dir.join("project.json"),serde_json::to_vec(&serde_json::json!({"schema":2,"name":"raster profiles","content_revision":"1","canvas":[16,4],"assets":"assets","clients":[{"name":"game","adapter":"crysirror-script-v1","game":"profiles"}]})).unwrap()).unwrap();
    let mut runtime=Runtime::load(&dir.join("project.json"),&dir.join("state")).unwrap();runtime.tick(0,InputFrame::default()).unwrap();
    let client=runtime.client_id("game").unwrap();
    let specifications=[
        serde_json::json!({"id":1,"rect":[0,0,3,3],"opacity":32,"paint":{"kind":"image","asset":"corner.png"},"filter":{"ignore_tint_alpha":true,"encoded_sampling":true,"distortion":{"kind":"nine_tap","offset":[1./3.,1./3.],"center_weight":0.2,"neighbor_weight":0.1}}}),
        serde_json::json!({"id":2,"rect":[4,0,8,2],"paint":{"kind":"image","asset":"split.png"},"filter":{"encoded_sampling":true,"distortion":{"kind":"quantized","steps":[2,1],"phase":[0.5,0.5]}}}),
    ];
    let layers=specifications.into_iter().map(serde_json::from_value).collect::<Result<Vec<LayerSpec>,_>>().unwrap();
    let mut system=LayerSystem::default();system.install(client,&layers).unwrap();
    let gpu=Arc::new(pollster::block_on(Gpu::request(&wgpu::Instance::default(),None)).unwrap());
    let mut renderer=Renderer::new(gpu,wgpu::TextureFormat::Rgba8UnormSrgb);renderer.set_engine_frame(system.frame([16,4]).unwrap()).unwrap();
    let image=renderer.capture(&runtime.host.scene,&mut runtime.assets,[16,4],[0,0,0,255]).unwrap();
    let corner=image.get_pixel(0,0).0;assert!(corner[0].abs_diff(128)<=2,"clamped corner {corner:?}");
    let center=image.get_pixel(1,1).0;assert!(center[0].abs_diff(26)<=2,"nine-tap center {center:?}");
    let quantized=image.get_pixel(6,0).0;assert!(quantized[0].abs_diff(128)<=2&&quantized[2].abs_diff(128)<=2,"encoded half-texel {quantized:?}");
}
#[test]
#[ignore="requires an actual wgpu adapter; execute explicitly"]
fn encoded_mask_channel_signed_offset_and_byte_truncation_reach_pixels(){
    let tmp=tempfile::tempdir().unwrap();let dir=tmp.path();std::fs::create_dir(dir.join("assets")).unwrap();
    image::RgbaImage::from_pixel(1,1,image::Rgba([240,1,128,255])).save(dir.join("assets/mask.png")).unwrap();
    std::fs::write(dir.join("project.json"),serde_json::to_vec(&serde_json::json!({"schema":2,"name":"mask byte","content_revision":"1","canvas":[3,1],"assets":"assets","clients":[{"name":"game","adapter":"crysirror-script-v1","game":"profiles"}]})).unwrap()).unwrap();
    let mut runtime=Runtime::load(&dir.join("project.json"),&dir.join("state")).unwrap();runtime.tick(0,InputFrame::default()).unwrap();
    let client=runtime.client_id("game").unwrap();
    let layers=[-64.5,-255.,255.].into_iter().enumerate().map(|(id,offset)|serde_json::from_value(serde_json::json!({"id":id,"rect":[id,0,1,1],"paint":{"kind":"solid","color":[255,255,255,255]},"filter":{"mask":{"asset":"mask.png","channel":"blue","offset":offset}}})).unwrap()).collect::<Vec<LayerSpec>>();
    let mut system=LayerSystem::default();system.install(client,&layers).unwrap();
    let gpu=Arc::new(pollster::block_on(Gpu::request(&wgpu::Instance::default(),None)).unwrap());let mut renderer=Renderer::new(gpu,wgpu::TextureFormat::Rgba8UnormSrgb);
    renderer.set_engine_frame(system.frame([3,1]).unwrap()).unwrap();
    let image=renderer.capture(&runtime.host.scene,&mut runtime.assets,[3,1],[0,0,0,0]).unwrap();
    assert_eq!(image.get_pixel(0,0).0[3],63);
    assert_eq!(image.get_pixel(1,0).0[3],0);
    assert_eq!(image.get_pixel(2,0).0[3],255);
}

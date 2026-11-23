//! Original object lifetime opcodes -> actual scoped font workers and layers.
//! The tiny coverage face is authored data, not a redistributed font binary.
use crysirror_core::{service::*, state::ClientId};
use crysirror_runtime::Runtime;
use crysirror_scene::layer::Paint;
use crysirror_siglus_vm::{
    fixture::SceneBuilder,
    runtime::{
        constants::{cd, fm},
        forms::codes as c,
    },
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

fn coverage_face() -> Arc<[u8]> {
    let mut b = b"TVP pre-rendered font\x1a".to_vec();
    b.extend([1, 2]);
    b.extend(1u32.to_le_bytes());
    b.extend(36u32.to_le_bytes());
    b.extend(38u32.to_le_bytes());
    b.extend((b'A' as u16).to_le_bytes());
    b.extend(58u32.to_le_bytes());
    for v in [2i16, 2, -1, 2, 3, 0, 4, 0] {
        b.extend(v.to_le_bytes());
    }
    b.extend([32, 0x43]);
    b.into()
}
fn make(scene: SceneBuilder) -> (tempfile::TempDir, Runtime) {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("assets")).unwrap();
    std::fs::write(
        root.path().join("assets/Scene.pck"),
        crysirror_siglus_vm::fixture::pack_scene(&scene.finish()),
    )
    .unwrap();
    let project = serde_json::json!({"schema":2,"name":"native strings","content_revision":"1","assets":"assets","canvas":[64,64],"clients":[{"name":"native","adapter":"siglus","game":"authored","config":{"scene":"Scene.pck","string_codec":"plain"}},{"name":"peer","adapter":"crysirror-script-v1","game":"peer","program":[]}]});
    let path = root.path().join("project.json");
    std::fs::write(&path, serde_json::to_vec(&project).unwrap()).unwrap();
    let runtime = siglus_reference::load(&path, &root.path().join("state")).unwrap();
    (root, runtime)
}
fn load(runtime: &mut Runtime) -> FontInfo {
    let fonts = runtime.host.font_library(ClientId(1)).unwrap();
    let job = fonts
        .begin(FontLoadSpec {
            name: "default".into(),
            source: FontLoadSource::PrecomputedBytes {
                bytes: coverage_face(),
                pixel_size: 12,
            },
            face_index: 0,
        })
        .unwrap();
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        match fonts.status(job).unwrap() {
            FontLoadState::Ready(info) => {
                fonts.release_job(job).unwrap();
                return info;
            }
            FontLoadState::Failed(e) => panic!("{e}"),
            FontLoadState::Pending => {
                assert!(Instant::now() < end);
                std::thread::yield_now();
            }
        }
    }
}
fn native(runtime:&Runtime)->serde_json::Value {runtime.inspect()["sessions"][0]["native"].clone()}
fn settle_at(runtime:&mut Runtime,marker:i32) {
 let until=Instant::now()+Duration::from_secs(10);
 loop {
  runtime.tick(0,InputFrame::default()).unwrap();
  if native(runtime)["int_stack"].as_array().and_then(|a|a.last())==Some(&serde_json::json!(marker)) {return;}
  assert!(Instant::now()<until,"{}",runtime.inspect());std::thread::yield_now();
 }
}
fn glyph_count(runtime:&Runtime)->usize {runtime.host.engine.frame([64,64]).unwrap().iter().filter(|p|matches!(p.paint,Paint::GlyphRun{..})).count()}
#[test]
fn native_param_free_and_rebuild_retain_then_retire_actual_shared_glyph_resources() {
 let mut scene=SceneBuilder::default();
 for op in [c::ELM_SCRIPT_SET_FONT_BOLD,c::ELM_SCRIPT_SET_FONT_SHADOW] {scene.command(&[c::ELM_GLOBAL_SCRIPT,op],0,&[0],fm::VOID);}
 let root=[c::ELM_GLOBAL_FRONT,c::ELM_STAGE_OBJECT,c::ELM_ARRAY,52];
 let chain=|tail:&[i32]|[root.as_slice(),tail].concat();
 scene.command_string(&chain(&[c::ELM_OBJECT_CREATE_STRING]),1,"A",&[1],fm::VOID);
 scene.command_string(&chain(&[c::ELM_OBJECT_CHILD,c::ELM_ARRAY,3,c::ELM_OBJECT_CREATE_STRING]),2,"A",&[1,4,5],fm::VOID);
 scene.command(&chain(&[c::ELM_OBJECT_INIT_PARAM]),0,&[],fm::VOID);
 scene.command(&chain(&[c::ELM_OBJECT_DISP]),1,&[1],fm::VOID);
 scene.command(&chain(&[c::ELM_OBJECT_GET_TYPE]),0,&[],fm::INT);
 scene.command(&chain(&[c::ELM_OBJECT_EXIST_TYPE]),0,&[],fm::INT);
 scene.command(&chain(&[c::ELM_OBJECT_CHILD,c::ELM_OBJECTLIST_GET_SIZE]),0,&[],fm::INT);
 scene.int(11);scene.command(&[c::ELM_GLOBAL_TIMEWAIT],0,&[10],fm::VOID);
 scene.command(&chain(&[c::ELM_OBJECT_FREE]),0,&[],fm::VOID);
 scene.command(&chain(&[c::ELM_OBJECT_GET_TYPE]),0,&[],fm::INT);
 scene.command(&chain(&[c::ELM_OBJECT_EXIST_TYPE]),0,&[],fm::INT);
 scene.int(22);scene.command(&[c::ELM_GLOBAL_TIMEWAIT],0,&[10],fm::VOID);
 scene.command(&chain(&[c::ELM_OBJECT_CREATE_RECT]),2,&[0,0,8,6,0,255,0,255,1,9,8],fm::VOID);
 scene.command(&chain(&[c::ELM_OBJECT_CHILD,c::ELM_OBJECTLIST_GET_SIZE]),0,&[],fm::INT);
 scene.int(33);scene.code.push(cd::EOF);
 let (_dir,mut runtime)=make(scene);let font=load(&mut runtime);
 let peer:crysirror_scene::layer::LayerSpec=serde_json::from_value(serde_json::json!({"id":700,"rect":[50,50,5,5],"paint":{"kind":"solid","color":[0,0,255,255]}})).unwrap();
 runtime.host.engine.layers.install(ClientId(2),&[peer.clone()]).unwrap();
 settle_at(&mut runtime,11);assert_eq!(glyph_count(&runtime),2);
 assert_eq!(native(&runtime)["int_stack"],serde_json::json!([3,1,4,11]));
 // Both native resources hold actual completed engine font receipts. Removing
 // the owner's load alias must not invalidate the already captured glyphs.
 runtime.host.font_library(ClientId(1)).unwrap().release_font(font.handle).unwrap();
 runtime.tick(10,InputFrame::default()).unwrap();settle_at(&mut runtime,22);
 assert_eq!(glyph_count(&runtime),1);assert_eq!(native(&runtime)["int_stack"],serde_json::json!([3,1,4,11,0,0,22]));
 let frame=runtime.host.engine.frame([64,64]).unwrap();
 let (font,size,glyph)=frame.iter().find_map(|p|match &p.paint {Paint::GlyphRun{font,size,glyphs,..}=>Some((font,*size,&glyphs[0])),_=>None}).unwrap();
 let text=runtime.host.text.for_client(ClientId(1));let glyph_id=text.glyph_id(font,glyph.character).unwrap();
 let cell=text.rasterize_glyph_cell(font,crysirror_text::glyph::GlyphRaster{glyph_id,size,baseline:glyph.baseline}).unwrap();
 assert_eq!((cell.width,cell.height),(2,2));assert_eq!(cell.coverage.iter().filter(|v|**v>0.).count(),4);
 runtime.tick(10,InputFrame::default()).unwrap();settle_at(&mut runtime,33);
 assert_eq!(glyph_count(&runtime),0);assert_eq!(native(&runtime)["halted"],true);
 assert_eq!(native(&runtime)["int_stack"],serde_json::json!([3,1,4,11,0,0,22,0,33]));
 assert_eq!(serde_json::to_value(runtime.host.engine.layers.get(ClientId(2),700).unwrap()).unwrap(),serde_json::to_value(peer).unwrap());
 assert_eq!(native(&runtime)["unknown_opcodes"],serde_json::json!({}));
}

#[path = "support/siglus_reference.rs"]
mod siglus_reference;

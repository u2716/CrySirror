//! Authored original SET_CLIP/properties/events, not data-VM commands.
use super::*;
use crate::runtime::{
    constants::{cd, fm},
    forms::codes as c,
};
fn chain(index: i32, tail: &[i32]) -> Vec<i32> {
    [
        &[
            c::ELM_GLOBAL_FRONT,
            c::ELM_STAGE_OBJECT,
            c::ELM_ARRAY,
            index,
        ],
        tail,
    ]
    .concat()
}
fn install(host: &mut Host, scene: crate::fixture::SceneBuilder) {
    host.files.insert(
        "Asset/Scene.pck".into(),
        crate::fixture::pack_scene(&scene.finish()),
    );
}
#[test]
fn native_target_clip_is_absolute_and_shared_events_survive_framework_restore() {
    let (mut vm, mut host) = machine(false);
    let mut scene = crate::fixture::SceneBuilder::default();
    scene.command(
        &chain(0, &[c::ELM_OBJECT_CREATE_RECT]),
        2,
        &[0, 0, 200, 100, 255, 0, 0, 255, 1, 30, 40],
        fm::VOID,
    );
    scene.command(
        &chain(0, &[c::ELM_OBJECT_SET_CLIP]),
        0,
        &[1, -10, 10, 110, 90],
        fm::VOID,
    );
    scene.command(
        &chain(0, &[c::ELM_OBJECT_CLIP_RIGHT_EVE, c::int_event_op::SET]),
        0,
        &[210, 100, 0, 0],
        fm::VOID,
    );
    scene.command(&[c::ELM_GLOBAL_TIMEWAIT], 0, &[200], fm::VOID);
    scene.command(&chain(0, &[c::ELM_OBJECT_CLIP_USE]), 1, &[0], fm::VOID);
    scene.code.push(cd::EOF);
    install(&mut host, scene);
    vm.step(&mut host, InputFrame::default(), 5000).unwrap();
    let id = 0x80000000;
    assert_eq!(
        host.engine
            .layers
            .get(ClientId(1), id)
            .unwrap()
            .viewport_clip,
        Some([0., 10., 110., 80.])
    );
    host.now = 50;
    vm.step(&mut host, InputFrame::default(), 5000).unwrap();
    assert_eq!(
        host.engine
            .layers
            .get(ClientId(1), id)
            .unwrap()
            .viewport_clip,
        Some([0., 10., 160., 80.])
    );
    assert_eq!(
        host.engine.frame([320, 180]).unwrap()[0].clip,
        Some([0., 10., 160., 80.])
    );
    let snapshot = vm.snapshot().unwrap();
    vm.restore(&snapshot).unwrap();
    host.now = 200;
    assert_eq!(
        vm.step(&mut host, InputFrame::default(), 5000).unwrap(),
        VmYield::Halted
    );
    assert_eq!(
        host.engine
            .layers
            .get(ClientId(1), id)
            .unwrap()
            .viewport_clip,
        None
    );
}
#[test]
fn native_parent_clip_affects_its_payload_not_child_payloads_and_empty_is_not_an_error() {
    let (mut vm, mut host) = machine(false);
    let mut scene = crate::fixture::SceneBuilder::default();
    scene.command(
        &chain(52, &[c::ELM_OBJECT_CREATE_RECT]),
        2,
        &[0, 0, 10, 10, 255, 0, 0, 255, 1, 100, 40],
        fm::VOID,
    );
    scene.command(
        &chain(52, &[c::ELM_OBJECT_SET_CLIP]),
        0,
        &[1, 50, 0, 20, 30],
        fm::VOID,
    );
    scene.command(
        &chain(52, &[c::ELM_OBJECT_CHILD, c::ELM_OBJECTLIST_RESIZE]),
        0,
        &[1],
        fm::VOID,
    );
    scene.command(
        &chain(
            52,
            &[
                c::ELM_OBJECT_CHILD,
                c::ELM_ARRAY,
                0,
                c::ELM_OBJECT_CREATE_RECT,
            ],
        ),
        2,
        &[0, 0, 3, 3, 0, 255, 0, 255, 1, 0, 0],
        fm::VOID,
    );
    scene.code.push(cd::EOF);
    install(&mut host, scene);
    vm.step(&mut host, InputFrame::default(), 5000).unwrap();
    let frame = host.engine.frame([320, 180]).unwrap();
    assert!(!frame.iter().any(
        |p| matches!(&p.paint,crysirror_scene::layer::Paint::Solid{color} if *color==[255,0,0,255])
    ));
    let child=frame.iter().find(|p|matches!(&p.paint,crysirror_scene::layer::Paint::Solid{color} if *color==[0,255,0,255])).unwrap();
    assert_eq!(child.clip, Some([0., 0., 320., 180.]));
}
#[test]
fn native_parent_source_crop_affects_its_payload_not_child_payloads_and_empty_is_not_an_error() {
    let (mut vm, mut host) = machine(false);
    let mut scene = crate::fixture::SceneBuilder::default();
    scene.command(
        &chain(52, &[c::ELM_OBJECT_CREATE_RECT]),
        2,
        &[0, 0, 10, 10, 255, 0, 0, 255, 1, 100, 40],
        fm::VOID,
    );
    scene.command(
        &chain(52, &[c::ELM_OBJECT_SET_SRC_CLIP]),
        0,
        &[1, 50, 0, 20, 30],
        fm::VOID,
    );
    scene.command(
        &chain(52, &[c::ELM_OBJECT_CHILD, c::ELM_OBJECTLIST_RESIZE]),
        0,
        &[1],
        fm::VOID,
    );
    scene.command(
        &chain(
            52,
            &[
                c::ELM_OBJECT_CHILD,
                c::ELM_ARRAY,
                0,
                c::ELM_OBJECT_CREATE_RECT,
            ],
        ),
        2,
        &[0, 0, 3, 3, 0, 255, 0, 255, 1, 0, 0],
        fm::VOID,
    );
    scene.code.push(cd::EOF);
    install(&mut host, scene);
    vm.step(&mut host, InputFrame::default(), 5000).unwrap();
    let frame = host.engine.frame([320, 180]).unwrap();
    assert!(!frame.iter().any(
        |p| matches!(&p.paint,crysirror_scene::layer::Paint::Solid{color} if *color==[255,0,0,255])
    ));
    let child=frame.iter().find(|p|matches!(&p.paint,crysirror_scene::layer::Paint::Solid{color} if *color==[0,255,0,255])).unwrap();
    assert_eq!(child.clip, Some([0., 0., 320., 180.]));
}
#[test]
fn native_string_each_glyph_uses_the_same_target_clip() {
    let (mut vm, mut host) = machine(false);
    let mut scene = crate::fixture::SceneBuilder::default();
    scene.command_string(
        &chain(52, &[c::ELM_OBJECT_CREATE_STRING]),
        2,
        "AB",
        &[1, 100, 40],
        fm::VOID,
    );
    scene.command(
        &chain(52, &[c::ELM_OBJECT_SET_CLIP]),
        0,
        &[1, 105, 40, 115, 55],
        fm::VOID,
    );
    scene.code.push(cd::EOF);
    install(&mut host, scene);
    vm.step(&mut host, InputFrame::default(), 5000).unwrap();
    let frame = host.engine.frame([320, 180]).unwrap();
    let glyphs = frame
        .iter()
        .filter(|p| matches!(p.paint, crysirror_scene::layer::Paint::GlyphRun { .. }))
        .collect::<Vec<_>>();
    assert!(!glyphs.is_empty());
    assert!(glyphs.iter().all(|p| p.clip == Some([105., 40., 10., 15.])));
}

#[test]
fn native_source_crop_uses_signed_center_relative_edges_and_all_event_receipts() {
 let (mut vm,mut host)=machine(false);let mut scene=crate::fixture::SceneBuilder::default();
 scene.command(&chain(0,&[c::ELM_OBJECT_CREATE_RECT]),2,&[0,0,200,100,255,0,0,255,1,30,40],fm::VOID);
 scene.command(&chain(0,&[c::ELM_OBJECT_SET_CENTER]),0,&[20,10],fm::VOID);
 scene.command(&chain(0,&[c::ELM_OBJECT_SET_SRC_CLIP]),0,&[1,-10,-5,90,45],fm::VOID);
 for (event,to) in [(c::ELM_OBJECT_SRC_CLIP_LEFT_EVE,10),(c::ELM_OBJECT_SRC_CLIP_TOP_EVE,5),(c::ELM_OBJECT_SRC_CLIP_RIGHT_EVE,110),(c::ELM_OBJECT_SRC_CLIP_BOTTOM_EVE,65)] {
 scene.command(&chain(0,&[event,c::int_event_op::SET]),0,&[to,100,0,0],fm::VOID);
 }
 scene.command(&[c::ELM_GLOBAL_TIMEWAIT],0,&[200],fm::VOID);
 scene.command(&chain(0,&[c::ELM_OBJECT_SRC_CLIP_USE]),1,&[0],fm::VOID);scene.code.push(cd::EOF);install(&mut host,scene);
 vm.step(&mut host,InputFrame::default(),5000).unwrap();
 assert_eq!(host.engine.layers.get(ClientId(1),0x80000000).unwrap().source_crop,Some([10.,5.,100.,50.]));
 host.now=50;vm.step(&mut host,InputFrame::default(),5000).unwrap();
 assert_eq!(host.engine.layers.get(ClientId(1),0x80000000).unwrap().source_crop,Some([20.,10.,100.,55.]));
 let f=host.engine.frame([320,180]).unwrap();assert_eq!(f[0].extent,[100.,55.]);assert_eq!(f[0].uv,[0.1,0.1,0.6,0.65]);
 let save=vm.snapshot().unwrap();vm.restore(&save).unwrap();host.now=200;
 assert_eq!(vm.step(&mut host,InputFrame::default(),5000).unwrap(),VmYield::Halted);
 assert_eq!(host.engine.layers.get(ClientId(1),0x80000000).unwrap().source_crop,None);
}

#[test]
fn native_string_source_crop_is_per_original_glyph_texture_not_one_text_rect() {
 let (mut vm,mut host)=machine(false);let mut scene=crate::fixture::SceneBuilder::default();
 scene.command_string(&chain(52,&[c::ELM_OBJECT_CREATE_STRING]),2,"AB",&[1,100,40],fm::VOID);
 scene.command(&chain(52,&[c::ELM_OBJECT_SET_SRC_CLIP]),0,&[1,0,0,5,6],fm::VOID);scene.code.push(cd::EOF);install(&mut host,scene);
 vm.step(&mut host,InputFrame::default(),5000).unwrap();
 let frame=host.engine.frame([320,180]).unwrap();
 let crops=frame.iter().filter_map(|p|match &p.paint {crysirror_scene::layer::Paint::GlyphRun{glyphs,..}=>Some((glyphs[0].character,glyphs[0].crop.unwrap())),_=>None}).collect::<Vec<_>>();
 assert!(!crops.is_empty());assert!(crops.iter().all(|(_,r)|r[2..]==[5.,6.]));
 let a=crops.iter().find(|(c,_)|*c=='A').unwrap().1;let b=crops.iter().find(|(c,_)|*c=='B').unwrap().1;
 assert!(b[0]>a[0]);assert_eq!(b[1],a[1]);
}

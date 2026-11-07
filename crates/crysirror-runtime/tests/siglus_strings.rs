//! Original STRING bytecode -> real scoped font worker -> retained engine glyphs.
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
fn complete(runtime: &mut Runtime) {
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        runtime.tick(0, InputFrame::default()).unwrap();
        if runtime.inspect()["sessions"][0]["native"]["halted"] == true {
            return;
        }
        assert!(Instant::now() < end, "{}", runtime.inspect());
        std::thread::yield_now();
    }
}
#[test]
fn original_string_uses_actual_engine_font_worker_and_engine_owned_coverage() {
    let mut scene = SceneBuilder::default();
    scene.command(
        &[c::ELM_GLOBAL_SCRIPT, c::ELM_SCRIPT_SET_FONT_BOLD],
        0,
        &[0],
        fm::VOID,
    );
    scene.command(
        &[c::ELM_GLOBAL_SCRIPT, c::ELM_SCRIPT_SET_FONT_SHADOW],
        0,
        &[0],
        fm::VOID,
    );
    let root = [c::ELM_GLOBAL_FRONT, c::ELM_STAGE_OBJECT, c::ELM_ARRAY, 52];
    scene.command_string(
        &[root.as_slice(), &[c::ELM_OBJECT_CREATE_STRING]].concat(),
        2,
        "AA",
        &[1, 8, 8],
        fm::VOID,
    );
    scene.code.push(cd::EOF);
    let (_dir, mut runtime) = make(scene);
    let info = load(&mut runtime);
    assert!(
        runtime
            .host
            .font_library(ClientId(2))
            .unwrap()
            .resolve_alias(&info.alias)
            .unwrap()
            .is_none()
    );
    complete(&mut runtime);
    let frame = runtime.host.engine.frame([64, 64]).unwrap();
    let glyphs = frame
        .iter()
        .filter_map(|node| match &node.paint {
            Paint::GlyphRun {
                font, size, glyphs, ..
            } => Some((font, size, glyphs)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(glyphs.len(), 2);
    for (index, (font, size, glyphs)) in glyphs.iter().enumerate() {
        assert_eq!(**font, info.alias);
        assert_eq!(**size, 12.);
        assert_eq!(glyphs.len(), 1);
        let glyph = &glyphs[0];
        assert_eq!(glyph.character, 'A');
        assert_eq!(glyph.baseline[0], index as f32 * 6.);
        let id = runtime
            .host
            .text
            .for_client(ClientId(1))
            .glyph_id(font, 'A')
            .unwrap();
        let cell = runtime
            .host
            .text
            .for_client(ClientId(1))
            .rasterize_glyph_cell(
                font,
                crysirror_text::glyph::GlyphRaster {
                    glyph_id: id,
                    size: **size,
                    baseline: glyph.baseline,
                },
            )
            .unwrap();
        assert_eq!((cell.width, cell.height), (2, 2));
        assert_eq!(cell.coverage.iter().filter(|v| **v > 0.).count(), 4);
    }
    assert_eq!(
        runtime.inspect()["sessions"][0]["native"]["unknown_opcodes"],
        serde_json::json!({})
    );
}

#[test]
fn native_missing_glyph_fails_without_submitting_a_fake_string_frame() {
    let mut scene = SceneBuilder::default();
    scene.command_string(
        &[
            c::ELM_GLOBAL_FRONT,
            c::ELM_STAGE_OBJECT,
            c::ELM_ARRAY,
            52,
            c::ELM_OBJECT_CREATE_STRING,
        ],
        1,
        "B",
        &[1],
        fm::VOID,
    );
    scene.code.push(cd::EOF);
    let (_dir, mut runtime) = make(scene);
    load(&mut runtime);
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        match runtime.tick(0, InputFrame::default()) {
            Err(error) => {
                assert!(
                    format!("{error:#}").contains("precomputed font missing character"),
                    "{error:#}"
                );
                break;
            }
            Ok(()) => {
                assert!(Instant::now() < end);
                std::thread::yield_now();
            }
        }
    }
    assert!(runtime.host.engine.frame([64, 64]).unwrap().is_empty());
}

#[test]
fn native_vm_snapshot_rehydrates_a_string_bank_through_bounded_actual_workers() {
    use crysirror_core::vm::{VmAdapter, VmYield};
    let mut scene = SceneBuilder::default();
    scene.command(
        &[c::ELM_GLOBAL_SCRIPT, c::ELM_SCRIPT_SET_FONT_BOLD],
        0,
        &[0],
        fm::VOID,
    );
    scene.command(
        &[c::ELM_GLOBAL_SCRIPT, c::ELM_SCRIPT_SET_FONT_SHADOW],
        0,
        &[0],
        fm::VOID,
    );
    for index in 52..116 {
        scene.command_string(
            &[
                c::ELM_GLOBAL_FRONT,
                c::ELM_STAGE_OBJECT,
                c::ELM_ARRAY,
                index,
                c::ELM_OBJECT_CREATE_STRING,
            ],
            1,
            "A",
            &[1],
            fm::VOID,
        );
    }
    scene.code.push(cd::EOF);
    let (_dir, mut runtime) = make(scene);
    load(&mut runtime);
    // Exercise the native snapshot contract with the same live SharedHost.
    // Whole Runtime snapshots of dynamic font catalogs are explicitly gated;
    // this fixture does not weaken or claim that separate reconstruction path.
    let config =
        serde_json::from_value(serde_json::json!({"scene":"Scene.pck","string_codec":"plain"}))
            .unwrap();
    let mut vm = crysirror_siglus_vm::adapter::SiglusVm::new(ClientId(1), config).unwrap();
    let settle = |vm: &mut crysirror_siglus_vm::adapter::SiglusVm, runtime: &mut Runtime| {
        let end = Instant::now() + Duration::from_secs(10);
        loop {
            if vm
                .step(&mut runtime.host, InputFrame::default(), 5000)
                .unwrap()
                == VmYield::Halted
            {
                break;
            }
            assert!(Instant::now() < end, "{}", vm.diagnostics());
            std::thread::yield_now();
        }
    };
    settle(&mut vm, &mut runtime);
    let frame = runtime.host.engine.frame([64, 64]).unwrap();
    assert_eq!(
        frame
            .iter()
            .filter(|node| matches!(node.paint, Paint::GlyphRun { .. }))
            .count(),
        64
    );
    let fingerprint = |nodes: Vec<crysirror_scene::Primitive>| {
        serde_json::to_value(
            nodes
                .into_iter()
                .map(|node| (node.owner, node.id, node.quad, node.paint))
                .collect::<Vec<_>>(),
        )
        .unwrap()
    };
    let before = fingerprint(frame);
    let snapshot = vm.snapshot().unwrap();
    vm.restore(&snapshot).unwrap();
    settle(&mut vm, &mut runtime);
    assert_eq!(
        fingerprint(runtime.host.engine.frame([64, 64]).unwrap()),
        before
    );
}

#[path = "support/siglus_reference.rs"]
mod siglus_reference;

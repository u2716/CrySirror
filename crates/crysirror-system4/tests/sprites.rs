//! Shared layer acceptance for native properties, without an original VM.

use crysirror_core::{service::HostServices, state::ClientId};
use crysirror_runtime::Runtime;
use crysirror_system4::sprites::Sprites;
fn runtime() -> (tempfile::TempDir, Runtime) {
    let t = tempfile::tempdir().unwrap();
    std::fs::create_dir(t.path().join("assets")).unwrap();
    let path = t.path().join("project.json");
    std::fs::write(&path,br#"{"schema":2,"name":"sact layers","content_revision":"1","canvas":[800,600],"assets":"assets","clients":[{"name":"data","adapter":"crysirror-script-v1","game":"test"}]}"#).unwrap();
    let mut r = Runtime::load(&path, &t.path().join("state")).unwrap();
    r.host.submit_engine(ClientId(1), 1, br#"{"layers":[{"op":"put","layer":{"id":1090453504,"rect":[0,0,800,600],"paint":{"kind":"solid","color":[0,0,0,0]},"visible":false}}]}"#).unwrap();
    (t, r)
}
#[test]
fn native_sprite_create_position_visibility_order_and_opacity_are_engine_layers() {
    let (_t, mut r) = runtime();
    let c = ClientId(1);
    let mut s = Sprites::new(c, [800, 600]).unwrap();
    // WindowSurface is distinct from the native ordinary sprite table.
    assert!(s.get(-1).is_none());
    assert_eq!(
        s.scalar(c, b"SP_SetPos", &[7, 31, 42], &mut r.host)
            .unwrap(),
        Some(1)
    );
    // Metadata-only native sprites still own a transparent retained container
    // for text children; dimensions remain zero in native state.
    assert_eq!(s.get(7).unwrap().size, [0, 0]);
    assert_eq!(
        r.host.engine.layers.get(c, 0x4100_0007).unwrap().rect,
        [31., 42., 1., 1.]
    );
    assert_eq!(
        s.scalar(c, b"SP_Create", &[7, 20, 30, 200, 100, 50, -1], &mut r.host)
            .unwrap(),
        Some(1)
    );
    let layer = r.host.engine.layers.get(c, 0x4100_0007).unwrap();
    assert_eq!(layer.rect, [31., 42., 20., 30.]);
    // SactEngine 4B20 initializes byte40 (show) to1.
    assert!(layer.visible);
    s.scalar(c, b"SP_SetZ", &[7, -27], &mut r.host).unwrap();
    s.scalar(c, b"SP_SetShow", &[7, 1], &mut r.host).unwrap();
    assert_eq!(
        s.scalar(c, b"SP_SetBlendRate", &[7, 999], &mut r.host)
            .unwrap(),
        Some(0)
    );
    let layer = r.host.engine.layers.get(c, 0x4100_0007).unwrap();
    assert_eq!(layer.order, [-27, 7]);
    assert!(layer.visible);
    assert_eq!(layer.opacity, 255);
    assert_eq!(s.count(), 1);
    assert_eq!(s.max_z(), 0);
    assert_eq!(s.unused(7).unwrap(), 8);
    assert_eq!(
        s.scalar(c, b"SP_GetWidth", &[7], &mut r.host).unwrap(),
        Some(20)
    );
    assert!(s
        .scalar(ClientId(2), b"SP_SetX", &[7, 20], &mut r.host)
        .is_err());
    assert!(r.host.engine.layers.get(ClientId(2), 0x4100_0007).is_none());
    assert_eq!(s.delete(c, 7, &mut r.host).unwrap(), 1);
    assert!(r.host.engine.layers.get(c, 0x4100_0007).is_none());
    assert_eq!(s.delete(c, 7, &mut r.host).unwrap(), 0);
}
#[test]
fn unknown_blending_and_failed_engine_publication_retain_native_state() {
    let (_t, mut r) = runtime();
    let c = ClientId(1);
    let mut s = Sprites::new(c, [800, 600]).unwrap();
    s.create(c, 1, [10, 10], [0, 0, 0, 255], &mut r.host)
        .unwrap();
    assert!(s
        .scalar(c, b"SP_SetDrawMethod", &[1, 3], &mut r.host)
        .is_err());
    let old = serde_json::to_value(&r.host.engine).unwrap();
    assert!(s
        .scalar(c, b"SP_SetPos", &[1, i32::MAX, 0], &mut r.host)
        .is_err());
    assert_eq!(s.get(1).unwrap().position, [0, 0]);
    assert_eq!(serde_json::to_value(&r.host.engine).unwrap(), old);
    assert_eq!(
        s.scalar(c, b"NotARealCall", &[], &mut r.host).unwrap(),
        None
    );
}

#[test]
fn native_draw_methods_retain_policy_and_project_rgb_only_tables() {
    let (_t, mut r) = runtime();
    let c = ClientId(1);
    let mut sprites = Sprites::new(c, [800, 600]).unwrap();
    sprites
        .create(c, 7, [4, 3], [173, 99, 42, 0], &mut r.host)
        .unwrap();
    sprites
        .scalar(c, b"SP_SetBlendRate", &[7, 0], &mut r.host)
        .unwrap();
    assert_eq!(
        sprites
            .scalar(c, b"SP_SetDrawMethod", &[7, 1], &mut r.host)
            .unwrap(),
        Some(1)
    );
    assert_eq!(
        sprites
            .scalar(c, b"SP_GetDrawMethod", &[7], &mut r.host)
            .unwrap(),
        Some(1)
    );
    assert_eq!(
        sprites
            .scalar(c, b"SP_GetBlendRate", &[7], &mut r.host)
            .unwrap(),
        Some(0)
    );
    let layer = r.host.engine.layers.get(c, 0x4100_0007).unwrap();
    assert_eq!(layer.opacity, 0);
    assert_eq!(
        serde_json::to_value(layer.blend).unwrap(),
        serde_json::json!({"stored_byte":{"mode":"opaque_screen255","force_source_opaque":false,"force_output_opaque":false}})
    );
    sprites
        .scalar(c, b"SP_SetBrightness", &[7, 128], &mut r.host)
        .unwrap();
    assert_eq!(r.host.engine.layers.get(c, 0x4100_0007).unwrap().opacity, 0);
    // Brightness does not alter the screen table. Blend rate is its weight.
    sprites
        .scalar(c, b"SP_SetBlendRate", &[7, 128], &mut r.host)
        .unwrap();
    assert_eq!(
        r.host.engine.layers.get(c, 0x4100_0007).unwrap().opacity,
        128
    );
    sprites
        .scalar(c, b"SP_SetDrawMethod", &[7, 2], &mut r.host)
        .unwrap();
    // The original multiply blend-rate<255 branch draws no base pixels.
    assert_eq!(
        serde_json::to_value(&r.host.engine.layers.get(c, 0x4100_0007).unwrap().paint).unwrap(),
        serde_json::json!({"kind":"solid","color":[0,0,0,0]})
    );
    sprites
        .scalar(c, b"SP_SetBlendRate", &[7, 255], &mut r.host)
        .unwrap();
    assert_eq!(sprites.get(7).unwrap().draw_method, 2);
    assert_eq!(sprites.get(7).unwrap().brightness, 128);
    assert_eq!(
        serde_json::to_value(r.host.engine.layers.get(c, 0x4100_0007).unwrap().blend).unwrap()
            ["stored_byte"]["mode"],
        "opaque_multiply255"
    );
    sprites
        .scalar(c, b"SP_SetDrawMethod", &[7, 0], &mut r.host)
        .unwrap();
    assert_eq!(
        r.host.engine.layers.get(c, 0x4100_0007).unwrap().opacity,
        255
    );
    assert_eq!(
        sprites
            .scalar(c, b"SP_SetDrawMethod", &[0, 1], &mut r.host)
            .unwrap(),
        Some(0)
    );
    assert!(sprites.get(0).is_none());
}

#[test]
fn original_rate_guards_reject_without_publication_and_valid_brightness_lazily_declares() {
    let (_dir, mut r) = runtime();
    let owner = ClientId(1);
    let mut sprites = Sprites::new(owner, [800, 600]).unwrap();
    for name in [b"SP_SetBlendRate".as_slice(), b"SP_SetBrightness"] {
        for (number, value) in [(0, 128), (1, -1), (1, 256), (1, 999)] {
            assert_eq!(
                sprites
                    .scalar(owner, name, &[number, value], &mut r.host)
                    .unwrap(),
                Some(0)
            );
            assert_eq!(sprites.count(), 0);
            assert!(r.host.engine.layers.get(owner, 0x4100_0001).is_none());
        }
    }
    assert_eq!(
        sprites
            .scalar(owner, b"SP_SetBrightness", &[1, 128], &mut r.host)
            .unwrap(),
        Some(1)
    );
    assert_eq!(sprites.count(), 1);
    assert_eq!(sprites.get(1).unwrap().size, [0, 0]);
    assert_eq!(sprites.get(1).unwrap().brightness, 128);
    assert_eq!(
        sprites
            .scalar(owner, b"SP_GetBlendRate", &[1], &mut r.host)
            .unwrap(),
        Some(255)
    );
    let before = serde_json::to_value(&r.host.engine).unwrap();
    assert_eq!(
        sprites
            .scalar(owner, b"SP_SetBrightness", &[1, 256], &mut r.host)
            .unwrap(),
        Some(0)
    );
    assert_eq!(serde_json::to_value(&r.host.engine).unwrap(), before);
}

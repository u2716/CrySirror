//! Shared layer acceptance for native properties, without an original VM.

use crysirror_core::state::ClientId;
use crysirror_runtime::Runtime;
use crysirror_system4::sprites::Sprites;
fn runtime() -> (tempfile::TempDir, Runtime) {
    let t = tempfile::tempdir().unwrap();
    std::fs::create_dir(t.path().join("assets")).unwrap();
    let path = t.path().join("project.json");
    std::fs::write(&path,br#"{"schema":2,"name":"sact layers","content_revision":"1","canvas":[800,600],"assets":"assets","clients":[{"name":"data","adapter":"crysirror-script-v1","game":"test"}]}"#).unwrap();
    let r = Runtime::load(&path, &t.path().join("state")).unwrap();
    (t, r)
}
#[test]
fn native_sprite_create_position_visibility_order_and_opacity_are_engine_layers() {
    let (_t, mut r) = runtime();
    let c = ClientId(1);
    let mut s = Sprites::new(c, [800, 600]).unwrap();
    assert!(s.get(-1).unwrap().alpha);
    assert_eq!(
        s.scalar(c, b"SP_SetPos", &[7, 31, 42], &mut r.host)
            .unwrap(),
        Some(1)
    );
    assert!(r.host.engine.layers.get(c, 0x4100_0007).is_none());
    assert_eq!(
        s.scalar(c, b"SP_Create", &[7, 20, 30, 200, 100, 50, -1], &mut r.host)
            .unwrap(),
        Some(1)
    );
    let layer = r.host.engine.layers.get(c, 0x4100_0007).unwrap();
    assert_eq!(layer.rect, [31., 42., 20., 30.]);
    assert!(!layer.visible);
    s.scalar(c, b"SP_SetZ", &[7, -27], &mut r.host).unwrap();
    s.scalar(c, b"SP_SetShow", &[7, 1], &mut r.host).unwrap();
    s.scalar(c, b"SP_SetBlendRate", &[7, 999], &mut r.host)
        .unwrap();
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
    assert!(
        s.scalar(ClientId(2), b"SP_SetX", &[7, 20], &mut r.host)
            .is_err()
    );
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
    assert!(
        s.scalar(c, b"SP_SetDrawMethod", &[1, 2], &mut r.host)
            .is_err()
    );
    let old = serde_json::to_value(&r.host.engine).unwrap();
    assert!(
        s.scalar(c, b"SP_SetPos", &[1, i32::MAX, 0], &mut r.host)
            .is_err()
    );
    assert_eq!(s.get(1).unwrap().position, [0, 0]);
    assert_eq!(serde_json::to_value(&r.host.engine).unwrap(), old);
    assert_eq!(
        s.scalar(c, b"NotARealCall", &[], &mut r.host).unwrap(),
        None
    );
}

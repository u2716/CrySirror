use crysirror_core::state::ClientId;
use crysirror_engine::{App, Value};
use crysirror_runtime::Runtime;
use serde_json::{Value as J, json};
fn call(app: &mut App, method: &str, args: J) -> anyhow::Result<J> {
    let cx = app.context();
    let layers = cx.singleton("Layers")?;
    Ok(cx
        .call(
            layers,
            method,
            &args
                .as_array()
                .unwrap()
                .iter()
                .cloned()
                .map(Value::from_json)
                .collect::<Vec<_>>(),
        )?
        .to_json())
}
#[test]
fn public_viewport_clip_is_scoped_and_preserves_local_clip_and_live_motion() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("assets")).unwrap();
    let path = dir.path().join("project.json");
    std::fs::write(&path,serde_json::to_vec(&json!({"schema":2,"name":"clip API","content_revision":"1","assets":"assets","canvas":[64,64],"clients":[{"name":"a","adapter":"crysirror-script-v1","game":"a"},{"name":"b","adapter":"crysirror-script-v1","game":"b"}]})).unwrap()).unwrap();
    let mut app = App::new(
        Runtime::load(&path, &dir.path().join("state")).unwrap(),
        "a",
    )
    .unwrap();
    app.init().unwrap();
    app.start().unwrap();
    call(&mut app,"put",json!([{"id":1,"rect":[10,12,20,20],"clip":[0,0,10,10],"paint":{"kind":"solid","color":[255,255,255,255]}}])).unwrap();
    call(
        &mut app,
        "beginMotion",
        json!([1,{"property":"x","to":30,"duration":100,"external":true}]),
    )
    .unwrap();
    let before = call(&mut app, "motionStatus", json!([1])).unwrap();
    call(&mut app, "setViewportClip", json!([1, [15, 15, 8, 8]])).unwrap();
    assert_eq!(
        call(&mut app, "viewportClip", json!([1])).unwrap(),
        json!([15., 15., 8., 8.])
    );
    assert_eq!(call(&mut app, "motionStatus", json!([1])).unwrap(), before);
    call(&mut app, "setMotionElapsed", json!([1, "x", 50])).unwrap();
    let layer = app
        .runtime()
        .unwrap()
        .host
        .engine
        .layers
        .get(ClientId(1), 1)
        .unwrap();
    assert_eq!((layer.rect[0], layer.clip), (20., Some([0., 0., 10., 10.])));
    assert!(call(&mut app, "setViewportClip", json!([1, [0, 0, -1, 2]])).is_err());
    app.context().state().unwrap().client = ClientId(2);
    assert!(call(&mut app, "viewportClip", json!([1])).is_err());
    assert!(call(&mut app, "setViewportClip", json!([1, null])).is_err());
    app.context().state().unwrap().client = ClientId(1);
    call(&mut app, "setViewportClip", json!([1, [0, 0, 0, 2]])).unwrap();
    assert!(
        app.runtime()
            .unwrap()
            .host
            .engine
            .frame([64, 64])
            .unwrap()
            .is_empty()
    );
    call(&mut app, "setViewportClip", json!([1, null])).unwrap();
    assert_eq!(call(&mut app, "viewportClip", json!([1])).unwrap(), J::Null);
}

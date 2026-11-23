// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
use crysirror_core::state::ClientId;
use crysirror_engine::{App, Value};
use crysirror_runtime::Runtime;
use serde_json::{json, Value as Json};
fn call(app: &mut App, name: &str, args: Json) -> anyhow::Result<Json> {
    let cx = app.context();
    let h = cx.singleton("Layers")?;
    Ok(cx
        .call(
            h,
            name,
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
fn public_solid_color_is_scoped_validated_and_keeps_live_motion() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir(d.path().join("assets")).unwrap();
    let project = d.path().join("project.json");
    std::fs::write(&project,serde_json::to_vec(&json!({"schema":2,"name":"solid API","content_revision":"1","assets":"assets","canvas":[64,64],"clients":[{"name":"a","adapter":"crysirror-script-v1","game":"a"},{"name":"b","adapter":"crysirror-script-v1","game":"b"}]})).unwrap()).unwrap();
    let mut app = App::new(
        Runtime::load(&project, &d.path().join("state")).unwrap(),
        "a",
    )
    .unwrap();
    app.init().unwrap();
    app.start().unwrap();
    call(&mut app,"put",json!([{"id":1,"rect":[0,0,2,2],"paint":{"kind":"solid","color":[1,2,3,4]},"opacity":99,"clip":[0,0,2,2],"order":[2,3]}])).unwrap();
    call(
        &mut app,
        "beginMotion",
        json!([1,{"property":"x","to":4,"duration":100,"external":true}]),
    )
    .unwrap();
    let before = call(&mut app, "motionStatus", json!([1])).unwrap();
    call(&mut app, "setSolidColor", json!([1, [7, 8, 9, 10]])).unwrap();
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
    assert_eq!(
        (layer.rect[0], layer.opacity, layer.order, layer.clip),
        (2., 99, [2, 3], Some([0., 0., 2., 2.]))
    );
    assert_eq!(
        serde_json::to_value(&layer.paint).unwrap(),
        json!({"kind":"solid","color":[7,8,9,10]})
    );
    assert!(call(&mut app, "setSolidColor", json!([1, [256, 0, 0, 0]])).is_err());
    assert!(call(&mut app, "setSolidColor", json!([1, [0, 0, 0]])).is_err());
    app.context().state().unwrap().client = ClientId(2);
    assert!(call(&mut app, "setSolidColor", json!([1, [0, 0, 0, 0]])).is_err());
    assert!(app
        .typescript_declarations()
        .unwrap()
        .contains("setSolidColor("));
}

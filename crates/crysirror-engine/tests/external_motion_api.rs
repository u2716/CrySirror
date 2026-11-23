use crysirror_core::state::ClientId;
use crysirror_engine::{App, Value};
use crysirror_runtime::Runtime;
use serde_json::{json, Value as Json};
fn call(app: &mut App, name: &str, args: Json) -> anyhow::Result<Json> {
    let cx = app.context();
    let handle = cx.singleton("Layers")?;
    Ok(cx
        .call(
            handle,
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
fn public_external_producer_commits_pose_with_exact_scoped_receipts() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("assets")).unwrap();
    let project = root.path().join("project.json");
    std::fs::write(&project,serde_json::to_vec(&json!({"schema":2,"name":"external motion API","content_revision":"1","assets":"assets","canvas":[64,64],"clients":[{"name":"a","adapter":"crysirror-script-v1","game":"a"},{"name":"b","adapter":"crysirror-script-v1","game":"b"}]})).unwrap()).unwrap();
    let mut app = App::new(
        Runtime::load(&project, &root.path().join("state")).unwrap(),
        "a",
    )
    .unwrap();
    app.init().unwrap();
    app.start().unwrap();
    call(
        &mut app,
        "put",
        json!([{"id":1,"rect":[0,0,20,30],"paint":{"kind":"solid","color":[1,2,3,255]}}]),
    )
    .unwrap();
    call(&mut app, "beginExternalMotion", json!([1, 7])).unwrap();
    let status = call(&mut app, "motionStatus", json!([1])).unwrap();
    assert_eq!(status["complete"], false);
    let generation = status["drivers"][0]["stamp"]["generation"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(status["drivers"][0]["stamp"]["revision"], "0");
    call(
        &mut app,
        "requestExternalMotionSkip",
        json!([1, 7, generation]),
    )
    .unwrap();
    let status = call(&mut app, "motionStatus", json!([1])).unwrap();
    assert_eq!(status["drivers"][0]["skip_requested"], true);
    assert_eq!(status["complete"], false);
    let stamp = json!({"generation":generation,"revision":"1"});
    call(
        &mut app,
        "commitExternalMotion",
        json!([1,7,stamp,true,{"rect":[5,6,20,30],"opacity":99,"transform":{"local_matrix":[1,0,0,0,0,1,0,0,0,0,1,0,10,20,0,1]}}]),
    )
    .unwrap();
    assert_eq!(
        call(&mut app, "motionStatus", json!([1])).unwrap()["complete"],
        true
    );
    let frame = app
        .runtime()
        .unwrap()
        .host
        .engine
        .layers
        .frame([64, 64])
        .unwrap();
    assert_eq!([frame[0].quad[0][0], frame[0].quad[0][1]], [15., 26.]);
    assert!(call(
        &mut app,
        "commitExternalMotion",
        json!([1,7,stamp,true,{"opacity":0}])
    )
    .is_err());
    assert_eq!(
        app.runtime()
            .unwrap()
            .host
            .engine
            .layers
            .get(ClientId(1), 1)
            .unwrap()
            .opacity,
        99
    );
    call(&mut app, "setRect", json!([1, [8, 9, 20, 30]])).unwrap();
    assert_eq!(
        app.runtime()
            .unwrap()
            .host
            .engine
            .layers
            .get(ClientId(1), 1)
            .unwrap()
            .rect,
        [8., 9., 20., 30.]
    );
    app.context().state().unwrap().client = ClientId(2);
    assert!(call(&mut app, "removeExternalMotion", json!([1, 7, generation])).is_err());
    app.context().state().unwrap().client = ClientId(1);
    call(&mut app, "removeExternalMotion", json!([1, 7, generation])).unwrap();
    assert!(
        call(&mut app, "motionStatus", json!([1])).unwrap()["drivers"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(app
        .typescript_declarations()
        .unwrap()
        .contains("commitExternalMotion("));
}

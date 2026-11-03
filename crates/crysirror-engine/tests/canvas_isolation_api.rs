//! Public live isolated Canvas API, without any native adapter or GPU/device.
use crysirror_core::state::ClientId;
use crysirror_engine::{App, Value};
use crysirror_runtime::Runtime;
use crysirror_scene::layer::Paint;
use serde_json::{Value as Json, json};
fn call(app: &mut App, class: &'static str, method: &str, args: Json) -> anyhow::Result<Json> {
    let cx = app.context();
    let object = cx.singleton(class)?;
    Ok(cx
        .call(
            object,
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
fn public_canvas_isolation_refreshes_actual_owned_snapshot_through_typed_commands()
-> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    std::fs::create_dir(root.path().join("assets"))?;
    let path = root.path().join("project.json");
    std::fs::write(
        &path,
        serde_json::to_vec(
            &json!({"schema":2,"name":"isolated canvas API","content_revision":"1","assets":"assets","canvas":[32,32],"clients":[{"name":"a","adapter":"crysirror-script-v1","game":"unused"}]}),
        )?,
    )?;
    let mut app = App::new(Runtime::load(&path, &root.path().join("state"))?, "a")?;
    app.init()?;
    app.start()?;
    call(
        &mut app,
        "Layers",
        "put",
        json!([{"id":50,"rect":[2,3,8,8],"opacity":128,"paint":{"kind":"solid","color":[0,0,0,0]}}]),
    )?;
    call(
        &mut app,
        "Canvases",
        "apply",
        json!([[{"op":"create","id":1,"width":8,"height":8},{"op":"clear","id":1,"color":[0,0,0,0]},
  {"op":"draw","id":1,"layer":{"id":0,"rect":[0,0,8,8],"paint":{"kind":"solid","color":[255,0,0,255]}}},
  {"op":"present_isolated","id":1,"layer":10,"parent":50,"position":[1,1]}]]),
    )?;
    let source = call(&mut app, "Canvases", "get", json!([1]))?;
    assert_eq!(source["width"], 8);
    let before = {
        let cx = app.context();
        let state = cx.state()?;
        let layer = state
            .runtime
            .host
            .engine
            .layers
            .get(ClientId(1), 10)
            .unwrap();
        assert_eq!(layer.parent, Some(50));
        let Paint::Snapshot { layers, .. } = &layer.paint else {
            panic!("public command did not publish an actual shared snapshot")
        };
        layers.clone()
    };
    call(
        &mut app,
        "Canvases",
        "apply",
        json!([[{"op":"clear","id":1,"color":[0,255,0,255]}]]),
    )?;
    {
        let cx = app.context();
        let state = cx.state()?;
        let layer = state
            .runtime
            .host
            .engine
            .layers
            .get(ClientId(1), 10)
            .unwrap();
        let Paint::Snapshot { layers, .. } = &layer.paint else {
            panic!("source edits lost isolated mode")
        };
        assert!(!std::sync::Arc::ptr_eq(&before, layers));
        assert!(layers.iter().any(|l| matches!(
            l.paint,
            Paint::Solid {
                color: [0, 255, 0, 255]
            }
        )));
        assert!(before.iter().any(|l| matches!(
            l.paint,
            Paint::Solid {
                color: [255, 0, 0, 255]
            }
        )));
        assert!(
            state
                .runtime
                .host
                .engine
                .layers
                .get(ClientId(2), 10)
                .is_none()
        );
    }
    assert!(call(&mut app,"Canvases","apply",json!([[{"op":"clear","id":1,"color":[4,5,6,255]},{"op":"present_isolated","id":1,"layer":11,"parent":999}]] )).is_err());
    assert_eq!(
        call(&mut app, "Canvases", "readPixel", json!([1, 1, 1]))?,
        json!([0, 255, 0, 255])
    );
    Ok(())
}

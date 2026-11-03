//! Public transactional drawing/parent creation without a VM or graphics device.
use crysirror_core::state::ClientId;
use crysirror_engine::{App, Value};
use crysirror_runtime::Runtime;
use serde_json::{json, Value as Json};
fn call(app: &mut App, method: &str, batch: Json) -> anyhow::Result<()> {
    let cx = app.context();
    let object = cx.singleton("Canvases")?;
    cx.call(object, method, &[Value::from_json(batch)])?;
    Ok(())
}
fn app() -> anyhow::Result<(tempfile::TempDir, App)> {
    let root = tempfile::tempdir()?;
    std::fs::create_dir(root.path().join("assets"))?;
    let path = root.path().join("project.json");
    std::fs::write(
        &path,
        serde_json::to_vec(
            &json!({"schema":2,"name":"atomic Canvas batch API","content_revision":"1","assets":"assets","canvas":[16,16],"clients":[{"name":"a","adapter":"crysirror-script-v1","game":"unused"}]}),
        )?,
    )?;
    let mut app = App::new(Runtime::load(&path, &root.path().join("state"))?, "a")?;
    app.init()?;
    app.start()?;
    Ok((root, app))
}
fn owned_scene_state(engine: &crysirror_scene::EngineSystems) -> anyhow::Result<Json> {
    Ok(
        json!({"parent":engine.layers.get(ClientId(1),50),"surface":engine.canvases.surface(ClientId(1),1),
        "motion":engine.layers.motion_status(ClientId(1),50)?,"ui":&engine.ui,"world":&engine.world}),
    )
}
#[test]
fn public_batch_creates_parent_canvas_and_driver_atomically_and_preserves_old_apply(
) -> anyhow::Result<()> {
    let (_root, mut app) = app()?;
    call(
        &mut app,
        "applyBatch",
        json!({"prepare_layers":[{"op":"put","layer":{"id":50,"rect":[2,3,4,4],"paint":{"kind":"solid","color":[0,0,0,0]}}},{"op":"begin_external_motion","id":50,"driver":7}],"canvases":[{"op":"create","id":1,"width":4,"height":4},{"op":"clear","id":1,"color":[3,7,11,255]},{"op":"present_isolated","id":1,"layer":10,"parent":50}],"layers":[{"op":"set_opacity","id":50,"opacity":128}]}),
    )?;
    let (before, stamp) = {
        let cx = app.context();
        let state = cx.state()?;
        assert_eq!(
            state
                .runtime
                .host
                .engine
                .layers
                .get(ClientId(1), 10)
                .unwrap()
                .parent,
            Some(50)
        );
        assert_eq!(
            state
                .runtime
                .host
                .engine
                .layers
                .get(ClientId(1), 50)
                .unwrap()
                .opacity,
            128
        );
        assert!(state
            .runtime
            .host
            .engine
            .layers
            .get(ClientId(2), 50)
            .is_none());
        (
            owned_scene_state(&state.runtime.host.engine)?,
            state
                .runtime
                .host
                .engine
                .layers
                .motion_status(ClientId(1), 50)?
                .drivers[0]
                .stamp,
        )
    };
    assert!(call(&mut app,"applyBatch",json!({"prepare_layers":[{"op":"set_opacity","id":50,"opacity":64}],"canvases":[{"op":"clear","id":1,"color":[101,103,107,255]}],"layers":[{"op":"set_opacity","id":999,"opacity":1}]})).is_err());
    {
        let cx = app.context();
        let state = cx.state()?;
        assert_eq!(owned_scene_state(&state.runtime.host.engine)?, before);
        assert_eq!(
            state
                .runtime
                .host
                .engine
                .layers
                .motion_status(ClientId(1), 50)?
                .drivers[0]
                .stamp,
            stamp
        );
    }
    call(
        &mut app,
        "apply",
        json!([{"op":"clear","id":1,"color":[13,17,19,255]}]),
    )?;
    {
        let cx = app.context();
        let state = cx.state()?;
        assert_eq!(
            state
                .runtime
                .host
                .engine
                .canvases
                .sample_pixel(ClientId(1), 1, [1, 1])?,
            [13, 17, 19, 255]
        );
    }
    Ok(())
}
#[test]
fn public_batch_enforces_combined_count_payload_type_and_unknown_field_limits_before_mutation(
) -> anyhow::Result<()> {
    let (_root, mut app) = app()?;
    let before = {
        let cx = app.context();
        let state = cx.state()?;
        serde_json::to_value(&state.runtime.host.engine)?
    };
    assert!(call(&mut app, "applyBatch", json!([])).is_err());
    assert!(call(&mut app, "applyBatch", json!({"client":2})).is_err());
    let commands = vec![json!({"op":"create","id":1,"width":1,"height":1}); 2048];
    let layers = vec![json!({"op":"set_opacity","id":50,"opacity":128}); 2049];
    let error = call(
        &mut app,
        "applyBatch",
        json!({"canvases":commands,"prepare_layers":layers}),
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains("command budget"), "{error:#}");
    let error = call(
        &mut app,
        "applyBatch",
        json!({"ui":[{"op":"set_caption","caption":"x".repeat(4*1024*1024)}]}),
    )
    .unwrap_err();
    assert!(
        format!("{error:#}").contains("payload exceeds4MiB"),
        "{error:#}"
    );
    {
        let cx = app.context();
        let state = cx.state()?;
        assert_eq!(serde_json::to_value(&state.runtime.host.engine)?, before);
    }
    Ok(())
}

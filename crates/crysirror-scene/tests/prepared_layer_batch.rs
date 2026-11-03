//! Explicit layer/Canvas phases over the same atomic engine transaction.
//! No VM, native renderer, GPU, device or external assets are required.
use crysirror_core::state::ClientId;
use crysirror_scene::{layer::Paint, EngineSystems, ServiceBatch};
use serde_json::json;
const CLIENT: ClientId = ClientId(1);
fn batch(value: serde_json::Value) -> ServiceBatch {
    serde_json::from_value(value).unwrap()
}
#[test]
fn new_parent_and_real_motion_producer_are_created_before_canvas_in_one_commit() {
    let mut engine = EngineSystems::default();
    let old: ServiceBatch = serde_json::from_value(json!({"layers":[]})).unwrap();
    assert!(
        old.prepare_layers.is_empty(),
        "old serialized batches retain default semantics"
    );
    engine.submit(CLIENT,&batch(json!({
        "prepare_layers":[
            {"op":"put","layer":{"id":50,"rect":[2,3,4,4],"paint":{"kind":"solid","color":[0,0,0,0]}}},
            {"op":"begin_external_motion","id":50,"driver":7}
        ],
        "canvases":[
            {"op":"create","id":1,"width":4,"height":4},
            {"op":"clear","id":1,"color":[3,7,11,255]},
            {"op":"present_isolated","id":1,"layer":10,"parent":50}
        ],
        "layers":[{"op":"set_opacity","id":50,"opacity":128}]
    }))).unwrap();
    let status = engine.layers.motion_status(CLIENT, 50).unwrap();
    assert_eq!(status.drivers.len(), 1);
    assert!(!status.complete);
    assert_eq!(engine.layers.get(CLIENT, 10).unwrap().parent, Some(50));
    let frame = engine.layers.frame([16, 16]).unwrap();
    let snapshot = frame
        .iter()
        .find(|layer| matches!(layer.paint, Paint::Snapshot { .. }))
        .unwrap();
    assert_eq!(snapshot.opacity, 128);
    assert_eq!(&snapshot.quad[0][..2], &[2., 3.]);
    assert!(engine.layers.get(ClientId(2), 50).is_none());
    //Live external producers are intentionally not framework-serializable.
    //The engine transaction uses an in-memory clone, not a save/restore.
    let mut decoded = engine.clone();
    decoded.validate().unwrap();
    decoded.submit(CLIENT,&batch(json!({"prepare_layers":[{"op":"set_opacity","id":50,"opacity":91}],"canvases":[{"op":"clear","id":1,"color":[13,17,19,255]}]}))).unwrap();
    assert_eq!(
        decoded.layers.motion_status(CLIENT, 50).unwrap().drivers[0].stamp,
        status.drivers[0].stamp
    );
    assert_eq!(decoded.layers.get(CLIENT, 50).unwrap().opacity, 91);
}
#[test]
fn later_failure_rolls_back_prepared_layers_canvas_and_real_driver_revision() {
    let mut engine = EngineSystems::default();
    engine.submit(CLIENT,&batch(json!({"prepare_layers":[{"op":"put","layer":{"id":50,"rect":[0,0,4,4],"paint":{"kind":"solid","color":[0,0,0,0]}}},{"op":"begin_external_motion","id":50,"driver":7}],"canvases":[{"op":"create","id":1,"width":4,"height":4},{"op":"clear","id":1,"color":[3,7,11,255]},{"op":"present_isolated","id":1,"layer":10,"parent":50}]}))).unwrap();
    let stamp = engine.layers.motion_status(CLIENT, 50).unwrap().drivers[0].stamp;
    let Paint::Snapshot { layers: old, .. } = &engine.layers.get(CLIENT, 10).unwrap().paint else {
        unreachable!()
    };
    let old = old.clone();
    let before_layer = serde_json::to_value(engine.layers.get(CLIENT, 50)).unwrap();
    let before_canvas = serde_json::to_value(engine.canvases.surface(CLIENT, 1)).unwrap();
    let before_status =
        serde_json::to_value(engine.layers.motion_status(CLIENT, 50).unwrap()).unwrap();
    let before_ui = serde_json::to_value(&engine.ui).unwrap();
    let before_world = serde_json::to_value(&engine.world).unwrap();
    let mut next = stamp;
    next.revision += 1;
    assert!(engine.submit(CLIENT,&batch(json!({
        "prepare_layers":[
            {"op":"set_opacity","id":50,"opacity":64},
            {"op":"commit_external_motion","id":50,"driver":7,"stamp":next,"complete":true,"pose":{"rect":[5,6,4,4]}},
            {"op":"put","layer":{"id":60,"rect":[0,0,4,4],"paint":{"kind":"solid","color":[0,0,0,0]}}}
        ],
        "canvases":[{"op":"clear","id":1,"color":[101,103,107,255]},{"op":"present_isolated","id":1,"layer":11,"parent":60}],
        "layers":[{"op":"set_opacity","id":999,"opacity":1}]
    }))).is_err());
    assert_eq!(
        serde_json::to_value(engine.layers.get(CLIENT, 50)).unwrap(),
        before_layer
    );
    assert_eq!(
        serde_json::to_value(engine.canvases.surface(CLIENT, 1)).unwrap(),
        before_canvas
    );
    assert_eq!(
        serde_json::to_value(engine.layers.motion_status(CLIENT, 50).unwrap()).unwrap(),
        before_status
    );
    assert_eq!(serde_json::to_value(&engine.ui).unwrap(), before_ui);
    assert_eq!(serde_json::to_value(&engine.world).unwrap(), before_world);
    assert_eq!(
        engine.layers.motion_status(CLIENT, 50).unwrap().drivers[0].stamp,
        stamp
    );
    let Paint::Snapshot { layers: actual, .. } = &engine.layers.get(CLIENT, 10).unwrap().paint
    else {
        unreachable!()
    };
    assert!(std::sync::Arc::ptr_eq(&old, actual));
    assert!(engine.layers.get(CLIENT, 60).is_none());
    assert!(engine.layers.get(CLIENT, 11).is_none());
    assert_eq!(
        engine.canvases.sample_pixel(CLIENT, 1, [1, 1]).unwrap(),
        [3, 7, 11, 255]
    );
}
#[test]
fn ordinary_layers_still_follow_canvas_and_preparation_failures_publish_nothing() {
    let mut engine = EngineSystems::default();
    let before = serde_json::to_value(&engine).unwrap();
    assert!(engine.submit(CLIENT,&batch(json!({"canvases":[{"op":"create","id":1,"width":2,"height":2},{"op":"present_isolated","id":1,"layer":10,"parent":50}],"layers":[{"op":"put","layer":{"id":50,"rect":[0,0,2,2],"paint":{"kind":"solid","color":[0,0,0,0]}}}]}))).is_err());
    assert_eq!(serde_json::to_value(&engine).unwrap(), before);
    assert!(engine.submit(CLIENT,&batch(json!({"prepare_layers":[{"op":"put","layer":{"id":50,"rect":[0,0,2,2],"paint":{"kind":"solid","color":[0,0,0,0]}}},{"op":"set_opacity","id":999,"opacity":1}],"canvases":[{"op":"create","id":1,"width":2,"height":2}]}))).is_err());
    assert_eq!(serde_json::to_value(&engine).unwrap(), before);
}

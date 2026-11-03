use crysirror_core::state::ClientId;
use crysirror_scene::{layer::{LayerCommand, LayerSystem}, ui::{UiCommand, UiSystem}};
use serde_json::{json, Value};

fn commands(value: Value) -> Vec<LayerCommand> { serde_json::from_value(value).unwrap() }
fn put(layers: &mut LayerSystem, client: ClientId) {
    layers.apply_batch(client, &commands(json!([{"op":"put","layer":{"id":1,
        "rect":[0,0,20,30],"paint":{"kind":"solid","color":[1,2,3,255]}}}]))).unwrap();
}
#[test]
fn full_transform_tracks_continue_after_restore_and_remain_client_scoped() {
    let mut layers=LayerSystem::default();
    put(&mut layers,ClientId(1));put(&mut layers,ClientId(2));
    let targets=[("x",10.),("y",20.),("z",30.),("scale_x",2.),("scale_y",3.),("scale_z",4.),
        ("rotation_x",1.),("rotation_y",2.),("rotation_z",3.),("pivot_x",5.),("pivot_y",6.),("pivot_z",7.),("opacity",0.)];
    let mut batch=Vec::new();
    for (property,to) in targets {
        batch.push(json!({"op":"begin_motion","id":1,"track":{"property":property,"to":to,"duration":100,"external":true}}));
        batch.push(json!({"op":"set_motion_elapsed","id":1,"property":property,"elapsed":40}));
    }
    layers.apply_batch(ClientId(1),&commands(Value::Array(batch))).unwrap();
    let status=layers.motion_status(ClientId(1),1).unwrap();
    assert_eq!(status.tracks.len(),13);assert!(!status.complete);
    let before=serde_json::to_vec(&layers).unwrap();
    let mut restored:LayerSystem=serde_json::from_slice(&before).unwrap();restored.validate().unwrap();
    let layer=restored.get(ClientId(1),1).unwrap();
    assert_eq!((layer.rect[0],layer.rect[1],layer.transform.z),(4.,8.,12.));
    assert!((layer.transform.rotate-1.2).abs()<1e-6);
    assert!((layer.transform.scale_z-2.2).abs()<1e-6);
    assert_eq!((layer.transform.pivot_x,layer.transform.pivot_y,layer.transform.pivot_z),(2.,2.4,2.8));
    assert_eq!(layer.opacity,153);
    assert_eq!(restored.get(ClientId(2),1).unwrap().opacity,255);
    let batch=targets.iter().map(|(property,_)|json!({"op":"set_motion_elapsed","id":1,"property":property,"elapsed":100})).collect::<Vec<_>>();
    restored.apply_batch(ClientId(1),&commands(Value::Array(batch))).unwrap();
    let layer=restored.get(ClientId(1),1).unwrap();
    assert_eq!((layer.transform.rotate_x,layer.transform.rotate_y,layer.transform.rotate),(1.,2.,3.));
    assert_eq!((layer.transform.scale_x,layer.transform.scale_y,layer.transform.scale_z),(2.,3.,4.));
    assert_eq!((layer.transform.pivot_x,layer.transform.pivot_y,layer.transform.pivot_z),(5.,6.,7.));
    assert_eq!(layer.opacity,0);assert!(restored.motion_status(ClientId(1),1).unwrap().complete);
    assert_eq!(restored.get(ClientId(2),1).unwrap().transform.rotate,0.);
    assert!(restored.motion_status(ClientId(3),1).is_err());
}
#[test]
fn opacity_reports_the_actual_applied_byte_during_automatic_tick_and_finish() {
    let mut layers=LayerSystem::default();put(&mut layers,ClientId(1));
    layers.apply_batch(ClientId(1),&commands(json!([{"op":"begin_motion","id":1,
        "track":{"property":"opacity","from":0.5,"to":255.,"duration":10}}]))).unwrap();
    assert_eq!(layers.get(ClientId(1),1).unwrap().opacity,0);
    assert_eq!(layers.motion_status(ClientId(1),1).unwrap().tracks[0].current,0.);
    layers.tick(5);
    assert_eq!(layers.get(ClientId(1),1).unwrap().opacity,127);
    assert_eq!(layers.motion_status(ClientId(1),1).unwrap().tracks[0].current,127.);
    let mut restored:LayerSystem=serde_json::from_value(serde_json::to_value(&layers).unwrap()).unwrap();
    restored.validate().unwrap();restored.tick(5);
    assert_eq!(restored.get(ClientId(1),1).unwrap().opacity,255);
    assert!(restored.motion_status(ClientId(1),1).unwrap().complete);
    restored.apply_batch(ClientId(1),&commands(json!([
        {"op":"begin_motion","id":1,"track":{"property":"opacity","to":254.9,"duration":10}},
        {"op":"end_motion","id":1,"property":"opacity","finish":true}
    ]))).unwrap();
    assert_eq!(restored.get(ClientId(1),1).unwrap().opacity,254);
    assert!(restored.motion_status(ClientId(1),1).unwrap().tracks.is_empty());
}
#[test]
fn invalid_visual_endpoints_and_ui_transform_tracks_roll_back() {
    let mut layers=LayerSystem::default();put(&mut layers,ClientId(1));
    let before=serde_json::to_value(&layers).unwrap();
    for (property,to) in [("opacity",256.),("opacity",-1.),("scale_z",0.),("scale_z",-1.)] {
        let batch=commands(json!([
            {"op":"set_visible","id":1,"visible":false},
            {"op":"begin_motion","id":1,"track":{"property":property,"to":to,"duration":10}}
        ]));
        assert!(layers.apply_batch(ClientId(1),&batch).is_err());
        assert_eq!(serde_json::to_value(&layers).unwrap(),before);
    }
    let mut ui=UiSystem::default();
    ui.apply_batch(ClientId(1),&serde_json::from_value::<Vec<UiCommand>>(json!([
        {"op":"put","widget":{"id":1,"rect":[0,0,20,20]}}
    ])).unwrap()).unwrap();
    let before=serde_json::to_value(&ui).unwrap();
    for property in ["z","scale_z","rotation_x","rotation_y","rotation_z","pivot_x","pivot_y","pivot_z","opacity"] {
        let batch=serde_json::from_value::<Vec<UiCommand>>(json!([
            {"op":"begin_motion","id":1,"track":{"property":property,"to":2,"duration":10}}
        ])).unwrap();
        assert!(ui.apply_batch(ClientId(1),&batch).is_err());
        assert_eq!(serde_json::to_value(&ui).unwrap(),before);
    }
}

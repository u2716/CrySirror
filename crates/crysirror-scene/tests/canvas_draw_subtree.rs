//! Appending snapshots must retain root geometry and earlier painter content.
use crysirror_core::state::ClientId;
use crysirror_scene::{
    EngineSystems, ServiceBatch,
    canvas::{CanvasCommand, CanvasNode},
    layer::{LayerSpec, Paint},
};
use serde_json::json;
#[test]
fn appended_subtrees_keep_root_transform_group_alpha_and_painter_order() {
    let owner = ClientId(7);
    let mut engine = EngineSystems::default();
    let roots:Vec<LayerSpec>=vec![
  serde_json::from_value(json!({"id":1,"rect":[8,4,20,10],"opacity":128,"transform":{"scale_x":1.5,"rotate":0.2},"paint":{"kind":"solid","color":[255,0,0,255]}})).unwrap(),
  serde_json::from_value(json!({"id":2,"rect":[12,6,20,10],"paint":{"kind":"solid","color":[0,255,0,128]}})).unwrap(),
 ];
    engine.layers.install(owner, &roots).unwrap();
    let commands: Vec<CanvasCommand> = serde_json::from_value(json!([
     {"op":"create","id":3,"width":64,"height":64},
     {"op":"draw_subtree","source":1,"destination":3},
     {"op":"draw_subtree","source":2,"destination":3}
    ]))
    .unwrap();
    engine
        .submit(
            owner,
            &ServiceBatch {
                canvases: commands,
                ..Default::default()
            },
        )
        .unwrap();
    let surface = engine.canvases.surface(owner, 3).unwrap();
    assert_eq!(surface.nodes.len(), 2);
    let CanvasNode::Draw { layer: first } = &surface.nodes[0] else {
        panic!("expected first retained draw")
    };
    assert_eq!(first.rect, roots[0].rect);
    assert_eq!(first.opacity, 128);
    assert_eq!(first.transform, roots[0].transform);
    let Paint::Snapshot { layers, .. } = &first.paint else {
        panic!("expected isolated source")
    };
    assert_eq!(layers[0].rect, [0., 0., 20., 10.]);
    assert_eq!(layers[0].opacity, 255);
    assert_eq!(serde_json::to_value(&layers[0].paint).unwrap(),serde_json::to_value(&roots[0].paint).unwrap());
    let CanvasNode::Draw { layer: second } = &surface.nodes[1] else {
        panic!("expected second retained draw")
    };
    assert_eq!(second.rect, roots[1].rect);
}

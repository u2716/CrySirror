//! A zero-sized retained pose can expand again without invalid draw geometry.
use crysirror_core::state::ClientId;
use crysirror_scene::layer::{LayerCommand, LayerSpec, LayerSystem, SpriteTransform};
use serde_json::json;
#[test]
fn collapsed_parent_prunes_subtree_and_recovers_with_owner_scope() {
    let owner = ClientId(1);
    let peer = ClientId(2);
    let mut layers = LayerSystem::default();
    let specs:Vec<LayerSpec>=serde_json::from_value(json!([
        {"id":1,"rect":[20,10,100,100],"transform":{"scale_x":0},"paint":{"kind":"solid","color":[255,0,0,255]}},
        {"id":2,"parent":1,"rect":[0,0,20,20],"paint":{"kind":"solid","color":[0,255,0,255]}}
    ])).unwrap();
    layers.install(owner, &specs).unwrap();
    let mut other = specs[0].clone();
    other.transform.scale_x = 1.;
    layers.install(peer, &[other]).unwrap();
    let frame = layers.frame([200, 150]).unwrap();
    assert!(frame.iter().all(|p| p.owner == 2));
    let saved = serde_json::to_vec(&layers).unwrap();
    let restored: LayerSystem = serde_json::from_slice(&saved).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored.get(owner, 1).unwrap().transform.scale_x, 0.);
    let commands:Vec<LayerCommand>=serde_json::from_value(json!([
        {"op":"begin_motion","id":1,"track":{"property":"scale_x","from":0,"to":1,"duration":100,"external":true}},
        {"op":"set_motion_elapsed","id":1,"property":"scale_x","elapsed":50}
    ])).unwrap();
    layers.apply_batch(owner, &commands).unwrap();
    assert_eq!(layers.get(owner, 1).unwrap().transform.scale_x, 0.5);
    assert!(
        layers
            .frame([200, 150])
            .unwrap()
            .iter()
            .any(|p| p.owner == 1 && p.id == 2)
    );
    let commands:Vec<LayerCommand>=serde_json::from_value(json!([
        {"op":"begin_motion","id":1,"track":{"property":"scale_x","to":0,"duration":50,"external":true}},
        {"op":"set_motion_elapsed","id":1,"property":"scale_x","elapsed":50}
    ])).unwrap();
    layers.apply_batch(owner, &commands).unwrap();
    assert!(
        layers
            .frame([200, 150])
            .unwrap()
            .iter()
            .all(|p| p.owner == 2)
    );
}
#[test]
fn collapsed_quad_is_absent_and_negative_mirroring_remains_valid() {
    let mut transform = SpriteTransform {
        scale_x: 0.,
        ..Default::default()
    };
    transform.validate().unwrap();
    assert!(
        crysirror_scene::projection::sprite_quad_points(&transform, 0., 0., 20., 20., 100., 100.)
            .is_none()
    );
    transform.scale_x = -1.;
    transform.validate().unwrap();
    assert!(
        crysirror_scene::projection::sprite_quad_points(&transform, 0., 0., 20., 20., 100., 100.)
            .is_some()
    );
}

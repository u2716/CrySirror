//! Target-space clips, with no VM/window/GPU and exact retained state checks.
use crysirror_core::state::ClientId;
use crysirror_scene::{EngineSystems, ServiceBatch, canvas::CanvasCommand as C, layer::*};
use serde_json::json;
const A: ClientId = ClientId(1);
fn node(id: u32, parent: Option<u32>, rect: [f32; 4]) -> LayerSpec {
    serde_json::from_value(
        json!({"id":id,"parent":parent,"rect":rect,"paint":{"kind":"solid","color":[255,0,0,255]}}),
    )
    .unwrap()
}
#[test]
fn target_clips_ignore_parent_transforms_intersect_local_clips_and_keep_motion() {
    let mut layers = LayerSystem::default();
    let mut parent = node(1, None, [20., 10., 40., 40.]);
    parent.viewport_clip = Some([8., 6., 20., 14.]);
    parent.transform.scale_x = 2.;
    parent.transform.rotate = 0.25;
    let mut child = node(2, Some(1), [0., 0., 30., 30.]);
    child.viewport_clip = Some([12., 8., 60., 60.]);
    layers.install(A, &[parent.clone(), child]).unwrap();
    let frame = layers.frame([100, 100]).unwrap();
    assert_eq!(
        frame.iter().find(|p| p.id == 2).unwrap().clip,
        Some([12., 8., 16., 12.])
    );
    let driver:LayerCommand=serde_json::from_value(json!({"op":"begin_motion","id":2,"track":{"property":"x","to":4,"duration":100,"external":true}})).unwrap();
    layers.apply_batch(A, &[driver]).unwrap();
    let before = layers.motion_status(A, 2).unwrap();
    layers
        .apply_batch(
            A,
            &[LayerCommand::SetViewportClip {
                id: 2,
                clip: Some([9., 9., 4., 3.]),
            }],
        )
        .unwrap();
    assert_eq!(
        serde_json::to_value(layers.motion_status(A, 2).unwrap()).unwrap(),
        serde_json::to_value(before).unwrap()
    );
    parent.transform.rotate = 0.75;
    parent.transform.scale_x = 3.;
    layers
        .apply_batch(
            A,
            &[LayerCommand::SetTransform {
                id: 1,
                transform: parent.transform,
            }],
        )
        .unwrap();
    assert_eq!(
        layers
            .frame([100, 100])
            .unwrap()
            .iter()
            .find(|p| p.id == 2)
            .unwrap()
            .clip,
        Some([9., 9., 4., 3.])
    );
    // A local clip still follows its transform, independently of the target clip.
    layers
        .apply_batch(
            A,
            &[
                LayerCommand::SetTransform {
                    id: 1,
                    transform: Default::default(),
                },
                LayerCommand::SetClip {
                    id: 2,
                    clip: Some([0., 0., 2., 2.]),
                },
            ],
        )
        .unwrap();
    assert!(layers.frame([100, 100]).unwrap().iter().all(|p| p.id != 2));
}
#[test]
fn target_clip_zero_is_empty_and_bad_foreign_or_late_batches_rollback_with_serialized_state() {
    let mut layers = LayerSystem::default();
    layers
        .install(A, &[node(1, None, [0., 0., 10., 10.])])
        .unwrap();
    layers
        .apply_batch(
            A,
            &[LayerCommand::SetViewportClip {
                id: 1,
                clip: Some([0., 0., 0., 10.]),
            }],
        )
        .unwrap();
    assert!(layers.frame([20, 20]).unwrap().is_empty());
    let bytes = serde_json::to_vec(&layers).unwrap();
    for clip in [Some([0., 0., -1., 1.]), Some([f32::NAN, 0., 1., 1.])] {
        assert!(
            layers
                .apply_batch(A, &[LayerCommand::SetViewportClip { id: 1, clip }])
                .is_err()
        );
        assert_eq!(serde_json::to_vec(&layers).unwrap(), bytes);
    }
    assert!(
        layers
            .apply_batch(
                ClientId(2),
                &[LayerCommand::SetViewportClip { id: 1, clip: None }]
            )
            .is_err()
    );
    assert!(
        layers
            .transaction(|l| {
                l.apply_batch(A, &[LayerCommand::SetViewportClip { id: 1, clip: None }])?;
                l.apply_batch(
                    A,
                    &[LayerCommand::SetOpacity {
                        id: 999,
                        opacity: 1,
                    }],
                )
            })
            .is_err()
    );
    assert_eq!(serde_json::to_vec(&layers).unwrap(), bytes);
    let mut restored: LayerSystem = serde_json::from_slice(&bytes).unwrap();
    restored.validate().unwrap();
    restored
        .apply_batch(A, &[LayerCommand::SetViewportClip { id: 1, clip: None }])
        .unwrap();
    assert_eq!(restored.frame([20, 20]).unwrap().len(), 1);
}
#[test]
fn target_clip_survives_local_canvas_freeze_and_rejects_inexact_rotated_inverse() {
    let mut engine = EngineSystems::default();
    let mut root = node(1, None, [10., 6., 10., 10.]);
    root.viewport_clip = Some([12., 8., 4., 4.]);
    engine.layers.install(A, &[root.clone()]).unwrap();
    let create: C =
        serde_json::from_value(json!({"op":"create","id":3,"width":10,"height":10})).unwrap();
    let copy: C = serde_json::from_value(
        json!({"op":"copy_subtree","source":1,"destination":3,"rect":[0,0,10,10],"position":[0,0]}),
    )
    .unwrap();
    engine
        .submit(
            A,
            &ServiceBatch {
                canvases: vec![create, copy.clone()],
                ..Default::default()
            },
        )
        .unwrap();
    let surface = engine.canvases.surface(A, 3).unwrap();
    let specs = surface
        .nodes
        .iter()
        .find_map(|node| match node {
            crysirror_scene::canvas::CanvasNode::Draw { layer } => match &layer.paint {
                Paint::Snapshot { layers, .. } => Some(layers),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    let mut frozen = LayerSystem::default();
    frozen.install(A, specs).unwrap();
    assert_eq!(
        frozen.frame([10, 10]).unwrap()[0].clip,
        Some([2., 2., 4., 4.])
    );
    let old = serde_json::to_value(surface).unwrap();
    root.transform.rotate = 0.4;
    engine
        .layers
        .apply_batch(
            A,
            &[LayerCommand::SetTransform {
                id: 1,
                transform: root.transform,
            }],
        )
        .unwrap();
    let error = engine
        .submit(
            A,
            &ServiceBatch {
                canvases: vec![copy],
                ..Default::default()
            },
        )
        .unwrap_err();
    assert!(format!("{error:#}").contains("polygon clipping"));
    assert_eq!(
        old,
        serde_json::to_value(engine.canvases.surface(A, 3).unwrap()).unwrap()
    );
}

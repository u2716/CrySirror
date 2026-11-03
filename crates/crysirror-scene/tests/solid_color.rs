// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
use crysirror_core::state::ClientId;
use crysirror_scene::layer::{LayerCommand as C, LayerSpec, LayerSystem, Paint};
use serde_json::json;
fn layer(id: u32, paint: serde_json::Value) -> LayerSpec {
    serde_json::from_value(json!({"id":id,"rect":[0,2,4,4],"transform":{"z":12,"rotate":0.2},"opacity":77,"visible":false,"order":[2,9],"clip":[0,0,4,4],"paint":paint})).unwrap()
}
#[test]
fn solid_color_preserves_the_layer_and_unrelated_motion_and_depth() {
    let mut s = LayerSystem::default();
    let c = ClientId(1);
    let original = layer(1, json!({"kind":"solid","color":[1,2,3,4]}));
    s.apply_batch(
        c,
        &[
            C::Put { layer: original },
            C::SetDepth {
                id: 1,
                depth: Some(12.),
            },
            C::BeginMotion {
                id: 1,
                track: serde_json::from_value(json!({"property":"x","to":40,"duration":100}))
                    .unwrap(),
            },
        ],
    )
    .unwrap();
    s.tick(25);
    let before = s.get(c, 1).unwrap().clone();
    let motion = serde_json::to_value(s.motion_status(c, 1).unwrap()).unwrap();
    s.apply_batch(
        c,
        &[C::SetSolidColor {
            id: 1,
            color: [9, 8, 7, 6],
        }],
    )
    .unwrap();
    let mut expected = serde_json::to_value(before).unwrap();
    expected["paint"]["color"] = json!([9, 8, 7, 6]);
    assert_eq!(
        serde_json::to_value(s.get(c, 1).unwrap()).unwrap(),
        expected
    );
    assert_eq!(
        serde_json::to_value(s.motion_status(c, 1).unwrap()).unwrap(),
        motion
    );
    s.tick(25);
    assert_eq!(s.get(c, 1).unwrap().rect[0], 20.);
    assert!(matches!(
        s.get(c, 1).unwrap().paint,
        Paint::Solid {
            color: [9, 8, 7, 6]
        }
    ));
    // Visibility does not erase depth state. Re-enable solely to inspect frame.
    s.apply_batch(
        c,
        &[
            C::SetVisible {
                id: 1,
                visible: true,
            },
            C::SetClip { id: 1, clip: None },
        ],
    )
    .unwrap();
    assert_eq!(
        s.frame([64, 64]).unwrap()[0]
            .depth_order
            .as_ref()
            .unwrap()
            .path[0]
            .depth,
        12.
    );
}
#[test]
fn missing_foreign_and_non_solid_targets_fail_with_atomic_batch_rollback() {
    let mut s = LayerSystem::default();
    let c = ClientId(1);
    s.apply_batch(
        c,
        &[
            C::Put {
                layer: layer(1, json!({"kind":"solid","color":[1,2,3,4]})),
            },
            C::Put {
                layer: layer(2, json!({"kind":"image","asset":"test.png"})),
            },
            C::BeginMotion {
                id: 1,
                track: serde_json::from_value(json!({"property":"x","to":40,"duration":100}))
                    .unwrap(),
            },
        ],
    )
    .unwrap();
    let before = serde_json::to_value(s.get(c, 1).unwrap()).unwrap();
    let motion = serde_json::to_value(s.motion_status(c, 1).unwrap()).unwrap();
    assert!(s
        .apply_batch(
            c,
            &[
                C::SetSolidColor {
                    id: 1,
                    color: [0; 4]
                },
                C::SetSolidColor {
                    id: 2,
                    color: [0; 4]
                }
            ]
        )
        .is_err());
    assert_eq!(serde_json::to_value(s.get(c, 1).unwrap()).unwrap(), before);
    assert_eq!(
        serde_json::to_value(s.motion_status(c, 1).unwrap()).unwrap(),
        motion
    );
    assert!(s
        .apply_batch(
            ClientId(2),
            &[C::SetSolidColor {
                id: 1,
                color: [0; 4]
            }]
        )
        .is_err());
    assert!(s.get(ClientId(2), 1).is_none());
    assert!(s
        .apply_batch(
            c,
            &[C::SetSolidColor {
                id: 99,
                color: [0; 4]
            }]
        )
        .is_err());
}

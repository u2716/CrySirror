//! Live isolated Canvas resources, usable without a VM, window or GPU.
use crysirror_core::state::ClientId;
use crysirror_scene::{
    canvas::{CanvasCommand as C, CanvasSystem},
    layer::{LayerCommand as L, LayerSpec, LayerSystem, Paint},
};
use serde_json::json;
const CLIENT: ClientId = ClientId(1);
fn layer(id: u32, color: [u8; 4]) -> LayerSpec {
    serde_json::from_value(json!({"id":id,"rect":[0,0,8,8],"paint":{"kind":"solid","color":color}}))
        .unwrap()
}
fn draw(color: [u8; 4]) -> C {
    C::Draw {
        id: 1,
        layer: layer(0, color),
        atop: None,
    }
}
fn isolated(id: u32, parent: Option<u32>) -> C {
    C::PresentIsolated {
        id: 1,
        layer: id,
        order: 0,
        parent,
        position: [3., 4.],
    }
}
fn snapshot(layers: &LayerSystem, id: u32) -> std::sync::Arc<Vec<LayerSpec>> {
    let Paint::Snapshot { layers: inner, .. } = &layers.get(CLIENT, id).unwrap().paint else {
        panic!("not an engine snapshot resource")
    };
    inner.clone()
}
#[test]
fn overlapping_draws_compose_once_before_parent_opacity_and_transform() {
    let mut layers = LayerSystem::default();
    let mut parent = layer(50, [0; 4]);
    parent.rect = [2., 2., 8., 8.];
    parent.opacity = 128;
    parent.transform.scale_x = 2.;
    layers.install(CLIENT, &[parent]).unwrap();
    let mut c = CanvasSystem::default();
    c.apply_batch(
        CLIENT,
        &[
            C::Create {
                id: 1,
                width: 8,
                height: 8,
            },
            C::Clear {
                id: 1,
                color: [0; 4],
            },
            draw([255, 0, 0, 255]),
            draw([0, 0, 255, 255]),
            isolated(10, Some(50)),
        ],
        &mut layers,
    )
    .unwrap();
    let primitives = layers.frame([32, 32]).unwrap();
    let draws = primitives
        .iter()
        .filter(|p| matches!(p.paint, Paint::Snapshot { .. }))
        .collect::<Vec<_>>();
    assert_eq!(draws.len(), 1);
    assert_eq!(draws[0].opacity, 128);
    assert_eq!(&draws[0].quad[0][..2], &[8., 6.]);
    assert_eq!(&draws[0].quad[1][..2], &[24., 6.]);
    let mut inside = LayerSystem::default();
    inside.install(CLIENT, &snapshot(&layers, 10)).unwrap();
    let paints = inside
        .frame([8, 8])
        .unwrap()
        .into_iter()
        .filter(|p| matches!(p.paint,Paint::Solid{color} if color[3]!=0))
        .collect::<Vec<_>>();
    assert_eq!(paints.len(), 2);
    assert!(paints.iter().all(|p| p.opacity == 255));
    //The two opaque source draws have opaque combined coverage; parent128 is
    //applied to ONE draw. Independent parent multiplication would yield191.
    assert_eq!(c.sample_pixel(CLIENT, 1, [1, 1]).unwrap(), [0, 0, 255, 255]);
    assert!(layers.get(ClientId(2), 10).is_none());
}
#[test]
fn source_edits_replace_snapshot_without_mutating_old_capture_or_outer_producer() {
    let mut layers = LayerSystem::default();
    layers.install(CLIENT, &[layer(50, [0; 4])]).unwrap();
    layers
        .apply_batch(CLIENT, &[L::BeginExternalMotion { id: 50, driver: 7 }])
        .unwrap();
    let stamp = layers.motion_status(CLIENT, 50).unwrap().drivers[0].stamp;
    let mut c = CanvasSystem::default();
    c.apply_batch(
        CLIENT,
        &[
            C::Create {
                id: 1,
                width: 8,
                height: 8,
            },
            draw([255, 0, 0, 255]),
            isolated(10, Some(50)),
        ],
        &mut layers,
    )
    .unwrap();
    let before = snapshot(&layers, 10);
    c.apply_batch(
        CLIENT,
        &[C::Clear {
            id: 1,
            color: [0, 255, 0, 255],
        }],
        &mut layers,
    )
    .unwrap();
    let after = snapshot(&layers, 10);
    assert!(!std::sync::Arc::ptr_eq(&before, &after));
    assert_eq!(
        layers.motion_status(CLIENT, 50).unwrap().drivers[0].stamp,
        stamp
    );
    let mut old = LayerSystem::default();
    old.install(CLIENT, &before).unwrap();
    assert!(old.frame([8, 8]).unwrap().iter().any(|p| matches!(
        p.paint,
        Paint::Solid {
            color: [255, 0, 0, 255]
        }
    )));
    let roundtrip: CanvasSystem = serde_json::from_slice(&serde_json::to_vec(&c).unwrap()).unwrap();
    let mut layers = layers.clone();
    let mut c = roundtrip;
    c.apply_batch(
        CLIENT,
        &[C::Clear {
            id: 1,
            color: [9, 8, 7, 255],
        }],
        &mut layers,
    )
    .unwrap();
    assert!(matches!(
        layers.get(CLIENT, 10).unwrap().paint,
        Paint::Snapshot { .. }
    ));
    c.apply_batch(
        CLIENT,
        &[C::Hide { layer: 10 }, C::Destroy { id: 1 }],
        &mut layers,
    )
    .unwrap();
    assert!(layers.get(CLIENT, 10).is_none());
    assert!(layers.get(CLIENT, 50).is_some());
}
#[test]
fn changing_presentation_mode_removes_derived_nodes_and_invalid_scope_rolls_back() {
    let mut layers = LayerSystem::default();
    let mut c = CanvasSystem::default();
    c.apply_batch(
        CLIENT,
        &[
            C::Create {
                id: 1,
                width: 8,
                height: 8,
            },
            draw([1, 2, 3, 255]),
            C::Present {
                id: 1,
                layer: 10,
                order: 0,
                parent: None,
                position: [0.; 2],
            },
        ],
        &mut layers,
    )
    .unwrap();
    let count = layers.frame([8, 8]).unwrap().len();
    assert!(count >= 2);
    c.apply_batch(CLIENT, &[isolated(10, None)], &mut layers)
        .unwrap();
    assert_eq!(layers.frame([16, 16]).unwrap().len(), 1);
    let saved = serde_json::to_value(&c).unwrap();
    let before = snapshot(&layers, 10);
    assert!(
        c.apply_batch(ClientId(2), &[isolated(10, None)], &mut layers)
            .is_err()
    );
    assert!(
        c.apply_batch(
            CLIENT,
            &[
                C::Clear {
                    id: 1,
                    color: [4; 4]
                },
                isolated(10, Some(999))
            ],
            &mut layers
        )
        .is_err()
    );
    assert_eq!(serde_json::to_value(&c).unwrap(), saved);
    assert!(std::sync::Arc::ptr_eq(&before, &snapshot(&layers, 10)));
    c.apply_batch(
        CLIENT,
        &[C::Present {
            id: 1,
            layer: 10,
            order: 0,
            parent: None,
            position: [0.; 2],
        }],
        &mut layers,
    )
    .unwrap();
    assert!(matches!(
        layers.get(CLIENT, 10).unwrap().paint,
        Paint::Solid { .. }
    ));
    assert_eq!(layers.frame([8, 8]).unwrap().len(), count);
}

#[test]
fn nested_isolation_keeps_snapshot_matrix_and_depth_instead_of_inlining_them_away() {
    for matrix in [true, false] {
        let mut layers = LayerSystem::default();
        let mut c = CanvasSystem::default();
        c.apply_batch(
            CLIENT,
            &[
                C::Create {
                    id: 1,
                    width: 8,
                    height: 8,
                },
                draw([255, 0, 0, 255]),
                C::PresentIsolated {
                    id: 1,
                    layer: 10,
                    order: 0,
                    parent: None,
                    position: [0.; 2],
                },
            ],
            &mut layers,
        )
        .unwrap();
        let mut original = layers.get(CLIENT, 10).unwrap().clone();
        original.id = 0;
        //A manually reused unclipped snapshot meets the old inline shape.
        original.clip = None;
        if matrix {
            original.transform.local_matrix = Some([
                1., 0., 0., 0., 0.5, 1., 0., 0., 0., 0., 1., 0., 1., 2., 0., 1.,
            ]);
        } else {
            original.transform.z = 7.;
        }
        let expected = original.transform.clone();
        c.apply_batch(
            CLIENT,
            &[
                C::Create {
                    id: 2,
                    width: 8,
                    height: 8,
                },
                C::Clear {
                    id: 2,
                    color: [0; 4],
                },
                C::Draw {
                    id: 2,
                    layer: original,
                    atop: None,
                },
                C::PresentIsolated {
                    id: 2,
                    layer: 11,
                    order: 0,
                    parent: None,
                    position: [0.; 2],
                },
            ],
            &mut layers,
        )
        .unwrap();
        let captured = snapshot(&layers, 11);
        let nested = captured
            .iter()
            .find(|s| matches!(s.paint, Paint::Snapshot { .. }))
            .expect("nonidentity snapshot was incorrectly inlined");
        assert_eq!(nested.transform, expected);
        if matrix {
            let mut inner = LayerSystem::default();
            inner.install(CLIENT, &captured).unwrap();
            let p = inner
                .frame([8, 8])
                .unwrap()
                .into_iter()
                .find(|p| matches!(p.paint, Paint::Snapshot { .. }))
                .unwrap();
            assert_eq!(&p.quad[0][..2], &[1., 2.]);
        }
    }
}

#[test]
fn isolated_root_keeps_source_clip_for_separately_attached_public_children() {
    let mut layers = LayerSystem::default();
    let mut c = CanvasSystem::default();
    c.apply_batch(
        CLIENT,
        &[
            C::Create {
                id: 1,
                width: 8,
                height: 8,
            },
            C::Present {
                id: 1,
                layer: 10,
                order: 0,
                parent: None,
                position: [0.; 2],
            },
        ],
        &mut layers,
    )
    .unwrap();
    let mut child = layer(20, [9, 8, 7, 255]);
    child.parent = Some(10);
    child.rect = [10., 0., 2., 2.];
    layers
        .apply_batch(CLIENT, &[L::Put { layer: child }])
        .unwrap();
    let observed = |layers: &LayerSystem| {
        layers
            .frame([32, 32])
            .unwrap()
            .into_iter()
            .filter(|p| {
                matches!(
                    p.paint,
                    Paint::Solid {
                        color: [9, 8, 7, 255]
                    }
                )
            })
            .map(|p| (p.quad, p.clip, p.opacity))
            .collect::<Vec<_>>()
    };
    let before = observed(&layers);
    c.apply_batch(
        CLIENT,
        &[C::PresentIsolated {
            id: 1,
            layer: 10,
            order: 0,
            parent: None,
            position: [0.; 2],
        }],
        &mut layers,
    )
    .unwrap();
    assert_eq!(layers.get(CLIENT, 10).unwrap().clip, Some([0., 0., 8., 8.]));
    assert_eq!(observed(&layers), before);
}

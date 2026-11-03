use crysirror_core::state::ClientId;
use crysirror_scene::layer::{
    ExternalMotionPose, ExternalMotionStamp, LayerCommand as C, LayerSystem,
};
use serde_json::json;
const A: ClientId = ClientId(1);
fn put(layers: &mut LayerSystem, client: ClientId) {
    layers.apply_batch(client,&serde_json::from_value::<Vec<C>>(json!([{"op":"put","layer":{"id":1,"rect":[0,0,20,30],"paint":{"kind":"solid","color":[1,2,3,255]}}}])).unwrap()).unwrap();
}
fn begin(layers: &mut LayerSystem, client: ClientId) -> ExternalMotionStamp {
    layers
        .apply_batch(client, &[C::BeginExternalMotion { id: 1, driver: 7 }])
        .unwrap();
    layers.motion_status(client, 1).unwrap().drivers[0].stamp
}
fn next(mut stamp: ExternalMotionStamp) -> ExternalMotionStamp {
    stamp.revision += 1;
    stamp
}
fn commit(stamp: ExternalMotionStamp, complete: bool, rect: [f32; 4]) -> C {
    C::CommitExternalMotion {
        id: 1,
        driver: 7,
        stamp,
        complete,
        pose: Some(ExternalMotionPose {
            rect: Some(rect),
            opacity: Some(99),
            transform: None,
        }),
    }
}
#[test]
fn real_producer_and_scalar_completion_are_combined_and_skip_is_only_a_request() {
    let mut layers = LayerSystem::default();
    put(&mut layers, A);
    let stamp = begin(&mut layers, A);
    assert!(!layers.motion_status(A, 1).unwrap().complete);
    layers
        .apply_batch(
            A,
            &[C::RequestExternalMotionSkip {
                id: 1,
                driver: 7,
                generation: stamp.generation,
            }],
        )
        .unwrap();
    let status = layers.motion_status(A, 1).unwrap();
    assert!(!status.complete && status.drivers[0].skip_requested);
    layers.tick(100000);
    assert!(!layers.motion_status(A, 1).unwrap().complete);
    layers.apply_batch(A,&serde_json::from_value::<Vec<C>>(json!([{"op":"begin_motion","id":1,"track":{"property":"z","to":10,"duration":10,"external":true}}])).unwrap()).unwrap();
    layers
        .apply_batch(A, &[commit(next(stamp), true, [5., 6., 20., 30.])])
        .unwrap();
    let layer = layers.get(A, 1).unwrap();
    assert_eq!(layer.rect, [5., 6., 20., 30.]);
    assert_eq!(layer.opacity, 99);
    assert!(!layers.motion_status(A, 1).unwrap().complete);
    layers
        .apply_batch(
            A,
            &serde_json::from_value::<Vec<C>>(json!([{"op":"end_motion","id":1,"finish":true}]))
                .unwrap(),
        )
        .unwrap();
    assert!(layers.motion_status(A, 1).unwrap().complete);
    assert!(layers
        .apply_batch(A, &[commit(next(next(stamp)), false, [9., 9., 20., 30.])])
        .is_err());
    assert_eq!(layers.get(A, 1).unwrap().rect, [5., 6., 20., 30.]);
}
#[test]
fn stale_foreign_and_invalid_samples_roll_back_pose_and_the_real_revision() {
    let mut layers = LayerSystem::default();
    put(&mut layers, A);
    put(&mut layers, ClientId(2));
    let stamp = begin(&mut layers, A);
    let foreign = begin(&mut layers, ClientId(2));
    assert!(layers
        .apply_batch(A, &[commit(next(stamp), false, [5., 6., 0., 30.])])
        .is_err());
    assert_eq!(layers.motion_status(A, 1).unwrap().drivers[0].stamp, stamp);
    assert_eq!(layers.get(A, 1).unwrap().opacity, 255);
    assert!(layers
        .apply_batch(
            ClientId(2),
            &[commit(next(stamp), true, [9., 9., 20., 30.])]
        )
        .is_err());
    assert_eq!(
        layers.motion_status(ClientId(2), 1).unwrap().drivers[0].stamp,
        foreign
    );
    layers
        .apply_batch(A, &[commit(next(stamp), false, [5., 6., 20., 30.])])
        .unwrap();
    assert!(layers
        .apply_batch(
            A,
            &[
                C::SetVisible {
                    id: 1,
                    visible: false
                },
                commit(next(stamp), true, [9., 9., 20., 30.])
            ]
        )
        .is_err());
    let layer = layers.get(A, 1).unwrap();
    assert!(layer.visible);
    assert_eq!(layer.rect, [5., 6., 20., 30.]);
    let status = layers.motion_status(A, 1).unwrap();
    assert_eq!(status.drivers[0].stamp, next(stamp));
    assert!(!status.complete);
    assert!(layers.motion_status(ClientId(999), 1).is_err());
    layers.validate().unwrap();
}
#[test]
fn producer_retirement_reuse_and_snapshot_boundaries_never_accept_old_jobs() {
    let mut layers = LayerSystem::default();
    put(&mut layers, A);
    let old = begin(&mut layers, A);
    assert!(serde_json::to_vec(&layers).is_err());
    layers.apply_batch(A, &[C::Remove { id: 1 }]).unwrap();
    put(&mut layers, A);
    let new = begin(&mut layers, A);
    assert_ne!(old.generation, new.generation);
    assert!(layers
        .apply_batch(A, &[commit(next(old), true, [4., 4., 20., 30.])])
        .is_err());
    layers
        .apply_batch(A, &[commit(next(new), true, [4., 4., 20., 30.])])
        .unwrap();
    assert!(serde_json::to_vec(&layers).is_err());
    layers
        .apply_batch(
            A,
            &[C::RemoveExternalMotion {
                id: 1,
                driver: 7,
                generation: new.generation,
            }],
        )
        .unwrap();
    let bytes = serde_json::to_vec(&layers).unwrap();
    let restored: LayerSystem = serde_json::from_slice(&bytes).unwrap();
    restored.validate().unwrap();
    let mut poisoned: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    poisoned["motions"]["external"] = json!({"1":{}});
    assert!(serde_json::from_value::<LayerSystem>(poisoned).is_err());
    assert!(layers.motion_status(A, 1).unwrap().complete);
}
#[test]
fn independent_external_samples_and_rect_updates_preserve_unrelated_live_tracks() {
    let mut layers = LayerSystem::default();
    put(&mut layers, A);
    let stamp = begin(&mut layers, A);
    layers.apply_batch(A,&serde_json::from_value::<Vec<C>>(json!([{"op":"begin_motion","id":1,"track":{"property":"rotation_z","to":1,"duration":100,"external":true}}])).unwrap()).unwrap();
    layers
        .apply_batch(
            A,
            &[C::SetRect {
                id: 1,
                rect: [3., 4., 20., 30.],
            }],
        )
        .unwrap();
    let status = layers.motion_status(A, 1).unwrap();
    assert_eq!(status.drivers[0].stamp, stamp);
    assert_eq!(status.tracks.len(), 1);
    layers
        .apply_batch(
            A,
            &[C::EndMotion {
                id: 1,
                property: None,
                finish: true,
            }],
        )
        .unwrap();
    assert!(!layers.motion_status(A, 1).unwrap().complete);
    layers
        .apply_batch(A, &[commit(next(stamp), true, [3., 4., 20., 30.])])
        .unwrap();
    assert!(layers.motion_status(A, 1).unwrap().complete);
    assert_eq!(layers.get(A, 1).unwrap().transform.rotate, 1.);
    assert!(layers
        .apply_batch(
            A,
            &[C::SetRect {
                id: 1,
                rect: [3., 4., 0., 30.]
            }]
        )
        .is_err());
    assert_eq!(layers.get(A, 1).unwrap().rect, [3., 4., 20., 30.]);
}

#[test]
fn asynchronous_revision_stamps_preserve_large_u64_values_without_js_rounding() {
    let stamp = ExternalMotionStamp {
        generation: u64::MAX,
        revision: 9007199254740993,
    };
    let encoded = serde_json::to_value(stamp).unwrap();
    assert_eq!(
        encoded,
        json!({"generation":"18446744073709551615","revision":"9007199254740993"})
    );
    assert_eq!(
        serde_json::from_value::<ExternalMotionStamp>(encoded).unwrap(),
        stamp
    );
    assert!(serde_json::from_value::<ExternalMotionStamp>(
        json!({"generation":18446744073709551615u64,"revision":0})
    )
    .is_err());
}

use crysirror_core::state::ClientId;
use crysirror_scene::{
    layer::{LayerCommand as C, LayerSystem, SpriteTransform},
    projection,
};
use serde_json::json;
const A: ClientId = ClientId(1);
fn shear() -> [f32; 16] {
    [
        1., 0., 0., 0., 2., 1., 0., 0., 0., 0., 1., 0., 5., 7., 0., 1.,
    ]
}
fn put(layers: &mut LayerSystem, id: u32, parent: Option<u32>, rect: [f32; 4]) {
    let layer = serde_json::from_value(
        json!({"id":id,"parent":parent,"rect":rect,"paint":{"kind":"solid","color":[255,0,0,255]}}),
    )
    .unwrap();
    layers.apply_batch(A, &[C::Put { layer }]).unwrap();
}
#[test]
fn affine_shear_is_projected_after_local_scale_and_keeps_true_parent_geometry() {
    let transform = SpriteTransform {
        local_matrix: Some(shear()),
        scale_x: 2.,
        ..Default::default()
    };
    transform.validate().unwrap();
    assert!(transform.matrix_is_2d());
    let geometry =
        projection::sprite_quad_geometry_rect(&transform, 10., 20., 0., 0., 2., 3., 64., 64.)
            .unwrap();
    let points = geometry.projected.map(|p| [p.x, p.y, p.depth, p.clip_w]);
    assert_eq!(
        points,
        [
            [15., 27., 0., 1.],
            [19., 27., 0., 1.],
            [25., 30., 0., 1.],
            [21., 30., 0., 1.]
        ]
    );
    let mut layers = LayerSystem::default();
    put(&mut layers, 1, None, [10., 20., 1., 1.]);
    put(&mut layers, 2, Some(1), [3., 4., 2., 3.]);
    layers
        .apply_batch(A, &[C::SetTransform { id: 1, transform }])
        .unwrap();
    let frame = layers.frame([64, 64]).unwrap();
    let child = frame.iter().find(|p| p.id == 2).unwrap();
    assert_eq!(
        child.quad.map(|p| [p[0], p[1]]),
        [[29., 31.], [33., 31.], [39., 34.], [35., 34.]]
    );
    let restored: LayerSystem =
        serde_json::from_slice(&serde_json::to_vec(&layers).unwrap()).unwrap();
    restored.validate().unwrap();
    let restored = restored.frame([64, 64]).unwrap();
    assert_eq!(
        restored.iter().find(|p| p.id == 2).unwrap().quad,
        child.quad
    );
}
#[test]
fn full_affine_matrix_is_used_by_sprite_and_model_projection_with_world_depth() {
    //Known90-degree rotation aboutX, column-major. No Euler decomposition.
    let matrix = [
        1., 0., 0., 0., 0., 0., 1., 0., 0., -1., 0., 0., 0., 0., 5., 1.,
    ];
    let transform = SpriteTransform {
        local_matrix: Some(matrix),
        z: 40.,
        ..Default::default()
    };
    transform.validate().unwrap();
    assert!(!transform.matrix_is_2d());
    let quad =
        projection::sprite_quad_geometry_rect(&transform, 10., 20., 0., 0., 2., 3., 64., 64.)
            .unwrap();
    assert_eq!(quad.world[0], [10., 20., 45.]);
    assert_eq!(quad.world[2], [12., 20., 48.]);
    let point =
        projection::project_model_point(&transform, [2., 3., 4.], 10., 20., 64., 64.).unwrap();
    assert_eq!([point.x, point.y], [12., 16.]);
    assert!((point.depth - (0.5 - 48. / 100000.)).abs() < 1e-6);
    let mut layers = LayerSystem::default();
    put(&mut layers, 1, None, [0., 0., 20., 30.]);
    put(&mut layers, 2, Some(1), [0., 0., 2., 3.]);
    layers
        .apply_batch(A, &[C::SetTransform { id: 1, transform }])
        .unwrap();
    assert!(layers
        .frame([64, 64])
        .unwrap_err()
        .to_string()
        .contains("world hierarchy"));
}
#[test]
fn projective_singular_nonfinite_and_billboard_matrices_fail_before_publication() {
    let mut layers = LayerSystem::default();
    put(&mut layers, 1, None, [0., 0., 20., 30.]);
    let before = serde_json::to_value(&layers).unwrap();
    for kind in 0..4 {
        let mut transform = SpriteTransform {
            local_matrix: Some(shear()),
            ..Default::default()
        };
        match kind {
            0 => transform.local_matrix.as_mut().unwrap()[3] = 0.25,
            1 => transform.local_matrix.as_mut().unwrap()[0] = 0.,
            2 => transform.local_matrix.as_mut().unwrap()[0] = f32::NAN,
            _ => transform.billboard = true,
        }
        assert!(layers
            .apply_batch(
                A,
                &[
                    C::SetVisible {
                        id: 1,
                        visible: false
                    },
                    C::SetTransform { id: 1, transform }
                ]
            )
            .is_err());
        assert_eq!(serde_json::to_value(&layers).unwrap(), before);
    }
}

// SPDX-License-Identifier: MPL-2.0
use super::*;
use crysirror_scene::components::{EmitterSpec, Falloff};
use crysirror_scene::model::Camera;
use crysirror_scene::view::{
    GeometryLod, ParticleEmitter, PbrLight, PbrProbe, PbrVolume, RayTracedGi, ScreenSpaceReflections,
    VirtualShadows,
};
use serde_json::json;

fn from<T: serde::de::DeserializeOwned>(v: serde_json::Value) -> T {
    serde_json::from_value(v).unwrap()
}
const I: [[f32; 4]; 4] = [[1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 0.], [0., 0., 0., 1.]];

fn light(key: u64, x: f32, shape: LightShape, shadows: bool) -> PbrLight {
    PbrLight {
        key,
        position: [x, 0., 0.],
        direction: [0., 0., -1.],
        right: if matches!(shape, LightShape::Rect { .. }) { [1., 0., 0.] } else { [0.; 3] },
        up: if matches!(shape, LightShape::Rect { .. }) { [0., 0.5, 0.] } else { [0.; 3] },
        intensity: [100.; 3],
        range: 5.,
        shape,
        falloff: Falloff::InverseSquare,
        shadows,
        source_radius: 0.,
    }
}

fn heavy() -> PbrScene {
    let mut environment = Environment {
        hdr: Some("sky.hdr".into()),
        fog: Some(from(json!({}))),
        auto_exposure: Some(from(json!({}))),
        reflections: Some(ScreenSpaceReflections {
            ray_traced: true,
            ..from(json!({}))
        }),
        ray_traced_gi: Some(RayTracedGi {
            intensity: 2.,
            ..from(json!({}))
        }),
        geometry_lod: Some(GeometryLod { pixel_error: 1., gpu: true }),
        virtual_shadows: Some(VirtualShadows { texel_size: 0.02, levels: 8 }),
        ..Environment::default()
    };
    environment.shadow_distance = 200.;
    let emitter: EmitterSpec = from(json!({"rate": 400.0, "burst": 100, "max": 20000}));
    PbrScene {
        vfx: None,
        vfx_epoch: 0,
        vfx_visible: None,
        time: 0.,
        emitters: (0..4)
            .map(|k| ParticleEmitter { key: k, matrix: I, spec: emitter.clone() })
            .collect(),
        camera: Camera {
            eye: [0.; 3],
            target: [0., 0., -1.],
            up: [0., 1., 0.],
            fov_y_degrees: 60.,
            near: 0.1,
            far: 500.,
        },
        environment,
        environment_client: 1,
        instances: vec![],
        lights: (0..40)
            .map(|k| {
                let shape = match k % 3 {
                    0 => LightShape::Point,
                    1 => LightShape::Spot { inner_degrees: 20., outer_degrees: 35. },
                    _ => LightShape::Rect { size: [2., 1.] },
                };
                light(k, k as f32, shape, true)
            })
            .collect(),
        probes: (0..6)
            .map(|k| PbrProbe {
                key: k,
                position: [10. - k as f32, 0., 0.],
                extent: [2.; 3],
                intensity: 1.,
                blend_distance: 0.5,
                revision: 0,
                source: None,
                parallax: true,
            })
            .collect(),
        volumes: vec![PbrVolume {
            key: 9,
            center: [0.; 3],
            spec: from(json!({"extent": [10.0, 5.0, 10.0], "counts": [16, 8, 16], "probes_per_frame": 8, "idle_probes_per_frame": 4, "ray_traced": true})),
        }],
    }
}

#[test]
fn desktop_is_identity() {
    let s = heavy();
    let r = reduce(&s, Tier::Desktop);
    assert_eq!(r.scene, s);
    assert!(r.decisions.iter().all(|d| d.outcome == Outcome::Native));
    assert!(r.decision(Feature::RayTracedGi).is_some());
}

#[test]
fn reduced_environments_stay_valid() {
    for tier in [Tier::SwitchDocked, Tier::SwitchHandheld, Tier::Vita] {
        let r = reduce(&heavy(), tier);
        r.scene.environment.validate().unwrap();
        for v in &r.scene.volumes {
            v.spec.validate().unwrap();
            assert!(v.spec.counts.iter().product::<u32>() <= r.budget.max_volume_probes);
            assert!(!v.spec.ray_traced);
        }
        for e in &r.scene.emitters {
            e.spec.validate().unwrap();
        }
        let e = &r.scene.environment;
        assert!(e.ray_traced_gi.is_none() && e.virtual_shadows.is_none());
        assert!(e.shadow_distance <= r.budget.max_shadow_distance);
        assert!(r.scene.lights.len() <= r.budget.max_local_lights as usize);
        assert!(r.scene.probes.len() <= r.budget.reflection_probes as usize);
        let total: u32 = r.scene.emitters.iter().map(|e| e.spec.max).sum();
        assert!(total <= r.budget.max_particles);
        // Every reported feature is explicit; no non-native feature is silent.
        assert!(r.decisions.windows(2).all(|w| w[0].feature < w[1].feature));
        serde_json::to_string(&r.decisions).unwrap();
    }
}

#[test]
fn switch_keeps_screen_space_paths() {
    let r = reduce(&heavy(), Tier::SwitchDocked);
    let e = &r.scene.environment;
    let ssr = e.reflections.as_ref().unwrap();
    assert!(!ssr.ray_traced && ssr.steps == 24 && ssr.ray_samples == 1);
    assert!(ssr.thickness >= ssr.max_distance / 24.);
    assert!(e.temporal_aa && e.ambient_occlusion > 0.);
    assert!(e.geometry_lod.as_ref().unwrap().gpu);
    assert!(matches!(
        r.decision(Feature::RayTracedGi),
        Some(Outcome::Substituted { by: Technique::IrradianceVolumeRaster, .. })
    ));
    // RT GI intensity is carried by the raster volume.
    assert_eq!(r.scene.volumes[0].spec.intensity, 2.);
    assert!(r.scene.volumes[0].spec.visibility);
    assert!(r.scene.lights.iter().any(|l| matches!(l.shape, LightShape::Rect { .. })));
}

#[test]
fn vita_substitutes_compute_and_screen_space() {
    let r = reduce(&heavy(), Tier::Vita);
    let e = &r.scene.environment;
    assert!(e.reflections.is_none() && !e.temporal_aa && e.ambient_occlusion == 0.);
    let lod = e.geometry_lod.as_ref().unwrap();
    assert!(!lod.gpu && lod.pixel_error >= 2.);
    assert!(matches!(
        r.decision(Feature::ScreenSpaceReflections),
        Some(Outcome::Substituted { by: Technique::ReflectionProbes, .. })
    ));
    assert!(matches!(
        r.decision(Feature::TemporalAa),
        Some(Outcome::Substituted { by: Technique::Msaa, .. })
    ));
    assert!(matches!(
        r.decision(Feature::AutoExposure),
        Some(Outcome::Substituted { by: Technique::FragmentLuminanceReduction, .. })
    ));
    assert!(matches!(r.decision(Feature::LocalShadows), Some(Outcome::Disabled { .. })));
    assert!(r.scene.lights.iter().all(|l| !l.shadows));
    // Rect lights keep the shared closest-point Lambertian evaluation.
    let rect = r.scene.lights.iter().find(|l| l.key == 2).unwrap();
    let authored = &heavy().lights[2];
    assert_eq!((rect.shape, rect.right, rect.up), (authored.shape, authored.right, authored.up));
    // Nearest probes are kept.
    let mut keys: Vec<_> = r.scene.probes.iter().map(|p| p.key).collect();
    keys.sort();
    assert_eq!(keys, vec![4, 5]);
    // Nearest lights first.
    assert!(r.scene.lights.iter().all(|l| l.key < 16));
    assert!(!r.scene.volumes[0].spec.visibility);
    assert_eq!(r.scene.volumes[0].spec.probes_per_frame, 1);
}

#[test]
fn deterministic() {
    for tier in [Tier::SwitchHandheld, Tier::Vita] {
        assert_eq!(reduce(&heavy(), tier), reduce(&heavy(), tier));
    }
}

#[test]
fn console_tiers() {
    assert_eq!(Tier::for_console(Platform::PsVita, true), Tier::Vita);
    assert_eq!(Tier::for_console(Platform::Switch, false), Tier::SwitchHandheld);
    // VN composition is never reduced below the output resolution.
    for t in [Tier::SwitchDocked, Tier::SwitchHandheld, Tier::Vita] {
        let b = t.budget();
        let (o, s) = (b.output.unwrap(), b.scene_3d.unwrap());
        assert!(s[0] <= o[0] && s[1] <= o[1]);
    }
}

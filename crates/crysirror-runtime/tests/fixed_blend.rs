//! Engine-owned normalized immutable jobs, without a VM or device.
use crysirror_core::{image_blend::*, image_processing::*, service::HostServices, state::ClientId};
use crysirror_runtime::Runtime;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
fn runtime(root: &std::path::Path) -> Runtime {
    let project=serde_json::from_value(serde_json::json!({"schema":2,"name":"fixed blend","content_revision":"1","canvas":[2,1],"assets":"assets","clients":[{"name":"a","adapter":"crysirror-script-v1","game":"a"},{"name":"b","adapter":"crysirror-script-v1","game":"b"}]})).unwrap();
    Runtime::with_asset_root(project, root, &root.join("state"), Default::default()).unwrap()
}
fn ready(job: &mut ImageJob) -> Arc<dyn ImageResource> {
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        match job.poll().unwrap() {
            ImageProcessState::Ready(image) => return image,
            ImageProcessState::Failed(e) => panic!("{e}"),
            ImageProcessState::Pending => {
                assert!(Instant::now() < until);
                std::thread::yield_now();
            }
        }
    }
}
fn operation(base: ImageInput, source: ImageInput, state: RgbaBlendState) -> ImageInput {
    ImageInput::Composite {
        base: Box::new(base),
        layers: vec![ImageCompositeLayer {
            input: Box::new(source),
            source: [0, 0, 2, 1],
            destination: [0, 0],
            clip: Some([1, 0, 1, 1]),
            opacity: 255,
            math: ImageCompositeMath::Normalized(state),
        }],
    }
}
#[test]
fn real_worker_keeps_sources_immutable_and_publishes_scoped_clipped_normalized_pixels() {
    let t = tempfile::tempdir().unwrap();
    let mut runtime = runtime(t.path());
    let images = runtime.host.image_metadata(ClientId(1)).unwrap();
    let mut job = images
        .begin_process(
            ImageInput::Solid {
                size: [2, 1],
                color: [10, 20, 30, 61],
            },
            b"[]".as_slice().into(),
        )
        .unwrap();
    let original = ready(&mut job);
    let mut state = RgbaBlendState::default();
    state.color = RgbaBlendComponent {
        source: RgbaBlendFactor::SourceAlpha,
        destination: RgbaBlendFactor::OneMinusSourceAlpha,
        equation: RgbaBlendEquation::Add,
    };
    state.write_mask[3] = false;
    let mut job = images
        .begin_process(
            operation(
                ImageInput::Resource(original.clone()),
                ImageInput::Solid {
                    size: [2, 1],
                    color: [111, 122, 133, 202],
                },
                state,
            ),
            b"[]".as_slice().into(),
        )
        .unwrap();
    let result = ready(&mut job);
    assert_eq!(result.client(), ClientId(1));
    assert_eq!(result.size(), [2, 1]);
    assert_eq!(result.rgba().as_ref(), [10, 20, 30, 61, 90, 101, 112, 61]);
    assert_eq!(original.rgba().as_ref(), [10, 20, 30, 61, 10, 20, 30, 61]);
    assert!(Arc::ptr_eq(&ready(&mut job), &result));
    let foreign = runtime.host.image_metadata(ClientId(2)).unwrap();
    assert!(foreign
        .begin_process(
            operation(
                ImageInput::Resource(original),
                ImageInput::Solid {
                    size: [2, 1],
                    color: [0; 4]
                },
                state
            ),
            b"[]".as_slice().into()
        )
        .is_err());
    assert!(runtime
        .host
        .publish_image_resource(ClientId(2), 3, result.clone())
        .is_err());
    runtime
        .host
        .publish_image_resource(ClientId(1), 3, result)
        .unwrap();
}
#[test]
fn invalid_normalized_state_and_duplicate_opacity_fail_before_worker_admission() {
    let t = tempfile::tempdir().unwrap();
    let mut runtime = runtime(t.path());
    let images = runtime.host.image_metadata(ClientId(1)).unwrap();
    let solid = || ImageInput::Solid {
        size: [2, 1],
        color: [7; 4],
    };
    for state in [
        RgbaBlendState {
            constant: [f32::INFINITY; 4],
            ..Default::default()
        },
        RgbaBlendState {
            source_alpha_range: [0.8, 0.2],
            ..Default::default()
        },
    ] {
        assert!(images
            .begin_process(operation(solid(), solid(), state), b"[]".as_slice().into())
            .is_err());
    }
    let mut input = operation(solid(), solid(), Default::default());
    let ImageInput::Composite { layers, .. } = &mut input else {
        unreachable!()
    };
    layers[0].opacity = 128;
    assert!(images
        .begin_process(input, b"[]".as_slice().into())
        .is_err());
    let mut job = images
        .begin_process(solid(), b"[]".as_slice().into())
        .unwrap();
    assert_eq!(ready(&mut job).rgba().as_ref(), [7; 8]);
}

#[test]
fn integer_arithmetic_survives_async_job_preparation_and_uses_separately_truncated_products() {
    let t = tempfile::tempdir().unwrap();
    let mut runtime = runtime(t.path());
    let images = runtime.host.image_metadata(ClientId(1)).unwrap();
    let state = RgbaBlendState {
        arithmetic: RgbaBlendArithmetic::Byte255 {
            source: RgbaByteRounding::Floor,
            destination: RgbaByteRounding::Floor,
        },
        color: RgbaBlendComponent {
            source: RgbaBlendFactor::SourceAlpha,
            destination: RgbaBlendFactor::OneMinusSourceAlpha,
            equation: RgbaBlendEquation::Add,
        },
        write_mask: [true, true, true, false],
        ..Default::default()
    };
    let mut job = images
        .begin_process(
            operation(
                ImageInput::Solid {
                    size: [2, 1],
                    color: [10, 20, 30, 61],
                },
                ImageInput::Solid {
                    size: [2, 1],
                    color: [111, 122, 133, 202],
                },
                state,
            ),
            b"[]".as_slice().into(),
        )
        .unwrap();
    assert_eq!(
        ready(&mut job).rgba().as_ref(),
        [10, 20, 30, 61, 89, 100, 111, 61]
    );
}

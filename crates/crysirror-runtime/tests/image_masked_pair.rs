//! Three-input service graphs without a native VM, window or audio device.
use crysirror_core::{
    image_processing::{ImageInput, ImageProcessState, ImageResource},
    service::HostServices,
    state::ClientId,
};
use crysirror_runtime::Runtime;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
fn runtime() -> (tempfile::TempDir, Runtime) {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir(d.path().join("assets")).unwrap();
    let p = d.path().join("project.json");
    std::fs::write(&p,br#"{"schema":2,"name":"three inputs","content_revision":"1","canvas":[4,2],"assets":"assets","clients":[{"name":"one","adapter":"crysirror-script-v1","game":"one"},{"name":"two","adapter":"crysirror-script-v1","game":"two"}]}"#).unwrap();
    let rt = Runtime::load(&p, &d.path().join("state")).unwrap();
    (d, rt)
}
fn ready(mut job: crysirror_core::image_processing::ImageJob) -> Arc<dyn ImageResource> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match job.poll().unwrap() {
            ImageProcessState::Ready(r) => {
                assert!(matches!(job.poll().unwrap(), ImageProcessState::Ready(_)));
                return r;
            }
            ImageProcessState::Failed(e) => panic!("{e}"),
            ImageProcessState::Pending => {
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            }
        }
    }
}
fn effect() -> Arc<[u8]> {
    br#"[{"kind":"masked_pair_blend","spec":{"sources":[[0,0,1,1],[0,0,1,1]],"destination":[0,0],"weights256":[64,192]}}]"#.as_slice().into()
}
#[test]
fn ternary_worker_uses_three_frozen_resources_and_rejects_context_or_scope_mismatches() {
    let (_d, mut rt) = runtime();
    let images = rt.host.image_metadata(ClientId(1)).unwrap();
    let prepare = |color| {
        ready(
            images
                .begin_process(
                    ImageInput::Solid {
                        size: [1, 1],
                        color,
                    },
                    b"[]".as_slice().into(),
                )
                .unwrap(),
        )
    };
    let base = prepare([100, 100, 100, 91]);
    let first = prepare([200, 200, 200, 128]);
    let second = prepare([20, 20, 20, 255]);
    let input = || ImageInput::Ternary {
        destination: Box::new(ImageInput::Resource(base.clone())),
        first: Box::new(ImageInput::Resource(first.clone())),
        second: Box::new(ImageInput::Resource(second.clone())),
    };
    let result = ready(images.begin_process(input(), effect()).unwrap());
    assert_eq!(result.rgba().as_ref(), [52, 52, 52, 91]);
    assert_eq!(base.rgba().as_ref(), [100, 100, 100, 91]);
    assert_eq!(first.rgba().as_ref(), [200, 200, 200, 128]);
    assert_eq!(second.rgba().as_ref(), [20, 20, 20, 255]);
    assert_eq!(rt.host.now_ms(), 0);
    assert!(images
        .begin_process(input(), b"[]".as_slice().into())
        .is_err());
    assert!(images
        .begin_process(ImageInput::Resource(base.clone()), effect())
        .is_err());
    assert!(images
        .begin_process(
            ImageInput::Raster {
                destination: Box::new(ImageInput::Resource(base.clone())),
                source: Box::new(ImageInput::Resource(first.clone()))
            },
            effect()
        )
        .is_err());
    let foreign = ready(
        rt.host
            .image_metadata(ClientId(2))
            .unwrap()
            .begin_process(
                ImageInput::Solid {
                    size: [1, 1],
                    color: [0; 4],
                },
                b"[]".as_slice().into(),
            )
            .unwrap(),
    );
    for first_foreign in [true, false] {
        assert!(images
            .begin_process(
                ImageInput::Ternary {
                    destination: Box::new(ImageInput::Resource(base.clone())),
                    first: Box::new(ImageInput::Resource(if first_foreign {
                        foreign.clone()
                    } else {
                        first.clone()
                    })),
                    second: Box::new(ImageInput::Resource(if first_foreign {
                        second.clone()
                    } else {
                        foreign.clone()
                    }))
                },
                effect()
            )
            .is_err());
    }
}
#[test]
fn ternary_third_input_and_output_are_admitted_before_large_allocations() {
    let (_d, mut rt) = runtime();
    let images = rt.host.image_metadata(ClientId(1)).unwrap();
    let solid = |size| {
        Box::new(ImageInput::Solid {
            size,
            color: [0; 4],
        })
    };
    let bad = ImageInput::Ternary {
        destination: solid([1, 1]),
        first: solid([4096, 4096]),
        second: solid([4096, 4096]),
    };
    assert!(images
        .begin_process(bad, effect())
        .err()
        .unwrap()
        .to_string()
        .contains("composition byte budget"));
    let bad = ImageInput::Ternary {
        destination: solid([4097, 1024]),
        first: solid([1, 1]),
        second: solid([1, 1]),
    };
    assert!(images
        .begin_process(bad, effect())
        .err()
        .unwrap()
        .to_string()
        .contains("effect pixel budget"));
    let nested = ImageInput::Ternary {
        destination: solid([1, 1]),
        first: solid([1, 1]),
        second: solid([1, 1]),
    };
    let bad = ImageInput::Composite {
        base: Box::new(nested),
        layers: vec![],
    };
    assert!(images.begin_process(bad, b"[]".as_slice().into()).is_err());
}

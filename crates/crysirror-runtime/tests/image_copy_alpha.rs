//! Shared channel composition, independent of any VM, window or audio device.
use crysirror_core::{
    image_processing::{
        ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageProcessState, ImageResource,
    },
    service::HostServices,
    state::ClientId,
};
use crysirror_runtime::Runtime;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
#[derive(Debug)]
struct Coverage(ClientId);
impl ImageResource for Coverage {
    fn client(&self) -> ClientId {
        self.0
    }
    fn size(&self) -> [u32; 2] {
        [2, 2]
    }
    fn rgba(&self) -> Arc<[u8]> {
        vec![
            201, 202, 203, 0, 204, 205, 206, 64, 207, 208, 209, 128, 210, 211, 212, 255,
        ]
        .into()
    }
    fn opaque(&self) -> bool {
        false
    }
}
fn runtime(root: &std::path::Path) -> Runtime {
    let project = serde_json::from_value(serde_json::json!({
        "schema":2,"name":"shared alpha copy","content_revision":"1","canvas":[3,2],"assets":"assets",
        "clients":[{"name":"one","adapter":"crysirror-script-v1","game":"test"},
                   {"name":"two","adapter":"crysirror-script-v1","game":"test"}]
    })).unwrap();
    Runtime::with_asset_root(project, root, &root.join("state"), Default::default()).unwrap()
}
fn graph(source: Arc<dyn ImageResource>) -> ImageInput {
    ImageInput::Composite {
        base: Box::new(ImageInput::Solid {
            size: [3, 2],
            color: [9, 19, 29, 61],
        }),
        layers: vec![ImageCompositeLayer {
            input: Box::new(ImageInput::Resource(source)),
            source: [-1, 0, 3, 2],
            destination: [0, 0],
            clip: Some([1, 0, 2, 2]),
            opacity: 255,
            math: ImageCompositeMath::CopyAlpha,
        }],
    }
}
#[test]
fn alpha_copy_worker_preserves_rgb_and_destination_clip_source_phase_and_client_scope() {
    let root = tempfile::tempdir().unwrap();
    let mut runtime = runtime(root.path());
    let images = runtime.host.image_metadata(ClientId(1)).unwrap();
    assert!(images
        .begin_process(
            graph(Arc::new(Coverage(ClientId(2)))),
            b"[]".as_slice().into()
        )
        .is_err());
    let mut job = images
        .begin_process(
            graph(Arc::new(Coverage(ClientId(1)))),
            b"[]".as_slice().into(),
        )
        .unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    let resource = loop {
        match job.poll().unwrap() {
            ImageProcessState::Ready(resource) => break resource,
            ImageProcessState::Failed(error) => panic!("{error}"),
            ImageProcessState::Pending => {
                assert!(Instant::now() < until);
                std::thread::yield_now();
            }
        }
    };
    assert_eq!(resource.client(), ClientId(1));
    assert_eq!(resource.size(), [3, 2]);
    assert_eq!(
        resource.rgba().as_ref(),
        &[
            9, 19, 29, 61, 9, 19, 29, 0, 9, 19, 29, 64, 9, 19, 29, 61, 9, 19, 29, 128, 9, 19, 29,
            255,
        ]
    );
    assert!(!resource.opaque());
    assert_eq!(
        resource.alpha_region([1, 0, 2, 2]).unwrap().as_ref(),
        &[0, 64, 128, 255]
    );
    let ImageProcessState::Ready(retained) = job.poll().unwrap() else {
        panic!("ready receipt was not retained")
    };
    assert!(Arc::ptr_eq(&retained, &resource));
}
#[test]
fn dropping_alpha_copy_query_releases_its_input_after_shared_worker_cancellation() {
    let root = tempfile::tempdir().unwrap();
    let mut runtime = runtime(root.path());
    let images = runtime.host.image_metadata(ClientId(1)).unwrap();
    let source = Arc::new(Coverage(ClientId(1)));
    let weak = Arc::downgrade(&source);
    let job = images
        .begin_process(graph(source.clone()), b"[]".as_slice().into())
        .unwrap();
    drop(source);
    drop(job);
    let until = Instant::now() + Duration::from_secs(5);
    while weak.upgrade().is_some() {
        assert!(
            Instant::now() < until,
            "cancelled/finished worker retained abandoned input"
        );
        std::thread::yield_now();
    }
}

#![cfg(feature = "arc-formats")]
//! Engine-owned metadata/resource contracts without a native VM or renderer.
use crysirror_core::{
    image_encoding::{ImageEncodingInfoQuery, ImageEncodingInfoReceipt, ImageEncodingInfoState},
    image_processing::{ImageInput, ImageJob, ImageProcessState},
    service::HostServices,
    state::ClientId,
};
use crysirror_runtime::Runtime;
use std::time::{Duration, Instant};
fn qnt(alpha: bool) -> Vec<u8> {
    // Authored1x1 QNT2. The explicit zlib bytes inflate to [255,pad0].
    let compressed = [120, 156, 251, 207, 0, 0, 2, 0, 1, 0];
    let mut bytes = b"QNT\0".to_vec();
    for value in [
        2u32,
        68,
        (-7i32) as u32,
        9,
        1,
        1,
        24,
        1,
        0,
        if alpha { 10 } else { 0 },
    ] {
        bytes.extend(value.to_le_bytes());
    }
    bytes.resize(68, 0);
    if alpha {
        bytes.extend(compressed)
    }
    bytes
}
fn runtime() -> (tempfile::TempDir, Runtime) {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("assets")).unwrap();
    let project = root.path().join("project.json");
    std::fs::write(&project,br#"{"schema":2,"name":"encoded headers","content_revision":"1","canvas":[1,1],"assets":"assets","clients":[{"name":"one","adapter":"crysirror-script-v1","game":"one"}]}"#).unwrap();
    let runtime = Runtime::load(&project, &root.path().join("state")).unwrap();
    (root, runtime)
}
fn receipt(query: &mut dyn ImageEncodingInfoQuery) -> ImageEncodingInfoReceipt {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match query.poll().unwrap() {
            ImageEncodingInfoState::Pending => {
                assert!(Instant::now() < deadline);
                std::thread::yield_now()
            }
            ImageEncodingInfoState::Ready(receipt) => return receipt,
            ImageEncodingInfoState::Failed(error) => panic!("{error}"),
        }
    }
}
fn image(mut job: ImageJob) -> ImageProcessState {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match job.poll().unwrap() {
            ImageProcessState::Pending => {
                assert!(Instant::now() < deadline);
                std::thread::yield_now()
            }
            state => return state,
        }
    }
}
#[test]
fn scoped_encoded_planes_have_retained_identity_and_decode_separate_from_opacity() {
    let (root, mut runtime) = runtime();
    let owner = ClientId(1);
    let path = root.path().join("assets/picture.qnt");
    std::fs::write(&path, qnt(true)).unwrap();
    let metadata = runtime.host.image_metadata(owner).unwrap();
    let mut query = metadata.begin_encoding_info("picture.qnt").unwrap();
    assert_eq!(query.client(), owner);
    let original = receipt(query.as_mut());
    assert_eq!(
        query.poll().unwrap(),
        ImageEncodingInfoState::Ready(original.clone())
    );
    assert_eq!(original.client, owner);
    assert_eq!(original.asset.relative(), "picture.qnt");
    assert_eq!(original.encoding.source_origin, Some([-7, 9]));
    assert_eq!(original.encoding.has_alpha_plane, Some(true));
    assert_eq!(original.encoding.has_color_plane, Some(false));
    let input = || ImageInput::Asset {
        path: "picture.qnt".into(),
        size: [1, 1],
        fingerprint: Some(original.fingerprint),
    };
    let ImageProcessState::Ready(decoded) = image(
        metadata
            .begin_process(input(), b"[]".as_slice().into())
            .unwrap(),
    ) else {
        panic!("shared decode")
    };
    assert_eq!(decoded.client(), owner);
    assert_eq!(decoded.rgba().as_ref(), [0, 0, 0, 255]);
    assert!(decoded.opaque());
    std::fs::write(&path, qnt(false)).unwrap();
    let mut replacement = metadata.begin_encoding_info("picture.qnt").unwrap();
    let replacement = receipt(replacement.as_mut());
    assert_eq!(replacement.encoding.has_alpha_plane, Some(false));
    assert_ne!(replacement.fingerprint, original.fingerprint);
    assert!(matches!(
        image(
            metadata
                .begin_process(input(), b"[]".as_slice().into())
                .unwrap()
        ),
        ImageProcessState::Failed(_)
    ));
    assert!(metadata.begin_encoding_info("../outside.qnt").is_err());
    assert!(runtime.host.image_metadata(ClientId(999)).is_err());
    assert_eq!(runtime.host.now_ms(), 0);
}
#[test]
fn header_ready_does_not_ack_corrupted_payload_and_unknown_metadata_fails() {
    let (root, mut runtime) = runtime();
    let mut bytes = qnt(true);
    let end = bytes.len() - 1;
    bytes[end] ^= 7;
    std::fs::write(root.path().join("assets/bad.qnt"), bytes).unwrap();
    std::fs::write(root.path().join("assets/unknown"), b"unknown format").unwrap();
    let metadata = runtime.host.image_metadata(ClientId(1)).unwrap();
    let mut query = metadata.begin_encoding_info("bad.qnt").unwrap();
    let header = receipt(query.as_mut());
    assert!(matches!(
        image(
            metadata
                .begin_process(
                    ImageInput::Asset {
                        path: "bad.qnt".into(),
                        size: header.encoding.size,
                        fingerprint: Some(header.fingerprint)
                    },
                    b"[]".as_slice().into()
                )
                .unwrap()
        ),
        ImageProcessState::Failed(_)
    ));
    let mut query = metadata.begin_encoding_info("unknown").unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match query.poll().unwrap() {
            ImageEncodingInfoState::Pending => {
                assert!(Instant::now() < deadline);
                std::thread::yield_now()
            }
            ImageEncodingInfoState::Failed(error) => {
                assert!(error.to_string().contains("metadata"));
                assert_eq!(query.poll().unwrap(), ImageEncodingInfoState::Failed(error));
                break;
            }
            ImageEncodingInfoState::Ready(_) => panic!("unknown metadata fabricated"),
        }
    }
}

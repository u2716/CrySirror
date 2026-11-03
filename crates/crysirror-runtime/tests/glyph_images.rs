//! Authored coverage faces and shared image jobs: no VM/GPU/system font needed.
use crysirror_core::{image_processing::*, service::*, state::ClientId};
use crysirror_runtime::Runtime;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
fn runtime(root: &std::path::Path) -> Runtime {
    let project=serde_json::from_value(serde_json::json!({"schema":2,"name":"glyph images","content_revision":"1","canvas":[8,4],"assets":"assets","clients":[{"name":"one","adapter":"crysirror-script-v1","game":"test"},{"name":"two","adapter":"crysirror-script-v1","game":"test"}]})).unwrap();
    Runtime::with_asset_root(project, root, &root.join("state"), Default::default()).unwrap()
}
fn fixture() -> Arc<[u8]> {
    let mut b = b"TVP pre-rendered font\x1a".to_vec();
    b.extend([1, 2]);
    b.extend(1u32.to_le_bytes());
    b.extend(36u32.to_le_bytes());
    b.extend(38u32.to_le_bytes());
    b.extend((b'A' as u16).to_le_bytes());
    b.extend(58u32.to_le_bytes());
    for v in [2i16, 2, -1, 2, 3, 0, 4, 0] {
        b.extend(v.to_le_bytes());
    }
    b.extend([32, 0x43]);
    b.into()
}
fn font(lib: &dyn FontLibrary) -> FontInfo {
    let job = lib
        .begin(FontLoadSpec {
            name: "authored".into(),
            source: FontLoadSource::PrecomputedBytes {
                bytes: fixture(),
                pixel_size: 24,
            },
            face_index: 0,
        })
        .unwrap();
    let end = Instant::now() + Duration::from_secs(5);
    let info = loop {
        match lib.status(job).unwrap() {
            FontLoadState::Ready(info) => break info,
            FontLoadState::Failed(e) => panic!("{e:?}"),
            FontLoadState::Pending => {
                assert!(Instant::now() < end);
                std::thread::yield_now();
            }
        }
    };
    lib.release_job(job).unwrap();
    info
}
fn spec(alias: &str) -> GlyphImageSpec {
    GlyphImageSpec {
        font: alias.into(),
        size: 24,
        canvas: [4, 3],
        glyphs: vec![
            Glyph {
                glyph_id: 1,
                x_26_6: 64,
                y_26_6: 128,
                advance_26_6: 192,
            },
            Glyph {
                glyph_id: 1,
                x_26_6: 128,
                y_26_6: 128,
                advance_26_6: 192,
            },
        ],
    }
}
fn image(job: &mut ImageJob) -> Arc<dyn ImageResource> {
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        match job.poll().unwrap() {
            ImageProcessState::Ready(image) => return image,
            ImageProcessState::Failed(e) => panic!("{e}"),
            ImageProcessState::Pending => {
                assert!(Instant::now() < end);
                std::thread::yield_now();
            }
        }
    }
}
#[test]
fn scoped_frozen_glyph_worker_publishes_real_pixels_and_shared_copy_operations() {
    let root = tempfile::tempdir().unwrap();
    let mut rt = runtime(root.path());
    let owner = ClientId(1);
    let fonts = rt.host.font_library(owner).unwrap();
    let info = font(fonts.as_ref());
    let images = rt.host.image_metadata(owner).unwrap();
    let other = rt.host.image_metadata(ClientId(2)).unwrap();
    assert!(other.begin_glyph_image(spec(&info.alias)).is_err());
    let mut job = images.begin_glyph_image(spec(&info.alias)).unwrap();
    // Text retains a released face for as long as an admitted job holds it.
    fonts.release_font(info.handle).unwrap();
    let glyph = image(&mut job);
    assert_eq!(glyph.client(), owner);
    assert_eq!(glyph.size(), [4, 3]);
    assert!(!glyph.opaque());
    assert_eq!(
        glyph.alpha_region([0, 0, 4, 3]).unwrap().as_ref(),
        [128, 192, 128, 0, 128, 192, 128, 0, 0, 0, 0, 0]
    );
    assert_eq!(glyph.pixel_at([1, 0]).unwrap(), Some([255, 255, 255, 192]));
    let retained = image(&mut job);
    assert!(Arc::ptr_eq(&retained, &glyph));
    assert!(rt
        .host
        .publish_image_resource(ClientId(2), 9, glyph.clone())
        .is_err());
    let published = rt
        .host
        .publish_image_resource(owner, 9, glyph.clone())
        .unwrap();
    assert_eq!(published.size, [4, 3]);
    let graph = ImageInput::Composite {
        base: Box::new(ImageInput::Solid {
            size: [4, 3],
            color: [3, 5, 7, 61],
        }),
        layers: vec![ImageCompositeLayer {
            input: Box::new(ImageInput::Resource(glyph)),
            source: [0, 0, 4, 3],
            destination: [0, 0],
            clip: None,
            opacity: 255,
            math: ImageCompositeMath::CopyAlpha,
        }],
    };
    let copied = image(
        &mut images
            .begin_process(graph, b"[]".as_slice().into())
            .unwrap(),
    );
    assert_eq!(copied.pixel_at([1, 0]).unwrap(), Some([3, 5, 7, 192]));
    assert_eq!(copied.pixel_at([3, 2]).unwrap(), Some([3, 5, 7, 0]));
}
#[test]
fn bounds_invalid_glyph_and_drop_preserve_failure_and_no_implicit_publication() {
    let root = tempfile::tempdir().unwrap();
    let mut rt = runtime(root.path());
    let fonts = rt.host.font_library(ClientId(1)).unwrap();
    let info = font(fonts.as_ref());
    let images = rt.host.image_metadata(ClientId(1)).unwrap();
    let mut bad = spec(&info.alias);
    bad.canvas = [u32::MAX, u32::MAX];
    assert!(images.begin_glyph_image(bad).is_err());
    let mut bad = spec(&info.alias);
    bad.canvas = [8193, 1];
    assert!(images.begin_glyph_image(bad).is_err());
    let mut bad = spec(&info.alias);
    bad.size = 0;
    assert!(images.begin_glyph_image(bad).is_err());
    let mut bad = spec(&info.alias);
    bad.glyphs = vec![bad.glyphs[0].clone(); 65537];
    assert!(images.begin_glyph_image(bad).is_err());
    let mut bad = spec(&info.alias);
    bad.glyphs[0].glyph_id = 999;
    let mut job = images.begin_glyph_image(bad).unwrap();
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        match job.poll().unwrap() {
            ImageProcessState::Failed(_) => break,
            ImageProcessState::Ready(_) => panic!("bad glyph published"),
            ImageProcessState::Pending => {
                assert!(Instant::now() < end);
                std::thread::yield_now();
            }
        }
    }
    assert!(matches!(job.poll().unwrap(), ImageProcessState::Failed(_)));
    let job = images.begin_glyph_image(spec(&info.alias)).unwrap();
    drop(job);
    fonts.release_font(info.handle).unwrap();
}

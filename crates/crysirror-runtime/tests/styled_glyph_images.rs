//! Authored 3x3 binary faces and immutable styled jobs; no system font/VM/device.
use crysirror_core::{image_blend::*, image_processing::*, service::*, state::ClientId};
use crysirror_runtime::Runtime;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
fn fixture() -> Arc<[u8]> {
    let count = 10u32;
    let index = 36u32;
    let records = index + count * 2;
    let pixels = records + count * 20;
    let mut b = b"TVP pre-rendered font\x1a".to_vec();
    b.extend([1, 2]);
    b.extend(count.to_le_bytes());
    b.extend(index.to_le_bytes());
    b.extend(records.to_le_bytes());
    for i in 0..count {
        b.extend((b'A' as u16 + i as u16).to_le_bytes());
    }
    for i in 0..count {
        b.extend((pixels + i * 9).to_le_bytes());
        for v in [3i16, 3, 0, 3, 3, 0, 3, 0] {
            b.extend(v.to_le_bytes());
        }
    }
    let order = [4, 0, 2, 6, 8, 1, 3, 5, 7];
    for n in 0..10 {
        let mut p = [0u8; 9];
        for at in &order[..n] {
            p[*at] = 64;
        }
        b.extend(p);
    }
    b.into()
}
fn runtime(root: &std::path::Path) -> Runtime {
    let p=serde_json::from_value(serde_json::json!({"schema":2,"name":"styled","content_revision":"1","canvas":[2,1],"assets":"assets","clients":[{"name":"one","adapter":"crysirror-script-v1","game":"one"},{"name":"two","adapter":"crysirror-script-v1","game":"two"}]})).unwrap();
    Runtime::with_asset_root(p, root, &root.join("state"), Default::default()).unwrap()
}
fn font(lib: &dyn FontLibrary) -> FontInfo {
    let job = lib
        .begin(FontLoadSpec {
            name: "curve".into(),
            source: FontLoadSource::PrecomputedBytes {
                bytes: fixture(),
                pixel_size: 48,
            },
            face_index: 0,
        })
        .unwrap();
    let end = Instant::now() + Duration::from_secs(5);
    let info = loop {
        match lib.status(job).unwrap() {
            FontLoadState::Ready(i) => break i,
            FontLoadState::Failed(e) => panic!("{e}"),
            FontLoadState::Pending => {
                assert!(Instant::now() < end);
                std::thread::yield_now();
            }
        }
    };
    lib.release_job(job).unwrap();
    info
}
fn ready(job: &mut ImageJob) -> Arc<dyn ImageResource> {
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        match job.poll().unwrap() {
            ImageProcessState::Ready(i) => return i,
            ImageProcessState::Failed(e) => panic!("{e}"),
            ImageProcessState::Pending => {
                assert!(Instant::now() < end);
                std::thread::yield_now();
            }
        }
    }
}
fn spec(font: &str, glyph: u32) -> StyledGlyphImageSpec {
    StyledGlyphImageSpec {
        glyphs: GlyphImageSpec {
            font: font.into(),
            size: 16,
            canvas: [1, 1],
            glyphs: vec![Glyph {
                glyph_id: glyph,
                x_26_6: 0,
                y_26_6: 64,
                advance_26_6: 64,
            }],
        },
        raster: GlyphRasterOptions {
            supersample: 3,
            binary_cutoff: Some(128),
            coverage_lut: Some(Arc::from([0, 28, 56, 113, 141, 169, 198, 226, 254, 255])),
            ..Default::default()
        },
    }
}
#[test]
fn styled_worker_freezes_authorized_face_and_publishes_real_curve_pixels_without_catalog_side_effects(
) {
    let t = tempfile::tempdir().unwrap();
    let mut rt = runtime(t.path());
    let lib = rt.host.font_library(ClientId(1)).unwrap();
    let info = font(lib.as_ref());
    let images = rt.host.image_metadata(ClientId(1)).unwrap();
    let foreign = rt.host.image_metadata(ClientId(2)).unwrap();
    assert!(foreign
        .begin_styled_glyph_image(spec(&info.alias, 4))
        .is_err());
    let mut job = images
        .begin_styled_glyph_image(spec(&info.alias, 4))
        .unwrap();
    // Text retains a released face for as long as an admitted job holds it.
    lib.release_font(info.handle).unwrap();
    let image = ready(&mut job);
    assert_eq!(image.client(), ClientId(1));
    assert_eq!(image.size(), [1, 1]);
    assert_eq!(image.rgba().as_ref(), [255, 255, 255, 113]);
    assert!(Arc::ptr_eq(&image, &ready(&mut job)));
    assert!(rt
        .host
        .publish_image_resource(ClientId(2), 8, image.clone())
        .is_err());
    rt.host
        .publish_image_resource(ClientId(1), 8, image)
        .unwrap();
}
fn composite(base: ImageInput, source: ImageInput, math: ImageCompositeMath) -> ImageInput {
    ImageInput::Composite {
        base: Box::new(base),
        layers: vec![ImageCompositeLayer {
            input: Box::new(source),
            source: [0, 0, 1, 1],
            destination: [0, 0],
            clip: None,
            opacity: 255,
            math,
        }],
    }
}
#[test]
fn shared_styled_coverage_drives_exact_color_products_and_alpha_cell_overwrite() {
    let t = tempfile::tempdir().unwrap();
    let mut rt = runtime(t.path());
    let lib = rt.host.font_library(ClientId(1)).unwrap();
    let info = font(lib.as_ref());
    let images = rt.host.image_metadata(ClientId(1)).unwrap();
    let mut glyph = images
        .begin_styled_glyph_image(spec(&info.alias, 4))
        .unwrap();
    let glyph = ready(&mut glyph);
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
        source_scale: [0., 0., 0., 1.],
        source_bias: [200. / 255., 100. / 255., 50. / 255., 0.],
        write_mask: [true, true, true, false],
        ..Default::default()
    };
    let base = || ImageInput::Solid {
        size: [1, 1],
        color: [10, 20, 30, 61],
    };
    let mut paint = images
        .begin_process(
            composite(
                base(),
                ImageInput::Resource(glyph.clone()),
                ImageCompositeMath::Normalized(state),
            ),
            b"[]".as_slice().into(),
        )
        .unwrap();
    assert_eq!(ready(&mut paint).rgba().as_ref(), [93, 55, 38, 61]);
    let mut alpha = images
        .begin_process(
            composite(
                base(),
                ImageInput::Resource(glyph),
                ImageCompositeMath::CopyAlpha,
            ),
            b"[]".as_slice().into(),
        )
        .unwrap();
    assert_eq!(ready(&mut alpha).rgba().as_ref(), [10, 20, 30, 113]);
    let mut blank = images
        .begin_styled_glyph_image(spec(&info.alias, 1))
        .unwrap();
    let blank = ready(&mut blank);
    let mut alpha = images
        .begin_process(
            composite(
                base(),
                ImageInput::Resource(blank),
                ImageCompositeMath::CopyAlpha,
            ),
            b"[]".as_slice().into(),
        )
        .unwrap();
    assert_eq!(
        ready(&mut alpha).rgba().as_ref(),
        [10, 20, 30, 0],
        "blank glyph cells overwrite prior alpha; they are not a union overlay"
    );
}
#[test]
fn malformed_lookup_and_working_budget_are_rejected_before_admission() {
    let t = tempfile::tempdir().unwrap();
    let mut rt = runtime(t.path());
    let lib = rt.host.font_library(ClientId(1)).unwrap();
    let info = font(lib.as_ref());
    let images = rt.host.image_metadata(ClientId(1)).unwrap();
    let mut invalid = spec(&info.alias, 4);
    invalid.raster.coverage_lut = Some(Arc::from([0; 9]));
    assert!(images.begin_styled_glyph_image(invalid).is_err());
    let mut invalid = spec(&info.alias, 4);
    invalid.glyphs.size = 171;
    assert!(images.begin_styled_glyph_image(invalid).is_err());
    let mut invalid = spec(&info.alias, 4);
    invalid.glyphs.canvas = [8192, 10];
    invalid.raster.supersample = 4;
    invalid.raster.embolden_radius = 16;
    invalid.raster.coverage_lut = None;
    assert!(images.begin_styled_glyph_image(invalid).is_err());
    let mut valid = images
        .begin_styled_glyph_image(spec(&info.alias, 4))
        .unwrap();
    assert_eq!(ready(&mut valid).rgba().as_ref(), [255, 255, 255, 113]);
}

fn batch_spec(alias: &str) -> GlyphImageBatchSpec {
    let style = spec(alias, 1);
    GlyphImageBatchSpec {
        font: alias.into(),
        size: style.glyphs.size,
        raster: style.raster,
        cells: (1..=10)
            .map(|glyph| {
                let item = spec(alias, glyph);
                GlyphImageCell {
                    canvas: item.glyphs.canvas,
                    glyphs: item.glyphs.glyphs,
                }
            })
            .collect(),
    }
}
fn batch_ready(job: &mut dyn GlyphImageBatchQuery) -> Arc<dyn GlyphImageBatchResource> {
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        match job.poll().unwrap() {
            GlyphImageBatchState::Ready(batch) => return batch,
            GlyphImageBatchState::Failed(e) => panic!("{e}"),
            GlyphImageBatchState::Cancelled => panic!("unexpected cancellation"),
            GlyphImageBatchState::Pending => {
                assert!(Instant::now() < end);
                std::thread::yield_now();
            }
        }
    }
}
#[test]
fn cell_batch_freezes_one_face_and_returns_independently_owned_exact_cells() {
    let t = tempfile::tempdir().unwrap();
    let mut rt = runtime(t.path());
    let fonts = rt.host.font_library(ClientId(1)).unwrap();
    let info = font(fonts.as_ref());
    assert_eq!(fonts.face_style(&info.alias).unwrap(), None);
    assert_eq!(fonts.glyph_id(&info.alias, 'D').unwrap(), 4);
    let images = rt.host.image_metadata(ClientId(1)).unwrap();
    let foreign = rt.host.image_metadata(ClientId(2)).unwrap();
    assert!(foreign
        .begin_glyph_image_batch(batch_spec(&info.alias))
        .is_err());
    let mut query = images
        .begin_glyph_image_batch(batch_spec(&info.alias))
        .unwrap();
    assert_eq!(query.client(), ClientId(1));
    fonts.release_font(info.handle).unwrap();
    let batch = batch_ready(query.as_mut());
    assert_eq!(batch.client(), ClientId(1));
    let expected = [0, 28, 56, 113, 141, 169, 198, 226, 254, 255];
    for (cell, alpha) in batch.images().iter().zip(expected) {
        assert_eq!(cell.client(), ClientId(1));
        assert_eq!(cell.size(), [1, 1]);
        assert_eq!(&*cell.rgba(), &[255, 255, 255, alpha]);
    }
    let retained = batch.images()[4].clone();
    drop(batch);
    query.cancel().unwrap();
    assert!(matches!(
        query.poll().unwrap(),
        GlyphImageBatchState::Cancelled
    ));
    drop(query);
    assert_eq!(&*retained.rgba(), &[255, 255, 255, 141]);
}
#[test]
fn cell_batch_rejects_full_admission_and_never_exposes_late_partial_results() {
    let t = tempfile::tempdir().unwrap();
    let mut rt = runtime(t.path());
    let fonts = rt.host.font_library(ClientId(1)).unwrap();
    let info = font(fonts.as_ref());
    let images = rt.host.image_metadata(ClientId(1)).unwrap();
    let mut invalid = batch_spec(&info.alias);
    invalid.cells.last_mut().unwrap().canvas = [8193, 1];
    assert!(images.begin_glyph_image_batch(invalid).is_err());
    let mut invalid = batch_spec(&info.alias);
    invalid.cells.last_mut().unwrap().glyphs[0].glyph_id = 65535;
    let mut query = images.begin_glyph_image_batch(invalid).unwrap();
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        match query.poll().unwrap() {
            GlyphImageBatchState::Failed(_) => break,
            GlyphImageBatchState::Pending => {
                assert!(Instant::now() < end);
                std::thread::yield_now();
            }
            _ => panic!("late invalid glyph exposed partial successful batch"),
        }
    }
    assert!(matches!(
        query.poll().unwrap(),
        GlyphImageBatchState::Failed(_)
    ));
    query.cancel().unwrap();
    assert!(matches!(
        query.poll().unwrap(),
        GlyphImageBatchState::Cancelled
    ));
    let mut valid = images
        .begin_glyph_image_batch(batch_spec(&info.alias))
        .unwrap();
    assert_eq!(batch_ready(valid.as_mut()).images().len(), 10);
}

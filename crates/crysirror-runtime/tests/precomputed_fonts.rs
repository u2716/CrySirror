//! Self-authored two-pixel glyph coverage; no installed font, game or VM needed.
use crysirror_core::{
    path::{Mount, VfsPath},
    service::*,
    state::{ClientId, Color, Rect, TextRun},
};
use crysirror_runtime::font_resources::FontResources;
use crysirror_tasks::TaskPool;
use crysirror_text::{TextSystem, glyph::GlyphRaster};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
struct Empty;
impl ClientStorage for Empty {
    fn read(&self, _: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
        Err(crysirror_core::Error::NotFound)
    }
    fn write_atomic(&self, _: &VfsPath, _: &[u8]) -> crysirror_core::Result<()> {
        Err(crysirror_core::Error::ReadOnly)
    }
    fn remove(&self, _: &VfsPath) -> crysirror_core::Result<()> {
        Err(crysirror_core::Error::ReadOnly)
    }
    fn list(&self, _: Mount, _: &str) -> crysirror_core::Result<Vec<String>> {
        Ok(Vec::new())
    }
}
fn fixture() -> Arc<[u8]> {
    let mut b = b"TVP pre-rendered font\x1a".to_vec();
    b.extend([1, 2]);
    b.extend(1u32.to_le_bytes());
    b.extend(36u32.to_le_bytes());
    b.extend(38u32.to_le_bytes());
    b.extend((b'A' as u16).to_le_bytes());
    b.extend(58u32.to_le_bytes());
    for value in [2i16, 2, -1, 2, 3, 0, 4, 0] {
        b.extend(value.to_le_bytes());
    }
    b.extend([32, 0x43]);
    b.into()
}
fn ready(lib: &dyn FontLibrary, job: FontJob) -> FontLoadState {
    let now = Instant::now();
    loop {
        let state = lib.status(job).unwrap();
        if state != FontLoadState::Pending {
            return state;
        }
        assert!(now.elapsed() < Duration::from_secs(10));
        std::thread::yield_now();
    }
}
#[test]
fn workers_publish_fixed_metrics_and_shared_coverage_with_scoping_and_frozen_retention() {
    let text = TextSystem::default();
    let fonts = FontResources::new(
        text.clone(),
        TaskPool::new("bitmap-io", 1, 16).unwrap(),
        TaskPool::new("bitmap-cpu", 1, 16).unwrap(),
    );
    let a = fonts.for_client(ClientId(17), Arc::new(Empty));
    let b = fonts.for_client(ClientId(18), Arc::new(Empty));
    let job = a
        .begin(FontLoadSpec {
            name: "authored".into(),
            source: FontLoadSource::PrecomputedBytes {
                bytes: fixture(),
                pixel_size: 24,
            },
            face_index: 0,
        })
        .unwrap();
    assert!(b.status(job).is_err());
    let FontLoadState::Ready(info) = ready(a.as_ref(), job) else {
        panic!("font not ready")
    };
    a.release_job(job).unwrap();
    assert_eq!(info.collection_faces, 1);
    assert!(info.families.is_empty());
    assert_eq!(
        a.resolve_alias("authored").unwrap(),
        Some(info.alias.clone())
    );
    assert_eq!(
        a.resolve_alias(&info.alias).unwrap(),
        Some(info.alias.clone())
    );
    assert!(b.resolve_alias("authored").unwrap().is_none());
    assert!(b.resolve_alias(&info.alias).unwrap().is_none());
    let scoped_metrics = a.metrics(&info.alias, 24., 'A').unwrap();
    assert_eq!(scoped_metrics.advance, 4.);
    assert!(b.metrics(&info.alias, 24., 'A').is_err());
    assert!(a.metrics(&info.alias, 32., 'A').is_err());
    assert!(a.metrics(&info.alias, f32::NAN, 'A').is_err());
    let request = GlyphBoundsSpec {
        font: info.alias.clone(),
        glyphs: vec![
            GlyphBoundsRequest {
                character: 'A',
                size: 24.,
                baseline: [1., 2.],
            },
            GlyphBoundsRequest {
                character: 'A',
                size: 24.,
                baseline: [4., 5.],
            },
        ],
    };
    assert!(b.begin_glyph_bounds(request.clone()).is_err());
    let mut bounds = a.begin_glyph_bounds(request.clone()).unwrap();
    let start = Instant::now();
    let measured = loop {
        match bounds.poll().unwrap() {
            GlyphBoundsProgress::Pending => {
                assert!(start.elapsed() < Duration::from_secs(10));
                std::thread::yield_now();
            }
            GlyphBoundsProgress::Ready(resource) => break resource,
            _ => panic!("actual shared glyph bounds must become ready"),
        }
    };
    assert_eq!(measured.client(), ClientId(17));
    assert_eq!(measured.digest(), info.digest);
    assert_eq!(measured.glyphs()[0].origin, [0, 0]);
    assert_eq!(measured.glyphs()[1].origin, [3, 3]);
    assert_eq!(measured.glyphs()[0].size, [2, 2]);
    let GlyphBoundsProgress::Ready(repeated) = bounds.poll().unwrap() else {
        panic!("retained completion")
    };
    assert!(Arc::ptr_eq(&measured, &repeated));
    let mut cancelled = a.begin_glyph_bounds(request.clone()).unwrap();
    cancelled.cancel().unwrap();
    assert!(matches!(
        cancelled.poll().unwrap(),
        GlyphBoundsProgress::Cancelled
    ));
    assert!(matches!(
        cancelled.poll().unwrap(),
        GlyphBoundsProgress::Cancelled
    ));
    let mut bad = request.clone();
    bad.glyphs[0].size = f32::NAN;
    assert!(a.begin_glyph_bounds(bad).is_err());
    assert!(!b.is_loaded(&info.alias).unwrap());
    let view = text.for_client(ClientId(17));
    assert_eq!(
        fonts
            .measurement(ClientId(17))
            .measure_simple(&info.alias, 24., "AA")
            .unwrap()
            .advance,
        8.
    );
    let placed = view.position_simple(&info.alias, 24, "AA").unwrap();
    assert_eq!(placed[1].baseline, [3., 2.]);
    let id = view.glyph_id(&info.alias, 'A').unwrap();
    let cell = view
        .rasterize_glyph_cell(
            &info.alias,
            GlyphRaster {
                glyph_id: id,
                size: 24.,
                baseline: [1., 2.],
            },
        )
        .unwrap();
    assert_eq!(cell.origin, [0, 0]);
    assert_eq!(cell.coverage, [0.5; 4]);
    assert!(
        view.rasterize_glyph_cell(
            &info.alias,
            GlyphRaster {
                glyph_id: id,
                size: 23.,
                baseline: [1., 2.]
            }
        )
        .is_err()
    );
    let run = TextRun {
        text: "AA".into(),
        font: info.alias.clone(),
        size_px: 24,
        bounds: Rect {
            x: 0,
            y: 0,
            width: 8,
            height: 4,
        },
        color: Color([255; 4]),
        layout_profile: UNWRAPPED_TEXT_LAYOUT.into(),
    };
    let layout = view.layout_glyphs(&run).unwrap();
    assert_eq!(layout[1].x_26_6, 3 * 64);
    let pixels = view.rasterize(&run).unwrap();
    assert_eq!(pixels[3], 128);
    assert_eq!(pixels[(2 * 4) + 3], 128);
    assert_eq!(
        pixels,
        view.rasterize_glyphs(&info.alias, 24, [8, 4], &layout)
            .unwrap()
    );
    // Pixel size is external TFT metadata and must participate in identity.
    let other_job = a
        .begin(FontLoadSpec {
            name: "authored".into(),
            source: FontLoadSource::PrecomputedBytes {
                bytes: fixture(),
                pixel_size: 32,
            },
            face_index: 0,
        })
        .unwrap();
    let FontLoadState::Ready(other) = ready(a.as_ref(), other_job) else {
        panic!("second fixed-size face not ready")
    };
    a.release_job(other_job).unwrap();
    assert_ne!(info.digest, other.digest);
    assert_ne!(info.alias, other.alias);
    assert_eq!(
        view.measure_simple(&other.alias, 32., "A")
            .unwrap()
            .line_height,
        32.
    );
    assert!(view.measure_simple(&other.alias, 24., "A").is_err());
    assert_eq!(
        view.measure_simple(&info.alias, 24., "A")
            .unwrap()
            .line_height,
        24.
    );
    assert_eq!(a.resolve("authored").unwrap().unwrap().alias, other.alias);
    a.release_font(other.handle).unwrap();
    let frozen = view.frozen().unwrap();
    a.release_font(info.handle).unwrap();
    assert_eq!(frozen.rasterize(&run).unwrap(), pixels);
    assert!(!fonts.has_resources());
}
#[test]
fn source_limits_malformed_streams_and_nonzero_face_indices_do_not_publish_faces() {
    let text = TextSystem::default();
    let fonts = FontResources::new(
        text,
        TaskPool::new("bitmap-bad-io", 1, 16).unwrap(),
        TaskPool::new("bitmap-bad-cpu", 1, 16).unwrap(),
    );
    let lib = fonts.for_client(ClientId(1), Arc::new(Empty));
    assert!(
        lib.begin(FontLoadSpec {
            name: "wrong-index".into(),
            source: FontLoadSource::PrecomputedBytes {
                bytes: fixture(),
                pixel_size: 24
            },
            face_index: 1
        })
        .is_err()
    );
    let mut bad = fixture().to_vec();
    bad[58] = 0x43;
    let job = lib
        .begin(FontLoadSpec {
            name: "bad".into(),
            source: FontLoadSource::PrecomputedBytes {
                bytes: bad.into(),
                pixel_size: 24,
            },
            face_index: 0,
        })
        .unwrap();
    assert!(matches!(ready(lib.as_ref(), job), FontLoadState::Failed(_)));
    lib.release_job(job).unwrap();
    assert!(!fonts.has_resources());
    assert!(lib.resolve("bad").unwrap().is_none());
}

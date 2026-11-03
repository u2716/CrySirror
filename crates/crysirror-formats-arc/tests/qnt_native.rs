//! Authored QNT byte vectors at the shared image-codec entry point.
use crysirror_image::{DecoderRegistry, ImageDecodeContext, ImageSurface};
use std::io::Write;
fn z(raw: &[u8]) -> Vec<u8> {
    let mut w = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    w.write_all(raw).unwrap();
    w.finish().unwrap()
}
fn qnt(
    version: u32,
    width: u32,
    height: u32,
    color: Option<&[u8]>,
    alpha: Option<&[u8]>,
) -> Vec<u8> {
    let color = color.map(z).unwrap_or_default();
    let alpha = alpha.map(z).unwrap_or_default();
    let header = match version {
        0 => 48,
        1 => 52,
        _ => 68,
    };
    let mut b = b"QNT\0".to_vec();
    b.extend(version.to_le_bytes());
    if version != 0 {
        b.extend((header as u32).to_le_bytes());
    }
    b.extend([0; 8]);
    for value in [width, height, 24, 1, color.len() as u32, alpha.len() as u32] {
        b.extend(value.to_le_bytes());
    }
    b.resize(header, 0);
    b.extend(color);
    b.extend(alpha);
    b
}
fn decode(bytes: &[u8]) -> anyhow::Result<ImageSurface> {
    let registry = DecoderRegistry::default();
    crysirror_formats_arc::images_a::decode(
        "alice-soft/qnt",
        bytes,
        &ImageDecodeContext::new("native.qnt", &registry),
    )
}
#[test]
fn qnt_0_1_2_share_predictor_channel_order_and_alpha() {
    // Hand-authored 2x2 residuals: packed B/G/R, column then row per 2x2 block.
    let color = [30, 176, 216, 196, 20, 176, 216, 196, 10, 176, 216, 196];
    let alpha = [70, 246, 236, 241];
    let rgba = [
        10, 20, 30, 70, 50, 60, 70, 80, 90, 100, 110, 90, 130, 140, 150, 100,
    ];
    for version in 0..=2 {
        let d = decode(&qnt(version, 2, 2, Some(&color), Some(&alpha))).unwrap();
        assert_eq!([d.width, d.height], [2, 2]);
        assert_eq!(d.rgba, rgba, "version {version}");
    }
}
#[test]
fn qnt_odd_height_accepts_both_original_and_aldbuilder_alpha_padding() {
    for version in 0..=2 {
        for alpha in [vec![1, 0, 255, 0, 255, 0], vec![1, 0, 255, 0, 255, 0, 0, 0]] {
            let d = decode(&qnt(version, 1, 3, None, Some(&alpha))).unwrap();
            assert_eq!(d.rgba, [0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3]);
        }
    }
    let d = decode(&qnt(1, 1, 1, None, None)).unwrap();
    assert_eq!(d.rgba, [0, 0, 0, 255]);
}
#[test]
fn qnt_invalid_headers_and_short_or_oversized_planes_fail() {
    let base = qnt(1, 2, 2, Some(&[0; 12]), Some(&[255; 4]));
    for (at, value) in [
        (4, 3u32),
        (8, 43),
        (8, u32::MAX),
        (20, 0),
        (20, u32::MAX),
        (28, 32),
        (36, u32::MAX),
        (40, u32::MAX),
    ] {
        let mut b = base.clone();
        b[at..at + 4].copy_from_slice(&value.to_le_bytes());
        assert!(decode(&b).is_err(), "field {at}={value}");
    }
    for b in [
        qnt(1, 2, 2, Some(&[]), None),
        qnt(1, 2, 2, Some(&[0; 11]), None),
        qnt(1, 2, 2, Some(&[0; 13]), None),
        qnt(1, 1, 3, None, Some(&[0; 5])),
        qnt(1, 1, 3, None, Some(&[0; 9])),
    ] {
        assert!(decode(&b).is_err());
    }
    for size in [0, 3, 7, 43, 51, base.len() - 1] {
        assert!(decode(&base[..size]).is_err(), "truncated {size}");
    }
}
#[test]
fn original_plane_metadata_is_distinct_from_decoded_opacity_for_every_header() {
    let mut registry = DecoderRegistry::default();
    crysirror_formats_arc::register_images(&mut registry).unwrap();
    for version in 0..=2 {
        let mut rgb = qnt(version, 2, 2, Some(&[0; 12]), None);
        let mut alpha = qnt(version, 2, 2, Some(&[0; 12]), Some(&[255, 0, 0, 0]));
        let offset = if version == 0 { 8 } else { 12 };
        for bytes in [&mut rgb, &mut alpha] {
            bytes[offset..offset + 4].copy_from_slice(&(-17i32).to_le_bytes());
            bytes[offset + 4..offset + 8].copy_from_slice(&23i32.to_le_bytes());
        }
        let a = registry
            .encoding_info_with("original", &rgb, None, None, None)
            .unwrap()
            .unwrap();
        let b = registry
            .encoding_info_with("original", &alpha, None, None, None)
            .unwrap()
            .unwrap();
        assert_eq!(a.source_origin, Some([-17, 23]));
        assert_eq!(b.size, [2, 2]);
        assert_eq!(a.color_bits, Some(24));
        assert_eq!(a.has_color_plane, Some(true));
        assert_eq!(a.has_alpha_plane, Some(false));
        assert_eq!(b.has_alpha_plane, Some(true));
        assert_eq!(b.color_row_bytes, Some(6));
        assert_eq!(b.alpha_sample_bytes, Some(1));
        let pixels = decode(&rgb).unwrap();
        assert!(pixels.rgba.chunks_exact(4).all(|p| p[3] == 255));
        assert_eq!(pixels.rgba, decode(&alpha).unwrap().rgba);
    }
    let empty = qnt(1, 1, 1, None, None);
    let info = registry
        .encoding_info_with("original", &empty, None, None, None)
        .unwrap()
        .unwrap();
    assert_eq!(info.has_color_plane, Some(false));
    assert_eq!(info.has_alpha_plane, Some(false));
    assert_eq!(decode(&empty).unwrap().rgba, [0, 0, 0, 255]);
}
#[test]
fn encoded_metadata_confirms_ranges_not_successful_payload_decode() {
    let mut registry = DecoderRegistry::default();
    crysirror_formats_arc::register_images(&mut registry).unwrap();
    let mut damaged = qnt(2, 1, 1, None, Some(&[255, 0]));
    let last = damaged.len() - 1;
    damaged[last] ^= 7;
    assert!(registry
        .encoding_info_with("original", &damaged, None, None, None)
        .unwrap()
        .is_some());
    assert!(decode(&damaged).is_err());
    damaged.pop();
    assert!(registry
        .encoding_info_with("original", &damaged, None, None, None)
        .is_err());
    assert!(decode(&damaged).is_err());
}

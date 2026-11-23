// Copyright (c) 2026 CrySirror contributors. GPL-3.0-or-later.
//! Authored frame catalogs/pixels. No CatSystem VM or proprietary fixtures.
use crysirror_core::{
    service::{AssetReader, MemoryAssetReader},
    state::ClientId,
};
use crysirror_resource::{read_all, Archive, DecodeContext};
use std::{io::Write, sync::Arc};
fn mem(bytes: Vec<u8>) -> Arc<dyn AssetReader> {
    Arc::new(MemoryAssetReader(bytes.into()))
}
fn zlib(data: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}
fn stdinfo(
    depth: u32,
    origin: [i32; 2],
    base: Option<[i32; 2]>,
    size: [u32; 2],
    canvas: [u32; 2],
) -> Vec<u8> {
    let mut data = Vec::new();
    for value in [
        size[0],
        size[1],
        depth,
        origin[0] as u32,
        origin[1] as u32,
        canvas[0],
        canvas[1],
    ] {
        data.extend(value.to_le_bytes());
    }
    if let Some(base) = base {
        for value in [7, base[0] as u32, base[1] as u32] {
            data.extend(value.to_le_bytes());
        }
    }
    data
}
fn gamma(bits: &mut Vec<bool>, value: u32) {
    let width = value.ilog2();
    bits.extend(std::iter::repeat_n(false, width as usize));
    bits.push(true);
    for shift in (0..width).rev() {
        bits.push(value & (1 << shift) != 0);
    }
}
fn control() -> Vec<u8> {
    let mut bits = vec![true];
    gamma(&mut bits, 4);
    gamma(&mut bits, 4);
    let ctl: Vec<_> = bits
        .chunks(8)
        .map(|chunk| {
            chunk
                .iter()
                .enumerate()
                .fold(0, |byte, (bit, value)| byte | ((*value as u8) << bit))
        })
        .collect();
    let data = zlib(&[255; 4]);
    let packed_ctl = zlib(&ctl);
    let mut out = vec![0; 8];
    for length in [data.len(), 4, packed_ctl.len(), ctl.len()] {
        out.extend((length as u32).to_le_bytes());
    }
    out.extend(data);
    out.extend(packed_ctl);
    out
}
fn frame(id: u32, tags: Vec<(&[u8], Vec<u8>, usize)>) -> Vec<u8> {
    let mut out = vec![0; 4];
    out.extend(id.to_le_bytes());
    let count = tags.len();
    for (index, (name, data, padding)) in tags.into_iter().enumerate() {
        assert_eq!(name.len(), 8);
        out.extend(name);
        out.extend(
            (if index + 1 < count {
                16 + data.len() as u32 + padding as u32
            } else {
                0
            })
            .to_le_bytes(),
        );
        out.extend((data.len() as u32).to_le_bytes());
        out.extend(data);
        out.extend(std::iter::repeat_n(0xa5, padding));
    }
    out
}
fn file(mut frames: Vec<Vec<u8>>) -> Vec<u8> {
    let mut data = b"HG-3".to_vec();
    data.extend(12u32.to_le_bytes());
    data.extend(0x300u32.to_le_bytes());
    let count = frames.len();
    for (index, frame) in frames.iter_mut().enumerate() {
        if index + 1 < count {
            let size = frame.len() as u32;
            frame[..4].copy_from_slice(&size.to_le_bytes());
        }
        data.extend(frame.iter());
    }
    data
}
fn open(data: Vec<u8>, cx: &DecodeContext) -> anyhow::Result<Box<dyn Archive>> {
    crysirror_formats_arc::legacy_archive("cat-system/hg3")
        .unwrap()
        .open(mem(data), cx)
}
fn image(archive: &dyn Archive, id: u32, cx: &DecodeContext) -> crysirror_image::ImageSurface {
    let encoded = read_all(
        archive.open(id, cx).unwrap().as_ref(),
        cx.limits.max_entry_size,
    )
    .unwrap();
    crysirror_image::decode_standard(&encoded).unwrap()
}
#[test]
fn native_frame_ids_signed_origin_base_and_link_padding_are_not_lost() {
    let data = file(vec![
        frame(
            23,
            vec![
                (
                    b"stdinfo\0",
                    stdinfo(32, [-1, 1], Some([-11, 50]), [1, 1], [3, 2]),
                    7,
                ),
                (b"comment\0", b"native-padding".to_vec(), 3),
                (b"img0000\0", control(), 5),
            ],
        ),
        frame(
            0x80000001,
            vec![
                (b"stdinfo\0", stdinfo(32, [1, 0], None, [1, 1], [2, 1]), 0),
                (b"img0000\0", control(), 0),
            ],
        ),
    ]);
    let cx = DecodeContext::new(ClientId(5));
    let archive = open(data.clone(), &cx).unwrap();
    assert_eq!(
        archive
            .entries()
            .iter()
            .map(|entry| entry.path.as_str())
            .collect::<Vec<_>>(),
        ["000.png", "001.png"]
    );
    let meta = &archive.entries()[0].metadata["hg3"];
    assert_eq!(meta["frame_id"], 23);
    assert_eq!(meta["origin"], serde_json::json!([-1, 1]));
    assert_eq!(meta["base"], serde_json::json!([-11, 50]));
    assert_eq!(meta["flags"], 7);
    assert_eq!(meta["decoded_size"], serde_json::json!([3, 2]));
    assert_eq!(
        archive.entries()[1].metadata["hg3"]["frame_id"],
        0x80000001u32
    );
    assert!(archive.entries()[1].metadata["hg3"]["base"].is_null());
    assert!(archive.entries()[1].metadata["hg3"]["flags"].is_null());
    assert_eq!(
        image(archive.as_ref(), 0, &cx).rgba,
        vec![0; 3 * 2 * 4],
        "expanded negative source pixel is clipped"
    );
    let mut raw = cx.clone();
    raw.parameters.insert(
        "cat-system/hg3".into(),
        serde_json::json!({"expand_canvas":false}),
    );
    let archive = open(data, &raw).unwrap();
    assert_eq!(
        archive.entries()[0].metadata["hg3"]["decoded_size"],
        serde_json::json!([1, 1])
    );
    assert_eq!(
        image(archive.as_ref(), 0, &raw).rgba,
        [128; 4],
        "raw frame does not lose pixels outside logical canvas"
    );
}
#[test]
fn padded_rgb24_control_planes_have_opaque_alpha_without_guessing_metadata_flags() {
    let data = file(vec![frame(
        7,
        vec![
            (b"stdinfo\0", stdinfo(24, [0, 0], None, [1, 1], [1, 1]), 0),
            (b"img0000\0", control(), 0),
        ],
    )]);
    let cx = DecodeContext::new(ClientId(5));
    let archive = open(data, &cx).unwrap();
    assert_eq!(image(archive.as_ref(), 0, &cx).rgba, [128, 128, 128, 255]);
    assert_eq!(archive.entries()[0].metadata["hg3"]["color_bits"], 24);
}
#[test]
fn jpeg_alpha_and_standalone_jpeg_use_shared_decoding_and_preserve_top_down_rows() {
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 100)
        .encode(&[120; 12], 2, 2, image::ColorType::Rgb8)
        .unwrap();
    let original = crysirror_image::decode_standard(&jpeg).unwrap();
    let packed = zlib(&[0, 64, 128, 255]);
    let mut alpha = (packed.len() as u32).to_le_bytes().to_vec();
    alpha.extend(4u32.to_le_bytes());
    alpha.extend(packed);
    let std = stdinfo(24, [1, 0], Some([0, 1]), [2, 2], [3, 2]);
    let data = file(vec![
        frame(
            2,
            vec![
                (b"stdinfo\0", std.clone(), 0),
                (b"img_jpg\0", jpeg.clone(), 9),
                (b"img_al\0\0", alpha, 0),
            ],
        ),
        frame(4, vec![(b"stdinfo\0", std, 0), (b"img_jpg\0", jpeg, 0)]),
    ]);
    let cx = DecodeContext::new(ClientId(5));
    let archive = open(data, &cx).unwrap();
    let masked = image(archive.as_ref(), 0, &cx);
    let color = image(archive.as_ref(), 1, &cx);
    assert_eq!([masked.width, masked.height], [3, 2]);
    for (index, alpha) in [0, 64, 128, 255].into_iter().enumerate() {
        let at = ((index / 2) * 3 + index % 2 + 1) * 4;
        assert_eq!(
            &masked.rgba[at..at + 3],
            &original.rgba[index * 4..index * 4 + 3]
        );
        assert_eq!(masked.rgba[at + 3], alpha);
        assert_eq!(color.rgba[at + 3], 255);
    }
}
#[test]
fn invalid_tag_links_and_duplicate_headers_fail_before_pixel_publication() {
    let cx = DecodeContext::new(ClientId(5));
    let std = stdinfo(32, [0, 0], None, [1, 1], [1, 1]);
    let mut bad = file(vec![frame(
        1,
        vec![(b"stdinfo\0", std.clone(), 0), (b"img0000\0", control(), 0)],
    )]);
    bad[28..32].copy_from_slice(&16u32.to_le_bytes());
    assert!(open(bad, &cx).is_err());
    let duplicate = file(vec![frame(
        1,
        vec![
            (b"stdinfo\0", std.clone(), 0),
            (b"stdinfo\0", std, 0),
            (b"img0000\0", control(), 0),
        ],
    )]);
    assert!(open(duplicate, &cx).is_err());
    let mut invalid = cx.clone();
    invalid.parameters.insert(
        "cat-system/hg3".into(),
        serde_json::json!({"expand_canvas":0}),
    );
    let data = file(vec![frame(
        1,
        vec![
            (b"stdinfo\0", stdinfo(32, [0, 0], None, [1, 1], [1, 1]), 0),
            (b"img0000\0", control(), 0),
        ],
    )]);
    assert!(open(data, &invalid).is_err());
}

#[test]
fn native_attributes_and_mode_words_remain_raw_metadata_without_inferred_effects() {
    let mut attribute = Vec::new();
    for word in [-5i32, 7, 100, 32, -16711936] {
        attribute.extend(word.to_le_bytes());
    }
    let data = file(vec![frame(
        1,
        vec![
            (b"stdinfo\0", stdinfo(32, [0, 0], None, [1, 1], [1, 1]), 0),
            (b"ats0023\0", attribute, 2),
            (b"imgmode\0", 3u32.to_le_bytes().to_vec(), 0),
            (b"cptype\0\0", 0x80000000u32.to_le_bytes().to_vec(), 0),
            (b"img0000\0", control(), 0),
        ],
    )]);
    let cx = DecodeContext::new(ClientId(5));
    let archive = open(data, &cx).unwrap();
    let metadata = &archive.entries()[0].metadata["hg3"];
    assert_eq!(metadata["image_mode"], 3);
    assert_eq!(metadata["compression_type"], 0x80000000u32);
    assert_eq!(
        metadata["attributes"],
        serde_json::json!([{"id":23,"rect":[-5,7,100,32],"color":0xff00ff00u32}])
    );
    assert_eq!(
        image(archive.as_ref(), 0, &cx).rgba,
        [128; 4],
        "catalog mode words do not guess renderer operations"
    );
}

fn native_context() -> DecodeContext {
    let mut cx = DecodeContext::new(ClientId(5));
    cx.parameters.insert(
        "cat-system/hg3".into(),
        serde_json::json!({"expand_canvas":false,"pixel_profile":"native"}),
    );
    cx
}
// An authored inverse fixture writer. It starts from requested display pixels,
// then produces bottom-up native packed rows, deltas, the four bit planes and
// one all-literal run; no game bytes or original executable are involved.
fn segment(start: u32, width: u32, height: u32, depth: u32, rgba: &[u8]) -> Vec<u8> {
    assert_eq!(rgba.len(), width as usize * height as usize * 4);
    let channels = depth as usize / 8;
    let stride = width as usize * channels;
    let mut packed = Vec::new();
    for row in rgba.chunks_exact(width as usize * 4).rev() {
        for pixel in row.chunks_exact(4) {
            packed.extend([pixel[2], pixel[1], pixel[0]]);
            if channels == 4 {
                packed.push(pixel[3]);
            }
        }
    }
    let mut delta = packed.clone();
    for y in 1..height as usize {
        for x in 0..stride {
            delta[y * stride + x] =
                packed[y * stride + x].wrapping_sub(packed[(y - 1) * stride + x]);
        }
    }
    for x in channels..stride {
        delta[x] = packed[x].wrapping_sub(packed[x - channels]);
    }
    delta.resize((delta.len() + 3) & !3, 0);
    let plane = delta.len() / 4;
    let mut planes = vec![0u8; delta.len()];
    for (i, chunk) in delta.chunks_exact(4).enumerate() {
        let mut word = 0u32;
        for (j, &d) in chunk.iter().enumerate() {
            let zz = if d < 128 { d * 2 } else { (255 - d) * 2 + 1 };
            word |= u32::from(zz) << (j * 8);
        }
        for p in 0..4 {
            let spread = |v: u32| {
                let mut out = 0;
                for shift in [6, 4, 2, 0] {
                    out = (out << 6) | (v & (3 << shift));
                }
                out << (6 - p * 2)
            };
            let mask = spread(255);
            planes[p * plane + i] = (0..=255).find(|&v| spread(v) == (word & mask)).unwrap() as u8;
        }
    }
    let mut bits = vec![true];
    gamma(&mut bits, planes.len() as u32);
    gamma(&mut bits, planes.len() as u32);
    let ctl: Vec<u8> = bits
        .chunks(8)
        .map(|chunk| {
            chunk
                .iter()
                .enumerate()
                .fold(0, |v, (i, b)| v | (u8::from(*b) << i))
        })
        .collect();
    let data = zlib(&planes);
    let control = zlib(&ctl);
    let mut out = Vec::new();
    for word in [
        start,
        height,
        data.len() as u32,
        planes.len() as u32,
        control.len() as u32,
        ctl.len() as u32,
    ] {
        out.extend(word.to_le_bytes());
    }
    out.extend(data);
    out.extend(control);
    out
}
fn decoded_result(
    data: Vec<u8>,
    cx: &DecodeContext,
) -> anyhow::Result<crysirror_image::ImageSurface> {
    let archive = open(data, cx)?;
    let bytes = read_all(archive.open(0, cx)?.as_ref(), cx.limits.max_entry_size)?;
    Ok(crysirror_image::decode_standard(&bytes)?)
}
#[test]
fn native_packed_rgb24_rows_and_rgba32_segments_preserve_asymmetric_pixels() {
    let pixels = [
        11, 29, 201, 255, 72, 129, 5, 255, 202, 38, 91, 255, 254, 3, 66, 255, 19, 211, 44, 255, 90,
        172, 243, 255,
    ];
    let cx = native_context();
    let data = file(vec![frame(
        13,
        vec![
            (
                b"stdinfo\0",
                stdinfo(24, [0, 0], Some([0, 0]), [1, 6], [1, 6]),
                0,
            ),
            (b"img0000\0", segment(0, 1, 6, 24, &pixels), 0),
        ],
    )]);
    assert_eq!(decoded_result(data, &cx).unwrap().rgba, pixels);
    let pixels = [
        11, 29, 201, 41, 72, 129, 5, 91, 202, 38, 91, 121, 254, 3, 66, 231,
    ];
    let data = file(vec![frame(
        13,
        vec![
            (
                b"stdinfo\0",
                stdinfo(32, [0, 0], Some([-2, 7]), [2, 2], [2, 2]),
                0,
            ),
            // Physical tag order differs from numeric native load order.
            (b"img0001\0", segment(1, 2, 1, 32, &pixels[..8]), 3),
            (b"img0000\0", segment(0, 2, 1, 32, &pixels[8..]), 0),
        ],
    )]);
    let archive = open(data, &cx).unwrap();
    assert_eq!(
        archive.entries()[0].metadata["hg3"]["pixel_profile"],
        "native"
    );
    assert_eq!(image(archive.as_ref(), 0, &cx).rgba, pixels);
}
#[test]
fn native_segment_overlap_uses_ordinal_order_and_gaps_fail_visibly() {
    let cx = native_context();
    let a = [3, 7, 201, 90, 99, 111, 17, 65];
    let b = [250, 11, 41, 222];
    let standard = stdinfo(32, [0, 0], Some([0, 0]), [1, 2], [1, 2]);
    let data = file(vec![frame(
        0,
        vec![
            (b"stdinfo\0", standard.clone(), 0),
            (b"img0001\0", segment(0, 1, 1, 32, &b), 0),
            (b"img0000\0", segment(0, 1, 2, 32, &a), 0),
        ],
    )]);
    assert_eq!(
        decoded_result(data, &cx).unwrap().rgba,
        [a[..4].to_vec(), b.to_vec()].concat()
    );
    for tags in [
        vec![(b"img0000\0" as &[u8], segment(0, 1, 1, 32, &b), 0)],
        vec![(b"img0001\0" as &[u8], segment(0, 1, 2, 32, &a), 0)],
        vec![
            (b"img0000\0" as &[u8], segment(0, 1, 2, 32, &a), 0),
            (b"img0000\0", segment(0, 1, 2, 32, &a), 0),
        ],
        vec![(b"img0000\0" as &[u8], segment(2, 1, 1, 32, &b), 0)],
    ] {
        let mut all = vec![(b"stdinfo\0" as &[u8], standard.clone(), 0)];
        all.extend(tags);
        assert!(decoded_result(file(vec![frame(0, all)]), &cx).is_err());
    }
}
#[test]
fn native_control_mode_swaps_only_mode_one_and_profile_is_explicit() {
    let pixels = [15, 39, 210, 71];
    let mut cx = native_context();
    for mode in [0u32, 1, 2] {
        let data = file(vec![frame(
            0,
            vec![
                (
                    b"stdinfo\0",
                    stdinfo(32, [0, 0], Some([0, 0]), [1, 1], [1, 1]),
                    0,
                ),
                (b"imgmode\0", mode.to_le_bytes().to_vec(), 0),
                (b"img0000\0", segment(0, 1, 1, 32, &pixels), 0),
            ],
        )]);
        assert_eq!(
            decoded_result(data, &cx).unwrap().rgba,
            if mode == 1 { [210, 39, 15, 71] } else { pixels }
        );
    }
    let incomplete = file(vec![frame(
        0,
        vec![
            (b"stdinfo\0", stdinfo(32, [0, 0], None, [1, 1], [1, 1]), 0),
            (b"img0000\0", segment(0, 1, 1, 32, &pixels), 0),
        ],
    )]);
    assert!(open(incomplete, &cx).is_err());
    cx.parameters.get_mut("cat-system/hg3").unwrap()["pixel_profile"] = serde_json::json!("guess");
    let complete = file(vec![frame(
        0,
        vec![
            (
                b"stdinfo\0",
                stdinfo(32, [0, 0], Some([0, 0]), [1, 1], [1, 1]),
                0,
            ),
            (b"img0000\0", segment(0, 1, 1, 32, &pixels), 0),
        ],
    )]);
    assert!(open(complete, &cx).is_err());
}
#[test]
fn native_jpeg_modes_and_depth_specific_alpha_follow_the_explicit_profile() {
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 100)
        .encode(&[201, 33, 9, 91, 72, 12], 2, 1, image::ColorType::Rgb8)
        .unwrap();
    let original = crysirror_image::decode_standard(&jpeg).unwrap();
    let compressed = zlib(&[23, 191]);
    let mut alpha = (compressed.len() as u32).to_le_bytes().to_vec();
    alpha.extend(2u32.to_le_bytes());
    alpha.extend(compressed);
    let cx = native_context();
    for depth in [24, 32] {
        for mode in [None, Some(0u32), Some(1), Some(2)] {
            let mut tags = vec![
                (
                    b"stdinfo\0" as &[u8],
                    stdinfo(depth, [0, 0], Some([0, 0]), [2, 1], [2, 1]),
                    0,
                ),
                (b"img_jpg\0", jpeg.clone(), 0),
                (b"img_al\0\0", alpha.clone(), 0),
            ];
            if let Some(mode) = mode {
                tags.push((b"imgmode\0", mode.to_le_bytes().to_vec(), 0));
            }
            let actual = decoded_result(file(vec![frame(0, tags)]), &cx).unwrap();
            let mut expected = original.rgba.clone();
            for (i, pixel) in expected.chunks_exact_mut(4).enumerate() {
                if mode.is_some_and(|v| v != 1) {
                    pixel.swap(0, 2);
                }
                if depth == 32 {
                    pixel[3] = [23, 191][i];
                }
            }
            assert_eq!(actual.rgba, expected, "depth={depth} mode={mode:?}");
        }
    }
}
#[test]
fn native_webp_retains_embedded_alpha_and_uses_native_color_mode() {
    use image::ImageEncoder;
    let pixels = [201, 33, 9, 71, 19, 77, 221, 191];
    let mut webp = Vec::new();
    image::codecs::webp::WebPEncoder::new_lossless(&mut webp)
        .write_image(&pixels, 2, 1, image::ColorType::Rgba8)
        .unwrap();
    let cx = native_context();
    for mode in [None, Some(0u32), Some(1), Some(2)] {
        let mut tags = vec![
            (
                b"stdinfo\0" as &[u8],
                stdinfo(32, [0, 0], Some([0, 0]), [2, 1], [2, 1]),
                0,
            ),
            (b"img_wbp\0", webp.clone(), 0),
            // Native WebP branch does not read separate alpha, even malformed.
            (b"img_al\0\0", vec![1, 2, 3], 0),
        ];
        if let Some(mode) = mode {
            tags.push((b"imgmode\0", mode.to_le_bytes().to_vec(), 0));
        }
        let actual = decoded_result(file(vec![frame(0, tags)]), &cx).unwrap();
        let mut expected = pixels;
        if mode != Some(1) {
            for pixel in expected.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }
        }
        assert_eq!(actual.rgba, expected, "mode={mode:?}");
    }
}

#[test]
fn native_overlapping_segments_have_one_cumulative_decode_work_budget() {
    let make = || {
        let names = (0..64)
            .map(|i| format!("img{i:04}\0").into_bytes())
            .collect::<Vec<_>>();
        let pixels = [3, 29, 201, 77].repeat(16 * 16);
        let packed = segment(0, 16, 16, 32, &pixels);
        let mut tags = vec![(
            b"stdinfo\0".as_slice(),
            stdinfo(32, [0, 0], Some([0, 0]), [16, 16], [16, 16]),
            0,
        )];
        tags.extend(
            names
                .iter()
                .map(|name| (name.as_slice(), packed.clone(), 0)),
        );
        file(vec![frame(0, tags)])
    };
    let normal = native_context();
    assert_eq!(
        decoded_result(make(), &normal).unwrap().rgba,
        [3, 29, 201, 77].repeat(16 * 16)
    );
    let mut constrained = normal.clone();
    constrained.limits.max_resident_bytes = 16 * 1024;
    let error = decoded_result(make(), &constrained).unwrap_err();
    assert!(
        format!("{error:#}").contains("HG3 frame decode work byte limit"),
        "{error:#}"
    );
}

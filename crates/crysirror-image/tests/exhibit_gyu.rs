// SPDX-License-Identifier: MPL-2.0
use crysirror_image::DecoderRegistry;

fn stored(depth: u32, width: u32, palette: &[u8], colors: &[u8], alpha: &[u8]) -> Vec<u8> {
    let mut bytes = b"GYU\x1a\0\0\0\x01".to_vec();
    for field in [
        u32::MAX,
        depth,
        width,
        1,
        colors.len() as u32,
        alpha.len() as u32,
        (palette.len() / 4) as u32,
    ] {
        bytes.extend(field.to_le_bytes());
    }
    bytes.extend_from_slice(palette);
    bytes.extend_from_slice(colors);
    bytes.extend_from_slice(alpha);
    bytes
}

#[test]
fn indexed_pixels_use_bgrx_palette_and_expand_alpha() -> anyhow::Result<()> {
    let palette = [30, 20, 10, 0, 3, 2, 1, 77];
    let bytes = stored(8, 4, &palette, &[1, 0, 0, 1], &[0, 15, 16, 255]);
    let registry = DecoderRegistry::default();
    assert!(
        registry
            .probe("sprite.bin", &bytes)?
            .contains(&"exhibit/gyu")
    );
    assert!(
        !registry
            .probe("sprite.gyu", b"garbage")?
            .contains(&"exhibit/gyu")
    );
    let surface = registry.decode("sprite.bin", &bytes)?;
    assert_eq!(
        surface.rgba,
        [1, 2, 3, 0, 10, 20, 30, 240, 10, 20, 30, 255, 1, 2, 3, 255]
    );
    Ok(())
}

#[test]
fn thirty_two_bit_color_has_an_unused_fourth_byte() -> anyhow::Result<()> {
    let bytes = stored(32, 2, &[], &[1, 3, 5, 0, 2, 4, 6, 173], &[]);
    let surface = DecoderRegistry::default().decode("sprite.gyu", &bytes)?;
    assert_eq!(surface.rgba, [5, 3, 1, 255, 6, 4, 2, 255]);
    Ok(())
}

#[test]
fn truncated_oversized_or_unresolved_images_fail() {
    let registry = DecoderRegistry::default();
    let fixture = stored(8, 1, &[30, 20, 10, 255], &[0, 0, 0, 0], &[3, 0, 0, 0]);
    for end in 0..fixture.len() {
        assert!(
            registry.decode("sprite.gyu", &fixture[..end]).is_err(),
            "prefix {end}"
        );
    }
    for (offset, value, error) in [
        (8, 0u32, "keyed"),
        (16, u32::MAX, "dimensions"),
        (12, 16, "depth"),
        (24, 0, "color length"),
        (28, 2, "compressed alpha"),
        (32, 257, "palette size"),
        (32, 0, "palette size"),
    ] {
        let mut bytes = fixture.clone();
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        let actual = registry.decode("sprite.gyu", &bytes).unwrap_err();
        assert!(format!("{actual:#}").contains(error), "{actual:#}");
    }
    let mut compressed = fixture.clone();
    compressed[6..8].copy_from_slice(&0x1000u16.to_le_bytes());
    assert!(
        format!(
            "{:#}",
            registry.decode("sprite.gyu", &compressed).unwrap_err()
        )
        .contains("compression mode")
    );
    let separate = stored(32, 1, &[], &[0; 4], &[0; 4]);
    assert!(
        format!(
            "{:#}",
            registry.decode("sprite.gyu", &separate).unwrap_err()
        )
        .contains("separate alpha")
    );
    let mut bad_index = fixture;
    bad_index[40] = 1;
    assert!(
        format!(
            "{:#}",
            registry.decode("sprite.gyu", &bad_index).unwrap_err()
        )
        .contains("palette index")
    );
}

#[test]
fn compressed_color_and_alpha_use_separate_dictionaries() -> anyhow::Result<()> {
    // Three BGR literals, then a nine-byte overlapping match.
    let mut bytes = stored(24, 4, &[], &[7, 1, 2, 3, 0xee, 0xf6], &[15, 0, 4, 8, 16]);
    bytes[6..8].copy_from_slice(&0x0400u16.to_le_bytes());
    let surface = DecoderRegistry::default().decode("frame.gyu", &bytes)?;
    assert_eq!(
        surface.rgba,
        [3, 2, 1, 0, 3, 2, 1, 64, 3, 2, 1, 128, 3, 2, 1, 255]
    );
    Ok(())
}

#[test]
fn gyu_interleaves_literal_short_and_long_history_tokens() -> anyhow::Result<()> {
    // A literal B, a short distance-2 match of length two, then a long
    // distance-4 match of length four. The control bits are 1 0000 01.
    let packed = [0, 0, 0, 0, 0, 0x82, 1, 0xfe, 0xff, 0xe2];
    let mut bytes = stored(8, 8, &[7, 0, 0, 0, 0, 9, 0, 0], &packed, &[]);
    bytes[6..8].copy_from_slice(&0x0800u16.to_le_bytes());
    let image = DecoderRegistry::default().decode("sample.gyu", &bytes)?;
    assert_eq!(image.rgba, [0, 0, 7, 255, 0, 9, 0, 255].repeat(4));
    Ok(())
}

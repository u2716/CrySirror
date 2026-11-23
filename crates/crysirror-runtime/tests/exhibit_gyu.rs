// SPDX-License-Identifier: MPL-2.0
use crysirror_core::state::ClientId;
use crysirror_runtime::assets::Assets;

#[test]
fn client_reads_gyu_and_decodes_bottom_up_colors_and_alpha() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let sprites = root.path().join("sprites");
    std::fs::create_dir(&sprites)?;
    // Stored 2 x 2 BGR24. Both planes use a different amount of row padding.
    let mut bytes = b"GYU\x1a\x03\x00\x00\x01".to_vec();
    for value in [u32::MAX, 24, 2, 2, 16, 8, 0] {
        bytes.extend(value.to_le_bytes());
    }
    bytes.extend([60, 50, 40, 90, 80, 70, 99, 99, 3, 2, 1, 6, 5, 4, 98, 98]);
    bytes.extend([150, 200, 97, 97, 0, 17, 96, 96]);
    std::fs::write(sprites.join("panel.lvg"), bytes)?;

    let mut assets = Assets::new(root.path())?;
    let client = ClientId(1);
    assets.register(client, "sprites")?;
    let data = assets.read_for(client, "panel.lvg")?;
    let picture = assets.decode_image(client, "panel.lvg", &data)?;
    assert_eq!((picture.width, picture.height), (2, 2));
    assert_eq!(
        picture.rgba,
        [1, 2, 3, 0, 4, 5, 6, 17, 40, 50, 60, 150, 70, 80, 90, 200]
    );
    Ok(())
}

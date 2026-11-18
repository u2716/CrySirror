// SPDX-License-Identifier: MPL-2.0
//! Malie tiled image manifests. Mip levels halve the logical extent; native
//! texture names may contain Windows separators and remain capability-relative.
//! Shared by image decoding and the explicit DZI archive view.
use anyhow::{ensure, Context, Result};
use crysirror_core::path::{Mount, VfsPath};
use crysirror_image::{
    bitmap::{BitmapRect, BlitOptions, PixelMath},
    rgba_len, ImageDecodeContext, ImageSurface,
};
pub(crate) struct Level {
    pub size: [u32; 2],
    pub tiles: Vec<(u32, u32, String)>,
}
pub(crate) fn parse(bytes: &[u8], limit: usize) -> Result<Vec<Level>> {
    ensure!(bytes.len() <= 1024 * 1024, "DZI manifest byte budget");
    let mut lines = std::str::from_utf8(bytes)?.lines();
    ensure!(lines.next() == Some("DZI"), "DZI signature");
    let pair = |text: &str| -> Result<[u32; 2]> {
        let (a, b) = text.split_once(',').context("DZI dimension pair")?;
        Ok([a.parse()?, b.parse()?])
    };
    let mut size = pair(lines.next().context("DZI image extent")?)?;
    rgba_len(size[0], size[1])?;
    let count = lines.next().context("DZI level count")?.parse::<usize>()?;
    ensure!((1..=16).contains(&count), "DZI mip count budget");
    let mut levels = Vec::new();
    let mut total = 0usize;
    for _ in 0..count {
        let [cols, rows] = pair(lines.next().context("DZI tile grid")?)?;
        ensure!(
            cols > 0 && rows > 0 && cols <= size[0].div_ceil(256) && rows <= size[1].div_ceil(256),
            "DZI tile grid exceeds mip extent"
        );
        let n = (cols as usize)
            .checked_mul(rows as usize)
            .context("DZI tile count overflow")?;
        total = total
            .checked_add(n)
            .context("DZI total tile count overflow")?;
        ensure!(total <= limit, "DZI tile count budget");
        let mut tiles = Vec::new();
        for y in 0..rows {
            let names: Vec<_> = lines.next().context("DZI tile row")?.split(',').collect();
            ensure!(names.len() == cols as usize, "DZI tile row width");
            for (x, name) in names.into_iter().enumerate() {
                if name.is_empty() {
                    continue;
                }
                let name = name.replace('\\', "/");
                let path = VfsPath::new(Mount::Asset, &name)?;
                tiles.push((x as u32 * 256, y * 256, path.relative().to_owned()));
            }
        }
        levels.push(Level { size, tiles });
        size = [size[0].div_ceil(2), size[1].div_ceil(2)];
    }
    ensure!(
        lines.all(|line| line.trim().is_empty()),
        "DZI unexpected trailing records"
    );
    Ok(levels)
}
pub(crate) fn decode(bytes: &[u8], context: &ImageDecodeContext<'_>) -> Result<ImageSurface> {
    let levels = parse(bytes, 65536)?;
    let level = &levels[0];
    let mut output = ImageSurface {
        width: level.size[0],
        height: level.size[1],
        rgba: vec![0; rgba_len(level.size[0], level.size[1])?],
    };
    for (x, y, name) in &level.tiles {
        context.check_cancelled()?;
        let mut tile = None;
        for extension in ["webp", "png", "mgf"] {
            let path = format!("tex/{name}.{extension}");
            if let Some(bytes) = context.try_read_relative(&path)? {
                tile = Some(
                    context
                        .decode_embedded(&path, &bytes)
                        .with_context(|| format!("DZI tile {path:?}"))?,
                );
                break;
            }
        }
        let tile = tile.with_context(|| format!("DZI tile {name:?} is missing"))?;
        ensure!(
            tile.width <= 256 && tile.height <= 256,
            "DZI tile exceeds native 256-pixel cell"
        );
        output.blit(
            &tile,
            BlitOptions {
                source_rect: BitmapRect::new(0, 0, tile.width, tile.height),
                destination: [*x as i32, *y as i32],
                opacity: 255,
                mode: PixelMath::Copy,
                color_key: None,
            },
        )?;
    }
    Ok(output)
}

//! EXE 0x479900 / reference0x447FE0 / Koikake0x434E30 probe suffix
//! loaders until the first successful load. Koikake has only the first four.
//! Table 0x4EB680 is initialized at 0x401300; .mpg creates a movie sprite.
use super::*;

pub(super) const SUFFIXES: &[&str] = &[".pgd", ".mpg", ".tga", ".bmp", ".png", ".jpg"];

pub(super) fn load_sprite_resource(
    manager: &mut ResourceManager,
    movies: &mut MSpriteSystem,
    name: &str,
    extended: bool,
) -> anyhow::Result<(LoadedAsset, DecodedImage, Option<MSpriteHandle>)> {
    let mut last_error = anyhow::anyhow!("sprite resource {name:?} not found");
    let suffixes = if extended { SUFFIXES } else { &SUFFIXES[..4] };
    for suffix in suffixes {
        // The native helper always appends; an explicit extension is not
        // stripped by its caller or by the FILE.DAT string resolver.
        let Ok(mut asset) = manager.open(&format!("{name}{suffix}")) else {
            continue;
        };
        if *suffix == ".mpg" {
            match movies.load_movie(asset.name.clone(), std::mem::take(&mut asset.bytes)) {
                Ok(movie) => {
                    let image = DecodedImage {
                        width: movie.width,
                        height: movie.height,
                        cell_width: movie.width,
                        cell_height: movie.height,
                        offset_x: 0,
                        offset_y: 0,
                        rgba: movie.rgba,
                    };
                    return Ok((asset, image, Some(movie.handle)));
                }
                Err(error) => last_error = error,
            }
        } else {
            match decode_asset_image(manager, &asset) {
                Ok(image) => return Ok((asset, image, None)),
                Err(error) => last_error = error,
            }
        }
    }
    Err(last_error)
}

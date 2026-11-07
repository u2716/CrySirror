// SPDX-License-Identifier: MPL-2.0
//!UD33/412A40 selects4x4 cells from the native493120 phase policy. It is
//!not a monotonic Bayer dissolve: phase7 restores one previously selected
//!cell and phase8 advances two. Shared TileReveal owns pixels and cancellation.
use super::*;
use crysirror_core::image_processing::ImageResource;
pub(super) fn input(
    phase: u32,
    old: Arc<dyn ImageResource>,
    target: Arc<dyn ImageResource>,
) -> Result<ImageInput> {
    ensure!(phase < 256, "SugarPot checker update phase");
    let level = (phase / 16) as u16;
    //First phase selecting each cell, in the native X-major4x4 indexing.
    //These are operation thresholds, not a copied bitmap or guest resource.
    const FIRST: [u8; 16] = [10, 7, 9, 6, 1, 15, 3, 13, 8, 5, 11, 7, 4, 12, 2, 14];
    let mut thresholds = Vec::with_capacity(256 * 150);
    for row in 0..150 {
        for column in 0..256 {
            let index = 4 * (column % 4) + (row % 4);
            thresholds.push(if level == 7 && index == 6 {
                255
            } else {
                FIRST[index] - 1
            });
        }
    }
    Ok(ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(ImageInput::Resource(old)),
            source: Box::new(ImageInput::Resource(target)),
        }),
        effects: serde_json::to_vec(&vec![crysirror_image::effects::ImageEffect::TileReveal {
            spec: crysirror_image::tile_reveal::TileReveal {
                cell: [4, 4],
                grid: [256, 150],
                thresholds,
                value: level,
            },
        }])?
        .into(),
    })
}

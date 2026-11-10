// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native CG tile/difference painter policy over immutable shared image graphs.
//! This file never reads image bytes. Picture clipping, stored-byte copies,
//! ordinary masks and all wipe kernels are engine image operations.
use crate::native_cg_difference::{DifferencePrint, DifferenceState};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    image_processing::{
        ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageJob, ImageResource,
    },
    service::HostServices,
    state::ClientId,
};
use crysirror_image::{effects::ImageEffect, temporal_pattern::PatternRng};
use std::{collections::BTreeMap, sync::Arc};
#[derive(Clone)]
pub(crate) struct CgTile {
    /// Native masked or corner-keyed display lease; opaque blits ignore its
    /// coverage channel without destroying the immutable source generation.
    pub image: Arc<dyn ImageResource>,
    pub source_clip: [i32; 4],
    pub masked: bool,
}
pub(crate) struct CgPrintCandidate {
    pub input: ImageInput,
    pub difference_after: DifferenceState,
    pub random_after: Option<PatternRng>,
    pub tables_after: crate::native_simple_wipe::Tables,
}

/// PrintSimpleWipe reads raw viewport RGB without CharaClip. Prepare a checked
/// RGB view once on a shared worker. A source smaller than the viewport caused
/// native out-of-bounds reads: uncovered bytes are explicitly black here.
/// None similarly defines a never-initialized sabunPic, not a screenshot.
pub(crate) fn prepare_viewport(
    host: &mut dyn HostServices,
    client: ClientId,
    size: [u32; 2],
    image: Option<Arc<dyn ImageResource>>,
) -> Result<ImageJob> {
    ensure!(
        client.0 != 0 && size.iter().all(|n| (1..=16384).contains(n)),
        "NNN CG difference viewport scope"
    );
    let base = ImageInput::Solid {
        size,
        color: [0, 0, 0, 255],
    };
    let input = if let Some(image) = image {
        ensure!(image.client() == client, "NNN foreign CG difference source");
        ImageInput::Composite {
            base: Box::new(base),
            layers: vec![ImageCompositeLayer {
                input: Box::new(ImageInput::Resource(image)),
                source: [0, 0, size[0] as i32, size[1] as i32],
                destination: [0, 0],
                clip: None,
                opacity: 255,
                math: ImageCompositeMath::CopyRgb,
            }],
        }
    } else {
        base
    };
    Ok(host
        .image_metadata(client)?
        .begin_process(input, b"[]".as_slice().into())?)
}

pub(crate) fn plan_print(
    client: ClientId,
    canvas: [u32; 2],
    mut base: ImageInput,
    tiles: &[CgTile],
    columns: u32,
    position: [i64; 2],
    difference: &DifferenceState,
    old_viewport: Option<Arc<dyn ImageResource>>,
    new_viewports: &BTreeMap<usize, Arc<dyn ImageResource>>,
    random: Option<&PatternRng>,
    tables: &crate::native_simple_wipe::Tables,
) -> Result<CgPrintCandidate> {
    ensure!(
        client.0 != 0
            && !tiles.is_empty()
            && tiles.len() <= 256
            && columns > 0
            && columns <= 16
            && tiles.len() % columns as usize == 0,
        "NNN CG tile painter extent"
    );
    ensure!(
        canvas.iter().all(|n| (1..=16384).contains(n))
            && position.iter().all(|n| n.unsigned_abs() <= 1_000_000),
        "NNN CG tile painter coordinates"
    );
    let mut random = random.cloned();
    let mut tables = tables.clone();
    let mut layers = Vec::with_capacity(tiles.len());
    let first_size = tiles[0].image.size();
    for (index, tile) in tiles.iter().enumerate() {
        ensure!(tile.image.client() == client, "NNN CG tile client scope");
        let column = index % columns as usize;
        let row = index / columns as usize;
        match difference.plan(column as u32) {
            DifferencePrint::Wipe {
                kind,
                count,
                duration,
            } => {
                let old = old_viewport
                    .clone()
                    .context("NNN admitted CG difference old viewport absent")?;
                let new = new_viewports
                    .get(&index)
                    .context("NNN admitted CG difference new viewport absent")?
                    .clone();
                ensure!(
                    old.client() == client
                        && new.client() == client
                        && old.size() == canvas
                        && new.size() == canvas,
                    "NNN CG difference ready viewport scope"
                );
                // Even when a later first-column wipe overwrites these pixels,
                // this call still consumes its native RNG draws in tile order.
                let (input, mut effects) = if crate::native_simple_wipe::needs_random(kind) {
                    crate::native_simple_wipe::plan_with_random(
                        kind,
                        old,
                        new,
                        count,
                        duration,
                        random
                            .as_mut()
                            .context("NNN randomized CG wipe requires its native RNG owner")?,
                        &mut tables,
                    )?
                } else {
                    crate::native_simple_wipe::plan(kind, old, new, count, duration)?
                };
                effects.push(ImageEffect::OverrideAlpha { alpha: 255 });
                base = ImageInput::Process {
                    input: Box::new(input),
                    effects: serde_json::to_vec(&effects)?.into(),
                };
                // This wipe writes a complete viewport from its own raw old
                // and new pictures. Earlier ordinary tiles are overwritten;
                // discarding their DTOs avoids cloning a full canvas per tile.
                layers.clear();
            }
            kind => {
                let (rect, point, math) = if kind == DifferencePrint::CopyOpaque {
                    (
                        [0, 0, canvas[0] as i32, canvas[1] as i32],
                        [0, 0],
                        ImageCompositeMath::CopyRgb,
                    )
                } else {
                    // Print uses the first tile's extent for EVERY Blt. A
                    // larger later tile is cropped, never stretched or allowed
                    // to overwrite additional cells beyond that native size.
                    let point = [
                        i32::try_from(column as i64 * i64::from(first_size[0]) - position[0])?,
                        i32::try_from(row as i64 * i64::from(first_size[1]) - position[1])?,
                    ];
                    (
                        [0, 0, first_size[0] as i32, first_size[1] as i32],
                        point,
                        if tile.masked {
                            ImageCompositeMath::MaskSeparate256
                        } else {
                            ImageCompositeMath::CopyRgb
                        },
                    )
                };
                let Some((rect, point)) =
                    crysirror_image::coverage_bounds::intersect_source_window(
                        rect,
                        point,
                        tile.source_clip,
                    )?
                else {
                    continue;
                };
                layers.push(ImageCompositeLayer {
                    input: Box::new(ImageInput::Resource(tile.image.clone())),
                    source: rect,
                    destination: point,
                    clip: None,
                    opacity: 255,
                    math,
                });
            }
        }
    }
    if !layers.is_empty() {
        base = ImageInput::Composite {
            base: Box::new(base),
            layers,
        };
    }
    let mut difference_after = difference.clone();
    difference_after.printed();
    Ok(CgPrintCandidate {
        input: base,
        difference_after,
        random_after: random,
        tables_after: tables,
    })
}

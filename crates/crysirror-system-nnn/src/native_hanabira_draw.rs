// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! HANABIRA native Blt/Trans3/ChangeTranslateBlt selection and clipping. The
//! particle owner already consumed motion/RNG and opacity before this stage.
use crate::{
    image_bridge::NativePictureViews, native_hanabira::Sprite, native_picture_clip::CharaClip,
};
use anyhow::{Result, ensure};
use crysirror_core::image_processing::ImageResource;
use crysirror_image::{
    byte_raster::ByteRasterMath,
    packed_pair_batch::PackedPairBatch,
    packed_pair_blend::PackedPairBlend,
    row_blit::{RowBlit, RowBlitBatch},
};
use std::sync::Arc;
pub(crate) enum Draw {
    Rows {
        source: Arc<dyn ImageResource>,
        spec: RowBlitBatch,
    },
    Pairs {
        source: Arc<dyn ImageResource>,
        spec: PackedPairBatch,
    },
}
pub(crate) fn plan(
    sprites: Vec<Sprite>,
    picture: &NativePictureViews,
    load_ok: bool,
    canvas: [u32; 2],
) -> Result<Option<Draw>> {
    ensure!(sprites.len() <= 300, "NNN petal sprite count");
    let Some(first) = sprites.first() else {
        return Ok(None);
    };
    let paired = first.interpolate;
    ensure!(
        sprites.iter().all(|s| s.interpolate == paired),
        "NNN petal homogeneous animation mode"
    );
    let size = picture.raw.size();
    if paired {
        let mut operations = Vec::with_capacity(sprites.len());
        for sprite in sprites {
            let [x, y, w, h] = sprite.destination;
            let Some(geometry) = crate::native_picture_pair_geometry::pair(
                [x, y],
                [sprite.first, sprite.second],
                [w, h],
                [size; 2],
                [CharaClip::from_bounds(picture.source_clip); 2],
                [0, 0, canvas[0] as i32, canvas[1] as i32],
            )?
            else {
                continue;
            };
            operations.push(PackedPairBlend {
                sources: geometry.sources,
                destination: geometry.destination,
                weights256: sprite
                    .percentages
                    .map(|n| ((n.clamp(0, 100) * 256 + 50) / 100) as u16),
                use_masks: picture.has_mask,
            });
        }
        let spec = PackedPairBatch { operations };
        spec.validate()?;
        // ChangeBlt/Translate do not have Blt/Trans3's m_loadOkFlag guard;
        // retained buffers survive a failed replacement file and still draw.
        return Ok(Some(Draw::Pairs {
            source: if picture.has_mask {
                picture.raw.clone()
            } else {
                picture.opaque.clone()
            },
            spec,
        }));
    }
    if !load_ok {
        return Ok(None);
    }
    let percent = first.percentages[0];
    // TransLucentBlt3 returns immediately without CheckMaskExist. A retained
    // allocation alone does not enable it; the100% Blt instead uses colour key.
    if percent != 100 && !picture.has_mask {
        return Ok(None);
    }
    let source = if picture.has_mask {
        picture.raw.clone()
    } else {
        picture.keyed.clone()
    };
    let mut operations = Vec::with_capacity(sprites.len());
    for sprite in sprites {
        ensure!(
            sprite.percentages[0] == percent,
            "NNN petal opacity consistency"
        );
        let [x, y, w, h] = sprite.destination;
        let Some((src, dst)) = crysirror_image::coverage_bounds::intersect_source_window(
            [sprite.first[0], sprite.first[1], w, h],
            [x, y],
            picture.source_clip,
        )?
        else {
            continue;
        };
        operations.push(RowBlit {
            source: src,
            destination: dst,
            row_offsets: vec![0; src[3] as usize],
            row_math: vec![],
            math: if percent == 100 {
                ByteRasterMath::AlphaWeightedSeparateRgb256 { weight: 256 }
            } else {
                ByteRasterMath::MaskPiecewiseWrapping256 {
                    weight: ((percent.clamp(0, 100) * 256) / 100) as u16,
                }
            },
        });
    }
    let spec = RowBlitBatch { operations };
    spec.validate()?;
    Ok(Some(Draw::Rows { source, spec }))
}

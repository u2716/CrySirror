// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! ANIME19's original picture-bank animation. GetEternalType is2 and
//! GetLayerOrPicType is1; CountIsMax returns FALSE and Init/BeginPrint/End are
//! empty. Cell selection is a native bank lookup, not a sprite-sheet decode.
use anyhow::Result;
use crysirror_core::image_processing::{ImageInput, ImageResource};
use crysirror_image::{
    byte_raster::{ByteRaster, ByteRasterMath},
    effects::ImageEffect,
    row_blit::RowBlit,
    temporal_pattern::PatternRng,
};
use std::sync::Arc;

pub(crate) fn parameters(values: &[i32]) -> [i32; 17] {
    let mut p = [0; 17];
    for i in 0..17 {
        p[i] = values.get(i).copied().unwrap_or(match i {
            0 => 20,
            1 => 4,
            2 => 1,
            4 => 12,
            6 => 100,
            7 => p[6],
            9 => 10,
            // Omitted11..16 were copied from uninitialised stack words;
            // they are unused by the painter. Define zero but retain supplied
            // or original-save words instead of discarding their storage.
            _ => 0,
        });
    }
    p[1] = p[1].max(1);
    p[2] = p[2].max(1);
    p
}
pub(crate) fn validate(p: &[i32; 17], frames: i32) -> Result<()> {
    crate::native_curve::validate(frames, p[9], p[8])
}
pub(crate) fn calcu(p: &[i32; 17], count: &mut i32) {
    if p[10] != 0 {
        let divisor = p[1].wrapping_mul(p[2]).max(1);
        *count %= divisor;
    }
}
pub(crate) fn picture_slot(p: &[i32; 17], count: i32) -> i32 {
    let mut cell = count / p[2].max(1);
    if p[3] != 0 {
        if p[1] > 0 {
            cell %= p[1];
        }
    } else if cell >= p[1] {
        cell = p[1].wrapping_sub(1);
    }
    // Both the native software picture and its geometry use this0..15 clamp,
    // even in a source32 build whose allocation has additional picture slots.
    p[4].wrapping_add(cell).clamp(0, 15)
}

/// Metadata must come from the selected bank AFTER CountUp/Calcu. The current
/// effect's source/destination rectangles are intentionally not used.
pub(crate) struct BankAnimationPicture {
    pub raw: Arc<dyn ImageResource>,
    pub opaque: Arc<dyn ImageResource>,
    pub keyed: Arc<dyn ImageResource>,
    pub has_mask: bool,
    pub source: [i32; 4],
    pub destination: [i32; 2],
    pub source_clip: [i32; 4],
}
pub(crate) fn image_input(
    p: &[i32; 17],
    count: i32,
    frames: i32,
    base: ImageInput,
    picture: Option<&BankAnimationPicture>,
    random: &mut PatternRng,
) -> Result<Option<ImageInput>> {
    // Every native bank CPicture is allocated by CAllEffect's constructor,
    // including never-loaded slots. A missing decoded lease therefore still
    // evaluates the curve before CPicture's loadOkFlag prevents raster work.
    let percent = if p[4] != 0 {
        crate::native_curve::value_with_random(count, frames, p[7], p[6], p[9], p[8], random)?
    } else {
        0
    };
    let Some(picture) = picture else {
        return Ok(None);
    };
    // The source reads para5 (transFlag) but never uses it. These branches
    // really test the START BUFFER para4, not transFlag or the selected slot.
    let (image, math) = if p[4] == 0 {
        (picture.opaque.clone(), ByteRasterMath::CopyRgb)
    } else {
        if p[4] == 1 {
            // The curve is evaluated even though ordinary masked/keyed Blt
            // discards its result, retaining any native random draw.
            (
                if picture.has_mask {
                    picture.raw.clone()
                } else {
                    picture.keyed.clone()
                },
                ByteRasterMath::AlphaWeightedSeparateRgb256 { weight: 256 },
            )
        } else {
            // CPicture::TransLucentBlt3 returns on a missing mask before
            // drawing; no invented RGB-key or full-coverage fallback here.
            if !picture.has_mask || percent <= 0 {
                return Ok(None);
            }
            (
                picture.raw.clone(),
                ByteRasterMath::MaskPiecewiseWrapping256 {
                    weight: (percent.wrapping_mul(256) / 100) as u16,
                },
            )
        }
    };
    let Some((source, destination)) = crysirror_image::coverage_bounds::intersect_source_window(
        picture.source,
        picture.destination,
        picture.source_clip,
    )?
    else {
        return Ok(None);
    };
    let spec = RowBlit {
        source,
        destination,
        row_offsets: vec![0; source[3] as usize],
        math,
        row_math: vec![],
    };
    spec.validate()?;
    Ok(Some(ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(base),
            source: Box::new(ImageInput::Resource(image)),
        }),
        effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
            spec: ByteRaster::RowBlit { spec },
        }])?
        .into(),
    }))
}

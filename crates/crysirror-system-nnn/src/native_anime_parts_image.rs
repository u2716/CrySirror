// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Animation-part CPicture calls expressed only as shared immutable image DTOs.
use crate::{
    image_bridge::NativePictureViews,
    native_anime_parts::{AnimePartSpec, PartPaint, PartPrint},
};
use anyhow::Result;
use crysirror_core::{image_processing::ImageInput, region_map::clip_region_copy};
use crysirror_image::{
    byte_raster::{ByteRaster, ByteRasterMath},
    effects::ImageEffect,
    packed_pair_blend::PackedPairBlend,
    row_blit::RowBlit,
};

pub(crate) fn image_input(
    spec: &AnimePartSpec,
    print: PartPrint,
    image: &NativePictureViews,
    canvas: [u32; 2],
    base: ImageInput,
) -> Result<Option<ImageInput>> {
    let cells = match print.paint {
        PartPaint::Masked { cell } | PartPaint::Translucent { cell, .. } => [cell; 2],
        PartPaint::Pair { cells, .. } => cells,
    };
    regions_image_input(
        cells.map(|cell| spec.source(cell, image.raw.size()[0])),
        print,
        image,
        canvas,
        base,
    )
}
/// Common CPicture single/pair policy for callers with independently laid out
/// atlas cells (buttons use per-mode source origins, parts use a regular grid).
/// Every colour/coverage operation remains a shared immutable image job.
pub(crate) fn regions_image_input(
    regions: [[i32; 4]; 2],
    print: PartPrint,
    image: &NativePictureViews,
    canvas: [u32; 2],
    base: ImageInput,
) -> Result<Option<ImageInput>> {
    let picture = image.raw.size();
    let bounds = image.source_clip;
    let single = |region, percent: Option<i32>| -> Result<Option<(ImageInput, RowBlit)>> {
        let (source, math) = if let Some(percent) = percent {
            if !image.has_mask || percent <= 0 {
                return Ok(None);
            }
            (
                image.raw.clone(),
                ByteRasterMath::MaskPiecewiseWrapping256 {
                    weight: (percent.wrapping_mul(256) / 100) as u16,
                },
            )
        } else {
            (
                if image.has_mask {
                    image.raw.clone()
                } else {
                    image.keyed.clone()
                },
                ByteRasterMath::AlphaWeightedSeparateRgb256 { weight: 256 },
            )
        };
        let Some((region, at)) = crysirror_image::coverage_bounds::intersect_source_window(
            region,
            print.destination,
            bounds,
        )?
        else {
            return Ok(None);
        };
        let Some((region, at)) = clip_region_copy(
            Some(picture),
            canvas,
            region.map(i64::from),
            at.map(i64::from),
            None,
        )?
        else {
            return Ok(None);
        };
        let row = RowBlit {
            source: region.map(|n| n as i32),
            destination: at.map(|n| n as i32),
            row_offsets: vec![0; region[3] as usize],
            math,
            row_math: vec![],
        };
        row.validate()?;
        Ok(Some((ImageInput::Resource(source), row)))
    };
    match print.paint {
        PartPaint::Masked { .. } | PartPaint::Translucent { .. } => {
            let percent = match print.paint {
                PartPaint::Translucent { percent, .. } => Some(percent),
                _ => None,
            };
            let Some((source, row)) = single(regions[0], percent)? else {
                return Ok(None);
            };
            Ok(Some(ImageInput::Process {
                input: Box::new(ImageInput::Raster {
                    destination: Box::new(base),
                    source: Box::new(source),
                }),
                effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
                    spec: ByteRaster::RowBlit { spec: row },
                }])?
                .into(),
            }))
        }
        PartPaint::Pair { percent, .. } => {
            let Some((a, da)) = crysirror_image::coverage_bounds::intersect_source_window(
                regions[0],
                print.destination,
                bounds,
            )?
            else {
                return Ok(None);
            };
            let Some((b, db)) = crysirror_image::coverage_bounds::intersect_source_window(
                regions[1],
                print.destination,
                bounds,
            )?
            else {
                return Ok(None);
            };
            // ChangeTranslateBlt first unions the two independently trimmed
            // destination rectangles, then clips both physical atlas windows.
            let point = [da[0].min(db[0]), da[1].min(db[1])].map(i64::from);
            let extent = [
                i64::from((da[0] + a[2]).max(db[0] + b[2])) - point[0],
                i64::from((da[1] + a[3]).max(db[1] + b[3])) - point[1],
            ];
            let delta = [
                point[0] - i64::from(print.destination[0]),
                point[1] - i64::from(print.destination[1]),
            ];
            let sa = [
                i64::from(regions[0][0]) + delta[0],
                i64::from(regions[0][1]) + delta[1],
                extent[0],
                extent[1],
            ];
            let sb = [
                i64::from(regions[1][0]) + delta[0],
                i64::from(regions[1][1]) + delta[1],
                extent[0],
                extent[1],
            ];
            let Some((a, da)) = clip_region_copy(Some(picture), canvas, sa, point, None)? else {
                return Ok(None);
            };
            let sb = [
                sb[0] + da[0] - point[0],
                sb[1] + da[1] - point[1],
                a[2],
                a[3],
            ];
            let Some((b, db)) = clip_region_copy(Some(picture), canvas, sb, da, None)? else {
                return Ok(None);
            };
            let a = [a[0] + db[0] - da[0], a[1] + db[1] - da[1], b[2], b[3]];
            let pair = PackedPairBlend {
                sources: [a.map(|n| n as i32), b.map(|n| n as i32)],
                destination: db.map(|n| n as i32),
                use_masks: image.has_mask,
                weights256: percent.map(|n| ((n.clamp(0, 100) * 256 + 50) / 100) as u16),
            };
            pair.validate()?;
            Ok(Some(ImageInput::Process {
                input: Box::new(ImageInput::Ternary {
                    destination: Box::new(base),
                    first: Box::new(ImageInput::Resource(image.raw.clone())),
                    second: Box::new(ImageInput::Resource(image.raw.clone())),
                }),
                effects: serde_json::to_vec(&[ImageEffect::PackedPairBlend { spec: pair }])?.into(),
            }))
        }
    }
}

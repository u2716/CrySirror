// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CHARAMOZAIC139 native source/target clipping and surrounding Blt policy.
//! Mask-dependent random search and pixel painting are a shared Pattern job;
//! only its successful image/RNG receipt can advance the native global stream.
use crate::native_picture_effect::PictureEffectFrame;
use anyhow::{ensure, Result};
use crysirror_core::{
    image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageInput},
    service::PatternRequest,
};
use crysirror_image::{
    masked_block_sample::{BlockSamplePaint, MaskedBlockSample},
    temporal_pattern::PatternRng,
};

pub(crate) fn parameters(values: &[i32], canvas: [u32; 2]) -> [i32; 16] {
    std::array::from_fn(|i| {
        values.get(i).copied().unwrap_or(match i {
            0 => 20,
            1 | 2 => 8,
            6 | 7 => 64,
            11 => canvas[0] as i32,
            12 => canvas[1] as i32,
            13 => 1,
            15 => 100,
            _ => 0,
        })
    })
}
pub(crate) fn validate(p: &[i32; 16]) -> Result<()> {
    ensure!(
        [4, 5, 6, 7, 9, 10, 11, 12]
            .iter()
            .all(|i| p[*i].unsigned_abs() <= 1_000_000),
        "NNN CHARAMOZAIC geometry budget"
    );
    Ok(())
}
pub(crate) enum PaintPlan {
    Image(ImageInput),
    Pattern(PatternRequest),
}
pub(crate) fn image_plan(
    frame: &PictureEffectFrame,
    p: [i32; 16],
    base: ImageInput,
    random: &PatternRng,
) -> Result<Option<PaintPlan>> {
    validate(&p)?;
    if !frame.geometry_assigned || !frame.has_mask {
        return Ok(None);
    }
    let Some(picture) = frame.source.as_ref() else {
        return Ok(None);
    };
    let size = picture.size().map(i64::from);
    let canvas = frame.canvas.map(i64::from);
    let mut origin = [0i64; 2];
    let mut extent = size;
    if p[3] == 1 {
        origin = [size[0] / 2 + i64::from(p[4]), size[1] / 2 + i64::from(p[5])];
        extent = [i64::from(p[6]), i64::from(p[7])];
    }
    let mut all_clip =
        (0..2).any(|axis| origin[axis] >= size[axis] || origin[axis] + extent[axis] <= 0);
    for axis in 0..2 {
        if !all_clip && origin[axis] < 0 {
            extent[axis] += origin[axis];
            origin[axis] = 0;
            if extent[axis] <= 0 {
                all_clip = true;
            }
        }
    }
    for axis in 0..2 {
        if !all_clip && origin[axis] + extent[axis] > size[axis] {
            extent[axis] = size[axis] - origin[axis];
            if extent[axis] <= 0 {
                all_clip = true;
            }
        }
    }
    let mut target = [0i64; 2];
    let mut target_size = canvas;
    if p[8] == 1 {
        target = [i64::from(p[9]), i64::from(p[10])];
        target_size = [i64::from(p[11]), i64::from(p[12])];
    }
    let original_origin = origin;
    let mut point = [
        origin[0] + i64::from(frame.rectangles.destination[0]),
        origin[1] + i64::from(frame.rectangles.destination[1]),
    ];
    let mut source = origin;
    for axis in 0..2 {
        if !all_clip && target[axis] < 0 {
            target_size[axis] += target[axis];
            target[axis] = 0;
            if target_size[axis] <= 0 {
                all_clip = true;
            }
        }
    }
    for axis in 0..2 {
        if !all_clip && target[axis] + target_size[axis] >= canvas[axis] {
            target_size[axis] = canvas[axis] - target[axis];
            if target_size[axis] <= 0 {
                all_clip = true;
            }
        }
    }
    if (0..2).any(|axis| {
        point[axis] >= target[axis] + target_size[axis] || point[axis] + extent[axis] < target[axis]
    }) {
        all_clip = true;
    }
    for axis in 0..2 {
        if !all_clip && point[axis] < target[axis] {
            let delta = target[axis] - point[axis];
            source[axis] += delta;
            extent[axis] -= delta;
            point[axis] = target[axis];
            if extent[axis] <= 0 {
                all_clip = true;
            }
        }
    }
    for axis in 0..2 {
        if !all_clip && point[axis] + extent[axis] >= target[axis] + target_size[axis] {
            extent[axis] -= point[axis] + extent[axis] - (target[axis] + target_size[axis]);
            if extent[axis] <= 0 {
                all_clip = true;
            }
        }
    }
    let mut layers = Vec::with_capacity(4);
    let bounds = frame
        .source_clip
        .unwrap_or([0, 0, size[0] as i32, size[1] as i32]);
    let mut blt = |region: [i64; 4], at: [i64; 2]| -> Result<()> {
        if region[2] <= 0 || region[3] <= 0 {
            return Ok(());
        }
        let Some((region, at)) = crysirror_image::coverage_bounds::intersect_source_window(
            region.map(|n| n as i32),
            at.map(|n| n as i32),
            bounds,
        )?
        else {
            return Ok(());
        };
        layers.push(ImageCompositeLayer {
            input: Box::new(ImageInput::Resource(picture.clone())),
            source: region,
            destination: at,
            clip: None,
            opacity: 255,
            math: ImageCompositeMath::MaskSeparate256,
        });
        Ok(())
    };
    if all_clip {
        blt(
            [0, 0, size[0], size[1]],
            [
                i64::from(frame.rectangles.destination[0]),
                i64::from(frame.rectangles.destination[1]),
            ],
        )?;
    } else {
        // The four ordinary Blts keep their source/put geometry and order:
        // top, bottom, left, right. They do not inherit the mosaic target clip.
        blt(
            [0, 0, size[0], source[1]],
            [point[0] - source[0], point[1] - source[1]],
        )?;
        blt(
            [
                0,
                source[1] + extent[1],
                size[0],
                size[1] - source[1] - extent[1],
            ],
            [point[0] - source[0], point[1] + extent[1]],
        )?;
        blt(
            [0, source[1], source[0], extent[1]],
            [point[0] - source[0], point[1]],
        )?;
        blt(
            [
                source[0] + extent[0],
                source[1],
                size[0] - source[0] - extent[0],
                extent[1],
            ],
            [point[0] + extent[0], point[1]],
        )?;
    }
    let destination = if layers.is_empty() {
        base
    } else {
        ImageInput::Composite {
            base: Box::new(base),
            layers,
        }
    };
    if all_clip || extent.iter().any(|n| *n <= 0) {
        return Ok(Some(PaintPlan::Image(destination)));
    }
    // Zero/negative source block steps would make the original loops hang or
    // overflow. Reject them explicitly before scheduling a bounded worker.
    ensure!(
        p[1] > 0 && p[2] > 0,
        "NNN CHARAMOZAIC nonpositive block step"
    );
    let spec = MaskedBlockSample {
        source: [
            source[0] as u32,
            source[1] as u32,
            extent[0] as u32,
            extent[1] as u32,
        ],
        destination: point.map(|n| n as i32),
        cell: [(p[1] as u32).min(16384), (p[2] as u32).min(16384)],
        search_nonzero: p[13] == 1,
        paint: if p[13] == 0 {
            BlockSamplePaint::PixelCoverage
        } else {
            BlockSamplePaint::SampleCoverage {
                percent: (p[14] != 0).then_some(p[15]),
            }
        },
        paint_source_bounds: Some(bounds),
    };
    spec.validate()?;
    // Native clipping has already produced valid, nonnegative source bounds.
    ensure!(
        source[0] >= original_origin[0].max(0) && source[1] >= original_origin[1].max(0),
        "NNN CHARAMOZAIC source clip moved backwards"
    );
    Ok(Some(PaintPlan::Pattern(PatternRequest::SampleBlocks {
        destination,
        source: ImageInput::Resource(picture.clone()),
        spec: serde_json::to_vec(&spec)?.into(),
        rng: serde_json::to_vec(random)?.into(),
    })))
}

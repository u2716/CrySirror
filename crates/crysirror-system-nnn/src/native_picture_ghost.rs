// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CHARAGHOST110 native curve order and clipped row coverage. The software32
//! branch floors source/destination products separately; Win64 and D3D differ.
//! Shared engine image operations perform every pixel read/write.
use anyhow::{Result, ensure};
use crysirror_image::temporal_pattern::PatternRng;
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GhostDraw {
    pub source: [i32; 4],
    pub destination: [i32; 2],
    pub row_weights: Vec<u16>,
}
pub(crate) fn parameters(p: &[i32]) -> [i32; 16] {
    std::array::from_fn(|i| {
        // Omitted para7..15 were uninitialised source stack words. Explicit
        // zeroes are bounded replacements; all supplied/saved words survive.
        p.get(i).copied().unwrap_or(match i {
            0 => 20,
            1 => 200,
            3 => 100,
            4 => -100,
            _ => 0,
        })
    })
}
pub(crate) fn validate(p: &[i32; 16], frames: i32) -> Result<()> {
    crate::native_curve::validate(frames, p[6], p[5])
}
pub(crate) fn draw_plan(
    p: &[i32; 16],
    count: i32,
    frames: i32,
    picture_size: [u32; 2],
    point: [i32; 2],
    canvas: [u32; 2],
    has_mask: bool,
    random: &mut PatternRng,
) -> Result<Option<GhostDraw>> {
    // Empty allocated picture banks have no mask. Original Print exits here,
    // before evaluating either randomised CCalcuSpeed curve.
    if !has_mask {
        return Ok(None);
    }
    ensure!(
        picture_size
            .iter()
            .chain(&canvas)
            .all(|n| (1..=16384).contains(n)),
        "NNN CHARAGHOST dimensions"
    );
    let up = crate::native_curve::value_with_random(
        count,
        frames,
        p[1].wrapping_mul(256),
        p[2].wrapping_mul(256),
        p[6],
        p[5],
        random,
    )? / 100;
    let down = crate::native_curve::value_with_random(
        count,
        frames,
        p[3].wrapping_mul(256),
        p[4].wrapping_mul(256),
        p[6],
        p[5],
        random,
    )? / 100;
    let Some((source, destination)) = crysirror_core::region_map::clip_region_copy(
        Some(picture_size),
        canvas,
        [0, 0, i64::from(picture_size[0]), i64::from(picture_size[1])],
        point.map(i64::from),
        None,
    )?
    else {
        return Ok(None);
    };
    let source = source.map(|v| v as i32);
    let destination = destination.map(|v| v as i32);
    let mut row_weights = Vec::with_capacity(source[3] as usize);
    for j in 0..source[3] {
        // Native software yy is putY+deltaY+j: a positive placement includes
        // its screen-space Y; a clipped negative placement includes srcY.
        // D3D's j-deltaY formula is a different profile and is not substituted.
        let yy = destination[1].wrapping_add(source[1]).wrapping_add(j);
        let weight =
            up.wrapping_add(down.wrapping_sub(up).wrapping_mul(yy) / picture_size[1] as i32);
        row_weights.push(weight.clamp(0, 256) as u16);
    }
    Ok(Some(GhostDraw {
        source,
        destination,
        row_weights,
    }))
}

pub(crate) fn image_input(
    frame: &crate::native_picture_effect::PictureEffectFrame,
    p: [i32; 16],
    base: crysirror_core::image_processing::ImageInput,
    random: &mut PatternRng,
) -> Result<Option<crysirror_core::image_processing::ImageInput>> {
    use crysirror_core::image_processing::ImageInput;
    use crysirror_image::{
        byte_raster::{ByteRaster, ByteRasterMath},
        effects::ImageEffect,
        row_blit::RowBlit,
    };
    let Some(source) = &frame.source else {
        return Ok(None);
    };
    let Some(plan) = draw_plan(
        &p,
        frame.state.count,
        frame.state.frames,
        source.size(),
        [
            frame.rectangles.destination[0],
            frame.rectangles.destination[1],
        ],
        frame.canvas,
        frame.has_mask,
        random,
    )?
    else {
        return Ok(None);
    };
    let spec = RowBlit {
        source: plan.source,
        destination: plan.destination,
        row_offsets: vec![0; plan.source[3] as usize],
        math: ByteRasterMath::AlphaWeightedSeparateRgb256 { weight: 256 },
        row_math: plan
            .row_weights
            .into_iter()
            .map(|weight| ByteRasterMath::AlphaWeightedSeparateRgb256 { weight })
            .collect(),
    };
    spec.validate()?;
    Ok(Some(ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(base),
            source: Box::new(ImageInput::Resource(source.clone())),
        }),
        effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
            spec: ByteRaster::RowBlit { spec },
        }])?
        .into(),
    }))
}

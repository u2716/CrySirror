// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! TURNNEW140 native parameter/default quirks, curve order and rotated bounds.
//! Shared AffineSampleBlendByte owns inverse sampling, scaled neighbours,
//! colour arithmetic and normal/add/subtract stored-byte composition.
use crate::native_picture_effect::PictureEffectFrame;
use anyhow::Result;
use crysirror_core::image_processing::ImageInput;
use crysirror_image::{
    affine_sample_blend::{AffineSampleBlend, AffineSampleBlendByte, AffineSampleFilter},
    byte_raster::ByteRaster,
    effects::ImageEffect,
    temporal_pattern::PatternRng,
};
pub(crate) fn parameters(values: &[i32]) -> [i32; 36] {
    let mut p = [0; 36];
    for i in 0..36 {
        p[i] = values.get(i).copied().unwrap_or(match i {
            0 => 20,
            4 => p[3],
            7 | 8 | 11 => 100,
            9 => p[7],
            10 => p[8],
            12 => p[11],
            22 => p[21],
            23 => p[22],
            24 => p[23],
            25 => p[24],
            _ => 0,
        });
    }
    // Source START_R inherits END_B. Its duplicated START_G condition never
    // initializes START_B; omitted26 and unused27..35 are defined0, not stack
    // memory. Explicit words and all36 raw save words remain unchanged.
    p
}
pub(crate) fn validate(p: &[i32; 36], frames: i32) -> Result<()> {
    for (kind, subframe) in [(13, 14), (15, 16), (17, 18), (19, 20)] {
        crate::native_curve::validate(frames, p[subframe], p[kind])?;
    }
    Ok(())
}
pub(crate) fn image_input(
    frame: &PictureEffectFrame,
    p: &[i32; 36],
    base: ImageInput,
    rng: &mut PatternRng,
) -> Result<Option<ImageInput>> {
    validate(p, frame.state.frames)?;
    let curve = |start: i32, end: i32, kind: usize, subframe: usize, rng: &mut PatternRng| {
        crate::native_curve::value_with_random(
            frame.state.count,
            frame.state.frames,
            start,
            end,
            p[subframe],
            p[kind],
            rng,
        )
    };
    let percent = curve(p[12], p[11], 17, 18, rng)?;
    if percent <= 0 {
        return Ok(None);
    }
    let scale_x =
        (curve(p[9].wrapping_mul(100), p[7].wrapping_mul(100), 15, 16, rng)? / 100).max(1);
    let scale_y =
        (curve(p[10].wrapping_mul(100), p[8].wrapping_mul(100), 15, 16, rng)? / 100).max(1);
    let color_offset = [
        curve(p[24], p[21], 19, 20, rng)?,
        curve(p[25], p[22], 19, 20, rng)?,
        curve(p[26], p[23], 19, 20, rng)?,
    ];
    if !frame.geometry_assigned {
        return Ok(None);
    }
    let Some(picture) = frame.bank.picture(frame.slot).map(|b| &b.picture) else {
        return Ok(None);
    };
    // Original requires a non-NULL mask pointer before evaluating angle. The
    // prepared masked view represents that admitted storage, not global alpha.
    let Some(mask_storage) = &picture.mask_storage else {
        return Ok(None);
    };
    let size = picture.raw.size();
    let mut angle = curve(p[4].wrapping_mul(10), p[3].wrapping_mul(10), 13, 14, rng)?;
    if angle < 0 {
        angle = angle.wrapping_neg();
        if angle >= 3600 {
            angle %= 3600;
        }
        angle = angle.wrapping_neg().wrapping_add(3600);
    }
    if angle >= 3600 {
        angle %= 3600;
    }
    let radians = f64::from(angle) * (std::f64::consts::PI * 2.0 / 3600.0);
    let cosine = radians.cos();
    let sine = radians.sin();
    // Forward bounds use full doubles; the inverse trig is explicitly cast
    // through FLOAT before promotion back to double, as the source does.
    let inverse_cos = f64::from((-radians).cos() as f32);
    let inverse_sin = f64::from((-radians).sin() as f32);
    let source_center = [
        (size[0] as i32 / 2).wrapping_add(p[5]),
        (size[1] as i32 / 2).wrapping_add(p[6]),
    ];
    let destination_center = [
        frame.rectangles.destination[0].wrapping_add(source_center[0]),
        frame.rectangles.destination[1].wrapping_add(source_center[1]),
    ];
    let scale = [f64::from(scale_x) / 100.0, f64::from(scale_y) / 100.0];
    let inverse = [100.0 / f64::from(scale_x), 100.0 / f64::from(scale_y)];
    let mut minimum = [f64::INFINITY; 2];
    let mut maximum = [f64::NEG_INFINITY; 2];
    for corner in [
        [0, 0],
        [size[0] as i32, 0],
        [0, size[1] as i32],
        [size[0] as i32, size[1] as i32],
    ] {
        let x = f64::from(corner[0].wrapping_sub(source_center[0])) * scale[0];
        let y = f64::from(corner[1].wrapping_sub(source_center[1])) * scale[1];
        let transformed = [
            cosine * x - sine * y + f64::from(destination_center[0]),
            sine * x + cosine * y + f64::from(destination_center[1]),
        ];
        for axis in 0..2 {
            minimum[axis] = minimum[axis].min(transformed[axis]);
            maximum[axis] = maximum[axis].max(transformed[axis]);
        }
    }
    let start = minimum.map(|v| (v as i32).saturating_sub(2).max(0));
    let end = std::array::from_fn::<_, 2, _>(|i| {
        (maximum[i] as i32)
            .saturating_add(2)
            .min(frame.canvas[i] as i32)
    });
    if end[0] <= start[0] || end[1] <= start[1] {
        return Ok(None);
    }
    let spec = AffineSampleBlendByte {
        rectangle: [start[0], start[1], end[0] - start[0], end[1] - start[1]],
        destination_origin: destination_center,
        linear: [[inverse_cos, -inverse_sin], [inverse_sin, inverse_cos]],
        post_scale: inverse,
        translation: source_center,
        sample_bias: [0.5; 2],
        filter: if p[2] == 1 {
            AffineSampleFilter::ScaleNeighbor
        } else {
            AffineSampleFilter::Nearest
        },
        neighbor_step: inverse,
        color_offset,
        percent,
        blend: match p[1] {
            0 => AffineSampleBlend::Normal,
            1 => AffineSampleBlend::Add,
            2 => AffineSampleBlend::Subtract,
            _ => AffineSampleBlend::Copy,
        },
    };
    spec.validate()?;
    Ok(Some(ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(base),
            source: Box::new(ImageInput::Resource(mask_storage.view.clone())),
        }),
        effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
            spec: ByteRaster::AffineSampleBlend { spec },
        }])?
        .into(),
    }))
}
// CountIsMaxFALSE; no Calcu/Init/BeginPrint/End/InitParameter work. Source
// CharaClip and the large commented historical spline/MMX path do not execute.
// A repeated image load retaining mask storage is resource-bank policy, not a
// reason to invent alpha from RGB or move this raster into the native VM.

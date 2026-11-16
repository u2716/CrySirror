// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! MOVIE_WARP105 scalar native curves and distinct field/source centres.
//! Shared RadialTwistSample owns the per-pixel radial map and RGB composition.
use crate::native_picture_effect::PictureEffectFrame;
use anyhow::Result;
use crysirror_core::image_processing::ImageInput;
use crysirror_image::{
    byte_raster::ByteRaster, effects::ImageEffect, radial_twist_sample::RadialTwistSample,
    temporal_pattern::PatternRng,
};
pub(crate) fn parameters(values: &[i32], canvas: [u32; 2]) -> [i32; 16] {
    let mut p = [0; 16];
    for i in 0..16 {
        p[i] = values.get(i).copied().unwrap_or(match i {
            0 => 20,
            1 => canvas[0] as i32 / 2,
            2 => canvas[1] as i32 / 2,
            3 => 180,
            7 | 11 => 100,
            8 => p[7],
            12 => p[11],
            _ => 0,
        });
    }
    p
}
pub(crate) fn validate(p: &[i32; 16], frames: i32) -> Result<()> {
    for (kind, sub) in [(5, 6), (9, 10), (13, 14)] {
        crate::native_curve::validate(frames, p[sub], p[kind])?;
    }
    Ok(())
}
pub(crate) fn image_input(
    frame: &PictureEffectFrame,
    p: &[i32; 16],
    base: ImageInput,
    rng: &mut PatternRng,
) -> Result<Option<ImageInput>> {
    validate(p, frame.state.frames)?;
    let Some(picture) = frame.bank.picture(frame.slot).map(|b| &b.picture) else {
        return Ok(None);
    };
    // Source rejects indexed buffers and CheckMaskExistFALSE BEFORE curves.
    // Script picture slots are the32-bit load profile; ANIMATION's separate
    // indexed buffers are not aliases for this native bank.
    if !picture.has_mask {
        return Ok(None);
    }
    let size = picture.raw.size();
    let phase = crate::native_curve::value_with_random(
        frame.state.count,
        frame.state.frames,
        0,
        10000,
        p[6],
        p[5],
        rng,
    )?;
    let phase = 10000i32.wrapping_sub(phase);
    let percent = crate::native_curve::value_with_random(
        frame.state.count,
        frame.state.frames,
        p[8],
        p[7],
        p[10],
        p[9],
        rng,
    )?
    .clamp(0, 100);
    let scale = crate::native_curve::value_with_random(
        frame.state.count,
        frame.state.frames,
        p[12].wrapping_mul(10),
        p[11].wrapping_mul(10),
        p[14],
        p[13],
        rng,
    )?
    .max(1);
    let twist = p[3].wrapping_mul(phase) / 10000;
    // Native deliberately omits dst from the OUTPUT field origin, then adds
    // dst to the sampled SOURCE centre. Do not recenter both spaces together.
    let source_center = [
        frame.rectangles.destination[0].wrapping_add(p[1]),
        frame.rectangles.destination[1].wrapping_add(p[2]),
    ];
    let spec = RadialTwistSample {
        field_origin: [p[1], p[2]],
        source_center,
        radius_scale: 1.0 / (f64::from(scale) * 0.001),
        twist_degrees: twist,
        reference_radius: (size[0].min(size[1]) / 2).max(1),
        angle_pi: 3.14159,
        coverage_percent: percent as u8,
    };
    spec.validate()?;
    Ok(Some(ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(base),
            source: Box::new(ImageInput::Resource(picture.raw.clone())),
        }),
        effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
            spec: ByteRaster::RadialTwistSample { spec },
        }])?
        .into(),
    }))
}
// Omitted15 was uninitialized; define0 and preserve supplied/raw saved words.
// No InitParameter/Calcu/Init/End work (Calcu reads count only), CountIsMaxFALSE,
// CheckNeedPicPrintFALSE. The large commented forward splat never executes.

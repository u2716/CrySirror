// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! effectSaike.cpp's native parameters and six ordered curve samples.
//! Packed RGB multiplication/wrap and masked drawing are shared engine jobs.
use crate::native_picture_effect::PictureEffectFrame;
use anyhow::Result;
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageInput};
use crysirror_image::{effects::ImageEffect, rgb_lut::RgbLookup, temporal_pattern::PatternRng};
pub(crate) fn parameters(p: &[i32]) -> [i32; 20] {
    let mut out = [0; 20];
    for i in 0..20 {
        let default = match i {
            0 => 20,
            3..=5 => 200,
            6..=8 | 12..=14 => out[i - 3],
            15 => out[1],
            16 => out[2],
            _ => 0,
        };
        // Source copied omitted para17..19 from uninitialised stack storage.
        // Defined zero defaults replace that bug; saved words still round-trip.
        out[i] = p.get(i).copied().unwrap_or(default);
    }
    out
}
pub(crate) fn image_input(
    frame: &PictureEffectFrame,
    p: [i32; 20],
    base: ImageInput,
    random: &mut PatternRng,
) -> Result<Option<ImageInput>> {
    let mut multiplier = [0u16; 3];
    let mut offset = [0u8; 3];
    for c in 0..3 {
        multiplier[c] = (crate::native_curve::value_with_random(
            frame.state.count,
            frame.state.frames,
            p[c + 6].wrapping_mul(256),
            p[c + 3].wrapping_mul(256),
            p[2],
            p[1],
            random,
        )? / 100) as u16;
    }
    for c in 0..3 {
        offset[c] = crate::native_curve::value_with_random(
            frame.state.count,
            frame.state.frames,
            p[c + 12],
            p[c + 9],
            p[16],
            p[15],
            random,
        )? as u8;
    }
    let Some(source) = frame.source.as_ref() else {
        return Ok(None);
    };
    if !frame.has_mask {
        return Ok(None);
    }
    let [x, y, w, h] = frame.rectangles.destination;
    if w <= 0 || h <= 0 {
        return Ok(None);
    }
    let input = ImageInput::Process {
        input: Box::new(ImageInput::Resource(source.clone())),
        effects: serde_json::to_vec(&[ImageEffect::RgbLookup {
            spec: RgbLookup::low_word_affine(multiplier, offset),
        }])?
        .into(),
    };
    Ok(Some(ImageInput::Composite {
        base: Box::new(base),
        layers: vec![ImageCompositeLayer {
            input: Box::new(input),
            source: [frame.rectangles.source[0], frame.rectangles.source[1], w, h],
            destination: [x, y],
            clip: None,
            opacity: 255,
            math: ImageCompositeMath::MaskIncrement256,
        }],
    }))
}

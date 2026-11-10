// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CHARAmaskAppear123: native speed/phase policy and absolute-screen RGB mask.
//! The software32 path differs from the D3D parameter indices and mask layout.
use anyhow::Result;
use crysirror_image::temporal_pattern::PatternRng;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MaskAppearPhase {
    pub multiplier: u16,
    pub offset: u16,
    pub nonnegative_bias: u16,
    pub reverse: bool,
}
pub(crate) fn parameters(p: &[i32]) -> Option<[i32; 8]> {
    // SetParam(FALSE) changes no EFFECT field; CAllEffect still records this
    // attempted class in previousEffect. The dispatcher must preserve both.
    if p.len() < 2 {
        return None;
    }
    Some(std::array::from_fn(|i| {
        p.get(i).copied().unwrap_or(match i {
            0 => 20,
            1 => 13,
            2 => 1,
            6 => 100,
            _ => 0,
        })
    }))
}
pub(crate) fn phase(
    p: [i32; 8],
    count: i32,
    frames: i32,
    random: &mut PatternRng,
) -> Result<MaskAppearPhase> {
    // Native Print evaluates this curve before checking source or mask buffers.
    let c = crate::native_curve::value_with_random(
        count,
        frames.max(1),
        p[7],
        p[6],
        p[4],
        p[3],
        random,
    )?;
    let minus = p[2].wrapping_mul(256);
    let wide = 256i32.wrapping_add(minus);
    let mut ml = c.wrapping_mul(wide) / 100;
    if ml < 0 {
        ml = 0;
    }
    if ml > wide {
        ml = wide;
    }
    ml = ml.wrapping_sub(minus);
    Ok(MaskAppearPhase {
        multiplier: p[2] as u16,
        offset: ml as u16,
        nonnegative_bias: minus as u16,
        reverse: p[5] != 0,
    })
}

pub(crate) fn validate(p: &[i32; 8], frames: i32) -> Result<()> {
    crate::native_curve::validate(frames.max(1), p[4], p[3])
}
pub(crate) fn image_input(
    frame: &crate::native_picture_effect::PictureEffectFrame,
    p: [i32; 8],
    base: crysirror_core::image_processing::ImageInput,
    random: &mut PatternRng,
) -> Result<Option<crysirror_core::image_processing::ImageInput>> {
    use crysirror_core::image_processing::ImageInput;
    use crysirror_image::{effects::ImageEffect, packed_color_mask::PackedColorMask};
    let phase = phase(p, frame.state.count, frame.state.frames, random)?;
    if p[1] == -1 || !frame.has_mask {
        return Ok(None);
    }
    let (Some(source), Some(mask)) = (&frame.source, &frame.second) else {
        return Ok(None);
    };
    let mask_size = mask.size();
    if mask_size[0] < frame.canvas[0] || mask_size[1] < frame.canvas[1] {
        return Ok(None);
    }
    let size = source.size();
    let Some((source_rect, at)) = crysirror_core::region_map::clip_region_copy(
        Some(size),
        frame.canvas,
        [0, 0, i64::from(size[0]), i64::from(size[1])],
        [
            i64::from(frame.rectangles.destination[0]),
            i64::from(frame.rectangles.destination[1]),
        ],
        None,
    )?
    else {
        return Ok(None);
    };
    let spec = PackedColorMask {
        source: source_rect.map(|v| v as i32),
        destination: at.map(|v| v as i32),
        mask: at.map(|v| v as i32),
        reverse: phase.reverse,
        multiplier: phase.multiplier,
        offset: phase.offset,
        nonnegative_bias: phase.nonnegative_bias,
        limit: 255,
    };
    spec.validate()?;
    Ok(Some(ImageInput::Process {
        input: Box::new(ImageInput::Ternary {
            destination: Box::new(base),
            first: Box::new(ImageInput::Resource(source.clone())),
            second: Box::new(ImageInput::Resource(mask.clone())),
        }),
        effects: serde_json::to_vec(&[ImageEffect::PackedColorMask { spec }])?.into(),
    }))
}

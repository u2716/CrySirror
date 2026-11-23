// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! MASKWIPE60/MOVEMASKWIPE101 software32 scalar policy. Original setters,
//! CountIsMax, both complete Print bodies and MMX paths were inspected. D3D
//! paths are distinct profiles, not substitutes for these packed-word rules.
use crate::native_picture_effect::PictureEffectFrame;
use anyhow::{Result, ensure};
use crysirror_core::image_processing::{
    ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageResource,
};
use crysirror_image::{
    effects::ImageEffect, mask_field_pair::MaskFieldPair, temporal_pattern::PatternRng,
};
use std::sync::Arc;

pub(crate) fn parameters(command: i32, values: &[i32]) -> Result<[i32; 10]> {
    ensure!(matches!(command, 60 | 101), "NNN mask-wipe command");
    ensure!(
        values.len() >= 3,
        "NNN mask wipe requires frame, buffer and mask"
    );
    let mut p = [0; 10];
    p[..values.len().min(10)].copy_from_slice(&values[..values.len().min(10)]);
    if command == 60 && values.len() < 4 {
        p[3] = 1;
    }
    if values.len() < 9 {
        p[8] = 100;
    }
    Ok(p)
}
pub(crate) fn validate(command: i32, p: &[i32; 10], frames: i32) -> Result<()> {
    let [kind, sub] = if command == 60 { [4, 5] } else { [3, 4] };
    crate::native_curve::validate(frames.max(1), p[sub], p[kind])
}

fn picture(frame: &PictureEffectFrame, slot: i32) -> Option<(Arc<dyn ImageResource>, [i32; 2])> {
    // GetPicture clamps; GetDstPoint originally indexes unchecked EFFECT
    // storage. Use the same bounded bank for that undefined out-of-range case.
    let slot = slot.clamp(0, frame.bank_limit - 1);
    let p = frame.bank.picture(slot)?;
    let rect = frame.bank.rectangles(slot)?;
    Some((
        p.picture.opaque.clone(),
        [rect.destination[0], rect.destination[1]],
    ))
}
fn fits(image: &Arc<dyn ImageResource>, canvas: [u32; 2]) -> bool {
    image.size()[0] >= canvas[0] && image.size()[1] >= canvas[1]
}
fn copy(
    image: Arc<dyn ImageResource>,
    canvas: [u32; 2],
    point: [i32; 2],
    crop: bool,
) -> ImageInput {
    let size = image.size();
    let at = if crop {
        [
            (-i64::from(point[0])).clamp(0, i64::from(size[0] - canvas[0])) as i32,
            (-i64::from(point[1])).clamp(0, i64::from(size[1] - canvas[1])) as i32,
        ]
    } else {
        [0, 0]
    };
    ImageInput::Composite {
        base: Box::new(ImageInput::Solid {
            size: canvas,
            color: [0, 0, 0, 255],
        }),
        layers: vec![ImageCompositeLayer {
            input: Box::new(ImageInput::Resource(image)),
            source: [at[0], at[1], canvas[0] as i32, canvas[1] as i32],
            destination: [0, 0],
            clip: None,
            opacity: 255,
            math: ImageCompositeMath::CopyRgb,
        }],
    }
}

pub(crate) fn needs_capture(command: i32, p: &[i32; 10]) -> bool {
    let i = if command == 60 { 6 } else { 5 };
    p[i] != 0 || p[i + 1] != 0
}
pub(crate) fn image_input(
    frame: &PictureEffectFrame,
    command: i32,
    p: &[i32; 10],
    capture: &Option<Arc<dyn ImageResource>>,
    rng: &mut PatternRng,
) -> Result<Option<ImageInput>> {
    validate(command, p, frame.state.frames)?;
    let moving = command == 101;
    let (kind, sub, srcget, dstget) = if moving {
        (p[3], p[4], p[5] != 0, p[6] != 0)
    } else {
        (p[4], p[5], p[6] != 0, p[7] != 0)
    };
    let start = if moving {
        p[9].wrapping_mul(512) / 100
    } else {
        p[9]
    };
    let end = if moving {
        p[8].wrapping_mul(512) / 100
    } else {
        p[8]
    };
    // Curve/RNG evaluation precedes resource checks, including early endpoints.
    let c = crate::native_curve::value_with_random(
        frame.state.count,
        frame.state.frames.max(1),
        start,
        end,
        sub,
        kind,
        rng,
    )?;
    let from = if srcget {
        capture.clone().map(|i| (i, [0, 0]))
    } else {
        picture(frame, frame.slot)
    };
    let Some((first, first_point)) = from else {
        return Ok(None);
    };
    if !fits(&first, frame.canvas) {
        return Ok(None);
    }
    if moving && c == 0 {
        // Native endpoint Put(0,0,FALSE) ignores the effect's destination crop.
        return Ok(Some(copy(first, frame.canvas, [0, 0], false)));
    }
    let to = if dstget {
        capture.clone().map(|i| (i, [0, 0]))
    } else if p[1] != -1 {
        picture(frame, p[1])
    } else {
        None
    };
    let Some((second, second_point)) = to else {
        return Ok(None);
    };
    if !fits(&second, frame.canvas) {
        return Ok(None);
    }
    if moving && c >= 512 {
        return Ok(Some(copy(second, frame.canvas, [0, 0], false)));
    }
    if p[2] == -1 {
        return Ok(None);
    }
    let Some((mask, mask_point)) = picture(frame, p[2]) else {
        return Ok(None);
    };
    if !fits(&mask, frame.canvas) {
        return Ok(None);
    }
    let spec = if moving {
        let mut alpha = (c / 2).clamp(0, 256);
        if p[7] != 0 {
            alpha = if alpha < 128 { 0 } else { 256 };
        }
        let amplitude = if c > 256 { 512i32.wrapping_sub(c) } else { c };
        MaskFieldPair::Displacement {
            amplitude: amplitude.clamp(0, 256) as u16,
            second_weight128: (alpha >> 1) as u16,
        }
    } else {
        let bias = p[3].wrapping_mul(256);
        let wide = 256i32.wrapping_add(bias);
        let mut offset = c.wrapping_mul(wide) / 100;
        if offset < 0 {
            offset = 0;
        }
        if offset > wide {
            offset = wide;
        }
        offset = offset.wrapping_sub(bias);
        MaskFieldPair::ColorCoverage {
            multiplier: p[3] as u16,
            offset: offset as u16,
            nonnegative_bias: bias as u16,
        }
    };
    spec.validate()?;
    Ok(Some(ImageInput::Process {
        input: Box::new(ImageInput::Ternary {
            destination: Box::new(copy(first, frame.canvas, first_point, true)),
            first: Box::new(copy(second, frame.canvas, second_point, true)),
            second: Box::new(copy(mask, frame.canvas, mask_point, true)),
        }),
        effects: serde_json::to_vec(&[ImageEffect::MaskFieldPair { spec }])?.into(),
    }))
}
// CountIsMax sets count=countMax and returnsTRUE, but all command/sibling edits
// are commented out. Calcu, Init, BeginPrint and End have no active work;
// CheckNeedPicPrintFALSE, raw save retains exactly10parameter words.

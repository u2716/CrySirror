// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! STRETCHBLT65 software32 geometry, native RNG ordering and colour policy.
//! D3D has different rounding and omits colour curves; it is a separate profile.
use crate::native_stretch_blit::{StretchMath, StretchPlan, native_fixed};
use anyhow::{Result, ensure};
use crysirror_image::temporal_pattern::PatternRng;
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StretchPhase {
    pub scale: [f64; 2],
    pub color: Option<([u8; 3], [u8; 3])>,
}
pub(crate) fn parameters(p: &[i32]) -> [i32; 25] {
    let mut out = [0; 25];
    for i in 0..25 {
        out[i] = p.get(i).copied().unwrap_or(match i {
            0 => 20,
            3 | 4 | 10 => 100,
            5 => out[3],
            6 => out[4],
            11 => out[10],
            16 => out[15],
            17 => out[16],
            18 => out[15],
            19 => out[16],
            20 => out[17],
            _ => 0,
        });
    }
    // Original omitted para23/24 were uninitialised stack words. Zero is the
    // defined default; supplied and saved words still round-trip unchanged.
    out
}
pub(crate) fn validate(p: &[i32; 25], frames: i32) -> Result<()> {
    for (sub, kind) in [(p[22], p[21]), (p[8], p[7]), (p[13], p[12])] {
        crate::native_curve::validate(frames, sub, kind)?;
    }
    Ok(())
}
fn packed(color: [i32; 3]) -> [u8; 3] {
    let value = color[0].wrapping_shl(16) | color[1].wrapping_shl(8) | color[2];
    [(value >> 16) as u8, (value >> 8) as u8, value as u8]
}
pub(crate) fn phase(
    p: [i32; 25],
    count: i32,
    frames: i32,
    random: &mut PatternRng,
) -> Result<StretchPhase> {
    // Software Print always evaluates all RGB curves before scale, including
    // when colour is disabled or pic==-1. Do not elide their random draws.
    let mut rgb = [0; 3];
    for c in 0..3 {
        rgb[c] = crate::native_curve::value_with_random(
            count,
            frames,
            p[18 + c],
            p[15 + c],
            p[22],
            p[21],
            random,
        )?;
    }
    let mut scale = [0.0; 2];
    for axis in 0..2 {
        let value = crate::native_curve::value_with_random(
            count,
            frames,
            p[5 + axis].wrapping_mul(100),
            p[3 + axis].wrapping_mul(100),
            p[8],
            p[7],
            random,
        )?;
        scale[axis] = (f64::from(value) / 10000.0).max(0.001);
    }
    let color = (p[14] != 0).then(|| {
        (
            packed(rgb.map(|n| n.max(0))),
            packed(rgb.map(|n| n.wrapping_neg().max(0))),
        )
    });
    Ok(StretchPhase { scale, color })
}
pub(crate) fn draw_plan(
    p: [i32; 25],
    phase: StretchPhase,
    count: i32,
    frames: i32,
    picture: [u32; 2],
    point: [i32; 2],
    canvas: [u32; 2],
    has_mask: bool,
    random: &mut PatternRng,
) -> Result<Option<StretchPlan>> {
    ensure!(
        picture
            .iter()
            .chain(&canvas)
            .all(|v| (1..=16384).contains(v)),
        "NNN STRETCHBLT dimensions"
    );
    let mut source_origin = [0.0; 2];
    let mut source_extent = picture.map(f64::from);
    let mut bounds = [[0.0; 2]; 2];
    for axis in 0..2 {
        let center = f64::from(point[axis].wrapping_add(canvas[axis] as i32 / 2));
        let pivot = f64::from((picture[axis] as i32 / 2).wrapping_add(p[1 + axis]));
        let low = phase.scale[axis] * (-pivot) + center;
        let high = phase.scale[axis] * (f64::from(picture[axis]) - pivot) + center;
        if low >= f64::from(canvas[axis]) || high < 0.0 {
            return Ok(None);
        }
        bounds[axis] = [low, high];
        if low < 0.0 {
            let crop = -low / phase.scale[axis];
            source_origin[axis] += crop;
            source_extent[axis] -= crop;
            bounds[axis][0] = 0.0;
        }
        if high > f64::from(canvas[axis]) {
            source_extent[axis] -= (high - f64::from(canvas[axis])) / phase.scale[axis];
            bounds[axis][1] = f64::from(canvas[axis]);
        }
        source_extent[axis] = source_extent[axis].max(0.0);
    }
    let destination = [
        bounds[0][0] as i32,
        bounds[1][0] as i32,
        (bounds[0][1] as i32).wrapping_sub(bounds[0][0] as i32),
        (bounds[1][1] as i32).wrapping_sub(bounds[1][0] as i32),
    ];
    if destination[2] <= 0 || destination[3] <= 0 {
        return Ok(None);
    }
    // The transparency curve is evaluated only after picture/geometry clipping
    // succeeds, even for an opaque or RGB-key mode that ignores its result.
    let weight = (crate::native_curve::value_with_random(
        count,
        frames,
        p[11].wrapping_mul(256),
        p[10].wrapping_mul(256),
        p[13],
        p[12],
        random,
    )? / 100)
        .clamp(0, 256);
    let mode = if p[9] == 2 && !has_mask { 1 } else { p[9] };
    let math = match mode {
        0 | 1 => StretchMath::Copy,
        2 => StretchMath::CoverageSeparate {
            weight: weight as u32,
            opaque_copy: weight == 256,
        },
        _ => {
            // Original MODE3 dereferences a null mask when an out-of-band
            // transFlag reaches it. A full-coverage plane is the defined
            // replacement; caller supplies opaque coverage in that case.
            StretchMath::CoverageSeparate {
                weight: weight as u32,
                opaque_copy: false,
            }
        }
    };
    Ok(Some(StretchPlan {
        destination,
        source_q16: source_origin.map(native_fixed),
        source_step_q16: std::array::from_fn(|axis| {
            native_fixed(source_extent[axis] / f64::from(destination[axis + 2]))
        }),
        math,
        corner_rgb_key: mode == 1,
    }))
}

pub(crate) fn image_input(
    frame: &crate::native_picture_effect::PictureEffectFrame,
    p: [i32; 25],
    base: crysirror_core::image_processing::ImageInput,
    random: &mut PatternRng,
) -> Result<Option<crysirror_core::image_processing::ImageInput>> {
    let phase = phase(p, frame.state.count, frame.state.frames, random)?;
    if !frame.geometry_assigned {
        return Ok(None);
    }
    let Some(source) = &frame.source else {
        return Ok(None);
    };
    let Some(plan) = draw_plan(
        p,
        phase,
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
    Ok(Some(crate::native_picture_sampling::fixed_input(
        base,
        source.clone(),
        plan,
        phase.color,
    )?))
}

// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! MEISAI72: original silhouette shifts the already-painted backdrop to the
//! left. Character RGB is never drawn. Calcu has no private state and both
//! CountIsMax/CheckNeedPicPrint return FALSE in effectMeisai.cpp.
use anyhow::Result;
use crysirror_image::{masked_backdrop_shift::MaskedBackdropShift, temporal_pattern::PatternRng};

pub(crate) fn parameters(values: &[i32]) -> [i32; 16] {
    let mut p = [0; 16];
    for i in 0..16 {
        // Omitted unused para7..15 were uninitialised C++ stack words.
        p[i] = values.get(i).copied().unwrap_or(match i {
            0 => 20,
            1 => 10,
            2 => 5,
            3 => p[1],
            4 => p[2],
            _ => 0,
        });
    }
    p
}
pub(crate) fn validate(p: &[i32; 16], frames: i32) -> Result<()> {
    crate::native_curve::validate(frames, p[6], p[5])
}
pub(crate) fn draw_plan(
    p: &[i32; 16],
    count: i32,
    frames: i32,
    canvas: [u32; 2],
    picture: [u32; 2],
    clip: [i32; 4],
    point: [i32; 2],
    assigned: bool,
    has_mask: bool,
    random: &mut PatternRng,
) -> Result<Option<MaskedBackdropShift>> {
    if !assigned || !has_mask {
        return Ok(None);
    }
    let Some((mut mask, destination)) =
        crate::native_picture_rainbow::clipped_geometry(canvas, picture, clip, point)?
    else {
        return Ok(None);
    };
    let delta_x =
        crate::native_curve::value_with_random(count, frames, p[3], p[1], p[6], p[5], random)?
            .max(0);
    // Native evaluates the second curve, then overwrites deltaY with zero.
    // Its random draw is observable even when the horizontal shift is too wide.
    let _delta_y =
        crate::native_curve::value_with_random(count, frames, p[4], p[2], p[6], p[5], random)?;
    if delta_x >= mask[2] {
        return Ok(None);
    }
    mask[2] -= delta_x;
    let spec = MaskedBackdropShift {
        mask,
        destination,
        offset: [delta_x, 0],
        channel: 3,
        threshold: 1,
    };
    spec.validate()?;
    Ok(Some(spec))
}

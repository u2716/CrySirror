// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! BGBOKASHI126, effectBGBokashi.cpp: shares125's curve and expanded-trim
//! geometry, but RGB sample counts ignore masks and normalise by32768/count.
//! The unused para8..15 C++ stack values have deterministic zero defaults.
use crate::native_picture_chara_blur::{self, BlurDraw};
use anyhow::Result;
use crysirror_image::{native_box_blur::NativeBoxBlurMath, temporal_pattern::PatternRng};

pub(crate) fn parameters(values: &[i32]) -> [i32; 16] {
    native_picture_chara_blur::parameters(values)
}
pub(crate) fn validate(p: &[i32; 16], frames: i32) -> Result<()> {
    native_picture_chara_blur::validate(p, frames)
}
pub(crate) fn draw_plan(
    p: &[i32; 16],
    count: i32,
    frames: i32,
    canvas: [u32; 2],
    picture: [u32; 2],
    clip: [i32; 4],
    point: [i32; 2],
    size: [i32; 2],
    random: &mut PatternRng,
) -> Result<Option<BlurDraw>> {
    let window = native_picture_chara_blur::window(p, count, frames, random)?;
    // Original Blt(FALSE) ignores the mask even for the1x1 fast path, and
    // its larger-window MMX uses pixel counts rather than alpha sums.
    native_picture_chara_blur::geometry(
        window,
        canvas,
        picture,
        clip,
        point,
        size,
        false,
        NativeBoxBlurMath::CoveredRgb,
        false,
    )
}

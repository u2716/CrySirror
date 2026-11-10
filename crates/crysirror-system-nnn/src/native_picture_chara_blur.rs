// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CHARABOKASHI125: native curve/trim/window policy. The shared binary blur
//! owns packed-byte sums and composition. CountIsMax and NeedPicPrint are false;
//! Init/Calcu/BeginPrint own no additional state. Software32 profile only.
use anyhow::{Result, ensure};
use crysirror_image::{
    native_box_blur::{NativeBoxBlur, NativeBoxBlurMath},
    temporal_pattern::PatternRng,
};

#[derive(Clone, Debug)]
pub(crate) enum BlurDraw {
    Copy {
        source: [i32; 4],
        destination: [i32; 2],
        masked: bool,
    },
    Blur(NativeBoxBlur),
}

pub(crate) fn parameters(values: &[i32]) -> [i32; 16] {
    let mut p = [0i32; 16];
    for i in 0..16 {
        // Omitted para9..15 are copied from an uninitialised C++ stack.
        // Define those unused words as zero; supplied/save words survive.
        p[i] = values.get(i).copied().unwrap_or(match i {
            0 => 20,
            1 | 2 => 8,
            3 => 100,
            4 => p[3],
            _ => 0,
        });
        match i {
            1 | 2 => p[i] = p[i].clamp(1, 256),
            3 | 4 => p[i] = p[i].clamp(0, 100),
            6 => p[i] = p[i].max(0),
            _ => {}
        }
        if i == 2 && p[1] * p[2] > 256 {
            p[2] = 256 / p[1];
        }
    }
    p
}

pub(crate) fn validate(p: &[i32; 16], frames: i32) -> Result<()> {
    ensure!(
        (1..=256).contains(&p[1])
            && (1..=256).contains(&p[2])
            && p[1] * p[2] <= 256
            && (0..=100).contains(&p[3])
            && (0..=100).contains(&p[4])
            && p[6] >= 0,
        "NNN picture blur parameters"
    );
    crate::native_curve::validate(frames, p[6], p[5])
}

pub(crate) fn window(
    p: &[i32; 16],
    count: i32,
    frames: i32,
    random: &mut PatternRng,
) -> Result<[u16; 2]> {
    let percent =
        crate::native_curve::value_with_random(count, frames, p[4], p[3], p[6], p[5], random)?;
    let mut size = [
        p[1].wrapping_mul(percent) / 100,
        p[2].wrapping_mul(percent) / 100,
    ];
    for n in &mut size {
        if p[7] != 0 && *n > 1 && *n & 1 == 0 {
            *n -= 1;
        }
        *n = (*n).clamp(1, 256);
    }
    if size[0] * size[1] > 256 {
        size[1] = 256 / size[0];
    }
    Ok(size.map(|n| n as u16))
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
    has_mask: bool,
    random: &mut PatternRng,
) -> Result<Option<BlurDraw>> {
    let window = window(p, count, frames, random)?;
    if u32::from(window[0]) * u32::from(window[1]) > 1 && !has_mask {
        // Original evaluates the curve before testing the mask. A 1x1
        // window instead calls ordinary Blt(TRUE), which can draw RGB.
        return Ok(None);
    }
    geometry(
        window,
        canvas,
        picture,
        clip,
        point,
        size,
        has_mask,
        NativeBoxBlurMath::PremultipliedMask256,
        p[8] != 0,
    )
}

pub(crate) fn geometry(
    window: [u16; 2],
    canvas: [u32; 2],
    picture: [u32; 2],
    clip: [i32; 4],
    point: [i32; 2],
    size: [i32; 2],
    masked: bool,
    math: NativeBoxBlurMath,
    repeat_bottom_updates: bool,
) -> Result<Option<BlurDraw>> {
    ensure!(
        canvas
            .iter()
            .chain(&picture)
            .all(|n| (1..=16384).contains(n)),
        "NNN picture blur dimensions"
    );
    ensure!(
        point
            .iter()
            .chain(&size)
            .chain(&clip)
            .all(|n| n.unsigned_abs() <= 1_000_000),
        "NNN picture blur coordinates"
    );
    let [left, top, right, bottom] = clip.map(i64::from);
    if left >= right || top >= bottom {
        return Ok(None);
    }
    ensure!(
        left >= 0 && top >= 0 && right <= i64::from(picture[0]) && bottom <= i64::from(picture[1]),
        "NNN picture blur source trim"
    );
    let mut source = [0i64; 2];
    let mut point = point.map(i64::from);
    let mut size = size.map(i64::from);
    for (axis, (start, end)) in [(left, right), (top, bottom)].into_iter().enumerate() {
        if source[axis] < start {
            let delta = start - source[axis];
            source[axis] += delta;
            point[axis] += delta;
            size[axis] -= delta;
        }
        if source[axis] + size[axis] > end {
            size[axis] = end - source[axis];
        }
    }
    let direct = window == [1, 1];
    if !direct {
        for axis in 0..2 {
            let before = i64::from(window[axis] / 2);
            source[axis] -= before;
            point[axis] -= before;
            size[axis] += i64::from(window[axis]);
        }
    }
    for axis in 0..2 {
        if point[axis] < 0 {
            let delta = -point[axis];
            source[axis] += delta;
            point[axis] += delta;
            size[axis] -= delta;
        }
        size[axis] = size[axis].min(i64::from(canvas[axis]) - point[axis]);
    }
    if size.iter().any(|n| *n <= 0) {
        return Ok(None);
    }
    if direct {
        return Ok(Some(BlurDraw::Copy {
            source: [
                source[0] as i32,
                source[1] as i32,
                size[0] as i32,
                size[1] as i32,
            ],
            destination: point.map(|n| n as i32),
            masked,
        }));
    }
    let spec = NativeBoxBlur {
        source: source.map(|n| n as i32),
        destination: [
            point[0] as i32,
            point[1] as i32,
            size[0] as i32,
            size[1] as i32,
        ],
        source_bounds: clip.map(|n| n as u32),
        window,
        math,
        repeat_bottom_updates,
    };
    spec.validate()?;
    Ok(Some(BlurDraw::Blur(spec)))
}

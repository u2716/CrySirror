// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CPicture::StretchBlt1/CStretchBlt::Print native16.16 phase and mode policy.
//! Native rectangle/mask/key decisions only; the shared FixedScanlines worker
//! owns source access and pixel arithmetic, including offscreen sample skips.
use anyhow::{Result, ensure};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StretchMath {
    Copy,
    CoverageSeparate { weight: u32, opaque_copy: bool },
    ConstantSeparate { weights: [u16; 2] },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StretchPlan {
    pub destination: [i32; 4],
    pub source_q16: [i64; 2],
    pub source_step_q16: [i64; 2],
    pub math: StretchMath,
    pub corner_rgb_key: bool,
}
pub(crate) fn native_fixed(value: f64) -> i64 {
    let mut whole = value as i32;
    if value < 0.0 {
        whole = whole.wrapping_sub(1);
    }
    let fraction = ((value - f64::from(whole)) * 65536.0) as i32;
    // Original decrements the integer part for every negative value, including
    // exact integers; a fraction65536 then loses its carry through the mask.
    // Keep that defined sample phase rather than correcting it to floor().
    i64::from(whole.wrapping_shl(16) | (fraction & 65535))
}
pub(crate) fn plan(
    picture: [u32; 2],
    canvas: [u32; 2],
    source: [i32; 4],
    destination: [i32; 4],
    has_mask: bool,
    transparent: bool,
    percent: i32,
) -> Result<Option<StretchPlan>> {
    ensure!(
        picture
            .iter()
            .chain(&canvas)
            .all(|v| (1..=16384).contains(v)),
        "NNN StretchBlt image dimensions"
    );
    let [sx, sy, sw, sh] = source;
    let [dx, dy, dw, dh] = destination;
    if sx < 0 || sy < 0 || sx >= picture[0] as i32 || sy >= picture[1] as i32 || sw == 0 || sh == 0
    {
        return Ok(None);
    }
    for axis in 0..2 {
        let origin = source[axis];
        let extent = source[axis + 2];
        let end = i64::from(origin) + i64::from(extent);
        if extent > 0 && end > i64::from(picture[axis]) || extent < 0 && end < 0 {
            return Ok(None);
        }
    }
    if dw <= 0 || dh <= 0 {
        return Ok(None);
    }
    let left = i64::from(dx).max(0);
    let top = i64::from(dy).max(0);
    let right = (i64::from(dx) + i64::from(dw)).min(i64::from(canvas[0]));
    let bottom = (i64::from(dy) + i64::from(dh)).min(i64::from(canvas[1]));
    if right <= left || bottom <= top {
        return Ok(None);
    }
    let step = [f64::from(sw) / f64::from(dw), f64::from(sh) / f64::from(dh)];
    let skip = [left - i64::from(dx), top - i64::from(dy)];
    let phase = std::array::from_fn(|axis| {
        native_fixed(step[axis] * skip[axis] as f64).wrapping_add(i64::from(source[axis]) * 65536)
    });
    let weight = percent.wrapping_mul(256) / 100;
    let math = if !transparent || !has_mask && percent == 100 {
        StretchMath::Copy
    } else if has_mask {
        StretchMath::CoverageSeparate {
            weight: if percent == 100 { 256 } else { weight as u32 },
            opaque_copy: percent == 100,
        }
    } else {
        let a = weight as u16;
        StretchMath::ConstantSeparate {
            weights: [a, a ^ 255],
        }
    };
    Ok(Some(StretchPlan {
        destination: [
            left as i32,
            top as i32,
            (right - left) as i32,
            (bottom - top) as i32,
        ],
        source_q16: phase,
        source_step_q16: step.map(native_fixed),
        math,
        corner_rgb_key: transparent && !has_mask,
    }))
}

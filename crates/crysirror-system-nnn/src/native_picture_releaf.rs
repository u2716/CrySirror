// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! RELEAF33, effectReleaf.cpp: software32 MMX prefix and native curve policy.
//! CountIsMax returns true; BeginPrint/Init/Calcu/End own no extra state.
//! Win64 omits the software body and D3D uses a different shader profile.
use anyhow::{Result, ensure};
use crysirror_image::{neighbor_relief::NeighborRelief, temporal_pattern::PatternRng};

pub(crate) fn parameters(values: &[i32]) -> [i32; 16] {
    // SetParam writes only five words. Remaining native EFFECT words are not
    // parameters for this command; preserve supplied/save words as other
    // picture-effect policies do and define omitted unused storage as zero.
    std::array::from_fn(|i| values.get(i).copied().unwrap_or(match i {
        0 => 20,
        2 => 100,
        _ => 0,
    }))
}

pub(crate) fn validate(parameters: &[i32; 16], frames: i32) -> Result<()> {
    crate::native_curve::validate(frames, parameters[4], parameters[1])
}

pub(crate) fn spec(
    parameters: &[i32; 16],
    count: i32,
    frames: i32,
    canvas: [u32; 2],
    random: &mut PatternRng,
) -> Result<NeighborRelief> {
    ensure!(canvas.iter().all(|n| (1..=16384).contains(n)), "NNN RELEAF canvas dimensions");
    let weight = (crate::native_curve::value_with_random(
        count,
        frames,
        parameters[3].wrapping_mul(256),
        parameters[2].wrapping_mul(256),
        parameters[4],
        parameters[1],
        random,
    )? / 100).clamp(0, 256) as u16;
    let length = u64::from(canvas[0]) * u64::from(canvas[1]);
    // LOOP1 transforms two pixels per iteration, stopping two pixels short
    // for even lengths and three short for odd lengths. Then LOOP2 repairs
    // each row's final pixel from its already transformed predecessor.
    // Original length<4 runs a zero/negative assembly counter out of bounds;
    // define that prefix as empty. Width1's preceding-row read is similarly
    // replaced by the shared kernel's bounded row-repair no-op.
    let pixels = (length / 2).saturating_sub(1) * 2;
    let spec = NeighborRelief { weight, pixels: pixels as u32, copy_row_last: true };
    spec.validate()?;
    Ok(spec)
}

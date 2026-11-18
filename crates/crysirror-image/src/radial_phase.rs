// SPDX-License-Identifier: MPL-2.0
//! Quantized polar wave sampling, reusable by destructive bitmap operations.
//! Distances, byte-angle quantization, binary32 trigonometric products and
//! truncating nearest coordinates are explicit, independently of rendering.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use crysirror_core::{image_processing::RadialPhaseField, state::ClientId};
use std::sync::{Arc, LazyLock};

static TRIG: LazyLock<[[f32; 2]; 256]> = LazyLock::new(|| {
    std::array::from_fn(|i| {
        let angle = i as f64 * std::f64::consts::PI / 128.;
        [angle.cos() as f32, angle.sin() as f32]
    })
});

/// Inverse polar rotation with linearly tapered strength. This is explicit
/// compatibility geometry, not an emulation of an invalid native callback.
/// Zero strength and pixels outside the disc retain their exact stored bytes.
pub fn twist(
    input: &ImageSurface,
    field: &RadialPhaseField,
    center: [i32; 2],
    radius: u16,
    strength: u16,
    clear: [u8; 4],
    check: &dyn Fn() -> Result<()>,
) -> Result<ImageSurface> {
    input.validate()?;
    field.validate(field.client, [input.width, input.height])?;
    ensure!(
        radius > 0
            && radius <= 16384
            && strength <= 1000
            && center.iter().all(|v| v.unsigned_abs() <= 16384),
        "polar twist bounds"
    );
    check()?;
    let mut output = input.clone();
    if strength == 0 {
        return Ok(output);
    }
    let radius = f64::from(radius);
    for y in 0..input.height {
        check()?;
        for x in 0..input.width {
            let at = (y * input.width + x) as usize;
            let distance = f64::from(field.distances[at]);
            ensure!(
                distance.is_finite() && distance >= 0.,
                "polar twist distance"
            );
            if distance >= radius {
                continue;
            }
            let phase = (f64::from(strength) * 256. / 1000. * (1. - distance / radius)).trunc()
                as u32 as u8;
            if phase == 0 {
                continue;
            }
            let [c, s] = TRIG[phase as usize];
            let dx = f64::from(x) - f64::from(center[0]);
            let dy = f64::from(y) - f64::from(center[1]);
            let sx = (f64::from(center[0]) + dx * f64::from(c) + dy * f64::from(s)).floor() as i64;
            let sy = (f64::from(center[1]) - dx * f64::from(s) + dy * f64::from(c)).floor() as i64;
            let dest = &mut output.rgba[at * 4..at * 4 + 4];
            if sx >= 0 && sy >= 0 && sx < i64::from(input.width) && sy < i64::from(input.height) {
                let from = (sy as usize * input.width as usize + sx as usize) * 4;
                dest.copy_from_slice(&input.rgba[from..from + 4]);
            } else {
                dest.copy_from_slice(&clear);
            }
        }
    }
    check()?;
    Ok(output)
}

pub fn prepare(
    client: ClientId,
    size: [u32; 2],
    center: [i32; 2],
    check: &dyn Fn() -> Result<()>,
) -> Result<Arc<RadialPhaseField>> {
    crate::rgba_len(size[0], size[1])?;
    ensure!(
        u64::from(size[0]) * u64::from(size[1]) <= MAX_EFFECT_PIXELS
            && center.iter().all(|n| n.unsigned_abs() <= 16384),
        "radial phase geometry budget"
    );
    let count = size[0] as usize * size[1] as usize;
    let mut distances = Vec::with_capacity(count);
    let mut angles = Vec::with_capacity(count);
    for y in 0..size[1] {
        check()?;
        let dy = center[1] - y as i32;
        let yy = (dy * dy) as f32;
        for x in 0..size[0] {
            let dx = center[0] - x as i32;
            let squared = (f64::from(dx) * f64::from(dx) + f64::from(yy)) as f32;
            distances.push(f64::from(squared).sqrt() as f32);
            let angle = f64::from(-dy).atan2(f64::from(dx)) / std::f64::consts::PI * 128. + 256.;
            angles.push((angle.trunc() as i32) as u8);
        }
    }
    check()?;
    Ok(Arc::new(RadialPhaseField {
        client,
        size,
        distances: distances.into(),
        angles: angles.into(),
    }))
}
pub fn warp(
    input: &ImageSurface,
    field: &RadialPhaseField,
    phase: u32,
    frequency: u16,
    amplitude: u16,
    clear: [u8; 4],
    check: &dyn Fn() -> Result<()>,
) -> Result<ImageSurface> {
    input.validate()?;
    field.validate(field.client, [input.width, input.height])?;
    ensure!(
        amplitude <= 1000 && frequency <= 6553,
        "radial phase parameters"
    );
    let mut output = ImageSurface {
        width: input.width,
        height: input.height,
        rgba: vec![0; input.rgba.len()],
    };
    for pixel in output.rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&clear);
    }
    for y in 0..input.height {
        check()?;
        for x in 0..input.width {
            let at = (y * input.width + x) as usize;
            let distance = field.distances[at];
            ensure!(
                distance.is_finite() && (0. ..=100000.).contains(&distance),
                "radial phase distance"
            );
            let index = ((f64::from(distance) - f64::from(phase as i32)) * f64::from(frequency))
                .trunc() as i64 as u8;
            let offset = (f64::from(TRIG[index as usize][1]) * f64::from(amplitude)) as f32;
            let [c, s] = TRIG[field.angles[at] as usize];
            let sx = (f64::from(c) * f64::from(offset) + f64::from(x)).trunc() as i64;
            let sy = (f64::from(y) - f64::from(s) * f64::from(offset)).trunc() as i64;
            if sx >= 0 && sx < i64::from(input.width) && sy >= 0 && sy < i64::from(input.height) {
                let from = ((sy as usize) * input.width as usize + sx as usize) * 4;
                output.rgba[at * 4..at * 4 + 4].copy_from_slice(&input.rgba[from..from + 4]);
            }
        }
    }
    check()?;
    Ok(output)
}

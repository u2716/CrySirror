// SPDX-License-Identifier: MPL-2.0
//! Spherical lens displacement: table preparation and 2x2 weighted sampling.
//! Tables are derived from a disc of radius `n/2`; sampling reproduces 16-bit
//! lane arithmetic (`pmullw` keeps the low word, `psrlw`, saturating adds).
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use crysirror_core::{image_processing::LensTable, state::ClientId};
use std::sync::Arc;

/// `curvature` scales the sphere radius; `distance` is the viewing distance in
/// disc radii. Extents must be square and at most `MAX_EFFECT_PIXELS`.
pub fn prepare(
    client: ClientId,
    n: u32,
    curvature: f64,
    distance: f64,
    check: &dyn Fn() -> Result<()>,
) -> Result<Arc<LensTable>> {
    ensure!(
        n > 0 && u64::from(n) * u64::from(n) <= MAX_EFFECT_PIXELS && n <= 16_384,
        "lens extent budget"
    );
    ensure!(
        curvature.is_finite() && distance.is_finite() && curvature > 0.,
        "lens parameters"
    );
    let size = n as usize;
    let radius = f64::from(n >> 1);
    let disc = radius * radius;
    let view = distance * radius;
    let sphere = (radius * curvature) * (radius * curvature);
    // The sphere must contain the disc, or the square roots below are undefined.
    ensure!(sphere >= disc, "lens curvature below disc radius");
    let rim = (sphere - disc).sqrt();
    let displacement = |d2: f64| {
        let g = (sphere - d2).sqrt() - rim;
        g / (view + g)
    };
    let (mut max_x, mut max_y) = (0f64, 0f64);
    for j in 0..size {
        check()?;
        let dy = j as f64 - radius;
        for i in 0..size {
            let dx = i as f64 - radius;
            let d2 = dx * dx + dy * dy;
            if disc > d2 {
                let t = displacement(d2);
                max_y = max_y.max((dy * t).abs());
                max_x = max_x.max((t * dx).abs());
            }
        }
    }
    let scale_x = 128. / (max_x + 1.);
    let scale_y = 128. / (max_y + 1.);
    let mut offsets = vec![[0i32; 2]; size * size];
    let mut weights = vec![[255u8, 255, 0, 255]; size * size];
    let mut outside = vec![false; size * size];
    for j in 0..size {
        check()?;
        let dy = j as f64 - radius;
        for i in 0..size {
            let dx = i as f64 - radius;
            let at = j * size + i;
            let d2 = dx * dx + dy * dy;
            if disc <= d2 {
                outside[at] = true;
                continue;
            }
            let t = displacement(d2);
            let axis = |delta: f64, scale: f64, at: usize| {
                let x = t * delta * scale;
                let mut offset = (-x.floor()) as i32;
                if at as i64 + i64::from(offset) >= size as i64 || at as i64 + i64::from(offset) < 0 {
                    offset = 0;
                }
                let a = x.abs();
                let b = f64::from(offset).abs();
                let frac = if b > a { b - a } else { a - b };
                (offset, ((frac * 255.) as i32) as u8)
            };
            let (ox, wx) = axis(dx, scale_x, i);
            let (oy, wy) = axis(dy, scale_y, j);
            offsets[at] = [ox, oy];
            weights[at] = [wx, 255 - wx, wy, 255 - wy];
        }
    }
    check()?;
    Ok(Arc::new(LensTable {
        client,
        size: n,
        offsets: offsets.into(),
        weights: weights.into(),
        outside: outside.into(),
    }))
}

pub fn warp(
    source: &ImageSurface,
    size: [u32; 2],
    table: &LensTable,
    origin: [i32; 2],
    full_cover: bool,
    check: &dyn Fn() -> Result<()>,
) -> Result<ImageSurface> {
    source.validate()?;
    table.validate(table.client, size)?;
    let n = table.size as i64;
    let (w, h) = (i64::from(source.width), i64::from(source.height));
    let pixel = |x: i64, y: i64| -> [u32; 4] {
        let at = ((y * w + x) * 4) as usize;
        std::array::from_fn(|c| u32::from(source.rgba[at + c]))
    };
    let mut out = ImageSurface {
        width: size[0],
        height: size[1],
        rgba: vec![0; crate::rgba_len(size[0], size[1])?],
    };
    let x_start = i64::from(origin[0]).max(0);
    let x_end = (i64::from(origin[0]) + n).min(w);
    // Native loops never write the first screen row.
    let y_start = i64::from(origin[1]).max(1);
    let y_end = (i64::from(origin[1]) + n).min(h);
    for y in y_start..y_end {
        check()?;
        let ly = y - i64::from(origin[1]);
        for x in x_start..x_end {
            let lx = x - i64::from(origin[0]);
            let index = (ly * n + lx) as usize;
            let dest = index * 4;
            if !full_cover && table.outside[index] {
                out.rgba[dest..dest + 4].copy_from_slice(&source.rgba[((y * w + x) * 4) as usize..][..4]);
                continue;
            }
            let [dx, dy] = table.offsets[index];
            let mut sy = y + i64::from(dy);
            sy = if sy > 0 { sy.min(h - 1) } else { 1 };
            let sx = (x + i64::from(dx)).clamp(1, w - 1);
            let [a0, a1, a2, a3] = table.weights[index].map(u32::from);
            let (p00, p10) = (pixel(sx, sy), pixel(sx - 1, sy));
            let (p01, p11) = (pixel(sx, sy - 1), pixel(sx - 1, sy - 1));
            for c in 0..4 {
                let row = |near: u32, far: u32, weight: u32| {
                    let sum = ((near * a1) >> 8) + ((far * a0) >> 8);
                    ((sum * weight) & 0xffff) >> 8
                };
                let value = row(p00[c], p10[c], a3) + row(p01[c], p11[c], a2);
                out.rgba[dest + c] = value.min(255) as u8;
            }
        }
    }
    check()?;
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn table_is_symmetric_and_bounded() {
        let client = ClientId(1);
        let t = prepare(client, 64, 8., 10., &|| Ok(())).unwrap();
        assert!(t.outside[0] && !t.outside[32 * 64 + 32]);
        assert!(t.offsets.iter().all(|o| o.iter().all(|v| v.abs() < 64)));
        // The centre pixel samples itself.
        assert_eq!(t.offsets[32 * 64 + 32], [0, 0]);
    }
    #[test]
    fn identity_weights_copy_source() {
        let client = ClientId(1);
        let t = prepare(client, 8, 8., 10., &|| Ok(())).unwrap();
        let src = ImageSurface { width: 16, height: 16, rgba: vec![200; 16 * 16 * 4] };
        let out = warp(&src, [8, 8], &t, [2, 2], false, &|| Ok(())).unwrap();
        // Outside-disc pixels are copied; a flat source stays flat inside too.
        let inside = out.rgba[(3 * 8 + 3) * 4];
        // Three independent /256 truncations lose a few levels, never gain.
        assert!((190..=200).contains(&inside), "{inside}");
        assert_eq!(&out.rgba[..4], &[200, 200, 200, 200]);
    }
}

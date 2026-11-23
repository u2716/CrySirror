// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Exact-overlap area resampling and stepped nearest stretching of stored
//! RGBA, with integer arithmetic only.
//!
//! `overlap_resample`: a destination pixel `k` on an axis of source length
//! `S` and destination length `D` covers `[k*S, (k+1)*S)` while source pixel
//! `i` covers `[i*D, (i+1)*D)`; each source pixel contributes its overlap
//! length. Row weights above 4096 are divided by 16 (column weights are not);
//! the weighted channel sums are divided with truncation. With a colour key,
//! keyed texels are excluded and an all-key window yields the key colour.
//! Behaviour reference: D.C.II P.C. 4045B0 (statically read as data).
//!
//! `stepped_nearest`: column/row `i` samples `0` for `i == 0`, otherwise
//! `floor((S/2 + i*S) / D)` (error accumulator starting at `S/2`, advanced
//! after each sample). Reference: D.C.II P.C. 40D7C0 without clipping.
use crate::ImageSurface;
use anyhow::{ensure, Result};

fn spans(source: i64, destination: i64) -> Vec<Vec<(usize, i64)>> {
    (0..destination)
        .map(|k| {
            let start = k * source;
            let end = start + source;
            let (first, last) = (start / destination, end / destination);
            if first == last {
                return vec![(first as usize, source)];
            }
            let mut out = vec![(first as usize, destination * (first + 1) - start)];
            out.extend((first + 1..last).map(|i| (i as usize, destination)));
            if end % destination != 0 {
                out.push((last as usize, end % destination));
            }
            out
        })
        .collect()
}

fn admit(source: &ImageSurface, destination: [u32; 2]) -> Result<()> {
    source.validate()?;
    crate::rgba_len(destination[0], destination[1])?;
    Ok(())
}

/// Area-weighted resample of `source` to `destination`. With `key`, texels
/// whose stored RGB equals it are excluded and their alpha is left at 255.
pub fn overlap_resample(
    source: &ImageSurface,
    destination: [u32; 2],
    key: Option<[u8; 3]>,
) -> Result<ImageSurface> {
    admit(source, destination)?;
    let columns = spans(source.width.into(), destination[0].into());
    let rows = spans(source.height.into(), destination[1].into());
    let width = source.width as usize;
    let mut rgba = vec![0u8; crate::rgba_len(destination[0], destination[1])?];
    for (y, row) in rows.iter().enumerate() {
        for (x, column) in columns.iter().enumerate() {
            let mut sum = [0i64; 4];
            let mut total = 0i64;
            for &(sy, wy) in row {
                let wy = if wy > 4096 { wy / 16 } else { wy };
                for &(sx, wx) in column {
                    let at = (sy * width + sx) * 4;
                    let texel = &source.rgba[at..at + 4];
                    if key.is_some_and(|key| texel[..3] == key) {
                        continue;
                    }
                    let weight = wy * wx;
                    total += weight;
                    for c in 0..4 {
                        sum[c] += weight * i64::from(texel[c]);
                    }
                }
            }
            ensure!(
                sum.iter().chain([&total]).all(|v| *v <= i64::from(i32::MAX)),
                "overlap resample accumulator exceeds the reviewed 32-bit range"
            );
            let at = (y * destination[0] as usize + x) * 4;
            let out = &mut rgba[at..at + 4];
            match (total, key) {
                (0, Some(key)) => out.copy_from_slice(&[key[0], key[1], key[2], 255]),
                (0, None) => {}
                _ => {
                    for c in 0..4 {
                        out[c] = (sum[c] / total) as u8;
                    }
                    if key.is_some() {
                        out[3] = 255;
                    }
                }
            }
        }
    }
    Ok(ImageSurface { width: destination[0], height: destination[1], rgba })
}

/// Stepped nearest-neighbour stretch of `source` to `destination`.
pub fn stepped_nearest(source: &ImageSurface, destination: [u32; 2]) -> Result<ImageSurface> {
    admit(source, destination)?;
    let index = |i: u32, s: u32, d: u32| -> usize {
        if i == 0 {
            0
        } else {
            ((u64::from(s / 2) + u64::from(i) * u64::from(s)) / u64::from(d)) as usize
        }
    };
    let mut rgba = Vec::with_capacity(crate::rgba_len(destination[0], destination[1])?);
    for y in 0..destination[1] {
        let row = index(y, source.height, destination[1]) * source.width as usize;
        for x in 0..destination[0] {
            let at = (row + index(x, source.width, destination[0])) * 4;
            rgba.extend_from_slice(&source.rgba[at..at + 4]);
        }
    }
    Ok(ImageSurface { width: destination[0], height: destination[1], rgba })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn surface(width: u32, height: u32, rgba: Vec<u8>) -> ImageSurface {
        ImageSurface { width, height, rgba }
    }
    #[test]
    fn overlap_weights_partial_texels() {
        // 4 -> 3: destination 0 takes texel0 (3) + texel1 (1).
        let source = surface(4, 1, [0u8, 40, 80, 120].iter().flat_map(|v| [*v, 0, 0, 255]).collect());
        let out = overlap_resample(&source, [3, 1], None).unwrap();
        assert_eq!([out.rgba[0], out.rgba[4], out.rgba[8]], [10, 60, 110]);
    }
    #[test]
    fn keyed_texels_are_excluded() {
        let source = surface(2, 1, vec![255, 0, 255, 255, 30, 60, 90, 255]);
        let out = overlap_resample(&source, [1, 1], Some([255, 0, 255])).unwrap();
        assert_eq!(out.rgba, [30, 60, 90, 255]);
        let keyed = surface(2, 1, vec![255, 0, 255, 255, 255, 0, 255, 255]);
        let out = overlap_resample(&keyed, [1, 1], Some([255, 0, 255])).unwrap();
        assert_eq!(out.rgba, [255, 0, 255, 255]);
    }
    #[test]
    fn nearest_steps_from_half_source() {
        let source = surface(4, 1, (0..4u8).flat_map(|v| [v, 0, 0, 255]).collect());
        let out = stepped_nearest(&source, [3, 1]).unwrap();
        // i=0 -> 0; i=1 -> (2+4)/3=2; i=2 -> (2+8)/3=3.
        assert_eq!([out.rgba[0], out.rgba[4], out.rgba[8]], [0, 2, 3]);
    }
}

/// Stretch a centered crop with endpoint-aligned nearest sampling, then average
/// its stored bytes with the retained destination, rounding each channel down.
/// A zero-sized crop on an axis samples its center coordinate throughout.
pub fn blend_centered_crop(history: &mut ImageSurface, source: &ImageSurface, crop: [u32; 2]) -> Result<()> {
    let [w,h] = [source.width,source.height];
    ensure!(w > 0 && h > 0 && [history.width,history.height] == [w,h]
        && crop[0] <= w && crop[1] <= h, "centered crop dimensions");
    let len = crate::rgba_len(w,h)?;
    ensure!(history.rgba.len()==len && source.rgba.len()==len, "centered crop buffer");
    let index = |i:u32, dimension:u32, extent:u32| -> usize {
        let span = u64::from(dimension.saturating_sub(1));
        let offset = (dimension - extent) / 2;
        let step = if span == 0 {0} else {
            (2*u64::from(i)*u64::from(extent.saturating_sub(1))+span)/(2*span)
        };
        offset as usize + step as usize
    };
    let columns: Vec<_> = (0..w).map(|x| 4*index(x,w,crop[0])).collect();
    for (y,row) in history.rgba.chunks_exact_mut(4*w as usize).enumerate() {
        let origin = 4*w as usize*index(y as u32,h,crop[1]);
        for (dst,&x) in row.chunks_exact_mut(4).zip(&columns) {
            for (d,&s) in dst.iter_mut().zip(&source.rgba[origin+x..origin+x+4]) {
                *d = (*d & s) + ((*d ^ s) >> 1);
            }
        }
    }
    Ok(())
}

// SPDX-License-Identifier: MPL-2.0
//! Position straight RGBA sprites on a transparent plane. The unsigned ARGB
//! ordering resolves overlaps by coverage, then by red, green and blue.
use crate::{ImageSurface, rgba_len};
use anyhow::{Result, ensure};

pub fn maximum_argb(
    size: [u32; 2],
    images: &[ImageSurface],
    stamps: &[(usize, [i32; 2])],
    opacity: u16,
) -> Result<ImageSurface> {
    ensure!(
        opacity <= 256 && stamps.len() <= 4096,
        "packed stamp limits"
    );
    compose(
        size,
        images,
        stamps.iter().map(|&(i, p)| (i, p, opacity)),
        256,
    )
}

/// Compose independently weighted stamps using the full 0..255 alpha range.
pub fn maximum_argb_weighted(
    size: [u32; 2],
    images: &[ImageSurface],
    stamps: &[(usize, [i32; 2], u8)],
) -> Result<ImageSurface> {
    ensure!(stamps.len() <= 4096, "packed stamp limits");
    compose(
        size,
        images,
        stamps.iter().map(|&(i, p, a)| (i, p, u16::from(a))),
        255,
    )
}

fn compose(
    size: [u32; 2],
    images: &[ImageSurface],
    stamps: impl Iterator<Item = (usize, [i32; 2], u16)>,
    divisor: u16,
) -> Result<ImageSurface> {
    for image in images {
        image.validate()?;
    }
    let [width, height] = size;
    let mut plane = ImageSurface {
        width,
        height,
        rgba: vec![0; rgba_len(width, height)?],
    };
    let mut work = 0u64;
    for (index, [x, y], opacity) in stamps {
        let image = images
            .get(index)
            .ok_or_else(|| anyhow::anyhow!("packed stamp source index"))?;
        let x = i64::from(x);
        let y = i64::from(y);
        let left = x.max(0);
        let right = (x + i64::from(image.width)).min(i64::from(width));
        let top = y.max(0);
        let bottom = (y + i64::from(image.height)).min(i64::from(height));
        if right <= left || bottom <= top {
            continue;
        }
        work += ((right - left) * (bottom - top)) as u64;
        ensure!(work <= 64 * 1024 * 1024, "packed stamp pixel budget");
        for row in top..bottom {
            let dest = (row * i64::from(width) + left) as usize * 4;
            let src = ((row - y) * i64::from(image.width) + left - x) as usize * 4;
            let len = (right - left) as usize * 4;
            for (dst, p) in plane.rgba[dest..dest + len]
                .chunks_exact_mut(4)
                .zip(image.rgba[src..src + len].chunks_exact(4))
            {
                let a = if opacity >= 255 {
                    p[3]
                } else {
                    (u16::from(p[3]) * opacity / divisor) as u8
                };
                if a > 0 && [a, p[0], p[1], p[2]] > [dst[3], dst[0], dst[1], dst[2]] {
                    dst.copy_from_slice(&[p[0], p[1], p[2], a]);
                }
            }
        }
    }
    Ok(plane)
}

/// Colour each stamp using the corresponding pixel of a stationary map. Each
/// nontransparent stamp replaces the destination. The bright branch uses
/// successive integer divisions and clamps the result to the byte range.
pub fn mapped_overlay(
    size: [u32; 2],
    map: &ImageSurface,
    images: &[ImageSurface],
    stamps: &[(usize, [i32; 2])],
) -> Result<ImageSurface> {
    map.validate()?;
    for image in images {
        image.validate()?;
    }
    ensure!(stamps.len() <= 4096, "mapped stamp count");
    let mut output = ImageSurface::filled(size[0], size[1], [0; 4])?;
    let mut work = 0u64;
    for &(index, [x, y]) in stamps {
        let image = images
            .get(index)
            .ok_or_else(|| anyhow::anyhow!("mapped stamp image index"))?;
        let x = i64::from(x);
        let y = i64::from(y);
        let right = (x + i64::from(image.width)).min(i64::from(size[0].min(map.width)));
        let bottom = (y + i64::from(image.height)).min(i64::from(size[1].min(map.height)));
        if right <= x.max(0) || bottom <= y.max(0) {
            continue;
        }
        work += ((right - x.max(0)) * (bottom - y.max(0))) as u64;
        ensure!(work <= 64 * 1024 * 1024, "mapped stamp pixel budget");
        for row in y.max(0)..bottom {
            for col in x.max(0)..right {
                let src = 4 * ((row - y) * i64::from(image.width) + col - x) as usize;
                if image.rgba[src + 3] == 0 {
                    continue;
                }
                let dst = 4 * (row * i64::from(size[0]) + col) as usize;
                let mask = 4 * (row * i64::from(map.width) + col) as usize;
                for i in 0..3 {
                    let a = i32::from(image.rgba[src + i]);
                    let b = i32::from(map.rgba[mask + i]);
                    let value = if b < 128 {
                        2 * a * b / 255
                    } else {
                        ensure!(a != 0, "zero channel in bright mapped stamp");
                        2 * (a + b - b / a / 255) - 255
                    };
                    output.rgba[dst + i] = value.clamp(0, 255) as u8;
                }
                output.rgba[dst + 3] = image.rgba[src + 3];
            }
        }
    }
    Ok(output)
}

/// Retain the greater packed ARGB pixel after subtracting coverage from the previous plane.
pub fn retain_maximum(frame: &mut ImageSurface, previous: &ImageSurface, decay: u8) -> Result<()> {
    frame.validate()?;
    previous.validate()?;
    ensure!(
        (frame.width, frame.height) == (previous.width, previous.height),
        "stamp history dimensions differ"
    );
    for (new, old) in frame
        .rgba
        .chunks_exact_mut(4)
        .zip(previous.rgba.chunks_exact(4))
    {
        let a = old[3].saturating_sub(decay);
        if [a, old[0], old[1], old[2]] > [new[3], new[0], new[1], new[2]] {
            new.copy_from_slice(&[old[0], old[1], old[2], a]);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clips_and_compares_whole_packed_pixel() {
        let images = [
            ImageSurface {
                width: 2,
                height: 1,
                rgba: vec![0, 0, 0, 0, 20, 4, 9, 200],
            },
            ImageSurface {
                width: 1,
                height: 1,
                rgba: vec![19, 255, 255, 200],
            },
        ];
        let stamps = [(0, [-1, 0]), (1, [0, 0])];
        assert_eq!(
            maximum_argb([1, 1], &images, &stamps, 128).unwrap().rgba,
            [20, 4, 9, 100]
        );
        assert_eq!(
            maximum_argb([1, 1], &images, &stamps, 255).unwrap().rgba,
            [20, 4, 9, 200]
        );
    }
}

/// Scale an alpha mask using a floor-mapped source origin. Add weighted coverage
/// to the destination's alpha, saturating at the supplied ceiling; touched RGB
/// is solid white. The source and destination use straight RGBA bytes.
pub fn add_white_mask(
    frame: &mut ImageSurface,
    mask: &ImageSurface,
    origin: [i32; 2],
    size: [u32; 2],
    weight: u32,
    divisor: u32,
    ceiling: u8,
) -> Result<()> {
    frame.validate()?;
    mask.validate()?;
    anyhow::ensure!(divisor != 0, "zero mask coverage divisor");
    if size.contains(&0) {
        return Ok(());
    }
    let [ox, oy] = origin.map(i64::from);
    for y in oy.max(0)..(oy + i64::from(size[1])).min(i64::from(frame.height)) {
        let sy = (y - oy) as u64 * u64::from(mask.height) / u64::from(size[1]);
        for x in ox.max(0)..(ox + i64::from(size[0])).min(i64::from(frame.width)) {
            let sx = (x - ox) as u64 * u64::from(mask.width) / u64::from(size[0]);
            let alpha = mask.rgba[((sy * u64::from(mask.width) + sx) * 4 + 3) as usize];
            if alpha == 0 {
                continue;
            }
            let off = ((y as usize) * frame.width as usize + x as usize) * 4;
            let add = u64::from(alpha) * u64::from(weight) / u64::from(divisor);
            let a = (u64::from(frame.rgba[off + 3]) + add).min(u64::from(ceiling)) as u8;
            frame.rgba[off..off + 4].copy_from_slice(&[255, 255, 255, a]);
        }
    }
    Ok(())
}

// Portions adapted from Kirikiri Z (krkrz/krkrz fd5c4baa6a2ef5978db1bd043634351f48667daf:
// visual/CharacterData.cpp tTVPCharacterData::Blur, visual/glgen/gengl.pl
// TVPChBlurCopy_c, TVPChBlurAddMulCopy_c, TVPChBlurMulCopy_c, fast_int_hypot).
// Copyright (c) W.Dee and contributors; Kirikiri Z Project Contributors.
// Kirikiri licence (docs/third-party/krkrz.txt). Modifications CrySirror
// contributors (MPL-2.0).
//! Blurred glyph coverage for text shadows/edges: a circular kernel weighted
//! `width - distance + 1`, normalised to 2^18 and scaled by `level/256`, with
//! each accumulation clamped to 255. Operates on 8-bit coverage, as the
//! native 256-level path does.

/// Approximate hypotenuse used by the native kernel (integer, not Euclidean).
pub fn fast_int_hypot(lx: i32, ly: i32) -> i32 {
    let (lx, ly) = (lx.abs(), ly.abs());
    let (len1, len2) = if lx >= ly { (lx, ly) } else { (ly, lx) };
    let t = len2 + (len2 >> 1);
    len1 - (len1 >> 5) - (len1 >> 7) + (t >> 2) + (t >> 6)
}

/// Blur `src` (`width`×`height` 8-bit coverage). With `radius == 0` the
/// coverage is only scaled by `level`. Returns the new cell size; the cell
/// grows by `radius` on every side (its origin moves by `-radius`).
pub fn blur_coverage(
    src: &[u8],
    width: usize,
    height: usize,
    radius: u32,
    level: i32,
) -> (Vec<u8>, usize, usize) {
    debug_assert_eq!(src.len(), width * height);
    if radius == 0 {
        if level == 255 {
            return (src.to_vec(), width, height);
        }
        // TVPChBlurMulCopy: level<<10, then >>18 (i.e. *level/256).
        let l = i64::from(level) << 10;
        let out = src
            .iter()
            .map(|&v| ((i64::from(v) * l) >> 18).min(255) as u8)
            .collect();
        return (out, width, height);
    }
    let bw = radius as i32;
    let (nw, nh) = (width + 2 * radius as usize, height + 2 * radius as usize);
    let mut dest = vec![0u8; nw * nh];
    let mut lvsum = 0i32;
    for y in -bw..=bw {
        for x in -bw..=bw {
            let len = fast_int_hypot(x, y);
            if len <= bw {
                lvsum += bw - len + 1;
            }
        }
    }
    let lvsum = if lvsum != 0 {
        (1 << 18) / lvsum
    } else {
        1 << 18
    };
    for y in -bw..=bw {
        for x in -bw..=bw {
            let len = fast_int_hypot(x, y);
            if len > bw {
                continue;
            }
            let weight = (i64::from(bw - len + 1) * i64::from(lvsum) * i64::from(level)) >> 8;
            for sy in 0..height {
                let row = (y + bw) as usize + sy;
                let base = row * nw + (x + bw) as usize;
                for (i, &s) in src[sy * width..(sy + 1) * width].iter().enumerate() {
                    let d = &mut dest[base + i];
                    let a = i64::from(*d) + ((i64::from(s) * weight) >> 18);
                    *d = a.min(255) as u8;
                }
            }
        }
    }
    (dest, nw, nh)
}

pub fn blur_image(
    source: &crate::ImageSurface,
    radius: u32,
    level: i32,
    check: &dyn Fn() -> anyhow::Result<()>,
) -> anyhow::Result<crate::ImageSurface> {
    source.validate()?;
    check()?;
    anyhow::ensure!(
        radius <= 64 && (0..=65535).contains(&level),
        "glyph blur parameters"
    );
    let work =
        u64::from(source.width) * u64::from(source.height) * u64::from(radius * 2 + 1).pow(2);
    anyhow::ensure!(work <= 64 * 1024 * 1024, "glyph blur work budget");
    let alpha: Vec<u8> = source.rgba.chunks_exact(4).map(|p| p[3]).collect();
    let (mask, w, h) = blur_coverage(
        &alpha,
        source.width as usize,
        source.height as usize,
        radius,
        level,
    );
    check()?;
    let mut rgba = vec![0; crate::rgba_len(w as u32, h as u32)?];
    for y in 0..h {
        check()?;
        for x in 0..w {
            let sx = x
                .saturating_sub(radius as usize)
                .min(source.width as usize - 1);
            let sy = y
                .saturating_sub(radius as usize)
                .min(source.height as usize - 1);
            let from = (sy * source.width as usize + sx) * 4;
            let at = (y * w + x) * 4;
            rgba[at..at + 3].copy_from_slice(&source.rgba[from..from + 3]);
            rgba[at + 3] = mask[y * w + x];
        }
    }
    Ok(crate::ImageSurface {
        width: w as u32,
        height: h as u32,
        rgba,
    })
}

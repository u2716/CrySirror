// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! RAIN2 56: per-slot centipixel motion and original ordered random draws.
//! Shared ByteRaster::Tile performs natural-size repetition in O(canvas),
//! without building one graph node per tiny source tile.
use crate::native_picture_effect::PictureEffectFrame;
use anyhow::{Result, ensure};
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageInput};
use crysirror_image::{
    byte_raster::{ByteRaster, ByteRasterMath},
    effects::ImageEffect,
    temporal_pattern::PatternRng,
};
pub(crate) fn parameters(values: &[i32]) -> [i32; 8] {
    let defaults = [20, 5, 30, 2, 3, 0, 50, 100];
    let mut p = std::array::from_fn(|i| values.get(i).copied().unwrap_or(defaults[i]));
    p[3] = p[3].clamp(1, 4);
    p[7] = p[7].max(1);
    p
}
// Six arrays of four words followed by initRnd. None belongs to raw EFFECT;
// InitParameter reruns SetParam, resetting these words and EFFECT count/max.
pub(crate) fn work(p: &[i32; 8]) -> [i32; 25] {
    let mut w = [0; 25];
    w[24] = 1;
    for (start, param) in [(8, 1), (12, 2), (16, 4), (20, 5)] {
        w[start] = p[param].wrapping_mul(100);
        for n in 1..p[3] as usize {
            w[start + n] = w[start + n - 1].wrapping_mul(p[6]) / 100;
        }
    }
    w
}
pub(crate) fn validate(p: &[i32; 8]) -> Result<()> {
    ensure!(
        (1..=4).contains(&p[3]) && p[7] > 0,
        "NNN RAIN2 plane/divisor words"
    );
    Ok(())
}
pub(crate) fn calcu(
    p: &[i32; 8],
    w: &mut [i32; 25],
    size: [u32; 2],
    canvas: [u32; 2],
    rng: &mut PatternRng,
) -> Result<()> {
    validate(p)?;
    ensure!(
        size.iter()
            .chain(canvas.iter())
            .all(|n| (1..=16384).contains(n)),
        "NNN RAIN2 image bounds"
    );
    let count = p[3] as usize;
    if w[24] != 0 {
        for n in 0..count {
            w[n] = (rng.next_draw()? % size[0]) as i32;
            w[4 + n] = (rng.next_draw()? % size[1]) as i32;
        }
        // The source initializes in WHOLE pixels, then treats these as
        // centipixels. Do not multiply this first random position by100.
        w[24] = 0;
    }
    // All X changes precede all Y changes. Chance, delta and sign draws are
    // not interleaved by particle, and degenerate modulo1 still draws.
    for axis in 0..2 {
        let speed = 8 + axis * 4;
        let spread = 16 + axis * 4;
        for n in 0..count {
            let rnd = w[spread + n];
            if rnd > 0 && rng.next_draw()? % p[7] as u32 == 0 {
                let mut delta = (rng.next_draw()? % rnd as u32) as i32;
                if rng.next_draw()? % 100 < 50 {
                    delta = delta.wrapping_neg();
                }
                let mut v = w[speed + n].wrapping_add(delta);
                let lo = p[1 + axis].wrapping_sub(p[4 + axis]).wrapping_mul(100);
                let hi = p[1 + axis].wrapping_add(p[4 + axis]).wrapping_mul(100);
                if v < lo {
                    v = lo;
                }
                if v > hi {
                    v = hi;
                }
                // Only the successful Y randomization branch clamps100.
                if axis == 1 && v < 100 {
                    v = 100;
                }
                w[speed + n] = v;
            }
        }
    }
    for n in 0..count {
        for axis in 0..2 {
            let i = axis * 4 + n;
            let unit = size[axis].min(canvas[axis]) as i32;
            let mut v = w[i].wrapping_add(w[8 + axis * 4 + n]);
            if (axis == 0 && v <= -unit * 100) || (axis == 1 && v < -unit * 100) {
                v = v.wrapping_add(unit * 1000);
            }
            if v >= unit * 100 {
                v %= unit * 100;
            }
            w[i] = v;
        }
    }
    Ok(())
}
pub(crate) fn image_input(
    frame: &PictureEffectFrame,
    p: &[i32; 8],
    w: &[i32; 25],
    mut base: ImageInput,
) -> Result<Option<ImageInput>> {
    validate(p)?;
    let Some(bank) = frame.bank.picture(frame.slot) else {
        return Ok(None);
    };
    let picture = &bank.picture;
    let source = if picture.has_mask {
        picture.raw.clone()
    } else {
        picture.keyed.clone()
    };
    let size = source.size();
    let Some((region, position)) = crysirror_image::coverage_bounds::intersect_source_window(
        [0, 0, size[0] as i32, size[1] as i32],
        [0, 0],
        picture.source_clip,
    )?
    else {
        return Ok(None);
    };
    // Keep the ORIGINAL period while making trimmed pixels transparent. A
    // cropped tile would incorrectly close the native CharaClip gaps.
    let source = ImageInput::Composite {
        base: Box::new(ImageInput::Solid {
            size,
            color: [0; 4],
        }),
        layers: vec![ImageCompositeLayer {
            input: Box::new(ImageInput::Resource(source)),
            source: region,
            destination: position,
            clip: None,
            opacity: 255,
            math: ImageCompositeMath::Copy,
        }],
    };
    for n in (0..p[3] as usize).rev() {
        let phase = [
            (-(i64::from(w[n]) / 100)).rem_euclid(i64::from(size[0])) as i32,
            (-(i64::from(w[4 + n]) / 100)).rem_euclid(i64::from(size[1])) as i32,
        ];
        base = ImageInput::Process {
            input: Box::new(ImageInput::Raster {
                destination: Box::new(base),
                source: Box::new(source.clone()),
            }),
            effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
                spec: ByteRaster::Tile {
                    source: [0, 0, size[0], size[1]],
                    destination: [0, 0, frame.canvas[0] as i32, frame.canvas[1] as i32],
                    phase,
                    first_run: None,
                    math: ByteRasterMath::AlphaWeightedSeparateRgb256 { weight: 256 },
                },
            }])?
            .into(),
        };
    }
    Ok(Some(base))
}
// CountIsMax TRUE leaves flag/command alone. CheckAllPrint TRUE and
// CheckNeedPicPrint FALSE: each reverse-ordered plane is the complete draw.

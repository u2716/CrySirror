// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! FISH111 original per-fish parameter work, RNG and atlas geometry.
//! All clipping and masked pixel work uses shared composition operations.
use crate::native_picture_effect::PictureEffectFrame;
use anyhow::{Result, ensure};
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageInput};
use crysirror_image::temporal_pattern::PatternRng;
pub(crate) fn parameters(values: &[i32], canvas: [u32; 2]) -> [i32; 176] {
    let defaults = [
        999999,
        1,
        32,
        32,
        0,
        0,
        canvas[0] as i32,
        canvas[1] as i32,
        1,
        1,
        1,
        1,
        3,
        100,
        0,
        0,
    ];
    // Source omitted14/15 copied uninitialized stack; unused initial work
    // is defined0. Active work is immediately replaced by InitFish draws.
    let mut p = [0; 176];
    for i in 0..16 {
        p[i] = values.get(i).copied().unwrap_or(defaults[i]);
    }
    p[0] = 999999;
    p[1] = p[1].min(40);
    p[12] = p[12].max(p[11]);
    p
}
pub(crate) fn validate(p: &[i32; 176]) -> Result<()> {
    ensure!(
        p[1] <= 40 && p[2] >= 0 && p[3] >= 0 && p[2] <= 16384 && p[3] <= 16384,
        "NNN FISH count/cell bounds"
    );
    Ok(())
}
fn draw(rng: &mut PatternRng, divisor: i32) -> Result<i32> {
    Ok((rng.next_draw()? % divisor.max(1) as u32) as i32)
}
pub(crate) fn initialize(p: &mut [i32; 176], rng: &mut PatternRng) -> Result<()> {
    validate(p)?;
    for n in 0..p[1].max(0) as usize {
        let k = 16 + n * 4;
        let dx = p[6].wrapping_sub(p[2]).max(1);
        let dy = p[7].wrapping_sub(p[3]).max(1);
        p[k] = p[4].wrapping_add(draw(rng, dx)?);
        p[k + 1] = p[5].wrapping_add(draw(rng, dy)?);
        let mut speed =
            p[11].wrapping_add(draw(rng, p[12].wrapping_sub(p[11]).wrapping_add(1).max(1))?);
        if draw(rng, 100)? < 50 {
            speed = speed.wrapping_neg();
        }
        p[k + 2] = speed;
        p[k + 3] = 0;
    }
    Ok(())
}
pub(crate) fn calcu(p: &mut [i32; 176], count: &mut i32, rng: &mut PatternRng) -> Result<()> {
    validate(p)?;
    if *count > 10000 {
        *count = 0;
    }
    let divisor = p[12].wrapping_sub(p[11]).wrapping_add(1).max(1);
    let anime = p[9].wrapping_mul(p[10]).max(1);
    for n in 0..p[1].max(0) as usize {
        let k = 16 + 4 * n;
        let mut speed = p[k + 2];
        if p[13] > 0 && draw(rng, 10000)? < p[13] {
            speed = p[11].wrapping_add(draw(rng, divisor)?);
            if draw(rng, 100)? < 50 {
                speed = speed.wrapping_neg();
            }
            p[k + 2] = speed;
            p[k + 3] = 0;
            let mut y = p[k + 1].wrapping_add(draw(rng, 5)?).wrapping_sub(2);
            if y < p[5] {
                y = p[5];
            }
            if y.wrapping_add(p[3]) > p[5].wrapping_add(p[7]) {
                y = p[5].wrapping_add(p[7]).wrapping_sub(p[3]);
            }
            p[k + 1] = y;
        }
        let mut x = p[k].wrapping_add(p[k + 2]);
        if x < p[4] {
            x = p[4];
            if speed < 0 {
                speed = speed.wrapping_neg();
            }
        }
        if x.wrapping_add(p[2]) > p[4].wrapping_add(p[6]) {
            x = p[4].wrapping_add(p[6]).wrapping_sub(p[2]);
            if speed > 0 {
                speed = speed.wrapping_neg();
            }
        }
        p[k + 2] = speed;
        p[k] = x;
        p[k + 3] = p[k + 3].wrapping_add(1) % anime;
    }
    Ok(())
}
pub(crate) fn image_input(
    frame: &PictureEffectFrame,
    p: &[i32; 176],
    base: ImageInput,
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
    let anime_frame = p[10].max(1);
    let adiv = p[9].wrapping_mul(anime_frame).max(1);
    let mut layers = Vec::new();
    for n in 0..p[1].max(0) as usize {
        let k = 16 + n * 4;
        let mut sx = if p[k + 2] > 0 { p[2] } else { 0 };
        if p[8] > 1 {
            sx = sx.wrapping_add(p[2].wrapping_mul(2).wrapping_mul(n as i32 % p[8]));
        }
        let sy = if p[9] > 1 {
            p[3].wrapping_mul((p[k + 3] % adiv) / anime_frame)
        } else {
            0
        };
        let Some((region, destination)) =
            crysirror_image::coverage_bounds::intersect_source_window(
                [sx, sy, p[2], p[3]],
                [p[k], p[k + 1]],
                picture.source_clip,
            )?
        else {
            continue;
        };
        layers.push(ImageCompositeLayer {
            input: Box::new(ImageInput::Resource(source.clone())),
            source: region,
            destination,
            clip: None,
            opacity: 255,
            math: ImageCompositeMath::MaskSeparate256,
        });
    }
    if layers.is_empty() {
        return Ok(None);
    }
    Ok(Some(ImageInput::Composite {
        base: Box::new(base),
        layers,
    }))
}
// Empty inherited InitParameter preserves all176 saved work/parameter words;
// no SetParam or RNG runs during restore. CountIsMax FALSE, no ordinary underlay.

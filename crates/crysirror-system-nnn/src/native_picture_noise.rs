// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! NOISE106 native band/counter/random policy from effectNoise.cpp. The
//! software32 branch uses the same shared masked row copies as CHARARASTER;
//! its Win64 source-pointer clipping defect is not inherited. No pixel loop
//! or image allocation belongs to this original VM policy.
use crate::native_picture_effect::PictureEffectFrame;
use anyhow::{Context, Result, ensure};
use crysirror_core::image_processing::ImageInput;
use crysirror_image::{
    byte_raster::{ByteRaster, ByteRasterMath},
    effects::ImageEffect,
    row_blit::RowBlit,
    temporal_pattern::PatternRng,
};
const MAX_BAND_ROWS: i32 = 65536;
pub(crate) fn parameters(p: &[i32]) -> [i32; 144] {
    let mut out = [0; 144];
    for i in 0..16 {
        let default = match i {
            0 => 20,
            1 | 3 => 2,
            2 => 32,
            4 => 40,
            5 | 6 => 50,
            9 => 100,
            10 => out[9],
            _ => 0,
        };
        // Source para13..15 and unused band scratch words were uninitialised.
        // Use explicit zero defaults; original saves hydrate all144 live words.
        out[i] = p.get(i).copied().unwrap_or(default);
    }
    out[1] = out[1].min(16);
    out
}
fn draw(random: &mut PatternRng, divisor: i32) -> Result<i32> {
    let value = random.next_draw()? as i32;
    value
        .checked_rem(divisor)
        .context("NNN NOISE native zero random modulus")
}
fn init_band(p: &mut [i32; 144], n: usize, height: i32, random: &mut PatternRng) -> Result<()> {
    let k = 16 + n * 8;
    let mut wait = p[6];
    let dw = wait.wrapping_mul(2) / 10;
    if dw > 0 {
        wait = wait
            .wrapping_add(draw(random, dw.wrapping_mul(2))?)
            .wrapping_sub(dw)
            .max(1);
    }
    p[k] = wait;
    let mut len = p[4];
    let variation = p[5].wrapping_mul(len) / 100;
    if variation > 0 {
        len = len
            .wrapping_add(draw(random, variation.wrapping_mul(2))?)
            .wrapping_sub(variation);
    }
    len = len.max(1);
    ensure!(len <= MAX_BAND_ROWS, "NNN NOISE band row budget");
    p[k + 2] = len;
    p[k + 1] = draw(random, height.wrapping_sub(len).max(1))?;
    p[k + 3] = 20 + draw(random, 60)?;
    Ok(())
}
pub(crate) fn initialize(p: &mut [i32; 144], height: i32, random: &mut PatternRng) -> Result<()> {
    for n in 0..p[1].max(0) as usize {
        init_band(p, n, height, random)?;
    }
    Ok(())
}
pub(crate) fn calcu(p: &mut [i32; 144], height: i32, random: &mut PatternRng) -> Result<()> {
    for n in 0..p[1].max(0) as usize {
        let k = 16 + n * 8;
        p[k] = p[k].wrapping_sub(1);
        if p[k] <= 0 {
            init_band(p, n, height, random)?;
        }
        let y = p[k + 1].wrapping_add(p[7]);
        p[k + 1] = y;
        if y < p[k + 2].wrapping_neg() || y >= height {
            init_band(p, n, height, random)?;
        }
        let mut power = p[k + 3].wrapping_add(draw(random, 10)? - 6);
        if draw(random, 100)? < 2 {
            power = power.wrapping_add(draw(random, 160)? - 40);
        }
        p[k + 3] = power.clamp(30, 100);
    }
    Ok(())
}
pub(crate) fn validate(p: &[i32; 144]) -> Result<()> {
    ensure!(p[1] <= 16, "NNN NOISE band count");
    // Zero length can occur only in an as-yet uninitialised parameter candidate.
    // Calcu initializes an expired bank before any render uses its lengths.
    for n in 0..p[1].max(0) as usize {
        ensure!(
            (0..=MAX_BAND_ROWS).contains(&p[16 + n * 8 + 2]),
            "NNN NOISE band row budget"
        );
    }
    Ok(())
}
pub(crate) fn image_input(
    frame: &PictureEffectFrame,
    p: [i32; 144],
    base: ImageInput,
    random: &mut PatternRng,
) -> Result<Option<ImageInput>> {
    validate(&p)?;
    let weight = if p[9] == 100 && p[10] == 100 {
        256
    } else {
        (crate::native_curve::value_with_random(
            frame.state.count,
            frame.state.frames,
            p[10].wrapping_mul(256),
            p[9].wrapping_mul(256),
            p[12],
            p[11],
            random,
        )? / 100)
            .clamp(0, 256)
    };
    let [_, h] = frame.canvas.map(|n| n as i32);
    let mut offsets = vec![0i32; h as usize];
    // Print fills the full screen table before clipping, even with no mask.
    if p[8] > 0 {
        for row in &mut offsets {
            let displacement = draw(random, p[8])?.wrapping_add(50) / 100;
            if displacement > 0 {
                *row = displacement.wrapping_mul(draw(random, 2)? * 2 - 1);
            }
        }
    }
    let wave = p[3].max(1);
    let [pw, ph] = frame
        .source
        .as_ref()
        .map(|s| s.size().map(|n| n as i32))
        .unwrap_or([1, 1]);
    let [x, y, _, _] = frame.rectangles.destination;
    let sy = (-i64::from(y)).max(0).min(i64::from(ph)) as i32;
    let dy = y.max(0);
    let rows = (i64::from(ph) - i64::from(sy))
        .min(i64::from(h) - i64::from(dy))
        .max(0) as i32;
    if rows == 0 {
        return Ok(None);
    }
    for n in 0..p[1].max(0) as usize {
        let k = 16 + n * 8;
        let start = p[k + 1].wrapping_add(dy);
        let len = p[k + 2];
        ensure!(
            (0..=MAX_BAND_ROWS).contains(&len),
            "NNN NOISE band row budget"
        );
        if len == 0 {
            continue;
        }
        let power = (p[2].wrapping_mul(p[k + 3]) / 100).max(1);
        let half = (len / 2).max(1);
        let denominator = half.wrapping_mul(wave).wrapping_mul(100);
        ensure!(
            denominator != 0 && wave.wrapping_mul(2) != 0,
            "NNN NOISE native wave division domain"
        );
        let mut amplitude = 100;
        // Do not crop this native loop: offscreen rows still consume rand().
        for row in 0..len {
            if row % wave == 0 {
                amplitude = draw(random, 70)? + 30;
            }
            let wave_mul = wave.wrapping_sub((row % wave).wrapping_mul(2));
            let sign = if row % wave.wrapping_mul(2) >= wave {
                -1
            } else {
                1
            };
            let mut envelope = row % half;
            if row % len >= half {
                if row % len < half.wrapping_mul(2) {
                    envelope = half.wrapping_sub(envelope);
                } else {
                    envelope = 0;
                }
            }
            let delta = amplitude
                .wrapping_mul(envelope)
                .wrapping_mul(wave_mul)
                .wrapping_mul(power)
                .checked_div(denominator)
                .context("NNN NOISE native wave division overflow")?
                .wrapping_mul(sign);
            let at = start.wrapping_add(row);
            if (0..h).contains(&at) {
                offsets[at as usize] = offsets[at as usize].wrapping_add(delta);
            }
        }
    }
    if !frame.has_mask {
        return Ok(None);
    }
    let Some(source) = frame.source.as_ref() else {
        return Ok(None);
    };
    let spec = RowBlit {
        row_math: Vec::new(),
        source: [0, sy, pw, rows],
        destination: [x, dy],
        row_offsets: offsets[dy as usize..(dy + rows) as usize].to_vec(),
        math: ByteRasterMath::AlphaWeightedRgb256 {
            weight: weight as u16,
        },
    };
    spec.validate()?;
    Ok(Some(ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(base),
            source: Box::new(ImageInput::Resource(source.clone())),
        }),
        effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
            spec: ByteRaster::RowBlit { spec },
        }])?
        .into(),
    }))
}

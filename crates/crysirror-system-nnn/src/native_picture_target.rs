// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! TARGET1(103): effectTarget1.cpp and 32-bit BoxFill/TransBoxFill/AddBoxFill.
//! These are native rectangle/clock/packed-colour parameters; shared RowBlit
//! performs clipping and stored-byte operations. Overlapping lines retain order.
use crate::native_picture_effect::PictureEffectFrame;
use anyhow::Result;
use crysirror_core::image_processing::ImageInput;
use crysirror_image::{
    byte_raster::{ByteRaster, ByteRasterMath},
    effects::ImageEffect,
    row_blit::RowBlit,
    temporal_pattern::PatternRng,
};
pub(crate) fn parameters(p: &[i32], canvas: [u32; 2]) -> [i32; 16] {
    let [w, h] = canvas.map(|n| n as i32);
    let mut out = [0; 16];
    for i in 0..16 {
        let default = match i {
            0 => 20,
            1 => w / 4,
            2 => h / 4,
            3 => w / 2,
            4 => h / 2,
            5 => 1,
            6..=8 => 255,
            13 => 50,
            _ => 0,
        };
        // Omitted para14/15 were uninitialised stack words in the source.
        // Define them as zero, retaining supplied or saved values.
        out[i] = p.get(i).copied().unwrap_or(default);
    }
    out[1] = out[1].clamp(0, w - 1);
    out[2] = out[2].clamp(0, h - 1);
    if out[1].wrapping_add(out[3]) > w {
        out[3] = w - out[1];
    }
    if out[2].wrapping_add(out[4]) > h {
        out[4] = h - out[2];
    }
    out[3] = out[3].max(1);
    out[4] = out[4].max(1);
    out[5] = out[5].clamp(1, w);
    // The final source check really reads red(para6), not line width(para5).
    // Keep this defined native dependency rather than silently correcting it.
    if out[6] > h {
        out[5] = h;
    }
    out
}
fn packed_rgb(color: [i32; 3]) -> [u8; 4] {
    let col =
        (color[0] as u32).wrapping_shl(16) | (color[1] as u32).wrapping_shl(8) | color[2] as u32;
    [(col >> 16) as u8, (col >> 8) as u8, col as u8, 255]
}
pub(crate) fn rectangle(
    base: ImageInput,
    canvas: [u32; 2],
    rect: [i32; 4],
    color: [u8; 4],
    math: ByteRasterMath,
) -> Result<ImageInput> {
    let [x, y, w, h] = rect;
    if w <= 0 || h <= 0 {
        return Ok(base);
    }
    let Some(([_, _, cw, ch], [dx, dy])) = crysirror_core::region_map::clip_region_copy(
        Some([w as u32, h as u32]),
        canvas,
        [0, 0, i64::from(w), i64::from(h)],
        [i64::from(x), i64::from(y)],
        None,
    )?
    else {
        return Ok(base);
    };
    let spec = RowBlit {
        row_math: Vec::new(),
        source: [0, 0, cw as i32, ch as i32],
        destination: [dx as i32, dy as i32],
        row_offsets: vec![0; ch as usize],
        math,
    };
    spec.validate()?;
    Ok(ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(base),
            source: Box::new(ImageInput::Solid {
                size: [cw as u32, ch as u32],
                color,
            }),
        }),
        effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
            spec: ByteRaster::RowBlit { spec },
        }])?
        .into(),
    })
}
pub(crate) fn image_input(
    frame: &PictureEffectFrame,
    p: [i32; 16],
    mut base: ImageInput,
    random: &mut PatternRng,
) -> Result<Option<ImageInput>> {
    let ps = crate::native_curve::value_with_random(
        frame.state.count,
        frame.state.frames,
        p[12].wrapping_mul(100),
        10000,
        p[11],
        p[10],
        random,
    )?;
    let [w, h] = frame.canvas.map(|n| n as i32);
    let line = p[5];
    let lerp =
        |start: i32, end: i32| start.wrapping_add(end.wrapping_sub(start).wrapping_mul(ps) / 10000);
    let x1 = lerp(0, p[1]);
    let x2 = lerp(
        w.wrapping_sub(line),
        p[1].wrapping_add(p[3]).wrapping_sub(line),
    );
    let y1 = lerp(0, p[2]);
    let y2 = lerp(
        h.wrapping_sub(line),
        p[2].wrapping_add(p[4]).wrapping_sub(line),
    );
    let rects = [
        [x1, 0, line, h],
        [x2, 0, line, h],
        [0, y1, w, line],
        [0, y2, w, line],
    ];
    let color = [p[6], p[7], p[8]];
    for rect in rects {
        match p[9] {
            0 => {
                base = rectangle(
                    base,
                    frame.canvas,
                    rect,
                    packed_rgb(color),
                    ByteRasterMath::CopyRgb,
                )?
            }
            1 => {
                let a = p[13].clamp(0, 100) * 256 / 100;
                // Native integer percent never produces255; for a<255,
                // floor(255*(a+1)/256)==a. At256 native opaque copy applies.
                let weight = if a == 256 { 256 } else { a + 1 } as u16;
                base = rectangle(
                    base,
                    frame.canvas,
                    rect,
                    packed_rgb(color),
                    ByteRasterMath::AlphaWeightedRgb256 { weight },
                )?;
            }
            _ => {
                // AddBoxFill is called without a percent argument (default100).
                let delta = color.map(|v| v.wrapping_mul(100) / 100);
                base = rectangle(
                    base,
                    frame.canvas,
                    rect,
                    packed_rgb(delta.map(|v| v.max(0))),
                    ByteRasterMath::SaturatedRgb { subtract: false },
                )?;
                base = rectangle(
                    base,
                    frame.canvas,
                    rect,
                    packed_rgb(delta.map(|v| v.min(0).wrapping_neg())),
                    ByteRasterMath::SaturatedRgb { subtract: true },
                )?;
            }
        }
    }
    Ok(Some(base))
}

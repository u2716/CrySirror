// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CHARARASTER107: effectCharaRaster.cpp's 32-bit software branch. The Win64
//! port omits transparency and has a clipping/mask-pointer mismatch; the 3D
//! branch samples a texture displacement plane. Neither replaces this profile.
//! Native phase/rows are data; shared RowBlit owns all pixel reads and writes.
use crate::native_picture_effect::PictureEffectFrame;
use anyhow::Result;
use crysirror_core::image_processing::ImageInput;
use crysirror_image::{
    byte_raster::{ByteRaster, ByteRasterMath},
    effects::ImageEffect,
    row_blit::RowBlit,
    temporal_pattern::PatternRng,
};
pub(crate) fn parameters(p: &[i32]) -> [i32; 16] {
    let mut out = [0; 16];
    for i in 0..16 {
        let default = match i {
            0 => 20,
            1 => 32,
            2 => 10,
            3 | 6 => 100,
            7 => out[6],
            _ => 0,
        };
        // The source copies omitted para10..15 from uninitialised stack data.
        // Zero is the defined replacement; saved/supplied words are retained.
        out[i] = p.get(i).copied().unwrap_or(default);
    }
    out
}
pub(crate) fn image_input(
    frame: &PictureEffectFrame,
    p: [i32; 16],
    base: ImageInput,
    random: &mut PatternRng,
) -> Result<Option<ImageInput>> {
    // AllEffect ordinary picture banks call LoadDWQ with b256Flag=FALSE,
    // including palette files, so their engine resources are expanded RGB.
    // The source's Check256 early return only applies to a separately created
    // indexed picture; no such object is admitted by this bank's loader.
    let raster = crate::native_curve::value_with_random(
        frame.state.count,
        frame.state.frames,
        0,
        p[1].wrapping_mul(100),
        p[5],
        p[4],
        random,
    )?;
    let weight = if p[6] == 100 && p[7] == 100 {
        256
    } else {
        (crate::native_curve::value_with_random(
            frame.state.count,
            frame.state.frames,
            p[7].wrapping_mul(256),
            p[6].wrapping_mul(256),
            p[9],
            p[8],
            random,
        )? / 100)
            .clamp(0, 256)
    };
    // AllEffect::GetPicture returns an allocated CPicture even before load.
    // An empty bank still consumes these curve draws before its eventual empty-mask exit.
    let Some(source) = frame.source.as_ref() else {
        return Ok(None);
    };
    let [w, h] = frame.canvas.map(|n| n as i32);
    let [pw, ph] = source.size().map(|n| n as i32);
    let [x, y, _, _] = frame.rectangles.destination;
    let sy = (-i64::from(y)).max(0).min(i64::from(ph)) as i32;
    let rows = (i64::from(ph) - i64::from(sy))
        .min(i64::from(h) - i64::from(y).max(0))
        .max(0) as i32;
    if rows == 0 {
        return Ok(None);
    }
    let amplitude = f64::from(raster) * 0.01;
    let mut k = 1.0 / f64::from(h);
    k *= 3.14159 * 2.0 * 3.0;
    k *= f64::from(p[3]);
    k *= 0.01;
    let phase = frame.state.count.wrapping_mul(p[2]);
    let mut row_offsets = Vec::with_capacity(rows as usize);
    for j in 0..rows {
        let theta = f64::from(sy.wrapping_add(j).wrapping_add(phase)) * k;
        let mut d = (theta.cos() * amplitude + 0.5) as i32;
        if d <= -w {
            d = -(w - 1);
        }
        if d >= w - 1 {
            d = w - 2;
        }
        row_offsets.push(d);
    }
    if !frame.has_mask {
        return Ok(None);
    }
    let spec = RowBlit {
        row_math: Vec::new(),
        source: [0, sy, pw, rows],
        destination: [x, y.max(0)],
        row_offsets,
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

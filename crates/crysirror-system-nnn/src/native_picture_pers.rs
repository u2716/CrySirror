// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! PERS67 original software32 angular row geometry and wrapping16.16 phase.
//! Reusable scanline sampling/mask arithmetic stays in the shared Image worker.
use crate::native_picture_effect::PictureEffectFrame;
use anyhow::{Result, ensure};
use crysirror_core::image_processing::ImageInput;
use crysirror_image::{
    byte_raster::{ByteRaster, ByteRasterMath},
    effects::ImageEffect,
    fixed_scanlines::{FixedSampleSpan, FixedScanlines, ScanlineMath},
};
pub(crate) fn parameters(values: &[i32], canvas: [u32; 2]) -> [i32; 16] {
    let mut p = [0; 16];
    for i in 0..16 {
        p[i] = values.get(i).copied().unwrap_or(match i {
            0 => 20,
            3 | 4 => 100,
            6 => p[1],
            7 => p[2],
            8 => p[3],
            9 => p[4],
            11 => canvas[1] as i32,
            12 => p[10],
            13 => p[11],
            _ => 0,
        });
    }
    // Omitted14/15 were copied from uninitialized stack and are never read;
    // define their initial contents0 while preserving all16 raw saved words.
    p
}
pub(crate) fn validate(p: &[i32; 16]) -> Result<()> {
    ensure!(
        p[10..14].iter().all(|v| v.unsigned_abs() <= 1_000_000),
        "NNN PERS row geometry budget"
    );
    Ok(())
}
pub(crate) fn image_input(
    frame: &PictureEffectFrame,
    p: &[i32; 16],
    base: ImageInput,
) -> Result<Option<ImageInput>> {
    validate(p)?;
    if !frame.geometry_assigned {
        return Ok(None);
    }
    let Some(picture) = frame.bank.picture(frame.slot).map(|b| &b.picture) else {
        return Ok(None);
    };
    let frames = frame.state.frames.max(1);
    let count = frame.state.count.min(frames);
    let interpolate = |start: i32, end: i32| {
        start.wrapping_add(end.wrapping_sub(start).wrapping_mul(count) / frames)
    };
    let dist1 = interpolate(p[6], p[1]);
    let dist2 = interpolate(p[7], p[2]);
    let mut height = interpolate(p[8], p[3]);
    if height == 0 {
        height = 1;
    }
    let percent = interpolate(p[9], p[4]).max(1);
    let clip_start = interpolate(p[12], p[10]).max(0);
    let clip_end = interpolate(p[13], p[11]).min(frame.canvas[1] as i32 - 1);
    let rows = clip_end.wrapping_sub(clip_start).wrapping_add(1);
    if rows < 1 {
        return Ok(None);
    }
    let size = picture.raw.size();
    let center_x = size[0] as i32 / 2;
    let add_x = frame.rectangles.destination[0]
        .wrapping_neg()
        .wrapping_add(center_x);
    let add_y = frame.rectangles.destination[1].wrapping_neg();
    let theta1 = f64::from(dist1).atan2(f64::from(height));
    let theta2 = f64::from(dist2).atan2(f64::from(height));
    let delta = theta2 - theta1;
    let squared = dist1
        .wrapping_mul(dist1)
        .wrapping_add(height.wrapping_mul(height));
    let radius = f64::from(squared).sqrt();
    // Source overflow/negative sqrt or zero radial scale fed undefined FPU
    // integer conversions. Define those exceptional profiles as no drawing.
    if !radius.is_finite() || radius == 0.0 {
        return Ok(None);
    }
    let mut scale = 1.0 / radius;
    scale /= f64::from(percent) / 100.0;
    let mut mode = p[5];
    if mode == 2 && !picture.has_mask {
        mode = 1;
    }
    // Only MODE0 is active: MODE1/2 bodies are commented out. Its mode1 does
    // NOT apply RGB keying. >=2 performs the raw mask/255-complement sum with
    // no zero/opaque shortcuts (even mask0 darkens destination by /256).
    let use_mask = (mode as u32) >= 2 && picture.mask_storage.is_some();
    let source = if use_mask {
        picture.mask_storage.as_ref().unwrap().view.clone()
    } else {
        picture.raw.clone()
    };
    let math = if use_mask {
        ScanlineMath::CoverageRawSummed {
            weight: 256,
            complement: 255,
        }
    } else {
        ScanlineMath::Stored {
            math: ByteRasterMath::CopyRgb,
        }
    };
    // Other unsigned modes>=2 with no real mask dereferenced NULL in source; define bounded opaque
    // coverage/copy instead. Original0/1 remain RGB-only without mask/key.
    let mut spans = Vec::with_capacity(rows as usize);
    let mut yy = 0.0;
    for row in 0..rows {
        let source_row = (yy + 0.5) as i32;
        let theta = theta1 + (delta * f64::from(row)) / f64::from(rows);
        let dot = (f64::from(height) / theta.cos()) * scale;
        if !dot.is_finite() {
            break;
        }
        // Native packs integer and fractional words separately using OR;
        // a negative fractional word must not be recomputed as one cast.
        let high = dot as i32;
        let low = ((dot - f64::from(high)) * 65536.0) as i32;
        let step = high.wrapping_shl(16) | low;
        let origin = add_x
            .wrapping_shl(16)
            .wrapping_sub(step.wrapping_mul(frame.canvas[0] as i32 / 2));
        let sy = source_row.wrapping_add(add_y);
        // Unsigned cmp against source height rejects negative rows. The
        // horizontal source accumulator still wraps in the common kernel.
        if sy >= 0 && (sy as u32) < size[1] {
            spans.push(FixedSampleSpan {
                destination: [0, clip_start + row, frame.canvas[0] as i32],
                source_q16: [i64::from(origin), i64::from(sy.wrapping_shl(16))],
                source_step_q16: [i64::from(step), 0],
                math,
            });
        }
        yy += dot;
    }
    let spec = FixedScanlines {
        spans,
        corner_rgb_key: false,
        source_color: None,
    };
    let operation = ByteRaster::FixedScanlinesWrapping32 { spec };
    operation.validate()?;
    // CPicture source clipping is deliberately absent in this raw GetBuffer
    // branch. The shader/D3D branch has different geometry, not this profile.
    Ok(Some(ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(base),
            source: Box::new(ImageInput::Resource(source)),
        }),
        effects: serde_json::to_vec(&[ImageEffect::ByteRaster { spec: operation }])?.into(),
    }))
}
// CountIsMaxFALSE; Init/BeginPrint/Calcu/End/inheritedInitParameter empty.
// CheckNeedPicPrintFALSE. Saved parameters, count/max and rectangles hydrate
// directly; no constructor, random draw or setter replays on restore.

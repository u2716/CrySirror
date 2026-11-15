// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! SOYOSOYO92 native clipped-height wind phase and per-slot clock words.
//! Shared fixed scanlines implement DeltaBlt's separate mask/255-complement
//! floors, immutable source reads and final displaced destination clipping.
use crate::native_picture_effect::PictureEffectFrame;
use anyhow::Result;
use crysirror_core::image_processing::ImageInput;
use crysirror_image::{
    byte_raster::ByteRaster,
    effects::ImageEffect,
    fixed_scanlines::{FixedSampleSpan, FixedScanlines, ScanlineMath},
};
pub(crate) fn parameters(values: &[i32]) -> [i32; 13] {
    let defaults = [20, 20, 10, 10, 50, 2, 1, -1, 0, 0, 1, 1, 0];
    std::array::from_fn(|i| values.get(i).copied().unwrap_or(defaults[i]))
}
pub(crate) fn calcu(p: &mut [i32; 13], count: &mut i32) {
    if p[6] != 0 && p[1] > 0 {
        *count %= p[1];
    }
    p[12] = p[12].wrapping_add(p[11]) % p[10].max(1);
}
pub(crate) fn image_input(
    frame: &PictureEffectFrame,
    p: &[i32; 13],
    base: ImageInput,
) -> Result<Option<ImageInput>> {
    if p[5] != 2 {
        return Ok(None);
    }
    let slot = (if p[7] == -1 { frame.slot } else { p[7] }).clamp(0, frame.bank_limit - 1);
    let Some(picture) = frame.bank.picture(slot).map(|p| &p.picture) else {
        return Ok(None);
    };
    // DeltaBlt(TRUE) has active code for md2/mask and md0/opaque only. If
    // CheckMaskExistFALSE it chooses md1, which performs NO raster operation.
    if !picture.has_mask {
        return Ok(None);
    }
    let size = picture.raw.size();
    let point = [
        frame.rectangles.destination[0],
        frame.rectangles.destination[1],
    ];
    let Some((source, destination)) = crysirror_image::coverage_bounds::intersect_source_window(
        [0, 0, size[0] as i32, size[1] as i32],
        point,
        picture.source_clip,
    )?
    else {
        return Ok(None);
    };
    // Clip() runs BEFORE wind phase: its resulting height determines sine
    // frequency. Cropping after displacement would change offscreen motion.
    let Some((destination_rect, source_origin)) =
        crysirror_image::coverage_bounds::intersect_source_window(
            [destination[0], destination[1], source[2], source[3]],
            [source[0], source[1]],
            [0, 0, frame.canvas[0] as i32, frame.canvas[1] as i32],
        )?
    else {
        return Ok(None);
    };
    let [x, y, width, height] = destination_rect;
    let mut count = frame.state.count;
    if p[1] > 0 {
        count %= p[1];
    }
    if count >= frame.state.frames {
        count = frame.state.frames.wrapping_sub(1);
    }
    count = count.max(0);
    let boundary = p[4].wrapping_mul(p[1]) / 100;
    let reverse = count >= boundary;
    let duration = if reverse {
        count = count.wrapping_sub(boundary);
        p[1].wrapping_sub(boundary)
    } else {
        boundary
    }
    .max(1);
    let mut theta = f64::from(count);
    theta /= f64::from(duration);
    theta *= 3.14159;
    let mut mix = 1.0 - theta.cos();
    mix *= 0.5;
    let right = f64::from(p[2]);
    let left = f64::from(p[3]);
    let displacement = if reverse {
        right - (right + left) * mix
    } else {
        -left + (right + left) * mix
    };
    let theta_row = 3.14159 / f64::from(height) / 2.0;
    // The source computes a divide-by-zero even for an unused scatter term;
    // only active scatter with omega0 was undefined. Define its period1.
    let period = if p[10] == 0 { 1 } else { p[10] };
    let scatter_phase = 3.14159 * 2.0 / f64::from(period);
    let mut spans = Vec::with_capacity(height as usize);
    for row in 0..height {
        let k = if p[8] != 0 { height - row } else { row };
        let weight = 1.0 - (f64::from(k) * theta_row).sin();
        let scatter = if p[9] > 0 {
            f64::from(p[9]) * (f64::from(row.wrapping_add(p[12]) % period) * scatter_phase).cos()
        } else {
            0.0
        };
        let offset = ((displacement + scatter) * weight + 0.5) as i32;
        spans.push(FixedSampleSpan {
            destination: [x.wrapping_add(offset), y + row, width],
            source_q16: [
                i64::from(source_origin[0]) << 16,
                i64::from(source_origin[1] + row) << 16,
            ],
            source_step_q16: [65536, 0],
            math: ScanlineMath::CoverageSeparate {
                weight: 256,
                opaque_copy: true,
            },
        });
    }
    let spec = ByteRaster::FixedScanlines {
        spec: FixedScanlines {
            spans,
            corner_rgb_key: false,
            source_color: None,
        },
    };
    spec.validate()?;
    Ok(Some(ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(base),
            source: Box::new(ImageInput::Resource(picture.raw.clone())),
        }),
        effects: serde_json::to_vec(&[ImageEffect::ByteRaster { spec }])?.into(),
    }))
}
// SetParam stores13words/count0/maxpara0. Empty InitParameter preserves all
// words, including scatter work12, and saved count/max. CountIsMaxFALSE,
// CheckAllPrintTRUE/CheckNeedPicPrintFALSE; shader is a separate profile.

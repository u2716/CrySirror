// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! effectBokashi77 native parameters, crop and MMX quantisation policy. The
//! shared sliding BoxReduce service owns sums, pixels and cancellation.
use crate::native_picture_effect::PictureEffectFrame;
use anyhow::{Result, ensure};
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageInput};
use crysirror_image::{
    box_reduce::{BoxAlpha, BoxAnchors, BoxArithmetic, BoxReduce},
    effects::ImageEffect,
    temporal_pattern::PatternRng,
};
pub(crate) fn parameters(p: &[i32]) -> [i32; 16] {
    let mut out = [0; 16];
    let supplied = p.len().min(16);
    out[..supplied].copy_from_slice(&p[..supplied]);
    let get = |i: usize, d| p.get(i).copied().unwrap_or(d);
    out[0] = get(0, 20);
    out[1] = get(1, 8).clamp(1, 256);
    out[2] = get(2, 8).clamp(1, 256);
    if out[1] * out[2] > 256 {
        out[2] = 256 / out[1];
    }
    out[3] = get(3, 100).clamp(0, 100);
    out[4] = get(4, out[3]).clamp(0, 100);
    out[5] = get(5, 0);
    out[6] = get(6, 0).max(0);
    out[7] = get(7, 0);
    // Source copies unused stack words8..15. Omitted words are initialized;
    // explicitly supplied/saved values retain their original record positions.
    out
}
pub(crate) fn image_input(
    frame: &PictureEffectFrame,
    p: [i32; 16],
    base: ImageInput,
    random: &mut PatternRng,
) -> Result<Option<ImageInput>> {
    let phase = crate::native_curve::value_with_random(
        frame.state.count,
        frame.state.frames,
        p[4],
        p[3],
        p[6],
        p[5],
        random,
    )?;
    let mut kernel = [
        p[1].wrapping_mul(phase) / 100,
        p[2].wrapping_mul(phase) / 100,
    ];
    if p[7] != 0 {
        for value in &mut kernel {
            if *value > 1 && *value & 1 == 0 {
                *value -= 1;
            }
        }
    }
    kernel = kernel.map(|v| v.clamp(1, 256));
    if kernel[0] * kernel[1] > 256 {
        kernel[1] = 256 / kernel[0];
    }
    let Some(source) = &frame.source else {
        return Ok(None);
    };
    let [x, y, width, height] = frame.rectangles.destination;
    let [sx, sy] = [-i64::from(x), -i64::from(y)];
    let [w, h] = frame.canvas;
    if sx < 0
        || sy < 0
        || sx + i64::from(w) > i64::from(width)
        || sy + i64::from(h) > i64::from(height)
    {
        return Ok(None);
    }
    if kernel == [1, 1] {
        return Ok(Some(ImageInput::Composite {
            base: Box::new(base),
            layers: vec![ImageCompositeLayer {
                input: Box::new(ImageInput::Resource(source.clone())),
                source: [0, 0, width, height],
                destination: [x, y],
                clip: None,
                opacity: 255,
                math: ImageCompositeMath::CopyRgb,
            }],
        }));
    }
    ensure!(
        source.size()[0] as i32 == width,
        "NNN BOKASHI native source pitch differs from picture width"
    );
    // Native kernels wider/taller than the screen overread scratch storage.
    // Restrict their footprint to initialized pixels rather than emulating RAM.
    kernel[0] = kernel[0].min(w as i32);
    kernel[1] = kernel[1].min(h as i32);
    let area = kernel[0] * kernel[1];
    let multiplier = (65536 / area).min(32767) as u16;
    let spec = BoxReduce {
        size: frame.canvas,
        source: Some([sx as u32, sy as u32, w, h]),
        window: kernel.map(|v| v as u16),
        anchors: BoxAnchors::WindowOrigins,
        arithmetic: BoxArithmetic::HalfWordHighDouble { multiplier },
        alpha: BoxAlpha::Opaque,
    };
    Ok(Some(ImageInput::Process {
        input: Box::new(ImageInput::Resource(source.clone())),
        effects: serde_json::to_vec(&[ImageEffect::BoxReduce { spec }])?.into(),
    }))
}

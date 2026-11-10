// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! effectStretch's native seven parameters/countdown and branch selection.
//! BoxReduce/RectMap perform every sample; the adapter has no raster buffer.
use crate::native_picture_effect::PictureEffectFrame;
use anyhow::{Result, ensure};
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageInput};
use crysirror_image::{effects::ImageEffect, temporal_pattern::PatternRng};
pub(crate) fn parameters(p: &[i32], canvas: [u32; 2]) -> Result<[i32; 7]> {
    ensure!(
        canvas.iter().all(|v| (1..=16384).contains(v)),
        "NNN STRETCH canvas"
    );
    let [w, h] = canvas.map(|v| v as i32);
    let get = |i: usize, d| p.get(i).copied().unwrap_or(d);
    let x = get(1, w / 4).clamp(0, w - 1);
    let y = get(2, h / 4).clamp(0, h - 1);
    let width = get(3, w - x * 2).min(w - x).max(1);
    let height = get(4, h - y * 2).min(h - y).max(1);
    Ok([get(0, 20), x, y, width, height, get(5, 0), get(6, 0)])
}
pub(crate) fn image_input(
    frame: &PictureEffectFrame,
    p: [i32; 7],
    base: ImageInput,
    rng: &mut PatternRng,
) -> Result<Option<ImageInput>> {
    let Some(source) = frame.source.as_ref().filter(|s| s.size() == frame.canvas) else {
        return Ok(None);
    };
    let [w, h] = frame.canvas.map(|v| v as i32);
    let [_, x, y, width, height, curve, subframe] = p;
    let phase = crate::native_curve::value_with_random(
        frame.state.count,
        frame.state.frames,
        0,
        256,
        subframe,
        curve,
        rng,
    )?;
    let scale = |a: i32, b: i32| a.wrapping_add(b.wrapping_mul(phase) / 256);
    let left = scale(x, -x).max(0);
    let top = scale(y, -y).max(0);
    let right = scale(x + width, w - x - width).min(w);
    let bottom = scale(y + height, h - y - height).min(h);
    if left >= w || top >= h || right < 0 || bottom < 0 || right <= left || bottom <= top {
        return Ok(None);
    }
    let size = [right - left, bottom - top];
    let exact = (2..=5).find(|b| size[0] == w / b && size[1] == h / b);
    let (effect, paint, origin) = if let Some(divisor) = exact {
        use crysirror_image::box_reduce::{BoxAlpha, BoxAnchors, BoxArithmetic, BoxReduce};
        // Original2x2 MMX uses the interpolated size. Its3..5 branches use
        // the initial rectangle's dimensions even when reached mid-animation.
        let out = if divisor == 2 {
            [w / 2, h / 2]
        } else {
            [width, height]
        };
        let stride = [w / out[0], h / out[1]];
        (
            ImageEffect::BoxReduce {
                spec: BoxReduce {
                    size: out.map(|v| v as u32),
                    source: None,
                    window: stride.map(|v| v as u16),
                    anchors: BoxAnchors::Grid {
                        stride: stride.map(|v| v as u32),
                    },
                    // Original2x2 MMX unpacks the lower pair twice for its
                    // second row: TL+TR+2*BL, not a symmetric four-pixel mean.
                    arithmetic: if divisor==2{BoxArithmetic::WeightedQuad{weights:[1,1,2,0],divisor:4}}else{BoxArithmetic::Mean},
                    alpha: BoxAlpha::Opaque,
                },
            },
            [0, 0, out[0], out[1]],
            [left, top],
        )
    } else {
        use crysirror_image::rect_map::{RectMap, RectMapRegion, RectMapSample};
        (
            ImageEffect::RectMap {
                spec: RectMap {
                    regions: vec![RectMapRegion {
                        source: [0, 0, w, h],
                        destination: [left, top, size[0], size[1]],
                    }],
                    output_size: None,
                    clear: [0, 0, 0, 255],
                    sample: RectMapSample::FloorQuantized {
                        fractional_bits: [None, Some(8)],
                    },
                    copy_alpha: false,
                },
            },
            [left, top, size[0], size[1]],
            [left, top],
        )
    };
    Ok(Some(ImageInput::Composite {
        base: Box::new(base),
        layers: vec![ImageCompositeLayer {
            input: Box::new(ImageInput::Process {
                input: Box::new(ImageInput::Resource(source.clone())),
                effects: serde_json::to_vec(&[effect])?.into(),
            }),
            source: paint,
            destination: origin,
            clip: None,
            opacity: 255,
            math: ImageCompositeMath::CopyRgb,
        }],
    }))
}

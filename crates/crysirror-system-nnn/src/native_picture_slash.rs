// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! SLASH76 native source-window and aperture phase. Shared AxisBoxSample owns
//! moving sums and packed RGB normalization; the adapter never inspects pixels.
use crate::native_picture_effect::PictureEffectFrame;
use anyhow::Result;
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageInput};
use crysirror_image::{
    axis_box_sample::AxisBoxSample, byte_raster::ByteRaster, effects::ImageEffect,
    temporal_pattern::PatternRng,
};
pub(crate) fn parameters(values: &[i32]) -> [i32; 6] {
    // Original unconditionally reads six caller words before defaults. Read
    // only present arguments and apply the same defaults, avoiding OOB reads.
    let mut p = std::array::from_fn(|i| {
        values.get(i).copied().unwrap_or(match i {
            0 => 20,
            4 => 32,
            5 => values.get(4).copied().unwrap_or(32).saturating_add(4),
            _ => 0,
        })
    });
    // para5's default uses the ORIGINAL para4 before it is clamped below.
    p[4] = p[4].clamp(1, 255);
    p[5] = p[5].max(p[4] + 4);
    p
}
pub(crate) fn validate(p: &[i32; 6], frames: i32) -> Result<()> {
    crate::native_curve::validate(frames, p[3], p[2])
}
pub(crate) fn image_input(
    frame: &PictureEffectFrame,
    p: &[i32; 6],
    base: ImageInput,
    rng: &mut PatternRng,
) -> Result<Option<ImageInput>> {
    let percent = crate::native_curve::value_with_random(
        frame.state.count,
        frame.state.frames,
        0,
        100,
        p[3],
        p[2],
        rng,
    )?
    .clamp(0, 100);
    if !frame.geometry_assigned {
        return Ok(None);
    }
    let Some(picture) = frame.bank.picture(frame.slot).map(|p| &p.picture) else {
        return Ok(None);
    };
    let size = picture.raw.size();
    if size[0] < frame.canvas[0] || size[1] < frame.canvas[1] {
        return Ok(None);
    }
    let point = [
        frame.rectangles.destination[0],
        frame.rectangles.destination[1],
    ];
    let source = point.map(|n| -i64::from(n));
    if source[0] < 0
        || source[1] < 0
        || (0..2).any(|axis| source[axis] + i64::from(frame.canvas[axis]) > i64::from(size[axis]))
    {
        return Ok(None);
    }
    let window = (percent.wrapping_mul(p[4]) / 100).clamp(0, 256);
    if window <= 1 {
        // Endpoint Blt(FALSE) follows its ordinary CharaClip, unlike the raw
        // pointer blur branches which deliberately ignore that character trim.
        let Some((rect, at)) = crysirror_image::coverage_bounds::intersect_source_window(
            [0, 0, size[0] as i32, size[1] as i32],
            point,
            picture.source_clip,
        )?
        else {
            return Ok(None);
        };
        return Ok(Some(ImageInput::Composite {
            base: Box::new(base),
            layers: vec![ImageCompositeLayer {
                input: Box::new(ImageInput::Resource(picture.opaque.clone())),
                source: rect,
                destination: at,
                clip: None,
                opacity: 255,
                math: ImageCompositeMath::CopyRgb,
            }],
        }));
    }
    let vertical = matches!(p[1], 0 | 1);
    let extent = if matches!(p[1], 2 | 3) {
        frame.canvas[0]
    } else {
        frame.canvas[1]
    } as i32;
    let max_space = extent.wrapping_sub(p[5]).max(0);
    let space = (percent.wrapping_mul(max_space) / 100).clamp(0, max_space);
    let phase = match p[1] {
        0 | 2 => [space, extent - window],
        1 | 3 => [0, extent - space - window],
        _ => [0, 0],
    };
    // Other direction IDs enter the original horizontal body without ever
    // initializing its X table. Define that undefined phase as stationary0.
    let spec = AxisBoxSample {
        source: [
            source[0] as i32 + if vertical { 0 } else { phase[0] },
            source[1] as i32 + if vertical { phase[0] } else { 0 },
        ],
        vertical,
        window: window as u16,
        phase,
        paired_columns: vertical,
    };
    spec.validate()?;
    Ok(Some(ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(base),
            source: Box::new(ImageInput::Resource(picture.opaque.clone())),
        }),
        effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
            spec: ByteRaster::AxisBoxSample { spec },
        }])?
        .into(),
    }))
}
// CountIsMaxTRUE changes no state. InitParameter inherited empty; Calcu, Init,
// BeginPrint and End own no semantic work. CheckNeedPicPrintFALSE.

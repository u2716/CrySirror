// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! WATERFALL91 ballistic row selection; shared scanlines own masked pixels.
use crate::native_picture_effect::PictureEffectFrame;
use anyhow::Result;
use crysirror_core::image_processing::ImageInput;
use crysirror_image::{
    byte_raster::{ByteRaster, ByteRasterMath},
    effects::ImageEffect,
    fixed_scanlines::{FixedSampleSpan, FixedScanlines, ScanlineMath},
};
pub(crate) fn parameters(values: &[i32]) -> [i32; 8] {
    let defaults = [20, 0, 0, 24, -1, 100, 20, 2];
    std::array::from_fn(|i| values.get(i).copied().unwrap_or(defaults[i]))
}
pub(crate) fn slot(p: &[i32; 8], own: i32, limit: i32) -> i32 {
    (if p[4] == -1 { own } else { p[4] }).clamp(0, limit - 1)
}
pub(crate) fn calcu(p: &[i32; 8], count: &mut i32, height: u32) {
    if height == 0 || p[5] < 1 {
        return;
    }
    let k = count.wrapping_mul(p[5]) / 100;
    if k >= height as i32 {
        *count = (k % height as i32).wrapping_mul(100) / p[5];
    }
}
pub(crate) fn image_input(
    frame: &PictureEffectFrame,
    p: &[i32; 8],
    base: ImageInput,
) -> Result<Option<ImageInput>> {
    let Some(bank) = frame.bank.picture(slot(p, frame.slot, frame.bank_limit)) else {
        return Ok(None);
    };
    let picture = &bank.picture;
    // The only active software32 raster branch is md2 with a real mask;
    // md2 falls to md1 when absent, and all md!=2 return without drawing.
    if p[7] != 2 || !picture.has_mask {
        return Ok(None);
    }
    let source = picture.raw.clone();
    let size = source.size();
    let [x, y] = [p[1], p[2]];
    if x >= frame.canvas[0] as i32 || y >= frame.canvas[1] as i32 {
        return Ok(None);
    }
    let width = (size[0] as i64).min(i64::from(frame.canvas[0]) - i64::from(x));
    let height = i64::from(p[3]).min(i64::from(frame.canvas[1]) - i64::from(y));
    if width < 1 || height < 1 {
        return Ok(None);
    }
    // Original negative destinations could overrun its temporary screen-height
    // table. Bounded rows plus ordinary shared destination clipping replace
    // those undefined accesses while preserving each surviving row's phase.
    let height = height.min(i64::from(frame.canvas[1])) as i32;
    let v0 = f64::from(p[5]) / 100.0;
    let grav = f64::from(p[6]) / 100.0;
    let dc = f64::from(frame.state.count) * v0;
    let mut spans = Vec::with_capacity(height as usize);
    for row in 0..height {
        let vv = (v0 * v0 + 2.0 * grav * f64::from(row)).sqrt();
        // Negative square roots had undefined integer conversion downstream.
        if !vv.is_finite(){continue;}
        // Division by0/NaN followed by integer conversion has no source
        // defined result. Its finite zero-gravity limit is uniform velocity.
        let time = if grav == 0.0 {
            if v0 == 0.0 { 0.0 } else { f64::from(row) / v0 }
        } else {
            (vv - v0) / grav
        };
        let coverage = if vv == 0.0 {
            256
        } else {
            ((v0 / vv) * 256.0 + 0.5) as i32
        }
        .clamp(0, 256);
        let source_y = ((time * v0 - dc + 0.5) as i32).rem_euclid(size[1] as i32);
        spans.push(FixedSampleSpan {
            destination: [x, y.wrapping_add(row), width as i32],
            source_q16: [0, i64::from(source_y) << 16],
            source_step_q16: [65536, 0],
            math: ScanlineMath::Stored {
                math: ByteRasterMath::MaskPiecewiseWrapping256 {
                    weight: coverage as u16,
                },
            },
        });
    }
    let spec = FixedScanlines {
        spans,
        corner_rgb_key: false,
        source_color: None,
    };
    spec.validate()?;
    Ok(Some(ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(base),
            source: Box::new(ImageInput::Resource(source)),
        }),
        effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
            spec: ByteRaster::FixedScanlines { spec },
        }])?
        .into(),
    }))
}
// CountIsMax FALSE, empty InitParameter/Init/Calcu helpers except the above
// native count reduction. CheckAllPrint TRUE; CheckNeedPicPrint FALSE.

// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! RAIN4 102 original four-depth ordering, per-slot distance and tiled stretch
//! policy. Shared RepeatScaleSample performs every repeated scaled pixel read.
use crate::native_picture_effect::PictureEffectFrame;
use anyhow::Result;
use crysirror_core::image_processing::ImageInput;
use crysirror_image::{
    byte_raster::ByteRaster, effects::ImageEffect, fixed_scanlines::ScanlineMath,
    repeat_scale_sample::RepeatScaleSample,
};
pub(crate) fn parameters(values: &[i32], canvas: [u32; 2]) -> [i32; 9] {
    let defaults = [
        20,
        2,
        500,
        20,
        -20,
        canvas[0] as i32 / 2,
        canvas[1] as i32 / 2,
        0,
        60,
    ];
    let mut p = std::array::from_fn(|i| values.get(i).copied().unwrap_or(defaults[i]));
    p[2] = p[2].max(10);
    p[3] = p[3].max(10);
    if p[3] >= p[2] {
        p[3] = p[2] - 1;
    }
    p[5] = p[5].clamp(0, canvas[0] as i32 - 1);
    p[6] = p[6].clamp(0, canvas[1] as i32 - 1);
    // Source Print has a four-word local dist array and no count guard. Bound
    // that formerly overflowing request; negative/zero counts already draw0.
    p[1] = p[1].min(4);
    p
}
pub(crate) fn calcu(p: &[i32; 9], distance: &mut i32) {
    *distance = distance.wrapping_add(p[4]);
    if p[4] < 0 {
        if *distance < p[3] {
            *distance = p[2];
        }
    } else if *distance > p[2] {
        *distance = p[3];
    }
}
pub(crate) fn image_input(
    frame: &PictureEffectFrame,
    p: &[i32; 9],
    distance: i32,
    mut base: ImageInput,
) -> Result<Option<ImageInput>> {
    let Some(picture) = frame.bank.picture(frame.slot).map(|p| &p.picture) else {
        return Ok(None);
    };
    let count = p[1].max(0).min(4) as usize;
    if count == 0 {
        return Ok(None);
    }
    let difference = p[2].wrapping_sub(p[3]);
    let difference = if difference == 0 { 1 } else { difference };
    let mut depths = [0; 4];
    let mut current = 0;
    let mut max = 0;
    let mut min = 10000;
    for (i, d) in depths[..count].iter_mut().enumerate() {
        *d = distance.wrapping_add(difference.wrapping_mul(i as i32) / count as i32);
        if *d > p[2] {
            *d = d.wrapping_sub(p[3]) % difference + p[3];
        }
        if p[4] < 0 {
            if *d > max {
                max = *d;
                current = i;
            }
        } else if *d < min {
            min = *d;
            current = i;
        }
    }
    let size = picture.raw.size();
    for _ in 0..count {
        let d = if depths[current] == 0 {
            1
        } else {
            depths[current]
        };
        let mut percent = 100;
        if p[8] > 0 {
            let low = d.wrapping_sub(p[3]);
            let high = p[2].wrapping_sub(d);
            if low < p[8] {
                percent = low.wrapping_mul(100i32.wrapping_sub(p[7])) / p[8] + p[7];
            } else if high < p[8] {
                percent = high.wrapping_mul(100i32.wrapping_sub(p[7])) / p[8] + p[7];
            }
        }
        percent = percent.clamp(1, 100);
        let period = size.map(|n| (n as i32).wrapping_mul(100) / d);
        if period.iter().any(|n| *n <= 0) {
            continue;
        }
        let weight = (percent * 256 / 100) as u16;
        let math = if picture.has_mask {
            ScanlineMath::CoverageSeparate {
                weight: u32::from(weight),
                opaque_copy: percent == 100,
            }
        } else if percent == 100 {
            ScanlineMath::ConstantSummed { weights: [256, 0] }
        } else {
            ScanlineMath::ConstantSeparate {
                weights: [weight, weight ^ 255],
            }
        };
        let spec = RepeatScaleSample {
            period: period.map(|n| n as u32),
            origin: [p[5] - period[0] / 2, p[6] - period[1] / 2],
            source: [0, 0, size[0], size[1]],
            math,
            corner_rgb_key: !picture.has_mask,
        };
        spec.validate()?;
        base = ImageInput::Process {
            input: Box::new(ImageInput::Raster {
                destination: Box::new(base),
                source: Box::new(ImageInput::Resource(if picture.has_mask {
                    picture.raw.clone()
                } else {
                    picture.opaque.clone()
                })),
            }),
            effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
                spec: ByteRaster::RepeatScaleSample { spec },
            }])?
            .into(),
        };
        // The original advances its circular depth index ONLY after a valid
        // positive-sized Stretch pass, not after a skipped depth.
        current = if p[4] < 0 {
            (current + count - 1) % count
        } else {
            (current + 1) % count
        };
    }
    Ok(Some(base))
}
// SetParam and raw InitParameter reset distance0 and count0. The32-slot class
// array is indexed by slot, not reset when another slot installs this effect.
// CountIsMaxTRUE changes nothing; CheckNeedPicPrintFALSE; CheckAllPrintTRUE.

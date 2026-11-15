// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! TURN64 software32 parameters, curve order, f32 spline and packed16.16 phase.
//! Shared wrapping scanlines own sampling, raw-key comparisons and all pixels.
use crate::native_picture_effect::PictureEffectFrame;
use anyhow::Result;
use crysirror_core::image_processing::ImageInput;
use crysirror_image::{
    byte_raster::{ByteRaster, ByteRasterMath},
    effects::ImageEffect,
    fixed_scanlines::{FixedSampleSpan, FixedScanlines, ScanlineMath},
    temporal_pattern::PatternRng,
};
pub(crate) fn parameters(values: &[i32]) -> [i32; 36] {
    let mut p = [0; 36];
    for i in 0..36 {
        p[i] = values.get(i).copied().unwrap_or(match i {
            0 => 20,
            2 => p[1],
            5 | 8 => 100,
            6 => p[5],
            9 => p[8],
            18 => p[17],
            19 => p[18],
            20 => p[17],
            21 => p[18],
            22 => p[19],
            28 => 200,
            29 => -100,
            30 => 300,
            31 => -200,
            32 => 400,
            33 => 100,
            _ => 0,
        });
    }
    // Source tests the wrong argument counts for5/6/7: supplied5 or6 can be
    // overwritten, and omitted7 stays uninitialized when length6/7. Define
    // only that missing word0 and preserve the actual overwrite conditions.
    if values.len() < 7 {
        p[5] = 100;
    }
    if values.len() < 8 {
        p[6] = p[5];
    }
    if values.len() < 6 {
        p[7] = 0;
    }
    p
}
pub(crate) fn validate(p: &[i32; 36], frames: i32) -> Result<()> {
    for (kind, sub) in [(10, 11), (12, 13), (14, 15), (23, 24)] {
        crate::native_curve::validate(frames, p[sub], p[kind])?;
    }
    if p[25] == 2 || p[25] == 3 {
        crate::native_curve::validate(frames, p[35], p[34])?;
    }
    Ok(())
}
fn fixed(v: f64) -> i32 {
    let mut high = v as i32;
    if v < 0.0 {
        high = high.wrapping_sub(1);
    }
    let low = ((v - f64::from(high)) * 65536.0) as i32;
    high.wrapping_shl(16) | (low & 65535)
}
fn spline(p: &[i32; 36], s: i32) -> [i32; 2] {
    let t = s as f32 * 0.0001f32;
    let r = 1.0f32 - t;
    let result = if p[25] == 3 {
        let a = t * t * t;
        let b = t * t * r;
        let c = t * r * r;
        let d = r * r * r;
        std::array::from_fn(|axis| {
            d * p[26 + axis] as f32
                + 3.0 * c * p[28 + axis] as f32
                + 3.0 * b * p[30 + axis] as f32
                + a * p[32 + axis] as f32
        })
    } else {
        let a = t * t;
        let b = t * r;
        let c = r * r;
        std::array::from_fn(|axis| {
            c * p[26 + axis] as f32 + 2.0 * b * p[28 + axis] as f32 + a * p[30 + axis] as f32
        })
    };
    result.map(|v: f32| (v + 0.5f32) as i32)
}
pub(crate) fn image_input(
    frame: &PictureEffectFrame,
    p: &[i32; 36],
    base: ImageInput,
    rng: &mut PatternRng,
) -> Result<Option<ImageInput>> {
    validate(p, frame.state.frames)?;
    let curve = |a: i32, b: i32, kind: usize, sub: usize, rng: &mut PatternRng| {
        crate::native_curve::value_with_random(
            frame.state.count,
            frame.state.frames,
            a,
            b,
            p[sub],
            p[kind],
            rng,
        )
    };
    let percent = curve(p[9], p[8], 14, 15, rng)?;
    let weight = 256i32.wrapping_mul(percent) / 100;
    let scale = (curve(p[6].wrapping_mul(10), p[5].wrapping_mul(10), 12, 13, rng)? / 10).max(1);
    let color = [
        curve(p[20], p[17], 23, 24, rng)?,
        curve(p[21], p[18], 23, 24, rng)?,
        curve(p[22], p[19], 23, 24, rng)?,
    ];
    if !frame.geometry_assigned {
        return Ok(None);
    }
    let Some(picture) = frame.bank.picture(frame.slot).map(|b| &b.picture) else {
        return Ok(None);
    };
    let size = picture.raw.size();
    let mut angle = curve(p[2].wrapping_mul(10), p[1].wrapping_mul(10), 10, 11, rng)?;
    if angle < 0 {
        angle = angle.wrapping_neg();
        if angle >= 3600 {
            angle %= 3600;
        }
        angle = angle.wrapping_neg().wrapping_add(3600);
    }
    if angle >= 3600 {
        angle %= 3600;
    }
    let radians = f64::from(angle) * (3.14159 * 2.0 / 3600.0);
    let mut point = [
        frame.rectangles.destination[0],
        frame.rectangles.destination[1],
    ];
    if p[25] == 2 || p[25] == 3 {
        let phase = curve(0, 10000, 34, 35, rng)?;
        let delta = spline(p, phase);
        point[0] = point[0].wrapping_add(delta[0]);
        point[1] = point[1].wrapping_add(delta[1]);
    }
    let mut cosine = radians.cos();
    let mut sine = radians.sin();
    cosine /= f64::from(scale);
    cosine *= 100.0;
    sine /= f64::from(scale);
    sine *= 100.0;
    let center = [
        (size[0] as i32 / 2).wrapping_add(p[3]),
        (size[1] as i32 / 2).wrapping_add(p[4]),
    ];
    let screen_center = [
        f64::from(frame.canvas[0] / 2) + f64::from(point[0]),
        f64::from(frame.canvas[1] / 2) + f64::from(point[1]),
    ];
    let transform = |x: f64, y: f64| {
        [
            cosine * x - sine * y + f64::from(center[0]) + 0.5,
            sine * x + cosine * y + f64::from(center[1]) + 0.5,
        ]
    };
    let a = transform(-screen_center[0], -screen_center[1]);
    let b = transform(
        f64::from(frame.canvas[0]) - screen_center[0],
        -screen_center[1],
    );
    let c = transform(
        -screen_center[0],
        f64::from(frame.canvas[1]) - screen_center[1],
    );
    let horizontal =
        std::array::from_fn::<_, 2, _>(|i| fixed((b[i] - a[i]) / f64::from(frame.canvas[0])));
    let vertical =
        std::array::from_fn::<_, 2, _>(|i| fixed((c[i] - a[i]) / f64::from(frame.canvas[1])));
    let origin = a.map(fixed);
    // Source's negative fixed helper decrements even an integral value (e.g.
    //-1.0), then discards its65536 fraction with an explicit low16 mask.
    let mode = if p[7] == 2 && (!picture.has_mask || picture.mask_storage.is_none()) {
        1
    } else {
        p[7]
    };
    let math = if mode == 0 || mode == 1 {
        ScanlineMath::Stored {
            math: ByteRasterMath::CopyRgb,
        }
    } else {
        ScanlineMath::CoverageWordSeparate {
            weight: weight as u16,
            complement: 256,
            opaque_copy: true,
        }
    };
    let source = if mode == 0 || mode == 1 {
        picture.raw.clone()
    } else {
        let Some(mask) = &picture.mask_storage else {
            return Ok(None);
        };
        mask.view.clone()
    };
    let source_color = if p[16] != 0 {
        let add = color.map(|v| v.max(0));
        let sub = color.map(|v| v.wrapping_neg().max(0));
        let pack = |rgb: [i32; 3]| {
            let p = rgb[0].wrapping_shl(16) | rgb[1].wrapping_shl(8) | rgb[2];
            [(p >> 16) as u8, (p >> 8) as u8, p as u8]
        };
        Some((pack(add), pack(sub)))
    } else {
        None
    };
    let spans = (0..frame.canvas[1] as i32)
        .map(|row| FixedSampleSpan {
            destination: [0, row, frame.canvas[0] as i32],
            source_q16: std::array::from_fn(|i| {
                i64::from(origin[i].wrapping_add(vertical[i].wrapping_mul(row)))
            }),
            source_step_q16: horizontal.map(i64::from),
            math,
        })
        .collect();
    let operation = ByteRaster::FixedScanlinesWrapping32 {
        spec: FixedScanlines {
            spans,
            corner_rgb_key: mode == 1,
            source_color,
        },
    };
    operation.validate()?;
    Ok(Some(ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(base),
            source: Box::new(ImageInput::Resource(source)),
        }),
        effects: serde_json::to_vec(&[ImageEffect::ByteRaster { spec: operation }])?.into(),
    }))
}
// CountIsMaxFALSE; no InitParameter/Calcu/Init/BeginPrint/End work, no ordinary
// picture pass. Raw saves preserve36words plus independent counters/rectangles.
// The D3D branch and commented old ADC loop are not the software32 profile.

// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! KONOHA104 original polar atlas geometry and count policy. Sprite pixels,
//! trimming and ordered copies remain common engine composition operations.
use crate::native_picture_effect::PictureEffectFrame;
use anyhow::{Result, ensure};
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageInput};

pub(crate) fn parameters(values: &[i32], canvas: [u32; 2]) -> [i32; 16] {
    let mut p = std::array::from_fn(|i| {
        values.get(i).copied().unwrap_or(match i {
            0 => 20,
            1 | 2 => 32,
            3 | 4 => 1,
            5 => 100,
            6 => 3,
            7 => 7,
            8 => 15,
            10 => 1000,
            12 => canvas[0] as i32 / 2,
            13 => canvas[1] as i32 / 2,
            _ => 0,
        })
    });
    // Setter always overwrites para0, even when the script supplies it. The
    // unread omitted14/15 copied uninitialized stack; define deterministic0.
    p[0] = 999999;
    p
}
pub(crate) fn validate(p: &[i32; 16]) -> Result<()> {
    ensure!(
        p[1] >= 0 && p[2] >= 0 && p[1] <= 16384 && p[2] <= 16384,
        "NNN KONOHA cell size"
    );
    ensure!(p[5] <= 4096, "NNN KONOHA sprite work budget");
    Ok(())
}
pub(crate) fn calcu(p: &[i32; 16], count: &mut i32) {
    if *count > 10000 {
        *count = p[11];
    }
}
/// Trigonometry belongs to native movement policy; each emitted rectangle is
/// an ordinary scoped shared sprite operation. No source pixels are inspected.
pub(crate) fn layers(
    frame: &PictureEffectFrame,
    p: &[i32; 16],
) -> Result<Vec<ImageCompositeLayer>> {
    validate(p)?;
    let Some(bank) = frame.bank.picture(frame.slot) else {
        return Ok(vec![]);
    };
    let picture = &bank.picture;
    let source = if picture.has_mask {
        picture.raw.clone()
    } else {
        picture.keyed.clone()
    };
    let [w, h] = [p[1], p[2]];
    let count = p[5].max(0);
    // Native zero divisors and angles>360 produced invalid table reads. Define
    // bounded angular partitions instead of allowing divide-by-zero or OOB.
    let partition = p[6].max(1);
    let angles = p[4].clamp(1, 360);
    let angle_step = 360 / angles;
    let delta = 360 / partition;
    // Keep the original32-bit subtraction. abs(INT_MIN) had no defined
    // signed result; a bounded positive radius span replaces that case.
    let radius_span = p[10]
        .wrapping_sub(p[9])
        .checked_abs()
        .unwrap_or(i32::MAX)
        .max(1);
    let mut angular_partition = 0i32;
    let mut out = Vec::new();
    for i in 0..count {
        let k = frame.state.count.wrapping_add(i).wrapping_sub(p[11]);
        angular_partition = angular_partition.wrapping_add(delta) % 360;
        if k < 0 {
            continue;
        }
        let mut radius = k.wrapping_mul(p[7]) % radius_span;
        if p[10] >= p[9] {
            radius = radius.wrapping_add(p[9]);
        } else {
            radius = p[9].wrapping_sub(radius).max(0);
        }
        // A negative velocity could index before the original360-entry
        // table. Normalize to its periodic angle as the defined replacement.
        let theta = (k.wrapping_mul(p[8]) % 360)
            .wrapping_add(i.wrapping_mul(360) / count)
            .wrapping_add(angular_partition)
            .rem_euclid(360);
        let radians = f64::from(theta) / 360.0 * (3.14159 * 2.0);
        // Source uses +0.5 then a truncating cast for BOTH signs. round() or a
        // standard-PI lookup would alter negative offsets and some endpoints.
        let cosine = (radians.cos() * 256.0 + 0.5) as i32;
        let sine = (radians.sin() * 256.0 + 0.5) as i32;
        let x = p[12]
            .wrapping_add(cosine.wrapping_mul(radius) / 256)
            .wrapping_sub(w / 2);
        let y = p[13]
            .wrapping_add(sine.wrapping_mul(radius) / 256)
            .wrapping_sub(h / 2);
        if x < -w
            || y < -h
            || i64::from(x) >= i64::from(frame.canvas[0]) + i64::from(w)
            || i64::from(y) >= i64::from(frame.canvas[1]) + i64::from(h)
        {
            continue;
        }
        let sy = if p[3] > 1 {
            h.wrapping_mul(i % p[3])
        } else {
            0
        };
        let sx = if p[4] > 1 {
            w.wrapping_mul(((theta + angle_step / 2) % 360 / angle_step) % angles)
        } else {
            0
        };
        let Some((region, destination)) =
            crysirror_image::coverage_bounds::intersect_source_window(
                [sx, sy, w, h],
                [x, y],
                picture.source_clip,
            )?
        else {
            continue;
        };
        out.push(ImageCompositeLayer {
            input: Box::new(ImageInput::Resource(source.clone())),
            source: region,
            destination,
            clip: None,
            opacity: 255,
            math: ImageCompositeMath::MaskSeparate256,
        });
    }
    Ok(out)
}

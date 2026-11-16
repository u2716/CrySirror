// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! STAFFROLL80 scalar fade-table, page-slot and original raw-mask policy.
//! Full620-line class/header reviewed. Page reads must complete sequentially:
//! recycling a picture slot can change the following page's base name/height.
use crate::image_bridge::NativePictureViews;
use anyhow::{Result, ensure};
use crysirror_core::image_processing::ImageInput;
use crysirror_image::{
    byte_raster::ByteRaster,
    effects::ImageEffect,
    fixed_scanlines::{FixedSampleSpan, FixedScanlines, ScanlineMath},
};

pub(crate) fn parameters(values: &[i32], canvas: [u32; 2]) -> [i32; 16] {
    let defaults = [
        20, 1, 0, 50, 100, 100, 50, 0, 100, 200, 280, 380, 1, 0, 0, 0,
    ];
    //14/15 were omitted uninitialized stack words, never read by this class.
    let mut p = std::array::from_fn(|i| values.get(i).copied().unwrap_or(defaults[i]));
    p[8] = p[8].max(0);
    for i in 9..12 {
        p[i] = p[i].max(p[i - 1]);
    }
    for v in &mut p[8..12] {
        *v = (*v).min(canvas[1] as i32);
    }
    p[12] = p[12].max(1);
    if p[12] == 2 {
        p[13] = 1;
    } else if p[12] > 2 {
        p[13] = p[13].max(2);
    }
    p
}
pub(crate) fn validate(p: &[i32; 16]) -> Result<()> {
    ensure!(
        (1..=4096).contains(&p[12]) && p[8..12].iter().all(|v| (0..=16384).contains(v)),
        "NNN staff page/table work budget"
    );
    Ok(())
}
pub(crate) fn row_weights(p: &[i32; 16], height: u32) -> Result<Vec<u16>> {
    ensure!((1..=16384).contains(&height), "NNN staff fade-table height");
    let mut table = vec![0; height as usize];
    let ys = [0, p[8], p[9], p[10], p[11], height as i32];
    for segment in 0..5 {
        let a = ys[segment];
        let b = ys[segment + 1];
        let divisor = b.wrapping_sub(a);
        if divisor < 1 {
            continue;
        }
        for y in a.max(0)..b.min(height as i32) {
            let percent = p[2 + segment].wrapping_add(
                y.wrapping_sub(a)
                    .wrapping_mul(p[3 + segment].wrapping_sub(p[2 + segment]))
                    / divisor,
            );
            table[y as usize] = (percent.wrapping_mul(256) / 100).clamp(0, 256) as u16;
        }
    }
    Ok(table)
}
pub(crate) fn page_slot(
    p: &[i32; 16],
    own: i32,
    delta: i32,
    bank_limit: i32,
) -> Result<Option<i32>> {
    ensure!(
        p[12] >= 1 && p[12] <= 4096 && delta >= 0 && delta < p[12],
        "NNN staff page-work budget"
    );
    ensure!(bank_limit > 0, "NNN staff physical picture bank");
    if delta == 0 {
        return Ok(Some(own.clamp(0, bank_limit - 1)));
    }
    let slot = own.wrapping_add(1).wrapping_add((delta - 1) % p[13].max(1));
    // The source literal is >16, NOT >=16. GetPicture then applies the
    // selected profile's physical bank clamp (Legacy16 versus Source32).
    Ok((0..=16)
        .contains(&slot)
        .then(|| slot.clamp(0, bank_limit - 1)))
}
pub(crate) fn page_input(
    p: &[i32; 16],
    point: [i32; 2],
    base_height: u32,
    delta: i32,
    picture: &NativePictureViews,
    canvas: [u32; 2],
    base: ImageInput,
) -> Result<Option<ImageInput>> {
    let weights = row_weights(p, canvas[1])?;
    let size = picture.raw.size();
    let at = [
        point[0],
        point[1].wrapping_add((base_height as i32).wrapping_mul(delta)),
    ];
    // StaffRollSub clips to the screen directly and ignores CharaClip.
    let Some((target, source)) = crysirror_image::coverage_bounds::intersect_source_window(
        [at[0], at[1], size[0] as i32, size[1] as i32],
        [0, 0],
        [0, 0, canvas[0] as i32, canvas[1] as i32],
    )?
    else {
        return Ok(None);
    };
    let source_image = if p[1] == 2 {
        // Source checks a pointer only after adding offsets to it. Check the
        // original allocation first; nonzero offsets must not revive NULL.
        let Some(storage) = &picture.mask_storage else {
            return Ok(None);
        };
        storage.view.clone()
    } else {
        picture.opaque.clone()
    };
    let mut spans = Vec::with_capacity(target[3] as usize);
    for row in 0..target[3] {
        let y = target[1] + row;
        let weight = weights[y as usize];
        if weight == 0 {
            continue;
        }
        let math = if p[1] == 2 {
            ScanlineMath::CoverageWordSeparate {
                weight,
                complement: 256,
                opaque_copy: false,
            }
        } else if weight == 256 {
            ScanlineMath::ConstantSummed { weights: [256, 0] }
        } else {
            ScanlineMath::ConstantSeparate {
                weights: [weight, 256 - weight],
            }
        };
        spans.push(FixedSampleSpan {
            destination: [target[0], y, target[2]],
            source_q16: [
                i64::from(source[0]) * 65536,
                i64::from(source[1] + row) * 65536,
            ],
            source_step_q16: [65536, 0],
            math,
        });
    }
    let spec = FixedScanlines {
        spans,
        corner_rgb_key: p[1] != 2,
        source_color: None,
    };
    spec.validate()?;
    Ok(Some(ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(base),
            source: Box::new(ImageInput::Resource(source_image)),
        }),
        effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
            spec: ByteRaster::FixedScanlines { spec },
        }])?
        .into(),
    }))
}
// InitParameter copies16words and replays SetParam (count/max reset). Calcu,
// BeginPrint/Init/End are empty; CountIsMaxFALSE, CheckNeedPicPrintFALSE.

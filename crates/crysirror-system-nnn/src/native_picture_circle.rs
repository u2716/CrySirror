// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CIRCLE28 scalar geometry and CountIsMax policy from the complete original
//! class/header. Shared FixedScanlines paints bounded spans; no private pixels.
use anyhow::{Result, ensure};
use crysirror_core::image_processing::ImageInput;
use crysirror_image::{
    byte_raster::ByteRaster,
    effects::ImageEffect,
    fixed_scanlines::{FixedSampleSpan, FixedScanlines, ScanlineMath},
};

pub(crate) fn parameters(values: &[i32], canvas: [u32; 2]) -> [i32; 8] {
    // The original expression is width² + height²/4 (not half-diagonal²).
    let r = (f64::from(canvas[0]).powi(2) + f64::from(canvas[1]).powi(2) / 4.).sqrt() as i32;
    let mut p = [0; 8];
    for i in 0..8 {
        p[i] = values.get(i).copied().unwrap_or(match i {
            0 => 20,
            1 => canvas[0] as i32 / 2,
            2 => canvas[1] as i32 / 2,
            3 => 60,
            4 => r,
            5 => 0,
            _ => p[i - 1],
        });
    }
    p
}
pub(crate) enum Terminal {
    Fill([i32; 8]),
    Nop { active: bool },
}
pub(crate) fn terminal(p: &[i32; 8], canvas: [u32; 2], assigned: bool) -> Option<Terminal> {
    let r =
        ((f64::from(canvas[0]).powi(2) + f64::from(canvas[1]).powi(2) / 4.).sqrt() + 0.5) as i32;
    if p[3] <= 1 {
        let mut words=*p;
        words[..3].copy_from_slice(&p[5..8]);
        // Source CountIsMax writes0..2, WITHOUT calling Fill.SetParam. Fill's
        // active Print reads1..3: the resulting RGB is G/B/targetRadius. Keep
        // this mismatch rather than shifting the copy into the expected ABI.
        Some(Terminal::Fill(words))
    } else if p[3] >= r {
        Some(Terminal::Nop { active: assigned })
    } else {
        None
    }
}
pub(crate) fn image_input(
    p: &[i32; 8],
    count: i32,
    frames: i32,
    canvas: [u32; 2],
    mut base: ImageInput,
) -> Result<ImageInput> {
    ensure!(
        canvas.iter().all(|n| (1..=16384).contains(n)),
        "NNN CIRCLE canvas"
    );
    let (divisor, c) = if frames < 1 { (1, 1) } else { (frames, count) };
    let radius = p[4]
        .wrapping_add(
            p[3].wrapping_sub(p[4])
                .wrapping_mul(c)
                .wrapping_add(divisor / 2)
                / divisor,
        )
        .max(1);
    ensure!(
        radius <= 1_000_000 && p[1].unsigned_abs() <= 1_000_000 && p[2].unsigned_abs() <= 1_000_000,
        "NNN CIRCLE geometry budget"
    );
    let packed = p[5].wrapping_shl(16) | p[6].wrapping_shl(8) | p[7];
    let color = [(packed >> 16) as u8, (packed >> 8) as u8, packed as u8, 255];
    let [cx, cy] = [i64::from(p[1]), i64::from(p[2])];
    let r = i64::from(radius);
    let width = canvas[0] as i64;
    let mut spans = Vec::with_capacity(canvas[1] as usize * 2);
    for y in 0..i64::from(canvas[1]) {
        let mut push = |left: i64, right: i64| {
            let left = left.max(0);
            let right = right.min(width);
            if right > left {
                spans.push(FixedSampleSpan {
                    destination: [left as i32, y as i32, (right - left) as i32],
                    source_q16: [0, 0],
                    source_step_q16: [0, 0],
                    math: ScanlineMath::ConstantSummed { weights: [256, 0] },
                });
            }
        };
        // The top fill includes centerY-r; the lower includes centerY+r.
        // Original center-loop redraws those rows in the same solid colour.
        if y <= cy - r || y >= cy + r {
            push(0, width);
            continue;
        }
        let dy = y - cy;
        // Squaring signed native ints could overflow. Use bounded wide squares
        // for that undefined case while retaining sqrt+.5/trunc row endpoints.
        let half = ((r * r - dy * dy).max(0) as f64).sqrt();
        let half = (half + 0.5) as i64;
        let left = cx - half;
        if left >= 0 {
            push(0, left + 1);
        }
        let right = cx + half;
        if right < width {
            push(right, width);
        }
    }
    // Two batches cover the shared span-count ceiling even on a16384-row
    // canvas. Each reuses the same one-pixel Solid and retains alpha.
    for batch in spans.chunks(16384) {
        let spec = FixedScanlines {
            spans: batch.to_vec(),
            corner_rgb_key: false,
            source_color: None,
        };
        spec.validate()?;
        base = ImageInput::Process {
            input: Box::new(ImageInput::Raster {
                destination: Box::new(base),
                source: Box::new(ImageInput::Solid {
                    size: [1, 1],
                    color,
                }),
            }),
            effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
                spec: ByteRaster::FixedScanlines { spec },
            }])?
            .into(),
        };
    }
    Ok(base)
}
// BeginPrint/Init/Calcu/End have no active work. Inherited CheckNeedPicPrintTRUE
// and GetLayerOrPicType1 require the ordinary picture pass BEFORE this aperture.

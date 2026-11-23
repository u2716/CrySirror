// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! KAGEROU22 software32 heat-wave phase. Common immutable scanlines perform
//! the MMX two-product blend; no framebuffer or private pixel loop lives here.
use anyhow::{Result, ensure};
use crysirror_core::image_processing::ImageInput;
use crysirror_image::{
    byte_raster::ByteRaster,
    effects::ImageEffect,
    fixed_scanlines::{FixedSampleSpan, FixedScanlines, ScanlineMath},
};
pub(crate) fn parameters(values: &[i32], canvas: [u32; 2]) -> [i32; 17] {
    // Source's omitted frame assignment mistakenly writes para1, then resets
    // it to0, leaving para0 undefined. Use its intended9999; unused para16=0.
    let defaults = [
        9999,
        0,
        canvas[1] as i32,
        0,
        100,
        100,
        100,
        9,
        7,
        11,
        4,
        17,
        -23,
        6,
        2,
        -5,
        0,
    ];
    let mut p = std::array::from_fn(|i| values.get(i).copied().unwrap_or(defaults[i]));
    p[1] = p[1].clamp(0, canvas[1] as i32 - 1);
    p[2] = p[2].clamp(0, canvas[1] as i32);
    p[2] = p[2].min(canvas[1] as i32 - p[1]);
    p
}
pub(crate) fn validate(p: &[i32; 17], canvas: [u32; 2]) -> Result<()> {
    ensure!(
        canvas.iter().all(|n| (1..=16384).contains(n)),
        "NNN KAGEROU canvas"
    );
    ensure!(
        p[1] >= 0 && p[1] < (canvas[1] as i32) && p[2] >= 0 && p[2] <= canvas[1] as i32 - p[1],
        "NNN KAGEROU row interval"
    );
    Ok(())
}
pub(crate) fn calcu(p: &[i32; 17], count: &mut i32) {
    if *count >= p[3] {
        *count = count.wrapping_sub(p[3]) % 9000;
        *count = count.wrapping_add(p[3]);
    }
}
pub(crate) fn image_input(
    p: &[i32; 17],
    count: i32,
    canvas: [u32; 2],
    base: ImageInput,
) -> Result<ImageInput> {
    validate(p, canvas)?;
    let total = if p[3] > 0 && count < p[3] {
        f64::from(count) / f64::from(p[3])
    } else {
        1.0
    };
    let scale = [
        f64::from(p[4]) / 100.0,
        f64::from(p[5]) / 100.0,
        f64::from(p[6]) / 100.0,
    ];
    let height = [p[7], p[10], p[13]].map(|v| f64::from(v) * scale[0]);
    let width = [p[8], p[11], p[14]].map(|v| f64::from(v) * scale[1]);
    let speed = [p[9], p[12], p[15]].map(|v| f64::from(v) * scale[2]);
    let start = p[1].min(canvas[1] as i32 - 1);
    let len = p[2].max(1);
    let factor = 3.14159 * 2.0 / f64::from(canvas[1]);
    let mut spans = Vec::with_capacity(len as usize);
    for row in start..start + len {
        let phases = [
            f64::from(row) * width[0] + speed[0] * f64::from(count),
            f64::from(row) * width[1] - speed[1] * f64::from(count),
            f64::from(row) * width[2] - speed[2] * f64::from(count),
        ];
        let d = (total
            * ((phases[0] * factor).cos() * height[0]
                + (phases[1] * factor).cos() * height[1]
                + (phases[2] * factor).cos() * height[2])
            + 0.5) as i32;
        // Original zero/negative MMX loop counts access outside the row.
        // Define that invalid displacement as an empty span.
        let magnitude = i64::from(d).abs();
        if magnitude >= i64::from(canvas[0]) {
            continue;
        }
        let destination_x = if d < 0 { -d } else { 0 };
        let source_x = if d > 0 { d } else { 0 };
        spans.push(FixedSampleSpan {
            destination: [destination_x, row, canvas[0] as i32 - magnitude as i32],
            source_q16: [i64::from(source_x) << 16, i64::from(row) << 16],
            source_step_q16: [65536, 0],
            math: ScanlineMath::ConstantSeparate { weights: [96, 160] },
        });
    }
    let spec = FixedScanlines {
        spans,
        corner_rgb_key: false,
        source_color: None,
    };
    spec.validate()?;
    // Source in-place loop walks away from its samples (left for positive
    // shifts, right for negative); an immutable source has the same values.
    Ok(ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(base.clone()),
            source: Box::new(base),
        }),
        effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
            spec: ByteRaster::FixedScanlines { spec },
        }])?
        .into(),
    })
}
// CountIsMax TRUE, empty Init/BeginPrint/End/InitParameter. Inherited
// CheckNeedPicPrint TRUE runs the ordinary picture before the screen pass.
// Win64 has no raster body, and the D3D shader is a separate native profile.

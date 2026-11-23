// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native BGZOOMIN37 parameter/phase policy. Sampling and clipping belong to
//! the shared RectMap image operation, not to a native framebuffer.
use crate::native_picture_effect::PictureEffectFrame;
use anyhow::{Result, ensure};
use crysirror_core::image_processing::ImageInput;
use crysirror_image::{
    effects::ImageEffect,
    rect_map::{RectMap, RectMapRegion, RectMapSample},
    temporal_pattern::PatternRng,
};
pub(crate) fn raster_parameters(p: &[i32], canvas: [u32; 2]) -> [i32; 5] {
    let defaults = [60, 0, 64, 0, 0];
    let mut out = std::array::from_fn(|i| p.get(i).copied().unwrap_or(defaults[i]));
    let half = canvas[0] as i32 / 2;
    for i in [1, 2] {
        if out[i] >= half {
            out[i] = half - 1;
        }
    }
    out
}
pub(crate) fn raster_input(
    frame: &PictureEffectFrame,
    p: [i32; 5],
    base: ImageInput,
    random: &mut PatternRng,
) -> Result<Option<ImageInput>> {
    let amplitude = crate::native_curve::value_with_random(
        frame.state.count,
        frame.state.frames,
        p[1].wrapping_mul(100),
        p[2].wrapping_mul(100),
        p[4],
        p[3],
        random,
    )?;
    if amplitude < 100 {
        return Ok(None);
    }
    let (count, direction) = if p[2] > p[1] {
        (frame.state.frames.wrapping_sub(frame.state.count), -1)
    } else {
        (frame.state.count, 1)
    };
    let width = frame.canvas[0] as i32;
    let height = frame.canvas[1] as i32;
    let scale = 3.14159 * 2. / f64::from(height) * 3.;
    let shifts = (0..height)
        .map(|row| {
            let theta = f64::from(
                row.wrapping_mul(direction)
                    .wrapping_add(count.wrapping_mul(10)),
            ) * scale;
            let mut shift = (theta.cos() * (f64::from(amplitude) / 100.) + 0.5) as i32;
            if shift <= -width {
                shift = -(width - 1);
            }
            if shift >= width - 1 {
                shift = width - 2;
            }
            shift
        })
        .collect();
    // The positive native branch clears its source prefix before memmove;
    // preserve those initialized black pixels, rather than a pure translation.
    let spec = crysirror_image::shift_rows::ShiftRows {
        shifts,
        clear: [0, 0, 0, 255],
        erase_positive_source: true,
    };
    Ok(Some(ImageInput::Process {
        input: Box::new(base),
        effects: serde_json::to_vec(&[ImageEffect::ShiftRows { spec }])?.into(),
    }))
}
pub(crate) fn scroll_parameters(p: &[i32]) -> [i32; 16] {
    // Source's no-argument setter writes para1 twice and leaves para0
    // uninitialised. para0 is saved but countMax is always9999: define it20.
    let mut out = [0; 16];
    out[0] = p.first().copied().unwrap_or(20);
    out[1] = p.get(1).copied().unwrap_or(32);
    out[2] = p.get(2).copied().unwrap_or(0);
    out[3] = p.get(3).copied().unwrap_or(1);
    out[13] = 1;
    out
}
pub(crate) fn scroll_input(
    frame: &PictureEffectFrame,
    p: [i32; 16],
    base: ImageInput,
) -> Result<Option<ImageInput>> {
    let Some(source) = &frame.source else {
        return Ok(None);
    };
    let [w, h] = frame.canvas.map(|v| v as i32);
    let x = p[14].rem_euclid(w);
    let y = p[15].rem_euclid(h);
    let mut layers = Vec::with_capacity(4);
    for row in -1..=0 {
        for column in -1..=0 {
            layers.push(crysirror_core::image_processing::ImageCompositeLayer {
                input: Box::new(ImageInput::Resource(source.clone())),
                source: [0, 0, w, h],
                destination: [x + column * w, y + row * h],
                clip: None,
                opacity: 255,
                math: crysirror_core::image_processing::ImageCompositeMath::CopyRgb,
            });
        }
    }
    Ok(Some(ImageInput::Composite {
        base: Box::new(base),
        layers,
    }))
}

pub(crate) fn bezier_parameters(p: &[i32]) -> [i32; 11] {
    let defaults = [1, 0, 0, 200, -100, 300, -200, 400, 100, 0, 0];
    std::array::from_fn(|i| p.get(i).copied().unwrap_or(defaults[i]))
}
pub(crate) fn quadratic_parameters(p: &[i32]) -> [i32; 9] {
    let defaults = [1, 0, 0, 200, -200, 400, 100, 0, 0];
    std::array::from_fn(|i| p.get(i).copied().unwrap_or(defaults[i]))
}
pub(crate) fn quadratic_input(
    frame: &PictureEffectFrame,
    p: [i32; 9],
    base: ImageInput,
    random: &mut PatternRng,
) -> Result<Option<ImageInput>> {
    // Despite its BSPLINE2 name, the original evaluates a three-control
    // quadratic Bezier with f32 products, not a natural cubic spline.
    let phase = crate::native_curve::value_with_random(
        frame.state.count,
        frame.state.frames,
        0,
        10000,
        p[8],
        p[7],
        random,
    )?;
    let path = crysirror_scene::animation::QuadraticBezierPath {
        control: std::array::from_fn(|i| [p[1 + i * 2] as f32, p[2 + i * 2] as f32, 0.]),
    };
    let point = path.sample(phase as f32 * 0.0001f32)?;
    let quantize = |v: f32| -> Result<i32> {
        let value = v + 0.5f32;
        ensure!(
            value >= -2147483648. && value < 2147483648.,
            "NNN BSPLINE2 native coordinate"
        );
        Ok(value as i32)
    };
    let delta = [quantize(point[0])?, quantize(point[1])?];
    let Some(source) = &frame.source else {
        return Ok(None);
    };
    let size = source.size();
    Ok(Some(ImageInput::Composite {
        base: Box::new(base),
        layers: vec![crysirror_core::image_processing::ImageCompositeLayer {
            input: Box::new(ImageInput::Resource(source.clone())),
            source: [0, 0, size[0] as i32, size[1] as i32],
            destination: [
                frame.rectangles.destination[0].wrapping_add(delta[0]),
                frame.rectangles.destination[1].wrapping_add(delta[1]),
            ],
            clip: None,
            opacity: 255,
            math: if frame.motion_background {
                crysirror_core::image_processing::ImageCompositeMath::CopyRgb
            } else {
                crysirror_core::image_processing::ImageCompositeMath::MaskSeparate256
            },
        }],
    }))
}
pub(crate) fn bezier_input(
    frame: &PictureEffectFrame,
    p: [i32; 11],
    base: ImageInput,
    random: &mut PatternRng,
) -> Result<Option<ImageInput>> {
    let phase = crate::native_curve::value_with_random(
        frame.state.count,
        frame.state.frames,
        0,
        10000,
        p[10],
        p[9],
        random,
    )?;
    let path = crysirror_scene::animation::CubicBezierPath {
        control: std::array::from_fn(|i| [p[1 + i * 2] as f32, p[2 + i * 2] as f32, 0.]),
    };
    let point = path.sample(phase as f32 * 0.0001f32)?;
    let quantize = |v: f32| -> Result<i32> {
        let value = v + 0.5f32;
        ensure!(
            value >= -2147483648. && value < 2147483648.,
            "NNN BEZIERMOVE native coordinate"
        );
        Ok(value as i32)
    };
    let delta = [quantize(point[0])?, quantize(point[1])?];
    let Some(source) = &frame.source else {
        return Ok(None);
    };
    let size = source.size();
    Ok(Some(ImageInput::Composite {
        base: Box::new(base),
        layers: vec![crysirror_core::image_processing::ImageCompositeLayer {
            input: Box::new(ImageInput::Resource(source.clone())),
            source: [0, 0, size[0] as i32, size[1] as i32],
            destination: [
                frame.rectangles.destination[0].wrapping_add(delta[0]),
                frame.rectangles.destination[1].wrapping_add(delta[1]),
            ],
            clip: None,
            opacity: 255,
            math: if frame.motion_background {
                crysirror_core::image_processing::ImageCompositeMath::CopyRgb
            } else {
                crysirror_core::image_processing::ImageCompositeMath::MaskSeparate256
            },
        }],
    }))
}

pub(crate) fn jitter_parameters(p: &[i32]) -> [i32; 26] {
    let get = |i: usize, d| p.get(i).copied().unwrap_or(d);
    // effectCharaBure reads lp->para[0] before copying its new parameters.
    // Use the declared new frame parameter instead of stale/uninitialised RAM.
    let mut out = [0; 26];
    out[..8].copy_from_slice(&[
        get(0, 20),
        get(1, 4).max(0),
        get(2, 4).max(0),
        get(3, 8),
        get(4, 50),
        get(5, 50),
        get(6, 0),
        get(7, 0),
    ]);
    out[21] = 1;
    out
}
pub(crate) fn jitter_resample(p: &mut [i32; 26], random: &mut PatternRng) -> Result<()> {
    p[22] = p[24];
    p[23] = p[25];
    let mut next = [0; 3];
    for i in 0..3 {
        let variance = if i == 2 { p[5] } else { p[4] };
        let divisor = p[i + 1].wrapping_mul(variance).max(1) as u32;
        next[i] = p[i + 1]
            .wrapping_mul(100)
            .wrapping_sub((random.next_draw()? % divisor) as i32)
            / 100;
    }
    if random.next_draw()? % 100 < 50 {
        next[0] = next[0].wrapping_neg();
    }
    if random.next_draw()? % 100 < 50 {
        next[1] = next[1].wrapping_neg();
    }
    p[24] = next[0];
    p[25] = next[1];
    p[21] = next[2];
    p[20] = 0;
    Ok(())
}
pub(crate) fn jitter_input(
    frame: &PictureEffectFrame,
    p: [i32; 26],
    base: ImageInput,
    random: &mut PatternRng,
) -> Result<Option<ImageInput>> {
    let dx =
        crate::native_curve::value_with_random(p[20], p[21], p[22], p[24], p[7], p[6], random)?;
    let dy =
        crate::native_curve::value_with_random(p[20], p[21], p[23], p[25], p[7], p[6], random)?;
    let Some(source) = &frame.source else {
        return Ok(None);
    };
    // Native Put uses the complete picture, ignores src/dst extents and uses
    // the fixed background-layer convention, independently of notTrans.
    let size = source.size();
    Ok(Some(ImageInput::Composite {
        base: Box::new(base),
        layers: vec![crysirror_core::image_processing::ImageCompositeLayer {
            input: Box::new(ImageInput::Resource(source.clone())),
            source: [0, 0, size[0] as i32, size[1] as i32],
            destination: [
                frame.rectangles.destination[0].wrapping_add(dx),
                frame.rectangles.destination[1].wrapping_add(dy),
            ],
            clip: None,
            opacity: 255,
            math: if frame.slot < 4 || frame.slot == 14 {
                crysirror_core::image_processing::ImageCompositeMath::CopyRgb
            } else {
                crysirror_core::image_processing::ImageCompositeMath::MaskSeparate256
            },
        }],
    }))
}

pub(crate) fn zoom_parameters(p: &[i32], canvas: [u32; 2]) -> Result<[i32; 9]> {
    ensure!(
        canvas.iter().all(|v| (1..=16384).contains(v)),
        "NNN BGZOOMIN canvas"
    );
    let [w, h] = canvas.map(|v| v as i32);
    let get = |i: usize, d| p.get(i).copied().unwrap_or(d);
    let x = get(1, w / 4).clamp(0, w - 1);
    let y = get(2, h / 4).clamp(0, h - 1);
    Ok([
        get(0, 20),
        x,
        y,
        get(3, w / 2).min(w - x),
        get(4, h / 2).min(h - y),
        get(5, 0),
        get(6, 0),
        get(7, 100),
        get(8, 0),
    ])
}
pub(crate) fn zoom_input(
    frame: &PictureEffectFrame,
    p: [i32; 9],
    random: &mut PatternRng,
) -> Result<Option<ImageInput>> {
    let [w, h] = frame.canvas.map(|v| v as i32);
    // Print evaluates this curve before checking native picture presence.
    let phase = crate::native_curve::value_with_random(
        frame.state.count,
        frame.state.frames,
        p[8].wrapping_mul(100),
        p[7].wrapping_mul(100),
        p[6],
        p[5],
        random,
    )?;
    let offset = |v: i32| v.wrapping_mul(phase).wrapping_add(5000) / 10000;
    let x1 = offset(p[1]).max(0);
    let y1 = offset(p[2]).max(0);
    let x2 = w
        .wrapping_add(offset(
            p[1].wrapping_add(p[3]).wrapping_sub(1).wrapping_sub(w),
        ))
        .min(w);
    let y2 = h
        .wrapping_add(offset(
            p[2].wrapping_add(p[4]).wrapping_sub(1).wrapping_sub(h),
        ))
        .min(h);
    if x2 <= x1 || y2 <= y1 {
        return Ok(None);
    }
    let Some(source) = frame.source.as_ref().filter(|s| s.size() == frame.canvas) else {
        return Ok(None);
    };
    let spec = RectMap {
        regions: vec![RectMapRegion {
            source: [x1, y1, x2 - x1, y2 - y1],
            destination: [0, 0, w, h],
        }],
        output_size: Some(frame.canvas),
        clear: [0, 0, 0, 255],
        sample: RectMapSample::FloorQuantized {
            fractional_bits: [Some(16), Some(16)],
        },
        copy_alpha: false,
    };
    Ok(Some(ImageInput::Process {
        input: Box::new(ImageInput::Resource(source.clone())),
        effects: serde_json::to_vec(&[ImageEffect::RectMap { spec }])?.into(),
    }))
}

/// effectCurtain.cpp: native parameter storage and five ordered curve samples.
/// Unused omitted para13..15 were copied from uninitialised stack memory;
/// define them as zero while preserving explicitly supplied/save values.
pub(crate) fn curtain_parameters(p: &[i32]) -> [i32; 16] {
    let mut out = [0; 16];
    for i in 0..16 {
        let default = match i {
            0 => 20,
            1 | 2 => 100,
            4 => out[3],
            5 => out[4],
            8 => out[3],
            9 => out[4],
            10 => out[5],
            _ => 0,
        };
        out[i] = p.get(i).copied().unwrap_or(default);
    }
    out
}
pub(crate) fn curtain_input(
    frame: &PictureEffectFrame,
    p: [i32; 16],
    base: ImageInput,
    random: &mut PatternRng,
) -> Result<Option<ImageInput>> {
    use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath};
    let [w, h] = frame.canvas;
    let up = crate::native_curve::value_with_random(
        frame.state.count,
        frame.state.frames,
        0,
        p[1],
        p[7],
        p[6],
        random,
    )?;
    let mut down = crate::native_curve::value_with_random(
        frame.state.count,
        frame.state.frames,
        0,
        (h as i32).wrapping_sub(p[1]).wrapping_sub(p[2]),
        p[7],
        p[6],
        random,
    )?;
    let mut rgb = [0i32; 3];
    for c in 0..3 {
        rgb[c] = crate::native_curve::value_with_random(
            frame.state.count,
            frame.state.frames,
            p[c + 8],
            p[c + 3],
            p[12],
            p[11],
            random,
        )?;
    }
    if up.wrapping_add(down) > h as i32 {
        down = (h as i32).wrapping_sub(up);
    }
    // Source REP STOSD can overrun its framebuffer for oversized curve values.
    // Clip writes to the engine canvas rather than accessing unowned memory.
    let up = up.clamp(0, h as i32);
    let down = down.clamp(0, h as i32);
    let packed = ((rgb[0] as u32) << 16) | ((rgb[1] as u32) << 8) | rgb[2] as u32;
    let color = [(packed >> 16) as u8, (packed >> 8) as u8, packed as u8, 255];
    let mut layers = Vec::with_capacity(2);
    for (y, rows) in [(0, up), (h as i32 - down, down)] {
        if rows > 0 {
            layers.push(ImageCompositeLayer {
                input: Box::new(ImageInput::Solid {
                    size: [w, rows as u32],
                    color,
                }),
                source: [0, 0, w as i32, rows],
                destination: [0, y],
                clip: None,
                opacity: 255,
                math: ImageCompositeMath::CopyRgb,
            });
        }
    }
    if layers.is_empty() {
        return Ok(None);
    }
    Ok(Some(ImageInput::Composite {
        base: Box::new(base),
        layers,
    }))
}

/// effectMeltDown.cpp uses a forward overlapping REP MOVSD, not memmove:
/// every row below the selected row becomes that same row. Shared RectMap
/// expresses this propagation over immutable engine images without guest pixels.
pub(crate) fn meltdown_input(
    frame: &PictureEffectFrame,
    p: [i32; 3],
    base: ImageInput,
    random: &mut PatternRng,
) -> Result<Option<ImageInput>> {
    let [w, h] = frame.canvas.map(|v| v as i32);
    let height = crate::native_curve::value_with_random(
        frame.state.count,
        frame.state.frames,
        0,
        h - 1,
        p[2],
        p[1],
        random,
    )?;
    let y = (h - 1).wrapping_sub(height).max(0);
    if y >= h - 1 {
        return Ok(None);
    }
    let spec = RectMap {
        regions: vec![
            RectMapRegion {
                source: [0, 0, w, y + 1],
                destination: [0, 0, w, y + 1],
            },
            RectMapRegion {
                source: [0, y, w, 1],
                destination: [0, y + 1, w, h - y - 1],
            },
        ],
        output_size: Some(frame.canvas),
        clear: [0; 4],
        sample: RectMapSample::Floor,
        copy_alpha: true,
    };
    Ok(Some(ImageInput::Process {
        input: Box::new(base),
        effects: serde_json::to_vec(&[ImageEffect::RectMap { spec }])?.into(),
    }))
}

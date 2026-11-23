// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! TACLIP136 software32 native parameter, curve-order and decoration policy.
//! The D3D implementation uses unrelated zoom parameters and is deliberately
//! not substituted for this masked software painter. Init/Calcu/BeginPrint are
//! empty; CountIsMax is TRUE and CheckNeedPicPrint is FALSE.
use anyhow::{Result, ensure};
use crysirror_image::{
    clipped_scanline_sample::ScanlineClip, floating_lines::FloatingLine,
    masked_clip_blit::MaskedClipBlit, temporal_pattern::PatternRng,
};

pub(crate) struct TaClipDraw {
    pub sample: MaskedClipBlit,
    pub lines: Vec<FloatingLine>,
    pub boxes: Vec<[i32; 4]>,
    pub color: [i32; 3],
    pub percent: i32,
}
pub(crate) fn parameters(values: &[i32]) -> [i32; 35] {
    let mut p = [0; 35];
    for i in 0..35 {
        p[i] = values.get(i).copied().unwrap_or(match i {
            0 => 20,
            // Source constants do not follow the current canvas centre.
            1 => 400,
            2 => 300,
            3 => -60,
            4 => -40,
            6 | 17 => 100,
            7 => p[6],
            13..=15 | 18..=20 => 255,
            21 => p[17],
            24..=26 => p[i - 6],
            29..=32 => p[i - 28],
            _ => 0,
        });
    }
    p
}
pub(crate) fn validate(p: &[i32; 35], frames: i32) -> Result<()> {
    crate::native_curve::validate(frames, p[34], p[33])?;
    crate::native_curve::validate(frames, p[9], p[8])?;
    if p[16] > 0 {
        ensure!(p[16] <= 16384, "NNN TACLIP decoration work budget");
        crate::native_curve::validate(frames, p[28], p[27])?;
        crate::native_curve::validate(frames, p[23], p[22])?;
    }
    if p[10] != 0 {
        // Print really passes para11 as subframe and para12 as type,
        // despite SetParam's comments giving those two the opposite names.
        crate::native_curve::validate(frames, p[11], p[12])?;
    }
    Ok(())
}
pub(crate) fn draw_plan(
    p: &[i32; 35],
    count: i32,
    frames: i32,
    canvas: [u32; 2],
    picture: Option<[i32; 2]>,
    random: &mut PatternRng,
) -> Result<Option<TaClipDraw>> {
    validate(p, frames)?;
    ensure!(
        canvas.iter().all(|n| (1..=16384).contains(n)),
        "NNN TACLIP canvas"
    );
    let [w, h] = canvas.map(|n| n as i32);
    let phase =
        crate::native_curve::value_with_random(count, frames, 0, 10000, p[34], p[33], random)?;
    let interpolate = |start: i32, end: i32| {
        start.wrapping_add(end.wrapping_sub(start).wrapping_mul(phase) / 10000)
    };
    let point = [interpolate(p[29], p[1]), interpolate(p[30], p[2])];
    let mut vector = [interpolate(p[31], p[3]), interpolate(p[32], p[4])];
    if vector == [0, 0] {
        vector[0] = 1;
    }
    let square = p[5];
    let line = p[16];
    let mut color = [p[18], p[19], p[20]];
    let mut percent = p[17];
    if line > 0 {
        for channel in 0..3 {
            color[channel] = crate::native_curve::value_with_random(
                count,
                frames,
                p[24 + channel],
                p[18 + channel],
                p[28],
                p[27],
                random,
            )?;
        }
        percent = crate::native_curve::value_with_random(
            count, frames, p[21], p[17], p[23], p[22], random,
        )?;
    }
    // Vector and line curves precede the native pic==-1/null checks. Keep
    // their draws even for an unassigned picture, but not later opacity/fade.
    let Some(destination) = picture else {
        return Ok(None);
    };
    let clip = if square == 0 {
        ScanlineClip::HalfPlane {
            origin: point,
            vector,
        }
    } else {
        if vector[0] == 0 || vector[1] == 0 {
            return Ok(None);
        }
        // The source has `if(1)` in both square branches: all nonzero square
        // modes use interpolated coordinates, unlike CLIPZOOMIN122.
        let end = [
            point[0].wrapping_add(vector[0]),
            point[1]
                .wrapping_add(vector[1])
                .wrapping_add(if vector[1] > 0 { -1 } else { 1 }),
        ];
        let left = point[0].min(end[0]);
        let right = point[0].max(end[0]);
        let top = point[1].min(end[1]);
        let bottom = point[1].max(end[1]);
        if left >= w || top >= h || right < 0 || bottom < 0 {
            return Ok(None);
        }
        ScanlineClip::Rectangle {
            bounds: [
                left,
                top,
                right.wrapping_sub(left),
                bottom.wrapping_sub(top).wrapping_add(1),
            ],
        }
    };
    let opacity = crate::native_curve::value_with_random(
        count,
        frames,
        p[7].wrapping_mul(100),
        p[6].wrapping_mul(100),
        p[9],
        p[8],
        random,
    )?;
    let weight = (opacity.wrapping_mul(255) / 10000) as u16;
    let source_scale_add = if p[10] != 0 {
        let fade =
            crate::native_curve::value_with_random(count, frames, 0, 255, p[11], p[12], random)?;
        let packed = fade.wrapping_shl(16) | fade.wrapping_shl(8) | fade;
        let scale = [(packed >> 16) as u8, (packed >> 8) as u8, packed as u8];
        let flash = [p[13], p[14], p[15]].map(|v| 255i32.wrapping_sub(fade).wrapping_mul(v) / 255);
        let add = (i64::from(flash[0]).wrapping_shl(32)
            | i64::from(flash[1]).wrapping_shl(16)
            | i64::from(flash[2])) as u64;
        Some((scale, [(add >> 32) as u16, (add >> 16) as u16, add as u16]))
    } else {
        None
    };
    let sample = MaskedClipBlit {
        destination,
        clip,
        weight,
        source_scale_add,
    };
    sample.validate()?;
    let mut lines = Vec::new();
    let mut boxes = Vec::new();
    if line > 0 {
        if square == 0 {
            let [cx, cy] = point;
            let [vx, vy] = vector;
            let abs = |v: i32| if v < -1 { v.wrapping_neg() } else { v };
            // Source abs(-1) remains-1. Its resulting zero-denominator branch
            // yields infinity and undefined pixel casts for an axis vector.
            // Select the finite axis branch only for those undefined cases.
            let major_x = vy == 0 || vx != 0 && abs(vx) > abs(vy);
            for i in 0..line {
                let shift = (i - line / 2) as f32;
                let (from, to) = if major_x {
                    let slope = (vy as f32) / (vx as f32);
                    (
                        [
                            -(line as f32),
                            -((cx.wrapping_add(line)) as f32) * slope + cy as f32 + shift,
                        ],
                        [
                            (w.wrapping_add(line)) as f32,
                            slope * (w.wrapping_add(line).wrapping_sub(cx)) as f32
                                + cy as f32
                                + shift,
                        ],
                    )
                } else {
                    let slope = (vx as f32) / (vy as f32);
                    (
                        [
                            -((cy.wrapping_add(line)) as f32) * slope + cx as f32 + shift,
                            -(line as f32),
                        ],
                        [
                            slope * (h.wrapping_add(line).wrapping_sub(cy)) as f32
                                + cx as f32
                                + shift,
                            (h.wrapping_add(line)) as f32,
                        ],
                    )
                };
                lines.push(FloatingLine {
                    from,
                    to,
                    color,
                    percent,
                });
            }
        } else {
            let start = point;
            let size = vector;
            let end = [
                start[0].wrapping_add(size[0]),
                start[1].wrapping_add(size[1]),
            ];
            let left = start[0].min(end[0]);
            let right = start[0].max(end[0]);
            let top = start[1].min(end[1]);
            let bottom = start[1].max(end[1]);
            let x = left.wrapping_sub(line / 2);
            let y = top.wrapping_sub(line / 2);
            let width = right.wrapping_sub(left).wrapping_add(line);
            boxes.push([x, y, width, line]);
            let mut bottom_y = bottom.wrapping_sub(line / 2);
            let mut bottom_h = line;
            if bottom_y < y.wrapping_add(line) {
                bottom_h = bottom_h.wrapping_sub(y.wrapping_add(line).wrapping_sub(bottom_y));
                bottom_y = y.wrapping_add(line);
            }
            if bottom_h > 0 {
                boxes.push([x, bottom_y, width, bottom_h]);
            }
            let middle = bottom_y.wrapping_sub(y.wrapping_add(line));
            if middle > 0 {
                boxes.push([x, y.wrapping_add(line), line, middle]);
                let mut right_x = right.wrapping_sub(line / 2);
                let mut right_w = line;
                if right_x < x.wrapping_add(right_w) {
                    // Original expression really subtracts the entire top
                    // width, not only the horizontal overlap.
                    right_w = right_w.wrapping_sub(width);
                    right_x = x.wrapping_add(width);
                }
                if right_w > 0 {
                    boxes.push([right_x, y.wrapping_add(line), right_w, middle]);
                }
            }
        }
    }
    Ok(Some(TaClipDraw {
        sample,
        lines,
        boxes,
        color,
        percent,
    }))
}

pub(crate) fn image_input(
    frame: &crate::native_picture_effect::PictureEffectFrame,
    p: &[i32; 35],
    base: crysirror_core::image_processing::ImageInput,
    random: &mut PatternRng,
) -> Result<Option<crysirror_core::image_processing::ImageInput>> {
    use crysirror_core::image_processing::ImageInput;
    use crysirror_image::{
        byte_raster::{ByteRaster, ByteRasterMath},
        effects::ImageEffect,
    };
    let picture = (frame.geometry_assigned && frame.source.is_some()).then_some([
        frame.rectangles.destination[0],
        frame.rectangles.destination[1],
    ]);
    let Some(plan) = draw_plan(
        p,
        frame.state.count,
        frame.state.frames,
        frame.canvas,
        picture,
        random,
    )?
    else {
        return Ok(None);
    };
    let source = frame.source.as_ref().unwrap().clone();
    let source = if frame.has_mask {
        ImageInput::Resource(source)
    } else {
        // The original software path dereferences a null mask on maskless
        // pictures. Define that invalid memory case as full coverage: RGB
        // does not become a corner key and remains in the shared image graph.
        ImageInput::Process {
            input: Box::new(ImageInput::Resource(source)),
            effects: serde_json::to_vec(&[ImageEffect::OverrideAlpha { alpha: 255 }])?.into(),
        }
    };
    let mut effects = vec![ImageEffect::ByteRaster {
        spec: ByteRaster::MaskedClipBlit { spec: plan.sample },
    }];
    if !plan.lines.is_empty() {
        effects.push(ImageEffect::FloatingLines { lines: plan.lines });
    }
    let mut input = ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(base),
            source: Box::new(source),
        }),
        effects: serde_json::to_vec(&effects)?.into(),
    };
    let packed = plan.color[0].wrapping_shl(16) | plan.color[1].wrapping_shl(8) | plan.color[2];
    let color = [(packed >> 16) as u8, (packed >> 8) as u8, packed as u8, 255];
    let weight = (plan.percent.clamp(0, 100) * 256 / 100) as u16;
    for rect in plan.boxes {
        input = crate::native_picture_target::rectangle(
            input,
            frame.canvas,
            rect,
            color,
            ByteRasterMath::MaskPiecewiseWrapping256 { weight },
        )?;
    }
    Ok(Some(input))
}

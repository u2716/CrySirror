// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CLIPZOOMIN122 native33 words, curve order and optional line/frame decoration.
//! Shared directional clipping/fixed scanline sampling renders the zoom;
//! shared floating strokes and rectangle fills render its native decoration.
//! Native axis loops can index temporary rows outside allocation when the
//! origin is offscreen. Shared clipping supplies a defined bounded replacement.
use anyhow::{Result, ensure};
use crysirror_image::{
    byte_raster::ByteRasterMath,
    clipped_scanline_sample::{ClippedScanlineSample, ScanlineClip},
    floating_lines::FloatingLine,
    temporal_pattern::PatternRng,
};
#[derive(Clone, Debug)]
pub(crate) struct ClipZoomDraw {
    pub sample: ClippedScanlineSample,
    pub lines: Vec<FloatingLine>,
    pub boxes: Vec<[i32; 4]>,
    pub color: [i32; 3],
    pub percent: i32,
}
pub(crate) fn parameters(p: &[i32], canvas: [u32; 2]) -> [i32; 33] {
    let [w, h] = canvas.map(|v| v as i32);
    let mut out: [i32;33] = [0; 33];
    for i in 0..33 {
        out[i] = p.get(i).copied().unwrap_or(match i {
            0 => 20,
            1 => w / 4,
            2 => h / 4,
            3 => w / 2,
            4 => h / 2,
            7 | 21 => 100,
            9 => out[1].wrapping_add(out[3] / 2),
            10 => out[2].wrapping_add(out[4] / 2),
            11 => -60,
            12 => -40,
            14 => out[9],
            15 => out[10],
            16 => out[11],
            17 => out[12],
            22..=24 => 255,
            25 => out[21],
            28 => out[22],
            29 => out[23],
            30 => out[24],
            31 => out[26],
            32 => out[27],
            _ => 0,
        });
        // Source clamps source origins/extent before dependent center defaults.
        if i == 1 {
            out[1] = out[1].clamp(0, w - 1);
        }
        if i == 2 {
            out[2] = out[2].clamp(0, h - 1);
        }
        if i == 3 && out[1].wrapping_add(out[3]) > w {
            out[3] = w - out[1];
        }
        if i == 4 && out[2].wrapping_add(out[4]) > h {
            out[4] = h - out[2];
        }
    }
    out
}
pub(crate) fn validate(p: &[i32; 33], frames: i32) -> Result<()> {
    crate::native_curve::validate(frames, p[6], p[5])?;
    crate::native_curve::validate(frames, p[19], p[18])?;
    if p[20] > 0 {
        crate::native_curve::validate(frames, p[32], p[31])?;
        crate::native_curve::validate(frames, p[27], p[26])?;
        ensure!(p[20] <= 16384, "NNN CLIPZOOMIN decoration work budget");
    }
    Ok(())
}
pub(crate) fn draw_plan(
    p: [i32; 33],
    count: i32,
    frames: i32,
    canvas: [u32; 2],
    random: &mut PatternRng,
) -> Result<Option<ClipZoomDraw>> {
    validate(&p, frames)?;
    let [w, h] = canvas.map(|v| v as i32);
    let zoom = crate::native_curve::value_with_random(
        count,
        frames,
        p[8].wrapping_mul(100),
        p[7].wrapping_mul(100),
        p[6],
        p[5],
        random,
    )?;
    let vector_phase =
        crate::native_curve::value_with_random(count, frames, 0, 10000, p[19], p[18], random)?;
    let interpolate = |start: i32, end: i32| {
        start.wrapping_add(end.wrapping_sub(start).wrapping_mul(vector_phase) / 10000)
    };
    let point = [interpolate(p[14], p[9]), interpolate(p[15], p[10])];
    let mut vector = [interpolate(p[16], p[11]), interpolate(p[17], p[12])];
    if vector == [0, 0] {
        vector[0] = 1;
    }
    let line = p[20];
    let mut color = [p[22], p[23], p[24]];
    let mut percent = p[21];
    if line > 0 {
        for c in 0..3 {
            color[c] = crate::native_curve::value_with_random(
                count,
                frames,
                p[28 + c],
                p[22 + c],
                p[32],
                p[31],
                random,
            )?;
        }
        percent = crate::native_curve::value_with_random(
            count, frames, p[25], p[21], p[27], p[26], random,
        )?;
    }
    let rectangle = if p[13] != 0 {
        if vector[0] == 0 || vector[1] == 0 {
            return Ok(None);
        }
        let start = if p[13] == 2 { point } else { [p[9], p[10]] };
        let size = if p[13] == 2 { vector } else { [p[11], p[12]] };
        let mut end = [
            start[0].wrapping_add(size[0]),
            start[1].wrapping_add(size[1]),
        ];
        end[1] = end[1].wrapping_add(if size[1] > 0 { -1 } else { 1 });
        let left = start[0].min(end[0]);
        let right = start[0].max(end[0]);
        let top = start[1].min(end[1]);
        let bottom = start[1].max(end[1]);
        if left >= w || top >= h || right < 0 || bottom < 0 {
            return Ok(None);
        }
        Some([
            left,
            top,
            right.wrapping_sub(left),
            bottom.wrapping_sub(top).wrapping_add(1),
        ])
    } else {
        None
    };
    let clip = rectangle
        .map(|bounds| ScanlineClip::Rectangle { bounds })
        .unwrap_or(ScanlineClip::HalfPlane {
            origin: point,
            vector,
        });
    let from = [
        (p[1].wrapping_mul(zoom).wrapping_add(5000) / 10000).max(0),
        (p[2].wrapping_mul(zoom).wrapping_add(5000) / 10000).max(0),
    ];
    let to = [
        w.wrapping_add(
            p[1].wrapping_add(p[3])
                .wrapping_sub(1)
                .wrapping_sub(w)
                .wrapping_mul(zoom)
                .wrapping_add(5000)
                / 10000,
        )
        .min(w),
        h.wrapping_add(
            p[2].wrapping_add(p[4])
                .wrapping_sub(1)
                .wrapping_sub(h)
                .wrapping_mul(zoom)
                .wrapping_add(5000)
                / 10000,
        )
        .min(h),
    ];
    if to[0] <= from[0] || to[1] <= from[1] {
        return Ok(None);
    }
    let sample = ClippedScanlineSample {
        source_q16: from.map(|v| i64::from(v) * 65536),
        source_step_q16: [
            i64::from((to[0] - from[0]).wrapping_mul(65536) / w),
            i64::from((to[1] - from[1]).wrapping_mul(65536) / h),
        ],
        clip,
        math: ByteRasterMath::CopyRgb,
    };
    let mut lines = Vec::new();
    let mut boxes = Vec::new();
    if line > 0 {
        if p[13] == 0 {
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
            let start = if p[13] == 2 { point } else { [p[9], p[10]] };
            let size = if p[13] == 2 { vector } else { [p[11], p[12]] };
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
    Ok(Some(ClipZoomDraw {
        sample,
        lines,
        boxes,
        color,
        percent,
    }))
}

pub(crate) fn image_input(
    frame: &crate::native_picture_effect::PictureEffectFrame,
    p: [i32; 33],
    base: crysirror_core::image_processing::ImageInput,
    random: &mut PatternRng,
) -> Result<Option<crysirror_core::image_processing::ImageInput>> {
    use crysirror_core::image_processing::ImageInput;
    use crysirror_image::effects::ImageEffect;
    let Some(plan) = draw_plan(
        p,
        frame.state.count,
        frame.state.frames,
        frame.canvas,
        random,
    )?
    else {
        return Ok(None);
    };
    if !frame.geometry_assigned {
        return Ok(None);
    }
    let Some(source) = &frame.source else {
        return Ok(None);
    };
    if source.size()[0] < frame.canvas[0] {
        return Ok(None);
    }
    // Original never checks source height before its raw reads. Shared sampling
    // defines out-of-source texels as no writes, avoiding out-of-allocation RAM.
    let mut effects = vec![ImageEffect::ByteRaster {
        spec: crysirror_image::byte_raster::ByteRaster::ClippedScanlineSample { spec: plan.sample },
    }];
    if !plan.lines.is_empty() {
        effects.push(ImageEffect::FloatingLines { lines: plan.lines });
    }
    let mut input = ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(base),
            source: Box::new(ImageInput::Resource(source.clone())),
        }),
        effects: serde_json::to_vec(&effects)?.into(),
    };
    let word = plan.color[0].wrapping_shl(16) | plan.color[1].wrapping_shl(8) | plan.color[2];
    let color = [(word >> 16) as u8, (word >> 8) as u8, word as u8, 255];
    let a = plan.percent.clamp(0, 100) * 256 / 100;
    let weight = if a == 256 { 256 } else { a + 1 } as u16;
    for rect in plan.boxes {
        input = crate::native_picture_target::rectangle(
            input,
            frame.canvas,
            rect,
            color,
            ByteRasterMath::AlphaWeightedRgb256 { weight },
        )?;
    }
    Ok(Some(input))
}

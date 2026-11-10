// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! SENTENCE133 reveals image atlas cells; it does not draw font glyphs.
//! Native reading order, curve/RNG and seven partial-cell modes remain here;
//! shared rectangle copies and fixed scanline sampling own image work.
use anyhow::{Result, ensure};
use crysirror_image::temporal_pattern::PatternRng;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SentenceCopy {
    Masked,
    Translucent(i32),
    Stretch(i32),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SentenceDraw {
    pub source: [i32; 4],
    pub destination: [i32; 4],
    pub mode: SentenceCopy,
}
pub(crate) fn parameters(p: &[i32]) -> [i32; 16] {
    let mut out = std::array::from_fn(|i| {
        p.get(i).copied().unwrap_or(match i {
            0 => 20,
            1 | 2 => 32,
            5 => 1,
            _ => 0,
        })
    });
    out[5] = out[5].max(1);
    // The source copied uninitialised para8..15 when omitted. Their explicit
    // zero defaults avoid uninitialised memory; supplied/saved words survive.
    out
}
pub(crate) fn validate(p: &[i32; 16], frames: i32) -> Result<()> {
    ensure!(
        p[1] != 0 && p[2] != 0 && p[5] > 0,
        "NNN SENTENCE native division requires nonzero cell sizes/positive resolution"
    );
    crate::native_curve::validate(frames, p[7], p[6])
}
pub(crate) fn draw_plan(
    p: [i32; 16],
    count: i32,
    frames: i32,
    picture: [u32; 2],
    point: [i32; 2],
    random: &mut PatternRng,
) -> Result<Vec<SentenceDraw>> {
    validate(&p, frames)?;
    let [cell_x, cell_y] = [p[1], p[2]];
    let columns = picture[0] as i32 / cell_x;
    let rows = picture[1] as i32 / cell_y;
    if columns <= 0 || rows <= 0 {
        return Ok(Vec::new());
    }
    let vertical = p[3] != 0;
    let primary = if vertical { rows } else { columns };
    let d = crate::native_curve::value_with_random(
        count,
        frames,
        0,
        columns.wrapping_mul(rows).wrapping_mul(p[5]),
        p[7],
        p[6],
        random,
    )?;
    let printed = d / p[5];
    let percent = (d % p[5]).wrapping_mul(100) / p[5];
    let block = printed / primary;
    let center = printed % primary;
    let mut draws = Vec::with_capacity(3);
    let mut copy = |source: [i32; 4], mode: SentenceCopy| {
        if source[2] > 0 && source[3] > 0 {
            draws.push(SentenceDraw {
                source,
                destination: [
                    point[0].wrapping_add(source[0]),
                    point[1].wrapping_add(source[1]),
                    source[2],
                    source[3],
                ],
                mode,
            });
        }
    };
    if printed > 0 {
        if block > 0 {
            copy(
                if vertical {
                    [
                        columns.wrapping_sub(block).wrapping_mul(cell_x),
                        0,
                        block.wrapping_mul(cell_x),
                        rows.wrapping_mul(cell_y),
                    ]
                } else {
                    [
                        0,
                        0,
                        columns.wrapping_mul(cell_x),
                        block.wrapping_mul(cell_y),
                    ]
                },
                SentenceCopy::Masked,
            );
        }
        if center > 0 {
            copy(
                if vertical {
                    [
                        columns
                            .wrapping_sub(block)
                            .wrapping_sub(1)
                            .wrapping_mul(cell_x),
                        0,
                        cell_x,
                        center.wrapping_mul(cell_y),
                    ]
                } else {
                    [
                        0,
                        block.wrapping_mul(cell_y),
                        center.wrapping_mul(cell_x),
                        cell_y,
                    ]
                },
                SentenceCopy::Masked,
            );
        }
    }
    if percent <= 0 {
        return Ok(draws);
    }
    let mut source = if vertical {
        [
            columns
                .wrapping_sub(printed / primary)
                .wrapping_sub(1)
                .wrapping_mul(cell_x),
            (printed % primary).wrapping_mul(cell_y),
            cell_x,
            cell_y,
        ]
    } else {
        [
            (printed % primary).wrapping_mul(cell_x),
            (printed / primary).wrapping_mul(cell_y),
            cell_x,
            cell_y,
        ]
    };
    let mut destination = [
        point[0].wrapping_add(source[0]),
        point[1].wrapping_add(source[1]),
        cell_x,
        cell_y,
    ];
    let mode = if percent >= 100 {
        SentenceCopy::Masked
    } else {
        match p[4] {
            0 => SentenceCopy::Masked,
            1 => SentenceCopy::Translucent(percent),
            2 | 3 | 5 | 6 => {
                let factor = if p[4] >= 5 {
                    200i32.wrapping_sub(percent)
                } else {
                    percent
                };
                let width = cell_x.wrapping_mul(factor) / 100;
                let height = cell_y.wrapping_mul(factor) / 100;
                if width <= 0 || height <= 0 {
                    return Ok(draws);
                }
                let dx = cell_x.wrapping_sub(width) / 2;
                let dy = cell_y.wrapping_sub(height) / 2;
                destination[0] = destination[0].wrapping_add(dx);
                destination[1] = destination[1].wrapping_add(dy);
                destination[2] = width;
                destination[3] = height;
                if p[4] == 3 {
                    source[0] = source[0].wrapping_add(dx);
                    source[1] = source[1].wrapping_add(dy);
                    source[2] = width;
                    source[3] = height;
                    SentenceCopy::Masked
                } else {
                    SentenceCopy::Stretch(if p[4] == 6 { percent } else { 100 })
                }
            }
            4 => {
                let width = if vertical {
                    cell_x
                } else {
                    cell_x.wrapping_mul(percent) / 100
                };
                let height = if vertical {
                    cell_y.wrapping_mul(percent) / 100
                } else {
                    cell_y
                };
                source[2] = width;
                source[3] = height;
                destination[2] = width;
                destination[3] = height;
                SentenceCopy::Masked
            }
            // Original has no branch for another partial-cell selector. Already
            // completed cells still draw; the partial cell deliberately does not.
            _ => return Ok(draws),
        }
    };
    if destination[2] > 0 && destination[3] > 0 {
        draws.push(SentenceDraw {
            source,
            destination,
            mode,
        });
    }
    Ok(draws)
}

pub(crate) fn image_input(
    frame: &crate::native_picture_effect::PictureEffectFrame,
    p: [i32; 16],
    mut base: crysirror_core::image_processing::ImageInput,
    random: &mut PatternRng,
) -> Result<Option<crysirror_core::image_processing::ImageInput>> {
    use crysirror_image::byte_raster::ByteRasterMath;
    let size = frame.source.as_ref().map(|s| s.size()).unwrap_or([1, 1]);
    let draws = draw_plan(
        p,
        frame.state.count,
        frame.state.frames,
        size,
        [
            frame.rectangles.destination[0],
            frame.rectangles.destination[1],
        ],
        random,
    )?;
    let Some(source) = &frame.source else {
        return Ok(None);
    };
    let mut painted = false;
    for draw in draws {
        match draw.mode {
            SentenceCopy::Masked => {
                base = crate::native_picture_sampling::copy_input(
                    base,
                    source.clone(),
                    draw.source,
                    [draw.destination[0], draw.destination[1]],
                    ByteRasterMath::AlphaWeightedSeparateRgb256 { weight: 256 },
                    frame.canvas,
                    frame.source_clip,
                )?;
            }
            SentenceCopy::Translucent(percent) => {
                if !frame.has_mask {
                    continue;
                }
                base = crate::native_picture_sampling::copy_input(
                    base,
                    source.clone(),
                    draw.source,
                    [draw.destination[0], draw.destination[1]],
                    ByteRasterMath::MaskPiecewiseWrapping256 {
                        weight: (percent.wrapping_mul(256) / 100) as u16,
                    },
                    frame.canvas,
                    frame.source_clip,
                )?;
            }
            SentenceCopy::Stretch(percent) => {
                let Some(plan) = crate::native_stretch_blit::plan(
                    size,
                    frame.canvas,
                    draw.source,
                    draw.destination,
                    frame.has_mask,
                    true,
                    percent,
                )?
                else {
                    continue;
                };
                base =
                    crate::native_picture_sampling::fixed_input(base, source.clone(), plan, None)?;
            }
        }
        painted = true;
    }
    Ok(painted.then_some(base))
}

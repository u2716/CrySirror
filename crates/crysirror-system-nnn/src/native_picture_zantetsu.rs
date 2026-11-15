// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! ZANTETSU86 software32. Native split-table, movement and clipping policy;
//! immutable shared scanline jobs retain left-then-right pixel ordering.
use crate::native_picture_clip::CharaClip;
use anyhow::{Result, ensure};
use crysirror_image::{
    fixed_scanlines::{FixedSampleSpan, FixedScanlines, ScanlineMath},
    temporal_pattern::PatternRng,
};

pub(crate) fn parameters(values: &[i32], canvas: [u32; 2]) -> [i32; 13] {
    std::array::from_fn(|i| {
        values.get(i).copied().unwrap_or(match i {
            0 => 20,
            2 | 9 => 100,
            3 => canvas[0] as i32 / 2,
            4 => canvas[1] as i32 / 2,
            5 => 2,
            6 => 1,
            _ => 0,
        })
    })
}
pub(crate) fn validate(p: &[i32; 13], frames: i32) -> Result<()> {
    crate::native_curve::validate(frames, p[8], p[7])?;
    crate::native_curve::validate(frames, p[12], p[11])
}
pub(crate) struct Phase {
    angle: i32,
    center: [i32; 2],
    vector: [f64; 2],
    displacement: [[i32; 2]; 2],
    percent: i32,
}
fn integer(value: f64) -> Result<i32> {
    ensure!(
        value.is_finite() && value >= -2147483648. && value < 2147483648.,
        "NNN ZANTETSU native integer conversion"
    );
    Ok(value as i32)
}
pub(crate) fn phase(
    p: &[i32; 13],
    count: i32,
    frames: i32,
    canvas: [u32; 2],
    random: &mut PatternRng,
) -> Result<Phase> {
    ensure!(
        canvas.iter().all(|n| (3..=16384).contains(n)),
        "NNN ZANTETSU canvas dimensions"
    );
    let angle = p[1].clamp(-90, 90);
    let center = std::array::from_fn(|axis| p[3 + axis].max(1).min(canvas[axis] as i32 - 2));
    let movement =
        crate::native_curve::value_with_random(count, frames, 0, 10000, p[8], p[7], random)?;
    // Trans mode para5 is read but unused in the software implementation.
    let percent =
        crate::native_curve::value_with_random(count, frames, p[10], p[9], p[12], p[11], random)?;
    let theta = f64::from(angle) / 360. * (3.14159 * 2.);
    let vector = [-theta.sin(), theta.cos()];
    let distance = f64::from(movement.wrapping_mul(p[2])) / 10000.;
    let multipliers = match p[6] {
        1 => [1., 0.],
        2 => [0., 1.],
        3 => [1., -1.],
        _ => [0., 0.],
    };
    let mut displacement = [[0; 2]; 2];
    for half in 0..2 {
        for axis in 0..2 {
            displacement[half][axis] = integer(vector[axis] * distance * multipliers[half] + 0.5)?;
        }
    }
    Ok(Phase {
        angle,
        center,
        vector,
        displacement,
        percent,
    })
}
pub(crate) fn draw_plan(
    phase: &Phase,
    canvas: [u32; 2],
    picture: [u32; 2],
    has_mask: bool,
    clip: CharaClip,
    destination: [i32; 2],
) -> Result<Vec<FixedScanlines>> {
    // CPicture::LeftBlt/RightBlt have code only for md==2. Missing masks
    // explicitly leave the screen unchanged; there is no RGB-key fallback.
    if !has_mask {
        return Ok(Vec::new());
    }
    ensure!(
        picture.iter().all(|n| (1..=16384).contains(n)),
        "NNN ZANTETSU picture dimensions"
    );
    let width = canvas[0] as i32;
    let height = canvas[1] as i32;
    let mut cuts = Vec::with_capacity(canvas[1] as usize);
    for y in 0..height {
        let cut = match phase.angle {
            -90 => {
                if y < phase.center[1] {
                    0
                } else {
                    width
                }
            }
            90 => {
                if y < phase.center[1] {
                    width
                } else {
                    0
                }
            }
            _ => phase.center[0]
                .wrapping_add(integer(
                    phase.vector[0] / phase.vector[1] * f64::from(y - phase.center[1]) + 0.5,
                )?)
                .clamp(0, width),
        };
        cuts.push(cut);
    }
    let weight = (phase.percent.min(100).wrapping_mul(256) / 100) as u32;
    let mut out = Vec::with_capacity(2);
    for half in 0..2 {
        let point = std::array::from_fn(|axis| {
            destination[axis].wrapping_add(phase.displacement[half][axis])
        });
        let Some((region, at)) =
            clip.source_clip([0, 0, picture[0] as i32, picture[1] as i32], point)?
        else {
            continue;
        };
        let Some((region, at)) = crysirror_core::region_map::clip_region_copy(
            Some(picture),
            canvas,
            region.map(i64::from),
            at.map(i64::from),
            None,
        )?
        else {
            continue;
        };
        let table_start = at[1] - i64::from(point[1]);
        // The original allocated one table of screenHeight. Reject an actual
        // overread from a taller shifted source instead of reading host memory.
        ensure!(
            table_start >= 0 && table_start + region[3] <= i64::from(height),
            "NNN ZANTETSU cut-table row outside native allocation"
        );
        let mut spans = Vec::with_capacity(region[3] as usize);
        for row in 0..region[3] {
            let cut = i64::from(cuts[(table_start + row) as usize]);
            let (skip, len) = if half == 0 {
                (0, (cut - region[0]).min(region[2]))
            } else {
                let skip = (cut - region[0]).max(0);
                (skip, region[2] - skip)
            };
            if len < 1 {
                continue;
            }
            spans.push(FixedSampleSpan {
                destination: [(at[0] + skip) as i32, (at[1] + row) as i32, len as i32],
                source_q16: [(region[0] + skip) * 65536, (region[1] + row) * 65536],
                source_step_q16: [65536, 0],
                math: ScanlineMath::CoverageByteSeparate { weight },
            });
        }
        let spec = FixedScanlines {
            spans,
            corner_rgb_key: false,
            source_color: None,
        };
        spec.validate()?;
        out.push(spec);
    }
    Ok(out)
}

pub(crate) fn image_input(
    frame: &crate::native_picture_effect::PictureEffectFrame,
    p: &[i32; 13],
    mut base: crysirror_core::image_processing::ImageInput,
    random: &mut PatternRng,
) -> Result<Option<crysirror_core::image_processing::ImageInput>> {
    use crysirror_core::image_processing::ImageInput;
    use crysirror_image::{byte_raster::ByteRaster, effects::ImageEffect};
    let phase = phase(
        p,
        frame.state.count,
        frame.state.frames,
        frame.canvas,
        random,
    )?;
    if !frame.geometry_assigned {
        return Ok(None);
    }
    let Some(source) = &frame.source else {
        return Ok(None);
    };
    let plans = draw_plan(
        &phase,
        frame.canvas,
        source.size(),
        frame.has_mask,
        CharaClip::from_bounds(frame.source_clip.unwrap_or([
            0,
            0,
            source.size()[0] as i32,
            source.size()[1] as i32,
        ])),
        [
            frame.rectangles.destination[0],
            frame.rectangles.destination[1],
        ],
    )?;
    if plans.is_empty() {
        return Ok(None);
    }
    // LeftBlt completes before RightBlt reads the destination, including any
    // overlap after the two different displacements. Each samples the same
    // immutable native source generation, not the left painter's output.
    for spec in plans {
        base = ImageInput::Process {
            input: Box::new(ImageInput::Raster {
                destination: Box::new(base),
                source: Box::new(ImageInput::Resource(source.clone())),
            }),
            effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
                spec: ByteRaster::FixedScanlines { spec },
            }])?
            .into(),
        };
    }
    Ok(Some(base))
}

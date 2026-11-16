// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! SPECIALMOVE61 software32: native parameter/default, clock, curve and drawing
//! decisions. Sampling, stored-byte colour and composition use shared jobs.
use crate::{
    native_picture_clip::CharaClip,
    native_stretch_blit::{self, StretchMath},
};
use anyhow::{Result, ensure};
use crysirror_image::{
    byte_raster::ByteRasterMath,
    fixed_scanlines::{FixedScanlines, ScanlineMath},
    temporal_pattern::PatternRng,
};

pub(crate) fn parameters(values: &[i32]) -> [i32; 64] {
    let mut p = [0; 64];
    for (to, from) in p.iter_mut().zip(values.iter().take(37)) {
        *to = *from;
    }
    let n = values.len();
    // Preserve the original off-by-one tests: supplied words11..13 can be
    // overwritten by these defaults. Remaining omitted stack words are zero.
    for i in 0..37 {
        let threshold = match i {
            11 => 13,
            12 => 14,
            13 | 14 => 15,
            _ => i + 1,
        };
        if n >= threshold {
            continue;
        }
        p[i] = match i {
            0 => 20,
            2 | 3 => 1,
            4 => p[0],
            11 | 23 | 24 => 100,
            12 => p[11],
            13 | 19 | 27 => p[3],
            14 | 20 | 28 => p[4],
            15 | 21 | 29 => p[5],
            16 | 22 | 30 => p[6],
            25 => p[23],
            26 => p[24],
            34..=36 => p[i - 3],
            _ => 0,
        };
    }
    // SetParam overwrites these work counters even when explicitly supplied.
    p[17] = 0;
    p[18] = 0;
    p[62] = 0;
    p[63] = 0;
    p
}
pub(crate) fn validate(p: &[i32; 64]) -> Result<()> {
    for (duration, sub, kind) in [(4, 6, 5), (14, 16, 15), (20, 22, 21), (28, 30, 29)] {
        crate::native_curve::validate(p[duration].max(1), p[sub], p[kind])?;
    }
    Ok(())
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Phase {
    pub counts: [i32; 4],
    pub transparency: i32,
    pub scale: [f64; 2],
    pub scale_enabled: bool,
    pub color: Option<([u8; 3], [u8; 3])>,
    pub displacement: [i32; 2],
}
fn integer(value: f64) -> Result<i32> {
    ensure!(
        value.is_finite() && value >= -2147483648. && value < 2147483648.,
        "NNN SPECIALMOVE native coordinate conversion"
    );
    Ok(value as i32)
}
fn packed(values: [i32; 3]) -> [u8; 3] {
    let word = values[0].wrapping_shl(16) | values[1].wrapping_shl(8) | values[2];
    [(word >> 16) as u8, (word >> 8) as u8, word as u8]
}
pub(crate) fn phase(p: &[i32; 64], count: i32, random: &mut PatternRng) -> Result<Phase> {
    // Calcu's first statement is return. Print ignores the old work counters,
    // deriving all four clocks from lp->count, then stores them before curves.
    let groups = [(3, 4), (13, 14), (19, 20), (27, 28)];
    let counts = groups.map(|(loop_word, duration_word)| {
        let duration = p[duration_word].max(1);
        let value = if count >= duration {
            if p[loop_word] != 0 {
                count % duration
            } else {
                duration
            }
        } else {
            count
        };
        value.clamp(0, duration)
    });
    let sample =
        |clock: usize, start: i32, end: i32, sub: usize, kind: usize, random: &mut PatternRng| {
            crate::native_curve::value_with_random(
                counts[clock],
                p[groups[clock].1].max(1),
                start,
                end,
                p[sub],
                p[kind],
                random,
            )
        };
    // Exact Print evaluation order matters for random curve selectors. Only
    // transparency elides the call when start==end; scale/RGB/move never do.
    let transparency = if p[12] == p[11] {
        p[12]
    } else {
        sample(1, p[12], p[11], 16, 15, random)?
    };
    let scale = [
        0.001
            * f64::from(sample(
                2,
                p[25].wrapping_mul(10),
                p[23].wrapping_mul(10),
                22,
                21,
                random,
            )?),
        0.001
            * f64::from(sample(
                2,
                p[26].wrapping_mul(10),
                p[24].wrapping_mul(10),
                22,
                21,
                random,
            )?),
    ];
    let scale_enabled = [p[25], p[26], p[23], p[24]].iter().any(|n| *n != 100);
    let delta = [
        sample(3, p[34], p[31], 30, 29, random)?,
        sample(3, p[35], p[32], 30, 29, random)?,
        sample(3, p[36], p[33], 30, 29, random)?,
    ];
    let color = (p[31..=36].iter().any(|n| *n != 0)).then(|| {
        (
            packed(delta.map(|n| n.max(0))),
            packed(delta.map(|n| n.wrapping_neg().max(0))),
        )
    });
    let progress = sample(0, 0, 1000, 6, 5, random)?;
    let mut displacement = [0; 2];
    for axis in 0..2 {
        let start = p[7 + axis];
        let end = p[9 + axis];
        displacement[axis] = match p[1] {
            0 => start.wrapping_add(end.wrapping_sub(start).wrapping_mul(progress) / 1000),
            1 => {
                let theta = f64::from(progress) * 3.14159 / 1000.;
                let eased = ((1. - theta.cos()) * 0.5).clamp(0., 1.);
                start.wrapping_add(integer(f64::from(end.wrapping_sub(start)) * eased + 0.5)?)
            }
            2 => {
                let t = f64::from(progress) * 0.001;
                integer(2. * t * (1. - t) * f64::from(start) + t * t * f64::from(end) + 0.5)?
            }
            _ => 0, // Original explicit fall-through leaves dx/dy zero.
        };
    }
    Ok(Phase {
        counts,
        transparency,
        scale,
        scale_enabled,
        color,
        displacement,
    })
}
pub(crate) fn store_print_clocks(p: &mut [i32; 64], phase: &Phase) {
    for (index, value) in [17, 18, 62, 63].into_iter().zip(phase.counts) {
        p[index] = value;
    }
}

/// Call after phase(), even when the native picture is absent: Print's clock
/// writes and curve RNG effects precede that resource early return.
pub(crate) fn draw_plan(
    p: &[i32; 64],
    phase: &Phase,
    picture: [u32; 2],
    canvas: [u32; 2],
    has_mask: bool,
    clip: CharaClip,
    destination: [i32; 2],
) -> Result<Option<FixedScanlines>> {
    ensure!(
        picture
            .iter()
            .chain(&canvas)
            .all(|n| (1..=16384).contains(n)),
        "NNN SPECIALMOVE image dimensions"
    );
    let point =
        std::array::from_fn(|axis| destination[axis].wrapping_add(phase.displacement[axis]));
    let trans = phase.transparency;
    if trans == 0 {
        return Ok(None);
    }
    let transparent = p[2] != 0;
    if phase.scale_enabled {
        let mut dimensions = [0; 2];
        for axis in 0..2 {
            dimensions[axis] = integer(0.5 + phase.scale[axis] * f64::from(picture[axis]))?;
        }
        if dimensions.iter().any(|n| *n <= 0) {
            return Ok(None);
        }
        let destination = [
            point[0].wrapping_add((picture[0] as i32).wrapping_sub(dimensions[0]) / 2),
            point[1].wrapping_add((picture[1] as i32).wrapping_sub(dimensions[1]) / 2),
            dimensions[0],
            dimensions[1],
        ];
        // StretchBlt1 and StretchAddSubBlt use the same native16.16 clipping
        // and mode policy. These helpers intentionally ignore CharaClip.
        let Some(plan) = native_stretch_blit::plan(
            picture,
            canvas,
            [0, 0, picture[0] as i32, picture[1] as i32],
            destination,
            has_mask,
            transparent,
            trans,
        )?
        else {
            return Ok(None);
        };
        let math = match plan.math {
            StretchMath::Copy => ScanlineMath::Stored {
                math: ByteRasterMath::CopyRgb,
            },
            StretchMath::CoverageSeparate {
                weight,
                opaque_copy,
            } => ScanlineMath::CoverageSeparate {
                weight,
                opaque_copy,
            },
            StretchMath::ConstantSeparate { weights } => ScanlineMath::ConstantSeparate { weights },
        };
        return Ok(Some(FixedScanlines::rectangle(
            plan.destination,
            plan.source_q16,
            plan.source_step_q16,
            math,
            plan.corner_rgb_key,
            phase.color,
        )?));
    }
    // Every nonstretch helper applies CPicture::Clip (CharaClip then screen).
    let Some((source, at)) =
        clip.source_clip([0, 0, picture[0] as i32, picture[1] as i32], point)?
    else {
        return Ok(None);
    };
    let Some((source, at)) = crysirror_core::region_map::clip_region_copy(
        Some(picture),
        canvas,
        source.map(i64::from),
        at.map(i64::from),
        None,
    )?
    else {
        return Ok(None);
    };
    let weight = trans.wrapping_mul(256) / 100;
    let (math, key) = if phase.color.is_some() {
        if transparent && has_mask {
            (
                ScanlineMath::CoverageSummed {
                    weight: weight as u32,
                },
                false,
            )
        } else {
            (
                ScanlineMath::ConstantSummed {
                    weights: [weight as u16, 256i32.wrapping_sub(weight) as u16],
                },
                transparent,
            )
        }
    } else if trans == 100 {
        if transparent && has_mask {
            (
                ScanlineMath::Stored {
                    math: ByteRasterMath::AlphaWeightedSeparateRgb256 { weight: 256 },
                },
                false,
            )
        } else {
            (
                ScanlineMath::Stored {
                    math: ByteRasterMath::CopyRgb,
                },
                transparent,
            )
        }
    } else {
        // TransLucentBlt3 requires a real mask and ignores para2's flag.
        if !has_mask || trans <= 0 {
            return Ok(None);
        }
        (
            ScanlineMath::Stored {
                math: ByteRasterMath::MaskPiecewiseWrapping256 {
                    weight: weight as u16,
                },
            },
            false,
        )
    };
    Ok(Some(FixedScanlines::rectangle(
        [
            at[0] as i32,
            at[1] as i32,
            source[2] as i32,
            source[3] as i32,
        ],
        [source[0] * 65536, source[1] * 65536],
        [65536, 65536],
        math,
        key,
        phase.color,
    )?))
}

pub(crate) fn image_input(
    frame: &mut crate::native_picture_effect::PictureEffectFrame,
    base: crysirror_core::image_processing::ImageInput,
    random: &mut PatternRng,
) -> Result<Option<crysirror_core::image_processing::ImageInput>> {
    use crate::native_picture_effect::PictureEffectKind;
    use crysirror_core::image_processing::ImageInput;
    use crysirror_image::{byte_raster::ByteRaster, effects::ImageEffect};
    let PictureEffectKind::SpecialMove { parameters } = &mut frame.state.kind else {
        anyhow::bail!("NNN SPECIALMOVE Print received another effect");
    };
    let phase = phase(parameters, frame.state.count, random)?;
    store_print_clocks(parameters, &phase);
    // These Print words and curve draws survive even if no assigned resource
    // can paint. The caller stores the candidate state before its early return.
    if !frame.geometry_assigned {
        return Ok(None);
    }
    let Some(source) = &frame.source else {
        return Ok(None);
    };
    let clip = CharaClip::from_bounds(frame.source_clip.unwrap_or([
        0,
        0,
        source.size()[0] as i32,
        source.size()[1] as i32,
    ]));
    let Some(spec) = draw_plan(
        parameters,
        &phase,
        source.size(),
        frame.canvas,
        frame.has_mask,
        clip,
        [
            frame.rectangles.destination[0],
            frame.rectangles.destination[1],
        ],
    )?
    else {
        return Ok(None);
    };
    Ok(Some(ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(base),
            source: Box::new(ImageInput::Resource(source.clone())),
        }),
        effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
            spec: ByteRaster::FixedScanlines { spec },
        }])?
        .into(),
    }))
}

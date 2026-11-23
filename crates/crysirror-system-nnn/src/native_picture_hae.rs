// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! HAE108 per-object show/hide clocks, atlas animation and ordered native RNG.
//! Shared sprite composition owns all clipping and byte arithmetic.
use crate::native_picture_effect::PictureEffectFrame;
use anyhow::{Context, Result, ensure};
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageInput};
use crysirror_image::temporal_pattern::PatternRng;
pub(crate) fn parameters(values: &[i32]) -> [i32; 144] {
    let defaults = [999999, 32, 32, 1, 0, 1, 1, 1, 30, 0, 10, 0, 0, 0, 0, 0];
    let mut p = [0; 144];
    // Omitted11..15 were stack garbage. Unused work6/7 is also defined0;
    // active positions/clocks are replaced by InitKe, including raw restore.
    for i in 0..16 {
        p[i] = values.get(i).copied().unwrap_or(defaults[i]);
    }
    p[0] = 999999;
    p[3] = p[3].min(16);
    p
}
pub(crate) fn validate(p: &[i32; 144]) -> Result<()> {
    ensure!(
        p[3] <= 16 && (0..=16384).contains(&p[1]) && (0..=16384).contains(&p[2]),
        "NNN HAE count/cell bounds"
    );
    Ok(())
}
fn draw(rng: &mut PatternRng, divisor: i32) -> Result<i32> {
    (rng.next_draw()? as i32)
        .checked_rem(divisor)
        .context("NNN HAE zero native random divisor")
}
fn stick(p: &[i32; 144], direction: i32, canvas: [u32; 2], mut at: [i32; 2]) -> [i32; 2] {
    match direction {
        1 => at[1] = p[2] / 2,
        2 => at[0] = (canvas[0] as i32).wrapping_sub(p[1] / 2),
        3 => at[1] = (canvas[1] as i32).wrapping_sub(p[2] / 2),
        4 => at[0] = p[1] / 2,
        _ => {}
    }
    at
}
fn initialize_one(
    p: &mut [i32; 144],
    n: usize,
    start: bool,
    ke: bool,
    canvas: [u32; 2],
    rng: &mut PatternRng,
) -> Result<()> {
    let k = 16 + n * 8;
    let r = draw(rng, p[8].wrapping_add(p[9]).max(1))?;
    p[k + 4] = 0;
    p[k + 5] = 0;
    p[k] = if r < p[8] || !start {
        (p[8] / 2).wrapping_add(draw(rng, p[8].max(1))?).max(1)
    } else {
        0
    };
    p[k + 1] = (p[9] / 2).wrapping_add(draw(rng, p[9].max(1))?).max(1);
    let x = draw(rng, canvas[0] as i32)?;
    let y = draw(rng, canvas[1] as i32)?;
    let at = if ke {
        if start {
            let direction = ke_direction(p[4], rng)?;
            p[k + 6] = direction;
            stick(p, direction, canvas, [x, y])
        } else {
            [x, y]
        }
    } else {
        stick(p, p[4], canvas, [x, y])
    };
    p[k + 2] = at[0];
    p[k + 3] = at[1];
    Ok(())
}
pub(crate) fn initialize(p: &mut [i32; 144], canvas: [u32; 2], rng: &mut PatternRng) -> Result<()> {
    initialize_mode(p, canvas, rng, false)
}
pub(crate) fn initialize_mode(
    p: &mut [i32; 144],
    canvas: [u32; 2],
    rng: &mut PatternRng,
    ke: bool,
) -> Result<()> {
    validate(p)?;
    for n in 0..p[3].max(0) as usize {
        initialize_one(p, n, true, ke, canvas, rng)?;
    }
    Ok(())
}
pub(crate) fn calcu(
    p: &mut [i32; 144],
    count: &mut i32,
    canvas: [u32; 2],
    rng: &mut PatternRng,
) -> Result<()> {
    calcu_mode(p, count, canvas, rng, false)
}
pub(crate) fn calcu_mode(
    p: &mut [i32; 144],
    count: &mut i32,
    canvas: [u32; 2],
    rng: &mut PatternRng,
    ke: bool,
) -> Result<()> {
    validate(p)?;
    if *count > 10000 {
        *count = 0;
    }
    for n in 0..p[3].max(0) as usize {
        let k = 16 + n * 8;
        if p[k] > 0 {
            p[k] = p[k].wrapping_sub(1);
        } else {
            p[k + 1] = p[k + 1].wrapping_sub(1);
            if p[k + 1] <= 0 {
                initialize_one(p, n, false, ke, canvas, rng)?;
            }
        }
        if p[6] > 1 {
            let mut c = p[k + 5].wrapping_add(1);
            if c >= p[7] {
                c = 0;
                p[k + 4] = p[k + 4].wrapping_add(1);
                if p[k + 4] >= p[6] {
                    p[k + 4] = 0;
                }
            }
            p[k + 5] = c;
        }
        // Even hidden objects move and consume BOTH draws. A range1 draw is
        // retained, and signed remainder follows C++ for a negative jitter.
        let divisor = p[10].wrapping_mul(2).wrapping_add(1);
        let x = p[k + 2]
            .wrapping_add(draw(rng, divisor)?)
            .wrapping_sub(p[10])
            .clamp(0, canvas[0] as i32 - 1);
        let y = p[k + 3]
            .wrapping_add(draw(rng, divisor)?)
            .wrapping_sub(p[10])
            .clamp(0, canvas[1] as i32 - 1);
        let at = if ke {
            if p[4] != 0 {
                let mut direction = p[k + 6];
                if p[11] > 0 && draw(rng, 10000)? < p[11] {
                    direction = ke_direction(p[4], rng)?;
                    p[k + 6] = direction;
                }
                stick(p, direction, canvas, [x, y])
            } else {
                [x, y]
            }
        } else {
            stick(p, p[4], canvas, [x, y])
        };
        p[k + 2] = at[0];
        p[k + 3] = at[1];
    }
    Ok(())
}
pub(crate) fn image_input(
    frame: &PictureEffectFrame,
    p: &[i32; 144],
    base: ImageInput,
) -> Result<Option<ImageInput>> {
    validate(p)?;
    let Some(bank) = frame.bank.picture(frame.slot) else {
        return Ok(None);
    };
    let picture = &bank.picture;
    let source = if picture.has_mask {
        picture.raw.clone()
    } else {
        picture.keyed.clone()
    };
    let mut layers = Vec::new();
    // Zero native parts was a divide-by-zero; define the first atlas column.
    let parts = if p[5] == 0 { 1 } else { p[5] };
    for n in 0..p[3].max(0) as usize {
        let k = 16 + n * 8;
        if p[k] <= 0 {
            continue;
        }
        let sx = (n as i32 % parts).wrapping_mul(p[1]);
        let sy = p[k + 4].wrapping_mul(p[2]);
        let at = [
            p[k + 2].wrapping_sub(p[1] / 2),
            p[k + 3].wrapping_sub(p[2] / 2),
        ];
        let Some((region, destination)) =
            crysirror_image::coverage_bounds::intersect_source_window(
                [sx, sy, p[1], p[2]],
                at,
                picture.source_clip,
            )?
        else {
            continue;
        };
        layers.push(ImageCompositeLayer {
            input: Box::new(ImageInput::Resource(source.clone())),
            source: region,
            destination,
            clip: None,
            opacity: 255,
            math: ImageCompositeMath::MaskSeparate256,
        });
    }
    if layers.is_empty() {
        return Ok(None);
    }
    Ok(Some(ImageInput::Composite {
        base: Box::new(base),
        layers,
    }))
}
// CountIsMax FALSE; CheckNeedPicPrint FALSE; Init/BeginPrint/End empty.
// InitParameter reruns the16-word setter and InitKe, consuming its RNG anew
// in the original all-slot initializer order, unlike FISH's empty restore hook.

fn ke_direction(bits: i32, rng: &mut PatternRng) -> Result<i32> {
    match bits {
        0 => return Ok(0),
        1 => return Ok(1),
        2 => return Ok(2),
        4 => return Ok(3),
        8 => return Ok(4),
        _ => {}
    }
    let choices: Vec<i32> = (0..4)
        .filter(|i| bits & (1 << i) != 0)
        .map(|i| i + 1)
        .collect();
    if choices.is_empty() {
        return Ok(0);
    }
    // Additional high bits make e.g.17 enter this general branch and consume
    // rand%1; do not optimize it to the literal1 special case.
    Ok(choices[draw(rng, choices.len() as i32)? as usize])
}

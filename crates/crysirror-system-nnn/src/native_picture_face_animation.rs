// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! MEPACHIKUCHIPAKU98 native blink/lip timing, random cycle and atlas policy.
//! Shared composition draws its selected bank cell; speaking is owner state,
//! not a private decoder, wall-clock query or substitute audio device.
use crate::native_picture_effect::PictureEffectFrame;
use anyhow::{Result, ensure};
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageInput};
use crysirror_image::temporal_pattern::PatternRng;
pub(crate) fn parameters(values: &[i32]) -> [i32; 15] {
    let defaults = [20, 32, 32, 2, -1, 0, 0, 1, 40, 80, 4, 80, 0, 0, 0];
    let mut p = std::array::from_fn(|i| {
        if i < 13 {
            values.get(i).copied().unwrap_or(defaults[i])
        } else {
            0
        }
    });
    p[13] = 0;
    p
}
pub(crate) fn validate(p: &[i32; 15]) -> Result<()> {
    ensure!(
        (0..=16384).contains(&p[1]) && (0..=16384).contains(&p[2]),
        "NNN face-animation cell bounds"
    );
    Ok(())
}
pub(crate) fn restart(p: &mut [i32; 15], rng: &mut PatternRng) -> Result<()> {
    p[13] = 0;
    let (average, random) = if p[6] & 1 != 0 {
        (p[10], p[11])
    } else {
        (p[8], p[9])
    };
    let divisor = random.wrapping_mul(2).wrapping_mul(average) / 100;
    let divisor = divisor.max(1);
    let offset = (rng.next_draw()? % divisor as u32) as i32;
    p[14] = offset
        .wrapping_sub(divisor / 2)
        .wrapping_add(average)
        .max(1);
    Ok(())
}
pub(crate) fn calcu(p: &mut [i32; 15], count: &mut i32, rng: &mut PatternRng) -> Result<bool> {
    validate(p)?;
    *count = 0;
    p[13] = p[13].wrapping_add(1);
    let mut c = p[13];
    let limit = p[14];
    if c < limit {
        return Ok(false);
    }
    let mut duration = p[3];
    if p[5] != 0 {
        duration = duration.wrapping_add(p[3].wrapping_sub(2));
    }
    duration = duration.wrapping_mul(p[7]);
    c = c.wrapping_sub(limit);
    let mut stop = false;
    if p[12] != 0
        && c == p[3].wrapping_sub(1).wrapping_mul(p[7])
        && ((rng.next_draw()? % 100) as i32) < p[12]
    {
        if p[5] != 0 {
            let delta = p[7].wrapping_mul(2);
            p[13] = p[13].wrapping_add(delta);
            c = c.wrapping_add(delta);
        } else {
            stop = true;
        }
    }
    if c >= duration || stop {
        restart(p, rng)?;
        return Ok(true);
    }
    Ok(false)
}
pub(crate) fn image_input(
    frame: &PictureEffectFrame,
    p: &[i32; 15],
    speaking: bool,
    base: ImageInput,
) -> Result<Option<ImageInput>> {
    validate(p)?;
    let mut cell = p[13];
    if cell < p[14] || (p[6] & 1 != 0 && !speaking) {
        return Ok(None);
    }
    cell = cell.wrapping_sub(p[14]);
    // Speed0 could reach undefined native division through externally hydrated
    // work. Define its divisor1; ordinary Calcu still uses the original speed.
    cell /= if p[7] == 0 { 1 } else { p[7] };
    if p[5] != 0 && cell >= p[3] {
        cell = cell.wrapping_sub(p[3]);
        cell = p[3].wrapping_sub(1).wrapping_sub(cell);
    }
    cell = cell.max(0);
    let slot = (if p[4] == -1 { frame.slot } else { p[4] }).clamp(0, frame.bank_limit - 1);
    let Some(bank) = frame.bank.picture(slot) else {
        return Ok(None);
    };
    let picture = &bank.picture;
    let point = frame
        .bank
        .rectangles(slot)
        .map(|r| [r.destination[0], r.destination[1]])
        .unwrap_or([0, 0]);
    let Some((source, destination)) = crysirror_image::coverage_bounds::intersect_source_window(
        [p[1].wrapping_mul(cell), 0, p[1], p[2]],
        point,
        picture.source_clip,
    )?
    else {
        return Ok(None);
    };
    let masked = p[6] & 2 != 0;
    let input = if !masked {
        picture.opaque.clone()
    } else if picture.has_mask {
        picture.raw.clone()
    } else {
        picture.keyed.clone()
    };
    Ok(Some(ImageInput::Composite {
        base: Box::new(base),
        layers: vec![ImageCompositeLayer {
            input: Box::new(ImageInput::Resource(input)),
            source,
            destination,
            clip: None,
            opacity: 255,
            math: if masked {
                ImageCompositeMath::MaskSeparate256
            } else {
                ImageCompositeMath::CopyRgb
            },
        }],
    }))
}
// SetParam countMax9999 and one restart draw; Calcu resets EFFECT.count0,
// updates work and may request full repaint. Our full ordered frame composition
// already covers that dirty-area request. Empty inherited InitParameter keeps
// all15 saved words/clocks; CheckAllPrint/CheckNeedPicPrint bothFALSE.

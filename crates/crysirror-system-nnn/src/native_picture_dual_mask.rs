// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! DUALMASK121 native curves and RGB packed fade policy. Two source masks are
//! tested/clipped before phase evaluation. Shared image workers own all pixels.
//! Win64 omits the global weight/fade and uses another denominator; it is not
//! substituted for the original software32 profile.
use anyhow::Result;
use crysirror_image::temporal_pattern::PatternRng;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DualMaskPhase {
    pub weight: u16,
    pub source_color: Option<([u8; 3], [u16; 3])>,
}
pub(crate) fn parameters(p: &[i32]) -> [i32; 12] {
    let mut out = [0; 12];
    for i in 0..12 {
        out[i] = p.get(i).copied().unwrap_or(match i {
            0 => 20,
            1 => 13,
            2 => 100,
            3 => out[2],
            9..=11 => 255,
            _ => 0,
        });
    }
    out
}
pub(crate) fn validate(p: &[i32; 12], frames: i32) -> Result<()> {
    crate::native_curve::validate(frames, p[5], p[4])?;
    if p[6] != 0 {
        crate::native_curve::validate(frames, p[8], p[7])?;
    }
    Ok(())
}
pub(crate) fn phase(
    p: [i32; 12],
    count: i32,
    frames: i32,
    random: &mut PatternRng,
) -> Result<DualMaskPhase> {
    let percent = crate::native_curve::value_with_random(
        count,
        frames,
        p[3].wrapping_mul(100),
        p[2].wrapping_mul(100),
        p[5],
        p[4],
        random,
    )?;
    let weight = (percent.wrapping_mul(256) / 10000) as u16;
    let source_color = if p[6] != 0 {
        let fade =
            crate::native_curve::value_with_random(count, frames, 0, 255, p[8], p[7], random)?;
        let packed = fade.wrapping_shl(16) | fade.wrapping_shl(8) | fade;
        let multiply = [(packed >> 16) as u8, (packed >> 8) as u8, packed as u8];
        let colors = [p[9], p[10], p[11]].map(|v| 255i32.wrapping_sub(fade).wrapping_mul(v) / 255);
        // Original signed negative shifts are undefined C++. Define their
        // packed-word replacement through explicit wrapping unsigned shifts;
        // ordinary signed/positive lane overlap is retained without memory UB.
        let add = ((i64::from(colors[0]) as u64).wrapping_shl(32))
            | ((i64::from(colors[1]) as u64).wrapping_shl(16))
            | (i64::from(colors[2]) as u64);
        Some((
            multiply,
            [(add >> 32) as u16, (add >> 16) as u16, add as u16],
        ))
    } else {
        None
    };
    Ok(DualMaskPhase {
        weight,
        source_color,
    })
}

pub(crate) fn image_input(
    frame: &crate::native_picture_effect::PictureEffectFrame,
    p: [i32; 12],
    base: crysirror_core::image_processing::ImageInput,
    random: &mut PatternRng,
) -> Result<Option<crysirror_core::image_processing::ImageInput>> {
    use crysirror_core::image_processing::ImageInput;
    use crysirror_image::{effects::ImageEffect, packed_dual_mask::PackedDualMask};
    if !frame.has_mask {
        return Ok(None);
    }
    let (Some(source), Some(mask)) = (&frame.source, &frame.second) else {
        return Ok(None);
    };
    let point = [
        frame.rectangles.destination[0],
        frame.rectangles.destination[1],
    ];
    let size = source.size();
    let mask_size = mask.size();
    let bounds=frame.source_clip.unwrap_or([0,0,size[0] as i32,size[1] as i32]);
    let Some((rect,point))=crysirror_image::coverage_bounds::intersect_source_window([0,0,size[0] as i32,size[1] as i32],point,bounds)? else{return Ok(None);};
    let mask_bounds=frame.second_clip.unwrap_or([0,0,mask_size[0] as i32,mask_size[1] as i32]);
    let mask_left=mask_bounds[0].max(0);
    let mask_top=mask_bounds[1].max(0);
    let mask_right=mask_bounds[2].min(mask_size[0] as i32);
    let mask_bottom=mask_bounds[3].min(mask_size[1] as i32);
    if mask_left>=mask_right||mask_top>=mask_bottom{return Ok(None);}
    let Some((source_rect, at)) = crysirror_core::region_map::clip_region_copy(
        Some(size), frame.canvas, rect.map(i64::from),point.map(i64::from),
        Some([i64::from(frame.second_destination[0])+i64::from(mask_left),
              i64::from(frame.second_destination[1])+i64::from(mask_top),
              i64::from(mask_right-mask_left),i64::from(mask_bottom-mask_top)]),
    )? else {return Ok(None);};
    let phase = phase(p, frame.state.count, frame.state.frames, random)?;
    let spec = PackedDualMask {
        source: source_rect.map(|v| v as i32),
        destination: at.map(|v| v as i32),
        mask: [
            (at[0] - i64::from(frame.second_destination[0])) as i32,
            (at[1] - i64::from(frame.second_destination[1])) as i32,
        ],
        weight: phase.weight,
        source_color: phase.source_color,
    };
    spec.validate()?;
    Ok(Some(ImageInput::Process {
        input: Box::new(ImageInput::Ternary {
            destination: Box::new(base),
            first: Box::new(ImageInput::Resource(source.clone())),
            second: Box::new(ImageInput::Resource(mask.clone())),
        }),
        effects: serde_json::to_vec(&[ImageEffect::PackedDualMask { spec }])?.into(),
    }))
}

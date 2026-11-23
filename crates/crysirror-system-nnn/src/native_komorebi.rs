// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! KOMOREBI78's three curve evaluations, packed offset parameters and two-mask
//! overlap geometry. Full546-line class/header and CPicture clipping read.
//! Shared raster must consume linear mask strides explicitly: the source uses
//! EFFECT destination WIDTH as mask pitch, not CPicture's physical width.
use crate::native_picture_clip::CharaClip;
use anyhow::{Result, ensure};
use crysirror_image::temporal_pattern::PatternRng;
pub(crate) fn parameters(values: &[i32]) -> [i32; 16] {
    let mut p = [0; 16];
    for i in 0..16 {
        p[i] = values.get(i).copied().unwrap_or(match i {
            0 => 20,
            2 => 128,
            3 | 4 => p[i - 1],
            5 => p[2],
            6 => p[3],
            7 => p[4],
            8 => -1,
            _ => 0,
        });
    }
    p[1] = p[1].clamp(0, 31);
    p[8] = p[8].clamp(-1, 31);
    p
}
pub(crate) fn validate(p: &[i32; 16]) -> Result<()> {
    crate::native_curve::validate(p[0], p[10], p[9])
}
#[derive(Clone, Copy)]
pub(crate) struct ColorOffset {
    pub add: [u8; 3],
    pub subtract: [u8; 3],
}
pub(crate) fn color(
    p: &[i32; 16],
    count: i32,
    frames: i32,
    rng: &mut PatternRng,
) -> Result<ColorOffset> {
    let (count, frames) = if frames < 1 {
        (0, 1)
    } else {
        (count.clamp(0, frames), frames)
    };
    let mut add = 0i32;
    let mut sub = 0i32;
    for channel in 0..3 {
        let d = crate::native_curve::value_with_random(
            count,
            frames,
            p[5 + channel],
            p[2 + channel],
            p[10],
            p[9],
            rng,
        )?;
        let shift = (2 - channel) * 8;
        if d >= 0 {
            add |= d.wrapping_shl(shift as u32);
        } else {
            sub |= d.wrapping_neg().wrapping_shl(shift as u32);
        }
    }
    let rgb = |v: i32| [(v >> 16) as u8, (v >> 8) as u8, v as u8];
    Ok(ColorOffset {
        add: rgb(add),
        subtract: rgb(sub),
    })
}
#[derive(Clone, Copy)]
pub(crate) struct Picture {
    pub physical: [u32; 2],
    pub clip: CharaClip,
    pub mask_exists: bool,
    pub indexed: bool,
    pub has_rgb_buffer: bool,
}
#[derive(Clone, Copy)]
pub(crate) struct Geometry {
    pub point: [i32; 2],
    pub size: [i32; 2],
}
pub(crate) struct MaskOverlap {
    pub destination: [i32; 4],
    pub origins: [[i32; 2]; 2],
    pub strides: [i32; 2],
    pub offset: ColorOffset,
}
fn clip(
    picture: Picture,
    geometry: Geometry,
    screen: [u32; 2],
) -> Result<Option<([i32; 4], [i32; 2])>> {
    let b = picture.clip.bounds();
    let width = b[2].wrapping_sub(b[0]);
    let height = b[3].wrapping_sub(b[1]);
    if width < 1 || height < 1 {
        return Ok(None);
    }
    let target = [
        geometry.point[0].wrapping_add(b[0]),
        geometry.point[1].wrapping_add(b[1]),
    ];
    let Some((source, at)) = picture
        .clip
        .source_clip([b[0], b[1], width, height], target)?
    else {
        return Ok(None);
    };
    let mut point = at.map(i64::from);
    let mut origin = [i64::from(source[0]), i64::from(source[1])];
    let mut extent = [i64::from(source[2]), i64::from(source[3])];
    if !crate::native_picture_pair_geometry::clip_etc(
        &mut point,
        &mut origin,
        &mut extent,
        picture.physical,
        [0, 0, i64::from(screen[0]), i64::from(screen[1])],
    ) {
        return Ok(None);
    }
    Ok(Some((
        [
            origin[0] as i32,
            origin[1] as i32,
            extent[0] as i32,
            extent[1] as i32,
        ],
        point.map(|v| v as i32),
    )))
}
pub(crate) fn overlap(
    offset: ColorOffset,
    pictures: [Option<Picture>; 2],
    geometry: [Geometry; 2],
    canvas: [u32; 2],
) -> Result<Option<MaskOverlap>> {
    ensure!(
        canvas.iter().all(|n| (1..=16384).contains(n)),
        "NNN dual-mask canvas"
    );
    let [Some(a), Some(b)] = pictures else {
        return Ok(None);
    };
    if [a, b]
        .iter()
        .any(|p| p.indexed || !p.mask_exists || !p.has_rgb_buffer)
    {
        return Ok(None);
    }
    let Some((ra, da)) = clip(a, geometry[0], canvas)? else {
        return Ok(None);
    };
    let Some((rb, db)) = clip(b, geometry[1], canvas)? else {
        return Ok(None);
    };
    let x = da[0].max(db[0]);
    let y = da[1].max(db[1]);
    let width = (da[0] + ra[2]).min(db[0] + rb[2]) - x;
    let height = (da[1] + ra[3]).min(db[1] + rb[3]) - y;
    if width < 1 || height < 1 {
        return Ok(None);
    }
    Ok(Some(MaskOverlap {
        destination: [x, y, width, height],
        origins: [
            [ra[0] + x - da[0], ra[1] + y - da[1]],
            [rb[0] + x - db[0], rb[1] + y - db[1]],
        ],
        strides: [geometry[0].size[0], geometry[1].size[0]],
        offset,
    }))
}

pub(crate) fn image_input(
    frame: &crate::native_picture_effect::PictureEffectFrame,
    p: &[i32; 16],
    base: crysirror_core::image_processing::ImageInput,
    rng: &mut PatternRng,
) -> Result<Option<crysirror_core::image_processing::ImageInput>> {
    use crysirror_core::image_processing::ImageInput;
    use crysirror_image::{effects::ImageEffect, linear_mask_offset::LinearMaskOffset};
    let offset = color(p, frame.state.count, frame.state.frames, rng)?;
    let first_geometry = if p[8] == -1 {
        frame.slot
    } else {
        p[8].clamp(0, frame.bank_limit - 1)
    };
    let second_slot = p[1].clamp(0, frame.bank_limit - 1);
    let a = frame.bank.picture(frame.slot);
    let b = frame.bank.picture(second_slot);
    let metadata = |picture: Option<&crate::native_picture_bank::BankPicture>| {
        picture.map(|p| Picture {
            physical: p.picture.raw.size(),
            clip: CharaClip::from_bounds(p.picture.source_clip),
            mask_exists: p.picture.has_mask,
            indexed: false,
            has_rgb_buffer: true,
        })
    };
    let geometry = |slot: i32| {
        let fallback = crate::native_geometry::NativeRects::natural(frame.canvas);
        let r = frame.bank.rectangles(slot).unwrap_or(&fallback);
        Geometry {
            point: [r.destination[0], r.destination[1]],
            size: [r.destination[2], r.destination[3]],
        }
    };
    let Some(plan) = overlap(
        offset,
        [metadata(a), metadata(b)],
        [geometry(first_geometry), geometry(second_slot)],
        frame.canvas,
    )?
    else {
        return Ok(None);
    };
    let masks = [a.unwrap(), b.unwrap()].map(|p| {
        p.picture
            .mask_storage
            .as_ref()
            .map(|m| m.buffer.clone())
            .ok_or_else(|| anyhow::anyhow!("NNN KOMOREBI mask allocation absent"))
    });
    let [a, b] = masks;
    let a = a?;
    let b = b?;
    let spec = LinearMaskOffset {
        destination: plan.destination,
        origins: plan.origins,
        strides: plan.strides,
        add: plan.offset.add,
        subtract: plan.offset.subtract,
    };
    spec.validate()?;
    Ok(Some(ImageInput::Process {
        input: Box::new(ImageInput::Ternary {
            destination: Box::new(base),
            first: Box::new(ImageInput::Resource(a)),
            second: Box::new(ImageInput::Resource(b)),
        }),
        effects: serde_json::to_vec(&[ImageEffect::LinearMaskOffset { spec }])?.into(),
    }))
}
// Native source1 is ALWAYS GetPicture(current layer), but its POSITION and
// STRIDE come from para8 (or current layer if -1). Source2/geometry2 use para1.
// GetOtherLayerPic results are read and discarded; assigned flags do not gate.
// All3 curves precede every resource/depth/mask guard. Raw InitParameter is
// inherited empty; omitted11..15 copied undefined words are initialized0.
// Native shared kernel must skip if either mask0, treat255 as identity, and
// otherwise combine both partial masks via floor(a*b/256). OffsetRGB is then
// scaled by that mask/256 (255 bypasses), applied with saturated WORD add then
// saturated WORD subtract, finally byte saturation. No source RGB is read by
// the loop despite both RGB pointers being checked before entry. Alpha stays.
// Explicit linear mask reads must bound formerly invalid stride/address
// arithmetic; ordinary 2D source sampling would silently change valid strides.
// CountIsMaxFALSE, Calcu/Init/BeginPrint empty, NeedPicPrintTRUE.

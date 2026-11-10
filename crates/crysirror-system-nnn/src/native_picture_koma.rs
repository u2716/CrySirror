// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! KOMAANIME63, effectKomaAnime.cpp: native cell selection, interpolation and
//! count-loop policy. Shared image jobs own clipping and packed-byte drawing.
use anyhow::{Result, ensure};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KomaBlend {
    Opaque,
    Mask,
    /// TransLucentBlt3 stores the low sixteen bits of each MMX multiplier.
    /// Values above100 percent are not silently clamped to ordinary opacity.
    Weighted {
        weight: u16,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KomaDraw {
    pub source: [i32; 4],
    pub destination: [i32; 2],
    pub blend: KomaBlend,
}
pub(crate) fn parameters(p: &[i32]) -> [i32; 16] {
    let mut out = [0; 16];
    for i in 0..16 {
        out[i] = p.get(i).copied().unwrap_or(match i {
            0 => 20,
            1 | 2 => 64,
            3 => 1,
            6 => out[4],
            7 => out[5],
            10 | 11 => 100,
            _ => 0,
        });
    }
    // SetParam copied omitted para15 from uninitialised stack storage.
    // Its deterministic default is zero; supplied/saved words still survive.
    out
}
pub(crate) fn calcu(p: &[i32; 16], count: &mut i32) {
    if p[14] > 0 {
        *count %= p[14];
    }
}
pub(crate) fn draw_plan(
    p: &[i32; 16],
    count: i32,
    frames: i32,
    picture_size: [u32; 2],
    assigned: bool,
    has_mask: bool,
) -> Result<Option<KomaDraw>> {
    if !assigned || count >= frames {
        return Ok(None);
    }
    ensure!(
        picture_size.iter().all(|n| (1..=16384).contains(n)),
        "NNN KOMAANIME picture dimensions"
    );
    let mut c = count;
    let mut divisor = frames.wrapping_sub(1);
    if divisor < 1 {
        divisor = 1;
        c = 0;
    }
    // The final visible sample intentionally stops at countMax-2.
    if c >= divisor {
        c = divisor - 1;
    }
    let interpolate = |start: i32, end: i32| {
        start.wrapping_add(end.wrapping_sub(start).wrapping_mul(c) / divisor)
    };
    let destination = [interpolate(p[6], p[4]), interpolate(p[7], p[5])];
    let [sx, sy] = [p[1], p[2]];
    if sx < 1 || sy < 1 {
        return Ok(None);
    }
    let columns = picture_size[0] as i32 / sx;
    let rows = picture_size[1] as i32 / sy;
    if columns < 1 || rows < 1 {
        return Ok(None);
    }
    let mut cell = c / p[3].max(1);
    if p[9] > 0 {
        cell %= p[9];
    }
    let source = [
        (cell % columns).wrapping_mul(sx),
        (cell / columns).wrapping_mul(sy),
        sx,
        sy,
    ];
    let blend = match p[8] {
        0 => KomaBlend::Opaque,
        2 => {
            let percent = interpolate(p[11], p[10]);
            if percent == 100 {
                if has_mask {
                    KomaBlend::Mask
                } else {
                    // AntiAliasBlt2 dereferences a null mask in this original
                    // branch. Define the missing mask as full coverage, rather
                    // than reading uninitialised memory or inventing a key.
                    KomaBlend::Opaque
                }
            } else {
                // CPicture::TransLucentBlt3 returns before clipping when the
                // picture has no mask; it never substitutes RGB-key coverage.
                if !has_mask || percent <= 0 {
                    return Ok(None);
                }
                KomaBlend::Weighted {
                    weight: (percent.wrapping_mul(256) / 100) as u16,
                }
            }
        }
        _ => KomaBlend::Mask,
    };
    Ok(Some(KomaDraw {
        source,
        destination,
        blend,
    }))
}

pub(crate) fn image_input(
    frame: &crate::native_picture_effect::PictureEffectFrame,
    p: [i32; 16],
    base: crysirror_core::image_processing::ImageInput,
) -> Result<Option<crysirror_core::image_processing::ImageInput>> {
    use crysirror_core::image_processing::ImageInput;
    use crysirror_image::{
        byte_raster::{ByteRaster, ByteRasterMath},
        effects::ImageEffect,
        row_blit::RowBlit,
    };
    let Some(source) = &frame.source else {
        return Ok(None);
    };
    let Some(plan) = draw_plan(
        &p,
        frame.state.count,
        frame.state.frames,
        source.size(),
        frame.geometry_assigned,
        frame.has_mask,
    )?
    else {
        return Ok(None);
    };
    let math = match plan.blend {
        KomaBlend::Opaque => ByteRasterMath::CopyRgb,
        KomaBlend::Mask => ByteRasterMath::AlphaWeightedSeparateRgb256 { weight: 256 },
        KomaBlend::Weighted { weight } => ByteRasterMath::MaskPiecewiseWrapping256 { weight },
    };
    let (source_rect,destination)=if let Some(bounds)=frame.source_clip {
        let Some(mapping)=crysirror_image::coverage_bounds::intersect_source_window(plan.source,plan.destination,bounds)? else{return Ok(None);};mapping
    }else{(plan.source,plan.destination)};
    let spec = RowBlit {
        source: source_rect,
        destination,
        row_offsets: vec![0; source_rect[3] as usize],
        math,
        row_math: Vec::new(),
    };
    spec.validate()?;
    Ok(Some(ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(base),
            source: Box::new(ImageInput::Resource(source.clone())),
        }),
        effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
            spec: ByteRaster::RowBlit { spec },
        }])?
        .into(),
    }))
}

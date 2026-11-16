// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! BOSSDEATH48: native curve order, scanline jitter and bottom wipe geometry.
//! The shared RowBlit job owns mask/key handling and all stored-byte writes.
use anyhow::{Context, Result, ensure};
use crysirror_core::image_processing::{ImageInput, ImageResource};
use crysirror_image::{
    byte_raster::{ByteRaster, ByteRasterMath},
    effects::ImageEffect,
    row_blit::RowBlit,
    temporal_pattern::PatternRng,
};
use std::sync::Arc;

pub(crate) fn parameters(values: &[i32]) -> [i32; 11] {
    let mut p = [0; 11];
    for i in 0..11 {
        p[i] = values.get(i).copied().unwrap_or(match i {
            0 => 20,
            2 => 8,
            3 => p[2],
            7 => 50,
            8 => p[7],
            _ => 0,
        });
    }
    if p[1] >= p[0] {
        p[1] = p[0].wrapping_sub(1);
    }
    p
}
pub(crate) fn validate(p: &[i32; 11], frames: i32) -> Result<()> {
    crate::native_curve::validate(frames, p[5], p[4])?;
    crate::native_curve::validate(frames, p[10], p[9])
}
pub(crate) struct Picture {
    pub raw: Arc<dyn ImageResource>,
    pub keyed: Arc<dyn ImageResource>,
    pub has_mask: bool,
    pub source_clip: [i32; 4],
}
pub(crate) fn image_input(
    p: &[i32; 11],
    count: i32,
    frames: i32,
    canvas: [u32; 2],
    picture_assigned: bool,
    picture: Option<&Picture>,
    destination: [i32; 2],
    base: ImageInput,
    random: &mut PatternRng,
) -> Result<Option<ImageInput>> {
    // Both curves execute before pic==-1, NULL CPicture and yStart exits,
    // including the transparency curve when the ordinary Blt branch uses none.
    let jitter =
        crate::native_curve::value_with_random(count, frames, p[3], p[2], p[5], p[4], random)?;
    let percent =
        crate::native_curve::value_with_random(count, frames, p[8], p[7], p[10], p[9], random)?;
    if !picture_assigned {
        return Ok(None);
    }
    ensure!(
        canvas.iter().all(|n| (1..=16384).contains(n)),
        "NNN BOSSDEATH canvas dimensions"
    );
    let height = canvas[1] as i32;
    let start = if count >= p[1] {
        let duration = frames.wrapping_sub(p[1]).max(1);
        let elapsed = count.wrapping_sub(p[1]).clamp(0, duration);
        elapsed.wrapping_mul(height) / duration
    } else {
        0
    };
    if start >= height {
        return Ok(None);
    }
    let rows = height.wrapping_sub(start);
    ensure!((0..=65536).contains(&rows), "NNN BOSSDEATH scanline budget");
    let mut offsets = Vec::with_capacity(rows as usize);
    for _ in 0..rows {
        offsets.push(if jitter > 0 {
            let divisor = jitter.wrapping_mul(2);
            let sample = random.next_draw()? as i32;
            sample
                .checked_rem(divisor)
                .context("NNN BOSSDEATH zero random modulus")?
                .wrapping_sub(jitter)
        } else {
            0
        });
    }
    // An allocated-but-not-loaded CPicture still consumed every jitter draw:
    // loadOkFlag is checked inside Blt, after the per-row random call.
    let Some(picture) = picture else {
        return Ok(None);
    };
    let size = picture.raw.size();
    ensure!(
        size.iter().all(|n| (1..=16384).contains(n)),
        "NNN BOSSDEATH picture dimensions"
    );
    let (source, math) = if p[6] == 0 {
        (
            if picture.has_mask {
                picture.raw.clone()
            } else {
                picture.keyed.clone()
            },
            ByteRasterMath::AlphaWeightedSeparateRgb256 { weight: 256 },
        )
    } else {
        // Every TransLucentBlt3 call requires its native mask. Jitter draws
        // above remain consumed when that helper subsequently returns early.
        if !picture.has_mask || percent <= 0 {
            return Ok(None);
        }
        (
            picture.raw.clone(),
            ByteRasterMath::MaskPiecewiseWrapping256 {
                weight: (percent.wrapping_mul(256) / 100) as u16,
            },
        )
    };
    // The original computes y0=lp->dst.top but never uses it: destination Y is
    // screen-space yStart+j, while source Y is j. Do not add native dst.top.
    let Some((region, at)) = crysirror_image::coverage_bounds::intersect_source_window(
        [0, 0, size[0] as i32, rows],
        [destination[0], start],
        picture.source_clip,
    )?
    else {
        return Ok(None);
    };
    ensure!(
        region[1] >= 0 && region[3] >= 0 && region[1] as i64 + region[3] as i64 <= rows as i64,
        "NNN BOSSDEATH source clipping rows"
    );
    let begin = region[1] as usize;
    let end = begin + region[3] as usize;
    let spec = RowBlit {
        source: region,
        destination: at,
        row_offsets: offsets[begin..end].to_vec(),
        math,
        row_math: Vec::new(),
    };
    spec.validate()?;
    Ok(Some(ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(base),
            source: Box::new(ImageInput::Resource(source)),
        }),
        effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
            spec: ByteRaster::RowBlit { spec },
        }])?
        .into(),
    }))
}

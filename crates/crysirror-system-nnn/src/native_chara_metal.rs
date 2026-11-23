// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CHARAMETAL112 software band position and source/screen geometry.
//! Entire1084-line class/header read. The D3D polygon path is distinct; source
//! colour-ramp generation, saturated RGB addition and mask products belong
//! in a shared spatial-colour image operation, not a VM pixel loop.
use anyhow::{Result, ensure};
use crysirror_image::temporal_pattern::PatternRng;
pub(crate) fn parameters(values: &[i32]) -> [i32; 16] {
    let mut p = [0; 16];
    for i in 0..16 {
        p[i] = values.get(i).copied().unwrap_or(match i {
            0 => 20,
            2 => 64,
            3 => 25,
            6 => 255,
            7 | 8 => p[i - 1],
            _ => 0,
        });
    }
    p
}
pub(crate) fn validate(p: &[i32; 16]) -> Result<()> {
    ensure!(p[2].unsigned_abs() <= 1_000_000, "NNN metallic band budget");
    crate::native_curve::validate(p[0], p[5], p[4])
}
pub(crate) struct MetalPlan {
    pub source: [i32; 4],
    pub destination: [i32; 2],
    pub table_origin: i32,
    pub table_extent: i32,
    /// Table coordinate at the first clipped destination pixel, followed by
    /// column/row increments. The native diagonal phase is screen-dependent.
    pub coordinate: i32,
    pub increments: [i32; 2],
    pub band_length: i32,
    pub gradient_length: i32,
    pub brightness: [i32; 3],
}
pub(crate) fn plan(
    p: &[i32; 16],
    count: i32,
    frames: i32,
    picture_size: [u32; 2],
    point: [i32; 2],
    canvas: [u32; 2],
    indexed: bool,
    rng: &mut PatternRng,
) -> Result<Option<MetalPlan>> {
    validate(p)?;
    // GetPicture always exists at an admitted bank index. Its metadata is1x1
    // before buffer allocation; the RGB/mask checks occur AFTER this curve.
    if indexed {
        return Ok(None);
    }
    ensure!(
        picture_size
            .iter()
            .chain(canvas.iter())
            .all(|n| (1..=16384).contains(n)),
        "NNN metallic extent"
    );
    let direction = p[1];
    let band = p[2];
    let gradient = p[3].wrapping_mul(band) / 100;
    let extent = band.wrapping_add(match direction {
        0 => picture_size[1] as i32,
        1 => picture_size[0] as i32,
        _ => (picture_size[0] as i32).wrapping_add(picture_size[1] as i32),
    });
    let start = crate::native_curve::value_with_random(count, frames, 0, extent, p[5], p[4], rng)?;
    let max_size = (canvas[0] as i32)
        .wrapping_add(canvas[1] as i32)
        .wrapping_add(band);
    // The source clears divSize+maxSize integers in a fixed temp buffer.
    // Reject formerly overflowing/unbounded allocation requests before any
    // worker writes, rather than reproducing memory beyond that buffer.
    ensure!(
        max_size >= 0 && i64::from(max_size) + i64::from(extent) <= 4_194_304,
        "NNN metallic source table allocation domain"
    );
    let Some((destination, source)) = crysirror_image::coverage_bounds::intersect_source_window(
        [
            point[0],
            point[1],
            picture_size[0] as i32,
            picture_size[1] as i32,
        ],
        [0, 0],
        [0, 0, canvas[0] as i32, canvas[1] as i32],
    )?
    else {
        return Ok(None);
    };
    let origin = start.wrapping_sub(band).wrapping_add(match direction {
        0 => point[1],
        1 => point[0],
        _ => point[0].wrapping_add(point[1]),
    });
    let (coordinate, increments) = match direction {
        0 => (destination[1], [0, 1]),
        2 => (destination[0].wrapping_add(canvas[1] as i32), [1, -1]),
        3 => (destination[0], [1, 1]),
        _ => (destination[0], [1, 0]),
    };
    Ok(Some(MetalPlan {
        source: [source[0], source[1], destination[2], destination[3]],
        destination: [destination[0], destination[1]],
        table_origin: origin,
        table_extent: max_size,
        coordinate,
        increments,
        band_length: band,
        gradient_length: gradient,
        brightness: [p[6], p[7], p[8]],
    }))
}

pub(crate) fn image_input(
    frame: &crate::native_picture_effect::PictureEffectFrame,
    p: &[i32; 16],
    base: crysirror_core::image_processing::ImageInput,
    rng: &mut PatternRng,
) -> Result<Option<crysirror_core::image_processing::ImageInput>> {
    use crysirror_core::image_processing::ImageInput;
    use crysirror_image::{
        byte_raster::ByteRaster, effects::ImageEffect, spatial_color_band::SpatialColorBand,
    };
    let picture = frame.bank.picture(frame.slot);
    let size = picture.map(|p| p.picture.raw.size()).unwrap_or([1, 1]);
    let point = [
        frame.rectangles.destination[0],
        frame.rectangles.destination[1],
    ];
    let Some(plan) = plan(
        p,
        frame.state.count,
        frame.state.frames,
        size,
        point,
        frame.canvas,
        false,
        rng,
    )?
    else {
        return Ok(None);
    };
    // GetMaskPic reads the retained allocation independently of maskExists.
    // The view already combines current RGB with that linear mask history.
    let Some(mask) = picture.and_then(|p| p.picture.mask_storage.as_ref()) else {
        return Ok(None);
    };
    let spec = SpatialColorBand {
        source: plan.source,
        destination: plan.destination,
        table_origin: plan.table_origin,
        table_extent: plan.table_extent,
        coordinate: plan.coordinate,
        increments: plan.increments,
        band_length: plan.band_length,
        gradient_length: plan.gradient_length,
        brightness: plan.brightness,
    };
    spec.validate()?;
    Ok(Some(ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(base),
            source: Box::new(ImageInput::Resource(mask.view.clone())),
        }),
        effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
            spec: ByteRaster::SpatialColorBand { spec },
        }])?
        .into(),
    }))
}
// Shared ramp contract:
// - Initialize the full bounded table0; for i in0..bandLength write only
//   if tableOrigin+i lies in0..tableExtent. Outside that range colour stays0.
// - If gradientLength>0, ascending i<gradient wins over the descending test
//   i>=bandLength-gradient. Each channel uses wrapping32-bit product /gradient
//   then clamps0..255. With gradient<=0, PACK the raw signed RGB using OR and
//   byte shifts without channel clamping; high bits can bleed between lanes.
// - Saturated BYTE-add that rampRGB to sourceRGB BEFORE mask composition.
//   Mask0 skips,255 copies that sum, otherwise floor each product/256
//   independently with complement256-mask and saturated-byte output.
// - Read retained GetMaskPic allocation even when CheckMaskExist is FALSE.
//   CharaClip and EFFECT.pic assignment are ignored. Alpha is engine coverage.
// - Direction2's initial table coordinate includes SCREEN height and then
//   decrements per LOCAL clipped row; direction3 increments per clipped row.
//   Do not replace those phases with a visually neater source-space diagonal.
// Fresh SetParam copies16 words (omitted9..15 undefined ->0); raw InitParameter
// inherited empty preserves them. Calcu empty, CountIsMaxFALSE, NeedPicPrintFALSE.

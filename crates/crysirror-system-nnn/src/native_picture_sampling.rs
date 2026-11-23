// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Thin native drawing plans -> CrySirror immutable raster jobs. This module
//! chooses source leases and explicit stored-byte profiles, never reads pixels.
use anyhow::Result;
use crysirror_core::image_processing::{ImageInput, ImageResource};
use crysirror_image::{
    byte_raster::{ByteRaster, ByteRasterMath},
    effects::ImageEffect,
    fixed_scanlines::{FixedScanlines, ScanlineMath},
    row_blit::RowBlit,
};
use std::sync::Arc;
pub(crate) fn fixed_input(
    base: ImageInput,
    source: Arc<dyn ImageResource>,
    plan: crate::native_stretch_blit::StretchPlan,
    color: Option<([u8; 3], [u8; 3])>,
) -> Result<ImageInput> {
    use crate::native_stretch_blit::StretchMath as M;
    let math = match plan.math {
        M::Copy => ScanlineMath::Stored {
            math: ByteRasterMath::CopyRgb,
        },
        M::CoverageSeparate {
            weight,
            opaque_copy,
        } => ScanlineMath::CoverageSeparate {
            weight,
            opaque_copy,
        },
        M::ConstantSeparate { weights } => ScanlineMath::ConstantSeparate { weights },
    };
    let spec = FixedScanlines::rectangle(
        plan.destination,
        plan.source_q16,
        plan.source_step_q16,
        math,
        plan.corner_rgb_key,
        color,
    )?;
    Ok(ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(base),
            source: Box::new(ImageInput::Resource(source)),
        }),
        effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
            spec: ByteRaster::FixedScanlines { spec },
        }])?
        .into(),
    })
}
pub(crate) fn copy_input(
    base: ImageInput,
    source: Arc<dyn ImageResource>,
    rect: [i32; 4],
    destination: [i32; 2],
    math: ByteRasterMath,
    canvas: [u32; 2],
    clip:Option<[i32;4]>,
) -> Result<ImageInput> {
    let (rect,destination)=if let Some(bounds)=clip {
        let Some(mapping)=crysirror_image::coverage_bounds::intersect_source_window(rect,destination,bounds)? else{return Ok(base);};
        mapping
    }else{(rect,destination)};
    if rect[2] <= 0 || rect[3] <= 0 {
        return Ok(base);
    }
    anyhow::ensure!(
        source
            .size()
            .iter()
            .chain(&canvas)
            .all(|v| (1..=16384).contains(v)),
        "NNN copied image dimensions"
    );
    let Some((region, at)) = crysirror_core::region_map::clip_region_copy(
        Some(source.size()),
        canvas,
        rect.map(i64::from),
        destination.map(i64::from),
        None,
    )?
    else {
        return Ok(base);
    };
    let spec = RowBlit {
        source: region.map(|v| v as i32),
        destination: at.map(|v| v as i32),
        row_offsets: vec![0; region[3] as usize],
        math,
        row_math: Vec::new(),
    };
    spec.validate()?;
    Ok(ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(base),
            source: Box::new(ImageInput::Resource(source)),
        }),
        effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
            spec: ByteRaster::RowBlit { spec },
        }])?
        .into(),
    })
}

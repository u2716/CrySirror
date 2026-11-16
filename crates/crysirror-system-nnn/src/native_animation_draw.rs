// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! ANIMATION137's draw semantics after its retained resource dependencies.
//! These are CPicture calls, not CHANGE40: no same-size, assigned-slot or
//! second-mask prerequisite. Pixel/index expansion remains in shared jobs.
use crate::{image_bridge::NativePictureViews, native_geometry::NativeRects};
use anyhow::{Result, ensure};
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageInput};
use crysirror_image::{effects::ImageEffect, packed_pair_blend::PackedPairBlend};

pub(crate) fn destination(count: i32, frames: i32, rectangles: &NativeRects) -> Result<[i32; 2]> {
    let phase = crate::native_curve::value(count, frames, 0, 10000, 0, 0)?;
    Ok(std::array::from_fn(|axis| {
        rectangles.destination_from[axis].wrapping_add(
            rectangles.destination_to[axis]
                .wrapping_sub(rectangles.destination_from[axis])
                .wrapping_mul(phase)
                / 10000,
        )
    }))
}

/// Call only after the primary LoadDWQ returned TRUE, and after handling
/// cache-only mode. Its TRUE may still coexist with loadOk FALSE. Old is Some
/// only when the requested old LoadDWQ returned TRUE; its failure falls back
/// to normal Blt instead of suppressing the whole animation.
pub(crate) fn image_input(
    base: ImageInput,
    current: &NativePictureViews,
    current_load_ok: bool,
    old: Option<(&NativePictureViews, [i32; 2])>,
    indexed: bool,
    at: [i32; 2],
    canvas: [u32; 2],
    background: bool,
) -> Result<Option<ImageInput>> {
    ensure!(
        canvas.iter().all(|n| (1..=16384).contains(n)),
        "NNN ANIMATION draw canvas"
    );
    let size = current.raw.size();
    ensure!(
        size.iter().all(|n| (1..=16384).contains(n)),
        "NNN ANIMATION picture size"
    );
    if let Some((old, percentages)) = old {
        ensure!(
            percentages.iter().all(|n| (0..=100).contains(n)),
            "NNN ANIMATION transition percent"
        );
        let Some(geometry) = crate::native_picture_pair_geometry::pair(
            at,
            [[0; 2]; 2],
            size.map(|n| n as i32),
            [old.raw.size(), size],
            [
                crate::native_picture_clip::CharaClip::from_bounds(old.source_clip),
                crate::native_picture_clip::CharaClip::from_bounds(current.source_clip),
            ],
            [0, 0, canvas[0] as i32, canvas[1] as i32],
        )?
        else {
            return Ok(None);
        };
        // ChangeTranslateBlt falls back to ChangeBlt if EITHER mask flag is
        // false. Both helpers use raw retained buffers without loadOk guards.
        let use_masks = !indexed && old.has_mask && current.has_mask;
        let spec = PackedPairBlend {
            sources: geometry.sources,
            destination: geometry.destination,
            weights256: percentages.map(|p| ((p * 256 + 50) / 100) as u16),
            use_masks,
        };
        spec.validate()?;
        return Ok(Some(ImageInput::Process {
            input: Box::new(ImageInput::Ternary {
                destination: Box::new(base),
                first: Box::new(ImageInput::Resource(old.raw.clone())),
                second: Box::new(ImageInput::Resource(current.raw.clone())),
            }),
            effects: serde_json::to_vec(&[ImageEffect::PackedPairBlend { spec }])?.into(),
        }));
    }
    if !current_load_ok {
        return Ok(None);
    }
    let Some((source, destination)) = crysirror_image::coverage_bounds::intersect_source_window(
        [0, 0, size[0] as i32, size[1] as i32],
        at,
        current.source_clip,
    )?
    else {
        return Ok(None);
    };
    let source_image = if background {
        &current.opaque
    } else if current.has_mask {
        &current.raw
    } else {
        &current.keyed
    };
    Ok(Some(ImageInput::Composite {
        base: Box::new(base),
        layers: vec![ImageCompositeLayer {
            input: Box::new(ImageInput::Resource(source_image.clone())),
            source,
            destination,
            clip: None,
            opacity: 255,
            math: if background {
                ImageCompositeMath::CopyRgb
            } else {
                ImageCompositeMath::MaskSeparate256
            },
        }],
    }))
}

// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CCommonButton::Print atlas/print-mode policy. All clipping, masks, two-source
//! interpolation and colour work use the common engine image graph builder.
use crate::{
    image_bridge::NativePictureViews,
    native_anime_parts::{PartPaint, PartPrint},
    native_title::{NativeButtonSpec, NativeButtonState},
};
use anyhow::Result;
use crysirror_core::image_processing::ImageInput;

pub(crate) fn image_input(
    spec: &NativeButtonSpec,
    state: &NativeButtonState,
    image: &NativePictureViews,
    canvas: [u32; 2],
    base: ImageInput,
) -> Result<Option<ImageInput>> {
    let (frames, percent) = state.frame(spec);
    let regions = [
        spec.source_rect(state.mode, frames[0], image.raw.size())?,
        spec.source_rect(state.mode, frames[1], image.raw.size())?,
    ];
    let (regions, paint) = if spec.print_mode == 2 && percent > 0 && percent < 100 {
        // The native Print helper is ChangeTranslateBlt, including its Beta
        // path for an RGB atlas: destination RGB is not included in that path.
        (
            regions,
            PartPaint::Pair {
                cells: [0, 1],
                percent: [i32::from(percent), 100 - i32::from(percent)],
            },
        )
    } else {
        let selected = if percent == 0 { 1 } else { 0 };
        ([regions[selected]; 2], PartPaint::Masked { cell: 0 })
    };
    let opaque;
    let source = if spec.print_mode == 0 {
        // Blt(FALSE) ignores masks and RGB keys; coverage alpha is not native
        // colour. The shared opaque view has no private pixel conversion.
        opaque = NativePictureViews {
            has_mask: false,
            mask_storage:None,
            source_clip: image.source_clip,
            raw: image.opaque.clone(),
            opaque: image.opaque.clone(),
            keyed: image.opaque.clone(),
        };
        &opaque
    } else {
        image
    };
    crate::native_anime_parts_image::regions_image_input(
        regions,
        PartPrint {
            destination: spec.origin,
            paint,
        },
        source,
        canvas,
        base,
    )
}

// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! BATHWINDOW's Calcu recipe and ordinary masked Print policy. No pixel access
//! occurs here; shared image jobs complete every mutation before first Print.
use crate::{
    native_bath_updates::Mutation,
    native_bath_window::BathWindowClass,
    native_picture_effect::{PictureEffectFrame, PictureEffectKind},
    native_presentation::NativeFrameEffect,
};
use anyhow::Result;
use crysirror_core::image_processing::ImageInput;
use crysirror_image::byte_raster::ByteRasterMath;
use std::sync::Arc;
pub(crate) fn class<'a>(
    mut effects: impl Iterator<Item = &'a NativeFrameEffect>,
) -> Option<Arc<BathWindowClass>> {
    effects.find_map(|e| {
        if let NativeFrameEffect::Picture(f) = e {
            f.bath_class.clone()
        } else {
            None
        }
    })
}
pub(crate) fn attach<'a>(
    effects: impl Iterator<Item = &'a mut NativeFrameEffect>,
    class: Option<Arc<BathWindowClass>>,
) {
    for e in effects {
        if let NativeFrameEffect::Picture(f) = e {
            if matches!(f.state.kind, PictureEffectKind::BathWindow { .. }) {
                f.bath_class = class.clone();
            }
        }
    }
}
pub(crate) fn mutations<'a>(effects: impl Iterator<Item = &'a NativeFrameEffect>) -> Vec<Mutation> {
    effects
        .filter_map(|e| {
            if let NativeFrameEffect::Picture(f) = e {
                f.bath_update.as_ref().map(|update| Mutation {
                    bank: f.bank.clone(),
                    bank_limit: f.bank_limit,
                    slot: f.slot,
                    update: update.clone(),
                })
            } else {
                None
            }
        })
        .collect()
}
pub(crate) fn image_input(
    frame: &PictureEffectFrame,
    p: &[i32; 9],
    base: ImageInput,
) -> Result<Option<ImageInput>> {
    let Some(picture) = frame.bank.picture(frame.slot).filter(|p| p.load_ok) else {
        return Ok(None);
    };
    let picture = &picture.picture;
    let source = if picture.has_mask {
        picture.raw.clone()
    } else {
        picture.keyed.clone()
    };
    Ok(Some(crate::native_picture_sampling::copy_input(
        base,
        source,
        [p[4], p[5], p[6], p[7]],
        [p[4], p[5]],
        ByteRasterMath::AlphaWeightedSeparateRgb256 { weight: 256 },
        frame.canvas,
        Some(picture.source_clip),
    )?))
}

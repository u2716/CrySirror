// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Shared class/ordered native Print routing for natural effects. This module
//! constructs engine image graphs; it never reads or renders resource pixels.
use crate::{
    image_bridge::NativePictureViews,
    native_particle_classes::Classes,
    native_picture_effect::{PictureEffectFrame, PictureEffectKind},
    native_presentation::NativeFrameEffect,
};
use anyhow::{Context, Result};
use crysirror_core::image_processing::ImageInput;
use crysirror_image::{
    byte_raster::ByteRaster, effects::ImageEffect, temporal_pattern::PatternRng,
};
use std::sync::Arc;
pub(crate) fn calcu_beach<'a>(
    classes: &mut Classes,
    effects: impl Iterator<Item = &'a NativeFrameEffect>,
) {
    // BEACH Calcu has no resource/RNG dependency. Every active slot changes
    // the ONE shared phase table before ANY slot Prints, even when invisible.
    for effect in effects {
        if let NativeFrameEffect::Picture(frame) = effect {
            if let PictureEffectKind::Beach { parameters } = frame.state.kind {
                Arc::make_mut(&mut classes.beach).calcu(&parameters);
            }
        }
    }
}
fn views(picture: &crate::image_bridge::PreparedPicture) -> NativePictureViews {
    NativePictureViews {
        has_mask: picture.has_mask,
        mask_storage: picture.mask_storage.clone(),
        source_clip: picture.source_clip,
        raw: picture.raw.clone(),
        opaque: picture.opaque.clone(),
        keyed: picture.keyed.clone(),
    }
}
pub(crate) fn image_input(
    frame: &PictureEffectFrame,
    classes: &mut Classes,
    base: ImageInput,
    random: &mut PatternRng,
) -> Result<Option<ImageInput>> {
    match frame.state.kind {
        PictureEffectKind::Beach { parameters } => {
            classes.beach.image_input(&parameters, frame.canvas, base)
        }
        PictureEffectKind::Hotaru { parameters } => {
            let Some(paint) = Arc::make_mut(&mut classes.fireflies).print_plan(
                &parameters,
                frame.state.count,
                frame.state.frames,
                frame.canvas,
                random,
            )?
            else {
                return Ok(None);
            };
            let spec = paint.brush_spec()?;
            Ok(Some(ImageInput::Process {
                input: Box::new(base),
                effects: serde_json::to_vec(&[ImageEffect::RadialRgbBrushes { spec }])?.into(),
            }))
        }
        PictureEffectKind::Hanabira { parameters } => {
            let picture = frame.bank.picture(frame.slot);
            let size = picture.map(|p| p.picture.raw.size()).unwrap_or([1, 1]);
            let sprites = Arc::make_mut(&mut classes.petals).print_plan(
                &parameters,
                frame.slot,
                frame.state.count,
                frame.state.frames,
                frame.canvas,
                size,
                random,
            )?;
            let Some(picture) = picture else {
                return Ok(None);
            };
            let Some(draw) = crate::native_hanabira_draw::plan(
                sprites,
                &views(&picture.picture),
                picture.load_ok,
                frame.canvas,
            )?
            else {
                return Ok(None);
            };
            Ok(Some(match draw {
                crate::native_hanabira_draw::Draw::Rows { source, spec } => ImageInput::Process {
                    input: Box::new(ImageInput::Raster {
                        destination: Box::new(base),
                        source: Box::new(ImageInput::Resource(source)),
                    }),
                    effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
                        spec: ByteRaster::RowBlitBatch { spec },
                    }])?
                    .into(),
                },
                crate::native_hanabira_draw::Draw::Pairs { source, spec } => ImageInput::Process {
                    input: Box::new(ImageInput::Ternary {
                        destination: Box::new(base),
                        first: Box::new(ImageInput::Resource(source.clone())),
                        second: Box::new(ImageInput::Resource(source)),
                    }),
                    effects: serde_json::to_vec(&[ImageEffect::PackedPairBatch { spec }])?.into(),
                },
            }))
        }
        PictureEffectKind::HotaruNew { parameters } => {
            let picture = frame.bank.picture(frame.slot);
            let size = picture.map(|p| p.picture.raw.size()).unwrap_or([1, 1]);
            let picture = picture.map(|p| (views(&p.picture), p.load_ok));
            let class = classes
                .fireflies_new
                .as_mut()
                .context("NNN HOTARUNEW absent in native constructor profile")?;
            Arc::make_mut(class).image_input(
                &parameters,
                frame.canvas,
                size,
                picture.as_ref().map(|(view, loaded)| (view, *loaded)),
                base,
                random,
            )
        }
        _ => anyhow::bail!("NNN natural Print class mismatch"),
    }
}

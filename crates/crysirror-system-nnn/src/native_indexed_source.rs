// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Indexed CPicture resource policy: shared codecs preserve source bytes;
//! native allocation and PNG palette-history rules stay explicit per session.
use crate::{
    native_png_palette::{IndexedCopy, PngPalette},
    native_resources::ResourceNames,
    resource_bridge::{NativeImageSource, NativeResources},
};
use anyhow::{Context, Result, ensure};
use crysirror_core::image_processing::ImageInput;
use crysirror_image::{
    indexed_plane::IndexedPlane, linear_byte_storage::LinearByteStorage, png_rows::PngRows,
};
use crysirror_tasks::{Cancellation, Task, TaskPool};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct Decoded {
    pub plane: IndexedPlane,
    pub storage: LinearByteStorage,
    pub palette: PngPalette,
    pub native_mask_exists: bool,
    pub source_clip: [i32; 4],
}
pub(crate) fn encoded(input: ImageInput) -> Result<(String, Arc<[u8]>)> {
    match input {
        ImageInput::Encoded { name, bytes, .. } => Ok((name, bytes)),
        ImageInput::Process { input, effects } => {
            let effects: Vec<crysirror_image::effects::ImageEffect> =
                serde_json::from_slice(&effects)?;
            ensure!(
                effects.iter().all(|e| matches!(
                    e,
                    crysirror_image::effects::ImageEffect::OverrideAlpha { alpha: 255 }
                )),
                "NNN indexed encoded source wrapper"
            );
            match *input {
                ImageInput::Encoded { name, bytes, .. } => Ok((name, bytes)),
                _ => anyhow::bail!("NNN indexed encoded source depth"),
            }
        }
        _ => anyhow::bail!("NNN indexed source is not an encoded DWQ lease"),
    }
}
pub(crate) fn read(
    resources: &NativeResources,
    names: &ResourceNames,
    levels: u32,
    name: &[u8],
    palette: PngPalette,
    old: &LinearByteStorage,
    old_palette: Option<[[u8; 3]; 256]>,
    cancel: &Cancellation,
) -> Result<Option<Decoded>> {
    cancel.check()?;
    let lookup = names.picture(name, levels)?;
    let Some(source) = resources.optional_image_source(&lookup, cancel)? else {
        return Ok(None);
    };
    decode_source(resources, source, palette, old, old_palette, cancel).map(Some)
}
/// Decode an already resolved source; constructor fallback probes must not
/// reopen the archive or reorder the static PNG-loader palette observation.
pub(crate) fn decode_source(
    resources: &NativeResources,
    source: NativeImageSource,
    mut palette: PngPalette,
    old: &LinearByteStorage,
    old_palette: Option<[[u8; 3]; 256]>,
    cancel: &Cancellation,
) -> Result<Decoded> {
    cancel.check()?;
    let mask_exists = source.has_mask;
    let (path, bytes) = encoded(source.input)?;
    let info = crysirror_formats_blackcyc::Dwq::metadata(&bytes)?;
    let pixels = crysirror_image::rgba_len(info.size[0], info.size[1])? / 4;
    let (plane, new_allocation, mask_exists) = if info.pack == 8 {
        let rows = PngRows::decode(
            bytes.get(64..).context("NNN indexed PNG payload absent")?,
            &|| cancel.check(),
        )?;
        ensure!(rows.size == info.size, "NNN indexed PNG dimension profile");
        palette.observe(&rows)?;
        let plane = match palette.copy8(&rows, &|| cancel.check())? {
            IndexedCopy::Pixels(plane) => Some(plane),
            IndexedCopy::RetainedPixelsBlackPalette => None,
        };
        (plane, pixels * 4 + 64, false)
    } else {
        let registry = crysirror_image::DecoderRegistry::default();
        let resource = crysirror_resource::DecodeContext::new(resources.client())
            .with_cancellation(cancel.clone());
        let context =
            crysirror_image::ImageDecodeContext::new(&path, &registry).with_resources(&resource);
        let (mut plane, depth) =
            crysirror_formats_blackcyc::Dwq::indexed_plane_and_depth(&bytes, &context)?;
        // LoadBitmapHeaderAndPalette touches m_palette only for8-bit BMP.
        // A later plain non8-bit load therefore retains an existing palette;
        // the fresh NULL/uninitialized case is explicitly black instead.
        if depth != 8 {
            plane.palette = old_palette.unwrap_or([[0; 3]; 256]);
        }
        (
            Some(plane),
            pixels * if depth == 8 { 1 } else { 4 } + 64,
            mask_exists,
        )
    };
    // Native MakeDataBuffer retains the allocation while its capacity fits,
    // otherwise reallocates without copying. Shared storage zeroes only that
    // originally undefined new allocation; non8-bit PNG leaves the old prefix
    // untouched and copies an originally uninitialized (now black) palette.
    let storage = old.replace_prefix(
        new_allocation,
        plane.as_ref().map(|p| p.indices.as_ref()),
        &|| cancel.check(),
    )?;
    let colors = plane.as_ref().map(|p| p.palette).unwrap_or([[0; 3]; 256]);
    let plane = IndexedPlane {
        size: info.size,
        palette: colors,
        indices: storage.prefix(pixels, &|| cancel.check())?,
    };
    plane.validate()?;
    cancel.check()?;
    let source_clip = if mask_exists {
        info.clip_insets
            .map(|insets| crate::native_picture_clip::CharaClip::header(info.size, insets).bounds())
            .unwrap_or([0, 0, info.size[0] as i32, info.size[1] as i32])
    } else {
        [0, 0, info.size[0] as i32, info.size[1] as i32]
    };
    // CutDataByMask returns immediately for b256. Only a pre-cut DWQ header
    // replaces its otherwise-full trim; do not scan expanded colour coverage.
    Ok(Decoded {
        plane,
        storage,
        palette,
        native_mask_exists: mask_exists,
        source_clip,
    })
}
/// Palette is a detached candidate from the native admission order. Caller
/// commits the returned palette only with this successful resource dependency;
/// concurrent completion must never choose the next palette history by timing.
pub(crate) fn begin(
    pool: &TaskPool,
    resources: NativeResources,
    names: ResourceNames,
    levels: u32,
    name: Vec<u8>,
    palette: PngPalette,
    old: LinearByteStorage,
    old_palette: Option<[[u8; 3]; 256]>,
) -> Result<Task<Option<Decoded>>> {
    pool.spawn(move |cancel| {
        read(
            &resources,
            &names,
            levels,
            &name,
            palette,
            &old,
            old_palette,
            &cancel,
        )
    })
}

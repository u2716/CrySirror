// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native picture-bank snapshots for effect-owned LoadDWQ. The cache's
//! filename/loadOk/mask-exists facts are independent of retained RGB/masks.
//! These helpers never read pixels and never publish a partial transaction.
use crate::{
    image_bridge::{NativePictureViews, PreparedPicture},
    native_picture_cache::PictureCache,
};
use anyhow::{Result, ensure};
use crysirror_core::state::ClientId;
use std::collections::BTreeMap;

pub(crate) type Bank = BTreeMap<i32, PictureCache<NativePictureViews>>;
pub(crate) fn snapshot(
    client: ClientId,
    physical: i32,
    pictures: &BTreeMap<i32, PreparedPicture>,
    retained: &Bank,
) -> Result<Bank> {
    ensure!(
        matches!(physical, 16 | 32),
        "NNN physical picture bank profile"
    );
    let mut bank = retained.clone();
    ensure!(
        bank.iter()
            .all(|(slot, p)| (0..physical).contains(slot) && p.client() == client),
        "NNN retained native picture bank scope"
    );
    for slot in 0..physical {
        if bank.contains_key(&slot) {
            continue;
        }
        let mut cache = if let Some(p) = pictures.get(&slot) {
            let views = NativePictureViews {
                has_mask: p.has_mask,
                mask_storage: p.mask_storage.clone(),
                source_clip: p.source_clip,
                raw: p.raw.clone(),
                opaque: p.opaque.clone(),
                keyed: p.keyed.clone(),
            };
            let mut cache = PictureCache::sized(client, views, false, p.has_mask)?;
            cache.filename = if p.name.contains(&b'\\') || p.name.contains(&b'/') {
                p.name.clone()
            } else {
                crate::native_picture_name::script_native_name(&p.name)?
            };
            cache
        } else {
            PictureCache::empty(client)?
        };
        // CPicture::InitWork initializes the empty filename/loadOk flag. This
        // is not an invented successful image or a zero substitute for a file.
        if cache.storage.is_none() {
            cache.load_ok = false;
        }
        bank.insert(slot, cache);
    }
    Ok(bank)
}

pub(crate) fn prepared(
    slot: i32,
    cache: &PictureCache<NativePictureViews>,
    previous: Option<&PreparedPicture>,
    opaque_display: bool,
) -> Option<PreparedPicture> {
    let views = cache.storage.as_ref()?;
    // A failed new load leaves the old allocation/clip, but its filename and
    // CheckMaskExist become the new cache facts. The owner MUST also retain
    // cache.load_ok: normal CPicture::Blt refuses these old pixels, whereas
    // raw GetBuffer users (including STAFFROLL) still see them.
    let display = if opaque_display {
        views.opaque.clone()
    } else if cache.mask_exists {
        views.raw.clone()
    } else {
        views.keyed.clone()
    };
    Some(PreparedPicture {
        has_mask: cache.mask_exists,
        mask_storage: views.mask_storage.clone(),
        source_clip: views.source_clip,
        slot,
        preload: previous.is_some_and(|p| p.preload),
        name: cache.filename.clone(),
        raw: views.raw.clone(),
        opaque: views.opaque.clone(),
        keyed: views.keyed.clone(),
        display,
    })
}
pub(crate) fn same_name(native: &[u8], script_basename: &[u8]) -> Result<bool> {
    Ok(native == script_basename
        || native == crate::native_picture_name::script_native_name(script_basename)?)
}

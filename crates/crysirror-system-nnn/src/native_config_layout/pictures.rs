// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Config's aliases of the existing native picture-bank/cache/reload owners.
//! Shared image workers decode; Config does not keep a second cache algorithm.
use super::allocation::Slot;
use crate::{
    image_bridge::NativePictureViews,
    native_picture_cache::{BeginLoad, PictureCache},
    native_picture_reload::{ContextData, Reload},
    native_resources::{ImageLookup, ResourceNames},
};
use anyhow::{Result, ensure};
use crysirror_core::{service::HostServices, state::ClientId};
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Key {
    Allocated(Slot),
    CommonBackground,
    System(Vec<u8>),
}
#[derive(Clone)]
pub(crate) struct Picture {
    pub object_exists: bool,
    pub cache: PictureCache<NativePictureViews>,
}
impl Picture {
    pub(crate) fn fresh(client: ClientId, object_exists: bool) -> Result<Self> {
        Ok(Self {
            object_exists,
            cache: PictureCache::empty(client)?,
        })
    }
    pub(crate) fn inherited(object_exists: bool, cache: PictureCache<NativePictureViews>) -> Self {
        Self {
            object_exists,
            cache,
        }
    }
    pub(crate) fn paint(&self) -> Option<&NativePictureViews> {
        if self.object_exists && self.cache.load_ok {
            self.cache.storage.as_ref()
        } else {
            None
        }
    }
    pub(crate) fn hit_picture(&self) -> Option<&NativePictureViews> {
        // CPicture::CheckOnPic tests existing storage/mask flags, not loadOk.
        // The same shared native cache preserves it across a failed reload.
        self.object_exists
            .then_some(self.cache.storage.as_ref())
            .flatten()
    }
}
pub(crate) fn system_request(
    name: Option<&[u8]>,
    names: &ResourceNames,
    levels: u32,
) -> Result<(Key, Vec<u8>, ImageLookup)> {
    let name = name.unwrap_or(b"");
    ensure!(
        name.len() <= 254 && !name.contains(&0),
        "NNN Config system picture filename"
    );
    let logical = [
        b"sys/".as_slice(),
        if name.is_empty() {
            b"error".as_slice()
        } else {
            name
        },
    ]
    .concat();
    // NULL and empty names choose original m_ppPic[0], loaded from sys/error.
    // This is an original asset identity, not a generated fallback picture.
    Ok((
        Key::System(name.to_vec()),
        logical.clone(),
        names.picture(&logical, levels)?,
    ))
}
pub(crate) struct Preparation {
    key: Key,
    candidate: Picture,
    reload: Option<Reload>,
}
impl Preparation {
    pub(crate) fn begin(
        key: Key,
        previous: Picture,
        name: Option<&[u8]>,
        context: &ContextData,
        owner: u64,
    ) -> Result<Self> {
        let mut candidate = previous;
        let reload = if !candidate.object_exists {
            None
        } else if let Some(name) = name {
            Some(Reload::begin(context, &candidate.cache, owner, name)?)
        } else {
            // Original passes a NULL basename into its %s format. Define that
            // nonportable input as a missing filename and never open a made-up
            // path. The marker is native cache metadata, not an asset request.
            match candidate.cache.begin(owner, b"sys\\(null)", false)? {
                BeginLoad::CachedTrue => (),
                BeginLoad::Read { token } => candidate.cache.missing(token)?,
                BeginLoad::Pending { .. } => {
                    anyhow::bail!("NNN Config null-name load has another retained owner")
                }
            }
            None
        };
        Ok(Self {
            key,
            candidate,
            reload,
        })
    }
    pub(crate) fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<(Key, Picture)>> {
        if let Some(reload) = self.reload.as_mut() {
            let Some(result) = reload.poll(host)? else {
                return Ok(None);
            };
            self.candidate.cache = result.cache;
            self.reload = None;
        }
        Ok(Some((self.key.clone(), self.candidate.clone())))
    }
}

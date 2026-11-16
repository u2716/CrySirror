// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Retained indexed CPicture read/colour-expansion transaction. Missing-file
//! FALSE, same-name TRUE and loadOk remain distinct. Palette history is an
//! explicit detached candidate committed by the native ordered-load owner.
use crate::{
    native_indexed_picture::{IndexedViews, Preparation as ImagePreparation},
    native_indexed_source::{self, Decoded},
    native_picture_cache::{BeginLoad, PictureCache, Storage},
    native_picture_reload::ContextData,
    native_png_palette::PngPalette,
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{service::HostServices, state::ClientId};
use crysirror_tasks::Task;
#[derive(Clone)]
pub(crate) struct IndexedPicture {
    pub decoded: Decoded,
    pub views: IndexedViews,
}
impl IndexedPicture {
    pub(crate) fn native_views(&self) -> crate::image_bridge::NativePictureViews {
        let mut view = self.views.unmasked_views();
        view.source_clip = self.decoded.source_clip;
        view
    }
}
impl Storage for IndexedPicture {
    fn validate_scope(&self, client: ClientId) -> Result<()> {
        self.decoded.plane.validate()?;
        ensure!(
            self.views.opaque.client() == client
                && self.views.keyed.client() == client
                && self.views.opaque.size() == self.decoded.plane.size
                && self.views.keyed.size() == self.decoded.plane.size,
            "NNN indexed cache picture scope/size"
        );
        Ok(())
    }
}
#[derive(Clone)]
pub(crate) struct Ready {
    pub cache: PictureCache<IndexedPicture>,
    pub palette: PngPalette,
    pub returned: bool,
}
pub(crate) struct Reload {
    candidate: PictureCache<IndexedPicture>,
    palette: PngPalette,
    token: Option<u64>,
    read: Option<Task<Option<Decoded>>>,
    image: Option<ImagePreparation>,
    decoded: Option<Decoded>,
    ready: Option<Ready>,
    failure: Option<String>,
    pub api_calls: u64,
}
impl Reload {
    pub(crate) fn begin(
        context: &ContextData,
        current: &PictureCache<IndexedPicture>,
        owner: u64,
        name: &[u8],
        palette: PngPalette,
    ) -> Result<Self> {
        let client = context.resources.client();
        ensure!(
            client == current.client() && (current.indexed || current.filename.is_empty()),
            "NNN indexed reload client/depth"
        );
        let mut candidate = current.clone();
        let disposition = candidate.begin(owner, name, true)?;
        let mut work = Self {
            candidate,
            palette,
            token: None,
            read: None,
            image: None,
            decoded: None,
            ready: None,
            failure: None,
            api_calls: 0,
        };
        match disposition {
            BeginLoad::CachedTrue => {
                work.ready = Some(Ready {
                    cache: work.candidate.clone(),
                    palette: work.palette.clone(),
                    returned: true,
                })
            }
            BeginLoad::Pending { .. } => {
                anyhow::bail!("NNN indexed reload pending receipt must be retained")
            }
            BeginLoad::Read { token } => {
                let old = current.storage.as_ref();
                work.read = Some(native_indexed_source::begin(
                    &context.pool,
                    context.resources.clone(),
                    context.names.clone(),
                    context.levels,
                    name.to_vec(),
                    work.palette.clone(),
                    old.map(|s| s.decoded.storage.clone()).unwrap_or_default(),
                    old.map(|s| s.decoded.plane.palette),
                )?);
                work.token = Some(token);
            }
        }
        Ok(work)
    }
    pub(crate) fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<Ready>> {
        if let Some(error) = &self.failure {
            anyhow::bail!("NNN indexed reload: {error}")
        }
        if let Some(ready) = &self.ready {
            return Ok(Some(ready.clone()));
        }
        let result = self.advance(host);
        if let Err(error) = &result {
            self.failure = Some(format!("{error:#}"));
        }
        result
    }
    fn advance(&mut self, host: &mut dyn HostServices) -> Result<Option<Ready>> {
        if let Some(read) = self.read.as_mut() {
            let Some(decoded) = read.try_take()? else {
                return Ok(None);
            };
            self.read = None;
            let token = self.token.context("NNN indexed reload token absent")?;
            let Some(decoded) = decoded else {
                self.candidate.missing(token)?;
                let ready = Ready {
                    cache: self.candidate.clone(),
                    palette: self.palette.clone(),
                    returned: false,
                };
                self.ready = Some(ready.clone());
                return Ok(Some(ready));
            };
            let image =
                ImagePreparation::begin(self.candidate.client(), decoded.plane.clone(), host)?;
            self.api_calls += image.api_calls;
            self.image = Some(image);
            self.decoded = Some(decoded);
        }
        let image = self
            .image
            .as_mut()
            .context("NNN indexed expansion dependency absent")?;
        let before = image.api_calls;
        let result = image.poll(host);
        self.api_calls += image.api_calls - before;
        let Some(views) = result? else {
            return Ok(None);
        };
        let decoded = self
            .decoded
            .take()
            .context("NNN indexed decoded storage absent")?;
        self.palette = decoded.palette.clone();
        let mask_exists = decoded.native_mask_exists;
        self.candidate.loaded(
            self.token.context("NNN indexed token absent")?,
            IndexedPicture { decoded, views },
            mask_exists,
        )?;
        self.image = None;
        let ready = Ready {
            cache: self.candidate.clone(),
            palette: self.palette.clone(),
            returned: true,
        };
        self.ready = Some(ready.clone());
        Ok(Some(ready))
    }
}

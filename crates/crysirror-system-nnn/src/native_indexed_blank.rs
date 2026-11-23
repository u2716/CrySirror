// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Sized indexed CPicture constructor dependency. Originally uninitialized
//! index bytes and NULL palette are defined as engine-owned zero/black planes;
//! allocating them never changes CPngLoader's separate global palette history.
use crate::{
    native_indexed_picture::Preparation as ImagePreparation, native_indexed_reload::IndexedPicture,
    native_indexed_source::Decoded, native_picture_cache::PictureCache,
    native_png_palette::PngPalette,
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{service::HostServices, state::ClientId};
use crysirror_image::{indexed_plane::IndexedPlane, linear_byte_storage::LinearByteStorage};
use crysirror_tasks::{Task, TaskPool};
pub(crate) struct Preparation {
    client: ClientId,
    task: Option<Task<Decoded>>,
    decoded: Option<Decoded>,
    image: Option<ImagePreparation>,
    ready: Option<PictureCache<IndexedPicture>>,
    failure: Option<String>,
}
impl Preparation {
    pub(crate) fn begin(
        client: ClientId,
        pool: &TaskPool,
        size: [u32; 2],
        palette: PngPalette,
    ) -> Result<Self> {
        ensure!(client.0 != 0, "NNN indexed blank client");
        let pixels = crysirror_image::rgba_len(size[0], size[1])? / 4;
        let task = pool.spawn(move |cancel| {
            let storage =
                LinearByteStorage::default()
                    .replace_prefix(pixels + 64, None, &|| cancel.check())?;
            let plane = IndexedPlane {
                size,
                indices: storage.prefix(pixels, &|| cancel.check())?,
                palette: [[0; 3]; 256],
            };
            plane.validate()?;
            Ok(Decoded {
                plane,
                storage,
                palette,
                native_mask_exists: false,
                source_clip: [0, 0, size[0] as i32, size[1] as i32],
            })
        })?;
        Ok(Self {
            client,
            task: Some(task),
            decoded: None,
            image: None,
            ready: None,
            failure: None,
        })
    }
    pub(crate) fn poll(
        &mut self,
        host: &mut dyn HostServices,
    ) -> Result<Option<PictureCache<IndexedPicture>>> {
        if let Some(error) = &self.failure {
            anyhow::bail!("NNN indexed blank: {error}")
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
    fn advance(
        &mut self,
        host: &mut dyn HostServices,
    ) -> Result<Option<PictureCache<IndexedPicture>>> {
        if let Some(task) = self.task.as_mut() {
            let Some(decoded) = task.try_take()? else {
                return Ok(None);
            };
            self.task = None;
            self.image = Some(ImagePreparation::begin(
                self.client,
                decoded.plane.clone(),
                host,
            )?);
            self.decoded = Some(decoded);
        }
        let Some(views) = self
            .image
            .as_mut()
            .context("NNN indexed blank image dependency")?
            .poll(host)?
        else {
            return Ok(None);
        };
        let ready = PictureCache::sized(
            self.client,
            IndexedPicture {
                decoded: self.decoded.take().context("NNN indexed blank storage")?,
                views,
            },
            true,
            false,
        )?;
        self.ready = Some(ready.clone());
        self.image = None;
        Ok(Some(ready))
    }
}

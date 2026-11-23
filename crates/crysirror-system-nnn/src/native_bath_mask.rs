// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! BATHWINDOW's retained mask update owns real shared image receipts. Steam
//! and erase run before the next native Calcu slot; Print sees the final mask.
use crate::{
    image_bridge::NativePictureViews, native_bath_window::Update, native_mask_storage::MaskStorage,
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource},
    service::HostServices,
    state::ClientId,
};
use crysirror_image::{
    byte_raster::ByteRaster, effects::ImageEffect, linear_channel_transfer::LinearChannelTransfer,
};
use std::sync::Arc;
#[derive(Clone)]
pub(crate) struct Ready {
    pub picture: NativePictureViews,
    pub flush_filename: bool,
}
pub(crate) struct Preparation {
    client: ClientId,
    picture: NativePictureViews,
    flush_filename: bool,
    job: Option<ImageJob>,
    updated_buffer: Option<Arc<dyn ImageResource>>,
    ready: Option<Ready>,
    failure: Option<String>,
    pub api_calls: u64,
}
impl Preparation {
    pub(crate) fn begin(
        client: ClientId,
        picture: NativePictureViews,
        atlas: Arc<dyn ImageResource>,
        update: Update,
        host: &mut dyn HostServices,
    ) -> Result<Self> {
        ensure!(
            picture.has_mask
                && picture.raw.client() == client
                && atlas.client() == client
                && atlas.size() == [128, 384],
            "NNN bath mask source scope/profile"
        );
        let mask = picture
            .mask_storage
            .as_ref()
            .context("NNN bath mask allocation absent")?;
        ensure!(
            mask.buffer.client() == client,
            "NNN bath retained mask scope"
        );
        let spec = update.brushes(picture.raw.size()[0])?;
        if spec
            .brushes
            .iter()
            .all(|b| b.destination[2] == 0 || b.destination[3] == 0)
        {
            // Source Calcu still drew the template selector/updated mouse,
            // but its zero stamp/erase loops write no mask byte. Filename
            // Flush remains real metadata; do not clone a canvas for a no-op.
            let ready = Ready {
                picture: picture.clone(),
                flush_filename: update.flush_filename,
            };
            return Ok(Self {
                client,
                picture,
                flush_filename: update.flush_filename,
                job: None,
                updated_buffer: None,
                ready: Some(ready),
                failure: None,
                api_calls: 0,
            });
        }
        let job = host.image_metadata(client)?.begin_process(
            ImageInput::Raster {
                destination: Box::new(ImageInput::Resource(mask.buffer.clone())),
                source: Box::new(ImageInput::Resource(atlas)),
            },
            serde_json::to_vec(&[ImageEffect::ByteRaster {
                spec: ByteRaster::AlphaBrushes { spec },
            }])?
            .into(),
        )?;
        Ok(Self {
            client,
            picture,
            flush_filename: update.flush_filename,
            job: Some(job),
            updated_buffer: None,
            ready: None,
            failure: None,
            api_calls: 1,
        })
    }
    pub(crate) fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<Ready>> {
        if let Some(error) = &self.failure {
            anyhow::bail!("NNN bath mask: {error}")
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
        let result = match self
            .job
            .as_mut()
            .context("NNN bath mask image dependency absent")?
            .poll()?
        {
            ImageProcessState::Pending => return Ok(None),
            ImageProcessState::Failed(error) => anyhow::bail!("NNN bath mask worker: {error}"),
            ImageProcessState::Ready(image) => image,
        };
        ensure!(
            result.client() == self.client,
            "NNN bath mask foreign image"
        );
        if self.updated_buffer.is_none() {
            ensure!(
                result.size() == self.picture.mask_storage.as_ref().unwrap().buffer.size(),
                "NNN bath retained mask capacity changed"
            );
            let size = self.picture.raw.size();
            let count = u64::from(size[0]) * u64::from(size[1]);
            let spec = LinearChannelTransfer {
                source_channel: 3,
                destination_channel: 3,
                source_offset: 0,
                destination_offset: 0,
                length: count.min(self.picture.mask_storage.as_ref().unwrap().capacity()),
            };
            self.job = Some(
                host.image_metadata(self.client)?.begin_process(
                    ImageInput::Raster {
                        destination: Box::new(ImageInput::Resource(self.picture.raw.clone())),
                        source: Box::new(ImageInput::Resource(result.clone())),
                    },
                    serde_json::to_vec(&[ImageEffect::ByteRaster {
                        spec: ByteRaster::LinearChannelTransfer { spec },
                    }])?
                    .into(),
                )?,
            );
            self.api_calls += 1;
            self.updated_buffer = Some(result);
            return Ok(None);
        }
        ensure!(
            result.size() == self.picture.raw.size(),
            "NNN bath picture result shape"
        );
        let mut picture = self.picture.clone();
        picture.raw = result.clone();
        picture.mask_storage = Some(MaskStorage {
            buffer: self.updated_buffer.take().unwrap(),
            view: result,
        });
        // Calcu never calls CutDataByMask: preserve the old CharaClip even
        // after a brush makes previously transparent pixels nonzero.
        let ready = Ready {
            picture,
            flush_filename: self.flush_filename,
        };
        self.ready = Some(ready.clone());
        self.job = None;
        Ok(Some(ready))
    }
}

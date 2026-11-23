// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Publish palette-indexed native pictures through packed/image services.
//! Blt256 keys the first SOURCE INDEX, so duplicate palette colours remain
//! distinct. Keying expanded RGB would erase legitimate opaque sprite cells.
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource},
    service::HostServices,
    state::ClientId,
};
use crysirror_image::{
    byte_raster::ByteRaster, effects::ImageEffect, indexed_plane::IndexedPlane,
    linear_channel_transfer::LinearChannelTransfer,
};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct IndexedViews {
    pub indices: IndexedPlane,
    pub opaque: Arc<dyn ImageResource>,
    pub keyed: Arc<dyn ImageResource>,
}
impl IndexedViews {
    pub(crate) fn unmasked_views(&self) -> crate::image_bridge::NativePictureViews {
        let [w, h] = self.indices.size;
        crate::image_bridge::NativePictureViews {
            has_mask: false,
            mask_storage: None,
            source_clip: [0, 0, w as i32, h as i32],
            raw: self.opaque.clone(),
            opaque: self.opaque.clone(),
            keyed: self.keyed.clone(),
        }
    }
}
pub(crate) struct Preparation {
    client: ClientId,
    plane: IndexedPlane,
    job: Option<ImageJob>,
    keyed: Option<Arc<dyn ImageResource>>,
    stage: u8,
    ready: Option<IndexedViews>,
    failure: Option<String>,
    pub api_calls: u64,
}
impl Preparation {
    pub(crate) fn begin(
        client: ClientId,
        plane: IndexedPlane,
        host: &mut dyn HostServices,
    ) -> Result<Self> {
        ensure!(client.0 != 0, "NNN indexed picture client");
        plane.validate()?;
        let job = host
            .image_metadata(client)?
            .begin_packed_pixels(plane.packed()?)?;
        Ok(Self {
            client,
            plane,
            job: Some(job),
            keyed: None,
            stage: 0,
            ready: None,
            failure: None,
            api_calls: 1,
        })
    }
    pub(crate) fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<IndexedViews>> {
        if let Some(error) = &self.failure {
            anyhow::bail!("NNN indexed picture: {error}");
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
    fn advance(&mut self, host: &mut dyn HostServices) -> Result<Option<IndexedViews>> {
        let image = match self
            .job
            .as_mut()
            .context("NNN indexed picture job absent")?
            .poll()?
        {
            ImageProcessState::Pending => return Ok(None),
            ImageProcessState::Failed(error) => {
                anyhow::bail!("NNN indexed picture worker: {error}")
            }
            ImageProcessState::Ready(image) => image,
        };
        ensure!(
            image.client() == self.client && image.size() == self.plane.size,
            "NNN indexed picture worker result scope"
        );
        match self.stage {
            0 => {
                let key = self.plane.indices[0];
                let copy = ImageEffect::ByteRaster {
                    spec: ByteRaster::LinearChannelTransfer {
                        spec: LinearChannelTransfer {
                            source_channel: 0,
                            destination_channel: 3,
                            source_offset: 0,
                            destination_offset: 0,
                            length: self.plane.indices.len() as u64,
                        },
                    },
                };
                let input = ImageInput::Process {
                    input: Box::new(ImageInput::Raster {
                        destination: Box::new(ImageInput::Resource(image.clone())),
                        source: Box::new(ImageInput::Resource(image)),
                    }),
                    effects: serde_json::to_vec(&[copy])?.into(),
                };
                let mut effects = vec![ImageEffect::AlphaLookup {
                    table: (0..=255).map(|i| if i == key { 0 } else { 255 }).collect(),
                }];
                effects.extend(self.plane.effects()?);
                self.job = Some(
                    host.image_metadata(self.client)?
                        .begin_process(input, serde_json::to_vec(&effects)?.into())?,
                );
                self.api_calls += 1;
                self.stage = 1;
                Ok(None)
            }
            1 => {
                self.keyed = Some(image.clone());
                self.job = Some(host.image_metadata(self.client)?.begin_process(
                    ImageInput::Resource(image),
                    serde_json::to_vec(&[ImageEffect::OverrideAlpha { alpha: 255 }])?.into(),
                )?);
                self.api_calls += 1;
                self.stage = 2;
                Ok(None)
            }
            2 => {
                let ready = IndexedViews {
                    indices: self.plane.clone(),
                    opaque: image,
                    keyed: self
                        .keyed
                        .as_ref()
                        .context("NNN indexed key plane absent")?
                        .clone(),
                };
                self.ready = Some(ready.clone());
                self.job = None;
                Ok(Some(ready))
            }
            _ => anyhow::bail!("NNN indexed picture stage"),
        }
    }
}

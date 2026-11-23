// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CPicture's mask pointer/capacity is distinct from CheckMaskExist. Loading an
//! RGB-only picture retains prior mask bytes; a smaller masked load overwrites
//! only its prefix. Leases and linear channel work belong to shared Image jobs.
use anyhow::{Result, bail, ensure};
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
pub(crate) struct MaskStorage {
    pub buffer: Arc<dyn ImageResource>,
    /// Current picture RGB with the retained mask byte stream attached to A.
    pub view: Arc<dyn ImageResource>,
}
impl MaskStorage {
    pub(crate) fn capacity(&self) -> u64 {
        let size = self.buffer.size();
        u64::from(size[0]) * u64::from(size[1])
    }
}
pub(crate) struct Preparation {
    client: ClientId,
    raw: Arc<dyn ImageResource>,
    has_mask: bool,
    old: Option<MaskStorage>,
    job: Option<ImageJob>,
    ready: Option<Option<MaskStorage>>,
    pub api_calls: u64,
}
impl Preparation {
    pub(crate) fn new(
        client: ClientId,
        raw: Arc<dyn ImageResource>,
        has_mask: bool,
        old: Option<MaskStorage>,
    ) -> Result<Self> {
        ensure!(
            raw.client() == client
                && old
                    .as_ref()
                    .is_none_or(|m| m.buffer.client() == client && m.view.client() == client),
            "NNN retained mask client scope"
        );
        Ok(Self {
            client,
            raw,
            has_mask,
            old,
            job: None,
            ready: None,
            api_calls: 0,
        })
    }
    pub(crate) fn poll(
        &mut self,
        host: &mut dyn HostServices,
    ) -> Result<Option<Option<MaskStorage>>> {
        if let Some(ready) = &self.ready {
            return Ok(Some(ready.clone()));
        }
        let size = self.raw.size();
        let count = u64::from(size[0]) * u64::from(size[1]);
        if self.job.is_none() {
            match (&self.old, self.has_mask) {
                (None, false) => {
                    self.ready = Some(None);
                    return Ok(Some(None));
                }
                (None, true) => {
                    let ready = Some(MaskStorage {
                        buffer: self.raw.clone(),
                        view: self.raw.clone(),
                    });
                    self.ready = Some(ready.clone());
                    return Ok(Some(ready));
                }
                (Some(old), true) if count >= old.capacity() => {
                    let ready = Some(MaskStorage {
                        buffer: self.raw.clone(),
                        view: self.raw.clone(),
                    });
                    self.ready = Some(ready.clone());
                    return Ok(Some(ready));
                }
                _ => {}
            }
            let old = self.old.as_ref().unwrap();
            let (destination, source, length) = if self.has_mask {
                (
                    ImageInput::Resource(old.buffer.clone()),
                    ImageInput::Resource(self.raw.clone()),
                    count,
                )
            } else {
                // Source reads past a retained smaller mask allocation were
                // undefined. Define only that tail0; valid retained bytes keep
                // their original LINEAR order across changing picture widths.
                (
                    ImageInput::Process {
                        input: Box::new(ImageInput::Resource(self.raw.clone())),
                        effects: serde_json::to_vec(&[ImageEffect::OverrideAlpha { alpha: 0 }])?
                            .into(),
                    },
                    ImageInput::Resource(old.buffer.clone()),
                    count.min(old.capacity()),
                )
            };
            let spec = ByteRaster::LinearChannelTransfer {
                spec: LinearChannelTransfer {
                    source_channel: 3,
                    destination_channel: 3,
                    source_offset: 0,
                    destination_offset: 0,
                    length,
                },
            };
            self.job = Some(host.image_metadata(self.client)?.begin_process(
                ImageInput::Raster {
                    destination: Box::new(destination),
                    source: Box::new(source),
                },
                serde_json::to_vec(&[ImageEffect::ByteRaster { spec }])?.into(),
            )?);
            self.api_calls += 1;
        }
        match self.job.as_mut().unwrap().poll()? {
            ImageProcessState::Pending => Ok(None),
            ImageProcessState::Failed(error) => bail!("NNN retained mask image job: {error}"),
            ImageProcessState::Ready(image) => {
                ensure!(
                    image.client() == self.client,
                    "NNN retained mask foreign image"
                );
                let ready = Some(if self.has_mask {
                    MaskStorage {
                        buffer: image,
                        view: self.raw.clone(),
                    }
                } else {
                    MaskStorage {
                        buffer: self.old.as_ref().unwrap().buffer.clone(),
                        view: image,
                    }
                });
                self.ready = Some(ready.clone());
                Ok(Some(ready))
            }
        }
    }
}

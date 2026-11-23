// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Constructor-owned optional native effect pictures. Scoped source reads
//! precede this owner-polled shared decode chain; no slot or VM receipt is
//! fabricated, and absence remains a distinct successful constructor result.
use crate::{
    image_bridge::{NativePictureViews, PicturePreparation},
    resource_bridge::NativeImageSource,
};
use anyhow::{Context, Result};
use crysirror_core::{service::HostServices, state::ClientId};
use crysirror_tasks::TaskPool;
pub(crate) struct Preparation {
    source: Option<NativeImageSource>,
    decode: Option<PicturePreparation>,
    ready: Option<Option<NativePictureViews>>,
}
impl Preparation {
    pub(crate) fn new(source: Option<NativeImageSource>) -> Self {
        let ready = source.is_none().then_some(None);
        Self {
            source,
            decode: None,
            ready,
        }
    }
    pub(crate) fn poll(
        &mut self,
        client: ClientId,
        pool: &TaskPool,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        if self.ready.is_some() {
            return Ok(true);
        }
        if self.decode.is_none() {
            let source = self
                .source
                .take()
                .context("NNN effect asset decode source absent")?;
            self.decode = Some(PicturePreparation::detached(client, source, pool, host)?);
        }
        let Some(picture) = self.decode.as_mut().unwrap().poll_detached(host)? else {
            return Ok(false);
        };
        self.ready = Some(Some(picture));
        self.decode = None;
        Ok(true)
    }
    pub(crate) fn finish(self) -> Result<Option<NativePictureViews>> {
        self.ready
            .context("NNN effect asset constructor is still pending")
    }
}

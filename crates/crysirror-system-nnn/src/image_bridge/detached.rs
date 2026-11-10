// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native CPicture views for mode-owned images outside the effect slot bank.
//! They use the same engine decode/alpha/coverage-query dependency chain as
//! scripted pictures, without publishing a slot or fabricating a VM receipt.
use super::*;
#[derive(Clone)]
pub(crate) struct NativePictureViews {
    pub has_mask: bool,
    pub source_clip: [i32; 4],
    pub raw: Arc<dyn ImageResource>,
    pub opaque: Arc<dyn ImageResource>,
    pub keyed: Arc<dyn ImageResource>,
}
impl PicturePreparation {
    pub(crate) fn detached(
        client: ClientId,
        source: crate::resource_bridge::NativeImageSource,
        pool: &TaskPool,
        host: &mut dyn HostServices,
    ) -> Result<Self> {
        ensure!(client.0 != 0, "NNN detached picture client identity");
        let mut clip =
            crate::native_picture_preparation_clip::PictureClipPreparation::new(pool.clone());
        clip.configure(source.has_mask, source.clip_insets)?;
        let job = host
            .image_metadata(client)?
            .begin_process(source.input, b"[]".as_slice().into())?;
        Ok(Self {
            client,
            // The detached wrapper never exposes this internal staging tag as
            // a picture-bank identity; poll_detached returns only image views.
            slot: -1,
            preload: true,
            name: Vec::new(),
            task: None,
            job: Some(job),
            raw: None,
            stage: 0,
            opaque: None,
            has_mask: Some(source.has_mask),
            clip,
            ready: None,
            api_calls: 1,
        })
    }
    pub(crate) fn poll_detached(
        &mut self,
        host: &mut dyn HostServices,
    ) -> Result<Option<NativePictureViews>> {
        Ok(self.poll(host)?.map(|picture| NativePictureViews {
            has_mask: picture.has_mask,
            source_clip: picture.source_clip,
            raw: picture.raw,
            opaque: picture.opaque,
            keyed: picture.keyed,
        }))
    }
}

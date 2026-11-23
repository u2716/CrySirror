// SPDX-License-Identifier: MPL-2.0
//! Native four-pixel bitmap allocation/row guards over shared worker images.
//! These are stored resource properties, distinct from opacity or source UVs.
use super::bitmap_read::{BitmapFormat, BitmapRead};
use super::*;
use crysirror_core::{
    image_encoding::{ImageEncodingInfoQuery, ImageEncodingInfoState},
    image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource},
};
pub(super) struct NativeBitmap {
    read: BitmapRead,
    format: BitmapFormat,
    encoding: Option<Box<dyn ImageEncodingInfoQuery>>,
    padding: Option<ImageJob>,
}
impl NativeBitmap {
    pub(super) fn set_encoding(&mut self, fingerprint: [u8; 32], alpha: bool) {
        self.read.set_fingerprint(fingerprint);
        self.format.alpha = Some(alpha);
    }
    pub(super) fn new(read: BitmapRead, format: BitmapFormat) -> Self {
        Self {
            read,
            format,
            encoding: None,
            padding: None,
        }
    }
    pub(super) fn poll(
        &mut self,
        client: ClientId,
        pool: &TaskPool,
        images: &Arc<dyn ImageMetadata>,
    ) -> Result<Option<Arc<dyn ImageResource>>> {
        if let Some(job) = &mut self.padding {
            return match job.poll()? {
                ImageProcessState::Pending => Ok(None),
                ImageProcessState::Failed(error) => bail!("N2System bitmap padding: {error}"),
                ImageProcessState::Ready(image) => Ok(Some(image)),
            };
        }
        let Some(image) = self.read.poll(client, pool, images)? else {
            return Ok(None);
        };
        let [w, h] = image.size();
        let padded = (w + 3) & !3;
        if padded == w {
            return Ok(Some(image));
        }
        if self.format.alpha.is_none() && !image.opaque() {
            self.format.alpha = Some(true);
        }
        if self.format.alpha.is_none() {
            if self.encoding.is_none() {
                let origin = self
                    .format
                    .origin
                    .as_ref()
                    .context("N2System bitmap guard columns need original encoded metadata")?;
                match images.begin_encoding_info(origin) {
                    Ok(query) => self.encoding = Some(query),
                    Err(Error::Limit("encoded image metadata task capacity")) => return Ok(None),
                    Err(e) => return Err(e.into()),
                }
            }
            match self.encoding.as_mut().unwrap().poll()? {
                ImageEncodingInfoState::Pending => return Ok(None),
                ImageEncodingInfoState::Failed(error) => return Err(error.into()),
                ImageEncodingInfoState::Ready(receipt) => {
                    ensure!(
                        receipt.client == client && receipt.encoding.size == [w, h],
                        "N2System native bitmap encoding identity"
                    );
                    self.format.alpha = Some(
                        receipt
                            .encoding
                            .has_alpha_plane
                            .context("N2System native bitmap alpha-plane metadata")?,
                    );
                    self.encoding = None;
                }
            }
        }
        let fill = [
            0,
            0,
            0,
            if self.format.alpha == Some(false) {
                255
            } else {
                0
            },
        ];
        let effects = Arc::from(serde_json::to_vec(
            &json!([{"kind":"canvas_margins","margins":[0,0,padded-w,0],"fill":fill}]),
        )?);
        match images.begin_process(ImageInput::Resource(image), effects) {
            Ok(job) => self.padding = Some(job),
            Err(Error::Limit("image preparation task capacity")) => {}
            Err(e) => return Err(e.into()),
        }
        Ok(None)
    }
}
impl N2SystemVm {
    pub(super) fn read_native_bitmap(
        &self,
        host: &mut dyn HostServices,
        current: u32,
        name: &str,
    ) -> Result<Option<NativeBitmap>> {
        if name.eq_ignore_ascii_case("SCREEN") {
            let capture = match host.begin_frame_capture(self.client) {
                Ok(job) => job,
                Err(Error::Limit("Canvas capture jobs")) => return Ok(None),
                Err(e) => return Err(e.into()),
            };
            return Ok(Some(NativeBitmap::new(
                BitmapRead::from_job(capture),
                BitmapFormat {
                    origin: None,
                    alpha: Some(false),
                },
            )));
        }
        let (id, layer): (u32, crysirror_scene::layer::LayerSpec) = if let Ok(asset) =
            resource::resolve(self.catalog.as_ref().unwrap(), name)
        {
            (
                current,
                serde_json::from_value(
                    json!({"id":current,"rect":[0,0,1,1],"paint":{"kind":"image","asset":asset}}),
                )?,
            )
        } else {
            let id = self
                .query(current, name)?
                .first()
                .copied()
                .context("N2System native bitmap source")?;
            if self.shades.get(&id).is_some_and(shade::Shade::pending) {
                return Ok(None);
            }
            let layer = if let Kind::Image { paint, size } = &self.entities.nodes[&id].kind {
                serde_json::from_value(json!({"id":id,"rect":[0,0,size[0],size[1]],"paint":paint}))?
            } else {
                serde_json::from_value(self.layer(host, id)?)?
            };
            (id, layer)
        };
        self.read_native_bitmap_layer(host, id, &layer)
    }
    pub(super) fn read_native_bitmap_layer(
        &self,
        host: &mut dyn HostServices,
        id: u32,
        layer: &crysirror_scene::layer::LayerSpec,
    ) -> Result<Option<NativeBitmap>> {
        let paint = serde_json::to_value(&layer.paint)?;
        let format = self
            .bitmap_formats
            .get(&id)
            .cloned()
            .unwrap_or_else(|| self.bitmap_format(&paint));
        let mut read = match BitmapRead::new(self, host, id, layer) {
            Ok(read) => read,
            Err(error)
                if matches!(
                    error.downcast_ref::<Error>(),
                    Some(Error::Limit("Canvas capture jobs"))
                ) =>
            {
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        if self.effects.contains_key(&id) {
            read.crop_view(
                layer.uv.unwrap_or([0., 0., 1., 1.]),
                self.entities.nodes[&id].size.map(|v| v as u32),
            )?;
        }
        Ok(Some(NativeBitmap::new(read, format)))
    }
}

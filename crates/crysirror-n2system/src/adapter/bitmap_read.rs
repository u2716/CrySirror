// SPDX-License-Identifier: MPL-2.0
//! Bounded asynchronous bitmap leases for native destructive operations.
use super::*;
use crysirror_core::image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource};
#[derive(Clone, Default)]
pub(super) struct BitmapFormat {
    pub origin: Option<String>,
    pub alpha: Option<bool>,
}
impl N2SystemVm {
    pub(super) fn bitmap_format(&self, paint: &Json) -> BitmapFormat {
        if paint["kind"] == "solid" {
            return BitmapFormat {
                origin: None,
                alpha: Some(false),
            };
        }
        if let Some(asset) = paint["asset"].as_str() {
            if let Some(catalog) = &self.catalog {
                if let Ok(path) = resource::resolve(catalog, asset) {
                    return BitmapFormat {
                        origin: Some(path),
                        alpha: None,
                    };
                }
            }
            if let Some((id, _)) = self
                .captured_images
                .iter()
                .find(|(_, path)| path.as_str() == asset)
            {
                if let Some(format) = self.bitmap_formats.get(id) {
                    return format.clone();
                }
            }
        }
        BitmapFormat::default()
    }
}
pub(super) struct BitmapRead {
    fingerprint: Option<[u8; 32]>,
    input: Option<ImageInput>,
    path: Option<String>,
    metadata: Option<Task<ImageInput>>,
    job: Option<ImageJob>,
}
impl BitmapRead {
    pub(super) fn from_job(job: ImageJob) -> Self {
        Self {
            input: None,
            path: None,
            metadata: None,
            job: Some(job),
            fingerprint: None,
        }
    }
    pub(super) fn new(
        vm: &N2SystemVm,
        host: &mut dyn HostServices,
        id: u32,
        layer: &crysirror_scene::layer::LayerSpec,
    ) -> Result<Self> {
        let mut read = Self {
            fingerprint: None,
            input: None,
            path: None,
            metadata: None,
            job: None,
        };
        if vm.surfaces.contains_key(&id) {
            read.job = Some(vm.begin_surface_image(host, id)?);
            return Ok(read);
        }
        match &layer.paint {
            crysirror_scene::layer::Paint::Image { asset, .. } => {
                if let Some(image) = host.published_image_resource(vm.client, asset)? {
                    read.input = Some(ImageInput::Resource(image));
                } else {
                    read.path = Some(asset.clone());
                }
            }
            crysirror_scene::layer::Paint::Solid { color } => {
                let size = vm.entities.nodes[&id].size.map(|v| v.ceil() as u32);
                crysirror_image::rgba_len(size[0], size[1])?;
                read.input = Some(ImageInput::Solid {
                    size,
                    color: *color,
                });
            }
            _ => bail!("N2System operation requires a bitmap resource"),
        }
        Ok(read)
    }
    pub(super) fn set_fingerprint(&mut self, fingerprint: [u8; 32]) {
        self.fingerprint = Some(fingerprint);
    }
    /// Plain stores a canvas capture plus a source view in the shared layer;
    /// native bitmap operations see only its actual local bitmap rectangle.
    pub(super) fn crop_view(&mut self, uv: [f32; 4], extent: [u32; 2]) -> Result<()> {
        let Some(ImageInput::Resource(image)) = &self.input else {
            bail!("N2System Plain bitmap requires an immutable capture lease");
        };
        let size = image.size();
        ensure!(
            uv.iter().all(|v| v.is_finite() && (0. ..=1.).contains(v)),
            "N2System Plain bitmap UV bounds"
        );
        let x = (uv[0] * size[0] as f32).round() as u32;
        let y = (uv[1] * size[1] as f32).round() as u32;
        ensure!(
            extent.iter().all(|n| *n > 0)
                && u64::from(x) + u64::from(extent[0]) <= u64::from(size[0])
                && u64::from(y) + u64::from(extent[1]) <= u64::from(size[1]),
            "N2System Plain bitmap source view bounds"
        );
        let effects = Arc::from(serde_json::to_vec(
            &json!([{"kind":"rect_map","spec":{"output_size":extent,"clear":[0,0,0,0],"regions":[{"source":[x,y,extent[0],extent[1]],"destination":[0,0,extent[0],extent[1]]}],"sample":"floor","copy_alpha":true}}]),
        )?);
        self.input = Some(ImageInput::Process {
            input: Box::new(self.input.take().unwrap()),
            effects,
        });
        Ok(())
    }
    pub(super) fn poll(
        &mut self,
        client: ClientId,
        pool: &TaskPool,
        images: &Arc<dyn ImageMetadata>,
    ) -> Result<Option<Arc<dyn ImageResource>>> {
        if let Some(ImageInput::Resource(image)) = &self.input {
            return Ok(Some(image.clone()));
        }
        if let Some(task) = &mut self.metadata {
            let Some(input) = task.try_take()? else {
                return Ok(None);
            };
            self.input = Some(input);
            self.metadata = None;
        }
        if self.input.is_none() && self.job.is_none() {
            let path = self
                .path
                .as_ref()
                .context("N2System bitmap source")?
                .clone();
            let images = images.clone();
            match pool.spawn(move |cancel| {
                cancel.check()?;
                let size = images.size(&path)?;
                cancel.check()?;
                Ok(ImageInput::Asset {
                    path,
                    size,
                    fingerprint: None,
                })
            }) {
                Ok(task) => self.metadata = Some(task),
                Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => {}
                Err(e) => return Err(e),
            }
            return Ok(None);
        }
        if self.job.is_none() {
            if let Some(ImageInput::Asset { fingerprint, .. }) = &mut self.input {
                *fingerprint = self.fingerprint;
            }
            match images.begin_process(self.input.as_ref().unwrap().clone(), Arc::from(&b"[]"[..]))
            {
                Ok(job) => self.job = Some(job),
                Err(Error::Limit("image preparation task capacity")) => return Ok(None),
                Err(e) => return Err(e.into()),
            }
        }
        match self.job.as_mut().unwrap().poll()? {
            ImageProcessState::Pending => Ok(None),
            ImageProcessState::Failed(error) => bail!("N2System bitmap read: {error}"),
            ImageProcessState::Ready(image) => {
                ensure!(image.client() == client, "N2System bitmap lease owner");
                self.input = Some(ImageInput::Resource(image.clone()));
                self.job = None;
                Ok(Some(image))
            }
        }
    }
}

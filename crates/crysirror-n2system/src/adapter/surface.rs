// SPDX-License-Identifier: MPL-2.0
//! Totono trial1.0.0.0: CreateSurface40D820 and SetSurface4132D0 ->
//! 44BAE0 ->457140. Destination is sprite[48], not entity parent5200.
//! The shared presenter retains bitmap contents and the ordinary painter order.
use super::*;
use crysirror_core::image_processing::{ImageInput, ImageJob, ImageProcess, ImageProcessState};
use crysirror_scene::layer::render_target::RenderTargetSpec as TargetSpec;

/// An RGB bitmap has no meaningful stored alpha. Normalize that representation
/// on shared workers when converting a native bitmap into an RGBA image lease.
struct BitmapRead {
    client: ClientId,
    size: [u32; 2],
    raw: ImageJob,
    opaque: bool,
    images: Arc<dyn ImageMetadata>,
    correction: Option<ImageJob>,
    state: ImageProcessState,
}
impl ImageProcess for BitmapRead {
    fn poll(&mut self) -> crysirror_core::Result<ImageProcessState> {
        if !matches!(self.state, ImageProcessState::Pending) {
            return Ok(self.state.clone());
        }
        let result = if let Some(job) = &mut self.correction {
            job.poll()?
        } else {
            self.raw.poll()?
        };
        match result {
            ImageProcessState::Ready(image) => {
                if image.client() != self.client || image.size() != self.size {
                    return Err(Error::Invalid("N2System surface image identity"));
                }
                if self.opaque && self.correction.is_none() {
                    match self.images.begin_process(
                        ImageInput::Resource(image),
                        Arc::from(&b"[{\"kind\":\"override_alpha\",\"alpha\":255}]"[..]),
                    ) {
                        Ok(job) => self.correction = Some(job),
                        Err(Error::Limit("image preparation task capacity")) => {}
                        Err(error) => return Err(error),
                    }
                } else {
                    self.state = ImageProcessState::Ready(image);
                }
            }
            ImageProcessState::Failed(error) => self.state = ImageProcessState::Failed(error),
            ImageProcessState::Pending => {}
        }
        Ok(self.state.clone())
    }
}

impl N2SystemVm {
    pub(super) fn begin_surface_image(
        &self,
        host: &mut dyn HostServices,
        id: u32,
    ) -> Result<ImageJob> {
        let spec = self
            .surfaces
            .get(&id)
            .context("N2System surface image source")?;
        Ok(Box::new(BitmapRead {
            client: self.client,
            size: spec.size,
            raw: host.begin_layer_target_image(self.client, id)?,
            opaque: spec.opaque,
            images: self
                .images
                .as_ref()
                .context("N2System surface image workers")?
                .clone(),
            correction: None,
            state: ImageProcessState::Pending,
        }))
    }
    pub(super) fn surface_source(&self, current: u32, name: &str) -> Result<Option<u32>> {
        if name.is_empty()
            || resource::resolve(
                self.catalog.as_ref().context("N2System surface catalog")?,
                name,
            )
            .is_ok()
        {
            return Ok(None);
        }
        Ok(self
            .query(current, name)?
            .first()
            .copied()
            .filter(|id| self.surfaces.contains_key(id)))
    }
    pub(super) fn create_surface(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        args: &Arguments<'_>,
    ) -> Result<()> {
        // Original491670 allocates zero-filled trial1024x576 backing. Retail
        // uses the configured native canvas; false selects RGB flags8 and true
        // alpha flags277. This flag changes sampling, not layer opacity.
        let spec = TargetSpec {
            size: self.config.canvas,
            clear: [0; 4],
            initial: None,
            opaque: !args.truth(4)?,
        };
        spec.validate()?;
        let id = self.visual(
            host,
            current,
            args,
            json!({"kind":"solid","color":[0,0,0,0]}),
            spec.size,
            None,
        )?;
        self.submit(
            host,
            vec![json!({"op":"create_render_target","id":id,"spec":spec})],
            vec![],
        )?;
        self.surfaces.insert(id, spec);
        self.bitmap_formats.insert(id,bitmap_read::BitmapFormat{origin:None,alpha:Some(args.truth(4)?)});
        Ok(())
    }
    pub(super) fn set_surface(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        state: &mut Execution,
        args: &Arguments<'_>,
    ) -> Result<bool> {
        // Native450740 does a single destination lookup. A missing destination
        // queues detachment; an existing non-bitmap destination is rejected.
        let name = args.string(1)?;
        let target = if name.is_empty() {
            None
        } else {
            self.query(current, &name)?.first().copied()
        };
        if let Some(id) = target {
            if self.shades.get(&id).is_some_and(shade::Shade::pending) {
                return Ok(false);
            }
        }
        if state
            .surface_preparation
            .as_ref()
            .is_some_and(|(id, _)| Some(*id) != target)
        {
            // Deletion/replacement by another NSS thread invalidates the
            // pending identity. Dropping the task cancels its old completion.
            state.surface_preparation = None;
        }
        let mut sources = BTreeSet::new();
        for root in self.query(current, &args.string(0)?)? {
            sources.extend(self.entities.bitmap_targets(root));
        }
        let mut commands = Vec::new();
        let mut promoted = None;
        if let Some(id) = target {
            if !self.surfaces.contains_key(&id) {
                ensure!(
                    !self.pages.contains_key(&id)
                        && !self.frame_animations.contains_key(&id)
                        && !self.alpha_masks.contains_key(&id)
                        && !self.effects.contains_key(&id),
                    "N2System SetSurface target requires a retained bitmap"
                );
                let entity = &self.entities.nodes[&id];
                let (paint, size) = match &entity.kind {
                    Kind::Image { paint, size } => (paint.clone(), *size),
                    Kind::Visual => {
                        let layer = self.layer(host, id)?;
                        let size = entity.size.map(|n| n.ceil() as u32);
                        (layer["paint"].clone(), size)
                    }
                    _ => bail!("N2System SetSurface destination has no bitmap"),
                };
                let paint: crysirror_scene::layer::Paint = serde_json::from_value(paint)?;
                let spec = match paint {
                    crysirror_scene::layer::Paint::Image { asset, .. } => {
                        // The sprite's visible rect can be a clipped view;
                        // native sprite+48 addresses the whole bitmap, not
                        // that view's dimensions. Resolve its actual backing.
                        if let Some(image) = host.published_image_resource(self.client, &asset)? {
                            TargetSpec {
                                size: image.size(),
                                clear: [0; 4],
                                initial: Some(asset),
                                opaque: false,
                            }
                        } else if let Some((_, task)) = state.surface_preparation.as_mut() {
                            let Some(spec) = task.try_take()? else {
                                return Ok(false);
                            };
                            state.surface_preparation = None;
                            ensure!(
                                spec.initial.as_deref() == Some(asset.as_str()),
                                "N2System surface bitmap changed during preparation"
                            );
                            spec
                        } else {
                            let images = self
                                .images
                                .as_ref()
                                .context("N2System image metadata")?
                                .clone();
                            match self
                                .pool
                                .as_ref()
                                .context("N2System surface task pool")?
                                .spawn(move |cancel| {
                                    cancel.check()?;
                                    let size = images.size(&asset)?;
                                    cancel.check()?;
                                    let spec = TargetSpec {
                                        size,
                                        clear: [0; 4],
                                        initial: Some(asset),
                                        opaque: false,
                                    };
                                    spec.validate()?;
                                    Ok(spec)
                                }) {
                                Ok(task) => state.surface_preparation = Some((id, task)),
                                Err(error) if error.is::<crysirror_tasks::CapacityExceeded>() => {}
                                Err(error) => return Err(error),
                            }
                            return Ok(false);
                        }
                    }
                    crysirror_scene::layer::Paint::Solid { color } => TargetSpec {
                        size,
                        clear: color,
                        initial: None,
                        opaque: color[3] == 255,
                    },
                    _ => bail!("N2System SetSurface destination is not a bitmap"),
                };
                spec.validate()?;
                commands.push(json!({"op":"create_render_target","id":id,"spec":spec}));
                promoted = Some((id, spec));
            }
        }
        for id in sources {
            let Some(entity) = self.entities.nodes.get(&id) else {
                continue;
            };
            if !matches!(entity.kind, Kind::Image { .. } | Kind::Visual) {
                continue;
            }
            // CPU in-place same-bitmap feedback has not been established as
            // equivalent to a GPU draw-boundary copy. Never silently substitute.
            ensure!(
                target != Some(id),
                "N2System same-bitmap SetSurface feedback needs its native overlap profile"
            );
            let layer = self.layer(host, id)?;
            if layer["paint"]["kind"] == "group" && matches!(entity.kind, Kind::Visual) {
                continue;
            }
            commands.push(json!({"op":"set_output","id":id,"target":target}));
        }
        self.submit(host, commands, vec![])?;
        if let Some((id, spec)) = promoted {
            self.surfaces.insert(id, spec);
        }
        Ok(true)
    }
}

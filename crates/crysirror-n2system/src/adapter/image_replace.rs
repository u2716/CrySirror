// SPDX-License-Identifier: MPL-2.0
//! SetImage: original Totono trial 4131C0 -> 44B9A0 -> 456E40.
//! Replacement preserves position, transform, tint and retained motion. Native
//! bitmap construction resets dimensions/UVs; old bitmap shade state is lost.
use super::*;
use crysirror_core::image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource};

pub(super) struct Replacement {
    format: bitmap_read::BitmapFormat,
    targets: Vec<u32>,
    source: Option<ImageInput>,
    asset: Option<String>,
    metadata: Option<Task<ImageInput>>,
    job: Option<ImageJob>,
}
impl Replacement {
    fn poll(
        &mut self,
        client: ClientId,
        pool: &TaskPool,
        images: &Arc<dyn ImageMetadata>,
    ) -> Result<Option<Arc<dyn ImageResource>>> {
        if let Some(ImageInput::Resource(image)) = &self.source {
            return Ok(Some(image.clone()));
        }
        if let Some(task) = &mut self.metadata {
            let Some(input) = task.try_take()? else {
                return Ok(None);
            };
            self.source = Some(input);
            self.metadata = None;
        }
        if self.source.is_none() && self.job.is_none() {
            let path = self
                .asset
                .as_ref()
                .context("N2System replacement source")?
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
            match images.begin_process(self.source.as_ref().unwrap().clone(), Arc::from(&b"[]"[..]))
            {
                Ok(job) => self.job = Some(job),
                Err(Error::Limit("image preparation task capacity")) => return Ok(None),
                Err(e) => return Err(e.into()),
            }
        }
        match self.job.as_mut().unwrap().poll()? {
            ImageProcessState::Pending => Ok(None),
            ImageProcessState::Failed(error) => bail!("N2System SetImage: {error}"),
            ImageProcessState::Ready(image) => {
                ensure!(
                    image.client() == client,
                    "N2System foreign replacement image"
                );
                Ok(Some(image))
            }
        }
    }
}
impl N2SystemVm {
    pub(super) fn native_set_image(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        state: &mut Execution,
        args: &Arguments<'_>,
    ) -> Result<bool> {
        if state.image_replace.is_none() {
            let name = args.string(1)?;
            let mut replacement = Replacement {
                format:bitmap_read::BitmapFormat::default(),
                targets: {
                    // Original 4511F0 applies the same callback to direct
                    // drawable children only when propagation is enabled.
                    let mut targets = BTreeSet::new();
                    for root in self.query(current, &args.string(0)?)? {
                        targets.extend(self.entities.bitmap_targets(root));
                    }
                    targets.into_iter().collect()
                },
                source: None,
                asset: None,
                metadata: None,
                job: None,
            };
            if name.eq_ignore_ascii_case("SCREEN") {
                replacement.format.alpha=Some(false);
                replacement.job = Some(host.begin_frame_capture(self.client)?);
            } else if let Some(id) = self.surface_source(current,&name)? {
                replacement.format.alpha=Some(!self.surfaces[&id].opaque);
                if self.shades.get(&id).is_some_and(shade::Shade::pending) {return Ok(false);}
                replacement.job = Some(self.begin_surface_image(host,id)?);
            } else {
                // A pending destructive write owns its source bitmap until
                // publication. Do not replace it with a stale unshaded input.
                if resource::resolve(self.catalog.as_ref().unwrap(), &name).is_err()
                    && self
                        .query(current, &name)?
                        .iter()
                        .any(|id| self.shades.get(id).is_some_and(shade::Shade::pending))
                {
                    return Ok(false);
                }
                let image = self
                    .source_image(current, &name)?
                    .context("N2System SetImage source missing")?;
                replacement.format=self.bitmap_format(&image.paint);
                let paint: crysirror_scene::layer::Paint = serde_json::from_value(image.paint)?;
                match paint {
                    crysirror_scene::layer::Paint::Solid { color } => {
                        replacement.source = Some(ImageInput::Solid {
                            size: image.size,
                            color,
                        });
                    }
                    crysirror_scene::layer::Paint::Image { asset, .. } => {
                        if let Some(image) = host.published_image_resource(self.client, &asset)? {
                            replacement.source = Some(ImageInput::Resource(image));
                        } else {
                            replacement.asset = Some(asset);
                        }
                    }
                    _ => bail!("N2System SetImage source is not a bitmap"),
                }
            }
            state.image_replace = Some(replacement);
        }
        let replacement = state.image_replace.as_mut().unwrap();
        let Some(image) = replacement.poll(
            self.client,
            self.pool.as_ref().unwrap(),
            self.images.as_ref().unwrap(),
        )?
        else {
            return Ok(false);
        };
        let size = image.size();
        let mut targets = Vec::new();
        for &id in &replacement.targets {
            let Some(entity) = self.entities.nodes.get(&id) else {
                continue;
            };
            if !matches!(entity.kind, Kind::Image { .. } | Kind::Visual) {
                continue;
            }
            ensure!(
                !self.pages.contains_key(&id)
                    && !self.frame_animations.contains_key(&id)
                    && !self.backlogs.contains_key(&id)
                    && !self.alpha_masks.contains_key(&id)
                    && !self.effects.contains_key(&id),
                "N2System SetImage requires an ordinary bitmap/sprite"
            );
            let layer = self.layer(host, id)?;
            let bitmap = matches!(entity.kind, Kind::Image { .. });
            if !bitmap && !self.surfaces.contains_key(&id) && !matches!(layer["paint"]["kind"].as_str(), Some("image" | "solid")) {
                // Original 456E40 explicitly ignores nodes with no bitmap.
                continue;
            }
            targets.push((id, bitmap, layer));
        }
        // Reserve immutable aliases before changing any target. Aliases share
        // one physical resource; replacing the source's own name is safe.
        let mut published = Vec::new();
        for (id, _, _) in &targets {
            match host.publish_image_resource(self.client, *id, image.clone()) {
                Ok(uploaded) => published.push(uploaded.path.relative().to_owned()),
                Err(error) => {
                    for path in published {
                        host.release_image_resource(self.client, &path)?;
                    }
                    return Err(error.into());
                }
            }
        }
        let mut changes = Vec::new();
        let mut surface_updates = Vec::new();
        for ((id, bitmap, layer), path) in targets.iter().zip(&published) {
            if self.surfaces.contains_key(id) {
                let spec=crysirror_scene::layer::render_target::RenderTargetSpec {size,clear:[0;4],initial:Some(path.clone()),opaque:image.opaque()};
                changes.push(json!({"op":"create_render_target","id":id,"spec":spec,"reset_view":true}));
                surface_updates.push((*id,spec));
            } else if !bitmap {
                // A single atomic batch clears the old clip/repeat view, then
                // installs the new image. Both SetPaint operations preserve
                // motion/producer clocks, unlike replacing the whole layer.
                changes.push(json!({"op":"set_paint","id":id,"paint":{"kind":"group"}}));
                changes
                    .push(json!({"op":"set_paint","id":id,"paint":{"kind":"image","asset":path}}));
            }
            changes.push(json!({"op":"set_rect","id":id,"rect":[layer["rect"][0],layer["rect"][1],size[0],size[1]]}));
        }
        if let Err(error) = self.submit(host, changes, vec![]) {
            for path in published {
                host.release_image_resource(self.client, &path)?;
            }
            return Err(error);
        }
        for (id,spec) in surface_updates {self.surfaces.insert(id,spec);}
        let format=state.image_replace.as_ref().unwrap().format.clone();
        for ((id, bitmap, _), path) in targets.into_iter().zip(published) {
            let entity = self.entities.nodes.get_mut(&id).unwrap();
            entity.size = size.map(|v| v as f32);
            if bitmap {
                entity.kind = Kind::Image {
                    paint: json!({"kind":"image","asset":path}),
                    size,
                };
            }
            self.remove_shade(host, id)?;
            self.remove_ripple(host,id,false)?;
            self.reset_blur_bitmap(host,id)?;
            self.framed_images.remove(&id);
            self.bitmap_formats.insert(id,format.clone());
            if let Some(old) = self.captured_images.insert(id, path) {
                host.release_image_resource(self.client, &old)?;
            }
        }
        state.image_replace = None;
        self.machine.complete(&mut state.thread, Value::Null)?;
        Ok(true)
    }
}

// SPDX-License-Identifier: MPL-2.0
//! Native SetShade: Totono trial 1.0.0.0, 413000/44B1B0/456C90/46F860.
//! Bitmap writes use the shared immutable worker filter; drawing stays retained.
//! Strengths are native byte sums, including collisions and explicit no-ops.
use super::*;
use crysirror_core::image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource};

pub(super) struct Shade {
    source_paint: Json,
    resource: bool,
    surface: bool,
    surface_readback: Option<ImageJob>,
    original_alias: Option<String>,
    source_uv: Option<[f32; 4]>,
    crop_extent: Option<[u32; 2]>,
    input: Option<ImageInput>,
    preparing: Option<Task<ImageInput>>,
    pending: Option<(u64, ImageJob)>,
    passes: u8,
    generation: u64,
    queued: std::collections::VecDeque<u8>,
    output: Option<String>,
}
impl Shade {
    pub(super) fn pending(&self) -> bool {
        self.surface_readback.is_some()
            || self.pending.is_some()
            || self.preparing.is_some()
            || !self.queued.is_empty()
    }
}
impl N2SystemVm {
    pub(super) fn set_shade(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        args: &Arguments<'_>,
    ) -> Result<()> {
        // The native parser stringifies constants before 44B1B0 hashes them.
        // Even unrecognised strings are accepted: 46F860 explicitly returns 0
        // without changing the bitmap. Do not turn them into guessed strengths.
        let strength = args.value(1).string()?;
        let (encoded, _, errors) = encoding_rs::SHIFT_JIS.encode(&strength);
        ensure!(
            !errors && encoded.len() <= 4096,
            "N2System shade strength encoding/length"
        );
        let hash: u32 = encoded
            .iter()
            .map(|&b| u32::from(if b >= 0x5a { b - 32 } else { b }))
            .sum();
        let passes = match hash {
            592 | 315 => 0,
            764 => 1,
            376 => 2,
            449 => 4,
            683 => 8,
            381 => 16,
            769 => 24,
            464 => 32,
            _ => return Ok(()),
        };
        let mut targets = BTreeSet::new();
        for root in self.query(current, &args.string(0)?)? {
            targets.extend(self.entities.bitmap_targets(root));
        }
        let mut restore = Vec::new();
        let mut release = Vec::new();
        let mut surface_updates = Vec::new();
        let mut restored_aliases = Vec::new();
        for id in targets {
            ensure!(!self.ripples.contains_key(&id),"N2System destructive shade during an active bitmap effect needs native arbitration");
            if !matches!(
                self.entities.nodes[&id].kind,
                Kind::Visual | Kind::Image { .. }
            ) {
                continue;
            }
            ensure!(
                !self.pages.contains_key(&id)
                    && !self.frame_animations.contains_key(&id)
                    && !self.backlogs.contains_key(&id)
                    && !self.alpha_masks.contains_key(&id)
                    && self.entities.nodes[&id].container.is_none(),
                "N2System SetShade requires a sprite or Plain effect"
            );
            if passes == 0 {
                if let Some(mut shade) = self.shades.remove(&id) {
                    if shade.surface {
                        if let Some(path) = shade.original_alias.take() {
                            let mut spec = self
                                .surfaces
                                .get(&id)
                                .context("N2System shaded surface owner")?
                                .clone();
                            spec.initial = Some(path.clone());
                            restore.push(json!({"op":"create_render_target","id":id,"spec":spec}));
                            surface_updates.push((id, spec));
                            restored_aliases.push((id, path));
                        }
                    } else if shade.resource {
                        let Kind::Image { paint, .. } =
                            &mut self.entities.nodes.get_mut(&id).unwrap().kind
                        else {
                            bail!("N2System shaded image changed identity");
                        };
                        *paint = shade.source_paint;
                    } else {
                        restore.push(json!({"op":"set_paint","id":id,"paint":shade.source_paint}));
                    }
                    if let Some(path) = shade.output {
                        release.push(path);
                    }
                    if let Some(path) = shade.original_alias {
                        release.push(path);
                    }
                }
                continue;
            }
            if let Some(shade) = self.shades.get_mut(&id) {
                ensure!(shade.queued.len() < 1024, "N2System shade operation budget");
                // Every invocation operates on the preceding result, even if
                // its strength matches. A queue preserves worker completion order.
                shade.passes = passes;
                shade.queued.push_back(passes);
                continue;
            }
            let surface = self.surfaces.contains_key(&id);
            let resource = matches!(self.entities.nodes[&id].kind, Kind::Image { .. });
            let layer: crysirror_scene::layer::LayerSpec = if let Kind::Image { paint, .. } =
                &self.entities.nodes[&id].kind
            {
                serde_json::from_value(
                    json!({"id":id,"paint":paint,"rect":[0.,0.,self.entities.nodes[&id].size[0],self.entities.nodes[&id].size[1]]}),
                )?
            } else {
                serde_json::from_value(self.layer(host, id)?)?
            };
            ensure!(
                matches!(
                    layer.paint,
                    crysirror_scene::layer::Paint::Image { .. }
                        | crysirror_scene::layer::Paint::Solid { .. }
                ),
                "N2System unsupported SetShade target paint"
            );
            // A uniform native bitmap remains uniform under the filter.
            if !surface && matches!(layer.paint, crysirror_scene::layer::Paint::Solid { .. }) {
                continue;
            }
            let paint = serde_json::to_value(&layer.paint)?;
            let surface_readback = if surface {
                Some(self.begin_surface_image(host, id)?)
            } else {
                None
            };
            let input = if surface {
                None
            } else {
                match &layer.paint {
                    crysirror_scene::layer::Paint::Solid { color } => Some(ImageInput::Solid {
                        size: [1, 1],
                        color: *color,
                    }),
                    crysirror_scene::layer::Paint::Image { asset, .. } => host
                        .published_image_resource(self.client, asset)?
                        .map(ImageInput::Resource),
                    _ => unreachable!(),
                }
            };
            self.shades.insert(
                id,
                Shade {
                    source_paint: paint,
                    resource,
                    surface,
                    surface_readback,
                    original_alias: None,
                    source_uv: layer.uv,
                    crop_extent: self
                        .effects
                        .contains_key(&id)
                        .then_some(self.entities.nodes[&id].size.map(|n| n as u32)),
                    input,
                    preparing: None,
                    pending: None,
                    passes,
                    generation: 1,
                    queued: std::collections::VecDeque::from([passes]),
                    output: None,
                },
            );
        }
        self.submit(host, restore, vec![])?;
        for (id, spec) in surface_updates {
            self.surfaces.insert(id, spec);
        }
        for (id, path) in restored_aliases {
            if let Some(old) = self.captured_images.insert(id, path.clone()) {
                if old != path {
                    release.push(old);
                }
            }
        }
        for path in release {
            host.release_image_resource(self.client, &path)?;
        }
        Ok(())
    }

    pub(super) fn refresh_shade_capture(
        &mut self,
        host: &mut dyn HostServices,
        id: u32,
        image: Arc<dyn ImageResource>,
    ) -> Result<()> {
        let uploaded = host.publish_image_resource(self.client, id, image.clone())?;
        let path = uploaded.path.relative().to_owned();
        let shade = self
            .shades
            .get_mut(&id)
            .context("N2System captured shade identity")?;
        shade.source_paint = json!({"kind":"image","asset":path});
        shade.input = Some(ImageInput::Resource(image));
        shade.preparing = None;
        shade.generation = shade
            .generation
            .checked_add(1)
            .context("N2System shade generation")?;
        shade.queued.clear();
        shade.queued.push_back(shade.passes);
        // Keep the admitted job. A changing Plain source coalesces its newest
        // input instead of continually cancelling work before it can finish.
        if let Some(previous) = self.captured_images.insert(id, path) {
            host.release_image_resource(self.client, &previous)?;
        }
        Ok(())
    }

    pub(super) fn remove_shade(&mut self, host: &mut dyn HostServices, id: u32) -> Result<()> {
        if let Some(shade) = self.shades.remove(&id) {
            if let Some(path) = shade.original_alias {
                host.release_image_resource(self.client, &path)?;
            }
            if let Some(path) = shade.output {
                host.release_image_resource(self.client, &path)?;
            }
        }
        Ok(())
    }

    pub(super) fn update_shades(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let mut changes = Vec::new();
        let mut release = Vec::new();
        let mut surface_updates = Vec::new();
        for (&id, shade) in &mut self.shades {
            if let Some(job) = &mut shade.surface_readback {
                match job.poll()? {
                    ImageProcessState::Pending => continue,
                    ImageProcessState::Failed(error) => bail!("N2System shade surface: {error}"),
                    ImageProcessState::Ready(image) => {
                        let uploaded =
                            host.publish_image_resource(self.client, id, image.clone())?;
                        let path = uploaded.path.relative().to_owned();
                        shade.source_paint = json!({"kind":"image","asset":path});
                        shade.original_alias = Some(path);
                        shade.input = Some(ImageInput::Resource(image));
                        shade.surface_readback = None;
                    }
                }
            }
            if let Some((generation, job)) = &mut shade.pending {
                match job.poll()? {
                    ImageProcessState::Pending => continue,
                    ImageProcessState::Failed(error) => bail!("N2System SetShade: {error}"),
                    ImageProcessState::Ready(image) => {
                        // A new capture may supersede the job's input. It must
                        // not be replaced with that older result for the next job.
                        if *generation == shade.generation {
                            shade.input = Some(ImageInput::Resource(image.clone()));
                        }
                        let uploaded = host.publish_image_resource(self.client, id, image)?;
                        let path = uploaded.path.relative().to_owned();
                        if let Some(old) = shade.output.replace(path.clone()) {
                            release.push(old);
                        }
                        let mut paint = shade.source_paint.clone();
                        if paint["kind"] == "solid" {
                            paint = json!({"kind":"image","asset":path});
                        } else {
                            paint["asset"] = json!(path);
                        }
                        if shade.surface {
                            let mut spec = self
                                .surfaces
                                .get(&id)
                                .context("N2System shaded surface identity")?
                                .clone();
                            spec.initial = Some(path.clone());
                            changes.push(json!({"op":"create_render_target","id":id,"spec":spec}));
                            surface_updates.push((id, spec));
                        } else if shade.resource {
                            let Kind::Image { paint: current, .. } = &mut self
                                .entities
                                .nodes
                                .get_mut(&id)
                                .context("N2System shaded image owner missing")?
                                .kind
                            else {
                                bail!("N2System shaded image changed identity");
                            };
                            *current = paint;
                        } else {
                            changes.push(json!({"op":"set_paint","id":id,"paint":paint}));
                        }
                        shade.pending = None;
                    }
                }
            }
            if shade.queued.is_empty() {
                continue;
            }
            if let Some(task) = &mut shade.preparing {
                let Some(input) = task.try_take()? else {
                    continue;
                };
                shade.input = Some(input);
                shade.preparing = None;
            }
            if shade.input.is_none() {
                let asset = shade.source_paint["asset"]
                    .as_str()
                    .context("N2System shade source asset")?
                    .to_owned();
                let metadata = self.images.as_ref().unwrap().clone();
                match self.pool.as_ref().unwrap().spawn(move |cancel| {
                    cancel.check()?;
                    let size = metadata.size(&asset)?;
                    cancel.check()?;
                    Ok(ImageInput::Asset {
                        path: asset,
                        size,
                        fingerprint: None,
                    })
                }) {
                    Ok(task) => shade.preparing = Some(task),
                    Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => {}
                    Err(e) => return Err(e),
                }
                continue;
            }
            let mut effects = Vec::new();
            let input = shade.input.as_ref().unwrap().clone();
            let mut crop = None;
            if let Some(extent) = shade.crop_extent {
                let ImageInput::Resource(image) = &input else {
                    bail!("N2System Plain shade requires a completed capture");
                };
                let size = image.size();
                let uv = shade.source_uv.unwrap_or([0., 0., 1., 1.]);
                let x = (uv[0] * size[0] as f32).round() as u32;
                let y = (uv[1] * size[1] as f32).round() as u32;
                ensure!(
                    u64::from(x) + u64::from(extent[0]) <= u64::from(size[0])
                        && u64::from(y) + u64::from(extent[1]) <= u64::from(size[1]),
                    "N2System Plain shade crop bounds"
                );
                effects.push(json!({"kind":"rect_map","spec":{"output_size":extent,"clear":[0,0,0,0],
                    "regions":[{"source":[x,y,extent[0],extent[1]],"destination":[0,0,extent[0],extent[1]]}],
                    "sample":"floor","copy_alpha":true}}));
                crop = Some((size, [x, y], extent));
            }
            effects.push(json!({"kind":"feedback_box_blur","spec":{"passes":shade.queued.front().copied().unwrap()}}));
            if let Some((size, [x, y], extent)) = crop {
                effects.push(json!({"kind":"rect_map","spec":{"output_size":size,"clear":[0,0,0,0],
                    "regions":[{"source":[0,0,extent[0],extent[1]],"destination":[x,y,extent[0],extent[1]]}],
                    "sample":"floor","copy_alpha":true}}));
            }
            match self
                .images
                .as_ref()
                .unwrap()
                .begin_process(input, Arc::from(serde_json::to_vec(&effects)?))
            {
                Ok(job) => {
                    shade.queued.pop_front();
                    shade.pending = Some((shade.generation, job));
                }
                Err(Error::Limit("image preparation task capacity")) => {}
                Err(error) => return Err(error.into()),
            }
        }
        self.submit(host, changes, vec![])?;
        for (id, spec) in surface_updates {
            self.surfaces.insert(id, spec);
        }
        for path in release {
            host.release_image_resource(self.client, &path)?;
        }
        Ok(())
    }
}

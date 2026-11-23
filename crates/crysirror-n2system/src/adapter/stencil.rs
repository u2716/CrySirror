// SPDX-License-Identifier: MPL-2.0
//! CreateStencil40D2A0, trial1.0.0.0. Channel conversion is shared image work;
//! the entity retains native ownership/propagation and never draws a picture.
//! Native4508E0 inherits framebuffer4984; nested framebuffer composition in
//! 419AF0 uses the shared packed-pair /256 profile for nested stencil planes.
//! CreateMask40D010 uses the same framebuffer with delta interpolation.
//! Constructor45DF00 uses integer strings/half extents, not fractional geometry.
use super::*;
use crysirror_core::image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource};

pub(super) struct Preparation {
    input: Option<ImageInput>,
    metadata: Option<Task<ImageInput>>,
    job: Option<ImageJob>,
    capturing: bool,
    effects: Arc<[u8]>,
    parent: u32,
    bitmap: Option<Arc<dyn ImageResource>>,
    plane_job: Option<ImageJob>,
    origin: Option<[i32; 2]>,
    padded: bool,
}
impl N2SystemVm {
    pub(super) fn create_stencil(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        state: &mut Execution,
        args: &Arguments<'_>,
        stencil: bool,
    ) -> Result<bool> {
        let (destination_parent, _) = self.entities.destination(current, &args.string(0)?)?;
        if state
            .stencil_preparation
            .as_ref()
            .is_some_and(|p| p.parent != destination_parent)
        {
            state.stencil_preparation = None;
        }
        let mut ancestor = Some(destination_parent);
        let mut parent_plane = None;
        for depth in 0..128 {
            let Some(id) = ancestor else {
                break;
            };
            if self.alpha_masks.contains_key(&id) {
                parent_plane = Some(
                    self.stencil_planes
                        .get(&id)
                        .context("N2System native matte framebuffer missing")?
                        .clone(),
                );
                break;
            }
            ancestor = self.entities.nodes[&id].parent;
            ensure!(
                depth < 127 || ancestor.is_none(),
                "N2System stencil ancestry depth"
            );
        }
        if state.stencil_preparation.is_none() {
            // atoi selects four modes. Every stored channel receives the same
            // transformed alpha, including alpha itself (not alpha squared).
            let mode = if stencil { args.integer(4)? } else { 0 };
            let table: Vec<u8> = (0..=255u8)
                .map(|a| match mode {
                    255 => {
                        if a == 255 {
                            255
                        } else {
                            0
                        }
                    }
                    0 => {
                        if a == 0 {
                            255
                        } else {
                            0
                        }
                    }
                    128 => a,
                    _ => {
                        if a != 0 && a != 255 {
                            255
                        } else {
                            0
                        }
                    }
                })
                .collect();
            let effects: Arc<[u8]> = serde_json::to_vec(&if stencil {
                json!([{"kind":"channel_lookup","channel":3,"write_mask":15,"table":table}])
            } else {
                json!([{"kind":"clear_transparent_rgb"}])
            })?
            .into();
            let name = args.string(if stencil { 5 } else { 4 })?;
            if !name.is_empty()
                && resource::resolve(
                    self.catalog.as_ref().context("N2System stencil catalog")?,
                    &name,
                )
                .is_err()
                && self
                    .query(current, &name)?
                    .iter()
                    .any(|id| self.shades.get(id).is_some_and(shade::Shade::pending))
            {
                return Ok(false);
            }
            let surface = self.surface_source(current, &name)?;
            let source = if name.is_empty() || surface.is_some() {
                None
            } else {
                self.source_image(current, &name)?
            };
            let mut preparation = Preparation {
                input: None,
                metadata: None,
                job: None,
                capturing: false,
                effects,
                parent: destination_parent,
                bitmap: None,
                plane_job: None,
                origin: None,
                padded: false,
            };
            if let Some(id) = surface {
                preparation.capturing = true;
                preparation.job = Some(self.begin_surface_image(host, id)?);
            } else if let Some(source) = source {
                match serde_json::from_value::<crysirror_scene::layer::Paint>(source.paint)? {
                    crysirror_scene::layer::Paint::Solid { color } => {
                        preparation.input = Some(ImageInput::Solid {
                            size: source.size,
                            color,
                        })
                    }
                    crysirror_scene::layer::Paint::Image { asset, .. } => {
                        if let Some(image) = host.published_image_resource(self.client, &asset)? {
                            preparation.input = Some(ImageInput::Resource(image));
                        } else {
                            let images = self
                                .images
                                .as_ref()
                                .context("N2System stencil images")?
                                .clone();
                            match self.pool.as_ref().context("N2System stencil pool")?.spawn(
                                move |cancel| {
                                    cancel.check()?;
                                    let size = images.size(&asset)?;
                                    cancel.check()?;
                                    Ok(ImageInput::Asset {
                                        path: asset,
                                        size,
                                        fingerprint: None,
                                    })
                                },
                            ) {
                                Ok(task) => preparation.metadata = Some(task),
                                Err(error) if error.is::<crysirror_tasks::CapacityExceeded>() => {
                                    return Ok(false);
                                }
                                Err(error) => return Err(error),
                            }
                        }
                    }
                    _ => bail!("N2System stencil source is not a bitmap"),
                }
            } else {
                // Native4915F0 failure falls back to491B80's current frame.
                preparation.capturing = true;
                preparation.job = Some(host.begin_frame_capture(self.client)?);
            }
            state.stencil_preparation = Some(preparation);
        }
        let preparation = state.stencil_preparation.as_mut().unwrap();
        if let Some(task) = &mut preparation.metadata {
            let Some(input) = task.try_take()? else {
                return Ok(false);
            };
            preparation.input = Some(input);
            preparation.metadata = None;
        }
        if preparation.capturing {
            match preparation.job.as_mut().unwrap().poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => bail!("N2System stencil capture: {error}"),
                ImageProcessState::Ready(image) => {
                    preparation.input = Some(ImageInput::Resource(image));
                    preparation.job = None;
                    preparation.capturing = false;
                }
            }
        }
        if preparation.bitmap.is_none() && preparation.job.is_none() && !preparation.padded {
            let input = preparation
                .input
                .as_ref()
                .context("N2System matte padding input")?;
            let size = match input {
                ImageInput::Resource(image) => image.size(),
                ImageInput::Asset { size, .. } | ImageInput::Solid { size, .. } => *size,
                _ => bail!("N2System matte input layout"),
            };
            let pad = (4 - size[0] % 4) % 4;
            if pad != 0 {
                let mut effects: Vec<Json> = serde_json::from_slice(&preparation.effects)?;
                //46EC10 rounds stored width to four pixels;4712D0 clears the
                // allocation. Padding precedes stencil alpha conversion.
                effects.insert(
                    0,
                    json!({"kind":"canvas_margins","margins":[0,0,pad,0],"fill":[0,0,0,0]}),
                );
                preparation.effects = serde_json::to_vec(&effects)?.into();
            }
            preparation.padded = true;
        }
        if preparation.bitmap.is_none() && preparation.job.is_none() {
            match self.images.as_ref().unwrap().begin_process(
                preparation
                    .input
                    .as_ref()
                    .context("N2System stencil input")?
                    .clone(),
                preparation.effects.clone(),
            ) {
                Ok(job) => preparation.job = Some(job),
                Err(Error::Limit("image preparation task capacity")) => return Ok(false),
                Err(error) => return Err(error.into()),
            }
        }
        if preparation.bitmap.is_none() {
            let image = match preparation.job.as_mut().unwrap().poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => bail!("N2System stencil preparation: {error}"),
                ImageProcessState::Ready(image) => image,
            };
            ensure!(
                image.client() == self.client,
                "N2System foreign stencil result"
            );
            preparation.bitmap = Some(image);
            preparation.job = None;
        }
        let bitmap = preparation.bitmap.as_ref().unwrap().clone();
        if preparation.origin.is_none() {
            let size = bitmap.size();
            preparation.origin = Some([
                self.bitmap_position(host, destination_parent, args.value(2), size[0], 0)?,
                self.bitmap_position(host, destination_parent, args.value(3), size[1], 1)?,
            ]);
        }
        let origin = preparation.origin.unwrap();
        if preparation.plane_job.is_none() {
            let has_parent = parent_plane.is_some();
            let multiply = stencil && has_parent;
            let interpolate = !stencil && has_parent;
            let destination = parent_plane
                .map(ImageInput::Resource)
                .unwrap_or(ImageInput::Solid {
                    size: self.config.canvas,
                    color: [0; 4],
                });
            let input = ImageInput::Raster {
                destination: Box::new(destination),
                source: Box::new(ImageInput::Resource(bitmap.clone())),
            };
            let effects: Arc<[u8]> = serde_json::to_vec(&json!([{"kind":"byte_raster","spec":{"operation":"matte_span_256","spec":{"destination":origin,"multiply":multiply,"interpolate_row_prefix":interpolate}}}]))?.into();
            match self.images.as_ref().unwrap().begin_process(input, effects) {
                Ok(job) => preparation.plane_job = Some(job),
                Err(Error::Limit("image preparation task capacity")) => return Ok(false),
                Err(error) => return Err(error.into()),
            }
        }
        let plane = match preparation.plane_job.as_mut().unwrap().poll()? {
            ImageProcessState::Pending => return Ok(false),
            ImageProcessState::Failed(error) => bail!("N2System stencil plane: {error}"),
            ImageProcessState::Ready(plane) => plane,
        };
        ensure!(
            plane.client() == self.client && plane.size() == self.config.canvas,
            "N2System stencil plane identity"
        );
        let id = self.visual(
            host,
            current,
            args,
            json!({"kind":"group"}),
            bitmap.size(),
            None,
        )?;
        // Coordinates were admitted with the plane preparation, before other
        // NSS threads could move a containing box while that worker completed.
        self.submit(host,vec![json!({"op":"set_rect","id":id,"rect":[origin[0],origin[1],bitmap.size()[0],bitmap.size()[1]]})],vec![])?;
        let resource = host.publish_image_resource(self.client, id, plane.clone())?;
        let path = resource.path.relative().to_owned();
        self.captured_images.insert(id, path.clone());
        self.alpha_masks.insert(id, path);
        // The constructor installs459BC0 for sprite animation callbacks. Its
        // matte geometry remains at admission; enabled propagation still lets
        // commands reach drawable descendants through the entity graph.
        self.stencils.insert(id, [0.; 2]);
        if plane.scalar_coverage()!=Some(true) {self.rgba_masks.insert(id);}
        self.stencil_planes.insert(id, plane);
        self.stencil_bitmaps.insert(id, bitmap);
        self.entities.nodes.get_mut(&id).unwrap().container =
            Some(args.truth(if stencil { 6 } else { 5 })?);
        state.stencil_preparation = None;
        Ok(true)
    }
}

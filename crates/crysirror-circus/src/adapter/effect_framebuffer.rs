//! Native capture selection uses the engine's immutable Canvas image worker.
use super::*;
use crysirror_core::image_processing::{ImageJob, ImageProcessState};
#[derive(Default)]
pub(super) struct Framebuffer {
    capture: Option<ImageJob>,
    capture_image: Option<UploadedImage>,
    capture_bitmap: Option<ImageJob>,
    capture_crop: Option<[i32; 4]>,
    bound_snapshot: Option<UploadedImage>,
    snapshot: Option<UploadedImage>,
    mask: Option<Mask>,
    flat: Option<Flat>,
    clear_pending: Option<bool>,
    pub(super) detached: bool,
    background: bool,
    tiles: Option<crysirror_vfx::tile_mask::TileMaskField>,
    tile_color: Option<[u8; 3]>,
    tile_job: Option<ImageJob>,
    tile_image: Option<UploadedImage>,
    tile_initializing: bool,
    tile_owner: Option<(i16, Option<[u8; 3]>)>,
}
struct Flat {
    image: Option<UploadedImage>,
    ramp: crysirror_core::service::SteppedRamp,
    out: bool,
    protected: bool,
    cleared: bool,
}
struct Mask {
    ramp: crysirror_core::service::SteppedRamp,
    asset: String,
    previous: i32,
}
impl Bridge<'_> {
    pub(in crate::adapter) fn captured_native_image(&self)->Option<&UploadedImage>{self.services.effect_control.framebuffer.bound_snapshot.as_ref()}
    pub(in crate::adapter) fn framebuffer_capture_pending(&self) -> bool {
        self.services
            .effect_control
            .framebuffer
            .capture_crop
            .is_some()
    }
    pub(super) fn begin_flat_framebuffer(
        &mut self,
        out: bool,
        ticks: i32,
        color: [u8; 3],
        protected: bool,
    ) -> Result<bool> {
        self.replace_framebuffer_color()?;
        let ramp = crysirror_core::service::SteppedRamp::new(
            if out { 0. } else { 1. },
            if out { 1. } else { 0. },
            if out {
                ticks
            } else {
                ticks
                    .checked_mul(2)
                    .context("Circus flat fade duration overflow")?
            },
        )?;
        self.submit(vec![json!({"op":"put","layer":{"id":0x3b01_0003u32,"rect":[0,0,1280,720],"paint":{"kind":"solid","color":[color[0],color[1],color[2],255]},"opacity":if out{0}else{255},"order":[3,0]}})],vec![])?;
        self.services.effect_control.framebuffer.flat = Some(Flat {
            image: None,
            ramp,
            out,
            protected,
            cleared: false,
        });
        Ok(true)
    }
    pub(in crate::adapter) fn begin_restore_snapshot_fade(
        &mut self,
        image: UploadedImage,
        ticks: i32,
    ) -> Result<()> {
        self.replace_framebuffer_color()?;
        let ramp = crysirror_core::service::SteppedRamp::new(
            1.,
            0.,
            ticks
                .checked_mul(2)
                .context("Circus restore fade duration overflow")?,
        )?;
        self.submit(vec![json!({"op":"put","layer":{"id":0x3b01_0003u32,"rect":[0,0,1280,720],"paint":{"kind":"image","asset":image.path.relative()},"filter":{"encoded_sampling":true},"opacity":255,"order":[3,0]}})],vec![])?;
        self.services.effect_control.framebuffer.flat = Some(Flat {
            image: Some(image),
            ramp,
            out: false,
            protected: false,
            cleared: false,
        });
        Ok(())
    }
    pub(super) fn advance_flat_framebuffer(&mut self) -> Result<()> {
        let fast = (self.services.effect_control.frame_fast
            || self.input.buttons_down & crysirror_core::service::buttons::CTRL != 0)
            && !self.services.effect_control.suppress_fast_color;
        let skip = self.services.effect_control.frame_skip_allowed
            && self.input.buttons_pressed
                & (crysirror_core::service::buttons::MOUSE_RIGHT
                    | crysirror_core::service::buttons::ESCAPE)
                != 0;
        let Some(flat) = &mut self.services.effect_control.framebuffer.flat else {
            return Ok(());
        };
        if !flat.ramp.active() {
            return Ok(());
        }
        let alpha = flat
            .ramp
            .advance_by(if fast { 2 } else { 1 }, skip)?
            .unwrap_or(0.)
            .clamp(0., 1.);
        let (active, out) = (flat.ramp.active(), flat.out);
        if active {
            self.submit(
                vec![json!({"op":"set_opacity","id":0x3b01_0003u32,"opacity":(alpha*255.)as u8})],
                vec![],
            )?;
        } else if out {
            self.services.effect_control.framebuffer.background = true;
            self.submit(vec![json!({"op":"set_visible","id":SCENE_ROOT,"visible":false}),json!({"op":"put","layer":{"id":COLOUR_ROOT,"rect":[0,0,1280,720],"paint":{"kind":"solid","color":[0,0,0,255]},"order":[1,0]}}),json!({"op":"set_opacity","id":0x3b01_0003u32,"opacity":255})],vec![])?;
        } else {
            let old = self.services.effect_control.framebuffer.flat.take();
            self.submit(vec![json!({"op":"remove","id":0x3b01_0003u32})], vec![])?;
            if let Some(image) = old.and_then(|flat| flat.image) {
                self.host
                    .release_image_resource(self.client, image.path.relative())?;
            }
        }
        Ok(())
    }
    fn clear_framebuffer_content(&mut self, protected: bool) -> Result<()> {
        for object in self.services.visual.roots().collect::<Vec<_>>() {
            self.native_visual(&crate::machine::visual_ops::VisualRequest::Priority {
                object,
                priority: if object == 0 { 0 } else { 50 },
            })?;
        }
        if !protected {
            self.native_text(&crate::machine::text_ops::TextRequest::Clear)?;
            self.clear_packed_presentation()?;
            if self.services.profile_captured {
                profile::clear_capture(self.host, self.client)?;
                self.services.profile_captured = false;
            }
            let slots = self
                .services
                .images
                .keys()
                .copied()
                .filter(|slot| !(37..=44).contains(slot))
                .collect::<Vec<_>>();
            self.submit(
                self.services
                    .displayed
                    .iter()
                    .filter(|slot| !(37..=44).contains(*slot))
                    .map(|slot| json!({"op":"remove","id":0x3700_0000+u32::from(*slot)}))
                    .collect(),
                vec![],
            )?;
            self.services
                .displayed
                .retain(|slot| (37..=44).contains(slot));
            for slot in slots {
                if let Some(image) = self.services.images.remove(&slot) {
                    self.host
                        .release_image_resource(self.client, image.path.relative())?;
                }
                self.services.image_names.remove(&slot);
                self.services.image_extents.remove(&slot);
            }
            if let Some(image) = self.services.effect_control.framebuffer.snapshot.take() {
                self.host
                    .release_image_resource(self.client, image.path.relative())?;
            }
            // An unprotected source clear has removed the old picture. Keep
            // the empty black scene drawable so the next native publication
            // can appear; a completed fade must not hide all future menus.
            self.root()?;
            let mut visibility = vec![json!({"op":"set_visible","id":SCENE_ROOT,"visible":true})];
            let color: Value = serde_json::from_slice(
                &self
                    .host
                    .query_engine(self.client, EngineQuery::Layer(COLOUR_ROOT))?,
            )?;
            if !color.is_null() {
                visibility.push(json!({"op":"set_visible","id":COLOUR_ROOT,"visible":false}));
            }
            self.submit(visibility, vec![])?;
            self.services.effect_control.framebuffer.background = false;
        }
        Ok(())
    }
    pub(super) fn clear_source_framebuffer(&mut self, protected: bool) -> Result<bool> {
        if let Some(owner) = self.services.effect_control.framebuffer.clear_pending {
            ensure!(owner == protected, "Circus framebuffer clear owner");
        } else {
            if !protected {
                self.cancel_framebuffer_color()?;
            }
            self.clear_framebuffer_content(protected)?;
            self.services.effect_control.framebuffer.clear_pending = Some(protected);
        }
        if !self.falling_effect(0, 0)? {
            return Ok(false);
        }
        self.services.effect_control.framebuffer.clear_pending = None;
        Ok(true)
    }
    pub(in crate::adapter) fn detach_framebuffer_color(&mut self) -> Result<()> {
        let commands = self
            .framebuffer_layer_ids()
            .into_iter()
            .map(|id| json!({"op":"set_visible","id":id,"visible":false}))
            .collect();
        self.services.effect_control.framebuffer.detached = true;
        self.submit(commands, vec![])?;
        if self.services.effect_control.framebuffer.background {
            self.submit(
                vec![json!({"op":"set_visible","id":COLOUR_ROOT,"visible":false})],
                vec![],
            )?;
        }
        Ok(())
    }
    pub(in crate::adapter) fn poll_framebuffer_status(&mut self) -> Result<bool> {
        if let Some(flat) = &self.services.effect_control.framebuffer.flat {
            if !flat.ramp.active() && flat.out {
                let (protected, cleared) = (flat.protected, flat.cleared);
                if !cleared {
                    self.clear_framebuffer_content(protected)?;
                    self.services
                        .effect_control
                        .framebuffer
                        .flat
                        .as_mut()
                        .unwrap()
                        .cleared = true;
                }
                if !self.falling_effect(0, 0)? {
                    return Ok(false);
                }
                self.services.effect_control.framebuffer.flat = None;
                self.submit(vec![json!({"op":"remove","id":0x3b01_0003u32})], vec![])?;
            }
        }
        Ok(self.framebuffer_complete())
    }
    pub(super) fn begin_tile_mask(&mut self, pattern: i16, color: Option<[u8; 3]>) -> Result<bool> {
        if self.services.effect_control.framebuffer.tile_initializing {
            ensure!(
                self.services.effect_control.framebuffer.tile_owner == Some((pattern, color)),
                "Circus tile mask initialization owner"
            );
            if !self.effects_ready()? {
                return Ok(false);
            }
            self.services.effect_control.framebuffer.tile_initializing = false;
            return Ok(true);
        }
        if !self.effects_ready()? {
            return Ok(false);
        }
        self.replace_framebuffer_color()?;
        if let Some(old) = self.services.effect_control.framebuffer.snapshot.take() {
            self.host
                .release_image_resource(self.client, old.path.relative())?;
        }
        self.services.effect_control.framebuffer.tile_owner = Some((pattern, color));
        use crysirror_vfx::tile_mask::{TileMaskField, TileMaskPattern as P, TileMaskSpec};
        let pattern = match pattern {
            0 => P::Scatter,
            1 => P::Diagonal,
            2 => P::Down,
            3 => P::ReverseDiagonal,
            4 => P::WideScatter,
            5 => P::Staircase,
            6 => P::Diamond,
            9 => P::Up,
            10 => P::RandomDelay,
            _ => P::Base,
        };
        let rng = self
            .services
            .sequence_rng
            .as_mut()
            .context("Circus tile mask random sequence missing")?;
        let field = TileMaskField::new(
            TileMaskSpec {
                grid: [41, 24],
                spacing: [32, 32],
                canvas: [1280, 720],
                pattern,
            },
            rng,
        )?;
        self.services.effect_control.framebuffer.tiles = Some(field);
        self.services.effect_control.framebuffer.tile_color = color;
        let state = rng.clone();
        if let Some(effect) = &mut self.services.effects {
            let crysirror_vfx::falling_atlas::SimulationClock::External { tick, .. } =
                effect.spec.clock
            else {
                bail!("Circus tile mask random clock");
            };
            let epoch = effect.spec.sequence_exchange.as_ref().map_or(Ok(1), |e| {
                e.epoch
                    .checked_add(1)
                    .context("Circus tile mask random epoch")
            })?;
            effect.spec.sequence_exchange = Some(crysirror_vfx::falling_atlas::SequenceExchange {
                epoch,
                base_tick: tick,
                state,
            });
            let spec = serde_json::to_value(&effect.spec)?;
            self.host.submit_engine(
                self.client,
                1,
                &serde_json::to_vec(
                    &json!({"world":[{"op":"set_vfx","id":0x3900_0000u32,"vfx":spec}]}),
                )?,
            )?;
            self.services.effect_control.framebuffer.tile_initializing = true;
            return Ok(false);
        }
        Ok(true)
    }
    pub(in crate::adapter) fn framebuffer_layer_ids(&self) -> Vec<u32> {
        let mut ids = Vec::new();
        if self.services.effect_control.framebuffer.flat.is_some() {
            ids.push(0x3b01_0003);
        }
        if self.services.effect_control.flash.is_some() {
            ids.push(0x3b01_0000);
        }
        if self.services.effect_control.framebuffer.mask.is_some() {
            ids.push(0x3b01_0001);
        }
        if self
            .services
            .effect_control
            .framebuffer
            .tile_image
            .is_some()
        {
            ids.push(0x3b01_0002);
        }
        ids
    }
    pub(in crate::adapter) fn framebuffer_complete(&self) -> bool {
        self.services.effect_control.framebuffer.flat.is_none()
            && self.services.effect_control.flash.is_none()
            && self.services.effect_control.framebuffer.mask.is_none()
            && self.services.effect_control.framebuffer.tiles.is_none()
            && self.services.effect_control.framebuffer.tile_job.is_none()
    }
    pub(in crate::adapter) fn framebuffer_ready(&mut self) -> Result<bool> {
        let Some(job) = &mut self.services.effect_control.framebuffer.tile_job else {
            return Ok(true);
        };
        let image = match job.poll()? {
            ImageProcessState::Pending => return Ok(false),
            ImageProcessState::Failed(error) => bail!("Circus tile mask image worker: {error}"),
            ImageProcessState::Ready(image) => image,
        };
        ensure!(
            image.client() == self.client && image.size() == [1280, 720],
            "Circus foreign tile mask image"
        );
        let image = self
            .host
            .publish_image_resource(self.client, 0x3508_0001, image)?;
        self.submit(vec![json!({"op":"put","layer":{"id":0x3b01_0002u32,"rect":[0,0,1280,720],"paint":{"kind":"image","asset":image.path.relative()},"visible":!self.services.effect_control.framebuffer.detached,"order":[3,0]}})],vec![])?;
        if let Some(old) = self
            .services
            .effect_control
            .framebuffer
            .tile_image
            .replace(image)
        {
            self.host
                .release_image_resource(self.client, old.path.relative())?;
        }
        self.services.effect_control.framebuffer.tile_job = None;
        Ok(true)
    }
    pub(super) fn advance_tile_mask(&mut self) -> Result<()> {
        ensure!(
            self.services.effect_control.framebuffer.tile_job.is_none(),
            "Circus tile mask frame bypassed completion fence"
        );
        let fast = self.services.effect_control.frame_fast
            || self.input.buttons_down & crysirror_core::service::buttons::CTRL != 0;
        let skip = self.services.effect_control.frame_skip_allowed
            && self.input.buttons_pressed
                & (crysirror_core::service::buttons::MOUSE_RIGHT
                    | crysirror_core::service::buttons::ESCAPE)
                != 0;
        let Some(field) = &mut self.services.effect_control.framebuffer.tiles else {
            return Ok(());
        };
        field.advance(if fast { 2 } else { 1 })?;
        if skip || !field.active() {
            self.services.effect_control.framebuffer.tiles = None;
            if let Some(old) = self.services.effect_control.framebuffer.tile_image.take() {
                self.submit(vec![json!({"op":"remove","id":0x3b01_0002u32})], vec![])?;
                self.host
                    .release_image_resource(self.client, old.path.relative())?;
            }
            return Ok(());
        }
        let samples = field.samples();
        if self.services.effect_control.framebuffer.detached {
            return Ok(());
        }

        let Some(color) = self.services.effect_control.framebuffer.tile_color else {
            return Ok(());
        };
        let atlas = self
            .services
            .image_extents
            .get(&45)
            .copied()
            .context("Circus effbreak atlas dimensions missing")?;
        let spec = crysirror_image::alpha_stamps::AlphaStamps {
            clear: Some(0),
            preserve_opaque: true,
            stamps: samples
                .into_iter()
                .map(|sample| crysirror_image::alpha_stamps::AlphaStamp {
                    source: [i32::from(sample.frame) * 32, 0, 32, 33],
                    position: sample.position,
                })
                .collect(),
        };
        let input = crysirror_core::image_processing::ImageInput::Raster {
            destination: Box::new(crysirror_core::image_processing::ImageInput::Solid {
                size: [1280, 720],
                color: [color[0], color[1], color[2], 0],
            }),
            source: Box::new(crysirror_core::image_processing::ImageInput::Asset {
                path: image_path("system\\effbreak")?.relative().into(),
                size: atlas,
                fingerprint: None,
            }),
        };
        self.services.effect_control.framebuffer.tile_job =
            Some(self.host.image_metadata(self.client)?.begin_process(
                input,
                Arc::from(serde_json::to_vec(&vec![
                    crysirror_image::effects::ImageEffect::AlphaStamps { spec },
                ])?),
            )?);
        Ok(())
    }
    pub(super) fn cancel_framebuffer_color(&mut self) -> Result<()> {
        let mut commands = Vec::new();
        if let Some(flat) = self.services.effect_control.framebuffer.flat.take() {
            commands.push(json!({"op":"remove","id":0x3b01_0003u32}));
            if let Some(image) = flat.image {
                self.host
                    .release_image_resource(self.client, image.path.relative())?;
            }
        }

        self.services.effect_control.framebuffer.tiles = None;
        self.services.effect_control.framebuffer.tile_job = None;
        self.services.effect_control.framebuffer.tile_initializing = false;
        if let Some(old) = self.services.effect_control.framebuffer.tile_image.take() {
            commands.push(json!({"op":"remove","id":0x3b01_0002u32}));
            self.host
                .release_image_resource(self.client, old.path.relative())?;
        }

        if self.services.effect_control.flash.take().is_some() {
            commands.push(json!({"op":"remove","id":0x3b01_0000u32}));
        }
        if self
            .services
            .effect_control
            .framebuffer
            .mask
            .take()
            .is_some()
        {
            commands.push(json!({"op":"remove","id":0x3b01_0001u32}));
        }
        self.submit(commands, vec![])
    }
    pub(super) fn replace_framebuffer_color(&mut self) -> Result<()> {
        self.cancel_framebuffer_color()?;
        self.services.effect_control.framebuffer.detached = false;
        if let Some((_, target, _)) = self.services.fade.take() {
            self.submit(vec![json!({"op":"end_crossfade","id":target})], vec![])?;
        }
        self.root()?;
        let mut commands = vec![json!({"op":"set_visible","id":SCENE_ROOT,"visible":true})];
        let color: Value = serde_json::from_slice(
            &self
                .host
                .query_engine(self.client, EngineQuery::Layer(COLOUR_ROOT))?,
        )?;
        if !color.is_null() {
            commands.push(json!({"op":"set_visible","id":COLOUR_ROOT,"visible":false}));
        }
        self.submit(commands, vec![])?;
        Ok(())
    }
    pub(super) fn begin_mask_framebuffer(
        &mut self,
        reverse: bool,
        ticks: i32,
        color: [u8; 3],
    ) -> Result<bool> {
        let Some(image) = self.services.images.get(&21) else {
            return Ok(true);
        };
        let (size, asset) = (image.size, image.path.relative().to_owned());
        self.replace_framebuffer_color()?;
        if size != [1280, 720] {
            return Ok(true);
        } //407D60 dimension mismatch completes its effect.
        let (start, end) = if reverse {
            (255., -255.)
        } else {
            (-255., 255.)
        };
        let ramp = crysirror_core::service::SteppedRamp::new(start, end, ticks)?;
        self.root()?;
        self.submit(vec![json!({"op":"put","layer":{"id":0x3b01_0001u32,"rect":[0,0,1280,720],"paint":{"kind":"solid","color":[color[0],color[1],color[2],255]},"order":[3,0],"filter":{"mask":{"asset":asset,"gray_alpha":false,"channel":"blue","offset":start}}}})],vec![])?;
        self.services.effect_control.framebuffer.mask = Some(Mask {
            ramp,
            asset,
            previous: 0,
        });
        Ok(true)
    }
    pub(super) fn advance_mask_framebuffer(&mut self) -> Result<()> {
        if self.services.effect_control.framebuffer.detached
            && self
                .services
                .effect_control
                .framebuffer
                .mask
                .take()
                .is_some()
        {
            self.submit(vec![json!({"op":"remove","id":0x3b01_0001u32})], vec![])?;
            return Ok(());
        }
        let fast = (self.services.effect_control.frame_fast
            || self.input.buttons_down & crysirror_core::service::buttons::CTRL != 0)
            && !self.services.effect_control.suppress_fast_color;
        let skip = self.services.effect_control.frame_skip_allowed
            && self.input.buttons_pressed
                & (crysirror_core::service::buttons::MOUSE_RIGHT
                    | crysirror_core::service::buttons::ESCAPE)
                != 0;
        let Some(mask) = &mut self.services.effect_control.framebuffer.mask else {
            return Ok(());
        };
        let offset = mask
            .ramp
            .advance_by(if fast { 2 } else { 1 }, skip)?
            .unwrap_or(0.);
        let active = mask.ramp.active();
        let changed = offset as i32 != mask.previous;
        mask.previous = offset as i32;
        let asset = mask.asset.clone();
        if !active {
            self.services.effect_control.framebuffer.mask = None;
            self.submit(vec![json!({"op":"remove","id":0x3b01_0001u32})], vec![])?;
        } else if changed {
            self.submit(vec![json!({"op":"set_filter","id":0x3b01_0001u32,"filter":{"mask":{"asset":asset,"gray_alpha":false,"channel":"blue","offset":offset}}})],vec![])?;
        }
        Ok(())
    }
    pub(super) fn capture_framebuffer(&mut self, crop: [i32; 4]) -> Result<bool> {
        if let Some(owner) = self.services.effect_control.framebuffer.capture_crop {
            ensure!(owner == crop, "Circus captured background crop ownership");
        } else {
            self.services.effect_control.framebuffer.capture_crop = Some(crop);
        }
        if self.services.effect_control.framebuffer.capture.is_none()
            && self
                .services
                .effect_control
                .framebuffer
                .capture_image
                .is_none()
        {
            if !self.effects_ready()? {
                return Ok(false);
            }
            self.update_native_thumbnail()?;
            let mut receipt = json!({});
            self.freeze_thumbnail(&mut receipt)?;
            ensure!(
                receipt["has_native_thumbnail"].as_bool() == Some(true),
                "Circus native4226B0 capture selection unavailable: {}",
                receipt["native_thumbnail_error"]
            );
            let canvas = u32::try_from(
                receipt["native_thumbnail_canvas"]
                    .as_u64()
                    .context("Circus native framebuffer Canvas receipt")?,
            )?;
            self.services.effect_control.framebuffer.capture =
                Some(self.host.begin_canvas_image(self.client, canvas)?);
        }
        if self
            .services
            .effect_control
            .framebuffer
            .capture_image
            .is_none()
        {
            let image = match self
                .services
                .effect_control
                .framebuffer
                .capture
                .as_mut()
                .unwrap()
                .poll()?
            {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => {
                    bail!("Circus framebuffer capture failed: {error}")
                }
                ImageProcessState::Ready(image) => image,
            };
            ensure!(
                image.client() == self.client && image.size() == [1280, 720],
                "Circus foreign framebuffer capture extent/owner"
            );
            //44C4E0(0) copies RGB with alpha forced255 into44C130's
            //2048x1024 allocation. Shared stored-byte composition does both
            //without a private VM pixel loop; logical1280x720 remains separate.
            use crysirror_core::image_processing::{
                ImageCompositeLayer, ImageCompositeMath, ImageInput,
            };
            let layer = |input, math| ImageCompositeLayer {
                input: Box::new(input),
                source: [0, 0, 1280, 720],
                destination: [0, 0],
                clip: None,
                opacity: 255,
                math,
            };
            let input = ImageInput::Composite {
                base: Box::new(ImageInput::Solid {
                    size: [2048, 1024],
                    color: [0, 0, 0, 0],
                }),
                layers: vec![
                    layer(
                        ImageInput::Solid {
                            size: [1280, 720],
                            color: [0, 0, 0, 255],
                        },
                        ImageCompositeMath::Copy,
                    ),
                    layer(
                        ImageInput::Resource(image.clone()),
                        ImageCompositeMath::CopyRgb,
                    ),
                ],
            };
            let bitmap = self
                .host
                .image_metadata(self.client)?
                .begin_process(input, Arc::from(&b"[]"[..]))?;
            let logical = self
                .host
                .publish_image_resource(self.client, 0x3508_0000, image)?;
            self.services.effect_control.framebuffer.capture_bitmap = Some(bitmap);
            self.services.effect_control.framebuffer.capture_image = Some(logical);
            self.services.effect_control.framebuffer.capture = None;
        }
        let bitmap = match self
            .services
            .effect_control
            .framebuffer
            .capture_bitmap
            .as_mut()
            .unwrap()
            .poll()?
        {
            ImageProcessState::Pending => return Ok(false),
            ImageProcessState::Failed(error) => {
                bail!("Circus captured bitmap worker failed: {error}")
            }
            ImageProcessState::Ready(image) => image,
        };
        ensure!(
            bitmap.client() == self.client && bitmap.size() == [2048, 1024],
            "Circus captured bitmap allocation receipt"
        );
        let bitmap = self
            .host
            .publish_image_resource(self.client, 0x3508_0002, bitmap)?;
        self.bind_transient_sprite_image(0, &bitmap, [1280,720], crop)?;
        let old_bound = self
            .services
            .effect_control
            .framebuffer
            .bound_snapshot
            .replace(bitmap);
        if let Some(old) = old_bound {
            self.host
                .release_image_resource(self.client, old.path.relative())?;
        }
        let image = self
            .services
            .effect_control
            .framebuffer
            .capture_image
            .take()
            .unwrap();
        if let Some(old) = self
            .services
            .effect_control
            .framebuffer
            .snapshot
            .replace(image)
        {
            self.host
                .release_image_resource(self.client, old.path.relative())?;
        }
        self.services.effect_control.framebuffer.capture_bitmap = None;
        self.services.effect_control.framebuffer.capture_crop = None;
        Ok(true)
    }
}

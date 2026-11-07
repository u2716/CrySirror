// SPDX-License-Identifier: MPL-2.0
//!44BAE0/4153B0/415400 tiled image-filter policy. Four native source rectangles
//!are submitted to the common immutable composition worker. No VM pixel loop.
use super::*;
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageResource};
pub(super) const OVERLAY: u32 = 60002;
pub(super) struct Filter {
    pub state: i32,
    pub name: String,
    image: UploadedImage,
    source: Arc<dyn ImageResource>,
    pub position: [i32; 2],
    pub velocity: [i32; 2],
    alpha: i32,
    next: u64,
    capture: Option<ImageJob>,
    process: Option<ImageJob>,
    pending: Option<(i32, [i32; 2])>,
    overlay: Option<UploadedImage>,
    painted: bool,
}
pub(super) struct Seed {
    state: i32,
    name: String,
    source: Arc<dyn ImageResource>,
    position: [i32; 2],
    velocity: [i32; 2],
    alpha: i32,
    next: u64,
}
impl Filter {
    pub(super) fn append_restore_projection(&mut self, plan: &mut stage::StagePlan) -> bool {
        let alpha = if self.state == 1 {
            self.alpha + (255 - self.alpha) / 4
        } else {
            self.alpha - self.alpha / 4
        };
        let alpha = if self.state == 2 && alpha < 16 {
            0
        } else {
            alpha.clamp(0, 255)
        };
        let mut position = self.position;
        let size = self.source.size();
        let mut layers = vec![];
        if alpha != 0 && size[0] >= 1024 && size[1] >= 600 {
            for (dy, sy, height) in axis(position[1], size[1] as i32, 600) {
                for (dx, sx, width) in axis(position[0], size[0] as i32, 1024) {
                    if width <= 0 || height <= 0 {
                        continue;
                    }
                    layers.push(ImageCompositeLayer {
                        input: Box::new(ImageInput::Resource(self.source.clone())),
                        source: [sx, sy, width, height],
                        destination: [dx, dy],
                        clip: None,
                        opacity: alpha as u8,
                        math: ImageCompositeMath::MaskScalar256,
                    });
                }
            }
            position = [
                advance(position[0], self.velocity[0], 1024, size[0] as i32),
                advance(position[1], self.velocity[1], 600, size[1] as i32),
            ];
        }
        self.pending = Some((alpha, position));
        let drawn = !layers.is_empty();
        plan.images(layers);
        drawn
    }
    pub(super) fn commit_restore_projection(&mut self, now: u64) -> Result<()> {
        let (alpha, position) = self
            .pending
            .take()
            .context("SugarPot restored filter candidate missing")?;
        self.alpha = alpha;
        self.position = position;
        if alpha == 0 {
            self.state = 0;
        }
        self.next = now.saturating_add(33);
        Ok(())
    }
    /// 43F430 during a UD: one 415400 step over a transition sample. Returns
    /// the tiled layers to compose over the sample (None when nothing is
    /// drawn) and advances alpha/position as the standalone tick does.
    pub(super) fn sample_layers(&mut self) -> Option<Vec<ImageCompositeLayer>> {
        if self.state == 0 {
            return None;
        }
        let alpha = if self.state == 1 {
            self.alpha + (255 - self.alpha) / 4
        } else {
            self.alpha - self.alpha / 4
        };
        let alpha = if self.state == 2 && alpha < 16 { 0 } else { alpha.clamp(0, 255) };
        self.alpha = alpha;
        if alpha == 0 {
            self.state = 0;
            return None;
        }
        if self.image.size[0] < 1024 || self.image.size[1] < 600 {
            return None;
        }
        let mut layers = vec![];
        for (dy, sy, height) in axis(self.position[1], self.image.size[1] as i32, 600) {
            for (dx, sx, width) in axis(self.position[0], self.image.size[0] as i32, 1024) {
                if width == 0 || height == 0 {
                    continue;
                }
                layers.push(ImageCompositeLayer {
                    input: Box::new(ImageInput::Resource(self.source.clone())),
                    source: [sx, sy, width, height],
                    destination: [dx, dy],
                    clip: None,
                    opacity: alpha as u8,
                    math: ImageCompositeMath::MaskScalar256,
                });
            }
        }
        self.position = [
            advance(self.position[0], self.velocity[0], 1024, self.image.size[0] as i32),
            advance(self.position[1], self.velocity[1], 600, self.image.size[1] as i32),
        ];
        Some(layers)
    }
    /// Withdraws the standalone overlay so a UD capture sees the bare stage.
    pub(super) fn take_overlay_layer(&mut self) -> bool {
        std::mem::replace(&mut self.painted, false)
    }
    pub(super) fn retain_initial_image(&mut self, image: UploadedImage) {
        self.overlay = Some(image);
        self.painted = true;
    }
    pub(super) fn detached_seed(&self) -> Seed {
        Seed {
            state: self.state,
            name: self.name.clone(),
            source: self.source.clone(),
            position: self.position,
            velocity: self.velocity,
            alpha: self.alpha,
            next: self.next,
        }
    }
    pub(super) fn install(seed: Seed, image: UploadedImage) -> Self {
        Self {
            state: seed.state,
            name: seed.name,
            source: seed.source,
            image,
            position: seed.position,
            velocity: seed.velocity,
            alpha: seed.alpha,
            next: seed.next,
            capture: None,
            process: None,
            pending: None,
            overlay: None,
            painted: false,
        }
    }
    pub(super) fn has_overlay(&self) -> bool {
        self.painted
    }
    pub(super) fn overlay_path(&self) -> Option<String> {
        self.overlay.as_ref().map(|i| i.path.relative().to_owned())
    }
    pub(super) fn source_path(&self) -> String {
        self.image.path.relative().to_owned()
    }

    pub(super) fn preparing(&self) -> bool {
        self.capture.is_some() || self.process.is_some()
    }
}
impl Seed {
    pub(super) fn source(&self) -> Arc<dyn ImageResource> {
        self.source.clone()
    }
    pub(super) fn restore_state(&mut self, state: i32, velocity: [i32; 2]) {
        //4561A0 state2 requests fade-out only from an already active state1;
        //state0 disables without discarding the old resident bitmap/position.
        if state == 0 {
            self.state = 0;
        } else if state == 2 && self.state == 1 {
            self.state = 2;
        }
        self.velocity = velocity;
    }
    pub(super) fn loaded(
        name: String,
        source: Arc<dyn ImageResource>,
        velocity: [i32; 2],
        next: u64,
    ) -> Self {
        Self {
            state: 1,
            name,
            position: source.size().map(|n| n as i32),
            source,
            velocity,
            alpha: 1,
            next,
        }
    }
}
//415400 wraps in the original inclusive[-canvas, image] interval. It retains
//the boundary frame at image extent before the next increment wraps.
fn advance(position: i32, delta: i32, canvas: i32, image: i32) -> i32 {
    let p = position.wrapping_add(delta);
    if p < -canvas {
        image
    } else if p > image {
        -canvas
    } else {
        p
    }
}
fn axis(position: i32, image: i32, canvas: i32) -> [(i32, i32, i32); 2] {
    if position >= 0 {
        let first = (image - position).min(canvas).max(0);
        [(0, position, first), (first, 0, canvas - first)]
    } else {
        let first = position.unsigned_abs().min(canvas as u32) as i32;
        [(0, image - first, first), (first, 0, canvas - first)]
    }
}
impl SugarPotVm {
    pub(super) fn filter_command(
        &mut self,
        h: &mut dyn HostServices,
        args: &[String],
    ) -> Result<bool> {
        let selector = self.number(args, 0, 0)?;
        if self.filter.as_ref().is_some_and(Filter::preparing) {
            return Ok(false);
        }
        if selector == 0 {
            if let Some(filter) = &mut self.filter {
                if filter.state == 1 {
                    filter.state = 2;
                }
            }
            return Ok(true);
        }
        let velocity = [self.number(args, 2, 0)?, self.number(args, 3, 0)?];
        if selector == 1 {
            let name = self.string(args, 1)?;
            // 415740 opens only grp\\<name>.abm (loose file, then archive).
            let path = self.resolve("grp", &name, &["abm"])?;
            let native = path
                .rsplit('/')
                .next()
                .context("SugarPot filter resource")?;
            let Some(image) = self.image(h, native)? else {
                return Ok(false);
            };
            let source = h
                .published_image_resource(self.client, image.path.relative())?
                .context("SugarPot filter shared source not published")?;
            let old = self.filter.take();
            if let Some(old) = old {
                if old.painted {
                    self.submit(h, vec![json!({"op":"remove","id":OVERLAY})], vec![])?;
                }
                if let Some(image) = old.overlay {
                    h.release_image_resource(self.client, image.path.relative())?;
                }
                self.declared_images.remove(old.image.path.relative());
            }
            self.declared_images.insert(image.path.relative().into());
            self.filter = Some(Filter {
                state: 1,
                name,
                position: image.size.map(|v| v as i32),
                velocity,
                alpha: 1,
                next: h.now_ms(),
                image,
                source,
                capture: None,
                process: None,
                pending: None,
                overlay: None,
                painted: false,
            });
        } else if let Some(filter) = &mut self.filter {
            filter.velocity = velocity;
        }
        //Other nonzero selectors only update velocity in44BAE0. They neither
        //start a filter nor substitute a successful image-load operation.
        Ok(true)
    }
    pub(super) fn filter_frame(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let Some(mut filter) = self.filter.take() else {
            return Ok(());
        };
        let result = self.poll_filter(h, &mut filter);
        self.filter = Some(filter);
        result
    }
    fn poll_filter(&mut self, h: &mut dyn HostServices, f: &mut Filter) -> Result<()> {
        if self.raster.as_ref().is_some_and(|r| r.state != 0) || self.mask_update.is_some() {
            if f.painted {
                self.submit(h, vec![json!({"op":"remove","id":OVERLAY})], vec![])?;
                f.painted = false;
            }
            return Ok(());
        }
        if let Some(job) = &mut f.process {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(()),
                ImageProcessState::Failed(e) => bail!("SugarPot tiled filter: {e}"),
                ImageProcessState::Ready(resource) => {
                    f.process = None;
                    let (alpha, position) =
                        f.pending.take().context("SugarPot filter completion")?;
                    f.alpha = alpha;
                    f.position = position;
                    let image = h.publish_image_resource(self.client, self.next_image, resource)?;
                    self.next_image = self
                        .next_image
                        .checked_add(1)
                        .context("SugarPot filter image ID")?;
                    self.submit(h,vec![json!({"op":"put","layer":{"id":OVERLAY,"rect":[0,0,1024,600],"order":[999,2],"paint":{"kind":"image","asset":image.path.relative()}}})],vec![])?;
                    f.painted = true;
                    if let Some(old) = f.overlay.replace(image) {
                        h.release_image_resource(self.client, old.path.relative())?;
                    }
                    f.next = h.now_ms().saturating_add(33);
                    return Ok(());
                }
            }
        }
        if let Some(job) = &mut f.capture {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(()),
                ImageProcessState::Failed(e) => bail!("SugarPot filter base capture: {e}"),
                ImageProcessState::Ready(base) => {
                    f.capture = None;
                    let source = f.source.clone();
                    let alpha = if f.state == 1 {
                        f.alpha + (255 - f.alpha) / 4
                    } else {
                        f.alpha - f.alpha / 4
                    };
                    let alpha = if f.state == 2 && alpha < 16 {
                        0
                    } else {
                        alpha.clamp(0, 255)
                    };
                    if alpha == 0 {
                        f.state = 0;
                        f.alpha = 0;
                        if f.painted {
                            self.submit(h, vec![json!({"op":"remove","id":OVERLAY})], vec![])?;
                            f.painted = false;
                        }
                        if let Some(old) = f.overlay.take() {
                            h.release_image_resource(self.client, old.path.relative())?;
                        }
                        return Ok(());
                    }
                    let mut layers = vec![];
                    for (dy, sy, height) in axis(f.position[1], f.image.size[1] as i32, 600) {
                        for (dx, sx, width) in axis(f.position[0], f.image.size[0] as i32, 1024) {
                            if width == 0 || height == 0 {
                                continue;
                            }
                            layers.push(ImageCompositeLayer {
                                input: Box::new(ImageInput::Resource(source.clone())),
                                source: [sx, sy, width, height],
                                destination: [dx, dy],
                                clip: None,
                                opacity: alpha as u8,
                                math: ImageCompositeMath::MaskScalar256,
                            });
                        }
                    }
                    let position = [
                        advance(f.position[0], f.velocity[0], 1024, f.image.size[0] as i32),
                        advance(f.position[1], f.velocity[1], 600, f.image.size[1] as i32),
                    ];
                    f.pending = Some((alpha, position));
                    //414140 scalar writes native temporary alpha, which the
                    //original opaque presenter ignores. The public result is an
                    //opaque captured presentation, with exactly the same RGB.
                    f.process = Some(
                        h.image_metadata(self.client)?.begin_process(
                            ImageInput::Composite {
                                base: Box::new(ImageInput::Resource(base)),
                                layers,
                            },
                            serde_json::to_vec(&vec![
                                crysirror_image::effects::ImageEffect::OverrideAlpha { alpha: 255 },
                            ])?
                            .into(),
                        )?,
                    );
                    return Ok(());
                }
            }
        }
        if f.state == 0 || h.now_ms() < f.next || h.now_ms() < self.transition_until {
            return Ok(());
        }
        if f.image.size[0] < 1024 || f.image.size[1] < 600 {
            //415400 intentionally draws and moves nothing for small images;
            //its independent fade state still reaches the disabled terminal.
            f.alpha = if f.state == 1 {
                f.alpha + (255 - f.alpha) / 4
            } else {
                f.alpha - f.alpha / 4
            };
            if f.state == 2 && f.alpha < 16 {
                f.alpha = 0;
                f.state = 0;
            }
            f.next = h.now_ms().saturating_add(33);
            return Ok(());
        }
        f.capture = Some(h.begin_layer_prefix_image_over(
            self.client,
            [1024, 600],
            [999, 2],
            [0, 0, 0, 255],
        )?);
        Ok(())
    }
}

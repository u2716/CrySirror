// SPDX-License-Identifier: MPL-2.0
//!44B9F0/414A10 three-family native fall policy. The fixed integer pool lives
//!in shared VFX; common PE/image/composition workers own assets and pixels.
use super::*;
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageResource};
use crysirror_vfx::cyclic_atlas::{
    AtlasNode, AtlasProjection, AtlasVariant, CyclicAtlasPool, CyclicAtlasSpec,
};
pub(super) const OVERLAY: u32 = 60003;
struct Prepare {
    worker: Option<Task<crate::resources::FallAssets>>,
    data: Option<crate::resources::FallAssets>,
    images: Vec<Arc<dyn ImageResource>>,
    job: Option<ImageJob>,
}
struct Step {
    pool: CyclicAtlasPool,
    projected: Vec<AtlasProjection>,
}
pub(super) struct Fall {
    pub state: i32,
    pub family: i32,
    pending_family: i32,
    alpha: i32,
    prepare: Option<Prepare>,
    images: Vec<Arc<dyn ImageResource>>,
    pool: Option<CyclicAtlasPool>,
    step: Option<Task<Step>>,
    candidate: Option<Step>,
    capture: Option<ImageJob>,
    process: Option<ImageJob>,
    base: Option<Arc<dyn ImageResource>>,
    pending_alpha: i32,
    next: u64,
    overlay: Option<UploadedImage>,
    painted: bool,
}
impl Fall {
    /// 414A10 within 43F430 during a UD: one fall step over a transition
    /// sample (fade, pool step, sprite layers). In-flight standalone work is
    /// dropped; the standalone tick resumes after the transition.
    pub(super) fn sample_layers(&mut self) -> Result<Option<Vec<ImageCompositeLayer>>> {
        self.step = None;
        self.candidate = None;
        self.capture = None;
        self.process = None;
        self.base = None;
        if self.state == 0 || self.prepare.is_some() || self.images.len() != 6 {
            return Ok(None);
        }
        let alpha = if self.state == 2 {
            let a = self.alpha - self.alpha / 4;
            if a < 16 { 0 } else { a }
        } else {
            self.alpha
        };
        if alpha == 0 {
            self.state = 0;
            self.alpha = 0;
            if self.pending_family != 3 {
                self.reset(self.pending_family)?;
            }
            return Ok(None);
        }
        let mut pool = self.pool.clone().context("SugarPot falling pool")?;
        let projected = pool.step(&|| Ok(()))?;
        let layers = projected
            .iter()
            .map(|sprite| ImageCompositeLayer {
                input: Box::new(ImageInput::Resource(
                    self.images[self.family as usize * 2 + sprite.variant].clone(),
                )),
                source: sprite.source,
                destination: sprite.position,
                clip: None,
                opacity: alpha as u8,
                math: ImageCompositeMath::MaskScalar256,
            })
            .collect();
        self.pool = Some(pool);
        self.alpha = alpha;
        Ok(Some(layers))
    }
    pub(super) fn take_overlay_layer(&mut self) -> bool {
        std::mem::replace(&mut self.painted, false)
    }
    pub(super) fn detached_seed(&self) -> Self {
        Self {
            state: self.state,
            family: self.family,
            pending_family: self.pending_family,
            alpha: self.alpha,
            images: self.images.clone(),
            pool: self.pool.clone(),
            next: self.next,
            ..Self::new()
        }
    }
    pub(super) fn retain_initial_image(&mut self, image: UploadedImage) {
        self.overlay = Some(image);
        self.painted = true;
    }
    pub(super) fn commit_restore_projection(&mut self, now: u64) -> Result<()> {
        if let Some(candidate) = self.candidate.take() {
            self.pool = Some(candidate.pool);
        }
        self.alpha = self.pending_alpha;
        self.next = now.saturating_add(33);
        Ok(())
    }
    pub(super) fn has_overlay(&self) -> bool {
        self.painted
    }
    pub(super) fn overlay_path(&self) -> Option<String> {
        self.overlay.as_ref().map(|i| i.path.relative().to_owned())
    }

    fn new() -> Self {
        Self {
            state: 0,
            family: 0,
            pending_family: 3,
            alpha: 0,
            prepare: None,
            images: vec![],
            pool: None,
            step: None,
            candidate: None,
            capture: None,
            process: None,
            base: None,
            pending_alpha: 0,
            next: 0,
            overlay: None,
            painted: false,
        }
    }
    pub(super) fn preparing(&self) -> bool {
        self.prepare.is_some()
            || self.step.is_some()
            || self.capture.is_some()
            || self.process.is_some()
    }
    fn reset(&mut self, family: i32) -> Result<()> {
        let family = if family < 0 {
            2
        } else if family > 2 {
            0
        } else {
            family
        };
        self.family = family;
        self.state = 1;
        self.alpha = 255;
        self.pending_family = 3;
        let pool = self
            .pool
            .as_mut()
            .context("SugarPot fall pool not prepared")?;
        let drift = (0..=64)
            .map(|i| {
                let sample =
                    (32. + 32. * (i as f64 * std::f64::consts::PI / 32.).sin()).round() as i32;
                (sample as f64 / 32. + 0.5) as i32 - 1
            })
            .collect::<Vec<_>>();
        pool.spec.variants = (0..2)
            .map(|variant| AtlasVariant {
                cell: match family {
                    1 => [
                        if variant == 0 { 16 } else { 24 },
                        if variant == 0 { 64 } else { 128 },
                    ],
                    _ => [if variant == 0 { 16 } else { 24 }; 2],
                },
                frames: if family == 2 { 16 } else { 1 },
                vertical_step: if family == 1 {
                    (variant + 1) * 64
                } else {
                    variant + 2
                },
                horizontal_step: if family == 1 { 24 } else { 0 },
                drift: if family == 1 { vec![] } else { drift.clone() },
            })
            .collect();
        pool.reset()?;
        Ok(())
    }
}
impl SugarPotVm {
    pub(super) fn restore_fall_state(
        &mut self,
        h: &mut dyn HostServices,
        state: i32,
        family: i32,
    ) -> Result<bool> {
        ensure!((0..=2).contains(&state), "SugarPot saved fall state");
        let mut fall = self.fall.take().unwrap_or_else(Fall::new);
        let result = (|| {
            if state == 0 {
                fall.state = 0;
                fall.pending_family = 3;
            } else if state == 2 {
                if fall.state == 1 {
                    fall.state = 2;
                }
            } else if fall.state == 2 {
                fall.pending_family = family;
            } else {
                if !self.prepare_fall(h, &mut fall)? {
                    return Ok(false);
                }
                fall.reset(family)?;
            }
            Ok(true)
        })();
        self.fall = Some(fall);
        result
    }
    pub(super) fn prepare_fall_restore_projection(&mut self) -> Result<bool> {
        let Some(fall) = self.fall.as_mut() else {
            return Ok(true);
        };
        if fall.state == 0 || fall.candidate.is_some() {
            return Ok(true);
        }
        if let Some(task) = &mut fall.step {
            let Some(candidate) = task.try_take()? else {
                return Ok(false);
            };
            fall.candidate = Some(candidate);
            fall.step = None;
            return Ok(true);
        }
        let alpha = if fall.state == 2 {
            let alpha = fall.alpha - fall.alpha / 4;
            if alpha < 16 { 0 } else { alpha }
        } else {
            fall.alpha
        };
        if alpha == 0 {
            fall.state = 0;
            fall.alpha = 0;
            if fall.pending_family == 3 {
                fall.pending_alpha = 0;
                return Ok(true);
            }
            fall.reset(fall.pending_family)?;
        }
        fall.pending_alpha = if alpha == 0 { 255 } else { alpha };
        let mut pool = fall
            .pool
            .as_ref()
            .context("SugarPot restored fall pool")?
            .clone();
        fall.step = Some(self.pool.as_ref().unwrap().spawn(move |cancel| {
            let projected = pool.step(&|| cancel.check())?;
            Ok(Step { pool, projected })
        })?);
        Ok(false)
    }
    pub(super) fn append_fall_restore_projection(
        &self,
        plan: &mut stage::StagePlan,
    ) -> Result<bool> {
        let Some(fall) = &self.fall else {
            return Ok(false);
        };
        let Some(step) = &fall.candidate else {
            return Ok(false);
        };
        let mut layers = Vec::with_capacity(step.projected.len());
        for sprite in &step.projected {
            let source = fall.images[fall.family as usize * 2 + sprite.variant].clone();
            //414140 opacity255 dispatches to40A1A0, which uses raw
            //source coverage. Only opacity<255 multiplies it by opacity/256.
            let input = ImageInput::Resource(source);
            layers.push(ImageCompositeLayer {
                input: Box::new(input),
                source: sprite.source,
                destination: sprite.position,
                clip: None,
                opacity: fall.pending_alpha as u8,
                math: ImageCompositeMath::MaskScalar256,
            });
        }
        //Even an empty projection owns the advanced pool; a checkpoint still
        //retains its unmodified base for the independent following filter.
        plan.images(layers);
        Ok(true)
    }
    pub(super) fn fall_command(
        &mut self,
        h: &mut dyn HostServices,
        args: &[String],
    ) -> Result<bool> {
        let selector = self.number(args, 0, 0)?;
        //The original default branch is an explicit no-op.
        if selector == 0 && self.fall.is_none() {
            return Ok(true);
        }
        if !(0..=3).contains(&selector) {
            return Ok(true);
        }
        let mut fall = self.fall.take().unwrap_or_else(Fall::new);
        let result = self.prepare_fall(h, &mut fall);
        if !result.as_ref().is_ok_and(|ready| *ready) {
            self.fall = Some(fall);
            return result;
        }
        if fall.step.is_some() || fall.capture.is_some() || fall.process.is_some() {
            self.fall = Some(fall);
            return Ok(false);
        }
        if selector == 0 {
            if fall.state == 1 {
                fall.state = 2;
            }
        } else if fall.state == 2 {
            fall.pending_family = selector - 1;
        } else {
            fall.reset(selector - 1)?;
        }
        self.fall = Some(fall);
        Ok(true)
    }
    fn prepare_fall(&mut self, h: &mut dyn HostServices, fall: &mut Fall) -> Result<bool> {
        if fall.images.len() == 6 {
            return Ok(true);
        }
        if fall.prepare.is_none() {
            let path = self
                .config
                .executable
                .clone()
                .context("SugarPot fall requires identified PE resource capability")?;
            let storage = self.storage.as_ref().unwrap().clone();
            fall.prepare =
                Some(Prepare {
                    worker: Some(self.pool.as_ref().unwrap().spawn(move |cancel| {
                        crate::resources::fall_assets(storage, path, &cancel)
                    })?),
                    data: None,
                    images: vec![],
                    job: None,
                });
        }
        let prepare = fall.prepare.as_mut().unwrap();
        if let Some(worker) = &mut prepare.worker {
            let Some(data) = worker.try_take()? else {
                return Ok(false);
            };
            prepare.data = Some(data);
            prepare.worker = None;
        }
        if let Some(job) = &mut prepare.job {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(e) => bail!("SugarPot embedded fall image: {e}"),
                ImageProcessState::Ready(resource) => {
                    prepare.images.push(resource);
                    prepare.job = None;
                }
            }
        }
        if prepare.images.len() < 6 {
            let i = prepare.images.len();
            //These IMG payloads have compressed ABM32 bodies despite their BM
            //headers. Select the existing public decoder by its actual format.
            prepare.job = Some(h.image_metadata(self.client)?.begin_encoded(
                &format!("embedded-fall-{i}.abm"),
                prepare.data.as_ref().unwrap().images[i].clone(),
            )?);
            return Ok(false);
        }
        let data = prepare.data.as_ref().unwrap();
        let nodes = (0..64)
            .map(|i| {
                let x = if i < 16 {
                    32 + i as i32 * 64
                } else {
                    -326 + i as i32 * 21
                };
                let sample_y = if i == 0 { 32 } else { data.phases[64 - i] };
                let home = [x, -((sample_y % 600) as i32)];
                AtlasNode {
                    position: home,
                    home,
                    variant: usize::from(i < 16),
                    frame: 0,
                    phase: (data.phases[i] % 65) as usize,
                    tick_phase: (i % 16) as u8,
                }
            })
            .collect();
        fall.pool = Some(CyclicAtlasPool {
            spec: CyclicAtlasSpec {
                canvas: [1024, 600],
                variants: vec![],
            },
            nodes,
        });
        fall.images = std::mem::take(&mut prepare.images);
        fall.prepare = None;
        Ok(true)
    }
    pub(super) fn fall_frame(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let Some(mut fall) = self.fall.take() else {
            return Ok(());
        };
        let result = self.poll_fall(h, &mut fall);
        self.fall = Some(fall);
        result
    }
    fn poll_fall(&mut self, h: &mut dyn HostServices, f: &mut Fall) -> Result<()> {
        if f.state == 0 || f.prepare.is_some() {
            return Ok(());
        }
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
                ImageProcessState::Failed(e) => bail!("SugarPot falling composition: {e}"),
                ImageProcessState::Ready(resource) => {
                    f.process = None;
                    f.base = None;
                    f.pool = Some(f.candidate.take().context("SugarPot fall candidate")?.pool);
                    f.alpha = f.pending_alpha;
                    let image = h.publish_image_resource(self.client, self.next_image, resource)?;
                    self.next_image = self
                        .next_image
                        .checked_add(1)
                        .context("SugarPot fall image ID")?;
                    self.submit(h,vec![json!({"op":"put","layer":{"id":OVERLAY,"rect":[0,0,1024,600],"order":[999,1],"paint":{"kind":"image","asset":image.path.relative()}}})],vec![])?;
                    f.painted = true;
                    if let Some(old) = f.overlay.replace(image) {
                        h.release_image_resource(self.client, old.path.relative())?;
                    }
                    f.next = h.now_ms().saturating_add(33);
                    return Ok(());
                }
            }
        }
        if let Some(worker) = &mut f.step {
            if let Some(candidate) = worker.try_take()? {
                f.candidate = Some(candidate);
                f.step = None;
            } else {
                return Ok(());
            }
        }
        if let Some(job) = &mut f.capture {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(()),
                ImageProcessState::Failed(e) => bail!("SugarPot fall base capture: {e}"),
                ImageProcessState::Ready(resource) => {
                    f.base = Some(resource);
                    f.capture = None;
                }
            }
        }
        if let (Some(base), Some(step)) = (&f.base, &f.candidate) {
            let mut layers = Vec::with_capacity(step.projected.len());
            for sprite in &step.projected {
                let source = f.images[f.family as usize * 2 + sprite.variant].clone();
                //414140 opacity255 dispatches to40A1A0, which uses raw
                //source coverage. Only opacity<255 multiplies it by opacity/256.
                let input = ImageInput::Resource(source);
                layers.push(ImageCompositeLayer {
                    input: Box::new(input),
                    source: sprite.source,
                    destination: sprite.position,
                    clip: None,
                    opacity: f.pending_alpha as u8,
                    math: ImageCompositeMath::MaskScalar256,
                });
            }
            f.process = Some(
                h.image_metadata(self.client)?.begin_process(
                    ImageInput::Composite {
                        base: Box::new(ImageInput::Resource(base.clone())),
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
        if f.step.is_some() || f.capture.is_some() || h.now_ms() < f.next || h.now_ms() < self.transition_until {
            return Ok(());
        }
        let alpha = if f.state == 2 {
            let a = f.alpha - f.alpha / 4;
            if a < 16 { 0 } else { a }
        } else {
            f.alpha
        };
        if alpha == 0 {
            f.state = 0;
            f.alpha = 0;
            if f.painted {
                self.submit(h, vec![json!({"op":"remove","id":OVERLAY})], vec![])?;
                f.painted = false;
            }
            if let Some(image) = f.overlay.take() {
                h.release_image_resource(self.client, image.path.relative())?;
            }
            if f.pending_family != 3 {
                f.reset(f.pending_family)?;
            } else {
                return Ok(());
            }
        }
        f.pending_alpha = if alpha == 0 { 255 } else { alpha };
        let mut pool = f.pool.as_ref().context("SugarPot falling pool")?.clone();
        f.step = Some(self.pool.as_ref().unwrap().spawn(move |cancel| {
            let projected = pool.step(&|| cancel.check())?;
            Ok(Step { pool, projected })
        })?);
        f.capture = Some(h.begin_layer_prefix_image_over(
            self.client,
            [1024, 600],
            [999, 0],
            [0, 0, 0, 255],
        )?);
        Ok(())
    }
}

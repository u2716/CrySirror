// SPDX-License-Identifier: MPL-2.0
//! Native UD2 policy over shared scene captures and stored-mask image workers.
use super::*;
use crysirror_core::image_processing::ImageResource;
type Resource = Arc<dyn ImageResource>;
pub(super) struct MaskUpdate {
    kind: i32,
    old: Option<Resource>,
    target: Option<Resource>,
    mask: Option<Resource>,
    capture: Option<ImageJob>,
    mask_decode: Option<ImageJob>,
    blend: Option<ImageJob>,
    overlay: Option<UploadedImage>,
    output: Option<Resource>,
    wave_candidate: Option<f32>,
    start: Option<u64>,
    duration: u32,
    scan_index: i32,
    stripes: update_scan::Stripes,
    /// UD25/26 radius of the current mask.
    iris: Option<i32>,
    /// The target capture slot holds the raster-mapped stage.
    raster_target: bool,
    /// A ready sample awaiting the filter composition (43F430).
    post: Option<(ImageJob, Resource)>,
}
impl MaskUpdate {
    pub(super) fn overlay_path(&self) -> Option<String> {
        self.overlay
            .as_ref()
            .map(|image| image.path.relative().to_owned())
    }
    pub(super) fn preparing(&self) -> bool {
        // The overlay lives above the capture prefix. Pending workers retain
        // the previous presented image; completed overlay samples may present.
        self.capture.is_some() || self.mask_decode.is_some() || self.blend.is_some() || self.post.is_some()
    }
}
impl SugarPotVm {
    fn finish_update(&mut self, h: &mut dyn HostServices, work: &mut MaskUpdate) -> Result<bool> {
        let removals = self.removals(h, &[60000])?;
        self.submit(h, removals, vec![])?;
        self.published_layers.retain(|id| *id != 60000);
        if let Some(image) = work.overlay.take() {
            h.release_image_resource(self.client, image.path.relative())?;
        }
        if matches!(work.kind, 10..=13) {
            self.ud_scan_counter = 0;
        }
        if matches!(work.kind, 15..=18) {
            self.ud_progressive_counter = -12;
        }
        Ok(true)
    }
    fn overlay(
        &mut self,
        h: &mut dyn HostServices,
        work: &mut MaskUpdate,
        resource: Resource,
    ) -> Result<()> {
        let image = h.publish_image_resource(self.client, self.next_image, resource.clone())?;
        work.output = Some(resource);
        self.next_image = self
            .next_image
            .checked_add(1)
            .context("SugarPot image ID exhausted")?;
        self.submit(h, vec![json!({"op":"put","layer":{"id":60000,"rect":[0,0,1024,600],"order":[2000,0],"paint":{"kind":"image","asset":image.path.relative()}}})], vec![])?;
        if let Some(old) = work.overlay.replace(image) {
            h.release_image_resource(self.client, old.path.relative())?;
        }
        if !self.published_layers.contains(&60000) {
            self.published_layers.push(60000);
        }
        Ok(())
    }
    pub(super) fn mask_update(
        &mut self,
        h: &mut dyn HostServices,
        args: &[String],
    ) -> Result<bool> {
        let mut work = if let Some(work) = self.mask_update.take() {
            work
        } else {
            // A running shared screen transition admits no prefix capture.
            if h.now_ms() < self.transition_until {
                return Ok(false);
            }
            let kind = self.number(args, 0, 0)?;
            //449870 captures/prepares a new native stage, while43F430 applies
            //fall/filter at each transition sample and raster can run at the
            //new stage's last non-SYS boundary. Reusing an old overlay here
            //would falsely report success with a stale target capture.
            // Raster distorts the stage below order 999; buttons (1000+) are
            // composed above that boundary and are not separable from it.
            ensure!(
                !self.raster.as_ref().is_some_and(|effect| effect.state != 0)
                    || !self.buttons.values().any(|b| b.sprite.visible),
                "SugarPot UD under an active raster with visible buttons"
            );
            let scan = matches!(kind,10..=13|15..=18);
            let default = if scan { 12 } else { 1000 };
            let duration = self.number(args, if kind == 2 { 2 } else { 1 }, default)?;
            let duration = if duration == 0 { default } else { duration };
            ensure!(duration > 0, "SugarPot mask duration");
            ensure!(
                !scan || duration <= 65536,
                "SugarPot update scan stride budget"
            );
            MaskUpdate {
                kind,
                old: None,
                target: None,
                mask: None,
                capture: Some(h.begin_layer_prefix_image_over(
                    self.client,
                    [1024, 600],
                    [1999, 0],
                    [0, 0, 0, 255],
                )?),
                mask_decode: None,
                blend: None,
                overlay: None,
                output: None,
                wave_candidate: None,
                start: None,
                duration: duration as u32,
                scan_index: if kind <= 13 {
                    self.ud_scan_counter
                } else {
                    self.ud_progressive_counter
                },
                stripes: Default::default(),
                iris: None,
                post: None,
                raster_target: false,
            }
        };
        // The filter is re-applied to each sample; its standalone overlay
        // (stage + filter) must not enter the target capture.
        if self.filter.as_mut().is_some_and(|f| f.take_overlay_layer()) {
            self.submit(h, vec![json!({"op":"remove","id":filter::OVERLAY})], vec![])?;
        }
        if self.fall.as_mut().is_some_and(|f| f.take_overlay_layer()) {
            self.submit(h, vec![json!({"op":"remove","id":fall::OVERLAY})], vec![])?;
        }
        if let Some(image) = self.raster.as_mut().and_then(|r| r.take_overlay()) {
            self.submit(h, vec![json!({"op":"remove","id":raster::OVERLAY})], vec![])?;
            h.release_image_resource(self.client, image.path.relative())?;
        }
        let ready = self.mask_update_step(h, args, &mut work)?;
        if !ready {
            self.mask_update = Some(work);
        }
        Ok(ready)
    }
    fn mask_update_step(
        &mut self,
        h: &mut dyn HostServices,
        args: &[String],
        work: &mut MaskUpdate,
    ) -> Result<bool> {
        if work.old.is_none() {
            match work.capture.as_mut().unwrap().poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(e) => bail!("SugarPot UD old scene: {e}"),
                ImageProcessState::Ready(resource) => {
                    work.old = Some(resource.clone());
                    self.white = None;
                    //442DB0 constructs the fresh target. Retained native
                    //temporary pixels belong only to the old capture above.
                    self.clear_stage_underlay();
                    for window in &mut self.windows {
                        window.visibility = 2;
                    }
                    self.publish(h, None)?;
                    self.overlay(h, work, resource)?;
                    // Under an active raster the new stage is captured below
                    // the raster boundary and distorted once (4383B0).
                    let boundary = if self.raster.as_ref().is_some_and(|r| r.state != 0) {
                        [999, 0]
                    } else {
                        [1999, 0]
                    };
                    work.capture = Some(h.begin_layer_prefix_image_over(
                        self.client,
                        [1024, 600],
                        boundary,
                        [0, 0, 0, 255],
                    )?);
                }
            }
        }
        if work.target.is_none() {
            match work.capture.as_mut().unwrap().poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(e) => bail!("SugarPot UD target scene: {e}"),
                ImageProcessState::Ready(resource) => {
                    work.capture = None;
                    if let Some(raster) = self.raster.as_mut().filter(|r| r.state != 0 && !work.raster_target) {
                        let spec = raster.peek_map();
                        work.capture = Some(h.image_metadata(self.client)?.begin_process(
                            ImageInput::Resource(resource),
                            serde_json::to_vec(&vec![
                                crysirror_image::effects::ImageEffect::RectMap { spec },
                            ])?
                            .into(),
                        )?);
                        work.raster_target = true;
                        return Ok(false);
                    }
                    work.target = Some(resource);
                }
            }
        }
        let scan = matches!(work.kind,10..=13|15..=18);
        let skipping = self.effect_skip || self.settings.header[4] != 0;
        //Complete the already admitted immutable sample before observing a
        //new clock/skip decision; worker errors cannot be hidden by a timer.
        let mut sample = None;
        if let Some((job, base)) = &mut work.post {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(e) => bail!("SugarPot UD filter sample: {e}"),
                ImageProcessState::Ready(presented) => {
                    sample = Some((base.clone(), presented));
                    work.post = None;
                }
            }
        }
        if sample.is_none() {
            if let Some(job) = &mut work.blend {
                match job.poll()? {
                    ImageProcessState::Pending => return Ok(false),
                    ImageProcessState::Failed(e) => bail!("SugarPot UD scalar mask: {e}"),
                    ImageProcessState::Ready(resource) => {
                        // 43F430: fall (414A10), then filter (415400).
                        let mut layers = vec![];
                        if let Some(fall) = self.fall.as_mut() {
                            layers.extend(fall.sample_layers()?.unwrap_or_default());
                        }
                        if let Some(filter) = self.filter.as_mut() {
                            layers.extend(filter.sample_layers().unwrap_or_default());
                        }
                        if !layers.is_empty() {
                            work.blend = None;
                            let job = h.image_metadata(self.client)?.begin_process(
                                ImageInput::Composite {
                                    base: Box::new(ImageInput::Resource(resource.clone())),
                                    layers,
                                },
                                serde_json::to_vec(&vec![
                                    crysirror_image::effects::ImageEffect::OverrideAlpha { alpha: 255 },
                                ])?
                                .into(),
                            )?;
                            work.post = Some((job, resource));
                            return Ok(false);
                        }
                        sample = Some((resource.clone(), resource));
                    }
                }
            }
        }
        if let Some((base, presented)) = sample {
            {
                {
                    self.overlay(h, work, presented)?;
                    work.output = Some(base);
                    if let Some(angle) = work.wave_candidate.take() {
                        self.ud_wave_phase = angle;
                    }
                    work.blend = None;
                    if scan {
                        work.mask = None;
                        work.scan_index = work
                            .scan_index
                            .checked_add(1)
                            .context("SugarPot update scan counter")?;
                        if work.kind <= 13 {
                            self.ud_scan_counter = work.scan_index;
                        } else {
                            self.ud_progressive_counter = work.scan_index;
                        }
                    }
                    return Ok(false);
                }
            }
        }
        if scan
            && work.mask_decode.is_none()
            && work.blend.is_none()
            && h.now_ms() < self.native_effect_next
        {
            return Ok(false);
        }
        if (scan
            && (skipping
                || update_scan::complete(work.kind, work.scan_index, work.duration as i32)))
            || (matches!(work.kind, 1 | 19..=39) && skipping)
        {
            return self.finish_update(h, work);
        }
        if (work.kind == 2 || scan) && work.mask.is_none() {
            if let Some(job) = &mut work.mask_decode {
                match job.poll()? {
                    ImageProcessState::Pending => return Ok(false),
                    ImageProcessState::Failed(e) => bail!("SugarPot UD mask: {e}"),
                    ImageProcessState::Ready(resource) => {
                        ensure!(
                            resource.size() == [1024, 600],
                            "SugarPot native mask dimensions"
                        );
                        work.mask = Some(resource);
                        work.mask_decode = None;
                    }
                }
            } else {
                if scan {
                    self.native_effect_next = h.now_ms().saturating_add(33);
                    update_scan::add(
                        work.kind,
                        work.scan_index,
                        work.duration as i32,
                        &mut work.stripes,
                    )?;
                    let (input, effect) = update_scan::mask(&work.stripes);
                    work.mask_decode = Some(
                        h.image_metadata(self.client)?
                            .begin_process(input, serde_json::to_vec(&vec![effect])?.into())?,
                    );
                    return Ok(false);
                }
                let name = self.string(args, 1)?;
                let path = self.resolve("grp", &name, &["msk"])?;
                let Some(bytes) = self.read(&path)? else {
                    return Ok(false);
                };
                // Native MSK uses the stored 8-bit index, not palette RGB.
                // All 65 original masks have an identity palette. Validate it
                // before using the public BMP decoder; never infer equivalence.
                ensure!(
                    bytes.len() >= 1078
                        && &bytes[..2] == b"BM"
                        && i32::from_le_bytes(bytes[18..22].try_into()?) == 1024
                        && i32::from_le_bytes(bytes[22..26].try_into()?) == 600
                        && u16::from_le_bytes(bytes[28..30].try_into()?) == 8
                        && u32::from_le_bytes(bytes[30..34].try_into()?) == 0,
                    "SugarPot mask BMP profile"
                );
                ensure!(
                    (0..256).all(|i| bytes[54 + i * 4..57 + i * 4] == [i as u8; 3]),
                    "SugarPot mask nonidentity index palette requires shared index-plane decoder"
                );
                work.mask_decode = Some(
                    h.image_metadata(self.client)?
                        .begin_encoded("native-mask.bmp", bytes)?,
                );
                return Ok(false);
            }
        }
        //449870 admits the actual MSK before the native skip branch. An
        //effects-off preference must not turn a missing/malformed mask into
        //a successful native resource operation.
        if work.kind == 2 && skipping {
            return self.finish_update(h, work);
        }
        if !scan && work.blend.is_none() && h.now_ms() < self.native_effect_next {
            return Ok(false);
        }
        if !scan && work.blend.is_none() {
            self.native_effect_next = h.now_ms().saturating_add(33);
        }
        let start = *work.start.get_or_insert_with(|| h.now_ms());
        // 435E13: 32-bit SHL9, unsigned DIV, then ADD EAX,EAX.
        // Native effect completes at duration/2; its first sample is phase 1.
        let elapsed = h.now_ms().wrapping_sub(start) as u32;
        let phase = if scan {
            0
        } else if matches!(work.kind, 2 | 31 | 32 | 34..=38) {
            (elapsed.wrapping_shl(9) / work.duration).wrapping_mul(2)
        } else {
            elapsed.wrapping_shl(8) / work.duration
        };
        if phase
            >= if matches!(work.kind, 2 | 31 | 32 | 34..=38) {
                512
            } else {
                256
            }
        {
            return self.finish_update(h, work);
        }

        if scan {
            let mut weights = vec![0; 256];
            weights[255] = 256;
            let effect = crysirror_image::effects::ImageEffect::Transition {
                spec: crysirror_image::transition::ImageTransition {
                    progress: 0.,
                    softness: 0.,
                    blur: 0.,
                    fixed_weighted: None,
                    rule_table: Some(crysirror_image::transition::RuleTable {
                        weights,
                        channel: 0,
                        operation: crysirror_image::transition::RuleOperation::Mix,
                    }),
                },
            };
            work.blend = Some(h.image_metadata(self.client)?.begin_process(
                ImageInput::Transition {
                    from: work.old.as_ref().unwrap().clone(),
                    to: work.target.as_ref().unwrap().clone(),
                    mask: work.mask.clone(),
                },
                serde_json::to_vec(&vec![effect])?.into(),
            )?);
            return Ok(false);
        }
        // UD32/37/38 ->435AB0 native30/35/36: the first half (phase <256)
        // processes the old stage with a = phase/256, the second half the
        // target with a = (512-phase)/256. 412E20 is a horizontal mean of
        // radius trunc(a*320) whose window ends one pixel past the row (a
        // radius <=1 copies the target, 408DB0); 4137D0/413980 fill random
        // runs of rand()%trunc(a*256+16)+1 pixels with the run's middle
        // pixel / truncated mean, drawing from the process C runtime stream.
        if matches!(work.kind, 32 | 37 | 38) {
            let (source, amount) = if phase < 256 {
                (work.old.clone().unwrap(), phase)
            } else {
                (work.target.clone().unwrap(), 512 - phase)
            };
            let a = f64::from((amount as f32 * 0.00390625f32).clamp(0., 1.));
            let effect = if work.kind == 32 {
                let radius = (a * 320.) as i32;
                if radius <= 1 {
                    None
                } else {
                    Some(crysirror_image::effects::ImageEffect::TruncatedRowBlur {
                        spec: crysirror_image::truncated_row_blur::TruncatedRowBlur {
                            radius: radius.min(320) as u32,
                            row_overrun: true,
                        },
                    })
                }
            } else {
                use crysirror_image::random_runs::{RandomRuns, RunColor};
                let spec = RandomRuns {
                    span: (a * 256. + 16.) as u32,
                    color: if work.kind == 37 { RunColor::Middle } else { RunColor::Mean },
                    state: self.crt_rand,
                };
                self.crt_rand = spec.state_after(1024, 600)?;
                Some(crysirror_image::effects::ImageEffect::RandomRuns { spec })
            };
            let input = if effect.is_some() { source } else { work.target.clone().unwrap() };
            let mut effects: Vec<_> = effect.into_iter().collect();
            effects.push(crysirror_image::effects::ImageEffect::OverrideAlpha { alpha: 255 });
            work.blend = Some(h.image_metadata(self.client)?.begin_process(
                ImageInput::Resource(input),
                serde_json::to_vec(&effects)?.into(),
            )?);
            return Ok(false);
        }
        // UD39 ->435AB0 native37 ->408DB0 + 411190: over the target, 1024
        // discs of the old stage (410ED0) move from (T0,T1) toward (T2,T3)
        // by trunc((T-T')*p) with radius trunc((1-p)*9), p = phase/256.
        if work.kind == 39 {
            let Some(tables) = self.disc_tables.clone() else {
                if self.disc_load.is_none() {
                    let path = self
                        .config
                        .executable
                        .clone()
                        .context("SugarPot UD39 requires the identified executable data")?;
                    let storage = self.storage.as_ref().unwrap().clone();
                    self.disc_load = Some(self.pool.as_ref().unwrap().spawn(move |cancel| {
                        crate::resources::disc_tables(storage, path, &cancel)
                    })?);
                }
                if let Some(tables) = self.disc_load.as_mut().unwrap().try_take()? {
                    self.disc_tables = Some(Arc::new(tables));
                    self.disc_load = None;
                }
                return Ok(false);
            };
            let p = f64::from(phase as f32 * 0.00390625f32);
            let radius = ((1. - p) * 9.) as i32;
            let discs = (0..1024)
                .map(|i| {
                    let [x0, y0, x1, y1] = [tables[0][i], tables[1][i], tables[2][i], tables[3][i]];
                    crysirror_image::disc_copy::DiscCopy {
                        source_center: [x0, y0],
                        destination_center: [
                            x0 - (f64::from(x0 - x1) * p) as i32,
                            y0 - (f64::from(y0 - y1) * p) as i32,
                        ],
                        radius,
                    }
                })
                .collect();
            let effects = vec![
                crysirror_image::effects::ImageEffect::DiscCopies {
                    spec: crysirror_image::disc_copy::DiscCopies { discs },
                },
                crysirror_image::effects::ImageEffect::OverrideAlpha { alpha: 255 },
            ];
            work.blend = Some(h.image_metadata(self.client)?.begin_process(
                ImageInput::Raster {
                    destination: Box::new(ImageInput::Resource(work.target.clone().unwrap())),
                    source: Box::new(ImageInput::Resource(work.old.clone().unwrap())),
                },
                serde_json::to_vec(&effects)?.into(),
            )?);
            return Ok(false);
        }
        if matches!(work.kind, 25 | 26) {
            let radius = update_iris::radius(work.kind, phase);
            if let Some(job) = &mut work.mask_decode {
                match job.poll()? {
                    ImageProcessState::Pending => return Ok(false),
                    ImageProcessState::Failed(e) => bail!("SugarPot UD circle mask: {e}"),
                    ImageProcessState::Ready(resource) => {
                        work.mask = Some(resource);
                        work.mask_decode = None;
                    }
                }
            }
            if work.iris != Some(radius) {
                let (input, effect) = update_iris::mask(radius);
                work.mask_decode = Some(
                    h.image_metadata(self.client)?
                        .begin_process(input, serde_json::to_vec(&vec![effect])?.into())?,
                );
                work.iris = Some(radius);
                return Ok(false);
            }
            let (inside, outside) = if work.kind == 25 {
                (work.old.clone().unwrap(), work.target.clone().unwrap())
            } else {
                (work.target.clone().unwrap(), work.old.clone().unwrap())
            };
            let mut weights = vec![0; 256];
            weights[255] = 256;
            let effect = crysirror_image::effects::ImageEffect::Transition {
                spec: crysirror_image::transition::ImageTransition {
                    progress: 0.,
                    softness: 0.,
                    blur: 0.,
                    fixed_weighted: None,
                    rule_table: Some(crysirror_image::transition::RuleTable {
                        weights,
                        channel: 0,
                        operation: crysirror_image::transition::RuleOperation::Mix,
                    }),
                },
            };
            work.blend = Some(h.image_metadata(self.client)?.begin_process(
                ImageInput::Transition { from: outside, to: inside, mask: work.mask.clone() },
                serde_json::to_vec(&vec![effect])?.into(),
            )?);
            return Ok(false);
        }
        if work.kind == 1 {
            //412440 copies the old stage and40A4A0's scalar path performs
            //d+floor((s-d)*phase/256), including phase255 (not a full copy).
            //The native terminal256 branch copies the target. MMX is distinct.
            let effect = crysirror_image::effects::ImageEffect::Transition {
                spec: crysirror_image::transition::ImageTransition {
                    progress: 0.,
                    softness: 0.,
                    blur: 0.,
                    rule_table: None,
                    fixed_weighted: Some(crysirror_image::fixed_mix::FixedMixSpec {
                        left_weight: 256 - phase,
                        right_weight: phase,
                        fractional_bits: 8,
                        rounding: crysirror_image::fixed_mix::FixedMixRounding::SumThenShift,
                        alpha: crysirror_image::fixed_mix::FixedMixAlpha::PreserveLeft,
                    }),
                },
            };
            work.blend = Some(h.image_metadata(self.client)?.begin_process(
                ImageInput::Transition {
                    from: work.old.as_ref().unwrap().clone(),
                    to: work.target.as_ref().unwrap().clone(),
                    mask: None,
                },
                serde_json::to_vec(&vec![effect])?.into(),
            )?);
            return Ok(false);
        }
        if matches!(work.kind, 19..=24) {
            // The old-scene variants redraw the whole frame from the original
            // old stage; the target variants accumulate on the last output.
            let base = if matches!(work.kind, 19 | 21 | 23) {
                work.old.as_ref().unwrap()
            } else {
                work.output.as_ref().unwrap_or(work.old.as_ref().unwrap())
            };
            let input = update_zoom::input(
                work.kind,
                phase,
                base.clone(),
                work.target.as_ref().unwrap().clone(),
            )?;
            work.blend = Some(
                h.image_metadata(self.client)?.begin_process(
                    input,
                    serde_json::to_vec(&vec![
                        crysirror_image::effects::ImageEffect::OverrideAlpha { alpha: 255 },
                    ])?
                    .into(),
                )?,
            );
            return Ok(false);
        }
        if matches!(work.kind, 29 | 30) {
            let input = update_bands::input(
                work.kind,
                phase,
                work.old.as_ref().unwrap().clone(),
                work.target.as_ref().unwrap().clone(),
            )?;
            work.blend = Some(
                h.image_metadata(self.client)?.begin_process(
                    input,
                    serde_json::to_vec(&vec![
                        crysirror_image::effects::ImageEffect::OverrideAlpha { alpha: 255 },
                    ])?
                    .into(),
                )?,
            );
            return Ok(false);
        }
        if matches!(work.kind, 27 | 28) {
            let input = update_angular::input(
                work.kind,
                phase,
                work.old.as_ref().unwrap().clone(),
                work.target.as_ref().unwrap().clone(),
            )?;
            work.blend = Some(
                h.image_metadata(self.client)?.begin_process(
                    input,
                    serde_json::to_vec(&vec![
                        crysirror_image::effects::ImageEffect::OverrideAlpha { alpha: 255 },
                    ])?
                    .into(),
                )?,
            );
            return Ok(false);
        }
        if work.kind == 31 {
            let input = update_mosaic::input(
                phase,
                work.old.as_ref().unwrap().clone(),
                work.target.as_ref().unwrap().clone(),
            )?;
            work.blend = Some(
                h.image_metadata(self.client)?.begin_process(
                    input,
                    serde_json::to_vec(&vec![
                        crysirror_image::effects::ImageEffect::OverrideAlpha { alpha: 255 },
                    ])?
                    .into(),
                )?,
            );
            return Ok(false);
        }
        if matches!(work.kind, 35 | 36) {
            let input = update_squeeze::input(
                work.kind,
                phase,
                work.old.as_ref().unwrap().clone(),
                work.target.as_ref().unwrap().clone(),
            )?;
            work.blend = Some(
                h.image_metadata(self.client)?.begin_process(
                    input,
                    serde_json::to_vec(&vec![
                        crysirror_image::effects::ImageEffect::OverrideAlpha { alpha: 255 },
                    ])?
                    .into(),
                )?,
            );
            return Ok(false);
        }
        if work.kind == 34 {
            let (input, next) = update_wave::input(
                phase,
                self.ud_wave_phase,
                work.old.as_ref().unwrap().clone(),
                work.target.as_ref().unwrap().clone(),
            )?;
            work.wave_candidate = Some(next);
            work.blend = Some(
                h.image_metadata(self.client)?.begin_process(
                    input,
                    serde_json::to_vec(&vec![
                        crysirror_image::effects::ImageEffect::OverrideAlpha { alpha: 255 },
                    ])?
                    .into(),
                )?,
            );
            return Ok(false);
        }
        if work.kind == 33 {
            let input = update_checker::input(
                phase,
                work.old.as_ref().unwrap().clone(),
                work.target.as_ref().unwrap().clone(),
            )?;
            work.blend = Some(
                h.image_metadata(self.client)?.begin_process(
                    input,
                    serde_json::to_vec(&vec![
                        crysirror_image::effects::ImageEffect::OverrideAlpha { alpha: 255 },
                    ])?
                    .into(),
                )?,
            );
            return Ok(false);
        }
        // 4126A0's scalar branch interpolates from new toward old as the
        // threshold decreases. Destination alpha is retained. MMX is distinct.
        let source = ImageInput::Ternary {
            destination: Box::new(ImageInput::Resource(work.target.as_ref().unwrap().clone())),
            first: Box::new(ImageInput::Resource(work.old.as_ref().unwrap().clone())),
            second: Box::new(ImageInput::Resource(work.mask.as_ref().unwrap().clone())),
        };
        let effect = crysirror_image::effects::ImageEffect::StoredMaskBlend {
            spec: crysirror_image::stored_mask::StoredMaskBlend {
                source_origin: [0, 0],
                mask_origin: [0, 0],
                parameter: 0,
                transparency: 0,
                control: 0,
                mask_channel: 0,
                threshold_phase: Some(phase.max(1) as u16),
                preserve_alpha: true,
            },
        };
        work.blend = Some(
            h.image_metadata(self.client)?
                .begin_process(source, serde_json::to_vec(&vec![effect])?.into())?,
        );
        Ok(false)
    }
}

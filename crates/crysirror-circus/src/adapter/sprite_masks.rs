//!4478E0/4442F0 resource policy. Shared immutable image workers own alpha
//!processing; every completed generation replaces all live resource aliases.
//!Frozen snapshots keep their old leases. No pixel loop lives in this adapter.
use super::*;
use crysirror_core::image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource};
use crysirror_core::service::SteppedRamp;
use crysirror_scene::layer::queued::{Active, SequenceTrack};
#[derive(Clone)]
pub(super) struct MaskDefinition {
    mask: UploadedImage,
    /// The declared rule is a resource dependency, independent of slot21.
    rule_lease: Option<Arc<dyn ImageResource>>,
    mode: bool,
    ticks: i32,
    alpha: f32,
    alpha_applied: bool,
}
struct Running {
    bank: u8,
    steps: u32,
    ramp: SteppedRamp,
    restored_alpha: bool,
    offset: f32,
    last_rendered: f32,
    baseline: Option<Arc<dyn ImageResource>>,
    rule: Option<Arc<dyn ImageResource>>,
    baseline_job: Option<ImageJob>,
    rule_job: Option<ImageJob>,
    pending: Option<Mutation>,
}
struct Mutation {
    job: ImageJob,
    source: String,
    frame: u32,
    ramp: SteppedRamp,
    offset: f32,
    restore_scalar: bool,
    complete: bool,
    preview: bool,
    ready: Option<Arc<dyn ImageResource>>,
}

#[derive(Default)]
pub(super) struct MaskQueues {
    actual: BTreeMap<u8, MaskDefinition>,
    captured: BTreeMap<u8, MaskDefinition>,
    running: Option<Running>,
    finished: bool,
    forced: bool,
}
impl Bridge<'_> {
    pub(in crate::adapter) fn declare_sprite_mask(
        &mut self,
        seed: &crate::machine::sprite_ops::SpriteSeed,
        mode: bool,
        ticks: i32,
    ) -> Result<bool> {
        ensure!(
            seed.object < 150,
            "Circus shader target has no retained bitmap drawable"
        );
        let Some(mask) = self.services.images.get(&21).cloned() else {
            return Ok(true);
        };
        if !self.native_sprite(&SpriteRequest {
            commands: vec![crate::machine::sprite_ops::SpriteCommand {
                seed: seed.clone(),
                operations: vec![],
            }],
        })? {
            return Ok(false);
        }
        let state = self
            .primary_sprite_states()?
            .into_iter()
            .find(|(object, _)| *object == seed.object)
            .context("Circus mask actor missing")?
            .1;
        if state
            .lanes
            .get(&7)
            .is_some_and(|lane| !lane.records.is_empty())
        {
            return Ok(true);
        }
        let mut ramp = SteppedRamp::new(
            if mode { -255. } else { 255. },
            if mode { 255. } else { -255. },
            ticks,
        )?;
        let mut frames = 0u32;
        if ticks > 0 {
            while ramp.active() {
                ramp.advance_by(1, false)?;
                frames += 1;
                ensure!(frames <= 1_000_000, "Circus mask counter budget");
            }
        } else {
            // A negative native divisor never reaches its boundary. The
            // shared cursor remains an effectively unbounded u32 tick driver.
            frames = u32::MAX;
        }
        let bank = if state.capture {
            u8::try_from(state.captured.get(&7).map_or(0, |records| records.len()))?
        } else {
            0
        };
        let rule_lease = self
            .host
            .published_image_resource(self.client, mask.path.relative())?;
        ensure!(
            rule_lease
                .as_ref()
                .is_none_or(|rule| rule.client() == self.client),
            "Circus foreign declared mask rule"
        );
        let definition = MaskDefinition {
            mask: mask.clone(),
            rule_lease,
            mode,
            ticks,
            alpha: state.values[2],
            alpha_applied: false,
        };
        let queues = self.services.primary.masks.entry(seed.object).or_default();
        if state.capture {
            queues.captured.insert(bank, definition);
        } else {
            queues.actual.insert(bank, definition);
        }
        queues.finished = false;
        let mut operations = Vec::new();
        if mode {
            operations.push(SequenceOperation::Set {
                channels: vec![2],
                values: vec![0.],
                retained: false,
            });
        }
        operations.push(SequenceOperation::Enqueue {
            lane: 7,
            bank: Some(bank),
            track: SequenceTrack::Frames {
                channel: 7,
                frames,
                hold_ticks: -1,
                looping: ticks < 0,
                capture: true,
            },
        });
        self.native_sprite(&SpriteRequest {
            commands: vec![crate::machine::sprite_ops::SpriteCommand {
                seed: seed.clone(),
                operations,
            }],
        })
    }
    pub(super) fn mirror_sprite_mask_operations(&mut self, request: &SpriteRequest) -> Result<()> {
        for command in &request.commands {
            let Some(queues) = self.services.primary.masks.get_mut(&command.seed.object) else {
                continue;
            };
            for operation in &command.operations {
                match operation {
                    SequenceOperation::Capture { enabled: true, .. } => queues.captured.clear(),
                    SequenceOperation::Finish { lanes, .. }
                        if lanes.as_ref().is_none_or(|lanes| lanes.contains(&7)) =>
                    {
                        queues.forced = true
                    }
                    SequenceOperation::Reset { .. } => {
                        queues.running = None;
                        queues.finished = false;
                    }
                    _ => {}
                }
            }
            if command
                .operations
                .iter()
                .any(|op| matches!(op, SequenceOperation::Replay))
                && !queues.captured.is_empty()
            {
                let current: Option<SequenceStatus> =
                    serde_json::from_slice(&self.host.query_engine(
                        self.client,
                        EngineQuery::LayerSequence(actor(command.seed.object)),
                    )?)?;
                queues.finished = false;
                if let Some(lane) = current.as_ref().and_then(|state| state.lanes.get(&7)) {
                    let definitions = queues.captured.values().cloned().collect::<Vec<_>>();
                    let mut ordinal = 0;
                    for bank in lane.records.keys() {
                        if !queues.actual.contains_key(bank) {
                            queues
                                .actual
                                .insert(*bank, definitions[ordinal % definitions.len()].clone());
                            ordinal += 1;
                        }
                    }
                }
            }
        }
        Ok(())
    }
    fn mask_image_input(&mut self, image: &UploadedImage) -> Result<ImageInput> {
        Ok(
            if let Some(resource) = self
                .host
                .published_image_resource(self.client, image.path.relative())?
            {
                ensure!(
                    resource.client() == self.client,
                    "Circus foreign alpha resource"
                );
                ImageInput::Resource(resource)
            } else {
                ImageInput::Asset {
                    path: image.path.relative().to_owned(),
                    size: image.size,
                    fingerprint: None,
                }
            },
        )
    }
    fn mask_target(&mut self, object: u16) -> Result<Option<UploadedImage>> {
        let mut layer: Value = serde_json::from_slice(&self.host.query_engine(
            self.client,
            EngineQuery::Layer(super::super::sprite_resources::alias(object)),
        )?)?;
        if layer["paint"]["kind"] != "image" {
            //447AC0 uses the same renderer188 resource while its decoded PCK
            //frame is presented by a separate retained leaf.
            layer = serde_json::from_slice(&self.host.query_engine(
                self.client,
                EngineQuery::Layer(0x3e00_0000 + u32::from(object)),
            )?)?;
        }
        if layer["paint"]["kind"] != "image" {
            return Ok(None);
        }
        let path = layer["paint"]["asset"]
            .as_str()
            .context("Circus mask target asset")?;
        if let Some(image) = self
            .services
            .images
            .values()
            .find(|image| image.path.relative() == path)
        {
            return Ok(Some(image.clone()));
        }
        if let Some(resource) = self.host.published_image_resource(self.client, path)? {
            ensure!(
                resource.client() == self.client,
                "Circus foreign target resource"
            );
            return Ok(Some(UploadedImage {
                path: VfsPath::new(Mount::Asset, path)?,
                size: resource.size(),
                opaque: resource.opaque(),
            }));
        }
        bail!("Circus mask target has no retained resource lease")
    }
    fn publish_mask_generation(
        &mut self,
        source: &str,
        image: Arc<dyn ImageResource>,
        object: u16,
    ) -> Result<()> {
        ensure!(
            image.client() == self.client,
            "Circus foreign prepared alpha resource"
        );
        let uploaded = self.host.publish_image_resource(
            self.client,
            0x3503_0000 + u32::from(object),
            image,
        )?;
        // One native bitmap may be held in several resource slots and drawable
        // aliases. This owner-scoped operation changes live paints atomically;
        // it deliberately does not rewrite frozen Snapshot paints.
        self.submit(
            vec![json!({"op":"rebind_image","from":source,"to":uploaded.path.relative()})],
            vec![],
        )?;
        for (slot, current) in &mut self.services.images {
            if current.path.relative() == source {
                *current = uploaded.clone();
            }
        }
        self.rebind_packed_image(source,&uploaded)?;
        self.host.release_image_resource(self.client, source)?;
        Ok(())
    }
    fn poll_mask_mutation(
        &mut self,
        object: u16,
        queues: &mut MaskQueues,
        commit_preview: bool,
    ) -> Result<bool> {
        let Some(run) = queues.running.as_mut() else {
            return Ok(true);
        };
        let Some(pending) = run.pending.as_mut() else {
            return Ok(true);
        };
        if pending.ready.is_none() {
            pending.ready = Some(match pending.job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => bail!("Circus shared alpha worker: {error}"),
                ImageProcessState::Ready(image) => image,
            });
        }
        if pending.preview && !commit_preview {
            return Ok(true);
        }
        let image = pending.ready.take().unwrap();
        let pending = run.pending.take().unwrap();
        self.publish_mask_generation(&pending.source, image, object)?;
        if pending.restore_scalar {
            let definition = queues
                .actual
                .get_mut(&run.bank)
                .context("Circus mask definition disappeared")?;
            self.submit(vec![json!({"op":"sequence","id":actor(object),"operation":{"kind":"set","channels":[2],"values":[definition.alpha],"retained":false}})],vec![])?;
            definition.alpha_applied = true;
            run.restored_alpha = true;
        }
        run.steps = pending.frame;
        run.ramp = pending.ramp;
        run.offset = pending.offset;
        run.last_rendered = pending.offset;
        if pending.complete {
            queues.actual.remove(&run.bank);
            queues.finished = true;
            queues.forced = false;
            queues.running = None;
        }
        Ok(true)
    }
    fn prepare_mask_resource(
        &mut self,
        object: u16,
        queues: &mut MaskQueues,
        bank: u8,
        frame: u32,
        complete: bool,
        force: bool,
        preview: bool,
    ) -> Result<bool> {
        if force {
            if let Some(run) = queues.running.as_mut() {
                if run.pending.as_ref().is_some_and(|pending| pending.preview) {
                    run.pending = None;
                }
            }
        }
        let commit_preview = !preview
            && queues
                .running
                .as_ref()
                .and_then(|run| run.pending.as_ref())
                .is_none_or(|pending| frame >= pending.frame);
        if !self.poll_mask_mutation(object, queues, commit_preview)? {
            return Ok(false);
        }
        if queues
            .running
            .as_ref()
            .is_some_and(|run| run.pending.is_some())
        {
            return Ok(true);
        }
        let Some(definition) = queues.actual.get(&bank).cloned() else {
            return Ok(true);
        };
        if queues
            .running
            .as_ref()
            .is_none_or(|run| run.bank != bank || (!force && frame < run.steps))
        {
            queues.running = Some(Running {
                bank,
                steps: 0,
                ramp: SteppedRamp::new(
                    if definition.mode { -255. } else { 255. },
                    if definition.mode { 255. } else { -255. },
                    definition.ticks,
                )?,
                restored_alpha: definition.alpha_applied,
                offset: if definition.mode { -255. } else { 255. },
                last_rendered: 0.,
                baseline: None,
                rule: definition.rule_lease.clone(),
                baseline_job: None,
                rule_job: None,
                pending: None,
            });
        }
        let run = queues.running.as_mut().unwrap();
        let desired = if force {
            run.steps
                .checked_add(1)
                .context("Circus forced mask phase")?
        } else {
            frame
        };
        if desired <= run.steps {
            return Ok(true);
        }
        let mut ramp = run.ramp.clone();
        let mut offset = run.offset;
        let mut steps = run.steps;
        while steps < desired {
            offset = ramp.advance_by(1, false)?.unwrap_or(offset);
            steps += 1;
        }
        //4442F0 compares the signed integer offset before locking either image.
        //A forced drain repeats its native update until that integer changes.
        if force {
            let mut retries = 0;
            while (offset as i32) == (run.last_rendered as i32) {
                offset = ramp.advance_by(1, false)?.unwrap_or(offset);
                steps = steps.checked_add(1).context("Circus mask phase")?;
                retries += 1;
                ensure!(
                    retries <= 1_000_000,
                    "Circus mask drain did not change its integer phase"
                );
            }
        }
        if (offset as i32) == (run.last_rendered as i32) {
            run.steps = steps;
            run.ramp = ramp;
            run.offset = offset;
            return Ok(true);
        }
        let Some(target) = self.mask_target(object)? else {
            queues.actual.remove(&bank);
            queues.running = None;
            queues.finished = true;
            return Ok(true);
        };
        if run.baseline.is_none() {
            if run.baseline_job.is_none() {
                let input = self.mask_image_input(&target)?;
                run.baseline_job = Some(
                    self.host
                        .image_metadata(self.client)?
                        .begin_process(input, Arc::from(&b"[]"[..]))?,
                );
            }
            match run.baseline_job.as_mut().unwrap().poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => bail!("Circus alpha baseline worker: {error}"),
                ImageProcessState::Ready(image) => {
                    run.baseline = Some(image);
                    run.baseline_job = None;
                }
            }
        }
        if run.rule.is_none() {
            if run.rule_job.is_none() {
                let input = self.mask_image_input(&definition.mask)?;
                run.rule_job = Some(
                    self.host
                        .image_metadata(self.client)?
                        .begin_process(input, Arc::from(&b"[]"[..]))?,
                );
            }
            match run.rule_job.as_mut().unwrap().poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => bail!("Circus alpha rule worker: {error}"),
                ImageProcessState::Ready(image) => {
                    run.rule = Some(image);
                    run.rule_job = None;
                }
            }
        }
        let input = ImageInput::Ternary {
            destination: Box::new(self.mask_image_input(&target)?),
            first: Box::new(ImageInput::Resource(run.baseline.as_ref().unwrap().clone())),
            second: Box::new(ImageInput::Resource(run.rule.as_ref().unwrap().clone())),
        };
        let complete = complete || force;
        let effects:Arc<[u8]>=serde_json::to_vec(&json!([{"kind":"alpha_mask_plane","spec":{"channel":2,"offset":offset,"restore":complete&&definition.mode}}]))?.into();
        run.pending = Some(Mutation {
            job: self
                .host
                .image_metadata(self.client)?
                .begin_process(input, effects)?,
            source: target.path.relative().to_owned(),
            frame: steps,
            ramp,
            offset,
            restore_scalar: definition.mode && !run.restored_alpha,
            complete,
            preview,
            ready: None,
        });
        self.poll_mask_mutation(object, queues, !preview)
    }
    /// Owner dependency gate for a mutation already started by a VM command.
    pub(in crate::adapter) fn sprite_mask_resources_ready(&mut self) -> Result<bool> {
        let objects = self
            .services
            .primary
            .masks
            .keys()
            .copied()
            .collect::<Vec<_>>();
        for object in objects {
            let mut queues = self.services.primary.masks.remove(&object).unwrap();
            let result = self.poll_mask_mutation(object, &mut queues, false);
            self.services.primary.masks.insert(object, queues);
            if !result? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn prepare_mask_states(&mut self, preview: bool) -> Result<bool> {
        if !self.sprite_mask_resources_ready()? {
            return Ok(false);
        }
        let states = self.primary_sprite_states()?;
        for (object, mut state) in states {
            if !self.services.primary.masks.contains_key(&object) {
                continue;
            }
            if preview {
                state =
                    serde_json::from_slice::<Option<SequenceStatus>>(&self.host.query_engine(
                        self.client,
                        EngineQuery::LayerSequencePreview {
                            id: actor(object),
                            rate: state.values[16],
                            fast: self.native_effect_fast(),
                        },
                    )?)?
                    .context("Circus mask preview owner")?;
            }
            let Some(lane) = state.lanes.get(&7) else {
                continue;
            };
            let Some(Active::Frames { frame, .. }) = lane.last_active.as_ref() else {
                continue;
            };
            let active = lane.active.is_some();
            let mut queues = self.services.primary.masks.remove(&object).unwrap();
            let bank = if active {
                lane.cursor as u8
            } else {
                queues.running.as_ref().map_or_else(
                    || queues.actual.keys().next().copied().unwrap_or(0),
                    |run| run.bank,
                )
            };
            let result = self.prepare_mask_resource(
                object,
                &mut queues,
                bank,
                *frame,
                !active,
                false,
                preview,
            );
            self.services.primary.masks.insert(object, queues);
            if !result? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    /// Prepare the exact next externally-clocked sequence sample before any
    /// native object advances. A resource worker can never advance the clock.
    pub(in crate::adapter) fn prepare_sprite_mask_frame(&mut self) -> Result<bool> {
        self.prepare_mask_states(true)
    }
    pub(in crate::adapter) fn refresh_sprite_masks(&mut self) -> Result<()> {
        ensure!(
            self.prepare_mask_states(false)?,
            "Circus committed mask tick has an unprepared image dependency"
        );
        Ok(())
    }
    ///444A4C0/447060 can drain several banks in one call. Each bank snapshots
    ///the preceding completed generation before its own immutable worker runs.
    pub(in crate::adapter) fn prepare_forced_sprite_masks(
        &mut self,
        request: &SpriteRequest,
    ) -> Result<bool> {
        for command in &request.commands {
            if !command.operations.iter().any(|operation|matches!(operation,SequenceOperation::Finish{cancel:false,lanes} if lanes.as_ref().is_none_or(|lanes|lanes.contains(&7)))){continue}
            let object = command.seed.object;
            if !self.services.primary.masks.contains_key(&object) {
                continue;
            }
            let mut queues = self.services.primary.masks.remove(&object).unwrap();
            let result = (|| -> Result<bool> {
                while let Some(bank) = queues.actual.keys().next().copied() {
                    if !self.prepare_mask_resource(
                        object,
                        &mut queues,
                        bank,
                        0,
                        true,
                        true,
                        false,
                    )? {
                        return Ok(false);
                    }
                }
                Ok(true)
            })();
            self.services.primary.masks.insert(object, queues);
            if !result? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub(in crate::adapter) fn finish_sprite_mask(&mut self, object: u16) -> Result<bool> {
        let Some((_, state)) = self
            .primary_sprite_states()?
            .into_iter()
            .find(|(id, _)| *id == object)
        else {
            return Ok(true);
        };
        let seed = crate::machine::sprite_ops::SpriteSeed {
            object,
            values: state.values,
            retained: state.retained,
        };
        self.native_sprite(&SpriteRequest {
            commands: vec![crate::machine::sprite_ops::SpriteCommand {
                seed,
                operations: vec![SequenceOperation::Finish {
                    cancel: false,
                    lanes: Some(vec![7]),
                }],
            }],
        })
    }
}

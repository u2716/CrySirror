// SPDX-License-Identifier: MPL-2.0
//! Aether 77a93db4's DrawLegacyTransition helper, not a native NSS command.
//! Quantized duration and 128-step accumulated coverage follow its translator
//! and LegacyTransitionCoverage. Shared presenter/worker services own captures,
//! mask sampling and image composition; no renderer is embedded in this VM.
use super::native_bitmap::NativeBitmap;
use super::*;
use crysirror_core::image_processing::{
    ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageJob, ImageProcessState, ImageResource,
};

pub(super) struct Preparing {
    ids: Vec<u32>,
    order: i32,
    duration: u64,
    dynamic: bool,
    before: ImageJob,
    after: ImageJob,
    mask: Option<NativeBitmap>,
    old: Option<Arc<dyn ImageResource>>,
    incoming: Option<Arc<dyn ImageResource>>,
    rule: Option<Arc<dyn ImageResource>>,
    mask_resize: Option<ImageJob>,
}
pub(super) struct Transition {
    ids: Vec<u32>,
    before: Arc<dyn ImageResource>,
    incoming: Arc<dyn ImageResource>,
    rule: Option<Arc<dyn ImageResource>>,
    dynamic: bool,
    start: u64,
    duration: u64,
    last_step: i32,
    capture: Option<ImageJob>,
    composite: Option<ImageJob>,
    composed: Option<Arc<dyn ImageResource>>,
    pending: Option<(i32, ImageJob)>,
}
impl N2SystemVm {
    fn legacy_scene(&self, host: &mut dyn HostServices, ids: &[u32]) -> Result<ImageJob> {
        let mut layers = BTreeMap::new();
        for &id in ids {
            if !self.entities.nodes.contains_key(&id) {
                continue;
            }
            let mut next = Some(id);
            for depth in 0..128 {
                let Some(at) = next else {
                    break;
                };
                if layers.contains_key(&at) {
                    break;
                }
                let mut layer = self.layer(host, at)?;
                next = layer["parent"].as_u64().map(|v| v as u32);
                if !ids.contains(&at) {
                    layer["paint"] = json!({"kind":"group"});
                }
                layers.insert(at, json!({"op":"put","layer":layer}));
                ensure!(
                    depth < 127 || next.is_none(),
                    "N2System legacy scene ancestry"
                );
            }
        }
        let commands = serde_json::to_vec(&layers.into_values().collect::<Vec<_>>())?;
        Ok(host.begin_detached_layers_image(
            self.client,
            self.config.canvas,
            [i32::MAX, i32::MAX],
            &commands,
        )?)
    }
    pub(super) fn native_legacy_transition(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        state: &mut Execution,
        args: &Arguments<'_>,
    ) -> Result<bool> {
        ensure!(
            (5..=6).contains(&args.len()),
            "N2System DrawLegacyTransition signature"
        );
        if state.legacy_transition.is_none() {
            let duration = args.time(1)? / 128 * 128;
            ensure!(duration <= 600_000, "N2System legacy transition duration");
            let masked = args.truth(3)?;
            let dynamic = args.len() > 5 && args.truth(5)?;
            ensure!(
                !masked || !dynamic,
                "masked dynamic legacy transition is not defined by the reference"
            );
            let mut ids = BTreeSet::new();
            for root in self.query(current, &args.string(0)?)? {
                for id in self.entities.subtree(root) {
                    if matches!(self.entities.nodes[&id].kind, Kind::Visual)
                        && self.layer(host, id)?["visible"] != false
                    {
                        ids.insert(id);
                    }
                }
            }
            if duration == 0 || ids.is_empty() {
                self.machine.complete(&mut state.thread, Value::Null)?;
                return Ok(true);
            }
            ensure!(
                self.legacy_transitions.is_empty(),
                "N2System overlapping legacy transitions"
            );
            let mut order = i32::MIN;
            for &id in &ids {
                order = order.max(
                    self.layer(host, id)?["order"][0]
                        .as_i64()
                        .context("legacy order")? as i32,
                );
            }
            order = order
                .checked_add(1)
                .context("N2System legacy overlay order")?;
            let mask = if masked {
                let Some(read) = self.read_native_bitmap(host, current, &args.string(2)?)? else {
                    return Ok(false);
                };
                Some(read)
            } else {
                None
            };
            let ids: Vec<_> = ids.into_iter().collect();
            state.legacy_transition = Some(Preparing {
                before: host.begin_frame_capture(self.client)?,
                after: self.legacy_scene(host, &ids)?,
                ids,
                order,
                duration,
                dynamic,
                mask,
                old: None,
                incoming: None,
                rule: None,
                mask_resize: None,
            });
        }
        let p = state.legacy_transition.as_mut().unwrap();
        if p.old.is_none() {
            p.old = poll(&mut p.before)?;
        }
        if p.incoming.is_none() {
            p.incoming = poll(&mut p.after)?;
        }
        if p.rule.is_none() {
            if let Some(mask) = &mut p.mask {
                p.rule = mask.poll(
                    self.client,
                    self.pool.as_ref().unwrap(),
                    self.images.as_ref().unwrap(),
                )?;
            }
        }
        if p.old.is_none() || p.incoming.is_none() || (p.mask.is_some() && p.rule.is_none()) {
            return Ok(false);
        }
        if let Some(rule) = &p.rule {
            if rule.size() != self.config.canvas {
                if p.mask_resize.is_none() {
                    let effects = Arc::from(serde_json::to_vec(
                        &json!([{"kind":"rect_map","spec":{"output_size":self.config.canvas,"clear":[0,0,0,0],"regions":[{"source":[0,0,rule.size()[0],rule.size()[1]],"destination":[0,0,self.config.canvas[0],self.config.canvas[1]]}],"sample":"floor","copy_alpha":true}}]),
                    )?);
                    match self
                        .images
                        .as_ref()
                        .unwrap()
                        .begin_process(ImageInput::Resource(rule.clone()), effects)
                    {
                        Ok(job) => p.mask_resize = Some(job),
                        Err(Error::Limit("image preparation task capacity")) => return Ok(false),
                        Err(e) => return Err(e.into()),
                    }
                }
                let Some(image) = poll(p.mask_resize.as_mut().unwrap())? else {
                    return Ok(false);
                };
                p.rule = Some(image);
                p.mask_resize = None;
            }
        }
        let p = state.legacy_transition.take().unwrap();
        self.legacy_serial = self
            .legacy_serial
            .checked_add(1)
            .context("N2System legacy transition identity")?;
        let id = self.create(
            host,
            current,
            &format!("__legacy_transition_{}", self.legacy_serial),
            Kind::Visual,
            self.config.canvas.map(|v| v as f32),
        )?;
        let old = p.old.unwrap();
        let uploaded = host.publish_image_resource(self.client, id, old.clone())?;
        let path = uploaded.path.relative().to_owned();
        self.submit(host,vec![json!({"op":"put","layer":{"id":id,"parent":null,"order":[p.order,0],"rect":[0,0,self.config.canvas[0],self.config.canvas[1]],"paint":{"kind":"image","asset":path}}})],vec![])?;
        self.captured_images.insert(id, path);
        self.legacy_transitions.insert(
            id,
            Transition {
                ids: p.ids,
                before: old,
                incoming: p.incoming.unwrap(),
                rule: p.rule,
                dynamic: p.dynamic,
                start: host.now_ms(),
                duration: p.duration,
                last_step: -1,
                capture: None,
                composite: None,
                composed: None,
                pending: None,
            },
        );
        if args.anim_wait(4, p.duration)? > 0 {
            state.wait = Wait::Actions {
                ids: vec![id],
                until: None,
            };
        }
        self.machine.complete(&mut state.thread, Value::Null)?;
        Ok(true)
    }
    pub(super) fn update_legacy_transitions(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let ids: Vec<_> = self.legacy_transitions.keys().copied().collect();
        for id in ids {
            let now = host.now_ms();
            let done = {
                let t = &self.legacy_transitions[&id];
                now.saturating_sub(t.start) >= t.duration
            };
            if done {
                self.legacy_transitions.remove(&id);
                self.destroy(host, id)?;
                continue;
            }
            let t = self.legacy_transitions.get_mut(&id).unwrap();
            if let Some((step, job)) = &mut t.pending {
                let Some(image) = poll(job)? else {
                    continue;
                };
                t.last_step = *step;
                if t.dynamic {
                    t.before = image.clone();
                }
                t.pending = None;
                self.publish_capture(host, id, image)?;
            }
            let t = &self.legacy_transitions[&id];
            let step = ((now.saturating_sub(t.start) * 128 / t.duration) as i32).min(127);
            if step <= t.last_step {
                continue;
            }
            if t.dynamic && t.capture.is_none() {
                let job = self.legacy_scene(host, &t.ids)?;
                self.legacy_transitions.get_mut(&id).unwrap().capture = Some(job);
            }
            let t = self.legacy_transitions.get_mut(&id).unwrap();
            if let Some(job) = &mut t.capture {
                let Some(image) = poll(job)? else {
                    continue;
                };
                t.incoming = image;
                t.capture = None;
            }
            let size = self.config.canvas;
            if t.composed.is_none() {
                if t.composite.is_none() {
                    let incoming = ImageInput::Composite {
                        base: Box::new(ImageInput::Resource(t.before.clone())),
                        layers: vec![ImageCompositeLayer {
                            input: Box::new(ImageInput::Resource(t.incoming.clone())),
                            source: [0, 0, size[0] as i32, size[1] as i32],
                            destination: [0, 0],
                            clip: None,
                            opacity: 255,
                            math: ImageCompositeMath::AlphaOnAlphaScalar256,
                        }],
                    };
                    match self
                        .images
                        .as_ref()
                        .unwrap()
                        .begin_process(incoming, Arc::from(&b"[]"[..]))
                    {
                        Ok(job) => t.composite = Some(job),
                        Err(Error::Limit("image preparation task capacity")) => continue,
                        Err(e) => return Err(e.into()),
                    }
                }
                let Some(image) = poll(t.composite.as_mut().unwrap())? else {
                    continue;
                };
                t.composed = Some(image);
                t.composite = None;
            }
            let first = if t.dynamic { t.last_step + 1 } else { 0 };
            let weights: Vec<u16> = (0..256)
                .map(|mask| {
                    let mut remaining = 1.;
                    for i in first..=step {
                        let value = if t.rule.is_some() {
                            (8 * i - mask).clamp(0, 255)
                        } else {
                            2 * i
                        };
                        if value >= 255 {
                            remaining = 0.;
                            break;
                        }
                        remaining *= f64::from(256 - value) / 256.;
                    }
                    ((1. - remaining) * 256.).round().clamp(0., 256.) as u16
                })
                .collect();
            let input = ImageInput::Transition {
                from: t.before.clone(),
                to: t.composed.as_ref().unwrap().clone(),
                mask: t.rule.clone(),
            };
            let effects = Arc::from(serde_json::to_vec(
                &json!([{"kind":"transition","spec":{"progress":0.,"softness":0.,"blur":0.,"rule_table":{"weights":weights,"channel":0,"operation":"mix"}}}]),
            )?);
            match self.images.as_ref().unwrap().begin_process(input, effects) {
                Ok(job) => {
                    t.pending = Some((step, job));
                    if t.dynamic {
                        t.composed = None;
                    }
                }
                Err(Error::Limit("image preparation task capacity")) => {}
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }
}
fn poll(job: &mut ImageJob) -> Result<Option<Arc<dyn ImageResource>>> {
    match job.poll()? {
        ImageProcessState::Pending => Ok(None),
        ImageProcessState::Ready(image) => Ok(Some(image)),
        ImageProcessState::Failed(e) => bail!("N2System legacy transition: {e}"),
    }
}

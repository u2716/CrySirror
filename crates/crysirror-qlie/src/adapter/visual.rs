use super::*;
use crysirror_core::{
    image_processing::ImageProcessState,
    service::{EngineQuery, MediaPhase, MediaRequest},
};
impl QlieVm {
    pub(super) fn load_image(
        &mut self,
        h: &mut dyn HostServices,
        id: &str,
        path: &str,
    ) -> Result<bool> {
        if [
            ".ogv", ".ogm", ".avi", ".mp4", ".webm", ".mpg", ".mpeg", ".wmv",
        ]
        .iter()
        .any(|ext| path.to_ascii_lowercase().ends_with(ext))
        {
            return self.load_video_image(h, id, path);
        }
        if !self.objects[id].preparing {
            let mut old = self.objects.remove(id).unwrap();
            let mut generator = old.generator.take();
            if let Some(group) = &mut generator {
                group.retire_registration();
            }
            let layer = old.layer;
            let kind = old.kind.clone();
            let position = old.position;
            let gain = old.gain;
            let props = old.properties.clone();
            let mut message = old.message.take();
            if let Some(message) = &mut message {
                message.created = false;
                message.bound_parent = None;
            }
            self.free_visual(h, &old)?;
            self.objects.insert(
                id.into(),
                Object {
                    kind,
                    layer,
                    position,
                    gain,
                    properties: props,
                    message,
                    generator,
                    preparing: true,
                    asset: Some(path.into()),
                    ..Default::default()
                },
            );
            let storage = h.storage(self.client)?;
            let source = path.to_owned();
            let client = self.client;
            self.load = Some((
                path.into(),
                self.pool
                    .spawn(move |cancel| resources::load(storage, client, &source, &cancel))?,
            ));
        }
        ensure!(
            self.objects[id].asset.as_deref() == Some(path),
            "QLIE image continuation owner"
        );
        if let Some((expected, job)) = &mut self.load {
            ensure!(expected == path, "QLIE resource completion identity");
            let Some(loaded) = job.try_take()? else {
                return Ok(false);
            };
            self.load = None;
            let o = self.objects.get_mut(id).unwrap();
            match loaded {
                resources::Loaded::Blob(bytes) => o.pending_blob = Some((path.into(), bytes)),
                resources::Loaded::Bundle(bundle) => o.bundle = Some(bundle),
            }
        }
        loop {
            let index = self.objects[id].images.len();
            if let Some(job) = &mut self.image_job {
                match job.poll()? {
                    ImageProcessState::Pending => return Ok(false),
                    ImageProcessState::Failed(e) => bail!("QLIE shared image decode: {e}"),
                    ImageProcessState::Ready(image) => {
                        self.next_image = self
                            .next_image
                            .checked_add(1)
                            .context("QLIE image id exhausted")?;
                        let published =
                            h.publish_image_resource(self.client, self.next_image, image)?;
                        self.image_job = None;
                        let o = self.objects.get_mut(id).unwrap();
                        o.images.push(Some(published));
                        if let Some(bundle) = &mut o.bundle {
                            bundle
                                .slot_mut(index)
                                .context("QLIE decoded image slot")?
                                .bytes = Arc::from([]);
                        } else {
                            o.pending_blob = None;
                        }
                        continue;
                    }
                }
            }
            let o = self.objects.get_mut(id).unwrap();
            let next = if let Some(bundle) = &o.bundle {
                bundle.slot(index).map(|member| {
                    (
                        member.name.clone(),
                        if member.nested.is_some() || member.metadata["kind"] == 7 {
                            Arc::from([])
                        } else {
                            member.bytes.clone()
                        },
                    )
                })
            } else {
                o.pending_blob.clone()
            };
            let Some((name, bytes)) = next else {
                o.preparing = false;
                o.started = h.now_ms();
                return Ok(true);
            };
            if bytes.is_empty() {
                o.images.push(None);
                continue;
            }
            self.image_job = Some(h.image_metadata(self.client)?.begin_encoded(&name, bytes)?);
            return Ok(false);
        }
    }
    fn load_video_image(&mut self, h: &mut dyn HostServices, id: &str, path: &str) -> Result<bool> {
        if !self.objects[id].preparing {
            let mut old = self.objects.remove(id).unwrap();
            let mut generator = old.generator.take();
            if let Some(group) = &mut generator {
                group.retire_registration();
            }
            let mut message = old.message.take();
            if let Some(message) = &mut message {
                message.created = false;
                message.bound_parent = None;
            }
            self.free_visual(h, &old)?;
            let media = (0..1024)
                .find(|n| !self.media_ids.contains(n))
                .context("QLIE media IDs exhausted")?;
            let image = h.media_request(
                self.client,
                MediaRequest::Play {
                    id: media,
                    asset: path.to_owned(),
                    looping: true,
                    audio: false,
                    gain: 1.,
                },
            )?;
            self.media_ids.insert(media);
            h.media_request(
                self.client,
                MediaRequest::SetClock {
                    id: media,
                    now_ms: 0,
                },
            )?;
            let mut properties = old.properties;
            properties.insert(
                "media_image".into(),
                vec![Value::Text(image.relative().into())],
            );
            self.objects.insert(
                id.into(),
                Object {
                    kind: old.kind,
                    layer: old.layer,
                    position: old.position,
                    properties,
                    message,
                    generator,
                    asset: Some(path.into()),
                    media: Some(media),
                    preparing: true,
                    gain: old.gain,
                    ..Default::default()
                },
            );
        }
        let o = &self.objects[id];
        ensure!(
            o.asset.as_deref() == Some(path),
            "QLIE video-image continuation owner"
        );
        let info = h.media_info(
            self.client,
            o.media.context("QLIE video image media handle")?,
        )?;
        ensure!(
            info.phase != MediaPhase::Failed,
            "QLIE video-image decoder failed: {path}"
        );
        if info.size.is_none() || info.published_frames == 0 {
            return Ok(false);
        }
        let o = self.objects.get_mut(id).unwrap();
        o.preparing = false;
        o.started = h.now_ms();
        Ok(true)
    }
    pub(super) fn free_visual(&mut self, h: &mut dyn HostServices, o: &Object) -> Result<()> {
        if o.created {
            self.submit(h, json!({"layers":[{"op":"remove","id":o.layer}]}))?;
            if o.ui_registered {
                self.submit(h, json!({"ui":[{"op":"remove","id":o.layer}]}))?;
            }
        }
        if let Some(id) = o.media {
            // Standalone movie images own a decoder just like ABMP members.
            // A replacement retains frozen Canvas frame leases, not this live
            // decoder/queue. Retiring only embedded_media leaks each old movie.
            h.media_request(self.client, MediaRequest::Remove { id })?;
            self.media_ids.remove(&id);
        }
        for media in o.embedded_media.values() {
            h.media_request(self.client, MediaRequest::Remove { id: media.id })?;
            self.media_ids.remove(&media.id);
        }
        for ticket in o.audio_tickets.keys() {
            h.release_audio(self.client, *ticket)?;
        }
        for channel in &o.audio_channels {
            h.submit_audio(
                self.client,
                crysirror_core::service::AudioRequest::Stop {
                    channel: *channel,
                    fade_ms: 0.,
                },
            )?;
        }
        let mut released = BTreeSet::new();
        for image in o.images.iter().flatten().chain(o.originals.values()) {
            if !released.insert(image.path.relative()) {
                continue;
            }
            h.release_image_resource(self.client, image.path.relative())?;
        }
        Ok(())
    }
    pub(super) fn replace_image(
        &mut self,
        h: &mut dyn HostServices,
        id: &str,
        index: usize,
        path: &str,
    ) -> Result<bool> {
        ensure!(index < 256, "QLIE image replacement index");
        let index = if let Some(bundle) = &self.objects[id].bundle {
            bundle
                .images
                .get(index)
                .context("QLIE native image slot")?
                .slot
        } else {
            index
        };
        if path.is_empty() {
            let o = self.objects.get_mut(id).unwrap();
            let mut retired = BTreeMap::new();
            if let Some(image) = o.images.get_mut(index).and_then(Option::take) {
                retired.insert(image.path.relative().to_owned(), image);
            }
            if let Some(image) = o.originals.remove(&index) {
                retired.insert(image.path.relative().to_owned(), image);
            }
            o.properties
                .insert(format!("empty_slot/{index}"), vec![Value::Int(1)]);
            for image in retired.into_values() {
                h.release_image_resource(self.client, image.path.relative())?;
            }
            return Ok(true);
        }
        if self.image_job.is_none() {
            if self.load.is_none() {
                let storage = h.storage(self.client)?;
                let client = self.client;
                let source = path.to_owned();
                self.load = Some((
                    path.into(),
                    self.pool
                        .spawn(move |cancel| resources::load(storage, client, &source, &cancel))?,
                ));
            }
            let (expected, job) = self.load.as_mut().unwrap();
            ensure!(expected == path, "QLIE replacement resource ownership");
            let Some(loaded) = job.try_take()? else {
                return Ok(false);
            };
            self.load = None;
            let resources::Loaded::Blob(bytes) = loaded else {
                bail!("QLIE nested animation replacement is not implemented")
            };
            self.image_job = Some(h.image_metadata(self.client)?.begin_encoded(path, bytes)?);
        }
        match self.image_job.as_mut().unwrap().poll()? {
            ImageProcessState::Pending => Ok(false),
            ImageProcessState::Failed(e) => bail!("QLIE replacement image: {e}"),
            ImageProcessState::Ready(image) => {
                self.next_image += 1;
                let published = h.publish_image_resource(self.client, self.next_image, image)?;
                let o = self.objects.get_mut(id).unwrap();
                o.properties.remove(&format!("empty_slot/{index}"));
                if let Some(original) = o.originals.remove(&index) {
                    if o.images
                        .get(index)
                        .and_then(|v| v.as_ref())
                        .is_none_or(|v| v.path != original.path)
                    {
                        h.release_image_resource(self.client, original.path.relative())?;
                    }
                }
                if o.images.len() <= index {
                    o.images.resize(index + 1, None);
                }
                if let Some(old) = o.images[index].replace(published) {
                    h.release_image_resource(self.client, old.path.relative())?;
                }
                self.image_job = None;
                Ok(true)
            }
        }
    }
    pub(super) fn put_parent(&mut self, h: &mut dyn HostServices, id: &str) -> Result<()> {
        let o = &self.objects[id];
        let parent = if o.kind == "eff" {
            1
        } else {
            self.camera_parent(o.position[2])
        };
        let layers = if o.created {
            json!([{"op":"set_parent","id":o.layer,"parent":parent},{"op":"set_visible","id":o.layer,"visible":o.visible},
                {"op":"set_order","id":o.layer,"order":[o.position[2],0]}])
        } else {
            json!([{"op":"put","layer":{"id":o.layer,"parent":parent,"rect":[o.position[0],o.position[1],self.config.canvas[0],self.config.canvas[1]],"order":[o.position[2],0],"visible":o.visible,"paint":{"kind":"solid","color":[0,0,0,0]}}}])
        };
        let ui = if o.ui_registered {
            json!([
                {"op":"set_visible","id":o.layer,"visible":o.visible},
                {"op":"set_enabled","id":o.layer,"enabled":!o.locked}
            ])
        } else {
            json!([])
        };
        let batch = json!({"layers":layers,"ui":ui});
        self.submit(h, batch)?;
        self.objects.get_mut(id).unwrap().created = true;
        Ok(())
    }
    pub(super) fn position(
        &mut self,
        h: &mut dyn HostServices,
        id: &str,
        duration: u64,
    ) -> Result<()> {
        self.position_repeated(h, id, duration, 0)
    }
    pub(super) fn position_repeated(
        &mut self,
        h: &mut dyn HostServices,
        id: &str,
        duration: u64,
        repeat: i32,
    ) -> Result<()> {
        if !self.objects[id].created {
            return self.put_parent(h, id);
        }
        self.put_parent(h, id)?;
        let o = &self.objects[id];
        let layer = o.layer;
        let [x, y, z] = o.position;
        if duration == 0 {
            self.submit(h,json!({"layers":[{"op":"set_rect","id":layer,"rect":[x,y,self.config.canvas[0],self.config.canvas[1]]},{"op":"set_order","id":layer,"order":[z,0]}]}))?;
        } else {
            let mode = o
                .properties
                .get("mm")
                .and_then(|a| a.first())
                .map(Value::integer)
                .transpose()?
                .unwrap_or(3);
            let easing = animation::easing(mode)?;
            self.submit(h,json!({"layers":[{"op":"begin_motion","id":layer,"track":{"property":"x","to":x,"duration":duration,"repeat":repeat,"easing":easing}},{"op":"begin_motion","id":layer,"track":{"property":"y","to":y,"duration":duration,"repeat":repeat,"easing":easing}}]}))?;
        }
        Ok(())
    }
    pub(super) fn effect(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        a: &[Value],
    ) -> Result<bool> {
        const CAPTURE: u32 = 0x10000000;
        let [w, height] = self.config.canvas;
        match op {
            "?c" => {
                ensure!(a.is_empty(), "QLIE capture-state query ABI");
                // Native 6DCBE8 ('?c'): ResultInt[0] is (+77 == 1),
                // ResultInt[1] is the retained capture/fade priority (+97).
                // A running fade is not an awaiting-capture state.
                m.result(0, Value::Int(i32::from(self.capture_waiting)));
                m.result(1, Value::Int(self.effect_priority));
            }
            "dc" | "sc" | "v" | "fdm" | "fdp" | "pri" => {
                // ImoGraphic keeps sound routing and the fade transform until
                // the next transition; setting these does not draw or play.
                self.objects
                    .get_mut("eff")
                    .context("QLIE effect state")?
                    .properties
                    .insert(op.into(), a.to_vec());
            }
            "bg" => {
                let c = a.first().context("QLIE background colour")?.integer()? as u32;
                self.submit(h,json!({"layers":[{"op":"set_solid_color","id":1,"color":[(c>>16)&255,(c>>8)&255,c&255,255]}]}))?;
            }
            "c" => {
                ensure!(a.len() <= 1, "QLIE capture ABI");
                // BufferCapture (6B9DCC) retains the first pending capture.
                // Recapturing here loses the old scene before a deferred fade.
                if self.capture_waiting {
                    return Ok(true);
                }
                self.cancel_animated_transition(h)?;
                self.rule_fade = None;
                let create = if !self.capture {
                    vec![json!({"op":"create","id":CAPTURE,"width":w,"height":height})]
                } else {
                    vec![]
                };
                let mut commands = create;
                commands.push(json!({"op":"clear","id":CAPTURE,"color":[0,0,0,255]}));
                commands.push(json!({"op":"copy_subtree","source":1,"destination":CAPTURE,"rect":[0,0,w,height],"position":[0,0]}));
                let priority = a.first().map(Value::integer).transpose()?.unwrap_or(60000);
                if !self.capture_scene(h, commands, priority)? {
                    return Ok(false);
                }
                // Native Draw6B9010 case1 presents the frozen old scene while
                // script/resource work prepares its replacement. Materialize
                // the public capture layer now, so an immediate eff\i and the
                // first rule-image fade have a real owned presentation too.
                self.submit(h,json!({"canvases":[{"op":"present_target","id":CAPTURE,"layer":CAPTURE,"order":priority}],"layers":[{"op":"end_motion","id":CAPTURE},{"op":"set_visible","id":CAPTURE,"visible":true},{"op":"set_filter","id":CAPTURE,"filter":null},{"op":"set_opacity","id":CAPTURE,"opacity":255}]}))?;
                self.capture = true;
                self.capture_waiting = true;
                self.effect_priority = priority;
            }
            "fdi" => {
                ensure!(self.capture, "QLIE fade requires captured scene");
                ensure!(a.len() >= 2, "QLIE fade ABI");
                ensure!(
                    matches!(a[0].integer()?, 0 | 1 | 3),
                    "QLIE transition mode {} requires its native effect mapping",
                    a[0].text()
                );
                let filename = a.get(2).map(Value::text).unwrap_or_default();
                let duration = a[1].integer()?.max(0) as u64;
                let priority = a
                    .get(3)
                    .map(Value::integer)
                    .transpose()?
                    .unwrap_or(i32::MAX);
                let mode = self
                    .objects
                    .get("eff")
                    .and_then(|o| o.properties.get("fdm"))
                    .and_then(|v| v.first())
                    .map(Value::integer)
                    .transpose()?
                    .unwrap_or(3);
                let easing = animation::easing(mode)?;
                if !filename.is_empty() {
                    let path = self.resolve(m, &filename)?;
                    if path.to_ascii_lowercase().ends_with(".b") {
                        let second = a.get(4).map(Value::integer).transpose()?.unwrap_or(-1) == 1;
                        return self
                            .animated_transition(h, &path, duration, priority, easing, second);
                    }
                }
                self.cancel_animated_transition(h)?;
                if filename.is_empty() {
                    self.rule_fade = None;
                    self.submit(h,json!({"canvases":[{"op":"present_target","id":CAPTURE,"layer":CAPTURE,"order":priority}],"layers":[{"op":"set_visible","id":CAPTURE,"visible":true},{"op":"set_filter","id":CAPTURE,"filter":null},{"op":"set_opacity","id":CAPTURE,"opacity":255},{"op":"begin_motion","id":CAPTURE,"track":{"property":"opacity","from":255,"to":0,"duration":duration,"easing":easing}}]}))?;
                } else {
                    let path = self.resolve(m, &filename)?;
                    if let Some(pose) = self.objects["eff"].properties.get("fdp") {
                        ensure!(
                            pose.iter()
                                .map(Value::integer)
                                .collect::<Result<Vec<_>>>()?
                                == [0, 0, 100, 100, 0],
                            "QLIE transformed transition rule requires shared image preparation"
                        );
                    }
                    if self.objects["eff"].asset.as_deref() != Some(&path)
                        || self.objects["eff"].preparing
                    {
                        // Preparation is worker-owned; the old transition is
                        // cancelled before its immutable rule lease is retired.
                        self.rule_fade = None;
                        if !self.load_image(h, "eff", &path)? {
                            return Ok(false);
                        }
                    }
                    let rule = self.objects["eff"]
                        .images
                        .first()
                        .and_then(|i| i.as_ref())
                        .context("QLIE transition rule image")?
                        .path
                        .relative()
                        .to_owned();
                    let power = a
                        .get(4)
                        .map(Value::integer)
                        .transpose()?
                        .unwrap_or(-1)
                        .clamp(0, 255) as f32;
                    const CLOCK: u32 = CAPTURE + 1;
                    let mut commands = vec![json!({"op":"end_motion","id":CAPTURE})];
                    if !self.effect_clock {
                        commands.push(json!({"op":"put","layer":{"id":CLOCK,"rect":[0,0,1,1],"visible":false,"paint":{"kind":"solid","color":[0,0,0,0]}}}));
                    } else {
                        commands.push(json!({"op":"end_motion","id":CLOCK}));
                    }
                    commands.push(json!({"op":"begin_motion","id":CLOCK,"track":{"property":"x","from":255,"to":0,"duration":duration,"easing":easing}}));
                    commands.push(json!({"op":"set_visible","id":CAPTURE,"visible":true}));
                    commands.push(json!({"op":"set_opacity","id":CAPTURE,"opacity":255}));
                    commands.push(json!({"op":"set_filter","id":CAPTURE,"filter":{"mask":{"asset":rule,"channel":"red","offset":255.*(power+1.),"scale":-power,"preserve_source_alpha":true}}}));
                    self.submit(h,json!({"canvases":[{"op":"present_target","id":CAPTURE,"layer":CAPTURE,"order":priority}],"layers":commands}))?;
                    self.effect_clock = true;
                    self.rule_fade = Some((rule, power));
                }
                self.capture_waiting = false;
                self.effect_priority = priority;
            }
            "i" => {
                self.cancel_animated_transition(h)?;
                self.rule_fade = None;
                self.capture_waiting = false;
                self.effect_priority = 65535;
                if self.capture {
                    self.submit(
                        h,
                        json!({"canvases":[{"op":"clear","id":CAPTURE,"color":[0,0,0,255]}],"layers":[{"op":"end_motion","id":CAPTURE},{"op":"set_filter","id":CAPTURE,"filter":null},{"op":"set_visible","id":CAPTURE,"visible":false}]}),
                    )?;
                }
            }
            _ => bail!("unsupported QLIE effect method \\{op}"),
        }
        Ok(true)
    }
    pub(super) fn update_visuals(&mut self, h: &mut dyn HostServices) -> Result<()> {
        self.update_generators(h)?;
        if let Some(end) = self.animated_fade {
            let clock: serde_json::Value = serde_json::from_slice(
                &h.query_engine(self.client, EngineQuery::Layer(0x10000001))?,
            )?;
            let time = clock["rect"][0]
                .as_f64()
                .context("QLIE animated transition clock")?
                .max(0.) as u64;
            let o = self.objects.get_mut("eff").unwrap();
            o.offset = time;
            o.started = h.now_ms();
            if time >= end {
                self.cancel_animated_transition(h)?;
            }
        }
        if let Some((asset, power)) = self.rule_fade.clone() {
            let clock: serde_json::Value = serde_json::from_slice(
                &h.query_engine(self.client, EngineQuery::Layer(0x10000001))?,
            )?;
            let alpha = clock["rect"][0]
                .as_f64()
                .context("QLIE transition clock sample")? as f32;
            if alpha <= 0. {
                self.submit(
                    h,
                    json!({"layers":[{"op":"set_visible","id":0x10000000,"visible":false}]}),
                )?;
                self.rule_fade = None;
            } else {
                self.submit(h,json!({"layers":[{"op":"set_filter","id":0x10000000,"filter":{"mask":{"asset":asset,"channel":"red","offset":alpha*(power+1.),"scale":-power,"preserve_source_alpha":true}}}]}))?;
            }
        }
        super::render::update(self, h)
    }
}

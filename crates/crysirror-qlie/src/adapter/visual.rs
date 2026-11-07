use super::*;
use crysirror_core::{
    image_processing::ImageProcessState,
    service::{EngineQuery, MediaPhase, MediaRequest},
};
impl QlieVm {
    pub(super) fn property_easing(
        &self,
        id: &str,
        key: &str,
    ) -> Result<crysirror_scene::layer::Easing> {
        let values = self.objects[id].properties.get(key);
        let mut mode = values
            .and_then(|v| v.first())
            .map(Value::integer)
            .transpose()?
            .unwrap_or(
                if matches!(self.objects[id].kind.as_str(), "cam" | "camex") {
                    1
                } else {
                    3
                },
            );
        let strength = values
            .and_then(|v| v.get(1))
            .map(Value::integer)
            .transpose()?
            .unwrap_or(100)
            .clamp(0, 200);
        if key == "fdm" {
            //6B89A0 stores a byte selector. Unsupported native fade modes,
            //including noisy6, normalize to Linear rather than failing.
            mode = i32::from(mode as u8);
            if !matches!(mode,1..=4|7..=8|12..=20) {
                mode = 1;
            }
        } else if matches!(self.objects[id].kind.as_str(), "cam" | "camex") {
            //6A7884 maps public script modes to camera6A6AD0's private
            //selector space. Modes with no mapping (including6) are Linear.
            //The remaining public names call the same shared curve kernels.
            if !matches!(mode, 0 | 1 | 3 | 4 | 7 | 8 | 12..=20) {
                mode = 1;
            }
        }
        animation::easing_with_strength(mode, strength)
    }
    /// Native synthetic GameThreadImage resources freeze the original game
    /// graphic into an engine Canvas; the presenter owns rasterization and
    /// asynchronous readback, just like any reusable engine image operation.
    pub(super) fn load_game_thread_image(
        &mut self,
        h: &mut dyn HostServices,
        id: &str,
        name: &str,
    ) -> Result<bool> {
        let native = name.to_ascii_lowercase();
        let (kind, priority) = native
            .rsplit_once(',')
            .context("QLIE game-thread capture priority")?;
        ensure!(
            matches!(kind, "$$$gamethreadimagez" | "$$$gamethreadimage"),
            "unsupported QLIE game-thread capture {name}"
        );
        let priority = priority
            .strip_suffix(".bmp")
            .context("QLIE game-thread capture extension")?;
        let priority = crate::expression::parse_integer(priority)
            .context("QLIE game-thread capture priority")?;
        if self.thread_image.is_none() {
            let mut object = self
                .objects
                .remove(id)
                .context("QLIE game-thread image target absent")?;
            let mut generator = object.generator.take();
            if let Some(group) = &mut generator {
                group.retire_registration();
            }
            let mut message = object.message.take();
            if let Some(message) = &mut message {
                message.created = false;
                message.bound_parent = None;
            }
            self.free_visual(h, &object)?;
            object = Object {
                thread: object.thread,
                kind: object.kind,
                layer: object.layer,
                position: object.position,
                gain: object.gain,
                properties: object.properties,
                generator,
                message,
                ..Default::default()
            };
            object.asset = Some(name.into());
            object.preparing = true;
            let canvas = 0x12000000u32
                .checked_add(object.layer)
                .context("QLIE snapshot canvas ID")?;
            self.objects.insert(id.into(), object);
            let mut commands = vec![
                json!({"op":"create","id":canvas,"width":self.config.canvas[0],"height":self.config.canvas[1]}),
                json!({"op":"clear","id":canvas,"color":[0,0,0,255]}),
            ];
            // The pause root itself may be hidden. CopySubtree intentionally
            // excludes source-root visibility/placement while retaining all
            // child transforms, image leases and compositor state.
            commands.push(json!({"op":"copy_subtree","source":self.root_layer_for(ThreadKind::Game),"destination":canvas,"rect":[0,0,self.config.canvas[0],self.config.canvas[1]],"position":[0,0]}));
            let old_thread = self.host_thread;
            self.host_thread = ThreadKind::Game;
            let ready = self.capture_scene(h, commands, priority);
            self.host_thread = old_thread;
            if !ready? {
                return Ok(false);
            }
            self.thread_image = Some((
                id.into(),
                canvas,
                h.begin_canvas_image(self.client, canvas)?,
            ));
        }
        let (owner, canvas, job) = self.thread_image.as_mut().unwrap();
        ensure!(owner == id, "QLIE game-thread image continuation identity");
        let canvas = *canvas;
        match job.poll()? {
            ImageProcessState::Pending => Ok(false),
            ImageProcessState::Failed(error) => bail!("QLIE game-thread image capture: {error}"),
            ImageProcessState::Ready(image) => {
                self.next_image = self
                    .next_image
                    .checked_add(1)
                    .context("QLIE image ID exhausted")?;
                let image = h.publish_image_resource(self.client, self.next_image, image)?;
                let object = self.objects.get_mut(id).unwrap();
                object.images.push(Some(image));
                object.preparing = false;
                object.started = h.now_ms();
                self.thread_image = None;
                self.submit(h, json!({"canvases":[{"op":"destroy","id":canvas}]}))?;
                Ok(true)
            }
        }
    }
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
            let thread = old.thread;
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
                    thread,
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
                        if member.nested.is_some() || member.model.is_some() || member.metadata["kind"] == 7 {
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
                    thread: old.thread,
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
        super::render::release_draw_images(h, self.client, o)?;
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
        let requested = index;
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
            o.properties.remove(&format!("save/replace/{requested}"));
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
                let lower = path.to_ascii_lowercase();
                let task = if let Some(rest) = lower.strip_prefix("$$$saveimage,") {
                    // The save slot's JPEG thumbnail (6B6F78); absent -> empty.
                    let slot = rest
                        .strip_suffix(".bmp")
                        .context("QLIE save thumbnail resource name")?;
                    let slot = crate::program::canonical(&path[13..13 + slot.len()])?;
                    let key = self.save_key();
                    self.pool.spawn(move |cancel| {
                        let bytes = save_metadata::read_thumbnail(storage, &slot, key, &cancel)?
                            .unwrap_or_default();
                        Ok(resources::Loaded::Blob(bytes.into()))
                    })?
                } else {
                    ensure!(!path.starts_with("$$$"), "unsupported QLIE virtual image {path}");
                    self.pool
                        .spawn(move |cancel| resources::load(storage, client, &source, &cancel))?
                };
                self.load = Some((path.into(), task));
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
            if bytes.is_empty() {
                return self.replace_image(h, id, requested, "");
            }
            let movie = [".ogv", ".ogm", ".avi", ".mp4", ".webm", ".mpg", ".mpeg", ".wmv"]
                .iter()
                .any(|ext| path.to_ascii_lowercase().ends_with(ext));
            if movie {
                // A movie in an ABMP slot is a kind-7 member: the shared media
                // decoder follows the slot's animation seek clock (render).
                let o = self.objects.get_mut(id).unwrap();
                let bundle = o
                    .bundle
                    .as_mut()
                    .context("QLIE movie replacement requires an animation slot")?;
                let member = bundle.slot_mut(index).context("QLIE movie replacement slot")?;
                member.metadata["kind"] = json!(7);
                member.bytes = bytes;
                if let Some(media) = o.embedded_media.remove(&index) {
                    h.media_request(self.client, MediaRequest::Remove { id: media.id })?;
                    self.media_ids.remove(&media.id);
                }
                if let Some(old) = o.images.get_mut(index).and_then(Option::take) {
                    h.release_image_resource(self.client, old.path.relative())?;
                }
                o.properties.remove(&format!("empty_slot/{index}"));
                o.properties.insert(
                    format!("save/replace/{requested}"),
                    vec![Value::Text(path.to_owned())],
                );
                return Ok(true);
            }
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
                o.properties.insert(
                    format!("save/replace/{requested}"),
                    vec![Value::Text(path.to_owned())],
                );
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
        let anchor = self.objects[id]
            .properties
            .get("anc")
            .and_then(|v| v.first())
            .map(Value::text)
            .filter(|a| a != id && self.objects.contains_key(a));
        let anchored = match anchor {
            Some(anchor) => Some(self.anchor_proxy(h, &anchor)?),
            None => None,
        };
        let o = &self.objects[id];
        let parent = if let Some(proxy) = anchored {
            proxy
        } else if o.kind == "eff" {
            self.root_layer_for(o.thread)
        } else {
            self.camera_parent(o.thread, o.position[2])
        };
        let layers = if o.created {
            json!([{"op":"set_parent","id":o.layer,"parent":parent},{"op":"set_visible","id":o.layer,"visible":o.visible},
                {"op":"set_order","id":o.layer,"order":[o.position[2],0]},
                {"op":"set_paint","id":o.layer,"paint":{"kind":"group"}},
                {"op":"set_depth","id":o.layer,"depth":null}])
        } else {
            // Native leaf priorities add object/key values before global
            // queue sorting. This root owns geometry, never a painter fence.
            json!([{"op":"put","layer":{"id":o.layer,"parent":parent,"rect":[o.position[0],o.position[1],self.config.canvas[0],self.config.canvas[1]],"order":[o.position[2],0],"visible":o.visible,"paint":{"kind":"group"}}},{"op":"set_depth","id":o.layer,"depth":null}])
        };
        let ui = if o.ui_registered {
            json!([
                {"op":"set_visible","id":o.layer,"visible":o.visible},
                {"op":"set_enabled","id":o.layer,"enabled":o.thread==self.input_thread()&&!o.locked&&self.input_mask_allows(o)}
            ])
        } else {
            json!([])
        };
        let batch = json!({"layers":layers,"ui":ui});
        self.submit(h, batch)?;
        self.objects.get_mut(id).unwrap().created = true;
        Ok(())
    }
    /// 5D1644 multiplies an anchored object's matrix by its anchor's. A
    /// group proxy beside the anchor mirrors only that geometry, so the
    /// anchor's own visibility/opacity/paint are not inherited.
    pub(super) fn anchor_proxy(&mut self, h: &mut dyn HostServices, anchor: &str) -> Result<u32> {
        if let Some(proxy) = self.anchor_proxies.get(anchor) {
            return Ok(*proxy);
        }
        if !self.objects[anchor].created {
            self.put_parent(h, anchor)?;
        }
        let a = &self.objects[anchor];
        let proxy = 0x30000000u32
            .checked_add(a.layer)
            .context("QLIE anchor proxy identity")?;
        let parent = self.camera_parent(a.thread, a.position[2]);
        let [w, height] = self.config.canvas;
        self.submit(h, json!({"layers":[{"op":"put","layer":{"id":proxy,"parent":parent,"rect":[a.position[0],a.position[1],w,height],"order":[a.position[2],0],"paint":{"kind":"group"}}},{"op":"set_depth","id":proxy,"depth":null}]}))?;
        self.anchor_proxies.insert(anchor.into(), proxy);
        Ok(proxy)
    }
    /// Copy each anchor's sampled rect/transform to its proxy this frame.
    pub(super) fn update_anchor_proxies(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let mut commands = Vec::new();
        for (anchor, proxy) in &self.anchor_proxies {
            // A retired anchor's proxy keeps its last sampled geometry.
            let Some(a) = self.objects.get(anchor).filter(|a| a.created) else {
                continue;
            };
            let Ok(bytes) = h.query_engine(self.client, EngineQuery::Layer(a.layer)) else {
                continue;
            };
            let spec: serde_json::Value = serde_json::from_slice(&bytes)?;
            if spec.is_null() {
                continue;
            }
            commands.push(json!({"op":"set_rect","id":proxy,"rect":spec["rect"]}));
            if !spec["transform"].is_null() {
                commands.push(json!({"op":"set_transform","id":proxy,"transform":spec["transform"]}));
            }
        }
        if !commands.is_empty() {
            self.submit(h, json!({"layers":commands}))?;
        }
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
            self.cancel_native_position(h, id)?;
            self.submit(h,json!({"layers":[{"op":"set_rect","id":layer,"rect":[x,y,self.config.canvas[0],self.config.canvas[1]]},{"op":"set_order","id":layer,"order":[z,0]}]}))?;
        } else {
            let mode = self.objects[id]
                .properties
                .get("mm")
                .and_then(|v| v.first())
                .map(Value::integer)
                .transpose()?
                .unwrap_or(3);
            if mode == 6 {
                return self.begin_jitter_position(h, id, duration, repeat);
            }
            self.cancel_native_position(h, id)?;
            let easing = self.property_easing(id, "mm")?;
            self.submit(h,json!({"layers":[{"op":"begin_motion","id":layer,"track":{"property":"x","to":x,"duration":duration,"repeat":repeat,"easing":easing}},{"op":"begin_motion","id":layer,"track":{"property":"y","to":y,"duration":duration,"repeat":repeat,"easing":easing}}]}))?;
        }
        Ok(())
    }
    /// eff\\i: end every screen effect and hide the captured buffer.
    pub(super) fn effect_reset(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let capture = self.capture_id(0);
        self.end_screen_effects(h)?;
        self.cancel_animated_transition(h)?;
        self.rule_fade = None;
        self.capture_waiting = false;
        self.effect_priority = 65535;
        if self.capture {
            self.submit(
                h,
                json!({"canvases":[{"op":"clear","id":capture,"color":[0,0,0,255]}],"layers":[{"op":"end_motion","id":capture},{"op":"set_filter","id":capture,"filter":null},{"op":"set_visible","id":capture,"visible":false}]}),
            )?;
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
        let target = self
            .objects
            .get(&m.scope)
            .context("QLIE effect target absent")?
            .thread;
        if target != self.host_thread {
            ensure!(
                target == ThreadKind::Game && self.host_thread == ThreadKind::System,
                "QLIE effect source-thread context"
            );
            let mut game = self
                .suspended_host
                .take()
                .context("QLIE paused game graphic absent")?;
            game.exchange_graphic(self);
            self.host_thread = ThreadKind::Game;
            let result = self.effect_current(h, m, op, a);
            self.host_thread = ThreadKind::System;
            game.exchange_graphic(self);
            self.suspended_host = Some(game);
            return result;
        }
        self.effect_current(h, m, op, a)
    }
    fn effect_current(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        a: &[Value],
    ) -> Result<bool> {
        let capture = self.capture_id(0);
        let eff = Self::builtin("eff", self.host_thread);
        let root = self.root_layer_for(self.host_thread);
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
            "fdm" => {
                ensure!((1..=2).contains(&a.len()), "QLIE fade mode ABI");
                let mut mode = i32::from(a[0].integer()? as u8);
                if !matches!(mode,1..=4|7..=8|12..=20) {
                    mode = 1;
                }
                let strength = a
                    .get(1)
                    .map(Value::integer)
                    .transpose()?
                    .unwrap_or(100)
                    .clamp(0, 200);
                self.objects
                    .get_mut(&eff)
                    .context("QLIE effect state")?
                    .properties
                    .insert(op.into(), vec![Value::Int(mode), Value::Int(strength)]);
            }
            // mb (6BA0F4 +4C) makes 6B9010 request a full presenter redraw
            // each frame (568D70); vtn (+71) selects the effect sounds'
            // volume track. Both are kept; the engine always redraws.
            "dc" | "sc" | "v" | "fdp" | "pri" | "mb" | "vtn" => {
                // ImoGraphic keeps sound routing and the fade transform until
                // the next transition; setting these does not draw or play.
                self.objects
                    .get_mut(&eff)
                    .context("QLIE effect state")?
                    .properties
                    .insert(op.into(), a.to_vec());
            }
            // Queries of the stored effect state (6DCBE8): ?v +6D, ?dc +60,
            // ?sc, ?vtn +71, ?mb (1 when set, else 0), ?fdm mode/strength,
            // ?bg the background colour.
            "?v" | "?dc" | "?sc" | "?vtn" | "?mb" | "?bg" => {
                let key = &op[1..];
                let value = self
                    .objects
                    .get(&eff)
                    .and_then(|o| o.properties.get(key))
                    .and_then(|v| v.first())
                    .cloned();
                let value = match op {
                    "?mb" => Value::Int(i32::from(value.map(|v| v.integer()).transpose()?.unwrap_or(0) == 1)),
                    _ => value.unwrap_or(Value::Int(0)),
                };
                m.result(0, value);
            }
            "?fdm" => {
                let values = self.objects.get(&eff).and_then(|o| o.properties.get("fdm")).cloned().unwrap_or_default();
                m.result(0, values.first().cloned().unwrap_or(Value::Int(1)));
                m.result(1, values.get(1).cloned().unwrap_or(Value::Int(100)));
            }
            "bg" => {
                let c = a.first().context("QLIE background colour")?.integer()? as u32;
                if let Some(o) = self.objects.get_mut(&eff) {
                    o.properties.insert("bg".into(), vec![Value::Int(c as i32)]);
                }
                self.submit(h,json!({"layers":[{"op":"set_solid_color","id":root,"color":[(c>>16)&255,(c>>8)&255,c&255,255]}]}))?;
            }
            "c" => {
                ensure!(a.len() <= 1, "QLIE capture ABI");
                // BufferCapture (6B9DCC) retains the first pending capture.
                // Recapturing here loses the old scene before a deferred fade.
                if self.capture_waiting {
                    return Ok(true);
                }
                self.end_screen_effects(h)?;
                self.cancel_animated_transition(h)?;
                self.rule_fade = None;
                let create = if !self.capture {
                    vec![json!({"op":"create","id":capture,"width":w,"height":height})]
                } else {
                    vec![]
                };
                let mut commands = create;
                commands.push(json!({"op":"clear","id":capture,"color":[0,0,0,255]}));
                commands.push(json!({"op":"copy_subtree","source":root,"destination":capture,"rect":[0,0,w,height],"position":[0,0]}));
                let priority = a.first().map(Value::integer).transpose()?.unwrap_or(60000);
                if !self.capture_scene(h, commands, priority)? {
                    return Ok(false);
                }
                // Native Draw6B9010 case1 presents the frozen old scene while
                // script/resource work prepares its replacement. Materialize
                // the public capture layer now, so an immediate eff\i and the
                // first rule-image fade have a real owned presentation too.
                self.submit(h,json!({"canvases":[{"op":"present_target","id":capture,"layer":capture,"parent":root,"order":priority}],"layers":[{"op":"end_motion","id":capture},{"op":"set_visible","id":capture,"visible":true},{"op":"set_filter","id":capture,"filter":null},{"op":"set_opacity","id":capture,"opacity":255}]}))?;
                self.capture = true;
                self.capture_waiting = true;
                self.effect_priority = priority;
            }
            "fdi" | "fdo" if a.first().map(Value::integer).transpose()? == Some(2) => {
                // 6DCBE8 mode 2: two 6B9AD8 blends. The first installs the
                // ARGB colour at once, the second fades it to alpha 0 (its
                // rgb kept) over the duration; +94 selects the blend mode.
                ensure!(a.len() >= 3, "QLIE colour transition ABI");
                let duration = a[1].integer()?.max(0) as u64;
                let colour = a[2].integer()? as u32;
                let mode = a.get(3).map(Value::integer).transpose()?.unwrap_or(0);
                ensure!((0..=6).contains(&mode), "QLIE colour transition blend {mode}");
                let priority = a.get(4).map(Value::integer).transpose()?.unwrap_or(i32::MAX);
                let easing = self.property_easing(&eff, "fdm")?;
                let flash = capture + 4;
                let rgb = [(colour >> 16) & 255, (colour >> 8) & 255, colour & 255, 255];
                let alpha = colour >> 24;
                // Case 8 fills with raster 3+mode. 55E8EC maps object
                // selectors 1->3, 0/2->4, 4->5, 5->7, 6->9; 538138 gives each
                // even raster its odd predecessor's operation.
                let selector = [1u8, 0, 4, 4, 5, 5, 6][mode as usize];
                let blend = super::animation::blend(selector);
                let mut commands = Vec::new();
                if std::mem::replace(&mut self.colour_flash, true) {
                    commands.push(json!({"op":"remove","id":flash}));
                }
                commands.push(json!({"op":"put","layer":{"id":flash,"parent":root,"rect":[0,0,w,height],"order":[priority,2],"opacity":alpha,"blend":blend,"paint":{"kind":"solid","color":rgb}}}));
                // fdo installs the rgb at alpha 0 and fades it in instead.
                let (from, to) = if op == "fdo" { (0, alpha) } else { (alpha, 0) };
                commands.push(json!({"op":"begin_motion","id":flash,"track":{"property":"opacity","from":from,"to":to,"duration":duration,"easing":easing}}));
                self.submit(h, json!({"layers":commands}))?;
                self.effect_priority = priority;
            }
            "fdi" if a.first().map(Value::integer).transpose()? == Some(4) => {
                // 6B8C98 / 6B9010 case 10: the capture stays opaque until
                // progress 111, fades out by 143, while the screen below the
                // effect priority is pixelated in cells of 2*min(p, 255-p).
                ensure!(self.capture, "QLIE fade requires captured scene");
                ensure!(a.len() >= 2, "QLIE mosaic transition ABI");
                let duration = a[1].integer()?.max(0) as u64;
                let priority = a.get(3).map(Value::integer).transpose()?.unwrap_or(i32::MAX);
                let easing = self.property_easing(&eff, "fdm")?;
                self.cancel_animated_transition(h)?;
                self.rule_fade = None;
                let clock = capture + 2;
                let backdrop = capture + 3;
                self.submit(h, json!({
                    "canvases":[{"op":"present_target","id":capture,"layer":capture,"parent":root,"order":priority}],
                    "layers":[
                        {"op":"end_motion","id":capture},
                        {"op":"set_visible","id":capture,"visible":true},
                        {"op":"set_filter","id":capture,"filter":null},
                        {"op":"set_opacity","id":capture,"opacity":255},
                        {"op":"put","layer":{"id":clock,"parent":root,"rect":[0,0,1,1],"visible":false,"paint":{"kind":"solid","color":[0,0,0,0]}}},
                        {"op":"begin_motion","id":clock,"track":{"property":"x","from":0,"to":255,"duration":duration,"easing":easing}},
                        {"op":"put","layer":{"id":backdrop,"parent":root,"rect":[0,0,w,height],"order":[priority,1],"paint":{"kind":"backdrop"}}}
                    ]}))?;
                self.mosaic = Some(priority);
                self.capture_waiting = false;
                self.effect_priority = priority;
            }
            // fdo modes other than 0..3 have no handler; a .b rule is ignored
            // by 6BA524.
            "fdo" if !matches!(a.first().map(Value::integer).transpose()?, Some(0 | 1 | 3))
                || a.get(2).map(Value::text).unwrap_or_default().to_ascii_lowercase().ends_with(".b") => {}
            // fdi/fdo (6B9A70/6B9AA4, 6B9BBC/6B9C70): fade-in reveals the
            // scene under the capture; fade-out (states 3/5) runs the same
            // progress backwards, bringing the capture back over the scene.
            "fdi" | "fdo" => {
                ensure!(self.capture, "QLIE fade requires captured scene");
                ensure!(a.len() >= 2, "QLIE fade ABI");
                let (from, to) = if op == "fdo" { (0, 255) } else { (255, 0) };
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
                let easing = self.property_easing(&eff, "fdm")?;
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
                    self.submit(h,json!({"canvases":[{"op":"present_target","id":capture,"layer":capture,"parent":root,"order":priority}],"layers":[{"op":"set_visible","id":capture,"visible":true},{"op":"set_filter","id":capture,"filter":null},{"op":"set_opacity","id":capture,"opacity":from},{"op":"begin_motion","id":capture,"track":{"property":"opacity","from":from,"to":to,"duration":duration,"easing":easing}}]}))?;
                } else {
                    let path = self.resolve(m, &filename)?;
                    if self.objects[&eff].asset.as_deref() != Some(&path)
                        || self.objects[&eff].preparing
                    {
                        // Preparation is worker-owned; the old transition is
                        // cancelled before its immutable rule lease is retired.
                        self.rule_fade = None;
                        if !self.load_image(h, &eff, &path)? {
                            return Ok(false);
                        }
                    }
                    let Some(rule) = self.prepare_rule(h, &eff, &path)? else {
                        return Ok(false);
                    };
                    let power = a
                        .get(4)
                        .map(Value::integer)
                        .transpose()?
                        .unwrap_or(-1)
                        .clamp(0, 255) as f32;
                    let clock = capture + 1;
                    let mut commands = vec![json!({"op":"end_motion","id":capture})];
                    if !self.effect_clock {
                        commands.push(json!({"op":"put","layer":{"id":clock,"parent":root,"rect":[0,0,1,1],"visible":false,"paint":{"kind":"solid","color":[0,0,0,0]}}}));
                    } else {
                        commands.push(json!({"op":"end_motion","id":clock}));
                    }
                    commands.push(json!({"op":"begin_motion","id":clock,"track":{"property":"x","from":from,"to":to,"duration":duration,"easing":easing}}));
                    commands.push(json!({"op":"set_visible","id":capture,"visible":true}));
                    commands.push(json!({"op":"set_opacity","id":capture,"opacity":255}));
                    commands.push(json!({"op":"set_filter","id":capture,"filter":{"mask":{"asset":rule,"channel":"red","offset":255.*(power+1.),"scale":-power,"preserve_source_alpha":true}}}));
                    self.submit(h,json!({"canvases":[{"op":"present_target","id":capture,"layer":capture,"parent":root,"order":priority}],"layers":commands}))?;
                    self.effect_clock = true;
                    self.rule_fade = Some((rule, power, op == "fdo"));
                }
                self.capture_waiting = false;
                self.effect_priority = priority;
            }
            "i" => self.effect_reset(h)?,
            _ => bail!("unsupported QLIE effect method \\{op}"),
        }
        Ok(true)
    }
    /// Drop a running mosaic and the colour-blend layer (eff\\c, eff\\i).
    fn end_screen_effects(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let capture = self.capture_id(0);
        let mut commands = Vec::new();
        if std::mem::take(&mut self.colour_flash) {
            commands.push(json!({"op":"remove","id":capture + 4}));
        }
        if self.mosaic.take().is_some() {
            commands.push(json!({"op":"remove","id":capture + 3}));
            commands.push(json!({"op":"remove","id":capture + 2}));
        }
        if commands.is_empty() {
            return Ok(());
        }
        self.submit(h, json!({"layers":commands}))
    }
    pub(super) fn update_visuals(&mut self, h: &mut dyn HostServices) -> Result<()> {
        self.update_native_positions(h)?;
        self.update_anchor_proxies(h)?;
        let eff = Self::builtin("eff", self.host_thread);
        let capture = self.capture_id(0);
        let clock = self.capture_id(1);
        self.update_generators(h)?;
        if let Some(end) = self.animated_fade {
            let clock: serde_json::Value =
                serde_json::from_slice(&h.query_engine(self.client, EngineQuery::Layer(clock))?)?;
            let time = clock["rect"][0]
                .as_f64()
                .context("QLIE animated transition clock")?
                .max(0.) as u64;
            let o = self.objects.get_mut(&eff).unwrap();
            o.offset = time;
            o.started = h.now_ms();
            if time >= end {
                self.cancel_animated_transition(h)?;
            }
        }
        if self.mosaic.is_some() {
            let clock: serde_json::Value = serde_json::from_slice(
                &h.query_engine(self.client, EngineQuery::Layer(capture + 2))?,
            )?;
            let x = clock["rect"][0].as_f64().context("QLIE mosaic clock")?;
            let p = ((x + 0.5) as i32).clamp(0, 255);
            let alpha = (255 * (255 - p - 112) / 32).clamp(0, 255);
            let cell = 2 * p.min(255 - p);
            let [w, height] = self.config.canvas;
            let filter = if cell >= 2 {
                // 568CC0 pixelates square cells anchored at the screen origin.
                let cell = cell as f32;
                json!({"distortion":{"kind":"quantized","steps":[(w as f32 / cell).max(1.),(height as f32 / cell).max(1.)],"phase":[0.,0.]}})
            } else {
                serde_json::Value::Null
            };
            let mut commands = vec![
                json!({"op":"set_opacity","id":capture,"opacity":alpha}),
                json!({"op":"set_filter","id":capture + 3,"filter":filter}),
            ];
            if p >= 255 {
                commands.push(json!({"op":"remove","id":capture + 3}));
                commands.push(json!({"op":"remove","id":capture + 2}));
                commands.push(json!({"op":"set_visible","id":capture,"visible":false}));
                self.mosaic = None;
            }
            self.submit(h, json!({"layers":commands}))?;
        }
        if let Some((asset, power, fade_out)) = self.rule_fade.clone() {
            let clock: serde_json::Value =
                serde_json::from_slice(&h.query_engine(self.client, EngineQuery::Layer(clock))?)?;
            let alpha = clock["rect"][0]
                .as_f64()
                .context("QLIE transition clock sample")? as f32;
            // A fade-out keeps its final (fully covering) mask.
            if alpha <= 0. && !fade_out {
                self.submit(
                    h,
                    json!({"layers":[{"op":"set_visible","id":capture,"visible":false}]}),
                )?;
                self.rule_fade = None;
            } else {
                self.submit(h,json!({"layers":[{"op":"set_filter","id":capture,"filter":{"mask":{"asset":asset,"channel":"red","offset":alpha*(power+1.),"scale":-power,"preserve_source_alpha":true}}}]}))?;
            }
        }
        super::render::update(self, h).context("QLIE object presentation")
    }
}

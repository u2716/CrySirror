//! Native resource traversal and key/event policy. Output consists of engine
//! Layer/Ui operations and reader-backed Media/Audio requests. No pixels are
//! composed here; nested ABMPs retain their scene hierarchy.
use super::*;
use crysirror_core::service::{
    AudioParameters, AudioPreparePhase, AudioRequest, EncodedAudio, EngineQuery, MediaRequest,
    MediaSourceOptions, PcmBus,
};

pub(super) struct DrawImage {
    key: String,
    job: Option<crysirror_core::image_processing::ImageJob>,
    image: Option<UploadedImage>,
    used: bool,
}
pub(super) fn release_draw_images(
    h: &mut dyn HostServices,
    client: ClientId,
    o: &Object,
) -> Result<()> {
    for image in o
        .draw_images
        .values()
        .filter_map(|state| state.image.as_ref())
    {
        h.release_image_resource(client, image.path.relative())?;
    }
    Ok(())
}
struct Traversal<'a> {
    host: &'a mut dyn HostServices,
    client: ClientId,
    object: &'a mut Object,
    ids: &'a mut BTreeSet<u32>,
    next_layer: &'a mut u32,
    next_image: &'a mut u32,
    commands: &'a mut Vec<serde_json::Value>,
    active: usize,
    canvas: [u32; 2],
    random: &'a mut animation_clock::RandomTable,
    pool: &'a TaskPool,
    audio_level: i32,
    message_alpha: [u8; 4],
    matte: Option<crysirror_scene::layer::LayerMask>,
    collision_pending: bool,
    collision_regions: Vec<([f32;4],crysirror_scene::ui::WidgetHitMask)>,
    collision_keys: Vec<String>,
    collision_sources: Vec<String>,
    direct_sample: bool,
    instance_scope: usize,
    paused: bool,
    priority:f32,
    coverage_write:bool,
}
impl Traversal<'_> {
    fn collision_frame(&mut self, parent:u32) -> Result<crysirror_scene::projection::HierarchyFrame> {
        let mut nodes=Vec::new();
        let mut next=Some(parent);
        let mut seen=BTreeSet::new();
        while let Some(id)=next {
            ensure!(seen.insert(id) && seen.len()<=64, "QLIE collision ancestor hierarchy");
            let node=if let Some(node)=self.object.published_children.iter()
                .find(|node|node["id"].as_u64()==Some(u64::from(id))) {node.clone()}
            else {serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::Layer(id))?)?};
            let rect:[f32;4]=serde_json::from_value(node["rect"].clone()).with_context(||format!("QLIE collision ancestor {id} of {:?}: {node}",self.object.asset))?;
            let transform=if node["transform"].is_null(){Default::default()}
                else{serde_json::from_value::<crysirror_scene::layer::SpriteTransform>(node["transform"].clone())?};
            nodes.push((rect,transform));
            next=node["parent"].as_u64().map(u32::try_from).transpose()?;
        }
        let mut frame=crysirror_scene::projection::HierarchyFrame::default();
        for (rect,transform) in nodes.iter().rev(){frame.push(transform,[rect[0],rect[1]])?;}
        Ok(frame)
    }
    fn collision_quad(&mut self,parent:u32,sample:&animation::Sample,size:[f32;2],pivot:[f32;3],scale:[f32;2]) -> Result<Option<[[f32;2];4]>> {
        let frame=self.collision_frame(parent)?;
        let transform=crysirror_scene::layer::SpriteTransform{
            scale_x:scale[0],scale_y:scale[1],scale_z:sample.scale_z/100.,
            pivot_x:pivot[0],pivot_y:pivot[1],pivot_z:pivot[2],z:sample.z-pivot[2],
            rotate:sample.rotation.to_radians(),rotate_x:-sample.rotation_xy[0].to_radians(),
            rotate_y:-sample.rotation_xy[1].to_radians(),..Default::default()
        };
        Ok(frame.quad(&transform,[sample.position[0]-pivot[0],sample.position[1]-pivot[1]],
            [0.,0.,size[0],size[1]],self.canvas.map(|value|value as f32)))
    }
    fn blend_mode(&self,sample:&animation::Sample)->u8 {
        self.object.properties.get("native_bm").and_then(|v|v.first())
            .and_then(|v|v.integer().ok()).map_or(sample.native_blend,|v|v as u8)
    }
    fn sampling(&self,sample:&animation::Sample)->u8 {
        //54640C's mosaic raster selects its integer source texels regardless
        //of the ordinary texture-filter selector.
        if sample.kind==22{return 1;}
        let mode=property_int(self.object,"tf",0)as u8;
        if mode==0 {sample.native_sampling}else{mode}
    }
    fn layer(&mut self, mut spec: serde_json::Value) -> Result<u32> {
        ensure!(self.active < 2048, "QLIE live animation layer budget");
        if self.object.children.len() <= self.active {
            ensure!(*self.next_layer < 0x10000000, "QLIE layer IDs exhausted");
            self.object.children.push(*self.next_layer);
            *self.next_layer += 1;
        }
        if let Some(rgb) = self.object.colour {
            if spec["paint"]["kind"] == "image" {
                let mut tint = [255u32; 4];
                if let Some(values) = spec["paint"]["tint"].as_array() {
                    for (i, v) in values.iter().take(4).enumerate() {
                        tint[i] = v.as_u64().unwrap_or(255) as u32;
                    }
                }
                for i in 0..3 {
                    tint[i] = tint[i] * u32::from(rgb[i]) / 255;
                }
                spec["paint"]["tint"] = json!(tint);
            } else {
                if !spec["filter"].is_object() {
                    spec["filter"] = json!({});
                }
                let mut multiply = [1.; 3];
                if let Some(previous) = spec["filter"]["multiply"].as_array() {
                    for (i, v) in previous.iter().take(3).enumerate() {
                        multiply[i] = v.as_f64().unwrap_or(1.) as f32;
                    }
                }
                for i in 0..3 {
                    multiply[i] *= f32::from(rgb[i]) / 255.;
                }
                spec["filter"]["multiply"] = json!(multiply);
            }
        }
        if let Some(blend) = self.object.properties.get("native_bm").and_then(|v| v.first()) {
            spec["blend"] = json!(animation::blend(blend.integer()?as u8));
        }
        //569388 quantizes only after native single-precision addition. Its
        //negative branch truncates then subtracts one, including exact negative
        //integers; parent layers carry geometry without a sorting fence.
        let priority=self.priority+spec["order"][0].as_f64().unwrap_or(0.)as f32;
        let minor=spec["order"][1].as_i64().unwrap_or(0);
        spec["order"]=json!([queue_integer(priority)?,minor]);
        let kind=spec["paint"]["kind"].as_str().unwrap_or("");
        if self.coverage_write&&!matches!(kind,"group"|"ordered_state"|"coverage_reset"|"coverage_write") {
            spec["paint"]=json!({"kind":"coverage_write","paint":spec["paint"].clone()});
        }
        if !matches!(spec["paint"]["kind"].as_str(),Some("image"|"legacy_mesh"|"projected_mesh"|"coverage_write")) {
            //Logical carriers and solid fills do not sample a texture.
            spec.as_object_mut().unwrap().remove("sampling");
        }
        let id = self.object.children[self.active];
        spec["id"] = json!(id);
        spec["tie_order"]=json!((u64::from(self.object.layer)<<32)|self.active as u64);
        let rect: [f32; 4] = serde_json::from_value(spec["rect"].clone())?;
        ensure!(
            crysirror_scene::layer::rect_valid(rect),
            "QLIE animation geometry {rect:?}, resource {:?}, motion {}, node {id}",
            self.object.asset,
            self.object.motion
        );
        if self.object.published_children.len() <= self.active {
            self.object.published_children.push(serde_json::Value::Null);
        }
        if self.object.published_children[self.active] != spec {
            self.commands.push(json!({"op":"put","layer":spec}));
            self.commands
                .push(json!({"op":"set_depth","id":id,"depth":null}));
            self.object.published_children[self.active] = spec;
        }
        self.active += 1;
        Ok(id)
    }
    fn dual_image(
        &mut self,
        bundle: &resources::Bundle,
        motion: usize,
        track: usize,
        sample: &animation::Sample,
        primary: &UploadedImage,
    ) -> Result<Option<UploadedImage>> {
        use crysirror_core::image_processing::{ImageInput, ImageProcessState};
        use crysirror_image::{
            dual_texture::{DualTextureByte128, DualTextureMode},
            effects::ImageEffect,
        };
        let amount = sample.mask_progress.clamp(0., 255.) as u8;
        let token = (
            bundle
                .id
                .wrapping_add(self.instance_scope.wrapping_mul(4096)),
            motion,
            track,
        );
        if amount == 0 {
            return Ok(Some(primary.clone()));
        }
        let Some(member) = usize::try_from(sample.mask)
            .ok()
            .and_then(|i| bundle.images.get(i))
        else {
            return Ok(None);
        };
        ensure!(self.object.message.is_none()||!member_message_surface(member),
            "QLIE dual secondary glyph surface requires an engine raster view");
        let Some(second) = self
            .object
            .images
            .get(member.slot)
            .and_then(Option::as_ref)
            .cloned()
        else {
            return Ok(None);
        };
        let second_virtual = std::array::from_fn::<_, 2, _>(|i| {
            member.metadata["virtual_size"][i]
                .as_u64()
                .filter(|n| *n > 0)
                .unwrap_or(u64::from(second.size[i])) as f32
        });
        let primary_member = usize::try_from(sample.image)
            .ok()
            .and_then(|i| bundle.images.get(i))
            .context("QLIE dual primary member")?;
        let primary_virtual = std::array::from_fn::<_, 2, _>(|i| {
            primary_member.metadata["virtual_size"][i]
                .as_u64()
                .filter(|n| *n > 0)
                .unwrap_or(u64::from(primary.size[i])) as f32
        });
        let primary_crop = source_rect(sample.primary_rect, primary_virtual);
        if primary_crop[2] <= 0. || primary_crop[3] <= 0. {
            return Ok(None);
        }
        let primary_rect = Some([
            primary_crop[0] * primary.size[0] as f32 / primary_virtual[0],
            primary_crop[1] * primary.size[1] as f32 / primary_virtual[1],
            primary_crop[2] * primary.size[0] as f32 / primary_virtual[0],
            primary_crop[3] * primary.size[1] as f32 / primary_virtual[1],
        ]);
        let secondary_rect = if sample.secondary_rect == [0.; 4] {
            None
        } else {
            let crop = source_rect(sample.secondary_rect, second_virtual);
            Some([
                crop[0] * second.size[0] as f32 / second_virtual[0],
                crop[1] * second.size[1] as f32 / second_virtual[1],
                crop[2] * second.size[0] as f32 / second_virtual[0],
                crop[3] * second.size[1] as f32 / second_virtual[1],
            ])
        };
        let spec = DualTextureByte128 {
            mode: match sample.kind {
                9 if amount == 255 => DualTextureMode::Replace,
                9 => DualTextureMode::Crossfade,
                11 => DualTextureMode::BlueAlpha,
                24 => DualTextureMode::AlphaWeighted,
                _ => unreachable!(),
            },
            amount,
            primary_rect,
            secondary_rect,
            linear: matches!(self.sampling(sample),2|3),
        };
        let key = serde_json::to_string(&json!([
            primary.path.relative(),
            second.path.relative(),
            spec
        ]))?;
        if !self.object.draw_images.contains_key(&token) {
            ensure!(
                self.object.draw_images.len() < 2048,
                "QLIE dual texture node budget"
            );
            self.object.draw_images.insert(
                token,
                DrawImage {
                    key: String::new(),
                    job: None,
                    image: None,
                    used: true,
                },
            );
        }
        let state = self.object.draw_images.get_mut(&token).unwrap();
        state.used = true;
        if state.key != key {
            // Dropping the previous job cancels its worker and prevents an
            // obsolete animation generation publishing into this node.
            state.job = None;
            let first = self
                .host
                .published_image_resource(self.client, primary.path.relative())?
                .context("QLIE dual primary immutable resource")?;
            let second = self
                .host
                .published_image_resource(self.client, second.path.relative())?
                .context("QLIE dual secondary immutable resource")?;
            let input = ImageInput::Raster {
                destination: Box::new(ImageInput::Resource(first)),
                source: Box::new(ImageInput::Resource(second)),
            };
            let effects = serde_json::to_vec(&[ImageEffect::DualTexture { spec }])?;
            state.job = Some(
                self.host
                    .image_metadata(self.client)?
                    .begin_process(input, effects.into())?,
            );
            state.key = key;
        }
        if let Some(job) = &mut state.job {
            match job.poll()? {
                ImageProcessState::Pending => {}
                ImageProcessState::Failed(error) => bail!("QLIE ABMP dual texture: {error}"),
                ImageProcessState::Ready(resource) => {
                    *self.next_image = self
                        .next_image
                        .checked_add(1)
                        .context("QLIE dual texture publication ID")?;
                    let image = self.host.publish_image_resource(
                        self.client,
                        *self.next_image,
                        resource,
                    )?;
                    if let Some(old) = state.image.replace(image) {
                        self.host
                            .release_image_resource(self.client, old.path.relative())?;
                    }
                    state.job = None;
                }
            }
        }
        // A pending generation retains only its last completed immutable
        // result; it never aliases a mutable worker buffer or foreign frame.
        Ok(state.image.clone())
    }
    fn particles(
        &mut self,
        bundle: &resources::Bundle,
        sample: &animation::Sample,
        parent: u32,
        depth: usize,
    ) -> Result<()> {
        let Some(member) = usize::try_from(sample.image)
            .ok()
            .and_then(|i| bundle.images.get(i))
        else {
            return Ok(());
        };
        let image = self
            .object
            .images
            .get(member.slot)
            .and_then(Option::as_ref)
            .cloned();
        let nested = member.nested.as_deref();
        if nested.is_none() && image.is_none() {
            return Ok(());
        }
        let n = std::array::from_fn::<_, 8, _>(|i| {
            sample.particle_legacy[i] + sample.particle_extra[i]
        });
        let spec = crysirror_image::procedural_instances::NoiseVolume {
            shape: if sample.kind == 21 {
                crysirror_image::procedural_instances::NoiseVolumeShape::Sphere { pi: 3.14159265 }
            } else {
                crysirror_image::procedural_instances::NoiseVolumeShape::Box
            },
            count: n[0].max(0.),
            seed: (sample.mask_power + sample.particle_extra[3]).round() as i64 as u16,
            cells: (n[2].round() as u32).max(1),
            phase: [n[4] / 100., n[5] / 100., n[6] / 100.],
            extent: [sample.scale[0], sample.scale[1], sample.scale_z],
            soft_edge: n[1].max(0.) / 100.,
            timeline: [sample.seek as i64, sample.seek_end as i64],
            motions: nested.map_or(1, |b| b.animation.motions.len().max(1) as u32),
        };
        let instances = spec.sample(self.random.words())?;
        let anchor = [
            sample.scale[0] * sample.anchor[0] / 100.,
            sample.scale[1] * sample.anchor[1] / 100.,
            sample.scale_z * sample.anchor_z / 100.,
        ];
        let root=self.layer(json!({"parent":parent,"rect":[sample.position[0]-anchor[0],sample.position[1]-anchor[1],1,1],"paint":{"kind":"group"},"transform":{"z":sample.z-anchor[2],"pivot_x":anchor[0],"pivot_y":anchor[1],"pivot_z":anchor[2],"rotate":sample.rotation.to_radians(),"rotate_x":-sample.rotation_xy[0].to_radians(),"rotate_y":-sample.rotation_xy[1].to_radians()}}))?;
        let logical = nested
            .map(|b| b.animation.size.map(|n| n as f32))
            .or_else(|| image.as_ref().map(|i| i.size.map(|n| n as f32)))
            .unwrap();
        let virtual_size = std::array::from_fn::<_, 2, _>(|i| {
            member.metadata["virtual_size"][i]
                .as_u64()
                .filter(|n| *n > 0)
                .map(|n| n as f32)
                .unwrap_or(logical[i])
        });
        let factor = 1. + n[7] / 100.;
        let scale = std::array::from_fn::<_, 2, _>(|i| {
            if logical[i] > 0. {
                virtual_size[i] / logical[i] * factor
            } else {
                factor
            }
        });
        for (ordinal, instance) in instances.into_iter().enumerate() {
            if instance.opacity == 0 {
                continue;
            }
            let mut tint = sample.tint.map(|v| v.clamp(0., 255.) as u8);
            tint[3] = (u16::from(tint[3]) * u16::from(instance.opacity) / 255) as u8;
            let paint = if let Some(image) = &image {
                json!({"kind":"image","asset":image.path.relative(),"tint":tint})
            } else {
                json!({"kind":"group","tint":[tint[0],tint[1],tint[2],255],"add":sample.add})
            };
            let node=self.layer(json!({"parent":root,"rect":[instance.position[0]-virtual_size[0]*0.5,instance.position[1]-virtual_size[1]*0.5,logical[0].max(1.),logical[1].max(1.)],"order":[sample.order,0],"opacity":if nested.is_some(){tint[3]}else{255},"paint":paint,"blend":sample.blend,"sampling":{"profile":"stored_byte128","linear":matches!(self.sampling(sample),2|3)},"filter":{"add":sample.add,"multiply":if nested.is_some(){[f32::from(tint[0])/255.,f32::from(tint[1])/255.,f32::from(tint[2])/255.]}else{[1.,1.,1.]}},"transform":{"z":instance.position[2],"scale_x":scale[0],"scale_y":scale[1]}}))?;
            if let Some(child) = nested {
                let previous = self.direct_sample;
                let previous_scope = self.instance_scope;
                let previous_priority=self.priority;
                self.priority+=sample.order;
                self.instance_scope = previous_scope
                    .checked_mul(1024)
                    .and_then(|scope| scope.checked_add(ordinal + 1))
                    .context("QLIE particle instance scope")?;
                self.direct_sample = true;
                let result = self.bundle(
                    child,
                    instance.time.max(0) as u64,
                    node,
                    instance.motion as usize,
                    depth + 1,
                );
                self.direct_sample = previous;
                self.instance_scope = previous_scope;
                self.priority=previous_priority;
                result?;
            }
        }
        Ok(())
    }
    fn event(
        &mut self,
        bundle: &resources::Bundle,
        track: &animation::Track,
        track_index: usize,
        time: u64,
        cycle: u64,
        motion: usize,
        values: &animation::Values,
    ) -> Result<()> {
        let Some(index) = track
            .frames
            .partition_point(|k| k[0] as u64 <= time)
            .checked_sub(1)
        else {
            return Ok(());
        };
        let key = &track.frames[index];
        let next = track.frames.get(index + 1).unwrap_or(key);
        let value = |field: usize| -> Result<i32> {
            Ok(values
                .number(key[field], time, key[0] as u64, next[0] as u64)?
                .round() as i32)
        };
        let event = key[52];
        if !matches!(event, 8 | 11) || !track.enabled {
            return Ok(());
        }
        let token = (bundle.id, motion, track_index);
        if self.object.events.get(&token) == Some(&(index, cycle)) {
            return Ok(());
        }
        self.object.events.insert(token, (index, cycle));
        ensure!(
            self.object.events.len() <= 4096,
            "QLIE animation event history budget"
        );
        if event == 11 {
            // TABMP 5A13AC adds Value1 to the animation's base channel and
            // dispatches the sound-stop callback. It must also cancel a
            // still-preparing start, otherwise it can sound after this stop.
            ensure!(
                (0..32).contains(&value(53)?),
                "QLIE animation stop channel offset"
            );
            let channel = 0x30000000 + self.object.layer * 32 + value(53)? as u32;
            let pending: Vec<_> = self
                .object
                .audio_tickets
                .iter()
                .filter_map(|(&ticket, &owner)| (owner == channel).then_some(ticket))
                .collect();
            for ticket in pending {
                self.host.release_audio(self.client, ticket)?;
                self.object.audio_tickets.remove(&ticket);
            }
            self.host.submit_audio(
                self.client,
                AudioRequest::Stop {
                    channel,
                    fade_ms: 0.,
                },
            )?;
            self.object.audio_channels.remove(&channel);
            return Ok(());
        }
        let sound = usize::try_from(value(53)?)?;
        let Some(sound) = bundle.sounds.get(sound).filter(|s| !s.bytes.is_empty()) else {
            return Ok(());
        };
        let gain = if value(54)? == 0 {
            1.
        } else {
            value(54)? as f32 / 256.
        };
        ensure!(
            (0..32).contains(&value(56)?),
            "QLIE animation sound channel offset"
        );
        let channel = 0x30000000 + self.object.layer * 32 + value(56)? as u32;
        let pending: Vec<_> = self
            .object
            .audio_tickets
            .iter()
            .filter_map(|(&ticket, &owner)| (owner == channel).then_some(ticket))
            .collect();
        for ticket in pending {
            self.host.release_audio(self.client, ticket)?;
            self.object.audio_tickets.remove(&ticket);
        }
        // ImageLib retains its native volume-group number (5D1E20).
        let native_bus = self
            .object
            .properties
            .get("vtn")
            .and_then(|v| v.first())
            .map(Value::integer)
            .transpose()?
            .unwrap_or(2);
        let (group, bus) = match native_bus {
            0 => (0, PcmBus::Music),
            1 => (1, PcmBus::Voice),
            2 => (2, PcmBus::Sfx),
            3 => (3, PcmBus::Ui),
            4 => (4, PcmBus::Ambient),
            _ => bail!("unsupported QLIE animation sound type {native_bus}"),
        };
        let ticket = self.host.begin_audio(
            self.client,
            AudioRequest::PlayResident {
                channel,
                source: EncodedAudio {
                    bytes: sound.bytes.clone(),
                    frame_limit: None,
                },
                intro: None,
                parameters: AudioParameters {
                    bus,
                    mix_group: Some(format!("qlie/{group}")),
                    gain: gain * self.object.gain,
                    looping: value(55)? != 0,
                    ..Default::default()
                },
            },
        )?;
        self.object.audio_tickets.insert(ticket, channel);
        self.object.audio_channels.insert(channel);
        Ok(())
    }
    fn bundle(
        &mut self,
        bundle: &resources::Bundle,
        time: u64,
        parent: u32,
        motion_index: usize,
        depth: usize,
    ) -> Result<()> {
        ensure!(depth < 8, "QLIE animation hierarchy depth");
        let mut external = [0.; 32];
        for (index, value) in external.iter_mut().enumerate() {
            if let Some(argument) = self
                .object
                .properties
                .get(&format!("exv/{index}"))
                .and_then(|a| a.first())
            {
                *value = argument
                    .text()
                    .parse::<f32>()
                    .context("QLIE external animation number")?;
                ensure!(
                    value.is_finite(),
                    "QLIE external animation number is not finite"
                );
            }
        }
        let values = animation::Values {
            external,
            elapsed: time,
            size: bundle.animation.size,
            canvas: self.canvas,
        };
        ensure!(
            self.object.animation_clocks.len() < 4096
                || self.object.animation_clocks.contains_key(&bundle.id),
            "QLIE animation clock budget"
        );
        let frame = if self.direct_sample {
            let motion = bundle
                .animation
                .motions
                .get(motion_index)
                .context("QLIE particle selected motion")?;
            animation_clock::Frame {
                motion: motion_index,
                tracks: motion.layers.iter().map(|_| (time, 0)).collect(),
                enabled: Vec::new(),
            }
        } else {
            let clock = self.object.animation_clocks.entry(bundle.id).or_default();
            if self.paused {
                if let Some(frame) = clock.frozen_frame(motion_index) {
                    frame
                } else {
                    clock.advance(
                        bundle,
                        motion_index,
                        time,
                        &values,
                        &self.object.skip_flags,
                        &self.object.jump_targets,
                        self.random,
                        self.audio_level,
                    )?
                }
            } else {
                clock.advance(
                    bundle,
                    motion_index,
                    time,
                    &values,
                    &self.object.skip_flags,
                    &self.object.jump_targets,
                    self.random,
                    self.audio_level,
                )?
            }
        };
        let motion_index = frame.motion;
        if depth == 0 {
            self.object.motion = motion_index;
        }
        let motion = bundle
            .animation
            .motions
            .get(motion_index)
            .with_context(|| format!("QLIE ABMP motion {motion_index}"))?;
        let draw_root = parent;
        let mut common: Vec<animation::Sample> = Vec::new();
        for (track_index, track) in motion.layers.iter().enumerate() {
            let (local, cycle) = frame.tracks[track_index];
            if !self.direct_sample && !self.paused {
                self.event(
                    bundle,
                    track,
                    track_index,
                    local,
                    cycle,
                    motion_index,
                    &values,
                )?;
            }
            let Some(mut sample) = animation::sample(track, local, &values, self.random)? else {
                continue;
            };
            //5A7BA4 consumes the global message-opacity bank only for the
            //native opt-in track tags, rather than dimming every scene image.
            let tag = track.name.to_ascii_lowercase();
            if tag.starts_with("*m") {
                for marker in ["*message", "*mesalpha"] {
                    if tag.contains(marker) {
                        sample.tint[3] =
                            (sample.tint[3] * f32::from(self.message_alpha[0]) / 255.).trunc();
                    }
                }
            }
            // TABMP's common-key stack adds positions/angles and multiplies
            // scales/colour before constructing the draw matrix (5BD9DC).
            // A scene parent per common track would rotate translations twice.
            if let Some(previous) = common.last() {
                for i in 0..2 {
                    sample.position[i] += previous.position[i];
                    sample.scale[i] *= previous.scale[i] / 100.;
                }
                sample.z += previous.z;
                sample.scale_z *= previous.scale_z / 100.;
                for i in 0..2 {
                    sample.rotation_xy[i] += previous.rotation_xy[i];
                }
                sample.rotation += previous.rotation;
                for i in 0..4 {
                    sample.tint[i] = sample.tint[i] * previous.tint[i] / 255.;
                }
                for i in 0..3 {
                    sample.add[i] = (sample.add[i] + previous.add[i]).min(1.);
                }
                sample.seek = sample.seek.saturating_add(previous.seek);
            }
            //5A3FD0 sends channel1 and3 through the same native common-key
            //stack sampler; channel2 pops that state before ordinary drawing.
            if matches!(track.channel, 1 | 3) {
                ensure!(common.len() < 32, "QLIE common-key stack budget");
                common.push(sample);
                continue;
            }
            if track.channel == 2 {
                common.pop();
            }
            //59DEC0 initializes renderer+3276 from track+45. 5A7BA4
            //samples the native key/common stack first, then skips disabled
            //draw records. In particular the Frame JPEG in the window/choice
            //animations belongs to a disabled editor helper track; drawing it
            //covers the actual scene and dialogue. Its clock still advances.
            //`lv` overrides +3276 for this clock's motion.
            if !frame
                .enabled
                .get(track_index)
                .copied()
                .flatten()
                .unwrap_or(track.enabled)
            {
                continue;
            }
            if sample.kind == 12 {
                //5A5D40 emits queue6 with literal255. Its globally sorted
                //position resets the client coverage plane, even across objects.
                self.layer(json!({"parent":draw_root,"rect":[0,0,1,1],"order":[sample.order,if matches!(sample.native_depth,0|4){0}else{128000}],"paint":{"kind":"coverage_reset","value":255}}))?;
                continue;
            }
            if sample.kind == 0 {
                continue;
            }
            if sample.kind == 5 {
                // 568D70 requests native dirty-strip refresh. CrySirror's
                // presentation submits the complete retained frame, so there
                // is no native dirty-strip cache to invalidate or preserve.
                continue;
            }
            if sample.kind == 6 {
                //5A4740 emits a unit collision quad through the same complete
                //ancestor transform as drawing, including rotated/nested views.
                let Some(quad)=self.collision_quad(parent,&sample,[1.,1.],
                    [sample.anchor[0]/100.,sample.anchor[1]/100.,0.],sample.scale)? else {continue};
                let plane=crysirror_scene::ui::WidgetHitMask {
                    size:[1,1],coverage:Arc::from([255u8]),threshold:1,
                    strict_bounds:false,source_from_local:None,regions:Vec::new(),
                };
                if let Some((bounds,mask))=plane.projected_source(quad,[0.,0.,self.canvas[0]as f32,self.canvas[1]as f32])? {
                    ensure!(self.collision_regions.len()<256,"QLIE collision region budget");
                    self.collision_regions.push((bounds,mask));
                    self.collision_keys.push(serde_json::to_string(&json!(["rect",quad]))?);
                    self.collision_sources.push("rect".into());
                }
                continue;
            }
            if sample.kind == 8 {
                //5A4C04 draws only in the editor. In player mode5A48B8
                //registers image coverage in the native collision table.
                let Some(member) = usize::try_from(sample.image)
                    .ok()
                    .and_then(|i| bundle.images.get(i))
                else {
                    continue;
                };
                let Some(image) = self.object.images.get(member.slot).and_then(Option::as_ref).cloned()
                else {
                    continue;
                };
                let offset = member_offset(member)?;
                let reference = std::array::from_fn::<_, 2, _>(|i| {
                    member.metadata["virtual_size"][i]
                        .as_u64()
                        .filter(|v| *v > 0)
                        .unwrap_or(u64::from(image.size[i])) as f32
                });
                let Some(quad)=self.collision_quad(parent,&sample,image.size.map(|value|value as f32),[
                    reference[0]*sample.anchor[0]/100.+offset[0],
                    reference[1]*sample.anchor[1]/100.+offset[1], -offset[2],
                ],[sample.scale[0]/100.,sample.scale[1]/100.])? else {continue};
                let source = image.path.relative().to_owned();
                let key = serde_json::to_string(&json!([
                    &source, quad,
                ]))?;
                if !self.object.hit_mask_sources.contains_key(&source) {
                    if self.object.hit_mask_job.is_none() {
                        let image = self.host.published_image_resource(self.client, &source)?
                            .context("QLIE collision image lease absent")?;
                        let size = image.size();
                        //Only source alpha extraction runs on the worker. The
                        //task key excludes animated geometry, so continuous
                        //movement cannot cancel every admitted completion.
                        self.object.hit_mask_job = Some((source.clone(), self.pool.spawn(move |cancel| {
                            crysirror_scene::ui::WidgetHitMask::from_rgba_channels(
                                size, size, &image.rgba(),
                                crysirror_scene::ui::hit_mask::CoverageChannel::Alpha,
                                1, &|| cancel.check(),
                            )
                        })?));
                    }
                    let Some(mask) = self.object.hit_mask_job.as_mut().unwrap().1.try_take()? else {
                        self.collision_pending = true;
                        continue;
                    };
                    let (owner, _) = self.object.hit_mask_job.take().unwrap();
                    let bytes=self.object.hit_mask_sources.values().map(|plane|plane.coverage.len()).sum::<usize>();
                    ensure!(bytes.checked_add(mask.coverage.len()).is_some_and(|bytes|bytes<=128*1024*1024),"QLIE source coverage budget");
                    self.object.hit_mask_sources.insert(owner,mask);
                    if !self.object.hit_mask_sources.contains_key(&source) {
                        //Finish an already admitted source before requesting
                        //the latest image. Never publish a different image's
                        //coverage as this draw's successful collision result.
                        self.collision_pending = true;
                        continue;
                    }
                }
                let plane = &self.object.hit_mask_sources[&source];
                let Some((bounds,mask))=plane.projected_source(quad,[0.,0.,self.canvas[0]as f32,self.canvas[1]as f32])? else {continue};
                ensure!(self.collision_regions.len()<256,"QLIE collision region budget");
                self.collision_regions.push((bounds,mask));
                self.collision_keys.push(key);
                self.collision_sources.push(source);
                continue;
            }
            if matches!(sample.kind, 15 | 16 | 17) {
                use crysirror_scene::layer::ordered_state::{Command,State};
                let mode=self.blend_mode(&sample);
                let command=match sample.kind {
                    15=>Command::Reset,17=>Command::Pop,
                    _=>Command::Push{state:State{
                        translation:[sample.position[0],sample.position[1],sample.z],
                        scale:[sample.scale[0]/100.,sample.scale[1]/100.,sample.scale_z/100.],
                        rotation:sample.rotation.to_radians(),pivot:[self.canvas[0]as f32*0.5,self.canvas[1]as f32*0.5,1000.],
                        tint:sample.tint.map(|v|v.clamp(0.,255.)as u8),
                        add:sample.add.map(|v|(v*255.).clamp(0.,255.)as u8),
                        blend:if mode&0x7f==0{None}else{Some(animation::blend(mode))},
                        linear:match self.sampling(&sample){1|4=>Some(false),2|3=>Some(true),_=>None},
                    }},
                };
                //567A34 applies these after the complete queue is sorted,
                //including records belonging to other image objects.
                self.layer(json!({"parent":draw_root,"rect":[0,0,1,1],"order":[sample.order,if matches!(sample.native_depth,0|4){0}else{128000}],"paint":{"kind":"ordered_state","command":command}}))?;
                continue;
            }
            let draw_parent = draw_root;
            if matches!(sample.kind, 20 | 21) {
                self.particles(bundle, &sample, draw_parent, depth)?;
                continue;
            }
            ensure!(
                matches!(
                    sample.kind,
                    1 | 2 | 3 | 4 | 7 | 9 | 10 | 11 | 13 | 14 | 18 | 19 | 22 | 23 | 24
                ),
                "unsupported QLIE ABMP drawing kind {}",
                sample.kind
            );
            let mut tint = sample.tint.map(|v| v.clamp(0., 255.) as u8);
            if matches!(sample.kind,18|19) {
                let Some(member)=usize::try_from(sample.image).ok().and_then(|i|bundle.images.get(i))else{continue};
                //5A64DC requires the original model member, never a bitmap or
                //a nested ABMP. Resource workers own import and legacy upgrade.
                let Some(model)=member.scene_model.as_ref()else{continue};
                let selected=sample.model_object;
                if selected<0 {continue}
                //5753AC delegates to43EE40's checked TList index. The
                //native profile has no implicit white material for index-1.
                //Only actually selected faces are inspected: hidden objects
                //and an invalid selection do not execute the material getter.
                let objects:Vec<_>=if selected==255{model.objects.iter().filter(|o|o.visible).collect()}else{
                    model.objects.get(selected as usize).into_iter().collect()
                };
                for object in objects {
                    let object=if sample.kind==19&&selected!=255&&f64::from(sample.model_morph_weight)>0.99999 {
                        model.objects.get(sample.model_morph_object as usize).filter(|target|
                            !std::ptr::eq(*target,object)&&target.vertices.len()==object.vertices.len()&&target.faces.len()==object.faces.len()).unwrap_or(object)
                    }else{object};
                    ensure!(object.faces.iter().all(|face|face.material>=0),"QLIE native model selected a face without a material");
                }
                let add=sample.add.map(|v|(v.min(1.)*255.)as u8);
                let spec=crysirror_scene::legacy_mesh::Spec {
                    model:model.clone(),
                    object:if selected==255{None}else{Some(selected as u32)},
                    //57A6B8 passes no morph target when source selects all.
                    morph_object:(sample.kind==19&&selected!=255).then_some(sample.model_morph_object as u32),
                    morph_weight:if sample.kind==19{sample.model_morph_weight}else{0.},
                    //57A414 uses basis[1,-1,1] for the owner's platform flag1.
                    //Ordinary ABMP geometry is Z-reflected by5A1418; the model
                    //entry bypasses that final reflection. Convert the source
                    //basis and the complete world pose explicitly, including
                    //camera ancestors, before shared lighting and projection.
                    basis:[1.,-1.,-1.],view_origin:Some([self.canvas[0]as f32*0.5,self.canvas[1]as f32*0.5,0.]),
                    post_transform:Some([1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,-1.,0.,0.,0.,2000.,1.]),
                    culling:true,
                    light_direction:Some([model.scene.light_direction[0],-model.scene.light_direction[1],-model.scene.light_direction[2]]),
                    uv_offset:[sample.primary_rect[0]/100.,sample.primary_rect[1]/100.],
                    //539638 clamps vertices before raster interpolation.
                    uv_bounds:Some([0.,0.,1.,1.]),
                    tint,add,
                    blend_override:if self.blend_mode(&sample)&0x7f==0{None}else{Some(animation::blend(self.blend_mode(&sample)))},
                    sampling_override:match self.sampling(&sample) {1|4=>Some(false),2|3=>Some(true),_=>None},
                    textures:(0..model.materials.len()).map(|index|bundle.images.get(index+1)
                        .and_then(|m|self.object.images.get(m.slot)).and_then(Option::as_ref)
                        .map(|image|image.path.relative().to_owned())).collect(),
                    ignore_coverage:false,
                };
                let offset=member_offset(member)?;
                self.layer(json!({"parent":draw_parent,"rect":[sample.position[0]-offset[0],sample.position[1]-offset[1],1,1],"order":[sample.order,0],"paint":{"kind":"legacy_mesh","spec":spec},"sampling":{"profile":"stored_byte128"},"transform":{"geometry_order":geometry_order(&sample,false),"pivot_x":offset[0],"pivot_y":offset[1],"pivot_z":-offset[2],"z":sample.z+offset[2],"scale_x":sample.scale[0]/100.,"scale_y":sample.scale[1]/100.,"scale_z":sample.scale_z/100.,"rotate_x":-sample.rotation_xy[0].to_radians(),"rotate_y":-sample.rotation_xy[1].to_radians(),"rotate":sample.rotation.to_radians()}}))?;
                continue;
            }
            let mut nested = None;
            let mut image_offset = [0.; 3];
            let mut reference_z = 0.;
            let mut virtual_depth=0.;
            let mut virtual_size = None;
            let mut text_surface = false;
            let mut snapshot_view = false;
            let mut view_uv = serde_json::Value::Null;
            let mut direct_view_uv=false;
            let (mut paint, size) = if matches!(sample.kind, 1 | 14) {
                // A unit quad keeps zero, negative and very large native fill
                // dimensions in the shared transform instead of invalid bounds.
                (json!({"kind":"solid","color":tint}), [1., 1.])
            } else {
                let Some(member) = usize::try_from(sample.image)
                    .ok()
                    .and_then(|i| bundle.images.get(i))
                else {
                    continue;
                };
                //Native3/4/7/13 dispatch nested animations explicitly. The
                //other image packet paths require a bitmap (member+200) and
                //emit no pixels when only a child animation is installed.
                if member.nested.is_some()&&!matches!(sample.kind,3|4|7|13){continue;}
                if sample.kind == 7 && member.nested.is_none() {
                    //5A6234 is the explicit nested-animation form. With no
                    //ABMP member it emits nothing, unlike image kind4/13.
                    continue;
                }
                if self
                    .object
                    .properties
                    .contains_key(&format!("empty_slot/{}", member.slot))
                {
                    continue;
                }
                image_offset = member_offset(member)?;

                let extent = [
                    member.metadata["virtual_size"][0].as_u64().unwrap_or(0),
                    member.metadata["virtual_size"][1].as_u64().unwrap_or(0),
                ];
                virtual_depth=member.metadata["virtual_size"][2].as_u64().unwrap_or(0)as f32;
                if extent[0] > 0 || extent[1] > 0 {
                    ensure!(
                        extent.iter().all(|v| *v <= 8192),
                        "QLIE image virtual extent"
                    );
                    virtual_size = Some([extent[0] as f32, extent[1] as f32]);
                }
                let message_binding = member.metadata["native_name"]
                    .as_str()
                    .unwrap_or("")
                    .split('*')
                    .skip(1)
                    .any(|part| {
                        let mut fields = part.split(',');
                        let name = fields.next().unwrap_or("");
                        //5A3B28 resolves both attributes through the same
                        //four imported-image slots. TMessageLib's constructor
                        //69D02C publishes its text bitmap specifically in0.
                        //Other indices retain their actual archive image when
                        //no corresponding external image has been installed.
                        (name.eq_ignore_ascii_case("messurface")
                            || name.eq_ignore_ascii_case("extimage"))
                            && fields
                                .next()
                                .and_then(crate::expression::parse_integer)
                                .unwrap_or(0)
                                == 0
                    });
                //ABMP resource attributes mesalpha/extalpha bind a chosen
                //global opacity slot, independently of the track-name gate.
                for attribute in member.metadata["native_name"]
                    .as_str()
                    .unwrap_or("")
                    .split('*')
                    .skip(1)
                {
                    let mut fields = attribute.split(',');
                    if fields.next().is_some_and(|tag| {
                        tag.eq_ignore_ascii_case("mesalpha") || tag.eq_ignore_ascii_case("extalpha")
                    }) {
                        let index = fields
                            .next()
                            .and_then(|v| crate::expression::parse_integer(v))
                            .unwrap_or(0);
                        if (0..4).contains(&index) {
                            tint[3] = (u32::from(tint[3])
                                * u32::from(self.message_alpha[index as usize])
                                / 255) as u8;
                        }
                    }
                }
                if let Some(view) = self.object.capture_views.get(&member.slot) {
                    snapshot_view = true;
                    view_uv = view["uv"].clone();
                    (
                        view["paint"].clone(),
                        [self.canvas[0] as f32, self.canvas[1] as f32],
                    )
                } else if message_binding && self.object.message.is_some() {
                    // TMessageLib registers its reusable text surface as
                    // imported image 0; *MesSurface is the native binding
                    // (5A3B28). Keep the text as an engine GlyphRun under the
                    // sampled animation node, replacing the editor placeholder.
                    let message=self.object.message.as_ref().unwrap();
                    if !message.created {continue;}
                    let extent=message.surface_size();
                    let amount=sample.mask_progress.clamp(0.,255.)as u8;
                    if sample.kind==9&&amount==255 {
                        //5A5514's endpoint replaces primary RGBA with the
                        //mapped secondary. It does not sample primary glyphs.
                        let Some(second)=usize::try_from(sample.mask).ok().and_then(|i|bundle.images.get(i))else{continue};
                        ensure!(!member_message_surface(second),"QLIE glyph-surface secondary replacement requires an engine raster view");
                        let Some(image)=self.object.images.get(second.slot).and_then(Option::as_ref)else{continue};
                        let reference=std::array::from_fn(|i|second.metadata["virtual_size"][i].as_u64().filter(|v|*v>0).unwrap_or(u64::from(image.size[i]))as f32);
                        let crop=source_rect(sample.secondary_rect,reference);
                        ensure!(crop[2]>0.&&crop[3]>0.,"QLIE glyph-surface replacement requires a nonempty secondary sample view");
                        view_uv=json!([crop[0]/reference[0],crop[1]/reference[1],(crop[0]+crop[2])/reference[0],(crop[1]+crop[3])/reference[1]]);
                        direct_view_uv=true;
                        (json!({"kind":"image","asset":image.path.relative(),"tint":tint}),extent)
                    }else{
                        //Imported text remains one engine-owned reveal source.
                        //Nonlinear bitmap operations need a raster view of the
                        //complete source, not independently filtered glyphs.
                        ensure!(!matches!(sample.kind,9|11|24)||amount==0,"QLIE glyph-surface dual drawing requires an engine raster view (kind {})",sample.kind);
                        ensure!(sample.kind!=10||sample.mask_progress<=0.||sample.mask_progress>=255.||sample.mask_power==0.,"QLIE glyph-surface spatial rule mask requires an engine raster view");
                        //The native minimum mosaic cell is2, including Value1=0.
                        ensure!(sample.kind!=22||extent.iter().all(|v|*v<=1.),"QLIE glyph-surface mosaic requires an engine raster view");
                        text_surface = true;
                        (json!({"kind":"glyph_view","source":message.layer,"offset":[0.,0.]}),extent)
                    }
                } else if let Some(child) = &member.nested {
                    nested = Some(child.as_ref());
                    reference_z=child.animation.depth as f32;
                    (
                        json!({"kind":"group"}),
                        [
                            child.animation.size[0] as f32,
                            child.animation.size[1] as f32,
                        ],
                    )
                } else if member.metadata["kind"] == 7 {
                    if !self.object.embedded_media.contains_key(&member.slot) {
                        let id = (0..1024)
                            .find(|n| !self.ids.contains(n))
                            .context("QLIE media IDs exhausted")?;
                        let path = self.host.media_reader(
                            self.client,
                            id,
                            Arc::new(resources::Memory(member.bytes.clone())),
                            MediaSourceOptions {
                                looping: true,
                                audio: false,
                                gain: 1.,
                            },
                        )?;
                        self.ids.insert(id);
                        self.host
                            .media_request(self.client, MediaRequest::SetClock { id, now_ms: 0 })?;
                        if sample.seek > 0 {
                            self.host.media_request(
                                self.client,
                                MediaRequest::Seek {
                                    id,
                                    position_ms: sample.seek,
                                },
                            )?;
                        }
                        self.object.embedded_media.insert(
                            member.slot,
                            EmbeddedMedia {
                                id,
                                path: path.relative().into(),
                                native_ms: sample.seek,
                                clock_ms: 0,
                            },
                        );
                    }
                    let media = self.object.embedded_media.get_mut(&member.slot).unwrap();
                    if sample.seek < media.native_ms {
                        self.host.media_request(
                            self.client,
                            MediaRequest::Seek {
                                id: media.id,
                                position_ms: sample.seek,
                            },
                        )?;
                    } else {
                        media.clock_ms = media
                            .clock_ms
                            .checked_add(sample.seek - media.native_ms)
                            .context("QLIE media clock overflow")?;
                    }
                    media.native_ms = sample.seek;
                    self.host.media_request(
                        self.client,
                        MediaRequest::SetClock {
                            id: media.id,
                            now_ms: media.clock_ms,
                        },
                    )?;
                    let path = &media.path;
                    let info = self.host.media_info(self.client, media.id)?;
                    let Some(size) = info.size else { continue };
                    (
                        json!({"kind":"image","asset":path,"tint":tint}),
                        [size[0] as f32, size[1] as f32],
                    )
                } else {
                    let Some(image) = self.object.images.get(member.slot).and_then(|i| i.as_ref())
                    else {
                        continue;
                    };
                    let image = image.clone();
                    let image = if matches!(sample.kind, 9 | 11 | 24) {
                        let Some(image) =
                            self.dual_image(bundle, motion_index, track_index, &sample, &image)?
                        else {
                            continue;
                        };
                        image
                    } else {
                        image
                    };
                    (
                        json!({"kind":"image","asset":image.path.relative(),"tint":tint}),
                        [image.size[0] as f32, image.size[1] as f32],
                    )
                }
            };
            let reference=virtual_size.map_or(size,|v|std::array::from_fn(|i|if v[i]>0.{v[i]}else{size[i]}));
            let texture_size=size;
            let mut size = size;
            if !matches!(sample.kind, 1 | 14) && nested.is_none() {
                let crop = source_rect(sample.primary_rect, reference);
                if crop[2] <= 0. || crop[3] <= 0. {
                    if self.coverage_write||sample.kind==13 {
                        //567908 admits the coverage plane before raster cull.
                        //An empty image is still a zero-pixel write, preserving
                        //the distinction from no coverage producer at all.
                        self.layer(json!({"parent":draw_parent,"rect":[sample.position[0],sample.position[1],1,1],"order":[sample.order,0],"opacity":0,"paint":{"kind":"coverage_write","paint":{"kind":"solid","color":[0,0,0,0]}},"transform":{"z":sample.z,"scale_x":0,"scale_y":0,"geometry_order":geometry_order(&sample,false)}}))?;
                    }
                    continue;
                }
                let base_uv: [f32; 4] = if view_uv.is_null() {
                    [0., 0., 1., 1.]
                } else {
                    serde_json::from_value(view_uv.clone())?
                };
                if text_surface {paint["offset"]=json!([-crop[0],-crop[1]]);} else if !direct_view_uv {
                view_uv = json!([
                    base_uv[0] + (base_uv[2] - base_uv[0]) * crop[0] / reference[0],
                    base_uv[1] + (base_uv[3] - base_uv[1]) * crop[1] / reference[1],
                    base_uv[0] + (base_uv[2] - base_uv[0]) * (crop[0] + crop[2]) / reference[0],
                    base_uv[1] + (base_uv[3] - base_uv[1]) * (crop[1] + crop[3]) / reference[1]
                ]);
                }
                size = [crop[2], crop[3]];
            }
            let absolute=(sample.kind==3||sample.absolute_dimensions)&&sample.kind!=2;
            //5A4350 computes the anchor from the draw rectangle (not the
            //uncropped image), snaps before adding the member's F32 offset.
            let absolute_bitmap=absolute&&nested.is_none();
            let anchor_reference=if absolute_bitmap{[100.,100.]}else{size};
            let anchor=|axis:usize|->Result<f32>{
                let value=anchor_reference[axis]*sample.anchor[axis]/100.;
                Ok(if sample.snap_anchor{queue_integer(value)?as f32}else{value})
            };
            let mut pivot=[anchor(0)?+image_offset[0],anchor(1)?+image_offset[1]];
            if absolute_bitmap {for axis in 0..2 {pivot[axis]*=size[axis]/100.;}}
            //All image/nested wrappers pass literal0 as reference depth to
            //5A4350. Child canvas depth affects scaling, not the anchor.
            let zanchor=0.;
            let pivot_z=-((if sample.snap_anchor{queue_integer(zanchor)?as f32}else{zanchor})+image_offset[2]);
            let scale=if matches!(sample.kind,1|14){sample.scale}
                else if sample.kind==2{[1.,1.]}
                else if absolute{std::array::from_fn(|i|if size[i]>0.{sample.scale[i]/size[i]}else{1.})}
                else if nested.is_some(){std::array::from_fn(|i|sample.scale[i]/100.*if size[i]>0.{reference[i]/size[i]}else{1.})}
                else{[sample.scale[0]/100.,sample.scale[1]/100.]};
            let scale_z=if nested.is_some(){
                if absolute{if reference_z>0.{sample.scale_z/reference_z}else{1.}}
                else{sample.scale_z/100.*if reference_z>0.&&virtual_depth>0.{virtual_depth/reference_z}else{1.}}
            }else{sample.scale_z/100.};
            let add = sample.add.map(|v|v.min(1.));
            let mut filter = json!({"add":add});
            if let Some(mask) = &self.matte {
                filter["mask"] = serde_json::to_value(mask)?;
            }
            if sample.kind == 22 {
                //5A5084 quantizes image sampling to2<<(Value1/32) texels.
                //The shared distortion owns the sampling operation.
                let cell = (2u32 << ((sample.mask_progress as i32).clamp(0, 255) / 32)) as f32;
                filter["distortion"] = json!({"kind":"quantized","steps":[(texture_size[0]/cell).max(1.),(texture_size[1]/cell).max(1.)],"phase":[0.,0.]});
            }
            if snapshot_view || text_surface {
                filter["multiply"] = json!(tint[..3]
                    .iter()
                    .map(|v| f32::from(*v) / 255.)
                    .collect::<Vec<_>>());
            }
            if sample.kind == 10 {
                let mask_member = usize::try_from(sample.mask)
                    .ok()
                    .and_then(|i| bundle.images.get(i))
                    .context("QLIE rule mask index")?;
                let mask = self
                    .object
                    .images
                    .get(mask_member.slot)
                    .and_then(|i| i.as_ref())
                    .context("QLIE rule mask must be a decoded image")?;
                ensure!(
                    (0.0..=256.0).contains(&sample.mask_power),
                    "QLIE rule mask power"
                );
                filter["mask"] = json!({"asset":mask.path.relative(),"channel":"red",
                    "offset":sample.mask_progress*(sample.mask_power+1.0),
                    "scale":-sample.mask_power,"preserve_source_alpha":true});
            }
            // A nested ABMP can declare a zero logical canvas. It still
            // owns a transform for its children; Layers do not clip this
            // transparent container, so a unit backing rectangle preserves
            // the original zero reference/pivot without invalid geometry.
            let node_size = if nested.is_some() {
                size.map(|v| v.max(1.))
            } else {
                size
            };
            if nested.is_some() {
                paint["tint"]=json!([tint[0],tint[1],tint[2],255]);
                paint["add"]=json!(sample.add);
            }
            //5A5288 (type23) retains the parent's transformed anchor while
            //discarding its linear basis. The shared Layer property performs
            //that operation for live and frozen composition alike.
            let previous_coverage=self.coverage_write;
            self.coverage_write|=sample.kind==13;
            let node=self.layer(json!({"parent":draw_parent,"rect":[sample.position[0]-pivot[0],sample.position[1]-pivot[1],node_size[0],node_size[1]],"order":[sample.order,0],"opacity":if snapshot_view||text_surface||nested.is_some() {tint[3]}else{255},"paint":paint,"uv":view_uv,"sampling":{"profile":"stored_byte128","linear":matches!(self.sampling(&sample),2|3)},"filter":if nested.is_some() {serde_json::Value::Null}else{filter.clone()},"source_crop":if text_surface{json!([0.,0.,size[0],size[1]])}else{serde_json::Value::Null},"transform":{"geometry_order":geometry_order(&sample,text_surface),"z":sample.z-pivot_z,"scale_x":scale[0],"scale_y":scale[1],"scale_z":scale_z,"pivot_x":pivot[0],"pivot_y":pivot[1],"pivot_z":pivot_z,"rotate":sample.rotation.to_radians(),"rotate_x":-sample.rotation_xy[0].to_radians(),"rotate_y":-sample.rotation_xy[1].to_radians(),"inherit_parent_basis":sample.kind!=23},"blend":sample.blend}))?;
            if let Some(child) = nested {
                let previous_priority=self.priority;
                self.priority+=sample.order;
                let result=self.bundle(child, sample.seek, node, sample.nested_motion, depth + 1);
                self.priority=previous_priority;
                self.coverage_write=previous_coverage;
                result?;
            }
            self.coverage_write=previous_coverage;
        }
        Ok(())
    }
}
fn member_message_surface(member:&resources::Member)->bool {
    member.metadata["native_name"].as_str().unwrap_or("").split('*').skip(1).any(|part|{
        let mut fields=part.split(',');let name=fields.next().unwrap_or("");
        (name.eq_ignore_ascii_case("messurface")||name.eq_ignore_ascii_case("extimage"))
            &&fields.next().and_then(crate::expression::parse_integer).unwrap_or(0)==0
    })
}
fn member_offset(member: &resources::Member) -> Result<[f32; 3]> {
    //5BB6DC serializes the IEEE754 words from member+68/72/76;
    //5A3D94 copies them unchanged and5A4350 consumes floats. Archive
    //metadata retains those raw signed32-bit words, not integer pixels.
    let offset = std::array::from_fn(|i| {
        f32::from_bits(member.metadata["offset"][i].as_i64().unwrap_or(0) as u32)
    });
    ensure!(
        offset.iter().all(|v| v.is_finite()),
        "QLIE non-finite resource offset for{}",
        member.name
    );
    Ok(offset)
}
//569388 uses FISTP(TRUNC), followed by an unconditional decrement for
//negative values. Preserve that boundary rather than replacing it with floor.
fn queue_integer(value:f32)->Result<i32>{
    ensure!(value.is_finite(),"QLIE non-finite queue coordinate");
    let value=f64::from(value.trunc())-if value<0.{1.}else{0.};
    ensure!(value>=i32::MIN as f64&&value<=i32::MAX as f64,"QLIE queue coordinate overflow");
    Ok(value as i32)
}
fn geometry_order(sample:&animation::Sample,descendants:bool)->Option<crysirror_scene::projection::GeometryOrder>{
    use crysirror_scene::projection::{GeometryOrder,DepthAggregate};
    let aggregate=match sample.native_depth{1=>DepthAggregate::Average,2=>DepthAggregate::Maximum,3=>DepthAggregate::Minimum,_=>return None};
    Some(GeometryOrder{aggregate,xy_weight:0.001,quantize:128.,negative_truncation_bias:true,apply_to_descendants:descendants})
}
///5A3A40/5A3AA0: only an all-zero declaration selects the whole resource;
///an explicitly empty/outside crop stays empty after clipping.
fn source_rect(mut rect: [f32; 4], size: [f32; 2]) -> [f32; 4] {
    if rect == [0.; 4] {
        rect = [0., 0., size[0], size[1]];
    }
    for i in 0..2 {
        rect[i] = rect[i].max(0.);
        rect[i + 2] = rect[i + 2].min(size[i] - rect[i]).max(0.);
    }
    rect
}
fn property_int(object: &Object, key: &str, default: i32) -> i32 {
    object
        .properties
        .get(key)
        .and_then(|v| v.first())
        .and_then(|v| v.integer().ok())
        .unwrap_or(default)
}
pub(super) fn image_extent(object: &Object) -> [f32; 2] {
    object
        .bundle
        .as_ref()
        .map(|b| b.animation.size)
        .or_else(|| object.images.iter().flatten().next().map(|i| i.size))
        .unwrap_or([0, 0])
        .map(|n| n as f32)
}
/// Native scrollbar semantics select engine-owned retained slider input,
/// image layers and scalar tween sampling. No adapter-side pixel composition.
fn update_bars(
    vm: &mut QlieVm,
    h: &mut dyn HostServices,
    commands: &mut Vec<serde_json::Value>,
    ui: &mut Vec<serde_json::Value>,
) -> Result<()> {
    let host_now = h.now_ms();
    let input_thread = vm.input_thread();
    let widget_roots = [vm.root_layer_for(ThreadKind::Game), vm.root_layer_for(ThreadKind::System)];
    let bars: Vec<_> = vm
        .objects
        .iter()
        .filter(|(_, o)| {
            o.kind == "bar"
                && o.created
                && o.thread == vm.host_thread
                && !o.preparing
                && !o.properties.contains_key("bar/loading")
        })
        .map(|(id, _)| id.clone())
        .collect();
    for id in bars {
        let bar = &vm.objects[&id];
        let now = vm.visual_time(bar.thread, bar.started, host_now);
        let child = |part: &str| {
            bar.properties
                .get(&format!("bar/{part}_id"))
                .and_then(|v| v.first())
                .map(Value::text)
        };
        let thumb_id = child("thumb").context("QLIE scrollbar has no thumb resource")?;
        let track_id = child("track").context("QLIE scrollbar has no track resource")?;
        let fill_id = child("fill");
        if !bar.visible {
            if bar.ui_registered {
                ui.push(json!({"op":"set_visible","id":bar.layer,"visible":false}));
            }
            continue;
        }
        let thumb = image_extent(
            vm.objects
                .get(&thumb_id)
                .context("QLIE scrollbar thumb object")?,
        );
        let track = image_extent(
            vm.objects
                .get(&track_id)
                .context("QLIE scrollbar track object")?,
        );
        // A zero declared thumb canvas is deliberate: scroll_button.b
        //uses a centered32px image with logical0x0, so the endpoint center
        //can reach both edges of the track. Native6A4050 keeps those zeros.
        //Only the retained input rectangle needs a positive track extent.
        if track.iter().any(|v| *v <= 0.) {
            if bar.ui_registered {
                ui.push(json!({"op":"set_visible","id":bar.layer,"visible":false}));
            }
            continue;
        }
        let vertical = property_int(bar, "bar/vertical", 0) != 0;
        let axis = usize::from(vertical);
        let minimum = (thumb[axis] as i32 / 2) as f32;
        let maximum = (track[axis] - minimum).max(minimum + 1.);
        let extent = track[axis].max(maximum + minimum);
        let maximum_value = property_int(bar, "bar/max", 100).max(0);
        let layer = bar.layer;
        if bar.ui_registered {
            let events: Vec<crysirror_scene::ui::UiEvent> = serde_json::from_slice(
                &h.query_engine(vm.client, EngineQuery::UiSubtreeEvents(layer))?,
            )?;
            if let Some(event) = events.into_iter().rev().find(|e| e.kind == "change") {
                let value = (event.value * maximum_value as f32)
                    .round()
                    .clamp(0., maximum_value as f32) as i32;
                let bar = vm.objects.get_mut(&id).unwrap();
                if value != property_int(bar, "bar/value", 0) {
                    bar.properties
                        .insert("bar/value".into(), vec![Value::Int(value)]);
                    bar.properties
                        .insert("bar/value_duration".into(), vec![Value::Int(0)]);
                    bar.properties
                        .insert("bar/pending_change".into(), vec![Value::Int(1)]);
                }
            }
        }
        let bar = &vm.objects[&id];
        let target = property_int(bar, "bar/value", 0).clamp(0, maximum_value);
        let started = bar
            .properties
            .get("bar/value_started")
            .and_then(|v| v.first())
            .and_then(|v| v.text().parse::<u64>().ok())
            .unwrap_or(now);
        let current = crysirror_scene::animation::ScalarTween::new(
            property_int(bar, "bar/value_from", target) as f32,
            target as f32,
            started,
            property_int(bar, "bar/value_duration", 0).max(0) as u64,
            crysirror_scene::layer::Easing::Progress {
                shape: crysirror_image::progress_curve::ProgressShape::Power {
                    exponent: 3.8,
                    direction: crysirror_image::progress_curve::ProgressDirection::Out,
                },
            },
        )?
        .sample(now);
        let normalized = if maximum_value == 0 {
            0.
        } else {
            (current / maximum_value as f32).clamp(0., 1.)
        };
        let center = minimum + ((maximum - minimum) * normalized).round();
        let mut thumb_position = [
            ((track[0] - thumb[0]) as i32 / 2) as f32,
            ((track[1] - thumb[1]) as i32 / 2) as f32,
        ];
        thumb_position[axis] = center - minimum;
        let native_layer: serde_json::Value =
            serde_json::from_slice(&h.query_engine(vm.client, EngineQuery::Layer(layer))?)?;
        let x = native_layer["rect"][0]
            .as_f64()
            .context("QLIE scrollbar x")? as f32;
        let y = native_layer["rect"][1]
            .as_f64()
            .context("QLIE scrollbar y")? as f32;
        let rect = [
            x,
            y,
            if vertical {
                track[0].max(thumb[0])
            } else {
                extent
            },
            if vertical {
                extent
            } else {
                track[1].max(thumb[1])
            },
        ];
        let insets = [minimum, extent - maximum];
        if bar.ui_registered {
            ui.push(json!({"op":"set_visible","id":layer,"visible":true}));
            ui.push(json!({"op":"set_enabled","id":layer,"enabled":bar.thread==input_thread&&!bar.locked}));
            if bar.ui_rect != Some(rect) {
                ui.push(json!({"op":"set_rect","id":layer,"rect":rect}));
            }
            ui.push(json!({"op":"set_value","id":layer,"value":if maximum_value==0{0.}else{target as f32/maximum_value as f32}}));
        } else {
            ui.push(json!({"op":"put","widget":{"id":layer,"parent":widget_roots[usize::from(bar.thread == ThreadKind::System)],"rect":rect,"z":bar.position[2],"kind":"slider","slider_axis":if vertical{"vertical"}else{"horizontal"},"slider_inset":insets,"visible":true,"enabled":bar.thread==input_thread&&!bar.locked,"interaction_only":true,"value":if maximum_value==0{0.}else{target as f32/maximum_value as f32}}}));
        }
        let bar = vm.objects.get_mut(&id).unwrap();
        let bar_priority=bar.position[2];
        bar.ui_registered = true;
        bar.ui_rect = Some(rect);
        for (child_id, position, z, is_fill) in [
            (Some(track_id), [0., 0.], 0, false),
            (fill_id, [0., 0.], 5, true),
            (Some(thumb_id), thumb_position, 10, false),
        ] {
            let Some(child_id) = child_id else { continue };
            let object = vm
                .objects
                .get_mut(&child_id)
                .context("QLIE scrollbar image child")?;
            object.position[2]=bar_priority.checked_add(z).context("QLIE scrollbar priority overflow")?;
            if !object.created {
                commands.push(json!({"op":"put","layer":{"id":object.layer,"parent":layer,"rect":[position[0],position[1],vm.config.canvas[0],vm.config.canvas[1]],"order":[z,0],"paint":{"kind":"group"}}}));
                commands.push(json!({"op":"set_depth","id":object.layer,"depth":null}));
                object.created = true;
            } else {
                commands.push(json!({"op":"set_parent","id":object.layer,"parent":layer}));
                commands.push(json!({"op":"set_rect","id":object.layer,"rect":[position[0],position[1],vm.config.canvas[0],vm.config.canvas[1]]}));
                commands.push(json!({"op":"set_order","id":object.layer,"order":[z,0]}));
                commands.push(json!({"op":"set_visible","id":object.layer,"visible":true}));
            }
            object.visible = true;
            if is_fill {
                let duration = object
                    .bundle
                    .as_ref()
                    .and_then(|b| b.animation.motions.get(object.motion))
                    .map(|m| m.duration_ms)
                    .unwrap_or(0);
                object.started = now;
                object.offset = (duration as f32 * normalized) as u64;
            }
        }
    }
    Ok(())
}
pub(super) fn update(vm: &mut QlieVm, h: &mut dyn HostServices) -> Result<()> {
    let host_now = h.now_ms();
    let frozen_game_ms = vm.game_pause_ms;
    let mut frozen_random = vm.paused_random.clone();
    let input_thread = vm.input_thread();
    let widget_roots = [vm.root_layer_for(ThreadKind::Game), vm.root_layer_for(ThreadKind::System)];
    let mut commands = Vec::new();
    let mut ui = Vec::new();
    update_bars(vm, h, &mut commands, &mut ui)?;
    // Scrollbar part roots must exist before parts query their hierarchy.
    if !commands.is_empty() || !ui.is_empty() {
        vm.submit(
            h,
            json!({"layers":std::mem::take(&mut commands),"ui":std::mem::take(&mut ui)}),
        )?;
    }
    let channels: BTreeMap<u32, u32> = vm
        .objects
        .values()
        .filter_map(|o| o.wave_channel)
        .map(|index| {
            let channel = vm
                .machine
                .as_ref()
                .and_then(|machine| machine.value(&format!("snd[{index}]")).ok())
                .and_then(|value| vm.objects.get(&value.text()))
                .map(|sound| sound.channel)
                .unwrap_or(0);
            (index, channel)
        })
        .collect();
    let message_alpha = vm
        .objects
        .values()
        .find(|o| o.kind == "cfg")
        .map(|o| {
            std::array::from_fn(|i| {
                o.properties
                    .get(&format!("ma/{i}"))
                    .and_then(|v| v.first())
                    .and_then(|v| v.integer().ok())
                    .unwrap_or(255)
                    .clamp(0, 255) as u8
            })
        })
        .unwrap_or([255; 4]);
    let input_masks: [i32; 2] = [ThreadKind::Game, ThreadKind::System].map(|thread| {
        vm.objects
            .get(&QlieVm::builtin("sys", thread))
            .and_then(|s| s.properties.get("in"))
            .and_then(|v| v.first())
            .and_then(|v| v.integer().ok())
            .unwrap_or(0xFFFFFFF)
    });
    // 6B1E04 updates objects with pri > 0, then 0, then < 0, each in list order.
    let mut order: Vec<(u8, String)> = vm
        .objects
        .iter()
        .map(|(id, o)| {
            let pri = o
                .properties
                .get("pri")
                .and_then(|v| v.first())
                .and_then(|v| v.integer().ok())
                .unwrap_or(0);
            (if pri > 0 { 0 } else if pri == 0 { 1 } else { 2 }, id.clone())
        })
        .collect();
    order.sort_by_key(|(group, _)| *group);
    for (_, object_id) in &order {
        let o = vm.objects.get_mut(object_id).unwrap();
        if o.thread != vm.host_thread {
            continue;
        }
        let paused = o.thread == ThreadKind::Game && frozen_game_ms.is_some();
        let now = if paused {
            frozen_game_ms.unwrap().max(o.started)
        } else {
            host_now
        };
        let audio_level = animation_clock::wave_level(h, vm.client, o, &channels)?;
        let mut pending = BTreeMap::new();
        for (ticket, channel) in std::mem::take(&mut o.audio_tickets) {
            let status = h.audio_preparation(vm.client, ticket)?;
            match status.phase {
                AudioPreparePhase::Pending => {
                    pending.insert(ticket, channel);
                }
                AudioPreparePhase::Failed => bail!("QLIE animation audio: {:?}", status.error),
                AudioPreparePhase::Ready => h.release_audio(vm.client, ticket)?,
            }
        }
        o.audio_tickets = pending;
        if !o.created || !o.visible || o.preparing {
            continue;
        }
        if o.properties.contains_key("col") {
            let layer: serde_json::Value =
                serde_json::from_slice(&h.query_engine(vm.client, EngineQuery::Layer(o.layer))?)?;
            let multiply: [f32; 3] = layer["filter"]["multiply"]
                .as_array()
                .map(|values| {
                    std::array::from_fn(|i| {
                        values.get(i).and_then(|v| v.as_f64()).unwrap_or(1.) as f32
                    })
                })
                .unwrap_or([1.; 3]);
            o.colour = Some(multiply.map(|v| (v * 255.).clamp(0., 255.) as u8));
            if let Some(message) = o.message.as_ref().filter(|m| m.created) {
                commands.push(
                    json!({"op":"set_filter","id":message.layer,"filter":{"multiply":multiply}}),
                );
            }
        }
        if matches!(o.kind.as_str(), "btn" | "sbtn") {
            let interaction: Option<crysirror_scene::ui::WidgetInteraction> =
                serde_json::from_slice(
                    &h.query_engine(vm.client, EngineQuery::UiInteraction(o.layer))?,
                )?;
            if let Some(input) = interaction {
                let state = (input.hovered, input.pressed, input.activation_serial);
                if state != o.button_state {
                    // 6A1xxx counts clicks at button +3844 for ?bc.
                    if state.2 != o.button_state.2 {
                        let count = property_int(o, "native/bc", 0).saturating_add(1);
                        o.properties.insert("native/bc".into(), vec![Value::Int(count)]);
                    }
                    // 6D474C mode 9: a btn wait takes the click (no callback).
                    let button_wait = matches!(vm.wait, Some(Wait::Button { .. }))
                        && o.thread == vm.host_thread;
                    if button_wait && state.2 != o.button_state.2 && vm.button_clicked.is_none() {
                        vm.button_clicked = Some(object_id.clone());
                    } else if o.kind == "sbtn" && state.2 != o.button_state.2 {
                        ensure!(
                            vm.button_callbacks.len() < 256,
                            "QLIE pending button callback budget"
                        );
                        vm.button_callbacks.push_back(object_id.clone());
                    }
                    let name = if state.2 != o.button_state.2 {
                        "*OnClick"
                    } else if state.1 {
                        "*OnMouseDown"
                    } else if state.0 {
                        "*OnMouse"
                    } else {
                        "*OffMouse"
                    };
                    let family = if o.locked {
                        Some("*OnLock")
                    } else if property_int(o, "ck", 0) != 0 {
                        Some("*OnCheck")
                    } else {
                        None
                    };
                    let mut names = Vec::new();
                    if let Some(prefix) = family {
                        names.push(format!("{prefix}{}", name.trim_start_matches('*')));
                        names.push(prefix.to_owned());
                    }
                    names.push(name.to_owned());
                    if let Some(index) = o.bundle.as_ref().and_then(|b| {
                        names.iter().find_map(|name| {
                            b.animation
                                .motions
                                .iter()
                                .position(|m| m.name.eq_ignore_ascii_case(name))
                        })
                    }) {
                        o.motion = index;
                        o.started = now;
                        o.offset = 0;
                        o.events.clear();
                        o.animation_clocks.clear();
                    }
                    o.button_state = state;
                }
            }
        }
        let speed = o
            .properties
            .get("ps")
            .and_then(|v| v.first())
            .map(Value::integer)
            .transpose()?
            .unwrap_or(100)
            .max(0) as u64;
        // 5CE5C8: tmp (+D2B) advances the TABMP by 0 ahead of the ost clock.
        let hold = o.properties.contains_key("tmp");
        if hold {
            o.started = now;
        }
        let mut time = now.saturating_sub(o.started).saturating_mul(speed) / 100 + o.offset;
        // 5CE5C8: with ost enabled the timeline follows the bound sc sound
        // channel's position (5C49EC), optionally looped over [start,start+len).
        if let Some(ost) = o.properties.get("ost").filter(|_| !hold).filter(|v| v.first().is_some_and(|f| f.integer().ok() == Some(1))) {
            let start = ost.get(1).and_then(|v| v.integer().ok()).unwrap_or(0) as i64;
            let length = ost.get(2).and_then(|v| v.integer().ok()).unwrap_or(0) as i64;
            let channel = o.wave_channel.and_then(|index| channels.get(&index).copied()).unwrap_or(0);
            let position = if channel == 0 {
                0
            } else {
                h.audio_info(vm.client, channel)?
                    .map(|i| ((i.position_frames as f64 * 1000. / f64::from(i.source_rate.max(1))).round()) as i64)
                    .unwrap_or(0)
            };
            let mut t = position;
            if length > 0 {
                // Delphi mod keeps the dividend's sign; negatives clamp to 0.
                t = start + (position - start) % length;
            }
            time = t.max(0) as u64;
        }
        let parent = o.layer;
        let priority=o.position[2]as f32;
        if let Some(message) = &mut o.message {
            message.bound_parent = None;
        }
        if o.kind != "theora" {
            if let Some(id) = o.media {
                h.media_request(vm.client, MediaRequest::SetClock { id, now_ms: time })?;
            }
        }
        for state in o.draw_images.values_mut() {
            state.used = false;
        }
        //Native collision preparation is synchronous. Its worker equivalent
        //must publish a whole replacement shape, not hide the live widget
        //between a pointer-down and release while the new mask is pending.
        let previous_hit = (o.hit_bounds, o.hit_mask.clone(), o.hit_world_space);
        let image_paths=o.images.iter().filter_map(|image|image.as_ref().map(|image|image.path.relative().to_owned())).collect::<BTreeSet<_>>();
        o.hit_mask_sources.retain(|path,_|image_paths.contains(path));
        let mut collision_pending = false;
        let mut collision_regions = Vec::new();
        let mut collision_keys = Vec::new();
        let mut collision_sources = Vec::new();
        let collision_pass = o.bundle.is_some();
        if collision_pass {
            o.hit_bounds = None;
            o.hit_world_space = false;
        }
        let active = if let Some(bundle) = o.bundle.take() {
            let mut traversal = Traversal {
                host: h,
                client: vm.client,
                object: o,
                ids: &mut vm.media_ids,
                next_layer: &mut vm.next_layer,
                next_image: &mut vm.next_image,
                commands: &mut commands,
                active: 0,
                canvas: vm.config.canvas,
                random: if paused {
                    frozen_random
                        .as_mut()
                        .context("QLIE paused animation random snapshot")?
                } else {
                    vm.random
                        .as_mut()
                        .context("QLIE animation random seed not ready")?
                },
                pool: &vm.pool,
                audio_level,
                message_alpha,
                matte: None,
                collision_pending: false,
                collision_regions: Vec::new(),
                collision_keys: Vec::new(),
                collision_sources: Vec::new(),
                direct_sample: false,
                instance_scope: 0,
                paused,
                priority,
                coverage_write:false,
            };
            let result = traversal.bundle(&bundle, time, parent, traversal.object.motion, 0);
            let active = traversal.active;
            collision_pending = traversal.collision_pending;
            collision_regions = std::mem::take(&mut traversal.collision_regions);
            collision_keys = std::mem::take(&mut traversal.collision_keys);
            collision_sources = std::mem::take(&mut traversal.collision_sources);
            o.bundle = Some(bundle);
            result?;
            active
        } else {
            let spec = if o.media.is_some() {
                let info = if let Some(id) = o.media {
                    Some(h.media_info(vm.client, id)?)
                } else {
                    None
                };
                if let Some(size) = info.and_then(|s| s.size) {
                    let asset = o
                        .properties
                        .get("media_image")
                        .and_then(|v| v.first())
                        .context("QLIE movie image resource missing")?
                        .text();
                    Some(
                        json!({"parent":o.layer,"rect":[0,0,size[0],size[1]],"paint":{"kind":"image","asset":asset}}),
                    )
                } else {
                    None
                }
            } else {
                o.images.first().and_then(|i|i.as_ref()).map(|image|json!({"parent":o.layer,"rect":[0,0,image.size[0],image.size[1]],"paint":{"kind":"image","asset":image.path.relative()}}))
            };
            if let Some(spec) = spec {
                let mut traversal = Traversal {
                    host: h,
                    client: vm.client,
                    object: o,
                    ids: &mut vm.media_ids,
                    next_layer: &mut vm.next_layer,
                    next_image: &mut vm.next_image,
                    commands: &mut commands,
                    active: 0,
                    canvas: vm.config.canvas,
                    random: if paused {
                        frozen_random
                            .as_mut()
                            .context("QLIE paused animation random snapshot")?
                    } else {
                        vm.random
                            .as_mut()
                            .context("QLIE animation random seed not ready")?
                    },
                    pool: &vm.pool,
                    audio_level,
                    message_alpha,
                    matte: None,
                    collision_pending: false,
                    collision_regions: Vec::new(),
                    collision_keys: Vec::new(),
                    collision_sources: Vec::new(),
                    direct_sample: false,
                    instance_scope: 0,
                    paused,
                    priority,
                coverage_write:false,
                };
                traversal.layer(spec)?;
                1
            } else {
                0
            }
        };
        if collision_pending {
            //A native collision pass publishes one complete table. A subset
            //whose sibling source is pending cannot replace a live gesture.
            o.hit_bounds = previous_hit.0;
            o.hit_mask = previous_hit.1;
            o.hit_world_space = previous_hit.2;
        } else if collision_pass {
            o.hit_mask_job=None;
            let mask=crysirror_scene::ui::WidgetHitMask::union(&collision_regions,
                [0.,0.,vm.config.canvas[0]as f32,vm.config.canvas[1]as f32])?;
            if let Some((bounds,mask))=mask {
                o.hit_bounds=Some(bounds);o.hit_mask=Some(mask);o.hit_world_space=true;
                let key=serde_json::to_string(&collision_keys)?;
                if o.hit_mask_key.as_deref()!=Some(key.as_str()) {
                    o.hit_mask_key=Some(key);
                    let sources=serde_json::to_string(&collision_sources)?;
                    let property=if o.hit_mask_plane_key.as_deref()==Some(sources.as_str()) {
                        "prepared_hit_geometry_changed"
                    } else { "prepared_hit_mask_changed" };
                    o.hit_mask_plane_key=Some(sources);
                    o.properties.insert(property.into(),vec![Value::Int(1)]);
                }
            } else {
                o.hit_bounds=None;o.hit_mask=None;o.hit_mask_key=None;o.hit_mask_plane_key=None;
            }
        }
        let retired: Vec<_> = o
            .draw_images
            .iter()
            .filter(|(_, state)| !state.used)
            .map(|(id, _)| *id)
            .collect();
        for id in retired {
            if let Some(state) = o.draw_images.remove(&id) {
                if let Some(image) = state.image {
                    h.release_image_resource(vm.client, image.path.relative())?;
                }
            }
        }
        if let Some(message) = &o.message {
            if message.created && o.bundle.is_some() && message.bound_parent.is_none() {
                commands.push(json!({"op":"set_visible","id":message.layer,"visible":false}));
            }
        }
        for i in active..o.children.len() {
            if !o
                .published_children
                .get(i)
                .is_none_or(serde_json::Value::is_null)
            {
                commands.push(json!({"op":"set_visible","id":o.children[i],"visible":false}));
                o.published_children[i] = serde_json::Value::Null;
            }
        }
        if matches!(o.kind.as_str(), "btn" | "sbtn") {
            let Some(bounds) = o.hit_bounds else {
                if o.ui_registered {
                    ui.push(json!({"op":"set_visible","id":o.layer,"visible":false}));
                }
                continue;
            };
            let layer: serde_json::Value =
                serde_json::from_slice(&h.query_engine(vm.client, EngineQuery::Layer(o.layer))?)?;
            let x = layer["rect"][0].as_f64().context("QLIE UI position X")?;
            let y = layer["rect"][1].as_f64().context("QLIE UI position Y")?;
            let rect = [
                bounds[0] + if o.hit_world_space {0.}else{x as f32},
                bounds[1] + if o.hit_world_space {0.}else{y as f32},
                bounds[2],
                bounds[3],
            ];
            let group = o
                .properties
                .get("in")
                .and_then(|v| v.first())
                .and_then(|v| v.integer().ok())
                .unwrap_or(255);
            let enabled = o.thread == input_thread
                && !o.locked
                && group & input_masks[usize::from(o.thread == ThreadKind::System)] != 0;
            if o.ui_registered {
                ui.push(json!({"op":"set_visible","id":o.layer,"visible":o.visible}));
                ui.push(json!({"op":"set_enabled","id":o.layer,"enabled":enabled}));
                if o.ui_rect != Some(rect) {
                    ui.push(json!({"op":"set_rect","id":o.layer,"rect":rect}));
                }
            } else {
                ui.push(json!({"op":"put","widget":{"id":o.layer,"parent":widget_roots[usize::from(o.thread == ThreadKind::System)],"rect":rect,"hit_mask":o.hit_mask,"z":o.position[2],"kind":"button","visible":o.visible,"enabled":enabled,"interaction_only":true}}));
                o.ui_registered = true;
            }
            if o.properties.remove("prepared_hit_mask_changed").is_some() {
                o.properties.remove("prepared_hit_geometry_changed");
                ui.push(json!({"op":"set_hit_mask","id":o.layer,"mask":o.hit_mask}));
            } else if o.properties.remove("prepared_hit_geometry_changed").is_some() {
                if let Some(mask)=&o.hit_mask {
                    ui.push(json!({"op":"set_hit_mask_geometry","id":o.layer,"geometry":mask.geometry()}));
                }
            }
            o.ui_rect = Some(rect);
        }
    }
    if !commands.is_empty() || !ui.is_empty() {
        vm.submit(h, json!({"layers":commands,"ui":ui}))?;
    }
    Ok(())
}

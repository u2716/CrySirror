//! Native resource traversal and key/event policy. Output consists of engine
//! Layer/Ui operations and reader-backed Media/Audio requests. No pixels are
//! composed here; nested ABMPs retain their scene hierarchy.
use super::*;
use crysirror_core::service::{
    AudioParameters, AudioPreparePhase, AudioRequest, EncodedAudio, EngineQuery, MediaRequest,
    MediaSourceOptions, PcmBus,
};

struct Traversal<'a> {
    host: &'a mut dyn HostServices,
    client: ClientId,
    object: &'a mut Object,
    ids: &'a mut BTreeSet<u32>,
    next_layer: &'a mut u32,
    commands: &'a mut Vec<serde_json::Value>,
    active: usize,
    canvas: [u32; 2],
    random: &'a mut animation_clock::RandomTable,
}
impl Traversal<'_> {
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
        if let Some(blend) = self.object.properties.get("blend").and_then(|v| v.first()) {
            spec["blend"] = json!(blend.text());
        }
        let id = self.object.children[self.active];
        spec["id"] = json!(id);
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
            self.object.published_children[self.active] = spec;
        }
        self.active += 1;
        Ok(id)
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
        if !matches!(event, 8 | 11) || !track.sound_enabled {
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
        let frame = self
            .object
            .animation_clocks
            .entry(bundle.id)
            .or_default()
            .advance(
                bundle,
                motion_index,
                time,
                &values,
                &self.object.skip_flags,
                &self.object.jump_targets,
                self.random,
            )?;
        let motion_index = frame.motion;
        if depth == 0 {
            self.object.motion = motion_index;
        }
        let motion = bundle
            .animation
            .motions
            .get(motion_index)
            .with_context(|| format!("QLIE ABMP motion {motion_index}"))?;
        // Native kinds 15/16/17 reset/push/pop the draw-state stack
        // (5A6128/5A61B0/5A616C -> 567A34), rather than drawing an image.
        // State parameters accumulate before building one matrix; putting a
        // nested transform on every push would multiply translations twice.
        let mut states: Vec<(u32, [f32; 2], [f32; 2], f32, [f32; 4], [f32; 3])> = Vec::new();
        let draw_root = parent;
        let mut common: Vec<animation::Sample> = Vec::new();
        for (track_index, track) in motion.layers.iter().enumerate() {
            let (local, cycle) = frame.tracks[track_index];
            self.event(
                bundle,
                track,
                track_index,
                local,
                cycle,
                motion_index,
                &values,
            )?;
            let Some(mut sample) = animation::sample(track, local, &values)? else {
                continue;
            };
            // TABMP's common-key stack adds positions/angles and multiplies
            // scales/colour before constructing the draw matrix (5BD9DC).
            // A scene parent per common track would rotate translations twice.
            if let Some(previous) = common.last() {
                for i in 0..2 {
                    sample.position[i] += previous.position[i];
                    sample.scale[i] *= previous.scale[i] / 100.;
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
                // Native collision-only tracks are invisible in player mode.
                // Their rectangle belongs to the shared UI input registry.
                ensure!(
                    depth == 0 && sample.rotation == 0.,
                    "QLIE transformed nested hit region"
                );
                self.object.hit_bounds = Some([
                    sample.position[0] - sample.scale[0] * sample.anchor[0] / 100.,
                    sample.position[1] - sample.scale[1] * sample.anchor[1] / 100.,
                    sample.scale[0],
                    sample.scale[1],
                ]);
                continue;
            }
            if matches!(sample.kind, 15 | 16 | 17) {
                match sample.kind {
                    15 => states.clear(),
                    17 => {
                        states.pop();
                    }
                    16 => {
                        ensure!(states.len() < 64, "QLIE animation state stack budget");
                        let previous = states
                            .last()
                            .copied()
                            .unwrap_or((draw_root, [0.; 2], [1.; 2], 0., [255.; 4], [0.; 3]));
                        let position = [
                            previous.1[0] + sample.position[0],
                            previous.1[1] + sample.position[1],
                        ];
                        let scale = [
                            previous.2[0] * sample.scale[0] / 100.,
                            previous.2[1] * sample.scale[1] / 100.,
                        ];
                        let rotation = previous.3 + sample.rotation;
                        let tint = std::array::from_fn::<_, 4, _>(|i| {
                            previous.4[i] * sample.tint[i] / 255.
                        });
                        let add = std::array::from_fn::<_, 3, _>(|i| {
                            (previous.5[i] + sample.add[i]).min(1.)
                        });
                        let group=self.layer(json!({"parent":draw_root,"rect":[position[0],position[1],bundle.animation.size[0].max(1),bundle.animation.size[1].max(1)],"order":[sample.order,0],"opacity":tint[3].clamp(0.,255.) as u8,"paint":{"kind":"solid","color":[0,0,0,0]},"transform":{"scale_x":scale[0],"scale_y":scale[1],"rotate":rotation.to_radians()}}))?;
                        states.push((group, position, scale, rotation, tint, add));
                    }
                    _ => unreachable!(),
                }
                continue;
            }
            let draw_parent = states.last().map(|v| v.0).unwrap_or(draw_root);
            ensure!(
                matches!(sample.kind, 1 | 2 | 4 | 7 | 10 | 13),
                "unsupported QLIE ABMP drawing kind {}",
                sample.kind
            );
            let mut tint = sample.tint.map(|v| v.clamp(0., 255.) as u8);
            if let Some(state) = states.last() {
                for i in 0..3 {
                    tint[i] = (tint[i] as f32 * state.4[i] / 255.).clamp(0., 255.) as u8;
                }
            }
            let mut nested = None;
            let mut image_offset = [0.; 2];
            let mut virtual_size = None;
            let mut text_surface = false;
            let mut snapshot_view = false;
            let mut view_uv = serde_json::Value::Null;
            let (paint, size) = if sample.kind == 1 {
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
                for (i, value) in image_offset.iter_mut().enumerate() {
                    *value = member.metadata["offset"][i].as_i64().unwrap_or(0) as f32;
                }
                let extent = [
                    member.metadata["virtual_size"][0].as_u64().unwrap_or(0),
                    member.metadata["virtual_size"][1].as_u64().unwrap_or(0),
                ];
                if extent[0] > 0 && extent[1] > 0 {
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
                        part.split(',')
                            .next()
                            .is_some_and(|name| name.eq_ignore_ascii_case("messurface"))
                    });
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
                    text_surface = true;
                    (
                        json!({"kind":"solid","color":[0,0,0,0]}),
                        self.object.message.as_ref().unwrap().surface_size(),
                    )
                } else if let Some(child) = &member.nested {
                    nested = Some(child.as_ref());
                    (
                        json!({"kind":"solid","color":[0,0,0,0]}),
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
                    (
                        json!({"kind":"image","asset":image.path.relative(),"tint":tint}),
                        [image.size[0] as f32, image.size[1] as f32],
                    )
                }
            };
            let reference = virtual_size.unwrap_or(size);
            let pivot = [
                reference[0] * sample.anchor[0] / 100. - image_offset[0],
                reference[1] * sample.anchor[1] / 100. - image_offset[1],
            ];
            // Native kind 2 uses the ordinary image draw path, overriding
            // sampled scale to identity (5A7BA4 -> 5A4EB4).
            let scale = if sample.kind == 1 {
                sample.scale
            } else if sample.kind == 2 {
                [1., 1.]
            } else {
                [sample.scale[0] / 100., sample.scale[1] / 100.]
            };
            let add = std::array::from_fn::<_, 3, _>(|i| {
                (sample.add[i] + states.last().map(|s| s.5[i]).unwrap_or(0.)).min(1.)
            });
            let mut filter = json!({"add":add});
            if snapshot_view {
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
            let node=self.layer(json!({"parent":draw_parent,"rect":[sample.position[0]-pivot[0],sample.position[1]-pivot[1],size[0],size[1]],"order":[sample.order,0],"opacity":if snapshot_view {tint[3]}else{255},"paint":paint,"uv":view_uv,"filter":filter,"transform":{"scale_x":scale[0],"scale_y":scale[1],"pivot_x":pivot[0],"pivot_y":pivot[1],"rotate":sample.rotation.to_radians()},"blend":sample.blend}))?;
            if text_surface {
                let message = self.object.message.as_mut().unwrap();
                ensure!(
                    message.bound_parent.is_none(),
                    "QLIE multiple live text-surface views require shared glyph-view publication"
                );
                message.bound_parent = Some(node);
                if message.created {
                    self.commands
                        .push(json!({"op":"set_parent","id":message.layer,"parent":node}));
                    self.commands
                        .push(json!({"op":"set_visible","id":message.layer,"visible":true}));
                }
            }
            if let Some(child) = nested {
                self.bundle(child, sample.seek, node, sample.nested_motion, depth + 1)?;
            }
        }
        Ok(())
    }
}
pub(super) fn update(vm: &mut QlieVm, h: &mut dyn HostServices) -> Result<()> {
    let now = h.now_ms();
    let mut commands = Vec::new();
    let mut ui = Vec::new();
    for (object_id, o) in &mut vm.objects {
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
                    if o.kind == "sbtn" && state.2 != o.button_state.2 {
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
                    if let Some(index) = o
                        .bundle
                        .as_ref()
                        .and_then(|b| b.animation.motions.iter().position(|m| m.name == name))
                    {
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
        let time = now.saturating_sub(o.started).saturating_mul(speed) / 100 + o.offset;
        let parent = o.layer;
        if let Some(message) = &mut o.message {
            message.bound_parent = None;
        }
        if o.kind != "theora" {
            if let Some(id) = o.media {
                h.media_request(vm.client, MediaRequest::SetClock { id, now_ms: time })?;
            }
        }
        let active = if let Some(bundle) = o.bundle.take() {
            let mut traversal = Traversal {
                host: h,
                client: vm.client,
                object: o,
                ids: &mut vm.media_ids,
                next_layer: &mut vm.next_layer,
                commands: &mut commands,
                active: 0,
                canvas: vm.config.canvas,
                random: vm
                    .random
                    .as_mut()
                    .context("QLIE animation random seed not ready")?,
            };
            let result = traversal.bundle(&bundle, time, parent, traversal.object.motion, 0);
            let active = traversal.active;
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
                    commands: &mut commands,
                    active: 0,
                    canvas: vm.config.canvas,
                    random: vm
                        .random
                        .as_mut()
                        .context("QLIE animation random seed not ready")?,
                };
                traversal.layer(spec)?;
                1
            } else {
                0
            }
        };
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
            let bounds = o
                .hit_bounds
                .context("QLIE button has no native collision rectangle")?;
            let layer: serde_json::Value =
                serde_json::from_slice(&h.query_engine(vm.client, EngineQuery::Layer(o.layer))?)?;
            let x = layer["rect"][0].as_f64().context("QLIE UI position X")?;
            let y = layer["rect"][1].as_f64().context("QLIE UI position Y")?;
            let rect = [
                x as f32 + bounds[0],
                y as f32 + bounds[1],
                bounds[2],
                bounds[3],
            ];
            if o.ui_registered {
                if o.ui_rect != Some(rect) {
                    ui.push(json!({"op":"set_rect","id":o.layer,"rect":rect}));
                }
            } else {
                ui.push(json!({"op":"put","widget":{"id":o.layer,"rect":rect,"z":o.position[2],"kind":"button","visible":o.visible,"enabled":!o.locked,"interaction_only":true}}));
                o.ui_registered = true;
            }
            o.ui_rect = Some(rect);
        }
    }
    if !commands.is_empty() || !ui.is_empty() {
        vm.submit(h, json!({"layers":commands,"ui":ui}))?;
    }
    Ok(())
}

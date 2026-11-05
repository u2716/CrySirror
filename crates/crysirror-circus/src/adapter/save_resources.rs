//!425640 static-resource reconstruction through ordinary shared Layer/image
//! services. Unsupported original state fails during preflight, before publish.
use super::*;
use crate::{
    machine::save_ops::SaveReply,
    native_save::{RestoreOrigin, RestorePlan},
};

pub(super) struct RestoreResources {
    plan: RestorePlan,
    cursor: usize,
    images: BTreeMap<u8, UploadedImage>,
    extents: BTreeMap<u8, [u32; 2]>,
    publish_wait: bool,
    queues_done: bool,
    audio_cursor: usize,
    audio_receipts: Vec<(u32, String, bool)>,
    music_receipt: Option<crate::machine::MusicAsset>,
    packed_cursor: usize,
    comments_done: bool,
    effects_cursor: u8,
    fade_prepared: bool,
    fade_job: Option<crysirror_core::image_processing::ImageJob>,
    fade_image: Option<UploadedImage>,
}
fn f(record: &[u8], object: usize) -> f32 {
    f32::from_bits(u32::from_le_bytes(
        record[object - 8..object - 4].try_into().unwrap(),
    ))
}
fn u(record: &[u8], object: usize) -> u32 {
    u32::from_le_bytes(record[object - 8..object - 4].try_into().unwrap())
}
fn preflight(plan: &RestorePlan) -> Result<()> {
    for image in &plan.images {
        if image.packed || image.native_ui {
            continue;
        }
        image_path(&image.name)?;
        if let Some(index) = image.native_sprite {
            let record = plan.body.sprite(index)?;
            ensure!((u(record.bytes(), 960)) <= 4, "Circus restored shader mode");
        }
    }
    Ok(())
}
fn owned_removal_roots(
    host: &mut dyn HostServices,
    client: ClientId,
    candidates: BTreeSet<u32>,
    ui: bool,
) -> Result<Vec<u32>> {
    let mut parents = BTreeMap::new();
    let mut existing = BTreeSet::new();
    for id in candidates {
        let query = if ui {
            EngineQuery::UiWidget(id)
        } else {
            EngineQuery::Layer(id)
        };
        let node: Value = serde_json::from_slice(&host.query_engine(client, query)?)?;
        if !node.is_null() {
            existing.insert(id);
            parents.insert(id, node["parent"].as_u64().map(u32::try_from).transpose()?);
        }
    }
    let mut roots = Vec::new();
    for id in &existing {
        let mut parent = parents[id];
        let mut redundant = false;
        let mut visited = BTreeSet::new();
        while let Some(p) = parent {
            ensure!(
                visited.insert(p) && visited.len() <= 256,
                "Circus restore retained hierarchy invalid"
            );
            if existing.contains(&p) {
                redundant = true;
                break;
            }
            if let Some(known) = parents.get(&p) {
                parent = *known;
                continue;
            }
            let query = if ui {
                EngineQuery::UiWidget(p)
            } else {
                EngineQuery::Layer(p)
            };
            let node: Value = serde_json::from_slice(&host.query_engine(client, query)?)?;
            parent = node["parent"].as_u64().map(u32::try_from).transpose()?;
            parents.insert(p, parent);
        }
        if !redundant {
            roots.push(*id);
        }
    }
    Ok(roots)
}
impl Bridge<'_> {
    pub(super) fn restore_native_resources(
        &mut self,
        plan: &RestorePlan,
    ) -> Result<Option<SaveReply>> {
        if self.services.saves.restore.is_none() {
            preflight(plan)?;
            //425640 reloads only0..36. System37..44 retain the current
            //leases even when the restored filename bytes name older assets.
            if !self.effects_ready()? {
                return Ok(None);
            }
            if !matches!(
                plan.origin,
                RestoreOrigin::Rollback {
                    preserve_effects: true
                }
            ) {
                let mut commands = Vec::new();
                for (object, state) in self.primary_sprite_states()? {
                    if object < 150 {
                        commands.push(crate::machine::sprite_ops::SpriteCommand {
                            seed: crate::machine::sprite_ops::SpriteSeed {
                                object,
                                values: state.values.clone(),
                                retained: state.retained.clone(),
                            },
                            operations: vec![
                                crysirror_scene::layer::queued::SequenceOperation::Finish {
                                    cancel: false,
                                    lanes: None,
                                },
                                crysirror_scene::layer::queued::SequenceOperation::Capture {
                                    enabled: state.capture,
                                    repeats: 0,
                                    forever: state.forever,
                                },
                            ],
                        });
                    }
                }
                self.native_sprite(&crate::machine::sprite_ops::SpriteRequest { commands }).context("Circus425640 finalize current primary controls")?;
                self.native_effect_control(&crate::machine::effect_ops::EffectRequest::Finish)?;
            }
            self.services.saves.restore = Some(RestoreResources {
                plan: plan.clone(),
                cursor: 0,
                images: BTreeMap::new(),
                extents: BTreeMap::new(),
                publish_wait: false,
                queues_done: false,
                audio_cursor: 0,
                audio_receipts: Vec::new(),
                music_receipt: None,
                packed_cursor: 0,
                comments_done: false,
                effects_cursor: 0,
                fade_prepared: false,
                fade_job: None,
                fade_image: None,
            });
        }
        let restore = self.services.saves.restore.as_ref().unwrap();
        ensure!(
            restore.plan == *plan,
            "Circus restored resource continuation ownership mismatch"
        );
        if self.services.saves.restore.as_ref().unwrap().publish_wait {
            return self.finish_native_resource_restore(plan);
        }
        if !self.services.saves.restore.as_ref().unwrap().fade_prepared {
            if plan.restore_fade_ticks.is_some() {
                if self
                    .services
                    .saves
                    .restore
                    .as_ref()
                    .unwrap()
                    .fade_job
                    .is_none()
                {
                    let job = self.native_restore_fade_capture().context("Circus425640 restore-fade selected-canvas capture")?;
                    if job.is_none() {
                        self.services.saves.restore.as_mut().unwrap().fade_prepared = true;
                    } else {
                        self.services.saves.restore.as_mut().unwrap().fade_job = job;
                        return Ok(None);
                    }
                }
                if let Some(job) = self
                    .services
                    .saves
                    .restore
                    .as_mut()
                    .unwrap()
                    .fade_job
                    .as_mut()
                {
                    use crysirror_core::image_processing::ImageProcessState;
                    let image = match job.poll()? {
                        ImageProcessState::Pending => return Ok(None),
                        ImageProcessState::Failed(error) => {
                            bail!("Circus restore fade capture failed: {error}")
                        }
                        ImageProcessState::Ready(image) => image,
                    };
                    ensure!(
                        image.client() == self.client && image.size() == [1280, 720],
                        "Circus restore fade image identity"
                    );
                    let image =
                        self.host
                            .publish_image_resource(self.client, 0x3509_0001, image)?;
                    let restore = self.services.saves.restore.as_mut().unwrap();
                    restore.fade_image = Some(image);
                    restore.fade_job = None;
                    restore.fade_prepared = true;
                }
            } else {
                self.services.saves.restore.as_mut().unwrap().fade_prepared = true;
            }
        }

        let cursor = self.services.saves.restore.as_ref().unwrap().cursor;
        if let Some(image) = plan.images.get(cursor) {
            if image.packed || image.native_ui {
                self.services.saves.restore.as_mut().unwrap().cursor += 1;
                return Ok(None);
            }
            let path=image_path(&image.name)?;
            if image.preloaded_archive_bitmap_reload() {
                let storage=self.services.storage.as_ref().context("Circus restore image storage owner")?;
                let present=storage.metadata(Mount::Asset,path.relative()).with_context(||format!("Circus425640 bitmap probe slot={} path={}",image.slot,path.relative()))?.is_some();
                if !present {
                    //425640 ->422CF0 ->41F1B0 releases this archive slot,
                    //then44C940/44FBA0 returns native null for its absent
                    //bitmap. The saved filename remains; no drawable exists
                    //and41E6A0(33..36) has no renderer. This precise archive
                    //case is not a general missing named-image fallback.
                    self.native_resource(&crate::machine::resource_ops::ResourceRequest::Unload{slot:image.slot})?.context("Circus425640 archive bitmap unload receipt")?;
                    self.services.packed.release_slot(image.slot);
                    self.services.saves.restore.as_mut().unwrap().cursor+=1;
                    return Ok(None)
                }
            }
            let Some(prepared) = self
                .services
                .prepare(Work::Image(image.name.clone()), self.client)
                .with_context(||format!("Circus425640 image reload slot={} path={}",image.slot,path.relative()))?
            else {
                return Ok(None);
            };
            let Prepared::Image(surface, logical) = prepared else {
                bail!("Circus restored image worker receipt mismatch")
            };
            let path = self.host.upload_rgba(
                self.client,
                0x36000000 + u32::from(image.slot),
                surface.width,
                surface.height,
                &surface.rgba,
            )?;
            let resource = UploadedImage {
                path,
                size: [surface.width, surface.height],
                opaque: surface.rgba.chunks_exact(4).all(|p| p[3] == 255),
            };
            let restore = self.services.saves.restore.as_mut().unwrap();
            restore.images.insert(image.slot, resource);
            restore.extents.insert(image.slot, logical);
            restore.cursor += 1;
            // GPU/resource publication belongs to the end-of-tick owner fence.
            return Ok(None);
        }
        // All dependencies and source fields are validated before the single
        // visible Layer/UI batch. Earlier uploads are immutable hidden leases.
        let restore = self.services.saves.restore.as_ref().unwrap();
        let mut remove = BTreeSet::from([0x36fffffe]);
        remove.extend(
            self.services
                .displayed
                .iter()
                .map(|slot| 0x37000000 + u32::from(*slot)),
        );
        remove.extend(
            self.services
                .layer_ids
                .iter()
                .filter(|(group, _)| !matches!(**group, 0 | 1))
                .flat_map(|(_, ids)| ids.iter().copied()),
        );
        let mut layers = owned_removal_roots(self.host, self.client, remove, false)?
            .into_iter()
            .map(|id| json!({"op":"remove","id":id}))
            .collect::<Vec<_>>();
        layers.push(json!({"op":"put","layer":{"id":SCENE_ROOT,"rect":[0,0,1280,720],"paint":{"kind":"solid","color":[0,0,0,255]},"order":[0,0]}}));
        for image in plan
            .images
            .iter()
            .filter(|image| !image.packed && !image.native_ui && image.native_sprite.is_some())
        {
            let native = image.native_sprite.unwrap();
            let sprite = plan.body.sprite(native)?;
            let record = sprite.bytes();
            let resource = &restore.images[&image.slot];
            let position = sprite.position();
            let percent = sprite.scale_percent();
            let source = [f(record,96),f(record,100),f(record,104),f(record,108)];
            let quad=crysirror_scene::layer::crop::texel_center_quad(resource.size,[f(record,72),f(record,76)],source,sprite.pivot())?;
            //447D20 restores the exact numerical controls afterwards. Finite
            //transport placeholders are never native values: this leaf stays
            //unpublished until the same owner installs its CPU track and the
            //engine's undefined-binding suppression takes over.
            let pos=position.map(|value|if value.is_finite(){value}else{0.});
            let pivot=quad.map(|quad|quad.pivot).unwrap_or([0.,0.]);
            let scale=percent.map(|value|if value.is_finite(){value / if percent.iter().all(|value|*value>=1000.){10000.}else{100.}}else{1.});
            let size=quad.map(|quad|quad.size).unwrap_or([1.,1.]);
            let uv=quad.map(|quad|quad.uv);
            let offset=[f(record,20),f(record,24)];
            ensure!(offset.iter().all(|value|value.is_finite()),"Circus restored texture offset");
            let offset_matrix=if offset==[0.,0.]{None}else{Some([1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.,offset[0],offset[1],0.,1.])};
            let rotation=f(record,144);let rotation=if rotation.is_finite(){rotation.to_radians()}else{0.};
            let alpha=f(record,112);let opacity=if alpha.is_finite(){(alpha.clamp(0.,1.)*255.) as u8}else{0};
            let order = if native < 10 {
                [0, native as i32]
            } else if native < 64 {
                [1, u(record, 904) as i32 * 64 + native as i32]
            } else {
                [2, native as i32]
            };
            let mode = u(record, 960) as u8;
            let frequency = if mode == 4 {
                let offset=f(record,116);
                if !offset.is_finite(){0.}else{1./offset}
            } else {
                0.
            };
            let filter = super::visual::native_filter(mode, frequency)?;
            let object = super::visual::object_layer(native as u16);
            let content = super::visual::content_layer(native as u16);
            layers.push(json!({"op":"put","layer":{"id":object,"parent":SCENE_ROOT,"opacity":match native{88=>if self.services.native_status_visible{255}else{0},75|76=>if self.services.native_window_visible{255}else{0},_=>255},"visible":self.native_primary_visible(native as u16,true),"rect":[pos[0],pos[1],1280,720],"paint":{"kind":"solid","color":[0,0,0,0]},"order":order,"transform":{"scale_x":scale[0],"scale_y":scale[1],"pivot_x":pivot[0],"pivot_y":pivot[1],"rotate":rotation,"local_matrix":offset_matrix}}}));
            layers.push(json!({"op":"put","layer":{"id":content,"parent":object,"rect":[0,0,1280,720],"opacity":opacity,"paint":{"kind":"solid","color":[0,0,0,0]},"order":[0,0]}}));
            layers.push(json!({"op":"put","layer":{"id":0x37000000+u32::from(image.slot),"parent":content,"rect":[0,0,size[0],size[1]],"paint":{"kind":"image","asset":resource.path.relative()},"filter":filter,"sampling":{"address_u":"repeat","address_v":"repeat"},"uv":uv,"visible":false,"order":[0,0]}}));
        }
        let mut ui_ids = self
            .services
            .ui_ids
            .iter()
            .filter(|(group, _)| !matches!(**group, 0 | 1))
            .flat_map(|(_, ids)| ids.iter())
            .copied()
            .collect::<BTreeSet<_>>();
        if self.services.menu.is_some() {
            ui_ids.insert(0x36fffffdu32);
        }
        let ui = owned_removal_roots(self.host, self.client, ui_ids, true)?
            .into_iter()
            .into_iter()
            .map(|id| json!({"op":"remove","id":id}))
            .collect();
        if plan.origin == RestoreOrigin::Slot {
            self.host.submit_audio(
                self.client,
                AudioRequest::Stop {
                    channel: 0,
                    fade_ms: 0.,
                },
            )?;
        }
        self.submit(layers, ui).context("Circus425640 restored Layer/UI publication")?;
        let restore = self.services.saves.restore.as_mut().unwrap();
        self.services
            .images
            .retain(|slot, _| (37..=44).contains(slot));
        self.services.images.extend(restore.images.clone());
        self.services
            .image_extents
            .retain(|slot, _| (37..=44).contains(slot));
        self.services.image_extents.extend(restore.extents.clone());
        self.services
            .image_names
            .retain(|slot, _| (37..=44).contains(slot));
        self.services
            .image_names
            .extend(plan.images.iter().map(|i| (i.slot, i.name.clone())));
        self.services.displayed = plan
            .images
            .iter()
            .filter(|image| image.native_sprite.is_some() && !image.packed && !image.native_ui)
            .map(|i| i.slot)
            .collect();
        self.services.positions.clear();
        self.services.sprite_scales.clear();
        for image in plan
            .images
            .iter()
            .filter(|image| !image.packed && !image.native_ui && image.native_sprite.is_some())
        {
            let sprite = plan.body.sprite(image.native_sprite.unwrap())?;
            self.services
                .positions
                .insert(image.slot, sprite.position().map(|v| v as i16));
            self.services.sprite_scales.insert(
                image.slot,
                (
                    sprite.scale_percent()[0] as i16,
                    sprite.pivot().map(|v| v as i16),
                ),
            );
        }
        self.services.visual.mark_restore_roots(
            plan.images
                .iter()
                .filter(|image| !image.packed)
                .filter_map(|image| image.native_sprite.map(|object| object as u16)),
        );
        for image in plan
            .images
            .iter()
            .filter(|image| !image.packed && image.native_sprite.is_some())
        {
            let sprite = plan.body.sprite(image.native_sprite.unwrap())?;
            let raw = sprite.bytes();
            let mode = u(raw, 960) as u8;
            self.services.visual.filters.insert(
                image.native_sprite.unwrap() as u16,
                super::visual::native_filter(mode, if mode == 4 { 1. / f(raw, 116) } else { 0. })?,
            );
        }
        self.services.menu = None;
        self.services.menu_images.clear();
        self.services.menu_image_job = None;
        self.services
            .ui_ids
            .retain(|group, _| matches!(*group, 0 | 1));
        self.services
            .layer_ids
            .retain(|group, _| matches!(*group, 0 | 1));
        self.services.fit_background = plan.body.native_word(66517*4)?!=0;
        self.services.scene_root = true;
        restore.publish_wait = true;
        Ok(None)
    }
    ///4219A0(1): retain each current endpoint independently, finish/reset,
    ///bind resource27 and declare the two source looping position banks.
    pub(super) fn restore_scrolling_pair(&mut self) -> Result<()> {
        self.native_scrolling_pair(true)
    }
    pub(super) fn falling_scene_special(&mut self, kind: i16, parameter: i16) -> Result<bool> {
        ensure!(matches!(kind, 7 | 16), "Circus special scene effect kind");
        if parameter == 0 {
            return Ok(self.native_resource(&crate::machine::resource_ops::ResourceRequest::Unload {
                slot: if kind == 7 { 26 } else { 27 },
            })?.is_some());
        }
        if kind == 16 { self.native_scrolling_pair(false)?; }
        Ok(true)
    }
    fn native_scrolling_pair(&mut self, preserve: bool) -> Result<()> {
        use crate::machine::sprite_ops::{SpriteCommand, SpriteRequest, SpriteResourceBinding, SpriteResourceRequest};
        use crysirror_scene::layer::queued::{DeltaSource, SequenceOperation, SequenceTrack};
        let states = self.primary_sprite_states()?;
        let positions = [64u16, 65].into_iter().map(|object| {
            let state = states.iter().find(|(id, _)| *id == object).map(|(_, state)| state);
            (object, if preserve { state.map_or([0., 0.], |state| [state.values[0], state.values[1]]) } else { [0., 0.] })
        }).collect::<Vec<_>>();
        let mut reset = Vec::new();
        for (object, _) in &positions {
            let seed = crate::native_live::NativeLiveState::sprite_reset_seed(*object)?;
            reset.push(SpriteCommand { seed: seed.clone(), operations: vec![
                SequenceOperation::Finish { cancel: false, lanes: None },
                SequenceOperation::Reset { values: seed.values, retained: seed.retained },
                SequenceOperation::Capture {
                    enabled: states.iter().find(|(id, _)| id == object).is_some_and(|(_, state)| state.capture),
                    repeats: 0, forever: false,
                },
            ] });
        }
        self.native_sprite(&SpriteRequest { commands: reset })?;
        self.native_sprite_resources(&SpriteResourceRequest { bindings: vec![
            SpriteResourceBinding { slot: 27, object: 64, crop: Some([0, 0, 3072, 576]) },
            SpriteResourceBinding { slot: 27, object: 65, crop: Some([0, 600, 3072, 1176]) },
        ] })?.context("Circus scrolling resource binding receipt")?;
        let mut commands = Vec::new();
        for (object, position) in positions {
            let seed = crate::native_live::NativeLiveState::sprite_reset_seed(object)?;
            commands.push(SpriteCommand { seed, operations: vec![
                SequenceOperation::Set { channels: vec![0, 1, 12, 13, 14, 15], values: vec![0., 0., 0., 0., 3072., 576.], retained: true },
                SequenceOperation::Enqueue { lane: 0, bank: None, track: SequenceTrack::Linear {
                    channels: vec![0, 1], target: vec![-1792., 0.], ticks: if object == 64 { 7200 } else { 3600 },
                    velocity_divisor: None, ieee_inputs: false,
                    references: vec![0, 1], loop_origins: vec![0, 1], delta: DeltaSource::Retained,
                    looping: true, start: None, complete_first: false, complete_zero: true,
                    fast: true, fast_channels: None, skip_zero: true, capture_zero: true,
                } },
                SequenceOperation::Set { channels: vec![0, 1], values: position.to_vec(), retained: true },
            ] });
        }
        self.native_sprite(&SpriteRequest { commands })?;
        Ok(())
    }
    fn finish_native_resource_restore(&mut self, plan: &RestorePlan) -> Result<Option<SaveReply>> {
        use crate::machine::{
            media_ops::AudioDeclaration,
            packed_ops::PackedRequest,
            sprite_ops::{SpriteCommand, SpriteRequest, SpriteSeed},
        };
        use crysirror_scene::layer::queued::{SequenceOperation, SequenceStatus};
        if !self.services.saves.restore.as_ref().unwrap().queues_done {
            let mut commands = Vec::new();
            let mut auxiliary = Vec::new();
            for image in plan
                .images
                .iter()
                .filter(|image| image.native_sprite.is_some())
            {
                let object = image.native_sprite.unwrap() as u16;
                let raw = plan.body.sprite(object as usize)?.bytes().to_vec();
                self.visual_root(object)?;
                self.native_visual(&crate::machine::visual_ops::VisualRequest::Priority {
                    object,
                    priority: i16::try_from(u(&raw, 904))?,
                })?;
                let current: Option<SequenceStatus> =
                    serde_json::from_slice(&self.host.query_engine(
                        self.client,
                        EngineQuery::LayerSequence(0x3a04_0000 + u32::from(object)),
                    ).with_context(||format!("Circus425640 primary control query object={object}"))?)
                    .with_context(||format!("Circus425640 primary control status decode object={object}"))?;
                let blank = SequenceStatus {
                    generation: 0,
                    revision: 0,
                    values: vec![0.; 17],
                    retained: vec![0.; 17],
                    lanes: BTreeMap::new(),
                    capture: false,
                    captured: BTreeMap::new(),
                    repeats: 0,
                    forever: false,
                    complete: true,
                };
                let state =
                    crate::native_control::from_record(&raw, current.as_ref().unwrap_or(&blank))
                    .with_context(||format!("Circus425640 source960 control decode object={object}"))?;
                commands.push(SpriteCommand {
                    seed: SpriteSeed {
                        object,
                        values: state.values.clone(),
                        retained: state.retained.clone(),
                    },
                    operations: vec![SequenceOperation::RestoreControl {
                        state,
                        seed_missing: vec![0, 1, 2, 3, 4, 6, 7, 8, 10],
                    }],
                });
                auxiliary.push((object, raw));
            }
            self.native_sprite(&SpriteRequest { commands }).context("Circus425640 publish primary RestoreControl declarations")?;
            //444080 offsets are added after the primary rotation/scale and
            //before auxiliary world-pivot scaling (4432E0). Refresh the view
            //baseline too; a previous scene's matrix is not this slot's owner.
            self.restore_stage_objects(&auxiliary).context("Circus425640 restore auxiliary view controls")?;
            let mut publication=Vec::new();
            for image in plan.images.iter().filter(|image|!image.packed&&!image.native_ui&&image.native_sprite.is_some()) {
                let record=plan.body.sprite(image.native_sprite.unwrap())?;
                let raw=record.bytes();let resource=&self.services.saves.restore.as_ref().unwrap().images[&image.slot];
                let quad=crysirror_scene::layer::crop::texel_center_quad(resource.size,[f(raw,72),f(raw,76)],[f(raw,96),f(raw,100),f(raw,104),f(raw,108)],record.pivot())?;
                publication.push(json!({"op":"set_visible","id":0x37000000u32+u32::from(image.slot),"visible":quad.is_some()}));
            }
            self.submit(publication,vec![])?;
            self.native_text(&crate::machine::text_ops::TextRequest::ClearDialogue)?;
            self.services.saves.restore.as_mut().unwrap().queues_done = true;
        }
        let packed = plan
            .images
            .iter()
            .filter(|image| image.packed)
            .collect::<Vec<_>>();
        let index = self.services.saves.restore.as_ref().unwrap().packed_cursor;
        if let Some(image) = packed.get(index) {
            let name = image
                .name
                .get(3..)
                .context("Circus restored pack basename")?;
            if !self.native_packed(&PackedRequest::Load {
                slot: 33,
                name: name.to_owned(),
            }).with_context(||format!("Circus425640 packed reload slot={} path=Advdata/GRP/{}",image.slot,name.replace('\\',"/")))? {
                return Ok(None);
            }
            let object = image
                .native_sprite
                .context("Circus packed restore object")? as u16;
            let native_sprite = plan.body.sprite(object as usize)?;
            let record = native_sprite.bytes();
            if !self.restore_packed_image(33, object, record)? {
                return Ok(None);
            }
            self.services.saves.restore.as_mut().unwrap().packed_cursor += 1;
            return Ok(None);
        }
        let index = self.services.saves.restore.as_ref().unwrap().audio_cursor;
        if let Some(audio) = plan.audio.get(index) {
            if audio.kind == 0 {
                if self.services.music_loops.is_none() {
                    let Some(Prepared::MusicLoops(records)) =
                        self.services.prepare(Work::MusicLoops, self.client)?
                    else {
                        return Ok(None);
                    };
                    self.services.sound_prefixes = Some(records.sfx_prefixes);
                    self.services.music_loops = Some(records.loops);
                }
                let mut name = audio
                    .name
                    .rsplit(['\\', '/'])
                    .next()
                    .unwrap_or(&audio.name)
                    .to_ascii_uppercase();
                if !name.ends_with(".PCM") {
                    name.push_str(".PCM");
                }
                let bounds = self
                    .services
                    .music_loops
                    .as_ref()
                    .unwrap()
                    .get(&name)
                    .copied()
                    .unwrap_or([0, 0]);
                if !self.play_audio_policy(
                    0,
                    &audio.name,
                    false,
                    true,
                    bounds[0],
                    (bounds[1] != 0).then_some(bounds[1]),
                    false,
                ).with_context(||format!("Circus425640 music reload path=Advdata/{}.PCM",audio.name.replace('\\',"/")))? {
                    return Ok(None);
                }
                self.services.saves.restore.as_mut().unwrap().music_receipt = Some(crate::machine::MusicAsset {
                    name: audio.name.clone(), loop_start: bounds[0],
                    loop_end: (bounds[1] != 0).then_some(bounds[1]),
                });
            } else {
                let Some(publication) = self.publish_native_sound(&AudioDeclaration {
                    name: audio.name.clone(),
                    voice: audio.kind == 2,
                    looping: audio.looping,
                    voice_volume: audio.voice_volume,
                }).with_context(||format!("Circus425640 sound reload kind={} name={}",audio.kind,audio.name))?
                else {
                    return Ok(None);
                };
                if audio.kind == 1 {
                    for channel in publication.channels {
                        self.services
                            .saves
                            .restore
                            .as_mut()
                            .unwrap()
                            .audio_receipts
                            .push((channel, audio.name.clone(), audio.looping));
                    }
                }
            }
            self.services.saves.restore.as_mut().unwrap().audio_cursor += 1;
            return Ok(None);
        }
        //425640 reinitializes the five shared pools in this exact order.
        //The rollback preserve flag suppresses them, but never4219A0's pair.
        let preserve = matches!(plan.origin, RestoreOrigin::Rollback { preserve_effects: true });
        loop {
            let cursor = self.services.saves.restore.as_ref().unwrap().effects_cursor;
            let declaration = match cursor {
                0 if !preserve => Some((i16::try_from(plan.falling[0] as i32)?, i16::try_from(plan.falling[1] as i32)?)),
                1 if !preserve && plan.falling[5] != 0 => Some((15, i16::try_from(plan.falling[5] as i32)?)),
                2 if !preserve && plan.falling[3] != 0 => Some((13, i16::try_from(plan.falling[3] as i32)?)),
                3 if !preserve && plan.falling[2] != 0 => Some((2, i16::try_from(plan.falling[2] as i32)?)),
                _ => None,
            };
            if let Some((kind, parameter)) = declaration {
                if !self.falling_effect(kind, parameter)? { return Ok(None); }
            }
            if cursor == 4 {
                if plan.falling[4] != 0 { self.restore_scrolling_pair()?; }
                self.services.saves.restore.as_mut().unwrap().effects_cursor = 5;
            }
            if cursor >= 4 { break; }
            self.services.saves.restore.as_mut().unwrap().effects_cursor += 1;
        }
        if !self.services.saves.restore.as_ref().unwrap().comments_done {
            if !self.restore_comment_record(&plan.movie,&plan.body.bytes()[266340-crate::native_save::LIVE_BASE..266340-crate::native_save::LIVE_BASE+16400])? {
                return Ok(None);
            }
            self.services.saves.restore.as_mut().unwrap().comments_done = true;
        }
        let restore = self.services.saves.restore.take().unwrap();
        if let (Some(image), Some(ticks)) = (restore.fade_image, plan.restore_fade_ticks) {
            self.begin_restore_snapshot_fade(image, ticks)?;
        }
        Ok(Some(SaveReply::RestoredMedia { audio: restore.audio_receipts, music: restore.music_receipt }))
    }
}

// SPDX-License-Identifier: MPL-2.0
//!Detached4561A0 reconstruction. Parsing/resource work never mutates the live
//!VM, starts playback or writes storage. Runtime alone commits the ready plan.
use super::*;
use crate::native_storage::slot::{Slot, SlotSprite};
use crysirror_core::{
    native_restore::{
        OriginalRestoreBatch, OriginalRestoreCommit, OriginalRestorePreparation,
        OriginalRestoreProgress,
    },
    service::{AudioReplacementJob, AudioReplacementResource, AudioReplacementState},
};
struct Parsed {
    slot: Slot,
    machine: Machine,
    native: (crate::text::Execution, Option<String>),
}
pub(super) struct Restore {
    client: ClientId,
    bytes: Arc<[u8]>,
    template: Option<Machine>,
    pool: TaskPool,
    worker: Option<Task<Parsed>>,
    vm: Option<SugarPotVm>,
    slot: Option<Slot>,
    native: Option<(crate::text::Execution, Option<String>)>,
    raster_plan: Option<stage::StagePlan>,
    raster_picture: Option<Arc<dyn crysirror_core::image_processing::ImageResource>>,
    effects_ready: bool,
    effects_kind: u8,
    filter_seed: Option<filter::Seed>,
    filter_seeded: bool,
    filter_draw: bool,
    filter_picture: Option<Arc<dyn crysirror_core::image_processing::ImageResource>>,
    fall_seeded: bool,
    fall_draw: bool,
    fall_picture: Option<Arc<dyn crysirror_core::image_processing::ImageResource>>,
    sprite: usize,
    window: usize,
    button: usize,
    audio_index: usize,
    audio_requests: Vec<AudioRequest>,
    audio_job: Option<AudioReplacementJob>,
    audio: Option<Arc<dyn AudioReplacementResource>>,
    rendered: bool,
    ready: bool,
    failed: Option<String>,
    old_layers: Vec<u32>,
    old_ui: Vec<u32>,
    release_images: Vec<String>,
}
impl SugarPotVm {
    pub(super) fn prepare_original_restore(&self, bytes: Arc<[u8]>) -> Result<Restore> {
        ensure!(
            self.machine.is_some(),
            "SugarPot original restore before native boot"
        );
        ensure!(
            !matches!(
                self.wait,
                Some(
                    Wait::EndMode
                        | Wait::Backlog
                        | Wait::BgmMode
                        | Wait::Gallery
                        | Wait::Recollection
                        | Wait::Settings
                        | Wait::Movie
                        | Wait::Dialog
                )
            ),
            "SugarPot original restore from a disabled native modal context"
        );
        let mut vm = Self::new(self.client, self.config.clone())?;
        vm.pool = self.pool.clone();
        vm.settings = self.settings.clone();
        vm.globals = self.globals.clone();
        vm.read_flags = self.read_flags.clone();
        vm.font_size = self.font_size;
        vm.font_color = self.font_color;
        vm.font_style = self.font_style;
        vm.title = self.title.clone();
        //4561A0 does not serialize/reset62D8 or non-mode0 gate switches.
        //44BBB0 may replace the caption while rebuilding the saved node.
        vm.caption = self.caption.clone();
        vm.save_enabled = self.save_enabled;
        vm.load_enabled = self.load_enabled;
        vm.skip_enabled = self.skip_enabled;
        vm.ud_wave_phase = self.ud_wave_phase;
        vm.ud_scan_counter = self.ud_scan_counter;
        vm.ud_progressive_counter = self.ud_progressive_counter;
        vm.native_effect_next = self.native_effect_next;
        vm.native_effect_index = self.native_effect_index;
        vm.raster = self.raster.as_ref().map(raster::Raster::detached_seed);
        vm.fall = self.fall.as_ref().map(fall::Fall::detached_seed);
        vm.backlog = self.backlog.clone();
        vm.staged_batches = Some(vec![]);
        let mut old_layers = self.published_layers.clone();
        old_layers.extend(
            self.gauges
                .iter()
                .enumerate()
                .filter(|(_, g)| {
                    g.fields[6] != 0
                        && self
                            .gauges
                            .first()
                            .is_some_and(|first| first.fields[7] == 0)
                })
                .map(|(id, _)| 45000 + id as u32),
        );
        old_layers.extend(
            self.selections
                .iter()
                .enumerate()
                .map(|(id, _)| 61000 + id as u32),
        );
        old_layers.extend([60001, 60002, 60003].into_iter().filter(|id| match *id {
            60001 => self.raster.as_ref().is_some_and(|r| r.has_overlay()),
            60002 => self.filter.as_ref().is_some_and(|r| r.has_overlay()),
            _ => self.fall.as_ref().is_some_and(|r| r.has_overlay()),
        }));
        old_layers.sort_unstable();
        old_layers.dedup();
        let mut old_ui = self.published_ui.clone();
        old_ui.extend(
            self.selections
                .iter()
                .enumerate()
                .map(|(id, _)| 61000 + id as u32),
        );
        if !self.selections.is_empty() {
            old_ui.push(choice::ROOT);
        }
        old_ui.sort_unstable();
        old_ui.dedup();
        let mut images = BTreeSet::new();
        for image in self
            .images
            .values()
            .chain(
                self.sprites
                    .iter()
                    .flat_map(|s| s.image.iter().chain(s.scratch.iter())),
            )
            .chain(
                self.buttons
                    .values()
                    .filter_map(|b| b.sprite.image.as_ref()),
            )
        {
            images.insert(image.path.relative().to_owned());
        }
        for a in self.animations.values().chain(
            self.sprites
                .iter()
                .filter_map(|s| s.animation.as_ref().map(|a| &a.asset)),
        ) {
            images.extend(a.frames.iter().map(|i| i.path.relative().to_owned()));
        }
        images.extend(self.slot_menu_resources());
        if let Some(r) = &self.recollection {
            images.extend(r.resource_paths());
        }
        if let Some(r) = &self.gallery {
            images.extend(r.resource_paths());
        }
        if let Some(r) = &self.bgm_mode {
            images.extend(r.resource_paths());
        }
        if let Some(r) = &self.backlog_view {
            images.extend(r.resource_paths());
        }
        if let Some(r) = &self.raster {
            images.extend(r.overlay_path());
        }
        if let Some(r) = &self.filter {
            images.extend(r.overlay_path());
            images.insert(r.source_path());
        }
        if let Some(r) = &self.fall {
            images.extend(r.overlay_path());
        }
        if let Some(r) = &self.geometry {
            images.extend(r.overlay_path());
        }
        if let Some(r) = &self.mask_update {
            images.extend(r.overlay_path());
        }
        if let Some(r) = &self.rotation {
            images.extend(r.overlay_path());
        }
        if let Some(r) = &self.warp {
            images.extend(r.overlay_path());
        }
        if let Some(image) = &self.stage_underlay {
            images.insert(image.path.relative().to_owned());
        }
        Ok(Restore {
            client: self.client,
            bytes,
            template: self.machine.clone(),
            pool: self.pool.as_ref().unwrap().clone(),
            worker: None,
            vm: Some(vm),
            slot: None,
            native: None,
            raster_plan: None,
            raster_picture: None,
            effects_ready: false,
            effects_kind: 0,
            filter_seed: self.filter.as_ref().map(filter::Filter::detached_seed),
            filter_seeded: false,
            filter_draw: false,
            filter_picture: None,
            fall_seeded: false,
            fall_draw: false,
            fall_picture: None,
            sprite: 0,
            window: 0,
            button: 0,
            audio_index: 0,
            audio_requests: vec![],
            audio_job: None,
            audio: None,
            rendered: false,
            ready: false,
            failed: None,
            old_layers,
            old_ui,
            release_images: images.into_iter().collect(),
        })
    }
}
impl Restore {
    fn advance(&mut self, h: &mut dyn HostServices) -> Result<bool> {
        if self.ready {
            return Ok(true);
        }
        if self.slot.is_none() {
            if self.worker.is_none() {
                let bytes = self.bytes.clone();
                let template = self
                    .template
                    .take()
                    .context("SugarPot detached template consumed")?;
                self.worker = Some(self.pool.spawn(move |cancel| {
                    cancel.check()?;
                    let slot = Slot::decode(&bytes)?;
                    ensure!(slot.buffers.len() == 1, "SugarPot saved BUF record count");
                    ensure!(
                        slot.sprites.len() <= 256
                            && slot.buttons.len() <= 1024
                            && slot.windows.len() <= 256,
                        "SugarPot saved object budget"
                    );
                    ensure!(
                        (0..=3).contains(&slot.execution[2]),
                        "SugarPot saved native mode"
                    );
                    let machine = Machine::from_original_slot(&template, &slot)?;
                    let native =
                        crate::text::Execution::from_original_slot(&slot, &machine, &|| {
                            cancel.check()?;
                            Ok(())
                        })?;
                    cancel.check()?;
                    Ok(Parsed {
                        slot,
                        machine,
                        native,
                    })
                })?);
            }
            let Some(parsed) = self.worker.as_mut().unwrap().try_take()? else {
                return Ok(false);
            };
            self.worker = None;
            let vm = self.vm.as_mut().unwrap();
            vm.machine = Some(parsed.machine);
            self.native = Some(parsed.native);
            vm.storage = Some(h.storage(self.client)?);
            vm.mode = parsed.slot.execution[2];
            vm.mono = parsed.slot.execution[0];
            vm.current_window = if parsed.slot.execution[1] < 0 {
                0
            } else {
                usize::try_from(parsed.slot.execution[1])?
            };
            ensure!(
                parsed.slot.windows.is_empty() || vm.current_window < parsed.slot.windows.len(),
                "SugarPot saved current window ID"
            );
            vm.native_audio = parsed.slot.audio.clone();
            vm.memo = parsed.slot.title.clone();
            vm.last_voice_suffix = parsed.slot.previous_caption.clone();
            vm.sprites = vec![
                Sprite {
                    divisions: 1,
                    ..Default::default()
                };
                parsed.slot.sprites.len()
            ];
            vm.button_capacity = parsed.slot.buttons.len();
            vm.gauges = parsed.slot.gauges.clone();
            vm.read_all = parsed.slot.buffers[0].fields[3] != 0;
            vm.right_button = parsed.slot.buffers[0].fields[4];
            let buffer = &parsed.slot.buffers[0];
            ensure!(
                (0..=3).contains(&buffer.fields[0]),
                "SugarPot saved raster state"
            );
            ensure!(
                buffer.fields[0] == 0 || matches!(buffer.fields[1], 0 | 1),
                "SugarPot saved raster family"
            );
            let raster = vm.raster.get_or_insert_with(raster::Raster::default);
            raster.state = buffer.fields[0];
            raster.family = buffer.fields[1];
            let rgb = parsed.slot.buffers[0].fields[2] as u32;
            vm.clear_color = [(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, 255];
            self.slot = Some(parsed.slot);
        }
        let slot = self.slot.as_ref().unwrap();
        let vm = self.vm.as_mut().unwrap();
        while self.sprite < slot.sprites.len() {
            if !restore_sprite(vm, h, self.sprite, &slot.sprites[self.sprite])? {
                return Ok(false);
            }
            self.sprite += 1;
        }
        while self.window < slot.windows.len() {
            let saved = &slot.windows[self.window];
            for name in saved
                .frame
                .iter()
                .map(|r| r.image.as_str())
                .chain(saved.foreground.iter().map(|r| r.image.as_str()))
                .chain(saved.page.iter().map(|r| r.image.as_str()))
                .chain(
                    saved
                        .buttons
                        .iter()
                        .flat_map(|b| [b.image.as_str(), b.overlay.as_str()])
                        .filter(|s| !s.is_empty()),
                )
            {
                let Some(image) = vm.image(h, name)? else {
                    return Ok(false);
                };
                vm.declared_images.insert(image.path.relative().into());
            }
            let f = saved.fields;
            if saved.frame.is_some() && !slot.current_text.is_empty() {
                let Some(image) = vm.image(h, "namewnd1.abm")? else {
                    return Ok(false);
                };
                vm.declared_images.insert(image.path.relative().to_owned());
            }
            let mut window = Window {
                native: saved.clone(),
                text_cursor: [f[12], f[13]],
                ..Default::default()
            };
            let ints = |values: &[i32]| {
                values
                    .iter()
                    .copied()
                    .map(Value::Integer)
                    .collect::<Vec<_>>()
            };
            window.properties.insert("size".into(), ints(&f[3..5]));
            window.properties.insert("pos".into(), ints(&f[1..3]));
            window.properties.insert("txt".into(), ints(&f[12..18]));
            window
                .properties
                .insert("cel".into(), ints(&[f[6], f[7], f[8], f[9], f[10]]));
            if let Some(frame) = &saved.frame {
                window.properties.insert(
                    "frm".into(),
                    vec![
                        Value::String(frame.image.clone()),
                        Value::Integer(frame.position[0]),
                        Value::Integer(frame.position[1]),
                    ],
                );
            }
            vm.windows.push(window);
            self.window += 1;
        }
        while self.button < slot.buttons.len() {
            let saved = &slot.buttons[self.button];
            if !saved.image.is_empty() {
                let Some(image) = vm.image(h, &saved.image)? else {
                    return Ok(false);
                };
                ensure!(image.size[1] % 3 == 0, "SugarPot restored button atlas");
                vm.buttons.insert(
                    self.button as i32,
                    Button {
                        sprite: Sprite {
                            native_depth: vm
                                .depths
                                .get(image.path.relative())
                                .copied()
                                .unwrap_or(0),
                            source_name: saved.image.clone(),
                            image: Some(image),
                            position: saved.position,
                            visible: true,
                            cell: 0,
                            divisions: 3,
                            ..Default::default()
                        },
                        label: (!saved.text.is_empty())
                            .then(|| (saved.text.clone(), saved.text_offset)),
                        enabled: slot.execution[3] == 64,
                        state: 0,
                    },
                );
            }
            self.button += 1;
        }
        if !self.filter_seeded {
            let saved = &slot.buffers[0];
            ensure!(
                (0..=2).contains(&saved.fields[7]),
                "SugarPot saved filter state"
            );
            let seed = if saved.fields[7] == 1 {
                let Some(image) = vm.image(h, &saved.image)? else {
                    return Ok(false);
                };
                let source = h
                    .published_image_resource(self.client, image.path.relative())?
                    .context("SugarPot restored filter source outside prepared scope")?;
                Some(filter::Seed::loaded(
                    saved.image.clone(),
                    source,
                    saved.position,
                    h.now_ms(),
                ))
            } else {
                self.filter_seed.take().map(|mut seed| {
                    seed.restore_state(saved.fields[7], saved.position);
                    seed
                })
            };
            if let Some(seed) = seed {
                //Retain explicit same-client immutable CPU data and reserve it
                //again in this ticket. No old lease/path or live catalog is
                //consulted by detached preparation.
                let image = h.publish_image_resource(self.client, vm.next_image, seed.source())?;
                vm.next_image = vm
                    .next_image
                    .checked_add(1)
                    .context("SugarPot restored filter source ID")?;
                vm.declared_images.insert(image.path.relative().to_owned());
                vm.filter = Some(filter::Filter::install(seed, image));
            }
            self.filter_seeded = true;
        }
        if !self.fall_seeded {
            let saved = &slot.buffers[0];
            if !vm.restore_fall_state(h, saved.fields[5], saved.fields[6])? {
                return Ok(false);
            }
            self.fall_seeded = true;
        }
        ensure!(
            vm.settings.header[4] != 0
                || (!vm.fall.as_ref().is_some_and(|effect| effect.state != 0)
                    && !vm.filter.as_ref().is_some_and(|effect| effect.state != 0)),
            "SugarPot effect80 fall/filter after-mix frame projection not connected"
        );
        if !self.effects_ready {
            //43F580 runs raster, or fall then filter, at the last active
            //non-SYS sprite. Main buttons are composed after this boundary.
            if self.raster_plan.is_none() {
                let raster = vm.raster.as_ref().is_some_and(|effect| effect.state != 0)
                    && vm
                        .sprites
                        .iter()
                        .any(|sprite| sprite.visible && sprite.system == 0);
                let fall = !raster && vm.fall.as_ref().is_some_and(|effect| effect.state != 0);
                let filter = !raster && vm.filter.as_ref().is_some_and(|effect| effect.state != 0);
                if fall && !vm.prepare_fall_restore_projection()? {
                    return Ok(false);
                }
                if raster || fall || filter {
                    self.effects_kind =
                        u8::from(raster) | (u8::from(fall) << 1) | (u8::from(filter) << 2);
                    let mut plan = vm.plan_stage_sprites(
                        h,
                        ImageInput::Solid {
                            size: [1024, 600],
                            color: vm.clear_color,
                        },
                        0..vm.sprites.len(),
                        false,
                    )?;
                    if raster {
                        let spec = vm.raster.as_mut().unwrap().prepare_map();
                        plan.effects(vec![crysirror_image::effects::ImageEffect::RectMap {
                            spec,
                        }]);
                    } else {
                        if fall {
                            self.fall_draw = vm.append_fall_restore_projection(&mut plan)?;
                            if self.fall_draw {
                                plan.checkpoint();
                            }
                        }
                        if filter {
                            self.filter_draw = vm
                                .filter
                                .as_mut()
                                .unwrap()
                                .append_restore_projection(&mut plan);
                        }
                    }
                    self.raster_plan = Some(plan);
                }
            }
            if let Some(plan) = &mut self.raster_plan {
                if !plan.poll(h, self.client)? {
                    return Ok(false);
                }
                if self.effects_kind & 1 != 0 {
                    let finished = vm.raster.as_mut().unwrap().commit_map(h.now_ms())?;
                    if !finished {
                        self.raster_picture = plan.ready_resource();
                    }
                } else {
                    if self.effects_kind & 2 != 0 {
                        vm.fall
                            .as_mut()
                            .unwrap()
                            .commit_restore_projection(h.now_ms())?;
                        if self.fall_draw {
                            self.fall_picture = plan.checkpoint_resource(0);
                        }
                    }
                    if self.effects_kind & 4 != 0 {
                        vm.filter
                            .as_mut()
                            .unwrap()
                            .commit_restore_projection(h.now_ms())?;
                        if self.filter_draw {
                            self.filter_picture = plan.ready_resource();
                        }
                    }
                }
                self.raster_plan = None;
            }
            self.effects_ready = true;
        }
        if !self.rendered {
            vm.submit(
                h,
                self.old_layers
                    .iter()
                    .map(|id| json!({"op":"remove","id":id}))
                    .collect(),
                self.old_ui
                    .iter()
                    .map(|id| json!({"op":"remove","id":id}))
                    .collect(),
            )?;
            vm.publish(h, None)?;
            if let Some(resource) = self.raster_picture.take() {
                let image = h.publish_image_resource(self.client, vm.next_image, resource)?;
                vm.next_image = vm
                    .next_image
                    .checked_add(1)
                    .context("SugarPot restored raster image ID")?;
                vm.submit(h, vec![json!({"op":"put","layer":{"id":60001,"rect":[0,0,1024,600],"order":[999,1],"sampling":{"linear":false},"paint":{"kind":"image","asset":image.path.relative()}}})], vec![])?;
                vm.raster.as_mut().unwrap().retain_initial_image(image);
            }
            if let Some(resource) = self.fall_picture.take() {
                let image = h.publish_image_resource(self.client, vm.next_image, resource)?;
                vm.next_image = vm
                    .next_image
                    .checked_add(1)
                    .context("SugarPot restored fall image ID")?;
                vm.submit(h, vec![json!({"op":"put","layer":{"id":60003,"rect":[0,0,1024,600],"order":[999,1],"sampling":{"linear":false},"paint":{"kind":"image","asset":image.path.relative()}}})], vec![])?;
                vm.fall.as_mut().unwrap().retain_initial_image(image);
            }
            if let Some(resource) = self.filter_picture.take() {
                let image = h.publish_image_resource(self.client, vm.next_image, resource)?;
                vm.next_image = vm
                    .next_image
                    .checked_add(1)
                    .context("SugarPot restored filter image ID")?;
                vm.submit(h, vec![json!({"op":"put","layer":{"id":60002,"rect":[0,0,1024,600],"order":[999,2],"sampling":{"linear":false},"paint":{"kind":"image","asset":image.path.relative()}}})], vec![])?;
                vm.filter.as_mut().unwrap().retain_initial_image(image);
            }
            for (id, gauge) in slot.gauges.iter().enumerate() {
                if gauge.fields[6] != 0 {
                    ensure!(
                        vm.gauge(h, "ggon", &[id.to_string(), "0".into()])?,
                        "SugarPot restored gauge preparation"
                    );
                }
            }
            let (native, reload_caption) = self
                .native
                .take()
                .context("SugarPot detached text work missing")?;
            if let Some(caption) = reload_caption {
                vm.caption = caption;
            }
            //4561A0 clears the linked history then seeds44C510 from6004 in
            //novel mode, otherwise62CC, with the saved last voice in String1.
            vm.pending_voice_suffix = slot.previous_caption.clone();
            vm.capture_history_voice(if vm.mode == 2 {
                &slot.resume_text
            } else {
                &slot.current_text
            })?;
            if !slot.current_text.is_empty() {
                ensure!(
                    vm.text_inner(h, &slot.current_text, false)?,
                    "SugarPot detached redraw unexpected audio"
                );
                ensure!(
                    vm.text_remainder.is_empty(),
                    "SugarPot detached saved redraw pagination not mapped"
                );
                //Completed native safe points restore the complete page, never
                //a fresh character reveal which would change its save cursor.
                vm.submit(
                    h,
                    vm.published_layers
                        .iter()
                        .copied()
                        .filter(|id| matches!(id, 30002 | 30003))
                        .map(|id| json!({"op":"complete_reveal","id":id}))
                        .collect(),
                    vec![],
                )?;
                vm.reveal_until = h.now_ms();
            }
            vm.native_text = native;
            let duration = if vm.settings.header[4] != 0 { 0 } else { 500 };
            vm.wait = match slot.execution[3] {
                0 => None,
                4 => Some(Wait::Input),
                8 | 16 => Some(Wait::Text),
                64 => Some(Wait::Buttons),
                32 => {
                    if duration == 0 {
                        vm.restore_choices(h, &slot.selections)?;
                    } else {
                        vm.selections = slot.selections.clone();
                    }
                    Some(Wait::Selection)
                }
                flag => bail!("SugarPot detached wait {flag} not mapped"),
            };
            if vm.mode == 0 {
                vm.save_enabled = true;
                vm.load_enabled = true;
            }
            vm.window_ui.epoch = h.now_ms();
            vm.window_ui.auto_start = h.now_ms();
            vm.window_ui.auto_units = vm.native_text.units;
            vm.poll_window_controls(h, &mut InputFrame::default())?;
            //4561A0 starts native effect80 for500ms, retaining the saved
            //wait and inline-clear flag until its own completion. The common
            //Layer transition owns the old/new presentation resources.
            if duration != 0 {
                //435AB0 case80 draws62CC/selection and SYS only at its
                //terminal branch, after the old/new stage mix has completed.
                let show_window = !slot.current_text.is_empty() || slot.execution[3] == 32;
                vm.clear_window_controls(h)?;
                let mut layers = vm
                    .published_layers
                    .iter()
                    .copied()
                    .filter(|id| (30000..40000).contains(id))
                    .map(|id| json!({"op":"remove","id":id}))
                    .collect::<Vec<_>>();
                layers.extend(vm.system_window_layers(false)?);
                vm.submit(h, layers, vec![])?;
                vm.published_layers
                    .retain(|id| !(30000..40000).contains(id));
                vm.track_system_window_layers(false, false);
                if let Some(window) = vm.windows.get_mut(vm.current_window) {
                    window.visibility = 2;
                }
                vm.submit(h,vec![json!({"op":"transition","transition":{"effect":{"kind":"crossfade"},"duration_ms":duration}})],vec![])?;
                let resume = vm.wait.take().map(Box::new);
                vm.wait = Some(Wait::RestoreTransition {
                    deadline: h.now_ms().saturating_add(duration),
                    resume,
                    show_window,
                });
            }
            vm.restore_clock_origin = Some(h.now_ms());
            self.rendered = true;
        }
        while self.audio_index < 2 {
            let (name, mode, channel, bus, enabled, looping) = if self.audio_index == 0 {
                (
                    &slot.audio.bgm,
                    slot.audio.bgm_mode,
                    0,
                    PcmBus::Music,
                    vm.settings.header[1] == 1,
                    slot.audio.bgm_mode == 2,
                )
            } else {
                (
                    &slot.audio.env,
                    slot.audio.env_mode,
                    1,
                    PcmBus::Ambient,
                    vm.settings.header[3] == 1 && slot.audio.env_mode == 2,
                    true,
                )
            };
            ensure!((0..=2).contains(&mode), "SugarPot saved audio mode");
            if !name.is_empty() && enabled {
                let path = vm.resolve(
                    if channel == 0 { "bgm" } else { "se" },
                    name,
                    &["ogg", "wav"],
                )?;
                let Some(bytes) = vm.read(&path)? else {
                    return Ok(false);
                };
                self.audio_requests.push(AudioRequest::Play {
                    channel,
                    source: EncodedAudio {
                        bytes,
                        frame_limit: None,
                    },
                    intro: None,
                    parameters: AudioParameters {
                        bus,
                        looping,
                        gain: vm.native_gain(bus)?,
                        ..Default::default()
                    },
                });
            }
            self.audio_index += 1;
        }
        if self.audio.is_none() {
            if self.audio_job.is_none() {
                self.audio_job = Some(h.begin_audio_replacement(
                    self.client,
                    std::mem::take(&mut self.audio_requests),
                )?);
            }
            match self.audio_job.as_mut().unwrap().poll()? {
                AudioReplacementState::Pending => return Ok(false),
                AudioReplacementState::Failed(e) => return Err(e.into()),
                AudioReplacementState::Ready(resource) => {
                    self.audio = Some(resource);
                    self.audio_job = None;
                }
            }
        }
        vm.last_music = slot.audio.bgm.clone();
        vm.last_music_loop = slot.audio.bgm_mode == 2;
        vm.last_env = slot.audio.env.clone();
        vm.last_env_loop = slot.audio.env_mode == 2;
        vm.storage = None;
        self.ready = true;
        Ok(true)
    }
}
fn restore_sprite(
    vm: &mut SugarPotVm,
    h: &mut dyn HostServices,
    id: usize,
    saved: &SlotSprite,
) -> Result<bool> {
    let f = saved.fields;
    if saved.image.is_empty() {
        vm.sprites[id].system = f[4];
        vm.sprites[id].attribute = f[5];
        vm.sprites[id].origin = f[6];
        return Ok(true);
    }
    let Some(mut image) = vm.image(h, &saved.image)? else {
        return Ok(false);
    };
    let mut animation = vm.animation_state(&saved.image, h.now_ms())?;
    if f[4] == 0 {
        if let Some(state) = animation.take() {
            let Some(state) = vm.tone_animation(h, saved.image.clone(), state)? else {
                return Ok(false);
            };
            animation = Some(state);
        } else {
            let Some(toned) = vm.tone_image(h, image)? else {
                return Ok(false);
            };
            image = toned;
        }
    }
    let divisions = u32::try_from(f[2] + 1)?;
    let cell = u32::try_from(f[3])?;
    ensure!(
        divisions > 0 && image.size[1] % divisions == 0 && cell < divisions,
        "SugarPot saved sprite cell bounds"
    );
    if let Some(state) = &mut animation {
        let sequence = usize::try_from(f[7])?;
        ensure!(
            sequence < state.asset.programs.len(),
            "SugarPot saved animation sequence"
        );
        state.selected = sequence;
        state.clock.cells = u16::try_from(state.asset.programs[sequence].len())?;
        image = state.asset.frames[state.asset.programs[sequence][0] as usize].clone();
    }
    vm.sprites[id] = Sprite {
        native_depth: vm.depths.get(image.path.relative()).copied().unwrap_or(0),
        source_name: saved.image.clone(),
        image: Some(image),
        position: [f[0], f[1]],
        visible: true,
        system: f[4],
        attribute: f[5],
        origin: f[6],
        cell,
        divisions,
        animation,
        scratch: None,
    };
    Ok(true)
}
impl OriginalRestorePreparation for Restore {
    fn client(&self) -> ClientId {
        self.client
    }
    fn poll(
        &mut self,
        h: &mut dyn HostServices,
    ) -> crysirror_core::Result<OriginalRestoreProgress> {
        if let Some(error) = &self.failed {
            return Err(Error::Io(error.clone()));
        }
        match self.advance(h) {
            Ok(true) => Ok(OriginalRestoreProgress::Ready),
            Ok(false) => Ok(OriginalRestoreProgress::Pending),
            Err(e) => {
                let error = format!("SugarPot original restore: {e:#}");
                self.failed = Some(error.clone());
                Err(Error::Io(error))
            }
        }
    }
    fn finish(mut self: Box<Self>) -> crysirror_core::Result<OriginalRestoreCommit> {
        let mut finish = || -> Result<OriginalRestoreCommit> {
            ensure!(
                self.ready && self.failed.is_none(),
                "SugarPot original restore is not ready"
            );
            let mut vm = self.vm.take().context("SugarPot restored VM consumed")?;
            let mut layers = vec![];
            let mut ui = vec![];
            for batch in vm
                .staged_batches
                .take()
                .context("SugarPot staged scene missing")?
            {
                let mut batch: Json = serde_json::from_slice(&batch.bytes)?;
                layers.extend(
                    batch["layers"]
                        .as_array_mut()
                        .context("SugarPot staged layer operations")?
                        .drain(..),
                );
                ui.extend(
                    batch["ui"]
                        .as_array_mut()
                        .context("SugarPot staged UI operations")?
                        .drain(..),
                );
            }
            ensure!(
                layers.len() + ui.len() <= 4096,
                "SugarPot detached output operation budget"
            );
            Ok(OriginalRestoreCommit {
                vm: Box::new(vm),
                commands: vec![],
                engine_batches: vec![OriginalRestoreBatch {
                    schema: 1,
                    bytes: serde_json::to_vec(&json!({"layers":layers,"ui":ui}))?.into(),
                }],
                audio: vec![],
                prepared_audio: self.audio.take(),
                release_images: std::mem::take(&mut self.release_images),
            })
        };
        finish().map_err(|e| Error::Io(format!("SugarPot original restore finish: {e:#}")))
    }
}

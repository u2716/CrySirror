// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Detached original native policy/leases. All raster work is an engine job.
use super::*;
use crate::{
    audio_bridge::AudioRestoreSeed, native_save::NativeSaveFile, native_text::TextRestoreSeed,
    native_ui::SelectionRestoreSeed,
};
pub(crate) struct BridgeRestoreSeed {
    pub bridge: Bridge,
    pub audio: Option<AudioRestoreSeed>,
    pub text: TextRestoreSeed,
    pub selection: SelectionRestoreSeed,
    pub remove_layers: Vec<u32>,
    pub remove_ui: Vec<u32>,
    pub release_images: Vec<String>,
}
impl Bridge {
    pub(crate) fn restore_original_auxiliary(
        &mut self,
        file: &NativeSaveFile,
        profile: &crate::native_save::LegacySaveProfile,
    ) -> Result<()> {
        let auxiliary = crate::native_save::AuxiliaryState::restore_using(file, profile, &self.auxiliary)?;
        self.auxiliary = auxiliary;
        Ok(())
    }
    pub(crate) fn bind_saved_cutin(&self,text:&mut TextRestoreSeed)->Result<()> {
        if self.cutin_enabled {text.bind_cutin(self.auxiliary.cutin_mode,&self.auxiliary.cutin_rows)?;}
        Ok(())
    }
    pub(crate) fn client(&self) -> ClientId {
        self.client
    }
    pub(crate) fn canvas_size(&self) -> [u32; 2] {
        self.bindings.size
    }
    pub(crate) fn image_resources(&self) -> Result<crate::resource_bridge::NativeResources> {
        Ok(self
            .images
            .as_ref()
            .context("NNN image capability absent")?
            .resources())
    }
    pub(crate) fn insert_saved_picture(
        &mut self,
        picture: crate::image_bridge::PreparedPicture,
    ) -> Result<()> {
        ensure!(
            picture.raw.client() == self.client && (0..16).contains(&picture.slot),
            "NNN saved picture ownership"
        );
        self.images
            .as_mut()
            .context("NNN image capability absent")?
            .pictures
            .insert(picture.slot, picture);
        Ok(())
    }
    pub(crate) fn original_restore_seed(&self) -> Result<BridgeRestoreSeed> {
        self.original_restore_seed_at_boundary(false)
    }
    /// Called only after the adapter has verified an idle calculated TITLE
    /// mode. The detached replacement becomes a foreground PRINT/SELECT actor;
    /// it does not inherit the discarded Title reader or its read-mark policy.
    pub(crate) fn idle_title_restore_seed(&self) -> Result<BridgeRestoreSeed> {
        self.original_restore_seed_at_boundary(true)
    }
    fn original_restore_seed_at_boundary(&self, idle_title: bool) -> Result<BridgeRestoreSeed> {
        ensure!(
            !self.dependency_pending()
                && self.draw_wait.is_none()
                && self.overlap_wait.is_none()
                && (idle_title || self.background_read_marks.is_none()),
            "NNN original restore requires an idle foreground resource boundary"
        );
        let text = self.text_restore_seed()?;
        let selection = self.selection_restore_seed()?;
        let mut bridge = Self::new(self.client, self.profile, self.bindings)?;
        // Bounded native RAM records and immutable tables/leases only. No live
        // task, VM, mixer, Scene or composed software screen is cloned.
        bridge.metadata = self.metadata.clone();
        bridge.auxiliary = self.auxiliary.clone();
        bridge.chart_runtime = self.chart_runtime.clone();
        bridge.simple_wipe_tables = self.simple_wipe_tables.clone();
        bridge.rainbow_palette=self.rainbow_palette.clone();
        bridge.get_buffer = self.get_buffer.clone();
        bridge.once_printed=self.once_printed.clone();
        bridge.user_gaiji_paths=self.user_gaiji_paths.clone();
        bridge.functions = self.functions.clone();
        bridge.system_file = self.system_file.clone();
        bridge.system_script = self.system_script;
        bridge.must_set_film = self.must_set_film;
        bridge.film_skip_button_configured=self.film_skip_button_configured;
        bridge.film_button=self.film_button.as_ref().map(|button|button.detached()).transpose()?;
        bridge.cutin_enabled=self.cutin_enabled;
        bridge.must_set_cg = self.must_set_cg;
        bridge.must_set_music = self.must_set_music;
        bridge.off_layer_dont_set_cg = self.off_layer_dont_set_cg;
        bridge.system_result_extension = self.system_result_extension;
        // Persistent results belong to the current process, not the old slot.
        bridge.result_data = self.result_data.clone();
        bridge.skip = self.skip.clone();
        bridge.legacy_demo_flag = self.legacy_demo_flag;
        bridge.script_mouse_number = self.script_mouse_number;
        bridge.script_cursor_number = self.script_cursor_number;
        bridge.script_window_number = self.script_window_number;
        bridge.configuration_mask = self.configuration_mask;
        bridge.window_binding = self
            .window_binding
            .as_ref()
            .map(|binding| binding.detached());
        bridge.auto_kill_skip = self.auto_kill_skip;
        bridge.cannot_kill_skip = self.cannot_kill_skip;
        bridge.shift_message_only = self.shift_message_only;
        bridge.f4_enabled = self.f4_enabled;
        bridge.f5_enabled = self.f5_enabled;
        bridge.mark_skipped_messages = self.mark_skipped_messages;
        bridge.skip.next_command = false;
        bridge.skip.effect_command = false;
        bridge.skip.film_end = false;
        bridge.option_off = self.option_off;
        bridge.launch_recorded = self.launch_recorded;
        bridge.frame_time_ms = self.frame_time_ms;
        bridge.default_frame_time_ms = self.default_frame_time_ms;
        bridge.actor_clock = self.actor_clock.clone();
        bridge.last_native_steps = self.last_native_steps;
        bridge.last_background_steps = self.last_background_steps;
        bridge.background_maximum_steps = self.background_maximum_steps;
        // Resource preparation consumes wall time, not native actor time.
        // Re-anchor the retained scheduler on its first post-commit update.
        bridge.actor_clock_at = None;
        bridge.overlap_captures = self.overlap_captures;
        bridge.frame_canvas_created = self.frame_canvas_created;
        bridge.voice_policy=self.voice_policy.clone();
        bridge.overlap_frame = self.overlap_frame.clone();
        bridge.general_enter_frame=self.general_enter_frame.clone();
        bridge.cg_difference_picture=self.cg_difference_picture.clone();
        bridge.pattern_cache = self.pattern_cache.clone();
        bridge.images = Some(
            self.images
                .as_ref()
                .context("NNN native image capability absent")?
                .detached_restore_copy()?,
        );
        let mut remove_layers: BTreeSet<u32> = self
            .image_paths
            .keys()
            .chain(self.fill_effects.keys())
            .map(|slot| self.layer_id(*slot))
            .collect::<Result<_>>()?;
        if self.display_path.is_some() {
            remove_layers.insert(self.bindings.effect_base + u32::from(self.bindings.layers));
        }
        if self.post_text_path.is_some() {
            remove_layers.insert(self.bindings.effect_base + u32::from(self.bindings.layers) + 1);
        }
        remove_layers.extend(self.text.as_ref().unwrap().restored_layer_ids());
        let (selection_layers, mut remove_ui) = self.selection.as_ref().unwrap().restored_node_ids();
        remove_layers.extend(selection_layers);
        let mut release_images: BTreeSet<String> = self.image_paths.values().cloned().collect();
        if let Some(button)=&self.film_button {
            let (layers,ui)=button.cleanup_ids();remove_layers.extend(layers);remove_ui.extend(ui);
            if let Some(path)=button.owned_path(){release_images.insert(path.to_owned());}
        }
        release_images.extend(self.display_path.iter().cloned());
        release_images.extend(self.post_text_path.iter().cloned());
        if let Some(path)=self.selection.as_ref().and_then(|s|s.owned_special_image()){release_images.insert(path.to_owned());}
        if let Some(binding) = &self.window_binding {
            release_images.extend(binding.owned_paths().iter().cloned());
        }
        if let Some(path) = text.previous_window_path() {
            release_images.insert(path.to_owned());
        }
        Ok(BridgeRestoreSeed {
            bridge,
            audio: Some(self.audio_restore_seed()?),
            text,
            selection,
            remove_layers: remove_layers.into_iter().collect(),
            remove_ui,
            release_images: release_images.into_iter().collect(),
        })
    }
    pub(crate) fn restore_native_status(&mut self, file: &NativeSaveFile) -> Result<()> {
        self.restore_native_status_block(&file.blocks[2])
    }
    /// Check the whole native policy before chart replay can change audible or
    /// visible state. Full-slot loads use this on a detached bridge as well.
    pub(crate) fn validate_native_status_block(&self, status: &crate::native_save::SaveBlock) -> Result<()> {
        let saved = NativeStatus::new(status)?;
        ensure!(saved.word(180) == 0 || self.window_binding.is_some(),
            "NNN saved window variant capability not configured");
        ensure!(saved.word(484) <= 5, "NNN saved automatic message speed table bounds");
        ensure!(saved.word(477) == 0 || !self.film_skip_button_configured || self.film_button.is_some(),
            "NNN saved film-skip button requires native UI consumer");
        let position = NativeSaveFile::status_position(status)?;
        ensure!((0..=3).contains(&position.run_mode), "NNN saved active reader index");
        ensure!(saved.word(45) <= 60000, "NNN original frameTime range");
        self.system_file.as_ref().context("NNN original SystemFile absent")?
            .validate_player_names([saved.name(448)?, saved.name(452)?])?;
        Ok(())
    }
    pub(crate) fn restore_native_status_block(&mut self, status: &crate::native_save::SaveBlock) -> Result<()> {
        self.validate_native_status_block(status)?;
        let saved = NativeStatus::new(status)?;
        let position = NativeSaveFile::status_position(status)?;
        // Revision exhaustion and both native C strings have been checked.
        // Apply names before the remaining infallible policy assignments.
        self.system_file.as_mut().unwrap()
            .restore_player_names([saved.name(448)?, saved.name(452)?])?;
        self.script_window_number = saved.word(180);
        self.configuration_mask = saved.word(476);
        self.skip_film_enabled = saved.word(477) != 0;
        self.message_window_visible = saved.word(44) != 0;
        // SetGameStatusByLoad calls SetEye for all native slots, including
        // inactive ones; chart checkpoints contain these gates, not EFFECTs.
        self.native_layer_off = saved.word(48) as u32;
        for slot in 0..16 {
            self.layer_visible.insert(slot, self.native_layer_off & (1 << slot) == 0);
        }
        // Legacy setters retain raw variants. The next window consumer prepares
        // its sources asynchronously before using or acknowledging that call.
        self.script_cursor_number = saved.word(181);
        self.script_mouse_number = saved.word(182);
        self.auto_message = saved.word(484);
        self.cannot_click = Some(saved.word(485) != 0);
        self.cannot_skip = saved.word(486);
        self.system_script = Some(match position.run_mode {
            0 => 0,
            1 => position.script_number,
            2 => position.script_sub_number,
            3 => position.script_sub_sub_number,
            _ => unreachable!(),
        });
        self.last_message_id = Some(saved.word(28));
        self.last_select_id = Some(saved.word(32));
        self.last_select_hero = saved.word(47);
        self.saved_remaining = saved.word(46);
        let frame_ms = saved.word(45);
        if frame_ms > 0 { self.set_frame_time(frame_ms as u64)?; }
        Ok(())
    }
    /// Stage ordinary retained layers for future native captures, plus the
    /// separate exact shared frame used for current Print/window composition.
    pub(crate) fn saved_picture_operations(
        &mut self,
        host: &mut dyn HostServices,
    ) -> Result<Vec<Value>> {
        let mut layers = Vec::new();
        for slot in self.active_effects.clone() {
            if self.picture_assignments.contains(&slot) {
                let picture = self
                    .images
                    .as_ref()
                    .and_then(|images| images.pictures.get(&slot))
                    .context("NNN saved picture lease absent")?;
                let opaque = self
                    .not_trans
                    .get(&slot)
                    .copied()
                    .unwrap_or(slot < 4 || slot == 14);
                let image = if opaque {
                    picture.opaque.clone()
                } else {
                    picture.keyed.clone()
                };
                let published =
                    host.publish_image_resource(self.client, self.layer_id(slot)?, image.clone())?;
                self.image_paths
                    .insert(slot, published.path.relative().to_owned());
                let rects = self
                    .rectangles
                    .get(&slot)
                    .context("NNN saved picture geometry absent")?;
                layers.push(self.picture_operation(
                    slot,
                    image.size(),
                    published.path.relative(),
                    rects,
                    picture.source_clip,
                )?);
            }
            if let Some(color) = self.fill_effects.get(&slot) {
                let [w, h] = self.bindings.size;
                // A fill overrides the picture's node but keeps its prepared
                // source/path so a later command can restore that same source.
                layers.retain(|node| {
                    node["layer"]["id"].as_u64() != Some(u64::from(self.layer_id(slot).unwrap()))
                });
                layers.push(json!({"op":"put","layer":{"id":self.layer_id(slot)?,"parent":self.bindings.root,
                    "rect":[0,0,w,h],"order":[slot,0],"visible":self.layer_visible.get(&slot).copied().unwrap_or(true),
                    "paint":{"kind":"solid","color":[color[0],color[1],color[2],255]}}}));
            }
        }
        Ok(layers)
    }
    pub(crate) fn next_saved_rainbow(&self)->Option<i32> {
        self.saved_rainbow.first().copied().filter(|slot|self.saved_suna.first_key_value().is_none_or(|(next,_)|slot<=next))
    }
    pub(crate) fn initialize_saved_rainbow(&mut self,slot:i32,random:&mut crysirror_image::temporal_pattern::PatternRng)->Result<()> {
        ensure!(self.next_saved_rainbow()==Some(slot),"NNN saved RAINBOW initialization order changed");
        let mut palette=(*self.rainbow_palette).clone();
        palette.on_set(random)?;
        self.rainbow_palette=Arc::new(palette);
        self.saved_rainbow.remove(&slot);
        Ok(())
    }
    pub(crate) fn next_saved_suna(&self) -> Option<i32> {
        self.saved_suna.keys().next().copied()
    }
    pub(crate) fn complete_saved_suna(
        &mut self,
        slot: i32,
        resource: Arc<dyn crysirror_core::service::PatternResource>,
    ) -> Result<()> {
        ensure!(
            resource.client() == self.client && resource.info().size == self.bindings.size,
            "NNN saved SUNA initialized resource scope"
        );
        let parameters = self
            .saved_suna
            .remove(&slot)
            .context("NNN saved SUNA init slot absent")?;
        self.pattern_cache = Some(resource.clone());
        for state in self.patterns.values_mut() {
            state.resource = resource.clone();
        }
        self.patterns.insert(
            slot,
            SunaState {
                parameters,
                resource,
                count: 0,
            },
        );
        Ok(())
    }
    pub(crate) fn validate_saved_wait_effects(
        &self,
        advancing: bool,
        frozen_background: bool,
    ) -> Result<()> {
        ensure!(
            self.saved_suna.is_empty() && self.saved_rainbow.is_empty(),
            "NNN saved effect resource/RNG initialization incomplete"
        );
        if advancing {
            let effects = self.frame_effects();
            self.visible_effects(&effects)?;
        } else {
            ensure!(
                frozen_background || !self.has_animated_effects(),
                "NNN frozen saved selection background requires its original mode0/1 background resource"
            );
        }
        Ok(())
    }
    pub(crate) fn begin_saved_frame(
        &self,
        base: Arc<dyn ImageResource>,
        random: PatternRng,
        host: &mut dyn HostServices,
    ) -> Result<crate::native_presentation::NativeFrameWork> {
        ensure!(
            self.saved_suna.is_empty(),
            "NNN saved SUNA class initialization is incomplete"
        );
        let effects = self.frame_effects();
        let visible = self.visible_effects(&effects)?;
        // Include every ordinary native layer. Unlike a live capture there is
        // no earlier-layer Canvas: the base here is an engine-owned solid.
        let underlays = self
            .frame_underlays(&effects, &BTreeSet::from([0]))?
            .into_iter()
            .filter(|(slot, _)| *slot < 15)
            .collect();
        let effects = effects.into_iter().filter(|(slot, _)| *slot < 15).collect();
        crate::native_presentation::NativeFrameWork::with_context_base(
            self.client,
            base,
            crate::native_presentation::ordered_frame_plan(&effects, &visible, &underlays),
            random,
            false,
            self.frame_buffers(),
            host,
        )
    }
    pub(crate) fn saved_post_text_plan(
        &self,
    ) -> Result<Vec<crate::native_presentation::NativeFrameEffect>> {
        let effects = self.frame_effects();
        let visible = self.visible_effects(&effects)?;
        let underlays = self
            .frame_underlays(&effects, &BTreeSet::from([15]))?
            .into_iter()
            .filter(|(slot, _)| *slot == 15)
            .collect();
        let effects = effects
            .into_iter()
            .filter(|(slot, _)| *slot == 15)
            .collect();
        Ok(crate::native_presentation::ordered_frame_plan(
            &effects, &visible, &underlays,
        ))
    }
    pub(crate) fn saved_scene_commands(&self, layers: &[Value]) -> Result<Vec<u8>> {
        let [w, h] = self.bindings.size;
        let mut commands = vec![json!({"op":"put","layer":{
            "id":self.bindings.root,"rect":[0,0,w,h],"paint":{"kind":"group"}
        }})];
        commands.extend_from_slice(layers);
        Ok(serde_json::to_vec(&commands)?)
    }
    pub(crate) fn install_saved_post_frame(
        &mut self,
        frame: &crate::native_presentation::NativeFrameResult,
        host: &mut dyn HostServices,
    ) -> Result<Value> {
        self.validate_frame_resource(frame.target.as_ref())?;
        self.adopt_frame_buffers(&frame.buffers);
        if let Some(pattern) = &frame.shared_pattern {
            self.pattern_cache = Some(pattern.clone());
            for state in self.patterns.values_mut() {
                state.resource = pattern.clone();
            }
        }
        let id = self.bindings.effect_base + u32::from(self.bindings.layers) + 1;
        let image = host.publish_image_resource(self.client, id, frame.target.clone())?;
        self.post_text_path = Some(image.path.relative().to_owned());
        self.last_frame = Some(frame.target.clone());
        let [w, h] = self.bindings.size;
        Ok(
            json!({"op":"put","layer":{"id":id,"parent":self.bindings.root,
            "rect":[0,0,w,h],"order":[i32::MAX-1,0],
            "paint":{"kind":"image","asset":image.path.relative()}}}),
        )
    }
    pub(crate) fn install_saved_frame(
        &mut self,
        frame: &crate::native_presentation::NativeFrameResult,
        host: &mut dyn HostServices,
    ) -> Result<Value> {
        self.validate_frame_resource(frame.target.as_ref())?;
        self.adopt_frame_buffers(&frame.buffers);
        if let Some(pattern) = &frame.shared_pattern {
            self.pattern_cache = Some(pattern.clone());
            for state in self.patterns.values_mut() {
                state.resource = pattern.clone();
            }
        }
        // Native restored presentation does not Calcu or CountUp the saved
        // effects. Print's pattern/random work stays in its returned candidate.
        let id = self.bindings.effect_base + u32::from(self.bindings.layers);
        let image = host.publish_image_resource(self.client, id, frame.target.clone())?;
        self.display_path = Some(image.path.relative().to_owned());
        self.last_frame = Some(frame.target.clone());
        let [w, h] = self.bindings.size;
        Ok(
            json!({"op":"put","layer":{"id":id,"rect":[0,0,w,h],"order":[self.bindings.layers,0],
            "paint":{"kind":"image","asset":image.path.relative()}}}),
        )
    }
}

/// Legacy1000's GAMESTATUS record, with its eight-word header excluded from
/// payload. Fixed offsets are checked once before any getter can index it.
struct NativeStatus<'a>(&'a [u8]);
impl<'a> NativeStatus<'a> {
    fn new(status: &'a crate::native_save::SaveBlock) -> Result<Self> {
        ensure!(status.code == 2 && status.payload.len() == 7056, "NNN native status ABI");
        Ok(Self(&status.payload))
    }
    fn word(&self, native: usize) -> i32 {
        let at = (native - 8) * 4;
        i32::from_le_bytes(self.0[at..at + 4].try_into().unwrap())
    }
    fn name(&self, native: usize) -> Result<&[u8]> {
        let at = (native - 8) * 4;
        let raw = &self.0[at..at + 16];
        let end = raw.iter().position(|byte| *byte == 0)
            .context("NNN saved player name lacks native terminator")?;
        Ok(&raw[..end])
    }
}

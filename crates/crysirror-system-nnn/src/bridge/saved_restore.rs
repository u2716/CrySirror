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
        ensure!(
            !self.dependency_pending()
                && self.draw_wait.is_none()
                && self.overlap_wait.is_none()
                && self.background_read_marks.is_none(),
            "NNN original restore requires an idle foreground resource boundary"
        );
        let text = self.text_restore_seed()?;
        let selection = self.selection_restore_seed()?;
        let mut bridge = Self::new(self.client, self.profile, self.bindings)?;
        // Bounded native RAM records and immutable tables/leases only. No live
        // task, VM, mixer, Scene or composed software screen is cloned.
        bridge.metadata = self.metadata.clone();
        bridge.functions = self.functions.clone();
        bridge.system_file = self.system_file.clone();
        bridge.system_script = self.system_script;
        bridge.must_set_film = self.must_set_film;
        bridge.system_result_extension = self.system_result_extension;
        bridge.skip = self.skip.clone();
        bridge.skip.next_command = false;
        bridge.skip.effect_command = false;
        bridge.skip.film_end = false;
        bridge.option_off = self.option_off;
        bridge.launch_recorded = self.launch_recorded;
        bridge.frame_time_ms = self.frame_time_ms;
        bridge.actor_clock = self.actor_clock.clone();
        bridge.actor_clock_at = self.actor_clock_at;
        bridge.overlap_captures = self.overlap_captures;
        bridge.frame_canvas_created = self.frame_canvas_created;
        bridge.overlap_frame = self.overlap_frame.clone();
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
        remove_layers.extend(self.text.as_ref().unwrap().restored_layer_ids());
        let (selection_layers, remove_ui) = self.selection.as_ref().unwrap().restored_node_ids();
        remove_layers.extend(selection_layers);
        let mut release_images: BTreeSet<String> = self.image_paths.values().cloned().collect();
        release_images.extend(self.display_path.iter().cloned());
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
        // Older53b630 has none of later branch's volume/font/rename-layer
        // extensions. These native producer profiles need their real consumers.
        ensure!(
            file.status_word(180)? == 0,
            "NNN saved nondefault window variant requires native resource rebinding"
        );
        ensure!(
            file.status_word(181)? == 0 && file.status_word(182)? == 0,
            "NNN saved cursor/mouse variant requires native pointer-resource consumer"
        );
        ensure!(
            file.status_word(484)? == 0,
            "NNN saved auto-message timing requires native consumer"
        );
        ensure!(
            file.status_word(486)? == 0,
            "NNN saved cannot-skip requires native skip consumer"
        );
        ensure!(
            file.status_word(477)? == 0,
            "NNN saved film-skip button requires native UI consumer"
        );
        self.message_window_visible = file.status_word(44)? != 0;
        self.last_message_id = Some(file.status_word(28)?);
        let frame_ms = file.status_word(45)?;
        if frame_ms > 0 {
            self.set_frame_time(frame_ms as u64)?;
        }
        let name = |word: usize| -> Result<&[u8]> {
            let start = (word - 8) * 4;
            let raw = &file.blocks[2].payload[start..start + 16];
            let end = raw
                .iter()
                .position(|byte| *byte == 0)
                .context("NNN saved player name lacks native terminator")?;
            Ok(&raw[..end])
        };
        self.system_file
            .as_mut()
            .context("NNN original SystemFile absent")?
            .restore_player_names([name(448)?, name(452)?])?;
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
    pub(crate) fn validate_saved_wait_effects(&self) -> Result<()> {
        ensure!(
            !self.active_effects.contains(&15),
            "NNN saved last screen effect requires post-text frame composition"
        );
        ensure!(
            self.saved_suna.is_empty()
                && self.patterns.is_empty()
                && self.fades.is_empty()
                && self.greys.is_empty()
                && self.chara_trans.is_empty()
                && self.old_lamps.is_empty()
                && self.buras.is_empty()
                && self.moves.is_empty(),
            "NNN saved message/selection with active effects requires native actor frame/window dependency consumer"
        );
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
        let underlays = self.frame_underlays(&effects, &BTreeSet::from([0]))?;
        crate::native_presentation::NativeFrameWork::with_saved_base(
            self.client,
            base,
            crate::native_presentation::ordered_frame_plan(&effects, &visible, &underlays),
            random,
            host,
        )
    }
    pub(crate) fn install_saved_frame(
        &mut self,
        frame: &crate::native_presentation::NativeFrameResult,
        host: &mut dyn HostServices,
    ) -> Result<Value> {
        self.validate_frame_resource(frame.target.as_ref())?;
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

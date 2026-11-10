//! TaihiAllEffect/FuqueAllEffect for modal background scripts. Native effect
//! records and immutable leases are swapped; the same Bridge keeps the single
//! audio/system-file owner. Logical Layers/UI are queried through shared APIs.
//! No composed screen, device pointer or second renderer is stored here.
use super::*;
macro_rules! visual_fields {
    ($($field:ident:$ty:ty),* $(,)?)=>{
        #[derive(Default)]
        pub(crate) struct MenuVisual {
            $($field:$ty,)*
            pictures:BTreeMap<i32,crate::image_bridge::PreparedPicture>,
            layers:Vec<Value>,ui:Vec<Value>,
        }
        impl MenuVisual {
            fn swap_fields(&mut self,bridge:&mut Bridge) {
                $(std::mem::swap(&mut self.$field,&mut bridge.$field);)*
                if let Some(images)=&mut bridge.images {std::mem::swap(&mut self.pictures,&mut images.pictures);}
            }
        }
    }
}
visual_fields! {
    active_effects:BTreeSet<i32>,previous_effects:BTreeMap<i32,i32>,
    inactive_effect_records:BTreeMap<i32,crate::native_save::SavedEffect>,
    patterns:BTreeMap<i32,SunaState>,pattern_cache:Option<Arc<dyn crysirror_core::service::PatternResource>>,
    saved_suna:BTreeMap<i32,SunaParameters>,not_trans:BTreeMap<i32,bool>,layer_visible:BTreeMap<i32,bool>,
    fades:BTreeMap<i32,FadeInState>,greys:BTreeMap<i32,crate::native_effects::LayerGreyState>,negas:BTreeMap<i32,crate::native_effects::LayerGreyState>,
    chara_trans:BTreeMap<i32,crate::native_chara_trans::CharaTransState>,old_lamps:BTreeMap<i32,crate::native_old_lamp::OldLampState>,
    buras:BTreeMap<i32,crate::native_bura::BuraState>,moves:BTreeMap<i32,crate::native_move::NativeMoveState>,
    scrolls:BTreeMap<i32,crate::native_effects_scroll::ScrollState>,fusumas:BTreeMap<i32,crate::native_effects_fusuma::FusumaState>,
    shakes:BTreeMap<i32,crate::native_effects_shake::ShakeState>,flashes:BTreeMap<i32,crate::native_effects_flash::FlashState>,
    mosaics:BTreeMap<i32,crate::native_effects_mosaic::MosaicState>,picture_effects:BTreeMap<i32,crate::native_picture_effect::PictureEffectState>,
    display_path:Option<String>,last_frame:Option<Arc<dyn ImageResource>>,message_background:Option<Arc<dyn ImageResource>>,
    post_text_path:Option<String>,overlap_frame:Option<Arc<dyn ImageResource>>,image_paths:BTreeMap<i32,String>,
    picture_numbers:BTreeMap<i32,i32>,picture_assignments:BTreeSet<i32>,rectangles:BTreeMap<i32,crate::native_geometry::NativeRects>,fill_effects:BTreeMap<i32,[u8;3]>,
    passive_counts:BTreeMap<i32,(i32,i32)>,native_layer_off:u32,message_window_visible:bool,
    draw_wait:Option<crate::native_draw::NativeDraw>,overlap_wait:Option<crate::native_overlap::NativeOverlap>,restored_wait:Option<SavedWaitKind>,
    idle_background:Option<crate::native_draw::NativeDraw>,
    actor_refresh_required:bool,
}
impl Bridge {
    pub(crate) fn menu_source_reader(&self) -> Option<i32> {
        self.system_script
    }
    pub(crate) fn set_menu_source_reader(&mut self, value: Option<i32>) {
        self.system_script = value;
    }
    pub(crate) fn reanchor_menu_waits(&mut self, now: u64, paused: u64) -> Result<()> {
        if let Some(wait) = &mut self.draw_wait {
            wait.reanchor_after_pause(now, paused)?;
        }
        if let Some(wait) = &mut self.overlap_wait {
            wait.reanchor_after_pause(now, paused)?;
        }
        self.actor_clock_at = None;
        Ok(())
    }
    fn modal_declarations(&self, host: &mut dyn HostServices) -> Result<(Vec<Value>, Vec<Value>)> {
        let mut layers = BTreeSet::from([self.bindings.root]);
        for n in 0..u32::from(self.bindings.layers) + 2 {
            layers.insert(self.bindings.effect_base + n);
        }
        if let Some(text) = &self.text {
            layers.extend(text.restored_layer_ids());
        }
        let mut widgets = Vec::new();
        if let Some(selection) = &self.selection {
            let (ids, ui) = selection.restored_node_ids();
            layers.extend(ids);
            widgets.extend(ui);
        }
        let mut layer_specs = Vec::new();
        let mut ui_specs = Vec::new();
        for id in layers {
            let value: Value = serde_json::from_slice(
                &host.query_engine(self.client, crysirror_core::service::EngineQuery::Layer(id))?,
            )?;
            if !value.is_null() {
                layer_specs.push(value);
            }
        }
        for id in widgets {
            let value: Value = serde_json::from_slice(&host.query_engine(
                self.client,
                crysirror_core::service::EngineQuery::UiWidget(id),
            )?)?;
            if !value.is_null() {
                ui_specs.push(value);
            }
        }
        Ok((layer_specs, ui_specs))
    }
    pub(crate) fn freeze_menu_visual(&mut self, host: &mut dyn HostServices) -> Result<MenuVisual> {
        ensure!(
            !self.dependency_pending() && self.draw_wait.is_none() && self.overlap_wait.is_none(),
            "NNN modal background needs a completed foreground input frame"
        );
        self.hide_film_button(host)?;
        let (layers, ui) = self.modal_declarations(host)?;
        let mut snapshot = MenuVisual::default();
        // Keep a ready immutable picture bank for the background's initial
        // native effects. Publication paths stay with the suspended owner.
        let pictures = self
            .images
            .as_ref()
            .context("NNN modal image capability absent")?
            .pictures
            .clone();
        snapshot.swap_fields(self);
        snapshot.layers = layers;
        snapshot.ui = ui;
        self.images.as_mut().unwrap().pictures = pictures;
        macro_rules! copy {($($field:ident),*)=>{$(self.$field=snapshot.$field.clone();)*};}
        copy!(
            active_effects,
            previous_effects,
            inactive_effect_records,
            patterns,
            pattern_cache,
            saved_suna,
            not_trans,
            layer_visible,
            fades,
            greys,
            negas,
            chara_trans,
            old_lamps,
            buras,
            moves,
            scrolls,
            fusumas,
            shakes,
            flashes,
            mosaics,
            picture_effects,
            last_frame,
            message_background,
            overlap_frame,
            picture_assignments,
            picture_numbers,
            rectangles,
            fill_effects,
            passive_counts,
            native_layer_off,
            message_window_visible
        );
        let mut layer_ops = snapshot
            .layers
            .iter()
            .filter(|s| s["id"] != self.bindings.root)
            .map(|s| json!({"op":"remove","id":s["id"]}))
            .collect::<Vec<_>>();
        layer_ops.push(json!({"op":"set_clock_paused","id":self.bindings.root,"paused":false}));
        let ui_ops = snapshot
            .ui
            .iter()
            .map(|s| json!({"op":"set_visible","id":s["id"],"visible":false}))
            .collect::<Vec<_>>();
        if let Err(error) = host.submit_engine(
            self.client,
            1,
            &serde_json::to_vec(&json!({"layers":layer_ops,"ui":ui_ops}))?,
        ) {
            snapshot.swap_fields(self);
            return Err(error.into());
        }
        self.actor_input = None;
        self.actor_ready_for_input = false;
        self.actor_clock_at = None;
        Ok(snapshot)
    }
}
impl MenuVisual {
    pub(crate) fn swap(&mut self, bridge: &mut Bridge, host: &mut dyn HostServices) -> Result<()> {
        ensure!(
            !bridge.dependency_pending(),
            "NNN modal visual switch during admitted image/audio work"
        );
        let (layers, ui) = bridge.modal_declarations(host)?;
        let mut layer_ops = layers
            .iter()
            .filter(|s| s["id"] != bridge.bindings.root)
            .map(|s| json!({"op":"remove","id":s["id"]}))
            .collect::<Vec<_>>();
        // Roots precede their descendants; retained GlyphView references are
        // resolved only after the complete atomic batch has been validated.
        let mut put = self.layers.clone();
        put.sort_by_key(|s| u8::from(s["id"] != bridge.bindings.root));
        layer_ops.extend(put.into_iter().map(|s| json!({"op":"put","layer":s})));
        let ui_ops = self
            .ui
            .iter()
            .cloned()
            .map(|s| json!({"op":"put","widget":s}))
            .collect::<Vec<_>>();
        host.submit_engine(
            bridge.client,
            1,
            &serde_json::to_vec(&json!({"layers":layer_ops,"ui":ui_ops}))?,
        )?;
        self.swap_fields(bridge);
        self.layers = layers;
        self.ui = ui;
        bridge.actor_input = None;
        bridge.actor_ready_for_input = false;
        bridge.actor_clock_at = None;
        Ok(())
    }
    pub(crate) fn resource_paths(&self) -> BTreeSet<String> {
        self.image_paths
            .values()
            .cloned()
            .chain(self.display_path.iter().cloned())
            .chain(self.post_text_path.iter().cloned())
            .chain(
                self.draw_wait
                    .as_ref()
                    .and_then(|w| w.owned_path())
                    .map(str::to_owned),
            )
            .chain(
                self.overlap_wait
                    .as_ref()
                    .and_then(|w| w.owned_path())
                    .map(str::to_owned),
            )
            .chain(
                self.idle_background
                    .as_ref()
                    .and_then(|w| w.owned_path())
                    .map(str::to_owned),
            )
            .collect()
    }
    pub(crate) fn retire(self, client: ClientId, host: &mut dyn HostServices) -> Result<()> {
        for path in self.resource_paths() {
            host.release_image_resource(client, &path)?;
        }
        Ok(())
    }
}

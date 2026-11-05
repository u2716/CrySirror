//! Original process11 group/action policy. Motion, coverage, hit testing and
//! retained visual state are engine capabilities; this module only maps them.
use super::*;
use crysirror_core::image_processing::{ImageInput, ImageProcessState};
#[path="menu_values.rs"]
mod values;
#[path="menu_gestures.rs"]
mod gestures;
#[path="menu_drag.rs"]
mod drag;
#[path="menu_keyboard.rs"]
mod keyboard;
const ROOT: u32 = 0x36ff_fffe;
const UI_ROOT: u32 = 0x36ff_fffd;
const BASE: u32 = 0x36ff_0000;
const CAPTURE: u32 = 0x3c01_0000;
const CAPTURE_LAYER: u32 = 0x1c01_0000;
pub(super) struct NativeMenu {
    pub(super) group: Group,
    declaration:Group,
    ticks: u64,
    /// Per-button original visual state99/1/2 and scale transition admission.
    states: Vec<i16>,
    scale_started: BTreeMap<u32, u64>,
    pending_sounds: Vec<u16>,
    pending_sound_assets: Vec<String>,
    sound_asset_started: bool,
    pending_action: Option<String>,
    pub(super) activation_ordinal: Option<usize>,
    pub(in crate::adapter) active: bool,
    capture_surface: bool,
    capture_visible: bool,
    pub(in crate::adapter) animated_images: BTreeSet<u32>,
    slider_values:BTreeMap<(i16,i16),i32>,
    pub(in crate::adapter) pending_values:Vec<crate::machine::MenuValue>,
    drag_origin:Option<[i32;2]>,
    drag_visual_origin:Option<[f32;2]>,
    swipe_delta:i32,
    gesture_tick:u64,
    drag_drop:drag::NativeDragState,
    keyboard:keyboard::KeyboardState,
    shift_action:Option<String>,
    shifting:bool,
}
impl NativeMenu {
    pub(in crate::adapter) fn text_stream_paused(&self)->bool{self.capture_visible}
    pub(super) fn ui_waiting(&self)->bool {self.active&&self.pending_action.is_none()&&self.pending_sounds.is_empty()&&self.pending_sound_assets.is_empty()}
}
impl Bridge<'_> {
    pub(super) fn native_overlay_policy(&mut self,window_visible:bool,confirmation_visible:bool,dialogue_visible:bool,status_visible:bool,indicator:Option<u8>)->Result<()> {
        let mut commands=Vec::new();
        let confirming=self.services.history.confirming()||self.services.system.gesture_confirming();
        let signature=(confirmation_visible,dialogue_visible,status_visible,confirming,window_visible);
        if self.services.native_paint_signature!=Some(signature) {
            // Draw gates do not overwrite saved actor visibility at184.
            for object in self.services.visual.roots() {
                let visible=match object {88=>status_visible,75|76=>window_visible,_=>continue};
                commands.push(json!({"op":"set_opacity","id":super::visual::object_layer(object),"opacity":if visible{255}else{0}}));
            }
            self.services.native_status_visible=status_visible;
            self.services.native_dialogue_visible=dialogue_visible;
            self.services.native_window_visible=window_visible;
            self.set_native_dialogue_visibility(window_visible)?;
            //4226B0 masks all builders and panel. Confirmation is a final
            //separate draw branch, masked by11480 but not history23872.
            for scope in 0..4 {
                self.with_menu_scope(scope,|bridge|{
                    if bridge.services.menu.is_some(){
                        let visible=dialogue_visible||(scope==0&&confirming&&confirmation_visible);
                        commands.push(json!({"op":"set_opacity","id":bridge.menu_root(),"opacity":if visible{255}else{0}}));
                    }
                    Ok(())
                })?;
            }
            self.services.native_paint_signature=Some(signature);
        }
        if self.services.native_indicator!=indicator||self.services.indicators_dirty {
            for slot in [17u8,18] {
                let Some(group)=self.services.template_groups.get(&slot)else{continue};
                let base=0x3600_0000+u32::from(slot)*65536;
                let mut drawable=BTreeSet::new();
                if indicator==Some(slot) {
                    drawable.extend((0..group.images.len()).map(|n|base+n as u32));
                    drawable.extend((0..group.texts.len()).map(|n|base+0xf000+n as u32));
                    for (button,row) in group.buttons.iter().enumerate() {
                        drawable.extend(row.images.iter().enumerate().filter(|(_,image)|crate::machine::menu_ops::image_visible(image.state,99)).map(|(image,_)|base+4096+button as u32*128+image as u32));
                        drawable.extend(row.texts.iter().enumerate().filter(|(_,text)|matches!(text.state,0|99)).map(|(text,_)|base+0x8000+button as u32*128+text as u32));
                    }
                }
                for id in self.services.layer_ids.get(&slot).into_iter().flatten() {
                    let layer:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::Layer(*id))?)?;
                    if !layer.is_null(){commands.push(json!({"op":"set_visible","id":id,"visible":drawable.contains(id)}));}
                }
            }
            self.services.native_indicator=indicator;self.services.indicators_dirty=false;
        }
        if commands.is_empty(){Ok(())}else{self.submit(commands,vec![])}
    }
    pub(in crate::adapter) fn menu_base(&self)->u32 { BASE-u32::from(self.services.menu_scope)*0x1_0000 }
    fn menu_root(&self)->u32 {self.menu_base()+0xfffe}
    fn menu_ui_root(&self)->u32 {self.menu_base()+0xfffd}
    fn menu_capture(&self)->u32 {CAPTURE+u32::from(self.services.menu_scope)}
    fn menu_capture_layer(&self)->u32 {CAPTURE_LAYER+u32::from(self.services.menu_scope)}
    pub(in crate::adapter) fn with_menu_scope<T>(&mut self,scope:u8,operation:impl FnOnce(&mut Self)->Result<T>)->Result<T> {
        ensure!(scope<=3,"Circus native CWnd scope");
        let previous=self.services.menu_scope;
        if previous==scope {return operation(self)}
        if let Some(menu)=self.services.menu.take(){self.services.scoped_menus.insert(previous,menu);}
        self.services.menu=self.services.scoped_menus.remove(&scope);
        self.services.menu_scope=scope;
        let result=operation(self);
        if let Some(menu)=self.services.menu.take(){self.services.scoped_menus.insert(scope,menu);}
        self.services.menu=self.services.scoped_menus.remove(&previous);
        self.services.menu_scope=previous;
        result
    }
    pub(super) fn advance_all_menus(&mut self)->Result<()> {
        for scope in 0..4 {self.with_menu_scope(scope,|bridge|bridge.advance_menu())?;}
        Ok(())
    }
    pub(super) fn clear_all_menus(&mut self)->Result<()> {
        for scope in 0..4 {self.with_menu_scope(scope,|bridge|bridge.clear_menu())?;}
        Ok(())
    }
    pub(super) fn reverse_menu_images(&mut self,reverse:bool)->Result<()> {
        let Some(menu)=self.services.menu.as_mut() else{return Ok(())};
        if reverse {menu.group.behavior_flags.insert(245);}else{menu.group.behavior_flags.remove(&245);}
        let count=menu.group.images.len();let mut commands=Vec::new();
        for n in 0..count {
            let id=self.menu_base()+n as u32;
            let layer:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::Layer(id))?)?;
            if !layer.is_null(){commands.push(json!({"op":"set_order","id":id,"order":[100,if reverse{-(n as i32)}else{n as i32}]}));}
        }
        self.submit(commands,vec![])
    }
    pub(super) fn update_menu_row(
        &mut self,
        ordinal: usize,
        action: &str,
        text: &str,
    ) -> Result<()> {
        let Some(mut menu) = self.services.menu.take() else {
            return Ok(());
        };
        let result = (|| {
            let Some(button) = menu.group.buttons.get_mut(ordinal) else {
                return Ok(());
            };
            button.action = action.into();
            button.behavior_flags.insert(209);
            for part in &mut button.texts {
                part.text = text.into();
            }
            let mut layers = Vec::new();
            self.text_layers(
                &mut layers,
                self.menu_root(),
                self.menu_base() + 0x8000 + ordinal as u32 * 128,
                &button.texts,
                100,
            )?;
            self.submit(layers, vec![])
        })();
        self.services.menu = Some(menu);
        result
    }
    pub(super) fn resume_menu(&mut self) -> Result<()> {
        let menu = self
            .services
            .menu
            .as_mut()
            .context("Circus retained gallery menu missing")?;
        menu.active = true;
        self.submit(
            vec![],
            vec![json!({"op":"set_enabled","id":self.menu_ui_root(),"enabled":true})],
        )
    }
    pub(super) fn clear_menu(&mut self) -> Result<()> {
        self.clear_text_streams(self.menu_base());
        if let Some(menu) = self.services.menu.take() {
            if menu.capture_surface {
                let mut canvases = vec![];
                if menu.capture_visible {
                    canvases.push(json!({"op":"hide","layer":self.menu_capture_layer()}));
                }
                canvases.push(json!({"op":"destroy","id":self.menu_capture()}));
                self.host.submit_engine(
                    self.client,
                    1,
                    &serde_json::to_vec(&json!({"canvases":canvases}))?,
                )?;
            }
            self.submit(
                vec![json!({"op":"remove","id":self.menu_root()})],
                vec![json!({"op":"remove","id":self.menu_ui_root()})],
            )?;
        }
        Ok(())
    }

    pub(super) fn begin_menu(&mut self, group: &Group) -> Result<bool> {
        if self.services.menu.as_ref().is_some_and(|m| m.active&&m.declaration==*group) {
            return Ok(true);
        }
        self.clear_menu()?;
        let declaration=group.clone();
        let mut prepared=group.clone();
        //413E80 converts the associated native paints once when the control
        //collection is activated. The second and later state2 paints become
        //the variable-width fill;100/101 remain movable knob declarations.
        let mut range_paints=0;
        for button in &mut prepared.buttons {
            if button.slider.as_ref().is_some_and(|slider|slider.step()!=0.) {
                for image in &mut button.images {
                    image.state=match image.state {99=>103,1=>104,2=>{let state=if range_paints==0{2}else{105};range_paints+=1;state},state=>state};
                }
            }
        }
        let group=&prepared;
        ensure!(
            group.buttons.len() <= 224 && group.images.len() <= 4096,
            "Circus retained menu declaration limit"
        );
        // Acquire immutable CPU leases once through shared image workers. The
        // empty operation graph preserves the original stored alpha bytes.
        for button in &group.buttons {
            if !button.behavior_flags.contains(&213) {
                continue;
            }
            let Some(part) = button
                .images
                .iter()
                .find(|p| p.state == 99 && self.native_part_image(p).is_some())
            else {
                continue;
            };
            let image=self.native_part_image(part).cloned().context("Circus coverage image identity")?;
            let key=image.path.relative().to_owned();
            if self.services.menu_images.contains_key(&key) {
                continue;
            }
            if let Some(image)=self.native_part_image(part).cloned() {
                if let Some(resource)=self.host.published_image_resource(self.client,image.path.relative())? {
                    self.services.menu_images.insert(key.clone(),resource);
                    continue;
                }
            }
            if let Some((slot, job)) = &mut self.services.menu_image_job {
                ensure!(*slot == key, "Circus menu image completion scope");
                match job.poll()? {
                    ImageProcessState::Pending => return Ok(false),
                    ImageProcessState::Failed(e) => bail!("Circus menu image lease failed: {e}"),
                    ImageProcessState::Ready(image) => {
                        ensure!(
                            image.client() == self.client,
                            "Circus foreign image coverage"
                        );
                        self.services.menu_images.insert(key.clone(), image);
                        self.services.menu_image_job = None;
                    }
                }
            } else {
                let name = self
                    .services
                    .image_names
                    .get(&part.slot)
                    .context("Circus menu image source")?;
                let source = ImageInput::Asset {
                    path: image_path(name)?.relative().into(),
                    size: self.services.image_extents[&part.slot],
                    fingerprint: None,
                };
                let job = self.host.image_metadata(self.client)?.begin_process(
                    ImageInput::Composite {
                        base: Box::new(source),
                        layers: vec![],
                    },
                    Arc::from(b"[]".as_slice()),
                )?;
                self.services.menu_image_job = Some((key, job));
                return Ok(false);
            }
        }
        self.root()?;
        let [x, y] = group.position;
        let mut layers = vec![
            json!({"op":"put","layer":{"id":self.menu_root(),"parent":SCENE_ROOT,"opacity":if self.services.native_dialogue_visible || self.services.menu_scope==0&&self.services.native_paint_signature.is_some_and(|signature|signature.0&&signature.3){255}else{0},"rect":[x,y,1280,720],"paint":{"kind":"solid","color":[0,0,0,0]},"order":[100,0]}}),
        ];
        let mut ui = vec![
            json!({"op":"put","widget":{"id":self.menu_ui_root(),"rect":[x,y,1280,720],"kind":"button","keyboard_focusable":false,"keyboard_navigation":false,"keyboard_activation":false,"key_event_scope":true,"key_events":keyboard::bindings(None,self.services.menu_scope,vec![]),"interaction_only":true}}),
        ];
        for (n, part) in group.images.iter().enumerate() {
            let id = self.menu_base() + n as u32;
            self.image_layer(&mut layers, id, part, 100)?;
            if group.behavior_flags.contains(&245) {
                if let Some(layer) = layers.iter_mut().rev().find(|v| v["layer"]["id"] == id) {
                    layer["layer"]["order"] = json!([100, -(n as i32)]);
                }
            }
            if let Some(last) = layers.iter_mut().rev().find(|v| v["layer"]["id"] == id) {
                last["layer"]["parent"] = json!(self.menu_root());
                last["layer"]["transform"]["pivot_x"]=json!(f32::from(part.source[2])*0.5);
                last["layer"]["transform"]["pivot_y"]=json!(f32::from(part.source[3])*0.5);
                last["layer"]["visible"] = json!(true);
            }
        }
        ensure!(
            group.texts.len() < 4094,
            "Circus menu text namespace budget"
        );
        self.text_layers(&mut layers, self.menu_root(), self.menu_base() + 0xf000, &group.texts, 100)?;
        for (n, button) in group.buttons.iter().enumerate() {
            let [x, y, r, b] = button.bounds;
            let width=i32::from(r)-i32::from(x);let height=i32::from(b)-i32::from(y);
            let widget = self.menu_base() + 0x8000 + n as u32;
            ensure!(i32::from(r)-i32::from(x)<=8192&&i32::from(b)-i32::from(y)<=8192&&(i64::from(r)-i64::from(x))*(i64::from(b)-i64::from(y))<=16*1024*1024,"Circus native UI coverage budget");
            ui.push(json!({"op":"put","widget":{"id":widget,"parent":self.menu_ui_root(),"z":group.buttons.len()-n,"rect":[x,y,width.max(1),height.max(1)],"kind":if button.slider.is_some(){"slider"}else{"button"},"slider_keyboard":false,"keyboard_navigation":false,"keyboard_activation":false,"key_events":keyboard::bindings(Some(button),self.services.menu_scope,vec![]),"interaction_only":true,"keyboard_focusable":button.slider.is_some()||!button.action.is_empty()&&!button.action.starts_with('-'),"focus_follows_pointer":true}}));
            ui.push(json!({"op":"blur","id":widget}));
            if button.behavior_flags.contains(&213) {
                if let Some(part) = button
                    .images
                    .iter()
                    .find(|p| p.state == 99 && self.native_part_image(p).is_some_and(|image|self.services.menu_images.contains_key(image.path.relative())))
                {
                    let key=self.native_part_image(part).context("Circus immutable coverage source")?.path.relative();
                    let resource = &self.services.menu_images[key];
                    let sx = u32::try_from(part.source[0])?;
                    let sy = u32::try_from(part.source[1])?;
                    let size = [u32::try_from(r - x)?, u32::try_from(b - y)?];
                    let coverage = resource.alpha_region([sx, sy, size[0], size[1]])?;
                    ui.push(json!({"op":"set_hit_mask","id":widget,"mask":{"size":size,"coverage":coverage,"threshold":1,"strict_bounds":true}}));
                } else {
                    //412FB0 returns false when no state99 texture exists.
                    //A missing alpha source must never become a rectangle hit.
                    let size=[u32::try_from(r-x)?,u32::try_from(b-y)?];
                    ui.push(json!({"op":"set_hit_mask","id":widget,"mask":{"size":size,"coverage":vec![0u8;usize::try_from(u64::from(size[0])*u64::from(size[1]))?],"threshold":1,"strict_bounds":true}}));
                }
            }
            ensure!(
                button.texts.len() <= 128,
                "Circus menu per-button text budget"
            );
            if width<=0||height<=0 {ui.push(json!({"op":"set_hit_mask","id":widget,"mask":{"size":[1,1],"coverage":[0],"threshold":1,"strict_bounds":true}}));}
            self.text_layers(
                &mut layers,
                self.menu_root(),
                self.menu_base() + 0x8000 + n as u32 * 128,
                &button.texts,
                100,
            )?;
            for (index,pivot) in &button.text_pivots {
                let id=self.menu_base()+0x8000+n as u32*128+*index as u32;
                if let Some(command)=layers.iter_mut().rev().find(|command|command["layer"]["id"]==id) {
                    command["layer"]["transform"]["pivot_x"]=json!(pivot[0]);command["layer"]["transform"]["pivot_y"]=json!(pivot[1]);
                }
            }
            for (k, part) in button.images.iter().enumerate() {
                let id = self.menu_base() + 4096 + n as u32 * 128 + k as u32;
                self.image_layer(&mut layers, id, part, 100)?;
                if let Some(last) = layers.iter_mut().rev().find(|v| v["layer"]["id"] == id) {
                    last["layer"]["parent"] = json!(self.menu_root());
                    last["layer"]["visible"] = json!(crate::machine::menu_ops::image_visible(part.state,99));
                    if let Some(pivot)=button.image_pivots.get(&k){last["layer"]["transform"]["pivot_x"]=json!(pivot[0]);last["layer"]["transform"]["pivot_y"]=json!(pivot[1]);}
                }
            }
        }
        if let Some(m)=&group.motion {
            let id=self.menu_root();
            layers.push(json!({"op":"begin_sequence","id":id,"spec":{"values":group.position,"bindings":[{"layer":id,"channel":0,"target":{"kind":"motion","property":"x"}},{"layer":id,"channel":1,"target":{"kind":"motion","property":"y"}}]}}));
            layers.push(json!({"op":"sequence","id":id,"operation":{"kind":"enqueue","lane":0,"track":{"kind":"linear","channels":[0,1],"target":m.to,"ticks":m.duration_ticks,"velocity_divisor":m.duration_ticks as f32,"start":m.from,"references":[0,1],"loop_origins":[0,1],"delta":"explicit_start","complete_first":false,"complete_zero":true}}}));
        }
        let mut animated_images=super::control_records::declaration_attributes(group,&mut layers,self.menu_base())?;
        let mut images=self.services.images.clone();
        let mut references:BTreeMap<String,[f32;2]>=self.services.images.iter().filter_map(|(slot,image)|self.services.image_extents.get(slot).map(|size|(image.path.relative().into(),size.map(|value|value as f32)))).collect();
        if let Some(image)=self.captured_native_image(){images.insert(45,image.clone());references.insert(image.path.relative().into(),[1280.,720.]);}
        animated_images.extend(menu_image_animations(group, &images, &mut layers,self.menu_base(),&references)?);
        // The original process starts with no pending previous-control action.
        // Engine events are retained across worker suspensions; transient hover
        // state alone must never serve as an activation mailbox.
        let _ = self.host.query_engine(self.client, EngineQuery::UiSubtreeEvents(self.menu_ui_root()))?;
        self.prepare_text_streams(group,&mut layers,self.menu_base())?;
        self.submit(layers, ui)?;
        let mut scalar_values: BTreeMap<_,_>=group.buttons.iter().filter_map(|button|button.slider.as_ref()).map(|slider|(slider.target(),slider.initial_value)).collect();
        for button in &group.buttons {if button.behavior_flags.contains(&214){scalar_values.insert((93,button.check_index),button.check_value);}}
        self.services.menu = Some(NativeMenu {
            declaration,
            group: group.clone(),
            ticks: 0,
            states: vec![99; group.buttons.len()],
            scale_started: BTreeMap::new(),
            pending_sounds: Vec::new(),
            pending_sound_assets: Vec::new(),
            sound_asset_started: false,
            pending_action: None,
            activation_ordinal: None,
            active: true,
            capture_surface: group.capture_transition.is_some(),
            capture_visible: group.capture_transition.is_some(),
            animated_images,
            slider_values:scalar_values,
            pending_values:Vec::new(),
            drag_origin:None,drag_visual_origin:None,swipe_delta:0,gesture_tick:u64::MAX,drag_drop:drag::NativeDragState::default(),keyboard:keyboard::KeyboardState::default(),shift_action:None,shifting:group.motion.is_some(),
        });
        if let Some(menu)=self.services.menu.take(){let result=self.update_native_slider_paints(&menu);self.services.menu=Some(menu);result?;}
        if let Some(parameters) = group.capture_transition {
            self.capture_menu(parameters)?;
        }
        Ok(true)
    }
    fn capture_menu(&mut self, [x, y, ticks]: [i16; 3]) -> Result<()> {
        let root: Value = serde_json::from_slice(
            &self
                .host
                .query_engine(self.client, EngineQuery::Layer(self.menu_root()))?,
        )?;
        let source = [
            -root["rect"][0].as_f64().context("Circus capture root x")?,
            -root["rect"][1].as_f64().context("Circus capture root y")?,
            1280.,
            720.,
        ];
        self.host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"canvases":[{"op":"create","id":self.menu_capture(),"width":1280,"height":720},{"op":"clear","id":self.menu_capture(),"color":[0,0,0,0]},{"op":"copy_subtree","source":self.menu_root(),"destination":self.menu_capture(),"rect":source,"position":[0,0]},{"op":"present_isolated","id":self.menu_capture(),"layer":self.menu_capture_layer(),"order":100,"parent":SCENE_ROOT,"position":[x,y]}]}))?)?;
        let mut layers = vec![json!({"op":"set_visible","id":self.menu_root(),"visible":false})];
        layers.extend(capture_tracks_for(self.menu_capture_layer(),[x,y,ticks])?);
        self.submit(
            layers,
            vec![json!({"op":"set_enabled","id":self.menu_ui_root(),"enabled":false})],
        )
    }
    fn advance_menu_capture(&mut self) -> Result<()> {
        if !self
            .services
            .menu
            .as_ref()
            .is_some_and(|m| m.capture_visible)
        {
            return Ok(());
        }
        let status: crysirror_scene::layer::queued::SequenceStatus = serde_json::from_slice(
            &self
                .host
                .query_engine(self.client, EngineQuery::LayerSequence(self.menu_capture_layer()))?,
        )?;
        let operation = crysirror_scene::layer::queued::SequenceOperation::Advance {
            generation: status.generation,
            revision: status
                .revision
                .checked_add(1)
                .context("Circus capture queue revision")?,
            rate: 1.,
            fast: false,
        };
        self.submit(
            vec![json!({"op":"sequence","id":self.menu_capture_layer(),"operation":operation})],
            vec![],
        )?;
        let status: Value = serde_json::from_slice(
            &self
                .host
                .query_engine(self.client, EngineQuery::LayerSequence(self.menu_capture_layer()))?,
        )?;
        if status["complete"] == true {
            self.host.submit_engine(
                self.client,
                1,
                &serde_json::to_vec(&json!({"canvases":[{"op":"hide","layer":self.menu_capture_layer()}]}))?,
            )?;
            self.submit(
                vec![json!({"op":"set_visible","id":self.menu_root(),"visible":true})],
                vec![json!({"op":"set_enabled","id":self.menu_ui_root(),"enabled":true})],
            )?;
            self.services.menu.as_mut().unwrap().capture_visible = false;
        }
        Ok(())
    }
    pub(super) fn menu_animating(&mut self) -> Result<bool> {
        let Some(menu) = self.services.menu.as_ref().filter(|menu| menu.active) else {
            return Ok(false);
        };
        let ids = menu.animated_images.iter().copied().collect::<Vec<_>>();
        if self.menu_moving()? {
            return Ok(true);
        }
        for id in ids {
            let state: crysirror_scene::layer::queued::SequenceStatus = serde_json::from_slice(
                &self
                    .host
                    .query_engine(self.client, EngineQuery::LayerSequence(id))?,
            )?;
            if !state.complete {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub(super) fn menu_moving(&mut self) -> Result<bool> {
        let Some(menu) = self.services.menu.as_ref() else {
            return Ok(false);
        };
        if menu.capture_visible||menu.shifting {return Ok(true)}
        let status: Value = serde_json::from_slice(
            &self
                .host
                .query_engine(self.client, EngineQuery::UiMotion(self.menu_ui_root()))?,
        )?;
        Ok(status["complete"] != true)
    }
    pub(super) fn advance_menu(&mut self) -> Result<()> {
        self.advance_menu_capture()?;
        if self
            .services
            .menu
            .as_ref()
            .is_some_and(|m| m.capture_visible)
        {
            return Ok(());
        }
        let Some(mut menu) = self.services.menu.take() else {return Ok(())};
        let result=(|| {
        if !menu.active{return Ok(())}
        menu.ticks += 1;
        let mut layers = Vec::new();
        let mut ui = Vec::new();
        for (&id, &started) in &menu.scale_started {
            for property in ["scale_x", "scale_y"] {
                layers.push(json!({"op":"set_motion_elapsed","id":id,"property":property,"elapsed":menu.ticks.saturating_sub(started)}));
            }
        }
        for id in &menu.animated_images {
            let state: crysirror_scene::layer::queued::SequenceStatus = serde_json::from_slice(
                &self
                    .host
                    .query_engine(self.client, EngineQuery::LayerSequence(*id))?,
            )?;
            layers.push(json!({"op":"sequence","id":id,"operation":{"kind":"advance","generation":state.generation,"revision":state.revision.checked_add(1).context("Circus menu image revision")?,"rate":1.,"fast":false}}));
            //416190 writes444250 after448CE0 on every draw. This changes
            //current alpha only; the pulse queue and its retained origin stay.
            if self.services.window_alpha_layers.contains(id) {
                layers.push(json!({"op":"sequence","id":id,"operation":{"kind":"set","channels":[3],"values":[self.services.window_image_native_alpha],"retained":false}}));
            }
        }
        self.submit(layers, ui)?;
        if menu.shifting {
            let id=self.menu_root();
            let state:crysirror_scene::layer::queued::SequenceStatus=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::LayerSequence(id))?)?;
            self.submit(vec![json!({"op":"sequence","id":id,"operation":{"kind":"advance","generation":state.generation,"revision":state.revision.checked_add(1).context("Circus group shift revision")?,"rate":if self.native_effect_fast(){2.}else{1.},"fast":false}})],vec![])?;
            let state:crysirror_scene::layer::queued::SequenceStatus=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::LayerSequence(id))?)?;
            let position=[state.values[0],state.values[1]];
            self.submit(vec![],vec![json!({"op":"set_rect","id":self.menu_ui_root(),"rect":[position[0],position[1],1280,720]})])?;
            if state.complete {menu.shifting=false;if let Some(action)=menu.shift_action.take().filter(|action|!action.is_empty()){menu.pending_action=Some(action);}}
        }
        self.update_native_slider_paints(&menu)?;
        Ok(())
        })();
        self.services.menu=Some(menu);result
    }
    pub(super) fn menu_events(&mut self)->Result<Vec<Value>> {
        Ok(serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::UiSubtreeEvents(self.menu_ui_root()))?)?)
    }
    pub(super) fn process_menu_fallback(&mut self)->Result<Option<String>> {self.with_menu_scope(0,|bridge|bridge.tick_menu())}
    pub(super) fn tick_menu(&mut self) -> Result<Option<String>> {
        if self.services.menu_scope==0 {
            for scope in 1..3 {self.with_menu_scope(scope,|bridge| {
                if bridge.services.menu.is_some(){if bridge.tick_menu()?.is_some(){bridge.resume_menu()?;}}
                Ok(())
            })?;}
        }
        if self.poll_menu_gesture()? {return Ok(None)}
        let Some(mut menu)=self.services.menu.take()else{return Ok(None)};
        let events: Vec<Value> =
            serde_json::from_slice(&self.host.query_engine(self.client, EngineQuery::UiSubtreeEvents(self.menu_ui_root()))?)?;
        let result = self.tick_menu_inner(&mut menu, &events, false);
        self.services.menu = Some(menu);
        result
    }
    pub(super) fn tick_menu_events(&mut self, events: &[Value]) -> Result<Option<String>> {
        let Some(mut menu) = self.services.menu.take() else {
            return Ok(None);
        };
        let result = self.tick_menu_inner(&mut menu, events, true);
        self.services.menu = Some(menu);
        result
    }
    fn tick_menu_inner(
        &mut self,
        menu: &mut NativeMenu,
        events: &[Value],
        _panel: bool,
    ) -> Result<Option<String>> {
        if menu.capture_visible {
            return Ok(None);
        }
        // Native policy consumes owner-scoped Ui pointer/key receipts. The
        //VM InputFrame has already been consumed by the shared input router.
        let keyboard=self.menu_keyboard_begin(menu,events)?;
        let events=keyboard.events.as_slice();
        if menu.pending_action.is_none()&&!self.poll_native_drag(menu)? {
            let activated=keyboard.activation.map(|n|u64::from(self.menu_base()+0x8000+n as u32)).into_iter().collect::<BTreeSet<_>>();
            let mut layers = Vec::new();
            for n in 0..menu.group.buttons.len() {
                let widget = self.menu_base() + 0x8000 + n as u32;
                let status: Value = serde_json::from_slice(
                    &self
                        .host
                        .query_engine(self.client, EngineQuery::UiInteraction(widget))?,
                )?;
                let hovered=keyboard.selected==Some(n);
                let pressed=keyboard.pressed==Some(n);
                let slider=menu.group.buttons[n].slider.is_some();
                let activated=if slider {self.poll_native_slider(menu,n,&status,events)?}else{activated.contains(&u64::from(widget))};
                let button=&menu.group.buttons[n];
                let state = if pressed {
                    2
                } else if hovered {
                    1
                } else {
                    99
                };
                if state != menu.states[n] {
                    if menu.states[n] == 99 && state != 99 && button.sounds[1] != 0 {
                        menu.pending_sounds.push(button.sounds[1].into());
                    }
                    if menu.states[n] == 99
                        && state != 99
                        && !button.alternate_actions[1].is_empty()
                    {
                        menu.pending_sound_assets
                            .push(button.alternate_actions[1].clone());
                    }
                    let transition = (menu.states[n] == 99) != (state == 99);
                    for (k, part) in button.images.iter().enumerate() {
                        if self.native_part_image(part).is_none()
                            || part.source[2] == 0
                            || part.source[3] == 0
                        {
                            continue;
                        }
                        let id = self.menu_base() + 4096 + n as u32 * 128 + k as u32;
                        layers
                            .push(json!({"op":"set_visible","id":id,"visible":crate::machine::menu_ops::image_visible(part.state,state)}));
                        if transition && menu.group.behavior_flags.contains(&242) {
                            let (from,to)=if state==99{(120.,100.)}else{(100.,120.)};
                            layers.push(json!({"op":"sequence","id":id,"operation":{"kind":"finish","cancel":true,"lanes":[4]}}));
                            layers.push(json!({"op":"sequence","id":id,"operation":{"kind":"set","channels":[4,5],"values":[from,from],"retained":true}}));
                            layers.push(json!({"op":"sequence","id":id,"operation":{"kind":"enqueue","lane":4,"track":{"kind":"linear","channels":[4,5],"target":[to,to],"ticks":15,"references":[4,4],"loop_origins":[4,5],"delta":"current_at_zero_bank","complete_first":true,"complete_zero":false,"fast":true,"skip_zero":true}}}));
                        }
                    }
                    for (k, part) in button.texts.iter().enumerate() {
                        layers.push(json!({"op":"set_visible","id":self.menu_base()+0x8000+n as u32*128+k as u32,"visible":part.state==0||part.state==state}));
                    }
                    menu.states[n] = state;
                }
                if activated && !button.behavior_flags.contains(&210) && !button.action.is_empty() && !button.action.starts_with('-') {
                    ensure!(
                        menu.pending_action.is_none(),
                        "Circus ambiguous menu activation"
                    );
                    menu.pending_action = Some(button.action.clone());
                    menu.activation_ordinal = Some(n);
                    if button.sounds[0] != 0 {
                        menu.pending_sounds.push(button.sounds[0].into());
                    }
                    if !button.alternate_actions[0].is_empty() {
                        menu.pending_sound_assets
                            .push(button.alternate_actions[0].clone());
                    }
                }
            }
            ensure!(
                menu.pending_sounds.len() <= 32,
                "Circus menu sound queue limit"
            );
            self.submit(layers, vec![])?;
            self.update_native_slider_paints(menu)?;
        }
        if let Some(&sound) = menu.pending_sounds.first() {
            let Some(name) = self.sound_name(sound)? else {
                return Ok(None);
            };
            if !self.play_audio(2, &name, false, false, 0, None)? {
                return Ok(None);
            }
            menu.pending_sounds.remove(0);
            if !menu.pending_sounds.is_empty() {
                return Ok(None);
            }
        }
        if let Some(name) = menu.pending_sound_assets.first() {
            if !menu.sound_asset_started {
                ensure!(
                    NativeHost::emit(self, &Event::StopAudio { channel: 8 })?,
                    "Circus menu voice stop must be synchronous"
                );
                menu.sound_asset_started = true;
            }
            if !self.play_audio(8, name, true, false, 0, None)? {
                return Ok(None);
            }
            menu.pending_sound_assets.remove(0);
            menu.sound_asset_started = false;
            if !menu.pending_sound_assets.is_empty() {
                return Ok(None);
            }
        }
        if let Some(action) = menu.pending_action.take() {
            self.submit(
                vec![],
                vec![json!({"op":"set_enabled","id":self.menu_ui_root(),"enabled":false})],
            )?;
            menu.active = false;
            return Ok(Some(action));
        }
        self.menu_keyboard_end(menu)?;
        Ok(None)
    }
}

#[cfg(test)]
fn capture_tracks(parameters:[i16;3])->Result<Vec<Value>> {capture_tracks_for(CAPTURE_LAYER,parameters)}
fn capture_tracks_for(layer:u32,[x, y, ticks]: [i16; 3]) -> Result<Vec<Value>> {
    let bindings=[("scale_x",0), ("scale_y",1),("x",2),("y",3)].into_iter().map(|(property,channel)|json!({"layer":layer,"channel":channel,"target":{"kind":"motion","property":property},"map":if channel<2{json!({"kind":"affine","scale":0.01,"offset":0})}else{json!({"kind":"identity"})}})).collect::<Vec<_>>();
    let scale = json!({"kind":"linear","channels":[0,1],"target":[100,100],"ticks":ticks.unsigned_abs(),"velocity_divisor":ticks as f32,"references":[0,0],"loop_origins":[0,0],"delta":"current_at_zero_bank","complete_first":true});
    let movement = json!({"kind":"linear","channels":[2,3],"target":[0,0],"ticks":ticks.unsigned_abs(),"velocity_divisor":ticks as f32,"references":[2,3],"loop_origins":[2,3],"delta":"current_at_zero_bank","complete_zero":true});
    Ok(vec![
        json!({"op":"begin_sequence","id":layer,"spec":{"values":[0,0,x,y],"bindings":bindings}}),
        json!({"op":"sequence","id":layer,"operation":{"kind":"enqueue","lane":0,"track":scale}}),
        json!({"op":"sequence","id":layer,"operation":{"kind":"enqueue","lane":1,"track":movement}}),
    ])
}
#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_scene::layer::{LayerCommand, LayerSpec, LayerSystem, queued::SequenceOperation};
    #[test]
    fn native_capture_uses_activation_tick_and_early_uniform_snap() {
        let client = ClientId(7);
        let mut layers = LayerSystem::default();
        let layer:LayerSpec=serde_json::from_value(json!({"id":CAPTURE_LAYER,"rect":[0,0,1280,720],"paint":{"kind":"solid","color":[0,0,0,0]}})).unwrap();
        layers.install(client, &[layer]).unwrap();
        let commands = capture_tracks([100, 40, 4])
            .unwrap()
            .into_iter()
            .map(serde_json::from_value)
            .collect::<std::result::Result<Vec<LayerCommand>, _>>()
            .unwrap();
        layers.apply_batch(client, &commands).unwrap();
        for (scale, x) in [(0., 100.), (0.25, 75.), (0.5, 50.), (1., 0.)] {
            let status = layers
                .sequence_status(client, CAPTURE_LAYER)
                .unwrap()
                .unwrap();
            layers
                .apply_batch(
                    client,
                    &[LayerCommand::Sequence {
                        id: CAPTURE_LAYER,
                        operation: SequenceOperation::Advance {
                            generation: status.generation,
                            revision: status.revision + 1,
                            rate: 1.,
                            fast: false,
                        },
                    }],
                )
                .unwrap();
            let layer = layers.get(client, CAPTURE_LAYER).unwrap();
            assert_eq!(layer.transform.scale_x, scale);
            assert_eq!(layer.transform.scale_y, scale);
            assert_eq!(layer.rect[0], x);
        }
        assert!(
            layers
                .sequence_status(client, CAPTURE_LAYER)
                .unwrap()
                .unwrap()
                .complete
        );
    }
}

fn menu_image_animations(
    group: &Group,
    images: &BTreeMap<u8, UploadedImage>,
    layers: &mut Vec<Value>,
    base:u32,
    references:&BTreeMap<String,[f32;2]>,
) -> Result<BTreeSet<u32>> {
    let mut actors:BTreeSet<u32>=layers.iter().filter(|command|command["op"]=="begin_sequence").filter_map(|command|command["id"].as_u64().and_then(|id|u32::try_from(id).ok())).filter(|id|*id!=base+0xfffe).collect();
    for (n, args) in group.image_animations.iter().enumerate() {
        let Some(target)=group.image_animation_owners.get(n).copied().or_else(||group.image_animation_targets.get(n).copied().map(|index|crate::machine::UiRecordTarget::Image{button:None,index})) else {continue};
        let part=match target {
            crate::machine::UiRecordTarget::Image{button:None,index}=>group.images.get(index),
            crate::machine::UiRecordTarget::Image{button:Some(button),index}=>group.buttons.get(button).and_then(|button|button.images.get(index)),
            _=>None,
        }.context("Circus animated native image record")?;
        let image=if part.binding_declared {part.resource_binding.as_deref().and_then(|binding|images.values().find(|image|image.path.relative()==binding))}else{images.get(&part.slot)};
        let Some(image)=image else{continue};
        let id=super::control_records::record_layer(target,base);
        if actors.insert(id) {
            layers.push(json!({"op":"begin_sequence","id":id,"spec":{"values":[0.,part.position[0] as f32,part.position[1] as f32,1.],"bindings":[{"layer":id,"channel":1,"target":{"kind":"motion","property":"x"}},{"layer":id,"channel":2,"target":{"kind":"motion","property":"y"}},{"layer":id,"channel":3,"target":{"kind":"motion","property":"opacity"},"map":{"kind":"clamp","low":0.,"high":1.,"scale":255.}}]}}));
        }
        if args[1] == 768 {
            let mut from = part.position;
            match args[2] {
                0 => from[0] -= 100,
                1 => from[0] += 100,
                2 => from[1] -= 100,
                3 => from[1] += 100,
                _ => {}
            }
            layers.push(json!({"op":"sequence","id":id,"operation":{"kind":"set","channels":[1,2,3],"values":[from[0],from[1],0.],"retained":true}}));
            layers.push(json!({"op":"sequence","id":id,"operation":{"kind":"enqueue","lane":0,"bank":u8::try_from(args[3]).ok(),"track":{"kind":"linear","channels":[1,2],"target":part.position,"ticks":30,"references":[1,2],"loop_origins":[1,2],"delta":"retained","complete_first":false,"complete_zero":true,"fast":true}}}));
            layers.push(json!({"op":"sequence","id":id,"operation":{"kind":"enqueue","lane":1,"bank":u8::try_from(args[3]).ok(),"track":{"kind":"linear","channels":[3],"target":[1.],"ticks":30,"references":[3],"loop_origins":[3],"delta":"retained","complete_first":false,"complete_zero":true,"fast":true}}}));
        } else {
            let step = match args[0] & 15 {
                0 => [args[2] as f32, 0.],
                1 => [0., args[2] as f32],
                6 => [0., 1024.],
                _ => continue,
            };
            let frames=if args[3]<0{1}else{args[3] as u32};
            let reference=references.get(image.path.relative()).context("Circus animation texture reference extent")?;
            let source=[f32::from(part.source[0]),f32::from(part.source[1]),(i32::from(part.source[0])+i32::from(part.source[2])) as f32,(i32::from(part.source[1])+i32::from(part.source[3])) as f32];
            // Rebuild only the sequence declaration before any native tick. UV
            // sampling and counter/activation policy remain shared services.
            let mut frame_channel = 0;
            if let Some(command) = layers
                .iter_mut()
                .find(|v| v["op"] == "begin_sequence" && v["id"] == id)
            {
                let values = command["spec"]["values"].as_array_mut().unwrap();
                frame_channel = u8::try_from(values.len())?;
                values.push(json!(-1.));
                let bindings=command["spec"]["bindings"].as_array_mut().unwrap();
                bindings.retain(|binding|!matches!(binding["target"]["kind"].as_str(),Some("texel_image_uv"|"image_uv")));
                bindings.push(json!({"layer":id,"channel":frame_channel,"target":{"kind":"texel_image_uv","allocation":image.size,"reference":reference,"source":source,"step":step,"terminal_frame":frames,"active_lane":2}}));
            }
            layers.push(json!({"op":"sequence","id":id,"operation":{"kind":"enqueue","lane":2,"track":{"kind":"frames","channel":frame_channel,"frames":frames,"hold_ticks":2*i32::from(args[1]),"looping":args[0]&0xf00!=0,"capture":false}}}));
        }
    }
    Ok(actors)
}

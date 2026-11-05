//! Client-owned Layer tracks implement native sprite commands. The adapter
//! supplies native render ticks; the shared engine samples all interpolation.
use super::*;
use crate::machine::visual_ops::{VisualProperty, VisualRequest, object_resource, resource_object};

#[derive(Default)]
pub(super) struct NativeVisual {
    roots: BTreeSet<u16>,
    tracks: BTreeSet<u32>,
    pub(super) filters:BTreeMap<u16,Value>,
    blur:Option<(crysirror_scene::animation::ScalarTween,u64,i16)>,
}
impl NativeVisual {
    pub(super) fn mark_restore_roots(&mut self,objects:impl Iterator<Item=u16>){self.roots.extend(objects);}
    pub(super) fn adopt_restore(&mut self, objects: impl IntoIterator<Item = u16>) {
        self.tracks.clear();
        self.filters.clear();self.blur=None;
        self.roots = objects.into_iter().collect();
    }
    pub(super) fn roots(&self) -> impl Iterator<Item = u16> + '_ {
        self.roots.iter().copied()
    }
}
pub(super) fn content_layer(object: u16) -> u32 {
    0x3a01_0000 + u32::from(object)
}
pub(super) fn painter_order(object: u16, priority: i16) -> [i32; 2] {
    if object < 10 {
        [0, i32::from(object)]
    } else if object <= 63 {
        [
            1,
            i32::from(priority.clamp(0, 100)) * 64 + i32::from(object),
        ]
    } else {
        [2, i32::from(object)]
    }
}
pub(super) fn object_layer(object: u16) -> u32 {
    object_resource(object).map_or(0x3a00_0000 + u32::from(object), |slot| {
        0x3501_0000 + u32::from(slot)
    })
}
impl Bridge<'_> {
    pub(super) fn visual_root(&mut self, object: u16) -> Result<u32> {
        ensure!(object < 150, "Circus native object index");
        self.root()?;
        let id = object_layer(object);
        if !self.services.visual.roots.contains(&object) {
            self.submit(vec![json!({"op":"put","layer":{"id":id,"parent":SCENE_ROOT,"visible":!self.services.effect_control.particle_wrapper(object),"opacity":match object {88=>if self.services.native_status_visible{255}else{0},75|76=>if self.services.native_window_visible{255}else{0},_=>255},"rect":[0,0,1280,720],"paint":{"kind":"solid","color":[0,0,0,0]},"order":painter_order(object,50)}}),json!({"op":"put","layer":{"id":content_layer(object),"parent":id,"rect":[0,0,1280,720],"paint":{"kind":"solid","color":[0,0,0,0]},"order":[0,0]}})],vec![])?;
            self.services.visual.roots.insert(object);
        }
        Ok(id)
    }
    fn visual_track(&mut self, id: u32, property: &str, to: f32, ticks: u32) -> Result<()> {
        self.submit(vec![json!({"op":"begin_motion","id":id,"track":{"property":property,"to":to,"duration":ticks,"external":true}})],vec![])?;
        self.services.visual.tracks.insert(id);
        Ok(())
    }
    pub(super) fn native_visual(&mut self, request: &VisualRequest) -> Result<bool> {
        match request {
            VisualRequest::Shader{pass}=>{
                self.services.visual.blur=None;
                self.set_native_filter(*pass,0.)?;
            },
            VisualRequest::Blur{from,to,ticks}=>{
                if *ticks==0 {
                    ensure!(from==to,"Circus native zero-duration blur has an undefined reciprocal step");
                    self.set_native_filter(4,f32::from(*to))?;self.services.visual.blur=None;
                }else{
                    ensure!(*from>0&&*to>0,"Circus native reciprocal blur has a nonfinite endpoint");
                    let curve=crysirror_scene::animation::ScalarTween::new(1./f32::from(*from),1./f32::from(*to),0,u64::from(*ticks),crysirror_scene::layer::Easing::Linear)?;
                    self.set_native_filter(4,f32::from(*from))?;
                    self.services.visual.blur=Some((curve,0,*to));
                }
            },
            VisualRequest::Finish => {
                let ids = self
                    .services
                    .visual
                    .tracks
                    .iter()
                    .copied()
                    .collect::<Vec<_>>();
                self.submit(
                    ids.into_iter()
                        .map(|id| json!({"op":"end_motion","id":id,"finish":true}))
                        .collect(),
                    vec![],
                )?;
                self.services.visual.tracks.clear();
                if let Some((_,_,target))=self.services.visual.blur.take(){self.set_native_filter(4,f32::from(target))?;}
            }
            VisualRequest::Position {
                object,
                position,
                cancel,
            } => {
                let id = self.visual_root(*object)?;
                let mut commands = Vec::new();
                if *cancel {
                    commands.push(json!({"op":"end_motion","id":id,"finish":true}));
                    self.services.visual.tracks.remove(&id);
                }
                let layer: Value = serde_json::from_slice(
                    &self
                        .host
                        .query_engine(self.client, EngineQuery::Layer(id))?,
                )?;
                let mut rect = layer["rect"]
                    .as_array()
                    .context("Circus native retained rectangle")?
                    .clone();
                rect[0] = json!(position[0]);
                rect[1] = json!(position[1]);
                commands.push(json!({"op":"set_rect","id":id,"rect":rect}));
                self.submit(commands, vec![])?;
                if let Some(resource) = object_resource(*object) {
                    if let (Ok(x), Ok(y)) = (i16::try_from(position[0]), i16::try_from(position[1]))
                    {
                        self.services.positions.insert(resource, [x, y]);
                    }
                }
            }
            VisualRequest::Move {
                object,
                position,
                ticks,
                cancel,
            } => {
                let id = self.visual_root(*object)?;
                if *cancel {
                    self.submit(
                        vec![json!({"op":"end_motion","id":id,"finish":true})],
                        vec![],
                    )?;
                }
                self.visual_track(id, "x", position[0] as f32, *ticks)?;
                self.visual_track(id, "y", position[1] as f32, *ticks)?;
            }
            VisualRequest::Priority { object, priority } => {
                let id = self.visual_root(*object)?;
                self.submit(
                    vec![
                        json!({"op":"set_order","id":id,"order":painter_order(*object,*priority)}),
                    ],
                    vec![],
                )?;
            }
            VisualRequest::Track {
                object,
                property,
                value,
                ticks,
            } => {
                let id = self.visual_root(*object)?;
                match property {
                    VisualProperty::Scale => {
                        self.visual_track(id, "scale_x", *value, *ticks)?;
                        self.visual_track(id, "scale_y", *value, *ticks)?;
                    }
                    VisualProperty::Rotation => {
                        self.visual_track(id, "rotation_z", *value, *ticks)?
                    }
                    VisualProperty::Alpha => {
                        ensure!(
                            (0.0..=1.0).contains(value),
                            "Circus native float alpha range"
                        );
                        self.visual_track(
                            content_layer(*object),
                            "opacity",
                            *value * 255.,
                            *ticks,
                        )?;
                    }
                }
            }
        }
        Ok(true)
    }
    pub(super) fn advance_visual(&mut self) -> Result<()> {
        if let Some((curve,tick,target))=self.services.visual.blur.take(){
            let tick=tick.checked_add(1).context("Circus filter clock overflow")?;
            self.set_native_filter(4,if curve.complete(tick){f32::from(target)}else{1./curve.sample(tick)})?;
            if !curve.complete(tick){self.services.visual.blur=Some((curve,tick,target));}
        }
        let ids = self
            .services
            .visual
            .tracks
            .iter()
            .copied()
            .collect::<Vec<_>>();
        for id in ids {
            let status: Value = serde_json::from_slice(
                &self
                    .host
                    .query_engine(self.client, EngineQuery::LayerMotion(id))?,
            )?;
            let tracks = status["tracks"]
                .as_array()
                .context("Circus shared motion status")?;
            let mut commands = Vec::new();
            for track in tracks {
                if track["external"] == true && track["complete"] != true {
                    let elapsed = track["elapsed"].as_u64().context("Circus motion elapsed")?;
                    let duration = track["duration"]
                        .as_u64()
                        .context("Circus motion duration")?;
                    commands.push(json!({"op":"set_motion_elapsed","id":id,"property":track["property"],"elapsed":elapsed.saturating_add(1).min(duration)}));
                }
            }
            if commands.is_empty() {
                self.services.visual.tracks.remove(&id);
            } else {
                self.submit(commands, vec![])?;
            }
        }
        Ok(())
    }
    pub(super) fn native_animation_wait_ready(&mut self) -> Result<bool> {
        //433E50 accumulates v106 from40EE60 only for these150 wrapper
        //indices.70..95 and the separately updated overlay renderer150
        //never contribute.40EE60 returns0 for paused/particle wrappers.
        let eligible = |object: u16| object < 150 && (object < 70 || object >= 96);
        let runnable = |object: u16| eligible(object)
            && !self.services.primary.paused.contains(&object)
            && !self.services.effect_control.particle_wrapper(object);
        let objects = self.services.visual.roots().filter(|object|runnable(*object)).collect::<BTreeSet<_>>();
        if !self.framebuffer_complete() || self.services.comments.animation_wait_busy() {
            return Ok(false);
        }
        if self.primary_sprite_states()?.iter().any(|(object,state)|objects.contains(object)&&!state.complete) {
            return Ok(false);
        }
        //447FF0 auxiliary scale/pivot queues belong to the same448CE0
        //renderer, hence use exactly the same native wrapper selection.
        let views=self.services.effect_control.filtered_view_layers(&objects);
        for id in views {
            let state:crysirror_scene::layer::queued::SequenceStatus=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::LayerSequence(id))?)?;
            if !state.complete{return Ok(false)}
        }
        //The compatibility blur affects native0..59, and408A70's global
        //transition above also contributes independently of sprite indices.
        if self.services.visual.blur.is_some(){return Ok(false)}
        for id in self.services.visual.tracks.iter().copied().filter(|id|objects.iter().any(|object|*id==object_layer(*object)||*id==content_layer(*object))).collect::<Vec<_>>() {
            let status:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::LayerMotion(id))?)?;
            if status["complete"]!=true{return Ok(false)}
        }
        Ok(true)
    }
    pub(super) fn visual_complete(&mut self) -> Result<bool> {
        if !self.framebuffer_complete(){return Ok(false)}
        if self.primary_sprite_states()?.iter().any(|(_,state)|!state.complete){return Ok(false)}
        if !self.stage_views_complete()?{return Ok(false)}
        if self.services.visual.blur.is_some(){return Ok(false)}
        for id in &self.services.visual.tracks {
            let status: Value = serde_json::from_slice(
                &self
                    .host
                    .query_engine(self.client, EngineQuery::LayerMotion(*id))?,
            )?;
            if status["complete"] != true {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub(super) fn resource_parent(&mut self, slot: u8) -> Result<u32> {
        match resource_object(slot) {
            Some(object) => {
                self.visual_root(object)?;
                Ok(content_layer(object))
            }
            None => {
                self.root()?;
                Ok(SCENE_ROOT)
            }
        }
    }
}

impl Bridge<'_> {
    pub(super) fn filter_for_slot(&self,slot:u8)->Value {
        resource_object(slot).and_then(|object|self.services.visual.filters.get(&object)).cloned().unwrap_or_else(||json!({"encoded_sampling":true}))
    }
    fn set_native_filter(&mut self,pass:u8,frequency:f32)->Result<()> {
        let filter=native_filter(pass,frequency)?;let mut commands=Vec::new();
        for object in 0..60 {
            self.services.visual.filters.insert(object,filter.clone());
            if let Some(slot)=object_resource(object){if self.services.displayed.contains(&slot){commands.push(json!({"op":"set_filter","id":0x3700_0000+u32::from(slot),"filter":filter}));}}
        }
        self.submit(commands,vec![])
    }
}
pub(super) fn native_filter(pass:u8,frequency:f32)->Result<Value>{
    Ok(match pass {
        0=>json!({"encoded_sampling":true}),
        1=>json!({"mono":1.,"luma_weights":[0.299,0.587,0.114],"ignore_tint_alpha":true,"encoded_sampling":true}),
        2=>json!({"invert":1.,"rgb_alpha_power":1.,"ignore_tint_alpha":true,"encoded_sampling":true}),
        3=>json!({"distortion":{"kind":"quantized","steps":[90.,90.],"phase":[0.5,0.5]},"ignore_tint_alpha":true,"encoded_sampling":true}),
        4 if frequency==0.||frequency==1000.=>json!({"encoded_sampling":true}),
        4=>{ensure!(frequency.is_finite()&&frequency!=0.,"Circus native blur sample frequency");json!({"distortion":{"kind":"nine_tap","offset":[1./frequency,1./frequency],"center_weight":0.2,"neighbor_weight":0.1},"ignore_tint_alpha":true,"encoded_sampling":true})},
        _=>bail!("Circus native shader pass {pass}"),
    })
}

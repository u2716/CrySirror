// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native Print/overlap dependency ordering. Rasterisation, dynamic patterns,
//! weighted mixing and publication resources are all engine services.
use crate::native_effects::{FadeInState, LayerGreyState, SunaState};
use anyhow::{Context,Result, bail, ensure};
use crysirror_core::{
    image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource},
    service::{
        HostServices, PatternJob, PatternOutput, PatternProcessState, PatternRequest,
        PatternResource,
    },
    state::ClientId,
};
use crysirror_image::temporal_pattern::PatternRng;
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};
mod staff;
#[derive(Clone)]
pub enum NativeFrameEffect {
    Slot(i32),
    Underlay{slot:i32,layer:crysirror_core::image_processing::ImageCompositeLayer},
    /// Painter boundary before native DRAW's window pass. Retains the raw
    /// effect background for following PRINT/window composition, without pixels.
    MessageBackground,
    /// A native picture/fill at its original painter position. All pixel work is shared.
    Paint(crysirror_core::image_processing::ImageCompositeLayer),
    Move(MoveFrame),
    Scroll(ScrollLayerFrame),
    Fusuma(crate::native_effects_fusuma::FusumaFrame),
    Shake(crate::native_effects_shake::ShakeFrame),
    Flash(crate::native_effects_flash::FlashFrame),
    Suna(SunaState),
    Fade(FadeInState),
    Grey(LayerGreyState),
    Nega(LayerGreyState),
    Chara(CharaTransFrame),
    Picture(crate::native_picture_effect::PictureEffectFrame),
    Mosaic(crate::native_effects_mosaic::MosaicState),
    Lamp(OldLampFrame),
    Bura(crate::native_bura::BuraState),
}
#[derive(Clone)]
pub struct MoveFrame {
    pub(crate) geometry_assigned: bool,
    pub(crate) source_clip:Option<[i32;4]>,
    pub slot: i32,
    pub state: crate::native_move::NativeMoveState,
    pub source: Option<Arc<dyn ImageResource>>,
    pub math: crysirror_core::image_processing::ImageCompositeMath,
    pub visible: bool,
    ///0 uses common native picture geometry/counts without MOVE's endpoint
    /// source-copy/NOP transition;52 keeps the real MOVE lifecycle.
    pub native_command: i32,
}
impl MoveFrame {
    pub(crate) fn command(&self) -> i32 {
        if self.native_command == 0 {
            0
        } else {
            self.state.command()
        }
    }
    pub(crate) fn count_up(&mut self, steps: u32) {
        if self.native_command == 0 {
            if self.state.count < self.state.frames {
                self.state.count = self
                    .state
                    .count
                    .saturating_add(steps as i32)
                    .min(self.state.frames);
            }
        } else {
            for _ in 0..steps {
                self.state.count_up();
            }
        }
    }
    pub(crate) fn paint(&self) -> Result<Option<crysirror_core::image_processing::ImageCompositeLayer>> {
        if !self.visible {return Ok(None);}
        let Some(image) = self.source.as_ref() else{return Ok(None);};
        let [x,y,w,h]=self.state.rectangles.destination;
        let [sx,sy,_,_]=self.state.rectangles.source;
        if w<=0||h<=0{return Ok(None);}
        let (region,at)=if let Some(bounds)=self.source_clip {
            let Some(mapping)=crysirror_image::coverage_bounds::intersect_source_window([sx,sy,w,h],[x,y],bounds)? else{return Ok(None);};mapping
        }else{([sx,sy,w,h],[x,y])};
        Ok(Some(crysirror_core::image_processing::ImageCompositeLayer {
            input:Box::new(ImageInput::Resource(image.clone())),source:region,destination:at,
            clip:None,opacity:255,math:self.math,
        }))
    }
}
#[derive(Clone)]
pub struct ScrollLayerFrame {
    pub slot: i32,
    pub frame: crate::native_effects_scroll::ScrollFrame,
    pub normal: MoveFrame,
    pub ended: bool,
}
impl ScrollLayerFrame {
    pub(crate) fn calcu(&mut self) {
        self.normal.state.count = self.frame.state.count;
        self.normal.state.frames = self.frame.state.frames;
        self.normal.state.calcu(self.normal.geometry_assigned);
        if !self.ended {
            self.frame.state.calcu();
        }
    }
    pub(crate) fn count_up(&mut self, steps: u32) {
        if !self.ended && self.frame.state.count_up(steps) {
            self.ended = true;
        }
        self.normal.state.count = self.frame.state.count;
    }
}
/// The native state selects a shared immutable source and rectangle. Image
/// graph workers perform the complete raster operation, including clipping.
#[derive(Clone)]
pub struct CharaTransFrame {
    pub state: crate::native_chara_trans::CharaTransState,
    pub source: Option<Arc<dyn ImageResource>>,
    pub rectangles: crate::native_geometry::NativeRects,
}
#[derive(Clone)]
pub struct OldLampFrame {
    pub slot: i32,
    pub state: crate::native_old_lamp::OldLampState,
    pub source: Option<Arc<dyn ImageResource>>,
    pub destination: [i32; 2],
}
/// Build the original ascending-slot Print sequence. Picture/fill pixels are
/// prepared image leases or Solid DTOs; native policy does not inspect pixels.
/// Calcu still runs over the entire resulting plan before the first Print.
pub fn ordered_frame_plan(
    effects: &BTreeMap<i32, NativeFrameEffect>,
    visible: &std::collections::BTreeSet<i32>,
    underlays: &BTreeMap<i32, crysirror_core::image_processing::ImageCompositeLayer>,
) -> Vec<NativeFrameEffect> {
    let slots: std::collections::BTreeSet<_> =
        effects.keys().chain(underlays.keys()).copied().collect();
    let mut plan = Vec::new();
    for slot in slots {
        let terminal_fill = matches!(effects.get(&slot),Some(NativeFrameEffect::Fade(state)) if state.command()==54);
        if let Some(layer) = underlays.get(&slot).filter(|_| !terminal_fill) {
            plan.push(NativeFrameEffect::Underlay{slot,layer:layer.clone()});
        }
        if let Some(effect) = effects.get(&slot) {
            if visible.contains(&slot)
                || matches!(
                    effect,
                    NativeFrameEffect::Lamp(_)
                        | NativeFrameEffect::Move(_)
                        | NativeFrameEffect::Scroll(_)
                        | NativeFrameEffect::Shake(_)
                        | NativeFrameEffect::Picture(_)
                )
            {
                if matches!(effect,NativeFrameEffect::Chara(_)|NativeFrameEffect::Fusuma(_)){plan.push(NativeFrameEffect::Slot(slot));}
                plan.push(effect.clone());
            }
        }
    }
    plan
}
pub(crate) fn count_effect_states(effects: &mut BTreeMap<i32, NativeFrameEffect>, count: u32) {
    if count == 0 {
        return;
    }
    effects.retain(|_, effect| match effect {
        NativeFrameEffect::Slot(_)|NativeFrameEffect::Underlay{..}|NativeFrameEffect::MessageBackground | NativeFrameEffect::Paint(_) => {
            unreachable!("paint DTO in native state map")
        }
        NativeFrameEffect::Move(frame) => {
            frame.count_up(count);
            true
        }
        NativeFrameEffect::Scroll(frame) => {
            frame.count_up(count);
            true
        }
        NativeFrameEffect::Shake(frame) => !frame.state.count_up(count),
        NativeFrameEffect::Flash(frame) => {
            frame.state.count_up(count);
            true
        }
        NativeFrameEffect::Fusuma(frame) => {
            frame.state.count_up(count);
            true
        }
        NativeFrameEffect::Suna(state) => {
            if state.count < state.parameters.frames {
                state.count = state
                    .count
                    .saturating_add(count as i32)
                    .min(state.parameters.frames);
            }
            true
        }
        NativeFrameEffect::Grey(state) | NativeFrameEffect::Nega(state) => {
            if state.count < state.frames {
                state.count = state.count.saturating_add(count as i32).min(state.frames);
            }
            true
        }
        NativeFrameEffect::Lamp(frame) => {
            frame.state.count = frame.state.count.saturating_add(count as i32).min(999999);
            true
        }
        NativeFrameEffect::Bura(state) => {
            if state.count < state.frames {
                state.count = state.count.saturating_add(count as i32).min(state.frames);
            }
            true
        }
        NativeFrameEffect::Mosaic(state) => {
            state.count_up(count);
            true
        }
        NativeFrameEffect::Picture(frame) => {
            frame.state.count_up(count);
            true
        }
        NativeFrameEffect::Chara(frame) => {
            if frame.state.count < frame.state.frames {
                frame.state.count = frame
                    .state
                    .count
                    .saturating_add(count as i32)
                    .min(frame.state.frames);
            }
            true
        }
        NativeFrameEffect::Fade(state) => {
            if state.count < state.frames {
                state.count = state.count.saturating_add(count as i32).min(state.frames);
                state.fade_out || state.count != state.frames
            } else {
                true
            }
        }
    });
}
fn calcu_effect(effect: &mut NativeFrameEffect, random: &mut PatternRng,rain:&mut Option<crate::native_rain::RainClass>,grass:&mut Option<crate::native_grass_plane::GrassClass>,speed:&mut Option<crate::native_speed_line::SpeedLineClass>,bath:&mut Option<crate::native_bath_window::BathWindowClass>) -> Result<()> {
    if let NativeFrameEffect::Picture(frame)=effect {
        if let crate::native_picture_effect::PictureEffectKind::SpeedLine{parameters}=frame.state.kind {
            speed.as_mut().context("NNN SPEEDLINE class not initialized")?.calcu(&parameters,random)?;
        }
        if matches!(frame.state.kind,crate::native_picture_effect::PictureEffectKind::Rain{..}) {
            rain.as_mut().context("NNN RAIN class not initialized")?.calcu(frame.canvas,random)?;
        }
        if let crate::native_picture_effect::PictureEffectKind::GrassPlane{parameters}=frame.state.kind{
            grass.as_mut().context("NNN GRASSPLANE class not initialized")?.calcu(&parameters,random)?;
        }
    }
    match effect {
        NativeFrameEffect::Move(frame) => frame.state.calcu(frame.geometry_assigned),
        NativeFrameEffect::Picture(frame) => frame.calcu(random)?,
        NativeFrameEffect::Scroll(frame) => frame.calcu(),
        NativeFrameEffect::Shake(frame) => frame.state.calcu(),
        NativeFrameEffect::Lamp(frame) => frame.state.calcu(random)?,
        _ => {}
    }
    if let NativeFrameEffect::Picture(frame)=effect {
        if let crate::native_picture_effect::PictureEffectKind::BathWindow{parameters}=frame.state.kind {
            frame.state.count=0;
            let picture=frame.bank.picture(frame.slot);
            frame.bath_update=bath.as_mut().context("NNN BATHWINDOW constructor state absent")?.calcu(
                &parameters,picture.map(|p|p.picture.raw.size()).unwrap_or([1,1]),frame.canvas,frame.bath_mouse,
                picture.is_some_and(|p|p.picture.has_mask),random)?;
        }
    }
    Ok(())
}
pub(crate) fn calcu_effect_states(
    effects: &mut BTreeMap<i32, NativeFrameEffect>,
    random: &mut PatternRng,
) -> Result<()> {
    let mut bath=crate::native_bath_frames::class(effects.values()).map(|c|(*c).clone());
    let mut rain=frame_rain(effects.values()).map(|class|(*class).clone());
    let mut grass=frame_grass(effects.values()).map(|class|(*class).clone());
    let mut speed=frame_speed(effects.values()).map(|class|(*class).clone());
    for effect in effects.values_mut() {
        calcu_effect(effect, random,&mut rain,&mut grass,&mut speed,&mut bath)?;
    }
    crate::native_bath_frames::attach(effects.values_mut(),bath.map(Arc::new));
    attach_rain(effects.values_mut(),rain.map(Arc::new));
    attach_grass(effects.values_mut(),grass.map(Arc::new));
    attach_speed(effects.values_mut(),speed.map(Arc::new));
    calcu_rainbows(effects.values_mut())?;
    let points=effects.iter().filter_map(|(slot,e)|match e{NativeFrameEffect::Picture(f)=>Some((*slot,f.geometry())),NativeFrameEffect::Move(f)=>Some((*slot,f.state.clone())),NativeFrameEffect::Scroll(f)=>Some((*slot,f.normal.state.clone())),_=>None}).collect();
    for effect in effects.values_mut(){if let NativeFrameEffect::Picture(frame)=effect{frame.resolve_points(&points);}}
    Ok(())
}
fn prepare_calcu(
    effects: &mut [NativeFrameEffect],
    random: &mut PatternRng,
    calcu: bool,
    moves: &mut BTreeMap<i32, crate::native_move::NativeMoveState>,
    scrolls: &mut BTreeMap<i32, ScrollLayerFrame>,
    shakes: &mut BTreeMap<i32, crate::native_effects_shake::ShakeState>,
) -> Result<BTreeMap<i32, crate::native_old_lamp::OldLampState>> {
    let mut updates = BTreeMap::new();
    let original_bath=crate::native_bath_frames::class(effects.iter());
    let mut bath=original_bath.as_ref().map(|c|(**c).clone());
    let original_rain=frame_rain(effects.iter());
    let original_grass=frame_grass(effects.iter());
    let original_speed=frame_speed(effects.iter());
    let mut speed=original_speed.as_ref().map(|class|(**class).clone());
    let mut grass=original_grass.as_ref().map(|class|(**class).clone());
    let mut rain=original_rain.as_ref().map(|class|(**class).clone());
    for effect in effects.iter_mut() {
        if calcu {
            calcu_effect(effect, random,&mut rain,&mut grass,&mut speed,&mut bath)?;
        }
        if let NativeFrameEffect::Shake(frame) = effect {
            shakes.insert(frame.slot, frame.state.clone());
        }
        if let NativeFrameEffect::Scroll(frame) = effect {
            scrolls.insert(frame.slot, frame.clone());
        }
        if let NativeFrameEffect::Picture(frame) = effect {
            moves.insert(frame.slot, frame.geometry());
        }
        if let NativeFrameEffect::Move(frame) = effect {
            moves.insert(frame.slot, frame.state.clone());
        }
        if let NativeFrameEffect::Lamp(frame) = effect {
            updates.insert(frame.slot, frame.state.clone());
        }
    }
    crate::native_bath_frames::attach(effects.iter_mut(),if calcu {bath.map(Arc::new)}else{original_bath});
    attach_rain(effects.iter_mut(),if calcu {rain.map(Arc::new)}else{original_rain});
    attach_grass(effects.iter_mut(),if calcu {grass.map(Arc::new)}else{original_grass});
    attach_speed(effects.iter_mut(),if calcu {speed.map(Arc::new)}else{original_speed});
    if calcu{calcu_rainbows(effects.iter_mut())?;}
    for effect in effects.iter_mut(){if let NativeFrameEffect::Picture(frame)=effect{frame.resolve_points(moves);}}
    Ok(updates)
}
fn frame_speed<'a>(mut effects:impl Iterator<Item=&'a NativeFrameEffect>)->Option<Arc<crate::native_speed_line::SpeedLineClass>> {
    effects.find_map(|e|if let NativeFrameEffect::Picture(frame)=e {frame.speed_line_class.clone()}else{None})
}
fn attach_speed<'a>(effects:impl Iterator<Item=&'a mut NativeFrameEffect>,class:Option<Arc<crate::native_speed_line::SpeedLineClass>>) {
    for effect in effects {if let NativeFrameEffect::Picture(frame)=effect {if matches!(frame.state.kind,crate::native_picture_effect::PictureEffectKind::SpeedLine{..}){frame.speed_line_class=class.clone();}}}
}
fn frame_rain<'a>(mut effects:impl Iterator<Item=&'a NativeFrameEffect>)->Option<Arc<crate::native_rain::RainClass>> {
    effects.find_map(|e|if let NativeFrameEffect::Picture(f)=e {if matches!(f.state.kind,crate::native_picture_effect::PictureEffectKind::Rain{..}) {return f.rain_class.clone();}None}else{None})
}
fn attach_rain<'a>(effects:impl Iterator<Item=&'a mut NativeFrameEffect>,rain:Option<Arc<crate::native_rain::RainClass>>) {
    for effect in effects {if let NativeFrameEffect::Picture(frame)=effect {if matches!(frame.state.kind,crate::native_picture_effect::PictureEffectKind::Rain{..}){frame.rain_class=rain.clone();}}}
}
fn frame_grass<'a>(mut effects:impl Iterator<Item=&'a NativeFrameEffect>)->Option<Arc<crate::native_grass_plane::GrassClass>>{
    effects.find_map(|e|if let NativeFrameEffect::Picture(frame)=e{frame.grass_class.clone()}else{None})
}
fn attach_grass<'a>(effects:impl Iterator<Item=&'a mut NativeFrameEffect>,grass:Option<Arc<crate::native_grass_plane::GrassClass>>){
    for effect in effects{if let NativeFrameEffect::Picture(frame)=effect{if matches!(frame.state.kind,crate::native_picture_effect::PictureEffectKind::GrassPlane{..}){frame.grass_class=grass.clone();}}}
}
fn frame_palette(effects:&[NativeFrameEffect])->Option<Arc<crate::native_picture_rainbow::RainbowPalette>> {
    effects.iter().find_map(|effect|if let NativeFrameEffect::Picture(frame)=effect{frame.rainbow_palette.clone()}else{None})
}
fn calcu_rainbows<'a>(effects:impl Iterator<Item=&'a mut NativeFrameEffect>)->Result<()> {
    let mut frames=effects.filter_map(|effect|if let NativeFrameEffect::Picture(frame)=effect{Some(frame)}else{None}).collect::<Vec<_>>();
    let Some(old)=frames.iter().find_map(|frame|frame.rainbow_palette.clone()) else{return Ok(());};
    let mut palette=(*old).clone();
    for frame in &frames {
        if let crate::native_picture_effect::PictureEffectKind::Rainbow{parameters}=frame.state.kind {palette.calcu(&parameters)?;}
    }
    let palette=Arc::new(palette);
    for frame in &mut frames {
        if matches!(frame.state.kind,crate::native_picture_effect::PictureEffectKind::Rainbow{..}){frame.rainbow_palette=Some(palette.clone());}
    }Ok(())
}
fn picture_states(effects:&[NativeFrameEffect])->BTreeMap<i32,crate::native_picture_effect::PictureEffectState>{
    effects.iter().filter_map(|e|if let NativeFrameEffect::Picture(f)=e{Some((f.slot,f.state.clone()))}else{None}).collect()
}
enum Work {
    BathUpdates(crate::native_bath_updates::Preparation),
    Start,
    Staff(crate::native_staff_pages::Pages),
    Animation(crate::native_animation_session::FrameWork),
    FullAnime(crate::native_full_anime::FullAnimePreparation),
    LayerSequence(ImageJob,VecDeque<Vec<crysirror_core::image_processing::ImageCompositeLayer>>),
    Old(ImageJob),
    Base(ImageJob),
    Effect(ImageJob),
    Advance(PatternJob, SunaState),
    Pattern(PatternJob),
    Sample(PatternJob),
    CaptureInit(ImageJob, crate::native_picture_effect::PictureEffectFrame),
    WipeOld(ImageJob, crate::native_picture_effect::PictureEffectFrame, ImageInput),
    WipeNew(ImageJob, crate::native_picture_effect::PictureEffectFrame, Arc<dyn ImageResource>),
}
#[derive(Clone)]
pub struct NativeFrameResult {
    pub(crate) buffers: crate::native_capture_effect::FrameBuffers,
    pub old: Arc<dyn ImageResource>,
    pub target: Arc<dyn ImageResource>,
    pub random: PatternRng,
    pub shared_pattern: Option<Arc<dyn PatternResource>>,
    pub base: Arc<dyn ImageResource>,
    pub lamp_updates: BTreeMap<i32, crate::native_old_lamp::OldLampState>,
    pub move_updates: BTreeMap<i32, crate::native_move::NativeMoveState>,
    pub scroll_updates: BTreeMap<i32, ScrollLayerFrame>,
    pub shake_updates: BTreeMap<i32, crate::native_effects_shake::ShakeState>,
    pub message_background: Option<Arc<dyn ImageResource>>,
}
pub struct NativeFrameWork {
    print_started:bool,
    did_calcu:bool,
    current_slot: Option<i32>,
    buffers: crate::native_capture_effect::FrameBuffers,
    client: ClientId,
    base_canvas: u32,
    old: Option<Arc<dyn ImageResource>>,
    current: Option<Arc<dyn ImageResource>>,
    effects: VecDeque<NativeFrameEffect>,
    random: PatternRng,
    pattern: Option<Arc<dyn PatternResource>>,
    work: Option<Work>,
    ready: Option<NativeFrameResult>,
    pub api_calls: u64,
    base: Option<Arc<dyn ImageResource>>,
    lamp_updates: BTreeMap<i32, crate::native_old_lamp::OldLampState>,
    move_updates: BTreeMap<i32, crate::native_move::NativeMoveState>,
    scroll_updates: BTreeMap<i32, ScrollLayerFrame>,
    shake_updates: BTreeMap<i32, crate::native_effects_shake::ShakeState>,
    message_background: Option<Arc<dyn ImageResource>>,
}
impl NativeFrameWork {
    pub(crate) fn stage(&self)->&'static str {match &self.work {None=>if self.ready.is_some(){"ready"}else{"none"},Some(w)=>match w {Work::BathUpdates(_)=>"bath",Work::Start=>"start",Work::Staff(_)=>"staff",Work::Animation(_)=>"animation",Work::FullAnime(_)=>"full_anime",Work::LayerSequence(..)=>"layer_sequence",Work::Old(_)=>"old",Work::Base(_)=>"base",Work::Effect(_)=>"effect",Work::Advance(..)=>"advance",Work::Pattern(_)=>"pattern",Work::Sample(_)=>"sample",Work::CaptureInit(..)=>"capture_init",Work::WipeOld(..)=>"wipe_old",Work::WipeNew(..)=>"wipe_new"}}}
    pub fn begin(
        client: ClientId,
        old_canvas: u32,
        base_canvas: u32,
        old: Option<Arc<dyn ImageResource>>,
        mut effects: Vec<NativeFrameEffect>,
        mut random: PatternRng,
        host: &mut dyn HostServices,
    ) -> Result<Self> {
        random.validate()?;
        let mut move_updates = BTreeMap::new();
        let mut scroll_updates = BTreeMap::new();
        let mut shake_updates = BTreeMap::new();
        let lamp_updates = prepare_calcu(
            &mut effects,
            &mut random,
            false,
            &mut move_updates,
            &mut scroll_updates,
            &mut shake_updates,
        )?;
        let work = if old.is_some() {
            Work::Base(host.begin_canvas_image(client, base_canvas)?)
        } else {
            Work::Old(host.begin_canvas_image(client, old_canvas)?)
        };
        Ok(Self {
            print_started:false,
            did_calcu:false,
            current_slot:None,
            buffers: crate::native_capture_effect::FrameBuffers{bath_updates:crate::native_bath_frames::mutations(effects.iter()),speed_line_class:frame_speed(effects.iter()),grass_class:frame_grass(effects.iter()),rain_class:frame_rain(effects.iter()),picture_updates:picture_states(&effects),rainbow_palette:frame_palette(&effects),..Default::default()},
            client,
            base_canvas,
            old,
            current: None,
            effects: effects.into(),
            random,
            pattern: None,
            message_background: None,
            work: Some(work),
            ready: None,
            api_calls: 1,
            base: None,
            lamp_updates,
            move_updates,
            scroll_updates,
            shake_updates,
        })
    }
    pub fn begin_draw(
        client: ClientId,
        base_canvas: u32,
        mut effects: Vec<NativeFrameEffect>,
        mut random: PatternRng,
        calcu: bool,
        host: &mut dyn HostServices,
    ) -> Result<Self> {
        random.validate()?;
        let mut move_updates = BTreeMap::new();
        let mut scroll_updates = BTreeMap::new();
        let mut shake_updates = BTreeMap::new();
        let lamp_updates = prepare_calcu(
            &mut effects,
            &mut random,
            calcu,
            &mut move_updates,
            &mut scroll_updates,
            &mut shake_updates,
        )?;
        Ok(Self {
            print_started:false,
            did_calcu:calcu,
            current_slot:None,
            buffers: crate::native_capture_effect::FrameBuffers{bath_updates:crate::native_bath_frames::mutations(effects.iter()),speed_line_class:frame_speed(effects.iter()),grass_class:frame_grass(effects.iter()),rain_class:frame_rain(effects.iter()),picture_updates:picture_states(&effects),rainbow_palette:frame_palette(&effects),..Default::default()},
            client,
            base_canvas,
            old: None,
            current: None,
            effects: effects.into(),
            random,
            pattern: None,
            message_background: None,
            work: Some(Work::Base(host.begin_canvas_image(client, base_canvas)?)),
            ready: None,
            api_calls: 1,
            base: None,
            lamp_updates,
            move_updates,
            scroll_updates,
            shake_updates,
        })
    }
    pub fn with_base(
        client: ClientId,
        base: Arc<dyn ImageResource>,
        effects: Vec<NativeFrameEffect>,
        random: PatternRng,
        host: &mut dyn HostServices,
    ) -> Result<Self> {
        Self::with_base_policy(client, base, effects, random, true, host)
    }
    pub(crate) fn with_saved_base(
        client: ClientId,
        base: Arc<dyn ImageResource>,
        effects: Vec<NativeFrameEffect>,
        random: PatternRng,
        host: &mut dyn HostServices,
    ) -> Result<Self> {
        Self::with_base_policy(client, base, effects, random, false, host)
    }
    fn with_base_policy(
        client: ClientId,
        base: Arc<dyn ImageResource>,
        effects: Vec<NativeFrameEffect>,
        random: PatternRng,
        calcu: bool,
        host: &mut dyn HostServices,
    ) -> Result<Self> {
        Self::with_context_base(client,base,effects,random,calcu,Default::default(),host)
    }
    pub(crate) fn with_context_base(
        client:ClientId,
        base:Arc<dyn ImageResource>,
        mut effects:Vec<NativeFrameEffect>,
        mut random:PatternRng,
        calcu:bool,
        buffers:crate::native_capture_effect::FrameBuffers,
        _host:&mut dyn HostServices,
    )->Result<Self>{
        ensure!(base.client() == client, "NNN cached base resource scope");
        if let Some(palette)=&buffers.rainbow_palette {
            for effect in &mut effects {
                if let NativeFrameEffect::Picture(frame)=effect {
                    if matches!(frame.state.kind,crate::native_picture_effect::PictureEffectKind::Rainbow{..}) {frame.rainbow_palette=Some(palette.clone());}
                }
            }
        }
        if let Some(class)=buffers.rain_class.as_ref() {
            for effect in &mut effects {if let NativeFrameEffect::Picture(frame)=effect {
                if matches!(frame.state.kind,crate::native_picture_effect::PictureEffectKind::Rain{..})&&(calcu||frame.rain_class.is_none()){frame.rain_class=Some(class.clone());}
            }}
        }
        if let Some(class)=buffers.speed_line_class.as_ref(){
            for effect in &mut effects{if let NativeFrameEffect::Picture(frame)=effect{
                if matches!(frame.state.kind,crate::native_picture_effect::PictureEffectKind::SpeedLine{..})&&(calcu||frame.speed_line_class.is_none()){frame.speed_line_class=Some(class.clone());}
            }}
        }
        if let Some(class)=buffers.grass_class.as_ref(){
            for effect in &mut effects{if let NativeFrameEffect::Picture(frame)=effect{
                if matches!(frame.state.kind,crate::native_picture_effect::PictureEffectKind::GrassPlane{..})&&(calcu||frame.grass_class.is_none()){frame.grass_class=Some(class.clone());}
            }}
        }
        random.validate()?;
        let mut move_updates = BTreeMap::new();
        let mut scroll_updates = BTreeMap::new();
        let mut shake_updates = BTreeMap::new();
        let lamp_updates = prepare_calcu(
            &mut effects,
            &mut random,
            calcu,
            &mut move_updates,
            &mut scroll_updates,
            &mut shake_updates,
        )?;
        let mut buffers=buffers;
        if calcu||buffers.bath_updates.is_empty(){buffers.bath_updates=crate::native_bath_frames::mutations(effects.iter());}
        if let Some(class)=crate::native_bath_frames::class(effects.iter()) {if let Some(classes)=buffers.particle_classes.as_mut(){classes.bath=(*class).clone();}}
        if !buffers.staff_bank.is_empty(){for(slot,state)in &move_updates{buffers.staff_geometry.insert(*slot,state.rectangles.clone());}}
        buffers.picture_updates=picture_states(&effects);
        buffers.rainbow_palette=frame_palette(&effects).or(buffers.rainbow_palette);
        buffers.rain_class=frame_rain(effects.iter()).or(buffers.rain_class);
        buffers.grass_class=frame_grass(effects.iter()).or(buffers.grass_class);
        buffers.speed_line_class=frame_speed(effects.iter()).or(buffers.speed_line_class);
        let mut job = Self {
            print_started:false,
            did_calcu:calcu,
            current_slot:None,
            buffers,
            client,
            base_canvas: 0,
            old: Some(base.clone()),
            current: Some(base.clone()),
            base: Some(base),
            effects: effects.into(),
            random,
            pattern: None,
            message_background: None,
            work: None,
            lamp_updates,
            move_updates,
            scroll_updates,
            shake_updates,
            ready: None,
            api_calls: 0,
        };
        // First pixel/resource admission belongs to poll, after callers have
        // attached retained FrameBuffers. Constructor-time next() otherwise
        // loses class-owned resource contexts before with_buffers can run.
        job.work=Some(Work::Start);
        Ok(job)
    }
    pub(crate) fn with_buffers(mut self,mut buffers:crate::native_capture_effect::FrameBuffers)->Self{
        if buffers.bath_updates.is_empty(){buffers.bath_updates=self.buffers.bath_updates.clone();}
        if let Some(class)=crate::native_bath_frames::class(self.effects.iter()){if let Some(classes)=buffers.particle_classes.as_mut(){classes.bath=(*class).clone();}}
        if !buffers.staff_bank.is_empty(){for(slot,state)in &self.move_updates{buffers.staff_geometry.insert(*slot,state.rectangles.clone());}}
        buffers.picture_updates=self.buffers.picture_updates.clone();
        buffers.rain_class=self.buffers.rain_class.clone().or(buffers.rain_class);
        buffers.grass_class=self.buffers.grass_class.clone().or(buffers.grass_class);
        buffers.speed_line_class=self.buffers.speed_line_class.clone().or(buffers.speed_line_class);
        buffers.taiyo_class=self.buffers.taiyo_class.clone().or(buffers.taiyo_class);
        buffers.rainbow_palette=self.buffers.rainbow_palette.clone().or(buffers.rainbow_palette);
        self.buffers=buffers;
        self
    }
    /// Continue the same native BeginEffect scope after the message/UI pass.
    /// Class-wide once flags must survive the pre/post text split.
    pub(crate) fn continue_print_scope(mut self)->Self {self.print_started=true;self.buffers.bath_updates.clear();self}
    /// Adjacent ordinary blits share one worker admission and one destination
    /// copy. Keep painter order and stop at any effect/capture boundary: those
    /// operations may observe or retain the intermediate screen.
    fn paint_batch(
        &mut self,
        host: &mut dyn HostServices,
        first: crysirror_core::image_processing::ImageCompositeLayer,
    ) -> Result<Work> {
        let flat = matches!(first.input.as_ref(), ImageInput::Resource(_) | ImageInput::Solid { .. });
        let mut layers = vec![first];
        while flat && self.effects.front().is_some_and(|effect| match effect {
            NativeFrameEffect::Underlay { layer, .. } | NativeFrameEffect::Paint(layer) => {
                matches!(layer.input.as_ref(), ImageInput::Resource(_) | ImageInput::Solid { .. })
            }
            NativeFrameEffect::Move(_) => true,
            _ => false,
        }) {
            let mut effect = self.effects.pop_front().unwrap();
            self.refresh_staff_effect(&mut effect)?;
            let layer = match effect {
                NativeFrameEffect::Underlay { slot, layer } => self.staff_underlay(slot, layer)?,
                NativeFrameEffect::Move(frame) => frame.paint()?,
                NativeFrameEffect::Paint(layer) => Some(layer),
                _ => unreachable!(),
            };
            if let Some(layer) = layer {
                layers.push(layer);
            }
            // Bound each admission below the shared graph's node budget.
            // Nested input graphs retain their individual admissions.
            if layers.len() == 64 {
                break;
            }
        }
        Ok(Work::Effect(host.image_metadata(self.client)?.begin_process(
            ImageInput::Composite {
                base: Box::new(ImageInput::Resource(self.current.clone().unwrap())),
                layers,
            },
            b"[]".as_slice().into(),
        )?))
    }
    fn next(&mut self, host: &mut dyn HostServices) -> Result<()> {
        if !self.print_started {
            if let Some(classes)=self.buffers.particle_classes.as_mut(){
                if self.did_calcu {crate::native_natural_frames::calcu_beach(classes,self.effects.iter());}
                classes.begin_print();
            }
            self.print_started=true;
            if self.did_calcu {
                self.buffers.animation_previous_nops.extend(animation_nops(self.effects.iter()));
            }
            if !self.buffers.bath_updates.is_empty() {
                let first=self.buffers.bath_updates.first().unwrap();
                let pictures=first.bank.pictures.iter().map(|(slot,p)|(*slot,p.picture.clone())).collect();
                let bank=crate::native_staff_bank::snapshot(self.client,first.bank_limit,&pictures,&self.buffers.staff_bank)?;
                let atlas=self.buffers.particle_classes.as_ref().context("NNN bath constructor brush owner absent")?.brush_atlas.clone();
                let updates=std::mem::take(&mut self.buffers.bath_updates);
                self.work=Some(Work::BathUpdates(crate::native_bath_updates::Preparation::begin(self.client,atlas,bank,updates)?));
                return Ok(());
            }
        }
        let Some(mut effect) = self.effects.pop_front() else {
            // CAllEffect::EndEffect runs after every Print, including invisible
            // layers. Keep mutations and RNG in the same candidate receipt;
            // no completion pump can replay CountIsMax or consume extra draws.
            for (slot,state) in self.buffers.picture_updates.iter_mut() {
                if let crate::native_picture_effect::PictureEffectKind::Circle{parameters,command:28,..}=state.kind {
                    if state.count>=state.frames {
                        let (canvas,assigned)=self.buffers.circle_ends.get(slot).copied().context("NNN Circle EndEffect frame context absent")?;
                        if let Some(terminal)=crate::native_picture_circle::terminal(&parameters,canvas,assigned) {
                            state.kind=match terminal {
                                crate::native_picture_circle::Terminal::Fill(parameters)=>{state.parameter_frames=parameters[0];crate::native_picture_effect::PictureEffectKind::Circle{parameters,command:54,active:true}},
                                crate::native_picture_circle::Terminal::Nop{active}=>crate::native_picture_effect::PictureEffectKind::Circle{parameters,command:0,active},
                            };
                        }
                    }
                }
                if let crate::native_picture_effect::PictureEffectKind::BossDeath{parameters}=state.kind {
                    if state.count>=state.frames {state.kind=crate::native_picture_effect::PictureEffectKind::StoppedBoss{parameters};}
                }
                if let crate::native_picture_effect::PictureEffectKind::Change{parameters}=&mut state.kind {
                    if state.count>=state.frames {
                        crate::native_picture_change::count_is_max(parameters,&mut state.count,state.frames,&mut self.random)?;
                    }
                }
            }
            self.ready = Some(NativeFrameResult {
                buffers:self.buffers.clone(),
                old: self.old.clone().unwrap(),
                target: self.current.clone().unwrap(),
                random: self.random.clone(),
                shared_pattern: self.pattern.clone(),
                base: self.base.clone().unwrap(),
                lamp_updates: self.lamp_updates.clone(),
                move_updates: self.move_updates.clone(),
                scroll_updates: self.scroll_updates.clone(),
                shake_updates: self.shake_updates.clone(),
                message_background: self.message_background.clone(),
            });
            return Ok(());
        };
        self.refresh_staff_effect(&mut effect)?;
        self.work = Some(match effect {
            NativeFrameEffect::Slot(slot)=>{self.current_slot=Some(slot);return self.next(host);}
            NativeFrameEffect::Underlay{slot,layer}=>{
                let Some(layer)=self.staff_underlay(slot,layer)? else{return self.next(host);};
                self.paint_batch(host, layer)?
            }
            NativeFrameEffect::MessageBackground => {
                self.message_background = self.current.clone();
                return self.next(host);
            }
            NativeFrameEffect::Move(frame) => {
                let Some(layer) = frame.paint()? else {
                    return self.next(host);
                };
                self.paint_batch(host, layer)?
            }
            NativeFrameEffect::Flash(frame) => {
                let input = frame.state.image_input(
                    ImageInput::Resource(self.current.clone().unwrap()),
                    frame.canvas,
                );
                Work::Effect(
                    host.image_metadata(self.client)?
                        .begin_process(input, b"[]".as_slice().into())?,
                )
            }
            NativeFrameEffect::Shake(frame) => {
                if !frame.visible {
                    return self.next(host);
                }
                let input = frame.image_input(
                    ImageInput::Resource(self.current.clone().unwrap()), &mut self.random,
                )?;
                Work::Effect(
                    host.image_metadata(self.client)?
                        .begin_process(input, b"[]".as_slice().into())?,
                )
            }
            NativeFrameEffect::Fusuma(frame) => {
                let input = frame.image_input_with_random(
                    ImageInput::Resource(self.current.clone().unwrap()),
                    &mut self.random,
                )?;
                Work::Effect(
                    host.image_metadata(self.client)?
                        .begin_process(input, b"[]".as_slice().into())?,
                )
            }
            NativeFrameEffect::Scroll(frame) => {
                if !frame.normal.visible {
                    return self.next(host);
                }
                let base = ImageInput::Resource(self.current.clone().unwrap());
                let input = if frame.ended {
                    let Some(layer) = frame.normal.paint()? else {
                        return self.next(host);
                    };
                    ImageInput::Composite {
                        base: Box::new(base),
                        layers: vec![layer],
                    }
                } else {
                    frame.frame.image_input(base)?
                };
                Work::Effect(
                    host.image_metadata(self.client)?
                        .begin_process(input, b"[]".as_slice().into())?,
                )
            }
            NativeFrameEffect::Paint(layer) => {
                self.paint_batch(host, layer)?
            }
            NativeFrameEffect::Suna(mut state) => {
                if let Some(shared) = &self.pattern {
                    state.resource = shared.clone();
                }
                let job = host.begin_pattern(
                    self.client,
                    PatternRequest::AdvancePrefix {
                        source: state.resource.clone(),
                        rng: serde_json::to_vec(&self.random)?.into(),
                        words: ((state.parameters.rect[2] / 2) * (state.parameters.rect[3] / 2) / 4)
                            as usize,
                    },
                )?;
                Work::Advance(job, state)
            }
            NativeFrameEffect::Fade(state) => {
                let effects =
                    serde_json::to_vec(&[state.image_effect_with_random(&mut self.random)?])?;
                Work::Effect(host.image_metadata(self.client)?.begin_process(
                    ImageInput::Resource(self.current.clone().unwrap()),
                    effects.into(),
                )?)
            }
            NativeFrameEffect::Bura(state) => {
                let effect = state.image_effect_with_random(
                    self.current.as_ref().unwrap().size(),
                    &mut self.random,
                )?;
                Work::Effect(host.image_metadata(self.client)?.begin_process(
                    ImageInput::Resource(self.current.clone().unwrap()),
                    serde_json::to_vec(&[effect])?.into(),
                )?)
            }
            NativeFrameEffect::Nega(state) => {
                let effects =
                    serde_json::to_vec(&[state.nega_image_effect_with_random(&mut self.random)?])?;
                Work::Effect(host.image_metadata(self.client)?.begin_process(
                    ImageInput::Resource(self.current.clone().unwrap()),
                    effects.into(),
                )?)
            }
            NativeFrameEffect::Grey(state) => {
                let effects =
                    serde_json::to_vec(&[state.image_effect_with_random(&mut self.random)?])?;
                Work::Effect(host.image_metadata(self.client)?.begin_process(
                    ImageInput::Resource(self.current.clone().unwrap()),
                    effects.into(),
                )?)
            }
            NativeFrameEffect::Lamp(frame) => {
                if !frame
                    .state
                    .visible(frame.source.is_some(), &mut self.random)?
                {
                    return self.next(host);
                }
                let source = frame.source.unwrap();
                let size = source.size();
                let input = ImageInput::Composite {
                    base: Box::new(ImageInput::Resource(self.current.clone().unwrap())),
                    layers: vec![crysirror_core::image_processing::ImageCompositeLayer {
                        input: Box::new(ImageInput::Resource(source)),
                        source: [0, 0, size[0] as i32, size[1] as i32],
                        destination: frame.destination,
                        clip: None,
                        opacity: 255,
                        math:
                            crysirror_core::image_processing::ImageCompositeMath::MaskPiecewise256 {
                                weight: 256,
                            },
                    }],
                };
                Work::Effect(
                    host.image_metadata(self.client)?
                        .begin_process(input, b"[]".as_slice().into())?,
                )
            }
            NativeFrameEffect::Mosaic(state) => {
                let current = self.current.clone().unwrap();
                let Some(effect) = state.effect(current.size(), &mut self.random)? else { return self.next(host); };
                Work::Effect(host.image_metadata(self.client)?.begin_process(ImageInput::Resource(current), serde_json::to_vec(&[effect])?.into())?)
            }
            NativeFrameEffect::Picture(mut frame) => {
                if matches!(frame.state.kind,crate::native_picture_effect::PictureEffectKind::Circle{command:28,..}) {self.buffers.circle_ends.insert(frame.slot,(frame.canvas,frame.geometry_assigned));}
                if !frame.visible{return self.next(host);}
                if let crate::native_picture_effect::PictureEffectKind::StaffRoll{parameters}=frame.state.kind {
                    self.work=Some(Work::Staff(self.begin_staff_pages(&frame,parameters)?));return Ok(());
                }
                if let crate::native_picture_effect::PictureEffectKind::Kira{parameters}=frame.state.kind {
                    let input=self.buffers.kira_class.image_input(&parameters,frame.canvas,ImageInput::Resource(self.current.clone().unwrap()),&mut self.random)?;
                    let Some(input)=input else{return self.next(host);};
                    self.work=Some(Work::Effect(host.image_metadata(self.client)?.begin_process(input,b"[]".as_slice().into())?));
                    self.api_calls+=1;return Ok(());
                }
                if matches!(frame.state.kind,crate::native_picture_effect::PictureEffectKind::Rain{..}) {
                    let class=frame.rain_class.as_ref().or(self.buffers.rain_class.as_ref()).context("NNN RAIN painter class absent")?;
                    let lines=class.lines(frame.canvas)?;
                    self.work=Some(Work::Effect(host.image_metadata(self.client)?.begin_process(ImageInput::Resource(self.current.clone().unwrap()),serde_json::to_vec(&[crysirror_image::effects::ImageEffect::IntegerLines{lines}])?.into())?));
                    self.api_calls+=1;return Ok(());
                }
                if let crate::native_picture_effect::PictureEffectKind::Rain2{parameters,work}=frame.state.kind {
                    let Some(input)=crate::native_rain2::image_input(&frame,&parameters,&work,ImageInput::Resource(self.current.clone().unwrap()))? else{return self.next(host);};
                    self.work=Some(Work::Effect(host.image_metadata(self.client)?.begin_process(input,b"[]".as_slice().into())?));self.api_calls+=1;return Ok(());
                }
                if let crate::native_picture_effect::PictureEffectKind::Taiyo{parameters}=frame.state.kind {
                    // A constructor can admit its first Print before external
                    // FrameBuffers attach. Seed from the retained class only
                    // once, then preserve earlier-slot Print mutations.
                    if self.buffers.taiyo_class.is_none(){self.buffers.taiyo_class=Some((**frame.taiyo_class.as_ref().context("NNN TAIYO painter class absent")?).clone());}
                    let input=self.buffers.taiyo_class.as_mut().unwrap().image_input(&parameters,frame.state.count,frame.state.frames,ImageInput::Resource(self.current.clone().unwrap()),&mut self.random)?;
                    self.work=Some(Work::Effect(host.image_metadata(self.client)?.begin_process(input,b"[]".as_slice().into())?));self.api_calls+=1;return Ok(());
                }
                if let crate::native_picture_effect::PictureEffectKind::GrassPlane{parameters}=frame.state.kind {
                    let class=frame.grass_class.as_ref().or(self.buffers.grass_class.as_ref()).context("NNN GRASSPLANE painter class absent")?;
                    let Some(input)=class.image_input(&frame,&parameters,ImageInput::Resource(self.current.clone().unwrap()))?else{return self.next(host);};
                    self.work=Some(Work::Effect(host.image_metadata(self.client)?.begin_process(input,b"[]".as_slice().into())?));self.api_calls+=1;return Ok(());
                }
                if matches!(frame.state.kind,crate::native_picture_effect::PictureEffectKind::Remember{..}) {
                    let input=self.buffers.remember_class.image_input(frame.canvas,ImageInput::Resource(self.current.clone().unwrap()))?;
                    self.work=Some(Work::Effect(host.image_metadata(self.client)?.begin_process(input,b"[]".as_slice().into())?));self.api_calls+=1;return Ok(());
                }
                if let crate::native_picture_effect::PictureEffectKind::Water{parameters}=frame.state.kind {
                    let input=self.buffers.water_class.image_input(parameters,frame.canvas,ImageInput::Resource(self.current.clone().unwrap()))?;
                    self.work=Some(Work::Effect(host.image_metadata(self.client)?.begin_process(input,b"[]".as_slice().into())?));
                    self.api_calls+=1;return Ok(());
                }
                if let crate::native_picture_effect::PictureEffectKind::Konoha{parameters}=frame.state.kind {
                    let layers=crate::native_picture_konoha::layers(&frame,&parameters)?;
                    if layers.is_empty(){return self.next(host);}
                    let mut batches:VecDeque<_>=layers.chunks(64).map(|chunk|chunk.to_vec()).collect();
                    let first=batches.pop_front().unwrap();
                    let job=host.image_metadata(self.client)?.begin_process(ImageInput::Composite{base:Box::new(ImageInput::Resource(self.current.clone().unwrap())),layers:first},b"[]".as_slice().into())?;
                    self.work=Some(Work::LayerSequence(job,batches));self.api_calls+=1;return Ok(());
                }
                if let crate::native_picture_effect::PictureEffectKind::FullAnime{parameters}=frame.state.kind {
                    let Some(request)=self.buffers.full_anime.request(&parameters,frame.state.count,frame.state.frames)? else{return self.next(host);};
                    let context=self.buffers.full_anime_context.as_ref().ok_or_else(||anyhow::anyhow!("NNN FULLANIME requires the session file-view worker"))?;
                    self.work=Some(Work::FullAnime(crate::native_full_anime::FullAnimePreparation::from_context(self.client,context,request.filename,frame.canvas)?));
                    return Ok(());
                }
                if let crate::native_picture_effect::PictureEffectKind::CharaMosaic{parameters}=frame.state.kind {
                    let Some(plan)=crate::native_picture_chara_mosaic::image_plan(&frame,parameters,ImageInput::Resource(self.current.clone().unwrap()),&self.random)? else{return self.next(host);};
                    self.work=Some(match plan {
                        crate::native_picture_chara_mosaic::PaintPlan::Image(input)=>Work::Effect(host.image_metadata(self.client)?.begin_process(input,b"[]".as_slice().into())?),
                        crate::native_picture_chara_mosaic::PaintPlan::Pattern(request)=>Work::Sample(host.begin_pattern(self.client,request)?),
                    });
                    self.api_calls+=1;return Ok(());
                }
                if matches!(frame.state.kind,crate::native_picture_effect::PictureEffectKind::SpecialMove{..}) {
                    let input=crate::native_picture_special_move::image_input(&mut frame,ImageInput::Resource(self.current.clone().unwrap()),&mut self.random)?;
                    self.buffers.picture_updates.insert(frame.slot,frame.state.clone());
                    let Some(input)=input else{return self.next(host);};
                    self.work=Some(Work::Effect(host.image_metadata(self.client)?.begin_process(input,b"[]".as_slice().into())?));
                    self.api_calls+=1;return Ok(());
                }
                if let crate::native_picture_effect::PictureEffectKind::Animation{parameters}=frame.state.kind {
                    let mut bank=self.buffers.animation.clone().context("NNN ANIMATION class absent")?;
                    bank.set_palette(self.buffers.png_palette.clone());
                    let context=self.buffers.staff_context.clone().context("NNN ANIMATION resource worker absent")?;
                    let Some(work)=crate::native_animation_session::FrameWork::begin(&bank,context,parameters,
                        frame.slot as u64,frame.state.count,frame.state.frames,frame.rectangles.clone(),frame.canvas,
                        frame.motion_background,self.buffers.animation_off,ImageInput::Resource(self.current.as_ref().unwrap().clone()))? else{return self.next(host);};
                    if let Some(state)=self.buffers.picture_updates.get_mut(&frame.slot){state.kind=crate::native_picture_effect::PictureEffectKind::Animation{parameters:work.parameters};}
                    self.work=Some(Work::Animation(work));return Ok(());
                }
                if let crate::native_picture_effect::PictureEffectKind::Hahen{parameters}=frame.state.kind {
                    let classes=self.buffers.particle_classes.as_ref().context("NNN HAHEN constructor state absent")?;
                    let picture=frame.bank.picture(frame.slot);
                    let Some(paint)=classes.hahen.paint(&parameters,frame.state.count,frame.state.frames,picture.is_some(),&mut self.random)? else {return self.next(host);};
                    let Some(picture)=picture else{return self.next(host);};
                    let spec=crate::native_hahen_draw::spec(paint)?;
                    let input=ImageInput::Process {input:Box::new(ImageInput::Raster {
                        destination:Box::new(ImageInput::Resource(self.current.clone().unwrap())),source:Box::new(ImageInput::Resource(picture.picture.opaque.clone()))}),
                        effects:serde_json::to_vec(&[crysirror_image::effects::ImageEffect::ByteRaster {spec:crysirror_image::byte_raster::ByteRaster::FloatingPolygonFragments{spec}}])?.into()};
                    self.work=Some(Work::Effect(host.image_metadata(self.client)?.begin_process(input,b"[]".as_slice().into())?));self.api_calls+=1;return Ok(());
                }
                if let crate::native_picture_effect::PictureEffectKind::BathWindow{parameters}=frame.state.kind {
                    let input=crate::native_bath_frames::image_input(&frame,&parameters,ImageInput::Resource(self.current.clone().unwrap()))?;
                    let Some(input)=input else{return self.next(host);};
                    self.work=Some(Work::Effect(host.image_metadata(self.client)?.begin_process(input,b"[]".as_slice().into())?));
                    self.api_calls+=1;return Ok(());
                }
                if matches!(frame.state.kind,crate::native_picture_effect::PictureEffectKind::Beach{..}|crate::native_picture_effect::PictureEffectKind::Hanabira{..}|crate::native_picture_effect::PictureEffectKind::Hotaru{..}|crate::native_picture_effect::PictureEffectKind::HotaruNew{..}) {
                    let classes=self.buffers.particle_classes.as_mut().context("NNN natural constructor class absent")?;
                    let input=crate::native_natural_frames::image_input(&frame,classes,ImageInput::Resource(self.current.clone().unwrap()),&mut self.random)?;
                    let Some(input)=input else{return self.next(host);};
                    self.work=Some(Work::Effect(host.image_metadata(self.client)?.begin_process(input,b"[]".as_slice().into())?));
                    self.api_calls+=1;return Ok(());
                }
                if matches!(frame.state.kind,crate::native_picture_effect::PictureEffectKind::Flower{..}|crate::native_picture_effect::PictureEffectKind::Snow{..}) {
                    let classes=self.buffers.particle_classes.as_mut().context("NNN natural effect constructor state absent")?;
                    let base=ImageInput::Resource(self.current.clone().unwrap());
                    let input=match frame.state.kind {
                        crate::native_picture_effect::PictureEffectKind::Flower{parameters}=>classes.print_flower(&parameters,frame.canvas,base,&mut self.random)?,
                        crate::native_picture_effect::PictureEffectKind::Snow{parameters,second}=>classes.print_snow(second,&parameters,frame.canvas,base,&mut self.random)?,
                        _=>unreachable!(),
                    };
                    let Some(input)=input else{return self.next(host);};
                    self.work=Some(Work::Effect(host.image_metadata(self.client)?.begin_process(input,b"[]".as_slice().into())?));
                    self.api_calls+=1;return Ok(());
                }
                if let crate::native_picture_effect::PictureEffectKind::Cloud{parameters}=frame.state.kind {
                    let picture=frame.bank.picture(frame.slot);
                    self.buffers.cloud_class.print_clock(&parameters,&mut frame.state.count,picture.map(|p|p.picture.raw.size()));
                    self.buffers.picture_updates.insert(frame.slot,frame.state.clone());
                    let Some(picture)=picture else{return self.next(host);};
                    let spec=self.buffers.cloud_class.raster_spec(&parameters,picture.picture.raw.size(),frame.canvas,picture.picture.has_mask)?;
                    let source=if spec.masked {picture.picture.mask_storage.as_ref().map(|m|m.view.clone()).unwrap_or_else(||picture.picture.raw.clone())}else{picture.picture.opaque.clone()};
                    let input=ImageInput::Process {input:Box::new(ImageInput::Raster {destination:Box::new(ImageInput::Resource(self.current.clone().unwrap())),source:Box::new(ImageInput::Resource(source))}),effects:serde_json::to_vec(&[crysirror_image::effects::ImageEffect::ByteRaster{spec:crysirror_image::byte_raster::ByteRaster::PackedRepeatRows{spec}}])?.into()};
                    self.work=Some(Work::Effect(host.image_metadata(self.client)?.begin_process(input,b"[]".as_slice().into())?));
                    self.api_calls+=1;return Ok(());
                }
                if let crate::native_picture_effect::PictureEffectKind::Hamon{parameters}=frame.state.kind {
                    let select=crate::native_hamon::sources(&parameters,frame.bank_limit)?;
                    if (select.first_capture||select.second_capture)&&self.buffers.capture.is_none(){
                        self.work=Some(Work::CaptureInit(host.image_metadata(self.client)?.begin_process(ImageInput::Solid{size:frame.canvas,color:[0,0,0,255]},b"[]".as_slice().into())?,frame));
                        self.api_calls+=1;return Ok(());
                    }
                    let field=self.buffers.hamon_field.as_ref().context("NNN HAMON retained field absent")?;
                    let input=crate::native_hamon::image_input(&frame,&parameters,field,self.buffers.capture.as_ref(),&mut self.random)?;
                    let Some(input)=input else{return self.next(host);};
                    self.work=Some(Work::Effect(host.image_metadata(self.client)?.begin_process(input,b"[]".as_slice().into())?));
                    self.api_calls+=1;return Ok(());
                }
                if let crate::native_picture_effect::PictureEffectKind::SunRays{..}=frame.state.kind {
                    let input=self.buffers.sun_rays_class.image_input(frame.state.count,frame.state.frames,frame.canvas,ImageInput::Resource(self.current.clone().unwrap()),&mut self.random)?;
                    let Some(input)=input else{return self.next(host);};
                    self.work=Some(Work::Effect(host.image_metadata(self.client)?.begin_process(input,b"[]".as_slice().into())?));
                    self.api_calls+=1;return Ok(());
                }
                if let crate::native_picture_effect::PictureEffectKind::OldFilm{parameters}= &mut frame.state.kind {
                    // Native OLDFILM mutates its line state in Print, not Calcu.
                    // Keep the candidate with the immutable render receipt, so
                    // retries never repeat RNG draws or acknowledge early.
                    let lines=crate::native_picture_old_film::print_plan(parameters,&mut self.random)?;
                    let input=crate::native_picture_old_film::image_input(ImageInput::Resource(self.current.clone().unwrap()),frame.canvas,&lines)?;
                    self.buffers.picture_updates.insert(frame.slot,frame.state.clone());
                    self.work=Some(Work::Effect(host.image_metadata(self.client)?.begin_process(input,b"[]".as_slice().into())?));
                    self.api_calls+=1;return Ok(());
                }
                if let crate::native_picture_effect::PictureEffectKind::MaskWipe{command,parameters}=frame.state.kind {
                    if crate::native_picture_mask_wipe::needs_capture(command,&parameters)&&self.buffers.capture.is_none(){
                        // CEffectGet allocates uninitialized screen-sized RAM in
                        // its constructor. Define the pre-GET buffer as black.
                        self.work=Some(Work::CaptureInit(host.image_metadata(self.client)?.begin_process(ImageInput::Solid{size:frame.canvas,color:[0,0,0,255]},b"[]".as_slice().into())?,frame));
                        self.api_calls+=1;return Ok(());
                    }
                    let input=crate::native_picture_mask_wipe::image_input(&frame,command,&parameters,&self.buffers.capture,&mut self.random)?;
                    let Some(input)=input else{return self.next(host);};
                    self.work=Some(Work::Effect(host.image_metadata(self.client)?.begin_process(input,b"[]".as_slice().into())?));
                    self.api_calls+=1;return Ok(());
                }
                if let Some((command,p))=frame.buffered_parameters(){
                    if command==75{
                        let paint=!self.buffers.once_printed.contains(&frame.slot)||(frame.state.count>=p[1]&&frame.state.count<p[1].wrapping_add(p[2]));
                        self.buffers.once_printed.insert(frame.slot);
                        let Some(layer)=frame.normal.as_ref().filter(|_|paint).map(|n|n.paint()).transpose()?.flatten() else{return self.next(host);};
                        self.work=Some(Work::Effect(host.image_metadata(self.client)?.begin_process(ImageInput::Composite{base:Box::new(ImageInput::Resource(self.current.clone().unwrap())),layers:vec![layer]},b"[]".as_slice().into())?));
                        self.api_calls+=1;return Ok(());
                    }
                    if command==66 {
                        // CPicture::Shrink only reallocates tightly packed storage;
                        // retaining the immutable engine resource preserves its pixels.
                        return self.next(host);
                    }
                    if command==73 {
                        self.buffers.capture=self.current.clone();
                        return self.next(host);
                    }
                    if command==59&&!frame.wipe_can_print(){return self.next(host);}
                    if frame.needs_capture()&&self.buffers.capture.is_none(){
                        // Original GET allocates uninitialised screen RAM. Define
                        // PUT/wipe-before-GET as black rather than exposing heap data.
                        Work::CaptureInit(host.image_metadata(self.client)?.begin_process(ImageInput::Solid{size:frame.canvas,color:[0,0,0,255]},b"[]".as_slice().into())?,frame)
                    }else if command==2{
                        let input=if frame.state.count==0&&p[4]==0{
                            frame.first_put_input(self.current.clone().unwrap())
                        }else{ImageInput::Resource(self.buffers.capture.clone().unwrap())};
                        Work::Effect(host.image_metadata(self.client)?.begin_process(input,b"[]".as_slice().into())?)
                    }else{
                        let (old,new)=frame.wipe_inputs(&self.buffers.capture)?;
                        Work::WipeOld(host.image_metadata(self.client)?.begin_process(old,b"[]".as_slice().into())?,frame,new)
                    }
                }else{
                let Some(input) = frame.image_input(ImageInput::Resource(self.current.clone().unwrap()), &mut self.random)? else {
                    return self.next(host);
                };
                Work::Effect(host.image_metadata(self.client)?.begin_process(input, b"[]".as_slice().into())?)
                }
            }
            NativeFrameEffect::Chara(frame) => {
                let Some(source) = frame.source else {
                    return self.next(host);
                };
                let [x, y, w, h] = frame.rectangles.destination;
                let [sx, sy, _, _] = frame.rectangles.source;
                let weight = frame.state.weight256_with_random(&mut self.random)?;
                let clip = frame.state.clip_rect();
                // Original Print returns without drawing for empty clipped
                // regions. The effect/count remains active at its endpoint.
                if w <= 0 || h <= 0 || clip.is_some_and(|r| r[2] <= 0 || r[3] <= 0) {
                    return self.next(host);
                }
                let input = ImageInput::Composite {
                    base: Box::new(ImageInput::Resource(self.current.clone().unwrap())),
                    layers: vec![crysirror_core::image_processing::ImageCompositeLayer {
                        input: Box::new(ImageInput::Resource(source)),
                        source: [sx, sy, w, h],
                        destination: [x, y],
                        clip,
                        opacity: 255,
                        math:
                            crysirror_core::image_processing::ImageCompositeMath::MaskPiecewise256 {
                                weight,
                            },
                    }],
                };
                Work::Effect(
                    host.image_metadata(self.client)?
                        .begin_process(input, b"[]".as_slice().into())?,
                )
            }
        });
        self.api_calls += 1;
        Ok(())
    }
    /// Bounded nonblocking dependency pump. Success retains immutable results;
    /// native random/effect state is still only a candidate until owner commit.
    pub fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<NativeFrameResult>> {
        for _ in 0..4 {
            if let Some(result) = &self.ready {
                return Ok(Some(result.clone()));
            }
            let work = self
                .work
                .take()
                .ok_or_else(|| anyhow::anyhow!("NNN frame dependency absent"))?;
            match work {
                Work::Start=>{self.next(host)?;}
                Work::BathUpdates(mut preparation)=>{
                    let before=preparation.api_calls;let result=preparation.poll(host);self.api_calls+=preparation.api_calls-before;
                    if let Some(ready)=result? {
                        self.buffers.staff_bank=ready.bank;self.buffers.staff_touched.extend(ready.touched);
                        self.next(host)?;
                    } else {self.work=Some(Work::BathUpdates(preparation));return Ok(None);}
                }
                Work::Staff(mut pages)=>{
                    let before=pages.api_calls;let result=pages.poll(host);self.api_calls+=pages.api_calls-before;
                    if let Some(result)=result?{for header in &result.png_headers{self.buffers.png_palette.observe_header(header)?;}self.current=Some(result.image);self.buffers.staff_bank=result.bank;self.buffers.staff_touched.extend(result.touched);self.next(host)?;}
                    else{self.work=Some(Work::Staff(pages));return Ok(None);}
                }
                Work::LayerSequence(mut job,mut batches)=>match job.poll()? {
                    ImageProcessState::Pending=>{self.work=Some(Work::LayerSequence(job,batches));return Ok(None);}
                    ImageProcessState::Failed(error)=>bail!("NNN sprite batch: {error}"),
                    ImageProcessState::Ready(image)=>{
                        ensure!(image.client()==self.client&&image.size()==self.current.as_ref().unwrap().size(),"NNN sprite batch scope/extent");
                        self.current=Some(image);
                        if let Some(layers)=batches.pop_front(){
                            let job=host.image_metadata(self.client)?.begin_process(ImageInput::Composite{base:Box::new(ImageInput::Resource(self.current.clone().unwrap())),layers},b"[]".as_slice().into())?;
                            self.work=Some(Work::LayerSequence(job,batches));self.api_calls+=1;
                        }else{self.next(host)?;}
                    }
                },
                Work::Animation(mut work)=>{
                    let Some(ready)=work.poll(host)? else{self.work=Some(Work::Animation(work));return Ok(None);};
                    if let Some(palette)=ready.bank.palette(){self.buffers.png_palette=palette.clone();}
                    self.buffers.animation=Some(ready.bank);
                    if let Some(image)=ready.image{self.current=Some(image);}
                    self.next(host)?;
                }
                Work::FullAnime(mut preparation) => {
                    let before=preparation.api_calls;
                    let ready=preparation.poll(host)?;
                    self.api_calls+=preparation.api_calls-before;
                    if let Some(crate::native_full_anime::Prepared(image))=ready {
                        // Native Print replaces the whole screen, ignores DWQ
                        // orientation/stride/mask and keeps the old frame when
                        // the optional numbered file is absent. The lastPrint
                        // write still commits for that missing-file case.
                        if let Some(image)=image {self.current=Some(image);}
                        self.next(host)?;
                    }else{
                        self.work=Some(Work::FullAnime(preparation));return Ok(None);
                    }
                }
                Work::Old(mut job) => match job.poll()? {
                    ImageProcessState::Pending => {
                        self.work = Some(Work::Old(job));
                        return Ok(None);
                    }
                    ImageProcessState::Failed(error) => {
                        bail!("NNN previous-frame engine capture: {error}")
                    }
                    ImageProcessState::Ready(image) => {
                        ensure!(
                            image.client() == self.client,
                            "NNN foreign previous-frame resource"
                        );
                        self.old = Some(image);
                        self.work = Some(Work::Base(
                            host.begin_canvas_image(self.client, self.base_canvas)?,
                        ));
                        self.api_calls += 1;
                    }
                },
                Work::Base(mut job) => match job.poll()? {
                    ImageProcessState::Pending => {
                        self.work = Some(Work::Base(job));
                        return Ok(None);
                    }
                    ImageProcessState::Failed(error) => {
                        bail!("NNN base-frame engine capture: {error}")
                    }
                    ImageProcessState::Ready(image) => {
                        ensure!(
                            image.client() == self.client,
                            "NNN foreign base-frame resource"
                        );
                        if self.old.is_none() {
                            self.old = Some(image.clone());
                        }
                        self.base = Some(image.clone());
                        self.current = Some(image);
                        self.next(host)?;
                    }
                },
                Work::Effect(mut job) => match job.poll()? {
                    ImageProcessState::Pending => {
                        self.work = Some(Work::Effect(job));
                        return Ok(None);
                    }
                    ImageProcessState::Failed(error) => {
                        bail!("NNN native effect engine operation: {error}")
                    }
                    ImageProcessState::Ready(image) => {
                        self.current = Some(image);
                        self.next(host)?;
                    }
                },
                Work::Sample(mut job)=>match job.poll()? {
                    PatternProcessState::Pending=>{self.work=Some(Work::Sample(job));return Ok(None);}
                    PatternProcessState::Failed(error)=>bail!("NNN masked block sampling: {error}"),
                    PatternProcessState::Ready(PatternOutput::ImageWithRng{image,rng_after})=>{
                        ensure!(image.client()==self.client && image.size()==self.current.as_ref().unwrap().size(),"NNN masked sample image scope/extent");
                        let random:PatternRng=serde_json::from_slice(&rng_after)?;
                        random.validate()?;
                        self.current=Some(image);self.random=random;
                        self.next(host)?;
                    }
                    PatternProcessState::Ready(_)=>bail!("NNN masked sample returned another pattern receipt"),
                },
                Work::CaptureInit(mut job,frame)=>match job.poll()?{
                    ImageProcessState::Pending=>{self.work=Some(Work::CaptureInit(job,frame));return Ok(None);}
                    ImageProcessState::Failed(error)=>bail!("NNN initial GET buffer: {error}"),
                    ImageProcessState::Ready(image)=>{
                        ensure!(image.client()==self.client,"NNN foreign GET buffer");
                        self.buffers.capture=Some(image);
                        self.effects.push_front(NativeFrameEffect::Picture(frame));
                        self.next(host)?;
                    }
                },
                Work::WipeOld(mut job,frame,new)=>match job.poll()?{
                    ImageProcessState::Pending=>{self.work=Some(Work::WipeOld(job,frame,new));return Ok(None);}
                    ImageProcessState::Failed(error)=>bail!("NNN wipe start crop: {error}"),
                    ImageProcessState::Ready(image)=>{
                        ensure!(image.client()==self.client,"NNN foreign wipe source");
                        self.work=Some(Work::WipeNew(host.image_metadata(self.client)?.begin_process(new,b"[]".as_slice().into())?,frame,image));self.api_calls+=1;
                    }
                },
                Work::WipeNew(mut job,frame,old)=>match job.poll()?{
                    ImageProcessState::Pending=>{self.work=Some(Work::WipeNew(job,frame,old));return Ok(None);}
                    ImageProcessState::Failed(error)=>bail!("NNN wipe end crop: {error}"),
                    ImageProcessState::Ready(new)=>{
                        ensure!(new.client()==self.client,"NNN foreign wipe target");
                        let (_,p)=frame.buffered_parameters().unwrap();
                        let progress=crate::native_curve::value_with_random(frame.state.count,frame.state.frames,p[6].wrapping_mul(100),p[5].wrapping_mul(100),p[4],p[3],&mut self.random)?.clamp(0,10000) as u32;
                        let (input,effects)=crate::native_simple_wipe::plan_with_random(p[1],old,new,progress,10000,&mut self.random,&mut self.buffers.wipe_tables)?;
                        self.work=Some(Work::Effect(host.image_metadata(self.client)?.begin_process(input,serde_json::to_vec(&effects)?.into())?));self.api_calls+=1;
                    }
                },
                Work::Advance(mut job, state) => match job.poll()? {
                    PatternProcessState::Pending => {
                        self.work = Some(Work::Advance(job, state));
                        return Ok(None);
                    }
                    PatternProcessState::Failed(error) => {
                        bail!("NNN native pattern advance: {error}")
                    }
                    PatternProcessState::Ready(PatternOutput::Resource {
                        resource,
                        rng_after,
                    }) => {
                        ensure!(
                            resource.client() == self.client,
                            "NNN foreign advanced pattern"
                        );
                        self.random = serde_json::from_slice(&rng_after)?;
                        self.random.validate()?;
                        self.pattern = Some(resource.clone());
                        if state.parameters.rect[2] == 0 || state.parameters.rect[3] == 0 {
                            self.next(host)?;
                            continue;
                        }
                        let opacity = state.opacity256_with_random(&mut self.random)?;
                        self.work = Some(Work::Pattern(host.begin_pattern(
                            self.client,
                            PatternRequest::Render {
                                source: resource,
                                destination: ImageInput::Resource(self.current.clone().unwrap()),
                                rect: state.parameters.rect,
                                opacity256: opacity,
                            },
                        )?));
                        self.api_calls += 1;
                    }
                    _ => bail!("NNN native advance returned image instead of pattern"),
                },
                Work::Pattern(mut job) => match job.poll()? {
                    PatternProcessState::Pending => {
                        self.work = Some(Work::Pattern(job));
                        return Ok(None);
                    }
                    PatternProcessState::Failed(error) => {
                        bail!("NNN native pattern render: {error}")
                    }
                    PatternProcessState::Ready(PatternOutput::Image(image)) => {
                        self.current = Some(image);
                        self.next(host)?;
                    }
                    _ => bail!("NNN native render returned pattern instead of image"),
                },
            }
        }
        Ok(self.ready.clone())
    }
}

pub(crate) fn animation_nops<'a>(effects:impl Iterator<Item=&'a NativeFrameEffect>)->std::collections::BTreeSet<i32>{
    effects.filter_map(|effect|if let NativeFrameEffect::Picture(frame)=effect{
        if let crate::native_picture_effect::PictureEffectKind::Animation{parameters}=frame.state.kind {
            let mut previous=137;crate::native_animation::calcu(&parameters,&mut previous);
            if previous==0{return Some(frame.slot);}
        }None
    }else{None}).collect()
}

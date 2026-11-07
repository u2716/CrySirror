// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native Print/overlap dependency ordering. Rasterisation, dynamic patterns,
//! weighted mixing and publication resources are all engine services.
use crate::native_effects::{FadeInState, LayerGreyState, SunaState};
use anyhow::{Result, bail, ensure};
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
#[derive(Clone)]
pub enum NativeFrameEffect {
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
    fn paint(&self) -> Option<crysirror_core::image_processing::ImageCompositeLayer> {
        if !self.visible {
            return None;
        }
        let image = self.source.as_ref()?;
        let [x, y, w, h] = self.state.rectangles.destination;
        let [sx, sy, _, _] = self.state.rectangles.source;
        if w <= 0 || h <= 0 {
            return None;
        }
        Some(crysirror_core::image_processing::ImageCompositeLayer {
            input: Box::new(ImageInput::Resource(image.clone())),
            source: [sx, sy, w, h],
            destination: [x, y],
            clip: None,
            opacity: 255,
            math: self.math,
        })
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
        self.normal.state.calcu(self.normal.source.is_some());
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
            plan.push(NativeFrameEffect::Paint(layer.clone()));
        }
        if let Some(effect) = effects.get(&slot) {
            if visible.contains(&slot)
                || matches!(
                    effect,
                    NativeFrameEffect::Lamp(_)
                        | NativeFrameEffect::Move(_)
                        | NativeFrameEffect::Scroll(_)
                        | NativeFrameEffect::Shake(_)
                )
            {
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
        NativeFrameEffect::MessageBackground | NativeFrameEffect::Paint(_) => {
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
fn calcu_effect(effect: &mut NativeFrameEffect, random: &mut PatternRng) -> Result<()> {
    match effect {
        NativeFrameEffect::Move(frame) => frame.state.calcu(frame.source.is_some()),
        NativeFrameEffect::Picture(frame) => frame.calcu(),
        NativeFrameEffect::Scroll(frame) => frame.calcu(),
        NativeFrameEffect::Shake(frame) => frame.state.calcu(),
        NativeFrameEffect::Lamp(frame) => frame.state.calcu(random)?,
        _ => {}
    }
    Ok(())
}
pub(crate) fn calcu_effect_states(
    effects: &mut BTreeMap<i32, NativeFrameEffect>,
    random: &mut PatternRng,
) -> Result<()> {
    for effect in effects.values_mut() {
        calcu_effect(effect, random)?;
    }
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
    for effect in effects {
        if calcu {
            calcu_effect(effect, random)?;
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
    Ok(updates)
}
enum Work {
    Old(ImageJob),
    Base(ImageJob),
    Effect(ImageJob),
    Advance(PatternJob, SunaState),
    Pattern(PatternJob),
}
#[derive(Clone)]
pub struct NativeFrameResult {
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
        mut effects: Vec<NativeFrameEffect>,
        mut random: PatternRng,
        calcu: bool,
        host: &mut dyn HostServices,
    ) -> Result<Self> {
        ensure!(base.client() == client, "NNN cached base resource scope");
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
        let mut job = Self {
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
        job.next(host)?;
        Ok(job)
    }
    fn next(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let Some(effect) = self.effects.pop_front() else {
            self.ready = Some(NativeFrameResult {
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
        self.work = Some(match effect {
            NativeFrameEffect::MessageBackground => {
                self.message_background = self.current.clone();
                return self.next(host);
            }
            NativeFrameEffect::Move(frame) => {
                let Some(layer) = frame.paint() else {
                    return self.next(host);
                };
                Work::Effect(host.image_metadata(self.client)?.begin_process(
                    ImageInput::Composite {
                        base: Box::new(ImageInput::Resource(self.current.clone().unwrap())),
                        layers: vec![layer],
                    },
                    b"[]".as_slice().into(),
                )?)
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
                let input = frame.state.image_input(
                    ImageInput::Resource(self.current.clone().unwrap()),
                    frame.canvas,
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
                    let Some(layer) = frame.normal.paint() else {
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
                let input = ImageInput::Composite {
                    base: Box::new(ImageInput::Resource(self.current.clone().unwrap())),
                    layers: vec![layer],
                };
                Work::Effect(
                    host.image_metadata(self.client)?
                        .begin_process(input, b"[]".as_slice().into())?,
                )
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
            NativeFrameEffect::Picture(frame) => {
                let Some(input) = frame.image_input(ImageInput::Resource(self.current.clone().unwrap()), &mut self.random)? else {
                    return self.next(host);
                };
                Work::Effect(host.image_metadata(self.client)?.begin_process(input, b"[]".as_slice().into())?)
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

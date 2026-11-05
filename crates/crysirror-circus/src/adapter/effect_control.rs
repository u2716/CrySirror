//! Live native-control mapping to engine-owned VFX and logical pointer motion.
use super::*;
use crate::machine::effect_ops::EffectRequest;
use crysirror_vfx::falling_atlas::FallingSpawnWindow;
#[path = "effect_framebuffer.rs"]
mod framebuffer;
#[path = "effect_stage.rs"]
mod stage;
#[derive(Default)]
pub(super) struct EffectControl {
    framebuffer: framebuffer::Framebuffer,
    ///40EDB0 initializes wrapper+8 to0;40EEC0/EED0 switch to pool1.
    ///Binding/resetting the inner sprite does not reconstruct this wrapper.
    particle_wrappers:BTreeSet<u16>,
    window: Option<FallingSpawnWindow>,
    aux_capture: bool,
    aux_repeats: u16,
    aux_forever: bool,
    window_pending: bool,
    pointer: Option<crysirror_core::service::SteppedPointerMotion>,
    views: BTreeMap<u16, stage::View>,
    orbit: crysirror_vfx::orbit_atlas::OrbitControl,
    orbit_pending: bool,
    flash: Option<crysirror_core::service::SteppedPulse>,
    frame_fast: bool,
    effects_enabled: bool,
    drift_window: Option<crysirror_vfx::drifting_atlas::DriftSpawnWindow>,
    suppress_fast_color: bool,
    frame_skip_allowed: bool,
}
impl EffectControl {
    pub(in crate::adapter) fn filtered_view_layers(&self,objects:&BTreeSet<u16>)->Vec<u32> {
        self.views.iter().filter(|(object,_)|objects.contains(object)).map(|(_,view)|view.layer).collect()
    }
    pub(super) fn particle_wrapper(&self,object:u16)->bool {self.particle_wrappers.contains(&object)}
    pub(super) fn spawn_window(&self) -> Option<FallingSpawnWindow> {
        self.window.clone()
    }
    pub(super) fn drift_window(&self) -> Option<crysirror_vfx::drifting_atlas::DriftSpawnWindow> {
        self.drift_window.clone()
    }
    pub(super) fn effects_enabled(&self) -> bool {
        self.effects_enabled
    }
    pub(super) fn orbit_control(&self) -> crysirror_vfx::orbit_atlas::OrbitControl {
        self.orbit.clone()
    }
    pub(super) fn clear_pool_controls(&mut self) {
        self.window = None;
        self.window_pending = false;
        self.drift_window = None;
    }
}
impl Bridge<'_> {
    pub(super) fn mark_particle_wrappers(&mut self,objects:&[u16])->Result<()> {
        let mut layers=Vec::new();
        for &object in objects {
            self.services.effect_control.particle_wrappers.insert(object);
            if self.services.visual.roots().any(|value|value==object) {
                layers.push(json!({"op":"set_visible","id":super::visual::object_layer(object),"visible":false}));
            }
        }
        self.submit(layers,vec![])
    }
    pub(in crate::adapter) fn native_effect_fast(&self) -> bool {
        self.services.effect_control.frame_fast
            || self.input.buttons_down & crysirror_core::service::buttons::CTRL != 0
    }
    pub(super) fn native_frame_fast(&self) -> bool {
        self.services.effect_control.frame_fast
            || self.input.buttons_down & crysirror_core::service::buttons::CTRL != 0
    }

    pub(super) fn native_effect_control(&mut self, request: &EffectRequest) -> Result<bool> {
        match request {
            EffectRequest::SpriteMask { seed, mode, ticks } => {
                self.declare_sprite_mask(seed, *mode, *ticks)
            }
            EffectRequest::ResetObjects { objects } => {
                self.reset_stage_objects(objects)?;
                Ok(true)
            }
            EffectRequest::AuxCapture {
                enabled,
                repeats,
                forever,
                replay,
            } => {
                self.services.effect_control.aux_capture = *enabled;
                self.services.effect_control.aux_repeats = *repeats;
                self.services.effect_control.aux_forever = *forever;
                let mut commands = Vec::new();
                for view in self.services.effect_control.views.values() {
                    commands.push(json!({"op":"sequence","id":view.layer,"operation":{"kind":"capture","enabled":enabled,"repeats":repeats,"forever":forever}}));
                    if *replay {
                        commands.push(
                            json!({"op":"sequence","id":view.layer,"operation":{"kind":"replay"}}),
                        );
                    }
                }
                self.submit(commands, vec![])?;
                Ok(true)
            }
            EffectRequest::ClearFrame { protected } => self.clear_source_framebuffer(*protected),
            EffectRequest::ResetColor => {
                self.replace_framebuffer_color()?;
                Ok(true)
            }
            EffectRequest::CancelColor => {
                self.cancel_framebuffer_color()?;
                Ok(true)
            }
            EffectRequest::PartialFallingClear => self.falling_effect(0, 0),
            EffectRequest::CaptureFrame { crop } => self.capture_framebuffer(*crop),
            EffectRequest::FlatFade {
                out,
                ticks,
                color,
                protected,
            } => self.begin_flat_framebuffer(*out, *ticks, *color, *protected),
            EffectRequest::TileMask { pattern, color } => self.begin_tile_mask(*pattern, *color),
            EffectRequest::MaskFade {
                reverse,
                ticks,
                color,
            } => self.begin_mask_framebuffer(*reverse, *ticks, *color),
            EffectRequest::FrameControl {
                fast,
                suppress_fast_color,
                allow_skip,
                effects_enabled,
            } => {
                self.services.effect_control.frame_fast = *fast;
                self.services.effect_control.effects_enabled = *effects_enabled;
                if let Some(effect) = &mut self.services.effects {
                    for emitter in &mut effect.spec.emitters {
                        let bypass = emitter.drifting_atlas.as_ref().is_some_and(|node| {
                            matches!(
                                node.motion,
                                crysirror_vfx::drifting_atlas::DriftMotion::Buoyant
                                    | crysirror_vfx::drifting_atlas::DriftMotion::FallingGlow
                            )
                        });
                        emitter.paused = !*effects_enabled && !bypass;
                    }
                }
                self.services.effect_control.suppress_fast_color = *suppress_fast_color;
                self.services.effect_control.frame_skip_allowed = *allow_skip;
                Ok(true)
            }
            EffectRequest::Flash { repeats } => {
                self.replace_framebuffer_color()?;
                self.root()?;
                self.submit(vec![json!({"op":"put","layer":{"id":0x3b01_0000u32,"rect":[0,0,1280,720],"paint":{"kind":"solid","color":[255,255,255,255]},"opacity":0,"order":[3,0]}})],vec![])?;
                self.services.effect_control.flash = Some(
                    crysirror_core::service::SteppedPulse::new(5, 1 + u32::from(*repeats))?,
                );
                Ok(true)
            }
            EffectRequest::FinishObjects{objects}=>{self.finish_stage_objects(objects)?;Ok(true)},
            EffectRequest::Finish => {
                self.finish_stage_views()?;
                Ok(true)
            }
            EffectRequest::SpawnWindow { bounds, chance } => {
                if !self.effects_ready()? {
                    return Ok(false);
                }
                self.mark_particle_wrappers(&[6,19,29,39,59])?;
                let window = FallingSpawnWindow {
                    bounds: [bounds[0], bounds[2]],
                    chance: *chance,
                    attempts: 100,
                };
                window.validate()?;
                if self.services.effect_control.window_pending {
                    ensure!(
                        self.services.effect_control.window.as_ref() == Some(&window),
                        "Circus falling-control completion ownership"
                    );
                    self.services.effect_control.window_pending = false;
                    return Ok(true);
                }
                if self.services.effect_control.window.as_ref() == Some(&window)
                    && self
                        .services
                        .effect_control
                        .drift_window
                        .as_ref()
                        .is_some_and(|old| old.bounds == *bounds)
                {
                    return Ok(true);
                }
                self.services.effect_control.window = Some(window.clone());
                self.services.effect_control.drift_window =
                    Some(crysirror_vfx::drifting_atlas::DriftSpawnWindow {
                        bounds: *bounds,
                        chance: *chance,
                        attempts: 100,
                    });
                if let Some(effect) = &mut self.services.effects {
                    for emitter in &mut effect.spec.emitters {
                        if let Some(node) = &mut emitter.falling_atlas {
                            node.spawn_window = Some(window.clone());
                        }
                        if let Some(node) = &mut emitter.drifting_atlas {
                            node.spawn_window = self.services.effect_control.drift_window.clone();
                        }
                    }
                    let crysirror_vfx::falling_atlas::SimulationClock::External { tick, .. } =
                        effect.spec.clock
                    else {
                        bail!("Circus falling-control clock")
                    };
                    let epoch =
                        effect
                            .spec
                            .sequence_exchange
                            .as_ref()
                            .map_or(Ok(1), |exchange| {
                                exchange
                                    .epoch
                                    .checked_add(1)
                                    .context("Circus falling-control exchange overflow")
                            })?;
                    effect.spec.sequence_exchange =
                        Some(crysirror_vfx::falling_atlas::SequenceExchange {
                            epoch,
                            base_tick: tick,
                            state: self
                                .services
                                .sequence_rng
                                .clone()
                                .context("Circus falling-control shared random state")?,
                        });
                    let spec = serde_json::to_value(&effect.spec)?;
                    self.host.submit_engine(
                        self.client,
                        1,
                        &serde_json::to_vec(
                            &json!({"world":[{"op":"set_vfx","id":0x3900_0000u32,"vfx":spec}]}),
                        )?,
                    )?;
                    self.services.effect_control.window_pending = true;
                    return Ok(false);
                }
                Ok(true)
            }
            EffectRequest::PointerMove { target, steps } => {
                self.services.effect_control.pointer =
                    Some(crysirror_core::service::SteppedPointerMotion::new(
                        [self.input.pointer_x, self.input.pointer_y],
                        *target,
                        *steps,
                    )?);
                Ok(true)
            }
            EffectRequest::FullImage {
                position,
                percent,
                ticks,
            } => {
                use crate::machine::visual_ops::{VisualProperty, VisualRequest};
                let id = self.visual_root(2)?;
                let layer: Value = serde_json::from_slice(
                    &self
                        .host
                        .query_engine(self.client, EngineQuery::Layer(id))?,
                )?;
                let mut transform = layer["transform"].clone();
                if let Some(logical) = self.services.image_extents.get(&3) {
                    let pivot = self
                        .services
                        .sprite_scales
                        .get(&3)
                        .map_or([0; 2], |(_, pivot)| *pivot);
                    transform["pivot_x"] = json!(if pivot[0] == 0 {
                        logical[0] as f32 / 2.
                    } else {
                        f32::from(pivot[0])
                    });
                    transform["pivot_y"] = json!(if pivot[1] == 0 {
                        logical[1] as f32 / 2.
                    } else {
                        f32::from(pivot[1])
                    });
                    self.submit(
                        vec![json!({"op":"set_transform","id":id,"transform":transform})],
                        vec![],
                    )?;
                }
                let scale = f32::from(*percent) / if *percent >= 1000 { 10000. } else { 100. };
                if let Some(ticks) = ticks {
                    if *ticks == 0 {
                        let equal = layer["rect"][0].as_f64() == Some(f64::from(position[0]))
                            && layer["rect"][1].as_f64() == Some(f64::from(position[1]))
                            && layer["transform"]["scale_x"].as_f64() == Some(f64::from(scale))
                            && layer["transform"]["scale_y"].as_f64() == Some(f64::from(scale));
                        ensure!(
                            equal,
                            "Circus native zero-duration full-image animation has undefined velocity"
                        );
                        return Ok(true);
                    }
                    self.native_visual(&VisualRequest::Move {
                        object: 2,
                        position: *position,
                        ticks: *ticks,
                        cancel: false,
                    })?;
                    self.native_visual(&VisualRequest::Track {
                        object: 2,
                        property: VisualProperty::Scale,
                        value: scale,
                        ticks: *ticks,
                    })?;
                } else {
                    self.native_visual(&VisualRequest::Position {
                        object: 2,
                        position: *position,
                        cancel: false,
                    })?;
                    let layer: Value = serde_json::from_slice(
                        &self
                            .host
                            .query_engine(self.client, EngineQuery::Layer(id))?,
                    )?;
                    let mut transform = layer["transform"].clone();
                    transform["scale_x"] = json!(scale);
                    transform["scale_y"] = json!(scale);
                    self.submit(
                        vec![json!({"op":"set_transform","id":id,"transform":transform})],
                        vec![],
                    )?;
                    let pivot = self
                        .services
                        .sprite_scales
                        .get(&3)
                        .map_or([0; 2], |(_, pivot)| *pivot);
                    self.services.sprite_scales.insert(3, (*percent, pivot));
                }
                Ok(true)
            }
            EffectRequest::RadialControl {
                origin,
                angles,
                ticks,
            } => {
                if !self.effects_ready()? {
                    return Ok(false);
                }
                let control = crysirror_vfx::orbit_atlas::OrbitControl {
                    origin: origin.map(|value| value as f32),
                    angle: angles[0] as f32,
                    target: (angles[1] != -1).then_some(angles[1] as f32),
                    ticks: *ticks,
                };
                control.validate()?;
                if self.services.effect_control.orbit_pending {
                    ensure!(
                        self.services.effect_control.orbit == control,
                        "Circus orbit-control completion ownership"
                    );
                    self.services.effect_control.orbit_pending = false;
                    return Ok(true);
                }
                self.services.effect_control.orbit = control.clone();
                if let Some(effect) = &mut self.services.effects {
                    if effect
                        .spec
                        .emitters
                        .iter()
                        .any(|emitter| emitter.orbit_atlas.is_some())
                    {
                        for emitter in &mut effect.spec.emitters {
                            if let Some(node) = &mut emitter.orbit_atlas {
                                node.control = control.clone();
                            }
                        }
                        let crysirror_vfx::falling_atlas::SimulationClock::External {
                            tick, ..
                        } = effect.spec.clock
                        else {
                            bail!("Circus orbit-control clock")
                        };
                        let epoch =
                            effect
                                .spec
                                .sequence_exchange
                                .as_ref()
                                .map_or(Ok(1), |exchange| {
                                    exchange
                                        .epoch
                                        .checked_add(1)
                                        .context("Circus orbit exchange overflow")
                                })?;
                        effect.spec.sequence_exchange =
                            Some(crysirror_vfx::falling_atlas::SequenceExchange {
                                epoch,
                                base_tick: tick,
                                state: self
                                    .services
                                    .sequence_rng
                                    .clone()
                                    .context("Circus orbit shared sequence")?,
                            });
                        let spec = serde_json::to_value(&effect.spec)?;
                        self.host.submit_engine(
                            self.client,
                            1,
                            &serde_json::to_vec(
                                &json!({"world":[{"op":"set_vfx","id":0x3900_0000u32,"vfx":spec}]}),
                            )?,
                        )?;
                        self.services.effect_control.orbit_pending = true;
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            EffectRequest::StageBatch {
                operations,
                ticks,
                references,
            } => self.stage_batch(operations, *ticks, references),
        }
    }
    pub(super) fn advance_effect_controls(&mut self) -> Result<()> {
        self.advance_stage_views()?;
        self.advance_mask_framebuffer()?;
        self.advance_flat_framebuffer()?;
        self.advance_tile_mask()?;
        if let Some(pulse) = &mut self.services.effect_control.flash {
            let physical_fast =
                self.input.buttons_down & crysirror_core::service::buttons::CTRL != 0;
            let fast = (self.services.effect_control.frame_fast || physical_fast)
                && !self.services.effect_control.suppress_fast_color;
            let skip = self.services.effect_control.frame_skip_allowed
                && self.input.buttons_pressed
                    & (crysirror_core::service::buttons::MOUSE_RIGHT
                        | crysirror_core::service::buttons::ESCAPE)
                    != 0;
            let opacity = pulse
                .advance_by(if fast { 2 } else { 1 }, skip)?
                .unwrap_or(0.);
            let active = pulse.active();
            if opacity == 0. && active && self.services.effect_control.framebuffer.detached {
                self.services.effect_control.framebuffer.detached = false;
                self.submit(
                    vec![json!({"op":"set_visible","id":0x3b01_0000u32,"visible":true})],
                    vec![],
                )?;
            }
            self.submit(
                vec![
                    json!({"op":"set_opacity","id":0x3b01_0000u32,"opacity":(opacity*255.) as u8}),
                ],
                vec![],
            )?;
            if !active {
                self.services.effect_control.flash = None;
                self.submit(vec![json!({"op":"remove","id":0x3b01_0000u32})], vec![])?;
            }
        }
        if let Some(motion) = &mut self.services.effect_control.pointer {
            if let Some(point) = motion.advance() {
                self.submit(vec![], vec![json!({"op":"warp_pointer","position":point})])?;
            }
            if self
                .services
                .effect_control
                .pointer
                .as_ref()
                .is_some_and(|motion| !motion.active())
            {
                self.services.effect_control.pointer = None;
            }
        }
        Ok(())
    }
}

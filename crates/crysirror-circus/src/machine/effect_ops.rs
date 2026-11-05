//! Native effect declarations. Shared workers own particle integration and
//! Layer motion; this module preserves original sequencing and scalar policy.
use super::*;
#[path = "framebuffer_ops.rs"]
mod framebuffer;
#[derive(Clone, Debug)]
pub enum EffectRequest {
    Finish,
    FinishObjects {objects:Vec<u16>},
    SpriteMask {
        seed: super::sprite_ops::SpriteSeed,
        mode: bool,
        ticks: i32,
    },
    ResetObjects {
        objects: Vec<u16>,
    },
    AuxCapture {
        enabled: bool,
        repeats: u16,
        forever: bool,
        replay: bool,
    },
    CaptureFrame {
        crop: [i32; 4],
    },
    FlatFade {
        out: bool,
        ticks: i32,
        color: [u8; 3],
        protected: bool,
    },
    TileMask {
        pattern: i16,
        color: Option<[u8; 3]>,
    },
    CancelColor,
    ResetColor,
    ClearFrame {
        protected: bool,
    },
    MaskFade {
        reverse: bool,
        ticks: i32,
        color: [u8; 3],
    },
    PartialFallingClear,
    FrameControl {
        fast: bool,
        suppress_fast_color: bool,
        allow_skip: bool,
        effects_enabled: bool,
    },
    Flash {
        repeats: u16,
    },
    SpawnWindow {
        bounds: [i32; 4],
        chance: i32,
    },
    PointerMove {
        target: [i32; 2],
        steps: u16,
    },
    FullImage {
        position: [i32; 2],
        percent: i16,
        ticks: Option<u32>,
    },
    RadialControl {
        origin: [i32; 2],
        angles: [i32; 2],
        ticks: i32,
    },
    StageBatch {
        operations: Vec<StageOperation>,
        ticks: i32,
        references: Vec<StageReference>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StageOperation {
    pub mode: i16,
    pub selector: i16,
    pub offset: [i32; 2],
    pub zoom: i32,
    pub restore_scale: bool,
    pub restore_position: bool,
    pub apply_view: bool,
}
#[derive(Clone, Debug)]
pub struct StageReference {
    pub object: u16,
    pub position: [f32; 2],
    pub scale: [f32; 2],
}
#[derive(Clone, Debug)]
pub struct StageSample {
    pub object: u16,
    pub scale: [f32; 2],
    pub pivot: [f32; 2],
    pub enabled: bool,
    pub declared: bool,
    pub sequence: crysirror_scene::layer::queued::SequenceStatus,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct EffectState {
    bounds: [i32; 4],
    chance: i32,
    duration: i16,
    pending: Vec<StageOperation>,
    pan: Option<StageOperation>,
    zoom: Option<StageOperation>,
    zoom_count: u8,
    #[serde(default)]
    immediate_stage_pc: Option<u32>,
    origin: [i32; 2],
    angles: [i32; 2],
    radial_ticks: i32,
    pub(super) frame_image_ticks: i32,
    #[serde(default)]
    framebuffer_started: Option<u32>,
    #[serde(default)]
    framebuffer_active: bool,
    #[serde(default)]
    framebuffer_clocked: bool,
    #[serde(default)]
    fadeout: Option<bool>,
}
impl Default for EffectState {
    fn default() -> Self {
        Self {
            bounds: [0; 4],
            chance: 0,
            duration: 5,
            pending: Vec::new(),
            pan: None,
            zoom: None,
            zoom_count: 0,
            immediate_stage_pc: None,
            origin: [0; 2],
            angles: [0, 0],
            radial_ticks: 0,
            frame_image_ticks: -1,
            framebuffer_started: None,
            framebuffer_active: false,
            framebuffer_clocked: false,
            fadeout: None,
        }
    }
}
fn primary_move(
    position: [f32; 2],
    ticks: i32,
) -> crysirror_scene::layer::queued::SequenceOperation {
    serde_json::from_value(serde_json::json!({"kind":"enqueue","lane":0,"track":{"kind":"linear","channels":[0,1],"target":position,"ticks":ticks.unsigned_abs(),"velocity_divisor":ticks as f32,"references":[0,1],"loop_origins":[0,1],"delta":"retained","complete_first":false,"complete_zero":true,"fast":true,"skip_zero":true}})).expect("static primary move schema")
}
fn primary_scale(percent: f32, ticks: i32) -> crysirror_scene::layer::queued::SequenceOperation {
    serde_json::from_value(serde_json::json!({"kind":"enqueue","lane":4,"track":{"kind":"linear","channels":[3,4],"target":[percent,percent],"ticks":ticks.unsigned_abs(),"velocity_divisor":ticks as f32,"references":[3,3],"loop_origins":[3,4],"delta":"current_at_zero_bank","complete_first":true,"complete_zero":false,"fast":true,"skip_zero":true}})).expect("static primary scale schema")
}
impl Machine {
    pub(super) fn execute_effects(
        &mut self,
        _program: &Program,
        i: &Instruction,
        a: [i16; 8],
        _text: &str,
        host: &mut dyn NativeHost,
        _now: u64,
    ) -> Result<Option<CommandResult>> {
        if i.opcode == 0x0f {
            return self.execute_framebuffer(_program, i, a, _text, host, _now);
        }
        if i.opcode == 0x9d && self.state.effect_ops.immediate_stage_pc == Some(i.offset) {
            if !self.apply_history_stage(host)? {
                return Ok(Some(CommandResult::Retry));
            }
            self.state.native_live.stage_opcode_completed(0x9d, false)?;
            return Ok(Some(CommandResult::Complete {
                next: i.next,
                outcome: Outcome::Advanced,
            }));
        }
        if i.opcode == 0x9d
            && a[1] == 100
            && self.state.dispatch_a2 == 0
            && self.state.effect_ops.zoom_count == 0
            && self.state.native_live.stage_zoom_idle()?
        {
            self.state.native_live.stage_opcode_completed(0x9d, true)?;
            return Ok(Some(CommandResult::Complete {
                next: i.next,
                outcome: Outcome::Advanced,
            }));
        }
        if matches!(i.opcode, 0x9a..=0x9d) {
            if !self.finish_framebuffer_actors(host, &(0..150).collect::<Vec<u16>>(), false)? {
                return Ok(Some(CommandResult::Retry));
            }
            //422650 finalizes old sprite animation declarations before the
            // next stage batch is prepared. It does not itself apply the batch.
            if !host.effect_control(&EffectRequest::Finish)? {
                return Ok(Some(CommandResult::Retry));
            }
            self.sync_effect_frame(&host.effect_states()?)?;
        }
        let request = match i.opcode {
            0x99 => {
                self.state.effect_ops.duration = a[0];
                None
            }
            0x9a | 0x9c => {
                let offset = if i.opcode == 0x9c {
                    [-i32::from(a[2]), -i32::from(a[3])]
                } else {
                    self.state
                        .effect_ops
                        .pan
                        .as_ref()
                        .map_or([1, 1], |operation| operation.offset)
                };
                self.state.effect_ops.pan = Some(StageOperation {
                    mode: a[0],
                    selector: if i.opcode == 0x9c { 255 } else { a[1] },
                    offset,
                    zoom: -1,
                    restore_scale: false,
                    restore_position: true,
                    apply_view: true,
                });
                if i.opcode == 0x9c {
                    self.state.native_flags.insert(66513, 1);
                    self.state.native_live.stage_pending(true, false);
                }
                None
            }
            0x9b => {
                let previous = self.state.effect_ops.zoom.as_ref();
                self.state.effect_ops.zoom = Some(StageOperation {
                    mode: a[0],
                    selector: 255,
                    offset: previous.map_or([0; 2], |operation| operation.offset),
                    zoom: a[1].into(),
                    restore_scale: true,
                    restore_position: previous.is_some_and(|operation| operation.apply_view),
                    apply_view: previous.is_some_and(|operation| operation.apply_view),
                });
                self.state.native_flags.insert(66513, 1);
                self.state.native_live.stage_pending(true, a[1] == 100);
                if a[1] == 100 {
                    self.state.native_flags.insert(66512, 0);
                }
                None
            }
            0x9d => {
                //The source writes both save flags before testing whether the
                //ten dispatcher-local declarations have already filled up.
                self.state
                    .native_flags
                    .insert(66513, i32::from(a[1] != 100));
                self.state
                    .native_live
                    .stage_pending(a[1] != 100, a[1] == 100);
                if a[1] == 100 {
                    self.state.native_flags.insert(66512, 0);
                }
                let history = self.state.dispatch_a2 != 0;
                if history && a[1] == 100 || self.state.effect_ops.zoom_count < 10 {
                    let clamp = |value: i16, limit: i32| i32::from(value).clamp(-limit, limit);
                    let operation = StageOperation {
                        mode: a[0],
                        selector: 255,
                        offset: [-clamp(a[2], 639), -clamp(a[3], 359)],
                        zoom: a[1].into(),
                        restore_scale: true,
                        restore_position: true,
                        apply_view: true,
                    };
                    if history && a[1] == 100 || self.state.effect_ops.zoom_count == 0 {
                        self.state.effect_ops.zoom = Some(operation)
                    } else {
                        self.state.effect_ops.pending.push(operation)
                    }
                    if !(history && a[1] == 100) {
                        self.state.effect_ops.zoom_count += 1;
                    }
                    if history {
                        //4284B0 case157 immediately executes4253A0(1). Its
                        //100% branch replaces the main declaration, then the
                        //helper consumes and clears all prior declarations.
                        self.state.effect_ops.immediate_stage_pc = Some(i.offset);
                    }
                }
                None
            }
            0xa4 => Some(EffectRequest::SpawnWindow {
                bounds: [a[0].into(), a[1].into(), a[2].into(), a[3].into()],
                chance: self.state.effect_ops.chance,
            }),
            0xa5 => Some(EffectRequest::SpawnWindow {
                bounds: self.state.effect_ops.bounds,
                chance: a[0].into(),
            }),
            0xa2 => Some(EffectRequest::RadialControl {
                origin: if a[0] == 0 {
                    [a[1].into(), a[2].into()]
                } else {
                    self.state.effect_ops.origin
                },
                angles: if a[0] != 0 {
                    [a[1].into(), a[2].into()]
                } else {
                    self.state.effect_ops.angles
                },
                ticks: if a[0] != 0 {
                    12 * i32::from(a[3])
                } else {
                    self.state.effect_ops.radial_ticks
                },
            }),
            0xaa => {
                if self.state.value(99, 205)? != 0 {
                    Some(EffectRequest::PointerMove {
                        target: [a[0].into(), a[1].into()],
                        steps: 10,
                    })
                } else {
                    None
                }
            }
            0xac => match a[3] {
                0 => {
                    let percent = i32::from(a[2]);
                    let inset = [
                        (1280 - 1280 * percent / 100) / 2,
                        (720 - 720 * percent / 100) / 2,
                    ];
                    Some(EffectRequest::FullImage {
                        position: [-i32::from(a[0]) - inset[0], -i32::from(a[1]) - inset[1]],
                        percent: a[2],
                        ticks: None,
                    })
                }
                1 => {
                    if self.state.effect_ops.frame_image_ticks == -1 {
                        self.state.effect_ops.frame_image_ticks = 30;
                    }
                    let ticks = self.state.effect_ops.frame_image_ticks;
                    let command = super::sprite_ops::SpriteCommand {
                        seed: self.state.native_live.sprite_seed(2)?,
                        operations: vec![
                            primary_move([f32::from(a[0]), f32::from(a[1])], ticks),
                            primary_scale(f32::from(a[2]), ticks),
                        ],
                    };
                    if !host.sprite(&super::sprite_ops::SpriteRequest {
                        commands: vec![command],
                    })? {
                        return Ok(Some(CommandResult::Retry));
                    }
                    self.sync_sprite_states(host)?;
                    None
                }
                2 => {
                    self.state.effect_ops.frame_image_ticks = a[0].into();
                    None
                }
                _ => None,
            },
            _ => return Ok(None),
        };
        if let Some(request) = request {
            if !host.effect_control(&request)? {
                return Ok(Some(CommandResult::Retry));
            }
            if let EffectRequest::SpawnWindow { bounds, chance } = request {
                self.state.effect_ops.bounds = bounds;
                self.state.effect_ops.chance = chance;
            }
            if let EffectRequest::RadialControl {
                origin,
                angles,
                ticks,
            } = request
            {
                self.state.effect_ops.origin = origin;
                self.state.effect_ops.angles = angles;
                self.state.effect_ops.radial_ticks = ticks;
                self.state.native_live.radial_control(
                    (a[0] == 0).then_some(origin),
                    (a[0] != 0).then_some((angles, ticks)),
                );
            }
            if let EffectRequest::FullImage {
                position,
                percent,
                ticks: None,
            } = request
            {
                self.state
                    .native_live
                    .full_image_immediate(position, percent);
            }
        }
        if self.state.effect_ops.immediate_stage_pc == Some(i.offset)
            && !self.apply_history_stage(host)?
        {
            return Ok(Some(CommandResult::Retry));
        }
        if matches!(i.opcode, 0x9a..=0x9d) {
            self.state
                .native_live
                .stage_opcode_completed(i.opcode, false)?;
        }
        Ok(Some(CommandResult::Complete {
            next: i.next,
            outcome: Outcome::Advanced,
        }))
    }
    fn apply_history_stage(&mut self, host: &mut dyn NativeHost) -> Result<bool> {
        //4253A0(1) forces every consumed main/extra operation to immediate
        //mode0. A service Pending must retain ownership of this declaration,
        //rather than appending it again when the instruction is retried.
        if let Some(pan) = &mut self.state.effect_ops.pan {
            pan.mode = 0;
        }
        if let Some(zoom) = &mut self.state.effect_ops.zoom {
            zoom.mode = 0;
        }
        for operation in &mut self.state.effect_ops.pending {
            operation.mode = 0;
        }
        if !self.prepare_effects_frame(host)? {
            return Ok(false);
        }
        self.sync_effect_frame(&host.effect_states()?)?;
        self.state.effect_ops.immediate_stage_pc = None;
        Ok(true)
    }
    pub(crate) fn reset_resource_stage(&mut self, host: &mut dyn NativeHost) -> Result<bool> {
        let mut active = false;
        for object in [10, 20, 30, 40, 50] {
            let record = self.state.native_live.object_record(object)?;
            active |= f32::from_le_bytes(record[728..732].try_into().unwrap()) != 100.
                || f32::from_le_bytes(record[28..32].try_into().unwrap()) != 1.
                || u32::from_le_bytes(record[732..736].try_into().unwrap()) != 0;
        }
        if active {
            self.state.effect_ops.pending.clear();
            self.state.effect_ops.zoom_count = 0;
            self.state.effect_ops.zoom = Some(StageOperation {
                mode: 0,
                selector: 255,
                offset: [0, 0],
                zoom: 100,
                restore_scale: true,
                restore_position: true,
                apply_view: true,
            });
            if !self.prepare_effects_frame(host)? {
                return Ok(false);
            }
        }
        self.state.effect_ops.pending.clear();
        self.state.effect_ops.zoom_count = 0;
        self.state.effect_ops.zoom = None;
        self.state.effect_ops.pan = None;
        Ok(true)
    }
    pub(crate) fn prepare_effects_frame(&mut self, host: &mut dyn NativeHost) -> Result<bool> {
        if !host.effect_control(&EffectRequest::FrameControl {
            fast: self.state.value(99, 214)? != 0 || self.state.value(99, 195)? != 0,
            suppress_fast_color: self.state.value(9, 99)? != 0,
            allow_skip: self.state.value(99, 192)? == 0,
            effects_enabled: self.state.value(99, 10)? != 0,
        })? {
            return Ok(false);
        }
        if self.state.effect_ops.pending.is_empty()
            && self.state.effect_ops.pan.is_none()
            && self.state.effect_ops.zoom.is_none()
        {
            return Ok(true);
        }
        let mut references = Vec::new();
        for object in [0, 10, 20, 30, 40, 50] {
            let (position, scale) = self.state.native_live.stage_reference(object)?;
            references.push(StageReference {
                object,
                position,
                scale,
            });
        }
        let mut operations = Vec::new();
        if let Some(zoom) = &self.state.effect_ops.zoom {
            let mut operation = zoom.clone();
            operation.apply_view = false;
            operation.restore_position = false;
            operations.push(operation);
        }
        if let Some(pan) = &self.state.effect_ops.pan {
            operations.push(pan.clone());
        } else if let Some(zoom) = &self.state.effect_ops.zoom {
            if zoom.apply_view {
                let mut operation = zoom.clone();
                operation.restore_scale = false;
                operations.push(operation);
            }
        }
        operations.extend(self.state.effect_ops.pending.clone());
        let request = EffectRequest::StageBatch {
            operations: operations.clone(),
            ticks: 12 * i32::from(self.state.effect_ops.duration),
            references: references.clone(),
        };
        let mut commands = Vec::new();
        for operation in &operations {
            if operation.mode == 3 {
                for reference in &references {
                    let mut steps = Vec::new();
                    if operation.restore_position
                        && ((0..=5).contains(&operation.selector) || operation.selector == 255)
                    {
                        steps.push(primary_move(
                            reference.position,
                            12 * i32::from(self.state.effect_ops.duration),
                        ));
                    }
                    if operation.restore_scale {
                        steps.push(primary_scale(
                            reference.scale[0],
                            12 * i32::from(self.state.effect_ops.duration),
                        ));
                    }
                    if !steps.is_empty() {
                        commands.push(super::sprite_ops::SpriteCommand {
                            seed: self.state.native_live.sprite_seed(reference.object)?,
                            operations: steps,
                        });
                    }
                }
            }
        }
        if !host.effect_control(&request)? {
            return Ok(false);
        }
        if !commands.is_empty() {
            if !host.sprite(&super::sprite_ops::SpriteRequest { commands })? {
                return Ok(false);
            }
            self.sync_sprite_states(host)?;
        }
        for operation in &operations {
            if operation.mode >= 2 {
                for reference in &references {
                    let position = (operation.restore_position
                        && ((0..=5).contains(&operation.selector) || operation.selector == 255))
                        .then_some(reference.position);
                    let scale = operation.restore_scale.then_some(reference.scale);
                    if operation.mode == 3 {
                        self.state.native_live.stage_reference_cache(
                            reference.object,
                            position,
                            scale,
                        )?;
                    } else {
                        self.state.native_live.stage_reference_commit(
                            reference.object,
                            position,
                            scale,
                        )?;
                    }
                }
            }
        }
        self.state.effect_ops.pending.clear();
        self.state.effect_ops.pan = None;
        self.state.effect_ops.zoom = None;
        self.state.effect_ops.zoom_count = 0;
        Ok(true)
    }
    pub(crate) fn sync_effect_frame(&mut self, samples: &[StageSample]) -> Result<()> {
        for sample in samples {
            self.state.native_live.stage_auxiliary_sample(
                sample.object,
                sample.scale,
                sample.pivot,
                sample.enabled,
                sample.declared,
                &sample.sequence,
            )?;
            self.state
                .native_live
                .stage_sequence_status(sample.object, &sample.sequence)?;
        }
        Ok(())
    }
    pub(super) fn commit_framebuffer_clear(&mut self, protected: bool) -> Result<()> {
        self.state
            .native_live
            .framebuffer_fadeout_complete(protected)?;
        if !protected {
            self.state
                .image_names
                .retain(|slot, _| (37..=44).contains(slot));
            self.state.pending_images.clear();
            self.state.builder = Group::default();
            self.state.current_button = None;
            self.state.dialogue.body.clear();
            self.state.dialogue.speaker.clear();
            self.state.assign(99, 246, 0)?;
            self.state.effect_ops.framebuffer_active = false;
            self.state.effect_ops.framebuffer_clocked = false;
        }
        for flag in [66336, 66337, 66339, 66340, 66341, 66342] {
            self.state.native_flags.insert(flag, 0);
        }
        Ok(())
    }
    pub(crate) fn framebuffer_waiting(&self) -> bool {
        self.state.effect_ops.framebuffer_active && self.state.effect_ops.framebuffer_clocked
    }
    pub(crate) fn poll_framebuffer(&mut self, host: &mut dyn NativeHost) -> Result<bool> {
        if !self.framebuffer_waiting() {
            return Ok(true);
        }
        let Some(reply) = host.system(super::system_ops::SystemRequest::FramebufferStatus)? else {
            return Ok(false);
        };
        let super::system_ops::SystemReply::FramebufferComplete(complete) = reply else {
            bail!("Circus framebuffer status receipt");
        };
        if !complete {
            return Ok(false);
        }
        if let Some(protected) = self.state.effect_ops.fadeout.take() {
            self.commit_framebuffer_clear(protected)?;
        }
        self.state.effect_ops.framebuffer_active = false;
        self.state.effect_ops.framebuffer_clocked = false;
        Ok(true)
    }
    pub(crate) fn clear_effect_pools(&mut self) {
        self.state.effect_ops.bounds = [0; 4];
        self.state.effect_ops.chance = 0;
    }
    pub(crate) fn reset_effect_dispatch(&mut self) {
        if self.state.effect_ops.framebuffer_active {
            self.state.effect_ops.framebuffer_clocked = true;
        }
        self.state.effect_ops.frame_image_ticks = -1;
    }
}

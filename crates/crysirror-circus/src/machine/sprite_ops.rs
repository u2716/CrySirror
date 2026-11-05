//! Native sprite declarations and owner fences. Incremental sampling belongs
//! to the engine's retained channel queues, never to this MES interpreter.
use super::*;
#[path="portrait_ops.rs"]
mod portraits;
use crysirror_scene::layer::queued::{
    DeltaSource, SequenceOperation, SequenceStatus, SequenceTrack,
};

pub const CHANNELS: usize = 17;
#[derive(Clone, Debug)]
pub struct SpriteSeed {
    pub object: u16,
    pub values: Vec<f32>,
    pub retained: Vec<f32>,
}
#[derive(Clone, Debug)]
pub struct SpriteCommand {
    pub seed: SpriteSeed,
    pub operations: Vec<SequenceOperation>,
}
#[derive(Clone, Debug)]
pub struct SpriteRequest {
    pub commands: Vec<SpriteCommand>,
}
#[derive(Clone, Debug)]
pub struct SpriteResourceBinding {
    pub slot: u8,
    pub object: u16,
    pub crop: Option<[i32; 4]>,
}
#[derive(Clone, Debug)]
pub struct SpriteResourceRequest {
    pub bindings: Vec<SpriteResourceBinding>,
}
#[derive(Clone, Debug)]
pub struct SpriteResourceReply {
    pub dimensions: BTreeMap<u16, Option<([u32; 2], [u32; 2])>>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct SpriteState {
    pub capture: bool,
    ///445AA0 does not clear1912 when declaring a later finite replay count.
    pub forever: bool,
    pub oscillation_ticks: u32,
    pub oscillation_cosine: bool,
    pub extra_rate: Option<f32>,
    pub relative_origins: BTreeMap<u16, [f32; 2]>,
    pub relative_active: BTreeSet<u16>,
    pub decorative: Option<DecorativePending>,
    pub decorative_active: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct DecorativePending {
    pc: u32,
    args: [i16; 8],
    draws: Vec<i32>,
}
#[derive(Clone, Debug)]
struct Decoration {
    object: u16,
    crop: [i32; 4],
    position: [i32; 2],
    pivot: [f32; 2],
    scale: f32,
    rotation: f32,
    alpha: f32,
    move_to: Option<[i32; 2]>,
    rotate_to: Option<f32>,
    scale_to: Vec<(f32, u32)>,
    fade: bool,
    ticks: u32,
}
fn linear(
    channels: Vec<u8>,
    target: Vec<f32>,
    ticks: u32,
    references: Vec<u8>,
    loop_origins: Vec<u8>,
    delta: DeltaSource,
    complete_first: bool,
    complete_zero: bool,
) -> SequenceTrack {
    SequenceTrack::Linear {
        channels,
        target,
        ticks,
        velocity_divisor: None, ieee_inputs: false,
        references,
        loop_origins,
        delta,
        looping: false,
        start: None,
        complete_first,
        complete_zero,
        fast: true,
        fast_channels: None,
        skip_zero: true,
        capture_zero: true,
    }
}
impl Machine {
    ///421870 binds renderer85 to the retained system37 atlas and declares
    ///its native180-tick alpha bank. Resource absence means a null texture,
    ///not a fabricated notice image or a skipped numeric sprite mutation.
    pub(super) fn native_skip_notice(&mut self, host: &mut dyn NativeHost, mode: i16) -> Result<bool> {
        if !(0..=2).contains(&mode) { return Ok(true); }
        let crop = if mode == 2 { [0, 103, 174, 120] } else { [0, 17 * i32::from(mode) + 69, 109, 17 * i32::from(mode) + 86] };
        let Some(reply) = host.sprite_resources(&SpriteResourceRequest { bindings: vec![
            SpriteResourceBinding { slot: 37, object: 85, crop: Some(crop) },
        ] })? else { return Ok(false); };
        self.state.native_live.bind_resource(37, 85, Some(crop), reply.dimensions.get(&85).copied().flatten())?;
        let request = SpriteRequest { commands: vec![SpriteCommand {
            seed: self.state.native_live.sprite_seed(85)?,
            operations: vec![
                SequenceOperation::Set { channels: vec![0, 1], values: vec![if mode == 2 { 1106. } else { 1171. }, 520.], retained: true },
                SequenceOperation::Set { channels: vec![2, 14, 15], values: vec![1., (crop[2] - crop[0]) as f32, 17.], retained: false },
                SequenceOperation::Enqueue { lane: 3, bank: None, track: linear(vec![2], vec![0.], 180, vec![2], vec![2], DeltaSource::CurrentAtZeroBank, true, false) },
            ],
        }] };
        if !host.sprite(&request)? { return Ok(false); }
        self.sync_sprite_states(host)?;
        Ok(true)
    }

    pub(crate) fn finish_sprite_dispatch(&mut self) {
        self.state.sprite.oscillation_ticks = 0;
        self.state.sprite.oscillation_cosine = false;
        self.state.sprite.relative_origins.clear();
        self.state.sprite.relative_active.clear();
    }
    pub(super) fn execute_sprite(
        &mut self,
        _program: &Program,
        i: &Instruction,
        a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
        _now: u64,
    ) -> Result<Option<CommandResult>> {
        if let Some(result)=self.execute_portrait(i,a,host)? {return Ok(Some(result));}
        if i.opcode == 0x38 {
            return self.execute_decoration(i, a, host);
        }
        let mut sprite = self.state.sprite.clone();
        let mut commands = Vec::new();
        let mut complete_state = None;
        match i.opcode {
            0x36 => {
                for object in 0..150 {
                    commands.push(SpriteCommand {
                        seed: self.state.native_live.sprite_seed(object)?,
                        operations: vec![
                            SequenceOperation::Finish {
                                cancel: false,
                                lanes: None,
                            },
                            SequenceOperation::Capture {
                                enabled: sprite.capture,
                                repeats: 0,
                                forever: sprite.forever,
                            },
                        ],
                    });
                }
                let mut values = vec![0.; CHANNELS];
                values[16] = sprite.extra_rate.unwrap_or(1.);
                commands.push(SpriteCommand {
                    seed: SpriteSeed {
                        object: 150,
                        retained: values.clone(),
                        values,
                    },
                    operations: vec![SequenceOperation::Finish {
                        cancel: false,
                        lanes: None,
                    }],
                });
            }
            0x26 | 0xa7 | 0xa8 => {
                if i.opcode == 0x26 {
                    let Some(reply) = host.system(system_ops::SystemRequest::License)? else {
                        return Ok(Some(CommandResult::Retry));
                    };
                    ensure!(
                        matches!(reply, system_ops::SystemReply::Done),
                        "Circus sprite license readiness receipt"
                    );
                }
                let (capture, repeats, forever) = if i.opcode == 0xa8 {
                    (
                        false,
                        if a[0] == 255 {
                            1
                        } else {
                            u16::try_from(a[0]).context("Circus captured replay count")?
                        },
                        sprite.forever || a[0] == 255,
                    )
                } else {
                    (a[0] == 0, 0, sprite.forever)
                };
                for object in 0..150 {
                    let mut operations = vec![SequenceOperation::Capture {
                        enabled: capture,
                        repeats,
                        forever,
                    }];
                    if i.opcode == 0xa8 {
                        operations.push(SequenceOperation::Replay);
                    }
                    commands.push(SpriteCommand {
                        seed: self.state.native_live.sprite_seed(object)?,
                        operations,
                    });
                }
                complete_state = Some((capture, forever));
            }
            0x39 => {
                if a[0] > 100 {
                    for object in 0..150 {
                        commands.push(SpriteCommand {
                            seed: self.state.native_live.sprite_seed(object)?,
                            operations: vec![SequenceOperation::Set {
                                channels: vec![16],
                                values: vec![f32::from(a[0]) / 100.],
                                retained: true,
                            }],
                        });
                    }
                    let mut values = vec![0.; CHANNELS];
                    values[16] = sprite.extra_rate.unwrap_or(1.);
                    commands.push(SpriteCommand {
                        seed: SpriteSeed {
                            object: 150,
                            retained: values.clone(),
                            values,
                        },
                        operations: vec![SequenceOperation::Set {
                            channels: vec![16],
                            values: vec![a[0] as f32 / 100.],
                            retained: true,
                        }],
                    });
                }
            }
            0x41 if !text.starts_with('-') || (text != "-" && text != "--") => {} // exact original declaration default.
            0x4a if matches!(text, ":n" | ":r") => {
                let enabled = text == ":n";
                self.state.native_flags.insert(66512, i32::from(enabled));
                self.state.native_flags.insert(23908, i32::from(enabled));
                self.state.callbacks.clear();
                self.state
                    .native_live
                    .background_protection_opcode(0x4a, enabled)?;
            }
            0x89 | 0x94 | 0xa1 => {
                let (object, position, cancel, immediate, ticks) = match i.opcode {
                    0x89 => (
                        0,
                        [a[0] as f32, a[1] as f32],
                        a[2] != 1,
                        a[2] == 1,
                        match a[2] {
                            0 => 120,
                            99 => 12 * i32::from(a[3]),
                            _ => 15 * (i32::from(a[3]) + 4),
                        },
                    ),
                    0x94 => (
                        u16::try_from(a[0]).context("Circus indexed position")?,
                        [a[1] as f32, a[2] as f32],
                        true,
                        true,
                        0,
                    ),
                    _ => (
                        u16::try_from(i32::from(a[0]) + 3).context("Circus direct position")?,
                        [a[1] as f32, a[2] as f32],
                        false,
                        true,
                        0,
                    ),
                };
                let mut operations = Vec::new();
                if cancel {
                    operations.push(SequenceOperation::Finish {
                        cancel: false,
                        lanes: Some(vec![0]),
                    });
                }
                if immediate {
                    operations.push(SequenceOperation::Set {
                        channels: vec![0, 1],
                        values: position.to_vec(),
                        retained: true,
                    });
                } else {
                    operations.push(SequenceOperation::Enqueue {
                        lane: 0,
                        bank: None,
                        track: linear(
                            vec![0, 1],
                            position.to_vec(),
                            u32::try_from(ticks).context("Circus movement duration")?,
                            vec![0, 1],
                            vec![0, 1],
                            DeltaSource::Retained,
                            false,
                            true,
                        ),
                    });
                }
                commands.push(SpriteCommand {
                    seed: self.state.native_live.sprite_seed(object)?,
                    operations,
                });
            }
            0x8e if a[3] & 0x20 != 0 => {
                sprite.oscillation_ticks = if a[0] == 0 {
                    60
                } else {
                    u32::try_from(a[0]).context("Circus oscillation duration")?
                };
                sprite.oscillation_cosine = a[1] != 0;
            }
            0x8e if a[3] & 0x10 != 0 => {
                let object = u16::try_from(10 * (i32::from(a[3] & 15) + 1))
                    .context("Circus oscillation object")?;
                let cosine = sprite.oscillation_cosine;
                let offset = if i.opcode == 0x8e && cosine { 90. } else { 0. };
                let ticks = sprite.oscillation_ticks;
                ensure!(
                    ticks > 0,
                    "Circus oscillation declaration missing nonzero native duration"
                );
                let track = SequenceTrack::Oscillation {
                    channel: if cosine { 1 } else { 0 },
                    amplitude: a[0] as f32,
                    start_degrees: a[1] as f32 + offset,
                    end_degrees: a[2] as f32 + offset,
                    ticks,
                    cosine,
                    looping: false,
                    radians_per_degree: 0.01745327777777778_f64,
                    fast: true,
                };
                commands.push(SpriteCommand {
                    seed: self.state.native_live.sprite_seed(object)?,
                    operations: vec![SequenceOperation::Enqueue {
                        lane: if cosine { 2 } else { 1 },
                        bank: None,
                        track,
                    }],
                });
            }
            0x8e => {
                let relative = a[2] & 0x40 != 0;
                let group = if relative { a[2] & 15 } else { a[2] };
                let object = u16::try_from(10 * (i32::from(group) + 1))
                    .context("Circus character movement object")?;
                let seed = self.state.native_live.sprite_seed(object)?;
                let origin = if relative {
                    if !sprite.relative_active.contains(&object) {
                        sprite
                            .relative_origins
                            .insert(object, [seed.retained[0].trunc(), seed.retained[1].trunc()]);
                    }
                    sprite.relative_origins[&object]
                } else {
                    [0.; 2]
                };
                let target = vec![a[0] as f32 + origin[0], a[1] as f32 + origin[1]];
                let ticks = if a[3] == 0 {
                    60
                } else {
                    u32::try_from(6 * i32::from(a[3]))
                        .context("Circus character movement duration")?
                };
                commands.push(SpriteCommand {
                    seed,
                    operations: vec![SequenceOperation::Enqueue {
                        lane: 0,
                        bank: None,
                        track: linear(
                            vec![0, 1],
                            target,
                            ticks,
                            vec![0, 1],
                            vec![0, 1],
                            DeltaSource::Retained,
                            false,
                            true,
                        ),
                    }],
                });
                if relative {
                    sprite.relative_active.insert(object);
                } else {
                    sprite.relative_active.remove(&object);
                }
            }
            0x8f if a[3] & 0x30 == 0 => {
                let ticks = if a[3] == 0 {
                    60
                } else {
                    u32::try_from(6 * i32::from(a[3]))
                        .context("Circus foreground movement duration")?
                };
                commands.push(SpriteCommand {
                    seed: self.state.native_live.sprite_seed(63)?,
                    operations: vec![
                        SequenceOperation::Finish {
                            cancel: false,
                            lanes: Some(vec![0]),
                        },
                        SequenceOperation::Enqueue {
                            lane: 0,
                            bank: None,
                            track: linear(
                                vec![0, 1],
                                vec![a[0] as f32, a[1] as f32],
                                ticks,
                                vec![0, 1],
                                vec![0, 1],
                                DeltaSource::Retained,
                                false,
                                true,
                            ),
                        },
                    ],
                });
            }
            0x9f | 0xa0 => {
                let object =
                    u16::try_from(i32::from(a[0]) + 3).context("Circus image-slide object")?;
                let slot =
                    u8::try_from(i32::from(a[0]) + 21).context("Circus image-slide resource")?;
                ensure!(
                    object < 150 && slot < 51,
                    "Circus image-slide selector range"
                );
                if i.opcode == 0x9f {
                    let finish = SpriteRequest {
                        commands: vec![SpriteCommand {
                            seed: self.state.native_live.sprite_seed(object)?,
                            operations: vec![
                                SequenceOperation::Finish {
                                    cancel: false,
                                    lanes: None,
                                },
                                SequenceOperation::Capture {
                                    enabled: sprite.capture,
                                    repeats: 0,
                                    forever: sprite.forever,
                                },
                            ],
                        }],
                    };
                    if !host.sprite(&finish)? {
                        return Ok(Some(CommandResult::Retry));
                    }
                    self.sync_sprite_states(host)?;
                }
                if !host.sprite_resource_bound(object, slot)? {
                    let reset = crate::native_live::NativeLiveState::sprite_reset_seed(object)?;
                    if !host.sprite(&SpriteRequest {
                        commands: vec![SpriteCommand {
                            seed: self.state.native_live.sprite_seed(object)?,
                            operations: vec![SequenceOperation::Reset {
                                values: reset.values,
                                retained: reset.retained,
                            }],
                        }],
                    })? {
                        return Ok(Some(CommandResult::Retry));
                    }
                    self.state.native_live.reset_object(object)?;
                    let logical = host
                        .image_dimensions(slot)?
                        .map_or([0, 0], |(_, logical)| logical);
                    let crop = [0, 0, i32::try_from(logical[0])?, i32::try_from(logical[1])?];
                    let Some(reply) = host.sprite_resources(&SpriteResourceRequest {
                        bindings: vec![SpriteResourceBinding {
                            slot,
                            object,
                            crop: Some(crop),
                        }],
                    })?
                    else {
                        return Ok(Some(CommandResult::Retry));
                    };
                    let dimensions = reply
                        .dimensions
                        .get(&object)
                        .context("Circus image-slide binding receipt")?
                        .to_owned();
                    self.state
                        .native_live
                        .bind_resource(slot, object, Some(crop), dimensions)?;
                }
                let seed = self.state.native_live.sprite_seed(object)?;
                let extent = vec![seed.values[14], seed.values[15]];
                let target = if i.opcode == 0x9f {
                    vec![a[1] as f32, 0.]
                } else {
                    vec![0., a[1] as f32]
                };
                let mut track = linear(
                    vec![0, 1],
                    target,
                    u32::try_from(12 * i32::from(a[2])).context("Circus image-slide duration")?,
                    vec![0, 1],
                    vec![0, 1],
                    DeltaSource::Retained,
                    false,
                    true,
                );
                if let SequenceTrack::Linear { looping, .. } = &mut track {
                    *looping = a[3] != 0;
                }
                commands.push(SpriteCommand {
                    seed,
                    operations: vec![
                        SequenceOperation::Set {
                            channels: vec![14, 15],
                            values: extent,
                            retained: false,
                        },
                        SequenceOperation::Enqueue {
                            lane: 0,
                            bank: None,
                            track,
                        },
                    ],
                });
            }
            0xb1 => {
                if (0..=100).contains(&a[0]) {
                    if !host.visual(&visual_ops::VisualRequest::Blur {
                        from: a[0] * 10,
                        to: a[0] * 10,
                        ticks: 0,
                    })? {
                        return Ok(Some(CommandResult::Retry));
                    }
                    if (1..100).contains(&a[0]) {
                        for object in 0..60 {
                            commands.push(SpriteCommand {
                                seed: self.state.native_live.sprite_seed(object)?,
                                operations: vec![SequenceOperation::Set {
                                    channels: vec![10, 11],
                                    values: vec![1. / (a[0] as f32 * 10.); 2],
                                    retained: false,
                                }],
                            });
                        }
                    }
                }
            }
            0xb2 => {
                if a[..2].iter().all(|v| (0..=100).contains(v)) {
                    //447C10 uses true reciprocal endpoints and a signed
                    //duration; endpoint0 and duration0 retain IEEE bits.
                    let ticks = u32::from(a[2].unsigned_abs());
                    let mut track = linear(
                        vec![10, 11],
                        vec![1. / (a[1] as f32 * 10.); 2],
                        ticks,
                        vec![10, 10],
                        vec![10, 10],
                        DeltaSource::ExplicitStart,
                        true,
                        false,
                    );
                    if let SequenceTrack::Linear { start,velocity_divisor,ieee_inputs, .. } = &mut track {
                        *start = Some(vec![1. / (a[0] as f32 * 10.); 2]);
                        *velocity_divisor=Some(a[2] as f32);
                        *ieee_inputs=true;
                    }
                    for object in 0..60 {
                        commands.push(SpriteCommand {
                            seed: self.state.native_live.sprite_seed(object)?,
                            operations: vec![SequenceOperation::Enqueue {
                                lane: 10,
                                bank: None,
                                track: track.clone(),
                            }],
                        });
                    }
                }
            }
            0x95 if !matches!(a[3], 0..=7) => {
                commands.push(SpriteCommand {
                    seed: self.state.native_live.sprite_seed(
                        u16::try_from(a[0]).context("Circus default sprite selector")?,
                    )?,
                    operations: vec![
                        SequenceOperation::Finish {
                            cancel: false,
                            lanes: None,
                        },
                        SequenceOperation::Capture {
                            enabled: sprite.capture,
                            repeats: 0,
                            forever: sprite.forever,
                        },
                    ],
                });
            }
            0x95 if matches!(a[3], 0..=3) => {
                let object = u16::try_from(a[0]).context("Circus sprite object index")?;
                ensure!(object < 150, "Circus sprite object index");
                let (lane, track) = match a[3] {
                    0 => (
                        0,
                        linear(
                            vec![0, 1],
                            vec![a[1] as f32, a[2] as f32],
                            60,
                            vec![0, 1],
                            vec![0, 1],
                            DeltaSource::Retained,
                            false,
                            true,
                        ),
                    ),
                    1 => (
                        4,
                        linear(
                            vec![3, 4],
                            vec![a[1] as f32; 2],
                            60,
                            vec![3, 3],
                            vec![3, 3],
                            DeltaSource::CurrentAtZeroBank,
                            true,
                            false,
                        ),
                    ),
                    2 => (
                        6,
                        linear(
                            vec![5],
                            vec![a[1] as f32],
                            60,
                            vec![5],
                            vec![5],
                            DeltaSource::CurrentAtZeroBank,
                            true,
                            false,
                        ),
                    ),
                    3 => (
                        3,
                        linear(
                            vec![2],
                            vec![a[1] as f32],
                            60,
                            vec![2],
                            vec![2],
                            DeltaSource::CurrentAtZeroBank,
                            true,
                            false,
                        ),
                    ),
                    _ => unreachable!(),
                };
                commands.push(SpriteCommand {
                    seed: self.state.native_live.sprite_seed(object)?,
                    operations: vec![
                        SequenceOperation::Finish {
                            cancel: false,
                            lanes: None,
                        },
                        SequenceOperation::Capture {
                            enabled: sprite.capture,
                            repeats: 0,
                            forever: sprite.forever,
                        },
                        SequenceOperation::Enqueue {
                            lane,
                            bank: None,
                            track,
                        },
                    ],
                });
            }
            _ => return Ok(None),
        }
        if !commands.is_empty() {
            if !host.sprite(&SpriteRequest { commands })? {
                return Ok(Some(CommandResult::Retry));
            }
            self.sync_sprite_states(host)?;
            match i.opcode {
                0x94 => self.state.native_live.object_position(
                    0x94,
                    a[0] as u16,
                    [a[1] as i32, a[2] as i32],
                )?,
                0xa1 => self.state.native_live.object_position(
                    0xa1,
                    (i32::from(a[0]) + 3) as u16,
                    [a[1] as i32, a[2] as i32],
                )?,
                0x89 if a[2] == 1 => {
                    self.state
                        .native_live
                        .object_position(0x89, 0, [a[0] as i32, a[1] as i32])?
                }
                _ => {}
            }
        }
        if i.opcode == 0x36 {
            if !host.effect_control(&effect_ops::EffectRequest::Finish)? {
                return Ok(Some(CommandResult::Retry));
            }
            // The shared stage owner has finished; its exact source samples
            // must be synchronized before a following1c can capture this frame.
            self.sync_effect_frame(&host.effect_states()?)?;
            let Some(reply) = host.text(&text_ops::TextRequest::FinishDrawableQueues)? else {
                return Ok(Some(CommandResult::Retry));
            };
            ensure!(
                matches!(reply, text_ops::TextReply::Done),
                "Circus36 text drawable finish receipt"
            );
        }
        if i.opcode == 0xb1 && (0..=100).contains(&a[0]) {
            self.state.native_live.blur_immediate(a[0] * 10)?;
        }
        if i.opcode == 0x39 && a[0] > 100 {
            sprite.extra_rate = Some(a[0] as f32 / 100.);
        }
        if let Some((capture, forever)) = complete_state {
            sprite.capture = capture;
            sprite.forever = forever;
        }
        self.state.sprite = sprite;
        self.state.native_live.sprite_opcode_completed(i.opcode);
        Ok(Some(CommandResult::Complete {
            next: i.next,
            outcome: Outcome::Advanced,
        }))
    }
    pub(crate) fn sync_sprite_states(&mut self, host: &mut dyn NativeHost) -> Result<()> {
        let mut decoration_busy = false;
        let states=host.sprite_states()?;
        self.state.resources.remember_controls(&states);
        for (object, status) in states {
            if (object < 70 || object >= 96) && !status.complete {
                decoration_busy = true;
            }
            if object == 150 {
                continue;
            }
            self.state
                .native_live
                .sprite_sequence_status(object, &status)?;
        }
        if self.state.sprite.decorative_active {
            self.state
                .native_flags
                .insert(1584, i32::from(decoration_busy));
            if !decoration_busy {
                self.state.sprite.decorative_active = false;
            }
        }
        Ok(())
    }
    pub(crate) fn sprite_input_blocked(&self) -> bool {
        self.state.sprite.decorative_active
    }
}

impl Machine {
    fn execute_decoration(
        &mut self,
        i: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
    ) -> Result<Option<CommandResult>> {
        let pending = self
            .state
            .sprite
            .decorative
            .get_or_insert_with(|| DecorativePending {
                pc: i.offset,
                args: a,
                draws: Vec::new(),
            });
        ensure!(
            pending.pc == i.offset && pending.args == a,
            "Circus decoration declaration changed during dependency wait"
        );
        let moduli: Vec<u16> = match a[0] {
            0 => [1280, 848, 80, 3, 5].repeat(53),
            1 => [1280, 848, 80, 3, 5].repeat(52),
            3 | 4 => vec![6],
            5 => [1480, 1440, 8, 90, 360, 5].repeat(54),
            _ => Vec::new(),
        };
        while pending.draws.len() < moduli.len() {
            let modulus = moduli[pending.draws.len()];
            let Some(value) = host.random(modulus)? else {
                return Ok(Some(CommandResult::Retry));
            };
            ensure!(
                value >= 0 && value < i32::from(modulus),
                "Circus decoration CRT random range"
            );
            pending.draws.push(value);
        }
        let mut draws = pending.draws.iter().copied();
        let ticks = u32::try_from(12 * i32::from(a[1])).context("Circus decoration duration")?;
        if (0..=5).contains(&a[0]) {
            ensure!(
                ticks > 0,
                "Circus decoration zero-duration has undefined original velocity"
            );
        }
        let base = |object, crop, position, pivot| Decoration {
            object,
            crop,
            position,
            pivot,
            scale: 100.,
            rotation: 0.,
            alpha: 1.,
            move_to: None,
            rotate_to: None,
            scale_to: Vec::new(),
            fade: false,
            ticks,
        };
        let mut specs = Vec::new();
        match a[0] {
            0 | 1 => {
                let mut panel = base(96, [0, 0, 732, 720], [-732, 0], [360., 355.]);
                panel.move_to = Some([if a[0] == 0 { 1536 } else { 1288 }, 0]);
                panel.rotate_to = Some(540.);
                specs.push(panel);
                if a[0] == 1 {
                    let mut panel = base(97, [0, 0, 732, 720], [1288, 0], [360., 355.]);
                    panel.move_to = Some([-732, 0]);
                    panel.rotate_to = Some(540.);
                    specs.push(panel);
                }
                for object in if a[0] == 0 { 97..150 } else { 98..150 } {
                    let x = draws.next().unwrap();
                    let y = draws.next().unwrap() - 128;
                    let scale = draws.next().unwrap() + 20;
                    let extra = draws.next().unwrap();
                    let alpha = draws.next().unwrap() as f32 / 10. + 0.5;
                    let right = a[0] == 1 && object >= 125;
                    let mut particle = base(
                        object,
                        [720, 0, 998, 256],
                        [if right { x + 2080 } else { -x - 800 }, y],
                        [0.; 2],
                    );
                    particle.scale = scale as f32;
                    particle.alpha = alpha;
                    particle.ticks = u32::try_from(12 * (i32::from(a[1]) + extra))?;
                    particle.move_to = Some([
                        if right {
                            -800
                        } else if a[0] == 0 {
                            1536
                        } else {
                            1280
                        },
                        y,
                    ]);
                    particle.rotate_to = Some(540.);
                    particle.fade = true;
                    specs.push(particle);
                }
            }
            2 => {
                let mut panel = base(96, [0, 0, 732, 720], [280, 0], [360., 355.]);
                panel.scale = 1.;
                panel.scale_to = vec![(500., ticks), (550., 30), (0., ticks)];
                specs.push(panel);
            }
            3 | 4 => {
                let image = draws.next().unwrap();
                let left = 200 * image;
                let mut panel = base(
                    96,
                    [
                        left,
                        if a[0] == 3 { 250 } else { 0 },
                        left + 200,
                        if a[0] == 3 { 500 } else { 250 },
                    ],
                    [if a[0] == 3 { -200 } else { 1280 }, 40],
                    [if a[0] == 3 { 1000. } else { -800. }, -800.],
                );
                panel.rotate_to = Some(if a[0] == 3 { -80. } else { 80. });
                specs.push(panel);
            }
            5 => {
                for object in 96..150 {
                    let x = draws.next().unwrap() - 200;
                    let y = -draws.next().unwrap();
                    let image = draws.next().unwrap();
                    let scale = draws.next().unwrap() + 10;
                    let angle = draws.next().unwrap();
                    let alpha = draws.next().unwrap() as f32 / 10. + 0.2000000029802322_f32;
                    let crop = match image {
                        0 => [512, 0, 788, 260],
                        1 => [220, 0, 512, 260],
                        _ => [0, 0, 220, 260],
                    };
                    let mut particle = base(object, crop, [x, y], [0.; 2]);
                    particle.scale = scale as f32;
                    particle.rotation = angle as f32;
                    particle.rotate_to = Some((angle + 720) as f32);
                    particle.alpha = alpha;
                    particle.move_to = Some([x, y + 2160]);
                    specs.push(particle);
                }
            }
            _ => {}
        }
        if !specs.is_empty() {
            let finish = SpriteRequest {
                commands: specs
                    .iter()
                    .map(|spec| {
                        Ok(SpriteCommand {
                            seed: self.state.native_live.sprite_seed(spec.object)?,
                            operations: vec![SequenceOperation::Finish {
                                cancel: false,
                                lanes: None,
                            }],
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            };
            if !host.sprite(&finish)? {
                return Ok(Some(CommandResult::Retry));
            }
            self.sync_sprite_states(host)?;
            // Native445010 precedes443EA0: Reset detaches the old texture,
            // so doing it after the resource ACK would erase the new alias.
            let reset_commands = specs.iter().map(|spec| {
                let reset = crate::native_live::NativeLiveState::sprite_reset_seed(spec.object)?;
                Ok(SpriteCommand {
                    seed: self.state.native_live.sprite_seed(spec.object)?,
                    operations: vec![SequenceOperation::Reset {
                        values: reset.values,
                        retained: reset.retained,
                    }, SequenceOperation::Capture {
                        enabled: self.state.sprite.capture, repeats: 0, forever: false,
                    }],
                })
            }).collect::<Result<Vec<_>>>()?;
            if !host.sprite(&SpriteRequest { commands: reset_commands })? {
                return Ok(Some(CommandResult::Retry));
            }
            let request = SpriteResourceRequest {
                bindings: specs
                    .iter()
                    .map(|spec| SpriteResourceBinding {
                        slot: 22,
                        object: spec.object,
                        crop: Some(spec.crop),
                    })
                    .collect(),
            };
            let Some(reply) = host.sprite_resources(&request)? else {
                return Ok(Some(CommandResult::Retry));
            };
            let mut commands = Vec::new();
            for spec in &specs {
                let mut operations = vec![
                    SequenceOperation::Set {
                        channels: vec![0, 1],
                        values: spec.position.map(|v| v as f32).to_vec(),
                        retained: true,
                    },
                    SequenceOperation::Set {
                        channels: vec![2, 3, 4, 5, 12, 13, 14, 15],
                        values: vec![
                            spec.alpha,
                            spec.scale,
                            spec.scale,
                            spec.rotation,
                            spec.pivot[0],
                            spec.pivot[1],
                            (spec.crop[2] - spec.crop[0]) as f32,
                            (spec.crop[3] - spec.crop[1]) as f32,
                        ],
                        retained: false,
                    },
                ];
                if let Some(position) = spec.move_to {
                    operations.push(SequenceOperation::Enqueue {
                        lane: 0,
                        bank: None,
                        track: linear(
                            vec![0, 1],
                            position.map(|v| v as f32).to_vec(),
                            spec.ticks,
                            vec![0, 1],
                            vec![0, 1],
                            DeltaSource::Retained,
                            false,
                            true,
                        ),
                    });
                }
                if let Some(angle) = spec.rotate_to {
                    operations.push(SequenceOperation::Enqueue {
                        lane: 6,
                        bank: None,
                        track: linear(
                            vec![5],
                            vec![angle],
                            spec.ticks,
                            vec![5],
                            vec![5],
                            DeltaSource::CurrentAtZeroBank,
                            true,
                            false,
                        ),
                    });
                }
                for (scale, ticks) in &spec.scale_to {
                    operations.push(SequenceOperation::Enqueue {
                        lane: 4,
                        bank: None,
                        track: linear(
                            vec![3, 4],
                            vec![*scale; 2],
                            *ticks,
                            vec![3, 3],
                            vec![3, 3],
                            DeltaSource::CurrentAtZeroBank,
                            true,
                            false,
                        ),
                    });
                }
                if spec.fade {
                    operations.push(SequenceOperation::Enqueue {
                        lane: 3,
                        bank: None,
                        track: linear(
                            vec![2],
                            vec![0.],
                            spec.ticks,
                            vec![2],
                            vec![2],
                            DeltaSource::CurrentAtZeroBank,
                            true,
                            false,
                        ),
                    });
                }
                commands.push(SpriteCommand {
                    seed: self.state.native_live.sprite_seed(spec.object)?,
                    operations,
                });
            }
            if !host.sprite(&SpriteRequest { commands })? {
                return Ok(Some(CommandResult::Retry));
            }
            for spec in &specs {
                self.state.native_live.reset_object(spec.object)?;
                self.state.native_live.bind_resource(
                    22,
                    spec.object,
                    Some(spec.crop),
                    reply
                        .dimensions
                        .get(&spec.object)
                        .context("Circus decorative binding receipt")?
                        .to_owned(),
                )?;
            }
            self.sync_sprite_states(host)?;
        }
        self.state.sprite.decorative = None;
        self.state.sprite.decorative_active = true;
        self.state.native_flags.insert(1584, 1);
        self.state.native_live.sprite_opcode_completed(0x38);
        Ok(Some(CommandResult::Complete {
            next: i.next,
            outcome: Outcome::Advanced,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Host {
        draws: usize,
        stall_draw: bool,
        stall_resource: bool,
        requests: Vec<SpriteRequest>,
    }
    impl NativeHost for Host {
        fn scene(&mut self, _: &str) -> Result<Option<Arc<Program>>> {
            Ok(None)
        }
        fn emit(&mut self, _: &Event) -> Result<bool> {
            bail!("unexpected policy event")
        }
        fn save_exists(&mut self, _: u16) -> Result<bool> {
            Ok(false)
        }
        fn random(&mut self, modulus: u16) -> Result<Option<i32>> {
            if self.draws == 3 && self.stall_draw {
                self.stall_draw = false;
                return Ok(None);
            }
            let value = (self.draws % usize::from(modulus)) as i32;
            self.draws += 1;
            Ok(Some(value))
        }
        fn sprite(&mut self, request: &SpriteRequest) -> Result<bool> {
            self.requests.push(request.clone());
            Ok(true)
        }
        fn sprite_states(&mut self) -> Result<Vec<(u16, SequenceStatus)>> {
            Ok(Vec::new())
        }
        fn sprite_resources(
            &mut self,
            request: &SpriteResourceRequest,
        ) -> Result<Option<SpriteResourceReply>> {
            if self.stall_resource {
                self.stall_resource = false;
                return Ok(None);
            }
            Ok(Some(SpriteResourceReply {
                dimensions: request
                    .bindings
                    .iter()
                    .map(|binding| (binding.object, Some(([2048, 1024], [1280, 720]))))
                    .collect(),
            }))
        }
    }
    #[test]
    fn decoration_draws_survive_dependency_retries_without_entering_script_history() {
        let mut machine = Machine::new("scene");
        let mut host = Host {
            stall_draw: true,
            stall_resource: true,
            ..Default::default()
        };
        let instruction = Instruction {
            offset: 3,
            next: 12,
            opcode: 0x38,
            arguments: Arguments::Words([1, 2, 0, 0]),
            continuation: None,
        };
        let args = [1, 2, 0, 0, 0, 0, 0, 0];
        assert!(matches!(
            machine
                .execute_decoration(&instruction, args, &mut host)
                .unwrap(),
            Some(CommandResult::Retry)
        ));
        assert_eq!(host.draws, 3);
        assert!(matches!(
            machine
                .execute_decoration(&instruction, args, &mut host)
                .unwrap(),
            Some(CommandResult::Retry)
        ));
        assert_eq!(host.draws, 260);
        assert_eq!(
            machine
                .state
                .sprite
                .decorative
                .as_ref()
                .unwrap()
                .draws
                .len(),
            260
        );
        assert!(matches!(
            machine
                .execute_decoration(&instruction, args, &mut host)
                .unwrap(),
            Some(CommandResult::Complete { next: 12, .. })
        ));
        assert_eq!(host.draws, 260);
        assert!(machine.state.random_history.is_empty());
        let final_request = host.requests.last().unwrap();
        assert_eq!(final_request.commands.len(), 54);
        let position = |object| {
            final_request
                .commands
                .iter()
                .find(|command| command.seed.object == object)
                .unwrap()
                .operations
                .iter()
                .find_map(|operation| match operation {
                    SequenceOperation::Set {
                        channels, values, ..
                    } if channels == &[0, 1] => Some(values.clone()),
                    _ => None,
                })
                .unwrap()
        };
        // Independent native case56 oracle: two panels,27 left particles,
        //25 right particles, with five CRT draws per particle.
        assert_eq!(position(96), [-732., 0.]);
        assert_eq!(position(97), [1288., 0.]);
        assert_eq!(position(98), [-800., -127.]);
        assert_eq!(position(125), [2215., 8.]);
        assert!(machine.state.sprite.decorative.is_none());
        assert!(machine.sprite_input_blocked());
    }
}

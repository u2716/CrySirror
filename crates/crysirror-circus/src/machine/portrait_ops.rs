//! Native8B/8C selectors and entrance declarations; animation stays shared.
use super::*;
impl Machine {
    pub(super) fn execute_portrait(
        &mut self,
        i: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
    ) -> Result<Option<CommandResult>> {
        if !matches!(i.opcode, 0x8b | 0x8c) {
            return Ok(None);
        }
        let done = || {
            Some(CommandResult::Complete {
                next: i.next,
                outcome: Outcome::Advanced,
            })
        };
        if a[3] == 255 {
            self.state.native_live.sprite_opcode_completed(i.opcode);
            return Ok(done());
        }
        let negative = (a[3] as u8 as i8) < 0;
        if a[3] != 0 && !negative && a[3] & 0x70 == 0 {
            self.state.native_live.sprite_opcode_completed(i.opcode);
            return Ok(done());
        }
        if i.opcode == 0x8b
            && !negative
            && a[3] & 0x70 == 0x10
            && self.state.native_flags.get(&66513).copied().unwrap_or(0) != 0
        {
            self.state.native_live.sprite_opcode_completed(i.opcode);
            return Ok(done());
        }
        let object = if i.opcode == 0x8c {
            63
        } else if a[3] == 0 {
            u16::try_from(10 * (i32::from(a[2]) + 1))
                .context("Circus portrait position selector")?
        } else {
            10 * ((a[3] & 15) as u16 + 1)
        };
        ensure!(object < 150, "Circus portrait object index");
        let fast = self.state.native_skipping();
        let history = self.state.dispatch_a2 != 0;
        let slot = if i.opcode == 0x8c {
            25
        } else {
            u8::try_from(4 + (a[3] & 15))?
        };
        let mut commands = Vec::new();
        let mut position = None;
        let mut scale = None;
        if a[3] == 0 {
            commands.push(SpriteCommand {
                seed: self.state.native_live.sprite_seed(object)?,
                operations: vec![
                    SequenceOperation::Finish {
                        cancel: false,
                        lanes: Some(vec![0]),
                    },
                    SequenceOperation::Set {
                        channels: vec![0, 1],
                        values: vec![a[0] as f32, a[1] as f32],
                        retained: true,
                    },
                ],
            });
            position = Some([a[0] as i32, a[1] as i32]);
        } else if (a[3] as u8 as i8) < 0 {
            self.state.resources.clear_pending(slot);
            self.state.pending_images.remove(&slot);
            self.state.image_names.remove(&slot);
            self.state.native_live.image_name(slot, "")?;
            if history || fast {
                if host
                    .resource(&super::resource_ops::ResourceRequest::Unload { slot: 21 })?
                    .is_none()
                {
                    return Ok(Some(CommandResult::Retry));
                }
                self.state.native_live.unload_image(21)?;
                self.state.image_names.remove(&21);
                self.state.resources.stage_unload(slot);
                self.state.pending_images.insert(slot);
            } else {
                if !host.effect_control(&super::effect_ops::EffectRequest::SpriteMask {
                    seed: self.state.native_live.sprite_seed(object)?,
                    mode: false,
                    ticks: 12 * i32::from(a[1]),
                })? {
                    return Ok(Some(CommandResult::Retry));
                }
                self.sync_sprite_states(host)?;
            }
        } else if a[3] & 0x40 != 0 {
            if i.opcode == 0x8c && self.state.native_flags.get(&25104).copied().unwrap_or(0) != 0 {
                self.state.native_live.sprite_opcode_completed(i.opcode);
                return Ok(done());
            }
            if history || fast {
                if host
                    .resource(&super::resource_ops::ResourceRequest::Unload { slot: 21 })?
                    .is_none()
                {
                    return Ok(Some(CommandResult::Retry));
                }
                self.state.native_live.unload_image(21)?;
                self.state.image_names.remove(&21);
            } else {
                if !host.sprite(&SpriteRequest {
                    commands: vec![SpriteCommand {
                        seed: self.state.native_live.sprite_seed(object)?,
                        operations: vec![SequenceOperation::Finish {
                            cancel: false,
                            lanes: None,
                        }],
                    }],
                })? {
                    return Ok(Some(CommandResult::Retry));
                }
                self.sync_sprite_states(host)?;
                if a[0] == 3 {
                    if !host.effect_control(&super::effect_ops::EffectRequest::SpriteMask {
                        seed: self.state.native_live.sprite_seed(object)?,
                        mode: true,
                        ticks: 12 * i32::from(a[1]),
                    })? {
                        return Ok(Some(CommandResult::Retry));
                    }
                    self.sync_sprite_states(host)?;
                } else if (0..=2).contains(&a[0]) {
                    let seed = self.state.native_live.sprite_seed(object)?;
                    let target = [seed.values[0], seed.values[1]];
                    let ticks = if i.opcode == 0x8b {
                        12
                    } else if a[0] == 2 {
                        20
                    } else {
                        6
                    };
                    let slide = if i.opcode == 0x8b {
                        match a[0] {
                            1 => Some(-20.),
                            2 => Some(20.),
                            _ => None,
                        }
                    } else {
                        match a[0] {
                            0 => Some(-20.),
                            1 => Some(20.),
                            _ => None,
                        }
                    };
                    let mut operations = vec![
                        SequenceOperation::Set {
                            channels: vec![2],
                            values: vec![0.],
                            retained: false,
                        },
                        SequenceOperation::Enqueue {
                            lane: 3,
                            bank: None,
                            track: linear(
                                vec![2],
                                vec![1.],
                                ticks,
                                vec![2],
                                vec![2],
                                DeltaSource::CurrentAtZeroBank,
                                true,
                                false,
                            ),
                        },
                    ];
                    if let Some(offset) = slide {
                        operations.push(SequenceOperation::Set {
                            channels: vec![0, 1],
                            values: vec![target[0] + offset, target[1]],
                            retained: true,
                        });
                        operations.push(SequenceOperation::Enqueue {
                            lane: 0,
                            bank: None,
                            track: linear(
                                vec![0, 1],
                                target.to_vec(),
                                ticks,
                                vec![0, 1],
                                vec![0, 1],
                                DeltaSource::Retained,
                                false,
                                true,
                            ),
                        });
                    }
                    commands.push(SpriteCommand { seed, operations });
                } // Other styles are original explicit switch defaults.
            }
        } else if a[3] & 0x20 != 0 {
            if !host.visual(&super::visual_ops::VisualRequest::Priority {
                object,
                priority: a[0],
            })? {
                return Ok(Some(CommandResult::Retry));
            }
            self.state.native_live.object_priority(object, a[0])?;
        } else if a[3] & 0x10 != 0 {
            if i.opcode == 0x8c || self.state.native_flags.get(&66513).copied().unwrap_or(0) == 0 {
                commands.push(SpriteCommand {
                    seed: self.state.native_live.sprite_seed(object)?,
                    operations: vec![SequenceOperation::Set {
                        channels: vec![3, 4, 12, 13],
                        values: vec![a[0] as f32, a[0] as f32, a[1] as f32, a[2] as f32],
                        retained: false,
                    }],
                });
                scale = Some((a[0], [a[1], a[2]]));
            }
        } // Positive selectors lacking40/20/10 are native no-ops.
        if !commands.is_empty() {
            if !host.sprite(&SpriteRequest { commands })? {
                return Ok(Some(CommandResult::Retry));
            }
            self.sync_sprite_states(host)?;
        }
        if position.is_some() || scale.is_some() {
            self.state
                .native_live
                .portrait_pose(object, position, scale)?;
        }
        self.state.native_live.sprite_opcode_completed(i.opcode);
        Ok(done())
    }
}

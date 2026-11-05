//! Native resource selectors and dispatcher-local image-animation declarations.
use super::*;
#[derive(Clone, Debug)]
pub enum PackedRequest {
    Load {
        slot: u8,
        name: String,
    },
    Play {
        slot: u8,
        seed: sprite_ops::SpriteSeed,
        position: [i16; 2],
        hold_ticks: i16,
        looping: bool,
    },
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct PackedState {
    object: Option<u16>,
    slot_offset: i16,
    looping: bool,
}
impl Machine {
    fn packed_publication_receipt(
        &mut self,
        host: &mut dyn NativeHost,
        object: u16,
        slot: u8,
        hold_ticks: i16,
        looping: bool,
    ) -> Result<()> {
        let frames = host
            .packed_frame_count(slot)?
            .context("Circus packed frame count receipt missing")?;
        let (allocation, logical) = host
            .packed_dimensions(object)?
            .context("Circus packed image extent receipt missing")?;
        self.state
            .native_live
            .packed_frame_attached(object, allocation, logical)?;
        let lineage=host.packed_lineage(object)?.context("Circus packed lineage publication receipt")?;
        self.state.native_live.packed_declared(object, slot, frames, hold_ticks, looping,lineage)?;
        Ok(())
    }

    pub(super) fn reset_packed_dispatch(&mut self) {
        self.state.packed = PackedState::default();
    }
    pub(super) fn select_packed_object(&mut self, object: Option<u16>) {
        self.state.packed.object = object;
    }
    pub(super) fn execute_packed(
        &mut self,
        _program: &Program,
        i: &Instruction,
        a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
        _now: u64,
    ) -> Result<Option<CommandResult>> {
        //4284B0 case149 guards both numeric animation and dispatcher-local
        //pack selectors before even finishing the preceding object's queues.
        if i.opcode==0x95 && (self.state.dispatch_a2!=0 || self.state.native_flags.get(&25104).copied().unwrap_or(0)!=0) {
            return Ok(Some(CommandResult::Complete{next:i.next,outcome:Outcome::Advanced}));
        }
        if i.opcode == 0x95 && matches!(a[3], 5..=7) {
            let object = u16::try_from(a[0]).context("Circus packed prelude object")?;
            let seed = self.state.native_live.sprite_seed(object)?;
            if !host.sprite(&sprite_ops::SpriteRequest {
                commands: vec![sprite_ops::SpriteCommand {
                    seed,
                    operations: vec![crysirror_scene::layer::queued::SequenceOperation::Finish {
                        cancel: false,
                        lanes: None,
                    }],
                }],
            })? {
                return Ok(Some(CommandResult::Retry));
            }
            self.sync_sprite_states(host)?;
        }
        match i.opcode {
            0x3c if (33..=36).contains(&a[0]) => {
                if !host.packed(&PackedRequest::Load {
                    slot: a[0] as u8,
                    name: text.to_owned(),
                })? {
                    return Ok(Some(CommandResult::Retry));
                }
                self.state.image_names.insert(a[0] as u8, text.to_owned());
                self.state.native_live.packed_resource_loaded(a[0] as u8, text)?;
            }
            0x8f if a[3] & 0x30 != 0 => {
                if self
                    .state
                    .image_names
                    .get(&33)
                    .is_some_and(|name| !name.is_empty())
                {
                    let request = PackedRequest::Play {
                        slot: 33,
                        seed: self.state.native_live.sprite_seed(63)?,
                        position: [a[0], a[1]],
                        hold_ticks: a[2],
                        looping: a[3] & 15 != 0,
                    };
                    if !host.packed(&request)? {
                        return Ok(Some(CommandResult::Retry));
                    }
                    self.state.native_live.reset_object(63)?;
                    self.packed_publication_receipt(host, 63, 33, a[2], a[3] & 15 != 0)?;
                    self.sync_sprite_states(host)?;
                    let name = format!("_p,{}", self.state.image_names[&33]);
                    self.state.image_names.insert(25, name.clone());
                    self.state.image_names.insert(33, String::new());
                    self.state.native_live.image_name(25, &name)?;
                    self.state.native_live.image_name(33, "")?;
                }
            }
            0x95 if a[3] == 5 => {
                self.state.packed.object = u16::try_from(a[0]).ok().filter(|object| *object < 150);
                self.state.packed.slot_offset = a[1];
            }
            0x95 if a[3] == 7 => self.state.packed.looping = a[0] != 0,
            0x95 if matches!(a[3], 4 | 6) => {
                let (object, slot, looping) = if a[3] == 4 {
                    (Some(66), 33, false)
                } else {
                    (
                        self.state.packed.object,
                        33i16
                            .checked_add(self.state.packed.slot_offset)
                            .context("Circus packed resource selector overflow")?,
                        self.state.packed.looping,
                    )
                };
                if !self
                    .state
                    .image_names
                    .get(&u8::try_from(slot)?)
                    .is_some_and(|name| !name.is_empty())
                {
                    return Ok(Some(CommandResult::Complete {
                        next: i.next,
                        outcome: Outcome::Advanced,
                    }));
                }
                let object =
                    object.context("Circus packed animation missing native object selector")?;
                let request = PackedRequest::Play {
                    slot: u8::try_from(slot).context("Circus packed resource selector")?,
                    seed: self.state.native_live.sprite_seed(object)?,
                    position: [a[0], a[1]],
                    hold_ticks: a[2],
                    looping,
                };
                if !host.packed(&request)? {
                    return Ok(Some(CommandResult::Retry));
                }
                self.state.native_live.reset_object(object)?;
                self.packed_publication_receipt(host, object, u8::try_from(slot)?, a[2], looping)?;
                self.sync_sprite_states(host)?;
            }
            _ => return Ok(None),
        }
        self.state.native_live.sprite_opcode_completed(i.opcode);
        Ok(Some(CommandResult::Complete {
            next: i.next,
            outcome: Outcome::Advanced,
        }))
    }
}

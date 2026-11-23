//! DC3WY portraits: texture slots 2..5 bound to objects 10/20/30/40.
//!
//! 0x3A slots 2..5 store a v403 record (`?` -> '1'+var(5,8)); the
//! dispatcher tail publishes it through 41E8B0 (load, 436F60 bind; a
//! "bst\\" name gets the 4371B0(-1,0,w-2,h-1) source rect) and then sets the
//! 437130 draw offset (0,-12) for "BST\\KSK", (0,0) otherwise, and for
//! "BST\\MRK" moves x left by (width-1024)/2 (4370F0 + 437170).
//! 0x81 k (a3 = 0): 43AE10 finishes object 10(k+1)'s lanes, 401040 sets its
//! position and target, 437170 retains it. 0x81 with flag 0x10 (and Buf23574
//! clear): 4371F0 scale a0 %, 437280 pivot (a1, a2), 437210 retains the
//! scale. 0x3D k: "." -> 43AED0 completes object 10(k+1)'s queue; "-" marks
//! slots k+2 and k+6 for release at the tail; "--" restores the 0x1B copy.
//! The shared sequence actor owns the channels: position + offset on 0/1,
//! scale on 3/4, pivot on 12/13.
//! Not connected (fail visibly): 0x81 flags 0x20/0x40 and negative selectors,
//! the "--" rollback, slots 6..9 (objects 12..42, which copy their portrait
//! object's record at the tail) and 0x93 portrait transitions.
use super::*;
use crate::machine::sprite_ops::{SpriteCommand, SpriteRequest, linear};
use crysirror_scene::layer::queued::{DeltaSource, SequenceOperation};

impl Machine {
    ///43B6A0 through the shared per-object mask fade (slot 16 rule).
    fn portrait_mask(&mut self, host: &mut dyn NativeHost, object: u16, appear: bool, ticks: i32) -> Result<bool> {
        let request = crate::machine::effect_ops::EffectRequest::SpriteMask { seed: self.object_seed(object), mode: appear, ticks };
        if !host.effect_control(&request)? {
            return Ok(false);
        }
        self.state.sprites.entry(object).or_default().actor = true;
        Ok(true)
    }
    fn portrait_operations(&mut self, host: &mut dyn NativeHost, object: u16, operations: Vec<SequenceOperation>) -> Result<bool> {
        let request = SpriteRequest {
            commands: vec![SpriteCommand { seed: self.object_seed(object), operations }],
        };
        if !host.sprite(&request)? {
            return Ok(false);
        }
        self.state.sprites.entry(object).or_default().actor = true;
        Ok(true)
    }

    pub(super) fn execute_portrait(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
    ) -> Result<Option<Outcome>> {
        match instruction.opcode {
            0x81 if a[3] == 255 => {} //native explicit early return
            0x81 if a[3] == 0 => {
                let object = u16::try_from(10 * (i32::from(a[2]) + 1)).context("DC3WY portrait object")?;
                ensure!(object < 120, "DC3WY native sprite index");
                let actor = self.state.sprites.get(&object).is_some_and(|sprite| sprite.actor);
                if actor {
                    let offset = self.state.sprites[&object].offset;
                    let operations = vec![
                        SequenceOperation::Finish { cancel: false, lanes: None },
                        SequenceOperation::Set {
                            channels: vec![0, 1],
                            values: vec![
                                f32::from(a[0]) + f32::from(offset[0]),
                                f32::from(a[1]) + f32::from(offset[1]),
                            ],
                            retained: true,
                        },
                    ];
                    if !self.portrait_operations(host, object, operations)? {
                        return Ok(Some(Outcome::Pending));
                    }
                }
                let sprite = self.state.sprites.entry(object).or_default();
                sprite.position = [a[0], a[1]];
                sprite.retained_position = sprite.position;
            }
            //Flag 0x80: clear portrait k's pending declaration and slot name
            //record. Skipping (41D820): release the slot-16 mask and declare
            //"-" for k (as 0x3D "-"). Otherwise 43B6A0(mask 16, out, 12*a1).
            0x81 if (a[3] as u8) & 0x80 != 0 => {
                let k = (a[3] & 0xf) as u8;
                let object = 10 * (u16::from(k) + 1);
                if self.skip_for(instruction.offset) {
                    if self.state.image_names.contains_key(&16) && !self.publish_texture(host, 16, "-")? {
                        return Ok(Some(Outcome::Pending));
                    }
                    self.state.resources.pending.insert(k + 2, "-".into());
                    self.state.resources.pending.insert(k + 6, "-".into());
                } else {
                    self.state.resources.pending.remove(&(k + 2));
                    if !self.portrait_mask(host, object, false, 12 * i32::from(a[1]))? {
                        return Ok(Some(Outcome::Pending));
                    }
                }
            }
            //Flag 0x40: portrait k appears. Skipping releases the slot-16
            //mask; otherwise 43AE10 then a0 0/1: alpha 0 -> 1 over 20 and a
            //20 px slide from the left/right, 2: the fade only, 3: 43B6A0
            //(mask 16, in, 12*a1).
            0x81 if (a[3] as u8) & 0x40 != 0 => {
                let k = (a[3] & 0xf) as u8;
                let object = 10 * (u16::from(k) + 1);
                if self.skip_for(instruction.offset) {
                    if self.state.image_names.contains_key(&16) && !self.publish_texture(host, 16, "-")? {
                        return Ok(Some(Outcome::Pending));
                    }
                } else {
                    match a[0] {
                        0..=2 => {
                            let sprite = self.state.sprites.get(&object).cloned().unwrap_or_default();
                            let [x, y] = [0, 1].map(|axis| f32::from(sprite.position[axis]) + f32::from(sprite.offset[axis]));
                            let mut operations = vec![
                                SequenceOperation::Finish { cancel: false, lanes: None },
                                SequenceOperation::Set { channels: vec![2], values: vec![0.], retained: true },
                                SequenceOperation::Enqueue { lane: 3, bank: None, track: linear(vec![2], vec![1.], 20, vec![2], vec![2], DeltaSource::Retained, false, true) },
                            ];
                            if a[0] < 2 {
                                let from = if a[0] == 0 { x - 20. } else { x + 20. };
                                operations.push(SequenceOperation::Set { channels: vec![0, 1], values: vec![from, y], retained: true });
                                operations.push(SequenceOperation::Enqueue { lane: 0, bank: None, track: linear(vec![0, 1], vec![x, y], 20, vec![0, 1], vec![0, 1], DeltaSource::Retained, false, true) });
                            }
                            if !self.portrait_operations(host, object, operations)? {
                                return Ok(Some(Outcome::Pending));
                            }
                        }
                        3 => {
                            if !self.portrait_mask(host, object, true, 12 * i32::from(a[1]))? {
                                return Ok(Some(Outcome::Pending));
                            }
                        }
                        _ => {}
                    }
                }
            }
            //0x84 x,y,k,t: k&0x10 is a no-op; otherwise 43AE10 then 43B110
            //moves portrait k to (x, y) over 6t ticks (60 when t is 0).
            0x84 if (a[2] as u8) & 0x10 != 0 => {}
            0x84 => {
                let object = u16::try_from(10 * (i32::from(a[2] & 0xf) + 1)).context("DC3WY portrait object")?;
                let ticks = if a[3] != 0 { 6 * i32::from(a[3]) } else { 60 };
                let ticks = u32::try_from(ticks).ok().filter(|ticks| *ticks > 0).context("DC3WY 0x84 duration")?;
                let offset = self.state.sprites.get(&object).map_or([0, 0], |sprite| sprite.offset);
                let target = vec![f32::from(a[0]) + f32::from(offset[0]), f32::from(a[1]) + f32::from(offset[1])];
                let operations = vec![
                    SequenceOperation::Finish { cancel: false, lanes: None },
                    SequenceOperation::Enqueue { lane: 0, bank: None, track: linear(vec![0, 1], target, ticks, vec![0, 1], vec![0, 1], DeltaSource::Retained, false, true) },
                ];
                if !self.portrait_operations(host, object, operations)? {
                    return Ok(Some(Outcome::Pending));
                }
                let sprite = self.state.sprites.entry(object).or_default();
                sprite.position = [a[0], a[1]];
            }
            //Flag 0x20: 4377B0 clamps a0 to 0..100 as the draw priority.
            0x81 if a[3] & 0x20 != 0 => {
                let object = 10 * (u16::try_from(a[3] & 0xf)? + 1);
                let priority = a[0].clamp(0, 100);
                if !host.visual(&crate::machine::visual_ops::VisualRequest::Priority { object, priority })? {
                    return Ok(Some(Outcome::Pending));
                }
                self.state.sprites.entry(object).or_default().priority = Some(priority);
            }
            //Buf23574 (an active 0x93 camera) skips the 0x10 scale.
            0x81 if a[3] & 0x10 != 0 && self.state.camera => {}
            0x81 if a[3] & 0x10 != 0 => {
                let object = 10 * (u16::try_from(a[3] & 0xf)? + 1);
                let scale = f32::from(a[0]);
                let pivot = [f32::from(a[1]), f32::from(a[2])];
                if self.state.sprites.get(&object).is_some_and(|sprite| sprite.actor) {
                    let operations = vec![
                        SequenceOperation::Set { channels: vec![3, 4], values: vec![scale, scale], retained: true },
                        SequenceOperation::Set { channels: vec![12, 13], values: pivot.to_vec(), retained: true },
                    ];
                    if !self.portrait_operations(host, object, operations)? {
                        return Ok(Some(Outcome::Pending));
                    }
                }
                let sprite = self.state.sprites.entry(object).or_default();
                sprite.scale = Some(scale);
                sprite.pivot = pivot;
            }
            0x81 => {} //no 0x10/0x20/0x40 flag: native does nothing
            //0x94: Buf23569, the tint mode 419F70 applies at publish.
            0x94 => self.state.tint = i32::from(a[0]),
            0x3d => {
                ensure!((0..4).contains(&a[0]), "DC3WY portrait index");
                let k = a[0] as u8;
                if text == "--" {
                    bail!("DC3WY 0x3D rollback (0x1B copy) is not connected");
                }
                if text == "-" {
                    self.state.resources.pending.insert(k + 2, "-".into());
                    self.state.resources.pending.insert(k + 6, "-".into());
                } else {
                    let object = 10 * (u16::from(k) + 1);
                    if self.state.sprites.get(&object).is_some_and(|sprite| sprite.actor)
                        && !self.portrait_operations(host, object, vec![SequenceOperation::Finish { cancel: false, lanes: None }])?
                    {
                        return Ok(Some(Outcome::Pending));
                    }
                }
            }
            _ => return Ok(None),
        }
        self.advance(instruction.next, instruction.opcode).map(Some)
    }

    ///LABEL_972 after 41E8B0 bound a portrait slot 2..5.
    pub(super) fn settle_portrait(&mut self, host: &mut dyn NativeHost, slot: u8, object: u16, name: &str, logical: [u32; 2]) -> Result<bool> {
        let upper = name.to_ascii_uppercase();
        let offset = if upper.starts_with("BST\\KSK") { [0, -12] } else { [0, 0] };
        let sprite = self.state.sprites.entry(object).or_default();
        sprite.offset = offset;
        let mut position = sprite.position.map(i32::from);
        //D.C.III draws the face (slot 25 here) at its 0x6C origin.
        if object == 65 {
            if let Some(origin) = self.state.face_origin {
                position = origin.map(i32::from);
                self.state.sprites.entry(object).or_default().position = origin;
            }
        }
        //4372F0 reads the texture's logical extent. A CRM part is a 1024-wide
        //canvas natively (the part is drawn at its CRXG origin, see
        //images::load_profile_image_origin), so the shift is zero there.
        if upper.starts_with("BST\\MRK") && !name.contains(',') {
            position[0] -= (i32::try_from(logical[0])? - 1024) / 2;
        }
        let _ = slot;
        self.place_object(host, object, position)
    }
}

///419F70 (first part) for slots 2..9: with tint mode Buf23569 nonzero
///(0x94; 1 = '*', 2 = '@'), a name whose modifier differs has its first
///'*' turned into '@', else its first '@' into '*', else, when it has a
///container ',', the mode's character inserted before the ','.
pub(super) fn tint_rewrite(name: &str, mode: i32) -> String {
    if mode == 0 {
        return name.to_owned();
    }
    let current = if name.contains('@') { 2 } else if name.contains('*') { 1 } else { 0 };
    if current == mode {
        return name.to_owned();
    }
    if let Some(at) = name.find('*') {
        return format!("{}@{}", &name[..at], &name[at + 1..]);
    }
    if let Some(at) = name.find('@') {
        return format!("{}*{}", &name[..at], &name[at + 1..]);
    }
    match (name.find(','), mode) {
        (Some(comma), 1) => format!("{}*{}", &name[..comma], &name[comma..]),
        (Some(comma), 2) => format!("{}@{}", &name[..comma], &name[comma..]),
        _ => name.to_owned(),
    }
}

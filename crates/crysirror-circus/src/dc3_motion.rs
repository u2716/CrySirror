//! 4233A0 case 0x7F: background (renderer **(this+22228), object 0) motion.
//!
//! Native: x/y -> v400[2049..2050], selector v400[2051], argument v400[2052].
//! Selector 1 (and every selector while 4233A0's replay argument is set)
//! calls 401040 -> 4370F0 (current and target position) and 437170
//! (retained reference = current). Selector 0/0x63/other first flush the
//! object's lanes57..65 (43AE10 -> 439E60 until empty, then 4382E0), then
//! 43B110(x, y, ticks, 0, -1) queues one linear record per axis whose
//! velocity is `(target - this[11|12]) / ticks` against the retained target
//! and only when that velocity is nonzero. Ticks: 0 -> 120, 0x63 -> 12*arg,
//! otherwise 15*(arg+4). These selectors set v365 and yield the dispatcher;
//! process 42A160 maps last opcode 0x7F to the same one-frame continuation
//! as 0x0E (13456 = 1). The incremental sampling belongs to the shared
//! sequence actor (same native object class as DC4PH 437C40/43B110).
//!
//! 0x3A's non-held background declaration runs 43AE10 then 437F40 ->
//! 437C40 (memset of the object, position 0). Only channels 0/1 are mutated
//! by admitted DC3 producers, so the actor projection is finish + set 0.
use super::*;
///41A670/41AFE0 camera objects (this+22228 + 40k).
pub(super) const CAMERA: [u16; 5] = [0, 10, 20, 30, 40];
use crate::machine::sprite_ops::{CHANNELS, SpriteCommand, SpriteRequest, SpriteSeed, linear};
use crysirror_scene::layer::queued::{DeltaSource, SequenceOperation};

impl Machine {
    ///437C40 defaults in the shared channel order (x, y, alpha, scale x/y,
    ///rotation, ..., rate). The seed is read only when the actor is created.
    fn background_seed(&self) -> SpriteSeed {
        self.object_seed(0)
    }
    pub(super) fn object_seed(&self, object: u16) -> SpriteSeed {
        let sprite = self.state.sprites.get(&object).cloned().unwrap_or_default();
        let mut values = vec![0.; CHANNELS];
        values[0] = f32::from(sprite.position[0]) + f32::from(sprite.offset[0]);
        values[1] = f32::from(sprite.position[1]) + f32::from(sprite.offset[1]);
        for channel in [2, 6, 7, 10, 11, 16] {
            values[channel] = 1.;
        }
        values[3] = sprite.scale.unwrap_or(100.);
        values[4] = sprite.scale.unwrap_or(100.);
        values[12] = sprite.pivot[0];
        values[13] = sprite.pivot[1];
        SpriteSeed { object, retained: values.clone(), values }
    }

    pub(super) fn execute_motion(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>> {
        if instruction.opcode == 0x13 && a[0] == 254 {
            return self.layer_fence(instruction, host, now).map(Some);
        }
        //0x35: 43AE10 on all 120 objects (finish every queue).
        if instruction.opcode == 0x35 {
            let actors: Vec<u16> = self.state.sprites.iter().filter(|(object, sprite)| **object < 120 && sprite.actor).map(|(object, _)| *object).collect();
            if !actors.is_empty() {
                let commands = actors
                    .iter()
                    .map(|object| SpriteCommand { seed: self.object_seed(*object), operations: vec![SequenceOperation::Finish { cancel: false, lanes: None }] })
                    .collect();
                if !host.sprite(&SpriteRequest { commands })? {
                    return Ok(Some(Outcome::Pending));
                }
            }
            return self.advance(instruction.next, instruction.opcode).map(Some);
        }
        //0x9D: 4388F0 on all 120 objects, capturing queued motion while the
        //operand is 0. 0x9E: 4388F0(off, count) then 43BB50 replays the
        //captured records `count` times (no forever operand; 42A160 owns
        //that flag).
        if matches!(instruction.opcode, 0x9d | 0x9e) {
            let replay = instruction.opcode == 0x9e;
            let repeats = if replay { u16::try_from(a[0]).context("DC3WY 0x9E replay count")? } else { 0 };
            let commands = (0..120u16)
                .map(|object| {
                    let mut operations = vec![SequenceOperation::Capture { enabled: !replay && a[0] == 0, repeats, forever: false }];
                    if replay {
                        operations.push(SequenceOperation::Replay);
                    }
                    SpriteCommand { seed: self.object_seed(object), operations }
                })
                .collect();
            if !host.sprite(&SpriteRequest { commands })? {
                return Ok(Some(Outcome::Pending));
            }
            for object in 0..120u16 {
                self.state.sprites.entry(object).or_default().actor = true;
            }
            return self.advance(instruction.next, instruction.opcode).map(Some);
        }
        if instruction.opcode == 0x93 {
            return self.portrait_transition(instruction, a, host).map(Some);
        }
        if instruction.opcode != 0x7f {
            return Ok(None);
        }
        let target = [a[0], a[1]];
        let position = target.map(f32::from).to_vec();
        let ticks = match a[2] {
            1 => None,
            0 => Some(120),
            0x63 => Some(12 * i32::from(a[3])),
            _ => Some(15 * (i32::from(a[3]) + 4)),
        };
        let operations = match ticks {
            None => vec![SequenceOperation::Set { channels: vec![0, 1], values: position, retained: true }],
            Some(ticks) => {
                //A non-positive duration makes 43B110 divide by zero or
                //invert the direction; no admitted script produces one.
                let ticks = u32::try_from(ticks)
                    .ok()
                    .filter(|ticks| *ticks > 0)
                    .context("DC3WY background motion duration")?;
                vec![
                    SequenceOperation::Finish { cancel: false, lanes: None },
                    SequenceOperation::Enqueue {
                        lane: 0,
                        bank: None,
                        track: linear(
                            vec![0, 1],
                            position,
                            ticks,
                            vec![0, 1],
                            vec![0, 1],
                            DeltaSource::Retained,
                            false,
                            true,
                        ),
                    },
                ]
            }
        };
        let request = SpriteRequest {
            commands: vec![SpriteCommand { seed: self.background_seed(), operations }],
        };
        if !host.sprite(&request)? {
            return Ok(Some(Outcome::Pending));
        }
        let background = self.state.sprites.entry(0).or_default();
        background.actor = true;
        background.position = target;
        if ticks.is_none() {
            background.retained_position = target; //437170
            return self.advance(instruction.next, instruction.opcode).map(Some);
        }
        self.advance(instruction.next, instruction.opcode)?;
        Ok(Some(Outcome::WaitUntil(now.saturating_add(16))))
    }

    ///4233A0 case 0x13 selector 254 sets 1584 and yields. While 1584 is set,
    ///42A160 holds the continuation (v79) when any object below 60 that is
    ///not 15..17/25..27/35..37/45..47/52/53 reports 40D600 (pending
    ///motion); otherwise the next frame dispatches. The first execution
    ///records the yield; the retry observes the shared actors.
    fn layer_fence(
        &mut self,
        instruction: &Instruction,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Outcome> {
        if self.state.layer_fence != Some(instruction.offset) {
            self.state.layer_fence = Some(instruction.offset);
            return Ok(Outcome::WaitUntil(now.saturating_add(16)));
        }
        const EXCLUDED: [u16; 14] = [15, 16, 17, 25, 26, 27, 35, 36, 37, 45, 46, 47, 52, 53];
        let busy = host.sprite_states()?.into_iter().any(|(object, state)| {
            object < 60 && !EXCLUDED.contains(&object) && !state.complete
        });
        if busy {
            return Ok(Outcome::WaitUntil(now.saturating_add(16)));
        }
        self.state.layer_fence = None;
        self.advance(instruction.next, instruction.opcode)
    }

    ///43AE10 on object 0 when its shared actor exists.
    pub(super) fn finish_background_actor(&mut self, host: &mut dyn NativeHost) -> Result<bool> {
        if !self.state.sprites.get(&0).is_some_and(|sprite| sprite.actor) {
            return Ok(true);
        }
        host.sprite(&SpriteRequest {
            commands: vec![SpriteCommand {
                seed: self.background_seed(),
                operations: vec![SequenceOperation::Finish { cancel: false, lanes: None }],
            }],
        })
    }

    ///4233A0 case 0x93. 4377F0 reports an object transformed when its
    ///scale (+220) is not 100, its alpha (+24) is not 1, or flag +224 is
    ///set. Admitted DC3 producers only move x/y, so objects 0/10/20/30/40
    ///are untransformed and `0x93 x,100` (dispatch) changes nothing.
    ///Otherwise every object's queue is finished (43AE10 x120), the local
    ///portrait records v399 are cleared, operand 100 drops the background
    ///hold (Buf23573), Buf23574 is set, and v378..v381 keep the operands for
    ///a later 0x3A portrait transition (v382 = this+21740).
    fn portrait_transition(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
    ) -> Result<Outcome> {
        if !self.camera_transformed() && a[1] == 100 {
            return self.advance(instruction.next, instruction.opcode);
        }
        //43AE10 on all 120 objects.
        let actors: Vec<u16> = self.state.sprites.iter().filter(|(object, sprite)| **object < 120 && sprite.actor).map(|(object, _)| *object).collect();
        if !actors.is_empty() {
            let commands = actors
                .iter()
                .map(|object| SpriteCommand { seed: self.object_seed(*object), operations: vec![SequenceOperation::Finish { cancel: false, lanes: None }] })
                .collect();
            if !host.sprite(&SpriteRequest { commands })? {
                return Ok(Outcome::Pending);
            }
        }
        //Dispatcher tail: 41AFE0 (mode < 2 only reads object 0) then
        //41A670(mode, 255, -x, -y, ticks, this+21740, scale) when x/y are
        //not both -1. Only the immediate mode 0 has script producers.
        ensure!(a[0] == 0, "DC3WY 0x93 camera mode {} (43BA50 animation / 437230 scale) is not connected", a[0]);
        if a[2] != -1 || a[3] != -1 {
            let x = -i32::from(a[2]);
            let y = -i32::from(a[3]);
            if !self.camera_zoom(host, i32::from(a[1]), x, y)? {
                return Ok(Outcome::Pending);
            }
        }
        if a[1] == 100 {
            self.state.resources.hold_background = false; //Buf23573
        }
        self.state.camera = true; //Buf23574
        self.advance(instruction.next, instruction.opcode)
    }

    ///4377F0 over the camera objects: zoom percent != 100.
    pub(super) fn camera_transformed(&self) -> bool {
        CAMERA.iter().any(|object| self.state.sprites.get(object).and_then(|sprite| sprite.zoom).is_some_and(|(scale, _)| scale != 100.))
    }

    ///41A670 mode 0: zoom objects 0/10/20/30/40 about screen points. Object
    ///0 (size w,h at px,py) centers on (512-x, 288-y); the portrait groups
    ///follow at 80% (512-80x/100, 288-80y/100). The percent is raised to
    ///cover 1024x576 (102400/w, 57600/h) and x/y clamp to +-512/+-288; -1
    ///takes the background center.
    pub(super) fn camera_zoom(&mut self, host: &mut dyn NativeHost, scale: i32, x: i32, y: i32) -> Result<bool> {
        let Some((_, [w, h])) = host.image_dimensions(1)? else {
            bail!("DC3WY 41A670 camera without a bound background (4372F0 extent 0)");
        };
        let (w, h) = (i32::try_from(w)?, i32::try_from(h)?);
        ensure!(w > 0 && h > 0, "DC3WY camera background extent");
        let [px, py] = self.state.sprites.get(&0).map_or([0, 0], |sprite| sprite.position).map(i32::from);
        let x = if x == -1 { 512 - (w / 2 - px) } else { x }.clamp(-512, 512);
        let y = if y == -1 { 288 - (h / 2 - py) } else { y }.clamp(-288, 288);
        let wide = if scale * w / 100 < 1024 { 102400 / w } else { scale };
        let tall = if scale * h / 100 >= 576 { scale } else { 57600 / h };
        let scale = wide.max(tall) as f32;
        for object in CAMERA {
            let center = if object == 0 {
                [(512 - x) as f32, (288 - y) as f32]
            } else {
                [(512 + -80 * x / 100) as f32, (288 + -80 * y / 100) as f32]
            };
            if !host.visual(&crate::machine::visual_ops::VisualRequest::Zoom { object, scale, center })? {
                return Ok(false);
            }
            self.state.sprites.entry(object).or_default().zoom = (scale != 100.).then_some((scale, center));
        }
        Ok(true)
    }

    ///4370F0 + 437170 on object 0 through its shared actor.
    pub(super) fn place_background(&mut self, host: &mut dyn NativeHost, position: [i32; 2]) -> Result<bool> {
        self.place_object(host, 0, position)
    }
    ///4370F0 (+ 437170) on `object` through its shared actor.
    pub(super) fn place_object(&mut self, host: &mut dyn NativeHost, object: u16, position: [i32; 2]) -> Result<bool> {
        let position = position.map(|value| i16::try_from(value)).map(|value| value.ok());
        let [Some(x), Some(y)] = position else { bail!("DC3WY background position range") };
        let offset = self.state.sprites.get(&object).map_or([0, 0], |sprite| sprite.offset);
        let request = SpriteRequest {
            commands: vec![SpriteCommand {
                seed: self.object_seed(object),
                operations: vec![SequenceOperation::Set {
                    channels: vec![0, 1],
                    values: vec![f32::from(x) + f32::from(offset[0]), f32::from(y) + f32::from(offset[1])],
                    retained: true,
                }],
            }],
        };
        if !host.sprite(&request)? {
            return Ok(false);
        }
        let background = self.state.sprites.entry(object).or_default();
        background.actor = true;
        background.position = [x, y];
        background.retained_position = [x, y];
        Ok(true)
    }

    ///0x3A non-held background: 43AE10 + 437F40 on the live actor.
    pub(super) fn reset_background_actor(&mut self, host: &mut dyn NativeHost) -> Result<bool> {
        if !self.state.sprites.get(&0).is_some_and(|sprite| sprite.actor) {
            return Ok(true);
        }
        let request = SpriteRequest {
            commands: vec![SpriteCommand {
                seed: self.background_seed(),
                operations: vec![
                    SequenceOperation::Finish { cancel: false, lanes: None },
                    SequenceOperation::Set { channels: vec![0, 1], values: vec![0., 0.], retained: true },
                ],
            }],
        };
        host.sprite(&request)
    }
}

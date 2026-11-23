//! D.C.III (DC3.exe, 0x8a71) opcode 0x0E effects that enter process state 14.
//!
//! 424FE0 case 14 stores the selector in C701E4 and the operand in C43C34,
//! then 42B300 runs the selected state each time its uElapse timer expires
//! (34 ms, scaled by the operand). This module keeps that policy: selector
//! rewrites, step tables, the C1CD3C progress, 4073F0's xorshift128 tile
//! setup, 408C40's lookup tables and 407F20's threshold. Frames are composed
//! by the bridge from the frozen presented screen (4E89D0) and the live next
//! screen (486DE0) using shared Canvas, Layer and image-worker services.
//!
//! Selectors and states (C701E4):
//!  * 0 (421D30): crossfade presented -> next, step 482AC0[speed] or
//!    482AD0[n] (n >= 8: 1 per n-7 ticks), 64 while skipping.
//!  * 1: next from black; 2: presented to black, then the full scene reset
//!    (401770(11), 421180, 422C40, 41EB80, 41EBD0, 4016E0).
//!  * 3: n white flashes (alpha -1 = palette 7) on alternate 34 ms ticks.
//!  * 4/6: presented screen shaken +-20 px on x/y over palette 0, 9 ticks
//!    (3 at speed 3 or while skipping).
//!  * 16 (407D20): 32x18 tile break with SYSTEM\EFF cracks, then the reset;
//!    palette-0 fade (2) when effects are off (7E0C7C = var(99,10)).
//!  * 17 (408D40): 408C40 lookup wipe, 1 = horizontal, 2 = diagonal; 0 while
//!    skipping; operands >= 3 leave 7E9314 = 0 and cut at once.
//!  * 21 (4083E0): rule wipe through the slot-13 mask (DC3WY slot 16), step
//!    47AA78[n] (24 from 6) or 47AA68[speed]; crossfade (0) while skipping,
//!    with effects off or without a mask. The mask is released afterwards.
//!  * 0..2, 16, 17, 20, 21 first run 42EFF0 (stop voice and sound effects)
//!    unless var(9,19).
//!  * Other selectors that reach 42B300 (8, 12, 15, 18, 19, 22.., not the
//!    interpreter-level ones) return at once.
//! Selector 20 (mask wipe to black) is not used by the shipped scripts and
//! fails visibly.
use super::*;
use crate::machine::effect_ops::{EffectRequest, OriginalScreen};
use crysirror_image::tile_scatter::TileScatter;

///482AC0 followed by 482AD0 (482AD0[n] = this[4 + n]; 482ACC = this[3]).
const STEP_SPEED: [i32; 16] = [4, 8, 16, 32, 32, 32, 16, 8, 4, 2, 1, 1, 7, 0, 1, 5];
///47AA68 followed by 47AA78 (47AA74 = this[3], 47AA8C = this[9]); the
///bytes after it are the native table neighbours read by larger indices.
const RULE_SPEED: [i32; 16] = [2, 4, 8, 16, 2, 4, 8, 12, 16, 24, 0, 50, 40, 30, 20, 10];
///Engine image slot for "SYSTEM\EFF?" (405E20 strips the trailing '?').
pub(crate) const CRACK_SLOT: u8 = 36;
///DC3WY slot of the original slot-13 mask (486E1C).
const MASK_SLOT: u8 = 16;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OriginalEffect {
    pc: u32,
    selector: i16,
    operand: i16,
    ///C1CD3C.
    progress: i32,
    due: u64,
    ///uElapse of the previous call.
    interval: u64,
    ///Step and tick count chosen by 421D30's entry.
    increment: i32,
    repeat: i32,
    begun: bool,
    ///Frames not yet accepted by the bridge, followed by what the call does.
    frames: Vec<OriginalScreen>,
    after: Option<After>,
    ///4073F0 field.
    scatter: Option<TileScatter>,
    ///408C40: table, 7E9314, row stride 7E931C, flag 7E9318.
    lookup: Option<(Vec<u8>, i32, i32, bool)>,
    ///407E30/407F20: 7E6888 and 7E6884.
    threshold: i32,
    rule_step: i32,
    reset_done: bool,
    ///407A80 reported that every tile has finished.
    finished: bool,
    released: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum After {
    Wait(u64),
    ///Full scene reset, then wait.
    ResetWait(u64),
    ///Return from 42B300 / the synchronous loop.
    Done,
    ///Scene reset, then return.
    ResetDone,
}

impl Machine {
    ///Selectors that read the presented screen: the bridge captures it at the
    ///start of the frame, before that frame's script changes.
    fn original_reads_presented(selector: i16) -> bool {
        matches!(selector, 0 | 2 | 3 | 4 | 6 | 16 | 17 | 20 | 21)
    }
    ///The next linearly reachable 0x0E from the current PC (before a click
    ///wait) reads the presented screen.
    pub fn original_effect_ahead(&self, program: &Program) -> bool {
        if !self.state.original || self.state.original_effect.is_some() {
            return false;
        }
        let mut pc = self.state.pc;
        for _ in 0..64 {
            let Ok(instruction) = program.instruction(pc) else { return false };
            match (program.original_opcode(instruction), &instruction.arguments) {
                (Some(0x0e), Arguments::Bytes([selector, _])) => {
                    if Self::original_reads_presented(i16::from(*selector)) {
                        return true;
                    }
                }
                //0x07 text click wait, 0x00/0x0f yields and ends.
                (Some(0x00 | 0x07 | 0x0f), _) | (None, _) => return false,
                _ => {}
            }
            pc = instruction.next;
        }
        false
    }

    fn original_skipping(&self) -> bool {
        self.native_skipping()
    }
    fn original_speed(&self) -> Result<usize> {
        let speed = self.value(99, 12)?;
        usize::try_from(speed)
            .ok()
            .filter(|speed| *speed < 16)
            .with_context(|| format!("D.C.III effect speed 7E0C84={speed} outside its native tables"))
    }
    fn original_palette(&self, index: u8) -> [u8; 3] {
        self.state.palette.get(&index).copied().map_or([0; 3], logical_color)
    }

    pub(super) fn execute_original_effects(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>> {
        if !self.state.original || instruction.opcode != 0x0e {
            return Ok(None);
        }
        //424FE0 case 14 selectors handled by the interpreter itself.
        if matches!(a[0], 5 | 7 | 9 | 10 | 11 | 13 | 14 | 30 | 31 | 32 | 51 | 52) {
            return Ok(None);
        }
        ensure!(a[0] != 20, "D.C.III 0x0E selector 20 (4082F0 mask wipe to black) is not connected");
        let mut e = match self.state.original_effect.take() {
            Some(e) => {
                ensure!(e.pc == instruction.offset, "D.C.III 0x0E effect owner");
                e
            }
            None => OriginalEffect { pc: instruction.offset, selector: a[0], operand: a[1], due: now, ..Default::default() },
        };
        let outcome = self.run_original_effect(&mut e, host, now);
        match outcome {
            Ok(Some(outcome)) => {
                self.state.original_effect = Some(e);
                Ok(Some(outcome))
            }
            Ok(None) => self.advance(instruction.next, instruction.opcode).map(Some),
            Err(error) => Err(error),
        }
    }

    ///Some(outcome) keeps the effect; None returns to the interpreter.
    fn run_original_effect(
        &mut self,
        e: &mut OriginalEffect,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>> {
        if !e.begun {
            if !self.begin_original_effect(e, host)? {
                return Ok(Some(Outcome::Pending));
            }
            e.begun = true;
        }
        loop {
            if e.after.is_none() {
                if now < e.due {
                    return Ok(Some(Outcome::WaitUntil(e.due)));
                }
                let after = self.original_call(e)?;
                e.after = Some(after);
            }
            while let Some(frame) = e.frames.first() {
                if !host.effect_control(&EffectRequest::OriginalScreen(frame.clone()))? {
                    return Ok(Some(Outcome::Pending));
                }
                e.frames.remove(0);
            }
            let after = e.after.unwrap();
            if matches!(after, After::ResetDone | After::Done) && !e.released {
                //407CD0 frees SYSTEM\EFF; 408270 releases the slot-13 mask
                //(486E1C = -1). Both precede the scene reset.
                let release = match e.selector {
                    16 => Some(CRACK_SLOT),
                    21 => Some(MASK_SLOT),
                    _ => None,
                };
                if let Some(slot) = release {
                    if !self.publish_texture(host, slot, "-")? {
                        return Ok(Some(Outcome::Pending));
                    }
                }
                e.released = true;
            }
            if matches!(after, After::ResetWait(_) | After::ResetDone) && !e.reset_done {
                if !self.original_reset(e, host)? {
                    return Ok(Some(Outcome::Pending));
                }
                e.reset_done = true;
            }
            match after {
                After::Wait(ms) | After::ResetWait(ms) => {
                    e.after = None;
                    e.reset_done = false;
                    e.interval = ms;
                    e.due = now.saturating_add(ms);
                    if ms > 0 {
                        return Ok(Some(Outcome::WaitUntil(e.due)));
                    }
                }
                After::Done | After::ResetDone => return Ok(None),
            }
        }
    }

    ///424FE0 case 14: audio stop and selector rewrites before state 14.
    fn begin_original_effect(&mut self, e: &mut OriginalEffect, host: &mut dyn NativeHost) -> Result<bool> {
        let stops = matches!(e.selector, 0..=2 | 16 | 17 | 20 | 21);
        if stops && self.value(9, 19)? == 0 {
            //42EFF0: 42DA60(3) and 42DA60(4..9): the voice and the sound
            //effect buffers.
            for kind in [2, 1] {
                if !host.emit(&Event::ReleaseAudioGroup { kind, forced: true, steps: 0 })? {
                    return Ok(false);
                }
            }
        }
        let skipping = self.original_skipping();
        let effects = self.value(99, 10)? != 0;
        match e.selector {
            16 if !effects => e.selector = 2,
            17 if skipping => e.selector = 0,
            17 => e.lookup = Some(lookup_table(e.operand)?),
            21 if skipping || !effects || !self.state.image_names.contains_key(&MASK_SLOT) => e.selector = 0,
            21 => {
                e.rule_step = match e.operand {
                    0 if skipping => RULE_SPEED[3],
                    0 => RULE_SPEED[self.original_speed()?],
                    n if n >= 6 => RULE_SPEED[9],
                    n => RULE_SPEED[4 + usize::try_from(n).context("D.C.III rule wipe operand")?],
                };
                ensure!(e.rule_step > 0, "D.C.III 407E30 zero rule step");
            }
            _ => {}
        }
        //4073F0: 405E20("SYSTEM\\EFF?") strips its trailing flag character.
        if e.selector == 16 && !self.publish_texture(host, CRACK_SLOT, "SYSTEM\\EFF")? {
            return Ok(false);
        }
        if e.selector == 17 {
            ensure!(
                e.lookup.as_ref().is_some_and(|lookup| lookup.3),
                "D.C.III 408C40 uniform table (operand 0) reads stale native bytes"
            );
        }
        if Self::original_reads_presented(e.selector) {
            e.frames.push(OriginalScreen::Hold);
        }
        Ok(true)
    }

    ///401770(11), 421180, 422C40, 41EB80, 41EBD0 and 4016E0: every plane,
    ///portrait, face and display record is cleared (the bridge's unprotected
    ///source clear).
    fn original_reset(&mut self, _e: &OriginalEffect, host: &mut dyn NativeHost) -> Result<bool> {
        if !host.effect_control(&EffectRequest::ClearFrame { protected: false })? {
            return Ok(false);
        }
        host.emit(&Event::ClearUi)
    }

    ///One call of the state-14 function (one timer expiry).
    fn original_call(&mut self, e: &mut OriginalEffect) -> Result<After> {
        let skipping = self.original_skipping();
        let n = i32::from(e.operand);
        let black = self.original_palette(0);
        let fade = |alpha: i32| -> u8 { (alpha.clamp(0, 127) * 255 / 127) as u8 };
        //42B300 cases 1/2: the per-call step and uElapse.
        let step = |m: &Self| -> Result<(i32, u64)> {
            Ok(if n == 0 || skipping {
                if skipping { (127, 0) } else { (STEP_SPEED[m.original_speed()?], 34) }
            } else if n >= 8 {
                (1, 34 * (n as u64 - 7))
            } else {
                (STEP_SPEED[4 + n as usize], 34)
            })
        };
        Ok(match e.selector {
            0 => {
                if e.progress == 0 && e.increment == 0 {
                    if skipping {
                        e.frames.push(OriginalScreen::Clear);
                        return Ok(After::Done);
                    }
                    (e.increment, e.repeat) = if n == 0 {
                        (STEP_SPEED[self.original_speed()?], 1)
                    } else if n >= 8 {
                        (1, n - 7)
                    } else {
                        (STEP_SPEED[4 + n as usize], 1)
                    };
                }
                let (increment, repeat) = if skipping { (64, 1) } else { (e.increment, e.repeat) };
                e.progress += increment;
                if e.progress >= 127 {
                    e.frames.push(OriginalScreen::Clear);
                    return Ok(After::Done);
                }
                e.frames.push(OriginalScreen::Frozen { opacity: fade(127 - e.progress) });
                After::Wait(34 * repeat as u64)
            }
            1 | 2 => {
                if e.progress >= 128 {
                    return Ok(After::Done);
                }
                if e.progress == 127 {
                    e.progress = 128;
                    e.frames.push(OriginalScreen::Clear);
                    return Ok(if e.selector == 1 { After::Done } else { After::ResetWait(e.interval) });
                }
                let (increment, interval) = step(self)?;
                e.progress = (e.progress + increment).min(127);
                let opacity = if e.selector == 1 { fade(127 - e.progress) } else { fade(e.progress) };
                e.frames.push(OriginalScreen::Solid { color: black, opacity });
                After::Wait(interval)
            }
            3 => {
                if e.progress >= 2 * n {
                    e.frames.push(OriginalScreen::Clear);
                    return Ok(After::Done);
                }
                e.progress += 1;
                e.frames.push(if e.progress & 1 != 0 {
                    OriginalScreen::Solid { color: self.original_palette(7), opacity: 255 }
                } else {
                    OriginalScreen::Frozen { opacity: 255 }
                });
                After::Wait(34)
            }
            4 | 6 => {
                if e.progress == 9 {
                    e.frames.push(OriginalScreen::Clear);
                    return Ok(After::Done);
                }
                let fast = self.original_speed()? == 3 || skipping;
                e.progress = if fast && e.progress == 3 { 9 } else { (e.progress + 1).min(9) };
                let delta = match e.progress % 3 {
                    1 => -20,
                    2 => 20,
                    _ => 0,
                };
                let offset = if e.selector == 4 { [delta, 0] } else { [0, delta] };
                e.frames.push(OriginalScreen::Shifted { offset, background: black });
                After::Wait(34)
            }
            16 => {
                if e.finished {
                    //407CD0 and the reset after the last 34 ms frame.
                    e.frames.push(OriginalScreen::Clear);
                    return Ok(After::ResetDone);
                }
                if e.scatter.is_none() {
                    e.scatter = Some(self.original_tile_field(n)?);
                    e.progress = 1;
                }
                let scatter = e.scatter.as_mut().unwrap();
                let done = scatter.step();
                let mut cracks = Vec::new();
                let mut tiles = Vec::new();
                for (index, tile) in scatter.tiles.iter().enumerate() {
                    if tile.life <= 0 {
                        continue;
                    }
                    let origin = [(index as i32 % 32) * 32, (index as i32 / 32) * 32];
                    if tile.delay >= 0 {
                        cracks.push((origin, tile.delay * 32));
                    }
                    tiles.push((origin, scatter.pixel(tile), tile.alpha.clamp(0, 16) as u8));
                }
                e.frames.push(OriginalScreen::Tiles { overlay: CRACK_SLOT, cracks, tiles, fill: black });
                e.finished = done;
                After::Wait(34)
            }
            17 => {
                let (table, limit, stride, flag) = e.lookup.clone().context("D.C.III 408C40 table")?;
                if limit <= 0 {
                    e.frames.push(OriginalScreen::Clear);
                    return Ok(After::Done);
                }
                let speed = self.original_speed()?;
                e.progress += if speed == 3 || skipping {
                    4
                } else if speed == 2 {
                    3
                } else if speed == 1 {
                    2
                } else {
                    1
                };
                let limit = (limit - 4 * e.progress).max(0);
                e.lookup = Some((table.clone(), limit, stride, flag));
                e.frames.push(OriginalScreen::Lookup { table, offset: i64::from(limit), step: [1, stride] });
                //At 7E9314 = 0 the next call takes LABEL_25.
                After::Wait(34)
            }
            21 => {
                if e.threshold >= 510 {
                    e.frames.push(OriginalScreen::Clear);
                    return Ok(After::Done);
                }
                let weights = (0..256)
                    .map(|mask: i32| {
                        let value = mask - e.threshold + 255;
                        if value >= 255 {
                            256
                        } else if value <= 0 {
                            0
                        } else {
                            value as u16 + 1
                        }
                    })
                    .collect();
                e.frames.push(OriginalScreen::Rule { slot: MASK_SLOT, weights });
                e.threshold += e.rule_step;
                After::Wait(34)
            }
            _ => After::Done,
        })
    }

    ///4073F0: the shared base loop, then the selector-specific delays and
    ///velocities, consuming 407190 (xorshift128) in the original order.
    fn original_tile_field(&mut self, kind: i32) -> Result<TileScatter> {
        let rng = &mut self.state.original_rng;
        let mut rand = || rng.next_i31();
        let mut s = TileScatter::new(32, 18, 32, [1024, 576])?;
        s.acceleration = 1024;
        s.life_step = 1;
        s.time_step = (1 << 13) / 100;
        for t in &mut s.tiles {
            let mut vx = 32 - (rand() & 0x1f);
            let vy = (rand() & 0x1f) + 32;
            if rand() & 1 == 0 {
                vx = (rand() & 0x1f) - 32;
            }
            t.life = rand() % 32 + 16;
            t.delay = 0;
            t.velocity = [vx << 10, vy << 10];
            t.alpha = 16;
        }
        let place = |index: usize| ((index / 32) as i32, (index % 32) as i32);
        match kind {
            0 => {
                s.acceleration = 0;
                for t in &mut s.tiles {
                    t.delay = 0;
                    let vx = 48 - (rand() & 0x1f);
                    let vy = 48 - (rand() & 0x1f);
                    t.velocity = [vx << 10, vy << 10];
                }
            }
            1 => {
                for (index, t) in s.tiles.iter_mut().enumerate() {
                    let (row, column) = place(index);
                    t.delay = rand() % 2 - row / 5 + column - 32;
                    let vx = 32 - (rand() & 0x1f);
                    let vy = 32 - (rand() & 0x30);
                    t.velocity = [vx << 10, vy << 10];
                }
            }
            2 => {
                s.acceleration = 0;
                for (index, t) in s.tiles.iter_mut().enumerate() {
                    let (row, _) = place(index);
                    t.delay = rand() % 2 + row - 18;
                    let vx = 4 - (rand() & 8);
                    let vy = (rand() & 0x1f) + 32;
                    t.velocity = [vx << 10, vy << 10];
                }
            }
            3 => {
                s.acceleration = -1024;
                for (index, t) in s.tiles.iter_mut().enumerate() {
                    let (row, column) = place(index);
                    t.delay = rand() % 2 - row / 5 - column;
                    let vx = (rand() & 0x1f) - 32;
                    let vy = 32 - (rand() & 0x30);
                    t.velocity = [vx << 10, vy << 10];
                }
            }
            4 => {
                s.acceleration = 0;
                for t in &mut s.tiles {
                    t.delay = 0;
                    let vx = 32 - (rand() & 0x3f);
                    let vy = 32 - (rand() & 0x30);
                    t.velocity = [vx << 10, vy << 10];
                }
            }
            5 => {
                s.acceleration = 0;
                for (index, t) in s.tiles.iter_mut().enumerate() {
                    let (row, column) = place(index);
                    t.delay = -2 * row - column;
                    let vx = 2 - (rand() & 8);
                    let vy = (rand() & 8) - 2;
                    t.velocity = [vx << 10, vy << 10];
                }
            }
            6 => {
                s.acceleration = 0;
                for (index, t) in s.tiles.iter_mut().enumerate() {
                    let (row, column) = place(index);
                    t.delay = -((9 - row).abs() + (16 - column).abs());
                    let vx = 4 - (rand() & 8);
                    let vy = 4 - (rand() & 8);
                    t.velocity = [vx << 10, vy << 10];
                }
            }
            9 => {
                s.acceleration = 0;
                s.time_step = (4 << 13) / 100;
                for t in &mut s.tiles {
                    t.delay = 0;
                    let vx = rand() % 16 - 8;
                    let vy = -32 - rand() % 32;
                    t.velocity = [vx << 10, vy << 10];
                }
            }
            10 => {
                for t in &mut s.tiles {
                    t.delay = -(rand() % 20);
                }
            }
            //4073F0's switch has no other case: the base loop stands.
            _ => {}
        }
        Ok(s)
    }
}

///408C40: the 2688-byte table, 7E9314, 7E931C and 7E9318.
fn lookup_table(operand: i16) -> Result<(Vec<u8>, i32, i32, bool)> {
    let ramp = |start: i32, shift: i32| -> Vec<u8> {
        (0..2688)
            .map(|i: i32| {
                if i < start {
                    255
                } else if i < start + 64 {
                    (-4 - 4 * (i - shift)) as u8
                } else {
                    0
                }
            })
            .collect()
    };
    Ok(match operand {
        0 => (vec![0; 2688], 255, 0, false),
        1 => (ramp(1024, 0), 1024, 0, true),
        2 => (ramp(1600, 64), 1600, 1, true),
        _ => (Vec::new(), 0, 0, true),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lookup_tables_follow_408c40() {
        let (table, start, stride, flag) = lookup_table(1).unwrap();
        assert_eq!((start, stride, flag), (1024, 0, true));
        assert_eq!((table[1023], table[1024], table[1025], table[1087], table[1088]), (255, 252, 248, 0, 0));
        let (table, start, stride, _) = lookup_table(2).unwrap();
        assert_eq!((start, stride), (1600, 1));
        assert_eq!((table[1599], table[1600], table[1663], table[1664]), (255, 252, 0, 0));
    }
}

//! DC2 native0x2b (41C530 case43 -> 41BCE0). Natives1..4 are the portraits
//! 41A950 draws with poses0..3; native5 is 41AB60's image drawn with pose4.
//! A load extends the 41BA90 dirty rectangle; unless the next instruction
//! batches another portrait change (0x02/0x2b/0x2c/0x65), 41B920 presents
//! it: 424880 for a pending native0 composition, then 420A20 "bup_fade",
//! a /16 feedback blend (40B300) of the recomposed scene. Shared Image owns
//! composition and blending; the VM keeps names, poses and timing.
use super::*;
use crate::dc2_bitmap::{Draw, Owner, Request};
use crate::machine::{
    Event,
    resource_ops::{ResourceReply, ResourceRequest},
};
///Host image slots of natives1..5 (native0 is host1, UI natives keep 6..8).
pub(crate) const fn host_slot(native: u8) -> u8 {
    20 + native
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct PortraitState {
    ///CEBA00 with B0F0A0/B0F0A4/CE3CD0/CE3CCC.
    pending: bool,
    dirty: [i32; 4],
    transition: Option<Transition>,
    ///Process101 (41C530 case101 selector0..4 -> 422E60 -> 420C10).
    #[serde(default)]
    motion: Option<Motion>,
    ///B0A69C: 0x2c expression names of portraits 0..3 (as rewritten by
    ///41BFE0), and CEBB44: whether that expression is stamped now.
    #[serde(default)]
    expressions: [Option<String>; 4],
    #[serde(default)]
    stamped: [bool; 4],
    ///PC whose own 41BCE0/41BFE0 body completed while its 41BC80 tail
    ///is still waiting for a re-stamp.
    #[serde(default)]
    tail_pc: Option<u32>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Motion {
    pc: u32,
    ///Shown x/y were stepped; the 4258C0(255) redraw is still pending.
    redraw: bool,
    deadline: Option<u64>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Transition {
    pc: u32,
    ///424880 over native0 (AC7050 branch) finished.
    native0_done: bool,
    native0: Option<crate::dc2_bitmap::Transition>,
    prepared: bool,
    ///B0CE30 counter, 0..=5.
    counter: u8,
    deadline: Option<u64>,
    mixing: Option<u8>,
}
impl Machine {
    fn native_name(&self, native: u8) -> Option<&str> {
        match native {
            1..=4 => self.state.primary_names[usize::from(native)].as_deref(),
            5 => self.state.native5_name.as_deref(),
            _ => None,
        }
    }
    fn set_native_name(&mut self, native: u8, name: Option<String>) {
        match native {
            1..=4 => self.state.primary_names[usize::from(native)] = name,
            _ => self.state.native5_name = name,
        }
    }
    fn unload_native(&mut self, native: u8, host: &mut dyn NativeHost) -> Result<bool> {
        if self.native_name(native).is_some() {
            match host.resource(&ResourceRequest::Unload { slot: host_slot(native) })? {
                None => return Ok(false),
                Some(ResourceReply::Done) => {}
                Some(_) => bail!("DC2 portrait release foreign receipt"),
            }
            self.set_native_name(native, None);
        }
        Ok(true)
    }
    ///41A950/41AB60 geometry with the shown pose. `size` is the loaded
    ///(already 41BCE0-scaled) extent.
    fn portrait_rect(&self, index: usize, size: [u32; 2]) -> Result<[i32; 4]> {
        let pose = &self.state.poses[index];
        let [x, y, percent] = pose.shown.map(i32::from);
        let (w, h) = (i32::try_from(size[0])?, i32::try_from(size[1])?);
        if index == 4 {
            ensure!(percent == 100, "DC2 scaled native5 (41AB60) is not connected");
            return Ok([x, y, w, h]);
        }
        Ok(scaled_rect([x, y], pose.center.map(i32::from), percent, [w, h]))
    }
    ///41BA90(index, loaded): extend the dirty rectangle with the new rect
    ///(declared pose) or, for a0 handle, the previously drawn one.
    fn extend_dirty(&mut self, index: usize, size: Option<[u32; 2]>) -> Result<()> {
        let rect = match size {
            Some(size) => {
                let pose = &self.state.poses[index];
                let (w, h) = (i32::try_from(size[0])?, i32::try_from(size[1])?);
                scaled_rect(
                    pose.position.map(i32::from),
                    pose.center.map(i32::from),
                    i32::from(pose.percent),
                    [w, h],
                )
            }
            None => self.state.poses[index].drawn,
        };
        let d = &mut self.state.portrait.dirty;
        d[0] = d[0].min(rect[0]);
        d[1] = d[1].min(rect[1]);
        d[2] = d[2].max(rect[0] + rect[2]);
        d[3] = d[3].max(rect[1] + rect[3]);
        Ok(())
    }
    ///4258C0 layers 4/8 as display records over native0.
    pub(super) fn portrait_draws(&mut self, host: &mut dyn NativeHost) -> Result<Vec<Draw>> {
        let mut draws = Vec::new();
        for native in 1..=5u8 {
            if self.native_name(native).is_none() {
                continue;
            }
            let index = usize::from(native - 1);
            let (_, logical) = host
                .image_dimensions(host_slot(native))?
                .context("DC2 loaded portrait extent")?;
            let rect = self.portrait_rect(index, logical)?;
            self.state.poses[index].drawn = rect;
            draws.push(Draw::Image {
                slot: host_slot(native),
                source: [0, 0, rect[2], rect[3]],
                destination: [rect[0], rect[1]],
            });
        }
        Ok(draws)
    }
    ///420C10 on each 10 ms process tick: step the shown x/y of pose CEB9F0
    ///toward (B0CE2C, B0CE30) by 8/16/32/64 (52B3C8 speed, 64 under 41A290),
    ///clamp, then redraw; the tick that finds it in place ends process101.
    ///4239B0 has no input branch for process101.
    pub(super) fn execute_motion(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Option<Outcome>> {
        if !(instruction.opcode == 0x65 && (0..5).contains(&a[3])) {
            return Ok(None);
        }
        let index = a[3] as usize;
        let mut motion = match self.state.portrait.motion.take() {
            Some(motion) => {
                ensure!(motion.pc == instruction.offset, "DC2 process101 instruction owner");
                motion
            }
            None => Motion { pc: instruction.offset, ..Default::default() },
        };
        if !motion.redraw {
            if let Some(deadline) = motion.deadline {
                if now < deadline {
                    self.state.portrait.motion = Some(motion);
                    return Ok(Some(Outcome::WaitUntil(deadline)));
                }
            }
            let pose = &mut self.state.poses[index];
            let target = [i32::from(a[0]), i32::from(a[1])];
            if [i32::from(pose.shown[0]), i32::from(pose.shown[1])] == target {
                return self.advance(instruction.next, instruction.opcode).map(Some);
            }
            let speed = self.state.settings.prefix(12)?.unwrap_or(0);
            let step = if speed == 3 || control {
                64
            } else if speed == 2 {
                32
            } else if speed == 1 {
                16
            } else {
                8
            };
            for axis in 0..2 {
                let shown = i32::from(pose.shown[axis]);
                let moved = if shown < target[axis] {
                    (shown + step).min(target[axis])
                } else if shown > target[axis] {
                    (shown - step).max(target[axis])
                } else {
                    shown
                };
                pose.shown[axis] = i16::try_from(moved)?;
            }
            motion.redraw = true;
        }
        if !self.redraw_scene(instruction.offset, host)? {
            self.state.portrait.motion = Some(motion);
            return Ok(Some(Outcome::Pending));
        }
        motion.redraw = false;
        //uElapse=10 (40A970); the next tick re-tests the position.
        let deadline = now.checked_add(10).context("DC2 process101 timer overflow")?;
        motion.deadline = Some(deadline);
        self.state.portrait.motion = Some(motion);
        Ok(Some(Outcome::WaitUntil(deadline)))
    }
    pub(super) fn execute_portrait(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 4],
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Option<Outcome>> {
        if instruction.opcode != 0x2b {
            return Ok(None);
        }
        if self.state.portrait.transition.as_ref().is_some_and(|t| t.pc == instruction.offset) {
            return self.portrait_transition(instruction, host, now, control).map(Some);
        }
        if self.state.portrait.tail_pc == Some(instruction.offset) {
            return self.finish_portraits(instruction, host, now, control).map(Some);
        }
        let slot = a[0] as u8;
        if !self.state.portrait.pending {
            self.state.portrait.dirty = [800, 600, 0, 0];
        }
        if slot >= 5 {
            for (bit, native) in [(0x10, 1u8), (0x20, 2), (0x40, 3), (0x80, 4), (0x08, 5)] {
                if slot & bit != 0 && !self.unload_native(native, host)? {
                    return Ok(Some(Outcome::Pending));
                }
            }
        } else {
            let index = usize::from(slot);
            let native = slot + 1;
            ensure!(!text.starts_with("--"), "DC2 portrait rollback (B070F4/B084C0) is not connected");
            if self.state.poses[index].percent == 0 {
                self.state.poses[index].percent = 100;
            }
            if !text.is_empty() {
                let erase = text == "-";
                let loaded = self.native_name(native).is_some();
                if loaded || !erase {
                    if loaded {
                        self.extend_dirty(index, None)?;
                        self.state.portrait.pending = true;
                    }
                    if erase {
                        if !self.unload_native(native, host)? {
                            return Ok(Some(Outcome::Pending));
                        }
                        //41BCE0 also empties B0A69C for the erased handle.
                        if index < 4 {
                            self.state.portrait.expressions[index] = None;
                            self.state.portrait.stamped[index] = false;
                        }
                    } else {
                        //402FE0 tints a trailing '@' (404110) or '*' (404240);
                        //it has no '?' modifier or ',' CRM container.
                        ensure!(
                            !text.contains(['?', ',']),
                            "DC2 portrait name with '?'/',' has no 402FE0 meaning"
                        );
                        //41BCE0 clears CEBB44: the plain portrait is loaded.
                        if !self.load_portrait(index, text, None, host, control)? {
                            return Ok(Some(Outcome::Pending));
                        }
                        if index < 4 {
                            self.state.portrait.stamped[index] = false;
                        }
                        self.set_native_name(native, Some(text.into()));
                    }
                    if !erase {
                        let (_, logical) = host
                            .image_dimensions(host_slot(native))?
                            .context("DC2 loaded portrait extent")?;
                        self.extend_dirty(index, Some(logical))?;
                        self.state.portrait.pending = true;
                    }
                    let pose = &mut self.state.poses[index];
                    pose.shown = [pose.position[0], pose.position[1], pose.percent];
                }
            }
        }
        //41BCE0 tail: a following portrait change batches into this one.
        let following = program.instruction(instruction.next)?.opcode;
        let fog = slot < 5 && self.native_name(slot + 1).is_some_and(|name| name.contains("bst\\fog"));
        if matches!(following, 0x02 | 0x2b | 0x2c | 0x65) || fog {
            return self.advance(instruction.next, instruction.opcode).map(Some);
        }
        self.finish_portraits(instruction, host, now, control).map(Some)
    }
    ///41BC80 re-stamps every recorded expression that a reload cleared,
    ///then 41B920 presents the dirty rectangle.
    fn finish_portraits(
        &mut self,
        instruction: &Instruction,
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Outcome> {
        for index in 0..4 {
            let Some(base) = self.native_name(index as u8 + 1).map(str::to_owned) else { continue };
            if self.state.portrait.stamped[index] {
                continue;
            }
            let Some(recorded) = self.state.portrait.expressions[index].clone() else { continue };
            //41BC80 hands B0A69C back to 41BFE0, which rewrites it again for
            //the size of the portrait loaded now.
            let (_, logical) = host
                .image_dimensions(host_slot(index as u8 + 1))?
                .context("DC2 expression target extent")?;
            let expression = expression_name(&recorded, logical[0] > 882 && logical[1] > 756);
            if !self.stamp_expression(index, &base, &expression, host, control)? {
                self.state.portrait.tail_pc = Some(instruction.offset);
                return Ok(Outcome::Pending);
            }
            self.state.portrait.expressions[index] = Some(expression);
        }
        self.state.portrait.tail_pc = None;
        if !self.state.portrait.pending {
            return self.advance(instruction.next, instruction.opcode);
        }
        self.state.portrait.pending = false;
        self.state.portrait.transition = Some(Transition { pc: instruction.offset, ..Default::default() });
        self.portrait_transition(instruction, host, now, control)
    }
    ///41BCE0(4, name, 1) from 0x26: load or erase ("-") native5 and mark
    ///the 41BA90 dirty rectangle; a3=1 neither batches nor presents.
    pub(super) fn load_native5(&mut self, name: &str, host: &mut dyn NativeHost, control: bool) -> Result<bool> {
        if !self.state.portrait.pending {
            self.state.portrait.dirty = [800, 600, 0, 0];
        }
        if self.state.poses[4].percent == 0 {
            self.state.poses[4].percent = 100;
        }
        let erase = name == "-";
        let loaded = self.native_name(5).is_some();
        if !loaded && erase {
            return Ok(true);
        }
        if loaded {
            self.extend_dirty(4, None)?;
            self.state.portrait.pending = true;
        }
        if erase {
            return self.unload_native(5, host);
        }
        if !self.load_portrait(4, name, None, host, control)? {
            return Ok(false);
        }
        self.set_native_name(5, Some(name.into()));
        let (_, logical) = host.image_dimensions(host_slot(5))?.context("DC2 native5 extent")?;
        self.extend_dirty(4, Some(logical))?;
        self.state.portrait.pending = true;
        let pose = &mut self.state.poses[4];
        pose.shown = [pose.position[0], pose.position[1], pose.percent];
        Ok(true)
    }
    ///41C530 case48: B0A69C..B0A81C are emptied with natives 1..4.
    pub(super) fn clear_expressions(&mut self) {
        self.state.portrait.expressions = Default::default();
        self.state.portrait.stamped = [false; 4];
    }
    ///Portrait `index` as `name` (+ stamped `expression`) at its pose percent.
    ///419C50 reuses an unscaled same-name handle; a scaled or stamped one is
    ///reloaded. The host keys its slot on all three.
    fn load_portrait(
        &mut self,
        index: usize,
        name: &str,
        expression: Option<&str>,
        host: &mut dyn NativeHost,
        control: bool,
    ) -> Result<bool> {
        let percent = self.state.poses[index].percent;
        ensure!(percent > 0, "DC2 negative portrait percent");
        let slot = host_slot(index as u8 + 1);
        let event = if percent == 100 && expression.is_none() {
            Event::LoadImage { slot, name: name.into() }
        } else {
            ensure!(index < 4, "DC2 scaled native5 (41AB60) is not connected");
            //41A290: Ctrl (or CEBA20) defers to 40D7C0's draw-time stretch,
            //as does any percent of 200 or more.
            Event::LoadScaledImage {
                slot,
                name: name.into(),
                percent,
                nearest: control || percent >= 200,
                expression: expression.map(str::to_owned),
            }
        };
        host.emit(&event)
    }
    ///41BFE0 stamp of an already rewritten expression name onto portrait
    ///`index` (loaded as `base`), extending the dirty rectangle.
    fn stamp_expression(
        &mut self,
        index: usize,
        base: &str,
        expression: &str,
        host: &mut dyn NativeHost,
        control: bool,
    ) -> Result<bool> {
        if !self.state.portrait.pending {
            self.state.portrait.dirty = [800, 600, 0, 0];
        }
        if !self.load_portrait(index, base, Some(expression), host, control)? {
            return Ok(false);
        }
        self.extend_dirty(index, None)?;
        let (_, logical) = host
            .image_dimensions(host_slot(index as u8 + 1))?
            .context("DC2 stamped portrait extent")?;
        self.extend_dirty(index, Some(logical))?;
        self.state.portrait.pending = true;
        self.state.portrait.stamped[index] = true;
        Ok(true)
    }
    ///41C530 case44 -> 41BFE0(index, name): stamp an expression onto
    ///portrait `index` (0..3). "-" restores the plain portrait.
    pub(super) fn execute_expression(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 4],
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Option<Outcome>> {
        if instruction.opcode != 0x2c {
            return Ok(None);
        }
        if self.state.portrait.transition.as_ref().is_some_and(|t| t.pc == instruction.offset) {
            return self.portrait_transition(instruction, host, now, control).map(Some);
        }
        if self.state.portrait.tail_pc == Some(instruction.offset) {
            return self.finish_portraits(instruction, host, now, control).map(Some);
        }
        let index = usize::try_from(a[0]).ok().filter(|index| *index < 4);
        let index = index.context("DC2 0x2c expression slot outside CEBB44's four entries")?;
        ensure!(!text.starts_with("--"), "DC2 expression rollback (B07374) is not connected");
        if !self.state.portrait.pending {
            self.state.portrait.dirty = [800, 600, 0, 0];
        }
        let base = self.native_name(index as u8 + 1).map(str::to_owned);
        if !text.is_empty() {
            if text == "-" {
                if let Some(base) = &base {
                    if self.state.portrait.stamped[index] || self.state.poses[index].percent != 100 {
                        self.extend_dirty(index, None)?;
                        self.state.portrait.pending = true;
                        if !self.load_portrait(index, base, None, host, control)? {
                            return Ok(Some(Outcome::Pending));
                        }
                    } else {
                        self.extend_dirty(index, None)?;
                        self.state.portrait.pending = true;
                    }
                }
                self.state.portrait.stamped[index] = false;
                //41BC80's nested call empties B0A69C for the "-" record.
                self.state.portrait.expressions[index] = None;
            } else {
                ensure!(!text.contains(['?', ',']), "DC2 expression name with '?'/',' has no 402FE0 meaning");
                let expression = match &base {
                    Some(_) => {
                        let (_, logical) = host
                            .image_dimensions(host_slot(index as u8 + 1))?
                            .context("DC2 expression target extent")?;
                        let large = logical[0] > 882 && logical[1] > 756;
                        expression_name(text, large)
                    }
                    //No handle: 41BFE0 takes the ordinary-size branch.
                    None => expression_name(text, false),
                };
                if let Some(base) = &base {
                    if !self.stamp_expression(index, base, &expression, host, control)? {
                        return Ok(Some(Outcome::Pending));
                    }
                }
                self.state.portrait.expressions[index] = Some(expression);
            }
        }
        let following = program.instruction(instruction.next)?.opcode;
        if matches!(following, 0x02 | 0x2b | 0x2c | 0x65) {
            return self.advance(instruction.next, instruction.opcode).map(Some);
        }
        self.finish_portraits(instruction, host, now, control).map(Some)
    }
    fn portrait_transition(
        &mut self,
        instruction: &Instruction,
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Outcome> {
        let mut t = self.state.portrait.transition.clone().context("DC2 portrait transition")?;
        let skipping = self.state.runtime.get(&25).copied().unwrap_or(0) == 0
            && (control || self.state.runtime.get(&242).copied().unwrap_or(0) != 0);
        let speed = self.state.settings.prefix(12)?.unwrap_or(0);
        //Weather stays above the composed screen (ordering gap recorded in
        //dc2_weather.rs).
        ensure!(
            self.state.draw.planes.iter().all(Vec::is_empty),
            "DC2 portrait composition with plane records is not connected"
        );
        ensure!(
            !self.state.resources.native0_composed,
            "DC2 portrait composition over a baked native0 is not connected"
        );
        let owner = self
            .state
            .resources
            .capture_owner
            .clone()
            .unwrap_or_else(|| Owner { scene: self.state.scene.clone(), pc: instruction.offset });
        if self.state.resources.capture_owner.is_none() {
            if !host.bitmap_stage(&Request::Capture(owner.clone()))? {
                return Ok(Outcome::Pending);
            }
            self.state.resources.capture_owner = Some(owner.clone());
        }
        //41B920 AC7050 branch: 41ADE0 + 424880(native0) without portraits.
        if !t.native0_done && self.state.draw.native_composition_pending {
            let mut fade = t.native0.clone().unwrap_or_default();
            if !fade.mixing {
                if let Some(deadline) = fade.deadline {
                    if now < deadline {
                        self.state.portrait.transition = Some(t);
                        return Ok(Outcome::WaitUntil(deadline));
                    }
                    fade.deadline = None;
                }
                if fade.progress >= 16 {
                    if !host.bitmap_stage(&Request::Commit { owner: owner.clone(), slot: 1 })? {
                        return Ok(Outcome::Pending);
                    }
                    self.state.draw.native_composition_pending = false;
                    t.native0_done = true;
                    t.native0 = None;
                    //Next phase starts from the newly presented screen.
                    if !host.bitmap_stage(&Request::Capture(owner.clone()))? {
                        self.state.portrait.transition = Some(t);
                        return Ok(Outcome::Pending);
                    }
                } else {
                    let (step, weight) = if speed == 3 || skipping {
                        (8, 10)
                    } else if speed == 2 {
                        (4, 8)
                    } else if speed == 1 {
                        (2, 6)
                    } else {
                        (1, 3)
                    };
                    fade.progress = (fade.progress + step).min(16);
                    fade.source_weight = weight;
                    fade.deadline = Some(now + 40);
                    fade.mixing = true;
                }
            }
            if !t.native0_done {
                t.native0 = Some(fade.clone());
                self.state.portrait.transition = Some(t.clone());
                if !host.bitmap_stage(&Request::Mix { owner, slot: 1, source_weight: fade.source_weight })? {
                    return Ok(Outcome::Pending);
                }
                fade.mixing = false;
                let deadline = fade.deadline.context("DC2 portrait native0 deadline")?;
                t.native0 = Some(fade);
                self.state.portrait.transition = Some(t);
                return Ok(Outcome::WaitUntil(deadline));
            }
        }
        self.state.draw.native_composition_pending = false;
        //420A20: 4258C0(255) of bg + portraits into the work copy.
        if !t.prepared {
            let mut draw = self.state.resources.display_base.clone();
            draw.extend(self.portrait_draws(host)?);
            if self.state.primary_names[0].is_some()
                && !host.bitmap_stage(&Request::Prepare { owner: owner.clone(), slot: 1, draw })?
            {
                self.state.portrait.transition = Some(t);
                return Ok(Outcome::Pending);
            }
            ensure!(self.state.primary_names[0].is_some(), "DC2 portrait without native0 background is not connected");
            t.prepared = true;
            if speed == 3 || skipping {
                t.counter = 5;
                t.mixing = Some(16);
            }
        }
        if t.mixing.is_none() {
            if let Some(deadline) = t.deadline {
                if now < deadline {
                    self.state.portrait.transition = Some(t);
                    return Ok(Outcome::WaitUntil(deadline));
                }
                t.deadline = None;
            }
            if t.counter >= 5 {
                self.state.resources.capture_owner = None;
                self.state.portrait.transition = None;
                return self.advance(instruction.next, instruction.opcode);
            }
            let (add, weight) = match speed {
                2 => (3, 12),
                1 => (2, 10),
                _ => (1, 6),
            };
            t.counter = (t.counter + add).min(5);
            t.mixing = Some(if t.counter == 5 { 16 } else { weight });
            t.deadline = Some(now + 40);
        }
        let weight = t.mixing.context("DC2 bup_fade weight")?;
        self.state.portrait.transition = Some(t.clone());
        let done = if weight == 16 {
            host.bitmap_stage(&Request::Commit { owner, slot: 1 })?
        } else {
            host.bitmap_stage(&Request::Mix { owner, slot: 1, source_weight: weight })?
        };
        if !done {
            return Ok(Outcome::Pending);
        }
        t.mixing = None;
        if weight == 16 && t.deadline.is_none() {
            //Fast path: 40A930 without a timer.
            self.state.resources.capture_owner = None;
            self.state.portrait.transition = None;
            return self.advance(instruction.next, instruction.opcode);
        }
        let deadline = t.deadline.context("DC2 bup_fade deadline")?;
        self.state.portrait.transition = Some(t);
        Ok(Outcome::WaitUntil(deadline))
    }
}

///41BA90/41A950: at 100% only oversize images (>882 or >756) move about the
///centre; otherwise the position is `centre + pos - percent*centre/100`.
fn scaled_rect(position: [i32; 2], center: [i32; 2], percent: i32, [w, h]: [i32; 2]) -> [i32; 4] {
    let [x, y] = position;
    let [cx, cy] = center;
    if percent == 100 {
        let x = if w > 882 { cx + x - cx * (100 * w / 882) / 100 } else { x };
        let y = if h > 756 { cy + y - cy * (100 * h / 756) / 100 } else { y };
        return [x, y, w, h];
    }
    [cx + x - percent * cx / 100, cy + y - percent * cy / 100, w, h]
}

///41BFE0 name rewrite. With `large` (loaded handle wider than 882 and
///taller than 756) the "_b" variant is selected: "x@" -> "x_b@",
///"x*" -> "x_b*", "x" -> "x_b" (names already ending "_b" keep it).
///Otherwise "_b" is dropped and a trailing '@'/'*' tint is kept.
fn expression_name(name: &str, large: bool) -> String {
    let at = name
        .rfind(['.', '\\', '/'])
        .filter(|at| name.as_bytes()[*at] == b'.')
        .unwrap_or(name.len());
    let (stem, extension) = name.split_at(at);
    let (stem, tint) = match stem.as_bytes().last() {
        Some(b'@') => (&stem[..stem.len() - 1], "@"),
        Some(b'*') => (&stem[..stem.len() - 1], "*"),
        _ => (stem, ""),
    };
    if large {
        //Native appends the suffix after the extension when one exists;
        //scripts name these without one.
        let suffix = if stem.ends_with("_b") { tint.to_owned() } else { format!("_b{tint}") };
        return format!("{stem}{extension}{suffix}");
    }
    let stem = if !tint.is_empty() || stem.ends_with("_b") {
        stem.strip_suffix("_b").unwrap_or(stem)
    } else {
        stem
    };
    format!("{stem}{tint}{extension}")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn expression_names_follow_41bfe0() {
        assert_eq!(expression_name("bst\\e_yme_gra1", true), "bst\\e_yme_gra1_b");
        assert_eq!(expression_name("bst\\e_yme_gra1_b", false), "bst\\e_yme_gra1");
        assert_eq!(expression_name("bst\\e_a@", true), "bst\\e_a_b@");
        assert_eq!(expression_name("bst\\e_a_b*", true), "bst\\e_a_b*");
        assert_eq!(expression_name("bst\\e_a_b@", false), "bst\\e_a@");
        assert_eq!(expression_name("bst\\e_a_b", true), "bst\\e_a_b");
        assert_eq!(expression_name("bst\\e_a", false), "bst\\e_a");
    }
    #[test]
    fn scaled_rect_moves_about_the_centre() {
        assert_eq!(scaled_rect([12, 214], [441, 322], 75, [495, 425]), [123, 295, 495, 425]);
        assert_eq!(scaled_rect([160, 0], [441, 322], 100, [1260, 1080]), [-25, -135, 1260, 1080]);
    }
}

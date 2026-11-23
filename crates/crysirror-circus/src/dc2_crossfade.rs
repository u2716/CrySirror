//! DC2 41C530 case13 selector20 -> 41F630 case20 crossfade. The old screen
//! ("fadechg2", 403CB0) is frozen once; each timer step adds 2*v8 to a /256
//! weight and 40D0A0 blends the newly composed scene over that capture.
//! The40ms (or operand-derived) timer and completion PC remain VM policy.
use super::*;
use crate::dc2_bitmap::{Owner, Request};
///50E828: text-speed steps; 50E838 = the same table from index4.
const STEPS: [u16; 12] = [8, 16, 32, 64, 64, 64, 32, 16, 8, 4, 2, 1];
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct Crossfade {
    progress: u16,
    deadline: Option<u64>,
    pending: Option<u16>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct Flash {
    started: bool,
    step: u16,
    deadline: Option<u64>,
    pending: Option<u16>,
}
impl Machine {
    ///A preceding 0x30 may already hold the frozen screen under its owner.
    fn effect_owner(&self, pc: u32) -> Owner {
        self.state
            .resources
            .capture_owner
            .clone()
            .unwrap_or_else(|| Owner { scene: self.state.scene.clone(), pc })
    }
    ///41C530 case13 selector5: 41ADE0 if pending, then 4258C0(255) redraws
    ///the whole scene into the screen and 40A930 presents it (state2 makes
    ///422E60 continue). No timer or blend is involved.
    pub(super) fn execute_redraw(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
    ) -> Result<Option<Outcome>> {
        //Selector12/15 run 420090(0,0,800,600) (+4224F0 for a shown window,
        //which stays a retained text layer here): the same immediate redraw.
        if !(instruction.opcode == 0x0d && matches!(a[0], 5 | 12 | 15)) {
            return Ok(None);
        }
        ensure!(
            self.state.draw.planes.iter().all(Vec::is_empty),
            "DC2 redraw with pending 41ADE0 display records is not connected"
        );
        if !self.redraw_scene(instruction.offset, host)? {
            return Ok(Some(Outcome::Pending));
        }
        self.advance(instruction.next, instruction.opcode).map(Some)
    }
    ///4258C0(255) + 40A930 over the engine: commit native0 at once.
    pub(super) fn redraw_scene(&mut self, pc: u32, host: &mut dyn NativeHost) -> Result<bool> {
        self.state.draw.native_composition_pending = false;
        if self.state.primary_names[0].is_none() && !self.state.resources.native0_composed {
            //No native0: the retained engine scene already is the redraw.
            return Ok(true);
        }
        let owner = self
            .state
            .resources
            .capture_owner
            .clone()
            .unwrap_or_else(|| Owner { scene: self.state.scene.clone(), pc });
        if self.state.resources.capture_owner.is_none() {
            if !host.bitmap_stage(&Request::Capture(owner.clone()))? {
                return Ok(false);
            }
            self.state.resources.capture_owner = Some(owner.clone());
        }
        let portraits = if self.state.resources.redraw_prepared.as_ref() == Some(&owner) { Vec::new() } else { self.portrait_draws(host)? };
        if !portraits.is_empty() {
            ensure!(
                !self.state.resources.native0_composed,
                "DC2 portrait redraw over baked native0 is not connected"
            );
            let mut draw = self.state.resources.display_base.clone();
            draw.extend(portraits);
            if !host.bitmap_stage(&Request::Prepare { owner: owner.clone(), slot: 1, draw })? {
                return Ok(false);
            }
            self.state.resources.redraw_prepared = Some(owner.clone());
        }
        if !host.bitmap_stage(&Request::Commit { owner, slot: 1 })? {
            return Ok(false);
        }
        self.state.resources.redraw_prepared = None;
        self.state.resources.capture_owner = None;
        self.state.resources.transition = None;
        Ok(true)
    }
    ///41C530 case13 default (selector3) -> 41F630 case3 "flash": AFA048
    ///counts to 2*v0; odd steps fill palette7, even steps restore the copy.
    pub(super) fn execute_flash(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>> {
        if !(instruction.opcode == 0x0d && a[0] == 3) {
            return Ok(None);
        }
        let mut flash = self.state.flash.clone().unwrap_or_default();
        if !flash.started {
            ensure!(
                self.state.draw.planes.iter().all(Vec::is_empty),
                "DC2 flash with pending 41ADE0 display records is not connected"
            );
            //Default branch: AC7050 -> 41ADE0 + 4258C0(255) + 40A930 first.
            if self.state.draw.native_composition_pending && !self.redraw_scene(instruction.offset, host)? {
                return Ok(Some(Outcome::Pending));
            }
            flash.started = true;
            self.state.flash = Some(flash.clone());
        }
        let owner = self.effect_owner(instruction.offset);
        if flash.pending.is_none() {
            if let Some(deadline) = flash.deadline {
                if now < deadline {
                    return Ok(Some(Outcome::WaitUntil(deadline)));
                }
                flash.deadline = None;
            }
            let total = 2 * i32::from(a[1] as u8);
            if i32::from(flash.step) >= total {
                if flash.step > 0 && !host.bitmap_stage(&Request::Flash { owner, fill: None, finalize: true })? {
                    return Ok(Some(Outcome::Pending));
                }
                self.state.flash = None;
                self.state.resources.capture_owner = None;
                return self.advance(instruction.next, instruction.opcode).map(Some);
            }
            if flash.step == 0 && !host.bitmap_stage(&Request::Capture(owner.clone()))? {
                return Ok(Some(Outcome::Pending));
            }
            flash.pending = Some(flash.step + 1);
            flash.deadline = Some(now.checked_add(40).context("DC2 flash timer overflow")?);
            self.state.flash = Some(flash.clone());
        }
        let step = flash.pending.context("DC2 flash pending step")?;
        let fill = if step & 1 == 1 {
            let raw = self.state.palette.get(&7).copied().unwrap_or([255; 3]);
            Some([raw[1], raw[0], raw[2]])
        } else {
            None
        };
        if !host.bitmap_stage(&Request::Flash { owner, fill, finalize: false })? {
            return Ok(Some(Outcome::Pending));
        }
        flash.step = step;
        flash.pending = None;
        let deadline = flash.deadline.context("DC2 flash step deadline")?;
        self.state.flash = Some(flash);
        Ok(Some(Outcome::WaitUntil(deadline)))
    }
    pub(super) fn execute_crossfade(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Option<Outcome>> {
        if !(instruction.opcode == 0x0d && matches!(a[0], 20..=22)) {
            return Ok(None);
        }
        ensure!(a[0] == 20, "DC2 41F630 case{} uses 40CF20, which is not connected", a[0]);
        //41ADE0 (AC7050) with no declared records leaves native0 unchanged,
        //as in the424880 path.
        ensure!(
            self.state.draw.planes.iter().all(Vec::is_empty),
            "DC2 crossfade with pending 41ADE0 display records is not connected"
        );
        self.state.draw.native_composition_pending = false;
        ensure!(
            self.state.primary_names[1..].iter().all(Option::is_none)
                && self.state.weather.is_none(),
            "DC2 crossfade over portraits/weather (4258C0 layers 2..8) is not connected"
        );
        let owner = self
            .state
            .resources
            .capture_owner
            .clone()
            .unwrap_or_else(|| Owner { scene: self.state.scene.clone(), pc: instruction.offset });
        if self.state.resources.capture_owner.is_none() {
            if !host.bitmap_stage(&Request::Capture(owner.clone()))? {
                return Ok(Some(Outcome::Pending));
            }
            self.state.resources.capture_owner = Some(owner.clone());
        }
        let mut fade = self.state.crossfade.clone().unwrap_or_default();
        if fade.pending.is_none() {
            if let Some(deadline) = fade.deadline {
                if now < deadline {
                    return Ok(Some(Outcome::WaitUntil(deadline)));
                }
                fade.deadline = None;
            }
            if fade.progress >= 256 {
                if !host.bitmap_stage(&Request::Commit { owner, slot: 1 })? {
                    return Ok(Some(Outcome::Pending));
                }
                self.state.resources.capture_owner = None;
                self.state.crossfade = None;
                return self.advance(instruction.next, instruction.opcode).map(Some);
            }
            let skipping = self.state.runtime.get(&25).copied().unwrap_or(0) == 0
                && (control || self.state.runtime.get(&242).copied().unwrap_or(0) != 0);
            //B0CE18 (case22 selector13) is never set by an accepted path: 0.
            let speed = a[1];
            let (step, elapse) = if speed != 0 {
                if speed >= 8 {
                    (1, 8 * (5 * u64::from(speed as u16) - 35))
                } else {
                    (STEPS[4 + speed as usize], 40)
                }
            } else if skipping {
                (STEPS[3], 40)
            } else {
                let index = self.state.settings.prefix(12)?.unwrap_or(0);
                (STEPS[usize::try_from(index.clamp(0, 3))?], 40)
            };
            fade.pending = Some((fade.progress + 2 * step).min(256));
            fade.deadline = Some(now.checked_add(elapse).context("DC2 crossfade timer overflow")?);
            self.state.crossfade = Some(fade.clone());
        }
        let weight = fade.pending.context("DC2 crossfade pending step")?;
        if !host.bitmap_stage(&Request::Crossfade { owner, slot: 1, weight })? {
            return Ok(Some(Outcome::Pending));
        }
        fade.progress = weight;
        fade.pending = None;
        let deadline = fade.deadline.context("DC2 crossfade step deadline")?;
        self.state.crossfade = Some(fade);
        Ok(Some(Outcome::WaitUntil(deadline)))
    }
}
impl Machine {
    ///41C530 case13 default (selector4/6) -> 41F630 case4 "yure" (x) or
    ///case6 "tateyure" (y): nine 40ms steps (3 then 9 when fast/skipping),
    ///step%3 = 1 -> -20 with the trailing edge filled, 2 -> +20 with the
    ///leading19 pixels filled, 0 -> the copy in place.
    pub(super) fn execute_shake(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Option<Outcome>> {
        if !(instruction.opcode == 0x0d && matches!(a[0], 4 | 6)) {
            return Ok(None);
        }
        let vertical = a[0] == 6;
        let mut shake = self.state.flash.clone().unwrap_or_default();
        if !shake.started {
            ensure!(
                self.state.draw.planes.iter().all(Vec::is_empty),
                "DC2 shake with pending 41ADE0 display records is not connected"
            );
            if self.state.draw.native_composition_pending && !self.redraw_scene(instruction.offset, host)? {
                return Ok(Some(Outcome::Pending));
            }
            shake.started = true;
            self.state.flash = Some(shake.clone());
        }
        let owner = self.effect_owner(instruction.offset);
        if shake.pending.is_none() {
            if let Some(deadline) = shake.deadline {
                if now < deadline {
                    return Ok(Some(Outcome::WaitUntil(deadline)));
                }
                shake.deadline = None;
            }
            if shake.step == 9 {
                if !host.bitmap_stage(&Request::Flash { owner, fill: None, finalize: true })? {
                    return Ok(Some(Outcome::Pending));
                }
                self.state.flash = None;
                self.state.resources.capture_owner = None;
                return self.advance(instruction.next, instruction.opcode).map(Some);
            }
            if shake.step == 0 && !host.bitmap_stage(&Request::Capture(owner.clone()))? {
                return Ok(Some(Outcome::Pending));
            }
            let skipping = self.state.runtime.get(&25).copied().unwrap_or(0) == 0
                && (control || self.state.runtime.get(&242).copied().unwrap_or(0) != 0);
            let fast = self.state.settings.prefix(12)?.unwrap_or(0) == 3 || skipping;
            let next = if fast && shake.step == 3 { 9 } else { (shake.step + 1).min(9) };
            shake.pending = Some(next);
            shake.deadline = Some(now.checked_add(40).context("DC2 shake timer overflow")?);
            self.state.flash = Some(shake.clone());
        }
        let step = shake.pending.context("DC2 shake pending step")?;
        let raw = self.state.palette.get(&0).copied().unwrap_or([0; 3]);
        let color = [raw[1], raw[0], raw[2]];
        let (offset, fill) = match (step % 3, vertical) {
            (1, false) => ([-20, 0], Some(([780, 0, 800, 600], color))),
            (2, false) => ([20, 0], Some(([0, 0, 19, 600], color))),
            (1, true) => ([0, -20], Some(([0, 580, 800, 600], color))),
            (2, true) => ([0, 20], Some(([0, 0, 800, 19], color))),
            _ => ([0, 0], None),
        };
        if !host.bitmap_stage(&Request::Shake { owner, offset, fill })? {
            return Ok(Some(Outcome::Pending));
        }
        shake.step = step;
        shake.pending = None;
        let deadline = shake.deadline.context("DC2 shake step deadline")?;
        self.state.flash = Some(shake);
        Ok(Some(Outcome::WaitUntil(deadline)))
    }
}
impl Machine {
    ///41C530 case13 selector1 -> 41F630 case1: fade the composed scene in
    ///from black, +8/+4/+2/+1 per 40ms by text speed (8 when skipping).
    pub(super) fn execute_fade_in(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Option<Outcome>> {
        if !(instruction.opcode == 0x0d && a[0] == 1) {
            return Ok(None);
        }
        let mut fade = self.state.flash.clone().unwrap_or_default();
        if !fade.started {
            ensure!(
                self.state.draw.planes.iter().all(Vec::is_empty),
                "DC2 fade-in with pending 41ADE0 display records is not connected"
            );
            ensure!(
                self.state.primary_names[1..].iter().all(Option::is_none) && self.state.weather.is_none(),
                "DC2 fade-in over portraits/weather (4258C0 layers 2..8) is not connected"
            );
            ensure!(
                self.state.primary_names[0].is_some() || self.state.resources.native0_composed,
                "DC2 fade-in without native0 is not connected"
            );
            //Selector1 skips the default branch's 4258C0(255) redraw.
            self.state.draw.native_composition_pending = false;
            fade.started = true;
            self.state.flash = Some(fade.clone());
        }
        let owner = self.effect_owner(instruction.offset);
        if fade.pending.is_none() {
            if let Some(deadline) = fade.deadline {
                if now < deadline {
                    return Ok(Some(Outcome::WaitUntil(deadline)));
                }
                fade.deadline = None;
            }
            if fade.step >= 16 {
                if !host.bitmap_stage(&Request::Commit { owner, slot: 1 })? {
                    return Ok(Some(Outcome::Pending));
                }
                self.state.flash = None;
                self.state.resources.capture_owner = None;
                return self.advance(instruction.next, instruction.opcode).map(Some);
            }
            let skipping = self.state.runtime.get(&25).copied().unwrap_or(0) == 0
                && (control || self.state.runtime.get(&242).copied().unwrap_or(0) != 0);
            let step = match self.state.settings.prefix(12)?.unwrap_or(0) {
                _ if skipping => 8,
                3 => 8,
                2 => 4,
                1 => 2,
                _ => 1,
            };
            fade.pending = Some((fade.step + step).min(16));
            fade.deadline = Some(now.checked_add(40).context("DC2 fade-in timer overflow")?);
            self.state.flash = Some(fade.clone());
        }
        let weight = fade.pending.context("DC2 fade-in pending step")?;
        if !host.bitmap_stage(&Request::FadeIn { owner, slot: 1, weight: u8::try_from(weight)? })? {
            return Ok(Some(Outcome::Pending));
        }
        fade.step = weight;
        fade.pending = None;
        let deadline = fade.deadline.context("DC2 fade-in step deadline")?;
        self.state.flash = Some(fade);
        Ok(Some(Outcome::WaitUntil(deadline)))
    }
}

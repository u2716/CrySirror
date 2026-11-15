//!41C530/422E60 process18 and4239B0's two release callbacks. Ordinary
//!waits publish41ADE0/41B300's completed native bitmap before suspending;
//!zero count is infinite, rather than an invented zero-duration success.
use super::*;
use crate::dc2_bitmap::{Owner, Request};
use crate::machine::{
    gesture_ops::{GestureReply, GestureRequest},
    system_ops::{SystemReply, SystemRequest},
};
use crysirror_core::service::buttons;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Wait {
    pc: u32,
    owner: Owner,
    flags: u8,
    due: Option<u64>,
    bake: bool,
    captured: bool,
    presented: bool,
    published_fence: bool,
}
impl Machine {
    pub(super) fn execute_wait(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Option<Outcome>> {
        if matches!(instruction.opcode, 0x3b | 0x3c) {
            let raw = program.literal_bytes(instruction.offset).context("DC2 original release callback")?;
            ensure!(raw.len() < 32, "DC2 release callback exceeds its native32-byte field");
            self.state.release_callbacks[usize::from(instruction.opcode - 0x3b)] = raw.to_vec();
            return self.advance(instruction.next, instruction.opcode).map(Some);
        }
        if instruction.opcode != 0x12 { return Ok(None); }
        if self.state.native_wait.is_none() {
            ensure!(a[0] >= 0 && a[1] & !9 == 0,
                "DC2 animated-text/anchored-clock timer requires its original process18 profile");
            let interval = if a[1] & 8 != 0 { 10 } else { 200 };
            let due = if a[0] == 0 { None } else {
                Some(now.checked_add(u64::try_from(a[0])? * interval).context("DC2 timer deadline overflow")?)
            };
            self.state.native_wait = Some(Wait {
                pc: instruction.offset,
                owner: self.state.resources.capture_owner.clone().unwrap_or_else(|| Owner {
                    scene: self.state.scene.clone(), pc: instruction.offset,
                }),
                flags: a[1] as u8, due,
                bake: !self.state.draw.native_composition_pending,
                captured: self.state.resources.capture_owner.is_some(),
                presented: !self.state.draw.native_composition_pending,
                published_fence: false,
            });
        }
        let mut wait = self.state.native_wait.as_ref().unwrap().clone();
        ensure!(wait.pc == instruction.offset && wait.owner.scene == self.state.scene,
            "DC2 timer scene/instruction owner");
        if !wait.bake {
            let Some((_, draw)) = self.default_composition(host, false)? else {return Ok(Some(Outcome::Pending));};
            if !host.bitmap_stage(&Request::Bake {owner: wait.owner.clone(), slot: 1, draw})? {
                return Ok(Some(Outcome::Pending));
            }
            self.state.draw.planes[0].clear();
            self.state.resources.display_base.clear();
            self.state.resources.native0_composed = true;
            self.state.draw.native_composition_pending = false;
            wait.bake = true;
            self.state.native_wait = Some(wait.clone());
        }
        if !wait.presented {
            if !wait.captured {
                if !host.bitmap_stage(&Request::Capture(wait.owner.clone()))? {return Ok(Some(Outcome::Pending));}
                wait.captured = true;
                self.state.resources.capture_owner = Some(wait.owner.clone());
                self.state.native_wait = Some(wait.clone());
            }
            //41B3A0 redraws the full native background, without starting a
            //feedback effect. The same engine clipped-copy operation used by
            //process98 handles this source16/16 receipt.
            if !host.bitmap_stage(&Request::Pan {
                owner: wait.owner.clone(), slot: 1,
                position: self.state.resources.background_position,
                source_weight: 16, finalize: true,
            })? {return Ok(Some(Outcome::Pending));}
            self.state.resources.capture_owner = None;
            wait.presented = true;
            self.state.native_wait = Some(wait.clone());
        }
        if !wait.published_fence {
            wait.published_fence = true;
            self.state.native_wait = Some(wait.clone());
            //Do not hide the completed screen behind a VM-pending flag for
            //the complete duration of an infinite input wait.
            return Ok(Some(Outcome::WaitUntil(now)));
        }
        let Some(reply) = host.system(SystemRequest::CallbackInput)? else {return Ok(Some(Outcome::Pending));};
        let SystemReply::CallbackInput {pressed: released, ..} = reply else {bail!("DC2 timer release observation receipt")};
        if crate::trace_enabled() && released != 0 {
            eprintln!("DC2 wait released={released:#x} flags={} callbacks={:?}", wait.flags, self.state.release_callbacks);
        }
        let mut callback = None;
        let mut consumed = 0;
        for (index, mask) in [buttons::MOUSE_RIGHT | buttons::ESCAPE, buttons::MOUSE_LEFT | buttons::ENTER].into_iter().enumerate() {
            if released & mask != 0 && !self.state.release_callbacks[index].is_empty() {
                callback = Some((index, named_label(program, &self.state.release_callbacks[index])?));
                consumed = mask;
                break;
            }
        }
        let interruption = wait.flags & 1 != 0 && released & (buttons::MOUSE_LEFT | buttons::MOUSE_RIGHT | buttons::ENTER | buttons::ESCAPE) != 0;
        let skipping = self.state.runtime.get(&313).copied().unwrap_or(0) != 0
            && self.state.runtime.get(&25).copied().unwrap_or(0) == 0
            && (control || self.state.runtime.get(&242).copied().unwrap_or(0) != 0);
        if callback.is_some() || interruption || skipping || wait.due.is_some_and(|due| now >= due) {
            if interruption && consumed == 0 {
                consumed = buttons::MOUSE_LEFT | buttons::MOUSE_RIGHT | buttons::ENTER | buttons::ESCAPE;
            }
            if consumed != 0 {
                match host.system(SystemRequest::Gesture(GestureRequest::ConsumeInput {mask: consumed}))? {
                    None => return Ok(Some(Outcome::Pending)),
                    Some(SystemReply::Gesture(GestureReply::Done)) => {},
                    Some(_) => bail!("DC2 timer input-consumption receipt"),
                }
            }
            self.state.native_wait = None;
            self.advance(instruction.next, instruction.opcode)?;
            if let Some((index, pc)) = callback {
                self.state.release_callbacks[index].clear();
                self.state.pc = pc;
            }
            return Ok(Some(Outcome::Advanced));
        }
        Ok(Some(match wait.due {Some(deadline) => Outcome::WaitUntil(deadline), None => Outcome::WaitForInput}))
    }
}

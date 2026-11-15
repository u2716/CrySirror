//!41F630 case2's captured-screen fade.40CCD0 scales the original stored
//! RGB bytes by(16-progress)/16; it never repeatedly darkens the last result.
//! The40ms native timer and completion-dependent PC remain VM policy.
use super::*;
use crate::machine::resource_ops::{ResourceReply, ResourceRequest};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Fade {
    owner: crate::dc2_bitmap::Owner,
    captured: bool,
    released: bool,
    progress: u8,
    deadline: Option<u64>,
    pending: Option<u8>,
}
impl Machine {
    pub(super) fn execute_fade(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Option<Outcome>> {
        use crate::dc2_bitmap::Request;
        if instruction.opcode != 0x0d || a[0] != 2 {
            return Ok(None);
        }
        if self.state.fade.is_none() {
            ensure!(
                self.state.resources.capture_owner.is_none()
                    && self.state.resources.transition.is_none(),
                "DC2 dimmer entered before prior feedback completion"
            );
            self.state.fade = Some(Fade {
                owner: crate::dc2_bitmap::Owner {
                    scene: self.state.scene.clone(),
                    pc: instruction.offset,
                },
                captured: false,
                released: false,
                progress: 0,
                deadline: None,
                pending: None,
            });
        }
        let mut fade = self.state.fade.as_ref().unwrap().clone();
        ensure!(
            fade.owner.pc == instruction.offset && fade.owner.scene == self.state.scene,
            "DC2 dimmer native instruction owner"
        );
        if !fade.captured {
            if !host.bitmap_stage(&Request::Capture(fade.owner.clone()))? {
                return Ok(Some(Outcome::Pending));
            }
            fade.captured = true;
            self.state.fade = Some(fade.clone());
        }
        if !fade.released {
            match host.resource(&ResourceRequest::Unload { slot: 1 })? {
                None => return Ok(Some(Outcome::Pending)),
                Some(ResourceReply::Done) => {}
                Some(_) => bail!("DC2 dimmer release foreign receipt"),
            }
            //419B60(0) clears the file-backed native0 and both declaration
            //counts;403CB0("fadeout") retains a separately captured bitmap.
            self.state.primary_names[0] = None;
            self.state.resources.native0_composed = false;
            self.state.primary_ids[0] = -1;
            self.state.draw.planes.iter_mut().for_each(Vec::clear);
            self.state.resources.display_base.clear();
            self.state.draw.native_composition_pending = false;
            fade.released = true;
            self.state.fade = Some(fade.clone());
        }
        if fade.pending.is_none() {
            if let Some(deadline) = fade.deadline {
                if now < deadline {
                    return Ok(Some(Outcome::WaitUntil(deadline)));
                }
                fade.deadline = None;
            }
            if fade.progress == 16 {
                self.state.fade = None;
                return self.advance(instruction.next, instruction.opcode).map(Some);
            }
            let skipping = self.state.runtime.get(&25).copied().unwrap_or(0) == 0
                && (control || self.state.runtime.get(&242).copied().unwrap_or(0) != 0);
            let step = if skipping {
                8
            } else {
                match self.state.settings.prefix(12)?.unwrap_or(0) {
                    3 => 8,
                    2 => 4,
                    1 => 2,
                    _ => 1,
                }
            };
            fade.pending = Some(fade.progress.saturating_add(step).min(16));
            fade.deadline = Some(now.checked_add(40).context("DC2 dimmer timer overflow")?);
            self.state.fade = Some(fade.clone());
        }
        let next = fade.pending.context("DC2 dimmer pending step")?;
        if !host.bitmap_stage(&Request::Darken {
            owner: fade.owner.clone(),
            source_weight: 16 - next,
        })? {
            return Ok(Some(Outcome::Pending));
        }
        fade.progress = next;
        fade.pending = None;
        let deadline = fade.deadline.context("DC2 dimmer step deadline")?;
        self.state.fade = Some(fade);
        Ok(Some(Outcome::WaitUntil(deadline)))
    }
}

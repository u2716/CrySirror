//! 41C530 case13 selector13 -> 424CA0: a per-pixel /16 feedback wipe of
//! native0 over the presented screen. 424B40 builds the weight field from
//! operand1; every 40 ms step adds the running AFA048 to all weights and
//! 40EAB0 blends `(w*src + (16-w)*dst) >> 4` with w clamped to 0..16. The
//! shared composite (MaskIncrement256) owns the arithmetic; the VM keeps the
//! field policy, AFA048 schedule and completion.
use super::*;
use crate::dc2_bitmap::{Owner, Request};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Wipe {
    ///Bitmap owner; may be a capture taken by a preceding 0x30.
    owner: Owner,
    scene: String,
    pc: u32,
    mode: u8,
    ///AFA048.
    step: i32,
    ///Sum added to every 424B40 weight so far.
    offset: i32,
    started: bool,
    ///The last blended field had every weight >= 16 (v2 == 0).
    complete: bool,
    blending: bool,
    deadline: Option<u64>,
}
///424B40 weight before any increment; `None` for modes it does not write.
pub fn field(mode: u8, x: i32, y: i32) -> Option<i32> {
    match mode {
        0 => Some(0),
        1 => Some(8 - x / 4),
        //v5 / -4 truncates toward zero for the non-negative row.
        2 => Some(8 - y / 4 - x / 4),
        _ => None,
    }
}
///Smallest weight in an 800x600 field.
fn minimum(mode: u8) -> Option<i32> {
    field(mode, 799, 599)
}
impl Machine {
    pub(super) fn execute_wipe(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Option<Outcome>> {
        if !(instruction.opcode == 0x0d && a[0] == 13) {
            return Ok(None);
        }
        let mut wipe = match self.state.wipe.take() {
            Some(wipe) => {
                ensure!(
                    wipe.scene == self.state.scene && wipe.pc == instruction.offset,
                    "DC2 424CA0 instruction owner"
                );
                wipe
            }
            None => {
                ensure!(
                    self.state.draw.planes.iter().all(Vec::is_empty),
                    "DC2 424CA0 with pending 41ADE0 display records is not connected"
                );
                //424CA0 blends the raw native0 handle; 4258C0 portraits are
                //not part of it (unlike 424880's 403E80 copy).
                ensure!(
                    self.state.primary_names[1..].iter().all(Option::is_none),
                    "DC2 424CA0 with loaded portraits is not connected"
                );
                ensure!(
                    self.state.primary_names[0].is_some() || self.state.resources.native0_composed,
                    "DC2 424CA0 without native0 (40E740 name diagnostic) is not connected"
                );
                ensure!(
                    self.state.resources.background_position == [0, 0],
                    "DC2 424CA0 at a nonzero B0B7D0/B0B7D4 origin is not connected"
                );
                let mode = u8::try_from(a[1]).ok().filter(|mode| minimum(*mode).is_some());
                let mode = mode.with_context(|| {
                    format!("DC2 424B40 field mode {} (99 random / stale field) is not connected", a[1])
                })?;
                Wipe {
                    owner: self.state.resources.capture_owner.clone().unwrap_or_else(|| Owner {
                        scene: self.state.scene.clone(),
                        pc: instruction.offset,
                    }),
                    scene: self.state.scene.clone(),
                    pc: instruction.offset,
                    mode,
                    step: 0,
                    offset: 0,
                    started: false,
                    complete: false,
                    blending: false,
                    deadline: None,
                }
            }
        };
        if self.state.resources.capture_owner.is_none() {
            if !host.bitmap_stage(&Request::Capture(wipe.owner.clone()))? {
                self.state.wipe = Some(wipe);
                return Ok(Some(Outcome::Pending));
            }
            self.state.resources.capture_owner = Some(wipe.owner.clone());
        }
        self.state.draw.native_composition_pending = false;
        if !wipe.blending {
            if let Some(deadline) = wipe.deadline {
                if now < deadline {
                    self.state.wipe = Some(wipe);
                    return Ok(Some(Outcome::WaitUntil(deadline)));
                }
            }
            if wipe.complete {
                //40B1E0 copies native0 itself.
                if !host.bitmap_stage(&Request::Commit { owner: wipe.owner.clone(), slot: 1 })? {
                    self.state.wipe = Some(wipe);
                    return Ok(Some(Outcome::Pending));
                }
                self.state.resources.capture_owner = None;
                self.state.resources.transition = None;
                return self.advance(instruction.next, instruction.opcode).map(Some);
            }
            let speed = self.state.settings.prefix(12)?.unwrap_or(0);
            wipe.step += if speed == 3 || control {
                8
            } else if speed == 2 {
                4
            } else if speed == 1 {
                2
            } else {
                1
            };
            if wipe.started {
                wipe.offset = wipe.offset.checked_add(wipe.step).context("DC2 424CA0 weight overflow")?;
                wipe.complete = minimum(wipe.mode).context("DC2 424B40 field")? + wipe.offset >= 16;
            }
            wipe.started = true;
            wipe.deadline = Some(now.checked_add(40).context("DC2 424CA0 timer overflow")?);
            wipe.blending = true;
        }
        if !host.bitmap_stage(&Request::Wipe {
            owner: wipe.owner.clone(),
            slot: 1,
            mode: wipe.mode,
            offset: wipe.offset,
        })? {
            self.state.wipe = Some(wipe);
            return Ok(Some(Outcome::Pending));
        }
        wipe.blending = false;
        wipe.step = wipe.step.min(16);
        let deadline = wipe.deadline.context("DC2 424CA0 deadline")?;
        self.state.wipe = Some(wipe);
        Ok(Some(Outcome::WaitUntil(deadline)))
    }
}

//!41C530 case98 /420880. Native position, feedback tail and timer receipts
//!remain project state; shared immutable Image jobs perform clipped byte blends.
use super::*;
use crate::dc2_bitmap::{Owner, Request};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Pan {
    owner: Owner,
    pc: u32,
    weight: u8,
    tail: u8,
    speed: i16,
    target: [i32; 2],
    deadline: Option<u64>,
    pending: Option<([i32; 2], u8)>,
    final_frame: bool,
}
impl Machine {
    pub(super) fn execute_pan(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Option<Outcome>> {
        if instruction.opcode != 0x62 {
            return Ok(None);
        }
        match a[2] {
            1 => {
                self.state.resources.background_position = [a[0].into(), a[1].into()];
                return self.advance(instruction.next, instruction.opcode).map(Some);
            }
            2 => {
                self.state.resources.secondary_position = [a[0].into(), a[1].into()];
                return self.advance(instruction.next, instruction.opcode).map(Some);
            }
            0 | 3 | 8..=135 => {}
            //The source switch has no body for these selectors.
            4..=7 => return self.advance(instruction.next, instruction.opcode).map(Some),
            _ => bail!("DC2 native pan speed selector {} is not mapped", a[2]),
        }
        if self.state.pan.is_none() {
            ensure!(
                self.state.fade.is_none() && self.state.resources.transition.is_none(),
                "DC2 pan overlaps a native effect"
            );
            ensure!(
                self.state.primary_names[1..].iter().all(Option::is_none)
                    && self.state.builder.count == 0
                    && self.state.draw.planes.iter().all(Vec::is_empty),
                "DC2 pan with live portraits/UI requires their original composition"
            );
            let weight = if a[2] == 3 {
                0
            } else {
                a[3].clamp(0, 16) as u8
            };
            let target = [i32::from(a[0]), i32::from(a[1])];
            //The original returns before drawing or allocating a timer here.
            if target == self.state.resources.background_position && weight == 0 {
                return self.advance(instruction.next, instruction.opcode).map(Some);
            }
            let owner = self
                .state
                .resources
                .capture_owner
                .clone()
                .unwrap_or_else(|| Owner {
                    scene: self.state.scene.clone(),
                    pc: instruction.offset,
                });
            self.state.pan = Some(Pan {
                owner,
                pc: instruction.offset,
                weight,
                tail: 0,
                speed: if a[2] == 3 {
                    a[3].checked_add(8).context("DC2 pan speed overflow")?
                } else {
                    a[2]
                },
                target,
                deadline: None,
                pending: None,
                final_frame: false,
            });
        }
        let mut pan = self.state.pan.as_ref().unwrap().clone();
        ensure!(
            pan.owner.scene == self.state.scene && pan.pc == instruction.offset,
            "DC2 pan scene/instruction owner"
        );
        if pan.pending.is_none() {
            if let Some(deadline) = pan.deadline {
                if now < deadline {
                    return Ok(Some(Outcome::WaitUntil(deadline)));
                }
                pan.deadline = None;
            }
            if pan.final_frame {
                self.state.resources.capture_owner = None;
                self.state.draw.native_composition_pending = false;
                self.state.pan = None;
                return self.advance(instruction.next, instruction.opcode).map(Some);
            }
            if self.state.resources.background_position == pan.target && pan.weight != 0 {
                pan.tail += 1;
                if u16::from(pan.weight) + u16::from(pan.tail) > 16 {
                    pan.weight = 0;
                }
            }
            let skipping = self.state.runtime.get(&25).copied().unwrap_or(0) == 0
                && (control || self.state.runtime.get(&242).copied().unwrap_or(0) != 0);
            let (step, interval) = if pan.speed != 0 {
                if self.state.runtime.get(&313).copied().unwrap_or(0) != 0 && skipping {
                    (16, 10)
                } else if (pan.speed.wrapping_sub(8) as i8) < 0 {
                    (1, 17 * u64::from((pan.speed.wrapping_sub(8) as u8) & 127))
                } else {
                    (i32::from(pan.speed) - 8, 17)
                }
            } else {
                (
                    if skipping {
                        16
                    } else {
                        match self.state.settings.prefix(12)?.unwrap_or(0) {
                            3 => 16,
                            2 => 8,
                            1 => 4,
                            _ => 2,
                        }
                    },
                    17,
                )
            };
            ensure!(step >= 0, "DC2 negative pan speed");
            let next = std::array::from_fn(|axis| {
                let at = self.state.resources.background_position[axis];
                if at < pan.target[axis] {
                    at.saturating_add(step).min(pan.target[axis])
                } else {
                    at.saturating_sub(step).max(pan.target[axis])
                }
            });
            pan.pending = Some((next, pan.weight));
            pan.deadline = Some(
                now.checked_add(interval)
                    .context("DC2 pan timer overflow")?,
            );
            self.state.pan = Some(pan.clone());
        }
        if self.state.resources.capture_owner.is_none() {
            if !host.bitmap_stage(&Request::Capture(pan.owner.clone()))? {
                return Ok(Some(Outcome::Pending));
            }
            self.state.resources.capture_owner = Some(pan.owner.clone());
        }
        let (position, weight) = pan.pending.context("DC2 pan pending frame")?;
        if !host.bitmap_stage(&Request::Pan {
            owner: pan.owner.clone(),
            slot: 1,
            position,
            source_weight: weight,
            finalize: weight == 0 && position == pan.target,
        })? {
            return Ok(Some(Outcome::Pending));
        }
        self.state.resources.background_position = position;
        pan.final_frame = weight == 0 && position == pan.target;
        pan.pending = None;
        let deadline = pan.deadline.context("DC2 pan frame deadline")?;
        self.state.pan = Some(pan);
        //Every completed17ms frame is presentable; Pending only covers the
        //actual worker receipt, never the complete native animation lifetime.
        Ok(Some(Outcome::WaitUntil(deadline)))
    }
}

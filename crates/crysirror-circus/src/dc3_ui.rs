//! DC3WY builder declarations, recovered from414A00/414630/410Bxx.
//! Text rasterization and native markup use CrySirror's existing text host;
//! the VM only retains source-owned records and consumes inline operands.
use super::*;
use crate::machine::{GroupMotion, TextPart};
impl Machine {
    pub(super) fn execute_ui(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 8],
    ) -> Result<Option<u32>> {
        let mut next = instruction.next;
        match instruction.opcode {
            0x32..=0x34 => {
                let flag = match instruction.opcode {
                    0x32 => {
                        if a[0] == 0 {
                            209
                        } else {
                            210
                        }
                    }
                    0x33 => 212,
                    _ => 213,
                };
                self.state.builder.pending_flags.insert(flag);
                if let Some(button) = &mut self.state.builder.pending_button {
                    button.behavior_flags.insert(flag);
                }
            }
            0x6c => {
                let literal = program
                    .instruction(next)
                    .context("DC3WY associated text literal")?;
                let Arguments::String(text) = &literal.arguments else {
                    bail!("DC3WY associated text operand")
                };
                ensure!(
                    (0x42..=0x5f).contains(&literal.opcode),
                    "DC3WY associated text requires a plain literal"
                );
                let button = self
                    .state
                    .builder
                    .pending_button
                    .as_mut()
                    .context("DC3WY text without current button")?;
                ensure!(
                    button.texts.len() < 128,
                    "DC3WY button text declaration limit"
                );
                //4308B0 creates the normal font at22px;414A00 passes no
                //alternate font selection. Encoded colour>128 adds outline.
                button.texts.push(TextPart {
                    position: [a[0], a[1]],
                    text: text.clone(),
                    color: a[2],
                    size: 22,
                    line_pitch: None,
                    state: a[3],
                });
                next = literal.next;
            }
            0x7d if a[2] == 0 => {} // native explicit branch suppression
            0x7d => {
                let literal = program
                    .instruction(next)
                    .context("DC3WY standalone text literal")?;
                let Arguments::String(text) = &literal.arguments else {
                    bail!("DC3WY standalone text operand")
                };
                ensure!(
                    (0x60..=0x63).contains(&literal.opcode),
                    "DC3WY standalone text requires its shifted literal"
                );
                ensure!(
                    self.state.builder.group.texts.len() < 4096,
                    "DC3WY standalone text declaration limit"
                );
                self.state.builder.group.texts.push(TextPart {
                    position: [a[0], a[1]],
                    text: text.clone(),
                    color: a[3],
                    size: 22,
                    line_pitch: Some(a[2]),
                    state: 99,
                });
                next = literal.next;
            }
            0x6e => {
                //410D70 sets the motion origin224/225. Equal endpoints
                //leave an existing native velocity flag868 untouched.
                ensure!(
                    a[..2] != a[2..4] || self.state.builder.group.motion.is_none(),
                    "DC3WY retained group velocity at equal endpoints requires its live sequence receipt"
                );
                self.state.builder.group.position = [a[0], a[1]];
                self.state.builder.group.motion = if a[..2] != a[2..4] {
                    Some(GroupMotion {
                        from: [a[0], a[1]],
                        to: [a[2], a[3]],
                        duration_ticks: 20,
                    })
                } else {
                    None
                };
            }
            0x7e => self.state.builder.offset = [a[0], a[1]], //410B80
            0x71 => {
                if a[1] <= 256 {
                    ensure!(
                        self.state.last_opcode != Some(0x7d),
                        "DC3WY standalone text reveal policy requires its text timeline receipt"
                    );
                    //43B790's low-nibble switch admits0(horizontal) and
                    //1(vertical) only; other kinds return without enqueuing.
                    if !matches!(a[0] & 15, 0 | 1) {
                        return Ok(Some(next));
                    }
                } else if a[1] != 768 {
                    return Ok(Some(next));
                }
                if let Some(index) = self.state.builder.group.images.len().checked_sub(1) {
                    //415800/415520 clone and replace only the last standalone
                    //image. Queue evaluation/UVs/fades are shared engine work.
                    self.state
                        .builder
                        .group
                        .image_animations
                        .push([a[0], a[1], a[2], a[3]]);
                    self.state.builder.group.image_animation_targets.push(index);
                    self.state.builder.group.image_animation_owners.push(
                        crate::machine::UiRecordTarget::Image {
                            button: None,
                            index,
                        },
                    );
                }
            }
            _ => return Ok(None),
        }
        Ok(Some(next))
    }
}

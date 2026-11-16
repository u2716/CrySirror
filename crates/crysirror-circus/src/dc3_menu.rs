//! 4233A0 cases 0x20/0x0A and process 42A160 case 10: the live builder
//! (this+10228) as a menu.
//!
//! 0x20 calls 414390 (seal the pending button into the live list); a nonzero
//! operand additionally runs 42B6C0/42CFC0/4112B0, not connected here.
//! 0x0A calls 414390 and 42CFC0(this+10244), sets last opcode 10 and yields.
//! Process case 10 then calls 4130B0(live builder) every frame (hit test,
//! hover/press sounds and paints: the shared menu service) and, once it
//! returns a button whose action string is nonempty, 41B350 -> 42F660 moves
//! the decoder to the 0x44 label with that name (an absent label leaves the
//! decoder after 0x0A) and dispatch resumes.
//! Known gap: 42CFC0 finalizes the scroll panel this+10244. DC3 panel
//! declarations are not modelled; producers that declare one are not
//! connected.
use super::*;

///The choice builder this+10236 (0x08/0x54/0x09).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChoiceState {
    pub builder: Builder,
    pub enabled: Vec<bool>,
    pub rows: usize,
}

impl Machine {
    pub(super) fn execute_menu(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
    ) -> Result<Option<Outcome>> {
        match instruction.opcode {
            0x20 => {
                ensure!(a[0] == 0, "DC3WY case 0x20 activation (42B6C0/4112B0) is not connected");
                self.state.builder.seal();
                self.advance(instruction.next, instruction.opcode).map(Some)
            }
            //410E10(1): builder field 234. 4130B0 then rescales a button's
            //images on a state change: 100 -> 120 over 15 ticks entering
            //state 1/2, 120 -> 100 otherwise (43B520). That is the shared
            //menu's group flag 242 (DC4PH 412BE0). Difference: native also
            //restarts on a 1<->2 change; the shared flag only on 99<->other.
            0x31 => {
                //4233A0 passes the constant 1 and ignores the operand.
                self.state.builder.group.behavior_flags.insert(242);
                self.advance(instruction.next, instruction.opcode).map(Some)
            }
            0x0a => {
                let mut builder = self.state.builder.clone();
                builder.seal();
                if !host.emit(&Event::BeginMenu(builder.group.clone()))? {
                    return Ok(Some(Outcome::Pending));
                }
                self.state.builder = builder;
                self.state.menu_wait = Some(instruction.offset);
                self.advance(instruction.next, instruction.opcode)?;
                Ok(Some(Outcome::WaitForInput))
            }
            //0x08: Buf42792/23576 = 1; 414E60 copies template this+10292
            //(Rust slot 11, from 0x18 3) into the choice builder this+10236
            //and resets the row counter this+13440.
            0x08 => {
                let template = self.state.templates.get(&11).cloned().context("DC3WY choice template this+10292 is absent")?;
                let rows = template.group.buttons.len();
                self.state.choice = Some(ChoiceState { builder: template, enabled: vec![false; rows], rows: 0 });
                self.advance(instruction.next, instruction.opcode).map(Some)
            }
            //0x54 label + following literal: 412EA0 gives the button with
            //ordinal 13440 that action and enables it (+620), and sets the
            //text of each of its text records; then 13440 increments.
            0x54 => {
                let label = match &instruction.arguments {
                    Arguments::String(label) => label.clone(),
                    _ => bail!("DC3WY choice label operand"),
                };
                let literal = program.instruction(instruction.next).context("DC3WY choice text literal")?;
                let Arguments::String(row_text) = &literal.arguments else { bail!("DC3WY choice text operand") };
                let choice = self.state.choice.as_mut().context("DC3WY 0x54 without 0x08")?;
                let index = choice.rows;
                if let Some(button) = choice.builder.group.buttons.get_mut(index) {
                    button.action = label;
                    for part in &mut button.texts {
                        part.text = row_text.clone();
                    }
                    choice.enabled[index] = true;
                }
                choice.rows += 1;
                self.advance(literal.next, instruction.opcode).map(Some)
            }
            //0x09 0: the message stays; 415AE0 activates the choice builder,
            //Buf23571 = 1, last opcode 6 and a yield. Process case 6 then
            //waits on the choice (41E740) and jumps to its label (41B350).
            0x09 if a[0] == 0 => {
                let choice = self.state.choice.as_ref().context("DC3WY 0x09 without 0x08")?;
                let mut group = choice.builder.group.clone();
                group.buttons = group
                    .buttons
                    .into_iter()
                    .zip(&choice.enabled)
                    .filter(|(_, enabled)| **enabled)
                    .map(|(button, _)| button)
                    .collect();
                if !host.emit(&Event::BeginMenu(group))? {
                    return Ok(Some(Outcome::Pending));
                }
                self.state.menu_wait = Some(instruction.offset);
                self.state.choice_wait = true;
                self.advance(instruction.next, instruction.opcode)?;
                Ok(Some(Outcome::WaitForInput))
            }
            _ => Ok(None),
        }
    }

    pub(super) fn poll_menu(&mut self, program: &Program, host: &mut dyn NativeHost) -> Result<Option<Outcome>> {
        let action = host.menu_action()?;
        if !host.callbacks_ready() {
            return Ok(Some(Outcome::Pending));
        }
        let Some(action) = action.filter(|action| !action.is_empty()) else {
            return Ok(Some(Outcome::WaitForInput));
        };
        if let Some(target) = program
            .instructions
            .iter()
            .take_while(|item| item.opcode != 0)
            .find(|item| item.opcode == 0x44 && matches!(&item.arguments, Arguments::String(label) if *label == action))
        {
            self.state.pc = target.offset;
        }
        self.state.menu_wait = None;
        if std::mem::take(&mut self.state.choice_wait) {
            //The chosen label leaves the choice: clear its UI owner.
            if !host.emit(&Event::ClearUi)? {
                return Ok(Some(Outcome::Pending));
            }
            self.state.choice = None;
        }
        Ok(None)
    }
}

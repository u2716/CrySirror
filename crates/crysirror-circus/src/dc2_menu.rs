//! Original process10 ordinary buttons (41C530/410980/411470/4239B0) and
//! the process9 text choice (41C530 cases 8/68/9). The VM retains action
//! bytes and source ordinals; shared Ui owns pointer hit testing, focus,
//! press and activation. Unsupported control families remain explicit
//! rather than being projected through the DC4 menu VM.
use super::*;
use super::text::{Glyph, TextFrame};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct MenuWait {
    pub owner: crate::dc2_bitmap::Owner,
    pub rows: Vec<(usize, ui::Button)>,
    pub base: Vec<crate::dc2_bitmap::Draw>,
    ///40FF00: resource bit 0x80 blits through 40C360 with 52B3B8 when
    ///52B3C0 is set; None draws those images opaquely.
    #[serde(default)]
    pub weight: Option<u8>,
    ///Process9 keeps the message window and its last 4224F0 text.
    #[serde(default)]
    pub text: Option<TextFrame>,
    ///41AD40 cells for text records: (row, record, cells).
    #[serde(default)]
    pub glyphs: Vec<(usize, usize, Vec<Glyph>)>,
}
///41C530 case8 with CEBA4C=1. `base` is CEBA48 (the AFE800 window rows);
///`rows[0]` is the AC7058 record copied by case8 itself.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Choice {
    pub base: usize,
    pub rows: Vec<ui::Button>,
}
impl Machine {
    pub(super) fn execute_menu(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
    ) -> Result<Option<Outcome>> {
        match instruction.opcode {
            0x08 => return self.begin_choice(instruction, a).map(Some),
            0x44 => return self.choice_row(program, instruction).map(Some),
            0x09 => return self.wait_choice(instruction, host),
            0x0a => {}
            _ => return Ok(None),
        }
        ensure!(
            a[0] == 0 && a[1] == 0,
            "DC2 timed/animated process10 requires its native timeline"
        );
        if let Some(menu) = &self.state.menu {
            ensure!(
                menu.owner.scene == self.state.scene && menu.owner.pc == instruction.offset,
                "DC2 retained menu instruction owner"
            );
            return Ok(Some(Outcome::Pending));
        }
        ensure!(
            self.state.draw.planes.iter().all(Vec::is_empty),
            "DC2 process10 needs its preceding display composition receipt"
        );
        let buttons = self.state.builder.buttons[..self.state.builder.count].to_vec();
        let Some(rows) = self.admit_rows(buttons.into_iter().enumerate(), host)? else {
            return Ok(Some(Outcome::Pending));
        };
        ensure!(
            !rows.is_empty(),
            "DC2 process10 has no admitted native controls"
        );
        //41C530 case10 calls41B3C0(0,0): the composed native0 is redrawn at
        //once. A screen captured for a not-yet-run transition is abandoned.
        if let Some(owner) = self.state.resources.capture_owner.clone() {
            if !host.bitmap_stage(&crate::dc2_bitmap::Request::Pan {
                owner,
                slot: 1,
                position: self.state.resources.background_position,
                source_weight: 16,
                finalize: true,
            })? {
                return Ok(Some(Outcome::Pending));
            }
            self.state.resources.capture_owner = None;
            self.state.resources.transition = None;
        }
        self.state.menu = Some(MenuWait {
            owner: crate::dc2_bitmap::Owner {
                scene: self.state.scene.clone(),
                pc: instruction.offset,
            },
            rows,
            base: self.state.resources.display_base.clone(),
            weight: None,
            text: None,
            glyphs: Vec::new(),
        });
        Ok(Some(Outcome::Pending))
    }
    ///410980/411470 admission: kind131 records are inert, flag bindings
    ///hide rows, and only static state0..4/9 paints are mapped.
    fn admit_rows(
        &mut self,
        buttons: impl Iterator<Item = (usize, ui::Button)>,
        host: &mut dyn NativeHost,
    ) -> Result<Option<Vec<(usize, ui::Button)>>> {
        let mut rows = Vec::new();
        for (ordinal, button) in buttons {
            if button.kind == 131 {
                continue;
            }
            ensure!(
                button.kind == 0,
                "DC2 process10 native control kind {} is not mapped",
                button.kind
            );
            let Some(shown) = self.display_binding(button.check, host)? else {
                return Ok(None);
            };
            if !shown {
                continue;
            }
            ensure!(
                button
                    .records
                    .iter()
                    .all(|record| matches!(record.state, 0..=4 | 9)),
                "DC2 animated/variable button image needs its native timeline"
            );
            rows.push((ordinal, button));
        }
        Ok(Some(rows))
    }
    ///41C530 case8: off_5016B0=AFE800, A5E0E4=CEBA48, CEBA4C=operand0;
    ///operand1 copies AC7058[0] after the window rows.
    fn begin_choice(&mut self, instruction: &Instruction, a: [i16; 4]) -> Result<Outcome> {
        ensure!(
            a[0] == 1,
            "DC2 0x08 choice mode {} (case68 412530 plain rows, 420090 redraw) is not connected",
            a[0]
        );
        let template = self
            .state
            .templates
            .get(&3)
            .context("DC2 text choice before its AC7058 template (0x17 selector3)")?;
        ensure!(template.count > 0, "DC2 empty AC7058 choice template");
        let frame = template.buttons[0].clone();
        let base = self.state.templates.get(&5).map_or(0, |window| window.count);
        self.state.choice = Some(Choice { base, rows: vec![frame] });
        self.advance(instruction.next, instruction.opcode)
    }
    ///41C530 case68 with CEBA4C=1: AC7058[k] is copied, every text image
    ///receives the choice text (AC6D2C) and width 28*len/2+1, and the label
    ///literal replaces the action at +836. The text literal follows.
    fn choice_row(&mut self, program: &Program, instruction: &Instruction) -> Result<Outcome> {
        let Arguments::String(label) = &instruction.arguments else {
            bail!("DC2 choice label literal layout")
        };
        let raw_label = program
            .literal_bytes(instruction.offset)
            .context("DC2 choice label bytes")?
            .to_vec();
        ensure!(raw_label.len() < 40, "DC2 choice label exceeds its native field");
        let literal = program.instruction(instruction.next)?;
        let Arguments::String(text) = &literal.arguments else {
            bail!("DC2 choice text literal layout")
        };
        let raw = program
            .literal_bytes(literal.offset)
            .context("DC2 choice text bytes")?;
        ensure!(raw.len() < 80, "DC2 choice text exceeds AC6D2C");
        ensure!(!text.contains('$'), "DC2 41B710 variable expansion in choices is not connected");
        let template = self.state.templates.get(&3).context("DC2 AC7058 choice template")?;
        let choice = self
            .state
            .choice
            .as_mut()
            .context("DC2 0x44 outside a 0x08 choice (case68 plain rows) is not connected")?;
        let k = choice.rows.len();
        ensure!(
            k < template.count,
            "DC2 choice row {k} exceeds the declared AC7058 template"
        );
        let mut row = template.buttons[k].clone();
        let width = i16::try_from(28 * raw.len() / 2 + 1)?;
        let mut text_image = false;
        for record in &mut row.records {
            if record.text.is_some() {
                text_image = true;
                record.text = Some(text.clone());
                record.source[2] = width;
            }
        }
        ensure!(
            text_image,
            "DC2 choice template row without text images (case68 876 pointer) is not connected"
        );
        row.action = label.clone();
        row.raw_action = raw_label;
        choice.rows.push(row);
        self.advance(literal.next, instruction.opcode)
    }
    ///41C530 case9: B0AB34=9, AC5A58=A5E0E4, 4224F0(-1) keeps the message;
    ///4239B0 then jumps to the selected record's label (419E80).
    fn wait_choice(
        &mut self,
        instruction: &Instruction,
        host: &mut dyn NativeHost,
    ) -> Result<Option<Outcome>> {
        if let Some(menu) = &self.state.menu {
            ensure!(
                menu.owner.scene == self.state.scene && menu.owner.pc == instruction.offset,
                "DC2 retained choice instruction owner"
            );
            return Ok(Some(Outcome::Pending));
        }
        let choice = self
            .state
            .choice
            .clone()
            .context("DC2 0x09 without a 0x08 text choice")?;
        ensure!(
            self.state.draw.planes.iter().all(Vec::is_empty),
            "DC2 process9 needs its preceding display composition receipt"
        );
        ensure!(
            self.state.resources.capture_owner.is_none(),
            "DC2 process9 over an unfinished screen transition"
        );
        let Some(rows) = self.admit_rows(
            choice.rows.into_iter().enumerate().map(|(k, row)| (choice.base + k, row)),
            host,
        )?
        else {
            return Ok(Some(Outcome::Pending));
        };
        ensure!(!rows.is_empty(), "DC2 process9 has no admitted choice rows");
        //422CB0: 41A300(3,30) selects 16 (14 past 20 bytes), (3,31) 16.
        let Some(small) = self.value(3, 30, host)? else {
            return Ok(Some(Outcome::Pending));
        };
        let Some(compact) = self.value(3, 31, host)? else {
            return Ok(Some(Outcome::Pending));
        };
        let mut glyphs = Vec::new();
        for (index, (_, row)) in rows.iter().enumerate() {
            for (slot, record) in row.records.iter().enumerate() {
                let Some(text) = &record.text else { continue };
                ensure!(
                    record.resource & 0x40 == 0 && record.resource as u16 & 0x8000 == 0,
                    "DC2 boxed/redrawn choice text style {} is not connected",
                    record.resource
                );
                //421910: '@' size prefixes, 0x816F/0x8170 ruby brackets and
                //newlines change the cell walk; none is reproduced here.
                ensure!(
                    !text.contains(['@', '\n', '\u{ff5b}', '\u{ff5d}']),
                    "DC2 choice text size/ruby escapes are not connected"
                );
                let length = text.chars().map(|c| if text::wide(c) { 2 } else { 1 }).sum::<usize>();
                let size: u8 = if small != 0 {
                    if length > 20 { 14 } else { 16 }
                } else if compact != 0 {
                    16
                } else {
                    28
                };
                let mut x = i32::from(record.position[0]) + 1;
                let y = i32::from(record.position[1]) + 1;
                let mut cells = Vec::new();
                for c in text.chars() {
                    cells.push(self.cell(x, y, size, c.to_string(), i32::from(record.resource))?);
                    x += i32::from(if text::wide(c) { size } else { size / 2 });
                }
                glyphs.push((index, slot, cells));
            }
        }
        let weight = if self.state.settings.prefix(10)?.unwrap_or(0) != 0 {
            let weight = self.state.settings.prefix(11)?.unwrap_or(0);
            ensure!((0..=16).contains(&weight), "DC2 native window weight");
            Some(weight as u8)
        } else {
            None
        };
        let text = self.state.text.window_shown.then(|| TextFrame {
            window: self.state.text.window.clone(),
            glyphs: self.state.text.displayed.clone(),
        });
        self.state.menu = Some(MenuWait {
            owner: crate::dc2_bitmap::Owner {
                scene: self.state.scene.clone(),
                pc: instruction.offset,
            },
            rows,
            base: self.state.resources.display_base.clone(),
            weight,
            text,
            glyphs,
        });
        Ok(Some(Outcome::Pending))
    }
    pub(crate) fn activate_menu(&mut self, program: &Program, ordinal: usize) -> Result<()> {
        let menu = self
            .state
            .menu
            .as_ref()
            .context("DC2 native menu activation without owner")?;
        ensure!(
            menu.owner.scene == self.state.scene && menu.owner.pc == self.state.pc,
            "DC2 native menu activation instruction changed"
        );
        let (_, button) = menu
            .rows
            .iter()
            .find(|(source, _)| *source == ordinal)
            .context("DC2 foreign menu ordinal")?;
        ensure!(
            !button.raw_action.is_empty() && !matches!(button.raw_action[0], b':' | b';' | b'*'),
            "DC2 native menu action family is not mapped"
        );
        let target = named_label(program, &button.raw_action)?;
        let instruction = program.instruction(self.state.pc)?;
        self.advance(instruction.next, instruction.opcode)?;
        self.state.pc = target;
        self.state.runtime.insert(252, ordinal as i32);
        self.state.menu = None;
        self.state.choice = None;
        Ok(())
    }
}

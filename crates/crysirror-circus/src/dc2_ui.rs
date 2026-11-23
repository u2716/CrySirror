//! Original DC2 880-byte UI declaration policy (412530/412790/412860).
//! Records remain native metadata; no rendering, glyph raster or hit-test
//! algorithm is implemented in this module. Conversion to shared UI must retain
//! their kind/state/slot distinctions instead of treating them as DC4 buttons.
use super::*;
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Image {
    pub state: i16,
    pub position: [i16; 2],
    pub source: [i16; 4],
    pub resource: i16,
    pub text: Option<String>,
    pub text_color: i16,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Button {
    pub kind: i16,
    pub check: i32,
    pub bounds: [i16; 4],
    pub action: String,
    pub raw_action: Vec<u8>,
    pub records: Vec<Image>,
    pub sounds: [Option<String>; 2],
    pub sound_ids: [u8; 2],
}
impl Default for Button {
    fn default() -> Self {
        Self {
            kind: 0,
            check: -1,
            bounds: [0; 4],
            action: String::new(),
            raw_action: Vec::new(),
            records: vec![Image::default(); 16],
            sounds: Default::default(),
            sound_ids: [255; 2],
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Builder {
    pub buttons: Vec<Button>,
    pub count: usize,
    pub current: Option<usize>,
    pub special: Button,
}
impl Machine {
    pub(super) fn execute_ui(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 4],
    ) -> Result<Option<u32>> {
        let mut next = instruction.next;
        match instruction.opcode {
            0x3d | 0x3e => {
                //Native61/62 retain a script literal pointer at872/868 and
                //replace click/hover numeric SFX IDs with255. Own its decoded
                //path in Rust; actual playback remains a Ui activation receipt.
                if let Some(button) = self.state.builder.current.and_then(|index| self.state.builder.buttons.get_mut(index)) {
                    let Arguments::String(name) = &instruction.arguments else { bail!("DC2 button voice literal layout") };
                    let index = if instruction.opcode == 0x3d { 1 } else { 0 };
                    button.sound_ids[index] = 255;
                    button.sounds[index] = Some(name.clone());
                }
            }
            0x14 => {
                if let Some(button) = self
                    .state
                    .builder
                    .current
                    .and_then(|index| self.state.builder.buttons.get_mut(index))
                {
                    button.sound_ids = [a[1] as u8, a[0] as u8];
                }
            }
            0x17 => {
                let selector = a[0] as u8;
                let count = match selector {
                    1 | 4 => Some(8),
                    2 => Some(4),
                    3 => Some(10),
                    _ => None,
                };
                if let Some(count) = count {
                    let mut template = self.state.builder.clone();
                    template.buttons.truncate(template.count.min(count));
                    template.count = template.buttons.len();
                    if matches!(selector, 1 | 4) {
                        for button in &mut template.buttons {
                            button.kind = 1
                        }
                    }
                    if selector == 3 {
                        if let Some(button) = template.buttons.first_mut() {
                            button.kind = 131
                        }
                    }
                    template.current = None;
                    self.state.templates.insert(selector, template);
                    //Native selector1 copies the same declarations into two independent
                    //owners, main and dialogue. Retain that second source-owned snapshot.
                    if selector == 1 {
                        self.state
                            .templates
                            .insert(5, self.state.templates[&1].clone());
                    }
                }
                self.state.builder.count = 0;
                self.state.runtime.insert(252, -1);
            }
            0x18 => {
                if a[0] == 1 && self.state.runtime.get(&3).copied().unwrap_or(0) == 0 {
                    //Native underflows with no previous button; safe implementation makes
                    //that malformed declaration explicit rather than touching prior memory.
                    let index = self
                        .state
                        .builder
                        .count
                        .checked_sub(1)
                        .context("DC2 disabled-button declaration has no previous record")?;
                    self.state.builder.buttons[index].kind = 131;
                }
            }
            0x19 => self.state.image_flag = 128,
            0x25 => self.state.pending_check = i32::from(a[1]) + (i32::from(a[0]) << 8),
            0x4f => {
                let action = program.instruction(next)?;
                let (text, kind) = if let Arguments::Bytes(bytes) = action.arguments {
                    ensure!(
                        action.opcode == 2,
                        "DC2 button action requires literal or native kind marker"
                    );
                    ("".to_owned(), i16::from(bytes[0]))
                } else {
                    let Arguments::String(text) = &action.arguments else {
                        bail!("DC2 button action literal layout")
                    };
                    let raw = program
                        .literal_bytes(action.offset)
                        .context("DC2 action original bytes")?;
                    //836..875 precedes the native pointer at876. The unchecked original
                    //strcpy could overwrite it; never reproduce this memory corruption.
                    ensure!(
                        raw.len() < 40,
                        "DC2 button action exceeds its native character field"
                    );
                    (text.clone(), if text == "*" { 1 } else { 0 })
                };
                next = action.next;
                if self.state.builder.count < 100 {
                    let button = Button {
                        kind,
                        bounds: a,
                        action: text,
                        raw_action: program.literal_bytes(action.offset).unwrap_or(&[]).to_vec(),
                        check: self.state.pending_check,
                        ..Default::default()
                    };
                    let index = self.state.builder.count;
                    self.state.builder.current = Some(index);
                    if index < self.state.builder.buttons.len() {
                        self.state.builder.buttons[index] = button
                    } else {
                        self.state.builder.buttons.push(button)
                    }
                    self.state.builder.count += 1;
                } else {
                    self.state.builder.current = None
                }
                self.state.pending_check = -1;
                self.state.image_flag = 0;
            }
            0x50 => {
                let extension = program.instruction(next)?;
                let Arguments::Words(extra) = extension.arguments else {
                    bail!("DC2 image requires following four-word operand")
                };
                next = extension.next;
                let state = extra[3];
                let button = if matches!(state, 14 | 15) {
                    Some(&mut self.state.builder.special)
                } else {
                    self.state
                        .builder
                        .current
                        .and_then(|index| self.state.builder.buttons.get_mut(index))
                };
                if let Some(button) = button {
                    if state == 20 {
                        button.kind = 4
                    } else if state == 21 {
                        button.kind = 12
                    }
                    let mut remaining = if state == 24 {
                        4
                    } else if state >= 22 {
                        2
                    } else {
                        1
                    };
                    let mut value = state;
                    for record in &mut button.records {
                        if record.state == 0 {
                            *record = Image {
                                state: value,
                                position: [a[0], a[1]],
                                source: [a[2], a[3], extra[0], extra[1]],
                                resource: extra[2] | self.state.image_flag,
                                ..Default::default()
                            };
                            remaining -= 1;
                            if remaining == 0 {
                                break;
                            }
                            value = 13;
                        }
                    }
                }
                self.state.image_flag = 0;
            }
            0x51 => {
                let literal = program.instruction(next)?;
                let Arguments::String(text) = &literal.arguments else {
                    bail!("DC2 text associated literal layout")
                };
                let raw = program
                    .literal_bytes(literal.offset)
                    .context("DC2 text original CP932")?;
                next = literal.next;
                if let Some(button) = self
                    .state
                    .builder
                    .current
                    .and_then(|index| self.state.builder.buttons.get_mut(index))
                {
                    if let Some(record) = button.records.iter_mut().find(|record| record.state == 0)
                    {
                        *record = Image {
                            state: a[3],
                            position: [a[0], a[1]],
                            source: [
                                0,
                                0,
                                i16::try_from(
                                    28usize
                                        .checked_mul(raw.len())
                                        .context("DC2 text width overflow")?
                                        / 2
                                        + 1,
                                )?,
                                29,
                            ],
                            resource: a[2],
                            text: Some(text.clone()),
                            text_color: a[2],
                        };
                    }
                }
            }
            _ => return Ok(None),
        }
        Ok(Some(next))
    }
}

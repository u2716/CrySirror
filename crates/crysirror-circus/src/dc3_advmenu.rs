//! DC3WY in-game menu round trip.
//!
//! Entry (422050, right button during process case 6, not skipping or in
//! auto mode, value 99/211 == 0): 40D6C0 on objects 10..47, 402B20 marks the
//! current history run read, then 420640("advmenu", 0): 41FA40 and 41FA90
//! render the scene into textures 33 and 34 (43C5B0/41E360(1)/43DE10),
//! 41FC40 flushes the inline events (41F1D0(1)), records every object and
//! copies the live body to the return buffer Buf+171432, the skip/auto
//! states clear, Buf23573 clears, and 420370(name, 1) switches scene without
//! releasing resources (420260(1)), clearing the message/speaker texts.
//! Return: case 0x1B (dc3_load mode 1/2) restores that body.
//! Not connected (fail visibly): value 99/211 == 1 (window hide toggle) and
//! the entry sound (432F20 of the global String buffer).
use super::*;
use crate::machine::resource_ops::{ResourceReply, ResourceRequest};
use crate::machine::text_ops::TextRequest;

impl Machine {
    ///422050's right-button branch while a rendered line waits.
    pub(super) fn right_click_menu(&mut self, host: &mut dyn NativeHost) -> Result<Option<Outcome>> {
        if self.state.menu_entry.is_none() {
            if self.native_skipping() || self.value(99, 13)? != 0 {
                self.state.skip_latches = [0, 0];
                let mut data = self.state.system_data.clone();
                ensure!(data.set_prefix_value(13, 0)?, "DC3WY auto-mode native field");
                self.state.system_data = data;
                return Ok(Some(Outcome::WaitForInput));
            }
            ensure!(self.value(99, 211)? == 0, "DC3WY right-button window hide (value 99/211) is not connected");
            self.begin_system_entry("ADVMENU")?;
        }
        self.continue_system_entry(host)
    }

    ///420640(name, 0) / D.C.III 423630(name, 0): 402B20 marks the current
    ///history run read; continue_system_entry renders the return captures
    ///and switches to the system scene.
    pub(super) fn begin_system_entry(&mut self, scene: &str) -> Result<()> {
        self.mark_current_run_read()?;
        self.state.menu_entry = Some(0);
        self.state.menu_entry_scene = Some(scene.to_owned());
        Ok(())
    }

    pub(super) fn continue_system_entry(&mut self, host: &mut dyn NativeHost) -> Result<Option<Outcome>> {
        while let Some(step) = self.state.menu_entry {
            let done = match step {
                0 => matches!(host.resource(&ResourceRequest::CaptureScreen { slot: 33 })?, Some(ResourceReply::Done)),
                1 => matches!(host.resource(&ResourceRequest::CaptureScreen { slot: 34 })?, Some(ResourceReply::Done)),
                2 => {
                    let objects = self.bound_objects(host)?;
                    let resume = self.state.snapshot.as_ref().map_or(self.state.pc, |body| {
                        u32::from_le_bytes(body[19920 - 17704..19924 - 17704].try_into().unwrap())
                    });
                    self.state.return_body = Some(self.project_body(resume, &objects)?);
                    true
                }
                3 => {
                    if !self.clear_dialogue_controls(host)? {
                        return Ok(Some(Outcome::Pending));
                    }
                    host.text(&TextRequest::ClearDialogue)?.is_some()
                }
                4 => {
                    let scene = self.state.menu_entry_scene.clone().unwrap_or_else(|| "ADVMENU".into());
                    if host.scene(&scene)?.is_none() {
                        return Ok(Some(Outcome::Pending));
                    }
                    self.state.skip_latches = [0, 0];
                    self.state.resources.hold_background = false;
                    self.state.text = Default::default();
                    self.state.text_snapshot = None;
                    self.state.tint = 0;
                    self.state.runtime_values.insert(245, -1);
                    for label in &mut self.state.handlers.labels {
                        label.clear();
                    }
                    self.state.scene = scene;
                    self.state.pc = 3;
                    self.state.last_opcode = None;
                    self.state.menu_entry = None;
                    self.state.menu_entry_scene = None;
                    return Ok(Some(Outcome::WaitUntil(0)));
                }
                _ => unreachable!(),
            };
            if !done {
                return Ok(Some(Outcome::Pending));
            }
            self.state.menu_entry = Some(step + 1);
        }
        Ok(None)
    }

    ///402B20 on the current history run.
    pub(super) fn mark_current_run_read(&mut self) -> Result<()> {
        let history = &self.state.history;
        let entry = history.entries.get(history.current).context("DC3WY history run index")?.clone();
        if entry.scene != -1 && entry.first != 0 {
            ensure!(entry.scene >= 0, "DC3WY history run scene");
            let mut data = self.state.system_data.clone();
            data.mark_read(entry.scene as usize, entry.first, entry.last)?;
            self.state.system_data = data;
        }
        Ok(())
    }

    ///Case 0x1B: restore the return body (dc3_load modes 1/2).
    pub(super) fn execute_return(&mut self, instruction: &Instruction, a: [i16; 8], host: &mut dyn NativeHost) -> Result<Option<Outcome>> {
        if instruction.opcode != 0x1b {
            return Ok(None);
        }
        if self.state.load.is_none() {
            let body = self.state.return_body.clone().context("DC3WY 0x1B without a return body (Buf+171432)")?;
            self.state.load = Some(load::LoadProgress {
                owner: instruction.offset,
                bytes: Some(body),
                phase: 1,
                cursor: 0,
                mode: if a[1] == 0 { 1 } else { 2 },
            });
        }
        self.execute_load(instruction, host)
    }
}

impl Machine {
    ///Return-buffer name of `slot` (Buf+171600+48*slot).
    fn return_name(&self, slot: u8) -> Result<String> {
        let body = self.state.return_body.as_ref().context("DC3WY \"--\" rollback without a return buffer")?;
        let at = 168 + usize::from(slot) * 48;
        let field = &body[at..at + 48];
        let end = field.iter().position(|byte| *byte == 0).context("DC3WY return name")?;
        let (text, _, errors) = encoding_rs::SHIFT_JIS.decode(&field[..end]);
        ensure!(!errors, "DC3WY return name is not CP932");
        Ok(text.into_owned())
    }

    ///Object `object`'s record fields from the return buffer (Buf+191060).
    fn return_record(&mut self, object: u16) -> Result<()> {
        let body = self.state.return_body.clone().context("DC3WY return buffer")?;
        let at = 37332 - 17704 + usize::from(object) * 468;
        let float = |offset: usize| f32::from_bits(u32::from_le_bytes(body[at + offset..at + offset + 4].try_into().unwrap()));
        let sprite = self.state.sprites.entry(object).or_default();
        sprite.position = [float(0) as i16, float(4) as i16];
        sprite.retained_position = sprite.position;
        sprite.offset = [float(12) as i16, float(16) as i16];
        let scale = float(104);
        sprite.scale = (scale != 100.).then_some(scale);
        sprite.pivot = [float(116), float(120)];
        Ok(())
    }

    ///"--" operands of 0x3A/0x43/0x3D/0x3F: re-declare from the return
    ///buffer (the menu redraws the game screen under itself).
    pub(super) fn execute_rollback(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>> {
        if text != "--" || !matches!(instruction.opcode, 0x3a | 0x43 | 0x3d | 0x3f) {
            return Ok(None);
        }
        match instruction.opcode {
            //Slots 1..=22 take the stored name (an empty one releases).
            0x3a if (1..=22).contains(&a[0]) => {
                let name = self.return_name(a[0] as u8)?;
                let name = if name.is_empty() { "-".to_owned() } else { name };
                self.execute_resources(program, instruction, a, &name, host, now)
            }
            //420180, then slot 1 reloads and object 0 takes its record.
            0x43 => {
                if self.state.rollback_step.is_none() {
                    for slot in (2u8..=9).chain([19, 25]) {
                        if self.state.image_names.contains_key(&slot) && !self.publish_texture(host, slot, "-")? {
                            return Ok(Some(Outcome::Pending));
                        }
                    }
                    if !host.emit(&Event::ClearUi)? {
                        return Ok(Some(Outcome::Pending));
                    }
                    for label in &mut self.state.handlers.labels {
                        label.clear();
                    }
                    self.state.rollback_step = Some(1);
                }
                let name = self.return_name(1)?;
                if !name.is_empty() {
                    self.return_record(0)?;
                    self.state.image_names.remove(&1);
                    if !self.publish_object_with(host, 1, 0, &name, false)? {
                        return Ok(Some(Outcome::Pending));
                    }
                }
                self.state.rollback_step = None;
                self.advance(instruction.next, instruction.opcode).map(Some)
            }
            //Portrait k: slot k+2 reloads and object 10(k+1) its record.
            0x3d => {
                ensure!((0..4).contains(&a[0]), "DC3WY portrait index");
                let slot = a[0] as u8 + 2;
                let name = self.return_name(slot)?;
                if !name.is_empty() {
                    let object = 10 * (u16::from(slot) - 1);
                    self.return_record(object)?;
                    self.state.image_names.remove(&slot);
                    if !self.publish_object_with(host, slot, object, &name, false)? {
                        return Ok(Some(Outcome::Pending));
                    }
                }
                self.state.resources.pending.remove(&slot);
                self.advance(instruction.next, instruction.opcode).map(Some)
            }
            0x3f => {
                ensure!((0..4).contains(&a[0]), "DC3WY companion index");
                ensure!(
                    self.return_name(a[0] as u8 + 6)?.is_empty(),
                    "DC3WY 0x3F companion record restore (slots 6..9) is not connected"
                );
                self.advance(instruction.next, instruction.opcode).map(Some)
            }
            _ => Ok(None),
        }
    }
}

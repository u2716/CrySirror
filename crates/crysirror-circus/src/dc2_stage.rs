//! Original424880 feedback timing. PC remains at0D until every engine bitmap
//! job and final copy completes. Rendering math is delegated to shared Image.
use super::*;
use crate::dc2_bitmap::{Owner, Request};
impl Machine {
    pub(super) fn default_composition(
        &mut self,
        host: &mut dyn NativeHost,
        include_buttons: bool,
    ) -> Result<Option<(Vec<crate::dc2_bitmap::Draw>, Vec<crate::dc2_bitmap::Draw>)>> {
        use crate::dc2_bitmap::Draw;
        ensure!(
            self.state.draw.planes[1].is_empty(),
            "DC2 alternate display plane composition is not connected"
        );
        let mut output = Vec::new();
        for record in self.state.draw.planes[0].clone() {
            let Some(visible) = self.display_binding(record.check, host)? else {
                return Ok(None);
            };
            if !visible {
                continue;
            }
            ensure!(
                record.kind != 20,
                "DC2 display text requires its engine glyph composition receipt"
            );
            if let Some(draw) = self.record_draw(&record)? {
                output.push(draw);
            }
        }
        let base = output.clone();
        if !include_buttons {
            return Ok(Some((base, output)));
        }
        for button in self.state.builder.buttons[..self.state.builder.count].to_vec() {
            if button.kind == 131 {
                continue;
            }
            let Some(visible) = self.display_binding(button.check, host)? else {
                return Ok(None);
            };
            if !visible {
                continue;
            }
            ensure!(
                button.kind & 4 == 0,
                "DC2 scalar control composition requires its original value binding"
            );
            for record in button.records.iter().filter(|record| record.state == 9) {
                ensure!(
                    record.text.is_none(),
                    "DC2 button text requires its shared glyph composition receipt"
                );
                let [_, _, width, height] = record.source;
                if width <= 0 || height <= 0 {
                    continue;
                }
                ensure!(
                    record.resource & 128 == 0 || self.state.settings.prefix(10)?.unwrap_or(0) == 0,
                    "DC2 translucent button composition requires its native scalar blend"
                );
                let slot = (record.resource & 127) as u8;
                ensure!(
                    slot != 9 && slot & 64 == 0,
                    "DC2 compound button image needs its original blend policy"
                );
                output.push(Draw::Image {
                    slot: if slot == 0 { 1 } else { slot },
                    source: [
                        i32::from(record.position[0]) - i32::from(record.source[0]),
                        i32::from(record.position[1]) - i32::from(record.source[1]),
                        width.into(),
                        height.into(),
                    ],
                    destination: record.position.map(i32::from),
                });
            }
        }
        Ok(Some((base, output)))
    }
    ///41ADE0/420090 non-text record. Bit 0x80 with 52B3C0 (system.dat +16)
    ///selects the 52B3B8 (+8) /16 weighted blits 40E3C0/40C360/40C0E0.
    pub(super) fn record_draw(&self, record: &draw::Record) -> Result<Option<crate::dc2_bitmap::Draw>> {
        use crate::dc2_bitmap::Draw;
        let [width, height] = record.size;
        if width <= 0 || height <= 0 {
            return Ok(None);
        }
        let kind = record.kind as u16;
        let weight = if kind & 128 != 0 && self.state.settings.prefix(10)?.unwrap_or(0) != 0 {
            let weight = self.state.settings.prefix(11)?.unwrap_or(0);
            ensure!((0..=16).contains(&weight), "DC2 native window weight");
            Some(weight as u8)
        } else {
            None
        };
        let rect = [record.position[0], record.position[1], width, height];
        Ok(Some(if kind & 64 != 0 {
            let color = self
                .state
                .palette
                .get(&((kind & 63) as u8))
                .copied()
                .context("DC2 display palette")?;
            match weight {
                Some(weight) => Draw::SolidBlend { rect, color: [color[0], color[1], color[2]], weight },
                None => Draw::Solid { rect, color: [color[0], color[1], color[2], 255] },
            }
        } else {
            let slot = (kind & 127) as u8;
            ensure!(slot != 9, "DC2 dual-texture display composition is not connected");
            let slot = if slot == 0 { 1 } else { slot };
            let source = [
                record.position[0].wrapping_sub(record.origin[0]),
                record.position[1].wrapping_sub(record.origin[1]),
                width,
                height,
            ];
            match weight {
                Some(weight) => Draw::Blend { slot, source, destination: record.position, weight },
                None => Draw::Image { slot, source, destination: record.position },
            }
        }))
    }
    pub(super) fn display_binding(&mut self, check: i32, host: &mut dyn NativeHost) -> Result<Option<bool>> {
        if check == -1 {
            return Ok(Some(true));
        }
        ensure!(check >= 0, "DC2 negative display variable binding");
        let Some(value) = self.value(((check / 256) & 127) as i16, (check % 256) as i16, host)?
        else {
            return Ok(None);
        };
        Ok(Some(if check & 0x8000 == 0 {
            value != 0
        } else {
            value == 0
        }))
    }
    pub(super) fn execute_stage(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Option<Outcome>> {
        let bake = instruction.opcode == 0x0d && a[0] == 10;
        if instruction.opcode == 0x20 || bake {
            ensure!(
                bake || a[0] == 0,
                "DC2 alternate UI presentation requires its original owner"
            );
            let owner = self
                .state
                .resources
                .capture_owner
                .clone()
                .unwrap_or_else(|| Owner {
                    scene: self.state.scene.clone(),
                    pc: instruction.offset,
                });
            let Some((base, draw)) = self.default_composition(host, !bake)? else {
                return Ok(Some(Outcome::Pending));
            };
            let request = if bake {
                Request::Bake { owner, slot: 1, draw }
            } else {
                Request::Prepare { owner, slot: 1, draw }
            };
            if !host.bitmap_stage(&request)? {
                return Ok(Some(Outcome::Pending));
            }
            if bake {
                //41C530 case13/10:41ADE0 completes before clearing the
                //filename and plane0 count; the composed bitmap remains native0.
                //This neither presents nor enters41F630's asynchronous effects.
                self.state.primary_names[0] = None;
                self.state.resources.native0_composed = true;
                self.state.draw.planes[0].clear();
            } else {
                self.state.draw.planes.iter_mut().for_each(Vec::clear);
            }
            //A bake folds the records into native0, so later redraws (process10
            //UI paint) start from that bitmap, not from slot-relative records.
            self.state.resources.display_base = if bake { Vec::new() } else { base };
            self.state.draw.native_composition_pending = bake;
            return self.advance(instruction.next, instruction.opcode).map(Some);
        }
        let explicit = instruction.opcode == 0x0d && a[0] == 0;
        //Case 76 with AC7050 set runs 41ADE0 then the same 424880 feedback
        //before its message, like a voice body (case 54).
        let voice = (instruction.opcode == 0x4c
            || instruction.opcode == 0x36
                && matches!(&instruction.arguments,Arguments::String(text) if !matches!(text.as_str(),"-"|"--")))
            && (self.state.draw.native_composition_pending
                || self.state.resources.transition.is_some());
        if !explicit && !voice {
            return Ok(None);
        }
        ensure!(
            self.state.draw.planes.iter().all(Vec::is_empty),
            "DC2 composed display declarations need their shared Layer/Text publication before native feedback"
        );
        ensure!(
            self.state.primary_names[1..].iter().all(Option::is_none),
            "DC2 portrait composition before native feedback is not connected"
        );
        ensure!(
            self.state.primary_names[0].is_some() || self.state.resources.native0_composed,
            "DC2 absent-background native diagnostic presentation is not connected"
        );
        let owner = self
            .state
            .resources
            .capture_owner
            .clone()
            .unwrap_or_else(|| Owner {
                scene: self.state.scene.clone(),
                pc: instruction.offset,
            });
        if self.state.resources.capture_owner.is_none() {
            if !host.bitmap_stage(&Request::Capture(owner.clone()))? {
                return Ok(Some(Outcome::Pending));
            }
            self.state.resources.capture_owner = Some(owner.clone());
        }
        if instruction.opcode == 0x4c && !self.clear_text(host)? {
            return Ok(Some(Outcome::Pending));
        }
        self.state.draw.native_composition_pending = false;
        let mut transition = self.state.resources.transition.clone().unwrap_or_default();
        if !transition.mixing {
            if let Some(deadline) = transition.deadline {
                if now < deadline {
                    return Ok(Some(Outcome::WaitUntil(deadline)));
                }
                transition.deadline = None;
            }
        }
        if transition.progress >= 16 && !transition.mixing {
            if !host.bitmap_stage(&Request::Commit { owner, slot: 1 })? {
                return Ok(Some(Outcome::Pending));
            }
            self.state.resources.capture_owner = None;
            self.state.resources.transition = None;
            self.state.draw.native_composition_pending = false;
            if explicit {
                self.advance(instruction.next, instruction.opcode)?;
            }
            //36 calls the same native424880 helper, then resumes its voice
            //body at the unchanged originalPC on the next presentable tick.
            return Ok(Some(Outcome::WaitUntil(now)));
        }
        if !transition.mixing {
            let skipping = self.state.runtime.get(&25).copied().unwrap_or(0) == 0
                && (control || self.state.runtime.get(&242).copied().unwrap_or(0) != 0);
            let speed = self.state.settings.prefix(12)?.unwrap_or(0);
            let (step, weight) = if speed == 3 || skipping {
                (8, 10)
            } else if speed == 2 {
                (4, 8)
            } else if speed == 1 {
                (2, 6)
            } else {
                (1, 3)
            };
            transition.progress = (transition.progress + step).min(16);
            transition.source_weight = weight;
            transition.deadline = Some(now.checked_add(40).context("DC2 bitmap timer overflow")?);
            transition.mixing = true;
        }
        if !host.bitmap_stage(&Request::Mix {
            owner,
            slot: 1,
            source_weight: transition.source_weight,
        })? {
            //Remember the selected native weight even when preferences/input change
            //while the asynchronous immutable image job is completing.
            self.state.resources.transition = Some(transition);
            return Ok(Some(Outcome::Pending));
        }
        transition.mixing = false;
        let deadline = transition
            .deadline
            .context("DC2 mixed frame has no native deadline")?;
        self.state.resources.transition = Some(transition);
        Ok(Some(Outcome::WaitUntil(deadline)))
    }
}

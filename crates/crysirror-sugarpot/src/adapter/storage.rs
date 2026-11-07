// SPDX-License-Identifier: MPL-2.0
//! Native logical state shared by the live setters and original slot codec.
//! No renderer resources or device handles enter these records.
use super::*;
use crate::native_storage::slot::{
    SlotWindow, WindowButton, WindowForeground, WindowFrame, WindowPage,
};

pub(super) fn set_window(
    window: &mut SlotWindow,
    selector: &str,
    values: &[Value],
    image_size: Option<[u32; 2]>,
    blend: i32,
) -> Result<()> {
    let n = |i: usize| values.get(i).map_or(Ok(0), Value::integer);
    let s = |i: usize| -> Result<String> {
        values
            .get(i)
            .map_or(Ok(String::new()), |v| Ok(v.string()?.into()))
    };
    let w = |i: usize| -> Result<i32> { Ok(n(i)? as i16 as i32) };
    let b = |i: usize| -> Result<i32> { Ok(n(i)? as i8 as i32) };
    match selector {
        "size" => {
            window.fields[3] = w(0)?;
            window.fields[4] = w(1)?;
        }
        "pos" => {
            window.fields[1] = w(0)?;
            window.fields[2] = w(1)?;
        }
        "txt" => {
            window.fields[12] = w(0)?;
            window.fields[13] = w(1)?;
            window.fields[14] = b(2)?;
            window.fields[15] = b(3)?;
            window.fields[16] = b(4)?;
            window.fields[17] = b(5)?;
        }
        "cel" => {
            for i in 0..4 {
                window.fields[6 + i] = w(i)?;
            }
            let color = n(4)? as u32;
            //454ED0 really ORs all four native bytes without shifts. Preserve
            //this lossy writer policy, rather than substituting packed RGBA.
            window.fields[10] =
                ((color & 255) | ((color >> 8) & 255) | ((color >> 16) & 255)) as i32;
            window.fields[11] = blend as i8 as i32;
        }
        "frm" => {
            let image = s(0)?;
            window.frame = if image.is_empty() {
                None
            } else {
                Some(WindowFrame {
                    image,
                    position: [w(1)?, w(2)?],
                })
            };
            if let Some(size) = image_size {
                window.fields[3] = window.fields[3].max(size[0] as i16 as i32);
                window.fields[4] = window.fields[4].max(size[1] as i16 as i32);
            }
        }
        "fp" | "pp" => {
            let image = s(0)?;
            if image.is_empty() {
                if selector == "fp" {
                    window.foreground = None;
                } else {
                    window.page = None;
                }
            } else {
                let size = image_size.context("SugarPot cursor sheet not prepared")?;
                ensure!(
                    size[0] > 0 && size[1] >= size[0] && size[1] % size[0] == 0,
                    "SugarPot square cursor sheet cells"
                );
                let last = (size[1] / size[0] - 1) as u8 as i32;
                if selector == "fp" {
                    let position = [w(1)?, w(2)?];
                    window.foreground = Some(WindowForeground {
                        image,
                        fields: [
                            position[0],
                            position[1],
                            last,
                            if n(1)? == -1 || n(2)? == -1 { -1 } else { 0 },
                        ],
                    });
                } else {
                    //446220 PP only installs its image and derives the cell
                    //count; the cursor starts at its zeroed allocation origin.
                    let position = window
                        .page
                        .as_ref()
                        .map_or([0, 0], |p| [p.fields[0], p.fields[1]]);
                    window.page = Some(WindowPage {
                        image,
                        fields: [position[0], position[1], last],
                    });
                }
            }
        }
        "btn" => {
            let image = s(0)?;
            if image.is_empty() {
                window.buttons.clear();
            } else {
                window.buttons.push(WindowButton {
                    image,
                    label: s(1)?,
                    position: [w(2)?, w(3)?],
                    overlay: s(4)?,
                    overlay_position: [w(5)?, w(6)?],
                });
            }
        }
        _ => bail!("SugarPot original window selector {selector}"),
    }
    Ok(())
}

impl SugarPotVm {
    pub(super) fn allocate_sprite_buffers(
        &mut self,
        h: &mut dyn HostServices,
        ids: &[usize],
        dimensions: [i32; 2],
    ) -> Result<bool> {
        ensure!(
            dimensions.iter().all(|n| *n > 0 && *n <= 16384),
            "SugarPot sprite allocation dimensions"
        );
        ensure!(
            (dimensions[0] as u64) * dimensions[1] as u64 * 4 <= 64 * 1024 * 1024,
            "SugarPot sprite allocation byte budget"
        );
        ensure!(
            ids.iter().all(|id| *id < self.sprites.len()),
            "SugarPot sprite allocation ID"
        );
        if let Some(job) = &mut self.allocating {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(e) => bail!("SugarPot sprite buffer allocation: {e}"),
                ImageProcessState::Ready(resource) => {
                    let image = h.publish_image_resource(self.client, self.next_image, resource)?;
                    self.next_image = self
                        .next_image
                        .checked_add(1)
                        .context("SugarPot image ID exhausted")?;
                    self.allocating = None;
                    let mut old = BTreeSet::new();
                    for id in ids {
                        if let Some(image) = self.sprites[*id].scratch.replace(image.clone()) {
                            old.insert(image.path.relative().to_owned());
                        }
                    }
                    // The source is immutable. ALL may share its zeroed lease;
                    // later copy-on-write image jobs create separate results.
                    for resource in old {
                        if !self.sprites.iter().any(|s| {
                            s.scratch
                                .as_ref()
                                .is_some_and(|p| p.path.relative() == resource)
                        }) {
                            h.release_image_resource(self.client, &resource)?;
                        }
                    }
                    return Ok(true);
                }
            }
        }
        if ids.is_empty() {
            return Ok(true);
        }
        self.allocating = Some(h.image_metadata(self.client)?.begin_process(
            ImageInput::Solid {
                size: dimensions.map(|n| n as u32),
                color: [0; 4],
            },
            Arc::<[u8]>::from(b"[]".as_slice()),
        )?);
        Ok(false)
    }
}

impl SugarPotVm {
    pub(super) fn native_slot(&self) -> Result<crate::native_storage::slot::Slot> {
        use crate::native_storage::slot::{
            Call, Slot, SlotBuffer, SlotButton, SlotSprite, StoredValue, Variable,
        };
        let machine = self.machine.as_ref().context("SugarPot slot before boot")?;
        ensure!(
            self.save_enabled,
            "SugarPot original save gate is disabled at this native operation"
        );
        ensure!(
            !self.presentation_pending()
                && self.native_write.is_none()
                && self.saving_globals.is_none()
                && !self.raster.as_ref().is_some_and(raster::Raster::preparing)
                && !self.filter.as_ref().is_some_and(filter::Filter::preparing),
            "SugarPot slot requires completed native operation"
        );
        ensure!(
            machine.state.macro_stack.is_empty()
                && !machine.state.frame.script.starts_with("#macro:"),
            "SugarPot original slot macro continuation not mapped"
        );
        ensure!(
            matches!(self.wait, Some(Wait::Text | Wait::Selection))
                || (matches!(self.wait, Some(Wait::Slots))
                    && (matches!(
                        self.slot_menu_source_wait(),
                        Some(Wait::Text | Wait::Selection)
                    ) || self.slot_menu_source_wait().is_none()))
                || (self.wait.is_none() && self.native_text.line == machine.state.frame.pc + 1),
            "SugarPot original slot wait profile not mapped"
        );
        let frame = &machine.state.frame;
        ensure!(
            self.text_remainder.is_empty(),
            "SugarPot original slot display-page continuation not mapped"
        );
        let (current_text, source, text_offset, waiting) = self.native_text.slot()?;
        let pc = self
            .native_text
            .line
            .checked_sub(1)
            .context("SugarPot saved text PC")?;
        let mut calls = vec![Call::Top];
        for depth in 0..=machine.state.stack.len() {
            for entry in machine.state.loops.iter().filter(|l| l.call_depth == depth) {
                ensure!(entry.macro_depth == 0, "SugarPot saved macro while");
                calls.push(Call::While(i32::try_from(entry.pc + 1)?));
            }
            if let Some(caller) = machine.state.stack.get(depth) {
                let ins = &machine.programs[&caller.script].instructions[caller.pc - 1].source;
                if ins.starts_with("^jsb") {
                    calls.push(Call::Jsb {
                        section: caller.section.clone(),
                        pc: i32::try_from(caller.pc)?,
                    });
                } else {
                    ensure!(ins.starts_with("^gsb"), "SugarPot saved native call kind");
                    calls.push(Call::Gsb {
                        script: caller.native_script.clone(),
                        section: caller.section.clone(),
                        pc: i32::try_from(caller.pc)?,
                        continuation: caller.context.clone(),
                    });
                }
            }
        }
        let mut variables = vec![];
        let mut definitions = vec![];
        for (is_def, bank, order, names) in [
            (
                false,
                &machine.state.variables,
                &machine.state.variable_order,
                &machine.state.list_names,
            ),
            (
                true,
                &machine.state.definitions,
                &machine.state.definition_order,
                &machine.state.definition_list_names,
            ),
        ] {
            ensure!(
                bank.len() == order.len(),
                "SugarPot native bank node order backing"
            );
            for name in order {
                let value = bank
                    .get(name)
                    .context("SugarPot native bank ordered node missing")?;
                let value = match value {
                    Value::List(values) => {
                        let names = names
                            .get(name)
                            .context("SugarPot saved native list names")?;
                        ensure!(
                            names.len() == values.len(),
                            "SugarPot saved native list shape"
                        );
                        let element_name = names
                            .first()
                            .context("SugarPot empty native list cannot be encoded")?
                            .clone();
                        ensure!(
                            names.iter().all(|name| *name == element_name),
                            "SugarPot mixed child-name list cannot be encoded"
                        );
                        StoredValue::List {
                            element_name,
                            elements: values.iter().map(stored_value).collect::<Result<_>>()?,
                        }
                    }
                    value => StoredValue::Scalar(stored_value(value)?),
                };
                let record = Variable {
                    name: name.clone(),
                    value,
                };
                if is_def {
                    definitions.push(record);
                } else {
                    variables.push(record);
                }
            }
        }
        ensure!(
            variables.first().is_some_and(|node| node.name == "reg"),
            "SugarPot original VAR first node is reg"
        );
        let sprites = self
            .sprites
            .iter()
            .map(|s| {
                let mut fields = [0; 8];
                if s.visible {
                    fields[..4].copy_from_slice(&[
                        s.position[0],
                        s.position[1],
                        (s.divisions as i32 - 1) as i8 as i32,
                        s.cell as i8 as i32,
                    ]);
                }
                fields[4..].copy_from_slice(&[
                    s.system as i8 as i32,
                    s.attribute as i8 as i32,
                    s.origin as i8 as i32,
                    s.animation.as_ref().map_or(0, |a| a.selected as u16 as i32),
                ]);
                SlotSprite {
                    image: if s.visible {
                        s.source_name.clone()
                    } else {
                        String::new()
                    },
                    fields,
                }
            })
            .collect();
        let mut buttons = Vec::with_capacity(self.button_capacity);
        for id in 0..self.button_capacity {
            let record = self.buttons.get(&(id as i32));
            let showing = record.is_some_and(|b| b.sprite.visible);
            let label = record.and_then(|b| b.label.as_ref());
            buttons.push(SlotButton {
                image: if showing {
                    record.unwrap().sprite.source_name.clone()
                } else {
                    String::new()
                },
                position: if showing {
                    record.unwrap().sprite.position
                } else {
                    [0; 2]
                },
                text: label.map_or_else(String::new, |t| t.0.clone()),
                text_offset: if showing {
                    label.map_or([0; 2], |t| t.1)
                } else {
                    [0; 2]
                },
            });
        }
        //404890/HEAP_ZERO_MEMORY initialize these controller records to zero;
        //The two raster words come from live native policy; unimplemented active
        //fall/filter commands still fail before mutation.
        let rgb = ((self.clear_color[0] as i32) << 16)
            | ((self.clear_color[1] as i32) << 8)
            | self.clear_color[2] as i32;
        Ok(Slot {
            title: self.memo.clone(),
            memo: self
                .slot_menu
                .as_ref()
                .map_or_else(String::new, |menu| menu.memo.clone()),
            previous_caption: self.last_voice_suffix.clone(),
            script: frame.native_script.clone(),
            section: frame.section.clone(),
            display_caption: frame.context.clone(),
            line: i32::try_from(pc + 1)?,
            text_offset,
            current_text,
            resume_text: source,
            audio: self.native_audio.clone(),
            execution: [
                self.mono,
                if self.windows.is_empty() {
                    -1
                } else {
                    self.current_window as i32
                },
                self.mode,
                waiting,
                //458870 startup/42F7F0 and4561A0/42F740 set60B4=1.
                //Those are its only native stores; the reader deliberately
                //skips the saved word and retains the freshly initialized1.
                1,
                self.native_text.inline_clear,
            ],
            calls,
            variables,
            definitions,
            selections: if waiting == 32 {
                self.selections.clone()
            } else {
                vec![]
            },
            //454ED0 writes Window+20. Visibility is the separate+24 byte
            //and is not present in WND records.440290 initializes+20 to0;
            //preserve its authoritative backing rather than saving visibility.
            windows: self.windows.iter().map(|w| w.native.clone()).collect(),
            sprites,
            buffers: vec![SlotBuffer {
                fields: [
                    self.raster.as_ref().map_or(0, |r| r.state),
                    self.raster.as_ref().map_or(0, |r| r.family),
                    rgb,
                    i32::from(self.read_all),
                    self.right_button,
                    self.fall.as_ref().map_or(0, |f| f.state),
                    self.fall.as_ref().map_or(0, |f| f.family),
                    self.filter.as_ref().map_or(0, |f| f.state),
                ],
                image: self
                    .filter
                    .as_ref()
                    .map_or_else(String::new, |f| f.name.clone()),
                //454ED0 writes5602/5603 (velocity), not the tile's live XY.
                position: self.filter.as_ref().map_or([0; 2], |f| f.velocity),
            }],
            buttons,
            gauges: self.gauges.clone(),
        })
    }
}

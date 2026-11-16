//! DC2 native32-byte display declarations (41C530 cases21/53/60/61).
//! These are retained script records, never a second raster/compositor. The
//! eventual process13 publication must map them to engine-owned Layer/Text.
use crate::{Arguments, Instruction, Program};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Record {
    pub position: [i32; 2],
    pub kind: i16,
    pub check: i32,
    pub origin: [i32; 2],
    pub size: [i32; 2],
    pub text: Option<String>,
    pub raw_text: Vec<u8>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DrawState {
    pub planes: [Vec<Record>; 2],
    pub selected: usize,
    pub offset: [i32; 2],
    pub snapshots: BTreeMap<u8, Vec<Record>>,
    pub native_composition_pending: bool,
    ///420090 updates these source-owned fields from type20 records. Keeping
    ///absence explicit avoids inventing a generation's default text geometry.
    pub text_geometry: [Option<[i32; 4]>; 2],
}
impl DrawState {
    pub fn declare(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 4],
        check: &mut i32,
    ) -> Result<Option<u32>> {
        let mut next = instruction.next;
        match instruction.opcode {
            0x21 => {
                let selector = a[0] as u8;
                let capacity = match selector {
                    //41C530 case33: B09B10/B09C50 hold 10, B09DA0/B09E40 5.
                    1 | 4 => Some(10),
                    3 | 6 | 7 => Some(30),
                    2 | 5 => Some(5),
                    _ => None,
                };
                if let Some(capacity) = capacity {
                    //41B560 always copies plane0, even when the most recent declarations
                    //selected plane1. It also truncates the native plane0 count itself.
                    self.planes[0].truncate(capacity);
                    self.snapshots.insert(selector, self.planes[0].clone());
                }
                self.native_composition_pending = true;
                self.planes[0].clear();
                self.selected = 0;
                if selector == 255 {
                    self.planes[1].clear();
                    self.selected = 1;
                }
                //Selector1's420090 additionally observes native flag bindings before
                //changing text geometry. The owning VM must resolve those values before
                //calling apply_window_geometry; declaration alone is not a pixel ACK.
            }
            0x53 => {
                let extension = program.instruction(next)?;
                let Arguments::Words(extra) = extension.arguments else {
                    bail!("DC2 standalone image following four-word operand")
                };
                next = extension.next;
                if self.planes[self.selected].len() < 60 {
                    let position = [
                        i32::from(a[0]).wrapping_add(self.offset[0]),
                        i32::from(a[1]).wrapping_add(self.offset[1]),
                    ];
                    let (origin, size) = if extra[2] & 0x40 != 0 {
                        (
                            [i32::from(a[2]), i32::from(a[3])],
                            [i32::from(extra[0]), i32::from(extra[1])],
                        )
                    } else {
                        (
                            [
                                position[0].wrapping_sub(i32::from(a[2])),
                                position[1].wrapping_sub(i32::from(a[3])),
                            ],
                            [
                                i32::from(extra[0]) - i32::from(a[2]),
                                i32::from(extra[1]) - i32::from(a[3]),
                            ],
                        )
                    };
                    self.planes[self.selected].push(Record {
                        position,
                        kind: extra[2],
                        check: *check,
                        origin,
                        size,
                        ..Default::default()
                    });
                    *check = -1;
                }
                self.native_composition_pending = true;
            }
            0x60 => {
                let literal = program.instruction(next)?;
                let Arguments::String(text) = &literal.arguments else {
                    bail!("DC2 standalone text following literal")
                };
                next = literal.next;
                if self.planes[self.selected].len() < 60 {
                    let raw = program
                        .literal_bytes(literal.offset)
                        .context("DC2 standalone text native bytes")?;
                    self.planes[self.selected].push(Record {
                        position: [a[0].into(), a[1].into()],
                        kind: 20,
                        check: *check,
                        size: [a[2].into(), a[3].into()],
                        text: Some(text.clone()),
                        raw_text: raw.to_vec(),
                        ..Default::default()
                    });
                    *check = -1;
                }
            }
            0x61 => {
                if a[2] == 0 {
                    self.offset = [a[0].into(), a[1].into()]
                } else {
                    self.offset[0] = self.offset[0].wrapping_add(a[0].into());
                    self.offset[1] = self.offset[1].wrapping_add(a[1].into());
                }
            }
            _ => return Ok(None),
        }
        Ok(Some(next))
    }
    pub fn apply_window_geometry(
        &mut self,
        visible: &[bool],
        alternate: bool,
        native_font_style: bool,
    ) -> Result<()> {
        let selector = if alternate { 4 } else { 1 };
        let records = self
            .snapshots
            .get(&selector)
            .context("DC2 original window template was not declared")?;
        ensure!(
            visible.len() == records.len(),
            "DC2 window-template variable observation extent"
        );
        for (record, visible) in records.iter().zip(visible) {
            if !visible || record.kind != 20 {
                continue;
            }
            let index = usize::from(record.size[0] == 0);
            self.text_geometry[index] = Some([
                record.position[0],
                record.position[1],
                record.size[0],
                if index == 0 && !native_font_style {
                    record.size[1] & 0xffbf
                } else {
                    record.size[1]
                },
            ]);
        }
        Ok(())
    }
}

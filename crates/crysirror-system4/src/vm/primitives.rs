// Copyright (C) 2019 Nunuhara Cabbage; Rust (C) 2026 CrySirror contributors.
// GPL-2.0-or-later. Reviewed xsystem4 a85467d src/vm.c opcode/value semantics.
//! Native scalar/reference/string values and page lifetimes. There is no
//! renderer, text layout, filesystem, timer or device implementation here.
use super::*;
impl Machine {
    pub(super) fn execute_primitives(
        &mut self,
        i: &crate::instruction::Instruction,
    ) -> Result<bool> {
        let a = &i.arguments;
        match i.name {
            "R_EQUALE" | "R_NOTE" => {
                let equal = self.peek(3)? == self.peek(1)? && self.peek(2)? == self.peek(0)?;
                self.consume(4)?;
                self.push(i32::from(equal == (i.name == "R_EQUALE")))?;
            }
            "CALLFUNC2" => {
                let function = usize::try_from(self.peek(1)?)?;
                self.program
                    .functions
                    .get(function)
                    .context("System4 indirect function index")?;
                self.consume(2)?;
                self.call(function, -1, Return::Address(i.next))?;
                return Ok(true);
            }
            "SH_LOCALDELETE" => {
                let page = self.local()?;
                let slot = self.heap.get(page, a[0])?;
                if slot != -1 {
                    self.heap.slot(slot)?;
                    self.schedule(
                        vec![
                            Action::Release(slot),
                            Action::SetVariable {
                                page,
                                index: a[0],
                                value: -1,
                            },
                        ],
                        i.next,
                    );
                    return Ok(true);
                }
            }
            "OBJSWAP" => {
                let (right, ri) = self.reference(1)?;
                let (left, li) = self.reference(3)?;
                let (lv, rv) = (self.heap.get(left, li)?, self.heap.get(right, ri)?);
                self.heap.set(left, li, rv)?;
                self.heap.set(right, ri, lv)?;
                self.consume(5)?;
            }
            "F_PLUSA" | "F_MINUSA" | "F_MULA" | "F_DIVA" => {
                let right = self.peek(0)?;
                let (page, index) = self.reference(1)?;
                let left = self.heap.get(page, index)?;
                let op = match i.name {
                    "F_PLUSA" => "F_ADD",
                    "F_MINUSA" => "F_SUB",
                    "F_MULA" => "F_MUL",
                    _ => "F_DIV",
                };
                let value = calculate(op, left, right)?;
                self.heap.set(page, index, value)?;
                self.consume(3)?;
                self.push(value)?;
            }
            "LI_ASSIGN" => {
                let value = self.peek(0)?.max(0);
                let (page, index) = self.reference(1)?;
                self.heap.set(page, index, value)?;
                self.consume(3)?;
                self.push(value)?;
            }
            "LI_PLUSA" | "LI_MINUSA" | "LI_MULA" | "LI_DIVA" | "LI_MODA" | "LI_ANDA" | "LI_ORA"
            | "LI_XORA" | "LI_LSHIFTA" | "LI_RSHIFTA" => {
                let right = i64::from(self.peek(0)?);
                let (page, index) = self.reference(1)?;
                let left = i64::from(self.heap.get(page, index)?);
                let value = match i.name {
                    "LI_PLUSA" => left + right,
                    "LI_MINUSA" => left - right,
                    "LI_MULA" => left * right,
                    "LI_DIVA" => {
                        if right == 0 {
                            0
                        } else {
                            left / right
                        }
                    }
                    "LI_MODA" => {
                        ensure!(right != 0, "System4 LI_MODA zero divisor");
                        left % right
                    }
                    "LI_ANDA" => left & right,
                    "LI_ORA" => left | right,
                    "LI_XORA" => left ^ right,
                    "LI_LSHIFTA" => {
                        ensure!(
                            (0..64).contains(&right),
                            "System4 undefined native long shift"
                        );
                        left.wrapping_shl(right as u32)
                    }
                    _ => {
                        ensure!(
                            (0..64).contains(&right),
                            "System4 undefined native long shift"
                        );
                        left >> right
                    }
                }
                .clamp(0, i64::from(i32::MAX)) as i32;
                self.heap.set(page, index, value)?;
                self.consume(3)?;
                self.push(value)?;
            }
            "LI_INC" | "LI_DEC" => {
                let (page, index) = self.reference(0)?;
                let value = (i64::from(self.heap.get(page, index)?)
                    + if i.name == "LI_INC" { 1 } else { -1 })
                .clamp(0, i64::from(i32::MAX)) as i32;
                self.heap.set(page, index, value)?;
                self.consume(2)?;
            }
            "S_LT" | "S_GT" | "S_LTE" | "S_GTE" | "S_NOTE" | "S_EQUALE" => {
                let (right, left) = (self.peek(0)?, self.peek(1)?);
                let r = self.heap.string(right)?;
                let l = self.heap.string(left)?;
                // Original strcmp stops at NUL, unlike native string.size.
                let r = r.split(|b| *b == 0).next().unwrap();
                let l = l.split(|b| *b == 0).next().unwrap();
                let value = match i.name {
                    "S_LT" => l < r,
                    "S_GT" => l > r,
                    "S_LTE" => l <= r,
                    "S_GTE" => l >= r,
                    "S_NOTE" => l != r,
                    _ => l == r,
                };
                self.consume(2)?;
                self.schedule(
                    vec![
                        Action::Release(right),
                        Action::Release(left),
                        Action::Push(i32::from(value)),
                    ],
                    i.next,
                );
                return Ok(true);
            }
            "S_LENGTHBYTE" => {
                let (page, index) = self.reference(0)?;
                let len = self.heap.string(self.heap.get(page, index)?)?.len();
                self.consume(2)?;
                self.push(i32::try_from(len)?)?;
            }
            "S_EMPTY" => {
                let slot = self.peek(0)?;
                let value = self.heap.string(slot)?.is_empty();
                self.consume(1)?;
                self.schedule(
                    vec![Action::Release(slot), Action::Push(i32::from(value))],
                    i.next,
                );
                return Ok(true);
            }
            "S_PLUSA" | "S_PLUSA2" => {
                let (right, left) = (self.peek(0)?, self.peek(1)?);
                let mut value = (*self.heap.string(left)?).clone();
                let source = self.heap.string(right)?;
                ensure!(
                    value
                        .len()
                        .checked_add(source.len())
                        .is_some_and(|n| n <= 4_000_000),
                    "System4 string append budget"
                );
                value.extend_from_slice(&source);
                let value = Arc::new(value);
                self.heap.set_string(left, value.clone())?;
                self.consume(2)?;
                // Release right before the new native result slot allocation.
                // String slots cannot run destructors, so this drain is local.
                self.release_string_now(right)?;
                let slot = self.heap.alloc(Object::String(value))?;
                self.push(slot)?;
            }
            "I_STRING" => {
                let value = self.peek(0)?;
                let text = crysirror_text::format::format_one_bytes(
                    b"%d",
                    crysirror_text::format::ByteFormatValue::Integer(value),
                )?
                .bytes;
                let slot = self.heap.alloc(Object::String(Arc::new(text)))?;
                self.consume(1)?;
                self.push(slot)?;
            }
            "A_FREE" => {
                let (page, index) = self.reference(0)?;
                let slot = self.heap.get(page, index)?;
                let array = self.heap.page_opt(slot)?;
                if let Some(array) = array {
                    ensure!(
                        matches!(array.kind, PageKind::Array { .. }),
                        "System4 A_FREE page kind"
                    );
                    let count = array.values.len();
                    self.consume(2)?;
                    self.schedule(
                        vec![
                            Action::DeleteMembers {
                                slot,
                                member: count,
                                call_destructors: true,
                            },
                            Action::ClearPage(slot),
                        ],
                        i.next,
                    );
                    return Ok(true);
                }
                self.consume(2)?;
            }
            "SH_LOCALREFREF" => {
                let page = self.local()?;
                let (x, y) = (self.heap.get(page, a[0])?, self.heap.get(page, a[0] + 1)?);
                self.push(x)?;
                self.push(y)?;
            }
            "SH_LOCALASSIGN_SUB_IMM" => {
                let page = self.local()?;
                let value = self.heap.get(page, a[0])?.wrapping_sub(a[1]);
                self.heap.set(page, a[0], value)?;
            }
            "PAGE_REF" => {
                let page = self.peek(0)?;
                let value = self.heap.get(page, a[0])?;
                self.consume(1)?;
                self.push(value)?;
            }
            "SH_STRUCTREF_GT_IMM" => {
                self.push(i32::from(self.heap.get(self.structure()?, a[0])? > a[1]))?
            }
            "SH_STRUCT_ASSIGN_LOCALREF_ITOB" => {
                let value = i32::from(self.heap.get(self.local()?, a[1])? != 0);
                self.heap.set(self.structure()?, a[0], value)?;
            }
            "SH_LOCAL_ASSIGN_STRUCTREF" => {
                let value = self.heap.get(self.structure()?, a[1])?;
                self.heap.set(self.local()?, a[0], value)?;
            }
            "SH_LOCALSTRUCT_ASSIGN_IMM" => {
                let page = self.heap.get(self.local()?, a[0])?;
                self.heap.set(page, a[1], a[2])?;
            }
            "SH_STRUCTREF2" | "SH_REF_STRUCTREF2" => {
                let page = if i.name == "SH_STRUCTREF2" {
                    self.structure()?
                } else {
                    self.peek(0)?
                };
                let child = self.heap.get(page, a[0])?;
                let value = self.heap.get(child, a[1])?;
                if i.name == "SH_REF_STRUCTREF2" {
                    self.consume(1)?;
                }
                self.push(value)?;
            }
            "SH_STRUCTREF3" => {
                let child = self.heap.get(self.structure()?, a[0])?;
                let nested = self.heap.get(child, a[1])?;
                let value = self.heap.get(nested, a[2])?;
                self.push(value)?;
            }
            "SH_LOC_LT_IMM_OR_LOC_GE_IMM" => {
                let value = self.heap.get(self.local()?, a[0])?;
                self.push(i32::from(value < a[1] || value >= a[2]))?;
            }
            _ => return Ok(false),
        }
        self.pc = i.next;
        Ok(true)
    }
    pub(crate) fn release_string_now(&mut self, slot: i32) -> Result<()> {
        self.heap.string(slot)?;
        let references = self.heap.slot(slot)?.references;
        if references > 1 {
            self.heap.slot_mut(slot)?.references -= 1;
        } else {
            self.heap.free(slot)?;
        }
        Ok(())
    }
}

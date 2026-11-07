// Copyright (C) 2019 Nunuhara Cabbage; Rust (C) 2026 CrySirror contributors.
// GPL-2.0-or-later. xsystem4 a85467d src/vm.c native shorthand operations.
//! Original compact bytecodes use the same pages, lifetimes and continuations
//! as ordinary instructions; no script translation or presentation is embedded.
use super::*;
fn prefix(bytes: &[u8]) -> &[u8] {
    bytes.split(|b| *b == 0).next().unwrap()
}
impl Machine {
    pub(super) fn execute_shorthand(
        &mut self,
        i: &crate::instruction::Instruction,
    ) -> Result<bool> {
        let a = &i.arguments;
        let mut next = i.next;
        match i.name {
            "SH_IF_LOC_LT_IMM" | "SH_IF_LOC_GE_IMM" | "SH_IF_LOC_GT_IMM" | "SH_IF_LOC_NE_IMM" => {
                let v = self.heap.get(self.local()?, a[0])?;
                let take = match i.name {
                    "SH_IF_LOC_LT_IMM" => v < a[1],
                    "SH_IF_LOC_GE_IMM" => v >= a[1],
                    "SH_IF_LOC_GT_IMM" => v > a[1],
                    _ => v != a[1],
                };
                if take {
                    next = u32::try_from(a[2])?;
                }
            }
            "SH_IF_STRUCTREF_GT_IMM" | "SH_IF_STRUCTREF_NE_IMM" | "SH_IF_STRUCTREF_EQ_IMM" => {
                let v = self.heap.get(self.structure()?, a[0])?;
                let take = match i.name {
                    "SH_IF_STRUCTREF_GT_IMM" => v > a[1],
                    "SH_IF_STRUCTREF_NE_IMM" => v != a[1],
                    _ => v == a[1],
                };
                if take {
                    next = u32::try_from(a[2])?;
                }
            }
            "SH_IF_STRUCTREF_NE_LOCALREF" => {
                if self.heap.get(self.structure()?, a[0])? != self.heap.get(self.local()?, a[1])? {
                    next = u32::try_from(a[2])?;
                }
            }
            "SH_IF_STRUCTREF_Z" => {
                if self.heap.get(self.structure()?, a[0])? == 0 {
                    next = u32::try_from(a[1])?;
                }
            }
            "SH_IF_STRUCT_A_NOT_EMPTY" => {
                let array = self.heap.get(self.structure()?, a[0])?;
                if self
                    .heap
                    .page_opt(array)?
                    .is_some_and(|p| !p.values.is_empty())
                {
                    next = u32::try_from(a[1])?;
                }
            }
            "THISCALLMETHOD_NOPARAM"
            | "SH_STRUCTREF_CALLMETHOD_NO_PARAM"
            | "SH_STRUCTREF2_CALLMETHOD_NO_PARAM" => {
                let s = self.structure()?;
                let (target, function) = match i.name {
                    "THISCALLMETHOD_NOPARAM" => (s, a[0]),
                    "SH_STRUCTREF_CALLMETHOD_NO_PARAM" => (self.heap.get(s, a[0])?, a[1]),
                    _ => (self.heap.get(self.heap.get(s, a[0])?, a[1])?, a[2]),
                };
                self.heap.page(target)?;
                self.call(usize::try_from(function)?, target, Return::Address(next))?;
                return Ok(true);
            }
            "SH_LOCREF_ASSIGN_MEM" => {
                let local = self.local()?;
                let (p, v) = (self.heap.get(local, a[0])?, self.heap.get(local, a[0] + 1)?);
                let value = self.heap.get(self.structure()?, a[1])?;
                self.heap.set(p, v, value)?;
            }
            "SH_REF_LOCAL_ASSIGN_STRUCTREF2" => {
                let child = self.heap.get(self.structure()?, a[0])?;
                let value = self.heap.get(child, a[2])?;
                let local = self.local()?;
                self.heap.set(
                    self.heap.get(local, a[1])?,
                    self.heap.get(local, a[1] + 1)?,
                    value,
                )?;
            }
            "SH_GLOBAL_ASSIGN_LOCAL" | "SH_GLOBAL_ASSIGN_IMM" => {
                let value = if i.name == "SH_GLOBAL_ASSIGN_IMM" {
                    a[1]
                } else {
                    self.heap.get(self.local()?, a[1])?
                };
                let old = self.heap.get(0, a[0])?;
                let kind = self.variable(0, a[0])?.kind;
                if old > 0 && (matches!(kind, 12 | 13) || is_array(kind)) {
                    self.schedule(
                        vec![
                            if i.name == "SH_GLOBAL_ASSIGN_IMM" {
                                Action::ExitRelease(old)
                            } else {
                                Action::Release(old)
                            },
                            Action::SetVariable {
                                page: 0,
                                index: a[0],
                                value,
                            },
                        ],
                        next,
                    );
                    return Ok(true);
                }
                self.heap.set(0, a[0], value)?;
            }
            "A_NUMOF_GLOB_1" | "A_NUMOF_STRUCT_1" => {
                let page = if i.name == "A_NUMOF_GLOB_1" {
                    0
                } else {
                    self.structure()?
                };
                let n = self.array_numof(self.heap.get(page, a[0])?, 1)?;
                self.push(n)?;
            }
            "SH_STRUCT_SR_REF" => {
                let slot = self.heap.get(self.structure()?, a[0])?;
                let copy = self.copy_object(slot, 0)?;
                self.push(copy)?;
            }
            "SH_GLOBAL_S_REF" | "SH_LOCAL_S_REF" | "SH_STRUCT_S_REF" | "S_REF2" => {
                let page = match i.name {
                    "SH_GLOBAL_S_REF" => 0,
                    "SH_LOCAL_S_REF" => self.local()?,
                    "SH_STRUCT_S_REF" => self.structure()?,
                    _ => self.peek(0)?,
                };
                let bytes = self.heap.string(self.heap.get(page, a[0])?)?;
                let slot = self.heap.alloc(Object::String(bytes))?;
                if i.name == "S_REF2" {
                    self.consume(1)?;
                }
                self.push(slot)?;
            }
            "SH_SREF_EMPTY" => {
                let (p, v) = self.reference(0)?;
                let empty = self.heap.string(self.heap.get(p, v)?)?.is_empty();
                self.consume(2)?;
                self.push(i32::from(empty))?;
            }
            "SH_LOCALSREF_EMPTY" | "SH_STRUCTSREF_EMPTY" | "SH_GLOBALSREF_EMPTY" => {
                let page = match i.name {
                    "SH_LOCALSREF_EMPTY" => self.local()?,
                    "SH_STRUCTSREF_EMPTY" => self.structure()?,
                    _ => 0,
                };
                let empty = self.heap.string(self.heap.get(page, a[0])?)?.is_empty();
                self.push(i32::from(empty))?;
            }
            "SH_S_ASSIGN_CALLSYS19" => {
                let index = self.peek(0)?;
                let name = usize::try_from(index)
                    .ok()
                    .and_then(|n| self.frames.len().checked_sub(n + 1))
                    .map(|n| self.program.functions[self.frames[n].function].name.clone())
                    .unwrap_or_default();
                self.heap.set_string(self.peek(1)?, Arc::new(name))?;
                self.consume(2)?;
            }
            "SH_LOCAL_APUSHBACK_LOCALSREF"
            | "SH_GLOBAL_APUSHBACK_LOCALSREF"
            | "SH_STRUCT_APUSHBACK_LOCALSREF" => {
                let page = match i.name {
                    "SH_LOCAL_APUSHBACK_LOCALSREF" => self.local()?,
                    "SH_GLOBAL_APUSHBACK_LOCALSREF" => 0,
                    _ => self.structure()?,
                };
                let var = self.variable(page, a[0])?;
                ensure!(
                    var.kind == 16 && var.rank == 1,
                    "System4 string shorthand expected rank1 array"
                );
                let slot = self.heap.get(page, a[0])?;
                let old = self.heap.page_opt(slot)?;
                if let Some(old) = old {
                    ensure!(
                        matches!(
                            old.kind,
                            PageKind::Array {
                                kind: 16,
                                rank: 1,
                                ..
                            }
                        ),
                        "System4 string shorthand page type/rank"
                    );
                }
                let source = self.heap.get(self.local()?, a[1])?;
                let string = self.heap.string(source)?;
                self.heap.check_growth(
                    1,
                    string
                        .len()
                        .div_ceil(4)
                        .checked_add(1)
                        .context("System4 string shorthand growth")?,
                )?;
                let mut page = old.cloned().unwrap_or_else(|| Page {
                    kind: PageKind::Array {
                        kind: 16,
                        structure: -1,
                        rank: 1,
                    },
                    values: Vec::new(),
                });
                let copied = self.heap.alloc(Object::String(string))?;
                page.values.push(copied);
                self.heap.set_page(slot, Some(page))?;
            }
            "SH_S_ASSIGN_REF" => {
                let (p, v) = self.reference(0)?;
                let bytes = self.heap.string(self.heap.get(p, v)?)?;
                let dst = self.peek(2)?;
                self.heap.set_string(dst, bytes)?;
                self.consume(3)?;
            }
            "SH_S_ASSIGN_STR0" => {
                let dst = self.peek(0)?;
                let bytes = self
                    .program
                    .strings
                    .get(usize::try_from(a[0])?)
                    .context("System4 literal string index")?
                    .clone();
                self.heap.set_string(dst, Arc::new(bytes))?;
                self.consume(1)?;
            }
            "SH_SASSIGN_LOCALSREF" | "SH_SASSIGN_STRUCTSREF" | "SH_SASSIGN_GLOBALSREF" => {
                let page = match i.name {
                    "SH_SASSIGN_LOCALSREF" => self.local()?,
                    "SH_SASSIGN_STRUCTSREF" => self.structure()?,
                    _ => 0,
                };
                let bytes = self.heap.string(self.heap.get(page, a[0])?)?;
                self.heap.set_string(self.peek(0)?, bytes)?;
                self.consume(1)?;
            }
            "SH_LOCALREF_SASSIGN_LOCALSREF" | "SH_STRUCTREF_SASSIGN_LOCALSREF" => {
                let page = if i.name == "SH_LOCALREF_SASSIGN_LOCALSREF" {
                    self.local()?
                } else {
                    self.structure()?
                };
                let dst = self.heap.get(page, a[0])?;
                let src = self.heap.get(self.local()?, a[1])?;
                let bytes = self.heap.string(src)?;
                self.heap.set_string(dst, bytes)?;
            }
            "SH_STRUCTSREF_EQ_LOCALSREF" | "SH_STRUCTSREF_NE_LOCALSREF" => {
                let left = self.heap.string(self.heap.get(self.structure()?, a[0])?)?;
                let right = self.heap.string(self.heap.get(self.local()?, a[1])?)?;
                let equal = prefix(&left) == prefix(&right);
                self.push(i32::from(equal == (i.name == "SH_STRUCTSREF_EQ_LOCALSREF")))?;
            }
            "SH_LOCALSREF_EQ_STR0"
            | "SH_LOCALSREF_NE_STR0"
            | "SH_STRUCTSREF_NE_STR0"
            | "SH_GLOBALSREF_NE_STR0" => {
                let page = match i.name {
                    "SH_STRUCTSREF_NE_STR0" => self.structure()?,
                    "SH_GLOBALSREF_NE_STR0" => 0,
                    _ => self.local()?,
                };
                let left = self.heap.string(self.heap.get(page, a[0])?)?;
                let right = self
                    .program
                    .strings
                    .get(usize::try_from(a[1])?)
                    .context("System4 literal string index")?;
                let equal = prefix(&left) == prefix(right);
                self.push(i32::from(equal == (i.name == "SH_LOCALSREF_EQ_STR0")))?;
            }
            "SH_IF_SREF_NE_STR0" => {
                let (p, v) = self.reference(0)?;
                let left = self.heap.string(self.heap.get(p, v)?)?;
                let right = self
                    .program
                    .strings
                    .get(usize::try_from(a[0])?)
                    .context("System4 literal string index")?;
                if prefix(&left) != prefix(right) {
                    next = u32::try_from(a[1])?;
                }
                self.consume(2)?;
            }
            _ => return Ok(false),
        }
        self.pc = next;
        Ok(true)
    }
}

// Copyright (C) 2019 Nunuhara Cabbage; Rust (C) 2026 CrySirror contributors.
// GPL-2.0-or-later. xsystem4 a85467d src/page.c and src/vm.c.
//! Native array allocation retains heap-slot identity and yields through actual
//! constructors/destructors; it contains no engine resource implementation.
use super::*;
impl Machine {
    pub(super) fn execute_array_allocation(
        &mut self,
        i: &crate::instruction::Instruction,
    ) -> Result<()> {
        let rank = usize::try_from(self.peek(0)?)?;
        ensure!(
            rank <= 32 && (rank > 0 || i.name == "A_ALLOC"),
            "System4 array rank budget"
        );
        let (page, index) = self.reference(rank + 1)?;
        let destination = self.heap.get(page, index)?;
        let var = self.variable(page, index)?;
        let kind = match var.kind {
            22..=25 => var.kind - 8,
            32 => 30,
            52 => 50,
            60 => 58,
            67 => 66,
            k if is_array(k) => k,
            _ => bail!("System4 allocation of non-array variable"),
        };
        let old = self.heap.page_opt(destination)?;
        if let Some(old) = old {
            ensure!(
                matches!(old.kind, PageKind::Array { .. }),
                "System4 allocation target not array page"
            );
        }
        let dimensions = (1..=rank)
            .rev()
            .map(|n| self.peek(n))
            .collect::<Result<Vec<_>>>()?;
        ensure!(
            dimensions.iter().all(|n| *n <= heap::MAX_VALUES as i32),
            "System4 array dimension budget"
        );
        let mut actions = Vec::new();
        if i.name == "A_REALLOC" {
            let n = dimensions[0];
            // Native negative realloc dimensions cause undefined allocation;
            // bounded admission fails before consuming the original operands.
            ensure!(n >= 0, "System4 negative realloc dimension");
            if let Some(old) = old {
                let PageKind::Array { rank: old_rank, .. } = old.kind else {
                    unreachable!()
                };
                ensure!(old_rank == rank, "System4 array realloc rank mismatch");
                let old_len = old.values.len();
                if n == 0 {
                    actions.push(Action::DeleteMembers {
                        slot: destination,
                        member: old_len,
                        call_destructors: true,
                    });
                    actions.push(Action::ClearPage(destination));
                    actions.push(Action::Consume(rank + 2));
                } else if n as usize <= old_len {
                    // Unlike delete_page_vars, realloc shrink finalizes the
                    // removed suffix in ascending index order.
                    for index in n as usize..old_len {
                        if owns_heap(self.variable(destination, index as i32)?.kind) {
                            actions.push(Action::Release(old.values[index]));
                        }
                    }
                    actions.push(Action::TruncatePage {
                        slot: destination,
                        len: n as usize,
                    });
                    actions.push(Action::Consume(rank + 2));
                } else {
                    actions.push(Action::Array(ArrayBuild {
                        dimensions: dimensions.clone(),
                        kind,
                        structure: var.structure,
                        destination,
                        next: i.next,
                        levels: vec![old.clone()],
                    }));
                }
            } else if n == 0 {
                actions.push(Action::Consume(rank + 2));
            } else {
                actions.push(Action::Array(ArrayBuild {
                    dimensions: dimensions.clone(),
                    kind,
                    structure: var.structure,
                    destination,
                    next: i.next,
                    levels: vec![Page {
                        kind: PageKind::Array {
                            kind,
                            structure: var.structure,
                            rank,
                        },
                        values: Vec::new(),
                    }],
                }));
            }
        } else {
            if let Some(old) = old {
                actions.push(Action::DeleteMembers {
                    slot: destination,
                    member: old.values.len(),
                    call_destructors: true,
                });
                actions.push(Action::ClearPage(destination));
            }
            if rank == 0 {
                actions.push(Action::ClearPage(destination));
                actions.push(Action::Consume(2));
            } else {
                actions.push(Action::Array(ArrayBuild {
                    dimensions,
                    kind,
                    structure: var.structure,
                    destination,
                    next: i.next,
                    levels: vec![Page {
                        kind: PageKind::Array {
                            kind,
                            structure: var.structure,
                            rank,
                        },
                        values: Vec::new(),
                    }],
                }));
            }
        }
        // The native rank is popped before any destructor/constructor executes.
        self.consume(1)?;
        self.schedule(actions, i.next);
        Ok(())
    }
}
impl Machine {
    pub(super) fn execute_array_values(
        &mut self,
        i: &crate::instruction::Instruction,
    ) -> Result<bool> {
        match i.name {
            "A_FILL" => {
                let (value, count, index) = (self.peek(0)?, self.peek(1)?, self.peek(2)?);
                let (p, v) = self.reference(3)?;
                let slot = self.heap.get(p, v)?;
                let mut actions = Vec::new();
                if let Some(page) = self.heap.page_opt(slot)? {
                    let PageKind::Array { kind, .. } = page.kind else {
                        bail!("System4 fill expected array page")
                    };
                    let length = i32::try_from(page.values.len())?;
                    let mut start = index;
                    let mut count = count;
                    if start < 0 {
                        count = count
                            .checked_add(start)
                            .context("System4 native array fill interval overflow")?;
                        start = 0;
                    }
                    if start >= length {
                        actions.push(Action::Push(0));
                    } else {
                        if start
                            .checked_add(count)
                            .context("System4 native array fill interval overflow")?
                            >= length
                        {
                            count = length - start;
                        }
                        actions.push(Action::ArrayFill {
                            slot,
                            index: start,
                            remaining: count,
                            value,
                            kind: element_kind(kind)?,
                            count,
                        });
                    }
                } else {
                    actions.push(Action::Push(0));
                }
                // Native early returns for null/out-of-range arrays do not
                // finalize the owned fill argument; preserve that real policy.
                self.consume(5)?;
                self.schedule(actions, i.next);
                return Ok(true);
            }
            "A_COPY" => {
                let (count, source_index, source, destination_index) =
                    (self.peek(0)?, self.peek(1)?, self.peek(2)?, self.peek(3)?);
                let (p, v) = self.reference(4)?;
                let destination = self.heap.get(p, v)?;
                let mut actions = Vec::new();
                if count > 0 {
                    let (dst, src) = (self.heap.page(destination)?, self.heap.page(source)?);
                    let (
                        PageKind::Array {
                            kind: dk, rank: 1, ..
                        },
                        PageKind::Array {
                            kind: sk, rank: 1, ..
                        },
                    ) = (&dst.kind, &src.kind)
                    else {
                        bail!("System4 copy requires one-dimensional arrays")
                    };
                    ensure!(dk == sk, "System4 array copy element types differ");
                    ensure!(
                        destination_index >= 0
                            && source_index >= 0
                            && destination_index
                                .checked_add(count)
                                .is_some_and(|n| n as usize <= dst.values.len())
                            && source_index
                                .checked_add(count)
                                .is_some_and(|n| n as usize <= src.values.len()),
                        "System4 native array copy bounds"
                    );
                    actions.push(Action::ArrayCopy {
                        destination,
                        destination_index,
                        source,
                        source_index,
                        remaining: count,
                        kind: element_kind(*dk)?,
                        count,
                    });
                } else {
                    actions.push(Action::Push(count));
                }
                self.consume(6)?;
                self.schedule(actions, i.next);
                return Ok(true);
            }
            "SH_STRUCT_A_PUSHBACK_LOCAL_STRUCT"
            | "SH_GLOBAL_A_PUSHBACK_LOCAL_STRUCT"
            | "SH_LOCAL_A_PUSHBACK_LOCAL_STRUCT" => {
                let p = match i.name {
                    "SH_STRUCT_A_PUSHBACK_LOCAL_STRUCT" => self.structure()?,
                    "SH_GLOBAL_A_PUSHBACK_LOCAL_STRUCT" => 0,
                    _ => self.local()?,
                };
                let var = self.variable(p, i.arguments[0])?;
                ensure!(
                    var.kind == 17,
                    "System4 shorthand pushback requires structure array"
                );
                let slot = self.heap.get(p, i.arguments[0])?;
                let old = self.heap.page_opt(slot)?.cloned();
                if let Some(old) = &old {
                    ensure!(
                        matches!(old.kind, PageKind::Array { rank: 1, .. }),
                        "System4 shorthand insertion requires one-dimensional array"
                    );
                }
                ensure!(
                    old.as_ref().map_or(0, |p| p.values.len()) < heap::MAX_VALUES
                        && self.heap.value_count() < heap::MAX_VALUES,
                    "System4 shorthand array budget"
                );
                let source = self.heap.get(self.local()?, i.arguments[1])?;
                let value = self.copy_object(source, 0)?;
                let mut page = old.unwrap_or_else(|| Page {
                    kind: PageKind::Array {
                        kind: var.kind,
                        structure: var.structure,
                        rank: 1,
                    },
                    values: Vec::new(),
                });
                page.values.push(value);
                self.schedule(
                    vec![Action::SetPage {
                        slot,
                        page: Some(page),
                    }],
                    i.next,
                );
                return Ok(true);
            }
            "A_PUSHBACK" | "A_INSERT" => {
                let value = self.peek(0)?;
                let offset = if i.name == "A_INSERT" { 2 } else { 1 };
                let (p, v) = self.reference(offset)?;
                let slot = self.heap.get(p, v)?;
                let var = self.variable(p, v)?;
                let kind = match var.kind {
                    22..=25 => var.kind - 8,
                    32 => 30,
                    52 => 50,
                    60 => 58,
                    67 => 66,
                    k if is_array(k) => k,
                    _ => bail!("System4 insertion into non-array variable"),
                };
                let old = self.heap.page_opt(slot)?.cloned();
                if let Some(old) = &old {
                    ensure!(
                        matches!(old.kind, PageKind::Array { rank: 1, .. }),
                        "System4 insertion requires one-dimensional array"
                    );
                }
                let old_len = old.as_ref().map_or(0, |p| p.values.len());
                ensure!(
                    old_len < heap::MAX_VALUES && self.heap.value_count() < heap::MAX_VALUES,
                    "System4 array insertion budget"
                );
                let index = if i.name == "A_PUSHBACK" {
                    old_len
                } else {
                    self.peek(1)?.clamp(0, old_len.saturating_sub(1) as i32) as usize
                };
                let placeholder = i.name == "A_PUSHBACK" || old.is_none();
                let mut page = old.unwrap_or_else(|| Page {
                    kind: PageKind::Array {
                        kind,
                        structure: var.structure,
                        rank: 1,
                    },
                    values: Vec::new(),
                });
                // Native pushback calls realloc_array(init_structs=false), then
                // variable_set releases the placeholder. Structure -1 is empty;
                // temporary string/array slots must really be allocated/freed.
                self.consume(if i.name == "A_PUSHBACK" { 3 } else { 4 })?;
                let mut actions = Vec::new();
                if placeholder {
                    let element = element_kind(kind)?;
                    let temporary = self.initial(element)?;
                    if owns_heap(element) && temporary != -1 {
                        actions.push(Action::Release(temporary));
                    }
                }
                page.values.insert(index, value);
                actions.push(Action::SetPage {
                    slot,
                    page: Some(page),
                });
                self.schedule(actions, i.next);
                return Ok(true);
            }
            "A_POPBACK" | "A_ERASE" => {
                let (p, v) = self.reference(if i.name == "A_ERASE" { 1 } else { 0 })?;
                let slot = self.heap.get(p, v)?;
                let old = self.heap.page_opt(slot)?.cloned();
                let index = if i.name == "A_ERASE" {
                    self.peek(0)?
                } else {
                    old.as_ref().map_or(-1, |p| p.values.len() as i32 - 1)
                };
                if let Some(old) = &old {
                    ensure!(
                        matches!(old.kind, PageKind::Array { rank: 1, .. }),
                        "System4 removal requires one-dimensional array"
                    );
                    if i.name == "A_POPBACK" {
                        ensure!(
                            !old.values.is_empty(),
                            "System4 popback from allocated empty array"
                        );
                    }
                }
                let valid = old
                    .as_ref()
                    .is_some_and(|p| index >= 0 && (index as usize) < p.values.len());
                let mut actions = Vec::new();
                if valid {
                    let mut page = old.unwrap();
                    let kind = self.variable(slot, index)?.kind;
                    let removed = page.values.remove(index as usize);
                    if owns_heap(kind) {
                        actions.push(Action::Release(removed));
                    }
                    actions.push(Action::SetPage {
                        slot,
                        page: if page.values.is_empty() {
                            None
                        } else {
                            Some(page)
                        },
                    });
                }
                if i.name == "A_ERASE" {
                    actions.push(Action::Push(i32::from(valid)));
                }
                self.consume(if i.name == "A_ERASE" { 3 } else { 2 })?;
                self.schedule(actions, i.next);
                return Ok(true);
            }
            "A_REVERSE" => {
                let (p, v) = self.reference(0)?;
                let slot = self.heap.get(p, v)?;
                if let Some(old) = self.heap.page_opt(slot)? {
                    ensure!(
                        matches!(old.kind, PageKind::Array { .. }),
                        "System4 reverse expected array"
                    );
                    self.heap.page_mut(slot)?.values.reverse();
                }
                self.consume(2)?;
                self.pc = i.next;
                return Ok(true);
            }
            _ => {}
        }
        Ok(false)
    }
}

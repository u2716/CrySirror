// Native DG_* VM/page policy, xsystem4 a85467d78eb5c7fb6e581d7fe5a6fdaec42b9ef7.
// (C) 2019 Nunuhara Cabbage; Rust (C) 2026 CrySirror contributors.
// GPL-2.0-or-later. Weak VM identities and original callbacks only; no host core.
use super::*;

impl Machine {
    fn delegate_sequence(&self, object: i32) -> i32 {
        self.heap
            .slot(object)
            .map_or(0, |slot| slot.sequence as i32)
    }
    fn delegate_page(&self, slot: i32) -> Result<Option<Page>> {
        let page = self.heap.page_opt(slot)?.cloned();
        if let Some(page) = &page {
            ensure!(
                page.kind == PageKind::Delegate && page.values.len() % 3 == 0,
                "System4 expected bounded delegate triples"
            );
        }
        Ok(page)
    }
    fn delegate_new_method(&self, object: i32, function: i32) -> Page {
        Page {
            kind: PageKind::Delegate,
            values: if function < 1 {
                Vec::new()
            } else {
                vec![object, function, self.delegate_sequence(object)]
            },
        }
    }
    fn delegate_contains(&self, page: &Page, object: i32, function: i32) -> bool {
        let sequence = self.delegate_sequence(object);
        page.values
            .chunks_exact(3)
            .any(|entry| entry == [object, function, sequence])
    }
    fn delegate_append(&self, page: &mut Option<Page>, object: i32, function: i32) -> Result<()> {
        if function < 1 {
            return Ok(());
        }
        if page.is_none() {
            *page = Some(self.delegate_new_method(object, function));
            return Ok(());
        }
        let page = page.as_mut().unwrap();
        if !self.delegate_contains(page, object, function) {
            ensure!(
                page.values.len() <= heap::MAX_VALUES - 3,
                "System4 delegate value budget"
            );
            page.values
                .extend([object, function, self.delegate_sequence(object)]);
        }
        Ok(())
    }
    fn delegate_erase(page: &mut Option<Page>, object: i32, function: i32) {
        if let Some(page) = page {
            if let Some(index) = page
                .values
                .chunks_exact(3)
                .position(|e| e[0] == object && e[1] == function)
            {
                page.values.drain(index * 3..index * 3 + 3);
            }
        }
    }
    fn function_matches_delegate(&self, delegate: usize, function: usize) -> bool {
        let Some(dg) = self.program.delegates.get(delegate) else {
            return false;
        };
        let Some(f) = self.program.functions.get(function) else {
            return false;
        };
        if dg.return_kind != f.return_kind
            || dg.return_structure != f.return_structure
            || dg.arguments != f.arguments
        {
            return false;
        }
        let count = dg.arguments as usize;
        if count > dg.variables.len() || count > f.variables.len() {
            return false;
        }
        dg.variables[..count]
            .iter()
            .zip(&f.variables[..count])
            .all(|(a, b)| (a.kind, a.structure, a.rank) == (b.kind, b.structure, b.rank))
    }
    fn delegate_call(
        &mut self,
        instruction: &crate::instruction::Instruction,
        check: &(dyn Fn() -> Result<()> + Sync),
    ) -> Result<()> {
        let delegate = usize::try_from(instruction.arguments[0])?;
        let definition = self
            .program
            .delegates
            .get(delegate)
            .context("System4 delegate type index")?
            .clone();
        let returns = usize::from(definition.return_kind != 0);
        let slot = self.peek(1 + returns)?;
        let index = usize::try_from(self.peek(returns)?)?;
        let mut page = self.delegate_page(slot)?;
        let mut selected = None;
        if let Some(page) = &mut page {
            while index < page.values.len() / 3 {
                check()?;
                let at = index * 3;
                let entry = &page.values[at..at + 3];
                if self.delegate_sequence(entry[0]) == entry[2] {
                    selected = Some((entry[0], entry[1]));
                    break;
                }
                page.values.drain(at..at + 3);
            }
        }
        if let Some((object, function)) = selected {
            let function = usize::try_from(function)?;
            ensure!(
                self.function_matches_delegate(delegate, function),
                "System4 incompatible delegate callback signature"
            );
            let argc = definition.arguments as usize;
            ensure!(
                argc <= definition.variables.len() && self.stack.len() >= argc + 2 + returns,
                "System4 delegate argument stack"
            );
            ensure!(
                self.frames.len() < MAX_CALLS,
                "System4 delegate call stack budget"
            );
            if object != -1 {
                self.heap.page(object)?;
            }
            let next_index = i32::try_from(
                index
                    .checked_add(1)
                    .context("System4 delegate index overflow")?,
            )?;
            // Retain caller arguments. The local page and its initialized locals
            // precede argument copies in the native heap allocation order.
            self.heap.set_page(slot, page)?;
            if returns != 0 {
                self.consume(1)?;
            }
            *self.stack.last_mut().unwrap() = next_index;
            let local = self.call_frame(function, object, Return::Address(instruction.next))?;
            for (i, variable) in definition.variables.iter().take(argc).enumerate() {
                if i % 256 == 0 {
                    check()?;
                }
                let value = self.peek(argc + 1 - i)?;
                let copied = self.copy_native_value(value, variable.kind)?;
                self.heap.set(local, i as i32, copied)?;
            }
        } else {
            let count = definition.variables.len();
            ensure!(
                self.stack.len() >= count + 2 + returns,
                "System4 delegate cleanup stack"
            );
            let target = u32::try_from(instruction.arguments[1])?;
            self.program.instruction(target)?;
            self.heap.set_page(slot, page)?;
            let returned = if returns != 0 {
                Some(self.peek(0)?)
            } else {
                None
            };
            let base = self.stack.len() - returns - 2 - count;
            let mut actions = Vec::new();
            for index in (0..count).rev() {
                let kind = definition.variables[index].kind;
                let value = self.stack[base + index];
                if owns_heap(kind) && !is_reference(kind) && value != -1 {
                    self.heap.slot(value)?;
                    actions.push(Action::Release(value));
                }
            }
            self.consume(count + 2 + returns)?;
            if let Some(value) = returned {
                actions.push(Action::Push(value));
            }
            self.schedule(actions, target);
        }
        Ok(())
    }
    pub(super) fn execute_delegates(
        &mut self,
        instruction: &crate::instruction::Instruction,
        check: &(dyn Fn() -> Result<()> + Sync),
    ) -> Result<bool> {
        let next = instruction.next;
        match instruction.name {
            "DG_SET" | "DG_ADD" | "DG_EXIST" | "DG_ERASE" => {
                let function = self.peek(0)?;
                let object = self.peek(1)?;
                let slot = self.peek(2)?;
                let mut page = self.delegate_page(slot)?;
                if instruction.name == "DG_EXIST" {
                    let exists = page
                        .as_ref()
                        .is_some_and(|p| self.delegate_contains(p, object, function));
                    self.consume(3)?;
                    self.push(i32::from(exists))?;
                } else {
                    match instruction.name {
                        "DG_SET" => page = Some(self.delegate_new_method(object, function)),
                        "DG_ADD" => self.delegate_append(&mut page, object, function)?,
                        _ => Self::delegate_erase(&mut page, object, function),
                    }
                    self.heap.set_page(slot, page)?;
                    self.consume(3)?;
                }
            }
            "DG_NUMOF" => {
                let slot = self.peek(0)?;
                let mut page = self.delegate_page(slot)?;
                if let Some(page) = &mut page {
                    let mut values = Vec::with_capacity(page.values.len());
                    for (index, entry) in page.values.chunks_exact(3).enumerate() {
                        if index % 256 == 0 {
                            check()?;
                        }
                        if self.delegate_sequence(entry[0]) == entry[2] {
                            values.extend_from_slice(entry);
                        }
                    }
                    page.values = values;
                }
                let count = page.as_ref().map_or(0, |p| p.values.len() / 3);
                self.heap.set_page(slot, page)?;
                self.consume(1)?;
                self.push(i32::try_from(count)?)?;
            }
            "DG_CLEAR" => {
                let slot = self.peek(0)?;
                if slot != 0 {
                    let mut page = self.delegate_page(slot)?;
                    if let Some(p) = &mut page {
                        p.values.clear();
                    }
                    self.heap.set_page(slot, page)?;
                }
                self.consume(1)?;
            }
            "DG_COPY" => {
                let slot = self.peek(0)?;
                self.delegate_page(slot)?;
                let copy = self.copy_object(slot, 0)?;
                self.consume(1)?;
                self.push(copy)?;
            }
            "DG_ASSIGN" | "DG_PLUSA" | "DG_MINUSA" => {
                let source = self.peek(0)?;
                let destination = self.peek(1)?;
                let source_page = self.delegate_page(source)?;
                let mut page = self.delegate_page(destination)?;
                if instruction.name == "DG_ASSIGN" {
                    page = source_page;
                } else if source == destination && instruction.name == "DG_MINUSA" {
                    // The original subtraction walks the live shrinking source
                    // when both handles alias. Preserve its skip-after-erase
                    // order rather than replacing it with set subtraction.
                    let count = page.as_ref().map_or(0, |p| p.values.len() / 3);
                    ensure!(
                        count
                            .checked_mul(count)
                            .is_some_and(|n| n <= 64 * 1024 * 1024),
                        "System4 delegate set comparison budget"
                    );
                    let mut index = 0;
                    while page.as_ref().is_some_and(|p| index < p.values.len() / 3) {
                        check()?;
                        let entry: [i32; 3] = page.as_ref().unwrap().values
                            [index * 3..index * 3 + 3]
                            .try_into()
                            .unwrap();
                        if self.delegate_sequence(entry[0]) == entry[2] {
                            Self::delegate_erase(&mut page, entry[0], entry[1]);
                        }
                        index += 1;
                    }
                } else if let Some(source_page) = source_page {
                    let source_count = source_page.values.len() / 3;
                    let destination_count = page.as_ref().map_or(0, |p| p.values.len() / 3);
                    ensure!(
                        source_count
                            .checked_mul(destination_count.saturating_add(source_count))
                            .is_some_and(|n| n <= 64 * 1024 * 1024),
                        "System4 delegate set comparison budget"
                    );
                    for (index, entry) in source_page.values.chunks_exact(3).enumerate() {
                        if index % 64 == 0 {
                            check()?;
                        }
                        if self.delegate_sequence(entry[0]) != entry[2] {
                            continue;
                        }
                        if instruction.name == "DG_PLUSA" {
                            self.delegate_append(&mut page, entry[0], entry[1])?;
                        } else {
                            Self::delegate_erase(&mut page, entry[0], entry[1]);
                        }
                    }
                }
                self.heap.set_page(destination, page)?;
                self.consume(2)?;
                self.push(source)?;
            }
            "DG_POP" => {
                let slot = self.peek(0)?;
                self.delegate_page(slot)?;
                self.consume(1)?;
                self.schedule(vec![Action::Release(slot)], next);
                return Ok(true);
            }
            "DG_NEW_FROM_METHOD" => {
                let function = self.peek(0)?;
                let object = self.peek(1)?;
                let slot = self.heap.alloc(Object::Page(Some(
                    self.delegate_new_method(object, function),
                )))?;
                self.consume(2)?;
                self.push(slot)?;
            }
            "DG_NEW" => {
                let slot = self
                    .heap
                    .alloc(Object::Page(Some(self.delegate_new_method(-1, 0))))?;
                self.push(slot)?;
            }
            "DG_CALLBEGIN" => {
                let index = usize::try_from(instruction.arguments[0])?;
                let dg = self
                    .program
                    .delegates
                    .get(index)
                    .context("System4 delegate type index")?;
                let argc = dg.arguments as usize;
                let extra = 1 + usize::from(dg.return_kind != 0);
                ensure!(
                    self.stack.len() >= argc + 1 && self.stack.len() + extra <= MAX_STACK,
                    "System4 delegate callbegin stack budget"
                );
                let start = self.stack.len() - argc - 1;
                let slot = self.stack[start];
                self.delegate_page(slot)?;
                self.stack[start..].rotate_left(1);
                self.push(0)?;
                if extra == 2 {
                    self.push(0)?;
                }
            }
            "DG_CALL" => {
                self.delegate_call(instruction, check)?;
                return Ok(true);
            }
            "DG_STR_TO_METHOD" => {
                let dg = self.peek(0)?;
                let string = self.peek(1)?;
                let name = self.heap.string(string)?;
                let name = name.split(|b| *b == 0).next().unwrap();
                let function = self
                    .program
                    .functions
                    .iter()
                    .position(|f| f.name == name)
                    .filter(|f| {
                        usize::try_from(dg)
                            .ok()
                            .is_some_and(|dg| self.function_matches_delegate(dg, *f))
                    })
                    .map_or(0, |f| f as i32);
                self.consume(2)?;
                self.release_string_now(string)?;
                self.push(function)?;
            }
            _ => return Ok(false),
        }
        self.pc = next;
        Ok(true)
    }
}

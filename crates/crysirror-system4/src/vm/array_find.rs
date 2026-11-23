// xsystem4 a85467d vm.c/page.c A_FIND/SH_A_FIND_SREF semantics (GPL2+).
// (C) 2019 Nunuhara Cabbage; Rust (C) 2026 CrySirror contributors.
// Native value ownership and callback continuation, not a hosted C interpreter.
use super::*;
#[derive(Clone, Debug)]
pub(super) struct Find {
    slot: i32,
    sequence: u32,
    kind: PageKind,
    length: usize,
    index: usize,
    end: usize,
    needle: i32,
    release: bool,
    function: usize,
    return_stack: usize,
}
impl Machine {
    pub(super) fn execute_array_find(
        &mut self,
        instruction: &crate::instruction::Instruction,
    ) -> Result<bool> {
        let (needle, function, owned) = match instruction.name {
            "A_FIND" => (self.peek(1)?, usize::try_from(self.peek(0)?)?, true),
            "SH_A_FIND_SREF" => {
                let (page, index) = self.reference(0)?;
                (self.heap.get(page, index)?, 0, false)
            }
            _ => return Ok(false),
        };
        let end = self.peek(2)?;
        let start = self.peek(3)?.max(0) as usize;
        let (page, index) = self.reference(4)?;
        let slot = self.heap.get(page, index)?;
        let Some(array) = self.heap.page_opt(slot)? else {
            self.consume(6)?;
            self.push(-1)?;
            self.pc = instruction.next;
            // Native A_FIND does not release a string needle for a NULL page.
            return Ok(true);
        };
        let PageKind::Array { kind, .. } = array.kind else {
            bail!("System4 A_FIND requires native array page");
        };
        let strings = element_kind(kind)? == 12;
        if strings {
            self.heap.string(needle)?;
        }
        if function != 0 {
            let f = self
                .program
                .functions
                .get(function)
                .context("System4 A_FIND compare function index")?;
            ensure!(
                f.arguments == 2
                    && f.variables.len() >= 2
                    && matches!(f.return_kind, 10 | 11 | 26 | 27 | 47 | 55),
                "System4 A_FIND native comparator ABI"
            );
            ensure!(
                self.frames.len() < MAX_CALLS && self.stack.len() <= MAX_STACK,
                "System4 A_FIND callback budget"
            );
        }
        let find = Find {
            slot,
            sequence: self.heap.slot(slot)?.sequence,
            kind: array.kind.clone(),
            length: array.values.len(),
            index: start,
            end: (end.max(0) as usize).min(array.values.len()),
            needle,
            release: strings && owned,
            function,
            return_stack: self.stack.len() - 6,
        };
        self.consume(6)?;
        self.schedule(vec![Action::ArrayFind(find)], instruction.next);
        Ok(true)
    }
    fn array_find_finish(&mut self, find: Find, result: i32) -> Result<()> {
        // Original result is pushed before the owned string-key unref. That
        // includes allocated empty string arrays; a NULL page follows the
        // distinct early branch above. SH_A_FIND_SREF never owns its needle.
        self.push(result)?;
        if find.release {
            self.release_string_now(find.needle)?;
        }
        Ok(())
    }
    pub(super) fn array_find_step(&mut self, mut find: Find) -> Result<()> {
        if find.index >= find.end {
            return self.array_find_finish(find, -1);
        }
        ensure!(
            self.heap.slot(find.slot)?.sequence == find.sequence,
            "System4 A_FIND callback invalidated original array identity"
        );
        let page = self.heap.page(find.slot)?;
        // The original C retains a page pointer across callbacks. Releasing
        // or reallocating that page is undefined there; fail explicitly here
        // rather than searching an unrelated replacement/reused heap slot.
        ensure!(
            page.kind == find.kind && page.values.len() == find.length,
            "System4 A_FIND callback changed array layout"
        );
        let value = page.values[find.index];
        if find.function != 0 {
            ensure!(
                self.stack.len() + 2 <= MAX_STACK,
                "System4 A_FIND argument budget"
            );
            self.stack.extend([find.needle, value]);
            let function = find.function;
            self.actions.push(Action::ArrayFindAfter(find));
            self.call(function, -1, Return::Actions)?;
            self.running_actions = false;
            return Ok(());
        }
        let PageKind::Array { kind, .. } = find.kind else {
            unreachable!()
        };
        let equal = if element_kind(kind)? == 12 {
            let needle = self.heap.string(find.needle)?;
            let value = self.heap.string(value)?;
            needle.split(|b| *b == 0).next() == value.split(|b| *b == 0).next()
        } else {
            find.needle == value
        };
        if equal {
            let index = i32::try_from(find.index)?;
            self.array_find_finish(find, index)
        } else {
            find.index += 1;
            self.actions.push(Action::ArrayFind(find));
            Ok(())
        }
    }
    pub(super) fn array_find_after(&mut self, mut find: Find) -> Result<()> {
        ensure!(
            self.heap.slot(find.slot)?.sequence == find.sequence,
            "System4 A_FIND callback invalidated original array identity"
        );
        let page = self.heap.page(find.slot)?;
        ensure!(
            page.kind == find.kind && page.values.len() == find.length,
            "System4 A_FIND callback changed array layout"
        );
        ensure!(
            self.stack.len() == find.return_stack + 1,
            "System4 A_FIND comparator did not return one original scalar"
        );
        let matched = self.peek(0)? != 0;
        self.consume(1)?;
        if matched {
            let index = i32::try_from(find.index)?;
            self.array_find_finish(find, index)
        } else {
            find.index += 1;
            self.actions.push(Action::ArrayFind(find));
            Ok(())
        }
    }
}

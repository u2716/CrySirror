// Copyright (C) 2019 Nunuhara Cabbage; Rust (C) 2026 CrySirror contributors.
// GPL-2.0-or-later. xsystem4 a85467d src/savedata.c global/group semantics.
//! Native value graph only. Storage/compression run on the owning save worker.
use super::*;
use crate::native_save::codec::{Array, Named, Record, Save, Value};
pub(crate) struct SaveArguments {
    pub key: Arc<Vec<u8>>,
    pub filename: Arc<Vec<u8>>,
    pub group: Option<Arc<Vec<u8>>>,
    pub output: Option<(i32, i32)>,
    pub loading: bool,
    slots: Vec<i32>,
    words: usize,
}
impl Machine {
    pub(crate) fn global_save_arguments(&self, call: &ExternalCall) -> Result<SaveArguments> {
        self.validate_system(call)?;
        let ExternalTarget::System(code) = call.target else {
            unreachable!()
        };
        ensure!(
            matches!(code, 1 | 2 | 24 | 25),
            "System4 global-save syscall"
        );
        let grouped = matches!(code, 24 | 25);
        let offset = if grouped { 2 } else { 0 };
        let output = if grouped {
            let (page, index) = self.reference(0)?;
            self.heap.get(page, index)?;
            Some((page, index))
        } else {
            None
        };
        let file = self.peek(offset)?;
        let key = self.peek(offset + 1)?;
        let group = if grouped {
            Some(self.heap.string(file)?)
        } else {
            None
        };
        let (filename, key, slots) = if grouped {
            let filename = self.peek(3)?;
            let key = self.peek(4)?;
            (
                self.heap.string(filename)?,
                self.heap.string(key)?,
                vec![file, filename, key],
            )
        } else {
            (
                self.heap.string(file)?,
                self.heap.string(key)?,
                vec![file, key],
            )
        };
        for slot in &slots {
            ensure!(
                self.heap.slot(*slot)?.references as usize
                    >= slots.iter().filter(|s| *s == slot).count(),
                "System4 global-save string ownership"
            );
        }
        ensure!(
            key.len() <= 1024 * 1024 && group.as_ref().is_none_or(|g| g.len() <= 1024 * 1024),
            "System4 global-save key/group budget"
        );
        Ok(SaveArguments {
            key,
            filename,
            group,
            output,
            loading: matches!(code, 2 | 25),
            slots,
            words: if grouped { 5 } else { 2 },
        })
    }
    pub(crate) fn finish_global_save(
        &mut self,
        call: &ExternalCall,
        args: &SaveArguments,
        success: bool,
        saved_count: Option<usize>,
    ) -> Result<()> {
        self.validate_system(call)?;
        if let (Some((page, index)), Some(n)) = (args.output, saved_count) {
            self.heap.get(page, index)?;
            self.heap.set(page, index, i32::try_from(n)?)?;
        }
        self.complete_system(call, args.words, Some(i32::from(success)))?;
        for slot in &args.slots {
            self.release_string_now(*slot)?;
        }
        Ok(())
    }
    pub(crate) fn capture_globals(
        &self,
        args: &SaveArguments,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<Save> {
        let key = cstr(&args.key);
        let group = args.group.as_ref().map(|g| cstr(g));
        let selected = if let Some(name) = group {
            self.program
                .global_groups
                .iter()
                .position(|g| g.as_slice() == name)
        } else {
            None
        };
        let mut capture = Capture {
            machine: self,
            save: Save::new(
                key.to_vec(),
                group.unwrap_or_default().to_vec(),
                i32::try_from(self.program.globals.len())?,
            ),
            nodes: 0,
            check,
        };
        capture.save.records.push(Record {
            kind: 1000,
            name: Vec::new(),
            indices: Vec::new(),
        });
        for (index, var) in self.program.globals.iter().enumerate() {
            check()?;
            if group.is_some() && selected.is_none_or(|g| var.group != Some(g as i32)) {
                continue;
            }
            let value = capture.value(var.kind, self.heap.get(0, index as i32)?, 0)?;
            capture.save.globals.push(Named {
                name: var.name.clone(),
                value,
                unknown: 1,
            });
        }
        capture.save.records[0].indices = (0..capture.save.globals.len())
            .map(i32::try_from)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        capture.save.validate()?;
        Ok(capture.save)
    }
    pub(crate) fn apply_globals(
        &mut self,
        args: &SaveArguments,
        save: &Save,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<()> {
        ensure!(
            save.key == cstr(&args.key),
            "System4 global-save key mismatch"
        );
        ensure!(
            save.group == args.group.as_ref().map(|g| cstr(g)).unwrap_or_default(),
            "System4 global-save group mismatch"
        );
        let mut nodes = 0usize;
        if let Some(indices) = &save.original_global_indices {
            ensure!(
                indices.len() == save.globals.len(),
                "System4 JSON global index extent"
            );
        }
        for (ordinal, global) in save.globals.iter().enumerate() {
            check()?;
            let index = if let Some(indices) = &save.original_global_indices {
                let index = indices[ordinal];
                ensure!(
                    index < self.program.globals.len(),
                    "System4 JSON global index"
                );
                index
            } else {
                self.program
                    .globals
                    .iter()
                    .position(|v| v.name == global.name)
                    .context("System4 global-save unknown global name")?
            };
            let var = self.program.globals[index].clone();
            ensure!(
                var.kind == global.value.kind,
                "System4 global-save global type mismatch"
            );
            let value =
                self.restore_global_value(save, &var, &global.value, &mut nodes, 0, check)?;
            let old = self.heap.get(0, index as i32)?;
            if matches!(var.kind, 12 | 13) || is_array(var.kind) {
                self.discard_global_value(old, check)?;
            }
            self.heap.set(0, index as i32, value)?;
            // Native GroupLoad increments the pointed counter after each assignment;
            // its address can itself name a global being loaded.
            if let Some((page, index)) = args.output {
                let old = self.heap.get(page, index)?;
                self.heap.set(
                    page,
                    index,
                    old.checked_add(1)
                        .context("System4 group-load count overflow")?,
                )?;
            }
        }
        Ok(())
    }
    fn restore_global_value(
        &mut self,
        save: &Save,
        var: &Variable,
        value: &Value,
        nodes: &mut usize,
        depth: usize,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<i32> {
        check()?;
        *nodes = nodes
            .checked_add(1)
            .filter(|n| *n <= 1_000_000)
            .context("System4 global-save restored value budget")?;
        ensure!(
            depth < 64 && var.kind == value.kind,
            "System4 global-save nesting/type"
        );
        match var.kind {
            0 | 10 | 11 | 27 | 47 | 55 => Ok(value.bits),
            12 => self.heap.alloc(Object::String(Arc::new(
                save.strings
                    .get(usize::try_from(value.bits)?)
                    .context("System4 global-save string index")?
                    .clone(),
            ))),
            13 => {
                let definition = self
                    .program
                    .structures
                    .get(usize::try_from(var.structure)?)
                    .context("System4 global-save structure index")?
                    .clone();
                let record = save
                    .records
                    .get(usize::try_from(value.bits)?)
                    .context("System4 global-save structure record")?;
                ensure!(
                    record.kind == 13 && record.name == definition.name,
                    "System4 global-save structure name/type"
                );
                let slot = self.alloc_structure(var.structure, depth + 1)?;
                for index in &record.indices {
                    let field = save
                        .fields
                        .get(usize::try_from(*index)?)
                        .context("System4 global-save structure member index")?;
                    let Some(index) = definition.members.iter().position(|v| v.name == field.name)
                    else {
                        continue;
                    };
                    let var = &definition.members[index];
                    let restored = self.restore_global_value(
                        save,
                        var,
                        &field.value,
                        nodes,
                        depth + 1,
                        check,
                    )?;
                    let old = self.heap.get(slot, index as i32)?;
                    if owns_heap(var.kind) {
                        self.discard_global_value(old, check)?;
                    }
                    self.heap.set(slot, index as i32, restored)?;
                }
                Ok(slot)
            }
            kind if is_array(kind) => {
                let array = save
                    .arrays
                    .get(usize::try_from(value.bits)?)
                    .context("System4 global-save array index")?;
                let Some(dimensions) = array.dimensions.as_ref() else {
                    return self.heap.alloc(Object::Page(None));
                };
                ensure!(
                    var.rank > 0 && var.rank as usize == dimensions.len(),
                    "System4 global-save array rank mismatch"
                );
                let mut flat = 0;
                let slot = self.restore_global_array(
                    save,
                    array,
                    var,
                    dimensions.len(),
                    &mut flat,
                    nodes,
                    depth + 1,
                    check,
                )?;
                ensure!(
                    flat == array.flat.len(),
                    "System4 global-save excess flat rows"
                );
                Ok(slot)
            }
            kind if is_reference(kind) => Ok(-1),
            _ => bail!(
                "unsupported System4 native global-save value type {}",
                var.kind
            ),
        }
    }
    fn restore_global_array(
        &mut self,
        save: &Save,
        array: &Array,
        var: &Variable,
        rank: usize,
        flat: &mut usize,
        nodes: &mut usize,
        depth: usize,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<i32> {
        check()?;
        ensure!(depth < 64 && rank > 0, "System4 global-save array nesting");
        let dimensions = array.dimensions.as_ref().unwrap();
        let count = dimensions[rank - 1];
        ensure!(count <= 1_000_000, "System4 global-save array dimension");
        let mut values = Vec::with_capacity(count);
        if rank > 1 {
            for _ in 0..count {
                values.push(self.restore_global_array(
                    save,
                    array,
                    var,
                    rank - 1,
                    flat,
                    nodes,
                    depth + 1,
                    check,
                )?);
            }
        } else {
            let row = array
                .flat
                .get(*flat)
                .context("System4 global-save missing flat row")?;
            ensure!(row.len() == count, "System4 global-save array row length");
            *flat += 1;
            let mut element = var.clone();
            element.kind = element_kind(var.kind)?;
            element.rank = 0;
            for value in row {
                values.push(self.restore_global_value(
                    save,
                    &element,
                    value,
                    nodes,
                    depth + 1,
                    check,
                )?);
            }
        }
        self.heap.alloc(Object::Page(Some(Page {
            kind: PageKind::Array {
                kind: var.kind,
                structure: var.structure,
                rank,
            },
            values,
        })))
    }
    pub(super) fn discard_global_value(
        &mut self,
        slot: i32,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<()> {
        let mut pending = vec![slot];
        let mut work = 0usize;
        while let Some(slot) = pending.pop() {
            check()?;
            if slot == -1 {
                continue;
            }
            work += 1;
            ensure!(
                work <= heap::MAX_VALUES,
                "System4 global-save old-value retirement budget"
            );
            if self.heap.slot(slot)?.references > 1 {
                self.heap.slot_mut(slot)?.references -= 1;
                continue;
            }
            if let Object::Page(Some(page)) = &self.heap.slot(slot)?.object {
                let values = page.values.clone();
                for (index, value) in values.into_iter().enumerate() {
                    if owns_heap(self.variable(slot, index as i32)?.kind) {
                        pending.push(value);
                    }
                }
            }
            self.heap.free(slot)?;
        }
        Ok(())
    }
}
fn cstr(bytes: &[u8]) -> &[u8] {
    bytes.split(|b| *b == 0).next().unwrap()
}
struct Capture<'a> {
    machine: &'a Machine,
    save: Save,
    nodes: usize,
    check: &'a dyn Fn() -> Result<()>,
}
impl Capture<'_> {
    fn value(&mut self, kind: i32, bits: i32, depth: usize) -> Result<Value> {
        (self.check)()?;
        self.nodes += 1;
        ensure!(
            self.nodes <= 1_000_000 && depth < 64,
            "System4 global-save captured value budget"
        );
        let bits = match kind {
            0 | 10 | 11 | 27 | 47 | 55 => bits,
            12 => {
                let index = i32::try_from(self.save.strings.len())?;
                self.save
                    .strings
                    .push(cstr(&self.machine.heap.string(bits)?).to_vec());
                index
            }
            13 => {
                let page = self.machine.heap.page(bits)?.clone();
                let PageKind::Structure(index) = page.kind else {
                    bail!("System4 global-save structure page kind")
                };
                let definition = self.machine.program.structures[index].clone();
                let mut indices = Vec::with_capacity(definition.members.len());
                ensure!(
                    page.values.len() == definition.members.len(),
                    "System4 global-save structure member extent"
                );
                for (var, bits) in definition.members.iter().zip(&page.values) {
                    let value = self.value(var.kind, *bits, depth + 1)?;
                    indices.push(i32::try_from(self.save.fields.len())?);
                    self.save.fields.push(Named {
                        name: var.name.clone(),
                        value,
                        unknown: 1,
                    });
                }
                let index = i32::try_from(self.save.records.len())?;
                self.save.records.push(Record {
                    kind: 13,
                    name: definition.name.clone(),
                    indices,
                });
                index
            }
            kind if is_array(kind) => {
                let mut array = Array {
                    dimensions: None,
                    flat: Vec::new(),
                };
                if let Some(page) = self.machine.heap.page_opt(bits)? {
                    let PageKind::Array {
                        kind: actual, rank, ..
                    } = page.kind
                    else {
                        bail!("System4 global-save array page kind")
                    };
                    ensure!(
                        kind == actual && rank > 0 && rank <= 32,
                        "System4 global-save array page type"
                    );
                    let mut dimensions = vec![0; rank];
                    let mut expected_rank = rank;
                    let mut page = page;
                    loop {
                        let PageKind::Array { rank, .. } = page.kind else {
                            bail!("System4 global-save nested array kind")
                        };
                        ensure!(
                            rank == expected_rank && rank > 0,
                            "System4 global-save nested array rank"
                        );
                        expected_rank -= 1;
                        dimensions[rank - 1] = page.values.len();
                        if rank == 1 {
                            break;
                        }
                        let first = page.values.first().context(
                            "System4 global-save empty outer array has no inner dimension",
                        )?;
                        page = self.machine.heap.page(*first)?;
                    }
                    array.dimensions = Some(dimensions);
                    self.capture_array(bits, &mut array, depth + 1)?;
                }
                let index = i32::try_from(self.save.arrays.len())?;
                self.save.arrays.push(array);
                index
            }
            k if is_reference(k) => -1,
            _ => bail!("unsupported System4 native captured global-save value type {kind}"),
        };
        Ok(Value { kind, bits })
    }
    fn capture_array(&mut self, slot: i32, array: &mut Array, depth: usize) -> Result<()> {
        (self.check)()?;
        ensure!(depth < 64, "System4 global-save array nesting");
        let page = self.machine.heap.page(slot)?;
        let PageKind::Array { kind, rank, .. } = page.kind else {
            bail!("System4 global-save flat array page kind")
        };
        if rank > 1 {
            let children = page.values.clone();
            for child in children {
                self.capture_array(child, array, depth + 1)?;
            }
        } else {
            let kind = element_kind(kind)?;
            let bits = page.values.clone();
            let mut row = Vec::with_capacity(bits.len());
            for value in bits {
                row.push(self.value(kind, value, depth + 1)?);
            }
            array.flat.push(row);
        }
        Ok(())
    }
}

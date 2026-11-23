// Copyright (C)2019 Nunuhara Cabbage; Rust (C)2026 CrySirror contributors.
// GPL-2.0-or-later. xsystem4 a85467d src/resume.c/native v5 call-stack policy.
//! Native RSM reconstruction preserves logical heap slots and owned references.
use super::*;
use crate::native_save::resume::{Body, CallFrame, HeapValue, Resume, ReturnRecord, Symbol};
pub(crate) struct ResumeArguments {
    pub key: Arc<Vec<u8>>,
    pub filename: Arc<Vec<u8>>,
    pub code: i32,
    pub output: Option<(i32, i32)>,
    pub comments: Option<i32>,
    strings: Vec<i32>,
    words: usize,
}
impl Machine {
    pub(crate) fn resume_arguments(&self, call: &ExternalCall) -> Result<ResumeArguments> {
        self.validate_system(call)?;
        let ExternalTarget::System(code) = call.target else {
            unreachable!()
        };
        ensure!(matches!(code, 8 | 9 | 22 | 23), "System4 resume syscall");
        let offset = match code {
            8 => 2,
            22 | 23 => 1,
            _ => 0,
        };
        let file = self.peek(offset)?;
        let key = self.peek(offset + 1)?;
        let output = if code == 8 {
            let (p, i) = self.reference(0)?;
            self.heap.get(p, i)?;
            Some((p, i))
        } else {
            None
        };
        let comments = if matches!(code, 22 | 23) {
            let slot = self.peek(0)?;
            self.heap.page_opt(slot)?;
            Some(slot)
        } else {
            None
        };
        let strings = vec![file, key];
        for slot in &strings {
            ensure!(
                self.heap.slot(*slot)?.references as usize
                    >= strings.iter().filter(|s| *s == slot).count(),
                "System4 resume string ownership"
            );
        }
        let key = self.heap.string(key)?;
        ensure!(key.len() <= 1024 * 1024, "System4 resume key budget");
        Ok(ResumeArguments {
            key,
            filename: self.heap.string(file)?,
            code,
            output,
            comments,
            strings,
            words: match code {
                8 => 4,
                22 | 23 => 3,
                _ => 2,
            },
        })
    }
    pub(crate) fn finish_resume_save(
        &mut self,
        call: &ExternalCall,
        args: &ResumeArguments,
        success: bool,
    ) -> Result<()> {
        self.validate_system(call)?;
        if let Some((p, i)) = args.output {
            self.heap.set(p, i, i32::from(success))?;
        }
        self.complete_system(
            call,
            args.words,
            Some(if args.code == 8 {
                1
            } else {
                i32::from(success)
            }),
        )?;
        for slot in &args.strings {
            self.release_string_now(*slot)?;
        }
        Ok(())
    }
    pub(crate) fn capture_resume(
        &self,
        call: &ExternalCall,
        args: &ResumeArguments,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<Resume> {
        self.validate_system(call)?;
        ensure!(
            args.code == 8
                && self.entered_main
                && !self.running_actions
                && matches!(self.actions.as_slice(), [Action::Finish]),
            "System4 resume capture outside stable main call stack"
        );
        ensure!(
            !self.frames.is_empty() && self.stack.len() >= 4,
            "System4 resume capture frame/stack"
        );
        let version = if self.program.delegates.is_empty() {
            7
        } else {
            9
        };
        let mut heap = Vec::with_capacity(self.heap.original_slots().len());
        for slot in self.heap.original_slots() {
            check()?;
            let Some(slot) = slot else {
                heap.push(HeapValue::Null);
                continue;
            };
            let references = slot.references;
            let sequence = slot.sequence;
            let item = match &slot.object {
                Object::String(bytes) => {
                    let mut bytes = bytes.as_ref().clone();
                    bytes.push(0);
                    HeapValue::String {
                        references,
                        sequence,
                        unknown: 0,
                        bytes,
                    }
                }
                Object::Page(None) => HeapValue::Array {
                    references,
                    sequence,
                    rank_minus_one: -1,
                    kind: 0,
                    structure: Symbol::Name(Vec::new()),
                    root_rank: -1,
                    allocated: false,
                    values: Vec::new(),
                },
                Object::Page(Some(page)) => match page.kind {
                    PageKind::Global => HeapValue::Frame {
                        references,
                        sequence,
                        global: true,
                        function: Symbol::Id(-1),
                        types: self.program.globals.iter().map(|v| v.kind).collect(),
                        structure: -1,
                        values: page.values.clone(),
                    },
                    PageKind::Local(index) => {
                        let f = &self.program.functions[index];
                        let structure = self
                            .frames
                            .iter()
                            .find(|frame| frame.local_page == heap.len() as i32)
                            .map_or(-1, |f| f.structure_page);
                        HeapValue::Frame {
                            references,
                            sequence,
                            global: false,
                            function: Symbol::Name(f.name.clone()),
                            types: f.variables.iter().map(|v| v.kind).collect(),
                            structure,
                            values: page.values.clone(),
                        }
                    }
                    PageKind::Structure(index) => {
                        let d = &self.program.structures[index];
                        let symbol = |i: i32| -> Result<Symbol> {
                            Ok(Symbol::Name(if i < 0 {
                                Vec::new()
                            } else {
                                self.program
                                    .functions
                                    .get(i as usize)
                                    .context("System4 resume structure function")?
                                    .name
                                    .clone()
                            }))
                        };
                        HeapValue::Structure {
                            references,
                            sequence,
                            constructor: symbol(d.constructor)?,
                            destructor: symbol(d.destructor)?,
                            structure: Symbol::Name(d.name.clone()),
                            types: d.members.iter().map(|v| v.kind).collect(),
                            values: page.values.clone(),
                        }
                    }
                    PageKind::Array {
                        kind,
                        structure,
                        rank,
                    } => HeapValue::Array {
                        references,
                        sequence,
                        rank_minus_one: i32::try_from(rank)? - 1,
                        kind,
                        structure: Symbol::Name(if structure < 0 {
                            Vec::new()
                        } else {
                            self.program
                                .structures
                                .get(structure as usize)
                                .context("System4 resume array structure")?
                                .name
                                .clone()
                        }),
                        root_rank: i32::try_from(rank)?,
                        allocated: !page.values.is_empty(),
                        values: page.values.clone(),
                    },
                    PageKind::Delegate => HeapValue::Delegate {
                        references,
                        sequence,
                        values: page.values.clone(),
                    },
                },
            };
            heap.push(item);
        }
        let mut frames = vec![CallFrame {
            kind: 4,
            local: -1,
            structure: -1,
        }];
        let mut returns = vec![ReturnRecord {
            address: -1,
            function: None,
            local: 0,
            crc: 0,
        }];
        for (i, frame) in self.frames.iter().enumerate() {
            check()?;
            frames.push(CallFrame {
                kind: if frame.structure_page >= 0 {
                    2
                } else if frame.function == self.program.main as usize {
                    0
                } else {
                    1
                },
                local: frame.local_page,
                structure: frame.structure_page,
            });
            if i > 0 {
                let Return::Address(address) = frame.return_to else {
                    bail!("System4 resume nested continuation is not an original call")
                };
                let parent = &self.program.functions[self.frames[i - 1].function];
                returns.push(return_record(address, parent)?);
            }
        }
        let top = &self.program.functions[self.frames.last().unwrap().function];
        let ip = return_record(call.next, top)?;
        let body = Body {
            ip,
            stack: self.stack[..self.stack.len() - 4].to_vec(),
            frames,
            returns,
            next_sequence: self.heap.original_next_sequence(),
            heap,
            functions: self
                .program
                .functions
                .iter()
                .map(|f| f.name.clone())
                .collect(),
        };
        Ok(Resume {
            version,
            key: cstr(&args.key).to_vec(),
            comments: Vec::new(),
            body: Some(body),
        })
    }
    pub(crate) fn apply_resume(
        &mut self,
        args: &ResumeArguments,
        save: &Resume,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<()> {
        ensure!(save.key == cstr(&args.key), "System4 resume key mismatch");
        let body = save
            .body
            .as_ref()
            .context("System4 cannot load a comments-only resume")?;
        if save.version >= 6 {
            ensure!(
                body.functions.len() == self.program.functions.len()
                    && body
                        .functions
                        .iter()
                        .zip(&self.program.functions)
                        .all(|(n, f)| *n == f.name),
                "System4 resume function-table revision mismatch"
            );
        }
        ensure!(
            body.frames
                .first()
                .is_some_and(|f| f.kind == 4 && f.local == -1)
                && body.frames.len() > 1
                && body.frames.len() <= MAX_CALLS + 1,
            "System4 resume call-stack bottom/count"
        );
        ensure!(
            body.returns.first().is_some_and(|r| r.address == -1)
                && matches!(body.returns.len(),n if n==body.frames.len()||n+1==body.frames.len()),
            "System4 resume return-record layout"
        );
        // Native files with equal frame/return counts carry one extra
        // bottom record. resume.c skips it before advancing over stack bottom.
        let skip = if body.returns.len() == body.frames.len() {
            2
        } else {
            1
        };
        let mut records = body.returns.iter().skip(skip);
        let mut entries = Vec::with_capacity(body.frames.len() - 1);
        let mut addresses = Vec::new();
        for frame in body.frames.iter().skip(1) {
            check()?;
            ensure!(frame.kind != 4, "System4 resume duplicate stack bottom");
            let record = records.next().unwrap_or(&body.ip);
            let name = record
                .function
                .as_ref()
                .context("System4 resume function return record")?;
            let f = self
                .program
                .functions
                .iter()
                .position(|f| &f.name == name)
                .context("System4 resume missing function")?;
            let definition = &self.program.functions[f];
            ensure!(
                definition.crc == record.crc && record.local >= 0,
                "System4 resume function CRC/offset"
            );
            let address = definition
                .address
                .checked_add(record.local as u32)
                .context("System4 resume return address overflow")?;
            addresses.push(address);
            entries.push((f, frame));
        }
        let next = *addresses
            .last()
            .context("System4 resume instruction pointer")?;
        let saved_call = next
            .checked_sub(6)
            .context("System4 resume instruction offset")?;
        let instruction = self.program.instruction(saved_call)?;
        ensure!(
            instruction.name == "CALLSYS"
                && instruction.arguments == [8]
                && instruction.next == next,
            "System4 resume must return from original ResumeSave"
        );
        let mut needed: std::collections::BTreeMap<u32, u32> =
            addresses.iter().map(|a| (*a, u32::MAX)).collect();
        let mut pc = 0;
        let mut remaining = needed.len();
        while pc < next || remaining > 0 {
            check()?;
            ensure!(
                (pc as usize) < self.program.code.len(),
                "System4 resume return outside bytecode"
            );
            let instruction = self.program.instruction(pc)?;
            if let Some(slot) = needed.get_mut(&instruction.next) {
                if *slot == u32::MAX {
                    remaining -= 1;
                }
                *slot = pc;
            }
            pc = instruction.next;
        }
        let mut slots = Vec::with_capacity(body.heap.len());
        for (index, entry) in body.heap.iter().enumerate() {
            check()?;
            let (references, sequence, object) = match entry {
                HeapValue::Null => {
                    slots.push(None);
                    continue;
                }
                HeapValue::Frame {
                    references,
                    sequence,
                    global,
                    function,
                    types,
                    values,
                    ..
                } => {
                    let (kind, vars) = if *global {
                        ensure!(index == 0, "System4 resume misplaced global page");
                        (PageKind::Global, &self.program.globals)
                    } else {
                        let f = resolve_function(&self.program, function)?;
                        ensure!(f >= 0, "System4 resume null local function");
                        (
                            PageKind::Local(f as usize),
                            &self.program.functions[f as usize].variables,
                        )
                    };
                    ensure!(
                        vars.len() == values.len()
                            && vars.iter().map(|v| v.kind).eq(types.iter().copied()),
                        "System4 resume frame variable types/count"
                    );
                    (
                        *references,
                        *sequence,
                        Object::Page(Some(Page {
                            kind,
                            values: values.clone(),
                        })),
                    )
                }
                HeapValue::String {
                    references,
                    sequence,
                    bytes,
                    ..
                } => (
                    *references,
                    *sequence,
                    Object::String(Arc::new(bytes[..bytes.len() - 1].to_vec())),
                ),
                HeapValue::Array {
                    references,
                    sequence,
                    rank_minus_one,
                    kind,
                    structure,
                    values,
                    ..
                } => {
                    let page = if *rank_minus_one < 0 {
                        None
                    } else {
                        ensure!(is_array(*kind), "System4 resume array type");
                        Some(Page {
                            kind: PageKind::Array {
                                kind: *kind,
                                structure: resolve_structure(&self.program, structure)?,
                                rank: (*rank_minus_one + 1) as usize,
                            },
                            values: values.clone(),
                        })
                    };
                    (*references, *sequence, Object::Page(page))
                }
                HeapValue::Structure {
                    references,
                    sequence,
                    constructor,
                    destructor,
                    structure,
                    types,
                    values,
                    ..
                } => {
                    let index = resolve_structure(&self.program, structure)?;
                    ensure!(index >= 0, "System4 resume structure identity");
                    let d = &self.program.structures[index as usize];
                    ensure!(
                        d.constructor == resolve_function(&self.program, constructor)?
                            && d.destructor == resolve_function(&self.program, destructor)?
                            && d.members.len() == values.len()
                            && d.members.iter().map(|v| v.kind).eq(types.iter().copied()),
                        "System4 resume structure definition mismatch"
                    );
                    (
                        *references,
                        *sequence,
                        Object::Page(Some(Page {
                            kind: PageKind::Structure(index as usize),
                            values: values.clone(),
                        })),
                    )
                }
                HeapValue::Delegate {
                    references,
                    sequence,
                    values,
                } => (
                    *references,
                    *sequence,
                    Object::Page(Some(Page {
                        kind: PageKind::Delegate,
                        values: values.clone(),
                    })),
                ),
            };
            let sequence = if save.version >= 9 {
                sequence
            } else {
                index as u32 + 1
            };
            slots.push(Some(heap::Slot {
                references,
                sequence,
                object,
            }));
        }
        let next_sequence = if save.version >= 9 {
            body.next_sequence
        } else {
            u32::try_from(slots.len())?
                .checked_add(1)
                .context("System4 resume legacy sequence")?
        };
        let heap = Heap::from_original_slots(slots, next_sequence)?;
        ensure!(
            matches!(heap.page(0)?.kind, PageKind::Global),
            "System4 resume global page missing"
        );
        let mut frames = Vec::with_capacity(entries.len());
        for (i, (function, frame)) in entries.into_iter().enumerate() {
            ensure!(
                matches!(heap.page(frame.local)?.kind,PageKind::Local(f)if f==function),
                "System4 resume frame local identity"
            );
            if frame.structure >= 0 {
                heap.page(frame.structure)?;
            }
            frames.push(Frame {
                function,
                local_page: frame.local,
                structure_page: frame.structure,
                call_pc: if i == 0 { 0 } else { needed[&addresses[i - 1]] },
                return_to: if i == 0 {
                    Return::Actions
                } else {
                    Return::Address(addresses[i - 1])
                },
            });
        }
        self.heap = heap;
        self.frames = frames;
        self.stack = body.stack.clone();
        self.push(0)?;
        self.pc = next;
        self.pending = None;
        self.actions = vec![Action::Finish];
        self.running_actions = false;
        self.halted = false;
        self.exit_status = None;
        self.entered_main = true;
        self.validate_resume_heap(check)?;
        Ok(())
    }
    pub(crate) fn capture_resume_comments(&self, args: &ResumeArguments) -> Result<Vec<Vec<u8>>> {
        let slot = args
            .comments
            .context("System4 resume comment array missing")?;
        let page = self.heap.page(slot)?;
        ensure!(
            matches!(
                page.kind,
                PageKind::Array {
                    kind: 16,
                    rank: 1,
                    ..
                }
            ),
            "System4 resume comments require rank-one string array"
        );
        ensure!(
            page.values.len() <= 1_000_000,
            "System4 resume comment count limit"
        );
        let mut size = 0usize;
        let mut comments = Vec::with_capacity(page.values.len());
        for slot in &page.values {
            let string = self.heap.string(*slot)?;
            let bytes = cstr(&string);
            size = size
                .checked_add(bytes.len() + 1)
                .filter(|s| *s <= 64 * 1024 * 1024)
                .context("System4 resume comment byte budget")?;
            comments.push(bytes.to_vec());
        }
        Ok(comments)
    }
    pub(crate) fn apply_resume_comments(
        &mut self,
        call: &ExternalCall,
        args: &ResumeArguments,
        comments: &[Vec<u8>],
        success: bool,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<()> {
        self.validate_system(call)?;
        ensure!(args.code == 23, "System4 resume read-comment syscall");
        let slot = args
            .comments
            .context("System4 resume comment target missing")?;
        if let Some(page) = self.heap.page_opt(slot)? {
            ensure!(
                matches!(
                    page.kind,
                    PageKind::Array {
                        kind: 16,
                        rank: 1,
                        ..
                    }
                ),
                "System4 resume comment target array type"
            );
        }
        let mut values = Vec::with_capacity(comments.len());
        for comment in comments {
            check()?;
            values.push(self.heap.alloc(Object::String(Arc::new(comment.clone())))?);
        }
        let old = self
            .heap
            .page_opt(slot)?
            .map(|p| p.values.clone())
            .unwrap_or_default();
        for value in old {
            self.discard_global_value(value, check)?;
        }
        let page = if values.is_empty() {
            None
        } else {
            Some(Page {
                kind: PageKind::Array {
                    kind: 16,
                    structure: 0,
                    rank: 1,
                },
                values,
            })
        };
        self.heap.set_page(slot, page)?;
        self.finish_resume_save(call, args, success)
    }
    pub(crate) fn prepare_comment_resume(
        &self,
        args: &ResumeArguments,
        old: Option<Resume>,
        comments: Vec<Vec<u8>>,
    ) -> Result<Resume> {
        let mut save = old.unwrap_or(Resume {
            version: if self.program.delegates.is_empty() {
                7
            } else {
                9
            },
            key: cstr(&args.key).to_vec(),
            comments: Vec::new(),
            body: None,
        });
        ensure!(
            save.key == cstr(&args.key),
            "System4 resume comment key mismatch"
        );
        if save.version <= 6 {
            if let Some(body) = &mut save.body {
                for value in &mut body.heap {
                    match value {
                        HeapValue::Frame {
                            function,
                            global: false,
                            ..
                        } => name_function(&self.program, function)?,
                        HeapValue::Array { structure, .. } => {
                            name_structure(&self.program, structure)?
                        }
                        HeapValue::Structure {
                            constructor,
                            destructor,
                            structure,
                            ..
                        } => {
                            name_function(&self.program, constructor)?;
                            name_function(&self.program, destructor)?;
                            name_structure(&self.program, structure)?;
                        }
                        _ => {}
                    }
                }
                body.functions = self
                    .program
                    .functions
                    .iter()
                    .map(|f| f.name.clone())
                    .collect();
            }
            save.version = 7;
        }
        save.comments = comments;
        Ok(save)
    }
    fn validate_resume_heap(&self, check: &dyn Fn() -> Result<()>) -> Result<()> {
        for (index, slot) in self.heap.original_slots().iter().enumerate() {
            check()?;
            let Some(slot) = slot else { continue };
            let Object::Page(Some(page)) = &slot.object else {
                continue;
            };
            if matches!(page.kind, PageKind::Delegate) {
                continue;
            }
            for (n, value) in page.values.iter().enumerate() {
                let var = self.variable(index as i32, n as i32)?;
                if owns_heap(var.kind) && *value != -1 {
                    let target = self.heap.slot(*value)?;
                    if matches!(var.kind, 12 | 20) {
                        ensure!(
                            matches!(target.object, Object::String(_)),
                            "System4 resume string reference type"
                        );
                    } else if is_array(var.kind) {
                        match &target.object {
                            Object::Page(None) => {}
                            Object::Page(Some(p)) => ensure!(
                                matches!(&p.kind,PageKind::Array{kind,rank,..}if *kind==var.kind&&var.rank>0&&*rank==var.rank as usize),
                                "System4 resume array reference kind/rank"
                            ),
                            _ => bail!("System4 resume array reference type"),
                        }
                    } else if var.kind == 13 {
                        ensure!(
                            matches!(&target.object,Object::Page(Some(Page{kind:PageKind::Structure(index),..}))if var.structure>=0&&*index==var.structure as usize),
                            "System4 resume structure reference identity"
                        );
                    } else if is_reference(var.kind) || var.kind == 63 {
                        ensure!(
                            matches!(target.object, Object::Page(_)) || matches!(var.kind, 20),
                            "System4 resume page reference type"
                        );
                    }
                }
            }
        }
        Ok(())
    }
}
fn cstr(b: &[u8]) -> &[u8] {
    b.split(|b| *b == 0).next().unwrap()
}
fn return_record(address: u32, function: &crate::ain::Function) -> Result<ReturnRecord> {
    Ok(ReturnRecord {
        address: i32::try_from(address)?,
        function: Some(function.name.clone()),
        local: i32::try_from(
            address
                .checked_sub(function.address)
                .context("System4 resume caller offset")?,
        )?,
        crc: function.crc,
    })
}
fn resolve_function(program: &Program, s: &Symbol) -> Result<i32> {
    match s {
        Symbol::Id(i) => {
            ensure!(
                *i >= -1 && (*i < 0 || (*i as usize) < program.functions.len()),
                "System4 resume function index"
            );
            Ok(*i)
        }
        Symbol::Name(n) if n.is_empty() => Ok(-1),
        Symbol::Name(n) => Ok(i32::try_from(
            program
                .functions
                .iter()
                .position(|f| &f.name == n)
                .context("System4 resume function symbol")?,
        )?),
    }
}
fn resolve_structure(program: &Program, s: &Symbol) -> Result<i32> {
    match s {
        Symbol::Id(i) => {
            ensure!(
                *i >= -1 && (*i < 0 || (*i as usize) < program.structures.len()),
                "System4 resume structure index"
            );
            Ok(*i)
        }
        Symbol::Name(n) if n.is_empty() => Ok(-1),
        Symbol::Name(n) => Ok(i32::try_from(
            program
                .structures
                .iter()
                .position(|f| &f.name == n)
                .context("System4 resume structure symbol")?,
        )?),
    }
}

fn name_function(program: &Program, symbol: &mut Symbol) -> Result<()> {
    if let Symbol::Id(_) = symbol {
        let index = resolve_function(program, symbol)?;
        *symbol = Symbol::Name(if index < 0 {
            Vec::new()
        } else {
            program.functions[index as usize].name.clone()
        });
    }
    Ok(())
}
fn name_structure(program: &Program, symbol: &mut Symbol) -> Result<()> {
    if let Symbol::Id(_) = symbol {
        let index = resolve_structure(program, symbol)?;
        *symbol = Symbol::Name(if index < 0 {
            Vec::new()
        } else {
            program.structures[index as usize].name.clone()
        });
    }
    Ok(())
}

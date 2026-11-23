// SPDX-License-Identifier: MPL-2.0

//! Bounded multi-slot stepping. Unimplemented host commands remain pending.
use super::abi::{frame_offset, ParameterStack, StringRef, SystemResult};
use super::evaluate::{error, Env, Frame, Journal, State};
use super::{DecodeError, Expression, InstructionKind, Term, VmProfile};
use crate::Script;
use std::{collections::BTreeMap, sync::Arc};

#[cfg(feature = "runtime")]
mod csp;

/// Frame table +82548 has 64 entries indexed by depth +82804; entry 0 is the base.
const MAX_CALL_DEPTH: usize = 63;
/// The +82864 accumulator and each string slot are 1024 bytes including the NUL.
const STRING_CAPACITY: usize = 1023;

fn c_string(bytes: &[u8], offset: u32, pc: usize, tag: u16) -> Result<&[u8], DecodeError> {
    let tail = bytes
        .get(offset as usize..)
        .ok_or_else(|| error(pc, tag, "string reference out of bounds"))?;
    let end = tail
        .iter()
        .position(|b| *b == 0)
        .ok_or_else(|| error(pc, tag, "unterminated string reference"))?;
    Ok(&tail[..end])
}
fn append(buffer: &mut Vec<u8>, bytes: &[u8], pc: usize, tag: u16) -> Result<(), DecodeError> {
    if buffer.len() + bytes.len() > STRING_CAPACITY {
        return Err(error(pc, tag, "string accumulator exceeds 1023 bytes"));
    }
    buffer.extend_from_slice(bytes);
    Ok(())
}
/// Out-of-range direct string operands leave the accumulator unchanged.
fn append_slot(
    state: &State,
    buffer: &mut Vec<u8>,
    index: u32,
    pc: usize,
    tag: u16,
) -> Result<(), DecodeError> {
    if index >= state.string_cells() {
        return Ok(());
    }
    let bytes = state
        .string(index)
        .ok_or_else(|| error(pc, tag, "read of unknown string slot"))?;
    append(buffer, bytes, pc, tag)
}

#[derive(Clone)]
struct Image {
    profile: VmProfile,
    script: Arc<Script>,
    data: Vec<u8>,
    modified_text: Option<Arc<Vec<u8>>>,
    boundaries: Arc<[u8]>,
    labels: Arc<[u32]>,
    name: Vec<u8>,
}

pub struct PreparedScript(Image);

impl PreparedScript {
    /// Prepare bounded immutable code before the owner can publish the image.
    pub(super) fn new(
        profile: VmProfile,
        script: Script,
        name: Vec<u8>,
    ) -> Result<Self, DecodeError> {
        if script.format() != profile.script_format() {
            return Err(error(0, 0, "script format does not match native ABI"));
        }
        let code = script.code();
        let mut boundaries = Vec::new();
        boundaries
            .try_reserve_exact(code.len().div_ceil(8))
            .map_err(|_| error(0, 0, "instruction boundary allocation failed"))?;
        boundaries.resize(code.len().div_ceil(8), 0);
        let mut pc = 0;
        while pc < code.len() {
            let insn = profile.decode_at(code, pc)?;
            boundaries[pc / 8] |= 1 << (pc % 8);
            pc = insn.end;
        }
        let mut labels = Vec::new();
        labels
            .try_reserve_exact(script.labels().len())
            .map_err(|_| error(0, 0, "label table allocation failed"))?;
        labels.extend(script.labels());
        let mut data = Vec::new();
        data.try_reserve_exact(script.initial_data().len())
            .map_err(|_| error(0, 0, "script data allocation failed"))?;
        data.extend_from_slice(script.initial_data());
        let image = Image {
            profile,
            script: Arc::new(script),
            data,
            modified_text: None,
            boundaries: boundaries.into(),
            labels: labels.into(),
            name,
        };
        let entry = image.script.entry_pc() as usize;
        if !image.is_boundary(entry) {
            return Err(error(entry, 0, "entry is not an instruction boundary"));
        }
        for &label in image.labels.iter() {
            if !image.is_boundary(label as usize) {
                return Err(error(
                    label as usize,
                    0,
                    "label is not an instruction boundary",
                ));
            }
        }
        Ok(Self(image))
    }
}
impl Image {
    fn is_boundary(&self, pc: usize) -> bool {
        self.boundaries
            .get(pc / 8)
            .is_some_and(|word| *word & (1 << (pc % 8)) != 0)
    }

    fn text_bytes(&self) -> &[u8] {
        self.modified_text
            .as_deref()
            .map_or_else(|| self.script.text_bytes(), Vec::as_slice)
    }

    fn write_text(&mut self, start: usize, bytes: &[u8], terminate: bool) -> Result<(), &'static str> {
        let end = start
            .checked_add(bytes.len() + usize::from(terminate))
            .filter(|&end| end <= self.text_bytes().len())
            .ok_or("CMVS string write outside script text")?;
        if self.modified_text.as_mut().and_then(Arc::get_mut).is_none() {
            let mut copy = Vec::new();
            let text = self.text_bytes();
            copy.try_reserve_exact(text.len())
                .map_err(|_| "CMVS mutable text allocation failed")?;
            copy.extend_from_slice(text);
            self.modified_text = Some(Arc::new(copy));
        }
        let text = self
            .modified_text
            .as_mut()
            .and_then(Arc::get_mut)
            .expect("unique text backing");
        text[start..start + bytes.len()].copy_from_slice(bytes);
        if terminate { text[end - 1] = 0; }
        Ok(())
    }
}

#[cfg(feature = "runtime")]
impl Machine {
    pub(super) fn snapshot(&self) -> serde_json::Value {
        use super::snapshot::hex;
        let images: Vec<_> = self
            .images
            .iter()
            .map(|(slot, image)| {
                serde_json::json!({
                    "slot": slot,
                    "name": hex(&image.name),
                    "data": hex(&image.data),
                    "text": image.modified_text.as_deref().map(|text| hex(text)),
                })
            })
            .collect();
        serde_json::json!({
            "images": images,
            "slot": self.slot,
            "pc": self.pc,
            "frames": self.frames,
            "cross": self.cross_calls,
            "native_return_stack": self.native_return_stack,
            "cvns_arguments": self.cvns_arguments,
            "repeat": self.repeat_count,
            "state": self.state.snapshot(),
            "stack": self.stack.snapshot(),
        })
    }

    pub(super) fn from_snapshot(
        value: &serde_json::Value,
        scripts: Vec<(u8, PreparedScript)>,
    ) -> Option<Self> {
        use super::snapshot::{bytes, word, words};
        let mut images = BTreeMap::new();
        for (slot, PreparedScript(mut image)) in scripts {
            let saved = value["images"]
                .as_array()?
                .iter()
                .find(|saved| word(saved, "slot") == Some(u32::from(slot)))?;
            image.data = bytes(saved, "data")?;
            image.modified_text = saved
                .get("text")
                .filter(|text| !text.is_null())
                .and_then(|_| bytes(saved, "text"))
                .map(Arc::new);
            images.insert(slot, image);
        }
        let slot = word(value, "slot")? as u8;
        images.contains_key(&slot).then_some(())?;
        let cvns = images.get(&0)?.profile == VmProfile::Cvns20040515X86;
        Some(Self {
            images,
            slot,
            pc: word(value, "pc")? as usize,
            state: State::from_snapshot(&value["state"])?,
            stack: ParameterStack::from_snapshot(&value["stack"], cvns)?,
            frames: words(value, "frames")?.into_iter().map(|at| at as usize).collect(),
            cvns_arguments: match value.get("cvns_arguments") { Some(_) => words(value,"cvns_arguments")?.try_into().ok()?, None=>[0;16] },
            native_return_stack: cvns && value["native_return_stack"].as_bool().unwrap_or(false),
            cross_calls: words(value, "cross")?.into_iter().map(|at| at as u8).collect(),
            pending: None,
            repeat_count: word(value, "repeat")?,
        })
    }

    /// Expanded PS2A image of the main script, stored in the slot's native third section.
    pub(super) fn main_expanded(&self) -> Option<Vec<u8>> {
        let image = self.images.get(&0)?;
        image.script.expanded_image(&image.data, image.text_bytes()).ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Continue,
    Yield,
    System { pc: usize, command: u16 },
}

#[derive(Clone)]
pub struct Machine {
    images: BTreeMap<u8, Image>,
    slot: u8,
    pc: usize,
    state: State,
    stack: ParameterStack,
    frames: Vec<usize>,
    cross_calls: Vec<u8>,
    // Original CSP1 keeps return contexts in the parameter stack alone.
    native_return_stack: bool,
    cvns_arguments: [u32;16],
    pending: Option<(usize, u16)>,
    repeat_count: u32,
}

impl Machine {
    pub fn new(PreparedScript(image): PreparedScript) -> Self {
        let pc = image.script.entry_pc() as usize;
        let state = match image.profile {
            VmProfile::Cvns20040515X86 => State::new().with_string_cells(256),
            _ => State::new(),
        };
        let stack = if image.profile == VmProfile::Cvns20040515X86 { ParameterStack::cvns() } else { ParameterStack::new(65536) };
        Self {
            images: BTreeMap::from([(0, image)]),
            slot: 0,
            pc,
            state,
            stack,
            frames: Vec::new(),
            cross_calls: Vec::new(),
            native_return_stack: false,
            cvns_arguments:[0;16],
            pending: None,
            repeat_count: 0,
        }
    }
    pub fn pc(&self) -> usize {
        self.pc
    }
    fn image(&self) -> &Image {
        &self.images[&self.slot]
    }
    pub fn current_slot(&self) -> u8 {
        self.slot
    }
    pub fn profile(&self) -> VmProfile {
        self.image().profile
    }
    pub fn current_name(&self) -> &[u8] {
        &self.image().name
    }
    /// Name of the script in the main slot (engine +14616).
    pub fn main_name(&self) -> &[u8] {
        self.images.get(&0).map_or(&[], |image| &image.name)
    }
    pub fn is_boundary(&self, slot: u8, pc: usize) -> bool {
        self.images
            .get(&slot)
            .is_some_and(|image| image.is_boundary(pc))
    }
    pub fn state(&self) -> &State {
        &self.state
    }
    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }
    pub fn stack(&self) -> &ParameterStack {
        &self.stack
    }
    /// Current slot's mutable script data, initialized from the PS2A unknown segment.
    pub fn script_data(&self) -> &[u8] {
        &self.image().data
    }
    pub fn pending_system(&self) -> Option<(usize, u16)> {
        self.pending
    }
    #[cfg(feature = "runtime")]
    pub(super) fn repeats_system(&self) -> bool {
        self.repeat_count != 0
    }
    pub fn resolve_string(&self, raw: u32) -> Result<&[u8], DecodeError> {
        let (bytes, offset) = match StringRef::decode(raw) {
            StringRef::Text(offset) => (self.image().text_bytes(), offset),
            StringRef::ScriptData(offset) => (self.script_data(), offset),
            StringRef::StringBank(index) => {
                let index = self.state.string_slot(index);
                return self
                    .state
                    .string(index)
                    .ok_or_else(|| error(self.pc, 0x201, "read of unknown string slot"));
            }
            StringRef::Frame(offset) => return self.frame_string(offset, self.pc, 0x201),
        };
        c_string(bytes, offset, self.pc, 0x201)
    }

    /// Read a byte within the referenced region, independent of C string termination.
    pub(super) fn string_storage_byte(
        &self,
        opcode: u16,
        raw: u32,
        index: usize,
    ) -> Result<u8, DecodeError> {
        let (bytes, offset) = match StringRef::decode(raw) {
            StringRef::Text(offset) => (self.image().text_bytes(), offset as usize),
            StringRef::ScriptData(offset) => (self.script_data(), offset as usize),
            StringRef::StringBank(slot) => {
                let cell = self
                    .state
                    .raw_string_cell(self.state.string_slot(slot))
                    .expect("normalized slot");
                (cell.as_slice(), 0)
            }
            StringRef::Frame(raw) => {
                let offset = frame_offset(self.frame_base(), raw, false)
                    .and_then(|start| start.checked_add(index))
                    .ok_or_else(|| {
                        error(self.pc, opcode, "fixed-width frame read outside stack")
                    })?;
                return self
                    .stack
                    .initialized_byte(offset)
                    .map_err(|e| error(self.pc, opcode, e.0));
            }
        };
        offset
            .checked_add(index)
            .and_then(|start| bytes.get(start))
            .copied()
            .ok_or_else(|| {
                error(
                    self.pc,
                    opcode,
                    "fixed-width string read outside its region",
                )
            })
    }

    /// RegQueryValueExA copies the stored byte count, without adding a string terminator.
    #[cfg(feature = "runtime")]
    pub(super) fn write_registry_bytes(&mut self, reference: u32, bytes: &[u8]) -> Result<(), DecodeError> {
        let pc = self.pc;
        let fail = |reason| error(pc, 0x2049, reason);
        match StringRef::decode(reference) {
            StringRef::Text(offset) => self.images.get_mut(&self.slot).expect("active image")
                .write_text(offset as usize, bytes, false).map_err(fail),
            StringRef::ScriptData(offset) => {
                let data = &mut self.images.get_mut(&self.slot).expect("active image").data;
                data.get_mut(offset as usize..).and_then(|tail| tail.get_mut(..bytes.len()))
                    .ok_or_else(|| fail("registry value outside script data"))?.copy_from_slice(bytes);
                Ok(())
            }
            StringRef::StringBank(slot) => self.state.write_cell_prefix(self.state.string_slot(slot), bytes).map_err(fail),
            StringRef::Frame(raw) => {
                let start = frame_offset(self.frame_base(), raw, false).ok_or_else(|| fail("registry destination outside stack"))?;
                self.stack.write_prefix(start, bytes).map_err(|e| fail(e.0))
            }
        }
    }

    pub(super) fn write_string_reference(
        &mut self,
        opcode: u16,
        reference: u32,
        bytes: &[u8],
    ) -> Result<(), DecodeError> {
        let (tag, offset) = match StringRef::decode(reference) {
            StringRef::StringBank(slot) => (0x121, self.state.string_slot(slot)),
            StringRef::ScriptData(offset) => (0x125, offset),
            StringRef::Frame(offset) => (0x127, offset),
            StringRef::Text(offset) => {
                return self
                    .images
                    .get_mut(&self.slot)
                    .expect("active image")
                    .write_text(offset as usize, bytes, true)
                    .map_err(|why| error(self.pc, opcode, why));
            }
        };
        self.assign_string((tag, offset, self.pc), bytes, &mut Journal::default())
            .map_err(|e| error(self.pc, opcode, e.reason))
    }

    /// Word at the current call frame minus `distance`, as tag 0x122/0x103 operands read it.
    fn frame_word(&self, distance: u32, pc: usize, tag: u16) -> Result<u32, DecodeError> {
        let offset = frame_offset(self.frame_base(), distance, true)
            .ok_or_else(|| error(pc, tag, "frame operand below stack base"))?;
        self.stack
            .read_u32_at(offset)
            .map_err(|e| error(pc, tag, e.0))
    }

    fn frame_string(&self, raw: u32, pc: usize, tag: u16) -> Result<&[u8], DecodeError> {
        let offset = frame_offset(self.frame_base(), raw, false)
            .ok_or_else(|| error(pc, tag, "frame string outside stack"))?;
        self.stack.c_string(offset).map_err(|e| error(pc, tag, e.0))
    }

    /// Writes a syscall's output string through a script string reference.
    #[cfg(feature = "runtime")]
    pub(super) fn store_string(&mut self, raw: u32, bytes: &[u8]) -> Result<(), DecodeError> {
        let (tag, value) = match StringRef::decode(raw) {
            StringRef::Text(_) => (0x120, 0),
            StringRef::ScriptData(offset) => (0x125, offset),
            StringRef::StringBank(slot) => (0x121, slot),
            StringRef::Frame(offset) => (0x127, offset),
        };
        let end = bytes.len().min(1023);
        self.assign_string((tag, value, self.pc), &bytes[..end], &mut Journal::default())
    }

    fn assign_string(
        &mut self,
        (tag, raw, pc): (u16, u32, usize),
        bytes: &[u8],
        journal: &mut Journal,
    ) -> Result<(), DecodeError> {
        let fail = |why| error(pc, tag, why);
        match tag {
            0x121 => {
                let slot = raw & 0x3fff_ffff;
                if slot < self.state.string_cells() {
                    self.state
                        .set_string_journaled(slot, bytes, journal)
                        .map_err(fail)?;
                }
            }
            0x125 => {
                let offset = raw as usize;
                let data = &mut self.images.get_mut(&self.slot).expect("active image").data;
                let end = offset
                    .checked_add(bytes.len() + 1)
                    .filter(|&end| end <= data.len())
                    .ok_or_else(|| fail("script string write outside data segment"))?;
                journal.data_string(offset, data[offset..end].to_vec());
                data[offset..end - 1].copy_from_slice(bytes);
                data[end - 1] = 0;
            }
            0x127 => {
                let offset = frame_offset(self.frame_base(), raw, false)
                    .ok_or_else(|| fail("frame string outside stack"))?;
                let old = self
                    .stack
                    .write_c_string(offset, bytes)
                    .map_err(|e| fail(e.0))?;
                journal.stack_string(offset, old);
            }
            0 | 0x120 | 0x122 => {}
            _ => return Err(fail("unsupported CMVS string assignment target")),
        }
        Ok(())
    }

    /// Operand reader 0x1400652e0 for string tags: appends to the accumulator.
    fn append_operand(
        &self,
        state: &State,
        buffer: &mut Vec<u8>,
        (tag, value, pc): (u16, u32, usize),
    ) -> Result<(), DecodeError> {
        let text = self.image().text_bytes();
        let bytes = match tag {
            // Placeholder left by a previous operator: the original reader returns 0.
            0 => return Ok(()),
            0x120 => c_string(text, value, pc, tag)?,
            0x125 => c_string(self.script_data(), value, pc, tag)?,
            0x127 => self.frame_string(value, pc, tag)?,
            0x121 => return append_slot(state, buffer, value & 0x3fff_ffff, pc, tag),
            0x122 => match StringRef::decode(self.frame_word(value, pc, tag)?) {
                StringRef::Text(offset) => c_string(text, offset, pc, tag)?,
                StringRef::ScriptData(offset) => c_string(self.script_data(), offset, pc, tag)?,
                StringRef::StringBank(index) => {
                    return append_slot(state, buffer, index, pc, tag);
                }
                StringRef::Frame(offset) => self.frame_string(offset, pc, tag)?,
            },
            _ => return Err(error(pc, tag, "string operand type not implemented")),
        };
        append(buffer, bytes, pc, tag)
    }

    /// 0x0201 / 0x140066b00: clears the accumulator, appends operands as operators read them,
    /// and sets the result only from the tag of the bottom entry. Commits all writes or none.
    fn string_expression(&mut self, expr: &Expression) -> Result<(), DecodeError> {
        let mut journal = Journal::default();
        let outcome = self.string_expression_in(expr, &mut journal);
        if outcome.is_err() {
            self.state.rollback(
                journal,
                Some(&mut self.stack),
                Some(&mut self.images.get_mut(&self.slot).expect("active image").data),
            );
        }
        outcome
    }
    fn string_expression_in(
        &mut self,
        expr: &Expression,
        journal: &mut Journal,
    ) -> Result<(), DecodeError> {
        let mut buffer = Vec::new();
        let mut entries: Vec<(u16, u32, usize)> = Vec::new();
        for term in &expr.terms {
            match term {
                Term::Nested {
                    pc,
                    tag: 0x121,
                    expression,
                } => {
                    // The nested integer evaluator also updates result, condition and integer 4096.
                    let base = self.frame_base();
                    let image = self.images.get_mut(&self.slot).expect("active image");
                    let mut env = Env {
                        labels: &image.labels,
                        data: Some(&mut image.data),
                        frame: Some(Frame {
                            stack: &mut self.stack,
                            base,
                        }),
                    };
                    let index = self.state.evaluate_in(expression, &mut env, journal)?;
                    entries.push((0x121, index as u32 | 0x8000_0000, *pc));
                }
                Term::Scalar { pc, tag, raw } => entries.push((*tag, *raw, *pc)),
                Term::Operator { pc, opcode } if (0x160..=0x172).contains(opcode) => {
                    let top = entries
                        .pop()
                        .ok_or_else(|| error(*pc, *opcode, "expression stack underflow"))?;
                    let second = *entries
                        .last()
                        .ok_or_else(|| error(*pc, *opcode, "expression stack underflow"))?;
                    if *opcode == 0x170 {
                        self.append_operand(&self.state, &mut buffer, top)?;
                        self.assign_string(second, &buffer, journal)?;
                    } else {
                        // Binary operators only read both operands, so strings concatenate.
                        self.append_operand(&self.state, &mut buffer, second)?;
                        self.append_operand(&self.state, &mut buffer, top)?;
                    }
                    *entries.last_mut().expect("checked above") = (0, 0, *pc);
                }
                Term::Nested { pc, tag, .. }
                | Term::Indexed { pc, tag, .. }
                | Term::Operator { pc, opcode: tag } => {
                    return Err(error(*pc, *tag, "string expression term not implemented"));
                }
            }
        }
        let &(tag, value, pc) = entries
            .first()
            .ok_or_else(|| error(expr.pc, expr.kind, "empty string expression"))?;
        let result = match tag {
            0x120 => {
                c_string(self.image().text_bytes(), value, pc, tag)?;
                Some(value)
            }
            0x121 => Some(value),
            0x122 => Some(self.frame_word(value, pc, tag)?),
            0x125 => Some(value | 0x4000_0000),
            0x127 => Some(value | 0xc000_0000),
            _ => None, // The original leaves +82812 unchanged.
        };
        if result.is_some() {
            self.state.set_result_journaled(result, journal);
        }
        // This evaluator does not modify integer condition flags itself.
        self.state.set_string_buffer_journaled(buffer, journal);
        Ok(())
    }

    /// frame[depth]: 0x140080530 zeroes entry 0 with SP, and 0x0410 writes only depth >= 1.
    fn frame_base(&self) -> usize {
        self.frames.last().copied().unwrap_or(0)
    }
    /// Runs a numeric evaluation with the current labels and frame; rolls back on failure.
    fn numeric<T>(
        &mut self,
        run: impl FnOnce(&mut State, &mut Env, &mut Journal) -> Result<T, DecodeError>,
    ) -> Result<T, DecodeError> {
        let base = self.frame_base();
        let mut journal = Journal::default();
        let outcome = {
            let image = self.images.get_mut(&self.slot).expect("active image");
            let mut env = Env {
                labels: &image.labels,
                data: Some(&mut image.data),
                frame: Some(Frame {
                    stack: &mut self.stack,
                    base,
                }),
            };
            run(&mut self.state, &mut env, &mut journal)
        };
        if outcome.is_err() {
            self.state.rollback(
                journal,
                Some(&mut self.stack),
                Some(&mut self.images.get_mut(&self.slot).expect("active image").data),
            );
        }
        outcome
    }

    fn expression(&mut self, expr: &Expression) -> Result<(), DecodeError> {
        match expr.kind {
            0x200 => {
                self.numeric(|state, env, journal| state.evaluate_in(expr, env, journal))?;
            }
            0x201 => self.string_expression(expr)?,
            0x202 => {
                self.numeric(|state, env, journal| state.evaluate_float_in(expr, env, journal))?;
            }
            _ => {
                return Err(error(
                    expr.pc,
                    expr.kind,
                    "expression type/operation not implemented",
                ));
            }
        }
        Ok(())
    }

    fn enter_frame(&mut self, pc: usize, opcode: u16, return_pc: usize) -> Result<(), DecodeError> {
        if self.frames.len() >= MAX_CALL_DEPTH {
            return Err(error(pc, opcode, "call frame table holds 63 nested calls"));
        }
        self.stack
            .push_u32(return_pc as u32)
            .map_err(|e| error(pc, opcode, e.0))?;
        self.frames.push(self.stack.len_bytes());
        Ok(())
    }

    fn return_from_frame(
        &mut self,
        pc: usize,
        opcode: u16,
        arguments: u16,
    ) -> Result<usize, DecodeError> {
        if self.frames.is_empty() {
            return Err(error(pc, opcode, "return without a caller"));
        }
        let next = self.stack.peek_u32(4).map_err(|e| error(pc, opcode, e.0))? as usize;
        if !self.is_boundary(self.slot, next) {
            return Err(error(pc, opcode, "invalid return address"));
        }
        self.stack
            .discard(4 + usize::from(arguments))
            .map_err(|e| error(pc, opcode, e.0))?;
        self.frames.pop();
        Ok(next)
    }

    pub fn step(&mut self) -> Result<Step, DecodeError> {
        if let Some((pc, command)) = self.pending {
            return Ok(Step::System { pc, command });
        }
        if !self.is_boundary(self.slot, self.pc) {
            return Err(error(
                self.pc,
                0,
                "PC outside decoded instruction boundaries",
            ));
        }
        let insn = self
            .profile()
            .decode_at(self.image().script.code(), self.pc)?;
        let pc = insn.pc;
        let op = insn.opcode;
        let mut next = insn.end;
        match &insn.kind {
            InstructionKind::Expression(expr) => self.expression(expr)?,
            InstructionKind::Jump { target } | InstructionKind::LongJump { target, .. } => {
                next = self.branch_target(pc, op, *target)?;
            }
            InstructionKind::ConditionalJump { target } => {
                let condition = self
                    .state
                    .condition
                    .ok_or_else(|| error(pc, op, "condition flag is unknown"))?;
                if (op == 0x401 && !condition) || (op == 0x402 && condition) {
                    next = self.branch_target(pc, op, *target)?;
                }
            }
            InstructionKind::CompareJump { value, target } => {
                if self
                    .state
                    .result
                    .ok_or_else(|| error(pc, op, "result register is unknown"))?
                    == *value
                {
                    next = self.branch_target(pc, op, *target)?;
                }
            }
            InstructionKind::Label { index } => {
                next = self
                    .image()
                    .labels
                    .get(*index as usize)
                    .copied()
                    .ok_or_else(|| error(pc, op, "label index outside table"))?
                    as usize;
                if op == 0x410 {
                    self.enter_frame(pc, op, insn.end)?;
                }
            }
            InstructionKind::StackAdjust { bytes } if op == 0x412 => {
                self.stack
                    .discard(*bytes as usize)
                    .map_err(|e| error(pc, op, e.0))?;
            }
            InstructionKind::StackAdjust { bytes } if op == 0x411 => {
                next = self.return_from_frame(pc, op, *bytes)?;
            }
            InstructionKind::Simple if op == 0x413 => {
                let raw = self
                    .state
                    .result
                    .ok_or_else(|| error(pc, op, "result register is unknown"))?;
                next = self.return_from_frame(pc, op, (raw as u16).wrapping_mul(4))?;
            }
            InstructionKind::Simple if op == 0x416 => {
                let raw = self
                    .state
                    .result
                    .ok_or_else(|| error(pc, op, "result register is unknown"))?;
                next = self.branch_target(pc, op, raw)?;
                self.enter_frame(pc, op, insn.end)?;
            }
            InstructionKind::Simple if op == 0x430 => {
                let raw = self
                    .state
                    .result
                    .ok_or_else(|| error(pc, op, "result register is unknown"))?;
                self.stack.push_u32(raw).map_err(|e| error(pc, op, e.0))?;
            }
            InstructionKind::StackImmediate { bytes, raw } => {
                self.stack
                    .reserve(*raw, usize::from(*bytes))
                    .map_err(|e| error(pc, op, e.0))?;
            }
            InstructionKind::ScriptReturn { .. } => {
                let slot = self.stack.peek_u32(4).map_err(|e| error(pc, op, e.0))?;
                next = self.stack.peek_u32(8).map_err(|e| error(pc, op, e.0))? as usize;
                let repeat = self.stack.peek_u32(12).map_err(|e| error(pc, op, e.0))?;
                if slot > 3
                    || !self.is_boundary(slot as u8, next)
                    || (!self.cross_calls.is_empty() || !self.native_return_stack)
                        && self.cross_calls.last().copied() != Some(slot as u8)
                {
                    return Err(error(pc, op, "invalid cross-script return context"));
                }
                self.stack.discard(12).map_err(|e| error(pc, op, e.0))?;
                self.slot = slot as u8;
                self.repeat_count = repeat;
                self.cross_calls.pop();
                if self.stack.len_bytes() == 0 { self.native_return_stack = false; }
            }
            InstructionKind::System { command: 0 } => {
                self.pc = next;
                self.repeat_count = 0;
                return Ok(Step::Yield);
            }
            &InstructionKind::System { command } => {
                let command = match self.profile() {
                    VmProfile::Cvns20040515X86 => {
                        if let Some(&count)=super::cvns_scratch::ARGUMENT_WORDS.get(command as usize) {
                            for index in 0..usize::from(count) {
                                if let Ok(word)=self.stack.peek_u32(4*(index+1)) {
                                    self.cvns_arguments[index]=word;
                                }
                            }
                            // sub_42b310 decrements its signed exponent in the argument block.
                            if command==0xe6 && self.cvns_arguments[1] as i32>1 {
                                self.cvns_arguments[1]=1;
                            }
                        }
                        super::cvns::route(command)
                    },
                    _ => command,
                };
                self.pending = Some((pc, command));
                return Ok(Step::System { pc, command });
            }
            _ => return Err(error(pc, op, "instruction execution is not implemented")),
        }
        self.pc = next;
        Ok(Step::Continue)
    }

    fn branch_target(&self, pc: usize, op: u16, target: u32) -> Result<usize, DecodeError> {
        let target = target as usize;
        if !self.is_boundary(self.slot, target) {
            return Err(error(pc, op, "jump target is not an instruction boundary"));
        }
        Ok(target)
    }

    fn transfer_stack(
        &self,
        argument_bytes: usize,
        op: u16,
    ) -> Result<ParameterStack, DecodeError> {
        if !self.is_boundary(self.slot, self.pc + 2) {
            return Err(error(
                self.pc,
                op,
                "cross-script return target is not an instruction boundary",
            ));
        }
        if self.cross_calls.len() >= 256 {
            return Err(error(self.pc, op, "cross-script call research limit"));
        }
        let mut stack = self.stack.clone();
        stack
            .discard(argument_bytes)
            .map_err(|e| error(self.pc, op, e.0))?;
        for value in [
            self.repeat_count,
            (self.pc + 2) as u32,
            u32::from(self.slot),
        ] {
            stack.push_u32(value).map_err(|e| error(self.pc, op, e.0))?;
        }
        Ok(stack)
    }

    /// Complete successful 0x2081: initialize public slot+1, then return via 0x0414.
    pub fn enter_common(
        &mut self,
        slot: u32,
        PreparedScript(image): PreparedScript,
    ) -> Result<(), DecodeError> {
        // CVNS 0x42c3c0 consumes only the name word and always loads image slot 1.
        let arguments = match self.pending {
            Some((_, 0x81)) => 8,
            Some((_, 0x812)) if slot == 0 => 4,
            _ => return Err(error(self.pc, 0x2081, "no pending common-script load")),
        };
        if slot >= 3 {
            return Err(error(self.pc, 0x2081, "common slot must be 0..2"));
        }
        let slot = slot as u8 + 1;
        if self.slot == slot || self.cross_calls.contains(&slot) {
            return Err(error(
                self.pc,
                0x2081,
                "cannot replace an active common script",
            ));
        }
        let entry = image.script.entry_pc() as usize;
        let stack = self.transfer_stack(arguments, 0x2081)?;
        self.images.insert(slot, image);
        self.stack = stack;
        self.cross_calls.push(self.slot);
        self.slot = slot;
        self.pc = entry;
        self.repeat_count = 0;
        self.pending = None;
        Ok(())
    }

    pub fn enter_main(&mut self, PreparedScript(image): PreparedScript) -> Result<(), DecodeError> {
        if !matches!(self.pending, Some((_, 0x03 | 0x80 | 0x805 | 0x810))) {
            return Err(error(self.pc, 0x2080, "no pending main-script load"));
        }
        self.pc = image.script.entry_pc() as usize;
        self.images.insert(0, image);
        self.slot = 0;
        self.repeat_count = 0;
        self.stack.discard(self.stack.len_bytes()).map_err(|e| error(self.pc, 0x2080, e.0))?;
        self.frames.clear();
        self.cross_calls.clear();
        self.native_return_stack = false;
        self.pending = None;
        Ok(())
    }

    pub fn call_callback(&mut self, slot: u8, target: usize) -> Result<(), DecodeError> {
        if !matches!(self.pending, Some((_, 0x8a))) || !self.is_boundary(slot, target) {
            return Err(error(
                self.pc,
                0x208a,
                "invalid CMVS callback request or target",
            ));
        }
        let stack = self.transfer_stack(4, 0x208a)?;
        self.cross_calls.push(self.slot);
        self.activate_callback(slot, target, stack);
        Ok(())
    }

    pub fn dispatch_callbacks(&mut self, callbacks: &[(u8, usize)]) -> Result<(), DecodeError> {
        let Some((&(slot, target), previous)) = callbacks.split_last() else {
            return Ok(());
        };
        if !self.is_boundary(self.slot, self.pc)
            || callbacks
                .iter()
                .any(|&(slot, target)| !self.is_boundary(slot, target))
            || callbacks.len() + self.cross_calls.len() > 256
        {
            return Err(error(
                self.pc,
                0x2088,
                "invalid CMVS automatic callback context",
            ));
        }
        let mut stack = self.stack.clone();
        let mut calls = self.cross_calls.clone();
        let frames = std::iter::once((self.repeat_count, self.pc as u32, self.slot)).chain(
            previous
                .iter()
                .map(|&(slot, target)| (0, target as u32, slot)),
        );
        for (repeat, pc, saved_slot) in frames {
            for value in [repeat, pc, u32::from(saved_slot)] {
                stack
                    .push_u32(value)
                    .map_err(|e| error(self.pc, 0x2088, e.0))?;
            }
            calls.push(saved_slot);
        }
        self.cross_calls = calls;
        self.activate_callback(slot, target, stack);
        Ok(())
    }

    fn activate_callback(&mut self, slot: u8, target: usize, stack: ParameterStack) {
        self.slot = slot;
        self.pc = target;
        self.stack = stack;
        self.repeat_count = 0;
        self.pending = None;
    }

    pub fn wait_system(&mut self, command: u16) -> Result<(), DecodeError> {
        if !matches!(self.pending, Some((_, pending)) if pending == command) {
            return Err(error(
                self.pc,
                0x2000 + command,
                "no matching CMVS system wait",
            ));
        }
        self.repeat_count = self.repeat_count.wrapping_add(1);
        Ok(())
    }

    /// Resume an ordinary command after a host has explicitly implemented its effect.
    /// Script transfer requires its own future API; zero flags cannot be used to skip it.
    pub fn complete_system(&mut self, flags: SystemResult) -> Result<Step, DecodeError> {
        let (pc, command) = self
            .pending
            .ok_or_else(|| error(self.pc, 0, "no pending system command"))?;
        // CVNS 0x2010 and 0x2012 also install their image and PC directly.
        let script_transfer = matches!(command, 0x03 | 0x80 | 0x81 | 0x810 | 0x812);
        if !flags.advances_pc() || script_transfer {
            return Err(error(
                pc,
                command + 0x2000,
                "script transfer/retry needs a dedicated host implementation",
            ));
        }
        self.stack
            .apply_cleanup(flags)
            .map_err(|e| error(pc, command + 0x2000, e.0))?;
        self.pc = pc + 2;
        self.pending = None;
        self.repeat_count = if flags.increments_repeat_counter() {
            self.repeat_count.wrapping_add(1)
        } else {
            0
        };
        Ok(if flags.yields() {
            Step::Yield
        } else {
            Step::Continue
        })
    }
}

// SPDX-License-Identifier: MPL-2.0

//! Stateful integer and float evaluation with explicit unknown values and transactional errors.
use super::abi::{frame_offset, ParameterStack, StackWord};
use super::{DecodeError, Expression, Term};
use std::collections::BTreeMap;

/// Float variables flt_14012df00..0x14012ff00 hold 2048 entries.
const FLOAT_LIMIT: u32 = 2048;
const STRING_SLOT_BYTES: usize = 1024;
type StringCell = Box<[u8; STRING_SLOT_BYTES]>;

pub(crate) fn error(pc: usize, opcode: u16, reason: &'static str) -> DecodeError {
    DecodeError {
        pc,
        opcode: Some(opcode),
        reason,
    }
}

#[derive(Debug)]
enum Undo {
    Integer(u32, Option<i32>),
    Bit(u32, Option<bool>),
    String(u32, Option<StringCell>),
    Buffer(Vec<u8>),
    Result(Option<u32>),
    Condition(Option<bool>),
    Float(u32, Option<u32>),
    SystemInteger(usize, Option<u32>),
    SystemFloat(usize, Option<u32>),
    Stack(usize, StackWord),
    StackString(usize, Vec<Option<u8>>),
    Data(usize, [u8; 4]),
    DataString(usize, Vec<u8>),
}

/// Previous values recorded before each write, so a failed evaluation is undone
/// without cloning the whole State.
#[derive(Debug, Default)]
pub(crate) struct Journal(Vec<Undo>);

impl Journal {
    pub(super) fn stack_string(&mut self, offset: usize, bytes: Vec<Option<u8>>) {
        self.0.push(Undo::StackString(offset, bytes));
    }

    pub(super) fn data_string(&mut self, offset: usize, bytes: Vec<u8>) {
        self.0.push(Undo::DataString(offset, bytes));
    }
}

/// Stack bytes addressed by frame operands; `base` is frame[depth], zero at depth 0.
pub(crate) struct Frame<'s> {
    pub(crate) stack: &'s mut ParameterStack,
    pub(crate) base: usize,
}

/// Memory outside State that operands may address.
pub(crate) struct Env<'a, 's> {
    pub(crate) labels: &'a [u32],
    pub(crate) frame: Option<Frame<'s>>,
    pub(crate) data: Option<&'s mut [u8]>,
}

impl Env<'_, '_> {
    fn data_offset(&self, raw: u32, pc: usize, tag: u16) -> Result<usize, DecodeError> {
        let data = self
            .data
            .as_deref()
            .ok_or_else(|| error(pc, tag, "script data operand needs a machine image"))?;
        let offset = usize::try_from(raw as i32)
            .map_err(|_| error(pc, tag, "negative script data offset"))?;
        if offset.checked_add(4).is_none_or(|end| end > data.len()) {
            return Err(error(pc, tag, "script data word outside data segment"));
        }
        Ok(offset)
    }
    fn load_data(&self, offset: usize) -> i32 {
        i32::from_le_bytes(
            self.data.as_deref().expect("checked data")[offset..offset + 4]
                .try_into()
                .expect("checked word"),
        )
    }
    fn store_data(&mut self, offset: usize, value: i32, journal: &mut Journal) {
        let data = self.data.as_deref_mut().expect("checked data");
        let old = data[offset..offset + 4].try_into().expect("checked word");
        journal.0.push(Undo::Data(offset, old));
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    /// Tags 0x103/0x129 address base-raw (arguments); 0x108/0x12c address base+raw (locals).
    fn offset(&self, tag: u16, raw: u32, pc: usize) -> Result<usize, DecodeError> {
        let frame = self
            .frame
            .as_ref()
            .ok_or_else(|| error(pc, tag, "frame operand needs the machine stack"))?;
        frame_offset(frame.base, raw, matches!(tag, 0x103 | 0x129))
            .ok_or_else(|| error(pc, tag, "frame operand outside the stack"))
    }
    fn load(&self, offset: usize, pc: usize, op: u16) -> Result<u32, DecodeError> {
        let frame = self.frame.as_ref().expect("offset checked the frame");
        frame
            .stack
            .read_u32_at(offset)
            .map_err(|e| error(pc, op, e.0))
    }
    fn store(
        &mut self,
        offset: usize,
        value: u32,
        journal: &mut Journal,
        pc: usize,
        op: u16,
    ) -> Result<(), DecodeError> {
        let frame = self.frame.as_mut().expect("offset checked the frame");
        let old = frame
            .stack
            .write_u32_at(offset, value)
            .map_err(|e| error(pc, op, e.0))?;
        journal.0.push(Undo::Stack(offset, old));
        Ok(())
    }
}

/// cvttss2si truncation, including the x86 indefinite value for NaN and out-of-range input.
pub(crate) fn truncate(value: f32) -> i32 {
    truncate_double(f64::from(value))
}

/// CVTTSD2SI into a 32-bit destination.
pub(super) fn truncate_double(value: f64) -> i32 {
    if !(-2_147_483_648.0..2_147_483_648.0).contains(&value.trunc()) {
        i32::MIN
    } else {
        value as i32
    }
}

fn truncate_wide_low(value: f32) -> i32 {
    let value = f64::from(value);
    let wide = if (-9_223_372_036_854_775_808.0..9_223_372_036_854_775_808.0).contains(&value) {
        value as i64
    } else {
        i64::MIN
    };
    wide as i32
}

/// Sparse overrides on the native zero-filled physical variable banks.
#[derive(Debug, Clone)]
pub struct State {
    integers: BTreeMap<u32, i32>,
    bits: BTreeMap<u32, bool>,
    floats: BTreeMap<u32, u32>,
    strings: BTreeMap<u32, StringCell>,
    system_integers: [Option<u32>; 7],
    /// Float system result slots 7..10 (+82844..+82856) as raw f32 bits.
    system_float_bits: [Option<u32>; 4],
    integer_limit: u32,
    bit_limit: u32,
    string_buffer: Vec<u8>,
    pub result: Option<u32>,
    pub condition: Option<bool>,
}

impl State {
    pub fn new() -> Self {
        Self {
            integers: BTreeMap::new(),
            bits: BTreeMap::new(),
            floats: BTreeMap::new(),
            strings: BTreeMap::new(),
            system_integers: [None; 7],
            system_float_bits: [None; 4],
            integer_limit: 4097,
            bit_limit: 262144,
            string_buffer: Vec::new(),
            result: None,
            condition: None,
        }
    }
    pub fn integer(&self, index: u32) -> Option<i32> {
        self.integers
            .get(&index)
            .copied()
            .or_else(|| (index < 4096).then_some(0))
    }
    /// Integer system result slots +82816..+82840, distinct from expression result +82812.
    pub fn system_integer(&self, index: u32) -> Option<u32> {
        self.system_integers.get(index as usize).copied().flatten()
    }
    pub fn set_system_integer(&mut self, index: u32, value: u32) -> Result<(), &'static str> {
        let slot = self
            .system_integers
            .get_mut(index as usize)
            .ok_or("integer system result slot must be 0..6")?;
        *slot = Some(value);
        Ok(())
    }
    #[cfg(feature = "runtime")]
    pub(crate) fn forget_system_integer(&mut self, index: usize) {
        self.system_integers[index] = None;
    }
    /// Float system result slot 7..10 as raw f32 bits.
    pub fn system_float_bits(&self, index: u32) -> Option<u32> {
        self.system_float_bits
            .get((index as usize).wrapping_sub(7))
            .copied()
            .flatten()
    }
    pub fn set_system_float_bits(&mut self, index: u32, bits: u32) -> Result<(), &'static str> {
        let slot = self
            .system_float_bits
            .get_mut((index as usize).wrapping_sub(7))
            .ok_or("float system result slot must be 7..10")?;
        *slot = Some(bits);
        Ok(())
    }
    pub fn bit(&self, index: u32) -> Option<bool> {
        self.bits
            .get(&index)
            .copied()
            .or_else(|| (index < self.bit_limit).then_some(false))
    }
    pub fn float_bits(&self, index: u32) -> Option<u32> {
        self.floats
            .get(&index)
            .copied()
            .or_else(|| (index < FLOAT_LIMIT).then_some(0))
    }
    pub fn string(&self, index: u32) -> Option<&[u8]> {
        let cell = self.raw_string_cell(index)?;
        let len = cell
            .iter()
            .position(|&byte| byte == 0)
            .expect("terminated string slot");
        Some(&cell[..len])
    }
    pub(super) fn raw_string_cell(&self, index: u32) -> Option<&[u8; STRING_SLOT_BYTES]> {
        static EMPTY: [u8; STRING_SLOT_BYTES] = [0; STRING_SLOT_BYTES];
        (index < 128).then(|| self.strings.get(&index).map_or(&EMPTY, Box::as_ref))
    }
    pub(crate) fn set_string_buffer_journaled(&mut self, bytes: Vec<u8>, journal: &mut Journal) {
        let old = std::mem::replace(&mut self.string_buffer, bytes);
        journal.0.push(Undo::Buffer(old));
    }
    pub(crate) fn set_result_journaled(&mut self, value: Option<u32>, journal: &mut Journal) {
        journal.0.push(Undo::Result(self.result));
        self.result = value;
    }
    pub(crate) fn set_string_journaled(
        &mut self,
        index: u32,
        bytes: &[u8],
        journal: &mut Journal,
    ) -> Result<(), &'static str> {
        Self::range(index, 1, 128)?;
        if bytes.len() >= STRING_SLOT_BYTES {
            return Err("invalid bounded string slot value");
        }
        let mut cell = Box::new(*self.raw_string_cell(index).expect("validated slot"));
        cell[..bytes.len()].copy_from_slice(bytes);
        cell[bytes.len()] = 0;
        let old = self.strings.insert(index, cell);
        journal.0.push(Undo::String(index, old));
        Ok(())
    }
    fn set_integer_journaled(
        &mut self,
        index: u32,
        value: i32,
        journal: &mut Journal,
    ) -> Result<(), &'static str> {
        if index >= self.integer_limit {
            return Err("integer variable outside research limit");
        }
        let old = self.integers.insert(index, value);
        journal.0.push(Undo::Integer(index, old));
        Ok(())
    }
    fn set_bit_journaled(
        &mut self,
        index: u32,
        value: bool,
        journal: &mut Journal,
    ) -> Result<(), &'static str> {
        if index >= self.bit_limit {
            return Err("bit variable outside research limit");
        }
        let old = self.bits.insert(index, value);
        journal.0.push(Undo::Bit(index, old));
        Ok(())
    }
    fn set_float_journaled(
        &mut self,
        index: u32,
        bits: u32,
        journal: &mut Journal,
    ) -> Result<(), &'static str> {
        if index >= FLOAT_LIMIT {
            return Err("float variable outside research limit");
        }
        let old = self.floats.insert(index, bits);
        journal.0.push(Undo::Float(index, old));
        Ok(())
    }
    /// Restores every journaled write in reverse order; stack words need the same stack.
    pub(crate) fn rollback(
        &mut self,
        journal: Journal,
        mut stack: Option<&mut ParameterStack>,
        mut data: Option<&mut [u8]>,
    ) {
        fn restore<V>(map: &mut BTreeMap<u32, V>, index: u32, old: Option<V>) {
            match old {
                Some(value) => map.insert(index, value),
                None => map.remove(&index),
            };
        }
        for undo in journal.0.into_iter().rev() {
            match undo {
                Undo::Integer(index, old) => restore(&mut self.integers, index, old),
                Undo::Bit(index, old) => restore(&mut self.bits, index, old),
                Undo::String(index, old) => restore(&mut self.strings, index, old),
                Undo::Float(index, old) => restore(&mut self.floats, index, old),
                Undo::SystemInteger(index, old) => self.system_integers[index] = old,
                Undo::SystemFloat(index, old) => self.system_float_bits[index] = old,
                Undo::Stack(offset, old) => {
                    let stack = stack.as_deref_mut().expect("stack writes need their stack");
                    stack.restore_at(offset, old);
                }
                Undo::StackString(offset, old) => {
                    stack
                        .as_deref_mut()
                        .expect("stack writes need their stack")
                        .restore_bytes(offset, &old);
                }
                Undo::Data(offset, old) => {
                    data.as_deref_mut().expect("data writes need their image")[offset..offset + 4]
                        .copy_from_slice(&old)
                }
                Undo::DataString(offset, old) => {
                    data.as_deref_mut().expect("data writes need their image")
                        [offset..offset + old.len()]
                        .copy_from_slice(&old);
                }
                Undo::Buffer(old) => self.string_buffer = old,
                Undo::Result(old) => self.result = old,
                Undo::Condition(old) => self.condition = old,
            }
        }
    }
    fn range(start: u32, count: u32, limit: u32) -> Result<std::ops::Range<u32>, &'static str> {
        let end = start.checked_add(count).ok_or("variable range overflow")?;
        if end > limit {
            return Err("variable range outside research limit");
        }
        Ok(start..end)
    }
    pub fn fill_integers(
        &mut self,
        start: u32,
        count: u32,
        value: i32,
    ) -> Result<(), &'static str> {
        for index in Self::range(start, count, 4096)? {
            self.integers.insert(index, value);
        }
        Ok(())
    }
    pub fn fill_bits(&mut self, start: u32, count: u32, value: bool) -> Result<(), &'static str> {
        let range = Self::range(start, count, self.bit_limit)?;
        if value {
            for index in range {
                self.bits.insert(index, true);
            }
        } else {
            self.bits.retain(|index, _| !range.contains(index));
        }
        Ok(())
    }

    #[cfg(feature = "runtime")]
    pub(super) fn snapshot(&self) -> serde_json::Value {
        use super::snapshot::hex;
        let bits: Vec<u32> = self.bits.iter().filter(|(_, set)| **set).map(|(at, _)| *at).collect();
        let strings: Vec<_> = self
            .strings
            .iter()
            .map(|(at, cell)| {
                let end = cell.iter().position(|&byte| byte == 0).unwrap_or(cell.len());
                serde_json::json!([at, hex(&cell[..end])])
            })
            .collect();
        serde_json::json!({
            "integers": self.integers.iter().collect::<Vec<_>>(),
            "bits": bits,
            "floats": self.floats.iter().collect::<Vec<_>>(),
            "strings": strings,
            "system": self.system_integers,
            "system_floats": self.system_float_bits,
            "integer_limit": self.integer_limit,
            "bit_limit": self.bit_limit,
            "buffer": hex(&self.string_buffer),
            "result": self.result,
            "condition": self.condition,
        })
    }

    #[cfg(feature = "runtime")]
    pub(super) fn from_snapshot(value: &serde_json::Value) -> Option<Self> {
        use super::snapshot::{bytes, unhex, word, words};
        let mut state = Self::new();
        for pair in value["integers"].as_array()? {
            state.integers.insert(pair[0].as_u64()? as u32, pair[1].as_i64()? as i32);
        }
        for bit in words(value, "bits")? {
            state.bits.insert(bit, true);
        }
        for pair in value["floats"].as_array()? {
            state.floats.insert(pair[0].as_u64()? as u32, pair[1].as_u64()? as u32);
        }
        for pair in value["strings"].as_array()? {
            let text = unhex(pair[1].as_str()?)?;
            state.fill_strings(pair[0].as_u64()? as u32, 1, &text).ok()?;
        }
        for (slot, saved) in state.system_integers.iter_mut().zip(value["system"].as_array()?) {
            *slot = saved.as_u64().map(|word| word as u32);
        }
        for (slot, saved) in state.system_float_bits.iter_mut().zip(value["system_floats"].as_array()?) {
            *slot = saved.as_u64().map(|word| word as u32);
        }
        state.integer_limit = word(value, "integer_limit")?;
        state.bit_limit = word(value, "bit_limit")?;
        state.string_buffer = bytes(value, "buffer")?;
        state.result = value["result"].as_u64().map(|word| word as u32);
        state.condition = value["condition"].as_bool();
        Some(state)
    }

    /// The persistent banks in CSS1 order: bits, integers, floats, then 64 C strings.
    pub(super) fn system_image(&self) -> Vec<u8> {
        let mut image = vec![0; 44800];
        for (&index, _) in self.bits.iter().filter(|(index, set)| **set && **index >= 2048) {
            let offset = index as usize - 2048;
            if offset < 32512 * 8 {
                image[offset / 8] |= 1 << (offset % 8);
            }
        }
        for slot in 0..2048 {
            let value = self.integers.get(&(2048 + slot as u32)).copied().unwrap_or(0);
            image[32512 + slot * 4..32516 + slot * 4].copy_from_slice(&value.to_le_bytes());
        }
        for slot in 0..1024 {
            let value = self.floats.get(&(1024 + slot as u32)).copied().unwrap_or(0);
            image[40704 + slot * 4..40708 + slot * 4].copy_from_slice(&value.to_le_bytes());
        }
        for slot in 64..128 {
            if let Some(cell) = self.strings.get(&slot) {
                let end = cell.iter().position(|&byte| byte == 0).unwrap_or(cell.len());
                image.extend_from_slice(&cell[..end]);
            }
            image.push(0);
        }
        image
    }

    pub fn restore_bits(&mut self, start: u32, packed: &[u8]) -> Result<(), &'static str> {
        let count = u32::try_from(packed.len())
            .ok()
            .and_then(|len| len.checked_mul(8))
            .ok_or("packed bit range overflow")?;
        self.fill_bits(start, count, false)?;
        for (index, &value) in packed.iter().enumerate() {
            for bit in 0..8 {
                if value & (1 << bit) != 0 {
                    self.bits.insert(start + (index as u32 * 8) + bit, true);
                }
            }
        }
        Ok(())
    }
    pub fn fill_floats(&mut self, start: u32, count: u32, bits: u32) -> Result<(), &'static str> {
        // Research capacity; the model preserves raw f32 bits, including NaN payloads.
        for index in Self::range(start, count, 2048)? {
            self.floats.insert(index, bits);
        }
        Ok(())
    }
    pub fn fill_strings(
        &mut self,
        start: u32,
        count: u32,
        bytes: &[u8],
    ) -> Result<(), &'static str> {
        if bytes.len() >= STRING_SLOT_BYTES || bytes.contains(&0) {
            return Err("invalid bounded string slot value");
        }
        for index in Self::range(start, count, 128)? {
            let cell = self
                .strings
                .entry(index)
                .or_insert_with(|| Box::new([0; STRING_SLOT_BYTES]));
            cell[..bytes.len()].copy_from_slice(bytes);
            cell[bytes.len()] = 0;
        }
        Ok(())
    }
    /// Evaluation inside a larger transaction; the caller owns rollback.
    pub(crate) fn evaluate_in(
        &mut self,
        expr: &Expression,
        env: &mut Env,
        journal: &mut Journal,
    ) -> Result<i32, DecodeError> {
        self.evaluate(expr, 0, env, journal)
    }
    fn read(&self, cell: Cell, env: &Env, pc: usize, op: u16) -> Result<i32, DecodeError> {
        match cell {
            Cell::Value(value) => Ok(value),
            Cell::Label(index) => env
                .labels
                .get(index as usize)
                .map(|&target| target as i32)
                .ok_or_else(|| error(pc, op, "label operand index out of bounds")),
            Cell::System(index) => match index {
                0..=6 => self
                    .system_integer(index)
                    .map(|value| value as i32)
                    .ok_or_else(|| error(pc, op, "unknown integer system result")),
                7..=10 => self
                    .system_float_bits(index)
                    .map(|bits| truncate(f32::from_bits(bits)))
                    .ok_or_else(|| error(pc, op, "unknown float system result")),
                _ => Ok(0),
            },
            Cell::FrameFloat(offset) => Ok(truncate(f32::from_bits(env.load(offset, pc, op)?))),
            Cell::DataFloat(offset) => Ok(truncate(f32::from_bits(env.load_data(offset) as u32))),
            Cell::Float(index) => self
                .float_bits(index)
                .map(|bits| truncate(f32::from_bits(bits)))
                .ok_or_else(|| error(pc, op, "unknown float variable")),
            Cell::Frame(offset) => Ok(env.load(offset, pc, op)? as i32),
            Cell::Data(offset) => Ok(env.load_data(offset)),
            Cell::Integer(index) => self
                .integer(index)
                .ok_or_else(|| error(pc, op, "read of unknown integer variable")),
            Cell::Bit(index) => self
                .bit(index)
                .map(i32::from)
                .ok_or_else(|| error(pc, op, "read of unknown bit variable")),
        }
    }
    fn write(
        &mut self,
        cell: Cell,
        value: i32,
        env: &mut Env,
        pc: usize,
        op: u16,
        journal: &mut Journal,
    ) -> Result<(), DecodeError> {
        match cell {
            Cell::Data(offset) => {
                env.store_data(offset, value, journal);
                return Ok(());
            }
            Cell::Frame(offset) => return env.store(offset, value as u32, journal, pc, op),
            Cell::Integer(index) => self.set_integer_journaled(index, value, journal),
            Cell::Bit(index) => self.set_bit_journaled(index, value != 0, journal),
            Cell::System(index @ 0..=6) => {
                let slot = &mut self.system_integers[index as usize];
                journal.0.push(Undo::SystemInteger(index as usize, *slot));
                *slot = Some(value as u32);
                Ok(())
            }
            // The native integer writer has no case for these descriptors.
            Cell::Value(_)
            | Cell::Label(_)
            | Cell::System(_)
            | Cell::Float(_)
            | Cell::FrameFloat(_)
            | Cell::DataFloat(_) => Ok(()),
        }
        .map_err(|why| error(pc, op, why))
    }

    fn indexed_offset(
        &mut self,
        base: u32,
        stride: u16,
        indices: &[Expression],
        depth: usize,
        env: &mut Env,
        journal: &mut Journal,
    ) -> Result<u32, DecodeError> {
        // The decoder guarantees one or two integer indices, left to right.
        let first = self.evaluate(&indices[0], depth, env, journal)? as u32;
        let mut offset = base.wrapping_add(first.wrapping_mul(u32::from(stride)));
        if let Some(second) = indices.get(1) {
            let second = self.evaluate(second, depth, env, journal)? as u32;
            offset = offset.wrapping_add(second.wrapping_mul(4));
        }
        Ok(offset)
    }
    fn evaluate(
        &mut self,
        expr: &Expression,
        depth: usize,
        env: &mut Env,
        journal: &mut Journal,
    ) -> Result<i32, DecodeError> {
        if depth >= 64 {
            return Err(error(
                expr.pc,
                expr.kind,
                "expression evaluation depth limit",
            ));
        }
        if expr.kind != 0x200 {
            return Err(error(expr.pc, expr.kind, "not an integer expression"));
        }
        let mut stack = Vec::new();
        for term in &expr.terms {
            match term {
                Term::Scalar {
                    tag: 0x100, raw, ..
                } => stack.push(Cell::Value(*raw as i32)),
                Term::Scalar {
                    tag: 0x10e, raw, ..
                } => stack.push(Cell::Label(*raw)),
                Term::Scalar {
                    tag: 0x10f, raw, ..
                } => stack.push(Cell::System(*raw)),
                Term::Scalar {
                    tag: 0x12a, raw, ..
                } => {
                    stack.push(Cell::Value(truncate_wide_low(f32::from_bits(*raw))));
                }
                Term::Scalar { pc, tag, raw } => {
                    stack.push(match tag {
                        0x103 | 0x108 | 0x10a => Cell::Frame(env.offset(*tag, *raw, *pc)?),
                        0x104 | 0x106 => Cell::Data(env.data_offset(*raw, *pc, *tag)?),
                        0x129 | 0x12c => Cell::FrameFloat(env.offset(*tag, *raw, *pc)?),
                        0x12e => Cell::DataFloat(env.data_offset(*raw, *pc, *tag)?),
                        _ => {
                            return Err(error(
                                *pc,
                                *tag,
                                "operand storage/type is not implemented",
                            ));
                        }
                    });
                }
                Term::Indexed {
                    pc,
                    tag,
                    base,
                    stride,
                    indices,
                } => {
                    let raw =
                        self.indexed_offset(*base, *stride, indices, depth + 1, env, journal)?;
                    stack.push(match tag {
                        0x105 | 0x107 | 0x110 | 0x111 => {
                            Cell::Data(env.data_offset(raw, *pc, *tag)?)
                        }
                        0x109 | 0x10b | 0x112 | 0x113 => Cell::Frame(env.offset(0x108, raw, *pc)?),
                        0x12d => Cell::FrameFloat(env.offset(0x12c, raw, *pc)?),
                        0x12f => Cell::DataFloat(env.data_offset(raw, *pc, *tag)?),
                        _ => return Err(error(*pc, *tag, "unsupported integer array descriptor")),
                    });
                }
                Term::Nested {
                    pc,
                    tag,
                    expression,
                } if matches!(tag, 0x200 | 0x101 | 0x102 | 0x12b) => {
                    let value = self.evaluate(expression, depth + 1, env, journal)?;
                    let cell = match tag {
                        0x200 => Cell::Value(value),
                        0x101 if value >= 0 && (value as u32) < self.integer_limit => {
                            Cell::Integer(value as u32)
                        }
                        0x102 if value >= 0 && (value as u32) < self.bit_limit => {
                            Cell::Bit(value as u32)
                        }
                        0x12b if value >= 0 && (value as u32) < FLOAT_LIMIT => {
                            Cell::Float(value as u32)
                        }
                        _ => return Err(error(*pc, *tag, "variable index outside research limit")),
                    };
                    stack.push(cell);
                }
                Term::Operator {
                    pc,
                    opcode: op @ (0x173 | 0x174),
                } => {
                    let cell = stack
                        .pop()
                        .ok_or_else(|| error(*pc, *op, "expression stack underflow"))?;
                    let old = self.read(cell, env, *pc, *op)?;
                    let value = if *op == 0x173 {
                        old.wrapping_add(1)
                    } else {
                        old.wrapping_sub(1)
                    };
                    self.write(cell, value, env, *pc, *op, journal)?;
                    stack.push(Cell::Value(value));
                }
                Term::Operator { pc, opcode } => {
                    let right_cell = stack
                        .pop()
                        .ok_or_else(|| error(*pc, *opcode, "expression stack underflow"))?;
                    let left_cell = stack
                        .pop()
                        .ok_or_else(|| error(*pc, *opcode, "expression stack underflow"))?;
                    let value = if *opcode == 0x170 {
                        let right = self.read(right_cell, env, *pc, *opcode)?;
                        self.write(left_cell, right, env, *pc, *opcode, journal)?;
                        right
                    } else {
                        // Preserve descriptor laziness: nested writes may change either source.
                        let left = self.read(left_cell, env, *pc, *opcode)?;
                        let right = self.read(right_cell, env, *pc, *opcode)?;
                        let binary = match opcode {
                            0x175 => 0x163,
                            0x176 => 0x164,
                            0x177 => 0x160,
                            0x178 => 0x161,
                            0x179 => 0x165,
                            0x17a => 0x166,
                            0x17b => 0x162,
                            0x17c => 0x167,
                            0x17d => 0x168,
                            0x17e => 0x169,
                            other => *other,
                        };
                        let value = binary_value(*pc, binary, left, right)?;
                        if (0x175..=0x17e).contains(opcode) {
                            self.write(left_cell, value, env, *pc, *opcode, journal)?;
                        }
                        value
                    };
                    stack.push(Cell::Value(value));
                }
                Term::Nested { pc, tag, .. } => {
                    return Err(error(*pc, *tag, "unsupported nested integer descriptor"));
                }
            }
        }
        if stack.len() != 1 {
            return Err(error(
                expr.pc,
                expr.kind,
                "expression does not produce exactly one value",
            ));
        }
        let value = self.read(stack[0], env, expr.pc, expr.kind)?;
        // The original integer evaluator writes its result to global integer 4096.
        self.set_integer_journaled(4096, value, journal)
            .map_err(|why| error(expr.pc, expr.kind, why))?;
        self.set_result_journaled(Some(value as u32), journal);
        journal.0.push(Undo::Condition(self.condition));
        self.condition = Some(value != 0);
        Ok(value)
    }

    /// Operand reader 0x1400659b0: integer storage converts to float.
    fn read_float(
        &self,
        cell: FloatCell,
        env: &Env,
        pc: usize,
        op: u16,
    ) -> Result<f32, DecodeError> {
        Ok(match cell {
            FloatCell::Value(value) => value,
            FloatCell::Integer(index) => self
                .integer(index)
                .ok_or_else(|| error(pc, op, "read of unknown integer variable"))?
                as f32,
            FloatCell::Bit(index) => f32::from(u8::from(
                self.bit(index)
                    .ok_or_else(|| error(pc, op, "read of unknown bit variable"))?,
            )),
            FloatCell::Float(index) => f32::from_bits(
                self.float_bits(index)
                    .ok_or_else(|| error(pc, op, "read of unknown float variable"))?,
            ),
            FloatCell::FrameInt(offset) => env.load(offset, pc, op)? as i32 as f32,
            FloatCell::FrameFloat(offset) => f32::from_bits(env.load(offset, pc, op)?),
            FloatCell::DataInt(offset) => env.load_data(offset) as f32,
            FloatCell::DataFloat(offset) => f32::from_bits(env.load_data(offset) as u32),
            FloatCell::Label(index) => *env
                .labels
                .get(index as usize)
                .ok_or_else(|| error(pc, op, "label operand index out of bounds"))?
                as f32,
            FloatCell::System(index) => match index {
                0..=6 => self
                    .system_integer(index)
                    .ok_or_else(|| error(pc, op, "unknown integer system result"))?
                    as i32 as f32,
                7..=10 => f32::from_bits(
                    self.system_float_bits(index)
                        .ok_or_else(|| error(pc, op, "unknown float system result"))?,
                ),
                _ => 0.0,
            },
        })
    }
    /// Operand writer 0x140065f00: integer storage receives the truncated value.
    fn write_float(
        &mut self,
        cell: FloatCell,
        value: f32,
        env: &mut Env,
        pc: usize,
        op: u16,
        journal: &mut Journal,
    ) -> Result<(), DecodeError> {
        match cell {
            FloatCell::Integer(index) => {
                self.set_integer_journaled(index, truncate(value), journal)
            }
            FloatCell::Bit(index) => self.set_bit_journaled(index, truncate(value) != 0, journal),
            FloatCell::Float(index) => self.set_float_journaled(index, value.to_bits(), journal),
            FloatCell::System(index) => {
                match index {
                    0..=6 => {
                        let slot = &mut self.system_integers[index as usize];
                        journal.0.push(Undo::SystemInteger(index as usize, *slot));
                        *slot = Some(truncate(value) as u32);
                    }
                    7..=10 => {
                        let index = (index - 7) as usize;
                        let slot = &mut self.system_float_bits[index];
                        journal.0.push(Undo::SystemFloat(index, *slot));
                        *slot = Some(value.to_bits());
                    }
                    _ => {}
                }
                Ok(())
            }
            FloatCell::FrameInt(offset) => {
                return env.store(offset, truncate_wide_low(value) as u32, journal, pc, op);
            }
            FloatCell::FrameFloat(offset) => {
                return env.store(offset, value.to_bits(), journal, pc, op);
            }
            FloatCell::DataInt(offset) => {
                env.store_data(offset, truncate_wide_low(value), journal);
                Ok(())
            }
            FloatCell::DataFloat(offset) => {
                env.store_data(offset, value.to_bits() as i32, journal);
                Ok(())
            }
            FloatCell::Value(_) | FloatCell::Label(_) => Ok(()),
        }
        .map_err(|why| error(pc, op, why))
    }
    /// 0x0202 / 0x140066e60: float postfix evaluation; result bits go to +82812 and the
    /// condition bit to `result != 0.0`. Integer 4096 changes only through nested integers.
    pub(crate) fn evaluate_float_in(
        &mut self,
        expr: &Expression,
        env: &mut Env,
        journal: &mut Journal,
    ) -> Result<f32, DecodeError> {
        if expr.kind != 0x202 {
            return Err(error(expr.pc, expr.kind, "not a float expression"));
        }
        let mut stack = Vec::new();
        for term in &expr.terms {
            match term {
                Term::Scalar {
                    tag: 0x100, raw, ..
                } => stack.push(FloatCell::Value(*raw as i32 as f32)),
                Term::Scalar {
                    tag: 0x12a, raw, ..
                } => stack.push(FloatCell::Value(f32::from_bits(*raw))),
                Term::Scalar {
                    tag: 0x10e, raw, ..
                } => stack.push(FloatCell::Label(*raw)),
                Term::Scalar {
                    tag: 0x10f, raw, ..
                } => stack.push(FloatCell::System(*raw)),
                Term::Scalar { pc, tag, raw } => {
                    stack.push(match tag {
                        0x103 | 0x108 | 0x10a => FloatCell::FrameInt(env.offset(*tag, *raw, *pc)?),
                        0x104 | 0x106 => FloatCell::DataInt(env.data_offset(*raw, *pc, *tag)?),
                        0x129 | 0x12c | 0x132 => {
                            FloatCell::FrameFloat(env.offset(*tag, *raw, *pc)?)
                        }
                        0x12e | 0x130 => FloatCell::DataFloat(env.data_offset(*raw, *pc, *tag)?),
                        _ => {
                            return Err(error(
                                *pc,
                                *tag,
                                "float operand storage/type is not implemented",
                            ));
                        }
                    });
                }
                Term::Indexed {
                    pc,
                    tag,
                    base,
                    stride,
                    indices,
                } => {
                    let raw = self.indexed_offset(*base, *stride, indices, 1, env, journal)?;
                    stack.push(match tag {
                        0x105 | 0x107 | 0x110 | 0x111 => {
                            FloatCell::DataInt(env.data_offset(raw, *pc, *tag)?)
                        }
                        0x109 | 0x10b | 0x112 | 0x113 => {
                            FloatCell::FrameInt(env.offset(0x108, raw, *pc)?)
                        }
                        0x12d | 0x133 | 0x136 | 0x137 => {
                            FloatCell::FrameFloat(env.offset(0x12c, raw, *pc)?)
                        }
                        0x12f | 0x131 | 0x134 | 0x135 => {
                            FloatCell::DataFloat(env.data_offset(raw, *pc, *tag)?)
                        }
                        _ => return Err(error(*pc, *tag, "unsupported float array descriptor")),
                    });
                }
                Term::Nested {
                    pc,
                    tag,
                    expression,
                } if matches!(tag, 0x200 | 0x101 | 0x102 | 0x12b) => {
                    let value = self.evaluate(expression, 1, env, journal)?;
                    let index = value as u32;
                    stack.push(match tag {
                        0x200 => FloatCell::Value(value as f32),
                        0x101 if value >= 0 && index < self.integer_limit => {
                            FloatCell::Integer(index)
                        }
                        0x102 if value >= 0 && index < self.bit_limit => FloatCell::Bit(index),
                        0x12b if value >= 0 && index < FLOAT_LIMIT => FloatCell::Float(index),
                        _ => return Err(error(*pc, *tag, "variable index outside research limit")),
                    });
                }
                Term::Operator {
                    pc,
                    opcode: op @ (0x173 | 0x174),
                } => {
                    let cell = stack
                        .pop()
                        .ok_or_else(|| error(*pc, *op, "expression stack underflow"))?;
                    let old = self.read_float(cell, env, *pc, *op)?;
                    let value = if *op == 0x173 { old + 1.0 } else { old - 1.0 };
                    self.write_float(cell, value, env, *pc, *op, journal)?;
                    stack.push(FloatCell::Value(value));
                }
                Term::Operator { pc, opcode } => {
                    let right_cell = stack
                        .pop()
                        .ok_or_else(|| error(*pc, *opcode, "expression stack underflow"))?;
                    let left_cell = stack
                        .pop()
                        .ok_or_else(|| error(*pc, *opcode, "expression stack underflow"))?;
                    let value = if *opcode == 0x170 {
                        let right = self.read_float(right_cell, env, *pc, *opcode)?;
                        self.write_float(left_cell, right, env, *pc, *opcode, journal)?;
                        right
                    } else if (0x175..=0x17e).contains(opcode) {
                        // The compound path reads the right operand first.
                        let right = self.read_float(right_cell, env, *pc, *opcode)?;
                        let left = self.read_float(left_cell, env, *pc, *opcode)?;
                        let binary = match opcode {
                            0x175 => 0x163,
                            0x176 => 0x164,
                            0x177 => 0x160,
                            0x178 => 0x161,
                            0x179 => 0x165,
                            0x17a => 0x166,
                            0x17b => 0x162,
                            0x17c => 0x167,
                            0x17d => 0x168,
                            _ => 0x169,
                        };
                        let value = float_binary(*pc, binary, left, right)?;
                        self.write_float(left_cell, value, env, *pc, *opcode, journal)?;
                        value
                    } else {
                        let left = self.read_float(left_cell, env, *pc, *opcode)?;
                        let right = self.read_float(right_cell, env, *pc, *opcode)?;
                        float_binary(*pc, *opcode, left, right)?
                    };
                    stack.push(FloatCell::Value(value));
                }
                Term::Nested { pc, tag, .. } => {
                    return Err(error(*pc, *tag, "unsupported nested float descriptor"));
                }
            }
        }
        if stack.len() != 1 {
            return Err(error(
                expr.pc,
                expr.kind,
                "expression does not produce exactly one value",
            ));
        }
        let value = self.read_float(stack[0], env, expr.pc, expr.kind)?;
        self.set_result_journaled(Some(value.to_bits()), journal);
        journal.0.push(Undo::Condition(self.condition));
        self.condition = Some(value != 0.0);
        Ok(value)
    }
}

#[derive(Clone, Copy)]
enum FloatCell {
    Value(f32),
    Integer(u32),
    Bit(u32),
    Float(u32),
    System(u32),
    FrameInt(usize),
    FrameFloat(usize),
    DataInt(usize),
    DataFloat(usize),
    Label(u32),
}

/// Float operators of 0x140066e60; bitwise, remainder and shifts truncate both sides first.
fn float_binary(pc: usize, opcode: u16, left: f32, right: f32) -> Result<f32, DecodeError> {
    let flag = |value: bool| f32::from(u8::from(value));
    Ok(match opcode {
        0x160 => left * right,
        0x161 => left / right,
        0x163 => left + right,
        0x164 => left - right,
        0x162 | 0x165..=0x169 => binary_value(pc, opcode, truncate(left), truncate(right))? as f32,
        0x16a => flag(left > right),
        0x16b => flag(left >= right),
        0x16c => flag(left < right),
        0x16d => flag(left <= right),
        0x16e => flag(left != 0.0 && right != 0.0),
        0x16f => flag(left != 0.0 || right != 0.0),
        0x171 => flag(left == right),
        0x172 => flag(left != right),
        _ => return Err(error(pc, opcode, "unsupported float operator")),
    })
}

#[derive(Clone, Copy)]
enum Cell {
    Value(i32),
    Label(u32),
    System(u32),
    Float(u32),
    FrameFloat(usize),
    DataFloat(usize),
    Integer(u32),
    Bit(u32),
    /// Absolute stack byte offset of a frame word.
    Frame(usize),
    Data(usize),
}

pub(crate) fn binary_value(
    pc: usize,
    opcode: u16,
    left: i32,
    right: i32,
) -> Result<i32, DecodeError> {
    Ok(match opcode {
        0x160 => left.wrapping_mul(right),
        0x161 => left
            .checked_div(right)
            .ok_or_else(|| error(pc, opcode, "invalid signed division"))?,
        0x162 => left
            .checked_rem(right)
            .ok_or_else(|| error(pc, opcode, "invalid signed remainder"))?,
        0x163 => left.wrapping_add(right),
        0x164 => left.wrapping_sub(right),
        0x165 => left & right,
        0x166 => left | right,
        0x167 => left ^ right,
        0x168 => left.wrapping_shl(right as u32),
        0x169 => left.wrapping_shr(right as u32),
        0x16a => i32::from(left > right),
        0x16b => i32::from(left >= right),
        0x16c => i32::from(left < right),
        0x16d => i32::from(left <= right),
        0x16e => i32::from(left != 0 && right != 0),
        0x16f => i32::from(left != 0 || right != 0),
        0x171 => i32::from(left == right),
        0x172 => i32::from(left != right),
        _ => return Err(error(pc, opcode, "unsupported integer operator")),
    })
}

// SPDX-License-Identifier: MPL-2.0
//! Lua 5.3 bytecode interpreter. Lua-to-Lua calls use an explicit frame stack;
//! host and built-in functions run to completion and may re-enter `call`.
use crate::chunk::{Constant, Op, Proto};
use crate::stdlib::{self, Builtin};
use crate::value::{Arena, Cell, FuncRef, Function, Table, TableRef, Value};
use anyhow::{bail, ensure, Context, Result};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Host bindings: functions created with `Lua::host_function` and userdata
/// values created by the host are dispatched here.
pub trait Host {
    fn call(&mut self, lua: &mut Lua, id: u32, args: Vec<Value>) -> Result<Vec<Value>>;
    fn index(&mut self, lua: &mut Lua, object: u32, key: &Value) -> Result<Value> {
        let _ = (lua, key);
        bail!("userdata {object} is not indexable")
    }
    fn new_index(&mut self, lua: &mut Lua, object: u32, key: &Value, value: Value) -> Result<()> {
        let _ = (lua, key, value);
        bail!("userdata {object} is not assignable")
    }
    /// Sink for `print`.
    fn print(&mut self, text: &[u8]) {
        let _ = text;
    }
}

struct Frame {
    func: FuncRef,
    proto: Arc<Proto>,
    upvalues: Vec<u32>,
    base: usize,
    pc: usize,
    varargs: Vec<Value>,
    /// Absolute stack slot receiving results and the wanted count (-1 = all).
    dest: usize,
    wanted: i32,
}

const MAX_FRAMES: usize = 220;
const MAX_STACK: usize = 1 << 20;

pub struct Lua {
    pub tables: Arena<Table>,
    pub funcs: Arena<Function>,
    pub cells: Arena<Cell>,
    stack: Vec<Value>,
    top: usize,
    frames: Vec<Frame>,
    open: BTreeMap<usize, u32>,
    pub globals: TableRef,
    pub string_lib: TableRef,
    /// Values the host keeps alive across collections.
    pub registry: Vec<Value>,
    /// Instruction budget per outermost `call`.
    pub budget: u64,
    steps: u64,
    allocations: usize,
    pub random: u64,
    depth: usize,
}

impl Default for Lua {
    fn default() -> Self {
        Self::new()
    }
}

impl Lua {
    pub fn new() -> Self {
        let mut tables = Arena::default();
        let globals = TableRef(tables.insert(Table::default()));
        let string_lib = TableRef(tables.insert(Table::default()));
        let mut lua = Lua {
            tables,
            funcs: Arena::default(),
            cells: Arena::default(),
            stack: Vec::new(),
            top: 0,
            frames: Vec::new(),
            open: BTreeMap::new(),
            globals,
            string_lib,
            registry: Vec::new(),
            budget: 200_000_000,
            steps: 0,
            allocations: 0,
            random: 0x2545_f491_4f6c_dd1d,
            depth: 0,
        };
        stdlib::open(&mut lua);
        lua
    }

    pub fn new_table(&mut self) -> TableRef {
        self.allocations += 1;
        TableRef(self.tables.insert(Table::default()))
    }
    pub fn table(&self, t: TableRef) -> Result<&Table> {
        self.tables.get(t.0).context("collected Lua table")
    }
    pub fn table_mut(&mut self, t: TableRef) -> Result<&mut Table> {
        self.tables.get_mut(t.0).context("collected Lua table")
    }
    pub fn host_function(&mut self, id: u32) -> Value {
        self.allocations += 1;
        Value::Function(FuncRef(self.funcs.insert(Function::Host(id))))
    }
    pub fn builtin(&mut self, id: Builtin, state: Vec<Value>) -> Value {
        self.allocations += 1;
        Value::Function(FuncRef(self.funcs.insert(Function::Builtin { id, state })))
    }
    pub fn global(&self, name: &str) -> Value {
        self.tables.get(self.globals.0).map_or(Value::Nil, |t| t.get_str(name))
    }
    pub fn set_global(&mut self, name: &str, value: Value) -> Result<()> {
        let g = self.globals;
        self.table_mut(g)?.set(&Value::str(name), value)
    }
    /// `t[name] = value` on a table value.
    pub fn set_field(&mut self, table: TableRef, name: &str, value: Value) -> Result<()> {
        self.table_mut(table)?.set(&Value::str(name), value)
    }
    pub fn get_field(&self, table: TableRef, name: &str) -> Result<Value> {
        Ok(self.table(table)?.get_str(name))
    }

    /// Instantiate a chunk's main function with `_ENV` = globals.
    pub fn load(&mut self, proto: Arc<Proto>) -> Result<Value> {
        ensure!(proto.upvalues.len() <= 1, "Lua main chunk upvalues");
        let env = self.cells.insert(Cell::Closed(Value::Table(self.globals)));
        let upvalues = if proto.upvalues.is_empty() { vec![] } else { vec![env] };
        self.allocations += 1;
        Ok(Value::Function(FuncRef(self.funcs.insert(Function::Lua { proto, upvalues }))))
    }

    pub fn is_idle(&self) -> bool {
        self.frames.is_empty() && self.depth == 0
    }

    /// Call any callable value and run it to completion.
    pub fn call(&mut self, func: &Value, args: Vec<Value>, host: &mut dyn Host) -> Result<Vec<Value>> {
        let outermost = self.depth == 0;
        if outermost {
            self.steps = 0;
        }
        ensure!(self.depth < 64, "Lua host re-entry depth");
        self.depth += 1;
        let result = self.call_inner(func, args, host);
        self.depth -= 1;
        if outermost {
            //A failed outermost call leaves no live frames behind.
            let base = 0;
            self.close(base);
            self.frames.clear();
            self.stack.clear();
            self.top = 0;
        }
        result
    }

    fn call_inner(&mut self, func: &Value, args: Vec<Value>, host: &mut dyn Host) -> Result<Vec<Value>> {
        let Value::Function(f) = func else { bail!("attempt to call a {} value", func.type_name()) };
        let function = self.funcs.get(f.0).context("collected Lua function")?.clone();
        match function {
            Function::Lua { proto, upvalues } => {
                let base = self.top.max(self.frames.last().map_or(0, |fr| fr.base + usize::from(fr.proto.stack) + 1));
                let entry = self.frames.len();
                self.ensure_stack(base + 1 + args.len())?;
                self.top = base + 1 + args.len();
                for (n, arg) in args.into_iter().enumerate() {
                    self.stack[base + 1 + n] = arg;
                }
                self.enter(*f, proto, upvalues, base + 1, base, -1)?;
                self.execute(entry, host)
            }
            Function::Builtin { id, state } => stdlib::call(self, host, *f, id, &state, args),
            Function::Host(id) => host.call(self, id, args),
        }
    }

    fn ensure_stack(&mut self, len: usize) -> Result<()> {
        ensure!(len <= MAX_STACK, "Lua stack overflow");
        if self.stack.len() < len {
            self.stack.resize(len, Value::Nil);
        }
        Ok(())
    }

    /// Push a Lua frame whose arguments are already at `base..self.top`
    /// (or, for host entry, at `base..` up to the pushed count).
    fn enter(&mut self, func: FuncRef, proto: Arc<Proto>, upvalues: Vec<u32>, base: usize, dest: usize, wanted: i32) -> Result<()> {
        ensure!(self.frames.len() < MAX_FRAMES, "Lua call depth (stack overflow)");
        let params = usize::from(proto.params);
        let size = usize::from(proto.stack).max(params) + 1;
        //Arguments supplied: from `base` up to `top` (CALL) or the host count.
        let supplied = self.top.saturating_sub(base);
        let mut varargs = Vec::new();
        if proto.vararg && supplied > params {
            varargs = self.stack[base + params..base + supplied].to_vec();
        }
        self.ensure_stack(base + size)?;
        for slot in &mut self.stack[base + params.min(supplied)..base + size] {
            *slot = Value::Nil;
        }
        self.top = base + size;
        self.frames.push(Frame { func, proto, upvalues, base, pc: 0, varargs, dest, wanted });
        Ok(())
    }

    fn constant(proto: &Proto, index: u32) -> Value {
        match &proto.constants[index as usize] {
            Constant::Nil => Value::Nil,
            Constant::Bool(v) => Value::Bool(*v),
            Constant::Int(v) => Value::Int(*v),
            Constant::Num(v) => Value::Num(*v),
            Constant::Str(s) => Value::Str(s.clone()),
        }
    }

    fn cell_get(&self, cell: u32) -> Value {
        match self.cells.get(cell) {
            Some(Cell::Open(slot)) => self.stack[*slot].clone(),
            Some(Cell::Closed(v)) => v.clone(),
            None => Value::Nil,
        }
    }
    fn cell_set(&mut self, cell: u32, value: Value) {
        match self.cells.get_mut(cell) {
            Some(Cell::Open(slot)) => {
                let slot = *slot;
                self.stack[slot] = value;
            }
            Some(Cell::Closed(v)) => *v = value,
            None => {}
        }
    }
    fn close(&mut self, level: usize) {
        let closing: Vec<(usize, u32)> = self.open.range(level..).map(|(k, v)| (*k, *v)).collect();
        for (slot, cell) in closing {
            let value = self.stack.get(slot).cloned().unwrap_or_default();
            if let Some(c) = self.cells.get_mut(cell) {
                *c = Cell::Closed(value);
            }
            self.open.remove(&slot);
        }
    }

    pub fn index(&mut self, object: &Value, key: &Value, host: &mut dyn Host) -> Result<Value> {
        match object {
            Value::Table(t) => Ok(self.table(*t)?.get(key)),
            Value::Str(_) => Ok(self.table(self.string_lib)?.get(key)),
            Value::UserData(u) => host.index(self, *u, key),
            other => bail!("attempt to index a {} value{}", other.type_name(), describe_key(key)),
        }
    }
    pub fn new_index(&mut self, object: &Value, key: &Value, value: Value, host: &mut dyn Host) -> Result<()> {
        match object {
            Value::Table(t) => {
                if matches!(value, Value::Table(_) | Value::Function(_)) {
                    self.allocations += 0;
                }
                self.table_mut(*t)?.set(key, value)
            }
            Value::UserData(u) => host.new_index(self, *u, key, value),
            other => bail!("attempt to index a {} value{}", other.type_name(), describe_key(key)),
        }
    }

    fn execute(&mut self, entry: usize, host: &mut dyn Host) -> Result<Vec<Value>> {
        let result = self.run(entry, host);
        if let Err(error) = result {
            //Describe the failing Lua location, then unwind this invocation.
            let location = self.frames.get(self.frames.len().saturating_sub(1)).map(|f| {
                format!("Lua function line {} pc {}", f.proto.line, f.pc.saturating_sub(1))
            });
            if let Some(frame) = self.frames.get(entry) {
                let base = frame.base;
                self.close(base);
            }
            self.frames.truncate(entry);
            return Err(match location {
                Some(location) => error.context(location),
                None => error,
            });
        }
        result
    }

    fn run(&mut self, entry: usize, host: &mut dyn Host) -> Result<Vec<Value>> {
        loop {
            self.steps += 1;
            ensure!(self.steps <= self.budget, "Lua instruction budget exhausted (infinite loop?)");
            let frame = self.frames.last_mut().context("Lua frame")?;
            let word = *frame.proto.code.get(frame.pc).context("Lua pc outside code")?;
            frame.pc += 1;
            let base = frame.base;
            let op = Op::decode(word);
            let a = base + op.a as usize;
            macro_rules! proto {
                () => {
                    self.frames.last().unwrap().proto
                };
            }
            macro_rules! rk {
                ($v:expr) => {{
                    let v = $v;
                    if v & 256 != 0 {
                        Self::constant(&proto!(), v & 255)
                    } else {
                        self.stack[base + v as usize].clone()
                    }
                }};
            }
            match op.code {
                0 => self.stack[a] = self.stack[base + op.b as usize].clone(),
                1 => self.stack[a] = Self::constant(&proto!(), Op::bx(word)),
                2 => {
                    let frame = self.frames.last_mut().unwrap();
                    let extra = frame.proto.code[frame.pc];
                    frame.pc += 1;
                    self.stack[a] = Self::constant(&proto!(), Op::ax(extra));
                }
                3 => {
                    self.stack[a] = Value::Bool(op.b != 0);
                    if op.c != 0 {
                        self.frames.last_mut().unwrap().pc += 1;
                    }
                }
                4 => {
                    for slot in &mut self.stack[a..=a + op.b as usize] {
                        *slot = Value::Nil;
                    }
                }
                5 => {
                    let cell = self.frames.last().unwrap().upvalues[op.b as usize];
                    self.stack[a] = self.cell_get(cell);
                }
                6 => {
                    let cell = self.frames.last().unwrap().upvalues[op.b as usize];
                    let table = self.cell_get(cell);
                    let key = rk!(op.c);
                    self.stack[a] = self.index(&table, &key, host)?;
                }
                7 => {
                    let table = self.stack[base + op.b as usize].clone();
                    let key = rk!(op.c);
                    self.stack[a] = self.index(&table, &key, host)?;
                }
                8 => {
                    let cell = self.frames.last().unwrap().upvalues[op.a as usize];
                    let table = self.cell_get(cell);
                    let (key, value) = (rk!(op.b), rk!(op.c));
                    self.new_index(&table, &key, value, host)?;
                }
                9 => {
                    let cell = self.frames.last().unwrap().upvalues[op.b as usize];
                    let value = self.stack[a].clone();
                    self.cell_set(cell, value);
                }
                10 => {
                    let table = self.stack[a].clone();
                    let (key, value) = (rk!(op.b), rk!(op.c));
                    self.new_index(&table, &key, value, host)?;
                }
                11 => {
                    let t = self.new_table();
                    self.stack[a] = Value::Table(t);
                }
                12 => {
                    let object = self.stack[base + op.b as usize].clone();
                    let key = rk!(op.c);
                    self.stack[a + 1] = object.clone();
                    self.stack[a] = self.index(&object, &key, host)?;
                }
                13..=24 => {
                    let (x, y) = (rk!(op.b), rk!(op.c));
                    self.stack[a] = arith(op.code, &x, &y)?;
                }
                25 => {
                    let x = self.stack[base + op.b as usize].clone();
                    self.stack[a] = match x.to_number() {
                        Some(Value::Int(v)) => Value::Int(v.wrapping_neg()),
                        Some(Value::Num(v)) => Value::Num(-v),
                        _ => bail!("attempt to perform arithmetic on a {} value", x.type_name()),
                    };
                }
                26 => {
                    let x = self.stack[base + op.b as usize].clone();
                    self.stack[a] = Value::Int(!to_bits(&x)?);
                }
                27 => self.stack[a] = Value::Bool(!self.stack[base + op.b as usize].truthy()),
                28 => {
                    let x = self.stack[base + op.b as usize].clone();
                    self.stack[a] = match &x {
                        Value::Str(s) => Value::Int(s.len() as i64),
                        Value::Table(t) => Value::Int(self.table(*t)?.len()),
                        other => bail!("attempt to get length of a {} value", other.type_name()),
                    };
                }
                29 => {
                    let mut out = Vec::new();
                    for slot in base + op.b as usize..=base + op.c as usize {
                        match &self.stack[slot] {
                            Value::Str(s) => out.extend_from_slice(s),
                            v @ (Value::Int(_) | Value::Num(_)) => out.extend(v.display()),
                            other => bail!("attempt to concatenate a {} value", other.type_name()),
                        }
                    }
                    self.stack[a] = Value::bytes(&out);
                }
                30 => {
                    if op.a != 0 {
                        self.close(base + op.a as usize - 1);
                    }
                    let frame = self.frames.last_mut().unwrap();
                    frame.pc = (frame.pc as i64 + i64::from(Op::sbx(word))) as usize;
                }
                31..=33 => {
                    let (x, y) = (rk!(op.b), rk!(op.c));
                    let truth = match op.code {
                        31 => x.raw_eq(&y),
                        32 => less(&x, &y, false)?,
                        _ => less(&x, &y, true)?,
                    };
                    if truth != (op.a != 0) {
                        self.frames.last_mut().unwrap().pc += 1;
                    }
                }
                34 => {
                    if self.stack[a].truthy() != (op.c != 0) {
                        self.frames.last_mut().unwrap().pc += 1;
                    }
                }
                35 => {
                    let value = self.stack[base + op.b as usize].clone();
                    if value.truthy() == (op.c != 0) {
                        self.stack[a] = value;
                    } else {
                        self.frames.last_mut().unwrap().pc += 1;
                    }
                }
                36 => {
                    let nargs = if op.b == 0 { self.top - (a + 1) } else { op.b as usize - 1 };
                    let func = self.stack[a].clone();
                    self.invoke(func, a, nargs, op.c as i32 - 1, host)?;
                }
                37 => {
                    let nargs = if op.b == 0 { self.top - (a + 1) } else { op.b as usize - 1 };
                    let func = self.stack[a].clone();
                    self.close(base);
                    let frame = self.frames.pop().unwrap();
                    let (dest, wanted) = (frame.dest, frame.wanted);
                    for n in 0..=nargs {
                        self.stack[dest + n] = self.stack[a + n].clone();
                    }
                    let lua = matches!(&func, Value::Function(f) if matches!(self.funcs.get(f.0), Some(Function::Lua { .. })));
                    if !lua && self.frames.len() == entry {
                        let args = self.stack[dest + 1..dest + 1 + nargs].to_vec();
                        self.top = dest;
                        return self.call_inner(&func, args, host);
                    }
                    self.invoke(func, dest, nargs, wanted, host)?;
                    if !lua && wanted >= 0 {
                        let caller = self.frames.last().unwrap();
                        self.top = caller.base + usize::from(caller.proto.stack);
                    }
                }
                38 => {
                    let count = if op.b == 0 { self.top - a } else { op.b as usize - 1 };
                    self.close(base);
                    if let Some(done) = self.finish_return(a, count, entry)? {
                        return Ok(done);
                    }
                }
                39 => {
                    let step = self.stack[a + 2].clone();
                    match (&self.stack[a], &step) {
                        (Value::Int(i), Value::Int(s)) => {
                            let limit = match &self.stack[a + 1] {
                                Value::Int(l) => *l,
                                _ => bail!("'for' limit"),
                            };
                            if let Some(next) = i.checked_add(*s) {
                                if if *s > 0 { next <= limit } else { next >= limit } {
                                    self.stack[a] = Value::Int(next);
                                    self.stack[a + 3] = Value::Int(next);
                                    let frame = self.frames.last_mut().unwrap();
                                    frame.pc = (frame.pc as i64 + i64::from(Op::sbx(word))) as usize;
                                }
                            }
                        }
                        _ => {
                            let i = self.stack[a].as_f64().context("'for' value")?;
                            let s = step.as_f64().context("'for' step")?;
                            let limit = self.stack[a + 1].as_f64().context("'for' limit")?;
                            let next = i + s;
                            if if s > 0. { next <= limit } else { next >= limit } {
                                self.stack[a] = Value::Num(next);
                                self.stack[a + 3] = Value::Num(next);
                                let frame = self.frames.last_mut().unwrap();
                                frame.pc = (frame.pc as i64 + i64::from(Op::sbx(word))) as usize;
                            }
                        }
                    }
                }
                40 => {
                    let init = self.stack[a].to_number().context("'for' initial value must be a number")?;
                    let limit = self.stack[a + 1].to_number().context("'for' limit must be a number")?;
                    let step = self.stack[a + 2].to_number().context("'for' step must be a number")?;
                    match (&init, &step) {
                        (Value::Int(i), Value::Int(s)) => {
                            ensure!(*s != 0, "'for' step is zero");
                            //Float limits clip to the integer range of the loop.
                            let limit = match limit {
                                Value::Int(l) => l,
                                Value::Num(l) => {
                                    if *s > 0 { l.floor().clamp(i64::MIN as f64, i64::MAX as f64) as i64 } else { l.ceil().clamp(i64::MIN as f64, i64::MAX as f64) as i64 }
                                }
                                _ => unreachable!(),
                            };
                            self.stack[a + 1] = Value::Int(limit);
                            self.stack[a] = Value::Int(i.wrapping_sub(*s));
                        }
                        _ => {
                            let s = step.as_f64().unwrap();
                            ensure!(s != 0., "'for' step is zero");
                            self.stack[a] = Value::Num(init.as_f64().unwrap() - s);
                            self.stack[a + 1] = Value::Num(limit.as_f64().unwrap());
                            self.stack[a + 2] = Value::Num(s);
                        }
                    }
                    let frame = self.frames.last_mut().unwrap();
                    frame.pc = (frame.pc as i64 + i64::from(Op::sbx(word))) as usize;
                }
                41 => {
                    let func = self.stack[a].clone();
                    self.ensure_stack(a + 6)?;
                    self.stack[a + 3] = func.clone();
                    self.stack[a + 4] = self.stack[a + 1].clone();
                    self.stack[a + 5] = self.stack[a + 2].clone();
                    self.invoke(func, a + 3, 2, op.c as i32, host)?;
                }
                42 => {
                    if !self.stack[a + 1].is_nil() {
                        self.stack[a] = self.stack[a + 1].clone();
                        let frame = self.frames.last_mut().unwrap();
                        frame.pc = (frame.pc as i64 + i64::from(Op::sbx(word))) as usize;
                    }
                }
                43 => {
                    let count = if op.b == 0 { self.top - a - 1 } else { op.b as usize };
                    let block = if op.c == 0 {
                        let frame = self.frames.last_mut().unwrap();
                        let extra = frame.proto.code[frame.pc];
                        frame.pc += 1;
                        Op::ax(extra) as i64
                    } else {
                        i64::from(op.c)
                    };
                    let Value::Table(t) = self.stack[a].clone() else { bail!("SETLIST target") };
                    let first = (block - 1) * 50;
                    for n in 1..=count {
                        let value = self.stack[a + n].clone();
                        self.table_mut(t)?.set(&Value::Int(first + n as i64), value)?;
                    }
                    self.top = base + usize::from(proto!().stack);
                }
                44 => {
                    let (proto, current) = {
                        let frame = self.frames.last().unwrap();
                        (frame.proto.protos[Op::bx(word) as usize].clone(), frame.upvalues.clone())
                    };
                    let mut upvalues = Vec::with_capacity(proto.upvalues.len());
                    for (instack, index) in &proto.upvalues {
                        if *instack {
                            let slot = base + usize::from(*index);
                            let cell = match self.open.get(&slot) {
                                Some(cell) => *cell,
                                None => {
                                    let cell = self.cells.insert(Cell::Open(slot));
                                    self.open.insert(slot, cell);
                                    cell
                                }
                            };
                            upvalues.push(cell);
                        } else {
                            upvalues.push(current[usize::from(*index)]);
                        }
                    }
                    self.allocations += 1;
                    self.stack[a] = Value::Function(FuncRef(self.funcs.insert(Function::Lua { proto, upvalues })));
                }
                45 => {
                    let varargs = self.frames.last().unwrap().varargs.clone();
                    let count = if op.b == 0 { varargs.len() } else { op.b as usize - 1 };
                    self.ensure_stack(a + count)?;
                    for n in 0..count {
                        self.stack[a + n] = varargs.get(n).cloned().unwrap_or_default();
                    }
                    if op.b == 0 {
                        self.top = a + count;
                    }
                }
                other => bail!("Lua opcode {other} is not valid here"),
            }
        }
    }

    /// Call `func` whose arguments are at `slot+1..slot+1+nargs`; results go
    /// to `slot..` (`wanted` < 0: all, setting `top`).
    fn invoke(&mut self, func: Value, slot: usize, nargs: usize, wanted: i32, host: &mut dyn Host) -> Result<()> {
        let Value::Function(f) = &func else {
            bail!("attempt to call a {} value", func.type_name())
        };
        let function = self.funcs.get(f.0).context("collected Lua function")?.clone();
        match function {
            Function::Lua { proto, upvalues } => {
                self.top = slot + 1 + nargs;
                self.enter(*f, proto, upvalues, slot + 1, slot, wanted)
            }
            Function::Builtin { id, state } => {
                let args = self.stack[slot + 1..slot + 1 + nargs].to_vec();
                let results = stdlib::call(self, host, *f, id, &state, args)?;
                self.place(slot, results, wanted)
            }
            Function::Host(id) => {
                let args = self.stack[slot + 1..slot + 1 + nargs].to_vec();
                let saved = self.top;
                self.top = slot + 1 + nargs;
                let results = host.call(self, id, args);
                self.top = saved;
                self.place(slot, results?, wanted)
            }
        }
    }

    fn place(&mut self, slot: usize, results: Vec<Value>, wanted: i32) -> Result<()> {
        let count = if wanted < 0 { results.len() } else { wanted as usize };
        self.ensure_stack(slot + count.max(1))?;
        let mut results = results.into_iter();
        for n in 0..count {
            self.stack[slot + n] = results.next().unwrap_or_default();
        }
        if wanted < 0 {
            self.top = slot + count;
        }
        Ok(())
    }

    /// Pop the current frame returning `count` values from `from`.
    fn finish_return(&mut self, from: usize, count: usize, entry: usize) -> Result<Option<Vec<Value>>> {
        let frame = self.frames.pop().context("return without frame")?;
        let values: Vec<Value> = self.stack[from..from + count].to_vec();
        if self.frames.len() == entry {
            self.top = frame.dest;
            return Ok(Some(values));
        }
        let caller = self.frames.last().unwrap();
        let caller_top = caller.base + usize::from(caller.proto.stack);
        self.place(frame.dest, values, frame.wanted)?;
        if frame.wanted >= 0 {
            self.top = caller_top;
        }
        Ok(None)
    }

    /// Mark-sweep from globals, the string library, `registry`, live frames
    /// and `roots`. Returns the number of freed objects.
    pub fn collect(&mut self, roots: &[Value]) -> usize {
        let mut tables = vec![false; self.tables.slots.len()];
        let mut funcs = vec![false; self.funcs.slots.len()];
        let mut cells = vec![false; self.cells.slots.len()];
        let mut work: Vec<Value> = vec![Value::Table(self.globals), Value::Table(self.string_lib)];
        work.extend(self.registry.iter().cloned());
        work.extend(roots.iter().cloned());
        work.extend(self.stack.iter().cloned());
        for frame in &self.frames {
            work.push(Value::Function(frame.func));
            work.extend(frame.varargs.iter().cloned());
        }
        for cell in self.open.values() {
            if let Some(m) = cells.get_mut(*cell as usize) {
                *m = true;
            }
        }
        while let Some(value) = work.pop() {
            match value {
                Value::Table(t) => {
                    let n = t.0 as usize;
                    if n >= tables.len() || tables[n] {
                        continue;
                    }
                    tables[n] = true;
                    if let Some(table) = self.tables.get(t.0) {
                        for (k, v) in table.values() {
                            work.push(k);
                            work.push(v.clone());
                        }
                    }
                }
                Value::Function(f) => {
                    let n = f.0 as usize;
                    if n >= funcs.len() || funcs[n] {
                        continue;
                    }
                    funcs[n] = true;
                    match self.funcs.get(f.0) {
                        Some(Function::Lua { upvalues, .. }) => {
                            for cell in upvalues {
                                let c = *cell as usize;
                                if c < cells.len() && !cells[c] {
                                    cells[c] = true;
                                    if let Some(Cell::Closed(v)) = self.cells.get(*cell) {
                                        work.push(v.clone());
                                    }
                                }
                            }
                        }
                        Some(Function::Builtin { state, .. }) => work.extend(state.iter().cloned()),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        self.allocations = 0;
        self.tables.retain(&tables) + self.funcs.retain(&funcs) + self.cells.retain(&cells)
    }
    /// Objects allocated since the last collection.
    pub fn allocations(&self) -> usize {
        self.allocations
    }
    pub fn next_random(&mut self) -> u64 {
        //xorshift64*: deterministic per VM, seeded by `math.randomseed`.
        let mut x = self.random;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.random = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
}

fn describe_key(key: &Value) -> String {
    match key {
        Value::Str(s) => format!(" (field '{}')", String::from_utf8_lossy(s)),
        _ => String::new(),
    }
}

pub fn to_bits(v: &Value) -> Result<i64> {
    match v.to_number() {
        Some(Value::Int(i)) => Ok(i),
        Some(Value::Num(f)) if f.fract() == 0. && f >= -9.2233720368547758e18 && f < 9.2233720368547758e18 => Ok(f as i64),
        Some(_) => bail!("number has no integer representation"),
        None => bail!("attempt to perform bitwise operation on a {} value", v.type_name()),
    }
}

pub fn arith(code: u8, x: &Value, y: &Value) -> Result<Value> {
    if (20..=24).contains(&code) {
        let (a, b) = (to_bits(x)?, to_bits(y)?);
        return Ok(Value::Int(match code {
            20 => a & b,
            21 => a | b,
            22 => a ^ b,
            23 => shift_left(a, b),
            _ => shift_left(a, b.wrapping_neg()),
        }));
    }
    let (Some(a), Some(b)) = (x.to_number(), y.to_number()) else {
        let bad = if x.to_number().is_none() { x } else { y };
        bail!("attempt to perform arithmetic on a {} value", bad.type_name())
    };
    if let (Value::Int(a), Value::Int(b)) = (&a, &b) {
        let (a, b) = (*a, *b);
        match code {
            13 => return Ok(Value::Int(a.wrapping_add(b))),
            14 => return Ok(Value::Int(a.wrapping_sub(b))),
            15 => return Ok(Value::Int(a.wrapping_mul(b))),
            16 => {
                ensure!(b != 0, "attempt to perform 'n%%0'");
                let r = a.wrapping_rem(b);
                return Ok(Value::Int(if r != 0 && (r ^ b) < 0 { r + b } else { r }));
            }
            19 => {
                ensure!(b != 0, "attempt to perform 'n//0'");
                let q = a.wrapping_div(b);
                return Ok(Value::Int(if (a % b != 0) && ((a ^ b) < 0) { q - 1 } else { q }));
            }
            _ => {}
        }
    }
    let (a, b) = (a.as_f64().unwrap(), b.as_f64().unwrap());
    Ok(Value::Num(match code {
        13 => a + b,
        14 => a - b,
        15 => a * b,
        16 => {
            let r = a % b;
            if r != 0. && (r < 0.) != (b < 0.) { r + b } else { r }
        }
        17 => a.powf(b),
        18 => a / b,
        19 => (a / b).floor(),
        _ => bail!("arithmetic opcode {code}"),
    }))
}

fn shift_left(a: i64, b: i64) -> i64 {
    if b <= -64 || b >= 64 {
        0
    } else if b >= 0 {
        ((a as u64) << b) as i64
    } else {
        ((a as u64) >> (-b)) as i64
    }
}

pub fn less(x: &Value, y: &Value, or_equal: bool) -> Result<bool> {
    match (x, y) {
        (Value::Int(a), Value::Int(b)) => Ok(if or_equal { a <= b } else { a < b }),
        (Value::Int(_) | Value::Num(_), Value::Int(_) | Value::Num(_)) => {
            let (a, b) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            Ok(if or_equal { a <= b } else { a < b })
        }
        (Value::Str(a), Value::Str(b)) => Ok(if or_equal { a <= b } else { a < b }),
        _ => bail!("attempt to compare {} with {}", x.type_name(), y.type_name()),
    }
}

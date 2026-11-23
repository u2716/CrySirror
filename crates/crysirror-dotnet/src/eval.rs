// SPDX-License-Identifier: MPL-2.0
//! Evaluation of constant static initializers (`.cctor`): the IL that C#
//! emits for field initializers built from literals, `new` of structs and
//! classes, object/collection initializers and constant arrays. Constructors
//! of the assembly's own types are evaluated when they only store their
//! arguments (and literals) into fields; constructors of external types
//! (`UnityEngine.Vector3`, …) are recorded with their arguments. Anything
//! else — branches, arbitrary calls — fails with its opcode or member name.
use crate::{table, Assembly, Token};
use anyhow::{bail, ensure, Context, Result};
use std::collections::BTreeMap;

/// Upper bounds of one evaluation.
const MAX_STEPS: usize = 4_000_000;
const MAX_OBJECTS: usize = 1_000_000;
const MAX_DEPTH: usize = 16;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Int(i64),
    Float(f64),
    Str(String),
    /// An object of the evaluation heap.
    Ref(usize),
    /// A field/method/type token (`ldtoken`).
    Token(u32),
    /// Address of a local (`ldloca`), for structs built in place.
    Local(usize),
}

impl Value {
    pub fn int(&self) -> Option<i64> {
        match self {
            Value::Int(v) => Some(*v),
            Value::Float(f) => Some(*f as i64),
            _ => None,
        }
    }
    pub fn float(&self) -> Option<f64> {
        match self {
            Value::Int(v) => Some(*v as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }
    pub fn str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Object {
    /// An instance: class name, fields set by its constructor/initializer,
    /// and for external types the constructor arguments.
    Instance { class: String, fields: BTreeMap<String, Value>, args: Vec<Value> },
    List(Vec<Value>),
    Dict(Vec<(Value, Value)>),
    Array(Vec<Value>),
}

/// Result of evaluating a type's static constructor.
#[derive(Debug, Default)]
pub struct Statics {
    pub heap: Vec<Object>,
    pub fields: BTreeMap<String, Value>,
}

impl Statics {
    pub fn object(&self, v: &Value) -> Option<&Object> {
        match v {
            Value::Ref(i) => self.heap.get(*i),
            _ => None,
        }
    }
    /// Entries of a dictionary value.
    pub fn dict(&self, v: &Value) -> Option<&[(Value, Value)]> {
        match self.object(v)? {
            Object::Dict(d) => Some(d),
            _ => None,
        }
    }
    /// Items of a list or array value.
    pub fn items(&self, v: &Value) -> Option<&[Value]> {
        match self.object(v)? {
            Object::List(l) | Object::Array(l) => Some(l),
            _ => None,
        }
    }
    /// A field of an instance value.
    pub fn field<'a>(&'a self, v: &Value, name: &str) -> Option<&'a Value> {
        match self.object(v)? {
            Object::Instance { fields, .. } => fields.get(name),
            _ => None,
        }
    }
    /// Constructor arguments of an external instance (`Vector3(x, y, z)`).
    pub fn args<'a>(&'a self, v: &Value) -> Option<&'a [Value]> {
        match self.object(v)? {
            Object::Instance { args, .. } => Some(args),
            _ => None,
        }
    }
}

impl Assembly {
    /// Literal names of an enum type: value → name.
    pub fn enum_names(&self, type_name: &str) -> Result<BTreeMap<i64, String>> {
        let row = self.find_type(type_name)?.with_context(|| format!("enum {type_name} not found"))?;
        let (fields, _) = self.members(row)?;
        let mut out = BTreeMap::new();
        for f in fields {
            let Some((ty, bytes)) = self.field_constant(f)? else { continue };
            let v = match (ty, bytes.len()) {
                (0x08, 4) | (0x09, 4) => i32::from_le_bytes(bytes.try_into()?) as i64,
                (0x04, 1) | (0x05, 1) => bytes[0] as i8 as i64,
                (0x06, 2) | (0x07, 2) => i16::from_le_bytes(bytes.try_into()?) as i64,
                (0x0a, 8) | (0x0b, 8) => i64::from_le_bytes(bytes.try_into()?),
                _ => continue,
            };
            out.insert(v, self.string(self.cell(table::FIELD, f, 1)?)?.to_owned());
        }
        Ok(out)
    }

    /// Evaluate the static constructor of `type_name`.
    pub fn eval_cctor(&self, type_name: &str) -> Result<Statics> {
        let row = self.find_type(type_name)?.with_context(|| format!("type {type_name} not found"))?;
        let (_, methods) = self.members(row)?;
        let cctor = methods
            .clone()
            .find(|m| self.cell(table::METHOD_DEF, *m, 3).ok().and_then(|n| self.string(n).ok()) == Some(".cctor"))
            .with_context(|| format!("{type_name} has no static constructor"))?;
        let mut st = Statics::default();
        let mut ev = Eval { asm: self, steps: 0, elem: BTreeMap::new() };
        ev.run(&mut st, cctor, None, Vec::new(), 0)?;
        Ok(st)
    }

    /// Name and parameter count of a MethodDef or MemberRef, and its owner
    /// type name.
    fn method_ref(&self, tok: Token) -> Result<(String, String, usize, Option<u32>)> {
        match tok.table {
            table::METHOD_DEF => {
                let name = self.string(self.cell(table::METHOD_DEF, tok.row, 3)?)?.to_owned();
                let sig = self.blob(self.cell(table::METHOD_DEF, tok.row, 4)?)?;
                let owner = self.method_owner(tok.row)?;
                Ok((self.type_name(Token { table: table::TYPE_DEF, row: owner })?, name, param_count(sig)?, Some(tok.row)))
            }
            table::MEMBER_REF => {
                let parent = Token::from_u32(self.cell(table::MEMBER_REF, tok.row, 0)?);
                let name = self.string(self.cell(table::MEMBER_REF, tok.row, 1)?)?.to_owned();
                let sig = self.blob(self.cell(table::MEMBER_REF, tok.row, 2)?)?;
                let class = match parent.table {
                    table::TYPE_SPEC => self.type_spec_name(parent.row)?,
                    _ => self.type_name(parent)?,
                };
                // A MemberRef into this assembly's own type resolves to its MethodDef.
                let local = match parent.table {
                    table::TYPE_DEF => self.find_method(parent.row, &name, param_count(sig)?)?,
                    _ => None,
                };
                Ok((class, name, param_count(sig)?, local))
            }
            table::METHOD_SPEC => {
                let m = Token::from_u32(self.cell(table::METHOD_SPEC, tok.row, 0)?);
                self.method_ref(m)
            }
            t => bail!("token table {t:#x} is not a method"),
        }
    }

    fn find_method(&self, ty: u32, name: &str, params: usize) -> Result<Option<u32>> {
        let (_, methods) = self.members(ty)?;
        for m in methods {
            if self.string(self.cell(table::METHOD_DEF, m, 3)?)? == name && param_count(self.blob(self.cell(table::METHOD_DEF, m, 4)?)?)? == params {
                return Ok(Some(m));
            }
        }
        Ok(None)
    }

    /// Generic type name of a TypeSpec (`GENERICINST` of a class).
    fn type_spec_name(&self, row: u32) -> Result<String> {
        let b = self.blob(self.cell(table::TYPE_SPEC, row, 0)?)?;
        let mut o = 0;
        let et = *b.first().context("typespec")?;
        o += 1;
        if et == 0x15 {
            o += 1; // CLASS / VALUETYPE
            let coded = crate::compressed(b, &mut o)?;
            let t = [table::TYPE_DEF, table::TYPE_REF, table::TYPE_SPEC][(coded & 3) as usize];
            return self.type_name(Token { table: t, row: coded >> 2 });
        }
        Ok(format!("<typespec {et:#x}>"))
    }

    /// Field name and owner type of a Field token or field MemberRef.
    fn field_ref(&self, tok: Token) -> Result<(String, String, Option<u32>)> {
        match tok.table {
            table::FIELD => {
                let owner = self.field_owner(tok.row)?;
                Ok((self.type_name(Token { table: table::TYPE_DEF, row: owner })?, self.string(self.cell(table::FIELD, tok.row, 1)?)?.to_owned(), Some(tok.row)))
            }
            table::MEMBER_REF => {
                let parent = Token::from_u32(self.cell(table::MEMBER_REF, tok.row, 0)?);
                let class = match parent.table {
                    table::TYPE_SPEC => self.type_spec_name(parent.row)?,
                    _ => self.type_name(parent)?,
                };
                Ok((class, self.string(self.cell(table::MEMBER_REF, tok.row, 1)?)?.to_owned(), None))
            }
            t => bail!("token table {t:#x} is not a field"),
        }
    }
}

/// Parameter count of a method signature blob (`HASTHIS`, count, …).
fn param_count(sig: &[u8]) -> Result<usize> {
    let mut o = 1;
    let conv = *sig.first().context("method signature")?;
    if conv & 0x10 != 0 {
        crate::compressed(sig, &mut o)?; // generic parameter count
    }
    Ok(crate::compressed(sig, &mut o)? as usize)
}

struct Eval<'a> {
    asm: &'a Assembly,
    steps: usize,
    /// `newarr` element type names by heap index.
    elem: BTreeMap<usize, String>,
}

impl Eval<'_> {
    fn alloc(&self, st: &mut Statics, o: Object) -> Result<Value> {
        ensure!(st.heap.len() < MAX_OBJECTS, "static initializer object limit");
        st.heap.push(o);
        Ok(Value::Ref(st.heap.len() - 1))
    }

    /// Run a method body; `this` for instance constructors.
    fn run(&mut self, st: &mut Statics, method: u32, this: Option<Value>, args: Vec<Value>, depth: usize) -> Result<Option<Value>> {
        ensure!(depth <= MAX_DEPTH, "static initializer call depth");
        let il = self.asm.method_body(method)?;
        let mut stack: Vec<Value> = Vec::new();
        let mut locals: Vec<Value> = vec![Value::Null; 256];
        let arg = |i: usize| -> Result<Value> {
            match (&this, i) {
                (Some(t), 0) => Ok(t.clone()),
                (Some(_), i) => args.get(i - 1).cloned().context("argument index"),
                (None, i) => args.get(i).cloned().context("argument index"),
            }
        };
        let mut pc = 0usize;
        let tok = |pc: &mut usize| -> Result<Token> {
            let t = u32::from_le_bytes(il.get(*pc..*pc + 4).context("IL token")?.try_into()?);
            *pc += 4;
            Ok(Token::from_u32(t))
        };
        let pop = |stack: &mut Vec<Value>| stack.pop().context("IL stack underflow");
        while pc < il.len() {
            self.steps += 1;
            ensure!(self.steps <= MAX_STEPS, "static initializer step limit");
            let op = il[pc];
            pc += 1;
            match op {
                0x00 => {}
                0x02..=0x05 => stack.push(arg((op - 0x02) as usize)?),
                0x06..=0x09 => stack.push(locals[(op - 0x06) as usize].clone()),
                0x0A..=0x0D => locals[(op - 0x0A) as usize] = pop(&mut stack)?,
                0x0E => {
                    stack.push(arg(il[pc] as usize)?);
                    pc += 1;
                }
                0x11 => {
                    stack.push(locals[il[pc] as usize].clone());
                    pc += 1;
                }
                0x13 => {
                    locals[il[pc] as usize] = pop(&mut stack)?;
                    pc += 1;
                }
                0x12 => {
                    stack.push(Value::Local(il[pc] as usize));
                    pc += 1;
                }
                0x14 => stack.push(Value::Null),
                0x15 => stack.push(Value::Int(-1)),
                0x16..=0x1E => stack.push(Value::Int((op - 0x16) as i64)),
                0x1F => {
                    stack.push(Value::Int(il[pc] as i8 as i64));
                    pc += 1;
                }
                0x20 => {
                    stack.push(Value::Int(i32::from_le_bytes(il[pc..pc + 4].try_into()?) as i64));
                    pc += 4;
                }
                0x21 => {
                    stack.push(Value::Int(i64::from_le_bytes(il[pc..pc + 8].try_into()?)));
                    pc += 8;
                }
                0x22 => {
                    stack.push(Value::Float(f32::from_le_bytes(il[pc..pc + 4].try_into()?) as f64));
                    pc += 4;
                }
                0x23 => {
                    stack.push(Value::Float(f64::from_le_bytes(il[pc..pc + 8].try_into()?)));
                    pc += 8;
                }
                0x25 => {
                    let v = stack.last().cloned().context("dup on empty stack")?;
                    stack.push(v);
                }
                0x26 => {
                    pop(&mut stack)?;
                }
                0x2A => return Ok(stack.pop()),
                // conv.*: numeric values keep their kind.
                0x67..=0x6E | 0x76 | 0xD1..=0xD3 | 0xB3..=0xBA | 0xD4 | 0xD5 | 0xE0 => {
                    if matches!(op, 0x6B | 0x6C | 0x76) {
                        let v = pop(&mut stack)?;
                        stack.push(Value::Float(v.float().context("conv.r on a non-number")?));
                    }
                }
                0x72 => {
                    let t = tok(&mut pc)?;
                    stack.push(Value::Str(self.asm.user_string(t.row)?));
                }
                0x28 | 0x6F => {
                    let t = tok(&mut pc)?;
                    let (class, name, n, local) = self.asm.method_ref(t)?;
                    let mut call_args = Vec::with_capacity(n);
                    for _ in 0..n {
                        call_args.push(pop(&mut stack)?);
                    }
                    call_args.reverse();
                    if name == "InitializeArray" && class.ends_with("RuntimeHelpers") {
                        let (Value::Ref(arr), Value::Token(field)) = (&call_args[0], &call_args[1]) else { bail!("InitializeArray arguments") };
                        self.init_array(st, *arr, Token::from_u32(*field))?;
                        continue;
                    }
                    let target = pop(&mut stack).ok();
                    // A struct constructor on a local's address.
                    if let (Some(Value::Local(l)), ".ctor") = (&target, name.as_str()) {
                        let v = self.alloc(st, Object::Instance { class: class.clone(), fields: BTreeMap::new(), args: Vec::new() })?;
                        match local {
                            Some(m) => {
                                self.run(st, m, Some(v.clone()), call_args, depth + 1)?;
                            }
                            None => {
                                if let Value::Ref(i) = v {
                                    if let Object::Instance { args, .. } = &mut st.heap[i] {
                                        *args = call_args;
                                    }
                                }
                            }
                        }
                        locals[*l] = v;
                        continue;
                    }
                    let target = match &target {
                        Some(Value::Ref(i)) => *i,
                        _ => {
                            // Base constructors (System.Object/ValueType) of an
                            // evaluated constructor do nothing.
                            if name == ".ctor" && (class == "System.Object" || class == "System.ValueType" || class.starts_with("UnityEngine.MonoBehaviour")) {
                                continue;
                            }
                            bail!("call {class}::{name} is not a constant initializer")
                        }
                    };
                    match (&mut st.heap[target], name.as_str()) {
                        (Object::Dict(d), "Add" | "set_Item") if call_args.len() == 2 => {
                            let mut it = call_args.into_iter();
                            let (k, v) = (it.next().unwrap(), it.next().unwrap());
                            if let Some(e) = d.iter_mut().find(|e| e.0 == k) {
                                e.1 = v;
                            } else {
                                d.push((k, v));
                            }
                        }
                        (Object::List(l), "Add") if call_args.len() == 1 => l.push(call_args.into_iter().next().unwrap()),
                        (Object::Instance { .. }, ".ctor") if class == "System.Object" => {}
                        (Object::Instance { class: c, .. }, ".ctor") => {
                            // Base constructor of an own type.
                            let c = c.clone();
                            match local {
                                Some(m) => {
                                    self.run(st, m, Some(Value::Ref(target)), call_args, depth + 1)?;
                                }
                                None => bail!("base constructor {class} of {c} is external"),
                            }
                        }
                        _ => bail!("call {class}::{name} is not a constant initializer"),
                    }
                }
                0x73 => {
                    let t = tok(&mut pc)?;
                    let (class, _, n, local) = self.asm.method_ref(t)?;
                    let mut call_args = Vec::with_capacity(n);
                    for _ in 0..n {
                        call_args.push(pop(&mut stack)?);
                    }
                    call_args.reverse();
                    let base = class.split('`').next().unwrap_or(&class);
                    let obj = match base {
                        "System.Collections.Generic.Dictionary" | "System.Collections.Generic.SortedDictionary" | "System.Collections.Hashtable" => Object::Dict(Vec::new()),
                        "System.Collections.Generic.List" | "System.Collections.Generic.HashSet" => Object::List(Vec::new()),
                        _ => Object::Instance { class: class.clone(), fields: BTreeMap::new(), args: Vec::new() },
                    };
                    let v = self.alloc(st, obj)?;
                    match local {
                        Some(m) => {
                            self.run(st, m, Some(v.clone()), call_args, depth + 1)?;
                        }
                        None => {
                            // External: keep the arguments (collections take
                            // capacities/comparers, which do not matter here).
                            if let Value::Ref(i) = v {
                                if let Object::Instance { args, .. } = &mut st.heap[i] {
                                    *args = call_args;
                                }
                            }
                        }
                    }
                    stack.push(v);
                }
                0x7B => {
                    let t = tok(&mut pc)?;
                    let (_, name, _) = self.asm.field_ref(t)?;
                    let obj = pop(&mut stack)?;
                    let v = match &obj {
                        Value::Ref(i) => match &st.heap[*i] {
                            Object::Instance { fields, .. } => fields.get(&name).cloned().unwrap_or(Value::Null),
                            _ => bail!("ldfld {name} on a collection"),
                        },
                        _ => bail!("ldfld {name} on a non-object"),
                    };
                    stack.push(v);
                }
                0x7D => {
                    let t = tok(&mut pc)?;
                    let (_, name, _) = self.asm.field_ref(t)?;
                    let v = pop(&mut stack)?;
                    let mut obj = pop(&mut stack)?;
                    if let Value::Local(l) = obj {
                        if !matches!(locals[l], Value::Ref(_)) {
                            locals[l] = self.alloc(st, Object::Instance { class: String::new(), fields: BTreeMap::new(), args: Vec::new() })?;
                        }
                        obj = locals[l].clone();
                    }
                    match obj {
                        Value::Ref(i) => match &mut st.heap[i] {
                            Object::Instance { fields, .. } => {
                                fields.insert(name, v);
                            }
                            _ => bail!("stfld {name} on a collection"),
                        },
                        _ => bail!("stfld {name} on a non-object"),
                    }
                }
                0x7E => {
                    let t = tok(&mut pc)?;
                    let (class, name, local) = self.asm.field_ref(t)?;
                    let v = match st.fields.get(&name) {
                        Some(v) if local.is_some() => v.clone(),
                        _ => match external_static(&class, &name) {
                            Some((c, args)) if !c.is_empty() => self.alloc(st, Object::Instance { class: c.to_owned(), fields: BTreeMap::new(), args })?,
                            Some((_, mut args)) => args.pop().unwrap_or(Value::Null),
                            None => bail!("ldsfld {class}::{name} before it is set"),
                        },
                    };
                    stack.push(v);
                }
                0x80 => {
                    let t = tok(&mut pc)?;
                    let (_, name, _) = self.asm.field_ref(t)?;
                    let v = pop(&mut stack)?;
                    st.fields.insert(name, v);
                }
                0x8C => {
                    tok(&mut pc)?; // box: the value stays as is
                }
                0x8D => {
                    let t = tok(&mut pc)?;
                    let n = pop(&mut stack)?.int().context("newarr length")?;
                    ensure!((0..=1_000_000).contains(&n), "newarr length {n}");
                    let v = self.alloc(st, Object::Array(vec![Value::Null; n as usize]))?;
                    if let Value::Ref(i) = v {
                        let name = if t.table == table::TYPE_SPEC { String::new() } else { self.asm.type_name(t)? };
                        self.elem.insert(i, name);
                    }
                    stack.push(v);
                }
                0x9C..=0xA2 | 0xA4 => {
                    if op == 0xA4 {
                        tok(&mut pc)?;
                    }
                    let v = pop(&mut stack)?;
                    let i = pop(&mut stack)?.int().context("stelem index")?;
                    let arr = pop(&mut stack)?;
                    let Value::Ref(a) = arr else { bail!("stelem on a non-array") };
                    let Object::Array(items) = &mut st.heap[a] else { bail!("stelem on a non-array") };
                    let slot = items.get_mut(i as usize).context("stelem index outside the array")?;
                    *slot = v;
                }
                0xD0 => {
                    let t = tok(&mut pc)?;
                    stack.push(Value::Token(((t.table as u32) << 24) | t.row));
                }
                0xFE => {
                    let op2 = *il.get(pc).context("IL prefix")?;
                    pc += 1;
                    match op2 {
                        0x15 => {
                            let t = tok(&mut pc)?;
                            let Value::Local(l) = pop(&mut stack)? else { bail!("initobj on a non-local") };
                            let class = if t.table == table::TYPE_SPEC { String::new() } else { self.asm.type_name(t)? };
                            locals[l] = self.alloc(st, Object::Instance { class, fields: BTreeMap::new(), args: Vec::new() })?;
                        }
                        _ => bail!("IL opcode FE {op2:02X} in a static initializer"),
                    }
                }
                _ => bail!("IL opcode {op:02X} at {} in a static initializer", pc - 1),
            }
        }
        Ok(None)
    }

    /// `RuntimeHelpers.InitializeArray`: elements of the `newarr` type from
    /// the field's RVA data (its `__StaticArrayInitTypeSize` class layout).
    fn init_array(&self, st: &mut Statics, arr: usize, field: Token) -> Result<()> {
        ensure!(field.table == table::FIELD, "InitializeArray needs a field token");
        let rva = self.asm.field_rva(field.row)?.context("InitializeArray field has no RVA data")?;
        let sig = self.asm.blob(self.asm.cell(table::FIELD, field.row, 2)?)?;
        ensure!(sig.len() >= 3 && sig[0] == 0x06 && sig[1] == 0x11, "InitializeArray field type");
        let mut o = 2;
        let coded = crate::compressed(sig, &mut o)?;
        ensure!(coded & 3 == 0, "InitializeArray data type is not a TypeDef");
        let layout_row = (1..=self.asm.rows(0x0F)).find(|r| self.asm.cell(0x0F, *r, 2).ok() == Some(coded >> 2)).context("InitializeArray data has no class layout")?;
        let total = self.asm.cell(0x0F, layout_row, 1)? as usize;
        let data = self.asm.at_rva(rva, total)?;
        let elem = self.elem.get(&arr).cloned().unwrap_or_default();
        let Object::Array(items) = &mut st.heap[arr] else { bail!("InitializeArray on a non-array") };
        let size = match elem.as_str() {
            "System.Byte" | "System.SByte" | "System.Boolean" => 1,
            "System.Int16" | "System.UInt16" | "System.Char" => 2,
            "System.Int32" | "System.UInt32" | "System.Single" => 4,
            "System.Int64" | "System.UInt64" | "System.Double" => 8,
            other => bail!("InitializeArray of {other}"),
        };
        ensure!(items.len() * size <= data.len(), "InitializeArray data too short");
        for (i, v) in items.iter_mut().enumerate() {
            let b = &data[i * size..(i + 1) * size];
            *v = match elem.as_str() {
                "System.Single" => Value::Float(f32::from_le_bytes(b.try_into()?) as f64),
                "System.Double" => Value::Float(f64::from_le_bytes(b.try_into()?)),
                "System.SByte" => Value::Int(b[0] as i8 as i64),
                "System.Byte" | "System.Boolean" => Value::Int(b[0] as i64),
                "System.Int16" => Value::Int(i16::from_le_bytes(b.try_into()?) as i64),
                "System.UInt16" | "System.Char" => Value::Int(u16::from_le_bytes(b.try_into()?) as i64),
                "System.Int32" => Value::Int(i32::from_le_bytes(b.try_into()?) as i64),
                "System.UInt32" => Value::Int(u32::from_le_bytes(b.try_into()?) as i64),
                _ => Value::Int(i64::from_le_bytes(b.try_into()?)),
            };
        }
        Ok(())
    }
}

/// Constant static fields of external types: `String.Empty` and the
/// `UnityEngine.Vector2/3/Color` shorthands, as (class, constructor args);
/// an empty class means a plain value (the last argument).
fn external_static(class: &str, name: &str) -> Option<(&'static str, Vec<Value>)> {
    let f = |v: &[f64]| v.iter().map(|x| Value::Float(*x)).collect::<Vec<_>>();
    Some(match (class, name) {
        ("System.String", "Empty") => ("", vec![Value::Str(String::new())]),
        ("UnityEngine.Vector3", n) => ("UnityEngine.Vector3", f(match n {
            "zero" => &[0., 0., 0.],
            "one" => &[1., 1., 1.],
            "up" => &[0., 1., 0.],
            "down" => &[0., -1., 0.],
            "left" => &[-1., 0., 0.],
            "right" => &[1., 0., 0.],
            "forward" => &[0., 0., 1.],
            "back" => &[0., 0., -1.],
            _ => return None,
        })),
        ("UnityEngine.Vector2", n) => ("UnityEngine.Vector2", f(match n {
            "zero" => &[0., 0.],
            "one" => &[1., 1.],
            "up" => &[0., 1.],
            "right" => &[1., 0.],
            _ => return None,
        })),
        ("UnityEngine.Color", n) => ("UnityEngine.Color", f(match n {
            "white" => &[1., 1., 1., 1.],
            "black" => &[0., 0., 0., 1.],
            "clear" => &[0., 0., 0., 0.],
            "red" => &[1., 0., 0., 1.],
            "green" => &[0., 1., 0., 1.],
            "blue" => &[0., 0., 1., 1.],
            _ => return None,
        })),
        ("UnityEngine.Quaternion", "identity") => ("UnityEngine.Quaternion", f(&[0., 0., 0., 1.])),
        _ => return None,
    })
}

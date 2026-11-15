// SPDX-License-Identifier: MPL-2.0
//! Values, tables and the handle heap. Tables, closures and upvalue cells live
//! in index arenas so a VM is `Send`, has no reference cycles to leak and can
//! be collected by a mark-sweep pass from explicit roots.
use crate::chunk::{Bytes, Proto};
use anyhow::{bail, Result};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TableRef(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FuncRef(pub u32);

#[derive(Clone, Debug, Default)]
pub enum Value {
    #[default]
    Nil,
    Bool(bool),
    Int(i64),
    Num(f64),
    Str(Bytes),
    Table(TableRef),
    Function(FuncRef),
    /// Host object handle; indexing is delegated to the `Host`.
    UserData(u32),
}

impl Value {
    pub fn str(text: &str) -> Self {
        Value::Str(Arc::from(text.as_bytes()))
    }
    pub fn bytes(bytes: &[u8]) -> Self {
        Value::Str(Arc::from(bytes))
    }
    pub fn truthy(&self) -> bool {
        !matches!(self, Value::Nil | Value::Bool(false))
    }
    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Nil => "nil",
            Value::Bool(_) => "boolean",
            Value::Int(_) | Value::Num(_) => "number",
            Value::Str(_) => "string",
            Value::Table(_) => "table",
            Value::Function(_) => "function",
            Value::UserData(_) => "userdata",
        }
    }
    /// Numeric value with Lua string coercion.
    pub fn to_number(&self) -> Option<Value> {
        match self {
            Value::Int(_) | Value::Num(_) => Some(self.clone()),
            Value::Str(s) => parse_number(std::str::from_utf8(s).ok()?.trim()),
            _ => None,
        }
    }
    pub fn as_f64(&self) -> Option<f64> {
        match self.to_number()? {
            Value::Int(v) => Some(v as f64),
            Value::Num(v) => Some(v),
            _ => None,
        }
    }
    /// Integer value (floats with an exact integer representation convert).
    pub fn as_i64(&self) -> Option<i64> {
        match self.to_number()? {
            Value::Int(v) => Some(v),
            Value::Num(v) if v.fract() == 0. && v >= -9.2233720368547758e18 && v < 9.2233720368547758e18 => Some(v as i64),
            _ => None,
        }
    }
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
    /// `tostring` for primitive values (tables/functions get handle names).
    pub fn display(&self) -> Vec<u8> {
        match self {
            Value::Nil => b"nil".to_vec(),
            Value::Bool(v) => v.to_string().into_bytes(),
            Value::Int(v) => v.to_string().into_bytes(),
            Value::Num(v) => format_float(*v).into_bytes(),
            Value::Str(s) => s.to_vec(),
            Value::Table(t) => format!("table: 0x{:08x}", t.0).into_bytes(),
            Value::Function(f) => format!("function: 0x{:08x}", f.0).into_bytes(),
            Value::UserData(u) => format!("userdata: 0x{:08x}", u).into_bytes(),
        }
    }
    pub fn raw_eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Nil, Value::Nil) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Num(a), Value::Num(b)) => a == b,
            (Value::Int(a), Value::Num(b)) | (Value::Num(b), Value::Int(a)) => (*a as f64) == *b && b.fract() == 0. && (*b as i64) == *a,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Table(a), Value::Table(b)) => a == b,
            (Value::Function(a), Value::Function(b)) => a == b,
            (Value::UserData(a), Value::UserData(b)) => a == b,
            _ => false,
        }
    }
}

/// `%.14g` as in Lua 5.3, with a `.0` suffix for integral floats.
pub fn format_float(v: f64) -> String {
    if v.is_nan() {
        return if v.is_sign_negative() { "-nan".into() } else { "nan".into() };
    }
    if v.is_infinite() {
        return if v > 0. { "inf".into() } else { "-inf".into() };
    }
    let text = crate::stdlib::format_g(v, 14, false);
    if text.contains(['.', 'e', 'n', 'i']) { text } else { format!("{text}.0") }
}

pub fn parse_number(text: &str) -> Option<Value> {
    let (negative, body) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    if let Some(hex) = body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
        if !hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            let mut v: i64 = 0;
            for b in hex.bytes() {
                v = v.wrapping_mul(16).wrapping_add(i64::from((b as char).to_digit(16)?));
            }
            return Some(Value::Int(if negative { v.wrapping_neg() } else { v }));
        }
        return None;
    }
    if body.is_empty() || !body.bytes().all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'e' | b'E' | b'+' | b'-')) {
        return None;
    }
    if body.bytes().all(|b| b.is_ascii_digit()) {
        if let Ok(v) = body.parse::<i64>() {
            return Some(Value::Int(if negative { v.wrapping_neg() } else { v }));
        }
    }
    let v: f64 = body.parse().ok()?;
    Some(Value::Num(if negative { -v } else { v }))
}

/// Normalized table key (floats with integer values become integers).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Bool(bool),
    Int(i64),
    Num(u64),
    Str(Bytes),
    Table(TableRef),
    Function(FuncRef),
    UserData(u32),
}
impl Key {
    pub fn from_value(value: &Value) -> Result<Option<Key>> {
        Ok(Some(match value {
            Value::Nil => return Ok(None),
            Value::Bool(v) => Key::Bool(*v),
            Value::Int(v) => Key::Int(*v),
            Value::Num(v) => {
                if v.is_nan() {
                    bail!("table index is NaN");
                }
                if v.fract() == 0. && *v >= -9.2233720368547758e18 && *v < 9.2233720368547758e18 {
                    Key::Int(*v as i64)
                } else {
                    Key::Num(v.to_bits())
                }
            }
            Value::Str(s) => Key::Str(s.clone()),
            Value::Table(t) => Key::Table(*t),
            Value::Function(f) => Key::Function(*f),
            Value::UserData(u) => Key::UserData(*u),
        }))
    }
    pub fn value(&self) -> Value {
        match self {
            Key::Bool(v) => Value::Bool(*v),
            Key::Int(v) => Value::Int(*v),
            Key::Num(v) => Value::Num(f64::from_bits(*v)),
            Key::Str(s) => Value::Str(s.clone()),
            Key::Table(t) => Value::Table(*t),
            Key::Function(f) => Value::Function(*f),
            Key::UserData(u) => Value::UserData(*u),
        }
    }
}

/// Array part for keys 1..=n plus an insertion-ordered hash part, so `next`
/// is deterministic across runs and platforms.
#[derive(Clone, Debug, Default)]
pub struct Table {
    pub array: Vec<Value>,
    entries: Vec<(Key, Value)>,
    index: HashMap<Key, usize>,
    live: usize,
}
impl Table {
    pub fn get(&self, key: &Value) -> Value {
        match key {
            Value::Int(i) => self.get_int(*i),
            Value::Num(v) if v.fract() == 0. => self.get_int(*v as i64),
            Value::Nil => Value::Nil,
            _ => match Key::from_value(key) {
                Ok(Some(key)) => self.get_key(&key),
                _ => Value::Nil,
            },
        }
    }
    pub fn get_str(&self, key: &str) -> Value {
        self.get_key(&Key::Str(Arc::from(key.as_bytes())))
    }
    pub fn get_int(&self, i: i64) -> Value {
        if i >= 1 && (i as usize) <= self.array.len() {
            return self.array[i as usize - 1].clone();
        }
        self.get_key(&Key::Int(i))
    }
    fn get_key(&self, key: &Key) -> Value {
        self.index.get(key).map_or(Value::Nil, |n| self.entries[*n].1.clone())
    }
    pub fn set(&mut self, key: &Value, value: Value) -> Result<()> {
        let Some(key) = Key::from_value(key)? else { bail!("table index is nil") };
        self.set_key(key, value);
        Ok(())
    }
    pub fn set_key(&mut self, key: Key, value: Value) {
        if let Key::Int(i) = key {
            let len = self.array.len() as i64;
            if i >= 1 && i <= len {
                self.array[i as usize - 1] = value;
                if i == len {
                    while self.array.last().is_some_and(Value::is_nil) {
                        self.array.pop();
                    }
                }
                return;
            }
            if i == len + 1 && !value.is_nil() {
                self.remove_key(&key);
                self.array.push(value);
                //Migrate following keys from the hash part.
                loop {
                    let next = Key::Int(self.array.len() as i64 + 1);
                    let Some(&n) = self.index.get(&next) else { break };
                    let moved = std::mem::take(&mut self.entries[n].1);
                    if moved.is_nil() {
                        break;
                    }
                    self.remove_key(&next);
                    self.array.push(moved);
                }
                return;
            }
        }
        if value.is_nil() {
            self.remove_key(&key);
            return;
        }
        match self.index.get(&key) {
            Some(&n) => {
                if self.entries[n].1.is_nil() {
                    self.live += 1;
                }
                self.entries[n].1 = value;
            }
            None => {
                self.index.insert(key.clone(), self.entries.len());
                self.entries.push((key, value));
                self.live += 1;
            }
        }
    }
    fn remove_key(&mut self, key: &Key) {
        if let Some(&n) = self.index.get(key) {
            if !self.entries[n].1.is_nil() {
                self.entries[n].1 = Value::Nil;
                self.live -= 1;
            }
        }
        //Compact tombstones when they dominate.
        if self.entries.len() > 32 && self.live * 2 < self.entries.len() {
            self.entries.retain(|(_, v)| !v.is_nil());
            self.index = self.entries.iter().enumerate().map(|(n, (k, _))| (k.clone(), n)).collect();
        }
    }
    /// Border (`#t`).
    pub fn len(&self) -> i64 {
        let mut n = self.array.len() as i64;
        while !self.get_key(&Key::Int(n + 1)).is_nil() {
            n += 1;
        }
        n
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// `next(t, key)`.
    pub fn next(&self, key: &Value) -> Result<Option<(Value, Value)>> {
        let mut start_hash = 0;
        let mut array_from = 0usize;
        match key {
            Value::Nil => {}
            _ => {
                let k = Key::from_value(key)?.expect("non-nil");
                match k {
                    Key::Int(i) if i >= 1 && (i as usize) <= self.array.len() => array_from = i as usize,
                    _ => {
                        array_from = usize::MAX;
                        let Some(&n) = self.index.get(&k) else { bail!("invalid key to 'next'") };
                        start_hash = n + 1;
                    }
                }
            }
        }
        if array_from != usize::MAX {
            for i in array_from..self.array.len() {
                if !self.array[i].is_nil() {
                    return Ok(Some((Value::Int(i as i64 + 1), self.array[i].clone())));
                }
            }
        }
        for (k, v) in &self.entries[start_hash.min(self.entries.len())..] {
            if !v.is_nil() {
                return Ok(Some((k.value(), v.clone())));
            }
        }
        Ok(None)
    }
    pub fn values(&self) -> impl Iterator<Item = (Value, &Value)> {
        self.array.iter().enumerate().map(|(i, v)| (Value::Int(i as i64 + 1), v)).chain(self.entries.iter().map(|(k, v)| (k.value(), v)))
    }
}

#[derive(Clone, Debug)]
pub enum Function {
    Lua { proto: Arc<Proto>, upvalues: Vec<u32> },
    /// Built-in library function with captured state (iterators).
    Builtin { id: crate::stdlib::Builtin, state: Vec<Value> },
    /// Host function, dispatched through `Host::call`.
    Host(u32),
}

#[derive(Clone, Debug)]
pub enum Cell {
    Open(usize),
    Closed(Value),
}

/// Arena with a free list; slots hold `None` once collected.
#[derive(Debug)]
pub struct Arena<T> {
    pub slots: Vec<Option<T>>,
    free: Vec<u32>,
}
impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self { slots: Vec::new(), free: Vec::new() }
    }
}
impl<T> Arena<T> {
    pub fn insert(&mut self, value: T) -> u32 {
        match self.free.pop() {
            Some(n) => {
                self.slots[n as usize] = Some(value);
                n
            }
            None => {
                self.slots.push(Some(value));
                (self.slots.len() - 1) as u32
            }
        }
    }
    pub fn get(&self, n: u32) -> Option<&T> {
        self.slots.get(n as usize).and_then(Option::as_ref)
    }
    pub fn get_mut(&mut self, n: u32) -> Option<&mut T> {
        self.slots.get_mut(n as usize).and_then(Option::as_mut)
    }
    pub fn retain(&mut self, marks: &[bool]) -> usize {
        let mut freed = 0;
        for (n, slot) in self.slots.iter_mut().enumerate() {
            if slot.is_some() && !marks.get(n).copied().unwrap_or(false) {
                *slot = None;
                self.free.push(n as u32);
                freed += 1;
            }
        }
        freed
    }
    pub fn live(&self) -> usize {
        self.slots.len() - self.free.len()
    }
}

// SPDX-License-Identifier: MPL-2.0
//! Base, string, math and table libraries. `io` and `os` are host policy
//! (storage and clocks belong to the embedding engine) and are not opened.
use crate::interp::{Host, Lua};
use crate::value::{format_float, Value};
use anyhow::{bail, ensure, Context, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Builtin {
    Print,
    Type,
    ToString,
    ToNumber,
    Next,
    Pairs,
    IPairs,
    IPairsIter,
    Select,
    Assert,
    Error,
    PCall,
    RawGet,
    RawSet,
    RawEqual,
    RawLen,
    Unpack,
    StrLen,
    StrSub,
    StrUpper,
    StrLower,
    StrRep,
    StrByte,
    StrChar,
    StrFormat,
    StrFind,
    StrMatch,
    StrGmatch,
    GmatchIter,
    StrGsub,
    StrReverse,
    MathFloor,
    MathCeil,
    MathAbs,
    MathMax,
    MathMin,
    MathSqrt,
    MathModf,
    MathFmod,
    MathRandom,
    MathRandomSeed,
    MathToInteger,
    MathType,
    MathSin,
    MathCos,
    MathExp,
    MathLog,
    TableInsert,
    TableRemove,
    TableConcat,
    TableSort,
}

pub fn open(lua: &mut Lua) {
    use Builtin::*;
    let base = [
        ("print", Print), ("type", Type), ("tostring", ToString), ("tonumber", ToNumber), ("next", Next), ("pairs", Pairs),
        ("ipairs", IPairs), ("select", Select), ("assert", Assert), ("error", Error), ("pcall", PCall), ("rawget", RawGet),
        ("rawset", RawSet), ("rawequal", RawEqual), ("rawlen", RawLen),
    ];
    for (name, id) in base {
        let f = lua.builtin(id, vec![]);
        lua.set_global(name, f).expect("globals");
    }
    let string = [
        ("len", StrLen), ("sub", StrSub), ("upper", StrUpper), ("lower", StrLower), ("rep", StrRep), ("byte", StrByte),
        ("char", StrChar), ("format", StrFormat), ("find", StrFind), ("match", StrMatch), ("gmatch", StrGmatch),
        ("gfind", StrGmatch), ("gsub", StrGsub), ("reverse", StrReverse),
    ];
    let lib = lua.string_lib;
    for (name, id) in string {
        let f = lua.builtin(id, vec![]);
        lua.set_field(lib, name, f).expect("string");
    }
    lua.set_global("string", Value::Table(lib)).expect("globals");
    let math = lua.new_table();
    for (name, id) in [
        ("floor", MathFloor), ("ceil", MathCeil), ("abs", MathAbs), ("max", MathMax), ("min", MathMin), ("sqrt", MathSqrt),
        ("modf", MathModf), ("fmod", MathFmod), ("random", MathRandom), ("randomseed", MathRandomSeed),
        ("tointeger", MathToInteger), ("type", MathType), ("sin", MathSin), ("cos", MathCos), ("exp", MathExp), ("log", MathLog),
    ] {
        let f = lua.builtin(id, vec![]);
        lua.set_field(math, name, f).expect("math");
    }
    lua.set_field(math, "pi", Value::Num(std::f64::consts::PI)).expect("math");
    lua.set_field(math, "huge", Value::Num(f64::INFINITY)).expect("math");
    lua.set_field(math, "maxinteger", Value::Int(i64::MAX)).expect("math");
    lua.set_field(math, "mininteger", Value::Int(i64::MIN)).expect("math");
    lua.set_global("math", Value::Table(math)).expect("globals");
    let table = lua.new_table();
    for (name, id) in [("insert", TableInsert), ("remove", TableRemove), ("concat", TableConcat), ("sort", TableSort), ("unpack", Unpack)] {
        let f = lua.builtin(id, vec![]);
        lua.set_field(table, name, f).expect("table");
    }
    lua.set_global("table", Value::Table(table)).expect("globals");
    let unpack = lua.builtin(Unpack, vec![]);
    lua.set_global("unpack", unpack).expect("globals");
}

fn arg(args: &[Value], n: usize) -> Value {
    args.get(n).cloned().unwrap_or_default()
}
fn bytes_arg(args: &[Value], n: usize, name: &str) -> Result<Vec<u8>> {
    match args.get(n) {
        Some(Value::Str(s)) => Ok(s.to_vec()),
        Some(v @ (Value::Int(_) | Value::Num(_))) => Ok(v.display()),
        other => bail!("bad argument #{} to '{name}' (string expected, got {})", n + 1, other.map_or("no value", Value::type_name)),
    }
}
fn int_arg(args: &[Value], n: usize, name: &str) -> Result<i64> {
    args.get(n).and_then(Value::as_i64).with_context(|| format!("bad argument #{} to '{name}' (number has no integer representation)", n + 1))
}
fn opt_int(args: &[Value], n: usize, default: i64, name: &str) -> Result<i64> {
    match args.get(n) {
        None | Some(Value::Nil) => Ok(default),
        _ => int_arg(args, n, name),
    }
}
fn num_arg(args: &[Value], n: usize, name: &str) -> Result<f64> {
    args.get(n).and_then(Value::as_f64).with_context(|| format!("bad argument #{} to '{name}' (number expected)", n + 1))
}
fn table_arg(args: &[Value], n: usize, name: &str) -> Result<crate::value::TableRef> {
    match args.get(n) {
        Some(Value::Table(t)) => Ok(*t),
        other => bail!("bad argument #{} to '{name}' (table expected, got {})", n + 1, other.map_or("no value", Value::type_name)),
    }
}

/// Lua string index (1-based, negatives from the end) to a byte offset.
fn start_index(i: i64, len: usize) -> usize {
    if i > 0 {
        (i as usize - 1).min(len)
    } else if i == 0 {
        0
    } else {
        len.saturating_sub((-i) as usize)
    }
}
fn end_index(j: i64, len: usize) -> usize {
    if j >= 0 {
        (j as usize).min(len)
    } else {
        (len as i64 + j + 1).max(0) as usize
    }
}

pub fn tostring(lua: &Lua, value: &Value) -> Vec<u8> {
    let _ = lua;
    value.display()
}

pub fn call(lua: &mut Lua, host: &mut dyn Host, this: crate::value::FuncRef, id: Builtin, state: &[Value], args: Vec<Value>) -> Result<Vec<Value>> {
    use Builtin::*;
    Ok(match id {
        Print => {
            let text: Vec<Vec<u8>> = args.iter().map(|v| tostring(lua, v)).collect();
            host.print(&text.join(&b'\t'));
            vec![]
        }
        Type => vec![Value::str(args.first().context("bad argument #1 to 'type' (value expected)")?.type_name())],
        ToString => vec![Value::bytes(&tostring(lua, &arg(&args, 0)))],
        ToNumber => {
            let value = arg(&args, 0);
            match args.get(1) {
                None | Some(Value::Nil) => vec![value.to_number().unwrap_or_default()],
                Some(base) => {
                    let base = base.as_i64().context("tonumber base")?;
                    ensure!((2..=36).contains(&base), "bad argument #2 to 'tonumber' (base out of range)");
                    let text = String::from_utf8_lossy(value.as_bytes().context("tonumber string")?).trim().to_ascii_lowercase();
                    vec![i64::from_str_radix(&text, base as u32).map(Value::Int).unwrap_or_default()]
                }
            }
        }
        Next => {
            let t = table_arg(&args, 0, "next")?;
            match lua.table(t)?.next(&arg(&args, 1))? {
                Some((k, v)) => vec![k, v],
                None => vec![Value::Nil],
            }
        }
        Pairs => {
            let t = table_arg(&args, 0, "pairs")?;
            let next = lua.builtin(Next, vec![]);
            vec![next, Value::Table(t), Value::Nil]
        }
        IPairs => {
            let t = arg(&args, 0);
            let iter = lua.builtin(IPairsIter, vec![]);
            vec![iter, t, Value::Int(0)]
        }
        IPairsIter => {
            let t = table_arg(&args, 0, "ipairs")?;
            let i = int_arg(&args, 1, "ipairs")? + 1;
            let v = lua.table(t)?.get_int(i);
            if v.is_nil() { vec![Value::Nil] } else { vec![Value::Int(i), v] }
        }
        Select => {
            let n = arg(&args, 0);
            if n.as_bytes() == Some(b"#") {
                vec![Value::Int(args.len() as i64 - 1)]
            } else {
                let n = int_arg(&args, 0, "select")?;
                let len = args.len() as i64 - 1;
                let from = if n < 0 { len + n + 1 } else { n };
                ensure!(from >= 1, "bad argument #1 to 'select' (index out of range)");
                args.into_iter().skip(from as usize).collect()
            }
        }
        Assert => {
            if !arg(&args, 0).truthy() {
                let message = args.get(1).map(|v| String::from_utf8_lossy(&v.display()).into_owned()).unwrap_or_else(|| "assertion failed!".into());
                bail!("{message}");
            }
            args
        }
        Error => bail!("{}", String::from_utf8_lossy(&arg(&args, 0).display())),
        PCall => {
            let mut args = args.into_iter();
            let func = args.next().unwrap_or_default();
            match lua.call(&func, args.collect(), host) {
                Ok(mut results) => {
                    results.insert(0, Value::Bool(true));
                    results
                }
                Err(error) => vec![Value::Bool(false), Value::str(&format!("{error:#}"))],
            }
        }
        RawGet => {
            let t = table_arg(&args, 0, "rawget")?;
            vec![lua.table(t)?.get(&arg(&args, 1))]
        }
        RawSet => {
            let t = table_arg(&args, 0, "rawset")?;
            lua.table_mut(t)?.set(&arg(&args, 1), arg(&args, 2))?;
            vec![Value::Table(t)]
        }
        RawEqual => vec![Value::Bool(arg(&args, 0).raw_eq(&arg(&args, 1)))],
        RawLen => match arg(&args, 0) {
            Value::Table(t) => vec![Value::Int(lua.table(t)?.len())],
            Value::Str(s) => vec![Value::Int(s.len() as i64)],
            _ => bail!("table or string expected"),
        },
        Unpack => {
            let t = table_arg(&args, 0, "unpack")?;
            let table = lua.table(t)?;
            let i = opt_int(&args, 1, 1, "unpack")?;
            let j = match args.get(2) {
                None | Some(Value::Nil) => table.len(),
                _ => int_arg(&args, 2, "unpack")?,
            };
            ensure!(j - i < 1_000_000, "too many results to unpack");
            (i..=j).map(|n| table.get_int(n)).collect()
        }
        StrLen => vec![Value::Int(bytes_arg(&args, 0, "len")?.len() as i64)],
        StrSub => {
            let s = bytes_arg(&args, 0, "sub")?;
            let i = opt_int(&args, 1, 1, "sub")?;
            let j = opt_int(&args, 2, -1, "sub")?;
            let (from, to) = (start_index(i, s.len()), end_index(j, s.len()));
            vec![Value::bytes(if from < to { &s[from..to] } else { b"" })]
        }
        StrUpper => vec![Value::bytes(&bytes_arg(&args, 0, "upper")?.to_ascii_uppercase())],
        StrLower => vec![Value::bytes(&bytes_arg(&args, 0, "lower")?.to_ascii_lowercase())],
        StrRep => {
            let s = bytes_arg(&args, 0, "rep")?;
            let n = int_arg(&args, 1, "rep")?.max(0) as usize;
            ensure!(s.len().saturating_mul(n) <= 64 << 20, "resulting string too large");
            vec![Value::bytes(&s.repeat(n))]
        }
        StrReverse => {
            let mut s = bytes_arg(&args, 0, "reverse")?;
            s.reverse();
            vec![Value::bytes(&s)]
        }
        StrByte => {
            let s = bytes_arg(&args, 0, "byte")?;
            let i = opt_int(&args, 1, 1, "byte")?;
            let j = opt_int(&args, 2, i, "byte")?;
            let (from, to) = (start_index(i, s.len()), end_index(j, s.len()));
            if from < to { s[from..to].iter().map(|b| Value::Int(i64::from(*b))).collect() } else { vec![] }
        }
        StrChar => {
            let out = args.iter().enumerate().map(|(n, _)| int_arg(&args, n, "char").and_then(|v| u8::try_from(v).context("value out of range"))).collect::<Result<Vec<u8>>>()?;
            vec![Value::bytes(&out)]
        }
        StrFormat => vec![Value::bytes(&format(lua, &args)?)],
        StrFind | StrMatch => {
            let s = bytes_arg(&args, 0, "find")?;
            let p = bytes_arg(&args, 1, "find")?;
            let init = start_index(opt_int(&args, 2, 1, "find")?, s.len());
            if init > s.len() {
                return Ok(vec![Value::Nil]);
            }
            let plain = id == StrFind && (arg(&args, 3).truthy() || !p.iter().any(|b| b"^$*+?.([%-".contains(b)));
            if plain {
                return Ok(match find_plain(&s[init..], &p) {
                    Some(at) => vec![Value::Int((init + at + 1) as i64), Value::Int((init + at + p.len()) as i64)],
                    None => vec![Value::Nil],
                });
            }
            match pattern_find(&s, &p, init)? {
                Some((start, end, captures)) => {
                    let mut out = Vec::new();
                    if id == StrFind {
                        out.push(Value::Int(start as i64 + 1));
                        out.push(Value::Int(end as i64));
                        out.extend(captures.iter().map(|c| capture_value(&s, c)));
                    } else if captures.is_empty() {
                        out.push(Value::bytes(&s[start..end]));
                    } else {
                        out.extend(captures.iter().map(|c| capture_value(&s, c)));
                    }
                    out
                }
                None => vec![Value::Nil],
            }
        }
        StrGmatch => {
            let s = bytes_arg(&args, 0, "gmatch")?;
            let p = bytes_arg(&args, 1, "gmatch")?;
            vec![lua.builtin(GmatchIter, vec![Value::bytes(&s), Value::bytes(&p), Value::Int(0)])]
        }
        GmatchIter => {
            let s = state[0].as_bytes().context("gmatch state")?.to_vec();
            let p = state[1].as_bytes().context("gmatch state")?.to_vec();
            //State lives in the closure; retrieve and advance it.
            let at = state[2].as_i64().unwrap_or(0) as usize;
            let mut start = at;
            while start <= s.len() {
                if let Some((m_start, m_end, captures)) = pattern_match_at(&s, &p, start)? {
                    let next = if m_end == m_start { m_end + 1 } else { m_end };
                    set_gmatch_position(lua, this, next)?;
                    return Ok(if captures.is_empty() {
                        vec![Value::bytes(&s[m_start..m_end])]
                    } else {
                        captures.iter().map(|c| capture_value(&s, c)).collect()
                    });
                }
                start += 1;
            }
            set_gmatch_position(lua, this, s.len() + 1)?;
            vec![Value::Nil]
        }
        StrGsub => {
            let s = bytes_arg(&args, 0, "gsub")?;
            let p = bytes_arg(&args, 1, "gsub")?;
            let replacement = arg(&args, 2);
            let max = match args.get(3) {
                None | Some(Value::Nil) => usize::MAX,
                _ => int_arg(&args, 3, "gsub")?.max(0) as usize,
            };
            let anchored = p.first() == Some(&b'^');
            let mut out = Vec::new();
            let mut at = 0;
            let mut count = 0;
            while count < max {
                match pattern_match_at(&s, &p, at)? {
                    Some((start, end, captures)) => {
                        count += 1;
                        let whole = Value::bytes(&s[start..end]);
                        let first = captures.first().map(|c| capture_value(&s, c)).unwrap_or_else(|| whole.clone());
                        let value = match &replacement {
                            Value::Str(r) => {
                                let mut text = Vec::new();
                                let mut i = 0;
                                while i < r.len() {
                                    if r[i] == b'%' && i + 1 < r.len() {
                                        let d = r[i + 1];
                                        if d == b'0' {
                                            text.extend_from_slice(&s[start..end]);
                                        } else if d.is_ascii_digit() {
                                            let c = captures.get(usize::from(d - b'1')).map(|c| capture_value(&s, c)).unwrap_or_else(|| whole.clone());
                                            text.extend(c.display());
                                        } else {
                                            text.push(d);
                                        }
                                        i += 2;
                                    } else {
                                        text.push(r[i]);
                                        i += 1;
                                    }
                                }
                                Value::bytes(&text)
                            }
                            Value::Int(_) | Value::Num(_) => Value::bytes(&replacement.display()),
                            Value::Table(t) => lua.table(*t)?.get(&first),
                            Value::Function(_) => {
                                let call_args = if captures.is_empty() { vec![whole.clone()] } else { captures.iter().map(|c| capture_value(&s, c)).collect() };
                                lua.call(&replacement, call_args, host)?.into_iter().next().unwrap_or_default()
                            }
                            _ => bail!("bad argument #3 to 'gsub' (string/function/table expected)"),
                        };
                        match value {
                            Value::Nil | Value::Bool(false) => out.extend_from_slice(&s[start..end]),
                            other => out.extend(other.display()),
                        }
                        if end > start {
                            at = end;
                        } else {
                            if start < s.len() {
                                out.push(s[start]);
                            }
                            at = start + 1;
                        }
                    }
                    None => {
                        if at < s.len() {
                            out.push(s[at]);
                        }
                        at += 1;
                    }
                }
                if at > s.len() || anchored {
                    break;
                }
            }
            if at < s.len() {
                out.extend_from_slice(&s[at..]);
            }
            vec![Value::bytes(&out), Value::Int(count as i64)]
        }
        MathFloor | MathCeil => match arg(&args, 0).to_number() {
            Some(Value::Int(v)) => vec![Value::Int(v)],
            Some(Value::Num(v)) => {
                let r = if id == MathFloor { v.floor() } else { v.ceil() };
                vec![if r >= -9.2233720368547758e18 && r < 9.2233720368547758e18 { Value::Int(r as i64) } else { Value::Num(r) }]
            }
            _ => bail!("bad argument #1 to 'floor' (number expected)"),
        },
        MathAbs => match arg(&args, 0).to_number() {
            Some(Value::Int(v)) => vec![Value::Int(v.wrapping_abs())],
            Some(Value::Num(v)) => vec![Value::Num(v.abs())],
            _ => bail!("bad argument #1 to 'abs' (number expected)"),
        },
        MathMax | MathMin => {
            ensure!(!args.is_empty(), "bad argument #1 to 'max' (number expected)");
            let mut best = args[0].to_number().context("number expected")?;
            for v in &args[1..] {
                let v = v.to_number().context("number expected")?;
                let replace = if id == MathMax { crate::interp::less(&best, &v, false)? } else { crate::interp::less(&v, &best, false)? };
                if replace {
                    best = v;
                }
            }
            vec![best]
        }
        MathSqrt => vec![Value::Num(num_arg(&args, 0, "sqrt")?.sqrt())],
        MathSin => vec![Value::Num(num_arg(&args, 0, "sin")?.sin())],
        MathCos => vec![Value::Num(num_arg(&args, 0, "cos")?.cos())],
        MathExp => vec![Value::Num(num_arg(&args, 0, "exp")?.exp())],
        MathLog => {
            let x = num_arg(&args, 0, "log")?;
            vec![Value::Num(match args.get(1) {
                None | Some(Value::Nil) => x.ln(),
                _ => x.log(num_arg(&args, 1, "log")?),
            })]
        }
        MathModf => {
            let x = num_arg(&args, 0, "modf")?;
            let int = if x.is_infinite() { x } else { x.trunc() };
            let frac = if x.is_infinite() { 0. } else { x - int };
            let int = if int.fract() == 0. && int.abs() < 9.2e18 { Value::Num(int) } else { Value::Num(int) };
            vec![int, Value::Num(frac)]
        }
        MathFmod => {
            let (a, b) = (num_arg(&args, 0, "fmod")?, num_arg(&args, 1, "fmod")?);
            vec![Value::Num(a % b)]
        }
        MathRandom => {
            let r = lua.next_random();
            match args.len() {
                0 => vec![Value::Num((r >> 11) as f64 / (1u64 << 53) as f64)],
                _ => {
                    let (low, high) = if args.len() == 1 { (1, int_arg(&args, 0, "random")?) } else { (int_arg(&args, 0, "random")?, int_arg(&args, 1, "random")?) };
                    ensure!(low <= high, "bad argument to 'random' (interval is empty)");
                    let span = (high as i128 - low as i128 + 1) as u128;
                    vec![Value::Int((low as i128 + (u128::from(r) % span) as i128) as i64)]
                }
            }
        }
        MathRandomSeed => {
            let seed = arg(&args, 0).as_f64().unwrap_or(0.);
            lua.random = (seed.to_bits() ^ 0x9e37_79b9_7f4a_7c15) | 1;
            vec![]
        }
        MathToInteger => vec![arg(&args, 0).as_i64().map(Value::Int).unwrap_or_default()],
        MathType => vec![match arg(&args, 0) {
            Value::Int(_) => Value::str("integer"),
            Value::Num(_) => Value::str("float"),
            _ => Value::Nil,
        }],
        TableInsert => {
            let t = table_arg(&args, 0, "insert")?;
            let len = lua.table(t)?.len();
            match args.len() {
                2 => lua.table_mut(t)?.set(&Value::Int(len + 1), args[1].clone())?,
                3 => {
                    let pos = int_arg(&args, 1, "insert")?;
                    ensure!(pos >= 1 && pos <= len + 1, "bad argument #2 to 'insert' (position out of bounds)");
                    let table = lua.table_mut(t)?;
                    let mut i = len;
                    while i >= pos {
                        let v = table.get_int(i);
                        table.set(&Value::Int(i + 1), v)?;
                        i -= 1;
                    }
                    table.set(&Value::Int(pos), args[2].clone())?;
                }
                _ => bail!("wrong number of arguments to 'insert'"),
            }
            vec![]
        }
        TableRemove => {
            let t = table_arg(&args, 0, "remove")?;
            let len = lua.table(t)?.len();
            let pos = opt_int(&args, 1, len, "remove")?;
            let table = lua.table_mut(t)?;
            if len == 0 && args.len() < 2 {
                return Ok(vec![Value::Nil]);
            }
            let removed = table.get_int(pos);
            let mut i = pos;
            while i < len {
                let v = table.get_int(i + 1);
                table.set(&Value::Int(i), v)?;
                i += 1;
            }
            if pos <= len {
                table.set(&Value::Int(len), Value::Nil)?;
            }
            vec![removed]
        }
        TableConcat => {
            let t = table_arg(&args, 0, "concat")?;
            let sep = match args.get(1) {
                None | Some(Value::Nil) => Vec::new(),
                _ => bytes_arg(&args, 1, "concat")?,
            };
            let table = lua.table(t)?;
            let i = opt_int(&args, 2, 1, "concat")?;
            let j = match args.get(3) {
                None | Some(Value::Nil) => table.len(),
                _ => int_arg(&args, 3, "concat")?,
            };
            let mut out = Vec::new();
            for n in i..=j {
                if n > i {
                    out.extend_from_slice(&sep);
                }
                match table.get_int(n) {
                    v @ (Value::Str(_) | Value::Int(_) | Value::Num(_)) => out.extend(v.display()),
                    other => bail!("invalid value (at index {n}) in table for 'concat' ({})", other.type_name()),
                }
            }
            vec![Value::bytes(&out)]
        }
        TableSort => {
            let t = table_arg(&args, 0, "sort")?;
            let len = lua.table(t)?.len();
            let mut items: Vec<Value> = (1..=len).map(|n| lua.table(t).map(|tb| tb.get_int(n))).collect::<Result<_>>()?;
            let comparator = arg(&args, 1);
            //Insertion-stable merge sort through the comparator (may call Lua).
            let mut error = None;
            let mut less_than = |lua: &mut Lua, x: &Value, y: &Value| -> bool {
                if error.is_some() {
                    return false;
                }
                let result = if comparator.is_nil() {
                    crate::interp::less(x, y, false)
                } else {
                    lua.call(&comparator, vec![x.clone(), y.clone()], host).map(|r| r.first().is_some_and(Value::truthy))
                };
                result.unwrap_or_else(|e| {
                    error = Some(e);
                    false
                })
            };
            let mut sorted: Vec<Value> = Vec::with_capacity(items.len());
            for item in items.drain(..) {
                let mut lo = 0;
                let mut hi = sorted.len();
                while lo < hi {
                    let mid = (lo + hi) / 2;
                    if less_than(lua, &item, &sorted[mid]) {
                        hi = mid;
                    } else {
                        lo = mid + 1;
                    }
                }
                sorted.insert(lo, item);
            }
            if let Some(error) = error {
                return Err(error);
            }
            let table = lua.table_mut(t)?;
            for (n, v) in sorted.into_iter().enumerate() {
                table.set(&Value::Int(n as i64 + 1), v)?;
            }
            vec![]
        }
    })
}

fn set_gmatch_position(lua: &mut Lua, this: crate::value::FuncRef, at: usize) -> Result<()> {
    match lua.funcs.get_mut(this.0) {
        Some(crate::value::Function::Builtin { id: Builtin::GmatchIter, state }) => {
            state[2] = Value::Int(at as i64);
            Ok(())
        }
        _ => bail!("gmatch iterator state lost"),
    }
}

fn find_plain(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[derive(Clone, Copy, Debug)]
enum Capture {
    Range(usize, usize),
    Position(usize),
}
fn capture_value(s: &[u8], c: &Capture) -> Value {
    match c {
        Capture::Range(a, b) => Value::bytes(&s[*a..*b]),
        Capture::Position(p) => Value::Int(*p as i64 + 1),
    }
}

/// First match at or after `init`: (start, end, captures).
fn pattern_find(s: &[u8], p: &[u8], init: usize) -> Result<Option<(usize, usize, Vec<Capture>)>> {
    let anchored = p.first() == Some(&b'^');
    let mut at = init;
    loop {
        if let Some(found) = pattern_match_at(s, p, at)? {
            return Ok(Some(found));
        }
        at += 1;
        if anchored || at > s.len() {
            return Ok(None);
        }
    }
}

fn pattern_match_at(s: &[u8], p: &[u8], at: usize) -> Result<Option<(usize, usize, Vec<Capture>)>> {
    let (p, _anchored) = match p.first() {
        Some(b'^') => (&p[1..], true),
        _ => (p, false),
    };
    let mut m = Matcher { s, p, level: 0, captures: [(0, CAP_UNFINISHED); 32], depth: 0 };
    match m.do_match(at, 0)? {
        Some(end) => {
            let captures = (0..m.level)
                .map(|n| {
                    let (start, len) = m.captures[n];
                    if len == CAP_POSITION { Capture::Position(start) } else { Capture::Range(start, start + len as usize) }
                })
                .collect();
            Ok(Some((at, end, captures)))
        }
        None => Ok(None),
    }
}

const CAP_UNFINISHED: isize = -1;
const CAP_POSITION: isize = -2;

struct Matcher<'a> {
    s: &'a [u8],
    p: &'a [u8],
    level: usize,
    captures: [(usize, isize); 32],
    depth: usize,
}

fn class_matches(c: u8, class: u8) -> bool {
    let result = match class.to_ascii_lowercase() {
        b'a' => c.is_ascii_alphabetic(),
        b'c' => c.is_ascii_control(),
        b'd' => c.is_ascii_digit(),
        b'g' => c.is_ascii_graphic(),
        b'l' => c.is_ascii_lowercase(),
        b'p' => c.is_ascii_punctuation(),
        b's' => c.is_ascii_whitespace() || c == 0x0b,
        b'u' => c.is_ascii_uppercase(),
        b'w' => c.is_ascii_alphanumeric(),
        b'x' => c.is_ascii_hexdigit(),
        _ => return class == c,
    };
    if class.is_ascii_uppercase() { !result } else { result }
}

impl Matcher<'_> {
    fn class_end(&self, mut pi: usize) -> Result<usize> {
        let p = self.p;
        let c = p[pi];
        pi += 1;
        if c == b'%' {
            ensure!(pi < p.len(), "malformed pattern (ends with '%')");
            return Ok(pi + 1);
        }
        if c == b'[' {
            if p.get(pi) == Some(&b'^') {
                pi += 1;
            }
            loop {
                ensure!(pi < p.len(), "malformed pattern (missing ']')");
                let cc = p[pi];
                pi += 1;
                if cc == b'%' {
                    pi += 1;
                } else if cc == b']' && pi > 0 {
                    //A ']' right after '[' or '[^' is literal.
                    return Ok(pi);
                }
                if pi < p.len() && p[pi] == b']' {
                    return Ok(pi + 1);
                }
            }
        }
        Ok(pi)
    }
    fn set_matches(&self, c: u8, start: usize, end: usize) -> bool {
        //p[start] == '[' and p[end-1] == ']'.
        let p = self.p;
        let mut i = start + 1;
        let negate = p.get(i) == Some(&b'^');
        if negate {
            i += 1;
        }
        let close = end - 1;
        let mut found = false;
        while i < close {
            if p[i] == b'%' && i + 1 < close {
                if class_matches(c, p[i + 1]) {
                    found = true;
                }
                i += 2;
            } else if i + 2 < close && p[i + 1] == b'-' {
                if p[i] <= c && c <= p[i + 2] {
                    found = true;
                }
                i += 3;
            } else {
                if p[i] == c {
                    found = true;
                }
                i += 1;
            }
        }
        found != negate
    }
    fn single(&self, si: usize, pi: usize, ep: usize) -> bool {
        let Some(&c) = self.s.get(si) else { return false };
        match self.p[pi] {
            b'.' => true,
            b'%' => class_matches(c, self.p[pi + 1]),
            b'[' => self.set_matches(c, pi, ep),
            other => other == c,
        }
    }
    fn do_match(&mut self, mut si: usize, mut pi: usize) -> Result<Option<usize>> {
        self.depth += 1;
        ensure!(self.depth < 200, "pattern too complex");
        let result = 'outer: loop {
            if pi >= self.p.len() {
                break Some(si);
            }
            match self.p[pi] {
                b'(' => {
                    let position = self.p.get(pi + 1) == Some(&b')');
                    ensure!(self.level < 32, "too many captures");
                    self.captures[self.level] = (si, if position { CAP_POSITION } else { CAP_UNFINISHED });
                    self.level += 1;
                    let r = self.do_match(si, pi + if position { 2 } else { 1 })?;
                    if r.is_none() {
                        self.level -= 1;
                    }
                    break r;
                }
                b')' => {
                    let open = (0..self.level).rev().find(|n| self.captures[*n].1 == CAP_UNFINISHED).context("invalid pattern capture")?;
                    self.captures[open].1 = (si - self.captures[open].0) as isize;
                    let r = self.do_match(si, pi + 1)?;
                    if r.is_none() {
                        self.captures[open].1 = CAP_UNFINISHED;
                    }
                    break r;
                }
                b'$' if pi + 1 == self.p.len() => break if si == self.s.len() { Some(si) } else { None },
                b'%' if self.p.get(pi + 1) == Some(&b'b') => {
                    let (open, close) = (*self.p.get(pi + 2).context("missing arguments to '%b'")?, *self.p.get(pi + 3).context("missing arguments to '%b'")?);
                    if self.s.get(si) != Some(&open) {
                        break None;
                    }
                    let mut depth = 1;
                    let mut i = si + 1;
                    let mut end = None;
                    while i < self.s.len() {
                        if self.s[i] == close {
                            depth -= 1;
                            if depth == 0 {
                                end = Some(i + 1);
                                break;
                            }
                        } else if self.s[i] == open {
                            depth += 1;
                        }
                        i += 1;
                    }
                    match end {
                        Some(e) => {
                            si = e;
                            pi += 4;
                            continue;
                        }
                        None => break None,
                    }
                }
                b'%' if self.p.get(pi + 1).is_some_and(u8::is_ascii_digit) => {
                    let n = usize::from(self.p[pi + 1] - b'1');
                    let (start, len) = *self.captures.get(n).context("invalid capture index")?;
                    ensure!(n < self.level && len >= 0, "invalid capture index");
                    let len = len as usize;
                    if self.s.len() - si >= len && self.s[start..start + len] == self.s[si..si + len] {
                        si += len;
                        pi += 2;
                        continue;
                    }
                    break None;
                }
                _ => {
                    let ep = self.class_end(pi)?;
                    let quantifier = self.p.get(ep).copied();
                    match quantifier {
                        Some(b'?') => {
                            if self.single(si, pi, ep) {
                                if let Some(r) = self.do_match(si + 1, ep + 1)? {
                                    break Some(r);
                                }
                            }
                            pi = ep + 1;
                            continue;
                        }
                        Some(b'+') | Some(b'*') => {
                            let min = usize::from(quantifier == Some(b'+'));
                            let mut count = 0;
                            while self.single(si + count, pi, ep) {
                                count += 1;
                            }
                            let mut found = None;
                            let mut n = count as isize;
                            while n >= min as isize {
                                if let Some(r) = self.do_match(si + n as usize, ep + 1)? {
                                    found = Some(r);
                                    break;
                                }
                                n -= 1;
                            }
                            break found;
                        }
                        Some(b'-') => {
                            let mut n = 0;
                            loop {
                                if let Some(r) = self.do_match(si + n, ep + 1)? {
                                    break 'outer Some(r);
                                }
                                if self.single(si + n, pi, ep) {
                                    n += 1;
                                } else {
                                    break 'outer None;
                                }
                            }
                        }
                        _ => {
                            if !self.single(si, pi, ep) {
                                break None;
                            }
                            si += 1;
                            pi = ep;
                            continue;
                        }
                    }
                }
            };
        };
        self.depth -= 1;
        Ok(result)
    }
}

/// `string.format`.
fn format(lua: &Lua, args: &[Value]) -> Result<Vec<u8>> {
    let spec = bytes_arg(args, 0, "format")?;
    let mut out = Vec::new();
    let mut next = 1;
    let mut i = 0;
    while i < spec.len() {
        let c = spec[i];
        i += 1;
        if c != b'%' {
            out.push(c);
            continue;
        }
        ensure!(i < spec.len(), "invalid conversion '%' to 'format'");
        if spec[i] == b'%' {
            out.push(b'%');
            i += 1;
            continue;
        }
        let start = i;
        while i < spec.len() && b"-+ #0".contains(&spec[i]) {
            i += 1;
        }
        while i < spec.len() && spec[i].is_ascii_digit() {
            i += 1;
        }
        if i < spec.len() && spec[i] == b'.' {
            i += 1;
            while i < spec.len() && spec[i].is_ascii_digit() {
                i += 1;
            }
        }
        ensure!(i < spec.len(), "invalid conversion to 'format'");
        let flags = std::str::from_utf8(&spec[start..i])?;
        let conversion = spec[i];
        i += 1;
        let value = arg(args, next);
        ensure!(next < args.len() || conversion == b'%', "bad argument #{} to 'format' (no value)", next + 1);
        next += 1;
        let (left, zero, plus, space) = (flags.contains('-'), flags.contains('0'), flags.contains('+'), flags.contains(' '));
        let digits: String = flags.chars().skip_while(|c| "-+ #0".contains(*c)).collect();
        let (width, precision) = match digits.split_once('.') {
            Some((w, p)) => (w.parse::<usize>().unwrap_or(0), Some(p.parse::<usize>().unwrap_or(0))),
            None => (digits.parse::<usize>().unwrap_or(0), None),
        };
        let body: Vec<u8> = match conversion {
            b'd' | b'i' => {
                let v = value.as_i64().context("bad argument to 'format' (number has no integer representation)")?;
                let mut text = v.unsigned_abs().to_string();
                if let Some(p) = precision {
                    while text.len() < p {
                        text.insert(0, '0');
                    }
                }
                let sign = if v < 0 { "-" } else if plus { "+" } else if space { " " } else { "" };
                pad_number(sign, &text, width, left, zero && precision.is_none())
            }
            b'u' => pad_number("", &(value.as_i64().context("format %u")? as u64).to_string(), width, left, zero),
            b'x' | b'X' | b'o' => {
                let v = value.as_i64().context("bad argument to 'format' (number has no integer representation)")?;
                let mut text = match conversion {
                    b'x' => format!("{:x}", v),
                    b'X' => format!("{:X}", v),
                    _ => format!("{:o}", v),
                };
                if let Some(p) = precision {
                    while text.len() < p {
                        text.insert(0, '0');
                    }
                }
                pad_number("", &text, width, left, zero)
            }
            b'c' => vec![u8::try_from(value.as_i64().context("format %c")?).unwrap_or(b'?')],
            b'f' | b'F' | b'e' | b'E' | b'g' | b'G' => {
                let v = value.as_f64().context("bad argument to 'format' (number expected)")?;
                let p = precision.unwrap_or(6);
                let text = match conversion {
                    b'f' | b'F' => format!("{:.*}", p, v.abs()),
                    b'e' | b'E' => {
                        let t = format_e(v.abs(), p);
                        if conversion == b'E' { t.to_uppercase() } else { t }
                    }
                    _ => {
                        let t = format_g(v.abs(), if p == 0 { 1 } else { p }, flags.contains('#'));
                        if conversion == b'G' { t.to_uppercase() } else { t }
                    }
                };
                let sign = if v.is_sign_negative() && !(v == 0. && !v.is_sign_negative()) && (v < 0. || v.is_sign_negative()) { "-" } else if plus { "+" } else if space { " " } else { "" };
                pad_number(sign, &text, width, left, zero)
            }
            b's' => {
                let mut text = tostring(lua, &value);
                if let Some(p) = precision {
                    text.truncate(p);
                }
                pad_bytes(text, width, left)
            }
            b'q' => {
                let s = value.display();
                let mut text = vec![b'"'];
                for b in s {
                    match b {
                        b'"' | b'\\' => text.extend([b'\\', b]),
                        b'\n' => text.extend(b"\\n"),
                        b'\r' => text.extend(b"\\r"),
                        0 => text.extend(b"\\0"),
                        _ => text.push(b),
                    }
                }
                text.push(b'"');
                text
            }
            b'a' | b'A' => bail!("'format' %a is not supported"),
            other => bail!("invalid option '%{}' to 'format'", other as char),
        };
        out.extend(body);
    }
    Ok(out)
}

fn pad_number(sign: &str, digits: &str, width: usize, left: bool, zero: bool) -> Vec<u8> {
    let len = sign.len() + digits.len();
    let mut out = String::new();
    if len >= width {
        out.push_str(sign);
        out.push_str(digits);
    } else if left {
        out.push_str(sign);
        out.push_str(digits);
        out.push_str(&" ".repeat(width - len));
    } else if zero {
        out.push_str(sign);
        out.push_str(&"0".repeat(width - len));
        out.push_str(digits);
    } else {
        out.push_str(&" ".repeat(width - len));
        out.push_str(sign);
        out.push_str(digits);
    }
    out.into_bytes()
}
fn pad_bytes(text: Vec<u8>, width: usize, left: bool) -> Vec<u8> {
    if text.len() >= width {
        return text;
    }
    let pad = vec![b' '; width - text.len()];
    if left { [text, pad].concat() } else { [pad, text].concat() }
}

fn format_e(v: f64, precision: usize) -> String {
    let text = format!("{:.*e}", precision, v);
    //Rust prints `1.5e2`; C prints `1.5e+02`.
    match text.split_once('e') {
        Some((mantissa, exponent)) => {
            let (sign, digits) = match exponent.strip_prefix('-') {
                Some(d) => ('-', d),
                None => ('+', exponent),
            };
            format!("{mantissa}e{sign}{digits:0>2}")
        }
        None => text,
    }
}

/// C `%.{precision}g`.
pub fn format_g(v: f64, precision: usize, alternate: bool) -> String {
    if v == 0. {
        return if v.is_sign_negative() { "-0".into() } else { "0".into() };
    }
    if !v.is_finite() {
        return format_float(v);
    }
    let precision = precision.max(1);
    let exponent = v.abs().log10().floor() as i32;
    //Rounding may bump the exponent (9.99 -> 10).
    let probe = format_e(v, precision - 1);
    let exponent = probe.split_once('e').and_then(|(_, e)| e.parse::<i32>().ok()).unwrap_or(exponent);
    let mut text = if exponent < -4 || exponent >= precision as i32 {
        probe
    } else {
        format!("{:.*}", (precision as i32 - 1 - exponent).max(0) as usize, v)
    };
    if !alternate {
        if let Some((mantissa, exp)) = text.clone().split_once('e') {
            let mantissa = if mantissa.contains('.') { mantissa.trim_end_matches('0').trim_end_matches('.') } else { mantissa };
            text = format!("{mantissa}e{exp}");
        } else if text.contains('.') {
            text = text.trim_end_matches('0').trim_end_matches('.').to_owned();
        }
    }
    text
}

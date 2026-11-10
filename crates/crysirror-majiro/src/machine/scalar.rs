// Copyright (c) 2026 CrySirror contributors. MIT.
//! Original scalar VM arithmetic, not host text/layout. Native x87 helpers
//! 40D910/40D990/40DA50/40DB60 widen inputs then store single precision.
use super::Value;
use anyhow::{bail, ensure, Result};
pub(super) fn branch(value:&Value,nonzero:bool)->bool {
    //40CF50/40CFC0 use the stored string extent, not strlen or a pointer.
    //The two comparisons intentionally differ for a zero-length allocation:
    //native true tests length>1, false tests length==1. Preserve that case.
    match value {
        Value::Integer(value)=>if nonzero{*value!=0}else{*value==0},
        Value::Float(value)=>if nonzero{*value!=0.0}else{*value==0.0},
        Value::String(bytes)=>if nonzero{bytes.len()>1}else{bytes.len()==1},
        Value::Array(_)=>nonzero,
    }
}
pub(super) fn number(value: &Value) -> Result<f64> {
    match value {
        Value::Integer(n) => Ok(f64::from(*n)),
        Value::Float(n) => Ok(f64::from(*n)),
        _ => bail!("Majiro numeric operand required"),
    }
}
pub(super) fn binary(op: u16, left: &Value, right: &Value) -> Result<Value> {
    if matches!(op,0x13a|0x142|0x14a|0x152|0x15a|0x162) {
        use std::cmp::Ordering::*;
        let ordering=crate::byte_policy::compare(left.bytes()?,right.bytes()?)?;
        return Ok(Value::Integer(i32::from(match op {
            0x13a=>ordering!=Greater,0x142=>ordering==Less,0x14a=>ordering!=Less,
            0x152=>ordering==Greater,0x15a=>ordering==Equal,_=>ordering!=Equal,
        })));
    }
    if op & 7 == 0 {
        return super::integer_binary(op, left, right);
    }
    if op == 0x11a {
        let a = left.stored()?;
        let b = right.stored()?;
        ensure!(!a.is_empty() && !b.is_empty(), "Majiro stored strings require their terminal storage byte");
        let count = a
            .len()
            .checked_add(b.len()-1)
            .ok_or_else(|| anyhow::anyhow!("Majiro string extent overflow"))?;
        ensure!(
            count <= 1024 * 1024,
            "Majiro concatenated string byte budget"
        );
        let mut out = Vec::with_capacity(count);
        //40DA90 copies the second stored allocation at left.length-1. It
        //overwrites that byte, including a normal C terminator, but does not
        //strlen/trim either allocation or discard their interior NUL bytes.
        out.extend_from_slice(&a[..a.len()-1]);
        out.extend_from_slice(b);
        return Ok(Value::String(out));
    }
    ensure!(op & 7 == 1, "Majiro scalar operator type unsupported");
    let a = number(left)?;
    let b = number(right)?;
    Ok(match op {
        0x101 => Value::Float((a * b) as f32),
        // Actual40D990 replaces a zero divisor with double0.00001. IEEE
        // infinity or a guessed zero would change the original script result.
        0x109 => Value::Float((a / if b == 0.0 { 0.00001 } else { b }) as f32),
        0x119 => Value::Float((a + b) as f32),
        0x121 => Value::Float((a - b) as f32),
        0x139 => Value::Integer(i32::from(a <= b)),
        0x141 => Value::Integer(i32::from(a < b)),
        0x149 => Value::Integer(i32::from(a >= b)),
        0x151 => Value::Integer(i32::from(a > b)),
        0x159 => Value::Integer(i32::from(a == b)),
        0x161 => Value::Integer(i32::from(a != b)),
        _ => bail!("Majiro unknown floating operator {op:#x}"),
    })
}
pub(super) fn convert_integer(value: &Value) -> Result<Value> {
    let Value::Float(value) = value else {
        bail!("Majiro conv.i requires float");
    };
    let wide = f64::from(*value);
    // Original40FE60 uses the x87 truncating helper and then the low32 bits.
    // Model defined int64 inputs explicitly; an indefinite x87 result is not
    // Rust's saturating float cast and remains an unsupported numeric profile.
    ensure!(
        wide.is_finite() && wide >= -9223372036854775808.0 && wide < 9223372036854775808.0,
        "Majiro conv.i outside defined native int64 domain"
    );
    Ok(Value::Integer((wide.trunc() as i64) as i32))
}

/// Scarlett409E80/409F00/409F70/409FE0/40A050; classic40D670..40D890.
/// Case values are peeked, never popped. The operand tag selects how to read
/// the selector's32-bit native payload; mixed numeric cases reinterpret bits.
pub(super) fn case_branch(op: u16, selector: &Value, operand: &Value) -> Result<bool> {
    let bits = |value: &Value| -> Result<u32> {
        match value {
            Value::Integer(value) => Ok(*value as u32),
            Value::Float(value) => Ok(value.to_bits()),
            _ => bail!("Majiro numeric case cannot reinterpret an object address"),
        }
    };
    match operand {
        Value::Integer(b) => {
            let a = bits(selector)? as i32;
            Ok(match op {
                0x831 => a != *b, 0x832 => a >= *b, 0x833 => a <= *b,
                0x838 => a < *b, 0x839 => a > *b,
                _ => bail!("Majiro invalid native case opcode {op:#x}"),
            })
        }
        Value::Float(b) => {
            let a = f32::from_bits(bits(selector)?);
            // x87 FCOMP C0/C3 tests: unordered takes832/839, not833/838.
            let unordered = a.is_nan() || b.is_nan();
            Ok(match op {
                0x831 => a != *b, 0x832 => a >= *b || unordered,
                0x833 => a <= *b, 0x838 => a < *b,
                0x839 => a > *b || unordered,
                _ => bail!("Majiro invalid native case opcode {op:#x}"),
            })
        }
        Value::String(b) if op == 0x831 => {
            let Value::String(a) = selector else {
                bail!("Majiro string case selector is not a native string object");
            };
            Ok(crate::byte_policy::compare(b, a)? != std::cmp::Ordering::Equal)
        }
        Value::Array(b) if op == 0x831 => {
            let Value::Array(a) = selector else {
                bail!("Majiro array case cannot reinterpret a non-array address");
            };
            // Original subtracts opaque object pointers. Only identity is
            // relevant to !=, so use native VM array identity without leaking
            // or manufacturing a32-bit host address.
            Ok(!std::sync::Arc::ptr_eq(&a.0, &b.0))
        }
        Value::String(_) | Value::Array(_) => {
            // Ordered source helpers use an uninitialised BOOL for object
            // tags: Scarlett reads stack scratch, later builds incoming ECX.
            // No source-level object ordering is defined. Report that domain
            // rather than inheriting scratch memory/register contents.
            bail!("Majiro ordered object case {op:#x} has undefined native scratch-BOOL semantics")
        }
    }
}

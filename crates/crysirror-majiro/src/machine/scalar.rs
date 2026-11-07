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
        let a = left.bytes()?;
        let b = right.bytes()?;
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

// Copyright (C) 2019 Nunuhara Cabbage; Rust (C) 2026 CrySirror contributors.
// GPL-2.0-or-later. xsystem4 a85467d src/hll/Math.c scalar ABI policy.
//! Scalar HLL intrinsics use Rust's CPU math, as the VM opcodes do. Random
//! streams, host-time seeding and geometry algorithms require shared facilities
//! and are deliberately absent here; an unknown call returns None, not success.
use crate::vm::{
    external::{Argument, HllReturn},
    Machine,
};
use anyhow::{bail, ensure, Result};
pub(crate) fn scalar(
    m: &mut Machine,
    name: &[u8],
    kind: i32,
    args: &[Argument],
) -> Result<Option<HllReturn>> {
    let integers = || {
        args.iter()
            .map(|a| match a {
                Argument::Scalar { kind: 10, bits } => Some(*bits),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()
    };
    let floats = || {
        args.iter()
            .map(|a| match a {
                Argument::Scalar { kind: 11, bits } => Some(f32::from_bits(*bits as u32)),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()
    };
    if kind == 10 {
        if let Some(values) = integers() {
            let value = match (name, values.as_slice()) {
                (b"Abs", [x]) => x
                    .checked_abs()
                    .ok_or_else(|| anyhow::anyhow!("System4 Math.Abs undefined native INT_MIN"))?,
                (b"Min", [a, b]) => {
                    if a < b {
                        *a
                    } else {
                        *b
                    }
                }
                (b"Max", [a, b]) => {
                    if a > b {
                        *a
                    } else {
                        *b
                    }
                }
                _ => return Ok(None),
            };
            return Ok(Some(HllReturn::Scalar(value)));
        }
        if let (b"Ceil", [x]) = (name, floats().unwrap_or_default().as_slice()) {
            let value = x.ceil();
            ensure!(
                value.is_finite()
                    && (value as f64) >= (i32::MIN as f64)
                    && (value as f64) <= (i32::MAX as f64),
                "System4 Math.Ceil undefined native integer conversion"
            );
            return Ok(Some(HllReturn::Scalar(value as i32)));
        }
    }
    if kind == 11 {
        if let Some(values) = floats() {
            let radians = |x: f32| ((x as f64) * (std::f64::consts::PI / 180.0)) as f32;
            let value = match (name, values.as_slice()) {
                (b"Cos", [x]) => radians(*x).cos(),
                (b"Sin", [x]) => radians(*x).sin(),
                (b"Tan", [x]) => radians(*x).tan(),
                (b"Sqrt", [x]) => x.sqrt(),
                (b"Asin", [x]) => x.asin(),
                (b"Acos", [x]) => x.acos(),
                (b"Atan", [x]) => x.atan(),
                (b"Atan2", [a, b]) => a.atan2(*b),
                (b"AbsF", [x]) => x.abs(),
                (b"Pow", [a, b]) => a.powf(*b),
                (b"Log", [x]) => x.ln(),
                (b"Log10", [x]) => x.log10(),
                (b"MinF", [a, b]) => {
                    if a < b {
                        *a
                    } else {
                        *b
                    }
                }
                (b"MaxF", [a, b]) => {
                    if a > b {
                        *a
                    } else {
                        *b
                    }
                }
                _ => return Ok(None),
            };
            return Ok(Some(HllReturn::Scalar(value.to_bits() as i32)));
        }
    }
    if matches!(name, b"Swap" | b"SwapF") {
        ensure!(kind == 0, "System4 Math.Swap return ABI");
        let expected = if name == b"Swap" { 18 } else { 19 };
        let [Argument::Reference {
            kind: a,
            page: ap,
            index: ai,
        }, Argument::Reference {
            kind: b,
            page: bp,
            index: bi,
        }] = args
        else {
            bail!("System4 Math.Swap reference ABI")
        };
        ensure!(
            *a == expected && *b == expected,
            "System4 Math.Swap reference type"
        );
        let (av, bv) = (m.heap.get(*ap, *ai)?, m.heap.get(*bp, *bi)?);
        m.heap.set(*ap, *ai, bv)?;
        m.heap.set(*bp, *bi, av)?;
        return Ok(Some(HllReturn::Void));
    }
    Ok(None)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ain::{Function, Program, Variable};
    use std::sync::Arc;
    fn machine() -> Machine {
        Machine::new(Arc::new(Program {
            version: 5,
            code: 0x2fu16.to_le_bytes().to_vec(),
            functions: vec![Function {
                name: b"main".to_vec(),
                address: 0,
                is_label: false,
                return_kind: 10,
                return_structure: -1,
                arguments: 0,
                variables: Vec::new(),
                crc: 0,
            }],
            globals: vec![
                Variable {
                    name: vec![],
                    kind: 10,
                    structure: -1,
                    rank: 0,
                    group: None
                };
                2
            ],
            ..Default::default()
        }))
        .unwrap()
    }
    fn float(value: f32) -> Argument {
        Argument::Scalar {
            kind: 11,
            bits: value.to_bits() as i32,
        }
    }
    #[test]
    fn scalar_abi_uses_degree_trig_and_native_nan_ternary_order() {
        let mut m = machine();
        let Some(HllReturn::Scalar(value)) = scalar(&mut m, b"Sin", 11, &[float(90.0)]).unwrap()
        else {
            panic!("sin return")
        };
        assert!((f32::from_bits(value as u32) - 1.0).abs() < 1e-6);
        let nan = f32::from_bits(0x7fc1_2345);
        let Some(HllReturn::Scalar(value)) =
            scalar(&mut m, b"MinF", 11, &[float(1.0), float(nan)]).unwrap()
        else {
            panic!("min return")
        };
        assert_eq!(value as u32, nan.to_bits());
        assert!(matches!(
            scalar(&mut m, b"Ceil", 10, &[float(1.5)]).unwrap(),
            Some(HllReturn::Scalar(2))
        ));
        assert!(scalar(&mut m, b"Ceil", 10, &[float(f32::NAN)]).is_err());
        assert!(scalar(&mut m, b"Rand", 10, &[]).unwrap().is_none());
    }
    #[test]
    fn swap_changes_only_original_references() {
        let mut m = machine();
        m.heap.set(0, 0, 31).unwrap();
        m.heap.set(0, 1, 47).unwrap();
        let arguments = [
            Argument::Reference {
                kind: 18,
                page: 0,
                index: 0,
            },
            Argument::Reference {
                kind: 18,
                page: 0,
                index: 1,
            },
        ];
        assert!(matches!(
            scalar(&mut m, b"Swap", 0, &arguments).unwrap(),
            Some(HllReturn::Void)
        ));
        assert_eq!(
            [m.heap.get(0, 0).unwrap(), m.heap.get(0, 1).unwrap()],
            [47, 31]
        );
    }
}

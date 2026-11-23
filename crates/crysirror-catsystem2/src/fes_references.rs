// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! 758E00/758D40 named-array reference policy. Resource lookup and scoped
//! handles remain owned by the caller, not process pointers in these bytes.
use crate::{native_interpreter::Interpreter, native_values::StringVariables};
use anyhow::{ensure, Result};
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct IndexedName {
    pub name: Vec<u8>,
    pub index: i32,
}
///An absent name has native array count0. Count lookup is exact-case. The
///unindexed/all-array sentinel-1 becomes scalar0 when the actual count<=1.
///Empty brackets have uninitialised original scratch data and are deliberately
///unsupported instead of inventing an expression or index.
pub(crate) fn evaluate(
    source: &[u8],
    vm: &mut Interpreter,
    strings: &StringVariables,
    count: &impl Fn(&[u8]) -> Option<u32>,
) -> Result<IndexedName> {
    ensure!(
        !source.is_empty() && source.len() < 512 && !source.contains(&0),
        "CatSystem2 FES reference source extent/NUL"
    );
    let end = source
        .iter()
        .position(|byte| matches!(byte, b' ' | b'['))
        .unwrap_or(source.len());
    ensure!(
        end > 0 && end < 32,
        "CatSystem2 FES reference original name extent"
    );
    let expression = if source.get(end) == Some(&b'[') {
        let tail = &source[end + 1..];
        let close = tail
            .iter()
            .position(|byte| matches!(byte, b' ' | b']'))
            .unwrap_or(tail.len());
        ensure!(
            tail.get(close) == Some(&b']') && close > 0 && close < 511,
            "CatSystem2 FES reference bracket/empty-index profile"
        );
        &tail[..close]
    } else {
        b"-1"
    };
    let name = strings.expand(&source[..end], 31)?;
    ensure!(
        !name.is_empty(),
        "CatSystem2 FES reference expanded empty name profile"
    );
    let mut index = vm.evaluate(expression)? as i32;
    if index == -1 && count(&name).unwrap_or(0) <= 1 {
        index = 0;
    }
    Ok(IndexedName { name, index })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scalar_array_sentinel_case_and_native_trailing_bytes_are_preserved() {
        let mut vm = Interpreter::new(8).unwrap();
        let strings = StringVariables::default();
        let count = |name: &[u8]| match name {
            b"array" => Some(4),
            b"scalar" => Some(1),
            _ => None,
        };
        assert_eq!(
            evaluate(b"array", &mut vm, &strings, &count).unwrap().index,
            -1
        );
        assert_eq!(
            evaluate(b"array[-1]", &mut vm, &strings, &count)
                .unwrap()
                .index,
            -1
        );
        assert_eq!(
            evaluate(b"ARRAY", &mut vm, &strings, &count).unwrap().index,
            0
        );
        assert_eq!(
            evaluate(b"scalar", &mut vm, &strings, &count)
                .unwrap()
                .index,
            0
        );
        assert_eq!(
            evaluate(b"missing", &mut vm, &strings, &count)
                .unwrap()
                .index,
            0
        );
        assert_eq!(
            evaluate(b"array[2]trailing", &mut vm, &strings, &count)
                .unwrap()
                .index,
            2
        );
        assert_eq!(
            evaluate(b"array suffix", &mut vm, &strings, &count)
                .unwrap()
                .index,
            -1
        );
        assert_eq!(
            evaluate(b"array[++$0]", &mut vm, &strings, &count)
                .unwrap()
                .index,
            1
        );
        assert_eq!(vm.flag(0), 1);
        for raw in [
            b"array[]".as_slice(),
            b"array[1 2]",
            b"array[1",
            b"[1]",
            b"array\0",
        ] {
            assert!(evaluate(raw, &mut vm, &strings, &count).is_err());
        }
    }
    #[test]
    fn original_name_string_expansion_precedes_real_array_count_lookup() {
        let mut vm = Interpreter::new(8).unwrap();
        let mut strings = StringVariables::default();
        strings.assign(2, b"array".to_vec()).unwrap();
        let count = |name: &[u8]| if name == b"array" { Some(3) } else { None };
        let got = evaluate(b"$str2", &mut vm, &strings, &count).unwrap();
        assert_eq!(
            got,
            IndexedName {
                name: b"array".to_vec(),
                index: -1
            }
        );
        let got = evaluate(b"$str2[7]", &mut vm, &strings, &count).unwrap();
        assert_eq!(got.index, 7);
        assert!(evaluate(b"$str3", &mut vm, &strings, &count).is_err());
    }
}

// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! 7597B0 original KEYBLOCK table admission. This is native button declaration
//! data; actual InputRouting/key actions must still be published separately.
use crate::{fes_declarations::KeyBlock, fes_values, native_interpreter::Interpreter};
use anyhow::{ensure, Result};
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct KeyBlocks(pub Vec<[u32; 6]>);
impl KeyBlocks {
    pub fn decode(declaration: &KeyBlock, interpreter: &mut Interpreter) -> Result<Self> {
        ensure!(
            (1..=4096).contains(&declaration.count) && declaration.rows.len() <= 65536,
            "CatSystem2 FES KEYBLOCK allocation/row budget"
        );
        let mut vm = interpreter.clone();
        let mut blocks =
            vec![[0, 0, u32::MAX, u32::MAX, u32::MAX, u32::MAX]; declaration.count as usize];
        for tokens in &declaration.rows {
            ensure!(
                (1..=3).contains(&tokens.len()),
                "CatSystem2 FES KEYBLOCK row layout"
            );
            let Some(index) = fes_values::component(&tokens[0], 0, Some(&mut vm))? else {
                continue;
            };
            //Out-of-range rows do not evaluate their value expressions. The
            //index expression has already executed, exactly like7597B0.
            if (index as i32) < 0 || index >= declaration.count {
                continue;
            }
            let Some(pair) = tokens.get(1) else {
                continue;
            };
            let target = &mut blocks[index as usize];
            let Some(first) = fes_values::component(pair, 0, Some(&mut vm))? else {
                continue;
            };
            target[0] = first;
            let Some(second) = fes_values::component(pair, 1, Some(&mut vm))? else {
                continue;
            };
            target[1] = second;
            if let Some(directions) = tokens.get(2) {
                for direction in 0..4 {
                    if let Some(value) =
                        fes_values::component(directions, direction, Some(&mut vm))?
                    {
                        target[2 + direction as usize] = value;
                    }
                }
            }
        }
        *interpreter = vm;
        Ok(Self(blocks))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn row(tokens: &[&[u8]]) -> Vec<Vec<u8>> {
        tokens.iter().map(|token| token.to_vec()).collect()
    }
    #[test]
    fn native_defaults_partial_rows_duplicates_and_ignored_tails_preserve_order() {
        let mut vm = Interpreter::new(8).unwrap();
        let declaration = KeyBlock {
            count: 3,
            rows: vec![
                row(&[b"1", b"10,20", b"1,2,3,4,++$7"]),
                row(&[b"1", b"30", b"++$7,++$7"]),
                row(&[b"2", b"5,6", b"9"]),
                row(&[b"-1", b"++$7,++$7"]),
                row(&[b"99", b"++$7,++$7"]),
            ],
        };
        let got = KeyBlocks::decode(&declaration, &mut vm).unwrap();
        assert_eq!(
            got.0,
            vec![
                [0, 0, u32::MAX, u32::MAX, u32::MAX, u32::MAX],
                [30, 20, 1, 2, 3, 4],
                [5, 6, 9, u32::MAX, u32::MAX, u32::MAX]
            ]
        );
        assert_eq!(vm.flag(7), 0);
    }
    #[test]
    fn original_component_expressions_run_once_and_failure_does_not_commit_bank() {
        let mut vm = Interpreter::new(8).unwrap();
        let declaration = KeyBlock {
            count: 1,
            rows: vec![
                row(&[b"$0+=1", b"++$1,++$1"]),
                row(&[b"0", b"++$1,++$1", b"++$2,,++$2"]),
            ],
        };
        let got = KeyBlocks::decode(&declaration, &mut vm).unwrap();
        assert_eq!(vm.flag(0), 1);
        assert_eq!(vm.flag(1), 2);
        assert_eq!(vm.flag(2), 2);
        assert_eq!(got.0[0], [1, 2, 1, 0, 2, u32::MAX]);
        let invalid = KeyBlock {
            count: 1,
            rows: vec![
                row(&[b"0", b"++$1,++$1"]),
                vec![b"0".to_vec(), vec![b'1'; 512]],
            ],
        };
        assert!(KeyBlocks::decode(&invalid, &mut vm).is_err());
        assert_eq!(vm.flag(1), 2);
    }
}

// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original OBJECT.F list/overload policy. Packed arithmetic is a shared Core
//! operation, also used by the native global lists. No resource/render state.
use super::{codes::intlist_op as op, int_list};
use crate::runtime::{Value, constants::RuntimeConstants};
use anyhow::{Context, Result, bail, ensure};
use std::sync::Arc;
const MAX_WORDS: usize = 4096;
fn store(list: &mut Arc<Vec<i64>>, form: u32, width: u32, index: i64, value: i64) {
    if crysirror_core::integer_fields::get(list, width, index).is_some() {
        int_list::bit_set(
            form,
            Arc::make_mut(list).as_mut_slice(),
            width,
            index,
            value,
        );
    } else {
        int_list::log_out_of_range(form, width, index, list.len());
    }
}
pub(super) fn dispatch(
    list: &mut Arc<Vec<i64>>,
    ids: &RuntimeConstants,
    tail: &[i32],
    args: &[Value],
    overload: i64,
) -> Result<Option<Value>> {
    let (width, tail) = match tail.first() {
        Some(&op::BIT) => (1, &tail[1..]),
        Some(&op::BIT2) => (2, &tail[1..]),
        Some(&op::BIT4) => (4, &tail[1..]),
        Some(&op::BIT8) => (8, &tail[1..]),
        Some(&op::BIT16) => (16, &tail[1..]),
        _ => (32, tail),
    };
    let integer = |i: usize| {
        args.get(i)
            .and_then(Value::as_i64)
            .context("native OBJECT.F integer")
    };
    if tail.len() == 1
        && matches!(
            tail[0],
            op::INIT | op::RESIZE | op::GET_SIZE | op::CLEAR | op::SETS
        )
    {
        match tail[0] {
            op::INIT => {
                ensure!(
                    overload == 0 && args.is_empty(),
                    "native OBJECT.F INIT arguments"
                );
                Arc::make_mut(list).clear();
            }
            op::RESIZE => {
                ensure!(
                    overload == 0 && args.len() == 1,
                    "native OBJECT.F RESIZE arguments"
                );
                let size = usize::try_from(integer(0)?.max(0))?;
                ensure!(size <= MAX_WORDS, "native OBJECT.F word budget");
                Arc::make_mut(list).resize(size, 0);
            }
            op::GET_SIZE => {
                ensure!(
                    overload == 0 && args.is_empty(),
                    "native OBJECT.F GET_SIZE arguments"
                );
                return Ok(Some(Value::Int(int_list::logical_size(list.len(), width))));
            }
            op::CLEAR => {
                ensure!(
                    matches!(overload, 0 | 1) && args.len() == 2 + usize::from(overload == 1),
                    "native OBJECT.F CLEAR overload/arguments"
                );
                let (first, last) = (integer(0)?, integer(1)?);
                let value = if overload == 0 { 0 } else { integer(2)? };
                // Native out-of-range stores are ignored; visit only the
                // bounded intersection, without an unbounded invalid loop.
                ensure!(
                    crysirror_core::integer_fields::fill_clipped(
                        Arc::make_mut(list).as_mut_slice(),
                        width,
                        first,
                        last,
                        value
                    )
                    .is_some(),
                    "native OBJECT.F packed range layout"
                );
            }
            op::SETS => {
                ensure!(
                    overload == 0 && !args.is_empty() && args.len() <= MAX_WORDS + 1,
                    "native OBJECT.F SETS arguments/budget"
                );
                let start = integer(0)?;
                let values = (1..args.len()).map(integer).collect::<Result<Vec<_>>>()?;
                for (offset, value) in values.into_iter().enumerate() {
                    let index = start
                        .checked_add(offset as i64)
                        .context("native OBJECT.F index overflow")?;
                    store(list, ids.obj_f as u32, width, index, value);
                }
            }
            _ => unreachable!(),
        }
        return Ok(None);
    }
    let index = match tail {
        [] => 0,
        [marker, index] if *marker == ids.elm_array || *marker == -1 => i64::from(*index),
        [index] if *index >= 0 => i64::from(*index),
        _ => bail!("unsupported native OBJECT.F chain {tail:?}"),
    };
    ensure!(
        matches!(overload, 0 | 1) && args.len() == usize::from(overload == 1),
        "native OBJECT.F index overload/arguments"
    );
    let value = if overload == 1 {
        Some(integer(0)?)
    } else {
        None
    };
    // The original bare-list shortcut is its first raw word, even after a
    // bit-width selector. Explicit indexed accesses use the selected width.
    let width = if tail.is_empty() { 32 } else { width };
    if tail.is_empty() && list.is_empty() {
        Arc::make_mut(list).push(0);
    }
    if let Some(value) = value {
        store(list, ids.obj_f as u32, width, index, value);
        Ok(None)
    } else {
        Ok(Some(Value::Int(int_list::bit_get(
            ids.obj_f as u32,
            list,
            width,
            index,
        ))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn value(list: &mut Arc<Vec<i64>>, tail: &[i32], args: &[i64], overload: i64) -> Option<i64> {
        dispatch(
            list,
            &RuntimeConstants::default(),
            tail,
            &args.iter().copied().map(Value::Int).collect::<Vec<_>>(),
            overload,
        )
        .unwrap()
        .and_then(|v| v.as_i64())
    }
    #[test]
    fn checkpoints_share_reads_detach_writes_and_preserve_original_vec_wire() {
        use crate::runtime::globals::ObjectPropLists;
        let mut lists = ObjectPropLists::default();
        lists.f = Arc::new(vec![17, -1, i64::from(i32::MIN)]);
        let saved = lists.clone();
        assert_eq!(value(&mut lists.f, &[-1, 0], &[], 0), Some(17));
        assert!(Arc::ptr_eq(&saved.f, &lists.f));
        value(&mut lists.f, &[-1, 0], &[29], 1);
        assert!(!Arc::ptr_eq(&saved.f, &lists.f));
        assert_eq!(saved.f[0], 17);
        let old = (
            Vec::<i64>::new(),
            Vec::<i64>::new(),
            Vec::<i64>::new(),
            Vec::<i64>::new(),
            saved.f.as_ref().clone(),
        );
        let wire = bincode::serialize(&old).unwrap();
        assert_eq!(wire, bincode::serialize(&saved).unwrap());
        let decoded: ObjectPropLists = bincode::deserialize(&wire).unwrap();
        assert_eq!(decoded.f, saved.f);
    }
    #[test]
    fn packed_fields_keep_native_word_sizes_sign_and_neighbor_values() {
        let mut list = Arc::new(vec![]);
        value(&mut list, &[op::RESIZE], &[2], 0);
        value(&mut list, &[-1, 0], &[-1], 1);
        assert_eq!(value(&mut list, &[op::BIT4, -1, 7], &[], 0), Some(15));
        value(&mut list, &[op::BIT8, -1, 3], &[0x12], 1);
        assert_eq!(value(&mut list, &[-1, 0], &[], 0), Some(0x12ff_ffff));
        assert_eq!(list[1], 0);
        value(&mut list, &[op::BIT4, op::CLEAR], &[7, 9, 0x123], 1);
        assert_eq!(value(&mut list, &[op::BIT4, -1, 6], &[], 0), Some(2));
        for index in 7..=9 {
            assert_eq!(value(&mut list, &[op::BIT4, -1, index], &[], 0), Some(3));
        }
        assert_eq!(list[1], 0x33);
        value(&mut list, &[op::BIT8, op::RESIZE], &[1], 0);
        assert_eq!(value(&mut list, &[op::BIT8, op::GET_SIZE], &[], 0), Some(4));
        // Bare F's scalar shortcut still returns its first raw signed word.
        assert_eq!(value(&mut list, &[op::BIT], &[], 0), Some(list[0]));
        value(&mut list, &[op::INIT], &[], 0);
        assert!(list.is_empty());
    }
    #[test]
    fn invalid_indices_preserve_native_zero_ignore_policy_and_ranges_stay_bounded() {
        let mut list = Arc::new(vec![8, 9]);
        value(&mut list, &[op::SETS], &[-1, 1, 2, 3], 0);
        assert_eq!(list.as_slice(), [2, 3]);
        assert_eq!(value(&mut list, &[-1, -1], &[], 0), Some(0));
        value(&mut list, &[-1, 99], &[7], 1);
        assert_eq!(list.as_slice(), [2, 3]);
        value(&mut list, &[op::CLEAR], &[i64::MIN, i64::MAX], 0);
        assert_eq!(list.as_slice(), [0, 0]);
        let before = list.clone();
        assert!(
            dispatch(
                &mut list,
                &RuntimeConstants::default(),
                &[op::RESIZE],
                &[Value::Int(4097)],
                0
            )
            .is_err()
        );
        assert_eq!(list, before);
        Arc::make_mut(&mut list).clear();
        assert!(
            dispatch(
                &mut list,
                &RuntimeConstants::default(),
                &[],
                &[Value::Str("bad".into())],
                1
            )
            .is_err()
        );
        assert!(list.is_empty());
    }
}

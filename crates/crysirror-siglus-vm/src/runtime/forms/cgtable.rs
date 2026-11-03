// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original CG flag/name/count semantics; decoding is a shared resource format.
//! Reference: siglus_rs 5f358ef, runtime/forms/cgtable.rs (MPL-2.0).
use super::codes::cgtable_op as op;
use crate::runtime::{CommandContext, Value};
use anyhow::{Context, Result, ensure};
fn flag(ctx: &CommandContext, index: i32) -> i32 {
    usize::try_from(index)
        .ok()
        .and_then(|i| ctx.tables.cg_flags.get(i))
        .copied()
        .unwrap_or(0)
}
fn write(ctx: &mut CommandContext, index: i32, value: i32) -> Result<()> {
    if index < 0 {
        return Ok(());
    }
    let index = index as usize;
    ensure!(index < 1_048_576, "native CG flag index budget");
    let flags = std::sync::Arc::make_mut(&mut ctx.tables.cg_flags);
    flags.resize(flags.len().max(index + 1), 0);
    flags[index] = value;
    Ok(())
}
pub(crate) fn dispatch(ctx: &mut CommandContext, args: &[Value]) -> Result<bool> {
    let call = ctx.vm_call.clone().context("native CG call metadata")?;
    let chain = &call.element;
    ensure!(chain.len() >= 2, "native CG chain");
    let command = chain[1];
    let int = |i: usize| {
        args.get(i)
            .and_then(Value::as_i64)
            .context("native CG integer")
    };
    let name = || {
        args.first()
            .and_then(Value::as_str)
            .context("native CG name")
    };
    let mut result = None;
    if command == op::FLAG {
        ensure!(
            chain.len() == 4 && chain[2] == ctx.ids.elm_array && matches!(call.al_id, 0 | 1),
            "native CG flag chain"
        );
        ensure!(
            args.len() == usize::from(call.al_id == 1),
            "native CG flag arguments"
        );
        if call.al_id == 1 {
            write(ctx, chain[3], int(0)? as i32)?;
        } else {
            result = Some(Value::Int(i64::from(flag(ctx, chain[3]))));
        }
    } else {
        ensure!(
            chain.len() == 2 && call.al_id == 0,
            "native CG command chain/overload"
        );
        let arity = match command {
            op::SET_DISABLE
            | op::SET_ENABLE
            | op::GET_CG_CNT
            | op::GET_LOOK_CNT
            | op::GET_LOOK_PERCENT => 0,
            op::SET_ALL_FLAG
            | op::GET_FLAG_NO_BY_NAME
            | op::GET_NAME_BY_FLAG_NO
            | op::GET_LOOK_BY_NAME => 1,
            op::SET_LOOK_BY_NAME => 2,
            _ => anyhow::bail!("unsupported native CG operation {command}"),
        };
        ensure!(args.len() == arity, "native CG command arguments");
        match command {
            op::SET_DISABLE => ctx.globals.cg_table_off = true,
            op::SET_ENABLE => ctx.globals.cg_table_off = false,
            op::SET_ALL_FLAG => {
                let value = int(0)? as i32;
                let indices = ctx
                    .tables
                    .cg_catalog
                    .as_ref()
                    .map(|c| c.entries.iter().map(|e| e.flag_no).collect::<Vec<_>>())
                    .unwrap_or_default();
                for index in indices {
                    write(ctx, index, value)?;
                }
            }
            op::GET_CG_CNT | op::GET_LOOK_CNT | op::GET_LOOK_PERCENT => {
                let total = ctx
                    .tables
                    .cg_catalog
                    .as_ref()
                    .map_or(0, |c| c.entries.len()) as i64;
                let looked = ctx.tables.cg_catalog.as_ref().map_or(0, |c| {
                    c.entries
                        .iter()
                        .filter(|e| e.flag_no >= 0 && flag(ctx, e.flag_no) == 1)
                        .count()
                }) as i64;
                result = Some(Value::Int(match command {
                    op::GET_CG_CNT => total,
                    op::GET_LOOK_CNT => looked,
                    _ => {
                        if total > 0 && looked > 0 {
                            (looked * 100 / total).max(1)
                        } else {
                            0
                        }
                    }
                }));
            }
            op::GET_FLAG_NO_BY_NAME | op::GET_LOOK_BY_NAME | op::SET_LOOK_BY_NAME => {
                let index = ctx
                    .tables
                    .cg_catalog
                    .as_ref()
                    .and_then(|c| c.by_name(name().ok()?))
                    .map(|e| e.flag_no);
                // Validate the string even when the configured table is empty.
                name()?;
                if command == op::SET_LOOK_BY_NAME {
                    if let Some(index) = index {
                        write(ctx, index, int(1)? as i32)?;
                    }
                } else {
                    result = Some(Value::Int(match index {
                        None => -1,
                        Some(index) => i64::from(if command == op::GET_FLAG_NO_BY_NAME {
                            index
                        } else {
                            flag(ctx, index)
                        }),
                    }));
                }
            }
            op::GET_NAME_BY_FLAG_NO => {
                let index = int(0)? as i32;
                result = Some(Value::Str(
                    ctx.tables
                        .cg_catalog
                        .as_ref()
                        .and_then(|c| c.by_flag(index))
                        .map_or_else(String::new, |e| e.name.clone()),
                ));
            }
            _ => unreachable!(),
        }
    }
    if let Some(value) = result {
        ctx.push(value)
    } else if call.ret_form != 0 {
        ctx.push(Value::Int(0))
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::VmCallMeta;
    use crysirror_resource::cg_catalog::{CgCatalog, CgCatalogEntry};
    fn call(ctx: &mut CommandContext, command: i32, args: &[Value]) -> Option<Value> {
        ctx.vm_call = Some(VmCallMeta {
            element: vec![78, command],
            al_id: 0,
            ret_form: 10,
        });
        dispatch(ctx, args).unwrap();
        ctx.pop()
    }
    #[test]
    fn signed_flags_exact_look_count_and_names_are_native_state_not_a_stub() {
        let mut ctx = CommandContext::new("asset".into());
        ctx.tables.cg_catalog = Some(std::sync::Arc::new(CgCatalog {
            version: 1,
            auto_flag: 0,
            entries: [("A", 7), ("B", 8), ("A", 9)]
                .into_iter()
                .map(|(name, flag_no)| CgCatalogEntry {
                    name: name.into(),
                    flag_no,
                    codes: [0; 5],
                    code_count: 0,
                })
                .collect(),
        }));
        assert_eq!(
            call(&mut ctx, op::GET_FLAG_NO_BY_NAME, &[Value::Str("a".into())])
                .and_then(|v| v.as_i64()),
            Some(7)
        );
        call(
            &mut ctx,
            op::SET_LOOK_BY_NAME,
            &[Value::Str("A".into()), Value::Int(2)],
        );
        assert_eq!(
            call(&mut ctx, op::GET_LOOK_CNT, &[]).and_then(|v| v.as_i64()),
            Some(0)
        );
        call(
            &mut ctx,
            op::SET_LOOK_BY_NAME,
            &[Value::Str("A".into()), Value::Int(1)],
        );
        assert_eq!(
            call(&mut ctx, op::GET_LOOK_PERCENT, &[]).and_then(|v| v.as_i64()),
            Some(33)
        );
        let checkpoint = ctx.clone();
        call(&mut ctx, op::SET_ALL_FLAG, &[Value::Int(-7)]);
        assert_eq!(flag(&ctx, 7), -7);
        assert_eq!(flag(&checkpoint, 7), 1);
        assert_eq!(
            call(&mut ctx, op::GET_LOOK_CNT, &[]).and_then(|v| v.as_i64()),
            Some(0)
        );
        call(&mut ctx, op::SET_DISABLE, &[]);
        assert!(ctx.globals.cg_table_off);
        call(
            &mut ctx,
            op::SET_LOOK_BY_NAME,
            &[Value::Str("A".into()), Value::Int(1)],
        );
        assert_eq!(flag(&ctx, 7), 1);
        call(&mut ctx, op::SET_ENABLE, &[]);
        assert!(!ctx.globals.cg_table_off);
        call(&mut ctx, op::SET_ALL_FLAG, &[Value::Int(1)]);
        assert_eq!(
            call(&mut ctx, op::GET_LOOK_CNT, &[]).and_then(|v| v.as_i64()),
            Some(3)
        );
        assert_eq!(
            call(&mut ctx, op::GET_LOOK_PERCENT, &[]).and_then(|v| v.as_i64()),
            Some(100)
        );
        assert_eq!(
            call(&mut ctx, op::GET_CG_CNT, &[]).and_then(|v| v.as_i64()),
            Some(3)
        );
        assert_eq!(
            call(&mut ctx, op::GET_NAME_BY_FLAG_NO, &[Value::Int(8)])
                .and_then(|v| v.as_str().map(str::to_owned)),
            Some("B".into())
        );
    }
    #[test]
    fn direct_flag_array_preserves_values_and_bounds_before_allocating() {
        let mut ctx = CommandContext::new("asset".into());
        let array = ctx.ids.elm_array;
        for value in [-7, 0, 1, 2, i32::MIN, i32::MAX] {
            ctx.vm_call = Some(VmCallMeta {
                element: vec![78, 0, array, 2],
                al_id: 1,
                ret_form: 0,
            });
            dispatch(&mut ctx, &[Value::Int(i64::from(value))]).unwrap();
            ctx.vm_call.as_mut().unwrap().al_id = 0;
            dispatch(&mut ctx, &[]).unwrap();
            assert_eq!(ctx.pop().and_then(|v| v.as_i64()), Some(i64::from(value)));
        }
        let before = ctx.tables.cg_flags.len();
        ctx.vm_call = Some(VmCallMeta {
            element: vec![78, 0, array, i32::MAX],
            al_id: 1,
            ret_form: 0,
        });
        assert!(dispatch(&mut ctx, &[Value::Int(1)]).is_err());
        assert_eq!(ctx.tables.cg_flags.len(), before);
    }
}

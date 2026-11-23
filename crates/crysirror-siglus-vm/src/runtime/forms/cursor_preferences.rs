// SPDX-License-Identifier: MPL-2.0
// Whole mouse-cursor preference family from siglus_rs
// 5f358ef1360b0439e92aed2b749e8f25d8721954 runtime/forms/syscom.rs.
use crate::runtime::{CommandContext, Value, forms::codes::syscom_op as op};
use anyhow::{Context, Result, ensure};
pub(super) fn defaults(ctx: &CommandContext) -> (i64, i64) {
    let get = |key: &str, fallback: i64| {
        ctx.tables
            .gameexe
            .as_ref()
            .and_then(|cfg| cfg.get_value(key))
            .and_then(|raw| {
                raw.split(|c: char| c == ',' || c.is_whitespace())
                    .find_map(|s| s.trim().parse::<i64>().ok())
            })
            .unwrap_or(fallback)
    };
    (
        get("CONFIG.MOUSE_CURSOR_HIDE_ONOFF", 0).clamp(0, 1),
        get("CONFIG.MOUSE_CURSOR_HIDE_TIME", 5000).max(0),
    )
}
pub(super) fn initialize(ctx: &mut CommandContext) {
    let (enabled, delay) = defaults(ctx);
    ctx.globals
        .syscom
        .config_int
        .insert(op::GET_MOUSE_CURSOR_HIDE_ONOFF, enabled);
    ctx.globals
        .syscom
        .config_int
        .insert(op::GET_MOUSE_CURSOR_HIDE_TIME, delay);
    ctx.globals.syscom.original_config.mouse_cursor_hide_onoff = enabled != 0;
    ctx.globals.syscom.original_config.mouse_cursor_hide_time = delay;
}
pub(super) fn dispatch(ctx: &mut CommandContext, args: &[Value]) -> Result<bool> {
    let call = ctx
        .vm_call
        .as_ref()
        .context("native cursor preference metadata")?;
    let command = *call
        .element
        .get(1)
        .context("native cursor preference opcode")?;
    let (key, action) = match command {
        op::SET_MOUSE_CURSOR_HIDE_ONOFF => (op::GET_MOUSE_CURSOR_HIDE_ONOFF, 0),
        op::SET_MOUSE_CURSOR_HIDE_ONOFF_DEFAULT => (op::GET_MOUSE_CURSOR_HIDE_ONOFF, 1),
        op::GET_MOUSE_CURSOR_HIDE_ONOFF => (op::GET_MOUSE_CURSOR_HIDE_ONOFF, 2),
        op::SET_MOUSE_CURSOR_HIDE_TIME => (op::GET_MOUSE_CURSOR_HIDE_TIME, 0),
        op::SET_MOUSE_CURSOR_HIDE_TIME_DEFAULT => (op::GET_MOUSE_CURSOR_HIDE_TIME, 1),
        op::GET_MOUSE_CURSOR_HIDE_TIME => (op::GET_MOUSE_CURSOR_HIDE_TIME, 2),
        _ => return Ok(false),
    };
    ensure!(
        call.element.len() == 2 && call.al_id == 0 && args.len() == usize::from(action == 0),
        "native cursor preference ABI"
    );
    let (on, delay) = defaults(ctx);
    let fallback = if key == op::GET_MOUSE_CURSOR_HIDE_ONOFF {
        on
    } else {
        delay
    };
    if action == 2 {
        ctx.push(Value::Int(
            ctx.globals
                .syscom
                .config_int
                .get(&key)
                .copied()
                .unwrap_or(fallback),
        ));
    } else {
        let value = if action == 1 {
            fallback
        } else {
            let value = args[0]
                .as_i64()
                .context("native cursor preference integer")?;
            ensure!(
                i32::try_from(value).is_ok(),
                "native cursor preference integer width"
            );
            if key == op::GET_MOUSE_CURSOR_HIDE_ONOFF {
                i64::from(value != 0)
            } else {
                value
            }
        };
        ctx.globals.syscom.config_int.insert(key, value);
        let config = &mut ctx.globals.syscom.original_config;
        if key == op::GET_MOUSE_CURSOR_HIDE_ONOFF {
            config.mouse_cursor_hide_onoff = value != 0;
        } else {
            config.mouse_cursor_hide_time = value;
        }
    }
    Ok(true)
}

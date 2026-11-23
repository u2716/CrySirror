// SPDX-License-Identifier: MPL-2.0
use crate::runtime::CommandContext;
fn extra_default(ctx: &CommandContext, index: usize, mode: bool) -> i64 {
    ctx.tables
        .gameexe
        .as_ref()
        .and_then(|cfg| {
            cfg.get_indexed_field_unquoted(
                if mode {
                    "CONFIG.GLOBAL_EXTRA_MODE"
                } else {
                    "CONFIG.GLOBAL_EXTRA_SWITCH"
                },
                index,
                if mode { "MODE" } else { "ONOFF" },
            )
        })
        .and_then(|s| s.parse().ok())
        .unwrap_or(i64::from(!mode))
}
pub(crate) fn initialize_config(ctx: &mut CommandContext) {
    super::syscom_preferences::initialize(ctx);
    super::sound_config::initialize(ctx);
    super::cursor_preferences::initialize(ctx);
    super::font_preferences::initialize(ctx);
    let mode = ctx
        .tables
        .gameexe
        .as_ref()
        .and_then(|cfg| cfg.get_i64("CONFIG.WINDOW_MODE"))
        .unwrap_or(0)
        .clamp(0, 1);
    ctx.globals
        .syscom
        .config_int
        .insert(super::codes::syscom_op::GET_WINDOW_MODE, mode);
    ctx.globals.syscom.original_config.screen_size_mode = mode;
    ctx.globals.syscom.original_config.global_extra_switch_flag =
        (0..4).map(|i| extra_default(ctx, i, false) != 0).collect();
    ctx.globals.syscom.original_config.global_extra_mode_flag =
        (0..4).map(|i| extra_default(ctx, i, true)).collect();
}
/// Game-defined extra switches/modes are native saved script policy, without
/// a second engine service or hard-coded title behavior.
pub(crate) fn dispatch_config(
    ctx: &mut CommandContext,
    args: &[crate::runtime::Value],
) -> anyhow::Result<bool> {
    if super::font_preferences::dispatch(ctx,args)? {return Ok(true);}
    if super::cursor_preferences::dispatch(ctx,args)? { return Ok(true); }
    if super::sound_config::dispatch(ctx, args)? { return Ok(true); }
    if super::syscom_preferences::dispatch(ctx, args)? {
        return Ok(true);
    }
    if dispatch_controls(ctx, args)? {
        return Ok(true);
    }
    if super::syscom_flags::dispatch(ctx, args)? {
        return Ok(true);
    }
    use crate::runtime::{Value, forms::codes::syscom_op as op};
    use anyhow::{Context, ensure};
    let call = ctx.vm_call.as_ref().context("native SYSCOM metadata")?;
    ensure!(
        ctx.globals
            .syscom
            .original_config
            .global_extra_switch_flag
            .len()
            == 4
            && ctx
                .globals
                .syscom
                .original_config
                .global_extra_mode_flag
                .len()
                == 4,
        "native extra configuration array shape"
    );
    ensure!(
        call.element.len() == 2,
        "unsupported nested SYSCOM configuration"
    );
    let (mode, action) = match call.element[1] {
        op::SET_GLOBAL_EXTRA_SWITCH_ONOFF => (false, 0),
        op::SET_GLOBAL_EXTRA_SWITCH_ONOFF_DEFAULT => (false, 1),
        op::GET_GLOBAL_EXTRA_SWITCH_ONOFF => (false, 2),
        op::SET_GLOBAL_EXTRA_MODE_VALUE => (true, 0),
        op::SET_GLOBAL_EXTRA_MODE_VALUE_DEFAULT => (true, 1),
        op::GET_GLOBAL_EXTRA_MODE_VALUE => (true, 2),
        _ => return Ok(false),
    };
    ensure!(
        args.len() == if action == 0 { 2 } else { 1 },
        "native extra switch/mode arguments"
    );
    let index = args[0].as_i64().context("native extra switch/mode index")?;
    let valid = (0..4).contains(&index);
    if action == 2 {
        let cfg = &ctx.globals.syscom.original_config;
        ctx.push(Value::Int(if valid {
            if mode {
                cfg.global_extra_mode_flag[index as usize]
            } else {
                i64::from(cfg.global_extra_switch_flag[index as usize])
            }
        } else {
            0
        }));
    } else {
        let value = if action == 0 {
            args[1].as_i64().context("native extra switch/mode value")?
        } else {
            extra_default(ctx, index as usize, mode)
        };
        if valid {
            let cfg = &mut ctx.globals.syscom.original_config;
            if mode {
                cfg.global_extra_mode_flag[index as usize] = value;
            } else {
                cfg.global_extra_switch_flag[index as usize] = value != 0;
            }
        }
    }
    Ok(true)
}
fn dispatch_controls(
    ctx: &mut CommandContext,
    args: &[crate::runtime::Value],
) -> anyhow::Result<bool> {
    use crate::runtime::forms::codes::syscom_op as op;
    use anyhow::{Context, ensure};
    let call = ctx
        .vm_call
        .as_ref()
        .context("native SYSCOM controls metadata")?;
    let command = *call
        .element
        .get(1)
        .context("native SYSCOM controls operation")?;
    if matches!(
        command,
        op::SET_WINDOW_MODE | op::SET_WINDOW_MODE_DEFAULT | op::GET_WINDOW_MODE
    ) {
        ensure!(
            call.element.len() == 2 && call.al_id == 0,
            "native window-mode chain/overload"
        );
        ensure!(
            args.len() == usize::from(command == op::SET_WINDOW_MODE),
            "native window-mode arguments"
        );
        if command == op::GET_WINDOW_MODE {
            ctx.push(crate::runtime::Value::Int(
                ctx.globals
                    .syscom
                    .config_int
                    .get(&op::GET_WINDOW_MODE)
                    .copied()
                    .unwrap_or(0),
            ));
        } else {
            let value = if command == op::SET_WINDOW_MODE {
                args[0].as_i64().context("native window mode")?
            } else {
                ctx.tables
                    .gameexe
                    .as_ref()
                    .and_then(|c| c.get_i64("CONFIG.WINDOW_MODE"))
                    .unwrap_or(0)
                    .clamp(0, 1)
            };
            ctx.globals
                .syscom
                .config_int
                .insert(op::GET_WINDOW_MODE, value);
            ctx.globals.syscom.original_config.screen_size_mode = value.clamp(0, 1);
        }
        return Ok(true);
    }
    let indexed = matches!(command, op::SET_MWND_BTN_ENABLE | op::SET_MWND_BTN_DISABLE);
    if !indexed
        && !matches!(
            command,
            op::SET_SYSCOM_MENU_ENABLE
                | op::SET_SYSCOM_MENU_DISABLE
                | op::SET_MWND_BTN_TOUCH_ENABLE
                | op::SET_MWND_BTN_TOUCH_DISABLE
        )
    {
        return Ok(false);
    }
    ensure!(
        call.element.len() == 2,
        "unsupported native SYSCOM controls chain"
    );
    if indexed {
        ensure!(
            args.len() <= 1 && call.al_id == args.len() as i64,
            "native button-control overload/arguments"
        );
        let disabled = command == op::SET_MWND_BTN_DISABLE;
        if args.is_empty() {
            ctx.globals.syscom.mwnd_btn_disable_all = disabled;
        } else {
            let index = args[0].as_i64().context("native message button index")?;
            ensure!(
                i32::try_from(index).is_ok()
                    && (ctx.globals.syscom.mwnd_btn_disable.contains_key(&index)
                        || ctx.globals.syscom.mwnd_btn_disable.len() < 1024),
                "native button-control index/map budget"
            );
            ctx.globals.syscom.mwnd_btn_disable.insert(index, disabled);
        }
    } else {
        ensure!(
            args.is_empty() && call.al_id == 0,
            "native SYSCOM control arguments"
        );
        match command {
            op::SET_SYSCOM_MENU_ENABLE => ctx.globals.syscom.syscom_menu_disable = false,
            op::SET_SYSCOM_MENU_DISABLE => ctx.globals.syscom.syscom_menu_disable = true,
            op::SET_MWND_BTN_TOUCH_ENABLE => ctx.globals.syscom.mwnd_btn_touch_disable = false,
            op::SET_MWND_BTN_TOUCH_DISABLE => ctx.globals.syscom.mwnd_btn_touch_disable = true,
            _ => unreachable!(),
        }
    }
    Ok(true)
}

#[cfg(test)]
mod config_tests {
    use super::*;
    use crate::runtime::{Value, VmCallMeta, forms::codes::syscom_op as op};
    #[test]
    fn game_extra_defaults_and_overrides_are_native_saved_state() {
        let mut ctx = CommandContext::new("asset".into());
        ctx.tables.gameexe = Some(std::sync::Arc::new(
            crate::gameexe::GameexeConfig::from_text(
                "#CONFIG.GLOBAL_EXTRA_SWITCH.000.ONOFF=0\n#CONFIG.GLOBAL_EXTRA_MODE.001.MODE=9\n",
            ),
        ));
        initialize_config(&mut ctx);
        assert!(!ctx.globals.syscom.original_config.global_extra_switch_flag[0]);
        assert_eq!(
            ctx.globals.syscom.original_config.global_extra_mode_flag[1],
            9
        );
        ctx.vm_call = Some(VmCallMeta {
            element: vec![63, op::SET_GLOBAL_EXTRA_SWITCH_ONOFF],
            ..Default::default()
        });
        dispatch_config(&mut ctx, &[Value::Int(0), Value::Int(1)]).unwrap();
        ctx.vm_call.as_mut().unwrap().element[1] = op::GET_GLOBAL_EXTRA_SWITCH_ONOFF;
        dispatch_config(&mut ctx, &[Value::Int(0)]).unwrap();
        assert_eq!(ctx.pop().and_then(|v| v.as_i64()), Some(1));
        ctx.vm_call.as_mut().unwrap().element[1] = op::SET_GLOBAL_EXTRA_SWITCH_ONOFF_DEFAULT;
        dispatch_config(&mut ctx, &[Value::Int(0)]).unwrap();
        assert!(!ctx.globals.syscom.original_config.global_extra_switch_flag[0]);
        let checkpoint = ctx.clone();
        assert!(
            std::sync::Arc::ptr_eq(
                ctx.tables.gameexe.as_ref().unwrap(),
                checkpoint.tables.gameexe.as_ref().unwrap()
            ),
            "checkpoints share immutable config"
        );
        ctx.globals.syscom.original_config.global_extra_mode_flag[1] = 3;
        assert_eq!(
            checkpoint
                .globals
                .syscom
                .original_config
                .global_extra_mode_flag[1],
            9,
            "mutable native config remains independent"
        );
    }
}
// Original save slots, thumbnails and global/config/read streams.
pub use super::syscom_original::{
    CAPTURE_PRIOR_SAVE, free_runtime_save_thumb_capture, load_global_save, write_global_save,
};
pub(crate) use super::syscom_original::{
    prepare_runtime_save_thumb_capture, read_save_flag_values, write_runtime_slot_thumb,
    write_save_flag_values,
};
pub fn append_current_save_message(ctx: &mut CommandContext, msg: &str) {
    ctx.globals.syscom.current_save_message.push_str(msg);
    ctx.globals.syscom.current_save_message = ctx
        .globals
        .syscom
        .current_save_message
        .chars()
        .take(256)
        .collect();
    ctx.globals.syscom.current_save_full_message.push_str(msg);
    ctx.globals.syscom.current_save_full_message = ctx
        .globals
        .syscom
        .current_save_full_message
        .chars()
        .take(256)
        .collect();
    if let Some(s) = &mut ctx.local_save_snapshot {
        s.save_msg = ctx.globals.syscom.current_save_message.clone();
        s.save_full_msg = ctx.globals.syscom.current_save_full_message.clone();
    }
}

pub(crate) fn cursor_defaults(ctx:&CommandContext)->(i64,i64) { super::cursor_preferences::defaults(ctx) }

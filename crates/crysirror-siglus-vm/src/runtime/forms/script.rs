// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Checked native SCRIPT boundary. Original opcode behavior lives together in
//! script_native; reusable text/input/timeline/audio mechanisms stay in engine.
use super::codes;
use crate::runtime::{CommandContext, Value};
use anyhow::{Context, Result, ensure};
/// Original eng_etc.cpp skip precedence, shared by the native query and
/// native frame's 32x game-time selection. No engine playback timer lives here.
pub(crate) fn is_skipping(ctx: &CommandContext) -> Result<bool> {
    Ok(ctx.runtime_is_skipping())
}

pub(super) fn signature(op: i32) -> Option<(usize, bool, bool, &'static str)> {
    Some(match op {
        codes::ELM_SCRIPT_SET_AUTO_SAVEPOINT_OFF => (0, false, true, "SET_AUTO_SAVEPOINT_OFF"),
        codes::ELM_SCRIPT_SET_AUTO_SAVEPOINT_ON => (0, false, true, "SET_AUTO_SAVEPOINT_ON"),
        codes::ELM_SCRIPT_SET_SKIP_DISABLE => (0, false, true, "SET_SKIP_DISABLE"),
        codes::ELM_SCRIPT_SET_SKIP_ENABLE => (0, false, true, "SET_SKIP_ENABLE"),
        codes::ELM_SCRIPT_SET_SKIP_DISABLE_FLAG => (1, false, true, "SET_SKIP_DISABLE_FLAG"),
        codes::ELM_SCRIPT_GET_SKIP_DISABLE_FLAG => (0, false, true, "GET_SKIP_DISABLE_FLAG"),
        codes::ELM_SCRIPT_SET_CTRL_SKIP_DISABLE => (0, false, true, "SET_CTRL_SKIP_DISABLE"),
        codes::ELM_SCRIPT_SET_CTRL_SKIP_ENABLE => (0, false, true, "SET_CTRL_SKIP_ENABLE"),
        codes::ELM_SCRIPT_SET_CTRL_SKIP_DISABLE_FLAG => {
            (1, false, true, "SET_CTRL_SKIP_DISABLE_FLAG")
        }
        codes::ELM_SCRIPT_GET_CTRL_SKIP_DISABLE_FLAG => {
            (0, false, true, "GET_CTRL_SKIP_DISABLE_FLAG")
        }
        codes::ELM_SCRIPT_CHECK_SKIP => (0, false, true, "CHECK_SKIP"),
        codes::ELM_SCRIPT_SET_STOP_SKIP_BY_KEY_DISABLE => {
            (0, false, true, "SET_STOP_SKIP_BY_KEY_DISABLE")
        }
        codes::ELM_SCRIPT_SET_STOP_SKIP_BY_KEY_ENABLE => {
            (0, false, true, "SET_STOP_SKIP_BY_KEY_ENABLE")
        }
        codes::ELM_SCRIPT_SET_END_MSG_BY_KEY_DISABLE => {
            (0, false, true, "SET_END_MSG_BY_KEY_DISABLE")
        }
        codes::ELM_SCRIPT_SET_END_MSG_BY_KEY_ENABLE => {
            (0, false, true, "SET_END_MSG_BY_KEY_ENABLE")
        }
        codes::ELM_SCRIPT_SET_SKIP_UNREAD_MESSAGE_FLAG => {
            (1, false, true, "SET_SKIP_UNREAD_MESSAGE_FLAG")
        }
        codes::ELM_SCRIPT_GET_SKIP_UNREAD_MESSAGE_FLAG => {
            (0, false, true, "GET_SKIP_UNREAD_MESSAGE_FLAG")
        }
        codes::ELM_SCRIPT_START_AUTO_MODE => (0, false, true, "START_AUTO_MODE"),
        codes::ELM_SCRIPT_END_AUTO_MODE => (0, false, true, "END_AUTO_MODE"),
        codes::ELM_SCRIPT_SET_AUTO_MODE_MOJI_WAIT => (1, false, true, "SET_AUTO_MODE_MOJI_WAIT"),
        codes::ELM_SCRIPT_SET_AUTO_MODE_MOJI_WAIT_DEFAULT => {
            (0, false, true, "SET_AUTO_MODE_MOJI_WAIT_DEFAULT")
        }
        codes::ELM_SCRIPT_GET_AUTO_MODE_MOJI_WAIT => (0, false, true, "GET_AUTO_MODE_MOJI_WAIT"),
        codes::ELM_SCRIPT_SET_AUTO_MODE_MIN_WAIT => (1, false, true, "SET_AUTO_MODE_MIN_WAIT"),
        codes::ELM_SCRIPT_SET_AUTO_MODE_MIN_WAIT_DEFAULT => {
            (0, false, true, "SET_AUTO_MODE_MIN_WAIT_DEFAULT")
        }
        codes::ELM_SCRIPT_GET_AUTO_MODE_MIN_WAIT => (0, false, true, "GET_AUTO_MODE_MIN_WAIT"),
        codes::ELM_SCRIPT_SET_AUTO_MODE_MOJI_CNT => (1, false, true, "SET_AUTO_MODE_MOJI_CNT"),
        codes::ELM_SCRIPT_SET_MOUSE_CURSOR_HIDE_ONOFF => {
            (1, false, true, "SET_MOUSE_CURSOR_HIDE_ONOFF")
        }
        codes::ELM_SCRIPT_SET_MOUSE_CURSOR_HIDE_ONOFF_DEFAULT => {
            (0, false, true, "SET_MOUSE_CURSOR_HIDE_ONOFF_DEFAULT")
        }
        codes::ELM_SCRIPT_GET_MOUSE_CURSOR_HIDE_ONOFF => {
            (0, false, true, "GET_MOUSE_CURSOR_HIDE_ONOFF")
        }
        codes::ELM_SCRIPT_SET_MOUSE_CURSOR_HIDE_TIME => {
            (1, false, true, "SET_MOUSE_CURSOR_HIDE_TIME")
        }
        codes::ELM_SCRIPT_SET_MOUSE_CURSOR_HIDE_TIME_DEFAULT => {
            (0, false, true, "SET_MOUSE_CURSOR_HIDE_TIME_DEFAULT")
        }
        codes::ELM_SCRIPT_GET_MOUSE_CURSOR_HIDE_TIME => {
            (0, false, true, "GET_MOUSE_CURSOR_HIDE_TIME")
        }
        codes::ELM_SCRIPT_SET_MESSAGE_SPEED => (1, false, true, "SET_MESSAGE_SPEED"),
        codes::ELM_SCRIPT_SET_MESSAGE_SPEED_DEFAULT => {
            (0, false, true, "SET_MESSAGE_SPEED_DEFAULT")
        }
        codes::ELM_SCRIPT_GET_MESSAGE_SPEED => (0, false, true, "GET_MESSAGE_SPEED"),
        codes::ELM_SCRIPT_SET_MESSAGE_NOWAIT_FLAG => (1, false, true, "SET_MESSAGE_NOWAIT_FLAG"),
        codes::ELM_SCRIPT_GET_MESSAGE_NOWAIT_FLAG => (0, false, true, "GET_MESSAGE_NOWAIT_FLAG"),
        codes::ELM_SCRIPT_SET_MSG_ASYNC_MODE_ON => (0, false, true, "SET_MSG_ASYNC_MODE_ON"),
        codes::ELM_SCRIPT_SET_MSG_ASYNC_MODE_ON_ONCE => {
            (0, false, true, "SET_MSG_ASYNC_MODE_ON_ONCE")
        }
        codes::ELM_SCRIPT_SET_MSG_ASYNC_MODE_OFF => (0, false, true, "SET_MSG_ASYNC_MODE_OFF"),
        codes::ELM_SCRIPT_SET_HIDE_MWND_DISABLE => (0, false, true, "SET_HIDE_MWND_DISABLE"),
        codes::ELM_SCRIPT_SET_HIDE_MWND_ENABLE => (0, false, true, "SET_HIDE_MWND_ENABLE"),
        codes::ELM_SCRIPT_SET_MSG_BACK_DISABLE => (0, false, true, "SET_MSG_BACK_DISABLE"),
        codes::ELM_SCRIPT_SET_MSG_BACK_ENABLE => (0, false, true, "SET_MSG_BACK_ENABLE"),
        codes::ELM_SCRIPT_SET_MSG_BACK_OFF => (0, false, true, "SET_MSG_BACK_OFF"),
        codes::ELM_SCRIPT_SET_MSG_BACK_ON => (0, false, true, "SET_MSG_BACK_ON"),
        codes::ELM_SCRIPT_SET_MSG_BACK_DISP_OFF => (0, false, true, "SET_MSG_BACK_DISP_OFF"),
        codes::ELM_SCRIPT_SET_MSG_BACK_DISP_ON => (0, false, true, "SET_MSG_BACK_DISP_ON"),
        codes::ELM_SCRIPT_SET_MOUSE_DISP_OFF => (0, false, true, "SET_MOUSE_DISP_OFF"),
        codes::ELM_SCRIPT_SET_MOUSE_DISP_ON => (0, false, true, "SET_MOUSE_DISP_ON"),
        codes::ELM_SCRIPT_SET_MOUSE_MOVE_BY_KEY_DISABLE => {
            (0, false, true, "SET_MOUSE_MOVE_BY_KEY_DISABLE")
        }
        codes::ELM_SCRIPT_SET_MOUSE_MOVE_BY_KEY_ENABLE => {
            (0, false, true, "SET_MOUSE_MOVE_BY_KEY_ENABLE")
        }
        codes::ELM_SCRIPT_SET_KEY_DISABLE => (1, false, true, "SET_KEY_DISABLE"),
        codes::ELM_SCRIPT_SET_KEY_ENABLE => (1, false, true, "SET_KEY_ENABLE"),
        codes::ELM_SCRIPT_SET_MWND_ANIME_OFF_FLAG => (1, false, true, "SET_MWND_ANIME_OFF_FLAG"),
        codes::ELM_SCRIPT_GET_MWND_ANIME_OFF_FLAG => (0, false, true, "GET_MWND_ANIME_OFF_FLAG"),
        codes::ELM_SCRIPT_SET_MWND_ANIME_ON_FLAG => (1, false, true, "SET_MWND_ANIME_ON_FLAG"),
        codes::ELM_SCRIPT_GET_MWND_ANIME_ON_FLAG => (0, false, true, "GET_MWND_ANIME_ON_FLAG"),
        codes::ELM_SCRIPT_SET_MWND_DISP_OFF_FLAG => (1, false, true, "SET_MWND_DISP_OFF_FLAG"),
        codes::ELM_SCRIPT_GET_MWND_DISP_OFF_FLAG => (0, false, true, "GET_MWND_DISP_OFF_FLAG"),
        codes::ELM_SCRIPT_SET_KOE_DONT_STOP_ON_FLAG => {
            (1, false, true, "SET_KOE_DONT_STOP_ON_FLAG")
        }
        codes::ELM_SCRIPT_GET_KOE_DONT_STOP_ON_FLAG => {
            (0, false, true, "GET_KOE_DONT_STOP_ON_FLAG")
        }
        codes::ELM_SCRIPT_SET_KOE_DONT_STOP_OFF_FLAG => {
            (1, false, true, "SET_KOE_DONT_STOP_OFF_FLAG")
        }
        codes::ELM_SCRIPT_GET_KOE_DONT_STOP_OFF_FLAG => {
            (0, false, true, "GET_KOE_DONT_STOP_OFF_FLAG")
        }
        codes::ELM_SCRIPT_SET_SHORTCUT_ENABLE => (0, false, true, "SET_SHORTCUT_ENABLE"),
        codes::ELM_SCRIPT_SET_SHORTCUT_DISABLE => (0, false, true, "SET_SHORTCUT_DISABLE"),
        codes::ELM_SCRIPT_SET_QUAKE_STOP_FLAG => (1, false, true, "SET_QUAKE_STOP_FLAG"),
        codes::ELM_SCRIPT_GET_QUAKE_STOP_FLAG => (0, false, true, "GET_QUAKE_STOP_FLAG"),
        codes::ELM_SCRIPT_SET_EMOTE_MOUTH_STOP_FLAG => {
            (1, false, true, "SET_EMOTE_MOUTH_STOP_FLAG")
        }
        codes::ELM_SCRIPT_GET_EMOTE_MOUTH_STOP_FLAG => {
            (0, false, true, "GET_EMOTE_MOUTH_STOP_FLAG")
        }
        codes::ELM_SCRIPT_START_BGMFADE => (0, false, true, "START_BGMFADE"),
        codes::ELM_SCRIPT_END_BGMFADE => (0, false, true, "END_BGMFADE"),
        codes::ELM_SCRIPT_SET_VSYNC_WAIT_OFF_FLAG => (1, false, true, "SET_VSYNC_WAIT_OFF_FLAG"),
        codes::ELM_SCRIPT_GET_VSYNC_WAIT_OFF_FLAG => (0, false, true, "GET_VSYNC_WAIT_OFF_FLAG"),
        codes::ELM_SCRIPT_SET_SKIP_TRIGGER => (0, false, true, "SET_SKIP_TRIGGER"),
        codes::ELM_SCRIPT_IGNORE_R_ON => (0, false, true, "IGNORE_R_ON"),
        codes::ELM_SCRIPT_IGNORE_R_OFF => (0, false, true, "IGNORE_R_OFF"),
        codes::ELM_SCRIPT_SET_CURSOR_NO => (1, false, true, "SET_CURSOR_NO"),
        codes::ELM_SCRIPT_GET_CURSOR_NO => (0, false, true, "GET_CURSOR_NO"),
        codes::ELM_SCRIPT_SET_TIME_STOP_FLAG => (1, false, true, "SET_TIME_STOP_FLAG"),
        codes::ELM_SCRIPT_GET_TIME_STOP_FLAG => (0, false, true, "GET_TIME_STOP_FLAG"),
        codes::ELM_SCRIPT_SET_COUNTER_TIME_STOP_FLAG => {
            (1, false, true, "SET_COUNTER_TIME_STOP_FLAG")
        }
        codes::ELM_SCRIPT_GET_COUNTER_TIME_STOP_FLAG => {
            (0, false, true, "GET_COUNTER_TIME_STOP_FLAG")
        }
        codes::ELM_SCRIPT_SET_FRAME_ACTION_TIME_STOP_FLAG => {
            (1, false, true, "SET_FRAME_ACTION_TIME_STOP_FLAG")
        }
        codes::ELM_SCRIPT_GET_FRAME_ACTION_TIME_STOP_FLAG => {
            (0, false, true, "GET_FRAME_ACTION_TIME_STOP_FLAG")
        }
        codes::ELM_SCRIPT_SET_STAGE_TIME_STOP_FLAG => (1, false, true, "SET_STAGE_TIME_STOP_FLAG"),
        codes::ELM_SCRIPT_GET_STAGE_TIME_STOP_FLAG => (0, false, true, "GET_STAGE_TIME_STOP_FLAG"),
        codes::ELM_SCRIPT_SET_FONT_NAME => (1, true, true, "SET_FONT_NAME"),
        codes::ELM_SCRIPT_SET_FONT_NAME_DEFAULT => (0, false, true, "SET_FONT_NAME_DEFAULT"),
        codes::ELM_SCRIPT_GET_FONT_NAME => (0, false, true, "GET_FONT_NAME"),
        codes::ELM_SCRIPT_SET_FONT_BOLD => (1, false, true, "SET_FONT_BOLD"),
        codes::ELM_SCRIPT_SET_FONT_BOLD_DEFAULT => (0, false, true, "SET_FONT_BOLD_DEFAULT"),
        codes::ELM_SCRIPT_GET_FONT_BOLD => (0, false, true, "GET_FONT_BOLD"),
        codes::ELM_SCRIPT_SET_FONT_SHADOW => (1, false, true, "SET_FONT_SHADOW"),
        codes::ELM_SCRIPT_SET_FONT_SHADOW_DEFAULT => (0, false, true, "SET_FONT_SHADOW_DEFAULT"),
        codes::ELM_SCRIPT_GET_FONT_SHADOW => (0, false, true, "GET_FONT_SHADOW"),
        codes::ELM_SCRIPT_SET_JOYPAD_MODE_OVERRIDE => (1, false, false, "SET_JOYPAD_MODE_OVERRIDE"),
        codes::ELM_SCRIPT_RESET_JOYPAD_MODE_OVERRIDE => {
            (0, false, false, "RESET_JOYPAD_MODE_OVERRIDE")
        }
        codes::ELM_SCRIPT_GET_JOYPAD_MODE_OVERRIDE => (0, false, true, "GET_JOYPAD_MODE_OVERRIDE"),
        _ => return None,
    })
}
pub fn dispatch(ctx: &mut CommandContext, args: &[Value]) -> Result<bool> {
    let call = ctx.vm_call.as_ref().context("native SCRIPT metadata")?;
    ensure!(
        call.element.len() == 2 && call.al_id == 0,
        "native SCRIPT chain/overload"
    );
    let op = call.element[1];
    let form = call.element[0] as u32;
    let (arity, string, ready, name) = signature(op).context("unknown native SCRIPT opcode")?;
    ensure!(args.len() == arity, "native SCRIPT {name} argument count");
    if arity == 1 {
        if string {
            ensure!(
                args[0].as_str().is_some_and(|s| s.len() <= 4096),
                "native SCRIPT string/budget"
            );
        } else {
            let value = args[0].as_i64().context("native SCRIPT integer")?;
            ensure!(i32::try_from(value).is_ok(), "native SCRIPT integer width");
            if op == codes::ELM_SCRIPT_SET_CURSOR_NO {
                ensure!((-1..256).contains(&value), "native cursor index limit");
            }
        }
    }
    // All original command bodies are ported as one module. Flags whose effect has no
    // engine consumer yet still store, save and read back as the original does; only
    // the joypad override, which needs real gamepad policy, stays refused.
    ensure!(
        ready,
        "native SCRIPT {name}: shared capability consumer is not connected"
    );
    ensure!(
        super::script_native::dispatch(ctx, form, args)?,
        "native SCRIPT {name} was not dispatched"
    );
    Ok(true)
}
/// EXCALL owns only the original font and joypad sub-family. Validate its
/// arguments with the same public signature table before native state changes.
pub(super) fn dispatch_excall(ctx:&mut CommandContext, op:i32, args:&[Value])->Result<bool> {
    use codes::*;
    ensure!(matches!(op, ELM_SCRIPT_SET_FONT_NAME | ELM_SCRIPT_SET_FONT_NAME_DEFAULT
        | ELM_SCRIPT_GET_FONT_NAME | ELM_SCRIPT_SET_FONT_BOLD | ELM_SCRIPT_SET_FONT_BOLD_DEFAULT
        | ELM_SCRIPT_GET_FONT_BOLD | ELM_SCRIPT_SET_FONT_SHADOW | ELM_SCRIPT_SET_FONT_SHADOW_DEFAULT
        | ELM_SCRIPT_GET_FONT_SHADOW | ELM_SCRIPT_SET_JOYPAD_MODE_OVERRIDE
        | ELM_SCRIPT_RESET_JOYPAD_MODE_OVERRIDE | ELM_SCRIPT_GET_JOYPAD_MODE_OVERRIDE),
        "unsupported native EXCALL.SCRIPT opcode {op}");
    let (arity,string,ready,name)=signature(op).context("native EXCALL.SCRIPT signature")?;
    ensure!(args.len()==arity,"native EXCALL.SCRIPT {name} argument count");
    if arity==1 {
        if string {ensure!(args[0].as_str().is_some_and(|s|s.len()<=4096),"native EXCALL.SCRIPT string/budget");}
        else {ensure!(args[0].as_i64().is_some_and(|v|i32::try_from(v).is_ok()),"native EXCALL.SCRIPT integer width");}
    }
    ensure!(ready,"native EXCALL.SCRIPT {name}: shared capability consumer is not connected");
    super::script_native::dispatch_excall(ctx,op,args)
}

#[cfg(test)]
mod font_tests {
    use super::*;
    #[test]
    fn font_overrides_keep_raw_default_values_and_validate_before_mutation() {
        let mut ctx = CommandContext::new("asset".into());
        ctx.vm_call = Some(crate::runtime::VmCallMeta {
            element: vec![72, codes::ELM_SCRIPT_SET_FONT_SHADOW],
            ..Default::default()
        });
        dispatch(&mut ctx, &[Value::Int(3)]).unwrap();
        assert_eq!(ctx.globals.script.font_shadow, 3);
        assert!(dispatch(&mut ctx, &[Value::Int(i64::MAX)]).is_err());
        assert_eq!(ctx.globals.script.font_shadow, 3);
        ctx.vm_call.as_mut().unwrap().element[1] = codes::ELM_SCRIPT_GET_FONT_SHADOW;
        dispatch(&mut ctx, &[]).unwrap();
        assert_eq!(ctx.pop().and_then(|v| v.as_i64()), Some(3));
        ctx.vm_call.as_mut().unwrap().element[1] = codes::ELM_SCRIPT_SET_FONT_SHADOW_DEFAULT;
        dispatch(&mut ctx, &[]).unwrap();
        assert_eq!(ctx.globals.script.font_shadow, -1);
        ctx.vm_call.as_mut().unwrap().element[1] = codes::ELM_SCRIPT_GET_FONT_SHADOW;
        dispatch(&mut ctx, &[]).unwrap();
        assert_eq!(ctx.pop().and_then(|v| v.as_i64()), Some(-1));
    }
    #[test]
    fn skip_query_preserves_ctrl_disable_focus_and_shift_precedence() {
        let mut ctx = CommandContext::new("asset".into());
        ctx.vm_call = Some(crate::runtime::VmCallMeta {
            element: vec![64, codes::ELM_SCRIPT_CHECK_SKIP],
            ..Default::default()
        });
        ctx.input.on_key_down(crate::runtime::input::VmKey::Control);
        ctx.globals.script.skip_disable = true;
        dispatch(&mut ctx, &[]).unwrap();
        assert_eq!(ctx.pop().and_then(|v| v.as_i64()), Some(1));
        ctx.globals.script.ctrl_disable = true;
        dispatch(&mut ctx, &[]).unwrap();
        assert_eq!(ctx.pop().and_then(|v| v.as_i64()), Some(0));
        ctx.globals.script.ctrl_disable = false;
        ctx.input.on_key_down(crate::runtime::input::VmKey::Shift);
        dispatch(&mut ctx, &[]).unwrap();
        assert_eq!(ctx.pop().and_then(|v| v.as_i64()), Some(0));
        ctx.input.on_key_up(crate::runtime::input::VmKey::Shift);
        ctx.globals.focused_editbox = Some((1, 0));
        dispatch(&mut ctx, &[]).unwrap();
        assert_eq!(ctx.pop().and_then(|v| v.as_i64()), Some(0));
    }
}

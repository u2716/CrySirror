// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Complete original MWND KOE/EXKOE family. Only receiver, message/save and
//! history policy lives here; voice resources, PCM, rate and waits use the
//! existing engine-backed KOE bridge. Reference: pinned stage.rs Koe family.
use super::*;
use crate::runtime::{
    Value,
    forms::{codes, stage},
};

pub(crate) fn global_operation(op: i32) -> Option<i32> {
    Some(match op {
        codes::ELM_MWND_KOE => codes::ELM_GLOBAL_KOE,
        codes::ELM_MWND_KOE_PLAY_WAIT => codes::ELM_GLOBAL_KOE_PLAY_WAIT,
        codes::ELM_MWND_KOE_PLAY_WAIT_KEY => codes::ELM_GLOBAL_KOE_PLAY_WAIT_KEY,
        codes::ELM_MWND_EXKOE => codes::ELM_GLOBAL_EXKOE,
        codes::ELM_MWND_EXKOE_PLAY_WAIT => codes::ELM_GLOBAL_EXKOE_PLAY_WAIT,
        codes::ELM_MWND_EXKOE_PLAY_WAIT_KEY => codes::ELM_GLOBAL_EXKOE_PLAY_WAIT_KEY,
        _ => return None,
    })
}

pub(crate) fn window_operation(ctx: &CommandContext) -> Option<i32> {
    let e = &ctx.vm_call.as_ref()?.element;
    match e.as_slice() {
        [root, codes::ELM_STAGE_MWND, codes::ELM_ARRAY, _, op]
            if matches!(
                *root,
                codes::ELM_GLOBAL_BACK | codes::ELM_GLOBAL_FRONT | codes::ELM_GLOBAL_NEXT
            ) =>
        {
            global_operation(*op)
        }
        [
            root,
            codes::ELM_ARRAY,
            _,
            codes::ELM_STAGE_MWND,
            codes::ELM_ARRAY,
            _,
            op,
        ] if stage::is_stage_form_id(ctx, *root) => global_operation(*op),
        _ => None,
    }
}

fn target(ctx: &CommandContext) -> Result<(u32, i64, usize)> {
    let element = ctx
        .vm_call
        .as_ref()
        .map(|m| m.element.as_slice())
        .unwrap_or(&[]);
    let (root, stage_index, index) = match element {
        [root, codes::ELM_STAGE_MWND, codes::ELM_ARRAY, index, _]
            if matches!(
                *root,
                codes::ELM_GLOBAL_BACK | codes::ELM_GLOBAL_FRONT | codes::ELM_GLOBAL_NEXT
            ) =>
        {
            (
                *root,
                match *root {
                    codes::ELM_GLOBAL_BACK => 0,
                    codes::ELM_GLOBAL_FRONT => 1,
                    _ => 2,
                },
                *index,
            )
        }
        [
            root,
            codes::ELM_ARRAY,
            stage_index,
            codes::ELM_STAGE_MWND,
            codes::ELM_ARRAY,
            index,
            _,
        ] if stage::is_stage_form_id(ctx, *root) => (*root, i64::from(*stage_index), *index),
        _ => (
            *ctx.globals
                .current_mwnd_element
                .first()
                .unwrap_or(&(ctx.ids.form_global_stage as i32)),
            ctx.globals.current_mwnd_stage_idx,
            i32::try_from(ctx.globals.current_mwnd_no.unwrap_or(0))?,
        ),
    };
    ensure!(
        (0..=2).contains(&stage_index) && (0..16).contains(&index),
        "native voice MWND receiver bounds"
    );
    Ok((
        stage::stage_storage_form_id(ctx, root),
        stage_index,
        index as usize,
    ))
}

pub(crate) fn remember(
    ctx: &mut CommandContext,
    number: i64,
    character: i64,
    ex: bool,
) -> Result<()> {
    if ex {
        return Ok(());
    }
    let (family, stage, index) = target(ctx)?;
    let history = ctx.ids.form_global_msgbk;
    if number >= 0 && history != 0 {
        let st = ctx.globals.msgbk_forms.entry(history).or_default();
        ensure!(
            st.history
                .get(st.history_insert_pos)
                .is_none_or(|entry| entry.koe_no_list.len() < 4096),
            "native message voice history budget"
        );
        st.add_koe(
            number,
            character,
            ctx.current_scene_no.unwrap_or(-1),
            if ctx.current_line_no > 0 {
                ctx.current_line_no
            } else {
                0
            },
        );
    }
    let windows = ctx
        .globals
        .stage_forms
        .entry(family)
        .or_default()
        .mwnd_lists
        .entry(stage)
        .or_default();
    windows.resize_with(windows.len().max(index + 1), Default::default);
    windows[index].koe = Some((number, character));
    Ok(())
}

pub(crate) fn dispatch(ctx: &mut CommandContext, op: i32, args: &[Value]) -> Result<bool> {
    let Some(global) = global_operation(op) else {
        return Ok(false);
    };
    target(ctx)?;
    crate::koe_bridge::dispatch(ctx, global as u32, args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{VmCallMeta, constants::fm};
    fn call(ctx: &mut CommandContext, chain: Vec<i32>, args: &[Value], ret: i32) {
        let root = chain[0] as u32;
        ctx.vm_call = Some(VmCallMeta {
            element: chain,
            al_id: 0,
            ret_form: i64::from(ret),
        });
        crate::runtime::dispatch_form_code(ctx, root, args).unwrap();
    }
    #[test]
    fn native_window_voice_keeps_receiver_history_and_exkoe_does_not_replace_message_voice() {
        let mut ctx = CommandContext::new("asset".into());
        ctx.current_scene_no = Some(3);
        ctx.current_line_no = 47;
        let root = [
            codes::ELM_GLOBAL_BACK,
            codes::ELM_STAGE_MWND,
            codes::ELM_ARRAY,
            2,
        ];
        call(
            &mut ctx,
            [root.as_slice(), &[codes::ELM_MWND_KOE]].concat(),
            &[Value::Int(101), Value::Int(7)],
            fm::VOID,
        );
        assert!(
            ctx.ui.window.is_none(),
            "voice does not require a private or engine text renderer"
        );
        assert!(
            !ctx.koe.pending.is_empty(),
            "shared audio admission receives the native declaration"
        );
        assert!(ctx.take_read_flag_no_request());
        let family = stage::stage_storage_form_id(&ctx, root[0]);
        assert_eq!(
            ctx.globals.stage_forms[&family].mwnd_lists[&0][2].koe,
            Some((101, 7))
        );
        let history = ctx.ids.form_global_msgbk;
        let first = &ctx.globals.msgbk_forms[&history].history[0];
        assert_eq!(first.koe_no_list, vec![101]);
        assert_eq!(first.chr_no_list, vec![7]);
        assert_eq!((first.scn_no, first.line_no), (3, 47));
        let named = |id, n| Value::NamedArg {
            id,
            value: Box::new(Value::Int(n)),
        };
        call(
            &mut ctx,
            [root.as_slice(), &[codes::ELM_MWND_EXKOE]].concat(),
            &[named(0, 202), named(1, 8)],
            fm::VOID,
        );
        assert_eq!(ctx.koe.current_no(), 202);
        assert!(!ctx.take_read_flag_no_request());
        assert_eq!(
            ctx.globals.stage_forms[&family].mwnd_lists[&0][2].koe,
            Some((101, 7))
        );
        assert_eq!(ctx.globals.syscom.replay_koe, Some((101, 7)));
        assert_eq!(
            ctx.globals.msgbk_forms[&history].history[0].koe_no_list,
            vec![101]
        );
    }
    #[test]
    fn explicit_stage_voice_wait_returns_at_original_eof_and_invalid_receiver_is_atomic() {
        let mut ctx = CommandContext::new("asset".into());
        call(
            &mut ctx,
            vec![
                codes::ELM_GLOBAL_STAGE,
                -1,
                2,
                codes::ELM_STAGE_MWND,
                -1,
                3,
                codes::ELM_MWND_KOE_PLAY_WAIT_KEY,
            ],
            &[Value::Int(-1), Value::Int(5)],
            fm::INT,
        );
        assert!(
            !ctx.wait_poll(),
            "negative original voice has no playing source"
        );
        assert_eq!(ctx.stack.pop().and_then(|v| v.as_i64()), Some(0));
        let family = stage::stage_storage_form_id(&ctx, codes::ELM_GLOBAL_STAGE);
        assert_eq!(
            ctx.globals.stage_forms[&family].mwnd_lists[&2][3].koe,
            Some((-1, 5))
        );
        ctx.vm_call = Some(VmCallMeta {
            element: vec![codes::ELM_GLOBAL_KOE, 123],
            ..Default::default()
        });
        let before = bincode::serialize(&ctx).unwrap();
        assert!(
            crate::runtime::dispatch_form_code(
                &mut ctx,
                codes::ELM_GLOBAL_KOE as u32,
                &[Value::Int(999)]
            )
            .is_err()
        );
        assert_eq!(before, bincode::serialize(&ctx).unwrap());
        ctx.vm_call = Some(VmCallMeta {
            element: vec![
                codes::ELM_GLOBAL_FRONT,
                codes::ELM_STAGE_MWND,
                -1,
                16,
                codes::ELM_MWND_KOE,
            ],
            ..Default::default()
        });
        let before = bincode::serialize(&ctx).unwrap();
        assert!(
            crate::runtime::dispatch_form_code(
                &mut ctx,
                codes::ELM_GLOBAL_FRONT as u32,
                &[Value::Int(100)]
            )
            .is_err()
        );
        assert_eq!(before, bincode::serialize(&ctx).unwrap());
    }
}

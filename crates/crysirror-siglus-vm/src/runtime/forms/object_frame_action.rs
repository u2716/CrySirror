// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original object callback lifecycle policy. Counters use the shared Scene
//! timeline; command execution remains the original VM frame-action runner.
use super::passive_object::ObjectWait;
use crate::runtime::{
    Value,
    forms::codes,
    globals::{Counter, ObjectFrameActionState, PendingFrameActionFinish},
};
use anyhow::{Context, Result, bail, ensure};
fn counter(
    counter: &mut Counter,
    tail: &[i32],
    args: &[Value],
    overload: i64,
) -> Result<Option<Value>> {
    if tail.len() == 1 {
        if overload == 1 {
            ensure!(args.len() == 1, "native object counter setter arguments");
            counter.set_count(args[0].as_i64().context("native object counter value")?);
            return Ok(None);
        }
        ensure!(
            overload == 0 && args.is_empty(),
            "native object counter getter arguments"
        );
        return Ok(Some(Value::Int(counter.get_count())));
    }
    ensure!(
        tail.len() == 2 && overload == 0,
        "native object counter operation chain"
    );
    let command = tail[1];
    let integer = |i: usize| {
        args.get(i)
            .and_then(Value::as_i64)
            .context("native object counter integer")
    };
    if matches!(
        command,
        codes::ELM_COUNTER_START_FRAME
            | codes::ELM_COUNTER_START_FRAME_REAL
            | codes::ELM_COUNTER_START_FRAME_LOOP
            | codes::ELM_COUNTER_START_FRAME_LOOP_REAL
    ) {
        ensure!(args.len() == 3, "native object range counter arguments");
        let (from, to, time) = (integer(0)?, integer(1)?, integer(2)?);
        match command {
            codes::ELM_COUNTER_START_FRAME => counter.start_frame(from, to, time),
            codes::ELM_COUNTER_START_FRAME_REAL => counter.start_frame_real(from, to, time),
            codes::ELM_COUNTER_START_FRAME_LOOP => counter.start_frame_loop(from, to, time),
            _ => counter.start_frame_loop_real(from, to, time),
        }
        return Ok(None);
    }
    ensure!(
        args.len()
            == usize::from(matches!(
                command,
                codes::ELM_COUNTER_SET | codes::ELM_COUNTER_CHECK_VALUE
            )),
        "native object counter arguments"
    );
    match command {
        codes::ELM_COUNTER_SET => counter.set_count(integer(0)?),
        codes::ELM_COUNTER_GET => return Ok(Some(Value::Int(counter.get_count()))),
        codes::ELM_COUNTER_RESET => counter.reset(),
        codes::ELM_COUNTER_START => counter.start(),
        codes::ELM_COUNTER_START_REAL => counter.start_real(),
        codes::ELM_COUNTER_STOP => counter.stop(),
        codes::ELM_COUNTER_RESUME => counter.resume(),
        codes::ELM_COUNTER_CHECK_ACTIVE => {
            return Ok(Some(Value::Int(i64::from(counter.is_running()))));
        }
        codes::ELM_COUNTER_CHECK_VALUE => {
            return Ok(Some(Value::Int(i64::from(
                counter.get_count() >= integer(0)?,
            ))));
        }
        _ => bail!("unsupported native object counter operation {command}"),
    }
    Ok(None)
}

pub(super) fn dispatch(
    fa: &mut ObjectFrameActionState,
    tail: &[i32],
    args: &[Value],
    overload: i64,
    frame_chain: &[i32],
    object_chain: &[i32],
    context: &mut ObjectWait<'_>,
) -> Result<Option<Value>> {
    let Some((&op, rest)) = tail.split_first() else {
        ensure!(
            args.is_empty() && overload == 0,
            "native frame-action value arguments"
        );
        return Ok(None);
    };
    if op == codes::ELM_FRAMEACTION_COUNTER {
        return counter(&mut fa.counter, tail, args, overload);
    }
    ensure!(
        rest.is_empty() && overload == 0,
        "native frame-action chain/overload"
    );
    match op {
        codes::ELM_FRAMEACTION_START | codes::ELM_FRAMEACTION_START_REAL => {
            ensure!(
                (2..=258).contains(&args.len()),
                "native frame-action START arguments/budget"
            );
            let end_time = args[0].as_i64().context("native frame-action end time")?;
            let command = args[1]
                .as_str()
                .context("native frame-action command name")?;
            ensure!(command.len() <= 4096, "native frame-action name budget");
            queue_finish(fa, frame_chain, object_chain, false, context);
            fa.end_time = end_time;
            fa.cmd_name = command.into();
            fa.scn_name = context.scene.into();
            fa.real_time_flag = op == codes::ELM_FRAMEACTION_START_REAL;
            fa.end_flag = false;
            fa.counter.reset();
            if fa.real_time_flag {
                fa.counter.start_real();
            } else {
                fa.counter.start();
            }
            fa.args = args[2..].to_vec();
            Ok(None)
        }
        codes::ELM_FRAMEACTION_END => {
            ensure!(args.is_empty(), "native frame-action END arguments");
            queue_finish(fa, frame_chain, object_chain, true, context);
            fa.reinit_without_finish();
            Ok(None)
        }
        codes::ELM_FRAMEACTION_IS_END_ACTION => {
            ensure!(
                args.is_empty(),
                "native frame-action IS_END_ACTION arguments"
            );
            Ok(Some(Value::Int(i64::from(fa.end_flag))))
        }
        _ => bail!("unsupported native object frame-action operation {op}"),
    }
}
fn queue_finish(
    fa: &ObjectFrameActionState,
    frame_chain: &[i32],
    object_chain: &[i32],
    reinit: bool,
    context: &mut ObjectWait<'_>,
) {
    if fa.cmd_name.is_empty() {
        return;
    }
    context.finishes.push(PendingFrameActionFinish {
        frame_action_chain: frame_chain.to_vec(),
        object_chain: Some(object_chain.to_vec()),
        snapshot: fa.clone(),
        reinit_after_finish: reinit,
        scn_name: fa.scn_name.clone(),
        cmd_name: fa.cmd_name.clone(),
        end_time: fa.end_time,
        args: fa.args.clone(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{CommandContext, VmCallMeta};
    fn call(ctx: &mut CommandContext, chain: &[i32], args: &[Value]) {
        ctx.vm_call = Some(VmCallMeta {
            element: chain.to_vec(),
            al_id: 0,
            ret_form: 0,
        });
        super::super::stage::dispatch(ctx, args).unwrap();
    }
    #[test]
    fn object_and_channel_finish_keep_structural_owners_and_shared_clock_domains() {
        let mut ctx = CommandContext::new(".".into());
        ctx.current_scene_name = Some("original_scene".into());
        // Numeric index115 is also the channel opcode. Never locate the owner
        // by searching for an opcode's numeric value in the element chain.
        let object = [codes::ELM_GLOBAL_FRONT, codes::ELM_STAGE_OBJECT, -1, 115];
        let list = [object.as_slice(), &[codes::ELM_OBJECT_FRAME_ACTION_CH]].concat();
        call(&mut ctx, &[list.as_slice(), &[codes::ELM_FRAMEACTIONLIST_RESIZE]].concat(), &[Value::Int(2)]);
        call(&mut ctx, &[list.as_slice(), &[codes::ELM_FRAMEACTIONLIST_GET_SIZE]].concat(), &[]);
        assert_eq!(ctx.stack.pop().and_then(|v| v.as_i64()), Some(2));
        let action = [
            object.as_slice(),
            &[codes::ELM_OBJECT_FRAME_ACTION_CH, -1, 0],
        ]
        .concat();
        let command = |op| [action.as_slice(), &[op]].concat();
        let start =
            |end, name: &str| vec![Value::Int(end), Value::Str(name.into()), Value::Int(91)];
        call(
            &mut ctx,
            &command(codes::ELM_FRAMEACTION_START),
            &start(100, "old"),
        );
        let family = super::super::stage::stage_storage_form_id(&ctx, object[0]);
        ctx.globals
            .stage_forms
            .get_mut(&family)
            .unwrap()
            .object_lists
            .get_mut(&1)
            .unwrap()[115]
            .frame_action_ch[0]
            .counter
            .update_time(7, 40);
        call(&mut ctx, &command(codes::ELM_FRAMEACTION_COUNTER), &[]);
        assert_eq!(ctx.stack.pop().and_then(|v| v.as_i64()), Some(7));
        call(
            &mut ctx,
            &command(codes::ELM_FRAMEACTION_START_REAL),
            &start(150, "new"),
        );
        let pending = ctx.globals.pending_frame_action_finishes.last().unwrap();
        assert_eq!(pending.object_chain.as_deref(), Some(object.as_slice()));
        assert_eq!(pending.frame_action_chain, action);
        assert_eq!(pending.snapshot.counter.get_count(), 7);
        assert_eq!(pending.snapshot.cmd_name, "old");
        assert_eq!(pending.snapshot.args.len(), 1);
        assert_eq!(pending.snapshot.args[0].as_i64(), Some(91));
        assert!(!pending.reinit_after_finish);
        ctx.globals
            .stage_forms
            .get_mut(&family)
            .unwrap()
            .object_lists
            .get_mut(&1)
            .unwrap()[115]
            .frame_action_ch[0]
            .counter
            .update_time(7, 40);
        call(&mut ctx, &command(codes::ELM_FRAMEACTION_COUNTER), &[]);
        assert_eq!(ctx.stack.pop().and_then(|v| v.as_i64()), Some(40));
        call(&mut ctx, &command(codes::ELM_FRAMEACTION_END), &[]);
        let pending = ctx.globals.pending_frame_action_finishes.last().unwrap();
        assert!(pending.reinit_after_finish);
        assert_eq!(pending.snapshot.end_time, 150);
        let fa = &ctx.globals.stage_forms[&family].object_lists[&1][115].frame_action_ch[0];
        assert!(fa.cmd_name.is_empty());
        assert_eq!(fa.end_time, 150);
        assert_eq!(fa.counter.get_count(), 0);
    }
    #[test]
    fn malformed_restart_does_not_lose_the_active_callback_or_queue_a_finish() {
        let mut ctx = CommandContext::new(".".into());
        let root = [
            codes::ELM_GLOBAL_FRONT,
            codes::ELM_STAGE_OBJECT,
            -1,
            0,
            codes::ELM_OBJECT_FRAME_ACTION,
        ];
        let start = [root.as_slice(), &[codes::ELM_FRAMEACTION_START]].concat();
        call(
            &mut ctx,
            &start,
            &[Value::Int(100), Value::Str("active".into())],
        );
        let before = bincode::serialize(&ctx.globals.stage_forms).unwrap();
        assert!(super::super::stage::dispatch(&mut ctx, &[Value::Int(200)]).is_err());
        assert_eq!(
            before,
            bincode::serialize(&ctx.globals.stage_forms).unwrap()
        );
        assert!(ctx.globals.pending_frame_action_finishes.is_empty());
        ctx.vm_call.as_mut().unwrap().element = [
            root.as_slice(),
            &[codes::ELM_FRAMEACTION_COUNTER, codes::ELM_COUNTER_WAIT],
        ]
        .concat();
        assert!(super::super::stage::dispatch(&mut ctx, &[Value::Int(500)]).is_err());
    }
}

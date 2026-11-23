// Native policy regression tests exercise the original SCRIPT dispatcher.
use super::codes::script_op as op;
use crate::runtime::{CommandContext, Value, VmCallMeta};
use anyhow::Result;
fn dispatch(
    ctx: &mut CommandContext,
    command: i32,
    args: &[Value],
    overload: i64,
) -> Result<Option<Option<Value>>> {
    if !super::script::signature(command).is_some_and(|(_, _, ready, _)| ready) {
        return Ok(None);
    }
    let previous = ctx.vm_call.replace(VmCallMeta {
        element: vec![ctx.ids.form_global_script as i32, command],
        al_id: overload,
        ret_form: 0,
    });
    let len = ctx.stack.len();
    let result = super::script::dispatch(ctx, args);
    ctx.vm_call = previous;
    result?;
    ctx.stack.truncate(len);
    Ok(Some(None))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn set(ctx: &mut CommandContext, command: i32, value: i64) {
        assert!(
            dispatch(ctx, command, &[Value::Int(value)], 0)
                .unwrap()
                .is_some()
        );
    }
    #[test]
    fn counter_stop_flags_pause_shared_timelines_without_losing_resume_values() {
        let mut ctx = CommandContext::new("asset".into());
        let mut counter = crysirror_scene::timeline_counter::Counter::default();
        counter.start();
        ctx.globals.counter_lists.insert(40, vec![counter]);
        ctx.globals.tick_frame(100, 100, &[], None);
        assert_eq!(ctx.globals.counter_lists[&40][0].get_count(), 100);
        set(&mut ctx, op::SET_COUNTER_TIME_STOP_FLAG, 1);
        ctx.globals.tick_frame(500, 500, &[], None);
        assert_eq!(ctx.globals.counter_lists[&40][0].get_count(), 100);
        set(&mut ctx, op::SET_COUNTER_TIME_STOP_FLAG, 0);
        set(&mut ctx, op::SET_TIME_STOP_FLAG, 1);
        ctx.globals.tick_frame(700, 700, &[], None);
        assert_eq!(ctx.globals.counter_lists[&40][0].get_count(), 100);
        set(&mut ctx, op::SET_TIME_STOP_FLAG, 0);
        ctx.globals.tick_frame(80, 80, &[], None);
        assert_eq!(ctx.globals.counter_lists[&40][0].get_count(), 180);
    }
    #[test]
    fn key_policy_filters_shared_input_queries_without_fabricating_release_edges() {
        let mut ctx = CommandContext::new("asset".into());
        ctx.input.on_key_down(crate::runtime::input::VmKey::Enter);
        ctx.sync_script_input_from_runtime();
        let down = i64::from(ctx.ids.key_op_is_down);
        let up = i64::from(ctx.ids.key_op_is_up);
        assert_eq!(super::super::key::query(&mut ctx, 13, down), 1);
        set(&mut ctx, op::SET_KEY_DISABLE, 13);
        assert_eq!(super::super::key::query(&mut ctx, 13, down), 0);
        assert_eq!(super::super::key::query(&mut ctx, 13, up), 1);
        set(&mut ctx, op::SET_KEY_ENABLE, 13);
        assert_eq!(super::super::key::query(&mut ctx, 13, down), 1);
        assert!(ctx.input.vk_is_down(13));
    }
    #[test]
    fn malformed_policy_calls_fail_before_mutating_native_state() {
        let mut ctx = CommandContext::new("asset".into());
        assert!(dispatch(&mut ctx, op::SET_MSG_BACK_OFF, &[Value::Int(1)], 0).is_err());
        assert!(!ctx.globals.script.msg_back_off);
        assert!(dispatch(&mut ctx, op::SET_MESSAGE_SPEED, &[Value::Int(i64::MAX)], 0).is_err());
        assert_eq!(ctx.globals.script.msg_speed, -1);
        assert!(
            dispatch(
                &mut ctx,
                op::SET_MESSAGE_NOWAIT_FLAG,
                &[Value::Str("1".into())],
                0
            )
            .is_err()
        );
        assert!(!ctx.globals.script.msg_nowait);
        assert!(
            dispatch(&mut ctx, op::IGNORE_R_ON, &[], 0)
                .unwrap()
                .is_some()
        );
    }
}

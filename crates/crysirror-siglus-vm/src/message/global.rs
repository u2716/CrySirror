// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original GLOBAL aliases for the current native message window. All text,
//! input, reveal and animation work still goes through the shared MWND bridge.
use super::*;
use crate::runtime::{Value, forms::codes};
fn target(op: i32) -> Option<i32> {
    match op {
        codes::ELM_GLOBAL_OPEN => Some(codes::ELM_MWND_OPEN),
        codes::ELM_GLOBAL_OPEN_WAIT => Some(codes::ELM_MWND_OPEN_WAIT),
        codes::ELM_GLOBAL_OPEN_NOWAIT => Some(codes::ELM_MWND_OPEN_NOWAIT),
        codes::ELM_GLOBAL_CLOSE => Some(codes::ELM_MWND_CLOSE),
        codes::ELM_GLOBAL_CLOSE_WAIT => Some(codes::ELM_MWND_CLOSE_WAIT),
        codes::ELM_GLOBAL_CLOSE_NOWAIT => Some(codes::ELM_MWND_CLOSE_NOWAIT),
        codes::ELM_GLOBAL_END_CLOSE => Some(codes::ELM_MWND_END_CLOSE),
        codes::ELM_GLOBAL_MSG_BLOCK => Some(codes::ELM_MWND_MSG_BLOCK),
        codes::ELM_GLOBAL_MSG_PP_BLOCK => Some(codes::ELM_MWND_MSG_PP_BLOCK),
        codes::ELM_GLOBAL_CLEAR => Some(codes::ELM_MWND_CLEAR),
        codes::ELM_GLOBAL_PRINT => Some(codes::ELM_MWND_PRINT),
        codes::ELM_GLOBAL_NL => Some(codes::ELM_MWND_NL),
        codes::ELM_GLOBAL_NLI => Some(codes::ELM_MWND_NLI),
        codes::ELM_GLOBAL_WAIT_MSG => Some(codes::ELM_MWND_WAIT_MSG),
        codes::ELM_GLOBAL_PP => Some(codes::ELM_MWND_PP),
        codes::ELM_GLOBAL_R => Some(codes::ELM_MWND_R),
        codes::ELM_GLOBAL_PAGE => Some(codes::ELM_MWND_PAGE),
        codes::ELM_GLOBAL_SET_NAMAE => Some(codes::ELM_MWND_SET_NAMAE),
        codes::ELM_GLOBAL_NEXT_MSG => Some(codes::ELM_MWND_NEXT_MSG),
        codes::ELM_GLOBAL_MULTI_MSG => Some(codes::ELM_MWND_MULTI_MSG),
        codes::ELM_GLOBAL_RUBY => Some(codes::ELM_MWND_RUBY),
        codes::ELM_GLOBAL_KOE => Some(codes::ELM_MWND_KOE),
        codes::ELM_GLOBAL_KOE_PLAY_WAIT => Some(codes::ELM_MWND_KOE_PLAY_WAIT),
        codes::ELM_GLOBAL_KOE_PLAY_WAIT_KEY => Some(codes::ELM_MWND_KOE_PLAY_WAIT_KEY),
        codes::ELM_GLOBAL_SIZE => Some(codes::ELM_MWND_SIZE),
        codes::ELM_GLOBAL_COLOR => Some(codes::ELM_MWND_COLOR),
        codes::ELM_GLOBAL_INDENT => Some(codes::ELM_MWND_INDENT),
        codes::ELM_GLOBAL_CLEAR_INDENT => Some(codes::ELM_MWND_CLEAR_INDENT),
        codes::ELM_GLOBAL_START_SLIDE_MSG => Some(codes::ELM_MWND_START_SLIDE_MSG),
        codes::ELM_GLOBAL_END_SLIDE_MSG => Some(codes::ELM_MWND_END_SLIDE_MSG),
        codes::ELM_GLOBAL_SET_WAKU => Some(codes::ELM_MWND_SET_WAKU),
        codes::ELM_GLOBAL_CLEAR_FACE => Some(codes::ELM_MWND_CLEAR_FACE),
        codes::ELM_GLOBAL_SET_FACE => Some(codes::ELM_MWND_SET_FACE),
        codes::ELM_GLOBAL_REP_POS => Some(codes::ELM_MWND_REP_POS),
        codes::ELM_GLOBAL_MSGBTN => Some(codes::ELM_MWND_MSGBTN),
        _ => None,
    }
}

pub(crate) fn dispatch(ctx: &mut CommandContext, form: u32, args: &[Value]) -> Result<bool> {
    let Some(op) = target(form as i32) else {
        return Ok(false);
    };
    let call = ctx
        .vm_call
        .as_ref()
        .context("native global message metadata")?;
    ensure!(
        call.element.as_slice() == [form as i32],
        "native global message chain"
    );
    supported_target(ctx)?;
    // Unsupported MWND operations fail in the common semantic dispatcher;
    // alias registration never substitutes a zero/default return for them.
    ops::dispatch(ctx, op, args)
}

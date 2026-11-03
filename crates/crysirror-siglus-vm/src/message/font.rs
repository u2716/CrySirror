// SPDX-License-Identifier: MPL-2.0
//! Native SCRIPT/EXCALL/config font precedence; resource lookup/metrics belong
//! to the shared client font capability, never a private font cache.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Selection {
    pub requested: String,
    pub alias: String,
    pub fallback: bool,
    pub scoped: bool,
}
fn requested(ctx: &CommandContext) -> String {
    let config = ctx
        .globals
        .syscom
        .config_str
        .get(&crate::runtime::forms::codes::syscom_op::GET_FONT_NAME)
        .map(String::as_str)
        .unwrap_or("");
    let override_name = if ctx.globals.syscom.msg_back_open {
        ""
    } else if ctx.excall_state.ex_call_flag {
        ctx.excall_state.font_name.as_str()
    } else {
        ctx.globals.script.font_name.as_str()
    };
    if override_name.is_empty() {
        config.to_owned()
    } else {
        override_name.to_owned()
    }
}
pub(super) fn select(
    ctx: &CommandContext,
    client: ClientId,
    host: &mut dyn HostServices,
) -> Result<Option<Selection>> {
    let Some(window) = &ctx.ui.window else {
        return Ok(None);
    };
    let requested = requested(ctx);
    if requested.is_empty() || requested == window.font {
        return Ok(Some(Selection {
            requested,
            alias: window.font.clone(),
            fallback: false,
            scoped: false,
        }));
    }
    let library = host.font_library(client)?;
    let found = library.resolve_alias(&requested)?;
    Ok(Some(Selection {
        fallback: found.is_none(),
        alias: found.unwrap_or_else(|| window.font.clone()),
        requested,
        scoped: true,
    }))
}

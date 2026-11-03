// SPDX-License-Identifier: MPL-2.0
//! Complete original SYSCOM font preference group, with capability-only
//! existence queries. Native fields persist; engine Font/Text own resources.
use crate::runtime::{CommandContext, Value, forms::codes::syscom_op as op};
use anyhow::{Context, Result, ensure};
fn integer(ctx: &CommandContext, key: &str, fallback: i64) -> i64 {
    ctx.tables
        .gameexe
        .as_ref()
        .and_then(|cfg| cfg.get_i64(key))
        .unwrap_or(fallback)
}
pub(super) fn defaults(ctx: &CommandContext) -> (String, bool, i64) {
    let name = match integer(ctx, "CONFIG.FONT.TYPE", 0) {
        0 => "ＭＳ ゴシック".into(),
        1 => "ＭＳ 明朝".into(),
        2 => "メイリオ".into(),
        3 => {
            let name = ctx
                .tables
                .gameexe
                .as_ref()
                .and_then(|cfg| cfg.get_unquoted("CONFIG.FONT.NAME"))
                .unwrap_or("");
            if name.is_empty() {
                "ＭＳ ゴシック".into()
            } else {
                name.to_owned()
            }
        }
        _ => "ＭＳ ゴシック".into(),
    };
    (
        name,
        integer(ctx, "CONFIG.FONT.FUTOKU", 0) != 0,
        integer(ctx, "CONFIG.FONT.SHADOW", 2).clamp(0, 3),
    )
}
pub(super) fn initialize(ctx: &mut CommandContext) {
    let (name, bold, shadow) = defaults(ctx);
    let cfg = &mut ctx.globals.syscom;
    cfg.original_config.font_name = name.clone();
    cfg.original_config.font_futoku = bold;
    cfg.original_config.font_shadow = shadow;
    // Explicit caller-authored plain MWND profiles without Gameexe retain their
    // configured face/decoration. Real native configuration is adopted whole.
    if ctx.tables.gameexe.is_some() {
        cfg.config_str.insert(op::GET_FONT_NAME, name);
        cfg.config_int.insert(op::GET_FONT_BOLD, i64::from(bold));
        cfg.config_int.insert(op::GET_FONT_DECORATION, shadow);
    }
}
pub(super) fn dispatch(ctx: &mut CommandContext, args: &[Value]) -> Result<bool> {
    let call = ctx
        .vm_call
        .as_ref()
        .context("native font preference metadata")?;
    let Some(&command) = call.element.get(1) else {
        return Ok(false);
    };
    if !matches!(
        command,
        op::SET_FONT_NAME
            | op::SET_FONT_NAME_DEFAULT
            | op::GET_FONT_NAME
            | op::IS_FONT_EXIST
            | op::SET_FONT_BOLD
            | op::SET_FONT_BOLD_DEFAULT
            | op::GET_FONT_BOLD
            | op::SET_FONT_DECORATION
            | op::SET_FONT_DECORATION_DEFAULT
            | op::GET_FONT_DECORATION
    ) {
        return Ok(false);
    }
    ensure!(
        call.element.len() == 2 && call.al_id == 0,
        "native font preference chain/overload"
    );
    let setter = matches!(
        command,
        op::SET_FONT_NAME | op::IS_FONT_EXIST | op::SET_FONT_BOLD | op::SET_FONT_DECORATION
    );
    ensure!(
        args.len() == usize::from(setter),
        "native font preference argument count"
    );
    let string = if matches!(command, op::SET_FONT_NAME | op::IS_FONT_EXIST) {
        let name = args[0].as_str().context("native font preference string")?;
        ensure!(
            name.len() <= 256 && !name.chars().any(char::is_control),
            "native font preference name budget"
        );
        Some(name.to_owned())
    } else {
        None
    };
    let value = if matches!(command, op::SET_FONT_BOLD | op::SET_FONT_DECORATION) {
        let value = args[0].as_i64().context("native font preference integer")?;
        ensure!(
            i32::try_from(value).is_ok(),
            "native font preference integer width"
        );
        value
    } else {
        0
    };
    let (default_name, default_bold, default_shadow) = defaults(ctx);
    if command == op::IS_FONT_EXIST {
        let exists = crate::io_fs::probe_font(string.as_deref().unwrap())?;
        ctx.push(Value::Int(i64::from(exists)));
        return Ok(true);
    }
    let cfg = &mut ctx.globals.syscom;
    match command {
        op::SET_FONT_NAME | op::SET_FONT_NAME_DEFAULT => {
            let name = if command == op::SET_FONT_NAME {
                string.unwrap()
            } else {
                default_name
            };
            cfg.original_config.font_name = name.clone();
            cfg.config_str.insert(op::GET_FONT_NAME, name);
        }
        op::GET_FONT_NAME => {
            let name = cfg
                .config_str
                .get(&op::GET_FONT_NAME)
                .filter(|n| !n.is_empty())
                .cloned()
                .unwrap_or(default_name);
            ctx.push(Value::Str(name));
            return Ok(true);
        }
        op::SET_FONT_BOLD | op::SET_FONT_BOLD_DEFAULT => {
            let bold = if command == op::SET_FONT_BOLD {
                value != 0
            } else {
                default_bold
            };
            cfg.original_config.font_futoku = bold;
            cfg.config_int.insert(op::GET_FONT_BOLD, i64::from(bold));
        }
        op::GET_FONT_BOLD => {
            let value = cfg.config_int.get(&op::GET_FONT_BOLD).copied().unwrap_or(0);
            ctx.push(Value::Int(value));
            return Ok(true);
        }
        op::SET_FONT_DECORATION | op::SET_FONT_DECORATION_DEFAULT => {
            let shadow = if command == op::SET_FONT_DECORATION {
                value.clamp(0, 3)
            } else {
                default_shadow
            };
            cfg.original_config.font_shadow = shadow;
            cfg.config_int.insert(op::GET_FONT_DECORATION, shadow);
        }
        op::GET_FONT_DECORATION => {
            let value = cfg
                .config_int
                .get(&op::GET_FONT_DECORATION)
                .copied()
                .unwrap_or(2);
            ctx.push(Value::Int(value));
            return Ok(true);
        }
        _ => unreachable!(),
    }
    ctx.push(Value::Int(0));
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{VmCallMeta,forms::codes};
    fn call(ctx:&mut CommandContext,operation:i32,args:&[Value])->Option<Value> {
        ctx.vm_call=Some(VmCallMeta{element:vec![codes::ELM_GLOBAL_SYSCOM,operation],al_id:0,ret_form:0});
        assert!(dispatch(ctx,args).unwrap());ctx.pop()
    }
    #[test]
    fn full_native_font_preferences_preserve_type_defaults_clamps_and_raw_script_scope() {
        let mut ctx=CommandContext::new(".".into());
        ctx.tables.gameexe=Some(std::sync::Arc::new(crate::gameexe::GameexeConfig::from_text(
            "#CONFIG.FONT.TYPE=3\n#CONFIG.FONT.NAME=\"Authored Family\"\n#CONFIG.FONT.FUTOKU=7\n#CONFIG.FONT.SHADOW=99\n")));
        initialize(&mut ctx);
        assert_eq!(call(&mut ctx,op::GET_FONT_NAME,&[]).and_then(|v|v.as_str().map(str::to_owned)),Some("Authored Family".into()));
        assert_eq!(call(&mut ctx,op::GET_FONT_BOLD,&[]).and_then(|v|v.as_i64()),Some(1));
        assert_eq!(call(&mut ctx,op::GET_FONT_DECORATION,&[]).and_then(|v|v.as_i64()),Some(3));
        call(&mut ctx,op::SET_FONT_NAME,&[Value::Str(String::new())]);
        assert!(ctx.globals.syscom.original_config.font_name.is_empty());
        assert_eq!(call(&mut ctx,op::GET_FONT_NAME,&[]).and_then(|v|v.as_str().map(str::to_owned)),Some("Authored Family".into()));
        call(&mut ctx,op::SET_FONT_DECORATION,&[Value::Int(-7)]);
        assert_eq!(ctx.globals.syscom.original_config.font_shadow,0);
        call(&mut ctx,op::SET_FONT_BOLD,&[Value::Int(-1)]);
        assert!(ctx.globals.syscom.original_config.font_futoku);
        call(&mut ctx,op::SET_FONT_NAME_DEFAULT,&[]);
        assert_eq!(ctx.globals.syscom.original_config.font_name,"Authored Family");
        call(&mut ctx,op::SET_FONT_DECORATION_DEFAULT,&[]);
        assert_eq!(ctx.globals.syscom.original_config.font_shadow,3);
        ctx.vm_call=Some(VmCallMeta{element:vec![codes::ELM_GLOBAL_SYSCOM,op::SET_FONT_DECORATION],al_id:0,ret_form:0});
        assert!(dispatch(&mut ctx,&[Value::Int(i64::MAX)]).is_err());
        assert_eq!(ctx.globals.syscom.original_config.font_shadow,3);
        assert_eq!(ctx.globals.script.font_shadow,-1);
        for (mode,name) in [(0,"ＭＳ ゴシック"),(1,"ＭＳ 明朝"),(2,"メイリオ"),(8,"ＭＳ ゴシック")] {
            ctx.tables.gameexe=Some(std::sync::Arc::new(crate::gameexe::GameexeConfig::from_text(&format!("#CONFIG.FONT.TYPE={mode}"))));
            assert_eq!(defaults(&ctx).0,name);
        }
    }
}

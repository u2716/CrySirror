// SPDX-License-Identifier: MPL-2.0
// Whole sound preference family adopted from xmoezzz/siglus_rs
// 5f358ef1360b0439e92aed2b749e8f25d8721954 runtime/forms/syscom.rs.
//! Native saved category/volume/duck settings; mechanisms live in shared Audio.
use crate::runtime::{CommandContext, Value, forms::codes::syscom_op::*};
use anyhow::{Context, Result, ensure};
fn cfg_get_int(st: &crate::runtime::globals::SyscomRuntimeState, key: i32, default: i64) -> i64 {
    st.config_int.get(&key).copied().unwrap_or(default)
}

fn cfg_set_int(st: &mut crate::runtime::globals::SyscomRuntimeState, key: i32, value: i64) {
    st.config_int.insert(key, value);
}

fn volume_to_raw(v: i64) -> u8 {
    v.clamp(0, 255) as u8
}

fn standard_sound_volume_key(sound_type: usize) -> Option<i32> {
    match sound_type {
        0 => Some(GET_BGM_VOLUME),
        1 => Some(GET_KOE_VOLUME),
        2 => Some(GET_PCM_VOLUME),
        3 => Some(GET_SE_VOLUME),
        4 => Some(GET_MOV_VOLUME),
        _ => None,
    }
}

fn standard_sound_onoff_key(sound_type: usize) -> Option<i32> {
    match sound_type {
        0 => Some(GET_BGM_ONOFF),
        1 => Some(GET_KOE_ONOFF),
        2 => Some(GET_PCM_ONOFF),
        3 => Some(GET_SE_ONOFF),
        4 => Some(GET_MOV_ONOFF),
        _ => None,
    }
}

fn set_sound_volume_by_type(ctx: &mut CommandContext, sound_type: usize, value: i64) {
    if sound_type >= ctx.globals.syscom.original_config.sound_user_volume.len() {
        return;
    }
    let value = value.clamp(0, 255);
    ctx.globals.syscom.original_config.sound_user_volume[sound_type] = value;
    if let Some(key) = standard_sound_volume_key(sound_type) {
        cfg_set_int(&mut ctx.globals.syscom, key, value);
    }
}

fn get_sound_volume_by_type(ctx: &CommandContext, sound_type: usize) -> i64 {
    if sound_type >= ctx.globals.syscom.original_config.sound_user_volume.len() {
        return 255;
    }
    standard_sound_volume_key(sound_type)
        .map(|key| {
            cfg_get_int(
                &ctx.globals.syscom,
                key,
                ctx.globals.syscom.original_config.sound_user_volume[sound_type],
            )
        })
        .unwrap_or(ctx.globals.syscom.original_config.sound_user_volume[sound_type])
        .clamp(0, 255)
}

fn set_sound_onoff_by_type(ctx: &mut CommandContext, sound_type: usize, value: bool) {
    if sound_type >= ctx.globals.syscom.original_config.play_sound_check.len() {
        return;
    }
    ctx.globals.syscom.original_config.play_sound_check[sound_type] = value;
    if let Some(key) = standard_sound_onoff_key(sound_type) {
        cfg_set_int(&mut ctx.globals.syscom, key, if value { 1 } else { 0 });
    }
}

fn get_sound_onoff_by_type(ctx: &CommandContext, sound_type: usize) -> bool {
    if sound_type >= ctx.globals.syscom.original_config.play_sound_check.len() {
        return true;
    }
    standard_sound_onoff_key(sound_type)
        .map(|key| {
            cfg_get_int(
                &ctx.globals.syscom,
                key,
                if ctx.globals.syscom.original_config.play_sound_check[sound_type] {
                    1
                } else {
                    0
                },
            ) != 0
        })
        .unwrap_or(ctx.globals.syscom.original_config.play_sound_check[sound_type])
}

fn config_default_sound_volume(ctx: &CommandContext, sound_type: usize) -> i64 {
    let key = match sound_type {
        0 => Some("CONFIG.VOLUME.BGM"),
        1 => Some("CONFIG.VOLUME.KOE"),
        2 => Some("CONFIG.VOLUME.PCM"),
        3 => Some("CONFIG.VOLUME.SE"),
        4 => Some("CONFIG.VOLUME.MOV"),
        _ => None,
    };
    key.map(|key| gameexe_i64_or(ctx, key, 255))
        .unwrap_or(255)
        .clamp(0, 255)
}

fn config_default_sound_onoff(_ctx: &CommandContext, _sound_type: usize) -> bool {
    // The original Gameexe parser initializes every sound channel to enabled;
    // there is no #CONFIG directive that overrides these five checks.
    true
}

fn gameexe_i64_or(ctx: &CommandContext, key: &str, default: i64) -> i64 {
    ctx.tables
        .gameexe
        .as_ref()
        .and_then(|cfg| cfg.get_i64(key))
        .unwrap_or(default)
}

fn gameexe_bool_or(ctx: &CommandContext, key: &str, fallback: bool) -> bool {
    gameexe_i64_or(ctx, key, i64::from(fallback)) != 0
}
fn p_i64(args: &[Value], index: usize) -> i64 {
    args.get(index).and_then(Value::as_i64).unwrap_or(0)
}
fn p_bool(args: &[Value], index: usize) -> bool {
    p_i64(args, index) != 0
}
pub(crate) fn initialize(ctx: &mut CommandContext) {
    let all = gameexe_i64_or(ctx, "CONFIG.VOLUME.ALL", 255).clamp(0, 255);
    ctx.globals.syscom.original_config.all_sound_user_volume = all;
    ctx.globals.syscom.original_config.play_all_sound_check = true;
    cfg_set_int(&mut ctx.globals.syscom, GET_ALL_VOLUME, all);
    cfg_set_int(&mut ctx.globals.syscom, GET_ALL_ONOFF, 1);
    for index in 0..32 {
        let value = config_default_sound_volume(ctx, index);
        set_sound_volume_by_type(ctx, index, value);
        set_sound_onoff_by_type(ctx, index, true);
    }
    let volume = gameexe_i64_or(ctx, "CONFIG.BGMFADE_VOLUME", 192).clamp(0, 255);
    let enabled = gameexe_bool_or(ctx, "CONFIG.BGMFADE_ONOFF", true);
    ctx.globals.syscom.original_config.bgmfade_volume = volume;
    ctx.globals.syscom.original_config.bgmfade_use_check = enabled;
    cfg_set_int(&mut ctx.globals.syscom, GET_BGMFADE_VOLUME, volume);
    cfg_set_int(
        &mut ctx.globals.syscom,
        GET_BGMFADE_ONOFF,
        i64::from(enabled),
    );
    cfg_set_int(&mut ctx.globals.syscom, GET_KOEMODE, 0);
}
pub(crate) fn dispatch(ctx: &mut CommandContext, params: &[Value]) -> Result<bool> {
    let call = ctx
        .vm_call
        .as_ref()
        .context("native sound configuration metadata")?;
    let op = *call
        .element
        .get(1)
        .context("native sound configuration opcode")?;
    let arity = match op {
        SET_ALL_VOLUME => Some(1),
        SET_BGM_VOLUME => Some(1),
        SET_KOE_VOLUME => Some(1),
        SET_PCM_VOLUME => Some(1),
        SET_SE_VOLUME => Some(1),
        SET_MOV_VOLUME => Some(1),
        SET_SOUND_VOLUME => Some(2),
        SET_ALL_VOLUME_DEFAULT => Some(0),
        SET_BGM_VOLUME_DEFAULT => Some(0),
        SET_KOE_VOLUME_DEFAULT => Some(0),
        SET_PCM_VOLUME_DEFAULT => Some(0),
        SET_SE_VOLUME_DEFAULT => Some(0),
        SET_MOV_VOLUME_DEFAULT => Some(0),
        SET_SOUND_VOLUME_DEFAULT => Some(1),
        GET_MOV_VOLUME => Some(0),
        GET_SOUND_VOLUME => Some(1),
        SET_ALL_ONOFF => Some(1),
        SET_BGM_ONOFF => Some(1),
        SET_KOE_ONOFF => Some(1),
        SET_PCM_ONOFF => Some(1),
        SET_SE_ONOFF => Some(1),
        SET_MOV_ONOFF => Some(1),
        SET_SOUND_ONOFF => Some(2),
        SET_ALL_ONOFF_DEFAULT => Some(0),
        SET_BGM_ONOFF_DEFAULT => Some(0),
        SET_KOE_ONOFF_DEFAULT => Some(0),
        SET_PCM_ONOFF_DEFAULT => Some(0),
        SET_SE_ONOFF_DEFAULT => Some(0),
        SET_MOV_ONOFF_DEFAULT => Some(0),
        SET_SOUND_ONOFF_DEFAULT => Some(1),
        GET_MOV_ONOFF => Some(0),
        GET_SOUND_ONOFF => Some(1),
        SET_BGMFADE_VOLUME => Some(1),
        SET_BGMFADE_ONOFF => Some(1),
        SET_BGMFADE_VOLUME_DEFAULT => Some(0),
        SET_BGMFADE_ONOFF_DEFAULT => Some(0),
        GET_BGMFADE_VOLUME => Some(0),
        GET_BGMFADE_ONOFF => Some(0),
        SET_KOEMODE => Some(1),
        SET_KOEMODE_DEFAULT => Some(0),
        GET_KOEMODE => Some(0),
        GET_ALL_VOLUME => Some(0),
        GET_BGM_VOLUME => Some(0),
        GET_KOE_VOLUME => Some(0),
        GET_PCM_VOLUME => Some(0),
        GET_SE_VOLUME => Some(0),
        GET_ALL_ONOFF => Some(0),
        GET_BGM_ONOFF => Some(0),
        GET_KOE_ONOFF => Some(0),
        GET_PCM_ONOFF => Some(0),
        GET_SE_ONOFF => Some(0),
        _ => None,
    };
    let Some(arity) = arity else {
        return Ok(false);
    };
    ensure!(
        call.element.len() == 2 && call.al_id == 0 && params.len() == arity,
        "native sound configuration ABI"
    );
    ensure!(
        params
            .iter()
            .all(|value| value.as_i64().is_some_and(|v| i32::try_from(v).is_ok())),
        "native sound configuration integer width/type"
    );
    match op {
        SET_ALL_VOLUME => {
            let value = p_i64(params, 0).clamp(0, 255);
            ctx.globals.syscom.original_config.all_sound_user_volume = value;
            cfg_set_int(&mut ctx.globals.syscom, GET_ALL_VOLUME, value);
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_BGM_VOLUME => {
            set_sound_volume_by_type(ctx, 0, p_i64(params, 0));
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_KOE_VOLUME => {
            set_sound_volume_by_type(ctx, 1, p_i64(params, 0));
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_PCM_VOLUME => {
            set_sound_volume_by_type(ctx, 2, p_i64(params, 0));
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_SE_VOLUME => {
            set_sound_volume_by_type(ctx, 3, p_i64(params, 0));
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_MOV_VOLUME => {
            set_sound_volume_by_type(ctx, 4, p_i64(params, 0));
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_SOUND_VOLUME => {
            let sound_type = p_i64(params, 0);
            if (0..32).contains(&sound_type) {
                set_sound_volume_by_type(ctx, sound_type as usize, p_i64(params, 1));
                crate::native_sound::update_audio_routing(ctx, 0, true)?;
            }
        }
        SET_ALL_VOLUME_DEFAULT => {
            let value = gameexe_i64_or(ctx, "CONFIG.VOLUME.ALL", 255).clamp(0, 255);
            ctx.globals.syscom.original_config.all_sound_user_volume = value;
            cfg_set_int(&mut ctx.globals.syscom, GET_ALL_VOLUME, value);
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_BGM_VOLUME_DEFAULT => {
            let value = config_default_sound_volume(ctx, 0);
            set_sound_volume_by_type(ctx, 0, value);
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_KOE_VOLUME_DEFAULT => {
            let value = config_default_sound_volume(ctx, 1);
            set_sound_volume_by_type(ctx, 1, value);
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_PCM_VOLUME_DEFAULT => {
            let value = config_default_sound_volume(ctx, 2);
            set_sound_volume_by_type(ctx, 2, value);
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_SE_VOLUME_DEFAULT => {
            let value = config_default_sound_volume(ctx, 3);
            set_sound_volume_by_type(ctx, 3, value);
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_MOV_VOLUME_DEFAULT => {
            let value = config_default_sound_volume(ctx, 4);
            set_sound_volume_by_type(ctx, 4, value);
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_SOUND_VOLUME_DEFAULT => {
            let sound_type = p_i64(params, 0);
            if (0..32).contains(&sound_type) {
                let value = config_default_sound_volume(ctx, sound_type as usize);
                set_sound_volume_by_type(ctx, sound_type as usize, value);
                crate::native_sound::update_audio_routing(ctx, 0, true)?;
            }
        }
        GET_ALL_VOLUME | GET_BGM_VOLUME | GET_KOE_VOLUME | GET_PCM_VOLUME | GET_SE_VOLUME
        | GET_MOV_VOLUME => {
            let v = cfg_get_int(&ctx.globals.syscom, op, 255).clamp(0, 255);
            ctx.push(Value::Int(v));
            return Ok(true);
        }
        GET_SOUND_VOLUME => {
            let sound_type = p_i64(params, 0);
            let value = if (0..32).contains(&sound_type) {
                get_sound_volume_by_type(ctx, sound_type as usize)
            } else {
                255
            };
            ctx.push(Value::Int(value));
            return Ok(true);
        }
        SET_ALL_ONOFF => {
            let value = p_bool(params, 0);
            ctx.globals.syscom.original_config.play_all_sound_check = value;
            cfg_set_int(
                &mut ctx.globals.syscom,
                GET_ALL_ONOFF,
                if value { 1 } else { 0 },
            );
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_BGM_ONOFF => {
            set_sound_onoff_by_type(ctx, 0, p_bool(params, 0));
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_KOE_ONOFF => {
            set_sound_onoff_by_type(ctx, 1, p_bool(params, 0));
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_PCM_ONOFF => {
            set_sound_onoff_by_type(ctx, 2, p_bool(params, 0));
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_SE_ONOFF => {
            set_sound_onoff_by_type(ctx, 3, p_bool(params, 0));
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_MOV_ONOFF => {
            set_sound_onoff_by_type(ctx, 4, p_bool(params, 0));
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_SOUND_ONOFF => {
            let sound_type = p_i64(params, 0);
            if (0..32).contains(&sound_type) {
                set_sound_onoff_by_type(ctx, sound_type as usize, p_bool(params, 1));
                crate::native_sound::update_audio_routing(ctx, 0, true)?;
            }
        }
        SET_ALL_ONOFF_DEFAULT => {
            ctx.globals.syscom.original_config.play_all_sound_check = true;
            cfg_set_int(&mut ctx.globals.syscom, GET_ALL_ONOFF, 1);
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_BGM_ONOFF_DEFAULT => {
            let value = config_default_sound_onoff(ctx, 0);
            set_sound_onoff_by_type(ctx, 0, value);
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_KOE_ONOFF_DEFAULT => {
            let value = config_default_sound_onoff(ctx, 1);
            set_sound_onoff_by_type(ctx, 1, value);
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_PCM_ONOFF_DEFAULT => {
            let value = config_default_sound_onoff(ctx, 2);
            set_sound_onoff_by_type(ctx, 2, value);
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_SE_ONOFF_DEFAULT => {
            let value = config_default_sound_onoff(ctx, 3);
            set_sound_onoff_by_type(ctx, 3, value);
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_MOV_ONOFF_DEFAULT => {
            let value = config_default_sound_onoff(ctx, 4);
            set_sound_onoff_by_type(ctx, 4, value);
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_SOUND_ONOFF_DEFAULT => {
            let sound_type = p_i64(params, 0);
            if (0..32).contains(&sound_type) {
                let value = config_default_sound_onoff(ctx, sound_type as usize);
                set_sound_onoff_by_type(ctx, sound_type as usize, value);
                crate::native_sound::update_audio_routing(ctx, 0, true)?;
            }
        }
        GET_ALL_ONOFF | GET_BGM_ONOFF | GET_KOE_ONOFF | GET_PCM_ONOFF | GET_SE_ONOFF
        | GET_MOV_ONOFF => {
            let v = cfg_get_int(&ctx.globals.syscom, op, 1);
            ctx.push(Value::Int(if v != 0 { 1 } else { 0 }));
            return Ok(true);
        }
        GET_SOUND_ONOFF => {
            let sound_type = p_i64(params, 0);
            let value = if (0..32).contains(&sound_type) {
                get_sound_onoff_by_type(ctx, sound_type as usize)
            } else {
                true
            };
            ctx.push(Value::Int(if value { 1 } else { 0 }));
            return Ok(true);
        }
        SET_BGMFADE_VOLUME => {
            let value = p_i64(params, 0).clamp(0, 255);
            ctx.globals.syscom.original_config.bgmfade_volume = value;
            cfg_set_int(&mut ctx.globals.syscom, GET_BGMFADE_VOLUME, value);
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_BGMFADE_ONOFF => {
            let value = p_bool(params, 0);
            ctx.globals.syscom.original_config.bgmfade_use_check = value;
            cfg_set_int(
                &mut ctx.globals.syscom,
                GET_BGMFADE_ONOFF,
                if value { 1 } else { 0 },
            );
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_BGMFADE_VOLUME_DEFAULT => {
            let value = gameexe_i64_or(ctx, "CONFIG.BGMFADE_VOLUME", 192).clamp(0, 255);
            ctx.globals.syscom.original_config.bgmfade_volume = value;
            cfg_set_int(&mut ctx.globals.syscom, GET_BGMFADE_VOLUME, value);
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_BGMFADE_ONOFF_DEFAULT => {
            let value = gameexe_bool_or(ctx, "CONFIG.BGMFADE_ONOFF", true);
            ctx.globals.syscom.original_config.bgmfade_use_check = value;
            cfg_set_int(
                &mut ctx.globals.syscom,
                GET_BGMFADE_ONOFF,
                if value { 1 } else { 0 },
            );
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        GET_BGMFADE_VOLUME | GET_BGMFADE_ONOFF => {
            let default = if op == GET_BGMFADE_ONOFF { 1 } else { 192 };
            let v = cfg_get_int(&ctx.globals.syscom, op, default);
            ctx.push(Value::Int(v));
            return Ok(true);
        }
        SET_KOEMODE => {
            let value = match p_i64(params, 0) {
                0..=2 => p_i64(params, 0),
                _ => 0,
            };
            ctx.globals.syscom.original_config.koe_mode = value;
            cfg_set_int(&mut ctx.globals.syscom, GET_KOEMODE, value);
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        SET_KOEMODE_DEFAULT => {
            ctx.globals.syscom.original_config.koe_mode = 0;
            cfg_set_int(&mut ctx.globals.syscom, GET_KOEMODE, 0);
            crate::native_sound::update_audio_routing(ctx, 0, true)?;
        }
        GET_KOEMODE => {
            let v = cfg_get_int(&ctx.globals.syscom, GET_KOEMODE, 0);
            ctx.push(Value::Int(v));
            return Ok(true);
        }
        _ => unreachable!(),
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::VmCallMeta;
    fn call(ctx: &mut CommandContext, op: i32, args: &[Value]) -> Result<bool> {
        ctx.vm_call = Some(VmCallMeta {
            element: vec![63, op],
            ..Default::default()
        });
        dispatch(ctx, args)
    }
    #[test]
    fn whole_sound_defaults_and_indexed_categories_follow_original_records() {
        let mut ctx = CommandContext::new("asset".into());
        ctx.tables.gameexe = Some(std::sync::Arc::new(
            crate::gameexe::GameexeConfig::from_text(
                "#CONFIG.VOLUME.ALL=201\n#CONFIG.VOLUME.BGM=193\n#CONFIG.BGMFADE_VOLUME=129\n#CONFIG.BGMFADE_ONOFF=0\n",
            ),
        ));
        initialize(&mut ctx);
        assert_eq!(
            ctx.globals.syscom.original_config.all_sound_user_volume,
            201
        );
        assert_eq!(ctx.globals.syscom.original_config.sound_user_volume[0], 193);
        assert!(!ctx.globals.syscom.original_config.bgmfade_use_check);
        call(
            &mut ctx,
            SET_SOUND_VOLUME,
            &[Value::Int(17), Value::Int(300)],
        )
        .unwrap();
        assert_eq!(
            ctx.globals.syscom.original_config.sound_user_volume[17],
            255
        );
        call(&mut ctx, SET_SOUND_ONOFF, &[Value::Int(17), Value::Int(0)]).unwrap();
        assert!(!ctx.globals.syscom.original_config.play_sound_check[17]);
        call(&mut ctx, SET_SOUND_ONOFF_DEFAULT, &[Value::Int(17)]).unwrap();
        assert!(ctx.globals.syscom.original_config.play_sound_check[17]);
        call(&mut ctx, SET_ALL_VOLUME, &[Value::Int(-3)]).unwrap();
        call(&mut ctx, GET_ALL_VOLUME, &[]).unwrap();
        assert!(matches!(ctx.pop(), Some(Value::Int(0))));
        call(&mut ctx, SET_ALL_VOLUME_DEFAULT, &[]).unwrap();
        call(&mut ctx, GET_ALL_VOLUME, &[]).unwrap();
        assert!(matches!(ctx.pop(), Some(Value::Int(201))));
        call(&mut ctx, GET_SOUND_VOLUME, &[Value::Int(32)]).unwrap();
        assert!(matches!(ctx.pop(), Some(Value::Int(255))));
        call(&mut ctx, GET_SOUND_ONOFF, &[Value::Int(-1)]).unwrap();
        assert!(matches!(ctx.pop(), Some(Value::Int(1))));
        assert!(
            ctx.audio.pending.is_empty(),
            "preferences without a source do not fabricate playback"
        );
    }
    #[test]
    fn malformed_native_sound_calls_fail_before_changing_configuration() {
        let mut ctx = CommandContext::new("asset".into());
        initialize(&mut ctx);
        assert!(call(&mut ctx, SET_ALL_VOLUME, &[Value::Int(i64::MAX)]).is_err());
        assert!(call(&mut ctx, SET_SOUND_VOLUME, &[Value::Int(0)]).is_err());
        assert!(call(&mut ctx, SET_BGM_ONOFF, &[Value::Str("1".into())]).is_err());
        assert_eq!(
            ctx.globals.syscom.original_config.all_sound_user_volume,
            255
        );
        assert_eq!(ctx.globals.syscom.original_config.sound_user_volume[0], 255);
        assert!(ctx.globals.syscom.original_config.play_sound_check[0]);
    }
}

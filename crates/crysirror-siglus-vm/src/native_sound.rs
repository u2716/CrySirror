// SPDX-License-Identifier: MPL-2.0
// Adopted as a whole policy group from xmoezzz/siglus_rs
// 5f358ef1360b0439e92aed2b749e8f25d8721954 runtime/forms/syscom.rs.
// See UPSTREAM.md, SOURCE.json and upstream-files.json for provenance.
//! Original integer category/character/PCMCH/two-level BGM routing policy.
//! Scene owns integer curve evaluation; shared Audio owns actual group gains.
use crate::runtime::CommandContext;
use anyhow::Result;
fn mul_raw(lhs: i64, rhs: i64) -> i64 {
    lhs.clamp(0, 255) * rhs.clamp(0, 255) / 255
}

fn linear_limit_i64(time: i64, start_time: i64, start: i64, end_time: i64, end: i64) -> i64 {
    crysirror_scene::animation::sample_integer_line(time, start_time, start, end_time, end)
}
fn parse_i64_list_local(raw: &str) -> Vec<i64> {
    raw.split(|c: char| c == ',' || c.is_whitespace())
        .filter_map(|part| {
            let t = part.trim();
            if t.is_empty() {
                None
            } else {
                t.parse::<i64>().ok()
            }
        })
        .collect()
}

fn gameexe_i64_or(ctx: &CommandContext, key: &str, default: i64) -> i64 {
    ctx.tables
        .gameexe
        .as_ref()
        .and_then(|cfg| cfg.get_i64(key))
        .unwrap_or(default)
}

fn chrkoe_chara_numbers(ctx: &CommandContext, index: usize) -> Vec<i64> {
    let Some(entry) = ctx
        .tables
        .gameexe
        .as_ref()
        .and_then(|cfg| cfg.get_indexed_entry("CHRKOE", index))
    else {
        return Vec::new();
    };
    // #CHRKOE.i = name, check_mode, check_name, onoff, volume, (chara...)
    // `value_items` is intentionally a CSV-like view and does not preserve the
    // parenthesized list as one token, so recover the list from the raw value.
    let Some(open) = entry.value.find('(') else {
        return Vec::new();
    };
    let close = entry.value.rfind(')').unwrap_or(entry.value.len());
    if close <= open {
        return Vec::new();
    }
    parse_i64_list_local(&entry.value[open + 1..close])
}

fn chrkoe_route(
    config: &crate::runtime::globals::OriginalConfigRuntimeState,
    chara_lists: &[Vec<i64>],
    chara_no: i64,
) -> (bool, i64) {
    if chara_no < 0 {
        return (true, 255);
    }
    let mut onoff = true;
    let mut volume = 255;
    for (index, state) in config.chrkoe.iter().enumerate() {
        if chara_lists
            .get(index)
            .is_some_and(|numbers| numbers.contains(&chara_no))
        {
            if !state.onoff {
                onoff = false;
            }
            volume = mul_raw(volume, state.volume);
        }
    }
    (onoff, volume)
}

fn pcmch_routed_volume(
    config: &crate::runtime::globals::OriginalConfigRuntimeState,
    chara_lists: &[Vec<i64>],
    state: &crate::runtime::globals::PcmChPersistentState,
    all_total: i64,
    category_total: &[i64; 5],
    bgmfade_total: i64,
    bgmfade2_total: i64,
) -> u8 {
    let volume_type = state.volume_type as i32;
    let mut volume = match volume_type {
        -1 => all_total,
        0..=4 => category_total[volume_type as usize],
        16..=31 => {
            let index = volume_type as usize;
            if config.play_sound_check[index] {
                mul_raw(all_total, config.sound_user_volume[index])
            } else {
                0
            }
        }
        _ => 255,
    };

    if state.bgm_fade_target_flag {
        volume = mul_raw(volume, bgmfade_total);
    }
    if state.bgm_fade2_target_flag {
        volume = mul_raw(volume, bgmfade2_total);
    }
    let (chara_onoff, chara_volume) = chrkoe_route(config, chara_lists, state.chara_no);
    volume = mul_raw(volume, chara_volume);
    if !chara_onoff {
        volume = 0;
    }
    volume.clamp(0, 255) as u8
}

/// Recompute the original sound routing graph.
///
/// PCMCH script volume remains on the individual Kira handle. This function
/// supplies the independent C++ `m_system_volume` term selected by
/// `volume_type`, character voice settings and the two BGM-fade target flags.
pub(crate) fn update_audio_routing(
    ctx: &mut CommandContext,
    real_delta_ms: i32,
    change_force: bool,
) -> Result<()> {
    use crate::audio::TrackKind;

    // Per-frame routing reads the settings while updating routing state;
    // a local copy keeps the copy-on-write globals uniquely owned.
    let config = &ctx.globals.syscom.original_config.clone();
    let chrkoe_lists = {
        if ctx.globals.sound_routing.chrkoe_chara_lists_cache.len() < config.chrkoe.len() {
            let lists = (0..config.chrkoe.len())
                .map(|index| chrkoe_chara_numbers(ctx, index))
                .collect::<Vec<Vec<i64>>>();
            ctx.globals.sound_routing.chrkoe_chara_lists_cache = std::sync::Arc::new(lists);
        }
        std::sync::Arc::clone(&ctx.globals.sound_routing.chrkoe_chara_lists_cache)
    };
    let all_total = if config.play_all_sound_check {
        config.all_sound_user_volume.clamp(0, 255)
    } else {
        0
    };
    let mut category_total = [0i64; 5];
    for (index, total) in category_total.iter_mut().enumerate() {
        *total = if config.play_sound_check[index] {
            mul_raw(all_total, config.sound_user_volume[index])
        } else {
            0
        };
    }

    // Source PCMCH channels are updated before the fade state transition in
    // ifc_sound.cpp. Their audible state decides whether both fades are needed.
    let old_fade = ctx.globals.sound_routing.bgmfade_total_volume;
    let old_fade2 = ctx.globals.sound_routing.bgmfade2_total_volume;
    let mut pcmch_bgm_fade_use = false;
    let source_channels: Vec<(usize, crate::runtime::globals::PcmChPersistentState)> = ctx
        .globals
        .pcmch_persistent
        .iter()
        .cloned()
        .enumerate()
        .filter(|(_, state)| state.bgm_fade_source_flag)
        .collect();
    for (channel, state) in source_channels {
        let routed = pcmch_routed_volume(
            config,
            chrkoe_lists.as_slice(),
            &state,
            all_total,
            &category_total,
            old_fade,
            old_fade2,
        );
        ctx.audio.set_slot_system_volume_raw(channel, routed)?;
        if routed > 0 && ctx.pcm.channels.playing(channel) {
            pcmch_bgm_fade_use = true;
        }
    }

    // The actual KOE player can contain either a normal message voice or an
    // EXKOE voice. EXKOE ignores the KOE on/off checkbox but still uses the KOE
    // user-volume slider; both kinds use character-group volume.
    let koe_chara_no = ctx.globals.sound_routing.koe_chara_no;
    let koe_ex_flag = ctx.globals.sound_routing.koe_ex_flag;
    let (koe_chara_onoff, koe_chara_volume) =
        chrkoe_route(config, chrkoe_lists.as_slice(), koe_chara_no);
    let mut koe_buf_total = if koe_ex_flag {
        mul_raw(all_total, config.sound_user_volume[1])
    } else {
        category_total[1]
    };
    koe_buf_total = mul_raw(koe_buf_total, koe_chara_volume);
    if !koe_ex_flag
        && (!koe_chara_onoff
            || !config.play_all_sound_check
            || !config.play_sound_check[1]
            || config.koe_mode == 1)
    {
        // tnm_update_sound_volume stops a normal voice when its current config
        // says it must not play. EXKOE is deliberately not stopped here.
        if ctx.koe.is_playing_any() {
            ctx.koe.stop(None)?;
        }
        koe_buf_total = 0;
    }
    let koe_audible = koe_buf_total > 0 && ctx.koe.is_playing_any();
    let use_bgmfade = config.bgmfade_use_check
        && (ctx.globals.script.bgmfade_flag || koe_audible || pcmch_bgm_fade_use);
    let use_bgmfade2 = koe_audible || pcmch_bgm_fade_use;
    let bgmfade2_in_start = gameexe_i64_or(ctx, "BGMFADE2.IN_START_TIME", 0);
    let bgmfade2_in_len = gameexe_i64_or(ctx, "BGMFADE2.IN_TIME_LEN", 500).max(0);
    let bgmfade2_out_start = gameexe_i64_or(ctx, "BGMFADE2.OUT_START_TIME", 0);
    let bgmfade2_out_len = gameexe_i64_or(ctx, "BGMFADE2.OUT_TIME_LEN", 500).max(0);
    let bgmfade2_volume = gameexe_i64_or(ctx, "BGMFADE2.VOLUME", 0).clamp(0, 255);
    let bgmfade2_need = ctx.globals.sound_routing.bgmfade2_need_flag;

    // Keep the mutable borrow of the routing sub-state in a narrow scope.
    // The volume application below also mutates other CommandContext fields.
    let (fade, fade2) = {
        let routing = &mut ctx.globals.sound_routing;
        routing.bgmfade_cur_time = routing
            .bgmfade_cur_time
            .saturating_add(real_delta_ms.max(0) as i64);
        routing.bgmfade2_cur_time = routing
            .bgmfade2_cur_time
            .saturating_add(real_delta_ms.max(0) as i64);

        if routing.bgmfade_flag != use_bgmfade {
            routing.bgmfade_cur_time = 0;
            routing.bgmfade_start_value = routing.bgmfade_total_volume;
            routing.bgmfade_flag = use_bgmfade;
        }
        if routing.bgmfade2_flag != use_bgmfade2 {
            routing.bgmfade2_cur_time = 0;
            routing.bgmfade2_start_value = routing.bgmfade2_total_volume;
            routing.bgmfade2_flag = use_bgmfade2;
        }

        routing.bgmfade_total_volume = if routing.bgmfade_flag {
            linear_limit_i64(
                routing.bgmfade_cur_time,
                0,
                routing.bgmfade_start_value,
                1000,
                config.bgmfade_volume,
            )
        } else {
            linear_limit_i64(
                routing.bgmfade_cur_time,
                2000,
                routing.bgmfade_start_value,
                3000,
                255,
            )
        }
        .clamp(0, 255);

        routing.bgmfade2_total_volume = if routing.bgmfade2_flag {
            linear_limit_i64(
                routing.bgmfade2_cur_time,
                bgmfade2_in_start,
                routing.bgmfade2_start_value,
                bgmfade2_in_start.saturating_add(bgmfade2_in_len),
                bgmfade2_volume,
            )
        } else if bgmfade2_need {
            linear_limit_i64(
                routing.bgmfade2_cur_time,
                bgmfade2_out_start,
                routing.bgmfade2_start_value,
                bgmfade2_out_start.saturating_add(bgmfade2_out_len),
                255,
            )
        } else {
            // Original one-voice-after-another protection waits 500ms before
            // restoring BGM when no message voice is currently attached.
            linear_limit_i64(
                routing.bgmfade2_cur_time,
                500,
                routing.bgmfade2_start_value,
                500i64.saturating_add(bgmfade2_out_len),
                255,
            )
        }
        .clamp(0, 255);

        (routing.bgmfade_total_volume, routing.bgmfade2_total_volume)
    };

    let bgm_total = mul_raw(category_total[0], fade) as u8;
    ctx.audio
        .set_track_master_volume_raw(TrackKind::Bgm, bgm_total)?;
    ctx.audio
        .set_track_master_volume_raw(TrackKind::Koe, koe_buf_total.clamp(0, 255) as u8)?;
    // Shared global PCM and PCMCH have separate gain groups. Their engine
    // voices do not pass through a second legacy PCM track/master.
    ctx.audio
        .set_track_master_volume_raw(TrackKind::Se, category_total[3] as u8)?;
    ctx.audio
        .set_track_master_volume_raw(TrackKind::Mov, category_total[4] as u8)?;
    ctx.audio
        .set_track_master_volume_raw(TrackKind::GlobalPcm, category_total[2] as u8)?;

    let normal_channels: Vec<(usize, crate::runtime::globals::PcmChPersistentState)> = ctx
        .globals
        .pcmch_persistent
        .iter()
        .cloned()
        .enumerate()
        .filter(|(_, state)| !state.bgm_fade_source_flag)
        .collect();
    for (channel, state) in normal_channels {
        let routed = pcmch_routed_volume(
            config,
            chrkoe_lists.as_slice(),
            &state,
            all_total,
            &category_total,
            fade,
            fade2,
        );
        ctx.audio.set_slot_system_volume_raw(channel, routed)?;
    }

    if change_force {
        // Force source channels through the newly calculated target as well.
        // The C++ path reaches them on the following sound update; this extra
        // pass is required only for immediate config/load application where no
        // real-time frame has elapsed yet.
        let sources: Vec<(usize, crate::runtime::globals::PcmChPersistentState)> = ctx
            .globals
            .pcmch_persistent
            .iter()
            .cloned()
            .enumerate()
            .filter(|(_, state)| state.bgm_fade_source_flag)
            .collect();
        for (channel, state) in sources {
            let routed = pcmch_routed_volume(
                config,
                chrkoe_lists.as_slice(),
                &state,
                all_total,
                &category_total,
                fade,
                fade2,
            );
            ctx.audio.set_slot_system_volume_raw(channel, routed)?;
        }
    }
    // Only instantiate groups for an admitted native sound source. A text-only
    // VM does not acquire audio capability merely because config has defaults.
    for (kind, used) in [
        (TrackKind::Bgm, ctx.bgm.current.is_some()),
        (
            TrackKind::Koe,
            ctx.koe.current_no() >= 0 || !ctx.koe.pending.is_empty(),
        ),
        (TrackKind::Se, ctx.se.has_source() || ctx.se.pending()),
        (
            TrackKind::GlobalPcm,
            ctx.pcm.has_source() || ctx.pcm.pending(),
        ),
    ] {
        ctx.audio.activate(kind.group().into(), used)?;
    }
    for channel in 0..ctx.globals.pcmch_persistent.len() {
        ctx.audio.activate(
            crate::audio::pcmch_group(channel),
            ctx.pcm.channels.allocated(channel),
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bgm_bridge::Play;
    use crate::runtime::{
        Value, VmCallMeta,
        forms::{codes, script},
    };
    use crysirror_audio::channels::ChannelAudio;
    use crysirror_core::service::{AudioParameters, AudioRequest, PcmAudio};

    fn bgm(ctx: &mut CommandContext) {
        ctx.bgm.current = Some(Play {
            name: "cue".into(),
            looping: true,
            fade_in: 0,
            fade_out: 0,
            start: 0,
            paused: false,
            delay: 0,
        });
    }
    fn publish(ctx: &mut CommandContext, audio: &mut ChannelAudio, client: u32) {
        for request in std::mem::take(&mut ctx.audio.pending) {
            audio.submit(client, request).unwrap();
        }
    }
    fn sample(audio: &mut ChannelAudio) -> f32 {
        audio.render(1).unwrap();
        audio.take_output()[0][0]
    }
    fn play(audio: &mut ChannelAudio, client: u32, channel: u32, group: Option<&str>, gain: f32) {
        audio
            .submit(
                client,
                AudioRequest::PlayPcm {
                    channel,
                    source: PcmAudio::new(48000, vec![[1.; 2]; 4096]).unwrap(),
                    parameters: AudioParameters {
                        gain,
                        looping: true,
                        mix_group: group.map(str::to_owned),
                        ..Default::default()
                    },
                },
            )
            .unwrap();
    }
    #[test]
    fn complete_script_duck_reaches_real_pcm_and_delayed_restore_preserves_foreign_audio() {
        let mut ctx = CommandContext::new("asset".into());
        bgm(&mut ctx);
        ctx.globals.syscom.original_config.all_sound_user_volume = 201;
        ctx.globals.syscom.original_config.sound_user_volume[0] = 193;
        ctx.globals.syscom.original_config.bgmfade_volume = 129;
        let mut audio = ChannelAudio::default();
        update_audio_routing(&mut ctx, 0, false).unwrap();
        publish(&mut ctx, &mut audio, 1);
        play(&mut audio, 1, 0, Some("siglus.system.bgm"), 0.5);
        play(&mut audio, 2, 0, None, 0.25);
        let near = |a: f32, b: f32| assert!((a - b).abs() < 2e-6, "{a} != {b}");
        near(sample(&mut audio), 0.25 + 0.5 * 152. / 255.);
        ctx.vm_call = Some(VmCallMeta {
            element: vec![64, codes::ELM_SCRIPT_START_BGMFADE],
            ..Default::default()
        });
        script::dispatch(&mut ctx, &[]).unwrap();
        assert!(matches!(ctx.pop(), Some(Value::Int(0))));
        update_audio_routing(&mut ctx, 0, false).unwrap();
        update_audio_routing(&mut ctx, 500, false).unwrap();
        publish(&mut ctx, &mut audio, 1);
        // Original truncates 201*193/255, then 152*192/255 before f32.
        near(sample(&mut audio), 0.25 + 0.5 * 114. / 255.);
        update_audio_routing(&mut ctx, 500, false).unwrap();
        publish(&mut ctx, &mut audio, 1);
        near(sample(&mut audio), 0.25 + 0.5 * 76. / 255.);
        ctx.vm_call.as_mut().unwrap().element[1] = codes::ELM_SCRIPT_END_BGMFADE;
        script::dispatch(&mut ctx, &[]).unwrap();
        update_audio_routing(&mut ctx, 0, false).unwrap();
        update_audio_routing(&mut ctx, 1999, false).unwrap();
        publish(&mut ctx, &mut audio, 1);
        near(sample(&mut audio), 0.25 + 0.5 * 76. / 255.);
        update_audio_routing(&mut ctx, 501, false).unwrap();
        publish(&mut ctx, &mut audio, 1);
        near(sample(&mut audio), 0.25 + 0.5 * 114. / 255.);
        update_audio_routing(&mut ctx, 500, false).unwrap();
        publish(&mut ctx, &mut audio, 1);
        near(sample(&mut audio), 0.25 + 0.5 * 152. / 255.);
        assert!(audio.is_playing(2, 0));
    }
    #[test]
    fn source_and_target_pcmch_use_previous_fades_and_original_second_fade_delay() {
        use crate::runtime::globals::PcmChPersistentState;
        let mut ctx = CommandContext::new("asset".into());
        bgm(&mut ctx);
        crate::pcmch_bridge::restore(
            &mut ctx,
            0,
            PcmChPersistentState {
                pcm_name: "source".into(),
                loop_flag: true,
                bgm_fade_source_flag: true,
                ..Default::default()
            },
        )
        .unwrap();
        crate::pcmch_bridge::restore(
            &mut ctx,
            1,
            PcmChPersistentState {
                pcm_name: "target".into(),
                loop_flag: true,
                bgm_fade2_target_flag: true,
                ..Default::default()
            },
        )
        .unwrap();
        update_audio_routing(&mut ctx, 0, false).unwrap();
        update_audio_routing(&mut ctx, 250, false).unwrap();
        assert_eq!(ctx.globals.sound_routing.bgmfade_total_volume, 240);
        assert_eq!(ctx.globals.sound_routing.bgmfade2_total_volume, 128);
        let mut audio = ChannelAudio::default();
        publish(&mut ctx, &mut audio, 1);
        play(&mut audio, 1, 5, Some("siglus.system.pcmch.1"), 1.);
        assert!((sample(&mut audio) - 128. / 255.).abs() < 2e-6);
        crate::pcmch_bridge::stop_event_channel(&mut ctx, 0).unwrap();
        update_audio_routing(&mut ctx, 0, false).unwrap();
        assert!(!ctx.globals.sound_routing.bgmfade2_flag);
        update_audio_routing(&mut ctx, 499, false).unwrap();
        assert_eq!(ctx.globals.sound_routing.bgmfade2_total_volume, 128);
        update_audio_routing(&mut ctx, 251, false).unwrap();
        assert_eq!(ctx.globals.sound_routing.bgmfade2_total_volume, 191);
    }
    #[test]
    fn text_only_default_does_not_acquire_audio_and_hub_wire_stays_empty() {
        let mut ctx = CommandContext::new("asset".into());
        update_audio_routing(&mut ctx, 100, false).unwrap();
        assert!(ctx.audio.pending.is_empty());
        #[derive(serde::Serialize)]
        struct OldHub;
        assert_eq!(
            bincode::serialize(&OldHub).unwrap(),
            bincode::serialize(&ctx.audio).unwrap()
        );
    }
}

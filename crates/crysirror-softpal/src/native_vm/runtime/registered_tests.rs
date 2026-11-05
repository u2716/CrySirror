use super::tests::empty_core_assets;
use super::*;

#[test]
fn native_text_breaks_preserve_line_boundaries_including_leading_breaks() {
    assert_eq!(
        parse_pal_text_directives("first<br>second<br>third"),
        ("first\nsecond\nthird".into(), None)
    );
    assert_eq!(
        parse_pal_text_directives("<br>first"),
        ("\nfirst".into(), None)
    );
    assert_eq!(
        parse_pal_text_directives("<s26>first<br>second</s>"),
        ("first\nsecond".into(), Some(26))
    );
    assert_eq!(
        parse_pal_text_directives("<size=30>first"),
        ("first".into(), Some(30))
    );
    assert_eq!(
        parse_pal_text_directives("first<br"),
        ("first<br".into(), None)
    );
}

fn registered_se_call(
    vm: &mut ScriptRuntime,
    assets: &CoreAssets,
    resources: &mut ResourceManager,
    audio: &mut AudioSystem,
    index: u16,
    args: &[i32],
) -> ExtCallOutcome {
    vm.stack.extend(args.iter().rev());
    vm.dispatch_registered_se(
        index,
        assets,
        Nls::ShiftJis,
        Some(resources),
        Some(audio),
        None,
    )
}

#[test]
fn registered_se_groups_prefix_volume_and_preloaded_lifetime_follow_native_calls() {
    let root = std::env::temp_dir().join(format!(
        "sena_se_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let mut wav = b"RIFF".to_vec();
    wav.extend(40u32.to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(8000u32.to_le_bytes());
    wav.extend(8000u32.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(8u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend(4u32.to_le_bytes());
    wav.extend([128; 4]);
    std::fs::write(root.join("SE_A.wav"), wav).unwrap();
    let mut resources = ResourceManager::new(&root, Nls::ShiftJis);
    let (mut assets, _) = empty_core_assets(false);
    assets.registered_softpal = true;
    let mut vm = ScriptRuntime::boot(24, ScriptRuntimeConfig::default());
    let name = vm.store_dynamic_string("SE_A".into());
    vm.se_prefix_info[0] = SePrefixInfo {
        initialized: true,
        enabled: 1,
        volume: 40,
        prefix: "SE".into(),
    };
    vm.se_prefix_info[1] = SePrefixInfo {
        initialized: true,
        enabled: 0,
        volume: 25,
        prefix: "SE_".into(),
    };
    assert_eq!(vm.registered_se_prefix_volume("se_A"), None);
    assert_eq!(vm.registered_se_prefix_volume("SE_A"), Some(2500));
    let mut audio = AudioSystem::new(crate::native_vm::audio::AudioConfig { enabled: false }).unwrap();
    vm.stack = vec![777];
    for (lane, group, percent) in [
        (0, PalSoundGroup::GROUP5, 20),
        (1, PalSoundGroup::GROUP4, 50),
        (2, PalSoundGroup::GROUP6, 80),
    ] {
        assert_eq!(
            registered_se_call(
                &mut vm,
                &assets,
                &mut resources,
                &mut audio,
                0,
                &[name, lane, lane]
            ),
            ExtCallOutcome::Continue
        );
        let handle = vm.game_audio[&(5, lane)];
        assert_eq!(handle.group, group);
        assert_eq!(audio.channel_volume(handle).unwrap().raw(), 2500);
        registered_se_call(
            &mut vm,
            &assets,
            &mut resources,
            &mut audio,
            4,
            &[percent, lane],
        );
        assert_eq!(
            registered_se_call(&mut vm, &assets, &mut resources, &mut audio, 5, &[lane]),
            ExtCallOutcome::Value(percent)
        );
    }
    assert_eq!(audio.group_volume(PalSoundGroup::GROUP5).raw(), 2000);
    assert_eq!(audio.group_volume(PalSoundGroup::GROUP4).raw(), 5000);
    assert_eq!(audio.group_volume(PalSoundGroup::GROUP6).raw(), 8000);
    vm.se_prefix_info[1].volume = 99;
    registered_se_call(
        &mut vm,
        &assets,
        &mut resources,
        &mut audio,
        1,
        &[0, 1, 200],
    );
    assert_eq!(
        audio.channel_volume(vm.game_audio[&(5, 0)]).unwrap().raw(),
        2500,
        "play must not rematch the changed prefix"
    );
    registered_se_call(&mut vm, &assets, &mut resources, &mut audio, 3, &[0, 0]);
    assert!(vm.game_audio.contains_key(&(5, 0)), "stop keeps a preload");
    registered_se_call(&mut vm, &assets, &mut resources, &mut audio, 6, &[0]);
    assert!(!vm.game_audio.contains_key(&(5, 0)));
    assert!(vm.game_audio.contains_key(&(5, 1)));
    assert_eq!(vm.stack, [777], "unload consumes one argument");
    let handle = vm.game_audio[&(5, 1)];
    assert_eq!(audio.channel_frequency(handle).unwrap(), 8000);
    registered_se_call(
        &mut vm,
        &assets,
        &mut resources,
        &mut audio,
        15,
        &[1, 16000, 1],
    );
    assert_eq!(
        registered_se_call(&mut vm, &assets, &mut resources, &mut audio, 16, &[1]),
        ExtCallOutcome::Value(16000)
    );
    registered_se_call(
        &mut vm,
        &assets,
        &mut resources,
        &mut audio,
        18,
        &[1, 10, 1],
    );
    assert_eq!(
        audio.channel_volume(handle).unwrap().raw(),
        2500,
        "native immediate-volume quirk must not directly address the logical slot"
    );
    vm.text_state.voice_autopan_limit = 4000;
    registered_se_call(&mut vm, &assets, &mut resources, &mut audio, 13, &[20]);
    registered_se_call(
        &mut vm,
        &assets,
        &mut resources,
        &mut audio,
        1,
        &[1, 0x20, 0],
    );
    assert_eq!(audio.channel_volume(handle).unwrap().raw(), 1000);
    assert_eq!(audio.channel_pan(handle).unwrap(), 4000);
    assert_eq!(vm.registered_se_pan, 0);
    registered_se_call(
        &mut vm,
        &assets,
        &mut resources,
        &mut audio,
        1,
        &[1, 0x20, 0],
    );
    assert_eq!(audio.channel_pan(handle).unwrap(), 0);
    registered_se_call(&mut vm, &assets, &mut resources, &mut audio, 14, &[1, 1]);
    assert_eq!(audio.group_volume(PalSoundGroup::GROUP4).raw(), 0);
    assert_eq!(
        registered_se_call(&mut vm, &assets, &mut resources, &mut audio, 5, &[1]),
        ExtCallOutcome::Value(50)
    );
    registered_se_call(&mut vm, &assets, &mut resources, &mut audio, 14, &[0, 1]);
    assert_eq!(audio.group_volume(PalSoundGroup::GROUP4).raw(), 5000);
    registered_se_call(&mut vm, &assets, &mut resources, &mut audio, 15, &[1, 0, 1]);
    assert_eq!(audio.channel_frequency(handle).unwrap(), 8000);
    for prefix in &mut vm.se_prefix_info {
        prefix.initialized = false;
    }
    assert_eq!(
        registered_se_call(
            &mut vm,
            &assets,
            &mut resources,
            &mut audio,
            17,
            &[4, name, 0, 0, 1, 200, 16000]
        ),
        ExtCallOutcome::Value(4)
    );
    assert_eq!(
        audio.channel_volume(vm.game_audio[&(5, 4)]).unwrap().raw(),
        10000
    );
    assert_eq!(
        audio.channel_frequency(vm.game_audio[&(5, 4)]).unwrap(),
        16000
    );
    let result = registered_se_call(
        &mut vm,
        &assets,
        &mut resources,
        &mut audio,
        2,
        &[3, name, 2, 0, 0],
    );
    assert!(matches!(result, ExtCallOutcome::Wait { value: 3, .. }));
    assert_eq!(vm.pc, 12);
    assert_eq!(vm.pending_registered_se_wait, Some(3));
    assert_eq!(
        registered_se_call(&mut vm, &assets, &mut resources, &mut audio, 2, &[]),
        ExtCallOutcome::Continue
    );
    assert_eq!(
        vm.stack,
        [777],
        "resuming a wait must not pop its arguments twice"
    );
    assert!(!vm.game_audio.contains_key(&(5, 3)));
    assert_eq!(vm.pending_registered_se_wait, None);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn registered_queries_preserve_stack_and_observe_host_state() {
    let (mut assets, points) = empty_core_assets(false);
    assets.registered_softpal = true;
    let mut vm = ScriptRuntime::boot(12, ScriptRuntimeConfig::default());
    vm.stack = vec![0x1234, 0x5678];
    assert_eq!(
        vm.dispatch_extcall(9, 63, &[], &assets, &points, None, None, None, None, None),
        ExtCallOutcome::Value(1)
    );
    vm.set_host_active(false);
    assert_eq!(
        vm.dispatch_extcall(9, 63, &[], &assets, &points, None, None, None, None, None),
        ExtCallOutcome::Value(0)
    );
    vm.set_pal_time(u32::MAX - 10);
    vm.observe_window_geometry(0, 0, 1280, 720, true);
    vm.set_pal_time(20);
    vm.observe_window_geometry(10, 20, 1280, 720, true);
    assert_eq!(
        vm.dispatch_extcall(18, 127, &[], &assets, &points, None, None, None, None, None),
        ExtCallOutcome::Value(-11)
    );
    vm.observe_window_geometry(10, 20, 1280, 720, false);
    assert_eq!(
        vm.dispatch_extcall(18, 127, &[], &assets, &points, None, None, None, None, None),
        ExtCallOutcome::Value(20)
    );
    assert_eq!(
        vm.dispatch_extcall(18, 96, &[], &assets, &points, None, None, None, None, None),
        ExtCallOutcome::Value(0)
    );
    vm.set_host_gpu_renderer(true);
    assert_eq!(
        vm.dispatch_extcall(18, 96, &[], &assets, &points, None, None, None, None, None),
        ExtCallOutcome::Value(8)
    );
    assert_eq!(vm.stack, [0x1234, 0x5678]);
}

#[test]
fn native_noop_pops_two_words_without_writing_its_destination() {
    let (mut assets, points) = empty_core_assets(false);
    assets.registered_softpal = true;
    assets.script.bytes = b"Sv20".to_vec();
    for word in [0u32, 12, 0x10017, 0x3003b, 7] {
        assets.script.bytes.extend(word.to_le_bytes());
    }
    let script = assets.script_image().unwrap();
    let mut vm = ScriptRuntime::boot(12, ScriptRuntimeConfig::default());
    vm.vars[7] = 987;
    vm.stack = vec![777, 20, 10];
    assert!(matches!(
        vm.step(&script, &points, &[], &assets, None, None, None, None, None)
            .unwrap(),
        StepResult::Continue
    ));
    assert_eq!(vm.pc, 24);
    assert_eq!(vm.vars[7], 987);
    assert_eq!(vm.stack, [777]);
}

#[test]
fn primary_aliases_and_effect_bounds_are_versioned() {
    assert_eq!(primary_extcall_for_profile(36, true), Some((20, 0)));
    assert_ne!(primary_extcall_for_profile(36, false), Some((20, 0)));
    assert_eq!(primary_extcall_for_profile(252, true), Some((2, 42)));
    assert_eq!(primary_extcall_for_profile(253, true), Some((2, 43)));
    assert_eq!(primary_extcall_for_profile(257, true), Some((2, 47)));
    for (registered, extended, maximum) in [(false, false, 29), (false, true, 49), (true, true, 99)]
    {
        let mut effects = PalEffectSystem::new();
        effects.set_registered_profile(registered, extended);
        assert_eq!(effects.effect_ex(maximum + 1, 1, 0, false, 0), 0);
        assert_eq!(effects.effect_ex(maximum, 1, 0, false, 0), 1);
    }
}

#[test]
fn movie_sprite_extcall_decodes_mpeg_and_keeps_arguments_balanced() {
    let root = std::env::temp_dir().join(format!(
        "sena_msp_dispatch_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("clip.mpg"),
        include_bytes!("../../tests/fixtures/timing_mpeg2_bframes.mpg"),
    )
    .unwrap();
    let mut manager = ResourceManager::new(&root, Nls::ShiftJis);
    let (mut assets, _) = empty_core_assets(false);
    assets.text_dat.bytes = b"clip\0".to_vec();
    let mut vm = ScriptRuntime::boot(12, ScriptRuntimeConfig::default());
    let mut sprites = SpriteSystem::new();
    vm.stack = vec![777];
    vm.stack.extend([0, 3, 0, 0, 1, 0, 0, 0].into_iter().rev());
    assert_eq!(
        vm.ext_msp_set_loop_sp_ep(
            &assets,
            Nls::ShiftJis,
            Some(&mut manager),
            Some(&mut sprites),
            None
        ),
        ExtCallOutcome::Continue
    );
    assert_eq!(vm.stack, [777]);
    let entry = &vm.game_msprites[&3];
    assert!(entry.playing);
    assert!(vm.msprite_system.check(entry.handle.unwrap()));
    assert_eq!(
        sprites.get(vm.game_sprites[&3]).unwrap().texture_size.width,
        32
    );
    assert_eq!(vm.msprite_system.advance(40).len(), 1);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn registered_sprite_probes_decode_failures_and_keeps_mpeg_stopped_until_play() {
    let root = std::env::temp_dir().join(format!(
        "sena_sprite_formats_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("scene.pgd"), b"invalid PGD").unwrap();
    std::fs::write(
        root.join("scene.jpg"),
        include_bytes!("../../tests/fixtures/color_blocks.jpg"),
    )
    .unwrap();
    std::fs::write(
        root.join("scene.mpg"),
        include_bytes!("../../tests/fixtures/timing_mpeg2_bframes.mpg"),
    )
    .unwrap();
    let mut resources = ResourceManager::new(&root, Nls::ShiftJis);
    let (mut assets, _) = empty_core_assets(false);
    assets.registered_softpal = true;
    let mut vm = ScriptRuntime::boot(12, ScriptRuntimeConfig::default());
    let name = vm.store_dynamic_string("scene".into());
    let mut sprites = SpriteSystem::new();
    vm.stack = vec![777, 0, 0, 0, 3, name];
    assert_eq!(
        vm.ext_sp_set(
            5,
            &assets,
            Nls::ShiftJis,
            Some(&mut resources),
            Some(&mut sprites),
            None,
            false
        ),
        ExtCallOutcome::Value(1)
    );
    assert_eq!(vm.stack, [777]);
    let state = &vm.game_msprites[&3];
    let decoder = state.handle.unwrap();
    assert!(!state.playing);
    assert_eq!(
        sprites.get(vm.game_sprites[&3]).unwrap().source_name,
        "scene.mpg"
    );
    assert!(vm.msprite_system.advance(100).is_empty());
    vm.stack.extend([0, 3]);
    assert_eq!(vm.ext_msp_play(), ExtCallOutcome::Continue);
    assert_eq!(vm.msprite_system.advance(40).len(), 1);
    std::fs::remove_file(root.join("scene.mpg")).unwrap();
    vm.stack.extend([0, 0, 0, 3, name]);
    assert_eq!(
        vm.ext_sp_set(
            5,
            &assets,
            Nls::ShiftJis,
            Some(&mut resources),
            Some(&mut sprites),
            None,
            false
        ),
        ExtCallOutcome::Value(1)
    );
    assert!(!vm.msprite_system.check(decoder));
    assert!(!vm.game_msprites.contains_key(&3));
    assert_eq!(
        sprites.get(vm.game_sprites[&3]).unwrap().texture_size.width,
        16
    );
    assert!(registered_images::load_sprite_resource(
        &mut resources,
        &mut vm.msprite_system,
        "scene.jpg",
        true
    )
    .is_err());
    std::fs::remove_dir_all(root).unwrap();
}

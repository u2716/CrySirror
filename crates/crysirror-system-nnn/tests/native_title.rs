//! Authored native Title over actual common image workers and retained widgets.
use crysirror_core::{
    image_processing::{ImageInput, ImageProcessState, ImageResource},
    service::{buttons, EngineQuery, HostServices, InputFrame, MemoryAssetReader},
    state::ClientId,
};
use crysirror_system_nnn::{
    adapter::NativeAdapter,
    audio_bridge::AudioBridge,
    bridge::{Bridge, CallbackProfile, SceneBindings},
    config::Names,
    expression::VariableProfile,
    machine::Machine,
    native_audio::{AudioBindings, NativeAudioPolicy},
    native_bootstrap::NativeConfiguration,
    native_resources::ResourceNames,
    native_services_bootstrap::NativePreferences,
    native_shell::NativeStartupProfile,
    native_title::{NativeTitlePlayer, NativeTitleSpec},
    resource_bridge::NativeResources,
    Program, XorProfile,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
fn names(source: &str) -> Names {
    Names::parse(source.as_bytes(), false).unwrap()
}
fn configuration() -> NativeConfiguration {
    NativeConfiguration {
        main_control: None,
        resources: ResourceNames::new("").unwrap(),
        setup: names("dwqPackLevel 0"),
        variable_names: names(""),
        project: names("back"),
        commands: names(""),
        functions: names(""),
        initial: None,
        module_sha256: "authored".into(),
        profile: VariableProfile::Legacy1000,
        scripts: XorProfile::default(),
    }
}
fn profile(config: &NativeConfiguration) -> NativeStartupProfile {
    NativeStartupProfile::from_tables(config,&names("nextMode logo"),&names("logoNumber 1 nextMode opening"),&names("backScriptFlag 1 backScriptName back nextMode printmovie"),names("backScriptFlag 1 backScriptName back basicButtonNumber 1 menu1Button1FileName button menu1Button1SizeX 2 menu1Button1SizeY 1 menu1Button1PrintX 1 menu1Button1PrintY 1 menu1Button1PicSrcX2 2 menu1Button1PicSrcY2 0 menu1Button1PicSrcX3 2 menu1Button1PicSrcY3 2 menu1Button1PicSrcX4 2 menu1Button1PicSrcY4 3 menu1Button1CheckPicX 0 menu1Button1CheckPicY 3 menu1Button1AnimeType1 5 menu1Button1AnimePattern1 2 menu1Button1AnimeSpeed1 2 menu1Button1AnimePattern3 1 menu1Button1AnimeType3 1 menu1Button1AnimePattern4 1 menu1Button1AnimeType4 1 menu1Button1ClickTime 1 bgmNumber -1 bgmRestartFlag 0"),names("movieFileName movie.mpg")).unwrap()
}
#[test]
fn native_button_copy_policy_keeps_frame_math_and_does_not_inherit_accelerators() {
    let config = configuration();
    let mut startup = profile(&config);
    startup.title=names("backScriptFlag 1 backScriptName back basicButtonNumber 2 menu1Button1FileName button menu1Button1SizeX 2 menu1Button1SizeY 1 menu1Button1PrintX 1 menu1Button1PrintY 1 menu1Button1AnimeType1 5 menu1Button1AnimePattern1 2 menu1Button1AnimeSpeed1 2 menu1Button1AccelKey1 L menu1Button1ClickSound 1 menu1Button1EnterWait 7 menu1Button1ExitWait 9");
    let spec = NativeTitleSpec::from_profile(&startup, &config).unwrap();
    assert_eq!(spec.buttons[1].origin, [1, 2]);
    assert_eq!(spec.buttons[1].accelerators, [0; 3]);
    assert_eq!(spec.buttons[0].accelerators[0], b'L');
    assert_eq!(spec.buttons[1].sounds[0], 1);
    assert_eq!(
        spec.buttons[1].sound_waits,
        [7, 7],
        "reviewed CopyParam inherits exit delay from enter delay"
    );
    let mut state = crysirror_system_nnn::native_title::NativeButtonState::new(&spec.buttons[0]);
    assert_eq!(state.frame(&spec.buttons[0]), ([0, 1], 100));
    state.printed(&spec.buttons[0]);
    assert_eq!(state.frame(&spec.buttons[0]), ([0, 1], 50));
    state.printed(&spec.buttons[0]);
    assert_eq!(state.frame(&spec.buttons[0]), ([1, 0], 100));
    assert_eq!(
        spec.buttons[0].source_rect(0, 1, [4, 1]).unwrap(),
        [2, 0, 2, 1],
        "vertical-first frame wraps into next column"
    );
}
fn setup() -> (
    tempfile::TempDir,
    crysirror_runtime::Runtime,
    NativeTitlePlayer,
    NativeAdapter,
    Arc<dyn ImageResource>,
) {
    setup_buttons(1)
}
fn setup_buttons(
    count: usize,
) -> (
    tempfile::TempDir,
    crysirror_runtime::Runtime,
    NativeTitlePlayer,
    NativeAdapter,
    Arc<dyn ImageResource>,
) {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("game")).unwrap();
    let project:crysirror_runtime::Project=serde_json::from_value(serde_json::json!({"schema":2,"name":"authored title","content_revision":"1","assets":"game","canvas":[16,16],"clients":[{"name":"native","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
    let mut runtime = crysirror_runtime::Runtime::with_asset_root(
        project,
        root.path(),
        &root.path().join("state"),
        Default::default(),
    )
    .unwrap();
    crysirror_formats_blackcyc::register_images(&mut runtime.assets.decoders).unwrap();
    runtime.tick(0, Default::default()).unwrap();
    let mut sprite = crysirror_image::ImageSurface::filled(4, 4, [0, 0, 0, 255]).unwrap();
    for (x, y, color) in [
        (0, 0, [200, 0, 0, 255]),
        (0, 1, [0, 0, 200, 255]),
        (2, 0, [0, 180, 0, 255]),
        (2, 1, [160, 160, 0, 255]),
        (2, 2, [160, 0, 160, 255]),
        (2, 3, [80, 80, 80, 255]),
    ] {
        for dx in 0..2 {
            let at = ((y * 4 + x + dx) * 4) as usize;
            sprite.rgba[at..at + 4].copy_from_slice(&color);
        }
    }
    sprite.rgba[(3 * 4) * 4 + 3] = 0;
    let mut bytes = vec![b' '; 64];
    bytes[..4].copy_from_slice(b"PNG ");
    bytes[36..40].copy_from_slice(&4u32.to_le_bytes());
    bytes[40..44].copy_from_slice(&4u32.to_le_bytes());
    bytes[48..64].copy_from_slice(b"PACKTYPE=8      ");
    bytes.extend(crysirror_image::encode_png(&sprite).unwrap());
    let bytes: Arc<[u8]> = bytes.into();
    let resources = NativeResources::blackcyc(
        ClientId(1),
        Arc::new(move |path, cancel| {
            cancel.check()?;
            assert_eq!(path, "dwq/sys/button.dwq");
            Ok(Some(Arc::new(MemoryAssetReader(bytes.clone()))))
        }),
    )
    .unwrap();
    let config = configuration();
    let mut startup = profile(&config);
    let field = startup
        .title
        .tokens
        .iter()
        .position(|s| s == b"basicButtonNumber")
        .unwrap();
    startup.title.tokens[field + 1] = count.to_string().into_bytes();
    let spec = NativeTitleSpec::from_profile(&startup, &config).unwrap();
    let player =
        NativeTitlePlayer::new(ClientId(1), 9004, 9100, [16, 16], 20, spec, resources).unwrap();
    let mut bridge = Bridge::new(
        ClientId(1),
        CallbackProfile::Mugen2Legacy,
        SceneBindings {
            root: 1,
            overlap_canvas: 7,
            effect_base: 100,
            size: [16, 16],
            layers: 16,
            music_channel: 0,
        },
    )
    .unwrap();
    bridge
        .configure_audio(
            AudioBridge::new(
                ClientId(1),
                NativeAudioPolicy::new(
                    NativePreferences::original_defaults().audio,
                    AudioBindings {
                        music: 0,
                        sound_base: 16,
                        voice_base: 32,
                    },
                )
                .unwrap(),
                names(""),
                names(""),
                Arc::new(|_, _| anyhow::bail!("no audio source expected")),
            )
            .unwrap(),
        )
        .unwrap();
    let mut words = vec![0u32; 32];
    words[0] = 32;
    words[1] = 0x66660001;
    words[2] = 0x55550001;
    words.extend([3, 0x66660002, 0x55560006]);
    words[18] = 35;
    let program = Arc::new(
        Program::parse(
            &words
                .into_iter()
                .flat_map(|w| (!w).to_le_bytes())
                .collect::<Vec<_>>(),
            XorProfile::default(),
        )
        .unwrap(),
    );
    let vm = NativeAdapter::from_prepared(program, Machine::default(), bridge).unwrap();
    let mut job = runtime
        .host
        .image_metadata(ClientId(1))
        .unwrap()
        .begin_process(
            ImageInput::Solid {
                size: [16, 16],
                color: [20, 30, 40, 255],
            },
            b"[]".as_slice().into(),
        )
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let background = loop {
        match job.poll().unwrap() {
            ImageProcessState::Ready(image) => break image,
            ImageProcessState::Failed(e) => panic!("{e}"),
            ImageProcessState::Pending => {
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            }
        }
    };
    (root, runtime, player, vm, background)
}
fn pixels(runtime: &crysirror_runtime::Runtime) -> Arc<[u8]> {
    let layer = runtime.host.engine.layers.get(ClientId(1), 9004).unwrap();
    let crysirror_scene::layer::Paint::Image { asset, .. } = &layer.paint else {
        panic!("ordinary shared image layer required")
    };
    runtime.assets.surface(ClientId(1), asset).unwrap().rgba
}
fn until_counter(
    runtime: &mut crysirror_runtime::Runtime,
    player: &mut NativeTitlePlayer,
    vm: &mut NativeAdapter,
    background: &Arc<dyn ImageResource>,
    target: u64,
) -> Option<usize> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let action = player
            .poll(
                &mut runtime.host,
                InputFrame {
                    pointer_x: 15,
                    pointer_y: 15,
                    ..Default::default()
                },
                background.clone(),
                vm,
            )
            .unwrap();
        if action.is_some() || player.diagnostics()["counter"].as_u64().unwrap() >= target {
            return action;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
}
#[test]
fn title_shared_workers_publish_exact_masked_frames_and_engine_widgets_own_pointer_hit() {
    let (_root, mut runtime, mut player, mut vm, background) = setup();
    until_counter(&mut runtime, &mut player, &mut vm, &background, 1);
    let old = pixels(&runtime);
    let at = (1 * 16 + 1) * 4;
    assert_eq!(&old[at..at + 4], &[200, 0, 0, 255]);
    runtime
        .tick(
            20,
            InputFrame {
                pointer_x: 15,
                pointer_y: 15,
                ..Default::default()
            },
        )
        .unwrap();
    until_counter(&mut runtime, &mut player, &mut vm, &background, 2);
    let second = pixels(&runtime);
    assert_eq!(
        &second[at..at + 4],
        &[100, 0, 100, 255],
        "real shared Ternary/MaskedPair worker preserves independent products"
    );
    assert_eq!(
        &old[at..at + 4],
        &[200, 0, 0, 255],
        "older immutable frame retains its generation"
    );
    runtime
        .tick(
            20,
            InputFrame {
                buttons_down: buttons::MOUSE_LEFT,
                buttons_pressed: buttons::MOUSE_LEFT,
                pointer_x: 1,
                pointer_y: 1,
            },
        )
        .unwrap();
    let interaction: serde_json::Value = serde_json::from_slice(
        &runtime
            .host
            .query_engine(ClientId(1), EngineQuery::UiInteraction(9100))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        interaction["hovered"], false,
        "transparent native check-mask pixel passes pointer through"
    );
    until_counter(&mut runtime, &mut player, &mut vm, &background, 3);
    runtime.tick(0, Default::default()).unwrap();
    runtime
        .tick(
            20,
            InputFrame {
                buttons_down: buttons::MOUSE_LEFT,
                buttons_pressed: buttons::MOUSE_LEFT,
                pointer_x: 2,
                pointer_y: 1,
            },
        )
        .unwrap();
    let interaction: serde_json::Value = serde_json::from_slice(
        &runtime
            .host
            .query_engine(ClientId(1), EngineQuery::UiInteraction(9100))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(interaction["hovered"], true);
    assert_eq!(
        until_counter(&mut runtime, &mut player, &mut vm, &background, 4),
        None,
        "native clickTime1 cannot select on its down frame"
    );
    runtime.tick(20, Default::default()).unwrap();
    assert_eq!(
        until_counter(&mut runtime, &mut player, &mut vm, &background, 6),
        Some(0),
        "native action follows final clicked-frame publication"
    );
}

#[test]
fn native_accelerator_uses_shared_virtual_key_mapping_without_pointer_hover() {
    let (_root, mut runtime, mut player, mut vm, background) = setup();
    player.spec.buttons[0].accelerators = [b'C', 0, 0];
    until_counter(&mut runtime, &mut player, &mut vm, &background, 1);
    let input = InputFrame {
        buttons_down: buttons::KEY_C,
        buttons_pressed: buttons::KEY_C,
        pointer_x: 15,
        pointer_y: 15,
    };
    runtime.tick(20, input).unwrap();
    assert_eq!(
        player
            .poll(&mut runtime.host, input, background.clone(), &mut vm)
            .unwrap(),
        None
    );
    until_counter(&mut runtime, &mut player, &mut vm, &background, 2);
    runtime.tick(20, Default::default()).unwrap();
    assert_eq!(
        until_counter(&mut runtime, &mut player, &mut vm, &background, 4),
        Some(0)
    );
}

#[test]
fn menu_group_first_click_locks_other_held_accelerators_until_its_completed_frame() {
    let (_root, mut runtime, mut player, mut vm, background) = setup_buttons(2);
    player.spec.buttons[0].accelerators = [b'C', 0, 0];
    player.spec.buttons[1].accelerators = [b'L', 0, 0];
    until_counter(&mut runtime, &mut player, &mut vm, &background, 1);
    let input = InputFrame {
        buttons_down: buttons::KEY_C | buttons::KEY_L,
        buttons_pressed: buttons::KEY_C | buttons::KEY_L,
        pointer_x: 15,
        pointer_y: 15,
    };
    runtime.tick(20, input).unwrap();
    assert_eq!(
        player
            .poll(&mut runtime.host, input, background.clone(), &mut vm)
            .unwrap(),
        None
    );
    until_counter(&mut runtime, &mut player, &mut vm, &background, 2);
    runtime.tick(20, Default::default()).unwrap();
    assert_eq!(
        until_counter(&mut runtime, &mut player, &mut vm, &background, 4),
        Some(0),
        "original group locks its first clicked button; later held key cannot replace it"
    );
}

#[test]
fn native_auto_restart_and_background_end_are_retained_until_shared_frame_completion() {
    use crysirror_system_nnn::native_shell::NativeMode;
    for by_script in [false, true] {
        let (_root, mut runtime, mut player, mut vm, background) = setup();
        player.spec.auto_restart = if by_script { 0 } else { 1 };
        player.spec.script_end_exits = by_script;
        player.spec.restart_next = NativeMode::Nothing;
        if by_script {
            player.script_ended();
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            assert!(player
                .poll(
                    &mut runtime.host,
                    Default::default(),
                    background.clone(),
                    &mut vm
                )
                .unwrap()
                .is_none());
            if let Some(mode) = player.take_restart() {
                assert_eq!(mode, NativeMode::Nothing);
                assert_eq!(player.diagnostics()["counter"], 1);
                assert!(runtime.host.engine.layers.get(ClientId(1), 9004).is_some());
                assert_eq!(&pixels(&runtime)[..4], &[20, 30, 40, 255]);
                break;
            }
            // A pending immutable image graph cannot emit an early mode return.
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(
            player.take_restart().is_none(),
            "native return is consumed once"
        );
        assert_eq!(
            runtime.host.now_ms(),
            0,
            "preparation does not advance the engine clock"
        );
    }
}

#[test]
fn completed_button_selection_precedes_same_frame_restart_and_script_end() {
    let (_root, mut runtime, mut player, mut vm, background) = setup();
    player.spec.auto_restart = 1;
    player.spec.script_end_exits = true;
    player.spec.buttons[0].click_frames = 0;
    player.spec.buttons[0].accelerators = [b'L', 0, 0];
    player.script_ended();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let action = player
            .poll(
                &mut runtime.host,
                InputFrame {
                    buttons_down: buttons::KEY_L,
                    ..Default::default()
                },
                background.clone(),
                &mut vm,
            )
            .unwrap();
        assert!(
            player.take_restart().is_none(),
            "original button branch returns before auto/script-end checks"
        );
        if let Some(action) = action {
            assert_eq!(action, 0);
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
}

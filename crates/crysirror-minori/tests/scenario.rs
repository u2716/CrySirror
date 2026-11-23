// SPDX-License-Identifier: MPL-2.0
use crysirror_core::service::{InputFrame, buttons};
use crysirror_runtime::{AdapterRegistry, Runtime};
use serde_json::json;
use std::{
    fs,
    time::{Duration, Instant},
};

#[test]
fn player_loads_include_then_chains_to_a_label_and_resumes_after_input() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("assets");
    fs::create_dir_all(root.join("scr")).unwrap();
    fs::write(
        root.join("scr/test.sc"),
        "#include \"init.sc\"\n.chain Story.sc#resume\n",
    )
    .unwrap();
    fs::write(root.join("scr/init.sc"), ".setGlobal SCORE = 7\n").unwrap();
    fs::write(root.join("scr/story.sc"),".goto missing\n.label resume\n.if SCORE == 7 OK\n.goto missing\n.label OK\nHello\n.shakeScreen H 2 12\n.wait 2\n.end\n").unwrap();
    let project=serde_json::from_value(json!({"schema":2,"name":"minori scenario","content_revision":"1","canvas":[800,600],"assets":"assets","clients":[{"name":"game","adapter":"minori","game":"authored","config":{"abi":crysirror_minori::SCENARIO_ABI}}]})).unwrap();
    let mut registry = AdapterRegistry::default();
    crysirror_minori::register(&mut registry).unwrap();
    let mut runtime =
        Runtime::with_asset_root(project, &root, &temp.path().join("state"), registry).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut inputs = 0;
    loop {
        let native = &runtime.inspect()["sessions"][0]["native"];
        if native["state"] == "finished" {
            break;
        }
        let mut input = InputFrame::default();
        if native["state"] == "input" {
            assert_eq!(
                native["script"].as_str().unwrap().to_ascii_lowercase(),
                "story.sc"
            );
            input.buttons_pressed = buttons::ENTER;
            inputs += 1;
        }
        runtime.tick(16, input).unwrap();
        assert!(Instant::now() < deadline, "{native}");
        std::thread::yield_now();
    }
    assert!(inputs >= 1);
}

#[test]
fn scenario_counter_runs_during_input_and_survives_a_chain() {
    let temp = tempfile::tempdir().unwrap();
    let assets = temp.path().join("assets");
    fs::create_dir_all(assets.join("scr")).unwrap();
    fs::write(assets.join("scr/test.sc"), ".setTimer 0\nA\n.chain next.sc").unwrap();
    fs::write(assets.join("scr/next.sc"), ".waitTimer 100\nB").unwrap();
    let project = serde_json::from_value(json!({
        "schema": 2, "name": "timer", "content_revision": "1", "canvas": [800, 600],
        "assets": "assets", "clients": [{"name": "game", "adapter": "minori",
            "game": "authored", "config": {"abi": crysirror_minori::SCENARIO_ABI}}]
    })).unwrap();
    let mut registry = AdapterRegistry::default();
    crysirror_minori::register(&mut registry).unwrap();
    let mut runtime = Runtime::with_asset_root(project, &assets, &temp.path().join("state"), registry).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let until_input = |runtime: &mut Runtime, script: &str| {
        loop {
            runtime.tick(0, InputFrame::default()).unwrap();
            let native = &runtime.inspect()["sessions"][0]["native"];
            if native["script"] == script && native["state"] == "input" { break; }
            assert!(Instant::now() < deadline, "{native}");
            std::thread::yield_now();
        }
    };
    until_input(&mut runtime, "test.sc");
    for _ in 0..10 {
        runtime.tick(100, InputFrame::default()).unwrap();
    }
    runtime.tick(10, InputFrame { buttons_pressed: buttons::ENTER, ..Default::default() }).unwrap();
    // Loading next.sc may require worker polls. Keep virtual time fixed: a
    // fresh relative wait at the chain destination would never finish.
    until_input(&mut runtime, "next.sc");
}

#[test]
fn installed_title_keeps_settings_across_two_trips_through_the_story() {
    let temp = tempfile::tempdir().unwrap();
    let assets = temp.path().join("assets");
    for bank in ["sys", "scr"] {
        fs::create_dir_all(assets.join(bank)).unwrap();
    }
    let title = crysirror_image::ImageSurface {
        width: 800,
        height: 600,
        rgba: vec![255; 800 * 600 * 4],
    };
    let png = crysirror_image::encode_png(&title).unwrap();
    for name in [
        "topmenu.png",
        "topmenuover.png",
        "configbase.png",
        "knob.png",
    ] {
        fs::write(assets.join("sys").join(name), &png).unwrap();
    }
    bitmap(&temp, "sys/checkmark.png", [12,12], [100,230,70,255]);
    pcm_wave(&temp, "voice/act-01.wav", &[8192;8]);
    fs::write(assets.join("scr/test.sc"), ".message 1 act-01.wav Actor A\n.end\n").unwrap();
    let project = serde_json::from_value(json!({
        "schema":2,"name":"native title","content_revision":"1","canvas":[800,600],
        "assets":"assets","clients":[{"name":"game","adapter":"minori","game":"authored",
        "config":{"abi":crysirror_minori::SCENARIO_ABI,"commands":["end","playVoice","message"],"voice_actors":["act","etc"],"voice_marks":[[568,358],[690,358]]}}]
    }))
    .unwrap();
    let mut registry = AdapterRegistry::default();
    crysirror_minori::register(&mut registry).unwrap();
    let mut runtime =
        Runtime::with_asset_root(project, &assets, &temp.path().join("state"), registry).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let wait = |runtime: &mut Runtime, state: &str| {
        loop {
            runtime.tick(16, InputFrame::default()).unwrap();
            let native = &runtime.inspect()["sessions"][0]["native"];
            if native["state"] == state {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "waiting for {state}, got {native}"
            );
            std::thread::yield_now();
        }
    };
    let click = |x, y| InputFrame {
        pointer_x: x,
        pointer_y: y,
        buttons_pressed: buttons::MOUSE_LEFT,
        ..Default::default()
    };
    wait(&mut runtime, "title");
    runtime.tick(16, click(700, 110)).unwrap();
    wait(&mut runtime, "settings");
    assert_eq!(layer(&mut runtime,8010).unwrap()["at"],json!([568.,358.]));
    runtime.tick(16, click(625, 360)).unwrap();
    assert!(layer(&mut runtime,8010).is_none());
    runtime.tick(16, click(695, 560)).unwrap();
    wait(&mut runtime, "title");
    until(&mut runtime, |r| r.inspect()["sessions"][0]["yield"] == "WaitForInput");
    runtime.tick(16, click(700, 110)).unwrap();
    wait(&mut runtime, "settings");
    assert!(layer(&mut runtime,8010).is_some(), "cancel must restore the actor flag");
    runtime.tick(16, click(625, 360)).unwrap();
    assert!(layer(&mut runtime,8010).is_none());
    assert!(layer(&mut runtime,8011).is_some());
    runtime.tick(16, click(29, 100)).unwrap();
    runtime.tick(16, click(615,565)).unwrap();

    for _ in 0..2 {
        wait(&mut runtime, "title");
        // Let the asynchronous settings commit complete before sending a single click.
        while runtime.inspect()["sessions"][0]["yield"] != "WaitForInput" {
            assert!(Instant::now() < deadline);
            runtime.tick(16, InputFrame::default()).unwrap();
            std::thread::yield_now();
        }
        runtime.tick(16, click(700, 50)).unwrap();
        wait(&mut runtime, "input");
        let client = runtime.inspect()["sessions"][0]["client"].as_u64().unwrap() as u32;
        assert!(!runtime.host.channel_audio.is_playing(client,1), "disabled actor emitted voice");
        assert!(layer(&mut runtime,8011).is_none());
        // At the minimum interval the first press advances the fully revealed line.
        runtime
            .tick(
                16,
                InputFrame {
                    buttons_pressed: buttons::ENTER,
                    ..Default::default()
                },
            )
            .unwrap();
        wait(&mut runtime, "title");
    }
}

fn authored(temp: &tempfile::TempDir, script: &str, commands: &[&str]) -> Runtime {
    authored_with_config(temp,script,commands,json!({}))
}

fn authored_with_config(temp: &tempfile::TempDir, script: &str, commands: &[&str], extra: serde_json::Value) -> Runtime {
    let assets = temp.path().join("assets");
    fs::create_dir_all(assets.join("scr")).unwrap();
    fs::write(assets.join("scr/test.sc"), encoding_rs::SHIFT_JIS.encode(script).0).unwrap();
    let mut config=json!({"abi":crysirror_minori::SCENARIO_ABI,"canvas":[800,600],"commands":commands});
    config.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    let project = serde_json::from_value(json!({
        "schema":2, "name":"scenario operands", "content_revision":"1", "canvas":[800,600],
        "assets":"assets","clients":[{"name":"game","adapter":"minori","game":"authored",
        "config":config}]
    })).unwrap();
    let mut registry = AdapterRegistry::default();
    crysirror_minori::register(&mut registry).unwrap();
    for name in ["topmenu.png","topmenutxt.png"] {
        bitmap(temp, &format!("sys/{name}"), [800,600], [0,0,0,255]);
    }
    let mut r = Runtime::with_asset_root(project, &assets, &temp.path().join("state"), registry).unwrap();
    until(&mut r, |r| r.inspect()["sessions"][0]["yield"] == "WaitForInput");
    r.tick(1, InputFrame { pointer_x: 100, pointer_y: 360, buttons_pressed: buttons::MOUSE_LEFT, ..Default::default() }).unwrap();
    r
}

fn until(runtime: &mut Runtime, mut predicate: impl FnMut(&mut Runtime) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(8);
    while !predicate(runtime) {
        assert!(Instant::now() < deadline, "scenario checkpoint timed out");
        runtime.tick(1, InputFrame::default()).unwrap();
        std::thread::yield_now();
    }
}

fn bitmap(temp: &tempfile::TempDir, asset: &str, size: [u32;2], color: [u8;4]) {
    let path = temp.path().join("assets").join(asset);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let image = crysirror_image::ImageSurface::filled(size[0], size[1], color).unwrap();
    fs::write(path, crysirror_image::encode_png(&image).unwrap()).unwrap();
}

fn layer(runtime: &mut Runtime, id: u32) -> Option<serde_json::Value> {
    let frame = runtime.schedule_render_snapshot().unwrap().wait(runtime).unwrap();
    frame.layers.iter().find(|item| item.id == id).map(|item|
        json!({"paint":item.paint,"opacity":item.opacity,"at":&item.quad[0][..2],"order":item.order}))
}

#[test]
fn installed_reader_skips_unregistered_lines_and_applies_qualified_voice_pan_and_gain() {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("assets/voice");
    fs::create_dir_all(&directory).unwrap();
    let mut wav = Vec::from(*b"RIFF");
    let bytes = 88200u32;
    wav.extend((bytes+36).to_le_bytes());
    wav.extend(*b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(44100u32.to_le_bytes());
    wav.extend(88200u32.to_le_bytes());
    wav.extend(2u16.to_le_bytes());
    wav.extend(16u16.to_le_bytes());
    wav.extend(*b"data");
    wav.extend(bytes.to_le_bytes());
    for _ in 0..bytes/2 { wav.extend(8192i16.to_le_bytes()); }
    fs::write(directory.join("sample"),wav).unwrap();
    for command in [".playVoice sample[40tail,-100]\n.message 1   A", ".message 1 sample[40tail,-100] actor A"] {
        let source = format!(".load unused\n.@stage bad\n{command}\n.playVoice missing\n.message 2   B");
        let mut runtime = authored(&temp, &source, &["playVoice","message","scrollXF"]);
        let client = runtime.inspect()["sessions"][0]["client"].as_u64().unwrap() as u32;
        until(&mut runtime, |r| r.host.channel_audio.is_playing(client,1));
        let info = runtime.host.channel_audio.playback_info(client,1).unwrap().unwrap();
        assert!((info.gain - 0.35 / 2.0_f32.sqrt()).abs() < 0.001, "{}", info.gain);
        runtime.host.take_pcm_output();
        runtime.tick(40,InputFrame::default()).unwrap();
        let pcm = runtime.host.take_pcm_output();
        assert!(pcm.iter().any(|x| x[0].abs() > 0.001), "missing left output");
        assert!(pcm.iter().all(|x| x[1].abs() < 0.0001), "voice pan");
        for _ in 0..3 {
            runtime.tick(12, InputFrame { buttons_pressed: buttons::ENTER, ..Default::default() }).unwrap();
        }
        assert!(!runtime.host.channel_audio.is_playing(client,1));
    }
}

#[test]
fn inline_replacement_fades_at_the_original_position_and_retires_the_old_layer() {
    let temp = tempfile::tempdir().unwrap();
    for (name, color) in [("old",[10,20,30,255]),("new",[100,90,80,255])] {
        bitmap(&temp,&format!("st/{name}.png"),[4,4],color);
    }
    let mut r = authored(&temp, ".char load 11 old.png\n.char pos 11 18 12\n.message 1   A\\x{load,20,11,new.png,40,255}",
        &["char","message","scrollXF"]);
    until(&mut r, |r| r.inspect()["sessions"][0]["native"]["state"] == "input");
    assert_eq!(layer(&mut r,20004).unwrap()["at"],json!([16.,12.]));
    assert!(layer(&mut r,20008).is_none());
    for _ in 0..30 { r.tick(10,InputFrame::default()).unwrap(); }
    let new = layer(&mut r,20008).unwrap();
    assert_eq!(new["at"],json!([16.,12.]));
    assert!((1..255).contains(&new["opacity"].as_u64().unwrap()));
    assert!(layer(&mut r,20004).is_some());
    for _ in 0..45 { r.tick(10,InputFrame::default()).unwrap(); }
    assert!(layer(&mut r,20004).is_none());
    let new = layer(&mut r,20008).unwrap();
    assert_eq!(new["paint"]["asset"],"st/new.png");
    assert_eq!(new["opacity"],254);
}

#[test]
fn stage_walk_carries_signed_displacement_and_clear_restores_the_stage() {
    let temp = tempfile::tempdir().unwrap();
    bitmap(&temp,"bg/far.png",[1024,600],[20,30,40,255]);
    bitmap(&temp,"bg/near.png",[1024,600],[50,60,70,255]);
    fs::create_dir_all(temp.path().join("assets/st")).unwrap();
    fs::write(temp.path().join("assets/st/sync.txt"),b"2\n3\n").unwrap();
    let mut r = authored(&temp, ".stage * far.png 0 0 near.png 0\n.effect WScroll2 sync:sync.txt 60 -8\n.message 1   A\n.effect *\n.message 2   B",
        &["stage","effect","message","scrollXF"]);
    until(&mut r, |r| r.inspect()["sessions"][0]["native"]["state"] == "input");
    for _ in 0..12 { r.tick(10,InputFrame::default()).unwrap(); }
    let far = layer(&mut r,1).unwrap();
    let near = layer(&mut r,2).unwrap();
    assert_ne!(far["at"][0],0.);
    assert_ne!(far["at"][0],near["at"][0]);
    assert_eq!(layer(&mut r,510).unwrap()["paint"]["asset"],"bg/far.png");
    r.tick(10,InputFrame { buttons_pressed: buttons::ENTER, ..Default::default() }).unwrap();
    until(&mut r, |r| layer(r,510).is_none());
    assert!(layer(&mut r,510).is_none());
    assert_eq!(layer(&mut r,1).unwrap()["at"],json!([0.,0.]));
}

#[test]
fn dual_walk_uses_the_third_operand_as_signed_speed() {
    let temp = tempfile::tempdir().unwrap();
    bitmap(&temp, "bg/far.png", [1024,600], [0,0,0,255]);
    bitmap(&temp, "bg/near.png", [1024,600], [128,128,128,255]);
    let mut r = authored(&temp,
        ".stage * far.png 0 0 near.png 0\n.effect WScroll3 * 10 20 -8\n.message 1   A",
        &["stage","effect","message","scrollXF"]);
    until(&mut r, |r| layer(r,510).is_some());
    let far = layer(&mut r, 1).unwrap();
    // A negative far-plane displacement leaves its origin near -width.
    assert!(far["at"][0].as_f64().unwrap() < -900.);
    assert!(layer(&mut r, 2).unwrap()["at"][0].as_f64().unwrap() > -512.);
}

#[test]
fn early_crossfade_copies_the_second_source_at_the_endpoint_and_releases_it() {
    let temp = tempfile::tempdir().unwrap();
    bitmap(&temp,"bg/first.png",[8,8],[40,30,20,255]);
    bitmap(&temp,"bg/second.png",[8,8],[10,20,90,255]);
    let mut r = authored(&temp, ".effect CrossFade2 first.png:second.png:*:* 255 10\n.message 1   A\n.effect *\n.message 2   B",
        &["effect","message"]);
    until(&mut r, |r| layer(r,560).is_some_and(|v| v["paint"]["asset"] == "bg/second.png"));
    assert_eq!(layer(&mut r,560).unwrap()["order"],json!([1,1]));
    r.tick(10,InputFrame { buttons_pressed: buttons::ENTER, ..Default::default() }).unwrap();
    until(&mut r, |r| layer(r,560).is_none());
}

#[test]
fn standing_ani_draws_a_bgra_eye_patch_at_its_signed_offset() {
    let temp = tempfile::tempdir().unwrap();
    bitmap(&temp,"st/face.png",[16,16],[80,80,80,255]);
    let mut ani = vec![0,1,1,0,0,0,0,0];
    let (label,_,_) = encoding_rs::SHIFT_JIS.encode("actor 中目");
    ani.extend_from_slice(&label);
    ani.push(0);
    for field in [1i16,1,32,-2,4] { ani.extend(field.to_le_bytes()); }
    ani.extend([10,20,30,255]);
    ani.extend([0;8]);
    fs::write(temp.path().join("assets/st/face.ani"),ani).unwrap();
    let mut r = authored(&temp, ".char load 8 face.png\n.char pos 8 30 40\n.message 1   A", &["char","message","scrollXF"]);
    until(&mut r, |r| r.inspect()["sessions"][0]["native"]["state"] == "input");
    let patch_id = 0x1000_0000 + 20004 * 64;
    let deadline = Instant::now() + Duration::from_secs(5);
    let patch = loop {
        if let Some(patch) = layer(&mut r,patch_id) { break patch; }
        assert!(Instant::now() < deadline);
        r.tick(10,InputFrame::default()).unwrap();
    };
    assert_eq!(patch["at"],json!([20.,44.]));
    let client = crysirror_core::state::ClientId(r.inspect()["sessions"][0]["client"].as_u64().unwrap() as u32);
    let surface = r.assets.surface(client,patch["paint"]["asset"].as_str().unwrap()).unwrap();
    assert_eq!(surface.rgba.as_ref(), &[30,20,10,255]);
}

#[test]
fn message_auto_advance_reaches_the_following_scene_without_another_click() {
    let temp = tempfile::tempdir().unwrap();
    bitmap(&temp, "bg/next.png", [8,8], [12,34,56,255]);
    let mut r = authored(&temp, ".message 1   Auto\\v\\a\n.stage * next.png 0 0\n.message 2   Hold", &["message","stage"]);
    until(&mut r, |r| layer(r,1).is_some_and(|v| v["paint"]["asset"]=="bg/next.png"));
    assert_eq!(layer(&mut r,1).unwrap()["paint"]["asset"],"bg/next.png");
}

#[test]
fn stacked_stage_modifiers_select_the_ani_patch_before_the_next_message() {
    let temp = tempfile::tempdir().unwrap();
    bitmap(&temp, "bg/face.png", [2, 1], [0, 0, 0, 255]);
    let mut ani = vec![0, 1, 2, 0, 0, 0, 0, 0];
    for (label, x) in [("actor 閉じ口", 0i16), ("actor 閉じ目", 1)] {
        ani.extend_from_slice(&encoding_rs::SHIFT_JIS.encode(label).0);
        ani.push(0);
        for field in [1i16, 1, 32, x, 0] { ani.extend(field.to_le_bytes()); }
        ani.extend([255, 255, 255, 128]);
    }
    fs::write(temp.path().join("assets/bg/face.ani"), ani).unwrap();
    let mut r = authored(&temp,
        ".stage * [sepia][nolipsync:0]face.png 0 0\n.message 1   One\n.stage * [noblink:2][sepia]face.png 0 0\n.message 2   Two",
        &["stage", "message"]);
    for patch in [0usize, 1] {
        until(&mut r, |r| r.inspect()["sessions"][0]["native"]["state"] == "input");
        let source = layer(&mut r, 1).unwrap();
        let client = crysirror_core::state::ClientId(1);
        let surface = r.assets.surface(client, source["paint"]["asset"].as_str().unwrap()).unwrap();
        let pixels: Vec<_> = surface.rgba.chunks_exact(4).collect();
        assert_eq!(pixels[patch], [119, 99, 72, 191]);
        assert_eq!(pixels[1 - patch], [0, 0, 0, 255]);
        if patch == 0 {
            r.tick(16, InputFrame { buttons_pressed: buttons::ENTER, ..Default::default() }).unwrap();
            // The first press can finish the current reveal.
            r.tick(16, InputFrame { buttons_pressed: buttons::ENTER, ..Default::default() }).unwrap();
        }
    }
}

#[test]
fn mode_one_dialogue_aligns_name_and_body_to_the_native_panel() {
    let temp = tempfile::tempdir().unwrap();
    bitmap(&temp, "sys/msgpanel.png", [1280, 264], [0, 0, 0, 128]);
    let mut r = authored_with_config(&temp,
        ".panel 1\n.message 1 * Actor Line\n.panel 0\n.end",
        &["panel", "message", "end"],
        json!({"canvas":[1280,720], "message_panel_layout":"columns-65-169"}));
    until(&mut r, |r| layer(r,101).is_some_and(|v| v["paint"]["text"] == "Line"));
    assert_eq!(layer(&mut r,100).unwrap()["at"], json!([0.,520.]));
    let name = layer(&mut r,102).unwrap();
    assert_eq!(name["at"], json!([65.,588.]));
    assert_eq!(name["paint"]["text"], "Actor");
    assert_eq!(layer(&mut r,101).unwrap()["at"], json!([169.,588.]));
}

fn pcm_wave(temp: &tempfile::TempDir, relative: &str, blocks: &[i16]) {
    let samples = (blocks.len() * 4096) as u32;
    let mut wav = Vec::from(*b"RIFF");
    wav.extend((36 + samples * 2).to_le_bytes());
    wav.extend(*b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(44100u32.to_le_bytes());
    wav.extend(88200u32.to_le_bytes());
    wav.extend(2u16.to_le_bytes());
    wav.extend(16u16.to_le_bytes());
    wav.extend(*b"data");
    wav.extend((samples * 2).to_le_bytes());
    for &level in blocks {
        for i in 0..4096 {
            // The first silent block has one large peak. Its average stays below 800.
            wav.extend(
                (if level == 0 && i == 0 {
                    30000i16
                } else {
                    level
                })
                .to_le_bytes(),
            );
        }
    }
    let path = temp.path().join("assets").join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, wav).unwrap();
}

fn actor_patches(temp: &tempfile::TempDir, poses: &[&str]) {
    bitmap(temp, "st/face.png", [32, 32], [80, 80, 80, 255]);
    let mut ani = vec![0, 1, poses.len() as u8, 0, 0, 0, 0, 0];
    for (index, pose) in poses.iter().enumerate() {
        let label = format!("$真響<01> {pose}");
        let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode(&label);
        ani.extend_from_slice(&bytes);
        ani.push(0);
        for field in [4i16, 4, 32, 8, 12] {
            ani.extend(field.to_le_bytes());
        }
        for _ in 0..16 {
            ani.extend([0, 0, 20 + 60 * index as u8, 255]);
        }
    }
    ani.extend([0; 8]);
    fs::write(temp.path().join("assets/st/face.ani"), ani).unwrap();
}

fn rendered_pixel(r: &mut Runtime, id: u32, x: usize, y: usize) -> Option<[u8; 4]> {
    let l = layer(r, id)?;
    let client =
        crysirror_core::state::ClientId(r.inspect()["sessions"][0]["client"].as_u64()? as u32);
    let surface = r.assets.surface(client, l["paint"]["asset"].as_str()?)?;
    let offset = (y * surface.width as usize + x) * 4;
    surface.rgba.get(offset..offset + 4)?.try_into().ok()
}

#[test]
fn retail_effect_secondary_snow_fades_without_retiring_primary_fireflies() {
    let temp = tempfile::tempdir().unwrap();
    bitmap(&temp, "bg/first.png", [80, 60], [80, 0, 0, 255]);
    bitmap(&temp, "bg/second.png", [80, 60], [0, 0, 80, 255]);
    for size in ["s", "m", "l"] {
        bitmap(&temp, &format!("sys/snow{size}.png"), [3, 3], [255; 4]);
        bitmap(&temp, &format!("sys/firefly_c{size}.png"), [3, 3], [255; 4]);
    }
    let mut runtime = authored(&temp, concat!(
        ".effect2 SnowH\n.effect *\n.transition 0 * 30\n.stage * first.png 0 0\n",
        ".effect Firefly Firefly_c 90 6000\n.transition 0 * 30\n.stage * second.png 0 0\n.message 1   A\n",
        ".effect2 fadeout\n.message 2   B\n",
        ".effect2 SnowH\n.transition 0 * 25\n.stage * first.png 0 0\n.message 3   C\n",
        ".transition 0 * 30\n.stage * second.png 0 0\n.effect2 fadeout\n.message 4   D",
    ), &["effect", "effect2", "message", "scrollXF", "stage", "transition"]);
    let client = crysirror_core::state::ClientId(
        runtime.inspect()["sessions"][0]["client"].as_u64().unwrap() as u32,
    );
    let advance = InputFrame { buttons_pressed: buttons::ENTER, ..Default::default() };
    for cycle in 0..2 {
        let deadline = Instant::now() + Duration::from_secs(8);
        loop {
            runtime.tick(100, InputFrame::default()).unwrap();
            if let Some(snow) = layer(&mut runtime, 503) {
                let pixels = runtime.assets.surface(client, snow["paint"]["asset"].as_str().unwrap()).unwrap();
                if pixels.rgba.chunks_exact(4).any(|p| p[3] >= 128)
                    && runtime.inspect()["sessions"][0]["native"]["state"] == "input" {
                    break;
                }
            }
            assert!(Instant::now() < deadline, "snow never became visible in cycle {cycle}");
            std::thread::yield_now();
        }
        assert!(layer(&mut runtime, 500).is_some());
        runtime.tick(10, advance).unwrap();
        let mut saw_fading_pixels = false;
        until(&mut runtime, |r| {
            r.tick(10, InputFrame::default()).unwrap();
            if let Some(snow) = layer(r, 503) {
                let pixels = r.assets.surface(client, snow["paint"]["asset"].as_str().unwrap()).unwrap();
                let peak = pixels.rgba.chunks_exact(4).map(|p| p[3]).max().unwrap();
                saw_fading_pixels |= (1..128).contains(&peak);
                return false;
            }
            r.inspect()["sessions"][0]["native"]["state"] == "input"
        });
        assert!(saw_fading_pixels, "slot 2 disappeared without publishing a fade");
        assert!(layer(&mut runtime, 500).is_some(), "slot 2 retired slot 1");
        if cycle == 0 { runtime.tick(10, advance).unwrap(); }
    }
}

#[test]
fn retail_debug_clears_only_the_latched_control_key() {
    let temp = tempfile::tempdir().unwrap();
    let mut r = authored(&temp, ".message 1   A\n.debug\n.message 2   B", &["message", "debug"]);
    until(&mut r, |r| r.inspect()["sessions"][0]["native"]["state"] == "input");
    let held = InputFrame { buttons_down: buttons::CTRL, ..Default::default() };
    r.tick(200, InputFrame { buttons_pressed: buttons::CTRL, ..held }).unwrap();
    assert_eq!(r.inspect()["sessions"][0]["native"]["control_latched"], true);
    r.tick(10, InputFrame { buttons_pressed: buttons::ENTER, ..held }).unwrap();
    for _ in 0..10 { r.tick(10, held).unwrap(); }
    assert_eq!(r.inspect()["sessions"][0]["native"]["control_latched"], false);
    r.tick(10, InputFrame::default()).unwrap();
    r.tick(10, InputFrame { buttons_pressed: buttons::CTRL, ..held }).unwrap();
    assert_eq!(r.inspect()["sessions"][0]["native"]["control_latched"], true);
}

#[test]
fn retail_effect_secondary_reuses_the_full_tuple_and_restarts_on_one_changed_operand() {
    let temp = tempfile::tempdir().unwrap();
    for size in ["s", "m", "l"] {
        bitmap(&temp, &format!("sys/snow{size}.png"), [3, 3], [255; 4]);
    }
    let mut r = authored(&temp, concat!(
        ".effect2 SnowH * 1 2 3\n.message 1   A\n",
        ".effect2 SnowH * 1 2 3\n.message 2   B\n",
        ".effect2 SnowH * 1 2 4\n.message 3   C"),
        &["effect2", "message", "scrollXF"]);
    let client = crysirror_core::state::ClientId(r.inspect()["sessions"][0]["client"].as_u64().unwrap() as u32);
    let peak = |r: &mut Runtime| {
        let snow = layer(r, 503)?;
        let surface = r.assets.surface(client, snow["paint"]["asset"].as_str()?)?;
        surface.rgba.chunks_exact(4).map(|p| p[3]).max()
    };
    until(&mut r, |r| {
        r.tick(100, InputFrame::default()).unwrap();
        peak(r).is_some_and(|v| v >= 128) && r.inspect()["sessions"][0]["native"]["state"] == "input"
    });
    let advance = InputFrame { buttons_pressed: buttons::ENTER, ..Default::default() };
    r.tick(10, advance).unwrap();
    until(&mut r, |r| layer(r, 101).is_some_and(|l| l["paint"]["text"] == "B") );
    assert!(peak(&mut r).is_some_and(|v| v >= 128));
    r.tick(10, advance).unwrap();
    until(&mut r, |r| layer(r, 101).is_some_and(|l| l["paint"]["text"] == "C") );
    assert!(peak(&mut r).is_some_and(|v| v < 128));
}

#[test]
fn retail_effect_screen_preference_suppresses_both_weather_slots() {
    use crysirror_core::{path::{Mount, VfsPath}, service::HostServices, state::ClientId};
    let temp = tempfile::tempdir().unwrap();
    let registry = ["effect", "effect2", "message", "scrollXF", "stage", "transition"];
    let mut setup = authored(&temp, ".message 1   A", &registry);
    let client = ClientId(setup.inspect()["sessions"][0]["client"].as_u64().unwrap() as u32);
    let storage = setup.host.storage(client).unwrap();
    drop(setup);
    storage.write_atomic(&VfsPath::new(Mount::Save, "sysparams.txt").unwrap(),
        b"screenEffect\t0\r\n").unwrap();
    bitmap(&temp, "bg/final.png", [80, 60], [150, 0, 0, 255]);
    let mut r = authored_with_config(&temp, concat!(
        ".effect Snow\n.effect2 SnowH\n.transition 2 * 1\n.stage * final.png 0 0\n",
        ".transition 3 * 0\n.stage * final.png 0 0\n.message 2   B"),
        &registry, json!({"tile_transition_grid":"canvas-fast-finish"}));
    until(&mut r, |r| r.inspect()["sessions"][0]["native"]["state"] == "input");
    for _ in 0..5 { r.tick(100, InputFrame::default()).unwrap(); }
    assert!(layer(&mut r, 500).is_none());
    assert!(layer(&mut r, 503).is_none());
    assert!(r.render_transition().unwrap().is_none());
    assert_eq!(layer(&mut r, 1).unwrap()["paint"]["asset"], "bg/final.png");
}

#[test]
fn retail_saved_draw_intervals_gate_snow_and_tile_pixels() {
    use crysirror_core::{path::{Mount, VfsPath}, service::HostServices, state::ClientId};
    let temp = tempfile::tempdir().unwrap();
    let registry = ["effect2", "message", "scrollXF", "stage", "transition"];
    let mut setup = authored(&temp, ".message 1   A", &registry);
    let c = ClientId(setup.inspect()["sessions"][0]["client"].as_u64().unwrap() as u32);
    let storage = setup.host.storage(c).unwrap();
    drop(setup);
    storage.write_atomic(&VfsPath::new(Mount::Save, "sysparams.txt").unwrap(),
        b"effectDrawInterval\t300\r\ntransitionDrawInterval\t300\r\n").unwrap();
    for size in ["s", "m", "l"] {
        bitmap(&temp, &format!("sys/snow{size}.png"), [3, 3], [255; 4]);
    }
    bitmap(&temp, "bg/to.png", [80, 60], [0, 0, 80, 255]);
    let mut r = authored_with_config(&temp, concat!(
        ".effect2 SnowH\n.message 1   A\n.effect2 *\n",
        ".transition 3 * 10\n.stage * to.png 0 0\n.message 2   B"),
        &registry, json!({"tile_transition_grid":"canvas-fast-finish"}));
    let c = ClientId(r.inspect()["sessions"][0]["client"].as_u64().unwrap() as u32);
    until(&mut r, |r| r.inspect()["sessions"][0]["native"]["state"] == "input" && layer(r, 503).is_some());
    let snow = layer(&mut r, 503).unwrap()["paint"]["asset"].as_str().unwrap().to_owned();
    r.tick(2900, InputFrame::default()).unwrap();
    assert!(r.assets.surface(c, &snow).unwrap().rgba.chunks_exact(4).all(|p| p[3] == 0));
    r.tick(200, InputFrame::default()).unwrap();
    until(&mut r, |r| r.assets.surface(c, &snow).unwrap().rgba.chunks_exact(4).any(|p| p[3] > 0));
    r.tick(1, InputFrame { buttons_pressed: buttons::ENTER, ..Default::default() }).unwrap();
    until(&mut r, |r| r.render_transition().unwrap().is_some());
    let mask = r.render_transition().unwrap().unwrap().mask.unwrap();
    r.tick(2900, InputFrame::default()).unwrap();
    assert_eq!(r.assets.surface(c, &mask).unwrap().rgba[0], 255);
    r.tick(200, InputFrame::default()).unwrap();
    assert_eq!(r.assets.surface(c, &mask).unwrap().rgba[0], 0);
}

#[test]
fn retail_tile_transition_publishes_the_mask_and_releases_it_on_completion() {
    for grid in ["fixed-800x600", "canvas"] {
        let temp = tempfile::tempdir().unwrap();
        bitmap(&temp, "bg/old.png", [800, 600], [150, 0, 0, 255]);
        bitmap(&temp, "bg/new.png", [800, 600], [0, 0, 150, 255]);
        let mut runtime = authored_with_config(&temp,
            ".stage * old.png 0 0\n.transition 3tiles * 200speed\n.stage * new.png 0 0\n.message 1   Done",
            &["stage", "transition", "message"], json!({"tile_transition_grid":grid}));
        until(&mut runtime, |r| r.render_transition().unwrap().is_some());
        let transition = runtime.render_transition().unwrap().unwrap();
        let mask = transition.mask.unwrap();
        let client = crysirror_core::state::ClientId(transition.owner);
        assert_eq!(transition.code, 1001, "shared byte-rule transition");
        assert_eq!(&runtime.assets.surface(client, &mask).unwrap().rgba[..4], &[255; 4]);
        runtime.tick(150, InputFrame::default()).unwrap();
        let bitmap = runtime.assets.surface(client, &mask).unwrap();
        assert_eq!(&bitmap.rgba[..4], &[0, 0, 0, 255]);
        assert_eq!(&bitmap.rgba[4 * 4..5 * 4], &[255; 4]);
        assert_eq!(&bitmap.rgba[40 * 4..41 * 4], &[0, 0, 0, 255]);
        runtime.tick(1500, InputFrame::default()).unwrap();
        runtime.tick(50, InputFrame::default()).unwrap();
        until(&mut runtime, |r| r.inspect()["sessions"][0]["native"]["state"] == "input");
        assert!(runtime.render_transition().unwrap().is_none());
        assert!(runtime.assets.surface(client, &mask).is_none());
        assert_eq!(layer(&mut runtime, 1).unwrap()["paint"]["asset"], "bg/new.png");
    }
}

#[test]
fn retail_tile_transition_reports_unverified_stage_sources_and_zoom() {
    for (mode, grid, animate, expected) in [
        (2, "canvas", false, "unsupported minori stage transition 2"),
        (4, "canvas", true, "changing or replaced stage surfaces"),
        (7, "", false, "unsupported minori stage transition 7"),
    ] {
        let temp = tempfile::tempdir().unwrap();
        bitmap(&temp, "bg/old.png", [80, 60], [150, 0, 0, 255]);
        bitmap(&temp, "bg/new.png", [80, 60], [0, 0, 150, 255]);
        for size in ["s", "m", "l"] {
            bitmap(&temp, &format!("sys/snow{size}.png"), [3, 3], [255; 4]);
        }
        let effect = if animate { ".effect2 SnowH\n" } else { "" };
        let config = if grid.is_empty() { json!({}) } else { json!({"tile_transition_grid":grid}) };
        let mut r = authored_with_config(&temp, &format!(
            ".stage * old.png 0 0\n{effect}.message 1   A\n.transition {mode} * 10\n.stage * new.png 0 0"),
            &["stage", "message", "transition", "effect2", "scrollXF"], config);
        until(&mut r, |r| r.inspect()["sessions"][0]["native"]["state"] == "input");
        let mut input = InputFrame { buttons_pressed: buttons::ENTER, ..Default::default() };
        let deadline = Instant::now() + Duration::from_secs(8);
        let error = loop {
            if let Err(error) = r.tick(100, input) { break error; }
            input = InputFrame::default();
            assert!(Instant::now() < deadline, "stage accepted unsupported mode {mode}");
            std::thread::yield_now();
        };
        assert!(format!("{error:#}").contains(expected), "{error:#}");
        assert!(r.render_transition().unwrap().is_none());
        assert_eq!(layer(&mut r, 1).unwrap()["paint"]["asset"], "bg/old.png");
    }
}

#[test]
fn retail_tile_transition_obeys_profile_speed_limit_and_native_advance_key() {
    for (grid, speed, hold) in [
        ("canvas-fast-finish", 256, false), ("canvas-fast-finish", 255, true),
        ("canvas", 256, true), ("fixed-800x600", 256, true), ("canvas", 0, true),
    ] {
        let temp = tempfile::tempdir().unwrap();
        bitmap(&temp, "bg/old.png", [80, 60], [150, 0, 0, 255]);
        bitmap(&temp, "bg/new.png", [80, 60], [0, 0, 150, 255]);
        let mut r = authored_with_config(&temp, &format!(
            ".stage * old.png 0 0\n.transition 3 * {speed}\n.stage * new.png 0 0\n.message 1   Done"),
            &["stage", "message", "transition"], json!({"tile_transition_grid":grid}));
        until(&mut r, |r| {
            r.render_transition().unwrap().is_some() || r.inspect()["sessions"][0]["native"]["state"] == "input"
        });
        assert_eq!(r.render_transition().unwrap().is_some(), hold, "{grid} {speed}");
        if hold {
            r.tick(10, InputFrame { buttons_pressed: buttons::SPACE, ..Default::default() }).unwrap();
            assert!(r.render_transition().unwrap().is_some());
            r.tick(10, InputFrame { buttons_pressed: buttons::ENTER, ..Default::default() }).unwrap();
            assert!(r.render_transition().unwrap().is_none());
        }
        assert_eq!(layer(&mut r, 1).unwrap()["paint"]["asset"], "bg/new.png");
    }
}

#[test]
fn bgm_and_se_follow_the_registry_fade_clock_and_release_the_tail() {
    for late in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        for bank in ["bgm", "se"] {
            pcm_wave(&temp, &format!("{bank}/tone.wav"), &[4096; 24]);
        }
        let mut commands = vec!["playBGM", "playSE", "message"];
        if late {
            commands.push("scrollXF");
        }
        let mut r = authored(
            &temp,
            ".playBGM tone.wav 4 4 100\n.playSE tone.wav true 4 4\n.message 1   A\n.playBGM * 0 2\n.playSE * false 0 2\n.message 2   B",
            &commands,
        );
        let c = r.inspect()["sessions"][0]["client"].as_u64().unwrap() as u32;
        until(&mut r, |r| r.host.channel_audio.is_playing(c, 2));
        for _ in 0..8 {
            r.tick(10, InputFrame::default()).unwrap();
        }
        for ch in [0, 2] {
            let g = r
                .host
                .channel_audio
                .playback_info(c, ch)
                .unwrap()
                .unwrap()
                .gain;
            if late {
                assert!((g - 1.).abs() < 0.001, "late {ch} gain {g}");
            } else {
                assert!((0.0..0.15).contains(&g) && g > 0., "early {ch} gain {g}");
            }
        }
        r.tick(
            5,
            InputFrame {
                buttons_pressed: buttons::ENTER,
                ..Default::default()
            },
        )
        .unwrap();
        until(&mut r, |r| {
            !r.host.channel_audio.is_playing(c, 0) && !r.host.channel_audio.is_playing(c, 2)
        });
    }
}

#[test]
fn ani_pose_selector_overrides_global_blink_at_original_deadlines() {
    let temp = tempfile::tempdir().unwrap();
    actor_patches(&temp, &["開き目", "中目 5", "閉じ目"]);
    let mut r = authored(
        &temp,
        ".setGlobal EYE_SEQ_DEFAULT = 2\n.char load 8 face.png\n.char pos 8 260 150\n.message 1   A",
        &["setGlobal", "char", "message", "scrollXF"],
    );
    let id = 0x1000_0000 + 20004 * 64;
    until(&mut r, |r| {
        rendered_pixel(r, id, 0, 0) == Some([20, 0, 0, 255])
    });
    for expected in [80, 140, 80, 20] {
        for _ in 0..30 {
            r.tick(10, InputFrame::default()).unwrap();
        }
        assert_eq!(
            rendered_pixel(&mut r, id, 0, 0),
            Some([expected, 0, 0, 255])
        );
    }
}

#[test]
fn detected_eye_program_uses_the_early_three_tick_period() {
    let temp = tempfile::tempdir().unwrap();
    actor_patches(&temp, &["開き目", "中目", "閉じ目"]);
    let programs = vec![vec![(0, 3), (1, 3), (2, 3), (1, 3)]; 6];
    let mut r = authored_with_config(
        &temp,
        ".setGlobal EYE_SEQ_DEFAULT = 5\n.char load 8 face.png\n.char pos 8 260 150\n.message 1   A",
        &["setGlobal", "char", "message", "scrollXF"],
        json!({"eye_sequences":programs}),
    );
    let id = 0x1000_0000 + 20004 * 64;
    until(&mut r, |r| {
        rendered_pixel(r, id, 0, 0) == Some([20, 0, 0, 255])
    });
    for expected in [80, 140, 80, 20] {
        for _ in 0..3 {
            r.tick(10, InputFrame::default()).unwrap();
        }
        assert_eq!(
            rendered_pixel(&mut r, id, 0, 0),
            Some([expected, 0, 0, 255])
        );
    }
}

#[test]
fn mouth_uses_completed_pcm_windows_and_updates_once_per_callback() {
    let temp = tempfile::tempdir().unwrap();
    actor_patches(&temp, &["閉じ口", "中口", "開き口"]);
    pcm_wave(
        &temp,
        "voice/act-01.wav",
        &[0, 2000, 4000, 0, 0, 0, 0, 0, 0, 0],
    );
    let mut r = authored(
        &temp,
        ".char load 8 face.png\n.char pos 8 260 150\n.message 1 act-01.wav @真　響=誰 A",
        &["char", "message", "scrollXF"],
    );
    let c = r.inspect()["sessions"][0]["client"].as_u64().unwrap() as u32;
    let id = 0x1000_0000 + 20004 * 64;
    for (block, color) in [(1, 20), (2, 80), (3, 140), (4, 80), (5, 20)] {
        until(&mut r, |r| {
            r.host
                .channel_audio
                .playback_info(c, 1)
                .unwrap()
                .is_some_and(|v| v.position_frames >= 4096 * block + 400)
        });
        for _ in 0..3 {
            r.tick(1, InputFrame::default()).unwrap();
        }
        assert_eq!(
            rendered_pixel(&mut r, id, 0, 0),
            Some([color, 0, 0, 255]),
            "block {block}"
        );
    }
}

#[test]
fn breath_accumulates_white_mask_at_speaking_mouth_and_clears() {
    let temp = tempfile::tempdir().unwrap();
    actor_patches(&temp, &["閉じ口", "中口", "開き口"]);
    pcm_wave(&temp, "voice/act-01.wav", &[4000; 80]);
    bitmap(&temp, "sys/breath.png", [48, 48], [80, 40, 20, 255]);
    let mut r = authored(
        &temp,
        ".char load 8 face.png\n.char pos 8 260 150\n.effect Breath *\n.message 1 act-01.wav #真　響 A\n.effect *\n.message 2   B",
        &["char", "effect", "message", "scrollXF"],
    );
    let id = 0x1000_0000 + 20004 * 64;
    until(&mut r, |r| layer(r, id).is_some());
    let position = layer(&mut r, id).unwrap()["at"].clone();
    let x = position[0].as_f64().unwrap() as usize + 2;
    let y = position[1].as_f64().unwrap() as usize + 2;
    until(&mut r, |r| {
        rendered_pixel(r, 552, x, y)
            .is_some_and(|[r, g, b, a]| [r, g, b] == [255; 3] && (1..=127).contains(&a))
    });
    assert_eq!(rendered_pixel(&mut r, 552, 0, 0), Some([0; 4]));
    r.tick(
        10,
        InputFrame {
            buttons_pressed: buttons::ENTER,
            ..Default::default()
        },
    )
    .unwrap();
    until(&mut r, |r| layer(r, 552).is_none());
}

#[test]
fn replacing_music_crossfades_two_live_transports_and_reuses_the_incoming_loop() {
    let t = tempfile::tempdir().unwrap();
    pcm_wave(&t, "bgm/a.wav", &[9000; 16]);
    pcm_wave(&t, "bgm/b.wav", &[4000; 16]);
    let mut r = authored(
        &t,
        ".playBGM a.wav 0 0 100\n.message 1   A\n.playBGM b.wav 20 40 80\n.message 2   B\n.playBGM b.wav 10 0 40\n.message 3   C",
        &["playBGM", "message", "scrollXF"],
    );
    let c = r.inspect()["sessions"][0]["client"].as_u64().unwrap() as u32;
    until(&mut r, |r| {
        r.inspect()["sessions"][0]["native"]["state"] == "input"
    });
    let enter = InputFrame {
        buttons_pressed: buttons::ENTER,
        ..Default::default()
    };
    r.tick(15, enter).unwrap();
    until(&mut r, |r| r.host.channel_audio.is_playing(c, 32));
    for _ in 0..7 {
        r.tick(10, InputFrame::default()).unwrap();
    }
    let old = r.host.channel_audio.playback_info(c, 0).unwrap().unwrap();
    let new = r.host.channel_audio.playback_info(c, 32).unwrap().unwrap();
    assert!((0.0..1.0).contains(&old.gain) && old.gain > 0. && new.gain > 0. && new.gain < 0.8);
    for _ in 0..45 {
        r.tick(10, Default::default()).unwrap();
    }
    assert!(!r.host.channel_audio.is_playing(c, 0));
    let before = r.host.channel_audio.playback_info(c, 32).unwrap().unwrap();
    r.tick(1, enter).unwrap();
    for _ in 0..12 {
        r.tick(10, Default::default()).unwrap();
    }
    let after = r.host.channel_audio.playback_info(c, 32).unwrap().unwrap();
    assert!(after.position_frames > before.position_frames);
    assert!((after.gain - 0.4).abs() < 0.001);
    assert!(!r.host.channel_audio.is_playing(c, 0));
}

#[test]
fn stereo_level_window_counts_interleaved_samples_and_discards_an_incomplete_tail() {
    use crysirror_audio::decoder::AudioSource;
    struct Stereo {
        cursor: usize,
    }
    impl AudioSource for Stereo {
        fn channel_count(&self) -> Option<u16> {
            Some(2)
        }
        fn sample_rate(&self) -> u32 {
            44_100
        }
        fn frame_count(&self) -> Option<u64> {
            Some(4100)
        }
        fn seek(&mut self, frame: u64) -> anyhow::Result<()> {
            self.cursor = frame as usize;
            Ok(())
        }
        fn read(&mut self, out: &mut [[f32; 2]]) -> anyhow::Result<usize> {
            let n = out.len().min(113).min(4100 - self.cursor);
            for frame in &mut out[..n] {
                *frame = if self.cursor < 2048 {
                    [-1., 0.]
                } else {
                    [0.25, -0.5]
                };
                self.cursor += 1;
            }
            Ok(n)
        }
    }
    let mut observed = vec![];
    crysirror_audio::pcm_levels::read_windows(
        &mut Stereo { cursor: 0 },
        4096,
        |channels, level| {
            observed.push((channels, level));
            Ok(())
        },
        || false,
    )
    .unwrap();
    assert_eq!(observed, [(2, 16384), (2, 12288)]);
}

#[test]
fn native_voice_properties_gate_unknown_actor_and_scale_recognized_dialogue() {
    use crysirror_core::{path::{Mount,VfsPath}, service::HostServices, state::ClientId};
    let t=tempfile::tempdir().unwrap();
    for name in ["oth-01.wav","act-01.wav"] {
        pcm_wave(&t,&format!("voice/{name}"),&[8192;24]);
    }
    let setup=authored(&t,".message 1   A",&["message","scrollXF"]);
    let c=ClientId(setup.inspect()["sessions"][0]["client"].as_u64().unwrap() as u32);
    let mut setup=setup;
    let storage=setup.host.storage(c).unwrap();
    let path=VfsPath::new(Mount::Save,"sysparams.txt").unwrap();
    storage.write_atomic(&path,b"volumeVoice\t50\r\netc\t0\r\nact\t1\r\n").unwrap();
    drop(setup);
    let mut r=authored_with_config(&t,
        ".message 1 oth-01.wav Other A\n.message 2 act-01.wav Actor B",
        &["message","scrollXF"],json!({"voice_actors":["act","etc"]}));
    until(&mut r, |r| r.inspect()["sessions"][0]["native"]["state"]=="input");
    assert!(!r.host.channel_audio.is_playing(c.0,1));
    r.tick(10,InputFrame{buttons_pressed:buttons::ENTER,..Default::default()}).unwrap();
    until(&mut r, |r| r.host.channel_audio.is_playing(c.0,1));
    let info=r.host.channel_audio.playback_info(c.0,1).unwrap().unwrap();
    assert!((info.gain-0.45).abs()<0.001);
    drop(r);
    storage.write_atomic(&path,b"muteVoice\t1\r\nact\t1\r\n").unwrap();
    let mut muted=authored_with_config(&t,".message 1 act-01.wav Actor A",&["message","scrollXF"],json!({"voice_actors":["act","etc"]}));
    until(&mut muted,|r|r.inspect()["sessions"][0]["native"]["state"]=="input");
    assert!(!muted.host.channel_audio.is_playing(c.0,1));
}

#[test]
fn breath_mask_clips_and_accumulates_coverage_with_a_half_alpha_ceiling() {
    use crysirror_image::{ImageSurface,packed_stamps::add_white_mask};
    let mask=ImageSurface{width:2,height:1,rgba:vec![1,2,3,0,80,70,60,255]};
    let mut frame=ImageSurface::filled(5,1,[0;4]).unwrap();
    for _ in 0..4 {add_white_mask(&mut frame,&mask,[-1,0],[4,1],128,512,127).unwrap();}
    assert_eq!(frame.rgba.as_slice(), &[0,0,0,0,255,255,255,127,255,255,255,127,0,0,0,0,0,0,0,0]);
}

#[test]
fn retail_tile_copies_the_current_sqz_pixels_only_on_each_native_draw() {
    use std::io::Write;
    let tmp = tempfile::tempdir().unwrap();
    bitmap(&tmp, "bg/old.png", [80, 40], [20, 20, 20, 255]);
    let frames = [[0, 215, 0, 255], [0, 0, 190, 255], [220, 0, 0, 255]].map(|bgra: [u8; 4]| {
        let mut writer = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        writer.write_all(&bgra.repeat(80 * 40)).unwrap();
        writer.finish().unwrap()
    });
    let mut bytes = b"SQZ1".to_vec();
    for v in [3u32, 80, 40, 16] { bytes.extend(v.to_le_bytes()); }
    let mut offset = 20 + 8 * frames.len() as u32;
    for f in &frames {
        bytes.extend(offset.to_le_bytes());
        bytes.extend((f.len() as u32).to_le_bytes());
        offset += f.len() as u32;
    }
    for f in frames { bytes.extend(f); }
    fs::write(tmp.path().join("assets/bg/live.sqz"), bytes).unwrap();
    let mut r = authored_with_config(&tmp,
        ".stage * old.png 0 0\n.transition 4 * 20\n.stage * live.sqz 0 0\n.message 1   Done",
        &["stage", "message", "transition", "scrollXF"], json!({"canvas":[80,40],"tile_transition_grid":"canvas"}));
    let client = crysirror_core::state::ClientId(r.inspect()["sessions"][0]["client"].as_u64().unwrap() as u32);
    let deadline = Instant::now() + Duration::from_secs(8);
    let path = loop {
        r.tick(0, InputFrame::default()).unwrap();
        if r.render_transition().unwrap().is_some() && let Some(l) = layer(&mut r, 1)
            && let Some(path) = l["paint"]["asset"].as_str() { break path.to_owned(); }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    };
    let color = |r: &Runtime, x: usize| -> [u8; 4] {
        r.assets.surface(client, &path).unwrap().rgba[4*x..4*x+4].try_into().unwrap()
    };
    let change_frame = |r: &mut Runtime, dt| {
        let revision = r.assets.surface(client, &path).unwrap().revision;
        r.tick(dt, InputFrame::default()).unwrap();
        while r.assets.surface(client, &path).unwrap().revision == revision {
            assert!(Instant::now() < deadline);
            r.tick(0, InputFrame::default()).unwrap();
            std::thread::yield_now();
        }
    };
    r.tick(50, InputFrame::default()).unwrap();
    assert_eq!(color(&r, 0), [190,0,0,255]);
    change_frame(&mut r, 30);
    assert_eq!(color(&r, 0), [190,0,0,255], "decoder changed color between draws");
    r.tick(20, InputFrame::default()).unwrap();
    assert_eq!(color(&r, 0), [0,0,220,255], "the growing square recopies its interior");
    assert_eq!(color(&r, 40), [0,0,220,255]);
    change_frame(&mut r, 30);
    r.tick(20, InputFrame::default()).unwrap();
    assert_eq!(color(&r, 0), [0,0,220,255], "completed tile retains its last copy");
    assert_eq!(color(&r, 40), [0,215,0,255], "later tile samples a different decoded frame");
    r.tick(50, InputFrame::default()).unwrap();
    assert!(r.render_transition().unwrap().is_none());
}

#[test]
fn retail_zoom_samples_both_stages_and_averages_against_each_previous_draw() {
    let tmp = tempfile::tempdir().unwrap();
    for (name,base,step) in [("old",0_u8,20_u8),("new",200,1)] {
        let mut img=crysirror_image::ImageSurface::filled(8,4,[0,0,0,255]).unwrap();
        for (i,pixel) in img.rgba.chunks_exact_mut(4).enumerate() { pixel[0]=base+(i%8) as u8*step; }
        fs::create_dir_all(tmp.path().join("assets/bg")).unwrap();
        fs::write(tmp.path().join(format!("assets/bg/{name}.png")),crysirror_image::encode_png(&img).unwrap()).unwrap();
    }
    let mut r=authored_with_config(&tmp,
        ".stage * old.png 0 0\n.transition 2 * 10\n.stage * new.png 0 0\n.wait 1000",
        &["stage","transition","wait","scrollXF"],json!({"canvas":[8,4],"tile_transition_grid":"canvas"}));
    let deadline=Instant::now()+Duration::from_secs(8);
    loop {
        r.tick(0,InputFrame::default()).unwrap();
        if r.inspect()["sessions"][0]["native"]["state"]=="transition" {break;}
        assert!(Instant::now()<deadline);
        std::thread::yield_now();
    }
    let c=crysirror_core::state::ClientId(r.inspect()["sessions"][0]["client"].as_u64().unwrap() as u32);
    for (dt,left,right) in [(200,20,120),(800,50,100),(1000,127,152),(1000,165,178)] {
        r.tick(dt,InputFrame::default()).unwrap();
        loop {
            let l=layer(&mut r,1).unwrap();
            if let Some(img)=r.assets.surface(c,l["paint"]["asset"].as_str().unwrap()) {
                assert_eq!([img.rgba[0],img.rgba[28]],[left,right]);break;
            }
            assert!(Instant::now()<deadline);
            r.tick(0,InputFrame::default()).unwrap();std::thread::yield_now();
        }
    }
    r.tick(1000,InputFrame::default()).unwrap();
    assert_eq!(layer(&mut r,1).unwrap()["paint"]["asset"],"bg/new.png");
}

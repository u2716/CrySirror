// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Public Audio reflection -> actual shared finite transport/PCM, no native VM.
use crysirror_core::{service::InputFrame, state::ClientId};
use crysirror_engine::{App, Value};
use crysirror_runtime::Runtime;
use serde_json::json;
fn value(v: serde_json::Value) -> Value {
    Value::from_json(v)
}
fn call(app: &mut App, name: &str, args: &[Value]) -> Value {
    let cx = app.context();
    let audio = cx.singleton("Audio").unwrap();
    cx.call(audio, name, args).unwrap()
}
fn app() -> (tempfile::TempDir, App) {
    let t = tempfile::tempdir().unwrap();
    std::fs::create_dir(t.path().join("assets")).unwrap();
    let mut wav = b"RIFF".to_vec();
    wav.extend(164u32.to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(2u16.to_le_bytes());
    wav.extend(8000u32.to_le_bytes());
    wav.extend(32000u32.to_le_bytes());
    wav.extend(4u16.to_le_bytes());
    wav.extend(16u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend(128u32.to_le_bytes());
    for _ in 0..32 {
        wav.extend(8192i16.to_le_bytes());
        wav.extend(16384i16.to_le_bytes());
    }
    std::fs::write(t.path().join("assets/tone.wav"), wav).unwrap();
    let project=serde_json::from_value(json!({"schema":2,"name":"public counted audio","content_revision":"1","canvas":[1,1],"assets":"assets","clients":[{"name":"one","adapter":"crysirror-script-v1","game":"one"}]})).unwrap();
    let runtime = Runtime::with_asset_root(
        project,
        &t.path().join("assets"),
        &t.path().join("state"),
        Default::default(),
    )
    .unwrap();
    let mut app = App::new(runtime, "one").unwrap();
    app.init().unwrap();
    app.start().unwrap();
    (t, app)
}
fn output(app: &mut App, ms: u64) -> Vec<[f32; 2]> {
    app.frame(ms, InputFrame::default()).unwrap();
    app.context().state().unwrap().mixer_audio.take_output()
}
fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-6, "{a} != {b}");
}
#[test]
fn public_loop_schedule_and_swap_update_real_shared_pcm_and_scoped_observations() {
    let (_t, mut app) = app();
    let h = call(
        &mut app,
        "playSfx",
        &[value(
            json!({"asset":"tone.wav","looping":true,"loop_passes":3,"swap_channels":true,"loop_points":{"start":0,"end":8}}),
        )],
    );
    let out = output(&mut app, 1);
    assert_eq!(out.len(), 48);
    for p in out {
        near(p[0], 0.5);
        near(p[1], 0.25);
    }
    let info = call(&mut app, "playbackInfo", &[h.clone()]).to_json();
    assert_eq!(info["loop_passes"], 2);
    assert_eq!(info["swap_channels"], true);
    call(
        &mut app,
        "setVoice",
        &[h.clone(), value(json!({"swap_channels":false}))],
    );
    call(
        &mut app,
        "setVoiceLoop",
        &[h.clone(), value(json!({"start":0,"end":4,"passes":2}))],
    );
    let info = call(&mut app, "playbackInfo", &[h.clone()]).to_json();
    assert_eq!(info["loop_passes"], 2);
    assert_eq!(info["swap_channels"], false);
    let cx = app.context();
    let audio = cx.singleton("Audio").unwrap();
    assert!(cx
        .call(
            audio,
            "setVoiceLoop",
            &[h.clone(), value(json!({"start":4,"end":4,"passes":2}))]
        )
        .is_err());
    app.context().state().unwrap().client = ClientId(2);
    let cx = app.context();
    let audio = cx.singleton("Audio").unwrap();
    assert!(cx
        .call(
            audio,
            "setVoiceLoop",
            &[h.clone(), value(json!({"start":0,"passes":2}))]
        )
        .is_err());
    app.context().state().unwrap().client = ClientId(1);
    let out = output(&mut app, 2);
    for (i, p) in out.iter().enumerate() {
        if i < 48 {
            near(p[0], 0.25);
            near(p[1], 0.5);
        } else {
            assert_eq!(*p, [0.; 2]);
        }
    }
    assert_eq!(call(&mut app, "voice", &[h]).to_json()["playing"], false);
}

#[test]
fn public_gain_target_and_finish_use_real_shared_faders_without_zero_envelope() {
    let (_t, mut app) = app();
    let h = call(
        &mut app,
        "playSfx",
        &[value(json!({"asset":"tone.wav","looping":true}))],
    );
    call(
        &mut app,
        "fadeVoiceGain",
        &[h.clone(), Value::Float(0.5), Value::Float(2.), Value::Null],
    );
    let out = output(&mut app, 1);
    assert_eq!(out.len(), 48);
    near(out[0][0], 0.25);
    let info = call(&mut app, "playbackInfo", &[h.clone()]).to_json();
    near(info["gain"].as_f64().unwrap() as f32, 0.75);
    assert_eq!(info["gain_fade_frames"], 48);
    assert_eq!(info["gain_stop_after_fade"], false);
    call(&mut app, "finishVoiceGainFade", &[h.clone()]);
    for p in output(&mut app, 1) {
        near(p[0], 0.125);
        near(p[1], 0.25);
    }
    call(
        &mut app,
        "fadeVoiceGain",
        &[
            h.clone(),
            Value::Float(0.25),
            Value::Float(1.),
            Value::Bool(true),
        ],
    );
    output(&mut app, 1);
    assert_eq!(call(&mut app, "voice", &[h]).to_json()["playing"], false);
}

#[test]
fn public_channel_seek_retains_actual_cursor_result_without_resetting_transport_controls() {
    use crysirror_core::service::{AudioParameters, AudioRequest, PcmAudio};
    let (_t, mut app) = app();
    app.context()
        .state()
        .unwrap()
        .runtime
        .host
        .channel_audio
        .submit(
            1,
            AudioRequest::PlayPcm {
                channel: 17,
                source: PcmAudio::new(48000, vec![[0.5, 0.25]; 100]).unwrap(),
                parameters: AudioParameters {
                    gain: 0.5,
                    paused: true,
                    looping: true,
                    loop_passes: Some(3),
                    swap_channels: true,
                    ..Default::default()
                },
            },
        )
        .unwrap();
    let ticket = call(
        &mut app,
        "beginSeekChannel",
        &[Value::Int(17), Value::Int(70)],
    );
    let status = call(&mut app, "channelSeekStatus", &[ticket.clone()]).to_json();
    assert_eq!(status["phase"], "ready");
    assert_eq!(status["channel"], 17);
    let info = call(&mut app, "nativeChannelInfo", &[Value::Int(17)]).to_json();
    assert_eq!(info["position_frames"], 70);
    assert_eq!(info["paused"], true);
    assert_eq!(info["gain"], 0.5);
    assert_eq!(info["loop_passes"], 3);
    assert_eq!(info["swap_channels"], true);
    app.context().state().unwrap().client = ClientId(2);
    let cx = app.context();
    let api = cx.singleton("Audio").unwrap();
    assert!(cx
        .call(api, "channelSeekStatus", &[ticket.clone()])
        .is_err());
    app.context().state().unwrap().client = ClientId(1);
    app.context()
        .state()
        .unwrap()
        .runtime
        .host
        .channel_audio
        .submit(
            1,
            AudioRequest::Stop {
                channel: 17,
                fade_ms: 0.,
            },
        )
        .unwrap();
    assert_eq!(
        call(&mut app, "channelSeekStatus", &[ticket.clone()]).to_json()["phase"],
        "ready"
    );
    call(&mut app, "cancelChannelSeek", &[ticket.clone()]);
    let cx = app.context();
    let api = cx.singleton("Audio").unwrap();
    assert!(cx.call(api, "channelSeekStatus", &[ticket]).is_err());
}

#[test]
fn public_retained_voice_parks_real_pcm_control_clock_and_reactivates_only_after_actual_seek() {
    let (_t, mut app) = app();
    let h = call(
        &mut app,
        "playSfx",
        &[value(
            json!({"asset":"tone.wav","retain_on_finish":true,"transport_active":false}),
        )],
    );
    call(
        &mut app,
        "fadeVoiceGain",
        &[
            h.clone(),
            Value::Float(0.5),
            Value::Float(2.),
            Value::Bool(false),
        ],
    );
    assert!(output(&mut app, 1).iter().all(|p| *p == [0.; 2]));
    let info = call(&mut app, "playbackInfo", &[h.clone()]).to_json();
    assert_eq!(info["retain_on_finish"], true);
    assert_eq!(info["transport_active"], false);
    assert_eq!(info["position_frames"], 0);
    near(info["gain"].as_f64().unwrap() as f32, 0.75);
    call(
        &mut app,
        "setVoice",
        &[h.clone(), value(json!({"transport_active":true}))],
    );
    let out = output(&mut app, 5);
    assert!(out.iter().take(180).any(|p| p[0] > 0.));
    assert!(out.iter().skip(200).all(|p| *p == [0.; 2]));
    let info = call(&mut app, "playbackInfo", &[h.clone()]).to_json();
    assert_eq!(info["transport_active"], false);
    assert_eq!(info["source_playing"], false);
    assert_eq!(info["position_frames"], 32);
    let cx = app.context();
    let api = cx.singleton("Audio").unwrap();
    assert!(cx
        .call(
            api,
            "setVoice",
            &[
                h.clone(),
                value(json!({"gain":0.9,"transport_active":true}))
            ]
        )
        .is_err());
    near(
        call(&mut app, "playbackInfo", &[h.clone()]).to_json()["gain"]
            .as_f64()
            .unwrap() as f32,
        0.5,
    );
    call(&mut app, "seekVoice", &[h.clone(), Value::Int(0)]);
    call(
        &mut app,
        "setVoice",
        &[h.clone(), value(json!({"transport_active":true}))],
    );
    for p in output(&mut app, 1) {
        near(p[0], 0.125);
        near(p[1], 0.25);
    }
}

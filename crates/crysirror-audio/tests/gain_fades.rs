// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Gain ramps/retirement are common audio state, independent of any native VM.
use crysirror_audio::{
    channels::ChannelAudio,
    decoder::Clip,
    mixer::{Mixer, MixerConfig, VoiceSpec},
    render_job::MixJob,
};
use crysirror_core::service::{AudioParameters, AudioRequest, PcmAudio};
use std::time::{Duration, Instant};
fn mixer() -> Mixer {
    Mixer::new(MixerConfig {
        sample_rate: 8000,
        max_voices: 4,
        seed: 1,
    })
    .unwrap()
}
fn source(count: usize) -> Box<dyn crysirror_audio::decoder::AudioSource> {
    Clip::new(8000, vec![[0.4; 2]; count]).unwrap().source()
}
fn render(m: &mut Mixer, n: usize) -> Vec<[f32; 2]> {
    let mut out = vec![[0.; 2]; n];
    m.render(&mut out).unwrap();
    out
}
fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-6, "{a} != {b}");
}
#[test]
fn nonzero_gain_target_and_stop_use_one_real_ramp_with_pause_and_exact_inspection() {
    let mut m = mixer();
    let h = m.play(1, source(100), VoiceSpec::default()).unwrap();
    m.fade_gain_with_stop(1, h, 0.25, 1., true).unwrap();
    for (i, p) in render(&mut m, 4).iter().enumerate() {
        near(p[0], 0.4 * (1. - 0.75 * i as f32 / 8.));
    }
    let info = m.playback_info(1, h).unwrap();
    near(info.gain, 0.625);
    near(info.gain_target, 0.25);
    assert_eq!(info.gain_fade_frames, 4);
    assert!(info.gain_stop_after_fade && info.stopping);
    assert_eq!(info.fade_out_frames, 0);
    m.pause(1, h, true).unwrap();
    assert!(render(&mut m, 7).iter().flatten().all(|v| *v == 0.));
    assert_eq!(m.playback_info(1, h).unwrap().gain_fade_frames, 4);
    m.pause(1, h, false).unwrap();
    let out = render(&mut m, 5);
    for (i, p) in out[..4].iter().enumerate() {
        near(p[0], 0.4 * (0.625 - 0.75 * i as f32 / 8.));
    }
    assert_eq!(out[4], [0.; 2]);
    assert!(!m.is_playing(1, h));
}
#[test]
fn finishing_gain_fade_publishes_target_without_seek_and_only_gain_stop_retires_voice() {
    let mut m = mixer();
    let h = m.play(1, source(100), VoiceSpec::default()).unwrap();
    m.fade_gain(1, h, 0.25, 2.).unwrap();
    render(&mut m, 2);
    let position = m.position(1, h).unwrap();
    assert!(m.finish_gain_fade(2, h).is_err());
    m.finish_gain_fade(1, h).unwrap();
    m.finish_gain_fade(1, h).unwrap();
    let info = m.playback_info(1, h).unwrap();
    near(info.gain, 0.25);
    assert_eq!(info.gain_fade_frames, 0);
    assert_eq!(m.position(1, h).unwrap(), position);
    near(render(&mut m, 1)[0][0], 0.1);
    m.fade_gain_with_stop(1, h, 0.75, 2., true).unwrap();
    m.finish_gain_fade(1, h).unwrap();
    assert!(!m.is_playing(1, h));
    assert!(m.finish_gain_fade(1, h).is_err());
}
#[test]
fn new_gain_control_cancels_only_gain_retirement_and_independent_stop_deadlines_survive() {
    let mut m = mixer();
    let h = m.play(1, source(100), VoiceSpec::default()).unwrap();
    m.fade_gain_with_stop(1, h, 0.5, 1., true).unwrap();
    m.set_gain(1, h, 0.8).unwrap();
    render(&mut m, 9);
    assert!(m.is_playing(1, h));
    assert!(!m.playback_info(1, h).unwrap().gain_stop_after_fade);
    m.stop(1, h, 2.).unwrap();
    m.fade_gain_with_stop(1, h, 0.25, 4., false).unwrap();
    let info = m.playback_info(1, h).unwrap();
    assert_eq!(info.fade_out_frames, 16);
    assert_eq!(info.gain_fade_frames, 32);
    render(&mut m, 16);
    assert!(!m.is_playing(1, h));
}
#[test]
fn source_eof_does_not_fake_gain_completion_or_prolong_source_playing() {
    let mut m = mixer();
    let h = m.play(1, source(2), VoiceSpec::default()).unwrap();
    m.fade_gain_with_stop(1, h, 0.2, 1., true).unwrap();
    render(&mut m, 3);
    let info = m.playback_info(1, h).unwrap();
    assert!(!info.source_playing);
    assert!(info.stopping && info.gain_stop_after_fade);
    assert_eq!(info.gain_fade_frames, 5);
    assert!(render(&mut m, 5).iter().flatten().all(|v| *v == 0.));
    assert!(!m.is_playing(1, h));
}
#[test]
fn rendered_gain_and_stop_observations_publish_with_worker_pcm() {
    let mut m = mixer();
    let h = m.play(1, source(100), VoiceSpec::default()).unwrap();
    m.fade_gain_with_stop(1, h, 0.25, 2., true).unwrap();
    let pool = crysirror_tasks::TaskPool::new("gain-fade", 1, 1).unwrap();
    let mut job = MixJob::begin(&pool, &mut [&mut m], 4, vec![]).unwrap();
    let info = m.playback_info(1, h).unwrap();
    near(info.gain, 1.);
    assert_eq!(info.gain_fade_frames, 16);
    assert!(m.finish_gain_fade(1, h).is_err());
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        if job.poll(&mut [&mut m]).unwrap().is_some() {
            break;
        }
        assert!(Instant::now() < until);
        std::thread::yield_now();
    }
    let info = m.playback_info(1, h).unwrap();
    near(info.gain, 0.8125);
    assert_eq!(info.gain_fade_frames, 12);
    assert!(info.gain_stop_after_fade);
}
#[test]
fn typed_gain_ramp_and_finish_requests_reach_shared_channel_samples() {
    let mut a = ChannelAudio::default();
    a.submit(
        1,
        AudioRequest::PlayPcm {
            channel: 7,
            source: PcmAudio::new(48000, vec![[0.4; 2]; 512]).unwrap(),
            parameters: AudioParameters::default(),
        },
    )
    .unwrap();
    a.submit(
        1,
        AudioRequest::FadeGain {
            channel: 7,
            gain: 0.5,
            fade_ms: 2.,
            stop_after: false,
        },
    )
    .unwrap();
    a.render(1).unwrap();
    let info = a.playback_info(1, 7).unwrap().unwrap();
    near(info.gain, 0.75);
    assert_eq!(info.gain_fade_frames, 48);
    a.submit(1, AudioRequest::FinishGainFade { channel: 7 })
        .unwrap();
    a.take_output();
    a.render(1).unwrap();
    for p in a.take_output() {
        near(p[0], 0.2);
    }
    a.submit(
        1,
        AudioRequest::FadeGain {
            channel: 7,
            gain: 0.25,
            fade_ms: 1.,
            stop_after: true,
        },
    )
    .unwrap();
    a.render(1).unwrap();
    assert!(!a.is_playing(1, 7));
    assert!(a
        .submit(
            1,
            AudioRequest::FadeGain {
                channel: 88,
                gain: f32::NAN,
                fade_ms: 1.,
                stop_after: false
            }
        )
        .is_err());
}

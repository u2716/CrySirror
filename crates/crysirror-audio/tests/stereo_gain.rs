// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Actual PCM controls and worker publication, no native VM or device.
use crysirror_audio::{
    channels::ChannelAudio,
    decoder::Clip,
    mixer::{Mixer, MixerConfig, VoiceSpec},
    render_job::MixJob,
};
use crysirror_core::service::{AudioParameters, AudioRequest, PcmAudio};
fn mixer() -> Mixer {
    Mixer::new(MixerConfig {
        sample_rate: 8000,
        max_voices: 2,
        seed: 1,
    })
    .unwrap()
}
fn clip() -> Clip {
    Clip::new(8000, vec![[0.2, 0.4]; 512]).unwrap()
}
fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 1e-6, "{actual} != {expected}");
}
#[test]
fn independent_stereo_amplitudes_and_ramps_preserve_pan_and_fail_atomically() {
    let mut m = mixer();
    let h = m.play(1, clip().source(), VoiceSpec::default()).unwrap();
    m.set_stereo_gain(1, h, [0.25, 1.], 0.).unwrap();
    let mut out = [[0.; 2]; 4];
    m.render(&mut out).unwrap();
    for frame in out {
        near(frame[0], 0.05);
        near(frame[1], 0.4);
    }
    for gains in [[f32::NAN, 0.], [0., 5.], [-1., 1.]] {
        assert!(m.set_stereo_gain(1, h, gains, 0.).is_err());
        assert_eq!(m.stereo_gain(1, h).unwrap(), [0.25, 1.]);
    }
    assert!(m.set_stereo_gain(2, h, [0., 0.], 0.).is_err());
    assert!(m.set_stereo_gain(1, h, [0., 0.], -1.).is_err());
    m.set_stereo_gain(1, h, [1., 1.], 0.).unwrap();
    m.set_stereo_gain(1, h, [0., 1.], 1.).unwrap();
    let mut out = [[0.; 2]; 9];
    m.render(&mut out).unwrap();
    for (i, frame) in out.iter().enumerate() {
        near(frame[0], 0.2 * (1. - (i as f32 / 8.).min(1.)));
        near(frame[1], 0.4);
    }
    m.set_stereo_gain(1, h, [1., 1.], 0.).unwrap();
    m.set_pan(1, h, 0., 0.).unwrap();
    let mut out = [[0.; 2]; 1];
    m.render(&mut out).unwrap();
    near(out[0][0], 0.2 * std::f32::consts::SQRT_2);
    near(out[0][1], 0.);
    m.stop(1, h, 0.).unwrap();
    let fresh = m.play(1, clip().source(), VoiceSpec::default()).unwrap();
    assert_eq!(m.stereo_gain(1, fresh).unwrap(), [1.; 2]);
    assert!(m.stereo_gain(1, h).is_err());
}
#[test]
fn typed_channel_request_changes_actual_pcm_without_replacing_the_source() {
    let mut audio = ChannelAudio::default();
    audio
        .submit(
            1,
            AudioRequest::PlayPcm {
                channel: 3,
                source: PcmAudio::new(8000, vec![[0.2, 0.4]; 512]).unwrap(),
                parameters: AudioParameters {
                    looping: true,
                    ..Default::default()
                },
            },
        )
        .unwrap();
    audio
        .submit(
            1,
            AudioRequest::SetStereoGain {
                channel: 3,
                gains: [0.25, 1.],
                fade_ms: 0.,
            },
        )
        .unwrap();
    audio.render(1).unwrap();
    let first = audio.take_output();
    assert!(!first.is_empty());
    for frame in first {
        near(frame[0], 0.05);
        near(frame[1], 0.4);
    }
    let before = audio.playback_info(1, 3).unwrap().unwrap().position_frames;
    assert!(audio
        .submit(
            1,
            AudioRequest::SetStereoGain {
                channel: 3,
                gains: [0., f32::INFINITY],
                fade_ms: 0.
            }
        )
        .is_err());
    assert_eq!(
        audio.playback_info(1, 3).unwrap().unwrap().position_frames,
        before
    );
    audio.render(1).unwrap();
    for frame in audio.take_output() {
        near(frame[0], 0.05);
        near(frame[1], 0.4);
    }
}
#[test]
fn render_worker_owns_channel_ramps_and_publishes_observation_and_pcm_together() {
    let mut m = mixer();
    let h = m.play(1, clip().source(), VoiceSpec::default()).unwrap();
    m.set_stereo_gain(1, h, [0., 1.], 1.).unwrap();
    let pool = crysirror_tasks::TaskPool::new("stereo-gain", 1, 1).unwrap();
    let mut job = MixJob::begin(&pool, &mut [&mut m], 16, vec![]).unwrap();
    assert_eq!(m.stereo_gain(1, h).unwrap(), [1., 1.]);
    assert!(m.set_stereo_gain(1, h, [0., 0.], 0.).is_err());
    assert!(m.stereo_gain(2, h).is_err());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let pcm = loop {
        if let Some(pcm) = job.poll(&mut [&mut m]).unwrap() {
            break pcm;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    };
    assert_eq!(m.stereo_gain(1, h).unwrap(), [0., 1.]);
    for (i, frame) in pcm.iter().enumerate() {
        near(frame[0], 0.2 * (1. - (i as f32 / 8.).min(1.)));
        near(frame[1], 0.4);
    }
}

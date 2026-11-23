// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Engine-owned finite transport and channel exchange, no VM/device involved.
use crysirror_audio::{
    channels::ChannelAudio,
    decoder::Clip,
    mixer::{LoopPoints, Mixer, MixerConfig, VoiceSpec},
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
fn source(samples: &[f32]) -> Box<dyn crysirror_audio::decoder::AudioSource> {
    Clip::new(8000, samples.iter().map(|x| [*x; 2]).collect())
        .unwrap()
        .source()
}
fn render(m: &mut Mixer, count: usize) -> Vec<[f32; 2]> {
    let mut out = vec![[0.; 2]; count];
    m.render(&mut out).unwrap();
    out
}
fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 1e-6, "{actual} != {expected}");
}
fn expect(actual: &[[f32; 2]], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (a, e) in actual.iter().zip(expected) {
        near(a[0], *e);
        near(a[1], *e);
    }
}
#[test]
fn finite_loop_counts_current_pass_and_stops_at_range_instead_of_source_end() {
    let mut m = mixer();
    let h = m
        .play(
            1,
            source(&[0.1, 0.2, 0.3, 0.4, 0.5]),
            VoiceSpec {
                looping: true,
                loop_passes: Some(3),
                loop_points: Some(LoopPoints { start: 2, end: 4 }),
                ..Default::default()
            },
        )
        .unwrap();
    expect(&render(&mut m, 4), &[0.1, 0.2, 0.3, 0.4]);
    let info = m.playback_info(1, h).unwrap();
    assert_eq!(info.loop_passes, Some(2));
    assert_eq!(info.position_frames, 2);
    expect(&render(&mut m, 2), &[0.3, 0.4]);
    assert_eq!(m.playback_info(1, h).unwrap().loop_passes, Some(1));
    expect(&render(&mut m, 4), &[0.3, 0.4, 0., 0.]);
    assert!(!m.is_playing(1, h));
}
#[test]
fn final_fractional_pass_holds_last_sample_and_fast_steps_count_all_crossings() {
    let mut m = mixer();
    m.play(
        1,
        source(&[0.1, 0.2, 0.3, 0.9]),
        VoiceSpec {
            rate: 0.5,
            looping: true,
            loop_passes: Some(2),
            loop_points: Some(LoopPoints { start: 1, end: 3 }),
            ..Default::default()
        },
    )
    .unwrap();
    expect(
        &render(&mut m, 12),
        &[0.1, 0.15, 0.2, 0.25, 0.3, 0.25, 0.2, 0.25, 0.3, 0.3, 0., 0.],
    );
    let mut m = mixer();
    let h = m
        .play(
            1,
            source(&[0.3]),
            VoiceSpec {
                rate: 4.,
                looping: true,
                loop_passes: Some(5),
                ..Default::default()
            },
        )
        .unwrap();
    expect(&render(&mut m, 1), &[0.3]);
    assert_eq!(m.playback_info(1, h).unwrap().loop_passes, Some(1));
    expect(&render(&mut m, 2), &[0.3, 0.]);
    assert!(!m.is_playing(1, h));
}
#[test]
fn live_schedule_validates_atomically_and_pause_seek_preserve_remaining_count() {
    let mut m = mixer();
    let h = m
        .play(
            1,
            source(&[0.1, 0.2, 0.3, 0.4, 0.5]),
            VoiceSpec {
                looping: true,
                loop_passes: Some(3),
                ..Default::default()
            },
        )
        .unwrap();
    render(&mut m, 1);
    for (owner, start, end, passes) in [
        (2, 0, Some(5), Some(2)),
        (1, 3, Some(3), Some(2)),
        (1, 0, Some(6), Some(2)),
        (1, 0, Some(4), Some(0)),
    ] {
        assert!(m.set_loop_schedule(owner, h, start, end, passes).is_err());
        let info = m.playback_info(1, h).unwrap();
        assert_eq!(info.position_frames, 1);
        assert_eq!(info.loop_passes, Some(3));
    }
    m.set_loop_schedule(1, h, 1, Some(4), Some(2)).unwrap();
    assert_eq!(m.playback_info(1, h).unwrap().position_frames, 1);
    m.pause(1, h, true).unwrap();
    expect(&render(&mut m, 7), &[0.; 7]);
    assert_eq!(m.playback_info(1, h).unwrap().loop_passes, Some(2));
    m.seek(1, h, 2).unwrap();
    assert_eq!(m.playback_info(1, h).unwrap().loop_passes, Some(2));
    m.pause(1, h, false).unwrap();
    expect(&render(&mut m, 2), &[0.3, 0.4]);
    assert_eq!(m.playback_info(1, h).unwrap().loop_passes, Some(1));
    m.set_looped(1, h, true, None).unwrap();
    assert_eq!(m.playback_info(1, h).unwrap().loop_passes, None);
    expect(&render(&mut m, 4), &[0.2, 0.3, 0.4, 0.2]);
}
#[test]
fn decoded_exchange_is_independent_of_output_stereo_gain_and_keeps_transport() {
    let mut m = mixer();
    let h = m
        .play(
            1,
            Clip::new(8000, vec![[0.2, 0.6]; 16]).unwrap().source(),
            VoiceSpec {
                swap_channels: true,
                ..Default::default()
            },
        )
        .unwrap();
    m.set_stereo_gain(1, h, [0.5, 1.], 0.).unwrap();
    let out = render(&mut m, 2);
    near(out[0][0], 0.3);
    near(out[0][1], 0.2);
    assert!(m.playback_info(1, h).unwrap().swap_channels);
    assert!(m.set_channel_swap(2, h, false).is_err());
    m.set_channel_swap(1, h, false).unwrap();
    assert_eq!(m.playback_info(1, h).unwrap().position_frames, 2);
    let out = render(&mut m, 1);
    near(out[0][0], 0.1);
    near(out[0][1], 0.6);
    assert_eq!(m.playback_info(1, h).unwrap().position_frames, 3);
}
#[test]
fn typed_channel_controls_and_startup_parameters_reach_actual_pcm() {
    let mut a = ChannelAudio::default();
    a.submit(
        1,
        AudioRequest::PlayPcm {
            channel: 7,
            source: PcmAudio::new(48000, vec![[0.2, 0.6]; 512]).unwrap(),
            parameters: AudioParameters {
                looping: true,
                loop_passes: Some(4),
                swap_channels: true,
                ..Default::default()
            },
        },
    )
    .unwrap();
    a.submit(
        1,
        AudioRequest::SetLoopSchedule {
            channel: 7,
            start: 0,
            end: Some(64),
            passes: Some(2),
        },
    )
    .unwrap();
    a.render(1).unwrap();
    let out = a.take_output();
    assert_eq!(out.len(), 48);
    for p in out {
        near(p[0], 0.6);
        near(p[1], 0.2);
    }
    a.submit(
        1,
        AudioRequest::SetChannelSwap {
            channel: 7,
            swapped: false,
        },
    )
    .unwrap();
    a.render(2).unwrap();
    let out = a.take_output();
    assert_eq!(out.len(), 96);
    for (i, p) in out.iter().enumerate() {
        if i < 80 {
            near(p[0], 0.2);
            near(p[1], 0.6);
        } else {
            assert_eq!(*p, [0.; 2]);
        }
    }
    assert!(!a.is_playing(1, 7));
}
#[test]
fn worker_holds_old_count_and_swap_observation_until_actual_publication() {
    let mut m = mixer();
    let h = m
        .play(
            1,
            source(&[0.1, 0.2, 0.3, 0.4]),
            VoiceSpec {
                looping: true,
                loop_passes: Some(3),
                swap_channels: true,
                ..Default::default()
            },
        )
        .unwrap();
    let pool = crysirror_tasks::TaskPool::new("counted-loop", 1, 1).unwrap();
    let mut job = MixJob::begin(&pool, &mut [&mut m], 5, vec![]).unwrap();
    assert_eq!(m.playback_info(1, h).unwrap().loop_passes, Some(3));
    assert!(m.set_loop_schedule(1, h, 0, None, Some(9)).is_err());
    assert!(m.set_channel_swap(1, h, false).is_err());
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        if job.poll(&mut [&mut m]).unwrap().is_some() {
            break;
        }
        assert!(Instant::now() < until);
        std::thread::yield_now();
    }
    let info = m.playback_info(1, h).unwrap();
    assert_eq!(info.loop_passes, Some(2));
    assert_eq!(info.position_frames, 1);
    assert!(info.swap_channels);
}
#[test]
fn invalid_initial_count_does_not_admit_a_voice() {
    let mut m = mixer();
    for spec in [
        VoiceSpec {
            loop_passes: Some(2),
            ..Default::default()
        },
        VoiceSpec {
            looping: true,
            loop_passes: Some(0),
            ..Default::default()
        },
    ] {
        assert!(m.play(1, source(&[0.1]), spec).is_err());
    }
    assert_eq!(m.stats().active, 0);
}

#[test]
fn unknown_length_source_finishes_counted_passes_at_real_decoder_eof() {
    struct Unknown {
        at: usize,
    }
    impl crysirror_audio::decoder::AudioSource for Unknown {
        fn sample_rate(&self) -> u32 {
            8000
        }
        fn frame_count(&self) -> Option<u64> {
            None
        }
        fn read(&mut self, out: &mut [[f32; 2]]) -> anyhow::Result<usize> {
            let n = out.len().min(2 - self.at);
            for (i, p) in out[..n].iter_mut().enumerate() {
                *p = [if self.at + i == 0 { 0.2 } else { 0.4 }; 2];
            }
            self.at += n;
            Ok(n)
        }
        fn seek(&mut self, frame: u64) -> anyhow::Result<()> {
            anyhow::ensure!(frame <= 2, "seek");
            self.at = frame as usize;
            Ok(())
        }
    }
    let mut m = mixer();
    m.play(
        1,
        Box::new(Unknown { at: 0 }),
        VoiceSpec {
            looping: true,
            loop_passes: Some(2),
            ..Default::default()
        },
    )
    .unwrap();
    expect(&render(&mut m, 6), &[0.2, 0.4, 0.2, 0.4, 0., 0.]);
    let mut a = ChannelAudio::default();
    assert!(a
        .submit(
            1,
            AudioRequest::SetLoopSchedule {
                channel: 7,
                start: 3,
                end: Some(3),
                passes: Some(2)
            }
        )
        .is_err());
    assert!(a
        .submit(
            1,
            AudioRequest::SetLoopSchedule {
                channel: 7,
                start: 0,
                end: None,
                passes: Some(0)
            }
        )
        .is_err());
}

//! Canonical retained sources and control clocks, without any VM or device.
use crysirror_audio::{
    channels::ChannelAudio,
    decoder::{AudioSource, Clip},
    mixer::{LoopPoints, Mixer, MixerConfig, VoiceSpec},
    render_job::MixJob,
};
use crysirror_core::service::{AudioParameters, AudioRequest, PcmAudio};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
fn mixer(count: usize) -> Mixer {
    Mixer::new(MixerConfig {
        sample_rate: 8000,
        max_voices: count,
        seed: 1,
    })
    .unwrap()
}
fn source(samples: &[f32]) -> Box<dyn AudioSource> {
    Clip::new(8000, samples.iter().map(|v| [*v; 2]).collect())
        .unwrap()
        .source()
}
fn render(m: &mut Mixer, count: usize) -> Vec<[f32; 2]> {
    let mut out = vec![[0.; 2]; count];
    m.render(&mut out).unwrap();
    out
}
fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-6, "{a} != {b}");
}
struct Counted {
    inner: Box<dyn AudioSource>,
    reads: Arc<AtomicUsize>,
    seeks: Arc<AtomicUsize>,
    drops: Arc<AtomicUsize>,
    unknown: bool,
}
impl Drop for Counted {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}
impl AudioSource for Counted {
    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }
    fn frame_count(&self) -> Option<u64> {
        if self.unknown {
            None
        } else {
            self.inner.frame_count()
        }
    }
    fn read(&mut self, out: &mut [[f32; 2]]) -> anyhow::Result<usize> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        self.inner.read(out)
    }
    fn seek(&mut self, frame: u64) -> anyhow::Result<()> {
        self.seeks.fetch_add(1, Ordering::SeqCst);
        self.inner.seek(frame)
    }
}
#[test]
fn retained_eof_keeps_one_source_exact_cursor_and_gain_then_replays_by_seek_without_reopen() {
    let mut m = mixer(2);
    let reads = Arc::new(AtomicUsize::new(0));
    let seeks = Arc::new(AtomicUsize::new(0));
    let drops = Arc::new(AtomicUsize::new(0));
    let h = m
        .play(
            1,
            Box::new(Counted {
                inner: source(&[0.1, 0.2, 0.3, 0.4]),
                reads: reads.clone(),
                seeks: seeks.clone(),
                drops: drops.clone(),
                unknown: false,
            }),
            VoiceSpec {
                retain_on_finish: true,
                ..Default::default()
            },
        )
        .unwrap();
    m.fade_gain(1, h, 0.25, 1.).unwrap();
    render(&mut m, 4);
    let info = m.playback_info(1, h).unwrap();
    assert!(!info.source_playing && info.retain_on_finish && !info.transport_active);
    assert_eq!(info.position_frames, 4);
    assert_eq!(info.transport_frames, 4);
    assert_eq!(info.gain_fade_frames, 4);
    let count = reads.load(Ordering::SeqCst);
    assert!(render(&mut m, 4).iter().all(|p| *p == [0.; 2]));
    let info = m.playback_info(1, h).unwrap();
    near(info.gain, 0.25);
    assert_eq!(info.gain_fade_frames, 0);
    assert_eq!(info.position_frames, 4);
    assert_eq!(reads.load(Ordering::SeqCst), count);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    assert!(m.set_transport_active(1, h, true).is_err());
    assert!(m.set_transport_active(2, h, false).is_err());
    m.seek(1, h, 0).unwrap();
    m.set_transport_active(1, h, true).unwrap();
    let out = render(&mut m, 4);
    for (p, v) in out.iter().zip([0.025, 0.05, 0.075, 0.1]) {
        near(p[0], v);
    }
    assert_eq!(seeks.load(Ordering::SeqCst), 1);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    m.stop(1, h, 0.).unwrap();
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(!m.is_playing(1, h));
}
#[test]
fn inactive_transport_advances_independent_gain_but_full_pause_freezes_it() {
    let mut m = mixer(2);
    let h = m
        .play(
            1,
            source(&[0.4; 32]),
            VoiceSpec {
                retain_on_finish: true,
                transport_active: false,
                ..Default::default()
            },
        )
        .unwrap();
    m.fade_gain(1, h, 0.5, 1.).unwrap();
    assert!(render(&mut m, 4).iter().all(|p| *p == [0.; 2]));
    let info = m.playback_info(1, h).unwrap();
    near(info.gain, 0.75);
    assert_eq!(info.position_frames, 0);
    assert_eq!(info.transport_frames, 0);
    m.pause(1, h, true).unwrap();
    render(&mut m, 4);
    near(m.playback_info(1, h).unwrap().gain, 0.75);
    m.pause(1, h, false).unwrap();
    render(&mut m, 4);
    near(m.playback_info(1, h).unwrap().gain, 0.5);
    m.set_transport_active(1, h, true).unwrap();
    near(render(&mut m, 1)[0][0], 0.2);
    assert_eq!(m.playback_info(1, h).unwrap().position_frames, 1);
}
#[test]
fn gain_stop_and_finish_park_at_the_actual_frame_without_destroying_or_resetting_controls() {
    let mut m = mixer(2);
    let h = m
        .play(
            1,
            source(&[0.4; 32]),
            VoiceSpec {
                retain_on_finish: true,
                ..Default::default()
            },
        )
        .unwrap();
    m.fade_gain_with_stop(1, h, 0.5, 0.5, true).unwrap();
    render(&mut m, 12);
    let info = m.playback_info(1, h).unwrap();
    assert!(info.retain_on_finish && !info.transport_active && info.source_playing);
    assert!(!info.gain_stop_after_fade);
    assert_eq!(info.position_frames, 4);
    near(info.gain, 0.5);
    m.set_transport_active(1, h, true).unwrap();
    near(render(&mut m, 1)[0][0], 0.2);
    m.fade_gain_with_stop(1, h, 0.25, 4., true).unwrap();
    m.finish_gain_fade(1, h).unwrap();
    let info = m.playback_info(1, h).unwrap();
    assert!(!info.transport_active);
    near(info.gain, 0.25);
    assert_eq!(info.position_frames, 5);
    m.stop(1, h, 0.).unwrap();
    assert!(!m.is_playing(1, h));
}
#[test]
fn counted_loop_completion_is_retained_and_seek_does_not_invent_another_pass() {
    let mut m = mixer(2);
    let h = m
        .play(
            1,
            source(&[0.1, 0.2, 0.3, 0.4, 0.5]),
            VoiceSpec {
                retain_on_finish: true,
                looping: true,
                loop_passes: Some(2),
                loop_points: Some(LoopPoints { start: 2, end: 4 }),
                ..Default::default()
            },
        )
        .unwrap();
    let out = render(&mut m, 8);
    for (p, v) in out.iter().zip([0.1, 0.2, 0.3, 0.4, 0.3, 0.4, 0., 0.]) {
        near(p[0], v);
    }
    let info = m.playback_info(1, h).unwrap();
    assert_eq!(info.loop_passes, Some(1));
    assert_eq!(info.position_frames, 4);
    assert!(!info.transport_active && !info.source_playing);
    m.seek(1, h, 2).unwrap();
    m.set_transport_active(1, h, true).unwrap();
    let out = render(&mut m, 4);
    for (p, v) in out.iter().zip([0.3, 0.4, 0., 0.]) {
        near(p[0], v);
    }
    assert_eq!(m.playback_info(1, h).unwrap().loop_passes, Some(1));
}
#[test]
fn retained_admission_and_protection_never_steal_another_canonical_or_peer_source() {
    let mut m = mixer(2);
    let own = m
        .play(
            1,
            source(&[0.1; 32]),
            VoiceSpec {
                retain_on_finish: true,
                transport_active: false,
                ..Default::default()
            },
        )
        .unwrap();
    let peer = m.play(2, source(&[0.2; 32]), VoiceSpec::default()).unwrap();
    assert!(m
        .play(
            3,
            source(&[0.3; 32]),
            VoiceSpec {
                retain_on_finish: true,
                priority: i32::MAX,
                ..Default::default()
            }
        )
        .is_err());
    assert!(m.is_playing(1, own) && m.is_playing(2, peer));
    let new = m
        .play(
            3,
            source(&[0.3; 32]),
            VoiceSpec {
                priority: i32::MAX,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(m.is_playing(1, own) && m.is_playing(3, new));
    assert!(!m.is_playing(2, peer));
    m.stop(3, new, 0.).unwrap();
    m.set_client_limit(1, 1).unwrap();
    assert!(m.play(1, source(&[0.4; 32]), VoiceSpec::default()).is_err());
    assert!(m.is_playing(1, own));
}
#[test]
fn retained_completion_is_published_only_with_the_worker_frame() {
    let mut m = mixer(2);
    let h = m
        .play(
            1,
            source(&[0.1, 0.2]),
            VoiceSpec {
                retain_on_finish: true,
                ..Default::default()
            },
        )
        .unwrap();
    let pool = crysirror_tasks::TaskPool::new("retained-frame", 1, 1).unwrap();
    let mut job = MixJob::begin(&pool, &mut [&mut m], 4, vec![]).unwrap();
    assert!(m.playback_info(1, h).unwrap().transport_active);
    assert_eq!(m.position(1, h).unwrap(), 0.);
    assert!(m.set_transport_active(1, h, false).is_err());
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        if job.poll(&mut [&mut m]).unwrap().is_some() {
            break;
        }
        assert!(Instant::now() < until);
        std::thread::yield_now();
    }
    let info = m.playback_info(1, h).unwrap();
    assert!(!info.transport_active && !info.source_playing);
    assert_eq!(info.position_frames, 2);
}
#[test]
fn unknown_length_stream_end_retains_and_does_not_keep_reading() {
    let mut m = mixer(2);
    let reads = Arc::new(AtomicUsize::new(0));
    let h = m
        .play(
            1,
            Box::new(Counted {
                inner: source(&[0.1, 0.2]),
                reads: reads.clone(),
                seeks: Arc::new(AtomicUsize::new(0)),
                drops: Arc::new(AtomicUsize::new(0)),
                unknown: true,
            }),
            VoiceSpec {
                retain_on_finish: true,
                ..Default::default()
            },
        )
        .unwrap();
    render(&mut m, 8);
    let info = m.playback_info(1, h).unwrap();
    assert!(!info.source_playing && !info.transport_active);
    assert_eq!(info.source_frames, None);
    assert_eq!(info.position_frames, 2);
    let before = reads.load(Ordering::SeqCst);
    render(&mut m, 8);
    assert_eq!(reads.load(Ordering::SeqCst), before);
}
#[test]
fn typed_channel_activity_reuses_the_same_pcm_cursor_and_survives_eof_until_explicit_stop() {
    let mut a = ChannelAudio::default();
    a.submit(
        1,
        AudioRequest::PlayPcm {
            channel: 7,
            source: PcmAudio::new(48000, vec![[0.4; 2]; 96]).unwrap(),
            parameters: AudioParameters {
                retain_on_finish: true,
                transport_active: false,
                ..Default::default()
            },
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
    assert!(a.take_output().iter().all(|p| *p == [0.; 2]));
    near(a.playback_info(1, 7).unwrap().unwrap().gain, 0.75);
    a.submit(
        1,
        AudioRequest::SetTransportActive {
            channel: 7,
            active: true,
        },
    )
    .unwrap();
    a.render(3).unwrap();
    let info = a.playback_info(1, 7).unwrap().unwrap();
    assert_eq!(info.position_frames, 96);
    assert!(!info.transport_active && !info.source_playing);
    assert!(a
        .submit(
            2,
            AudioRequest::SetTransportActive {
                channel: 7,
                active: true
            }
        )
        .is_err());
    a.submit(
        1,
        AudioRequest::Stop {
            channel: 7,
            fade_ms: 0.,
        },
    )
    .unwrap();
    assert!(a.playback_info(1, 7).unwrap().is_none());
}

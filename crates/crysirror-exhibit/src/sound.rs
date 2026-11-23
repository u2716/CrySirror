// SPDX-License-Identifier: MPL-2.0
//! Native RLD v1 music, loop-effect and one-shot channel policy.
use crate::registers::{Bank, Change, Registers, Value};
use crate::voice;
use anyhow::{bail, ensure, Context, Result};
use crysirror_core::service::{buttons, InputFrame};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{
        AudioAsset, AudioParameters, AudioPcm16Gain, AudioPreparePhase, AudioRequest, HostServices,
        PcmBus,
    },
    state::ClientId,
};
use crysirror_tasks::TaskPool;

// Retouch's 257-entry amplitude curve, indexed after applying the system
// channel volume. Values at either endpoint are valid coefficients.
const LEVELS: [u16; 257] = [
    0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 6, 6, 6, 7, 7, 8, 9, 9, 10,
    10, 11, 11, 12, 13, 13, 14, 15, 16, 16, 17, 18, 19, 20, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29,
    30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 43, 44, 45, 46, 47, 49, 50, 51, 52, 54, 55, 56,
    57, 59, 60, 61, 63, 64, 65, 67, 68, 70, 71, 72, 74, 75, 77, 78, 80, 81, 82, 84, 85, 87, 88, 90,
    91, 93, 94, 96, 97, 99, 100, 102, 104, 105, 107, 108, 110, 111, 113, 114, 116, 118, 119, 121,
    122, 124, 125, 127, 129, 130, 132, 133, 135, 136, 138, 139, 141, 143, 144, 146, 147, 149, 150,
    152, 153, 155, 157, 158, 160, 161, 163, 164, 166, 167, 169, 170, 172, 173, 175, 176, 177, 179,
    180, 182, 183, 185, 186, 187, 189, 190, 192, 193, 194, 196, 197, 198, 200, 201, 202, 203, 205,
    206, 207, 208, 210, 211, 212, 213, 214, 216, 217, 218, 219, 220, 221, 222, 223, 224, 225, 226,
    227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237, 237, 238, 239, 240, 241, 241, 242, 243,
    244, 244, 245, 246, 246, 247, 247, 248, 248, 249, 250, 250, 251, 251, 251, 252, 252, 253, 253,
    253, 254, 254, 254, 255, 255, 255, 255, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256,
    256,
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Sound {
    Environment {
        id: i32,
        fade: i32,
        levels: [i32; 2],
    },
    StopEnvironment(i32),
    Music {
        id: i32,
        fade: i32,
        levels: [i32; 2],
        looping: bool,
    },
    Loop {
        id: i32,
        fade: i32,
        levels: [i32; 2],
    },
    Effect {
        id: i32,
        levels: [i32; 2],
        slot: i32,
        wait: bool,
    },
    Voice {
        id: i32,
        levels: [i32; 2],
        wait: bool,
    },
    LeaveVoice(i32),
    StopMusic {
        fade: i32,
        wait: bool,
    },
    StopLoop(i32),
    LeaveEffect {
        fade: i32,
        slot: i32,
    },
}
pub(super) fn decode(op: u16, flags: u8, args: &[i32]) -> Result<Sound> {
    Ok(match (op, flags, args) {
        (0x34, 0, [id, fade, l, r]) => Sound::Environment {
            id: *id,
            fade: *fade,
            levels: [*l, *r],
        },
        (0x35, 0, [fade]) => Sound::StopEnvironment(*fade),
        (0x32, 0, [id, fade, l, r, looping, 0]) => Sound::Music {
            id: *id,
            fade: *fade,
            levels: [*l, *r],
            looping: *looping != 0,
        },
        (0x33, 0, [fade, wait]) => Sound::StopMusic {
            fade: *fade,
            wait: *wait != 0,
        },
        (0x36, 0, [id, fade, l, r]) => Sound::Loop {
            id: *id,
            fade: *fade,
            levels: [*l, *r],
        },
        (0x37, 0, [fade]) => Sound::StopLoop(*fade),
        (0x38 | 0x3a, 0x10, [id, l, r, _, 1]) => Sound::Voice {
            id: *id,
            levels: [*l, *r],
            wait: op == 0x3a,
        },
        (0x39, 0x10, [fade, _, 1]) => Sound::LeaveVoice(*fade),
        (0x38 | 0x3a, 0, [id, l, r, slot]) | (0x38 | 0x3a, 0x10, [id, l, r, slot, 0]) => {
            Sound::Effect {
                id: *id,
                levels: [*l, *r],
                slot: *slot,
                wait: op == 0x3a,
            }
        }
        (0x39, 0, [fade, slot]) | (0x39, 0x10, [fade, slot, 0]) => Sound::LeaveEffect {
            fade: *fade,
            slot: *slot,
        },
        _ => bail!("unsupported v1 audio opcode {op:#x} flags {flags:#x} operands {args:?}"),
    })
}
#[derive(Clone, Copy)]
enum Finish {
    Continue,
    WaitForEnd(u32),
    WaitFade(u32),
}
struct Pending {
    ticket: u64,
    channel: u32,
    finish: Finish,
    seen: Option<i32>,
}
enum FadeWait {
    Start { channel: u32, ms: u32 },
    Stop(u32),
}
pub(super) struct Audio {
    pub scene: [i32; 12],
    pub environment: crate::environment::Environment,
    music: Option<i32>,
    loop_effect: Option<i32>,
    loading: Option<Pending>,
    waiting_end: Option<u32>,
    fade: Option<FadeWait>,
    voice: Option<voice::Voice>,
    mark_after_wait: Option<i32>,
}
impl Default for Audio {
    fn default() -> Self {
        Self {
            environment: Default::default(),
            scene: [0, -1, 0, 0, -1, 0, 0, -1, -1, -1, 214, 214],
            music: None,
            loop_effect: None,
            loading: None,
            waiting_end: None,
            fade: None,
            voice: None,
            mark_after_wait: None,
        }
    }
}
fn effect_channel(slot: i32) -> u32 {
    if slot == 1 {
        15
    } else {
        14
    }
}
pub(super) fn stereo(regs: &Registers, base: i32, levels: [i32; 2]) -> Result<[u32; 2]> {
    let mut stereo = [0; 2];
    for (i, level) in levels.into_iter().enumerate() {
        let n = level.wrapping_mul(regs.read(Value::System(base + i as i32))?) / 256;
        ensure!(n >= 0, "unsupported retained stereo level");
        stereo[i] = u32::from(LEVELS[n.min(256) as usize]);
    }
    Ok(stereo)
}
impl Audio {
    pub fn preparation_pending(&self) -> bool {
        self.loading.is_some() || self.voice.is_some() || self.environment.preparing()
    }
    fn finish(&mut self, channel: u32, action: Finish) {
        match action {
            Finish::Continue => {}
            Finish::WaitForEnd(ch) => self.waiting_end = Some(ch),
            Finish::WaitFade(ms) => self.fade = Some(FadeWait::Start { channel, ms }),
        }
    }
    pub fn poll(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        r: &mut Registers,
        input: InputFrame,
    ) -> Result<bool> {
        if !self.environment.poll(h, c)? {
            return Ok(false);
        }
        if let Some(v) = &mut self.voice {
            if !v.poll(h, c)? {
                return Ok(false);
            }
            if v.wait {
                self.waiting_end = Some(voice::CHANNEL);
                self.mark_after_wait = Some(v.id);
            } else {
                r.mark_message(v.id)?;
            }
            self.voice = None;
        }
        if let Some(p) = &self.loading {
            let state = h.audio_preparation(c, p.ticket)?;
            if matches!(state.phase, AudioPreparePhase::Pending) {
                return Ok(false);
            }
            h.release_audio(c, p.ticket)?;
            let p = self
                .loading
                .take()
                .context("missing ExHibit sound request")?;
            if matches!(state.phase, AudioPreparePhase::Failed) {
                bail!("ExHibit sound preparation failed: {:?}", state.error)
            }
            if let Some(id) = p.seen {
                r.change(Change::Experience(Bank::Sound, vec![id]))?;
            }
            self.finish(p.channel, p.finish);
        }
        if let Some(fade) = &self.fade {
            let pending = match *fade {
                FadeWait::Start { channel, ms } => h.audio_info(c, channel)?.is_some_and(|a| {
                    a.age_frames < (u64::from(ms) * u64::from(a.output_rate)).div_ceil(1000)
                }),
                FadeWait::Stop(channel) => h
                    .audio_info(c, channel)?
                    .is_some_and(|a| a.stopping && a.fade_out_frames > 0),
            };
            if pending {
                return Ok(false);
            }
            self.fade = None;
        }
        if let Some(ch) = self.waiting_end {
            let mask = buttons::CTRL | buttons::ENTER | buttons::SPACE | buttons::MOUSE_LEFT;
            if (input.buttons_down | input.buttons_pressed) & mask == 0 && h.audio_playing(c, ch)? {
                return Ok(false);
            }
            h.submit_audio(
                c,
                AudioRequest::Stop {
                    channel: ch,
                    fade_ms: 0.,
                },
            )?;
            self.waiting_end = None;
            if let Some(id) = self.mark_after_wait.take() {
                r.mark_message(id)?;
            }
        }
        Ok(true)
    }
    pub fn issue(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        r: &Registers,
        worker: &TaskPool,
        s: Sound,
    ) -> Result<()> {
        const MUSIC: u32 = 0;
        const LOOP: u32 = 12;
        let (channel, id, path, bus, levels, base, fade, looping, finish, seen) = match s {
            Sound::Environment { id, fade, levels } => {
                return self
                    .environment
                    .play(h, c, r, id, fade, levels, &mut self.scene);
            }
            Sound::StopEnvironment(fade) => {
                self.fade = self
                    .environment
                    .stop(h, c, fade, &mut self.scene)?
                    .map(FadeWait::Stop);
                return Ok(());
            }
            Sound::Voice { id, levels, wait } => {
                if id >= 0 {
                    self.voice = Some(voice::Voice::begin(
                        h,
                        worker,
                        c,
                        id,
                        wait,
                        stereo(r, 38, levels)?,
                    )?);
                } else if wait {
                    self.waiting_end = Some(voice::CHANNEL);
                }
                return Ok(());
            }
            Sound::LeaveVoice(fade) => return self.stop(h, c, voice::CHANNEL, fade, fade == 0),
            Sound::Music {
                id,
                fade,
                levels,
                looping,
            } => {
                if id < 0 || self.music == Some(id) {
                    return Ok(());
                }
                self.scene[1..4].copy_from_slice(&[id, levels[0], levels[1]]);
                self.scene[0] = (self.scene[0] & !3) | i32::from(looping);
                if r.read(Value::System(31))? != 0 {
                    return Ok(());
                }
                let replace = self.music.is_some();
                (
                    MUSIC,
                    id,
                    format!("res/s/m/m{id:02}.ogg"),
                    PcmBus::Music,
                    levels,
                    32,
                    fade,
                    looping,
                    if replace && fade > 0 {
                        Finish::WaitFade(fade as u32)
                    } else {
                        Finish::Continue
                    },
                    Some(id),
                )
            }
            Sound::Loop { id, fade, levels } => {
                if id < 0 || self.loop_effect == Some(id) {
                    return Ok(());
                }
                self.scene[4..7].copy_from_slice(&[id, levels[0], levels[1]]);
                if r.read(Value::System(37))? != 0 {
                    self.loop_effect = Some(id);
                    return Ok(());
                }
                (
                    LOOP,
                    id,
                    format!("res/s/e/{id:03}.wav"),
                    PcmBus::Sfx,
                    levels,
                    38,
                    fade,
                    true,
                    if fade > 0 {
                        Finish::WaitFade(fade as u32)
                    } else {
                        Finish::Continue
                    },
                    None,
                )
            }
            Sound::Effect {
                id,
                slot,
                levels,
                wait,
            } => {
                if id < 0 || r.read(Value::System(37))? != 0 {
                    return Ok(());
                }
                let ch = effect_channel(slot);
                (
                    ch,
                    id,
                    format!("res/s/e/{id:03}.wav"),
                    PcmBus::Sfx,
                    levels,
                    38,
                    0,
                    false,
                    if wait {
                        Finish::WaitForEnd(ch)
                    } else {
                        Finish::Continue
                    },
                    None,
                )
            }
            Sound::StopMusic { fade, wait } => {
                self.scene[1..4].copy_from_slice(&[-1, 0, 0]);
                self.scene[0] &= !3;
                if self.music.take().is_some() {
                    self.stop(h, c, MUSIC, fade, wait)?;
                }
                return Ok(());
            }
            Sound::StopLoop(fade) => {
                self.scene[4..7].copy_from_slice(&[-1, 0, 0]);
                if self.loop_effect.take().is_some() {
                    self.stop(h, c, LOOP, fade, false)?;
                }
                return Ok(());
            }
            Sound::LeaveEffect { slot, fade } => {
                return self.stop(h, c, effect_channel(slot), fade, fade == 0);
            }
        };
        let parameters = AudioParameters {
            bus,
            looping,
            fade_ms: fade.max(0) as f32,
            replace_fade_ms: fade.max(0) as f32,
            pcm16_gain: Some(AudioPcm16Gain {
                stereo: stereo(r, base, levels)?,
                ..Default::default()
            }),
            ..Default::default()
        };
        let ticket = h.begin_audio(
            c,
            AudioRequest::PlayAsset {
                channel,
                source: AudioAsset {
                    path: VfsPath::new(Mount::Asset, &path)?,
                    frame_limit: None,
                },
                intro: None,
                parameters,
            },
        )?;
        if channel == MUSIC {
            self.music = Some(id)
        }
        if channel == LOOP {
            self.loop_effect = Some(id)
        }
        self.loading = Some(Pending {
            ticket,
            channel,
            finish,
            seen,
        });
        Ok(())
    }
    fn stop(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        channel: u32,
        fade: i32,
        wait: bool,
    ) -> Result<()> {
        if wait {
            self.waiting_end = Some(channel);
        } else if h.audio_playing(c, channel)? {
            h.submit_audio(
                c,
                AudioRequest::Stop {
                    channel,
                    fade_ms: fade.max(0) as f32,
                },
            )?;
            if fade > 0 {
                self.fade = Some(FadeWait::Stop(channel));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::native_state_tests::fresh;
    use crysirror_core::{
        service::{AudioPreparation, FileSystem},
        state::Command,
        Error,
    };
    #[derive(Default)]
    pub(crate) struct Host {
        pub(crate) requests: Vec<AudioRequest>,
        ready: bool,
        playing: bool,
        released: Vec<u64>,
    }
    impl FileSystem for Host {
        fn read(&mut self, _: ClientId, _: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
            Err(Error::NotFound)
        }
        fn write_atomic(
            &mut self,
            _: ClientId,
            _: &VfsPath,
            _: &[u8],
        ) -> crysirror_core::Result<()> {
            Err(Error::ReadOnly)
        }
    }
    impl HostServices for Host {
        fn submit(&mut self, _: ClientId, _: &[Command]) -> crysirror_core::Result<()> {
            Ok(())
        }
        fn now_ms(&self) -> u64 {
            0
        }
        fn audio_playing(&self, _: ClientId, _: u32) -> crysirror_core::Result<bool> {
            Ok(self.playing)
        }
        fn begin_audio(&mut self, _: ClientId, r: AudioRequest) -> crysirror_core::Result<u64> {
            self.requests.push(r);
            Ok(self.requests.len() as u64)
        }
        fn submit_audio(&mut self, _: ClientId, r: AudioRequest) -> crysirror_core::Result<()> {
            self.requests.push(r);
            Ok(())
        }
        fn audio_preparation(
            &mut self,
            _: ClientId,
            _: u64,
        ) -> crysirror_core::Result<AudioPreparation> {
            Ok(AudioPreparation {
                phase: if self.ready {
                    AudioPreparePhase::Ready
                } else {
                    AudioPreparePhase::Pending
                },
                channel: 0,
                error: None,
            })
        }
        fn release_audio(&mut self, _: ClientId, t: u64) -> crysirror_core::Result<()> {
            self.released.push(t);
            Ok(())
        }
    }
    #[test]
    fn bgm_publishes_native_stereo_then_marks_experience_only_once() -> Result<()> {
        let (c, mut h, mut a, mut r) = (ClientId(7), Host::default(), Audio::default(), fresh(1)?);
        let pool = TaskPool::new("exhibit-test", 1, 1)?;
        let cmd = decode(0x32, 0, &[5, 0, 256, 0, 1, 0])?;
        a.issue(&mut h, c, &r, &pool, cmd)?;
        let AudioRequest::PlayAsset {
            channel,
            source,
            parameters,
            ..
        } = &h.requests[0]
        else {
            bail!("expected music")
        };
        assert_eq!(*channel, 0);
        assert_eq!(source.path.relative(), "res/s/m/m05.ogg");
        assert_eq!(parameters.pcm16_gain.unwrap().stereo, [81, 0]);
        assert!(parameters.looping);
        assert!(!a.poll(&mut h, c, &mut r, InputFrame::default())?);
        assert_eq!(r.read(Value::Seen(Bank::Sound, 5))?, 0);
        h.ready = true;
        assert!(a.poll(&mut h, c, &mut r, InputFrame::default())?);
        a.issue(&mut h, c, &r, &pool, cmd)?;
        assert_eq!(h.released, [1]);
        assert_eq!(h.requests.len(), 1);
        assert_eq!(r.read(Value::Seen(Bank::Sound, 5))?, 1);
        Ok(())
    }
    #[test]
    fn blocking_effect_uses_the_selected_channel_and_stops_on_native_skip_input() -> Result<()> {
        let (c, mut h, mut a, mut r) = (ClientId(4), Host::default(), Audio::default(), fresh(1)?);
        let pool = TaskPool::new("exhibit-test", 1, 1)?;
        h.ready = true;
        h.playing = true;
        a.issue(&mut h, c, &r, &pool, decode(0x3a, 0, &[19, 256, 128, 1])?)?;
        assert!(!a.poll(&mut h, c, &mut r, InputFrame::default())?);
        assert!(a.poll(
            &mut h,
            c,
            &mut r,
            InputFrame {
                buttons_down: buttons::CTRL,
                ..Default::default()
            }
        )?);
        assert!(
            matches!(h.requests.last(),Some(AudioRequest::Stop{channel:15,fade_ms}) if *fade_ms==0.)
        );
        assert_eq!(effect_channel(-1), 14);
        Ok(())
    }
    #[test]
    fn four_lane_environment_uses_lane_defaults_and_only_the_base_fade() -> Result<()> {
        let mut e = crate::environment::Environment::default();
        let def = crate::environment::Definition::parse(
            b"7,18,2,0,144,144,16,0,0,256,256,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1",
        )?;
        e.define(def)?;
        let (c, mut h, r) = (ClientId(1), Host::default(), fresh(1)?);
        let mut scene = [0; 12];
        e.play(&mut h, c, &r, 7, 120, [256, -1], &mut scene)?;
        assert_eq!(&scene[7..10], &[7, 256, -1]);
        assert_eq!(h.requests.len(), 2);
        for (i, (ch, path, looping, fade, levels)) in [
            (4, "res/s/e/018.wav", true, 120., [256, 144]),
            (5, "res/s/e/016.wav", false, 0., [256, 256]),
        ]
        .into_iter()
        .enumerate()
        {
            let AudioRequest::PlayAsset {
                channel,
                source,
                parameters,
                ..
            } = &h.requests[i]
            else {
                bail!("expected lane source")
            };
            assert_eq!(
                (
                    *channel,
                    source.path.relative(),
                    parameters.looping,
                    parameters.fade_ms
                ),
                (ch, path, looping, fade)
            );
            assert_eq!(
                parameters.pcm16_gain.unwrap().stereo,
                stereo(&r, 35, levels)?
            );
        }
        Ok(())
    }
}

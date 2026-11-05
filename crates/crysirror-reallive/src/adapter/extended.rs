// SPDX-License-Identifier: MPL-2.0
// Operation meanings adopted from siglus_rs 256c398 crates/reallive/src/
// modules/{sound,msg,sys,sys_max,menu,shk,mod}.rs, pcm_event.rs (MPL-2.0).
//! The remaining original module operations mapped onto public engine
//! services: sound bookkeeping and PCM events (shared audio), text window
//! state (retained Layers), screen flashes (a retained solid Layer), message
//! boxes and text input boxes (shared UI widgets), interrupts and menu
//! returns (VM frames). Original opcode, value and wait semantics stay here;
//! no pixels are produced by the VM.
use super::*;
use crate::vm::{
    machine::{Frame, FrameKind},
    pcm_event::{self, Entry, Mode, PcmEvent},
    sound_state::{SoundState, WAV_CHANNELS},
};
use std::collections::BTreeSet;

/// Engine layer of the screen flash (`BOXFLUSH`, `RECTPIKACHU`, ...).
const FLASH_LAYER: u32 = 700_000;
/// UI ids of `MsgBox` buttons and title/message labels.
const MESSAGE_BOX: u32 = 720_000;
/// UI ids of `CreateInput` text boxes, by input index.
const TEXT_INPUT: u32 = 730_000;
/// Engine layers of window face images (`FACE_OPEN`), by window and slot.
const FACE_LAYER: u32 = 740_000;
/// Logical audio channel of PCM channel `n` (0..16).
pub(super) fn wav_channel(n: usize) -> u32 {
    100 + n.min(WAV_CHANNELS - 1) as u32
}

/// Original `graphics::Flash` (adopted).
#[derive(Debug, Clone)]
pub(super) struct Flash {
    pub area: Option<[i32; 4]>,
    pub colour: [u8; 3],
    pub start: u64,
    /// Length of one flash, in ms.
    pub time: u64,
    /// How many times it flashes.
    pub count: u32,
    /// Fades out instead of cutting off.
    pub fade: bool,
    /// Started by a `*PIKACHU*` (blink) command.
    pub blink: bool,
}

impl Flash {
    pub fn finished(&self, now: u64) -> bool {
        now >= self.start + self.time * u64::from(self.count.max(1))
    }

    /// The opacity at `now` (0 when off).
    pub fn alpha(&self, now: u64) -> u8 {
        if now < self.start || self.finished(now) || self.time == 0 {
            return 0;
        }
        let t = (now - self.start) % self.time;
        let blinking = self.count > 1;
        // Blinks are on for the first half of each period.
        let on = if blinking { self.time / 2 } else { self.time };
        if t >= on.max(1) {
            return 0;
        }
        if self.fade {
            (255.0 * (1.0 - t as f64 / on.max(1) as f64)) as u8
        } else {
            255
        }
    }
}

/// A `CreateInput` box (original `ui::TextInput`).
#[derive(Debug, Clone)]
pub(super) struct TextInput {
    pub widget: u32,
    pub text: String,
}

#[derive(Default)]
pub(super) struct Ext {
    /// The voice/PCM wait being prepared also ends on a click.
    pub pending_audio_key: bool,
    pub pcm_events: BTreeMap<i32, PcmEvent>,
    /// Audio preparations started by PCM events: (ticket, channel).
    pub event_tickets: Vec<(u64, u32)>,
    pub flash: Option<Flash>,
    flash_shown: bool,
    /// Text windows hidden by `ShowBackground`/`MESSAGE_ERASE`.
    pub text_hidden: bool,
    /// `SET_WINDOW_DISP_OFF` / `CCOM_MESSAGEWINDOW_OFF` windows.
    pub windows_off: BTreeSet<u32>,
    pub message_box: Vec<u32>,
    /// `G00BUF_LOAD`: preloaded images by buffer.
    pub g00_buffers: BTreeMap<i32, String>,
    pub inputs: BTreeMap<i32, TextInput>,
    /// Per window: open mod, open time, close mod, close time, enabled
    /// (`#WINDOW.nnn.OPEN_ANM_*`, `Get/Set*AnmMod/Time`).
    pub window_anm: BTreeMap<i32, [i32; 5]>,
    /// `FACE_OPEN(file, slot)` images by (window, slot).
    pub faces: BTreeMap<(u32, usize), String>,
    /// `CCOM_BTNSEL_OFF`, `CCOM_OBJECT_OFF` buffers.
    pub ccom_hide_buttons: bool,
    pub ccom_hidden_objects: BTreeSet<i32>,
    /// `MESSAGE_SPEED`: this message's speed.
    pub speed_override: Option<i32>,
    /// `\ruby` start pen per window.
    pub ruby_start: BTreeMap<u32, i32>,
    /// Indentation pen per window (`SetIndent`).
    pub indent: BTreeMap<u32, i32>,
    /// Operations the original also only notes (`note_unimplemented`).
    pub notes: BTreeMap<String, u64>,
    last_pointer: (i32, i32),
    /// `ShakeScreen*` / `ShakeLayers*` (shared VFX shake curves).
    pub screen_shake: Option<crysirror_vfx::shake::Shake>,
    pub layer_shake: Option<crysirror_vfx::shake::Shake>,
    shake_applied: bool,
    /// The global file as last written, and a frame counter for checks.
    pub global_written: Vec<u8>,
    global_check: u64,
}

/// Engine audio for the adopted PCM events (`pcm_event::System` seam).
struct EventAudio<'a> {
    vm: &'a mut RealLiveVm,
    m: &'a mut Machine,
    h: &'a mut dyn HostServices,
}

impl pcm_event::System for EventAudio<'_> {
    fn now(&self) -> u64 {
        self.m.sys.now()
    }
    fn random(&mut self, min: i32, max: i32) -> i32 {
        self.m.sys.random(min, max)
    }
    fn wav_playing(&mut self, channel: usize) -> bool {
        let logical = wav_channel(channel);
        self.vm.ext.event_tickets.iter().any(|(_, c)| *c == logical)
            || self.h.audio_playing(self.vm.client, logical).unwrap_or(false)
    }
    fn wav_play(&mut self, name: &str) -> Result<usize> {
        let now = self.m.sys.now();
        let channel = (0..WAV_CHANNELS)
            .find(|&c| !self.wav_playing(c))
            .unwrap_or(0);
        let ticket = self.vm.begin_wav(self.m, self.h, name, wav_channel(channel), false, 0., None)?;
        self.vm.ext.event_tickets.push((ticket, wav_channel(channel)));
        let track = self.m.sys.sound.track_mut(Some(channel));
        *track = crate::vm::sound_state::Track { name: name.to_owned(), looped: false, started: now, ..Default::default() };
        Ok(channel)
    }
    fn wav_stop(&mut self, channel: usize) {
        let _ = self.h.submit_audio(
            self.vm.client,
            AudioRequest::Stop { channel: wav_channel(channel), fade_ms: 0. },
        );
    }
}

fn ms(value: i32) -> u64 {
    value.max(0) as u64
}

impl RealLiveVm {
    /// Starts `name` (a `wav` resource) on a logical channel through the
    /// shared audio worker; returns the preparation ticket.
    pub(super) fn begin_wav(
        &mut self,
        m: &Machine,
        h: &mut dyn HostServices,
        name: &str,
        channel: u32,
        looping: bool,
        fade: f32,
        start: Option<(u64, Option<u64>)>,
    ) -> Result<u64> {
        let asset = self.resource(m, "wav", name)?;
        let index = (channel.saturating_sub(100)) as usize;
        let gain = settings_volume_ext(&m.sys.settings, crate::vm::settings::channel::PCM)
            * m.sys.sound.wav_volume[index.min(WAV_CHANNELS - 1)].value(m.sys.now()) as f32
            / 255.;
        let (start_ms, start_frame) = start.unwrap_or((0, None));
        h.begin_audio(
            self.client,
            AudioRequest::PlayAsset {
                channel,
                source: AudioAsset { path: VfsPath::new(Mount::Asset, &asset).map_err(err)?, frame_limit: None },
                intro: None,
                parameters: AudioParameters {
                    looping,
                    gain,
                    fade_ms: fade,
                    start_ms,
                    start_frame,
                    bus: PcmBus::Sfx,
                    ..Default::default()
                },
            },
        )
        .map_err(err)
    }

    /// Starts music `name` (`#DSTRACK` aware) on channel 0; the script waits
    /// for preparation like the first bridge's `bgmPlay`.
    fn begin_bgm(
        &mut self,
        m: &mut Machine,
        h: &mut dyn HostServices,
        name: &str,
        looped: bool,
        from: Option<(u64, Option<u64>)>,
        wait: bool,
    ) -> Result<()> {
        let mut name = name.to_owned();
        let mut loop_start = 0;
        let mut start_frame = None;
        let mut frame_limit = None;
        for e in m.gameexe.filter("DSTRACK") {
            let strings = e.strs();
            if strings.get(1).or(strings.first()).is_some_and(|s| s.eq_ignore_ascii_case(&name)) {
                let v = e.ints();
                name = strings[0].to_string();
                let from = v.first().copied().unwrap_or(0).max(0) as u64;
                start_frame = Some(from);
                loop_start = (v.get(2).copied().unwrap_or(0).max(0) as u64).max(from);
                frame_limit = v.get(1).copied().filter(|v| *v > 0).map(|v| v as u64);
                break;
            }
        }
        let (start_ms, explicit_frame) = from.unwrap_or((0, None));
        if let Some(f) = explicit_frame {
            start_frame = Some(start_frame.unwrap_or(0) + f);
        } else if start_ms > 0 {
            start_frame = None;
        }
        let asset = self.resource(m, "bgm", &name)?;
        let gain = settings_volume_ext(&m.sys.settings, crate::vm::settings::channel::BGM)
            * m.sys.sound.bgm_volume.value(m.sys.now()) as f32
            / 255.;
        let ticket = h
            .begin_audio(
                self.client,
                AudioRequest::PlayAsset {
                    channel: 0,
                    source: AudioAsset { path: VfsPath::new(Mount::Asset, &asset).map_err(err)?, frame_limit },
                    intro: None,
                    parameters: AudioParameters {
                        looping: looped,
                        loop_start,
                        start_frame,
                        start_ms,
                        clamp_range: true,
                        gain,
                        bus: PcmBus::Music,
                        ..Default::default()
                    },
                },
            )
            .map_err(err)?;
        let now = m.sys.now();
        m.sys.sound.bgm = crate::vm::sound_state::Track { name, looped, started: now, ..Default::default() };
        self.pending_audio = Some((ticket, wait.then_some(0)));
        self.poll_audio(h)?;
        Ok(())
    }

    fn set_gain(&mut self, h: &mut dyn HostServices, channel: u32, volume: i32, fade: u64) -> Result<()> {
        h.submit_audio(
            self.client,
            AudioRequest::SetGain { channel, gain: volume.clamp(0, 255) as f32 / 255., fade_ms: fade as f32 },
        )
        .map_err(err)
    }

    fn stop_channel(&mut self, h: &mut dyn HostServices, channel: u32, fade: u64) -> Result<()> {
        h.submit_audio(self.client, AudioRequest::Stop { channel, fade_ms: fade as f32 }).map_err(err)
    }

    /// The original `sound` module operations the first bridge did not map
    /// (adopted from `modules/sound.rs`). Returns false for the ones the
    /// first bridge handles.
    pub(super) fn sound_extended(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<bool> {
        let now = m.sys.now();
        let op = c.op.opcode;
        let channel_param = |m: &mut Machine, at: usize| -> Result<usize> {
            Ok(m.int_param(c, at)?.clamp(0, WAV_CHANNELS as i32 - 1) as usize)
        };
        match (c.op.module, op) {
            // Record what the first bridge starts, for the queries below.
            (20, 0..=2) => {
                let name = m.str_param(c, 0)?;
                m.sys.sound.bgm = crate::vm::sound_state::Track { name, looped: op != 2, started: now, ..Default::default() };
                return Ok(false);
            }
            (21, 0..=2) => {
                let name = m.str_param(c, 0)?;
                let channel = if Machine::param_count(c) >= 2 { channel_param(m, 1)? } else { 0 };
                *m.sys.sound.track_mut(Some(channel)) =
                    crate::vm::sound_state::Track { name, looped: op == 2, started: now, ..Default::default() };
                return Ok(false);
            }
            // bgmStop (both forms)
            (20, 5 | 6) => {
                self.stop_channel(h, 0, 0)?;
                m.sys.sound.bgm = Default::default();
            }
            // bgmStatus: 0 stopped, 1 playing, 2 fading out.
            (20, 7 | 107) => {
                let playing = h.audio_playing(self.client, 0).map_err(err)?;
                m.store = match playing {
                    true if SoundState::fading(&m.sys.sound.bgm, now) => 2,
                    true => 1,
                    false => 0,
                };
            }
            // bgmRewind: the track again from the start.
            (20, 8) => {
                let track = m.sys.sound.bgm.clone();
                if !track.name.is_empty() && h.audio_playing(self.client, 0).map_err(err)? {
                    self.begin_bgm(m, h, &track.name, track.looped, None, false)?;
                }
            }
            (20, 11) => m.store = m.sys.sound.bgm_volume.value(now),
            (20, 12) => {
                let volume = m.int_param(c, 0)?;
                let fade = ms(m.int_param_or(c, 1, 0)?);
                m.sys.sound.bgm_volume.retarget(now, volume, fade);
                self.set_gain(h, 0, volume, fade)?;
            }
            (20, 13 | 14) => {
                let fade = ms(m.int_param_or(c, 0, 0)?);
                let volume = if op == 13 { 255 } else { 0 };
                m.sys.sound.bgm_volume.retarget(now, volume, fade);
                self.set_gain(h, 0, volume, fade)?;
            }
            // MCHANGELOOP / MCHANGEONESHOT
            (20, 15 | 16) => {
                m.sys.sound.bgm.looped = op == 15;
                if h.audio_playing(self.client, 0).map_err(err)? {
                    h.submit_audio(self.client, AudioRequest::SetLooped { channel: 0, looping: op == 15, loop_start: None })
                        .map_err(err)?;
                }
            }
            // bgmFadeOut(time = 1000) / bgmFadeOutEx (waits)
            (20, 105 | 106) => {
                let fade = ms(m.int_param_or(c, 0, 1000)?);
                m.sys.sound.bgm.ends_at = Some(now + fade);
                self.stop_channel(h, 0, fade)?;
                if op == 106 {
                    self.wait = Some(Wait::Audio { channel: 0 });
                }
            }
            // bgmTimer: milliseconds since the track started.
            (20, 200) => m.store = m.sys.sound.bgm_elapsed(now),
            // MCHANGE(name): another track, looping as the current one does.
            (20, 300) => {
                let name = m.str_param(c, 0)?;
                let playing = h.audio_playing(self.client, 0).map_err(err)?;
                let looped = !playing || m.sys.sound.bgm.looped;
                self.begin_bgm(m, h, &name, looped, None, false)?;
            }
            // DEBUG_MPLAY(_WAIT, _ONESHOT)_MILLISECOND / _SECOND / _SAMPLE
            (20, 1000..=1002 | 2000..=2002 | 3000..=3002) => {
                let name = m.str_param(c, 0)?;
                let at = m.int_param_or(c, 1, 0)?.max(0) as u64;
                let from = match op / 1000 {
                    1 => (at, None),
                    2 => (at * 1000, None),
                    _ => (0, Some(at)),
                };
                let variant = op % 1000;
                self.begin_bgm(m, h, &name, variant == 0, Some(from), variant == 1)?;
            }
            // wavRewind(channel)
            (21, 8) => {
                let channel = channel_param(m, 0)?;
                let track = m.sys.sound.wav[channel].clone();
                if !track.name.is_empty() && h.audio_playing(self.client, wav_channel(channel)).map_err(err)? {
                    let ticket = self.begin_wav(m, h, &track.name, wav_channel(channel), track.looped, 0., None)?;
                    self.pending_audio = Some((ticket, None));
                    self.poll_audio(h)?;
                }
            }
            (21, 11) => {
                let channel = channel_param(m, 0)?;
                m.store = m.sys.sound.wav_volume[channel].value(now);
            }
            (21, 12) => {
                let channel = channel_param(m, 0)?;
                let volume = m.int_param(c, 1)?;
                let fade = ms(m.int_param_or(c, 2, 0)?);
                m.sys.sound.wav_volume[channel].retarget(now, volume, fade);
                self.set_gain(h, wav_channel(channel), volume, fade)?;
            }
            (21, 13 | 14) => {
                let channel = channel_param(m, 0)?;
                let fade = ms(m.int_param_or(c, 1, 0)?);
                let volume = if op == 13 { 255 } else { 0 };
                m.sys.sound.wav_volume[channel].retarget(now, volume, fade);
                self.set_gain(h, wav_channel(channel), volume, fade)?;
            }
            // PCMCHANGELOOP / PCMCHANGEONESHOT(channel)
            (21, 15 | 16) => {
                let channel = channel_param(m, 0)?;
                m.sys.sound.wav[channel].looped = op == 15;
                if h.audio_playing(self.client, wav_channel(channel)).map_err(err)? {
                    h.submit_audio(
                        self.client,
                        AudioRequest::SetLooped { channel: wav_channel(channel), looping: op == 15, loop_start: None },
                    )
                    .map_err(err)?;
                }
            }
            // PCMSTOPALLMAIN / PCMSTOPALLEXTRA: the first / second half.
            (21, 21 | 22) => {
                let half = WAV_CHANNELS / 2;
                let range = if op == 21 { 0..half } else { half..WAV_CHANNELS };
                for channel in range {
                    self.stop_channel(h, wav_channel(channel), 0)?;
                }
            }
            // PCMVOLSETALL(volume) / PCMVOLMAXALL(time) / PCMVOLMINALL(time)
            (21, 32..=34) => {
                let (volume, fade) = match op {
                    32 => (m.int_param(c, 0)?, 0),
                    33 => (255, ms(m.int_param_or(c, 0, 0)?)),
                    _ => (0, ms(m.int_param_or(c, 0, 0)?)),
                };
                for channel in 0..WAV_CHANNELS {
                    m.sys.sound.wav_volume[channel].retarget(now, volume, fade);
                    self.set_gain(h, wav_channel(channel), volume, fade)?;
                }
            }
            // PCMEVENT_LOOP / _RANDOM / _ONESHOT(event, (name, [weight,] min, max)...)
            (21, 40..=42) => {
                let number = m.int_param(c, 0)?;
                let mode = match op {
                    40 => Mode::Loop,
                    41 => Mode::Random,
                    _ => Mode::OneShot,
                };
                let mut entries = Vec::new();
                for index in 1..c.params.len() {
                    let pieces = m.complex_param(c, index)?;
                    let Some(name) = pieces.first() else { continue };
                    let name = m.eval_str(name)?;
                    let ints = pieces[1..].iter().map(|piece| m.eval_int(piece)).collect::<Result<Vec<_>>>()?;
                    let (weight, waits) = if mode == Mode::Random && ints.len() >= 3 {
                        (ints[0], &ints[1..])
                    } else {
                        (1, &ints[..])
                    };
                    entries.push(Entry {
                        name,
                        weight,
                        wait_min: waits.first().copied().unwrap_or(0),
                        wait_max: waits.get(1).copied().unwrap_or(0),
                    });
                }
                if let Some(mut old) = self.ext.pcm_events.remove(&number) {
                    old.stop(&mut EventAudio { vm: self, m, h }, true);
                }
                self.ext.pcm_events.insert(number, PcmEvent::new(mode, entries));
            }
            // PCMEVENT_STOP(event, stop sound) / PCMEVENT_STOPALL(stop sound)
            (21, 50 | 51) => {
                let (numbers, stop_sound) = if op == 50 {
                    let number = m.int_param(c, 0)?;
                    (vec![number], m.int_param_or(c, 1, 0)? != 0)
                } else {
                    (self.ext.pcm_events.keys().copied().collect(), m.int_param_or(c, 0, 0)? != 0)
                };
                for number in numbers {
                    if let Some(mut event) = self.ext.pcm_events.remove(&number) {
                        event.stop(&mut EventAudio { vm: self, m, h }, stop_sound);
                    }
                }
            }
            (21, 52) => {
                let number = m.int_param(c, 0)?;
                m.store = i32::from(self.ext.pcm_events.contains_key(&number));
            }
            (21, 53) => {
                let number = m.int_param(c, 0)?;
                self.wait = Some(Wait::PcmEvent(number));
            }
            // PCMBUF_LOAD / PCMBUF_FREE / PCMBUF_FREEALL: preloading; files
            // are read when played (as in the original).
            (21, 1000..=1002) => {}
            // wavFadeOut(channel[, time = 1000]) / wavStopEx
            (21, 105 | 106) => {
                let channel = channel_param(m, 0)?;
                let fade = ms(m.int_param_or(c, 1, 1000)?);
                m.sys.sound.wav[channel].ends_at = Some(now + fade);
                self.stop_channel(h, wav_channel(channel), fade)?;
            }
            // PCMFADECHECK(channel)
            (21, 107) => {
                let channel = channel_param(m, 0)?;
                m.store = i32::from(
                    h.audio_playing(self.client, wav_channel(channel)).map_err(err)?
                        && SoundState::fading(&m.sys.sound.wav[channel], now),
                );
            }
            // PCMPLAYWAITKEY(name, channel) / PCMWAITKEY(channel)
            (21, 201) => {
                let name = m.str_param(c, 0)?;
                let channel = channel_param(m, 1)?;
                *m.sys.sound.track_mut(Some(channel)) =
                    crate::vm::sound_state::Track { name: name.clone(), started: now, ..Default::default() };
                let ticket = self.begin_wav(m, h, &name, wav_channel(channel), false, 0., None)?;
                self.ext.pending_audio_key = true;
                self.pending_audio = Some((ticket, Some(wav_channel(channel))));
                self.poll_audio(h)?;
            }
            (21, 203) => {
                let channel = channel_param(m, 0)?;
                self.wait = Some(Wait::AudioKey { channel: wav_channel(channel) });
            }
            // SEVOLGET / SEVOLSET(volume) / SEVOLMAX(time) / SEVOLMIN(time)
            (22, 11) => m.store = m.sys.sound.se_volume.value(now),
            (22, 12..=14) => {
                let (volume, fade) = match op {
                    12 => (m.int_param(c, 0)?, 0),
                    13 => (255, ms(m.int_param_or(c, 0, 0)?)),
                    _ => (0, ms(m.int_param_or(c, 0, 0)?)),
                };
                m.sys.sound.se_volume.retarget(now, volume, fade);
                for channel in 32..48 {
                    self.set_gain(h, channel, volume, fade)?;
                }
            }
            (23, 11) => m.store = m.sys.sound.koe_volume.value(now),
            (23, 12) => {
                let volume = m.int_param(c, 0)?;
                let fade = ms(m.int_param_or(c, 1, 0)?);
                m.sys.sound.koe_volume.retarget(now, volume, fade);
                self.set_gain(h, 1, volume, fade)?;
            }
            (23, 13 | 14) => {
                let fade = ms(m.int_param_or(c, 0, 0)?);
                let volume = if op == 13 { 255 } else { 0 };
                m.sys.sound.koe_volume.retarget(now, volume, fade);
                self.set_gain(h, 1, volume, fade)?;
            }
            // koePlayExC / koeDoPlayExC: a click also ends the wait.
            (23, 7 | 10) => {
                self.ext.pending_audio_key = true;
                return Ok(false);
            }
            // KOE_DONT_MEMORY / KOEPLAYWAIT_ / KOEPLAYWAITKEY_ and the EX
            // forms: a voice not remembered for replay or the backlog.
            (23, 15..=20) => {
                let id = m.int_param(c, 0)?;
                self.stop_channel(h, 1, 0)?;
                let settings = &m.sys.settings;
                let muted = settings.koe_mode == 2 || !settings.enabled[crate::vm::settings::channel::KOE];
                let wait = (op - 15) % 3;
                if !m.sys.should_fast_forward() && !muted {
                    self.ext.pending_audio_key = wait == 2;
                    self.play_koe(m, h, id, wait != 0)?;
                }
            }
            // KOE_REPLAY_CLEAR: drop the voice-replay markers.
            (23, 101) => m.sys.sound.last_koe = (-1, 0),
            // SET_KOEFILEMODE(mode): which voice archives to read; any is read.
            (23, 100) => {
                let mode = m.int_param(c, 0)?;
                m.sys.remembered.insert(23100, mode);
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Per frame, before the VM runs: PCM events, the screen flash, input
    /// tables, interrupts (original `Engine::tick`).
    pub(super) fn tick_extended(&mut self, m: &mut Machine, h: &mut dyn HostServices, input: InputFrame) -> Result<()> {
        // Preparations started by events.
        let mut tickets = std::mem::take(&mut self.ext.event_tickets);
        tickets.retain(|(ticket, _)| match h.audio_preparation(self.client, *ticket) {
            Ok(status) if status.phase == AudioPreparePhase::Pending => true,
            _ => {
                let _ = h.release_audio(self.client, *ticket);
                false
            }
        });
        self.ext.event_tickets = tickets;
        if !self.ext.pcm_events.is_empty() {
            let mut events = std::mem::take(&mut self.ext.pcm_events);
            pcm_event::update_all(&mut events, &mut EventAudio { vm: self, m, h });
            // Events started while updating (none can be) are kept too.
            events.append(&mut self.ext.pcm_events);
            self.ext.pcm_events = events;
        }
        self.update_flash(h, m.sys.now())?;
        if self.ext.screen_shake.is_some() || self.ext.layer_shake.is_some() || self.ext.shake_applied {
            if m.sys.should_fast_forward() {
                self.ext.screen_shake = None;
                self.ext.layer_shake = None;
            }
            self.apply_shake(h, m.sys.now())?;
        }
        // Input state the adopted queries read.
        let pointer = (input.pointer_x, input.pointer_y);
        m.sys.input.mouse = pointer;
        if input.buttons_pressed & buttons::WHEEL_UP != 0 {
            m.sys.input.wheel -= 1;
        }
        if input.buttons_pressed & buttons::WHEEL_DOWN != 0 {
            m.sys.input.wheel += 1;
        }
        m.sys.input.keys_held.clear();
        for (code, bit) in KEY_CODES {
            if input.buttons_down & bit != 0 {
                m.sys.input.keys_held.insert(*code);
            }
            if input.buttons_pressed & bit != 0 {
                m.sys.input.keys_pressed.insert(*code);
            }
        }
        // Original input events of this frame (button edges, keys).
        {
            use crate::vm::sys_state::{Button, InputEvent, Key};
            let mut events = Vec::new();
            for (bit, button) in [(buttons::MOUSE_LEFT, Button::Left), (buttons::MOUSE_RIGHT, Button::Right)] {
                let down = input.buttons_down & bit != 0;
                let was = self.last_down & bit != 0;
                if (down && !was) || input.buttons_pressed & bit != 0 {
                    events.push(InputEvent::Press(button));
                }
                if !down && (was || input.buttons_pressed & bit != 0) {
                    events.push(InputEvent::Release(button));
                }
            }
            for (bit, key) in [(buttons::ESCAPE, Key::Escape), (buttons::ENTER, Key::Enter), (buttons::SPACE, Key::Space)] {
                if input.buttons_pressed & bit != 0 {
                    events.push(InputEvent::KeyDown(key));
                }
            }
            m.sys.input.events = events;
        }
        if let Some(mut polled) = m.sys.polled_buttons.take() {
            polled.update(&mut m.sys);
            m.sys.polled_buttons = Some(polled);
        }
        if let Some((x1, y1, x2, y2)) = m.sys.mouse_area {
            let clamped = (pointer.0.clamp(x1, x2.max(x1)), pointer.1.clamp(y1, y2.max(y1)));
            if clamped != pointer {
                m.sys.warp_cursor = Some(clamped);
            }
        }
        if let Some((x, y)) = m.sys.warp_cursor.take() {
            if self.ext.last_pointer != (x, y) {
                match h.warp_pointer(self.client, x, y) {
                    Ok(()) | Err(Error::Unsupported(_)) => {}
                    Err(e) => return Err(err(e)),
                }
            }
        }
        self.ext.last_pointer = pointer;
        // Text boxes: the engine widgets hold the edited text.
        for input in self.ext.inputs.values_mut() {
            if let Ok(bytes) = h.query_engine(self.client, EngineQuery::UiWidget(input.widget)) {
                if let Ok(value) = serde_json::from_slice::<Value>(&bytes) {
                    if let Some(text) = value["text"].as_str() {
                        input.text = text.to_owned();
                    }
                }
            }
        }
        // `SetInterrupt`: the handler runs at the start of a frame and
        // returns with `yield`. Here it runs only between engine waits, so
        // a handler never interleaves with an adapter-owned pending call.
        if self.wait.is_none() && self.pending_audio.is_none() && m.pending().is_none() {
            if let Some((scene, entrypoint)) = m.interrupt {
                if !m.in_interrupt && !m.interrupt_suspended && !m.halted {
                    match m.archive.scenario(scene) {
                        Ok(scenario) => {
                            let ip = scenario.entrypoint(entrypoint).unwrap_or(0);
                            m.stack.push(Frame::new(scenario, ip, FrameKind::Farcall));
                            m.in_interrupt = true;
                        }
                        Err(error) => {
                            m.report(format!("interrupt handler: {error:#}"));
                            m.interrupt = None;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn update_flash(&mut self, h: &mut dyn HostServices, now: u64) -> Result<()> {
        let Some(flash) = self.ext.flash.clone() else {
            if std::mem::take(&mut self.ext.flash_shown) {
                self.submit(h, json!({"layers":[{"op":"remove","id":FLASH_LAYER}]}))?;
            }
            return Ok(());
        };
        if flash.finished(now) {
            self.ext.flash = None;
            return self.update_flash(h, now);
        }
        let alpha = flash.alpha(now);
        let area = flash.area.unwrap_or([0, 0, self.width as i32, self.height as i32]);
        let [r, g, b] = flash.colour;
        self.submit(
            h,
            json!({"layers":[{"op":"put","layer":{"id":FLASH_LAYER,"parent":SCENE_ROOT,"rect":area,"order":[5000,0],"paint":{"kind":"solid","color":[r,g,b,255]},"opacity":alpha}}]}),
        )?;
        self.ext.flash_shown = true;
        Ok(())
    }

    /// The Box module (1:41): `BOXFLUSH`, `RECTFLUSH` (flash the screen or
    /// an area), their `FADE` forms, `BOXPIKACHU` / `RECTPIKACHU` (blink
    /// `cnt` times); `NEXT` forms do not wait. BOX areas are corners, RECT
    /// areas position and size. Adopted from `modules/mod.rs::flash`.
    pub(super) fn flash(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<()> {
        let opcode = c.op.opcode;
        let now = m.sys.now();
        // FLUSHSTOP / FLUSHWAIT / FLUSHCHECK, PIKACHUSTOP / _WAIT / _CHECK
        if matches!(opcode, 100..=102 | 110..=112) {
            let blink = opcode >= 110;
            let running = self.ext.flash.as_ref().is_some_and(|f| f.blink == blink && !f.finished(now));
            match opcode % 10 {
                0 => {
                    if running {
                        self.ext.flash = None;
                        self.update_flash(h, now)?;
                    }
                }
                1 => self.wait = Some(Wait::Flash),
                _ => m.store = i32::from(running),
            }
            return Ok(());
        }
        ensure!(opcode <= 33 && opcode % 10 <= 3, "unsupported native flash operation {}", c.op);
        let rect_form = opcode % 2 == 1;
        let wait = opcode % 10 < 2;
        let fade = matches!(opcode / 10, 1 | 3);
        let blink = opcode >= 20;
        let v = m.int_params_from(c, 0)?;
        let colour_at = if v.len() >= 7 { 4 } else { 0 };
        let area = (v.len() >= 7).then(|| {
            if rect_form {
                [v[0], v[1], v[2], v[3]]
            } else {
                [v[0].min(v[2]), v[1].min(v[3]), (v[2] - v[0]).abs() + 1, (v[3] - v[1]).abs() + 1]
            }
        });
        let byte = |i: usize| v.get(i).copied().unwrap_or(0).clamp(0, 255) as u8;
        let colour = [byte(colour_at), byte(colour_at + 1), byte(colour_at + 2)];
        let mut rest = v.get(colour_at + 3..).unwrap_or(&[]).iter().copied();
        let count = if blink { rest.next().unwrap_or(1).max(1) as u32 } else { 1 };
        let time = rest.next().unwrap_or(if blink { 100 } else { 50 }).max(1) as u64;
        self.ext.flash = Some(Flash { area, colour, start: now, time, count, fade, blink });
        self.update_flash(h, now)?;
        if wait {
            self.wait = Some(Wait::Time { until: now + time * u64::from(count), cancel: false });
        }
        Ok(())
    }

    /// `G00BUF_LOAD(buf, file)` / `G00BUF_FREE(buf)` / `G00BUF_FREEALL`
    /// (1:14): images held ahead of use. The engine image cache loads the
    /// resource now; the buffer remembers the name.
    pub(super) fn g00_buffers(&mut self, m: &mut Machine, c: &Command) -> Result<()> {
        match c.op.opcode {
            1000 => {
                let buf = m.int_param(c, 0)?;
                let name = m.str_param(c, 1)?;
                // Fails visibly like the original when the image is absent.
                m.sys.gfx.load_image(&name)?;
                self.ext.g00_buffers.insert(buf, name.trim().to_lowercase());
            }
            1001 => {
                let buf = m.int_param(c, 0)?;
                self.ext.g00_buffers.remove(&buf);
            }
            1002 => self.ext.g00_buffers.clear(),
            _ => bail!("unsupported native g00 buffer operation {}", c.op),
        }
        Ok(())
    }

    /// Debug module (1:255): messages for the original's debug window go to
    /// the diagnostic log (original `scr::debug`).
    pub(super) fn debug(&mut self, m: &mut Machine, c: &Command) -> Result<()> {
        let mut parts = Vec::new();
        for (i, p) in c.params.iter().enumerate() {
            if p.value.is_string() {
                parts.push(m.str_param(c, i)?);
            } else if let Ok(v) = m.int_param(c, i) {
                parts.push(v.to_string());
            }
        }
        let text = format!("debug {} {}", c.op.opcode, parts.join(" "));
        if std::env::var_os("CRYSIRROR_REALLIVE_TRACE").is_some() {
            eprintln!("[reallive {}:{}] {text}", m.scene_number(), m.line);
        }
        *self.ext.notes.entry(format!("debug {}", c.op.opcode)).or_default() += 1;
        Ok(())
    }

    /// The Os module (1:5): `shell` / `launch` of the game's own files (its
    /// manual) are offered to the host; the disc check is the first
    /// bridge's policy; `GET_COMPUTER_ID` is a fixed identity.
    pub(super) fn os(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<()> {
        match c.op.opcode {
            0 | 10 => {
                let name = m.str_param(c, 0)?.replace('\\', "/");
                let path = self.resources.as_ref().context("RealLive resources")?.borrow_mut().file("", &name)?;
                match path {
                    Some(path) => match h.open_document(self.client, &VfsPath::new(Mount::Asset, &path).map_err(err)?) {
                        Ok(()) => {}
                        // The original also only notes what it cannot open.
                        Err(Error::Unsupported(_)) => *self.ext.notes.entry(format!("shell {name}")).or_default() += 1,
                        Err(e) => return Err(err(e)),
                    },
                    None => *self.ext.notes.entry(format!("shell {name}")).or_default() += 1,
                }
            }
            120 => m.store = 1,
            _ => bail!("unsupported native os operation {}", c.op),
        }
        Ok(())
    }

    fn window_layers(&self, h: &mut dyn HostServices, window: Option<u32>) -> Result<Vec<u32>> {
        let mut ids = Vec::new();
        for w in 0..64u32 {
            if window.is_some_and(|x| x != w) {
                continue;
            }
            for i in 0..3 {
                let id = 10000 + w * 4 + i;
                if !self.query(h, EngineQuery::Layer(id))?.is_null() {
                    ids.push(id);
                }
            }
            for slot in 0..8u32 {
                let id = FACE_LAYER + w * 8 + slot;
                if self.ext.faces.contains_key(&(w, slot as usize)) && !self.query(h, EngineQuery::Layer(id))?.is_null() {
                    ids.push(id);
                }
            }
            let frame = 20000 + w * 8..20000 + w * 8 + 8;
            let nine = selection::NINE_SLICE_BASE + w * 16..selection::NINE_SLICE_BASE + w * 16 + 16;
            ids.extend(self.waku_ids.iter().copied().filter(|id| frame.contains(id) || nine.contains(id)));
        }
        Ok(ids)
    }

    /// Hides or shows every text window (frames, text, faces) without
    /// clearing them (original `text.hidden_temporarily`).
    pub(super) fn set_text_hidden(&mut self, h: &mut dyn HostServices, hidden: bool) -> Result<()> {
        self.ext.text_hidden = hidden;
        self.apply_window_visibility(h)
    }

    /// Applies `ShowBackground` and `SET_WINDOW_DISP_OFF` to the retained
    /// window layers.
    pub(super) fn apply_window_visibility(&mut self, h: &mut dyn HostServices) -> Result<()> {
        if !self.ext.text_hidden && self.ext.windows_off.is_empty() && !self.ext.flash_shown && self.ext.faces.is_empty() {
            // Nothing was hidden: every layer is already visible.
        }
        let mut ops = Vec::new();
        for w in 0..64u32 {
            let visible = !self.ext.text_hidden && !self.ext.windows_off.contains(&w);
            for id in self.window_layers(h, Some(w))? {
                ops.push(json!({"op":"set_visible","id":id,"visible":visible}));
            }
        }
        if !ops.is_empty() {
            self.submit(h, json!({ "layers": ops }))?;
        }
        Ok(())
    }

    /// `FACE_OPEN(file, slot)` / `FACE_CLOSE(slot)`: images shown in the
    /// active window at `#WINDOW.nnn.FACE.slot` (retained image Layers).
    fn face(&mut self, m: &mut Machine, h: &mut dyn HostServices, open: Option<String>, slot: usize) -> Result<()> {
        let window = self.window;
        let id = FACE_LAYER + window * 8 + slot as u32;
        match open {
            None => {
                if self.ext.faces.remove(&(window, slot)).is_some() && !self.query(h, EngineQuery::Layer(id))?.is_null() {
                    self.submit(h, json!({"layers":[{"op":"remove","id":id}]}))?;
                }
            }
            Some(file) => {
                let image = m.sys.gfx.load_image(&file)?;
                let prefix = format!("WINDOW.{window:03}.");
                let pos = m.gameexe.ints(&(prefix.clone() + "POS"));
                let face = m.gameexe.ints(&format!("{prefix}FACE.{slot:03}"));
                let bx = pos.get(1).copied().unwrap_or(0) + face.first().copied().unwrap_or(0);
                let by = pos.get(2).copied().unwrap_or(0) + face.get(1).copied().unwrap_or(0);
                let region = image.region(0);
                let w = region.width().max(1) as f32;
                let hgt = region.height().max(1) as f32;
                let size = [image.width.max(1) as f32, image.height.max(1) as f32];
                let uv = [
                    region.x1 as f32 / size[0],
                    region.y1 as f32 / size[1],
                    (region.x2 + 1) as f32 / size[0],
                    (region.y2 + 1) as f32 / size[1],
                ];
                let layer = json!({"id":id,"parent":SCENE_ROOT,"rect":[bx,by,w,hgt],"order":[1000,2],"paint":{"kind":"image","asset":image.asset},"uv":uv});
                self.submit(h, json!({"layers":[{"op":"put","layer":layer}]}))?;
                self.ext.faces.insert((window, slot), file);
                self.apply_window_visibility(h)?;
            }
        }
        Ok(())
    }

    /// Module 0:3 operations beyond the first bridge (adopted `msg.rs`).
    pub(super) fn message_extended(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<bool> {
        let window = self.window;
        match c.op.opcode {
            // SetFontColourAll / FontColour for every window.
            106 => self.color = m.int_param_or(c, 0, 0)?.max(0) as u32,
            // FontSizeAll([size])
            107 => {
                if c.params.is_empty() {
                    self.sizes.clear();
                } else {
                    let size = m.int_param(c, 0)?;
                    ensure!((1..=512).contains(&size), "native text size");
                    for w in 0..64 {
                        self.sizes.insert(w, size);
                    }
                }
            }
            // MESSAGE_SPEED(speed): this message's speed.
            108 => self.ext.speed_override = Some(m.int_param(c, 0)?),
            109 => m.sys.remembered.insert(30109, 1).map_or((), |_| ()),
            110 => m.sys.remembered.insert(30109, 0).map_or((), |_| ()),
            111 => m.store = window as i32,
            // MULTI_MESSAGE(window...): the text goes to the first window named.
            112 => {
                let w = m.int_param(c, 0)?;
                ensure!((0..64).contains(&w), "native text window");
                self.window = w as u32;
            }
            // `\ruby{base}={ruby}`: begin, then display the ruby text above.
            120 if Machine::param_count(c) == 0 => {
                let x = self.pen.get(&window).copied().unwrap_or([0, 0])[0];
                self.ext.ruby_start.insert(window, x);
            }
            120 => {
                let ruby = m.str_param(c, 0)?;
                if let Some(start) = self.ext.ruby_start.remove(&window) {
                    self.ruby(m, h, &ruby, start)?;
                }
            }
            // msgHideAllTemp: hide until a click.
            170 => {
                self.set_text_hidden(h, true)?;
                self.wait = Some(Wait::ShowBackground);
            }
            // SetIndent / ClearIndent
            300 => {
                let x = self.pen.get(&window).copied().unwrap_or([0, 0])[0];
                self.ext.indent.insert(window, x);
            }
            301 => {
                self.ext.indent.remove(&window);
            }
            // TextPos(x, y) / TextPosX / TextPosY, TextOffset (relative).
            310 | 311 | 312 | 320 | 321 | 322 => {
                let luby = self.window_geometry(m, window).luby;
                let a = m.int_param(c, 0)?;
                let b = if matches!(c.op.opcode, 310 | 320) { m.int_param(c, 1)? } else { 0 };
                let pen = self.pen.entry(window).or_insert([0, 0]);
                match c.op.opcode {
                    310 => *pen = [a, b + luby],
                    311 => pen[0] = a,
                    312 => pen[1] = a + luby,
                    320 => *pen = [pen[0] + a, pen[1] + b],
                    321 => pen[0] += a,
                    _ => pen[1] += a,
                }
            }
            // GetTextPos(x, y) / GetTextPos(y)
            330 => {
                let luby = self.window_geometry(m, window).luby;
                let [x, y] = self.pen.get(&window).copied().unwrap_or([0, 0]);
                if Machine::param_count(c) >= 2 {
                    let tx = m.int_target_param(c, 0)?;
                    let ty = m.int_target_param(c, 1)?;
                    m.set_target(tx, x)?;
                    m.set_target(ty, y - luby)?;
                } else {
                    let ty = m.int_target_param(c, 0)?;
                    m.set_target(ty, y - luby)?;
                }
            }
            // Characters shown in a window / all windows.
            340 => {
                let w = m.int_param_or(c, 0, window as i32)?;
                m.store = self.glyph_count(h, w.max(0) as u32)?;
            }
            341 => {
                let mut total = 0;
                for w in 0..64 {
                    total += self.glyph_count(h, w)?;
                }
                m.store = total;
            }
            // SYSBTN_OPEN / _CLOSE and the button pattern switches: the
            // original's floating system buttons, remembered as set.
            400 | 401 => {
                m.sys.remembered.insert(30400, i32::from(c.op.opcode == 400));
            }
            402..=409 | 502..=509 => {
                let opcode = c.op.opcode;
                let base: u16 = if opcode >= 500 { 30500 } else { 30401 };
                match opcode % 100 {
                    2 | 4 | 6 => {
                        let key = base + (opcode % 100 - 2) / 2;
                        m.store = m.sys.remembered.get(&key).copied().unwrap_or(0);
                    }
                    3 | 5 => {
                        let value = m.int_param(c, 0)?;
                        m.sys.remembered.insert(base + (opcode % 100 - 3) / 2, value);
                    }
                    7 => {
                        m.sys.remembered.insert(base + 2, 0);
                    }
                    8 => {
                        m.sys.remembered.insert(base + 2, 1);
                    }
                    _ => {
                        let value = m.int_param(c, 0)?;
                        m.sys.remembered.insert(base + 2, value);
                    }
                }
            }
            1000 => {
                let file = m.str_param(c, 0)?;
                let slot = m.int_param_or(c, 1, 0)?.clamp(0, 7) as usize;
                self.face(m, h, Some(file), slot)?;
            }
            1001 => {
                let slot = m.int_param_or(c, 0, 0)?.clamp(0, 7) as usize;
                self.face(m, h, None, slot)?;
            }
            // FLAGINDEX(var): a debugging aid of the original's window.
            2000 => {}
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn glyph_count(&self, h: &mut dyn HostServices, window: u32) -> Result<i32> {
        let v = self.query(h, EngineQuery::Layer(10000 + window * 4))?;
        Ok(v["paint"]["glyphs"].as_array().map_or(0, |g| g.len() as i32))
    }

    /// Ruby text (`\ruby`) above the characters from `start` to the pen,
    /// in a separate retained glyph run of the window.
    fn ruby(&mut self, m: &mut Machine, h: &mut dyn HostServices, ruby: &str, start: i32) -> Result<()> {
        let window = self.window;
        let g = self.window_geometry(m, window);
        let size = (g.luby.max(1)).min(512);
        let [end, y] = self.pen.get(&window).copied().unwrap_or([0, 0]);
        let count = ruby.chars().count().max(1) as i32;
        let width = (end - start).max(size * count);
        let step = width / count;
        let id = 10000 + window * 4 + 2;
        let mut glyphs = match self.query(h, EngineQuery::Layer(id))? {
            Value::Null => Vec::new(),
            v => v["paint"]["glyphs"].as_array().cloned().unwrap_or_default(),
        };
        let metrics = h.font_metrics(&self.config.font_alias, size as f32, '字').map_err(err)?;
        let colour = super::commands::colour(&m.gameexe, self.color as i32);
        for (i, ch) in ruby.chars().enumerate() {
            let x = g.x + start + i as i32 * step + (step - size) / 2;
            let baseline = g.y + y - g.luby + metrics.ascent.round() as i32;
            glyphs.push(json!({"character":ch.to_string(),"baseline":[x,baseline],"size":size,"color":colour,"bold":false}));
        }
        let layer = json!({"id":id,"parent":SCENE_ROOT,"rect":[0,0,self.width,self.height],"order":[1000,1],"paint":{"kind":"glyph_run","font":self.config.font_alias,"size":size,"color":colour,"glyphs":glyphs}});
        self.submit(h, json!({"layers":[{"op":"put","layer":layer}]}))
    }

    /// `MsgBox` family: an engine dialog of shared UI widgets.
    fn message_box(&mut self, h: &mut dyn HostServices, title: &str, text: &str, buttons: &[(&str, i32)]) -> Result<()> {
        let w = self.width as i32;
        let hgt = self.height as i32;
        let (bw, bh) = (w * 3 / 5, hgt / 3);
        let (x, y) = ((w - bw) / 2, (hgt - bh) / 2);
        let mut ops = vec![
            json!({"op":"put","widget":{"id":MESSAGE_BOX,"kind":"label","rect":[x,y,bw,32],"text":title,"font":self.config.font_alias,"text_size":20,"z":2000}}),
            json!({"op":"put","widget":{"id":MESSAGE_BOX+1,"kind":"label","rect":[x,y+40,bw,bh-90],"text":text,"font":self.config.font_alias,"text_size":20,"z":2000}}),
        ];
        let mut ids = Vec::new();
        let mut values = Vec::new();
        let n = buttons.len() as i32;
        for (i, (label, value)) in buttons.iter().enumerate() {
            let id = MESSAGE_BOX + 10 + i as u32;
            let bx = x + (bw / n) * i as i32 + 8;
            ops.push(json!({"op":"put","widget":{"id":id,"kind":"button","rect":[bx,y+bh-44,bw/n-16,36],"text":label,"font":self.config.font_alias,"text_size":20,"z":2000}}));
            ids.push(id);
            values.push(*value);
        }
        self.ext.message_box = std::iter::once(MESSAGE_BOX).chain(std::iter::once(MESSAGE_BOX + 1)).chain(ids.iter().copied()).collect();
        self.submit(h, json!({ "ui": ops }))?;
        self.wait = Some(Wait::MessageBox { ids, values });
        Ok(())
    }

    pub(super) fn close_message_box(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let ops: Vec<Value> = std::mem::take(&mut self.ext.message_box).into_iter().map(|id| json!({"op":"remove","id":id})).collect();
        self.submit(h, json!({ "ui": ops }))
    }

    /// `ReturnMenu` / `MenuReturn`: restart at `#SEEN_MENU` with fresh local
    /// state (adopted `menu::return_to_menu`); the screen, windows and
    /// sounds are the engine's and are cleared through its services.
    fn return_to_menu(&mut self, m: &mut Machine, h: &mut dyn HostServices) -> Result<()> {
        let scene = m
            .gameexe
            .int("SEEN_MENU")
            .or_else(|| m.gameexe.int("SEEN_START"))
            .or_else(|| m.archive.first())
            .unwrap_or(1);
        let scenario = m.archive.scenario(scene)?;
        let ip = scenario.entrypoint(0).unwrap_or(0);
        // reset_for_load: long operations, interrupts and presentation.
        m.clear_long_ops();
        m.in_interrupt = false;
        self.hide_text(h)?;
        for (_, w) in std::mem::take(&mut self.ext.faces) {
            let _ = w;
        }
        let mut gfx = std::mem::take(&mut m.sys.gfx);
        let loader = gfx.loader.take();
        m.sys.gfx = crate::vm::graphics::Gfx { loader, width: gfx.width, height: gfx.height, ..Default::default() };
        m.memory.local = Default::default();
        m.init_local_memory();
        m.stack = vec![Frame::new(scenario, ip, FrameKind::Root)];
        m.halted = false;
        m.mark_savepoint();
        Ok(())
    }

    /// Module 1:4 operations beyond the first bridge whose effect is an
    /// engine service (adopted `sys.rs`/`sys_max.rs`).
    pub(super) fn system_extended(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<Option<Next>> {
        let now = h.now_ms();
        let opcode = c.op.opcode;
        match opcode {
            // TIMEWAITKEY_NOMOUSECLEAR / DONTJUMP_TIMEWAIT(KEY)(_NOMOUSECLEAR)
            102..=105 => {
                let time = ms(m.int_param(c, 0)?);
                self.wait = Some(Wait::Time { until: now + time, cancel: opcode != 103 });
            }
            // KEYWAIT_NOMOUSECLEAR
            107 => self.wait = Some(Wait::Click { record: None }),
            // CmpTimer / CmpExTimer, SetTimer / SetExTimer
            115 | 125 | 116 | 126 => {
                let layer = usize::from(opcode >= 120);
                let time = m.int_param(c, 0)?;
                let i = m.int_param_or(c, 1, 0)?;
                ensure!((0..256).contains(&i), "timer index");
                if opcode % 10 == 5 {
                    let value = now.saturating_sub(self.timers[layer][i as usize]).min(i32::MAX as u64) as i32;
                    m.store = i32::from(value > time);
                } else {
                    self.timers[layer][i as usize] = now.saturating_sub(time.max(0) as u64);
                }
            }
            // AUTOMODEWAIT...: in auto mode, the pause auto mode gives.
            180..=185 => {
                if m.sys.settings.auto_mode {
                    let s = &m.sys.settings;
                    let time = s.auto_base_time.max(0) as u64;
                    self.wait = Some(Wait::Time { until: now + time, cancel: matches!(opcode, 181 | 182 | 184 | 185) });
                } else {
                    m.store = 0;
                }
            }
            // SET/GET_WAIP_WINDOWCLOSE_MOD, SET/GET_GRPCOM_WINDOWCLOSE_MOD
            212 => self.wipe_closes = m.int_param(c, 0)? != 0,
            213 => m.store = i32::from(self.wipe_closes),
            217 => self.grp_closes = m.int_param(c, 0)? != 0,
            218 => m.store = i32::from(self.grp_closes),
            // SET_WINDOW_WAKUMOD(window, mod) / SET_WINDOW_ATTRMOD(window, mod)
            417 | 428 => {
                let w = m.int_param(c, 0)?;
                let value = m.int_param(c, 1)?;
                m.sys.remembered.insert(31000 + (w.clamp(0, 63) as u16) * 2 + u16::from(opcode == 428), value);
            }
            // EnableWindowAnm / DisableWindowAnm([window])
            460 | 461 => {
                for w in self.window_range(m, c)? {
                    self.window_anm_mut(m, w)[4] = i32::from(opcode == 460);
                }
            }
            // Get/Set Open/Close AnmMod/AnmTime(window[, value])
            462..=469 => {
                let w = m.int_param(c, 0)?;
                let field = usize::from((opcode - 462) / 2);
                if opcode % 2 == 1 {
                    let value = m.int_param(c, 1)?;
                    self.window_anm_mut(m, w)[field] = value;
                } else {
                    m.store = self.window_anm_mut(m, w)[field];
                }
            }
            // INIT_WINDOW_OPEN_ANIME / _CLOSE_ANIME([window]); GET/SET_WINDOW_ANIME_MOD
            470 | 471 => {
                let windows: Vec<i32> = if c.params.is_empty() { (0..64).collect() } else { vec![m.int_param(c, 0)?] };
                for w in windows {
                    let fresh = window_anm_from_gameexe(&m.gameexe, w);
                    let entry = self.window_anm_mut(m, w);
                    if opcode == 470 {
                        entry[0] = fresh[0];
                        entry[1] = fresh[1];
                    } else {
                        entry[2] = fresh[2];
                        entry[3] = fresh[3];
                    }
                }
            }
            472 => {
                let w = m.int_param(c, 0)?;
                m.store = self.window_anm_mut(m, w)[4];
            }
            473 => {
                let w = m.int_param(c, 0)?;
                let value = m.int_param(c, 1)?;
                self.window_anm_mut(m, w)[4] = i32::from(value != 0);
            }
            // SET_WINDOW_DISP_ON / _OFF([window])
            480 | 481 => {
                for w in self.window_range(m, c)? {
                    if opcode == 481 {
                        self.ext.windows_off.insert(w as u32);
                    } else {
                        self.ext.windows_off.remove(&(w as u32));
                    }
                }
                self.apply_window_visibility(h)?;
            }
            // MESSAGE_ERASE(_CHECK): hide the windows until a click.
            750 => {
                self.set_text_hidden(h, true)?;
                self.wait = Some(Wait::ShowBackground);
            }
            751 => m.store = i32::from(self.ext.text_hidden),
            // KOE_REPLAY / KOE_REPLAY_CHECK
            795 => {
                let (koe, _) = m.sys.sound.last_koe;
                if koe >= 0 && !m.sys.should_fast_forward() {
                    self.play_koe(m, h, koe, false)?;
                }
            }
            796 => m.store = i32::from(h.audio_playing(self.client, 1).map_err(err)?),
            // MenuReturn / MenuReturn2 / ReturnMenu
            1201..=1203 => {
                if opcode != 1203 {
                    for channel in [0u32, 1].into_iter().chain(100..116).chain(32..48) {
                        self.stop_channel(h, channel, 0)?;
                    }
                }
                self.return_to_menu(m, h)?;
                return Ok(Some(Next::Jumped));
            }
            // Text input boxes (adopted `menu::text_input`).
            1700..=1711 => self.text_input(m, h, c)?,
            1400..=1414 | 1420 | 1421 | 1423..=1425 | 1459 => {
                self.save_query(m, h, c)?;
            }
            _ => return Ok(None),
        }
        Ok(Some(Next::Advance))
    }

    fn window_range(&self, m: &mut Machine, c: &Command) -> Result<Vec<i32>> {
        let v = m.int_params_from(c, 0)?;
        Ok(match v[..] {
            [] => (0..64).collect(),
            [one] => vec![one.clamp(0, 63)],
            [first, last, ..] => (first.clamp(0, 63)..=last.clamp(0, 63)).collect(),
        })
    }

    fn window_anm_mut(&mut self, m: &Machine, w: i32) -> &mut [i32; 5] {
        let w = w.clamp(0, 63);
        self.ext.window_anm.entry(w).or_insert_with(|| window_anm_from_gameexe(&m.gameexe, w))
    }

    /// `CreateInput`, `CloseInput`, `CloseAllInputs`, `FocusInput`,
    /// `SetInput`, `GetInput`: shared UI text fields.
    fn text_input(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<()> {
        match c.op.opcode {
            1700 => {
                let values = m.int_params_from(c, 0)?;
                let index = values.first().copied().unwrap_or(0);
                let v = |i: usize| values.get(i).copied().unwrap_or(0);
                let (x, y) = (v(1), v(2));
                let (w, hgt) = (v(3).max(16), v(4).max(16));
                let size = if v(5) > 0 { v(5) } else { (hgt - 4).clamp(8, 64) };
                let widget = TEXT_INPUT + index.clamp(0, 999) as u32;
                let text = self.ext.inputs.get(&index).map(|i| i.text.clone()).unwrap_or_default();
                self.submit(h, json!({"ui":[{"op":"put","widget":{"id":widget,"kind":"text_field","rect":[x,y,w,hgt],"text":text,"font":self.config.font_alias,"text_size":size,"z":1500}}]}))?;
                self.ext.inputs.insert(index, TextInput { widget, text });
            }
            1701 => {
                let index = m.int_param(c, 0)?;
                if let Some(input) = self.ext.inputs.remove(&index) {
                    self.submit(h, json!({"ui":[{"op":"remove","id":input.widget}]}))?;
                }
            }
            1702 => {
                let ops: Vec<Value> = std::mem::take(&mut self.ext.inputs).into_values().map(|i| json!({"op":"remove","id":i.widget})).collect();
                self.submit(h, json!({ "ui": ops }))?;
            }
            1703 => {
                let index = m.int_param(c, 0)?;
                if let Some(input) = self.ext.inputs.get(&index) {
                    let id = input.widget;
                    self.submit(h, json!({"ui":[{"op":"focus","id":id}]}))?;
                }
            }
            1710 => {
                let index = m.int_param(c, 0)?;
                let text = m.str_param(c, 1)?;
                if let Some(input) = self.ext.inputs.get_mut(&index) {
                    input.text = text.clone();
                    let id = input.widget;
                    self.submit(h, json!({"ui":[{"op":"set_text","id":id,"text":text}]}))?;
                }
            }
            1711 => {
                let index = m.int_param(c, 0)?;
                let target = m.str_target_param(c, 1)?;
                let text = self.ext.inputs.get(&index).map(|i| i.text.clone()).unwrap_or_default();
                m.write_string(target, text)?;
            }
            _ => bail!("unsupported native text input operation {}", c.op),
        }
        Ok(())
    }

    /// Module 0:4 operations whose effect is an engine service.
    pub(super) fn event_loop_extended(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<Next> {
        let opcode = c.op.opcode;
        match opcode {
            // PAGE: wait, then start a new page.
            163 => {
                self.toolbar.pause_started = None;
                self.wait = Some(Wait::Pause { kind: 210, window: self.window });
            }
            // CANCELCALL: the right-click routine, as if right-clicked.
            200 => {
                if let Some((scene, entrypoint)) = cancel_call(m) {
                    let scenario = m.archive.scenario(scene)?;
                    let ip = scenario.entrypoint(entrypoint).unwrap_or(0);
                    m.advance();
                    m.stack.push(Frame::new(scenario, ip, FrameKind::Farcall));
                    return Ok(Next::Jumped);
                }
            }
            201 => m.store = i32::from(cancel_call(m).is_some()),
            // MPLAY(name): music.
            289 => {
                let name = m.str_param(c, 0)?;
                self.begin_bgm(m, h, &name, true, None, false)?;
            }
            // PCMPLAY(name, channel)
            371 => {
                let name = m.str_param(c, 0)?;
                let channel = m.int_param_or(c, 1, 0)?.clamp(0, WAV_CHANNELS as i32 - 1) as usize;
                let ticket = self.begin_wav(m, h, &name, wav_channel(channel), false, 0., None)?;
                self.pending_audio = Some((ticket, None));
                self.poll_audio(h)?;
            }
            // MsgBox(title, text) with OK, Yes/No, Yes/No/Cancel buttons.
            400..=403 => {
                let title = m.str_param(c, 0)?;
                let message = match c.params.get(1) {
                    Some(param) if param.value.is_string() => m.str_param(c, 1)?,
                    Some(_) => m.int_param(c, 1)?.to_string(),
                    None => String::new(),
                };
                let buttons: &[(&str, i32)] = match opcode {
                    400 => &[("OK", 1)],
                    401 if c.op.overload == 1 => &[("OK", 1), ("キャンセル", 0)],
                    401 => &[("OK", 1)],
                    402 => &[("はい", 1), ("いいえ", 0)],
                    _ => &[("はい", 1), ("いいえ", 0), ("キャンセル", -1)],
                };
                self.message_box(h, &title, &message, buttons)?;
            }
            // ShowBackground: hide the text windows until a click.
            1000 => {
                self.set_text_hidden(h, true)?;
                self.wait = Some(Wait::ShowBackground);
            }
            // CCOM_MESSAGEWINDOW_ON / _OFF([window | first, last])
            1200 | 1201 => {
                for w in self.window_range(m, c)? {
                    if opcode == 1201 {
                        self.ext.windows_off.insert(w as u32);
                    } else {
                        self.ext.windows_off.remove(&(w as u32));
                    }
                }
                self.apply_window_visibility(h)?;
            }
            // CCOM_BTNSEL_ON / _OFF
            1202 => self.ext.ccom_hide_buttons = false,
            1203 => self.ext.ccom_hide_buttons = true,
            // CCOM_OBJECT_ON / _OFF(buffer | first, last)
            1204 | 1205 => {
                let first = m.int_param(c, 0)?;
                let last = m.int_param_or(c, 1, first)?;
                for index in first..=last {
                    if opcode == 1205 {
                        self.ext.ccom_hidden_objects.insert(index);
                    } else {
                        self.ext.ccom_hidden_objects.remove(&index);
                    }
                }
            }
            // CCOM_SET_WINDOW_EXBTN_ON / _OFF (_NOW, _SAVEPOINT)([button])
            2010..=2015 => {
                let on = opcode % 2 == 0;
                let ids: Vec<i32> = if c.params.is_empty() { (0..8).collect() } else { vec![m.int_param(c, 0)?] };
                for b in ids {
                    m.sys.remembered.insert(4600 + b.clamp(0, 9) as u16, i32::from(on));
                }
            }
            _ => bail!("unsupported native event-loop operation {}", c.op),
        }
        Ok(Next::Advance)
    }
}


impl RealLiveVm {
    /// Screen shaking (module 1:13) and layer shaking (module 1:12), adopted
    /// from `modules/shk.rs`. Both share numbering: 1xxx starts a shake,
    /// 3xxx starts it and waits; xx100-xx203 are motions, xx102 is 2D,
    /// xx103 a circle (layers), xx300 uses `#SHAKE`.
    pub(super) fn shake(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<()> {
        use crysirror_vfx::shake::{Layers, Motion, Shake, ShakeKind};
        let layered = c.op.module == 12;
        let opcode = c.op.opcode;
        let now = m.sys.now();
        if opcode == 0 {
            // ShakeStop([time]) / ShakeLayersStop([time][, flag])
            let time = m.int_param_or(c, 0, 0)?.max(0) as u64;
            let slot = if layered { &mut self.ext.layer_shake } else { &mut self.ext.screen_shake };
            match slot {
                Some(shake) if time > 0 => shake.stop = Some((now, time)),
                _ => *slot = None,
            }
            return self.apply_shake(h, now);
        }
        let wait = opcode >= 3000;
        let sub = opcode % 1000;
        let mut index = 0;
        let mut next = |m: &mut Machine| -> Result<i32> {
            let value = m.int_param(c, index)?;
            index += 1;
            Ok(value)
        };
        let motion_of = |opcode: u16| -> Option<Motion> {
            Some(match opcode {
                100 => Motion::DownUp,
                101 => Motion::RightLeft,
                200 => Motion::Up,
                201 => Motion::Down,
                202 => Motion::Left,
                203 => Motion::Right,
                400 => Motion::Zoom,
                _ => return None,
            })
        };
        let kind = match sub {
            103 if layered => ShakeKind::Circle {
                h_amount: next(m)?,
                h_speed: next(m)?,
                v_amount: next(m)?,
                v_speed: next(m)?,
            },
            102 => ShakeKind::TwoD {
                h_amount: next(m)?,
                h_speed: next(m)?,
                v_amount: next(m)?,
                v_speed: next(m)?,
            },
            300 => {
                let spec = next(m)?;
                shake_pattern(&m.gameexe, spec)
            }
            other => match motion_of(other) {
                Some(motion) => ShakeKind::Single { motion, amount: next(m)?, speed: next(m)? },
                None => bail!("unsupported native shake operation {}", c.op),
            },
        };
        let rep = next(m)?;
        let faderep = next(m)?;
        let layers = if layered {
            Layers {
                window: next(m)? != 0,
                text: next(m)? != 0,
                background: next(m)? != 0,
                objects: next(m)? != 0,
            }
        } else {
            Layers::ALL
        };
        let shake = Shake::new(kind, now, rep, faderep, layers);
        if layered {
            self.ext.layer_shake = Some(shake);
        } else {
            self.ext.screen_shake = Some(shake);
        }
        self.apply_shake(h, now)?;
        if wait && (rep != 0 || faderep != 0) {
            self.wait = Some(Wait::ShakeEnd { layer: layered });
        }
        Ok(())
    }

    /// Moves the scene root by the running shakes (original
    /// `Graphics::shake_offsets` applied by the compositor; here the shared
    /// Layer transform moves the retained scene).
    pub(super) fn apply_shake(&mut self, h: &mut dyn HostServices, now: u64) -> Result<()> {
        let mut total = crysirror_vfx::shake::Offset::default();
        for slot in [&mut self.ext.screen_shake, &mut self.ext.layer_shake] {
            if let Some(shake) = slot.as_ref() {
                match shake.offset(now) {
                    Some(o) => {
                        total.dx += o.dx;
                        total.dy += o.dy;
                        total.zoom += o.zoom;
                    }
                    None => *slot = None,
                }
            }
        }
        let active = self.ext.screen_shake.is_some() || self.ext.layer_shake.is_some();
        if !active && !self.ext.shake_applied {
            return Ok(());
        }
        let (w, hgt) = (self.width as f32, self.height as f32);
        let scale = 1.0 + total.zoom as f32;
        let layer = json!({"id":SCENE_ROOT,"rect":[total.dx.round() as f32,total.dy.round() as f32,w,hgt],"paint":{"kind":"solid","color":[0,0,0,0]},"transform":{"scale_x":scale,"scale_y":scale,"scale_z":1.0,"pivot_x":w/2.,"pivot_y":hgt/2.}});
        self.submit(h, json!({"layers":[{"op":"put","layer":layer}]}))?;
        self.ext.shake_applied = active;
        Ok(())
    }
}

/// `#SHAKE.spec` (or `#SHAKE.000`) as `x, y, wait` steps.
fn shake_pattern(exe: &Gameexe, spec: i32) -> crysirror_vfx::shake::ShakeKind {
    let values = {
        let own = exe.ints(&format!("SHAKE.{spec:03}"));
        if own.is_empty() { exe.ints("SHAKE.000") } else { own }
    };
    crysirror_vfx::shake::ShakeKind::Pattern(values.chunks_exact(3).map(|c| (c[0], c[1], c[2])).collect())
}


/// Scratch canvases of `grpShift`/`grpSlide` (DC 0 under the window, and
/// the window being composed).
const PAN_UNDER: u32 = 900_001;
const PAN_FRAME: u32 = 900_002;

/// `grpPan`, `grpShift`, `grpSlide` (original `longop::PanOp`): scroll an
/// area of a DC through a window of DC 0, as retained Canvas copies.
#[derive(Debug, Clone)]
pub(super) struct Pan {
    kind: u16,
    source: i32,
    a: (i32, i32),
    b: (i32, i32),
    window: [i32; 4],
    direction: i32,
    time: i32,
    start: u64,
}

impl RealLiveVm {
    /// Module 1:33 operations beyond the first bridge (adopted `grp.rs`).
    pub(super) fn graphics_extended(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command, op: u16, rec: bool) -> Result<bool> {
        let n = c.params.len();
        let now = m.sys.now();
        let rgb = |m: &mut Machine, at: usize| -> Result<[u8; 3]> {
            Ok([
                m.int_param(c, at)?.clamp(0, 255) as u8,
                m.int_param(c, at + 1)?.clamp(0, 255) as u8,
                m.int_param(c, at + 2)?.clamp(0, 255) as u8,
            ])
        };
        let rect = |m: &mut Machine, at: usize| -> Result<[i32; 4]> {
            let v = [m.int_param(c, at)?, m.int_param(c, at + 1)?, m.int_param(c, at + 2)?, m.int_param(c, at + 3)?];
            Ok(if rec { v } else { [v[0], v[1], v[2] - v[0] + 1, v[3] - v[1] + 1] })
        };
        match op {
            // grpFlash(r, g, b[, time]) / (area, r, g, b[, time]): the
            // colour covers the scene for `time` (original `FlashOp`).
            404 => {
                let (area, at) = if n >= 7 { (Some(rect(m, 0)?), 4) } else { (None, 0) };
                let colour = rgb(m, at)?;
                let time = if n > at + 3 { m.int_param(c, at + 3)? } else { 50 }.max(0) as u64;
                self.ext.flash = Some(Flash { area, colour, start: now, time: time.max(1), count: 1, fade: false, blink: false });
                self.update_flash(h, now)?;
                self.wait = Some(Wait::Flash);
            }
            // BOXPIKACHU_OLD(r, g, b, count): the screen blinks; waits.
            405 => {
                let colour = rgb(m, 0)?;
                let count = m.int_param(c, 3)?.max(1) as u32;
                self.ext.flash = Some(Flash { area: None, colour, start: now, time: 100, count, fade: false, blink: true });
                self.update_flash(h, now)?;
                self.wait = Some(Wait::Flash);
            }
            // grpPan / grpShift / grpSlide
            406..=408 => {
                let a = (m.int_param(c, 0)?, m.int_param(c, 1)?);
                let b = (m.int_param(c, 2)?, m.int_param(c, 3)?);
                let source = m.int_param(c, 4)?;
                let window = rect(m, 5)?;
                let (direction, time) = if op == 406 {
                    (0, m.int_param(c, 9)?)
                } else {
                    (m.int_param(c, 9)?, m.int_param(c, 10)?)
                };
                // grpShift/grpSlide give the block as corners; rec as a size.
                let b = if op != 406 && !rec { (b.0 - a.0 + 1, b.1 - a.1 + 1) } else { b };
                if op != 406 {
                    let [x, y, w, hgt] = window;
                    self.canvas(h, vec![
                        json!({"op":"create","id":PAN_UNDER,"width":w.max(1),"height":hgt.max(1)}),
                        json!({"op":"copy","source":0,"destination":PAN_UNDER,"rect":[x,y,w,hgt],"position":[0,0],"opacity":255,"replace":true}),
                    ])?;
                }
                let pan = Pan { kind: op, source, a, b, window, direction, time, start: now };
                if self.step_pan(h, &pan, now, m.sys.should_fast_forward())? {
                    self.finish_pan(h, &pan)?;
                } else {
                    self.wait = Some(Wait::Pan(pan));
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// One frame of a pan; true at the end.
    pub(super) fn step_pan(&mut self, h: &mut dyn HostServices, pan: &Pan, now: u64, fast: bool) -> Result<bool> {
        let t = if pan.time <= 0 || fast {
            1.0
        } else {
            (now.saturating_sub(pan.start) as f64 / f64::from(pan.time)).clamp(0.0, 1.0)
        };
        let [wx, wy, ww, wh] = pan.window;
        if pan.kind == 406 {
            let x = (f64::from(pan.a.0) + f64::from(pan.b.0 - pan.a.0) * t).round() as i32;
            let y = (f64::from(pan.a.1) + f64::from(pan.b.1 - pan.a.1) * t).round() as i32;
            self.canvas(h, vec![json!({"op":"copy","source":pan.source,"destination":0,"rect":[x,y,ww,wh],"position":[wx,wy],"opacity":255,"replace":true})])?;
            return Ok(t >= 1.0);
        }
        // Direction: 0 down, 1 up, 2 right, 3 left.
        let (horizontal, positive) = match pan.direction {
            0 => (false, true),
            1 => (false, false),
            2 => (true, true),
            _ => (true, false),
        };
        let span = if horizontal { pan.b.0 } else { pan.b.1 };
        let shown = (f64::from(span) * t).round() as i32;
        let block = if horizontal { [pan.a.0, pan.a.1, pan.b.0, wh] } else { [pan.a.0, pan.a.1, ww, pan.b.1] };
        let (bx, by) = match (horizontal, positive) {
            (false, true) => (wx, wy - span + shown),
            (false, false) => (wx, wy + wh - shown),
            (true, true) => (wx - span + shown, wy),
            (true, false) => (wx + ww - shown, wy),
        };
        // Shift: the old contents move out ahead of the new block; slide:
        // the new block moves in over them.
        let (ox, oy) = if pan.kind == 407 {
            match (horizontal, positive) {
                (false, true) => (wx, wy + shown),
                (false, false) => (wx, wy - shown),
                (true, true) => (wx + shown, wy),
                (true, false) => (wx - shown, wy),
            }
        } else {
            (wx, wy)
        };
        self.canvas(h, vec![
            json!({"op":"create","id":PAN_FRAME,"width":ww.max(1),"height":wh.max(1)}),
            json!({"op":"copy","source":PAN_UNDER,"destination":PAN_FRAME,"rect":[0,0,ww,wh],"position":[ox-wx,oy-wy],"opacity":255,"replace":true}),
            json!({"op":"copy","source":pan.source,"destination":PAN_FRAME,"rect":block,"position":[bx-wx,by-wy],"opacity":255,"replace":true}),
            json!({"op":"copy","source":PAN_FRAME,"destination":0,"rect":[0,0,ww,wh],"position":[wx,wy],"opacity":255,"replace":true}),
        ])?;
        Ok(t >= 1.0)
    }

    pub(super) fn finish_pan(&mut self, h: &mut dyn HostServices, pan: &Pan) -> Result<()> {
        if pan.kind != 406 {
            // Scratch canvases end at one pixel (engine-owned, reused).
            self.canvas(h, vec![
                json!({"op":"create","id":PAN_UNDER,"width":1,"height":1}),
                json!({"op":"create","id":PAN_FRAME,"width":1,"height":1}),
            ])?;
        }
        Ok(())
    }
}


impl RealLiveVm {
    /// Interface sounds the VM queued (`System::play_se`): `#SE.nnn` files
    /// on the rotating SE channels, prepared without holding the script.
    pub(super) fn play_queued_se(&mut self, m: &mut Machine, h: &mut dyn HostServices) -> Result<()> {
        for number in std::mem::take(&mut m.sys.se_requests) {
            let file = m.gameexe.get(&format!("SE.{number:03}")).and_then(|e| e.str(0).map(str::to_owned)).unwrap_or_default();
            if file.is_empty() {
                continue;
            }
            let asset = self.resource(m, "wav", &file)?;
            let slot = 32 + (self.se_serial % 16);
            self.se_serial = self.se_serial.wrapping_add(1);
            let gain = settings_volume_ext(&m.sys.settings, crate::vm::settings::channel::SE)
                * m.sys.sound.se_volume.value(m.sys.now()) as f32
                / 255.;
            let ticket = h
                .begin_audio(
                    self.client,
                    AudioRequest::PlayAsset {
                        channel: slot,
                        source: AudioAsset { path: VfsPath::new(Mount::Asset, &asset).map_err(err)?, frame_limit: None },
                        intro: None,
                        parameters: AudioParameters { gain, bus: PcmBus::Sfx, ..Default::default() },
                    },
                )
                .map_err(err)?;
            self.ext.event_tickets.push((ticket, slot));
        }
        Ok(())
    }
}


/// The client's capability save mount (`save/`), the only place saves are
/// read or written.
struct SaveMount<'a> {
    client: ClientId,
    h: &'a mut dyn HostServices,
}

impl crate::vm::save::Store for SaveMount<'_> {
    fn read(&mut self, name: &str) -> Option<Vec<u8>> {
        let path = VfsPath::new(Mount::Save, name).ok()?;
        self.h.read(self.client, &path).ok()
    }
    fn write(&mut self, name: &str, bytes: &[u8]) -> Result<()> {
        let path = VfsPath::new(Mount::Save, name).map_err(err)?;
        self.h.write_atomic(self.client, &path, bytes).map_err(err)
    }
}

/// `#SAVE_CNT` (original `ui::slot_count`).
fn slot_count(m: &Machine) -> i32 {
    m.gameexe.int("SAVE_CNT").or_else(|| m.gameexe.int("SAVEFILE_CNT")).unwrap_or(100).clamp(1, 1000)
}

impl RealLiveVm {
    /// Original `load_global` at boot, from the save mount.
    pub(super) fn load_global(&mut self, m: &mut Machine, h: &mut dyn HostServices) -> Result<()> {
        crate::vm::save::load_global(m, &mut SaveMount { client: self.client, h })?;
        self.ext.global_written = crate::vm::save::global_fingerprint(m);
        Ok(())
    }

    /// Writes the global file when global memory, read flags or settings
    /// changed (the original writes it with every save and at exit).
    pub(super) fn persist_global(&mut self, m: &Machine, h: &mut dyn HostServices) -> Result<()> {
        self.ext.global_check = self.ext.global_check.wrapping_add(1);
        if self.ext.global_check % 120 != 0 {
            return Ok(());
        }
        let now = crate::vm::save::global_fingerprint(m);
        if now != self.ext.global_written {
            match crate::vm::save::save_global(m, &mut SaveMount { client: self.client, h }) {
                Ok(()) => self.ext.global_written = now,
                // Hosts without a writable save mount keep running; the
                // state stays in memory like the original's non-persistent mode.
                Err(e) if e.to_string().contains("nsupported") || e.to_string().contains("ead-only") => {
                    self.ext.global_written = now;
                    *self.ext.notes.entry("global save not persisted".into()).or_default() += 1;
                }
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Save-slot queries (original `save::sys_query` and the RealLiveMax
    /// `GET_SAVE_*` functions) over the save mount.
    pub(super) fn save_query(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<bool> {
        let opcode = c.op.opcode;
        let mut store = SaveMount { client: self.client, h };
        match opcode {
            1409..=1414 | 1421 => crate::vm::save::sys_query(m, &mut store, c)?,
            // GET_SAVE_YEAR..GET_SAVE_MILLISECOND(slot), GET_SAVE_TITLE
            1400..=1408 => {
                let slot = m.int_param(c, 0)?;
                match (crate::vm::save::any_slot_header(m, &mut store, slot), opcode) {
                    (Some(header), 1408) => {
                        crate::vm::modules::sys_max::write_str(m, c, 1, header.title)?;
                        m.store = 1;
                    }
                    (Some(header), _) => m.store = header.date[usize::from(opcode - 1400)],
                    (None, _) => m.store = 0,
                }
            }
            1420 => m.store = slot_count(m),
            1423 => {
                let text = m.gameexe.str("SAVE_NODATA").unwrap_or("").to_owned();
                crate::vm::modules::sys_max::write_str(m, c, 0, text)?;
            }
            1424 => m.store = slot_count(m),
            1425 => m.store = m.latest_save,
            // GET_SAVE_MESSAGE(slot, str)
            1459 => {
                let slot = m.int_param(c, 0)?;
                let message = crate::vm::save::slot_extra(m, &mut store, slot).map(|e| e.message);
                m.store = i32::from(message.is_some());
                crate::vm::modules::sys_max::write_str(m, c, 1, message.unwrap_or_default())?;
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}


impl RealLiveVm {
    /// `#CGTABLE_FILENAME` from the data folder, decoded by the shared
    /// catalogue decoder (`crysirror_resource::cg_catalog`).
    pub(super) fn load_cg_table(&mut self, name: &str) -> Result<crysirror_resource::cg_catalog::CgCatalog> {
        let resources = self.resources.as_ref().context("RealLive resources")?.clone();
        let mut r = resources.borrow_mut();
        let path = match r.file("dat", name)? {
            Some(p) => p,
            None => r.file("", name)?.with_context(|| format!("reallive: CG table {name} not found"))?,
        };
        let bytes = r.read(&path)?;
        crysirror_resource::cg_catalog::decode(&bytes, &|| Ok(())).with_context(|| format!("reallive: CG table {name}"))
    }
}

/// `#CANCELCALL`: the scenario and entrypoint of a custom menu, if enabled
/// (adopted `menu::cancel_call`).
fn cancel_call(machine: &Machine) -> Option<(i32, i32)> {
    if machine.gameexe.int("CANCELCALL_MOD").unwrap_or(0) == 0 {
        return None;
    }
    let values = machine.gameexe.ints("CANCELCALL");
    Some((*values.first()?, values.get(1).copied().unwrap_or(0)))
}

/// `#WINDOW.nnn.OPEN_ANM_MOD/_TIME`, `CLOSE_ANM_MOD/_TIME` (original
/// `text::WindowConfig::from_gameexe`), animation enabled.
fn window_anm_from_gameexe(exe: &Gameexe, w: i32) -> [i32; 5] {
    let key = |k: &str| exe.int(&format!("WINDOW.{:03}.{k}", w.clamp(0, 999))).unwrap_or(0);
    [key("OPEN_ANM_MOD"), key("OPEN_ANM_TIME"), key("CLOSE_ANM_MOD"), key("CLOSE_ANM_TIME"), 1]
}

fn settings_volume_ext(s: &crate::vm::settings::Settings, index: usize) -> f32 {
    if !s.enabled[index] {
        return 0.;
    }
    s.volume[index].clamp(0, 255) as f32 / 255.
}

/// Original key codes (Windows virtual keys) for the keys the shared input
/// frame reports, for `GET_KEYTABLE_*`.
const KEY_CODES: &[(i32, u64)] = &[
    (0x0d, buttons::ENTER),
    (0x20, buttons::SPACE),
    (0x1b, buttons::ESCAPE),
    (0x11, buttons::CTRL),
    (0x31, buttons::KEY_1),
    (0x32, buttons::KEY_2),
    (0x33, buttons::KEY_3),
    (0x34, buttons::KEY_4),
    (0x35, buttons::KEY_5),
    (0x36, buttons::KEY_6),
    (0x37, buttons::KEY_7),
    (0x38, buttons::KEY_8),
    (0x39, buttons::KEY_9),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flash_alpha_follows_original_blink_and_fade() {
        let f = Flash { area: None, colour: [255, 255, 255], start: 100, time: 100, count: 3, fade: false, blink: true };
        assert_eq!(f.alpha(100), 255);
        assert_eq!(f.alpha(160), 0);
        assert_eq!(f.alpha(210), 255);
        assert!(f.finished(400));
        let g = Flash { area: None, colour: [0, 0, 0], start: 0, time: 100, count: 1, fade: true, blink: false };
        assert_eq!(g.alpha(0), 255);
        assert_eq!(g.alpha(50), 127);
        assert_eq!(g.alpha(100), 0);
    }
}

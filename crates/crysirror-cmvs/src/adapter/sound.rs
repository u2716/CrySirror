// SPDX-License-Identifier: MPL-2.0
//! CMVS sound-channel policy over the shared audio owner.

use crate::vm::{CmvsVm, EffectSound};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    service::{
        AudioParameters, AudioPcm16Gain, AudioPcm16GainUpdate, AudioPlaybackInfo,
        AudioPreparePhase, AudioRequest, AudioSourceMeterSpec, EncodedAudio, HostServices,
    },
    state::ClientId,
};
use serde::Deserialize;

#[derive(Clone, Copy, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct EffectsSettings {
    pub enabled: bool,
    master_volume: u16,
    effects_volume: u16,
}

impl Default for EffectsSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            master_volume: 256,
            effects_volume: 256,
        }
    }
}

impl EffectsSettings {
    pub fn validate(self) -> Result<()> {
        ensure!(
            self.master_volume <= 256 && self.effects_volume <= 256,
            "CMVS effects volume outside 0..256"
        );
        Ok(())
    }

    pub(super) fn level(self, volume: i32) -> u32 {
        let combined = (u32::from(self.master_volume) * u32::from(self.effects_volume)) >> 8;
        if volume > 0 {
            (combined.wrapping_mul(volume as u32)) >> 8
        } else {
            combined
        }
    }

    fn gain(self, volume: i32) -> f32 {
        scalar(self.level(volume).min(256))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Channel {
    Empty,
    Prepared,
    Started,
}

pub(super) struct Effects {
    pub settings: EffectsSettings,
    channels: [Channel; 6],
    pending: Option<(u64, usize, bool, EffectSource)>,
    restart: Option<EffectRestart>,
    sources: [Option<EffectSource>; 6],
    volume_words: [i32; 6],
    mix_words: [EffectMix; 6],
}

struct EffectRestart {
    ticket: u64,
    slot: usize,
    looping: bool,
}

#[derive(Clone, Default)]
struct EffectSource {
    name: String,
    volume: i32,
    looping: bool,
    started: bool,
}

#[derive(Clone, Copy)]
struct EffectMix {
    multiplier: u32,
    pan: i32,
}

impl Default for EffectMix {
    fn default() -> Self {
        Self {
            multiplier: 256,
            pan: 0,
        }
    }
}

impl EffectMix {
    fn reopened(self) -> AudioPcm16Gain {
        AudioPcm16Gain {
            multiplier: if (self.multiplier as i32) < 256 {
                self.multiplier
            } else {
                256
            },
            stereo: pcm16_pan(self.pan),
        }
    }
}

fn pcm16_pan(pan: i32) -> [u32; 2] {
    let level = 256_u32.wrapping_sub(pan.unsigned_abs());
    if pan > 0 {
        [level, 256]
    } else {
        [256, level]
    }
}

pub(super) fn fader_update(opcode: u16, word: u32) -> AudioPcm16GainUpdate {
    if matches!(opcode, 0x20ad | 0x20b9) {
        AudioPcm16GainUpdate::Stereo(pcm16_pan(word as i32))
    } else {
        AudioPcm16GainUpdate::Multiplier(word)
    }
}

pub(super) const MUSIC_CHANNELS: [u32; 2] = [0, 1];
/// Client mix group of every CMVS channel, muted while the inactive window must be silent.
pub(super) const MIX_GROUP: &str = "cmvs";
pub(super) const EFFECT_CHANNELS: [u32; 6] = [9, 10, 11, 12, 13, 14];
pub(super) const DEVICE_SOUND_CHANNEL: u32 = 15;

pub(super) fn meter_spec(quality: i32, buffer: u32) -> AudioSourceMeterSpec {
    let divisor = if quality == 0 { 2 } else { 1 };
    AudioSourceMeterSpec {
        reference_rate: 44_100 / divisor,
        window_frames: [1536, 2048, 3072, 4096][buffer.min(3) as usize] / divisor,
        history_windows: 3,
    }
}

#[derive(Default)]
pub(super) struct DeviceSound {
    pending: Option<u64>,
}

pub(super) struct DeviceSoundRequest {
    pub name: Option<String>,
    base_level: u32,
    fade_ms: u32,
    looping: bool,
    start: bool,
}

impl DeviceSoundRequest {
    pub fn read(vm: &CmvsVm, music: MusicSettings) -> Result<Self> {
        let [_, name, level, fade_ms, looping, start] = vm.owner_arguments(0x2240)?;
        let base_level = u32::from(music.master_volume).wrapping_mul(level) >> 8;
        Ok(Self {
            name: vm.sound_name(name)?,
            base_level: if base_level <= 256 { base_level } else { 256 },
            fade_ms,
            looping: looping != 0,
            start: start != 0,
        })
    }
}

impl DeviceSound {
    pub fn load(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
        request: DeviceSoundRequest,
        bytes: Option<Vec<u8>>,
        meter: AudioSourceMeterSpec,
    ) -> Result<()> {
        ensure!(
            self.pending.is_none(),
            "CMVS device sound already preparing"
        );
        let Some(bytes) = bytes else {
            vm.complete_owner_call(6, Some(0))?;
            return Ok(());
        };
        let ticket = host.begin_metered_audio(
            client,
            AudioRequest::Play {
                channel: DEVICE_SOUND_CHANNEL,
                source: EncodedAudio {
                    bytes: bytes.into(),
                    frame_limit: None,
                },
                intro: None,
                parameters: AudioParameters {
                    gain: scalar(request.base_level),
                    pcm16_gain: Some(AudioPcm16Gain::default()),
                    looping: request.looping,
                    paused: !request.start,
                    fade_ms: if request.start {
                        request.fade_ms as f32
                    } else {
                        0.
                    },
                    mix_group: Some(MIX_GROUP.into()),
                    ..AudioParameters::default()
                },
            },
            meter,
        )?;
        self.pending = Some(ticket);
        Ok(())
    }

    pub fn poll(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
    ) -> Result<bool> {
        let Some(ticket) = self.pending else {
            return Ok(false);
        };
        let status = host.audio_preparation(client, ticket)?;
        ensure!(
            status.channel == DEVICE_SOUND_CHANNEL,
            "CMVS device sound receipt channel differs"
        );
        if status.phase == AudioPreparePhase::Pending {
            return Ok(true);
        }
        host.release_audio(client, ticket)?;
        self.pending = None;
        let ready = status.phase == AudioPreparePhase::Ready;
        vm.complete_owner_call(6, Some(u32::from(ready)))?;
        Ok(true)
    }

    pub fn stop(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<()> {
        host.submit_audio(
            client,
            AudioRequest::Stop {
                channel: DEVICE_SOUND_CHANNEL,
                fade_ms: 0.,
            },
        )?;
        Ok(())
    }

    pub fn position(host: &dyn HostServices, client: ClientId) -> Result<u32> {
        Ok(host
            .audio_info(client, DEVICE_SOUND_CHANNEL)?
            .filter(|info| {
                info.source_playing && !info.paused && !info.stopping && info.output_rate != 0
            })
            .map_or(u32::MAX, |info| {
                (info.transport_frames.saturating_mul(1000) / u64::from(info.output_rate)) as u32
            }))
    }
}

pub(super) struct EffectReplay {
    pub slot: usize,
    pub name: String,
    pub parameters: AudioParameters,
}

impl Effects {
    pub fn new(settings: EffectsSettings) -> Self {
        Self {
            settings,
            channels: [Channel::Empty; 6],
            pending: None,
            restart: None,
            sources: Default::default(),
            volume_words: [0; 6],
            mix_words: [EffectMix::default(); 6],
        }
    }

    pub fn load(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
        bytes: Option<Vec<u8>>,
    ) -> Result<()> {
        let EffectSound::Load {
            channel,
            name,
            volume,
            looping,
            immediate,
            fade_ms,
            ..
        } = vm.pending_effect_sound()?
        else {
            anyhow::bail!("stale CMVS effect preparation");
        };
        let record = EffectSource {
            name,
            volume,
            looping,
            started: immediate,
        };
        ensure!(self.pending.is_none(), "CMVS effect decode already pending");
        self.stop(host, client, channel)?;
        let Some(bytes) = bytes else {
            vm.complete_effect_sound(Some(false))?;
            self.remember(channel, record);
            return Ok(());
        };
        let source = EncodedAudio {
            bytes: bytes.into(),
            frame_limit: None,
        };
        let parameters = AudioParameters {
            gain: self.settings.gain(volume),
            pcm16_gain: Some(AudioPcm16Gain::default()),
            looping,
            retain_on_finish: true,
            transport_active: immediate,
            fade_ms: if immediate { fade_ms as f32 } else { 0. },
            mix_group: Some(MIX_GROUP.into()),
            ..AudioParameters::default()
        };
        let ticket = host.begin_audio(
            client,
            AudioRequest::Play {
                channel: device_channel(channel),
                source,
                intro: None,
                parameters,
            },
        )?;
        self.pending = Some((ticket, channel, immediate, record));
        Ok(())
    }

    /// True while this owner is still handling a prior effect command.
    pub fn poll(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
    ) -> Result<bool> {
        if let Some(restart) = &self.restart {
            let channel = device_channel(restart.slot);
            let status = host.audio_seek_preparation(client, restart.ticket)?;
            ensure!(
                status.channel == channel,
                "CMVS effect seek receipt channel differs"
            );
            if status.phase == AudioPreparePhase::Pending {
                return Ok(true);
            }
            host.release_audio_seek(client, restart.ticket)?;
            let restart = self.restart.take().expect("validated pending restart");
            if status.phase != AudioPreparePhase::Ready {
                anyhow::bail!("CMVS effect rewind failed: {:?}", status.error);
            }
            self.channels[restart.slot] = Channel::Started;
            self.remember_start(restart.slot, restart.looping);
            vm.complete_owner_call(2, None)?;
            return Ok(false);
        }
        let Some((ticket, channel, immediate, _)) = &self.pending else {
            return Ok(false);
        };
        let (ticket, channel, immediate) = (*ticket, *channel, *immediate);
        let status = host.audio_preparation(client, ticket)?;
        ensure!(
            status.channel == device_channel(channel),
            "CMVS effect receipt channel differs"
        );
        if status.phase == AudioPreparePhase::Pending {
            return Ok(true);
        }
        host.release_audio(client, ticket)?;
        let (_, _, _, source) = self.pending.take().expect("validated pending sound");
        let ready = status.phase == AudioPreparePhase::Ready;
        if ready {
            self.channels[channel] = if immediate {
                Channel::Started
            } else {
                Channel::Prepared
            };
        }
        vm.complete_effect_sound(Some(ready))
            .context("commit CMVS effect sound")?;
        self.remember(channel, source);
        Ok(true)
    }

    fn remember(&mut self, slot: usize, record: EffectSource) {
        self.volume_words[slot] = record.volume;
        self.mix_words[slot] = EffectMix::default();
        self.sources[slot] = Some(record);
    }

    pub fn set_fader(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
        opcode: u16,
        word: u32,
    ) -> Result<()> {
        host.update_audio_pcm16_gain(client, device_channel(slot), fader_update(opcode, word))?;
        if opcode == 0x20b8 {
            self.mix_words[slot].multiplier = word;
        } else {
            self.mix_words[slot].pan = word as i32;
        }
        Ok(())
    }

    pub fn reset_records(&mut self) {
        self.volume_words.fill(0);
        self.mix_words.fill(EffectMix {
            multiplier: 0,
            pan: 0,
        });
    }

    pub fn remember_disabled(&mut self, vm: &mut CmvsVm) -> Result<()> {
        let EffectSound::Load {
            channel,
            name,
            volume,
            looping,
            immediate,
            ..
        } = vm.pending_effect_sound()?
        else {
            anyhow::bail!("CMVS effect registration missing");
        };
        vm.complete_effect_sound(None)?;
        self.remember(
            channel,
            EffectSource {
                name,
                volume,
                looping,
                started: immediate,
            },
        );
        Ok(())
    }

    pub fn reopen_plan(&self) -> Vec<EffectReplay> {
        self.sources
            .iter()
            .enumerate()
            .filter_map(|(slot, record)| {
                let record = record.as_ref().filter(|record| !record.name.is_empty())?;
                Some(EffectReplay {
                    slot,
                    name: record.name.clone(),
                    parameters: AudioParameters {
                        gain: self.settings.gain(record.volume),
                        pcm16_gain: Some(self.mix_words[slot].reopened()),
                        looping: record.looping,
                        retain_on_finish: true,
                        transport_active: record.looping && record.started,
                        mix_group: Some(MIX_GROUP.into()),
                        ..AudioParameters::default()
                    },
                })
            })
            .collect()
    }

    pub fn set_availability(&mut self, enabled: bool, decoded: [bool; 6]) {
        self.settings.enabled = enabled;
        for (slot, ready) in decoded.into_iter().enumerate() {
            let record = self.sources[slot].as_mut();
            if enabled {
                if let Some(record) = record {
                    record.started &= record.looping;
                }
            }
            self.channels[slot] = if !ready {
                Channel::Empty
            } else if self.sources[slot]
                .as_ref()
                .is_some_and(|record| record.started)
            {
                Channel::Started
            } else {
                Channel::Prepared
            };
        }
    }

    pub fn start(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        channel: usize,
        looping: bool,
    ) -> Result<()> {
        if self.settings.enabled
            && host.resume_retained_audio_channel(client, device_channel(channel), looping)?
        {
            self.channels[channel] = Channel::Started;
        }
        self.remember_start(channel, looping);
        Ok(())
    }

    fn remember_start(&mut self, channel: usize, looping: bool) {
        let source = self.sources[channel].get_or_insert_with(EffectSource::default);
        source.looping = looping;
        source.started = true;
    }

    pub fn pause(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
    ) -> Result<()> {
        if self.channels[slot] == Channel::Started {
            let channel = device_channel(slot);
            match host.audio_info(client, channel)? {
                Some(info) => {
                    if info.transport_active {
                        set_effect_active(host, client, channel, false)?;
                    }
                    self.channels[slot] = Channel::Prepared;
                }
                None => self.channels[slot] = Channel::Empty,
            }
        }
        if let Some(source) = &mut self.sources[slot] {
            source.started = false;
        }
        Ok(())
    }

    pub fn restart(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
        looping: bool,
    ) -> Result<bool> {
        ensure!(self.restart.is_none(), "CMVS effect rewind already pending");
        let channel = device_channel(slot);
        if self.settings.enabled && host.audio_info(client, channel)?.is_some() {
            let ticket = host.begin_retained_audio_rewind(client, channel, looping)?;
            self.restart = Some(EffectRestart {
                ticket,
                slot,
                looping,
            });
            return Ok(true);
        }
        self.remember_start(slot, looping);
        Ok(false)
    }

    pub fn stop(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        channel: usize,
    ) -> Result<()> {
        if self.channels[channel] != Channel::Empty {
            host.submit_audio(
                client,
                AudioRequest::Stop {
                    channel: device_channel(channel),
                    fade_ms: 0.,
                },
            )?;
            self.channels[channel] = Channel::Empty;
        }
        self.sources[channel] = None;
        Ok(())
    }

    pub fn fade(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        channel: usize,
        duration: u32,
    ) -> Result<()> {
        let device = device_channel(channel);
        let playing = host.audio_info(client, device)?;
        if playing.is_some_and(|info| !info.stopping) {
            host.submit_audio(
                client,
                AudioRequest::Stop {
                    channel: device,
                    fade_ms: duration as f32,
                },
            )?;
        }
        self.sources[channel] = None;
        Ok(())
    }

    pub fn position(
        &self,
        host: &dyn HostServices,
        client: ClientId,
        channel: usize,
    ) -> Result<u32> {
        position(host, client, device_channel(channel))
    }
}

fn set_effect_active(
    host: &mut dyn HostServices,
    client: ClientId,
    channel: u32,
    active: bool,
) -> Result<()> {
    host.submit_audio(client, AudioRequest::SetTransportActive { channel, active })?;
    Ok(())
}

pub(super) fn device_channel(channel: usize) -> u32 {
    channel as u32 + 9
}

#[derive(Clone, Copy, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct VoiceSettings {
    pub enabled: bool,
    master_volume: u16,
    voice_volume: u16,
}

impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            master_volume: 256,
            voice_volume: 256,
        }
    }
}

impl VoiceSettings {
    pub fn validate(self) -> Result<()> {
        ensure!(
            self.master_volume <= 256 && self.voice_volume <= 256,
            "CMVS voice volume outside 0..256"
        );
        Ok(())
    }

    fn gain(self, volume: i32) -> f32 {
        let master = (u32::from(self.master_volume) * u32::from(self.voice_volume)) >> 8;
        let gain = if volume > 0 {
            master.wrapping_mul(volume as u32) >> 8
        } else {
            master
        };
        gain.min(256) as f32 / 256.
    }
}

#[derive(Clone)]
pub(super) struct VoiceRequest {
    pub name: Option<String>,
    channel: u32,
    volume: i32,
    start: bool,
    rate: f32,
    words: u16,
}

impl VoiceRequest {
    pub fn read(vm: &CmvsVm, opcode: u16) -> Result<Self> {
        let [channel, name, volume, start] = vm.owner_arguments(opcode)?;
        ensure!(channel < 4, "CMVS voice channel outside 0..3");
        let (rate, words) = if opcode == 0x20af {
            let [_, _, _, _, raw] = vm.owner_arguments(opcode)?;
            let rate = f32::from_bits(raw);
            ensure!(rate.is_finite(), "CMVS voice rate must be finite");
            (
                if (1.0..=3.0).contains(&rate) {
                    rate
                } else {
                    1.0
                },
                5,
            )
        } else {
            (1.0, 4)
        };
        Ok(Self {
            name: vm.sound_name(name)?,
            channel: channel + 5,
            volume: volume as i32,
            start: start != 0,
            rate,
            words,
        })
    }

    pub fn disabled(&self, vm: &mut CmvsVm) -> Result<()> {
        vm.complete_owner_call(self.words, Some(0))?;
        Ok(())
    }
}

pub(super) struct Voices {
    pub settings: VoiceSettings,
    pending: Option<(u64, VoiceRequest)>,
    sources: [Option<VoiceRequest>; 4],
    /// Voice record +244: a fade-out has started (0x14009f320).
    fading_out: [bool; 4],
    /// Decoded channel count of each prepared voice, for its PCM byte counter.
    channels: [u32; 4],
}

impl Voices {
    pub fn new(settings: VoiceSettings) -> Self {
        Self {
            settings,
            pending: None,
            sources: Default::default(),
            fading_out: [false; 4],
            channels: [1; 4],
        }
    }

    /// `0x20ab` (0x14009f480): the decoder's cumulative 16-bit PCM bytes at the source
    /// rate, or -1 unless the voice is playing. Loops do not reset it.
    pub fn decoded_bytes(&self, host: &dyn HostServices, client: ClientId, slot: u32) -> Result<u32> {
        let Some(info) = host.audio_info(client, slot + 5)?.filter(|info| {
            info.source_playing && info.transport_active && !info.paused && info.output_rate != 0
        }) else {
            return Ok(u32::MAX);
        };
        let frames = info.transport_frames * u64::from(info.source_rate) / u64::from(info.output_rate);
        Ok((frames * u64::from(self.channels[slot as usize]) * 2) as u32)
    }

    /// Voices that hold the music down: playing, and not fading out unless
    /// `0x223e` also counts fading voices (0x1400a07f0).
    pub fn ducking(&self, host: &dyn HostServices, client: ClientId, fading: bool) -> Result<bool> {
        for slot in 0..4u32 {
            if (fading || !self.fading_out[slot as usize]) && host.audio_info(client, slot + 5)?.is_some() {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn start(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: u32,
        looping: bool,
    ) -> Result<()> {
        if host.resume_retained_audio_channel(client, 5 + slot, looping)? {
            if let Some(source) = &mut self.sources[slot as usize] {
                source.start = true;
            }
        }
        Ok(())
    }

    pub fn play(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
        request: VoiceRequest,
        source: Option<Vec<u8>>,
        meter: AudioSourceMeterSpec,
    ) -> Result<()> {
        self.stop(host, client, request.channel, 0)?;
        let Some(bytes) = source else {
            vm.complete_voice(request.words, false)?;
            return Ok(());
        };
        self.channels[(request.channel - 5) as usize] = source_channels(&bytes);
        let ticket = host.begin_metered_audio(
            client,
            AudioRequest::Play {
                channel: request.channel,
                source: EncodedAudio {
                    bytes: bytes.into(),
                    frame_limit: None,
                },
                intro: None,
                parameters: AudioParameters {
                    gain: self.settings.gain(request.volume),
                    pcm16_gain: Some(AudioPcm16Gain::default()),
                    rate: request.rate,
                    paused: !request.start,
                    retain_on_finish: true,
                    bus: crysirror_core::service::PcmBus::Voice,
                    mix_group: Some(MIX_GROUP.into()),
                    ..Default::default()
                },
            },
            meter,
        )?;
        self.pending = Some((ticket, request));
        Ok(())
    }

    pub fn poll(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
    ) -> Result<bool> {
        let Some((ticket, request)) = &self.pending else {
            return Ok(false);
        };
        let status = host.audio_preparation(client, *ticket)?;
        ensure!(
            status.channel == request.channel,
            "CMVS voice receipt channel differs"
        );
        if status.phase == AudioPreparePhase::Pending {
            return Ok(true);
        }
        host.release_audio(client, *ticket)?;
        let (_, request) = self.pending.take().expect("validated pending voice");
        let loaded = status.phase == AudioPreparePhase::Ready;
        vm.complete_voice(request.words, loaded)?;
        if loaded {
            let index = (request.channel - 5) as usize;
            self.sources[index] = Some(request);
        }
        Ok(true)
    }

    pub fn stop(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        channel: u32,
        fade: u32,
    ) -> Result<()> {
        host.submit_audio(
            client,
            AudioRequest::Stop {
                channel,
                fade_ms: fade as f32,
            },
        )?;
        if fade == 0 {
            self.sources[(channel - 5) as usize] = None;
        }
        self.fading_out[(channel - 5) as usize] = fade != 0;
        Ok(())
    }

    pub fn released(
        &mut self,
        host: &dyn HostServices,
        client: ClientId,
        slot: u32,
    ) -> Result<bool> {
        let released = host.audio_info(client, slot + 5)?.is_none();
        if released {
            self.sources[slot as usize] = None;
        }
        Ok(released)
    }
}

pub(super) fn position(host: &dyn HostServices, client: ClientId, channel: u32) -> Result<u32> {
    Ok(host
        .audio_info(client, channel)?
        .filter(|info| {
            info.source_playing && info.transport_active && !info.paused && info.output_rate != 0
        })
        .map_or(u32::MAX, |info| {
            (info.transport_frames.saturating_mul(1000) / u64::from(info.output_rate)) as u32
        }))
}

#[derive(Clone, Copy, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct MusicSettings {
    pub enabled: bool,
    master_volume: u16,
    music_volume: u16,
}

impl Default for MusicSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            master_volume: 256,
            music_volume: 256,
        }
    }
}

impl MusicSettings {
    pub fn validate(self) -> Result<()> {
        ensure!(
            self.master_volume <= 256 && self.music_volume <= 256,
            "CMVS music volume outside 0..256"
        );
        Ok(())
    }

    pub fn gain(self) -> f32 {
        ((u32::from(self.master_volume) * u32::from(self.music_volume)) >> 8) as f32 / 256.
    }
}

#[derive(Clone, Copy)]
pub(super) struct MovieVolume {
    pub raw: u16,
    pub gain: f32,
}

impl MovieVolume {
    pub fn new(music: MusicSettings, raw: Option<u16>, legacy_gain: Option<f32>) -> Result<Self> {
        let gain = legacy_gain.unwrap_or(1.0);
        ensure!(
            gain.is_finite() && (0.0..=1.0).contains(&gain),
            "CMVS movie gain"
        );
        if let Some(raw) = raw {
            ensure!(raw <= 256, "CMVS movie volume outside 0..256");
            return Ok(Self {
                raw,
                gain: scalar(combine(music.master_volume, raw)),
            });
        }
        let raw = if music.master_volume == 0 || legacy_gain.is_none() {
            256
        } else {
            (gain * 65536.0 / f32::from(music.master_volume))
                .round()
                .min(256.0) as u16
        };
        Ok(Self { raw, gain })
    }
}

fn combine(master: u16, volume: u16) -> u32 {
    (u32::from(master) * u32::from(volume)) >> 8
}

fn scalar(volume: u32) -> f32 {
    volume as f32 / 256.0
}

#[cfg(test)]
mod level_tests {
    use super::*;

    #[test]
    fn source_meter_uses_native_sound_quality_and_buffer_modes() {
        for (buffer, frames) in [1536, 2048, 3072, 4096].into_iter().enumerate() {
            let standard = meter_spec(1, buffer as u32);
            let reduced = meter_spec(0, buffer as u32);
            assert_eq!(
                (standard.reference_rate, standard.window_frames),
                (44_100, frames)
            );
            assert_eq!(
                (reduced.reference_rate, reduced.window_frames),
                (22_050, frames / 2)
            );
            assert_eq!(standard.history_windows, 3);
            assert_eq!(reduced.history_windows, 3);
        }
        assert_eq!(meter_spec(-1, u32::MAX), meter_spec(1, 3));
    }

    #[test]
    fn pan_uses_native_unsigned_coefficients_after_signed_subtraction() {
        assert_eq!(pcm16_pan(64), [192, 256]);
        assert_eq!(pcm16_pan(-256), [256, 0]);
        assert_eq!(pcm16_pan(257), [u32::MAX, 256]);
        assert_eq!(pcm16_pan(i32::MIN), [256, 0x8000_0100]);
    }

    #[test]
    fn reopening_effects_only_autoplays_loops_marked_started() {
        let mut effects = Effects::new(EffectsSettings::default());
        for (slot, looping, started) in [(0, true, true), (1, false, true), (2, true, false)] {
            effects.remember(
                slot,
                EffectSource {
                    name: format!("sound{slot}"),
                    volume: 128,
                    looping,
                    started,
                },
            );
        }
        effects.mix_words[0] = EffectMix {
            multiplier: u32::MAX,
            pan: -64,
        };
        effects.mix_words[1] = EffectMix {
            multiplier: 512,
            pan: 128,
        };
        effects.set_availability(false, [false; 6]);
        let recipe = effects.reopen_plan();
        assert_eq!(recipe.len(), 3);
        assert_eq!(recipe[0].parameters.gain, 0.5);
        assert_eq!(
            recipe[0].parameters.pcm16_gain,
            Some(AudioPcm16Gain {
                multiplier: u32::MAX,
                stereo: [256, 192],
            })
        );
        assert_eq!(
            recipe[1].parameters.pcm16_gain,
            Some(AudioPcm16Gain {
                multiplier: 256,
                stereo: [128, 256],
            })
        );
        assert!(recipe[0].parameters.transport_active);
        assert!(!recipe[1].parameters.transport_active && !recipe[2].parameters.transport_active);
        assert!(recipe.iter().all(|sound| sound.parameters.retain_on_finish));
        effects.set_availability(true, [true, true, true, false, false, false]);
        assert!(effects.channels[0] == Channel::Started);
        assert!(effects.channels[1] == Channel::Prepared);
        assert!(!effects.sources[1].as_ref().unwrap().started);
    }

    #[test]
    fn music_reopening_retains_the_intro_loop_pair_and_uses_slot_one() {
        let mut music = Music::new(MusicSettings {
            enabled: false,
            ..MusicSettings::default()
        });
        music.remember_disabled(MusicRequest {
            restored: false,
            first: "intro".into(),
            second: Some("loop".into()),
            duration: 500,
            looping: false,
            words: 4,
        });
        let request = music.reopen_request().unwrap();
        assert!(request.looping && request.duration == 0);
        assert_eq!(request.second.as_deref(), Some("loop"));
        music.commit_reopen(request);
        music.clear_playback();
        assert_eq!(music.current, 0);
        assert!(music.reopen_request().is_some());
    }

    #[test]
    fn master_and_effect_level_updates_follow_distinct_native_paths() {
        let mut effects = Effects::new(EffectsSettings::default());
        effects.volume_words[..4].copy_from_slice(&[128, 1024, -1, 1]);
        let levels = Levels {
            music: MusicSettings {
                enabled: false,
                ..MusicSettings::default()
            },
            effects: effects.settings,
            voices: VoiceSettings::default(),
            movie: MovieVolume {
                raw: 192,
                gain: 0.75,
            },
        };

        let master = levels.updated(0x22d9, 128);
        assert_eq!(
            master.live_channels(0x22d9, &effects),
            (5..15).map(|channel| (channel, 0.5)).collect::<Vec<_>>()
        );
        assert_eq!(master.movie.gain, 96.0 / 256.0);
        let category = levels.updated(0x22db, 128);
        assert_eq!(
            category.live_channels(0x22db, &effects),
            [(9, 0.25), (11, 0.5), (12, 0.0), (13, 0.5), (14, 0.5)]
        );
        assert_eq!(levels.updated(0x22dd, u32::MAX).movie.raw, 0);
        assert_eq!(levels.updated(0x22dc, 999).read(0x22dc), 256);
    }
}

#[derive(Clone, Copy)]
pub(super) struct Levels {
    pub music: MusicSettings,
    pub effects: EffectsSettings,
    pub voices: VoiceSettings,
    pub movie: MovieVolume,
}

impl Levels {
    pub fn read(self, opcode: u16) -> u32 {
        u32::from(match opcode {
            0x22d9 => self.music.master_volume,
            0x22da => self.music.music_volume,
            0x22db => self.effects.effects_volume,
            0x22dc => self.voices.voice_volume,
            0x22dd => self.movie.raw,
            _ => unreachable!("not a volume operation"),
        })
    }

    pub fn updated(mut self, opcode: u16, raw: u32) -> Self {
        let value = (raw as i32).clamp(0, 256) as u16;
        match opcode {
            0x22d9 => {
                self.music.master_volume = value;
                self.effects.master_volume = value;
                self.voices.master_volume = value;
            }
            0x22da => self.music.music_volume = value,
            0x22db => self.effects.effects_volume = value,
            0x22dc => self.voices.voice_volume = value,
            0x22dd => self.movie.raw = value,
            _ => unreachable!("not a volume operation"),
        }
        if matches!(opcode, 0x22d9 | 0x22dd) {
            self.movie.gain = scalar(combine(self.music.master_volume, self.movie.raw));
        }
        self
    }

    pub fn live_channels(self, opcode: u16, effects: &Effects) -> Vec<(u32, f32)> {
        let mut updates = Vec::new();
        if self.music.enabled && matches!(opcode, 0x22d9 | 0x22da) {
            updates.extend((0..2).map(|channel| (channel, self.music.gain())));
        }
        if matches!(opcode, 0x22d9 | 0x22dc) {
            let gain = scalar(combine(self.voices.master_volume, self.voices.voice_volume));
            updates.extend((5..9).map(|channel| (channel, gain)));
        }
        if matches!(opcode, 0x22d9 | 0x22db) {
            let base = combine(self.effects.master_volume, self.effects.effects_volume);
            updates.extend(effects.volume_words.iter().enumerate().filter_map(
                |(slot, &volume)| {
                    let level = if opcode == 0x22db && volume > 0 {
                        base.wrapping_mul(volume as u32) >> 8
                    } else {
                        base
                    };
                    (level <= 256).then(|| (device_channel(slot), scalar(level)))
                },
            ));
        }
        updates
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct MusicRequest {
    /// Started by a save restore, with no script call waiting on it.
    #[serde(default)]
    pub restored: bool,
    pub first: String,
    pub second: Option<String>,
    duration: u32,
    looping: bool,
    words: u16,
}

impl MusicRequest {
    pub fn read(vm: &CmvsVm, opcode: u16) -> Result<Self> {
        let (first, second, duration, looping, words) = if opcode == 0x20a4 {
            let [first, second, duration, looping] = vm.owner_arguments(opcode)?;
            (first, vm.sound_name(second)?, duration, looping != 0, 4)
        } else {
            let [first, duration, looping] = vm.owner_arguments(opcode)?;
            (first, None, duration, looping != 0, 3)
        };
        Ok(Self {
            restored: false,
            first: vm.sound_name(first)?.context("CMVS music name is empty")?,
            second,
            duration,
            looping,
            words,
        })
    }

    pub fn complete(&self, vm: &mut CmvsVm) -> Result<()> {
        if self.restored {
            return Ok(());
        }
        vm.complete_owner_call(self.words, None)?;
        Ok(())
    }

    pub fn recipe(
        &self,
        channel: u32,
        primary: Vec<u8>,
        secondary: Option<Vec<u8>>,
        gain: f32,
    ) -> AudioRequest {
        let first = EncodedAudio {
            bytes: primary.into(),
            frame_limit: None,
        };
        let (source, intro) = match secondary {
            Some(bytes) => (
                EncodedAudio {
                    bytes: bytes.into(),
                    frame_limit: None,
                },
                Some(first),
            ),
            None => (first, None),
        };
        AudioRequest::Play {
            channel,
            source,
            intro,
            parameters: AudioParameters {
                gain,
                pcm16_gain: Some(AudioPcm16Gain::default()),
                looping: self.looping,
                fade_ms: self.duration as f32,
                bus: crysirror_core::service::PcmBus::Music,
                mix_group: Some(MIX_GROUP.into()),
                ..AudioParameters::default()
            },
        }
    }

    pub fn read_sources(
        &self,
        sources: &crate::vm::ScriptSources,
        cancel: &crysirror_tasks::Cancellation,
    ) -> Result<(Vec<u8>, Option<Vec<u8>>)> {
        let read = |name: &str| -> Result<Vec<u8>> {
            cancel.check()?;
            let input = sources
                .open_entry(name)?
                .with_context(|| format!("CMVS music missing: {name}"))?;
            crysirror_resource::read_all(input.as_ref(), 64 * 1024 * 1024)
        };
        let first = read(&self.first)?;
        let second = if self.looping {
            self.second.as_deref().map(read).transpose()?
        } else {
            None
        };
        Ok((first, second))
    }
}

pub(super) struct Music {
    pub settings: MusicSettings,
    /// Host time of the last request made while music output is off (BGM object +160).
    disabled_since: u64,
    current: u32,
    pending: Option<(u64, MusicRequest, u32)>,
    sources: [Option<MusicRequest>; 2],
    retained: Option<MusicRequest>,
}

impl Music {
    pub fn new(settings: MusicSettings) -> Self {
        Self {
            settings,
            disabled_since: 0,
            current: 0,
            pending: None,
            sources: Default::default(),
            retained: None,
        }
    }

    pub fn stop(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<()> {
        if self.settings.enabled {
            for channel in 0..=1 {
                host.submit_audio(
                    client,
                    AudioRequest::Stop {
                        channel,
                        fade_ms: 0.,
                    },
                )?;
            }
        }
        self.clear_playback();
        self.retained = None;
        Ok(())
    }

    pub fn fade(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        duration: u32,
    ) -> Result<()> {
        if self.settings.enabled {
            host.submit_audio(
                client,
                AudioRequest::Stop {
                    channel: self.current,
                    fade_ms: duration as f32,
                },
            )?;
            self.current ^= 1;
        }
        self.retained = None;
        Ok(())
    }

    pub fn load(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        request: MusicRequest,
        primary: Vec<u8>,
        secondary: Option<Vec<u8>>,
    ) -> Result<()> {
        if request.duration == 0 {
            self.stop(host, client)?;
        }
        let channel = self.current ^ 1;
        let ticket = host.begin_audio(
            client,
            request.recipe(channel, primary, secondary, self.settings.gain()),
        )?;
        self.pending = Some((ticket, request, channel));
        Ok(())
    }

    pub fn poll(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
    ) -> Result<bool> {
        let Some((ticket, request, channel)) = &self.pending else {
            return Ok(false);
        };
        let status = host.audio_preparation(client, *ticket)?;
        ensure!(
            status.channel == *channel,
            "CMVS music receipt channel differs"
        );
        if status.phase == AudioPreparePhase::Pending {
            return Ok(true);
        }
        host.release_audio(client, *ticket)?;
        ensure!(
            status.phase == AudioPreparePhase::Ready,
            "CMVS music preparation failed: {:?}",
            status.error
        );
        if request.duration != 0 {
            host.submit_audio(
                client,
                AudioRequest::Stop {
                    channel: self.current,
                    fade_ms: request.duration as f32,
                },
            )?;
        }
        self.current = *channel;
        request.complete(vm)?;
        self.retained = Some(request.clone());
        self.sources[*channel as usize] = Some(request.clone());
        self.pending = None;
        Ok(true)
    }

    /// `0x20a3` (0x1400a1f90): the current slot's playback time in milliseconds, -1 when
    /// it is not playing, or the time since the last request while music is off.
    pub fn elapsed(&self, host: &dyn HostServices, client: ClientId, now: u64) -> Result<u32> {
        if self.settings.enabled {
            position(host, client, MUSIC_CHANNELS[self.current as usize])
        } else {
            Ok(now.saturating_sub(self.disabled_since) as u32)
        }
    }

    pub fn remember_disabled_at(&mut self, request: MusicRequest, now: u64) {
        self.disabled_since = now;
        self.remember_disabled(request);
    }

    pub fn remember_disabled(&mut self, request: MusicRequest) {
        self.retained = Some(request);
        self.current ^= 1;
    }

    pub fn clear_playback(&mut self) {
        if self.settings.enabled {
            self.current = 0;
        }
        self.sources = Default::default();
    }

    pub fn reopen_request(&self) -> Option<MusicRequest> {
        self.retained.clone().map(|mut request| {
            request.duration = 0;
            request.looping = true;
            request
        })
    }

    pub fn commit_reopen(&mut self, request: MusicRequest) {
        self.settings.enabled = true;
        self.current = 1;
        self.sources = [None, Some(request.clone())];
        self.retained = Some(request);
    }
}

pub(super) struct AudioCheckpoint {
    _music_index: u32,
    _music: [Option<MusicRequest>; 2],
    _effects: [Option<EffectSource>; 6],
    _effect_mix: [EffectMix; 6],
    _voices: [Option<VoiceRequest>; 4],
    _transport: [Option<AudioPlaybackInfo>; 12],
}

impl AudioCheckpoint {
    pub fn music(&self) -> Option<&MusicRequest> {
        self._music.get(self._music_index as usize)?.as_ref()
    }

    pub fn capture(
        host: &dyn HostServices,
        client: ClientId,
        music: &Music,
        effects: &Effects,
        voices: &Voices,
    ) -> Result<Self> {
        ensure!(
            music.pending.is_none()
                && effects.pending.is_none()
                && effects.restart.is_none()
                && voices.pending.is_none(),
            "CMVS audio preparation in progress"
        );
        let mut transport = [None; 12];
        for (i, channel) in [0, 1].into_iter().chain(5..15).enumerate() {
            transport[i] = host.audio_info(client, channel)?;
        }
        Ok(Self {
            _music_index: music.current,
            _music: music.sources.clone(),
            _effects: effects.sources.clone(),
            _effect_mix: effects.mix_words,
            _voices: voices.sources.clone(),
            _transport: transport,
        })
    }
}

/// BGM ducking while voices play (0x1400a07f0 in the 2021 build, 0x140087df0 in 2016).
/// `0x2238..=0x223f` stage new parameters that the next idle tick adopts.
#[derive(Clone)]
pub(super) struct Ducking {
    /// Live level in percent, fade and hold milliseconds and the fading-voice flag.
    live: [u32; 4],
    staged: Option<[u32; 4]>,
    /// 0 idle, 1 lowering, 2 lowered, 3 holding, 4 restoring.
    state: u8,
    since: u64,
    from: u32,
    factor: u32,
}

impl Default for Ducking {
    fn default() -> Self {
        Self { live: [128, 500, 1000, 0], staged: None, state: 0, since: 0, from: 256, factor: 256 }
    }
}

impl Ducking {
    /// 0x14009f640: staged values while present, otherwise the live ones.
    pub fn parameters(&self) -> [u32; 4] {
        self.staged.unwrap_or(self.live)
    }

    /// 0x1400a0b60 clamps the level to 100 and stages all four words.
    pub fn stage(&mut self, mut values: [u32; 4]) {
        values[0] = values[0].min(100);
        self.staged = Some(values);
    }

    /// Q8 multiplier for music slots 0 and 1 after one mixer tick.
    pub fn advance(&mut self, now: u64, voices: bool) -> u32 {
        let [level, fade, hold, _] = self.live;
        let target = (level << 8) / 100;
        let elapsed = now.saturating_sub(self.since) as u32;
        let factor = match self.state {
            1 if elapsed < fade => self.from - elapsed * (self.from.saturating_sub(target)) / fade,
            1 => {
                self.state = 2;
                target
            }
            2 if voices => target,
            2 => {
                self.state = 3;
                self.since = now;
                target
            }
            3 => {
                if elapsed >= hold {
                    self.state = 4;
                    self.from = target;
                    self.since = now;
                }
                if voices {
                    self.state = 2;
                }
                target
            }
            4 => {
                let factor = if elapsed >= fade {
                    self.state = 0;
                    256
                } else {
                    self.from + (256 - self.from) * elapsed / fade
                };
                if voices {
                    self.state = 1;
                    self.from = factor;
                    self.since = now;
                }
                factor
            }
            _ => {
                if let Some(staged) = self.staged.take() {
                    self.live = staged;
                }
                if self.live[0] < 100 && voices {
                    self.state = 1;
                    self.from = 256;
                    self.since = now;
                }
                256
            }
        };
        self.factor = factor;
        factor
    }

    /// Whether voices flagged as fading out also hold the music down.
    pub fn counts_fading(&self) -> bool {
        self.live[3] != 0
    }

    pub fn factor(&self) -> f32 {
        self.factor as f32 / 256.
    }
}

#[cfg(test)]
mod ducking_tests {
    use super::Ducking;

    #[test]
    fn music_lowers_holds_and_restores() {
        let mut ducking = Ducking::default();
        assert_eq!(ducking.advance(0, true), 256, "the default level does not duck");
        ducking.stage([50, 100, 200, 0]);
        assert_eq!(ducking.parameters(), [50, 100, 200, 0]);
        assert_eq!(ducking.advance(10, true), 256);
        assert_eq!(ducking.advance(60, true), 192);
        assert_eq!(ducking.advance(110, true), 128);
        assert_eq!(ducking.advance(120, true), 128);
        assert_eq!(ducking.advance(130, false), 128);
        assert_eq!(ducking.advance(330, false), 128);
        assert_eq!(ducking.advance(380, false), 192);
        assert_eq!(ducking.advance(430, false), 256);
    }
}

/// Channel count from an Ogg Vorbis identification header or a RIFF WAVE `fmt ` chunk.
fn source_channels(bytes: &[u8]) -> u32 {
    if let Some(at) = bytes.windows(7).take(256).position(|window| window == b"\x01vorbis") {
        if let Some(&channels) = bytes.get(at + 11) {
            return u32::from(channels.max(1));
        }
    }
    if bytes.starts_with(b"RIFF") {
        if let Some(at) = bytes.windows(4).take(4096).position(|window| window == b"fmt ") {
            if let Some(channels) = bytes.get(at + 10..at + 12) {
                return u32::from(u16::from_le_bytes([channels[0], channels[1]]).max(1));
            }
        }
    }
    1
}

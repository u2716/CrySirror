//! BGI A0 transport, gain-bank and resident-slot policy over engine audio.
use super::*;
use crysirror_audio::{
    legacy_envelopes::{GainRamp128, compose_128_factors},
    wave_box::WaveBoxInfo,
};
use crysirror_bgi_vm::HostMemoryWrite;
use crysirror_core::service::{
    AudioParameters, AudioPreparePhase, AudioRequest, AudioResourceHandle, AudioResourceInfo,
    EncodedAudio, PcmBus,
};

#[derive(Clone)]
struct Slot {
    source: EncodedAudio,
    intro: Option<EncodedAudio>,
    resource: AudioResourceHandle,
    info: AudioResourceInfo,
    wave: Option<WaveBoxInfo>,
    rate: f32,
    decode_gain: f32,
    pan: f32,
    looping: bool,
    needs_restart: bool,
    native_start: i32,
}
struct Loading {
    files: Option<Task<Vec<(String, Option<Vec<u8>>)>>>,
    bytes: Vec<(String, Option<Vec<u8>>)>,
    resources: Vec<(AudioResourceHandle, AudioResourceInfo)>,
    ticket: Option<u64>,
    index: usize,
}
struct Playback {
    ticket: u64,
    channel: u32,
    result: i32,
}
struct PendingCall {
    loading: Option<Loading>,
    playback: Option<Playback>,
    cd_scan: Option<Task<BTreeMap<u8, (String, Arc<[u8]>)>>>,
}
#[derive(Default)]
pub(crate) struct AudioState {
    pending: BTreeMap<u64, PendingCall>,
    slots: BTreeMap<u32, Slot>,
    banks: BTreeMap<u32, [u8; 4]>,
    ramps: BTreeMap<(u32, usize), GainRamp128>,
    loading: Option<Loading>,
    playback: Option<Playback>,
    pub(super) cd_scan: Option<Task<BTreeMap<u8, (String, Arc<[u8]>)>>>,
    pub(super) cd_tracks: BTreeMap<u8, (String, Arc<[u8]>)>,
    pub(super) cd_open: bool,
    pub(super) cd_mode: i32,
    pub(super) cd_track: Option<u8>,
    pub(super) cd_notify: bool,
}
const CD_CHANNEL: u32 = 0x1fe;
fn channel(value: i32, se: bool) -> Result<u32> {
    ensure!(
        (value as u32) < if se { 64 } else { 16 },
        "BGI {} channel {value}",
        if se { "SE" } else { "BGM" }
    );
    Ok(value as u32 + if se { 0x100 } else { 0 })
}
fn volume(value: i32) -> Result<u8> {
    ensure!((value as u32) <= 128, "BGI volume {value}");
    Ok(value as u8)
}
fn pan(value: i32) -> Result<f32> {
    ensure!((value as u32) <= 128, "BGI pan {value}");
    Ok(value as f32 / 128.0)
}
fn fixed(value: i32) -> f32 {
    value as f32 / 65536.0
}

impl AudioState {
    pub(super) fn enter_call(&mut self, id: u64) {
        if let Some(work) = self.pending.remove(&id) {
            self.loading = work.loading;
            self.playback = work.playback;
            self.cd_scan = work.cd_scan;
        }
    }
    pub(super) fn leave_call(&mut self, id: u64, complete: bool) {
        let work = PendingCall {
            loading: self.loading.take(),
            playback: self.playback.take(),
            cd_scan: self.cd_scan.take(),
        };
        if !complete {
            self.pending.insert(id, work);
        }
    }
    pub(super) fn cd_ticket(&mut self, ticket: u64, result: i32) {
        self.playback = Some(Playback {
            ticket,
            channel: CD_CHANNEL,
            result,
        });
    }
    fn gain(&self, channel: u32) -> f32 {
        compose_128_factors(self.banks.get(&channel).copied().unwrap_or([128; 4])) as f32
    }
    fn bank(&mut self, channel: u32, index: usize, value: u8) {
        self.banks.entry(channel).or_insert([128; 4])[index] = value;
        self.ramps.remove(&(channel, index));
    }
    pub(crate) fn tick(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<()> {
        let now = host.now_ms() as u32;
        let ramps = self
            .ramps
            .iter()
            .map(|(&key, &ramp)| (key, ramp.sample(now)))
            .collect::<Vec<_>>();
        let mut changed = std::collections::BTreeSet::new();
        for ((channel, index), (value, done)) in ramps {
            let banks = self.banks.entry(channel).or_insert([128; 4]);
            if banks[index] != value {
                banks[index] = value;
                changed.insert(channel);
            }
            if done {
                self.ramps.remove(&(channel, index));
            }
        }
        for channel in changed {
            if self.slots.contains_key(&channel) {
                host.submit_audio(
                    client,
                    AudioRequest::SetGain {
                        channel,
                        gain: self.gain(channel),
                        fade_ms: 0.0,
                    },
                )?;
            }
        }
        Ok(())
    }
}

impl BgiVm {
    pub(super) fn audio_results(
        vm: &mut Machine,
        call: &NativeCall,
        result: i32,
        writes: Vec<HostMemoryWrite>,
    ) -> Result<()> {
        let outputs = match call.outputs {
            0 => vec![],
            1 => vec![result],
            _ => bail!("BGI sound native result ABI"),
        };
        vm.commit_call_writes(&outputs, writes)
    }
    pub(super) fn poll_audio_playback(
        &mut self,
        host: &mut dyn HostServices,
        vm: &mut Machine,
        call: &NativeCall,
    ) -> Result<bool> {
        let pending = self
            .audio
            .playback
            .as_ref()
            .context("BGI audio publication")?;
        let state = host.audio_preparation(self.client, pending.ticket)?;
        match state.phase {
            AudioPreparePhase::Pending => Ok(false),
            AudioPreparePhase::Failed => {
                let pending = self.audio.playback.take().unwrap();
                host.release_audio(self.client, pending.ticket)?;
                bail!("BGI playback failed: {}", state.error.unwrap_or_default())
            }
            AudioPreparePhase::Ready => {
                let pending = self.audio.playback.take().unwrap();
                host.release_audio(self.client, pending.ticket)?;
                if let Some(slot) = self.audio.slots.get_mut(&pending.channel) {
                    slot.needs_restart = false;
                }
                Self::audio_results(vm, call, pending.result, vec![])?;
                Ok(true)
            }
        }
    }
    fn start_audio_slot(
        &mut self,
        host: &mut dyn HostServices,
        vm: &mut Machine,
        call: &NativeCall,
        channel: u32,
        result: i32,
    ) -> Result<bool> {
        let slot = self
            .audio
            .slots
            .get(&channel)
            .context("BGI resident sound slot")?;
        let parameters = AudioParameters {
            gain: self.audio.gain(channel),
            decode_gain: slot.decode_gain,
            rate: slot.rate,
            pan: slot.pan,
            looping: slot.looping,
            loop_start: slot.wave.map_or(0, |wave| u64::from(wave.loop_start)),
            retain_on_finish: true,
            bus: if channel < 0x100 {
                PcmBus::Music
            } else {
                PcmBus::Sfx
            },
            ..Default::default()
        };
        let request = if let Some(intro) = &slot.intro {
            AudioRequest::Play {
                channel,
                source: slot.source.clone(),
                intro: Some(intro.clone()),
                parameters,
            }
        } else {
            AudioRequest::PlayResource {
                channel,
                resource: slot.resource,
                parameters,
            }
        };
        let ticket = host.begin_audio(self.client, request)?;
        self.audio.playback = Some(Playback {
            ticket,
            channel,
            result,
        });
        self.poll_audio_playback(host, vm, call)
    }
    fn begin_sound_load(
        &mut self,
        host: &mut dyn HostServices,
        vm: &Machine,
        call: &NativeCall,
    ) -> Result<()> {
        let a = &call.args;
        let channel = if call.id == 0xc0 {
            0x13f
        } else {
            channel(a[0], call.id >= 0x20)?
        };
        let _ = channel;
        match call.id {
            0x10 => {
                volume(a[2])?;
            }
            0x11 => {
                pan(a[4])?;
                volume(a[3])?;
            }
            0x12 => {
                pan(a[6])?;
                volume(a[5])?;
            }
            _ => (),
        }
        let mut files = match call.id {
            0x10 => vec![(String::new(), vm.string(a[1])?)],
            0x11 | 0x20 | 0x21 | 0x23 | 0x27 => vec![(vm.string(a[1])?, vm.string(a[2])?)],
            0x12 => {
                let archive = vm.string(a[1])?;
                let intro = vm.string(a[2])?;
                let loop_file = vm.string(a[3])?;
                vec![
                    (archive.clone(), intro.clone()),
                    (
                        archive,
                        if loop_file.is_empty() {
                            intro
                        } else {
                            loop_file
                        },
                    ),
                ]
            }
            0x28 => {
                let header = vm.memory(a[1] as u32, 64)?;
                let length = u32::from_le_bytes(header[..4].try_into().unwrap()) as usize
                    + u32::from_le_bytes(header[8..12].try_into().unwrap()) as usize;
                self.audio.loading = Some(Loading {
                    files: None,
                    bytes: vec![("memory.bw".into(), Some(vm.memory(a[1] as u32, length)?))],
                    resources: vec![],
                    ticket: None,
                    index: 0,
                });
                return Ok(());
            }
            0xc0 => vec![(String::new(), vm.string(a[0])?)],
            _ => bail!("BGI sound load native"),
        };
        if files.len() == 2 && files[0] == files[1] {
            files.pop();
        }
        let storage = host.storage(self.client)?;
        let config = self.config.clone();
        let client = self.client;
        let files = self.pool.spawn(move |cancel| {
            let mut result = Vec::new();
            for (archive, file) in files {
                cancel.check()?;
                let bytes =
                    program_io::read_raw(storage.as_ref(), client, &config, &archive, &file)?
                        .map(program_io::decode)
                        .transpose()?;
                result.push((file, bytes));
            }
            cancel.check()?;
            Ok(result)
        })?;
        self.audio.loading = Some(Loading {
            files: Some(files),
            bytes: vec![],
            resources: vec![],
            ticket: None,
            index: 0,
        });
        Ok(())
    }
    fn poll_sound_load(
        &mut self,
        host: &mut dyn HostServices,
        vm: &mut Machine,
        call: &NativeCall,
    ) -> Result<bool> {
        if self.audio.loading.is_none() {
            self.begin_sound_load(host, vm, call)?;
        }
        let load = self.audio.loading.as_mut().unwrap();
        if let Some(task) = &mut load.files {
            let Some(bytes) = task.try_take()? else {
                return Ok(false);
            };
            load.bytes = bytes;
            load.files = None;
        }
        while load.index < load.bytes.len() {
            let (name, bytes) = &load.bytes[load.index];
            let Some(bytes) = bytes else {
                if call.id < 0x20 {
                    bail!("BGI required music is missing: {name}");
                }
                self.audio.loading = None;
                if call.id != 0xc0 {
                    let channel = channel(call.args[0], true)?;
                    host.submit_audio(
                        self.client,
                        AudioRequest::Stop {
                            channel,
                            fade_ms: 0.0,
                        },
                    )?;
                    if let Some(old) = self.audio.slots.remove(&channel) {
                        host.release_audio_resource(self.client, old.resource)?;
                    }
                }
                Self::audio_results(vm, call, 0, vec![])?;
                return Ok(true);
            };
            let ticket = if let Some(ticket) = load.ticket {
                ticket
            } else {
                let ticket = host.begin_encoded_audio_resource(
                    self.client,
                    name,
                    EncodedAudio {
                        bytes: Arc::from(bytes.clone()),
                        frame_limit: None,
                    },
                    true,
                )?;
                load.ticket = Some(ticket);
                ticket
            };
            let state = host.audio_resource_preparation(self.client, ticket)?;
            match state.phase {
                AudioPreparePhase::Pending => return Ok(false),
                AudioPreparePhase::Failed => {
                    host.release_audio_resource_preparation(self.client, ticket)?;
                    bail!(
                        "BGI invalid sound {name}: {}",
                        state.error.unwrap_or_default()
                    );
                }
                AudioPreparePhase::Ready => {
                    let resource = state.resource.context("BGI sound preparation resource")?;
                    let info = host.audio_resource_info(self.client, resource)?;
                    load.resources.push((resource, info));
                    host.release_audio_resource_preparation(self.client, ticket)?;
                    load.ticket = None;
                    load.index += 1;
                }
            }
        }
        let mut load = self.audio.loading.take().unwrap();
        let a = &call.args;
        let channel = if call.id == 0xc0 {
            0x13f
        } else {
            channel(a[0], call.id >= 0x20)?
        };
        let pan = match call.id {
            0x11 => pan(a[4])?,
            0x12 => pan(a[6])?,
            _ => 0.5,
        };
        let gain = match call.id {
            0x10 => volume(a[2])?,
            0x11 => volume(a[3])?,
            0x12 => volume(a[5])?,
            _ => 128,
        };
        let rate = match call.id {
            0x23 => 2.0,
            0x27 => fixed(a[5]),
            0x28 => fixed(a[4]),
            _ => 1.0,
        };
        let decode_gain = match call.id {
            0x21 | 0x23 | 0x27 => fixed(a[4]),
            0x28 => fixed(a[3]),
            _ => 1.0,
        };
        let native_start = if matches!(call.id, 0x21 | 0x23 | 0x27) {
            a[3]
        } else if call.id == 0x28 {
            a[2]
        } else {
            0
        };
        let looping = call.id == 0x11 || (call.id == 0x12 && a[4] != 0);
        let mut intro = None;
        if load.resources.len() == 2 {
            let bytes = load.bytes[0].1.take().unwrap();
            intro = Some(EncodedAudio {
                bytes: Arc::from(bytes),
                frame_limit: None,
            });
            let (handle, _) = load.resources.remove(0);
            host.release_audio_resource(self.client, handle)?;
        }
        let index = load.bytes.len() - 1;
        let bytes = load.bytes[index].1.take().unwrap();
        let wave = WaveBoxInfo::parse(&bytes);
        let (resource, info) = load.resources.remove(0);
        host.submit_audio(
            self.client,
            AudioRequest::Stop {
                channel,
                fade_ms: 0.0,
            },
        )?;
        if let Some(old) = self.audio.slots.insert(
            channel,
            Slot {
                source: EncodedAudio {
                    bytes: Arc::from(bytes),
                    frame_limit: None,
                },
                intro,
                resource,
                info,
                wave,
                rate,
                decode_gain,
                pan,
                looping,
                needs_restart: true,
                native_start,
            },
        ) {
            host.release_audio_resource(self.client, old.resource)?;
        }
        self.audio.bank(channel, 2, gain);
        self.audio.bank(channel, 3, 128);
        if call.id == 0xc0 {
            self.start_audio_slot(host, vm, call, channel, 1)
        } else {
            Self::audio_results(vm, call, 0, vec![])?;
            Ok(true)
        }
    }

    pub(crate) fn audio_call(
        &mut self,
        host: &mut dyn HostServices,
        vm: &mut Machine,
        call: &NativeCall,
    ) -> Result<bool> {
        if self.audio.playback.is_some() {
            return self.poll_audio_playback(host, vm, call);
        }
        let a = &call.args;
        let id = call.id;
        if matches!(
            id,
            0x10 | 0x11 | 0x12 | 0x20 | 0x21 | 0x23 | 0x27 | 0x28 | 0xc0
        ) {
            return self.poll_sound_load(host, vm, call);
        }
        let mut result = 0;
        let mut writes = vec![];
        match id {
            0x00 => result = 20,
            0x08 | 0x09 | 0x1c | 0x2c => {
                let value = volume(a[1])?;
                let channel = channel(a[0], matches!(id, 0x09 | 0x2c))?;
                self.audio
                    .bank(channel, usize::from(matches!(id, 0x1c | 0x2c)), value);
                if self.audio.slots.contains_key(&channel) {
                    host.submit_audio(
                        self.client,
                        AudioRequest::SetGain {
                            channel,
                            gain: self.audio.gain(channel),
                            fade_ms: 0.0,
                        },
                    )?;
                }
            }
            0x14 => {
                let channel = channel(a[0], false)?;
                if let Some(slot) = self.audio.slots.get(&channel) {
                    if a[1] != 0 && slot.needs_restart {
                        return self.start_audio_slot(host, vm, call, channel, 0);
                    }
                    if !slot.needs_restart {
                        host.submit_audio(
                            self.client,
                            AudioRequest::Pause {
                                channel,
                                paused: a[1] == 0,
                                fade_ms: 0.0,
                            },
                        )?;
                    }
                }
            }
            0x15 => {
                let info = if (a[0] as u32) < 16 {
                    host.audio_info(self.client, a[0] as u32)?
                } else {
                    None
                };
                let playing = info
                    .as_ref()
                    .is_some_and(|info| info.source_playing && !info.paused);
                result = i32::from(!playing);
                if a[1] != 0 {
                    let position = info.filter(|_| playing).map_or(0, |info| {
                        (info.position_frames.saturating_mul(1000)
                            / u64::from(info.source_rate.max(1)))
                        .min(i32::MAX as u64) as i32
                    });
                    writes.push(HostMemoryWrite {
                        address: a[1] as u32,
                        bytes: position.to_le_bytes().to_vec(),
                    });
                }
            }
            0x16 | 0x18 | 0x19 | 0x26 => {
                let value = if id == 0x16 {
                    volume(a[1])?
                } else if id == 0x18 {
                    128
                } else {
                    0
                };
                let channel = channel(a[0], id == 0x26)?;
                if self.audio.slots.contains_key(&channel) {
                    let index = if id == 0x16 { 2 } else { 3 };
                    let duration = if id == 0x16 { a[2] } else { a[1] };
                    let current = self.audio.banks.entry(channel).or_insert([128; 4])[index];
                    self.audio.ramps.insert(
                        (channel, index),
                        GainRamp128::new(host.now_ms() as u32, duration, current, value),
                    );
                }
            }
            0x17 => {
                let pan = pan(a[1])?;
                let channel = channel(a[0], false)?;
                if let Some(slot) = self.audio.slots.get_mut(&channel) {
                    slot.pan = pan;
                    if !slot.needs_restart {
                        host.submit_audio(
                            self.client,
                            AudioRequest::SetPan {
                                channel,
                                pan,
                                fade_ms: 0.0,
                            },
                        )?;
                    }
                }
            }
            0x22 | 0x25 => {
                let channel = channel(a[0], true)?;
                if self.audio.slots.contains_key(&channel) {
                    host.submit_audio(
                        self.client,
                        AudioRequest::Stop {
                            channel,
                            fade_ms: 0.0,
                        },
                    )?;
                }
                if id == 0x22 {
                    if let Some(slot) = self.audio.slots.remove(&channel) {
                        host.release_audio_resource(self.client, slot.resource)?;
                    }
                } else if let Some(slot) = self.audio.slots.get_mut(&channel) {
                    slot.needs_restart = true;
                }
            }
            0x24 => {
                let pan = pan(a[2])?;
                let volume = volume(a[1])?;
                let channel = channel(a[0], true)?;
                if self.audio.slots.contains_key(&channel) {
                    self.audio.bank(channel, 2, volume);
                    self.audio.bank(channel, 3, 128);
                    self.audio.slots.get_mut(&channel).unwrap().pan = pan;
                    let result = self.sound_length(channel);
                    return self.start_audio_slot(host, vm, call, channel, result);
                }
            }
            0x2f => {
                let channel = channel(a[0], true)?;
                result = self.sound_length(channel);
            }
            0x80 | 0x81 | 0x84 | 0x85 | 0x86 => return self.cd_audio_call(host, vm, call),
            _ => bail!("BGI unknown native sound A0:{id:02x}"),
        }
        Self::audio_results(vm, call, result, writes)?;
        self.calls += 1;
        Ok(true)
    }
    fn sound_length(&self, channel: u32) -> i32 {
        let Some(slot) = self.audio.slots.get(&channel) else {
            return 0;
        };
        let Some(wave) = slot.wave else { return 0 };
        if wave.sample_rate == 0 || slot.rate == 0.0 {
            return 0;
        }
        let scale = (65536.0 / f64::from(slot.rate)) as i64 as u32;
        let value =
            f64::from(wave.frames) * 1000.0 / f64::from(wave.sample_rate) * f64::from(scale);
        ((value as i64 as u32) >> 16) as i32
    }
}

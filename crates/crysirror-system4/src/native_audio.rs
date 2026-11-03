// Native SACT2/audio.c/audio_mixer.c policy from xsystem4 a85467d (GPL2+).
// (C) 2019 Nunuhara Cabbage; Rust (C) 2026 CrySirror contributors.
// Resources, decoder/mixer/loops/faders and mix groups are CrySirror services.
mod lifecycle;

use crate::{
    adapter::{Bridge, Dispatch},
    assets::Class,
    bootstrap::Bootstrap,
    vm::{
        external::{Argument, HllReturn},
        ExternalCall, ExternalTarget, Machine,
    },
};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{
        AudioAsset, AudioMixGroupSettings, AudioParameters, AudioPlaybackInfo, AudioPreparePhase,
        AudioRequest, AudioResourceHandle, AudioResourceInfo, ClientStorage, HostServices, PcmBus,
    },
    state::ClientId,
};
use crysirror_formats_arc::alice_audio_metadata::{MusicLoopCatalog, WaveRoutingCatalog};
use crysirror_resource::DecodeContext;
use crysirror_tasks::{Cancellation, TaskPool};
use std::{collections::BTreeMap, sync::Arc};
#[derive(Default)]
pub(crate) struct AudioMetadata {
    pub(crate) music: Option<MusicLoopCatalog>,
    pub(crate) wave: Option<WaveRoutingCatalog>,
    pub(crate) reviewed_music: Option<crate::native_audio_profile::ReviewedMusic>,
}
impl AudioMetadata {
    pub(crate) fn load(
        owner: ClientId,
        storage: &dyn ClientStorage,
        cancel: Cancellation,
    ) -> Result<Self> {
        let names = storage.list(Mount::Asset, "")?;
        ensure!(
            names.len() <= 8192,
            "System4 root metadata enumeration budget"
        );
        let cx = DecodeContext::new(owner).with_cancellation(cancel.clone());
        let mut out = Self::default();
        for name in names {
            cancel.check()?;
            let lower = name.to_ascii_lowercase();
            if lower.ends_with(".bgi") {
                ensure!(out.music.is_none(), "System4 ambiguous music metadata");
                let reader = storage.open_asset(&VfsPath::new(Mount::Asset, &name)?)?;
                out.music = Some(MusicLoopCatalog::load(reader.as_ref(), &cx)?);
            } else if lower.ends_with(".wai") {
                ensure!(out.wave.is_none(), "System4 ambiguous wave metadata");
                let reader = storage.open_asset(&VfsPath::new(Mount::Asset, &name)?)?;
                out.wave = Some(WaveRoutingCatalog::load(reader.as_ref(), &cx)?);
            }
        }
        Ok(out)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Music,
    Sound,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    kind: Kind,
    number: i32,
}
impl Key {
    fn channel(self) -> Result<u32> {
        ensure!(
            (0..65536).contains(&self.number),
            "System4 audio native channel budget"
        );
        Ok(0x5200_0000 + self.number as u32 + if self.kind == Kind::Music { 65536 } else { 0 })
    }
}
struct Channel {
    resource: AudioResourceHandle,
    info: AudioResourceInfo,
    data_number: i32,
    position: u64,
    hold_position: bool,
    count: u32,
    loop_start: u64,
    loop_end: u64,
    group: usize,
    volume: u8,
    swapped: bool,
    playing: bool,
}
struct Group {
    name: Vec<u8>,
    volume: i32,
    default_volume: i32,
    muted: bool,
    parent: Option<usize>,
}
enum Pending {
    Locate {
        call: ExternalCall,
        key: Key,
        task: crysirror_tasks::Task<Option<VfsPath>>,
        resolved: Option<Option<VfsPath>>,
    },
    Prepare {
        call: ExternalCall,
        key: Key,
        data_number: i32,
        ticket: Option<u64>,
        resource: Option<AudioResourceHandle>,
    },
    Warm {
        call: ExternalCall,
        key: Key,
        resource_ticket: Option<u64>,
        voice_ticket: Option<u64>,
        resource: Option<AudioResourceHandle>,
        channel: Option<Channel>,
        ready: bool,
        stopped: bool,
    },
    Transport {
        call: ExternalCall,
        key: Key,
        ticket: Option<u64>,
        ready: bool,
        playing: bool,
        reported_position: Option<u64>,
    },
}
pub(crate) struct NativeAudio {
    owner: ClientId,
    metadata: AudioMetadata,
    groups: Vec<Group>,
    exposed_groups: usize,
    groups_ready: bool,
    channels: BTreeMap<Key, Channel>,
    pending: Option<Pending>,
    reviewed_absent_calls: u64,
    // File lookup belongs on this shared IO pool; no ambient filesystem exists.
    _pool: TaskPool,
}
impl NativeAudio {
    pub(crate) fn new(owner: ClientId, pool: TaskPool) -> Self {
        Self {
            owner,
            metadata: AudioMetadata::default(),
            groups: Vec::new(),
            exposed_groups: 0,
            groups_ready: false,
            channels: BTreeMap::new(),
            pending: None,
            reviewed_absent_calls: 0,
            _pool: pool,
        }
    }
    pub(crate) fn configure(
        &mut self,
        bootstrap: &Bootstrap,
        default_volume: i32,
        metadata: AudioMetadata,
    ) -> Result<()> {
        ensure!(
            self.channels.is_empty() && self.pending.is_none(),
            "System4 audio configured after native resources"
        );
        self.exposed_groups = bootstrap.mixers.len();
        let mut groups = bootstrap
            .mixers
            .iter()
            .map(|m| {
                Ok(Group {
                    name: cp932(&m.name)?,
                    volume: m.volume,
                    default_volume: m.volume,
                    muted: false,
                    parent: None,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        if groups.is_empty() {
            for name in ["Music", "Sound", "Master"] {
                groups.push(Group {
                    name: name.as_bytes().to_vec(),
                    volume: default_volume,
                    default_volume,
                    muted: false,
                    parent: None,
                });
            }
        }
        let master = if groups[0].name == b"Master"
            || groups[0].name == b"\x83\x7d\x83\x58\x83\x5e\x81\x5b"
        {
            0
        } else if self.exposed_groups == 0 {
            2
        } else {
            groups.push(Group {
                name: b"Master".to_vec(),
                volume: default_volume,
                default_volume,
                muted: false,
                parent: None,
            });
            groups.len() - 1
        };
        let mut parent = master;
        for (index, group) in groups.iter_mut().enumerate() {
            if index == master {
                continue;
            }
            group.parent = Some(parent);
            if group.name == b"Voice" || group.name == b"\x89\xb9\x90\xba" {
                parent = index;
            }
        }
        self.groups = groups;
        self.metadata = metadata;
        Ok(())
    }
    fn group_name(index: usize) -> String {
        format!("__system4/mixer/{index}")
    }
    fn ensure_groups(&mut self, host: &mut dyn HostServices) -> Result<()> {
        if self.groups_ready {
            return Ok(());
        }
        ensure!(
            !self.groups.is_empty(),
            "System4 original audio configuration absent"
        );
        let master = self
            .groups
            .iter()
            .position(|g| g.parent.is_none())
            .context("System4 audio master group absent")?;
        for index in std::iter::once(master).chain((0..self.groups.len()).filter(|n| *n != master))
        {
            let g = &self.groups[index];
            host.submit_audio(
                self.owner,
                AudioRequest::SetMixGroup {
                    name: Self::group_name(index),
                    settings: AudioMixGroupSettings {
                        parent: g.parent.map(Self::group_name),
                        gain: g.volume.clamp(0, 100) as f32 / 100.,
                        muted: g.muted,
                    },
                    fade_ms: 0.,
                },
            )?;
        }
        self.groups_ready = true;
        Ok(())
    }
    pub(crate) fn dispatch_reviewed_unavailable(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        bridge: &Bridge,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        let ExternalTarget::Hll { library, function } = call.target else {
            return Ok(None);
        };
        let program = machine.program.clone();
        let library = &program.libraries[library];
        let function = &library.functions[function];
        if library.name != b"SACT2"
            || !matches!(
                function.name.as_slice(),
                b"Music_Pause"
                    | b"Music_Restart"
                    | b"Music_IsPause"
                    | b"Music_GetLoopCount"
                    | b"Music_GetSamplePos"
                    | b"Music_GetSampleLength"
                    | b"Music_Seek"
            )
        {
            return Ok(None);
        }
        if self.metadata.reviewed_music.is_none()
            && matches!(
                function.name.as_slice(),
                b"Music_GetLoopCount" | b"Music_GetSamplePos" | b"Music_GetSampleLength"
            )
        {
            return Ok(None);
        }
        ensure!(
            owner == self.owner && self.pending.is_none(),
            "System4 foreign/stale reviewed music call"
        );
        ensure!(
            bridge.sact.as_ref().is_some_and(|s| s.initialized),
            "System4 reviewed music call before SACT init"
        );
        let arguments = machine.hll_arguments(call)?;
        let arity = if function.name == b"Music_Seek" { 2 } else { 1 };
        ensure!(
            function.return_kind == 10
                && arguments.len() == arity
                && arguments
                    .iter()
                    .all(|arg| matches!(arg, Argument::Scalar { kind: 10, .. })),
            "System4 original music optional-interface ABI"
        );
        ensure!(
            self.metadata
                .reviewed_music
                .is_some_and(|p| p.unavailable(&function.name)),
            "unsupported System4 optional music interface profile; original SACT2/MusicSystem pair is unreviewed"
        );
        let count = self
            .reviewed_absent_calls
            .checked_add(1)
            .context("System4 reviewed music call counter")?;
        self.ensure_groups(host)?;
        machine.complete_hll(call, HllReturn::Scalar(0))?;
        self.reviewed_absent_calls = count;
        Ok(Some(Dispatch::Completed))
    }
    // Entire original SystemService mixer family; native names/defaults are
    // policy, while actual gain/mute/hierarchy are shared engine mix groups.
    pub(crate) fn dispatch_mixer(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        let ExternalTarget::Hll { library, function } = call.target else {
            return Ok(None);
        };
        let program = machine.program.clone();
        let library = &program.libraries[library];
        if library.name != b"SystemService" {
            return Ok(None);
        }
        let function = &library.functions[function];
        let name = function.name.as_slice();
        if !matches!(
            name,
            b"GetMixerNumof"
                | b"GetMixerName"
                | b"GetMixerVolume"
                | b"GetMixerDefaultVolume"
                | b"GetMixerMute"
                | b"SetMixerName"
                | b"SetMixerVolume"
                | b"SetMixerMute"
        ) {
            return Ok(None);
        }
        ensure!(owner == self.owner, "System4 foreign mixer owner");
        ensure!(
            matches!(function.return_kind, 10 | 47),
            "System4 mixer return ABI"
        );
        ensure!(
            !self.groups.is_empty(),
            "System4 original mixer configuration absent"
        );
        let args = machine.hll_arguments(call)?;
        if name == b"GetMixerNumof" {
            ensure!(args.is_empty(), "System4 mixer count ABI");
            machine.complete_hll(call, HllReturn::Scalar(self.exposed_groups as i32))?;
            return Ok(Some(Dispatch::Completed));
        }
        let Some(Argument::Scalar {
            kind: 10,
            bits: index,
        }) = args.first()
        else {
            anyhow::bail!("System4 mixer index ABI")
        };
        ensure!(args.len() == 2, "System4 mixer argument count");
        match (name, &args[1]) {
            (b"GetMixerName", Argument::HeapReference { kind: 20, slot }) => {
                let _ = machine.heap.string(*slot)?;
            }
            (
                b"GetMixerVolume" | b"GetMixerDefaultVolume" | b"GetMixerMute",
                Argument::Reference {
                    kind: 18,
                    page,
                    index,
                },
            ) => {
                let _ = machine.heap.get(*page, *index)?;
            }
            (b"SetMixerName", Argument::String { .. }) => {}
            (b"SetMixerVolume" | b"SetMixerMute", Argument::Scalar { kind: 10, bits: _ }) => {}
            _ => anyhow::bail!("System4 mixer argument ABI"),
        }
        let valid = usize::try_from(*index)
            .ok()
            .filter(|i| *i < self.exposed_groups);
        let Some(index) = valid else {
            machine.complete_hll(call, HllReturn::Scalar(0))?;
            return Ok(Some(Dispatch::Completed));
        };
        self.ensure_groups(host)?;
        let group = &mut self.groups[index];
        match (name, &args[1]) {
            (b"GetMixerName", Argument::HeapReference { slot, .. }) => {
                machine
                    .heap
                    .set_string(*slot, Arc::new(group.name.clone()))?;
            }
            (
                b"GetMixerDefaultVolume",
                Argument::Reference {
                    page, index: field, ..
                },
            ) => {
                machine.heap.set(*page, *field, group.default_volume)?;
            }
            (
                b"GetMixerVolume" | b"GetMixerMute",
                Argument::Reference {
                    page, index: field, ..
                },
            ) => {
                let info = host
                    .audio_mix_group(owner, &Self::group_name(index))?
                    .context("System4 engine mixer group disappeared")?;
                let value = if name == b"GetMixerVolume" {
                    (info.settings.gain * 100.).clamp(0., 100.) as i32
                } else {
                    i32::from(info.settings.muted)
                };
                machine.heap.set(*page, *field, value)?;
            }
            (b"SetMixerName", Argument::String { bytes, .. }) => {
                group.name =
                    bytes[..bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len())].to_vec();
            }
            (b"SetMixerVolume" | b"SetMixerMute", Argument::Scalar { bits, .. }) => {
                let live = host
                    .audio_mix_group(owner, &Self::group_name(index))?
                    .context("System4 engine mixer group disappeared")?;
                let volume = if name == b"SetMixerVolume" {
                    (*bits).clamp(0, 100)
                } else {
                    (live.settings.gain * 100.).clamp(0., 100.) as i32
                };
                let muted = if name == b"SetMixerMute" {
                    *bits != 0
                } else {
                    live.settings.muted
                };
                host.submit_audio(
                    owner,
                    AudioRequest::SetMixGroup {
                        name: Self::group_name(index),
                        settings: AudioMixGroupSettings {
                            parent: group.parent.map(Self::group_name),
                            gain: volume as f32 / 100.,
                            muted,
                        },
                        fade_ms: 0.,
                    },
                )?;
                group.volume = volume;
                group.muted = muted;
            }
            _ => unreachable!(),
        }
        machine.complete_hll(call, HllReturn::Scalar(1))?;
        Ok(Some(Dispatch::Completed))
    }
    fn new_channel(
        &self,
        key: Key,
        number: i32,
        resource: AudioResourceHandle,
        info: AudioResourceInfo,
    ) -> Result<Channel> {
        ensure!(
            resource.client == self.owner.0 && info.source_rate > 0,
            "System4 foreign/invalid audio resource"
        );
        ensure!(
            !info.has_replay_gain,
            "System4 native ReplayGain policy is not reviewed"
        );
        let frames = info
            .source_frames
            .context("System4 native audio needs actual decoded length")?;
        ensure!(frames > 0, "System4 native empty audio");
        let (mut count, mut start, mut end, mut group, mut volume) = (
            if key.kind == Kind::Music { 0 } else { 1 },
            0,
            frames,
            if number < 0 || key.kind == Kind::Music {
                0
            } else {
                1
            },
            100,
        );
        if key.kind == Kind::Music {
            if let Some(entry) = self.metadata.music.as_ref().and_then(|m| m.entry(number)) {
                count = entry.loop_count.max(0) as u32;
                start = (entry.loop_start.max(0) as u64).min(frames);
                end = (entry.loop_end.max(0) as u64).min(frames);
                group = entry.mixer_group.max(0) as usize;
                volume = entry.volume_percent.clamp(0, 100);
            }
        } else if let Some(entry) = u32::try_from(number)
            .ok()
            .and_then(|n| self.metadata.wave.as_ref()?.entry(n))
        {
            ensure!(entry.mixer_group >= 0, "System4 negative wave mixer index");
            group = entry.mixer_group as usize;
        }
        ensure!(
            group < self.groups.len(),
            "System4 audio metadata references missing mixer {group}"
        );
        Ok(Channel {
            resource,
            info,
            data_number: number,
            position: 0,
            hold_position: false,
            count,
            loop_start: start,
            loop_end: end,
            group,
            volume: volume as u8,
            swapped: false,
            playing: false,
        })
    }
    fn retire(&mut self, key: Key, host: &mut dyn HostServices) -> Result<bool> {
        let Some(channel) = self.channels.get(&key) else {
            return Ok(false);
        };
        host.submit_audio(
            self.owner,
            AudioRequest::Stop {
                channel: key.channel()?,
                fade_ms: 0.,
            },
        )?;
        host.release_audio_resource(self.owner, channel.resource)?;
        self.channels.remove(&key);
        Ok(true)
    }
    fn observe(
        &mut self,
        key: Key,
        host: &mut dyn HostServices,
    ) -> Result<Option<AudioPlaybackInfo>> {
        let configured_count = key.kind == Kind::Music && self.metadata.reviewed_music.is_some();
        let channel = self
            .channels
            .get_mut(&key)
            .context("System4 unprepared audio observation")?;
        let info = host.audio_info(self.owner, key.channel()?)?;
        let info = info.context(
            "System4 canonical audio voice disappeared; retained completion is required",
        )?;
        ensure!(
            info.retain_on_finish,
            "System4 host did not retain the canonical audio source"
        );
        if !channel.hold_position {
            channel.position = info.position_frames;
        }
        // Original CMusic::PlayImp decrements a thread-local copy of its
        // configured loop count. A later Play starts that configured count
        // again; the optional public loop-query interface is absent here.
        if !configured_count {
            channel.count = if info.looping {
                info.loop_passes.unwrap_or(0)
            } else {
                1
            };
        }
        channel.volume = (info.gain * 100.).clamp(0., 100.) as u8;
        channel.playing = info.source_playing && info.transport_active && !info.paused;
        Ok(Some(info))
    }
    pub(crate) fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"reviewed_music_profile":self.metadata.reviewed_music.map(|p|format!("{p:?}")),"reviewed_absent_calls":self.reviewed_absent_calls,"groups":self.groups.len(),"exposed_groups":self.exposed_groups,"groups_ready":self.groups_ready,"prepared":self.channels.len(),"pending":self.pending.is_some(),"music_metadata":self.metadata.music.as_ref().map_or(0,|m|m.entries.len()),"wave_metadata":self.metadata.wave.as_ref().map_or(0,|m|m.entries.len())})
    }
    pub(crate) fn has_state(&self) -> bool {
        !self.channels.is_empty() || self.pending.is_some()
    }
    pub(crate) fn close(&mut self, host: &mut dyn HostServices) -> Result<()> {
        self.close_commands(host)?;
        for key in self.channels.keys().copied().collect::<Vec<_>>() {
            self.retire(key, host)?;
        }
        Ok(())
    }
    pub(crate) fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        bridge: &Bridge,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        let ExternalTarget::Hll { library, function } = call.target else {
            ensure!(
                self.pending.is_none(),
                "System4 changed pending audio target"
            );
            return Ok(None);
        };
        let program = machine.program.clone();
        let library = &program.libraries[library];
        let function = &library.functions[function];
        if library.name != b"SACT2"
            || !(function.name.starts_with(b"Music_") || function.name.starts_with(b"Sound_"))
        {
            ensure!(self.pending.is_none(), "System4 changed pending audio HLL");
            return Ok(None);
        }
        ensure!(owner == self.owner, "System4 foreign native audio owner");
        ensure!(
            bridge.sact.as_ref().is_some_and(|s| s.initialized),
            "System4 audio before real SACT init"
        );
        ensure!(function.return_kind == 10, "System4 SACT audio return ABI");
        let arguments = machine.hll_arguments(call)?;
        self.ensure_groups(host)?;
        if let Some(dispatch) = self.poll_command(machine, call, host)? {
            return Ok(Some(dispatch));
        }
        if function.name == b"Sound_PrepareFromFile" {
            return Ok(Some(self.prepare_file(machine, call, &arguments, host)?));
        }
        let values = arguments
            .iter()
            .map(|a| match a {
                Argument::Scalar { kind: 10, bits } => Ok(*bits),
                _ => anyhow::bail!("System4 native audio scalar ABI"),
            })
            .collect::<Result<Vec<_>>>()?;
        let local_name = function
            .name
            .strip_prefix(b"Music_")
            .or_else(|| function.name.strip_prefix(b"Sound_"))
            .unwrap();
        if function.name.starts_with(b"Music_") {
            ensure!(
                !matches!(
                    local_name,
                    b"GetUnuseChannel"
                        | b"PrepareFromFile"
                        | b"ReverseLR"
                        | b"GetVolume"
                        | b"GetTimeLength"
                        | b"GetGroupNum"
                ),
                "unsupported original Music export {}",
                String::from_utf8_lossy(local_name)
            );
        } else {
            ensure!(
                !matches!(
                    local_name,
                    b"SetLoopStartPos"
                        | b"SetLoopEndPos"
                        | b"GetSamplePos"
                        | b"GetSampleLength"
                        | b"MillisecondsToSamples"
                ),
                "unsupported original Sound export {}",
                String::from_utf8_lossy(local_name)
            );
        }
        let arity = match local_name {
            b"IsExist" | b"Unprepare" | b"Play" | b"Stop" | b"IsPlay" | b"GetLoopCount"
            | b"ReverseLR" | b"GetVolume" | b"GetPos" | b"GetLength" | b"GetTimeLength"
            | b"GetSamplePos" | b"GetSampleLength" | b"GetGroupNum" | b"StopFade" | b"IsFade" => 1,
            b"Prepare"
            | b"SetLoopCount"
            | b"SetLoopStartPos"
            | b"SetLoopEndPos"
            | b"MillisecondsToSamples" => 2,
            b"Fade" => 4,
            b"GetUnuseChannel" => 0,
            _ => anyhow::bail!(
                "unsupported original SACT audio export {}",
                String::from_utf8_lossy(local_name)
            ),
        };
        ensure!(values.len() == arity, "System4 native audio argument count");
        let (kind, name) = if let Some(name) = function.name.strip_prefix(b"Music_") {
            (Kind::Music, name)
        } else {
            (Kind::Sound, function.name.strip_prefix(b"Sound_").unwrap())
        };
        if name == b"MillisecondsToSamples" {
            let [ms, rate] = values.as_slice() else {
                anyhow::bail!("System4 milliseconds-to-samples ABI")
            };
            let value = (i64::from(*ms) * i64::from(*rate) / 1000) as i32;
            machine.complete_hll(call, HllReturn::Scalar(value))?;
            return Ok(Some(Dispatch::Completed));
        }
        if name == b"IsExist" {
            let [number] = values.as_slice() else {
                anyhow::bail!("System4 audio existence ABI")
            };
            let class = if kind == Kind::Music {
                Class::Music
            } else {
                Class::Sound
            };
            let found = bridge
                .assets
                .as_ref()
                .context("System4 numbered audio assets absent")?
                .path(class, *number)?
                .is_some();
            machine.complete_hll(call, HllReturn::Scalar(i32::from(found)))?;
            return Ok(Some(Dispatch::Completed));
        }
        if name == b"GetUnuseChannel" {
            ensure!(
                kind == Kind::Sound && values.is_empty(),
                "System4 unused audio channel ABI"
            );
            let number = (0..65536)
                .find(|number| {
                    !self.channels.contains_key(&Key {
                        kind,
                        number: *number,
                    })
                })
                .context("System4 audio channel pool exhausted")?;
            machine.complete_hll(call, HllReturn::Scalar(number))?;
            return Ok(Some(Dispatch::Completed));
        }
        if name == b"Prepare" {
            let [id, number] = values.as_slice() else {
                anyhow::bail!("System4 audio prepare ABI")
            };
            if *id < 0 {
                machine.complete_hll(call, HllReturn::Scalar(0))?;
                return Ok(Some(Dispatch::Completed));
            }
            let key = Key { kind, number: *id };
            key.channel()?;
            let path = bridge
                .assets
                .as_ref()
                .context("System4 numbered audio assets absent")?
                .path(
                    if kind == Kind::Music {
                        Class::Music
                    } else {
                        Class::Sound
                    },
                    *number,
                )?;
            let Some(path) = path else {
                self.retire(key, host)?;
                machine.complete_hll(call, HllReturn::Scalar(0))?;
                return Ok(Some(Dispatch::Completed));
            };
            let ticket = host.begin_audio_resource(
                owner,
                AudioAsset {
                    path: VfsPath::new(Mount::Asset, &path)?,
                    frame_limit: None,
                },
                true,
            )?;
            self.pending = Some(Pending::Prepare {
                call: call.clone(),
                key,
                data_number: *number,
                ticket: Some(ticket),
                resource: None,
            });
            return Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))));
        }
        let Some(&number) = values.first() else {
            anyhow::bail!("System4 native audio channel argument")
        };
        let key = Key { kind, number };
        if kind == Kind::Music
            && self.metadata.reviewed_music.is_some()
            && matches!(
                name,
                b"SetLoopCount" | b"SetLoopStartPos" | b"SetLoopEndPos"
            )
        {
            // These exact interfaces are absent, but the native setters first
            // declare a resource via CA90(create=true). A prepared canonical
            // resource already satisfies that side effect; missing declarations
            // need a real empty-instance capability and cannot be faked by0.
            ensure!(self.channels.contains_key(&key),"System4 native optional music setter requires an unimplemented empty resource declaration");
            let count = self
                .reviewed_absent_calls
                .checked_add(1)
                .context("System4 reviewed music call counter")?;
            machine.complete_hll(call, HllReturn::Scalar(0))?;
            self.reviewed_absent_calls = count;
            return Ok(Some(Dispatch::Completed));
        }
        if name == b"Unprepare" && values.len() == 1 {
            let existed = if number < 0 {
                false
            } else {
                self.retire(key, host)?
            };
            machine.complete_hll(call, HllReturn::Scalar(i32::from(existed)))?;
            return Ok(Some(Dispatch::Completed));
        }
        if number < 0 || !self.channels.contains_key(&key) {
            ensure!(
                matches!(
                    name,
                    b"Play"
                        | b"Stop"
                        | b"IsPlay"
                        | b"SetLoopCount"
                        | b"GetLoopCount"
                        | b"SetLoopStartPos"
                        | b"SetLoopEndPos"
                        | b"Fade"
                        | b"StopFade"
                        | b"IsFade"
                        | b"Pause"
                        | b"Restart"
                        | b"IsPause"
                        | b"GetPos"
                        | b"GetLength"
                        | b"GetSamplePos"
                        | b"GetSampleLength"
                        | b"ReverseLR"
                        | b"GetVolume"
                        | b"GetTimeLength"
                        | b"GetGroupNum"
                ),
                "unsupported original SACT audio export {}",
                String::from_utf8_lossy(name)
            );
            machine.complete_hll(call, HllReturn::Scalar(0))?;
            return Ok(Some(Dispatch::Completed));
        }
        let info = self.observe(key, host)?;
        let channel = self.channels.get_mut(&key).unwrap();
        let returned = match (name, values.as_slice()) {
            (b"Play", [_]) => {
                if channel.playing {
                    if key.kind == Kind::Music && self.metadata.reviewed_music.is_some() {
                        0
                    } else {
                        1
                    }
                } else {
                    return Ok(Some(self.transport(key, true, call, host)?));
                }
            }
            (b"Stop", [_]) => {
                return Ok(Some(self.transport(key, false, call, host)?));
            }
            (b"IsPlay", [_]) => i32::from(channel.playing),
            (b"GetLoopCount", [_]) => channel.count as i32,
            (b"SetLoopCount", [_, count]) => {
                {
                    host.submit_audio(
                        owner,
                        AudioRequest::SetLoopSchedule {
                            channel: key.channel()?,
                            start: channel.loop_start,
                            end: Some(channel.loop_end),
                            passes: if *count == 0 {
                                None
                            } else {
                                Some(*count as u32)
                            },
                        },
                    )?;
                }
                channel.count = *count as u32;
                1
            }
            (b"SetLoopStartPos", [_, start]) => {
                let start = *start as u32 as u64;
                {
                    host.submit_audio(
                        owner,
                        AudioRequest::SetLoopSchedule {
                            channel: key.channel()?,
                            start,
                            end: Some(channel.loop_end),
                            passes: if channel.count == 0 {
                                None
                            } else {
                                Some(channel.count)
                            },
                        },
                    )?;
                }
                channel.loop_start = start;
                1
            }
            (b"SetLoopEndPos", [_, end]) => {
                let end = *end as u32 as u64;
                {
                    host.submit_audio(
                        owner,
                        AudioRequest::SetLoopSchedule {
                            channel: key.channel()?,
                            start: channel.loop_start,
                            end: Some(end),
                            passes: if channel.count == 0 {
                                None
                            } else {
                                Some(channel.count)
                            },
                        },
                    )?;
                }
                channel.loop_end = end;
                1
            }
            (b"ReverseLR", [_]) => {
                let swapped = !channel.swapped;
                {
                    host.submit_audio(
                        owner,
                        AudioRequest::SetChannelSwap {
                            channel: key.channel()?,
                            swapped,
                        },
                    )?;
                }
                channel.swapped = swapped;
                1
            }
            (b"GetVolume", [_]) => i32::from(channel.volume),
            (b"GetPos", [_]) => {
                ((u128::from(channel.position) * 1000) / u128::from(channel.info.source_rate))
                    as i32
            }
            (b"GetLength" | b"GetTimeLength", [_]) => {
                ((u128::from(channel.info.source_frames.unwrap()) * 1000)
                    / u128::from(channel.info.source_rate)) as i32
            }
            (b"GetSamplePos", [_]) => channel.position as i32,
            (b"GetSampleLength", [_]) => channel.info.source_frames.unwrap() as i32,
            (b"GetGroupNum", [_]) => channel.group as i32,
            (b"Fade", [_, milliseconds, volume, stop]) => {
                ensure!(
                    *milliseconds >= 0,
                    "System4 negative native audio fade duration unsupported"
                );
                let volume = (*volume).clamp(0, 100) as u8;
                if *milliseconds == 0 && *stop != 0 {
                    return Ok(Some(self.transport(key, false, call, host)?));
                }
                host.submit_audio(
                    owner,
                    AudioRequest::FadeGain {
                        channel: key.channel()?,
                        gain: f32::from(volume) / 100.,
                        fade_ms: *milliseconds as f32,
                        stop_after: *stop != 0,
                    },
                )?;
                if *milliseconds == 0 {
                    channel.volume = volume;
                }
                1
            }
            (b"StopFade", [_]) => {
                if info.is_some_and(|i| i.gain_fade_frames > 0) {
                    host.submit_audio(
                        owner,
                        AudioRequest::FinishGainFade {
                            channel: key.channel()?,
                        },
                    )?;
                }
                1
            }
            (b"IsFade", [_]) => i32::from(info.is_some_and(|i| i.gain_fade_frames > 0)),
            // Unknown ABI/services fail without acknowledging the native call.
            _ => anyhow::bail!(
                "unsupported original SACT audio export/ABI {}",
                String::from_utf8_lossy(name)
            ),
        };
        machine.complete_hll(call, HllReturn::Scalar(returned))?;
        Ok(Some(Dispatch::Completed))
    }
}
fn cp932(text: &str) -> Result<Vec<u8>> {
    let (bytes, _, errors) = encoding_rs::SHIFT_JIS.encode(text);
    ensure!(!errors, "System4 native audio name cannot be encoded");
    Ok(bytes.into_owned())
}

#[cfg(all(test, feature = "runtime"))]
pub(crate) mod tests {
    use super::*;
    use crate::{
        ain::{Function, HllFunction, Library, Program},
        sact::Sact,
        vm::Step,
    };
    use crysirror_core::service::MemoryAssetReader;
    use crysirror_formats_arc::ald_catalog::{AldCatalog, AldVolume};
    use crysirror_resource::ArchiveMount;
    use crysirror_runtime::Runtime;
    use std::time::{Duration, Instant};
    pub(crate) fn setup() -> (tempfile::TempDir, Runtime, Bridge, NativeAudio) {
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir(t.path().join("assets")).unwrap();
        let p=serde_json::from_value(serde_json::json!({"schema":2,"name":"native SACT audio","content_revision":"1","canvas":[1,1],"assets":"assets","clients":[{"name":"one","adapter":"crysirror-script-v1","game":"test"}]})).unwrap();
        let mut r = Runtime::with_asset_root(
            p,
            &t.path().join("assets"),
            &t.path().join("state"),
            Default::default(),
        )
        .unwrap();
        r.set_audio_nonblocking(true);
        let mut payload = b"RIFF".to_vec();
        payload.extend(2084u32.to_le_bytes());
        payload.extend(b"WAVEfmt ");
        payload.extend(16u32.to_le_bytes());
        payload.extend(1u16.to_le_bytes());
        payload.extend(2u16.to_le_bytes());
        payload.extend(48000u32.to_le_bytes());
        payload.extend(192000u32.to_le_bytes());
        payload.extend(4u16.to_le_bytes());
        payload.extend(16u16.to_le_bytes());
        payload.extend(b"data");
        payload.extend(2048u32.to_le_bytes());
        for _ in 0..512 {
            payload.extend(8192i16.to_le_bytes());
            payload.extend(16384i16.to_le_bytes());
        }
        let mut b = vec![0; 3072];
        for (i, offset) in [256u32, 512, 3072].into_iter().enumerate() {
            b[i * 3..i * 3 + 3].copy_from_slice(&[
                (offset >> 8) as u8,
                (offset >> 16) as u8,
                (offset >> 24) as u8,
            ]);
        }
        b[256..259].copy_from_slice(&[1, 1, 0]);
        b[512..516].copy_from_slice(&32u32.to_le_bytes());
        b[516..520].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        b[528..536].copy_from_slice(b"tone.wav");
        b[544..544 + payload.len()].copy_from_slice(&payload);
        std::fs::write(t.path().join("assets/gameWA.ald"), &b).unwrap();
        let cx = DecodeContext::new(ClientId(1));
        let catalog = AldCatalog::load(
            vec![AldVolume {
                number: 0,
                source: Arc::new(MemoryAssetReader(b.into())),
            }],
            &cx,
        )
        .unwrap();
        r.mount_provider(
            ClientId(1),
            "__sound",
            Arc::new(ArchiveMount::new(Box::new(catalog), cx).unwrap()),
        )
        .unwrap();
        let storage = r.host.storage(ClientId(1)).unwrap();
        let assets = crate::assets::Assets::load(
            ClientId(1),
            storage.as_ref(),
            &[crate::assets::Volume {
                class: Class::Sound,
                volume: 0,
                source: "gameWA.ald".into(),
                mount: "__sound".into(),
            }],
            Default::default(),
        )
        .unwrap();
        let mut bridge = Bridge::default();
        let mut sact = Sact::new([1, 1], ["g".into(), "m".into()]).unwrap();
        sact.initialized = true;
        bridge.sact = Some(sact);
        bridge.assets = Some(Arc::new(assets));
        let config = Bootstrap {
            program: "game.ain".into(),
            game_name: "test".into(),
            save_folder: "SaveData".into(),
            native_vm: None,
            canvas: [1, 1],
            joypad: false,
            mixers: vec![
                crate::bootstrap::Mixer {
                    name: "Music".into(),
                    volume: 100,
                },
                crate::bootstrap::Mixer {
                    name: "Sound".into(),
                    volume: 100,
                },
            ],
        };
        let pool = TaskPool::new("sact-audio-test", 1, 2).unwrap();
        let mut audio = NativeAudio::new(ClientId(1), pool);
        audio
            .configure(&config, 100, AudioMetadata::default())
            .unwrap();
        (t, r, bridge, audio)
    }
    pub(crate) fn pending(name: &str, args: &[i32]) -> (Machine, ExternalCall) {
        let mut code = Vec::new();
        for v in args {
            code.extend(0u16.to_le_bytes());
            code.extend(v.to_le_bytes());
        }
        code.extend(0x5au16.to_le_bytes());
        code.extend(0i32.to_le_bytes());
        code.extend(0i32.to_le_bytes());
        code.extend(0x2fu16.to_le_bytes());
        let mut m = Machine::new(Arc::new(Program {
            version: 5,
            code,
            functions: vec![Function {
                name: b"main".to_vec(),
                address: 0,
                is_label: false,
                return_kind: 10,
                return_structure: -1,
                arguments: 0,
                variables: vec![],
                crc: 0,
            }],
            libraries: vec![Library {
                name: b"SACT2".to_vec(),
                functions: vec![HllFunction {
                    name: name.as_bytes().to_vec(),
                    return_kind: 10,
                    arguments: args.iter().map(|_| (vec![], 10)).collect(),
                }],
            }],
            ..Default::default()
        }))
        .unwrap();
        for _ in 0..100 {
            if let Step::External(call) = m.step().unwrap() {
                return (m, call);
            }
        }
        panic!("native audio call missing")
    }
    fn call(a: &mut NativeAudio, r: &mut Runtime, b: &Bridge, name: &str, args: &[i32]) -> i32 {
        let (mut m, c) = pending(name, args);
        let until = Instant::now() + Duration::from_secs(10);
        loop {
            match a.dispatch(ClientId(1), &mut m, &c, b, &mut r.host).unwrap() {
                Some(Dispatch::Completed) => break,
                Some(Dispatch::WaitUntil(_)) => {
                    assert_eq!(m.pc, c.pc);
                    assert_eq!(m.stack, args);
                    assert!(Instant::now() < until);
                    std::thread::yield_now()
                }
                _ => panic!("wrong audio dispatch"),
            }
        }
        m.stack[0]
    }
    fn mixer_pending(name: &str, raw: &[i32], kinds: &[i32]) -> (Machine, ExternalCall) {
        let (mut machine, call) = pending(name, raw);
        let program = Arc::make_mut(&mut machine.program);
        program.libraries[0].name = b"SystemService".to_vec();
        program.libraries[0].functions[0].arguments =
            kinds.iter().map(|kind| (vec![], *kind)).collect();
        (machine, call)
    }
    fn mixer_scalar(a: &mut NativeAudio, r: &mut Runtime, name: &str, args: &[i32]) -> i32 {
        let (mut m, c) = mixer_pending(name, args, &vec![10; args.len()]);
        assert!(matches!(
            a.dispatch_mixer(ClientId(1), &mut m, &c, &mut r.host)
                .unwrap(),
            Some(Dispatch::Completed)
        ));
        m.stack[0]
    }
    fn mixer_read(a: &mut NativeAudio, r: &mut Runtime, name: &str, index: i32) -> (i32, i32) {
        use crate::vm::heap::{Object, Page, PageKind};
        let (mut m, c) = mixer_pending(name, &[index, 1, 0], &[10, 18]);
        let page = m
            .heap
            .alloc(Object::Page(Some(Page {
                kind: PageKind::Global,
                values: vec![77],
            })))
            .unwrap();
        m.stack[1] = page;
        assert!(matches!(
            a.dispatch_mixer(ClientId(1), &mut m, &c, &mut r.host)
                .unwrap(),
            Some(Dispatch::Completed)
        ));
        (m.stack[0], m.heap.get(page, 0).unwrap())
    }
    #[test]
    fn original_system_mixer_entire_family_uses_shared_gain_mute_and_exact_native_refs() {
        let (_t, mut r, b, mut a) = setup();
        assert_eq!(mixer_scalar(&mut a, &mut r, "GetMixerNumof", &[]), 2);
        assert_eq!(
            mixer_read(&mut a, &mut r, "GetMixerDefaultVolume", 0),
            (1, 100)
        );
        assert_eq!(mixer_scalar(&mut a, &mut r, "SetMixerVolume", &[1, 50]), 1);
        assert_eq!(mixer_read(&mut a, &mut r, "GetMixerVolume", 1), (1, 50));
        assert_eq!(mixer_scalar(&mut a, &mut r, "SetMixerMute", &[1, -1]), 1);
        assert_eq!(mixer_read(&mut a, &mut r, "GetMixerMute", 1), (1, 1));
        assert_eq!(mixer_read(&mut a, &mut r, "GetMixerVolume", 1), (1, 50));
        assert_eq!(mixer_read(&mut a, &mut r, "GetMixerVolume", 2), (0, 77));
        assert_eq!(
            mixer_scalar(&mut a, &mut r, "SetMixerVolume", &[-1, 100]),
            0
        );
        assert_eq!(call(&mut a, &mut r, &b, "Sound_Prepare", &[7, 1]), 1);
        assert_eq!(call(&mut a, &mut r, &b, "Sound_Play", &[7]), 1);
        r.host.channel_audio.render(1).unwrap();
        assert!(r
            .host
            .channel_audio
            .take_output()
            .iter()
            .flatten()
            .all(|v| *v == 0.));
        assert_eq!(mixer_scalar(&mut a, &mut r, "SetMixerMute", &[1, 0]), 1);
        r.host.channel_audio.render(1).unwrap();
        for p in r.host.channel_audio.take_output() {
            assert_eq!(p, [0.125, 0.25]);
        }
        let (mut m, c) = mixer_pending("GetMixerName", &[1, 1], &[10, 20]);
        let slot = m
            .heap
            .alloc(crate::vm::heap::Object::String(Arc::new(b"old".to_vec())))
            .unwrap();
        m.stack[1] = slot;
        a.dispatch_mixer(ClientId(1), &mut m, &c, &mut r.host)
            .unwrap();
        assert_eq!(m.heap.string(slot).unwrap().as_slice(), b"Sound");
        let (mut m, c) = mixer_pending("SetMixerName", &[1, 1], &[10, 12]);
        let slot = m
            .heap
            .alloc(crate::vm::heap::Object::String(Arc::new(
                b"renamed\0unused".to_vec(),
            )))
            .unwrap();
        m.stack[1] = slot;
        a.dispatch_mixer(ClientId(1), &mut m, &c, &mut r.host)
            .unwrap();
        assert_eq!(a.groups[1].name, b"renamed");
        assert_eq!(a.groups[1].parent, Some(2)); // Renaming does not reparent.
        let (mut m, c) = mixer_pending("SetMixerVolume", &[1, 80], &[10, 10]);
        assert!(a
            .dispatch_mixer(ClientId(2), &mut m, &c, &mut r.host)
            .is_err());
        assert_eq!(m.pc, c.pc);
        assert_eq!(m.stack, vec![1, 80]);
        assert_eq!(mixer_read(&mut a, &mut r, "GetMixerVolume", 1), (1, 50));
        a.close(&mut r.host).unwrap();
    }
    #[test]
    fn actual_native_prepare_play_finite_loop_and_swap_own_shared_resources_and_pcm() {
        let (_t, mut r, b, mut a) = setup();
        assert_eq!(call(&mut a, &mut r, &b, "Sound_GetUnuseChannel", &[]), 0);
        assert_eq!(call(&mut a, &mut r, &b, "Sound_Prepare", &[7, 1]), 1);
        assert_eq!(
            a.channels[&Key {
                kind: Kind::Sound,
                number: 7
            }]
                .info
                .source_frames,
            Some(512)
        );
        assert_eq!(call(&mut a, &mut r, &b, "Sound_SetLoopCount", &[7, 3]), 1);
        assert_eq!(call(&mut a, &mut r, &b, "Sound_ReverseLR", &[7]), 1);
        assert_eq!(call(&mut a, &mut r, &b, "Sound_Play", &[7]), 1);
        r.host.channel_audio.render(10).unwrap();
        let out = r.host.channel_audio.take_output();
        assert_eq!(out.len(), 480);
        for p in out {
            assert!((p[0] - 0.5).abs() < 1e-6);
            assert!((p[1] - 0.25).abs() < 1e-6);
        }
        r.host.channel_audio.render(12).unwrap();
        assert_eq!(call(&mut a, &mut r, &b, "Sound_GetLoopCount", &[7]), 1);
        r.host.channel_audio.render(12).unwrap();
        assert_eq!(call(&mut a, &mut r, &b, "Sound_IsPlay", &[7]), 0);
        assert_eq!(call(&mut a, &mut r, &b, "Sound_GetLoopCount", &[7]), 1);
        let resource = a.channels[&Key {
            kind: Kind::Sound,
            number: 7,
        }]
            .resource;
        a.close(&mut r.host).unwrap();
        assert!(r.host.audio_resource_info(ClientId(1), resource).is_err());
        assert!(!a.has_state());
    }
    #[test]
    fn native_fade_to_nonzero_then_finish_has_real_common_gain_and_no_fake_ack() {
        let (_t, mut r, b, mut a) = setup();
        call(&mut a, &mut r, &b, "Sound_Prepare", &[2, 1]);
        call(&mut a, &mut r, &b, "Sound_Play", &[2]);
        call(&mut a, &mut r, &b, "Sound_Fade", &[2, 2, 50, 0]);
        r.host.channel_audio.render(1).unwrap();
        assert_eq!(call(&mut a, &mut r, &b, "Sound_GetVolume", &[2]), 75);
        assert_eq!(call(&mut a, &mut r, &b, "Sound_IsFade", &[2]), 1);
        call(&mut a, &mut r, &b, "Sound_StopFade", &[2]);
        assert_eq!(call(&mut a, &mut r, &b, "Sound_IsFade", &[2]), 0);
        r.host.channel_audio.take_output();
        r.host.channel_audio.render(1).unwrap();
        for p in r.host.channel_audio.take_output() {
            assert!((p[0] - 0.125).abs() < 1e-6);
        }
        let (mut m, c) = pending("Sound_AuthoredMissing", &[2]);
        assert!(a
            .dispatch(ClientId(1), &mut m, &c, &b, &mut r.host)
            .is_err());
        assert_eq!(m.pc, c.pc);
        assert_eq!(m.stack, [2]);
        assert_eq!(m.step().unwrap(), Step::External(c));
    }
    #[test]
    fn foreign_or_malformed_calls_and_missing_numbered_prepare_keep_explicit_boundaries() {
        let (_t, mut r, b, mut a) = setup();
        let (mut m, c) = pending("Sound_Play", &[2]);
        assert!(a
            .dispatch(ClientId(2), &mut m, &c, &b, &mut r.host)
            .is_err());
        assert_eq!(m.pc, c.pc);
        let (mut m, c) = pending("Sound_Play", &[2, 9]);
        assert!(a
            .dispatch(ClientId(1), &mut m, &c, &b, &mut r.host)
            .is_err());
        assert_eq!(m.stack, [2, 9]);
        assert_eq!(call(&mut a, &mut r, &b, "Sound_Prepare", &[2, 1]), 1);
        assert_eq!(call(&mut a, &mut r, &b, "Sound_Prepare", &[2, 999]), 0);
        assert!(a.channels.is_empty());
        let (mut m, c) = pending("Music_Seek", &[0, 100]);
        assert!(a
            .dispatch(ClientId(1), &mut m, &c, &b, &mut r.host)
            .is_err());
        assert_eq!(m.pc, c.pc);
    }
    #[test]
    fn reviewed_optional_music_family_requires_actual_version_profile_and_does_not_claim_pause_support(
    ) {
        let (_t, mut r, b, mut a) = setup();
        let (mut m, c) = pending("Music_Pause", &[7]);
        assert!(a
            .dispatch_reviewed_unavailable(ClientId(1), &mut m, &c, &b, &mut r.host)
            .is_err());
        assert_eq!(m.pc, c.pc);
        assert_eq!(m.stack, vec![7]);
        a.metadata.reviewed_music =
            Some(crate::native_audio_profile::ReviewedMusic::Rance7Original);
        for name in [
            "Music_Pause",
            "Music_Restart",
            "Music_IsPause",
            "Music_GetLoopCount",
            "Music_GetSamplePos",
            "Music_GetSampleLength",
        ] {
            let (mut m, c) = pending(name, &[7]);
            assert_eq!(
                a.dispatch_reviewed_unavailable(ClientId(1), &mut m, &c, &b, &mut r.host)
                    .unwrap(),
                Some(Dispatch::Completed)
            );
            assert_eq!(m.stack, vec![0]);
        }
        let (mut seek, seek_call) = pending("Music_Seek", &[7, 123]);
        assert_eq!(
            a.dispatch_reviewed_unavailable(ClientId(1), &mut seek, &seek_call, &b, &mut r.host)
                .unwrap(),
            Some(Dispatch::Completed)
        );
        assert_eq!(seek.stack, [0]);
        assert_eq!(a.reviewed_absent_calls, 7);
        assert!(a.channels.is_empty());
        let (mut m, c) = pending("Music_Pause", &[7]);
        assert!(a
            .dispatch_reviewed_unavailable(ClientId(2), &mut m, &c, &b, &mut r.host)
            .is_err());
        assert_eq!(m.pc, c.pc);
        let (mut m, c) = pending("Music_Pause", &[]);
        assert!(a
            .dispatch_reviewed_unavailable(ClientId(1), &mut m, &c, &b, &mut r.host)
            .is_err());
        assert_eq!(m.pc, c.pc);
        assert_eq!(a.reviewed_absent_calls, 7);
    }
    #[test]
    fn native_prepare_stop_replay_keep_one_real_shared_instance_and_fader_clock() {
        let (_t, mut r, b, mut a) = setup();
        call(&mut a, &mut r, &b, "Sound_Prepare", &[7, 1]);
        let key = Key {
            kind: Kind::Sound,
            number: 7,
        };
        let initial = r
            .host
            .audio_info(ClientId(1), key.channel().unwrap())
            .unwrap()
            .unwrap();
        assert!(initial.retain_on_finish && !initial.transport_active && !initial.paused);
        r.host.channel_audio.render(1).unwrap();
        assert!(r
            .host
            .channel_audio
            .take_output()
            .iter()
            .all(|p| *p == [0.; 2]));
        call(&mut a, &mut r, &b, "Sound_Play", &[7]);
        r.host.channel_audio.render(2).unwrap();
        r.host.channel_audio.take_output();
        let before = r
            .host
            .audio_info(ClientId(1), key.channel().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(before.position_frames, 96);
        call(&mut a, &mut r, &b, "Sound_Stop", &[7]);
        let stopped = r
            .host
            .audio_info(ClientId(1), key.channel().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(stopped.age_frames, before.age_frames);
        assert_eq!(stopped.position_frames, 0);
        assert!(!stopped.transport_active);
        call(&mut a, &mut r, &b, "Sound_Fade", &[7, 2, 50, 0]);
        r.host.channel_audio.render(2).unwrap();
        assert!(r
            .host
            .channel_audio
            .take_output()
            .iter()
            .all(|p| *p == [0.; 2]));
        assert_eq!(call(&mut a, &mut r, &b, "Sound_GetVolume", &[7]), 50);
        call(&mut a, &mut r, &b, "Sound_Play", &[7]);
        r.host.channel_audio.render(1).unwrap();
        for p in r.host.channel_audio.take_output() {
            assert!((p[0] - 0.125).abs() < 1e-6);
            assert!((p[1] - 0.25).abs() < 1e-6);
        }
        let replay = r
            .host
            .audio_info(ClientId(1), key.channel().unwrap())
            .unwrap()
            .unwrap();
        assert!(replay.age_frames > before.age_frames);
        assert_eq!(replay.position_frames, 48);
    }
    #[test]
    fn native_eof_keeps_actual_terminal_state_and_reuses_source_after_a_completed_seek() {
        let (_t, mut r, b, mut a) = setup();
        call(&mut a, &mut r, &b, "Sound_Prepare", &[3, 1]);
        call(&mut a, &mut r, &b, "Sound_Play", &[3]);
        r.host.channel_audio.render(20).unwrap();
        r.host.channel_audio.take_output();
        assert_eq!(call(&mut a, &mut r, &b, "Sound_IsPlay", &[3]), 0);
        let key = Key {
            kind: Kind::Sound,
            number: 3,
        };
        let ended = r
            .host
            .audio_info(ClientId(1), key.channel().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(ended.position_frames, 512);
        assert!(!ended.transport_active);
        assert_eq!(a.channels[&key].position, 512);
        call(&mut a, &mut r, &b, "Sound_Fade", &[3, 2, 50, 0]);
        r.host.channel_audio.render(2).unwrap();
        r.host.channel_audio.take_output();
        assert_eq!(call(&mut a, &mut r, &b, "Sound_GetVolume", &[3]), 50);
        call(&mut a, &mut r, &b, "Sound_Play", &[3]);
        r.host.channel_audio.render(1).unwrap();
        assert!(r
            .host
            .channel_audio
            .take_output()
            .iter()
            .all(|p| (p[0] - 0.125).abs() < 1e-6));
        assert!(
            r.host
                .audio_info(ClientId(1), key.channel().unwrap())
                .unwrap()
                .unwrap()
                .age_frames
                > ended.age_frames
        );
        a.close(&mut r.host).unwrap();
        assert!(r
            .host
            .audio_info(ClientId(1), key.channel().unwrap())
            .unwrap()
            .is_none());
    }
    #[test]
    fn native_prepare_from_file_uses_shared_case_worker_and_keeps_original_string_until_completion()
    {
        let (t, mut r, b, mut a) = setup();
        let path = b
            .assets
            .as_ref()
            .unwrap()
            .path(Class::Sound, 1)
            .unwrap()
            .unwrap();
        let bytes = r
            .host
            .storage(ClientId(1))
            .unwrap()
            .read(&VfsPath::new(Mount::Asset, &path).unwrap())
            .unwrap();
        std::fs::write(t.path().join("assets/Tone.wav"), bytes).unwrap();
        let (mut m, c) = pending("Sound_PrepareFromFile", &[9, 0]);
        let slot = m
            .heap
            .alloc(crate::vm::heap::Object::String(Arc::new(
                b"TONE.WAV".to_vec(),
            )))
            .unwrap();
        m.stack[1] = slot;
        Arc::make_mut(&mut m.program).libraries[0].functions[0].arguments =
            vec![(vec![], 10), (vec![], 12)];
        let until = Instant::now() + Duration::from_secs(10);
        let mut waits = 0;
        loop {
            match a
                .dispatch(ClientId(1), &mut m, &c, &b, &mut r.host)
                .unwrap()
            {
                Some(Dispatch::Completed) => break,
                Some(Dispatch::WaitUntil(_)) => {
                    waits += 1;
                    assert_eq!(m.pc, c.pc);
                    assert_eq!(m.stack, [9, slot]);
                    assert_eq!(m.heap.string(slot).unwrap().as_slice(), b"TONE.WAV");
                    assert!(Instant::now() < until);
                    std::thread::yield_now();
                }
                x => panic!("{x:?}"),
            }
        }
        assert!(waits > 0);
        assert_eq!(m.stack, [1]);
        let key = Key {
            kind: Kind::Sound,
            number: 9,
        };
        assert_eq!(a.channels[&key].data_number, -1);
        assert_eq!(a.channels[&key].group, 0);
        assert_eq!(a.channels[&key].info.source_frames, Some(512));
        assert!(
            !r.host
                .audio_info(ClientId(1), key.channel().unwrap())
                .unwrap()
                .unwrap()
                .transport_active
        );
        for _ in 0..100 {
            if m.step().unwrap() == Step::Halted {
                break;
            }
        }
        assert!(m.heap.slot(slot).is_err());
    }
    struct FailingClose<'a> {
        inner: &'a mut dyn HostServices,
        fail: bool,
        voice_releases: usize,
        stops: usize,
        resource_releases: usize,
        resource_tickets: usize,
    }
    impl crysirror_core::service::FileSystem for FailingClose<'_> {
        fn read(&mut self, c: ClientId, p: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
            self.inner.read(c, p)
        }
        fn write_atomic(
            &mut self,
            c: ClientId,
            p: &VfsPath,
            b: &[u8],
        ) -> crysirror_core::Result<()> {
            self.inner.write_atomic(c, p, b)
        }
    }
    impl HostServices for FailingClose<'_> {
        fn submit(
            &mut self,
            c: ClientId,
            b: &[crysirror_core::state::Command],
        ) -> crysirror_core::Result<()> {
            self.inner.submit(c, b)
        }
        fn now_ms(&self) -> u64 {
            self.inner.now_ms()
        }
        fn submit_audio(&mut self, c: ClientId, r: AudioRequest) -> crysirror_core::Result<()> {
            if matches!(r, AudioRequest::Stop { .. }) {
                self.stops += 1;
            }
            self.inner.submit_audio(c, r)
        }
        fn release_audio(&mut self, c: ClientId, id: u64) -> crysirror_core::Result<()> {
            self.voice_releases += 1;
            self.inner.release_audio(c, id)
        }
        fn release_audio_resource(
            &mut self,
            c: ClientId,
            id: AudioResourceHandle,
        ) -> crysirror_core::Result<()> {
            self.resource_releases += 1;
            if self.fail {
                self.fail = false;
                return Err(crysirror_core::Error::Io(
                    "authored transient release failure".into(),
                ));
            }
            self.inner.release_audio_resource(c, id)
        }
        fn release_audio_resource_preparation(
            &mut self,
            c: ClientId,
            id: u64,
        ) -> crysirror_core::Result<()> {
            self.resource_tickets += 1;
            self.inner.release_audio_resource_preparation(c, id)
        }
    }
    #[test]
    fn closing_a_pending_canonical_prepare_retains_authority_after_partial_cleanup_failure() {
        let (_t, mut r, b, mut a) = setup();
        let (mut m, c) = pending("Sound_Prepare", &[7, 1]);
        let until = Instant::now() + Duration::from_secs(10);
        while !matches!(a.pending, Some(Pending::Warm { .. })) {
            assert!(matches!(
                a.dispatch(ClientId(1), &mut m, &c, &b, &mut r.host)
                    .unwrap(),
                Some(Dispatch::WaitUntil(_))
            ));
            assert!(Instant::now() < until);
            std::thread::yield_now();
        }
        let resource = match a.pending.as_ref().unwrap() {
            Pending::Warm {
                resource: Some(r), ..
            } => *r,
            _ => panic!(),
        };
        let mut host = FailingClose {
            inner: &mut r.host,
            fail: true,
            voice_releases: 0,
            stops: 0,
            resource_releases: 0,
            resource_tickets: 0,
        };
        assert!(a.close_commands(&mut host).is_err());
        assert!(a.pending.is_some());
        a.close_commands(&mut host).unwrap();
        assert_eq!(host.voice_releases, 1);
        assert_eq!(host.stops, 1);
        assert_eq!(host.resource_releases, 2);
        assert_eq!(host.resource_tickets, 1);
        assert!(a.pending.is_none());
        assert!(r.host.audio_resource_info(ClientId(1), resource).is_err());
        assert_eq!(m.pc, c.pc);
        assert_eq!(m.stack, [7, 1]);
    }
}

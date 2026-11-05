use std::collections::BTreeMap;
use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle, StaticSoundSettings};
use kira::sound::{EndPosition, PlaybackPosition, PlaybackState, Region};
use kira::{AudioManager, AudioManagerSettings, Frame, Tween};
#[cfg(not(feature = "host-audio"))]
use kira::DefaultBackend;
#[cfg(feature = "host-audio")]
pub mod host_backend;
#[cfg(feature = "host-audio")]
use host_backend::HostBackend as DefaultBackend;
use crate::native_asset::{LoadedAsset, ResourceManager};
use wmv_decoder::AsfWmaDecoder;

mod movie;
mod decode;
#[cfg(not(target_arch = "wasm32"))]
pub mod stream;
mod native_sound;
mod source_format;
pub use source_format::{PcmSourceFormat, PcmSourceKind};
use native_sound::{NativeSoundData, NativeSoundHandle};
pub use movie::{decode_movie_audio, movie_loop_region, movie_loop_samples, resize_movie_audio};

#[derive(Clone, Debug)]
pub struct AudioConfig {
    pub enabled: bool,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

pub struct AudioSystem {
    enabled: bool,
    #[cfg(feature = "host-audio")]
    sink: host_backend::AudioSink,
    manager: Option<AudioManager<DefaultBackend>>,
    groups: BTreeMap<PalSoundGroup, Vec<AudioSlot>>,
    primary_volume: PalVolume,
    pal_time_ms: u32,
    fade_clock_ms: Arc<AtomicU32>,
    group_volumes: BTreeMap<PalSoundGroup, PalVolume>,
}

impl AudioSystem {
    pub fn new(config: AudioConfig) -> anyhow::Result<Self> {
        #[cfg(feature = "host-audio")]
        let sink = host_backend::AudioSink::default();
        #[allow(unused_mut)]
        let mut settings = AudioManagerSettings::<DefaultBackend>::default();
        #[cfg(feature = "host-audio")]
        {
            settings.backend_settings = sink.clone();
        }
        let manager = if config.enabled {
            match AudioManager::<DefaultBackend>::new(settings) {
                Ok(manager) => Some(manager),
                Err(error) => {
                    log::warn!("audio backend initialization failed; audio disabled: {error:?}");
                    None
                }
            }
        } else {
            None
        };
        let mut groups = BTreeMap::new();
        let mut group_volumes = BTreeMap::new();
        for group in PalSoundGroup::ALL {
            groups.insert(
                group,
                (0..group.slot_count())
                    .map(|_| AudioSlot {
                        // Native worker initialization creates its two stream
                        // buffers in each bank with stable command 5.
                        worker_command: if matches!(group.0, 0 | 3) { 5 } else { 0 },
                        ..AudioSlot::default()
                    })
                    .collect(),
            );
            group_volumes.insert(group, PalVolume::MAX);
        }
        Ok(Self {
            enabled: manager.is_some(),
            #[cfg(feature = "host-audio")]
            sink,
            manager,
            groups,
            primary_volume: PalVolume::MAX,
            pal_time_ms: 0,
            fade_clock_ms: Arc::new(AtomicU32::new(0)),
            group_volumes,
        })
    }

    #[cfg(feature = "host-audio")]
    pub fn host_sink(&self) -> host_backend::AudioSink {
        self.sink.clone()
    }

    pub fn set_pal_time(&mut self, milliseconds: u32) {
        self.pal_time_ms = milliseconds;
        self.fade_clock_ms.store(milliseconds, Ordering::Release);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Submit a movie soundtrack outside PAL's ordinary sound groups. The
    /// caller owns its handle and synchronizes seek/stop with the movie clock.
    pub fn play_movie_sound(&mut self, data: StaticSoundData) -> anyhow::Result<StaticSoundHandle> {
        let manager = self.manager.as_mut()
            .ok_or_else(|| anyhow::anyhow!("movie audio backend is disabled"))?;
        manager.play(data).map_err(|error| anyhow::anyhow!("movie audio playback failed: {error}"))
    }

    pub fn load_static_from_resource(
        &mut self,
        resource_manager: &mut ResourceManager,
        name: &str,
        group: PalSoundGroup,
    ) -> anyhow::Result<AudioHandle> {
        let asset = resource_manager.open(name)?;
        self.load_static_asset(asset, group)
    }

    pub fn load_static_asset(
        &mut self,
        asset: LoadedAsset,
        group: PalSoundGroup,
    ) -> anyhow::Result<AudioHandle> {
        self.load_static_asset_range(asset, group, None)
    }

    pub fn load_static_asset_range(
        &mut self, asset: LoadedAsset, group: PalSoundGroup,
        range: Option<crate::native_vm::abi::PalSoundLoadRange32>,
    ) -> anyhow::Result<AudioHandle> {
        let (data, format) = decode_asset_sound(&asset.bytes, group)?;
        let handle = self.load_sound_data(asset.name, data, group)?;
        self.set_source_format(handle, format)?;
        if let Some(range) = range.filter(|range| range.enabled != 0 && group == PalSoundGroup::GROUP3) {
            if let Err(error) = self.set_load_range(handle, range) {
                self.release(handle)?;
                return Err(error);
            }
        }
        Ok(handle)
    }

    /// Keep PAL's group-3 Ogg stream encoded until the decoder worker needs PCM.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_streaming_ogg(&mut self, name: impl Into<String>, bytes: &[u8],
        group: PalSoundGroup) -> anyhow::Result<AudioHandle> {
        anyhow::ensure!(group == PalSoundGroup::GROUP3, "Ogg streaming requires PAL group 3");
        let stream = stream::StreamAsset::new(bytes)?;
        let format = stream.source_format;
        let handle = self.load_sound_data(name, LoadedSound::Stream(stream), group)?;
        self.set_source_format(handle, format)?;
        Ok(handle)
    }

    pub fn set_source_format(&mut self, handle: AudioHandle, format: PcmSourceFormat) -> anyhow::Result<()> {
        let slot = self.slot_mut(handle)?;
        ensure_loaded(slot, handle)?;
        slot.source_format = Some(format);
        if slot.source_group == PalSoundGroup::GROUP3 && !slot.loop_bytes_explicit {
            slot.loop_end_bytes = format.decoded_bytes as i32;
        }
        Ok(())
    }

    pub fn source_format(&self, handle: AudioHandle) -> anyhow::Result<Option<PcmSourceFormat>> {
        Ok(self.slot(handle)?.source_format)
    }

    /// Return the reviewed native buffer format, not the mixer's stereo f32
    /// layout. PAL preallocates both stream banks as 44.1 kHz stereo PCM16.
    pub fn sound_buffer_format(&self, handle: AudioHandle) -> anyhow::Result<crate::native_vm::abi::PalWaveFormatEx32> {
        let slot = self.slot(handle)?;
        if matches!(handle.group.0, 0 | 3) || (slot.data.is_some() && matches!(slot.source_group.0, 0 | 3)) {
            return Ok(crate::native_vm::abi::PalWaveFormatEx32 {
                format_tag: 1, channels: 2, samples_per_second: 44_100,
                average_bytes_per_second: 176_400, block_align: 4,
                bits_per_sample: 16, extra_size: 0,
            });
        }
        ensure_loaded(slot, handle)?;
        let format = slot.source_format.filter(|format| format.native_pal_format()
            && format.format_tag == 1 && format.extra_size == 0)
            .ok_or_else(|| anyhow::anyhow!("native buffer format unavailable for this source"))?;
        Ok(crate::native_vm::abi::PalWaveFormatEx32 {
            format_tag: format.format_tag, channels: format.channels,
            samples_per_second: format.sample_rate,
            average_bytes_per_second: format.average_bytes_per_second,
            block_align: format.block_align, bits_per_sample: format.bits_per_sample,
            extra_size: format.extra_size,
        })
    }

    /// Reserve an unloaded group-3 entry without opening a resource. The first
    /// native word is unused, and the reservation does not retain a filename.
    pub fn load_set_reserved(&mut self, _unused_word: i32, group: i32) -> u32 {
        if group != 3 { return 0; }
        let Ok(index) = self.find_free_slot(PalSoundGroup::GROUP3) else { return 0; };
        let handle = AudioHandle::new(PalSoundGroup::GROUP3, index);
        self.slot_mut(handle).unwrap().reserved_word = 1;
        handle.raw()
    }

    /// Apply the reviewed side effect of PalSoundSetReserved. Its incidental
    /// native EAX value is deliberately not presented as a result contract.
    pub fn set_reserved(&mut self, group: i32, raw_handle: u32, reserved: i32) {
        if group != 3 || !(0x2000_0000..=0x2000_0001).contains(&raw_handle) { return; }
        let handle = AudioHandle::new(PalSoundGroup::GROUP3, (raw_handle & 1) as usize);
        self.slot_mut(handle).unwrap().reserved_word = reserved;
    }

    /// Apply the native loader's group-3 byte-range descriptor. Other groups
    /// ignore the pointer, as in the original WAV and Ogg loaders.
    pub fn set_load_range(&mut self, handle: AudioHandle, range: crate::native_vm::abi::PalSoundLoadRange32) -> anyhow::Result<()> {
        let slot = self.slot_mut(handle)?;
        ensure_loaded(slot, handle)?;
        if range.enabled == 0 || slot.source_group != PalSoundGroup::GROUP3 { return Ok(()); }
        let format = slot.source_format.ok_or_else(|| anyhow::anyhow!("PCM source format is unavailable"))?;
        format.frame_at_byte(range.initial_start_bytes as i32)?;
        slot.initial_start_bytes = range.initial_start_bytes as i32;
        let end = if range.loop_end_bytes == 0 { format.decoded_bytes } else { range.loop_end_bytes };
        self.set_start_end(handle, range.loop_start_bytes as i32, end as i32)
    }

    pub fn load_static_data(
        &mut self,
        name: impl Into<String>,
        data: StaticSoundData,
        group: PalSoundGroup,
    ) -> anyhow::Result<AudioHandle> {
        self.load_sound_data(name, LoadedSound::Static(data), group)
    }

    fn load_sound_data(&mut self, name: impl Into<String>, data: LoadedSound,
        group: PalSoundGroup) -> anyhow::Result<AudioHandle> {
        let slot_index = self.find_free_slot(group)?;
        let slot = self.slot_mut(AudioHandle::new(group, slot_index))?;
        slot.name = Some(name.into());
        slot.data = Some(data);
        slot.handle = None;
        slot.loop_mode = 0;
        slot.volume = PalVolume::MAX;
        slot.initial_start_bytes = 0;
        slot.loop_start_bytes = 0;
        slot.loop_end_bytes = 0;
        slot.loop_bytes_explicit = false;
        slot.source_group = group;
        slot.source_format = None;
        slot.pause_word = 0;
        slot.reserved_word = 0;
        slot.worker_command = 0;
        slot.fade_start_ms = 0;
        slot.fade_duration_ms = 0;
        slot.copied_resource = false;
        slot.loop_start_samples = 0;
        slot.loop_end_samples = 0;
        slot.pan = 0;
        slot.frequency = 0;
        slot.volume_ratio = 0.0;
        Ok(AudioHandle::new(group, slot_index))
    }

    /// PalSoundLoadEx (newer DLL 0x100B3060) amplifies and clips signed
    /// 16-bit PCM at load time, independently of subsequent channel gain.
    pub fn load_static_data_ex(
        &mut self,
        name: impl Into<String>,
        mut data: StaticSoundData,
        group: PalSoundGroup,
        gain: f32,
    ) -> anyhow::Result<AudioHandle> {
        anyhow::ensure!(group == PalSoundGroup::GROUP4, "PalSoundLoadEx requires group 4");
        if gain > 0.0 && gain != 1.0 {
            for frame in Arc::make_mut(&mut data.frames) {
                frame.left = amplify_pcm16(frame.left, gain);
                frame.right = amplify_pcm16(frame.right, gain);
            }
        }
        let handle = self.load_static_data(name, data, group)?;
        self.slot_mut(handle)?.volume_ratio = gain;
        Ok(handle)
    }

    pub fn copy_sound(
        &mut self,
        source: AudioHandle,
        target_group: PalSoundGroup,
    ) -> anyhow::Result<AudioHandle> {
        // PAL chooses a destination bank but memcpy preserves the source
        // record's scalar configuration. The duplicated buffer starts stopped.
        let (data, name, looping, volume, initial_start_bytes, loop_start_bytes, loop_end_bytes, loop_bytes_explicit, source_group, source_format, pause_word, reserved_word, worker_command, fade_start_ms, fade_duration_ms, loop_start_samples,
            loop_end_samples, pan, frequency, volume_ratio) = {
            let source_slot = self.slot(source)?;
            let data = source_slot.data.clone().ok_or_else(|| anyhow::anyhow!("audio handle {:?} has no loaded sound data", source))?;
            (data, source_slot.name.clone(), source_slot.loop_mode, source_slot.volume,
                source_slot.initial_start_bytes, source_slot.loop_start_bytes, source_slot.loop_end_bytes, source_slot.loop_bytes_explicit, source_slot.source_group, source_slot.source_format, source_slot.pause_word, source_slot.reserved_word, source_slot.native_worker_command(), source_slot.fade_start_ms, source_slot.fade_duration_ms, source_slot.loop_start_samples,
                source_slot.loop_end_samples, source_slot.pan, source_slot.frequency, source_slot.volume_ratio)
        };
        let slot_index = self.find_free_slot(target_group)?;
        let slot = self.slot_mut(AudioHandle::new(target_group, slot_index))?;
        slot.name = name;
        slot.data = Some(data);
        slot.handle = None;
        slot.loop_mode = looping;
        slot.volume = volume;
        slot.initial_start_bytes = initial_start_bytes;
        slot.loop_start_bytes = loop_start_bytes;
        slot.loop_end_bytes = loop_end_bytes;
        slot.loop_bytes_explicit = loop_bytes_explicit;
        slot.source_group = source_group;
        slot.source_format = source_format;
        slot.pause_word = pause_word;
        slot.reserved_word = reserved_word;
        slot.worker_command = worker_command;
        slot.fade_start_ms = fade_start_ms;
        slot.fade_duration_ms = fade_duration_ms;
        slot.copied_resource = true;
        slot.loop_start_samples = loop_start_samples;
        slot.loop_end_samples = loop_end_samples;
        slot.pan = pan;
        slot.frequency = frequency;
        slot.volume_ratio = volume_ratio;
        Ok(AudioHandle::new(target_group, slot_index))
    }

    pub fn play(&mut self, handle: AudioHandle, looping: bool) -> anyhow::Result<()> {
        let source_group = self.slot(handle)?.source_group;
        let duration = if source_group == PalSoundGroup::GROUP0 || source_group == PalSoundGroup::GROUP3 { 10 } else { 0 };
        let old_clock = self.slot(handle)?.fade_start_ms;
        let old_duration = self.slot(handle)?.fade_duration_ms;
        let old_command = self.slot(handle)?.native_worker_command();
        self.play_native(handle, i32::from(looping), duration)?;
        if duration == 0 {
            self.slot_mut(handle)?.fade_start_ms = old_clock;
            self.slot_mut(handle)?.fade_duration_ms = old_duration;
            let slot = self.slot_mut(handle)?;
            slot.worker_command = old_command;
            if let Some(sound) = &slot.handle { sound.set_worker_command(old_command); }
        }
        Ok(())
    }

    /// PAL's PlayFade starts at silence; its envelope is independent of the
    /// channel/group gain so voice volume and autopan updates preserve the fade.
    pub fn play_fade(&mut self, handle: AudioHandle, looping: bool, fade_ms: i32) -> anyhow::Result<()> {
        self.play_native(handle, i32::from(looping), fade_ms)
    }

    pub fn play_native(&mut self, handle: AudioHandle, loop_mode: i32, fade_ms: i32) -> anyhow::Result<()> {
        if !self.enabled {
            return Ok(());
        }
        let (data, start, end, region, pan, rate) = {
            let slot = self.slot(handle)?;
            let data = slot.data.clone().ok_or_else(||
                anyhow::anyhow!("audio handle {:?} has no loaded sound data", handle))?;
            let mut start = None;
            let mut end = None;
            if slot.source_group == PalSoundGroup::GROUP3 {
                if let Some(format) = slot.source_format {
                    let index = format.frame_at_byte(slot.initial_start_bytes)?;
                    anyhow::ensure!(index <= data.num_frames(), "native initial byte offset exceeds decoded audio");
                    start = Some(PlaybackPosition::Samples(index));
                    if slot.loop_bytes_explicit && slot.loop_end_bytes != 0 {
                        let limit = format.frame_at_byte(slot.loop_end_bytes)?;
                        anyhow::ensure!(limit <= data.num_frames() && limit > index,
                            "unsupported native byte range outside decoded audio");
                        end = Some((0, limit));
                    }
                }
            }
            let rate = if slot.frequency > 0 {
                f64::from(slot.frequency) / f64::from(data.sample_rate())
            } else { 1.0 };
            (data, start, end, loop_region_for(loop_mode != 0,
                slot.loop_start_samples, slot.loop_end_samples), pal_pan_settings(slot.pan).0, rate)
        };
        // A native DirectSound buffer is restarted, not duplicated, by Play.
        // Dropping a Kira handle alone leaves its sound alive on the mixer.
        if let Some(mut previous) = self.slot_mut(handle)?.handle.take() {
            previous.stop(immediate_audio_tween());
        }
        let effective = self.effective_volume(handle)?;
        let decibels = native_volume_decibels(effective).unwrap_or(-100.0)
            + pal_pan_settings(self.slot(handle)?.pan).1;
        log::debug!(
            "[trace-audio] play handle={handle:?} loop_mode={loop_mode} effective_raw={} db={decibels:.2}",
            effective.raw()
        );
        let manager = self
            .manager
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("audio backend is disabled"))?;
        // Set the destination gain before scheduling audio to avoid a first
        // buffer at full volume; NativeSound owns the amplitude envelope.
        let static_handle = match data {
            LoadedSound::Static(mut data) => {
                if let Some(start) = start { data = data.start_position(start); }
                if let Some(end) = end { data.slice = Some(end); }
                if let Some(region) = region { data = data.loop_region(region); }
                data = data.panning(pan).playback_rate(rate);
                let mut playback = prepare_playback(data, decibels, fade_ms);
                playback.clock_ms = Some(self.fade_clock_ms.clone());
                manager.play(playback)?
            }
            #[cfg(not(target_arch = "wasm32"))]
            LoadedSound::Stream(asset) => {
                let mut data = asset.sound()?.volume(decibels).panning(pan).playback_rate(rate);
                if let Some(start) = start { data = data.start_position(start); }
                if let Some(end) = end { data.slice = Some(end); }
                if let Some(region) = region { data = data.loop_region(region); }
                manager.play(NativeSoundData { data, fade_ms: fade_ms as u32,
                    clock_ms: Some(self.fade_clock_ms.clone()) })?
            }
        };
        let now = self.pal_time_ms;
        let slot = self.slot_mut(handle)?;
        slot.fade_start_ms = now;
        slot.fade_duration_ms = fade_ms;
        slot.handle = Some(static_handle);
        slot.loop_mode = loop_mode;
        slot.worker_command = 5;
        Ok(())
    }

    pub fn stop(&mut self, handle: AudioHandle) -> anyhow::Result<()> {
        let slot = self.slot_mut(handle)?;
        let started = slot.handle.is_some();
        if let Some(sound) = slot.handle.as_mut() {
            sound.stop(immediate_audio_tween());
        }
        slot.handle = None;
        if started { slot.worker_command = 5; }
        if started && slot.source_group == PalSoundGroup::GROUP3 {
            slot.data = None; slot.reserved_word = 0;
        }
        Ok(())
    }

    pub fn pause(&mut self, handle: AudioHandle) -> anyhow::Result<()> {
        self.pause_native(handle, 1, 0).map(|_| ())
    }

    pub fn stop_fade(&mut self, handle: AudioHandle, duration_ms: u32) -> anyhow::Result<()> {
        let now = self.pal_time_ms;
        let slot = self.slot_mut(handle)?;
        if let Some(sound) = slot.handle.as_mut() {
            if sound.fade_state() == 2 && duration_ms != 0 { return Ok(()); }
            slot.fade_start_ms = now;
            slot.fade_duration_ms = duration_ms as i32;
            sound.stop(Tween {
                duration: std::time::Duration::from_millis(u64::from(duration_ms)),
                ..Tween::default()
            });
        }
        Ok(())
    }

    pub fn pause_fade(&mut self, handle: AudioHandle, paused: bool, duration_ms: u32) -> anyhow::Result<()> {
        self.pause_native(handle, i32::from(paused), duration_ms as i32).map(|_| ())
    }

    pub fn pause_native(&mut self, handle: AudioHandle, pause: i32, duration_ms: i32) -> anyhow::Result<i32> {
        let now = self.pal_time_ms;
        let slot = self.slot_mut(handle)?;
        let Some(sound) = slot.handle.as_mut() else { return Ok(0); };
        if slot.data.is_none() || sound.state() == PlaybackState::Stopped || slot.pause_word == pause { return Ok(0); }
        slot.pause_word = pause;
        if duration_ms != 0 {
            slot.fade_start_ms = now;
            slot.fade_duration_ms = duration_ms;
        }
        let tween = Tween {
            duration: std::time::Duration::from_millis(u64::from(duration_ms as u32)),
            ..Tween::default()
        };
        if pause != 0 { sound.pause(tween); } else { sound.resume(tween); }
        Ok(1)
    }

    pub fn resume(&mut self, handle: AudioHandle) -> anyhow::Result<()> {
        self.pause_native(handle, 0, 0).map(|_| ())
    }

    pub fn release(&mut self, handle: AudioHandle) -> anyhow::Result<()> {
        self.stop(handle)?;
        let slot = self.slot_mut(handle)?;
        *slot = AudioSlot::default();
        Ok(())
    }

    pub fn release_group(&mut self, group: PalSoundGroup) -> anyhow::Result<()> {
        let len = self.groups.get(&group).map_or(0, Vec::len);
        for index in 0..len {
            self.release(AudioHandle::new(group, index))?;
        }
        Ok(())
    }

    pub fn set_primary_volume(&mut self, volume: PalVolume) -> anyhow::Result<()> {
        self.primary_volume = volume;
        log::debug!(
            "[trace-audio] set_primary_volume raw={}",
            self.primary_volume.raw()
        );
        self.apply_all_volumes()
    }

    pub fn primary_volume(&self) -> PalVolume {
        self.primary_volume
    }

    pub fn set_group_volume(
        &mut self,
        group: PalSoundGroup,
        volume: PalVolume,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(group.0 <= 6, "invalid native sound group");
        self.group_volumes.insert(group, volume.clamped());
        log::debug!(
            "[trace-audio] set_group_volume group={group:?} raw={}",
            volume.clamped().raw()
        );
        let len = self.groups.get(&group).map_or(0, Vec::len);
        for index in 0..len {
            self.apply_slot_volume(AudioHandle::new(group, index))?;
        }
        Ok(())
    }

    pub fn group_volume(&self, group: PalSoundGroup) -> PalVolume {
        *self.group_volumes.get(&group).unwrap_or(&PalVolume::MAX)
    }

    pub fn set_channel_volume(
        &mut self,
        handle: AudioHandle,
        volume: PalVolume,
    ) -> anyhow::Result<()> {
        let slot = self.slot_mut(handle)?;
        ensure_loaded(slot, handle)?;
        slot.volume = volume;
        log::debug!(
            "[trace-audio] set_channel_volume handle={handle:?} raw={}",
            volume.raw()
        );
        self.apply_slot_volume(handle)
    }

    pub fn channel_volume(&self, handle: AudioHandle) -> anyhow::Result<PalVolume> {
        let slot = self.slot(handle)?;
        ensure_loaded(slot, handle)?;
        Ok(slot.volume)
    }

    /// The native setter only stores this float. LoadEx consumes its initial
    /// value while amplifying PCM; subsequent writes do not rescale the buffer.
    pub fn set_volume_ratio(&mut self, handle: AudioHandle, gain: f32) -> anyhow::Result<i32> {
        let slot = self.slot_mut(handle)?;
        if slot.data.is_none() { return Ok(0); }
        slot.volume_ratio = gain;
        Ok(1)
    }

    /// Store native byte offsets without seeking or reinterpreting them as milliseconds.
    /// Group-3 playback converts through the retained source PCM layout;
    /// Ogg retains the native byte-offset shift by two.
    pub fn set_start_end(
        &mut self,
        handle: AudioHandle,
        loop_start_bytes: i32,
        loop_end_bytes: i32,
    ) -> anyhow::Result<()> {
        let slot = self.slot_mut(handle)?;
        ensure_loaded(slot, handle)?;
        slot.loop_start_bytes = loop_start_bytes;
        slot.loop_end_bytes = loop_end_bytes;
        slot.loop_bytes_explicit = true;
        if let Some(format) = slot.source_format.filter(|_| slot.source_group == PalSoundGroup::GROUP3) {
            slot.loop_start_samples = format.frame_at_byte(loop_start_bytes)? as i64;
            slot.loop_end_samples = format.frame_at_byte(loop_end_bytes)? as i64;
            if slot.loop_mode != 0 {
                let region = loop_region_for(true, slot.loop_start_samples, slot.loop_end_samples);
                if let Some(sound) = slot.handle.as_mut() { sound.set_loop_region(region); }
            }
        }
        Ok(())
    }

    /// `BGM.CSV` loop columns are PCM sample indices, not milliseconds.
    pub fn set_loop_samples(
        &mut self,
        handle: AudioHandle,
        loop_start_samples: i64,
        loop_end_samples: i64,
    ) -> anyhow::Result<()> {
        let slot = self.slot_mut(handle)?;
        ensure_loaded(slot, handle)?;
        slot.loop_start_samples = loop_start_samples;
        slot.loop_end_samples = loop_end_samples;
        if let Some(format) = slot.source_format {
            let stride = i64::from(format.seek_bytes_per_frame);
            if stride != 0 {
                slot.loop_start_bytes = loop_start_samples.wrapping_mul(stride) as i32;
                slot.loop_end_bytes = loop_end_samples.wrapping_mul(stride) as i32;
                slot.loop_bytes_explicit = loop_end_samples > 0;
            }
        }
        Ok(())
    }

    pub fn loop_samples(&self, handle: AudioHandle) -> anyhow::Result<(i64, i64)> {
        let slot = self.slot(handle)?;
        ensure_loaded(slot, handle)?;
        Ok((slot.loop_start_samples, slot.loop_end_samples))
    }

    pub fn start_end(&self, handle: AudioHandle) -> anyhow::Result<(i32, i32)> {
        let slot = self.slot(handle)?;
        ensure_loaded(slot, handle)?;
        Ok((slot.loop_start_bytes, slot.loop_end_bytes))
    }

    pub fn set_channel_pan(&mut self, handle: AudioHandle, pan: i32) -> anyhow::Result<()> {
        let slot = self.slot_mut(handle)?;
        ensure_loaded(slot, handle)?;
        slot.pan = pan.clamp(-10_000, 10_000);
        if let Some(sound) = slot.handle.as_mut() {
            sound.set_panning(pal_pan_settings(slot.pan).0, Tween {
                duration: std::time::Duration::ZERO,
                ..Tween::default()
            });
        }
        self.apply_slot_volume(handle)
    }

    pub fn channel_pan(&self, handle: AudioHandle) -> anyhow::Result<i32> {
        let slot = self.slot(handle)?;
        ensure_loaded(slot, handle)?;
        Ok(slot.pan)
    }

    pub fn set_channel_frequency(
        &mut self,
        handle: AudioHandle,
        frequency: i32,
    ) -> anyhow::Result<()> {
        let slot = self.slot_mut(handle)?;
        ensure_loaded(slot, handle)?;
        // DirectSound accepts zero as the original sample rate. Invalid rates
        // leave the channel unchanged; the VM wrapper ignores that API error.
        anyhow::ensure!(frequency == 0 || (100..=200_000).contains(&frequency), "invalid sound frequency");
        slot.frequency = frequency;
        let original = slot.data.as_ref().unwrap().sample_rate();
        let rate = if frequency == 0 { 1.0 } else { f64::from(frequency) / f64::from(original) };
        if let Some(sound) = slot.handle.as_mut() {
            sound.set_playback_rate(rate, Tween { duration: std::time::Duration::ZERO, ..Tween::default() });
        }
        Ok(())
    }

    pub fn channel_frequency(&self, handle: AudioHandle) -> anyhow::Result<i32> {
        let slot = self.slot(handle)?;
        ensure_loaded(slot, handle)?;
        Ok(if slot.frequency == 0 { slot.data.as_ref().unwrap().sample_rate() as i32 } else { slot.frequency })
    }

    /// Reviewed newer PalSoundGetParamByType selector map. Fields belonging to
    /// DirectSound's streaming ring or resource addresses have no portable
    /// equivalent yet and return an explicit error instead of a fake value.
    pub fn sound_param_by_type(&self, group: i32, index: u32, selector: i32) -> anyhow::Result<i32> {
        let Some(slots) = u8::try_from(group).ok().and_then(|group| self.groups.get(&PalSoundGroup(group))) else {
            return Ok(0);
        };
        let Some(slot) = slots.get(index as usize) else { return Ok(0); };
        let playing = slot.handle.as_ref().is_some_and(|sound| sound.state() != PlaybackState::Stopped);
        Ok(match selector {
            0 => slot.initial_start_bytes,
            1 => slot.loop_start_bytes,
            2 if slot.source_group == PalSoundGroup::GROUP3 && slot.data.is_some() && !slot.loop_bytes_explicit && slot.source_format.is_none() => {
                anyhow::bail!("native default loop end needs original decoded PCM byte length")
            }
            2 => slot.loop_end_bytes,
            4 if slot.data.is_some() && !matches!(slot.source_group.0, 0 | 3) => {
                let format = slot.source_format.filter(|format| format.native_pal_format())
                    .ok_or_else(|| anyhow::anyhow!("native buffer byte size unavailable"))?;
                format.decoded_bytes as i32
            }
            6 if playing && !matches!(slot.source_group.0, 0 | 3) => {
                let format = slot.source_format.filter(|format| format.native_pal_format() && format.seek_bytes_per_frame != 0)
                    .ok_or_else(|| anyhow::anyhow!("native buffer cursor byte format unavailable"))?;
                let position = slot.handle.as_ref().unwrap().position();
                let frame = (position * f64::from(format.sample_rate)).floor() as u64;
                frame.wrapping_mul(u64::from(format.block_align)) as i32
            }
            10 if slot.data.is_some() => {
                let format = slot.source_format.filter(|format| format.native_pal_format())
                    .ok_or_else(|| anyhow::anyhow!("native source format flag unavailable"))?;
                i32::from(format.kind == PcmSourceKind::Ogg)
            }
            19 if slot.copied_resource => 0,
            23 if playing && slot.source_group == PalSoundGroup::GROUP3 => {
                let format = slot.source_format.filter(|format| format.native_pal_format() && format.seek_bytes_per_frame == format.block_align)
                    .ok_or_else(|| anyhow::anyhow!("native source cursor byte alignment unavailable"))?;
                let position = slot.handle.as_ref().unwrap().position();
                let frame = (position * f64::from(format.sample_rate)).floor() as u64;
                frame.wrapping_mul(u64::from(format.block_align)) as i32
            }
            3 | 4 | 11 | 19 | 22 | 23 if slot.data.is_some() => {
                anyhow::bail!("native sound selector {selector} needs unimplemented resource/streaming metadata")
            }
            3 | 4 | 10 | 11 | 19 | 22 | 23 => 0,
            5 | 6 if slot.data.is_none() || !playing => -1,
            5 => self.pal_time_ms.wrapping_sub(slot.fade_start_ms) as i32,
            6 => anyhow::bail!("native streaming ring-buffer play cursor is unavailable"),
            7 => i32::from(slot.data.is_some()),
            8 => i32::from(playing),
            9 => slot.loop_mode,
            12 => slot.pause_word,
            13 => slot.native_worker_command(),
            14 => i32::from(slot.source_group.0),
            15 => slot.handle.as_ref().map_or(0, |sound| sound.fade_state()),
            16 => slot.fade_duration_ms,
            20 => slot.fade_start_ms as i32,
            17 => slot.reserved_word,
            18 => i32::from(slot.reserved_word != 0 || slot.data.is_some() || playing),
            21 => if slot.data.is_some() { slot.volume.raw() } else { 0 },
            _ => 0,
        })
    }

    /// PalSoundGetParam addresses group 3; the earlier reference only supports
    /// selectors 0..4. Out-of-range indices are rejected rather than emulating
    /// the original unchecked memory read.
    pub fn sound_param(&self, index: i32, selector: i32, extended: bool) -> anyhow::Result<i32> {
        anyhow::ensure!((0..2).contains(&index), "invalid native group 3 sound index");
        if !extended && !(0..=4).contains(&selector) { return Ok(0); }
        self.sound_param_by_type(3, index as u32, selector)
    }

    /// The native zero-argument query returns an index, not a channel count.
    pub fn now_channel(&self) -> i32 {
        self.groups.get(&PalSoundGroup::GROUP3).and_then(|slots| slots.iter().position(|slot| {
            slot.handle.as_ref().is_some_and(|sound| sound.state() != PlaybackState::Stopped)
        })).map_or(-1, |index| index as i32)
    }

    pub fn native_free_channel_count(&self, group: i32) -> i32 {
        let group = if (0..=6).contains(&group) { PalSoundGroup(group as u8) } else { PalSoundGroup::GROUP0 };
        self.free_channel_count(group) as i32
    }

    pub fn native_group3_status(&self, index: i32) -> anyhow::Result<i32> {
        anyhow::ensure!((0..2).contains(&index), "invalid native group 3 sound index");
        let slot = self.slot(AudioHandle::new(PalSoundGroup::GROUP3, index as usize))?;
        Ok(slot.native_worker_command())
    }

    pub fn fade_state(&self, handle: AudioHandle) -> anyhow::Result<i32> {
        let slot = self.slot(handle)?;
        Ok(if slot.data.is_none() { -1 } else { slot.handle.as_ref().map_or(0, |sound| sound.fade_state()) })
    }

    /// Portable convenience enum; this is not native PalSoundGetStatus.
    pub fn sound_status(&self, handle: AudioHandle) -> anyhow::Result<PalSoundStatus> {
        let slot = self.slot(handle)?;
        if slot.data.is_none() {
            return Ok(PalSoundStatus::Free);
        }
        if let Some(sound) = slot.handle.as_ref() {
            return Ok(match sound.state() {
                PlaybackState::Playing => PalSoundStatus::Playing,
                PlaybackState::Pausing
                | PlaybackState::Paused
                | PlaybackState::WaitingToResume
                | PlaybackState::Resuming => PalSoundStatus::Paused,
                PlaybackState::Stopping | PlaybackState::Stopped => PalSoundStatus::Stopped,
            });
        }
        Ok(PalSoundStatus::Loaded)
    }

    pub fn loaded_channel_count_for_handle(&self, handle: AudioHandle) -> usize {
        self.loaded_channel_count(handle.group)
    }

    pub fn group_channel_count(&self, group: PalSoundGroup) -> usize {
        group.slot_count()
    }

    pub fn now_channel_count(&self, group: PalSoundGroup) -> usize {
        self.groups
            .get(&group)
            .map(|slots| {
                slots
                    .iter()
                    .filter(|slot| {
                        slot.handle
                            .as_ref()
                            .is_some_and(|sound| sound.state().is_advancing())
                    })
                    .count()
            })
            .unwrap_or(0)
    }

    pub fn is_playing(&self, handle: AudioHandle) -> anyhow::Result<bool> {
        let slot = self.slot(handle)?;
        Ok(match slot.handle.as_ref() {
            Some(sound) if slot.source_group == PalSoundGroup::GROUP3 => sound.state() != PlaybackState::Stopped,
            Some(sound) => sound.state().is_advancing(),
            None => false,
        })
    }

    pub fn loaded_channel_count(&self, group: PalSoundGroup) -> usize {
        self.groups
            .get(&group)
            .map(|slots| slots.iter().filter(|slot| slot.data.is_some()).count())
            .unwrap_or(0)
    }

    pub fn free_channel_count(&self, group: PalSoundGroup) -> usize {
        self.groups
            .get(&group)
            .map(|slots| slots.iter().filter(|slot| slot.data.is_none() && slot.reserved_word == 0).count())
            .unwrap_or(0)
    }

    pub fn update(&mut self) {
        for slots in self.groups.values_mut() {
            for slot in slots {
                if let Some(sound) = &mut slot.handle { sound.report_errors(); }
                if slot
                    .handle
                    .as_ref()
                    .is_some_and(|sound| matches!(sound.state(), PlaybackState::Stopped))
                {
                    let natural_end = slot.handle.as_ref().is_some_and(NativeSoundHandle::ended_naturally);
                    slot.worker_command = slot.native_worker_command();
                    slot.handle = None;
                    if slot.source_group == PalSoundGroup::GROUP3 || (slot.source_group == PalSoundGroup::GROUP0 && natural_end) {
                        slot.data = None; slot.reserved_word = 0;
                    }
                }
            }
        }
    }

    fn find_free_slot(&self, group: PalSoundGroup) -> anyhow::Result<usize> {
        self.groups
            .get(&group)
            .and_then(|slots| slots.iter().position(|slot| slot.data.is_none() && slot.reserved_word == 0))
            .ok_or_else(|| anyhow::anyhow!("no free audio slot in group {:?}", group))
    }

    fn apply_all_volumes(&mut self) -> anyhow::Result<()> {
        for group in PalSoundGroup::ALL {
            let len = self.groups.get(&group).map_or(0, Vec::len);
            for index in 0..len {
                self.apply_slot_volume(AudioHandle::new(group, index))?;
            }
        }
        Ok(())
    }

    fn apply_slot_volume(&mut self, handle: AudioHandle) -> anyhow::Result<()> {
        let effective = self.effective_volume(handle)?;
        // Invalid DirectSound gain leaves the existing buffer level intact.
        // The native retry timeout is not reproduced by the portable mixer.
        let Some(decibels) = native_volume_decibels(effective) else { return Ok(()); };
        let decibels = decibels + pal_pan_settings(self.slot(handle)?.pan).1;
        let slot = self.slot_mut(handle)?;
        let has_loaded_sound = slot.data.is_some() || slot.handle.is_some();
        if has_loaded_sound {
            log::debug!(
                "[trace-audio] apply_slot_volume handle={handle:?} effective_raw={} db={decibels:.2}",
                effective.raw()
            );
        }
        if let Some(sound) = slot.handle.as_mut() {
            sound.set_volume(decibels, immediate_audio_tween());
        }
        Ok(())
    }

    pub fn effective_volume(&self, handle: AudioHandle) -> anyhow::Result<PalVolume> {
        let slot = self.slot(handle)?;
        let group = self.group_volume(slot.source_group);
        // The native helper rounds the group/channel product through float32,
        // then performs a wrapping 32-bit master multiplication.
        let combined = (group.raw() as f32 * (slot.volume.raw() as f32 / 10000.0)).trunc();
        let combined = if !combined.is_finite() || !(-2147483648.0..2147483648.0).contains(&combined) {
            i32::MIN
        } else { combined as i32 };
        Ok(PalVolume::from_raw(combined.wrapping_mul(self.primary_volume.raw()) / 10000))
    }

    fn slot(&self, handle: AudioHandle) -> anyhow::Result<&AudioSlot> {
        let slots = self
            .groups
            .get(&handle.group)
            .ok_or_else(|| anyhow::anyhow!("invalid audio group {:?}", handle.group))?;
        slots
            .get(handle.index)
            .ok_or_else(|| anyhow::anyhow!("audio handle {:?} points outside its group", handle))
    }

    fn slot_mut(&mut self, handle: AudioHandle) -> anyhow::Result<&mut AudioSlot> {
        let slots = self
            .groups
            .get_mut(&handle.group)
            .ok_or_else(|| anyhow::anyhow!("invalid audio group {:?}", handle.group))?;
        slots
            .get_mut(handle.index)
            .ok_or_else(|| anyhow::anyhow!("audio handle {:?} points outside its group", handle))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PalSoundStatus {
    Free,
    Loaded,
    Playing,
    Paused,
    Stopped,
}

impl PalSoundStatus {
    pub const fn raw(self) -> i32 {
        match self {
            Self::Free => 0,
            Self::Loaded => 1,
            Self::Playing => 2,
            Self::Paused => 3,
            Self::Stopped => 4,
        }
    }
}

/// PAL forwards pan directly to DirectSound SetPan: hundredths of a dB
/// attenuate the opposite speaker, with the near speaker unchanged. Convert
/// both pan and gain to Kira's equal-power law so panning never boosts it.
fn pal_pan_settings(pan: i32) -> (f32, f32) {
    let pan = pan.clamp(-10_000, 10_000);
    let quiet_power = 10.0_f32.powf(-(pan.abs() as f32) / 1000.0);
    let panning = (1.0 - quiet_power) / (1.0 + quiet_power) * pan.signum() as f32;
    let gain_db = 10.0 * ((1.0 + quiet_power) * 0.5).log10();
    (panning, gain_db)
}

fn decode_asset_sound(bytes: &[u8], _group: PalSoundGroup)
    -> anyhow::Result<(LoadedSound, PcmSourceFormat)>
{
    #[cfg(target_os = "vita")]
    if _group == PalSoundGroup::GROUP3 && bytes.starts_with(b"OggS") {
        let stream = stream::StreamAsset::new(bytes)?;
        let format = stream.source_format;
        return Ok((LoadedSound::Stream(stream), format));
    }
    let data = decode_static_sound(bytes)?;
    let format = source_format::describe_source(bytes, &data)?;
    Ok((LoadedSound::Static(data), format))
}

#[derive(Clone)]
enum LoadedSound {
    Static(StaticSoundData),
    #[cfg(not(target_arch = "wasm32"))]
    Stream(stream::StreamAsset),
}
impl LoadedSound {
    fn sample_rate(&self) -> u32 {
        match self { Self::Static(data) => data.sample_rate, #[cfg(not(target_arch = "wasm32"))] Self::Stream(data) => data.sample_rate }
    }
    fn num_frames(&self) -> usize {
        match self { Self::Static(data) => data.frames.len(), #[cfg(not(target_arch = "wasm32"))] Self::Stream(data) => data.frame_count }
    }
    #[cfg(test)]
    fn static_data(&self) -> &StaticSoundData {
        match self { Self::Static(data) => data, #[cfg(not(target_arch = "wasm32"))] Self::Stream(_) => panic!("expected static PCM") }
    }
}

#[derive(Default)]
struct AudioSlot {
    name: Option<String>,
    data: Option<LoadedSound>,
    handle: Option<NativeSoundHandle>,
    loop_mode: i32,
    volume: PalVolume,
    initial_start_bytes: i32,
    loop_start_bytes: i32,
    loop_end_bytes: i32,
    loop_bytes_explicit: bool,
    source_group: PalSoundGroup,
    source_format: Option<PcmSourceFormat>,
    pause_word: i32,
    reserved_word: i32,
    worker_command: i32,
    fade_start_ms: u32,
    fade_duration_ms: i32,
    copied_resource: bool,
    loop_start_samples: i64,
    loop_end_samples: i64,
    pan: i32,
    frequency: i32,
    volume_ratio: f32,
}

impl AudioSlot {
    fn native_worker_command(&self) -> i32 {
        self.handle.as_ref().map_or(self.worker_command, |sound| {
            if matches!(self.source_group.0, 0 | 3) && sound.ended_naturally() { 0 }
            else { sound.worker_command() }
        })
    }
}

/// One `BGM.CSV` row. Loop columns are PCM sample indices in that file.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BgmLoop {
    pub intro_samples: i64,
    pub loop_start_samples: i64,
    pub loop_end_samples: i64,
}

pub fn audio_lookup_key(name: &str) -> String {
    let leaf = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let stem = match leaf.rsplit_once('.') {
        Some((stem, ext))
            if ext.eq_ignore_ascii_case("ogg")
                || ext.eq_ignore_ascii_case("wav")
                || ext.eq_ignore_ascii_case("wma")
                || ext.eq_ignore_ascii_case("mix") =>
        {
            stem
        }
        _ => leaf,
    };
    stem.to_ascii_uppercase()
}

/// Parse Palette `BGM.CSV` (`name, intro, loop_start, loop_end`).
pub fn parse_bgm_csv(bytes: &[u8]) -> BTreeMap<String, BgmLoop> {
    let text = decode_bgm_csv_text(bytes);
    let mut table = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim().trim_start_matches('\u{feff}');
        if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
            continue;
        }
        let parts = split_csv_row(line);
        if parts.len() < 4 {
            continue;
        }
        let name = parts[0].trim().trim_matches('"').trim();
        if name.is_empty() {
            continue;
        }
        let Ok(intro_samples) = parts[1].trim().parse::<i64>() else {
            continue;
        };
        let Ok(loop_start_samples) = parts[2].trim().parse::<i64>() else {
            continue;
        };
        let Ok(loop_end_samples) = parts[3].trim().parse::<i64>() else {
            continue;
        };
        table.insert(
            audio_lookup_key(name),
            BgmLoop {
                intro_samples,
                loop_start_samples,
                loop_end_samples,
            },
        );
    }
    table
}

/// Voice lists use null-padded 32-byte names in older releases or 32 UTF-16LE
/// code units in the native 0x442C10 reader. They are not encoded audio.
pub fn parse_mix_playlist(bytes: &[u8]) -> Option<Vec<String>> {
    if !bytes.is_empty() && bytes.len() % 64 == 0 && !is_encoded_audio(bytes) {
        let wide: Option<Vec<String>> = bytes.chunks_exact(64).map(|chunk| {
            let units: Vec<u16> = chunk.chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]])).collect();
            let end = units.iter().position(|&unit| unit == 0)?;
            if end == 0 || units[end..].iter().any(|&unit| unit != 0) {
                return None;
            }
            let name = String::from_utf16(&units[..end]).ok()?;
            // Distinguish narrow records: ASCII names packed as UTF-16 form
            // arbitrary CJK codepoints, but never a valid ASCII first byte.
            if !name.chars().next()?.is_ascii_graphic() || name.chars().any(char::is_control) {
                return None;
            }
            Some(name)
        }).collect();
        if wide.is_some() { return wide; }
    }
    if bytes.is_empty() || bytes.len() % 32 != 0 || is_encoded_audio(bytes) {
        return None;
    }
    let mut names = Vec::new();
    for chunk in bytes.chunks_exact(32) {
        let end = chunk.iter().position(|&byte| byte == 0)?;
        if end == 0 || chunk[end..].iter().any(|&byte| byte != 0) {
            return None;
        }
        let raw = &chunk[..end];
        if !raw
            .iter()
            .all(|byte| byte.is_ascii_graphic() || *byte >= 0x80)
        {
            return None;
        }
        names.push(decode_mix_name(raw));
    }
    if names.is_empty() {
        None
    } else {
        Some(names)
    }
}

pub fn decode_static_sound(bytes: &[u8]) -> anyhow::Result<StaticSoundData> {
    if is_asf_container(bytes) {
        return decode_wma_sound(bytes);
    }
    decode::decode_static(bytes)
}

/// Retain source format when a decoded clip still corresponds to one input
/// stream. A MIX sum has no native single-buffer byte layout.
pub fn source_pcm_format(bytes: &[u8], data: &StaticSoundData) -> anyhow::Result<Option<PcmSourceFormat>> {
    if parse_mix_playlist(bytes).is_some() { return Ok(None); }
    source_format::describe_source(bytes, data).map(Some)
}

/// Decode one voice, or sum every clip named by a `.MIX` playlist.
pub fn decode_game_audio(
    bytes: &[u8],
    open_member: &mut dyn FnMut(&str) -> anyhow::Result<Vec<u8>>,
) -> anyhow::Result<StaticSoundData> {
    decode_game_audio_depth(bytes, open_member, 2)
}

fn decode_game_audio_depth(
    bytes: &[u8],
    open_member: &mut dyn FnMut(&str) -> anyhow::Result<Vec<u8>>,
    depth: u8,
) -> anyhow::Result<StaticSoundData> {
    if depth > 0 {
        if let Some(names) = parse_mix_playlist(bytes) {
            let mut sounds = Vec::new();
            for name in names {
                match open_member(&name) {
                    Ok(member) => match decode_game_audio_depth(&member, open_member, depth - 1) {
                        Ok(sound) => sounds.push(sound),
                        Err(err) => {
                            log::warn!("[trace-audio] mix member {name:?} decode failed: {err}");
                        }
                    },
                    Err(err) => {
                        log::warn!("[trace-audio] mix member {name:?} open failed: {err}");
                    }
                }
            }
            if sounds.is_empty() {
                anyhow::bail!("mix playlist produced no playable voices");
            }
            return mix_static_sounds(&sounds);
        }
    }
    decode_static_sound(bytes)
}

pub fn mix_static_sounds(sounds: &[StaticSoundData]) -> anyhow::Result<StaticSoundData> {
    let first = sounds
        .first()
        .ok_or_else(|| anyhow::anyhow!("mix playlist is empty"))?;
    if sounds.len() == 1 {
        return Ok(first.clone());
    }
    let sample_rate = first.sample_rate.max(1);
    let mut mixed: Vec<Frame> = Vec::new();
    for sound in sounds {
        let frames = frames_at_rate(sound, sample_rate);
        if frames.len() > mixed.len() {
            mixed.resize(frames.len(), Frame::ZERO);
        }
        for (dst, src) in mixed.iter_mut().zip(frames) {
            *dst += src;
            dst.left = dst.left.clamp(-1.0, 1.0);
            dst.right = dst.right.clamp(-1.0, 1.0);
        }
    }
    Ok(StaticSoundData {
        sample_rate,
        frames: Arc::from(mixed),
        settings: StaticSoundSettings::new(),
        slice: None,
    })
}

pub fn apply_loop_region(
    data: StaticSoundData,
    looping: bool,
    loop_start_samples: i64,
    loop_end_samples: i64,
) -> StaticSoundData {
    let Some(region) = loop_region_for(looping, loop_start_samples, loop_end_samples) else {
        return data;
    };
    data.loop_region(region)
}

fn loop_region_for(
    looping: bool,
    loop_start_samples: i64,
    loop_end_samples: i64,
) -> Option<Region> {
    if !looping {
        return None;
    }
    if loop_end_samples > loop_start_samples && loop_start_samples >= 0 {
        let start = loop_start_samples as usize;
        let end = loop_end_samples as usize;
        Some(Region {
            start: PlaybackPosition::Samples(start),
            end: EndPosition::Custom(PlaybackPosition::Samples(end)),
        })
    } else {
        Some(Region {
            start: PlaybackPosition::Samples(0),
            end: EndPosition::EndOfAudio,
        })
    }
}

fn frames_at_rate(sound: &StaticSoundData, sample_rate: u32) -> Vec<Frame> {
    let source = sound.frames.as_ref();
    if source.is_empty() || sound.sample_rate == 0 || sound.sample_rate == sample_rate {
        return source.to_vec();
    }
    let dst_len = ((source.len() as u64 * u64::from(sample_rate)) / u64::from(sound.sample_rate))
        .max(1) as usize;
    let mut out = Vec::with_capacity(dst_len);
    for index in 0..dst_len {
        let src_pos = index as f64 * f64::from(sound.sample_rate) / f64::from(sample_rate);
        let src_index = src_pos.floor() as usize;
        let frac = (src_pos - src_index as f64) as f32;
        let left = source.get(src_index).copied().unwrap_or(Frame::ZERO);
        let right = source.get(src_index + 1).copied().unwrap_or(left);
        out.push(Frame::new(
            left.left + (right.left - left.left) * frac,
            left.right + (right.right - left.right) * frac,
        ));
    }
    out
}

fn is_asf_container(bytes: &[u8]) -> bool {
    bytes.len() >= 16 && bytes.starts_with(&[0x30, 0x26, 0xB2, 0x75])
}

fn is_encoded_audio(bytes: &[u8]) -> bool {
    is_asf_container(bytes)
        || bytes.starts_with(b"OggS")
        || bytes.starts_with(b"RIFF")
        || bytes.starts_with(b"ID3")
        || bytes.starts_with(&[0xFF, 0xFB])
}

fn decode_wma_sound(bytes: &[u8]) -> anyhow::Result<StaticSoundData> {
    let mut decoder = AsfWmaDecoder::open(Cursor::new(bytes.to_vec()))
        .map_err(|err| anyhow::anyhow!("wma open failed: {err}"))?;
    let sample_rate = decoder.sample_rate().max(1);
    let channels = usize::from(decoder.channels().max(1));
    let mut frames = Vec::new();
    loop {
        match decoder.next_frame() {
            Ok(Some(decoded)) => {
                append_interleaved_pcm(&mut frames, &decoded.frame.samples, channels)
            }
            Ok(None) => break,
            Err(wmv_decoder::DecoderError::EndOfStream) => break,
            Err(err) => return Err(anyhow::anyhow!("wma decode failed: {err}")),
        }
    }
    if frames.is_empty() {
        anyhow::bail!("wma decode produced no samples");
    }
    Ok(StaticSoundData {
        sample_rate,
        frames: Arc::from(frames),
        settings: StaticSoundSettings::new(),
        slice: None,
    })
}

fn append_interleaved_pcm(frames: &mut Vec<Frame>, samples: &[f32], channels: usize) {
    if channels <= 1 {
        frames.extend(samples.iter().copied().map(Frame::from_mono));
        return;
    }
    for chunk in samples.chunks(channels) {
        let left = chunk.first().copied().unwrap_or(0.0);
        let right = chunk.get(1).copied().unwrap_or(left);
        frames.push(Frame::new(left, right));
    }
}

fn decode_bgm_csv_text(bytes: &[u8]) -> String {
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_owned();
    }
    let (text, _, _) = encoding_rs::SHIFT_JIS.decode(bytes);
    text.into_owned()
}

fn decode_mix_name(bytes: &[u8]) -> String {
    if bytes.iter().all(|byte| byte.is_ascii()) {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    let (text, _, _) = encoding_rs::SHIFT_JIS.decode(bytes);
    text.into_owned()
}

fn split_csv_row(line: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut rest = line;
    while !rest.is_empty() {
        if rest.starts_with('"') {
            if let Some(end) = rest[1..].find('"') {
                parts.push(&rest[1..1 + end]);
                rest = rest[1 + end + 1..].trim_start_matches(',').trim_start();
                continue;
            }
        }
        if let Some(comma) = rest.find(',') {
            parts.push(rest[..comma].trim());
            rest = rest[comma + 1..].trim_start();
        } else {
            parts.push(rest.trim());
            break;
        }
    }
    parts
}

fn ensure_loaded(slot: &AudioSlot, handle: AudioHandle) -> anyhow::Result<()> {
    if slot.data.is_some() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "audio handle {:?} has no loaded sound data",
            handle
        ))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct AudioHandle {
    pub group: PalSoundGroup,
    pub index: usize,
}

impl AudioHandle {
    pub fn new(group: PalSoundGroup, index: usize) -> Self {
        Self { group, index }
    }

    pub fn raw(self) -> u32 {
        self.group.raw_prefix() | self.index as u32
    }

    pub fn from_raw(raw: u32) -> Option<Self> {
        let prefix = raw & 0xF000_0000;
        let index = (raw & 0x0FFF_FFFF) as usize;
        PalSoundGroup::from_raw_prefix(prefix).map(|group| Self { group, index })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PalSoundGroup(pub u8);

impl Default for PalSoundGroup {
    fn default() -> Self { Self::GROUP0 }
}

impl PalSoundGroup {
    pub const GROUP0: Self = Self(0);
    pub const GROUP1: Self = Self(1);
    pub const GROUP2: Self = Self(2);
    pub const GROUP3: Self = Self(3);
    pub const GROUP4: Self = Self(4);
    pub const GROUP5: Self = Self(5);
    pub const GROUP6: Self = Self(6);
    pub const ALL: [Self; 7] = [
        Self::GROUP0,
        Self::GROUP1,
        Self::GROUP2,
        Self::GROUP3,
        Self::GROUP4,
        Self::GROUP5,
        Self::GROUP6,
    ];

    pub fn from_original_kind(kind: i32) -> Self {
        match kind {
            0 => Self::GROUP0,
            2 => Self::GROUP2,
            3 => Self::GROUP3,
            4 => Self::GROUP4,
            5 => Self::GROUP5,
            6 => Self::GROUP6,
            _ => Self::GROUP1,
        }
    }

    pub fn slot_count(self) -> usize {
        match self.0 {
            0 => 2,
            1 => 16,
            2 => 8,
            3 => 2,
            4 => 16,
            5 => 64,
            6 => 16,
            _ => 0,
        }
    }

    pub fn raw_prefix(self) -> u32 {
        match self.0 {
            0 => 0x1000_0000,
            1 => 0x3000_0000,
            2 => 0x7000_0000,
            3 => 0x2000_0000,
            4 => 0x4000_0000,
            5 => 0x5000_0000,
            6 => 0x6000_0000,
            _ => 0,
        }
    }

    pub fn from_raw_prefix(prefix: u32) -> Option<Self> {
        match prefix {
            0x1000_0000 => Some(Self::GROUP0),
            0x3000_0000 => Some(Self::GROUP1),
            0x7000_0000 => Some(Self::GROUP2),
            0x2000_0000 => Some(Self::GROUP3),
            0x4000_0000 => Some(Self::GROUP4),
            0x5000_0000 => Some(Self::GROUP5),
            0x6000_0000 => Some(Self::GROUP6),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PalVolume(i32);

impl PalVolume {
    pub const MIN: Self = Self(0);
    pub const MAX: Self = Self(10000);

    pub fn from_raw(value: i32) -> Self {
        Self(value)
    }

    pub fn raw(self) -> i32 {
        self.0
    }

    pub fn clamped(self) -> Self {
        Self(self.0.clamp(Self::MIN.0, Self::MAX.0))
    }

    pub fn to_decibels(self) -> f64 {
        let raw = self.clamped().0;
        if raw == 0 {
            -100.0
        } else {
            (2000.0 * (raw as f64 / 10000.0).log10()).trunc() / 100.0
        }
    }
}

impl Default for PalVolume {
    fn default() -> Self {
        Self::MAX
    }
}

fn amplify_pcm16(sample: f32, gain: f32) -> f32 {
    let pcm = (sample * 32768.0).clamp(-32768.0, 32767.0).trunc();
    let scaled = pcm * gain;
    // CVTTSS2SI returns INT_MIN on overflow, rather than Rust's saturating
    // float cast. The native loop then clamps that result to signed 16 bits.
    let integer = if !scaled.is_finite() || scaled >= 2147483648.0 || scaled < -2147483648.0 {
        i32::MIN
    } else { scaled as i32 };
    integer.clamp(-32768, 32767) as f32 / 32768.0
}

fn native_volume_decibels(volume: PalVolume) -> Option<f32> {
    (0..=10_000).contains(&volume.raw()).then(|| volume.to_decibels() as f32)
}

fn immediate_audio_tween() -> Tween {
    Tween { duration: std::time::Duration::ZERO, ..Tween::default() }
}

fn prepare_playback(mut data: StaticSoundData, decibels: f32, fade_ms: i32) -> NativeSoundData {
    data.settings.fade_in_tween = None;
    NativeSoundData { data: data.volume(decibels), fade_ms: fade_ms as u32, clock_ms: None }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn constant_sound() -> StaticSoundData {
        StaticSoundData { sample_rate: 48000, frames: vec![Frame::from_mono(0.25); 48000].into(),
            settings: StaticSoundSettings::default(), slice: None }
    }

    #[cfg(feature = "host-audio")]
    #[test]
    fn replay_stops_the_prior_mixer_sound_and_immediate_stop_leaves_no_orphan() {
        let mut audio = AudioSystem::new(AudioConfig { enabled: true }).unwrap();
        let sink = audio.host_sink();
        let handle = audio.load_static_data("repeat", constant_sound(), PalSoundGroup::GROUP4).unwrap();
        let mut samples = [0; 1024];
        audio.play(handle, true).unwrap();
        sink.render_i16(&mut samples);
        assert!(samples.iter().any(|&sample| sample > 100));
        audio.play(handle, true).unwrap();
        sink.render_i16(&mut samples);
        audio.stop(handle).unwrap();
        sink.render_i16(&mut samples);
        assert!(samples.iter().all(|&sample| sample == 0));
    }

    #[test]
    fn copied_sound_preserves_scalar_configuration_without_starting_playback() {
        let mut audio = AudioSystem::new(AudioConfig { enabled: false }).unwrap();
        let source = audio.load_static_data("source", constant_sound(), PalSoundGroup::GROUP4).unwrap();
        audio.set_channel_volume(source, PalVolume::from_raw(3456)).unwrap();
        audio.set_channel_pan(source, 1234).unwrap();
        audio.set_channel_frequency(source, 24000).unwrap();
        audio.set_start_end(source, 80, 500).unwrap();
        audio.set_loop_samples(source, 120, 450).unwrap();
        let copied = audio.copy_sound(source, PalSoundGroup::GROUP5).unwrap();
        assert_eq!(audio.channel_volume(copied).unwrap().raw(),3456);
        assert_eq!(audio.channel_pan(copied).unwrap(),1234);
        assert_eq!(audio.channel_frequency(copied).unwrap(),24000);
        assert_eq!(audio.start_end(copied).unwrap(),(80,500));
        assert_eq!(audio.loop_samples(copied).unwrap(),(120,450));
        assert!(!audio.is_playing(copied).unwrap());
        audio.release(source).unwrap();
        assert_eq!(audio.channel_frequency(copied).unwrap(),24000);
    }
    #[test]
    fn native_query_map_preserves_byte_bounds_group_and_raw_gain() {
        let mut audio = AudioSystem::new(AudioConfig { enabled: false }).unwrap();
        let handle = audio.load_static_data("native", constant_sound(), PalSoundGroup::GROUP3).unwrap();
        audio.set_start_end(handle, -4, i32::MAX).unwrap();
        audio.set_primary_volume(PalVolume::from_raw(12_345)).unwrap();
        audio.set_channel_volume(handle, PalVolume::from_raw(-123)).unwrap();
        audio.set_group_volume(PalSoundGroup::GROUP3, PalVolume::from_raw(12_345)).unwrap();
        assert_eq!(audio.primary_volume().raw(), 12_345);
        assert_eq!(audio.channel_volume(handle).unwrap().raw(), -123);
        assert_eq!(audio.group_volume(PalSoundGroup::GROUP3).raw(), 10_000);
        assert_eq!(audio.sound_param(0, 0, true).unwrap(), 0);
        assert_eq!(audio.sound_param(0, 1, true).unwrap(), -4);
        assert_eq!(audio.sound_param(0, 2, true).unwrap(), i32::MAX);
        assert_eq!(audio.sound_param(0, 7, true).unwrap(), 1);
        assert_eq!(audio.sound_param(0, 14, true).unwrap(), 3);
        assert_eq!(audio.sound_param(0, 21, true).unwrap(), -123);
        assert_eq!(audio.sound_param(0, 21, false).unwrap(), 0);
        assert!(audio.sound_param(0, 19, true).is_err());
        assert_eq!(audio.sound_param_by_type(7, 0, 7).unwrap(), 0);
        assert_eq!(audio.sound_param_by_type(3, 2, 7).unwrap(), 0);
        assert_eq!(audio.native_free_channel_count(-1), 2);
        assert_eq!(audio.native_free_channel_count(3), 1);
        assert_eq!(audio.now_channel(), -1);
        audio.stop(handle).unwrap();
        assert_eq!(audio.loaded_channel_count(PalSoundGroup::GROUP3), 1);
        audio.release(handle).unwrap();
        assert_eq!(audio.sound_param(0, 7, true).unwrap(), 0);
    }

    #[test]
    fn copied_buffer_keeps_source_group_volume_selection() {
        let mut audio = AudioSystem::new(AudioConfig { enabled: false }).unwrap();
        let source = audio.load_static_data("group", constant_sound(), PalSoundGroup::GROUP4).unwrap();
        audio.set_group_volume(PalSoundGroup::GROUP4, PalVolume::from_raw(2500)).unwrap();
        audio.set_group_volume(PalSoundGroup::GROUP5, PalVolume::from_raw(9000)).unwrap();
        let copied = audio.copy_sound(source, PalSoundGroup::GROUP5).unwrap();
        assert_eq!(audio.effective_volume(copied).unwrap().raw(), 2500);
        assert_eq!(audio.sound_param_by_type(5, copied.index as u32, 14).unwrap(), 4);
    }

    #[cfg(feature = "host-audio")]
    #[test]
    fn raw_pause_words_compare_without_boolean_narrowing_and_group3_retains_play_latch() {
        let mut audio = AudioSystem::new(AudioConfig { enabled: true }).unwrap();
        let sink = audio.host_sink();
        let handle = audio.load_static_data("pause", constant_sound(), PalSoundGroup::GROUP3).unwrap();
        let mut samples = [0; 512];
        audio.play_native(handle, -1, 0).unwrap();
        sink.render_i16(&mut samples);
        assert_eq!(audio.sound_param(0, 9, true).unwrap(), -1);
        assert_eq!(audio.pause_native(handle, 0x100, 0).unwrap(), 1);
        assert_eq!(audio.pause_native(handle, 0x100, 0).unwrap(), 0);
        assert_eq!(audio.pause_native(handle, 1, 0).unwrap(), 1);
        sink.render_i16(&mut samples);
        assert!(samples.iter().all(|&sample| sample == 0));
        assert!(audio.is_playing(handle).unwrap());
        assert_eq!(audio.now_channel(), 0);
        assert_eq!(audio.sound_param(0, 12, true).unwrap(), 1);
        audio.stop_fade(handle, 1).unwrap();
        // Native fades follow the PAL clock, independently of PCM consumption.
        audio.set_pal_time(1);
        sink.render_i16(&mut samples);
        audio.update();
        assert_eq!(audio.loaded_channel_count(PalSoundGroup::GROUP3), 0);
        assert_eq!(audio.now_channel(), -1);
    }

    #[test]
    fn load_ex_amplifies_pcm_before_channel_volume_and_frequency_queries_use_sample_rate() {
        use super::*;
        let data = StaticSoundData { sample_rate: 8000,
            frames: Arc::from([Frame::new(0.25, -0.75), Frame::new(0.75, -0.25)]),
            settings: StaticSoundSettings::default(), slice: None };
        let mut audio = AudioSystem::new(AudioConfig { enabled: false }).unwrap();
        assert!(audio.load_static_data_ex("bad", data.clone(), PalSoundGroup::GROUP5, 2.0).is_err());
        let handle = audio.load_static_data_ex("gain", data, PalSoundGroup::GROUP4, 2.0).unwrap();
        let data = audio.slot(handle).unwrap().data.as_ref().unwrap().static_data();
        assert_eq!(data.frames[0], Frame::new(0.5, -1.0));
        assert_eq!(data.frames[1], Frame::new(32767.0 / 32768.0, -0.5));
        audio.set_channel_volume(handle, PalVolume::from_raw(2500)).unwrap();
        assert_eq!(audio.slot(handle).unwrap().data.as_ref().unwrap().static_data().frames[0].left, 0.5);
        assert_eq!(audio.channel_frequency(handle).unwrap(), 8000);
        audio.set_channel_frequency(handle, 16000).unwrap();
        assert_eq!(audio.channel_frequency(handle).unwrap(), 16000);
        assert!(audio.set_channel_frequency(handle, -1).is_err());
        assert_eq!(audio.channel_frequency(handle).unwrap(), 16000);
        audio.set_channel_frequency(handle, 0).unwrap();
        assert_eq!(audio.channel_frequency(handle).unwrap(), 8000);
    }
    use super::{
        apply_loop_region, audio_lookup_key, decode_game_audio, decode_static_sound,
        mix_static_sounds, parse_bgm_csv, parse_mix_playlist,
    };
    use kira::sound::{EndPosition, PlaybackPosition};

    #[test]
    fn voice_fade_uses_independent_backend_envelope() {
        use kira::sound::SoundData;
        let data = kira::sound::static_sound::StaticSoundData {
            sample_rate: 1000,
            frames: vec![kira::Frame::new(1.0, 1.0); 3000].into(),
            settings: Default::default(),
            slice: None,
        };
        let prepared = super::prepare_playback(data.clone(), -6.0, 1000);
        assert_eq!(std::time::Duration::from_millis(u64::from(prepared.fade_ms)),
            std::time::Duration::from_secs(1));
        let (mut sound, _handle) = prepared.into_sound().unwrap();
        let builder = kira::info::MockInfoBuilder::new();
        let info = builder.build();
        sound.on_start_processing();
        let initial = sound.process_one(0.001, &info).left.abs();
        let mut middle = initial;
        for _ in 0..499 { middle = sound.process_one(0.001, &info).left.abs(); }
        let mut end = middle;
        for _ in 0..600 { end = sound.process_one(0.001, &info).left.abs(); }
        assert!(initial < middle && middle < end, "{initial} < {middle} < {end}");
        assert!((end - 10.0_f32.powf(-6.0 / 20.0)).abs() < 0.001, "final gain {end}");
        assert_eq!(super::prepare_playback(data.clone(), -6.0, -1).fade_ms, u32::MAX);
        let direct = super::prepare_playback(data, -6.0, 0);
        assert_eq!(direct.fade_ms, 0);
        let (mut direct, _handle) = direct.into_sound().unwrap();
        direct.on_start_processing();
        let _ = direct.process_one(0.001, &info);
        assert!((direct.process_one(0.001, &info).left - end).abs() < 0.001);
    }

    #[test]
    fn directsound_pan_attenuates_only_the_opposite_speaker() {
        for pan in [-10_000, -2_000, -600, 0, 600, 2_000, 10_000] {
            let (position, gain_db) = super::pal_pan_settings(pan);
            let output = (kira::Frame::new(1.0, 1.0) * 10.0_f32.powf(gain_db / 20.0))
                .panned(position.into());
            let expected_quiet = 10.0_f32.powf(-(pan.abs() as f32) / 2000.0);
            let (near, far) = if pan >= 0 { (output.right, output.left) }
                else { (output.left, output.right) };
            assert!((near - 1.0).abs() < 0.0001, "pan={pan}, near={near}");
            assert!((far - expected_quiet).abs() < 0.0001, "pan={pan}, far={far}");
        }
    }

    #[test]
    fn bgm_csv_rows_are_sample_loop_points() {
        let csv = b"//name,intro,loop_start,loop_end\r\n\"BGM01\",0,0,7931200\r\n\"bgm007.ogg\",0,562275,5325075\r\n\"BGM00\",0,0,0\r\n";
        let table = parse_bgm_csv(csv);
        assert_eq!(table["BGM01"].loop_end_samples, 7_931_200);
        assert_eq!(table["BGM007"].loop_start_samples, 562_275);
        assert_eq!(table["BGM007"].loop_end_samples, 5_325_075);
        assert_eq!(table["BGM00"].loop_end_samples, 0);
        assert_eq!(audio_lookup_key("bgm\\BGM01.OGG"), "BGM01");
    }

    #[test]
    fn parses_real_bgm_csv_when_fixture_is_set() {
        let Some(path) = std::env::var_os("SENA_BGM_CSV") else {
            return;
        };
        let bytes = std::fs::read(path).expect("read SENA_BGM_CSV");
        let table = parse_bgm_csv(&bytes);
        assert!(table.len() >= 10, "rows {}", table.len());
        let tracked = table
            .values()
            .filter(|row| row.loop_end_samples > row.loop_start_samples)
            .count();
        assert!(tracked >= 1, "expected at least one loop region");
    }

    #[test]
    fn mix_playlist_lists_voice_names_and_rejects_audio() {
        let mut bytes = vec![0u8; 64];
        bytes[..8].copy_from_slice(b"vo98_005");
        bytes[32..32 + 9].copy_from_slice(b"vo01_1257");
        assert_eq!(
            parse_mix_playlist(&bytes).as_deref(),
            Some(["vo98_005".to_owned(), "vo01_1257".to_owned()].as_slice())
        );
        assert!(parse_mix_playlist(b"OggSrest").is_none());
        assert!(parse_mix_playlist(&[0x30, 0x26, 0xB2, 0x75]).is_none());
    }

    #[test]
    fn mix_playlist_reads_native_wide_records() {
        let mut bytes = vec![0u8; 128];
        for (record, name) in ["speaker01_line", "speaker02_line"].iter().enumerate() {
            for (i, unit) in name.encode_utf16().enumerate() {
                bytes[record * 64 + i * 2..record * 64 + i * 2 + 2]
                    .copy_from_slice(&unit.to_le_bytes());
            }
        }
        assert_eq!(parse_mix_playlist(&bytes), Some(vec![
            "speaker01_line".to_owned(), "speaker02_line".to_owned()]));
        bytes[127] = 1;
        assert!(parse_mix_playlist(&bytes).is_none());
    }

    #[test]
    fn mix_playlist_sums_member_clips_into_one_buffer() {
        let left = pcm16_wav(8_000, &[0, 1000, 0]);
        let right = pcm16_wav(8_000, &[0, 1000]);
        let mut playlist = vec![0u8; 64];
        playlist[..1].copy_from_slice(b"a");
        playlist[32..33].copy_from_slice(b"b");
        let mixed = decode_game_audio(&playlist, &mut |name| {
            Ok(match name {
                "a" => left.clone(),
                "b" => right.clone(),
                other => anyhow::bail!("missing {other}"),
            })
        })
        .expect("mix");
        assert_eq!(mixed.frames.len(), 3);
        assert!(mixed.frames[1].left > mixed.frames[0].left);
        assert!(mixed.frames[2].left.abs() < mixed.frames[1].left.abs());
    }

    #[test]
    fn loop_region_uses_sample_points_only_while_looping() {
        let sound = decode_static_sound(&pcm16_wav(8_000, &[0, 1000, 0, -1000])).expect("wav");
        let once = apply_loop_region(sound.clone(), false, 10, 30);
        assert!(once.settings.loop_region.is_none());
        let ranged = apply_loop_region(sound.clone(), true, 562_275, 5_325_075);
        let region = ranged.settings.loop_region.expect("region");
        assert_eq!(region.start, PlaybackPosition::Samples(562_275));
        assert_eq!(
            region.end,
            EndPosition::Custom(PlaybackPosition::Samples(5_325_075))
        );
        let whole = apply_loop_region(sound, true, 0, 0);
        let region = whole.settings.loop_region.expect("whole");
        assert_eq!(region.end, EndPosition::EndOfAudio);
    }

    #[test]
    fn truncated_asf_is_routed_to_the_wma_decoder() {
        let bytes = [0x30, 0x26, 0xB2, 0x75, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let err = decode_static_sound(&bytes).expect_err("truncated wma");
        let message = format!("{err:#}");
        assert!(
            message.contains("wma"),
            "unexpected decoder error: {message}"
        );
    }

    #[test]
    fn real_mix_decodes_when_game_root_is_set() {
        let Some(root) = std::env::var_os("SENA_GAME_ROOT") else {
            return;
        };
        let Some(mix_name) = std::env::var("SENA_MIX_NAME").ok() else {
            return;
        };
        let mut manager = crate::native_asset::ResourceManager::bootstrap(root, crate::native_asset::Nls::ShiftJis)
            .expect("bootstrap");
        let asset = manager.open(&mix_name).expect("mix");
        let names = parse_mix_playlist(&asset.bytes).expect("playlist");
        assert!(!names.is_empty());
        let mixed = decode_game_audio(&asset.bytes, &mut |member| {
            for ext in ["", ".OGG", ".ogg", ".WAV", ".wav", ".WMA", ".wma"] {
                if let Ok(opened) = manager.open(&format!("{member}{ext}")) {
                    return Ok(opened.bytes);
                }
            }
            anyhow::bail!("missing mix member {member}");
        })
        .expect("decode mix");
        assert!(mixed.frames.len() > 100, "frames {}", mixed.frames.len());
    }

    #[test]
    fn single_mix_member_keeps_its_samples() {
        let clip = pcm16_wav(8_000, &[0, 2000, -2000, 0]);
        let sounds = [decode_static_sound(&clip).expect("wav")];
        let mixed = mix_static_sounds(&sounds).expect("one");
        assert_eq!(mixed.frames.len(), sounds[0].frames.len());
    }

    fn pcm16_wav(rate: u32, samples: &[i16]) -> Vec<u8> {
        let data_len = (samples.len() * 2) as u32;
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_len).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&rate.to_le_bytes());
        out.extend_from_slice(&(rate * 2).to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        for sample in samples {
            out.extend_from_slice(&sample.to_le_bytes());
        }
        out
    }
}

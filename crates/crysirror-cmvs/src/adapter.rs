// SPDX-License-Identifier: MPL-2.0
//! CMVS session ownership and shared host effects.

mod buttons;
mod composite;
mod cvns;
mod dialog;
mod display;
mod effect;
mod host;
mod fade;
mod light;
mod mask;
mod menu;
mod message;
mod movie;
mod quake;
mod raster;
mod retained;
mod scene_effect;
mod sequence;
mod sample;
mod sound;
mod text_debug;
mod thumbnail;
mod visual;
mod visual_opacity;
mod visual_track;

use crate::vm::{
    cpz6_from_pe, latest_slot_range, probe_save_slot,
    scan_latest_slot, CmvsVm, Cpz6Resources, EffectSound, MainVmCheckpoint, PreparedSaveMenu,
    PreparedSystemSave, PreparedTransfer, SceneObject, ScriptSources, SlotHeaderEdit,
    SlotHeaderScratch, SlotMetadata, SlotPreview, VmProfile, VmYield,
};
use anyhow::{bail, ensure, Context, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    scene_checkpoint::{SceneCheckpoint, SceneCheckpointScope},
    service::{
        AssetReader, AudioReplacementJob, AudioReplacementState, AudioRequest,
        AudioSourceMeterSpec, CivilTime, CivilTimeQuery, CivilTimeState, ClientStorage,
        EncodedAudio, HostServices, InputFrame,
    },
    state::ClientId,
    vm::{VmAdapter, VmYield as HostYield},
    Error,
};
use crysirror_resource::{pe::PeData, read_all, DecodeContext, FormatRegistry};
use crysirror_runtime::{model::ClientSpec, runtime::VmFactory, AdapterRegistry};
use crysirror_tasks::{Task, TaskPool};
use serde::Deserialize;
use std::sync::Arc;

const CLEAR_LAYER: u32 = 1;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    profile: String,
    script: String,
    executable: String,
    canvas: [u32; 2],
    #[serde(default)]
    effects: sound::EffectsSettings,
    #[serde(default)]
    music: sound::MusicSettings,
    #[serde(default)]
    voices: sound::VoiceSettings,
    visual_effects: Option<bool>,
    movie_gain: Option<f32>,
    movie_volume: Option<u16>,
}

struct Session {
    storage: Arc<dyn ClientStorage>,
    vm: CmvsVm,
    cpz6: Arc<Cpz6Resources>,
    menu: menu::NativeMenu,
    latest_save_header: SlotHeaderScratch,
    probed_save_header: SlotHeaderScratch,
    save_preview_error: Option<String>,
    thumbnail: thumbnail::Thumbnail,
    boot_clock: u64,
    civil_clock: Option<Box<dyn CivilTimeQuery>>,
    cvns_checkpoint_clock: Option<(Box<dyn CivilTimeQuery>, Option<PreparedTransfer>)>,
    cvns_slot: Option<(Box<[u8;600]>, Vec<u8>)>,
    cvns_load: Option<Box<cvns::restore::Load>>,
    cvns_loaded_before: bool,
    cvns_backup_capture: bool,
    active_draw: bool,
    previous_frame: Option<u64>,
    frame_delta: u32,
    /// CVNS sprite and bitmap scene; unused by the CMVS profiles.
    cvns: cvns::Scene,
    frame_pending: bool,
    syscall_fault: Option<SyscallFault>,
    main_checkpoint: Option<MainCheckpoint>,
    clear_layer: bool,
    effects: sound::Effects,
    music: sound::Music,
    voices: sound::Voices,
    device_sound: sound::DeviceSound,
    samples: sample::Samples,
    audio_meter: AudioSourceMeterSpec,
    movies: movie::Movies,
    pending_audio_switch: Option<(AudioReopen, AudioReplacementJob)>,
    movie_volume: sound::MovieVolume,
    objects: visual::Objects,
    buttons: buttons::Groups,
    messages: message::Messages,
    fade: Option<fade::Fade>,
    masks: [Option<mask::Active>; 2],
    quake: Option<quake::Quake>,
    display: display::Display,
    dialogs: dialog::Dialogs,
    /// `0x2310..=0x2317` object picture stacks.
    composites: composite::Composites,
    /// `0x23a4..=0x23a8` text debug records and their dialog.
    text_debug: text_debug::TextDebug,
    host: host::HostState,
    ducking: sound::Ducking,
    /// Effect records `0x2290..=0x22af` and the engine's `rand()` they draw from.
    fx: effect::Effects,
    special_scenes: [Option<scene_effect::Scene>; 4],
    pending_scene: Option<PendingScene>,
    rand: effect::Rand,
    /// `0x2338` preload: the file name and its worker read, then its bytes.
    prefetch: Option<(String, Prefetch)>,
    /// `0x203f`: the next resource open consults the preload (engine resources +24792).
    use_prefetch: bool,
    /// The mix group is silenced while the window is inactive (0x14002d620, 0x1400a0210).
    muted: Option<bool>,
    /// Last host pointer-activity token and when it changed.
    pointer_token: Option<(u64, u64)>,
    light: light::Light,
    scene_movie_objects: [Option<usize>; 3],
    /// Step-list image loads started by frame playback, applied in order.
    sequence_loads: std::collections::VecDeque<Task<Vec<SequenceImage>>>,
    screen_capture: Option<crysirror_core::image_processing::ImageJob>,
    captured_screen: Option<Arc<dyn crysirror_core::image_processing::ImageResource>>,
    /// `0x222f` timer: duration and elapsed milliseconds.
    screen_timer: Option<[u32; 2]>,
}

/// An image a step-list record replaces once the worker has decoded it.
struct SequenceImage {
    target: SequenceTarget,
    name: String,
    fade: u32,
    image: visual::DecodedImage,
}

#[derive(Clone, Copy)]
enum SequenceTarget {
    Object(usize),
    /// Part of message window 0; negative parts are its root.
    Window(i32),
}

impl From<Vec<SequenceImage>> for Completion {
    fn from(images: Vec<SequenceImage>) -> Self {
        Completion::SequenceImages(images)
    }
}

enum Prefetch {
    Loading(Task<Option<Arc<[u8]>>>),
    Ready(Option<Arc<[u8]>>),
}

struct SyscallFault {
    bits: u32,
    script: String,
    pc: usize,
    opcode: u16,
}

struct MainCheckpoint {
    native: MainVmCheckpoint,
    _scene: Arc<dyn SceneCheckpoint>,
    _objects: visual::Objects,
    _buttons: buttons::Groups,
    _messages: message::Messages,
    _fade: Option<fade::Fade>,
    _masks: [Option<mask::Active>; 2],
    _quake: Option<quake::Quake>,
    _audio: sound::AudioCheckpoint,
    /// Scene movie frame clocks (the CMV objects of 0x14004b6e0).
    _scenes: [Option<movie::ScenePlayer>; 3],
    _effects: [Option<effect::Saved>; 4],
}

impl Session {
    fn release_special_scene(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        index: usize,
    ) -> Result<()> {
        if let Some(scene) = self.special_scenes.get_mut(index) {
            if let Some(scene) = scene {
                for slot in scene.sample_slots() {
                    self.samples.release(host, client, slot)?;
                }
                match scene {
                    scene_effect::Scene::Walk(_) => self.effects.stop(host, client, scene_effect::WALK_STREAM)?,
                    scene_effect::Scene::Mesh(mesh) => mesh.release(host, client, index)?,
                }
            }
            *scene = None;
        }
        Ok(())
    }

    fn poll_special_scene(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<bool> {
        let Some(scene) = self.pending_scene.as_mut() else { return Ok(false) };
        if !scene.sounds.poll(client)? {
            return Ok(true);
        }
        let PendingScene { opcode, index, images, sounds } = self.pending_scene.take().expect("pending special scene");
        self.install_special_scene(host, client, opcode, index, images, Some(sounds))?;
        Ok(true)
    }

    fn install_special_scene(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        opcode: u16,
        index: usize,
        images: SceneImages,
        sounds: Option<sample::PreparedGroup>,
    ) -> Result<()> {
        self.release_special_scene(host, client, index)?;
        self.fx.release(host, client, &mut self.objects, index)?;
        let mut scene = if opcode == 0x2291 {
            scene_effect::Scene::Walk(scene_effect::Walk::start(&mut self.light))
        } else {
            scene_effect::Scene::Mesh(scene_effect::MeshScene::new(
                host, client, opcode, index, images.textures, &mut self.rand, images.door_fallback,
            )?)
        };
        let audio = sounds
            .map(|group| self.samples.replace_group(host, client, group))
            .transpose();
        if let Err(error) = audio {
            if let scene_effect::Scene::Mesh(mesh) = &mut scene {
                mesh.release(host, client, index)?;
            }
            return Err(error);
        }
        match opcode {
            0x2293 => self.light.set_view([0., 3.2, 0.], Some(0.), Some(0.)),
            0x2294 => self.light.set_view([0., 1., 0.], Some(0.), Some(0.)),
            0x2297 => self.light.set_view([0.; 3], Some(0.), Some(0.)),
            _ => {}
        }
        self.special_scenes[index] = Some(scene);
        self.vm.complete_owner_call(scene_effect::words(opcode), None)?;
        Ok(())
    }

    /// Milliseconds since the host last reported pointer movement, when it reports any.
    fn pointer_idle(
        &mut self,
        host: &dyn HostServices,
        client: ClientId,
        now: u64,
    ) -> Result<Option<u64>> {
        let Some(token) = host.pointer_activity(client)? else {
            return Ok(None);
        };
        if self.pointer_token.is_none_or(|(previous, _)| previous != token) {
            self.pointer_token = Some((token, now));
        }
        Ok(self.pointer_token.map(|(_, since)| now.saturating_sub(since)))
    }

    /// Bytes from the `0x2338` preload for the next resource open, when `0x203f` armed it
    /// (0x140048a30 consumes the flag on every open, hit or miss).
    fn take_prefetched(&mut self, name: &str) -> Option<Arc<[u8]>> {
        if !std::mem::take(&mut self.use_prefetch) {
            return None;
        }
        match &self.prefetch {
            Some((cached, Prefetch::Ready(bytes))) if cached.eq_ignore_ascii_case(name) => bytes.clone(),
            _ => None,
        }
    }

    /// Starts object effect `opcode` in record `index`; returns its stack words. The
    /// record keeps its name and words, which a save stores (0x1400525b0).
    #[allow(clippy::too_many_arguments)]
    fn create_effect(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        opcode: u16,
        index: usize,
        name: String,
        parameters: Vec<u32>,
        image: Option<visual::DecodedImage>,
    ) -> Result<u16> {
        self.release_special_scene(host, client, index)?;
        let now = host.now_ms();
        let cameras = self.vm.cameras();
        let saved = effect::Saved { opcode, name, words: parameters.clone() };
        let (fx, objects, rand) = (&mut self.fx, &mut self.objects, &mut self.rand);
        let shape = || anyhow::anyhow!("CMVS {opcode:#x} words");
        let words = match opcode {
            0x22a2 => {
                fx.field(host, client, objects, rand, index, image, parameters.try_into().map_err(|_| shape())?, &cameras, now)?;
                8
            }
            0x22a5 => {
                fx.drift(host, client, objects, rand, index, image, parameters.try_into().map_err(|_| shape())?, now)?;
                8
            }
            0x22a1 | 0x22a4 => {
                fx.streaks(host, client, objects, index, image, opcode == 0x22a4, parameters.try_into().map_err(|_| shape())?, now)?;
                8
            }
            0x22a3 => {
                fx.looped(host, client, objects, rand, index, image, parameters.try_into().map_err(|_| shape())?, now)?;
                6
            }
            0x22a9 => {
                fx.flurry(host, client, objects, rand, index, image, parameters.try_into().map_err(|_| shape())?, now)?;
                6
            }
            _ => {
                fx.fall(host, client, objects, rand, index, image, parameters.try_into().map_err(|_| shape())?, opcode == 0x22a7, now)?;
                6
            }
        };
        fx.remember(index, saved);
        Ok(words)
    }

    /// Applies step-list actions; image records come back as loads for a worker.
    fn apply_sequence(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        actions: Vec<sequence::Action>,
    ) -> Result<Vec<(SequenceTarget, String, u32)>> {
        let mut loads = Vec::new();
        for action in actions {
            match action {
                sequence::Action::Image { slot, name, fade } => loads.push((SequenceTarget::Object(slot), name, fade)),
                sequence::Action::Window { part, name, fade } => {
                    if self.messages.contains(0) {
                        loads.push((SequenceTarget::Window(part), name, fade));
                    }
                }
                sequence::Action::World { slot, mode, duration, target, instant } => {
                    self.objects.sequence_world(host, client, slot, mode, duration, target, instant)?;
                }
                sequence::Action::Camera { mode, duration, target } => {
                    if duration == 0 {
                        self.vm.set_camera_position(target);
                    } else {
                        self.vm.tween_camera(mode, target, duration);
                    }
                }
            }
        }
        Ok(loads)
    }

    fn spawn_sequence_images<T: Send + 'static>(
        &self,
        pool: &TaskPool,
        client: ClientId,
        loads: Vec<(SequenceTarget, String, u32)>,
    ) -> Result<Task<T>>
    where
        T: From<Vec<SequenceImage>>,
    {
        let cpz6 = self.cpz6.clone();
        let storage = self.storage.clone();
        let sources = self.vm.resources().sources(0x2011).cloned().collect();
        let context = DecodeContext::new(client);
        pool.spawn(move |cancel| {
            let sources = ScriptSources::prepare(storage, &cpz6_registry()?, &cpz6, sources)?;
            let mut images = Vec::with_capacity(loads.len());
            for (target, name, fade) in loads {
                cancel.check()?;
                // A missing image leaves the record's target unchanged, as the native loader fails.
                match visual::prepare_image(&sources, &name, &context.clone().with_cancellation(cancel.clone())) {
                    Ok(image) => images.push(SequenceImage { target, name, fade, image }),
                    Err(error) if error.to_string().starts_with("CMVS image missing") => {}
                    Err(error) => return Err(error),
                }
            }
            Ok(T::from(images))
        })
    }

    fn apply_sequence_images(&mut self, host: &mut dyn HostServices, client: ClientId, images: Vec<SequenceImage>) -> Result<()> {
        for SequenceImage { target, name, fade, image } in images {
            match target {
                SequenceTarget::Object(slot) if self.objects.contains(slot) => {
                    self.objects.crossfade_image(host, client, slot, image, fade)?;
                    self.objects.set_source(slot, None, name);
                }
                SequenceTarget::Window(part) => {
                    let slot = message::object_slot(0);
                    if !self.objects.contains(slot) {
                        continue;
                    }
                    match usize::try_from(part) {
                        Ok(part) if self.objects.has_child(slot, part) => {
                            self.objects.child_image(host, client, slot, part, image)?;
                            self.objects.set_source(slot, Some(part), name);
                        }
                        Ok(_) => {}
                        Err(_) => {
                            self.objects.crossfade_image(host, client, slot, image, fade)?;
                            self.objects.set_source(slot, None, name);
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// One frame of step-list playback (0x14002c2c0 runs the list with the frame).
    fn advance_sequence(&mut self, host: &mut dyn HostServices, client: ClientId, pool: &TaskPool, now: u64) -> Result<()> {
        while let Some(task) = self.sequence_loads.front_mut() {
            let Some(images) = task.poll_owner()? else { break };
            self.sequence_loads.pop_front();
            self.apply_sequence_images(host, client, images)?;
        }
        let actions = {
            let objects = &self.objects;
            sequence::run(&mut self.vm, false, now as u32, |slot| slot < 256 && objects.contains(slot))
        };
        let loads = self.apply_sequence(host, client, actions)?;
        if !loads.is_empty() {
            let task = self.spawn_sequence_images(pool, client, loads)?;
            self.sequence_loads.push_back(task);
        }
        Ok(())
    }

    /// Whether the preload worker is still reading.
    fn prefetch_busy(&mut self) -> Result<bool> {
        let Some((_, Prefetch::Loading(task))) = &mut self.prefetch else {
            return Ok(false);
        };
        match task.poll_owner()? {
            Some(bytes) => {
                let name = self.prefetch.take().map(|(name, _)| name).unwrap_or_default();
                self.prefetch = Some((name, Prefetch::Ready(bytes)));
                Ok(false)
            }
            None => Ok(true),
        }
    }

    /// The original stops mixing on deactivation unless ACTIVE_DRAW or `0x22ed` is set.
    fn apply_inactive_mute(&mut self, host: &mut dyn HostServices, client: ClientId, focused: bool) -> Result<()> {
        let background_audio = self.vm.profile() != VmProfile::Cvns20040515X86 && self.display.inactive_sound();
        let muted = !focused && !self.active_draw && !background_audio;
        if self.muted == Some(muted) {
            return Ok(());
        }
        host.submit_audio(
            client,
            AudioRequest::SetMixGroupGain {
                name: sound::MIX_GROUP.into(),
                gain: if muted { 0. } else { 1. },
                fade_ms: 0.,
            },
        )?;
        let configured = if self.vm.profile() == VmProfile::Cvns20040515X86 { self.cvns.movie_gain() } else { self.movie_volume.gain };
        let gain = if muted { 0. } else { configured };
        let media: Vec<_> = self.movies.gain_targets().chain(self.cvns.movies.gain_targets()).map(|id| (id, gain)).collect();
        if !media.is_empty() {
            host.set_playback_gains(client, &[], &media)?;
        }
        self.muted = Some(muted);
        Ok(())
    }

    /// One BGM ducking tick, applied to music slots 0 and 1 when the level changes.
    fn advance_ducking(&mut self, host: &mut dyn HostServices, client: ClientId, now: u64) -> Result<()> {
        let before = self.ducking.factor();
        let voices = self.voices.ducking(host, client, self.ducking.counts_fading())?;
        self.ducking.advance(now, voices);
        let factor = self.ducking.factor();
        if factor != before && self.music.settings.enabled {
            let gain = self.music.gain() * factor;
            let audio = sound::MUSIC_CHANNELS.map(|channel| (channel, gain));
            host.set_playback_gains(client, &audio, &[])?;
        }
        Ok(())
    }

    fn latch_syscall_fault(&mut self, pc: usize, opcode: u16, bit: u32) {
        match &mut self.syscall_fault {
            Some(fault) => fault.bits |= bit,
            None => {
                self.syscall_fault = Some(SyscallFault {
                    bits: bit,
                    script: String::from_utf8_lossy(self.vm.script_name()).into_owned(),
                    pc,
                    opcode,
                });
            }
        }
    }

    fn finish_native_frame(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        canvas: [u32; 2],
        waiting: HostYield,
    ) -> Result<HostYield> {
        self.frame_pending = true;
        if self.vm.profile() == VmProfile::Cvns20040515X86 {
            self.cvns.frame(host, client, canvas)?;
        }
        if let Some(fault) = &self.syscall_fault {
            bail!(
                "CMVS native error flags {:#x} after syscall {:#06x} at {}+{:#x}",
                fault.bits,
                fault.opcode,
                fault.script,
                fault.pc,
            );
        }
        let view = self.light.view();
        let cameras = self.vm.cameras();
        for (index, scene) in self.special_scenes.iter_mut().enumerate() {
            if let Some(scene_effect::Scene::Mesh(mesh)) = scene {
                mesh.draw(host, client, index, canvas, view, &cameras[0], self.frame_delta)?;
            }
        }
        // Native special scenes update in slot order after the draw.
        for scene in self.special_scenes.iter_mut().flatten() {
            use scene_effect::SceneSound;
            match scene.advance(&mut self.light, self.frame_delta) {
                SceneSound::None => {}
                SceneSound::Play { slot, stop_stream } => {
                    self.samples.play(host, client, self.effects.settings, slot, 0, false)?;
                    if stop_stream {
                        self.effects.stop(host, client, scene_effect::WALK_STREAM)?;
                    }
                }
                SceneSound::StopStream => self.effects.stop(host, client, scene_effect::WALK_STREAM)?,
                SceneSound::StopResident(slot) => self.samples.stop(host, client, slot)?,
                SceneSound::Duplicate(first, second) => {
                    self.samples.duplicate(host, client, self.effects.settings, first)?;
                    if let Some(second) = second {
                        self.samples.duplicate(host, client, self.effects.settings, second)?;
                    }
                }
            }
        }
        Ok(waiting)
    }

    fn poll_audio_switch(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<bool> {
        let Some((_, job)) = self.pending_audio_switch.as_mut() else {
            return Ok(false);
        };
        ensure!(job.client() == client, "CMVS foreign audio replacement job");
        match job.poll()? {
            AudioReplacementState::Pending => {}
            AudioReplacementState::Failed(error) => {
                self.pending_audio_switch = None;
                return Err(error.into());
            }
            AudioReplacementState::Ready(resource) => {
                let (operation, _) = self
                    .pending_audio_switch
                    .take()
                    .expect("validated pending switch");
                ensure!(
                    resource.client() == client,
                    "CMVS foreign audio replacement receipt"
                );
                host.replace_audio_channels(client, &operation.scope(), Some(&resource))?;
                let words = if self.vm.profile() == VmProfile::Cvns20040515X86 { 4 } else { 2 };
                self.vm.complete_owner_call(words, None)?;
                match operation {
                    AudioReopen::Music(request) => self.music.commit_reopen(request),
                    AudioReopen::Effects(decoded) => self.effects.set_availability(true, decoded),
                }
            }
        }
        Ok(true)
    }

    fn cvns_volume(&mut self, host: &mut dyn HostServices, client: ClientId, key: u32) -> Result<()> {
        let mut channels = Vec::new();
        let mut media = Vec::new();
        if matches!(key, 10 | 11) {
            let gain = self.cvns.music_gain();
            self.music.cvns_gain = Some(gain);
            channels.extend(sound::MUSIC_CHANNELS.map(|channel| (channel, gain)));
        }
        if matches!(key, 10 | 12) {
            channels.extend(self.effects.cvns_live_gains(self.cvns.effect_levels(), key == 12));
        }
        if matches!(key, 10 | 13) {
            let (_, master, voice) = self.cvns.voice_settings();
            let gain = sound::cvns_gain(master, voice, 0);
            channels.extend((5..9).map(|channel| (channel, gain)));
        }
        if matches!(key, 10 | 14) {
            self.movie_volume.gain = self.cvns.movie_gain();
            let gain = if self.muted == Some(true) { 0. } else { self.movie_volume.gain };
            media.extend(self.movies.gain_targets().chain(self.cvns.movies.gain_targets()).map(|id| (id, gain)));
        }
        host.set_playback_gains(client, &channels, &media)?;
        Ok(())
    }

    fn volume_command(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        opcode: u16,
    ) -> Result<()> {
        let [mode, value] = self.vm.owner_arguments(opcode)?;
        let levels = sound::Levels {
            music: self.music.settings,
            effects: self.effects.settings,
            voices: self.voices.settings,
            movie: self.movie_volume,
        };
        if mode == 0 {
            self.vm.complete_owner_call(2, Some(levels.read(opcode)))?;
            return Ok(());
        }
        let next = levels.updated(opcode, value);
        let mut audio = next.live_channels(opcode, &self.effects);
        for (channel, gain) in &mut audio {
            if sound::MUSIC_CHANNELS.contains(channel) {
                *gain *= self.ducking.factor();
            }
        }
        let media = if matches!(opcode, 0x22d9 | 0x22dd) {
            self.movies
                .gain_targets()
                .map(|id| (id, next.movie.gain))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        if !audio.is_empty() || !media.is_empty() {
            host.set_playback_gains(client, &audio, &media)?;
        }
        self.vm.complete_owner_call(2, None)?;
        self.music.settings = next.music;
        self.effects.settings = next.effects;
        self.voices.settings = next.voices;
        self.movie_volume = next.movie;
        Ok(())
    }

    fn poll_cvns_checkpoint(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
    ) -> Result<bool> {
        let Some((query, _)) = self.cvns_checkpoint_clock.as_mut() else {
            return Ok(false);
        };
        let sample = match query.poll()? {
            CivilTimeState::Pending => return Ok(true),
            CivilTimeState::Failed(why) => anyhow::bail!("CVNS checkpoint clock: {why}"),
            CivilTimeState::Ready(sample) => sample,
        };
        ensure!(sample.client == client, "CVNS foreign checkpoint clock");
        let (_, main) = self.cvns_checkpoint_clock.take().unwrap();
        let (scene, audio, records) = self.capture_scene(host, client)?;
        let mut native = if let Some(main) = main {
            self.vm.complete_main_script(main, records)?
        } else {
            self.vm.complete_scene_checkpoint(records)?
        };
        self.cvns_slot = Some((
            self.vm.cvns_header(sample),
            std::mem::take(&mut native.cvns_records),
        ));
        self.retain_checkpoint(native, scene, audio);
        Ok(false)
    }

    fn poll_civil_clock(&mut self, client: ClientId) -> Result<bool> {
        let Some(query) = self.civil_clock.as_mut() else {
            return Ok(false);
        };
        ensure!(query.client() == client, "CMVS foreign civil clock query");
        let sample = match query.poll()? {
            CivilTimeState::Pending => return Ok(true),
            CivilTimeState::Ready(sample) => sample,
            CivilTimeState::Failed(message) => anyhow::bail!("CMVS civil clock: {message}"),
        };
        ensure!(sample.client == client, "CMVS foreign civil clock receipt");
        self.vm
            .complete_owner_results(0, &civil_registers(sample)?)?;
        self.civil_clock = None;
        Ok(false)
    }

    /// Advances a pending screen capture and retains its finished frame.
    fn poll_screen_capture(&mut self) -> Result<()> {
        use crysirror_core::image_processing::ImageProcessState;
        let Some(job) = &mut self.screen_capture else {
            return Ok(());
        };
        match job.poll()? {
            ImageProcessState::Pending => {}
            ImageProcessState::Ready(frame) => {
                self.captured_screen = Some(frame);
                self.screen_capture = None;
            }
            ImageProcessState::Failed(why) => anyhow::bail!("CMVS screen capture: {why}"),
        }
        Ok(())
    }

    /// Replaces the live game with a loaded save slot.
    fn restore_slot(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        canvas: [u32; 2],
        loaded: LoadedSlot,
    ) -> Result<()> {
        let LoadedSlot { snapshot, scripts, images, movies, music, effects, seconds, mattes } = loaded;
        self.stop_outputs(host, client)?;
        self.effects.reset_records();

        for mask in &mut self.masks {
            mask::Active::remove(mask, &mut self.objects, host, client, false)?;
        }
        fade::Fade::remove(&mut self.fade, host, client)?;
        self.fx.clear(host, client, &mut self.objects)?;
        self.sequence_loads.clear();
        self.composites.clear();
        self.vm
            .restore_checkpoint(&snapshot["vm"], scripts)
            .context("CMVS save checkpoint is damaged")?;
        self.vm.sequence_mut().rebase(host.now_ms() as u32);
        let cameras = self.vm.cameras();
        self.objects.restore(host, client, &snapshot["objects"], images, cameras)?;
        self.scene_movie_objects = [None; 3];
        for movie in movies {
            // A save from before frame clocks were kept replays its movie from the start.
            let saved = serde_json::from_value::<Option<movie::ScenePlayer>>(snapshot["scenes"][movie.slot].clone());
            let mut player = match saved.ok().flatten() {
                Some(player) => player,
                None => {
                    let mut head = [0; 36];
                    movie.source.read_at(0, &mut head)?;
                    let [frames, rate, scale] = movie::timing(&head);
                    let mut player = movie::ScenePlayer::new(frames, rate, scale, 0);
                    player.play(0, 0, -1, true, host.now_ms());
                    player
                }
            };
            player.rebase(host.now_ms());
            let path = self.movies.load_scene(host, client, movie.slot, movie.source, player)?;
            self.objects.resume_movie(
                host, client, movie.object, movie.slot, movie.name, path, movie.size, movie.alpha,
            )?;
            self.scene_movie_objects[movie.slot] = Some(movie.object);
        }
        for (slot, name, matte) in seconds {
            if self.objects.contains(slot) {
                self.objects.set_second(host, client, slot, name, matte)?;
            }
        }
        for (index, record, image) in effects {
            self.create_effect(host, client, record.opcode, index, record.name, record.words, image)?;
        }
        self.messages.restore(&snapshot["messages"], host, client, &self.objects, canvas)?;
        for (field, name, matte) in mattes {
            if self.messages.has_field(field) {
                self.messages.set_field_matte(host, client, field, name, matte, &self.objects, canvas)?;
            }
        }
        self.light = serde_json::from_value(snapshot["light"].clone()).unwrap_or_default();
        self.buttons = serde_json::from_value(snapshot["buttons"].clone())?;
        if let Some((request, first, second)) = music.filter(|_| self.music.settings.enabled) {
            self.music.load(host, client, request, first, second)?;
        }
        self.quake = None;
        self.screen_timer = None;
        self.main_checkpoint = None;
        self.cvns_slot = None;
        self.thumbnail.clear();
        self.previous_frame = None;
        self.frame_pending = true;
        Ok(())
    }

    fn stop_outputs(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<()> {
        self.pending_scene = None;
        for index in 0..self.special_scenes.len() {
            self.release_special_scene(host, client, index)?;
        }
        self.movies.stop(host, client)?;
        for slot in 0..3 {
            self.movies.remove_scene(host, client, slot)?;
        }
        self.music.stop(host, client)?;
        for channel in 5..9 {
            self.voices.stop(host, client, channel, 0)?;
        }
        for channel in 0..self.effects.count() {
            self.effects.stop(host, client, channel)?;
        }
        self.device_sound.stop(host, client)?;
        self.samples.clear(host, client)?;
        Ok(())
    }

    fn reset(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        prepared: PreparedTransfer,
    ) -> Result<()> {
        self.vm.validate_reset(&prepared)?;
        self.stop_outputs(host, client)?;
        self.effects.reset_records();
        if self.vm.profile() == VmProfile::Cvns20040515X86 {
            self.cvns.reset_scene(host, client)?;
            self.thumbnail.clear();
            self.main_checkpoint = None;
        self.cvns_slot = None;
            self.previous_frame = None;
            self.frame_pending = true;
            self.vm.complete_reset(prepared)?;
            return Ok(());
        }
        for mask in &mut self.masks {
            mask::Active::remove(mask, &mut self.objects, host, client, false)?;
        }
        fade::Fade::remove(&mut self.fade, host, client)?;
        // 0x14002b200 also releases the effect records and their objects.
        self.fx.clear(host, client, &mut self.objects)?;
        self.sequence_loads.clear();
        self.composites.clear();
        self.objects.reset_all(host, client)?;
        self.messages.reset(host, client)?;
        if self.clear_layer {
            let layer =
                serde_json::json!({"op":"set_solid_color","id":CLEAR_LAYER,"color":[0,0,0,255]});
            host.submit_engine(
                client,
                1,
                &serde_json::to_vec(&serde_json::json!({"layers":[layer]}))?,
            )?;
        }
        if let Some(slots) = self.vm.save_menu() {
            self.menu
                .publish(
                    host,
                    client,
                    slots,
                    self.vm.menu_options().1,
                    &self.display.menu_checks(),
                )?;
        }
        self.buttons = buttons::Groups::default();
        self.light = Default::default();
        self.quake = None;
        self.main_checkpoint = None;
        self.cvns_slot = None;
        self.save_preview_error = None;
        self.thumbnail.clear();
        self.previous_frame = None;
        self.frame_pending = true;
        self.vm.complete_reset(prepared)?;
        Ok(())
    }

    fn capture_scene(
        &self,
        host: &mut dyn HostServices,
        client: ClientId,
    ) -> Result<(Arc<dyn SceneCheckpoint>, sound::AudioCheckpoint, Vec<u8>)> {
        ensure!(
            !self.movies.live(),
            "CMVS scene checkpoint has a live movie"
        );
        let audio = sound::AudioCheckpoint::capture(
            host,
            client,
            &self.music,
            &self.effects,
            &self.voices,
        )?;
        let mut scope = SceneCheckpointScope::default();
        if self.clear_layer {
            scope.layer_roots.push(CLEAR_LAYER);
        }
        self.objects.capture(&mut scope);
        self.messages.capture(&mut scope);
        if self.fade.is_some() {
            scope.layer_roots.push(fade::LAYER);
        }
        let scene = host.capture_scene_checkpoint(client, &scope)?;
        let mut records = Vec::new();
        if self.vm.profile() == VmProfile::Cvns20040515X86 {
            self.cvns.checkpoint_records(&mut records)?;
            self.effects.csp_record(&mut records)?;
            self.music.csp_record(&mut records)?;
        }
        Ok((scene, audio, records))
    }

    fn retain_checkpoint(
        &mut self,
        native: MainVmCheckpoint,
        scene: Arc<dyn SceneCheckpoint>,
        audio: sound::AudioCheckpoint,
    ) {
        self.main_checkpoint = Some(MainCheckpoint {
            native,
            _scene: scene,
            _objects: self.objects.clone(),
            _buttons: self.buttons.clone(),
            _messages: self.messages.clone(),
            _fade: self.fade,
            _masks: self.masks.clone(),
            _quake: self.quake.clone(),
            _audio: audio,
            _scenes: self.movies.players().clone(),
            _effects: self.fx.saved(),
        });
    }
}

/// Everything a save restore needs, prepared away from the VM owner.
struct LoadedSlot {
    snapshot: serde_json::Value,
    scripts: crate::vm::SavedScripts,
    images: Vec<(usize, Option<usize>, String, visual::DecodedImage)>,
    movies: Vec<SavedMovie>,
    music: Option<(sound::MusicRequest, Vec<u8>, Option<Vec<u8>>)>,
    /// Effect records and their images, started again after the objects.
    effects: Vec<(usize, effect::Saved, Option<visual::DecodedImage>)>,
    /// Second textures of objects and text field mattes, as screen-sized mattes.
    seconds: Vec<(usize, String, Option<visual::DecodedImage>)>,
    mattes: Vec<(usize, String, Option<visual::DecodedImage>)>,
}

struct SavedMovie {
    object: usize,
    slot: usize,
    name: String,
    source: Arc<dyn crysirror_core::service::AssetReader>,
    size: [u32; 2],
    alpha: bool,
}

/// Local-independent timestamp in the native slot label layout.
fn slot_stamp() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64);
    #[cfg(target_arch = "wasm32")]
    let seconds = 0i64;
    let days = seconds.div_euclid(86_400) + 719_468;
    let era = days.div_euclid(146_097);
    let day = days.rem_euclid(146_097);
    let year_of_era = (day - day / 1460 + day / 36_524 - day / 146_096) / 365;
    let day_of_year = day - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted = (5 * day_of_year + 2) / 153;
    let date = day_of_year - (153 * shifted + 2) / 5 + 1;
    let month = if shifted < 10 { shifted + 3 } else { shifted - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    let clock = seconds.rem_euclid(86_400);
    format!(
        "{year:04}-{month:02}-{date:02} {:02}:{:02}:{:02} ",
        clock / 3600,
        clock / 60 % 60,
        clock % 60
    )
}

type SceneSample = (usize, &'static str, Option<Arc<dyn AssetReader>>);

fn read_scene_image(
    sources: &ScriptSources,
    name: &str,
    context: &DecodeContext,
) -> Result<Option<visual::DecodedImage>> {
    if name.is_empty() {
        return Ok(None);
    }
    match visual::prepare_image(sources, name, context) {
        Ok(image) => Ok(Some(image)),
        Err(error) if error.to_string().starts_with("CMVS image missing") => Ok(None),
        Err(error) => Err(error),
    }
}

struct PendingScene {
    opcode: u16,
    index: usize,
    images: SceneImages,
    sounds: sample::PreparedGroup,
}

#[derive(Default)]
struct SceneImages {
    textures: Vec<Option<visual::DecodedImage>>,
    door_fallback: [bool; 2],
}

enum Completion {
    Boot(Box<Session>),
    /// CVNS `0x21c0` found no `save000.dat` and leaves the banks unchanged.
    CvnsSystemData(Option<crate::vm::css0::SystemData>),
    CvnsSaveMenu(crate::vm::SaveMenu),
    CvnsLoaded(Box<cvns::restore::Load>),
    CvnsSlotMissing,
    CvnsWritten(Option<crate::vm::SaveMenu>, bool),
    /// CVNS `0x2051`/`0x2130` bitmap slot, operand words and decoded image.
    CvnsBitmap(cvns::bitmap::Prepared),
    CvnsMaskSheet(Option<cvns::mask_fonts::Sheet>),
    CvnsRegistry(cvns::registry::Reply),
    CvnsRule(Option<cvns::rule::Active>, Option<cvns::bitmap::Prepared>),
    ChildImage(usize, usize, String, visual::DecodedImage),
    SystemSaved,
    /// `cmvs.cfg` written under the save root after a settings change.
    ConfigSaved,
    Cursor(u32, Option<crysirror_image::cursor::CursorImage>),
    SecondTexture(usize, String, Option<visual::DecodedImage>),
    /// Object effect image (`0x22a1..=0x22a7`, `0x22a9`) and its parameter words.
    EffectImage(u16, usize, String, Vec<u32>, Option<visual::DecodedImage>),
    VariantImage(usize, String, u16, u32, visual::DecodedImage),
    /// `0x23a4` read the `.dvg` file.
    TextDebugLoaded(Vec<text_debug::Record>),
    /// `0x23a6` wrote the `.dvg` file after the dialog closed.
    TextDebugSaved,
    /// `0x2310` layer picture: slot, layer, position.
    CompositeLayer(usize, usize, [i32; 2], Option<visual::DecodedImage>),
    /// `0x23c0` image font sheet and its cell size.
    ImageFontSheet(u32, Option<visual::DecodedImage>),
    /// `0x2397` field matte.
    FieldMatte(usize, String, Option<visual::DecodedImage>),
    /// `0x228c..=0x228e` changed slot files; the menu rescan follows for slots below ten.
    SlotFiles(u16, Option<PreparedSaveMenu>),
    /// `0x2284` flush images; the owner call completes once they are applied.
    SequenceImages(Vec<SequenceImage>),
    SlotSaved,
    SlotLoaded(std::result::Result<Box<LoadedSlot>, String>),
    /// `0x2308`/`0x230e`: slot, object, name, source, picture size, alpha, the first frame
    /// and the header timing, and the stack words.
    SceneMovie(usize, usize, String, Arc<dyn crysirror_core::service::AssetReader>, [u32; 2], bool, movie::ScenePlayer, u16),
    SaveMenu(PreparedSaveMenu),
    LatestSaveSlot(u32, SlotHeaderScratch),
    ProbeSaveSlot(u32, SlotHeaderScratch),
    EditSaveSlot { words: u16, edited: bool },
    CvnsSlotMetadata(Option<SlotMetadata>, Option<cvns::bitmap::Prepared>),
    SaveSlotMetadata(Option<SlotMetadata>, Option<SavePreviewReplacement>),
    SystemSave(PreparedSystemSave),
    Script(PreparedTransfer),
    Icon(crysirror_image::ImageSurface),
    EffectAudio(Option<Vec<u8>>),
    Sample(usize, String, Option<Arc<dyn AssetReader>>),
    SceneAssets {
        opcode: u16,
        index: usize,
        images: SceneImages,
        audio: Vec<SceneSample>,
    },
    SceneReload(usize, Vec<Option<visual::DecodedImage>>),
    DoorRules(usize, [Option<visual::DecodedImage>; 2]),
    Music(sound::MusicRequest, Vec<u8>, Option<Vec<u8>>),
    AudioSwitch(AudioReopen, Vec<AudioRequest>),
    Voice(sound::VoiceRequest, Option<Vec<u8>>),
    DeviceSound(sound::DeviceSoundRequest, Option<Vec<u8>>),
    MovieSource(Option<Arc<dyn AssetReader>>),
    CvnsSceneMovie(usize, Option<cvns::movie::Clip>),
    ObjectImage(visual::DecodedImage),
    /// A mask rule image; None when it is missing, which the original reports as 0.
    MaskImage(mask::Request, Option<visual::DecodedImage>),
    WindowImage(usize, String, visual::DecodedImage),
}

struct SavePreviewReplacement {
    object: usize,
    child: Option<usize>,
    image: Result<visual::DecodedImage>,
}

enum AudioReopen {
    Music(sound::MusicRequest),
    Effects(Vec<bool>),
}

impl AudioReopen {
    fn scope(&self) -> Vec<u32> {
        match self {
            Self::Music(_) => sound::MUSIC_CHANNELS.to_vec(),
            Self::Effects(decoded) => (0..decoded.len()).map(sound::device_channel).collect(),
        }
    }
}

struct Adapter {
    client: ClientId,
    profile: VmProfile,
    entry: VfsPath,
    executable: VfsPath,
    canvas: [u32; 2],
    effects_settings: sound::EffectsSettings,
    music_settings: sound::MusicSettings,
    voice_settings: sound::VoiceSettings,
    visual_effects: bool,
    movie_volume: sound::MovieVolume,
    pool: Option<TaskPool>,
    job: Option<Task<Completion>>,
    session: Option<Box<Session>>,
}

impl Adapter {
    fn begin_boot(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let profile = self.profile;
        let storage = host.storage(self.client)?;
        let client = self.client;
        let entry = self.entry.clone();
        let executable = self.executable.clone();
        let canvas = self.canvas;
        let effects_settings = self.effects_settings;
        let music_settings = self.music_settings;
        let mut voice_settings = self.voice_settings;
        let movie_volume = self.movie_volume;
        let pool = TaskPool::new("cmvs-storage", 1, 2)?;
        self.job = Some(pool.spawn(move |cancel| {
            let script = storage.open_asset(&entry)?;
            let mut vm = CmvsVm::prepare(profile, script.as_ref())?;
            vm.configure_canvas(canvas);
            let key_config = VfsPath::new(Mount::Asset, "key.cfg")?;
            if let Some(path) =
                crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(
                    storage.as_ref(),
                    &key_config,
                )?
            {
                let source = storage.open_asset(&path)?;
                vm.configure_input(&read_all(source.as_ref(), 64 * 1024)?)?;
            }
            let mut messages = message::Messages::default();
            let mut active_draw = false;
            let mut audio_meter = sound::meter_spec(0, 0);
            let config_name = if profile == VmProfile::Cvns20040515X86 { "cvns.cfg" } else { "cmvs.cfg" };
            let config = VfsPath::new(Mount::Asset, config_name)?;
            // Settings written back by this player take precedence over the installed file.
            let overlay = VfsPath::new(Mount::Save, config_name)?;
            let saved_config = match storage.read(&overlay) {
                Ok(bytes) => Some(bytes),
                Err(Error::NotFound) => None,
                Err(error) => return Err(error.into()),
            };
            let installed_config = if let Some(path) =
                crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(
                    storage.as_ref(),
                    &config,
                )?
            {
                let source = storage.open_asset(&path)?;
                Some(read_all(source.as_ref(), 256 * 1024)?)
            } else {
                None
            };
            let display_config = saved_config.or(installed_config).unwrap_or_default();
            if !display_config.is_empty() {
                let (text, _) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(&display_config);
                active_draw = ini_integer(&text, "[SYSTEM_INFO]", "ACTIVE_DRAW") != 0;
                audio_meter = sound::meter_spec(
                    ini_integer(&text, "[SOUND_INFO]", "SOUND_QUALITY"),
                    ini_integer(&text, "[SOUND_INFO]", "SOUND_BUFFER") as u32,
                );
                messages.configure(&display_config);
            }
            let mut display = display::Display::new(profile, canvas, &display_config);
            messages.set_alpha_revision(display.alpha_revision());
            cancel.check()?;
            let executable = storage.open_asset(&executable)?;
            let bytes = read_all(executable.as_ref(), 16 * 1024 * 1024)?;
            let pe = PeData::parse(&bytes)?;
            let mut menu = menu::NativeMenu::prepare(profile, &pe, canvas)?;
            let cpz6 = Arc::new(cpz6_from_pe(profile, DecodeContext::new(client), &pe)?);
            cancel.check()?;
            let mut cvns = cvns::Scene::default();
            let mut music = sound::Music::new(music_settings);
            let mut effects_settings = effects_settings;
            let cvns_levels = (profile == VmProfile::Cvns20040515X86).then(|| {
                cvns.configure(&display_config);
                for key in 1..=31 {
                    let changes = if (7..=9).contains(&key) {
                        vec![(33010 + key - 7, true, cvns.setting(key) != Some(0))]
                    } else { cvns.menu_setting(key) };
                    menu.update_cvns_setting(&changes);
                }
                let fullscreen = display.cvns_mode() != 0;
                menu.update_cvns_setting(&[(32743, fullscreen, !fullscreen), (32744, !fullscreen, fullscreen)]);

                display.set_cvns_cursor_delay(cvns.setting(29).unwrap_or(1));
                effects_settings.enabled = cvns.setting(8) != Some(0);
                music.settings.enabled = cvns.setting(7) != Some(0);
                voice_settings.enabled = cvns.setting(9) != Some(0);
                music.cvns_gain = Some(cvns.music_gain());
                cvns.effect_levels()
            });
            Ok(Completion::Boot(Box::new(Session {
                storage,
                vm,
                cpz6,
                menu,
                latest_save_header: [0; 64],
                probed_save_header: [0; 64],
                save_preview_error: None,
                thumbnail: thumbnail::Thumbnail::new(client),
                boot_clock: 0,
                civil_clock: None,
                cvns_checkpoint_clock: None,
                cvns_slot: None,
                cvns_load: None,
                cvns_loaded_before: false,
                cvns_backup_capture: false,
                active_draw,
                previous_frame: None,
                frame_delta: 0,
                cvns,
                frame_pending: true,
                syscall_fault: None,
                main_checkpoint: None,
                clear_layer: false,
                effects: sound::Effects::new(effects_settings, cvns_levels),
                music,
                voices: sound::Voices::new(voice_settings),
                device_sound: sound::DeviceSound::default(),
                samples: sample::Samples::default(),
                audio_meter,
                movies: movie::Movies::default(),
                pending_audio_switch: None,
                movie_volume,
                objects: visual::Objects::new(canvas),
                buttons: buttons::Groups::default(),
                messages,
                fade: None,
                masks: Default::default(),
                quake: None,
                display,
                dialogs: Default::default(),
                text_debug: Default::default(),
                composites: Default::default(),
                host: Default::default(),
                ducking: Default::default(),
                fx: Default::default(),
                special_scenes: Default::default(),
                pending_scene: None,
                rand: effect::Rand::seeded(0),
                prefetch: None,
                use_prefetch: false,
                muted: None,
                pointer_token: None,
                light: Default::default(),
                scene_movie_objects: [None; 3],
                sequence_loads: Default::default(),
                screen_capture: None,
                captured_screen: None,
                screen_timer: None,
            })))
        })?);
        self.pool = Some(pool);
        Ok(())
    }

    fn publish(&mut self, host: &mut dyn HostServices, completion: Completion) -> Result<()> {
        match completion {
            Completion::Boot(mut session) => {
                session.boot_clock = host.now_ms();
                session.cvns.seed(session.boot_clock as u32);
                if self.profile == VmProfile::Cvns20040515X86 {
                    session.display.start_cvns(host, self.client)?;
                }
                // The engine seeds rand() from the clock at startup (srand in 0x14002c7e0).
                session.rand = effect::Rand::seeded(session.boot_clock as u32);
                session.messages.check_initial_face(host, self.client)?;
                self.session = Some(session);
            }
            completion => {
                let session = self.session.as_mut().context("CMVS session missing")?;
                match completion {
                    Completion::CvnsSlotMissing => {
                        let wake = HostYield::WaitUntil(host.now_ms());
                        session.finish_native_frame(host, self.client, self.canvas, wake)?;
                    }
                    Completion::CvnsWritten(menu, backup) => {
                        if backup {
                            session.menu.update_cvns_setting(&[(161, true, false)]);
                        }
                        if let Some(menu) = menu {
                            session.menu.update_cvns_slots(&menu)?;
                            let (visible, enabled) = session.vm.menu_options();
                            if visible {
                                session
                                    .menu
                                    .publish(host, self.client, &menu, enabled, &[])?;
                            }
                            session.vm.complete_cvns_menu(menu)?;
                        } else {
                            session.vm.complete_owner_call(1, None)?;
                        }
                    }
                    Completion::CvnsLoaded(mut load) => {
                        let gain = if session.muted == Some(true) {
                            0.
                        } else {
                            session.cvns.movie_gain()
                        };
                        if let Err(error) = load.stage(&session.cvns, host, self.client, gain) {
                            load.discard(host, self.client)?;
                            return Err(error);
                        }
                        session.cvns_load = Some(load);
                    }
                    Completion::CvnsSaveMenu(slots) => {
                        session.menu.update_cvns_slots(&slots)?;
                        let (visible, enabled) = session.vm.menu_options();
                        if visible {
                            session.menu.publish(host, self.client, &slots, enabled, &[])?;
                        }
                        session.vm.complete_cvns_menu(slots)?;
                    }
                    Completion::SaveMenu(prepared) => {
                        let (visible, enabled) = session.vm.menu_options();
                        if visible {
                            session
                                .menu
                                .publish(
                                host,
                                self.client,
                                prepared.menu(),
                                enabled,
                                &session.display.menu_checks(),
                            )?;
                        }
                        session.vm.complete_save_menu(prepared)?;
                    }
                    Completion::LatestSaveSlot(slot, header) => {
                        session.vm.complete_owner_call(2, Some(slot))?;
                        session.latest_save_header = header;
                    }
                    Completion::ProbeSaveSlot(found, header) => {
                        session.vm.complete_owner_call(1, Some(found))?;
                        session.probed_save_header = header;
                    }
                    Completion::EditSaveSlot { words, edited } => {
                        session
                            .vm
                            .complete_owner_call(words, Some(u32::from(edited)))?;
                    }
                    Completion::CvnsSlotMetadata(metadata, bitmap) => {
                        session.vm.complete_slot_metadata(metadata.as_ref(), || {
                            if let Some(bitmap) = bitmap { session.cvns.install_bitmap(host, self.client, bitmap)?; }
                            Ok(())
                        })?;
                    }
                    Completion::SaveSlotMetadata(metadata, replacement) => {
                        let mut failure = None;
                        session.vm.complete_slot_metadata(metadata.as_ref(), || {
                            if let Some(SavePreviewReplacement {
                                object,
                                child,
                                image,
                            }) = replacement
                            {
                                let image = match image {
                                    Ok(image) => Some(image),
                                    Err(error) => {
                                        failure = Some(format!("{error:#}"));
                                        None
                                    }
                                };
                                session.objects.replace_preview(
                                    host,
                                    self.client,
                                    object,
                                    child,
                                    image,
                                )?;
                            }
                            Ok(())
                        })?;
                        session.save_preview_error = failure;
                    }
                    Completion::CvnsSystemData(data) => session.vm.complete_cvns_system_load(data)?,
                    Completion::CvnsRule(mask, captured) => {
                        session.cvns.commit_rule(host, self.client, &mut session.vm, mask, captured)?
                    }
                    Completion::CvnsRegistry(reply) => reply.complete(&mut session.vm)?,
                    Completion::CvnsMaskSheet(sheet) => {
                        if let Some(sheet) = sheet { session.cvns.masks.sheet(sheet)?; }
                        session.vm.complete_owner_call(2, None)?;
                    }
                    Completion::CvnsBitmap(bitmap) => {
                        session.cvns.commit_bitmap(host, self.client, &mut session.vm, bitmap)?
                    }
                    Completion::SystemSave(prepared) => {
                        session.vm.complete_system_save(prepared)?
                    }
                    Completion::Script(prepared) => {
                        if prepared.is_reset() {
                            session.reset(host, self.client, prepared)?;
                        } else if prepared.is_main() && session.vm.profile()==VmProfile::Cvns20040515X86 {
                            session.cvns_checkpoint_clock=Some((host.begin_civil_time(self.client)?,Some(prepared)));
                        } else if prepared.is_main() {
                            let (scene, audio, records) = session.capture_scene(host, self.client)?;
                            let native = session.vm.complete_main_script(prepared, records)?;
                            session.retain_checkpoint(native, scene, audio);
                        } else {
                            session.vm.complete_common_script(prepared)?;
                        }
                    }
                    Completion::Icon(icon) => {
                        host.request_window_icon(
                            self.client,
                            [icon.width, icon.height],
                            icon.rgba,
                        )?;
                        session.vm.complete_icon()?;
                    }
                    Completion::EffectAudio(bytes) => {
                        session
                            .effects
                            .load(host, self.client, &mut session.vm, bytes)?
                    }
                    Completion::Sample(slot, name, reader) => {
                        session.samples.load(
                            host,
                            self.client,
                            &mut session.vm,
                            slot,
                            &name,
                            reader,
                        )?;
                    }
                    Completion::SceneAssets { opcode, index, images, audio } => {
                        if audio.is_empty() {
                            session.install_special_scene(host, self.client, opcode, index, images, None)?;
                        } else {
                            let sounds = sample::PreparedGroup::begin(host, self.client, audio)?;
                            session.pending_scene = Some(PendingScene { opcode, index, images, sounds });
                        }
                    }
                    Completion::SceneReload(index, images) => {
                        let Some(scene_effect::Scene::Mesh(mesh)) = session.special_scenes[index].as_mut() else {
                            bail!("CMVS scene was released during texture replacement");
                        };
                        mesh.replace_images(host, self.client, index, images)?;
                        session.vm.complete_owner_call(4, None)?;
                    }
                    Completion::DoorRules(index, images) => {
                        let Some(scene_effect::Scene::Mesh(mesh)) = session.special_scenes[index].as_mut() else {
                            bail!("CMVS panels were released during rule loading");
                        };
                        mesh.load_door_rules(host, self.client, index, images)?;
                    }
                    Completion::Music(request, first, second) => {
                        session
                            .music
                            .load(host, self.client, request, first, second)?;
                    }
                    Completion::AudioSwitch(operation, recipes) => {
                        let job = host.begin_audio_replacement(self.client, recipes)?;
                        session.pending_audio_switch = Some((operation, job));
                    }
                    Completion::Voice(request, bytes) => {
                        session.voices.play(
                            host,
                            self.client,
                            &mut session.vm,
                            request,
                            bytes,
                            session.audio_meter,
                        )?;
                    }
                    Completion::DeviceSound(request, bytes) => {
                        session.device_sound.load(
                            host,
                            self.client,
                            &mut session.vm,
                            request,
                            bytes,
                            session.audio_meter,
                        )?;
                    }
                    Completion::CvnsSceneMovie(index, clip) => {
                        if let Some(clip) = clip {
                            let gain = if session.muted == Some(true) { 0. } else { session.cvns.movie_gain() };
                            session.cvns.movies.load(host, self.client, index, clip, gain)?;
                        } else {
                            session.vm.complete_owner_call(2, Some(0))?;
                        }
                    }
                    Completion::MovieSource(source) => {
                        session.movies.begin(
                            host,
                            self.client,
                            &mut session.vm,
                            source,
                            if session.muted == Some(true) { 0. } else if self.profile == VmProfile::Cvns20040515X86 { session.cvns.movie_gain() } else { session.movie_volume.gain },
                            (self.profile == VmProfile::Cvns20040515X86).then(|| session.cvns.setting(31).unwrap_or(0)),
                        )?;
                    }
                    Completion::SceneMovie(index, object, name, source, size, alpha, player, words) => {
                        let path = session.movies.load_scene(host, self.client, index, source, player)?;
                        session.objects.movie(host, self.client, object, path, size, alpha)?;
                        session.objects.set_movie_source(object, index, name);
                        session.scene_movie_objects[index] = Some(object);
                        session.vm.complete_owner_call(words, None)?;
                    }
                    Completion::SystemSaved => session.vm.complete_owner_call(0, None)?,
                    Completion::ConfigSaved => {}
                    Completion::EffectImage(opcode, index, name, parameters, image) => {
                        let words = session.create_effect(host, self.client, opcode, index, name, parameters, image)?;
                        session.vm.complete_owner_call(words, None)?;
                    }
                    Completion::TextDebugLoaded(records) => {
                        session.text_debug.append(records);
                        session.vm.complete_owner_call(0, None)?;
                    }
                    Completion::TextDebugSaved => {}
                    Completion::CompositeLayer(slot, layer, position, image) => {
                        session.composites.place(slot, layer, image.as_ref(), position);
                        session.vm.complete_owner_call(5, None)?;
                    }
                    Completion::ImageFontSheet(cell, image) => {
                        session.messages.add_image_font_sheet(image.as_ref(), cell);
                        session.vm.complete_owner_call(2, None)?;
                    }
                    Completion::FieldMatte(index, name, matte) => {
                        session.messages.set_field_matte(host, self.client, index, name, matte, &session.objects, self.canvas)?;
                        session.vm.complete_owner_call(2, None)?;
                    }
                    Completion::SecondTexture(slot, name, matte) => {
                        let loaded = matte.is_some();
                        session.objects.set_second(host, self.client, slot, name, matte)?;
                        session.vm.complete_owner_call(4, Some(u32::from(loaded)))?;
                    }
                    Completion::Cursor(index, image) => {
                        session.host.select_cursor(host, self.client, index, image)?
                    }
                    Completion::SlotSaved => session.vm.complete_owner_call(1, None)?,
                    Completion::VariantImage(slot, name, words, fade, image) => {
                        session.objects.crossfade_image(host, self.client, slot, image, fade)?;
                        session.objects.set_source(slot, None, name);
                        session.vm.complete_owner_call(words, Some(1))?;
                    }
                    Completion::SlotFiles(words, menu) => {
                        if let Some(menu) = menu {
                            if session.vm.refresh_save_menu(menu) && session.menu.published() {
                                let slots = session.vm.save_menu().context("CMVS menu has not been loaded")?;
                                let checks = session.display.menu_checks();
                                session.menu.publish(host, self.client, slots, session.vm.menu_options().1, &checks)?;
                            }
                        }
                        session.vm.complete_owner_call(words, None)?;
                    }
                    Completion::SequenceImages(images) => {
                        session.apply_sequence_images(host, self.client, images)?;
                        session.vm.sequence_mut().reset();
                        session.vm.complete_owner_call(0, None)?;
                    }
                    Completion::SlotLoaded(Ok(loaded)) => {
                        session.restore_slot(host, self.client, self.canvas, *loaded)?;
                    }
                    Completion::SlotLoaded(Err(why)) => {
                        session.save_preview_error = Some(why);
                        session.vm.complete_owner_call(1, None)?;
                    }
                    Completion::ChildImage(slot, child, name, image) => {
                        session.objects.child_image(host, self.client, slot, child, image)?;
                        session.objects.set_source(slot, Some(child), name);
                        session.vm.complete_owner_call(3, Some(1))?;
                    }
                    Completion::ObjectImage(image) => {
                        let (SceneObject::Load { slot, name }, _) =
                            session.vm.pending_scene_object()?
                        else {
                            bail!("stale CMVS image preparation");
                        };
                        session.objects.image(host, self.client, slot, image)?;
                        session.objects.set_source(slot, None, name);
                        session.vm.complete_scene_object(Some(true))?;
                    }
                    Completion::MaskImage(request, image) => {
                        let channel = request.channel;
                        let words = request.words;
                        if let Some(image) = image {
                            mask::Active::begin(
                                &mut session.masks[channel],
                                &mut session.objects,
                                host,
                                self.client,
                                request,
                                image,
                            )?;
                            session.vm.complete_owner_call(words, Some(1))?;
                        } else {
                            // 0x140088200 deletes the captured object 0 again when the rule fails.
                            if request.screen {
                                session.objects.remove_object(host, self.client, 0)?;
                            }
                            session.vm.complete_owner_call(words, Some(0))?;
                        }
                    }
                    Completion::WindowImage(index, name, image) => {
                        let [slot, _] = session.vm.owner_arguments(0x2170)?;
                        ensure!(
                            slot as usize == index && session.messages.contains(index),
                            "stale CMVS window image"
                        );
                        session.objects.image(
                            host,
                            self.client,
                            message::object_slot(index),
                            image,
                        )?;
                        session.objects.set_source(message::object_slot(index), None, name);
                        session.vm.complete_owner_call(2, None)?;
                    }
                    Completion::Boot(_) => unreachable!(),
                }
            }
        }
        Ok(())
    }

    fn advance(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        budget: u32,
    ) -> Result<HostYield> {
        let waiting = HostYield::WaitUntil(host.now_ms().saturating_add(1));
        let running = if let Some(session) = &mut self.session {
            let focused = host.window_focused(self.client)? != Some(false);
            session.apply_inactive_mute(host, self.client, focused)?;
            let running = session.active_draw || focused;
            if running {
                session.vm.admit_input(input);
                session.cvns.admit_input(input);
            } else {
                session.previous_frame = None;
                session.frame_delta = 0;
                session.vm.suspend_frame();
            }
            running
        } else {
            true
        };
        if let Some(job) = &mut self.job {
            if let Some(result) = job.poll_owner()? {
                self.job = None;
                self.publish(host, result)?;
            }
            return Ok(waiting);
        }
        let Some(session) = &mut self.session else {
            self.begin_boot(host)?;
            return Ok(waiting);
        };
        if session.poll_cvns_checkpoint(host,self.client)? || session.poll_civil_clock(self.client)? {
            return Ok(waiting);
        }
        if let Some(mut load) = session.cvns_load.take() {
            let gain = if session.muted == Some(true) {
                0.
            } else {
                session.cvns.movie_gain()
            };
            match load.ready(host, self.client, gain) {
                Ok(true) => {
                    load.commit(session, self.canvas, host, self.client)?;
                    return session.finish_native_frame(host, self.client, self.canvas, waiting);
                }
                Ok(false) => session.cvns_load = Some(load),
                Err(error) => {
                    load.discard(host, self.client)?;
                    return Err(error);
                }
            }
            return Ok(waiting);
        }
        if session.poll_audio_switch(host, self.client)? {
            return Ok(waiting);
        }
        let pool = self.pool.as_ref().context("CMVS worker pool missing")?;
        if let Some((index, names)) = session.special_scenes.iter_mut().enumerate().find_map(|(index, scene)| {
            let Some(scene_effect::Scene::Mesh(mesh)) = scene else { return None };
            mesh.door_rule_request().map(|names| (index, names))
        }) {
            let storage = session.storage.clone();
            let cpz6 = session.cpz6.clone();
            let roots = session.vm.resources().sources(0x2011).cloned().collect();
            let context = DecodeContext::new(self.client);
            self.job = Some(pool.spawn(move |cancel| {
                let context = context.with_cancellation(cancel.clone());
                let sources = ScriptSources::prepare(storage, &cpz6_registry()?, &cpz6, roots)?;
                let mut images = [None, None];
                for (image, name) in images.iter_mut().zip(names) {
                    cancel.check()?;
                    *image = read_scene_image(&sources, name, &context)?;
                }
                Ok(Completion::DoorRules(index, images))
            })?);
            return Ok(waiting);
        }
        if session.cvns.movies.poll(host, self.client, &mut session.vm)? {
            return Ok(waiting);
        }
        if session.thumbnail.poll(&mut session.vm)? {
            return Ok(waiting);
        }
        if session.effects.poll(host, self.client, &mut session.vm)? {
            return Ok(waiting);
        }
        if session.music.poll(host, self.client, &mut session.vm)? {
            return Ok(waiting);
        }
        if session.voices.poll(host, self.client, &mut session.vm)? {
            return Ok(waiting);
        }
        if session
            .device_sound
            .poll(host, self.client, &mut session.vm)?
        {
            return Ok(waiting);
        }
        if session.samples.poll(self.client, &mut session.vm)? {
            return Ok(waiting);
        }
        if session.poll_special_scene(host, self.client)? {
            return Ok(waiting);
        }
        if session
            .movies
            .poll(host, self.client, &mut session.vm, self.canvas)?
        {
            return Ok(waiting);
        }
        if self.profile == VmProfile::Cvns20040515X86 && session.movies.live() {
            // 0x402770 pumps the movie and input, and runs no script until exit.
            if session.cvns.consume_movie_skip() {
                session.movies.stop(host, self.client)?;
            } else if session.movies.frame(host, self.client)? != u32::MAX {
                return Ok(waiting);
            }
            session.previous_frame = None;
        }
        if !running {
            return Ok(waiting);
        }
        if session.frame_pending {
            if !session.cvns.finish_screen_copy(host, self.client)? { return Ok(waiting); }
            let now = host.now_ms();
            if let Some(due) = session.display.frame_due(now) {
                return Ok(HostYield::WaitUntil(due));
            }
            let idle = session.pointer_idle(host, self.client, now)?;
            session.display.advance_cursor(host, self.client, idle)?;
            session.advance_ducking(host, self.client, now)?;
            let cameras = session.vm.cameras();
            let ground = session.objects.collision_points();
            session.fx.advance(host, self.client, &mut session.objects, &mut session.rand, &cameras, &ground, now)?;
            session.advance_sequence(host, self.client, pool, now)?;
            session.movies.advance_scenes(host, self.client, now)?;
            let frame = session.previous_frame.map_or(0, |previous| now.saturating_sub(previous)) as u32;
            session.objects.advance_rasters(host, self.client, frame)?;
            let delta = session
                .previous_frame
                .map_or(0, |previous| now.saturating_sub(previous));
            session.frame_delta = delta as u32;
            let previous_quake = session.objects.window_offset();
            fade::Fade::advance(&mut session.fade, host, self.client, delta as u32)?;
            for mask in &mut session.masks {
                mask::Active::advance(mask, &mut session.objects, host, self.client, delta as u32)?;
            }
            if session
                .quake
                .as_mut()
                .is_some_and(|quake| quake.advance((delta as u32).min(200)))
            {
                session.quake = None;
            }
            session
                .objects
                .set_quake(host, self.client, quake::offsets(&session.quake))?;
            session.vm.advance_camera_tween(delta as u32);
            session.light.advance(delta as u32);
            let cameras = session.vm.cameras();
            session.objects.set_cameras(host, self.client, cameras)?;
            if let Some([duration, elapsed]) = &mut session.screen_timer {
                *elapsed = elapsed.saturating_add((delta as u32).min(200)).min(*duration);
            }
            session
                .objects
                .advance_crossfades(host, self.client, delta as u32)?;
            session
                .objects
                .advance_opacity(host, self.client, delta as u32)?;
            session.messages.advance(
                host,
                self.client,
                &mut session.objects,
                delta as u32,
                self.canvas,
                previous_quake != quake::offsets(&session.quake)[1],
            )?;
            session.messages.advance_fields(
                host,
                self.client,
                &session.objects,
                delta as u32,
                self.canvas,
            )?;
            session.vm.begin_frame(delta as u32)?;
            if self.profile == VmProfile::Cvns20040515X86 {
                session.cvns.begin_frame(now as u32, session.display.frame_period());
            }
            session.previous_frame = Some(now);
            session.frame_pending = false;
        }
        let mut remaining = budget;
        loop {
            let yielded = session.vm.run_slice(&mut remaining).with_context(|| {
                format!("execute CMVS family script {}", String::from_utf8_lossy(session.vm.script_name()))
            })?;
            let step = match yielded {
                VmYield::BudgetExhausted => return Ok(HostYield::BudgetExhausted),
                VmYield::Frame => return session.finish_native_frame(host, self.client, self.canvas, waiting),
                VmYield::SystemCall { pc, opcode } => (pc, opcode),
            };
            session.vm.set_owner_alias(None);
            // CVNS selectors without a CMVS counterpart pend as 0x2800 + selector.
            let step = match crate::vm::cvns_selector(step.1) {
                Some(0x21ed) => {
                    let [key, name, raw, _] = session.vm.owner_arguments(step.1)?;
                    let face = session.vm.text_reference(name).context("CVNS setting string")?;
                    let before = session.cvns.setting(key);
                    session.cvns.write_setting(key, raw, face);
                    let changed = before != session.cvns.setting(key);
                    if (1..=31).contains(&key) && key != 6 && (changed || key == 15) {
                        session.cvns.settings_dirty = true;
                    }
                    if key == 2 {
                        session.cvns.reset_clock(session.cvns.setting(2) != Some(0), host.now_ms() as u32);
                    }
                    if key == 30 { session.display.cvns_auto_move(raw != 0); }
                    if key == 29 {
                        session.display.set_cvns_cursor_delay(session.cvns.setting(29).unwrap_or(1));
                    }
                    if key == 5 {
                        session.active_draw = raw != 0;
                        let focused = host.window_focused(self.client)? != Some(false);
                        session.apply_inactive_mute(host, self.client, focused)?;
                    }
                    if (10..=14).contains(&key) {
                        session.cvns_volume(host, self.client, key)?;
                    }
                    if !matches!(key, 6..=9) || changed {
                        let changes = session.cvns.menu_setting(key);
                        if !changes.is_empty() {
                            session.menu.update_cvns_setting(&changes);
                            if session.menu.published() {
                                let slots = session.vm.save_menu().context("CVNS slot menu missing")?;
                                session.menu.publish(host, self.client, slots, session.vm.menu_options().1, &[])?;
                            }
                        }
                    }
                    if matches!(key, 7..=9) && changed {
                        step
                    } else {
                        session.vm.complete_owner_call(4, None)?;
                        continue;
                    }
                }
                Some(0x2007) => {
                    session.vm.complete_owner_call(0, Some(session.display.cvns_last_frames()))?;
                    continue;
                }
                Some(0x2002) => {
                    session.display.cvns_set_frame_period(&mut session.vm)?;
                    continue;
                }
                Some(0x217c) => {
                    let [mode] = session.vm.owner_arguments(step.1)?;
                    if mode as i32 > 3 {
                        session.latch_syscall_fault(step.0, 0x217c, 8);
                    } else {
                        let fullscreen = mode != 0;
                        session.display.cvns_set_mode(host, self.client, fullscreen)?;
                        session.menu.update_cvns_setting(&[(32743, fullscreen, !fullscreen), (32744, !fullscreen, fullscreen)]);
                        if session.menu.published() {
                            let slots = session.vm.save_menu().context("CVNS slot menu missing")?;
                            session.menu.publish(host, self.client, slots, session.vm.menu_options().1, &[])?;
                        }
                        session.cvns.settings_dirty = true;
                    }
                    session.vm.finish_owner_call(1, None, true)?;
                    return session.finish_native_frame(host, self.client, self.canvas, waiting);
                }
                Some(0x2182) => {
                    let state = if host.window_maximized(self.client)? == Some(true) { 1 }
                        else if host.window_minimized(self.client)? == Some(true) { 2 } else { 0 };
                    session.vm.complete_owner_call(0, Some(state))?;
                    continue;
                }
                Some(0x2180) => {
                    let [x, y] = session.display.cvns_position(host, self.client)?;
                    session.vm.complete_owner_results(0, &[(1, x as u32), (2, y as u32)])?;
                    continue;
                }
                Some(0x2181) => {
                    let [x, y] = session.vm.owner_arguments(step.1)?;
                    session.display.cvns_move(host, self.client, [x as i32, y as i32])?;
                    session.cvns.settings_dirty = true;
                    session.vm.finish_owner_call(2, None, true)?;
                    return session.finish_native_frame(host, self.client, self.canvas, waiting);
                }
                Some(0x2003) => {
                    session.vm.complete_owner_call(0, Some(session.display.frame_period()))?;
                    continue;
                }
                Some(0x21a3) => {
                    let [slot] = session.vm.owner_arguments(step.1)?;
                    if slot as i32 <= 0 {
                        session.vm.complete_owner_call(1, None)?;
                        continue;
                    }
                    let backup_enabled = slot < 950 && session.vm.menu_options().1;
                    if backup_enabled
                        && session.cvns_loaded_before
                        && !session.cvns_backup_capture
                        && session.vm.settings().thumbnail_size.iter().all(|&n| n != 0)
                    {
                        session.cvns_backup_capture = true;
                        session
                            .thumbnail
                            .begin_backup(host, pool, &mut session.vm)?;
                        return Ok(waiting);
                    }
                    session.cvns_backup_capture = false;
                    let backup = if backup_enabled {
                        cvns::slot::Write::capture(session, 998)?
                    } else {
                        None
                    };
                    if slot < 950 {
                        session
                            .menu
                            .update_cvns_setting(&[(160, backup_enabled, false)]);
                    }
                    let seed = session.vm.cvns_restore_seed();
                    let resources = session.vm.resources();
                    let paths = |op| resources.sources(op).cloned().collect::<Vec<_>>();
                    let (images, sounds, bgm) = (paths(0x2011), paths(0x2012), paths(0x2013));
                    let root = resources.movie_root().cloned();
                    let base = resources.save_file_base().map(|(p, dir)| (p.clone(), dir));
                    let storage = session.storage.clone();
                    let cpz = session.cpz6.clone();
                    let client = self.client;
                    let random = session.cvns.restore_random();
                    let generation = session.cvns.masks.generation();
                    let effects = sound::Effects::new(
                        session.effects.settings,
                        Some(session.cvns.effect_levels()),
                    );
                    let mut music = sound::Music::new(session.music.settings);
                    music.cvns_gain = Some(session.cvns.music_gain());
                    self.job = Some(pool.spawn(move |cancel| {
                        use crate::vm::save_slot::{SlotFormat, unpack_native};
                        let base = base.as_ref().map(|(p, dir)| (p, *dir));
                        if let Some(backup) = backup {
                            backup.write(storage.as_ref(), || Ok(cancel.check()?))?;
                        }
                        let Some((_, reader)) =
                            crate::vm::open_save_slot(storage.as_ref(), base, slot)?
                        else {
                            return Ok(Completion::CvnsSlotMissing);
                        };
                        let mut saved = unpack_native(SlotFormat::Csp1, reader.as_ref(), || {
                            Ok(cancel.check()?)
                        })?;
                        let thumbnail = saved.thumbnail.take();
                        let vm = seed.prepare(saved)?;
                        let registry = cpz6_registry()?;
                        let sources =
                            |list| ScriptSources::prepare(storage.clone(), &registry, &cpz, list);
                        let load = cvns::restore::Load::prepare(
                            vm,
                            thumbnail,
                            &sources(images)?,
                            &sources(sounds)?,
                            &sources(bgm)?,
                            storage.as_ref(),
                            root.as_ref(),
                            client,
                            &cancel,
                            random,
                            generation,
                            effects,
                            music,
                        )?;
                        Ok(Completion::CvnsLoaded(Box::new(load)))
                    })?);
                    return Ok(waiting);
                }
                Some(0x21a4) => {
                    let [slot] = session.vm.owner_arguments(step.1)?;
                    if slot as i32 <= 0 {
                        session.vm.complete_owner_call(1, None)?;
                        continue;
                    }
                    let Some(write) = cvns::slot::Write::capture(session, slot)? else {
                        session.vm.complete_owner_call(1, None)?;
                        continue;
                    };
                    let base = session
                        .vm
                        .resources()
                        .save_file_base()
                        .map(|(path, dir)| (path.clone(), dir));
                    let count = session.vm.save_menu().map(|m| m.slots.len() as u32);
                    let storage = session.storage.clone();
                    self.job = Some(pool.spawn(move |cancel| {
                        let check = || Ok(cancel.check()?);
                        let backup = write.write(storage.as_ref(), check)?;
                        let base = base.as_ref().map(|(path, dir)| (path, *dir));
                        let menu = count
                            .filter(|_| slot <= 256)
                            .map(|n| crate::vm::scan_cvns_menu(storage.as_ref(), base, n, check))
                            .transpose()?;
                        Ok(Completion::CvnsWritten(menu, backup))
                    })?);
                    return Ok(waiting);
                }
                Some(selector @ (0x21c0 | 0x21c1)) => {
                    let root = session.vm.resources().save_suffix().context("CVNS save directory is not registered")?;
                    let path = VfsPath::new(Mount::Save, &format!("{}/save000.dat", root.relative().trim_end_matches('/')))?;
                    let storage = session.storage.clone();
                    if selector == 0x21c0 {
                        let suffix = session.vm.cvns_save_suffix()?;
                        self.job = Some(pool.spawn(move |cancel| {
                            cancel.check()?;
                            let data = match storage.open_read(&path) {
                                Err(Error::NotFound) => None,
                                result => {
                                    let bytes = read_all(result?.as_ref(), 2 * 1024 * 1024)?;
                                    Some(crate::vm::css0::SystemData::decode(bytes, suffix)?)
                                }
                            };
                            Ok(Completion::CvnsSystemData(data))
                        })?);
                    } else {
                        let data = session.vm.cvns_system_data()?;
                        let salts = session.cvns.save_salts();
                        self.job = Some(pool.spawn(move |cancel| {
                            let bytes = data.encode(salts, || Ok(cancel.check()?))?;
                            cancel.check()?;
                            storage.write_atomic(&path, &bytes)?;
                            Ok(Completion::SystemSaved)
                        })?);
                    }
                    return Ok(waiting);
                }
                Some(0x20b8 | 0x21b1 | 0x21b2 | 0x21b6 | 0x21b7 | 0x21ab | 0x21ac) => step,
                Some(selector @ (0x21a7 | 0x21ae)) => {
                    let base = session.vm.resources().save_file_base().map(|(path, dir)| (path.clone(), dir));
                    let storage = session.storage.clone();
                    let first = session.vm.owner_argument(step.1, 4)?;
                    let latest = selector == 0x21a7;
                    let count = if latest { session.vm.owner_argument(step.1, 8)? } else { 1 };
                    let scratch = if latest { session.latest_save_header } else { session.probed_save_header };
                    self.job = Some(pool.spawn(move |cancel| {
                        let base = base.as_ref().map(|(path, dir)| (path, *dir));
                        let check = || Ok(cancel.check()?);
                        if latest {
                            let (slot, scratch) = crate::vm::latest_cvns_slot(storage.as_ref(), base, first, count, scratch, check)?;
                            Ok(Completion::LatestSaveSlot(slot, scratch))
                        } else {
                            let (found, scratch) = crate::vm::probe_cvns_slot(storage.as_ref(), base, first, scratch, check)?;
                            Ok(Completion::ProbeSaveSlot(found, scratch))
                        }
                    })?);
                    return Ok(waiting);
                }
                Some(0x21a1) => {
                    let [count] = session.vm.owner_arguments(step.1)?;
                    let base = session.vm.resources().save_file_base().map(|(path, dir)| (path.clone(), dir));
                    let storage = session.storage.clone();
                    self.job = Some(pool.spawn(move |cancel| {
                        let base = base.as_ref().map(|(path, dir)| (path, *dir));
                        Ok(Completion::CvnsSaveMenu(crate::vm::scan_cvns_menu(
                            storage.as_ref(), base, count, || Ok(cancel.check()?),
                        )?))
                    })?);
                    return Ok(waiting);
                }
                Some(selector @ (0x20af | 0x20b7 | 0x20bd | 0x20be)) => {
                    let [slot, word] = session.vm.owner_arguments(step.1)?;
                    let voice = matches!(selector, 0x20bd | 0x20be);
                    let slots = if voice { 4 } else { 48 };
                    if slot >= slots {
                        session.latch_syscall_fault(step.0, selector, 0x8000);
                    } else if voice {
                        host.update_audio_pcm16_gain(self.client, slot + 5, sound::cvns_fader_update(selector, word))?;
                    } else {
                        session.effects.set_fader(host, self.client, slot as usize, selector, word)?;
                    }
                    session.vm.complete_owner_call(2, None)?;
                    continue;
                }
                Some(selector @ 0x20b9..=0x20bb) => {
                    let [slot] = session.vm.owner_arguments(step.1)?;
                    let words = if selector == 0x20ba { 1 } else { 2 };
                    if slot >= 4 {
                        session.latch_syscall_fault(step.0, selector, 0x8000);
                    } else {
                        match selector {
                            0x20b9 => {
                                let [_, looping] = session.vm.owner_arguments(step.1)?;
                                session.voices.start(host, self.client, slot, looping != 0)?;
                            }
                            0x20ba => session.voices.stop(host, self.client, slot + 5, 0)?,
                            _ => {
                                let [_, duration] = session.vm.owner_arguments(step.1)?;
                                session.voices.fade_cvns(host, self.client, slot, duration)?;
                            }
                        }
                    }
                    session.vm.complete_owner_call(words, None)?;
                    continue;
                }
                Some(selector @ (0x20b6 | 0x20bc | 0x20c6)) => {
                    let [slot] = session.vm.owner_arguments(step.1)?;
                    let voice = selector == 0x20bc;
                    let slots = if selector == 0x20b6 { 48 } else { 4 };
                    if slot >= slots {
                        session.latch_syscall_fault(step.0, selector, 0x8000);
                        session.vm.complete_owner_call(1, None)?;
                    } else {
                        let position = if voice {
                            sound::position(host, self.client, slot + 5)?
                        } else {
                            let offset = if selector == 0x20c6 { 48 } else { 0 };
                            session.effects.position(host, self.client, offset + slot as usize)?
                        };
                        let elapsed = if position == u32::MAX {
                            position
                        } else {
                            position.saturating_sub(session.cvns.audio_lead_ms)
                        };
                        session.vm.complete_owner_call(1, Some(elapsed))?;
                    }
                    continue;
                }
                Some(0x20a4) => {
                    let position = session.music.elapsed(host, self.client, host.now_ms())?;
                    let result = if session.music.settings.enabled && position != u32::MAX {
                        position.saturating_sub(session.cvns.audio_lead_ms)
                    } else { position };
                    session.vm.complete_owner_call(0, Some(result))?;
                    continue;
                }
                Some(0x20c0) => (step.0, 0x20c0),
                Some(0x20c5) => {
                    let [slot] = session.vm.owner_arguments(step.1)?;
                    if slot >= 4 {
                        session.latch_syscall_fault(step.0, 0x20c5, 0x8000);
                        session.vm.complete_owner_call(6, None)?;
                        continue;
                    }
                    step
                }
                Some(0x20c7) => {
                    let [slot] = session.vm.owner_arguments(step.1)?;
                    if slot >= 4 {
                        session.latch_syscall_fault(step.0, 0x20c7, 0x8000);
                    } else {
                        session.effects.stop(host, self.client, slot as usize + 48)?;
                    }
                    session.vm.complete_owner_call(1, None)?;
                    continue;
                }
                Some(0x20eb) => {
                    // 0x42b530 always zeroes the two vector results and sets bit 0.
                    let _: [u32; 4] = session.vm.owner_arguments(step.1)?;
                    session.latch_syscall_fault(step.0, 0x20eb, 1);
                    session.vm.complete_owner_results(4, &[(1, 0), (2, 0)])?;
                    continue;
                }
                Some(0x2041) => {
                    let _: [u32; 1] = session.vm.owner_arguments(step.1)?;
                    session.latch_syscall_fault(step.0, 0x2041, 1);
                    session.vm.complete_owner_call(1, None)?;
                    continue;
                }
                Some(0x217f) => {
                    let [width, height] = host.display_info().context("CVNS display dimensions unavailable")?.screen;
                    session.vm.complete_owner_results(0, &[(1, width), (2, height)])?;
                    continue;
                }
                Some(0x217d) => {
                    session.vm.complete_owner_results(0, &[(1, self.canvas[0]), (2, self.canvas[1])])?;
                    continue;
                }
                Some(0x2005) => (step.0, 0x2003),
                Some(0x2010) => (step.0, 0x2080),
                Some(0x2012) => (step.0, 0x2081),
                Some(selector) => {
                    match session.cvns.command(selector, self.canvas, session.muted == Some(true), &mut session.vm, host, self.client)? {
                        Some(cvns::Step::Done) => continue,
                        Some(cvns::Step::Fault { words, flags }) => {
                            session.latch_syscall_fault(step.0, selector, flags);
                            session.vm.complete_owner_call(words, None)?;
                            continue;
                        }
                        Some(cvns::Step::Frame) => {
                            return session.finish_native_frame(host, self.client, self.canvas, waiting);
                        }
                        Some(cvns::Step::Pending) => return Ok(waiting),
                        Some(cvns::Step::Rule(mut request)) => {
                            let cpz6 = session.cpz6.clone();
                            let sources = session.vm.resources().sources(0x2011).cloned().collect();
                            let client = self.client;
                            let storage = session.storage.clone();
                            self.job = Some(pool.spawn(move |cancel| {
                                let context = DecodeContext::new(client).with_cancellation(cancel.clone());
                                let sources = ScriptSources::prepare(storage, &cpz6_registry()?, &cpz6, sources)?;
                                let mask = if sources.open_entry(&request.name)?.is_some() {
                                    let image = visual::prepare_image(&sources, &request.name, &context)?;
                                    Some(cvns::rule::Active::new(&request, image))
                                } else { None };
                                let captured = request.screen.take().map(|image| cvns::bitmap::Job {
                                    slot: cvns::BitmapSlot::Global(0), words: 3, picture: Default::default(),
                                    edit: cvns::bitmap::Edit::Capture(image),
                                }.run(client, |_, _| unreachable!())).transpose()?;
                                cancel.check()?;
                                Ok(Completion::CvnsRule(mask, captured))
                            })?);
                            return Ok(waiting);
                        }
                        Some(cvns::Step::Movie(index, path, name)) => {
                            let storage = session.storage.clone();
                            self.job = Some(pool.spawn(move |cancel| {
                                cancel.check()?;
                                let path = crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(storage.as_ref(), &path)?;
                                let clip = path.map(|p| cvns::movie::Clip::read(storage.open_asset(&p)?, name)).transpose()?;
                                cancel.check()?;
                                Ok(Completion::CvnsSceneMovie(index, clip))
                            })?);
                            return Ok(waiting);
                        }
                        Some(cvns::Step::Registry(request)) => {
                            let storage = session.storage.clone();
                            self.job = Some(pool.spawn(move |cancel| {
                                cancel.check()?;
                                Ok(Completion::CvnsRegistry(request.run(storage.as_ref())?))
                            })?);
                            return Ok(waiting);
                        }
                        Some(cvns::Step::MaskSheet(name, size)) => {
                            let cpz6 = session.cpz6.clone();
                            let sources = session.vm.resources().sources(0x2011).cloned().collect();
                            let client = self.client;
                            let storage = session.storage.clone();
                            self.job = Some(pool.spawn(move |cancel| {
                                let sources = ScriptSources::prepare(storage, &cpz6_registry()?, &cpz6, sources)?;
                                let sheet = cvns::mask_fonts::load(&sources, &name, size,
                                    &DecodeContext::new(client).with_cancellation(cancel.clone()))?;
                                cancel.check()?;
                                Ok(Completion::CvnsMaskSheet(sheet))
                            })?);
                            return Ok(waiting);
                        }
                        Some(cvns::Step::Bitmap(job)) => {
                            let cpz6 = session.cpz6.clone();
                            let sources = session.vm.resources().sources(0x2011).cloned().collect();
                            let context = DecodeContext::new(self.client);
                            let storage = session.storage.clone();
                            self.job = Some(pool.spawn(move |cancel| {
                                cancel.check()?;
                                let bitmap = job.run(context.client, |name, frame| {
                                    let mut context = context.with_cancellation(cancel.clone());
                                    context.parameters.insert("purple-software/pb2-frame".into(), frame.into());
                                    let sources = ScriptSources::prepare(storage, &cpz6_registry()?, &cpz6, sources)?;
                                    visual::prepare_image(&sources, name, &context)
                                })?;
                                cancel.check()?;
                                Ok(Completion::CvnsBitmap(bitmap))
                            })?);
                            return Ok(waiting);
                        }
                        None => {}
                    }
                    bail!(
                    "unsupported CVNS syscall 0x{selector:04x} at {}+0x{:x}",
                    String::from_utf8_lossy(session.vm.script_name()),
                    step.0,
                    )
                }
                None => step,
            };
            session.messages.set_alias_words(match step.1 {
                0x2380 => Some(4),
                0x2381 => Some(6),
                _ => None,
            });
            if matches!(step.1, 0x2380..=0x2382) {
                let [index] = session.vm.owner_arguments(step.1)?;
                session.messages.reset_field(index as usize);
            }
            let step = match retained::aliased_command(step.1) {
                Some((command, target)) => {
                    let [index] = session.vm.owner_arguments(step.1)?;
                    let slot = match target {
                        retained::AliasTarget::Window if index < 8 => {
                            message::object_slot(index as usize) as u32
                        }
                        retained::AliasTarget::Field if index < 8 => index + 12,
                        retained::AliasTarget::List if index < 8 => index + 6,
                        _ => u32::MAX,
                    };
                    session.vm.set_owner_alias(Some((command, slot)));
                    (step.0, command)
                }
                None => step,
            };
            if self.profile == VmProfile::Cvns20040515X86 {
                let effect_call = match step.1 {
                    0x20b0 => Some((0x20b0, 5)),
                    0x20ba => Some((0x20ad, 6)),
                    0x20b1 => Some((0x20b1, 2)),
                    0x20b2 => Some((0x20b2, 1)),
                    0x20b3 => Some((0x20b3, 1)),
                    0x20b4 => Some((0x20ae, 2)),
                    0x20b6 => Some((0x20b5, 2)),
                    _ => None,
                };
                if let Some((selector, words)) = effect_call {
                    let [slot] = session.vm.owner_arguments(step.1)?;
                    if slot >= 48 {
                        session.latch_syscall_fault(step.0, selector, 0x8000);
                        session.vm.complete_owner_call(words, None)?;
                        continue;
                    }
                }
            }
            let storage = session.storage.clone();
            match step.1 {
                0x2007 => {
                    let millis = host.now_ms().saturating_sub(session.boot_clock) as u32;
                    session
                        .vm
                        .complete_owner_results(0, &[(0, millis / 1000)])?;
                }
                0x200d => {
                    let query = host.begin_civil_time(self.client)?;
                    ensure!(
                        query.client() == self.client,
                        "CMVS foreign civil clock query"
                    );
                    session.civil_clock = Some(query);
                    return Ok(waiting);
                }
                0x2001 => {
                    if session.cvns.settings_dirty {
                        let bytes = session.display.cvns_config_bytes(&session.cvns);
                        let path = VfsPath::new(Mount::Save, "cvns.cfg")?;
                        self.job = Some(pool.spawn(move |cancel| {
                            cancel.check()?;
                            storage.write_atomic(&path, &bytes)?;
                            Ok(Completion::ConfigSaved)
                        })?);
                        session.cvns.settings_dirty = false;
                        return Ok(waiting);
                    }
                    for mask in &mut session.masks {
                        mask::Active::remove(mask, &mut session.objects, host, self.client, false)?;
                    }
                    session.stop_outputs(host, self.client)?;
                    session.vm.complete_quit()?;
                    return Ok(HostYield::Halted);
                }
                0x22b3 => {
                    let request = session
                        .vm
                        .pending_save_menu()?
                        .context("CMVS menu request missing")?;
                    self.job = Some(pool.spawn(move |cancel| {
                        cancel.check()?;
                        let menu = request.prepare(storage.as_ref())?;
                        cancel.check()?;
                        Ok(Completion::SaveMenu(menu))
                    })?);
                }
                0x22b2 => {
                    if session.vm.profile()==VmProfile::Cvns20040515X86 {
                        session.cvns_checkpoint_clock=Some((host.begin_civil_time(self.client)?,None));
                        return Ok(waiting);
                    }
                    let (scene, audio, records) = session.capture_scene(host, self.client)?;
                    let native = session.vm.complete_scene_checkpoint(records)?;
                    session.retain_checkpoint(native, scene, audio);
                }
                0x22b4 => {
                    let enabled = session
                        .vm
                        .pending_save_group()?
                        .context("CMVS save group request missing")?;
                    let slots = session
                        .vm
                        .save_menu()
                        .context("CMVS menu has not been loaded")?;
                    if session.vm.menu_options().0 {
                        let checks = session.display.menu_checks();
                        session.menu.publish(host, self.client, slots, enabled, &checks)?;
                    }
                    session.vm.complete_save_group(enabled)?;
                }
                0x22b7 => {
                    let [first, count] = session.vm.owner_arguments(step.1)?;
                    let range = latest_slot_range(first, count);
                    if range.is_empty() {
                        session.vm.complete_owner_call(2, Some(u32::MAX))?;
                    } else {
                        let base = session
                            .vm
                            .resources()
                            .save_file_base()
                            .map(|(path, directory)| (path.clone(), directory));
                        let header = session.latest_save_header;
                        self.job = Some(pool.spawn(move |cancel| {
                            cancel.check()?;
                            let (slot, header) = scan_latest_slot(
                                storage.as_ref(),
                                base.as_ref().map(|(path, directory)| (path, *directory)),
                                range,
                                header,
                                || Ok(cancel.check()?),
                            )?;
                            cancel.check()?;
                            Ok(Completion::LatestSaveSlot(slot, header))
                        })?);
                    }
                }
                0x22b8 => {
                    if session.thumbnail.begin(host, pool, &mut session.vm)? {
                        return Ok(waiting);
                    }
                }
                0x22bb => {
                    let [slot] = session.vm.owner_arguments(step.1)?;
                    let base = session
                        .vm
                        .resources()
                        .save_file_base()
                        .map(|(path, directory)| (path.clone(), directory));
                    let header = session.probed_save_header;
                    self.job = Some(pool.spawn(move |cancel| {
                        cancel.check()?;
                        let (found, header) = probe_save_slot(
                            storage.as_ref(),
                            base.as_ref().map(|(path, directory)| (path, *directory)),
                            slot,
                            header,
                            || Ok(cancel.check()?),
                        )?;
                        cancel.check()?;
                        Ok(Completion::ProbeSaveSlot(found, header))
                    })?);
                }
                0x29ac | 0x22bd => {
                    let cvns = step.1 == 0x29ac;
                    let slot = session.vm.owner_argument(step.1, 4)?;
                    let object = session.vm.owner_argument(step.1, 8)?;
                    let child = if cvns { -1 } else { session.vm.owner_argument(step.1, 12)? as i32 };
                    if (cvns && (slot == 0 || object >= 128)) || (!cvns && ((slot as i32) < 0 || object >= 256)) {
                        session.vm.complete_owner_call(if cvns { 4 } else { 5 }, Some(0))?;
                        session.save_preview_error = None;
                        continue;
                    }
                    let object = object as usize;
                    let wants_image = child < 0 || child < 768 && session.objects.contains(object);
                    let child = usize::try_from(child).ok();
                    let base = session
                        .vm
                        .resources()
                        .save_file_base()
                        .map(|(path, directory)| (path.clone(), directory));
                    let dimensions = session.vm.settings().thumbnail_size;
                    let sources: Vec<_> = session.vm.resources().sources(0x2011).cloned().collect();
                    let cpz6 = session.cpz6.clone();
                    let context = DecodeContext::new(self.client);
                    self.job = Some(pool.spawn(move |cancel| {
                        let mut metadata = crate::vm::read_gameplay_metadata(
                            storage.as_ref(),
                            base.as_ref().map(|(path, directory)| (path, *directory)),
                            slot,
                            wants_image, cvns,
                            || Ok(cancel.check()?),
                        )?;
                        let client = context.client;
                        let replacement = metadata
                            .as_mut()
                            .and_then(|metadata| metadata.preview.take())
                            .map(|preview| {
                                let context = context.with_cancellation(cancel.clone());
                                let image = match preview {
                                    SlotPreview::Black => crysirror_image::ImageSurface::filled(
                                        dimensions[0],
                                        dimensions[1],
                                        [0, 0, 0, 255],
                                    )
                                    .map(|image| (image, true)),
                                    SlotPreview::Encoded(bytes) => {
                                        let mounted = std::cell::OnceCell::new();
                                        let read_asset = |name: &str| {
                                            let sources = mounted
                                                .get_or_init(|| {
                                                    ScriptSources::prepare(
                                                        storage.clone(),
                                                        &cpz6_registry()?,
                                                        &cpz6,
                                                        sources.clone(),
                                                    )
                                                })
                                                .as_ref()
                                                .map_err(|error| Error::Io(format!("{error:#}")))?;
                                            let reader = sources
                                                .open_entry(name)
                                                .map_err(|error| Error::Io(format!("{error:#}")))?
                                                .ok_or(Error::NotFound)?;
                                            crysirror_resource::io::read_all_checked(
                                                reader.as_ref(),
                                                crysirror_image::MAX_DECODED_BYTES as u64,
                                                &|| Ok(cancel.check()?),
                                            )
                                            .map_err(|error| Error::Io(format!("{error:#}")))
                                        };
                                        crysirror_formats_arc::cmvs_image::decode(
                                            &bytes,
                                            &read_asset,
                                            &context,
                                        )
                                    }
                                }
                                .and_then(|(image, alpha)| {
                                    visual::DecodedImage::from_surface(
                                        context.client,
                                        image,
                                        Some(alpha),
                                    )
                                });
                                SavePreviewReplacement {
                                    object,
                                    child,
                                    image,
                                }
                            });
                        cancel.check()?;
                        if cvns {
                            let bitmap = replacement.map(|r| -> anyhow::Result<_> {
                                let image = r.image?;
                                cvns::bitmap::Job {
                                    slot: cvns::BitmapSlot::Global(object), words: 4, picture: Default::default(),
                                    edit: cvns::bitmap::Edit::Load { layer: 0, position: [0; 2], flag: u32::from(image.has_alpha()),
                                        name: Vec::new(), frame: 0 },
                                }.run(client, |_, _| Ok(image))
                            }).transpose()?;
                            Ok(Completion::CvnsSlotMetadata(metadata, bitmap))
                        } else { Ok(Completion::SaveSlotMetadata(metadata, replacement)) }
                    })?);
                }
                0x29ab | 0x22bc | 0x22be => {
                    let cvns = step.1 == 0x29ab;
                    let (slot, words, edit) = if step.1 != 0x22be {
                        let [slot, reference] = session.vm.owner_arguments(step.1)?;
                        let comment = session
                            .vm
                            .text_reference(reference)
                            .context("CMVS save comment reference")?;
                        (slot, 2, SlotHeaderEdit::Comment(comment.to_vec()))
                    } else {
                        let [slot, first, second] = session.vm.owner_arguments(step.1)?;
                        (slot, 3, SlotHeaderEdit::Values([first, second]))
                    };
                    let base = session
                        .vm
                        .resources()
                        .save_file_base()
                        .map(|(path, directory)| (path.clone(), directory));
                    self.job = Some(pool.spawn(move |cancel| {
                        cancel.check()?;
                        let edited = crate::vm::edit_slot_header(
                            storage.as_ref(),
                            base.as_ref().map(|(path, directory)| (path, *directory)),
                            slot,
                            edit, cvns,
                            || Ok(cancel.check()?),
                        )?;
                        Ok(Completion::EditSaveSlot { words, edited })
                    })?);
                }
                0x22cd => {
                    let enabled = session
                        .vm
                        .pending_popup()?
                        .context("CMVS popup request missing")?;
                    if let Some(slots) = session.vm.save_menu() {
                        if enabled {
                            session.menu.publish(
                                host,
                                self.client,
                                slots,
                                session.vm.menu_options().1,
                                &session.display.menu_checks(),
                            )?;
                        } else if session.vm.menu_options().0 {
                            session.menu.hide(host, self.client)?;
                        }
                    }
                    session.vm.complete_popup(enabled)?;
                }
                0x22cb => {
                    let color = session
                        .vm
                        .pending_clear_color()?
                        .context("CMVS clear color request missing")?;
                    let command = if session.clear_layer {
                        serde_json::json!({"op":"set_solid_color","id":CLEAR_LAYER,"color":color})
                    } else {
                        serde_json::json!({"op":"put","layer":{
                            "id":CLEAR_LAYER,"rect":[0,0,self.canvas[0],self.canvas[1]],
                            "order":[i32::MIN,0],"paint":{"kind":"solid","color":color}
                        }})
                    };
                    let batch = serde_json::json!({"layers":[command]});
                    host.submit_engine(self.client, 1, &serde_json::to_vec(&batch)?)?;
                    session.vm.complete_clear_color()?;
                    session.clear_layer = true;
                }
                0x2220..=0x2224 => {
                    let can_skip = !self.visual_effects || session.vm.skipping();
                    if fade::Fade::apply(
                        &mut session.fade,
                        host,
                        self.client,
                        self.canvas,
                        &mut session.vm,
                        can_skip,
                    )? {
                        session.frame_pending = true;
                    }
                }
                0x2128 => {
                    let request = session
                        .vm
                        .pending_system_save()?
                        .context("CMVS system save request missing")?;
                    self.job = Some(pool.spawn(move |cancel| {
                        cancel.check()?;
                        let save = request.prepare(storage.as_ref())?;
                        cancel.check()?;
                        Ok(Completion::SystemSave(save))
                    })?);
                }
                0x2160 | 0x2161 => {
                    let title = session
                        .vm
                        .pending_title()?
                        .context("CMVS title request missing")?;
                    let batch = serde_json::json!({"ui":[{"op":"set_title","text":title}]});
                    host.submit_engine(self.client, 1, &serde_json::to_vec(&batch)?)?;
                    session.vm.complete_title()?;
                }
                0x208f => {
                    let path = session
                        .vm
                        .pending_icon()?
                        .context("CMVS icon request missing")?;
                    self.job = Some(pool.spawn(move |cancel| {
                        let path =
                            crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(
                                storage.as_ref(),
                                &path,
                            )?
                            .context("CMVS window icon missing")?;
                        let source = storage.open_asset(&path)?;
                        let bytes = read_all(source.as_ref(), 4 * 1024 * 1024)?;
                        cancel.check()?;
                        ensure!(
                            bytes.starts_with(b"\0\0\x01\0"),
                            "invalid CMVS window icon signature"
                        );
                        let icon = crysirror_image::cursor::decode_cursor(&bytes)?
                            .frames
                            .into_iter()
                            .next()
                            .context("CMVS window icon has no frame")?;
                        cancel.check()?;
                        Ok(Completion::Icon(icon.image))
                    })?);
                }
                0x20b0 | 0x20b1 | 0x20b3 | 0x20ba | 0x28c5 => match session.vm.pending_effect_sound()? {
                    EffectSound::Load { name, .. } if step.1 == 0x28c5 || session.effects.settings.enabled => {
                        let cpz6 = session.cpz6.clone();
                        let sources = session.vm.resources().sources(0x2012).cloned().collect();
                        self.job = Some(pool.spawn(move |cancel| {
                            cancel.check()?;
                            let sources =
                                ScriptSources::prepare(storage, &cpz6_registry()?, &cpz6, sources)?;
                            let bytes = sources
                                .open_entry(&name)?
                                .map(|input| read_all(input.as_ref(), 32 * 1024 * 1024))
                                .transpose()?;
                            cancel.check()?;
                            Ok(Completion::EffectAudio(bytes))
                        })?);
                    }
                    EffectSound::Load { .. } => {
                        session.effects.remember_disabled(&mut session.vm)?
                    }
                    EffectSound::Start { channel, looping } => {
                        session.effects.start(host, self.client, channel, looping)?;
                        session.vm.complete_effect_sound(None)?;
                    }
                    EffectSound::Stop { channel } => {
                        session.effects.stop(host, self.client, channel)?;
                        session.vm.complete_effect_sound(None)?;
                    }
                },
                0x20b5 => {
                    session.effects.stop_all(host,self.client)?;
                    session.vm.complete_owner_call(0, None)?;
                }
                0x20b2 | 0x20b4 => {
                    let [slot] = session.vm.owner_arguments(step.1)?;
                    let words = if step.1 == 0x20b2 { 1 } else { 2 };
                    if slot as usize >= session.effects.count() {
                        session.latch_syscall_fault(step.0, step.1, 0x200);
                    } else if step.1 == 0x20b2 {
                        session.effects.pause(host, self.client, slot as usize)?;
                    } else {
                        let [_, looping] = session.vm.owner_arguments(step.1)?;
                        if session.effects.restart(
                            host,
                            self.client,
                            slot as usize,
                            looping != 0,
                        )? {
                            if session.effects.poll(host, self.client, &mut session.vm)? {
                                return Ok(waiting);
                            }
                            continue;
                        }
                    }
                    session.vm.complete_owner_call(words, None)?;
                }
                0x20b6 | 0x20b7 => {
                    let [channel] = session.vm.owner_arguments(step.1)?;
                    ensure!((channel as usize) < session.effects.count(), "effect channel outside profile");
                    let channel = channel as usize;
                    if step.1 == 0x20b6 {
                        let [_, duration] = session.vm.owner_arguments(step.1)?;
                        session.effects.fade(host, self.client, channel, duration)?;
                        session.vm.complete_owner_call(2, None)?;
                    } else {
                        let position = session.effects.position(host, self.client, channel)?;
                        session.vm.complete_owner_call(1, Some(position))?;
                    }
                }
                0x20a0 | 0x20a4 => {
                    let request = sound::MusicRequest::read(&session.vm, step.1)?;
                    if session.music.settings.enabled {
                        let cpz6 = session.cpz6.clone();
                        let sources = session.vm.resources().sources(0x2013).cloned().collect();
                        self.job = Some(pool.spawn(move |cancel| {
                            cancel.check()?;
                            let sources =
                                ScriptSources::prepare(storage, &cpz6_registry()?, &cpz6, sources)?;
                            let (first, second) = request.read_sources(&sources, &cancel)?;
                            cancel.check()?;
                            Ok(Completion::Music(request, first, second))
                        })?);
                    } else {
                        request.complete(&mut session.vm)?;
                        session.music.remember_disabled_at(request, host.now_ms());
                    }
                }
                0x20a1 => {
                    session.music.stop(host, self.client)?;
                    session.vm.complete_owner_call(0, None)?;
                }
                0x20a2 => {
                    let [milliseconds] = session.vm.owner_arguments(step.1)?;
                    session.music.fade(host, self.client, milliseconds)?;
                    session.vm.complete_owner_call(1, None)?;
                }
                0x20a5 => {
                    let [gain] = session.vm.owner_arguments(step.1)?;
                    let gain = gain.min(256) as u16;
                    host.set_audio_pcm16_device_gain(self.client, gain)?;
                    session.vm.complete_owner_call(1, None)?;
                    session.samples.set_device_gain(gain);
                }
                0x20a6 | 0x20af | 0x28b8 => {
                    let mut enabled = session.voices.settings.enabled;
                    let request = if step.1 == 0x28b8 {
                        let [slot] = session.vm.owner_arguments(step.1)?;
                        if slot >= 4 {
                            session.latch_syscall_fault(step.0, 0x20b8, 0x8000);
                            session.vm.complete_owner_call(5, None)?;
                            continue;
                        }
                        let (use_voice, master, voices) = session.cvns.voice_settings();
                        enabled = use_voice;
                        sound::VoiceRequest::read_cvns(&session.vm, step.1, master, voices)?
                    } else {
                        sound::VoiceRequest::read(&session.vm, step.1)?
                    };
                    if !enabled {
                        request.disabled(&mut session.vm)?;
                    } else {
                        let cpz6 = session.cpz6.clone();
                        let sources = session.vm.resources().sources(0x2014).cloned().collect();
                        self.job = Some(pool.spawn(move |cancel| {
                            cancel.check()?;
                            let bytes = if let Some(name) = &request.name {
                                let sources = ScriptSources::prepare(
                                    storage,
                                    &cpz6_registry()?,
                                    &cpz6,
                                    sources,
                                )?;
                                sources
                                    .open_entry(name)?
                                    .map(|input| read_all(input.as_ref(), 32 * 1024 * 1024))
                                    .transpose()?
                            } else {
                                None
                            };
                            cancel.check()?;
                            Ok(Completion::Voice(request, bytes))
                        })?);
                    }
                }
                0x20a7 => {
                    let [slot, looping] = session.vm.owner_arguments(step.1)?;
                    if slot > 3 {
                        session.latch_syscall_fault(step.0, step.1, 0x400);
                    } else {
                        session
                            .voices
                            .start(host, self.client, slot, looping != 0)?;
                    }
                    session.vm.complete_owner_call(2, None)?;
                }
                0x20bc | 0x20bd => {
                    let [slot] = session.vm.owner_arguments(step.1)?;
                    let valid = slot <= 3;
                    if !valid {
                        session.latch_syscall_fault(step.0, step.1, 0x400);
                    }
                    if step.1 == 0x20bc {
                        if valid {
                            host.set_audio_channel_retirement(self.client, slot + 5, true)?;
                        }
                    } else if !valid || !session.voices.released(host, self.client, slot)? {
                        session.vm.wait_owner_call(step.1, None)?;
                        return session.finish_native_frame(host, self.client, self.canvas, waiting);
                    }
                    session.vm.complete_owner_call(1, None)?;
                }
                0x20a8 | 0x20a9 | 0x20aa => {
                    let [voice] = session.vm.owner_arguments(step.1)?;
                    ensure!(voice < 4, "CMVS voice channel outside 0..3");
                    let (words, result) = match step.1 {
                        0x20a8 => {
                            session.voices.stop(host, self.client, voice + 5, 0)?;
                            (1, None)
                        }
                        0x20a9 => {
                            let [_, fade] = session.vm.owner_arguments(step.1)?;
                            session.voices.stop(host, self.client, voice + 5, fade)?;
                            (2, None)
                        }
                        _ => (1, Some(sound::position(host, self.client, voice + 5)?)),
                    };
                    session.vm.complete_owner_call(words, result)?;
                }
                0x2240..=0x2242 | 0x2244 => {
                    let [slot] = session.vm.owner_arguments(step.1)?;
                    if slot != 0 {
                        let words = match step.1 {
                            0x2240 => 6,
                            0x2244 => 2,
                            _ => 1,
                        };
                        session.vm.complete_owner_call(words, None)?;
                        bail!("CMVS device sound slot {slot} sets native error 0x200");
                    }
                    match step.1 {
                        0x2240 => {
                            let mut request = sound::DeviceSoundRequest::read(
                                &session.vm,
                                session.music.settings,
                            )?;
                            session.device_sound.stop(host, self.client)?;
                            if let Some(name) = request.name.take() {
                                let cpz6 = session.cpz6.clone();
                                let sources =
                                    session.vm.resources().sources(0x2012).cloned().collect();
                                self.job = Some(pool.spawn(move |cancel| {
                                    cancel.check()?;
                                    let sources = ScriptSources::prepare(
                                        storage,
                                        &cpz6_registry()?,
                                        &cpz6,
                                        sources,
                                    )?;
                                    let bytes = sources
                                        .open_entry(&name)?
                                        .map(|input| read_all(input.as_ref(), 32 * 1024 * 1024))
                                        .transpose()?;
                                    cancel.check()?;
                                    Ok(Completion::DeviceSound(request, bytes))
                                })?);
                            } else {
                                session.vm.complete_owner_call(6, Some(0))?;
                            }
                        }
                        0x2241 => {
                            session.device_sound.stop(host, self.client)?;
                            session.vm.complete_owner_call(1, None)?;
                        }
                        0x2242 => {
                            let position = sound::DeviceSound::position(host, self.client)?;
                            session.vm.complete_owner_call(1, Some(position))?;
                        }
                        _ => {
                            let [_, multiplier] = session.vm.owner_arguments(step.1)?;
                            host.update_audio_pcm16_gain(
                                self.client,
                                sound::DEVICE_SOUND_CHANNEL,
                                sound::fader_update(step.1, multiplier),
                            )?;
                            session.vm.complete_owner_call(2, None)?;
                        }
                    }
                }
                0x20ac | 0x20ad | 0x20b8 | 0x20b9 => {
                    let [slot, word] = session.vm.owner_arguments(step.1)?;
                    if matches!(step.1, 0x20ac | 0x20ad) && slot < 4 {
                        host.update_audio_pcm16_gain(
                            self.client,
                            slot + 5,
                            sound::fader_update(step.1, word),
                        )?;
                    } else if matches!(step.1, 0x20b8 | 0x20b9) && slot < 6 {
                        session.effects.set_fader(
                            host,
                            self.client,
                            slot as usize,
                            step.1,
                            word,
                        )?;
                    } else {
                        let bit = if matches!(step.1, 0x20ac | 0x20ad) {
                            0x400
                        } else {
                            0x200
                        };
                        session.vm.complete_owner_call(2, None)?;
                        bail!("CMVS PCM16 fader slot {slot} sets native error {bit:#x}");
                    }
                    session.vm.complete_owner_call(2, None)?;
                }
                0x20ae | 0x2243 => {
                    let [slot] = session.vm.owner_arguments(step.1)?;
                    let channel = match step.1 {
                        0x20ae if slot < 4 => slot + 5,
                        0x2243 if slot == 0 => sound::DEVICE_SOUND_CHANNEL,
                        _ => {
                            let bit = if step.1 == 0x20ae { 0x400 } else { 0x200 };
                            session.vm.complete_owner_call(1, None)?;
                            bail!("CMVS audio level slot {slot} sets native error {bit:#x}");
                        }
                    };
                    let [left, right] = host
                        .audio_source_level(self.client, channel)?
                        .unwrap_or([0; 2]);
                    session
                        .vm
                        .complete_owner_results(1, &[(4, left), (5, right)])?;
                }
                0x2248..=0x224b | 0x224f => {
                    let [slot] = session.vm.owner_arguments(step.1)?;
                    let words = match step.1 {
                        0x2248 => 2,
                        0x224a => 3,
                        _ => 1,
                    };
                    if slot >= sample::SLOT_COUNT as u32 {
                        session.vm.complete_owner_call(words, None)?;
                        bail!("CMVS hardware sample slot {slot} sets native error 0x200");
                    }
                    let slot = slot as usize;
                    match step.1 {
                        0x2248 => {
                            let [_, reference] = session.vm.owner_arguments(step.1)?;
                            let name = session.vm.sound_name(reference)?;
                            session.samples.release(host, self.client, slot)?;
                            if let Some(name) = name {
                                let cpz6 = session.cpz6.clone();
                                let sources =
                                    session.vm.resources().sources(0x2012).cloned().collect();
                                self.job = Some(pool.spawn(move |cancel| {
                                    cancel.check()?;
                                    let sources = ScriptSources::prepare(
                                        storage,
                                        &cpz6_registry()?,
                                        &cpz6,
                                        sources,
                                    )?;
                                    let reader = sources.open_entry(&name)?;
                                    cancel.check()?;
                                    Ok(Completion::Sample(slot, name, reader))
                                })?);
                                return Ok(waiting);
                            }
                            session.vm.complete_owner_call(words, Some(0))?;
                            continue;
                        }
                        0x2249 => session.samples.release(host, self.client, slot)?,
                        0x224a => {
                            let [_, volume, looping] = session.vm.owner_arguments(step.1)?;
                            session.samples.play(
                                host,
                                self.client,
                                session.effects.settings,
                                slot,
                                volume as i32,
                                looping != 0,
                            )?;
                        }
                        0x224b => session.samples.stop(host, self.client, slot)?,
                        _ => session.samples.duplicate(
                            host,
                            self.client,
                            session.effects.settings,
                            slot,
                        )?,
                    }
                    session.vm.complete_owner_call(words, None)?;
                }
                0x20c0 | 0x20c4 => {
                    let path = session.vm.movie_path()?;
                    self.job = Some(pool.spawn(move |cancel| {
                        cancel.check()?;
                        let path =
                            crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(
                                storage.as_ref(),
                                &path,
                            )?;
                        let source = path.map(|path| storage.open_asset(&path)).transpose()?;
                        cancel.check()?;
                        Ok(Completion::MovieSource(source))
                    })?);
                }
                0x20c1 | 0x20c5 => {
                    session.movies.stop(host, self.client)?;
                    session.vm.complete_owner_call(0, None)?;
                }
                0x20c2 | 0x20c6 => {
                    let frame = session.movies.frame(host, self.client)?;
                    session.vm.complete_owner_call(0, Some(frame))?;
                }
                0x230a => {
                    let [slot] = session.vm.owner_arguments(step.1)?;
                    session
                        .movies
                        .remove_scene(host, self.client, slot as usize)?;
                    session.vm.complete_owner_call(1, None)?;
                }
                0x237c..=0x237f => {
                    if !session.buttons.list_command(step.1, &mut session.vm)? {
                        session.latch_syscall_fault(step.0, step.1, 0x10_0000);
                    }
                }
                0x2087 | 0x2210..=0x221b => {
                    session.buttons.apply(
                        step.1,
                        &mut session.vm,
                        &mut session.objects,
                        host,
                        self.client,
                        self.canvas,
                    )?;
                }
                0x20ec
                | 0x2112
                | 0x2114..=0x2117
                | 0x2119
                | 0x211e..=0x211f
                | 0x2140..=0x214a
                | 0x214c..=0x2151
                | 0x2155
                | 0x215a..=0x215b
                | 0x215d
                | 0x215f
                | 0x2171
                | 0x2178..=0x217b
                | 0x217c
                | 0x217f
                | 0x2180
                | 0x2182..=0x2186
                | 0x218d
                | 0x2190..=0x2193
                | 0x2194..=0x2197
                | 0x21cf..=0x21d2 => {
                    session.messages.apply(
                        step.1,
                        host,
                        self.client,
                        &mut session.vm,
                        &mut session.objects,
                        self.canvas,
                    )?;
                }
                0x2152 | 0x2153 | 0x2157 | 0x2158 | 0x215c => {
                    session.frame_pending = session.messages.output(
                        step.1,
                        host,
                        self.client,
                        &mut session.vm,
                        &mut session.objects,
                        self.canvas,
                        session.frame_delta,
                    )?;
                }
                0x22d0 => session.messages.voice_cut(&mut session.vm, step.1)?,
                0x22d5 => {
                    let [mode] = session.vm.owner_arguments(step.1)?;
                    if mode == 0 {
                        session
                            .vm
                            .complete_owner_call(2, Some(u32::from(session.active_draw)))?;
                    } else {
                        let [_, value] = session.vm.owner_arguments(step.1)?;
                        session.vm.complete_owner_call(2, None)?;
                        session.active_draw = value != 0;
                    }
                }
                opcode @ (0x22d6..=0x22d8 | 0x29ed) => {
                    let (switch, mode, value, words) = if opcode == 0x29ed {
                        let [key, _, value, _] = session.vm.owner_arguments(opcode)?;
                        (0x22d6 + (key - 7) as u16, 1, value, 4)
                    } else {
                        let [mode, value] = session.vm.owner_arguments(opcode)?;
                        (opcode, mode, value, 2)
                    };
                    let current = match switch {
                        0x22d6 => session.music.settings.enabled,
                        0x22d7 => session.effects.settings.enabled,
                        _ => session.voices.settings.enabled,
                    };
                    if mode == 0 {
                        session
                            .vm
                            .complete_owner_call(words, Some(u32::from(current)))?;
                        continue;
                    }
                    let enable = value != 0;
                    if current == enable {
                        session.vm.complete_owner_call(words, None)?;
                        continue;
                    }
                    match switch {
                        0x22d6 => {
                            if let Some(request) = session.music.reopen_request() {
                                if enable {
                                    let cpz6 = session.cpz6.clone();
                                    let paths =
                                        session.vm.resources().sources(0x2013).cloned().collect();
                                    let gain = session.music.gain();
                                    self.job = Some(pool.spawn(move |cancel| {
                                        let sources = ScriptSources::prepare(
                                            storage,
                                            &cpz6_registry()?,
                                            &cpz6,
                                            paths,
                                        )?;
                                        let (primary, secondary) =
                                            request.read_sources(&sources, &cancel)?;
                                        let recipe = request.recipe(1, primary, secondary, gain);
                                        cancel.check()?;
                                        Ok(Completion::AudioSwitch(
                                            AudioReopen::Music(request),
                                            vec![recipe],
                                        ))
                                    })?);
                                    return Ok(waiting);
                                }
                                host.replace_audio_channels(
                                    self.client,
                                    &sound::MUSIC_CHANNELS,
                                    None,
                                )?;
                                session.music.clear_playback();
                            }
                            session.vm.complete_owner_call(words, None)?;
                            session.music.settings.enabled = enable;
                        }
                        0x22d7 => {
                            if enable {
                                let count = session.effects.switched_channels();
                                let plan = session.effects.reopen_plan();
                                if !plan.is_empty() {
                                    let cpz6 = session.cpz6.clone();
                                    let paths =
                                        session.vm.resources().sources(0x2012).cloned().collect();
                                    self.job = Some(pool.spawn(move |cancel| {
                                        let sources = ScriptSources::prepare(
                                            storage,
                                            &cpz6_registry()?,
                                            &cpz6,
                                            paths,
                                        )?;
                                        let mut decoded = vec![false; count];
                                        let mut recipes = Vec::new();
                                        for effect in plan {
                                            cancel.check()?;
                                            let Some(input) = sources.open_entry(&effect.name)?
                                            else {
                                                continue;
                                            };
                                            let bytes = read_all(input.as_ref(), 32 * 1024 * 1024)?;
                                            recipes.push(AudioRequest::Play {
                                                channel: sound::device_channel(effect.slot),
                                                source: EncodedAudio {
                                                    bytes: bytes.into(),
                                                    frame_limit: None,
                                                },
                                                intro: None,
                                                parameters: effect.parameters,
                                            });
                                            decoded[effect.slot] = true;
                                        }
                                        cancel.check()?;
                                        Ok(Completion::AudioSwitch(
                                            AudioReopen::Effects(decoded),
                                            recipes,
                                        ))
                                    })?);
                                    return Ok(waiting);
                                }
                            } else {
                                host.replace_audio_channels(
                                    self.client,
                                    &(0..session.effects.switched_channels()).map(sound::device_channel).collect::<Vec<_>>(),
                                    None,
                                )?;
                            }
                            session.vm.complete_owner_call(words, None)?;
                            session.effects.set_availability(enable, vec![false; session.effects.switched_channels()]);
                        }
                        _ => {
                            session.vm.complete_owner_call(words, None)?;
                            session.voices.settings.enabled = enable;
                        }
                    }
                }
                0x22d9..=0x22dd => session.volume_command(host, self.client, step.1)?,
                0x22de..=0x22e4 | 0x22e6..=0x22e8 | 0x22f8 => {
                    session.messages.text_setting(&mut session.vm, step.1)?;
                }
                0x2106 => session
                    .messages
                    .select_text_face(host, self.client, &mut session.vm)?,
                0x219d | 0x219f | 0x21a1 | 0x21a3 | 0x21a9 | 0x21ab | 0x21ad | 0x21af | 0x21b1
                | 0x21b3 | 0x21b5 | 0x21b7 | 0x21b9 | 0x21bb | 0x21bd | 0x22f1 | 0x22f3 | 0x22f5
                | 0x2345 | 0x2347 | 0x2349 | 0x234b => {
                    let action = retained::cleared_action(step.1).context("CMVS action clear")?;
                    session.vm.consume_action(action);
                    session.vm.complete_owner_call(0, None)?;
                }
                0x20ca => {
                    let [x, y, width, height] = session.vm.owner_arguments(step.1)?;
                    let [px, py] = session.vm.pointer();
                    let [x, y] = [x as i32, y as i32];
                    let inside = px >= x
                        && px <= (width as i32).wrapping_add(x)
                        && py >= y
                        && py <= (height as i32).wrapping_add(y);
                    session.vm.complete_owner_call(4, Some(u32::from(inside)))?;
                }
                0x222f => {
                    let [_, _, _, duration] = session.vm.owner_arguments(step.1)?;
                    session.screen_timer = Some([duration, 0]);
                    session.vm.complete_owner_call(5, Some(1))?;
                }
                0x2247 => {
                    let [_, _, _, duration] = session.vm.owner_arguments(step.1)?;
                    session.screen_timer = Some([duration, 0]);
                    session.vm.complete_owner_call(5, Some(1))?;
                }
                0x2232 => {
                    let left = session
                        .screen_timer
                        .map_or(u32::MAX, |[duration, elapsed]| duration - elapsed);
                    session.vm.complete_owner_call(0, Some(left))?;
                }
                0x2165..=0x2167 => {
                    let [width, height] = self.canvas;
                    session.vm.complete_owner_results(0, &[(1, width), (2, height)])?;
                }
                0x2230 | 0x2231 => {
                    session.screen_timer = None;
                    session.vm.complete_owner_call(0, None)?;
                }
                0x2260..=0x2264 | 0x2270 | 0x2272..=0x2275 => {
                    session.light.command(step.1, &mut session.vm)?;
                }
                0x2069 | 0x2233 | 0x2271 => {
                    let [skippable] = session.vm.owner_arguments(step.1)?;
                    let active = if step.1 == 0x2069 {
                        session.vm.camera_tween_active()
                    } else if step.1 == 0x2271 {
                        session.light.active()
                    } else {
                        session.screen_timer.is_some_and(|[duration, elapsed]| elapsed < duration)
                    };
                    if !active {
                        session.vm.complete_owner_call(1, Some(0))?;
                    } else {
                        let [_, _, bypass, _] = session.vm.message_modes();
                        let clicked = session.vm.action(0)[0];
                        if skippable != 0 && (session.vm.skipping() || bypass || clicked) {
                            if clicked {
                                session.vm.consume_action(0);
                            }
                            if step.1 == 0x2069 {
                                session.vm.finish_camera_tween();
                            } else if step.1 == 0x2271 {
                                session.light.finish(true);
                            } else {
                                session.screen_timer = None;
                            }
                            session.vm.finish_owner_call(1, Some(1), true)?;
                        } else {
                            session.vm.wait_owner_call(step.1, Some(0))?;
                        }
                        session.frame_pending = true;
                    }
                }
                0x2154 | 0x2156 | 0x2159 | 0x215e => {
                    let present = session.messages.window_text(
                        step.1,
                        host,
                        self.client,
                        &mut session.vm,
                        &session.objects,
                        self.canvas,
                    )?;
                    if !present {
                        session.latch_syscall_fault(step.0, step.1, 0x10000);
                    }
                }
                0x2032 | 0x2037 | 0x203b | 0x22c4 => {
                    let [slot, reference] = session.vm.owner_arguments(step.1)?;
                    let words = match step.1 {
                        0x2032 | 0x2037 => 3,
                        0x203b => 5,
                        _ => 4,
                    };
                    let fade = if step.1 == 0x22c4 {
                        session.vm.owner_argument(step.1, 12)?
                    } else {
                        0
                    };
                    if slot >= 256 || !session.objects.contains(slot as usize) {
                        session.latch_syscall_fault(step.0, step.1, 0x10);
                        session.vm.complete_owner_call(words, None)?;
                    } else {
                        let name = session.vm.image_name(step.1, reference)?;
                        if step.1 != 0x22c4 {
                            session.vm.record_seen_image(&name);
                        }
                        let cpz6 = session.cpz6.clone();
                        let sources = session.vm.resources().sources(0x2011).cloned().collect();
                        let context = DecodeContext::new(self.client);
                        let prefetched = session.take_prefetched(&name);
                        self.job = Some(pool.spawn(move |cancel| {
                            cancel.check()?;
                            let sources =
                                ScriptSources::prepare(storage, &cpz6_registry()?, &cpz6, sources)?;
                            let image = visual::prepare_image_with(
                                &sources,
                                &name,
                                prefetched,
                                &context.with_cancellation(cancel.clone()),
                            )?;
                            cancel.check()?;
                            Ok(Completion::VariantImage(slot as usize, name, words, fade, image))
                        })?);
                    }
                }
                0x22c5 => {
                    let [slot] = session.vm.owner_arguments(step.1)?;
                    session.objects.finish_crossfade(host, self.client, slot as usize)?;
                    session.vm.complete_owner_call(1, None)?;
                }
                0x22c6 => {
                    let [slot] = session.vm.owner_arguments(step.1)?;
                    let active = session.objects.crossfade_active(slot as usize);
                    session.vm.complete_owner_call(1, Some(u32::from(active)))?;
                }
                0x22c7 => {
                    let [slot, skippable] = session.vm.owner_arguments(step.1)?;
                    if !session.objects.crossfade_active(slot as usize) {
                        session.vm.complete_owner_call(2, Some(0))?;
                    } else {
                        let [_, _, bypass, _] = session.vm.message_modes();
                        let clicked = session.vm.action(0)[0];
                        if skippable != 0 && (session.vm.skipping() || bypass || clicked) {
                            if clicked {
                                session.vm.consume_action(0);
                            }
                            session.objects.finish_crossfade(host, self.client, slot as usize)?;
                            session.vm.finish_owner_call(2, Some(1), true)?;
                        } else {
                            session.vm.wait_owner_call(step.1, Some(0))?;
                        }
                        session.frame_pending = true;
                    }
                }
                0x20e4 | 0x2390 | 0x2396 | 0x239a..=0x23a0 | 0x23b8 | 0x23ba | 0x23bb => {
                    let present = session.messages.field_command(
                        step.1,
                        host,
                        self.client,
                        &mut session.vm,
                        &session.objects,
                        self.canvas,
                    )?;
                    if !present {
                        session.latch_syscall_fault(step.0, step.1, 0x1000);
                    }
                }
                0x2384 | 0x238d | 0x2398 => {
                    let present = session.messages.field_position(
                        step.1,
                        host,
                        self.client,
                        &mut session.vm,
                        &session.objects,
                        self.canvas,
                    )?;
                    if !present {
                        session.latch_syscall_fault(step.0, step.1, 0x1000);
                    }
                }
                0x2304 => {
                    session.captured_screen = None;
                    session.screen_capture = Some(host.begin_frame_capture(self.client)?);
                    session.vm.complete_owner_call(0, None)?;
                }
                0x2305 => {
                    session.screen_capture = None;
                    session.captured_screen = None;
                    session.vm.complete_owner_call(0, None)?;
                }
                0x2306 => {
                    session.poll_screen_capture()?;
                    let ready = session.captured_screen.is_some();
                    session.vm.complete_owner_call(0, Some(u32::from(ready)))?;
                }
                0x203c..=0x203e => {
                    let [slot, child] = session.vm.owner_arguments(step.1)?;
                    if slot >= 256 || (child as i32) >= 0 || !session.objects.contains(slot as usize) {
                        session.latch_syscall_fault(step.0, step.1, 0x10);
                        session.vm.complete_owner_call(2, None)?;
                    } else if step.1 == 0x203c {
                        let frame = session.captured_screen.take();
                        let loaded = frame.is_some();
                        if let Some(frame) = frame {
                            let image = visual::DecodedImage::from_frame(frame);
                            session.objects.image(host, self.client, slot as usize, image)?;
                        }
                        session.vm.complete_owner_call(2, Some(u32::from(loaded)))?;
                    } else {
                        if session.captured_screen.is_none() && session.screen_capture.is_none() {
                            session.screen_capture = Some(host.begin_frame_capture(self.client)?);
                        }
                        session.poll_screen_capture()?;
                        if let Some(frame) = session.captured_screen.take() {
                            let image = visual::DecodedImage::from_frame(frame);
                            session.objects.image(host, self.client, slot as usize, image)?;
                            session.vm.complete_owner_call(2, Some(1))?;
                        } else {
                            session.vm.wait_owner_call(step.1, None)?;
                            session.frame_pending = true;
                        }
                    }
                }
                // The 2021 table latches the object error for the removed render targets.
                0x202b | 0x2300 | 0x2301 if self.profile == VmProfile::Pc20211007X64 => {
                    session.latch_syscall_fault(step.0, step.1, 0x10);
                    let words = if step.1 == 0x202b { 3 } else { 1 };
                    session.vm.complete_owner_call(words, None)?;
                }
                0x228c..=0x228e => {
                    // 0x14007f020 swaps, 0x14007ef30 deletes and 0x14007ed70 moves the
                    // second slot onto the first, keeping the replaced file as savedel.dat.
                    let words: u16 = if step.1 == 0x228d { 1 } else { 2 };
                    let [first] = session.vm.owner_arguments(step.1)?;
                    let second = if words == 2 { session.vm.owner_argument(step.1, 8)? } else { first };
                    if first > 999 || second > 999 {
                        session.vm.complete_owner_call(words, None)?;
                        continue;
                    }
                    let paths = [session.vm.save_slot_path(first)?, session.vm.save_slot_path(second)?];
                    let backup = paths[0]
                        .relative()
                        .rsplit_once('/')
                        .map_or_else(|| "savedel.dat".to_owned(), |(directory, _)| format!("{directory}/savedel.dat"));
                    let backup = VfsPath::new(Mount::Save, &backup)?;
                    let rescan = (first < 10 && second < 10).then(|| session.vm.save_menu_rescan()).flatten();
                    let opcode = step.1;
                    self.job = Some(pool.spawn(move |cancel| {
                        cancel.check()?;
                        let read = |path: &VfsPath| match storage.read(path) {
                            Ok(bytes) => Ok(Some(bytes)),
                            Err(crysirror_core::Error::NotFound) => Ok(None),
                            Err(error) => Err(anyhow::Error::from(error)),
                        };
                        let remove = |path: &VfsPath| match storage.remove(path) {
                            Ok(()) | Err(crysirror_core::Error::NotFound) => Ok(()),
                            Err(error) => Err(anyhow::Error::from(error)),
                        };
                        let [a, b] = [read(&paths[0])?, read(&paths[1])?];
                        match opcode {
                            0x228c if first != second => match (a, b) {
                                (Some(a), Some(b)) => {
                                    storage.write_atomic(&paths[0], &b)?;
                                    storage.write_atomic(&paths[1], &a)?;
                                }
                                (Some(a), None) => {
                                    storage.write_atomic(&paths[1], &a)?;
                                    remove(&paths[0])?;
                                }
                                (None, Some(b)) => {
                                    storage.write_atomic(&paths[0], &b)?;
                                    remove(&paths[1])?;
                                }
                                (None, None) => {}
                            },
                            0x228d => remove(&paths[0])?,
                            0x228e if first != second => {
                                if let Some(b) = b {
                                    if let Some(a) = a {
                                        storage.write_atomic(&backup, &a)?;
                                    }
                                    storage.write_atomic(&paths[0], &b)?;
                                    remove(&paths[1])?;
                                }
                            }
                            _ => {}
                        }
                        cancel.check()?;
                        let menu = rescan.map(|request| request.prepare(storage.as_ref())).transpose()?;
                        Ok(Completion::SlotFiles(words, menu))
                    })?);
                }
                0x2328..=0x2332 if !(0x232a..=0x232f).contains(&step.1) => {
                    let words: u16 = match step.1 {
                        0x2328 => 10,
                        0x2332 => 4,
                        _ => 1,
                    };
                    let [slot] = session.vm.owner_arguments(step.1)?;
                    let slot = slot as i32;
                    let present = slot < 256
                        && slot >= 0
                        && match step.1 {
                            0x2328 => {
                                let mode = session.vm.owner_argument(step.1, 8)?;
                                let mut words = [0; 8];
                                for (index, word) in words.iter_mut().enumerate() {
                                    *word = session.vm.owner_argument(step.1, 12 + 4 * index)?;
                                }
                                session.objects.start_raster(host, self.client, slot as usize, mode, words)?
                            }
                            0x2329 => session.objects.stop_raster(host, self.client, slot as usize)?,
                            _ => {
                                let mut point = [0.0; 3];
                                if step.1 == 0x2332 {
                                    for (index, axis) in point.iter_mut().enumerate() {
                                        *axis = f32::from_bits(session.vm.owner_argument(step.1, 8 + 4 * index)?);
                                    }
                                }
                                session.objects.point_command(slot as usize, step.1, point)
                            }
                        };
                    if !present {
                        session.latch_syscall_fault(step.0, step.1, 0x10);
                    }
                    session.vm.complete_owner_call(words, None)?;
                }
                0x2282 => {
                    let now = host.now_ms() as u32;
                    session.vm.sequence_mut().start(now);
                    session.vm.complete_owner_call(0, None)?;
                }
                0x2284 => {
                    // 0x14008a000: the remaining records run at once, then the list is cleared.
                    let actions = {
                        let objects = &session.objects;
                        sequence::run(&mut session.vm, true, host.now_ms() as u32, |slot| slot < 256 && objects.contains(slot))
                    };
                    let loads = session.apply_sequence(host, self.client, actions)?;
                    if loads.is_empty() {
                        session.vm.sequence_mut().reset();
                        session.vm.complete_owner_call(0, None)?;
                    } else {
                        let task = session.spawn_sequence_images(pool, self.client, loads)?;
                        self.job = Some(task);
                    }
                }
                0x22b6 => {
                    let [slot] = session.vm.owner_arguments(step.1)?;
                    let Some(checkpoint) = session.main_checkpoint.as_ref().filter(|_| slot <= 999)
                    else {
                        session.vm.complete_owner_call(1, None)?;
                        continue;
                    };
                    let snapshot = serde_json::to_vec(&serde_json::json!({
                        "format": "crysirror-cmvs-1",
                        "vm": checkpoint.native.snapshot(),
                        "objects": checkpoint._objects.snapshot(),
                        "buttons": checkpoint._buttons,
                        "messages": checkpoint._messages.snapshot(),
                        "light": session.light,
                        "music": checkpoint._audio.music(),
                        "scenes": checkpoint._scenes,
                        "effects": checkpoint._effects,
                    }))?;
                    let main = checkpoint
                        .native
                        .main_expanded()
                        .context("CMVS save main script image")?;
                    let title = checkpoint.native.save_title().to_vec();
                    let bitmap = session.thumbnail.saved_bitmap();
                    let path = session.vm.save_slot_path(slot)?;
                    self.job = Some(pool.spawn(move |cancel| {
                        cancel.check()?;
                        let thumbnail = bitmap
                            .map(|bitmap| read_all(bitmap.as_ref(), 64 * 1024 * 1024))
                            .transpose()?;
                        let header = crate::vm::save_slot::slot_header(
                            &slot_stamp(),
                            &title,
                            thumbnail.is_some(),
                        );
                        let file = crate::vm::save_slot::pack(
                            header,
                            &snapshot,
                            thumbnail.as_deref(),
                            &main,
                            || Ok(cancel.check()?),
                        )?;
                        cancel.check()?;
                        storage.write_atomic(&path, &file)?;
                        Ok(Completion::SlotSaved)
                    })?);
                }
                0x22b5 => {
                    let [slot] = session.vm.owner_arguments(step.1)?;
                    let base = session
                        .vm
                        .resources()
                        .save_file_base()
                        .map(|(path, directory)| (path.clone(), directory));
                    let profile = session.vm.profile();
                    let cpz6 = session.cpz6.clone();
                    let scripts = session.vm.resources().script_sources();
                    let pictures: Vec<_> = session.vm.resources().sources(0x2011).cloned().collect();
                    let videos: Vec<_> = session.vm.resources().sources(0x201a).cloned().collect();
                    let tracks: Vec<_> = session.vm.resources().sources(0x2013).cloned().collect();
                    let context = DecodeContext::new(self.client);
                    let canvas = self.canvas;
                    self.job = Some(pool.spawn(move |cancel| {
                        let load = || -> Result<Box<LoadedSlot>> {
                            let (_, reader) = crate::vm::open_save_slot(
                                storage.as_ref(),
                                base.as_ref().map(|(path, directory)| (path, *directory)),
                                slot,
                            )?
                            .context("CMVS save slot is empty")?;
                            let saved = crate::vm::save_slot::unpack(reader.as_ref(), || {
                                Ok(cancel.check()?)
                            })?;
                            let snapshot: serde_json::Value =
                                serde_json::from_slice(&saved.checkpoint)
                                    .context("CMVS save was not written by CrySirror")?;
                            ensure!(
                                snapshot["format"] == "crysirror-cmvs-1",
                                "unsupported CMVS save payload"
                            );
                            let registry = cpz6_registry()?;
                            let open = |list: Vec<_>| {
                                ScriptSources::prepare(storage.clone(), &registry, &cpz6, list)
                            };
                            let scripts = crate::vm::SavedScripts::prepare(
                                profile,
                                &open(scripts.clone())?,
                                &snapshot,
                            )?;
                            let sources = open(pictures.clone())?;
                            let mut images = Vec::new();
                            for (object, child, name) in
                                visual::Objects::saved_images(&snapshot["objects"])
                            {
                                cancel.check()?;
                                let image = visual::prepare_image(
                                    &sources,
                                    &name,
                                    &context.clone().with_cancellation(cancel.clone()),
                                )?;
                                images.push((object, child, name, image));
                            }
                            let sources = open(videos.clone())?;
                            let mut movies = Vec::new();
                            for (object, slot, name, size, alpha) in
                                visual::Objects::saved_movies(&snapshot["objects"])
                            {
                                let source = sources
                                    .open_entry(&name)?
                                    .with_context(|| format!("CMVS saved movie missing: {name}"))?;
                                movies.push(SavedMovie { object, slot, name, source, size, alpha });
                            }
                            let music = match snapshot["music"].is_null() {
                                true => None,
                                false => {
                                    let mut request: sound::MusicRequest =
                                        serde_json::from_value(snapshot["music"].clone())?;
                                    request.restored = true;
                                    let (first, second) =
                                        request.read_sources(&open(tracks.clone())?, &cancel)?;
                                    Some((request, first, second))
                                }
                            };
                            let mut effects = Vec::new();
                            let saved: Vec<Option<effect::Saved>> =
                                serde_json::from_value(snapshot["effects"].clone()).unwrap_or_default();
                            for (index, record) in saved.into_iter().enumerate().take(4) {
                                let Some(record) = record else { continue };
                                cancel.check()?;
                                let image = match visual::prepare_image(&sources, &record.name, &context.clone().with_cancellation(cancel.clone())) {
                                    Ok(image) => Some(image),
                                    Err(error) if error.to_string().starts_with("CMVS image missing") => None,
                                    Err(error) => return Err(error),
                                };
                                effects.push((index, record, image));
                            }
                            let matte = |name: &str| -> Result<Option<visual::DecodedImage>> {
                                match visual::prepare_image(&sources, name, &context.clone().with_cancellation(cancel.clone())) {
                                    Ok(image) => Ok(Some(image.screen_matte(canvas)?)),
                                    Err(error) if error.to_string().starts_with("CMVS image missing") => Ok(None),
                                    Err(error) => Err(error),
                                }
                            };
                            let mut seconds = Vec::new();
                            for (slot, name) in visual::Objects::saved_seconds(&snapshot["objects"]) {
                                cancel.check()?;
                                let image = matte(&name)?;
                                seconds.push((slot, name, image));
                            }
                            let mut mattes = Vec::new();
                            for (field, name) in message::Messages::saved_mattes(&snapshot["messages"]) {
                                cancel.check()?;
                                let image = matte(&name)?;
                                mattes.push((field, name, image));
                            }
                            Ok(Box::new(LoadedSlot { snapshot, scripts, images, movies, music, effects, seconds, mattes }))
                        };
                        Ok(Completion::SlotLoaded(load().map_err(|why| format!("{why:#}"))))
                    })?);
                }
                0x2129 => {
                    let (path, backup, image) = session.vm.system_save_image()?;
                    self.job = Some(pool.spawn(move |cancel| {
                        cancel.check()?;
                        match storage.open_read(&path) {
                            Ok(source) => storage.write_atomic(
                                &backup,
                                &read_all(source.as_ref(), 16 * 1024 * 1024)?,
                            )?,
                            Err(Error::NotFound) => {}
                            Err(error) => return Err(error.into()),
                        }
                        storage.write_atomic(&path, &image)?;
                        Ok(Completion::SystemSaved)
                    })?);
                }
                0x2308 | 0x230e => {
                    // 0x14008a760 / 0x14008a950: slot, object, name, an unused word and the
                    // first frame; 0x230e adds the reader mode 0x2308 fixes at 2.
                    let words: u16 = if step.1 == 0x2308 { 5 } else { 6 };
                    let [index, object, reference, _, start] = session.vm.owner_arguments(step.1)?;
                    if index > 2 || object >= 256 {
                        session.latch_syscall_fault(step.0, step.1, 0x100_0000);
                        session.vm.complete_owner_call(words, None)?;
                    } else {
                        let name = session.vm.image_name(step.1, reference)?;
                        let cpz6 = session.cpz6.clone();
                        let sources = session.vm.resources().sources(0x201a).cloned().collect();
                        self.job = Some(pool.spawn(move |cancel| {
                            cancel.check()?;
                            let sources =
                                ScriptSources::prepare(storage, &cpz6_registry()?, &cpz6, sources)?;
                            let source = sources
                                .open_entry(&name)?
                                .with_context(|| format!("CMVS scene movie missing: {name}"))?;
                            let mut head = [0; 36];
                            source.read_at(0, &mut head)?;
                            let word = |at: usize| {
                                u32::from_le_bytes([head[at], head[at + 1], head[at + 2], head[at + 3]])
                            };
                            let size = [word(28), word(32)];
                            let [frames, rate, scale] = movie::timing(&head);
                            let player = movie::ScenePlayer::new(frames, rate, scale, start.min(frames.saturating_sub(1)));
                            cancel.check()?;
                            Ok(Completion::SceneMovie(
                                index as usize,
                                object as usize,
                                name,
                                source,
                                size,
                                &head[..4] == b"CMV8",
                                player,
                                words,
                            ))
                        })?);
                    }
                }
                0x2309 | 0x230b..=0x230d | 0x230f | 0x231a..=0x231e => {
                    let words: u16 = match step.1 {
                        0x2309 => 5,
                        0x230d | 0x231b | 0x231d => 2,
                        _ => 1,
                    };
                    let [index] = session.vm.owner_arguments(step.1)?;
                    if index > 2 {
                        session.latch_syscall_fault(step.0, step.1, 0x100_0000);
                        session.vm.complete_owner_call(words, None)?;
                        continue;
                    }
                    let slot = index as usize;
                    let now = host.now_ms();
                    let mut results = Vec::new();
                    let mut seek = None;
                    match session.movies.player(slot) {
                        None => match step.1 {
                            0x230f => results.extend([(0, u32::MAX), (4, 0)]),
                            0x231a..=0x231c | 0x231e => results.push((0, 0)),
                            _ => {}
                        },
                        Some(player) => match step.1 {
                            0x2309 => {
                                let [_, section, start, end, looping] = session.vm.owner_arguments(step.1)?;
                                player.play(section, start as i32, end as i32, looping != 0, now);
                                seek = Some(false);
                            }
                            0x230b => {
                                player.release(now);
                                seek = Some(false);
                            }
                            0x230c => {
                                player.hold();
                                seek = Some(false);
                            }
                            0x230d => player.set_mode(session.vm.owner_argument(step.1, 8)?),
                            0x231d => player.set_stop(session.vm.owner_argument(step.1, 8)?),
                            0x230f => {
                                let (section, frame) = player.position();
                                results.extend([(0, section), (4, frame)]);
                            }
                            0x231a => results.push((0, u32::from(player.playing()))),
                            0x231b => {
                                let frame = session.vm.owner_argument(step.1, 8)?;
                                seek = Some(player.seek(frame));
                                results.push((0, 1));
                            }
                            0x231c => {
                                let shown = player.take_pending();
                                seek = shown.map(|shown| shown);
                                results.push((0, u32::from(shown.is_some())));
                            }
                            _ => results.push((0, player.frame())),
                        },
                    }
                    if let Some(seek) = seek {
                        session.movies.sync_scene(host, self.client, slot, seek)?;
                    }
                    session.vm.complete_owner_results(words, &results)?;
                }
                0x2038 => {
                    let [slot, child, reference] = session.vm.owner_arguments(step.1)?;
                    if slot >= 256 || !session.objects.has_child(slot as usize, child as usize) {
                        session.latch_syscall_fault(step.0, step.1, 0x10);
                        session.vm.complete_owner_call(3, None)?;
                    } else {
                        let name = session.vm.image_name(step.1, reference)?;
                        let cpz6 = session.cpz6.clone();
                        let sources = session.vm.resources().sources(0x2011).cloned().collect();
                        let context = DecodeContext::new(self.client);
                        let prefetched = session.take_prefetched(&name);
                        self.job = Some(pool.spawn(move |cancel| {
                            cancel.check()?;
                            let sources =
                                ScriptSources::prepare(storage, &cpz6_registry()?, &cpz6, sources)?;
                            let image = visual::prepare_image_with(
                                &sources,
                                &name,
                                prefetched,
                                &context.with_cancellation(cancel.clone()),
                            )?;
                            cancel.check()?;
                            Ok(Completion::ChildImage(slot as usize, child as usize, name, image))
                        })?);
                    }
                }
                0x2056 => {
                    let [camera, alternate, x, y, z, lift, focus] = session.vm.owner_arguments(step.1)?;
                    let point = [x, y, z].map(f32::from_bits);
                    let point = match session.vm.cameras().get(camera as usize) {
                        Some(camera) => visual::project_point(
                            camera,
                            alternate == 1,
                            point,
                            f32::from_bits(lift),
                            f32::from_bits(focus),
                        ),
                        None => point,
                    };
                    let [x, y, z] = point.map(|value| value as i32 as u32);
                    session.vm.complete_owner_results(7, &[(1, x), (2, y), (3, z)])?;
                }
                0x20a3 => {
                    let elapsed = session.music.elapsed(host, self.client, host.now_ms())?;
                    session.vm.complete_owner_call(0, Some(elapsed))?;
                }
                0x20ab => {
                    let [voice] = session.vm.owner_arguments(step.1)?;
                    if voice > 3 {
                        session.latch_syscall_fault(step.0, step.1, 0x400);
                        session.vm.complete_owner_call(1, None)?;
                    } else {
                        let bytes = session.voices.decoded_bytes(host, self.client, voice)?;
                        session.vm.complete_owner_call(1, Some(bytes))?;
                    }
                }
                0x2238..=0x223f => {
                    // 0x14009c9b0 family: odd selectors read, even ones stage one word.
                    let index = usize::from((step.1 - 0x2238) / 2);
                    let mut values = session.ducking.parameters();
                    if step.1 & 1 != 0 {
                        let value = values[index];
                        let value = if index == 3 { u32::from(value != 0) } else { value };
                        session.vm.complete_owner_call(0, Some(value))?;
                    } else {
                        let [value] = session.vm.owner_arguments(step.1)?;
                        values[index] = match index {
                            0 => (value as i32).clamp(0, 100) as u32,
                            1 => (value as i32).min(2000) as u32,
                            2 => (value as i32).min(3000) as u32,
                            _ => u32::from(value != 0),
                        };
                        session.ducking.stage(values);
                        session.vm.complete_owner_call(1, None)?;
                    }
                }
                0x2290..=0x2297 => {
                    let opcode = step.1;
                    let [index] = session.vm.owner_arguments(opcode)?;
                    if index > 3 {
                        session.vm.complete_owner_call(scene_effect::words(opcode), None)?;
                    } else {
                        let names = match opcode {
                            0x2290 => {
                                let [_, ground, water] = session.vm.owner_arguments(opcode)?;
                                vec![session.vm.image_name(opcode, ground)?, session.vm.image_name(opcode, water)?]
                            }
                            0x2292 => scene_effect::TOWER_TEXTURES.map(str::to_owned).into(),
                            0x2294 => scene_effect::PARTICLE_TEXTURES.map(str::to_owned).into(),
                            0x2295 => scene_effect::ACTOR_TEXTURES.map(str::to_owned).into(),
                            0x2296 => scene_effect::SPIN_TEXTURES.map(str::to_owned).into(),
                            0x2297 => scene_effect::ORBIT_TEXTURES.map(str::to_owned).into(),
                            _ => Vec::new(),
                        };
                        let samples = match opcode {
                            0x2291 => scene_effect::WALK_SAMPLES,
                            0x2292 => scene_effect::TOWER_SAMPLES,
                            0x2294 => scene_effect::PARTICLE_SAMPLE,
                            0x2295 => scene_effect::ACTOR_SAMPLES,
                            _ => &[],
                        };
                        let cpz6 = session.cpz6.clone();
                        let image_roots = session.vm.resources().sources(0x2011).cloned().collect();
                        let sound_roots = session.vm.resources().sources(0x2012).cloned().collect();
                        let context = DecodeContext::new(self.client);
                        self.job = Some(pool.spawn(move |cancel| {
                            let context = context.with_cancellation(cancel.clone());
                            let registry = cpz6_registry()?;
                            let mut images = SceneImages::default();
                            if opcode == 0x2293 || !names.is_empty() {
                                let sources = ScriptSources::prepare(storage.clone(), &registry, &cpz6, image_roots)?;
                                if opcode == 0x2293 {
                                    for (index, [preferred, alternative]) in scene_effect::DOOR_PANELS.into_iter().enumerate() {
                                        cancel.check()?;
                                        let mut image = read_scene_image(&sources, preferred, &context)?;
                                        if image.as_ref().is_none_or(|image| image.size()[0] != 512) {
                                            image = read_scene_image(&sources, alternative, &context)?;
                                            images.door_fallback[index] = true;
                                        }
                                        images.textures.push(image);
                                    }
                                    images.textures.push(read_scene_image(&sources, scene_effect::DOOR_OVERLAY, &context)?);
                                } else {
                                    for name in names {
                                        cancel.check()?;
                                        images.textures.push(read_scene_image(&sources, &name, &context)?);
                                    }
                                }
                            }
                            let mut audio = Vec::with_capacity(samples.len());
                            if !samples.is_empty() {
                                let sources = ScriptSources::prepare(storage, &registry, &cpz6, sound_roots)?;
                                for &(slot, name) in samples {
                                    cancel.check()?;
                                    audio.push((slot, name, sources.open_entry(name)?));
                                }
                            }
                            Ok(Completion::SceneAssets { opcode, index: index as usize, images, audio })
                        })?);
                    }
                }
                0x22a0 => {
                    let [index] = session.vm.owner_arguments(step.1)?;
                    session.release_special_scene(host, self.client, index as usize)?;
                    session.fx.release(host, self.client, &mut session.objects, index as usize)?;
                    session.vm.complete_owner_call(1, None)?;
                }
                0x22ae => {
                    let [index] = session.vm.owner_arguments(step.1)?;
                    let status = (index <= 3).then(|| {
                        let index = index as usize;
                        session.special_scenes[index].as_ref().map_or_else(
                            || session.fx.status(index),
                            scene_effect::Scene::status,
                        )
                    });
                    session.vm.complete_owner_call(1, status)?;
                }
                0x22a8 => {
                    let [index, source, size, _] = session.vm.owner_arguments(step.1)?;
                    if index <= 3 && source < 256 {
                        let now = host.now_ms();
                        session.release_special_scene(host, self.client, index as usize)?;
                        session.fx.shatter(
                            host,
                            self.client,
                            &mut session.objects,
                            &mut session.rand,
                            index as usize,
                            source as usize,
                            size,
                            now,
                        )?;
                    }
                    session.vm.complete_owner_call(4, None)?;
                }
                0x22af => {
                    let [index, key, value] = session.vm.owner_arguments(step.1)?;
                    let index = index as usize;
                    let requested_images = match session.special_scenes.get(index) {
                        Some(Some(scene_effect::Scene::Mesh(mesh))) => mesh.requested_images(key),
                        _ => None,
                    };
                    if let Some(names) = requested_images {
                        let cpz6 = session.cpz6.clone();
                        let roots = session.vm.resources().sources(0x2011).cloned().collect();
                        let context = DecodeContext::new(self.client);
                        self.job = Some(pool.spawn(move |cancel| {
                            let context = context.with_cancellation(cancel.clone());
                            let sources = ScriptSources::prepare(storage, &cpz6_registry()?, &cpz6, roots)?;
                            let mut images = Vec::with_capacity(names.len());
                            for name in names {
                                cancel.check()?;
                                images.push(read_scene_image(&sources, name, &context)?);
                            }
                            Ok(Completion::SceneReload(index, images))
                        })?);
                    } else {
                        match session.special_scenes.get_mut(index) {
                            Some(Some(scene)) => scene.trigger(key),
                            _ => session.fx.set(index, key, value),
                        }
                        session.vm.complete_owner_call(4, None)?;
                    }
                }
                0x22a1..=0x22a7 | 0x22a9 => {
                    let [index, reference] = session.vm.owner_arguments(step.1)?;
                    let words: u16 = if matches!(step.1, 0x22a1 | 0x22a2 | 0x22a4 | 0x22a5) { 8 } else { 6 };
                    if index > 3 {
                        session.vm.complete_owner_call(words, None)?;
                    } else {
                        let name = session.vm.image_name(step.1, reference)?;
                        let mut parameters = (3..=words)
                            .map(|word| session.vm.owner_argument(step.1, 4 * usize::from(word)))
                            .collect::<Result<Vec<_>, _>>()?;
                        // Particle counts sit in the last word but one (0x22a2) or the last word;
                        // streak emitters end with their interval instead.
                        let count = match step.1 {
                            0x22a1 | 0x22a4 => None,
                            0x22a2 => Some(parameters.len() - 2),
                            _ => Some(parameters.len() - 1),
                        };
                        if let Some(count) = count {
                            parameters[count] = parameters[count].min(4096);
                        }
                        let opcode = step.1;
                        let cpz6 = session.cpz6.clone();
                        let sources = session.vm.resources().sources(0x2011).cloned().collect();
                        let context = DecodeContext::new(self.client);
                        self.job = Some(pool.spawn(move |cancel| {
                            cancel.check()?;
                            let sources = ScriptSources::prepare(storage, &cpz6_registry()?, &cpz6, sources)?;
                            let image = match visual::prepare_image(&sources, &name, &context.with_cancellation(cancel.clone())) {
                                Ok(image) => Some(image),
                                Err(error) if error.to_string().starts_with("CMVS image missing") => None,
                                Err(error) => return Err(error),
                            };
                            Ok(Completion::EffectImage(opcode, index as usize, name, parameters, image))
                        })?);
                    }
                }
                0x203f => {
                    session.use_prefetch = true;
                    session.vm.complete_owner_call(0, None)?;
                }
                0x2338 => {
                    // 0x140047cb0 waits for an earlier preload, then hands the name to the loader.
                    if session.prefetch_busy()? {
                        session.vm.wait_owner_call(step.1, None)?;
                        session.frame_pending = true;
                        continue;
                    }
                    let [reference] = session.vm.owner_arguments(step.1)?;
                    let name = session.vm.image_name(step.1, reference)?;
                    let cpz6 = session.cpz6.clone();
                    let sources = session.vm.resources().sources(0x2011).cloned().collect();
                    let read = name.clone();
                    let task = pool.spawn(move |cancel| {
                        cancel.check()?;
                        let sources = ScriptSources::prepare(storage, &cpz6_registry()?, &cpz6, sources)?;
                        let bytes = sources
                            .open_entry(&read)?
                            .map(|input| read_all(input.as_ref(), 64 * 1024 * 1024))
                            .transpose()?;
                        Ok(bytes.map(Arc::from))
                    })?;
                    session.prefetch = Some((name, Prefetch::Loading(task)));
                    session.vm.complete_owner_call(1, None)?;
                }
                0x2339 | 0x233a => {
                    // 0x2339 retries while the loader is busy; 0x233a waits, then clears the cache.
                    if session.prefetch_busy()? {
                        session.vm.wait_owner_call(step.1, None)?;
                        session.frame_pending = true;
                        continue;
                    }
                    if step.1 == 0x233a {
                        session.prefetch = None;
                    }
                    session.vm.complete_owner_call(0, None)?;
                }
                0x222e => {
                    // The original captures the back buffer into a new screen-sized object 0,
                    // then runs the 0x222a hiding mask on it.
                    if session.captured_screen.is_none() && session.screen_capture.is_none() {
                        session.screen_capture = Some(host.begin_frame_capture(self.client)?);
                    }
                    session.poll_screen_capture()?;
                    let Some(frame) = session.captured_screen.take() else {
                        session.vm.wait_owner_call(step.1, None)?;
                        session.frame_pending = true;
                        continue;
                    };
                    let request = mask::Request::screen(&session.vm, step.1)?;
                    mask::Active::remove(&mut session.masks[0], &mut session.objects, host, self.client, false)?;
                    session.objects.reset(host, self.client, 0, None, true)?;
                    let image = visual::DecodedImage::from_frame(frame);
                    let [width, height] = image.size().map(|axis| axis as i16);
                    session.objects.image(host, self.client, 0, image)?;
                    for property in [
                        crate::vm::DrawProperty::Frame { crop: [0, 0, width, height], origin: [0, 0] },
                        crate::vm::DrawProperty::Position([0, 0]),
                        crate::vm::DrawProperty::Priority(0),
                    ] {
                        session.objects.draw(host, self.client, 0, None, property)?;
                    }
                    let cpz6 = session.cpz6.clone();
                    let sources = session.vm.resources().sources(0x2011).cloned().collect();
                    let context = DecodeContext::new(self.client);
                    self.job = Some(pool.spawn(move |cancel| {
                        cancel.check()?;
                        let sources =
                            ScriptSources::prepare(storage, &cpz6_registry()?, &cpz6, sources)?;
                        let image = match visual::prepare_image(&sources, &request.name, &context.with_cancellation(cancel.clone())) {
                            Ok(image) => Some(image.invert_rgb()?),
                            Err(error) if error.to_string().starts_with("CMVS image missing") => None,
                            Err(error) => return Err(error),
                        };
                        cancel.check()?;
                        Ok(Completion::MaskImage(request, image))
                    })?);
                }
                0x2034 => {
                    let [slot, child, x, y] = session.vm.owner_arguments(step.1)?;
                    if slot >= 256 || !session.objects.contains(slot as usize) {
                        session.latch_syscall_fault(step.0, step.1, 0x10);
                        session.vm.complete_owner_call(4, None)?;
                    } else {
                        let child = usize::try_from(child as i32).ok();
                        let value = session.objects.pixel(
                            host,
                            self.client,
                            slot as usize,
                            child,
                            [x as i32, y as i32],
                        )?;
                        session.vm.complete_owner_call(4, Some(value))?;
                    }
                }
                0x2397 => {
                    let [index, reference] = session.vm.owner_arguments(step.1)?;
                    if !session.messages.has_field(index as usize) {
                        session.latch_syscall_fault(step.0, step.1, 0x1000);
                        session.vm.complete_owner_call(2, None)?;
                        continue;
                    }
                    let name = session.vm.image_name(step.1, reference)?;
                    let cpz6 = session.cpz6.clone();
                    let sources = session.vm.resources().sources(0x2011).cloned().collect();
                    let context = DecodeContext::new(self.client);
                    let canvas = self.canvas;
                    let prefetched = session.take_prefetched(&name);
                    self.job = Some(pool.spawn(move |cancel| {
                        cancel.check()?;
                        let sources = ScriptSources::prepare(storage, &cpz6_registry()?, &cpz6, sources)?;
                        let image = match visual::prepare_image_with(&sources, &name, prefetched, &context.with_cancellation(cancel.clone())) {
                            Ok(image) => Some(image.screen_matte(canvas)?),
                            Err(error) if error.to_string().starts_with("CMVS image missing") => None,
                            Err(error) => return Err(error),
                        };
                        Ok(Completion::FieldMatte(index as usize, name, image))
                    })?);
                }
                0x2035 => {
                    let [slot, reference] = session.vm.owner_arguments(step.1)?;
                    if slot >= 256 || !session.objects.contains(slot as usize) {
                        session.latch_syscall_fault(step.0, step.1, 0x10);
                        session.vm.complete_owner_call(4, None)?;
                    } else {
                        let name = session.vm.image_name(step.1, reference)?;
                        session.vm.record_seen_image(&name);
                        let cpz6 = session.cpz6.clone();
                        let sources = session.vm.resources().sources(0x2011).cloned().collect();
                        let context = DecodeContext::new(self.client);
                        let canvas = self.canvas;
                        let prefetched = session.take_prefetched(&name);
                        self.job = Some(pool.spawn(move |cancel| {
                            cancel.check()?;
                            let sources =
                                ScriptSources::prepare(storage, &cpz6_registry()?, &cpz6, sources)?;
                            let image = match visual::prepare_image_with(
                                &sources,
                                &name,
                                prefetched,
                                &context.with_cancellation(cancel.clone()),
                            ) {
                                Ok(image) => Some(image.screen_matte(canvas)?),
                                Err(error)
                                    if error.to_string().starts_with("CMVS image missing") =>
                                {
                                    None
                                }
                                Err(error) => return Err(error),
                            };
                            cancel.check()?;
                            Ok(Completion::SecondTexture(slot as usize, name, image))
                        })?);
                    }
                }
                0x201c | 0x20cb | 0x20cc | 0x2099 => {
                    match session.host.command(step.1, &mut session.vm, host, self.client)? {
                        Some(host::Host::LoadCursor(index, path)) => {
                            self.job = Some(pool.spawn(move |cancel| {
                                cancel.check()?;
                                let image = host::HostState::read_cursor(&storage, &path)?;
                                Ok(Completion::Cursor(index, image))
                            })?);
                        }
                        Some(host::Host::Done) => {}
                        None => unreachable!("CMVS host selector"),
                    }
                }
                0x2310..=0x2317 => {
                    let words: u16 = match step.1 {
                        0x2310 => 5,
                        0x2311 => 2,
                        0x2312 => 1,
                        0x2313 | 0x2314 => 3,
                        0x2315 => 4,
                        _ => 6,
                    };
                    let [slot] = session.vm.owner_arguments(step.1)?;
                    if slot >= 256 || !session.objects.contains(slot as usize) {
                        session.latch_syscall_fault(step.0, step.1, 0x10);
                        session.vm.complete_owner_call(words, None)?;
                        continue;
                    }
                    let slot = slot as usize;
                    let word = |vm: &crate::vm::CmvsVm, index: usize| vm.owner_argument(step.1, 4 * index);
                    match step.1 {
                        0x2310 => {
                            let [layer, reference, x, y] = [word(&session.vm, 2)?, word(&session.vm, 3)?, word(&session.vm, 4)?, word(&session.vm, 5)?];
                            let name = session.vm.image_name(step.1, reference)?;
                            let cpz6 = session.cpz6.clone();
                            let sources = session.vm.resources().sources(0x2011).cloned().collect();
                            let context = DecodeContext::new(self.client);
                            self.job = Some(pool.spawn(move |cancel| {
                                cancel.check()?;
                                let sources = ScriptSources::prepare(storage, &cpz6_registry()?, &cpz6, sources)?;
                                let image = match visual::prepare_image(&sources, &name, &context.with_cancellation(cancel.clone())) {
                                    Ok(image) => Some(image),
                                    Err(error) if error.to_string().starts_with("CMVS image missing") => None,
                                    Err(error) => return Err(error),
                                };
                                Ok(Completion::CompositeLayer(slot, (layer as usize).min(15), [x as i32, y as i32], image))
                            })?);
                            continue;
                        }
                        0x2311 => session.composites.release(slot, word(&session.vm, 2)? as usize),
                        0x2312 => session.composites.bake(host, self.client, &mut session.objects, slot)?,
                        0x2313 | 0x2314 => {
                            let [layer, color] = [word(&session.vm, 2)?, word(&session.vm, 3)?];
                            session.composites.adjust(slot, step.1, layer as usize, [color, 0]);
                        }
                        0x2315 => {
                            let [layer, mode, parameter] = [word(&session.vm, 2)?, word(&session.vm, 3)?, word(&session.vm, 4)?];
                            session.composites.adjust(slot, step.1, layer as usize, [mode, parameter]);
                        }
                        0x2316 => {
                            let [layer, source, child, x, y] =
                                [word(&session.vm, 2)?, word(&session.vm, 3)?, word(&session.vm, 4)?, word(&session.vm, 5)?, word(&session.vm, 6)?];
                            if source >= 256 || !session.objects.contains(source as usize) {
                                session.latch_syscall_fault(step.0, step.1, 0x10);
                            } else {
                                let child = (child as i32 >= 0).then_some(child as usize);
                                // 0x140045420 copies the source picture when it has one.
                                if let Some((path, alpha)) = session.objects.picture(source as usize, child) {
                                    if let Some(picture) = host.published_image_resource(self.client, &path)? {
                                        let [width, height] = picture.size();
                                        let image = crysirror_image::ImageSurface { width, height, rgba: picture.rgba().to_vec() };
                                        session.composites.copy(slot, (layer as usize).min(15), image, alpha, [x as i32, y as i32]);
                                    }
                                }
                            }
                        }
                        _ => {
                            let [layer, width, height, x, y] =
                                [word(&session.vm, 2)?, word(&session.vm, 3)?, word(&session.vm, 4)?, word(&session.vm, 5)?, word(&session.vm, 6)?];
                            if width as u64 * height as u64 <= 64 * 1024 * 1024 {
                                session.composites.solid(slot, (layer as usize).min(15), [width, height], [x as i32, y as i32]);
                            }
                        }
                    }
                    session.vm.complete_owner_call(words, None)?;
                }
                0x23c0 => {
                    let [reference, cell] = session.vm.owner_arguments(step.1)?;
                    let name = session.vm.image_name(step.1, reference)?;
                    let cpz6 = session.cpz6.clone();
                    let sources = session.vm.resources().sources(0x2011).cloned().collect();
                    let context = DecodeContext::new(self.client);
                    self.job = Some(pool.spawn(move |cancel| {
                        cancel.check()?;
                        let sources = ScriptSources::prepare(storage, &cpz6_registry()?, &cpz6, sources)?;
                        let image = match visual::prepare_image(&sources, &name, &context.with_cancellation(cancel.clone())) {
                            Ok(image) => Some(image),
                            Err(error) if error.to_string().starts_with("CMVS image missing") => None,
                            Err(error) => return Err(error),
                        };
                        Ok(Completion::ImageFontSheet(cell, image))
                    })?);
                }
                0x23c1 | 0x23c2 => session.messages.image_font_command(step.1, &mut session.vm)?,
                0x23a4..=0x23a8 => {
                    let font = session.messages.ui_font(host, self.client)?;
                    // 0x14002d4f0 prefixes the slot directory the save files use.
                    let slot = session.vm.save_slot_path(0)?;
                    let prefix = slot.relative().strip_suffix("save000.dat").unwrap_or_default().to_owned();
                    match session.text_debug.command(step.1, &mut session.vm, host, self.client, self.canvas, &font, &prefix)? {
                        text_debug::Request::Waiting => session.frame_pending = true,
                        text_debug::Request::Done => {}
                        text_debug::Request::Load(path) => {
                            let path = VfsPath::new(Mount::Save, &path)?;
                            self.job = Some(pool.spawn(move |cancel| {
                                cancel.check()?;
                                let bytes = match storage.read(&path) {
                                    Ok(bytes) => bytes,
                                    Err(crysirror_core::Error::NotFound) => Vec::new(),
                                    Err(error) => return Err(error.into()),
                                };
                                ensure!(bytes.len() <= 2 * 1024 * 1024, "CMVS text debug file size");
                                Ok(Completion::TextDebugLoaded(text_debug::parse_file(&bytes)))
                            })?);
                        }
                        text_debug::Request::Save(path, bytes) => {
                            let path = VfsPath::new(Mount::Save, &path)?;
                            self.job = Some(pool.spawn(move |cancel| {
                                cancel.check()?;
                                storage.write_atomic(&path, &bytes)?;
                                Ok(Completion::TextDebugSaved)
                            })?);
                        }
                    }
                }
                0x213a..=0x213d | 0x29b1 | 0x29b2 | 0x29b6 | 0x29b7 => {
                    let font = if step.1 == 0x29b6 { session.cvns.default_font(host, self.client)?.to_string() }
                        else { session.messages.ui_font(host, self.client)?.to_string() };
                    match session.dialogs.command(
                        step.1,
                        &mut session.vm,
                        host,
                        self.client,
                        self.canvas,
                        &font,
                    )? {
                        dialog::Outcome::Waiting => session.frame_pending = true,
                        dialog::Outcome::Done => {}
                        dialog::Outcome::Face(face) if step.1 == 0x29b6 => {
                            session.cvns.select_font(host, self.client, face)?;
                        }
                        dialog::Outcome::Face(face) => {
                            session.messages.select_default_face(host, self.client, face)?
                        }
                    }
                }
                opcode
                    if session.display.command(
                        opcode,
                        &mut session.vm,
                        host,
                        self.client,
                        self.canvas,
                    )? =>
                {
                    if opcode == 0x22f9 {
                        session.messages.set_alpha_revision(session.display.alpha_revision());
                    }
                    if let Some(bytes) = session.display.take_config() {
                        if opcode == 0x22f9 && session.menu.published() {
                            let slots = session.vm.save_menu().context("CMVS menu has not been loaded")?;
                            let checks = session.display.menu_checks();
                            session.menu.publish(host, self.client, slots, session.vm.menu_options().1, &checks)?;
                        }
                        let path = display::Display::config_path()?;
                        self.job = Some(pool.spawn(move |cancel| {
                            cancel.check()?;
                            storage.write_atomic(&path, &bytes)?;
                            Ok(Completion::ConfigSaved)
                        })?);
                    }
                }
                0x2025
                | 0x2039
                | 0x2026
                | 0x202c..=0x202d
                | 0x2041..=0x2043
                | 0x2049..=0x204c
                | 0x204e
                | 0x204f
                | 0x2051
                | 0x2054
                | 0x2055
                | 0x2070..=0x2075
                | 0x2078
                | 0x2321 => {
                    let cameras = session.vm.cameras();
                    session.objects.set_cameras(host, self.client, cameras)?;
                    if let Some(bit) = session.objects.property_command(
                        step.1,
                        host,
                        self.client,
                        &mut session.vm,
                    )? {
                        session.latch_syscall_fault(step.0, step.1, bit);
                    }
                }
                0x2100..=0x2105
                | 0x2107..=0x2111
                | 0x2113
                | 0x2118
                | 0x211a
                | 0x211c
                | 0x211d
                | 0x20e7 => {
                    let present = session.messages.object_command(
                        step.1,
                        host,
                        self.client,
                        &mut session.vm,
                        &session.objects,
                        self.canvas,
                    )?;
                    if !present {
                        session.latch_syscall_fault(step.0, step.1, 0x1000);
                    }
                }
                0x2170 => {
                    let [slot, reference] = session.vm.owner_arguments(step.1)?;
                    ensure!(
                        session.messages.contains(slot as usize),
                        "CMVS window image target missing"
                    );
                    let name = session.vm.image_name(step.1, reference)?;
                    let cpz6 = session.cpz6.clone();
                    let sources = session.vm.resources().sources(0x2011).cloned().collect();
                    let context = DecodeContext::new(self.client);
                    let prefetched = session.take_prefetched(&name);
                    self.job = Some(pool.spawn(move |cancel| {
                        cancel.check()?;
                        let sources =
                            ScriptSources::prepare(storage, &cpz6_registry()?, &cpz6, sources)?;
                        let image = visual::prepare_image_with(
                            &sources,
                            &name,
                            prefetched,
                            &context.with_cancellation(cancel.clone()),
                        )?;
                        cancel.check()?;
                        Ok(Completion::WindowImage(slot as usize, name, image))
                    })?);
                }
                0x2027 => {
                    let [slot, child] = session.vm.owner_arguments(step.1)?;
                    ensure!(slot < 256, "CMVS object slot outside 0..255");
                    let exists = if (child as i32) < 0 {
                        session.objects.contains(slot as usize)
                    } else {
                        session.objects.has_child(slot as usize, child as usize)
                    };
                    session.vm.complete_owner_call(2, Some(u32::from(exists)))?;
                }
                0x21ea..=0x21ee => {
                    session
                        .objects
                        .opacity_command(step.1, host, self.client, &mut session.vm)?
                }
                0x21e0..=0x21e9 | 0x21ef..=0x21fd => {
                    if let Some(bit) = session.objects.track_command(
                        step.1,
                        host,
                        self.client,
                        &mut session.vm,
                    )? {
                        session.latch_syscall_fault(step.0, step.1, bit);
                    }
                }
                0x220c | 0x2225..=0x2229 => {
                    session.frame_pending =
                        quake::command(&mut session.quake, &mut session.vm, step.1)?;
                    session
                        .objects
                        .set_quake(host, self.client, quake::offsets(&session.quake))?;
                }
                0x222a | 0x2234 => {
                    let request = mask::Request::read(&session.vm, step.1)?;
                    ensure!(
                        !session.masks[request.channel ^ 1]
                            .as_ref()
                            .is_some_and(|mask| mask.target() == request.slot),
                        "CMVS mask channels cannot share an object's shader"
                    );
                    if request.channel == 1 {
                        ensure!(
                            !session.masks[1].as_ref().is_some_and(|mask| {
                                mask.target() != request.slot
                                    && session.objects.contains(mask.target())
                            }),
                            "CMVS second mask replacement leaves an existing target's shader bound"
                        );
                    }
                    let cpz6 = session.cpz6.clone();
                    let sources = session.vm.resources().sources(0x2011).cloned().collect();
                    let context = DecodeContext::new(self.client);
                    self.job = Some(pool.spawn(move |cancel| {
                        cancel.check()?;
                        let sources =
                            ScriptSources::prepare(storage, &cpz6_registry()?, &cpz6, sources)?;
                        let image = match visual::prepare_image(&sources, &request.name, &context.with_cancellation(cancel.clone())) {
                            Ok(image) if request.reveal => Some(image),
                            Ok(image) => Some(image.invert_rgb()?),
                            Err(error) if error.to_string().starts_with("CMVS image missing") => None,
                            Err(error) => return Err(error),
                        };
                        cancel.check()?;
                        Ok(Completion::MaskImage(request, image))
                    })?);
                }
                0x222b | 0x2235 => {
                    let [skip] = session.vm.owner_arguments(step.1)?;
                    let channel = usize::from(step.1 == 0x2235);
                    let active = session.masks[channel].is_some();
                    let finish = active
                        && skip != 0
                        && (!self.visual_effects
                            || session.vm.skipping()
                            || session.vm.take_fade_confirm());
                    if finish {
                        mask::Active::remove(
                            &mut session.masks[channel],
                            &mut session.objects,
                            host,
                            self.client,
                            true,
                        )?;
                        session.vm.finish_owner_call(1, Some(1), true)?;
                    } else if active {
                        session.vm.wait_owner_call(step.1, Some(0))?;
                    } else {
                        session.vm.complete_owner_call(1, Some(0))?;
                    }
                    session.frame_pending = active;
                }
                0x222c | 0x2236 => {
                    let channel = usize::from(step.1 == 0x2236);
                    let remaining = session.masks[channel]
                        .as_ref()
                        .map_or(u32::MAX, mask::Active::remaining);
                    session.vm.complete_owner_call(0, Some(remaining))?;
                }
                0x222d | 0x2237 => {
                    let channel = usize::from(step.1 == 0x2237);
                    mask::Active::remove(
                        &mut session.masks[channel],
                        &mut session.objects,
                        host,
                        self.client,
                        true,
                    )?;
                    session.vm.complete_owner_call(0, None)?;
                }
                0x22d1 => {
                    let [set] = session.vm.owner_arguments(step.1)?;
                    if set != 0 {
                        let [_, next] = session.vm.owner_arguments(step.1)?;
                        session.vm.complete_owner_call(2, None)?;
                        self.visual_effects = next != 0;
                    } else {
                        session
                            .vm
                            .complete_owner_call(2, Some(u32::from(self.visual_effects)))?;
                    }
                }
                0x2028 => {
                    let [target, source] = session.vm.owner_arguments(step.1)?;
                    ensure!(
                        target < 256 && source < 256,
                        "CMVS object slot outside 0..255"
                    );
                    session
                        .objects
                        .move_to(host, self.client, target as usize, source as usize)?;
                    session
                        .buttons
                        .move_object(source as usize, target as usize);
                    session.vm.complete_owner_call(2, None)?;
                }
                0x2031 => {
                    let [slot] = session.vm.owner_arguments(step.1)?;
                    if slot < 256 && session.objects.contains(slot as usize) {
                        session
                            .objects
                            .release_image(host, self.client, slot as usize)?;
                    } else {
                        session.latch_syscall_fault(step.0, step.1, 0x10);
                    }
                    session.vm.complete_owner_call(1, None)?;
                }
                0x2033 => {
                    let [slot] = session.vm.owner_arguments(step.1)?;
                    ensure!(slot < 256, "CMVS image query slot outside 0..255");
                    if let Some(([width, height], alpha)) =
                        session.objects.image_info(slot as usize)?
                    {
                        session.vm.complete_owner_results(
                            1,
                            &[(0, 1), (1, width), (2, height), (3, u32::from(alpha))],
                        )?;
                    } else {
                        session.vm.complete_owner_call(1, Some(0))?;
                    }
                }
                0x202a | 0x202e => {
                    let [slot, child] = session.vm.owner_arguments(step.1)?;
                    ensure!(
                        slot < 256 && (child as i32) < 768,
                        "CMVS object flag target"
                    );
                    let child = ((child as i32) >= 0).then_some(child as usize);
                    let (property, words) = if step.1 == 0x202a {
                        let [_, _, width, height] = session.vm.owner_arguments(step.1)?;
                        (crate::vm::DrawProperty::LogicalSize([width, height]), 4)
                    } else {
                        (crate::vm::DrawProperty::ClearQuakeFollow, 2)
                    };
                    session
                        .objects
                        .draw(host, self.client, slot as usize, child, property)?;
                    session.vm.complete_owner_call(words, None)?;
                }
                0x2020..=0x2023 | 0x202f | 0x2030 | 0x2040 | 0x2044..=0x2048 | 0x2050 => {
                    match session.vm.pending_scene_object()?.0 {
                        SceneObject::Clear => {
                            session.objects.clear(host, self.client)?;
                            session.vm.complete_scene_object(None)?;
                        }
                        SceneObject::Reset {
                            slot,
                            child,
                            create,
                        } => {
                            session
                                .objects
                                .reset(host, self.client, slot, child, create)?;
                            session.vm.complete_scene_object(None)?;
                        }
                        SceneObject::Draw {
                            slot,
                            child,
                            property,
                        } => {
                            session
                                .objects
                                .draw(host, self.client, slot, child, property)?;
                            session.vm.complete_scene_object(None)?;
                        }
                        SceneObject::Load { slot, name } => {
                            ensure!(session.objects.contains(slot), "CMVS image object missing");
                            session.vm.record_seen_image(&name);
                            let cpz6 = session.cpz6.clone();
                            let sources = session.vm.resources().sources(0x2011).cloned().collect();
                            let context = DecodeContext::new(self.client);
                            let prefetched = session.take_prefetched(&name);
                            self.job = Some(pool.spawn(move |cancel| {
                                cancel.check()?;
                                let sources = ScriptSources::prepare(
                                    storage,
                                    &cpz6_registry()?,
                                    &cpz6,
                                    sources,
                                )?;
                                let image = visual::prepare_image_with(
                                    &sources,
                                    &name,
                                    prefetched,
                                    &context.with_cancellation(cancel.clone()),
                                )?;
                                cancel.check()?;
                                Ok(Completion::ObjectImage(image))
                            })?);
                        }
                    }
                }
                0x2003 | 0x2080 | 0x2081 => {
                    let request = session
                        .vm
                        .pending_script()?
                        .context("CMVS script request missing")?;
                    let cpz6 = session.cpz6.clone();
                    let sources = session.vm.resources().script_sources();
                    self.job = Some(pool.spawn(move |cancel| {
                        cancel.check()?;
                        let sources =
                            ScriptSources::prepare(storage, &cpz6_registry()?, &cpz6, sources)?;
                        cancel.check()?;
                        let script = sources.prepare_transfer(request)?;
                        cancel.check()?;
                        Ok(Completion::Script(script))
                    })?);
                }
                _ => bail!(
                    "unsupported CMVS syscall 0x{:04x} at {}+0x{:x}",
                    step.1,
                    String::from_utf8_lossy(session.vm.script_name()),
                    step.0,
                ),
            }
            if session.frame_pending {
                return session.finish_native_frame(host, self.client, self.canvas, waiting);
            }
            if self.job.is_some() {
                return Ok(waiting);
            }
        }
    }
}

impl VmAdapter for Adapter {
    fn instruction_budget(&self) -> u32 {
        match self.session.as_ref().filter(|_| self.profile == VmProfile::Cvns20040515X86).map(|s| s.cvns.priority) {
            Some(0) => 2048,
            Some(2) => 512,
            _ => 1024,
        }
    }

    fn engine_id(&self) -> &'static str {
        "cmvs"
    }
    fn snapshot_schema(&self) -> u32 {
        1
    }
    fn preparation_pending(&self) -> bool {
        self.session.is_none()
    }

    fn diagnostics(&self) -> Vec<u8> {
        let frontier = self.session.as_ref().map(|s| {
            serde_json::json!({
                "script":String::from_utf8_lossy(s.vm.script_name()),"pc":s.vm.pc(),
            })
        });
        let checkpoint = self
            .session
            .as_ref()
            .and_then(|s| s.main_checkpoint.as_ref())
            .map(|checkpoint| {
                let (name, pc) = checkpoint.native.location();
                serde_json::json!({"script":String::from_utf8_lossy(name),"pc":pc})
            });
        serde_json::to_vec(&serde_json::json!({
            "frontier":frontier,"storage_pending":self.job.is_some(),"main_checkpoint":checkpoint,
            "save_preview_error":self.session.as_ref().and_then(|session| session.save_preview_error.as_deref()),
            "save_thumbnail":self.session.as_ref().map(|session| session.thumbnail.diagnostics()),
        }))
        .unwrap_or_default()
    }

    fn step(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        budget: u32,
    ) -> crysirror_core::Result<HostYield> {
        self.advance(host, input, budget)
            .map_err(|error| Error::Io(format!("{error:#}")))
    }

    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Err(Error::Unsupported("CMVS engine snapshot"))
    }

    fn restore(&mut self, _: &[u8]) -> crysirror_core::Result<()> {
        Err(Error::Unsupported("CMVS engine snapshot"))
    }
}

struct Factory;
impl VmFactory for Factory {
    fn create(&self, client: ClientId, spec: &ClientSpec) -> Result<Box<dyn VmAdapter>> {
        ensure!(spec.program.is_empty(), "CMVS requires native script data");
        let config: Config = serde_json::from_value(spec.config.clone())?;
        let profile = match config.profile.as_str() {
            "cvns-20040515-x86" => VmProfile::Cvns20040515X86,
            "pc-20160201-x64" => VmProfile::Pc20160201X64,
            "pc-20211007-x64" => VmProfile::Pc20211007X64,
            profile => bail!("unsupported CMVS ABI profile {profile}"),
        };
        ensure!(
            config.canvas.iter().all(|axis| (1..=8192).contains(axis)),
            "CMVS canvas dimensions"
        );
        config.effects.validate()?;
        config.music.validate()?;
        config.voices.validate()?;
        let movie_volume =
            sound::MovieVolume::new(config.music, config.movie_volume, config.movie_gain)?;
        let entry = VfsPath::new(Mount::Asset, &config.script)?;
        let executable = VfsPath::new(Mount::Asset, &config.executable)?;
        Ok(Box::new(Adapter {
            client,
            profile,
            entry,
            executable,
            canvas: config.canvas,
            effects_settings: config.effects,
            music_settings: config.music,
            voice_settings: config.voices,
            visual_effects: config.visual_effects.unwrap_or(true),
            movie_volume,
            pool: None,
            job: None,
            session: None,
        }))
    }
}

pub fn register(registry: &mut AdapterRegistry) -> Result<()> {
    registry.register("cmvs", Arc::new(Factory))
}

fn civil_registers(sample: CivilTime) -> Result<[(u32, u32); 3]> {
    let weekday = sample
        .weekday()
        .context("CMVS invalid local calendar date")?;
    let year = u16::try_from(sample.year).context("CMVS local calendar year outside SYSTEMTIME")?;
    ensure!(
        sample.hour < 24 && sample.minute < 60 && sample.second < 60,
        "CMVS invalid local clock time"
    );
    let date = (u32::from(year) * 100 + u32::from(sample.month)) * 100 + u32::from(sample.day);
    let time =
        (u32::from(sample.hour) * 100 + u32::from(sample.minute)) * 100 + u32::from(sample.second);
    Ok([(4, date), (5, time), (6, u32::from(weekday))])
}

fn ini_integer(text: &str, section: &str, key: &str) -> i32 {
    text.lines()
        .map(str::trim)
        .skip_while(|line| !line.eq_ignore_ascii_case(section))
        .skip(1)
        .take_while(|line| !line.starts_with('['))
        .filter_map(|line| line.split_once('='))
        .find(|(candidate, _)| candidate.trim().eq_ignore_ascii_case(key))
        .and_then(|(_, value)| value.trim().parse::<i32>().ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_clock_uses_windows_weekday_and_native_result_slots() {
        let sample = CivilTime {
            client: ClientId(1),
            unix_ms: 0,
            utc_offset_seconds: 0,
            year: 2026,
            month: 11,
            day: 15,
            hour: 9,
            minute: 7,
            second: 3,
            millisecond: 0,
        };
        assert_eq!(
            civil_registers(sample).unwrap(),
            [(4, 20_261_115), (5, 90_703), (6, 0)]
        );
        assert!(civil_registers(CivilTime { day: 31, ..sample }).is_err());
    }
}

fn cpz6_registry() -> Result<FormatRegistry> {
    let mut registry = FormatRegistry::default();
    registry.register(
        crysirror_formats_arc::legacy_archive("purple-software/cpz1")
            .context("shared CPZ1 format missing")?,
    )?;
    registry.register(
        crysirror_formats_arc::legacy_archive("purple-software/cpz6")
            .context("shared CPZ6 format missing")?,
    )?;
    registry.register(
        crysirror_formats_arc::legacy_archive("purple-software/cpz7")
            .context("shared CPZ7 format missing")?,
    )?;
    Ok(registry)
}

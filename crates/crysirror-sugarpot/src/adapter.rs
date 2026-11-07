// SPDX-License-Identifier: MPL-2.0
//! Thin semantic boundary for the Tsukumo-2012 native profile. Nothing here
//! decodes/composes pixels or mixes PCM. Native waits retain the originating PC
//! through resource completion; the engine owns layers, widgets and voices.
mod animation;
mod backlog;
mod bgm_mode;
mod choice;
mod endmode;
mod fall;
mod filter;
mod gauge;
mod geometry;
mod load;
mod menu;
mod motion;
mod raster;
mod recollection;
mod restore;
mod rotation;
mod save;
mod slots;
mod stage;
mod storage;
mod update;
mod update_angular;
mod update_bands;
mod update_checker;
mod update_mosaic;
mod update_iris;
mod update_scan;
mod update_squeeze;
mod update_wave;
mod update_zoom;
mod warp;
mod window_system;
mod window_ui;
use crate::{
    expression::Value,
    machine::{Event, Machine},
};
use anyhow::{Context, Result, bail, ensure};
use crysirror_core::{
    Error,
    image_processing::{ImageInput, ImageJob, ImageProcessState},
    path::{Mount, VfsPath},
    service::{
        AudioParameters, AudioPreparePhase, AudioRequest, ClientStorage, EncodedAudio, EngineQuery,
        HostServices, InputFrame, MediaPhase, MediaRequest, PcmBus, UploadedImage, buttons,
    },
    state::ClientId,
    vm::{VmAdapter, VmYield},
};
use crysirror_tasks::{Task, TaskPool};
use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    ///Original executable is read-only PE resource data, never run or loaded.
    #[serde(default)]
    pub executable: Option<String>,
    pub script: String,
    pub canvas: [u32; 2],
    pub assets: BTreeMap<String, String>,
    pub font: String,
}
#[derive(Clone, Default)]
struct Sprite {
    source_name: String,
    native_depth: i16,
    scratch: Option<UploadedImage>,
    image: Option<UploadedImage>,
    position: [i32; 2],
    visible: bool,
    system: i32,
    origin: i32,
    attribute: i32,
    cell: u32,
    divisions: u32,
    animation: Option<animation::AnimationState>,
}
#[derive(Clone)]
struct Button {
    sprite: Sprite,
    label: Option<(String, [i32; 2])>,
    enabled: bool,
    state: u32,
}
#[derive(Clone)]
struct Window {
    native: crate::native_storage::slot::SlotWindow,
    properties: BTreeMap<String, Vec<Value>>,
    buttons: Vec<Vec<Value>>,
    //Native byte24 distinguishes right-click hidden0, shown1 and an
    //effect/explicit wndoff hidden2. Wndon accepts2 only (44A280).
    visibility: u8,
    //Native WORDs26/28 and BYTEs30/31 are execution state distinct from the
    //saved TXT origin38/40. Original restore derives them by redraw.
    text_cursor: [i32; 2],
    text_cell: [i32; 2],
    text_paint: Vec<Json>,
}
impl Default for Window {
    fn default() -> Self {
        //446220 initializes newly allocated windows to state2, not0.
        Self {
            native: Default::default(),
            properties: Default::default(),
            buttons: vec![],
            visibility: 2,
            text_cursor: [0; 2],
            text_cell: [0; 2],
            text_paint: vec![],
        }
    }
}
#[derive(Clone, Debug)]
enum Wait {
    Clock(u64),
    RestoreTransition {
        deadline: u64,
        resume: Option<Box<Wait>>,
        show_window: bool,
    },
    Input,
    Buttons,
    Selection,
    Geometry,
    Rotation,
    Warp,
    Text,
    Audio(u32),
    AudioStop(u32),
    Movie,
    OriginalLoad,
    OriginalSave,
    Slots,
    Dialog,
    Settings,
    Recollection,
    Gallery,
    BgmMode,
    Backlog,
    EndMode,
    Shake(u64),
    EfMovement(motion::EfMovement),
    Raster {
        remaining: u32,
        next: u64,
    },
    AnimationNumber {
        id: usize,
        target: usize,
        deadline: u64,
        next: u64,
    },
    Motion {
        start: u64,
        duration: u64,
        kind: i32,
    },
}
struct Read {
    path: String,
    job: Task<Arc<[u8]>>,
}
struct Image {
    path: String,
    job: ImageJob,
}
pub struct SugarPotVm {
    client: ClientId,
    config: Config,
    machine: Option<Machine>,
    pool: Option<TaskPool>,
    boot: Option<Task<crate::resources::Boot>>,
    settings: crate::native_storage::SystemSettings,
    globals: BTreeMap<String, Value>,
    read_flags: Vec<crate::native_storage::ReadFlag>,
    saving_globals: Option<Task<()>>,
    native_write: Option<(String, Task<()>)>,
    native_load: Option<load::NativeLoad>,
    load_command_done: bool,
    native_save: Option<save::NativeSave>,
    save_command_done: bool,
    slot_menu: Option<slots::SlotMenu>,
    read_all: bool,
    current_read: bool,
    right_button: i32,
    storage: Option<Arc<dyn ClientStorage>>,
    reading: Option<Read>,
    decoded: Option<Image>,
    allocating: Option<ImageJob>,
    images: BTreeMap<String, UploadedImage>,
    animations: BTreeMap<String, Arc<animation::AnimationAsset>>,
    animation_load: Option<animation::AnimationLoad>,
    animation_tone: Option<animation::AnimationTone>,
    depths: BTreeMap<String, i16>,
    mask_update: Option<update::MaskUpdate>,
    ud_wave_phase: f32,
    /// The C runtime rand() state shared by `rand` (srand(clock) first) and
    /// the UD37/38 run effects; the runtime's initial state is 1.
    crt_rand: u32,
    /// Published mono/sepia copies (tone_image), collected like the cache.
    toned: BTreeSet<String>,
    /// Host time at which the last submitted screen transition ends; prefix
    /// captures are not admitted while it runs.
    pub(crate) transition_until: u64,
    disc_tables: Option<Arc<[Vec<i32>; 4]>>,
    disc_load: Option<Task<[Vec<i32>; 4]>>,
    ud_scan_counter: i32,
    ud_progressive_counter: i32,
    effect_skip: bool,
    //408540 calls42FFC0 only after33ms, updating a1[5918] first. Workers
    //may be polled between those native ticks without advancing an effect.
    native_effect_next: u64,
    //Shared native E518D8 scratch, deliberately not encoded in slot files.
    native_effect_index: i32,
    moves: Vec<motion::Move>,
    shaking: Option<motion::Shaking>,
    geometry: Option<geometry::Geometry>,
    rotation: Option<rotation::Rotation>,
    warp: Option<warp::Warp>,
    stage_underlay: Option<UploadedImage>,
    clear_underlay: bool,
    raster: Option<raster::Raster>,
    filter: Option<filter::Filter>,
    fall: Option<fall::Fall>,
    gauges: Vec<crate::native_storage::slot::SlotGauge>,
    mono: i32,
    mono_next: Option<usize>,
    toning: Option<(String, ImageJob)>,
    next_image: u32,
    declared_images: BTreeSet<String>,
    sprites: Vec<Sprite>,
    button_capacity: usize,
    buttons: BTreeMap<i32, Button>,
    windows: Vec<Window>,
    current_window: usize,
    selections: Vec<crate::native_storage::slot::Selection>,
    selection_paints: Vec<(Json, bool)>,
    selection_focus: Option<usize>,
    backlog: Window,
    backlog_view: Option<backlog::View>,
    wait: Option<Wait>,
    mode: i32,
    save_enabled: bool,
    load_enabled: bool,
    complete_effect_gates: bool,
    skip_enabled: bool,
    font_size: i32,
    font_color: [u8; 4],
    font_style: [i32; 2],
    title: String,
    memo: String,
    audio: Option<u64>,
    audio_setup: bool,
    last_music: String,
    last_music_loop: bool,
    last_env: String,
    last_env_loop: bool,
    native_audio: crate::native_storage::slot::SlotAudio,
    settings_closing: bool,
    recollection: Option<recollection::Recollection>,
    gallery: Option<recollection::Recollection>,
    bgm_mode: Option<bgm_mode::BgmMode>,
    end_mode: Option<endmode::EndMode>,
    menu_audio: Vec<(String, u32, PcmBus, bool)>,
    voice_group: String,
    history: backlog::History,
    published: bool,
    text_published: bool,
    text: String,
    native_text: crate::text::Execution,
    pending_text: Option<crate::text::Prepared>,
    text_remainder: String,
    novel_body: String,
    caption: String,
    last_voice_suffix: String,
    //Native String1 belongs to cvon/44BEA0, and is consumed/cleared only by
    //44C510 history insertion. It is independent of current speaker and CV bus.
    pending_voice_suffix: String,
    text_glyphs: usize,
    reveal_until: u64,
    white: Option<[u8; 4]>,
    clear_color: [u8; 4],
    movie: bool,
    desktop_request: Option<Box<dyn crysirror_core::service::DesktopRequestJob>>,
    staged_batches: Option<Vec<crysirror_core::native_restore::OriginalRestoreBatch>>,
    restore_clock_origin: Option<u64>,
    published_layers: Vec<u32>,
    published_ui: Vec<u32>,
    window_layers: Vec<Json>,
    window_ui: window_ui::WindowUi,
    error: Option<String>,
    api_calls: u64,
}
impl SugarPotVm {
    pub fn new(client: ClientId, mut config: Config) -> Result<Self> {
        ensure!(
            config.canvas == [1024, 600],
            "SugarPot native profile is 1024x600"
        );
        VfsPath::new(Mount::Asset, &config.script)?;
        if let Some(executable) = &config.executable {
            VfsPath::new(Mount::Asset, executable)?;
        }
        ensure!(
            !config.font.is_empty(),
            "SugarPot project font alias required"
        );
        ensure!(
            config.assets.len() <= 100000,
            "SugarPot asset catalog budget"
        );
        config.script = config.script.to_ascii_lowercase();
        config.assets = config
            .assets
            .into_iter()
            .map(|(k, v)| (k.to_ascii_lowercase(), v))
            .collect();
        Ok(Self {
            client,
            config,
            machine: None,
            pool: None,
            boot: None,
            settings: Default::default(),
            globals: BTreeMap::new(),
            read_flags: vec![],
            saving_globals: None,
            native_write: None,
            native_load: None,
            load_command_done: false,
            native_save: None,
            save_command_done: false,
            slot_menu: None,
            read_all: false,
            current_read: false,
            right_button: 0,
            storage: None,
            reading: None,
            decoded: None,
            allocating: None,
            images: BTreeMap::new(),
            animations: BTreeMap::new(),
            animation_load: None,
            animation_tone: None,
            depths: BTreeMap::new(),
            mask_update: None,
            ud_wave_phase: 0.,
            crt_rand: 1,
            toned: BTreeSet::new(),
            transition_until: 0,
            disc_tables: None,
            disc_load: None,
            ud_scan_counter: 0,
            ud_progressive_counter: -12,
            effect_skip: false,
            native_effect_next: 0,
            native_effect_index: 0,
            moves: vec![],
            shaking: None,
            geometry: None,
            rotation: None,
            warp: None,
            stage_underlay: None,
            clear_underlay: false,
            raster: None,
            filter: None,
            fall: None,
            gauges: vec![],
            mono: 0,
            mono_next: None,
            toning: None,
            next_image: 1,
            declared_images: BTreeSet::new(),
            sprites: vec![],
            button_capacity: 0,
            buttons: BTreeMap::new(),
            windows: vec![],
            current_window: 0,
            selections: vec![],
            selection_paints: vec![],
            selection_focus: None,
            backlog: backlog::default_window(),
            backlog_view: None,
            wait: None,
            mode: 0,
            save_enabled: false,
            load_enabled: false,
            complete_effect_gates: false,
            skip_enabled: false,
            font_size: 20,
            font_color: [255; 4],
            font_style: [0; 2],
            title: String::new(),
            memo: String::new(),
            audio: None,
            audio_setup: false,
            last_music: String::new(),
            last_music_loop: true,
            last_env: String::new(),
            last_env_loop: true,
            native_audio: crate::native_storage::slot::SlotAudio {
                bgm_mode: 0,
                bgm_gain: 0,
                bgm: String::new(),
                se_mode: 0,
                se: String::new(),
                env_mode: 0,
                env: String::new(),
            },
            settings_closing: false,
            recollection: None,
            gallery: None,
            bgm_mode: None,
            end_mode: None,
            menu_audio: vec![],
            voice_group: String::new(),
            history: Default::default(),
            published: false,
            text_published: false,
            text: String::new(),
            native_text: Default::default(),
            pending_text: None,
            text_remainder: String::new(),
            novel_body: String::new(),
            caption: String::new(),
            last_voice_suffix: String::new(),
            pending_voice_suffix: String::new(),
            text_glyphs: 0,
            reveal_until: 0,
            white: None,
            clear_color: [0, 0, 0, 255],
            movie: false,
            desktop_request: None,
            staged_batches: None,
            restore_clock_origin: None,
            published_layers: vec![],
            published_ui: vec![],
            window_layers: vec![],
            window_ui: Default::default(),
            error: None,
            api_calls: 0,
        })
    }
    /// Remove commands for the listed layers that currently exist.
    fn removals(&self, h: &mut dyn HostServices, ids: &[u32]) -> Result<Vec<Json>> {
        let mut out = Vec::new();
        for id in ids {
            let spec: Option<Json> = serde_json::from_slice(&h.query_engine(
                self.client,
                crysirror_core::service::EngineQuery::Layer(*id),
            )?)?;
            if spec.is_some_and(|v| !v.is_null()) {
                out.push(json!({"op":"remove","id":id}));
            }
        }
        Ok(out)
    }
    fn submit(&mut self, h: &mut dyn HostServices, layers: Vec<Json>, ui: Vec<Json>) -> Result<()> {
        for command in &layers {
            if command["op"] == "transition" {
                let duration = command["transition"]["duration_ms"].as_u64().unwrap_or(0);
                self.transition_until = self
                    .transition_until
                    .max(h.now_ms().saturating_add(duration).saturating_add(1));
            }
        }
        let bytes = serde_json::to_vec(&json!({"layers":layers,"ui":ui}))?;
        if let Some(batches) = &mut self.staged_batches {
            ensure!(batches.len() < 128, "SugarPot detached scene batch budget");
            batches.push(crysirror_core::native_restore::OriginalRestoreBatch {
                schema: 1,
                bytes: bytes.into(),
            });
        } else {
            h.submit_engine(self.client, 1, &bytes)?;
        }
        self.api_calls += 1;
        Ok(())
    }
    fn begin_effect_gates(&mut self) {
        if self.mode == 0 {
            self.save_enabled = false;
            self.load_enabled = false;
            self.complete_effect_gates = true;
        }
    }
    fn finish_effect_gates(&mut self) {
        if self.complete_effect_gates {
            if self.mode == 0 {
                self.save_enabled = true;
                self.load_enabled = true;
            }
            self.complete_effect_gates = false;
        }
    }
    fn number(&self, a: &[String], i: usize, default: i32) -> Result<i32> {
        match a.get(i) {
            Some(s) => self.machine.as_ref().unwrap().number(s),
            None => Ok(default),
        }
    }
    fn value(&self, a: &[String], i: usize) -> Result<Value> {
        self.machine
            .as_ref()
            .unwrap()
            .value(a.get(i).context("SugarPot missing argument")?)
    }
    fn string(&self, a: &[String], i: usize) -> Result<String> {
        Ok(self.value(a, i)?.string()?.into())
    }
    fn resolve(&self, bank: &str, name: &str, extensions: &[&str]) -> Result<String> {
        ensure!(
            !name.is_empty() && !name.contains(['\\', '/', ':']) && name.len() <= 128,
            "SugarPot resource basename"
        );
        let name = name.to_ascii_lowercase();
        if name.contains('.') {
            return self
                .config
                .assets
                .get(&format!("{bank}/{name}"))
                .cloned()
                .with_context(|| format!("SugarPot missing resource {bank}/{name}"));
        }
        for ext in extensions {
            if let Some(path) = self.config.assets.get(&format!("{bank}/{name}.{ext}")) {
                return Ok(path.clone());
            }
        }
        bail!("SugarPot missing resource {bank}/{name} ({extensions:?})")
    }
    fn read(&mut self, path: &str) -> Result<Option<Arc<[u8]>>> {
        if let Some(read) = self.reading.as_mut() {
            ensure!(read.path == path, "SugarPot read continuation mismatch");
            if let Some(bytes) = read.job.try_take()? {
                self.reading = None;
                return Ok(Some(bytes));
            }
            return Ok(None);
        }
        let storage = self.storage.as_ref().unwrap().clone();
        let source = path.to_owned();
        self.reading = Some(Read {
            path: path.into(),
            job: self
                .pool
                .as_ref()
                .unwrap()
                .spawn(move |cancel| crate::resources::read(storage, source, &cancel))?,
        });
        Ok(None)
    }
    fn image(&mut self, h: &mut dyn HostServices, name: &str) -> Result<Option<UploadedImage>> {
        let path = self.resolve("grp", name, &["abm", "bmp", "dat"])?;
        if self.animation_load.is_some() {
            return self.load_animation(h, &path);
        }
        if let Some(image) = self.images.get(&path) {
            return Ok(Some(image.clone()));
        }
        if self.images.len() >= 48 && self.decoded.is_none() {
            let mut retained = self.declared_images.clone();
            for asset in self.animations.values() {
                for image in &asset.frames {
                    retained.insert(image.path.relative().into());
                }
            }
            for image in self.sprites.iter().filter_map(|s| s.image.as_ref()).chain(
                self.buttons
                    .values()
                    .filter_map(|b| b.sprite.image.as_ref()),
            ) {
                retained.insert(image.path.relative().into());
            }
            let retired = self
                .images
                .iter()
                .filter(|(_, image)| !retained.contains(image.path.relative()))
                .map(|(key, image)| (key.clone(), image.path.relative().to_owned()))
                .collect::<Vec<_>>();
            for (key, resource) in retired {
                h.release_image_resource(self.client, &resource)?;
                self.images.remove(&key);
            }
        }
        if let Some(image) = self.decoded.as_mut() {
            ensure!(image.path == path, "SugarPot image continuation mismatch");
            match image.job.poll()? {
                ImageProcessState::Pending => return Ok(None),
                ImageProcessState::Ready(resource) => {
                    ensure!(
                        self.images.len() < 128,
                        "SugarPot resident image count budget"
                    );
                    let image = h.publish_image_resource(self.client, self.next_image, resource)?;
                    self.next_image = self
                        .next_image
                        .checked_add(1)
                        .context("SugarPot image ID exhausted")?;
                    self.decoded = None;
                    if let Some(depth) = self.depths.get(&path).copied() {
                        self.depths.insert(image.path.relative().into(), depth);
                    }
                    self.images.insert(path, image.clone());
                    return Ok(Some(image));
                }
                ImageProcessState::Failed(e) => bail!("SugarPot image preparation failed: {e}"),
            }
        }
        let Some(bytes) = self.read(&path)? else {
            return Ok(None);
        };
        let depth = bytes
            .get(28..30)
            .map(|v| i16::from_le_bytes(v.try_into().unwrap()))
            .context("SugarPot image depth header")?;
        self.depths.insert(path.clone(), depth);
        if matches!(depth, 1 | 3) {
            self.begin_animation(path, bytes)?;
            return Ok(None);
        }
        self.decoded = Some(Image {
            path: path.clone(),
            job: h.image_metadata(self.client)?.begin_encoded(&path, bytes)?,
        });
        Ok(None)
    }
    fn tone_image(
        &mut self,
        h: &mut dyn HostServices,
        image: UploadedImage,
    ) -> Result<Option<UploadedImage>> {
        if self.mono == 0 || matches!(self.depths.get(image.path.relative()), Some(8 | -8)) {
            return Ok(Some(image));
        }
        if let Some((path, job)) = &mut self.toning {
            ensure!(
                path == image.path.relative(),
                "SugarPot tone continuation mismatch"
            );
            match job.poll()? {
                ImageProcessState::Pending => return Ok(None),
                ImageProcessState::Failed(e) => bail!("SugarPot mono preparation: {e}"),
                ImageProcessState::Ready(resource) => {
                    let prepared =
                        h.publish_image_resource(self.client, self.next_image, resource)?;
                    self.next_image = self
                        .next_image
                        .checked_add(1)
                        .context("SugarPot image ID exhausted")?;
                    if let Some(depth) = self.depths.get(image.path.relative()).copied() {
                        self.depths.insert(prepared.path.relative().into(), depth);
                    }
                    self.toning = None;
                    self.toned.insert(prepared.path.relative().into());
                    return Ok(Some(prepared));
                }
            }
        }
        // Retire toned copies no sprite, button, animation or declaration
        // still uses (the same retention rule as the decoded-image cache).
        if self.toned.len() >= 16 {
            let mut retained = self.declared_images.clone();
            for asset in self.animations.values() {
                for frame in &asset.frames {
                    retained.insert(frame.path.relative().into());
                }
            }
            for used in self.sprites.iter().filter_map(|s| s.image.as_ref()).chain(
                self.buttons.values().filter_map(|b| b.sprite.image.as_ref()),
            ) {
                retained.insert(used.path.relative().into());
            }
            retained.insert(image.path.relative().into());
            let retired: Vec<String> =
                self.toned.iter().filter(|p| !retained.contains(*p)).cloned().collect();
            for path in retired {
                h.release_image_resource(self.client, &path)?;
                self.depths.remove(&path);
                self.toned.remove(&path);
            }
        }
        let resource = h
            .published_image_resource(self.client, image.path.relative())?
            .context("SugarPot mono source resource")?;
        let effects = serde_json::to_vec(&crate::tone::effects(self.mono))?.into();
        self.toning = Some((
            image.path.relative().into(),
            h.image_metadata(self.client)?
                .begin_process(ImageInput::Resource(resource), effects)?,
        ));
        Ok(None)
    }
    fn mono(&mut self, h: &mut dyn HostServices, args: &[String]) -> Result<bool> {
        if self.mono_next.is_none() {
            self.mono = self.number(args, 0, 0)?.wrapping_add(1);
            if self.mono == 0 {
                return Ok(true);
            } // Native disable is not restoration.
            self.mono_next = Some(0);
        }
        while let Some(index) = self.mono_next {
            let Some(sprite) = self.sprites.get(index) else {
                self.mono_next = None;
                return Ok(true);
            };
            if sprite.system != 0 || sprite.image.is_none() {
                self.mono_next = Some(index + 1);
                continue;
            }
            if let Some(state) = sprite.animation.clone() {
                let Some(state) = self.tone_animation(h, format!("sprite:{index}"), state)? else {
                    return Ok(false);
                };
                let cursor = state.clock.frame(h.now_ms())? as usize;
                self.sprites[index].image = Some(
                    state.asset.frames[state.asset.programs[state.selected][cursor] as usize]
                        .clone(),
                );
                self.sprites[index].animation = Some(state);
                self.mono_next = Some(index + 1);
                continue;
            }
            let image = sprite.image.clone().unwrap();
            let Some(image) = self.tone_image(h, image)? else {
                return Ok(false);
            };
            self.sprites[index].image = Some(image);
            self.mono_next = Some(index + 1);
        }
        Ok(true)
    }
    fn boot(&mut self, h: &mut dyn HostServices) -> Result<bool> {
        if self.storage.is_none() {
            self.storage = Some(h.storage(self.client)?);
            if self.pool.is_none() {
                self.pool = Some(TaskPool::new("sugarpot-resources", 1, 4)?);
            }
        }
        if let Some(origin) = self.restore_clock_origin.take() {
            let delta = h.now_ms().saturating_sub(origin);
            self.reveal_until = self.reveal_until.saturating_add(delta);
            if let Some(Wait::RestoreTransition { deadline, .. }) = &mut self.wait {
                *deadline = deadline.saturating_add(delta);
            }
            for sprite in &mut self.sprites {
                if let Some(animation) = &mut sprite.animation {
                    animation.clock.started_ms = animation.clock.started_ms.saturating_add(delta);
                }
            }
        }
        if self.machine.is_some() {
            return Ok(true);
        }
        if self.boot.is_none() {
            let storage = self.storage.as_ref().unwrap().clone();
            let config = self.config.clone();
            let client = self.client;
            self.boot =
                Some(self.pool.as_ref().unwrap().spawn(move |cancel| {
                    crate::resources::boot(storage, config, client, &cancel)
                })?);
        }
        let Some(boot) = self.boot.as_mut().unwrap().try_take()? else {
            return Ok(false);
        };
        self.font_size = boot.settings.values[0];
        self.settings = boot.settings;
        self.globals = boot
            .globals
            .into_iter()
            .map(|binding| (binding.name, native_value(binding.value)))
            .collect();
        self.read_flags = boot.read_flags;
        self.machine = Some(Machine::new(boot.programs, &self.config.script)?);
        self.boot = None;
        Ok(true)
    }
    fn mark_read(&mut self) -> Result<()> {
        let f = &self.machine.as_ref().unwrap().state.frame;
        //454A80 keys are original filename, section and last ^if/^sw context.
        let name = format!("{},{},{}", f.native_script, f.section, f.context);
        let pc = u32::try_from(f.pc + 1)?;
        if let Some(flag) = self.read_flags.iter_mut().find(|flag| flag.name == name) {
            self.current_read = flag.bits > pc;
            if flag.bits < pc {
                flag.bits = pc;
            }
        } else {
            ensure!(self.read_flags.len() < 21845, "SugarPot read flag budget");
            self.current_read = false;
            self.read_flags.push(crate::native_storage::ReadFlag {
                name,
                kind: 37,
                bits: pc,
            });
        }
        Ok(())
    }
    fn persist_preferences(&mut self, kind: &str) -> Result<bool> {
        if let Some((pending, job)) = &mut self.native_write {
            ensure!(pending == kind, "SugarPot preference write continuation");
            if job.try_take()?.is_none() {
                return Ok(false);
            }
            self.native_write = None;
            return Ok(true);
        }
        let system = self.settings.encode()?;
        let flags = crate::native_storage::encode_read_flags(&self.read_flags)?;
        let storage = self.storage.as_ref().unwrap().clone();
        let task = self.pool.as_ref().unwrap().spawn(move |cancel| {
            crate::native_storage::write_file(
                storage.clone(),
                &VfsPath::new(Mount::Save, "system.dat")?,
                &system,
                &cancel,
            )?;
            crate::native_storage::write_file(
                storage,
                &VfsPath::new(Mount::Save, "readal.dat")?,
                &flags,
                &cancel,
            )
        })?;
        self.native_write = Some((kind.into(), task));
        Ok(false)
    }
    fn global(&mut self, args: &[String]) -> Result<bool> {
        if let Some(task) = &mut self.saving_globals {
            if task.try_take()?.is_none() {
                return Ok(false);
            }
            self.saving_globals = None;
            return Ok(true);
        }
        let declaration = args.first().context("SugarPot global name")?.trim();
        let kind = declaration.chars().next().context("SugarPot global type")?;
        ensure!(matches!(kind, '!' | '%' | '$'), "SugarPot global type");
        let key = &declaration[1..];
        ensure!(
            !key.is_empty() && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'),
            "SugarPot global identifier"
        );
        let old = self.globals.get(key).cloned();
        let requested = args
            .get(1)
            .map(|v| self.machine.as_ref().unwrap().value(v))
            .transpose()?;
        // Native sentinel is raw DWORD FFFFFFFF (float NaN), also produced by
        // the ordinary integer spelling -1. It queries without writing.
        let query = matches!(requested, Some(Value::Integer(-1)))
            || matches!(requested, Some(Value::Float(n)) if n.to_bits() == u32::MAX);
        let machine = self.machine.as_mut().unwrap();
        machine
            .state
            .variables
            .insert("reg".into(), old.clone().unwrap_or(Value::Integer(-1)));
        if query {
            return Ok(true);
        }
        let value = requested.unwrap_or_else(|| match kind {
            '$' => Value::String(String::new()),
            '!' => Value::Float(0.),
            _ => Value::Integer(0),
        });
        ensure!(
            matches!(
                (kind, &value),
                ('$', Value::String(_)) | ('!', Value::Float(_)) | ('%', Value::Integer(_))
            ),
            "SugarPot global value type"
        );
        self.globals.insert(key.to_owned(), value);
        self.write_globals()?;
        Ok(false)
    }
    fn write_globals(&mut self) -> Result<()> {
        let bindings = self
            .globals
            .iter()
            .map(|(name, v)| {
                Ok(crate::native_storage::Binding {
                    name: name.clone(),
                    value: stored_value(v)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let bytes = crate::native_storage::encode_globals(&bindings)?;
        let storage = self.storage.as_ref().unwrap().clone();
        let path = VfsPath::new(Mount::Save, "global.dat")?;
        self.saving_globals = Some(self.pool.as_ref().unwrap().spawn(move |cancel| {
            crate::native_storage::write_file(storage, &path, &bytes, &cancel)
        })?);
        Ok(())
    }
    fn native_image_blend(depth: i16, attribute: i32) -> crysirror_scene::layer::Blend {
        let attribute = attribute as u8;
        if matches!(depth, 24 | 32) {
            crysirror_scene::layer::Blend::StoredByte {
                mode: if attribute == 1 {
                    crysirror_image::rgb_lut::ConstantByteBlend::Add
                } else if depth == 32 {
                    crysirror_image::rgb_lut::ConstantByteBlend::Scalar256
                } else {
                    crysirror_image::rgb_lut::ConstantByteBlend::OpaqueScalar256
                },
                force_source_opaque: attribute == 1,
                //Original presenter consumes XRGB. Resource alpha remains
                //stored on immutable ABM images; only the stage is opaque.
                force_output_opaque: true,
            }
        } else {
            crysirror_scene::layer::Blend::Normal
        }
    }
    fn asset_image_blend(&self, image: &UploadedImage) -> crysirror_scene::layer::Blend {
        Self::native_image_blend(
            self.depths.get(image.path.relative()).copied().unwrap_or(0),
            0,
        )
    }
    fn sprite_layer(id: u32, s: &Sprite, order: i32) -> Result<Option<Json>> {
        let Some(image) = &s.image else {
            return Ok(None);
        };
        if !s.visible {
            return Ok(None);
        }
        ensure!(
            s.divisions > 0 && image.size[1] % s.divisions == 0 && s.cell < s.divisions,
            "SugarPot image cell layout"
        );
        let height = image.size[1] / s.divisions;
        //43F580's24-bit ATR flag is consulted only for sprite0. The32-bit
        //43D6C0 dispatch uses1:add RGB,2:black silhouette,3:white silhouette;
        //411220/43C850 ignore both caller opacity and SYS2's override.
        let attribute = if s.native_depth == 24 && id != 1 {
            0
        } else {
            s.attribute as u8 as i32
        };
        let mut layer = json!({"op":"put","layer":{"id":id,"rect":[s.position[0],s.position[1],image.size[0],height],"order":[order,0],"blend":Self::native_image_blend(s.native_depth,attribute),"sampling":{"linear":false},"uv":[0.,s.cell as f32/s.divisions as f32,1.,(s.cell+1) as f32/s.divisions as f32],"paint":{"kind":"image","asset":image.path.relative()}}});
        if s.native_depth == 32 && matches!(attribute, 2 | 3) {
            layer["layer"]["filter"] =
                json!({"multiply":[0.,0.,0.],"invert":if attribute==3 {1.}else{0.}});
        }
        Ok(Some(layer))
    }
    fn sprite_opacity(sprite: &Sprite, background: bool, opacity: i32) -> i32 {
        let attribute = sprite.attribute as u8;
        if (sprite.native_depth == 32 && matches!(attribute, 1..=3))
            || (sprite.native_depth == 24 && background && attribute == 1)
        {
            255
        } else {
            opacity
        }
    }
    fn publish(&mut self, h: &mut dyn HostServices, transition: Option<(Json, u64)>) -> Result<()> {
        self.clear_window_controls(h)?;
        //A script mutation can precede its UD presentation. Keep the prior
        //underlay lease until its last published layer has been replaced.
        let retiring_underlay = if self.clear_underlay {
            self.stage_underlay.take()
        } else {
            None
        };
        self.clear_underlay = false;
        let mut layers = vec![];
        let mut ui = vec![];
        for id in &self.published_layers {
            layers.push(json!({"op":"remove","id":id}));
        }
        for id in &self.published_ui {
            ui.push(json!({"op":"remove","id":id}));
        }
        let underlay = self
            .stage_underlay
            .as_ref()
            .filter(|_| !self.sprites.first().is_some_and(|sprite| sprite.visible));
        let paint = underlay.map_or_else(
            || json!({"kind":"solid","color":self.clear_color}),
            |image| json!({"kind":"image","asset":image.path.relative()}),
        );
        layers.push(json!({"op":"put","layer":{"id":20001,"rect":[0,0,1024,600],"order":[-1,0],"paint":paint}}));
        let system_visible = self
            .windows
            .get(self.current_window)
            .is_some_and(|window| window.visibility == 1);
        for (id, s) in self.sprites.iter().enumerate() {
            //4383B0/433310 keep SYS1/2 tied to the visible native text
            //window. The base sprite0 remains an explicit exception.
            if id != 0 && s.system != 0 && !system_visible {
                continue;
            }
            if let Some(mut layer) = Self::sprite_layer(
                id as u32 + 1,
                s,
                if id == 0 {
                    0
                } else if s.system != 0 {
                    2000 + id as i32
                } else {
                    100 + id as i32
                },
            )? {
                if s.system & 0x7f == 2 {
                    layer["layer"]["opacity"] = json!(Self::sprite_opacity(
                        s,
                        id == 0,
                        255 * self.settings.values[10].clamp(0, 100) / 100
                    ));
                }
                layers.push(layer);
            }
        }
        for (id, b) in &self.buttons {
            if let Some(layer) = Self::sprite_layer(1000 + *id as u32, &b.sprite, 1000 + *id)? {
                layers.push(layer);
                let image = b.sprite.image.as_ref().unwrap();
                ui.push(json!({"op":"put","widget":{"id":1000+*id as u32,"rect":[b.sprite.position[0],b.sprite.position[1],image.size[0],image.size[1]/b.sprite.divisions],"z":1000+*id,"kind":"button","interaction_only":true,"enabled":b.enabled,"visible":true}}));
                if let Some((text, offset)) = &b.label {
                    let options = crysirror_text::grid::CellLayoutOptions { cell_widths: Vec::new(),
                        bounds: [32767, 32767],
                        size: self.font_size,
                        spacing: [0, 0],
                        half_width: true,
                        half_spacing: false,
                        hanging: 0,
                        forbidden_line_start: String::new(),
                        indent_pairs: vec![],
                    };
                    let layout = crysirror_text::grid::layout_cells(
                        text,
                        &options,
                        &mut crysirror_text::grid::CellCursor::at_start(),
                    )?;
                    let glyphs = layout.glyphs.iter().map(|g|json!({"character":g.character,"baseline":[g.position[0]+offset[0],g.position[1]+offset[1]+self.font_size]})).collect::<Vec<_>>();
                    layers.push(json!({"op":"put","layer":{"id":2000+*id as u32,"rect":[b.sprite.position[0],b.sprite.position[1],image.size[0],image.size[1]/b.sprite.divisions],"order":[1000+*id,1],"paint":{"kind":"glyph_run","font":self.config.font,"size":self.font_size,"color":self.font_color,"glyphs":glyphs}}}));
                }
            }
        }
        let show_window = self
            .windows
            .get(self.current_window)
            .is_some_and(|w| w.visibility == 1);
        if show_window {
            layers.extend(self.window_layers.clone());
        }
        if let Some(color) = self.white {
            layers.push(json!({"op":"put","layer":{"id":20000,"rect":[0,0,1024,600],"order":[3000,0],"paint":{"kind":"solid","color":color}}}));
        }
        if let Some((effect, duration)) = transition {
            if duration > 0 {
                layers.push(json!({"op":"transition","transition":{"effect":effect,"duration_ms":duration}}));
            }
        }
        let layer_ids = layers
            .iter()
            .filter_map(|v| v["layer"]["id"].as_u64().map(|n| n as u32))
            .collect();
        let ui_ids = ui
            .iter()
            .filter_map(|v| v["widget"]["id"].as_u64().map(|n| n as u32))
            .collect();
        self.submit(h, layers, ui)?;
        if let Some(image) = retiring_underlay {
            h.release_image_resource(self.client, image.path.relative())?;
        }
        self.published_layers = layer_ids;
        self.published_ui = ui_ids;
        self.published = true;
        self.text_published = show_window;
        Ok(())
    }
    fn stop_audio(&mut self, h: &mut dyn HostServices, channel: u32, fade: i32) -> Result<()> {
        ensure!(fade >= 0, "SugarPot negative audio fade");
        h.submit_audio(
            self.client,
            AudioRequest::Stop {
                channel,
                fade_ms: fade as f32,
            },
        )?;
        Ok(())
    }
    fn play_audio(
        &mut self,
        h: &mut dyn HostServices,
        name: &str,
        channel: u32,
        bus: PcmBus,
        looping: bool,
    ) -> Result<bool> {
        if let Some(ticket) = self.audio {
            let state = h.audio_preparation(self.client, ticket)?;
            match state.phase {
                AudioPreparePhase::Pending => return Ok(false),
                AudioPreparePhase::Failed => bail!(
                    "SugarPot shared audio failed: {}",
                    state.error.unwrap_or_default()
                ),
                AudioPreparePhase::Ready => {
                    h.release_audio(self.client, ticket)?;
                    self.audio = None;
                    return Ok(true);
                }
            }
        }
        let enabled = match bus {
            PcmBus::Music => self.settings.header[1],
            PcmBus::Voice => self.settings.header[2],
            _ => self.settings.header[3],
        };
        if enabled != 1 {
            return Ok(true);
        }
        let gain = self.native_gain(bus)?;
        let path = self.resolve(
            match bus {
                PcmBus::Music => "bgm",
                PcmBus::Voice => "cv",
                _ => "se",
            },
            name,
            &["ogg", "wav"],
        )?;
        let Some(bytes) = self.read(&path)? else {
            return Ok(false);
        };
        self.audio = Some(h.begin_audio(
            self.client,
            AudioRequest::Play {
                channel,
                source: EncodedAudio {
                    bytes,
                    frame_limit: None,
                },
                intro: None,
                parameters: AudioParameters {
                    bus,
                    looping,
                    gain,
                    ..Default::default()
                },
            },
        )?);
        Ok(false)
    }
    fn declaration(
        &mut self,
        h: &mut dyn HostServices,
        name: &str,
        args: &[String],
    ) -> Result<bool> {
        let values = args
            .iter()
            .map(|s| self.machine.as_ref().unwrap().value(s))
            .collect::<Result<Vec<_>>>()?;
        match name {
            "spr" => {
                ensure!(args.len() == 1, "SugarPot spr allocation signature");
                let n = self.number(args, 0, 0)?;
                ensure!((1..=256).contains(&n), "SugarPot sprite allocation limit");
                ensure!(
                    self.sprites.is_empty(),
                    "SugarPot repeated sprite allocation"
                );
                self.sprites = vec![
                    Sprite {
                        divisions: 1,
                        ..Default::default()
                    };
                    n as usize
                ];
            }
            "btn" if args.len() == 1 => {
                let n = self.number(args, 0, 0)?;
                ensure!((1..=256).contains(&n), "SugarPot button allocation limit");
                ensure!(
                    self.button_capacity == 0,
                    "SugarPot repeated button allocation"
                );
                self.button_capacity = n as usize;
            }
            "ch" | "bg" => {
                let id = if name == "bg" {
                    0
                } else {
                    self.number(args, 0, 0)?
                };
                let selector = match self.number(args, 1, 0)? {
                    1 => "size",
                    2 => "pos",
                    4 => "sys",
                    5 => "atr",
                    6 => "pnt",
                    n => bail!("SugarPot {name} selector {n}"),
                };
                ensure!(
                    name != "bg" || matches!(selector, "size" | "pos"),
                    "SugarPot bg unsupported selector"
                );
                let rest = 2;
                let x = self.number(args, rest, 0)?;
                let y = self.number(args, rest + 1, 0)?;
                let ids: Vec<_> = if id == -1 {
                    (1..self.sprites.len()).collect()
                } else {
                    vec![usize::try_from(id)?]
                };
                if selector == "size" {
                    return self.allocate_sprite_buffers(h, &ids, [x, y]);
                }
                for id in ids {
                    let s = self
                        .sprites
                        .get_mut(id)
                        .context("SugarPot sprite ID outside allocation")?;
                    match selector {
                        "pos" => s.position = [x as i16 as i32, y as i16 as i32],
                        "sys" => s.system = if matches!(x, 1..=3) { x } else { 0 },
                        "pnt" => s.origin = if matches!(x, 1 | 2) { x } else { 0 },
                        "atr" => s.attribute = if matches!(x, 1..=3) { x } else { 0 },
                        _ => bail!("unsupported SugarPot {name} selector {selector}"),
                    }
                }
            }
            "wnd" => {
                if args.len() == 1 {
                    let n = self.number(args, 0, 0)?;
                    ensure!((1..=16).contains(&n), "SugarPot window allocation limit");
                    ensure!(
                        self.windows.is_empty(),
                        "SugarPot repeated window allocation"
                    );
                    self.windows = (0..n).map(|_| Window::default()).collect();
                } else {
                    let id = self.number(args, 0, 0)?;
                    let selector = match self.number(args, 1, 0)? {
                        1 => "size",
                        2 => "pos",
                        4 => "txt",
                        5 => "cel",
                        6 => "frm",
                        7 => "fp",
                        8 => "pp",
                        9 => "btn",
                        n => bail!("SugarPot wnd selector {n}"),
                    }
                    .to_owned();
                    ensure!(
                        ["size", "pos", "txt", "cel", "frm", "fp", "pp", "btn"]
                            .contains(&selector.as_str()),
                        "unsupported SugarPot wnd selector {selector}"
                    );
                    if id == -1 && selector == "btn" {
                        //446220's ALL button branch only scans its sentinel.
                        return Ok(true);
                    }
                    let mut prepared_size = None;
                    if ["frm", "fp", "pp", "btn"].contains(&selector.as_str()) {
                        let image = values
                            .get(2)
                            .context("SugarPot wnd image parameter")?
                            .string()?;
                        if !image.is_empty() {
                            let Some(image) = self.image(h, image)? else {
                                return Ok(false);
                            };
                            prepared_size = Some(image.size);
                            self.declared_images.insert(image.path.relative().into());
                        }
                        if selector == "btn" {
                            if let Some(Value::String(overlay)) = values.get(6) {
                                if !overlay.is_empty() {
                                    let Some(image) = self.image(h, overlay)? else {
                                        return Ok(false);
                                    };
                                    self.declared_images.insert(image.path.relative().into());
                                }
                            }
                        }
                    }
                    let blend = self.settings.values[10];
                    let ids = if id == -1 {
                        (0..self.windows.len()).collect::<Vec<_>>()
                    } else {
                        vec![usize::try_from(id)?]
                    };
                    for index in ids {
                        let w = self
                            .windows
                            .get_mut(index)
                            .context("SugarPot window outside allocation")?;
                        storage::set_window(
                            &mut w.native,
                            &selector,
                            &values[2..],
                            prepared_size,
                            blend,
                        )?;
                        if selector == "txt" {
                            w.text_cursor = [w.native.fields[12], w.native.fields[13]];
                            w.text_cell = [0, 0];
                        }
                        if selector == "btn" {
                            if values
                                .get(2)
                                .context("SugarPot window button resource")?
                                .string()?
                                .is_empty()
                            {
                                w.buttons.clear();
                            } else {
                                w.buttons.push(values[2..].to_vec());
                            }
                        } else {
                            w.properties.insert(selector.clone(), values[2..].to_vec());
                        }
                    }
                }
            }
            "wbl" => return self.configure_backlog(h, args, &values),
            _ => bail!("unsupported SugarPot declaration {name}"),
        }
        Ok(true)
    }
    fn visual(&mut self, h: &mut dyn HostServices, name: &str, args: &[String]) -> Result<bool> {
        if name == "cgon" {
            if let Some(task) = &mut self.saving_globals {
                if task.try_take()?.is_none() {
                    return Ok(false);
                }
                self.saving_globals = None;
                return Ok(true);
            }
        }
        match name {
            "bgon" | "cgon" | "chon" => {
                let id = if name == "chon" {
                    usize::try_from(self.number(args, 0, 0)?)?
                } else {
                    0
                };
                if name == "chon" && self.sprites.get(id).context("SugarPot sprite ID")?.system == 3
                {
                    self.sprites[id].visible = false;
                    return Ok(true);
                }
                let n = self.string(args, if name == "chon" { 1 } else { 0 })?;
                let Some(image) = self.image(h, &n)? else {
                    return Ok(false);
                };
                let mut animation = self.animation_state(&n, h.now_ms())?;
                let image = if self.sprites.get(id).context("SugarPot sprite ID")?.system == 0 {
                    if let Some(state) = animation.take() {
                        let Some(state) = self.tone_animation(h, n.clone(), state)? else {
                            return Ok(false);
                        };
                        let image = state.asset.frames[state.asset.programs[0][0] as usize].clone();
                        animation = Some(state);
                        image
                    } else {
                        let Some(image) = self.tone_image(h, image)? else {
                            return Ok(false);
                        };
                        image
                    }
                } else {
                    image
                };
                let x = self.number(
                    args,
                    if name == "chon" { 2 } else { 1 },
                    if name == "chon" { 65523 } else { 0 },
                )?;
                let y = self.number(args, if name == "chon" { 3 } else { 2 }, 0)?;
                let division = self.number(args, if name == "chon" { 5 } else { 3 }, 0)? + 1;
                let cell = self.number(args, if name == "chon" { 4 } else { 4 }, 0)?;
                ensure!(cell >= 0 && division > 0, "SugarPot native sprite cell");
                let native_depth = self.depths.get(image.path.relative()).copied().unwrap_or(0);
                let s = self
                    .sprites
                    .get_mut(id)
                    .context("SugarPot sprite outside allocation")?;
                s.animation = animation;
                let mut pos = if name == "chon" {
                    match x {
                        65520 => 256,
                        65521 => 512,
                        65522 => 767,
                        65523 => s.position[0],
                        n => n,
                    }
                } else {
                    x
                };
                if x != 65523 {
                    pos -= match s.origin {
                        1 => image.size[0] as i32 / 2,
                        2 => image.size[0] as i32,
                        _ => 0,
                    };
                }
                s.position = [pos as i16 as i32, y as i16 as i32];
                s.native_depth = native_depth;
                s.image = Some(image);
                s.source_name = n.clone();
                s.visible = true;
                s.cell = cell as u32;
                s.divisions = division as u32;
                if id == 0 {
                    self.clear_stage_underlay();
                }
                if name == "cgon" {
                    //44AF02 looks up the original argument in global.dat,
                    //not a synthetic per-VM cg namespace.
                    self.globals.insert(n, Value::Integer(1));
                    self.write_globals()?;
                    return Ok(false);
                }
            }
            "choff" => {
                let id = self.number(args, 0, 0)?;
                if id == 0 {
                    self.clear_stage_underlay();
                }
                if id == -1 {
                    for s in self.sprites.iter_mut().skip(1) {
                        if s.system == 0 {
                            s.visible = false;
                        }
                    }
                } else {
                    self.sprites
                        .get_mut(usize::try_from(id)?)
                        .context("SugarPot choff ID")?
                        .visible = false;
                }
            }
            "bgoff" | "cgoff" => {
                self.clear_stage_underlay();
                self.sprites
                    .get_mut(0)
                    .context("SugarPot background not allocated")?
                    .visible = false;
            }
            "btnset" => {
                let id = self.number(args, 0, 0)?;
                ensure!(
                    id >= 0 && (id as usize) < self.button_capacity,
                    "SugarPot btnset outside allocation"
                );
                let source = self.string(args, 1)?;
                let Some(image) = self.image(h, &source)? else {
                    return Ok(false);
                };
                ensure!(
                    image.size[1] % 3 == 0,
                    "SugarPot native three-cell button sheet"
                );
                let label = if args.len() > 4 {
                    let text = self.string(args, 4)?;
                    if text.is_empty() {
                        None
                    } else {
                        Some((text, [self.number(args, 5, 0)?, self.number(args, 6, 0)?]))
                    }
                } else {
                    None
                };
                let sprite = Sprite {
                    native_depth: self.depths.get(image.path.relative()).copied().unwrap_or(0),
                    source_name: source,
                    image: Some(image),
                    position: [
                        self.number(args, 2, 0)? as i16 as i32,
                        self.number(args, 3, 0)? as i16 as i32,
                    ],
                    visible: true,
                    divisions: 3,
                    ..Default::default()
                };
                self.buttons.insert(
                    id,
                    Button {
                        sprite,
                        label,
                        enabled: false,
                        state: 0,
                    },
                );
            }
            "btnon" | "btnoff" | "btnreset" => {
                let id = self.number(args, 0, 0)?;
                if name == "btnreset" && id == -1 {
                    self.right_button = -1;
                }
                for (n, b) in &mut self.buttons {
                    if id == -1 || id == *n {
                        if name == "btnon" {
                            b.sprite.cell = 0;
                        } else {
                            b.sprite.visible = false;
                            b.enabled = false;
                            b.label = None;
                        }
                    }
                }
            }
            "ud" => {
                self.begin_effect_gates();
                let kind = self.number(args, 0, 0)?;
                if matches!(kind, 1 | 2 | 10..=13 | 15..=39) {
                    let ready = self.mask_update(h, args)?;
                    if ready {
                        self.finish_effect_gates();
                    }
                    return Ok(ready);
                }
                ensure!(
                    !matches!(kind, 19..=39),
                    "unsupported SugarPot update effect {kind}"
                );
                self.white = None;
                if kind != 0 {
                    for w in &mut self.windows {
                        w.visibility = 2;
                    }
                }
                //449870's reviewed default (3..9,14 and other values) uses
                //native state5: instant target copy with all windows state2.
                self.clear_stage_underlay();
                self.publish(h, None)?;
                if kind == 0
                    && self
                        .windows
                        .get(self.current_window)
                        .is_some_and(|window| window.visibility == 1)
                    && !self.native_text.committed.is_empty()
                {
                    let raw = self.native_text.committed.clone();
                    let waiting = self.wait.take();
                    let ready = self.text_inner(h, &raw, false)?;
                    self.wait = waiting;
                    if !ready {
                        return Ok(false);
                    }
                }
                self.finish_effect_gates();
            }
            "wout" | "bout" | "win" | "bin" => {
                self.begin_effect_gates();
                let n = self.number(args, 0, 1000)?;
                let duration = if n == 0 { 1000 } else { n };
                ensure!(duration > 0, "SugarPot fade duration");
                self.white = if name.ends_with("out") {
                    Some(if name.starts_with('w') {
                        [255; 4]
                    } else {
                        [0, 0, 0, 255]
                    })
                } else {
                    None
                };
                for w in &mut self.windows {
                    w.visibility = 2;
                }
                self.publish(h, Some((json!({"kind":"crossfade"}), duration as u64)))?;
                self.wait = Some(Wait::Clock(h.now_ms().saturating_add(duration as u64)));
            }
            "wndoff" | "wndon" => {
                let duration = self.number(args, 0, 100)?;
                let duration = if duration == 0 { 100 } else { duration };
                ensure!(duration > 0, "SugarPot native window fade duration");
                let showing = name == "wndon";
                if self.windows.get(self.current_window).is_none_or(|w| {
                    if showing {
                        w.visibility != 2
                    } else {
                        w.visibility != 1
                    }
                }) {
                    return Ok(true);
                }
                self.begin_effect_gates();
                self.clear_window_controls(h)?;
                if showing && self.window_layers.is_empty() {
                    self.window_layers =
                        self.window_background(&self.windows[self.current_window])?;
                }
                let mut layers = if showing {
                    self.window_layers.clone()
                } else {
                    self.published_layers
                        .iter()
                        .copied()
                        .filter(|id| (30000..40000).contains(id))
                        .map(|id| json!({"op":"remove","id":id}))
                        .collect::<Vec<_>>()
                };
                layers.extend(self.system_window_layers(showing)?);
                layers.push(json!({"op":"transition","transition":{"effect":{"kind":"crossfade"},"duration_ms":duration}}));
                self.submit(h, layers, vec![])?;
                self.published_layers
                    .retain(|id| !(30000..40000).contains(id));
                if showing {
                    self.published_layers.extend(
                        self.window_layers
                            .iter()
                            .filter_map(|v| v["layer"]["id"].as_u64().map(|v| v as u32)),
                    );
                }
                self.windows[self.current_window].visibility = if showing { 1 } else { 2 };
                self.track_system_window_layers(showing, false);
                self.text_published = showing;
                self.wait = Some(Wait::Clock(h.now_ms().saturating_add(duration as u64)));
            }
            _ => bail!("unsupported SugarPot visual command {name}"),
        }
        Ok(true)
    }
    fn window_background(&self, window: &Window) -> Result<Vec<Json>> {
        //441FF0/441DE0 copy the bare stage through the CEL restoration
        //rectangle.4422A0 then draws FRM; it never draws an invented solid
        //RGBA veil. Window.byte64 is retained by454ED0 but not consumed by
        //this profile's drawing path (unlike sprite SYS2's global4654 weight).
        let Some(frame) = &window.native.frame else {
            return Ok(vec![]);
        };
        let path = self.resolve("grp", &frame.image, &["abm", "bmp", "dat"])?;
        let image = self
            .images
            .get(&path)
            .context("SugarPot declared window frame not prepared")?;
        let f = window.native.fields;
        Ok(vec![
            json!({"op":"put","layer":{"id":30001,"rect":[f[1]+frame.position[0],f[2]+frame.position[1],image.size[0],image.size[1]],
            "order":[3001,0],"blend":self.asset_image_blend(image),"sampling":{"linear":false},
            "paint":{"kind":"image","asset":image.path.relative()}}}),
        ])
    }
    fn text(&mut self, h: &mut dyn HostServices, source: &str) -> Result<bool> {
        self.text_inner(h, source, true)
    }
    fn text_inner(&mut self, h: &mut dyn HostServices, source: &str, record: bool) -> Result<bool> {
        let machine = self.machine.as_ref().unwrap();
        let frame = &machine.state.frame;
        let append_page = record
            && self.mode != 2
            && machine.programs[&frame.script]
                .instructions
                .get(frame.pc + 1)
                .is_some_and(|i| i.source.is_empty());
        let prepared = if record {
            crate::text::prepare(source, machine, append_page)?
        } else {
            crate::text::prepare_display(source, machine)?
        };
        self.render_text(h, prepared, record, false)
    }
    fn render_text(
        &mut self,
        h: &mut dyn HostServices,
        prepared: crate::text::Prepared,
        record: bool,
        continuation: bool,
    ) -> Result<bool> {
        let line = self.machine.as_ref().unwrap().state.frame.pc + 1;
        let stop = prepared.stop;
        let boxed_speaker = self
            .windows
            .get(self.current_window)
            .is_some_and(|w| w.native.frame.is_some());
        let name_image =
            if boxed_speaker && (prepared.caption.is_some() || !self.caption.is_empty()) {
                //40AA70 loads this separate GRP resource. Admit it before playing
                //a c cue so an image-worker retry cannot replay that same voice.
                let Some(image) = self.image(h, "namewnd1.abm")? else {
                    return Ok(false);
                };
                self.declared_images
                    .insert(image.path.relative().to_owned());
                Some(image)
            } else {
                None
            };
        if let Some(voice) = &prepared.voice {
            if !self.play_audio(h, voice, 2, PcmBus::Voice, false)? {
                return Ok(false);
            }
        }
        let rebuilding_inline = record && self.native_text.inline_clear != 0;
        let append_native_line =
            record && !self.native_text.committed.is_empty() && self.native_text.inline_clear == 0;
        if record {
            if self.mode == 0 {
                self.save_enabled = false;
                self.load_enabled = false;
            }
            self.native_text.begin(&prepared, line)?;
            self.window_ui.auto_units = prepared.units;
            self.capture_history_voice(&prepared.native)?;
        }
        let retaining = continuation || append_native_line;
        let (layout_native, layout_limit) = if retaining {
            self.native_text.layout_segment()?
        } else if record {
            self.native_text.layout_segment()?
        } else {
            let length = encoding_rs::SHIFT_JIS.encode(&prepared.native).0.len();
            (prepared.native.clone(), length)
        };
        if let Some(caption) = prepared.caption {
            self.caption = caption;
        }
        let source = prepared.body;
        self.clear_window_controls(h)?;
        let w = self
            .windows
            .get(self.current_window)
            .context("SugarPot text window not allocated")?;
        let pos = w
            .properties
            .get("pos")
            .context("SugarPot text window position missing")?;
        let txt = w
            .properties
            .get("txt")
            .context("SugarPot text window layout missing")?;
        let size = w
            .properties
            .get("size")
            .context("SugarPot text window size missing")?;
        let n = |v: &[Value], i: usize, default: i32| -> Result<i32> {
            v.get(i).map_or(Ok(default), Value::integer)
        };
        let window = [n(pos, 0, 0)?, n(pos, 1, 0)?, n(size, 0, 1)?, n(size, 1, 1)?];
        let origin = [n(txt, 0, 0)?, n(txt, 1, 0)?];
        let spacing = [n(txt, 3, 0)?, n(txt, 2, 0)?];
        let rows = n(txt, 4, 1)?;
        let columns = n(txt, 5, 1)?;
        ensure!(rows > 0 && columns > 0, "SugarPot empty text window layout");
        let speaker_count = source
            .strip_prefix('[')
            .and_then(|s| s.split_once(']'))
            .map_or(0, |(speaker, _)| speaker.chars().count());
        let mut body = if let Some(rest) = source.strip_prefix('[') {
            let (name, body) = rest
                .split_once(']')
                .context("SugarPot unterminated speaker")?;
            format!("{name}\n{body}")
        } else {
            source.clone()
        };
        if continuation {
            body = if speaker_count > 0 {
                format!("{}\n{body}", self.text)
            } else {
                format!("{}{body}", self.text)
            };
        } else if append_native_line {
            body = format!("{}\n{body}", self.text);
        }
        let (body_page, explicit_remainder) = body.split_once('\u{000c}').unwrap_or((&body, ""));
        let options = crysirror_text::grid::CellLayoutOptions { cell_widths: Vec::new(),
            bounds: [
                //Native columns decide wrapping; the shared group's signed
                //advance may be zero or negative. Surface width is a clipping
                //bound, not a second scalar-width wrap policy.
                window[2],
                //The native row counter can reset at a node boundary without
                //resetting its WORD Y position. Native break hints perform
                //cursor resets; a second geometric cyclic limit would differ.
                32767,
            ],
            size: self.font_size,
            spacing,
            half_width: true,
            half_spacing: false,
            hanging: 0,
            forbidden_line_start: String::new(),
            indent_pairs: vec![],
        };
        let previous = if retaining || (rebuilding_inline && boxed_speaker) {
            w.text_paint.clone()
        } else {
            vec![]
        };
        let previous_count = previous.len();
        let mut cursor = crysirror_text::grid::CellCursor::at_start();
        if retaining {
            cursor.position = [w.text_cursor[0] - origin[0], w.text_cursor[1] - origin[1]];
        }
        let mut prefix_cells = Vec::new();
        let mut initial_cell = if continuation { w.text_cell } else { [0, 0] };
        if rebuilding_inline && !boxed_speaker {
            //44F4D9 supplies62CC directly to43C010 (EAX), not43B3F0.
            //Thus an old '[' is body data here. A separate62D8 name is drawn
            //first; a framed window instead copies its prior immutable pixels
            //back and preserves the old per-glyph decoration above.
            if !self.caption.is_empty() {
                let header = format!("[{}]", self.caption);
                prefix_cells = crate::text::cells::groups(
                    &header,
                    encoding_rs::SHIFT_JIS.encode(&header).0.len(),
                    self.font_size,
                    spacing[0],
                    columns,
                    rows,
                    crate::text::cells::Profile::Live,
                    false,
                    [0, 0],
                )?
                .cells;
            }
            let old = crate::text::cells::groups(
                &self.native_text.committed,
                encoding_rs::SHIFT_JIS
                    .encode(&self.native_text.committed)
                    .0
                    .len(),
                self.font_size,
                spacing[0],
                columns,
                rows,
                crate::text::cells::Profile::Body,
                false,
                [0, 0],
            )?;
            initial_cell = [old.column, old.row];
            prefix_cells.extend(old.cells);
        }
        let rebuilt_glyphs = prefix_cells
            .iter()
            .map(|cell| cell.glyphs.len())
            .sum::<usize>();
        let mut grouped = crate::text::cells::groups(
            &layout_native,
            layout_limit,
            self.font_size,
            spacing[0],
            columns,
            rows,
            if !record && !continuation {
                crate::text::cells::Profile::Redraw
            } else {
                crate::text::cells::Profile::Live
            },
            boxed_speaker,
            initial_cell,
        )?;
        prefix_cells.append(&mut grouped.cells);
        grouped.cells = prefix_cells;
        let layout =
            crysirror_text::grid::layout_cell_groups(&grouped.cells, &options, &mut cursor, false)?;
        ensure!(
            layout.consumed_groups == grouped.cells.len(),
            "SugarPot native grouped page cannot admit its cells"
        );
        self.text_remainder = explicit_remainder.to_owned();
        self.novel_body = if self.mode == 2 && self.text_remainder.is_empty() {
            body_page.to_owned()
        } else {
            String::new()
        };
        let mut glyphs=layout.glyphs.iter().map(|g|json!({"character":g.character,"baseline":[g.position[0]+origin[0],g.position[1]+origin[1]+self.font_size]})).collect::<Vec<_>>();
        let mut layers = vec![];
        for id in self
            .published_layers
            .iter()
            .copied()
            .filter(|id| (30000..40000).contains(id))
        {
            layers.push(json!({"op":"remove","id":id}));
        }
        let mut ids = vec![];
        layers.extend(self.system_window_layers(true)?);
        let background = self.window_background(w)?;
        ids.extend(
            background
                .iter()
                .filter_map(|layer| layer["layer"]["id"].as_u64().map(|id| id as u32)),
        );
        layers.extend(background);
        if let Some(image) = &name_image {
            //43C560's framed-name path resets the body cursor to TXT origin;
            //the label occupies the independently sized NameWnd, not a body
            //line. Its original byte-pair count determines centering.
            let groups =
                crate::text::cells::name_groups(&self.caption, self.font_size, spacing[0])?;
            let width = encoding_rs::SHIFT_JIS.encode(&self.caption).0.len() / 2;
            let x = image.size[0] as i32 / 2
                - ((width as i32 * (self.font_size + spacing[0])) >> 1)
                + 1;
            let y = image.size[1] as i32 / 2
                - ((self.font_size + self.font_style[0] as u8 as i32) >> 1)
                + 1;
            let mut name_cursor = crysirror_text::grid::CellCursor::at_start();
            let name_layout = crysirror_text::grid::layout_cell_groups(
                &groups,
                &options,
                &mut name_cursor,
                false,
            )?;
            ensure!(
                name_layout.consumed_groups == groups.len(),
                "SugarPot native name cell budget"
            );
            let shadow = self.font_style[0] as u8 as i32;
            let glyphs=name_layout.glyphs.iter().map(|g|json!({"character":g.character,
                "baseline":[x+g.position[0],y+g.position[1]+self.font_size],
                "style":{"shadow":if shadow==0 {Json::Null}else{json!({"offset":[shadow,shadow],
                    "color":[self.font_style[1]as u8,(self.font_style[1]>>8)as u8,(self.font_style[1]>>16)as u8,255]})}}})).collect::<Vec<_>>();
            layers.push(json!({"op":"put","layer":{"id":30004,"rect":[window[0]+1,window[1],image.size[0],image.size[1]],
                "order":[3002,0],"blend":self.asset_image_blend(image),"sampling":{"linear":false},
                "paint":{"kind":"image","asset":image.path.relative()}}}));
            layers.push(json!({"op":"put","layer":{"id":30005,"rect":[window[0],window[1],image.size[0],image.size[1]],
                "order":[3003,1],"paint":{"kind":"glyph_run","font":self.config.font,"size":self.font_size,
                "color":self.font_color,"glyphs":glyphs}}}));
            ids.extend([30004, 30005]);
        }
        let interval = match self.settings.header[8] {
            0 => 90,
            1 => 45,
            2 => 16,
            _ => 0,
        };
        //439F00 draws the speaker label before character pacing. A p/c
        //continuation retains every previously visible glyph on the same page.
        let initial = (if !record && !continuation {
            previous_count + glyphs.len()
        } else if retaining || rebuilding_inline {
            previous_count
        } else if record && !self.native_text.committed.is_empty() {
            self.text_glyphs
        } else {
            0
        })
        .saturating_add(rebuilt_glyphs)
        .saturating_add(if boxed_speaker {
            0
        } else if source.starts_with('[') {
            speaker_count + 2
        } else {
            0
        })
        .min(previous_count + glyphs.len());
        //CellLayout returns one positioned Unicode scalar per glyph; native
        //439F00 consumes one DBCS character or two single-byte characters.
        //Map those native cells onto actual layout glyph-prefix boundaries,
        //never interpret an encoded byte offset as a positioned-glyph index.
        let mut prefix_steps = Vec::new();
        let mut next = previous_count;
        for cell in &grouped.cells {
            next += cell.glyphs.len();
            if next > initial && prefix_steps.last() != Some(&next) {
                prefix_steps.push(next);
            }
        }
        let reveal =
            json!({"interval_ms":interval,"initial_visible":initial,"prefix_steps":prefix_steps});
        self.reveal_until = h
            .now_ms()
            .saturating_add(prefix_steps.len() as u64 * interval);
        let shadow_offset = self.font_style[0] as u8 as i32;
        if shadow_offset != 0 {
            //Per-glyph engine decoration preserves draw order when a cyclic
            //window revisits an older cell. A separate whole-run shadow pass
            //would incorrectly place every old face above each newer shadow.
            for glyph in &mut glyphs {
                glyph["style"] = json!({"shadow":{"offset":[shadow_offset,shadow_offset],"color":[self.font_style[1] as u8,(self.font_style[1]>>8) as u8,(self.font_style[1]>>16) as u8,255]}});
            }
        }
        let text_color = if self.read_all && self.current_read {
            [168, 255, 255, 255]
        } else {
            self.font_color
        };
        for glyph in &mut glyphs {
            glyph["font"] = json!(self.config.font);
            glyph["size"] = json!(self.font_size);
            glyph["color"] = json!(text_color);
            if glyph.get("style").is_none() {
                glyph["style"] = json!({});
            }
        }
        let mut retained = previous;
        retained.append(&mut glyphs);
        let glyphs = retained;
        ensure!(
            glyphs.len() <= 65536,
            "SugarPot retained native text glyph budget"
        );
        layers.push(json!({"op":"put","layer":{"id":30003,"rect":window,"order":[3003,0],"paint":{"kind":"glyph_run","font":self.config.font,"size":self.font_size,"color":text_color,"glyphs":glyphs,"reveal":reveal}}}));
        ids.push(30003);
        self.window_layers = layers
            .iter()
            .filter(|v| {
                v["op"] == "put"
                    && v["layer"]["id"]
                        .as_u64()
                        .is_some_and(|id| (30000..40000).contains(&id))
            })
            .cloned()
            .collect();
        self.submit(h, layers, vec![])?;
        self.published_layers
            .retain(|id| !(30000..40000).contains(id));
        self.published_layers.extend(ids);
        self.track_system_window_layers(true, false);
        self.windows[self.current_window].visibility = 1;
        let local_cursor = [
            origin[0] + cursor.position[0],
            origin[1] + cursor.position[1],
        ];
        let prompt_cursor = if grouped.column >= columns {
            [origin[0], local_cursor[1] + self.font_size + spacing[1]]
        } else {
            local_cursor
        };
        let native_window = &mut self.windows[self.current_window].native;
        if record || continuation {
            if matches!(stop, crate::text::Stop::Line | crate::text::Stop::Prompt) {
                if let Some(pp) = &mut native_window.page {
                    pp.fields[0] = prompt_cursor[0];
                    pp.fields[1] = prompt_cursor[1];
                }
            }
            if stop == crate::text::Stop::Page {
                if let Some(fp) = &mut native_window.foreground {
                    if fp.fields[3] == -1 {
                        fp.fields[0] = prompt_cursor[0];
                        fp.fields[1] = prompt_cursor[1];
                    }
                }
            }
        }
        self.windows[self.current_window].text_cursor = local_cursor;
        self.windows[self.current_window].text_cell = [grouped.column, grouped.row];
        if !record && !continuation && grouped.column >= columns {
            //43B3F0 has an additional terminal redraw wrap. It does not
            //rewrite the saved FP/PP positions restored by4561A0.
            let next = grouped.row + 1;
            self.windows[self.current_window].text_cursor =
                if next >= rows { origin } else { prompt_cursor };
            self.windows[self.current_window].text_cell = [0, if next >= rows { 0 } else { next }];
        }
        self.windows[self.current_window].text_paint = glyphs.clone();
        if record && !continuation {
            self.window_ui.auto_start = h.now_ms();
        }
        if record || continuation {
            self.window_ui.epoch = self.reveal_until;
        }
        self.text = body_page.into();
        self.text_glyphs = glyphs.len();
        self.text_published = true;
        //Even i waits until its preceding glyphs finish. It differs from a
        //page only at the completed native boundary, not at text publication.
        self.wait = Some(Wait::Text);
        Ok(true)
    }
    fn complete_native_text(&mut self) -> Result<()> {
        let advance = self.native_text.advances_line_on_completion();
        self.native_text.complete()?;
        if advance {
            self.advance_native_line()?;
        }
        Ok(())
    }
    fn capture_history_voice(&mut self, native: &str) -> Result<()> {
        let override_suffix = crate::text::voice_suffix_override(native)?;
        self.last_voice_suffix =
            override_suffix.unwrap_or_else(|| self.pending_voice_suffix.clone());
        let (bytes, _, invalid) = encoding_rs::SHIFT_JIS.encode(&self.last_voice_suffix);
        ensure!(
            !invalid && bytes.len() < 32,
            "SugarPot native history voice field CP932/32-byte bound"
        );
        self.pending_voice_suffix.clear();
        self.history_record(native)?;
        Ok(())
    }
    fn advance_native_line(&mut self) -> Result<()> {
        let window = self
            .windows
            .get_mut(self.current_window)
            .context("SugarPot completed text window")?;
        let f = window.native.fields;
        //439F00's NUL branch tests old row < rows, unlike explicit n's
        //postincrement >= rows. Keep its legitimate extra clipped row;
        //the WORD Y and BYTE row counter remain separate state.
        window.text_cursor[0] = f[12];
        window.text_cell[0] = 0;
        if window.text_cell[1] < f[16] {
            window.text_cell[1] += 1;
            window.text_cursor[1] =
                window.text_cursor[1].wrapping_add(self.font_size + f[14]) as i16 as i32;
        } else {
            window.text_cell[1] = 0;
            window.text_cursor[1] = f[13];
        }
        Ok(())
    }
    fn poll_wait(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
    ) -> Result<Option<VmYield>> {
        let Some(wait) = self.wait.clone() else {
            return Ok(None);
        };
        if let Some(prepared) = self.pending_text.clone() {
            if self.render_text(h, prepared, false, true)? {
                self.pending_text = None;
            }
            return Ok(Some(VmYield::BudgetExhausted));
        }
        let skipping = self.skip_enabled
            && (input.buttons_down & buttons::CTRL != 0 || self.window_ui.skip)
            && (self.settings.header[10] == 0 || self.current_read);
        let mut advance = skipping
            || input.buttons_pressed & (buttons::ENTER | buttons::SPACE | buttons::MOUSE_LEFT) != 0;
        match wait {
            Wait::Backlog => {
                self.poll_backlog(h, input)?;
                return Ok(Some(VmYield::BudgetExhausted));
            }
            Wait::Slots => {
                self.poll_slot_menu(h, input)?;
                return Ok(Some(VmYield::BudgetExhausted));
            }
            Wait::OriginalSave => {
                self.wait = self.poll_native_save(h, input)?.map(|wait| *wait);
                return Ok(Some(VmYield::BudgetExhausted));
            }
            Wait::OriginalLoad => {
                self.wait = self.poll_native_load(h, input)?.map(|wait| *wait);
                return Ok(Some(VmYield::BudgetExhausted));
            }
            Wait::RestoreTransition {
                deadline,
                resume,
                show_window,
            } => {
                if h.now_ms() < deadline {
                    return Ok(Some(VmYield::WaitUntil(deadline)));
                }
                if matches!(resume.as_deref(), Some(Wait::Selection)) {
                    let selections = self.selections.clone();
                    self.restore_choices(h, &selections)?;
                } else if show_window {
                    let mut layers = self.window_layers.clone();
                    layers.extend(self.system_window_layers(true)?);
                    self.submit(h, layers, vec![])?;
                    self.published_layers.extend(
                        self.window_layers
                            .iter()
                            .filter_map(|layer| layer["layer"]["id"].as_u64().map(|id| id as u32)),
                    );
                    if let Some(window) = self.windows.get_mut(self.current_window) {
                        window.visibility = 1;
                    }
                    self.track_system_window_layers(true, false);
                }
                self.wait = resume.map(|wait| *wait);
                return Ok(Some(VmYield::BudgetExhausted));
            }
            Wait::Clock(deadline) => {
                if h.now_ms() < deadline {
                    return Ok(Some(VmYield::WaitUntil(deadline)));
                }
            }
            Wait::Input | Wait::Text => {
                if matches!(wait, Wait::Text)
                    && h.now_ms() >= self.reveal_until
                    && self.text_remainder.is_empty()
                {
                    self.complete_native_text()?;
                    if self.mode == 0 && self.native_text.waiting == 16 {
                        self.save_enabled = true;
                        self.load_enabled = true;
                    }
                }
                if matches!(wait, Wait::Text)
                    && self.text_remainder.is_empty()
                    && self.native_text.has_automatic_boundary()
                {
                    if let Some(prepared) = self.native_text.continuation() {
                        self.pending_text = Some(prepared);
                    } else {
                        self.wait = None;
                    }
                    return Ok(Some(VmYield::BudgetExhausted));
                }
                if self.poll_window_controls(h, input)? {
                    return Ok(Some(VmYield::BudgetExhausted));
                }
                if input.buttons_pressed & (buttons::WHEEL_UP | buttons::PAGE_UP) != 0
                    && self.open_backlog(h)?
                {
                    input.buttons_pressed = 0;
                    return Ok(Some(VmYield::BudgetExhausted));
                }
                let auto_delay = (1000i64 * self.settings.values[8] as i64)
                    .max(
                        1000i64 * self.settings.values[7] as i64
                            + self.window_ui.auto_units as i64 * self.settings.values[6] as i64,
                    )
                    .max(0) as u64;
                if self.settings.values[5] == 1
                    && matches!(self.native_text.waiting, 8 | 16)
                    && (self.settings.values[9] == 1
                        || h.now_ms() >= self.window_ui.auto_start.saturating_add(auto_delay))
                    && !h.audio_playing(self.client, 2)?
                {
                    advance = true;
                }
                if input.buttons_pressed & buttons::MOUSE_RIGHT != 0 {
                    let showing = self
                        .windows
                        .get(self.current_window)
                        .is_none_or(|w| w.visibility != 1);
                    self.clear_window_controls(h)?;
                    let mut layers = if showing {
                        self.window_layers.clone()
                    } else {
                        self.published_layers
                            .iter()
                            .copied()
                            .filter(|id| (30000..40000).contains(id))
                            .map(|id| json!({"op":"remove","id":id}))
                            .collect()
                    };
                    layers.extend(self.system_window_layers(showing)?);
                    self.submit(h, layers, vec![])?;
                    if let Some(window) = self.windows.get_mut(self.current_window) {
                        window.visibility = u8::from(showing);
                    }
                    self.published_layers
                        .retain(|id| !(30000..40000).contains(id));
                    if showing {
                        self.published_layers.extend(
                            self.window_layers
                                .iter()
                                .filter_map(|v| v["layer"]["id"].as_u64().map(|id| id as u32)),
                        );
                    }
                    self.track_system_window_layers(showing, !showing);
                    input.buttons_pressed = 0;
                    return Ok(Some(VmYield::WaitForInput));
                }
                ensure!(
                    input.buttons_pressed & buttons::ESCAPE == 0,
                    "SugarPot native popup menu interaction not connected"
                );
                if !advance {
                    return Ok(Some(if self.animation_running() {
                        VmYield::BudgetExhausted
                    } else {
                        VmYield::WaitForInput
                    }));
                }
                if matches!(wait, Wait::Text) && h.now_ms() < self.reveal_until {
                    let layers = self
                        .published_layers
                        .iter()
                        .copied()
                        .filter(|id| matches!(id, 30002 | 30003))
                        .map(|id| json!({"op":"complete_reveal","id":id}))
                        .collect();
                    self.submit(h, layers, vec![])?;
                    self.reveal_until = h.now_ms();
                    if self.text_remainder.is_empty() {
                        self.complete_native_text()?;
                        if self.mode == 0 && self.native_text.waiting == 16 {
                            self.save_enabled = true;
                            self.load_enabled = true;
                        }
                    }
                    input.buttons_pressed = 0;
                    return Ok(Some(if self.native_text.has_automatic_boundary() {
                        VmYield::BudgetExhausted
                    } else {
                        VmYield::WaitForInput
                    }));
                }
                input.buttons_pressed &= !(buttons::ENTER | buttons::SPACE | buttons::MOUSE_LEFT);
                if matches!(wait, Wait::Text) && !self.text_remainder.is_empty() {
                    let remainder = std::mem::take(&mut self.text_remainder);
                    self.novel_body.clear();
                    ensure!(
                        self.text(h, &remainder)?,
                        "SugarPot paged text unexpected preparation"
                    );
                    self.native_text.invalidate_continuation();
                    return Ok(Some(VmYield::WaitForInput));
                }
                if matches!(wait, Wait::Text) {
                    self.suspend_window_controls(h)?;
                    if let Some(prepared) = self.native_text.continuation() {
                        self.pending_text = Some(prepared);
                        return Ok(Some(VmYield::BudgetExhausted));
                    }
                    if self.native_text.waiting == 16 {
                        //4306F0 clears the page immediately at confirmation,
                        //before another source node or asset can be prepared.
                        //419C remains the original work buffer;62CC and62D8
                        //are cleared, not reconstructed from display text.
                        let ids = [30002u32, 30003, 30004, 30005];
                        let present: Vec<_> = ids
                            .iter()
                            .filter(|id| self.published_layers.contains(id))
                            .map(|id| json!({"op":"remove","id":id}))
                            .collect();
                        self.submit(h, present, vec![])?;
                        self.published_layers.retain(|id| !ids.contains(id));
                        self.window_layers.retain(|layer| {
                            layer["layer"]["id"]
                                .as_u64()
                                .is_none_or(|id| !ids.contains(&(id as u32)))
                        });
                        let window = &mut self.windows[self.current_window];
                        window.text_cursor = [window.native.fields[12], window.native.fields[13]];
                        window.text_cell = [0, 0];
                        window.text_paint.clear();
                        self.caption.clear();
                        self.text.clear();
                        self.text_remainder.clear();
                        self.novel_body.clear();
                        self.text_glyphs = 0;
                    }
                    self.native_text.confirm();
                }
            }
            Wait::Motion {
                start,
                duration,
                kind,
            } => {
                let elapsed = h.now_ms().saturating_sub(start);
                self.motion_frame(h, elapsed, duration, kind)?;
                if elapsed < duration {
                    return Ok(Some(VmYield::BudgetExhausted));
                }
            }
            Wait::Raster { remaining, next } => {
                if self.raster.as_ref().is_some_and(|r| r.state != 0) && remaining > 0 {
                    if h.now_ms() >= next {
                        self.wait = Some(Wait::Raster {
                            remaining: remaining - 1,
                            next: h.now_ms().saturating_add(33),
                        });
                    }
                    return Ok(Some(VmYield::BudgetExhausted));
                }
            }
            Wait::EfMovement(motion) => {
                if !self.ef_movement_frame(h, &motion, skipping)? {
                    return Ok(Some(VmYield::BudgetExhausted));
                }
            }
            Wait::Geometry => {
                if !self.geometry_frame(h, skipping)? {
                    return Ok(Some(VmYield::BudgetExhausted));
                }
            }
            Wait::Rotation => {
                if !self.rotation_frame(h, skipping)? {
                    return Ok(Some(VmYield::BudgetExhausted));
                }
            }
            Wait::Warp => {
                if !self.warp_frame(h, skipping)? {
                    return Ok(Some(VmYield::BudgetExhausted));
                }
            }
            Wait::Shake(deadline) => {
                let finished = h.now_ms() >= deadline || skipping || self.settings.header[4] != 0;
                self.shake_frame(h, finished)?;
                if !finished {
                    return Ok(Some(VmYield::BudgetExhausted));
                }
            }
            Wait::AnimationNumber {
                id,
                target,
                deadline,
                next,
            } => {
                if h.now_ms() < next {
                    return Ok(Some(VmYield::WaitUntil(next)));
                }
                let animation = self.sprites[id]
                    .animation
                    .as_ref()
                    .context("SugarPot animation wait state")?;
                let cursor = animation.clock.frame(h.now_ms())? as usize;
                if animation.asset.programs[animation.selected][cursor]
                    == animation.asset.programs[target][0]
                    || h.now_ms() >= deadline
                {
                    self.select_animation(id, target, h.now_ms())?;
                } else {
                    self.wait = Some(Wait::AnimationNumber {
                        id,
                        target,
                        deadline,
                        next: h.now_ms().saturating_add(64),
                    });
                    return Ok(Some(VmYield::BudgetExhausted));
                }
            }
            Wait::Selection => {
                if input.buttons_pressed & (buttons::WHEEL_UP | buttons::PAGE_UP) != 0
                    && self.open_backlog(h)?
                {
                    input.buttons_pressed = 0;
                    return Ok(Some(VmYield::BudgetExhausted));
                }
                if self.poll_window_controls(h, input)? {
                    return Ok(Some(VmYield::BudgetExhausted));
                }
                if !self.poll_choices(h, input)? {
                    return Ok(Some(VmYield::BudgetExhausted));
                }
            }
            Wait::EndMode => {
                if !self.poll_end_mode(h, input)? {
                    return Ok(Some(VmYield::BudgetExhausted));
                }
            }
            Wait::Gallery => {
                if !self.poll_gallery(h, input)? {
                    return Ok(Some(VmYield::BudgetExhausted));
                }
            }
            Wait::BgmMode => {
                if !self.poll_bgm_mode(h, input)? {
                    return Ok(Some(VmYield::BudgetExhausted));
                }
            }
            Wait::Recollection => {
                if !self.poll_recollection(h, input)? {
                    return Ok(Some(VmYield::BudgetExhausted));
                }
            }
            Wait::Settings => {
                if !self.poll_settings(h, input)? {
                    return Ok(Some(VmYield::BudgetExhausted));
                }
                if let Some(resume) = self.window_ui.suspended.take() {
                    self.wait = Some(*resume);
                    return Ok(Some(VmYield::BudgetExhausted));
                }
            }
            Wait::Dialog => {
                let events: Vec<Json> = serde_json::from_slice(
                    &h.query_engine(self.client, EngineQuery::DialogEvents)?,
                )?;
                if !events.iter().any(|event| {
                    event["kind"] == "dialog_result"
                        && event["widget"]["id"] == 65500
                        && event["widget"]["owner"] == self.client.0
                }) {
                    return Ok(Some(VmYield::WaitForInput));
                }
                input.buttons_pressed = 0;
            }
            Wait::Movie => {
                let info = h.media_info(self.client, 0)?;
                match info.phase {
                    MediaPhase::Loading | MediaPhase::Playing => {
                        return Ok(Some(VmYield::BudgetExhausted));
                    }
                    MediaPhase::Failed => bail!("SugarPot movie shared decoder failed"),
                    MediaPhase::Ended | MediaPhase::Stopped => {
                        self.submit(h, vec![json!({"op":"remove","id":65000})], vec![])?;
                        h.media_request(self.client, MediaRequest::Remove { id: 0 })?;
                        self.movie = false;
                    }
                }
            }
            Wait::Audio(channel) => {
                if h.audio_playing(self.client, channel)? {
                    return Ok(Some(VmYield::BudgetExhausted));
                }
            }
            Wait::AudioStop(channel) => {
                if h.audio_info(self.client, channel)?
                    .is_some_and(|info| info.stopping)
                {
                    return Ok(Some(VmYield::BudgetExhausted));
                }
            }
            Wait::Buttons => {
                let mut selected = if input.buttons_pressed & buttons::MOUSE_RIGHT != 0
                    && self.right_button != -1
                {
                    Some(self.right_button)
                } else {
                    None
                };
                let mut layers = vec![];
                for (id, b) in &mut self.buttons {
                    if !b.sprite.visible || !b.enabled {
                        continue;
                    }
                    let state: Json = serde_json::from_slice(&h.query_engine(
                        self.client,
                        EngineQuery::UiInteraction(1000 + *id as u32),
                    )?)?;
                    let cell = if state["pressed"] == true {
                        2
                    } else if state["hovered"] == true || state["focused"] == true {
                        1
                    } else {
                        0
                    };
                    if b.state != cell {
                        b.state = cell;
                        b.sprite.cell = cell;
                        if let Some(layer) =
                            Self::sprite_layer(1000 + *id as u32, &b.sprite, 1000 + *id)?
                        {
                            layers.push(layer);
                        }
                    }
                    if state["activated"] == true {
                        selected = Some(*id);
                    }
                }
                if !layers.is_empty() {
                    self.submit(h, layers, vec![])?;
                }
                let Some(id) = selected else {
                    return Ok(Some(if self.animation_running() {
                        VmYield::BudgetExhausted
                    } else {
                        VmYield::WaitForInput
                    }));
                };
                let mut ui = vec![];
                for (n, b) in &mut self.buttons {
                    if b.sprite.visible && b.enabled {
                        b.enabled = false;
                        ui.push(json!({"op":"set_enabled","id":1000+*n as u32,"enabled":false}));
                    }
                }
                self.submit(h, vec![], ui)?;
                self.machine.as_mut().unwrap().set_reg(id);
                input.buttons_pressed = 0;
            }
        }
        self.wait = None;
        self.finish_effect_gates();
        Ok(None)
    }
    fn command(&mut self, h: &mut dyn HostServices, name: &str, args: &[String]) -> Result<bool> {
        if self.machine.as_mut().unwrap().value_command(name, args)? {
            return Ok(true);
        }
        match name {
            "var" => self.machine.as_mut().unwrap().declare(args)?,
            "spr" | "btn" | "wnd" | "wbl" | "ch" | "bg" => return self.declaration(h, name, args),
            "bgon" | "cgon" | "chon" | "bgoff" | "cgoff" | "choff" | "ud" | "wout" | "bout"
            | "win" | "bin" | "btnset" | "btnon" | "btnoff" | "btnreset" | "wndoff" | "wndon" => {
                return self.visual(h, name, args);
            }
            "title" => {
                let title = self.string(args, 0)?;
                self.submit(h, vec![], vec![json!({"op":"set_title","text":title})])?;
                self.title = title;
                self.memo.clear();
            }
            "memo" => {
                let memo = self.string(args, 0)?;
                self.submit(h,vec![],vec![json!({"op":"set_title","text":if memo.is_empty(){self.title.clone()}else{format!("{} {}",self.title,memo)}})])?;
                self.memo = memo;
            }
            "fnt" => {
                let size = self.number(args, 0, 20)?;
                ensure!((1..=512).contains(&size), "SugarPot font size");
                let color = self.number(args, 1, 0xffffff)? as u32;
                self.font_size = size;
                self.font_color = [color as u8, (color >> 8) as u8, (color >> 16) as u8, 255];
                self.settings.values[0] = size;
                self.font_style = [self.number(args, 2, 0)?, self.number(args, 3, 0)?];
            }
            "mode" => {
                let mode = self.number(args, 0, 0)?;
                ensure!((0..=3).contains(&mode), "SugarPot native mode");
                self.mode = mode;
            }
            "saveenable" => {
                if self.mode == 0 {
                    self.save_enabled = true;
                }
            }
            "savedisable" => {
                if self.mode == 0 {
                    self.save_enabled = false;
                }
            }
            "loadenable" => {
                if self.mode == 0 {
                    self.load_enabled = true;
                }
            }
            "loaddisable" => {
                if self.mode == 0 {
                    self.load_enabled = false;
                }
            }
            "skipenable" => self.skip_enabled = true,
            "skipdisable" => self.skip_enabled = false,
            "wait" => {
                let ms = self.number(args, 0, 0)?;
                ensure!(ms >= 0, "SugarPot negative wait");
                self.wait = Some(Wait::Clock(h.now_ms().saturating_add(ms as u64)));
            }
            "pause" => self.wait = Some(Wait::Input),
            "btnenable" => {
                if self.number(args, 0, 0)? != 0 {
                    return Ok(true);
                }
                let mut ui = vec![];
                let mut any = false;
                for (id, b) in &mut self.buttons {
                    if b.sprite.visible {
                        b.enabled = true;
                        any = true;
                        ui.push(json!({"op":"set_enabled","id":1000+*id as u32,"enabled":true}));
                    }
                }
                ensure!(any, "SugarPot button wait has no visible native buttons");
                self.submit(h, vec![], ui)?;
                self.wait = Some(Wait::Buttons);
            }
            "btndisable" => {
                let mut ui = vec![];
                for (id, b) in &mut self.buttons {
                    if b.sprite.visible {
                        b.enabled = false;
                        ui.push(json!({"op":"set_enabled","id":1000+*id as u32,"enabled":false}));
                    }
                }
                self.submit(h, vec![], ui)?;
            }
            "rand" => {
                let n = self.number(args, 0, 0)?;
                ensure!(n > 0, "SugarPot rand range must be positive");
                // 445DF0: srand(GetTickCount/timeGetTime), then rand() % n.
                let mut random =
                    crysirror_image::temporal_pattern::PatternRng::msvc(h.now_ms() as u32);
                let draw = random.next_draw()?;
                self.crt_rand = random.state;
                self.machine
                    .as_mut()
                    .unwrap()
                    .set_reg((draw % (n as u32)) as i32);
            }
            "cv" => {
                let group = self.string(args, 0)?;
                // 447A10: 450D40 finds the "cv" node; 450890 appends it to the
                // ordered bank when absent.
                let state = &mut self.machine.as_mut().unwrap().state;
                if !state.variables.contains_key("cv") {
                    state.variable_order.push("cv".into());
                }
                state.variables.insert("cv".into(), Value::String(group.clone()));
                self.voice_group = group;
            }
            "bgmon" | "envon" | "seon" | "cvon" => {
                if name == "bgmon" {
                    if let Value::Integer(value) = self.value(args, 0)? {
                        self.machine
                            .as_mut()
                            .unwrap()
                            .state
                            .variables
                            .insert("bgm".into(), Value::Integer(value));
                        return Ok(true);
                    }
                }
                let source = self.string(args, 0)?;
                let mode = if name == "cvon" {
                    0
                } else {
                    self.number(args, 1, 0)?
                };
                ensure!(
                    (0..=if name == "seon" { 2 } else { 1 }).contains(&mode),
                    "SugarPot native audio play mode"
                );
                let track = self.number(args, 2, 0)?.clamp(0, 15) as u32;
                let (channel, bus, looping) = match name {
                    "bgmon" => (0, PcmBus::Music, mode == 0),
                    "envon" => (1, PcmBus::Ambient, mode == 0),
                    "cvon" => (2, PcmBus::Voice, false),
                    _ => (
                        3 + if mode == 1 { 0 } else { track },
                        PcmBus::Sfx,
                        mode == 2,
                    ),
                };
                if name == "bgmon" && source == self.last_music {
                    return Ok(true);
                }
                //45B130 mode1 stops all sixteen SE tracks once before opening
                //its synchronous source on track0. Retrying a worker cannot
                //repeat that stop or erase another newly admitted source.
                if !self.audio_setup {
                    if name == "cvon" {
                        //447F40 writes String1 even if settings/skip suppress
                        //the audio transport. 44C510 consumes it at next text.
                        self.pending_voice_suffix = source.clone();
                    }
                    if name == "bgmon" {
                        self.machine
                            .as_mut()
                            .unwrap()
                            .state
                            .variables
                            .insert("bgm".into(), Value::String(source.clone()));
                    }
                    if name == "seon" && mode == 1 {
                        for channel in 3..19 {
                            self.stop_audio(h, channel, 0)?;
                        }
                    }
                    self.audio_setup = true;
                }
                if !self.play_audio(h, &source, channel, bus, looping)? {
                    return Ok(false);
                }
                self.audio_setup = false;
                if name == "bgmon" {
                    self.last_music = source.clone();
                    self.last_music_loop = looping;
                    self.native_audio.bgm_mode = if looping { 2 } else { 1 };
                    self.native_audio.bgm = source.clone();
                }
                if name == "envon" {
                    self.last_env = source.clone();
                    self.last_env_loop = looping;
                    self.native_audio.env_mode = if looping { 2 } else { 1 };
                    self.native_audio.env = source.clone();
                }
                if name == "seon" {
                    self.native_audio.se_mode = match mode {
                        1 => 3,
                        2 => 2,
                        _ => 1,
                    };
                    if self.settings.header[3] == 1 {
                        self.native_audio.se = source;
                    }
                }
                if name == "seon" && mode == 1 {
                    self.wait = Some(Wait::Audio(channel));
                }
            }
            "bgmoff" | "envoff" | "seoff" | "cvoff" => {
                if name == "seoff" {
                    let track = self.number(args, 0, 0)?;
                    if track == -1 {
                        for channel in 3..19 {
                            self.stop_audio(h, channel, 0)?;
                        }
                    } else {
                        ensure!((0..=15).contains(&track), "SugarPot SE stop track");
                        self.stop_audio(h, 3 + track as u32, 0)?;
                    }
                } else {
                    let channel = match name {
                        "bgmoff" => 0,
                        "envoff" => 1,
                        _ => 2,
                    };
                    let fade = if name != "cvoff" && self.number(args, 0, 0)? == 0 {
                        let n = self.number(args, 1, 1200)?;
                        if n == 0 { 1200 } else { n }
                    } else {
                        0
                    };
                    self.stop_audio(h, channel, fade)?;
                    if name == "bgmoff" {
                        self.last_music.clear();
                        self.native_audio.bgm_mode = 0;
                        self.native_audio.bgm.clear();
                        if let Some(binding) = self
                            .machine
                            .as_mut()
                            .unwrap()
                            .state
                            .variables
                            .get_mut("bgm")
                        {
                            *binding = match binding {
                                Value::Integer(_) => Value::Integer(0),
                                _ => Value::String(String::new()),
                            };
                        }
                    }
                    if name == "envoff" {
                        self.last_env.clear();
                        self.native_audio.env_mode = 0;
                        self.native_audio.env.clear();
                    }
                    if fade > 0 {
                        self.wait = Some(Wait::AudioStop(channel));
                    }
                }
            }
            "fill" => {
                self.clear_stage_underlay();
                let color = [
                    self.number(args, 0, 0)? as u8,
                    self.number(args, 1, 0)? as u8,
                    self.number(args, 2, 0)? as u8,
                    255,
                ];
                self.sprites
                    .get_mut(0)
                    .context("SugarPot background not allocated")?
                    .visible = false;
                self.clear_color = color;
            }
            "cls" => {
                let id = usize::try_from(self.number(args, 0, 0)?)?;
                self.windows.get(id).context("SugarPot cls window ID")?;
                if id != self.current_window {
                    let window = &mut self.windows[id];
                    window.text_cursor = [window.native.fields[12], window.native.fields[13]];
                    window.text_cell = [0, 0];
                    window.text_paint.clear();
                    return Ok(true);
                }
                self.clear_window_controls(h)?;
                let ids = self
                    .published_layers
                    .iter()
                    .copied()
                    .filter(|id| (30002..40000).contains(id))
                    .collect::<Vec<_>>();
                self.submit(
                    h,
                    ids.iter()
                        .map(|id| json!({"op":"remove","id":id}))
                        .collect(),
                    vec![],
                )?;
                self.published_layers.retain(|id| !ids.contains(id));
                self.window_layers.retain(|command| {
                    command["layer"]["id"]
                        .as_u64()
                        .is_none_or(|id| !(30002..40000).contains(&id))
                });
                self.text.clear();
                //4481B0 clears this window's pixels/cursor only. It does not
                //erase419C/62CC or replace the command's native source PC.
                self.text_remainder.clear();
                self.novel_body.clear();
                self.text_glyphs = 0;
                let window = &mut self.windows[id];
                window.text_cursor = [window.native.fields[12], window.native.fields[13]];
                window.text_cell = [0, 0];
                window.text_paint.clear();
            }
            "backtitle" => {
                if !self.persist_preferences("backtitle")? {
                    return Ok(false);
                }
                for channel in 0..19 {
                    self.stop_audio(h, channel, 0)?;
                }
                if self.movie {
                    h.media_request(self.client, MediaRequest::Remove { id: 0 })?;
                    self.movie = false;
                }
                let removals = self.removals(h, &[65000, 60000])?;
                self.submit(h, removals, vec![])?;
                self.history.clear();
                self.last_voice_suffix.clear();
                self.text.clear();
                self.novel_body.clear();
                self.text_remainder.clear();
                self.wait = None;
                self.moves.clear();
                self.mask_update = None;
                self.clear_stage_underlay();
                self.machine.as_mut().unwrap().return_title()?;
                self.submit(h, vec![], vec![json!({"op":"set_title","text":self.title})])?;
            }
            "bmp" => {
                if let Some((kind, job)) = &mut self.native_write {
                    ensure!(kind == "bmp", "SugarPot native write continuation");
                    if job.try_take()?.is_none() {
                        return Ok(false);
                    }
                    self.native_write = None;
                    return Ok(true);
                }
                let name = self.string(args, 0)?;
                let path = self.resolve("grp", &name, &["abm", "bmp"])?;
                let Some(bytes) = self.read(&path)? else {
                    return Ok(false);
                };
                let target = VfsPath::new(
                    Mount::Save,
                    &format!(
                        "bmp/{}",
                        path.rsplit('/')
                            .next()
                            .context("SugarPot bitmap export name")?
                    ),
                )?;
                let storage = self.storage.as_ref().unwrap().clone();
                let task = self.pool.as_ref().unwrap().spawn(move |cancel| {
                    if crate::native_storage::read_file(storage.clone(), &target, &cancel)?
                        .is_some()
                    {
                        return Ok(());
                    }
                    crate::native_storage::write_file(storage, &target, &bytes, &cancel)
                })?;
                self.native_write = Some(("bmp".into(), task));
                return Ok(false);
            }
            "debug" => {
                //443360 only evaluates/displays when the native debug binding
                //exists and its DWORD is nonzero; otherwise it skips the line.
                let active = self
                    .machine
                    .as_ref()
                    .unwrap()
                    .state
                    .variables
                    .get("dbg")
                    .map(Value::integer)
                    .transpose()?
                    .unwrap_or(0)
                    != 0;
                if active {
                    let value = self.value(args, 0)?;
                    let message = match value {
                        Value::String(s) => s,
                        Value::Integer(n) => n.to_string(),
                        Value::Float(n) => n.to_string(),
                        Value::List(_) => "0".into(),
                    };
                    self.submit(h,vec![],vec![json!({"op":"open_dialog","dialog":{"id":65500,"title":"Debug","message":message,"buttons":[{"id":1,"label":"OK"}],"default":1,"font":self.config.font}})])?;
                    self.wait = Some(Wait::Dialog);
                }
            }
            "float" => {
                let selector = self.number(args, 0, 0)?;
                match selector {
                    1 if self.save_enabled => self.start_slot_menu(false)?,
                    2 if self.load_enabled => self.start_slot_menu(true)?,
                    1 | 2 => {} //405520/4056E0 disabled native gates return0.
                    3 => return self.gallery(h),
                    4 => return self.recollection(h),
                    5 => return self.bgm_mode(h),
                    6 => return self.end_mode(h),
                    7 => {
                        if self.save_command_done {
                            self.save_command_done = false;
                        } else if self.save_enabled {
                            self.start_native_save(h, 100, true)?;
                            return Ok(false);
                        }
                    }
                    8 => {
                        if self.load_command_done {
                            self.load_command_done = false;
                        } else if self.load_enabled {
                            self.start_native_load(100, true, true)?;
                            return Ok(false);
                        }
                    }
                    9 => self.settings_menu(h)?,
                    11 => {
                        use crysirror_core::service::{
                            DesktopRequest, DesktopRequestState, DesktopResponse,
                        };
                        //445740 ShellExecute(open) has no reg/result assignment.
                        //Only the host's explicit HTTP(S) capability is admitted;
                        //arbitrary executable/file targets remain a visible error.
                        if let Some(job) = &mut self.desktop_request {
                            match job.poll()? {
                                DesktopRequestState::Pending => return Ok(false),
                                DesktopRequestState::Ready(response) => {
                                    ensure!(
                                        response == DesktopResponse::Opened,
                                        "SugarPot URL response"
                                    );
                                    self.desktop_request = None;
                                }
                            }
                        } else {
                            let request = DesktopRequest::OpenUrl {
                                url: self.string(args, 1)?,
                            };
                            request.validate()?;
                            self.desktop_request =
                                Some(h.begin_desktop_request(self.client, request)?);
                            return Ok(false);
                        }
                    }
                    //445740 has no case0/10 or other default action. Its
                    //unobserved host return is selector-1; no reg assignment,
                    //resource request, modal state or script transfer occurs.
                    _ => {}
                }
            }
            "anmnum" => {
                let id = usize::try_from(self.number(args, 0, 0)?)?;
                let requested = self.number(args, 1, 0)? as u16 as usize;
                let animation = self
                    .sprites
                    .get_mut(id)
                    .context("SugarPot anmnum ID")?
                    .animation
                    .as_mut()
                    .context("SugarPot anmnum requires animation")?;
                let target = requested.min(animation.asset.programs.len() - 1);
                if target != animation.selected {
                    animation.clock.frame_ms = 64;
                    self.wait = Some(Wait::AnimationNumber {
                        id,
                        target,
                        deadline: h.now_ms().saturating_add(9600),
                        next: h.now_ms().saturating_add(64),
                    });
                }
            }
            "gg" | "ggon" | "ggoff" => return self.gauge(h, name, args),
            "ef" => {
                self.begin_effect_gates();
                let ready = self.ef(h, args)?;
                if ready && self.wait.is_none() {
                    self.finish_effect_gates();
                }
                return Ok(ready);
            }
            "efex" => {
                let effect = matches!(self.number(args, 1, 0)?, 1..=3);
                if effect {
                    self.begin_effect_gates();
                }
                let ready = self.efex(h, args)?;
                if effect && ready && self.wait.is_none() {
                    self.finish_effect_gates();
                }
                return Ok(ready);
            }
            "movie" | "avi" => {
                ensure!(!self.movie, "SugarPot overlapping movie");
                if name == "movie" {
                    ensure!(
                        self.number(args, 1, 0)? == 0 && self.number(args, 2, 0)? == 0,
                        "SugarPot nondefault native movie flag profile"
                    );
                }
                let native = self.string(args, 0)?;
                ensure!(
                    !native.contains(['/', '\\', ':']) && !native.is_empty(),
                    "SugarPot movie basename"
                );
                let path = if name == "avi" {
                    format!("{native}.avi")
                } else if native.contains('.') {
                    format!("movie/{native}")
                } else {
                    format!("movie/{native}.mpg")
                };
                VfsPath::new(Mount::Asset, &path)?;
                let texture = h.media_request(
                    self.client,
                    MediaRequest::Play {
                        id: 0,
                        asset: path,
                        looping: false,
                        audio: true,
                        gain: 1.,
                    },
                )?;
                self.submit(h,vec![json!({"op":"put","layer":{"id":65000,"rect":[0,0,1024,600],"order":[6500,0],"paint":{"kind":"image","asset":texture.relative()}}})],vec![])?;
                self.movie = true;
                self.wait = Some(Wait::Movie);
            }
            "raster" => return self.raster_command(args, h),
            "filter" => return self.filter_command(h, args),
            "fall" => return self.fall_command(h, args),
            "mono" => return self.mono(h, args),
            "global" => return self.global(args),
            "readal" => self.read_all = self.number(args, 0, 0)? != 0,
            "rbtn" => self.right_button = self.number(args, 0, 0)?,
            "clrbacklog" => {
                self.history.clear();
                self.last_voice_suffix.clear();
                self.text.clear();
                self.native_text.clear();
                self.text_remainder.clear();
                self.novel_body.clear();
            }
            "exit" => {
                if !self.persist_preferences("exit")? {
                    return Ok(false);
                }
                self.machine.as_mut().unwrap().state.halted = true;
            }
            "nop" => {} // native slot0 points to the explicit nullsub_1.
            _ => bail!(
                "unsupported SugarPot native command {name}({})",
                args.join(",")
            ),
        }
        Ok(true)
    }
    fn step_inner(
        &mut self,
        h: &mut dyn HostServices,
        mut input: InputFrame,
        budget: u32,
    ) -> Result<VmYield> {
        let was_preparing = self.machine.is_none();
        if !self.boot(h)? {
            return Ok(VmYield::BudgetExhausted);
        }
        if was_preparing {
            return Ok(VmYield::BudgetExhausted);
        }
        self.effect_skip = self.skip_enabled
            && (input.buttons_down & buttons::CTRL != 0 || self.window_ui.skip)
            && (self.settings.header[10] == 0 || self.current_read);
        if !matches!(
            self.wait,
            Some(
                Wait::Settings
                    | Wait::Recollection
                    | Wait::Gallery
                    | Wait::BgmMode
                    | Wait::Backlog
                    | Wait::EndMode
                    | Wait::OriginalLoad
                    | Wait::OriginalSave
                    | Wait::Slots
                    //Effect80 freezes ordinary sprite/raster updates. Native
                    //43F430's fall/filter-after-mix combination needs a separate
                    //shared transition input and is rejected during preparation.
                    | Wait::RestoreTransition { .. }
            )
        ) {
            self.animate(h)?;
            self.raster_frame(h)?;
            self.fall_frame(h)?;
            self.filter_frame(h)?;
        }
        if let Some(wait) = self.poll_wait(h, &mut input)? {
            return Ok(wait);
        }
        for _ in 0..budget {
            let frame = self.machine.as_ref().unwrap().state.frame.clone();
            let (line, offset) = self
                .machine
                .as_ref()
                .unwrap()
                .current()
                .map(|i| (i.line, i.byte_offset))
                .unwrap_or((0, 0));
            self.mark_read()?;
            let event = self.machine.as_mut().unwrap().event().with_context(|| {
                format!(
                    "SugarPot {}:{} native PC{} source offset{offset:#x}",
                    frame.script,
                    line,
                    frame.pc + 1
                )
            })?;
            let Some(event) = event else { continue };
            let ready = match event {
                Event::Blank => {
                    let line = self.machine.as_ref().unwrap().state.frame.pc + 1;
                    if self.native_text.blank(line, self.mode == 2)? {
                        self.capture_history_voice("\\n")?;
                        self.advance_native_line()?;
                    }
                    true
                }
                Event::Halted => return Ok(VmYield::Halted),
                Event::Window(id) => {
                    let index = usize::try_from(id)?;
                    self.windows
                        .get(index)
                        .context("SugarPot current window outside allocation")?;
                    let visibility = self
                        .windows
                        .get(self.current_window)
                        .map_or(2, |w| w.visibility);
                    self.current_window = index;
                    self.windows[index].visibility = visibility;
                    true
                }
                Event::Selection(options) => {
                    self.native_text
                        .begin_command(self.machine.as_ref().unwrap().state.frame.pc + 1);
                    self.choices(h, &options)?;
                    true
                }
                Event::Text(text) => self.text(h, &text).with_context(|| {
                    format!(
                        "SugarPot {}:{} native PC{} source offset{offset:#x} text",
                        frame.script,
                        line,
                        frame.pc + 1
                    )
                })?,
                Event::Command { name, arguments } => {
                    self.native_text
                        .begin_command(self.machine.as_ref().unwrap().state.frame.pc + 1);
                    self.command(h, &name, &arguments).with_context(|| {
                        format!(
                            "SugarPot {}:{} native PC{} source offset{offset:#x} command {name}",
                            frame.script,
                            line,
                            frame.pc + 1
                        )
                    })?
                }
            };
            if !ready {
                return Ok(VmYield::BudgetExhausted);
            }
            self.machine.as_mut().unwrap().commit();
            if let Some(wait) = &self.wait {
                return Ok(match wait {
                    Wait::Clock(n) | Wait::RestoreTransition { deadline: n, .. } => {
                        VmYield::WaitUntil(*n)
                    }
                    Wait::AnimationNumber { next, .. } => VmYield::WaitUntil(*next),
                    Wait::Audio(_)
                    | Wait::AudioStop(_)
                    | Wait::Movie
                    | Wait::Motion { .. }
                    | Wait::Shake(_)
                    | Wait::Raster { .. }
                    | Wait::EfMovement(_) => VmYield::BudgetExhausted,
                    Wait::Geometry => VmYield::BudgetExhausted,
                    Wait::Rotation => VmYield::BudgetExhausted,
                    Wait::Warp => VmYield::BudgetExhausted,
                    Wait::BgmMode | Wait::Backlog => VmYield::BudgetExhausted,
                    _ => {
                        if self.animation_running() {
                            VmYield::BudgetExhausted
                        } else {
                            VmYield::WaitForInput
                        }
                    }
                });
            }
        }
        Ok(VmYield::BudgetExhausted)
    }
}
impl VmAdapter for SugarPotVm {
    fn engine_id(&self) -> &'static str {
        "sugarpot"
    }
    fn snapshot_schema(&self) -> u32 {
        1
    }
    fn preparation_pending(&self) -> bool {
        self.machine.is_none()
    }
    fn presentation_pending(&self) -> bool {
        let backlog_pending = self
            .backlog_view
            .as_ref()
            .is_some_and(backlog::View::preparing);
        backlog_pending
            || self.reading.is_some()
            || self.decoded.is_some()
            || self.allocating.is_some()
            || self.fall.as_ref().is_some_and(fall::Fall::preparing)
            || self
                .recollection
                .as_ref()
                .is_some_and(recollection::Recollection::preparing)
            || self
                .gallery
                .as_ref()
                .is_some_and(recollection::Recollection::preparing)
            || self
                .bgm_mode
                .as_ref()
                .is_some_and(bgm_mode::BgmMode::preparing)
            || self
                .end_mode
                .as_ref()
                .is_some_and(endmode::EndMode::preparing)
            || self.audio.is_some()
            || self.animation_load.is_some()
            || self.animation_tone.is_some()
            || self.toning.is_some()
            || self
                .geometry
                .as_ref()
                .is_some_and(geometry::Geometry::preparing)
            || self
                .rotation
                .as_ref()
                .is_some_and(rotation::Rotation::preparing)
            || self.warp.as_ref().is_some_and(warp::Warp::preparing)
            || self
                .mask_update
                .as_ref()
                .is_some_and(update::MaskUpdate::preparing)
    }
    fn step(
        &mut self,
        h: &mut dyn HostServices,
        input: InputFrame,
        budget: u32,
    ) -> crysirror_core::Result<VmYield> {
        match self.step_inner(h, input, budget) {
            Ok(v) => Ok(v),
            Err(e) => {
                let message = format!("{e:#}");
                self.error = Some(message.clone());
                Err(Error::Io(message))
            }
        }
    }
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Err(Error::Unsupported(
            "SugarPot framework snapshot reconstruction",
        ))
    }
    fn restore(&mut self, _: &[u8]) -> crysirror_core::Result<()> {
        Err(Error::Unsupported(
            "SugarPot framework snapshot reconstruction",
        ))
    }
    fn begin_original_restore(
        &self,
        bytes: Arc<[u8]>,
    ) -> crysirror_core::Result<Box<dyn crysirror_core::native_restore::OriginalRestorePreparation>>
    {
        self.prepare_original_restore(bytes)
            .map(|job| {
                Box::new(job) as Box<dyn crysirror_core::native_restore::OriginalRestorePreparation>
            })
            .map_err(|e| Error::Io(format!("SugarPot original restore admission: {e:#}")))
    }
    fn export_original_save(&mut self) -> crysirror_core::Result<Vec<u8>> {
        self.native_slot()
            .and_then(|slot| slot.encode())
            .map_err(|e| Error::Io(format!("{e:#}")))
    }
    fn diagnostics(&self) -> Vec<u8> {
        serde_json::to_vec(&json!({"adapter":"sugarpot","profile":"tsukumo-2012","mode":"adopt","bootstrap_pending":self.machine.is_none(),"frame":self.machine.as_ref().map(|m|&m.state.frame),"executed":self.machine.as_ref().map_or(0,|m|m.state.executed),"wait":format!("{:?}",self.wait),"sprites":self.sprites.iter().filter(|s|s.visible).count(),"buttons":self.buttons.values().filter(|b|b.sprite.visible).count(),"text":self.text,"text_glyphs":self.text_glyphs,"api_calls":self.api_calls,"save_enabled":self.save_enabled,"load_enabled":self.load_enabled,"original_saves":"unsupported","framework_saves":"unsupported","error":self.error})).unwrap_or_default()
    }
}
#[cfg(feature = "runtime")]
pub fn register(registry: &mut crysirror_runtime::AdapterRegistry) -> Result<()> {
    registry.register("sugarpot", Arc::new(Factory))
}
#[cfg(feature = "runtime")]
struct Factory;
#[cfg(feature = "runtime")]
impl crysirror_runtime::runtime::VmFactory for Factory {
    fn create(
        &self,
        client: ClientId,
        spec: &crysirror_runtime::model::ClientSpec,
    ) -> Result<Box<dyn VmAdapter>> {
        ensure!(
            spec.program.is_empty(),
            "SugarPot executes native SCR, not data actions"
        );
        Ok(Box::new(SugarPotVm::new(
            client,
            serde_json::from_value(spec.config.clone())?,
        )?))
    }
}

fn native_value(value: crate::native_storage::NativeValue) -> Value {
    use crate::native_storage::NativeValue;
    match value {
        NativeValue::Integer(n) => Value::Integer(n),
        NativeValue::FloatBits(n) => Value::Float(f32::from_bits(n)),
        NativeValue::Text(s) => Value::String(s),
    }
}
fn stored_value(value: &Value) -> Result<crate::native_storage::NativeValue> {
    use crate::native_storage::NativeValue;
    Ok(match value {
        Value::Integer(n) => NativeValue::Integer(*n),
        Value::Float(n) => NativeValue::FloatBits(n.to_bits()),
        Value::String(s) => NativeValue::Text(s.clone()),
        Value::List(_) => bail!("SugarPot global container"),
    })
}

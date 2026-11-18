// SPDX-License-Identifier: MPL-2.0
//! Adopt-mode host translation. Native VM state never contains device handles.
//! NSS grammar/value policy: NitroSharp e3869842. Thread join/request behavior:
//! NitroSharp legacy 36701328b3167a2eef754a624b73b7d1e54fb2bb (MIT).
use crate::{
    arguments::{Arguments, coordinate},
    catalog::Builtin,
    entity::{Entities, Kind},
    program::Module,
    resource::{self, Catalog},
    value::Value,
    vm::{self, Call, Machine, Step, Thread},
};
use anyhow::{Context, Result, bail, ensure};
use crysirror_core::{
    Error,
    path::{Mount, VfsPath},
    service::{
        AudioAsset, AudioParameters, AudioPreparePhase, AudioRequest, ClientStorage, EngineQuery,
        HostServices, ImageMetadata, InputFrame, buttons,
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
    pub entry: String,
    #[serde(default = "script_root")]
    pub script_root: String,
    #[serde(default = "encoding")]
    pub encoding: String,
    pub canvas: [u32; 2],
    #[serde(default = "font")]
    pub font: String,
    /// Original script's platform value is a content ABI, not the host OS.
    pub platform: i32,
    #[serde(default)]
    pub archive_material: Option<crate::archive::ArchiveMaterial>,
    /// Font assets are prepared after the game's archives have been mounted.
    #[serde(default)]
    pub font_sources: BTreeMap<String, String>,
}
mod auxiliary;
mod backlog;
mod bitmap_pair;
mod bitmap_position;
mod bitmap_read;
mod blur;
mod camera;
mod legacy_transition;
mod composite;
mod tone;
mod checkpoint;
mod choice;
mod cube;
mod font_cache;
mod frame_animation;
mod frame_image;
mod graphics;
mod image_replace;
mod lens;
mod media;
mod modal_movie;
mod native_bitmap;
mod native_file;
mod particles;
mod preload;
mod restore;
mod ripple;
mod scrollbar;
mod shade;
mod shake;
mod stencil;
mod storage;
mod surface;
mod system;
mod text_fade;
mod text_host;
mod waits;
fn script_root() -> String {
    "nss".into()
}
fn encoding() -> String {
    "shift-jis".into()
}
fn font() -> String {
    "default".into()
}
struct Boot {
    catalog: Arc<Catalog>,
    modules: Vec<(String, Module)>,
    common: Option<storage::Common>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Image {
    paint: Json,
    size: [u32; 2],
}
enum Wait {
    None,
    Frame(u64),
    Until(u64),
    Input(Option<u64>),
    Motion {
        ids: Vec<u32>,
        properties: Vec<&'static str>,
        until: Option<u64>,
    },
    Audio {
        ids: Vec<u32>,
        allow_movie_skip: bool,
    },
    Actions {
        ids: Vec<u32>,
        until: Option<u64>,
    },
    Join(u32),
    Text {
        ids: Vec<u32>,
        until: Option<u64>,
    },
    Line(u32),
    Frames(Vec<u32>),
    Shakes {
        ids: Vec<u32>,
        until: Option<u64>,
    },
}
struct Execution {
    thread: Thread,
    entry: vm::Frame,
    wait: Wait,
    enabled: bool,
    image: Option<Task<Image>>,
    image_replace: Option<image_replace::Replacement>,
    surface_preparation: Option<(
        u32,
        Task<crysirror_scene::layer::render_target::RenderTargetSpec>,
    )>,
    surface_source: Option<(u32, crysirror_core::image_processing::ImageJob)>,
    stencil_preparation: Option<stencil::Preparation>,
    ripple_preparation: Option<ripple::Preparing>,
    lens_table: Option<Task<Arc<crysirror_core::image_processing::LensTable>>>,
    particle_sprite: Option<Task<[u32; 2]>>,
    frame_image: Option<frame_image::Preparing>,
    difference: Option<bitmap_pair::Difference>,
    animation_bitmap: Option<native_bitmap::NativeBitmap>,
    cube_preparation: Option<cube::Preparing>,
    glyph_preparation: Option<font_cache::Preparing>,
    tone: Option<tone::Preparing>,
    composite: Option<composite::Preparing>,
    legacy_transition: Option<legacy_transition::Preparing>,
    shake_preparation: Option<shake::Preparing>,
    preload: Option<preload::Loading>,
    frame_images: Option<Task<frame_animation::Prepared>>,
    frame_surfaces: Option<frame_animation::SurfaceFrames>,
    frame_preparation: Option<frame_animation::Preparing>,
    capture: Option<crysirror_core::image_processing::ImageJob>,
    audio: Vec<u64>,
    dsp_paused: BTreeSet<u64>,
    module: Option<Task<Vec<(String, Module)>>>,
    pending_text: Option<(u32, u64)>,
    page: Option<u32>,
    dialog: Option<u32>,
    civil: Option<Box<dyn crysirror_core::service::CivilTimeQuery>>,
    desktop: Option<Box<dyn crysirror_core::service::DesktopRequestJob>>,
    interrupt: bool,
    file: Option<Task<storage::FileResult>>,
    movie_init: Option<u32>,
    seeks: Vec<u64>,
    retire_when_done: bool,
}
impl Execution {
    fn new(thread: Thread) -> Self {
        let entry = thread.frames[0].clone();
        Self {
            thread,
            entry,
            wait: Wait::None,
            enabled: true,
            image: None,
            image_replace: None,
            surface_preparation: None,
            surface_source: None,
            stencil_preparation: None,
            ripple_preparation: None,
            lens_table: None,
            particle_sprite: None,
            frame_image: None,
            difference: None,
            animation_bitmap: None,
            cube_preparation: None,
            glyph_preparation: None,
            tone: None,
            composite: None,
            legacy_transition: None,
            shake_preparation: None,
            preload: None,
            frame_images: None,
            frame_surfaces: None,
            frame_preparation: None,
            capture: None,
            audio: Vec::new(),
            dsp_paused: BTreeSet::new(),
            module: None,
            pending_text: None,
            page: None,
            dialog: None,
            civil: None,
            desktop: None,
            interrupt: false,
            file: None,
            movie_init: None,
            seeks: Vec::new(),
            retire_when_done: false,
        }
    }
}
pub struct N2SystemVm {
    client: ClientId,
    config: Config,
    machine: Machine,
    entities: Entities,
    threads: BTreeMap<u32, Execution>,
    main: u32,
    pool: Option<TaskPool>,
    boot: Option<Task<Boot>>,
    storage: Option<Arc<dyn ClientStorage>>,
    catalog: Option<Arc<Catalog>>,
    images: Option<Arc<dyn ImageMetadata>>,
    started: bool,
    frame: u64,
    start_ms: Option<u64>,
    error: Option<String>,
    executed: u64,
    style: crate::text::Style,
    pages: BTreeMap<u32, crate::text::Page>,
    measurement: Option<Arc<dyn crysirror_core::service::TextMeasurement>>,
    fonts: Option<Arc<dyn crysirror_core::service::FontLibrary>>,
    text_dirty: BTreeSet<u32>,
    text_jobs: BTreeMap<u32, Task<(u64, crate::text::Layout)>>,
    text_published: BTreeMap<u32, u64>,
    voices: BTreeMap<String, u32>,
    voice_tickets: Vec<u64>,
    choices: BTreeMap<u32, choice::Choice>,
    rng: Option<crysirror_core::random::Mt19937>,
    seed: Option<Box<dyn crysirror_core::service::SessionSeedQuery>>,
    native_dialog: Option<(u32, u32)>,
    native_dialog_results: BTreeMap<u32, u32>,
    disused: BTreeSet<u32>,
    pending_restore: Option<Box<restore::Restore>>,
    restore_sounds: Vec<u32>,
    restored_audio: Vec<(u64, bool)>,
    lenses: BTreeMap<u32, lens::LensEffect>,
    particles: BTreeMap<u32, particles::Particles>,
    /// Request NoLog: dialogue text of this page is not recorded in the backlog.
    no_log: BTreeSet<u32>,
    shortcuts: BTreeMap<u64, String>,
    shortcut_job: Option<(String, Task<Vec<(String, Module)>>)>,
    interrupt: Option<u32>,
    fullscreen: Option<bool>,
    video_capture: Option<crysirror_core::image_processing::ImageJob>,
    video_hold: Option<String>,
    read_marks: BTreeMap<String, BTreeMap<String, bool>>,
    font_jobs: Vec<crysirror_core::service::FontJob>,
    font_handles: Vec<crysirror_core::service::FontHandle>,
    fonts_prepared: bool,
    glyph_caches: BTreeMap<u32, font_cache::Cache>,
    sound_resources: BTreeMap<u32, media::SoundResource>,
    movies: BTreeMap<u32, media::Movie>,
    modal_movie: Option<modal_movie::ModalMovie>,
    modal_movie_results: BTreeSet<u32>,
    paths: BTreeMap<u32, graphics::PathTrack>,
    cameras: BTreeMap<u32, camera::Camera>,
    tone_backups: BTreeMap<u32, Arc<dyn crysirror_core::image_processing::ImageResource>>,
    legacy_serial: u64,
    legacy_transitions: BTreeMap<u32, legacy_transition::Transition>,
    cubes: BTreeMap<u32, cube::Cube>,
    blurs: BTreeMap<u32, blur::Blur>,
    ripples: BTreeMap<u32, ripple::Ripple>,
    framed_images: BTreeSet<u32>,
    bitmap_formats: BTreeMap<u32, bitmap_read::BitmapFormat>,
    animation_preloads: Vec<Arc<dyn crysirror_core::image_processing::ImageResource>>,
    shakes: BTreeMap<u32, shake::Shake>,
    history: Vec<(String, String, String)>,
    hidden_for_text: BTreeSet<u32>,
    captured_images: BTreeMap<u32, String>,
    effects: BTreeMap<u32, graphics::PlainEffect>,
    mask_transitions: BTreeMap<u32, graphics::MaskTrack>,
    reset_requested: bool,
    reboot_requested: bool,
    twitter_reported: bool,
    frame_animations: BTreeMap<u32, frame_animation::Animation>,
    scrollbars: BTreeMap<u32, scrollbar::Scrollbar>,
    alpha_masks: BTreeMap<u32, String>,
    backlogs: BTreeMap<u32, backlog::Backlog>,
    history_revision: u64,
    history_newline: bool,
    text_fades: BTreeMap<u32, text_fade::TextFade>,
    animation_image_leases: BTreeMap<u32, Vec<String>>,
    shades: BTreeMap<u32, shade::Shade>,
    preloaded: BTreeMap<String, preload::Cached>,
    surfaces: BTreeMap<u32, crysirror_scene::layer::render_target::RenderTargetSpec>,
    stencils: BTreeMap<u32, [f32; 2]>,
    stencil_planes: BTreeMap<u32, Arc<dyn crysirror_core::image_processing::ImageResource>>,
    stencil_bitmaps: BTreeMap<u32, Arc<dyn crysirror_core::image_processing::ImageResource>>,
    rgba_masks: BTreeSet<u32>,
}
impl N2SystemVm {
    pub fn new(client: ClientId, mut config: Config) -> Result<Self> {
        vm::module_name(&config.entry)?;
        ensure!(
            matches!(config.encoding.as_str(), "shift-jis" | "utf-8"),
            "N2System source encoding profile"
        );
        ensure!(
            config.canvas.iter().all(|n| (1..=16384).contains(n)),
            "N2System canvas extent"
        );
        config.script_root = config
            .script_root
            .replace('\\', "/")
            .trim_end_matches('/')
            .into();
        if !config.script_root.is_empty() {
            VfsPath::new(Mount::Asset, &config.script_root)?;
        }
        let style = crate::text::Style {
            vertical_glyphs: false,
            font: config.font.clone(),
            size: 24,
            color: [255; 4],
            outline: None,
            bold: false,
            italic_font: None,
        };
        Ok(Self {
            client,
            config,
            machine: Machine::default(),
            entities: Entities::default(),
            threads: BTreeMap::new(),
            main: 0,
            pool: None,
            boot: None,
            storage: None,
            catalog: None,
            images: None,
            started: false,
            frame: 0,
            start_ms: None,
            error: None,
            executed: 0,
            style,
            pages: BTreeMap::new(),
            measurement: None,
            fonts: None,
            text_dirty: BTreeSet::new(),
            text_jobs: BTreeMap::new(),
            text_published: BTreeMap::new(),
            voices: BTreeMap::new(),
            voice_tickets: Vec::new(),
            choices: BTreeMap::new(),
            rng: None,
            seed: None,
            native_dialog: None,
            native_dialog_results: BTreeMap::new(),
            disused: BTreeSet::new(),
            pending_restore: None,
            restore_sounds: Vec::new(),
            restored_audio: Vec::new(),
            lenses: BTreeMap::new(),
            particles: BTreeMap::new(),
            no_log: BTreeSet::new(),
            shortcuts: BTreeMap::new(),
            shortcut_job: None,
            interrupt: None,
            fullscreen: None,
            video_capture: None,
            video_hold: None,
            read_marks: BTreeMap::new(),
            font_jobs: Vec::new(),
            font_handles: Vec::new(),
            fonts_prepared: false,
            glyph_caches: BTreeMap::new(),
            sound_resources: BTreeMap::new(),
            movies: BTreeMap::new(),
            modal_movie: None,
            modal_movie_results: BTreeSet::new(),
            paths: BTreeMap::new(),
            cameras: BTreeMap::new(),
            tone_backups: BTreeMap::new(),
            legacy_serial: 0,
            legacy_transitions: BTreeMap::new(),
            cubes: BTreeMap::new(),
            blurs: BTreeMap::new(),
            ripples: BTreeMap::new(),
            framed_images: BTreeSet::new(),
            bitmap_formats: BTreeMap::new(),
            animation_preloads: Vec::new(),
            shakes: BTreeMap::new(),
            history: Vec::new(),
            hidden_for_text: BTreeSet::new(),
            captured_images: BTreeMap::new(),
            effects: BTreeMap::new(),
            mask_transitions: BTreeMap::new(),
            reset_requested: false,
            reboot_requested: false,
            twitter_reported: false,
            frame_animations: BTreeMap::new(),
            scrollbars: BTreeMap::new(),
            alpha_masks: BTreeMap::new(),
            backlogs: BTreeMap::new(),
            history_revision: 0,
            history_newline: false,
            text_fades: BTreeMap::new(),
            animation_image_leases: BTreeMap::new(),
            shades: BTreeMap::new(),
            preloaded: BTreeMap::new(),
            surfaces: BTreeMap::new(),
            stencils: BTreeMap::new(),
            stencil_planes: BTreeMap::new(),
            stencil_bitmaps: BTreeMap::new(),
            rgba_masks: BTreeSet::new(),
        })
    }
    fn submit(&self, host: &mut dyn HostServices, layers: Vec<Json>, ui: Vec<Json>) -> Result<()> {
        if layers.is_empty() && ui.is_empty() {
            return Ok(());
        }
        host.submit_engine(
            self.client,
            1,
            &serde_json::to_vec(&json!({"layers":layers,"ui":ui}))?,
        )?;
        Ok(())
    }
    fn prepare(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        if self.started {
            return Ok(true);
        }
        if !self.fonts_prepared {
            use crysirror_core::service::{FontLoadSource, FontLoadSpec, FontLoadState};
            if self.fonts.is_none() {
                self.fonts = Some(host.font_library(self.client)?);
            }
            let library = self.fonts.as_ref().unwrap();
            if self.font_jobs.is_empty() && self.font_handles.is_empty() {
                ensure!(
                    self.config.font_sources.len() <= 8,
                    "N2System font source capacity"
                );
                for (name, path) in &self.config.font_sources {
                    self.font_jobs.push(library.begin(FontLoadSpec {
                        name: name.clone(),
                        source: FontLoadSource::Storage(VfsPath::new(Mount::Asset, path)?),
                        face_index: 0,
                    })?);
                }
            }
            let mut pending = Vec::new();
            for job in std::mem::take(&mut self.font_jobs) {
                match library.status(job)? {
                    FontLoadState::Pending => pending.push(job),
                    FontLoadState::Failed(error) => bail!("N2System game font: {error}"),
                    FontLoadState::Ready(info) => {
                        if info.logical_alias == self.config.font {
                            self.style.font = info.alias.clone();
                        }
                        self.font_handles.push(info.handle);
                        library.release_job(job)?;
                    }
                }
            }
            self.font_jobs = pending;
            if !self.font_jobs.is_empty() {
                return Ok(false);
            }
            self.fonts_prepared = true;
        }
        if self.boot.is_none() {
            self.storage = Some(host.storage(self.client)?);
            self.images = Some(host.image_metadata(self.client)?);
            self.measurement = Some(host.text_measurement(self.client)?);
            self.fonts = Some(host.font_library(self.client)?);
            self.pool = Some(TaskPool::new("n2system-resources", 1, 4)?);
            let storage = self.storage.as_ref().unwrap().clone();
            let config = self.config.clone();
            self.boot = Some(self.pool.as_ref().unwrap().spawn(move |cancel| {
                let catalog = Arc::new(resource::catalog(storage.as_ref(), &cancel)?);
                let modules = resource::modules(
                    storage.clone(),
                    catalog.clone(),
                    &config.script_root,
                    &config.entry,
                    &config.encoding,
                    BTreeSet::new(),
                    &cancel,
                )?;
                let common = storage::Common::load(
                    storage.as_ref(),
                    config.archive_material.as_ref(),
                    &|| cancel.check(),
                )?;
                Ok(Boot {
                    catalog,
                    modules,
                    common,
                })
            })?);
        }
        let Some(boot) = self.boot.as_mut().unwrap().try_take()? else {
            return Ok(false);
        };
        let mut machine = Machine::default();
        for (name, module) in boot.modules {
            machine.insert(&name, module)?;
        }
        if let Some(mut common) = boot.common {
            machine.merge_common(
                &mut common.flags,
                &common.arrays,
                common.native_variables.is_some(),
            )?;
            self.read_marks = common.read;
        }
        machine.variables.insert(
            "SYSTEM_save_path".into(),
            Value::String("n2system-v1/".into()),
        );
        machine.flags.insert(
            "SYSTEM_save_path".into(),
            Value::String("n2system-v1/".into()),
        );
        let thread = machine.start(&self.config.entry, None, "main".into())?;
        let main = self.entities.root("main".into())?;
        self.submit(host, vec![self.entity_group(main)?], vec![])?;
        self.main = main;
        self.machine = machine;
        self.threads.insert(main, Execution::new(thread));
        self.catalog = Some(boot.catalog);
        self.boot = None;
        self.started = true;
        Ok(false)
    }
    fn query(&self, current: u32, query: &str) -> Result<Vec<u32>> {
        self.entities.query(current, self.main, query)
    }
    fn layer(&self, host: &mut dyn HostServices, id: u32) -> Result<Json> {
        let layer: Json =
            serde_json::from_slice(&host.query_engine(self.client, EngineQuery::Layer(id))?)?;
        ensure!(
            layer.is_object(),
            "N2System owned shared layer {id} is missing"
        );
        Ok(layer)
    }
    fn entity_group(&self, id: u32) -> Result<Json> {
        let mut group = self.entities.nodes[&id].group();
        group["layer"]["parent"] = json!(self.entities.render_parent(id)?);
        Ok(group)
    }
    fn native_position(
        &self,
        host: &mut dyn HostServices,
        id: u32,
        value: &Value,
        current: f32,
        size: f32,
        axis: usize,
    ) -> Result<f32> {
        let mut result = self.native_position_in_parent(
            host,
            self.entities.nodes[&id].parent,
            value,
            current,
            size,
            axis,
        )?;
        if self.framed_images.contains(&id)
            && matches!(value, Value::Number(_))
            && result != current
        {
            result -= 8.;
        }
        Ok(result)
    }
    fn native_position_in_parent(
        &self,
        host: &mut dyn HostServices,
        parent: Option<u32>,
        value: &Value,
        current: f32,
        size: f32,
        axis: usize,
    ) -> Result<f32> {
        let mut origin = 0.;
        let mut bounds = self.config.canvas[axis] as f32;
        if let Some(parent) = parent {
            let entity = &self.entities.nodes[&parent];
            if matches!(entity.kind, Kind::Visual) {
                bounds = entity.size[axis];
                if entity.container != Some(false) {
                    origin = self.layer(host, parent)?["rect"][axis]
                        .as_f64()
                        .context("N2System parent origin")? as f32;
                }
            }
        }
        // Center is aligned to the native design canvas, even inside a box.
        if value.is_constant("Center") || value.is_constant("Middle") {
            bounds = self.config.canvas[axis] as f32;
            origin = 0.;
        }
        Ok(origin + coordinate(value, current - origin, size, bounds, axis)?)
    }
    fn create(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        path: &str,
        kind: Kind,
        size: [f32; 2],
    ) -> Result<u32> {
        let (parent, name) = self.entities.destination(current, path)?;
        if let Some(old) = self.entities.child(parent, &name) {
            self.destroy(host, old)?;
        }
        let id = self.entities.add(name, Some(parent), kind, size)?;
        self.submit(host, vec![self.entity_group(id)?], vec![])?;
        Ok(id)
    }
    fn destroy(&mut self, host: &mut dyn HostServices, id: u32) -> Result<()> {
        let ids = self.entities.subtree(id);
        let mut commands = Vec::new();
        for id in ids.iter().rev() {
            commands.push(json!({"op":"remove","id":id}));
        }
        // Remove children before their parent; the layer service owns its own
        // retained resources and cancels tracks as part of the same batch.
        self.submit(host, commands, vec![])?;
        for entity in self.entities.remove(id) {
            if self.choices.remove(&entity.id).is_some_and(|c| c.widget) {
                self.submit(host, vec![], vec![json!({"op":"remove","id":entity.id})])?;
            }
            if self.scrollbars.remove(&entity.id).is_some() {
                self.submit(host, vec![], vec![json!({"op":"remove","id":entity.id})])?;
            }
            self.pages.remove(&entity.id);
            self.text_fades.remove(&entity.id);
            self.text_jobs.remove(&entity.id);
            self.text_dirty.remove(&entity.id);
            self.text_published.remove(&entity.id);
            self.paths.remove(&entity.id);
            self.cameras.remove(&entity.id);
            self.tone_backups.remove(&entity.id);
            self.legacy_transitions.remove(&entity.id);
            self.cubes.remove(&entity.id);
            self.glyph_caches.remove(&entity.id);
            self.blurs.remove(&entity.id);
            self.remove_ripple(host, entity.id, false)?;
            self.framed_images.remove(&entity.id);
            self.bitmap_formats.remove(&entity.id);
            self.shakes.remove(&entity.id);
            self.hidden_for_text.remove(&entity.id);
            self.disused.remove(&entity.id);
            self.no_log.remove(&entity.id);
            self.effects.remove(&entity.id);
            self.lenses.remove(&entity.id);
            self.particles.remove(&entity.id);
            self.mask_transitions.remove(&entity.id);
            self.frame_animations.remove(&entity.id);
            self.alpha_masks.remove(&entity.id);
            self.surfaces.remove(&entity.id);
            self.stencils.remove(&entity.id);
            self.stencil_planes.remove(&entity.id);
            self.stencil_bitmaps.remove(&entity.id);
            self.rgba_masks.remove(&entity.id);
            self.backlogs.remove(&entity.id);
            self.remove_shade(host, entity.id)?;
            if let Some(paths) = self.animation_image_leases.remove(&entity.id) {
                for path in paths {
                    host.release_image_resource(self.client, &path)?;
                }
            }
            if let Some(path) = self.captured_images.remove(&entity.id) {
                host.release_image_resource(self.client, &path)?;
            }
            if let Some(resource) = self.sound_resources.remove(&entity.id) {
                if let Some(ticket) = resource.ticket {
                    host.release_audio_resource_preparation(self.client, ticket)?;
                }
                if let Some(handle) = resource.handle {
                    host.release_audio_resource(self.client, handle)?;
                }
            }
            if self.movies.remove(&entity.id).is_some() {
                host.media_request(
                    self.client,
                    crysirror_core::service::MediaRequest::Remove { id: entity.id },
                )?;
            }
            if matches!(entity.kind, Kind::Sound { playing: true, .. }) {
                host.submit_audio(
                    self.client,
                    AudioRequest::Stop {
                        channel: entity.id,
                        fade_ms: 0.,
                    },
                )?;
            }
            if let Some(state) = self.threads.remove(&entity.id) {
                self.release_execution(host, state)?;
            }
            self.voices.retain(|_, channel| *channel != entity.id);
            let mut pending = Vec::new();
            for ticket in std::mem::take(&mut self.voice_tickets) {
                if host.audio_preparation(self.client, ticket)?.channel == entity.id {
                    host.release_audio(self.client, ticket)?;
                } else {
                    pending.push(ticket);
                }
            }
            self.voice_tickets = pending;
            let mut restored = Vec::new();
            for (ticket, paused) in std::mem::take(&mut self.restored_audio) {
                if host.audio_preparation(self.client, ticket)?.channel == entity.id {
                    host.release_audio(self.client, ticket)?;
                } else { restored.push((ticket,paused)); }
            }
            self.restored_audio = restored;
        }
        Ok(())
    }
    fn release_execution(&self, host: &mut dyn HostServices, state: Execution) -> Result<()> {
        for ticket in state.audio {
            host.release_audio(self.client, ticket)?;
        }
        for ticket in state.seeks {
            host.release_audio_seek(self.client, ticket)?;
        }
        // Dropping the remaining owner jobs cancels their scoped work.
        Ok(())
    }
    fn visual(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        args: &Arguments<'_>,
        paint: Json,
        size: [u32; 2],
        uv: Option<[f32; 4]>,
    ) -> Result<u32> {
        self.visual_owned(host, current, args, paint, size, uv, None)
    }
    fn visual_owned(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        args: &Arguments<'_>,
        mut paint: Json,
        size: [u32; 2],
        uv: Option<[f32; 4]>,
        source: Option<Arc<dyn crysirror_core::image_processing::ImageResource>>,
    ) -> Result<u32> {
        // Resolve the immutable input before replacing the destination. A
        // script may create a visual over the name of its source image.
        let format = self.bitmap_format(&paint);
        let source = match source {
            Some(source) => Some(source),
            None => match paint.get("asset").and_then(Json::as_str) {
                Some(path) => host.published_image_resource(self.client, path)?,
                None => None,
            },
        };
        let id = self.create(
            host,
            current,
            &args.string(0)?,
            Kind::Visual,
            size.map(|n| n as f32),
        )?;
        if let Some(image) = source {
            let uploaded = host.publish_image_resource(self.client, id, image)?;
            let path = uploaded.path.relative().to_owned();
            paint["asset"] = json!(path);
            self.captured_images.insert(id, path);
        }
        let parent = self.entities.render_parent(id)?;
        self.bitmap_formats.insert(id, format);
        let x = self.native_position(host, id, args.value(2), 0., size[0] as f32, 0)?;
        let y = self.native_position(host, id, args.value(3), 0., size[1] as f32, 1)?;
        self.submit(
            host,
            vec![json!({"op":"put","layer":{"id":id,"parent":parent,
            "order":[args.priority(1)?,0],"rect":[x,y,size[0],size[1]],"paint":paint,"uv":uv,
            "transform":{"pivot_x":size[0] as f32*0.5,"pivot_y":size[1] as f32*0.5}}})],
            vec![],
        )?;
        Ok(id)
    }
    fn source_image(&self, current: u32, source: &str) -> Result<Option<Image>> {
        if let Ok(path) = resource::resolve(self.catalog.as_ref().unwrap(), source) {
            if let Some(image) = self
                .preloaded
                .get(&resource::asset_key(&path)?)
                .and_then(|c| c.image.clone())
            {
                return Ok(Some(image));
            }
            return Ok(Some(Image {
                paint: json!({"kind":"image","asset":path}),
                size: [0; 2],
            }));
        }
        let ids = self.query(current, source)?;
        if let Some(id) = ids.first() {
            ensure!(
                !self.surfaces.contains_key(id),
                "N2System live surface source requires a completed target readback"
            );
            if let Kind::Image { paint, size } = &self.entities.nodes[id].kind {
                return Ok(Some(Image {
                    paint: paint.clone(),
                    size: *size,
                }));
            }
        }
        Ok(None)
    }
    fn native(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        state: &mut Execution,
        input: &InputFrame,
        call: &Call,
    ) -> Result<bool> {
        let args = Arguments::new(call);
        let now = host.now_ms();
        let mut result = Value::Null;
        match call.function {
            Builtin::SetTone => return self.native_tone(host, current, state, &args),
            Builtin::SetComposite => return self.native_composite(host, current, state, &args),
            Builtin::DrawLegacyTransition => return self.native_legacy_transition(host, current, state, &args),
            Builtin::CreateCamera | Builtin::MoveCamera | Builtin::MoveCube => {
                return self.native_camera(host, current, state, &args, call);
            }
            Builtin::CreateCube | Builtin::SetFov => {
                return self.native_cube(host, current, state, &args, call);
            }
            Builtin::CreateSurface => self.create_surface(host, current, &args)?,
            Builtin::CreateStencil | Builtin::CreateMask => {
                if !self.create_stencil(
                    host,
                    current,
                    state,
                    &args,
                    call.function == Builtin::CreateStencil,
                )? {
                    return Ok(false);
                }
            }
            Builtin::SetSurface => {
                if !self.set_surface(host, current, state, &args)? {
                    return Ok(false);
                }
            }
            Builtin::Escape => {
                // Native415B60 explicitly ignores nonpositive/noninteger input.
                if let Ok(count) = args.string(0)?.parse::<i32>() {
                    if count > 0 {
                        if self.machine.escape(&mut state.thread, count as usize)? {
                            self.end_selection(host)?;
                        }
                        return Ok(true);
                    }
                }
            }
            Builtin::Shake => return self.native_shake(host, current, state, &args),
            Builtin::SetImage => return self.native_set_image(host, current, state, &args),
            // Trial 1.0.0.0 413F30/412D20 only parse arguments and call
            // 47BFB0 (query-name formatting); they never queue an operation or
            // write a target. These are original explicit no-ops, not stand-ins.
            Builtin::Disable | Builtin::SetChip => {}
            Builtin::SetBaseTextInterval => {
                // Original trial 414B00 -> 48EA70 updates the global interval
                // and its script-visible local mirror. Negative values do not
                // alter either setting. Existing text keeps its admitted clock.
                let interval = args.integer(0)?;
                if interval >= 0 {
                    self.machine.variables.insert(
                        "SYSTEM_text_interval".into(),
                        Value::Number(interval as f32),
                    );
                }
            }
            Builtin::SetShade => self.set_shade(host, current, &args)?,
            Builtin::SetBlur => self.native_blur(host, current, &args)?,
            Builtin::DrawEffect => return self.native_draw_effect(host, current, state, &args),
            Builtin::CreateFrame => return self.native_frame_image(host, current, state, &args),
            Builtin::CreateDifferenceTexture => {
                return self.native_difference(host, current, state, &args);
            }
            Builtin::CreateAnimation => {
                return self.native_animation_preload(host, current, state, &args);
            }
            Builtin::LoadFile | Builtin::UnloadFile => {
                return self.native_preload(host, state, &args, call);
            }
            Builtin::CreateScrollbar
            | Builtin::SetScrollbar
            | Builtin::ScrollbarValue
            | Builtin::SetScrollSpeed => {
                return self.native_scrollbar(host, current, state, &args, call);
            }
            Builtin::CreateFrameAnimation
            | Builtin::WaitFrameAnimation
            | Builtin::FinishFrameAnimation => {
                return self.native_frame_animation(host, current, state, &args, call);
            }
            Builtin::log
            | Builtin::LegacyUnsupported
            | Builtin::Twitter_Init
            | Builtin::Twitter_CheckAcount
            | Builtin::Twitter_InputAcount
            | Builtin::Twitter_SetAcount
            | Builtin::Twitter_InputTweet
            | Builtin::Twitter_Tweet
            | Builtin::XBOX360_LockVideo
            | Builtin::XBOX360_Presence
            | Builtin::XBOX360_AwardGameIcon
            | Builtin::XBOX360_Achieved
            | Builtin::XBOX360_IsAchieved
            | Builtin::XBOX360_InitUser
            | Builtin::XBOX360_IsSignin
            | Builtin::XBOX360_CheckStorage
            | Builtin::XBOX360_SelectStorage
            | Builtin::XBOX360_StorageSize
            | Builtin::XBOX360_UserIndex
            | Builtin::XBOX360_PadTrigger
            | Builtin::XBOX360_CurrentStorage
            | Builtin::Reset
            | Builtin::Reboot => return self.native_auxiliary(host, state, &args, call),
            Builtin::SoundAmplitude => {
                let character = args.string(1)?;
                result = Value::Number(if let Some(channel) = self.voices.get(&character) {
                    host.audio_info(self.client, *channel)?.map_or(0., |info| {
                        // Legacy media.cs uses four signed-16 source samples,
                        // not post-volume peak or a fabricated talking state.
                        (info
                            .source_taps
                            .iter()
                            .map(|s| (s * 32768.).clamp(-32768., 32767.).trunc().abs())
                            .sum::<f32>()
                            / 4.)
                            .trunc()
                    })
                } else {
                    0.
                });
            }
            Builtin::Draw
            | Builtin::DrawTransition
            | Builtin::CreateEffect
            | Builtin::WaitFade
            | Builtin::Rotate
            | Builtin::BezierMove
            | Builtin::SetVertex
            | Builtin::SetColor => {
                return self.native_graphics(host, current, state, &args, call);
            }
            Builtin::ClearBacklog => {
                self.history.clear();
                self.history_newline = false;
                self.history_revision = self
                    .history_revision
                    .checked_add(1)
                    .context("backlog revision")?;
            }
            Builtin::SetBacklog => {
                let mut plain = String::new();
                for part in crate::text::parse(&args.string(0)?, self.style.clone())? {
                    match part {
                        crate::text::Part::Text(s, _) | crate::text::Part::Ruby(s, _, _) => {
                            plain.push_str(&s)
                        }
                        _ => {}
                    }
                }
                self.append_backlog(plain)?;
            }
            Builtin::CreateBacklog => self.create_backlog(host, current, &args)?,
            Builtin::EnableBacklog => {
                result = Value::Boolean(!self.history.is_empty());
            }
            Builtin::CreateSound
            | Builtin::SoundEffect
            | Builtin::CreateMovie
            | Builtin::MoviePlay
            | Builtin::SetPan
            | Builtin::SetFrequency
            | Builtin::SetStream
            | Builtin::SetLoopPoint
            | Builtin::DurationTime
            | Builtin::PassageTime
            | Builtin::RemainTime => {
                return self.native_media(host, current, state, &args, call);
            }
            Builtin::Save
            | Builtin::Load
            | Builtin::ExistSave
            | Builtin::MountSavedata
            | Builtin::XBOX360_ExistContent
            | Builtin::DeleteSaveFile
            | Builtin::ReadFile
            | Builtin::WriteFile => {
                return self.native_storage(host, current, state, &args, call);
            }
            Builtin::Conquest => {
                // Shipped NSS uses a null scene to mark a whole module and
                // nullable third argument for query versus explicit set.
                let module = vm::module_name(&args.string(0)?)?;
                let scene = if matches!(args.value(1), Value::Null) {
                    String::new()
                } else {
                    args.string(1)?
                };
                ensure!(scene.len() <= 4096, "N2System read scene name");
                ensure!(
                    self.read_marks.len() < 4096 || self.read_marks.contains_key(&module),
                    "N2System read module capacity"
                );
                let marks = self.read_marks.entry(module).or_default();
                if !matches!(args.value(2), Value::Null) {
                    ensure!(
                        marks.len() < 32766 || marks.contains_key(&scene),
                        "N2System read mark capacity"
                    );
                    marks.insert(scene.clone(), args.truth(2)?);
                }
                result = Value::Boolean(
                    marks
                        .get(&scene)
                        .or_else(|| marks.get(""))
                        .copied()
                        .unwrap_or(false),
                );
            }
            Builtin::ClearScore => {
                let scope = args.string(0)?.to_ascii_uppercase();
                let table = match scope.as_str() {
                    "LOCAL" => &mut self.machine.variables,
                    "GLOBAL" => &mut self.machine.flags,
                    _ => bail!("N2System score scope {scope}"),
                };
                for value in table.values_mut() {
                    *value = Value::Number(0.);
                }
            }
            Builtin::Message
            | Builtin::System
            | Builtin::SetShortcut
            | Builtin::Random
            | Builtin::DateTime
            | Builtin::String
            | Builtin::MilliTime
            | Builtin::LockVideo
            | Builtin::IsInterrupt
            | Builtin::EnableDirect3DHAL
            | Builtin::EndScene => {
                return self.native_system(host, current, state, &args, call);
            }
            Builtin::CreateChoice => {
                self.create_choice(host, current, &args.string(0)?)?;
            }
            Builtin::SetNextFocus => {
                let first = self.query(current, &args.string(0)?)?;
                let second = self
                    .query(current, &args.string(1)?)?
                    .into_iter()
                    .find_map(|id| self.choice_owner(id));
                let direction = match args.constant(2)? {
                    "Left" => 0,
                    "Up" => 1,
                    "Right" => 2,
                    "Down" => 3,
                    name => bail!("N2System focus direction {name}"),
                };
                if let Some(second) = second {
                    for id in first {
                        if let Some(owner) = self.choice_owner(id) {
                            self.choices.get_mut(&owner).unwrap().next[direction] = Some(second);
                        }
                    }
                }
            }
            Builtin::CreateWindow
            | Builtin::SetTextOrientation
            | Builtin::CreateText
            | Builtin::LoadText
            | Builtin::SetFont
            | Builtin::WaitText => {
                return self.native_text(host, current, state, &args, call);
            }
            Builtin::LoadFont => return self.native_font_cache(host, current, state, &args),
            Builtin::Wait => {
                let ms = args.time(0)?;
                state.wait = if ms == 0 {
                    Wait::Frame(self.frame)
                } else {
                    Wait::Until(now.saturating_add(ms))
                };
            }
            Builtin::WaitKey => {
                state.wait = Wait::Input(if args.len() == 0 {
                    None
                } else {
                    Some(now.saturating_add(args.time(0)?))
                });
            }
            Builtin::CreateName => {
                self.create(host, current, &args.string(0)?, Kind::Name, [0.; 2])?;
            }
            Builtin::CreateProcess => {
                ensure!(self.threads.len() < 255, "N2System thread capacity");
                let target = args.string(4)?;
                let mut thread =
                    self.machine
                        .start(&call.module, Some(&target), args.string(0)?)?;
                thread.scopes = state.thread.inherited_scopes()?;
                let id = self.create(host, current, &args.string(0)?, Kind::Thread, [0.; 2])?;
                // CreateProcess declares a suspended thread. Request(Start)
                // starts execution; coordinates are consumed by the NSS ABI.
                let mut execution = Execution::new(thread);
                execution.enabled = false;
                execution.interrupt = state.interrupt;
                self.threads.insert(id, execution);
            }
            Builtin::SetAlias => {
                for id in self.query(current, &args.string(0)?)? {
                    self.entities.alias(id, args.string(1)?)?;
                }
            }
            Builtin::Delete => {
                for id in self.query(current, &args.string(0)?)? {
                    if self.entities.nodes.get(&id).is_some_and(|e| !e.locked) {
                        self.destroy(host, id)?;
                    }
                }
            }
            Builtin::LoadImage | Builtin::CreateTexture | Builtin::CreateClipTexture => {
                let slot = match call.function {
                    Builtin::LoadImage => 1,
                    Builtin::CreateTexture => 4,
                    _ => 8,
                };
                if args.string(slot)?.eq_ignore_ascii_case("SCREEN") {
                    return self.capture_visual(host, current, state, &args, call);
                }
                let source = args.string(slot)?;
                if resource::resolve(self.catalog.as_ref().unwrap(), &source).is_err()
                    && self
                        .query(current, &source)?
                        .iter()
                        .any(|id| self.shades.get(id).is_some_and(shade::Shade::pending))
                {
                    return Ok(false);
                }
                let mut surface_lease = None;
                let surface = self.surface_source(current, &source)?;
                if state
                    .surface_source
                    .as_ref()
                    .is_some_and(|(id, _)| Some(*id) != surface)
                {
                    state.surface_source = None;
                }
                let image = if let Some(id) = surface {
                    state.image = None;
                    if state.surface_source.is_none() {
                        state.surface_source = Some((id, self.begin_surface_image(host, id)?));
                    }
                    let image = match state.surface_source.as_mut().unwrap().1.poll()? {
                        crysirror_core::image_processing::ImageProcessState::Pending => {
                            return Ok(false);
                        }
                        crysirror_core::image_processing::ImageProcessState::Failed(error) => {
                            bail!("N2System surface source: {error}")
                        }
                        crysirror_core::image_processing::ImageProcessState::Ready(image) => image,
                    };
                    let size = image.size();
                    surface_lease = Some(image);
                    state.surface_source = None;
                    Image {
                        paint: json!({"kind":"image","asset":""}),
                        size,
                    }
                } else if let Some(task) = state.image.as_mut() {
                    let Some(image) = task.try_take()? else {
                        return Ok(false);
                    };
                    state.image = None;
                    image
                } else {
                    let Some(image) = self.source_image(current, &args.string(slot)?)? else {
                        // Original 40C140: when neither a resolvable bitmap nor
                        // a live entity matches, no entity is created and the
                        // script continues (e.g. the "VIDEO" source, which has
                        // no string in the trial binary).
                        host.append_log(
                            self.client,
                            &format!(
                                "N2System: {:?} has no image source {:?}; nothing created",
                                call.function,
                                args.string(slot)?
                            ),
                        )?;
                        self.machine.complete(&mut state.thread, Value::Null)?;
                        return Ok(true);
                    };
                    if image.size == [0; 2] {
                        let metadata = self.images.as_ref().unwrap().clone();
                        let path = image.paint["asset"]
                            .as_str()
                            .context("N2System image asset identity")?
                            .to_owned();
                        match self.pool.as_ref().unwrap().spawn(move |cancel| {
                            cancel.check()?;
                            let size = metadata.size(&path)?;
                            cancel.check()?;
                            ensure!(
                                size.iter().all(|n| *n > 0 && *n <= 16384),
                                "N2System image dimensions"
                            );
                            Ok(Image {
                                paint: json!({"kind":"image","asset":path}),
                                size,
                            })
                        }) {
                            Ok(task) => state.image = Some(task),
                            Err(e)
                                if e.downcast_ref::<crysirror_tasks::CapacityExceeded>()
                                    .is_some() => {}
                            Err(e) => return Err(e),
                        }
                        return Ok(false);
                    }
                    image
                };
                if call.function == Builtin::LoadImage {
                    let format = surface.and_then(|id| self.surfaces.get(&id)).map_or_else(
                        || self.bitmap_format(&image.paint),
                        |s| bitmap_read::BitmapFormat {
                            origin: None,
                            alpha: Some(!s.opaque),
                        },
                    );
                    let source = if let Some(source) = surface_lease {
                        Some(source)
                    } else {
                        match image.paint.get("asset").and_then(Json::as_str) {
                            Some(path) => host.published_image_resource(self.client, path)?,
                            None => None,
                        }
                    };
                    let id = self.create(
                        host,
                        current,
                        &args.string(0)?,
                        Kind::Image {
                            paint: image.paint,
                            size: image.size,
                        },
                        image.size.map(|n| n as f32),
                    )?;
                    if let Some(source) = source {
                        let uploaded = host.publish_image_resource(self.client, id, source)?;
                        let path = uploaded.path.relative().to_owned();
                        if let Kind::Image { paint, .. } =
                            &mut self.entities.nodes.get_mut(&id).unwrap().kind
                        {
                            paint["asset"] = json!(path);
                        }
                        self.captured_images.insert(id, path);
                    }
                    self.bitmap_formats.insert(id, format);
                } else {
                    let (size, uv) = if call.function == Builtin::CreateClipTexture {
                        let x = args.integer(4)?;
                        let y = args.integer(5)?;
                        let w = args.integer(6)?;
                        let h = args.integer(7)?;
                        ensure!(
                            x >= 0
                                && y >= 0
                                && w > 0
                                && h > 0
                                && i64::from(x) + i64::from(w) <= i64::from(image.size[0])
                                && i64::from(y) + i64::from(h) <= i64::from(image.size[1]),
                            "N2System source clip outside image"
                        );
                        (
                            [w as u32, h as u32],
                            Some([
                                x as f32 / image.size[0] as f32,
                                y as f32 / image.size[1] as f32,
                                (x + w) as f32 / image.size[0] as f32,
                                (y + h) as f32 / image.size[1] as f32,
                            ]),
                        )
                    } else {
                        (image.size, None)
                    };
                    let surface_format = surface.and_then(|id| self.surfaces.get(&id)).map(|s| {
                        bitmap_read::BitmapFormat {
                            origin: None,
                            alpha: Some(!s.opaque),
                        }
                    });
                    let id = self.visual_owned(
                        host,
                        current,
                        &args,
                        image.paint,
                        size,
                        uv,
                        surface_lease,
                    )?;
                    if let Some(format) = surface_format {
                        self.bitmap_formats.insert(id, format);
                    }
                }
            }
            Builtin::LoadColor | Builtin::CreateColor => {
                let offset = if call.function == Builtin::LoadColor {
                    1
                } else {
                    4
                };
                let width = args.integer(offset)?;
                let height = args.integer(offset + 1)?;
                ensure!(
                    (1..=16384).contains(&width) && (1..=16384).contains(&height),
                    "N2System color extent"
                );
                let color = args.color(if args.len() == 8 { 7 } else { offset + 2 })?;
                let paint = json!({"kind":"solid","color":color});
                if call.function == Builtin::LoadColor {
                    self.create(
                        host,
                        current,
                        &args.string(0)?,
                        Kind::Image {
                            paint,
                            size: [width as u32, height as u32],
                        },
                        [width as f32, height as f32],
                    )?;
                } else {
                    self.visual(
                        host,
                        current,
                        &args,
                        paint,
                        [width as u32, height as u32],
                        None,
                    )?;
                }
            }
            Builtin::ImageHorizon | Builtin::ImageVertical => {
                let size = self
                    .query(current, &args.string(0)?)?
                    .first()
                    .and_then(|id| self.entities.nodes.get(id))
                    .map(|e| e.size)
                    .unwrap_or([0.; 2]);
                result = Value::Number(size[usize::from(call.function == Builtin::ImageVertical)]);
            }
            Builtin::Position => {
                let point = if let Some(id) = self.query(current, &args.string(0)?)?.first() {
                    let l = self.layer(host, *id)?;
                    [
                        l["rect"][0].as_f64().unwrap_or(0.),
                        l["rect"][1].as_f64().unwrap_or(0.),
                    ]
                } else {
                    [0.; 2]
                };
                self.machine
                    .assign(args.reference(1)?, Value::Number(point[0].trunc() as f32))?;
                self.machine
                    .assign(args.reference(2)?, Value::Number(point[1].trunc() as f32))?;
            }
            Builtin::CursorPosition => {
                self.machine
                    .assign(args.reference(0)?, Value::Number(input.pointer_x as f32))?;
                self.machine
                    .assign(args.reference(1)?, Value::Number(input.pointer_y as f32))?;
            }
            Builtin::MoveCursor => {
                host.warp_pointer(self.client, args.integer(0)?, args.integer(1)?)?;
            }
            Builtin::Move | Builtin::Zoom | Builtin::Fade => {
                let ids = self.query(current, &args.string(0)?)?;
                let (duration, values, ease, wait, properties) = match call.function {
                    Builtin::Move => (
                        args.time(1)?,
                        vec![args.value(2).clone(), args.value(3).clone()],
                        if args.len() == 6 { 4 } else { usize::MAX },
                        if args.len() == 6 { 5 } else { usize::MAX },
                        vec!["x", "y"],
                    ),
                    Builtin::Zoom => (
                        args.time(1)?,
                        vec![
                            Value::Number(args.integer(2)? as f32 / 1000.),
                            Value::Number(args.integer(3)? as f32 / 1000.),
                        ],
                        4,
                        5,
                        vec!["scale_x", "scale_y"],
                    ),
                    _ => {
                        if args.len() == 3 {
                            (
                                0,
                                vec![Value::Number(args.integer(1)? as f32 * 255. / 1000.)],
                                usize::MAX,
                                usize::MAX,
                                vec!["opacity"],
                            )
                        } else {
                            let swapped = args.len() > 4
                                && matches!(args.value(args.len() - 1), Value::Constant(_));
                            (
                                args.time(1)?,
                                vec![Value::Number(args.integer(2)? as f32 * 255. / 1000.)],
                                if args.len() == 4 {
                                    usize::MAX
                                } else if swapped {
                                    4
                                } else {
                                    3
                                },
                                if args.len() == 4 || swapped { 3 } else { 4 },
                                vec!["opacity"],
                            )
                        }
                    }
                };
                let easing = easing(&args, ease)?;
                let mut operations = Vec::new();
                let mut animated = BTreeSet::new();
                for id in ids {
                    for node in self.entities.animation_targets(id) {
                        if !animated.insert(node) {
                            continue;
                        }
                        if call.function == Builtin::Zoom
                            && self.animate_cube(host, node, &args, call.function)?
                        {
                            continue;
                        }
                        if call.function == Builtin::Move {
                            self.paths.remove(&node);
                        }
                        let layer = self.layer(host, node)?;
                        let size = self.entities.nodes[&node].size;
                        for (axis, property) in properties.iter().enumerate() {
                            let to = if call.function == Builtin::Move {
                                self.native_position(
                                    host,
                                    node,
                                    &values[axis],
                                    layer["rect"][axis]
                                        .as_f64()
                                        .context("N2System layer position")?
                                        as f32,
                                    size[axis],
                                    axis,
                                )?
                            } else {
                                values[axis].number()?
                            };
                            operations.push(json!({"op":"begin_motion","id":node,"track":{"property":property,"to":to,"duration":duration,"easing":easing}}));
                            self.record_blur_motion(
                                host,
                                node,
                                property,
                                to,
                                duration,
                                easing.clone(),
                            )?;
                        }
                    }
                }
                self.submit(host, operations, vec![])?;
                let timeout = args.anim_wait(wait, duration)?;
                if timeout > 0 {
                    state.wait = Wait::Motion {
                        ids: animated.into_iter().collect(),
                        properties,
                        until: Some(now.saturating_add(timeout)),
                    };
                }
            }
            Builtin::WaitMove | Builtin::WaitAction => {
                let roots = self.query(current, &args.string(0)?)?;
                if call.function == Builtin::WaitAction {
                    let ids = roots
                        .into_iter()
                        .flat_map(|id| self.entities.subtree(id))
                        .filter(|id| *id != current)
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .collect();
                    let timeout = if matches!(args.value(1),Value::String(s) if s=="nulll") {
                        0
                    } else {
                        args.time(1)?
                    };
                    state.wait = Wait::Actions {
                        ids,
                        until: (timeout > 0).then(|| now.saturating_add(timeout)),
                    };
                    self.machine.complete(&mut state.thread, Value::Null)?;
                    return Ok(true);
                }
                let ids = roots;
                let properties = if call.function == Builtin::WaitMove {
                    vec!["x", "y"]
                } else {
                    Vec::new()
                };
                let timeout = args.time(1)?;
                state.wait = Wait::Motion {
                    ids,
                    properties,
                    until: if timeout > 0 {
                        Some(now.saturating_add(timeout))
                    } else {
                        None
                    },
                };
            }
            Builtin::SetLoop | Builtin::SetVolume => {
                for id in self.query(current, &args.string(0)?)? {
                    if let Kind::Sound {
                        gain,
                        looping,
                        playing,
                        ..
                    } = &mut self.entities.nodes.get_mut(&id).unwrap().kind
                    {
                        if call.function == Builtin::SetLoop {
                            *looping = args.truth(1)?;
                            if *playing {
                                host.submit_audio(
                                    self.client,
                                    AudioRequest::SetLooped {
                                        channel: id,
                                        looping: *looping,
                                        loop_start: None,
                                    },
                                )?;
                            }
                        } else {
                            *gain = args.integer(2)? as f32 / 1000.;
                            ensure!((0. ..=4.).contains(gain), "N2System audio gain range");
                            if *playing {
                                host.submit_audio(
                                    self.client,
                                    AudioRequest::SetGain {
                                        channel: id,
                                        gain: *gain,
                                        fade_ms: args.time(1)? as f32,
                                    },
                                )?;
                            }
                        }
                    }
                }
            }
            Builtin::WaitPlay => {
                let mut ids = BTreeSet::new();
                for root in self.query(current, &args.string(0)?)? {
                    for id in self.entities.subtree(root) {
                        if self.playback_wait_active(host, id)? {
                            ids.insert(id);
                        }
                    }
                }
                state.wait = Wait::Audio {
                    ids: ids.into_iter().collect(),
                    // Original 411F50 reads this operand as a time string where
                    // null/"null"/"true" mean no limit; null is therefore valid.
                    allow_movie_skip: args.len() > 1
                        && !matches!(args.value(1), Value::Null)
                        && args.truth(1)?,
                };
            }
            Builtin::Request => {
                let action = args.constant(1)?;
                if action == "Play" {
                    for id in self.query(current, &args.string(0)?)? {
                        if self.sound_resources.contains_key(&id) && !self.sound_ready(host, id)? {
                            return Ok(false);
                        }
                    }
                }
                for id in self.query(current, &args.string(0)?)? {
                    if self.choices.contains_key(&id) && matches!(action, "Enter" | "Erase") {
                        let enabled = action == "Enter";
                        let choice = self.choices.get_mut(&id).unwrap();
                        choice.disabled = !enabled;
                        if !enabled {
                            choice.state = 0;
                            let mut layers = Vec::new();
                            for (index, name) in choice::STATES[..3].iter().enumerate() {
                                if let Some(group) = self.entities.child(id, name) {
                                    layers.push(
                                        json!({"op":"set_visible","id":group,"visible":true}),
                                    );
                                    layers.push(json!({"op":"begin_motion","id":group,"track":{
                                        "property":"opacity","to":if index==0 {255}else{0},"duration":0}}));
                                }
                            }
                            self.submit(host, layers, vec![])?;
                        }
                        continue;
                    }
                    if action == "Smoothing" {
                        // Native sprites already use the shared linear sampler.
                        ensure!(
                            self.layer(host, id)?["sampling"]["linear"] == true,
                            "N2System linear sampling unavailable"
                        );
                        continue;
                    }
                    if action == "Hideable" {
                        self.entities.nodes.get_mut(&id).unwrap().hideable = true;
                        continue;
                    }
                    if action == "Passive" {
                        let effect = self
                            .effects
                            .get_mut(&id)
                            .context("N2System Passive requires a Plain effect")?;
                        effect.passive = true;
                        effect.pending = None;
                        continue;
                    }
                    if action == "Lock" || action == "Unlock" {
                        self.entities.nodes.get_mut(&id).unwrap().locked = action == "Lock";
                        continue;
                    }
                    if action == "NoLog" {
                        self.no_log.insert(id);
                        continue;
                    }
                    if action == "Disused" {
                        self.disused.insert(id);
                        continue;
                    }
                    if action == "PushText" {
                        if let Some(page) = self.pages.get_mut(&id) {
                            page.disable_animation = true;
                            self.finish_text_fade(host, id)?;
                            self.submit(
                                host,
                                vec![json!({"op":"complete_reveal","id":id})],
                                vec![],
                            )?;
                            continue;
                        }
                    }
                    if action == "Enter" {
                        if let Some(thread) = self.pages.get(&id).and_then(|p| p.thread) {
                            if let Some(execution) = self.threads.get_mut(&thread) {
                                execution.enabled = true;
                            }
                        }
                    }
                    if let Some(movie) = self.movies.get(&id).cloned() {
                        use crysirror_core::service::MediaRequest;
                        let request = match action {
                            "Play" => MediaRequest::Play {
                                id,
                                asset: movie.source,
                                looping: movie.looping,
                                audio: true,
                                gain: movie.gain,
                            },
                            "Stop" => MediaRequest::Stop { id },
                            "Pause" => MediaRequest::Pause { id },
                            "Resume" => MediaRequest::Resume { id },
                            _ => {
                                // Visibility, locking and blend requests still
                                // follow the ordinary retained visual branch.
                                let operation = match action {
                                    "Enter" | "Erase" => {
                                        json!({"op":"set_visible","id":id,"visible":action=="Enter"})
                                    }
                                    "AddRender" => json!({"op":"set_blend","id":id,"blend":"add"}),
                                    "SubRender" => {
                                        json!({"op":"set_blend","id":id,"blend":"subtract"})
                                    }
                                    "MulRender" => {
                                        json!({"op":"set_blend","id":id,"blend":"multiply"})
                                    }
                                    _ => bail!("N2System Request {action} on movie"),
                                };
                                self.submit(host, vec![operation], vec![])?;
                                continue;
                            }
                        };
                        let surface = host.media_request(self.client, request)?;
                        if action == "Play" {
                            let path = surface.relative().to_owned();
                            self.movies.get_mut(&id).unwrap().surface = path.clone();
                            self.submit(host,vec![json!({"op":"set_paint","id":id,"paint":{"kind":"image","asset":path}})],vec![])?;
                        }
                        continue;
                    }
                    if self.frame_animations.contains_key(&id)
                        && matches!(action, "Start" | "Resume" | "Stop")
                    {
                        self.control_frame_animation(host, id, action == "Stop")?;
                        continue;
                    }
                    if matches!(self.entities.nodes[&id].kind, Kind::Thread) {
                        let target = if id == current {
                            &mut *state
                        } else {
                            self.threads
                                .get_mut(&id)
                                .context("N2System thread entity has no execution state")?
                        };
                        match action {
                            "Start" => {
                                // Start resumes an existing process; only a
                                // completed process needs a fresh entry stack.
                                if target.thread.halted {
                                    let first = target.entry.clone();
                                    let scopes = target.thread.scopes.clone();
                                    target.thread = self.machine.start(
                                        &first.module,
                                        Some(&first.routine),
                                        target.thread.name.clone(),
                                    )?;
                                    target.thread.scopes = scopes;
                                    target.wait = Wait::None;
                                }
                                target.enabled = true;
                            }
                            "Resume" | "Enter" => target.enabled = true,
                            "Pause" | "Stop" | "Erase" => target.enabled = false,
                            _ => bail!("N2System Request {action} on thread"),
                        }
                    } else if let Kind::Sound {
                        source,
                        kind,
                        gain,
                        looping,
                        playing,
                    } = &mut self.entities.nodes.get_mut(&id).unwrap().kind
                    {
                        match action {
                            "Play" => {
                                let _ = (source, kind, gain, looping, playing);
                                self.play_sound(host, state, id)?;
                            }
                            "Stop" => {
                                host.submit_audio(
                                    self.client,
                                    AudioRequest::Stop {
                                        channel: id,
                                        fade_ms: 0.,
                                    },
                                )?;
                                *playing = false;
                                if let Some(resource) = self.sound_resources.get_mut(&id) {
                                    if let Some(handle) = resource.handle.take() {
                                        host.release_audio_resource(self.client, handle)?;
                                    }
                                }
                            }
                            "Pause" | "Resume" => {
                                host.submit_audio(
                                    self.client,
                                    AudioRequest::Pause {
                                        channel: id,
                                        paused: action == "Pause",
                                        fade_ms: 0.,
                                    },
                                )?;
                            }
                            _ => bail!("N2System Request {action} on audio"),
                        }
                    } else {
                        let operation = match action {
                            "Enter" | "Erase" => {
                                if action == "Erase" {
                                    self.hidden_for_text.remove(&id);
                                }
                                json!({"op":"set_visible","id":id,"visible":action=="Enter"})
                            }
                            "AddRender" => json!({"op":"set_blend","id":id,"blend":"add"}),
                            "SubRender" => json!({"op":"set_blend","id":id,"blend":"subtract"}),
                            "MulRender" => json!({"op":"set_blend","id":id,"blend":"multiply"}),
                            _ => bail!("N2System Request {action} on visual entity"),
                        };
                        self.submit(host, vec![operation], vec![])?;
                    }
                }
                if !state.audio.is_empty() {
                    return Ok(false);
                }
            }
            Builtin::Time => {
                result =
                    Value::Number((now.saturating_sub(self.start_ms.unwrap_or(now)) / 1000) as f32);
            }
            Builtin::Platform => result = Value::Number(self.config.platform as f32),
            Builtin::ModuleFileName => result = Value::String(format!("{}.nss", call.module)),
            Builtin::Integer => {
                result = match args.value(0) {
                    Value::Number(n) => Value::Number(n.trunc()),
                    v => v.clone(),
                };
            }
            Builtin::AvailableFile => {
                result = Value::Boolean(
                    self.catalog
                        .as_ref()
                        .unwrap()
                        .contains_key(&resource::asset_key(&args.string(0)?)?),
                );
            }
            Builtin::assert => {
                ensure!(args.truth(0)?, "N2System script assertion failed");
            }
            Builtin::assert_eq => {
                ensure!(
                    args.value(0)
                        .binary(crate::value::Binary::Equal, args.value(1))?
                        .truth()?,
                    "N2System script equality assertion failed"
                );
            }
            Builtin::fail | Builtin::fail_msg => {
                bail!("N2System script failure: {}", args.string(0)?)
            }
            Builtin::Exit => {
                state.thread.halted = true;
                for execution in self.threads.values_mut() {
                    execution.thread.halted = true;
                }
            }
            function => bail!(
                "N2System native service {} is not connected ({}::{} instruction {})",
                function.name(),
                call.module,
                call.routine,
                call.instruction
            ),
        }
        self.machine.complete(&mut state.thread, result)?;
        Ok(true)
    }
    fn awake(
        &self,
        host: &mut dyn HostServices,
        state: &Execution,
        input: &InputFrame,
    ) -> Result<bool> {
        let now = host.now_ms();
        Ok(match &state.wait {
            Wait::None => true,
            Wait::Frame(frame) => self.frame > *frame,
            Wait::Until(at) => now >= *at,
            Wait::Input(deadline) => {
                input.buttons_pressed & (buttons::MOUSE_LEFT | buttons::ENTER | buttons::SPACE) != 0
                    || deadline.is_some_and(|at| now >= at)
            }
            Wait::Join(id) => self.threads.get(id).is_none_or(|e| e.thread.halted),
            Wait::Text { ids, until } => {
                until.is_some_and(|at| now >= at)
                    || ids.iter().all(|id| {
                        self.pages.get(id).is_none_or(|page| {
                            page.line_read
                                && page.thread.is_none_or(|thread| {
                                    self.threads
                                        .get(&thread)
                                        .is_none_or(|state| state.thread.halted)
                                })
                        })
                    })
            }
            Wait::Line(id) => self.pages.get(id).is_none_or(|page| page.line_read),
            Wait::Shakes { ids, until } => {
                until.is_some_and(|at| now >= at)
                    || ids.iter().all(|id| !self.shakes.contains_key(id))
            }
            Wait::Frames(ids) => ids.iter().all(|id| {
                self.frame_animations
                    .get(id)
                    .is_none_or(|a| a.clock.complete)
            }),
            Wait::Audio { ids, .. } => {
                let mut done = true;
                for id in ids {
                    done &= !self.playback_wait_active(host, *id)?;
                }
                done
            }
            Wait::Actions { ids, until } => {
                if until.is_some_and(|at| now >= at) {
                    true
                } else {
                    let mut done = true;
                    for &id in ids {
                        done &= !self.action_wait_active(host, id)?;
                    }
                    done
                }
            }
            Wait::Motion {
                ids,
                properties,
                until,
            } => {
                if until.is_some_and(|at| now >= at) {
                    true
                } else {
                    let mut done = true;
                    for id in ids {
                        if !self.entities.nodes.contains_key(id) {
                            continue;
                        }
                        let status: Json = serde_json::from_slice(
                            &host.query_engine(self.client, EngineQuery::LayerMotion(*id))?,
                        )?;
                        let tracks = status["tracks"]
                            .as_array()
                            .context("N2System invalid shared motion status")?;
                        done &= !tracks.iter().any(|t| {
                            t["complete"] != true
                                && (properties.is_empty()
                                    || properties.iter().any(|p| t["property"] == *p))
                        });
                        if let Some(blur) = self.blurs.get(id) {
                            done &= !blur.active(now)?;
                        }
                        if let Some(cube) = self.cubes.get(id) {
                            done &= !cube.active(now, properties);
                        }
                        if let Some(camera) = self.cameras.get(id) {
                            done &= !camera.active_channels(now, properties);
                        }
                        if properties.is_empty()
                            || properties.contains(&"x")
                            || properties.contains(&"y")
                        {
                            done &= self
                                .paths
                                .get(id)
                                .is_none_or(|track| track.progress.complete(now));
                        }
                        if properties.is_empty() {
                            done &= self
                                .mask_transitions
                                .get(id)
                                .is_none_or(|t| t.progress.complete(now));
                        }
                    }
                    done
                }
            }
        })
    }
    fn retire_completed(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let retired = self
            .threads
            .iter()
            .filter(|(_, s)| s.retire_when_done && s.thread.halted)
            .map(|(&id, _)| id)
            .collect::<Vec<_>>();
        for id in retired {
            if let Some(state) = self.threads.remove(&id) {
                self.release_execution(host, state)?;
            }
            // Scene resources may outlive their calling script. Retire only
            // the execution; keep their native ownership group until empty.
            if !self.entities.nodes.values().any(|e| e.parent == Some(id)) {
                self.destroy(host, id)?;
            }
        }
        for id in self.disused.iter().copied().collect::<Vec<_>>() {
            let Some(entity) = self.entities.nodes.get(&id) else {
                self.disused.remove(&id);
                continue;
            };
            let idle = if let Some(page) = self.pages.get(&id) {
                page.line_read
                    && page
                        .thread
                        .is_none_or(|t| self.threads.get(&t).is_none_or(|s| s.thread.halted))
            } else if matches!(entity.kind, Kind::Thread) {
                self.threads.get(&id).is_none_or(|s| s.thread.halted)
            } else if matches!(entity.kind, Kind::Sound { .. }) {
                host.audio_info(self.client, id)?
                    .is_none_or(|a| !a.source_playing || (a.gain == 0. && a.gain_fade_frames == 0))
            } else if matches!(entity.kind, Kind::Visual) {
                let motion: Json = serde_json::from_slice(
                    &host.query_engine(self.client, EngineQuery::LayerMotion(id))?,
                )?;
                !motion["tracks"]
                    .as_array()
                    .context("N2System motion receipt")?
                    .iter()
                    .any(|t| t["complete"] != true)
                    && self
                        .paths
                        .get(&id)
                        .is_none_or(|p| p.progress.complete(host.now_ms()))
                    && self
                        .mask_transitions
                        .get(&id)
                        .is_none_or(|p| p.progress.complete(host.now_ms()))
                    && self
                        .frame_animations
                        .get(&id)
                        .is_none_or(|a| a.clock.complete)
                    && self.shades.get(&id).is_none_or(|shade| !shade.pending())
                    && !self.shakes.contains_key(&id)
            } else {
                true
            };
            if idle {
                self.destroy(host, id)?;
            }
        }
        Ok(())
    }
    fn step_inner(
        &mut self,
        host: &mut dyn HostServices,
        mut input: InputFrame,
        budget: u32,
    ) -> Result<VmYield> {
        self.start_ms.get_or_insert(host.now_ms());
        if !self.prepare(host)? {
            return Ok(VmYield::WaitUntil(host.now_ms().saturating_add(1)));
        }
        self.frame = self
            .frame
            .checked_add(1)
            .context("N2System frame clock exhausted")?;
        if self.modal_movie.is_some() {
            self.advance_modal_movie(host, &mut input)?;
            if self.modal_movie.is_some() {
                return Ok(VmYield::WaitUntil(host.now_ms().saturating_add(1)));
            }
        }
        let was_modal = self.native_dialog.is_some();
        if !was_modal {
            self.interrupt_movie_wait(host, &mut input)?;
            self.interrupt_frame_waits(&mut input);
        }
        self.update_native_system(host, &input)?;
        self.sample_paths(host)?;
        self.sample_cameras(host)?;
        self.sample_cubes(host)?;
        self.sample_blurs(host)?;
        self.update_ripples(host)?;
        self.sample_shakes(host)?;
        self.sample_frame_animations(host)?;
        self.sample_mask_transitions(host)?;
        self.update_effects(host)?;
        self.update_lenses(host)?;
        self.update_particles(host)?;
        self.update_legacy_transitions(host)?;
        self.update_restored_sounds(host)?;
        self.update_shades(host)?;
        self.sample_audio_rates(host)?;
        if was_modal {
            input.buttons_down = 0;
            input.buttons_pressed = 0;
        }
        self.refresh_choices(host)?;
        self.update_scrollbars(host, &input)?;
        self.update_backlogs(host)?;
        self.poll_text_layout(host)?;
        self.sample_text_fades(host)?;
        self.update_text_input(host, &input)?;
        let ids: Vec<_> = self.threads.keys().copied().collect();
        let quantum = (budget / (ids.len().max(1) as u32)).max(1);
        let mut remaining = budget;
        for id in ids {
            if self.reset_requested || self.pending_restore.is_some() {
                break;
            }
            if remaining == 0 {
                break;
            }
            let Some(mut state) = self.threads.remove(&id) else {
                continue;
            };
            let result = (|| -> Result<()> {
                if !state.enabled || state.thread.halted || !self.awake(host, &state, &input)? {
                    return Ok(());
                }
                if self.native_dialog.is_some_and(|(_, owner)| owner != id)
                    || (self.interrupt.is_some() && !state.interrupt)
                {
                    return Ok(());
                }
                state.wait = Wait::None;
                if let Some((page, revision)) = state.pending_text {
                    if self.pages.contains_key(&page)
                        && self
                            .text_published
                            .get(&page)
                            .is_none_or(|published| *published < revision)
                    {
                        return Ok(());
                    }
                    state.pending_text = None;
                    if state.thread.pending.is_some() {
                        self.machine.complete(&mut state.thread, Value::Null)?;
                    }
                }
                if let Some(task) = &mut state.module {
                    let Some(modules) = task.try_take()? else {
                        return Ok(());
                    };
                    for (name, module) in modules {
                        self.machine.insert(&name, module)?;
                    }
                    state.module = None;
                }
                if !state.audio.is_empty() {
                    let mut pending = Vec::new();
                    for ticket in std::mem::take(&mut state.audio) {
                        let info = host.audio_preparation(self.client, ticket)?;
                        match info.phase {
                            AudioPreparePhase::Pending => pending.push(ticket),
                            AudioPreparePhase::Ready => {
                                self.connect_sound_effects(host, info.channel, state.dsp_paused.remove(&ticket))?;
                                host.release_audio(self.client, ticket)?;
                            }
                            AudioPreparePhase::Failed => {
                                host.release_audio(self.client, ticket)?;
                                bail!(
                                    "N2System audio preparation: {}",
                                    info.error.unwrap_or_default()
                                );
                            }
                        }
                    }
                    state.audio = pending;
                    if !state.audio.is_empty() {
                        return Ok(());
                    }
                    self.machine.complete(&mut state.thread, Value::Null)?;
                }
                let allowance = quantum.min(remaining);
                for _ in 0..allowance {
                    remaining -= 1;
                    self.executed = self.executed.saturating_add(1);
                    match self.machine.step(&mut state.thread, 1)? {
                        Step::Budget => {}
                        Step::Halted => break,
                        Step::Frame => {
                            state.wait = Wait::Frame(self.frame);
                            break;
                        }
                        Step::Call(call) => {
                            let complete = self
                                .native(host, id, &mut state, &input, &call)
                                .with_context(|| {
                                    format!(
                                        "N2System {}::{} instruction {} ({})",
                                        call.module,
                                        call.routine,
                                        call.instruction,
                                        call.function.name()
                                    )
                                })?;
                            if !complete
                                || !matches!(state.wait, Wait::None)
                                || !state.enabled
                                || !self.entities.nodes.contains_key(&id)
                            {
                                break;
                            }
                        }
                        Step::Load(module) => {
                            let storage = self.storage.as_ref().unwrap().clone();
                            let catalog = self.catalog.as_ref().unwrap().clone();
                            let config = self.config.clone();
                            let existing = self.machine.modules.keys().cloned().collect();
                            match self.pool.as_ref().unwrap().spawn(move |cancel| {
                                resource::modules(
                                    storage,
                                    catalog,
                                    &config.script_root,
                                    &module,
                                    &config.encoding,
                                    existing,
                                    &cancel,
                                )
                            }) {
                                Ok(task) => state.module = Some(task),
                                Err(e)
                                    if e.downcast_ref::<crysirror_tasks::CapacityExceeded>()
                                        .is_some() => {}
                                Err(e) => return Err(e),
                            }
                            break;
                        }
                        Step::Scene(module, routine) => {
                            ensure!(self.threads.len() < 255, "N2System scene thread capacity");
                            let child = self.entities.root(routine.clone())?;
                            self.submit(host, vec![self.entity_group(child)?], vec![])?;
                            let mut child_state = Execution::new(self.machine.start(
                                &module,
                                Some(&routine),
                                routine.clone(),
                            )?);
                            child_state.interrupt = state.interrupt;
                            child_state.retire_when_done = true;
                            child_state.thread.scopes = state.thread.inherited_scopes()?;
                            self.threads.insert(child, child_state);
                            state.wait = Wait::Join(child);
                            break;
                        }
                        Step::ClearText => {
                            let id = state
                                .page
                                .context("N2System ClearPage outside a dialogue thread")?;
                            self.clear_text(host, id)?;
                        }
                        Step::AppendText(markup) => {
                            let id = state
                                .page
                                .context("N2System AppendDialogue outside a dialogue thread")?;
                            self.append_text(host, id, &markup)?;
                            state.pending_text = Some((id, self.pages[&id].revision));
                            break;
                        }
                        Step::LineEnd => {
                            let id = state
                                .page
                                .context("N2System LineEnd outside a dialogue thread")?;
                            state.wait = Wait::Line(id);
                            self.history_newline = true;
                            break;
                        }
                        Step::Choice(query) => {
                            let pressed = self.choice_input(host, id, &query, &input)?;
                            state.thread.choice_result(pressed)?;
                        }
                        Step::SelectEnd => self.end_selection(host)?,
                    }
                }
                Ok(())
            })();
            if self.entities.nodes.contains_key(&id) {
                self.threads.insert(id, state);
            } else {
                self.release_execution(host, state)?;
            }
            result?;
            if self.modal_movie.is_some() {
                break;
            }
        }
        if let Some(restore) = self.pending_restore.take() {
            self.apply_restore(host, *restore)?;
            return Ok(VmYield::WaitUntil(host.now_ms().saturating_add(1)));
        }
        if self.reset_requested {
            if self.reboot_requested {
                self.reboot_native_session(host)?;
            } else {
                self.reset_native_world(host)?;
            }
            return Ok(VmYield::WaitUntil(host.now_ms().saturating_add(1)));
        }
        self.update_alpha_masks(host)?;
        self.retire_completed(host)?;
        self.machine
            .collect_arrays(self.threads.values().map(|state| &state.thread));
        if self.threads.values().all(|e| e.thread.halted) {
            Ok(VmYield::Halted)
        } else {
            Ok(VmYield::WaitUntil(host.now_ms().saturating_add(1)))
        }
    }
}

fn easing(args: &Arguments<'_>, at: usize) -> Result<Json> {
    if at >= args.len() || matches!(args.value(at), Value::Null) {
        return Ok(json!("linear"));
    }
    Ok(match args.constant(at)? {
        "_None" => json!("linear"),
        "Axl1" => json!("ease_in"),
        "Dxl1" => json!("ease_out"),
        "Axl2" | "Dxl2" => {
            json!({"progress":{"shape":{"kind":"power","exponent":3.,"direction":if args.constant(at)?=="Axl2"{"in"}else{"out"}}}})
        }
        "Axl3" => json!("ease_in_quart"),
        "Dxl3" => json!("ease_out_quart"),
        "AxlAuto" => json!({"progress":{"shape":{"kind":"sine","direction":"in"}}}),
        "DxlAuto" => json!({"progress":{"shape":{"kind":"sine","direction":"out"}}}),
        "AxlDxl" => json!({"progress":{"shape":{"kind":"half_cosine","pi":std::f64::consts::PI}}}),
        "DxlAxl" => json!({"progress":{"shape":{"kind":"inverse_half_cosine"}}}),
        name => bail!("N2System easing {name} has no connected shared curve"),
    })
}
impl VmAdapter for N2SystemVm {
    fn instruction_budget(&self) -> u32 {
        16384
    }
    fn engine_id(&self) -> &'static str {
        "n2system"
    }
    fn snapshot_schema(&self) -> u32 {
        1
    }
    fn preparation_pending(&self) -> bool {
        !self.started
    }
    fn step(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        budget: u32,
    ) -> crysirror_core::Result<VmYield> {
        self.step_inner(host, input, budget).map_err(|error| {
            let message = format!("{error:#}");
            self.error = Some(message.clone());
            Error::Io(message)
        })
    }
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Err(Error::Unsupported("N2System retained-resource snapshot"))
    }
    fn restore(&mut self, _: &[u8]) -> crysirror_core::Result<()> {
        Err(Error::Unsupported("N2System retained-resource snapshot"))
    }
    fn diagnostics(&self) -> Vec<u8> {
        serde_json::to_vec(&json!({"adapter":"n2system","mode":"adopt","prepared":self.started,
        "modules":self.machine.modules.keys().collect::<Vec<_>>(),"threads":self.threads.len(),"entities":self.entities.nodes.len(),
        "executed_instructions":self.executed,"unresolved_functions":self.machine.unresolved_functions,"error":self.error})).unwrap_or_default()
    }
}
#[cfg(feature = "runtime")]
pub fn register(registry: &mut crysirror_runtime::AdapterRegistry) -> Result<()> {
    registry.register("n2system", Arc::new(Factory))
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
            "N2System executes native NSS scripts"
        );
        Ok(Box::new(N2SystemVm::new(
            client,
            serde_json::from_value(spec.config.clone())?,
        )?))
    }
}

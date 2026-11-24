// SPDX-License-Identifier: MPL-2.0
//! Player session for the installed BCAS20097 disc.
//!
//! Task mode (the default) boots the native DC scripts from `dc1.psarc`
//! (`startup` and the modules it requests), selects a task subnode of the
//! game task graph, loads that subnode's level with its `cache` levels and
//! the node's world level from `pak22.psarc`, places the player at the
//! subnode's continue spawner and runs the state-script processes of the
//! level's spawners every frame. Spawners with flag 0x40000000 (continue
//! points, companions, cameras: inferred as no-auto-birth) start hidden
//! and appear on `spawn-entity`. The player moves on the walkable level
//! surfaces (WASD, Shift runs, arrows turn the camera); Page Down/Page Up
//! step to the next/previous task subnode in task order.
//!
//! `{"level": NAME}` without a task opens the free-camera level viewer
//! (WASD/QE move, arrows turn, Page Down/Up switch `pak22.txt` levels).
//! Native animation, collision, AI and saves are not executed
//! (SATD-nd-retail-runtime).
use crate::{
    dc::sid,
    game::Game,
    ground::Ground,
    levels,
    package::Directory,
    scene::{self, SpawnerRecord},
    ss::{Property, World},
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    Error,
    path::{Mount, VfsPath},
    service::{AssetReader, HostServices, InputFrame, buttons as b},
    state::ClientId,
    vfs::VfsProvider,
    vm::{VmAdapter, VmYield},
};
use crysirror_resource::{ArchiveMount, DecodeContext, FormatRegistry, read_all};
use serde::Deserialize;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub const ENGINE_ID: &str = "naughty-dog";
const MAIN: &str = "PS3_GAME/USRDIR/build/main";
/// Largest decoded package and base-colour image edge for the viewer.
const PACKAGE_LIMIT: u64 = 256 << 20;
const MAX_TEXTURE: u32 = 512;
const FRAME_MS: u64 = 16;
const SCREEN: [u32; 2] = [1280, 720];
const FADE_LAYER: u32 = 0x4e44_0001;
const NO_AUTO_BIRTH: u32 = 0x4000_0000;
const MOVIE_LAYER: u32 = 0x4e44_0002;
/// The world assigns the first video the first video surface slot.
const MOVIE_SURFACE: &str = "__surface/1000";
/// World videos are read whole (`crysirror_runtime::vfs::MAX_FILE`).
const MAX_WORLD_VIDEO: u64 = crysirror_core::limits::LARGE_ASSET_BYTES;
const MAX_LOG: usize = 64;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// `level-name` of `pak22.txt` for the level viewer.
    #[serde(default)]
    pub level: Option<String>,
    /// Task subnode to start (default: the first story task).
    #[serde(default)]
    pub task: Option<String>,
}

fn valid_name(n: &str) -> bool {
    !n.is_empty() && n.len() <= 128 && n.bytes().all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.level.as_deref().is_none_or(valid_name), "Naughty Dog level name");
        ensure!(self.task.as_deref().is_none_or(valid_name), "Naughty Dog task name");
        Ok(())
    }
    fn viewer(&self) -> bool {
        self.level.is_some() && self.task.is_none()
    }
}

/// Free camera in metres; yaw/pitch in radians.
#[derive(Clone, Copy, Debug)]
struct Camera {
    eye: [f32; 3],
    yaw: f32,
    pitch: f32,
    speed: f32,
}

impl Camera {
    fn forward(&self) -> [f32; 3] {
        [self.yaw.sin() * self.pitch.cos(), self.pitch.sin(), -self.yaw.cos() * self.pitch.cos()]
    }
    fn op(&self) -> serde_json::Value {
        let f = self.forward();
        let target: [f32; 3] = std::array::from_fn(|k| self.eye[k] + f[k]);
        json!({"op": "set_camera", "camera": {"eye": self.eye, "target": target, "up": [0.0, 1.0, 0.0], "fov_y_degrees": 60.0, "near": 0.05, "far": 5000.0}})
    }
    fn step(&mut self, input: &InputFrame, seconds: f32) -> bool {
        let down = |m| input.buttons_down & m != 0;
        let turn = 1.5 * seconds;
        let mut moved = false;
        let mut adjust = |cond: bool, f: &mut dyn FnMut()| {
            if cond {
                f();
                moved = true;
            }
        };
        adjust(down(b::LEFT), &mut || self.yaw -= turn);
        adjust(down(b::RIGHT), &mut || self.yaw += turn);
        adjust(down(b::UP), &mut || self.pitch = (self.pitch + turn).min(1.5));
        adjust(down(b::DOWN), &mut || self.pitch = (self.pitch - turn).max(-1.5));
        let speed = self.speed * seconds * if down(b::SHIFT) { 4. } else { 1. };
        let (f, r) = (self.forward(), [self.yaw.cos(), 0., self.yaw.sin()]);
        let mut delta = [0f32; 3];
        for (key, dir, sign) in [(b::KEY_W, f, 1.), (b::KEY_S, f, -1.), (b::KEY_D, r, 1.), (b::KEY_A, r, -1.), (b::KEY_E, [0., 1., 0.], 1.), (b::KEY_Q, [0., 1., 0.], -1.)] {
            if down(key) {
                moved = true;
                for k in 0..3 {
                    delta[k] += dir[k] * sign * speed;
                }
            }
        }
        for k in 0..3 {
            self.eye[k] += delta[k];
        }
        moved
    }
}

/// The archive path of a declared package: the exact name, else its only
/// numbered sibling (`con-1-start-ingame` → `con-1-start-ingame10.pak`).
/// The native variant rule is unknown (SATD-nd-retail-runtime).
fn package_path(mount: &ArchiveMount, name: &str) -> Option<String> {
    let exact = format!("{name}.pak");
    if mount.exists(&exact) {
        return Some(exact);
    }
    let mut numbered = (0..100).map(|n| format!("{name}{n}.pak")).filter(|p| mount.exists(p));
    let first = numbered.next()?;
    numbered.next().is_none().then_some(first)
}

/// One queued load step.
enum Load {
    /// A level package of `pak22.psarc`.
    Package(String),
    /// An art group of `actor27.psarc`, placed at each spawner.
    Actor(String, Vec<SpawnerRecord>),
    /// Part of a package's meshes whose model exceeded the host's
    /// per-model limit: `(package, archive path, meshes)`.
    Part(String, String, Vec<crate::mesh::Mesh>),
}

/// Largest generated model: the host's large-asset limit less a margin.
const MODEL_LIMIT: usize = (crysirror_core::limits::LARGE_ASSET_BYTES as usize) / 16 * 15;

/// The player character placed at the continue spawner.
struct Player {
    entity: Option<u32>,
    position: [f32; 3],
    yaw: f32,
    fall: f32,
    /// Camera orbit yaw and pitch around the player.
    view_yaw: f32,
    view_pitch: f32,
    controls: bool,
}

/// Engine side of the script natives: spawner entities, prints and fades.
#[derive(Default)]
struct Stage {
    now: f64,
    /// Spawner records of the loaded packages by name `StringId`.
    spawners: HashMap<u32, SpawnerRecord>,
    /// Entities of each spawner.
    entities: HashMap<u32, Vec<u32>>,
    visible: HashSet<u32>,
    levels: HashSet<u32>,
    ops: Vec<serde_json::Value>,
    log: Vec<String>,
    fade: f32,
    fade_color: [u8; 3],
    fade_target: f32,
    fade_rate: f32,
    controls: Option<bool>,
    camera: Option<u32>,
    /// Left stick from the movement keys.
    stick: [f32; 2],
}

impl Stage {
    fn set_visible(&mut self, name: u32, visible: bool) -> bool {
        let Some(ids) = self.entities.get(&name) else { return false };
        for id in ids {
            self.ops.push(json!({"op": "set_visible", "id": id, "visible": visible}));
        }
        if visible { self.visible.insert(name) } else { self.visible.remove(&name) }
    }
    fn log(&mut self, line: String) {
        if self.log.len() >= MAX_LOG {
            self.log.remove(0);
        }
        self.log.push(line);
    }
}

impl World for Stage {
    fn now(&self) -> f64 {
        self.now
    }
    fn print(&mut self, text: &str) {
        self.log(format!("print: {text}"));
    }
    fn level_spawned(&self, level: u32) -> bool {
        self.levels.contains(&level)
    }
    fn object_spawned(&self, name: u32) -> bool {
        self.visible.contains(&name)
    }
    fn spawn(&mut self, name: u32) -> bool {
        self.set_visible(name, true);
        self.spawners.contains_key(&name)
    }
    fn kill(&mut self, name: u32) -> bool {
        self.set_visible(name, false)
    }
    fn locator(&self, name: u32) -> Option<([f32; 3], [f32; 4])> {
        self.spawners.get(&name).map(|s| (s.position, s.rotation))
    }
    fn property(&self, spawner: u32, prop: u32) -> Option<Property> {
        self.spawners.get(&spawner)?.properties.iter().find(|(k, _)| *k == prop).map(|(_, v)| v.clone())
    }
    fn fade(&mut self, color: [f32; 3], alpha: f32, seconds: f32) {
        self.fade_color = color.map(|c| (c.clamp(0., 1.) * 255.) as u8);
        self.fade_target = alpha.clamp(0., 1.);
        if seconds <= 0. {
            self.fade = self.fade_target;
            self.fade_rate = 0.;
        } else {
            self.fade_rate = 1. / seconds;
        }
    }
    fn camera(&mut self, name: u32, _seconds: f32) {
        self.camera = Some(name);
    }
    fn player_controls(&mut self, enabled: bool) {
        self.controls = Some(enabled);
    }
    fn sticks(&self, pad: u32) -> [f32; 4] {
        if pad == 0 { [self.stick[0], self.stick[1], 0., 0.] } else { [0.; 4] }
    }
}

/// A selected task subnode with its levels.
struct TaskPlan {
    name: String,
    level: String,
    continue_player: Option<String>,
    /// Level names whose packages load (subnode level, caches, world,
    /// then the displayed levels of the world's load regions).
    levels: Vec<String>,
    /// Load regions evaluated: `(region, scheme, displayed, not displayed)`.
    regions: Vec<(String, u32, Vec<String>, Vec<String>)>,
}

/// The mounted disc data and the presented level.
struct Session {
    mount: ArchiveMount,
    /// Level names of `pak22.txt` in native order, with their packages.
    levels: Vec<(String, Vec<String>)>,
    all_levels: std::collections::BTreeMap<String, crate::Level>,
    current: usize,
    camera: Camera,
    last_ms: u64,
    start_ms: u64,
    triangles: usize,
    packages: Vec<String>,
    /// Spawned entities and their uploaded models.
    entities: Vec<(u32, VfsPath)>,
    next_model: u32,
    /// Packages of the current level still to load, one per tick (the host
    /// bounds generated model bytes per tick).
    pending: std::collections::VecDeque<Load>,
    /// `actor27.psarc`, for spawned art groups.
    actors: ArchiveMount,
    bounds: Option<([f32; 3], [f32; 3])>,
    start: Option<scene::StartCamera>,
    /// Whether a level light table has set the environment.
    lit: bool,
    // Task mode.
    modules: Option<ArchiveMount>,
    game: Option<Game>,
    /// Task subnode names in task order.
    tasks: Vec<String>,
    plan: Option<TaskPlan>,
    stage: Stage,
    records: Vec<SpawnerRecord>,
    ground: Ground,
    player: Option<Player>,
    started: bool,
    script_error: Option<String>,
    /// End time (stage seconds) of the playing task movie, and whether a
    /// world video and its layer present it.
    movie: Option<f64>,
    movie_video: bool,
}

/// The world environment: the level's directional light and ambient term
/// when known. Native intensities (0.9–19 on the installed levels) are
/// clamped to the renderer's sun range; the mapping is not calibrated
/// against the original renderer (SATD-nd-retail-runtime).
fn environment(light: Option<&scene::LevelLight>) -> serde_json::Value {
    match light {
        None => json!({"op": "set_environment", "environment": {"show_sky": true, "intensity": 1.0, "ambient": [0.55, 0.55, 0.55], "sun": {"direction": [0.3, -0.8, -0.4], "color": [1.0, 0.97, 0.9], "intensity": 0.6, "shadow_strength": 0.0}}}),
        Some(l) => {
            let ambient = l.ambient.map_or([0.45; 3], |a| a.map(|v| (v * 1.6).clamp(0.05, 1.5)));
            json!({"op": "set_environment", "environment": {"show_sky": true, "intensity": 1.0, "ambient": ambient, "sun": {"direction": l.direction, "color": l.color.map(|c| c.clamp(0., 4.)), "intensity": l.intensity.clamp(0.2, 4.0), "shadow_strength": 0.6}}})
        }
    }
}

fn yaw_of(q: [f32; 4]) -> f32 {
    let f = glam::Quat::from_array(q).normalize() * glam::Vec3::Z;
    f.x.atan2(f.z)
}

/// The duration of an MPEG-2 video elementary stream: picture start codes
/// over the sequence header's frame rate.
fn movie_seconds(reader: &dyn AssetReader) -> Result<f64> {
    let len = reader.len()?;
    let mut head = [0u8; 8];
    crysirror_resource::read_exact_at(reader, 0, &mut head)?;
    ensure!(head[..4] == [0, 0, 1, 0xb3], "not an MPEG video sequence");
    let fps = match head[7] & 15 {
        1 => 24000. / 1001.,
        2 => 24.,
        3 => 25.,
        4 => 30000. / 1001.,
        5 => 30.,
        6 => 50.,
        7 => 60000. / 1001.,
        8 => 60.,
        c => anyhow::bail!("MPEG frame rate code {c}"),
    };
    let (mut pictures, mut at, mut buf) = (0u64, 0u64, vec![0u8; 4 << 20]);
    while at < len {
        let n = ((len - at) as usize).min(buf.len());
        crysirror_resource::read_exact_at(reader, at, &mut buf[..n])?;
        pictures += buf[..n].windows(4).filter(|w| *w == [0, 0, 1, 0]).count() as u64;
        // Overlap three bytes so codes across chunks are seen once.
        at += if at + (n as u64) < len { n as u64 - 3 } else { n as u64 };
    }
    Ok(pictures as f64 / fps)
}

pub struct NaughtyDogVm {
    client: ClientId,
    config: Config,
    state: Option<Session>,
}

impl NaughtyDogVm {
    pub fn new(client: ClientId, config: Config) -> Result<Self> {
        config.validate()?;
        Ok(Self { client, config, state: None })
    }

    fn boot(&mut self, host: &mut dyn HostServices) -> Result<Session> {
        let storage = host.storage(self.client)?;
        let open = |p: &str| -> Result<Arc<dyn AssetReader>> { Ok(storage.open_asset(&VfsPath::new(Mount::Asset, &format!("{MAIN}/{p}"))?)?) };
        let manifest = read_all(open("pak22.txt")?.as_ref(), 4 << 20)?;
        let text = std::str::from_utf8(&manifest)?;
        let all = levels(text)?;
        // Native order: the order of `level-name` directives.
        // A level presents its own packages followed by those of the levels
        // it names with `cache` (for example the menu's train-wreck views).
        let level_list: Vec<(String, Vec<String>)> = text
            .lines()
            .filter_map(|l| l.strip_prefix("level-name "))
            .filter_map(|n| {
                let level = all.get(n)?;
                let mut packages = level.packages.clone();
                for cache in &level.caches {
                    for p in all.get(cache).map(|l| l.packages.as_slice()).unwrap_or_default() {
                        if !packages.contains(p) {
                            packages.push(p.clone());
                        }
                    }
                }
                Some((n.to_owned(), packages))
            })
            .collect();
        let mut registry = FormatRegistry::default();
        crysirror_formats_arc::register(&mut registry)?;
        let mut context = DecodeContext::new(self.client);
        context.limits.max_entry_size = PACKAGE_LIMIT;
        let (_, archive) = registry.open(open("pak22.psarc")?, Some("sony/psarc"), &context)?;
        let mount = ArchiveMount::new(archive, context.clone())?;
        let (_, actor_archive) = registry.open(open("actor27.psarc")?, Some("sony/psarc"), &context)?;
        let actors = ArchiveMount::new(actor_archive, context.clone())?;
        let camera = Camera { eye: [0.; 3], yaw: 0., pitch: 0., speed: 5. };
        let now = host.now_ms();
        let mut session = Session {
            mount,
            levels: level_list,
            all_levels: all,
            current: 0,
            camera,
            last_ms: now,
            start_ms: now,
            triangles: 0,
            packages: Vec::new(),
            entities: Vec::new(),
            next_model: 0,
            pending: Default::default(),
            actors,
            bounds: None,
            start: None,
            lit: false,
            modules: None,
            game: None,
            tasks: Vec::new(),
            plan: None,
            stage: Stage::default(),
            records: Vec::new(),
            ground: Ground::default(),
            player: None,
            started: false,
            script_error: None,
            movie: None,
            movie_video: false,
        };
        if self.config.viewer() {
            let level = self.config.level.as_deref().unwrap_or("menu-temple");
            session.current = session.levels.iter().position(|(n, _)| n == level).with_context(|| format!("unknown Naughty Dog level {level}"))?;
        } else {
            let (_, dc) = registry.open(open("dc1.psarc")?, Some("sony/psarc"), &context)?;
            let modules = ArchiveMount::new(dc, context)?;
            let game = Game::boot(&modules, &mut session.stage)?;
            let g = &game.rt.inner.tasks;
            session.tasks = g.order.iter().filter_map(|id| g.node(*id)).flat_map(|n| n.order.iter().filter_map(|s| n.subnodes.iter().find(|x| x.id == *s)).map(|s| s.name.clone())).collect();
            let task = self.config.task.clone().or_else(|| game.default_task()).context("the task graph has no story task")?;
            session.current = session.tasks.iter().position(|t| *t == task).with_context(|| format!("unknown Naughty Dog task {task}"))?;
            session.modules = Some(modules);
            session.game = Some(game);
        }
        self.begin(host, &mut session)?;
        Ok(session)
    }

    /// Select the task subnode `session.current` and its levels.
    fn plan(session: &mut Session) -> Result<TaskPlan> {
        let game = session.game.as_mut().context("task mode")?;
        let name = session.tasks[session.current].clone();
        let (node, sub) = game.select(&name)?;
        let n = &game.rt.inner.tasks.nodes[node];
        let s = &n.subnodes[sub];
        // A subnode without a level (most movies) loads nothing.
        if s.level.is_empty() {
            return Ok(TaskPlan { name, level: String::new(), continue_player: None, levels: Vec::new(), regions: Vec::new() });
        }
        let mut levels = vec![s.level.clone()];
        for c in session.all_levels.get(&s.level).map(|l| l.caches.as_slice()).unwrap_or_default() {
            if !levels.contains(c) {
                levels.push(c.clone());
            }
        }
        if !n.world.is_empty() && !levels.contains(&n.world) {
            levels.push(n.world.clone());
        }
        Ok(TaskPlan { name, level: s.level.clone(), continue_player: s.continue_player.clone(), levels, regions: Vec::new() })
    }

    /// Unload the presented level and queue the packages of the current
    /// level (viewer) or task subnode.
    fn begin(&self, host: &mut dyn HostServices, session: &mut Session) -> Result<()> {
        let mut world = vec![environment(None)];
        for (id, asset) in session.entities.drain(..) {
            world.push(json!({"op": "despawn", "id": id}));
            host.release_model(self.client, &asset)?;
        }
        session.triangles = 0;
        session.packages.clear();
        session.bounds = None;
        session.start = None;
        session.lit = false;
        session.records.clear();
        session.ground = Ground::default();
        session.player = None;
        session.started = false;
        session.script_error = None;
        session.movie = None;
        session.movie_video = false;
        session.stage.spawners.clear();
        session.stage.entities.clear();
        session.stage.visible.clear();
        session.stage.ops.clear();
        session.stage.fade = 0.;
        session.stage.fade_target = 0.;
        if session.game.is_some() {
            // A task restarts the script runtime from the booted modules.
            if let (Some(modules), Some(game)) = (&session.modules, &mut session.game) {
                if !game.rt.inner.procs.is_empty() {
                    *game = Game::boot(modules, &mut session.stage)?;
                }
            }
            let plan = Self::plan(session)?;
            let mut packages = Vec::new();
            for l in &plan.levels {
                for p in session.all_levels.get(l).map(|l| l.packages.as_slice()).unwrap_or_default() {
                    if !packages.contains(p) {
                        packages.push(p.clone());
                    }
                }
            }
            session.stage.levels = plan.levels.iter().map(|l| sid(l)).collect();
            session.pending = packages.into_iter().map(Load::Package).collect();
            session.plan = Some(plan);
        } else {
            session.pending = session.levels[session.current].1.iter().cloned().map(Load::Package).collect();
        }
        host.submit_engine(self.client, 1, &serde_json::to_vec(&json!({"layers": [{"op": "put", "layer": {"id": FADE_LAYER, "rect": [0, 0, SCREEN[0], SCREEN[1]], "paint": {"kind": "solid", "color": [0, 0, 0, 0]}}}], "world": world}))?)?;
        Ok(())
    }

    /// Load and present the next queued package; frame the camera once the
    /// level's start camera is known or the last package is in.
    fn load_next(&self, host: &mut dyn HostServices, session: &mut Session) -> Result<()> {
        let Some(step) = session.pending.pop_front() else { return Ok(()) };
        let mut world = Vec::new();
        let package_name = match step {
            Load::Package(name) => name,
            Load::Actor(art, spawners) => {
                self.load_actor(host, session, &art, &spawners, &mut world)?;
                String::new()
            }
            Load::Part(name, path, meshes) => {
                let package = session.mount.open(&path)?;
                let directory = Directory::read(package.as_ref())?;
                self.upload_meshes(host, session, &name, &path, meshes, package.as_ref(), &directory, &mut world)?;
                String::new()
            }
        };
        if let Some(path) = (!package_name.is_empty()).then(|| package_path(&session.mount, &package_name)).flatten() {
            // Read and relocate the package once; every item below parses
            // the same copy.
            let package: Arc<dyn AssetReader> = Arc::new(crysirror_core::service::MemoryAssetReader(read_all(session.mount.open(&path)?.as_ref(), PACKAGE_LIMIT)?.into()));
            let directory = Directory::read(package.as_ref())?;
            let loaded = crate::mesh::Loaded::read(package.as_ref(), PACKAGE_LIMIT)?;
            let meshes = scene::meshes_of(&loaded, &directory)?;
            if !session.lit {
                if let Some(item) = directory.items.iter().find(|i| i.kind == "LIGHT_TABLE_1") {
                    if let Some(light) = scene::level_light(&loaded, item.payload_offset as usize)? {
                        world.push(environment(Some(&light)));
                        session.lit = true;
                    }
                }
            }
            // Spawned art groups follow the level packages.
            let mut groups: std::collections::BTreeMap<String, Vec<SpawnerRecord>> = Default::default();
            for item in directory.items.iter().filter(|i| i.kind == "SPAWNER_GROUP") {
                for s in scene::spawner_records(&loaded, item.payload_offset as usize)? {
                    session.stage.spawners.insert(sid(&s.name), s.clone());
                    if let Some(art) = s.art_group.clone() {
                        groups.entry(art).or_default().push(s.clone());
                    }
                    session.records.push(s);
                }
            }
            for (art, list) in groups {
                session.pending.push_back(Load::Actor(art, list));
            }
            if session.start.is_none() {
                if let Some(item) = directory.items.iter().find(|i| i.kind == "CAMERA_TABLE_1") {
                    if let Some(c) = scene::start_camera(&loaded, item.payload_offset as usize)? {
                        // The level's start camera (`CAMERA_TABLE_1`, metres).
                        session.camera = Camera { eye: c.position, yaw: c.forward[0].atan2(-c.forward[2]), pitch: c.forward[1].clamp(-1., 1.).asin(), speed: 5. };
                        if session.game.is_none() {
                            world.push(session.camera.op());
                        }
                        session.start = Some(c);
                    }
                }
            }
            if session.game.is_some() {
                session.ground.add(&meshes);
                if let Some(item) = directory.items.iter().find(|i| i.kind == "REGION_TREE") {
                    let regions = crate::region::load_regions(&loaded, item.payload_offset as usize)?;
                    if !regions.is_empty() {
                        self.apply_load_regions(session, &package_name, &loaded, &regions)?;
                    }
                }
            }
            if let Some((l, h)) = scene::placed_bounds(&meshes) {
                session.bounds = Some(match session.bounds {
                    None => (l, h),
                    Some((a, b)) => (std::array::from_fn(|k| a[k].min(l[k])), std::array::from_fn(|k| b[k].max(h[k]))),
                });
                session.triangles += meshes.iter().flat_map(|m| &m.fragments).map(|f| f.triangles.len()).sum::<usize>();
                session.packages.push(package_name.clone());
                self.upload_meshes(host, session, &package_name, &path, meshes, package.as_ref(), &directory, &mut world)?;
            }
        }
        if session.pending.is_empty() {
            let (lo, hi) = session.bounds.unwrap_or(([-5.; 3], [5.; 3]));
            let (center, size): ([f32; 3], f32) = (std::array::from_fn(|k| (lo[k] + hi[k]) * 0.5), (0..3).map(|k| hi[k] - lo[k]).fold(0., f32::max));
            session.camera.speed = (size * 0.05).clamp(2., 50.);
            if session.game.is_some() {
                self.start_task(session, &mut world)?;
                if let Some((video, layer)) = self.start_movie(host, session)? {
                    world.push(video);
                    host.submit_engine(self.client, 1, &serde_json::to_vec(&json!({"layers": [layer], "world": []}))?)?;
                }
            } else if session.start.is_none() {
                // South of the placed bounds, slightly above, facing them.
                session.camera = Camera { eye: [center[0], center[1] + size * 0.15, center[2] + size * 0.6], yaw: 0., pitch: -0.15, speed: session.camera.speed };
                world.push(session.camera.op());
            }
        }
        if !world.is_empty() {
            host.submit_engine(self.client, 1, &serde_json::to_vec(&json!({"layers": [], "world": world}))?)?;
        }
        Ok(())
    }

    /// Upload a package's meshes as one model and spawn it; a model over
    /// the host limit is split into two halves queued as separate steps.
    #[allow(clippy::too_many_arguments)]
    fn upload_meshes(&self, host: &mut dyn HostServices, session: &mut Session, name: &str, path: &str, mut meshes: Vec<crate::mesh::Mesh>, package: &dyn AssetReader, directory: &Directory, world: &mut Vec<serde_json::Value>) -> Result<()> {
        // `to_glb` refuses a model over the import limit; split on either.
        let glb = scene::document(name, &meshes, package, directory, MAX_TEXTURE)?.to_glb().ok().filter(|g| g.len() <= MODEL_LIMIT);
        let Some(glb) = glb else {
            ensure!(meshes.len() > 1, "{name}: one mesh exceeds the model limit");
            let second = meshes.split_off(meshes.len() / 2);
            session.pending.push_front(Load::Part(name.to_owned(), path.to_owned(), second));
            session.pending.push_front(Load::Part(name.to_owned(), path.to_owned(), meshes));
            return Ok(());
        };
        session.next_model += 1;
        let asset = host.upload_model(self.client, 0x4e44_0000 + session.next_model, &glb)?;
        let id = 0x100 + session.next_model;
        world.push(json!({"op": "spawn", "entity": {"id": id, "name": name, "geometry": {"kind": "scene", "asset": asset.relative()}}}));
        session.entities.push((id, asset));
        Ok(())
    }

    /// Place the player and start the level's state-script processes.
    fn start_task(&self, session: &mut Session, world: &mut Vec<serde_json::Value>) -> Result<()> {
        session.ground.prepare();
        let plan = session.plan.as_ref().context("task plan")?;
        let spawner = plan.continue_player.as_deref().and_then(|n| session.stage.spawners.get(&sid(n))).cloned();
        let (position, yaw) = match &spawner {
            Some(s) => (s.position, yaw_of(s.rotation)),
            None => (session.start.as_ref().map_or([0.; 3], |c| c.position), session.camera.yaw),
        };
        let entity = spawner.as_ref().and_then(|s| session.stage.entities.get(&sid(&s.name))).and_then(|v| v.first().copied());
        if let Some(s) = &spawner {
            session.stage.set_visible(sid(&s.name), true);
        }
        let mut player = Player { entity, position, yaw, fall: 0., view_yaw: yaw + std::f32::consts::PI, view_pitch: -0.25, controls: true };
        if let Some(h) = session.ground.height(position[0], position[2], position[1] + 1.) {
            player.position[1] = h;
        }
        session.player = Some(player);
        session.stage.log(format!("task {} level {}: player at {:?}", plan.name, plan.level, position));
        let (Some(game), Some(modules)) = (session.game.as_mut(), session.modules.as_ref()) else { return Ok(()) };
        match game.start(modules, &mut session.stage, &session.records) {
            Ok(n) => session.stage.log(format!("{n} state-script processes")),
            Err(e) => session.script_error = Some(format!("{e:#}")),
        }
        world.append(&mut session.stage.ops);
        world.push(Self::follow(session.player.as_ref().expect("player"), &session.ground));
        session.started = true;
        Ok(())
    }

    /// A movie task subnode plays `movie1/NAME.m2v` (the subnode name
    /// without `movie-`) full screen; it completes when the stream ends or
    /// Enter/Escape skips it.
    fn start_movie(&self, host: &mut dyn HostServices, session: &mut Session) -> Result<Option<(serde_json::Value, serde_json::Value)>> {
        let Some(name) = session.plan.as_ref().and_then(|p| p.name.strip_prefix("movie-")) else { return Ok(None) };
        let path = format!("{MAIN}/movie1/{name}.m2v");
        let storage = host.storage(self.client)?;
        let Ok(reader) = storage.open_asset(&VfsPath::new(Mount::Asset, &path)?) else { return Ok(None) };
        // World videos are read whole, within the runtime's per-file limit;
        // a longer movie is skipped and its task completes.
        if reader.len()? > MAX_WORLD_VIDEO {
            session.movie = Some(session.stage.now);
            session.stage.log(format!("movie {name}.m2v exceeds the {} MiB world-video read limit; skipped", MAX_WORLD_VIDEO >> 20));
            return Ok(None);
        }
        let seconds = movie_seconds(reader.as_ref())?;
        session.movie = Some(session.stage.now + seconds);
        session.movie_video = true;
        session.stage.log(format!("movie {name}.m2v {seconds:.1} s"));
        let video = json!({"op": "set_video", "name": "movie", "video": {"asset": path, "playing": true}});
        let layer = json!({"op": "put", "layer": {"id": MOVIE_LAYER, "rect": [0, 0, SCREEN[0], SCREEN[1]], "paint": {"kind": "image", "asset": MOVIE_SURFACE}}});
        Ok(Some((video, layer)))
    }

    /// Apply a task change: a subnode with the same levels keeps the
    /// running scripts; otherwise its levels load.
    fn change_task(&self, host: &mut dyn HostServices, session: &mut Session, at: (usize, usize)) -> Result<()> {
        let Some(game) = session.game.as_mut() else { return Ok(()) };
        let name = game.rt.inner.tasks.nodes[at.0].subnodes[at.1].name.clone();
        session.stage.log(format!("task change to {name}"));
        let Some(index) = session.tasks.iter().position(|t| *t == name) else { return Ok(()) };
        session.current = index;
        let plan = Self::plan(session)?;
        if session.plan.as_ref().is_some_and(|p| p.levels == plan.levels) {
            session.plan = Some(plan);
            if let Some((video, layer)) = self.start_movie(host, session)? {
                host.submit_engine(self.client, 1, &serde_json::to_vec(&json!({"layers": [layer], "world": [video]}))?)?;
            }
            return Ok(());
        }
        self.begin(host, session)
    }

    /// Evaluate load regions for the current task: each region's scheme
    /// lambda runs in the script VM (the package is mapped into its address
    /// space) and the scheme's displayed levels are queued.
    fn apply_load_regions(&self, session: &mut Session, package: &str, loaded: &crate::mesh::Loaded, regions: &[crate::region::LoadRegion]) -> Result<()> {
        let Some(game) = session.game.as_mut() else { return Ok(()) };
        let base = game.rt.dc.map_package(sid(package), &loaded.data, loaded.pointers())?;
        for region in regions {
            let scheme = match region.scheme {
                Some(off) => game.rt.run_lambda(&mut session.stage, base + u32::try_from(off)?, &[])?,
                None => 1,
            };
            let (shown, hidden) = crate::region::levels(region, scheme);
            let plan = session.plan.as_mut().context("task plan")?;
            for level in shown.iter().chain(&hidden) {
                session.stage.levels.insert(sid(level));
            }
            for level in &shown {
                if plan.levels.contains(level) {
                    continue;
                }
                plan.levels.push(level.clone());
                for p in session.all_levels.get(level).map(|l| l.packages.as_slice()).unwrap_or_default() {
                    let queued = session.pending.iter().any(|l| matches!(l, Load::Package(x) if x == p));
                    if !queued && !session.packages.contains(p) {
                        session.pending.push_back(Load::Package(p.clone()));
                    }
                }
            }
            session.stage.log(format!("load region {} scheme {scheme}: {} displayed, {} not displayed", region.name, shown.len(), hidden.len()));
            plan.regions.push((region.name.clone(), scheme, shown, hidden));
        }
        Ok(())
    }

    /// Third-person camera behind and above the player, pulled in front of
    /// the first surface between the player's head and the camera.
    fn follow(p: &Player, ground: &Ground) -> serde_json::Value {
        let target = [p.position[0], p.position[1] + 1.5, p.position[2]];
        let cy = p.view_pitch.cos();
        let dir = [-p.view_yaw.sin() * cy, -p.view_pitch.sin(), p.view_yaw.cos() * cy];
        let far = 4.;
        let want: [f32; 3] = std::array::from_fn(|k| target[k] + dir[k] * far);
        let d = ground.raycast(target, want).map_or(far, |t| (t * far - 0.25).max(0.3));
        let eye: [f32; 3] = std::array::from_fn(|k| target[k] + dir[k] * d);
        json!({"op": "set_camera", "camera": {"eye": eye, "target": target, "up": [0.0, 1.0, 0.0], "fov_y_degrees": 60.0, "near": 0.05, "far": 5000.0}})
    }

    /// Player movement on the walkable surfaces, the script tick and the
    /// presentation of their effects.
    fn play(&self, host: &mut dyn HostServices, session: &mut Session, input: &InputFrame, seconds: f32) -> Result<()> {
        let mut world = Vec::new();
        let mut layers = Vec::new();
        session.stage.now = (host.now_ms().saturating_sub(session.start_ms)) as f64 / 1000.;
        let key = |m: u64| f32::from(u8::from(input.buttons_down & m != 0));
        session.stage.stick = [key(b::KEY_D) - key(b::KEY_A), key(b::KEY_W) - key(b::KEY_S)];
        if let Some(c) = session.stage.controls.take() {
            if let Some(p) = &mut session.player {
                p.controls = c;
            }
        }
        if let Some(p) = &mut session.player {
            let down = |m| input.buttons_down & m != 0;
            let turn = 1.8 * seconds;
            if down(b::LEFT) {
                p.view_yaw -= turn;
            }
            if down(b::RIGHT) {
                p.view_yaw += turn;
            }
            if down(b::UP) {
                p.view_pitch = (p.view_pitch - turn).max(-1.2);
            }
            if down(b::DOWN) {
                p.view_pitch = (p.view_pitch + turn).min(0.6);
            }
            let (fx, fz) = (p.view_yaw.sin(), -p.view_yaw.cos());
            let mut mv = [0f32; 2];
            if p.controls {
                for (key, dx, dz) in [(b::KEY_W, fx, fz), (b::KEY_S, -fx, -fz), (b::KEY_D, -fz, fx), (b::KEY_A, fz, -fx)] {
                    if down(key) {
                        mv[0] += dx;
                        mv[1] += dz;
                    }
                }
            }
            let len = (mv[0] * mv[0] + mv[1] * mv[1]).sqrt();
            if len > 0. {
                let speed = if down(b::SHIFT) { 6. } else { 2.5 } * seconds / len;
                let next = [p.position[0] + mv[0] * speed, p.position[2] + mv[1] * speed];
                // Step up at most 0.6 m; a missing surface ahead blocks.
                if let Some(h) = session.ground.height(next[0], next[1], p.position[1] + 0.6) {
                    if h > p.position[1] - 3. || session.ground.is_empty() {
                        p.position[0] = next[0];
                        p.position[2] = next[1];
                    }
                }
                p.yaw = mv[0].atan2(mv[1]);
            }
            match session.ground.height(p.position[0], p.position[2], p.position[1] + 0.6) {
                Some(h) if h >= p.position[1] - p.fall * seconds - 0.05 => {
                    p.position[1] = h;
                    p.fall = 0.;
                }
                Some(_) => {
                    p.fall = (p.fall + 9.8 * seconds).min(30.);
                    p.position[1] -= p.fall * seconds;
                }
                None => {}
            }
            if let Some(id) = p.entity {
                world.push(json!({"op": "set_transform", "id": id, "transform": {"translation": p.position, "rotation_degrees": [0.0, p.yaw.to_degrees(), 0.0], "scale": [1.0, 1.0, 1.0]}}));
            }
            world.push(Self::follow(p, &session.ground));
        }
        if let (Some(game), Some(modules)) = (session.game.as_mut(), session.modules.as_ref()) {
            if session.script_error.is_none() {
                let ticked = game.rt.tick(&mut session.stage).and_then(|_| game.load_requested(modules, &mut session.stage));
                if let Err(e) = ticked {
                    session.script_error = Some(format!("{e:#}"));
                }
            }
        }
        world.append(&mut session.stage.ops);
        let skip = input.buttons_pressed & (b::ENTER | b::ESCAPE) != 0;
        let mut change = session.game.as_mut().and_then(|g| g.rt.inner.task_change.take());
        if let Some(end) = session.movie {
            if skip || session.stage.now >= end {
                session.movie = None;
                if std::mem::take(&mut session.movie_video) {
                    world.push(json!({"op": "set_video", "name": "movie", "video": null}));
                    layers.push(json!({"op": "remove", "id": MOVIE_LAYER}));
                }
                if let Some(game) = session.game.as_mut() {
                    if let Some(at) = game.rt.inner.task {
                        let record = game.rt.dc.mem.u32(game.rt.inner.tasks.nodes[at.0].subnodes[at.1].address + 0x1c)?;
                        if record != 0 {
                            game.rt.dc.mem.write_u32(record, 3)?;
                        }
                        change = change.or(game.rt.inner.tasks.next(at));
                    }
                }
            }
        }
        let st = &mut session.stage;
        if st.fade != st.fade_target {
            let step = if st.fade_rate > 0. { st.fade_rate * seconds } else { 1. };
            st.fade = if st.fade < st.fade_target { (st.fade + step).min(st.fade_target) } else { (st.fade - step).max(st.fade_target) };
            let [r, g, b] = st.fade_color;
            layers.push(json!({"op": "set_solid_color", "id": FADE_LAYER, "color": [r, g, b, (st.fade * 255.).round() as u8]}));
        }
        host.submit_engine(self.client, 1, &serde_json::to_vec(&json!({"layers": layers, "world": world}))?)?;
        if let Some(at) = change {
            self.change_task(host, session, at)?;
        }
        Ok(())
    }
}

impl NaughtyDogVm {
    /// Upload an art group's meshes once and spawn an entity per spawner.
    /// An art group without an installed package or render meshes is
    /// skipped. In task mode no-auto-birth spawners start hidden.
    fn load_actor(&self, host: &mut dyn HostServices, session: &mut Session, art: &str, spawners: &[SpawnerRecord], world: &mut Vec<serde_json::Value>) -> Result<()> {
        let path = format!("{art}.pak");
        if !session.actors.exists(&path) {
            return Ok(());
        }
        let package: Arc<dyn AssetReader> = Arc::new(crysirror_core::service::MemoryAssetReader(read_all(session.actors.open(&path)?.as_ref(), PACKAGE_LIMIT)?.into()));
        let directory = Directory::read(package.as_ref())?;
        let mut meshes = scene::package_meshes(package.as_ref(), &directory, PACKAGE_LIMIT)?;
        if meshes.iter().all(|m| m.fragments.iter().all(|f| f.triangles.is_empty())) {
            return Ok(());
        }
        scene::place_art_group(&mut meshes);
        let glb = scene::document(art, &meshes, package.as_ref(), &directory, MAX_TEXTURE)?.to_glb()?;
        session.next_model += 1;
        let asset = host.upload_model(self.client, 0x4e44_0000 + session.next_model, &glb)?;
        let triangles: usize = meshes.iter().flat_map(|m| &m.fragments).map(|f| f.triangles.len()).sum();
        let task = session.game.is_some();
        for s in spawners {
            session.next_model += 1;
            let id = 0x100 + session.next_model;
            let q = glam::Quat::from_array(s.rotation).normalize();
            let (x, y, z) = q.to_euler(glam::EulerRot::XYZ);
            let visible = !task || s.flags & NO_AUTO_BIRTH == 0;
            if task && visible {
                let m = glam::Mat4::from_rotation_translation(q, glam::Vec3::from_array(s.position));
                session.ground.add_placed(&meshes, Some(m.to_cols_array()));
            }
            world.push(json!({"op": "spawn", "entity": {"id": id, "name": format!("{art}:{}", s.name), "visible": visible, "transform": {"translation": s.position, "rotation_degrees": [x.to_degrees(), y.to_degrees(), z.to_degrees()], "scale": [1.0, 1.0, 1.0]}, "geometry": {"kind": "scene", "asset": asset.relative()}}}));
            session.entities.push((id, asset.clone()));
            let name = sid(&s.name);
            session.stage.entities.entry(name).or_default().push(id);
            if visible {
                session.stage.visible.insert(name);
            }
            session.triangles += triangles;
        }
        session.packages.push(format!("{art}×{}", spawners.len()));
        Ok(())
    }
}

impl VmAdapter for NaughtyDogVm {
    fn engine_id(&self) -> &'static str {
        ENGINE_ID
    }
    fn snapshot_schema(&self) -> u32 {
        1
    }
    fn step(&mut self, host: &mut dyn HostServices, input: InputFrame, _budget: u32) -> crysirror_core::Result<VmYield> {
        let fail = |e: anyhow::Error| Error::Io(format!("{e:#}"));
        if self.state.is_none() {
            let session = self.boot(host).map_err(fail)?;
            self.state = Some(session);
            return Ok(VmYield::WaitUntil(host.now_ms() + FRAME_MS));
        }
        let mut session = self.state.take().expect("booted");
        // Page Down / Page Up: next / previous level or task subnode.
        let step = i32::from(input.buttons_pressed & b::PAGE_DOWN != 0) - i32::from(input.buttons_pressed & b::PAGE_UP != 0);
        if step != 0 {
            let n = if session.game.is_some() { session.tasks.len() } else { session.levels.len() } as i32;
            session.current = (session.current as i32 + step).rem_euclid(n) as usize;
            let begun = self.begin(host, &mut session);
            self.state = Some(session);
            begun.map_err(fail)?;
            return Ok(VmYield::WaitUntil(host.now_ms() + FRAME_MS));
        }
        if !session.pending.is_empty() {
            let loaded = self.load_next(host, &mut session);
            self.state = Some(session);
            loaded.map_err(fail)?;
            return Ok(VmYield::WaitUntil(host.now_ms() + FRAME_MS));
        }
        let now = host.now_ms();
        let seconds = (now.saturating_sub(session.last_ms)).min(100) as f32 / 1000.;
        session.last_ms = now;
        if session.started {
            let played = self.play(host, &mut session, &input, seconds);
            self.state = Some(session);
            played.map_err(fail)?;
            return Ok(VmYield::WaitUntil(now + FRAME_MS));
        }
        let moved = session.camera.step(&input, seconds);
        let op = session.camera.op();
        self.state = Some(session);
        if moved {
            host.submit_engine(self.client, 1, &serde_json::to_vec(&json!({"layers": [], "world": [op]})).map_err(|e| Error::Io(e.to_string()))?)?;
        }
        Ok(VmYield::WaitUntil(now + FRAME_MS))
    }
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Err(Error::Unsupported("Naughty Dog session snapshots"))
    }
    fn restore(&mut self, _bytes: &[u8]) -> crysirror_core::Result<()> {
        Err(Error::Unsupported("Naughty Dog session snapshots"))
    }
    fn diagnostics(&self) -> Vec<u8> {
        let v = match &self.state {
            None => json!({"loaded": false, "config": {"level": self.config.level, "task": self.config.task}}),
            Some(s) => {
                let mut v = json!({"loaded": s.pending.is_empty(), "pending": s.pending.len(), "packages": s.packages, "triangles": s.triangles, "eye": s.camera.eye});
                match (&s.game, &s.plan) {
                    (Some(game), Some(plan)) => {
                        let rt = &game.rt;
                        let states: Vec<_> = rt.inner.procs.iter().enumerate().filter(|(_, p)| !p.dead).take(64).map(|(i, p)| format!("{:08x}:{:08x}", p.name, rt.state_name(i).unwrap_or(0))).collect();
                        v["task"] = json!(plan.name);
                        v["level"] = json!(plan.level);
                        v["levels"] = json!(plan.levels);
                        v["load_regions"] = json!(plan.regions);
                        v["modules"] = json!(game.loaded.len());
                        v["processes"] = json!(rt.inner.procs.iter().filter(|p| !p.dead).count());
                        v["states"] = json!(states);
                        v["missing_natives"] = json!(rt.missing());
                        v["script_errors"] = json!(rt.inner.errors.iter().take(32).collect::<Vec<_>>());
                        v["script_error"] = json!(s.script_error);
                        v["log"] = json!(s.stage.log);
                        v["player"] = json!(s.player.as_ref().map(|p| p.position));
                        v["ground_triangles"] = json!(s.ground.len());
                        v["movie"] = json!(s.movie);
                    }
                    _ => v["level"] = json!(s.levels[s.current].0),
                }
                v
            }
        };
        serde_json::to_vec(&v).unwrap_or_default()
    }
}

#[cfg(feature = "runtime")]
pub fn register(registry: &mut crysirror_runtime::AdapterRegistry) -> Result<()> {
    registry.register(ENGINE_ID, Arc::new(Factory))
}

#[cfg(feature = "runtime")]
struct Factory;

#[cfg(feature = "runtime")]
impl crysirror_runtime::runtime::VmFactory for Factory {
    fn create(&self, client: ClientId, spec: &crysirror_runtime::model::ClientSpec) -> Result<Box<dyn VmAdapter>> {
        ensure!(spec.program.is_empty(), "the Naughty Dog player executes no data actions");
        Ok(Box::new(NaughtyDogVm::new(client, serde_json::from_value(spec.config.clone())?)?))
    }
}

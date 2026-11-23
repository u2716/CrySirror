// SPDX-License-Identifier: MPL-2.0
//! Level viewer session for the installed BCAS20097 disc: mounts
//! `pak22.psarc` through client storage, decodes the selected level's
//! background and prototype meshes with their base-colour textures and
//! presents them in the shared world renderer.
//!
//! Native boot, scripts, actors' placement and the original camera are not
//! executed (SATD-nd-retail-runtime). The camera starts outside the level
//! bounds and is user-controlled: WASD/QE move, arrows turn, Shift is fast,
//! Page Down/Page Up switch to the next/previous level of `pak22.txt`.
use crate::{levels, package::Directory, scene};
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
use std::sync::Arc;

pub const ENGINE_ID: &str = "naughty-dog";
const MAIN: &str = "PS3_GAME/USRDIR/build/main";
/// Largest decoded package and base-colour image edge for the viewer.
const PACKAGE_LIMIT: u64 = 256 << 20;
const MAX_TEXTURE: u32 = 1024;
const FRAME_MS: u64 = 16;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// `level-name` of `pak22.txt`.
    #[serde(default = "default_level")]
    pub level: String,
}

fn default_level() -> String {
    "menu-temple".into()
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.level.is_empty() && self.level.len() <= 128 && self.level.bytes().all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c)),
            "Naughty Dog level name"
        );
        Ok(())
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

/// The mounted disc data and the presented level.
struct Session {
    mount: ArchiveMount,
    /// Level names of `pak22.txt` in native order, with their packages.
    levels: Vec<(String, Vec<String>)>,
    current: usize,
    camera: Camera,
    last_ms: u64,
    triangles: usize,
    packages: Vec<String>,
    /// Spawned entities and their uploaded models.
    entities: Vec<(u32, VfsPath)>,
    next_model: u32,
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
        let levels: Vec<(String, Vec<String>)> = text
            .lines()
            .filter_map(|l| l.strip_prefix("level-name "))
            .filter_map(|n| all.get(n).map(|l| (n.to_owned(), l.packages.clone())))
            .collect();
        let current = levels.iter().position(|(n, _)| *n == self.config.level).with_context(|| format!("unknown Naughty Dog level {}", self.config.level))?;
        let mut registry = FormatRegistry::default();
        crysirror_formats_arc::register(&mut registry)?;
        let mut context = DecodeContext::new(self.client);
        context.limits.max_entry_size = PACKAGE_LIMIT;
        let (_, archive) = registry.open(open("pak22.psarc")?, Some("sony/psarc"), &context)?;
        let mount = ArchiveMount::new(archive, context)?;
        let camera = Camera { eye: [0.; 3], yaw: 0., pitch: 0., speed: 5. };
        let mut session = Session { mount, levels, current, camera, last_ms: host.now_ms(), triangles: 0, packages: Vec::new(), entities: Vec::new(), next_model: 0 };
        self.load(host, &mut session)?;
        Ok(session)
    }

    /// Replace the presented level with `session.current`.
    fn load(&self, host: &mut dyn HostServices, session: &mut Session) -> Result<()> {
        let mut world = vec![json!({"op": "set_environment", "environment": {"show_sky": true, "intensity": 1.0, "ambient": [0.55, 0.55, 0.55], "sun": {"direction": [0.3, -0.8, -0.4], "color": [1.0, 0.97, 0.9], "intensity": 0.6, "shadow_strength": 0.0}}})];
        for (id, asset) in session.entities.drain(..) {
            world.push(json!({"op": "despawn", "id": id}));
            host.release_model(self.client, &asset)?;
        }
        let packages = session.levels[session.current].1.clone();
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        session.triangles = 0;
        session.packages.clear();
        // Native package variant selection is unresolved: numbered
        // siblings (`…-ingame10.pak`) are not substituted.
        for package_name in &packages {
            let path = format!("{package_name}.pak");
            if !session.mount.exists(&path) {
                continue;
            }
            let package = session.mount.open(&path)?;
            let directory = Directory::read(package.as_ref())?;
            let meshes = scene::package_meshes(package.as_ref(), &directory, PACKAGE_LIMIT)?;
            let Some((l, h)) = scene::bounds(&meshes) else { continue };
            for k in 0..3 {
                lo[k] = lo[k].min(l[k]);
                hi[k] = hi[k].max(h[k]);
            }
            session.triangles += meshes.iter().flat_map(|m| &m.fragments).map(|f| f.triangles.len()).sum::<usize>();
            let glb = scene::document(package_name, &meshes, package.as_ref(), &directory, MAX_TEXTURE)?.to_glb()?;
            session.next_model += 1;
            let asset = host.upload_model(self.client, 0x4e44_0000 + session.next_model, &glb)?;
            let id = 0x100 + session.next_model;
            world.push(json!({"op": "spawn", "entity": {"id": id, "name": package_name, "geometry": {"kind": "scene", "asset": asset.relative()}}}));
            session.entities.push((id, asset));
            session.packages.push(package_name.clone());
        }
        if session.packages.is_empty() {
            // A level without installed meshes (for example only a numbered
            // variant): keep the camera at the origin.
            lo = [-500.; 3];
            hi = [500.; 3];
        }
        let m = scene::METRES_PER_UNIT;
        let (center, size): ([f32; 3], f32) = (std::array::from_fn(|k| (lo[k] + hi[k]) * 0.5 * m), (0..3).map(|k| (hi[k] - lo[k]) * m).fold(0., f32::max));
        // Start south of the bounds, slightly above, looking at the centre.
        session.camera = Camera { eye: [center[0], center[1] + size * 0.15, center[2] + size * 0.6], yaw: 0., pitch: -0.15, speed: (size * 0.1).clamp(2., 50.) };
        world.push(session.camera.op());
        host.submit_engine(self.client, 1, &serde_json::to_vec(&json!({"layers": [], "world": world}))?)?;
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
        // Page Down / Page Up: next / previous level of `pak22.txt`.
        let step = i32::from(input.buttons_pressed & b::PAGE_DOWN != 0) - i32::from(input.buttons_pressed & b::PAGE_UP != 0);
        if step != 0 {
            let n = session.levels.len() as i32;
            session.current = (session.current as i32 + step).rem_euclid(n) as usize;
            let loaded = self.load(host, &mut session);
            self.state = Some(session);
            loaded.map_err(fail)?;
            return Ok(VmYield::WaitUntil(host.now_ms() + FRAME_MS));
        }
        let now = host.now_ms();
        let seconds = (now.saturating_sub(session.last_ms)).min(100) as f32 / 1000.;
        session.last_ms = now;
        let moved = session.camera.step(&input, seconds);
        let op = session.camera.op();
        self.state = Some(session);
        if moved {
            host.submit_engine(self.client, 1, &serde_json::to_vec(&json!({"layers": [], "world": [op]})).map_err(|e| Error::Io(e.to_string()))?)?;
        }
        Ok(VmYield::WaitUntil(now + FRAME_MS))
    }
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Err(Error::Unsupported("Naughty Dog viewer snapshots"))
    }
    fn restore(&mut self, _bytes: &[u8]) -> crysirror_core::Result<()> {
        Err(Error::Unsupported("Naughty Dog viewer snapshots"))
    }
    fn diagnostics(&self) -> Vec<u8> {
        let v = match &self.state {
            None => json!({"level": self.config.level, "loaded": false}),
            Some(s) => json!({"level": s.levels[s.current].0, "loaded": true, "packages": s.packages, "triangles": s.triangles, "eye": s.camera.eye}),
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
        ensure!(spec.program.is_empty(), "the Naughty Dog viewer executes no data actions");
        Ok(Box::new(NaughtyDogVm::new(client, serde_json::from_value(spec.config.clone())?)?))
    }
}

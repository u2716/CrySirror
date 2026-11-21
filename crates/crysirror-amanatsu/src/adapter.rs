// SPDX-License-Identifier: MPL-2.0
//! The 甘夏ろけーしょん VM session: the installed Unity data, the active
//! build scene and its uGUI presentation through shared layer services.
use crate::{
    actionmap::{self, ActionMap},
    level::{Fade, Level},
    map::{self, MapInfo},
    scenes,
    sound::{self, Sound},
    ui::{Host, Ui},
    unity::Unity,
};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    path::VfsPath,
    service::{FontLibrary, HostServices, InputFrame, TextMeasurement},
    state::ClientId,
    vm::{VmAdapter, VmYield},
    Error,
};
use crysirror_unity::{ugui, PPtr};
use serde::Deserialize;
use serde_json::json;
use std::{cell::RefCell, sync::Arc};

pub const ENGINE_ID: &str = "amanatsu";
pub const WIDTH: f32 = 1920.;
pub const HEIGHT: f32 = 1080.;
/// Layer id of the scene fade canvas, above every uGUI layer.
const FADE_LAYER: u32 = 0x0F00;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Unity player data directory (`AmanatsuLocation_Data`).
    pub data_dir: String,
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        ensure!(!self.data_dir.is_empty() && !self.data_dir.contains(".."), "AmanatsuLocation data directory");
        Ok(())
    }
}

/// The running build scene's controller.
pub enum Controller {
    /// `InitScene`: global setup, then `_loadLevel`.
    Init,
    Logo(scenes::Logo),
    Title(scenes::Title),
    ActionMap(ActionMap),
    /// A scene whose controller is not ported: shown as laid out.
    Static,
}

pub struct Game {
    pub unity: Unity,
    pub ui: Ui,
    pub build_scenes: Vec<String>,
    pub level: Level,
    pub controller: Controller,
    pub sound: Sound,
    pub sound_ids: scenes::SoundIds,
    rng: u64,
    /// `Manager.Scene.SceneFadeCanvas` and its `_time`.
    pub fade: Fade,
    pub fade_seconds: f32,
    /// A `SceneSettings.Load` waiting for its fade-in.
    pub pending: Option<(i32, u64)>,
    pub now: u64,
    last: u64,
    /// Pressed Button: (hierarchy — 0 the build scene, 1.. overlays — node).
    pressed: Option<(usize, usize)>,
    /// Instantiated UI prefabs above the build scene, each with its uGUI
    /// presentation (`Ui` slot = index + 1).
    pub overlays: Vec<(Level, Ui)>,
    /// `map_info` rows (loaded with the ActionMap scene).
    pub maps: Vec<MapInfo>,
    /// A `ChangeMap(id, timeZone)` waiting for the covering fade.
    pub pending_map: Option<(i32, i32)>,
    /// World entities of the loaded map and their model assets.
    map_entities: Vec<(u32, VfsPath)>,
    next_model: u32,
    pub view: Option<map::View>,
    /// World commands and layer removals of this frame.
    world: Vec<serde_json::Value>,
    removed: Vec<serde_json::Value>,
    released: Vec<VfsPath>,
    pub quit: bool,
    pub log: Vec<String>,
}

/// `SceneSettings_00_Master`: id → scene name.
fn scene_name(id: i32) -> Option<&'static str> {
    Some(match id {
        0 => "Init",
        1 => "Logo",
        2 => "Attention",
        3 => "ConvertHumanDataScene",
        4 => "Title",
        5 => "CustomScene",
        6 => "ActionMap",
        7 => "Memory",
        _ => return None,
    })
}

impl Game {
    fn open(&mut self, name: &str) -> Result<()> {
        self.unload();
        let index = self.build_scenes.iter().position(|s| s == name).with_context(|| format!("scene {name} is not a build scene"))?;
        let key = format!("level{index}");
        let mut level = Level::load(&mut self.unity, &key, name)?;
        self.controller = match name {
            "Init" => Controller::Init,
            "Logo" => Controller::Logo(scenes::Logo::new(&level)),
            "Title" => Controller::Title(scenes::Title::new(&mut level, self.sound_ids)),
            "ActionMap" => {
                // `Initialize`: the sub fade canvas starts transparent, the
                // map list is read and `MapSelectUI` opens.
                if let Some(n) = level.find("SubFadeCanvas") {
                    level.states.entry(n).or_default().group_alpha = Some(0.);
                }
                self.maps = map::map_infos(&mut self.unity)?;
                let mut select = Level::prefab(&mut self.unity, actionmap::UI_BUNDLE, "MapSelectUI")?;
                let c = ActionMap::new(&mut select, &self.maps);
                self.overlays.push((select, Ui::new(1)));
                self.log.push("SATD-amanatsu-port: ActionMap tutorial ConfirmDialog, player card and ActionMapUI not ported".into());
                Controller::ActionMap(c)
            }
            _ => {
                self.log.push(format!("SATD-amanatsu-port: {name} scene controller not ported"));
                Controller::Static
            }
        };
        self.log.push(format!("scene {name} ({key})"));
        // Each scene's Start reveals it (`StartFadeAsync(Out)`).
        if !matches!(self.controller, Controller::Init) {
            self.fade.start(self.now, false, self.fade_seconds);
        }
        self.level = level;
        Ok(())
    }

    /// Unload the presentation of the previous scene: its layers, overlays
    /// and map entities (models are released by the next step).
    fn unload(&mut self) {
        let mut removed = self.ui.clear();
        self.ui = Ui::default();
        for (_, mut ui) in std::mem::take(&mut self.overlays) {
            removed.extend(ui.clear());
        }
        self.removed.append(&mut removed);
        for (entity, path) in std::mem::take(&mut self.map_entities) {
            self.world.push(json!({"op": "despawn", "id": entity}));
            self.released.push(path);
        }
        self.view = None;
    }

    /// Replace the map's world entities (`ReleaseMapObject` + load).
    fn change_map(&mut self, host: &mut dyn HostServices, client: ClientId, id: i32, time_zone: i32) -> Result<()> {
        self.release_map(host, client)?;
        let info = self.maps.iter().find(|m| m.id == id).cloned().with_context(|| format!("map {id} not in map_info"))?;
        let t = std::time::Instant::now();
        let world = map::load(&mut self.unity, &info, time_zone)?;
        for (i, glb) in world.chunks.iter().enumerate() {
            self.next_model += 1;
            let path = host.upload_model(client, 0x6000_0000 + self.next_model, glb)?;
            let entity = 0x100 + self.next_model;
            self.world.push(json!({"op": "spawn", "entity": {"id": entity, "name": format!("{}#{i}", info.scene.1), "geometry": {"kind": "scene", "asset": path.relative()}}}));
            self.map_entities.push((entity, path));
        }
        self.world.push(world.environment());
        self.view = world.camera().cloned();
        if let Some(v) = &self.view {
            self.world.push(v.command());
        }
        let r = &world.report;
        self.log.push(format!(
            "ChangeMap({id} {}, zone {time_zone}): {} chunks, {} renderers ({} LOD-culled, {} skipped), {} materials, {} textures in {:?}",
            info.name,
            world.chunks.len(),
            r.renderers,
            r.lod_culled,
            r.skipped_renderers,
            r.materials,
            r.textures,
            t.elapsed()
        ));
        for (shader, (support, n)) in &r.shaders {
            if support != "Mapped" {
                self.log.push(format!("map shader {shader}: {support} ({n} materials)"));
            }
        }
        self.log.extend(r.errors.iter().take(8).cloned());
        self.log.extend(world.log);
        Ok(())
    }

    fn release_map(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<()> {
        for (entity, path) in std::mem::take(&mut self.map_entities) {
            self.world.push(json!({"op": "despawn", "id": entity}));
            self.released.push(path);
        }
        for path in std::mem::take(&mut self.released) {
            host.release_model(client, &path)?;
        }
        Ok(())
    }

    /// Run this frame's sound requests of the scene logic.
    fn sounds(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<()> {
        for call in std::mem::take(&mut self.level.sounds) {
            let r = match &call {
                sound::Call::Bgm(id) => self.sound.play_bgm(host, client, &mut self.unity, *id),
                sound::Call::SystemSe(id) => self.sound.play_system_se(host, client, &mut self.unity, *id),
                sound::Call::StopAll => self.sound.stop_all(host, client),
                sound::Call::RandomClip(prefix) => (|| {
                    let clips = Sound::clips_under(&mut self.unity, prefix)?;
                    if clips.is_empty() {
                        return Ok(());
                    }
                    // xorshift: `ShuffleSelf` then the first element.
                    self.rng ^= self.rng << 13;
                    self.rng ^= self.rng >> 7;
                    self.rng ^= self.rng << 17;
                    let (bundle, name) = &clips[(self.rng % clips.len() as u64) as usize];
                    self.sound.play(host, client, &mut self.unity, sound::Kind::SystemSe, bundle, name, 1.)
                })(),
            };
            if let Err(e) = r {
                self.log.push(format!("sound {call:?}: {e:#}"));
            }
        }
        self.sound.poll(host, client)?;
        self.log.append(&mut self.sound.log);
        Ok(())
    }
}

struct HostAdapter<'a> {
    host: &'a mut dyn HostServices,
    client: ClientId,
}

impl Host for HostAdapter<'_> {
    fn upload(&mut self, id: u32, width: u32, height: u32, rgba: &[u8]) -> Result<String> {
        Ok(self.host.upload_rgba(self.client, id, width, height, rgba)?.relative().to_owned())
    }
    fn fonts(&mut self) -> Result<Arc<dyn FontLibrary>> {
        Ok(self.host.font_library(self.client)?)
    }
    fn measurement(&mut self) -> Result<Arc<dyn TextMeasurement>> {
        Ok(self.host.text_measurement(self.client)?)
    }
}

pub struct AmanatsuVm {
    client: ClientId,
    config: Config,
    game: Option<Game>,
    error: Option<String>,
}

impl AmanatsuVm {
    pub fn new(client: ClientId, config: Config) -> Result<Self> {
        config.validate()?;
        Ok(Self { client, config, game: None, error: None })
    }

    fn boot(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let storage = host.storage(self.client)?;
        let mut unity = Unity::new(storage, &self.config.data_dir)?;
        let build_scenes = unity.build_scenes()?;
        let init = Level::load(&mut unity, "level0", "Init")?;
        // InitScene: `SceneFadeCanvas.SetColor(_fadeColor)` over a covered
        // screen, then `_loadLevel` by build index.
        let f = init.scene.nodes.iter().flat_map(|n| &n.components).find(|c| c.script == "AL.Scene.InitScene").map(|c| c.fields.clone()).unwrap_or_default();
        let rgba = f.fields("_fadeColor").and_then(|c| c.int("rgba")).unwrap_or(0xFFFF_FFFF) as u32;
        let color = [rgba & 0xFF, (rgba >> 8) & 0xFF, (rgba >> 16) & 0xFF].map(|c| c as f32 / 255.);
        let load = f.int("_loadLevel").unwrap_or(1) as usize;
        let sound = Sound::load(&mut unity)?;
        let pd = unity.find_script_object("StreamingAssets/aa", "AL.PropertyData")?.context("AL.PropertyData")?;
        let sse = pd.fields("_sseID").cloned().unwrap_or_default();
        let sound_ids = scenes::SoundIds {
            ok_s: sse.int("OK_s").unwrap_or(1) as i32,
            ok_l: sse.int("OK_l").unwrap_or(2) as i32,
            cancel: sse.int("Cancel").unwrap_or(3) as i32,
            title_bgm: pd.fields("_bgmID").and_then(|b| b.int("Title")).unwrap_or(0) as i32,
        };
        let mut game = Game {
            sound,
            sound_ids,
            rng: host.now_ms() | 1,
            unity,
            ui: Ui::default(),
            build_scenes,
            level: init,
            controller: Controller::Init,
            fade: Fade::covered(color),
            // SceneFadeCanvas prefab `_time` (Addressables manager bundle).
            fade_seconds: 1.0,
            pending: None,
            now: host.now_ms(),
            last: host.now_ms(),
            pressed: None,
            overlays: Vec::new(),
            maps: Vec::new(),
            pending_map: None,
            map_entities: Vec::new(),
            next_model: 0,
            view: None,
            world: Vec::new(),
            removed: Vec::new(),
            released: Vec::new(),
            quit: false,
            log: Vec::new(),
        };
        let name = game.build_scenes.get(load).cloned().context("InitScene._loadLevel")?;
        game.open(&name)?;
        self.game = Some(game);
        Ok(())
    }

    fn step_inner(&mut self, host: &mut dyn HostServices, input: InputFrame) -> Result<VmYield> {
        use crysirror_core::service::buttons as b;
        if self.game.is_none() {
            self.boot(host)?;
        }
        let game = self.game.as_mut().context("game")?;
        game.now = host.now_ms();
        let dt = (game.now.saturating_sub(game.last)).min(100) as f32 / 1000.;
        game.last = game.now;
        let now = game.now;

        // Scene loads wait for their fade-in.
        if let Some((id, _)) = game.pending {
            if !game.fade.fading(now) {
                game.pending = None;
                let name = scene_name(id).context("SceneSettings id")?;
                game.open(name)?;
            }
        }
        game.level.advance(dt);
        for (o, _) in &mut game.overlays {
            o.advance(dt);
        }
        let client = self.client;
        // A map change waits for the covering fade, then loads behind it.
        if let Some((id, zone)) = game.pending_map {
            if !game.fade.fading(now) {
                game.pending_map = None;
                if let Some((_, mut ui)) = (!game.overlays.is_empty()).then(|| game.overlays.remove(0)) {
                    game.removed.extend(ui.clear());
                }
                game.change_map(host, client, id, zone)?;
                game.fade.start(host.now_ms(), false, game.fade_seconds);
            }
        }
        if !game.released.is_empty() {
            game.release_map(host, client)?;
        }

        // Layout and compose the build scene (slot 0) and each overlay.
        let mut ops = std::mem::take(&mut game.removed);
        let mut layouts = Vec::new();
        {
            let Game { unity, ui, level, overlays, .. } = &mut *game;
            let mut hierarchies: Vec<(&mut Level, &mut Ui)> = vec![(level, ui)];
            hierarchies.extend(overlays.iter_mut().map(|(l, u)| (l, u)));
            for (slot, (level, ui)) in hierarchies.into_iter().enumerate() {
                let key = level.key.clone();
                let states = level.frame_states();
                let cell = RefCell::new((&mut *unity, &mut *ui, HostAdapter { host: &mut *host, client }));
                let info = |p: PPtr| {
                    let mut c = cell.borrow_mut();
                    c.0.sprite_meta(&key, p).map(|(b, _, ppu)| (b, ppu))
                };
                let size = |p: PPtr| {
                    let mut c = cell.borrow_mut();
                    c.0.sprite_meta(&key, p).map(|(_, s, ppu)| (s, ppu))
                };
                let measure = |t: &str, f: PPtr, px: f32| {
                    let mut c = cell.borrow_mut();
                    let (u, ui, h) = &mut *c;
                    ui.measure(u, h, &key, t, f, px)
                };
                let svc = ugui::Services { sprite_info: &info, sprite_size: &size, measure: &measure };
                let layout = ugui::layout(&level.scene, &states, WIDTH, HEIGHT, &svc);
                drop(svc);
                let (unity, ui, mut h) = cell.into_inner();
                ops.extend(ui.compose(unity, &mut h, &key, &layout, slot as i32 * 1_000_000)?);
                layouts.push(layout);
            }
        }
        let a = game.fade.alpha(now);
        let c = game.fade.color.map(|v| (v * 255. + 0.5) as u8);
        ops.push(json!({"op": "put", "layer": {"id": FADE_LAYER, "rect": [0., 0., WIDTH, HEIGHT], "paint": {"kind": "solid", "color": [c[0], c[1], c[2], (a.clamp(0., 1.) * 255. + 0.5) as u8]}, "order": [i32::MAX / 2, 0]}}));
        let world = std::mem::take(&mut game.world);
        host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers": ops, "world": world}))?)?;

        // Input: a Button click is a press and release on the same handler;
        // the topmost hierarchy with a hit receives it.
        let fading = game.fade.fading(now) || game.pending.is_some() || game.pending_map.is_some();
        let (x, y) = (input.pointer_x as f32, input.pointer_y as f32);
        let mut handler = None;
        for (h, layout) in layouts.iter().enumerate().rev() {
            let level = if h == 0 { &game.level } else { &game.overlays[h - 1].0 };
            if let Some(n) = layout.hits.iter().rev().find(|(_, r)| r.contains(x, y)).map(|(n, _)| *n) {
                handler = level.click_handler(n).filter(|&n| level.selectable_interactable(n) && !layout.non_interactable.contains(&n)).map(|n| (h, n));
                break;
            }
        }
        let mut click = None;
        if input.buttons_pressed & b::MOUSE_LEFT != 0 {
            game.pressed = handler;
        }
        if input.buttons_down & b::MOUSE_LEFT == 0 {
            if let Some(p) = game.pressed.take() {
                if Some(p) == handler && !fading {
                    click = Some(p);
                }
            }
        }
        let scene_input = scenes::Input {
            now,
            fading,
            click: (input.buttons_pressed & b::MOUSE_LEFT != 0).then_some((x, y)),
            right_click: input.buttons_pressed & b::MOUSE_RIGHT != 0,
            escape: input.buttons_pressed & b::ESCAPE != 0,
            f1: input.buttons_pressed & b::F1 != 0,
        };
        let mut request = match &mut game.controller {
            Controller::Init | Controller::Static => scenes::Request::None,
            Controller::Logo(l) => l.step(&mut game.level, &scene_input),
            Controller::Title(t) => t.step(&mut game.level, &scene_input),
            Controller::ActionMap(m) => {
                if (scene_input.right_click || scene_input.escape) && !fading {
                    if let Some((select, _)) = game.overlays.first_mut() {
                        m.back(select);
                    }
                }
                scenes::Request::None
            }
        };
        if let Some((h, button)) = click {
            if h > 0 {
                let level = &mut game.overlays[h - 1].0;
                game.log.push(format!("click {}", level.scene.path(button)));
                let calls = level.on_click(button);
                let script = level.invoke(&calls);
                match &mut game.controller {
                    Controller::ActionMap(m) if h == 1 => match m.click(level, button) {
                        actionmap::Action::ChangeMap { id, time_zone } => {
                            game.level.sounds.push(sound::Call::SystemSe(game.sound_ids.ok_l));
                            game.fade.start(now, true, game.fade_seconds);
                            game.pending_map = Some((id, time_zone));
                            game.log.push(format!("MapSelectCallBack → ChangeMap({id}, {time_zone})"));
                        }
                        actionmap::Action::Scene(r) => request = r,
                        actionmap::Action::None => {}
                    },
                    _ => {}
                }
                for call in script {
                    level.log.push(format!("unported script call {}.{}", call.type_name, call.method));
                }
                let mut l = std::mem::take(&mut level.log);
                game.log.append(&mut l);
            } else {
                let calls = game.level.on_click(button);
                game.log.push(format!("click {}", game.level.scene.path(button)));
                for call in game.level.invoke(&calls) {
                    let r = match &mut game.controller {
                        Controller::Title(t) => t.call(&mut game.level, &call, now),
                        _ => {
                            game.level.log.push(format!("unported script call {}.{}", call.type_name, call.method));
                            scenes::Request::None
                        }
                    };
                    if r != scenes::Request::None {
                        request = r;
                    }
                }
            }
        }
        for (o, _) in &mut game.overlays {
            game.log.append(&mut o.log);
        }
        game.sounds(host, client)?;
        game.log.append(&mut game.level.log);
        match request {
            scenes::Request::None => {}
            scenes::Request::Quit => game.quit = true,
            scenes::Request::Load(id) if game.pending.is_none() => {
                // SceneSettings FadeType In: cover, then load.
                game.fade.start(now, true, game.fade_seconds);
                game.pending = Some((id, now));
                game.log.push(format!("SceneSettings.Load({id})"));
            }
            scenes::Request::Load(_) => {}
        }
        if game.quit {
            return Ok(VmYield::Halted);
        }
        Ok(VmYield::WaitUntil(game.now + 16))
    }
}

impl VmAdapter for AmanatsuVm {
    fn engine_id(&self) -> &'static str {
        ENGINE_ID
    }
    fn snapshot_schema(&self) -> u32 {
        1
    }
    fn preparation_pending(&self) -> bool {
        false
    }
    fn step(&mut self, host: &mut dyn HostServices, input: InputFrame, _budget: u32) -> crysirror_core::Result<VmYield> {
        match self.step_inner(host, input) {
            Ok(v) => Ok(v),
            Err(e) => {
                let message = format!("{e:#}");
                self.error = Some(message.clone());
                Err(Error::Io(message))
            }
        }
    }
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Err(Error::Unsupported("AmanatsuLocation framework snapshot"))
    }
    fn restore(&mut self, _: &[u8]) -> crysirror_core::Result<()> {
        Err(Error::Unsupported("AmanatsuLocation framework snapshot"))
    }
    fn diagnostics(&self) -> Vec<u8> {
        let Some(g) = &self.game else {
            return serde_json::to_vec(&json!({"error": self.error})).unwrap_or_default();
        };
        serde_json::to_vec(&json!({
            "scene": g.level.name,
            "level": g.level.key,
            "fade": g.fade.alpha(g.now),
            "menu": match &g.controller { Controller::Title(t) => format!("{:?}", t.select), Controller::ActionMap(m) => format!("{:?}", m.phase), _ => String::new() },
            "overlays": g.overlays.iter().map(|(l, _)| l.name.clone()).collect::<Vec<_>>(),
            "map_entities": g.map_entities.len(),
            "camera": g.view.as_ref().map(|v| json!({"eye": v.eye, "target": v.target, "fov": v.fov})),
            "error": self.error,
            "hits": g.ui.hits.iter().map(|(n, r)| json!({"path": g.level.scene.path(*n), "rect": [r.x0, r.y0, r.x1, r.y1]})).chain(g.overlays.iter().flat_map(|(l, u)| u.hits.iter().map(|(n, r)| json!({"path": l.scene.path(*n), "rect": [r.x0, r.y0, r.x1, r.y1]})))).collect::<Vec<_>>(),
            "log": g.log.iter().rev().take(32).collect::<Vec<_>>(),
        }))
        .unwrap_or_default()
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
        ensure!(spec.program.is_empty(), "AmanatsuLocation executes native scenes, not data actions");
        Ok(Box::new(AmanatsuVm::new(client, serde_json::from_value(spec.config.clone())?)?))
    }
}

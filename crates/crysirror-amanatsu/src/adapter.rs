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
/// ILLGAMES characters are modelled at ten Unity units per metre: physics
/// accelerations scale by this.
const CHARACTER_UNIT: f32 = 10.;
/// The first default heroine card of the installation.
const DEFAULT_FEMALE_CARD: &str = "DefaultData/0/chara/female/AL_F_00.png";
/// The first custom-scene background sample of the installation.
const DEFAULT_CUSTOM_BG: &str = "DefaultData/common/bg/al_mapsample00.png";
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

/// `EventManager.Massage` progress.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MassagePhase {
    /// The `agree` line on the current map.
    Agree,
    /// Fading to the esthetics map.
    Enter,
    /// `GetMassageStartAsset` before the massage.
    Before,
    /// `OilScene.PlayAsync`.
    Play,
    /// The `Evaluation` scenario after Exit.
    Evaluation,
}

/// A massage in progress (`EventManager.Massage` + `AL.Oil.OilScene`).
struct MassageRun {
    card: String,
    phase: MassagePhase,
    /// The map to return to.
    return_map: i32,
    state: crate::oil::Massage,
    /// The heroine's model for CPU posing, her placement (engine space)
    /// and the pose clip start.
    model: Option<crysirror_asset::pbr::PbrModel>,
    placement: glam::Mat4,
    started: u64,
    last_pointer: Option<(f32, f32)>,
    last_ms: u64,
    stage: usize,
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
    /// `NPCData` map of each registered heroine (by card) in this zone.
    placement: std::collections::HashMap<String, i32>,
    excluded_maps: Vec<i32>,
    map_limit: usize,
    /// `PropertyData._param`.
    param_table: crate::params::ParamTable,
    /// `NPCData.GameParameter`/`Count` and card characteristic by card.
    npc: std::collections::HashMap<String, (crate::params::GameParameter, crate::params::Count, usize)>,
    /// `PropertyData._adv` assets and the `_mapID` → map asset keys.
    adv_assets: crate::commands::AdvAssets,
    adv_maps: std::collections::HashMap<i32, &'static str>,
    /// `ExploreProperties.Cost` and its `_costUpLVs`.
    cost: (crate::commands::CostProperty, Vec<i32>),
    /// A command was played during the current menu visit.
    visit_played: bool,
    /// `AL.Oil` tables (loaded on the first massage) and the massage.
    oil: Option<crate::oil::OilData>,
    massage: Option<MassageRun>,
    /// A loaded save waiting for the ActionMap scene.
    loaded_save: Option<crate::save::SaveFile>,
    /// `SaveData.Save` after `EventManager.TimeZone` (zone).
    pending_auto_save: Option<i32>,
    client_id: ClientId,
    /// Command points waiting for the conversation to end: (card, kind).
    pending_points: Option<(String, &'static str)>,
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
    /// Light entities of the loaded map.
    map_lights: Vec<u32>,
    /// The uploaded sky of the map's time zone.
    sky: Option<VfsPath>,
    next_model: u32,
    pub view: Option<map::View>,
    /// The map's move-point cameras and the current/default one
    /// (`ActionMapUI._rpMapCameraID`).
    cameras: Vec<map::View>,
    camera: usize,
    default_camera: usize,
    /// `ActionMapUI` (overlay 0 while a map is shown).
    hud: Option<crate::hud::Hud>,
    /// Talk steps waiting for host services.
    pending_steps: Vec<crate::talk::Step>,
    /// Registered heroines (card paths), `SaveData.NPCDataList` order.
    pub heroines: Vec<String>,
    chara_select: Option<crate::charaselect::CharaSelect>,
    /// Heroines standing on the map: (card, engine-space feet position,
    /// personality, first name).
    spots: Vec<(String, [f32; 3], i64, String)>,
    /// Heroines met (first-meeting events played), by card.
    met: std::collections::HashSet<String>,
    /// The command menu: (spot, overlay index).
    command: Option<(usize, usize)>,
    /// A conversation in progress and its overlay index.
    talk: Option<(crate::talk::Talk, usize)>,
    /// `chara/list/characustom` tables (loaded once).
    lists: Option<crate::chara::Lists>,
    /// Uploaded card thumbnails by card path.
    thumbnails: std::collections::HashMap<String, String>,
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
                    let st = level.states.entry(n).or_default();
                    st.group_alpha = Some(0.);
                    st.blocks_raycasts = Some(false);
                }
                self.maps = map::map_infos(&mut self.unity)?;
                let mut select = Level::prefab(&mut self.unity, actionmap::UI_BUNDLE, "MapSelectUI")?;
                let mut c = ActionMap::new(&mut select, &self.maps);
                if let Some(file) = self.loaded_save.take() {
                    c.time_zone = file.save.time_zone.clamp(0, 3);
                    crate::hud::set_image_sprites(&mut select, c.time_zone as usize);
                    self.apply_save(file);
                }
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
        for id in std::mem::take(&mut self.map_lights) {
            self.world.push(json!({"op": "despawn", "id": id}));
        }
        self.hud = None;
        self.view = None;
        self.spots.clear();
    }

    /// Replace the map's world entities (`ReleaseMapObject` + load).
    fn change_map(&mut self, host: &mut dyn HostServices, client: ClientId, id: i32, time_zone: i32) -> Result<()> {
        self.release_map(host, client)?;
        self.spots.clear();
        let info = self.maps.iter().find(|m| m.id == id).cloned().with_context(|| format!("map {id} not in map_info"))?;
        let t = std::time::Instant::now();
        let world = map::load(&mut self.unity, &info, time_zone)?;
        self.spawn_models(host, client, &info.scene.1, &world.chunks)?;
        let mut env = world.environment();
        if let Some(sky) = &world.sky {
            let img = sky.render(512, 256)?;
            self.next_model += 1;
            let path = host.upload_hdr(client, 0x6000_0000 + self.next_model, img.width, img.height, &img.rgb)?;
            env["environment"]["hdr"] = json!(path.relative());
            // The previous sky is no longer the environment: release it.
            if let Some(old) = self.sky.replace(path) {
                host.release_model(client, &old)?;
            }
        }
        self.world.push(env);
        for spawn in world.light_entities(0x4000) {
            if let Some(id) = spawn["entity"]["id"].as_u64() {
                self.map_lights.push(id as u32);
            }
            self.world.push(spawn);
        }
        // Registered heroines at the map's standing action points
        // (SATD-amanatsu-port: `ActionPoint` choice, their `_animIDList`
        // motions and NavMesh wandering are not ported).
        self.place_heroines(false);
        let massage_entry = self.massage.as_ref().filter(|m| m.phase == MassagePhase::Enter).map(|m| m.card.clone());
        if let Some(card) = massage_entry {
            if let Some((_, pos, rot)) = world.points.iter().find(|p| p.0 == "Est00").cloned() {
                self.spawn_oil_heroine(host, client, &card, pos, rot, world.sun.as_ref().map_or(1., |l| l.intensity))?;
            } else {
                self.log.push("SATD-amanatsu-port: esthetics map without Est00".into());
            }
        }
        let heroines: Vec<String> = if self.massage.is_some() { Vec::new() } else { self.heroines.iter().filter(|h| self.placement.get(*h) == Some(&id)).cloned().collect() };
        for (card, point) in heroines.iter().zip(world.points.iter().filter(|p| p.0.contains("tati"))) {
            let (_, pos, rot) = point;
            let q = glam::Quat::from_array(crysirror_unity::gltf::mirror_quat(*rot));
            let (x, y, z) = q.to_euler(glam::EulerRot::XYZ);
            let t = json!({"translation": crysirror_unity::gltf::mirror_point(*pos), "rotation_degrees": [x.to_degrees(), y.to_degrees(), z.to_degrees()], "scale": [1.0, 1.0, 1.0]});
            let sun = world.sun.as_ref().map_or(1., |l| l.intensity);
            match self.spawn_character(host, client, card, Some(t), sun) {
                Ok(()) => {
                    let (personality, name) = self.unity.read_all(card).ok().and_then(|b| crate::card::Card::parse(&b).ok()).and_then(|c| c.parameter()).map_or((0, String::new()), |p| (p.0, p.1));
                    self.spots.push((card.clone(), crysirror_unity::gltf::mirror_point(*pos), personality, name));
                }
                Err(e) => self.log.push(format!("heroine {card}: {e:#}")),
            }
        }
        self.cameras = world.cameras.iter().map(|c| c.0.clone()).collect();
        let best = world.cameras.iter().map(|c| c.1).max().unwrap_or(0);
        self.default_camera = world.cameras.iter().position(|c| c.1 == best).unwrap_or(0);
        self.camera = self.default_camera;
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

    /// Upload GLB chunks and spawn them as world entities at the origin.
    fn spawn_models(&mut self, host: &mut dyn HostServices, client: ClientId, name: &str, chunks: &[Vec<u8>]) -> Result<()> {
        for (i, glb) in chunks.iter().enumerate() {
            self.next_model += 1;
            let path = host.upload_model(client, 0x6000_0000 + self.next_model, glb)?;
            let entity = 0x100 + self.next_model;
            self.world.push(json!({"op": "spawn", "entity": {"id": entity, "name": format!("{name}#{i}"), "geometry": {"kind": "scene", "asset": path.relative()}}}));
            self.map_entities.push((entity, path));
        }
        Ok(())
    }

    /// Assemble a card's character, posed by the motion controller's
    /// default state (`Pose_D_00_Loop`), and spawn it with an engine-space
    /// transform (`None`: at the origin).
    fn spawn_character(&mut self, host: &mut dyn HostServices, client: ClientId, card_path: &str, transform: Option<serde_json::Value>, sun: f32) -> Result<()> {
        self.spawn_character_motion(host, client, card_path, transform, sun, None).map(|_| ())
    }

    /// `spawn_character` with a controller state (bundle, controller,
    /// state) instead of the idle action motion; returns the GLB chunks.
    fn spawn_character_motion(&mut self, host: &mut dyn HostServices, client: ClientId, card_path: &str, transform: Option<serde_json::Value>, sun: f32, motion: Option<(&str, &str, &str)>) -> Result<Vec<Vec<u8>>> {
        let idle: &str = motion.map_or("Pose_D_00_Loop", |m| m.2);
        let card = crate::card::Card::parse(&self.unity.read_all(card_path)?)?;
        if self.lists.is_none() {
            self.lists = Some(crate::chara::Lists::load(&mut self.unity)?);
        }
        let lists = self.lists.take().context("lists")?;
        let human = crate::chara::Human::from_card(&mut self.unity, &lists, &card, true);
        self.lists = Some(lists);
        let human = human?;
        self.log.extend(human.log.iter().cloned());
        let options = crysirror_unity::world6::GlbOptions { max_texture: 1024, texture_budget: 48 << 20, spring_unit: CHARACTER_UNIT };
        let tracks = match motion {
            Some((bundle, ctl, state)) => human.controller_tracks(&mut self.unity, bundle, ctl, state),
            None => human.state_tracks(&mut self.unity, idle),
        };
        let clips = match tracks {
            Ok(t) => vec![t],
            Err(e) => {
                self.log.push(format!("{idle}: {e:#}"));
                Vec::new()
            }
        };
        let mut assets = crate::chara::CharaAssets { unity: &mut self.unity, human: &human, light: 0.7 / sun.max(0.7) };
        let (chunks, report) = crysirror_unity::world6::scene_glb_animated(&human.scene, &human.key, &[0], &mut assets, &options, "CrySirror AmanatsuLocation character importer", &clips)?;
        let first = self.map_entities.len();
        self.spawn_models(host, client, "chara", &chunks)?;
        for (entity, _) in self.map_entities[first..].to_vec() {
            if let Some(t) = &transform {
                self.world.push(json!({"op": "set_transform", "id": entity, "transform": t}));
            }
            if !clips.is_empty() {
                self.world.push(json!({"op": "set_animation", "id": entity, "animation": {"layers": [{"clip": idle, "looping": true}]}}));
            }
        }
        self.log.push(format!("character {card_path}: {} renderers, {} materials, {} tracks", report.renderers, report.materials, clips.first().map_or(0, |c| c.tracks.len())));
        self.log.extend(report.errors.iter().take(8).cloned());
        Ok(chunks)
    }

    /// `CustomScene` start: the default heroine (`HumanData` preset) in
    /// front of the main camera.
    fn custom_scene(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<()> {
        // SATD-amanatsu-port: the scene's own default (`HumanData`
        // preset for the chosen sex) is approximated by the first default
        // female card.
        self.spawn_character(host, client, DEFAULT_FEMALE_CARD, None, 0.7)?;
        // The main camera's authored transform frames the character
        // (characters are about 16 units tall). SATD-amanatsu-port:
        // `CameraController` orbit/zoom input is not ported.
        if let Some(cam) = self.level.find("CustomScene/Camera/MainCamera") {
            let fov = self.level.scene.nodes[cam].components.iter().find(|c| c.class_id == 20).and_then(|c| c.fields.float("field of view")).unwrap_or(23.);
            let mut lens = crysirror_unity::managed::Fields::default();
            lens.0.push(("FieldOfView".into(), crysirror_unity::managed::Value::Float(fov)));
            let v = map::view(&self.level.scene, cam, Some(&lens));
            self.world.push(v.command());
            self.view = Some(v);
        }
        // `Cvs_BackGround` (screen space – camera, behind the character)
        // shows the background image: drawn as an image plane filling the
        // view far behind the character. SATD-amanatsu-port: the default
        // background is the first `DefaultData/common/bg` sample, not the
        // custom scene's saved choice.
        if let (Some(v), Ok(bytes)) = (self.view.clone(), self.unity.read_all(DEFAULT_CUSTOM_BG)) {
            match self.background_plane(&v, bytes, 80.) {
                Ok(glb) => self.spawn_models(host, client, "background", &[glb])?,
                Err(e) => self.log.push(format!("custom background: {e:#}")),
            }
        }
        // The canvases themselves are not drawn over the world.
        // `SaveFrame` canvases belong to the card-capture cameras.
        for path in ["CustomScene/UI/Cvs_BackGround", "CustomScene/UI/Root/Cvs_BackGround_Cover", "CustomScene/UI/Root/SaveFrame"] {
            if let Some(n) = self.level.find(path) {
                self.level.set_active(n, false);
            }
        }
        self.world.push(json!({"op": "set_environment", "environment": {"show_sky": false, "intensity": 1.0, "bloom": 0.0, "tonemap": "none", "ambient_occlusion": 0.0, "ambient": [0.15, 0.15, 0.16], "sun": {"direction": [0.3, -0.5, -0.8], "color": [1.0, 1.0, 1.0], "intensity": 0.7, "shadow_strength": 0.3}}}));
        Ok(())
    }

    /// An image filling the view `v` at `distance` (aspect preserved by
    /// covering, as `RawImage` with an aspect fitter would).
    fn background_plane(&self, v: &map::View, png: Vec<u8>, distance: f32) -> Result<Vec<u8>> {
        use glam::Vec3;
        let img = crysirror_image::decode_standard(&png)?;
        let eye = Vec3::from_array(v.eye);
        let fwd = (Vec3::from_array(v.target) - eye).normalize();
        let right = fwd.cross(Vec3::from_array(v.up)).normalize();
        let up = right.cross(fwd);
        let h = 2. * distance * (v.fov.to_radians() / 2.).tan() * 1.02;
        let w = h * WIDTH / HEIGHT;
        let aspect = img.width as f32 / img.height.max(1) as f32;
        let (w, h) = if aspect > w / h { (h * aspect, h) } else { (w, w / aspect) };
        let c = eye + fwd * distance;
        let corner = |x: f32, y: f32| (c + right * (x * w / 2.) + up * (y * h / 2.)).to_array();
        crysirror_unity::gltf::image_plane_glb("CrySirror AmanatsuLocation background", png, [corner(-1., 1.), corner(1., 1.), corner(1., -1.), corner(-1., -1.)])
    }

    /// A card's PNG thumbnail uploaded once.
    fn thumbnail(&mut self, host: &mut dyn HostServices, client: ClientId, card: &str) -> Option<String> {
        if let Some(a) = self.thumbnails.get(card) {
            return Some(a.clone());
        }
        let img = self.unity.read_all(card).ok().and_then(|b| crysirror_image::decode_standard(&b).ok())?;
        let id = 0x5000_0000 + self.thumbnails.len() as u32;
        let asset = host.upload_rgba(client, id, img.width, img.height, &img.rgba).ok()?.relative().to_owned();
        self.thumbnails.insert(card.to_owned(), asset.clone());
        Some(asset)
    }

    /// `CharaSelectUI.Open`: the preset heroine cards.
    fn open_chara_select(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<()> {
        let mut level = Level::prefab(&mut self.unity, actionmap::UI_BUNDLE, "CharaSelectUI")?;
        let dir = "DefaultData/0/chara/female";
        let cards: Vec<String> = self.unity.list_dir(dir).into_iter().filter(|n| n.ends_with(".png")).map(|n| format!("{dir}/{n}")).collect();
        let cards: Vec<(String, Option<String>)> = cards.into_iter().map(|c| (c.clone(), self.thumbnail(host, client, &c))).collect();
        let registered: Vec<(String, Option<String>)> = self.heroines.clone().into_iter().map(|c| (c.clone(), self.thumbnail(host, client, &c))).collect();
        self.chara_select = crate::charaselect::CharaSelect::new(&mut level, &cards, &registered);
        match &self.chara_select {
            Some(cs) => self.log.push(cs.describe(&level)),
            None => self.log.push("CharaSelectUI: list controls not found".into()),
        }
        self.log.push(format!("CharaSelectUI.Open: {} cards, {} registered", cards.len(), registered.len()));
        self.overlays.push((level, Ui::new(2)));
        Ok(())
    }

    /// The heroine under a screen point (camera ray against her standing
    /// volume: radius 3, height 16 world units).
    fn pick_heroine(&self, x: f32, y: f32) -> Option<usize> {
        use glam::Vec3;
        let v = self.view.as_ref()?;
        let eye = Vec3::from_array(v.eye);
        let fwd = (Vec3::from_array(v.target) - eye).normalize();
        let right = fwd.cross(Vec3::from_array(v.up)).normalize();
        let up = right.cross(fwd);
        let th = (v.fov.to_radians() / 2.).tan();
        let ndc = [x / WIDTH * 2. - 1., 1. - y / HEIGHT * 2.];
        let d = (fwd + right * ndc[0] * th * WIDTH / HEIGHT + up * ndc[1] * th).normalize();
        let mut best: Option<(f32, usize)> = None;
        for (i, (_, p, _, _)) in self.spots.iter().enumerate() {
            let c = Vec3::from_array(*p);
            let (ox, oz, dx, dz) = (eye.x - c.x, eye.z - c.z, d.x, d.z);
            let a = dx * dx + dz * dz;
            let b = 2. * (ox * dx + oz * dz);
            let cc = ox * ox + oz * oz - 9.;
            let disc = b * b - 4. * a * cc;
            if a < 1e-6 || disc < 0. {
                continue;
            }
            let t = (-b - disc.sqrt()) / (2. * a);
            let h = eye.y + d.y * t - c.y;
            if t > 0. && (0. ..=16.).contains(&h) && best.is_none_or(|b| t < b.0) {
                best = Some((t, i));
            }
        }
        best.map(|b| b.1)
    }

    fn random(&mut self, n: usize) -> usize {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng % n.max(1) as u64) as usize
    }

    /// `ActionMapScene.SetCharaForMap` over `GetForPlacementTable`: the
    /// unplaced heroines in shuffled order each take the first map of a
    /// shuffled map list with fewer than `_mapLimitFull` heroines (map 0
    /// when all are full). `clear` resets every heroine first (a new zone).
    /// SATD-amanatsu-port: link partners (`isLink`) are not kept together.
    fn place_heroines(&mut self, clear: bool) {
        if clear {
            self.placement.clear();
            // `TimeZoneInternal`: every heroine's `RecoveryCost`.
            let (c, ups) = self.cost.clone();
            for (gp, _, _) in self.npc.values_mut() {
                gp.cost = c.get_max(&ups, gp.favorability.lv);
            }
        }
        let maps: Vec<i32> = self.maps.iter().map(|m| m.id).filter(|id| !self.excluded_maps.contains(id)).collect();
        let mut heroines: Vec<String> = self.heroines.iter().filter(|h| !self.placement.contains_key(*h)).cloned().collect();
        for i in (1..heroines.len()).rev() {
            let j = self.random(i + 1);
            heroines.swap(i, j);
        }
        for h in heroines {
            let mut order = maps.clone();
            for i in (1..order.len()).rev() {
                let j = self.random(i + 1);
                order.swap(i, j);
            }
            let count = |m: i32, p: &std::collections::HashMap<String, i32>| p.values().filter(|v| **v == m).count();
            let map = order.iter().copied().find(|m| count(*m, &self.placement) < self.map_limit).unwrap_or(0);
            self.log.push(format!("SetCharaForMap: {h} → map {map}"));
            self.placement.insert(h, map);
        }
    }

    /// The heroine's `NPCData` parameters (created from the card on first
    /// use: all parameters at 0, `GameParameter_AL.characteristic`).
    fn npc(&mut self, card: &str) -> &mut (crate::params::GameParameter, crate::params::Count, usize) {
        if !self.npc.contains_key(card) {
            let characteristic = self
                .unity
                .read_all(card)
                .ok()
                .and_then(|b| crate::card::Card::parse(&b).ok())
                .and_then(|c| c.block("GameParameter_AL").and_then(|b| crysirror_unity::msgpack::decode(b).ok()))
                .and_then(|v| v.get("characteristic").and_then(crysirror_unity::msgpack::Value::as_i64))
                .unwrap_or(0)
                .max(0) as usize;
            // `RecoveryCost` on entry: the full cost for favourability LV 0.
            let gp = crate::params::GameParameter { cost: self.cost.0.get_max(&self.cost.1, 0), ..Default::default() };
            self.npc.insert(card.to_owned(), (gp, Default::default(), characteristic));
        }
        self.npc.get_mut(card).expect("inserted")
    }

    /// `CommandUI.SetParameter`: gauges (`ImgGauge.fillAmount` = Point /
    /// LVPoint) and level toggles (`TglLVs[i].isOn` for i < LV).
    fn show_parameters(&mut self, card: &str) {
        let gp = self.npc(card).0.clone();
        let Some((_, ci)) = self.command else { return };
        let points = self.param_table.points.clone();
        let Some((level, _)) = self.overlays.get_mut(ci) else { return };
        let Some(ui) = level.scene.nodes.iter().flat_map(|n| &n.components).find(|c| c.script == "AL.CommandUI").map(|c| c.fields.clone()) else { return };
        // `SetCost`: `GetMax(favourability LV)` hourglasses shown, the
        // remaining cost lit.
        let max = self.cost.0.get_max(&self.cost.1, gp.favorability.lv);
        for (k, t) in ui.array("_tglCosts").unwrap_or(&[]).iter().enumerate() {
            let crysirror_unity::managed::Value::PPtr(t) = t else { continue };
            let Some(tn) = level.owner(t.path_id) else { continue };
            level.set_active(tn, (k as i32) < max);
            let graphic = level.scene.nodes[tn].components.iter().find(|c| c.script == "UnityEngine.UI.Toggle").and_then(|c| c.fields.pptr("graphic")).and_then(|p| level.owner(p.path_id));
            if let Some(g) = graphic {
                level.set_active(g, (k as i32) < gp.cost);
            }
        }
        for (i, (field, info)) in [("_favorability", gp.favorability), ("_inclusiveness", gp.inclusiveness), ("_proactivity", gp.proactivity), ("_curiosity", gp.curiosity)].iter().enumerate() {
            let Some(f) = ui.fields(field) else { continue };
            let lv_point = points.get(i).map_or(100, |p| p.lv_point).max(1);
            if let Some(n) = f.pptr("ImgGauge").and_then(|p| level.owner(p.path_id)) {
                level.states.entry(n).or_default().fill = Some((info.point as f32 / lv_point as f32).clamp(0., 1.));
            }
            for (k, t) in f.array("TglLVs").unwrap_or(&[]).iter().enumerate() {
                let crysirror_unity::managed::Value::PPtr(t) = t else { continue };
                let Some(tn) = level.owner(t.path_id) else { continue };
                let graphic = level.scene.nodes[tn].components.iter().find(|c| c.script == "UnityEngine.UI.Toggle").and_then(|c| c.fields.pptr("graphic")).and_then(|p| level.owner(p.path_id));
                if let Some(g) = graphic {
                    level.set_active(g, (k as i32) < info.lv);
                }
            }
        }
    }

    /// `EventManager.Talk` after the conversation: the command's points
    /// (`CommandUI.AddPoint`).
    fn apply_points(&mut self, card: String, kind: &str, select: Option<usize>) {
        let characteristic = self.npc(&card).2;
        let table = std::mem::take(&mut self.param_table);
        let mut rng = self.rng;
        let mut random = |n: usize| {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            (rng % n.max(1) as u64) as usize
        };
        let add = table.command_points(kind, select, characteristic, &mut random);
        self.rng = rng;
        let gp = &mut self.npc(&card).0;
        let ups = table.add_points(gp, add);
        let msg = format!(
            "AddPoint {kind} (select {select:?}): +{add:?}, {ups} level-ups → favorability LV{} {} (late {}), inclusiveness LV{} {}, proactivity LV{} {}, curiosity LV{} {}",
            gp.favorability.lv, gp.favorability.point, gp.late_point, gp.inclusiveness.lv, gp.inclusiveness.point, gp.proactivity.lv, gp.proactivity.point, gp.curiosity.lv, gp.curiosity.point
        );
        self.param_table = table;
        self.log.push(msg);
        self.show_parameters(&card);
    }

    /// The state as `SaveData`/`PlayerData`/`NPCData` (`SaveData.Save`).
    /// SATD-amanatsu-port: the player is the default male card
    /// (`_defaultMaleCardFileName`), tutorial progress and versions are 0,
    /// motion/link state is not kept.
    fn save_file(&mut self, zone: i32, comment: &str) -> crate::save::SaveFile {
        use crate::save::*;
        let (text, entry) = now_text();
        let card = |unity: &mut Unity, path: &str| unity.read_all(path).ok();
        let player_path = "DefaultData/0/chara/male/AL_M_00.png";
        let player = PlayerData { actor: Actor { chara_file_name: Some("AL_M_00".into()), unique_id: 0, human: card(&mut self.unity, player_path), version: Some(Version::default()), ..Default::default() }, is_futanari: false };
        let mut npcs = Vec::new();
        for (i, h) in self.heroines.clone().iter().enumerate() {
            let (gp, count, _) = self.npc(h).clone();
            let name = std::path::Path::new(h).file_stem().and_then(|s| s.to_str()).unwrap_or("").to_owned();
            npcs.push(NpcData {
                actor: Actor { chara_file_name: Some(name), unique_id: i as i32 + 1, human: card(&mut self.unity, h), version: Some(Version::default()), data_id: Some(h.clone()), ..Default::default() },
                action: ActionState { version: Some(Version::default()), link_partner: -1, map_id: self.placement.get(h).copied().unwrap_or(-1) },
                parameter: gp,
                count,
                count_extra: CountExtra { version: Some(Version::default()), ..Default::default() },
                save_time_text: Some(text.clone()),
                entry_time: entry,
                ..Default::default()
            });
        }
        SaveFile { save: SaveData { load_product_id: 100, version: Some(Version::default()), save_time_text: Some(text), time_zone: zone, entry_count: npcs.len() as i32, comment: Some(comment.into()), ..Default::default() }, player, npcs }
    }

    /// Restore heroines, their parameters and placement from a save (cards
    /// by `DataID` path, else `CharaFileName` among the preset cards).
    fn apply_save(&mut self, file: crate::save::SaveFile) {
        self.heroines.clear();
        self.npc.clear();
        self.placement.clear();
        for n in file.npcs {
            let name = n.actor.chara_file_name.clone().unwrap_or_default();
            let path = n.actor.data_id.clone().filter(|p| self.unity.read_all(p).is_ok()).or_else(|| (0..4).map(|d| format!("DefaultData/{d}/chara/female/{name}.png")).find(|p| self.unity.read_all(p).is_ok()));
            let Some(path) = path else {
                self.log.push(format!("save heroine {name}: card not found"));
                continue;
            };
            self.heroines.push(path.clone());
            let characteristic = self.npc(&path).2;
            self.npc.insert(path.clone(), (n.parameter, n.count, characteristic));
            if n.action.map_id >= 0 {
                self.placement.insert(path, n.action.map_id);
            }
        }
    }

    /// `SaveData.Save(isAuto: true)` after a time-zone change: the next
    /// `a_NNN.sav` in the engine's save mount (`user/`).
    /// SATD-amanatsu-port: the original's auto-save count limit/rotation
    /// beyond the next free number is not reproduced.
    fn auto_save(&mut self, host: &mut dyn HostServices, zone: i32) -> Result<()> {
        let storage = host.storage(self.client_id)?;
        let existing = storage.list(crysirror_core::path::Mount::Save, "user").unwrap_or_default();
        let next = existing.iter().filter_map(|f| f.rsplit('/').next()?.strip_prefix("a_")?.strip_suffix(".sav")?.parse::<u32>().ok()).max().map_or(0, |n| (n + 1) % 1000);
        let name = crate::save::file_name(&format!("{next:03}"), true);
        let bytes = self.save_file(zone, "オートセーブデータ").encode()?;
        let _ = storage.create_directory(&VfsPath::new(crysirror_core::path::Mount::Save, "user")?);
        storage.write_atomic(&VfsPath::new(crysirror_core::path::Mount::Save, &format!("user/{name}"))?, &bytes)?;
        self.log.push(format!("SaveData.Save(auto): user/{name} ({} bytes)", bytes.len()));
        Ok(())
    }

    /// The newest `.sav` of the engine's save mount.
    fn newest_save(&mut self, host: &mut dyn HostServices) -> Result<Option<(String, crate::save::SaveFile)>> {
        let storage = host.storage(self.client_id)?;
        let mut files: Vec<String> = storage.list(crysirror_core::path::Mount::Save, "user").unwrap_or_default().into_iter().filter(|f| f.ends_with(".sav")).collect();
        let mut best: Option<(String, crate::save::SaveFile, i64)> = None;
        files.sort();
        for f in files {
            let rel = if f.starts_with("user/") { f.clone() } else { format!("user/{f}") };
            let Ok(bytes) = storage.read(&VfsPath::new(crysirror_core::path::Mount::Save, &rel)?) else { continue };
            match crate::save::SaveFile::decode(&bytes) {
                Ok(file) => {
                    let t = file.npcs.first().map_or(0, |n| n.entry_time);
                    if best.as_ref().is_none_or(|b| t >= b.2) {
                        best = Some((rel, file, t));
                    }
                }
                Err(e) => self.log.push(format!("save {rel}: {e:#}")),
            }
        }
        Ok(best.map(|b| (b.0, b.1)))
    }

    fn set_hud_visible(&mut self, on: bool) {
        if self.hud.is_some() {
            if let Some((level, _)) = self.overlays.first_mut() {
                for r in level.scene.roots.clone() {
                    level.set_active(r, on);
                }
            }
        }
    }

    /// The massage heroine on the esthetics bed (`Est00`) in the first
    /// oil pose (`AnimationInfo` 0, its controller's default state), kept
    /// as a CPU model for the touch raycast.
    fn spawn_oil_heroine(&mut self, host: &mut dyn HostServices, client: ClientId, card: &str, pos: [f32; 3], rot: [f32; 4], sun: f32) -> Result<()> {
        if self.oil.is_none() {
            self.oil = Some(crate::oil::OilData::load(&mut self.unity)?);
        }
        let pose = self.oil.as_ref().and_then(|o| o.poses.first().cloned()).context("oil poses")?;
        let bundle = format!("lib/{}", pose.anime.0);
        // The controller's default state (`Pose_Base_Loop_00`).
        let state = {
            use crysirror_unity::animator::Controller;
            let (k, id) = self.unity.bundle_asset(&bundle, &pose.anime.1, 91)?.context("oil controller")?;
            let ctl = Controller::decode(&self.unity.decode(&k, id)?)?;
            let l = &ctl.layers[0];
            ctl.names.get(&l.states[l.default_state].name_id).cloned().context("oil default state")?
        };
        let q = glam::Quat::from_array(crysirror_unity::gltf::mirror_quat(rot));
        let tr = glam::Vec3::from_array(crysirror_unity::gltf::mirror_point(pos));
        let (x, y, z) = q.to_euler(glam::EulerRot::XYZ);
        let t = json!({"translation": tr.to_array(), "rotation_degrees": [x.to_degrees(), y.to_degrees(), z.to_degrees()], "scale": [1.0, 1.0, 1.0]});
        let chunks = self.spawn_character_motion(host, client, card, Some(t), sun, Some((&bundle, &pose.anime.1, &state)))?;
        let model = chunks.first().and_then(|g| crysirror_asset::pbr::load_pbr(g, |_| anyhow::bail!("external resource")).ok());
        let (personality, name) = self.unity.read_all(card).ok().and_then(|b| crate::card::Card::parse(&b).ok()).and_then(|c| c.parameter()).map_or((0, String::new()), |p| (p.0, p.1));
        self.spots.push((card.to_owned(), tr.to_array(), personality, name));
        if let (Some(m), Some(oil)) = (self.massage.as_mut(), self.oil.as_ref()) {
            m.state = crate::oil::Massage::new(oil);
            m.model = model;
            m.placement = glam::Mat4::from_rotation_translation(q, tr);
            m.started = self.now;
        }
        self.log.push(format!("OilScene: {card} on Est00 in {} ({state})", pose.name));
        Ok(())
    }

    /// `OilScene.Create`: the `OilMassageScene` UI over the bed, its
    /// camera placed at `Est00`, then the massage start scenario.
    fn enter_oil_scene(&mut self, host: &mut dyn HostServices, client: ClientId, now: u64) -> Result<()> {
        let _ = (host, client);
        let mut level = Level::prefab(&mut self.unity, crate::oil::SCENE_BUNDLE, crate::oil::SCENE_PREFAB)?;
        // `OilCameraController.SetTarget`: the pose's camera data around
        // the character root on the bed.
        let pose_camera = self.oil.as_ref().and_then(|o| o.poses.first()).map(|p| p.camera.clone()).unwrap_or_default();
        if let (Ok(cam), Some(m)) = (crate::oil::CameraData::load(&mut self.unity, &pose_camera), self.massage.as_ref()) {
            let mirror = glam::Mat4::from_scale(glam::Vec3::new(-1., 1., 1.));
            let (eye, target, up) = cam.eye(mirror * m.placement * mirror);
            let v = map::View {
                eye: crysirror_unity::gltf::mirror_point(eye.to_array()),
                target: crysirror_unity::gltf::mirror_point(target.to_array()),
                up: crysirror_unity::gltf::mirror_point(up.to_array()),
                fov: cam.fov,
                near: 0.1,
                far: 1000.,
            };
            self.world.push(v.command());
            self.view = Some(v);
        }
        // The setting panels start closed.
        for path in ["OilMassageScene/Canvas/ccr_dropShadow/Normal/RightTopArea/ClothesSetting/root", "OilMassageScene/Canvas/ccr_dropShadow/Normal/RightBottomArea/LightSetting/root", "OilMassageScene/Canvas/ccr_dropShadow/Normal/RightBottomArea/DragArea"] {
            if let Some(n) = level.find(path) {
                level.set_active(n, false);
            }
        }
        for path in ["OilMassageScene/Canvas/ccr_dropShadow/Hide", "OilMassageScene/Canvas/ccr_dropShadow/Normal/Posture Root", "OilMassageScene/Canvas/ccr_dropShadow/Normal/Button Go to H"] {
            if let Some(n) = level.find(path) {
                level.set_active(n, false);
            }
        }
        self.overlays.push((level, Ui::new(6)));
        self.update_oil_gauges();
        let fav = self.massage.as_ref().map(|m| m.card.clone()).map(|c| self.npc(&c).0.favorability.lv).unwrap_or(0);
        let mut rng = self.rng;
        let mut random = |n: usize| game_random(&mut rng, n);
        let list: Vec<String> = crate::commands::choose(&self.adv_assets.massage_before(fav), &mut random).into_iter().collect();
        self.rng = rng;
        if let Some(m) = self.massage.as_mut() {
            m.phase = MassagePhase::Before;
            m.started = now;
        }
        if list.is_empty() {
            if let Some(m) = self.massage.as_mut() {
                m.phase = MassagePhase::Play;
            }
        } else {
            self.start_talk(0, list)?;
        }
        Ok(())
    }

    /// The oil overlay (the last overlay while a massage plays).
    fn oil_overlay(&self) -> Option<usize> {
        self.massage.as_ref()?;
        self.overlays.iter().position(|(l, _)| l.name == crate::oil::SCENE_PREFAB)
    }

    /// `UIController._sliderSatisfaction` from the gauge.
    fn update_oil_gauges(&mut self) {
        let (Some(oi), Some(m), Some(oil)) = (self.oil_overlay(), self.massage.as_ref(), self.oil.as_ref()) else { return };
        let value = m.state.satisfaction(oil);
        let level = &mut self.overlays[oi].0;
        if let Some(n) = level.find("OilMassageScene/Canvas/ccr_dropShadow/Normal/Image Gauge Background/Slider Satisfaction") {
            if let Some(f) = level.scene.nodes[n].components.iter().find(|c| c.script == "UnityEngine.UI.Slider").map(|c| c.fields.clone()) {
                crysirror_unity::ugui::slider_value(&level.scene, &f, value, &mut level.states);
            }
        }
    }

    /// `MouseTouch.Update` while dragging: the touched hit object (posed
    /// on the CPU from the playing clip) gains satisfaction.
    /// SATD-amanatsu-port: `GetMouseMove` is read as the Unity mouse axis
    /// (pixels × 0.1) and the rate applied per second of rubbing.
    fn massage_touch(&mut self, x: f32, y: f32, down: bool, now: u64) {
        let Some(m) = self.massage.as_mut() else { return };
        if m.phase != MassagePhase::Play {
            return;
        }
        let prev = m.last_pointer.replace((x, y));
        let dt = (now.saturating_sub(m.last_ms) as f32 / 1000.).min(0.1);
        m.last_ms = now;
        let (Some((px, py)), true) = (prev, down) else { return };
        let moved = ((x - px).hypot(y - py)) * 0.1;
        if moved <= 0. {
            return;
        }
        let (Some(model), Some(oil), Some(v)) = (m.model.as_ref(), self.oil.as_ref(), self.view.as_ref()) else { return };
        use glam::Vec3;
        let eye = Vec3::from_array(v.eye);
        let fwd = (Vec3::from_array(v.target) - eye).normalize();
        let right = fwd.cross(Vec3::from_array(v.up)).normalize();
        let up = right.cross(fwd);
        let th = (v.fov.to_radians() / 2.).tan();
        let ndc = [x / WIDTH * 2. - 1., 1. - y / HEIGHT * 2.];
        let dir = (fwd + right * ndc[0] * th * WIDTH / HEIGHT + up * ndc[1] * th).normalize();
        let Some(clip) = model.animations.first() else { return };
        let t = ((now.saturating_sub(m.started)) as f32 / 1000.) % clip.duration.max(1e-3);
        let mut local: Vec<crysirror_asset::pbr::Trs> = model.nodes.iter().map(|n| n.rest).collect();
        crysirror_asset::anim::sample_clip(model, clip, t, &mut local);
        let global = crysirror_asset::anim::global_pose(model, &local);
        let placement = m.placement;
        let bone = |name: &str| model.nodes.iter().position(|n| n.name == name).map(|i| placement * global[i]);
        if let Some(hit) = crate::oil::raycast(&oil.volumes, eye, dir, &bone) {
            let tag = hit.tag.clone();
            // Rate per second: the per-frame rule scaled to 60 Hz.
            if m.state.touch(oil, &tag, moved * dt * 60.) {
                self.update_oil_gauges();
            }
        }
    }

    /// A conversation ended during a massage: advance its phase.
    fn massage_talk_ended(&mut self, now: u64) -> Result<()> {
        let Some(phase) = self.massage.as_ref().map(|m| m.phase) else { return Ok(()) };
        match phase {
            MassagePhase::Agree => {
                // `StartProc` → fade, `ChangeMapAsync(Esthetics)`.
                if let Some((_, ci)) = self.command.take() {
                    if ci < self.overlays.len() {
                        let (_, mut ui) = self.overlays.remove(ci);
                        self.removed.extend(ui.clear());
                    }
                }
                let zone = if let Controller::ActionMap(m) = &self.controller { m.time_zone } else { 0 };
                let esthetics = self.excluded_maps.first().copied().unwrap_or(10);
                if let Some(m) = self.massage.as_mut() {
                    m.phase = MassagePhase::Enter;
                }
                self.fade.start(now, true, self.fade_seconds);
                self.pending_map = Some((esthetics, zone));
                self.log.push(format!("EventManager.Massage → map {esthetics}"));
            }
            MassagePhase::Before => {
                if let Some(m) = self.massage.as_mut() {
                    m.phase = MassagePhase::Play;
                    m.last_pointer = None;
                }
            }
            MassagePhase::Evaluation => {
                // Points by the evaluation, `Count.Massage`, cost, back to the map.
                let Some(m) = self.massage.take() else { return Ok(()) };
                self.apply_points(m.card.clone(), "Massage", Some(m.stage));
                {
                    let (gp, count, _) = self.npc(&m.card);
                    count.massage += 1;
                    gp.cost = (gp.cost - 1).max(0);
                }
                if let Some(oi) = self.overlays.iter().position(|(l, _)| l.name == crate::oil::SCENE_PREFAB) {
                    let (_, mut ui) = self.overlays.remove(oi);
                    self.removed.extend(ui.clear());
                }
                let zone = if let Controller::ActionMap(c) = &self.controller { c.time_zone } else { 0 };
                self.fade.start(now, true, self.fade_seconds);
                self.pending_map = Some((m.return_map, zone));
                let massages = self.npc(&m.card).1.massage;
                self.log.push(format!("massage done: stage {}, massages {massages}", m.stage));
            }
            _ => {}
        }
        Ok(())
    }

    /// `UIController` Exit: the result and its evaluation scenario.
    fn massage_exit(&mut self) -> Result<()> {
        let (Some(m), Some(oil)) = (self.massage.as_mut(), self.oil.as_ref()) else { return Ok(()) };
        if m.phase != MassagePhase::Play {
            return Ok(());
        }
        m.stage = m.state.stage(oil);
        m.phase = MassagePhase::Evaluation;
        let stage = m.stage;
        let value = m.state.satisfaction(oil);
        let c = self.adv_assets.massage_evaluation(stage);
        let mut rng = self.rng;
        let mut random = |n: usize| game_random(&mut rng, n);
        let list: Vec<String> = crate::commands::choose(&c, &mut random).into_iter().collect();
        self.rng = rng;
        self.log.push(format!("OilScene end: satisfaction {value:.1}, stage {stage}, {list:?}"));
        self.start_talk(0, list)
    }

    /// `ActionMapUI.Create/OpenExe` over the loaded map (overlay 0).
    fn open_hud(&mut self, map_id: i32, zone: i32) -> Result<()> {
        let (mut level, hud) = crate::hud::Hud::open(&mut self.unity)?;
        let name = self.maps.iter().find(|m| m.id == map_id).map(|m| m.name.clone()).unwrap_or_default();
        hud.set_map_name(&mut level, &name);
        crate::hud::set_image_sprites(&mut level, zone.clamp(0, 3) as usize);
        self.overlays.insert(0, (level, Ui::new(5)));
        self.hud = Some(hud);
        Ok(())
    }

    /// A HUD button (`ActionMapUI.Initialize` handlers).
    fn hud_action(&mut self, action: crate::hud::HudAction, now: u64) -> Result<()> {
        use crate::hud::HudAction as H;
        let Controller::ActionMap(m) = &mut self.controller else { return Ok(()) };
        let actionmap::Phase::Map(map_id) = m.phase else { return Ok(()) };
        match action {
            H::CameraPrev | H::CameraNext | H::CameraReset if !self.cameras.is_empty() => {
                let n = self.cameras.len();
                self.camera = match action {
                    H::CameraPrev => (self.camera + n - 1) % n,
                    H::CameraNext => (self.camera + 1) % n,
                    _ => self.default_camera,
                };
                self.level.sounds.push(sound::Call::SystemSe(self.sound_ids.ok_s));
                let v = self.cameras[self.camera].clone();
                self.world.push(v.command());
                self.view = Some(v);
            }
            H::Map2D => {
                // `CloseExeAsync`, then `MapSelectUI.Create/OpenExe` over the map.
                self.level.sounds.push(sound::Call::SystemSe(self.sound_ids.ok_s));
                if self.overlays.len() == 1 {
                    let (_, mut ui) = self.overlays.remove(0);
                    self.removed.extend(ui.clear());
                    self.hud = None;
                    let zone = m.time_zone;
                    let mut select = Level::prefab(&mut self.unity, actionmap::UI_BUNDLE, "MapSelectUI")?;
                    let mut c = ActionMap::new(&mut select, &self.maps);
                    c.time_zone = zone;
                    c.map = Some(map_id);
                    crate::hud::set_image_sprites(&mut select, zone.clamp(0, 3) as usize);
                    self.overlays.push((select, Ui::new(1)));
                    self.controller = Controller::ActionMap(c);
                }
            }
            H::NextTime => {
                // `EventManager.TimeZone`: SE, fade, `Cycle.IncrementTimeZone`,
                // `TimeZoneInternal` (the map again in the new zone).
                // SATD-amanatsu-port: night events, heroine placement tables,
                // BGM per zone and the auto save are not ported.
                self.level.sounds.push(sound::Call::SystemSe(self.sound_ids.time_zone_se));
                m.time_zone = (m.time_zone + 1) % 4;
                self.fade.start(now, true, self.fade_seconds);
                let zone = m.time_zone;
                self.place_heroines(true);
                self.pending_map = Some((map_id, zone));
                self.log.push(format!("EventManager.TimeZone → zone {zone}"));
                self.pending_auto_save = Some(zone);
            }
            H::Config | H::Help | H::CameraPrev | H::CameraNext | H::CameraReset => self.log.push(format!("SATD-amanatsu-port: ActionMapUI {action:?} not ported")),
        }
        Ok(())
    }

    /// A click on a standing heroine: the first meeting, else the command
    /// menu (`CommandScene.OpenExe`) with her greeting.
    fn heroine_clicked(&mut self, spot: usize) -> Result<()> {
        let (card, _, personality, _) = self.spots[spot].clone();
        let index = crate::talk::scenario_index(&mut self.unity, personality)?;
        if self.met.insert(card) {
            let list = crate::talk::first_meeting(&index, self.now);
            return self.start_talk(spot, list);
        }
        let mut level = Level::prefab(&mut self.unity, "lib/am/pre/s/000_00.unity3d", "command")?;
        for path in ["Command/Canvas/Hide", "Command/Canvas/Panel/WaterGun", "Command/Canvas/Panel/Command/WithPlay"] {
            if let Some(n) = level.find(path) {
                level.set_active(n, false);
            }
        }
        if let Some(p) = level.find("Command/Canvas/Panel") {
            crate::talk::show(&mut level, p);
        }
        self.overlays.push((level, Ui::new(4)));
        self.command = Some((spot, self.overlays.len() - 1));
        let card = self.spots[spot].0.clone();
        self.show_parameters(&card);
        self.visit_played = false;
        // `Map3DToCommandScene`: the map HUD closes while the menu is open.
        self.set_hud_visible(false);
        // `GetPreludeAsset`: a greeting voice.
        if let Some(prelude) = crate::talk::pick(&index, "prelude", self.now) {
            self.start_talk(spot, vec![prelude])?;
        }
        Ok(())
    }

    /// Start a conversation with a standing heroine.
    fn start_talk(&mut self, spot: usize, scenarios: Vec<String>) -> Result<()> {
        let (_, _, personality, name) = self.spots[spot].clone();
        let key = self.unity.bundle_files().into_iter().find(|k| k.starts_with(&format!("bundle:{}#", crate::talk::ADV_BUNDLE)) && !k.ends_with(".resS")).context("ADV bundle")?;
        let mut level = Level::root_of(&mut self.unity, &key, "ADVCore")?;
        let mut talk = crate::talk::Talk::start(&mut self.unity, &mut level, personality, &name, self.now, scenarios)?;
        let steps = talk.advance(&mut self.unity, &mut level, self.now);
        self.overlays.push((level, Ui::new(3)));
        let index = self.overlays.len() - 1;
        self.log.push(format!("talk with {name} (c{personality:02}): {}", talk.program.name));
        self.talk = Some((talk, index));
        self.pending_steps = steps;
        Ok(())
    }

    fn release_map(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<()> {
        for id in std::mem::take(&mut self.map_lights) {
            self.world.push(json!({"op": "despawn", "id": id}));
        }
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
            zone_bgm: ["Morning", "Afternoon", "EarlyEvening", "Evening"].map(|k| pd.fields("_bgmID").and_then(|b| b.int(k)).unwrap_or(2) as i32),
            time_zone_se: sse.int("TimeZoneChange").unwrap_or(6) as i32,
        };
        // `MapManager.GetForPlacementTable`: maps other than the esthetics
        // and H maps take up to `_mapLimitFull` heroines each.
        let map_ids = pd.fields("_mapID");
        let excluded_maps: Vec<i32> = ["EstheticsMap", "HMaps"]
            .iter()
            .flat_map(|k| map_ids.and_then(|m| m.array(k)).unwrap_or(&[]).iter().filter_map(|v| if let crysirror_unity::managed::Value::Int(i) = v { Some(*i as i32) } else { None }))
            .collect();
        let map_limit = pd.int("_mapLimitFull").unwrap_or(6).max(1) as usize;
        let (adv_assets, adv_maps) = crate::commands::AdvAssets::from_property_data(&pd.fields("_adv").cloned().unwrap_or_default(), pd.fields("_mapID"));
        let mut game = Game {
            sound,
            sound_ids,
            rng: host.now_ms() | 1,
            placement: std::collections::HashMap::new(),
            excluded_maps,
            map_limit,
            param_table: crate::params::ParamTable::from_property_data(&pd.fields("_param").cloned().unwrap_or_default()),
            npc: std::collections::HashMap::new(),
            pending_points: None,
            loaded_save: None,
            oil: None,
            massage: None,
            adv_assets,
            adv_maps,
            cost: crate::commands::CostProperty::from_property_data(pd.fields("_explore")),
            visit_played: false,
            pending_auto_save: None,
            client_id: self.client,
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
            map_lights: Vec::new(),
            sky: None,
            next_model: 0,
            view: None,
            cameras: Vec::new(),
            camera: 0,
            default_camera: 0,
            hud: None,
            pending_steps: Vec::new(),
            heroines: Vec::new(),
            chara_select: None,
            spots: Vec::new(),
            met: std::collections::HashSet::new(),
            command: None,
            talk: None,
            lists: None,
            thumbnails: std::collections::HashMap::new(),
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
                if name == "CustomScene" {
                    game.custom_scene(host, self.client)?;
                }
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
                game.hud = None;
                game.change_map(host, client, id, zone)?;
                if game.massage.as_ref().is_some_and(|m| m.phase == MassagePhase::Enter) {
                    game.enter_oil_scene(host, client, now)?;
                } else {
                    game.open_hud(id, zone)?;
                }
                // `PlayBGMBYActionMap`.
                let bgm = game.sound_ids.zone_bgm[zone.clamp(0, 3) as usize];
                game.level.sounds.push(sound::Call::Bgm(bgm));
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
        if game.massage.is_some() && game.talk.is_none() {
            let down = input.buttons_down & b::MOUSE_LEFT != 0 && handler.is_none() && !fading;
            game.massage_touch(x, y, down, now);
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
        // A conversation takes the clicks; timed waits run every frame.
        if let Some((talk, oi)) = game.talk.as_mut() {
            let level = &mut game.overlays[*oi].0;
            let mut steps = talk.tick(&mut game.unity, level, now);
            if input.buttons_pressed & b::MOUSE_LEFT != 0 && !fading {
                let node = handler.filter(|(h, _)| *h == *oi + 1).map(|(_, n)| n);
                steps.extend(talk.click(&mut game.unity, level, node, now));
            }
            game.pending_steps.extend(steps);
            click = None;
            game.pressed = None;
        } else if input.buttons_pressed & b::MOUSE_LEFT != 0 && handler.is_none() && !fading && game.massage.is_none() && matches!(&game.controller, Controller::ActionMap(m) if matches!(m.phase, actionmap::Phase::Map(_))) {
            let hit_ui = layouts.iter().any(|l| l.hits.iter().any(|(_, r)| r.contains(x, y)));
            if !hit_ui {
                if let Some(spot) = game.pick_heroine(x, y).filter(|_| game.command.is_none()) {
                    if let Err(e) = game.heroine_clicked(spot) {
                        game.log.push(format!("talk: {e:#}"));
                    }
                }
            }
        }
        let mut massage_talk_ended = false;
        for step in std::mem::take(&mut game.pending_steps) {
            match step {
                crate::talk::Step::Voice(bundle, asset) => {
                    if let Err(e) = game.sound.play(host, client, &mut game.unity, sound::Kind::Voice, &bundle, &asset, 1.) {
                        game.log.push(format!("voice {asset}: {e:#}"));
                    }
                }
                crate::talk::Step::StopVoice => {}
                crate::talk::Step::End => {
                    if let Some((talk, oi)) = game.talk.take() {
                        game.log.extend(talk.log);
                        if let Some((card, kind)) = game.pending_points.take() {
                            game.apply_points(card, kind, talk.selected);
                        }
                        massage_talk_ended = true;
                        if oi < game.overlays.len() {
                            let (_, mut ui) = game.overlays.remove(oi);
                            game.removed.extend(ui.clear());
                        }
                        game.log.push("talk ended".into());
                        if let Some((_, ci)) = game.command {
                            if let Some((l, _)) = game.overlays.get_mut(ci) {
                                if let Some(p) = l.find("Command/Canvas/Panel") {
                                    crate::talk::show(l, p);
                                }
                            }
                        }
                    }
                }
                crate::talk::Step::None => {}
            }
        }
        if let Some((talk, _)) = game.talk.as_mut() {
            game.log.append(&mut talk.log);
        }
        let mut open_chara_select = false;
        let mut close_chara_select = false;
        let mut command_click: Option<(usize, usize, String)> = None;
        let mut hud_click: Option<crate::hud::HudAction> = None;
        let mut replace_heroines = false;
        let mut massage_exit = false;
        if let Some((h, button)) = click {
            if h > 0 {
                let level = &mut game.overlays[h - 1].0;
                game.log.push(format!("click {}", level.scene.path(button)));
                let calls = level.on_click(button);
                let script = level.invoke(&calls);
                if game.massage.is_some() && level.name == crate::oil::SCENE_PREFAB && level.scene.nodes[button].name == "Button Exit" {
                    massage_exit = true;
                }
                if h == 1 && game.command.is_none() && game.talk.is_none() {
                    if let Some(a) = game.hud.as_ref().and_then(|hud| hud.action(button)) {
                        hud_click = Some(a);
                    }
                }
                if let Some((spot, ci)) = game.command {
                    if h == ci + 1 {
                        let name = level.scene.nodes[button].name.clone();
                        command_click = Some((spot, ci, name));
                    }
                }
                if h == 2 && game.command.is_none() {
                    if let Some(cs) = game.chara_select.as_mut() {
                        let mut entered = None;
                        let out = cs.click(level, button, &mut |card| {
                            entered = Some(card.to_owned());
                            game.thumbnails.get(card).cloned()
                        });
                        if let Some(card) = entered {
                            if !game.heroines.contains(&card) {
                                game.heroines.push(card.clone());
                                game.log.push(format!("heroine entered: {card}"));
                            }
                        }
                        if let crate::charaselect::Outcome::Close = out {
                            close_chara_select = true;
                        }
                    }
                }
                match &mut game.controller {
                    Controller::ActionMap(m) if h == 1 && game.hud.is_none() => match m.click(level, button) {
                        actionmap::Action::ChangeMap { id, time_zone } => {
                            game.level.sounds.push(sound::Call::SystemSe(game.sound_ids.ok_l));
                            game.fade.start(now, true, game.fade_seconds);
                            game.pending_map = Some((id, time_zone));
                            game.log.push(format!("MapSelectCallBack → ChangeMap({id}, {time_zone})"));
                        }
                        actionmap::Action::Scene(r) => request = r,
                        actionmap::Action::OpenCharaSelect => open_chara_select = true,
                        actionmap::Action::NextTimeZone(zone) => {
                            // `MapSelectUI.ChangeTimeZone` after `EventManager.TimeZone`.
                            game.level.sounds.push(sound::Call::SystemSe(game.sound_ids.time_zone_se));
                            let n = crate::hud::set_image_sprites(level, zone.clamp(0, 3) as usize);
                            replace_heroines = true;
                            game.log.push(format!("EventManager.TimeZone → zone {zone} ({n} sprites)"));
                        }
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
        if replace_heroines {
            game.place_heroines(true);
        }
        if massage_talk_ended {
            game.massage_talk_ended(now)?;
        }
        if massage_exit {
            game.massage_exit()?;
        }
        if let Some(zone) = game.pending_auto_save.take() {
            if let Err(e) = game.auto_save(host, zone) {
                game.log.push(format!("SaveData.Save: {e:#}"));
            }
        }
        if let Some(a) = hud_click {
            game.log.push(format!("ActionMapUI {a:?}"));
            game.hud_action(a, now)?;
        }
        if let Some((spot, ci, name)) = command_click.take() {
            let personality = game.spots.get(spot).map_or(0, |s| s.2);
            let index = crate::talk::scenario_index(&mut game.unity, personality)?;
            let end = name == "btnEnd";
            let mut list = Vec::new();
            let card = game.spots.get(spot).map(|s| s.0.clone()).unwrap_or_default();
            let (fav_lv, cur_lv, cost) = {
                let gp = &game.npc(&card).0;
                (gp.favorability.lv, gp.curiosity.lv, gp.cost)
            };
            let kind = match name.as_str() {
                "btnTalk" => Some("Talk"),
                "btnListen" => Some("Listen"),
                "btnHTopic" => Some("HTopic"),
                _ => None,
            };
            if end {
                // `EndProc`: the end asset of the visit.
                let c = game.adv_assets.end(game.visit_played, cost <= 0 && game.visit_played, fav_lv);
                let mut random = |n: usize| game_random(&mut game.rng, n);
                list.extend(crate::commands::choose(&c, &mut random));
            } else if let Some(kind) = kind {
                // `EventManager.Talk`: the agree line, then (with cost) the
                // command's asset, points and `ConsumptionCost`.
                let exhausted = cost <= 0;
                let zone = if let Controller::ActionMap(m) = &game.controller { m.time_zone } else { 0 };
                let map_key = if let Controller::ActionMap(m) = &game.controller { if let actionmap::Phase::Map(id) = m.phase { game.adv_maps.get(&id).copied() } else { None } } else { None };
                let ctx = crate::commands::Context { favorability_lv: fav_lv, map_key, zone };
                let mut rng = game.rng;
                let mut random = |n: usize| game_random(&mut rng, n);
                list.extend(crate::commands::choose(&game.adv_assets.agree(kind, fav_lv, exhausted), &mut random));
                if !exhausted {
                    let c = match kind {
                        "Talk" => game.adv_assets.talk("_talk", &ctx),
                        "Listen" => game.adv_assets.talk("_listen", &ctx),
                        _ => game.adv_assets.h_topic(random(100) < 50, fav_lv, cur_lv),
                    };
                    list.extend(crate::commands::choose(&c, &mut random));
                    game.pending_points = Some((card.clone(), kind));
                    let gp = &mut game.npc(&card).0;
                    gp.cost = (gp.cost - 1).max(0);
                }
                game.rng = rng;
                game.visit_played = true;
                game.show_parameters(&card);
            } else if name == "btnMassage" {
                // `EventManager.Massage`: the agree line, then the oil scene
                // on the esthetics map.
                let exhausted = cost <= 0;
                let mut rng = game.rng;
                let mut random = |n: usize| game_random(&mut rng, n);
                list.extend(crate::commands::choose(&game.adv_assets.agree("Massage", fav_lv, exhausted), &mut random));
                game.rng = rng;
                game.visit_played = true;
                let return_map = if let Controller::ActionMap(m) = &game.controller { if let actionmap::Phase::Map(id) = m.phase { id } else { 0 } } else { 0 };
                if !exhausted {
                    game.massage = Some(MassageRun { card: card.clone(), phase: MassagePhase::Agree, return_map, state: Default::default(), model: None, placement: glam::Mat4::IDENTITY, started: now, last_pointer: None, last_ms: now, stage: 0 });
                }
            } else if let Some(family) = crate::talk::command_family(&name) {
                list.extend(crate::talk::pick(&index, "agree", now));
                if family != "agree" {
                    list.extend(crate::talk::pick(&index, family, now));
                }
                if family == "msg" {
                    list.extend(crate::talk::pick(&index, "msge", now));
                }
                if family == "with" {
                    list.extend(crate::talk::pick(&index, "wend", now));
                }
                game.visit_played = true;
                if matches!(name.as_str(), "btnInvite" | "btnH" | "btnMassage") {
                    game.log.push(format!("SATD-amanatsu-port: {name} scene (invite/massage/H) not ported"));
                }
            } else {
                game.log.push(format!("SATD-amanatsu-port: command {name} not ported"));
            }
            if end {
                // `CommandSceneToMap`: the menu closes.
                if ci < game.overlays.len() {
                    let (_, mut ui) = game.overlays.remove(ci);
                    game.removed.extend(ui.clear());
                }
                game.command = None;
                game.set_hud_visible(true);
            } else if let Some(p) = game.overlays.get_mut(ci).and_then(|(l, _)| l.find("Command/Canvas/Panel").map(|p| (l, p))) {
                // Hidden while the event plays.
                p.0.set_active(p.1, false);
            }
            if !list.is_empty() && game.talk.is_none() {
                game.log.push(format!("command {name}: {list:?}"));
                if let Err(e) = game.start_talk(spot, list) {
                    game.log.push(format!("talk: {e:#}"));
                }
            }
        }
        if close_chara_select && game.overlays.len() == 2 {
            if let Some((_, mut ui)) = game.overlays.pop() {
                game.removed.extend(ui.clear());
            }
            game.chara_select = None;
            game.log.push("CharaSelectUI.Close".into());
        }
        if open_chara_select && game.overlays.len() == 1 {
            game.open_chara_select(host, client)?;
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
            scenes::Request::LoadSave if game.pending.is_none() => match game.newest_save(host) {
                Ok(Some((name, file))) => {
                    game.log.push(format!("SaveData.Load({name}): zone {}, {} heroines", file.save.time_zone, file.npcs.len()));
                    game.loaded_save = Some(file);
                    game.fade.start(now, true, game.fade_seconds);
                    game.pending = Some((scenes::scene_id::ACTION_MAP, now));
                }
                Ok(None) => game.log.push("SaveLoadUI: no save data".into()),
                Err(e) => game.log.push(format!("SaveData.Load: {e:#}")),
            },
            scenes::Request::LoadSave => {}
        }
        if game.quit {
            return Ok(VmYield::Halted);
        }
        Ok(VmYield::WaitUntil(game.now + 16))
    }
}

fn game_random(rng: &mut u64, n: usize) -> usize {
    *rng ^= *rng << 13;
    *rng ^= *rng >> 7;
    *rng ^= *rng << 17;
    (*rng % n.max(1) as u64) as usize
}

/// `SaveData.UpdateSaveTime`: the local time as `yyyy/MM/dd HH:mm` and as
/// a .NET `DateTime` binary (ticks since 0001-01-01, kind Local).
fn now_text() -> (String, i64) {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    let ticks = (secs + 62_135_596_800) * 10_000_000;
    (format!("{y:04}/{m:02}/{d:02} {:02}:{:02}", rem / 3600, rem % 3600 / 60), ticks | (2i64 << 62))
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

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

/// An H scene in progress (`EventManager.H` + `AL.H.HScene`).
struct HRun {
    card: String,
    phase: MassagePhase,
    return_map: i32,
    posture: Option<crate::h::Posture>,
    /// Entities of the heroine and the male.
    entities: Vec<u32>,
    /// The playing motion (`Idle`, `WLoop1`, `SLoop1`, `OrgasmF`).
    motion: &'static str,
    /// When a one-shot motion ends.
    motion_until: u64,
    /// `Gauge Female` (0..100).
    gauge: f32,
    /// `Gauge Male` (0..100).
    male_gauge: f32,
    /// The finish states the posture's heroine animator has.
    finishes: Vec<String>,
    orgasms: u32,
    last_ms: u64,
    /// A night event H (`HDetail` from `NightEvent`): the points kind
    /// after it (`H_NightShowerAfter`, `H_NightMorningAfter`).
    night: Option<&'static str>,
}

/// A short movie (`EventManager.EventShortMovie` / `ShortMovie.PlayAsync`).
struct MoviePlay {
    movie: crate::sm::Movie,
    card: String,
    /// The movie's map is loading (false: playing).
    entering: bool,
    start_ms: u64,
    /// The short movie point (Unity space).
    base: Option<([f32; 3], [f32; 4])>,
    cameras: std::collections::HashMap<String, crysirror_unity::animator::Clip>,
    entities: Vec<u32>,
    heroine_clip: Option<usize>,
    fades: Vec<(f64, bool, f64, [f32; 3])>,
    /// The ADV after the movie (`endADVAsset`).
    after: Vec<String>,
}

/// `EventManager.NightEvent` phases.
#[derive(Clone, Copy, Debug, PartialEq)]
enum NightPhase {
    /// `Before_Night_(NO_)Cottage`: the invitation (choice 1 declines).
    Invite,
    /// `Before_Night_(NO_)Cottage_ShortMovie`: after the shower movie.
    Shower,
    /// `Night_H_After`.
    HAfter,
    /// `Night`: the next morning loads.
    NewDay,
    /// `Night_Morning` (`morning`, `Judge` by the event id).
    Morning,
    /// `_h.Event_After` after a morning H.
    MorningHAfter,
    /// `Greetings_ShortMovie` after the morning movie.
    Greetings,
}

struct NightRun {
    card: String,
    phase: NightPhase,
    cottage: bool,
}

/// The H motions baked into the characters.
/// The H motions baked into the characters (each posture has the
/// finishes of its category: `OrgasmF`, `OrgasmM_IN`, `OrgasmM_OUT`).
const H_MOTIONS: [&str; 6] = ["Idle", "WLoop1", "SLoop1", "OrgasmF", "OrgasmM_IN", "OrgasmM_OUT"];

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
    /// The H scene.
    h: Option<HRun>,
    /// The H postures (loaded on the first H).
    h_postures: Option<Vec<crate::h::Posture>>,
    /// A "一緒に遊ぶ" waiting for its scenarios to end (card).
    pending_with: Option<String>,
    /// The loaded map's H place (`H/H00`: position, rotation).
    h_point: Option<([f32; 3], [f32; 4])>,
    /// TMP font registrations of every hierarchy.
    font_cache: crate::ui::FontCache,
    /// The night event.
    night: Option<NightRun>,
    /// A night event heroine waiting to stand at the night event point.
    pending_night: Option<String>,
    /// `NPCData.Count` members beyond the gates (`IsH`, `IsNightH`, …).
    npc_extra: std::collections::HashMap<String, crate::save::CountExtra>,
    /// The loaded map's action points and key light intensity.
    map_points: Vec<(String, [f32; 3], [f32; 4])>,
    map_sun: f32,
    /// The last conversation's choice (`SelectNum`) and `IsHScene`.
    last_talk: (Option<usize>, bool),
    /// `ExploreProperties.Event._nightEventRate` (percent).
    night_rate: i64,
    /// `_nightEventShowerSMID`/`_nightEventSMID` movies' maps
    /// (`al_short_movie` `Map`).
    night_movie_maps: (i32, i32),
    /// `_nightEventShowerSMID`, `_nightEventSMID`.
    night_movie_ids: (i32, i32),
    /// `al_short_movie`: movie id → map.
    movie_maps: std::collections::HashMap<i32, i32>,
    movie: Option<MoviePlay>,
    /// A short movie to start when the current conversation ends (card,
    /// id, after ADV).
    pending_movie: Option<(String, i32, Vec<String>)>,
    /// The map a "一緒に遊ぶ" movie returns to.
    with_return: Option<i32>,
    /// The height pick (`_S`/`_M`/`_L` index) of the last character.
    last_height_pick: usize,
    /// The H place kind (`HPoint._placeType`) of the H scene.
    h_place: i32,
    /// `PostureSelect` of the H scene.
    h_ui: Option<crate::hui::PostureUi>,
    /// The states the last character baked (`spawn_character_states`).
    baked_states: Vec<String>,
    /// The place the posture lists were built for.
    h_ui_place: Option<i32>,
    /// The map's H points and the one in use.
    h_points: Vec<crate::h::HPoint>,
    h_point_index: Option<usize>,
    /// The point mode (`PointSetting`) and the view it replaced.
    h_point_ui: Option<(crate::hui::PointUi, Option<map::View>)>,
    /// A posture change waiting for the next step (file, id).
    h_change: Option<(String, i32)>,
    /// The H pair waiting to spawn (key light intensity).
    h_spawn_sun: Option<f32>,
    /// The covered map change released the old map (loads next step).
    map_released: bool,
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
    /// `SaveLoadUI` (its overlay is named `SaveLoadUI`).
    save_load: Option<crate::saveload::SaveLoad>,
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
        // An H scene spawns two characters next to the map.
        let budget = if self.h.is_some() { 64 << 20 } else { map::MAP_TEXTURE_BUDGET };
        let world = map::load_with_budget(&mut self.unity, &info, time_zone, budget)?;
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
        // The H place: `H/H00`, or `NightEventH00` for a night event H
        // (`MovePointInfo.GetNightEventHPoint`).
        let place = if self.h.as_ref().is_some_and(|h| h.night.is_some()) { "NightEventH00" } else { "H00" };
        self.h_point = world.points.iter().find(|p| p.0 == place).map(|p| (p.1, p.2));
        self.h_place = 0;
        if self.h.as_ref().is_some_and(|h| h.night.is_none() && h.phase == MassagePhase::Enter) {
            // `HPointCtrl.FindFirst`: a first `HPoint` of the map.
            match crate::h::load_hpoints(&mut self.unity, id) {
                Ok(points) => {
                    let first: Vec<usize> = (0..points.len()).filter(|&i| points[i].first).collect();
                    let pick = if first.is_empty() { (!points.is_empty()).then_some(0) } else { Some(first[(self.rng % first.len() as u64) as usize]) };
                    if let Some(i) = pick {
                        let p = &points[i];
                        self.h_point = Some((p.pos, p.rot));
                        self.h_place = p.place;
                        self.log.push(format!("HPointCtrl.FindFirst: place {} id {}", p.place, p.id));
                    }
                    self.h_point_index = pick;
                    self.h_points = points;
                }
                Err(e) => self.log.push(format!("H points of map {id}: {e:#}")),
            }
        }
        self.map_points = world.points.clone();
        self.map_sun = world.sun.as_ref().map_or(1., |l| l.intensity);
        let h_entry = self.h.as_ref().filter(|h| h.phase == MassagePhase::Enter).map(|h| h.card.clone());
        if h_entry.is_some() {
            // Spawned on the next step, after the previous map's models
            // leave the prepared-asset budget.
            match self.h_point {
                Some(_) => self.h_spawn_sun = Some(world.sun.as_ref().map_or(1., |l| l.intensity)),
                None => self.log.push("SATD-amanatsu-port: H map without H00".into()),
            }
        }
        let massage_entry = self.massage.as_ref().filter(|m| m.phase == MassagePhase::Enter).map(|m| m.card.clone());
        if let Some(card) = massage_entry {
            if let Some((_, pos, rot)) = world.points.iter().find(|p| p.0 == "Est00").cloned() {
                self.spawn_oil_heroine(host, client, &card, pos, rot, world.sun.as_ref().map_or(1., |l| l.intensity))?;
            } else {
                self.log.push("SATD-amanatsu-port: esthetics map without Est00".into());
            }
        }
        if let Some(m) = self.movie.as_mut().filter(|m| m.entering) {
            m.base = world.short_movies.iter().find(|p| p.0 == m.movie.id).map(|p| (p.1, p.2));
        }
        let heroines: Vec<String> = if self.massage.is_some() || self.h.is_some() || self.night.is_some() || self.movie.is_some() { Vec::new() } else { self.heroines.iter().filter(|h| self.placement.get(*h) == Some(&id)).cloned().collect() };
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
        let human = crate::chara::Human::from_card(&mut self.unity, &lists, &card, !card_path.contains("/male/"));
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

    /// A character with several exact states of one controller baked as
    /// named clips (H postures), playing `play`; returns its entities.
    fn spawn_character_states(&mut self, host: &mut dyn HostServices, client: ClientId, card_path: &str, transform: serde_json::Value, sun: f32, bundle: &str, controller: &str, states: &[&str], play: &str) -> Result<Vec<u32>> {
        let card = crate::card::Card::parse(&self.unity.read_all(card_path)?)?;
        if self.lists.is_none() {
            self.lists = Some(crate::chara::Lists::load(&mut self.unity)?);
        }
        let lists = self.lists.take().context("lists")?;
        let human = crate::chara::Human::from_card(&mut self.unity, &lists, &card, !card_path.contains("/male/"));
        self.lists = Some(lists);
        let human = human?;
        let suffix = human.height_suffix();
        self.last_height_pick = match suffix {
            "S" => 0,
            "M" => 1,
            _ => 2,
        };
        let mut clips = Vec::new();
        self.baked_states.clear();
        for state in states {
            // `_L`/`_M`/`_S` by height, falling back to the other sizes.
            // States named without the size pick their motion by height
            // (the short movie animators).
            let found = [suffix, "M", "L", "S"].iter().find_map(|sz| human.exact_state_tracks(&mut self.unity, bundle, controller, &format!("{state}_{sz}"), state).ok()).or_else(|| human.controller_tracks(&mut self.unity, bundle, controller, state).ok());
            match found {
                Some(t) => {
                    clips.push(t);
                    self.baked_states.push((*state).to_owned());
                }
                None => self.log.push(format!("H state {state} not in {controller}")),
            }
        }
        // Two characters share the H map's budget: 512 px, 16 MiB each.
        let options = crysirror_unity::world6::GlbOptions { max_texture: 512, texture_budget: 16 << 20, spring_unit: CHARACTER_UNIT };
        let mut assets = crate::chara::CharaAssets { unity: &mut self.unity, human: &human, light: 0.7 / sun.max(0.7) };
        let (chunks, _) = crysirror_unity::world6::scene_glb_animated(&human.scene, &human.key, &[0], &mut assets, &options, "CrySirror AmanatsuLocation character importer", &clips)?;
        self.log.push(format!("H character GLB {} bytes", chunks.iter().map(Vec::len).sum::<usize>()));
        let first = self.map_entities.len();
        self.spawn_models(host, client, "hchara", &chunks)?;
        let ids: Vec<u32> = self.map_entities[first..].iter().map(|e| e.0).collect();
        for &entity in &ids {
            self.world.push(json!({"op": "set_transform", "id": entity, "transform": transform}));
            self.world.push(json!({"op": "set_animation", "id": entity, "animation": {"layers": [{"clip": play, "looping": true}]}}));
        }
        self.log.push(format!("H character {card_path}: {controller} {} clips", clips.len()));
        Ok(ids)
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
        // `SetCost`: commands need cost; H also favourability LV > 0, its
        // lock cover shows below that.
        let can = gp.cost > 0;
        for (field, on) in [("_btnTalk", can), ("_btnListen", can), ("_btnMassage", can), ("_btnInvite", can), ("_btnHTopic", can), ("_btnH", can && gp.favorability.lv > 0)] {
            if let Some(n) = ui.pptr(field).and_then(|p| level.owner(p.path_id)) {
                level.interactable.insert(n, on);
            }
        }
        if let Some(n) = ui.pptr("_imgHCover").and_then(|p| level.owner(p.path_id)) {
            level.set_active(n, gp.favorability.lv <= 0);
        }
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
                count_extra: self.npc_extra.get(h).cloned().unwrap_or(CountExtra { version: Some(Version::default()), action_motion_id: -1, next_motion_id: -1, ..Default::default() }),
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
        self.npc_extra.clear();
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
            self.npc_extra.insert(path.clone(), n.count_extra);
            if n.action.map_id >= 0 {
                self.placement.insert(path, n.action.map_id);
            }
        }
    }

    /// `SaveData.Save(isAuto: true)` after a time-zone change, in the
    /// engine's save mount (`user/`).
    /// SATD-amanatsu-port: `Config.InGameData.AutoSaveMode` (mode 2 saves
    /// only in the morning) is not read; every zone change saves.
    fn auto_save(&mut self, host: &mut dyn HostServices, zone: i32) -> Result<()> {
        let storage = host.storage(self.client_id)?;
        // `Save(…, isAuto)` → `EnsureFileNameIfEmpty`: an auto save is
        // always `000` (`a_000.sav`, overwritten).
        let name = crate::save::file_name("000", true);
        let bytes = self.save_file(zone, "オートセーブデータ").encode()?;
        let _ = storage.create_directory(&VfsPath::new(crysirror_core::path::Mount::Save, "user")?);
        storage.write_atomic(&VfsPath::new(crysirror_core::path::Mount::Save, &format!("user/{name}"))?, &bytes)?;
        self.log.push(format!("SaveData.Save(auto): user/{name} ({} bytes)", bytes.len()));
        Ok(())
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

    /// `EventManager._With_d__41` after the short movie: favourability
    /// takes the late points as its new gauge and one level.
    fn favorability_up(&mut self, card: &str) {
        let max = self.param_table.points.first().map_or(6, |p| p.max_lv);
        let (gp, _, _) = self.npc(card);
        gp.favorability.point = gp.late_point;
        gp.late_point = 0;
        gp.favorability.lv = (gp.favorability.lv + 1).min(max);
        gp.favorability.is_max_lv = gp.favorability.lv == max;
        let lv = gp.favorability.lv;
        self.log.push(format!("favourability LV{lv}"));
        self.show_parameters(card);
    }

    /// `HScene.Initialize`: the heroine and the male (the default male
    /// card) at the H place in the starting posture, framed by its camera.
    /// SATD-amanatsu-port: the posture comes from the posture list's first
    /// standing kiss (`aib` 立ちキス) instead of `PostureMainSelecter`.
    fn spawn_h_pair(&mut self, host: &mut dyn HostServices, client: ClientId, card: &str, pos: [f32; 3], rot: [f32; 4], sun: f32) -> Result<()> {
        if self.h_postures.is_none() {
            self.h_postures = Some(crate::h::load_postures(&mut self.unity)?);
        }
        let lv = {
            let gp = &self.npc(card).0;
            [gp.favorability.lv, gp.inclusiveness.lv, gp.proactivity.lv, gp.curiosity.lv]
        };
        let place = self.h_place;
        let change = self.h_change.take();
        let mut rng = self.rng;
        let mut random = |n: usize| game_random(&mut rng, n);
        let postures = self.h_postures.as_deref().unwrap_or(&[]);
        // `FindPosture(isFirst)`, or the posture chosen in `PostureSelect`.
        // SATD-amanatsu-port: exhaustion (`isWeakness`) is not tracked.
        let posture = match &change {
            Some((file, id)) => postures.iter().find(|p| &p.file == file && p.id == *id),
            None => crate::h::first_posture(postures, place, lv, &mut random),
        }
        .or(postures.first())
        .cloned()
        .context("H postures")?;
        let usable: Vec<crate::h::Posture> = postures.iter().filter(|p| crate::h::category_allowed(p.mode) && p.usable(false, place, lv, false)).cloned().collect();
        self.rng = rng;
        let q = glam::Quat::from_array(crysirror_unity::gltf::mirror_quat(rot));
        let tr = glam::Vec3::from_array(crysirror_unity::gltf::mirror_point(pos));
        let (x, y, z) = q.to_euler(glam::EulerRot::XYZ);
        let t = json!({"translation": tr.to_array(), "rotation_degrees": [x.to_degrees(), y.to_degrees(), z.to_degrees()], "scale": [1.0, 1.0, 1.0]});
        let mut entities = Vec::new();
        let mut finishes = Vec::new();
        for c in &posture.charas {
            if c.info.bundle.is_empty() {
                continue;
            }
            let who = if c.is_male() { "DefaultData/0/chara/male/AL_M_00.png".to_owned() } else { card.to_owned() };
            let bundle = format!("lib/{}", c.info.bundle);
            entities.extend(self.spawn_character_states(host, client, &who, t.clone(), sun, &bundle, &c.info.asset, &H_MOTIONS, "Idle")?);
            if !c.is_male() {
                finishes = self.baked_states.iter().filter(|s| s.starts_with("Orgasm")).cloned().collect();
            }
        }
        let (personality, name) = self.unity.read_all(card).ok().and_then(|b| crate::card::Card::parse(&b).ok()).and_then(|c| c.parameter()).map_or((0, String::new()), |p| (p.0, p.1));
        if !self.spots.iter().any(|s| s.0 == card) {
            self.spots.push((card.to_owned(), tr.to_array(), personality, name));
        }
        // `PostureSelect` over the usable postures.
        if let Some(i) = self.overlays.iter().position(|(l, _)| l.name == crate::h::SCENE_PREFAB) {
            let level = &mut self.overlays[i].0;
            if self.h_ui_place != Some(place) {
                if let Some(old) = self.h_ui.take() {
                    old.dispose(level);
                }
                self.h_ui_place = Some(place);
            }
            match self.h_ui.as_mut() {
                Some(ui) => ui.set_current(level, &posture.file, posture.id),
                None => {
                    let refs: Vec<&crate::h::Posture> = usable.iter().collect();
                    self.h_ui = crate::hui::PostureUi::new(level, &refs, &posture);
                }
            }
        }
        // The posture camera around the H place.
        if let Ok(cam) = crate::oil::CameraData::load_from(&mut self.unity, "lib/h/list/camera/000_01.unity3d", &posture.camera) {
            let mirror = glam::Mat4::from_scale(glam::Vec3::new(-1., 1., 1.));
            let (eye, target, up) = cam.eye(mirror * glam::Mat4::from_rotation_translation(q, tr) * mirror);
            let v = map::View { eye: crysirror_unity::gltf::mirror_point(eye.to_array()), target: crysirror_unity::gltf::mirror_point(target.to_array()), up: crysirror_unity::gltf::mirror_point(up.to_array()), fov: cam.fov, near: 0.1, far: 1000. };
            self.world.push(v.command());
            self.view = Some(v);
        }
        self.log.push(format!("HScene: {} ({}{})", posture.name, posture.file, posture.id));
        if let Some(h) = self.h.as_mut() {
            h.posture = Some(posture);
            h.entities = entities;
            h.finishes = finishes;
        }
        Ok(())
    }

    /// `HScene.Create`: the H UI; play starts in `Idle`.
    fn enter_h_scene(&mut self, now: u64) -> Result<()> {
        let mut level = Level::prefab(&mut self.unity, crate::h::SCENE_BUNDLE, crate::h::SCENE_PREFAB)?;
        self.h_ui = None;
        self.h_ui_place = None;
        self.h_point_ui = None;
        for path in ["HScene/Canvas Point", "HScene/Canvas/ccr_dropShadow/Option/RightBottomArea/MoveSetting/root"] {
            if let Some(n) = level.find(path) {
                level.set_active(n, false);
            }
        }
        self.overlays.push((level, Ui::new(7)));
        if let Some(h) = self.h.as_mut() {
            h.phase = MassagePhase::Play;
            h.last_ms = now;
        }
        self.h_gauges();
        Ok(())
    }

    fn h_play(&mut self, motion: &'static str, looping: bool) {
        let Some(h) = self.h.as_mut() else { return };
        h.motion = motion;
        for &e in &h.entities {
            self.world.push(json!({"op": "set_animation", "id": e, "animation": {"layers": [{"clip": motion, "looping": looping}]}}));
        }
    }

    /// `Gauge Female` fill and the finish buttons' availability.
    fn h_gauges(&mut self) {
        let Some((gauge, male, finishes)) = self.h.as_ref().map(|h| (h.gauge, h.male_gauge, h.finishes.clone())) else { return };
        let Some(oi) = self.overlays.iter().position(|(l, _)| l.name == crate::h::SCENE_PREFAB) else { return };
        let level = &mut self.overlays[oi].0;
        for (who, value) in [("Female", gauge), ("Male", male)] {
            for (i, path) in ["Fill Area 0", "Fill Area 1", "Fill Area 2"].iter().enumerate() {
                // Three stages of a third each.
                if let Some(n) = level.find(&format!("HScene/Canvas/ccr_dropShadow/GaugeController/Gauge {who}/{path}/Fill")) {
                    let t = ((value / 100. * 3.) - i as f32).clamp(0., 1.);
                    level.states.entry(n).or_default().fill = Some(t);
                }
            }
        }
        let has = |s: &str| finishes.iter().any(|f| f == s);
        for (button, ok) in [
            ("Finish Female", gauge >= 70. && has("OrgasmF")),
            ("Finish Male", male >= 70. && has("OrgasmM_IN")),
            ("Finish Shot", male >= 70. && has("OrgasmM_OUT")),
            ("Finish Double", gauge >= 70. && male >= 70. && has("OrgasmF") && has("OrgasmM_IN")),
        ] {
            if let Some(n) = level.find(&format!("HScene/Canvas/ccr_dropShadow/GaugeController/{button}")) {
                level.interactable.insert(n, ok);
            }
        }
    }

    /// `MotionPlayer`: a click on the scene steps Idle → weak loop →
    /// strong loop → weak loop; the loops fill the female gauge
    /// (SATD-amanatsu-port: rates 4/s weak and 8/s strong, no wheel speed,
    /// voices, IK or male gauge); a finished one-shot returns to Idle.
    fn h_update(&mut self, clicked: bool, now: u64) {
        let Some(h) = self.h.as_mut() else { return };
        if h.phase != MassagePhase::Play {
            return;
        }
        let dt = (now.saturating_sub(h.last_ms) as f32 / 1000.).min(0.1);
        h.last_ms = now;
        let rate = match h.motion {
            "WLoop1" => 4.,
            "SLoop1" => 8.,
            _ => 0.,
        };
        h.gauge = (h.gauge + rate * dt).min(100.);
        h.male_gauge = (h.male_gauge + rate * dt).min(100.);
        if h.motion.starts_with("Orgasm") && now >= h.motion_until {
            self.h_play("Idle", true);
        } else if clicked {
            let next = match h.motion {
                "Idle" => Some("WLoop1"),
                "WLoop1" => Some("SLoop1"),
                "SLoop1" => Some("WLoop1"),
                _ => None,
            };
            if let Some(n) = next {
                self.h_play(n, true);
            }
        }
        self.h_gauges();
    }

    /// `PointSetting`: the point canvas over the map's overview camera with
    /// a marker on every other H point.
    fn open_h_points(&mut self) {
        // SATD-amanatsu-port: the point mode's free camera is not ported;
        // the H camera stays and points outside it are marked at the screen
        // edge toward them.
        let Some(view) = self.view.clone() else { return };
        let marks: Vec<(usize, i32, [f32; 2])> = self
            .h_points
            .iter()
            .enumerate()
            .filter(|(i, _)| Some(*i) != self.h_point_index)
            .map(|(i, p)| (i, p.place, project_edge(&view, crysirror_unity::gltf::mirror_point(p.pos))))
            .collect();
        let Some(oi) = self.overlays.iter().position(|(l, _)| l.name == crate::h::SCENE_PREFAB) else { return };
        if let Some(ui) = crate::hui::PointUi::open(&mut self.overlays[oi].0, &marks, HEIGHT) {
            self.log.push(format!("PointSetting: {} points", marks.len()));
            self.world.push(view.command());
            self.h_point_ui = Some((ui, self.view.replace(view)));
        }
    }

    /// A point-mode click: back, or `HScene.ChangePoint` → a posture usable
    /// at the new place (`FindPosture`).
    fn h_point_action(&mut self, action: crate::hui::PointAction) {
        use crate::hui::PointAction as P;
        if action == P::None {
            return;
        }
        let Some((ui, view)) = self.h_point_ui.take() else { return };
        if let Some(oi) = self.overlays.iter().position(|(l, _)| l.name == crate::h::SCENE_PREFAB) {
            ui.close(&mut self.overlays[oi].0);
        }
        if let Some(v) = view {
            self.world.push(v.command());
            self.view = Some(v);
        }
        let P::Move(i) = action else { return };
        let Some(p) = self.h_points.get(i).cloned() else { return };
        let Some(card) = self.h.as_ref().map(|h| h.card.clone()) else { return };
        let lv = {
            let gp = &self.npc(&card).0;
            [gp.favorability.lv, gp.inclusiveness.lv, gp.proactivity.lv, gp.curiosity.lv]
        };
        let list: Vec<(String, i32)> = self.h_postures.as_deref().unwrap_or(&[]).iter().filter(|q| crate::h::category_allowed(q.mode) && q.usable(false, p.place, lv, false)).map(|q| (q.file.clone(), q.id)).collect();
        if list.is_empty() {
            self.log.push(format!("HScene.ChangePoint: no posture for place {}", p.place));
            return;
        }
        let pick = list[self.random(list.len())].clone();
        self.h_point = Some((p.pos, p.rot));
        self.h_place = p.place;
        self.h_point_index = Some(i);
        self.h_change = Some(pick);
        self.log.push(format!("HScene.ChangePoint: place {} id {}", p.place, p.id));
    }

    /// H UI buttons: `Finish Female` (orgasm), `Exit` (end).
    fn h_button(&mut self, name: &str, now: u64) -> Result<()> {
        match name {
            "Point" => self.open_h_points(),
            "Finish Female" | "Finish Male" | "Finish Shot" | "Finish Double" => {
                // SATD-amanatsu-port: the finish menus (外出し/飲ませる/吐かせる),
                // fluids and the male finish count are not ported.
                let (state, female, male): (&'static str, bool, bool) = match name {
                    "Finish Female" => ("OrgasmF", true, false),
                    "Finish Male" => ("OrgasmM_IN", false, true),
                    "Finish Shot" => ("OrgasmM_OUT", false, true),
                    _ => ("OrgasmF", true, true),
                };
                let ok = self.h.as_ref().is_some_and(|h| h.finishes.iter().any(|f| f == state) && (!female || h.gauge >= 70.) && (!male || h.male_gauge >= 70.));
                if let Some(h) = self.h.as_mut().filter(|_| ok) {
                    if female {
                        h.gauge = 0.;
                        h.orgasms += 1;
                    }
                    if male {
                        h.male_gauge = 0.;
                    }
                    h.motion_until = now + 3000;
                    self.h_play(state, false);
                    self.log.push(format!("HScene finish: {state}"));
                } else {
                    self.log.push(format!("HScene {name}: not available"));
                }
            }
            "Exit" => {
                // `HScene.End` → `AddHCount`, the evaluation scenario.
                // SATD-amanatsu-port: the evaluation index is the orgasm count.
                let Some(h) = self.h.as_mut().filter(|h| h.phase == MassagePhase::Play) else { return Ok(()) };
                h.phase = MassagePhase::Evaluation;
                let index = (h.orgasms as usize).min(19);
                let night = h.night.is_some();
                let mut rng = self.rng;
                let mut random = |n: usize| game_random(&mut rng, n);
                // A night event H continues in `NightEvent` instead.
                let list: Vec<String> = if night { Vec::new() } else { crate::commands::choose(&self.adv_assets.h_evaluation(index), &mut random).into_iter().collect() };
                self.rng = rng;
                if let Some(oi) = self.overlays.iter().position(|(l, _)| l.name == crate::h::SCENE_PREFAB) {
                    let (_, mut ui) = self.overlays.remove(oi);
                    self.removed.extend(ui.clear());
                }
                self.log.push(format!("HScene end: {} orgasms, {list:?}", self.h.as_ref().map_or(0, |h| h.orgasms)));
                if list.is_empty() {
                    self.h_talk_ended(now)?;
                } else {
                    self.start_talk(0, list)?;
                }
            }
            other => self.log.push(format!("SATD-amanatsu-port: HScene button {other} not ported")),
        }
        Ok(())
    }

    /// A conversation ended during H: advance its phase.
    fn h_talk_ended(&mut self, now: u64) -> Result<()> {
        let Some(phase) = self.h.as_ref().map(|h| h.phase) else { return Ok(()) };
        match phase {
            MassagePhase::Agree => {
                if let Some((_, ci)) = self.command.take() {
                    if ci < self.overlays.len() {
                        let (_, mut ui) = self.overlays.remove(ci);
                        self.removed.extend(ui.clear());
                    }
                }
                let zone = if let Controller::ActionMap(m) = &self.controller { m.time_zone } else { 0 };
                let here = if let Controller::ActionMap(m) = &self.controller { if let actionmap::Phase::Map(id) = m.phase { Some(id) } else { None } } else { None };
                // An H map: the current one with an H place, else the hotel.
                let target = if self.h_point.is_some() { here.unwrap_or(12) } else { 12 };
                if let Some(h) = self.h.as_mut() {
                    h.phase = MassagePhase::Enter;
                }
                self.fade.start(now, true, self.fade_seconds);
                self.pending_map = Some((target, zone));
                self.log.push(format!("EventManager.H → map {target}"));
            }
            MassagePhase::Evaluation => {
                let Some(h) = self.h.take() else { return Ok(()) };
                // `HDetail`: `IsTimeZoneH`, `IsH`, and `IsNightH` at night.
                let zone = if let Controller::ActionMap(c) = &self.controller { c.time_zone } else { 0 };
                {
                    let e = self.extra(&h.card);
                    e.is_time_zone_h = true;
                    e.is_h = true;
                    e.is_night_h |= zone == 3;
                }
                if let Some(kind) = h.night {
                    // `NightEvent`: the event's points (`GetSelect(0)`),
                    // `AddHCount(force)`, then its after scenario.
                    self.apply_points(h.card.clone(), kind, Some(0));
                    let count = &mut self.npc(&h.card).1;
                    count.h = (count.h + 1).min(100);
                    let (phase, list) = if kind == "H_NightShowerAfter" { (NightPhase::HAfter, self.adv_assets.event("Night_H_After")) } else { (NightPhase::MorningHAfter, self.adv_assets.h_event_after()) };
                    if let Some(n) = self.night.as_mut() {
                        n.phase = phase;
                    }
                    self.log.push(format!("night H done: {} orgasms", h.orgasms));
                    return self.night_talk(&h.card, list.into_iter().collect(), &[]);
                }
                let index = (h.orgasms as usize).min(7);
                self.apply_points(h.card.clone(), "H", Some(index));
                {
                    let (gp, count, _) = self.npc(&h.card);
                    count.h = (count.h + 1).min(100);
                    gp.cost = (gp.cost - 1).max(0);
                }
                let zone = if let Controller::ActionMap(c) = &self.controller { c.time_zone } else { 0 };
                self.fade.start(now, true, self.fade_seconds);
                self.pending_map = Some((h.return_map, zone));
                let n = self.npc(&h.card).1.h;
                self.log.push(format!("H done: {} orgasms, H count {n}", h.orgasms));
            }
            _ => {}
        }
        Ok(())
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
                // `EventManager.TimeZone`: at night, `NightEvent` with
                // `GetNightEventHeroine` when `IsNightEvent` rolls.
                if m.time_zone == 3 {
                    if let Some(card) = self.night_event_heroine() {
                        let roll = self.random(100) as i64 + 1;
                        self.log.push(format!("IsNightEvent {card}: {roll} <= {}", self.night_rate));
                        if roll <= self.night_rate {
                            self.night = Some(NightRun { card: card.clone(), phase: NightPhase::Invite, cottage: self.adv_maps.get(&map_id) == Some(&"Cottage") });
                            self.pending_night = Some(card);
                            self.set_hud_visible(false);
                            return Ok(());
                        }
                    }
                }
                self.next_time_zone(map_id, now);
            }
            H::Config | H::Help | H::CameraPrev | H::CameraNext | H::CameraReset => self.log.push(format!("SATD-amanatsu-port: ActionMapUI {action:?} not ported")),
        }
        Ok(())
    }

    /// `SaveLoadUI.Create` + `OpenExe`: over the title (`IsTitle`) or the
    /// map select.
    fn open_save_load(&mut self, host: &mut dyn HostServices, is_title: bool) -> Result<()> {
        if self.save_load.is_some() {
            return Ok(());
        }
        let mut level = Level::prefab(&mut self.unity, actionmap::UI_BUNDLE, crate::saveload::PREFAB)?;
        let Some(mut sl) = crate::saveload::SaveLoad::new(&mut level, is_title) else {
            self.log.push("SATD-amanatsu-port: SaveLoadUI controls not found".into());
            return Ok(());
        };
        sl.new_time = now_text().0;
        sl.new_zone = if let Controller::ActionMap(m) = &self.controller { m.time_zone } else { 0 };
        let infos = self.save_slots(host);
        sl.set_infos(&mut level, infos);
        self.log.push(format!("SaveLoadUI.Open (title {is_title}): {} slots with data", sl.infos.len()));
        self.overlays.push((level, Ui::new(9)));
        self.save_load = Some(sl);
        Ok(())
    }

    /// The `SaveData` of every slot file (`SaveInfo` 0..99 and the auto
    /// slot) in the engine's save mount.
    fn save_slots(&mut self, host: &mut dyn HostServices) -> std::collections::HashMap<usize, crate::saveload::SlotData> {
        let mut out = std::collections::HashMap::new();
        let Ok(storage) = host.storage(self.client_id) else { return out };
        let files: std::collections::HashSet<String> = storage.list(crysirror_core::path::Mount::Save, "user").unwrap_or_default().into_iter().map(|f| f.rsplit('/').next().unwrap_or("").to_owned()).collect();
        for id in (0..100).chain([crate::saveload::AUTO]) {
            let name = crate::saveload::file_name(id);
            if !files.contains(&name) {
                continue;
            }
            let Ok(path) = VfsPath::new(crysirror_core::path::Mount::Save, &format!("user/{name}")) else { continue };
            match storage.read(&path).map_err(anyhow::Error::from).and_then(|b| crate::save::SaveFile::decode(&b)) {
                Ok(f) => {
                    out.insert(id, crate::saveload::SlotData { time_zone: f.save.time_zone, time: f.save.save_time_text.clone().unwrap_or_default(), comment: f.save.comment.clone().unwrap_or_default() });
                }
                Err(e) => self.log.push(format!("save {name}: {e:#}")),
            }
        }
        out
    }

    fn close_save_load(&mut self) {
        if let Some(i) = self.overlays.iter().position(|(l, _)| l.name == crate::saveload::PREFAB) {
            let (_, mut ui) = self.overlays.remove(i);
            self.removed.extend(ui.clear());
        }
        self.save_load = None;
    }

    /// A `SaveLoadUI` request: load (`LoadSaveFileInternal` → the action
    /// map), save (`SaveData.Save(fileName)`) or delete a slot file.
    fn save_load_action(&mut self, host: &mut dyn HostServices, action: crate::saveload::Action, now: u64) -> Result<()> {
        use crate::saveload::Action as A;
        let path = |id: usize| VfsPath::new(crysirror_core::path::Mount::Save, &format!("user/{}", crate::saveload::file_name(id)));
        match action {
            A::None => {}
            A::Close => {
                self.level.sounds.push(sound::Call::SystemSe(self.sound_ids.ok_s));
                self.close_save_load();
            }
            A::Load(id) => {
                let storage = host.storage(self.client_id)?;
                let file = crate::save::SaveFile::decode(&storage.read(&path(id)?)?)?;
                self.log.push(format!("SaveData.Load({}): zone {}, {} heroines", crate::saveload::file_name(id), file.save.time_zone, file.npcs.len()));
                self.close_save_load();
                self.loaded_save = Some(file);
                self.fade.start(now, true, self.fade_seconds);
                self.pending = Some((scenes::scene_id::ACTION_MAP, now));
            }
            A::Save(id, comment) => {
                let zone = if let Controller::ActionMap(m) = &self.controller { m.time_zone } else { 0 };
                let bytes = self.save_file(zone, &comment).encode()?;
                let storage = host.storage(self.client_id)?;
                let _ = storage.create_directory(&VfsPath::new(crysirror_core::path::Mount::Save, "user")?);
                storage.write_atomic(&path(id)?, &bytes)?;
                self.log.push(format!("SaveData.Save: user/{} ({} bytes)", crate::saveload::file_name(id), bytes.len()));
                self.refresh_save_load(host);
            }
            A::Delete(id) => {
                let storage = host.storage(self.client_id)?;
                storage.remove(&path(id)?)?;
                self.log.push(format!("SaveLoadUI delete: user/{}", crate::saveload::file_name(id)));
                self.refresh_save_load(host);
            }
        }
        Ok(())
    }

    fn refresh_save_load(&mut self, host: &mut dyn HostServices) {
        let infos = self.save_slots(host);
        if let (Some(sl), Some(i)) = (self.save_load.as_mut(), self.overlays.iter().position(|(l, _)| l.name == crate::saveload::PREFAB)) {
            sl.set_infos(&mut self.overlays[i].0, infos);
        }
    }

    /// `EventManager.TimeZone`: SE, fade, `Cycle.IncrementTimeZone`, every
    /// heroine's per-zone counts, `TimeZoneInternal` (the map again in the
    /// new zone with new placements) and the auto save.
    fn next_time_zone(&mut self, map_id: i32, now: u64) {
        let Controller::ActionMap(m) = &mut self.controller else { return };
        m.time_zone = (m.time_zone + 1) % 4;
        let zone = m.time_zone;
        self.level.sounds.push(sound::Call::SystemSe(self.sound_ids.time_zone_se));
        self.fade.start(now, true, self.fade_seconds);
        self.place_heroines(true);
        for c in self.npc_extra.values_mut() {
            c.time_zone_play_command = 0;
            c.is_time_zone_h = false;
            c.action_motion_id = -1;
            c.next_motion_id = -1;
        }
        self.pending_map = Some((map_id, zone));
        self.log.push(format!("EventManager.TimeZone → zone {zone}"));
        self.pending_auto_save = Some(zone);
    }

    fn extra(&mut self, card: &str) -> &mut crate::save::CountExtra {
        self.npc_extra.entry(card.to_owned()).or_insert_with(|| crate::save::CountExtra { version: Some(crate::save::Version::default()), action_motion_id: -1, next_motion_id: -1, ..Default::default() })
    }

    /// `GetNightEventHeroine`: a random heroine with favourability LV 4 or
    /// more who had H (`IsH`) but no night H yet (`IsNightH`).
    fn night_event_heroine(&mut self) -> Option<String> {
        let mut list = Vec::new();
        for h in self.heroines.clone() {
            let lv = self.npc(&h).0.favorability.lv;
            let e = self.extra(&h).clone();
            if lv >= 4 && e.is_h && !e.is_night_h {
                list.push(h);
            }
        }
        (!list.is_empty()).then(|| {
            let i = self.random(list.len());
            list.swap_remove(i)
        })
    }

    /// `ActionPoint.Reflect`: the night event heroine at the map's
    /// `NightEvent00` (unless she already stands on this map).
    fn night_spawn_heroine(&mut self, host: &mut dyn HostServices, client: ClientId, card: &str) -> Result<()> {
        if self.spots.iter().any(|s| s.0 == card) {
            return Ok(());
        }
        let (pos, rot) = self.map_points.iter().find(|p| p.0 == "NightEvent00").map(|p| (p.1, p.2)).unwrap_or(([0.; 3], [0., 0., 0., 1.]));
        let q = glam::Quat::from_array(crysirror_unity::gltf::mirror_quat(rot));
        let (x, y, z) = q.to_euler(glam::EulerRot::XYZ);
        let t = json!({"translation": crysirror_unity::gltf::mirror_point(pos), "rotation_degrees": [x.to_degrees(), y.to_degrees(), z.to_degrees()], "scale": [1.0, 1.0, 1.0]});
        let sun = self.map_sun;
        self.spawn_character(host, client, card, Some(t), sun)?;
        let (personality, name) = self.unity.read_all(card).ok().and_then(|b| crate::card::Card::parse(&b).ok()).and_then(|c| c.parameter()).map_or((0, String::new()), |p| (p.0, p.1));
        self.spots.push((card.to_owned(), crysirror_unity::gltf::mirror_point(pos), personality, name));
        Ok(())
    }

    /// A night event conversation with its heroine.
    fn night_talk(&mut self, card: &str, scenarios: Vec<String>, vars: &[(&str, crate::adv::Var)]) -> Result<()> {
        let spot = match self.spots.iter().position(|s| s.0 == card) {
            Some(i) => i,
            None => {
                let (personality, name) = self.unity.read_all(card).ok().and_then(|b| crate::card::Card::parse(&b).ok()).and_then(|c| c.parameter()).map_or((0, String::new()), |p| (p.0, p.1));
                self.spots.push((card.to_owned(), [0.; 3], personality, name));
                self.spots.len() - 1
            }
        };
        self.start_talk_vars(spot, scenarios, vars)
    }

    /// `NightEvent` after one of its conversations.
    fn night_talk_ended(&mut self, now: u64) -> Result<()> {
        let Some((card, phase, cottage)) = self.night.as_ref().map(|n| (n.card.clone(), n.phase, n.cottage)) else { return Ok(()) };
        let (selected, is_h_scene) = self.last_talk;
        let event = |key: &str| self.adv_assets.event(key).into_iter().collect::<Vec<String>>();
        let (shower_map, morning_map) = self.night_movie_maps;
        match phase {
            NightPhase::Invite if selected == Some(1) => {
                // Declined: `Night` (the next morning without the event).
                self.log.push(format!("NightEvent {card}: declined"));
                self.night = None;
                let map = self.current_map();
                self.next_time_zone(map, now);
            }
            NightPhase::Invite => {
                // `EventShortMovie(_nightEventShowerSMID)` and its ADV.
                // SATD-amanatsu-port: the after-bath coordinate is not applied.
                let list = event(if cottage { "Before_Night_Cottage_ShortMovie" } else { "Before_Night_NO_Cottage_ShortMovie" });
                self.night.as_mut().unwrap().phase = NightPhase::Shower;
                let id = self.night_movie_ids.0;
                self.start_movie(&card, id, list, now);
            }
            NightPhase::Shower => {
                // `HDetail(Event1)` at the movie map's night H place.
                self.h = Some(HRun { card: card.clone(), phase: MassagePhase::Enter, return_map: shower_map, posture: None, entities: Vec::new(), motion: "Idle", motion_until: 0, gauge: 0., male_gauge: 0., finishes: Vec::new(), orgasms: 0, last_ms: now, night: Some("H_NightShowerAfter") });
                self.fade.start(now, true, self.fade_seconds);
                self.pending_map = Some((shower_map, 3));
                self.log.push(format!("NightEvent {card}: H on map {shower_map}"));
            }
            NightPhase::HAfter => {
                // `Night`: the next morning on that map, with her.
                self.night.as_mut().unwrap().phase = NightPhase::NewDay;
                if let Controller::ActionMap(m) = &mut self.controller {
                    m.phase = actionmap::Phase::Map(shower_map);
                }
                self.next_time_zone(shower_map, now);
            }
            NightPhase::Morning if is_h_scene => {
                // The morning H (`HDetail(Event2/3)`).
                self.h = Some(HRun { card: card.clone(), phase: MassagePhase::Enter, return_map: morning_map, posture: None, entities: Vec::new(), motion: "Idle", motion_until: 0, gauge: 0., male_gauge: 0., finishes: Vec::new(), orgasms: 0, last_ms: now, night: Some("H_NightMorningAfter") });
                self.fade.start(now, true, self.fade_seconds);
                let zone = if let Controller::ActionMap(m) = &self.controller { m.time_zone } else { 0 };
                self.pending_map = Some((morning_map, zone));
            }
            NightPhase::Morning | NightPhase::MorningHAfter | NightPhase::Greetings => {
                // `SceneEnd` → `EndProc`: the action map again.
                self.night = None;
                self.log.push(format!("NightEvent {card}: end"));
                self.fade.start(now, true, self.fade_seconds);
                let zone = if let Controller::ActionMap(m) = &self.controller { m.time_zone } else { 0 };
                self.pending_map = Some((self.current_map(), zone));
            }
            NightPhase::NewDay => {}
        }
        Ok(())
    }

    /// `NightEvent` after `Night`: by `NightEventCount` (the first three
    /// in order, then at random) `Night_Morning` with `Judge`, or the
    /// greeting movie.
    /// SATD-amanatsu-port: the flag that turns events 1 and 2 into 3 and
    /// the morning movie (`_nightEventSMID`) are not ported.
    fn night_morning(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<()> {
        let Some(card) = self.night.as_ref().map(|n| n.card.clone()) else { return Ok(()) };
        self.night_spawn_heroine(host, client, &card)?;
        let count = self.npc(&card).1.night_events;
        let id = if count < 3 {
            self.npc(&card).1.night_events = (count + 1).min(3);
            count
        } else {
            self.random(4) as i32
        };
        self.log.push(format!("NightEvent {card}: morning event {id}"));
        if id != 0 {
            let judge = match id {
                1 => 1,
                2 => 0,
                _ => 2,
            };
            self.night.as_mut().unwrap().phase = NightPhase::Morning;
            let list = self.adv_assets.event("Night_Morning").into_iter().collect();
            self.night_talk(&card, list, &[("Judge", crate::adv::Var::Int(judge))])
        } else {
            self.night.as_mut().unwrap().phase = NightPhase::Greetings;
            let list = self.adv_assets.event("Greetings_ShortMovie").into_iter().collect();
            let (id, now) = (self.night_movie_ids.1, self.now);
            self.start_movie(&card, id, list, now);
            Ok(())
        }
    }

    fn current_map(&self) -> i32 {
        if let Controller::ActionMap(m) = &self.controller {
            if let actionmap::Phase::Map(id) = m.phase {
                return id;
            }
        }
        0
    }

    /// `EventShortMovie`: the movie's map (`ALShortMovieContent.MapID`)
    /// behind the fade, then `ShortMovie.PlayAsync` and the ADV after it.
    fn start_movie(&mut self, card: &str, id: i32, after: Vec<String>, now: u64) {
        // `CommandScene` closes for the movie.
        self.command = None;
        while let Some(i) = self.overlays.iter().position(|(l, _)| l.name == "command") {
            let (_, mut ui) = self.overlays.remove(i);
            self.removed.extend(ui.clear());
        }
        let movie = match crate::sm::Movie::load(&mut self.unity, id) {
            Ok(m) => m,
            Err(e) => {
                self.log.push(format!("ShortMovie {id}: {e:#}"));
                let _ = self.night_talk(card, after, &[]);
                return;
            }
        };
        let zone = if let Controller::ActionMap(m) = &self.controller { m.time_zone } else { 0 };
        let map = self.movie_maps.get(&id).copied().unwrap_or_else(|| self.current_map());
        self.log.push(format!("ShortMovie {id} on map {map}: {:.1} s", movie.duration));
        self.movie = Some(MoviePlay { movie, card: card.to_owned(), entering: true, start_ms: now, base: None, cameras: Default::default(), entities: Vec::new(), heroine_clip: None, fades: Vec::new(), after });
        self.fade.start(now, true, self.fade_seconds);
        self.pending_map = Some((map, zone));
    }

    /// `ShortMovie.Initialize`/`Load`: the heroine at the short movie point
    /// in its animator, the camera clips at her height.
    fn begin_movie(&mut self, host: &mut dyn HostServices, client: ClientId, now: u64) -> Result<()> {
        let Some((card, id, states, base)) = self.movie.as_ref().map(|m| (m.card.clone(), m.movie.id, m.movie.heroine_states(), m.base)) else { return Ok(()) };
        let (pos, rot) = base.unwrap_or(([0.; 3], [0., 0., 0., 1.]));
        if base.is_none() {
            self.log.push(format!("SATD-amanatsu-port: short movie point {id} not on the map"));
        }
        let q = glam::Quat::from_array(crysirror_unity::gltf::mirror_quat(rot));
        let (x, y, z) = q.to_euler(glam::EulerRot::XYZ);
        let t = json!({"translation": crysirror_unity::gltf::mirror_point(pos), "rotation_degrees": [x.to_degrees(), y.to_degrees(), z.to_degrees()], "scale": [1.0, 1.0, 1.0]});
        let refs: Vec<&str> = states.iter().map(String::as_str).collect();
        let first = refs.first().copied().unwrap_or("Idle");
        let sun = self.map_sun;
        let entities = self.spawn_character_states(host, client, &card, t, sun, crate::sm::ANIMATORS, &format!("stm_f_{id:02}"), &refs, first)?;
        let pick = self.last_height_pick;
        let cameras = match self.movie.as_ref().map(|m| m.movie.clone()) {
            Some(m) => m.camera_clips(&mut self.unity, pick).unwrap_or_default(),
            None => Default::default(),
        };
        let (personality, name) = self.unity.read_all(&card).ok().and_then(|b| crate::card::Card::parse(&b).ok()).and_then(|c| c.parameter()).map_or((0, String::new()), |p| (p.0, p.1));
        if !self.spots.iter().any(|s| s.0 == card) {
            self.spots.push((card.clone(), crysirror_unity::gltf::mirror_point(pos), personality, name));
        }
        if let Some(m) = self.movie.as_mut() {
            // `SceneFadeInfo`: In covers, Out reveals, InOut covers then
            // reveals.
            for f in &m.movie.fades {
                match f.kind {
                    1 => m.fades.push((f.start, true, f.duration, f.color)),
                    2 => m.fades.push((f.start, false, f.duration, f.color)),
                    3 => {
                        m.fades.push((f.start, true, f.duration / 2., f.color));
                        m.fades.push((f.start + f.duration / 2., false, f.duration / 2., f.color));
                    }
                    _ => {}
                }
            }
            m.entering = false;
            m.start_ms = now;
            m.entities = entities;
            m.cameras = cameras;
        }
        self.movie_update(now);
        Ok(())
    }

    /// One frame of the movie: heroine states, camera, fades; at the end
    /// the ADV after it.
    fn movie_update(&mut self, now: u64) {
        let Some(m) = self.movie.as_mut().filter(|m| !m.entering) else { return };
        let t = now.saturating_sub(m.start_ms) as f64 / 1000.;
        // The heroine's state of the latest started clip.
        if let Some(i) = m.movie.heroine.iter().rposition(|c| c.start <= t) {
            if m.heroine_clip != Some(i) {
                m.heroine_clip = Some(i);
                let c = &m.movie.heroine[i];
                for &e in &m.entities {
                    self.world.push(json!({"op": "set_animation", "id": e, "animation": {"layers": [{"clip": c.state, "looping": c.looping}]}}));
                }
            }
        }
        // The camera of the active (or next) camera clip, relative to the
        // short movie point.
        let cam = m.movie.camera.iter().rev().find(|c| c.start <= t).or(m.movie.camera.first()).cloned();
        if let Some(c) = cam {
            let local = (c.clip_in + (t.clamp(c.start, c.end) - c.start) * c.time_scale) as f32;
            if let Some((p, r, fov)) = m.cameras.get(&c.state).and_then(|clip| crate::sm::camera_pose(clip, clip.start + local)) {
                let (bp, br) = m.base.unwrap_or(([0.; 3], [0., 0., 0., 1.]));
                let base = glam::Mat4::from_rotation_translation(glam::Quat::from_array(br), glam::Vec3::from_array(bp));
                let eye = base.transform_point3(glam::Vec3::from_array(p));
                let rot = glam::Quat::from_array(br) * glam::Quat::from_array(r);
                // The animated rig's camera looks along its −Z (the camera
                // object is turned half way round under it).
                let fwd = rot * -glam::Vec3::Z;
                let up = rot * glam::Vec3::Y;
                let v = map::View { eye: crysirror_unity::gltf::mirror_point(eye.to_array()), target: crysirror_unity::gltf::mirror_point((eye + fwd).to_array()), up: crysirror_unity::gltf::mirror_point(up.to_array()), fov: fov.unwrap_or(40.), near: 0.1, far: 1000. };
                self.world.push(v.command());
                self.view = Some(v);
            }
        }
        let m = self.movie.as_mut().unwrap();
        while m.fades.first().is_some_and(|f| f.0 <= t) {
            let (_, cover, duration, color) = m.fades.remove(0);
            self.fade.color = color;
            self.fade.start(now, cover, duration as f32);
        }
        if t >= m.movie.duration {
            let m = self.movie.take().unwrap();
            self.log.push(format!("ShortMovie {} end", m.movie.id));
            if !self.fade.fading(now) {
                self.fade.start(now, false, 0.3);
            }
            if let Err(e) = self.night_talk(&m.card, m.after, &[]) {
                self.log.push(format!("ShortMovie ADV: {e:#}"));
            }
        }
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
        // `CommandUI.IsNextShortMovie`: "一緒に遊ぶ" only with a full
        // favourability gauge and the level's massage/H progress.
        let with_ready = {
            let card = self.spots[spot].0.clone();
            let lv_point = self.param_table.points.first().map_or(100, |p| p.lv_point);
            let (gp, count, _) = self.npc(&card).clone();
            !gp.favorability.is_max_lv && gp.favorability.point >= lv_point && crate::params::favorability_gate(&gp.favorability, &count)
        };
        let hidden: &[&str] = if with_ready { &["Command/Canvas/Hide", "Command/Canvas/Panel/WaterGun"] } else { &["Command/Canvas/Hide", "Command/Canvas/Panel/WaterGun", "Command/Canvas/Panel/Command/WithPlay"] };
        for path in hidden.iter().copied() {
            if let Some(n) = level.find(path) {
                level.set_active(n, false);
            }
        }
        if with_ready {
            if let Some(n) = level.find("Command/Canvas/Panel/Command/WithPlay") {
                level.set_active(n, true);
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
        self.start_talk_vars(spot, scenarios, &[])
    }

    /// A conversation with `ADVCore.PackData` variables set first.
    fn start_talk_vars(&mut self, spot: usize, scenarios: Vec<String>, vars: &[(&str, crate::adv::Var)]) -> Result<()> {
        let (_, _, personality, name) = self.spots[spot].clone();
        let key = self.unity.bundle_files().into_iter().find(|k| k.starts_with(&format!("bundle:{}#", crate::talk::ADV_BUNDLE)) && !k.ends_with(".resS")).context("ADV bundle")?;
        let mut level = Level::root_of(&mut self.unity, &key, "ADVCore")?;
        let mut talk = crate::talk::Talk::start(&mut self.unity, &mut level, personality, &name, self.now, scenarios)?;
        for (k, v) in vars {
            talk.program.vars.insert((*k).to_owned(), v.clone());
        }
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
    fonts: &'a mut crate::ui::FontCache,
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
    fn font_cache(&mut self) -> &mut crate::ui::FontCache {
        self.fonts
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
        let event = pd.fields("_explore").and_then(|e| e.fields("_event"));
        let night_rate = event.and_then(|e| e.int("_nightEventRate")).unwrap_or(50);
        let movie_maps = crate::tables::short_movie_maps(&mut unity).unwrap_or_default();
        let movie = |key: &str, default: i64| movie_maps.get(&(event.and_then(|e| e.int(key)).unwrap_or(default) as i32)).copied().unwrap_or(2);
        let night_movie_maps = (movie("_nightEventShowerSMID", 20), movie("_nightEventSMID", 22));
        let night_movie_ids = (event.and_then(|e| e.int("_nightEventShowerSMID")).unwrap_or(20) as i32, event.and_then(|e| e.int("_nightEventSMID")).unwrap_or(22) as i32);
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
            pending_with: None,
            h: None,
            h_postures: None,
            h_point: None,
            h_spawn_sun: None,
            h_place: 0,
            h_ui: None,
            h_change: None,
            h_points: Vec::new(),
            h_ui_place: None,
            baked_states: Vec::new(),
            h_point_index: None,
            h_point_ui: None,
            font_cache: Default::default(),
            night: None,
            pending_night: None,
            npc_extra: std::collections::HashMap::new(),
            map_points: Vec::new(),
            map_sun: 1.,
            last_talk: (None, false),
            night_rate,
            night_movie_maps,
            night_movie_ids,
            movie_maps,
            movie: None,
            pending_movie: None,
            last_height_pick: 1,
            with_return: None,
            map_released: false,
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
            save_load: None,
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
            if !game.fade.fading(now) && !game.map_released {
                // `ReleaseMapObject` first: the old map's models leave the
                // prepared-asset budget before the next map loads.
                game.release_map(host, client)?;
                game.map_released = true;
            } else if !game.fade.fading(now) {
                game.pending_map = None;
                game.map_released = false;
                if let Some((_, mut ui)) = (!game.overlays.is_empty()).then(|| game.overlays.remove(0)) {
                    game.removed.extend(ui.clear());
                }
                game.hud = None;
                game.change_map(host, client, id, zone)?;
                if game.massage.as_ref().is_some_and(|m| m.phase == MassagePhase::Enter) {
                    game.enter_oil_scene(host, client, now)?;
                } else if game.h.as_ref().is_some_and(|h| h.phase == MassagePhase::Enter) {
                    game.enter_h_scene(now)?;
                } else if game.movie.as_ref().is_some_and(|m| m.entering) {
                    game.begin_movie(host, client, now)?;
                } else if game.night.as_ref().is_some_and(|n| n.phase == NightPhase::NewDay) {
                    game.night_morning(host, client)?;
                } else {
                    if let Controller::ActionMap(m) = &mut game.controller {
                        m.phase = actionmap::Phase::Map(id);
                    }
                    game.open_hud(id, zone)?;
                }
                // `PlayBGMBYActionMap`.
                let bgm = game.sound_ids.zone_bgm[zone.clamp(0, 3) as usize];
                game.level.sounds.push(sound::Call::Bgm(bgm));
                game.fade.start(host.now_ms(), false, game.fade_seconds);
            }
        }
        if let Some(card) = game.pending_night.take() {
            game.night_spawn_heroine(host, client, &card)?;
            let cottage = game.night.as_ref().is_some_and(|n| n.cottage);
            let list = game.adv_assets.event(if cottage { "Before_Night_Cottage" } else { "Before_Night_NO_Cottage" }).into_iter().collect();
            game.log.push(format!("NightEvent {card}"));
            game.night_talk(&card, list, &[])?;
        }
        if game.h_change.is_some() && game.h.as_ref().is_some_and(|h| h.phase == MassagePhase::Play) {
            // `HScene.ChangePosture`: the pair again in the new posture.
            let ids = game.h.as_mut().map(|h| std::mem::take(&mut h.entities)).unwrap_or_default();
            let mut released = Vec::new();
            game.map_entities.retain(|(e, path)| {
                let gone = ids.contains(e);
                if gone {
                    released.push(path.clone());
                }
                !gone
            });
            for id in &ids {
                game.world.push(json!({"op": "despawn", "id": id}));
            }
            for path in released {
                host.release_model(client, &path)?;
            }
            if let (Some((pos, rot)), Some(card)) = (game.h_point, game.h.as_ref().map(|h| h.card.clone())) {
                let sun = game.map_sun;
                if let Err(e) = game.spawn_h_pair(host, client, &card, pos, rot, sun) {
                    game.log.push(format!("HScene.ChangePosture: {e:#}"));
                }
                if let Some(h) = game.h.as_mut() {
                    h.motion = "Idle";
                }
            }
        }
        if let Some(sun) = game.h_spawn_sun.take() {
            if let (Some((pos, rot)), Some(card)) = (game.h_point, game.h.as_ref().map(|h| h.card.clone())) {
                if let Err(e) = game.spawn_h_pair(host, client, &card, pos, rot, sun) {
                    game.log.push(format!("HScene: {e:#}"));
                }
            }
        }
        if !game.released.is_empty() {
            game.release_map(host, client)?;
        }

        // Layout and compose the build scene (slot 0) and each overlay.
        let mut ops = std::mem::take(&mut game.removed);
        let mut layouts = Vec::new();
        {
            let Game { unity, ui, level, overlays, font_cache, .. } = &mut *game;
            let mut hierarchies: Vec<(&mut Level, &mut Ui)> = vec![(level, ui)];
            hierarchies.extend(overlays.iter_mut().map(|(l, u)| (l, u)));
            for (slot, (level, ui)) in hierarchies.into_iter().enumerate() {
                let key = level.key.clone();
                let states = level.frame_states();
                let cell = RefCell::new((&mut *unity, &mut *ui, HostAdapter { host: &mut *host, client, fonts: &mut *font_cache }));
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
        if game.movie.is_some() {
            game.movie_update(now);
        }
        if game.h.is_some() && game.talk.is_none() {
            let clicked_scene = input.buttons_pressed & b::MOUSE_LEFT != 0 && handler.is_none() && !fading;
            game.h_update(clicked_scene, now);
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
        } else if input.buttons_pressed & b::MOUSE_LEFT != 0 && handler.is_none() && !fading && game.massage.is_none() && game.h.is_none() && game.night.is_none() && game.movie.is_none() && matches!(&game.controller, Controller::ActionMap(m) if matches!(m.phase, actionmap::Phase::Map(_))) {
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
        let mut night_talk_ended = false;
        let mut movie_start: Option<(String, i32, Vec<String>)> = None;
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
                        let is_h_scene = matches!(talk.program.vars.get("IsHScene"), Some(crate::adv::Var::Bool(true)));
                        game.last_talk = (talk.selected, is_h_scene);
                        night_talk_ended = game.night.is_some();
                        if let Some((card, kind)) = game.pending_points.take() {
                            game.apply_points(card, kind, talk.selected);
                        }
                        massage_talk_ended = true;
                        if let Some(m) = game.pending_movie.take() {
                            movie_start = Some(m);
                        } else if let Some(card) = game.pending_with.take() {
                            game.favorability_up(&card);
                            // `EndProc`: back to the action map after the movie.
                            if let Some(map) = game.with_return.take() {
                                let zone = if let Controller::ActionMap(m) = &game.controller { m.time_zone } else { 0 };
                                game.fade.start(now, true, game.fade_seconds);
                                game.pending_map = Some((map, zone));
                            }
                        }
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
        let mut h_button: Option<String> = None;
        let mut save_load_action: Option<crate::saveload::Action> = None;
        let mut h_point_action: Option<crate::hui::PointAction> = None;
        let mut open_save_load: Option<bool> = None;
        if let Some((h, button)) = click {
            if h > 0 {
                let level = &mut game.overlays[h - 1].0;
                game.log.push(format!("click {}", level.scene.path(button)));
                let calls = level.on_click(button);
                let script = level.invoke(&calls);
                if game.massage.is_some() && level.name == crate::oil::SCENE_PREFAB && level.scene.nodes[button].name == "Button Exit" {
                    massage_exit = true;
                }
                let point_action = if game.h.is_some() && level.name == crate::h::SCENE_PREFAB { game.h_point_ui.as_ref().map(|(ui, _)| ui.click(button)) } else { None };
                if let Some(a) = point_action {
                    h_point_action = Some(a);
                } else if game.h.is_some() && level.name == crate::h::SCENE_PREFAB {
                    let posture = game.h_ui.as_mut().map(|ui| ui.click(level, button));
                    match posture {
                        Some(crate::hui::Action::Change(file, id)) => game.h_change = Some((file, id)),
                        _ if level.scene.path(button).starts_with(crate::hui::ROOT) => {}
                        _ => h_button = Some(level.scene.nodes[button].name.clone()),
                    }
                }
                if level.name == crate::saveload::PREFAB {
                    if let Some(sl) = game.save_load.as_mut() {
                        save_load_action = Some(sl.click(level, button));
                    }
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
                        actionmap::Action::OpenSaveLoad => {
                            game.level.sounds.push(sound::Call::SystemSe(game.sound_ids.ok_s));
                            open_save_load = Some(false);
                        }
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
            game.h_talk_ended(now)?;
        }
        if night_talk_ended {
            game.night_talk_ended(now)?;
        }
        if let Some((card, id, after)) = movie_start {
            game.start_movie(&card, id, after, now);
        }
        if massage_exit {
            game.massage_exit()?;
        }
        if let Some(name) = h_button {
            game.h_button(&name, now)?;
        }
        if let Some(a) = h_point_action {
            game.h_point_action(a);
        }
        if let Some(a) = save_load_action {
            if let Err(e) = game.save_load_action(host, a, now) {
                game.log.push(format!("SaveLoadUI: {e:#}"));
            }
        }
        if let Some(is_title) = open_save_load {
            game.open_save_load(host, is_title)?;
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
            } else if name == "btnH" {
                // `EventManager.H`: agree, the opener, then the H scene (on
                // an H map when this one has no H place).
                let mut rng = game.rng;
                let mut random = |n: usize| game_random(&mut rng, n);
                let return_map = if let Controller::ActionMap(m) = &game.controller { if let actionmap::Phase::Map(id) = m.phase { id } else { 0 } } else { 0 };
                let here = game.h_point.is_some();
                list.extend(crate::commands::choose(&game.adv_assets.agree("H", fav_lv, false), &mut random));
                list.extend(crate::commands::choose(&game.adv_assets.h_start(here), &mut random));
                game.rng = rng;
                game.visit_played = true;
                game.h = Some(HRun { card: card.clone(), phase: MassagePhase::Agree, return_map, posture: None, entities: Vec::new(), motion: "Idle", motion_until: 0, gauge: 0., male_gauge: 0., finishes: Vec::new(), orgasms: 0, last_ms: now, night: None });
            } else if name == "btnWith" {
                // `EventManager.With`: the agree line, the short movie and
                // `_shortMoiveAfterAssets[LV]`, then the favourability level.
                // SATD-amanatsu-port: the ShortMovie timeline is not played.
                let mut rng = game.rng;
                let mut random = |n: usize| game_random(&mut rng, n);
                list.extend(crate::commands::choose(&game.adv_assets.agree("With", fav_lv, false), &mut random));
                let after: Vec<String> = crate::commands::choose(&game.adv_assets.short_movie_after(fav_lv), &mut random).into_iter().collect();
                game.rng = rng;
                game.visit_played = true;
                game.pending_with = Some(card.clone());
                // `tableSMPeronals[personality][LV]`.
                let personality = game.spots.get(spot).map_or(0, |s| s.2);
                match crate::sm::personal_movie(&mut game.unity, personality, fav_lv) {
                    Ok(Some(id)) => {
                        game.pending_movie = Some((card.clone(), id, after));
                        game.with_return = Some(game.current_map());
                    }
                    _ => list.extend(after),
                }
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
            scenes::Request::LoadSave if game.pending.is_none() => game.open_save_load(host, true)?,
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
/// A world point on the screen of a view (pixels), if in front of it.
fn project(v: &map::View, p: [f32; 3]) -> Option<[f32; 2]> {
    use glam::Vec3;
    let eye = Vec3::from_array(v.eye);
    let fwd = (Vec3::from_array(v.target) - eye).normalize();
    let right = fwd.cross(Vec3::from_array(v.up)).normalize();
    let up = right.cross(fwd);
    let th = (v.fov.to_radians() / 2.).tan();
    let d = Vec3::from_array(p) - eye;
    let z = d.dot(fwd);
    if z <= 0.01 {
        return None;
    }
    let x = d.dot(right) / (z * th * WIDTH / HEIGHT);
    let y = d.dot(up) / (z * th);
    let s = [(x + 1.) / 2. * WIDTH, (1. - y) / 2. * HEIGHT];
    ((0. ..=WIDTH).contains(&s[0]) && (0. ..=HEIGHT).contains(&s[1])).then_some(s)
}

/// `project`, or for a point outside the view the screen edge position
/// toward it (48 px margin).
fn project_edge(v: &map::View, p: [f32; 3]) -> [f32; 2] {
    use glam::Vec3;
    if let Some(s) = project(v, p) {
        return s;
    }
    let eye = Vec3::from_array(v.eye);
    let fwd = (Vec3::from_array(v.target) - eye).normalize();
    let right = fwd.cross(Vec3::from_array(v.up)).normalize();
    let up = right.cross(fwd);
    let d = Vec3::from_array(p) - eye;
    let dir = glam::Vec2::new(d.dot(right), -d.dot(up)).normalize_or(glam::Vec2::X);
    let (cx, cy, m) = (WIDTH / 2., HEIGHT / 2., 48.);
    let t = ((cx - m) / dir.x.abs().max(1e-6)).min((cy - m) / dir.y.abs().max(1e-6));
    [cx + dir.x * t, cy + dir.y * t]
}

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

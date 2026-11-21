// SPDX-License-Identifier: MPL-2.0
//! `Manager.MapManager`: the `map_info` lists (`ma/lis/*`), and a map as
//! world content — the additive map scene (`LoadMapObjectAsync` →
//! `Scene.LoadBaseSceneAsync`), its time-zone light setting
//! (`LightmapSettings_<map>_<zone>`: object on/off lists and fog) and the
//! move-point prefab's Cinemachine camera positions.
use crate::{tables::Table, unity::Unity};
use anyhow::{Context, Result};
use crysirror_unity::{
    gltf::mirror_point,
    managed::Value,
    scene6::Scene6,
    world6::{self, GlbOptions, GlbReport, RenderSettings6, SceneLight},
};
use glam::Vec3;
use serde_json::{json, Value as Json};

/// Prepared texture bytes a map may take of the session's 256 MiB
/// prepared-asset budget.
pub const MAP_TEXTURE_BUDGET: usize = 128 << 20;

/// A `MapListContent` row.
#[derive(Clone, Debug, PartialEq)]
pub struct MapInfo {
    pub id: i32,
    pub name: String,
    /// Scene bundle (`ma/pre/sc/…`) and scene name (`map000`).
    pub scene: (String, String),
    pub point: (String, String),
    /// Light setting bundle and asset prefix (`LightmapSettings_000_`).
    pub light: (String, String),
}

/// Every `map_info[…]` table of `lib/ma/lis`.
pub fn map_infos(unity: &mut Unity) -> Result<Vec<MapInfo>> {
    let mut out = Vec::new();
    let bundles: Vec<String> = unity.bundle_files().into_iter().filter_map(|k| k.strip_prefix("bundle:").and_then(|r| r.split_once('#')).map(|(b, _)| b.to_owned())).filter(|b| b.starts_with("lib/ma/lis/")).collect();
    for b in bundles {
        let stem = b.rsplit('/').next().unwrap_or(&b).trim_end_matches(".unity3d").to_owned();
        let Some((k, id)) = unity.bundle_asset(&b, &format!("map_info[{stem}]"), 49)? else { continue };
        let t = Table::parse(&unity.text_asset(&k, id)?)?;
        for r in &t.rows {
            let s = |i: usize| r.get(i).cloned().unwrap_or_default();
            // id, names (jp/en/zh-CN/zh-TW), then (manifest, bundle, asset)
            // triples: scene, navmesh, point, light setting, thumbnail.
            out.push(MapInfo {
                id: s(0).parse().with_context(|| format!("map_info id {:?}", r.first()))?,
                name: s(1),
                scene: (s(6), s(7)),
                point: (s(12), s(13)),
                light: (s(15), s(16)),
            });
        }
    }
    out.sort_by_key(|m| m.id);
    out.dedup_by_key(|m| m.id);
    Ok(out)
}

/// A camera in engine space.
#[derive(Clone, Debug, PartialEq)]
pub struct View {
    pub eye: [f32; 3],
    pub target: [f32; 3],
    pub up: [f32; 3],
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

impl View {
    pub fn command(&self) -> Json {
        json!({"op": "set_camera", "camera": {"eye": self.eye, "target": self.target, "up": self.up, "fov_y_degrees": self.fov, "near": self.near, "far": self.far}})
    }
}

/// A loaded map: GLB chunks and its light/camera state.
pub struct MapWorld {
    pub info: MapInfo,
    pub chunks: Vec<Vec<u8>>,
    pub report: GlbReport,
    pub sun: Option<SceneLight>,
    pub settings: RenderSettings6,
    /// Fog of the light setting (colour, start, end).
    pub fog: Option<([f32; 3], f32, f32)>,
    /// Cinemachine positions `Camera/Pos1..` of the move-point prefab with
    /// their `Priority`.
    pub cameras: Vec<(View, i64)>,
    /// `AL.ActionPoint`s of the move-point prefab: (name, Unity world
    /// position, world rotation).
    pub points: Vec<(String, [f32; 3], [f32; 4])>,
    pub log: Vec<String>,
}

/// Activate/deactivate scene objects by name (`LightMapDataObject.Change`).
fn toggle(scene: &mut Scene6, names: &[Value], on: bool) -> usize {
    let mut n = 0;
    for v in names {
        let Value::String(name) = v else { continue };
        for node in scene.nodes.iter_mut().filter(|x| &x.name == name) {
            node.active = on;
            n += 1;
        }
    }
    n
}

fn color3(v: Option<&Value>) -> Option<[f32; 3]> {
    match v {
        Some(Value::Struct(s)) => Some(["r", "g", "b"].map(|k| s.float(k).unwrap_or(0.) as f32)),
        _ => None,
    }
}

/// Unity world camera of a transform node as an engine view (`lens`:
/// Cinemachine `Lens` fields).
pub fn view(scene: &Scene6, node: usize, lens: Option<&crysirror_unity::managed::Fields>) -> View {
    let m = world6::world_matrix(scene, node);
    let eye = m.w_axis.truncate();
    let fwd = m.transform_vector3(Vec3::Z).normalize_or_zero();
    let up = m.transform_vector3(Vec3::Y).normalize_or_zero();
    let f = |n: &str, d: f64| lens.and_then(|l| l.float(n)).unwrap_or(d) as f32;
    View {
        eye: mirror_point(eye.to_array()),
        target: mirror_point((eye + fwd).to_array()),
        up: mirror_point(up.to_array()),
        fov: f("FieldOfView", 60.).clamp(1., 170.),
        near: f("NearClipPlane", 0.1).max(0.05),
        far: f("FarClipPlane", 1000.).min(5000.),
    }
}

/// `MapManager.ChangeMap(id, timeZone)`: the map scene with the zone's
/// light setting applied, as GLB chunks.
pub fn load(unity: &mut Unity, info: &MapInfo, time_zone: i32) -> Result<MapWorld> {
    let mut log = Vec::new();
    let key = format!("bundle:lib/{}#BuildPlayer-{}", info.scene.0, info.scene.1);
    let mut scene = unity.scene(&key).with_context(|| format!("map scene {}", info.scene.1))?;
    // Light setting of the time zone.
    let mut fog = None;
    let light_name = format!("{}{time_zone:02}", info.light.1);
    match unity.bundle_asset(&format!("lib/{}", info.light.0), &light_name, crysirror_unity::class_id::MONO_BEHAVIOUR)? {
        Some((k, id)) => {
            let f = unity.decode(&k, id)?;
            let off = toggle(&mut scene, f.array("_offObjectNames").unwrap_or(&[]), false);
            let on = toggle(&mut scene, f.array("_onObjectNames").unwrap_or(&[]), true);
            log.push(format!("{light_name}: {on} objects on, {off} off"));
            if f.int("_fog").unwrap_or(0) != 0 {
                fog = color3(f.get("_fogColor")).map(|c| (c, f.float("_fogStartDistance").unwrap_or(0.) as f32, f.float("_fogEndDistance").unwrap_or(1000.) as f32));
            }
            // SATD-amanatsu-port: lightmaps, reflection probes and the
            // Azure sky time of day of the setting are not applied.
        }
        None => log.push(format!("light setting {light_name} not found")),
    }
    let file = unity.file(&key)?;
    let settings = file
        .objects
        .iter()
        .find(|o| o.class_id == crysirror_unity::class_id::RENDER_SETTINGS)
        .copied()
        .map(|o| unity.decode(&key, o.path_id).map(|f| RenderSettings6::decode(&f)))
        .transpose()?
        .unwrap_or_default();
    // The key light: the brightest enabled directional light that lights
    // the Default layer.
    let sun = world6::scene_lights(&scene).into_iter().filter(|l| l.kind == 1 && l.culling_mask & 1 != 0 && l.intensity > 0.01).max_by(|a, b| a.intensity.total_cmp(&b.intensity));
    let roots = scene.roots.clone();
    // SATD-amanatsu-port: one texture budget for the map; characters and
    // the rest of the session share the player's prepared-asset budget.
    let options = GlbOptions { max_texture: 1024, texture_budget: MAP_TEXTURE_BUDGET };
    let (chunks, report) = world6::scene_glb(&scene, &key, &roots, unity, &options, "CrySirror AmanatsuLocation map importer")?;
    // Move-point prefab cameras.
    let mut cameras = Vec::new();
    let mut action_points = Vec::new();
    match crate::level::Level::prefab(unity, &format!("lib/{}", info.point.0), &info.point.1) {
        Ok(points) => {
            for (i, n) in points.scene.nodes.iter().enumerate() {
                if n.components.iter().any(|c| c.script == "AL.ActionPoint") {
                    let (_, rot, pos) = world6::world_matrix(&points.scene, i).to_scale_rotation_translation();
                    action_points.push((n.name.clone(), pos.to_array(), rot.to_array()));
                }
            }
            for i in 1..=9 {
                let Some(n) = points.scene.find(&format!("{}/Camera/Pos{i}/CinemachineCamera", points.scene.nodes[0].name)) else { break };
                let cam = points.scene.nodes[n].components.iter().find(|c| c.script.ends_with("CinemachineCamera"));
                let lens = cam.and_then(|c| c.fields.fields("Lens"));
                let priority = cam.and_then(|c| c.fields.fields("Priority")).filter(|p| p.int("Enabled").unwrap_or(0) != 0).and_then(|p| p.int("m_Value")).unwrap_or(0);
                cameras.push((view(&points.scene, n, lens), priority));
            }
        }
        Err(e) => log.push(format!("move points {}: {e:#}", info.point.1)),
    }
    Ok(MapWorld { info: info.clone(), chunks, report, sun, settings, fog, cameras, points: action_points, log })
}

impl MapWorld {
    /// The live Cinemachine camera: the highest priority (the first of
    /// equal priorities).
    pub fn camera(&self) -> Option<&View> {
        let best = self.cameras.iter().map(|c| c.1).max()?;
        self.cameras.iter().find(|c| c.1 == best).map(|c| &c.0)
    }

    /// `set_environment` of the map: Unity forward lighting without
    /// tonemapping, the key light and the RenderSettings ambient.
    pub fn environment(&self) -> Json {
        let sun = self.sun.as_ref().map(|l| {
            let d = mirror_point(l.direction);
            json!({"direction": d, "color": l.color.map(|c| c.clamp(0., 4.)), "intensity": l.intensity.clamp(0., 8.), "shadow_strength": 0.6})
        });
        let mut env = json!({
            "show_sky": true, "intensity": 1.0, "bloom": 0.02, "tonemap": "none", "ambient_occlusion": 0.0,
            "ambient": self.settings.ambient_linear(),
            "sun": sun,
            "shadow_distance": 80.0,
        });
        if let Some((c, start, end)) = self.fog {
            let decode = |v: f32| if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
            // Unity linear fog (0 at `start`, 1 at `end`) as exponential
            // extinction matching it at mid-distance (≈78% at `end`).
            let density = 1.5 / (end - start).max(1.);
            env["fog"] = json!({"color": c.map(decode), "density": density, "height_falloff": 0.0, "start_distance": start, "max_opacity": 0.85});
        }
        json!({"op": "set_environment", "environment": env})
    }
}

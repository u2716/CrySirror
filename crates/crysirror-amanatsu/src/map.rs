// SPDX-License-Identifier: MPL-2.0
//! `Manager.MapManager`: the `map_info` lists (`ma/lis/*`), and a map as
//! world content — the additive map scene (`LoadMapObjectAsync` →
//! `Scene.LoadBaseSceneAsync`), its time-zone light setting
//! (`LightmapSettings_<map>_<zone>`: object on/off lists and fog) and the
//! move-point prefab's Cinemachine camera positions.
pub mod wandering;

use crate::{tables::Table, unity::Unity};
use anyhow::{Context, Result};
use crysirror_unity::{
    gltf::mirror_point,
    managed::Value,
    scene6::Scene6,
    world6::{self, GlbOptions, GlbReport, RenderSettings6, SceneLight},
};
use glam::Vec3;
use std::collections::HashMap;
use serde_json::{json, Value as Json};

/// Prepared texture bytes a map may take of the session's 256 MiB
/// prepared-asset budget.
pub const MAP_TEXTURE_BUDGET: usize = 128 << 20;

/// Culling-mask bits of the layers map renderers use (Default, 4, 7, 8;
/// the map lights' masks are 0x180, 0x190, 0x900, 0x980).
const MAP_LAYERS: u32 = 0x1 | 0x10 | 0x80 | 0x100;
/// Layer mask of the character lights (`Directional Light Sun Chara`).
const CHARA_LAYER: u32 = 0x80;

/// A `MapListContent` row.
#[derive(Clone, Debug, PartialEq)]
pub struct MapInfo {
    pub id: i32,
    pub name: String,
    /// Scene bundle (`ma/pre/sc/…`) and scene name (`map000`).
    pub scene: (String, String),
    pub navigation: (String, String),
    pub point: (String, String),
    /// Light setting bundle and asset prefix (`LightmapSettings_000_`).
    pub light: (String, String),
}

/// `MapManager.LoadCameraLight(map, zone, camera)`: the `camera_light[…]`
/// row of the map (`id, manifest, bundle, asset prefix`) names the prefab
/// `<prefix><zone:00>` that is instantiated under the camera; its
/// character-layer directional light with the `CameraLightCtrl` intensity
/// (`CharaLightInfo.SetIntensity(true)`). The direction is in the
/// prefab's parent space, i.e. Unity camera space.
/// SATD-amanatsu-port: the 0.2 s intensity fades of a camera change and
/// further lights of the prefab are not applied.
pub fn camera_light(unity: &mut Unity, map: i32, zone: i32) -> Result<Option<SceneLight>> {
    let bundles: Vec<String> = unity.bundle_files().into_iter().filter_map(|k| k.strip_prefix("bundle:").and_then(|r| r.split_once('#')).map(|(b, _)| b.to_owned())).filter(|b| b.starts_with("lib/ma/lis/")).collect();
    let mut row = None;
    for b in bundles {
        let stem = b.rsplit('/').next().unwrap_or(&b).trim_end_matches(".unity3d").to_owned();
        let Some((k, id)) = unity.bundle_asset(&b, &format!("camera_light[{stem}]"), 49)? else { continue };
        let t = Table::parse(&unity.text_asset(&k, id)?)?;
        if let Some(r) = t.rows.iter().find(|r| r.first().and_then(|v| v.parse::<i32>().ok()) == Some(map)) {
            row = Some((r.get(2).cloned().unwrap_or_default(), r.get(3).cloned().unwrap_or_default()));
        }
    }
    let Some((bundle, prefix)) = row else { return Ok(None) };
    let prefab = crate::level::Level::prefab(unity, &format!("lib/{bundle}"), &format!("{prefix}{zone:02}"))?;
    let scene = &prefab.scene;
    let mut intensity: HashMap<usize, f32> = HashMap::new();
    for n in &scene.nodes {
        for c in n.components.iter().filter(|c| c.script == "AL.CameraLightCtrl") {
            let infos = c.fields.fields("_cameraLightInfo").and_then(|f| f.array("Infos")).unwrap_or(&[]);
            for info in infos {
                let Value::Struct(f) = info else { continue };
                let (Some(light), Some(k)) = (f.pptr("Light"), f.float("Intensity")) else { continue };
                if let Some(node) = scene.nodes.iter().position(|n| n.components.iter().any(|c| c.path_id == light.path_id)) {
                    intensity.insert(node, k as f32);
                }
            }
        }
    }
    Ok(world6::scene_lights(scene).into_iter().filter(|l| l.kind == 1 && l.culling_mask & CHARA_LAYER != 0).map(|mut l| {
        if let Some(k) = intensity.get(&l.node) {
            l.intensity = *k;
        }
        l
    }).filter(|l| l.intensity > 0.01).max_by(|a, b| a.intensity.total_cmp(&b.intensity)))
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
                navigation: (s(9), s(10)),
                point: (s(12), s(13)),
                light: (s(15), s(16)),
            });
        }
    }
    out.sort_by_key(|m| m.id);
    out.dedup_by_key(|m| m.id);
    Ok(out)
}

/// Post-processing of the map's active global `Volume` profile.
#[derive(Clone, Debug, PartialEq)]
pub struct Post {
    pub aces: bool,
    /// Exposure in stops (`tonemapExposurePre`).
    pub exposure: f32,
    pub saturation: f32,
    pub contrast: f32,
    pub color_filter: [f32; 3],
    pub bloom: f32,
}

impl Default for Post {
    fn default() -> Self {
        Self { aces: false, exposure: 0., saturation: 1., contrast: 1., color_filter: [1.; 3], bloom: 0.02 }
    }
}

/// Linear RGB of a black body at `kelvin` relative to 6600 K (white), the
/// colour temperature filter of Beautify (Tanner Helland's fit).
fn kelvin_filter(kelvin: f32) -> [f32; 3] {
    let rgb = |k: f32| -> [f32; 3] {
        let t = k / 100.;
        let r = if t <= 66. { 255. } else { 329.698_73 * (t - 60.).powf(-0.133_204_76) };
        let g = if t <= 66. { 99.470_8 * t.ln() - 161.119_57 } else { 288.122_16 * (t - 60.).powf(-0.075_514_85) };
        let b = if t >= 66. { 255. } else if t <= 19. { 0. } else { 138.517_73 * (t - 10.).ln() - 305.044_8 };
        [r, g, b].map(|c| (c / 255.).clamp(0.01, 1.))
    };
    let (a, w) = (rgb(kelvin.clamp(1000., 40000.)), rgb(6600.));
    [a[0] / w[0], a[1] / w[1], a[2] / w[2]]
}

/// The first enabled global `Volume` of the active nodes.
fn volume_post(unity: &mut Unity, key: &str, scene: &Scene6) -> Option<Post> {
    let active: Vec<bool> = (0..scene.nodes.len())
        .map(|i| {
            let mut n = Some(i);
            while let Some(k) = n {
                if !scene.nodes[k].active {
                    return false;
                }
                n = scene.nodes[k].parent;
            }
            true
        })
        .collect();
    let profile = scene.nodes.iter().enumerate().filter(|(i, _)| active[*i]).flat_map(|(_, n)| &n.components).find(|c| c.script.ends_with("Rendering.Volume") && c.fields.int("m_IsGlobal").unwrap_or(0) != 0 && c.fields.int("m_Enabled").unwrap_or(1) != 0)?.fields.pptr("sharedProfile")?;
    let (pk, pid) = unity.target(key, profile).ok()?;
    let prof = unity.decode(&pk, pid).ok()?;
    let mut post = Post::default();
    let over = |f: &crysirror_unity::managed::Fields, name: &str| -> Option<f32> {
        let v = f.fields(name)?;
        (v.int("m_OverrideState").unwrap_or(0) != 0).then(|| v.float("m_Value").map(|x| x as f32).or_else(|| v.int("m_Value").map(|x| x as f32)))?
    };
    for c in prof.array("components").unwrap_or(&[]) {
        let Value::PPtr(c) = c else { continue };
        let Ok((ck, cid)) = unity.target(&pk, *c) else { continue };
        let Ok(f) = unity.decode(&ck, cid) else { continue };
        if f.int("active").unwrap_or(1) == 0 {
            continue;
        }
        match f.string("m_Name").unwrap_or("") {
            "Beautify" => {
                if over(&f, "disabled").unwrap_or(0.) != 0. {
                    continue;
                }
                if let Some(t) = over(&f, "tonemap") {
                    post.aces = t > 0.;
                }
                if let Some(e) = over(&f, "tonemapExposurePre") {
                    post.exposure = e.max(0.01).log2();
                }
                // Beautify `saturate` is a vibrance boost (0 neutral).
                if let Some(v) = over(&f, "saturate") {
                    post.saturation = (1. + v * 0.12).clamp(0., 4.);
                }
                if let Some(v) = over(&f, "contrast") {
                    post.contrast = v.clamp(0., 4.);
                }
                if let (Some(k), Some(b)) = (over(&f, "colorTemp"), over(&f, "colorTempBlend")) {
                    let t = kelvin_filter(k);
                    post.color_filter = [0, 1, 2].map(|i| 1. + (t[i] - 1.) * b.clamp(0., 1.));
                }
            }
            "Bloom" => {
                if let Some(i) = over(&f, "intensity") {
                    post.bloom = (i * 0.06).clamp(0., 1.);
                }
            }
            _ => {}
        }
    }
    Some(post)
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
    /// The character key light (`Directional Light Sun Chara`): the
    /// brightest enabled directional light of the character layer (7).
    pub chara_light: Option<SceneLight>,
    /// `MapManager.LoadCameraLight`: the zone's camera light prefab
    /// (instantiated under the camera); its direction is in Unity camera
    /// space.
    pub camera_light: Option<SceneLight>,
    /// Point and spot lights of the active light groups (lamps of the
    /// evening/night settings).
    pub lights: Vec<SceneLight>,
    /// The time-of-day sky of the light setting's Azure parameters.
    pub sky: Option<crysirror_asset::sky::GradientSky>,
    pub settings: RenderSettings6,
    /// The active global Volume's post settings (Beautify + URP Bloom).
    pub post: Post,
    /// Fog of the light setting (colour, start, end).
    pub fog: Option<([f32; 3], f32, f32)>,
    /// Cinemachine positions `Camera/Pos1..` of the move-point prefab with
    /// their `Priority`.
    pub cameras: Vec<(View, i64)>,
    /// `AL.ActionPoint`s of the move-point prefab: (name, Unity world
    /// position, world rotation).
    pub points: Vec<(String, [f32; 3], [f32; 4])>,
    /// The points referenced by MovePointInfo._wandering, in authored order.
    pub wander_points: Vec<wandering::Point>,
    /// ActionPoint._animIDList by point name.
    pub point_anims: HashMap<String, Vec<i32>>,
    /// `ActionPoint`s of kind 1 (`GetShortMoviePoint`): short movie id,
    /// position, rotation.
    pub short_movies: Vec<(i32, [f32; 3], [f32; 4])>,
    /// `AL.ItemOutlinable` objects of the move-point prefab (the rest
    /// items of `EventManager.Item`): name, Unity world position.
    pub items: Vec<(String, [f32; 3])>,
    /// Particle systems of the active nodes as engine VFX systems.
    pub particles: Vec<MapParticles>,
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

/// The light setting's Azure sky (`_azure`: `_csTimeSystem._timeLine` hour
/// and `_latitube`; `_DefaultPreset.Scat` `_rayleighColor`/`_mieColor`
/// gradients over the day) as a gradient sky. The sun elevation follows the
/// hour angle at the latitude (declination 0); the glow sits toward the
/// map's key light.
///
/// SATD-amanatsu-port: Azure's Rayleigh/Mie scattering integral, clouds,
/// moon and stars are not evaluated; zenith/horizon radiances are a fit
/// scaled by the gradient tints.
pub fn azure_sky(azure: &crysirror_unity::managed::Fields, sun: Option<&SceneLight>) -> crysirror_asset::sky::GradientSky {
    let time = azure.fields("_csTimeSystem");
    let hour = time.and_then(|t| t.float("_timeLine")).unwrap_or(12.) as f32;
    let lat = time.and_then(|t| t.float("_latitube")).unwrap_or(0.) as f32;
    let elevation = (lat.to_radians().cos() * ((hour - 12.) * 15.).to_radians().cos()).clamp(-1., 1.);
    let day = ((elevation + 0.1) / 0.25).clamp(0., 1.);
    let day = day * day * (3. - 2. * day);
    let scat = azure.fields("_DefaultPreset").and_then(|p| p.fields("Scat"));
    let tint = |name: &str| scat.and_then(|s| s.fields(name)).and_then(|g| world6::gradient(g, hour / 24.)).unwrap_or([1.; 3]);
    let (rayleigh, mie) = (tint("_rayleighColor"), tint("_mieColor"));
    let mix = |night: [f32; 3], dayc: [f32; 3], t: [f32; 3], k: f32| [0, 1, 2].map(|i| night[i] + (dayc[i] * t[i] * k - night[i]) * day);
    let zenith = mix([0.008, 0.012, 0.03], [0.2, 0.42, 1.0], rayleigh, 1.3);
    let mut horizon = mix([0.02, 0.025, 0.045], [0.75, 0.82, 0.95], rayleigh, 1.2);
    // Low sun: the horizon takes the Mie colour (sunset band).
    let low = (1. - elevation.abs() * 3.).clamp(0., 1.) * day;
    for i in 0..3 {
        horizon[i] += mie[i] * low * 0.9;
    }
    let sun_dir = sun.map_or([0., elevation, (1. - elevation * elevation).max(0.).sqrt()], |l| {
        let d = mirror_point(l.direction);
        [-d[0], -d[1], -d[2]]
    });
    crysirror_asset::sky::GradientSky {
        sun: sun_dir,
        zenith,
        horizon,
        ground: horizon.map(|c| c * 0.3),
        glow: mie.map(|c| c * 3. * day),
        glow_power: 12.,
    }
}

/// `MapManager.ChangeMap(id, timeZone)`: the map scene with the zone's
/// light setting applied, as GLB chunks.
pub fn load(unity: &mut Unity, info: &MapInfo, time_zone: i32) -> Result<MapWorld> {
    load_with_budget(unity, info, time_zone, MAP_TEXTURE_BUDGET)
}

/// A map particle system with its `_MainTex` (cache key, pixels).
#[derive(Clone, Debug)]
pub struct MapParticles {
    pub name: String,
    pub import: crysirror_unity::particles::ParticleImport,
    pub texture: Option<(String, crysirror_image::ImageSurface)>,
}

/// The particle systems of the map's active nodes. Azure's weather
/// effects (`AzureCoreSystem/VisualFX`) emit only under its weather
/// profiles and are left out with the weather.
fn map_particles(unity: &mut Unity, key: &str, scene: &Scene6, log: &mut Vec<String>) -> Vec<MapParticles> {
    use crysirror_unity::particles;
    let mut out = Vec::new();
    for (n, p) in particles::scene_particles(scene).iter().enumerate() {
        let name = scene.path(p.node);
        if name.contains("/AzureCoreSystem/") {
            // SATD-amanatsu-port: Azure weather (rain, storm, ripples).
            continue;
        }
        let material = p.renderer.and_then(particles::renderer_material).and_then(|m| {
            let (f, o) = unity.target(key, m).ok()?;
            let mf = unity.decode(&f, o).ok()?;
            let shader = mf.pptr("m_Shader").and_then(|s| unity.target(&f, s).ok()).and_then(|(sf, so)| unity.decode(&sf, so).ok()).map(|s| world6::shader_name(&s)).unwrap_or_default();
            Some((f, world6::UnityMaterial::decode(&mf, shader)))
        });
        let seed = name.bytes().fold(0x811c_9dc5u32, |h, b| (h ^ u32::from(b)).wrapping_mul(0x0100_0193));
        match particles::particle_system(scene, p, material.as_ref().map(|m| &m.1), seed) {
            Ok(import) => {
                let texture = import.texture.zip(material.as_ref()).and_then(|(t, (f, _))| {
                    let (tf, to) = unity.target(f, t).ok()?;
                    let img = unity.texture(&tf, to).map_err(|e| log.push(format!("particle {name} texture: {e:#}"))).ok()?;
                    (img.width <= 2048 && img.height <= 2048).then(|| (format!("{tf}#{to}"), img))
                });
                if !import.notes.is_empty() {
                    log.push(format!("particle {name}: {}", import.notes.join("; ")));
                }
                out.push(MapParticles { name: format!("{name}#{n}"), import, texture });
            }
            Err(e) => log.push(format!("particle {name}: {e:#}")),
        }
    }
    out
}

/// LightmapSettingsSaveData.Update matches each renderer by its exact name,
/// then resolves the lightmap index by the registered colour texture identity.
fn receiver_lightmaps(unity: &mut Unity, file: &str, fields: &crysirror_unity::managed::Fields, scene: &Scene6) -> Result<HashMap<usize, world6::RendererLightmap>> {
    let mut receivers = HashMap::new();
    for row in fields.array("_lightmapIndices").unwrap_or_default() {
        let Value::Struct(row) = row else { anyhow::bail!("invalid lightmap receiver") };
        let name = row.string("ObjectName").context("lightmap receiver name")?;
        let index = row.int("LightmapIndex").context("lightmap receiver index")?;
        if index < 0 { continue }
        let mut texture = |array| -> Result<(String, i64)> {
            let Some(Value::PPtr(p)) = fields.array(array).and_then(|values| values.get(index as usize)) else {
                anyhow::bail!("{name} lacks {array}[{index}]")
            };
            unity.target(file, *p)
        };
        let radiance = texture("_colorLightmaps")?;
        let direction = texture("_dirLightmaps")?;
        let tiling_offset = match row.get("LightmapTilingOffset") {
            Some(Value::Floats(v)) => v.as_slice().try_into().context("lightmap ST length")?,
            Some(Value::Struct(v)) => [v.float("x"), v.float("y"), v.float("z"), v.float("w")].map(|x| x.unwrap_or(f64::NAN) as f32),
            _ => anyhow::bail!("missing lightmap ST for {name}"),
        };
        anyhow::ensure!(tiling_offset.iter().all(|x| x.is_finite()), "invalid lightmap ST for {name}");
        for (node, obj) in scene.nodes.iter().enumerate().filter(|(_, obj)| obj.name == name) {
            if obj.components.iter().any(|c| c.class_id == crysirror_unity::class_id::MESH_RENDERER) {
                receivers.insert(node, world6::RendererLightmap { radiance: radiance.clone(), direction: direction.clone(), tiling_offset });
            }
        }
    }
    Ok(receivers)
}

/// `load` with a texture budget (scenes with two characters, such as H,
/// leave them room in the prepared-asset budget).
pub fn load_with_budget(unity: &mut Unity, info: &MapInfo, time_zone: i32, texture_budget: usize) -> Result<MapWorld> {
    let mut log = Vec::new();
    let key = format!("bundle:lib/{}#BuildPlayer-{}", info.scene.0, info.scene.1);
    let mut scene = unity.scene(&key).with_context(|| format!("map scene {}", info.scene.1))?;
    // Light setting of the time zone.
    let mut fog = None;
    let mut azure = None;
    let mut lightmaps = HashMap::new();
    let light_name = format!("{}{time_zone:02}", info.light.1);
    match unity.bundle_asset(&format!("lib/{}", info.light.0), &light_name, crysirror_unity::class_id::MONO_BEHAVIOUR)? {
        Some((k, id)) => {
            let f = unity.decode(&k, id)?;
            let off = toggle(&mut scene, f.array("_offObjectNames").unwrap_or(&[]), false);
            let on = toggle(&mut scene, f.array("_onObjectNames").unwrap_or(&[]), true);
            log.push(format!("{light_name}: {on} objects on, {off} off"));
            azure = f.fields("_azure").cloned();
            if f.int("_fog").unwrap_or(0) != 0 {
                fog = color3(f.get("_fogColor")).map(|c| (c, f.float("_fogStartDistance").unwrap_or(0.) as f32, f.float("_fogEndDistance").unwrap_or(1000.) as f32));
            }
            lightmaps = receiver_lightmaps(unity, &k, &f, &scene)?;
            log.push(format!("{light_name}: {} directional lightmap receivers", lightmaps.len()));
        }
        None => log.push(format!("light setting {light_name} not found")),
    }
    let post = match volume_post(unity, &key, &scene) {
        Some(p) => {
            log.push(format!("volume post: {p:?}"));
            p
        }
        None => {
            log.push("no active global Volume".into());
            Post::default()
        }
    };
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
    // SATD-amanatsu-port: further character-layer directional lights
    // (`Directional Light C`, a weaker top fill) are not applied.
    let chara_light = world6::scene_lights(&scene).into_iter().filter(|l| l.kind == 1 && l.culling_mask & CHARA_LAYER != 0 && l.intensity > 0.01).max_by(|a, b| a.intensity.total_cmp(&b.intensity));
    log.push(format!("character light: {:?}", chara_light.as_ref().map(|l| (scene.path(l.node), l.intensity))));
    // Map renderers sit on the Default, water (4) and map (7, 8) layers.
    let lights: Vec<SceneLight> = world6::scene_lights(&scene).into_iter().filter(|l| matches!(l.kind, 0 | 2) && l.culling_mask & MAP_LAYERS != 0 && l.intensity > 0.01 && l.range > 0.).collect();
    log.push(format!("{} local lights", lights.len()));
    let roots = scene.roots.clone();
    // SATD-amanatsu-port: one texture budget for the map; characters and
    // the rest of the session share the player's prepared-asset budget.
    let options = GlbOptions { max_texture: 1024, texture_budget, spring_unit: 10., morph_targets: false };
    let (chunks, report) = world6::scene_glb_with_lightmaps(&scene, &key, &roots, unity, &options, "CrySirror AmanatsuLocation map importer", &lightmaps)?;
    // Move-point prefab cameras.
    let mut cameras = Vec::new();
    let mut action_points = Vec::new();
    let mut wander_points = Vec::new();
    let mut short_movies = Vec::new();
    let mut items = Vec::new();
    let mut point_anims = HashMap::new();
    match crate::level::Level::prefab(unity, &format!("lib/{}", info.point.0), &info.point.1) {
        Ok(points) => {
            wander_points = wandering::points(&points.scene)?;
            for (i, n) in points.scene.nodes.iter().enumerate() {
                if n.components.iter().any(|c| c.script == "AL.ItemOutlinable") {
                    let pos = world6::world_matrix(&points.scene, i).w_axis.truncate();
                    items.push((n.name.clone(), pos.to_array()));
                }
                if n.components.iter().any(|c| c.script == "AL.ActionPoint") {
                    let (_, rot, pos) = world6::world_matrix(&points.scene, i).to_scale_rotation_translation();
                    action_points.push((n.name.clone(), pos.to_array(), rot.to_array()));
                    if let Some(list) = n.components.iter().find(|c| c.script == "AL.ActionPoint").and_then(|c| c.fields.array("_animIDList")) {
                        let ids: Vec<i32> = list.iter().filter_map(|v| if let Value::Int(i) = v { Some(*i as i32) } else { None }).collect();
                        point_anims.insert(n.name.clone(), ids);
                    }
                    if let Some(c) = n.components.iter().find(|c| c.script == "AL.ActionPoint").filter(|c| c.fields.int("_kind") == Some(1)) {
                        short_movies.push((c.fields.int("_shortMoiveID").unwrap_or(0) as i32, pos.to_array(), rot.to_array()));
                    }
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
    let particles = map_particles(unity, &key, &scene, &mut log);
    log.push(format!("{} particle systems", particles.len()));
    let sky = azure.as_ref().map(|a| azure_sky(a, sun.as_ref()));
    if let Some(s) = &sky {
        log.push(format!("Azure sky: zenith {:?} horizon {:?}", s.zenith, s.horizon));
    }
    let camera_light = match camera_light(unity, info.id, time_zone) {
        Ok(l) => l,
        Err(e) => {
            log.push(format!("camera light: {e:#}"));
            None
        }
    };
    log.push(format!("camera light: {:?}", camera_light.as_ref().map(|l| (l.direction, l.color, l.intensity))));
    Ok(MapWorld { info: info.clone(), chunks, report, sun, chara_light, camera_light, lights, sky, settings, post, fog, cameras, points: action_points, wander_points, point_anims, short_movies, items, particles, log })
}

impl MapWorld {
    /// Intensity of the light characters are shaded with.
    pub fn chara_key(&self) -> f32 {
        self.chara_light.as_ref().or(self.sun.as_ref()).map_or(1., |l| l.intensity)
    }

    /// Engine entity specs of the local lights (URP: candela-like intensity
    /// with inverse-square falloff inside `range`; spot cones along the
    /// light's forward, the engine's local −Z).
    pub fn light_entities(&self, first_id: u32) -> Vec<Json> {
        self.lights
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let pos = mirror_point(l.position);
                let dir = Vec3::from_array(mirror_point(l.direction)).normalize_or_zero();
                let q = glam::Quat::from_rotation_arc(Vec3::NEG_Z, if dir == Vec3::ZERO { Vec3::NEG_Z } else { dir });
                let (x, y, z) = q.to_euler(glam::EulerRot::XYZ);
                let shape = if l.kind == 0 {
                    json!({"kind": "spot", "inner_degrees": (l.inner_spot_angle * 0.5).clamp(0., 89.), "outer_degrees": (l.spot_angle * 0.5).clamp(0.5, 89.)})
                } else {
                    json!({"kind": "point"})
                };
                json!({"op": "spawn", "entity": {
                    "id": first_id + i as u32,
                    "name": format!("light{i}"),
                    "transform": {"translation": pos, "rotation_degrees": [x.to_degrees(), y.to_degrees(), z.to_degrees()], "scale": [1.0, 1.0, 1.0]},
                    "light": {"shape": shape, "color": l.color.map(|c| c.clamp(0., 4.)), "intensity": l.intensity.clamp(0., 1e7), "units": "candela", "range": l.range.clamp(0.01, 2000.), "shadows": false},
                }})
            })
            .collect()
    }

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
        // SATD-amanatsu-port: Beautify sharpen, depth of field, film grain,
        // chromatic aberration, vignette and ShadowsMidtonesHighlights are
        // not converted.
        let p = &self.post;
        let mut env = json!({
            "show_sky": true, "intensity": 1.0, "bloom": p.bloom, "tonemap": if p.aces { "aces" } else { "none" }, "exposure": p.exposure, "ambient_occlusion": 0.0,
            "ambient": self.settings.ambient_linear(),
            "sun": sun,
            "toon_light": self.chara_light.as_ref().map(|l| json!({"direction": mirror_point(l.direction), "color": l.color.map(|c| c.clamp(0., 4.)), "intensity": l.intensity.clamp(0., 8.)})),
            "shadow_distance": 80.0,
            "grading": {"color_filter": p.color_filter, "contrast": p.contrast, "saturation": p.saturation},
        });
        // The camera light keys the characters while it exists (Unity
        // camera space → engine view space: z flips).
        if let Some(l) = &self.camera_light {
            let d = l.direction;
            env["toon_light"] = json!({"direction": [d[0], d[1], -d[2]], "color": l.color.map(|c| c.clamp(0., 4.)), "intensity": l.intensity.clamp(0., 8.)});
            env["toon_light_camera"] = json!(true);
        }
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

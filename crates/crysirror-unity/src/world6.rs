// SPDX-License-Identifier: MPL-2.0
//! Unity 2020+ 3D content as engine world assets: `Material` objects as
//! engine material data and GameObject hierarchies (MeshFilter +
//! MeshRenderer, LOD groups, static batches) as GLB chunks for the world
//! renderer, plus the lights and `RenderSettings` a level declares.
//!
//! Shader families map onto the engine's PBR and toon models; shaders
//! without an engine equivalent are approximated or skipped and recorded in
//! the GLB `dropped` notes so callers can report them.
use crate::{
    gltf::GlbBuilder,
    managed::{Fields, Value},
    scene6::{Component, Scene6},
    unity6::Mesh6,
    PPtr,
};
use anyhow::Result;
use crysirror_asset::pbr::{AlphaMode, PbrMaterial};
use crysirror_scene::material::NormalEncoding;
use crysirror_image::ImageSurface;
use glam::{Mat4, Quat, Vec3};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub const MESH_RENDERER: i32 = 23;
pub const MESH_FILTER: i32 = 33;
pub const SKINNED_MESH_RENDERER: i32 = 137;
pub const LIGHT: i32 = 108;
pub const LOD_GROUP: i32 = 205;
pub const SHADER: i32 = 48;

/// A material texture slot (`m_TexEnvs` entry).
#[derive(Clone, Debug, PartialEq)]
pub struct TexEnv {
    pub texture: PPtr,
    pub scale: [f32; 2],
    pub offset: [f32; 2],
}

/// A decoded Unity `Material` with its shader name.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UnityMaterial {
    pub name: String,
    /// Serialized file holding the material (its texture references
    /// resolve against it); empty when decoded without one.
    pub file: String,
    pub shader: String,
    pub textures: HashMap<String, TexEnv>,
    pub floats: HashMap<String, f32>,
    pub colors: HashMap<String, [f32; 4]>,
    pub keywords: Vec<String>,
    pub render_queue: i32,
}

fn xy(v: Option<&Value>, default: [f32; 2]) -> [f32; 2] {
    match v {
        Some(Value::Struct(s)) => [s.float("x").map_or(default[0], |x| x as f32), s.float("y").map_or(default[1], |y| y as f32)],
        _ => default,
    }
}

fn rgba(v: Option<&Value>) -> Option<[f32; 4]> {
    match v {
        Some(Value::Struct(s)) => Some(["r", "g", "b", "a"].map(|k| s.float(k).unwrap_or(0.) as f32)),
        _ => None,
    }
}

fn number(v: Option<&Value>) -> Option<f32> {
    match v {
        Some(Value::Float(f)) => Some(*f as f32),
        Some(Value::Int(i)) => Some(*i as f32),
        Some(Value::Bool(b)) => Some(f32::from(u8::from(*b))),
        _ => None,
    }
}

/// `m_ParsedForm.m_Name` of a `Shader` object.
pub fn shader_name(f: &Fields) -> String {
    f.fields("m_ParsedForm").and_then(|p| p.string("m_Name")).or_else(|| f.string("m_Name")).unwrap_or("").to_owned()
}

impl UnityMaterial {
    pub fn decode(f: &Fields, shader: String) -> Self {
        let mut m = UnityMaterial { name: f.string("m_Name").unwrap_or("").to_owned(), shader, render_queue: f.int("m_CustomRenderQueue").unwrap_or(-1) as i32, ..Default::default() };
        m.keywords = f.array("m_ValidKeywords").unwrap_or(&[]).iter().filter_map(|k| if let Value::String(s) = k { Some(s.clone()) } else { None }).collect();
        let Some(props) = f.fields("m_SavedProperties") else { return m };
        let entries = |list: &str| -> Vec<(String, Value)> {
            props
                .array(list)
                .unwrap_or(&[])
                .iter()
                .filter_map(|e| match e {
                    Value::Struct(s) => Some((s.string("first")?.to_owned(), s.get("second")?.clone())),
                    _ => None,
                })
                .collect()
        };
        for (name, v) in entries("m_TexEnvs") {
            if let Value::Struct(s) = v {
                let texture = s.pptr("m_Texture").unwrap_or_default();
                m.textures.insert(name, TexEnv { texture, scale: xy(s.get("m_Scale"), [1.; 2]), offset: xy(s.get("m_Offset"), [0.; 2]) });
            }
        }
        for list in ["m_Ints", "m_Floats"] {
            for (name, v) in entries(list) {
                if let Some(x) = number(Some(&v)) {
                    m.floats.insert(name, x);
                }
            }
        }
        for (name, v) in entries("m_Colors") {
            if let Some(c) = rgba(Some(&v)) {
                m.colors.insert(name, c);
            }
        }
        m
    }

    pub fn float(&self, name: &str) -> Option<f32> {
        self.floats.get(name).copied()
    }
    pub fn color(&self, name: &str) -> Option<[f32; 4]> {
        self.colors.get(name).copied()
    }
    /// A texture slot with a non-null texture.
    pub fn texture(&self, name: &str) -> Option<&TexEnv> {
        self.textures.get(name).filter(|t| !t.texture.is_null())
    }
    pub fn keyword(&self, name: &str) -> bool {
        self.keywords.iter().any(|k| k == name)
    }
}

/// How a material samples a texture; importers convert the Unity encoding
/// into the engine's slot layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TexUse {
    /// sRGB colour (+ alpha).
    Color,
    /// Unity tangent-space normal (`UnpackNormalmapRGorAG`: x = r·a, y = g).
    Normal,
    /// Unity metallic (R) / smoothness (A) at `smoothness` ×1000, as glTF
    /// roughness (G) / metalness (B).
    MetallicGloss { smoothness_milli: u16 },
    /// Occlusion in G (Unity) as R.
    Occlusion,
}

/// What an importer knows how to draw for a shader.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShaderSupport {
    /// Mapped to an engine model.
    Mapped,
    /// Drawn with an approximation (reported).
    Approximated,
    /// Not drawn (screen-space or volumetric effects without geometry
    /// meaning); reported.
    Skipped,
}

fn srgb_to_linear(c: f32) -> f32 {
    let c = c.clamp(0., 1.);
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Unity material colours are authored gamma-encoded; a linear-space
/// project uploads them linear.
pub fn linear_color(c: [f32; 4]) -> [f32; 4] {
    [srgb_to_linear(c[0]), srgb_to_linear(c[1]), srgb_to_linear(c[2]), c[3].clamp(0., 1.)]
}

/// Engine material of a Unity material. `image` returns the GLB image index
/// of a texture slot converted for its use (`None` when unavailable).
pub fn engine_material(m: &UnityMaterial, image: &mut dyn FnMut(&TexEnv, TexUse) -> Result<Option<usize>>) -> Result<(PbrMaterial, ShaderSupport)> {
    let shader = m.shader.as_str();
    let lower = shader.to_ascii_lowercase();
    let skipped = lower.starts_with("lux urp/fx") || lower.contains("heat_haze") || lower.contains("skybox") || lower.starts_with("hidden/");
    if skipped {
        return Ok((PbrMaterial::default(), ShaderSupport::Skipped));
    }
    let mut out = PbrMaterial { specular_factor: 0.5, ..Default::default() };
    let mut support = ShaderSupport::Mapped;
    let base_slot = ["_BaseMap", "_MainTex", "_BaseColorMap", "_AlbedoMap"].into_iter().find(|n| m.texture(n).is_some());
    let color = m.color("_BaseColor").or_else(|| m.color("_Color")).unwrap_or([1.; 4]);
    out.base_color_factor = linear_color(color);
    if let Some(slot) = base_slot {
        let t = m.texture(slot).cloned().expect("slot present");
        if let Some(i) = image(&t, TexUse::Color)? {
            out.base_color_texture = Some(i);
            out.uv_scale = t.scale;
            // Unity UVs are bottom-up; the GLB flips V per vertex.
            out.uv_offset = [t.offset[0], 1. - t.scale[1] - t.offset[1]];
        }
    }
    let float = |n: &str| m.float(n);
    // Surface type / alpha clipping (URP: _Surface 1 transparent,
    // _AlphaClip 1 cutout; legacy: _Mode 1 cutout, 2/3 fade/transparent).
    let transparent = float("_Surface") == Some(1.) || matches!(float("_Mode"), Some(m) if m >= 2.);
    let clip = float("_AlphaClip") == Some(1.) || float("_Mode") == Some(1.) || m.keyword("_ALPHATEST_ON");
    out.alpha_cutoff = float("_Cutoff").or_else(|| float("_AlphaCutoff")).unwrap_or(0.5).clamp(0., 1.);
    out.alpha_mode = if transparent {
        if float("_Blend") == Some(2.) { AlphaMode::Additive } else { AlphaMode::Blend }
    } else if clip {
        AlphaMode::Mask
    } else {
        AlphaMode::Opaque
    };
    out.double_sided = float("_Cull").or_else(|| float("_CullMode")) == Some(0.) || float("_TwoSided").is_some_and(|t| t != 2.);
    let smoothness = float("_Smoothness").or_else(|| float("_Glossiness")).unwrap_or(0.5).clamp(0., 1.);
    out.metallic_factor = float("_Metallic").unwrap_or(0.).clamp(0., 1.);
    out.roughness_factor = 1. - smoothness;
    if let Some(t) = m.texture("_BumpMap").or_else(|| m.texture("_NormalMap")).or_else(|| m.texture("_Normal_map")).cloned() {
        if let Some(i) = image(&t, TexUse::Normal)? {
            out.normal_texture = Some(i);
            out.normal_encoding = NormalEncoding::Rgb;
            out.normal_scale = float("_BumpScale").unwrap_or(1.).clamp(-4., 4.);
        }
    }
    if let Some(t) = m.texture("_MetallicGlossMap").cloned() {
        let scale = float("_GlossMapScale").unwrap_or(smoothness).clamp(0., 1.);
        if let Some(i) = image(&t, TexUse::MetallicGloss { smoothness_milli: (scale * 1000.) as u16 })? {
            out.metallic_roughness_texture = Some(i);
            out.metallic_factor = 1.;
            out.roughness_factor = 1.;
        }
    }
    if let Some(t) = m.texture("_OcclusionMap").cloned() {
        if let Some(i) = image(&t, TexUse::Occlusion)? {
            out.occlusion_texture = Some(i);
            out.occlusion_strength = float("_OcclusionStrength").unwrap_or(1.).clamp(0., 1.);
        }
    }
    if m.keyword("_EMISSION") {
        let e = m.color("_EmissionColor").unwrap_or([0.; 4]);
        out.emissive_factor = [e[0].max(0.), e[1].max(0.), e[2].max(0.)];
        if let Some(t) = m.texture("_EmissionMap").cloned() {
            out.emissive_texture = image(&t, TexUse::Color)?;
        }
    }
    let family_mapped = lower.starts_with("universal render pipeline/lit")
        || lower.starts_with("universal render pipeline/simple lit")
        || lower == "al/map"
        || lower.starts_with("standard")
        || lower.starts_with("legacy shaders/diffuse")
        || lower.starts_with("legacy shaders/bumped")
        || lower.starts_with("probuilder");
    if lower.contains("speedtree") {
        // SpeedTree 8: leaves/fronds are alpha tested at 0.33 and two-sided.
        out.alpha_mode = AlphaMode::Mask;
        out.alpha_cutoff = float("_Cutoff").unwrap_or(0.333).clamp(0., 1.);
        out.double_sided = float("_TwoSided").map_or(true, |t| t != 2.);
        // Foliage is lit diffusely (SpeedTree's subsurface/backface model has
        // no engine equivalent): rough, weakly specular.
        out.roughness_factor = (1. - float("_Glossiness").unwrap_or(0.2).clamp(0., 1.)).max(0.75);
        out.specular_factor = 0.2;
        out.metallic_factor = 0.;
    } else if lower.contains("unlit") {
        out.unlit = true;
    } else if lower.contains("particles") || lower.contains("vfx") || lower.contains("vefects") {
        let tint = m.color("_TintColor").map(|c| [c[0] * 2., c[1] * 2., c[2] * 2., c[3] * 2.]).unwrap_or(color);
        out.base_color_factor = linear_color(tint);
        out.unlit = true;
        out.double_sided = true;
        out.alpha_mode = if lower.contains("additive") { AlphaMode::Additive } else { AlphaMode::Blend };
        support = ShaderSupport::Approximated;
    } else if lower.contains("water") {
        // NatureManufacture water: a glossy translucent surface.
        let c = m.color("_ShallowColor").or_else(|| m.color("_AlbedoColor")).or_else(|| m.color("_BaseColor")).unwrap_or([0.25, 0.45, 0.55, 0.7]);
        out.base_color_factor = linear_color([c[0], c[1], c[2], c[3].clamp(0.5, 0.85)]);
        out.base_color_texture = None;
        out.alpha_mode = AlphaMode::Blend;
        out.roughness_factor = 0.08;
        out.metallic_factor = 0.;
        support = ShaderSupport::Approximated;
    } else if !family_mapped {
        support = ShaderSupport::Approximated;
    }
    Ok((out, support))
}

/// Objects the GLB assembly needs from the caller's Unity files. Pointers
/// are relative to the serialized file `file`.
pub trait SceneAssets {
    /// The object a pointer names: (file key, path id, class id, fields);
    /// `None` for null or unavailable (built-in) objects. Only `Material`
    /// and `Shader` fields are read; other classes may return empty fields.
    fn object(&mut self, file: &str, p: PPtr) -> Result<Option<(String, i64, i32, Fields)>>;
    fn mesh(&mut self, file: &str, path_id: i64) -> Result<Arc<Mesh6>>;
    /// Level 0 pixels (top-down RGBA8).
    fn texture(&mut self, file: &str, path_id: i64) -> Result<ImageSurface>;
    /// Level 0 size without decoding the pixels.
    fn texture_size(&mut self, file: &str, path_id: i64) -> Result<(u32, u32)>;
    /// Consumer semantics for a material drawn by a renderer of `file`
    /// (e.g. textures a game composes at runtime); `None` keeps the shader
    /// mapping.
    fn material_hook(&mut self, _file: &str, _m: &UnityMaterial) -> Result<Option<MaterialOverride>> {
        Ok(None)
    }
}

pub use crysirror_scene::material::ToonShadeKind;

/// A consumer's replacement of parts of a mapped material.
#[derive(Clone, Debug, Default)]
pub struct MaterialOverride {
    /// Base colour texture (top-down RGBA8, sRGB) with a cache key.
    pub base_texture: Option<(String, ImageSurface)>,
    /// Linear base colour factor.
    pub base_color: Option<[f32; 4]>,
    /// Do not draw the material.
    pub hidden: bool,
    pub alpha_mode: Option<AlphaMode>,
    /// Cel shading instead of PBR: the engine toon model with this
    /// lookup texture (256×3, see `crysirror_scene::material::ToonShading`)
    /// and inverted-hull outline (clip-space width, colour).
    pub toon: Option<(ImageSurface, f32, [f32; 4])>,
    /// Scale of the toon ambient term (`ToonShading::ambient_scale`).
    pub toon_ambient: Option<f32>,
    /// Bound of the toon key light (`ToonShading::light_limit`).
    pub toon_light_limit: Option<f32>,
    /// `ToonShading::shade_from_base`.
    pub toon_shade_from_base: bool,
    /// `ToonShading::shadow_rim`.
    pub toon_shadow_rim: Option<[f32; 4]>,
    /// `ToonShading::shadow_light`.
    pub toon_shadow_light: Option<[f32; 4]>,
    /// `ToonShading::shade_kind` and `shade_color`.
    pub toon_shade: Option<(ToonShadeKind, [f32; 3])>,
    /// Toon shadow layer (`ToonShading::shadow_texture`: top-down RGBA8,
    /// display-referred colour and amount, sampled raw) with a cache key.
    pub toon_shadow: Option<(String, ImageSurface)>,
}

#[derive(Clone, Debug)]
pub struct GlbOptions {
    /// Textures larger than this on either side are box-downsampled by
    /// powers of two.
    pub max_texture: u32,
    /// Prepared RGBA8 bytes (with mips) all textures may take: the largest
    /// textures are halved until the set fits.
    pub texture_budget: usize,
    /// Scale from Unity physics units (m/s²) to model units for spring
    /// gravity (e.g. 10 when characters are modelled at ten units per metre).
    pub spring_unit: f32,
    /// Export skinned meshes' blend shapes as named glTF morph targets
    /// (each shape's full-weight frame; weights 0..1).
    pub morph_targets: bool,
}

impl Default for GlbOptions {
    fn default() -> Self {
        Self { max_texture: 1024, texture_budget: 128 << 20, spring_unit: 1., morph_targets: false }
    }
}

/// Prepared RGBA8 bytes of a `w`×`h` texture with its mip chain.
fn prepared_bytes(w: u32, h: u32) -> usize {
    w as usize * h as usize * 4 * 4 / 3
}

fn halve_to(mut w: u32, mut h: u32, max: u32) -> (u32, u32) {
    while w > max || h > max {
        (w, h) = ((w / 2).max(1), (h / 2).max(1));
    }
    (w, h)
}

/// Per-texture maximum side so the set fits `budget`: the texture with the
/// largest prepared size is halved first (never below 64).
pub fn fit_textures<K: Clone + Eq + std::hash::Hash>(sizes: &[(K, u32, u32)], max: u32, budget: usize) -> HashMap<K, u32> {
    let mut cur: Vec<(K, u32, u32)> = sizes.iter().map(|(k, w, h)| {
        let (w, h) = halve_to(*w, *h, max);
        (k.clone(), w, h)
    }).collect();
    let mut total: usize = cur.iter().map(|(_, w, h)| prepared_bytes(*w, *h)).sum();
    while total > budget {
        let Some(i) = (0..cur.len()).filter(|&i| cur[i].1.max(cur[i].2) > 64).max_by_key(|&i| prepared_bytes(cur[i].1, cur[i].2)) else { break };
        let (_, w, h) = cur[i].clone();
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        total = total - prepared_bytes(w, h) + prepared_bytes(nw, nh);
        cur[i].1 = nw;
        cur[i].2 = nh;
    }
    cur.into_iter().map(|(k, w, h)| (k, w.max(h))).collect()
}

/// What a GLB assembly drew and left out.
#[derive(Clone, Debug, Default)]
pub struct GlbReport {
    /// Prepared texture bytes after [`fit_textures`].
    pub texture_bytes: usize,
    pub renderers: usize,
    pub skipped_renderers: usize,
    pub lod_culled: usize,
    pub materials: usize,
    pub textures: usize,
    /// Shader name → (support, material count).
    pub shaders: std::collections::BTreeMap<String, (String, usize)>,
    pub errors: Vec<String>,
}

fn downsample(mut img: ImageSurface, max: u32) -> ImageSurface {
    while img.width > max || img.height > max {
        let (w, h) = ((img.width / 2).max(1), (img.height / 2).max(1));
        let mut out = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let mut acc = [0u32; 4];
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let sx = (x * 2 + dx).min(img.width - 1);
                    let sy = (y * 2 + dy).min(img.height - 1);
                    let i = ((sy * img.width + sx) * 4) as usize;
                    for c in 0..4 {
                        acc[c] += u32::from(img.rgba[i + c]);
                    }
                }
                let o = ((y * w + x) * 4) as usize;
                for c in 0..4 {
                    out[o + c] = ((acc[c] + 2) / 4) as u8;
                }
            }
        }
        img = ImageSurface { width: w, height: h, rgba: out };
    }
    img
}

/// Convert decoded Unity texels for an engine slot.
pub fn convert_texels(img: &mut ImageSurface, usage: TexUse) {
    match usage {
        TexUse::Color => {}
        TexUse::Normal => {
            for px in img.rgba.chunks_exact_mut(4) {
                let x = f32::from(px[0]) * f32::from(px[3]) / (255. * 255.) * 2. - 1.;
                let y = f32::from(px[1]) / 255. * 2. - 1.;
                let z = (1. - x * x - y * y).max(0.).sqrt();
                px[0] = ((x * 0.5 + 0.5) * 255. + 0.5) as u8;
                px[1] = ((y * 0.5 + 0.5) * 255. + 0.5) as u8;
                px[2] = ((z * 0.5 + 0.5) * 255. + 0.5) as u8;
                px[3] = 255;
            }
        }
        TexUse::MetallicGloss { smoothness_milli } => {
            let s = f32::from(smoothness_milli) / 1000.;
            for px in img.rgba.chunks_exact_mut(4) {
                let metal = px[0];
                let rough = 255. - f32::from(px[3]) * s;
                px[0] = 255;
                px[1] = rough.clamp(0., 255.) as u8;
                px[2] = metal;
                px[3] = 255;
            }
        }
        TexUse::Occlusion => {
            for px in img.rgba.chunks_exact_mut(4) {
                px[0] = px[1];
                px[3] = 255;
            }
        }
    }
}

/// A node's Unity world matrix.
pub fn world_matrix(scene: &Scene6, mut node: usize) -> Mat4 {
    let mut m = Mat4::IDENTITY;
    loop {
        let x = &scene.nodes[node].xform;
        let local = Mat4::from_scale_rotation_translation(Vec3::from_array(x.scale), Quat::from_array(x.rotation).normalize(), Vec3::from_array(x.position));
        m = local * m;
        match scene.nodes[node].parent {
            Some(p) => node = p,
            None => return m,
        }
    }
}

/// One GLB under construction: its builder and the per-document caches.
struct Chunk {
    glb: GlbBuilder,
    /// Scene node → GLB node.
    nodes: HashMap<usize, usize>,
    materials: HashMap<(String, i64), Option<usize>>,
    images: HashMap<(String, i64, TexUse), Option<usize>>,
    /// (mesh file, path id, sub-mesh range, materials) → GLB mesh.
    meshes: HashMap<(String, i64, usize, usize, Vec<Option<usize>>), Option<usize>>,
    batch_root: Option<usize>,
    /// Spring chains on scene nodes (resolved to GLB nodes at the end).
    springs: Vec<(usize, crysirror_asset::spring::SpringChain)>,
    /// `GlbOptions::max_texture`, also applied to consumer override images.
    max_texture: u32,
}

impl Chunk {
    fn new(generator: &str, max_texture: u32) -> Self {
        Self { glb: GlbBuilder::new(generator), nodes: HashMap::new(), materials: HashMap::new(), images: HashMap::new(), meshes: HashMap::new(), batch_root: None, springs: Vec::new(), max_texture }
    }

    /// The GLB node of a scene node, creating its ancestor chain.
    fn node(&mut self, scene: &Scene6, node: usize) -> Result<usize> {
        if let Some(i) = self.nodes.get(&node) {
            return Ok(*i);
        }
        let parent = match scene.nodes[node].parent {
            Some(p) => Some(self.node(scene, p)?),
            None => None,
        };
        let x = &scene.nodes[node].xform;
        let i = self.glb.node(&scene.nodes[node].name, parent, x.position, x.rotation, x.scale)?;
        self.nodes.insert(node, i);
        Ok(i)
    }
}

/// Byte budget of one GLB chunk (the engine accepts at most 64 MiB per
/// converted model; JSON and alignment stay well below the margin).
pub const CHUNK_BUDGET: usize = 40 << 20;

fn renderer_enabled(f: &Fields) -> bool {
    f.bool("m_Enabled").or_else(|| f.int("m_Enabled").map(|v| v != 0)).unwrap_or(true)
}

/// Assemble the active renderers under `roots` of `scene` (pointers
/// relative to `file`) into GLB chunks of at most [`CHUNK_BUDGET`]; every
/// chunk carries the ancestor transforms of what it draws and is placed at
/// the world origin. Empty when nothing is drawn.
pub fn scene_glb(scene: &Scene6, file: &str, roots: &[usize], assets: &mut dyn SceneAssets, options: &GlbOptions, generator: &str) -> Result<(Vec<Vec<u8>>, GlbReport)> {
    scene_glb_animated(scene, file, roots, assets, options, generator, &[])
}

/// Unity-space values of a transform track.
#[derive(Clone, Debug, PartialEq)]
pub enum TrackValues {
    Translation(Vec<[f32; 3]>),
    Rotation(Vec<[f32; 4]>),
    Scale(Vec<[f32; 3]>),
}

/// A clip's transform curves sampled per scene node.
#[derive(Clone, Debug, Default)]
pub struct ClipTracks {
    pub name: String,
    pub tracks: Vec<(usize, Vec<f32>, TrackValues)>,
    /// Bindings without a node under the root, or not transform curves.
    pub unbound: usize,
}

/// Sample the generic transform bindings of `clip` (attributes 1 position,
/// 2 rotation, 3 scale) at `fps` over its duration, bound by path hash to
/// nodes under `root`.
pub fn clip_tracks(scene: &Scene6, root: usize, clip: &crate::animator::Clip, name: &str, fps: f32) -> ClipTracks {
    let mut paths: HashMap<u32, usize> = HashMap::new();
    paths.insert(0, root);
    let mut stack: Vec<(usize, String)> = scene.nodes[root].children.iter().map(|&c| (c, scene.nodes[c].name.clone())).collect();
    while let Some((n, path)) = stack.pop() {
        paths.entry(crate::animator::crc32(&path)).or_insert(n);
        for &c in &scene.nodes[n].children {
            stack.push((c, format!("{path}/{}", scene.nodes[c].name)));
        }
    }
    let d = clip.duration();
    let frames = ((d * fps).ceil() as usize).max(1);
    let times: Vec<f32> = (0..=frames).map(|i| (i as f32 / frames as f32) * d).collect();
    let samples: Vec<Vec<f32>> = times.iter().map(|t| clip.sample(*t)).collect();
    let mut out = ClipTracks { name: name.to_owned(), ..Default::default() };
    let mut at = 0;
    for b in &clip.bindings {
        let base = at;
        at += b.curves;
        let node = paths.get(&b.path).copied();
        let (Some(node), true) = (node, b.class_id == crate::class_id::TRANSFORM) else {
            out.unbound += 1;
            continue;
        };
        let v = |s: &Vec<f32>, k: usize| s.get(base + k).copied().unwrap_or(0.);
        let values = match b.attribute {
            1 => TrackValues::Translation(samples.iter().map(|s| [v(s, 0), v(s, 1), v(s, 2)]).collect()),
            2 => {
                let mut prev = [0., 0., 0., 1.];
                TrackValues::Rotation(
                    samples
                        .iter()
                        .map(|s| {
                            let q = Quat::from_xyzw(v(s, 0), v(s, 1), v(s, 2), v(s, 3)).normalize();
                            let mut q = q.to_array();
                            // Keep consecutive keys in one hemisphere.
                            if q.iter().zip(&prev).map(|(a, b)| a * b).sum::<f32>() < 0. {
                                q = q.map(|x| -x);
                            }
                            prev = q;
                            q
                        })
                        .collect(),
                )
            }
            3 => TrackValues::Scale(samples.iter().map(|s| [v(s, 0), v(s, 1), v(s, 2)]).collect()),
            _ => {
                out.unbound += 1;
                continue;
            }
        };
        out.tracks.push((node, times.clone(), values));
    }
    out
}

/// [`scene_glb`] with clips as glTF animations on the chunks' nodes.
pub fn scene_glb_animated(scene: &Scene6, file: &str, roots: &[usize], assets: &mut dyn SceneAssets, options: &GlbOptions, generator: &str, clips: &[ClipTracks]) -> Result<(Vec<Vec<u8>>, GlbReport)> {
    let mut report = GlbReport::default();
    // LOD groups: only LOD 0 renderers are drawn (no distance switching).
    let mut lod_hidden: HashSet<i64> = HashSet::new();
    for n in &scene.nodes {
        for c in n.components.iter().filter(|c| c.class_id == LOD_GROUP) {
            for lod in c.fields.array("m_LODs").unwrap_or(&[]).iter().skip(1) {
                let Value::Struct(lod) = lod else { continue };
                for r in lod.array("renderers").unwrap_or(&[]) {
                    if let Value::Struct(r) = r {
                        if let Some(p) = r.pptr("renderer").filter(|p| p.file_id == 0) {
                            lod_hidden.insert(p.path_id);
                        }
                    }
                }
            }
        }
    }
    // Drawable renderers, grouped by material so a texture lands in one
    // chunk.
    let mut drawable: Vec<(Vec<PPtr>, usize)> = Vec::new();
    let mut stack: Vec<usize> = roots.iter().rev().copied().collect();
    while let Some(node) = stack.pop() {
        let n = &scene.nodes[node];
        if !n.active {
            continue;
        }
        for &child in n.children.iter().rev() {
            stack.push(child);
        }
        let Some(r) = n.components.iter().find(|c| (c.class_id == MESH_RENDERER && n.components.iter().any(|f| f.class_id == MESH_FILTER)) || c.class_id == SKINNED_MESH_RENDERER) else { continue };
        if !renderer_enabled(&r.fields) {
            report.skipped_renderers += 1;
        } else if lod_hidden.contains(&r.path_id) {
            report.lod_culled += 1;
        } else {
            let mats = r.fields.array("m_Materials").unwrap_or(&[]).iter().filter_map(|p| if let Value::PPtr(p) = p { Some(*p) } else { None }).collect();
            drawable.push((mats, node));
        }
    }
    drawable.sort_by_key(|(m, n)| (scene.nodes[*n].origin, m.iter().map(|p| (p.file_id, p.path_id)).collect::<Vec<_>>()));
    // Transforms by (origin, path id): grafted parts keep their own ids.
    let transforms: HashMap<(u32, i64), usize> = scene.nodes.iter().enumerate().map(|(i, n)| ((n.origin, n.transform), i)).collect();
    // Texture budget: every (texture, use) the materials sample, sized.
    let mut seen_materials: HashSet<(String, i64)> = HashSet::new();
    let mut wanted: Vec<((String, i64, TexUse), u32, u32)> = Vec::new();
    let mut seen_textures: HashSet<(String, i64, TexUse)> = HashSet::new();
    for (p, n) in drawable.iter().flat_map(|(m, n)| m.iter().map(move |p| (p, *n))) {
        let Some((mf, mid, class, fields)) = assets.object(scene.file_of(n, file), *p)? else { continue };
        if class != crate::class_id::MATERIAL || !seen_materials.insert((mf.clone(), mid)) {
            continue;
        }
        let shader = match assets.object(&mf, fields.pptr("m_Shader").unwrap_or_default())? {
            Some((_, _, _, s)) => shader_name(&s),
            None => String::new(),
        };
        let m = UnityMaterial::decode(&fields, shader);
        let mut uses = Vec::new();
        engine_material(&m, &mut |t, usage| {
            uses.push((t.texture, usage));
            Ok(None)
        })?;
        for (tp, usage) in uses {
            let Some((tf, tid, class, _)) = assets.object(&mf, tp)? else { continue };
            if class != crate::class_id::TEXTURE_2D || !seen_textures.insert((tf.clone(), tid, usage)) {
                continue;
            }
            if let Ok((w, h)) = assets.texture_size(&tf, tid) {
                wanted.push(((tf, tid, usage), w, h));
            }
        }
    }
    let sides = fit_textures(&wanted, options.max_texture, options.texture_budget);
    report.texture_bytes = wanted.iter().map(|(k, w, h)| {
        let (w, h) = halve_to(*w, *h, sides[k]);
        prepared_bytes(w, h)
    }).sum();
    let mut chunks = Vec::new();
    let springs = magica_bone_cloths(scene, options.spring_unit);
    let mut chunk = Chunk::new(generator, options.max_texture);
    chunk.springs = springs.clone();
    for (_, node) in drawable {
        let n = &scene.nodes[node];
        let Some(r) = n.components.iter().find(|c| c.class_id == MESH_RENDERER || c.class_id == SKINNED_MESH_RENDERER) else { continue };
        let skinned = r.class_id == SKINNED_MESH_RENDERER;
        let mesh_ptr = if skinned { r.fields.pptr("m_Mesh") } else { n.components.iter().find(|c| c.class_id == MESH_FILTER).and_then(|f| f.fields.pptr("m_Mesh")) }.unwrap_or_default();
        if chunk.glb.estimated_bytes() > CHUNK_BUDGET {
            let mut fresh = Chunk::new(generator, options.max_texture);
            fresh.springs = springs.clone();
            let full = std::mem::replace(&mut chunk, fresh);
            report.materials += full.materials.values().filter(|m| m.is_some()).count();
            report.textures += full.images.values().filter(|i| i.is_some()).count();
            chunks.push(finish_chunk(full, clips)?);
        }
        let drawn = (|| -> Result<bool> {
            let nf = scene.file_of(node, file);
            let Some((mf, mid, _, _)) = assets.object(nf, mesh_ptr)? else { return Ok(false) };
            let mesh = assets.mesh(&mf, mid)?;
            let mut ids = Vec::new();
            for p in r.fields.array("m_Materials").unwrap_or(&[]) {
                if let Value::PPtr(p) = p {
                    ids.push(material(assets, &mut chunk, &mut report, nf, *p, &sides)?);
                }
            }
            let (first, count) = r.fields.fields("m_StaticBatchInfo").map(|s| (s.int("firstSubMesh").unwrap_or(0) as usize, s.int("subMeshCount").unwrap_or(0) as usize)).unwrap_or((0, 0));
            let (holder, range) = if count > 0 {
                // Static batch: world-space vertices of the combined mesh,
                // drawn under an identity root.
                let root = match chunk.batch_root {
                    Some(b) => b,
                    None => {
                        let b = chunk.glb.node("static batch", None, [0.; 3], [0., 0., 0., 1.], [1.; 3])?;
                        chunk.batch_root = Some(b);
                        b
                    }
                };
                (chunk.glb.node(&n.name, Some(root), [0.; 3], [0., 0., 0., 1.], [1.; 3])?, first..first + count)
            } else {
                (chunk.node(scene, node)?, 0..mesh.submeshes.len())
            };
            if skinned {
                // Joints: the renderer's `m_Bones` transforms (with their
                // ancestors) in this chunk; the mesh node's own transform is
                // ignored by glTF skinning, as Unity ignores the renderer's.
                let mut joints = Vec::new();
                for b in r.fields.array("m_Bones").unwrap_or(&[]) {
                    let Value::PPtr(b) = b else { continue };
                    let bone = transforms.get(&(n.origin, b.path_id)).copied().filter(|_| b.file_id == 0).map(|bn| scene.bone_alias.get(&bn).copied().unwrap_or(bn));
                    joints.push(match bone {
                        Some(bn) => chunk.node(scene, bn)?,
                        None => holder,
                    });
                }
                anyhow::ensure!(mesh.bind_poses.len() == joints.len(), "skin of {} has {} bones for {} bind poses", mesh.name, joints.len(), mesh.bind_poses.len());
                let skin = chunk.glb.skin(joints, &mesh.bind_poses);
                chunk.glb.set_skin(holder, skin);
                let index = chunk.glb.mesh6_full(holder, &mesh, range, &ids, true, options.morph_targets)?;
                return Ok(index.is_some());
            }
            let key = (mf, mid, range.start, range.end, ids.clone());
            let index = match chunk.meshes.get(&key) {
                Some(i) => {
                    if let Some(i) = i {
                        chunk.glb.set_mesh(holder, *i);
                    }
                    *i
                }
                None => {
                    let i = chunk.glb.mesh6(holder, &mesh, range, &ids)?;
                    chunk.meshes.insert(key, i);
                    i
                }
            };
            Ok(index.is_some())
        })();
        match drawn {
            Ok(true) => report.renderers += 1,
            Ok(false) => report.skipped_renderers += 1,
            Err(e) => {
                report.skipped_renderers += 1;
                report.errors.push(format!("{}: {e:#}", scene.path(node)));
            }
        }
    }
    report.materials += chunk.materials.values().filter(|m| m.is_some()).count();
    report.textures += chunk.images.values().filter(|i| i.is_some()).count();
    if !chunk.glb.is_empty() {
        chunks.push(finish_chunk(chunk, clips)?);
    }
    Ok((chunks, report))
}

/// MagicaCloth 2 BoneCloth (`clothType` 1) and BoneSpring (10) components as
/// spring chains on their root bones (resolved through bone aliases), with
/// `unit` engine units per metre. Gravity, damping, angle restoration
/// (stiffness, velocity attenuation), angle limit, particle radius, the
/// spring's distance limit and capsule/sphere colliders of `colliderList`
/// are converted; depth curves, inertia and self collision are not.
pub fn magica_bone_cloths(scene: &Scene6, unit: f32) -> Vec<(usize, crysirror_asset::spring::SpringChain)> {
    let transforms: HashMap<(u32, i64), usize> = scene.nodes.iter().enumerate().map(|(i, n)| ((n.origin, n.transform), i)).collect();
    let components: HashMap<(u32, i64), (usize, &Component)> = scene.nodes.iter().enumerate().flat_map(|(i, n)| n.components.iter().map(move |c| ((n.origin, c.path_id), (i, c)))).collect();
    let alias = |n: usize| scene.bone_alias.get(&n).copied().unwrap_or(n);
    let mut out = Vec::new();
    for n in scene.nodes.iter().filter(|n| n.active) {
        for c in n.components.iter().filter(|c| c.script.ends_with("MagicaCloth")) {
            let Some(d) = c.fields.fields("serializeData") else { continue };
            let kind = d.int("clothType").unwrap_or(0);
            if kind != 1 && kind != 10 {
                continue;
            }
            let value = |f: Option<&Fields>, default: f32| f.and_then(|f| f.float("value")).map_or(default, |v| v as f32);
            let dir = d.fields("gravityDirection").map_or([0., -1., 0.], |g| ["x", "y", "z"].map(|k| g.float(k).unwrap_or(0.) as f32));
            let g = d.float("gravity").unwrap_or(5.) as f32 * unit;
            let gravity = crate::gltf::mirror_point([dir[0] * g, dir[1] * g, dir[2] * g]);
            let restoration = d.fields("angleRestorationConstraint");
            let restore = restoration.and_then(|r| r.int("useAngleRestoration")).unwrap_or(1) != 0;
            let mut stiffness = if restore { value(restoration.and_then(|r| r.fields("stiffness")), 0.2) } else { 0. };
            let attenuation = if restore { restoration.and_then(|r| r.float("velocityAttenuation")).unwrap_or(0.) as f32 } else { 0. };
            let limit = d.fields("angleLimitConstraint");
            let angle_limit = if limit.and_then(|l| l.int("useAngleLimit")).unwrap_or(0) != 0 { value(limit.and_then(|l| l.fields("limitAngle")), 60.) } else { 0. };
            let mut max_distance = 0.;
            if kind == 10 {
                if let Some(sp) = d.fields("springConstraint").filter(|s| s.int("useSpring").unwrap_or(0) != 0) {
                    stiffness += sp.float("springPower").unwrap_or(0.) as f32;
                    max_distance = sp.float("limitDistance").unwrap_or(0.) as f32 * unit;
                }
            }
            let mut colliders = Vec::new();
            for p in d.fields("colliderCollisionConstraint").and_then(|c| c.array("colliderList")).unwrap_or(&[]) {
                let Value::PPtr(p) = p else { continue };
                if p.path_id == 0 || p.file_id != 0 {
                    continue;
                }
                let Some(&(node, col)) = components.get(&(n.origin, p.path_id)) else { continue };
                if let Some(c) = magica_collider(col) {
                    colliders.push(crysirror_asset::spring::SpringCollider { node: alias(node), ..c });
                }
            }
            let chain = crysirror_asset::spring::SpringChain {
                root: 0,
                gravity,
                damping: (value(d.fields("damping"), 0.05).max(attenuation) * 0.5).clamp(0., 1.),
                stiffness: stiffness.clamp(0., 1.),
                angle_limit: angle_limit.clamp(0., 180.),
                radius: (value(d.fields("radius"), 0.02) * unit).max(0.),
                max_distance: max_distance.clamp(0., 1e4),
                colliders,
            };
            for b in d.array("rootBones").unwrap_or(&[]) {
                let Value::PPtr(b) = b else { continue };
                if let Some(&node) = transforms.get(&(n.origin, b.path_id)) {
                    out.push((alias(node), chain.clone()));
                }
            }
        }
    }
    out
}

/// A `MagicaCapsuleCollider` (`size`: start radius, end radius, length along
/// `direction` X/Y/Z, centred or from `center`) or `MagicaSphereCollider` in
/// its node's space (node index left 0).
fn magica_collider(c: &Component) -> Option<crysirror_asset::spring::SpringCollider> {
    let v = |name: &str| c.fields.fields(name).map(|f| ["x", "y", "z"].map(|k| f.float(k).unwrap_or(0.) as f32));
    let center = v("center").unwrap_or([0.; 3]);
    let size = v("size")?;
    if c.script.ends_with("MagicaSphereCollider") {
        return Some(crysirror_asset::spring::SpringCollider { node: 0, offset: crate::gltf::mirror_point(center), tail: None, radius: size[0].max(0.) });
    }
    if !c.script.ends_with("MagicaCapsuleCollider") {
        return None;
    }
    let mut axis = [0.; 3];
    axis[c.fields.int("direction").unwrap_or(0).clamp(0, 2) as usize] = if c.fields.int("reverseDirection").unwrap_or(0) != 0 { -1. } else { 1. };
    let (start, end) = if c.fields.int("alignedOnCenter").unwrap_or(1) != 0 { (-0.5 * size[2], 0.5 * size[2]) } else { (0., size[2]) };
    let at = |t: f32| [center[0] + axis[0] * t, center[1] + axis[1] * t, center[2] + axis[2] * t];
    let radius = if c.fields.int("radiusSeparation").unwrap_or(0) != 0 { size[0].max(size[1]) } else { size[0] };
    Some(crysirror_asset::spring::SpringCollider { node: 0, offset: crate::gltf::mirror_point(at(start)), tail: Some(crate::gltf::mirror_point(at(end))), radius: radius.max(0.) })
}

/// Add the clips' tracks of nodes present in the chunk, then encode.
fn finish_chunk(mut chunk: Chunk, clips: &[ClipTracks]) -> Result<Vec<u8>> {
    for (node, mut chain) in std::mem::take(&mut chunk.springs) {
        if let Some(&root) = chunk.nodes.get(&node) {
            chain.root = root;
            // Colliders on nodes outside this chunk are dropped.
            chain.colliders.retain_mut(|c| chunk.nodes.get(&c.node).map(|&n| c.node = n).is_some());
            chunk.glb.spring(chain);
        }
    }
    for c in clips {
        let tracks: Vec<_> = c.tracks.iter().filter_map(|(n, t, v)| Some((*chunk.nodes.get(n)?, t.clone(), v.clone()))).collect();
        if !tracks.is_empty() {
            chunk.glb.animation(&c.name, tracks);
        }
    }
    chunk.glb.finish()
}

fn material(assets: &mut dyn SceneAssets, chunk: &mut Chunk, report: &mut GlbReport, file: &str, p: PPtr, sides: &HashMap<(String, i64, TexUse), u32>) -> Result<Option<usize>> {
    let Some((mf, mid, class, fields)) = assets.object(file, p)? else { return Ok(None) };
    if class != crate::class_id::MATERIAL {
        return Ok(None);
    }
    // Per renderer file: a consumer hook may draw a shared material
    // differently for each part.
    let cache_key = (format!("{file}|{mf}"), mid);
    if let Some(id) = chunk.materials.get(&cache_key) {
        return Ok(*id);
    }
    let shader = match assets.object(&mf, fields.pptr("m_Shader").unwrap_or_default())? {
        Some((_, _, _, s)) => shader_name(&s),
        None => String::new(),
    };
    let mut m = UnityMaterial::decode(&fields, shader);
    m.file = mf.clone();
    let (glb, images) = (&mut chunk.glb, &mut chunk.images);
    let mut image = |t: &TexEnv, usage: TexUse| -> Result<Option<usize>> {
        let Some((tf, tid, class, _)) = assets.object(&mf, t.texture)? else { return Ok(None) };
        if class != crate::class_id::TEXTURE_2D {
            return Ok(None);
        }
        if let Some(i) = images.get(&(tf.clone(), tid, usage)) {
            return Ok(*i);
        }
        let img = match assets.texture(&tf, tid) {
            Ok(img) => {
                let max = sides.get(&(tf.clone(), tid, usage)).copied().unwrap_or(64);
                let mut img = downsample(img, max);
                convert_texels(&mut img, usage);
                let png = crysirror_image::encode_png(&img)?;
                Some(glb.image(&format!("{tf}:{tid}:{usage:?}"), || Ok((png, "image/png")))?)
            }
            Err(e) => {
                report.errors.push(format!("texture {tid} of material {}: {e:#}", m.name));
                None
            }
        };
        images.insert((tf, tid, usage), img);
        Ok(img)
    };
    let (mut engine, mut support) = engine_material(&m, &mut image)?;
    if let Some(o) = assets.material_hook(file, &m)? {
        if o.hidden {
            support = ShaderSupport::Skipped;
        }
        if let Some((key, img)) = o.base_texture {
            let img = downsample(img, chunk.max_texture.min(1024));
            let png = crysirror_image::encode_png(&img)?;
            engine.base_color_texture = Some(chunk.glb.image(&key, || Ok((png, "image/png")))?);
            engine.uv_scale = [1.; 2];
            engine.uv_offset = [0.; 2];
        }
        if let Some(c) = o.base_color {
            engine.base_color_factor = c;
        }
        if let Some(a) = o.alpha_mode {
            engine.alpha_mode = a;
            engine.double_sided |= a.transparent();
        }
        if let Some((lut, width, color)) = o.toon {
            let png = crysirror_image::encode_png(&lut)?;
            let key = format!("toon-lut:{:x}", png.iter().fold(0u64, |h, b| h.wrapping_mul(31).wrapping_add(u64::from(*b))));
            let lookup = chunk.glb.image(&key, || Ok((png, "image/png")))?;
            // Toon materials use the lookup/shadow/highlight slots instead.
            engine.metallic_roughness_texture = None;
            engine.occlusion_texture = None;
            engine.specular_texture = None;
            engine.effect = None;
            // Cast shadows take the ramp's shadow tone (URP toon shaders
            // fold shadow attenuation into the ramp coordinate).
            let shadow = match &o.toon_shadow {
                Some((key, img)) => {
                    let png = crysirror_image::encode_png(&downsample(img.clone(), chunk.max_texture.min(1024)))?;
                    Some(chunk.glb.image(key, || Ok((png, "image/png")))?)
                }
                None => None,
            };
            engine.toon = Some(crysirror_scene::material::ToonShading {
                lookup_texture: Some(lookup),
                shadow_texture: shadow,
                cast_shadow_ramp: true,
                ambient_scale: o.toon_ambient.unwrap_or(1.).clamp(0., 16.),
                light_limit: o.toon_light_limit.unwrap_or(0.).clamp(0., 16.),
                shade_from_base: o.toon_shade_from_base,
                shadow_rim: o.toon_shadow_rim.unwrap_or([0.; 4]).map(|x| x.clamp(0., 1.)),
                shadow_light: o.toon_shadow_light.unwrap_or([0.; 4]).map(|x| x.clamp(0., 1.)),
                shade_kind: o.toon_shade.map_or(Default::default(), |s| s.0),
                shade_color: o.toon_shade.map_or([0.5; 3], |s| s.1.map(|x| x.clamp(0., 1.))),
                outline_width: width.clamp(0., 0.1),
                outline_color: color,
                ..Default::default()
            });
        }
    }
    let e = report.shaders.entry(m.shader.clone()).or_insert_with(|| (format!("{support:?}"), 0));
    e.1 += 1;
    let id = match support {
        ShaderSupport::Skipped => None,
        ShaderSupport::Approximated => {
            chunk.glb.dropped(format!("material {} shader {} approximated", m.name, m.shader));
            Some(chunk.glb.material(&m.name, engine))
        }
        ShaderSupport::Mapped => Some(chunk.glb.material(&m.name, engine)),
    };
    chunk.materials.insert(cache_key, id);
    Ok(id)
}

/// A light of a level, in Unity world space.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneLight {
    pub node: usize,
    /// 0 spot, 1 directional, 2 point, 3 area.
    pub kind: i64,
    pub color: [f32; 3],
    pub intensity: f32,
    pub range: f32,
    /// `m_CullingMask` layer bits.
    pub culling_mask: u32,
    pub position: [f32; 3],
    /// Unity forward (+Z) of the light.
    pub direction: [f32; 3],
    /// Spot cone: `m_SpotAngle` (outer, full angle) and `m_InnerSpotAngle`
    /// in degrees.
    pub spot_angle: f32,
    pub inner_spot_angle: f32,
    /// `m_Shadows.m_Type` (0 none, 1 hard, 2 soft).
    pub shadows: i64,
    /// `m_Lightmapping` (bake type): 4 realtime, 1 mixed, 2 baked only.
    pub bake: i64,
}

/// Evaluate a serialized `UnityEngine.Gradient` (`key0..7` colours at
/// `ctime0..7`/65535, `m_NumColorKeys`; `m_Mode` 0 blends, 1 steps) at
/// `t` in 0..1; RGB only.
pub fn gradient(g: &Fields, t: f32) -> Option<[f32; 3]> {
    let n = g.int("m_NumColorKeys").unwrap_or(0).clamp(0, 8) as usize;
    let keys: Vec<(f32, [f32; 3])> = (0..n)
        .filter_map(|i| {
            let c = rgba(g.get(&format!("key{i}")))?;
            Some((g.int(&format!("ctime{i}")).unwrap_or(0) as f32 / 65535., [c[0], c[1], c[2]]))
        })
        .collect();
    let first = keys.first()?;
    let t = t.clamp(0., 1.);
    if t <= first.0 {
        return Some(first.1);
    }
    for w in keys.windows(2) {
        let ((t0, a), (t1, b)) = (w[0], w[1]);
        if t <= t1 {
            if g.int("m_Mode").unwrap_or(0) == 1 {
                return Some(b);
            }
            let f = if t1 > t0 { (t - t0) / (t1 - t0) } else { 1. };
            return Some([0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * f));
        }
    }
    keys.last().map(|k| k.1)
}

/// The enabled lights of active nodes.
pub fn scene_lights(scene: &Scene6) -> Vec<SceneLight> {
    let mut out = Vec::new();
    for (i, n) in scene.nodes.iter().enumerate() {
        let mut active = n.active;
        let mut p = n.parent;
        while let Some(q) = p {
            active &= scene.nodes[q].active;
            p = scene.nodes[q].parent;
        }
        if !active {
            continue;
        }
        for c in n.components.iter().filter(|c| c.class_id == LIGHT) {
            if c.fields.int("m_Enabled").unwrap_or(1) == 0 {
                continue;
            }
            let m = world_matrix(scene, i);
            let col = rgba(c.fields.get("m_Color")).unwrap_or([1.; 4]);
            out.push(SceneLight {
                node: i,
                kind: c.fields.int("m_Type").unwrap_or(1),
                color: [col[0], col[1], col[2]],
                intensity: c.fields.float("m_Intensity").unwrap_or(1.) as f32,
                range: c.fields.float("m_Range").unwrap_or(10.) as f32,
                culling_mask: c.fields.fields("m_CullingMask").and_then(|m| m.int("m_Bits")).unwrap_or(-1) as u32,
                position: m.w_axis.truncate().to_array(),
                direction: m.transform_vector3(Vec3::Z).normalize_or_zero().to_array(),
                spot_angle: c.fields.float("m_SpotAngle").unwrap_or(30.) as f32,
                inner_spot_angle: c.fields.float("m_InnerSpotAngle").unwrap_or(0.) as f32,
                shadows: c.fields.fields("m_Shadows").and_then(|f| f.int("m_Type")).unwrap_or(0),
                bake: c.fields.int("m_Lightmapping").unwrap_or(4),
            });
        }
    }
    out
}

/// `RenderSettings` ambient and fog.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RenderSettings6 {
    /// 0 skybox, 1 trilight, 3 flat.
    pub ambient_mode: i64,
    pub ambient_sky: [f32; 3],
    pub ambient_equator: [f32; 3],
    pub ambient_ground: [f32; 3],
    pub ambient_intensity: f32,
    /// SH band 0 of the ambient probe (linear), for skybox ambient.
    pub ambient_probe_dc: [f32; 3],
    pub fog: bool,
    pub fog_color: [f32; 3],
    pub fog_start: f32,
    pub fog_end: f32,
}

impl RenderSettings6 {
    pub fn decode(f: &Fields) -> Self {
        let c3 = |n: &str| rgba(f.get(n)).map_or([0.; 3], |c| [c[0], c[1], c[2]]);
        let sh = f.fields("m_AmbientProbe");
        let dc = |i: usize| sh.and_then(|s| s.float(&format!("sh[{i:2}]"))).unwrap_or(0.) as f32;
        Self {
            ambient_mode: f.int("m_AmbientMode").unwrap_or(0),
            ambient_sky: c3("m_AmbientSkyColor"),
            ambient_equator: c3("m_AmbientEquatorColor"),
            ambient_ground: c3("m_AmbientGroundColor"),
            ambient_intensity: f.float("m_AmbientIntensity").unwrap_or(1.) as f32,
            // Unity SH9 order: 9 coefficients per channel (r: 0..9, g: 9..18, b: 18..27).
            ambient_probe_dc: [dc(0), dc(9), dc(18)],
            fog: f.bool("m_Fog").unwrap_or(false),
            fog_color: c3("m_FogColor"),
            fog_start: f.float("m_LinearFogStart").unwrap_or(0.) as f32,
            fog_end: f.float("m_LinearFogEnd").unwrap_or(300.) as f32,
        }
    }

    /// A single linear ambient colour (the equator colour for trilight).
    pub fn ambient_linear(&self) -> [f32; 3] {
        let c = match self.ambient_mode {
            1 => [(self.ambient_sky[0] + self.ambient_equator[0] * 2. + self.ambient_ground[0]) / 4., (self.ambient_sky[1] + self.ambient_equator[1] * 2. + self.ambient_ground[1]) / 4., (self.ambient_sky[2] + self.ambient_equator[2] * 2. + self.ambient_ground[2]) / 4.],
            3 => self.ambient_sky,
            _ => return self.ambient_probe_dc.map(|v| v.max(0.)),
        };
        c.map(|v| srgb_to_linear(v) * self.ambient_intensity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_and_gloss_conversion() {
        let mut n = ImageSurface { width: 1, height: 1, rgba: vec![255, 128, 0, 128] };
        convert_texels(&mut n, TexUse::Normal);
        // x = r·a ≈ 0.5 → 0, y = 128 → 0, z = 1.
        assert!((i32::from(n.rgba[0]) - 128).abs() <= 1 && (i32::from(n.rgba[1]) - 128).abs() <= 1 && n.rgba[2] == 255);
        let mut g = ImageSurface { width: 1, height: 1, rgba: vec![200, 0, 0, 255] };
        convert_texels(&mut g, TexUse::MetallicGloss { smoothness_milli: 500 });
        assert_eq!((g.rgba[1], g.rgba[2]), (127, 200));
    }

    #[test]
    fn texture_budget_halves_the_largest_first() {
        let sizes = [("a", 2048, 2048), ("b", 512, 512), ("c", 64, 64)];
        let fit = fit_textures(&sizes, 1024, prepared_bytes(512, 512) * 2 + prepared_bytes(64, 64));
        assert_eq!((fit["a"], fit["b"], fit["c"]), (512, 512, 64));
        let all = fit_textures(&sizes, 1024, usize::MAX);
        assert_eq!((all["a"], all["b"]), (1024, 512));
    }

    #[test]
    fn material_mapping() {
        let mut m = UnityMaterial { shader: "Universal Render Pipeline/Lit".into(), ..Default::default() };
        m.floats.insert("_Surface".into(), 0.);
        m.floats.insert("_AlphaClip".into(), 1.);
        m.floats.insert("_Cutoff".into(), 0.3);
        m.floats.insert("_Smoothness".into(), 0.8);
        m.colors.insert("_BaseColor".into(), [1., 0.5, 0., 1.]);
        let (e, s) = engine_material(&m, &mut |_, _| Ok(None)).unwrap();
        assert_eq!(s, ShaderSupport::Mapped);
        assert_eq!(e.alpha_mode, AlphaMode::Mask);
        assert!((e.alpha_cutoff - 0.3).abs() < 1e-6 && (e.roughness_factor - 0.2).abs() < 1e-6);
        assert!((e.base_color_factor[1] - 0.214).abs() < 0.01);
        m.shader = "Lux URP/FX/Lightbeam".into();
        assert_eq!(engine_material(&m, &mut |_, _| Ok(None)).unwrap().1, ShaderSupport::Skipped);
    }
}

// SPDX-License-Identifier: MPL-2.0
//! Custom Maid 3D 2 (KISS) `.model` skinned meshes and `.anm` clips.
//!
//! Layouts follow the 1.00 loaders in Assembly-CSharp: `ImportCM.LoadSkinMesh_R`
//! (mesh, bones, bind poses, weights, sub-meshes, materials), `ImportCM.ReadMaterial`
//! (also the `.mate` body), `TMorph.LoadMoprhData2` (blend shapes) and
//! `ImportCM.LoadAniClip` (curves). Every value is a .NET `BinaryReader`
//! scalar or 7-bit length-prefixed UTF-8 string.
//!
//! Unity is left-handed with bottom-up texture rows; glTF is right-handed and
//! top-down. The conversion mirrors X (positions, normals, bone translations,
//! quaternions `(x, -y, -z, w)`, bind matrices `S·B·S`), reverses triangle
//! winding and flips V. Bone names are kept verbatim, so `.anm` paths and
//! game bone lookups bind by name. A bone flagged with a scale node gets the
//! `<name>_SCL_` child that the original creates, and skin joints bind to it.
use super::glb;
use crate::pbr::{AlphaMode, PbrMaterial};
use anyhow::{bail, ensure, Context, Result};
use std::collections::{BTreeMap, HashMap};

pub const MESH_SIGNATURE: &[u8] = b"\x0aCM3D2_MESH";
pub const ANIM_SIGNATURE: &[u8] = b"\x0aCM3D2_ANIM";
pub const MATERIAL_SIGNATURE: &[u8] = b"\x0eCM3D2_MATERIAL";
pub const MAX_BONES: usize = 4096;
pub const MAX_VERTICES: usize = 1 << 20;

/// `System.IO.BinaryReader` over a complete CM3D2 asset.
pub struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }
    pub fn position(&self) -> usize {
        self.at
    }
    pub fn remaining(&self) -> usize {
        self.bytes.len() - self.at
    }
    pub fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        let end = self
            .at
            .checked_add(count)
            .filter(|end| *end <= self.bytes.len())
            .context("truncated CM3D2 asset")?;
        let slice = &self.bytes[self.at..end];
        self.at = end;
        Ok(slice)
    }
    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    pub fn bool(&mut self) -> Result<bool> {
        Ok(self.u8()? != 0)
    }
    pub fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into()?))
    }
    pub fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into()?))
    }
    pub fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into()?))
    }
    pub fn i64(&mut self) -> Result<i64> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into()?))
    }
    pub fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into()?))
    }
    pub fn f32s<const N: usize>(&mut self) -> Result<[f32; N]> {
        let mut out = [0.; N];
        for value in &mut out {
            *value = self.f32()?;
        }
        Ok(out)
    }
    /// A non-negative count whose elements of `unit` bytes fit the input.
    pub fn count(&mut self, unit: usize) -> Result<usize> {
        let count = self.i32()?;
        ensure!(count >= 0, "negative CM3D2 element count");
        let count = count as usize;
        ensure!(
            count.saturating_mul(unit.max(1)) <= self.remaining(),
            "CM3D2 element count exceeds the asset"
        );
        Ok(count)
    }
    /// `BinaryReader.ReadString`.
    pub fn string(&mut self) -> Result<String> {
        let mut length = 0usize;
        for shift in (0..35).step_by(7) {
            let byte = self.u8()?;
            length |= usize::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                let raw = self.take(length)?;
                return String::from_utf8(raw.to_vec()).context("invalid UTF-8 in CM3D2 string");
            }
        }
        bail!("CM3D2 string length prefix is too long")
    }
    /// Signature string and version (1000 in CM3D2 1.00).
    pub fn header(&mut self, signature: &str) -> Result<i32> {
        let found = self.string()?;
        ensure!(found == signature, "expected {signature}, found {found}");
        self.i32()
    }
}

/// One material property, in file order.
#[derive(Clone, Debug, PartialEq)]
pub enum Property {
    /// `tex`/`null`.
    NoTexture,
    /// `tex`/`tex2d`: texture file stem, source path, offset and scale.
    Texture {
        name: String,
        path: String,
        offset: [f32; 2],
        scale: [f32; 2],
    },
    /// `tex`/`texRT`: a render texture bound by the game at run time.
    RenderTexture { name: String, path: String },
    Color([f32; 4]),
    Vector([f32; 4]),
    Float(f32),
}

/// `ImportCM.ReadMaterial`: name, shader, default-material resource, properties.
#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    pub name: String,
    pub shader: String,
    pub default_material: String,
    pub properties: Vec<(String, Property)>,
}

impl Material {
    pub fn read(r: &mut Reader) -> Result<Self> {
        let name = r.string()?;
        let shader = r.string()?;
        let default_material = r.string()?;
        let mut properties = Vec::new();
        loop {
            let kind = r.string()?;
            if kind == "end" {
                break;
            }
            ensure!(properties.len() < 1024, "CM3D2 material property limit");
            let property = r.string()?;
            let value = match kind.as_str() {
                "tex" => match r.string()?.as_str() {
                    "null" => Property::NoTexture,
                    "tex2d" => Property::Texture {
                        name: r.string()?,
                        path: r.string()?,
                        offset: r.f32s()?,
                        scale: r.f32s()?,
                    },
                    "texRT" => Property::RenderTexture { name: r.string()?, path: r.string()? },
                    other => bail!("unknown CM3D2 texture kind {other}"),
                },
                "col" => Property::Color(r.f32s()?),
                "vec" => Property::Vector(r.f32s()?),
                "f" => Property::Float(r.f32()?),
                // The original logs and then misreads the stream; stop instead.
                other => bail!("invalid CM3D2 material property type {other}"),
            };
            properties.push((property, value));
        }
        Ok(Self { name, shader, default_material, properties })
    }
    /// A `.mate` file: header, file name, then the material body.
    pub fn parse_file(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);
        let version = r.header("CM3D2_MATERIAL")?;
        ensure!(version >= 1000, "unsupported CM3D2 material version {version}");
        let _name = r.string()?;
        Self::read(&mut r)
    }
    pub fn get(&self, property: &str) -> Option<&Property> {
        self.properties.iter().rev().find(|(name, _)| name == property).map(|(_, value)| value)
    }
    pub fn color(&self, property: &str) -> Option<[f32; 4]> {
        match self.get(property)? {
            Property::Color(c) | Property::Vector(c) => Some(*c),
            _ => None,
        }
    }
    pub fn float(&self, property: &str) -> Option<f32> {
        match self.get(property)? {
            Property::Float(v) => Some(*v),
            _ => None,
        }
    }
    pub fn texture(&self, property: &str) -> Option<(&str, [f32; 2], [f32; 2])> {
        match self.get(property)? {
            Property::Texture { name, offset, scale, .. } => Some((name, *offset, *scale)),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bone {
    pub name: String,
    /// Creates the `<name>_SCL_` child that skinning binds to.
    pub scale_node: bool,
    /// Optional authored local scale, introduced in mesh version 2001.
    pub scale: [f32; 3],
    pub parent: Option<usize>,
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Morph {
    pub name: String,
    /// Vertex index, position delta, normal delta.
    pub vertices: Vec<(u16, [f32; 3], [f32; 3])>,
}

/// A decoded `.model`, in Unity (source) space.
#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    pub version: i32,
    pub name: String,
    /// Bone that carries the SkinnedMeshRenderer.
    pub root_bone: String,
    pub bones: Vec<Bone>,
    /// Skin bone names (resolved to the `_SCL_` child when present).
    pub skin_bones: Vec<String>,
    /// Column-major bind poses, one per skin bone.
    pub bind_poses: Vec<[f32; 16]>,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub tangents: Vec<[f32; 4]>,
    pub joints: Vec<[u16; 4]>,
    pub weights: Vec<[f32; 4]>,
    pub submeshes: Vec<Vec<u16>>,
    pub materials: Vec<Material>,
    pub morphs: Vec<Morph>,
}

impl Mesh {
    /// Drop triangles touching an invisible skin bone (`TMorph.FixVisibleFlag`):
    /// a triangle goes when any vertex's first bone, or any further bone with
    /// a positive weight, is hidden. `visible` is indexed like `skin_bones`;
    /// missing entries count as visible.
    pub fn hide_skin_bones(&mut self, visible: &[bool]) {
        let shown = |b: u16| visible.get(b as usize).copied().unwrap_or(true);
        let vertex_shown = |v: u16| {
            let (Some(j), Some(w)) = (self.joints.get(v as usize), self.weights.get(v as usize)) else { return true };
            shown(j[0]) && (1..4).all(|k| w[k] <= 0. || shown(j[k]))
        };
        let submeshes = std::mem::take(&mut self.submeshes);
        self.submeshes = submeshes
            .into_iter()
            .map(|tris| tris.chunks_exact(3).filter(|t| t.iter().all(|&v| vertex_shown(v))).flatten().copied().collect())
            .collect();
    }

    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);
        let version = r.header("CM3D2_MESH")?;
        ensure!(version >= 1000, "unsupported CM3D2 mesh version {version}");
        let name = r.string()?;
        let root_bone = r.string()?;
        let count = r.count(2)?;
        ensure!(count <= MAX_BONES, "CM3D2 bone limit");
        let mut bones = Vec::with_capacity(count);
        for _ in 0..count {
            bones.push(Bone {
                name: r.string()?,
                scale_node: r.bool()?,
                scale: [1.; 3],
                parent: None,
                translation: [0.; 3],
                rotation: [0., 0., 0., 1.],
            });
        }
        for index in 0..count {
            let parent = r.i32()?;
            bones[index].parent = if parent >= 0 {
                ensure!((parent as usize) < count && parent as usize != index, "CM3D2 bone parent");
                Some(parent as usize)
            } else {
                None
            };
        }
        for bone in &mut bones {
            bone.translation = r.f32s()?;
            bone.rotation = r.f32s()?;
            if version >= 2001 && r.bool()? {
                bone.scale = r.f32s()?;
            }
        }
        // Reject parent cycles before anything walks the hierarchy.
        for start in 0..count {
            let (mut at, mut steps) = (Some(start), 0);
            while let Some(i) = at {
                steps += 1;
                ensure!(steps <= count, "CM3D2 bone hierarchy cycle");
                at = bones[i].parent;
            }
        }
        let vertex_count = r.count(32)?;
        ensure!(vertex_count <= MAX_VERTICES, "CM3D2 vertex limit");
        let submesh_count = r.count(4)?;
        let skin_count = r.count(1)?;
        ensure!(skin_count <= MAX_BONES, "CM3D2 skin bone limit");
        let mut skin_bones = Vec::with_capacity(skin_count);
        for _ in 0..skin_count {
            skin_bones.push(r.string()?);
        }
        let mut bind_poses = Vec::with_capacity(skin_count);
        for _ in 0..skin_count {
            bind_poses.push(r.f32s()?);
        }
        let mut positions = Vec::with_capacity(vertex_count);
        let mut normals = Vec::with_capacity(vertex_count);
        let mut uvs = Vec::with_capacity(vertex_count);
        for _ in 0..vertex_count {
            positions.push(r.f32s()?);
            normals.push(r.f32s()?);
            uvs.push(r.f32s()?);
        }
        let tangent_count = r.count(16)?;
        ensure!(tangent_count == 0 || tangent_count == vertex_count, "CM3D2 tangent count");
        let mut tangents = Vec::with_capacity(tangent_count);
        for _ in 0..tangent_count {
            tangents.push(r.f32s()?);
        }
        ensure!(r.remaining() >= vertex_count * 24, "truncated CM3D2 bone weights");
        let mut joints = Vec::with_capacity(vertex_count);
        let mut weights = Vec::with_capacity(vertex_count);
        for _ in 0..vertex_count {
            let j = [r.u16()?, r.u16()?, r.u16()?, r.u16()?];
            let w: [f32; 4] = r.f32s()?;
            for k in 0..4 {
                ensure!(w[k] == 0. || (j[k] as usize) < skin_count, "CM3D2 bone weight index");
            }
            joints.push(j);
            weights.push(w);
        }
        let mut submeshes = Vec::with_capacity(submesh_count);
        for _ in 0..submesh_count {
            let n = r.count(2)?;
            let mut indices = Vec::with_capacity(n);
            for _ in 0..n {
                let i = r.u16()?;
                ensure!((i as usize) < vertex_count, "CM3D2 triangle index");
                indices.push(i);
            }
            ensure!(indices.len() % 3 == 0, "CM3D2 sub-mesh is not a triangle list");
            submeshes.push(indices);
        }
        let material_count = r.count(3)?;
        let mut materials = Vec::with_capacity(material_count);
        for _ in 0..material_count {
            materials.push(Material::read(&mut r)?);
        }
        let mut morphs = Vec::new();
        loop {
            match r.string()?.as_str() {
                "end" => break,
                "morph" => {
                    let name = r.string()?;
                    let n = r.count(26)?;
                    let mut vertices = Vec::with_capacity(n);
                    for _ in 0..n {
                        let index = r.u16()?;
                        ensure!((index as usize) < vertex_count, "CM3D2 morph vertex index");
                        vertices.push((index, r.f32s()?, r.f32s()?));
                    }
                    morphs.push(Morph { name, vertices });
                    ensure!(morphs.len() <= crate::pbr::MAX_MORPH_TARGETS, "CM3D2 morph limit");
                }
                // The original ignores unknown tags and keeps reading names.
                _ => ensure!(r.remaining() > 0, "unterminated CM3D2 mesh trailer"),
            }
        }
        Ok(Self {
            version,
            name,
            root_bone,
            bones,
            skin_bones,
            bind_poses,
            positions,
            normals,
            uvs,
            tangents,
            joints,
            weights,
            submeshes,
            materials,
            morphs,
        })
    }
}

/// One `.anm` curve set for a bone path. Times are shared by the curves of a
/// group in the 1.00 data; differing times are resampled at their union.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Curve {
    /// `time, value, inTangent, outTangent` keys.
    pub keys: Vec<[f32; 4]>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BoneTrack {
    /// Unity relative path (`Bip01/Bip01 Pelvis/...`).
    pub path: String,
    /// Index = property code − 100: rotation x, y, z, w, position x, y, z.
    pub curves: [Option<Curve>; 7],
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Animation {
    pub version: i32,
    pub tracks: Vec<BoneTrack>,
}

impl Animation {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);
        let version = r.header("CM3D2_ANIM")?;
        let mut tracks: Vec<BoneTrack> = Vec::new();
        loop {
            match r.u8()? {
                0 => break,
                1 => {
                    ensure!(tracks.len() < MAX_BONES, "CM3D2 animation bone limit");
                    tracks.push(BoneTrack { path: r.string()?, ..Default::default() });
                }
                code @ 100..=106 => {
                    let n = r.count(16)?;
                    let mut keys = Vec::with_capacity(n);
                    for _ in 0..n {
                        keys.push(r.f32s()?);
                    }
                    let track = tracks.last_mut().context("CM3D2 curve before its bone path")?;
                    track.curves[usize::from(code - 100)] = Some(Curve { keys });
                }
                // The original logs `com <code>` and continues at the next byte.
                code => bail!("unknown CM3D2 animation command {code}"),
            }
        }
        Ok(Self { version, tracks })
    }
}

/// Unity Hermite evaluation of one curve (`AnimationCurve.Evaluate` with
/// clamped ends; infinite tangents hold the left key).
pub fn evaluate(keys: &[[f32; 4]], t: f32) -> f32 {
    let Some(first) = keys.first() else { return 0. };
    if t <= first[0] || keys.len() == 1 {
        return first[1];
    }
    let last = keys[keys.len() - 1];
    if t >= last[0] {
        return last[1];
    }
    let i = keys.partition_point(|k| k[0] <= t) - 1;
    let (a, b) = (keys[i], keys[i + 1]);
    let dt = b[0] - a[0];
    if dt <= 0. || !a[3].is_finite() || !b[2].is_finite() {
        return a[1];
    }
    let s = (t - a[0]) / dt;
    let (s2, s3) = (s * s, s * s * s);
    a[1] * (2. * s3 - 3. * s2 + 1.)
        + a[3] * dt * (s3 - 2. * s2 + s)
        + b[1] * (-2. * s3 + 3. * s2)
        + b[2] * dt * (s3 - s2)
}

fn mirror_point(v: [f32; 3]) -> [f32; 3] {
    [-v[0], v[1], v[2]]
}
fn mirror_quat(q: [f32; 4]) -> [f32; 4] {
    [q[0], -q[1], -q[2], q[3]]
}
/// `S·M·S` with S = diag(-1, 1, 1, 1), column-major.
fn mirror_matrix(m: [f32; 16]) -> [f32; 16] {
    let mut out = m;
    for col in 0..4 {
        for row in 0..4 {
            let sign = if (row == 0) != (col == 0) { -1. } else { 1. };
            out[col * 4 + row] = m[col * 4 + row] * sign;
        }
    }
    out
}

/// How `.mate`/model shader properties become an engine material.
///
/// The `CM3D2/Toony_Lighted*` family maps to the engine toon model with the
/// semantics of their compiled forward passes; `table` is the `_MultiColTex`
/// colour table (`MaidParts`, 256 RGBA texels) when `_UseMulticolTex` is on.
/// SATD-cm3d2-rendering (crates/crysirror-cm3d2/SATD.md): non-toon shaders.
fn engine_material(
    m: &Material,
    images: &mut Images,
    doc: &mut glb::Document,
    table: Option<&[[u8; 4]; 256]>,
    layers: &[&TextureLayer],
    palettes: Option<&TexturePalettes>,
    dropped: &mut Vec<String>,
) -> PbrMaterial {
    let shader = m.shader.as_str();
    let mut out = PbrMaterial { roughness_factor: 0.9, specular_factor: 0.2, ..Default::default() };
    let toon = shader.starts_with("CM3D2/Toony_Lighted") || shader.starts_with("CM3D2/Lighted");
    if let Some([r, g, b, a]) = m.color("_Color") {
        out.base_color_factor = [r, g, b, a].map(|c| c.clamp(0., 1.));
    } else if toon {
        // Shader default `_Color` (0.5, 0.5, 0.5, 1).
        out.base_color_factor = [0.5, 0.5, 0.5, 1.];
    }
    if let Some((name, offset, scale)) = m.texture("_MainTex") {
        out.base_color_texture = images.embed_layered(doc, name, "_MainTex", layers, palettes);
        // Unity samples `uv * scale + offset` bottom-up; the importer flips V.
        out.uv_scale = scale;
        out.uv_offset = [offset[0], 1. - scale[1] - offset[1]];
    }
    out.alpha_mode = if shader.contains("Trans") {
        AlphaMode::Blend
    } else {
        AlphaMode::Opaque
    };
    match shader {
        "CM3D2/Toony_Lighted_Outline"
        | "CM3D2/Toony_Lighted_Trans"
        | "CM3D2/Toony_Lighted_Outline_Tex"
        | "CM3D2/Toony_Lighted_Hair_Outline_Tex"
        | "CM3D2/Toony_Lighted_Hair_Outline"
        | "CM3D2/Toony_Lighted_Hair"
        | "CM3D2/Toony_Lighted_Outline_Trans"
        | "CM3D2/Toony_Lighted"
        | "CM3D2/Toony_Lighted_Trans_NoZ"
        | "CM3D2/Lighted"
        | "CM3D2/Lighted_Trans" => out.toon = Some(toon_shading(m, images, doc, table, layers, palettes)),
        "Diffuse" => {}
        "CM3D2/Man" => {
            // A translucent silhouette (`_Color` alpha = CMSystem.ManAlpha).
            out.unlit = true;
            out.alpha_mode = AlphaMode::Blend;
        }
        "CM3D2/Mosaic" => {
            out.alpha_mode = AlphaMode::Blend;
            out.base_color_factor = [0.5, 0.5, 0.5, 1.];
            dropped.push(format!("material `{}`: mosaic render texture", m.name));
        }
        other => dropped.push(format!("material `{}`: unknown shader {other}", m.name)),
    }
    out
}

/// The engine toon model of a CM3D2 lit shader, without textures: the
/// `CM3D2/Toony_Lighted*` family, and `CM3D2/Lighted*`, whose forward pass
/// is `c · light + pow(n·h, 48) · shininess + c · ambient` (no ramp, rim or
/// shadow layer; bake its lookup with a white ramp). Shader property
/// defaults apply where the material omits one.
pub fn toon_parameters(
    shader: &str,
    color: &dyn Fn(&str) -> Option<[f32; 4]>,
    float: &dyn Fn(&str) -> Option<f32>,
) -> crysirror_scene::material::ToonShading<usize> {
    let rgb = |name: &str, d: [f32; 3]| color(name).map_or(d, |c| [c[0], c[1], c[2]].map(|x| x.clamp(0., 16.)));
    let f = |name: &str, d: f32| float(name).unwrap_or(d);
    let toony = shader.starts_with("CM3D2/Toony");
    let hair = shader.contains("Hair");
    crysirror_scene::material::ToonShading {
        lookup_texture: None,
        outline_texture: None,
        shadow_texture: None,
        highlight_texture: None,
        blend_texture: None,
        liquid_blobs: None,
        liquid_regions: None,
        liquid_tiling: [1., 1., 0., 0.],
        liquid_color: [1.; 3],
        color_table: false,
        rim_color: if toony { rgb("_RimColor", [0., 0., 1.]) } else { [0.; 3] },
        rim_power: f("_RimPower", 3.).clamp(0., 64.),
        rim_shift: if toony { f("_RimShift", 0.).clamp(-1., 1.) } else { 0. },
        shadow_color: rgb("_ShadowColor", [0.5; 3]),
        cast_shadow_ramp: false,
        ambient_scale: 1.,
        light_limit: 0.,
        shade_from_base: false,
        shadow_rim: [0.; 4],
        shadow_light: [0.; 4],
        shade_kind: Default::default(),
        shade_color: [0.5; 3],
        shadow_tone_override: [0.; 4],
        surface_line_texture: None,
        line_model: Default::default(),
        line_color: [0.; 4],
        shininess: f("_Shininess", 0.).clamp(0., 16.),
        highlight_rate: if hair { f("_HiRate", 0.).clamp(0., 16.) } else { 0. },
        highlight_power: if hair { f("_HiPow", 0.).clamp(0., 256.) } else { 1. },
        outline_width: if shader.contains("Outline") { f("_OutlineWidth", 0.0016).clamp(0., 0.1) } else { 0. },
        outline_color: color("_OutlineColor").unwrap_or([0., 0., 0., 1.]).map(|x| x.clamp(0., 16.)),
    }
}

/// The 256×3 toon lookup image: the diffuse ramp (row 0) and shadow rate
/// (row 1) sampled at (u, u) as the forward pass does, and the colour table
/// (row 2). A missing ramp is the shader default grey, unless `white_ramp`
/// (`CM3D2/Lighted`); a missing rate is white.
pub fn toon_lookup(
    ramp: Option<&crysirror_image::ImageSurface>,
    rate: Option<&crysirror_image::ImageSurface>,
    table: Option<&[[u8; 4]; 256]>,
    white_ramp: bool,
) -> crysirror_image::ImageSurface {
    let mut lut = vec![0u8; 256 * 3 * 4];
    let flat = if white_ramp { [1.; 4] } else { [0.5, 0.5, 0.5, 1.] };
    for x in 0..256 {
        let u = x as f32 / 255.;
        let r = ramp.map_or(flat, |img| sample_bilinear(img, u, u));
        let s = rate.map_or([1.; 4], |img| sample_bilinear(img, u, u));
        let t = table.map_or([0; 4], |t| t[x]);
        lut[x * 4..x * 4 + 4].copy_from_slice(&r.map(|c| (c * 255.).round() as u8));
        lut[(256 + x) * 4..(256 + x) * 4 + 4].copy_from_slice(&s.map(|c| (c * 255.).round() as u8));
        lut[(512 + x) * 4..(512 + x) * 4 + 4].copy_from_slice(&[t[0], t[1], t[2], 255]);
    }
    crysirror_image::ImageSurface { width: 256, height: 3, rgba: lut }
}

/// Toon shading of a `.mate`/model material with its baked lookup texture.
fn toon_shading(
    m: &Material,
    images: &mut Images,
    doc: &mut glb::Document,
    table: Option<&[[u8; 4]; 256]>,
    layers: &[&TextureLayer],
    palettes: Option<&TexturePalettes>,
) -> crysirror_scene::material::ToonShading<usize> {
    let mut out = toon_parameters(&m.shader, &|n| m.color(n), &|n| m.float(n));
    let toony = m.shader.starts_with("CM3D2/Toony");
    let use_table = toony && !m.shader.ends_with("_Outline_Tex") && table.is_some() && m.float("_UseMulticolTex").unwrap_or(0.) != 0.;
    let ramp = if toony { m.texture("_ToonRamp").and_then(|(n, ..)| images.decode_property(n, "_ToonRamp", palettes)) } else { None };
    let rate = if toony { m.texture("_ShadowRateToon").and_then(|(n, ..)| images.decode_property(n, "_ShadowRateToon", palettes)) } else { None };
    let mut lut = toon_lookup(ramp.as_ref(), rate.as_ref(), table.filter(|_| use_table), !toony);
    if m.shader.ends_with("_Outline_Tex") {
        let ramp = m.texture("_OutlineToonRamp").and_then(|(n, ..)| images.decode_property(n, "_OutlineToonRamp", palettes));
        for (x, pixel) in lut.rgba[512*4..].chunks_exact_mut(4).enumerate() {
            let u = x as f32 / 255.;
            let color = ramp.as_ref().map_or([0.5,0.5,0.5,1.], |r| sample_bilinear(r, u, u));
            pixel.copy_from_slice(&color.map(|v| (v * 255.).round() as u8));
        }
        out.outline_texture = m.texture("_OutlineTex").and_then(|(n, ..)| images.embed_layered(doc, n, "_OutlineTex", layers, palettes))
            .or_else(|| {
                let gray = crysirror_image::ImageSurface {width:1, height:1, rgba:vec![128,128,128,255]};
                let png = crysirror_image::encode_png(&gray).ok()?;
                doc.images.push((png, "image/png"));
                Some(doc.images.len()-1)
            });
    }
    out.lookup_texture = crysirror_image::encode_png(&lut).ok().map(|png| {
        doc.images.push((png, "image/png"));
        doc.images.len() - 1
    });
    out.color_table = use_table && out.lookup_texture.is_some();
    if toony {
        out.shadow_texture = m.texture("_ShadowTex").and_then(|(n, ..)| images.embed_layered(doc, n, "_ShadowTex", layers, palettes));
    }
    if m.shader.contains("Hair") {
        out.highlight_texture = m.texture("_HiTex").and_then(|(n, ..)| images.embed_layered(doc, n, "_HiTex", layers, palettes));
    }
    out
}

/// Unity bilinear sample with repeat wrapping, `v` bottom-up.
fn sample_bilinear(img: &crysirror_image::ImageSurface, u: f32, v: f32) -> [f32; 4] {
    let (w, h) = (img.width.max(1) as i64, img.height.max(1) as i64);
    let x = u * w as f32 - 0.5;
    let y = (1. - v) * h as f32 - 0.5;
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let texel = |xi: i64, yi: i64| -> [f32; 4] {
        let (xi, yi) = (xi.rem_euclid(w), yi.rem_euclid(h));
        let o = ((yi * w + xi) * 4) as usize;
        let px = img.rgba.get(o..o + 4).unwrap_or(&[0, 0, 0, 255]);
        [px[0], px[1], px[2], px[3]].map(|c| c as f32 / 255.)
    };
    let (a, b, c, d) = (texel(x0 as i64, y0 as i64), texel(x0 as i64 + 1, y0 as i64), texel(x0 as i64, y0 as i64 + 1), texel(x0 as i64 + 1, y0 as i64 + 1));
    std::array::from_fn(|k| (a[k] * (1. - fx) + b[k] * fx) * (1. - fy) + (c[k] * (1. - fx) + d[k] * fx) * fy)
}

/// Loads each `.tex`/PNG once and embeds the encoded image.
struct Images<'e> {
    external: &'e mut dyn FnMut(&str) -> Result<Vec<u8>>,
    cache: HashMap<(String, bool), Option<usize>>,
    decoded: HashMap<String, Option<crysirror_image::ImageSurface>>,
}
impl Images<'_> {
    /// Decoded pixels of `<name>.tex` (for baking), cached.
    fn decode(&mut self, name: &str) -> Option<crysirror_image::ImageSurface> {
        if let Some(hit) = self.decoded.get(name) {
            return hit.clone();
        }
        let file = format!("{name}.tex");
        let surface = (self.external)(&file).ok().and_then(|bytes| {
            let payload = if bytes.starts_with(b"\x09CM3D2_TEX") {
                crysirror_image::kiss_tex::payload(&bytes).ok()?.to_vec()
            } else {
                bytes
            };
            crysirror_image::decode_standard(&payload).ok()
        });
        self.decoded.insert(name.to_owned(), surface.clone());
        surface
    }
    fn decode_property(&mut self, name: &str, property: &str, palettes: Option<&TexturePalettes>) -> Option<crysirror_image::ImageSurface> {
        let mut image = self.decode(name)?;
        if let Some(palette) = palettes.and_then(|p| p.get(property)) {
            palette.apply(&mut image);
        }
        Some(image)
    }
    /// [`Self::embed`] with the `property` layers of `layers` composed over it.
    fn embed_layered(&mut self, doc: &mut glb::Document, name: &str, property: &str, layers: &[&TextureLayer], palettes: Option<&TexturePalettes>) -> Option<usize> {
        let mut mine: Vec<&TextureLayer> = layers.iter().copied().filter(|l| l.property == property).collect();
        if mine.is_empty() && !palettes.is_some_and(|p| p.contains_key(property)) {
            return self.embed(doc, name, property == "_MainTex");
        }
        // MulTexProc: layers in order; a layer's LaySets in insertion order.
        mine.sort_by_key(|l| l.layer);
        let Some(mut base) = self.decode_property(name, property, palettes) else {
            doc.dropped.push(format!("texture `{name}.tex`: layered base not decodable"));
            return None;
        };
        for l in mine {
            let stem = l.file.strip_suffix(".tex").unwrap_or(&l.file);
            let Some(src) = self.decode(stem) else {
                doc.dropped.push(format!("texture layer `{}` missing", l.file));
                continue;
            };
            let (w, h) = (base.width, base.height);
            if let Some((cx, cy, rot, scale)) = l.placement {
                // GL.LoadPixelMatrix(0, w, h, 0) · TRS(pos, rotZ, scale), the
                // texture drawn over (−tw/2, −th/2, tw, th).
                let (sin, cos) = rot.to_radians().sin_cos();
                let (tw, th) = (src.width as f32, src.height as f32);
                for y in 0..h {
                    for x in 0..w {
                        let (dx, dy) = (x as f32 + 0.5 - cx as f32, y as f32 + 0.5 - cy as f32);
                        let (lx, ly) = ((dx * cos + dy * sin) / scale.max(1e-6), (-dx * sin + dy * cos) / scale.max(1e-6));
                        let (u, v) = (lx / tw + 0.5, ly / th + 0.5);
                        if !(0.0..1.).contains(&u) || !(0.0..1.).contains(&v) {
                            continue;
                        }
                        let s = sample_bilinear(&src, u, 1. - v);
                        let o = ((y * w + x) * 4) as usize;
                        for k in 0..3 {
                            let d = base.rgba[o + k] as f32 / 255.;
                            base.rgba[o + k] = ((s[k] * s[3] + d * (1. - s[3])).clamp(0., 1.) * 255.).round() as u8;
                        }
                    }
                }
                continue;
            }
            for y in 0..h {
                for x in 0..w {
                    // Blit: the layer stretched over the target, sampled at texel centres.
                    let s = sample_bilinear(&src, (x as f32 + 0.5) / w as f32, 1. - (y as f32 + 0.5) / h as f32);
                    let o = ((y * w + x) * 4) as usize;
                    for k in 0..3 {
                        let d = base.rgba[o + k] as f32 / 255.;
                        let v = match l.blend {
                            LayerBlend::Alpha => s[k] * s[3] + d * (1. - s[3]),
                            LayerBlend::Multiply => d * (1. + (s[k] - 1.) * s[3]),
                        };
                        base.rgba[o + k] = (v.clamp(0., 1.) * 255.).round() as u8;
                    }
                }
            }
        }
        match crysirror_image::encode_png(&base) {
            Ok(png) => {
                doc.images.push((png, "image/png"));
                Some(doc.images.len() - 1)
            }
            Err(e) => {
                doc.dropped.push(format!("texture `{name}.tex` layers: {e:#}"));
                None
            }
        }
    }
    fn embed(&mut self, doc: &mut glb::Document, name: &str, srgb: bool) -> Option<usize> {
        let key = (name.to_owned(), srgb);
        if let Some(hit) = self.cache.get(&key) {
            return *hit;
        }
        let file = format!("{name}.tex");
        let result = (self.external)(&file).and_then(|bytes| {
            let payload = if bytes.starts_with(b"\x09CM3D2_TEX") {
                crysirror_image::kiss_tex::payload(&bytes)?.to_vec()
            } else {
                bytes
            };
            let mime = if payload.starts_with(b"\x89PNG") {
                "image/png"
            } else if payload.starts_with(b"\xff\xd8\xff") {
                "image/jpeg"
            } else {
                bail!("texture is neither PNG nor JPEG")
            };
            doc.images.push((payload, mime));
            Ok(doc.images.len() - 1)
        });
        let index = match result {
            Ok(i) => Some(i),
            Err(e) => {
                doc.dropped.push(format!("texture `{file}`: {e:#}"));
                None
            }
        };
        self.cache.insert(key, index);
        index
    }
}

/// Convert a `.model` into a GLB document. Textures are resolved by file name
/// (`<stem>.tex`) through `external`, the game's flat archive namespace.
pub fn convert_mesh(bytes: &[u8], external: &mut dyn FnMut(&str) -> Result<Vec<u8>>) -> Result<glb::Document> {
    convert_mesh_with(bytes, external, &MaterialEdits::default())
}

/// A per-texture colour lookup applied before material sampling and layer composition.
#[derive(Clone, Debug, PartialEq)]
pub struct TexturePalette {
    pub colors: [[u8; 4]; 256],
    /// Single-channel source render textures sample with an opaque alpha.
    pub opaque_source: bool,
}

pub type TexturePalettes = BTreeMap<String, TexturePalette>;

impl TexturePalette {
    fn apply(&self, image: &mut crysirror_image::ImageSurface) {
        // Unity 5.6 InfinityColor fragment program: sample the colour table at
        // (base.r * 0.995, 0.5) with bilinear repeat, preserving base alpha.
        let lut: [[u8; 3]; 256] = std::array::from_fn(|r| {
            let u = r as f32 / 255. * 0.995 * 256. - 0.5;
            let i = u.floor() as i32;
            let f = u - i as f32;
            let a = self.colors[i.rem_euclid(256) as usize];
            let b = self.colors[(i + 1).rem_euclid(256) as usize];
            std::array::from_fn(|k| (f32::from(a[k]) + (f32::from(b[k]) - f32::from(a[k])) * f).round() as u8)
        });
        for pixel in image.rgba.chunks_exact_mut(4) {
            let rgb = lut[pixel[0] as usize];
            pixel[..3].copy_from_slice(&rgb);
            if self.opaque_source { pixel[3] = 255; }
        }
    }
}

/// Material changes a game applies to a loaded mesh (`TBody.ChangeMaterial`,
/// `ChangeTex`, `ChangeCol`), by material index.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MaterialEdits {
    /// Whole `.mate` replacements.
    pub replace: BTreeMap<usize, Material>,
    /// `(material, property, texture stem)`.
    pub textures: Vec<(usize, String, String)>,
    /// `(material, property, rgba)`.
    pub colors: Vec<(usize, String, [f32; 4])>,
    /// `_MultiColTex` colour tables by material (`MaidParts` parts colour).
    pub color_tables: BTreeMap<usize, [[u8; 4]; 256]>,
    /// Per-property palette bindings in the Unity 5.6 texture cache.
    pub texture_palettes: BTreeMap<usize, TexturePalettes>,
    /// `_UseMulticolTex` set by a texture change, by material.
    pub multicolor: BTreeMap<usize, bool>,
    /// `TBody.MulTexSet` layers, composed in layer order over the texture of
    /// their material property (`MulTexProc`).
    pub layers: Vec<TextureLayer>,
}

/// One `テクスチャ合成` layer.
#[derive(Clone, Debug, PartialEq)]
pub struct TextureLayer {
    pub material: usize,
    pub property: String,
    pub layer: i32,
    /// Texture file name (`.tex`).
    pub file: String,
    pub blend: LayerBlend,
    /// `LaySet` of an added layer: centre pixel (x, y from the top),
    /// rotation (degrees) and scale; the texture is alpha-drawn at its own
    /// size (`Graphics.DrawTexture`). `None` stretches it over the target.
    pub placement: Option<(i32, i32, f32, f32)>,
}

/// `GameUty.SystemMaterial` blits: `2dAlpha` (source over) and `2dMultiply`
/// (destination × lerp(1, source, source alpha)); the layer is stretched to
/// the base texture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerBlend {
    Alpha,
    Multiply,
}

impl MaterialEdits {
    fn apply(&self, index: usize, m: &Material) -> Material {
        let mut m = self.replace.get(&index).cloned().unwrap_or_else(|| m.clone());
        for (i, prop, name) in &self.textures {
            if *i == index {
                let (offset, scale) = match m.get(prop) {
                    Some(Property::Texture { offset, scale, .. }) => (*offset, *scale),
                    _ => ([0.; 2], [1.; 2]),
                };
                let stem = name.strip_suffix(".tex").unwrap_or(name).to_owned();
                m.properties.push((prop.clone(), Property::Texture { name: stem, path: String::new(), offset, scale }));
            }
        }
        for (i, prop, color) in &self.colors {
            if *i == index {
                m.properties.push((prop.clone(), Property::Color(*color)));
            }
        }
        if let Some(on) = self.multicolor.get(&index) {
            m.properties.push(("_UseMulticolTex".into(), Property::Float(if *on { 1. } else { 0. })));
        }
        m
    }
}

pub fn convert_mesh_with(
    bytes: &[u8],
    external: &mut dyn FnMut(&str) -> Result<Vec<u8>>,
    edits: &MaterialEdits,
) -> Result<glb::Document> {
    convert_parsed(Mesh::parse(bytes)?, external, edits)
}

/// [`convert_mesh_with`] over an already parsed (and possibly edited) mesh.
pub fn convert_parsed(
    mut mesh: Mesh,
    external: &mut dyn FnMut(&str) -> Result<Vec<u8>>,
    edits: &MaterialEdits,
) -> Result<glb::Document> {
    mesh.materials = mesh.materials.iter().enumerate().map(|(i, m)| edits.apply(i, m)).collect();
    let mut doc = glb::Document { generator: "CrySirror CM3D2 model importer".into(), ..Default::default() };
    // Nodes: every bone, then the `_SCL_` children.
    let mut nodes: Vec<glb::Node> = mesh
        .bones
        .iter()
        .map(|b| glb::Node {
            translation: mirror_point(b.translation),
            rotation: mirror_quat(b.rotation),
            scale: b.scale,
            ..glb::Node::named(&b.name)
        })
        .collect();
    let mut by_name: HashMap<&str, usize> = HashMap::new();
    for (i, b) in mesh.bones.iter().enumerate() {
        by_name.entry(b.name.as_str()).or_insert(i);
    }
    let mut skin_target: HashMap<&str, usize> = by_name.clone();
    for (i, b) in mesh.bones.iter().enumerate() {
        match b.parent {
            Some(p) => nodes[p].children.push(i),
            None => doc.roots.push(i),
        }
        if b.scale_node {
            let child = nodes.len();
            nodes.push(glb::Node::named(format!("{}_SCL_", b.name)));
            nodes[i].children.push(child);
            skin_target.insert(b.name.as_str(), child);
        }
    }
    let carrier = *by_name.get(mesh.root_bone.as_str()).with_context(|| {
        format!("CM3D2 mesh renderer bone {} missing", mesh.root_bone)
    })?;
    // Skin. The original skips unknown bone names (leaving a null bone); a
    // vertex weighted to such a bone would be undefined, so it is an error.
    let mut joints = Vec::with_capacity(mesh.skin_bones.len());
    for (k, name) in mesh.skin_bones.iter().enumerate() {
        match skin_target.get(name.as_str()) {
            Some(node) => joints.push(*node),
            None => {
                ensure!(
                    !mesh.joints.iter().zip(&mesh.weights).any(|(j, w)| (0..4).any(|i| j[i] as usize == k && w[i] > 0.)),
                    "CM3D2 skin bone {name} is weighted but missing"
                );
                doc.dropped.push(format!("skin bone `{name}` missing from the hierarchy"));
                joints.push(carrier);
            }
        }
    }
    let skin = if joints.is_empty() {
        None
    } else {
        doc.skins.push(glb::Skin {
            joints,
            inverse_bind: mesh.bind_poses.iter().map(|m| mirror_matrix(*m)).collect(),
            skeleton: None,
        });
        Some(0)
    };
    // Materials.
    let mut images = Images { external, cache: HashMap::new(), decoded: HashMap::new() };
    for (index, m) in mesh.materials.iter().enumerate() {
        let mut dropped = Vec::new();
        let layers: Vec<&TextureLayer> = edits.layers.iter().filter(|l| l.material == index).collect();
        let engine = engine_material(m, &mut images, &mut doc, edits.color_tables.get(&index), &layers, edits.texture_palettes.get(&index), &mut dropped);
        doc.dropped.extend(dropped);
        doc.materials.push(glb::Material {
            name: m.name.clone(),
            base_color: engine.base_color_factor,
            base_color_texture: engine.base_color_texture,
            alpha_mode: match engine.alpha_mode {
                AlphaMode::Opaque => "OPAQUE",
                AlphaMode::Mask => "MASK",
                _ => "BLEND",
            },
            roughness: engine.roughness_factor,
            engine: Some(engine),
            ..Default::default()
        });
    }
    // One primitive per sub-mesh, compacted to the vertices it uses so
    // shared vertex/morph arrays are not duplicated per primitive.
    let names: Vec<String> = mesh.morphs.iter().map(|m| m.name.clone()).collect();
    let mut dense: Vec<Vec<([f32; 3], [f32; 3])>> = Vec::with_capacity(mesh.morphs.len());
    for m in &mesh.morphs {
        let mut d = vec![([0.; 3], [0.; 3]); mesh.positions.len()];
        for (i, p, n) in &m.vertices {
            d[*i as usize] = (mirror_point(*p), mirror_point(*n));
        }
        dense.push(d);
    }
    let mut prims = Vec::with_capacity(mesh.submeshes.len());
    for (s, indices) in mesh.submeshes.iter().enumerate() {
        if indices.is_empty() {
            continue;
        }
        let mut remap: BTreeMap<u16, u32> = BTreeMap::new();
        let mut order = Vec::new();
        for i in indices {
            remap.entry(*i).or_insert_with(|| {
                order.push(*i as usize);
                (order.len() - 1) as u32
            });
        }
        let mut p = glb::Primitive {
            material: (s < doc.materials.len()).then_some(s),
            ..Default::default()
        };
        for &v in &order {
            p.positions.push(mirror_point(mesh.positions[v]));
            p.normals.push(mirror_point(mesh.normals[v]));
            let uv = mesh.uvs[v];
            p.uvs.push([uv[0], 1. - uv[1]]);
            if skin.is_some() {
                p.joints.push(mesh.joints[v]);
                p.weights.push(mesh.weights[v]);
            }
        }
        for tri in indices.chunks_exact(3) {
            // Mirroring flips handedness, so the winding is reversed.
            p.indices.extend([remap[&tri[0]], remap[&tri[2]], remap[&tri[1]]]);
        }
        for d in &dense {
            p.targets.push(order.iter().map(|v| d[*v].0).collect());
            p.target_normals.push(order.iter().map(|v| d[*v].1).collect());
        }
        p.target_names = names.clone();
        prims.push(p);
    }
    if mesh.submeshes.len() != mesh.materials.len() {
        doc.dropped.push(format!(
            "{} sub-meshes with {} materials",
            mesh.submeshes.len(),
            mesh.materials.len()
        ));
    }
    if !prims.is_empty() {
        doc.meshes.push((mesh.name.clone(), prims));
        nodes[carrier].mesh = Some(0);
        nodes[carrier].skin = skin;
    }
    doc.nodes = nodes;
    Ok(doc)
}

/// Convert an `.anm` into a GLB of its bone paths and one clip named `clip`.
/// Engine retargeting binds the clip to a model by node name.
pub fn convert_animation(bytes: &[u8], clip: &str) -> Result<glb::Document> {
    let anim = Animation::parse(bytes)?;
    let mut doc = glb::Document { generator: "CrySirror CM3D2 animation importer".into(), ..Default::default() };
    let mut path_nodes: HashMap<String, usize> = HashMap::new();
    let mut node_for = |doc: &mut glb::Document, path: &str| -> usize {
        let mut parent: Option<usize> = None;
        let mut prefix = String::new();
        for part in path.split('/') {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            let index = match path_nodes.get(&prefix) {
                Some(i) => *i,
                None => {
                    doc.nodes.push(glb::Node::named(part));
                    let i = doc.nodes.len() - 1;
                    match parent {
                        Some(p) => doc.nodes[p].children.push(i),
                        None => doc.roots.push(i),
                    }
                    path_nodes.insert(prefix.clone(), i);
                    i
                }
            };
            parent = Some(index);
        }
        parent.unwrap_or(0)
    };
    let mut tracks = Vec::new();
    let mut duration = 0f32;
    let mut infinite = 0usize;
    for track in &anim.tracks {
        if track.path.is_empty() {
            continue;
        }
        let node = node_for(&mut doc, &track.path);
        // `ImportCM.LoadAniClip` binds positions only for the root `Bip01`,
        // `_IK_` and `ManBip` paths.
        let marked = format!("{}*", track.path);
        let positions = marked.contains("Bip01*") || marked.contains("_IK_") || marked.contains("ManBip*");
        for (range, rotation) in [(0..4, true), (4..7, false)] {
            if !rotation && !positions {
                continue;
            }
            let curves: Vec<&Curve> = track.curves[range.clone()].iter().flatten().collect();
            if curves.is_empty() {
                continue;
            }
            ensure!(curves.len() == range.len(), "CM3D2 bone {} has a partial curve group", track.path);
            let shared = curves.iter().all(|c| {
                c.keys.len() == curves[0].keys.len()
                    && c.keys.iter().zip(&curves[0].keys).all(|(a, b)| a[0] == b[0])
            });
            let mut times: Vec<f32> = if shared {
                curves[0].keys.iter().map(|k| k[0]).collect()
            } else {
                let mut t: Vec<f32> = curves.iter().flat_map(|c| c.keys.iter().map(|k| k[0])).collect();
                t.sort_by(f32::total_cmp);
                t.dedup();
                t
            };
            times.retain(|t| t.is_finite());
            if times.is_empty() {
                continue;
            }
            ensure!(times.windows(2).all(|w| w[0] < w[1]) && times[0] >= 0., "CM3D2 curve times");
            duration = duration.max(*times.last().unwrap());
            let mut component = |c: &Curve, k: usize, t: f32| -> [f32; 3] {
                if shared {
                    let key = c.keys[k];
                    let mut fix = |x: f32| {
                        if x.is_finite() {
                            x
                        } else {
                            infinite += 1;
                            0.
                        }
                    };
                    [fix(key[2]), key[1], fix(key[3])]
                } else {
                    [0., evaluate(&c.keys, t), 0.]
                }
            };
            let mut values = Vec::with_capacity(times.len());
            let mut ins = Vec::with_capacity(times.len());
            let mut outs = Vec::with_capacity(times.len());
            for (k, &t) in times.iter().enumerate() {
                let mut v = [0.; 4];
                let mut a = [0.; 4];
                let mut b = [0.; 4];
                for (i, c) in curves.iter().enumerate() {
                    let [ti, x, to] = component(c, k, t);
                    a[i] = ti;
                    v[i] = x;
                    b[i] = to;
                }
                if rotation {
                    v = mirror_quat(v);
                    a = mirror_quat(a);
                    b = mirror_quat(b);
                } else {
                    for x in [&mut v, &mut a, &mut b] {
                        x[0] = -x[0];
                    }
                }
                values.push(v);
                ins.push(a);
                outs.push(b);
            }
            let values = if rotation {
                glb::Channel::Rotation(values)
            } else {
                glb::Channel::Translation(values.iter().map(|v| [v[0], v[1], v[2]]).collect())
            };
            tracks.push(glb::Track {
                node,
                times,
                values,
                tangents: shared.then_some((ins, outs)),
            });
        }
    }
    if infinite > 0 {
        doc.dropped.push(format!("{infinite} stepped (infinite) curve tangents sampled as flat"));
    }
    ensure!(!doc.nodes.is_empty(), "CM3D2 animation has no bone paths");
    let _ = duration;
    doc.animations.push(glb::Animation { name: clip.to_owned(), tracks });
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn per_texture_palettes_keep_independent_colors_and_source_alpha() -> Result<()> {
        use crysirror_image::ImageSurface;
        let original = ImageSurface { width: 1, height: 1, rgba: vec![137, 12, 85, 91] };
        let bytes = crysirror_image::encode_png(&original)?;
        let mut external = |_: &str| Ok(bytes.clone());
        let mut images = Images { external: &mut external, cache: HashMap::new(), decoded: HashMap::new() };
        let palette = |channel, opaque_source| {
            let colors = std::array::from_fn(|i| {
                let mut color = [0u8; 4];
                color[channel] = i as u8;
                color
            });
            TexturePalette { colors, opaque_source }
        };
        let colors = BTreeMap::from([
            ("_MainTex".into(), palette(0, false)), ("_ShadowTex".into(), palette(2, true)),
        ]);
        assert_eq!(images.decode_property("shared", "_ShadowTex", Some(&colors)).unwrap().rgba, [0,0,136,255]);
        assert_eq!(images.decode_property("shared", "_MainTex", Some(&colors)).unwrap().rgba, [136,0,0,91]);
        assert_eq!(images.decode_property("shared", "_HiTex", Some(&colors)).unwrap().rgba, original.rgba);
        Ok(())
    }

    fn string(out: &mut Vec<u8>, s: &str) {
        let mut n = s.len();
        loop {
            let byte = (n & 0x7f) as u8;
            n >>= 7;
            if n == 0 {
                out.push(byte);
                break;
            }
            out.push(byte | 0x80);
        }
        out.extend(s.as_bytes());
    }
    fn f(out: &mut Vec<u8>, v: &[f32]) {
        for x in v {
            out.extend(x.to_le_bytes());
        }
    }
    fn i(out: &mut Vec<u8>, v: i32) {
        out.extend(v.to_le_bytes());
    }

    /// Two bones, one triangle, a material, a morph.
    fn sample_mesh() -> Vec<u8> {
        let mut b = Vec::new();
        string(&mut b, "CM3D2_MESH");
        i(&mut b, 1000);
        string(&mut b, "tri");
        string(&mut b, "Bip01");
        i(&mut b, 2);
        string(&mut b, "Bip01");
        b.push(0);
        string(&mut b, "Bip01 Spine");
        b.push(1);
        i(&mut b, -1);
        i(&mut b, 0);
        f(&mut b, &[1., 0., 0., 0., 0., 0., 1.]);
        f(&mut b, &[0., 1., 0., 0., 0.70710677, 0., 0.70710677]);
        i(&mut b, 3); // vertices
        i(&mut b, 1); // sub-meshes
        i(&mut b, 1); // skin bones
        string(&mut b, "Bip01 Spine");
        let mut identity = [0f32; 16];
        identity[0] = 1.;
        identity[5] = 1.;
        identity[10] = 1.;
        identity[15] = 1.;
        identity[12] = 2.;
        f(&mut b, &identity);
        for (p, uv) in [([1., 0., 0.], [0., 0.]), ([0., 1., 0.], [1., 0.]), ([0., 0., 1.], [0., 1.])] {
            f(&mut b, &p);
            f(&mut b, &[0., 0., 1.]);
            f(&mut b, &uv);
        }
        i(&mut b, 0); // tangents
        for _ in 0..3 {
            b.extend([0, 0, 0, 0, 0, 0, 0, 0]);
            f(&mut b, &[1., 0., 0., 0.]);
        }
        i(&mut b, 3);
        for v in [0u16, 1, 2] {
            b.extend(v.to_le_bytes());
        }
        i(&mut b, 1);
        string(&mut b, "body");
        string(&mut b, "CM3D2/Toony_Lighted_Trans");
        string(&mut b, "CM3D2__Toony_Lighted_Trans");
        string(&mut b, "col");
        string(&mut b, "_Color");
        f(&mut b, &[1., 0.5, 0.25, 1.]);
        string(&mut b, "f");
        string(&mut b, "_Shininess");
        f(&mut b, &[0.]);
        string(&mut b, "end");
        string(&mut b, "morph");
        string(&mut b, "smile");
        i(&mut b, 1);
        b.extend(2u16.to_le_bytes());
        f(&mut b, &[0.5, 0., 0., 0., 0., 0.]);
        string(&mut b, "end");
        b
    }

    #[test]
    fn mesh_layout_and_basis() {
        let bytes = sample_mesh();
        let mesh = Mesh::parse(&bytes).unwrap();
        assert_eq!(mesh.bones[1].parent, Some(0));
        assert!(mesh.bones[1].scale_node);
        assert_eq!(mesh.materials[0].color("_Color"), Some([1., 0.5, 0.25, 1.]));
        assert_eq!(mesh.morphs[0].vertices, vec![(2, [0.5, 0., 0.], [0.; 3])]);
        let doc = convert_mesh(&bytes, &mut |_| bail!("no textures")).unwrap();
        // Bones mirrored, `_SCL_` child appended and used as the joint.
        assert_eq!(doc.nodes[0].translation, [-1., 0., 0.]);
        assert_eq!(doc.nodes[1].rotation, [0., -0.70710677, 0., 0.70710677]);
        assert_eq!(doc.nodes[2].name, "Bip01 Spine_SCL_");
        assert_eq!(doc.skins[0].joints, vec![2]);
        assert_eq!(doc.skins[0].inverse_bind[0][12], -2.);
        let prim = &doc.meshes[0].1[0];
        assert_eq!(prim.indices, vec![0, 2, 1]);
        assert_eq!(prim.positions[0], [-1., 0., 0.]);
        assert_eq!(prim.uvs[2], [0., 0.]);
        assert_eq!(prim.targets[0][2], [-0.5, 0., 0.]);
        assert_eq!(prim.target_names, vec!["smile".to_string()]);
        let engine = doc.materials[0].engine.as_ref().unwrap();
        assert_eq!(engine.alpha_mode, AlphaMode::Blend);
        // Toony_Lighted_Trans: the toon model with a baked 256×3 lookup
        // (default grey ramp, white shadow rate) and the material's colour.
        let toon = engine.toon.as_ref().unwrap();
        assert_eq!(engine.base_color_factor, [1., 0.5, 0.25, 1.]);
        assert!(!toon.color_table && toon.shadow_texture.is_none());
        let lookup = crysirror_image::decode_standard(&doc.images[toon.lookup_texture.unwrap()].0).unwrap();
        assert_eq!((lookup.width, lookup.height), (256, 3));
        assert_eq!(&lookup.rgba[0..4], &[128, 128, 128, 255]);
        assert_eq!(&lookup.rgba[256 * 4..256 * 4 + 4], &[255; 4]);
        let glb = doc.to_glb().unwrap();
        let model = crate::pbr::load_pbr(&glb, |_| bail!("external")).unwrap();
        assert_eq!(model.meshes[0][0].morphs[0].name, "smile");
        assert_eq!(model.materials[0].alpha_mode, AlphaMode::Blend);
        assert!(model.materials[0].toon.is_some());
    }

    #[test]
    fn textured_outline_materials_keep_surface_and_hull_ramps_separate() -> Result<()> {
        let pixel = |color: [u8; 4]| crysirror_image::encode_png(&crysirror_image::ImageSurface {
            width: 1, height: 1, rgba: color.to_vec(),
        });
        for shader in ["CM3D2/Toony_Lighted_Outline_Tex", "CM3D2/Toony_Lighted_Hair_Outline_Tex"] {
            let mut mesh = Mesh::parse(&sample_mesh())?;
            mesh.materials[0].shader = shader.to_owned();
            let mut edits = MaterialEdits::default();
            for (prop, name) in [("_MainTex","edge"),("_ToonRamp","surface"),("_OutlineTex","edge"),("_OutlineToonRamp","edge_ramp")] {
                edits.textures.push((0, prop.to_owned(), name.to_owned()));
            }
            let doc = convert_parsed(mesh, &mut |n| match n {
                "surface.tex" => pixel([110,120,130,255]),
                "edge.tex" => pixel([55,40,20,255]),
                "edge_ramp.tex" => pixel([220,210,200,255]),
                _ => bail!("unavailable texture {n}"),
            }, &edits)?;
            let model = crate::pbr::load_pbr(&doc.to_glb()?, |_| bail!("external"))?;
            let toon = model.materials[0].toon.as_ref().context("missing forward toon pass")?;
            let ramp = &model.images[toon.lookup_texture.context("lookup texture")?].levels[0];
            assert_eq!(&ramp[..4], &[110,120,130,255]);
            assert_eq!(&ramp[512*4..512*4+4], &[220,210,200,255]);
            let hull = &model.images[toon.outline_texture.context("outline texture")?];
            assert!(model.images[model.materials[0].base_color_texture.context("main texture")?].srgb);
            assert!(!hull.srgb);
            assert_eq!(&hull.levels[0], &[55,40,20,255]);
        }
        Ok(())
    }

    #[test]
    fn mirror_matrix_conjugates_by_the_x_reflection() {
        let m = glam::Mat4::from_rotation_translation(
            glam::Quat::from_rotation_y(0.4) * glam::Quat::from_rotation_x(0.2),
            glam::Vec3::new(1., 2., 3.),
        );
        let s = glam::Mat4::from_scale(glam::Vec3::new(-1., 1., 1.));
        let expected = s * m * s;
        let got = glam::Mat4::from_cols_array(&mirror_matrix(m.to_cols_array()));
        assert!(got.abs_diff_eq(expected, 1e-6));
        // The quaternion rule agrees with the matrix rule.
        let q = glam::Quat::from_rotation_y(0.4) * glam::Quat::from_rotation_x(0.2);
        let mq = glam::Quat::from_array(mirror_quat(q.to_array()));
        let rm = s * glam::Mat4::from_quat(q) * s;
        assert!(glam::Mat4::from_quat(mq).abs_diff_eq(rm, 1e-6));
    }

    #[test]
    fn animation_curves_become_cubic_tracks() {
        let mut b = Vec::new();
        string(&mut b, "CM3D2_ANIM");
        i(&mut b, 1000);
        b.push(1);
        string(&mut b, "Bip01");
        for code in 100..107u8 {
            b.push(code);
            i(&mut b, 2);
            f(&mut b, &[0., 0.1, 0., 1.]);
            f(&mut b, &[1., 0.2, 1., 0.]);
        }
        b.push(1);
        string(&mut b, "Bip01/Bip01 Spine");
        for code in 100..107u8 {
            b.push(code);
            i(&mut b, 1);
            f(&mut b, &[0., 0.5, 0., 0.]);
        }
        b.push(0);
        let doc = convert_animation(&b, "walk").unwrap();
        assert_eq!(doc.nodes.len(), 2);
        assert_eq!(doc.nodes[0].children, vec![1]);
        let tracks = &doc.animations[0].tracks;
        // Root: rotation + position; child: rotation only (position filter).
        assert_eq!(tracks.len(), 3);
        match &tracks[1].values {
            glb::Channel::Translation(v) => assert_eq!(v[0], [-0.1, 0.1, 0.1]),
            _ => panic!("expected translation"),
        }
        match &tracks[0].values {
            glb::Channel::Rotation(v) => assert_eq!(v[1], [0.2, -0.2, -0.2, 0.2]),
            _ => panic!("expected rotation"),
        }
        let glb = doc.to_glb().unwrap();
        let model = crate::pbr::load_pbr(&glb, |_| bail!("external")).unwrap();
        assert_eq!(model.animations[0].name, "walk");
        assert_eq!(model.animations[0].channels[0].interpolation, crate::pbr::Interpolation::CubicSpline);
        assert!((evaluate(&[[0., 0., 0., 0.], [1., 1., 0., 0.]], 0.5) - 0.5).abs() < 1e-6);
    }
}

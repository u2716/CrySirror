// SPDX-License-Identifier: MPL-2.0
//! Fixed readers for the Unity 4.x classes CrySirror consumes. Each reader
//! follows the 4.x serialization order of the class's fields; trailing fields
//! a consumer does not need are not read.
use crate::{PPtr, Reader};
use anyhow::{ensure, Context, Result};

#[derive(Clone, Debug, PartialEq)]
pub struct GameObject {
    /// `(class id, component)` in component order.
    pub components: Vec<(i32, PPtr)>,
    pub layer: u32,
    pub name: String,
    pub tag: u16,
    pub active: bool,
}
impl GameObject {
    pub fn read(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);
        let n = r.count(12)?;
        let mut components = Vec::with_capacity(n);
        for _ in 0..n {
            components.push((r.i32()?, r.pptr()?));
        }
        Ok(Self {
            components,
            layer: r.u32()?,
            name: r.string()?,
            tag: r.u16()?,
            active: r.bool()?,
        })
    }
    pub fn component(&self, class_id: i32) -> Option<PPtr> {
        self.components.iter().find(|(c, _)| *c == class_id).map(|(_, p)| *p)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Transform {
    pub game_object: PPtr,
    /// `[x, y, z, w]`, Unity (left-handed) space.
    pub rotation: [f32; 4],
    pub position: [f32; 3],
    pub scale: [f32; 3],
    pub children: Vec<PPtr>,
    pub parent: PPtr,
}
impl Transform {
    pub fn read(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);
        Ok(Self {
            game_object: r.pptr()?,
            rotation: r.f32s()?,
            position: r.f32s()?,
            scale: r.f32s()?,
            children: r.pptrs()?,
            parent: r.pptr()?,
        })
    }
}

/// The fixed part of a `MonoBehaviour`; script fields follow at `body`.
#[derive(Clone, Debug, PartialEq)]
pub struct MonoBehaviour {
    pub game_object: PPtr,
    pub enabled: bool,
    pub script: PPtr,
    pub name: String,
    /// Offset of the first script-defined field.
    pub body: usize,
}
impl MonoBehaviour {
    pub fn read(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);
        let game_object = r.pptr()?;
        let enabled = r.bool()?;
        r.align()?;
        let script = r.pptr()?;
        let name = r.string()?;
        Ok(Self { game_object, enabled, script, name, body: r.position() })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MonoScript {
    pub name: String,
    pub execution_order: i32,
    pub class_name: String,
    pub namespace: String,
    pub assembly: String,
}
impl MonoScript {
    pub fn read(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);
        let name = r.string()?;
        let execution_order = r.i32()?;
        let _properties_hash = r.u32()?;
        Ok(Self {
            name,
            execution_order,
            class_name: r.string()?,
            namespace: r.string()?,
            assembly: r.string()?,
        })
    }
    /// `Namespace.Class`, or the class alone.
    pub fn full_name(&self) -> String {
        if self.namespace.is_empty() {
            self.class_name.clone()
        } else {
            format!("{}.{}", self.namespace, self.class_name)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Texture2D {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub format: i32,
    pub mipmap: bool,
    pub image_count: i32,
    pub filter_mode: i32,
    pub wrap_mode: i32,
    /// Encoded image data (all mip levels, bottom-up rows).
    pub data: Vec<u8>,
}
impl Texture2D {
    pub fn read(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);
        let name = r.string()?;
        let width = r.i32()?;
        let height = r.i32()?;
        let _complete_size = r.i32()?;
        let format = r.i32()?;
        let mipmap = r.bool()?;
        let _readable = r.bool()?;
        let _read_allowed = r.bool()?;
        r.align()?;
        let image_count = r.i32()?;
        let _dimension = r.i32()?;
        let filter_mode = r.i32()?;
        let _aniso = r.i32()?;
        let _mip_bias = r.f32()?;
        let wrap_mode = r.i32()?;
        let _lightmap_format = r.i32()?;
        let _color_space = r.i32()?;
        let data = r.bytes()?.to_vec();
        ensure!((0..=16384).contains(&width) && (0..=16384).contains(&height), "Unity texture size");
        Ok(Self {
            name,
            width: width as u32,
            height: height as u32,
            format,
            mipmap,
            image_count,
            filter_mode,
            wrap_mode,
            data,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TexEnv {
    pub texture: PPtr,
    pub scale: [f32; 2],
    pub offset: [f32; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    pub name: String,
    pub shader: PPtr,
    pub keywords: Vec<String>,
    pub render_queue: i32,
    pub textures: Vec<(String, TexEnv)>,
    pub floats: Vec<(String, f32)>,
    pub colors: Vec<(String, [f32; 4])>,
}
impl Material {
    pub fn read(bytes: &[u8]) -> Result<Self> {
        Self::read_layout(bytes, false)
    }
    pub fn read_56(bytes: &[u8]) -> Result<Self> {
        Self::read_layout(bytes, true)
    }
    fn read_layout(bytes: &[u8], unity56: bool) -> Result<Self> {
        let mut r = Reader::new(bytes);
        let name = r.string()?;
        let pointer = |r: &mut Reader| if unity56 { r.pptr64() } else { r.pptr() };
        let shader = pointer(&mut r)?;
        let keywords = if unity56 {
            let keys = r.string()?.split_whitespace().map(str::to_owned).collect();
            r.take(8)?;
            keys
        } else { r.strings()? };
        let render_queue = r.i32()?;
        if unity56 {
            for _ in 0..r.count(8)? { r.string()?; r.string()?; }
            for _ in 0..r.count(4)? { r.string()?; }
        }
        let n = r.count(28)?;
        let mut textures = Vec::with_capacity(n);
        for _ in 0..n {
            let key = r.string()?;
            textures.push((key, TexEnv { texture: pointer(&mut r)?, scale: r.f32s()?, offset: r.f32s()? }));
        }
        let n = r.count(8)?;
        let mut floats = Vec::with_capacity(n);
        for _ in 0..n {
            floats.push((r.string()?, r.f32()?));
        }
        let n = r.count(20)?;
        let mut colors = Vec::with_capacity(n);
        for _ in 0..n {
            colors.push((r.string()?, r.f32s()?));
        }
        Ok(Self { name, shader, keywords, render_queue, textures, floats, colors })
    }
    pub fn texture(&self, name: &str) -> Option<&TexEnv> {
        self.textures.iter().find(|(n, _)| n == name).map(|(_, t)| t)
    }
    pub fn color(&self, name: &str) -> Option<[f32; 4]> {
        self.colors.iter().find(|(n, _)| n == name).map(|(_, c)| *c)
    }
    pub fn float(&self, name: &str) -> Option<f32> {
        self.floats.iter().find(|(n, _)| n == name).map(|(_, f)| *f)
    }
}

/// A Unity 4 `Font`: dynamic fonts carry the original TrueType/OpenType data.
#[derive(Clone, Debug, PartialEq)]
pub struct Font {
    pub name: String,
    pub line_spacing: f32,
    pub default_material: PPtr,
    pub texture: PPtr,
    pub font_data: Vec<u8>,
}
impl Font {
    pub fn read(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);
        let name = r.string()?;
        let _ascii_start = r.i32()?;
        let _kerning = r.f32()?;
        let line_spacing = r.f32()?;
        let _character_spacing = r.i32()?;
        let _character_padding = r.i32()?;
        let _convert_case = r.i32()?;
        let default_material = r.pptr()?;
        let n = r.count(44)?;
        for _ in 0..n {
            r.take(40)?;
            r.bool()?;
            r.align()?;
        }
        let texture = r.pptr()?;
        let n = r.count(8)?;
        r.take(n * 8)?;
        let _pixel_scale = r.f32()?;
        let font_data = r.bytes()?.to_vec();
        Ok(Self { name, line_spacing, default_material, texture, font_data })
    }
}

/// `ResourceManager.m_Container`: `Resources.Load` path → object.
pub fn resource_container(bytes: &[u8]) -> Result<Vec<(String, PPtr)>> {
    let mut r = Reader::new(bytes);
    let n = r.count(12)?;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push((r.string()?, r.pptr()?));
    }
    Ok(out)
}

/// `ResourceManager.m_Container` of serialized format ≥ 14 files (64-bit
/// path ids).
pub fn resource_container_64(bytes: &[u8]) -> Result<Vec<(String, PPtr)>> {
    let mut r = Reader::new(bytes);
    let n = r.count(16)?;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let name = r.string()?;
        out.push((name, PPtr { file_id: r.i32()?, path_id: r.i64()? }));
    }
    Ok(out)
}

/// `BuildSettings.levels`: scene paths in build order (`level<n>`).
pub fn build_levels(bytes: &[u8]) -> Result<Vec<String>> {
    Reader::new(bytes).strings()
}

/// The leading `RenderSettings` fields (fog and the flat ambient light,
/// display-referred as serialized).
#[derive(Clone, Debug, PartialEq)]
pub struct RenderSettings {
    pub fog: bool,
    pub fog_color: [f32; 4],
    pub fog_mode: i32,
    pub fog_density: f32,
    pub linear_fog_start: f32,
    pub linear_fog_end: f32,
    pub ambient_light: [f32; 4],
}
impl RenderSettings {
    pub fn read(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);
        let fog = r.bool()?;
        r.align()?;
        Ok(Self {
            fog,
            fog_color: r.f32s()?,
            fog_mode: r.i32()?,
            fog_density: r.f32()?,
            linear_fog_start: r.f32()?,
            linear_fog_end: r.f32()?,
            ambient_light: r.f32s()?,
        })
    }
}

/// The leading `Camera` fields.
#[derive(Clone, Debug, PartialEq)]
pub struct Camera {
    pub game_object: PPtr,
    pub enabled: bool,
    pub clear_flags: u32,
    pub background: [f32; 4],
    pub viewport: [f32; 4],
    pub near: f32,
    pub far: f32,
    pub fov: f32,
    pub orthographic: bool,
    pub orthographic_size: f32,
    pub depth: f32,
    pub culling_mask: u32,
}
impl Camera {
    pub fn read(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);
        let game_object = r.pptr()?;
        let enabled = r.bool()?;
        r.align()?;
        let clear_flags = r.u32()?;
        let background = r.f32s()?;
        let viewport = r.f32s()?;
        let near = r.f32()?;
        let far = r.f32()?;
        let fov = r.f32()?;
        let orthographic = r.bool()?;
        r.align()?;
        Ok(Self {
            game_object,
            enabled,
            clear_flags,
            background,
            viewport,
            near,
            far,
            fov,
            orthographic,
            orthographic_size: r.f32()?,
            depth: r.f32()?,
            culling_mask: r.u32()?,
        })
    }
}

/// The leading `Sprite` fields (4.6): texture rect in pixels, bottom-up.
#[derive(Clone, Debug, PartialEq)]
pub struct Sprite {
    pub name: String,
    pub rect: [f32; 4],
    pub offset: [f32; 2],
    pub border: [f32; 4],
    pub pixels_to_units: f32,
    pub texture: PPtr,
}
impl Sprite {
    pub fn read(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);
        let name = r.string()?;
        let rect = r.f32s()?;
        let offset = r.f32s()?;
        let border = r.f32s()?;
        let pixels_to_units = r.f32()?;
        let _extrude = r.u32()?;
        Ok(Self { name, rect, offset, border, pixels_to_units, texture: r.pptr()? })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubMesh {
    pub first_byte: u32,
    pub index_count: u32,
    pub topology: i32,
    pub first_vertex: u32,
    pub vertex_count: u32,
}

/// A Unity 4 `Mesh` with uncompressed vertex data (Unity space, Y up,
/// left-handed; UV origin bottom-left).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mesh {
    pub name: String,
    pub submeshes: Vec<SubMesh>,
    pub indices: Vec<u16>,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub colors: Vec<[u8; 4]>,
    pub uv0: Vec<[f32; 2]>,
    pub uv1: Vec<[f32; 2]>,
}

/// Unity 4 vertex channels in `m_Channels` order.
const CHANNEL_POSITION: usize = 0;
const CHANNEL_NORMAL: usize = 1;
const CHANNEL_COLOR: usize = 2;
const CHANNEL_UV0: usize = 3;
const CHANNEL_UV1: usize = 4;

impl Mesh {
    pub fn read(bytes: &[u8]) -> Result<Self> {
        Self::read_layout(bytes, false)
    }
    pub fn read_56(bytes: &[u8]) -> Result<Self> {
        Self::read_layout(bytes, true)
    }
    fn read_layout(bytes: &[u8], unity56: bool) -> Result<Self> {
        let mut r = Reader::new(bytes);
        let name = r.string()?;
        let n = r.count(44)?;
        let mut submeshes = Vec::with_capacity(n);
        for _ in 0..n {
            let s = SubMesh { first_byte: r.u32()?, index_count: r.u32()?, topology: r.i32()?, first_vertex: r.u32()?, vertex_count: r.u32()? };
            r.f32s::<6>()?;
            submeshes.push(s);
        }
        // m_Shapes (BlendShapeData).
        let n = r.count(40)?;
        r.take(n * 40)?;
        let n = r.count(12)?;
        r.take(n * 12)?;
        let n = r.count(16)?;
        for _ in 0..n {
            r.string()?;
            r.take(12)?;
        }
        let n = r.count(4)?;
        r.take(n * 4)?;
        // Bind poses, bone hashes, root bone hash.
        let n = r.count(64)?;
        r.take(n * 64)?;
        let n = r.count(4)?;
        r.take(n * 4)?;
        r.u32()?;
        let compression = r.u8()?;
        if !unity56 { r.u8()?; }
        r.take(3)?;
        r.align()?;
        ensure!(compression == 0, "compressed Unity mesh {name} is not supported");
        let index = r.bytes()?;
        let indices = index.chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect();
        let n = r.count(32)?;
        r.take(n * 32)?;
        // m_VertexData.
        let _current = r.u32()?;
        let vertex_count = r.u32()? as usize;
        let n = r.count(4)?;
        let mut channels = Vec::with_capacity(n);
        for _ in 0..n {
            let c = r.take(4)?;
            channels.push([c[0], c[1], c[2], c[3]]);
        }
        let mut streams = Vec::new();
        if unity56 {
            // Unity 5 stores the channel table. Streams are reconstructed from
            // its format, dimension and byte offsets; each starts 16-byte aligned.
            let mut strides = [0usize; 16];
            for &[stream, offset, format, dim] in &channels {
                if dim == 0 { continue; }
                let size = match format { 0=>4, 1=>2, 2|3=>1, _=>anyhow::bail!("unsupported Unity 5 vertex format {format}") };
                let stride = strides.get_mut(stream as usize).context("Unity vertex stream index")?;
                *stride=(*stride).max(offset as usize + size * dim as usize);
            }
            let mut base=0usize;
            for stride in strides {
                streams.push((base,stride));
                base=vertex_count.checked_mul(stride).and_then(|n| n.checked_add(base)).and_then(|n|n.checked_add(15)).context("Unity vertex stream overflow")? & !15;
            }
        } else {
            let n = r.count(12)?;
            for _ in 0..n {
                let _mask = r.u32()?;
                let offset = r.u32()? as usize;
                let stride = r.u8()? as usize;
                r.take(3)?;
                streams.push((offset, stride));
            }
        }
        let data = r.bytes()?;
        ensure!(vertex_count <= data.len(), "Unity vertex count exceeds buffer");
        let mut mesh = Mesh { name, submeshes, indices, ..Default::default() };
        let read = |channel: usize, out_dim: usize| -> Result<Vec<Vec<f32>>> {
            let Some(&[stream, offset, format, dim]) = channels.get(channel) else { return Ok(Vec::new()) };
            if dim == 0 {
                return Ok(Vec::new());
            }
            ensure!(dim as usize >= out_dim, "short Unity vertex attribute");
            let (base, stride) = *streams.get(stream as usize).context("Unity vertex stream")?;
            let size = match format {
                0 => 4,
                1 => 2,
                _ => 1,
            };
            let mut out = Vec::with_capacity(vertex_count);
            for v in 0..vertex_count {
                let at = base + v * stride + offset as usize;
                let mut item = Vec::with_capacity(out_dim);
                for k in 0..(dim as usize).min(out_dim) {
                    let p = at + k * size;
                    let raw = data.get(p..p + size).context("Unity vertex outside its data")?;
                    item.push(match format {
                        0 => f32::from_le_bytes(raw.try_into()?),
                        2 | 3 => f32::from(raw[0]),
                        _ => bail_half(raw)?,
                    });
                }
                out.push(item);
            }
            Ok(out)
        };
        mesh.positions = read(CHANNEL_POSITION, 3)?.into_iter().map(|v| [v[0], v[1], v[2]]).collect();
        mesh.normals = read(CHANNEL_NORMAL, 3)?.into_iter().map(|v| [v[0], v[1], v[2]]).collect();
        mesh.uv0 = read(CHANNEL_UV0, 2)?.into_iter().map(|v| [v[0], v[1]]).collect();
        mesh.uv1 = read(CHANNEL_UV1, 2)?.into_iter().map(|v| [v[0], v[1]]).collect();
        // Colour is one packed RGBA8 element (format 2, dimension 1).
        if let Some(&[stream, offset, 2, 1]) = channels.get(CHANNEL_COLOR) {
            let (base, stride) = *streams.get(stream as usize).context("Unity vertex stream")?;
            for v in 0..vertex_count {
                let at = base + v * stride + offset as usize;
                let c = data.get(at..at + 4).context("Unity vertex colour outside its data")?;
                mesh.colors.push([c[0], c[1], c[2], c[3]]);
            }
        }
        ensure!(mesh.positions.len() == vertex_count, "Unity mesh without positions");
        Ok(mesh)
    }
}

fn bail_half(raw: &[u8]) -> Result<f32> {
    let h = u16::from_le_bytes(raw.try_into()?);
    let sign = if h & 0x8000 != 0 { -1. } else { 1. };
    let exp = (h >> 10) & 0x1f;
    let frac = f32::from(h & 0x3ff);
    Ok(sign * match exp {
        0 => frac * 2f32.powi(-24),
        31 => f32::INFINITY,
        e => (1. + frac / 1024.) * 2f32.powi(i32::from(e) - 15),
    })
}

/// `MeshRenderer`/`SkinnedMeshRenderer` leading fields.
#[derive(Clone, Debug, PartialEq)]
pub struct Renderer {
    pub game_object: PPtr,
    pub enabled: bool,
    pub lightmap_index: u8,
    pub lightmap_tiling_offset: [f32; 4],
    pub materials: Vec<PPtr>,
}
impl Renderer {
    pub fn read(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);
        let game_object = r.pptr()?;
        let enabled = r.bool()?;
        let _cast = r.bool()?;
        let _receive = r.bool()?;
        let lightmap_index = r.u8()?;
        Ok(Self { game_object, enabled, lightmap_index, lightmap_tiling_offset: r.f32s()?, materials: r.pptrs()? })
    }
}

/// `MeshFilter.m_Mesh`.
pub fn mesh_filter(bytes: &[u8]) -> Result<PPtr> {
    let mut r = Reader::new(bytes);
    r.pptr()?;
    r.pptr()
}

/// The `Shader "Name"` of a 4.x shader asset's source.
pub fn shader_name(bytes: &[u8]) -> Result<String> {
    let mut r = Reader::new(bytes);
    let _name = r.string()?;
    let script = r.string()?;
    let start = script.find("Shader \"").map(|i| i + 8).context("shader without a name")?;
    let end = script[start..].find('"').context("unterminated shader name")?;
    Ok(script[start..start + end].to_owned())
}

/// `TextAsset.m_Script` bytes.
pub fn text_asset(bytes: &[u8]) -> Result<(String, Vec<u8>)> {
    let mut r = Reader::new(bytes);
    let name = r.string()?;
    Ok((name, r.bytes()?.to_vec()))
}

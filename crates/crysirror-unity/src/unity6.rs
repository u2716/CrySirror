// SPDX-License-Identifier: MPL-2.0
//! Engine data from type-tree objects of Unity 2020+ files (`managed::Fields`
//! from `SerializedFile::decode`).
use crate::managed::{Fields, Value};
use anyhow::{bail, ensure, Context, Result};
use crysirror_core::service::AssetReader;
use crysirror_image::{block_compressed::Block, ImageSurface};
use std::sync::Arc;

/// Resolves a `StreamingInfo.path` (`archive:/CAB-…/CAB-….resS`) to a reader.
pub type StreamResolver<'a> = &'a dyn Fn(&str) -> Option<Arc<dyn AssetReader>>;

/// The `.resS` node name of a `StreamingInfo.path`.
pub fn stream_node(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn bytes_of(f: &Fields, name: &str) -> Option<Vec<u8>> {
    match f.get(name)? {
        Value::Bytes(b) => Some(b.clone()),
        _ => None,
    }
}

/// `m_StreamData` bytes, or the inline field.
pub fn stream_or_inline(f: &Fields, inline: &str, resolve: StreamResolver) -> Result<Vec<u8>> {
    if let Some(Value::Struct(s)) = f.get("m_StreamData") {
        let size = s.int("size").unwrap_or(0);
        if size > 0 {
            let path = s.string("path").unwrap_or("");
            let offset = s.int("offset").unwrap_or(0);
            ensure!(offset >= 0 && size <= 1 << 30, "streaming info {path} {offset}+{size}");
            let reader = resolve(path).with_context(|| format!("stream {path} not available"))?;
            let mut out = vec![0u8; size as usize];
            let mut done = 0;
            while done < out.len() {
                let n = reader.read_at(offset as u64 + done as u64, &mut out[done..]).map_err(|e| anyhow::anyhow!("stream read: {e}"))?;
                ensure!(n > 0, "truncated stream {path}");
                done += n;
            }
            return Ok(out);
        }
    }
    bytes_of(f, inline).with_context(|| format!("no {inline} data"))
}

/// `Texture2D` level 0 as top-down RGBA8.
pub fn texture_2d(f: &Fields, resolve: StreamResolver) -> Result<ImageSurface> {
    let (w, h) = (f.int("m_Width").unwrap_or(0), f.int("m_Height").unwrap_or(0));
    ensure!(w > 0 && h > 0 && w <= 16384 && h <= 16384, "Texture2D size {w}x{h}");
    let (w, h) = (w as u32, h as u32);
    let format = f.int("m_TextureFormat").unwrap_or(-1);
    let data = stream_or_inline(f, "image data", resolve)?;
    let block = |b: Block| -> Result<ImageSurface> { crysirror_image::block_compressed::decode(b, w, h, &data) };
    let mut surface = match format {
        // Alpha8, ARGB4444, RGB24, RGBA32, ARGB32, DXT1, DXT5: the Unity 4
        // decoders.
        1..=5 | 10 | 12 => {
            let t = crate::classes::Texture2D {
                name: f.string("m_Name").unwrap_or("").to_owned(),
                width: w,
                height: h,
                format: format as i32,
                mipmap: false,
                image_count: 1,
                filter_mode: 0,
                wrap_mode: 0,
                data,
            };
            // The Unity 4 decoder flips rows itself.
            return crate::texture::decode(&t);
        }
        24 => block(Block::Bc6hUnsigned)?,
        25 => block(Block::Bc7)?,
        26 => block(Block::Bc4)?,
        27 => block(Block::Bc5)?,
        other => bail!("unsupported Unity texture format {other}"),
    };
    // Unity rows are bottom-up.
    let row = w as usize * 4;
    let mut flipped = Vec::with_capacity(surface.rgba.len());
    for y in (0..h as usize).rev() {
        flipped.extend_from_slice(&surface.rgba[y * row..(y + 1) * row]);
    }
    surface.rgba = flipped;
    Ok(surface)
}

/// The slices of a Unity `Texture2DArray` at mip level 0 (top-down RGBA8,
/// bytes as stored) and whether the format is sRGB. `m_Format` is a
/// `GraphicsFormat`; each slice holds its full mip chain in turn.
pub fn texture_2d_array(f: &Fields, resolve: StreamResolver) -> Result<(Vec<ImageSurface>, bool)> {
    let (w, h, depth) = (f.int("m_Width").unwrap_or(0), f.int("m_Height").unwrap_or(0), f.int("m_Depth").unwrap_or(0));
    ensure!(w > 0 && h > 0 && w <= 16384 && h <= 16384 && (1..=256).contains(&depth), "Texture2DArray size {w}x{h}x{depth}");
    let (w, h) = (w as u32, h as u32);
    let format = f.int("m_Format").unwrap_or(-1);
    let mips = f.int("m_MipCount").unwrap_or(1).clamp(1, 16) as u32;
    // GraphicsFormat: block kind (None: RGBA8) and sRGB.
    let (block, srgb) = match format {
        3 => (None, true),
        7 => (None, false),
        4 => (None, true),
        8 => (None, false),
        96 | 97 => (Some(Block::Bc1), format == 96),
        100 | 101 => (Some(Block::Bc3), format == 100),
        102 => (Some(Block::Bc4), false),
        104 => (Some(Block::Bc5), false),
        108 | 109 => (Some(Block::Bc7), format == 108),
        other => bail!("unsupported Texture2DArray format {other}"),
    };
    let channels = if matches!(format, 3 | 7) { 3 } else { 4 };
    let level = |l: u32| -> usize {
        let (lw, lh) = ((w >> l).max(1) as usize, (h >> l).max(1) as usize);
        match block {
            Some(b) => lw.div_ceil(4) * lh.div_ceil(4) * b.block_bytes(),
            None => lw * lh * channels,
        }
    };
    let slice: usize = (0..mips).map(level).sum();
    let data = stream_or_inline(f, "image data", resolve)?;
    ensure!(data.len() >= slice * depth as usize, "Texture2DArray data {} < {}", data.len(), slice * depth as usize);
    let mut out = Vec::new();
    for s in 0..depth as usize {
        let bytes = &data[s * slice..s * slice + level(0)];
        let mut surface = match block {
            Some(b) => crysirror_image::block_compressed::decode(b, w, h, bytes)?,
            None => ImageSurface { width: w, height: h, rgba: bytes.chunks_exact(channels).flat_map(|p| [p[0], p[1], p[2], if channels == 4 { p[3] } else { 255 }]).collect() },
        };
        // Unity rows are bottom-up.
        let row = w as usize * 4;
        let mut flipped = Vec::with_capacity(surface.rgba.len());
        for y in (0..h as usize).rev() {
            flipped.extend_from_slice(&surface.rgba[y * row..(y + 1) * row]);
        }
        surface.rgba = flipped;
        out.push(surface);
    }
    Ok((out, srgb))
}

/// `SubMesh` of a Unity 2020+ mesh (index range in indices, not bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct SubMesh6 {
    pub first_index: u32,
    pub index_count: u32,
    pub topology: i32,
    pub base_vertex: u32,
}

/// One blend shape channel: name, frames of (weight, per-vertex deltas).
#[derive(Clone, Debug, PartialEq)]
pub struct BlendShape {
    pub name: String,
    pub frames: Vec<(f32, Vec<(u32, [f32; 3], [f32; 3])>)>,
}

/// A decoded Unity 2020+ `Mesh`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mesh6 {
    pub name: String,
    pub submeshes: Vec<SubMesh6>,
    pub indices: Vec<u32>,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub tangents: Vec<[f32; 4]>,
    pub colors: Vec<[f32; 4]>,
    pub uv: [Vec<[f32; 2]>; 4],
    pub weights: Vec<[f32; 4]>,
    pub joints: Vec<[u16; 4]>,
    pub bind_poses: Vec<[f32; 16]>,
    pub bone_name_hashes: Vec<u32>,
    pub root_bone_name_hash: u32,
    pub blend_shapes: Vec<BlendShape>,
}

/// `VertexFormat` byte sizes (Unity 2019+).
fn format_size(f: i64) -> Result<usize> {
    Ok(match f {
        0 | 10 | 11 => 4,
        1 | 4 | 5 | 8 | 9 => 2,
        2 | 3 | 6 | 7 => 1,
        other => bail!("vertex format {other}"),
    })
}

fn component(b: &[u8], f: i64) -> f32 {
    match f {
        0 => f32::from_le_bytes(b[..4].try_into().unwrap()),
        1 => f16_to_f32(u16::from_le_bytes([b[0], b[1]])),
        2 => f32::from(b[0]) / 255.,
        3 => (f32::from(b[0] as i8) / 127.).max(-1.),
        4 => f32::from(u16::from_le_bytes([b[0], b[1]])) / 65535.,
        5 => (f32::from(i16::from_le_bytes([b[0], b[1]])) / 32767.).max(-1.),
        6 => f32::from(b[0]),
        7 => f32::from(b[0] as i8),
        8 => f32::from(u16::from_le_bytes([b[0], b[1]])),
        9 => f32::from(i16::from_le_bytes([b[0], b[1]])),
        10 => u32::from_le_bytes(b[..4].try_into().unwrap()) as f32,
        _ => i32::from_le_bytes(b[..4].try_into().unwrap()) as f32,
    }
}

fn f16_to_f32(h: u16) -> f32 {
    let sign = if h & 0x8000 != 0 { -1. } else { 1. };
    let exp = (h >> 10) & 0x1f;
    let man = f32::from(h & 0x3ff);
    sign * match exp {
        0 => man * 2f32.powi(-24),
        31 => if man == 0. { f32::INFINITY } else { f32::NAN },
        e => (1. + man / 1024.) * 2f32.powi(i32::from(e) - 15),
    }
}

fn floats(v: Option<&Value>) -> Vec<f32> {
    match v {
        Some(Value::Floats(f)) => f.clone(),
        Some(Value::Array(a)) => a.iter().filter_map(|x| if let Value::Float(f) = x { Some(*f as f32) } else { None }).collect(),
        _ => Vec::new(),
    }
}

fn vec3(v: Option<&Value>) -> [f32; 3] {
    match v {
        Some(Value::Struct(s)) => [s.float("x").unwrap_or(0.) as f32, s.float("y").unwrap_or(0.) as f32, s.float("z").unwrap_or(0.) as f32],
        _ => [0.; 3],
    }
}

fn array(v: Option<&Value>) -> &[Value] {
    match v {
        Some(Value::Array(a)) => a,
        _ => &[],
    }
}

/// Decode a `Mesh` (uncompressed vertex data; `m_MeshCompression` meshes
/// fail visibly).
pub fn mesh(f: &Fields, resolve: StreamResolver) -> Result<Mesh6> {
    let mut m = Mesh6 { name: f.string("m_Name").unwrap_or("").to_owned(), ..Default::default() };
    ensure!(f.int("m_MeshCompression").unwrap_or(0) == 0, "compressed mesh {}", m.name);
    let vd = match f.get("m_VertexData") {
        Some(Value::Struct(s)) => s,
        _ => bail!("mesh {} without vertex data", m.name),
    };
    let count = vd.int("m_VertexCount").unwrap_or(0).max(0) as usize;
    let data = match f.get("m_StreamData") {
        Some(Value::Struct(s)) if s.int("size").unwrap_or(0) > 0 => stream_or_inline(f, "", resolve)?,
        _ => bytes_of(vd, "m_DataSize").unwrap_or_default(),
    };
    // Channels: (stream, offset, format, dimension & 0xf).
    let channels: Vec<(i64, usize, i64, usize)> = array(vd.get("m_Channels"))
        .iter()
        .map(|c| match c {
            Value::Struct(s) => (s.int("stream").unwrap_or(0), s.int("offset").unwrap_or(0) as usize, s.int("format").unwrap_or(0), (s.int("dimension").unwrap_or(0) & 0xf) as usize),
            _ => (0, 0, 0, 0),
        })
        .collect();
    let streams = channels.iter().map(|c| c.0).max().map_or(0, |s| s + 1);
    let mut stream_offset = vec![0usize; streams as usize];
    let mut stride = vec![0usize; streams as usize];
    for s in 0..streams {
        for c in channels.iter().filter(|c| c.0 == s && c.3 > 0) {
            stride[s as usize] = stride[s as usize].max(c.1 + format_size(c.2)? * c.3);
        }
    }
    let mut at = 0usize;
    for s in 0..streams as usize {
        stream_offset[s] = at;
        at += stride[s] * count;
        at = at.div_ceil(16) * 16;
    }
    ensure!(at.saturating_sub(16) <= data.len(), "mesh {} vertex data {} < {at}", m.name, data.len());
    let read = |ch: usize| -> Result<Vec<Vec<f32>>> {
        let Some(&(s, off, fmt, dim)) = channels.get(ch).filter(|c| c.3 > 0) else { return Ok(Vec::new()) };
        let size = format_size(fmt)?;
        let mut out = Vec::with_capacity(count);
        for v in 0..count {
            let base = stream_offset[s as usize] + v * stride[s as usize] + off;
            let b = data.get(base..base + size * dim).with_context(|| format!("mesh vertex {v} channel {ch}"))?;
            out.push((0..dim).map(|d| component(&b[d * size..], fmt)).collect());
        }
        Ok(out)
    };
    let take3 = |v: Vec<Vec<f32>>| v.into_iter().map(|c| [c.first().copied().unwrap_or(0.), c.get(1).copied().unwrap_or(0.), c.get(2).copied().unwrap_or(0.)]).collect::<Vec<_>>();
    let take4 = |v: Vec<Vec<f32>>, w: f32| v.into_iter().map(|c| [c.first().copied().unwrap_or(0.), c.get(1).copied().unwrap_or(0.), c.get(2).copied().unwrap_or(0.), c.get(3).copied().unwrap_or(w)]).collect::<Vec<_>>();
    m.positions = take3(read(0)?);
    m.normals = take3(read(1)?);
    m.tangents = take4(read(2)?, 1.);
    m.colors = take4(read(3)?, 1.);
    for i in 0..4 {
        m.uv[i] = read(4 + i)?.into_iter().map(|c| [c.first().copied().unwrap_or(0.), c.get(1).copied().unwrap_or(0.)]).collect();
    }
    m.weights = take4(read(12)?, 0.);
    m.joints = read(13)?.into_iter().map(|c| [0, 1, 2, 3].map(|i| c.get(i).copied().unwrap_or(0.) as u16)).collect();
    // Unity stores only the weights of the used influences: the remainder
    // belongs to the first bone when fewer than four weights are stored.
    if channels.get(12).is_some_and(|c| c.3 > 0 && c.3 < 4) {
        for w in &mut m.weights {
            let used: f32 = w.iter().sum();
            if used < 1. {
                w[channels[12].3.min(3)] = 1. - used;
            }
        }
    }
    let index_bytes = bytes_of(f, "m_IndexBuffer").unwrap_or_default();
    m.indices = if f.int("m_IndexFormat").unwrap_or(0) == 1 {
        index_bytes.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect()
    } else {
        index_bytes.chunks_exact(2).map(|c| u32::from(u16::from_le_bytes([c[0], c[1]]))).collect()
    };
    let index_size = if f.int("m_IndexFormat").unwrap_or(0) == 1 { 4 } else { 2 };
    for s in array(f.get("m_SubMeshes")) {
        if let Value::Struct(s) = s {
            m.submeshes.push(SubMesh6 {
                first_index: (s.int("firstByte").unwrap_or(0) / index_size) as u32,
                index_count: s.int("indexCount").unwrap_or(0) as u32,
                topology: s.int("topology").unwrap_or(0) as i32,
                base_vertex: s.int("baseVertex").unwrap_or(0) as u32,
            });
        }
    }
    for p in array(f.get("m_BindPose")) {
        if let Value::Struct(s) = p {
            let mut mat = [0f32; 16];
            for r in 0..4 {
                for c in 0..4 {
                    mat[c * 4 + r] = s.float(&format!("e{r}{c}")).unwrap_or(0.) as f32;
                }
            }
            m.bind_poses.push(mat);
        }
    }
    m.bone_name_hashes = array(f.get("m_BoneNameHashes")).iter().filter_map(|v| if let Value::Int(i) = v { Some(*i as u32) } else { None }).collect();
    m.root_bone_name_hash = f.int("m_RootBoneNameHash").unwrap_or(0) as u32;
    if let Some(Value::Struct(shapes)) = f.get("m_Shapes") {
        let verts: Vec<(u32, [f32; 3], [f32; 3])> = array(shapes.get("vertices"))
            .iter()
            .filter_map(|v| if let Value::Struct(s) = v { Some((s.int("index").unwrap_or(0) as u32, vec3(s.get("vertex")), vec3(s.get("normal")))) } else { None })
            .collect();
        let frames: Vec<(usize, usize)> = array(shapes.get("shapes"))
            .iter()
            .filter_map(|v| if let Value::Struct(s) = v { Some((s.int("firstVertex").unwrap_or(0) as usize, s.int("vertexCount").unwrap_or(0) as usize)) } else { None })
            .collect();
        let weights = floats(shapes.get("fullWeights"));
        for ch in array(shapes.get("channels")) {
            let Value::Struct(c) = ch else { continue };
            let (frame, n) = (c.int("frameIndex").unwrap_or(0) as usize, c.int("frameCount").unwrap_or(0) as usize);
            let mut shape = BlendShape { name: c.string("name").unwrap_or("").to_owned(), frames: Vec::new() };
            for i in frame..frame + n {
                let (first, cnt) = *frames.get(i).context("blend shape frame")?;
                let deltas = verts.get(first..first + cnt).context("blend shape vertices")?.to_vec();
                shape.frames.push((weights.get(i).copied().unwrap_or(100.), deltas));
            }
            m.blend_shapes.push(shape);
        }
    }
    Ok(m)
}

/// A sprite's texture and texture rectangle (x, y, w, h in texture pixels,
/// bottom-up): `m_RD`, or for sprites packed in a `SpriteAtlas` the
/// atlas's `m_RenderDataMap` entry for the sprite's `m_RenderDataKey`.
/// `atlas` decodes the sprite's `m_SpriteAtlas` (same file reference).
/// The texture pointer is relative to the file that holds the render data
/// (`true` when it is the atlas's file).
pub fn sprite_render(sprite: &Fields, atlas: &dyn Fn(crate::PPtr) -> Option<Fields>) -> Option<(crate::PPtr, [f32; 4], bool)> {
    let rect = |s: &Fields| -> Option<[f32; 4]> {
        match s.get("textureRect") {
            Some(Value::Struct(r)) => Some(["x", "y", "width", "height"].map(|k| r.float(k).unwrap_or(0.) as f32)),
            _ => None,
        }
    };
    if let Some(Value::Struct(rd)) = sprite.get("m_RD") {
        if let Some(Value::PPtr(t)) = rd.get("texture") {
            if !t.is_null() {
                return Some((*t, rect(rd)?, false));
            }
        }
    }
    let Some(Value::PPtr(a)) = sprite.get("m_SpriteAtlas") else { return None };
    if a.is_null() {
        return None;
    }
    let atlas = atlas(*a)?;
    let key = match sprite.get("m_RenderDataKey") {
        Some(Value::Struct(k)) => k.clone(),
        _ => return None,
    };
    let Some(Value::Array(map)) = atlas.get("m_RenderDataMap") else { return None };
    for e in map {
        let Value::Struct(pair) = e else { continue };
        if pair.get("first") == Some(&Value::Struct(key.clone())) {
            if let Some(Value::Struct(d)) = pair.get("second") {
                if let Some(Value::PPtr(t)) = d.get("texture") {
                    return Some((*t, rect(d)?, true));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texture_array_slices() {
        // 2×1 RGB8 (GraphicsFormat 7), two slices with a 1×1 mip each.
        let data = vec![1u8, 2, 3, 4, 5, 6, 9, 9, 9, 10, 11, 12, 13, 14, 15, 9, 9, 9];
        let f = Fields(vec![
            ("m_Width".into(), Value::Int(2)),
            ("m_Height".into(), Value::Int(1)),
            ("m_Depth".into(), Value::Int(2)),
            ("m_Format".into(), Value::Int(7)),
            ("m_MipCount".into(), Value::Int(2)),
            ("image data".into(), Value::Bytes(data)),
        ]);
        let (slices, srgb) = texture_2d_array(&f, &|_| None).unwrap();
        assert!(!srgb);
        assert_eq!(slices.len(), 2);
        assert_eq!(slices[1].rgba, vec![10, 11, 12, 255, 13, 14, 15, 255]);
    }
}

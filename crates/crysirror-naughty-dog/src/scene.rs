// SPDX-License-Identifier: MPL-2.0
//! A package's prototype meshes as one glTF document for the shared world
//! renderer: one glTF mesh per prototype (a primitive per fragment with its
//! group's base-colour texture), placed once per native instance. Native
//! mesh units are centimetres; instance matrices are in metres.
use crate::{
    mesh::{self, Mesh},
    package::Directory,
    texture::VramDesc,
};
use anyhow::Result;
use crysirror_asset::interchange::glb::{Document, Material, Node, Primitive};
use crysirror_core::service::AssetReader;
use std::collections::HashMap;

/// A level start camera from `CAMERA_TABLE_1`, in metres.
#[derive(Clone, Debug, PartialEq)]
pub struct StartCamera {
    pub name: String,
    pub position: [f32; 3],
    pub forward: [f32; 3],
    pub up: [f32; 3],
    /// Horizontal and vertical field of view in radians.
    pub fov: [f32; 2],
    pub near: f32,
    pub far: f32,
}

/// The level's directional light and ambient term from `LIGHT_TABLE_1`.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelLight {
    pub color: [f32; 3],
    /// Unit direction the light travels.
    pub direction: [f32; 3],
    pub intensity: f32,
    /// First RGB spherical-harmonic term of the table's probe record, when
    /// non-zero.
    pub ambient: Option<[f32; 3]>,
}

/// `LIGHT_TABLE_1`: the light record pointer at +0x28 holds colour,
/// position, direction, position, zero, range (`f32::MAX` for directional)
/// and intensity; the probe record at +0x30 holds a position followed by
/// RGB spherical-harmonic coefficients.
pub fn level_light(p: &mesh::Loaded, payload: usize) -> Result<Option<LevelLight>> {
    // Some installed tables (for example `museum-sewer-end`) carry no light
    // record.
    let Ok(record) = p.ptr(payload + 0x28) else { return Ok(None) };
    let f = |i: usize| p.f32(record + 4 * i);
    let color = [f(0)?, f(1)?, f(2)?];
    let direction = [f(6)?, f(7)?, f(8)?];
    let (range, intensity) = (f(13)?, f(14)?);
    let length = direction.iter().map(|v| v * v).sum::<f32>().sqrt();
    if range < 1e30 || !(0.9..1.1).contains(&length) || !color.iter().chain([&intensity]).all(|v| v.is_finite() && *v >= 0.) {
        return Ok(None);
    }
    let ambient = match p.ptr(payload + 0x30) {
        Ok(probe) => {
            let sh = [p.f32(probe + 12)?, p.f32(probe + 16)?, p.f32(probe + 20)?];
            (sh.iter().all(|v| v.is_finite() && *v >= 0.) && sh.iter().any(|v| *v > 0.)).then_some(sh)
        }
        Err(_) => None,
    };
    Ok(Some(LevelLight { color, direction: direction.map(|v| v / length), intensity, ambient }))
}

/// The first camera of a `CAMERA_TABLE_1` payload: count, pointer to
/// records of a name pointer followed by position, forward, up, two field
/// of view angles, near and far.
pub fn start_camera(p: &mesh::Loaded, payload: usize) -> Result<Option<StartCamera>> {
    if p.u32(payload)? == 0 {
        return Ok(None);
    }
    let record = p.ptr(payload + 4)?;
    let name_at = p.ptr(record)?;
    let end = p.data[name_at..].iter().take(256).position(|b| *b == 0).unwrap_or(0);
    let f = |i: usize| p.f32(record + 4 + 4 * i);
    let c = StartCamera {
        name: String::from_utf8_lossy(&p.data[name_at..name_at + end]).into_owned(),
        position: [f(0)?, f(1)?, f(2)?],
        forward: [f(3)?, f(4)?, f(5)?],
        up: [f(6)?, f(7)?, f(8)?],
        fov: [f(9)?, f(10)?],
        near: f(11)?,
        far: f(12)?,
    };
    anyhow::ensure!(c.position.iter().chain(&c.forward).chain(&c.fov).all(|v| v.is_finite()), "ND camera values");
    Ok(Some(c))
}

pub const METRES_PER_UNIT: f32 = 0.01;

/// Every render mesh of a package (collision collections excluded).
pub fn package_meshes(source: &dyn AssetReader, directory: &Directory, limit: u64) -> Result<Vec<Mesh>> {
    let collections: Vec<_> = directory.items.iter().filter(|i| i.kind == "PM_COLLECTION_2").collect();
    if collections.is_empty() {
        return Ok(Vec::new());
    }
    let loaded = mesh::Loaded::read(source, limit)?;
    let mut meshes = Vec::new();
    for item in collections {
        meshes.extend(mesh::prototype_meshes(&loaded, item.payload_offset as usize)?);
    }
    Ok(meshes)
}

/// Axis-aligned bounds of all positions, in native units.
pub fn bounds(meshes: &[Mesh]) -> Option<([f32; 3], [f32; 3])> {
    let mut points = meshes.iter().flat_map(|m| &m.fragments).flat_map(|f| &f.positions);
    let first = *points.next()?;
    Some(points.fold((first, first), |(lo, hi), p| (std::array::from_fn(|k| lo[k].min(p[k])), std::array::from_fn(|k| hi[k].max(p[k])))))
}

/// An instance's world matrix. Identity-matrix instances (the large
/// background chunks) are placed at their culling sphere's centre: with
/// that rule every ray of the `menu-temple` start camera hits nearby
/// geometry (576 of 576, median 15.2 m), and with the identity none does.
/// Inferred from data (SATD-nd-retail-runtime).
pub fn placement(mesh: &Mesh, instance: usize) -> [f32; 16] {
    let mut m = mesh.instances[instance];
    if m[12] == 0. && m[13] == 0. && m[14] == 0. {
        if let Some(s) = mesh.spheres.get(instance) {
            m[12..15].copy_from_slice(&s[..3]);
        }
    }
    m
}

/// World-space bounds in metres of every placed instance's mesh bounds.
pub fn placed_bounds(meshes: &[Mesh]) -> Option<([f32; 3], [f32; 3])> {
    let mut out: Option<([f32; 3], [f32; 3])> = None;
    for mesh in meshes {
        let Some((lo, hi)) = bounds(std::slice::from_ref(mesh)) else { continue };
        for i in 0..mesh.instances.len() {
            let m = placement(mesh, i);
            for corner in 0..8 {
                let p: [f32; 3] = std::array::from_fn(|k| (if corner >> k & 1 == 0 { lo[k] } else { hi[k] }) * METRES_PER_UNIT);
                let w: [f32; 3] = std::array::from_fn(|c| p[0] * m[c] + p[1] * m[4 + c] + p[2] * m[8 + c] + m[12 + c]);
                out = Some(match out {
                    None => (w, w),
                    Some((a, b)) => (std::array::from_fn(|k| a[k].min(w[k])), std::array::from_fn(|k| b[k].max(w[k]))),
                });
            }
        }
    }
    out
}

/// Translation, rotation (xyzw) and scale of a row-major matrix.
fn trs(m: &[f32; 16]) -> ([f32; 3], [f32; 4], [f32; 3]) {
    // Row-major with row vectors equals glam's column-major layout.
    let (s, r, t) = glam::Mat4::from_cols_array(m).to_scale_rotation_translation();
    (t.to_array(), r.to_array(), s.to_array())
}

/// Build the document. Textures decode from the package's VRAM data;
/// `max_texture` halves oversized base-colour images by box filtering.
pub fn document(name: &str, meshes: &[Mesh], source: &dyn AssetReader, directory: &Directory, max_texture: u32) -> Result<Document> {
    let mut doc = Document { generator: "CrySirror Naughty Dog background importer".into(), ..Default::default() };
    let mut textures = HashMap::new();
    for item in directory.items.iter().filter(|i| i.kind == "VRAM_DESC") {
        let desc = VramDesc::read(source, item.payload_offset)?;
        textures.insert(desc.id, desc);
    }
    let mut images: HashMap<u32, usize> = HashMap::new();
    for (index, mesh) in meshes.iter().enumerate() {
        let mut primitives = Vec::new();
        for group in &mesh.groups {
            let image = match group.material.base_color().and_then(|id| textures.get(&id)) {
                None => None,
                Some(desc) => match images.get(&desc.id) {
                    Some(&i) => Some(i),
                    None => {
                        let mut surface = desc.decode(source, directory.texture_base)?;
                        while surface.width > max_texture.max(1) || surface.height > max_texture.max(1) {
                            surface = halve(&surface);
                        }
                        doc.images.push((crysirror_image::encode_png(&surface)?, "image/png"));
                        images.insert(desc.id, doc.images.len() - 1);
                        Some(doc.images.len() - 1)
                    }
                },
            };
            doc.materials.push(Material { name: group.material.name.clone(), base_color_texture: image, ..Default::default() });
            let material = doc.materials.len() - 1;
            for f in &mesh.fragments[group.fragments.clone()] {
                if f.positions.is_empty() || f.triangles.is_empty() {
                    continue;
                }
                primitives.push(Primitive {
                    positions: f.positions.clone(),
                    normals: if f.normals.len() == f.positions.len() { f.normals.clone() } else { Vec::new() },
                    uvs: if f.uvs.len() == f.positions.len() { f.uvs.clone() } else { Vec::new() },
                    indices: f.triangles.iter().flatten().map(|&i| u32::from(i)).collect(),
                    material: Some(material),
                    ..Default::default()
                });
            }
        }
        if primitives.is_empty() || mesh.instances.is_empty() {
            continue;
        }
        doc.meshes.push((format!("{name}/{index}"), primitives));
        let mesh_index = doc.meshes.len() - 1;
        for i in 0..mesh.instances.len() {
            let (translation, rotation, scale) = trs(&placement(mesh, i));
            let mut geometry = Node::named(format!("{name}/{index}/{i}/geometry"));
            geometry.scale = [METRES_PER_UNIT; 3];
            geometry.mesh = Some(mesh_index);
            doc.nodes.push(geometry);
            let mut placed = Node::named(format!("{name}/{index}/{i}"));
            placed.translation = translation;
            placed.rotation = rotation;
            placed.scale = scale;
            placed.children = vec![doc.nodes.len() - 1];
            doc.roots.push(doc.nodes.len());
            doc.nodes.push(placed);
        }
    }
    Ok(doc)
}

fn halve(s: &crysirror_image::ImageSurface) -> crysirror_image::ImageSurface {
    let (w, h) = ((s.width / 2).max(1), (s.height / 2).max(1));
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            for c in 0..4 {
                let mut sum = 0u32;
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let (sx, sy) = ((2 * x + dx).min(s.width - 1), (2 * y + dy).min(s.height - 1));
                    sum += u32::from(s.rgba[((sy * s.width + sx) * 4 + c) as usize]);
                }
                rgba[((y * w + x) * 4 + c) as usize] = ((sum + 2) / 4) as u8;
            }
        }
    }
    crysirror_image::ImageSurface { width: w, height: h, rgba }
}

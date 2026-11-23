// SPDX-License-Identifier: MPL-2.0
//! A package's prototype meshes as one glTF document for the shared world
//! renderer: one primitive per fragment with its group's base-colour
//! texture, in metres (native units are centimetres).
use crate::{
    mesh::{self, Mesh},
    package::Directory,
    texture::VramDesc,
};
use anyhow::Result;
use crysirror_asset::interchange::glb::{Document, Material, Node, Primitive};
use crysirror_core::service::AssetReader;
use std::collections::HashMap;

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
    let mut primitives = Vec::new();
    for mesh in meshes {
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
                    uvs: if f.uvs.len() == f.positions.len() { f.uvs.clone() } else { Vec::new() },
                    indices: f.triangles.iter().flatten().map(|&i| u32::from(i)).collect(),
                    material: Some(material),
                    ..Default::default()
                });
            }
        }
    }
    doc.meshes.push((name.to_owned(), primitives));
    let mut node = Node::named(name);
    node.scale = [METRES_PER_UNIT; 3];
    node.mesh = Some(0);
    doc.roots.push(0);
    doc.nodes.push(node);
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

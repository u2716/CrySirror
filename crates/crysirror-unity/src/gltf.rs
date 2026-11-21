// SPDX-License-Identifier: MPL-2.0
//! Assemble Unity GameObject hierarchies (Transform + MeshFilter/
//! MeshRenderer) into one GLB for the engine's world renderer.
//!
//! Unity is left-handed with bottom-up UVs; glTF is right-handed with
//! top-down UVs. Conversion mirrors X (positions, normals, translations,
//! quaternions `(x, -y, -z, w)`), reverses triangle winding and flips V.
use crate::classes::Mesh;
use anyhow::{ensure, Result};
use crysirror_asset::{interchange::glb, pbr::PbrMaterial};
use std::collections::HashMap;

pub fn mirror_point(v: [f32; 3]) -> [f32; 3] {
    [-v[0], v[1], v[2]]
}
pub fn mirror_quat(q: [f32; 4]) -> [f32; 4] {
    [q[0], -q[1], -q[2], q[3]]
}
/// `S·M·S` with S = diag(−1, 1, 1, 1) (column-major), e.g. bind poses.
pub fn mirror_matrix(m: [f32; 16]) -> [f32; 16] {
    let mut out = m;
    for col in 0..4 {
        for row in 0..4 {
            if (row == 0) != (col == 0) {
                out[col * 4 + row] = -m[col * 4 + row];
            }
        }
    }
    out
}

#[derive(Default)]
pub struct GlbBuilder {
    doc: glb::Document,
    images: HashMap<String, usize>,
    bytes: usize,
}

impl GlbBuilder {
    pub fn new(generator: &str) -> Self {
        Self { doc: glb::Document { generator: generator.into(), ..Default::default() }, images: HashMap::new(), bytes: 0 }
    }

    /// Add a node with its Unity local transform; returns its index.
    pub fn node(&mut self, name: &str, parent: Option<usize>, position: [f32; 3], rotation: [f32; 4], scale: [f32; 3]) -> Result<usize> {
        let index = self.doc.nodes.len();
        self.doc.nodes.push(glb::Node {
            translation: mirror_point(position),
            rotation: mirror_quat(rotation),
            scale,
            ..glb::Node::named(name)
        });
        match parent {
            Some(p) => {
                ensure!(p < index, "glTF parent must precede its child");
                self.doc.nodes[p].children.push(index);
            }
            None => self.doc.roots.push(index),
        }
        Ok(index)
    }

    /// Embed an encoded PNG/JPEG once per `key`.
    pub fn image(&mut self, key: &str, encoded: impl FnOnce() -> Result<(Vec<u8>, &'static str)>) -> Result<usize> {
        if let Some(i) = self.images.get(key) {
            return Ok(*i);
        }
        let (bytes, mime) = encoded()?;
        self.bytes += bytes.len();
        self.doc.images.push((bytes, mime));
        let i = self.doc.images.len() - 1;
        self.images.insert(key.to_owned(), i);
        Ok(i)
    }

    pub fn material(&mut self, name: &str, engine: PbrMaterial) -> usize {
        self.doc.materials.push(glb::Material {
            name: name.into(),
            base_color: engine.base_color_factor,
            base_color_texture: engine.base_color_texture,
            alpha_mode: match engine.alpha_mode {
                crysirror_asset::pbr::AlphaMode::Opaque => "OPAQUE",
                crysirror_asset::pbr::AlphaMode::Mask => "MASK",
                _ => "BLEND",
            },
            double_sided: engine.double_sided,
            roughness: engine.roughness_factor,
            engine: Some(engine),
            ..Default::default()
        });
        self.doc.materials.len() - 1
    }

    /// Attach `mesh` to `node`, one primitive per sub-mesh with the
    /// renderer's material of the same index (Unity reuses the last one).
    pub fn mesh(&mut self, node: usize, mesh: &Mesh, materials: &[usize]) -> Result<()> {
        let mut prims = Vec::new();
        for (s, sub) in mesh.submeshes.iter().enumerate() {
            ensure!(sub.topology == 0, "Unity mesh {} topology {}", mesh.name, sub.topology);
            let start = sub.first_byte as usize / 2;
            let end = start + sub.index_count as usize;
            let Some(indices) = mesh.indices.get(start..end) else { continue };
            if indices.len() < 3 {
                continue;
            }
            let mut remap: HashMap<u16, u32> = HashMap::new();
            let mut p = glb::Primitive { material: materials.get(s).or(materials.last()).copied(), ..Default::default() };
            for &i in indices {
                if remap.contains_key(&i) {
                    continue;
                }
                let v = i as usize;
                remap.insert(i, p.positions.len() as u32);
                p.positions.push(mirror_point(mesh.positions[v]));
                if let Some(n) = mesh.normals.get(v) {
                    p.normals.push(mirror_point(*n));
                }
                if let Some(uv) = mesh.uv0.get(v) {
                    p.uvs.push([uv[0], 1. - uv[1]]);
                }
            }
            if p.normals.len() != p.positions.len() {
                p.normals.clear();
            }
            if p.uvs.len() != p.positions.len() {
                p.uvs.clear();
            }
            for tri in indices.chunks_exact(3) {
                p.indices.extend([remap[&tri[0]], remap[&tri[2]], remap[&tri[1]]]);
            }
            prims.push(p);
        }
        if prims.is_empty() {
            return Ok(());
        }
        self.doc.meshes.push((mesh.name.clone(), prims));
        self.doc.nodes[node].mesh = Some(self.doc.meshes.len() - 1);
        Ok(())
    }

    /// Attach sub-meshes `range` of a Unity 2020+ mesh to `node`, one
    /// primitive per triangle sub-mesh with the renderer material of the
    /// same position (Unity reuses the last; `None` skips the sub-mesh).
    /// Returns the GLB mesh index (`None` when nothing was drawn), which
    /// other nodes may share through [`Self::set_mesh`].
    pub fn mesh6(&mut self, node: usize, mesh: &crate::unity6::Mesh6, range: std::ops::Range<usize>, materials: &[Option<usize>]) -> Result<Option<usize>> {
        self.mesh6_skinned(node, mesh, range, materials, false)
    }

    /// A skin of joint nodes with Unity bind poses; returns its index.
    pub fn skin(&mut self, joints: Vec<usize>, bind_poses: &[[f32; 16]]) -> usize {
        self.doc.skins.push(glb::Skin { joints, inverse_bind: bind_poses.iter().map(|m| mirror_matrix(*m)).collect(), skeleton: None });
        self.doc.skins.len() - 1
    }

    pub fn set_skin(&mut self, node: usize, skin: usize) {
        self.doc.nodes[node].skin = Some(skin);
    }

    /// [`Self::mesh6`] with the mesh's joints/weights when `skinned`.
    pub fn mesh6_skinned(&mut self, node: usize, mesh: &crate::unity6::Mesh6, range: std::ops::Range<usize>, materials: &[Option<usize>], skinned: bool) -> Result<Option<usize>> {
        let skinned = skinned && mesh.joints.len() == mesh.positions.len() && mesh.weights.len() == mesh.positions.len();
        let mut prims = Vec::new();
        for (slot, s) in range.enumerate() {
            let Some(sub) = mesh.submeshes.get(s) else { continue };
            if sub.topology != 0 {
                self.dropped(format!("mesh {} sub-mesh {s} topology {}", mesh.name, sub.topology));
                continue;
            }
            let Some(material) = materials.get(slot).or(materials.last()).copied() else { continue };
            let Some(material) = material else { continue };
            let start = sub.first_index as usize;
            let Some(indices) = mesh.indices.get(start..start + sub.index_count as usize) else { continue };
            let mut remap: HashMap<u32, u32> = HashMap::new();
            let mut p = glb::Primitive { material: Some(material), ..Default::default() };
            let mut tri = Vec::with_capacity(indices.len());
            for &i in indices {
                let v = (i + sub.base_vertex) as usize;
                if v >= mesh.positions.len() {
                    continue;
                }
                let at = *remap.entry(v as u32).or_insert_with(|| {
                    p.positions.push(mirror_point(mesh.positions[v]));
                    if let Some(n) = mesh.normals.get(v) {
                        p.normals.push(mirror_point(*n));
                    }
                    if let Some(uv) = mesh.uv[0].get(v) {
                        p.uvs.push([uv[0], 1. - uv[1]]);
                    }
                    if skinned {
                        p.joints.push(mesh.joints[v]);
                        p.weights.push(mesh.weights[v]);
                    }
                    (p.positions.len() - 1) as u32
                });
                tri.push(at);
            }
            if p.normals.len() != p.positions.len() {
                p.normals.clear();
            }
            if p.uvs.len() != p.positions.len() {
                p.uvs.clear();
            }
            for t in tri.chunks_exact(3) {
                p.indices.extend([t[0], t[2], t[1]]);
            }
            if !p.indices.is_empty() {
                self.bytes += p.positions.len() * if skinned { 56 } else { 32 } + p.indices.len() * 4;
                prims.push(p);
            }
        }
        if prims.is_empty() {
            return Ok(None);
        }
        self.doc.meshes.push((mesh.name.clone(), prims));
        let index = self.doc.meshes.len() - 1;
        self.doc.nodes[node].mesh = Some(index);
        Ok(Some(index))
    }

    /// A named animation of Unity-space transform tracks on GLB nodes.
    pub fn animation(&mut self, name: &str, tracks: Vec<(usize, Vec<f32>, crate::world6::TrackValues)>) {
        let tracks = tracks
            .into_iter()
            .map(|(node, times, values)| glb::Track {
                node,
                times,
                values: match values {
                    crate::world6::TrackValues::Translation(v) => glb::Channel::Translation(v.into_iter().map(mirror_point).collect()),
                    crate::world6::TrackValues::Rotation(v) => glb::Channel::Rotation(v.into_iter().map(mirror_quat).collect()),
                    crate::world6::TrackValues::Scale(v) => glb::Channel::Scale(v),
                },
                tangents: None,
            })
            .collect();
        self.doc.animations.push(glb::Animation { name: name.into(), tracks });
    }

    pub fn set_mesh(&mut self, node: usize, mesh: usize) {
        self.doc.nodes[node].mesh = Some(mesh);
    }

    /// Approximate GLB payload so far (vertex attributes, indices, images).
    pub fn estimated_bytes(&self) -> usize {
        self.bytes
    }

    pub fn dropped(&mut self, note: String) {
        self.doc.dropped.push(note);
    }

    pub fn is_empty(&self) -> bool {
        self.doc.meshes.is_empty()
    }

    pub fn finish(self) -> Result<Vec<u8>> {
        self.doc.to_glb()
    }
}

/// One unlit, two-sided textured quad with corners (top-left, top-right,
/// bottom-right, bottom-left) in engine space: a screen-space camera canvas
/// (e.g. a background image) placed in the 3D view.
pub fn image_plane_glb(generator: &str, png: Vec<u8>, corners: [[f32; 3]; 4]) -> Result<Vec<u8>> {
    let mut doc = glb::Document { generator: generator.into(), ..Default::default() };
    doc.images.push((png, "image/png"));
    let engine = PbrMaterial { base_color_texture: Some(0), unlit: true, double_sided: true, ..Default::default() };
    doc.materials.push(glb::Material { name: "image".into(), base_color_texture: Some(0), double_sided: true, engine: Some(engine), ..Default::default() });
    let [a, b, c, d] = corners;
    let n = {
        let (u, v) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [d[0] - a[0], d[1] - a[1], d[2] - a[2]]);
        let n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
        let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-6);
        n.map(|x| x / l)
    };
    let p = glb::Primitive { positions: vec![a, b, c, d], normals: vec![n; 4], uvs: vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]], indices: vec![0, 3, 2, 0, 2, 1], material: Some(0), ..Default::default() };
    doc.meshes.push(("image".into(), vec![p]));
    doc.nodes.push(glb::Node { mesh: Some(0), ..glb::Node::named("image") });
    doc.roots.push(0);
    doc.to_glb()
}

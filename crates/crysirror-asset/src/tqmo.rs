// SPDX-License-Identifier: MPL-2.0
//! Device-free TQMO/MQO binary model import.
//!
//! Native format evidence: 5770D8 (model), 578980 (scene), 5783CC/
//! 5784D0/5785E0 (materials), 577F1C/578020/578100 (objects), 4E4C94
//! (signed-i16 length followed by CP932 or UTF16). These are file records,
//! not copied Delphi objects, GPU buffers or an embedded native renderer.
//! Geometry stays in its source basis; source version is retained so a caller
//! can select a declared basis conversion rather than silently lose metadata.
use anyhow::{Context, Result, ensure};
use encoding_rs::SHIFT_JIS;
use serde::{Deserialize, Serialize};
mod legacy;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Model {
    pub version: u8,
    #[serde(default)]
    pub legacy_upgraded: bool,
    pub scene: Scene,
    pub materials: Vec<Material>,
    pub objects: Vec<Object>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scene {
    pub version: u8,
    pub position: [f32; 3],
    /// File offset corresponds to native scene field16.
    pub parameter: f32,
    /// Native fields28,32,36,40, in file order.
    pub camera: [f32; 4],
    pub ambient_argb: [f32; 4],
    pub light_direction: [f32; 3],
    pub light_argb: [f32; 4],
    pub base_argb: [f32; 4],
    pub fog_near: f32,
    pub fog_far: f32,
    pub fog_mode: f32,
    pub fog_argb: [f32; 4],
    pub extra_vectors: [[f32; 3]; 2],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Material {
    pub version: u8,
    /// The additional v1.1 DWORD is read but not used by the native loader.
    pub attributes: u32,
    pub name: String,
    pub texture: String,
    pub shader: i32,
    pub vertex_color: i32,
    pub color_argb: [f32; 4],
    pub diffuse: f32,
    pub ambient: f32,
    pub emission: f32,
    pub specular: f32,
    pub power: f32,
    /// Stored fields56/60. Both original readers mistakenly invoke the stream
    /// write vtable for these fields, leaving the native values zero. Retain
    /// authored bytes here; caller policy can explicitly select original-zero
    /// behavior or the corrected authored values instead of losing the data.
    pub stored_render_flags: [u32; 2],
    /// Effective flags after the original loader's zero-write behavior and
    /// optional v1.0 material-name annotations, separate from authored bytes.
    #[serde(default)]
    pub native_render_flags: [u32; 2],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Object {
    pub version: u8,
    pub attributes: u32,
    pub name: String,
    pub visible: bool,
    pub facet_angle: f32,
    pub vertices: Vec<[f32; 3]>,
    pub faces: Vec<Face>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Face {
    pub corners: u8,
    pub material: i32,
    pub vertices: [u32; 4],
    pub uvs: [[f32; 2]; 4],
    pub colors_argb: [[f32; 4]; 4],
    pub color_argb: [f32; 4],
    pub normal: [f32; 3],
}
impl Model {
    /// Convert decoded geometry/materials to the shared fixed-function scene
    /// evaluator. Blend and sampler selection are explicit caller policy:
    /// native readers and corrected authoring import may use different flags.
    pub fn to_scene_model(
        &self,
        material_render: &[(crysirror_scene::layer::Blend, bool)],
    ) -> Result<crysirror_scene::legacy_mesh::Model> {
        use crysirror_scene::legacy_mesh as target;
        ensure!(
            material_render.len() == self.materials.len(),
            "TQMO material render policy length"
        );
        let model = target::Model {
            scene: target::Scene {
                ambient_argb: self.scene.ambient_argb,
                light_direction: self.scene.light_direction,
                specular_argb: self.scene.base_argb,
                fog_near: self.scene.fog_near,
                fog_far: self.scene.fog_far,
                fog_power: self.scene.fog_mode,
                fog_argb: self.scene.fog_argb,
                fog_axes: self.scene.extra_vectors[1],
                fog_origin: [0.; 3],
            },
            materials: self
                .materials
                .iter()
                .zip(material_render)
                .map(|(m, (blend, linear))| target::Material {
                    shader: m.shader,
                    vertex_color: m.vertex_color == 1,
                    color_argb: m.color_argb,
                    diffuse: m.diffuse,
                    ambient: m.ambient,
                    emission: m.emission,
                    specular: m.specular,
                    power: m.power,
                    blend: *blend,
                    linear: *linear,
                })
                .collect(),
            objects: self
                .objects
                .iter()
                .map(|o| target::Object {
                    visible: o.visible,
                    vertices: o.vertices.clone(),
                    faces: o
                        .faces
                        .iter()
                        .map(|f| target::Face {
                            corners: f.corners,
                            material: f.material,
                            vertices: f.vertices,
                            uvs: f.uvs,
                            color_argb: f.color_argb,
                            normal: f.normal,
                        })
                        .collect(),
                })
                .collect(),
        };
        model.validate()?;
        Ok(model)
    }
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() <= crate::MAX_IMPORT_BYTES, "TQMO input budget");
        // 57A79C/57A860: a model resource may carry a 16-byte
        // "ARGBModel1.0" container tag before the TQMO model record.
        let bytes = match bytes.get(..16) {
            Some(tag) if tag.starts_with(b"ARGBModel1.0") && tag[12..].iter().all(|b| *b == 0) => {
                &bytes[16..]
            }
            _ => bytes,
        };
        let mut c = Cursor { bytes, position: 0 };
        let version = c.version("TQMOmdl", &[10, 11])?;
        let scene = c.scene()?;
        let material_count = c.count(4096)?;
        let mut materials = Vec::with_capacity(material_count);
        for _ in 0..material_count {
            materials.push(c.material()?);
        }
        let object_count = c.count(4096)?;
        let mut objects = Vec::with_capacity(object_count);
        let mut vertices = 0usize;
        let mut faces = 0usize;
        for _ in 0..object_count {
            let object = c.object(material_count)?;
            vertices = vertices
                .checked_add(object.vertices.len())
                .context("TQMO vertex count overflow")?;
            faces = faces
                .checked_add(object.faces.len())
                .context("TQMO face count overflow")?;
            ensure!(
                vertices <= 1_000_000 && faces <= crate::MAX_TRIANGLES,
                "TQMO model geometry budget"
            );
            objects.push(object);
        }
        ensure!(c.position == bytes.len(), "TQMO trailing model data");
        Ok(Self {
            version,
            legacy_upgraded: false,
            scene,
            materials,
            objects,
        })
    }
}
impl Object {
    /// Apply a reflected/scaled basis, including native corner winding,
    /// UV/colour permutation and recomputed face normals/colour averages.
    /// TQMO 1.0 native upgrade invokes this with [1,-1,1].
    pub fn scale_basis(&mut self, scale: [f32; 3]) -> Result<()> {
        ensure!(
            scale.iter().all(|v| v.is_finite()),
            "TQMO nonfinite basis scale"
        );
        for vertex in &mut self.vertices {
            for axis in 0..3 {
                vertex[axis] *= scale[axis];
            }
            ensure!(
                vertex.iter().all(|v| v.is_finite()),
                "TQMO basis vertex overflow"
            );
        }
        for face in &mut self.faces {
            let count = usize::from(face.corners);
            if scale[0] * scale[1] * scale[2] < 0. {
                face.vertices.swap(1, count - 1);
                face.uvs.swap(1, count - 1);
                face.colors_argb.swap(1, count - 1);
            }
            let [a, b, c] = std::array::from_fn(|i| {
                glam::Vec3::from_array(self.vertices[face.vertices[i] as usize])
            });
            face.normal = (b - a).cross(c - a).normalize_or_zero().to_array();
            face.color_argb = std::array::from_fn(|component| {
                let mut total = 0f32;
                for corner in 0..count {
                    total += face.colors_argb[corner][component];
                }
                total * if count == 3 { 0.33333334 } else { 0.25 }
            });
        }
        Ok(())
    }
}
struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}
impl<'a> Cursor<'a> {
    fn take(&mut self, length: usize) -> Result<&'a [u8]> {
        let end = self
            .position
            .checked_add(length)
            .context("TQMO offset overflow")?;
        let bytes = self
            .bytes
            .get(self.position..end)
            .context("truncated TQMO record")?;
        self.position = end;
        Ok(bytes)
    }
    fn word(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into()?))
    }
    fn integer(&mut self) -> Result<i32> {
        Ok(self.word()? as i32)
    }
    fn float(&mut self) -> Result<f32> {
        let value = f32::from_bits(self.word()?);
        ensure!(value.is_finite(), "TQMO nonfinite numeric field");
        Ok(value)
    }
    fn floats<const N: usize>(&mut self) -> Result<[f32; N]> {
        let mut out = [0.; N];
        for value in &mut out {
            *value = self.float()?;
        }
        Ok(out)
    }
    fn count(&mut self, limit: usize) -> Result<usize> {
        let count = usize::try_from(self.integer()?).context("negative TQMO record count")?;
        ensure!(count <= limit, "TQMO record count budget");
        Ok(count)
    }
    fn version(&mut self, family: &str, supported: &[u8]) -> Result<u8> {
        let raw = self.take(16)?;
        let end = raw.iter().position(|b| *b == 0).unwrap_or(raw.len());
        let tag = std::str::from_utf8(&raw[..end]).context("TQMO tag encoding")?;
        for version in supported {
            if tag == format!("{family}{}.{}", version / 10, version % 10) {
                return Ok(*version);
            }
        }
        anyhow::bail!("unsupported TQMO record {tag:?}")
    }
    fn string(&mut self, ansi: bool) -> Result<String> {
        let size = i16::from_le_bytes(self.take(2)?.try_into()?);
        let size = usize::try_from(size).context("negative TQMO string length")?;
        if ansi {
            SHIFT_JIS
                .decode_without_bom_handling_and_without_replacement(self.take(size)?)
                .map(|s| s.into_owned())
                .context("invalid TQMO CP932 string")
        } else {
            let bytes = self.take(size * 2)?;
            let units = bytes
                .chunks_exact(2)
                .map(|p| u16::from_le_bytes([p[0], p[1]]))
                .collect::<Vec<_>>();
            String::from_utf16(&units).context("invalid TQMO UTF16 string")
        }
    }
    fn scene(&mut self) -> Result<Scene> {
        let version = self.version("MQOscn", &[10, 11, 12])?;
        let position = self.floats()?;
        let parameter = self.float()?;
        let camera = self.floats()?;
        let ambient_argb = self.floats()?;
        let light_direction = self.floats()?;
        let light_argb = if version == 12 {
            self.floats()?
        } else {
            [1.; 4]
        };
        let base_argb = self.floats()?;
        let (fog_near, fog_far, fog_mode, fog_argb, extra_vectors) = if version >= 11 {
            (
                self.float()?,
                self.float()?,
                self.float()?,
                self.floats()?,
                [self.floats()?, self.floats()?],
            )
        } else {
            (
                100000.,
                100000.,
                1.,
                [1., 0., 0., 0.],
                [[0., 0., 0.], [0., 0., 1.]],
            )
        };
        Ok(Scene {
            version,
            position,
            parameter,
            camera,
            ambient_argb,
            light_direction,
            light_argb,
            base_argb,
            fog_near,
            fog_far,
            fog_mode,
            fog_argb,
            extra_vectors,
        })
    }
    fn material(&mut self) -> Result<Material> {
        let version = self.version("MQOmat", &[10, 11])?;
        let attributes = if version == 11 { self.word()? } else { 0 };
        let name = self.string(version == 10)?;
        let texture = self.string(version == 10)?;
        let shader = self.integer()?;
        let vertex_color = self.integer()?;
        let color_argb = self.floats()?;
        let diffuse = self.float()?;
        let ambient = self.float()?;
        let emission = self.float()?;
        let specular = self.float()?;
        let power = self.float()?;
        let stored_render_flags = [self.word()?, self.word()?];
        Ok(Material {
            version,
            attributes,
            name,
            texture,
            shader,
            vertex_color,
            color_argb,
            diffuse,
            ambient,
            emission,
            specular,
            power,
            stored_render_flags,
            native_render_flags: [0; 2],
        })
    }
    fn object(&mut self, material_count: usize) -> Result<Object> {
        let version = self.version("MQOobj", &[10, 11])?;
        let attributes = if version == 11 { self.word()? } else { 0 };
        let name = self.string(version == 10)?;
        let visible = self.take(1)?[0] != 0;
        let facet_angle = self.float()?;
        let vertex_count = self.count(1_000_000)?;
        let face_count = self.count(crate::MAX_TRIANGLES)?;
        ensure!(
            vertex_count
                .checked_mul(12)
                .and_then(|v| face_count.checked_mul(148).and_then(|f| v.checked_add(f)))
                .is_some_and(|size| size <= self.bytes.len() - self.position),
            "truncated TQMO geometry arrays"
        );
        let mut vertices = Vec::with_capacity(vertex_count);
        for _ in 0..vertex_count {
            vertices.push(self.floats()?);
        }
        let mut faces = Vec::with_capacity(face_count);
        for _ in 0..face_count {
            let corners = self.count(4)?;
            ensure!(
                matches!(corners, 3 | 4),
                "TQMO polygon must have 3 or 4 corners"
            );
            let material = self.integer()?;
            ensure!(
                material >= -1 && (material < 0 || (material as usize) < material_count),
                "TQMO material index"
            );
            let indices: [u32; 4] = [self.word()?, self.word()?, self.word()?, self.word()?];
            ensure!(
                indices[..corners]
                    .iter()
                    .all(|i| (*i as usize) < vertex_count),
                "TQMO face vertex index"
            );
            let u: [f32; 4] = self.floats()?;
            let v: [f32; 4] = self.floats()?;
            let uvs = std::array::from_fn(|i| [u[i], v[i]]);
            let colors_argb = [
                self.floats()?,
                self.floats()?,
                self.floats()?,
                self.floats()?,
            ];
            let color_argb = self.floats()?;
            let normal = self.floats()?;
            faces.push(Face {
                corners: corners as u8,
                material,
                vertices: indices,
                uvs,
                colors_argb,
                color_argb,
                normal,
            });
        }
        Ok(Object {
            version,
            attributes,
            name,
            visible,
            facet_angle,
            vertices,
            faces,
        })
    }
}

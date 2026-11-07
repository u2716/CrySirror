//! Immutable fixed-function model inputs and worker-ready mesh evaluation.
//!
//! Flat material lighting, object morphs and anisotropic distance fog are
//! reusable scene capabilities. Native record readers map into these types;
//! no VM, file reader, graphics device or proprietary object lives here.
use crate::{
    layer::{Blend, SpriteTransform},
    projected_mesh::{MeshFrame, Part, Vertex},
    projection,
};
use anyhow::{Context, Result, ensure};
use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Model {
    pub scene: Scene,
    pub materials: Vec<Material>,
    pub objects: Vec<Object>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scene {
    pub ambient_argb: [f32; 4],
    pub light_direction: [f32; 3],
    pub specular_argb: [f32; 4],
    pub fog_near: f32,
    pub fog_far: f32,
    pub fog_power: f32,
    pub fog_argb: [f32; 4],
    pub fog_axes: [f32; 3],
    pub fog_origin: [f32; 3],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Material {
    pub shader: i32,
    pub vertex_color: bool,
    pub color_argb: [f32; 4],
    pub diffuse: f32,
    pub ambient: f32,
    pub emission: f32,
    pub specular: f32,
    pub power: f32,
    pub blend: Blend,
    /// Portable sampling policy, selected explicitly by the importer/profile.
    pub linear: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Object {
    pub visible: bool,
    pub vertices: Vec<[f32; 3]>,
    pub faces: Vec<Face>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Face {
    pub corners: u8,
    pub material: i32,
    pub vertices: [u32; 4],
    pub uvs: [[f32; 2]; 4],
    pub color_argb: [f32; 4],
    pub normal: [f32; 3],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spec {
    pub model: Arc<Model>,
    /// None draws each visible object; an explicit object draws regardless of
    /// its visibility flag, matching ordinary object-selection semantics.
    pub object: Option<u32>,
    pub morph_object: Option<u32>,
    pub morph_weight: f32,
    /// Declared importer/render-profile basis before the retained model pose.
    pub basis: [f32; 3],
    /// Optional affine coordinate conversion after the full retained model
    /// hierarchy, before lighting/fog and geometric queue ordering.
    #[serde(default)]
    pub post_transform: Option<[f32; 16]>,
    /// Cull a complete face if any of its projected triangles has negative
    /// signed area in canvas coordinates. False preserves two-sided geometry.
    #[serde(default)]
    pub culling: bool,
    /// Optional view-space lighting/fog origin, independent of projection.
    /// A native software queue may already contain camera-relative geometry.
    pub view_origin: Option<[f32; 3]>,
    pub light_direction: Option<[f32; 3]>,
    pub uv_offset: [f32; 2],
    /// Optional per-vertex texture coordinate bounds, applied before clipping
    /// and interpolation. This is distinct from fragment sampler addressing.
    #[serde(default)]
    pub uv_bounds: Option<[f32; 4]>,
    pub tint: [u8; 4],
    pub add: [u8; 3],
    pub blend_override: Option<Blend>,
    pub sampling_override: Option<bool>,
    /// Texture paths indexed by material. None is a solid white texture.
    pub textures: Vec<Option<String>>,
    /// Discard coverage supplied by the ordered compositor for this draw.
    /// This does not erase the retained coverage state for subsequent draws.
    pub ignore_coverage: bool,
}
fn finite(mut values: impl Iterator<Item = f32>) -> bool {
    values.all(|v| v.is_finite() && v.abs() <= 1_000_000.)
}
impl Model {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.materials.len() <= 4096 && self.objects.len() <= 4096,
            "legacy mesh object/material budget"
        );
        let s = &self.scene;
        ensure!(
            finite(
                s.ambient_argb
                    .into_iter()
                    .chain(s.light_direction)
                    .chain(s.specular_argb)
                    .chain([s.fog_near, s.fog_far, s.fog_power])
                    .chain(s.fog_argb)
                    .chain(s.fog_axes)
                    .chain(s.fog_origin)
            ),
            "legacy mesh scene values"
        );
        let mut vertices = 0usize;
        let mut faces = 0usize;
        for material in &self.materials {
            ensure!(
                finite(material.color_argb.into_iter().chain([
                    material.diffuse,
                    material.ambient,
                    material.emission,
                    material.specular,
                    material.power
                ])),
                "legacy mesh material values"
            );
        }
        for object in &self.objects {
            vertices = vertices
                .checked_add(object.vertices.len())
                .context("legacy mesh vertex overflow")?;
            faces = faces
                .checked_add(object.faces.len())
                .context("legacy mesh face overflow")?;
            ensure!(
                vertices <= 1_000_000 && faces <= 333333,
                "legacy mesh geometry budget"
            );
            ensure!(
                finite(object.vertices.iter().flatten().copied()),
                "legacy mesh vertex values"
            );
            for face in &object.faces {
                ensure!(
                    matches!(face.corners, 3 | 4)
                        && face.vertices[..usize::from(face.corners)]
                            .iter()
                            .all(|v| (*v as usize) < object.vertices.len()),
                    "legacy mesh face indices"
                );
                ensure!(
                    face.material >= -1
                        && (face.material < 0 || (face.material as usize) < self.materials.len()),
                    "legacy mesh material index"
                );
                ensure!(
                    finite(
                        face.uvs
                            .iter()
                            .flatten()
                            .copied()
                            .chain(face.color_argb)
                            .chain(face.normal)
                    ),
                    "legacy mesh face values"
                );
            }
        }
        Ok(())
    }
}
impl Spec {
    pub fn validate(&self) -> Result<()> {
        self.model.validate()?;
        ensure!(
            self.morph_weight.is_finite() && finite(self.uv_offset.into_iter().chain(self.basis)),
            "legacy mesh draw values"
        );
        ensure!(
            self.view_origin.is_none_or(|v| finite(v.into_iter())),
            "legacy mesh view origin"
        );
        ensure!(
            self.light_direction.is_none_or(|v| finite(v.into_iter())),
            "legacy mesh light direction"
        );
        if let Some(matrix) = self.post_transform {
            ensure!(
                finite(matrix.into_iter())
                    && matrix[3] == 0.
                    && matrix[7] == 0.
                    && matrix[11] == 0.
                    && matrix[15] == 1.,
                "legacy mesh post-transform bounds"
            );
        }
        if let Some(bounds) = self.uv_bounds {
            ensure!(
                finite(bounds.into_iter()) && bounds[2] > bounds[0] && bounds[3] > bounds[1],
                "legacy mesh vertex UV bounds"
            );
        }
        ensure!(self.textures.len() <= 4096, "legacy mesh texture budget");
        for texture in self.textures.iter().flatten() {
            crysirror_core::path::VfsPath::new(crysirror_core::path::Mount::Asset, texture)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            ensure!(
                !texture.starts_with("__surface/")
                    && crysirror_core::service::frame_history_layer(texture).is_none(),
                "legacy mesh requires immutable image dependencies"
            );
        }
        Ok(())
    }
}
fn byte(value: f32) -> u8 {
    ((value * 255. + 0.5).trunc() as i64) as u8
}
fn linear(value: u8) -> f32 {
    let v = f32::from(value) / 255.;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}
fn normalized(v: Vec3) -> Vec3 {
    let length = v.length();
    if f64::from(length) >= 0.00001 {
        v / length
    } else {
        Vec3::ZERO
    }
}
#[derive(Clone, Copy)]
struct Corner {
    position: Vec3,
    uv: [f32; 2],
    color: [f32; 4],
}
fn clip_depth(mut polygon: Vec<Corner>, camera: &SpriteTransform) -> Vec<Corner> {
    let (eye, forward, near, far) = if let Some(lens) = camera.canvas_projection {
        if lens.near_clamp {
            if lens.admits(polygon.iter().map(|v| v.position.to_array())) {
                return polygon;
            } else {
                return Vec::new();
            }
        }
        (Vec3::ZERO, Vec3::Z, lens.near, lens.far)
    } else {
        if !camera.camera_enabled {
            return polygon;
        }
        let eye = Vec3::from_array(camera.camera_eye);
        (
            eye,
            normalized(Vec3::from_array(camera.camera_target) - eye),
            1.,
            10000.,
        )
    };
    //Clip interpolated attributes before division for ordinary perspective;
    //the explicit near-clamp profile instead preserves its complete face.
    for (plane, sign) in [(near, 1.), (far, -1.)] {
        if polygon.is_empty() {
            break;
        }
        let mut clipped = Vec::with_capacity(polygon.len() + 1);
        let mut previous = *polygon.last().unwrap();
        let mut previous_distance = ((previous.position - eye).dot(forward) - plane) * sign;
        for current in polygon {
            let distance = ((current.position - eye).dot(forward) - plane) * sign;
            if (distance >= 0.) != (previous_distance >= 0.) {
                let amount = previous_distance / (previous_distance - distance);
                clipped.push(Corner {
                    position: previous.position.lerp(current.position, amount),
                    uv: std::array::from_fn(|i| {
                        previous.uv[i] + (current.uv[i] - previous.uv[i]) * amount
                    }),
                    color: std::array::from_fn(|i| {
                        previous.color[i] + (current.color[i] - previous.color[i]) * amount
                    }),
                });
            }
            if distance >= 0. {
                clipped.push(current)
            }
            previous = current;
            previous_distance = distance;
        }
        polygon = clipped;
    }
    polygon
}
/// Evaluate with the complete retained model matrix and final camera.
/// Face admission order remains stable; this function never distance-sorts.
pub fn evaluate_world(
    spec: &Spec,
    matrix: Mat4,
    camera: &SpriteTransform,
    canvas: [u32; 2],
) -> Result<WorldFrame> {
    ensure!(
        matrix.is_finite() && canvas.iter().all(|v| *v > 0 && *v <= 16384),
        "legacy mesh projection bounds"
    );
    let matrix = spec
        .post_transform
        .map_or(matrix, |post| Mat4::from_cols_array(&post) * matrix);
    let mut frame = MeshFrame {
        size: canvas.map(|v| v as f32),
        parts: Vec::new(),
    };
    let model = &spec.model;
    let scene = &model.scene;
    let light = Vec3::from_array(spec.light_direction.unwrap_or(scene.light_direction));
    let view = spec.view_origin.map(Vec3::from_array).unwrap_or_else(|| {
        if camera.camera_enabled {
            Vec3::from_array(camera.camera_eye)
        } else {
            Vec3::ZERO
        }
    });
    let selection: Vec<usize> = if let Some(index) = spec.object {
        if (index as usize) >= model.objects.len() {
            return Ok(WorldFrame {
                frame,
                culling: spec.culling,
            });
        }
        vec![index as usize]
    } else {
        model
            .objects
            .iter()
            .enumerate()
            .filter_map(|(i, o)| o.visible.then_some(i))
            .collect()
    };
    for index in selection {
        let mut object = &model.objects[index];
        let mut morph = spec
            .morph_object
            .and_then(|i| model.objects.get(i as usize))
            .filter(|other| {
                !std::ptr::eq(*other, object)
                    && other.vertices.len() == object.vertices.len()
                    && other.faces.len() == object.faces.len()
            });
        //579D38 selects the target object itself at the upper endpoint, so
        //its face topology/material assignments replace the source too.
        if f64::from(spec.morph_weight) > 0.99999 {
            if let Some(target) = morph {
                object = target;
                morph = None;
            }
        } else if f64::from(spec.morph_weight) < 0.00001 {
            morph = None;
        }
        let amount = if morph.is_some() {
            spec.morph_weight.clamp(0., 1.)
        } else {
            0.
        };
        let positions: Vec<Vec3> = object
            .vertices
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let from = Vec3::from_array(*v);
                let value = morph.map_or(from, |to| {
                    from.lerp(Vec3::from_array(to.vertices[i]), amount)
                });
                matrix.transform_point3(value * Vec3::from_array(spec.basis))
            })
            .collect();
        for (face_index, face) in object.faces.iter().enumerate() {
            let material = if face.material >= 0 {
                model.materials.get(face.material as usize)
            } else {
                None
            };
            let fallback = Material {
                shader: 1,
                vertex_color: true,
                color_argb: [1.; 4],
                diffuse: 1.,
                ambient: 0.,
                emission: 0.,
                specular: 0.,
                power: 0.,
                blend: Blend::Normal,
                linear: false,
            };
            let material = material.unwrap_or(&fallback);
            let other = morph
                .and_then(|o| o.faces.get(face_index))
                .filter(|f| f.corners == face.corners);
            let mix = |a: f32, b: f32| a * (1. - amount) + b * amount;
            let color = if material.vertex_color {
                std::array::from_fn(|i| {
                    other.map_or(face.color_argb[i], |f| {
                        mix(face.color_argb[i], f.color_argb[i])
                    })
                })
            } else {
                material.color_argb
            };
            let normal = other.map_or(Vec3::from_array(face.normal), |f| {
                normalized(Vec3::from_array(face.normal).lerp(Vec3::from_array(f.normal), amount))
            });
            let normal =
                normalized(matrix.transform_vector3(normal * Vec3::from_array(spec.basis)));
            let count = usize::from(face.corners);
            let centre = face.vertices[..count]
                .iter()
                .map(|i| positions[*i as usize])
                .sum::<Vec3>()
                / count as f32;
            let (mut diffuse, mut additive) = if material.shader == 1 {
                (color, [1., 0., 0., 0.])
            } else {
                let dot = normal.dot(light).max(0.);
                let intensity = (material.diffuse * dot + material.emission).min(1.);
                let diffuse = [
                    color[0],
                    (color[1] * intensity + material.ambient * scene.ambient_argb[1]).min(1.),
                    (color[2] * intensity + material.ambient * scene.ambient_argb[2]).min(1.),
                    (color[3] * intensity + material.ambient * scene.ambient_argb[3]).min(1.),
                ];
                let specular = if f64::from(material.specular) >= 0.001
                    && f64::from(material.power) >= 0.001
                {
                    normalized(normal * (2. * dot) - light)
                        .dot(normalized(view - centre))
                        .clamp(0., 1.)
                        .powf(material.power)
                        * material.specular
                } else {
                    0.
                };
                (
                    diffuse,
                    [
                        1.,
                        scene.specular_argb[1] * specular,
                        scene.specular_argb[2] * specular,
                        scene.specular_argb[3] * specular,
                    ],
                )
            };
            if scene.fog_near + 1. < scene.fog_far {
                let distance = ((centre - view - Vec3::from_array(scene.fog_origin))
                    * Vec3::from_array(scene.fog_axes))
                .length();
                let fog = ((distance - scene.fog_near) / (scene.fog_far - scene.fog_near))
                    .clamp(0., 1.)
                    .powf(scene.fog_power);
                ensure!(fog.is_finite(), "legacy mesh fog exponent");
                diffuse[0] = diffuse[0] * (1. - fog) + scene.fog_argb[0] * fog;
                for i in 1..4 {
                    diffuse[i] *= 1. - fog;
                    additive[i] = additive[i] * (1. - fog) + scene.fog_argb[i] * fog;
                }
            }
            //57932C quantizes ARGB to stored bytes before integer colour
            //multiplication/addition. Scene output converts RGB to linear only
            //after those native byte operations, never before.
            let diffuse = diffuse.map(byte);
            let additive = additive.map(byte);
            let rgb = std::array::from_fn::<_, 3, _>(|i| {
                linear((u16::from(diffuse[i + 1]) * u16::from(spec.tint[i]) / 255) as u8)
            });
            let alpha =
                f32::from((u16::from(diffuse[0]) * u16::from(spec.tint[3]) / 255) as u8) / 255.;
            let mut polygon = Vec::with_capacity(count);
            for (corner, &vertex) in face.vertices[..count].iter().enumerate() {
                let uv = std::array::from_fn(|axis| {
                    let value = other.map_or(face.uvs[corner][axis], |f| {
                        mix(face.uvs[corner][axis], f.uvs[corner][axis])
                    }) + spec.uv_offset[axis];
                    spec.uv_bounds
                        .map_or(value, |bounds| value.clamp(bounds[axis], bounds[axis + 2]))
                });
                polygon.push(Corner {
                    position: positions[vertex as usize],
                    uv,
                    color: [rgb[0], rgb[1], rgb[2], alpha],
                });
            }
            let mut vertices = Vec::with_capacity(polygon.len());
            for corner in polygon {
                vertices.push(Vertex {
                    position: [corner.position.x, corner.position.y, corner.position.z, 1.],
                    uv: corner.uv,
                    color: corner.color,
                });
            }
            let indices = (1..vertices.len() as u32 - 1)
                .flat_map(|i| [0, i, i + 1])
                .collect();
            let part = Part {
                vertices,
                indices,
                texture: if face.material >= 0 {
                    spec.textures.get(face.material as usize).cloned().flatten()
                } else {
                    None
                },
                blend: Some(spec.blend_override.unwrap_or(material.blend)),
                add: std::array::from_fn(|i| {
                    f32::from(additive[i + 1].saturating_add(spec.add[i])) / 255.
                }),
                linear: spec.sampling_override.unwrap_or(material.linear),
            };
            // Preserve native face boundaries for geometric queue ordering
            // and destination-dependent blends, even when draw state matches.
            frame.parts.push(part)
        }
    }
    frame.validate()?;
    Ok(WorldFrame {
        frame,
        culling: spec.culling,
    })
}
/// Completed lighting in world coordinates. Ordered compositor scopes may
/// transform this geometry afterwards without incorrectly relighting it.
pub struct WorldFrame {
    frame: MeshFrame,
    culling: bool,
}
impl WorldFrame {
    pub fn ordered_faces(
        self,
        order: crate::projection::GeometryOrder,
    ) -> Result<Vec<(i32, Self)>> {
        self.frame
            .parts
            .into_iter()
            .map(|part| {
                let key = order.key(
                    part.vertices
                        .iter()
                        .map(|v| [v.position[0], v.position[1], v.position[2]]),
                )?;
                Ok((
                    key,
                    Self {
                        culling: self.culling,
                        frame: MeshFrame {
                            size: self.frame.size,
                            parts: vec![part],
                        },
                    },
                ))
            })
            .collect()
    }
    pub fn transform(&mut self, matrix: Mat4) -> Result<()> {
        ensure!(matrix.is_finite(), "legacy mesh ordered transform");
        for part in &mut self.frame.parts {
            for vertex in &mut part.vertices {
                let p = matrix.transform_point3(Vec3::new(
                    vertex.position[0],
                    vertex.position[1],
                    vertex.position[2],
                ));
                ensure!(p.is_finite(), "legacy mesh transformed vertex overflow");
                vertex.position = [p.x, p.y, p.z, 1.];
            }
        }
        Ok(())
    }
    pub fn project(self, camera: &SpriteTransform, canvas: [u32; 2]) -> Result<MeshFrame> {
        let mut frame = MeshFrame {
            size: canvas.map(|v| v as f32),
            parts: Vec::with_capacity(self.frame.parts.len()),
        };
        for source in self.frame.parts {
            if let Some(lens) = camera.canvas_projection {
                if !lens.admits(
                    source
                        .vertices
                        .iter()
                        .map(|v| [v.position[0], v.position[1], v.position[2]]),
                ) {
                    continue;
                }
            }
            if self.culling {
                let projected = source
                    .vertices
                    .iter()
                    .map(|v| {
                        projection::project_world_point_unclipped(
                            camera,
                            [v.position[0], v.position[1], v.position[2]],
                            frame.size,
                        )
                    })
                    .collect::<Option<Vec<_>>>();
                let Some(projected) = projected else { continue };
                if source.indices.chunks_exact(3).any(|t| {
                    let a = projected[t[0] as usize];
                    let b = projected[t[1] as usize];
                    let c = projected[t[2] as usize];
                    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x) < 0.
                }) {
                    continue;
                }
            }
            let mut vertices = Vec::new();
            let mut indices = Vec::new();
            for triangle in source.indices.chunks_exact(3) {
                let polygon = triangle
                    .iter()
                    .map(|i| {
                        let vertex = &source.vertices[*i as usize];
                        Corner {
                            position: Vec3::new(
                                vertex.position[0],
                                vertex.position[1],
                                vertex.position[2],
                            ),
                            uv: vertex.uv,
                            color: vertex.color,
                        }
                    })
                    .collect();
                let polygon = if camera.canvas_projection.is_some_and(|l| l.near_clamp) {
                    polygon
                } else {
                    clip_depth(polygon, camera)
                };
                if polygon.len() < 3 {
                    continue;
                }
                let base = vertices.len() as u32;
                let count = polygon.len() as u32;
                for corner in polygon {
                    let p = projection::project_world_point(
                        camera,
                        corner.position.to_array(),
                        frame.size,
                    )
                    .context("legacy mesh projection after depth clipping")?;
                    vertices.push(Vertex {
                        position: [p.x, p.y, p.depth, p.clip_w],
                        uv: corner.uv,
                        color: corner.color,
                    });
                }
                indices.extend((1..count - 1).flat_map(|i| [base, base + i, base + i + 1]));
            }
            if !vertices.is_empty() {
                frame.parts.push(Part {
                    vertices,
                    indices,
                    texture: source.texture,
                    blend: source.blend,
                    add: source.add,
                    linear: source.linear,
                });
            }
        }
        frame.validate()?;
        Ok(frame)
    }
}
pub fn evaluate(
    spec: &Spec,
    matrix: Mat4,
    camera: &SpriteTransform,
    canvas: [u32; 2],
) -> Result<MeshFrame> {
    evaluate_world(spec, matrix, camera, canvas)?.project(camera, canvas)
}

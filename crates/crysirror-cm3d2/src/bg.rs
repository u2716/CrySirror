// SPDX-License-Identifier: MPL-2.0
//! 3D scene content: level/prefab meshes as world entities, and the main
//! camera (`CameraMain` over `UltimateOrbitCamera`).
//!
//! `UltimateOrbitCamera.SetTransform`: rotation = Rotate(0, around.x, 0) in
//! world space then Rotate(around.y, 0, 0) in self space; position =
//! rotation · (0, 0, −distance) + target. Unity space is converted to the
//! engine's right-handed space by mirroring X.
use crate::unity_scene::{Component, ObjectKey, Scene, UnityData, UnityFiles};
use anyhow::{Context, Result};
use crysirror_asset::pbr::{AlphaMode, PbrMaterial};
use crysirror_unity::{
    class_id as c,
    classes::{mesh_filter, shader_name, Material, Mesh, Renderer, Texture2D},
    gltf::{mirror_point, GlbBuilder},
};
use glam::{Quat, Vec3};
use serde_json::{json, Value as Json};
use std::collections::HashMap;

/// Texture2D object of a material property (`None` for built-ins/empty).
fn property_texture_key(data: &mut UnityData, opener: &mut dyn UnityFiles, key: &ObjectKey, m: &Material, name: &str) -> Result<Option<(ObjectKey, Texture2D)>> {
    let Some(env) = m.texture(name) else { return Ok(None) };
    let Some(tk) = data.resolve(opener, &key.file, env.texture)? else { return Ok(None) };
    if tk.file.starts_with("builtin:") {
        return Ok(None);
    }
    let (class, tb) = data.bytes(opener, &tk)?;
    if class != c::TEXTURE_2D {
        return Ok(None);
    }
    let t = Texture2D::read(&tb)?;
    Ok((t.width > 0).then_some((tk, t)))
}

/// Decoded pixels of a material property texture.
fn property_texture(data: &mut UnityData, opener: &mut dyn UnityFiles, key: &ObjectKey, m: &Material, name: &str) -> Result<Option<crysirror_image::ImageSurface>> {
    property_texture_key(data, opener, key, m, name)?.map(|(_, t)| crysirror_unity::texture::decode(&t)).transpose()
}

/// GLB image of a material property texture, shared by object.
fn property_image(data: &mut UnityData, opener: &mut dyn UnityFiles, glb: &mut GlbBuilder, key: &ObjectKey, m: &Material, name: &str) -> Result<Option<usize>> {
    let Some((tk, t)) = property_texture_key(data, opener, key, m, name)? else { return Ok(None) };
    Ok(Some(glb.image(&format!("{}:{}", tk.file, tk.path_id), || {
        let img = crysirror_unity::texture::decode(&t)?;
        Ok((crysirror_image::encode_png(&img)?, "image/png"))
    })?))
}

/// Material of a Unity built-in or game shader, as engine material data.
/// The CM3D2 lit shaders use the engine toon model (`crysirror-asset`
/// `toon_parameters`); SATD-cm3d2-rendering: other shaders are
/// approximated, a lightmapped renderer (`m_LightmapIndex` < 254) draws its
/// albedo unlit.
fn material(data: &mut UnityData, opener: &mut dyn UnityFiles, glb: &mut GlbBuilder, key: &ObjectKey, lightmapped: bool) -> Result<usize> {
    let (class, bytes) = data.bytes(opener, key)?;
    anyhow::ensure!(class == c::MATERIAL, "renderer material is not a Material");
    let m = Material::read(&bytes)?;
    let shader = match data.resolve(opener, &key.file, m.shader)? {
        Some(k) if !k.file.starts_with("builtin:") => shader_name(&data.bytes(opener, &k)?.1).unwrap_or_default(),
        _ => String::new(),
    };
    let mut out = PbrMaterial { roughness_factor: 0.85, specular_factor: 0.3, ..Default::default() };
    let toon = shader.starts_with("CM3D2/Toony_Lighted") || shader.starts_with("CM3D2/Lighted");
    if let Some(c) = m.color("_Color") {
        out.base_color_factor = c.map(|v| v.clamp(0., 1.));
    } else if toon {
        out.base_color_factor = [0.5, 0.5, 0.5, 1.];
    }
    let lower = shader.to_ascii_lowercase();
    out.alpha_mode = if lower.contains("cutout") {
        AlphaMode::Mask
    } else if lower.contains("transparent") || lower.contains("trans") {
        AlphaMode::Blend
    } else {
        AlphaMode::Opaque
    };
    out.unlit = !toon && (lightmapped || lower.contains("unlit") || lower.contains("self-illumin"));
    if let Some(env) = m.texture("_MainTex") {
        if let Some(index) = property_image(data, opener, glb, key, &m, "_MainTex")? {
            out.base_color_texture = Some(index);
            out.uv_scale = env.scale;
            out.uv_offset = [env.offset[0], 1. - env.scale[1] - env.offset[1]];
        }
    }
    if toon {
        let mut t = crysirror_asset::interchange::cm3d2::toon_parameters(&shader, &|n| m.color(n), &|n| m.float(n));
        let toony = shader.starts_with("CM3D2/Toony");
        let ramp = if toony { property_texture(data, opener, key, &m, "_ToonRamp")? } else { None };
        let rate = if toony { property_texture(data, opener, key, &m, "_ShadowRateToon")? } else { None };
        let lut = crysirror_asset::interchange::cm3d2::toon_lookup(ramp.as_ref(), rate.as_ref(), None, !toony);
        let png = crysirror_image::encode_png(&lut)?;
        t.lookup_texture = Some(glb.image(&format!("toon-lookup:{}:{}", key.file, key.path_id), || Ok((png, "image/png")))?);
        if toony {
            t.shadow_texture = property_image(data, opener, glb, key, &m, "_ShadowTex")?;
        }
        if shader.contains("Hair") {
            t.highlight_texture = property_image(data, opener, glb, key, &m, "_HiTex")?;
        }
        out.toon = Some(t);
    } else if !shader.is_empty() && !matches!(lower.as_str(), "diffuse" | "unlit/texture" | "transparent/diffuse" | "transparent/cutout/diffuse") {
        glb.dropped(format!("material {} shader {shader} approximated", m.name));
    }
    Ok(glb.material(&m.name, out))
}

/// Build one GLB of the meshes under `root`; `None` when it has none.
pub fn build(data: &mut UnityData, opener: &mut dyn UnityFiles, scene: &Scene, root: usize) -> Result<Option<Vec<u8>>> {
    let mut glb = GlbBuilder::new("CrySirror CM3D2 Unity scene importer");
    let mut materials: HashMap<(ObjectKey, bool), usize> = HashMap::new();
    let mut stack = vec![(root, None)];
    while let Some((node, parent)) = stack.pop() {
        let n = &scene.nodes[node];
        if !n.active {
            continue;
        }
        let index = glb.node(&n.name, parent, n.position, n.rotation, n.scale)?;
        let filter = n.components.iter().find(|(_, c)| matches!(c, Component::Other { class_id } if *class_id == c::MESH_FILTER));
        let renderer = n.components.iter().find(|(_, c)| matches!(c, Component::Other { class_id } if *class_id == c::MESH_RENDERER));
        if let (Some((fk, _)), Some((rk, _))) = (filter, renderer) {
            let r = Renderer::read(&data.bytes(opener, rk)?.1)?;
            let mesh_ptr = mesh_filter(&data.bytes(opener, fk)?.1)?;
            if r.enabled {
                if let Some(mk) = data.resolve(opener, &fk.file, mesh_ptr)? {
                    if !mk.file.starts_with("builtin:") {
                        let mesh = Mesh::read(&data.bytes(opener, &mk)?.1).with_context(|| format!("mesh of {}", n.name))?;
                        let mut ids = Vec::with_capacity(r.materials.len());
                        for p in &r.materials {
                            let Some(k) = data.resolve(opener, &rk.file, *p)? else { continue };
                            if k.file.starts_with("builtin:") {
                                continue;
                            }
                            let lightmapped = r.lightmap_index < 254;
                            let id = match materials.get(&(k.clone(), lightmapped)) {
                                Some(id) => *id,
                                None => {
                                    let id = material(data, opener, &mut glb, &k, lightmapped)?;
                                    materials.insert((k, lightmapped), id);
                                    id
                                }
                            };
                            ids.push(id);
                        }
                        glb.mesh(index, &mesh, &ids)?;
                    }
                }
            }
        }
        for &child in n.children.iter().rev() {
            stack.push((child, Some(index)));
        }
    }
    if glb.is_empty() {
        return Ok(None);
    }
    Ok(Some(glb.finish()?))
}

/// The engine camera of an orbit camera state (Unity world space input).
pub fn orbit_camera(target: [f32; 3], around: [f32; 2], distance: f32, fov: f32) -> Json {
    orbit_camera_offset(target, around, distance, fov, 0.)
}

/// `UltimateOrbitCamera.LateUpdate` with `offsetTarget.x` screen pixels
/// (of the 1080-high virtual screen): the camera moves sideways so the
/// target projects `offset_px` right of the screen centre.
pub fn orbit_camera_offset(target: [f32; 3], around: [f32; 2], distance: f32, fov: f32, offset_px: f32) -> Json {
    let rotation = Quat::from_rotation_y(around[0].to_radians()) * Quat::from_rotation_x(around[1].to_radians());
    let t = Vec3::from_array(target);
    let mut eye = rotation * Vec3::new(0., 0., -distance) + t;
    let per_px = 2. * distance * (fov.to_radians() / 2.).tan() / crate::ngui::HEIGHT;
    let shift = rotation * Vec3::X * (offset_px * per_px);
    eye -= shift;
    let look = t - shift;
    json!({"camera": {
        "eye": mirror_point(eye.to_array()),
        "target": mirror_point(look.to_array()),
        "up": [0., 1., 0.],
        "fov_y_degrees": fov,
        "near": 0.1,
        "far": 1000.,
    }})
}

// SPDX-License-Identifier: MPL-2.0
//! Export a static project through the shared glTF importer for native GXM diagnosis.
//! No proprietary parser, shader, animation approximation or game bytes are bundled.
use anyhow::{Context, Result, ensure};
use crysirror_asset::pbr::{self, PbrModel};
use crysirror_scene::{model::Scene3d, world::Transform};
use glam::{Mat4, Vec3};
use serde_json::{Value, json};
use std::{fs, io::{BufWriter, Write}, path::Path};

fn words(w: &mut impl Write, values: &[u32]) -> Result<()> {
    for v in values { w.write_all(&v.to_le_bytes())?; } Ok(())
}
fn floats(w: &mut impl Write, values: &[f32]) -> Result<()> {
    ensure!(values.iter().all(|v| v.is_finite()), "non-finite packet value");
    for v in values { w.write_all(&v.to_le_bytes())?; } Ok(())
}
fn scoped(root: &Path, uri: &str) -> Result<Vec<u8>> {
    ensure!(!Path::new(uri).is_absolute() && !uri.contains(":") && !uri.contains("\\"), "external asset URI");
    let path = root.join(uri).canonicalize()?;
    ensure!(path.starts_with(root) && path.metadata()?.len() <= 64*1024*1024, "asset scope/budget");
    Ok(fs::read(path)?)
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    ensure!(args.len()==3, "console_scene PROJECT.json NEW_OUTPUT_DIRECTORY");
    let project = Path::new(&args[1]).canonicalize()?;
    ensure!(project.metadata()?.len() <= 8*1024*1024, "project budget");
    let p: Value = serde_json::from_slice(&fs::read(&project)?)?;
    let scene: Scene3d = serde_json::from_value(p["scene3d"].clone())?;
    scene.validate()?;
    ensure!(scene.meshes.is_empty(), "diagnostic currently requires glTF entity geometry");
    let root = project.parent().context("project parent")?.join(p["assets"].as_str().context("assets")?).canonicalize()?;
    let mut models: Vec<(PbrModel, Mat4)> = Vec::new();
    let mut environment = Value::Null;
    let mut light_count = 0;
    for client in p["clients"].as_array().context("clients")? {
        if let Some(e)=client["systems"].get("environment") { environment=e.clone(); }
        for entity in client["systems"]["entities"].as_array().context("entities")? {
            if entity.get("light").is_some() { light_count+=1; }
            let Some(g)=entity.get("geometry") else {continue};
            ensure!(g["kind"]=="scene" && entity.get("parent").is_none(), "unsupported diagnostic entity geometry/parent");
            let asset=g["asset"].as_str().context("scene asset")?;
            let file=root.join(asset).canonicalize()?;
            ensure!(file.starts_with(&root), "model escapes assets");
            let model=pbr::load_pbr(&fs::read(&file)?, |uri| scoped(file.parent().context("model parent")?,uri))?;
            ensure!(model.skins.is_empty() && model.animations.is_empty() && model.meshes.iter().flatten().all(|p|p.skin_attrs.is_none()&&p.morphs.is_empty()), "static diagnostic does not evaluate skinning/animation/morphs");
            let transform: Transform=entity.get("transform").map(|v|serde_json::from_value(v.clone())).transpose()?.unwrap_or_default();
            transform.validate()?;
            models.push((model,transform.matrix()));
        }
    }
    let (mut textures,mut materials,mut draws,mut vertices,mut indices,mut tex_bytes)=(0,0,0,0,0,0usize);
    for (model,_) in &models {
        textures+=model.images.len();materials+=model.materials.len();
        tex_bytes+=model.images.iter().map(|i|i.levels[0].len()).sum::<usize>();
        for node in &model.nodes {if let Some(mesh)=node.mesh {
            for primitive in &model.meshes[mesh] { draws+=1;vertices+=primitive.vertices.len();indices+=primitive.indices.len(); }
        }}
    }
    ensure!(textures<=1024 && materials<=4096 && draws<=4096 && tex_bytes<=128*1024*1024 && vertices<=2_000_000 && indices<=6_000_000, "native diagnostic packet budget");
    ensure!(models.iter().flat_map(|(m,_)|m.materials.iter()).all(|m|m.effect.is_none()&&m.depth_fade==0.),"unsupported diagnostic material hook/depth fade");
    let out=Path::new(&args[2]);fs::create_dir_all(out)?;
    let mut w=BufWriter::new(fs::OpenOptions::new().write(true).create_new(true).open(out.join("scene.bin"))?);
    words(&mut w,&[u32::from_le_bytes(*b"CGS3"),1,480,272,textures as u32,materials as u32,draws as u32])?;
    let c=&scene.camera;let view=Mat4::look_at_rh(c.eye.into(),c.target.into(),c.up.into());
    let vp=Mat4::perspective_rh(c.fov_y_degrees.to_radians(),480./272.,c.near,c.far)*view;
    floats(&mut w,&vp.to_cols_array())?;floats(&mut w,&view.to_cols_array())?;
    floats(&mut w,&[c.eye[0],c.eye[1],c.eye[2],1./64.])?;
    let dir: [f32;3]=serde_json::from_value(environment["sun"]["direction"].clone())?;
    let color: [f32;3]=serde_json::from_value(environment["sun"]["color"].clone())?;
    let dir=Vec3::from_array(dir).normalize();
    floats(&mut w,&[dir.x,dir.y,dir.z,environment["sun"]["intensity"].as_f64().unwrap_or(1.) as f32])?;
    floats(&mut w,&[color[0],color[1],color[2],0.])?;
    floats(&mut w,&[environment["intensity"].as_f64().unwrap_or(1.) as f32,0.,0.,0.])?;
    floats(&mut w,&[2f32.powf(environment["exposure"].as_f64().unwrap_or(0.) as f32),if environment["tonemap"]=="aces"{1.}else{0.},0.,0.])?;
    for (m,_) in &models {for i in &m.images {
        ensure!(i.width>0&&i.height>0&&i.width<=2048&&i.height<=2048&&i.levels[0].len()==i.width as usize*i.height as usize*4,"texture budget/layout");
        words(&mut w,&[i.width,i.height,i.srgb as u32])?;w.write_all(&i.levels[0])?;
    }}
    let mut texture_base=0;
    for (m,_) in &models {
        for mat in &m.materials {
            floats(&mut w,&mat.base_color_factor)?;
            floats(&mut w,&[mat.emissive_factor[0],mat.emissive_factor[1],mat.emissive_factor[2],mat.emissive_strength])?;
            floats(&mut w,&[mat.metallic_factor,mat.roughness_factor,mat.normal_scale,mat.occlusion_strength])?;
            floats(&mut w,&[mat.alpha_mode.code(),mat.alpha_cutoff,mat.unlit as u8 as f32,mat.opacity_from_alpha as u8 as f32])?;
            floats(&mut w,&[mat.uv_scale[0],mat.uv_scale[1],mat.uv_offset[0],mat.uv_offset[1]])?;
            floats(&mut w,&[0.,0.,if mat.normal_texture.is_some(){mat.normal_encoding.code()}else{0.},mat.specular_factor])?;
            for image in [mat.base_color_texture,mat.metallic_roughness_texture,mat.normal_texture,mat.occlusion_texture,mat.emissive_texture,mat.opacity_texture,mat.specular_texture] {
                words(&mut w,&[image.map_or(u32::MAX,|i|(texture_base+i) as u32)])?;
            }
            words(&mut w,&[mat.double_sided as u32])?;
        }
        texture_base+=m.images.len();
    }
    let mut material_base=0;
    for (m,entity) in &models {
        let world=m.rest_world();
        for (i,node) in m.nodes.iter().enumerate() {if let Some(mesh)=node.mesh {
            let matrix=*entity*world[i];let normal=matrix.inverse().transpose();
            for primitive in &m.meshes[mesh] {
                words(&mut w,&[(material_base+primitive.material) as u32,primitive.vertices.len() as u32,primitive.indices.len() as u32])?;
                for v in &primitive.vertices {
                    floats(&mut w,&matrix.transform_point3(v.position.into()).to_array())?;
                    floats(&mut w,&normal.transform_vector3(v.normal.into()).normalize_or_zero().to_array())?;
                    floats(&mut w,&v.uv)?;
                    let t=matrix.transform_vector3(Vec3::new(v.tangent[0],v.tangent[1],v.tangent[2])).normalize_or_zero();
                    floats(&mut w,&[t.x,t.y,t.z,v.tangent[3]*matrix.determinant().signum()])?;
                }
                ensure!(primitive.indices.iter().all(|i|(*i as usize)<primitive.vertices.len()),"index range");
                words(&mut w,&primitive.indices)?;
            }
        }}
        material_base+=m.materials.len();
    }
    w.flush()?;
    let report=json!({"models":models.len(),"textures":textures,"texture_rgba_bytes":tex_bytes,"materials":materials,"draws":draws,"vertices":vertices,"triangles":indices/3,"packet_bytes":out.join("scene.bin").metadata()?.len(),"camera":c,"import_findings":models.iter().flat_map(|m|m.0.dropped.iter()).collect::<Vec<_>>(),"diagnostic_limits":["static geometry only","base mip only","sun and constant SH ambient only","no shadows, local lights, probes, bloom or temporal effects"],"omitted_local_lights":light_count});
    fs::write(out.join("summary.json"),serde_json::to_vec_pretty(&report)?)?;
    println!("{report}");Ok(())
}

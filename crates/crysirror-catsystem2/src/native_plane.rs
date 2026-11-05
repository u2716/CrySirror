// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! KCS Plane natives 78..94 and 343/345. Each forwards one task message (41..75 in
//! 5E01F0) to the render manager's pose words; here the pose lives on the Plane's
//! shared Layer. No pixels or private composition belong to this module.
use crate::task_policy::NativePlane;
use anyhow::{ensure, Context, Result};
use crysirror_core::{service::{EngineQuery, HostServices}, state::ClientId};
use serde_json::json;

pub(crate) fn publish(host: &mut dyn HostServices, client: ClientId, plane: &NativePlane) -> Result<()> {
    let Some(payload) = plane.payload else { return Ok(()) };
    let pose = &plane.pose;
    let mut layer: crysirror_scene::layer::LayerSpec =
        serde_json::from_slice::<Option<crysirror_scene::layer::LayerSpec>>(&host.query_engine(client, EngineQuery::Layer(payload))?)?
            .context("CatSystem2 Plane payload layer missing")?;
    layer.transform.z = pose.position[2];
    // Zero is a collapsed native quad. The shared Layer omits that subtree
    // until it expands again; an epsilon would expose a tiny picture during
    // scene setup and change the scale/translation animation near zero.
    layer.transform.scale_x = pose.scale[0];
    layer.transform.scale_y = pose.scale[1];
    // 6822D0 uses the supplied matrix instead of Euler rotation, after scale.
    layer.transform.rotate = if pose.matrix.is_some() {0.} else {pose.rotation};
    layer.transform.local_matrix = pose.matrix;
    // The native world base is the point about which scale/rotation acts.
    // rect = base - position, so its local pivot must be position. Leaving
    // the default zero pivot shrinks captured scenes toward their top-left.
    let source_position = plane.source_pose.map_or([0.;3], |p| p.position);
    layer.transform.pivot_x = pose.position[0]+source_position[0];
    layer.transform.pivot_y = pose.position[1]+source_position[1];
    // 85 places the quad's own anchor and 84 the world point it sits on: a 1536-wide
    // background with anchor 768 on base 640 spans -128..1408.
    let rect = [pose.base[0] - pose.position[0] - source_position[0], pose.base[1] - pose.position[1] - source_position[1], pose.size[0].max(1.), pose.size[1].max(1.)];
    let commands = json!([
        {"op":"set_rect","id":payload,"rect":rect},
        {"op":"set_transform","id":payload,"transform":layer.transform},
        {"op":"set_opacity","id":plane.layer,"opacity":pose.alpha},
    ]);
    host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers":commands}))?)?;
    Ok(())
}
/// 63FE70 → message41 → 67A290: an empty render-target quad of the given size.
pub(crate) fn create_target(host: &mut dyn HostServices, client: ClientId, plane: &mut NativePlane, width: f32, height: f32) -> Result<()> {
    let payload = plane.layer + 1;
    let mut commands = Vec::new();
    if plane.payload.is_some() {
        commands.push(json!({"op":"remove","id":payload}));
    }
    commands.push(json!({"op":"put","layer":{"id":payload,"parent":plane.layer,"rect":[0.,0.,width.max(1.),height.max(1.)],"paint":{"kind":"solid","color":[0,0,0,0]}}}));
    host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers":commands}))?)?;
    plane.payload = Some(payload);
    plane.pose.size = [width, height];
    plane.pose.matrix = None;
    plane.source_pose = Some(crate::task_policy::PlanePose { size: [width,height], ..Default::default() });
    Ok(())
}
pub(crate) fn remove_payload(host: &mut dyn HostServices, client: ClientId, plane: &mut NativePlane) -> Result<()> {
    if plane.payload.take().is_some() {
        host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers":[{"op":"remove","id":plane.layer+1}]}))?)?;
    }
    plane.pose.size = [0.; 2];
    plane.source_pose = None;
    Ok(())
}

/// Source mesh coordinates select VRAM; they are not the screen mesh's base.
/// 6824D0 transforms source vertices, then 682BC0 subtracts the source base
/// from the generated screen geometry. Canvas owns the actual atlas copy.
#[cfg(feature="runtime")]
pub(crate) fn publish_source(host:&mut dyn HostServices, client:ClientId, tasks:&mut crate::task_policy::NativeTaskPolicy, id:u32)->Result<()> {
    let plane=tasks.plane(id)?.clone();
    let (Some(source),Some(payload),Some(list))=(plane.source_pose,plane.payload,tasks.main_image_list()) else {return Ok(())};
    // Scripts reset unused slots with a zero-sized target before writing the
    // source base. There is no atlas coverage to publish for that object.
    if source.size.iter().any(|v|*v<1.) {return publish(host,client,&plane);}
    let matrix_identity=source.matrix.is_none_or(|m|m==glam::Mat4::IDENTITY.to_cols_array());
    ensure!(source.scale==[1.;3] && (source.rotation==0.||source.matrix.is_some()) && matrix_identity, "CatSystem2 transformed source mesh requires an affine atlas view");
    let origin=[source.base[0]-source.position[0],source.base[1]-source.position[1]];
    ensure!(origin.iter().all(|v|v.is_finite()&&v.fract()==0.&&v.abs()<=1_000_000.),"CatSystem2 nonintegral source mesh atlas origin");
    let publication=tasks.list_publication(list,client)?;
    publication.plane(crate::native_image_publication::Plane {
        canvas:0x5c000000u32.checked_add(id).context("CatSystem2 Plane Canvas range")?,
        presentation:0x10000000u32.checked_add(id).context("CatSystem2 presentation ID range")?,
        parent:payload,source:[origin[0] as i32,origin[1] as i32,source.size[0] as i32,source.size[1] as i32],
    })?;
    publication.publish(host,client,Vec::new())?;
    publish(host,client,&plane)
}

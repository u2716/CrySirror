// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! KCS Plane natives 78..94 and 343/345. Each forwards one task message (41..75 in
//! 5E01F0) to the render manager's pose words; here the pose lives on the Plane's
//! shared Layer. No pixels or private composition belong to this module.
use crate::task_policy::NativePlane;
use anyhow::{Context, Result};
use crysirror_core::{service::{EngineQuery, HostServices}, state::ClientId};
use serde_json::json;

pub(crate) fn publish(host: &mut dyn HostServices, client: ClientId, plane: &NativePlane) -> Result<()> {
    let Some(payload) = plane.payload else { return Ok(()) };
    let pose = &plane.pose;
    let mut layer: crysirror_scene::layer::LayerSpec =
        serde_json::from_slice::<Option<crysirror_scene::layer::LayerSpec>>(&host.query_engine(client, EngineQuery::Layer(payload))?)?
            .context("CatSystem2 Plane payload layer missing")?;
    layer.transform.z = pose.position[2];
    // A script may scale to nothing; the shared transform needs an invertible matrix.
    let invertible = |scale: f32| if scale.abs() < 1e-4 { 1e-4f32.copysign(scale) } else { scale };
    layer.transform.scale_x = invertible(pose.scale[0]);
    layer.transform.scale_y = invertible(pose.scale[1]);
    layer.transform.rotate = pose.rotation;
    // 85 places the quad's own anchor and 84 the world point it sits on: a 1536-wide
    // background with anchor 768 on base 640 spans -128..1408.
    let rect = [pose.base[0] - pose.position[0], pose.base[1] - pose.position[1], pose.size[0].max(1.), pose.size[1].max(1.)];
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
    Ok(())
}
pub(crate) fn remove_payload(host: &mut dyn HostServices, client: ClientId, plane: &mut NativePlane) -> Result<()> {
    if plane.payload.take().is_some() {
        host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers":[{"op":"remove","id":plane.layer+1}]}))?)?;
    }
    plane.pose.size = [0.; 2];
    Ok(())
}

//! Validate an immutable, client-scoped Layer candidate without changing the
//! running scene. The existing presenter and image workers own rasterization.
use crysirror_core::{Error, Result, state::ClientId};
use crysirror_scene::{
    canvas::MaterializeRequest,
    layer::{LayerCommand, LayerSystem},
};
use std::sync::Arc;

pub(crate) fn request(
    client: ClientId,
    size: [u32; 2],
    order_exclusive: [i32; 2],
    commands: &[u8],
) -> Result<MaterializeRequest> {
    if commands.len() > 4 * 1024 * 1024 || size.iter().any(|v| *v == 0 || *v > 8192) {
        return Err(Error::Limit("detached Layer scene size"));
    }
    let commands: Vec<LayerCommand> =
        serde_json::from_slice(commands).map_err(|e| Error::Io(e.to_string()))?;
    if commands.len() > 8192 {
        return Err(Error::Limit("detached Layer commands"));
    }
    if !commands
        .iter()
        .all(|c| matches!(c, LayerCommand::Put { .. } | LayerCommand::Remove { .. }))
    {
        return Err(Error::Unsupported(
            "detached Layer scene requires declarative put/remove",
        ));
    }
    let mut scene = LayerSystem::default();
    scene
        .apply_batch(client, &commands)
        .map_err(|e| Error::Io(e.to_string()))?;
    let mut layers = scene
        .prefix_snapshot_specs(client, order_exclusive)
        .map_err(|e| Error::Io(e.to_string()))?;
    // An explicit full-size identity root keeps single, offset top-level nodes
    // in capture coordinates, too. Never borrow a root from the live scene.
    let root = (0..=layers.len() as u32)
        .map(|n| u32::MAX - n)
        .find(|id| layers.iter().all(|l| l.id != *id))
        .ok_or(Error::Limit("detached Layer root id"))?;
    for layer in layers.iter_mut().filter(|l| l.parent.is_none()) {
        layer.parent = Some(root);
    }
    layers.insert(
        0,
        serde_json::from_value(serde_json::json!({
            "id":root, "rect":[0,0,size[0],size[1]], "paint":{"kind":"group"}
        }))
        .map_err(|e| Error::Io(e.to_string()))?,
    );
    Ok(MaterializeRequest {
        client: client.0,
        surface: 0,
        size,
        layers: Arc::new(layers),
    })
}

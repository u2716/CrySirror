//! Scoped immutable scene capabilities; no runtime recursion or device state.
use crate::runtime::SharedHost;
use crysirror_core::{
    Error, Result,
    scene_checkpoint::{SceneCheckpoint, SceneCheckpointScope},
    state::ClientId,
};
use std::{collections::BTreeSet, sync::Arc};
struct Capture {
    owner: Arc<()>,
    client: ClientId,
    epoch: Arc<()>,
    scene: crysirror_scene::checkpoint::ClientSceneCheckpoint,
    _images: crate::published_images::CheckpointPins,
    _fonts: Vec<crysirror_text::TextSystem>,
}
impl SceneCheckpoint for Capture {
    fn client(&self) -> ClientId {
        self.client
    }
    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
}
pub(crate) fn capture(
    host: &mut SharedHost,
    client: ClientId,
    scope: &SceneCheckpointScope,
) -> Result<Arc<dyn SceneCheckpoint>> {
    let scene =
        crysirror_scene::checkpoint::ClientSceneCheckpoint::capture(&host.engine, client, scope)
            .map_err(|e| Error::Io(format!("{e:#}")))?;
    if scope.generated_images.len() > 4096 {
        return Err(Error::Limit("scene checkpoint image references"));
    }
    let mut paths = BTreeSet::new();
    for path in &scope.generated_images {
        if !(path.starts_with("__image_resource/") || path.starts_with("__crysirror/")) {
            return Err(Error::Invalid("scene checkpoint generated image path"));
        }
        paths.insert(path.clone());
    }
    let complete = scene.visit_image_dependencies(&mut |owner, path| {
        if owner == client
            && (path.starts_with("__image_resource/") || path.starts_with("__crysirror/"))
        {
            paths.insert(path.to_owned());
        }
    });
    if !complete {
        return Err(Error::Unsupported(
            "scene checkpoint dynamic image dependencies",
        ));
    }
    let pins = host.published_images.pin_checkpoint(client, &paths)?;
    if scope.font_aliases.len() > 4096 {
        return Err(Error::Limit("scene checkpoint fonts"));
    }
    let mut fonts = scope.font_aliases.iter().cloned().collect::<BTreeSet<_>>();
    scene.visit_font_dependencies(&mut |owner, alias| {
        if owner == client {
            fonts.insert(alias.to_owned());
        }
    });
    if fonts.len() > 4096 {
        return Err(Error::Limit("scene checkpoint fonts"));
    }
    let text = host.text.for_client(client);
    let fonts = fonts
        .into_iter()
        .map(|alias| {
            text.frozen_font(&alias)
                .map_err(|error| Error::Io(format!("scene checkpoint font {alias}: {error:#}")))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Arc::new(Capture {
        owner: host.pattern_owner.clone(),
        client,
        epoch: host
            .scene_checkpoint_epochs
            .entry(client)
            .or_insert_with(|| Arc::new(()))
            .clone(),
        scene,
        _images: pins,
        _fonts: fonts,
    }))
}
pub(crate) fn restore(
    host: &mut SharedHost,
    client: ClientId,
    checkpoint: &dyn SceneCheckpoint,
    remove: &SceneCheckpointScope,
) -> Result<()> {
    let capture = checkpoint
        .as_any()
        .downcast_ref::<Capture>()
        .ok_or(Error::Invalid("foreign scene checkpoint backend"))?;
    if client != capture.client
        || checkpoint.client() != client
        || !Arc::ptr_eq(&host.pattern_owner, &capture.owner)
        || host
            .scene_checkpoint_epochs
            .get(&client)
            .is_none_or(|epoch| !Arc::ptr_eq(epoch, &capture.epoch))
    {
        return Err(Error::Invalid("foreign scene checkpoint authority"));
    }
    capture
        .scene
        .restore(&mut host.engine, client, remove)
        .map_err(|e| Error::Io(format!("{e:#}")))
}

//! Device-free scoped scene state. This captures real retained clocks, not a
//! screenshot or a declaration reconstructed with restarted durations.
use crate::{EngineSystems, canvas::CanvasSystem, layer::LayerSystem, ui::UiSystem};
use anyhow::{Result, ensure};
use crysirror_core::{scene_checkpoint::SceneCheckpointScope, state::ClientId};
use std::collections::BTreeSet;
#[derive(Clone)]
pub struct ClientSceneCheckpoint {
    client: ClientId,
    scope: SceneCheckpointScope,
    layers: LayerSystem,
    canvases: CanvasSystem,
    ui: UiSystem,
}
impl ClientSceneCheckpoint {
    pub fn capture(
        engine: &EngineSystems,
        client: ClientId,
        scope: &SceneCheckpointScope,
    ) -> Result<Self> {
        ensure!(
            scope.layer_roots.len() <= 16384
                && scope.canvases.len() <= 65536
                && scope.widget_roots.len() <= 4096,
            "scene checkpoint selection budget"
        );
        Ok(Self {
            client,
            scope: scope.clone(),
            layers: engine
                .layers
                .checkpoint_scope(client.0, &scope.layer_roots)?,
            canvases: engine
                .canvases
                .checkpoint_scope(client.0, &scope.canvases.iter().copied().collect())?,
            ui: engine.ui.checkpoint_scope(client.0, &scope.widget_roots)?,
        })
    }
    pub fn restore(
        &self,
        engine: &mut EngineSystems,
        client: ClientId,
        remove: &SceneCheckpointScope,
    ) -> Result<()> {
        ensure!(client == self.client, "foreign scene checkpoint owner");
        // A captured subtree is the replacement authority. The caller may
        // replace its current descendants, including newly created children,
        // but cannot add another root/canvas to retire a peer scope.
        ensure!(
            remove
                .layer_roots
                .iter()
                .all(|id| self.scope.layer_roots.contains(id))
                && remove
                    .widget_roots
                    .iter()
                    .all(|id| self.scope.widget_roots.contains(id))
                && remove
                    .canvases
                    .iter()
                    .all(|id| self.scope.canvases.contains(id)),
            "scene checkpoint removal exceeds captured scope"
        );
        let mut staged = engine.clone();
        let roots = remove
            .layer_roots
            .iter()
            .chain(&self.scope.layer_roots)
            .copied()
            .collect::<Vec<_>>();
        let widgets = remove
            .widget_roots
            .iter()
            .chain(&self.scope.widget_roots)
            .copied()
            .collect::<Vec<_>>();
        let canvases = remove
            .canvases
            .iter()
            .chain(&self.scope.canvases)
            .copied()
            .collect::<BTreeSet<_>>();
        staged
            .layers
            .restore_checkpoint_scope(client.0, &roots, &self.layers)?;
        staged
            .canvases
            .restore_checkpoint_scope(client.0, &canvases, &self.canvases)?;
        staged
            .ui
            .restore_checkpoint_scope(client.0, &widgets, &self.ui)?;
        staged.validate()?;
        *engine = staged;
        Ok(())
    }
    pub fn visit_font_dependencies(&self, visitor: &mut dyn FnMut(ClientId, &str)) {
        self.layers.visit_font_dependencies(visitor);
        self.canvases.visit_font_dependencies(visitor);
        self.ui.visit_font_dependencies(visitor);
    }
    pub fn visit_image_dependencies(&self, visitor: &mut dyn FnMut(ClientId, &str)) -> bool {
        let complete = self.layers.visit_image_dependencies(visitor);
        self.canvases.visit_image_dependencies(visitor);
        complete
    }
}

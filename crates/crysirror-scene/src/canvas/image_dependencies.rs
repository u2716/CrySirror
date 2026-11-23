// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
use super::*;
impl CanvasSystem {
    /// Include unpresented surfaces and pending compactions, not only the
    /// materialized/presented layer which may already contain raster pixels.
    pub fn visit_font_dependencies(&self, visitor: &mut dyn FnMut(ClientId, &str)) {
        fn nodes(items: &[CanvasNode], visitor: &mut dyn FnMut(&str)) {
            for node in items {
                match node {
                    CanvasNode::Draw { layer } => layer.visit_font_dependencies(visitor),
                    CanvasNode::Group {
                        nodes: children, ..
                    } => nodes(children, visitor),
                }
            }
        }
        for (client, surfaces) in &self.clients {
            for surface in surfaces.values() {
                nodes(&surface.nodes, &mut |font| visitor(ClientId(*client), font));
            }
        }
        for request in self.materialize.values() {
            for layer in request.layers.iter() {
                layer.visit_font_dependencies(&mut |font| visitor(ClientId(request.client), font));
            }
        }
    }
    /// Include unpresented surfaces and pending compacted generations: both may
    /// be copied or materialized after the caller releases its image handle.
    pub fn visit_image_dependencies(&self, visitor: &mut dyn FnMut(ClientId, &str)) {
        fn nodes(items: &[CanvasNode], visitor: &mut dyn FnMut(&str)) {
            for node in items {
                match node {
                    CanvasNode::Draw { layer } => layer.visit_image_dependencies(visitor),
                    CanvasNode::Group {
                        nodes: children, ..
                    } => nodes(children, visitor),
                }
            }
        }
        for (client, surfaces) in &self.clients {
            for surface in surfaces.values() {
                nodes(&surface.nodes, &mut |asset| {
                    visitor(ClientId(*client), asset)
                });
            }
        }
        for request in self.materialize.values() {
            for layer in request.layers.iter() {
                layer.visit_image_dependencies(&mut |asset| {
                    visitor(ClientId(request.client), asset)
                });
            }
        }
    }
}

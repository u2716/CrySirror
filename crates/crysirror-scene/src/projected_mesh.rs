// SPDX-License-Identifier: MPL-2.0
//! Immutable projected geometry for the ordered Layer compositor. Projection
//! and shading producers run in shared CPU services, without GPU handles.
use crate::layer::Blend;
use anyhow::{Result, ensure};
use crysirror_core::path::{Mount, VfsPath};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeshFrame {
    /// Local projected canvas dimensions. Layer quads map this plane into
    /// the destination; individual vertex depth and clip W remain intact.
    pub size: [f32; 2],
    pub parts: Vec<Part>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vertex {
    /// X/Y in local projected pixels, NDC depth, and homogeneous clip W.
    pub position: [f32; 4],
    pub uv: [f32; 2],
    /// Linear RGBA, before retained layer tint/opacity.
    pub color: [f32; 4],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Part {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    /// None samples opaque white. Paths remain client scoped and immutable.
    #[serde(default)]
    pub texture: Option<String>,
    /// None inherits the retained layer blend.
    #[serde(default)]
    pub blend: Option<Blend>,
    /// Gamma-space RGB addition, composed with the layer's filter.
    #[serde(default)]
    pub add: [f32; 3],
    #[serde(default = "linear")]
    pub linear: bool,
}
fn linear() -> bool {
    true
}
impl MeshFrame {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.size
                .iter()
                .all(|v| v.is_finite() && *v > 0. && *v <= 16384.),
            "projected mesh dimensions"
        );
        ensure!(self.parts.len() <= 65536, "projected mesh part budget");
        let mut vertices = 0usize;
        let mut indices = 0usize;
        for part in &self.parts {
            vertices = vertices
                .checked_add(part.vertices.len())
                .ok_or_else(|| anyhow::anyhow!("projected mesh vertex overflow"))?;
            indices = indices
                .checked_add(part.indices.len())
                .ok_or_else(|| anyhow::anyhow!("projected mesh index overflow"))?;
            ensure!(
                vertices <= 1_000_000 && indices <= 3_000_000,
                "projected mesh geometry budget"
            );
            ensure!(
                part.indices.len() % 3 == 0
                    && part
                        .indices
                        .iter()
                        .all(|i| (*i as usize) < part.vertices.len()),
                "projected mesh triangle indices"
            );
            ensure!(
                part.add.iter().all(|v| v.is_finite() && v.abs() <= 1.),
                "projected mesh additive color"
            );
            for vertex in &part.vertices {
                ensure!(
                    vertex
                        .position
                        .iter()
                        .chain(&vertex.uv)
                        .chain(&vertex.color)
                        .all(|v| v.is_finite())
                        && vertex.position[3].abs() >= 1e-20,
                    "projected mesh vertex attributes"
                );
            }
            if let Some(texture) = &part.texture {
                VfsPath::new(Mount::Asset, texture).map_err(|e| anyhow::anyhow!("{e}"))?;
                ensure!(
                    !texture.starts_with("__surface/")
                        && crysirror_core::service::frame_history_layer(texture).is_none(),
                    "projected mesh requires immutable image dependencies"
                );
            }
        }
        Ok(())
    }
}

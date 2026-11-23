// SPDX-License-Identifier: MPL-2.0
//! Device-independent bounded geometry for the native software-ray profile.
//!
//! Build on an engine CPU worker, then transfer the immutable result to the
//! renderer owner. A scene generation must retire only after its last GPU fence.
//! Neither VM state nor this crate contains native GPU pointers.
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;
use alloc::vec::Vec;

pub const MAX_TRIANGLES: usize = 1024;
pub const LEAF_TRIANGLES: usize = 4;
pub const MAX_COORDINATE: f32 = 1_000_000.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Triangle {
    pub vertices: [[f32; 3]; 3],
    /// Flat opaque linear RGB. Textured/alpha-tested surfaces are outside v1.
    pub color: [f32; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildError {
    TooManyTriangles,
    InvalidCoordinate,
    InvalidColor,
    InvalidTextureWidth,
}

#[derive(Clone, Copy, Debug)]
struct Node {
    min: [f32; 3],
    max: [f32; 3],
    escape: usize,
    first: usize,
    count: usize,
}

/// Row-major logical RG32F texture. Native hosts must select RG channel order,
/// nearest sampling, clamp addressing, no mips, no gamma, and native row padding.
/// The logical texels here do not include that device-specific row padding.
#[derive(Debug)]
pub struct DataTexture {
    width: u32,
    height: u32,
    texels: Vec<[f32; 2]>,
}

impl DataTexture {
    pub fn width(&self) -> u32 { self.width }
    pub fn height(&self) -> u32 { self.height }
    pub fn texels(&self) -> &[[f32; 2]] { &self.texels }
}

#[derive(Debug)]
pub struct PackedScene {
    nodes: DataTexture,
    triangles: DataTexture,
    node_count: u32,
    triangle_count: u32,
}

impl PackedScene {
    pub fn nodes(&self) -> &DataTexture { &self.nodes }
    pub fn triangles(&self) -> &DataTexture { &self.triangles }
    pub fn node_count(&self) -> u32 { self.node_count }
    pub fn triangle_count(&self) -> u32 { self.triangle_count }
}

/// v1: four RG32F texels per preorder node, six per reordered triangle.
/// Internal nodes descend to index+1. A miss or completed leaf follows escape.
/// Each traversal visits at most node_count nodes, without a GPU stack or a
/// truncated visit budget. Empty scenes use zero counts and one zero texture row.
pub fn build(triangles: &[Triangle], texture_width: u32) -> Result<PackedScene, BuildError> {
    if triangles.len() > MAX_TRIANGLES {
        return Err(BuildError::TooManyTriangles);
    }
    if texture_width == 0 || texture_width > 1024 || !texture_width.is_power_of_two() {
        return Err(BuildError::InvalidTextureWidth);
    }
    for triangle in triangles {
        if triangle.vertices.iter().flatten().any(|v| !v.is_finite() || v.abs() > MAX_COORDINATE) {
            return Err(BuildError::InvalidCoordinate);
        }
        if triangle.color.iter().any(|v| !v.is_finite() || *v < 0.0 || *v > 1.0) {
            return Err(BuildError::InvalidColor);
        }
    }
    let mut ordered = triangles.to_vec();
    let mut nodes = Vec::new();
    if !ordered.is_empty() {
        partition(&mut ordered, 0, &mut nodes);
    }
    let mut node_data = Vec::with_capacity(nodes.len() * 4);
    for node in &nodes {
        // first*5 + count is exact in f32 for the bounded v1 profile.
        node_data.extend_from_slice(&[
            [node.min[0], node.min[1]],
            [node.min[2], node.escape as f32],
            [node.max[0], node.max[1]],
            [node.max[2], (node.first * 5 + node.count) as f32],
        ]);
    }
    let mut triangle_data = Vec::with_capacity(ordered.len() * 6);
    for triangle in &ordered {
        for (vertex, color) in triangle.vertices.iter().zip(triangle.color) {
            triangle_data.extend_from_slice(&[[vertex[0], vertex[1]], [vertex[2], color]]);
        }
    }
    Ok(PackedScene {
        nodes: texture(node_data, texture_width),
        triangles: texture(triangle_data, texture_width),
        node_count: nodes.len() as u32,
        triangle_count: ordered.len() as u32,
    })
}

fn texture(mut texels: Vec<[f32; 2]>, width: u32) -> DataTexture {
    let height = texels.len().div_ceil(width as usize).max(1) as u32;
    texels.resize(width as usize * height as usize, [0.0; 2]);
    DataTexture { width, height, texels }
}

fn partition(triangles: &mut [Triangle], first: usize, nodes: &mut Vec<Node>) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for triangle in triangles.iter() {
        for vertex in triangle.vertices {
            for axis in 0..3 {
                min[axis] = min[axis].min(vertex[axis]);
                max[axis] = max[axis].max(vertex[axis]);
            }
        }
    }
    let index = nodes.len();
    nodes.push(Node { min, max, escape: 0, first: 0, count: 0 });
    if triangles.len() <= LEAF_TRIANGLES {
        nodes[index].first = first;
        nodes[index].count = triangles.len();
    } else {
        let axis = (0..3).max_by(|a, b| (max[*a] - min[*a]).total_cmp(&(max[*b] - min[*b]))).unwrap();
        let middle = triangles.len() / 2;
        let centroid = |t: &Triangle| t.vertices.iter().map(|v| v[axis]).sum::<f32>();
        triangles.select_nth_unstable_by(middle, |a, b| centroid(a).total_cmp(&centroid(b)));
        let (left, right) = triangles.split_at_mut(middle);
        partition(left, first, nodes);
        partition(right, first + middle, nodes);
    }
    nodes[index].escape = nodes.len();
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn triangle(x: f32) -> Triangle {
        Triangle { vertices: [[x, 0.0, 0.0], [x + 1.0, 0.0, 0.0], [x, 1.0, 0.0]], color: [0.2, 0.4, 0.6] }
    }

    #[test]
    fn every_leaf_is_reachable_and_escape_links_terminate() {
        let input = (0..37).map(|i| triangle(i as f32)).collect::<Vec<_>>();
        let scene = build(&input, 16).unwrap();
        let mut seen = vec![false; input.len()];
        let mut index = 0;
        let mut visits = 0;
        while index < scene.node_count as usize {
            let escape = scene.nodes.texels[index * 4 + 1][1] as usize;
            assert!(escape > index && escape <= scene.node_count as usize);
            let packed = scene.nodes.texels[index * 4 + 3][1] as usize;
            let count = packed % 5;
            if count == 0 {
                index += 1;
            } else {
                let first = packed / 5;
                for item in &mut seen[first..first + count] {
                    assert!(!*item);
                    *item = true;
                }
                index = escape;
            }
            visits += 1;
            assert!(visits <= scene.node_count);
        }
        assert!(seen.into_iter().all(|v| v));
    }

    #[test]
    fn rejects_unrepresentable_inputs_without_truncation() {
        assert_eq!(build(&vec![triangle(0.0); MAX_TRIANGLES + 1], 16).unwrap_err(), BuildError::TooManyTriangles);
        assert_eq!(build(&[triangle(f32::NAN)], 16).unwrap_err(), BuildError::InvalidCoordinate);
        assert_eq!(build(&[], 3).unwrap_err(), BuildError::InvalidTextureWidth);
        let empty = build(&[], 16).unwrap();
        assert_eq!(empty.node_count, 0);
        assert_eq!(empty.nodes.height, 1);
    }
}

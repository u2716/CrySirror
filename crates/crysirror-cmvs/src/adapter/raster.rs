// SPDX-License-Identifier: MPL-2.0
//! Object rasters `0x2328`/`0x2329` (0x14004f5d0, 0x14004fb60, 0x14004fb40) and the
//! object point lists `0x2330..=0x2332` (+8248, 0x140051090..0x1400510e0) that type 6
//! effects collide with. While a raster is active the object draws only its raster rows.

use serde::{Deserialize, Serialize};

/// Record +8300..+8336 as the script set them, and the running phase.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Raster {
    /// 1: one-pixel rows stretched about a pivot that swings along a cosine;
    /// 16: echoes at `spacing` steps either side with falling opacity.
    pub mode: u32,
    /// Words 3..10 of `0x2328` (+8304..+8332).
    pub words: [u32; 8],
    /// +8336, milliseconds advanced by every frame while the mode is 1.
    pub phase: u32,
    /// Crop width and height of the parent when the raster started.
    pub size: [i32; 2],
}

/// One raster child: crop, origin, position, scale, opacity and order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Row {
    pub crop: [i16; 4],
    pub origin: [i16; 2],
    pub position: [i32; 2],
    pub scale: [f32; 2],
    pub opacity: i32,
    pub order: i32,
}

/// Parent record fields the rows derive from.
#[derive(Clone, Copy)]
pub(super) struct Parent {
    pub origin: [i16; 2],
    pub position: [i32; 2],
    pub opacity: i32,
    pub order: i32,
}

impl Raster {
    pub fn new(mode: u32, words: [u32; 8], size: [i32; 2]) -> Self {
        Self { mode, words, phase: 0, size }
    }

    fn word(&self, index: usize) -> i32 {
        self.words[index - 3] as i32
    }

    /// Hundredths of a degree for `row` (integer arithmetic of 0x14004fb60). A zero
    /// divisor faults in the original; it reads as angle 0 here.
    fn angle(&self, row: i32) -> i32 {
        let (step, divisor, period) = (self.word(9), self.word(6), self.word(10));
        if divisor == 0 || period == 0 {
            return 0;
        }
        let offset = (self.phase as i32).wrapping_add(row.wrapping_mul(step) / divisor);
        36_000i32.wrapping_mul(offset % period) / period
    }

    pub fn rows(&self, parent: Parent) -> Vec<Row> {
        let [width, height] = self.size;
        match self.mode {
            1 => {
                let half = width / 2;
                let scale = [self.word(7) as f32 / 1000.0, 1.0];
                (0..height.max(0))
                    .map(|row| {
                        // 0x14001f8c0: (int)(cos(angle) * radius) with the angle in hundredths.
                        let angle = self.angle(row);
                        let radians = if angle == 0 { 0.0 } else { f64::from(angle) / 18_000.0 * std::f64::consts::PI };
                        let swing = (radians.cos() * f64::from(half)) as i32;
                        Row {
                            crop: [0, row as i16, width as i16, 1],
                            origin: [(half + swing + i32::from(parent.origin[0])) as i16, parent.origin[1]],
                            position: [parent.position[0] + swing + half, row + parent.position[1]],
                            scale,
                            opacity: parent.opacity,
                            order: parent.order,
                        }
                    })
                    .collect()
            }
            16 => {
                let spacing = self.word(6);
                let mut rows = Vec::new();
                for copy in 0..self.word(7).max(0) {
                    let row = Row {
                        crop: [0, 0, width as i16, height as i16],
                        origin: parent.origin,
                        position: [parent.position[0] + spacing * copy, parent.position[1]],
                        scale: [1.0; 2],
                        opacity: 255 / (copy + 1),
                        order: parent.order - copy,
                    };
                    rows.push(row);
                    if copy != 0 {
                        rows.push(Row { position: [parent.position[0] - spacing * copy, parent.position[1]], ..row });
                    }
                }
                rows
            }
            _ => Vec::new(),
        }
    }

    /// Mode 1 swings with time; other modes keep the rows they started with.
    pub fn advance(&mut self, delta: u32) -> bool {
        if self.mode == 1 {
            self.phase = self.phase.wrapping_add(delta);
            true
        } else {
            false
        }
    }
}

/// `0x2330..=0x2332`: at most 128 points; consecutive triples are triangles.
pub(super) const POINT_LIMIT: usize = 128;

/// 0x14005f960: the first triangle a vertical ray from height 35 at the particle's
/// x/z crosses, as the hit point. Triangles cover consecutive point triples.
pub(super) fn ground(points: &[[f32; 3]], at: [f32; 3]) -> Option<[f32; 3]> {
    fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
    }
    fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [b[2] * a[1] - a[2] * b[1], a[2] * b[0] - b[2] * a[0], a[0] * b[1] - b[0] * a[1]]
    }
    fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
        a[1] * b[1] + a[0] * b[0] + a[2] * b[2]
    }
    let origin = [at[0], 35.0, at[2]];
    let direction = [0.0, 1.0, 0.0];
    for triangle in points.chunks_exact(3) {
        let [a, b, c] = [triangle[0], triangle[1], triangle[2]];
        let normal = cross(sub(c, a), sub(b, a));
        let length = dot(normal, normal).sqrt();
        let normal = if length > 0.0 { normal.map(|axis| axis / length) } else { normal };
        let t = dot(sub(a, origin), normal) / dot(direction, normal);
        let hit = [direction[0] * t + origin[0], direction[1] * t + origin[1], direction[2] * t + origin[2]];
        let inside = [(a, b), (b, c), (c, a)].iter().all(|&(from, to)| dot(cross(sub(hit, from), sub(to, from)), normal) >= 0.0);
        if inside {
            return Some(hit);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echoes_spread_both_ways_with_falling_opacity() {
        let raster = Raster::new(16, [0, 0, 0, 10, 3, 0, 0, 0], [40, 20]);
        let parent = Parent { origin: [0; 2], position: [100, 50], opacity: 255, order: 9 };
        let rows = raster.rows(parent);
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[1].position, [110, 50]);
        assert_eq!(rows[2].position, [90, 50]);
        assert_eq!((rows[3].opacity, rows[3].order), (85, 7));
    }

    #[test]
    fn rows_keep_their_unscaled_placement() {
        let raster = Raster::new(1, [0, 0, 0, 1, 1000, 0, 1, 360], [64, 4]);
        let parent = Parent { origin: [2, 3], position: [10, 20], opacity: 200, order: 1 };
        for (index, row) in raster.rows(parent).iter().enumerate() {
            assert_eq!(row.position[0] - i32::from(row.origin[0]), 8);
            assert_eq!(row.position[1], 20 + index as i32);
            assert_eq!(row.crop, [0, index as i16, 64, 1]);
        }
    }

    #[test]
    fn a_ray_lands_on_a_floor_triangle() {
        let floor = [[-10.0, 2.0, -10.0], [10.0, 2.0, -10.0], [0.0, 2.0, 10.0]];
        let hit = ground(&floor, [0.0, 30.0, 0.0]).or_else(|| ground(&[floor[0], floor[2], floor[1]], [0.0, 30.0, 0.0]));
        assert_eq!(hit.map(|point| point[1]), Some(2.0));
        assert!(ground(&floor, [50.0, 30.0, 50.0]).is_none());
    }
}

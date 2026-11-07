// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Immutable engine-owned pointer coverage, independent of a VM or renderer.
pub mod affine;
pub mod channel;
pub mod union;
use anyhow::{Result, ensure};
pub use channel::CoverageChannel;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
/// One byte per logical widget pixel. Transparent samples permit pointer input
/// to reach lower widgets; keyboard focus and activation do not sample this mask.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WidgetHitMask {
    /// Width and height of the local, unscaled coverage grid.
    pub size: [u32; 2],
    /// Row-major immutable coverage bytes, shared across staged scene copies.
    pub coverage: Arc<[u8]>,
    /// A pointer is eligible when its coverage is at least this byte.
    pub threshold: u8,
    /// Exclude the local top and left edges as well as the normal right/bottom edges.
    #[serde(default)]
    pub strict_bounds: bool,
    /// Optional inverse affine map from logical widget-local coordinates to
    /// immutable source-coverage pixels as a row-major3x3 homogeneous matrix. Geometry can
    /// change without allocating or resampling the alpha plane.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_from_local: Option<[f32; 9]>,
    /// Additional leaf coverage regions, relative to this widget. A retained
    /// union shares each source plane instead of rasterizing a new bitmap.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regions: Vec<WidgetHitRegion>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WidgetHitRegion {
    pub rect: [f32; 4],
    pub mask: Box<WidgetHitMask>,
}
impl WidgetHitMask {
    pub(crate) fn validate(&self, rect: [f32; 4]) -> Result<()> {
        let [w, h] = self.size;
        ensure!(
            w > 0 && h > 0 && w <= 8192 && h <= 8192,
            "UI hit mask dimensions"
        );
        let len = u64::from(w) * u64::from(h);
        ensure!(
            len <= 16 * 1024 * 1024 && len == self.coverage.len() as u64,
            "UI hit mask coverage size"
        );
        if let Some(matrix) = self.source_from_local {
            ensure!(
                matrix
                    .iter()
                    .all(|value| value.is_finite() && value.abs() <= 1.0e12),
                "UI hit mask inverse affine bounds"
            );
        } else {
            ensure!(
                rect[2].ceil() == w as f32 && rect[3].ceil() == h as f32,
                "UI hit mask must match logical widget extent"
            );
        }
        ensure!(self.regions.len() <= 256, "UI hit region budget");
        for region in &self.regions {
            ensure!(
                crate::layer::rect_valid(region.rect) && region.mask.regions.is_empty(),
                "UI hit unions require finite leaf regions"
            );
            region.mask.validate(region.rect)?;
        }
        Ok(())
    }
    pub fn hit(&self, x: i32, y: i32, rect: [f32; 4]) -> bool {
        let local = [x as f32 - rect[0], y as f32 - rect[1]];
        if local[0] < 0. || local[1] < 0. || local[0] >= rect[2] || local[1] >= rect[3] {
            return false;
        }
        self.hit_base(x, y, rect)
            || self.regions.iter().any(|region| {
                region.mask.hit_base(
                    x,
                    y,
                    [
                        rect[0] + region.rect[0],
                        rect[1] + region.rect[1],
                        region.rect[2],
                        region.rect[3],
                    ],
                )
            })
    }
    fn hit_base(&self, x: i32, y: i32, rect: [f32; 4]) -> bool {
        let x = x as f32 - rect[0];
        let y = y as f32 - rect[1];
        if x < 0.
            || y < 0.
            || x >= rect[2]
            || y >= rect[3]
            || (self.strict_bounds && (x <= 0. || y <= 0.))
        {
            return false;
        }
        let [x, y] = if let Some(m) = self.source_from_local {
            let w = m[6] * x + m[7] * y + m[8];
            if !w.is_finite() || w.abs() <= 1.0e-12 {
                return false;
            }
            [
                (m[0] * x + m[1] * y + m[2]) / w,
                (m[3] * x + m[4] * y + m[5]) / w,
            ]
        } else {
            [x, y]
        };
        if !x.is_finite() || !y.is_finite() || x < 0. || y < 0. {
            return false;
        }
        let x = x.floor() as u32;
        let y = y.floor() as u32;
        if x >= self.size[0] || y >= self.size[1] {
            return false;
        }
        let index = u64::from(y) * u64::from(self.size[0]) + u64::from(x);
        self.coverage
            .get(index as usize)
            .is_some_and(|&v| v >= self.threshold)
    }
}

/// Small retained-geometry update. Source coverage planes and thresholds remain
/// owned by the current widget; no pixel bytes cross this command boundary.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WidgetMaskGeometry {
    pub source_from_local: Option<[f32; 9]>,
    #[serde(default)]
    pub regions: Vec<WidgetRegionGeometry>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WidgetRegionGeometry {
    pub rect: [f32; 4],
    pub source_from_local: Option<[f32; 9]>,
}
impl WidgetHitMask {
    pub fn geometry(&self) -> WidgetMaskGeometry {
        WidgetMaskGeometry {
            source_from_local: self.source_from_local,
            regions: self
                .regions
                .iter()
                .map(|region| WidgetRegionGeometry {
                    rect: region.rect,
                    source_from_local: region.mask.source_from_local,
                })
                .collect(),
        }
    }
    pub(crate) fn set_geometry(
        &mut self,
        geometry: &WidgetMaskGeometry,
        rect: [f32; 4],
    ) -> Result<()> {
        ensure!(
            geometry.regions.len() == self.regions.len(),
            "UI hit geometry plane topology changed"
        );
        self.source_from_local = geometry.source_from_local;
        for (region, geometry) in self.regions.iter_mut().zip(&geometry.regions) {
            region.rect = geometry.rect;
            region.mask.source_from_local = geometry.source_from_local;
        }
        self.validate(rect)
    }
}

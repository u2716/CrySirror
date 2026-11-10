// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Fixed-grid source sampling through a rectangle or directed half-plane.
//! The explicit float half-bias and inclusive axis edges are reusable stored-
//! pixel policies, independent of native effect IDs, pictures and clocks.
use crate::{
    ImageSurface,
    byte_raster::ByteRasterMath,
    fixed_scanlines::{FixedSampleSpan, FixedScanlines, ScanlineMath},
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ScanlineClip {
    Rectangle {
        bounds: [i32; 4],
    },
    /// Directed boundary: dy<0 retains the right side,dy>0 the inclusive left
    /// side. A horizontal vector retains bottom/top for dx>0/dx<0 respectively.
    HalfPlane {
        origin: [i32; 2],
        vector: [i32; 2],
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClippedScanlineSample {
    pub source_q16: [i64; 2],
    pub source_step_q16: [i64; 2],
    pub clip: ScanlineClip,
    pub math: ByteRasterMath,
}
impl ClippedScanlineSample {
    pub fn validate(&self) -> Result<()> {
        self.math.validate()?;
        match self.clip {
            ScanlineClip::Rectangle { bounds } => ensure!(
                bounds.iter().all(|v| v.unsigned_abs() <= 1_000_000)
                    && bounds[2] >= 0
                    && bounds[3] >= 0,
                "scanline rectangle clip bounds"
            ),
            ScanlineClip::HalfPlane { origin, vector } => ensure!(
                origin
                    .iter()
                    .chain(&vector)
                    .all(|v| v.unsigned_abs() <= 1_000_000)
                    && vector != [0, 0],
                "scanline half-plane parameters"
            ),
        }
        Ok(())
    }
    pub fn apply(
        &self,
        destination: &ImageSurface,
        source: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        destination.validate()?;
        let width = i64::from(destination.width);
        let mut spans = Vec::with_capacity(destination.height as usize);
        for row in 0..destination.height {
            check()?;
            let y = i64::from(row);
            let (left, right) = match self.clip {
                ScanlineClip::Rectangle {
                    bounds: [x, top, w, h],
                } => {
                    if y < i64::from(top) || y >= i64::from(top) + i64::from(h) {
                        continue;
                    }
                    (i64::from(x), i64::from(x) + i64::from(w))
                }
                ScanlineClip::HalfPlane {
                    origin: [cx, cy],
                    vector: [vx, vy],
                } => {
                    if vy == 0 {
                        if vx > 0 && y < i64::from(cy) || vx < 0 && y > i64::from(cy) {
                            continue;
                        }
                        (0, width)
                    } else if vx == 0 {
                        if vy < 0 {
                            (i64::from(cx), width)
                        } else {
                            (0, i64::from(cx) + 1)
                        }
                    } else {
                        let slope = (vx as f32) / (vy as f32);
                        let bias = if vy >= 0 { -1.0 } else { 0.0 };
                        let limit = i64::from(cx)
                            + (((y - i64::from(cy)) as f32) * slope + bias + 0.5) as i64;
                        if vy < 0 {
                            (limit.max(0), width)
                        } else {
                            (0, (limit + 1).min(width))
                        }
                    }
                }
            };
            let left = left.max(0);
            let right = right.min(width);
            if right <= left {
                continue;
            }
            spans.push(FixedSampleSpan {
                destination: [left as i32, row as i32, (right - left) as i32],
                source_q16: [
                    self.source_q16[0]
                        .checked_add(
                            self.source_step_q16[0]
                                .checked_mul(left)
                                .ok_or_else(|| anyhow::anyhow!("scanline x phase overflow"))?,
                        )
                        .ok_or_else(|| anyhow::anyhow!("scanline x phase overflow"))?,
                    self.source_q16[1]
                        .checked_add(
                            self.source_step_q16[1]
                                .checked_mul(y)
                                .ok_or_else(|| anyhow::anyhow!("scanline y phase overflow"))?,
                        )
                        .ok_or_else(|| anyhow::anyhow!("scanline y phase overflow"))?,
                ],
                source_step_q16: [self.source_step_q16[0], 0],
                math: ScanlineMath::Stored { math: self.math },
            });
        }
        FixedScanlines {
            spans,
            corner_rgb_key: false,
            source_color: None,
        }
        .apply(destination, source, check)
    }
}

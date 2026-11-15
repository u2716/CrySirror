// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Inclusive integer ellipse spans with explicit stored-byte paint arithmetic.
//! Geometry and pixels are reusable image-worker operations, without a VM.
use crate::{effects::MAX_EFFECT_PIXELS, fixed_scanlines::ScanlineMath, ImageSurface};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EllipticPaintMath {
    /// Independent low-word RGB weights,16-bit wrapping sum, one /256 shift.
    Mix { color: [u8; 3], weights: [u16; 2] },
    /// Saturating addition then subtraction, independently per stored RGB byte.
    Offset { add: [u8; 3], subtract: [u8; 3] },
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EllipticCutout {
    pub center: [i32; 2],
    pub radius: [i32; 2],
    pub padding: i32,
    /// True intersects cut boundaries with the outer span. False produces
    /// independent left/right spans bounded only by the image rectangle.
    pub intersect_outer: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EllipticPaint {
    pub center: [i32; 2],
    pub radius: [i32; 2],
    pub cutout: Option<EllipticCutout>,
    /// Inclusive outer row range. None uses centerY-radiusY..centerY+radiusY.
    /// Rows outside the ellipse retain the clamped one-pixel endpoint.
    pub rows: Option<[i32; 2]>,
    pub math: EllipticPaintMath,
}
impl EllipticPaint {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.center
                .iter()
                .chain(&self.radius)
                .all(|n| n.unsigned_abs() <= 1_000_000),
            "elliptic paint coordinate budget"
        );
        if let Some(hole) = self.cutout {
            ensure!(
                hole.center
                    .iter()
                    .chain(&hole.radius)
                    .chain(std::iter::once(&hole.padding))
                    .all(|n| n.unsigned_abs() <= 1_000_000),
                "elliptic cutout coordinate budget"
            );
        }
        ensure!(
            self.rows
                .is_none_or(|r| r.iter().all(|n| n.unsigned_abs() <= 2_000_000)),
            "elliptic paint row budget"
        );
        Ok(())
    }
    pub fn apply(
        &self,
        input: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        input.validate()?;
        ensure!(
            u64::from(input.width) * u64::from(input.height) <= MAX_EFFECT_PIXELS,
            "elliptic paint image budget"
        );
        check()?;
        let mut output = input.clone();
        if self.radius.iter().any(|n| *n < 1)
            || self.cutout.is_some_and(|h| h.radius.iter().any(|n| *n < 1))
        {
            return Ok(output);
        }
        let [cx, cy] = self.center.map(i64::from);
        let [rx, ry] = self.radius.map(i64::from);
        let range = self
            .rows
            .map(|r| r.map(i64::from))
            .unwrap_or([cy - ry, cy + ry]);
        let top = range[0].max(0);
        let bottom = range[1].min(i64::from(input.height) - 1);
        for row in top..=bottom {
            check()?;
            let y = (row as f64 - cy as f64) / ry as f64;
            let mut remaining = y * y;
            remaining *= -1.;
            remaining += 1.;
            if remaining < 0. {
                remaining = 0.;
            }
            let half = (remaining.sqrt() * rx as f64 + 0.5) as i64;
            let left = (cx - half).max(0);
            let right = (cx + half).min(i64::from(input.width) - 1);
            if right < left {
                continue;
            }
            let spans = if let Some(hole) = self.cutout.filter(|h| {
                row >= i64::from(h.center[1]) - i64::from(h.radius[1])
                    && row <= i64::from(h.center[1]) + i64::from(h.radius[1])
            }) {
                let yy = (row as f64 - f64::from(hole.center[1])) / f64::from(hole.radius[1]);
                let mut remaining = yy * yy;
                remaining *= -1.;
                remaining += 1.;
                if remaining < 0. {
                    remaining = 0.;
                }
                let half = (remaining.sqrt() * f64::from(hole.radius[0]) + 0.5) as i64;
                let mut hole_left = (i64::from(hole.center[0]) - half - i64::from(hole.padding))
                    .min(i64::from(input.width) - 1);
                let mut hole_right =
                    (i64::from(hole.center[0]) + half + i64::from(hole.padding)).max(0);
                if hole.intersect_outer {
                    hole_left = hole_left.min(right);
                    hole_right = hole_right.max(left);
                }
                [[left, hole_left], [hole_right, right]]
            } else {
                [[left, right], [1, 0]]
            };
            for [left, right] in spans {
                for column in left..=right {
                    let at = (row as usize * input.width as usize + column as usize) * 4;
                    let destination = &mut output.rgba[at..at + 4];
                    match self.math {
                        EllipticPaintMath::Mix { color, weights } => {
                            ScanlineMath::ConstantSummed { weights }
                                .pixel(destination, &[color[0], color[1], color[2], 255]);
                        }
                        EllipticPaintMath::Offset { add, subtract } => {
                            for c in 0..3 {
                                destination[c] = destination[c]
                                    .saturating_add(add[c])
                                    .saturating_sub(subtract[c]);
                            }
                        }
                    }
                }
            }
        }
        check()?;
        Ok(output)
    }
}

// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Batched signed RGB radial brushes over stored colour, preserving coverage.
//! One radial table and one immutable-output allocation serve all placements.
use crate::ImageSurface;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadialRgbBrushes {
    pub brush_size: [u32; 2],
    pub coordinate_origin: [i32; 2],
    pub coordinate_step: [i32; 2],
    pub radial_strength: u32,
    pub signed_rgb: [i32; 3],
    pub percent: u32,
    pub positions: Vec<[i32; 2]>,
}
impl RadialRgbBrushes {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.brush_size.iter().all(|n| (1..=1024).contains(n))
                && self
                    .coordinate_origin
                    .iter()
                    .chain(&self.coordinate_step)
                    .all(|n| n.unsigned_abs() <= 1_000_000)
                && self.radial_strength <= 1_000_000
                && self.percent <= 100
                && self.positions.len() <= 4096
                && self
                    .positions
                    .iter()
                    .flatten()
                    .all(|n| n.unsigned_abs() <= 1_000_000),
            "radial RGB brush domain"
        );
        ensure!(
            u64::from(self.brush_size[0])
                * u64::from(self.brush_size[1])
                * self.positions.len() as u64
                <= crate::effects::MAX_EFFECT_PIXELS,
            "radial RGB brush work budget"
        );
        Ok(())
    }
    pub fn scratch_bytes(&self) -> Result<usize> {
        self.validate()?;
        Ok(self.brush_size[0] as usize * self.brush_size[1] as usize * 6)
    }
    pub fn apply(
        &self,
        destination: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        destination.validate()?;
        ensure!(
            u64::from(destination.width) * u64::from(destination.height)
                <= crate::effects::MAX_EFFECT_PIXELS,
            "radial RGB brush output budget"
        );
        let mut table = Vec::<[[u8; 3]; 2]>::with_capacity(self.scratch_bytes()? / 6);
        for y in 0..self.brush_size[1] {
            check()?;
            for x in 0..self.brush_size[0] {
                let dx = i64::from(self.coordinate_origin[0])
                    + i64::from(x) * i64::from(self.coordinate_step[0]);
                let dy = i64::from(self.coordinate_origin[1])
                    + i64::from(y) * i64::from(self.coordinate_step[1]);
                // Preserve the explicit sqrt/square round trip: at certain
                // integer radii, algebraic cancellation changes truncation.
                let r = ((dx as f64) * (dx as f64) + (dy as f64) * (dy as f64)).sqrt();
                let radial = (r * r * f64::from(self.radial_strength)) as i32;
                let mut pixel = [[0; 3]; 2];
                for c in 0..3 {
                    let subtract = self.signed_rgb[c] < 0;
                    // Defined 32-bit packed arithmetic replaces original
                    // signed-overflow UB, including abs(INT_MIN).
                    let amplitude = if subtract {
                        self.signed_rgb[c].wrapping_neg()
                    } else {
                        self.signed_rgb[c]
                    };
                    let value = amplitude
                        .wrapping_sub(radial)
                        .wrapping_mul(self.percent as i32)
                        / 100;
                    pixel[usize::from(subtract)][c] = value.clamp(0, 255) as u8;
                }
                table.push(pixel);
            }
        }
        check()?;
        let mut out = destination.clone();
        for [x, y] in &self.positions {
            check()?;
            // A complete-brush profile rejects partial sprites, matching
            // callers whose centres are admitted by an inset screen clip.
            if *x < 0
                || *y < 0
                || i64::from(*x) + i64::from(self.brush_size[0]) > i64::from(out.width)
                || i64::from(*y) + i64::from(self.brush_size[1]) > i64::from(out.height)
            {
                continue;
            }
            for row in 0..self.brush_size[1] {
                check()?;
                for col in 0..self.brush_size[0] {
                    let brush = table[row as usize * self.brush_size[0] as usize + col as usize];
                    let at = ((*y as usize + row as usize) * out.width as usize
                        + *x as usize
                        + col as usize)
                        * 4;
                    for c in 0..3 {
                        out.rgba[at + c] = out.rgba[at + c]
                            .saturating_add(brush[0][c])
                            .saturating_sub(brush[1][c]);
                    }
                }
            }
        }
        check()?;
        Ok(out)
    }
}

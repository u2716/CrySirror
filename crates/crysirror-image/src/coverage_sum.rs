// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Saturating sums of explicitly offset coverage samples, for glyph edges and
//! authored coverage kernels. Pixels outside the immutable source contribute zero.
use crate::{effects::MAX_EFFECT_PIXELS, ImageSurface};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageSum {
    pub offsets: Vec<[i32; 2]>,
    pub color: [u8; 3],
}
impl CoverageSum {
    pub fn apply(
        &self,
        input: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        input.validate()?;
        ensure!(
            !self.offsets.is_empty()
                && self.offsets.len() <= 256
                && self
                    .offsets
                    .iter()
                    .flatten()
                    .all(|v| v.unsigned_abs() <= 1024),
            "coverage sum kernel extent"
        );
        let pixels = u64::from(input.width) * u64::from(input.height);
        ensure!(
            pixels <= MAX_EFFECT_PIXELS && pixels * self.offsets.len() as u64 <= 268_435_456,
            "coverage sum work budget"
        );
        let mut output = ImageSurface {
            width: input.width,
            height: input.height,
            rgba: vec![0; input.rgba.len()],
        };
        for y in 0..input.height as i32 {
            check()?;
            for x in 0..input.width as i32 {
                if x & 255 == 0 {check()?;}
                let mut alpha = 0u32;
                for [dx, dy] in &self.offsets {
                    let sx = x + dx;
                    let sy = y + dy;
                    if sx >= 0 && sy >= 0 && sx < input.width as i32 && sy < input.height as i32 {
                        alpha += u32::from(
                            input.rgba[(sy as usize * input.width as usize + sx as usize) * 4 + 3],
                        );
                    }
                }
                let at = (y as usize * input.width as usize + x as usize) * 4;
                output.rgba[at..at + 3].copy_from_slice(&self.color);
                output.rgba[at + 3] = alpha.min(255) as u8;
            }
        }
        check()?;
        Ok(output)
    }
}

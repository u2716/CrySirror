// Copyright (C) 2026 CrySirror contributors. MPL-2.0.
//! Stored-byte rectangle sampling with an explicit8.8 coordinate profile.
//! Source evidence: AliceLogo2 native10009900 (RGB)/10009CA0 (coverage),
//! statically read as data. No native surface or executable code is hosted.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

/// Resize an in-bounds source window. All four independent stored channels are
/// sampled alike; RGB-only/alpha-only writes use the shared composite write
/// mask afterwards, rather than inventing alpha in the sampling kernel.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RectResampleByte256 {
    /// Output width/height in pixels; zero extents are rejected.
    pub size: [u32; 2],
    /// Source x/y/width/height. Integer parts truncate toward zero. Fractional
    /// origin uses nearest-integer decomposition followed by truncating8.8
    /// conversion; extent increments are integer floor((index*extent)<<8)/size.
    pub source: [f32; 4],
}
impl RectResampleByte256 {
    pub fn validate(&self) -> Result<()> {
        crate::rgba_len(self.size[0], self.size[1])?;
        ensure!(
            u64::from(self.size[0]) * u64::from(self.size[1]) <= MAX_EFFECT_PIXELS,
            "byte256 output pixel budget"
        );
        ensure!(
            self.source
                .iter()
                .all(|v| v.is_finite() && (0. ..=16384.).contains(v)),
            "byte256 source coordinate range"
        );
        ensure!(
            self.source[2] >= 1. && self.source[3] >= 1.,
            "byte256 source extent"
        );
        Ok(())
    }
    pub fn output_size(&self, input: [u32; 2]) -> Result<[u32; 2]> {
        self.validate()?;
        crate::rgba_len(input[0], input[1])?;
        ensure!(
            u64::from(input[0]) * u64::from(input[1]) <= MAX_EFFECT_PIXELS,
            "byte256 input pixel budget"
        );
        for axis in 0..2 {
            ensure!(
                self.source[axis].trunc() as u32 + self.source[axis + 2].trunc() as u32
                    <= input[axis],
                "byte256 source window outside immutable input"
            );
        }
        Ok(self.size)
    }
    pub fn apply(
        &self,
        input: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        input.validate()?;
        self.output_size([input.width, input.height])?;
        check()?;
        let origin = [self.source[0] as i64, self.source[1] as i64];
        let extent = [self.source[2] as i64, self.source[3] as i64];
        let phase = std::array::from_fn::<_, 2, _>(|axis| {
            let fraction = f64::from(self.source[axis]) - origin[axis] as f64;
            let whole = fraction.round_ties_even();
            whole as i64 * 256 + ((fraction - whole) * 256.).trunc() as i64
        });
        let coordinate = |axis: usize, index: u32| {
            let q =
                phase[axis] + ((i64::from(index) * extent[axis]) << 8) / i64::from(self.size[axis]);
            let at = q >> 8;
            // Native clamps each neighbour to its declared source window. The
            // phase weight remains unchanged at either edge; equal taps make
            // that weight immaterial. No half-texel offset is introduced.
            (
                at.clamp(0, extent[axis] - 1) + origin[axis],
                (at + 1).clamp(0, extent[axis] - 1) + origin[axis],
                (q & 255) as i32,
            )
        };
        let tap = |x: i64, y: i64| -> [u8; 4] {
            let at = ((y * i64::from(input.width) + x) * 4) as usize;
            input.rgba[at..at + 4].try_into().unwrap()
        };
        let mix = |a: [u8; 4], b: [u8; 4], weight: i32| -> [u8; 4] {
            std::array::from_fn(|c| {
                (i32::from(a[c]) + ((weight * (i32::from(b[c]) - i32::from(a[c]))) >> 8)) as u8
            })
        };
        let mut output = ImageSurface {
            width: self.size[0],
            height: self.size[1],
            rgba: vec![0; crate::rgba_len(self.size[0], self.size[1])?],
        };
        for y in 0..self.size[1] {
            check()?;
            let (y0, y1, wy) = coordinate(1, y);
            for x in 0..self.size[0] {
                let (x0, x1, wx) = coordinate(0, x);
                let value = mix(
                    mix(tap(x0, y0), tap(x1, y0), wx),
                    mix(tap(x0, y1), tap(x1, y1), wx),
                    wy,
                );
                let at = ((u64::from(y) * u64::from(self.size[0]) + u64::from(x)) * 4) as usize;
                output.rgba[at..at + 4].copy_from_slice(&value);
            }
        }
        check()?;
        Ok(output)
    }
}

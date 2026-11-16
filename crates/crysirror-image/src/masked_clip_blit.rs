// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Stored-mask copies through a directed scanline boundary. Explicit packed
//! low-word products preserve old software sprite arithmetic without exposing
//! native pictures, effect IDs, clocks or executable pointers to the kernel.
use crate::{ImageSurface, clipped_scanline_sample::ScanlineClip};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaskedClipBlit {
    pub destination: [i32; 2],
    pub clip: ScanlineClip,
    /// Each nonzero stored coverage byte is incremented before a wrapping
    /// low-word multiplication by this weight and a logical eight-bit shift.
    pub weight: u16,
    /// Independent byte multipliers followed by a logical shift of eight,
    /// then wrapping packed-word additions. RGB words are not prematurely
    /// clamped to bytes before the final covered blend.
    pub source_scale_add: Option<([u8; 3], [u16; 3])>,
}
impl MaskedClipBlit {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.destination
                .iter()
                .all(|n| n.unsigned_abs() <= 1_000_000),
            "masked clipped blit destination bounds"
        );
        match self.clip {
            ScanlineClip::Rectangle { bounds } => ensure!(
                bounds.iter().all(|n| n.unsigned_abs() <= 1_000_000)
                    && bounds[2] >= 0
                    && bounds[3] >= 0,
                "masked clipped rectangle bounds"
            ),
            ScanlineClip::HalfPlane { origin, vector } => ensure!(
                origin
                    .iter()
                    .chain(&vector)
                    .all(|n| n.unsigned_abs() <= 1_000_000)
                    && vector != [0, 0],
                "masked clipped half-plane bounds"
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
        source.validate()?;
        ensure!(
            u64::from(destination.width) * u64::from(destination.height)
                <= crate::effects::MAX_EFFECT_PIXELS,
            "masked clipped blit image budget"
        );
        let mut out = destination.clone();
        let [dx, dy] = self.destination.map(i64::from);
        let top = dy.max(0);
        let bottom = (dy + i64::from(source.height)).min(i64::from(out.height));
        let width = i64::from(out.width);
        for y in top..bottom {
            check()?;
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
                        let slope = vx as f32 / vy as f32;
                        let bias = if vy >= 0 { -1.0 } else { 0.0 };
                        let edge = i64::from(cx)
                            + (((y - i64::from(cy)) as f32) * slope + bias + 0.5) as i64;
                        if vy < 0 {
                            (edge.max(0), width)
                        } else {
                            (0, (edge + 1).min(width))
                        }
                    }
                }
            };
            // Native axis loops could address rows before/after their work
            // table. Intersect signed geometry before indexing: out-of-screen
            // coordinates have a defined empty contribution instead of RAM
            // reads/writes. Image coverage remains independent of output alpha.
            let left = left.max(dx).max(0);
            let right = right.min(dx + i64::from(source.width)).min(width);
            for x in left..right {
                let si = (((y - dy) as u32 * source.width + (x - dx) as u32) * 4) as usize;
                let sample = &source.rgba[si..si + 4];
                if sample[3] == 0 {
                    continue;
                }
                let a = (u16::from(sample[3]) + 1).wrapping_mul(self.weight) >> 8;
                let b = 256u16.wrapping_sub(a);
                let di = ((y as u32 * out.width + x as u32) * 4) as usize;
                for channel in 0..3 {
                    let mut value = u16::from(sample[channel]);
                    if let Some((scale, add)) = self.source_scale_add {
                        value = (value.wrapping_mul(u16::from(scale[channel])) >> 8)
                            .wrapping_add(add[channel]);
                    }
                    out.rgba[di + channel] = (value
                        .wrapping_mul(a)
                        .wrapping_add(u16::from(out.rgba[di + channel]).wrapping_mul(b))
                        >> 8) as u8;
                }
            }
        }
        Ok(out)
    }
}

// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Worker-prepared signed direction/radius data carried by an ordinary scoped
//! immutable image lease. Four directions and four radius offsets occupy six
//! RGBA storage pixels; no VM-owned image buffer or ambient cache is required.
use crate::{effects::MAX_EFFECT_PIXELS, rgba_len};
use anyhow::{Result, ensure};
use crysirror_core::image_processing::{PackedImageSpec, PackedPixelLayout};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum RadialSquaredMath {
    Wide,
    Wrapping32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackedRadialField {
    pub canvas: [u32; 2],
    pub center: [i32; 2],
    /// Each sample's direction-X numerator may use another lane's coordinate.
    /// Radius and direction-Y always use the sample's own radius. Ordinary
    /// fields select [0,1,2,3]; compatibility fields can select explicit lanes.
    pub x_lanes: [u8; 4],
    pub squared_math: RadialSquaredMath,
}
impl PackedRadialField {
    pub fn storage_size(&self) -> Result<[u32; 2]> {
        ensure!(
            self.canvas.iter().all(|n| (1..=16384).contains(n))
                && self.center.iter().all(|n| n.unsigned_abs() <= 1_000_000)
                && self.x_lanes.iter().all(|n| *n < 4),
            "packed radial field domain"
        );
        let width = (self.canvas[0] + 3) & !3;
        let size = [width * 3 / 2, self.canvas[1]];
        rgba_len(size[0], size[1])?;
        ensure!(
            u64::from(size[0]) * u64::from(size[1]) <= MAX_EFFECT_PIXELS,
            "packed radial field storage budget"
        );
        Ok(size)
    }
    pub fn radius_count(&self) -> Result<usize> {
        self.storage_size()?;
        let square = u64::from(self.canvas[0]) * u64::from(self.canvas[0])
            + u64::from(self.canvas[1]) * u64::from(self.canvas[1]);
        Ok(((square as f64).sqrt() + 1.0) as usize)
    }
    pub fn prepare(&self, check: &dyn Fn() -> Result<()>) -> Result<PackedImageSpec> {
        let size = self.storage_size()?;
        let stride = size[0] as usize * 4;
        let max_radius = (self.radius_count()? as i32 - 1).max(1);
        let mut bytes = vec![0u8; rgba_len(size[0], size[1])?];
        check()?;
        for y in 0..self.canvas[1] {
            check()?;
            let dy = (y as i32).wrapping_sub(self.center[1]);
            for group in 0..(self.canvas[0] + 3) / 4 {
                if group % 1024 == 0 {
                    check()?;
                }
                let x = (group * 4) as i32;
                let deltas = std::array::from_fn::<_, 4, _>(|lane| {
                    x.wrapping_sub(self.center[0]).wrapping_add(lane as i32)
                });
                for lane in 0..4 {
                    let dx = deltas[lane];
                    let square = match self.squared_math {
                        RadialSquaredMath::Wide => (i64::from(dx) * i64::from(dx)
                            + i64::from(dy) * i64::from(dy))
                        .max(1) as f64,
                        RadialSquaredMath::Wrapping32 => {
                            f64::from(dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy)).max(1))
                        }
                    };
                    let radius = square.sqrt();
                    let direction_x = ((f64::from(deltas[self.x_lanes[lane] as usize]) * 256.0
                        / radius)
                        + 0.5) as i32 as i16;
                    let direction_y = ((f64::from(dy) * 256.0 / radius) + 0.5) as i32 as i16;
                    let radius_word =
                        (((radius + 0.5) as i32).clamp(1, max_radius).wrapping_mul(4)) as u16;
                    let at = y as usize * stride + group as usize * 24;
                    bytes[at + lane * 4..at + lane * 4 + 2]
                        .copy_from_slice(&direction_x.to_le_bytes());
                    bytes[at + lane * 4 + 2..at + lane * 4 + 4]
                        .copy_from_slice(&direction_y.to_le_bytes());
                    bytes[at + 16 + lane * 2..at + 18 + lane * 2]
                        .copy_from_slice(&radius_word.to_le_bytes());
                }
            }
        }
        // Packed interop accepts BGRA. Arrange that transport so its immutable
        // decoded RGBA bytes preserve the exact little-endian data layout.
        for (row, pixels) in bytes.chunks_mut(stride).enumerate() {
            if row % 64 == 0 {
                check()?;
            }
            for pixel in pixels.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }
        }
        check()?;
        Ok(PackedImageSpec {
            size,
            layout: PackedPixelLayout::Bgra8,
            bytes: bytes.into(),
        })
    }
}

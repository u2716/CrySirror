// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Quantized spatial RGB bands added to source pixels before exact stored-mask
//! composition. Coordinates and ramp-table domain are independent of source UVs.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpatialColorBand {
    pub source: [i32; 4],
    pub destination: [i32; 2],
    pub table_origin: i32,
    pub table_extent: i32,
    pub coordinate: i32,
    pub increments: [i32; 2],
    pub band_length: i32,
    pub gradient_length: i32,
    pub brightness: [i32; 3],
}
impl SpatialColorBand {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.source
                .iter()
                .chain(&self.destination)
                .chain([
                    &self.table_origin,
                    &self.coordinate,
                    &self.band_length,
                    &self.gradient_length
                ])
                .all(|n| n.unsigned_abs() <= 1_000_000),
            "spatial colour band coordinates"
        );
        ensure!(
            self.source[2] >= 0
                && self.source[3] >= 0
                && self.source[2] as u64 * self.source[3] as u64 <= MAX_EFFECT_PIXELS * 4,
            "spatial colour band sample budget"
        );
        ensure!(
            (0..=4_194_304).contains(&self.table_extent)
                && self.increments.iter().all(|n| n.unsigned_abs() <= 16384),
            "spatial colour band ramp domain"
        );
        Ok(())
    }
    fn color(&self, coordinate: i32) -> [u8; 3] {
        let local = i64::from(coordinate) - i64::from(self.table_origin);
        if coordinate < 0
            || coordinate >= self.table_extent
            || local < 0
            || local >= i64::from(self.band_length)
        {
            return [0; 3];
        }
        let local = local as i32;
        let color = if self.gradient_length > 0 {
            let numerator = if local < self.gradient_length {
                Some(local)
            } else if local >= self.band_length.wrapping_sub(self.gradient_length) {
                Some(self.band_length.wrapping_sub(local))
            } else {
                None
            };
            self.brightness.map(|n| {
                numerator
                    .map_or(n, |v| n.wrapping_mul(v) / self.gradient_length)
                    .clamp(0, 255)
            })
        } else {
            self.brightness
        };
        // An ungraded raw signed RGB parameter is packed with OR, not clamped
        // per channel. Graded parameters take the explicit clamp above.
        let packed = color[0].wrapping_shl(16) | color[1].wrapping_shl(8) | color[2];
        [(packed >> 16) as u8, (packed >> 8) as u8, packed as u8]
    }
    pub fn apply(
        &self,
        destination: &ImageSurface,
        source: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        for image in [destination, source] {
            image.validate()?;
            ensure!(
                u64::from(image.width) * u64::from(image.height) <= MAX_EFFECT_PIXELS,
                "spatial colour band image budget"
            );
        }
        check()?;
        let mut output = destination.clone();
        let [sx, sy, w, h] = self.source.map(i64::from);
        let [dx, dy] = self.destination.map(i64::from);
        let left = 0.max(-sx).max(-dx);
        let top = 0.max(-sy).max(-dy);
        let right = w
            .min(i64::from(source.width) - sx)
            .min(i64::from(destination.width) - dx);
        let bottom = h
            .min(i64::from(source.height) - sy)
            .min(i64::from(destination.height) - dy);
        for y in top..bottom {
            check()?;
            for x in left..right {
                if (x - left) % 4096 == 0 {
                    check()?;
                }
                let si = (((sy + y) * i64::from(source.width) + sx + x) * 4) as usize;
                let mask = u16::from(source.rgba[si + 3]);
                if mask == 0 {
                    continue;
                }
                let di = (((dy + y) * i64::from(destination.width) + dx + x) * 4) as usize;
                let coordinate = self
                    .coordinate
                    .wrapping_add(self.increments[0].wrapping_mul(x as i32))
                    .wrapping_add(self.increments[1].wrapping_mul(y as i32));
                let color = self.color(coordinate);
                for c in 0..3 {
                    let value = source.rgba[si + c].saturating_add(color[c]);
                    output.rgba[di + c] = if mask == 255 {
                        value
                    } else {
                        ((u16::from(value) * mask >> 8)
                            + (u16::from(output.rgba[di + c]) * (256 - mask) >> 8))
                            .min(255) as u8
                    };
                }
            }
        }
        check()?;
        Ok(output)
    }
}

// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Row-colour interpolation followed by stored-mask composition. Each product
//! truncates independently; coverage255 copies,0 skips. Alpha is retained.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RowTintBlend {
    pub source: [i32; 4],
    pub destination: [i32; 2],
    pub colors: Vec<[u8; 3]>,
    pub weight: u16,
    /// A selected row colour omits the source RGB contribution. Kept explicit
    /// for packed native dispatchers which branch on colour, not on weight.
    pub source_bypass_color: Option<[u8; 3]>,
}
impl RowTintBlend {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.weight <= 256
                && self.source[2] >= 0
                && (0..=16384).contains(&self.source[3])
                && self.colors.len() == self.source[3] as usize,
            "row tint weight/rows"
        );
        ensure!(
            self.source
                .iter()
                .chain(&self.destination)
                .all(|n| n.unsigned_abs() <= 1_000_000),
            "row tint coordinates"
        );
        ensure!(
            self.source[2] as u64 * self.source[3] as u64 <= MAX_EFFECT_PIXELS,
            "row tint work budget"
        );
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
            [source, destination]
                .iter()
                .all(|i| u64::from(i.width) * u64::from(i.height) <= MAX_EFFECT_PIXELS),
            "row tint image budget"
        );
        check()?;
        let mut out = destination.clone();
        let [sx, sy, w, h] = self.source.map(i64::from);
        let [dx, dy] = self.destination.map(i64::from);
        let left = 0.max(-sx).max(-dx);
        let top = 0.max(-sy).max(-dy);
        let right = w
            .min(i64::from(source.width) - sx)
            .min(i64::from(out.width) - dx);
        let bottom = h
            .min(i64::from(source.height) - sy)
            .min(i64::from(out.height) - dy);
        let weight = u32::from(self.weight);
        for y in top..bottom {
            check()?;
            let color = self.colors[y as usize];
            let bypass = self.source_bypass_color == Some(color);
            for x in left..right {
                if x % 4096 == 0 {
                    check()?;
                }
                let si = (((sy + y) * i64::from(source.width) + sx + x) * 4) as usize;
                let di = (((dy + y) * i64::from(out.width) + dx + x) * 4) as usize;
                let mask = u32::from(source.rgba[si + 3]);
                if mask == 0 {
                    continue;
                }
                for c in 0..3 {
                    let mut value = u32::from(color[c]) * weight >> 8;
                    if !bypass {
                        value += u32::from(source.rgba[si + c]) * (256 - weight) >> 8;
                    }
                    if mask != 255 {
                        value = (value * mask >> 8)
                            + (u32::from(destination.rgba[di + c]) * (256 - mask) >> 8);
                    }
                    out.rgba[di + c] = value as u8;
                }
            }
        }
        check()?;
        Ok(out)
    }
}

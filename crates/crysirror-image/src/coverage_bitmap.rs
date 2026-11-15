//! Checked external glyph/bitmap coverage imported by shared image workers.
use crate::ImageSurface;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageBitmap {
    pub size: [u32; 2],
    pub bytes: Vec<u8>,
    pub levels: u32,
    pub color: [u8; 4],
}
impl CoverageBitmap {
    pub fn validate(&self) -> Result<()> {
        let n = u64::from(self.size[0]) * u64::from(self.size[1]);
        ensure!(
            self.size.iter().all(|v| *v > 0 && *v <= 4096) && n <= 1_048_576,
            "coverage bitmap extent"
        );
        ensure!(
            matches!(self.levels, 2 | 65 | 256) || self.levels > 256,
            "coverage levels"
        );
        ensure!(
            self.bytes.len() as u64 == n * if self.levels > 256 { 4 } else { 1 },
            "coverage bitmap length"
        );
        Ok(())
    }
    pub fn apply(&self, check: &dyn Fn() -> Result<()>) -> Result<ImageSurface> {
        self.validate()?;
        let mut out = ImageSurface {
            width: self.size[0],
            height: self.size[1],
            rgba: vec![0; crate::rgba_len(self.size[0], self.size[1])?],
        };
        for (i, p) in out.rgba.chunks_exact_mut(4).enumerate() {
            if i % 4096 == 0 {
                check()?;
            }
            if self.levels > 256 {
                let b = &self.bytes[i * 4..i * 4 + 4];
                p.copy_from_slice(&[b[2], b[1], b[0], b[3]]);
            } else {
                let alpha =
                    (u32::from(self.bytes[i]).min(self.levels - 1) * 255 / (self.levels - 1)) as u8;
                p.copy_from_slice(&[
                    self.color[0],
                    self.color[1],
                    self.color[2],
                    ((u16::from(alpha) * u16::from(self.color[3]) + 127) / 255) as u8,
                ]);
            }
        }
        Ok(out)
    }
}

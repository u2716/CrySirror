// SPDX-License-Identifier: MPL-2.0
//! Copies midpoint-circle discs of a source surface onto a destination.
//! Each disc is drawn as four symmetric half-open row spans per step (the
//! classic integer midpoint circle). A span clipped at the destination's
//! left edge keeps its unclipped source start, as legacy blitters that clip
//! only the destination pointer did; source reads are linear in the source
//! buffer and must stay inside it.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscCopy {
    pub source_center: [i32; 2],
    pub destination_center: [i32; 2],
    /// Negative radii draw nothing.
    pub radius: i32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscCopies {
    pub discs: Vec<DiscCopy>,
}
impl DiscCopies {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.discs.len() <= 16384, "disc copy budget");
        ensure!(
            self.discs.iter().all(|d| d.radius <= 8192
                && d.source_center.iter().chain(&d.destination_center).all(|v| v.abs() <= 1 << 20)),
            "disc copy geometry"
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
            source.width == destination.width
                && u64::from(destination.width) * u64::from(destination.height) <= MAX_EFFECT_PIXELS,
            "disc copy surfaces"
        );
        let (w, h) = (destination.width as i64, destination.height as i64);
        let mut out = destination.clone();
        let total = source.rgba.len() as i64;
        for (n, disc) in self.discs.iter().enumerate() {
            if n % 256 == 0 {
                check()?;
            }
            let r = i64::from(disc.radius);
            if r < 0 {
                continue;
            }
            let [cx, cy] = disc.destination_center.map(i64::from);
            let [sx, sy] = disc.source_center.map(i64::from);
            let mut span = |row: i64, left: i64, right: i64, source_x: i64, source_row: i64| -> Result<()> {
                if !(0..h).contains(&row) {
                    return Ok(());
                }
                let (start, end) = (left.max(0), right.min(w));
                if start >= end {
                    return Ok(());
                }
                let len = end - start;
                let from = (source_row * w + source_x) * 4;
                ensure!(from >= 0 && from + len * 4 <= total, "disc copy source read outside the image");
                let to = ((row * w + start) * 4) as usize;
                out.rgba[to..to + len as usize * 4]
                    .copy_from_slice(&source.rgba[from as usize..(from + len * 4) as usize]);
                Ok(())
            };
            let (mut error, mut odd, mut x_extent, mut k) = (r, 0i64, r, 0i64);
            loop {
                span(cy + k, cx - x_extent, cx + x_extent, sx - x_extent, sy + k)?;
                span(cy - k, cx - x_extent, cx + x_extent, sx - x_extent, sy - k)?;
                span(cy + x_extent, cx - k, cx + k, sx - k, sy + x_extent)?;
                span(cy - x_extent, cx - k, cx + k, sx - k, sy - x_extent)?;
                error += -1 - odd;
                if error < 0 {
                    error += 2 * x_extent - 2;
                    x_extent -= 1;
                }
                odd += 2;
                k += 1;
                if x_extent < k {
                    break;
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn copies_a_moved_disc() {
        let size = 32u32;
        let source = ImageSurface { width: size, height: size, rgba: vec![200; (size * size * 4) as usize] };
        let destination = ImageSurface { width: size, height: size, rgba: vec![0; (size * size * 4) as usize] };
        let spec = DiscCopies {
            discs: vec![DiscCopy { source_center: [10, 10], destination_center: [20, 16], radius: 4 }],
        };
        let out = spec.apply(&destination, &source, &|| Ok(())).unwrap();
        let px = |x: u32, y: u32| out.rgba[((y * size + x) * 4) as usize];
        assert_eq!(px(20, 16), 200);
        assert_eq!(px(5, 5), 0);
        assert_eq!(px(24, 16), 0);
    }
}

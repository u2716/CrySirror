// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Source-dependent stochastic block painting. Sampling, coverage search and
//! returned RNG state form one cancellable worker transaction, independent of
//! VMs, native picture IDs, scene layers and output devices.
use crate::{
    effects::MAX_EFFECT_PIXELS, fixed_scanlines::ScanlineMath, temporal_pattern::PatternRng,
    ImageSurface,
};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlockSamplePaint {
    /// Paint the sampled RGB through each output pixel's own source coverage.
    /// Zero skips,255 copies, partial products sum before a single /256 shift
    /// with the255 complement. Optional source bounds affect painting only.
    PixelCoverage,
    /// Paint a flat rectangle. None maps sampled alpha to floor(alpha*100/255)
    /// (255 is100); Some supplies a percent. Nonpositive skips,100 copies,
    /// other positive values clamp to100 and use complementary /256 weights.
    SampleCoverage { percent: Option<i32> },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaskedBlockSample {
    pub source: [u32; 4],
    pub destination: [i32; 2],
    pub cell: [u32; 2],
    /// On a zero initial sample, try area random coordinates; then ALWAYS
    /// advance through at most area coordinates in row-major cyclic order,
    /// even when a preceding random retry found coverage. Search consumes no
    /// random values during its linear pass.
    pub search_nonzero: bool,
    pub paint: BlockSamplePaint,
    pub paint_source_bounds: Option<[i32; 4]>,
}
impl MaskedBlockSample {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.source.iter().all(|n| *n <= 16384)
                && self.source[2] > 0
                && self.source[3] > 0
                && self.cell.iter().all(|n| (1..=16384).contains(n)),
            "masked block sampling source/cell bounds"
        );
        ensure!(
            u64::from(self.source[2]) * u64::from(self.source[3]) <= MAX_EFFECT_PIXELS,
            "masked block sampling work budget"
        );
        ensure!(
            self.destination
                .iter()
                .all(|n| n.unsigned_abs() <= 1_000_000)
                && self
                    .paint_source_bounds
                    .is_none_or(|r| r.iter().all(|n| n.unsigned_abs() <= 1_000_000)),
            "masked block sampling destination/clip bounds"
        );
        Ok(())
    }
    pub fn apply(
        &self,
        destination: &ImageSurface,
        source: &ImageSurface,
        random: &mut PatternRng,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        for image in [destination, source] {
            image.validate()?;
            ensure!(
                u64::from(image.width) * u64::from(image.height) <= MAX_EFFECT_PIXELS,
                "masked block sampling image budget"
            );
        }
        for axis in 0..2 {
            ensure!(
                u64::from(self.source[axis]) + u64::from(self.source[axis + 2])
                    <= u64::from([source.width, source.height][axis]),
                "masked block sampling source rectangle"
            );
        }
        random.validate()?;
        check()?;
        let mut candidate = random.clone();
        let mut output = destination.clone();
        let [sx, sy, width, height] = self.source;
        for by in (0..height).step_by(self.cell[1] as usize) {
            check()?;
            let h = self.cell[1].min(height - by);
            for bx in (0..width).step_by(self.cell[0] as usize) {
                check()?;
                let w = self.cell[0].min(width - bx);
                let area = u64::from(w) * u64::from(h);
                let alpha_at = |x: u32, y: u32| -> u8 {
                    source.rgba[(((sy + by + y) as usize * source.width as usize
                        + (sx + bx + x) as usize)
                        * 4)
                        + 3]
                };
                let mut x = candidate.next_draw()? % w;
                let mut y = candidate.next_draw()? % h;
                let mut alpha = alpha_at(x, y);
                if alpha == 0 && self.search_nonzero {
                    for attempt in 0..area {
                        if attempt % 256 == 0 {
                            check()?;
                        }
                        x = candidate.next_draw()? % w;
                        y = candidate.next_draw()? % h;
                        alpha = alpha_at(x, y);
                        if alpha != 0 {
                            break;
                        }
                    }
                    // Deliberately unconditional after random retries: the
                    // next nonzero sample, rather than the just-found pixel,
                    // determines colour/opacity in this documented policy.
                    for attempt in 0..area {
                        if attempt % 256 == 0 {
                            check()?;
                        }
                        x += 1;
                        if x >= w {
                            x = 0;
                            y = (y + 1) % h;
                        }
                        alpha = alpha_at(x, y);
                        if alpha != 0 {
                            break;
                        }
                    }
                }
                if alpha == 0 {
                    continue;
                }
                let sample =
                    ((sy + by + y) as usize * source.width as usize + (sx + bx + x) as usize) * 4;
                let color: [u8; 3] = source.rgba[sample..sample + 3].try_into().unwrap();
                let math = match self.paint {
                    BlockSamplePaint::PixelCoverage => ScanlineMath::CoverageSummed { weight: 256 },
                    BlockSamplePaint::SampleCoverage { percent } => {
                        let percent = percent.unwrap_or_else(|| {
                            if alpha == 255 {
                                100
                            } else {
                                i32::from(alpha) * 100 / 255
                            }
                        });
                        if percent <= 0 {
                            continue;
                        }
                        let a = (percent.min(100) * 256 / 100) as u16;
                        ScanlineMath::ConstantSummed {
                            weights: [a, 256 - a],
                        }
                    }
                };
                for row in 0..h {
                    if row % 16 == 0 {
                        check()?;
                    }
                    let source_y = sy + by + row;
                    let target_y = i64::from(self.destination[1]) + i64::from(by + row);
                    if target_y < 0 || target_y >= i64::from(output.height) {
                        continue;
                    }
                    for column in 0..w {
                        let source_x = sx + bx + column;
                        if matches!(self.paint, BlockSamplePaint::PixelCoverage)
                            && self.paint_source_bounds.is_some_and(|[l, t, r, b]| {
                                i64::from(source_x) < i64::from(l)
                                    || i64::from(source_x) >= i64::from(r)
                                    || i64::from(source_y) < i64::from(t)
                                    || i64::from(source_y) >= i64::from(b)
                            })
                        {
                            continue;
                        }
                        let target_x = i64::from(self.destination[0]) + i64::from(bx + column);
                        if target_x < 0 || target_x >= i64::from(output.width) {
                            continue;
                        }
                        let coverage = source.rgba[((source_y as usize * source.width as usize
                            + source_x as usize)
                            * 4)
                            + 3];
                        let pixel = [color[0], color[1], color[2], coverage];
                        let at =
                            (target_y as usize * output.width as usize + target_x as usize) * 4;
                        math.pixel(&mut output.rgba[at..at + 4], &pixel);
                    }
                }
            }
        }
        check()?;
        *random = candidate;
        Ok(output)
    }
}

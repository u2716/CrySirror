// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Reusable randomized reveals over explicit caller-owned random samples.
//! Workers never draw RNG; retries and cancellation cannot change its sequence.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RandomWipeGeometry {
    RaggedEdge { reverse: bool, gradient: u16 },
    RowDissolve,
    Mosaic,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RandomWipe {
    pub geometry: RandomWipeGeometry,
    pub count: u32,
    pub duration: u32,
    pub samples: Vec<u16>,
}
impl RandomWipe {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.count <= 864000
                && self.duration <= 864000
                && self.samples.len() <= 32768
                && self.samples.iter().all(|n| *n <= 32767),
            "random wipe sample/clock budget"
        );
        if let RandomWipeGeometry::RaggedEdge { gradient, .. } = self.geometry {
            ensure!(gradient <= 256, "random wipe gradient width");
        }
        Ok(())
    }
    pub fn apply(
        &self,
        old: &ImageSurface,
        new: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        old.validate()?;
        new.validate()?;
        ensure!(
            [old.width, old.height] == [new.width, new.height]
                && u64::from(old.width) * u64::from(old.height) <= MAX_EFFECT_PIXELS,
            "random wipe image budget/extent"
        );
        let w = i64::from(old.width);
        let h = i64::from(old.height);
        let duration = i64::from(self.duration.max(1));
        let c = i64::from(self.count).min(duration);
        let mut output = old.clone();
        match self.geometry {
            RandomWipeGeometry::RaggedEdge { reverse, gradient } => {
                ensure!(
                    self.samples.len() == h as usize * 2,
                    "random edge sample extent"
                );
                let (old, new, c) = if reverse {
                    (new, old, duration - c)
                } else {
                    (old, new, c)
                };
                output = old.clone();
                let gradient = i64::from(gradient);
                let delta = if gradient > 0 { 256 / gradient } else { 0 };
                for y in 0..h {
                    check()?;
                    let start = -(i64::from(self.samples[y as usize * 2]) * w / 32768) - gradient;
                    let end = w + i64::from(self.samples[y as usize * 2 + 1]) * w / 65536;
                    let edge = start + (end - start) * c / duration;
                    let left = edge.clamp(0, w);
                    let right = (edge + gradient).clamp(0, w);
                    for x in 0..w {
                        let at = ((y * w + x) * 4) as usize;
                        if x < left {
                            output.rgba[at..at + 4].copy_from_slice(&new.rgba[at..at + 4]);
                        } else if x < right {
                            let a = (256 + edge.min(0) * delta - (x - left) * delta).clamp(0, 256)
                                as u32;
                            for channel in 0..3 {
                                output.rgba[at + channel] =
                                    (((u32::from(new.rgba[at + channel]) * a) >> 8)
                                        + ((u32::from(old.rgba[at + channel]) * (256 - a)) >> 8))
                                        .min(255) as u8;
                            }
                            output.rgba[at + 3] = 255;
                        }
                    }
                }
            }
            RandomWipeGeometry::RowDissolve => {
                ensure!(self.samples.len() == h as usize, "random row sample extent");
                for y in 0..h {
                    check()?;
                    if i64::from(self.samples[y as usize]) % duration < c {
                        let start = (y * w * 4) as usize;
                        let end = start + w as usize * 4;
                        output.rgba[start..end].copy_from_slice(&new.rgba[start..end]);
                    }
                }
            }
            RandomWipeGeometry::Mosaic => {
                let half = (duration / 2).max(1);
                let (source, c) = if c >= half {
                    (new, (duration - c).clamp(0, half))
                } else {
                    (old, c.min(half))
                };
                let cell = ((c + 1) * 64 / half).clamp(1, 64);
                if cell == 1 {
                    ensure!(
                        self.samples.is_empty(),
                        "unit mosaic consumes no random samples"
                    );
                    return Ok(source.clone());
                }
                ensure!(self.samples.len() == 2, "mosaic random phase extent");
                let xm = w % cell;
                let ym = h % cell;
                let dx = i64::from(self.samples[0]) % if xm == 0 { cell } else { xm };
                let dy = i64::from(self.samples[1]) % if ym == 0 { cell } else { ym };
                // Repair the source's skipped last full block when height%cell==0:
                // every output row is initialized instead of retaining prior screen RAM.
                for y in 0..h {
                    check()?;
                    for x in 0..w {
                        let sx = x / cell * cell + dx;
                        let sy = y / cell * cell + dy;
                        let at = ((y * w + x) * 4) as usize;
                        let from = ((sy * w + sx) * 4) as usize;
                        output.rgba[at..at + 4].copy_from_slice(&source.rgba[from..from + 4]);
                    }
                }
            }
        }
        check()?;
        Ok(output)
    }
}

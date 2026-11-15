// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Independent clipped scanline copies over immutable engine image resources.
use crate::{ImageSurface, byte_raster::ByteRasterMath, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RowBlit {
    /// Source rectangle before destination clipping.
    pub source: [i32; 4],
    pub destination: [i32; 2],
    /// One horizontal destination displacement for each source-rectangle row.
    pub row_offsets: Vec<i32>,
    pub math: ByteRasterMath,
    /// Optional per-row byte arithmetic, indexed before clipping. Empty uses
    /// `math` for every row; otherwise one entry is required per source row.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub row_math: Vec<ByteRasterMath>,
}
impl RowBlit {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.source
                .iter()
                .chain(&self.destination)
                .chain(&self.row_offsets)
                .all(|v| v.unsigned_abs() <= 1_000_000),
            "row blit coordinates"
        );
        ensure!(
            self.source[2] >= 0
                && (0..=16384).contains(&self.source[3])
                && self.row_offsets.len() == self.source[3] as usize,
            "row blit rectangle/offset count"
        );
        ensure!(
            self.source[2] as u64 * self.source[3] as u64 <= MAX_EFFECT_PIXELS * 4,
            "row blit work budget"
        );
        ensure!(
            self.row_math.is_empty() || self.row_math.len() == self.source[3] as usize,
            "row blit math count"
        );
        self.math.validate()?;
        for math in &self.row_math {
            math.validate()?;
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
            [destination, source]
                .iter()
                .all(|i| u64::from(i.width) * u64::from(i.height) <= MAX_EFFECT_PIXELS),
            "row blit image budget"
        );
        check()?;
        let mut out = destination.clone();
        self.apply_to(&mut out, source, check)?;
        Ok(out)
    }
    /// Private in-place executor for a validated ordered batch. Both surfaces
    /// were validated at admission; source always remains immutable, including
    /// when both logical operands refer to the same initial resource lease.
    pub(crate) fn apply_to(
        &self,
        out: &mut ImageSurface,
        source: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<()> {
        let [sx, sy, width, height] = self.source.map(i64::from);
        let [dx, dy] = self.destination.map(i64::from);
        let y0 = 0.max(-sy).max(-dy);
        let y1 = height
            .min(i64::from(source.height) - sy)
            .min(i64::from(out.height) - dy);
        for y in y0..y1 {
            check()?;
            let dx = dx + i64::from(self.row_offsets[y as usize]);
            let x0 = 0.max(-sx).max(-dx);
            let x1 = width
                .min(i64::from(source.width) - sx)
                .min(i64::from(out.width) - dx);
            for x in x0..x1 {
                if (x - x0) % 4096 == 0 {
                    check()?;
                }
                let si = (((sy + y) * i64::from(source.width) + sx + x) * 4) as usize;
                let di = (((dy + y) * i64::from(out.width) + dx + x) * 4) as usize;
                self.row_math
                    .get(y as usize)
                    .copied()
                    .unwrap_or(self.math)
                    .pixel(&mut out.rgba[di..di + 4], &source.rgba[si..si + 4]);
            }
        }
        check()?;
        Ok(())
    }
}

/// Ordered sprite/scanline operations sharing one immutable source and one
/// output allocation. Overlapping entries observe preceding destination writes.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RowBlitBatch {
    pub operations: Vec<RowBlit>,
}
impl RowBlitBatch {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.operations.len() <= 4096,
            "row blit batch operation budget"
        );
        let mut samples = 0u64;
        let mut metadata = 0usize;
        for op in &self.operations {
            op.validate()?;
            samples += op.source[2] as u64 * op.source[3] as u64;
            metadata += op.row_offsets.len() + op.row_math.len();
            ensure!(
                samples <= MAX_EFFECT_PIXELS * 4 && metadata <= 1_048_576,
                "row blit batch sample/metadata budget"
            );
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
        for image in [destination, source] {
            image.validate()?;
            ensure!(
                u64::from(image.width) * u64::from(image.height) <= MAX_EFFECT_PIXELS,
                "row blit batch image budget"
            );
        }
        check()?;
        let mut output = destination.clone();
        for op in &self.operations {
            check()?;
            op.apply_to(&mut output, source, check)?;
        }
        check()?;
        Ok(output)
    }
}

// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Explicit16.16 scanline sampling over immutable image pairs. Span geometry,
//! phase and pixel arithmetic remain independent of scene/window/VM state.
use crate::{ImageSurface, byte_raster::ByteRasterMath, effects::MAX_EFFECT_PIXELS};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ScanlineMath {
    Stored {
        math: ByteRasterMath,
    },
    /// Multiply stored source coverage by a32-bit wrapping weight, then take
    /// bits8..23. Source/destination products wrap to16bits and floor separately;
    /// the destination weight XORs the source weight's low byte with255.
    CoverageSeparate {
        weight: u32,
        opaque_copy: bool,
    },
    /// Independent packed-word RGB weights; products floor separately and their
    /// sum saturates to255. Coverage is ignored, destination alpha is retained.
    ConstantSeparate {
        weights: [u16; 2],
    },
}
impl ScanlineMath {
    fn validate(self) -> Result<()> {
        if let Self::Stored { math } = self {
            math.validate()?;
        }
        Ok(())
    }
    fn pixel(self, d: &mut [u8], s: &[u8]) {
        let (a, b) = match self {
            Self::Stored { math } => {
                math.pixel(d, s);
                return;
            }
            Self::CoverageSeparate {
                weight,
                opaque_copy,
            } => {
                if s[3] == 0 {
                    return;
                }
                if opaque_copy && s[3] == 255 {
                    d[..3].copy_from_slice(&s[..3]);
                    return;
                }
                let a = (u32::from(s[3]).wrapping_mul(weight) >> 8) as u16;
                (a, a ^ 255)
            }
            Self::ConstantSeparate { weights } => (weights[0], weights[1]),
        };
        for c in 0..3 {
            let source = u16::from(s[c]).wrapping_mul(a) >> 8;
            let destination = u16::from(d[c]).wrapping_mul(b) >> 8;
            d[c] = source.saturating_add(destination).min(255) as u8;
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixedSampleSpan {
    /// Destination x,y,width; the source advances once for each output pixel.
    pub destination: [i32; 3],
    pub source_q16: [i64; 2],
    pub source_step_q16: [i64; 2],
    pub math: ScanlineMath,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixedScanlines {
    pub spans: Vec<FixedSampleSpan>,
    /// Skip a sampled pixel when its RGB equals source(0,0), independently of
    /// a stored coverage plane. This comparison precedes pixel arithmetic.
    pub corner_rgb_key: bool,
    /// Saturating RGB addition/subtraction after key comparison, before
    /// blending. Source coverage remains unchanged.
    pub source_color: Option<([u8; 3], [u8; 3])>,
}
impl FixedScanlines {
    /// Construct a rectangle from one source origin and independent x/y
    /// increments. Phase is retained when the destination is later clipped.
    pub fn rectangle(
        destination: [i32; 4],
        source_q16: [i64; 2],
        source_step_q16: [i64; 2],
        math: ScanlineMath,
        corner_rgb_key: bool,
        source_color: Option<([u8; 3], [u8; 3])>,
    ) -> Result<Self> {
        ensure!(
            destination[2] >= 0 && (0..=16384).contains(&destination[3]),
            "fixed scanline rectangle bounds"
        );
        let mut spans = Vec::with_capacity(destination[3] as usize);
        for row in 0..destination[3] {
            spans.push(FixedSampleSpan {
                destination: [
                    destination[0],
                    destination[1]
                        .checked_add(row)
                        .context("fixed scanline row overflow")?,
                    destination[2],
                ],
                source_q16: [
                    source_q16[0],
                    source_q16[1]
                        .checked_add(
                            source_step_q16[1]
                                .checked_mul(i64::from(row))
                                .context("fixed scanline row phase overflow")?,
                        )
                        .context("fixed scanline row phase overflow")?,
                ],
                source_step_q16: [source_step_q16[0], 0],
                math,
            });
        }
        let spec = Self {
            spans,
            corner_rgb_key,
            source_color,
        };
        spec.validate()?;
        Ok(spec)
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(self.spans.len() <= 16384, "fixed scanline span budget");
        let mut work = 0u64;
        for span in &self.spans {
            ensure!(
                span.destination
                    .iter()
                    .all(|v| v.unsigned_abs() <= 1_000_000)
                    && span.destination[2] >= 0,
                "fixed scanline destination bounds"
            );
            work = work
                .checked_add(span.destination[2] as u64)
                .context("fixed scanline work overflow")?;
            ensure!(work <= MAX_EFFECT_PIXELS * 4, "fixed scanline work budget");
            for axis in 0..2 {
                let end = span.source_step_q16[axis]
                    .checked_mul(i64::from(span.destination[2].saturating_sub(1)))
                    .and_then(|v| span.source_q16[axis].checked_add(v))
                    .context("fixed scanline phase overflow")?;
                ensure!(
                    [span.source_q16[axis], end]
                        .iter()
                        .all(|v| v.unsigned_abs() <= 65536u64 * 1_000_000),
                    "fixed scanline source coordinate budget"
                );
            }
            span.math.validate()?;
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
                "fixed scanline image budget"
            );
        }
        check()?;
        let mut output = destination.clone();
        for span in &self.spans {
            check()?;
            let [dx, dy, width] = span.destination.map(i64::from);
            if dy < 0 || dy >= i64::from(destination.height) {
                continue;
            }
            let left = 0.max(-dx);
            let right = width.min(i64::from(destination.width) - dx);
            for x in left..right {
                let sx = (span.source_q16[0] + span.source_step_q16[0] * x) >> 16;
                let sy = (span.source_q16[1] + span.source_step_q16[1] * x) >> 16;
                if sx < 0
                    || sy < 0
                    || sx >= i64::from(source.width)
                    || sy >= i64::from(source.height)
                {
                    continue;
                }
                let si = ((sy * i64::from(source.width) + sx) * 4) as usize;
                let di = ((dy * i64::from(destination.width) + dx + x) * 4) as usize;
                let pixel = &source.rgba[si..si + 4];
                if self.corner_rgb_key && pixel[..3] == source.rgba[..3] {
                    continue;
                }
                if let Some((add, sub)) = &self.source_color {
                    let mut adjusted: [u8; 4] = pixel.try_into().unwrap();
                    for c in 0..3 {
                        adjusted[c] = adjusted[c].saturating_add(add[c]).saturating_sub(sub[c]);
                    }
                    span.math.pixel(&mut output.rgba[di..di + 4], &adjusted);
                } else {
                    span.math.pixel(&mut output.rgba[di..di + 4], pixel);
                }
            }
        }
        check()?;
        Ok(output)
    }
}

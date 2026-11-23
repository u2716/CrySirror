// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Ordered solid or stored-byte-gradient RGB spans. Two independent saturating
//! byte weights mix low-word products with one final /256; alpha is retained.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredGradientSpan {
    pub destination: [i32; 3],
    pub color: [u8; 3],
    pub weights: [u8; 2],
    pub steps: [i16; 2],
    pub copy: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredGradientSpans {
    pub spans: Vec<StoredGradientSpan>,
}
impl StoredGradientSpans {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.spans.len() <= 16384, "stored gradient span count");
        let mut work = 0u64;
        for span in &self.spans {
            ensure!(
                span.destination
                    .iter()
                    .all(|n| n.unsigned_abs() <= 1_000_000)
                    && span.destination[2] >= 0,
                "stored gradient coordinates"
            );
            work += span.destination[2] as u64;
            ensure!(
                work <= MAX_EFFECT_PIXELS * 4,
                "stored gradient sample budget"
            );
        }
        Ok(())
    }
    pub fn apply(
        &self,
        input: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        input.validate()?;
        ensure!(
            u64::from(input.width) * u64::from(input.height) <= MAX_EFFECT_PIXELS,
            "stored gradient image budget"
        );
        check()?;
        let mut out = input.clone();
        for span in &self.spans {
            check()?;
            let [x, y, w] = span.destination.map(i64::from);
            if y < 0 || y >= i64::from(input.height) {
                continue;
            }
            let first = x.max(0);
            let end = (x + w).min(i64::from(input.width));
            for column in first..end {
                if (column - first) % 4096 == 0 {
                    check()?;
                }
                let at = (y as usize * input.width as usize + column as usize) * 4;
                if span.copy {
                    out.rgba[at..at + 3].copy_from_slice(&span.color);
                    continue;
                }
                let distance = column - x;
                let weights = std::array::from_fn::<_, 2, _>(|i| {
                    (i64::from(span.weights[i]) + distance * i64::from(span.steps[i])).clamp(0, 255)
                        as u16
                });
                for c in 0..3 {
                    out.rgba[at + c] = (u16::from(span.color[c])
                        .wrapping_mul(weights[0])
                        .wrapping_add(u16::from(out.rgba[at + c]).wrapping_mul(weights[1]))
                        >> 8) as u8;
                }
            }
        }
        check()?;
        Ok(out)
    }
}

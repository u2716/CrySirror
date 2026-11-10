// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Explicit signed half-biased integer DDA strokes. Pixels follow
//! start + trunc((delta * sample + divisor/2) / divisor), including reversed
//! segments; this intentionally differs from symmetric Bresenham rounding.
use crate::ImageSurface;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntegerLine {
    pub from: [i32; 2],
    pub to: [i32; 2],
    pub color: [u8; 4],
}
pub fn validate(lines: &[IntegerLine]) -> Result<()> {
    ensure!(lines.len() <= 16384, "integer line count budget");
    let mut total = 0u64;
    for line in lines {
        ensure!(
            line.from
                .iter()
                .chain(&line.to)
                .all(|n| n.unsigned_abs() <= 1_000_000),
            "integer line coordinate budget"
        );
        let dx = (i64::from(line.to[0]) - i64::from(line.from[0])).unsigned_abs();
        let dy = (i64::from(line.to[1]) - i64::from(line.from[1])).unsigned_abs();
        total += dx.max(dy) + 1;
        ensure!(total <= 8_388_608, "integer line sample budget");
    }
    Ok(())
}
impl ImageSurface {
    pub fn integer_lines(
        &self,
        lines: &[IntegerLine],
        check: &dyn Fn() -> Result<()>,
    ) -> Result<Self> {
        self.validate()?;
        validate(lines)?;
        ensure!(
            u64::from(self.width) * u64::from(self.height) <= crate::effects::MAX_EFFECT_PIXELS,
            "integer line canvas budget"
        );
        let mut out = self.clone();
        for line in lines {
            check()?;
            let delta = [
                i64::from(line.to[0]) - i64::from(line.from[0]),
                i64::from(line.to[1]) - i64::from(line.from[1]),
            ];
            let distance = delta[0].abs().max(delta[1].abs());
            let divisor = distance.max(1);
            let half = divisor / 2;
            for sample in 0..=distance {
                if sample % 4096 == 0 {
                    check()?;
                }
                let p = std::array::from_fn::<_, 2, _>(|i| {
                    i64::from(line.from[i]) + (delta[i] * sample + half) / divisor
                });
                if p[0] >= 0
                    && p[1] >= 0
                    && p[0] < i64::from(out.width)
                    && p[1] < i64::from(out.height)
                {
                    let at = (p[1] as usize * out.width as usize + p[0] as usize) * 4;
                    out.rgba[at..at + 4].copy_from_slice(&line.color);
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
    fn integer_lines_preserve_reversed_signed_rounding_clip_and_cancellation() {
        let source = ImageSurface::filled(5, 3, [0; 4]).unwrap();
        let line = IntegerLine {
            from: [4, 1],
            to: [0, 1],
            color: [10, 20, 30, 255],
        };
        let out = source.integer_lines(&[line], &|| Ok(())).unwrap();
        assert_eq!(&out.rgba[5 * 4..6 * 4], &[0; 4]);
        for x in 1..=4 {
            assert_eq!(&out.rgba[(5 + x) * 4..(6 + x) * 4], &line.color);
        }
        assert!(
            source
                .integer_lines(&[line], &|| anyhow::bail!("cancelled"))
                .is_err()
        );
        assert!(
            validate(&vec![
                IntegerLine {
                    from: [-1_000_000, 0],
                    to: [1_000_000, 0],
                    color: [0; 4]
                };
                5
            ])
            .is_err()
        );
    }
}

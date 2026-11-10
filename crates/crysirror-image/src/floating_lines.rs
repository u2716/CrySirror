// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Explicit single-precision half-biased DDA strokes and packed signed RGB
//! arithmetic. Repeated samples and overlapping lines blend in emission order.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloatingLine {
    pub from: [f32; 2],
    pub to: [f32; 2],
    pub color: [i32; 3],
    /// Source weight=floor(percent*255/100), destination=255-source; the
    /// summed signed product shifts by8.100 percent retains native /256 loss.
    pub percent: i32,
}
impl FloatingLine {
    fn samples(&self) -> i32 {
        ((self.from[0] - self.to[0]).abs() + 1.0).max((self.from[1] - self.to[1]).abs() + 1.0)
            as i32
    }
}
pub fn validate(lines: &[FloatingLine]) -> Result<()> {
    ensure!(lines.len() <= 16384, "floating line count budget");
    let mut samples = 0u64;
    for line in lines {
        ensure!(
            line.from
                .iter()
                .chain(&line.to)
                .all(|n| n.is_finite() && n.abs() <= 1_000_000.0),
            "floating line coordinate budget"
        );
        samples += line.samples().max(0) as u64;
        ensure!(samples <= 8_388_608, "floating line sample budget");
    }
    Ok(())
}
impl ImageSurface {
    pub fn floating_lines(
        &self,
        lines: &[FloatingLine],
        check: &dyn Fn() -> Result<()>,
    ) -> Result<Self> {
        self.validate()?;
        validate(lines)?;
        ensure!(
            u64::from(self.width) * u64::from(self.height) <= MAX_EFFECT_PIXELS,
            "floating line image budget"
        );
        check()?;
        let mut output = self.clone();
        for line in lines {
            check()?;
            if line.percent <= 0 {
                continue;
            }
            let delta = [line.to[0] - line.from[0], line.to[1] - line.from[1]];
            let reciprocal = 1.0 / delta[0].abs().max(delta[1].abs()).max(1.0);
            let a = line.percent.wrapping_mul(255) / 100;
            let b = 255i32.wrapping_sub(a);
            let source = line.color.map(|v| v.wrapping_mul(a));
            for i in 0..line.samples() {
                if i % 4096 == 0 {
                    check()?;
                }
                let phase = (i as f32) * reciprocal;
                let x = (line.from[0] + delta[0] * phase + 0.5) as i32;
                let y = (line.from[1] + delta[1] * phase + 0.5) as i32;
                if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
                    continue;
                }
                let at = (y as usize * self.width as usize + x as usize) * 4;
                let rgb = std::array::from_fn::<_, 3, _>(|c| {
                    i32::from(output.rgba[at + c])
                        .wrapping_mul(b)
                        .wrapping_add(source[c])
                        >> 8
                });
                // Pack raw signed lanes before extracting stored bytes; values
                // outside0..255 intentionally retain cross-lane bit overlap.
                let word = rgb[0].wrapping_shl(16) | rgb[1].wrapping_shl(8) | rgb[2];
                output.rgba[at] = (word >> 16) as u8;
                output.rgba[at + 1] = (word >> 8) as u8;
                output.rgba[at + 2] = word as u8;
            }
        }
        check()?;
        Ok(output)
    }
}

// SPDX-License-Identifier: MPL-2.0
//! Independent staggered opacity clocks. Fractional positions let ruby and
//! other annotations share their base run's reveal span without extra ticks.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StaggeredFade {
    positions: Vec<f32>,
    interval_ms: f32,
    duration_ms: f32,
    elapsed_ms: f32,
    forced: bool,
}
impl StaggeredFade {
    pub fn new(positions: Vec<f32>, interval_ms: f32, duration_ms: f32) -> Result<Self> {
        let value = Self {
            positions,
            interval_ms,
            duration_ms,
            elapsed_ms: 0.,
            forced: false,
        };
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.positions.len() <= 65536
                && self.positions.iter().all(|p| p.is_finite() && *p >= 0.),
            "staggered fade positions"
        );
        ensure!(
            [self.interval_ms, self.duration_ms, self.elapsed_ms]
                .iter()
                .all(|v| v.is_finite() && (0. ..=86_400_000.).contains(v)),
            "staggered fade clock"
        );
        ensure!(
            self.end_ms() <= 86_400_000.,
            "staggered fade total duration"
        );
        Ok(())
    }
    fn end_ms(&self) -> f32 {
        self.positions.iter().copied().fold(0., f32::max) * self.interval_ms + self.duration_ms
    }
    pub fn advance(&mut self, delta_ms: f32) -> Result<()> {
        ensure!(
            delta_ms.is_finite() && delta_ms >= 0.,
            "staggered fade delta"
        );
        self.elapsed_ms = (self.elapsed_ms + delta_ms).min(self.end_ms());
        Ok(())
    }
    pub fn complete(&self) -> bool {
        self.forced || self.positions.is_empty() || self.elapsed_ms >= self.end_ms()
    }
    pub fn finish(&mut self) {
        self.forced = true;
    }
    pub fn opacity(&self, index: usize) -> Option<f32> {
        let position = *self.positions.get(index)?;
        if self.forced {
            return Some(1.);
        }
        let age = self.elapsed_ms - position * self.interval_ms;
        Some(if self.duration_ms > 0. {
            (age / self.duration_ms).clamp(0., 1.)
        } else {
            if age >= 0. { 1. } else { 0. }
        })
    }
    /// Apply to a fresh immutable-source copy, including edges, shadows and
    /// inline images. Colours already contain native alpha; never accumulate
    /// rounding by applying another frame to a previously modified copy.
    pub fn apply(
        &self,
        glyphs: &mut [crate::layer::PositionedGlyph],
        initial: usize,
        default_color: [u8; 4],
    ) -> Result<()> {
        ensure!(
            initial <= glyphs.len() && glyphs.len() - initial == self.positions.len(),
            "staggered fade glyph count"
        );
        for (index, glyph) in glyphs[initial..].iter_mut().enumerate() {
            let opacity = self.opacity(index).expect("validated fade index");
            let alpha = |a: u8| (f32::from(a) * opacity).round().clamp(0., 255.) as u8;
            let mut color = glyph.color.unwrap_or(default_color);
            color[3] = alpha(color[3]);
            glyph.color = Some(color);
            if let Some(style) = &mut glyph.style {
                if let Some(outline) = &mut style.outline {
                    outline.color[3] = alpha(outline.color[3]);
                }
                if let Some(shadow) = &mut style.shadow {
                    shadow.color[3] = alpha(shadow.color[3]);
                }
            }
            if let Some(image) = &mut glyph.image {
                image.tint[3] = alpha(image.tint[3]);
            }
        }
        Ok(())
    }
}

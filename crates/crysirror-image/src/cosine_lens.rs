//! Axis-selective cosine lens with explicit byte-domain sampling.
use crate::{effects::MAX_EFFECT_PIXELS, ImageSurface};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CosineLensByte128 {
    pub amount: f32,
    pub axes: [bool; 2],
}
impl CosineLensByte128 {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.amount.is_finite() && (-1.0..=1.).contains(&self.amount),
            "cosine lens amount must be -1..1"
        );
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
            "cosine lens pixel budget"
        );
        check()?;
        let center = [
            (input.width - 1) as f32 / 2.,
            (input.height - 1) as f32 / 2.,
        ];
        let cosine = |position: u32, axis: usize| {
            if !self.axes[axis] {
                return 0.;
            }
            // A single-pixel axis has zero radius. The native division was
            //undefined there; use its centre phase instead of NaN/indefinite.
            let phase = if center[axis] == 0. {
                0.
            } else {
                ((f64::from(position) - f64::from(center[axis])) / f64::from(center[axis])) as f32
            };
            ((314159265_f64 / 100000000.) * f64::from(phase) * 0.5).cos() as f32
        };
        let horizontal: Vec<_> = (0..input.width).map(|x| cosine(x, 0)).collect();
        let mut out = input.clone();
        for y in 0..input.height {
            check()?;
            let vertical = cosine(y, 1);
            for x in 0..input.width {
                let horizontal = horizontal[x as usize];
                let both = self.axes[0] && self.axes[1];
                let shape = if both {
                    if self.amount < 0. {
                        f64::from(horizontal) * f64::from(vertical)
                    } else {
                        f64::from(horizontal) + f64::from(vertical)
                            - f64::from(horizontal) * f64::from(vertical)
                    }
                } else {
                    f64::from(horizontal) + f64::from(vertical)
                } as f32;
                let delta = (f64::from(self.amount.abs())
                    * f64::from(shape)
                    * if self.amount < 0. { -600. } else { 600. })
                    as f32;
                let denominator = ((f64::from(delta) + 1000.) * 0.001) as f32;
                let scale = (1. / f64::from(denominator)) as f32;
                let point = [
                    ((f64::from(x) - f64::from(center[0])) * f64::from(scale)
                        + f64::from(center[0])) as f32,
                    ((f64::from(y) - f64::from(center[1])) * f64::from(scale)
                        + f64::from(center[1])) as f32,
                ];
                let pixel = crate::byte_sampling::bilinear128_validated(input, point);
                let at = ((y * input.width + x) * 4) as usize;
                out.rgba[at..at + 4].copy_from_slice(&pixel);
            }
        }
        check()?;
        Ok(out)
    }
}

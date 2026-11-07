//! Affine rectangle/ellipse feathering over stored alpha. Source RGB is retained.
use crate::{effects::MAX_EFFECT_PIXELS, ImageSurface};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageShape {
    Rectangle,
    Circle,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShapeMask {
    pub translation: [f32; 2],
    pub scale_percent: [u32; 2],
    pub rotation_turns: f32,
    pub feather: f32,
    pub contrast: f32,
    pub shape: CoverageShape,
    pub invert: bool,
    pub amount256: u16,
    /// Defined coverage at the ellipse's singular radial centre.
    pub circle_center_coverage: f32,
}
impl ShapeMask {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.translation
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 1_000_000.),
            "shape mask translation"
        );
        ensure!(
            self.scale_percent
                .iter()
                .all(|v| (1..=1_000_000).contains(v)),
            "shape mask scale"
        );
        ensure!(
            self.rotation_turns.is_finite() && self.rotation_turns.abs() <= 1_000_000.,
            "shape mask rotation"
        );
        ensure!(
            self.feather.is_finite() && (1.0..=1_000_000.).contains(&self.feather),
            "shape mask feather"
        );
        ensure!(
            self.contrast.is_finite() && (0.0..=1.).contains(&self.contrast),
            "shape mask contrast"
        );
        ensure!(
            self.circle_center_coverage.is_finite()
                && (0.0..=1.).contains(&self.circle_center_coverage),
            "shape mask centre coverage"
        );
        ensure!(self.amount256 <= 256, "shape mask amount");
        Ok(())
    }
    fn coverage(&self, point: [f32; 2], size: [u32; 2]) -> f32 {
        match self.shape {
            CoverageShape::Rectangle => {
                let mut factors = [0.; 4];
                for axis in 0..2 {
                    let feather = (100. / f64::from(self.scale_percent[axis])
                        * f64::from(self.feather)) as f32;
                    let reciprocal = (1. / f64::from(feather)) as f32;
                    let low = (f64::from(point[axis]) + 1.) as f32;
                    let high = (f64::from(size[axis]) - f64::from(point[axis])) as f32;
                    factors[axis * 2] =
                        (f64::from(low.clamp(0., feather)) * f64::from(reciprocal)) as f32;
                    factors[axis * 2 + 1] =
                        (f64::from(high.clamp(0., feather)) * f64::from(reciprocal)) as f32;
                }
                factors.iter().map(|v| f64::from(*v)).product::<f64>() as f32
            }
            CoverageShape::Circle => {
                let normalized = std::array::from_fn::<_, 2, _>(|i| {
                    let fraction = (f64::from(point[i]) / f64::from(size[i])) as f32;
                    (f64::from(fraction) - 0.5) as f32
                });
                let radius = (normalized
                    .iter()
                    .map(|v| f64::from(*v) * f64::from(*v))
                    .sum::<f64>()
                    .sqrt()) as f32;
                if radius > 0.5 {
                    return 0.;
                }
                if radius == 0. {
                    return self.circle_center_coverage;
                }
                let outward = (0.5 / f64::from(radius)) as f32;
                let edge = std::array::from_fn::<_, 2, _>(|i| {
                    let border = (f64::from(normalized[i]) * f64::from(outward)) as f32;
                    let delta = (f64::from(border) - f64::from(normalized[i])) as f32;
                    (f64::from(size[i])
                        * f64::from(delta)
                        * (f64::from(self.scale_percent[i]) * 0.01)) as f32
                });
                let distance = (edge
                    .iter()
                    .map(|v| f64::from(*v) * f64::from(*v))
                    .sum::<f64>()
                    .sqrt()) as f32;
                if distance > self.feather {
                    1.
                } else {
                    (f64::from(distance) / f64::from(self.feather)) as f32
                }
            }
        }
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
            "shape mask pixel budget"
        );
        check()?;
        let size = [input.width, input.height];
        let half = size.map(|v| (v / 2) as f32);
        let angle = f64::from(self.rotation_turns) * 2. * (314159265_f64 / 100000000.);
        let (sine, cosine) = (angle.sin() as f32, angle.cos() as f32);
        let scale = self.scale_percent.map(|v| (100. / f64::from(v)) as f32);
        let origin = std::array::from_fn::<_, 2, _>(|i| -self.translation[i] - half[i]);
        let rotated = [
            (f64::from(origin[0]) * f64::from(cosine) - f64::from(origin[1]) * f64::from(sine))
                as f32,
            (f64::from(origin[0]) * f64::from(sine) + f64::from(origin[1]) * f64::from(cosine))
                as f32,
        ];
        let offset = std::array::from_fn::<_, 2, _>(|i| {
            let scaled = (f64::from(rotated[i]) * f64::from(scale[i])) as f32;
            (f64::from(scaled) + f64::from(half[i])) as f32
        });
        let matrix = [
            cosine * scale[0],
            -sine * scale[0],
            sine * scale[1],
            cosine * scale[1],
        ];
        let soft = (1. - self.contrast).max(0.001);
        let gain = (1. / f64::from(soft)) as f32;
        let mut out = input.clone();
        for y in 0..input.height {
            check()?;
            for x in 0..input.width {
                let point = [
                    (f64::from(x) * f64::from(matrix[0])
                        + f64::from(y) * f64::from(matrix[1])
                        + f64::from(offset[0])) as f32,
                    (f64::from(x) * f64::from(matrix[2])
                        + f64::from(y) * f64::from(matrix[3])
                        + f64::from(offset[1])) as f32,
                ];
                let coverage = self.coverage(point, size);
                let centered = (f64::from(coverage) - 0.5) as f32;
                let expanded = (f64::from(centered) * f64::from(gain)) as f32;
                let coverage = ((f64::from(expanded) + 0.5) as f32).clamp(0., 1.);
                let mut amount = (f64::from(coverage) * 256. + 0.5).trunc() as u32;
                if self.invert {
                    amount = 256 - amount;
                }
                let factor = (amount * u32::from(self.amount256)
                    + ((256 - u32::from(self.amount256)) << 8))
                    / 256;
                let alpha = ((y * input.width + x) * 4 + 3) as usize;
                out.rgba[alpha] = (u32::from(input.rgba[alpha]) * factor / 256) as u8;
            }
        }
        check()?;
        Ok(out)
    }
}

//! Two-axis wave displacement with explicit integer periods, sampled sine,
//! immutable noise tables and byte-exact independent RGBA interpolation.
use crate::{
    ImageSurface,
    byte_sampling::{ByteSampleEdge, bilinear128_edge_validated},
    effects::MAX_EFFECT_PIXELS,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaveShape {
    Sin,
    AbsSin,
    Square,
    AbsSquare,
    Triangle,
    AbsTriangle,
    Saw,
    AbsSaw,
    Random,
    AbsRandom,
    RandomLinear,
    AbsRandomLinear,
}
impl WaveShape {
    pub fn uses_noise(self) -> bool {
        matches!(
            self,
            Self::Random | Self::AbsRandom | Self::RandomLinear | Self::AbsRandomLinear
        )
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaveDisplaceByte128 {
    pub wave: WaveShape,
    pub edge: ByteSampleEdge,
    pub periods: [u32; 2],
    pub amplitudes: [f32; 2],
    pub phases: [f64; 2],
    /// Parallel displaces each axis with its own wave; false crosses axes.
    pub parallel: bool,
    /// Exactly1024 explicit values for noise modes, empty otherwise. This
    /// freezes the owner's noise bank before its asynchronous job begins.
    pub noise: Vec<u16>,
}
impl WaveDisplaceByte128 {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.periods.iter().all(|v| *v > 0 && *v <= i32::MAX as u32),
            "wave period bounds"
        );
        ensure!(
            self.phases
                .iter()
                .all(|v| v.is_finite() && (-1.0..=1.).contains(v)),
            "wave phase bounds"
        );
        ensure!(
            self.amplitudes
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 1_000_000.),
            "wave amplitude bounds"
        );
        ensure!(
            if self.wave.uses_noise() {
                self.noise.len() == 1024
            } else {
                self.noise.is_empty()
            },
            "wave noise bank length"
        );
        Ok(())
    }
    fn sample(&self, pixel: u32, axis: usize, sine: &[f32; 1024]) -> f32 {
        let period = self.periods[axis];
        let reciprocal = (1. / f64::from(period)) as f32;
        let fraction = ((pixel % period) as f32 * reciprocal) as f32;
        let mut phase = (f64::from(fraction) + self.phases[axis]) as f32;
        if phase > 1. {
            phase = (f64::from(phase) - 1.) as f32;
        }
        use WaveShape::*;
        let signed = match self.wave {
            Sin | AbsSin => {
                let position = phase * 1023.;
                let index = if position < 0. {
                    position.trunc() as i32 - 1
                } else {
                    position.trunc() as i32
                };
                let t = (f64::from(position) - f64::from(index)) as f32;
                let a = sine[(index & 1022) as usize];
                let b = sine[((index + 1) & 1022) as usize];
                ((f64::from(b) - f64::from(a)) * f64::from(t) + f64::from(a)) as f32
            }
            Square => {
                if phase >= 0.5 {
                    1.
                } else {
                    -1.
                }
            }
            AbsSquare => {
                if phase >= 0.5 {
                    1.
                } else {
                    0.
                }
            }
            Triangle | AbsTriangle => {
                let p = f64::from(phase);
                (if p >= 0.75 {
                    -(1. - p) * 4.
                } else if p >= 0.5 {
                    -(p - 0.5) * 4.
                } else if p >= 0.25 {
                    (0.5 - p) * 4.
                } else {
                    p * 4.
                }) as f32
            }
            Saw | AbsSaw => (f64::from(phase) * 2. - 1.) as f32,
            Random | AbsRandom | RandomLinear | AbsRandomLinear => {
                // Noise index is the integer period quotient, not phase bits.
                let index = (pixel / period) as usize & 1023;
                let value = |i: usize| (f64::from(self.noise[i]) * (2. / 65535.) - 1.) as f32;
                let a = value(index);
                if matches!(self.wave, RandomLinear | AbsRandomLinear) {
                    ((f64::from(value((index + 1) & 1023)) - f64::from(a)) * f64::from(phase)
                        + f64::from(a)) as f32
                } else {
                    a
                }
            }
        };
        if matches!(
            self.wave,
            AbsSin | AbsTriangle | AbsSaw | AbsRandom | AbsRandomLinear
        ) {
            signed.abs()
        } else {
            signed
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
            "wave pixel budget"
        );
        check()?;
        //4FCCC0 constructs the table mathematically, with its original pi
        //constant. No numeric table from an executable is embedded here.
        let sine = std::array::from_fn(|i| {
            ((i as f64) * (314159265_f64 / 100000000.) / 512.).sin() as f32
        });
        let mut output = input.clone();
        for y in 0..input.height {
            check()?;
            let dy = f64::from(self.sample(y, 1, &sine)) * f64::from(self.amplitudes[1]);
            for x in 0..input.width {
                let dx = f64::from(self.sample(x, 0, &sine)) * f64::from(self.amplitudes[0]);
                let displacement = if self.parallel { [dx, dy] } else { [dy, dx] };
                let point = [
                    (f64::from(x) - displacement[0]) as f32,
                    (f64::from(y) - displacement[1]) as f32,
                ];
                let pixel = bilinear128_edge_validated(input, point, self.edge);
                let at = ((y * input.width + x) * 4) as usize;
                output.rgba[at..at + 4].copy_from_slice(&pixel);
            }
        }
        check()?;
        Ok(output)
    }
}

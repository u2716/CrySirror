//! Device-free procedural scalar signals. Callers supply elapsed time and a
//! deterministic stream identity; no VM, wall clock or device is retained.
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Waveform {
    Sine,
    Square,
    /// Positive half-cycle, then zero; distinct from absolute signed square.
    Pulse,
    Triangle,
    Noise,
    /// Uniform nonnegative control samples; not absolute signed noise.
    PositiveNoise,
    LinearNoise,
    /// Interpolate nonnegative controls before sampling, without rectification.
    PositiveLinearNoise,
    SmoothNoise,
    /// Four neighbouring controls, cubic Bezier quarter-length tangents.
    CubicNoise,
    PositiveCubicNoise,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalFade {
    None,
    In,
    Out,
    InOut,
    InQuart,
    OutQuart,
    InOutQuart,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScalarOscillator {
    pub waveform: Waveform,
    pub absolute: bool,
    pub period_ms: u64,
    pub duration_ms: u64,
    pub delay_ms: u64,
    /// Continue oscillating at unit envelope after the initial fade duration.
    pub sustained: bool,
    pub fade: SignalFade,
    pub seed: u32,
}
impl ScalarOscillator {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (1..=86_400_000).contains(&self.period_ms)
                && self.duration_ms <= 86_400_000
                && self.delay_ms <= 86_400_000,
            "oscillator time bounds"
        );
        Ok(())
    }
    pub fn complete(&self, elapsed_ms: u64) -> bool {
        !self.sustained
            && elapsed_ms.saturating_sub(self.delay_ms) >= self.duration_ms
            && elapsed_ms >= self.delay_ms
    }
    pub fn sample(&self, elapsed_ms: u64, stream: u32) -> Result<f32> {
        self.validate()?;
        if elapsed_ms < self.delay_ms || self.complete(elapsed_ms) {
            return Ok(0.);
        }
        let time = elapsed_ms - self.delay_ms;
        let phase = (time % self.period_ms) as f64 / self.period_ms as f64;
        let index = time / self.period_ms;
        let noise = |at: u64| {
            let mut x =
                at ^ u64::from(self.seed) ^ u64::from(stream).wrapping_mul(0x9e3779b97f4a7c15);
            x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
            ((x ^ (x >> 31)) >> 40) as f64 / 16777215. * 2. - 1.
        };
        let wave = match self.waveform {
            Waveform::Sine => (phase * std::f64::consts::TAU).sin(),
            Waveform::Square => {
                if phase < 0.5 {
                    1.
                } else {
                    -1.
                }
            }
            Waveform::Pulse => {
                if phase < 0.5 {
                    1.
                } else {
                    0.
                }
            }
            Waveform::Triangle => 1. - 4. * ((phase + 0.25).fract() - 0.5).abs(),
            Waveform::Noise => noise(index),
            Waveform::PositiveNoise => (noise(index) + 1.) * 0.5,
            Waveform::LinearNoise | Waveform::PositiveLinearNoise | Waveform::SmoothNoise => {
                let t = if matches!(self.waveform, Waveform::SmoothNoise) {
                    phase * phase * (3. - 2. * phase)
                } else {
                    phase
                };
                let value = noise(index) + (noise(index.wrapping_add(1)) - noise(index)) * t;
                if matches!(self.waveform, Waveform::PositiveLinearNoise) {
                    (value + 1.) * 0.5
                } else {
                    value
                }
            }
            Waveform::CubicNoise | Waveform::PositiveCubicNoise => {
                let control = |at| {
                    let value = noise(at);
                    if matches!(self.waveform, Waveform::PositiveCubicNoise) {
                        (value + 1.) * 0.5
                    } else {
                        value
                    }
                };
                let p0 = control(index.wrapping_sub(1));
                let p1 = control(index);
                let p2 = control(index.wrapping_add(1));
                let p3 = control(index.wrapping_add(2));
                // Native four-control profile4FD0FC/4FD220. The tangent
                // controls are quarters of neighbouring secants, not the
                // smoothstep profile used by SmoothNoise.
                let c1 = p1 + (p2 - p0) * 0.25;
                let c2 = p2 + (p1 - p3) * 0.25;
                let left = 1. - phase;
                left.powi(3) * p1
                    + 3. * left.powi(2) * phase * c1
                    + 3. * left * phase.powi(2) * c2
                    + phase.powi(3) * p2
            }
        };
        let p = if self.duration_ms == 0 {
            1.
        } else {
            (time as f64 / self.duration_ms as f64).min(1.)
        };
        let envelope = if p >= 1. {
            1.
        } else {
            match self.fade {
                SignalFade::None => 1.,
                SignalFade::In => p,
                SignalFade::Out => 1. - p,
                SignalFade::InOut => 1. - (2. * p - 1.).abs(),
                SignalFade::InQuart => p.powi(4),
                SignalFade::OutQuart => 1. - p.powi(4),
                SignalFade::InOutQuart => {
                    if p < 0.5 {
                        (2. * p).powi(4)
                    } else {
                        1. - (2. * p - 1.).powi(4)
                    }
                }
            }
        };
        Ok(((if self.absolute { wave.abs() } else { wave }) * envelope) as f32)
    }
}

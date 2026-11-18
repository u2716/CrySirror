//! Portable bounded stereo feedback delay parameters, independent of a device.
use crate::{Error, Result};
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StereoEchoSpec {
    /// Wet output fraction0..1; dry output uses1-wet.
    pub wet: f32,
    /// Delayed signal fed back into the delay input,0..1.
    pub feedback: f32,
    /// Independent left/right delay in milliseconds,1..2000.
    pub delay_ms: [f32; 2],
    /// Feed the opposite channel's delayed signal into each delay line.
    pub cross_feedback: bool,
}
impl StereoEchoSpec {
    pub fn validate(self) -> Result<()> {
        if !self.wet.is_finite()
            || !(0. ..=1.).contains(&self.wet)
            || !self.feedback.is_finite()
            || !(0. ..=1.).contains(&self.feedback)
            || self
                .delay_ms
                .iter()
                .any(|v| !v.is_finite() || !(1. ..=2000.).contains(v))
        {
            return Err(Error::Invalid("stereo echo parameters"));
        }
        Ok(())
    }
}

/// Portable streaming stereo effects. These define portable DSP parameters,
/// not a sample-identical implementation of a platform's proprietary effect.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StereoEffectSpec {
    Chorus {
        wet: f32,
        depth: f32,
        feedback: f32,
        delay_ms: f32,
        frequency_hz: f32,
        phase_degrees: f32,
    },
    Reverb {
        input_gain: f32,
        wet: f32,
        decay_seconds: f32,
        damping: f32,
    },
}
impl StereoEffectSpec {
    pub fn validate(self) -> Result<()> {
        let bounded = |v: f32, low: f32, high: f32| v.is_finite() && (low..=high).contains(&v);
        let valid = match self {
            Self::Chorus {
                wet,
                depth,
                feedback,
                delay_ms,
                frequency_hz,
                phase_degrees,
            } => {
                bounded(wet, 0., 1.)
                    && bounded(depth, 0., 1.)
                    && bounded(feedback, -0.99, 0.99)
                    && bounded(delay_ms, 0., 20.)
                    && bounded(frequency_hz, 0., 10.)
                    && bounded(phase_degrees, -180., 180.)
            }
            Self::Reverb {
                input_gain,
                wet,
                decay_seconds,
                damping,
            } => {
                bounded(input_gain, 0., 4.)
                    && bounded(wet, 0., 4.)
                    && bounded(decay_seconds, 0.1, 20.)
                    && bounded(damping, 0., 1.)
            }
        };
        if valid {
            Ok(())
        } else {
            Err(Error::Invalid("stereo effect parameters"))
        }
    }
}

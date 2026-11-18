// SPDX-License-Identifier: MPL-2.0
//! Streaming portable chorus/late reverb; bounded history, no song buffering.
use anyhow::Result;
use crysirror_acoustics::reverb::{LateReverb, ReverbSpec};
use crysirror_core::service::StereoEffectSpec;

pub(crate) enum Effect {
    Chorus {
        spec: StereoEffectSpec,
        lines: [Vec<f32>; 2],
        at: usize,
        phase: f64,
        rate: u32,
    },
    Reverb {
        graph: LateReverb,
        input: f32,
        wet: f32,
    },
}
impl Effect {
    pub fn prepare(rate: u32, spec: StereoEffectSpec) -> Result<Self> {
        spec.validate().map_err(|e| anyhow::anyhow!("{e}"))?;
        match spec {
            StereoEffectSpec::Chorus { .. } => {
                let len = (rate as usize * 40 / 1000).max(2) + 2;
                let mut lines = [Vec::new(), Vec::new()];
                for line in &mut lines {
                    line.try_reserve_exact(len)?;
                    line.resize(len, 0.);
                }
                Ok(Self::Chorus {
                    spec,
                    lines,
                    at: 0,
                    phase: 0.,
                    rate,
                })
            }
            StereoEffectSpec::Reverb {
                input_gain,
                wet,
                decay_seconds,
                damping,
            } => Ok(Self::Reverb {
                graph: LateReverb::prepare(
                    rate,
                    ReverbSpec {
                        room_size_metres: 10.,
                        decay_seconds: f64::from(decay_seconds),
                        damping: f64::from(damping),
                    },
                )?,
                input: input_gain,
                wet,
            }),
        }
    }
    pub fn process(&mut self, input: [f32; 2]) -> Result<[f32; 2]> {
        match self {
            Self::Chorus {
                spec,
                lines,
                at,
                phase,
                rate,
            } => {
                let StereoEffectSpec::Chorus {
                    wet,
                    depth,
                    feedback,
                    delay_ms,
                    frequency_hz,
                    phase_degrees,
                } = *spec
                else {
                    unreachable!()
                };
                let len = lines[0].len();
                let delayed: [f32; 2] = std::array::from_fn(|c| {
                    let angle = *phase
                        + if c == 0 {
                            0.
                        } else {
                            f64::from(phase_degrees).to_radians()
                        };
                    let delay = (f64::from(delay_ms)
                        * (1. + f64::from(depth) * angle.sin())
                        * f64::from(*rate)
                        / 1000.)
                        .max(1.);
                    let read = (*at as f64 - delay).rem_euclid(len as f64);
                    let floor = read.floor() as usize;
                    let mix = (read - floor as f64) as f32;
                    lines[c][floor] * (1. - mix) + lines[c][(floor + 1) % len] * mix
                });
                for c in 0..2 {
                    lines[c][*at] = input[c] + delayed[c] * feedback;
                }
                *at = (*at + 1) % len;
                *phase = (*phase
                    + std::f64::consts::TAU * f64::from(frequency_hz) / f64::from(*rate))
                    % std::f64::consts::TAU;
                Ok(std::array::from_fn(|c| {
                    input[c] * (1. - wet) + delayed[c] * wet
                }))
            }
            Self::Reverb {
                graph,
                input: gain,
                wet,
            } => {
                let dry = input.map(|v| v * *gain);
                let tail = graph.process_frame(dry)?;
                Ok(std::array::from_fn(|c| dry[c] + tail[c] * *wet))
            }
        }
    }
}

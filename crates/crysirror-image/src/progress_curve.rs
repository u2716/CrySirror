// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Bounded device-free progress envelopes. Native adapters map their parameter
//! encodings to explicit knots/shapes; every other engine client can reuse the
//! same double-precision evaluation without a VM, host clock or RNG.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
/// One caller-clock position and normalized envelope value.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressKnot {
    /// Caller-selected time units, independent of wall-clock time.
    pub at: u64,
    /// Finite normalized value in0..=1.
    pub value: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProgressShape {
    /// A native immediate assignment, independent of intermediate progress.
    End,
    Linear,
    QuadraticIn,
    SquareRoot,
    HalfCosine {
        pi: f64,
    },
    /// Four-control B-spline with duplicate0/1 endpoints, blended with linear
    ///progress by an explicit tension. General controls use sample_four_point.
    BSpline {
        strength: f64,
    },
    /// General power curve. Both directions preserve overshoot-free endpoints.
    Power {
        exponent: f64,
        direction: ProgressDirection,
    },
    /// Alternating endpoint values over an even number of segments.
    Square {
        segments: u32,
    },
    /// Triangle wave, with an odd segment count ending at one.
    Triangle {
        segments: u32,
    },
    /// Cosine wave with caller-specified angular distance.
    Cosine {
        radians: f64,
    },
    /// Decaying cosine, optionally rectified, or time-reversed. Output may
    /// overshoot; preserving it is essential for spring/back animation.
    DampedCosine {
        exponent: f64,
        radians: f64,
        absolute: bool,
        reverse: bool,
    },
    /// Right-continuous quantized linear progress.
    Steps {
        segments: u32,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressDirection {
    In,
    Out,
    InOut,
    OutIn,
}

/// Constant-size caller-clock mappings. Periodic envelopes do not expand into
/// one knot per cycle, so a long effect has the same memory cost as a short one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProgressTimeMap {
    Linear,
    Delayed {
        delay: u64,
    },
    LeadingRamp {
        duration: u64,
    },
    TrianglePulse {
        peak: u64,
    },
    Trapezoid {
        edge: u64,
    },
    TriangleWave {
        half_period: u64,
        reverse: bool,
    },
    SawWave {
        period: u64,
        reverse: bool,
    },
    /// Two full cycles followed by a final ramp/pulse fitted into the
    /// remaining duration. The cycle boundary belongs to the periodic part.
    TwoCyclesThenTail {
        period: u64,
        triangle: bool,
    },
}

/// Sample an explicit time mapping, then a shape, then a caller-supplied
/// multiplicative noise draw. This owns neither an RNG nor a host clock.
/// A zero duration is one time unit; time is clamped to the complete duration.
pub fn sample_timed_progress(
    mapping: ProgressTimeMap,
    shape: ProgressShape,
    time: u64,
    duration: u64,
    unit_noise: f64,
) -> Result<f64> {
    shape.validate()?;
    ensure!(
        unit_noise.is_finite() && (0.0..=1.0).contains(&unit_noise),
        "progress time noise must be a finite unit value"
    );
    // Keep integer clock-to-f64 conversion exact, including period products.
    const LIMIT: u64 = 1u64 << 51;
    ensure!(
        time <= LIMIT && duration <= LIMIT,
        "progress time clock budget"
    );
    let maximum = duration.max(1);
    let now = time.min(maximum);
    let ratio = |n: u64, d: u64| n as f64 / d.max(1) as f64;
    let triangle = |clock: u64, period: u64| -> Result<f64> {
        let period = period.max(1);
        ensure!(period <= LIMIT / 4, "progress wave period budget");
        let remainder = clock % (period * 2);
        Ok(ratio(
            if remainder < period {
                remainder
            } else {
                period * 2 - remainder
            },
            period,
        ))
    };
    let raw = match mapping {
        ProgressTimeMap::Linear => ratio(now, maximum),
        ProgressTimeMap::Delayed { delay } => {
            ensure!(delay <= LIMIT, "progress delay budget");
            if now < delay {
                0.0
            } else {
                ratio(now - delay, maximum.saturating_sub(delay))
            }
        }
        ProgressTimeMap::LeadingRamp { duration } => {
            ensure!(duration <= LIMIT, "progress leading ramp budget");
            if now < duration {
                ratio(now, duration)
            } else {
                1.0
            }
        }
        ProgressTimeMap::TrianglePulse { peak } => {
            ensure!(peak <= LIMIT, "progress pulse budget");
            if now < peak {
                ratio(now, peak)
            } else {
                ratio(maximum - now, maximum.saturating_sub(peak))
            }
        }
        ProgressTimeMap::Trapezoid { edge } => {
            ensure!(edge <= LIMIT, "progress trapezoid budget");
            if now < edge {
                ratio(now, edge)
            } else if now >= maximum.saturating_sub(edge) {
                ratio(maximum - now, edge)
            } else {
                1.0
            }
        }
        ProgressTimeMap::TriangleWave {
            half_period,
            reverse,
        } => {
            let value = triangle(now, half_period)?;
            if reverse { 1.0 - value } else { value }
        }
        ProgressTimeMap::SawWave { period, reverse } => {
            let period = period.max(1);
            ensure!(period <= LIMIT / 4, "progress wave period budget");
            let value = ratio(now % period, period);
            if reverse { 1.0 - value } else { value }
        }
        ProgressTimeMap::TwoCyclesThenTail {
            period,
            triangle: is_triangle,
        } => {
            let period = period.max(1);
            ensure!(period <= LIMIT / 4, "progress wave period budget");
            let boundary = period * if is_triangle { 4 } else { 2 };
            if now <= boundary {
                if is_triangle {
                    triangle(now, period)?
                } else {
                    ratio(now % period, period)
                }
            } else {
                let span = maximum - boundary;
                let elapsed = now - boundary;
                if is_triangle {
                    let half = (span / 2).max(1);
                    // Signed final numerator is intentional for odd tails.
                    let numerator = if elapsed <= half {
                        elapsed as f64
                    } else {
                        span as f64 - elapsed as f64
                    };
                    numerator / half as f64
                } else {
                    ratio(elapsed, span)
                }
            }
        }
    };
    Ok(shape.apply(raw.clamp(0.0, 1.0)) * unit_noise)
}
impl ProgressShape {
    pub fn validate(self) -> Result<()> {
        match self {
            Self::BSpline { strength } => ensure!(
                strength.is_finite() && strength.abs() <= 10000.,
                "progress spline strength"
            ),
            Self::HalfCosine { pi } => ensure!(
                pi.is_finite() && (0.0..=16.0).contains(&pi),
                "progress curve cosine parameter"
            ),
            Self::Power { exponent, .. } | Self::DampedCosine { exponent, .. } => ensure!(
                exponent.is_finite() && (0.0..=1024.0).contains(&exponent),
                "progress curve exponent"
            ),
            _ => {}
        }
        match self {
            Self::Square { segments } | Self::Triangle { segments } | Self::Steps { segments } => {
                ensure!(
                    (1..=1_000_000).contains(&segments),
                    "progress curve segment count"
                )
            }
            Self::Cosine { radians } | Self::DampedCosine { radians, .. } => ensure!(
                radians.is_finite() && radians.abs() <= 4_000_000.,
                "progress curve angular distance"
            ),
            _ => {}
        }
        Ok(())
    }
    /// Shape normalized progress without clamping its output. Admission must
    /// validate parameters; sampling does not own time or allocate state.
    pub fn apply(self, raw: f64) -> f64 {
        let p = raw.clamp(0., 1.);
        match self {
            Self::End => 1.,
            Self::Linear => p,
            Self::QuadraticIn => p * p,
            Self::SquareRoot => p.sqrt(),
            Self::HalfCosine { pi } => (1. - (p * pi).cos()) * 0.5,
            Self::BSpline { strength } => {
                let spline = (-2. * p * p * p + 3. * p * p + 3. * p + 1.) / 6.;
                if strength <= 1. {
                    spline * strength + (1. - strength) * p
                } else {
                    spline * (2. - strength) + (strength - 1.) * p
                }
            }
            Self::Power {
                exponent: e,
                direction,
            } => match direction {
                ProgressDirection::In => p.powf(e),
                ProgressDirection::Out => 1. - (1. - p).powf(e),
                ProgressDirection::InOut => {
                    if p < 0.5 {
                        (2. * p).powf(e) * 0.5
                    } else {
                        1. - (2. - 2. * p).powf(e) * 0.5
                    }
                }
                ProgressDirection::OutIn => {
                    if p < 0.5 {
                        (1. - (1. - 2. * p).powf(e)) * 0.5
                    } else {
                        0.5 + (2. * p - 1.).powf(e) * 0.5
                    }
                }
            },
            Self::Square { segments } => {
                if ((f64::from(segments) * p).trunc() as u32 & 1) == 0 {
                    1.
                } else {
                    0.
                }
            }
            Self::Triangle { segments } => {
                let position = f64::from(segments) * p;
                let whole = position.trunc();
                if whole as u32 & 1 == 0 {
                    position - whole
                } else {
                    1. - (position - whole)
                }
            }
            Self::Cosine { radians } => (1. - (p * radians).cos()) * 0.5,
            Self::DampedCosine {
                exponent,
                radians,
                absolute,
                reverse,
            } => {
                let t = if reverse { 1. - p } else { p };
                let carrier = (t * radians).cos();
                let carrier = if absolute { carrier.abs() } else { carrier };
                let value = 1. - carrier * (1. - t).powf(exponent);
                if reverse { 1. - value } else { value }
            }
            Self::Steps { segments } => (p * f64::from(segments)).floor() / f64::from(segments),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressCurve {
    /// At most64 ordered knots; duplicate times form right-continuous steps.
    pub knots: Vec<ProgressKnot>,
    pub shape: ProgressShape,
}
impl ProgressCurve {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.knots.is_empty() && self.knots.len() <= 64,
            "progress curve knot budget"
        );
        ensure!(
            self.knots
                .iter()
                .all(|p| p.value.is_finite() && (0.0..=1.0).contains(&p.value)),
            "progress curve finite unit values"
        );
        ensure!(
            self.knots.windows(2).all(|p| p[0].at <= p[1].at),
            "progress curve ordered time"
        );
        self.shape.validate()?;
        Ok(())
    }
    /// Outside the knots, retain the nearest endpoint. Repeated times are
    /// right-continuous steps: the last knot at an exact time wins. Sampling
    /// never advances time, consumes random numbers or changes this object.
    pub fn sample(&self, time: u64) -> Result<f64> {
        self.validate()?;
        let after = self.knots.partition_point(|p| p.at <= time);
        let raw = if after == 0 {
            self.knots[0].value
        } else if after == self.knots.len() {
            self.knots[after - 1].value
        } else {
            let a = self.knots[after - 1];
            let b = self.knots[after];
            a.value + (b.value - a.value) * ((time - a.at) as f64 / (b.at - a.at) as f64)
        }
        .clamp(0.0, 1.0);
        Ok(self.shape.apply(raw))
    }
}
/// Exact fixed-point progress families with integer truncation before trig.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FixedProgressShape {
    Linear,
    HalfCosine,
    SineOut,
    SineIn,
    PowerIn(f64),
    PowerOut(f64),
}
pub fn sample_fixed_progress(shape: FixedProgressShape, progress_8_24: i32) -> Result<i32> {
    let p = progress_8_24.clamp(0, 0x0100_0000);
    let power = |t: f64, e: f64| {
        if e.fract() == 0.0 {
            t.powi(e as i32)
        } else {
            t.powf(e)
        }
    };
    let value = match shape {
        FixedProgressShape::Linear => return Ok(p / 256),
        FixedProgressShape::HalfCosine => {
            let angle = 11_796_480i64 - 180i64 * i64::from(p) / 256;
            ((angle as i32 as f64 * std::f64::consts::PI / 11_796_480.).cos() + 1.) * 32_768.
        }
        FixedProgressShape::SineOut => {
            let angle = 90i64 * i64::from(p) / 256;
            (angle as i32 as f64 * std::f64::consts::PI / 11_796_480.).sin() * 65_536.
        }
        FixedProgressShape::SineIn => {
            let angle = 5_898_240i64 - 90i64 * i64::from(p) / 256;
            (1. - (angle as i32 as f64 * std::f64::consts::PI / 11_796_480.).sin()) * 65_536.
        }
        FixedProgressShape::PowerIn(exponent) => {
            ensure!(
                exponent.is_finite() && (1.0..=16.0).contains(&exponent),
                "fixed progress exponent"
            );
            power(f64::from(p) / 16_777_216., exponent) * 65_536.
        }
        FixedProgressShape::PowerOut(exponent) => {
            ensure!(
                exponent.is_finite() && (1.0..=16.0).contains(&exponent),
                "fixed progress exponent"
            );
            (1. - power(1. - f64::from(p) / 16_777_216., exponent)) * 65_536.
        }
    };
    Ok(value.clamp(0., 65_536.) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_envelope_steps_and_f64_shapes_do_not_own_clock_or_rng() {
        let mut c = ProgressCurve {
            knots: vec![
                ProgressKnot { at: 0, value: 0.0 },
                ProgressKnot { at: 60, value: 1.0 },
            ],
            shape: ProgressShape::QuadraticIn,
        };
        assert_eq!(c.sample(15).unwrap(), 0.0625);
        assert_eq!(c.sample(60).unwrap(), 1.0);
        c.shape = ProgressShape::SquareRoot;
        assert_eq!(c.sample(15).unwrap(), 0.5);
        c.shape = ProgressShape::HalfCosine { pi: 3.14159 };
        assert!((c.sample(30).unwrap() - 0.4999993366025517).abs() < 1e-14);
        let s = ProgressCurve {
            knots: vec![
                ProgressKnot { at: 0, value: 1.0 },
                ProgressKnot { at: 50, value: 1.0 },
                ProgressKnot { at: 50, value: 0.0 },
            ],
            shape: ProgressShape::Linear,
        };
        assert_eq!(s.sample(0).unwrap(), 1.0);
        assert_eq!(s.sample(49).unwrap(), 1.0);
        assert_eq!(s.sample(50).unwrap(), 0.0);
        assert_eq!(s.sample(51).unwrap(), 0.0);
        assert_eq!(
            serde_json::from_slice::<ProgressCurve>(&serde_json::to_vec(&s).unwrap()).unwrap(),
            s
        );
    }
    #[test]
    fn invalid_values_order_budget_and_shapes_fail_before_sampling() {
        let mut c = ProgressCurve {
            knots: vec![],
            shape: ProgressShape::Linear,
        };
        assert!(c.sample(0).is_err());
        c.knots = vec![
            ProgressKnot { at: 1, value: 0.0 },
            ProgressKnot { at: 0, value: 1.0 },
        ];
        assert!(c.validate().is_err());
        c.knots = vec![ProgressKnot {
            at: 0,
            value: f64::NAN,
        }];
        assert!(c.validate().is_err());
        c.knots = vec![ProgressKnot { at: 0, value: 0.0 }; 65];
        assert!(c.validate().is_err());
        c.knots.truncate(1);
        c.shape = ProgressShape::HalfCosine { pi: f64::INFINITY };
        assert!(c.validate().is_err());
    }
}

/// Device-free four-control scalar sampling. Controls are [previous,current,
/// next,following]; the caller selects the original key-neighbour policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FourPointCurve {
    BSpline,
    Hermite,
    TangentBezier,
    AlternatingBezier { odd: bool },
}
pub fn sample_four_point(
    curve: FourPointCurve,
    points: [f64; 4],
    raw: f64,
    strength: f64,
) -> Result<f64> {
    ensure!(
        points.iter().all(|p| p.is_finite())
            && raw.is_finite()
            && strength.is_finite()
            && strength.abs() <= 10000.,
        "four-point scalar parameters"
    );
    let [previous, current, next, following] = points;
    let p = raw.clamp(0., 1.);
    let q = 1. - p;
    let bezier = |t: f64, start: f64, first: f64, second: f64, end: f64| {
        let q = 1. - t;
        q * q * q * start + 3. * q * q * t * first + 3. * q * t * t * second + t * t * t * end
    };
    let hermite = |tension: f64| {
        let reduction = if current == next { 0.25 } else { 1. };
        let d0 = (next - previous) * tension * reduction;
        let d1 = (following - current) * tension * reduction;
        q * q * current * (2. * p + 1.)
            + p * p * next * (3. - 2. * p)
            + q * q * d0 * p
            + d1 * (p - 1.) * p * p
    };
    Ok(match curve {
        FourPointCurve::Hermite => hermite(strength),
        FourPointCurve::BSpline => {
            let cube = p * p * p;
            let square = p * p;
            let spline = (previous * q * q * q
                + (3. * cube - 6. * square + 4.) * current
                + (-3. * cube + 3. * square + 3. * p + 1.) * next
                + following * cube)
                / 6.;
            if strength == 1. {
                spline
            } else if strength < 1. {
                spline * strength + (1. - strength) * (current + (next - current) * p)
            } else {
                spline * (2. - strength) + (strength - 1.) * hermite(1.)
            }
        }
        FourPointCurve::TangentBezier => bezier(
            p,
            current,
            current + (next - previous) * 0.25 * strength,
            next + (current - following) * 0.25 * strength,
            next,
        ),
        FourPointCurve::AlternatingBezier { odd } => {
            let (start, end, pivot, t) = if odd {
                (previous, next, current, 0.5 + 0.5 * p)
            } else {
                (current, following, next, 0.5 * p)
            };
            bezier(
                t,
                start,
                (1. - strength) * start + pivot * strength,
                (1. - strength) * end + pivot * strength,
                end,
            )
        }
    })
}
/// Absolute scalar noise, tapered to zero at the curve endpoint. The engine
/// does not choose an RNG: native VMs and authored procedural producers can
/// provide the same stable sample/ownership boundary.
pub fn sample_scalar_jitter(
    shape: ProgressShape,
    start: f64,
    end: f64,
    raw: f64,
    amplitude: f64,
    unit_noise: f64,
) -> Result<f64> {
    shape.validate()?;
    ensure!(
        [start, end, raw, amplitude, unit_noise]
            .iter()
            .all(|n| n.is_finite())
            && (0.0..=1.0).contains(&unit_noise),
        "scalar jitter parameters"
    );
    let p = raw.clamp(0., 1.);
    Ok(start + (end - start) * shape.apply(p) + (unit_noise * 2. - 1.) * amplitude * (1. - p))
}

/// Evaluate an integer-control Bernstein curve with double-precision basis
/// arithmetic and a single-precision accumulator rounded after each control.
/// This explicit storage profile differs from De Casteljau interpolation near
/// integer boundaries. Degree<=12 keeps every signed32-bit factorial defined.
/// Progress is finite but may lie outside0..1; output truncates toward zero.
/// The caller's cancellation hook runs before allocation and every coefficient.
pub fn bernstein_i32_points(
    controls: &[[i32; 2]], progress: f32, check: &dyn Fn() -> Result<()>,
) -> Result<[i32; 2]> {
    ensure!(controls.len() <= 13 && progress.is_finite(), "Bernstein control/progress profile");
    check()?;
    if controls.is_empty() { return Ok([0, 0]); }
    let degree = controls.len()-1;
    let mut factorial = [1u32; 13];
    for n in 1..=degree { factorial[n] = factorial[n-1] * n as u32; }
    let power = |mut base: f64, mut exponent: usize| {
        let mut product = 1.;
        while exponent != 0 {
            if exponent & 1 != 0 { product *= base; }
            exponent >>= 1;
            if exponent != 0 { base *= base; }
        }
        product
    };
    let mut output = [0f32; 2];
    for (n, control) in controls.iter().enumerate() {
        check()?;
        let weight = power(f64::from(progress), n)
            * (f64::from(factorial[degree]) / (f64::from(factorial[degree-n]) * f64::from(factorial[n])))
            * power(1.-f64::from(progress),degree-n);
        for axis in 0..2 { output[axis] = (f64::from(control[axis])*weight+f64::from(output[axis])) as f32; }
    }
    ensure!(output.iter().all(|n| n.is_finite() && f64::from(*n)>=f64::from(i32::MIN) && f64::from(*n)<2147483648.),"Bernstein integer output range");
    check()?;
    Ok(output.map(|n|n as i32))
}

#[cfg(test)]
mod bernstein_tests {
    use super::*;
    #[test]
    fn stored_float_bernstein_preserves_extrapolation_and_truncation() {
        let points = [[0,0],[0,100],[100,100],[100,0]];
        assert_eq!(bernstein_i32_points(&points,0.5,&||Ok(())).unwrap(),[50,75]);
        assert_eq!(bernstein_i32_points(&[[0,0],[3,-3]],0.5,&||Ok(())).unwrap(),[1,-1]);
        assert_eq!(bernstein_i32_points(&[[10,20],[20,10]],2.,&||Ok(())).unwrap(),[30,0]);
        assert_eq!(bernstein_i32_points(&[],0.,&||Ok(())).unwrap(),[0,0]);
        assert!(bernstein_i32_points(&[[0;2];14],0.,&||Ok(())).is_err());
        assert!(bernstein_i32_points(&points,0.5,&||anyhow::bail!("cancelled")).is_err());
    }
}

// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native0x482070 bit-field decoding and final integer ABI quantization. The
//! reusable envelope/shape arithmetic is the engine's public ProgressCurve.
use anyhow::{Result, ensure};
use crysirror_image::progress_curve::{ProgressCurve, ProgressKnot, ProgressShape};
pub fn curve(max: i32, subframe: i32, kind: i32) -> Result<ProgressCurve> {
    ensure!(
        (0..200).contains(&kind),
        "NNN native random curve requires the shared global RNG transaction"
    );
    let code = kind % 100;
    let warp = code % 15;
    let shape = match code / 15 {
        2 => ProgressShape::QuadraticIn,
        4 => ProgressShape::SquareRoot,
        5 => ProgressShape::HalfCosine { pi: 3.14159 },
        _ => ProgressShape::Linear,
    };
    let max = max.max(1) as u64;
    let edge = subframe.max(0) as u64;
    let knots = match warp {
        0 => vec![
            ProgressKnot { at: 0, value: 0.0 },
            ProgressKnot {
                at: max,
                value: 1.0,
            },
        ],
        4 if edge == 0 => vec![
            ProgressKnot { at: 0, value: 1.0 },
            ProgressKnot {
                at: max,
                value: 1.0,
            },
            ProgressKnot {
                at: max,
                value: 0.0,
            },
        ],
        4 if edge > max => vec![
            ProgressKnot { at: 0, value: 0.0 },
            ProgressKnot {
                at: edge,
                value: 1.0,
            },
        ],
        4 if edge * 2 <= max => vec![
            ProgressKnot { at: 0, value: 0.0 },
            ProgressKnot {
                at: edge,
                value: 1.0,
            },
            ProgressKnot {
                at: max - edge,
                value: 1.0,
            },
            ProgressKnot {
                at: max,
                value: 0.0,
            },
        ],
        4 => vec![
            ProgressKnot { at: 0, value: 0.0 },
            ProgressKnot {
                at: edge,
                value: 1.0,
            },
            ProgressKnot {
                at: edge,
                value: (max - edge) as f64 / edge as f64,
            },
            ProgressKnot {
                at: max,
                value: 0.0,
            },
        ],
        _ => anyhow::bail!("NNN native curve time-map {warp} is not mapped to a shared envelope"),
    };
    let curve = ProgressCurve { knots, shape };
    curve.validate()?;
    Ok(curve)
}
fn progress(count: i32, max: i32, subframe: i32, kind: i32, noise: f64) -> Result<f64> {
    use crysirror_image::progress_curve::{ProgressTimeMap as Time, sample_timed_progress};
    // Actual legacy482070, not the later source's custom-power selector3:
    // signed division/remainder decode reversal, random, shape and time-map.
    let code = kind % 100;
    let delay = subframe.max(0) as u64;
    let mapping = match code % 15 {
        1 => Time::Delayed { delay },
        2 => Time::LeadingRamp { duration: delay },
        3 => Time::TrianglePulse { peak: delay },
        4 => Time::Trapezoid { edge: delay },
        5 | 6 => Time::TriangleWave {
            half_period: delay,
            reverse: code % 15 == 6,
        },
        7 | 8 => Time::SawWave {
            period: delay,
            reverse: code % 15 == 8,
        },
        9 | 10 => Time::TwoCyclesThenTail {
            period: delay,
            triangle: code % 15 == 9,
        },
        _ => Time::Linear,
    };
    let shape = match code / 15 {
        2 => ProgressShape::QuadraticIn,
        4 => ProgressShape::SquareRoot,
        5 => ProgressShape::HalfCosine { pi: 3.14159 },
        _ => ProgressShape::Linear,
    };
    sample_timed_progress(
        mapping,
        shape,
        count.clamp(0, max.max(1)) as u64,
        max.max(1) as u64,
        noise,
    )
}
pub(crate) fn validate(max: i32, subframe: i32, kind: i32) -> Result<()> {
    // Admission never samples the process-global random sequence.
    progress(0, max, subframe, kind, 1.)?;
    Ok(())
}
fn quantize(start: i32, end: i32, kind: i32, progress: f64) -> Result<i32> {
    let (start, end) = if kind / 100 % 2 != 0 {
        (end, start)
    } else {
        (start, end)
    };
    // The native signed32 subtraction precedes conversion to x87 double.
    let value = f64::from(start) + f64::from(end.wrapping_sub(start)) * progress + 0.5;
    ensure!(
        value.is_finite() && (-2147483648.0..2147483648.0).contains(&value.trunc()),
        "NNN native curve result exceeds defined signed integer profile"
    );
    Ok(value as i32)
}
pub fn value(count: i32, max: i32, start: i32, end: i32, subframe: i32, kind: i32) -> Result<i32> {
    ensure!(
        kind / 200 == 0,
        "NNN random curve requires its original global random transaction"
    );
    quantize(start, end, kind, progress(count, max, subframe, kind, 1.)?)
}
pub(crate) fn value_with_random(
    count: i32,
    max: i32,
    start: i32,
    end: i32,
    subframe: i32,
    kind: i32,
    random: &mut crysirror_image::temporal_pattern::PatternRng,
) -> Result<i32> {
    validate(max, subframe, kind)?;
    let noise = if kind / 200 != 0 {
        f64::from(random.next_draw()? % 10000) * 0.0001
    } else {
        1.
    };
    quantize(
        start,
        end,
        kind,
        progress(count, max, subframe, kind, noise)?,
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_linear_quadratic_and_zero_edge_pulse_map_without_fake_profiles() {
        assert_eq!(value(15, 60, 10100, 20100, 0, 30).unwrap(), 10725);
        assert_eq!(value(15, 60, 100, 0, 0, 0).unwrap(), 75);
        assert_eq!(value(0, 50, 256, 128, 0, 4).unwrap(), 128);
        assert_eq!(value(49, 50, 256, 128, 0, 4).unwrap(), 128);
        assert_eq!(value(50, 50, 256, 128, 0, 4).unwrap(), 256);
        assert_eq!(value(0, 50, 10, 20, 0, 100).unwrap(), 20);
        assert!(curve(20, 0, 1).is_err());
        assert!(curve(20, 0, 200).is_err());
    }
}

// SPDX-License-Identifier: MPL-2.0
//! Forward silence scan over a decoded source: the first probe position whose
//! leading interleaved 16-bit samples exceed a mean-magnitude threshold.
use crate::decoder::AudioSource;
use anyhow::{ensure, Result};

/// Probe parameters. `samples` interleaved samples are read at each probe;
/// their magnitude sum is divided by `divisor` before comparing with
/// `threshold` (strictly greater counts as audible).
#[derive(Clone, Copy, Debug)]
pub struct OnsetProbe {
    pub start_seconds: f64,
    pub step_seconds: f64,
    pub samples: usize,
    pub divisor: u32,
    pub threshold: i32,
}

/// Returns the first audible probe position in seconds, or None when every
/// probe before the end is below the threshold. `check` is polled between
/// probes for cancellation.
pub fn first_audible(
    source: &mut dyn AudioSource,
    probe: OnsetProbe,
    check: &dyn Fn() -> Result<()>,
) -> Result<Option<f64>> {
    ensure!(
        probe.step_seconds.is_finite()
            && probe.step_seconds > 0.
            && probe.start_seconds.is_finite()
            && (1..=65536).contains(&probe.samples)
            && probe.divisor > 0,
        "invalid onset probe"
    );
    let rate = f64::from(source.sample_rate().max(1));
    let Some(frames) = source.frame_count() else {
        anyhow::bail!("onset scan requires a source of known length");
    };
    let duration = frames as f64 / rate;
    let mono = source.channel_count() == Some(1);
    let frames_per_probe = if mono { probe.samples } else { probe.samples.div_ceil(2) };
    let mut block = vec![[0f32; 2]; frames_per_probe];
    let mut at = probe.start_seconds.max(0.);
    while at < duration {
        check()?;
        source.seek((at * rate) as u64)?;
        let mut filled = 0;
        while filled < block.len() {
            let n = source.read(&mut block[filled..])?;
            if n == 0 {
                break;
            }
            filled += n;
        }
        let mut sum = 0u64;
        let mut taken = 0;
        'frames: for frame in &block[..filled] {
            for value in if mono { &frame[..1] } else { &frame[..] } {
                if taken == probe.samples {
                    break 'frames;
                }
                let sample = (value.clamp(-1., 1.) * 32767.).round() as i32;
                sum += u64::from(sample.unsigned_abs());
                taken += 1;
            }
        }
        if (sum / u64::from(probe.divisor)) as i64 > i64::from(probe.threshold) {
            return Ok(Some(at));
        }
        at += probe.step_seconds;
    }
    Ok(None)
}

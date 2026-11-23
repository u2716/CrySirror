// SPDX-License-Identifier: MPL-2.0
//! Bounded device-free packed PCM analysis and positive-sign real spectra.
//! Inputs/results are owned data; callers commit them after cancellation checks.
use anyhow::{Result, ensure};
const LIMIT: usize = 65536;
/// Split little-endian unsigned8/signed16 PCM stored in complete32-bit words.
/// Mono output has no right plane; stereo planes retain original sample order.
pub fn split_pcm_words(
    words: &[i32],
    channels: u8,
    bits: u8,
    check: &dyn Fn() -> Result<()>,
) -> Result<(Vec<f32>, Option<Vec<f32>>)> {
    ensure!(
        words.len() <= LIMIT && matches!(channels, 1 | 2) && matches!(bits, 8 | 16),
        "packed PCM analysis profile/budget"
    );
    check()?;
    let count = words.len() * 4 / (usize::from(channels) * usize::from(bits / 8));
    let mut left = Vec::with_capacity(count);
    let mut right = (channels == 2).then(|| Vec::with_capacity(count));
    for &word in words {
        check()?;
        let bytes = word.to_le_bytes();
        if bits == 8 {
            for frame in bytes.chunks_exact(usize::from(channels)) {
                left.push(f32::from(frame[0]) / 128. - 1.);
                if let Some(r) = right.as_mut() {
                    r.push(f32::from(frame[1]) / 128. - 1.);
                }
            }
        } else {
            for frame in bytes.chunks_exact(2 * usize::from(channels)) {
                left.push(f32::from(i16::from_le_bytes([frame[0], frame[1]])) / 32768.);
                if let Some(r) = right.as_mut() {
                    r.push(f32::from(i16::from_le_bytes([frame[2], frame[3]])) / 32768.);
                }
            }
        }
    }
    check()?;
    Ok((left, right))
}
/// Periodic Hann window. First generation uses the full coefficient for the
/// product, retaining an f32 table for repeat calls of the same length.
#[derive(Clone, Default)]
pub struct HannWindow {
    weights: Vec<f32>,
    tau: Option<u32>,
}
impl HannWindow {
    pub fn apply(
        &mut self,
        input: &[f32],
        tau: f32,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<Vec<f32>> {
        ensure!(
            input.len() <= LIMIT && input.iter().all(|n| n.is_finite()) && tau.is_finite(),
            "Hann sample profile/budget"
        );
        check()?;
        let mut result = Vec::with_capacity(input.len());
        let mut weights = Vec::with_capacity(input.len());
        let cached = self.weights.len() == input.len() && self.tau == Some(tau.to_bits());
        for (i, &sample) in input.iter().enumerate() {
            check()?;
            if cached {
                result.push(sample * self.weights[i]);
            } else {
                let weight = 0.5 - ((i as f64 * f64::from(tau)) / input.len() as f64).cos() * 0.5;
                weights.push(weight as f32);
                result.push((f64::from(sample) * weight) as f32);
            }
        }
        check()?;
        if !cached {
            self.weights = weights;
            self.tau = Some(tau.to_bits());
        }
        Ok(result)
    }
}
/// Positive-sign forward RDFT, unnormalized. Packed layout is
/// `[DC, Nyquist, Re(1), Im(1), ...]`, matching Ooura's mathematical convention.
/// Uses the engine's f32 radix2 FFT; x87/Ooura bit identity is not promised.
pub fn real_spectrum(input: &[f32], check: &dyn Fn() -> Result<()>) -> Result<Vec<f32>> {
    let n = input.len();
    ensure!(
        (2..=LIMIT).contains(&n) && n.is_power_of_two() && input.iter().all(|n| n.is_finite()),
        "real FFT power-of-two profile/budget"
    );
    check()?;
    let mut work = vec![0f32; n * 2];
    for (pair, sample) in work.chunks_exact_mut(2).zip(input) {
        pair[0] = *sample;
    }
    crate::pcm_filters::fft(&mut work, n, 1.);
    check()?;
    let mut result = Vec::with_capacity(n);
    result.push(work[0]);
    result.push(work[n]);
    result.extend_from_slice(&work[2..n]);
    ensure!(
        result.iter().all(|n| n.is_finite()),
        "real FFT output overflow"
    );
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pcm_word_boundaries_and_stereo_order() {
        let (l, r) =
            split_pcm_words(&[i32::from_le_bytes([0, 128, 255, 64])], 2, 8, &|| Ok(())).unwrap();
        assert_eq!(l, [-1., 127. / 128.]);
        assert_eq!(r.unwrap(), [0., -0.5]);
        let (l, r) =
            split_pcm_words(&[i32::from_le_bytes([0, 128, 255, 127])], 2, 16, &|| Ok(())).unwrap();
        assert_eq!(l, [-1.]);
        assert_eq!(r.unwrap(), [32767. / 32768.]);
        assert!(split_pcm_words(&[0], 3, 16, &|| Ok(())).is_err());
    }
    #[test]
    fn real_spectrum_positive_sine_and_hann_cache() {
        let result = real_spectrum(&[0., 1., 0., -1.], &|| Ok(())).unwrap();
        for (a, b) in result.iter().zip([0., 0., 0., 2.]) {
            assert!((a - b).abs() < 1e-5);
        }
        let mut window = HannWindow::default();
        let first = window
            .apply(&[1.; 4], std::f32::consts::TAU, &|| Ok(()))
            .unwrap();
        assert!(
            first
                .iter()
                .zip([0., 0.5, 1., 0.5])
                .all(|(a, b)| (a - b).abs() < 1e-6)
        );
        assert_eq!(
            window
                .apply(&[1.; 4], std::f32::consts::TAU, &|| Ok(()))
                .unwrap(),
            first
        );
        assert!(real_spectrum(&[0.; 3], &|| Ok(())).is_err());
        assert!(
            window
                .apply(&[1.; 4], std::f32::consts::TAU, &|| anyhow::bail!(
                    "cancelled"
                ))
                .is_err()
        );
    }
}

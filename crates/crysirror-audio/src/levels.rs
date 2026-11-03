// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Shared amplitude/decibel conversion. Compatibility silence sentinels are
//! explicit and separate from ordinary finite decibels.
use anyhow::{ensure, Result};

/// Convert hundredths of one dB to linear amplitude. -16000..1204 fits the
/// shared mixer0..4 gain range. -10000 is finite -100dB, not implicit silence.
pub fn gain_from_db100(db100: i32) -> Result<f32> {
    ensure!((-16000..=1204).contains(&db100), "audio decibel gain range");
    Ok(10f64.powf(f64::from(db100) / 2000.) as f32)
}
/// Nearest integer hundredth dB, clamped to the supported finite floor.
/// Zero amplitude has no finite logarithm and returns None, never NaN/Infinity.
pub fn db100_from_gain(gain: f32) -> Result<Option<i32>> {
    ensure!(
        gain.is_finite() && (0.0..=4.).contains(&gain),
        "audio amplitude gain range"
    );
    Ok((gain > 0.).then(|| {
        (2000. * f64::from(gain).log10())
            .round()
            .clamp(-16000., 1204.) as i32
    }))
}
/// DirectSound-style bounded attenuation with -10000 as exact silence. Other
/// values are ordinary dB100, with zero as unity. This is not a percent curve.
pub fn gain_from_legacy_db100(db100: i32) -> Result<f32> {
    ensure!(
        (-10000..=0).contains(&db100),
        "legacy audio attenuation range"
    );
    if db100 == -10000 {
        Ok(0.)
    } else {
        gain_from_db100(db100)
    }
}
/// Capture bounded0..1 gain for a native attenuation control. Values at/below
/// -100dB map to its silence sentinel, preserving existing release semantics.
pub fn legacy_db100_from_gain(gain: f32) -> Result<i32> {
    ensure!(
        gain.is_finite() && (0.0..=1.).contains(&gain),
        "legacy audio amplitude range"
    );
    if gain <= 0.00001 {
        Ok(-10000)
    } else {
        Ok(db100_from_gain(gain)?.unwrap().clamp(-10000, 0))
    }
}

/// Validate independent output channel amplitudes before changing any voice
/// state. Keeping these separate from pan supports balance/attenuation laws.
pub fn validate_stereo_gains(gains: [f32; 2]) -> Result<()> {
    ensure!(
        gains
            .iter()
            .all(|gain| gain.is_finite() && (0.0..=4.).contains(gain)),
        "stereo output gains must each be finite0..4"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finite_decibels_and_legacy_silence_are_distinct() {
        assert!((gain_from_db100(-600).unwrap() - 0.5011872).abs() < 1e-7);
        assert_eq!(gain_from_db100(0).unwrap(), 1.);
        assert_eq!(db100_from_gain(0.).unwrap(), None);
        assert_eq!(db100_from_gain(1.).unwrap(), Some(0));
        assert_eq!(db100_from_gain(4.).unwrap(), Some(1204));
        assert_eq!(gain_from_legacy_db100(-10000).unwrap(), 0.);
        assert_eq!(gain_from_db100(-10000).unwrap(), 0.00001);
        assert_eq!(legacy_db100_from_gain(0.00001).unwrap(), -10000);
        assert_eq!(legacy_db100_from_gain(0.5).unwrap(), -602);
        assert!((gain_from_legacy_db100(-1000).unwrap() - 0.31622776).abs() < 1e-7);
    }
    #[test]
    fn malformed_levels_fail_and_rounding_is_bounded() {
        for n in [-16001, 1205, i32::MIN, i32::MAX] {
            assert!(gain_from_db100(n).is_err());
        }
        for n in [-10001, 1] {
            assert!(gain_from_legacy_db100(n).is_err());
        }
        for n in [f32::NAN, f32::INFINITY, -0.1, 4.1] {
            assert!(db100_from_gain(n).is_err());
        }
        for n in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
            assert!(legacy_db100_from_gain(n).is_err());
        }
        for n in (-15000..=1200).step_by(17) {
            assert_eq!(
                db100_from_gain(gain_from_db100(n).unwrap()).unwrap(),
                Some(n)
            );
        }
        assert_eq!(db100_from_gain(f32::MIN_POSITIVE).unwrap(), Some(-16000));
    }
    #[test]
    fn converted_levels_control_actual_device_free_pcm() {
        use crate::{
            decoder::Clip,
            mixer::{Mixer, MixerConfig, VoiceSpec},
        };
        let mut mixer = Mixer::new(MixerConfig {
            sample_rate: 8000,
            max_voices: 4,
            seed: 1,
        })
        .unwrap();
        let h = mixer
            .play(
                1,
                Clip::new(8000, vec![[0.5; 2]; 32]).unwrap().source(),
                VoiceSpec::default(),
            )
            .unwrap();
        mixer
            .set_gain(1, h, gain_from_db100(-600).unwrap())
            .unwrap();
        let mut out = [[0.; 2]; 4];
        mixer.render(&mut out).unwrap();
        for value in out.into_iter().flatten() {
            assert!((value - 0.2505936).abs() < 1e-6);
        }
        mixer
            .set_gain(1, h, gain_from_legacy_db100(-10000).unwrap())
            .unwrap();
        mixer.render(&mut out).unwrap();
        assert_eq!(out, [[0.; 2]; 4]);
    }
}

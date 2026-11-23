// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Shared amplitude/decibel conversion. Compatibility silence sentinels are
//! explicit and separate from ordinary finite decibels.
use anyhow::{ensure, Result};
use crysirror_core::service::{AudioPcm16Gain, Pcm16GainScale};

pub(crate) fn to_pcm16(sample: f32) -> i16 {
    (sample * 32_768.).round().clamp(-32_768., 32_767.) as i16
}

/// Quantize the own gain and transport fade before applying the legacy
/// secondary word and each channel's attenuation.
pub(crate) fn apply_pcm16_gain(
    frame: [f32; 2],
    control: AudioPcm16Gain,
    gain: f32,
    transport: f32,
    fading_out: bool,
    device_gain: u16,
) -> [f32; 2] {
    if control.scale == Pcm16GainScale::Percent {
        let base = (gain.clamp(0., 1.) * 100.).round() as u32;
        let ramp = transport.clamp(0., 1.) * 100.;
        let fade = if fading_out { ramp.ceil() } else { ramp.floor() } as u32;
        let level = (base * fade / 100).wrapping_mul(control.multiplier) / 100;
        return [0, 1].map(|side| {
            let coefficient = (level.wrapping_mul(control.stereo[side]) / 100).min(100);
            let sample = i32::from(to_pcm16(frame[side]));
            (sample * coefficient as i32 / 100) as f32 / 32_768.
        });
    }
    let base = (gain.clamp(0., 1.) * 256.).round() as u32;
    let fade = transport.clamp(0., 1.) * 256.;
    let fade = if fading_out {
        fade.ceil()
    } else {
        fade.floor()
    } as u32;
    let level = (base * fade) >> 8;
    let shared = level.wrapping_mul(control.multiplier).wrapping_mul(256) >> 16;
    let shared = (shared * u32::from(device_gain)) >> 8;
    [0, 1].map(|side| {
        let coefficient = (shared.wrapping_mul(control.stereo[side]) >> 8).min(256);
        let sample = i32::from(to_pcm16(frame[side]));
        ((sample * coefficient as i32) >> 8) as f32 / 32_768.
    })
}

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
    fn percentage_fader_truncates_each_product_and_signed_samples() {
        let unit = 1.0 / 32768.0;
        let control = AudioPcm16Gain {
            multiplier: 75,
            stereo: [100, 50],
            ..AudioPcm16Gain::percent()
        };
        // floor(75 * 73 / 100) = 54, then 54 and 27 per side.
        assert_eq!(
            apply_pcm16_gain([-101. * unit, 101. * unit], control, 0.73, 1., false, 256),
            [-54. * unit, 27. * unit]
        );
        let wrapped = AudioPcm16Gain { multiplier: 0x4000_0000, ..control };
        assert_eq!(apply_pcm16_gain([1.; 2], wrapped, 1., 1., false, 256), [0.; 2]);
        let boosted = AudioPcm16Gain { multiplier: 200, ..control };
        assert_eq!(
            apply_pcm16_gain([-unit, -unit], boosted, 1., 1., false, 256),
            [-unit; 2]
        );
    }

    #[test]
    fn pcm16_fader_preserves_shift_order_and_signed_samples() {
        let unit = 1. / 32_768.;
        let frame = [-unit, unit];
        let halfway = AudioPcm16Gain {
            multiplier: 128,
            ..Default::default()
        };
        assert_eq!(
            apply_pcm16_gain(frame, halfway, 1., 1., false, 256),
            [-unit, 0.]
        );
        let wraps = AudioPcm16Gain {
            multiplier: 65536,
            ..Default::default()
        };
        assert_eq!(apply_pcm16_gain(frame, wraps, 1., 1., false, 256), [0.; 2]);
        assert_eq!(
            apply_pcm16_gain([0.5; 2], wraps, 1., 0.5, false, 256),
            [0.5; 2],
        );
        let doubled = AudioPcm16Gain {
            multiplier: 512,
            stereo: [64, 256],
            ..Default::default()
        };
        assert_eq!(
            apply_pcm16_gain([0.5; 2], doubled, 1., 1., false, 256),
            [0.25, 0.5]
        );
        let wrapped_pan = AudioPcm16Gain {
            multiplier: 256,
            stereo: [0x8000_0100, 256],
            ..Default::default()
        };
        assert_eq!(
            apply_pcm16_gain([0.5; 2], wrapped_pan, 1. / 256., 1., false, 256),
            [0.5, 0.5 / 256.],
        );
    }

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

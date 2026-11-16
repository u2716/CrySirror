// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Phase vocoder time/pitch scaling in the KiriKiri formulation
//! (krkrz fd5c4baa `sound/PhaseVocoderDSP.cpp` and `PhaseVocoderFilter.cpp`,
//! W.Dee and contributors): Vorbis-power-sine windows, per-bin frequency
//! estimation, linear spectrum resampling for the pitch scale, phase
//! accumulation scaled by the exact hop ratio, overlap-add. Trigonometry is
//! exact where the native uses fast approximations.
//!
//! The stream shape follows the filter's decode loop: input arrives in hops
//! (the last one zero-padded), a frame is processed once a whole frame is
//! buffered, and only complete output hops are produced, so the result is
//! `frames · output_hop` long with the first frame's fade-in at the start.
use anyhow::{Result, ensure};
use crysirror_tasks::Cancellation;
use std::f64::consts::PI;

fn fft(re: &mut [f64], im: &mut [f64], inverse: bool) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = 2. * PI / len as f64 * if inverse { 1. } else { -1. };
        let (wr, wi) = (ang.cos(), ang.sin());
        for start in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1f64, 0f64);
            for k in 0..len / 2 {
                let (a, b) = (start + k, start + k + len / 2);
                let (xr, xi) = (re[b] * cr - im[b] * ci, re[b] * ci + im[b] * cr);
                re[b] = re[a] - xr;
                im[b] = im[a] - xi;
                re[a] += xr;
                im[a] += xi;
                let t = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = t;
            }
        }
        len <<= 1;
    }
}
fn wrap_pi(v: f32) -> f32 {
    let mut unit = (v as f64 * (1. / PI)) as i32;
    if unit >= 0 {
        unit += unit & 1;
    } else {
        unit -= unit & 1;
    }
    v - (PI * unit as f64) as f32
}

/// `overlap` 0 picks 2/4/8 from the time scale as the native does.
pub fn process(pitch: f32, time: f32, window: u32, overlap: u32, rate: u32, channels: usize, input: &[f32], cancel: &Cancellation) -> Result<Vec<f32>> {
    ensure!(window.is_power_of_two() && (64..=32768).contains(&window), "phase vocoder window");
    ensure!(matches!(overlap, 0 | 2 | 4 | 8 | 16 | 32), "phase vocoder overlap");
    ensure!(pitch.is_finite() && pitch > 0. && time.is_finite() && time > 0., "phase vocoder scales");
    let frame = window as usize;
    let over = match overlap {
        0 if time <= 0.2 => 2,
        0 if time <= 1.2 => 4,
        0 => 8,
        v => v as usize,
    };
    let in_hop = frame / over;
    let out_hop = ((in_hop as f32 * time) as usize) & !1;
    ensure!(in_hop > 0 && out_hop > 0, "phase vocoder hop size");
    let exact_time = out_hop as f32 / in_hop as f32;
    let volume = time / frame as f32 / pitch.sqrt() / over as f32 * 2. * 2.;
    let (mut win_in, mut win_out) = (vec![0f32; frame], vec![0f32; frame]);
    for i in 0..frame {
        let x = (i as f64 + 0.5) / frame as f64;
        let w = (PI / 2. * (PI * x).sin() * (PI * x).sin()).sin();
        win_in[i] = w as f32;
        win_out[i] = (w * volume as f64) as f32;
    }
    let over_rad = (2. * PI / over as f64) as f32;
    let over_rad_recp = (1. / over_rad as f64) as f32;
    let band = (rate as f64 / frame as f64) as f32;
    let band_recp = (1. / band as f64) as f32;
    let half = frame / 2;
    let frames_in = input.len() / channels;
    let chunks = frames_in.div_ceil(in_hop);
    let count = (chunks + 1).saturating_sub(over);
    let mut out = vec![0f32; (count * out_hop + frame) * channels];
    let mut last_anal = vec![vec![0f32; half]; channels];
    let mut last_synth = vec![vec![0f32; half]; channels];
    let (mut re, mut im) = (vec![0f64; frame], vec![0f64; frame]);
    let (mut anal, mut synth) = (vec![0f32; frame], vec![0f32; frame]);
    let mut adjust: u32 = 0;
    for f in 0..count {
        cancel.check()?;
        for ch in 0..channels {
            for i in 0..frame {
                let n = f * in_hop + i;
                let x = if n < frames_in { input[n * channels + ch] } else { 0. };
                re[i] = (x * win_in[i]) as f64;
                im[i] = 0.;
            }
            fft(&mut re, &mut im, false);
            // Ooura layout: a[2k] = Re, a[2k+1] = +sin sum (the conjugate).
            for k in 0..half {
                anal[2 * k] = re[k] as f32;
                anal[2 * k + 1] = -im[k] as f32;
            }
            anal[1] = 0.;
            let (la, ls) = (&mut last_anal[ch], &mut last_synth[ch]);
            if pitch != 1. {
                for i in 0..half {
                    let (r, m) = (anal[i * 2], anal[i * 2 + 1]);
                    let mag = (r * r + m * m).sqrt();
                    let ang = m.atan2(r);
                    let mut tmp = la[i] - ang;
                    la[i] = ang;
                    tmp -= i as f32 * over_rad;
                    tmp = wrap_pi(tmp) * over_rad_recp;
                    anal[i * 2] = mag;
                    anal[i * 2 + 1] = (i as f32 + tmp) * band;
                }
                let recp = 1. / pitch;
                for i in 0..half {
                    let fi = i as f32 * recp;
                    let index = fi as usize;
                    let frac = fi - index as f32;
                    if index + 1 < half {
                        synth[i * 2] = anal[index * 2] + frac * (anal[index * 2 + 2] - anal[index * 2]);
                        synth[i * 2 + 1] = pitch * (anal[index * 2 + 1] + frac * (anal[index * 2 + 3] - anal[index * 2 + 1]));
                    } else if index < half {
                        synth[i * 2] = anal[index * 2];
                        synth[i * 2 + 1] = anal[index * 2 + 1] * pitch;
                    } else {
                        synth[i * 2] = 0.;
                        synth[i * 2 + 1] = 0.;
                    }
                }
                for i in 0..half {
                    let (mag, freq) = (synth[i * 2], synth[i * 2 + 1]);
                    let mut tmp = freq * band_recp - i as f32;
                    tmp *= over_rad;
                    tmp += i as f32 * over_rad;
                    tmp *= exact_time;
                    ls[i] -= tmp;
                    let (s, c) = ls[i].sin_cos();
                    synth[i * 2] = mag * c;
                    synth[i * 2 + 1] = mag * s;
                }
            } else {
                for i in 0..half {
                    let (r, m) = (anal[i * 2], anal[i * 2 + 1]);
                    let mag = (r * r + m * m).sqrt();
                    let ang = m.atan2(r);
                    let mut tmp = la[i] - ang;
                    la[i] = ang;
                    let shift = i as f32 * over_rad;
                    tmp -= shift;
                    tmp = wrap_pi(tmp);
                    tmp += shift;
                    tmp *= exact_time;
                    ls[i] -= tmp;
                    let (s, c) = ls[i].sin_cos();
                    synth[i * 2] = mag * c;
                    synth[i * 2 + 1] = mag * s;
                }
            }
            synth[1] = 0.;
            // Inverse: Hermitian spectrum of Y_k = R_k - i·I_k; the Ooura
            // inverse is n/2 times the normalised transform.
            re.fill(0.);
            im.fill(0.);
            for k in 0..half {
                re[k] = synth[2 * k] as f64;
                im[k] = -synth[2 * k + 1] as f64;
                if k > 0 {
                    re[frame - k] = re[k];
                    im[frame - k] = -im[k];
                }
            }
            fft(&mut re, &mut im, true);
            for i in 0..frame {
                out[(f * out_hop + i) * channels + ch] += (re[i] * 0.5) as f32 * win_out[i];
            }
        }
        adjust = adjust.wrapping_add(0x03e8_a444);
        if adjust >= 0xfa29_11fe {
            adjust = 0;
            for ls in &mut last_synth {
                for v in ls.iter_mut() {
                    let n = (*v as f64 / (2. * PI)) as i64;
                    *v -= (n as f64 * (2. * PI)) as f32;
                }
            }
        }
    }
    out.truncate(count * out_hop * channels);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tone(hz: f32, n: usize) -> Vec<f32> {
        (0..n).map(|i| (2. * std::f32::consts::PI * hz * i as f32 / 44100.).sin() * 0.5).collect()
    }
    fn dominant(v: &[f32]) -> f32 {
        // Zero-crossing rate of the steady middle part.
        let mid = &v[v.len() / 4..v.len() * 3 / 4];
        let crossings = mid.windows(2).filter(|w| w[0] <= 0. && w[1] > 0.).count();
        crossings as f32 * 44100. / mid.len() as f32
    }
    #[test]
    fn pitch_and_time_scale() {
        let c = Cancellation::default();
        let input = tone(440., 44100);
        let same = process(1., 1., 4096, 8, 44100, 1, &input, &c).unwrap();
        assert!((dominant(&same) - 440.).abs() < 6., "{}", dominant(&same));
        let level = same[same.len() / 4..same.len() * 3 / 4].iter().fold(0f32, |m, x| m.max(x.abs()));
        assert!((level - 0.5).abs() < 0.05, "{level}");
        let up = process(1.5, 1., 4096, 8, 44100, 1, &input, &c).unwrap();
        assert!((dominant(&up) - 660.).abs() < 12., "{}", dominant(&up));
        let slow = process(1., 2., 4096, 8, 44100, 1, &input, &c).unwrap();
        assert!((dominant(&slow) - 440.).abs() < 8., "{}", dominant(&slow));
        assert!(slow.len() > same.len() * 19 / 10 && slow.len() <= same.len() * 2);
    }
}
